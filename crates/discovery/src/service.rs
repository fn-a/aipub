//! Discovery 服务核心实现
//!
//! 覆盖 ADP 的四种查询模式：
//!
//! | 查询类型 | 行为 |
//! |---|---|
//! | `explicit` | 关键词匹配；最高分低于阈值时自动降级为探索性查询 |
//! | `exploratory` | 返回可用智能体；提供 `query` 时按关键词排序 |
//! | `trending` | 过滤后随机打散 |
//! | `filtered` | 仅使用结构化过滤条件，忽略 `query` |
//!
//! 结果按参考实现的形状组织：`acsMap`（AIC -> ACS）、`agents`
//! （按分组组织的 `{aic, skillId, ranking, memo}`）与 `routes`。

use std::mem;
use std::time::Instant;
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;
use serde_json::{json, Value};
use protocols::dsp::ChangeEntry;
use protocols::adp::{
    validate_discovery_request, DiscoveryAgentGroup, DiscoveryAgentSkill,
    DiscoveryRequest, DiscoveryResponse, DiscoveryResult, DiscoveryRoute, FilterCondition,
    QueryType, DEFAULT_LIMIT, DiscoveryFilter,
};
use crate::{DiscoveryConfig, DiscoveryError};

/// Discovery 服务 trait
#[async_trait]
pub trait DiscoveryService: Send + Sync {
    /// 执行发现查询
    async fn discover(&self, request: DiscoveryRequest) -> Result<DiscoveryResponse, DiscoveryError>;
    
    /// 注册智能体 ACS
    async fn register_agent(&self, acs: Value) -> Result<(), DiscoveryError>;
    
    /// 注销智能体
    async fn unregister_agent(&self, aic: &str) -> Result<(), DiscoveryError>;
    
    /// 更新智能体 ACS
    async fn update_agent(&self, aic: &str, acs: Value) -> Result<(), DiscoveryError>;
}

/// Discovery 服务实现
pub struct DiscoveryServiceImpl {
    config: DiscoveryConfig,
    storage: Box<dyn crate::Storage>,
    matcher: Box<dyn crate::Matcher>,
    forwarder: Option<Box<dyn crate::Forwarder>>,
}

impl DiscoveryServiceImpl {
    pub fn new(
        config: DiscoveryConfig,
        storage: Box<dyn crate::Storage>,
        matcher: Box<dyn crate::Matcher>,
        forwarder: Option<Box<dyn crate::Forwarder>>,
    ) -> Self {
        Self {
            config,
            storage,
            matcher,
            forwarder,
        }
    }

    /// 当前配置
    pub fn config(&self) -> &DiscoveryConfig {
        &self.config
    }

    /// 读取智能体 ACS
    pub async fn get_agent(&self, aic: &str) -> Result<Option<Value>, DiscoveryError> {
        self.storage.get(aic).await
    }

    /// 应用一条注册服务变更（DSP）
    pub async fn apply_change(&self, entry: &ChangeEntry) -> Result<bool, DiscoveryError> {
        self.storage.apply_change(entry).await
    }

    /// 批量应用变更，返回已应用条数
    pub async fn apply_changes(&self, entries: &[ChangeEntry]) -> Result<usize, DiscoveryError> {
        let mut applied = 0;
        for entry in entries {
            if self.storage.apply_change(entry).await? {
                applied += 1;
            }
        }
        Ok(applied)
    }

    /// 统计信息
    pub async fn stats(&self) -> Result<DiscoveryStats, DiscoveryError> {
        let records = self.storage.records().await?;
        let active = records
            .iter()
            .filter(|record| is_discoverable(&record.acs))
            .count();
        Ok(DiscoveryStats {
            total_agents: records.len(),
            available_agents: active,
        })
    }

    /// 可发现候选集合（默认门控：`active` 为真且存在端点或 Web 应用入口）
    async fn candidates(&self, request: &DiscoveryRequest) -> Result<Vec<Value>, DiscoveryError> {
        let matched = self.storage.query(request.filter.clone()).await?;
        Ok(matched
            .into_iter()
            .filter(is_discoverable)
            .collect())
    }

