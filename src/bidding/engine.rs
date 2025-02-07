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

/// 辅助函数，根据 adm 内容判断广告类型，并注入 tracking URL
fn inject_tracking(adm: &str, bid_id: &str) -> String {
    if adm.contains("<html") {
        // Banner 广告：在 </body> 之前注入一个隐形的 tracking 像素
        let tracking_pixel = format!(
            "<img src=\"http://tk.rust-adx.com/impression?bid={}\" style=\"display:none;\" />",
            bid_id
        );
        if let Some(pos) = adm.rfind("</body>") {
            let (head, tail) = adm.split_at(pos);
            format!("{}{}{}", head, tracking_pixel, tail)
        } else {
            format!("{}{}", adm, tracking_pixel)
        }
    } else if adm.contains("<VAST") {
        // Video 广告：在 <InLine> 标签后插入 <Impression> 标签
        let impression_tag = format!(
            "<Impression><![CDATA[http://tk.rust-adx.com/impression?bid={}]]></Impression>",
            bid_id
        );
        if let Some(pos) = adm.find("<InLine>") {
            let pos = pos + "<InLine>".len();
            let (head, tail) = adm.split_at(pos);
            format!("{}{}{}", head, impression_tag, tail)
        } else {
            adm.to_string()
        }
    } else {
        // Native 广告：尝试解析 JSON，然后注入 tracking 字段
        if let Ok(mut v) = serde_json::from_str::<Value>(adm) {
            if let Value::Object(ref mut map) = v {
                map.insert(
                    "impression_tracking".to_string(),
                    Value::String(format!("http://tk.rust-adx.com/impression?bid={}", bid_id)),
                );
                map.insert(
                    "click_tracking".to_string(),
                    Value::String(format!("http://tk.rust-adx.com/click?bid={}", bid_id)),
                );
            }
            v.to_string()
        } else {
            // 如果解析失败，则直接返回原始 adm
            adm.to_string()
        }
    }
}

/// 处理竞价请求
/// 参数中增加了 runtime_logger 用于记录整条调用链日志。
pub async fn process_bid_request(
    bid_request: &BidRequest,
    config: &ConfigManager,
    log_manager: &Arc<LogManager>,
    runtime_logger: &Arc<RuntimeLogger>,
) -> Option<BidResponse> {
    // 将 bid_request 包装成 Arc
    let bid_request_arc = Arc::new((*bid_request).clone());
    let dsp_client = DspClient::new(config.active_demands());

    // 聚合各个 DSP 调用的详细信息
    let mut dsp_details = Vec::new();

    // 调用 DSP 获取竞价响应，返回元组包含 dsp_id、url、出价、BidResponse、状态和耗时
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
        log_manager.log("ERROR", log_entry.to_string()).await;
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
        log_manager.log("ERROR", log_entry.to_string()).await;
        winning_bid_opt = None;
    } else {
        valid_responses.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let mut checked_bids = Vec::new();
        for (winning_response, _) in valid_responses {
            for seatbid in winning_response.seatbid {
                for bid in seatbid.bid {
                    // 检查敏感内容
                    if contains_sensitive_content(&bid) {
                        let log_entry = json!({
                            "request_id": bid_request.id,
                            "adx_log": "bid_rejected",
                            "bid_id": bid.id,
                            "reason": "contains_sensitive_content",
                        });
                        log_manager.log("WARN", log_entry.to_string()).await;
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
            log_manager.log("ERROR", log_entry.to_string()).await;
            winning_bid_opt = None;
        } else {
            checked_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
            let winning_bid = checked_bids.first().unwrap().clone();
            adx_result = "success";
            winning_bid_opt = Some(winning_bid);
        }
    }

    // 在返回最终 BidResponse 前，对获胜 Bid 的 adm 字段注入 tracking URL
    let final_bid_response = winning_bid_opt.clone().map(|mut winning_bid| {
        if let Some(adm) = winning_bid.adm.as_ref() {
            let new_adm = inject_tracking(adm, &winning_bid.id);
            winning_bid.adm = Some(new_adm);
        }
        BidResponse {
            id: bid_request.id.clone(),
            seatbid: vec![SeatBid {
                bid: vec![winning_bid.clone()],
                seat: Some("".to_string()),
                group: Some(0),
            }],
            bidid: None,
            cur: Some("USD".to_string()),
            customdata: None,
            nbr: None,
        }
    });

    let aggregated_log = json!({
        "request_id": bid_request.id,
        "adx_inquiry_result": adx_result,
        "winning_bid": winning_bid_opt,
        "dsp_call_details": dsp_details,
    });
    let aggregated_log_str = aggregated_log.to_string();
    runtime_logger.log("INFO", &aggregated_log_str).await;

    final_bid_response
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
