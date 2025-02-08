// openrtb/response.rs

use serde::{Serialize, Deserialize};

/// **Top-level OpenRTB Bid Response（竞价响应）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BidResponse {
    pub id: String,               // 与 BidRequest 对应的 ID
    pub seatbid: Vec<SeatBid>,    // DSP 返回的 SeatBid（竞价广告列表）
    pub bidid: Option<String>,    // DSP 生成的竞价 ID（可选）
    pub cur: Option<String>,      // 竞价的货币类型（如 USD, CNY）
    pub customdata: Option<String>, // DSP 返回的自定义数据
    pub nbr: Option<i32>,         // 竞价失败原因代码（仅在未填充广告时返回）
}

/// **SeatBid（DSP 返回的竞价广告列表）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeatBid {
    pub bid: Vec<Bid>,           // 竞价广告的具体信息
    pub seat: Option<String>,    // DSP 的席位 ID（Seat ID）
    pub group: Option<i32>,      // 是否组合竞价（1 = 是，0 = 否）
}

/// **Bid（具体的竞价信息）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bid {
    pub id: String,               // 竞价 ID（DSP 生成）
    pub impid: String,            // 对应的 Impression ID
    pub price: f64,               // 竞价价格（货币单位同 `BidResponse.cur`）
    pub nurl: Option<String>,     // 点击时通知 DSP 的 URL
    pub adm: Option<String>,      // 广告物料（HTML、VAST XML、原生 JSON）
    pub adid: Option<String>,     // DSP 生成的广告 ID
    pub adomain: Option<Vec<String>>, // 广告主域名（如 ["example.com"]）
    pub cid: Option<String>,      // DSP 生成的 Campaign ID（广告系列 ID）
    pub crid: Option<String>,     // DSP 生成的 Creative ID（创意 ID）
    pub cat: Option<Vec<String>>, // 广告类别（IAB 分类）
    pub attr: Option<Vec<i32>>,   // 广告属性（如自动播放、可跳过等）
    pub dealid: Option<String>,   // 与 PMP 交易对应的 Deal ID
    pub h: Option<i32>,           // 广告高度（像素）
    pub w: Option<i32>,           // 广告宽度（像素）
    pub ext: Option<serde_json::Value>, // 额外的扩展字段
}
