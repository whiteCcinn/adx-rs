use axum::{Router, routing::post, Json};
use serde::{Serialize, Deserialize};
use tokio::net::TcpListener;
use axum::serve;

#[derive(Debug, Serialize, Deserialize)]
struct BidRequest {
    id: String,
    imp: Vec<Impression>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Impression {
    id: String,
    bidfloor: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BidResponse {
    id: String,
    seatbid: Vec<SeatBid>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SeatBid {
    bid: Vec<Bid>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Bid {
    id: String,
    impid: String,
    price: f64,
    adm: String,
}

/// 模拟 DSP 竞价响应
async fn handle_dsp_bid(Json(request): Json<BidRequest>) -> Json<BidResponse> {
    let mut bids = Vec::new();

    for imp in &request.imp {
        bids.push(Bid {
            id: format!("bid-{}", imp.id),
            impid: imp.id.clone(),
            price: imp.bidfloor + 0.5,  // DSP 以 `bidfloor + 0.5` 作为出价
            adm: "<html><body>Mock DSP Ad</body></html>".to_string(),
        });
    }

    Json(BidResponse {
        id: request.id.clone(),
        seatbid: vec![SeatBid { bid: bids }],
    })
}

pub async fn start_mock_dsp_server(port: u16) {
    let app = Router::new().route("/bid", post(handle_dsp_bid));

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Mock DSP running at http://{}", addr);

    serve(listener, app).await.unwrap();
}
