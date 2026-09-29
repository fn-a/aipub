//! ADP 请求校验与转发辅助
//!
//! 对齐参考实现 `acps-sdk/acps_sdk/adp/validators.py`。

use std::cmp::Ordering;
use serde_json::json;
use super::{
    AdpError, AdpErrorCode, DiscoveryRequest, FilterLogic, QueryType,
    FILTER_MAX_NESTING_DEPTH, FORWARD_DEPTH_LIMIT_DEFAULT, FORWARD_DEPTH_LIMIT_MAX,
    FORWARD_DEPTH_LIMIT_MIN, FORWARD_FANOUT_LIMIT_DEFAULT, FORWARD_FANOUT_LIMIT_MAX,
    FORWARD_FANOUT_LIMIT_MIN, FORWARD_EACH_TIMEOUT_MS_DEFAULT, FORWARD_TOTAL_TIMEOUT_MS_DEFAULT,
};

impl DiscoveryRequest {
    /// 解析查询类型（缺省为 `explicit`）
    pub fn query_type_kind(&self) -> QueryType {
        self.query_type
            .as_deref()
            .and_then(QueryType::from_str)
            .unwrap_or_default()
    }

    /// 有效的剩余扇出额度（`None` 时等于有效扇出限制，`0` 会被保留）
    pub fn get_effective_fanout_remaining(&self) -> usize {
        self.forward_fanout_remaining
            .unwrap_or_else(|| self.get_effective_fanout_limit())
    }

    /// 校验请求本身是否合法
    pub fn validate(&self) -> Result<(), AdpError> {
        validate_discovery_request(self)
    }
}

/// 校验发现请求
///
/// 校验顺序与参考实现一致：
/// 查询类型 -> explicit 查询内容 -> 转发深度 -> 转发扇出 -> 过滤条件嵌套深度。
pub fn validate_discovery_request(request: &DiscoveryRequest) -> Result<(), AdpError> {
    if let Some(raw_type) = request.query_type.as_deref() {
        if QueryType::from_str(raw_type).is_none() {
            return Err(AdpError::with_message(
                AdpErrorCode::FilterInvalid,
                format!(
                    "无效的查询类型: {:?}，允许的类型: explicit/exploratory/trending/filtered",
                    raw_type
                ),
            ));
        }
    }

    if request.query_type_kind() == QueryType::Explicit
        && request
            .query
            .as_deref()
            .map(|q| q.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(AdpError::with_message(
            AdpErrorCode::MissingQuery,
            "type='explicit' 时 query 字段必填且不能为空字符串",
        ));
    }

    if let Some(depth_limit) = request.forward_depth_limit {
        if !(FORWARD_DEPTH_LIMIT_MIN..=FORWARD_DEPTH_LIMIT_MAX).contains(&depth_limit) {
            return Err(AdpError::with_message(
                AdpErrorCode::ForwardDepthLimitInvalid,
                format!(
                    "forwardDepthLimit 必须在 [{}, {}] 区间，收到: {}",
                    FORWARD_DEPTH_LIMIT_MIN, FORWARD_DEPTH_LIMIT_MAX, depth_limit
                ),
            ));
        }
    }

    if let Some(fanout_limit) = request.forward_fanout_limit {
        if !(FORWARD_FANOUT_LIMIT_MIN..=FORWARD_FANOUT_LIMIT_MAX).contains(&fanout_limit) {
            return Err(AdpError::with_message(
                AdpErrorCode::ForwardFanoutLimitInvalid,
                format!(
                    "forwardFanoutLimit 必须在 [{}, {}] 区间，收到: {}",
                    FORWARD_FANOUT_LIMIT_MIN, FORWARD_FANOUT_LIMIT_MAX, fanout_limit
                ),
            ));
        }
    }

    if let Some(filter) = &request.filter {
        let depth = filter.depth();
        if depth > FILTER_MAX_NESTING_DEPTH {
            return Err(AdpError::with_message(
                AdpErrorCode::FilterInvalid,
                format!(
                    "过滤条件嵌套深度超过建议上限 {} 层，当前深度: {}",
                    FILTER_MAX_NESTING_DEPTH, depth
                ),
            ));
        }
    }

    Ok(())
}

/// 校验转发链
///
/// 1. 链路中不得出现当前服务器自身（环路）；
/// 2. `forwardChain.len() >= forwardDepthLimit` 视为深度超限；
/// 3. 提供 `sender_aic` 时，链路末位必须等于发送方 AIC（防篡改）。
pub fn validate_forward_chain(
    request: &DiscoveryRequest,
    current_server_aic: &str,
    sender_aic: Option<&str>,
) -> Result<(), AdpError> {
    let chain = request.forward_chain.clone().unwrap_or_default();
    let depth_limit = request.get_effective_depth_limit();

    if chain.iter().any(|aic| aic == current_server_aic) {
        return Err(AdpError::with_message(
            AdpErrorCode::ForwardLoopDetected,
            format!(
                "在 forwardChain 中检测到当前服务器 AIC {:?}，判定为环路。forwardChain: {:?}",
                current_server_aic, chain
            ),
        )
        .with_data(json!({
            "forwardChain": chain,
            "currentServerAic": current_server_aic,
        })));
    }

    if chain.len() >= depth_limit {
        return Err(AdpError::new(AdpErrorCode::ForwardDepthExceeded).with_data(json!({
            "forwardChain": chain,
            "forwardDepthLimit": depth_limit,
        })));
    }

    if let (Some(sender), Some(last)) = (sender_aic, chain.last()) {
        if last != sender {
            return Err(AdpError::new(AdpErrorCode::ForwardChainTampered).with_data(json!({
                "forwardChainLastAic": last,
                "senderAic": sender,
            })));
        }
    }

    Ok(())
}

