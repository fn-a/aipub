//! 群组管理

use async_trait::async_trait;
use crate::AuthorizeError;

/// 群组管理器 trait
#[async_trait]
pub trait GroupManager: Send + Sync {
    /// 创建群组
    async fn create(&self, leader_aic: &str) -> Result<Group, AuthorizeError>;
    
    /// 邀请成员
    async fn invite(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError>;
    
    /// 加入群组
    async fn join(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError>;
    
    /// 离开群组
    async fn leave(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError>;
}

/// 群组
#[derive(Debug, Clone)]
pub struct Group {
    pub id: String,
    pub leader_aic: String,
    pub partners: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
