use serde::{Serialize, Deserialize};
use once_cell::sync::OnceCell;
use simd_json::base::ValueAsArray;
use simd_json::OwnedValue;

/// OpenRTB BidRequest 结构体，
/// 对于每个对象或数组字段采用延迟解析方式存储为 OwnedValue（owned, 'static），
/// 并为每个大字段提供一个 lazy 缓存字段和 getter 方法。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BidRequest {
    pub id: String,

    /// 广告展示请求列表（imp）存储为 OwnedValue
    pub imp: Box<OwnedValue>,
    #[serde(skip)]
    pub imp_details: OnceCell<Vec<ImpDetail>>,

    /// 网站信息
    pub site: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub site_detail: OnceCell<SiteDetail>,

    /// 应用信息
    pub app: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub app_detail: OnceCell<AppDetail>,

    /// 设备信息
    pub device: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub device_detail: OnceCell<DeviceDetail>,

    /// 用户信息
    pub user: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub user_detail: OnceCell<UserDetail>,

    /// 请求来源信息
    pub source: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub source_detail: OnceCell<SourceDetail>,

    /// 隐私法规信息
    pub regs: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub regs_detail: OnceCell<RegsDetail>,

    // 其它简单字段
    pub test: Option<i32>,
    pub at: Option<i32>,
    pub tmax: Option<u64>,
    pub wseat: Option<Vec<String>>,
    pub bseat: Option<Vec<String>>,
    pub allimps: Option<i32>,
    pub cur: Option<Vec<String>>,
    pub wlang: Option<Vec<String>>,
    pub bcat: Option<Vec<String>>,
    pub badv: Option<Vec<String>>,
}

/// ImpDetail 表示对 imp 数组中单个元素的解析结果
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImpDetail {
    pub id: String,
    pub bidfloor: Option<f64>,

    /// banner 信息延迟解析：原始 JSON 存为 OwnedValue
    pub banner: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub banner_detail: OnceCell<BannerDetail>,

    /// video 信息延迟解析
    pub video: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub video_detail: OnceCell<VideoDetail>,

    /// audio 信息延迟解析
    pub audio: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub audio_detail: OnceCell<AudioDetail>,

    /// native 信息延迟解析
    pub native: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub native_detail: OnceCell<NativeDetail>,

    /// pmp 信息延迟解析
    pub pmp: Option<Box<OwnedValue>>,
    #[serde(skip)]
    pub pmp_detail: OnceCell<PmpDetail>,
}

/// BannerDetail 表示 banner 解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BannerDetail {
    pub w: i32,
    pub h: i32,
    // 可扩展其它字段
}

/// VideoDetail 表示 video 解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoDetail {
    pub mimes: Vec<String>,
    pub minduration: Option<i32>,
    pub maxduration: Option<i32>,
    pub protocols: Option<Vec<i32>>,
    pub w: Option<i32>,
    pub h: Option<i32>,
}

/// AudioDetail 表示 audio 解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioDetail {
    pub mimes: Vec<String>,
    pub minduration: Option<i32>,
    pub maxduration: Option<i32>,
}

/// NativeDetail 表示 native 解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NativeDetail {
    pub request: String,
    // 可扩展其它 native 字段
}

/// PmpDetail 表示 pmp 解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PmpDetail {
    pub private_auction: Option<i32>,
    pub deals: Option<Vec<Deal>>,
}

/// Deal 表示 pmp 中的交易信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Deal {
    pub id: String,
    pub bidfloor: Option<f64>,
}

/// SiteDetail 表示网站信息解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SiteDetail {
    pub id: String,
    pub name: Option<String>,
    pub domain: Option<String>,
}

/// AppDetail 表示应用信息解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppDetail {
    pub id: String,
    pub name: Option<String>,
}

/// DeviceDetail 表示设备信息解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceDetail {
    pub ua: Option<String>,
    pub ip: Option<String>,
}

/// UserDetail 表示用户信息解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDetail {
    pub id: Option<String>,
}

/// SourceDetail 表示请求来源解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourceDetail {
    pub fd: Option<i32>,
    pub tid: Option<String>,
}

/// RegsDetail 表示隐私法规解析后的数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegsDetail {
    pub coppa: Option<i32>,
    pub gdpr: Option<i32>,
}

// Getter 方法实现
impl BidRequest {
    pub fn get_imp_details(&self) -> &Vec<ImpDetail> {
        self.imp_details.get_or_init(|| {
            if let Some(arr) = self.imp.as_array() {
                arr.iter().map(|item| {
                    let s = serde_json::to_string(&*item)
                        .expect("Failed to convert imp item to JSON string");
                    serde_json::from_str(&s)
                        .expect("Failed to parse imp item into ImpDetail")
                }).collect()
            } else {
                Vec::new()
            }
        })
    }

    pub fn get_site_detail(&self) -> Option<&SiteDetail> {
        self.site.as_ref().map(|raw| {
            self.site_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert site to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse site into SiteDetail")
            })
        })
    }

    pub fn get_app_detail(&self) -> Option<&AppDetail> {
        self.app.as_ref().map(|raw| {
            self.app_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert app to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse app into AppDetail")
            })
        })
    }

    pub fn get_device_detail(&self) -> Option<&DeviceDetail> {
        self.device.as_ref().map(|raw| {
            self.device_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert device to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse device into DeviceDetail")
            })
        })
    }

    pub fn get_user_detail(&self) -> Option<&UserDetail> {
        self.user.as_ref().map(|raw| {
            self.user_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert user to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse user into UserDetail")
            })
        })
    }

    pub fn get_source_detail(&self) -> Option<&SourceDetail> {
        self.source.as_ref().map(|raw| {
            self.source_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert source to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse source into SourceDetail")
            })
        })
    }

    pub fn get_regs_detail(&self) -> Option<&RegsDetail> {
        self.regs.as_ref().map(|raw| {
            self.regs_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert regs to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse regs into RegsDetail")
            })
        })
    }
}

impl ImpDetail {
    pub fn get_banner_detail(&self) -> Option<&BannerDetail> {
        self.banner.as_ref().map(|raw| {
            self.banner_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert banner to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse banner into BannerDetail")
            })
        })
    }

    pub fn get_video_detail(&self) -> Option<&VideoDetail> {
        self.video.as_ref().map(|raw| {
            self.video_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert video to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse video into VideoDetail")
            })
        })
    }

    pub fn get_audio_detail(&self) -> Option<&AudioDetail> {
        self.audio.as_ref().map(|raw| {
            self.audio_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert audio to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse audio into AudioDetail")
            })
        })
    }

    pub fn get_native_detail(&self) -> Option<&NativeDetail> {
        self.native.as_ref().map(|raw| {
            self.native_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert native to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse native into NativeDetail")
            })
        })
    }

    pub fn get_pmp_detail(&self) -> Option<&PmpDetail> {
        self.pmp.as_ref().map(|raw| {
            self.pmp_detail.get_or_init(|| {
                let s = serde_json::to_string(&*raw)
                    .expect("Failed to convert pmp to JSON string");
                serde_json::from_str(&s)
                    .expect("Failed to parse pmp into PmpDetail")
            })
        })
    }
}
