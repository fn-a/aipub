//! EAB (External Account Binding) 凭据管理

use async_trait::async_trait;
use crate::RegistryError;

/// EAB 管理器 trait
#[async_trait]
pub trait EabManager: Send + Sync {
    /// 创建 EAB 凭据
    async fn create(&self, account_id: &str, aic: &str) -> Result<EabCredential, RegistryError>;
    
    /// 验证 EAB 凭据
    async fn verify(&self, key_id: &str, hmac_key: &str) -> Result<bool, RegistryError>;
    
    /// 撤销 EAB 凭据
    async fn revoke(&self, key_id: &str) -> Result<(), RegistryError>;
}

/// EAB 凭据
#[derive(Debug, Clone)]
pub struct EabCredential {
    pub key_id: String,
    pub hmac_key: String,
    pub account_id: String,
    pub aic: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
