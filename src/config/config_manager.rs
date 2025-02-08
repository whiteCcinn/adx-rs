// src/config/config_manager.rs

use crate::model::dsp::{Demand, DemandManager};
use crate::model::placements::{SspPlacement, DspPlacement};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigManager {
    pub demand_manager: DemandManager,
    #[serde(skip)]
    pub ssp_placements: Arc<RwLock<Vec<SspPlacement>>>,
    #[serde(skip)]
    pub dsp_placements: Arc<RwLock<Vec<DspPlacement>>>,
}

impl ConfigManager {
    pub fn new(demand_manager: DemandManager) -> Self {
        Self {
            demand_manager,
            ssp_placements: Arc::new(RwLock::new(Vec::new())),
            dsp_placements: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn active_demands(&self) -> Vec<Demand> {
        self.demand_manager.active_demands()
    }

    pub fn active_dsp_urls(&self) -> Vec<String> {
        self.demand_manager
            .active_demands()
            .iter()
            .map(|d| d.url.clone())
            .collect()
    }

    pub fn get_ssp_placements(&self) -> Vec<SspPlacement> {
        self.ssp_placements.read().unwrap().clone()
    }

    pub fn get_dsp_placements(&self) -> Vec<DspPlacement> {
        self.dsp_placements.read().unwrap().clone()
    }

    pub fn update_placements(&self, ssp: Vec<SspPlacement>, dsp: Vec<DspPlacement>) {
        {
            let mut lock = self.ssp_placements.write().unwrap();
            *lock = ssp;
        }
        {
            let mut lock = self.dsp_placements.write().unwrap();
            *lock = dsp;
        }
        println!("Placements configuration updated");
    }
}