/// 校验扇出预算是否足够
pub fn validate_fanout_budget(
    remaining: usize,
    required_branches: usize,
) -> Result<(), AdpError> {
    if remaining < required_branches {
        return Err(AdpError::new(AdpErrorCode::ForwardFanoutExceeded).with_data(json!({
            "availableBudget": remaining,
            "requiredBranches": required_branches,
        })));
    }
    Ok(())
}

/// 校验转发目标是否在信任列表内（列表为空视为不限制）
pub fn validate_trusted_target(request: &DiscoveryRequest, target_aic: &str) -> bool {
    match request.forward_trusted_servers.as_ref() {
        Some(servers) if !servers.is_empty() => servers.iter().any(|s| s == target_aic),
        _ => true,
    }
}

/// 是否还应继续向下一跳转发（总超时需大于单跳超时）
pub fn should_continue_forwarding(request: &DiscoveryRequest) -> bool {
    request.get_effective_total_timeout_ms() >= request.get_effective_each_timeout_ms()
}

/// 在转发前重新计算剩余总超时
pub fn remaining_total_timeout_ms(request: &DiscoveryRequest, elapsed_ms: u64) -> u64 {
    request
        .get_effective_total_timeout_ms()
        .saturating_sub(elapsed_ms)
}

/// 构造下一跳的转发请求
///
/// - 追加当前服务器 AIC 到 `forwardChain`；
/// - 刷新剩余扇出额度与剩余总超时；
/// - 可选覆盖信任服务器列表与转发签名；
/// - 补齐缺省的深度/扇出限制。
pub fn build_forwarded_request(
    request: &DiscoveryRequest,
    current_server_aic: &str,
    remaining_fanout: usize,
    elapsed_ms: u64,
    trusted_servers: Option<Vec<String>>,
    signature: Option<String>,
) -> DiscoveryRequest {
    let mut forwarded = request.clone();

    let mut chain = forwarded.forward_chain.clone().unwrap_or_default();
    chain.push(current_server_aic.to_string());
    forwarded.forward_chain = Some(chain);

    forwarded.forward_fanout_remaining = Some(remaining_fanout);
    forwarded.forward_total_timeout_ms = Some(remaining_total_timeout_ms(request, elapsed_ms));
    forwarded.forward_depth_limit = Some(request.get_effective_depth_limit());
    forwarded.forward_fanout_limit = Some(request.get_effective_fanout_limit());

    if let Some(servers) = trusted_servers {
        forwarded.forward_trusted_servers = Some(servers);
    }
    if let Some(signature) = signature {
        let mut signatures = forwarded.forward_signatures.clone().unwrap_or_default();
        signatures.push(signature);
        forwarded.forward_signatures = Some(signatures);
    }

    forwarded
}

/// 在多个转发分支间分配扇出预算
///
/// 每个分支至少保留 1 个额度，剩余额度按策略分配：
/// - 均分时余数从最后一个分支向前逐个分配；
/// - 加权时先按权重取整，余数按权重降序分配。
pub fn allocate_fanout_budget(
    total_remaining: usize,
    branch_count: usize,
    weights: Option<&[f64]>,
) -> Result<Vec<usize>, AdpError> {
    if branch_count == 0 {
        return Ok(vec![]);
    }

    validate_fanout_budget(total_remaining, branch_count)?;

    let distributable = total_remaining - branch_count;
    let mut allocation = vec![0usize; branch_count];

    match weights {
        None => {
            let base = distributable / branch_count;
            for slot in allocation.iter_mut() {
                *slot = base;
            }
            let remainder = distributable % branch_count;
            for i in 0..remainder {
                let index = branch_count - 1 - i;
                allocation[index] += 1;
            }
        }
        Some(weights) => {
            if weights.len() != branch_count {
                return Err(AdpError::with_message(
                    AdpErrorCode::ForwardFanoutLimitInvalid,
                    format!(
                        "权重列表长度 {} 与分支数 {} 不一致",
                        weights.len(),
                        branch_count
                    ),
                ));
            }

            let total_weight: f64 = weights.iter().sum();
            if total_weight <= 0.0 {
                return Ok(allocation);
            }

            let mut assigned = 0usize;
            for (slot, weight) in allocation.iter_mut().zip(weights.iter()) {
                let share = (distributable as f64 * weight / total_weight) as usize;
                *slot = share;
                assigned += share;
            }

            let mut remainder = distributable.saturating_sub(assigned);
            let mut order: Vec<usize> = (0..branch_count).collect();
            order.sort_by(|a, b| {
                weights[*b]
                    .partial_cmp(&weights[*a])
                    .unwrap_or(Ordering::Equal)
            });

            let mut cursor = 0;
            while remainder > 0 {
                allocation[order[cursor % branch_count]] += 1;
                remainder -= 1;
                cursor += 1;
            }
        }
    }

    Ok(allocation)
}

/// 规范化的逻辑关系（用于日志/调试输出）
pub fn filter_logic_name(request: &DiscoveryRequest) -> &'static str {
    request
        .filter
        .as_ref()
        .map(|filter| match filter.logic_kind() {
            FilterLogic::And => "and",
            FilterLogic::Or => "or",
            FilterLogic::Not => "not",
        })
        .unwrap_or("and")
}

/// 转发默认参数（用于构造初始请求）
pub fn default_forward_limits() -> (usize, usize, u64, u64) {
    (
        FORWARD_DEPTH_LIMIT_DEFAULT,
        FORWARD_FANOUT_LIMIT_DEFAULT,
        FORWARD_EACH_TIMEOUT_MS_DEFAULT,
        FORWARD_TOTAL_TIMEOUT_MS_DEFAULT,
    )
}
