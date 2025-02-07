use axum::{Router, routing::post, Json};
use serde::{Serialize, Deserialize};
use tokio::net::TcpListener;
use axum::serve;
use tokio::time::{sleep, Duration};
use tracing::info;
use rand::Rng;

// 引入 OpenRTB 数据结构，假设这些结构体已在 openrtb 模块中定义
use crate::openrtb::request::BidRequest;
use crate::openrtb::response::{Bid, BidResponse, SeatBid};

/// 模拟 DSP 竞价响应
/// 根据 impression 类型随机生成出价，并根据广告类型为 adm 字段注入 DSP 自己的 tracking URL。
async fn handle_dsp_bid(Json(request): Json<BidRequest>) -> Json<BidResponse> {
    info!(
        "Mock DSP received BidRequest: id={}, imp_count={}",
        request.id,
        request.imp.len()
    );

    // 模拟 DSP 处理延迟（100 ~ 300 毫秒）
    let delay_ms = rand::thread_rng().gen_range(100..300);
    sleep(Duration::from_millis(delay_ms)).await;

    let mut bids = Vec::new();

    for imp in &request.imp {
        // 生成 bid id（例如 "bid-imp1"）
        let bid_id = format!("bid-{}", imp.id);
        // 读取 bidfloor，若为 None 则默认 0.0
        let bidfloor = imp.bidfloor.unwrap_or(0.0);
        // 根据 impression 类型及 banner 尺寸决定 multiplier 范围
        let multiplier = if let Some(banner) = &imp.banner {
            if banner.w == Some(300) && banner.h == Some(250) {
                rand::thread_rng().gen_range(1.0..3.0)
            } else if banner.w == Some(728) && banner.h == Some(90) {
                rand::thread_rng().gen_range(0.8..1.2)
            } else {
                rand::thread_rng().gen_range(1.0..2.0)
            }
        } else if imp.video.is_some() {
            // 视频广告通常投入较高成本，使用较高的 multiplier 范围
            rand::thread_rng().gen_range(1.0..2.5)
        } else if imp.native.is_some() {
            // 原生广告采用稍微灵活的范围
            rand::thread_rng().gen_range(0.8..2.0)
        } else {
            // 默认范围
            rand::thread_rng().gen_range(1.0..2.0)
        };

        let price = bidfloor * multiplier;

        // 根据 impression 类型决定 adm 的内容，并注入 DSP 自己的 tracking URL
        let adm_value = if let Some(banner) = &imp.banner {
            // Banner 广告返回 HTML 格式，包含 dsp tracking 像素和点击链接
            Some(format!(
                "<html><body>Mock DSP Banner Ad<br/><a href=\"http://dsp-tracker.local/click?bid={bid_id}\" target=\"_blank\">Click Here</a><img src=\"http://dsp-tracker.local/impression?bid={bid_id}\" style=\"display:none;\" /></body></html>",
                bid_id = bid_id
            ))
        } else if imp.video.is_some() {
            // 视频广告返回 VAST XML 格式，插入 Impression 和 ClickTracking 标签
            Some(format!(
                r#"<VAST version="3.0">
  <Ad id="{bid_id}">
    <InLine>
      <AdSystem>Mock DSP</AdSystem>
      <AdTitle>Mock Video Ad</AdTitle>
      <Impression><![CDATA[http://dsp-tracker.local/impression?bid={bid_id}]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="640" height="360" bitrate="500">
                http://example.com/video.mp4
              </MediaFile>
            </MediaFiles>
            <VideoClicks>
              <ClickTracking><![CDATA[http://dsp-tracker.local/click?bid={bid_id}]]></ClickTracking>
            </VideoClicks>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#,
                bid_id = bid_id
            ))
        } else if imp.native.is_some() {
            // 原生广告返回 JSON 格式的创意，注入 tracking URL 字段
            Some(format!(
                r#"{{"native":{{"assets":[{{"title":{{"text":"Mock Native Ad"}}}},{{"img":{{"url":"http://example.com/native.jpg"}}}}],"impression_tracking":"http://dsp-tracker.local/impression?bid={bid_id}","click_tracking":"http://dsp-tracker.local/click?bid={bid_id}"}}}}"#,
                bid_id = bid_id
            ))
        } else {
            // 默认返回 HTML 格式
            Some(format!(
                "<html><body>Mock DSP Ad<br/><img src=\"http://dsp-tracker.local/impression?bid={bid_id}\" style=\"display:none;\" /></body></html>",
                bid_id = bid_id
            ))
        };

        bids.push(Bid {
            id: bid_id,
            impid: imp.id.clone(),
            price,
            adm: adm_value,
            nurl: None,
            adid: None,
            adomain: None,
            cid: None,
            crid: None,
            cat: None,
            attr: None,
            dealid: None,
            h: None,
            w: None,
            ext: None,
        });
    }

    let seatbid = SeatBid {
        bid: bids,
        seat: Some("mock_seat".to_string()),
        group: Some(0),
    };

    Json(BidResponse {
        id: request.id.clone(),
        seatbid: vec![seatbid],
        bidid: None,
        cur: Some("USD".to_string()),
        customdata: None,
        nbr: None,
    })
}

/// 启动 Mock DSP 服务
/// 服务监听指定端口（例如 9001），路由为 `/bid`
/// 请确保 ADX 服务调用 DSP 的 URL 与此一致
pub async fn start_mock_dsp_server(port: u16) {
    let app = Router::new().route("/bid", post(handle_dsp_bid));

    let addr = format!("0.0.0.0:{}", port);
    info!("Mock DSP running at http://{}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();
    serve(listener, app).await.unwrap();
}
