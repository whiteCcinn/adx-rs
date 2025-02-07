use serde::{Deserialize, Serialize};

/// **Top-level OpenRTB Bid Request**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BidRequest {
    pub id: String,                 // 请求 ID，每个竞价请求唯一
    pub imp: Vec<Imp>,              // 广告展示请求（Impression）列表
    pub site: Option<Site>,         // 网站信息（如果请求来源是 Web）
    pub app: Option<App>,           // 应用信息（如果请求来源是 App）
    pub device: Option<Device>,     // 设备信息（用户的浏览器、IP、设备 ID）
    pub user: Option<User>,         // 用户信息
    pub test: Option<i32>,          // 是否是测试请求（1 = 测试模式, 0 = 真实竞价）
    pub at: Option<i32>,            // 竞价模式（1 = 第一价格拍卖, 2 = 第二价格拍卖）
    pub tmax: Option<u64>,          // 竞价超时时间（毫秒）
    pub wseat: Option<Vec<String>>, // 允许的 DSP 供应商列表
    pub bseat: Option<Vec<String>>, // 屏蔽的 DSP 供应商列表
    pub allimps: Option<i32>,       // 是否对所有广告位都需要出价（1 = 是, 0 = 否）
    pub cur: Option<Vec<String>>,   // 允许的货币（如 USD, CNY）
    pub wlang: Option<Vec<String>>, // 允许的语言（ISO 639-1 格式）
    pub bcat: Option<Vec<String>>,  // 屏蔽的广告类别（IAB 分类）
    pub badv: Option<Vec<String>>,  // 屏蔽的广告主域名
    pub source: Option<Source>,     // 竞价请求来源信息
    pub regs: Option<Regs>,         // 隐私法规信息（如 GDPR、CCPA）
}

/// **Impression（广告展示请求）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Imp {
    pub id: String,                  // 展示请求 ID
    pub metric: Option<Vec<Metric>>, // 相关的度量指标（如可见性、点击率）
    pub banner: Option<Banner>,      // Banner 广告信息
    pub video: Option<Video>,        // 视频广告信息
    pub audio: Option<Audio>,        // 音频广告信息
    pub native: Option<Native>,      // 原生广告信息
    pub pmp: Option<Pmp>,            // 私有交易市场信息
    pub tagid: Option<String>,       // 该 Impression 在 SSP 系统中的标识符
    pub bidfloor: Option<f64>,       // 最低竞价（默认货币单位）
    pub bidfloorcur: Option<String>, // 最低竞价的货币类型（如 USD, EUR）
}

/// **Metric（广告度量）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metric {
    pub r#type: String,         // 度量类型（如 "click-through"）
    pub value: f64,             // 度量的数值
    pub vendor: Option<String>, // 供应商（如 Google, Nielsen）
}

/// **Banner（横幅广告）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Banner {
    pub w: Option<i32>,              // Banner 宽度（像素）
    pub h: Option<i32>,              // Banner 高度（像素）
    pub format: Option<Vec<Format>>, // 允许的广告格式（多个尺寸）
}

/// **Video（视频广告）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Video {
    pub mimes: Vec<String>,          // 支持的视频格式（如 video/mp4）
    pub minduration: Option<i32>,    // 最短持续时间（秒）
    pub maxduration: Option<i32>,    // 最长持续时间（秒）
    pub protocols: Option<Vec<i32>>, // 支持的视频协议（如 VAST）
    pub w: Option<i32>,              // 视频宽度（像素）
    pub h: Option<i32>,              // 视频高度（像素）
}

/// **Audio（音频广告）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Audio {
    pub mimes: Vec<String>,       // 支持的音频格式（如 audio/mp3）
    pub minduration: Option<i32>, // 最短播放时长（秒）
    pub maxduration: Option<i32>, // 最长播放时长（秒）
}

/// **Native（原生广告）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Native {
    pub request: String, // 原生广告请求 JSON
}

/// **Format（Banner 格式）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Format {
    pub w: i32, // 宽度（像素）
    pub h: i32, // 高度（像素）
}

/// **PMP（私有交易市场）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pmp {
    pub private_auction: Option<i32>, // 是否仅限私有竞价（1 = 是, 0 = 否）
    pub deals: Option<Vec<Deal>>,     // 允许的交易
}

/// **交易信息**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Deal {
    pub id: String,            // 交易 ID
    pub bidfloor: Option<f64>, // 最低竞价（货币单位）
}

/// **网站信息**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Site {
    pub id: String,             // 网站 ID
    pub name: Option<String>,   // 网站名称
    pub domain: Option<String>, // 网站域名
}

/// **App 信息**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct App {
    pub id: String,           // 应用 ID
    pub name: Option<String>, // 应用名称
}

/// **设备信息**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Device {
    pub ua: Option<String>, // 用户代理（User-Agent）
    pub ip: Option<String>, // 设备 IP 地址
}

/// **用户信息**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Option<String>, // 用户 ID
}

/// **Source（请求来源）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Source {
    pub fd: Option<i32>,     // 是否来自上游 DSP（1 = 是, 0 = 否）
    pub tid: Option<String>, // 交易 ID
}

/// **Regs（隐私法规）**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Regs {
    pub coppa: Option<i32>, // COPPA（儿童隐私保护）(1 = 是, 0 = 否)
    pub gdpr: Option<i32>,  // GDPR 适用性（1 = 是, 0 = 否）
}
