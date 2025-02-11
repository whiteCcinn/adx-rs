// src/api/handlers.rs

use axum::{extract::{State, Query}, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;
use crate::bidding::engine::process_bid_request;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::BidResponse;
use crate::model::context::Context;
use crate::model::ssp::Ssp;
use crate::AppState;

#[derive(Deserialize)]
pub struct SspQuery {
    pub ssp_uuid: String,
}

pub async fn handle_openrtb_request(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SspQuery>,
    Json(bid_request): Json<BidRequest>,
) -> (StatusCode, Json<BidResponse>) {
    // 通过查询参数获取 ssp_uuid
    let ssp_uuid = query.ssp_uuid;

    // 在全局 SSP 信息列表中查找匹配的 SSP
    let ssp = state.ssp_info.iter()
        .find(|s| s.uuid == ssp_uuid)
        .cloned()
        .expect("No matching SSP found");

    // 在 ConfigManager 中查找 SSP 广告位
    let ssp_placement = state.config.get_ssp_placements()
        .into_iter()
        .find(|sp| sp.ssp_uuid == ssp.uuid)
        .expect("No matching SSP placement found");

    // 构造 Context（贯穿整个调用链），由 API Handler 构造
    let context = Context {
        bid_request: bid_request.clone(),
        ssp,
        ssp_placement,
        dsp_requests: vec![], // 后续可构造 DSP 请求信息
        start_time: std::time::Instant::now(),
    };

    let bid_response = process_bid_request(&context, &state.config, &state.runtime_logger).await;

    match bid_response {
        Some(response) if !response.seatbid.is_empty() => {
            state.runtime_logger.log("INFO", &format!(
                r#"{{ "request_id": "{}", "adx_log": "adx_inquiry_success", "winning_price": {} }}"#,
                response.id,
                response.seatbid[0].bid[0].price
            )).await;
            (StatusCode::OK, Json(response))
        }
        _ => {
            state.runtime_logger.log("ERROR", &format!(
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
