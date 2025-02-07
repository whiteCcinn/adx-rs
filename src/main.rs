// main.rs

use axum::{Router, routing::post, serve};
use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use tracing_appender::rolling;
use tokio::net::TcpListener;

mod api;
mod bidding;
mod config;
mod logging;
mod openrtb;
mod mock_dsp;

use api::handlers::handle_openrtb_request;
use config::ConfigManager;
use logging::logger::LogManager;
use logging::runtime_logger::RuntimeLogger; // 引入改进后的多级别 RuntimeLogger

/// 统一的 AppState 结构体
#[derive(Clone)]
pub struct AppState {
    pub log_manager: Arc<LogManager>,
    pub runtime_logger: Arc<RuntimeLogger>, // 运行日志管理器
    pub config: Arc<ConfigManager>,
}

/// ADX 服务的命令行参数
#[derive(Parser, Debug)]
#[command(author = "whiteCcinn", version = "1.0", about = "An OpenRTB-based ADX Server")]
struct CliArgs {
    /// 服务监听端口
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// 日志目录
    #[arg(long, default_value = "logs")]
    log_dir: String,
}

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let args = CliArgs::parse();

    // 初始化 DemandManager（假设 bidding::dsp::init() 返回需要的 DemandManager）
    let demand_manager = bidding::dsp::init();

    // 启动 Mock DSP 服务器
    let dsp_mock_server = tokio::spawn(async {
        mock_dsp::start_mock_dsp_server(9001).await;
    });

    // 初始化 JSON 格式的 adx_log 日志（用于全局 tracing 日志）
    let log_file = rolling::hourly(args.log_dir.clone(), "adx_log.json");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env()) // 支持环境变量配置日志级别
        .with(
            fmt::layer()
                .json() // JSON 格式日志
                .with_writer(non_blocking),
        );
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global tracing subscriber");

    info!("ADX server starting on port {}", args.port);

    // 初始化 RuntimeLogger（多级别版本，需要传入文件前缀，此处使用 "runtime"）
    let runtime_logger = RuntimeLogger::new(
        &args.log_dir,
        "runtime", // 文件前缀，最终文件将形如 runtime_info.json、runtime_debug.json 等
        1000,      // 队列缓冲大小
        100,       // 批量写入条数
        1000,      // 定时写入间隔（毫秒）
    );
    let runtime_logger_clone = runtime_logger.clone();
    runtime_logger_clone
        .log("INFO", "ADX server is starting...")
        .await;

    // 初始化 ConfigManager
    let config = Arc::new(ConfigManager::new(demand_manager));

    // 初始化 LogManager（业务日志）
    let log_manager = LogManager::new(
        &args.log_dir,
        1000, // 队列缓冲大小
        100,  // 批量写入条数
        1000, // 定时写入间隔（毫秒）
    );

    // 创建 AppState
    let state = Arc::new(AppState {
        log_manager: log_manager.clone(),
        runtime_logger: runtime_logger.clone(),
        config: config.clone(),
    });

    // 启动 ADX 服务器
    let adx_server = tokio::spawn(async move {
        // 创建路由
        let app = Router::new()
            .route("/openrtb", post(handle_openrtb_request))
            .with_state(state.clone());

        let addr = format!("0.0.0.0:{}", args.port);
        runtime_logger
            .log("INFO", &format!("ADX server running at http://{}", addr))
            .await;

        let listener = TcpListener::bind(&addr).await.unwrap();
        serve(listener, app).await.unwrap();
    });

    // 监听 SIGTERM / SIGINT 信号，确保优雅退出
    signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");

    runtime_logger_clone
        .log("INFO", "Shutting down gracefully...")
        .await;

    // 刷新业务日志
    log_manager.shutdown().await;
    runtime_logger_clone
        .log("INFO", "ADX logs flushed before shutdown")
        .await;

    // 等待 ADX 服务器和 DSP Mock 服务器关闭
    tokio::try_join!(adx_server, dsp_mock_server).unwrap();

    runtime_logger_clone.log("INFO", "ADX server shut down.").await;
}
