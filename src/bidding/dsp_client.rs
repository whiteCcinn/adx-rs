use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Instant;
use reqwest::Client;
use tokio::time::{timeout, Duration};
use futures::future::join_all;
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::BidResponse;
use crate::bidding::dsp::Demand;
// 假设 Demand 定义类似如下：
// #[derive(Debug, Clone)]
// pub struct Demand {
//     pub id: u64,          // DSP ID
//     pub name: String,     // DSP 名称（现在不再使用）
//     pub url: String,      // DSP 竞价 API 地址
//     pub status: bool,     // 是否启用
//     pub timeout: Option<u64>, // 每个 DSP 可以有不同的超时（毫秒）
// }

/// DSP 客户端，用于并发向各个 DSP 发起请求并获取竞价响应
pub struct DspClient {
    client: Client,
    demands: Vec<Demand>,
}

impl DspClient {
    pub fn new(demands: Vec<Demand>) -> Self {
        Self {
            client: Client::new(),
            demands,
        }
    }

    /// 并发获取 DSP 竞价响应
    /// 返回的元组格式为：
    /// `(dsp_id, dsp_url, 最高出价, BidResponse, 状态描述, 请求耗时_ms)`
    pub async fn fetch_bids(&self, request: &Arc<BidRequest>) -> Vec<(u64, String, f64, BidResponse, String, u128)> {
        let tasks: Vec<_> = self.demands.iter()
            .filter(|demand| demand.status)
            .map(|demand| {
                let dsp_id = demand.id; // 使用 dsp_id
                let client = self.client.clone();
                let req = Arc::clone(request);
                let dsp_url = demand.url.clone();
                // 使用 Demand.timeout，如果没有则使用 BidRequest.tmax 或默认250ms
                let timeout_duration = Duration::from_millis(demand.timeout.unwrap_or(request.tmax.unwrap_or(250)));
                tokio::spawn(async move {
                    let start = Instant::now();
                    let response = timeout(timeout_duration, client.post(&dsp_url)
                        .header("Content-Type", "application/json")
                        .json(&*req)
                        .send())
                        .await;
                    let elapsed = start.elapsed().as_millis();
                    match response {
                        Ok(Ok(resp)) => {
                            match resp.json::<BidResponse>().await {
                                Ok(bid_response) => {
                                    // 提取所有 bid 的最高价格
                                    let price = bid_response.seatbid.iter()
                                        .flat_map(|seatbid| seatbid.bid.iter().map(|bid| bid.price))
                                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                                        .unwrap_or(0.0);
                                    Some((dsp_id, dsp_url, price, bid_response, "success".to_string(), elapsed))
                                }
                                Err(_) => Some((
                                    dsp_id,
                                    dsp_url,
                                    0.0,
                                    BidResponse {
                                        id: "".to_string(),
                                        seatbid: vec![],
                                        bidid: None,
                                        cur: None,
                                        customdata: None,
                                        nbr: None,
                                    },
                                    "json_parse_error".to_string(),
                                    elapsed,
                                )),
                            }
                        }
                        Ok(Err(_)) => Some((
                            dsp_id,
                            dsp_url,
                            0.0,
                            BidResponse {
                                id: "".to_string(),
                                seatbid: vec![],
                                bidid: None,
                                cur: None,
                                customdata: None,
                                nbr: None,
                            },
                            "invalid_response".to_string(),
                            elapsed,
                        )),
                        Err(_) => Some((
                            dsp_id,
                            dsp_url,
                            0.0,
                            BidResponse {
                                id: "".to_string(),
                                seatbid: vec![],
                                bidid: None,
                                cur: None,
                                customdata: None,
                                nbr: None,
                            },
                            "timeout".to_string(),
                            elapsed,
                        )),
                    }
                })
            })
            .collect();

        let mut results = join_all(tasks)
            .await
            .into_iter()
            .filter_map(|res| res.ok().flatten())
            .collect::<Vec<_>>();

        // 按最高出价降序排序
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));
        results
    }
}
