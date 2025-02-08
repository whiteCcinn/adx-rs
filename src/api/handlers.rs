// src/api/handlers.rs

use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use crate::bidding::engine::process_bid_request;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::BidResponse;
use crate::AppState;

/// **处理 OpenRTB 竞价请求**
pub async fn handle_openrtb_request(
    State(state): State<Arc<AppState>>,
    Json(bid_request): Json<BidRequest>,
) -> (StatusCode, Json<BidResponse>) {
    let runtime_logger = &state.runtime_logger;

    // 调用 process_bid_request 进行 DSP 竞价，并传入 runtime_logger 和 config
    let bid_response = process_bid_request(
        &bid_request,
        &state.config,
        &state.runtime_logger,  // 使用 runtime_logger 记录业务调用链日志
    ).await;

    match bid_response {
        Some(response) if !response.seatbid.is_empty() => {
            runtime_logger.log("INFO", &format!(
                r#"{{ "request_id": "{}", "adx_log": "adx_inquiry_success", "winning_price": {} }}"#,
                response.id,
                response.seatbid[0].bid[0].price
            )).await;
            (StatusCode::OK, Json(response))
        }
        _ => {
            runtime_logger.log("ERROR", &format!(
                r#"{{ "request_id": "{}", "adx_log": "adx_inquiry_failed" }}"#,
                bid_request.id
            )).await;
            (
                StatusCode::NO_CONTENT,
                Json(BidResponse {
                    id: bid_request.id.clone(),
                    seatbid: vec![],
                    bidid: None,
                    cur: Some("USD".to_string()),
                    customdata: None,
                    nbr: Some(3),
                }),
            )
        }
    }
}
