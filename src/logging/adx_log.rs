use serde::{Serialize, Deserialize};
use chrono::Utc;

/// **ADX 询价日志**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdxLog {
    pub timestamp: String,      // 记录时间
    pub log_type: String,       // 日志类型，如 "adx_bid_request"
    pub ssp_uuid: String,       // SSP 唯一标识
    pub request_id: String,     // OpenRTB `BidRequest.id`
    pub bid_attempts: usize,    // DSP 竞价次数
    pub status: String,         // 竞价结果 "success" or "failure"
    pub winning_dsp: Option<String>,  // 竞价胜出的 DSP
    pub winning_price: f64,     // 竞价胜出的价格
    pub dsp_bidding_log: Vec<DspBidLog>, // DSP 竞价日志
}

/// **DSP 竞价日志**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DspBidLog {
    pub dsp_name: String,       // DSP 名称
    pub dsp_url: String,        // DSP 请求地址
    pub bid_price: f64,         // DSP 竞价价格
    pub status: String,         // "success", "timeout", "no_fill"
}

impl AdxLog {
    /// **创建 `ADX` 询价日志**
    pub fn new(ssp_uuid: &str, request_id: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            log_type: "adx_bid_request".to_string(),
            ssp_uuid: ssp_uuid.to_string(),
            request_id: request_id.to_string(),
            bid_attempts: 0,
            status: "failure".to_string(),  // 默认失败，后续可更新
            winning_dsp: None,
            winning_price: 0.0,
            dsp_bidding_log: Vec::new(),
        }
    }

    /// **添加 DSP 竞价日志**
    pub fn add_dsp_bid_log(&mut self, dsp_name: &str, dsp_url: &str, bid_price: f64, status: &str) {
        self.dsp_bidding_log.push(DspBidLog {
            dsp_name: dsp_name.to_string(),
            dsp_url: dsp_url.to_string(),
            bid_price,
            status: status.to_string(),
        });
        self.bid_attempts += 1;
    }

    /// **设置竞价胜出 DSP**
    pub fn set_winner(&mut self, dsp_name: &str, price: f64) {
        self.status = "success".to_string();
        self.winning_dsp = Some(dsp_name.to_string());
        self.winning_price = price;
    }
}
