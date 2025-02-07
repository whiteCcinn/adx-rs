use crate::bidding::dsp::{Demand, DemandManager};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigManager {
    pub demand_manager: DemandManager,
}

impl ConfigManager {
    pub fn new(demand_manager: DemandManager) -> Self {
        Self { demand_manager }
    }

    /// **获取所有可用的 `Demand`**
    pub fn active_demands(&self) -> Vec<Demand> {
        self.demand_manager
            .active_demands()
            .iter()
            .filter(|d| d.status)
            .cloned()
            .collect()
    }

    pub fn active_dsp_urls(&self) -> Vec<String> {
        self.demand_manager
            .active_demands()
            .iter()
            .map(|demand| demand.url.clone())
            .collect()
    }
}
