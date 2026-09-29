//! Discovery 存储层
//!
//! 参考实现使用 PostgreSQL + pgvector；此处提供内存实现，
//! 支持 ADS 变更流（DSP）的版本幂等应用。

use std::collections::HashMap;
use async_trait::async_trait;
use tokio::sync::RwLock;
use serde_json::Value;
use protocols::adp::{matches_filter, DiscoveryFilter};
use protocols::dsp::ChangeEntry;
use crate::DiscoveryError;

/// 存储中的智能体记录
#[derive(Debug, Clone, PartialEq)]
pub struct StoredAgent {
    /// 智能体身份码
    pub aic: String,

    /// ACS 原文
    pub acs: Value,

    /// ACS 版本号
    pub version: u64,

    /// 最近应用的变更序号
    pub seq: u64,
}

/// 存储 trait
#[async_trait]
pub trait Storage: Send + Sync {
    /// 存储智能体 ACS
    async fn store(&self, acs: Value) -> Result<(), DiscoveryError>;
    
    /// 移除智能体
    async fn remove(&self, aic: &str) -> Result<(), DiscoveryError>;
    
    /// 更新智能体 ACS
    async fn update(&self, aic: &str, acs: Value) -> Result<(), DiscoveryError>;
    
    /// 获取智能体 ACS
    async fn get(&self, aic: &str) -> Result<Option<Value>, DiscoveryError>;
    
    /// 查询智能体列表
    async fn query(&self, filter: Option<DiscoveryFilter>) -> Result<Vec<Value>, DiscoveryError>;

    /// 应用一条变更流记录
    ///
    /// 返回 `true` 表示已应用；`false` 表示版本不大于已存版本（幂等丢弃）。
    async fn apply_change(&self, entry: &ChangeEntry) -> Result<bool, DiscoveryError>;

    /// 列出全部记录（含版本信息）
    async fn records(&self) -> Result<Vec<StoredAgent>, DiscoveryError>;
}

/// 内存存储实现（用于测试、演示与单进程集成）
#[derive(Debug)]
pub struct MemoryStorage {
    agents: RwLock<HashMap<String, StoredAgent>>,
}

impl MemoryStorage {
    /// 创建空存储
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// 当前记录数
    pub async fn len(&self) -> usize {
        self.agents.read().await.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.agents.read().await.is_empty()
    }

    /// 清空
    pub async fn clear(&self) {
        self.agents.write().await.clear();
    }

    /// 按本体前缀列出派生实体
    pub async fn list_by_ontology_prefix(&self, ontology_prefix: &str) -> Vec<StoredAgent> {
        let prefix = format!("{}.", ontology_prefix);
        self.agents
            .read()
            .await
            .values()
            .filter(|record| record.aic.starts_with(&prefix))
            .cloned()
            .collect()
    }
}

/// 从 ACS 中取出 AIC
fn extract_aic(acs: &Value) -> Result<String, DiscoveryError> {
    acs.get("aic")
        .and_then(|value| value.as_str())
        .filter(|aic| !aic.trim().is_empty())
        .map(|aic| aic.to_string())
        .ok_or_else(|| DiscoveryError::StorageError("ACS 缺少 aic 字段".to_string()))
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn store(&self, acs: Value) -> Result<(), DiscoveryError> {
        let aic = extract_aic(&acs)?;
        let mut agents = self.agents.write().await;
        let version = agents
            .get(&aic)
            .map(|record| record.version + 1)
            .unwrap_or(1);
        let seq = agents.get(&aic).map(|record| record.seq).unwrap_or(0);
        agents.insert(
            aic.clone(),
            StoredAgent {
                aic,
                acs,
                version,
                seq,
            },
        );
        Ok(())
    }
    
    async fn remove(&self, aic: &str) -> Result<(), DiscoveryError> {
        self.agents.write().await.remove(aic);
        Ok(())
    }
    
    async fn update(&self, aic: &str, acs: Value) -> Result<(), DiscoveryError> {
        let mut agents = self.agents.write().await;
        let record = agents
            .get_mut(aic)
            .ok_or_else(|| DiscoveryError::AgentNotFound(aic.to_string()))?;
        record.acs = acs;
        record.version += 1;
        Ok(())
    }
    
    async fn get(&self, aic: &str) -> Result<Option<Value>, DiscoveryError> {
        Ok(self.agents.read().await.get(aic).map(|r| r.acs.clone()))
    }
    
    async fn query(&self, filter: Option<DiscoveryFilter>) -> Result<Vec<Value>, DiscoveryError> {
        let agents = self.agents.read().await;
        let mut matched: Vec<Value> = agents
            .values()
            .filter(|record| matches_filter(&record.acs, filter.as_ref()))
            .map(|record| record.acs.clone())
            .collect();
        matched.sort_by(|a, b| {
            let left = a.get("aic").and_then(|v| v.as_str()).unwrap_or_default();
            let right = b.get("aic").and_then(|v| v.as_str()).unwrap_or_default();
            left.cmp(right)
        });
        Ok(matched)
    }

    async fn apply_change(&self, entry: &ChangeEntry) -> Result<bool, DiscoveryError> {
        let mut agents = self.agents.write().await;

        if let Some(existing) = agents.get(&entry.id) {
            // 版本未前进则丢弃，保证重复投递的幂等性
            if entry.version <= existing.version {
                return Ok(false);
            }
        }

        if entry.is_delete() {
            agents.remove(&entry.id);
            return Ok(true);
        }

        let payload = entry
            .payload
            .clone()
            .ok_or_else(|| DiscoveryError::StorageError("upsert 变更缺少 payload".to_string()))?;

        agents.insert(
            entry.id.clone(),
            StoredAgent {
                aic: entry.id.clone(),
                acs: payload,
                version: entry.version,
                seq: entry.seq,
            },
        );
        Ok(true)
    }

    async fn records(&self) -> Result<Vec<StoredAgent>, DiscoveryError> {
        let agents = self.agents.read().await;
        let mut records: Vec<StoredAgent> = agents.values().cloned().collect();
        records.sort_by(|a, b| a.aic.cmp(&b.aic));
        Ok(records)
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}
