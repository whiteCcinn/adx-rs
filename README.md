
基于`openrtb2.x`的adx服务，用rust实现

```shell
src/
 ├── api
 │   ├── handlers.rs         // HTTP 请求处理（调用 bidding/engine.rs 的逻辑）
 │   └── mod.rs              // 导出 handlers
 ├── bidding
 │   ├── dsp_client.rs       // DSP 客户端，负责并发调用各 DSP
 │   ├── engine.rs           // ADX 竞价处理核心（处理 DSP 响应、利润扣除、tracking 替换、调用链日志生成等）
 │   └── mod.rs              // 导出 dsp_client、engine 等模块
 ├── config
 │   ├── config_manager.rs   // 配置管理器，原有 DemandManager 和新增广告位配置，保持向后兼容
 │   └── mod.rs              // 导出 config_manager
 ├── logging
 │   ├── adx_log.rs          // ADX 询价调用链日志数据结构（业务日志，格式固定）
 │   ├── logger.rs           // （如果需要）业务日志记录模块
 │   ├── runtime_logger.rs   // 运行日志记录模块（记录服务运行状态、调试、错误等）
 │   └── mod.rs              // 导出 adx_log、logger、runtime_logger
 ├── model
 │   ├── adapters.rs         // 配置适配器，从 /static 下 JSON 文件读取广告位配置
 │   ├── dsp.rs              // DSP 基础信息数据模型（Demand、DemandManager）
 │   └── placements.rs       // 广告位相关数据模型：AdType 枚举、SspPlacement、DspPlacement
 ├── openrtb
 │   ├── request.rs          // OpenRTB BidRequest 定义
 │   └── response.rs         // OpenRTB BidResponse 及子结构定义
 ├── mock_dsp.rs             // 模拟 DSP 服务代码（用于测试 DSP 竞价流程）
 ├── main.rs                 // 主程序入口，初始化各模块、加载配置、启动 ADX 与 mock_dsp 服务器
 └── static
      ├── ssp_placements.json  // SSP 广告位配置
      └── dsp_placements.json   // DSP 广告位配置
 ```

调试代码
```shell
curl -X POST "http://127.0.0.1:8080/openrtb?ssp_uuid=ssp-uuid-001" \                                                                                                
  -H "Content-Type: application/json" \
  -d '{
    "id": "1234",
    "imp": [
      {
        "id": "imp1",
        "banner": { "w": 300, "h": 250 },
        "bidfloor": 0.5
      },
      {
        "id": "imp2",
        "banner": { "w": 728, "h": 90 },
        "bidfloor": 1.0
      },
      {
        "id": "imp3",
        "video": {
          "mimes": ["video/mp4"],
          "minduration": 5,
          "maxduration": 30,
          "w": 640,
          "h": 360,
          "protocols": [2, 3]
        },
        "bidfloor": 0.8
      },
      {
        "id": "imp4",
        "native": {
          "request": "{\"native\":{\"assets\":[{\"title\":{\"text\":\"Native Ad Title\"}},{\"img\":{\"url\":\"http://example.com/native.jpg\"}}]}}"
        },
        "bidfloor": 0.7
      }
    ],
    "site": {
      "id": "5678",
      "name": "example.com"
    },
    "user": {
      "id": "user1"
    },
    "tmax": 100
}' | jq .
```

响应例子：
```json
{
  "id": "1234",
  "seatbid": [
    {
      "bid": [
        {
          "id": "bid-imp3",
          "impid": "imp3",
          "price": 1.7844350876654955,
          "nurl": "http://example.com/nurl",
          "adm": "<VAST version=\"3.0\">\n  <Ad id=\"bid-imp3\">\n    <InLine>\n      <AdSystem>Mock DSP</AdSystem>\n      <AdTitle>Mock Video Ad</AdTitle>\n      <Impression><![CDATA[http://dsp-tracker.local/impression?bid=bid-imp3&price=1.4275480701323966]]></Impression>\n      <Creatives>\n        <Creative>\n          <Linear>\n            <Duration>00:00:30</Duration>\n            <MediaFiles>\n              <MediaFile delivery=\"progressive\" type=\"video/mp4\" width=\"640\" height=\"360\" bitrate=\"500\">\n                http://example.com/video.mp4\n              </MediaFile>\n            </MediaFiles>\n            <VideoClicks>\n              <ClickTracking><![CDATA[http://dsp-tracker.local/click?bid=bid-imp3&price=1.4275480701323966]]></ClickTracking>\n            </VideoClicks>\n          </Linear>\n        </Creative>\n      </Creatives>\n    </InLine>\n  </Ad>\n</VAST><Impression><![CDATA[http://tk.rust-adx.com/impression?price={AUCTION_PRICE}]]></Impression>",
          "adid": "ad-12345",
          "adomain": [
            "example.com"
          ],
          "cid": "cid-12345",
          "crid": "crid-12345",
          "cat": [
            "IAB1",
            "IAB2"
          ],
          "attr": [
            1,
            2
          ],
          "dealid": "deal-123",
          "h": 519,
          "w": 264,
          "ext": {
            "extra_info": "some_value"
          }
        }
      ],
      "seat": "",
      "group": 0
    }
  ],
  "bidid": null,
  "cur": "USD",
  "customdata": null,
  "nbr": null
}
```