//! ACL (访问控制列表) 与消息队列鉴权决策
//!
//! 对齐参考实现 `mq-auth-server` 的认证后端语义：
//!
//! - `/auth/user`：用户名必须是合法 AIC；
//! - `/auth/vhost`：用户名合法且 vhost 为 `acps`；
//! - `/auth/resource`：按队列/交换机命名规则判定；
//! - `/auth/topic`：仅 `inbox.topic` 主题交换机，且按路由键限定。
//!
//! 注意：参考实现**不校验密码**——任何合法 AIC 用户名配合任意密码都可通过
//! PLAIN 认证，真正的权限控制来自 mTLS 客户端证书与群组 ACL。

use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use tokio::sync::RwLock;
use protocols::aic::validate_aic_format;
use crate::AuthorizeError;

/// 共享 vhost
pub const SHARED_VHOST: &str = "acps";

/// 收件箱主题交换机名
pub const INBOX_TOPIC_EXCHANGE: &str = "inbox.topic";

/// 默认交换机名
pub const DEFAULT_EXCHANGE: &str = "amq.default";

/// 成员队列/交换机命名前缀与分隔符
pub const GROUP_RESOURCE_PREFIX: &str = "group_";
pub const INBOX_QUEUE_PREFIX: &str = "inbox_";

/// ACL 管理器 trait
#[async_trait]
pub trait AclManager: Send + Sync {
    /// 添加权限
    async fn add_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AuthorizeError>;
    
    /// 移除权限
    async fn remove_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AuthorizeError>;
    
    /// 检查权限
    async fn check_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthorizeError>;
}

/// 权限
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission {
    pub aic: String,
    pub resource: String,
    pub action: String,
}

/// 消息队列鉴权请求（对应 RabbitMQ HTTP 认证后端的表单字段）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqAuthRequest {
    /// 用户名（AIC）
    pub username: String,

    /// 密码（参考实现不校验）
    pub password: Option<String>,

    /// vhost
    pub vhost: String,

    /// 资源类型：`queue` / `exchange` / `topic`
    pub resource: Option<String>,

    /// 资源名称
    pub name: Option<String>,

    /// 权限：`configure` / `write` / `read`
    pub permission: Option<String>,

    /// 路由键（仅 topic 场景）
    pub routing_key: Option<String>,
}

impl MqAuthRequest {
    /// 构造 `/auth/user` 请求
    pub fn user(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: None,
            vhost: String::new(),
            resource: None,
            name: None,
            permission: None,
            routing_key: None,
        }
    }

    /// 构造 `/auth/vhost` 请求
    pub fn vhost(username: impl Into<String>, vhost: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: None,
            vhost: vhost.into(),
            resource: None,
            name: None,
            permission: None,
            routing_key: None,
        }
    }

    /// 构造 `/auth/resource` 请求
    pub fn resource(
        username: impl Into<String>,
        vhost: impl Into<String>,
        resource: impl Into<String>,
        name: impl Into<String>,
        permission: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: None,
            vhost: vhost.into(),
            resource: Some(resource.into()),
            name: Some(name.into()),
            permission: Some(permission.into()),
            routing_key: None,
        }
    }

    /// 构造 `/auth/topic` 请求
    pub fn topic(
        username: impl Into<String>,
        vhost: impl Into<String>,
        name: impl Into<String>,
        permission: impl Into<String>,
        routing_key: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: None,
            vhost: vhost.into(),
            resource: Some("topic".to_string()),
            name: Some(name.into()),
            permission: Some(permission.into()),
            routing_key: Some(routing_key.into()),
        }
    }
}

/// 鉴权决策
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthzDecision {
    /// 允许
    Allow,

    /// 拒绝（记录原因）
    Deny(String),

    /// 拒绝并要求关闭连接
    Refuse(String),
}

impl Default for AuthzDecision {
    /// 安全默认值：未判定时按拒绝处理（fail-closed）
    fn default() -> Self {
        AuthzDecision::Deny("undefined".to_string())
    }
}

impl AuthzDecision {
    /// 是否为允许
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthzDecision::Allow)
    }

    /// 拒绝原因
    pub fn reason(&self) -> Option<&str> {
        match self {
            AuthzDecision::Allow => None,
            AuthzDecision::Deny(reason) | AuthzDecision::Refuse(reason) => Some(reason),
        }
    }

    /// 转换为参考实现返回的纯文本响应
    pub fn as_response(&self) -> &'static str {
        if self.is_allowed() {
            "allow"
        } else {
            "deny"
        }
    }
}