    /// 按查询类型分派
    async fn dispatch(
        &self,
        request: &DiscoveryRequest,
        candidates: Vec<Value>,
        limit: usize,
    ) -> Result<(String, Vec<crate::MatchResult>), DiscoveryError> {
        let query = request.query.clone().unwrap_or_default();

        match request.query_type_kind() {
            QueryType::Filtered => {
                let group = if query.is_empty() {
                    "filtered".to_string()
                } else {
                    query.clone()
                };
                let results = candidates
                    .into_iter()
                    .take(limit)
                    .enumerate()
                    .filter_map(|(index, acs)| to_filtered_result(&acs, index + 1))
                    .collect();
                Ok((group, results))
            }

            QueryType::Trending => {
                let mut shuffled = candidates;
                shuffle(&mut shuffled);
                let results = shuffled
                    .into_iter()
                    .take(limit)
                    .enumerate()
                    .map(|(index, acs)| to_plain_result(&acs, index + 1, "trending"))
                    .collect();
                Ok(("trending".to_string(), results))
            }

            QueryType::Exploratory => {
                if query.trim().is_empty() {
                    let results = candidates
                        .into_iter()
                        .take(limit)
                        .enumerate()
                        .map(|(index, acs)| to_plain_result(&acs, index + 1, "exploratory"))
                        .collect();
                    Ok((String::new(), results))
                } else {
                    let results = self.matcher.match_agents(&query, candidates, limit).await?;
                    Ok((query, results))
                }
            }

            QueryType::Explicit => {
                let results = self
                    .matcher
                    .match_agents(&query, candidates.clone(), limit)
                    .await?;

                let best = results.first().map(|r| r.score).unwrap_or(0.0);
                if self.config.enable_exploratory_fallback
                    && (results.is_empty() || best < self.config.match_threshold)
                {
                    // 明确查询命中过弱时降级为探索性结果，避免直接返回空集
                    let fallback = self
                        .matcher
                        .match_agents(&query, candidates, limit)
                        .await
                        .unwrap_or_default();
                    if fallback.is_empty() {
                        let relaxed = self.relaxed_results(request, limit).await?;
                        return Ok((query, relaxed));
                    }
                    return Ok((query, fallback));
                }

                Ok((query, results))
            }
        }
    }

    /// 降级结果：忽略查询词，按过滤条件返回可用智能体
    async fn relaxed_results(
        &self,
        request: &DiscoveryRequest,
        limit: usize,
    ) -> Result<Vec<crate::MatchResult>, DiscoveryError> {
        let mut filters = vec![FilterCondition::eq("active", json!(true))];
        if let Some(original) = &request.filter {
            if let Some(conditions) = original.conditions.clone() {
                filters.extend(conditions);
            }
        }

        let filter = DiscoveryFilter {
            conditions: Some(filters),
            groups: request.filter.as_ref().and_then(|f| f.groups.clone()),
            logic: Some("and".to_string()),
        };

        let candidates: Vec<Value> = self
            .storage
            .query(Some(filter))
            .await?
            .into_iter()
            .filter(is_discoverable)
            .take(limit)
            .collect();

        Ok(candidates
            .into_iter()
            .enumerate()
            .map(|(index, acs)| to_plain_result(&acs, index + 1, "exploratory"))
            .collect())
    }
}

