// src/logging/runtime_logger.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::io::Write;
use std::time::{Duration as StdDuration};
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::time::{self, Duration};
use tokio::task;
use tracing_appender::rolling;
use tracing_appender::rolling::RollingFileAppender;
use serde_json::json;
use chrono::{FixedOffset, TimeZone, Utc};
use tokio::fs;
use tracing_subscriber::fmt::MakeWriter;

/// 单条日志消息
pub struct LogEntry {
    pub level: String,
    pub content: String,
}

/// 运行日志管理器（RuntimeLogger）
/// 将运行时日志按日志级别分流到不同的日志文件中。
pub struct RuntimeLogger {
    sender: Sender<LogEntry>,
    // 存储每个日志级别对应的 RollingFileAppender
    log_files: HashMap<String, Arc<RollingFileAppender>>,
}

impl RuntimeLogger {
    /// 创建一个新的 RuntimeLogger
    ///
    /// - `log_dir`: 日志文件存放目录
    /// - `file_prefix`: 文件前缀，例如 "runtime"（最终文件名形如 runtime_info.json 等）
    /// - `buffer_size`: mpsc 通道缓冲区大小
    /// - `batch_size`: 每个日志级别批量写入的日志条数
    /// - `flush_interval`: 定时刷新日志的时间间隔（毫秒）
    pub fn new(
        log_dir: &str,
        file_prefix: &str,
        buffer_size: usize,
        batch_size: usize,
        flush_interval: u64,
    ) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(buffer_size);
        // 定义需要分文件存储的日志级别
        let levels = vec!["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        let mut log_files = HashMap::new();
        for level in &levels {
            let file_name = format!("{}_{}.json", file_prefix, level.to_lowercase());
            let appender = rolling::hourly(log_dir, &file_name);
            log_files.insert(level.to_string(), Arc::new(appender));
        }
        let logger = Arc::new(Self { sender, log_files: log_files.clone() });
        tokio::spawn(Self::background_log_writer(log_files, receiver, batch_size, flush_interval));
        // 启动后台任务定期清理日志文件
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

    /// 记录运行日志，接受两个参数：level 和 message
    pub async fn log(&self, level: &str, message: &str) {
        let tz = FixedOffset::east(8 * 3600);
        let timestamp = tz.from_utc_datetime(&Utc::now().naive_utc()).to_rfc3339();
        let log_entry = json!({
        "timestamp": timestamp,
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

    /// 后台日志写入任务
    async fn background_log_writer(
        log_files: HashMap<String, Arc<RollingFileAppender>>,
        mut receiver: Receiver<LogEntry>,
        batch_size: usize,
        flush_interval: u64,
    ) {
        // 每个日志级别独立的缓冲区
        let mut buffers: HashMap<String, Vec<String>> = HashMap::new();
        for level in log_files.keys() {
            buffers.insert(level.clone(), Vec::new());
        }
        let mut interval = time::interval(Duration::from_millis(flush_interval));
        loop {
            tokio::select! {
                Some(entry) = receiver.recv() => {
                    buffers.entry(entry.level.clone()).or_default().push(entry.content);
                    if let Some(buffer) = buffers.get(&entry.level) {
                        if buffer.len() >= batch_size {
                            if let Some(appender) = log_files.get(&entry.level) {
                                Self::write_logs_to_disk(appender.clone(), buffer).await;
                            }
                            buffers.insert(entry.level.clone(), Vec::new());
                        }
                    }
                },
                _ = interval.tick() => {
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

    async fn cleanup_old_logs(log_dir: &str, retention_hours: u64) {
        use std::time::{SystemTime, Duration as StdDuration};
        let retention_duration = StdDuration::from_secs(retention_hours * 3600);
        let now = SystemTime::now();
        match tokio::fs::read_dir(log_dir).await {
            Ok(mut dir) => {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    let path = entry.path();
                    if let Ok(metadata) = entry.metadata().await {
                        if let Ok(modified) = metadata.modified() {
                            if now.duration_since(modified).unwrap_or_default() > retention_duration {
                                if let Err(e) = tokio::fs::remove_file(&path).await {
                                    eprintln!("Failed to delete old log file {:?}: {}", path, e);
                                } else {
                                    println!("Deleted old log file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                eprintln!("Failed to read log directory {}: {}", log_dir, e);
            }
        }
    }

    pub async fn shutdown(&self) {
        drop(&self.sender);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
