use serde::{Deserialize, Serialize};

/// OpenRTB Bid Request
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BidRequest {
    pub id: String,
    pub imp: Vec<Impression>,
    pub site: Site,
    pub user: User,
    pub tmax: Option<u64>, // 最大延迟（毫秒）
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Impression {
    pub id: String,
    pub bidfloor: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Site {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
}
