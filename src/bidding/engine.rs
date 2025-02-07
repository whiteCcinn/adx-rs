use std::sync::Arc;
use tokio::time::Duration;
use serde_json::{json, Value};
use tracing::info;

use crate::bidding::dsp_client::DspClient;
use crate::config::config_manager::ConfigManager;
use crate::logging::logger::LogManager;
use crate::logging::runtime_logger::RuntimeLogger;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::{Bid, BidResponse, SeatBid};

/// 辅助函数，根据 DSP 下发的 adm 内容注入 SSP tracking 信息，保留 {AUCTION_PRICE} 占位符。
fn inject_ssp_tracking(adm: &str) -> String {
    // Banner 类型（HTML）
    if adm.contains("<html") {
        let ssp_tag = "<img src=\"http://tk.rust-adx.com/impression?price={AUCTION_PRICE}\" style=\"display:none;\" />";
        if let Some(pos) = adm.rfind("</body>") {
            let (head, tail) = adm.split_at(pos);
            format!("{}{}{}", head, ssp_tag, tail)
        } else {
            format!("{}{}", adm, ssp_tag)
        }
    }
    // Video 类型（VAST XML）
    else if adm.contains("<VAST") {
        let ssp_tag = "<Impression><![CDATA[http://tk.rust-adx.com/impression?price={AUCTION_PRICE}]]></Impression>";
        if let Some(pos) = adm.find("<InLine>") {
            let pos = pos + "<InLine>".len();
            let (head, tail) = adm.split_at(pos);
            format!("{}{}{}", head, ssp_tag, tail)
        } else {
            adm.to_string()
        }
    }
    // Native 类型（JSON）
    else if adm.trim_start().starts_with("{") {
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(adm) {
            if let Value::Object(ref mut map) = v {
                map.insert(
                    "ssp_impression_tracking".to_string(),
                    Value::String("http://tk.rust-adx.com/impression?price={AUCTION_PRICE}".to_string()),
                );
                map.insert(
                    "ssp_click_tracking".to_string(),
                    Value::String("http://tk.rust-adx.com/click?price={AUCTION_PRICE}".to_string()),
                );
            }
            v.to_string()
        } else {
            adm.to_string()
        }
    }
    // 默认直接返回原始 adm
    else {
        adm.to_string()
    }
}

