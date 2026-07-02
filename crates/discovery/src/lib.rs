//! # Discovery - 智能体发现服务
//!
//! 基于 ADP (Agent Discovery Protocol) 协议实现的智能体发现服务。
//!
//! ## 功能特性
//!
//! - 智能体能力查询和匹配
//! - 结构化过滤条件支持
//! - 跨域发现和转发
//! - 向量相似度匹配（可选）
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────┐
//! │  Discovery API  │
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │  Query Engine   │
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │  Storage Layer  │
//! └─────────────────┘
//! ```

pub mod service;
pub mod storage;
pub mod matcher;
pub mod forwarder;

pub use service::*;
pub use storage::*;
pub use matcher::*;
pub use forwarder::*;

use thiserror::Error;

/// Discovery 服务错误类型
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("查询错误: {0}")]
    QueryError(String),
    
    #[error("存储错误: {0}")]
    StorageError(String),
    
    #[error("转发错误: {0}")]
    ForwardError(String),
    
    #[error("无效的过滤条件: {0}")]
    InvalidFilter(String),
    
    #[error("智能体未找到: {0}")]
    AgentNotFound(String),
}

/// Discovery 服务配置
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// 服务端口
    pub port: u16,
    
    /// 是否启用转发
    pub enable_forward: bool,
    
    /// 最大转发深度
    pub max_forward_depth: usize,
    
    /// 最大转发扇出
    pub max_forward_fanout: usize,
    
    /// 查询超时（毫秒）
    pub query_timeout_ms: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            enable_forward: true,
            max_forward_depth: 3,
            max_forward_fanout: 10,
            query_timeout_ms: 5000,
        }
    }
}
