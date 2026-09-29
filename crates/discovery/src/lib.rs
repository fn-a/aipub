//! # Discovery - 智能体发现服务
//!
//! 基于 ADP (Agent Discovery Protocol) 协议实现的智能体发现服务。
//!
//! ## 功能特性
//!
//! - 智能体能力查询和匹配（关键词打分，确定性强）
//! - 结构化过滤条件支持（35 种运算符 + AND/OR/NOT 组合）
//! - 跨域发现和转发（环路/深度/扇出校验 + 结果聚合）
//! - 变更流同步（DSP），版本幂等应用
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

use thiserror::Error;

pub mod service;
pub mod storage;
pub mod matcher;
pub mod forwarder;

pub use service::*;
pub use storage::*;
pub use matcher::*;
pub use forwarder::*;

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

    /// 本机发现服务 AIC（写入转发链首跳）
    pub local_server_aic: String,

    /// 明确查询的分数阈值，低于该值时降级为探索性查询
    ///
    /// 与参考实现 `_should_fallback_to_exploratory` 的阈值一致。
    pub match_threshold: f64,

    /// 是否允许明确查询降级为探索性查询
    pub enable_exploratory_fallback: bool,

    /// 信任的下游发现服务器地址列表（为空表示不转发）
    pub forward_trusted_servers: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            enable_forward: true,
            max_forward_depth: 3,
            max_forward_fanout: 5,
            query_timeout_ms: 10000,
            local_server_aic: "AIC-DS-A".to_string(),
            match_threshold: 0.06,
            enable_exploratory_fallback: true,
            forward_trusted_servers: vec![],
        }
    }
}

impl DiscoveryConfig {
    /// 指定本机 AIC
    pub fn with_local_server_aic(mut self, aic: impl Into<String>) -> Self {
        self.local_server_aic = aic.into();
        self
    }

    /// 指定信任的下游发现服务器
    pub fn with_forward_servers(mut self, servers: Vec<String>) -> Self {
        self.forward_trusted_servers = servers;
        self
    }

    /// 关闭转发
    pub fn without_forward(mut self) -> Self {
        self.enable_forward = false;
        self.forward_trusted_servers = vec![];
        self
    }
}
