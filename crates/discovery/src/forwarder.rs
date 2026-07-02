//! Discovery 转发器

use async_trait::async_trait;
use crate::DiscoveryError;

/// 转发器 trait
#[async_trait]
pub trait Forwarder: Send + Sync {
    /// 转发发现请求到下游服务器
    async fn forward(
        &self,
        request: protocols::adp::DiscoveryRequest,
        target_servers: Vec<String>,
    ) -> Result<Vec<protocols::adp::DiscoveryResult>, DiscoveryError>;
}

/// HTTP 转发器实现
pub struct HttpForwarder {
    client: reqwest::Client,
}

impl HttpForwarder {
    pub fn new() -> Result<Self, DiscoveryError> {
        let client = reqwest::Client::new();
        Ok(Self { client })
    }
}

#[async_trait]
impl Forwarder for HttpForwarder {
    async fn forward(
        &self,
        _request: protocols::adp::DiscoveryRequest,
        _target_servers: Vec<String>,
    ) -> Result<Vec<protocols::adp::DiscoveryResult>, DiscoveryError> {
        // TODO: 实现转发逻辑
        // 1. 检查转发深度和扇出限制
        // 2. 并发转发请求到下游服务器
        // 3. 聚合结果
        // 4. 处理超时和错误
        
        Ok(vec![])
    }
}

impl Default for HttpForwarder {
    fn default() -> Self {
        Self::new().expect("Failed to create HttpForwarder")
    }
}
