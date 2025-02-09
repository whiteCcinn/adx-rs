// src/model/placements.rs

use serde::{Serialize, Deserialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(try_from = "u8", into = "u8")]
pub enum AdType {
    Native = 1,
    Banner = 2,
    Video = 3,
}

impl TryFrom<u8> for AdType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AdType::Native),
            2 => Ok(AdType::Banner),
            3 => Ok(AdType::Video),
            _ => Err(format!("Invalid value for AdType: {}", value)),
        }
    }
}

impl From<AdType> for u8 {
    fn from(ad: AdType) -> Self {
        ad as u8
    }
}


/// SSP 广告位基础信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SspPlacement {
    pub ssp_id: u64,          // SSP 的 ID
    pub ssp_uuid: String,     // SSP 的 UUID
    pub placement_id: String, // 广告位的 ID
    pub ad_type: AdType,      // 广告位类型
    pub update_time: u64,     // 更新时间（Unix 时间戳）
    pub status: u8,           // 状态：1 = 开启, 2 = 禁用
}

/// DSP 广告位信息集合
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DspPlacement {
    pub dsp_id: u64,            // DSP 的 ID
    pub dsp_uuid: String,       // DSP 的 UUID
    pub tag_id: String,         // DSP 广告位 ID
    pub custom_ad_type: String, // 自定义广告位类型（如 "banner", "video", "native", "banner+video"）
    pub profit_rate: f64,       // 利润率（例如 0.2 表示 20%）
    pub auth: String,           // JSON 字符串，存储宽高等信息（banner/video时）
    pub update_time: u64,       // 更新时间
    pub status: u8,             // 状态：1 = 开启, 2 = 禁用
}
