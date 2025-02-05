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
    log_file: Arc<RollingFileAppender>, // ✅ 直接存 `Arc`
}

impl LogManager {
    /// ✅ 让 `RollingFileAppender` 作为 `Arc`
    pub fn new(log_dir: &str, buffer_size: usize, batch_size: usize, flush_interval: u64) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let log_file = Arc::new(rolling::hourly(log_dir, "adx_log.json"));

        let manager = Arc::new(Self { sender, log_file: log_file.clone() });

        // 启动异步后台任务
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            Self::background_log_writer(log_file, receiver, batch_size, flush_interval).await;
        });

        manager_clone
    }

    /// 发送日志
    pub async fn log(&self, message: String) {
        if let Err(e) = self.sender.send(message).await {
            eprintln!("Failed to send log message: {}", e);
        }
    }

    /// ✅ 让 `RollingFileAppender` 以 `Arc` 形式传递
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

    /// ✅ 使用 `Arc<RollingFileAppender>` 进行 `spawn_blocking()`
    async fn write_logs_to_disk(file: Arc<RollingFileAppender>, buffer: &mut Vec<String>) {
        let content = buffer.join("\n") + "\n";

        let file_clone = Arc::clone(&file); // ✅ 让 `RollingFileAppender` 进入 `spawn_blocking()`

        task::spawn_blocking(move || {
            let mut writer = file_clone.make_writer();
            if let Err(e) = writer.write_all(content.as_bytes()) {
                eprintln!("Failed to write logs to file: {}", e);
            }
        }).await.unwrap();

        buffer.clear();
    }

    /// ✅ 停止日志系统，确保数据安全写入磁盘
    pub async fn shutdown(&self) {
        // 关闭 `mpsc::Sender`
        drop(&self.sender); // ✅ 关闭 sender，让 `receiver.recv()` 返回 `None`

        // 确保后台日志线程可以完全处理完消息队列
        tokio::time::sleep(Duration::from_secs(1)).await; // 等待后台日志刷盘
    }
}