/// 内存 ACL 管理器
///
/// 同时维护两类信息：
/// 1. 通用 `(aic, resource, action)` 权限三元组；
/// 2. 群组成员关系（`leader_aic + group_id -> members`），对应参考实现的
///    Redis key `group_acl:{leader_aic}:{group_id}`。
#[derive(Debug, Default)]
pub struct MemoryAclManager {
    permissions: RwLock<HashSet<Permission>>,
    groups: RwLock<HashMap<(String, String), HashSet<String>>>,
}

impl MemoryAclManager {
    /// 创建空管理器
    pub fn new() -> Self {
        Self {
            permissions: RwLock::new(HashSet::new()),
            groups: RwLock::new(HashMap::new()),
        }
    }

    /// 新增群组成员（等价于参考实现的 `PUT /groups/{leader}/{group}/members/{member}`）
    pub async fn add_member(
        &self,
        leader_aic: &str,
        group_id: &str,
        member_aic: &str,
    ) -> Result<(), AuthorizeError> {
        validate_aic(leader_aic)?;
        validate_aic(member_aic)?;
        crate::group::validate_group_id(group_id)?;

        self.groups
            .write()
            .await
            .entry((leader_aic.to_string(), group_id.to_string()))
            .or_default()
            .insert(member_aic.to_string());
        Ok(())
    }

    /// 移除群组成员
    pub async fn remove_member(
        &self,
        leader_aic: &str,
        group_id: &str,
        member_aic: &str,
    ) -> Result<(), AuthorizeError> {
        let mut groups = self.groups.write().await;
        match groups.get_mut(&(leader_aic.to_string(), group_id.to_string())) {
            Some(members) => {
                members.remove(member_aic);
                Ok(())
            }
            None => Err(AuthorizeError::GroupError(format!(
                "群组不存在: {}/{}",
                leader_aic, group_id
            ))),
        }
    }

    /// 删除整个群组 ACL
    pub async fn remove_group(
        &self,
        leader_aic: &str,
        group_id: &str,
    ) -> Result<bool, AuthorizeError> {
        Ok(self
            .groups
            .write()
            .await
            .remove(&(leader_aic.to_string(), group_id.to_string()))
            .is_some())
    }

    /// 是否为群组成员
    pub async fn is_member(&self, leader_aic: &str, group_id: &str, aic: &str) -> bool {
        self.groups
            .read()
            .await
            .get(&(leader_aic.to_string(), group_id.to_string()))
            .map(|members| members.contains(aic))
            .unwrap_or(false)
    }

    /// 列出群组成员
    pub async fn members(&self, leader_aic: &str, group_id: &str) -> Vec<String> {
        let mut members: Vec<String> = self
            .groups
            .read()
            .await
            .get(&(leader_aic.to_string(), group_id.to_string()))
            .map(|members| members.iter().cloned().collect())
            .unwrap_or_default();
        members.sort();
        members
    }

    /// 列出某个 Leader 的全部群组
    pub async fn groups_of_leader(&self, leader_aic: &str) -> Vec<String> {
        let mut groups: Vec<String> = self
            .groups
            .read()
            .await
            .keys()
            .filter(|(leader, _)| leader == leader_aic)
            .map(|(_, group)| group.clone())
            .collect();
        groups.sort();
        groups
    }

    /// 当前群组数量
    pub async fn group_count(&self) -> usize {
        self.groups.read().await.len()
    }
}

#[async_trait]
impl AclManager for MemoryAclManager {
    async fn add_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AuthorizeError> {
        self.permissions.write().await.insert(Permission {
            aic: aic.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
        });
        Ok(())
    }

    async fn remove_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AuthorizeError> {
        self.permissions.write().await.remove(&Permission {
            aic: aic.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
        });
        Ok(())
    }

