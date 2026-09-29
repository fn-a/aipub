//! Discovery 转发器
//!
//! 支持跨域发现：把请求转发给下游发现服务并聚合结果。
//! 参考实现（`app/discovery/service.py`）在开启转发时会把下游响应原样返回，
//! 本实现按 ADP 规范的 `routes` 语义做聚合，保留本地结果并在
//! [`protocols::adp::DiscoveryRoute`] 中给出每一跳的状态与耗时。

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::time;
use async_trait::async_trait;
use serde_json::{json, Value};
use protocols::adp::{
    build_forwarded_request, validate_fanout_budget, validate_forward_chain,
    DiscoveryAgentGroup, DiscoveryRequest, DiscoveryResult, DiscoveryRoute,
    DISCOVERY_ENDPOINT_PATH, FORWARD_EACH_TIMEOUT_MS_DEFAULT, DiscoveryResponse,
};
use crate::DiscoveryError;

/// 转发器 trait
#[async_trait]
pub trait Forwarder: Send + Sync {
    /// 转发发现请求到下游服务器
    async fn forward(
        &self,
        request: DiscoveryRequest,
        target_servers: Vec<String>,
    ) -> Result<Vec<DiscoveryResult>, DiscoveryError>;
}

/// HTTP 转发器实现
///
/// 通过 `POST {server}/discover` 调用下游发现服务（ADP 约定的端点路径）。
pub struct HttpForwarder {
    client: reqwest::Client,
    /// 当前服务器自身的 AIC（写入转发链）
    local_aic: String,
}

impl HttpForwarder {
    /// 创建转发器
    pub fn new(local_aic: impl Into<String>) -> Result<Self, DiscoveryError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| DiscoveryError::ForwardError(e.to_string()))?;
        Ok(Self {
            client,
            local_aic: local_aic.into(),
        })
    }

    /// 当前服务器 AIC
    pub fn local_aic(&self) -> &str {
        &self.local_aic
    }

    /// 请求 URL
    fn endpoint(server: &str) -> String {
        format!("{}{}", server.trim_end_matches('/'), DISCOVERY_ENDPOINT_PATH)
    }

    /// 单跳转发
    async fn forward_one(
        &self,
        request: &DiscoveryRequest,
        server: &str,
        remaining_fanout: usize,
        elapsed_ms: u64,
    ) -> DiscoveryRoute {
        let started = Instant::now();
        let forwarded = build_forwarded_request(
            request,
            &self.local_aic,
            remaining_fanout,
            elapsed_ms,
            None,
            None,
        );

        let per_hop_timeout = Duration::from_millis(if forwarded.forward_each_timeout_ms.is_some() {
            forwarded.get_effective_each_timeout_ms()
        } else {
            FORWARD_EACH_TIMEOUT_MS_DEFAULT
        });

        let response = time::timeout(
            per_hop_timeout,
            self.client
                .post(Self::endpoint(server))
                .json(&forwarded)
                .send(),
        )
        .await;

        let duration_ms = started.elapsed().as_millis() as u64;
        let mut forward_chain = forwarded.forward_chain.clone().unwrap_or_default();

        match response {
            Err(_) => DiscoveryRoute {
                forward_chain,
                agent_groups: vec![],
                status: Some("timeout".to_string()),
                duration_ms: Some(duration_ms),
                metadata: Some(json!({ "server": server })),
            },
            Ok(Err(error)) => DiscoveryRoute {
                forward_chain,
                agent_groups: vec![],
                status: Some("error".to_string()),
                duration_ms: Some(duration_ms),
                metadata: Some(json!({
                    "server": server,
                    "error": error.to_string(),
                })),
            },
            Ok(Ok(response)) => {
                let parsed: Result<DiscoveryResponse, _> = response.json().await;
                match parsed {
                    Ok(discovery_response) => {
                        let groups = discovery_response
                            .result
                            .map(|result| result.agents)
                            .unwrap_or_default();
                        if forward_chain.is_empty() {
                            forward_chain.push(server.to_string());
                        }
                        DiscoveryRoute {
                            forward_chain,
                            agent_groups: groups,
                            status: Some("ok".to_string()),
                            duration_ms: Some(duration_ms),
                            metadata: Some(json!({ "server": server })),
                        }
                    }
                    Err(error) => DiscoveryRoute {
                        forward_chain,
                        agent_groups: vec![],
                        status: Some("error".to_string()),
                        duration_ms: Some(duration_ms),
                        metadata: Some(json!({
                            "server": server,
                            "error": error.to_string(),
                        })),
                    },
                }
            }
        }
    }
}

