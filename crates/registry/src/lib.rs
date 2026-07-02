//! # Registry - 智能体注册服务
//!
//! 基于 ATR (Agent Trust Registry) 协议实现的智能体注册服务。
//!
//! ## 功能特性
//!
//! - 智能体注册和注销
//! - ACS 能力描述管理
//! - EAB 凭据管理
//! - mTLS 证书验证
//! - 本体和实体关系管理

pub mod service;
pub mod storage;
pub mod eab;

pub use service::*;
pub use storage::*;
pub use eab::*;

use thiserror::Error;

/// Registry 服务错误类型
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("注册错误: {0}")]
    RegistrationError(String),
    
    #[error("验证错误: {0}")]
    VerificationError(String),
    
    #[error("存储错误: {0}")]
    StorageError(String),
    
    #[error("无效的 AIC: {0}")]
    InvalidAic(String),
    
    #[error("无效的 ACS: {0}")]
    InvalidAcs(String),
    
    #[error("EAB 凭据错误: {0}")]
    EabError(String),
}

/// Registry 服务配置
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// 服务端口
    pub port: u16,
    
    /// 是否启用自动审批
    pub auto_approval: bool,
    
    /// 是否需要 EAB 凭据
    pub require_eab: bool,
    
    /// 是否启用 mTLS
    pub enable_mtls: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            port: 8081,
            auto_approval: false,
            require_eab: true,
            enable_mtls: true,
        }
    }
}