    async fn check_permission(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthorizeError> {
        Ok(self.permissions.read().await.contains(&Permission {
            aic: aic.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
        }))
    }
}

/// 依据参考实现规则给出鉴权结论
pub async fn authorize_mq_request(
    request: &MqAuthRequest,
    acl: &MemoryAclManager,
) -> AuthzDecision {
    // 用户名必须是合法 AIC
    if validate_aic_format(&request.username).is_err() {
        return AuthzDecision::Deny("invalid-aic".to_string());
    }

    // /auth/user：仅校验用户名
    if request.resource.is_none() && request.vhost.is_empty() {
        return AuthzDecision::Allow;
    }

    // 所有后续判定的前提：vhost 必须是共享 vhost
    if request.vhost != SHARED_VHOST {
        return AuthzDecision::Deny("unsupported-vhost".to_string());
    }

    let Some(resource) = request.resource.as_deref() else {
        // /auth/vhost
        return AuthzDecision::Allow;
    };

    let permission = match request.permission.as_deref() {
        Some(permission @ ("configure" | "write" | "read")) => permission,
        _ => return AuthzDecision::Deny("unsupported-permission".to_string()),
    };

    let name = request.name.as_deref().unwrap_or_default();

    match resource {
        "exchange" => authorize_exchange(name, permission, &request.username, acl).await,
        "queue" => authorize_queue(name, permission, &request.username, acl).await,
        "topic" => authorize_topic(name, permission, request.routing_key.as_deref(), &request.username),
        _ => AuthzDecision::Deny("unsupported-resource".to_string()),
    }
}

/// 交换机授权
async fn authorize_exchange(
    name: &str,
    permission: &str,
    username: &str,
    group_acl: &MemoryAclManager,
) -> AuthzDecision {
    if name == DEFAULT_EXCHANGE {
        return AuthzDecision::Deny("blocked-default-exchange".to_string());
    }
    if name == INBOX_TOPIC_EXCHANGE {
        return AuthzDecision::Allow;
    }

    let Some((leader, group)) = parse_group_exchange(name) else {
        return AuthzDecision::Deny("invalid-group-exchange".to_string());
    };

    if permission == "configure" {
        return if leader == username {
            AuthzDecision::Allow
        } else {
            AuthzDecision::Deny("configure-not-leader".to_string())
        };
    }

    if group_acl.is_member(&leader, &group, username).await {
        AuthzDecision::Allow
    } else {
        AuthzDecision::Deny("not-group-member".to_string())
    }
}

/// 队列授权
async fn authorize_queue(
    name: &str,
    permission: &str,
    username: &str,
    group_acl: &MemoryAclManager,
) -> AuthzDecision {
    // 收件箱队列仅所有者可访问
    if let Some(owner) = name.strip_prefix(INBOX_QUEUE_PREFIX) {
        return if owner == username {
            AuthzDecision::Allow
        } else {
            AuthzDecision::Deny("foreign-inbox".to_string())
        };
    }

    let Some((leader, group, member)) = parse_group_queue(name) else {
        return AuthzDecision::Deny("invalid-group-queue".to_string());
    };

    let is_member = group_acl.is_member(&leader, &group, &member).await;
    let is_leader = leader == username;

    match permission {
        "configure" | "write" => {
            if is_leader || (username == member && is_member) {
                AuthzDecision::Allow
            } else if username == member {
                AuthzDecision::Deny("not-active-queue-member".to_string())
            } else {
                AuthzDecision::Deny("not-member".to_string())
            }
        }
        _ => {
            if username == member && is_member {
                AuthzDecision::Allow
            } else if username == member {
                AuthzDecision::Deny("read-not-active-member".to_string())
            } else {
                AuthzDecision::Deny("read-not-member".to_string())
            }
        }
    }
}

/// 主题授权
fn authorize_topic(
    name: &str,
    permission: &str,
    routing_key: Option<&str>,
    username: &str,
) -> AuthzDecision {
    if name != INBOX_TOPIC_EXCHANGE {
        return AuthzDecision::Deny("unsupported-exchange".to_string());
    }

    let Some(routing_key) = routing_key else {
        return AuthzDecision::Deny("invalid-routing-key".to_string());
    };

    let expected = format!("{}{}", INBOX_QUEUE_PREFIX, username);

    match permission {
        "read" => {
            if routing_key == expected {
                AuthzDecision::Allow
            } else {
                AuthzDecision::Deny("foreign-inbox-routing-key".to_string())
            }
        }
        "write" => {
            // 写允许路由到任意合法 AIC 的收件箱
            match routing_key.strip_prefix(INBOX_QUEUE_PREFIX) {
                Some(target) if validate_aic_format(target).is_ok() => AuthzDecision::Allow,
                _ => AuthzDecision::Deny("invalid-routing-key".to_string()),
            }
        }
        _ => AuthzDecision::Deny("unsupported-scope".to_string()),
    }
}

/// 解析 `group_{leader}_{group}`
fn parse_group_exchange(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix(GROUP_RESOURCE_PREFIX)?;
    let (leader, group) = rest.rsplit_once('_')?;
    if leader.is_empty() || group.is_empty() {
        return None;
    }
    Some((leader.to_string(), group.to_string()))
}

/// 解析 `group_{leader}_{group}_{member}`
fn parse_group_queue(name: &str) -> Option<(String, String, String)> {
    let rest = name.strip_prefix(GROUP_RESOURCE_PREFIX)?;
    let (prefix, member) = rest.rsplit_once('_')?;
    let (leader, group) = prefix.rsplit_once('_')?;
    if leader.is_empty() || group.is_empty() || member.is_empty() {
        return None;
    }
    Some((leader.to_string(), group.to_string(), member.to_string()))
}

/// 校验 AIC 合法性
pub(crate) fn validate_aic(aic: &str) -> Result<(), AuthorizeError> {
    validate_aic_format(aic)
        .map(|_| ())
        .map_err(|e| AuthorizeError::InvalidCredential(format!("非法 AIC: {}", e)))
}