#[async_trait]
impl DiscoveryService for DiscoveryServiceImpl {
    async fn discover(&self, request: DiscoveryRequest) -> Result<DiscoveryResponse, DiscoveryError> {
        let started = Instant::now();

        // 1. 请求校验（失败时按 ADP 错误体返回）
        if let Err(error) = validate_discovery_request(&request) {
            return Ok(DiscoveryResponse::failure(error.code, &error.message));
        }

        // 2. 过滤查询必须提供 filter
        if request.query_type_kind() == QueryType::Filtered && request.filter.is_none() {
            return Ok(DiscoveryResponse::failure(
                40004,
                "filtered 查询必须提供 filter 参数",
            ));
        }

        let limit = request.limit.unwrap_or(DEFAULT_LIMIT).max(1);

        // 3. 候选集合 + 查询分派
        let candidates = self.candidates(&request).await?;
        let (group_name, matches) = self.dispatch(&request, candidates, limit).await?;

        // 4. 组装结果
        let mut acs_map = HashMap::new();
        let mut agent_skills = Vec::new();
        for matched in &matches {
            if let Some(acs) = self.storage.get(&matched.aic).await? {
                acs_map.insert(matched.aic.clone(), acs);
            }
            agent_skills.push(DiscoveryAgentSkill {
                aic: matched.aic.clone(),
                skill_id: matched.skill_id.clone(),
                ranking: matched.ranking,
                memo: Some(matched.memo.clone()),
            });
        }

        let groups = vec![DiscoveryAgentGroup {
            group: group_name,
            agent_skills,
        }];

        let local_chain = if request.forward_chain.clone().unwrap_or_default().is_empty() {
            vec![self.config.local_server_aic.clone()]
        } else {
            request.forward_chain.clone().unwrap_or_default()
        };

        let mut result = DiscoveryResult {
            acs_map,
            agents: groups.clone(),
            routes: Some(vec![DiscoveryRoute {
                forward_chain: local_chain,
                agent_groups: groups,
                status: Some("ok".to_string()),
                duration_ms: Some(started.elapsed().as_millis() as u64),
                metadata: None,
            }]),
        };

        // 5. 跨域转发（filtered/trending 不转发，与参考实现一致）
        let forwardable = matches!(
            request.query_type_kind(),
            QueryType::Explicit | QueryType::Exploratory
        );
        if self.config.enable_forward && forwardable {
            if let Some(forwarder) = self.forwarder.as_ref() {
                if !self.config.forward_trusted_servers.is_empty() {
                    let forwarded = forwarder
                        .forward(request.clone(), self.config.forward_trusted_servers.clone())
                        .await?;
                    let mut acs_map = mem::take(&mut result.acs_map);
                    crate::forwarder::merge_forwarded_results(&mut result, forwarded, &mut acs_map);
                    result.acs_map = acs_map;
                }
            }
        }

        Ok(DiscoveryResponse::success(result))
    }
    
    async fn register_agent(&self, acs: Value) -> Result<(), DiscoveryError> {
        self.storage.store(acs).await
    }
    
    async fn unregister_agent(&self, aic: &str) -> Result<(), DiscoveryError> {
        self.storage.remove(aic).await
    }
    
    async fn update_agent(&self, aic: &str, acs: Value) -> Result<(), DiscoveryError> {
        self.storage.update(aic, acs).await
    }
}

/// 发现服务统计
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveryStats {
    /// 已同步的智能体总数
    pub total_agents: usize,

    /// 可被发现的智能体数
    pub available_agents: usize,
}

/// 默认门控：激活且存在服务入口
pub fn is_discoverable(acs: &Value) -> bool {
    let active = acs
        .get("active")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !active {
        return false;
    }

    let has_endpoint = acs
        .get("endPoints")
        .and_then(|value| value.as_array())
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let has_web_app = acs
        .get("webAppUrl")
        .and_then(|value| value.as_str())
        .map(|url| !url.trim().is_empty())
        .unwrap_or(false);

    has_endpoint || has_web_app
}

/// 构造仅带排名的匹配结果
fn to_plain_result(acs: &Value, ranking: usize, memo: &str) -> crate::MatchResult {
    let aic = acs
        .get("aic")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    crate::MatchResult {
        aic,
        skill_id: String::new(),
        score: 0.0,
        ranking,
        memo: memo.to_string(),
    }
}

/// 构造过滤查询结果（取该智能体的第一个技能）
fn to_filtered_result(acs: &Value, ranking: usize) -> Option<crate::MatchResult> {
    let aic = acs.get("aic").and_then(|value| value.as_str())?;
    let skill_id = acs
        .get("skills")
        .and_then(|value| value.as_array())
        .and_then(|skills| skills.first())
        .and_then(|skill| skill.get("id"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();

    Some(crate::MatchResult {
        aic: aic.to_string(),
        skill_id,
        score: 0.0,
        ranking,
        memo: "Filtered query result".to_string(),
    })
}

/// 用随机种子打散列表（trending 模式）
fn shuffle(items: &mut [Value]) {
    if items.len() < 2 {
        return;
    }

    let seed = Uuid::new_v4().as_u128();
    let modulus = items.len() as u128;
    let offset = (seed % modulus) as usize;

    let mut rotated: Vec<Value> = items.to_vec();
    rotated.rotate_left(offset);
    items.clone_from_slice(&rotated);
}