#[async_trait]
impl Forwarder for HttpForwarder {
    async fn forward(
        &self,
        request: DiscoveryRequest,
        target_servers: Vec<String>,
    ) -> Result<Vec<DiscoveryResult>, DiscoveryError> {
        if target_servers.is_empty() {
            return Ok(vec![]);
        }

        // 环路与深度校验
        validate_forward_chain(&request, &self.local_aic, None)
            .map_err(|e| DiscoveryError::ForwardError(e.to_string()))?;

        let fanout_limit = request.get_effective_fanout_limit();
        validate_fanout_budget(fanout_limit, target_servers.len())
            .map_err(|e| DiscoveryError::ForwardError(e.to_string()))?;

        let remaining = request.get_effective_fanout_remaining();
        let each_budget = if target_servers.is_empty() {
            0
        } else {
            remaining / target_servers.len()
        };

        let mut routes = Vec::new();
        for server in target_servers.iter().take(fanout_limit) {
            let route = self.forward_one(&request, server, each_budget, 0).await;
            routes.push(route);
        }

        let result = DiscoveryResult {
            acs_map: Default::default(),
            agents: vec![],
            routes: Some(routes),
        };

        Ok(vec![result])
    }
}

/// 进程内转发器
///
/// 直接把请求交给另一个 [`crate::DiscoveryService`]，便于在单进程内
/// 演示/测试跨域发现，无需真实网络。
pub struct InProcessForwarder {
    downstream: Arc<dyn crate::DiscoveryService>,
    local_aic: String,
    downstream_aic: String,
}

impl InProcessForwarder {
    /// 创建进程内转发器
    pub fn new(
        downstream: Arc<dyn crate::DiscoveryService>,
        local_aic: impl Into<String>,
        downstream_aic: impl Into<String>,
    ) -> Self {
        Self {
            downstream,
            local_aic: local_aic.into(),
            downstream_aic: downstream_aic.into(),
        }
    }
}

#[async_trait]
impl Forwarder for InProcessForwarder {
    async fn forward(
        &self,
        request: DiscoveryRequest,
        _target_servers: Vec<String>,
    ) -> Result<Vec<DiscoveryResult>, DiscoveryError> {
        let started = Instant::now();

        let forwarded = build_forwarded_request(
            &request,
            &self.local_aic,
            request.get_effective_fanout_remaining(),
            0,
            None,
            None,
        );

        let response = self.downstream.discover(forwarded.clone()).await?;
        let duration_ms = started.elapsed().as_millis() as u64;

        let mut forward_chain = forwarded.forward_chain.unwrap_or_default();
        if forward_chain.is_empty() {
            forward_chain.push(self.downstream_aic.clone());
        }

        let (agent_groups, status) = match response.result {
            Some(result) => (result.agents, "ok".to_string()),
            None => (vec![], "error".to_string()),
        };

        Ok(vec![DiscoveryResult {
            acs_map: Default::default(),
            agents: vec![],
            routes: Some(vec![DiscoveryRoute {
                forward_chain,
                agent_groups,
                status: Some(status),
                duration_ms: Some(duration_ms),
                metadata: Some(json!({ "server": self.downstream_aic })),
            }]),
        }])
    }
}

/// 合并多个下游结果到主结果
pub fn merge_forwarded_results(
    result: &mut DiscoveryResult,
    forwarded: Vec<DiscoveryResult>,
    acs_map: &mut HashMap<String, Value>,
) {
    for downstream in forwarded {
        acs_map.extend(downstream.acs_map);

        if let Some(routes) = downstream.routes {
            let target = result.routes.get_or_insert_with(Vec::new);
            for route in routes {
                // 下游分组结果并入本地分组列表
                for group in &route.agent_groups {
                    merge_group(&mut result.agents, group);
                }
                target.push(route);
            }
        }

        for group in downstream.agents {
            merge_group(&mut result.agents, &group);
        }
    }
}

/// 合并同名分组
fn merge_group(groups: &mut Vec<DiscoveryAgentGroup>, incoming: &DiscoveryAgentGroup) {
    match groups.iter_mut().find(|g| g.group == incoming.group) {
        Some(existing) => {
            for skill in &incoming.agent_skills {
                if !existing
                    .agent_skills
                    .iter()
                    .any(|s| s.aic == skill.aic && s.skill_id == skill.skill_id)
                {
                    existing.agent_skills.push(skill.clone());
                }
            }
        }
        None => groups.push(incoming.clone()),
    }
}
