// src/main.rs

use axum::{Router, routing::post, serve};
use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing::{info};
use tracing_subscriber::{fmt, EnvFilter, Registry};
use tracing_appender::rolling;
use tokio::net::TcpListener;
use std::fs;
use tracing_subscriber::layer::SubscriberExt;

mod api;
mod bidding;
mod config;
mod logging;
mod model;
mod openrtb;
mod mock_dsp;

use api::handlers::handle_openrtb_request;
use config::config_manager::ConfigManager;
use logging::runtime_logger::RuntimeLogger;
use model::adapters::FileConfigAdapter;
use model::dsp::init as dsp_init;
use model::ssp::Ssp;
use crate::model::adapters::ConfigAdapter;

#[derive(Clone)]
pub struct AppState {
    pub runtime_logger: Arc<RuntimeLogger>,
    pub config: Arc<ConfigManager>,
    pub ssp_info: Vec<Ssp>,
}

#[derive(Parser, Debug)]
#[command(author = "whiteCcinn", version = "1.0", about = "An OpenRTB-based ADX Server")]
struct CliArgs {
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value = "logs")]
    log_dir: String,
}

#[tokio::main]
async fn main() {
    // 设置环境变量 TZ 为东八区
    std::env::set_var("TZ", "Asia/Shanghai");

    let args = CliArgs::parse();

    // 初始化 DSP 基础信息
    let demand_manager = dsp_init();

    // 启动 Mock DSP 服务器（监听 9001 端口）
    let dsp_mock_server = tokio::spawn(async {
        mock_dsp::start_mock_dsp_server(9001).await;
    });

    // 初始化全局 tracing 日志
    let log_file = rolling::hourly(&args.log_dir, "adx_log.json");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().json().with_writer(non_blocking));
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global tracing subscriber");
    info!("ADX server starting on port {}", args.port);

    // 初始化运行日志记录器
    let runtime_logger = RuntimeLogger::new(&args.log_dir, "runtime", 1000, 100, 1000);
    runtime_logger.log("INFO", "ADX server is starting...").await;

    // 初始化 ConfigManager，并使用 FileConfigAdapter 从 /static 目录读取 SSP 广告位和 DSP 广告位配置
    let adapter = FileConfigAdapter::new("static/ssp_placements.json", "static/dsp_placements.json", "static/ssp_info.json");
    let config = Arc::new(ConfigManager::new(demand_manager));
    config.update_placements(adapter.get_ssp_placements(), adapter.get_dsp_placements());

    // 从 FileConfigAdapter 中读取 SSP 基础信息（多个 SSP）
    let ssp_info = adapter.get_ssp_info();

    // 构造全局状态 AppState，其中不在 main.rs 中构造 Context，
    // 而在 API Handler 中根据请求中的参数构造具体的 Context。
    let state = Arc::new(AppState {
        runtime_logger: runtime_logger.clone(),
        config: config.clone(),
        ssp_info,
    });

    let adx_server = tokio::spawn({
        let state = state.clone();
        let port = args.port;
        let runtime_logger = runtime_logger.clone();
        async move {
            let app = Router::new()
                .route("/openrtb", post(api::handlers::handle_openrtb_request))
                .with_state(state);
            let addr = format!("0.0.0.0:{}", port);
            runtime_logger.log("INFO", &format!("ADX server running at http://{}", addr)).await;
            let listener = TcpListener::bind(&addr).await.unwrap();
            serve(listener, app).await.unwrap();
        }
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            runtime_logger.log("INFO", "Shutting down gracefully...").await;
        }
    }

    runtime_logger.shutdown().await;
    tokio::try_join!(adx_server, dsp_mock_server).unwrap();
    runtime_logger.log("INFO", "ADX server shut down.").await;
}
