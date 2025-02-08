
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