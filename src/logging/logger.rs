// src/logging/logger.rs

use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::time::{self, Duration};
use tokio::task;
use std::io::Write;
use tracing_appender::rolling;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::fmt::MakeWriter;
use serde_json::json;
use chrono::Utc;

pub struct LogManager {
    sender: Sender<String>,
    log_file: Arc<RollingFileAppender>,
}

impl LogManager {
    pub fn new(log_dir: &str, buffer_size: usize, batch_size: usize, flush_interval: u64) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let log_file = Arc::new(rolling::hourly(log_dir, "adx_log.json"));
        let manager = Arc::new(Self { sender, log_file: log_file.clone() });
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            Self::background_log_writer(log_file, receiver, batch_size, flush_interval).await;
        });
        manager_clone
    }

    pub async fn log(&self, level: impl Into<String>, message: impl Into<String>) {
        let log_entry = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "message": message.into()
        })
            .to_string();

        if let Err(e) = self.sender.send(log_entry).await {
            eprintln!("Failed to send log message: {}", e);
        }
    }

    async fn background_log_writer(
        log_file: Arc<RollingFileAppender>,
        mut receiver: Receiver<String>,
        batch_size: usize,
        flush_interval: u64,
    ) {
        let mut buffer = Vec::new();
        let mut interval = time::interval(Duration::from_millis(flush_interval));
        loop {
            tokio::select! {
                Some(log) = receiver.recv() => {
                    buffer.push(log);
                    if buffer.len() >= batch_size {
                        Self::write_logs_to_disk(log_file.clone(), &buffer).await;
                        buffer.clear();
                    }
                }
                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        Self::write_logs_to_disk(log_file.clone(), &buffer).await;
                        buffer.clear();
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
                eprintln!("Failed to write logs to file: {}", e);
            }
        })
            .await
            .unwrap();
    }

    pub async fn shutdown(&self) {
        drop(&self.sender);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
