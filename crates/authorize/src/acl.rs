//! ACL (访问控制列表) 管理

use async_trait::async_trait;
use crate::AuthorizeError;

/// ACL 管理器 trait
#[async_trait]
pub trait AclManager: Send + Sync {
    /// 添加权限
    async fn add_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AuthorizeError>;
    
    /// 移除权限
    async fn remove_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AuthorizeError>;
    
    /// 检查权限
    async fn check_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthorizeError>;
}

/// 权限
#[derive(Debug, Clone)]
pub struct Permission {
    pub aic: String,
    pub resource: String,
    pub action: String,
}
