use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use crate::bidding::engine::process_bid_request;
use crate::config::ConfigManager;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::BidResponse;
use crate::AppState;

/// **处理 OpenRTB 竞价请求**
pub async fn handle_openrtb_request(
    State(state): State<Arc<AppState>>,
    Json(bid_request): Json<BidRequest>,
) -> (StatusCode, Json<BidResponse>) {
    let log_manager = &state.log_manager;
    let config = &state.config;

    // ✅ 调用 `process_bid_request` 进行 DSP 竞价，同时传入 runtime_logger
    let bid_response = process_bid_request(
        &bid_request,
        &state.config,
        &state.log_manager,
        &state.runtime_logger, // 传入 runtime_logger 参数
    ).await;

    match bid_response {
        Some(response) if !response.seatbid.is_empty() => {
            // ✅ **ADX 竞价成功，返回 `BidResponse`**
            log_manager.log(
                "INFO",
                format!(
                    r#"{{ "request_id": "{}", "adx_log": "adx_inquiry_success", "winning_price": {} }}"#,
                    response.id,
                    response.seatbid[0].bid[0].price
                )
            ).await;

            (StatusCode::OK, Json(response))
        }
        _ => {
            // ❌ **ADX 竞价失败，返回空 `BidResponse`**
            log_manager.log(
                "ERROR",
                format!(
                    r#"{{ "request_id": "{}", "adx_log": "adx_inquiry_failed" }}"#,
                    bid_request.id
                )
            ).await;

            (
                StatusCode::NO_CONTENT, // **204 - 无广告可填充**
                Json(BidResponse {
                    id: bid_request.id.clone(),
                    seatbid: vec![],
                    bidid: None,
                    cur: Some("USD".to_string()),
                    customdata: None,
                    nbr: Some(3), // ❌ `3` 表示 "无匹配广告（未填充）"
                }),
            )
        }
    }
}
