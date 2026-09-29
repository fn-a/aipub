//! ACS 能力相关辅助函数和类型

use std::collections::HashSet;
use regex::Regex;
use sha2::{Digest, Sha256};
use serde_json::to_string;
use crate::aic::validate_aic_format;
use super::{AgentCapabilitySpec, AgentEndPoint, SecurityScheme};

lazy_static::lazy_static! {
    /// 消息队列协议版本表达式正则（与参考实现 `acsSchema.json` 一致）
    static ref RE_MQ_PROTOCOL_VERSION: Regex = Regex::new(
        r"^(mqtt|amqp|kafka|redis|rabbitmq):(>=)?\d+\.(\*|\d+(\.\d+)?)( <\d+\.\d+(\.\d+)?)?$"
    ).expect("MQ 协议版本正则有效");
}

/// 传输协议与允许的 URL scheme 映射
///
/// 与参考实现 `registry-server/app/utils/acs.py` 的 `_TRANSPORT_SCHEME_MAP` 一致；
/// 未知传输协议不做 scheme 限制。
pub const TRANSPORT_SCHEME_MAP: [(&str, &[&str]); 5] = [
    ("AMQP", &["amqp", "amqps"]),
    ("GRPC", &["grpc", "grpcs", "http", "https"]),
    ("HTTP_JSON", &["http", "https"]),
    ("JSONRPC", &["http", "https"]),
    ("REST", &["http", "https"]),
];

/// 校验消息队列协议版本表达式是否符合规范
pub fn validate_mq_protocol_version(value: &str) -> Result<(), String> {
    if RE_MQ_PROTOCOL_VERSION.is_match(value) {
        Ok(())
    } else {
        Err(format!("消息队列协议版本不符合规范: '{}'", value))
    }
}

/// 解析 URL 的 scheme 与 host（netloc）
pub fn parse_url_scheme_and_host(url: &str) -> Option<(String, String)> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() {
        return None;
    }
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_string();
    if host.is_empty() {
        return None;
    }
    Some((scheme.to_lowercase(), host))
}

impl AgentCapabilitySpec {
    /// 验证 ACS 的基本有效性
    ///
    /// 依次校验：
    /// 1. AIC 格式；
    /// 2. 协议版本、名称、描述、版本号非空；
    /// 3. `messageQueue` 中的协议版本表达式符合规范；
    /// 4. 技能标识、名称、描述、版本非空且不重复；
    /// 5. 端点使用 mutualTLS 安全方案，且 URL scheme 与 transport 匹配。
    pub fn validate(&self) -> Result<(), String> {
        // 验证 AIC
        validate_aic_format(&self.aic)?;

        // 验证协议版本
        if self.protocol_version.is_empty() {
            return Err("协议版本不能为空".to_string());
        }

        // 验证名称
        if self.name.trim().is_empty() {
            return Err("智能体名称不能为空".to_string());
        }

        // 验证描述
        if self.description.trim().is_empty() {
            return Err("智能体描述不能为空".to_string());
        }

        // 验证版本号
        if self.version.trim().is_empty() {
            return Err("智能体版本号不能为空".to_string());
        }

        // 消息队列协议版本
        for mq in &self.capabilities.message_queue {
            validate_mq_protocol_version(mq.as_wire())?;
        }

        // 技能校验
        let mut seen_skill_ids = HashSet::new();
        for skill in &self.skills {
            if skill.id.trim().is_empty() {
                return Err("技能 ID 不能为空".to_string());
            }
            if skill.name.trim().is_empty() {
                return Err(format!("技能 {} 的名称不能为空", skill.id));
            }
            if skill.description.trim().is_empty() {
                return Err(format!("技能 {} 的描述不能为空", skill.id));
            }
            if skill.version.trim().is_empty() {
                return Err(format!("技能 {} 的版本号不能为空", skill.id));
            }
            if !seen_skill_ids.insert(skill.id.as_str()) {
                return Err(format!("技能 ID 重复: {}", skill.id));
            }
        }

        self.validate_endpoints()?;

        Ok(())
    }

