// src/model/adapters.rs

use crate::model::placements::{SspPlacement, DspPlacement};
use serde::{Serialize, Deserialize};
use std::fs;
use serde_json::Result as JsonResult;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigData {
    pub ssp_placements: Vec<SspPlacement>,
    pub dsp_placements: Vec<DspPlacement>,
}

pub trait ConfigAdapter: Send + Sync {
    fn get_ssp_placements(&self) -> Vec<SspPlacement>;
    fn get_dsp_placements(&self) -> Vec<DspPlacement>;
}

pub struct FileConfigAdapter {
    pub ssp_file: String,
    pub dsp_file: String,
}

impl FileConfigAdapter {
    pub fn new(ssp_file: &str, dsp_file: &str) -> Self {
        Self {
            ssp_file: ssp_file.to_string(),
            dsp_file: dsp_file.to_string(),
        }
    }
}

impl ConfigAdapter for FileConfigAdapter {
    fn get_ssp_placements(&self) -> Vec<SspPlacement> {
        let content = fs::read_to_string(&self.ssp_file).unwrap_or_else(|_| "[]".to_string());
        let config: JsonResult<Vec<SspPlacement>> = serde_json::from_str(&content);
        config.unwrap_or_default()
    }

    fn get_dsp_placements(&self) -> Vec<DspPlacement> {
        let content = fs::read_to_string(&self.dsp_file).unwrap_or_else(|_| "[]".to_string());
        let config: JsonResult<Vec<DspPlacement>> = serde_json::from_str(&content);
        config.unwrap_or_default()
    }
}
