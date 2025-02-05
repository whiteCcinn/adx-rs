#[derive(Clone, Debug)]
pub struct ConfigManager {
    pub dsp_endpoints: Vec<String>,
}

impl ConfigManager {
    pub fn new(dsp_endpoints: Vec<String>) -> Self {
        ConfigManager { dsp_endpoints }
    }

    pub fn from_args(dsp_endpoints: &str) -> Self {
        let endpoints = dsp_endpoints.split(',').map(String::from).collect();
        ConfigManager::new(endpoints)
    }
}
