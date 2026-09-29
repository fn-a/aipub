//! ADP 数据模型

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::{constants::*, DiscoveryFilter};

/// 发现请求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryRequest {
    /// 查询类型
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub query_type: Option<String>,
    
    /// 能力查询（自然语言描述）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    
    /// 上下文信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<DiscoveryContext>,
    
    /// 最大返回数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    
    /// 结构化过滤条件
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<DiscoveryFilter>,
    
    /// 转发深度限制
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardDepthLimit")]
    pub forward_depth_limit: Option<usize>,
    
    /// 转发扇出限制
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardFanoutLimit")]
    pub forward_fanout_limit: Option<usize>,
    
    /// 剩余扇出额度
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardFanoutRemaining")]
    pub forward_fanout_remaining: Option<usize>,
    
    /// 转发追踪链
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardChain")]
    pub forward_chain: Option<Vec<String>>,
    
    /// 信任的发现服务器列表
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardTrustedServers")]
    pub forward_trusted_servers: Option<Vec<String>>,
    
    /// 转发签名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardSignatures")]
    pub forward_signatures: Option<Vec<String>>,
    
    /// 单跳超时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardEachTimeoutMs")]
    pub forward_each_timeout_ms: Option<u64>,
    
    /// 总超时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forwardTotalTimeoutMs")]
    pub forward_total_timeout_ms: Option<u64>,
}

impl DiscoveryRequest {
    /// 创建新的发现请求
    pub fn new() -> Self {
        Self {
            query_type: Some("explicit".to_string()),
            query: None,
            context: None,
            limit: None,
            filter: None,
            forward_depth_limit: None,
            forward_fanout_limit: None,
            forward_fanout_remaining: None,
            forward_chain: None,
            forward_trusted_servers: None,
            forward_signatures: None,
            forward_each_timeout_ms: None,
            forward_total_timeout_ms: None,
        }
    }
    
    /// 设置查询
    pub fn with_query(mut self, query: &str) -> Self {
        self.query = Some(query.to_string());
        self
    }
    
    /// 设置限制
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// 设置过滤条件
    pub fn with_filter(mut self, filter: DiscoveryFilter) -> Self {
        self.filter = Some(filter);
        self
    }
    
    /// 设置查询类型
    pub fn with_query_type(mut self, query_type: &str) -> Self {
        self.query_type = Some(query_type.to_string());
        self
    }
    
    /// 获取有效的转发深度限制
    pub fn get_effective_depth_limit(&self) -> usize {
        self.forward_depth_limit.unwrap_or(FORWARD_DEPTH_LIMIT_DEFAULT)
    }
    
    /// 获取有效的转发扇出限制
    pub fn get_effective_fanout_limit(&self) -> usize {
        self.forward_fanout_limit.unwrap_or(FORWARD_FANOUT_LIMIT_DEFAULT)
    }
    
    /// 获取有效的单跳超时
    pub fn get_effective_each_timeout_ms(&self) -> u64 {
        self.forward_each_timeout_ms.unwrap_or(FORWARD_EACH_TIMEOUT_MS_DEFAULT)
    }
    
    /// 获取有效的总超时
    pub fn get_effective_total_timeout_ms(&self) -> u64 {
        self.forward_total_timeout_ms.unwrap_or(FORWARD_TOTAL_TIMEOUT_MS_DEFAULT)
    }
}

impl Default for DiscoveryRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// 发现上下文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryContext {
    /// 会话ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    
    /// 近期对话摘要
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "recentTurns")]
    pub recent_turns: Option<Vec<String>>,
    
    /// 用户画像
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "userProfile")]
    pub user_profile: Option<Value>,
    
    /// 额外元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 发现响应
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResponse {
    /// 结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<DiscoveryResult>,
    
    /// 错误
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

impl DiscoveryResponse {
    /// 创建成功响应
    pub fn success(result: DiscoveryResult) -> Self {
        Self {
            result: Some(result),
            error: None,
        }
    }
    
    /// 创建失败响应
    pub fn failure(code: i32, message: &str) -> Self {
        Self {
            result: None,
            error: Some(ErrorDetail {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
    
    /// 判断是否成功
    pub fn is_success(&self) -> bool {
        self.result.is_some()
    }
}

/// 发现结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    /// ACS 数据映射（AIC -> ACS）
    #[serde(rename = "acsMap")]
    pub acs_map: HashMap<String, Value>,
    
    /// 发现到的候选智能体列表
    pub agents: Vec<DiscoveryAgentGroup>,
    
    /// 下游响应路由集合
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<DiscoveryRoute>>,
}

impl DiscoveryResult {
    /// 创建新的发现结果
    pub fn new() -> Self {
        Self {
            acs_map: HashMap::new(),
            agents: vec![],
            routes: None,
        }
    }
}

impl Default for DiscoveryResult {
    fn default() -> Self {
        Self::new()
    }
}

/// 按分组组织的智能体匹配结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryAgentGroup {
    /// 分组标识
    pub group: String,
    
    /// 该分组下匹配到的智能体技能列表
    #[serde(rename = "agentSkills")]
    pub agent_skills: Vec<DiscoveryAgentSkill>,
}

/// 单个智能体技能的匹配结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryAgentSkill {
    /// 智能体身份码
    pub aic: String,
    
    /// 技能ID
    #[serde(rename = "skillId")]
    pub skill_id: String,
    
    /// 匹配排名
    pub ranking: usize,
    
    /// 备注信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// 发现结果的路由级细分
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryRoute {
    /// 转发链路
    #[serde(rename = "forwardChain")]
    pub forward_chain: Vec<String>,
    
    /// 按分组组织的候选智能体列表
    #[serde(rename = "agentGroups")]
    pub agent_groups: Vec<DiscoveryAgentGroup>,
    
    /// 响应状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    
    /// 总耗时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
    
    /// 额外扩展信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 错误详情
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// 错误代码
    pub code: i32,
    
    /// 错误消息
    pub message: String,
    
    /// 错误数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
