// src/model/context.rs

use crate::openrtb::request::BidRequest;
use crate::model::ssp::Ssp;
use crate::model::placements::{SspPlacement, DspPlacement};
use crate::model::dsp::Demand;
use std::time::Instant;
use serde::{Serialize, Deserialize};

fn default_instant() -> Instant {
    Instant::now()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Context {
    pub bid_request: BidRequest,
    /// 当前请求所属的 SSP 基础信息
    pub ssp: Ssp,
    /// 当前请求对应的 SSP 广告位
    pub ssp_placement: SspPlacement,
    /// DSP 请求列表及对应的 DSP 广告位信息（待关联）
    pub dsp_requests: Vec<(Demand, DspPlacement)>,
    /// 请求开始时间，用于计算总耗时（不参与序列化）
    #[serde(skip, default = "default_instant")]
    pub start_time: Instant,
}
