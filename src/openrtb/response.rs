use serde::{Deserialize, Serialize};

/// OpenRTB Bid Response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BidResponse {
    pub id: String,
    pub seatbid: Vec<SeatBid>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeatBid {
    pub bid: Vec<Bid>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bid {
    pub id: String,
    pub impid: String,
    pub price: f64,
    pub adm: String, // Ad markup (HTML or URL)
}
