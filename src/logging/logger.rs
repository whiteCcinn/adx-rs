use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::time::{self, Duration};
use tokio::task;
use std::io::Write;
use tracing_appender::rolling;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::fmt::MakeWriter;

pub struct LogManager {
    sender: Sender<String>,
    log_file: Arc<RollingFileAppender>, // 直接存 `Arc`
}

impl LogManager {
    /// 创建 LogManager，让 `RollingFileAppender` 作为 `Arc` 存储
    pub fn new(log_dir: &str, buffer_size: usize, batch_size: usize, flush_interval: u64) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let log_file = Arc::new(rolling::hourly(log_dir, "adx_log.json"));

        let manager = Arc::new(Self { sender, log_file: log_file.clone() });

        // 启动异步后台任务处理日志写入
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            Self::background_log_writer(log_file, receiver, batch_size, flush_interval).await;
        });

        manager_clone
    }

    /// 发送日志，接受两个参数：日志级别和日志消息
    /// 内部将二者合并后发送
    pub async fn log(&self, level: impl Into<String>, message: impl Into<String>) {
        let combined_message = format!("{}: {}", level.into(), message.into());
        if let Err(e) = self.sender.send(combined_message).await {
            eprintln!("Failed to send log message: {}", e);
        }
    }

    /// 后台异步日志写入任务
    /// 将通过 mpsc Receiver 接收到的日志批量写入 RollingFileAppender 管理的日志文件中
    async fn background_log_writer(
        log_file: Arc<RollingFileAppender>,
        mut receiver: Receiver<String>,
        batch_size: usize,
        flush_interval: u64
    ) {
        let mut buffer = Vec::new();
        let mut interval = time::interval(Duration::from_millis(flush_interval));

        loop {
            tokio::select! {
                Some(log) = receiver.recv() => {
                    buffer.push(log);
                    if buffer.len() >= batch_size {
                        Self::write_logs_to_disk(log_file.clone(), &mut buffer).await;
                    }
                }
                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        Self::write_logs_to_disk(log_file.clone(), &mut buffer).await;
                    }
                }
            }
        }
    }

    /// 将缓冲区中的日志合并后，通过阻塞任务写入日志文件
    async fn write_logs_to_disk(file: Arc<RollingFileAppender>, buffer: &mut Vec<String>) {
        let content = buffer.join("\n") + "\n";

        let file_clone = Arc::clone(&file);
        task::spawn_blocking(move || {
            let mut writer = file_clone.make_writer();
            if let Err(e) = writer.write_all(content.as_bytes()) {
                eprintln!("Failed to write logs to file: {}", e);
            }
        }).await.unwrap();

        buffer.clear();
    }

    /// 停止日志系统，确保所有日志数据都被写入磁盘
    pub async fn shutdown(&self) {
        // 关闭 sender，让 receiver.recv() 返回 None
        drop(&self.sender);
        // 等待后台日志任务刷盘
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
