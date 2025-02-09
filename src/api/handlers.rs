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
    // 从查询参数中获取 ssp_uuid
    let ssp_uuid = query.ssp_uuid;

    // 从 ConfigManager 获取所有 SSP 广告位配置，并选择匹配 ssp_uuid 的广告位
    let ssp_placement = state.config.get_ssp_placements()
        .into_iter()
        .find(|sp| sp.ssp_uuid == ssp_uuid)
        .expect("No matching SSP placement found");

    // 构造当前请求所属的 SSP 信息
    // 实际场景中，你可能根据 ssp_uuid 在数据库中查询对应的 SSP 信息
    let ssp = Ssp {
        id: 1,  // 示例数据
        uuid: ssp_uuid.clone(),
        name: "Test SSP".to_string(),
        qps: 100,
    };

    // 构造 Context，贯穿整个调用流程
    let context = Context {
        bid_request: bid_request.clone(),
        ssp,
        ssp_placement,
        dsp_requests: vec![], // 后续可以关联 DSP 信息
        start_time: std::time::Instant::now(),
    };

    // 调用竞价处理逻辑
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
