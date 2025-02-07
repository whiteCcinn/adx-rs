use proptest::prelude::*;
use proptest::strategy::{Just, ValueTree};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// **存储 DSP 需求方（Demand）的信息**
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demand {
    pub id: u64,             // DSP ID（由 DemandManager 分配，从 1 开始增长）
    pub name: String,        // DSP 名称（不包含空格，并以 _dsp 结尾）
    pub url: String,         // DSP 竞价 API 地址
    pub status: bool,        // 是否启用
    pub timeout: Option<u64>,// 每个 DSP 的超时（毫秒），至少 100
}

impl Demand {
    pub fn new(id: u64, name: &str, url: &str, status: bool, timeout: Option<u64>) -> Self {
        Self {
            id,
            name: name.to_string(),
            url: url.to_string(),
            status,
            timeout,
        }
    }
}

/// **DSP 管理器**
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandManager {
    pub demands: HashMap<u64, Demand>,
}

impl DemandManager {
    /// 创建一个新的 DemandManager
    pub fn new() -> Self {
        Self {
            demands: HashMap::new(),
        }
    }

    /// 添加 Demand
    pub fn add_demand(&mut self, demand: Demand) {
        self.demands.insert(demand.id, demand);
    }

    /// 删除 Demand
    pub fn remove_demand(&mut self, demand_id: u64) {
        self.demands.remove(&demand_id);
    }

    /// 获取 Demand 对象
    pub fn get_demand(&self, demand_id: u64) -> Option<&Demand> {
        self.demands.get(&demand_id)
    }

    /// 获取所有 active 的 DSP（status 为 true 的 Demand）
    pub fn active_demands(&self) -> Vec<Demand> {
        self.demands.values().filter(|d| d.status).cloned().collect()
    }
}

/// 使用 proptest 生成随机的 Demand
/// 其中 URL 固定为指向本地 9001 的 DSP 竞价 API，即 "http://localhost:9001/bid"
/// id 使用占位值 0，后续在 DemandManager 中统一赋值；
/// timeout 值在 [100, 1000) 范围内生成；
/// 名称由正则表达式 "[a-zA-Z]{5,15}" 生成（不含空格），然后追加后缀 "_dsp"
fn generate_demand() -> impl Strategy<Value = Demand> {
    (
        Just(0u64), // 占位 id
        // 生成 5 到 15 个字母，不包含空格，然后追加 "_dsp"
        "[a-zA-Z]{5,15}".prop_map(|s| format!("{}{}", s, "_dsp")),
        Just("http://localhost:9001/bid".to_string()),
        any::<bool>(),
        prop::option::of(100..1000u64),
    )
        .prop_map(|(_dummy_id, name, url, status, timeout)| {
            Demand {
                id: 0,
                name,
                url,
                status,
                timeout,
            }
        })
}

/// 使用 proptest 生成随机的 DemandManager
/// 生成 5~10 个 Demand 后，检查是否至少有一个 active（status 为 true），
/// 如果没有，则将第一个 Demand 的 status 置为 true；同时为所有 Demand 分配顺序 id（从 1 开始）
fn generate_demand_manager() -> impl Strategy<Value = DemandManager> {
    prop::collection::vec(generate_demand(), 5..10).prop_map(|mut demands| {
        // 如果没有 active DSP，则将第一个的 status 设为 true
        if !demands.iter().any(|d| d.status) {
            if let Some(first) = demands.first_mut() {
                first.status = true;
            }
        }
        // 为每个 Demand 分配顺序 id，从 1 开始
        for (i, demand) in demands.iter_mut().enumerate() {
            demand.id = (i as u64) + 1;
        }
        let mut manager = DemandManager::new();
        for demand in demands {
            manager.add_demand(demand);
        }
        manager
    })
}

/// 初始化并生成一个随机的 DemandManager，并打印生成的信息
pub fn init() -> DemandManager {
    let mut runner = proptest::test_runner::TestRunner::default();
    let demand_manager = generate_demand_manager()
        .new_tree(&mut runner)
        .unwrap()
        .current();

    println!("Generated DemandManager with {} Demands", demand_manager.demands.len());
    for demand in demand_manager.demands.values() {
        println!(
            "ID: {}, Name: {}, URL: {}, Status: {}, Timeout: {:?}",
            demand.id, demand.name, demand.url, demand.status, demand.timeout
        );
    }

    // 打印 active DSP（status 为 true）的信息
    let active_demands = demand_manager.active_demands();
    println!("\nActive DSPs:");
    for demand in active_demands {
        println!(
            "ID: {}, Name: {}, URL: {}, Timeout: {:?}",
            demand.id, demand.name, demand.url, demand.timeout
        );
    }

    demand_manager
}
