//! Registry 存储层

use std::collections::HashMap;
use async_trait::async_trait;
use tokio::sync::RwLock;
use protocols::acs::AgentCapabilitySpec;
use crate::RegistryError;

/// Registry 存储 trait
#[async_trait]
pub trait RegistryStorage: Send + Sync {
    /// 存储智能体 ACS
    async fn store(&self, acs: AgentCapabilitySpec) -> Result<(), RegistryError>;
    
    /// 移除智能体
    async fn remove(&self, aic: &str) -> Result<(), RegistryError>;
    
    /// 更新智能体 ACS
    async fn update(&self, aic: &str, acs: AgentCapabilitySpec) -> Result<(), RegistryError>;
    
    /// 获取智能体 ACS
    async fn get(&self, aic: &str) -> Result<Option<AgentCapabilitySpec>, RegistryError>;
    
    /// 设置激活状态
    async fn set_active(&self, aic: &str, active: bool) -> Result<(), RegistryError>;
    
    /// 列出所有智能体
    async fn list(&self) -> Result<Vec<AgentCapabilitySpec>, RegistryError>;
}

/// 内存存储实现
///
/// 适用于测试、演示与单进程集成场景；生产环境应替换为 PostgreSQL/Redis 实现。
#[derive(Debug, Default)]
pub struct MemoryRegistryStorage {
    agents: RwLock<HashMap<String, AgentCapabilitySpec>>,
}

impl MemoryRegistryStorage {
    /// 创建空存储
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// 当前智能体数量
    pub async fn len(&self) -> usize {
        self.agents.read().await.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.agents.read().await.is_empty()
    }

    /// 按本体前缀列出派生实体（1-8 级相同）
    pub async fn list_by_ontology_prefix(
        &self,
        ontology_prefix: &str,
    ) -> Vec<AgentCapabilitySpec> {
        let prefix = format!("{}.", ontology_prefix);
        self.agents
            .read()
            .await
            .values()
            .filter(|acs| acs.aic.starts_with(&prefix))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl RegistryStorage for MemoryRegistryStorage {
    async fn store(&self, acs: AgentCapabilitySpec) -> Result<(), RegistryError> {
        self.agents.write().await.insert(acs.aic.clone(), acs);
        Ok(())
    }

    async fn remove(&self, aic: &str) -> Result<(), RegistryError> {
        let removed = self.agents.write().await.remove(aic);
        if removed.is_none() {
            return Err(RegistryError::AgentNotFound(aic.to_string()));
        }
        Ok(())
    }

    async fn update(&self, aic: &str, acs: AgentCapabilitySpec) -> Result<(), RegistryError> {
        let mut agents = self.agents.write().await;
        if !agents.contains_key(aic) {
            return Err(RegistryError::AgentNotFound(aic.to_string()));
        }
        agents.insert(aic.to_string(), acs);
        Ok(())
    }

    async fn get(&self, aic: &str) -> Result<Option<AgentCapabilitySpec>, RegistryError> {
        Ok(self.agents.read().await.get(aic).cloned())
    }

    async fn set_active(&self, aic: &str, active: bool) -> Result<(), RegistryError> {
        let mut agents = self.agents.write().await;
        let acs = agents
            .get_mut(aic)
            .ok_or_else(|| RegistryError::AgentNotFound(aic.to_string()))?;
        acs.active = active;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<AgentCapabilitySpec>, RegistryError> {
        let mut agents: Vec<AgentCapabilitySpec> =
            self.agents.read().await.values().cloned().collect();
        agents.sort_by(|a, b| a.aic.cmp(&b.aic));
        Ok(agents)
    }
}
