// src/bidding/engine.rs

use std::sync::Arc;
use tokio::time::Duration;
use serde_json::{json, Value};
use tracing::info;
use std::time::Instant;

use crate::bidding::dsp_client::DspClient;
use crate::config::config_manager::ConfigManager;
use crate::logging::runtime_logger::RuntimeLogger;
use crate::openrtb::response::{Bid, BidResponse, SeatBid};
use crate::model::context::Context;

/// 辅助函数，根据 DSP 下发的 adm 内容生成 ADX 注入的 SSP tracking 部分（保留 {AUCTION_PRICE} 占位符）
fn generate_ssp_tracking(adm: &str) -> String {
    if adm.contains("<html") {
        "<img src=\"http://tk.rust-adx.com/impression?price={AUCTION_PRICE}\" style=\"display:none;\" />".to_string()
    } else if adm.contains("<VAST") {
        "<Impression><![CDATA[http://tk.rust-adx.com/impression?price={AUCTION_PRICE}]]></Impression>".to_string()
    } else if adm.trim_start().starts_with("{") {
        "".to_string() // native 类型不额外注入
    } else {
        "".to_string()
    }
}

/// 处理竞价请求，参数为 Context，贯穿整个调用链的信息
pub async fn process_bid_request(
    context: &Context,
    config: &ConfigManager,
    runtime_logger: &Arc<RuntimeLogger>,
) -> Option<BidResponse> {
    let bid_request = &context.bid_request;
    let dsp_client = DspClient::new(config.active_demands());
    let mut dsp_details = Vec::new();
    let bid_responses = dsp_client.fetch_bids(&Arc::new(bid_request.clone())).await;
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
        runtime_logger.log("ERROR", &log_entry.to_string()).await;
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
        runtime_logger.log("ERROR", &log_entry.to_string()).await;
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
                        runtime_logger.log("WARN", &log_entry.to_string()).await;
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
            runtime_logger.log("ERROR", &log_entry.to_string()).await;
            winning_bid_opt = None;
        } else {
            checked_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
            let mut winning_bid = checked_bids.first().unwrap().clone();
            adx_result = "success";
            let original_price = winning_bid.price;
            let final_price = original_price * 0.8; // 扣除20%利润后的价格

            // 先替换 DSP 下发的 offer 中的 {AUCTION_PRICE} 占位符为 final_price，
            // 然后生成 ADX 注入的 SSP tracking（其中 tracking URL 保留 {AUCTION_PRICE} 占位符），并追加
            if let Some(original_adm) = winning_bid.adm.as_ref() {
                let dsp_adm_processed = original_adm.replace("{AUCTION_PRICE}", &final_price.to_string());
                let ssp_tracking = generate_ssp_tracking(original_adm);
                let final_adm = format!("{}{}", dsp_adm_processed, ssp_tracking);
                winning_bid.adm = Some(final_adm);
            }
            let price_info = json!({
                "original_price": original_price,
                "final_price": final_price
            });
            dsp_details.push(price_info);
            winning_bid_opt = Some(winning_bid);
        }
    }

    // 记录整个调用链耗时，并判断是否超过 bid_request.tmax
    let elapsed_total = context.start_time.elapsed();
    if let Some(tmax) = bid_request.tmax {
        if elapsed_total > Duration::from_millis(tmax) {
            runtime_logger.log("WARN", &format!(
                "Processing time {} ms exceeded tmax {} ms",
                elapsed_total.as_millis(),
                tmax
            )).await;
        }
    }

    let aggregated_log = json!({
        "request_id": bid_request.id,
        "adx_inquiry_result": adx_result,
        "winning_bid": winning_bid_opt,
        "dsp_call_details": dsp_details,
        "elapsed_time_ms": elapsed_total.as_millis(),
    });
    runtime_logger.log("INFO", &aggregated_log.to_string()).await;

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

fn contains_sensitive_content(bid: &Bid) -> bool {
    let content = format!(
        "{} {}",
        bid.adm.as_ref().map(|s| s.as_str()).unwrap_or(""),
        bid.crid.as_deref().unwrap_or("")
    );
    let sensitive_keywords = ["forbidden", "banned", "restricted"];
    sensitive_keywords.iter().any(|&word| content.contains(word))
}
