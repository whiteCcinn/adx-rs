// src/model/dsp.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use proptest::prelude::*;
use proptest::strategy::ValueTree;

/// DSP 基础信息结构体，表示 DSP 的基本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demand {
    pub id: u64,              // DSP ID（由 DemandManager 分配，从 1 开始增长）
    pub name: String,         // DSP 名称（不含空格，并以 _dsp 结尾）
    pub url: String,          // DSP 竞价 API 地址
    pub status: bool,         // 是否启用
    pub timeout: Option<u64>, // 每个 DSP 的超时（毫秒），至少 100
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

/// DSP 管理器，管理多个 DSP 的 Demand 信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandManager {
    pub demands: HashMap<u64, Demand>,
}

impl DemandManager {
    pub fn new() -> Self {
        Self {
            demands: HashMap::new(),
        }
    }

    pub fn add_demand(&mut self, demand: Demand) {
        self.demands.insert(demand.id, demand);
    }

    pub fn remove_demand(&mut self, demand_id: u64) {
        self.demands.remove(&demand_id);
    }

    pub fn get_demand(&self, demand_id: u64) -> Option<&Demand> {
        self.demands.get(&demand_id)
    }

    pub fn active_demands(&self) -> Vec<Demand> {
        self.demands.values().filter(|d| d.status).cloned().collect()
    }
}

/// 使用 proptest 生成随机的 Demand
fn generate_demand() -> impl Strategy<Value = Demand> {
    (
        Just(0u64), // 占位 id
        "[a-zA-Z]{5,15}".prop_map(|s| format!("{}{}", s, "_dsp")),
        Just("http://localhost:9001/bid".to_string()),
        any::<bool>(),
        (100u64..1000u64),
    )
        .prop_map(|(_dummy, name, url, status, timeout)| {
            Demand {
                id: 0,
                name,
                url,
                status,
                timeout: Some(timeout),
            }
        })
}

/// 使用 proptest 生成随机的 DemandManager
fn generate_demand_manager() -> impl Strategy<Value = DemandManager> {
    proptest::collection::vec(generate_demand(), 5..10).prop_map(|mut demands| {
        if !demands.iter().any(|d| d.status) {
            if let Some(first) = demands.first_mut() {
                first.status = true;
            }
        }
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

    println!("\nActive DSPs:");
    for demand in demand_manager.active_demands() {
        println!(
            "ID: {}, Name: {}, URL: {}, Timeout: {:?}",
            demand.id, demand.name, demand.url, demand.timeout
        );
    }

    demand_manager
}
