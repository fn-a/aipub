//! Discovery 存储层

use async_trait::async_trait;
use crate::DiscoveryError;

/// 存储 trait
#[async_trait]
pub trait Storage: Send + Sync {
    /// 存储智能体 ACS
    async fn store(&self, acs: serde_json::Value) -> Result<(), DiscoveryError>;
    
    /// 移除智能体
    async fn remove(&self, aic: &str) -> Result<(), DiscoveryError>;
    
    /// 更新智能体 ACS
    async fn update(&self, aic: &str, acs: serde_json::Value) -> Result<(), DiscoveryError>;
    
    /// 获取智能体 ACS
    async fn get(&self, aic: &str) -> Result<Option<serde_json::Value>, DiscoveryError>;
    
    /// 查询智能体列表
    async fn query(&self, filter: Option<protocols::adp::DiscoveryFilter>) -> Result<Vec<serde_json::Value>, DiscoveryError>;
}

/// 内存存储实现（用于测试）
pub struct MemoryStorage {
    // TODO: 实现内存存储
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn store(&self, _acs: serde_json::Value) -> Result<(), DiscoveryError> {
        // TODO: 实现
        Ok(())
    }
    
    async fn remove(&self, _aic: &str) -> Result<(), DiscoveryError> {
        // TODO: 实现
        Ok(())
    }
    
    async fn update(&self, _aic: &str, _acs: serde_json::Value) -> Result<(), DiscoveryError> {
        // TODO: 实现
        Ok(())
    }
    
    async fn get(&self, _aic: &str) -> Result<Option<serde_json::Value>, DiscoveryError> {
        // TODO: 实现
        Ok(None)
    }
    
    async fn query(&self, _filter: Option<protocols::adp::DiscoveryFilter>) -> Result<Vec<serde_json::Value>, DiscoveryError> {
        // TODO: 实现
        Ok(vec![])
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}