    /// 收集类型为 `mutualTLS` 的安全方案名称
    pub fn mutual_tls_scheme_names(&self) -> Vec<&str> {
        self.security_schemes
            .iter()
            .filter(|(_, scheme)| matches!(scheme, SecurityScheme::MutualTls(_)))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// 校验服务端点
    ///
    /// 参考实现的强制规则：
    /// - 每个端点的 `security` 必须引用一个 `mutualTLS` 安全方案；
    /// - 端点 URL 必须带 scheme 与 host，且 scheme 与 `transport` 匹配。
    pub fn validate_endpoints(&self) -> Result<(), String> {
        if self.end_points.is_empty() {
            return Ok(());
        }

        let mtls_names = self.mutual_tls_scheme_names();

        for endpoint in &self.end_points {
            if !endpoint_uses_mutual_tls(endpoint, &mtls_names) {
                return Err("endPoint must use mutualTLS security scheme".to_string());
            }
            validate_endpoint_url(endpoint)?;
        }

        Ok(())
    }

    /// 替换端点 URL 中的 `{AIC}` 占位符
    pub fn resolve_aic_placeholder(&mut self) {
        for endpoint in self.end_points.iter_mut() {
            if endpoint.url.contains(super::AIC_URL_PLACEHOLDER) {
                endpoint.url = endpoint
                    .url
                    .replace(super::AIC_URL_PLACEHOLDER, &self.aic);
            }
        }
    }

    /// 由注册服务写入身份元数据（AIC、激活状态、最后修改时间）
    ///
    /// 同时解析端点 URL 中的 `{AIC}` 占位符，等价于参考实现审批通过后的改写。
    pub fn apply_registry_metadata(
        &mut self,
        aic: &str,
        active: bool,
        last_modified_time: &str,
    ) {
        self.aic = aic.to_string();
        self.active = active;
        self.last_modified_time = last_modified_time.to_string();
        self.resolve_aic_placeholder();
    }

    /// 计算 ACS 内容哈希（SHA-256，小写十六进制）
    ///
    /// 与参考实现的 `acs_hash = sha256(json.dumps(acs, ensure_ascii=False))` 对应：
    /// 这里对 `serde_json` 的紧凑序列化结果求哈希。
    pub fn acs_hash(&self) -> Result<String, String> {
        let json = to_string(self).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// 检查智能体是否支持流式响应
    pub fn supports_streaming(&self) -> bool {
        self.capabilities.streaming
    }

    /// 检查智能体是否支持异步通知
    pub fn supports_notification(&self) -> bool {
        self.capabilities.notification
    }

    /// 检查智能体是否支持指定的消息队列协议
    pub fn supports_mq_protocol(&self, protocol_name: &str) -> bool {
        self.capabilities
            .message_queue
            .iter()
            .any(|mq| {
                mq.protocol_name().eq_ignore_ascii_case(protocol_name)
                    || mq.protocol_family().eq_ignore_ascii_case(protocol_name)
            })
    }

    /// 获取指定ID的技能
    pub fn get_skill(&self, skill_id: &str) -> Option<&super::AgentSkill> {
        self.skills.iter().find(|skill| skill.id == skill_id)
    }

    /// 检查是否拥有指定ID的技能
    pub fn has_skill(&self, skill_id: &str) -> bool {
        self.get_skill(skill_id).is_some()
    }

    /// 获取所有技能标签
    pub fn get_all_tags(&self) -> Vec<&str> {
        self.skills
            .iter()
            .flat_map(|skill| skill.tags.iter())
            .map(|s| s.as_str())
            .collect()
    }

    /// 检查是否拥有指定标签的技能
    pub fn has_skill_with_tag(&self, tag: &str) -> bool {
        self.skills
            .iter()
            .any(|skill| skill.tags.iter().any(|t| t == tag))
    }

    /// 是否可被默认发现（`active` 为真，且存在端点或 Web 应用入口）
    pub fn is_discoverable(&self) -> bool {
        self.active && (!self.end_points.is_empty() || self.web_app_url.is_some())
    }
}

/// 判断端点是否引用了 mutualTLS 安全方案
fn endpoint_uses_mutual_tls(endpoint: &AgentEndPoint, mtls_names: &[&str]) -> bool {
    let Some(security) = endpoint.security.as_ref() else {
        return false;
    };

    security.iter().any(|requirement| {
        requirement
            .keys()
            .any(|name| mtls_names.contains(&name.as_str()))
    })
}

/// 校验端点 URL 与传输协议是否匹配
fn validate_endpoint_url(endpoint: &AgentEndPoint) -> Result<(), String> {
    let (scheme, _host) = parse_url_scheme_and_host(&endpoint.url)
        .ok_or_else(|| format!("endpoint URL format: {}", endpoint.url))?;

    let transport = endpoint.transport.to_uppercase();
    match TRANSPORT_SCHEME_MAP
        .iter()
        .find(|(name, _)| *name == transport)
    {
        Some((_, allowed)) => {
            if allowed.contains(&scheme.as_str()) {
                Ok(())
            } else {
                Err(format!("endpoint URL format: {}", endpoint.url))
            }
        }
        // 未知传输协议不做限制
        None => Ok(()),
    }
}
