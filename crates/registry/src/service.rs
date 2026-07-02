//! Registry 服务核心实现

use crate::{RegistryConfig, RegistryError};
use protocols::acs::AgentCapabilitySpec;
use async_trait::async_trait;

/// Registry 服务 trait
#[async_trait]
pub trait RegistryService: Send + Sync {
    /// 注册智能体
    async fn register(&self, acs: AgentCapabilitySpec) -> Result<String, RegistryError>;
    
    /// 注销智能体
    async fn unregister(&self, aic: &str) -> Result<(), RegistryError>;
    
    /// 更新智能体 ACS
    async fn update(&self, aic: &str, acs: AgentCapabilitySpec) -> Result<(), RegistryError>;
    
    /// 获取智能体 ACS
    async fn get(&self, aic: &str) -> Result<Option<AgentCapabilitySpec>, RegistryError>;
    
    /// 激活/停用智能体
    async fn set_active(&self, aic: &str, active: bool) -> Result<(), RegistryError>;
}

/// Registry 服务实现
pub struct RegistryServiceImpl {
    config: RegistryConfig,
    storage: Box<dyn crate::RegistryStorage>,
    eab_manager: Option<Box<dyn crate::EabManager>>,
}

impl RegistryServiceImpl {
    pub fn new(
        config: RegistryConfig,
        storage: Box<dyn crate::RegistryStorage>,
        eab_manager: Option<Box<dyn crate::EabManager>>,
    ) -> Self {
        Self {
            config,
            storage,
            eab_manager,
        }
    }
}

#[async_trait]
impl RegistryService for RegistryServiceImpl {
    async fn register(&self, acs: AgentCapabilitySpec) -> Result<String, RegistryError> {
        // TODO: 实现注册逻辑
        // 1. 验证 AIC
        // 2. 验证 ACS
        // 3. 检查 EAB 凭据（如果需要）
        // 4. 存储 ACS
        // 5. 返回 AIC
        
        acs.validate().map_err(|e| RegistryError::InvalidAcs(e))?;
        
        self.storage.store(acs.clone()).await?;
        
        Ok(acs.aic)
    }
    
    async fn unregister(&self, aic: &str) -> Result<(), RegistryError> {
        self.storage.remove(aic).await
    }
    
    async fn update(&self, aic: &str, acs: AgentCapabilitySpec) -> Result<(), RegistryError> {
        self.storage.update(aic, acs).await
    }
    
    async fn get(&self, aic: &str) -> Result<Option<AgentCapabilitySpec>, RegistryError> {
        self.storage.get(aic).await
    }
    
    async fn set_active(&self, aic: &str, active: bool) -> Result<(), RegistryError> {
        self.storage.set_active(aic, active).await
    }
}
