 ```shell
 src/
├── api/                   # API 层：处理 SSP 请求和响应
│   ├── handlers.rs        # 处理 OpenRTB 请求
│   └── models.rs          # OpenRTB 数据模型
├── bidding/               # 竞价处理模块
│   ├── engine.rs          # 核心竞价逻辑
│   ├── dsp_client.rs      # DSP 请求管理（并发和延迟解析）
│   └── parser.rs          # 延迟解析 DSP 响应的逻辑
├── config/                # 配置模块
│   └── config.rs          # 加载和管理服务配置
├── logging/               # 日志模块
│   └── logger.rs          # 日志初始化与跟踪
├── openrtb/               # OpenRTB 协议模块
│   ├── request.rs         # OpenRTB Bid Request 数据结构
│   └── response.rs        # OpenRTB Bid Response 数据结构
├── tests/                 # 测试模块
│   ├── api_tests.rs       # API 测试
│   ├── bidding_tests.rs   # 竞价模块测试
│   ├── dsp_mock.rs        # 模拟 DSP 响应
│   └── integration.rs     # 集成测试
├── main.rs                # 服务入口：整合所有模块并启动服务
 ```