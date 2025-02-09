// src/model/ssp.rs

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ssp {
    pub id: u64,
    pub uuid: String,
    pub name: String,
    pub qps: u32,
}