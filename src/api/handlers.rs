use axum::{extract::State, Json};
use std::sync::Arc;
use crate::AppState;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::BidResponse;
use crate::bidding::engine::process_bid_request; // ✅ 确保正确导入

/// 处理 OpenRTB 请求
pub async fn handle_openrtb_request(
    State(state): State<Arc<AppState>>,  // ✅ 提取 `AppState`
    Json(request): Json<BidRequest>,
) -> Json<BidResponse> {
    let log_manager = &state.log_manager;
    let config = &state.config;

    // 这里可以使用 log_manager 记录日志
    log_manager.log(format!("Received OpenRTB request: {:?}", request))
        .await;

    // 处理竞价请求
    let bid_response = process_bid_request(&request, config).await;

    Json(bid_response) // ✅ 直接返回 `BidResponse`，避免 `None`
}
