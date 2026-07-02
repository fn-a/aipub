//! # Authorize - 鉴权服务
//!
//! 面向群组交互模式的鉴权与群组 ACL 服务。
//!
//! ## 功能特性
//!
//! - 消息队列认证
//! - 群组访问控制 (ACL)
//! - 智能体权限管理
//! - 群组邀请和加入

pub mod service;
pub mod acl;
pub mod group;

pub use service::*;
pub use acl::*;
pub use group::*;

use thiserror::Error;

/// Authorize 服务错误类型
#[derive(Debug, Error)]
pub enum AuthorizeError {
    #[error("认证失败: {0}")]
    AuthenticationError(String),
    
    #[error("授权失败: {0}")]
    AuthorizationError(String),
    
    #[error("ACL 错误: {0}")]
    AclError(String),
    
    #[error("群组错误: {0}")]
    GroupError(String),
    
    #[error("无效的凭据: {0}")]
    InvalidCredential(String),
}

/// Authorize 服务配置
#[derive(Debug, Clone)]
pub struct AuthorizeConfig {
    /// 服务端口
    pub port: u16,
    
    /// 是否启用群组功能
    pub enable_group: bool,
    
    /// 默认权限
    pub default_permission: String,
}

impl Default for AuthorizeConfig {
    fn default() -> Self {
        Self {
            port: 8083,
            enable_group: true,
            default_permission: "read".to_string(),
        }
    }
}
