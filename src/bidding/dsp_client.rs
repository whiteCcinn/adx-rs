use std::sync::Arc;
use reqwest::Client;
use serde_json::Value;
use tokio::time::{timeout, Duration};
use futures::future::join_all;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::{Bid, BidResponse};

pub struct DspClient {
    client: Client,
    dsp_endpoints: Vec<String>,
}

impl DspClient {
    pub fn new(dsp_endpoints: Vec<String>) -> Self {
        Self {
            client: Client::new(),
            dsp_endpoints,
        }
    }

    /// ✅ 并发请求所有 DSP，并返回所有 `BidResponse`
    pub async fn fetch_bids(&self, request: Arc<BidRequest>) -> Vec<(BidResponse, f64)> {
        let timeout_duration = Duration::from_millis(request.tmax.unwrap_or(250));

        let tasks: Vec<_> = self.dsp_endpoints.iter().map(|dsp_url| {
            let client = self.client.clone();
            let req = Arc::clone(&request);
            let dsp_url = dsp_url.clone();

            tokio::spawn(async move {
                let response = timeout(timeout_duration, client.post(&dsp_url)
                    .header("Content-Type", "application/json")
                    .json(&*req)
                    .send())
                    .await;

                match response {
                    Ok(Ok(resp)) => resp.json::<Value>().await.ok(),
                    _ => None,
                }
            })
        }).collect();

        let results = join_all(tasks).await;

        let mut bid_responses = Vec::new();

        // ✅ 遍历所有 DSP 响应，提取 `BidResponse` 并保存
        for result in results.into_iter().flatten() {
            if let Some(json) = result {
                if let Some(seatbids) = json["seatbid"].as_array() {
                    for seat in seatbids {
                        if let Some(bid_array) = seat["bid"].as_array() {
                            for bid in bid_array {
                                if let Some(price) = bid["price"].as_f64() {
                                    if let Ok(bid_response) = serde_json::from_value::<BidResponse>(json.clone()) {
                                        bid_responses.push((bid_response, price));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ✅ 按 `price` 从高到低排序
        bid_responses.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        bid_responses
    }
}
