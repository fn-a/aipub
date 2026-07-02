//! Registry 存储层

use async_trait::async_trait;
use crate::RegistryError;
use protocols::acs::AgentCapabilitySpec;

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
