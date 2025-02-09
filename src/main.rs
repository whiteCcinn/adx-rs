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
use model::context::Context;
use crate::model::adapters::ConfigAdapter;

#[derive(Clone)]
pub struct AppState {
    pub runtime_logger: Arc<RuntimeLogger>,
    pub config: Arc<ConfigManager>,
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

    // 初始化运行日志记录器（用于记录服务运行状态、调试、错误等）
    let runtime_logger = RuntimeLogger::new(&args.log_dir, "runtime", 1000, 100, 1000);
    runtime_logger.log("INFO", "ADX server is starting...").await;

    // 初始化 ConfigManager，并使用 FileConfigAdapter 从 /static 目录读取广告位配置
    let adapter = FileConfigAdapter::new("static/ssp_placements.json", "static/dsp_placements.json");
    let config = Arc::new(ConfigManager::new(demand_manager));
    config.update_placements(adapter.get_ssp_placements(), adapter.get_dsp_placements());

    // 读取 ssp_info.json，得到所有 SSP 基础信息
    let ssp_info_str = fs::read_to_string("static/ssp_info.json")
        .expect("Unable to read ssp_info.json");
    let ssp_vec: Vec<Ssp> = serde_json::from_str(&ssp_info_str)
        .expect("Unable to parse ssp_info.json");

    // 假设请求中有 SSP 的标识（例如 ssp_uuid），这里示例默认选择第一个 SSP
    let ssp = ssp_vec.first().cloned().expect("No SSP info available");

    // 从 ConfigManager 中获取 SSP 广告位配置（假设返回的是 Vec<SspPlacement>）
    let ssp_placements = config.get_ssp_placements();
    // 根据 SSP 的 uuid 进行匹配，假设 ssp.uuid 与 SspPlacement.ssp_uuid 对应
    let ssp_placement = ssp_placements.into_iter()
        .find(|sp| sp.ssp_uuid == ssp.uuid)
        .expect("No matching SSP placement found");

    // 构造 Context，注意 ssp_placement 为单个广告位信息
    let context = Context {
        bid_request: openrtb::request::BidRequest {
            id: "test-request-001".to_string(),
            imp: Vec::new(), // 实际场景中填充广告展示请求
            site: None,
            app: None,
            device: None,
            user: None,
            test: None,
            at: None,
            tmax: None,
            wseat: None,
            bseat: None,
            allimps: None,
            cur: None,
            wlang: None,
            bcat: None,
            badv: None,
            source: None,
            regs: None,
        },
        ssp,
        ssp_placement,
        dsp_requests: Vec::new(), // 后续可以根据 active_dsps 和 dsp_placements 进行关联构造
        start_time: std::time::Instant::now(),
    };

    let state = Arc::new(AppState {
        runtime_logger: runtime_logger.clone(),
        config: config.clone(),
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
