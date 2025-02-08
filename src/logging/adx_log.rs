use serde::{Serialize, Deserialize};
use chrono::{FixedOffset, TimeZone, Utc};
use serde_json::to_string;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdxLog {
    pub timestamp: String,
    pub log_type: String,
    pub ssp_uuid: String,
    pub request_id: String,
    pub bid_attempts: usize,
    pub status: String,
    pub winning_dsp: Option<String>,
    pub winning_price: f64,
    pub dsp_bidding_log: Vec<DspBidLog>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DspBidLog {
    pub dsp_name: String,
    pub dsp_url: String,
    pub bid_price: f64,
    pub status: String,
}

impl AdxLog {
    pub fn new(ssp_uuid: &str, request_id: &str) -> Self {
        let tz = FixedOffset::east(8 * 3600);
        Self {
            timestamp: tz.from_utc_datetime(&Utc::now().naive_utc()).to_rfc3339(),
            log_type: "adx_bid_request".to_string(),
            ssp_uuid: ssp_uuid.to_string(),
            request_id: request_id.to_string(),
            bid_attempts: 0,
            status: "failure".to_string(),
            winning_dsp: None,
            winning_price: 0.0,
            dsp_bidding_log: Vec::new(),
        }
    }

    pub fn add_dsp_bid_log(&mut self, dsp_name: &str, dsp_url: &str, bid_price: f64, status: &str) {
        self.dsp_bidding_log.push(DspBidLog {
            dsp_name: dsp_name.to_string(),
            dsp_url: dsp_url.to_string(),
            bid_price,
            status: status.to_string(),
        });
        self.bid_attempts += 1;
    }

    pub fn set_winner(&mut self, dsp_name: &str, price: f64) {
        self.status = "success".to_string();
        self.winning_dsp = Some(dsp_name.to_string());
        self.winning_price = price;
    }
}

/// 将传入的日志内容写入指定文件（例如 "logs/adx_log.json"），
/// 使用同步 I/O 保证文件写入操作不会因异步兼容性问题而报错。
pub fn write_adx_log(log_content: &str, log_file: &str) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file)
        .expect("Unable to open adx_log file");
    let mut writer = BufWriter::new(file);
    // 注意这里传入 &mut writer
    writeln!(&mut writer, "{}", log_content).expect("Unable to write adx_log");
}

/// 生成调用链日志，并写入 adx_log 文件
pub fn log_adx_call_chain(aggregated_log: &serde_json::Value) {
    let tz = FixedOffset::east(8 * 3600);
    let timestamp = tz.from_utc_datetime(&Utc::now().naive_utc()).to_string();
    let log_entry = serde_json::json!({
        "timestamp": timestamp,
        "call_chain": aggregated_log
    });
    let log_str = to_string(&log_entry).unwrap();
    write_adx_log(&log_str, "logs/adx_log.json");
}
