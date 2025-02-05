use std::sync::Arc;
use crate::bidding::dsp_client::DspClient;
use crate::config::ConfigManager;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::{Bid, BidResponse, SeatBid};
use tokio::time::Duration;

const SENSITIVE_KEYWORDS: [&str; 2] = ["gambling", "adult"]; // ✅ 敏感关键字

/// ✅ 处理 `BidRequest`，并逐步选取符合条件的最高出价 DSP
pub async fn process_bid_request(
    bid_request: &BidRequest,
    config: &ConfigManager,
) -> BidResponse {
    let tmax = Duration::from_millis(bid_request.tmax.unwrap_or(250));
    let dsp_client = DspClient::new(config.dsp_endpoints.clone());

    // ✅ 获取所有 DSP 的 `BidResponse`
    let bid_responses = dsp_client.fetch_bids(Arc::new((*bid_request).clone())).await;

    for (full_response, _price) in bid_responses {
        // ✅ 检查 `adm` 是否包含敏感关键字
        let mut valid_bids = Vec::new();

        for seat in &full_response.seatbid {
            for bid in &seat.bid {
                if !SENSITIVE_KEYWORDS.iter().any(|kw| bid.adm.contains(kw)) {
                    valid_bids.push(bid.clone());
                }
            }
        }

        // ✅ 如果有有效的 Bid，返回这个 DSP 的竞价结果
        if !valid_bids.is_empty() {
            return BidResponse {
                id: bid_request.id.clone(),
                seatbid: vec![SeatBid { bid: valid_bids }],
            };
        }
    }

    // ✅ 如果没有合适的 DSP 竞价，返回空 `BidResponse`
    BidResponse {
        id: bid_request.id.clone(),
        seatbid: vec![],
    }
}
