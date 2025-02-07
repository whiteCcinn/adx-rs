// logging/runtime_logger.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::io::Write;
use std::time::{SystemTime, Duration as StdDuration};

use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::time::{self, Duration};
use tokio::task;
use tracing_appender::rolling;
use tracing_appender::rolling::RollingFileAppender;
use serde_json::json;
use chrono::Utc; // 用于获取当前时间
use tokio::fs;
use tracing_subscriber::fmt::MakeWriter;

/// 日志消息封装，包含日志级别和构造好的 JSON 格式内容
pub struct LogEntry {
    pub level: String,
    pub content: String,
}

/// **服务运行日志管理器**
///
/// 本管理器会根据日志级别分别写入到不同的文件中。文件名的格式为：
/// `{file_prefix}_{level}.json`，
/// 并使用 rolling 模式（例如按小时切割）。
///
/// 同时启动了一个后台任务定期扫描日志目录，将超过 72 小时的日志文件删除，
/// 从而保证只保留最近 72 小时的日志。
pub struct RuntimeLogger {
    sender: Sender<LogEntry>,
    // 每个日志级别对应一个 RollingFileAppender
    log_files: HashMap<String, Arc<RollingFileAppender>>,
}

impl RuntimeLogger {
    /// ✅ **创建 `RuntimeLogger`**
    ///
    /// * `log_dir`：日志文件存放目录
    /// * `file_prefix`：日志文件的前缀（最终文件名格式为 `{file_prefix}_{level}.json`）
    /// * `buffer_size`：mpsc 通道缓冲区大小
    /// * `batch_size`：每个日志级别批量写入的日志条数
    /// * `flush_interval`：定时刷新日志的时间间隔（毫秒）
    ///
    /// 此外，会启动一个后台任务每小时扫描一次日志目录，删除修改时间超过 72 小时的日志文件。
    pub fn new(
        log_dir: &str,
        file_prefix: &str,
        buffer_size: usize,
        batch_size: usize,
        flush_interval: u64,
    ) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(buffer_size);
        // 定义需要分文件存储的日志级别
        let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        let mut log_files = HashMap::new();

        for level in &levels {
            let file_name = format!("{}_{}.json", file_prefix, level.to_lowercase());
            let appender = rolling::hourly(log_dir, file_name);
            log_files.insert(level.to_string(), Arc::new(appender));
        }

        let logger = Arc::new(Self {
            sender,
            log_files: log_files.clone(),
        });

        // 启动后台异步日志写入任务
        tokio::spawn(Self::background_log_writer(
            log_files.clone(),
            receiver,
            batch_size,
            flush_interval,
        ));

        // 启动后台任务定期清理超过 72 小时的日志文件
        {
            let log_dir = log_dir.to_string();
            tokio::spawn(async move {
                let retention_hours = 72;
                let cleanup_interval = Duration::from_secs(3600); // 每小时扫描一次
                loop {
                    Self::cleanup_old_logs(&log_dir, retention_hours).await;
                    tokio::time::sleep(cleanup_interval).await;
                }
            });
        }

        logger
    }

    /// ✅ **异步发送日志**
    ///
    /// 调用该方法即可将日志条目发送到后台任务进行写入，
    /// 日志内容会被封装为 JSON 格式，并附上时间戳。
    pub async fn log(&self, level: &str, message: &str) {
        let log_entry = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "level": level,
            "message": message
        })
            .to_string();

        let entry = LogEntry {
            level: level.to_string(),
            content: log_entry,
        };

        if let Err(e) = self.sender.send(entry).await {
            eprintln!("Failed to send runtime log message: {}", e);
        }
    }

    /// ✅ **后台异步日志写入**
    ///
    /// 该函数从 mpsc Receiver 中批量接收日志，并将不同日志级别的日志分组后，
    /// 按设定的批量大小或定时间隔写入到对应的文件中。
    async fn background_log_writer(
        log_files: HashMap<String, Arc<RollingFileAppender>>,
        mut receiver: Receiver<LogEntry>,
        batch_size: usize,
        flush_interval: u64,
    ) {
        // 每个日志级别的缓存
        let mut buffers: HashMap<String, Vec<String>> = HashMap::new();
        // 初始化所有预定义的级别
        for level in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].iter() {
            buffers.insert(level.to_string(), Vec::new());
        }
        let mut interval = time::interval(Duration::from_millis(flush_interval));

        loop {
            tokio::select! {
                Some(entry) = receiver.recv() => {
                    // 将日志条目放入对应级别的缓存中
                    buffers.entry(entry.level.clone()).or_default().push(entry.content);
                    // 检查每个级别的缓存是否达到批量大小
                    for (level, buffer) in buffers.iter_mut() {
                        if buffer.len() >= batch_size {
                            if let Some(appender) = log_files.get(level) {
                                Self::write_logs_to_disk(appender.clone(), buffer).await;
                            }
                            buffer.clear();
                        }
                    }
                }
                _ = interval.tick() => {
                    // 定时刷新所有级别的日志缓存
                    for (level, buffer) in buffers.iter_mut() {
                        if !buffer.is_empty() {
                            if let Some(appender) = log_files.get(level) {
                                Self::write_logs_to_disk(appender.clone(), buffer).await;
                            }
                            buffer.clear();
                        }
                    }
                }
            }
        }
    }

    /// ✅ **将日志写入磁盘**
    ///
    /// 将传入缓冲区中的日志条目合并成一段文本后，通过阻塞任务写入到对应的 RollingFileAppender 管理的日志文件中。
    async fn write_logs_to_disk(file: Arc<RollingFileAppender>, buffer: &Vec<String>) {
        let content = buffer.join("\n") + "\n";

        let file_clone = Arc::clone(&file);
        task::spawn_blocking(move || {
            let mut writer = file_clone.make_writer();
            if let Err(e) = writer.write_all(content.as_bytes()) {
                eprintln!("Failed to write runtime logs: {}", e);
            }
        })
            .await
            .unwrap();
    }

    /// ✅ **后台日志文件清理**
    ///
    /// 扫描指定目录下的所有文件，如果文件的最后修改时间超过了 retention_hours 小时，则删除该文件。
    async fn cleanup_old_logs(log_dir: &str, retention_hours: u64) {
        let retention_duration = StdDuration::from_secs(retention_hours * 3600);
        let now = SystemTime::now();

        match fs::read_dir(log_dir).await {
            Ok(mut dir) => {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    let path = entry.path();
                    if let Ok(metadata) = entry.metadata().await {
                        if let Ok(modified) = metadata.modified() {
                            if now.duration_since(modified).unwrap_or_default() > retention_duration {
                                if let Err(e) = fs::remove_file(&path).await {
                                    eprintln!("Failed to delete old log file {:?}: {}", path, e);
                                } else {
                                    println!("Deleted old log file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read log directory {}: {}", log_dir, e);
            }
        }
    }
}
