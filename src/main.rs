use axum::{Router, routing::post, serve};
use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use tracing_appender::rolling;

mod api;
mod bidding;
mod config;
mod logging;
mod openrtb;
mod mock_dsp;

use api::handlers::handle_openrtb_request;
use config::ConfigManager;
use logging::logger::LogManager;
use tokio::net::TcpListener;

/// 统一的 `AppState` 结构体
#[derive(Clone)]
pub struct AppState {
    pub log_manager: Arc<LogManager>,
    pub config: Arc<ConfigManager>,
}

/// ADX 服务的命令行参数
#[derive(Parser, Debug)]
#[command(author = "Your Name", version = "1.0", about = "An OpenRTB-based ADX Server")]
struct CliArgs {
    /// DSP 服务列表（逗号分隔）
    #[arg(short, long, default_value = "http://dsp1.com/bid,http://dsp2.com/bid")]
    dsp_endpoints: String,

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

    // 解析 DSP 端点，并包含 Mock DSP 端口
    let mut dsp_endpoints: Vec<String> = args.dsp_endpoints
        .split(',')
        .map(|s| s.to_string())
        .collect();
    dsp_endpoints.push("http://127.0.0.1:9001/bid".to_string());  // ✅ 加入 Mock DSP

    // 启动 Mock DSP 服务器
    let dsp_mock_server = tokio::spawn(async {
        mock_dsp::start_mock_dsp_server(9001).await;
    });


    // 初始化 JSON 格式日志
    let log_file = rolling::hourly(args.log_dir.clone(), "adx_log.json");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env()) // 支持环境变量配置日志级别
        .with(
            fmt::layer()
                .json() // JSON 格式日志
                .with_writer(non_blocking), // 写入文件
        );

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global tracing subscriber");

    info!("ADX server starting on port {}", args.port);

    // 初始化配置
    let config = Arc::new(ConfigManager::new(dsp_endpoints));

    // 初始化日志管理器
    let log_manager = LogManager::new(
        &args.log_dir,
        1000, // 队列缓冲大小
        100,  // 批量写入条数
        1000, // 定时写入间隔（毫秒）
    );

    // 创建 `AppState`
    let state = Arc::new(AppState {
        log_manager: log_manager.clone(),
        config: config.clone(),
    });

    // 启动 ADX 服务器
    let adx_server = tokio::spawn(async move {
        // 创建路由
        let app = Router::new()
            .route("/openrtb", post(handle_openrtb_request))
            .with_state(state.clone());

        let addr = format!("0.0.0.0:{}", args.port);
        info!("ADX server running at http://{}", addr);
        let listener = TcpListener::bind(&addr).await.unwrap();
        serve(listener, app).await.unwrap();
    });

    // 监听 SIGTERM / SIGINT 信号，确保优雅退出
    signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");

    info!("Shutting down gracefully...");

    // 先刷新日志再退出
    log_manager.shutdown().await;
    // 关闭 ADX 和 DSP Mock
    tokio::try_join!(adx_server, dsp_mock_server).unwrap();

    info!("ADX server shut down.");
}
