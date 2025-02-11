// src/model/adapters.rs

use crate::model::placements::{SspPlacement, DspPlacement};
use crate::model::ssp::Ssp;
use serde::{Serialize, Deserialize};
use std::fs;
use serde_json::Result as JsonResult;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SspInfoData(pub Vec<Ssp>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlacementsData {
    pub ssp_placements: Vec<SspPlacement>,
    pub dsp_placements: Vec<DspPlacement>,
}

/// 配置适配器 trait，支持从不同数据源获取配置数据
pub trait ConfigAdapter: Send + Sync {
    fn get_ssp_placements(&self) -> Vec<SspPlacement>;
    fn get_dsp_placements(&self) -> Vec<DspPlacement>;
    fn get_ssp_info(&self) -> Vec<Ssp>;
}

/// 文件配置适配器，从静态 JSON 文件读取数据
pub struct FileConfigAdapter {
    pub ssp_placements_file: String,
    pub dsp_placements_file: String,
    pub ssp_info_file: String,
}

impl FileConfigAdapter {
    pub fn new(ssp_placements_file: &str, dsp_placements_file: &str, ssp_info_file: &str) -> Self {
        Self {
            ssp_placements_file: ssp_placements_file.to_string(),
            dsp_placements_file: dsp_placements_file.to_string(),
            ssp_info_file: ssp_info_file.to_string(),
        }
    }
}

impl ConfigAdapter for FileConfigAdapter {
    fn get_ssp_placements(&self) -> Vec<SspPlacement> {
        let content = fs::read_to_string(&self.ssp_placements_file)
            .unwrap_or_else(|_| "[]".to_string());
        let config: JsonResult<Vec<SspPlacement>> = serde_json::from_str(&content);
        config.unwrap_or_default()
    }

    fn get_dsp_placements(&self) -> Vec<DspPlacement> {
        let content = fs::read_to_string(&self.dsp_placements_file)
            .unwrap_or_else(|_| "[]".to_string());
        let config: JsonResult<Vec<DspPlacement>> = serde_json::from_str(&content);
        config.unwrap_or_default()
    }

    fn get_ssp_info(&self) -> Vec<Ssp> {
        let content = fs::read_to_string(&self.ssp_info_file)
            .unwrap_or_else(|_| "[]".to_string());
        let config: JsonResult<Vec<Ssp>> = serde_json::from_str(&content);
        config.unwrap_or_default()
    }
}