/// 处理竞价请求
/// 在 DSP 调用后，对返回结果进行聚合处理，选出获胜竞价后：
/// 1. 记录 DSP 原始价格和扣除利润后的最终价格；
/// 2. 替换 DSP 下发的 offer（adm 字段）中原始部分的 {AUCTION_PRICE} 占位符为最终价格；
/// 3. 注入 ADX 自身的 SSP tracking 信息（tracking URL中仍保留 {AUCTION_PRICE} 占位符）。
pub async fn process_bid_request(
    bid_request: &BidRequest,
    config: &ConfigManager,
    log_manager: &Arc<LogManager>,
    runtime_logger: &Arc<RuntimeLogger>,
) -> Option<BidResponse> {
    let bid_request_arc = Arc::new(bid_request.clone());
    let dsp_client = DspClient::new(config.active_demands());

    let mut dsp_details = Vec::new();
    let bid_responses = dsp_client.fetch_bids(&bid_request_arc).await;

    let mut valid_responses = Vec::new();
    let mut failed_dsp_logs = Vec::new();

    for (dsp_id, dsp_url, price, bid_response, status, elapsed) in bid_responses {
        let detail = json!({
            "dsp_id": dsp_id,
            "url": dsp_url,
            "bid_price": price,
            "result": status,
            "inquiry_time_ms": elapsed,
            "failure_reason": if status == "success" { Value::Null } else { json!(status) }
        });
        dsp_details.push(detail);

        if let Some(nbr) = bid_response.nbr {
            failed_dsp_logs.push(json!({
                "dsp_id": dsp_id,
                "url": dsp_url,
                "nbr": nbr,
                "result": status,
                "inquiry_time_ms": elapsed,
            }).to_string());
            continue;
        }
        if bid_response.seatbid.is_empty() {
            failed_dsp_logs.push(json!({
                "dsp_id": dsp_id,
                "url": dsp_url,
                "reason": "no_seatbid",
                "result": status,
                "inquiry_time_ms": elapsed,
            }).to_string());
            continue;
        }
        valid_responses.push((bid_response, price));
    }

    if !failed_dsp_logs.is_empty() {
        let log_entry = json!({
            "request_id": bid_request.id,
            "adx_log": "dsp_inquiry_failed",
            "details": failed_dsp_logs,
        });
        log_manager.log("ERROR", &log_entry.to_string()).await;
    }

    let adx_result;
    let winning_bid_opt;

    if valid_responses.is_empty() {
        adx_result = "failed";
        let log_entry = json!({
            "request_id": bid_request.id,
            "adx_log": "adx_inquiry_failed",
            "reason": "all_dsp_failed",
        });
        log_manager.log("ERROR", &log_entry.to_string()).await;
        winning_bid_opt = None;
    } else {
        valid_responses.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let mut checked_bids = Vec::new();
        for (winning_response, _) in valid_responses {
            for seatbid in winning_response.seatbid {
                for bid in seatbid.bid {
                    if contains_sensitive_content(&bid) {
                        let log_entry = json!({
                            "request_id": bid_request.id,
                            "adx_log": "bid_rejected",
                            "bid_id": bid.id,
                            "reason": "contains_sensitive_content",
                        });
                        log_manager.log("WARN", &log_entry.to_string()).await;
                        continue;
                    }
                    checked_bids.push(bid.clone());
                }
            }
        }
        if checked_bids.is_empty() {
            adx_result = "failed";
            let log_entry = json!({
                "request_id": bid_request.id,
                "adx_log": "adx_inquiry_failed",
                "reason": "all_bids_filtered",
            });
            log_manager.log("ERROR", &log_entry.to_string()).await;
            winning_bid_opt = None;
        } else {
            checked_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
            let mut winning_bid = checked_bids.first().unwrap().clone();
            adx_result = "success";
            let original_price = winning_bid.price;
            let final_price = original_price * 0.8; // 扣除20%利润后的价格

            // 先对 DSP 下发的原始 adm（不包含 ADX tracking）进行替换，将其中的 {AUCTION_PRICE} 替换为 final_price
            if let Some(original_adm) = winning_bid.adm.as_ref() {
                let dsp_adm_replaced = original_adm.replace("{AUCTION_PRICE}", &final_price.to_string());
                // 然后注入 SSP tracking（此部分 tracking 中仍保留 {AUCTION_PRICE} 占位符）
                let adm_with_tracking = inject_ssp_tracking(&dsp_adm_replaced);
                winning_bid.adm = Some(adm_with_tracking);
            }
            let price_info = json!({
                "original_price": original_price,
                "final_price": final_price
            });
            dsp_details.push(price_info);
            winning_bid_opt = Some(winning_bid);
        }
    }

    let aggregated_log = json!({
        "request_id": bid_request.id,
        "adx_inquiry_result": adx_result,
        "winning_bid": winning_bid_opt,
        "dsp_call_details": dsp_details,
    });
    let aggregated_log_str = aggregated_log.to_string();
    runtime_logger.log("INFO", &aggregated_log_str).await;

    winning_bid_opt.map(|winning_bid| {
        BidResponse {
            id: bid_request.id.clone(),
            seatbid: vec![SeatBid {
                bid: vec![winning_bid],
                seat: Some("".to_string()),
                group: Some(0),
            }],
            bidid: None,
            cur: Some("USD".to_string()),
            customdata: None,
            nbr: None,
        }
    })
}

/// 检查 Bid 是否包含敏感内容
fn contains_sensitive_content(bid: &Bid) -> bool {
    let content = format!(
        "{} {}",
        bid.adm.as_ref().map(|s| s.as_str()).unwrap_or(""),
        bid.crid.as_deref().unwrap_or("")
    );
    let sensitive_keywords = ["forbidden", "banned", "restricted"];
    sensitive_keywords.iter().any(|&word| content.contains(word))
}
