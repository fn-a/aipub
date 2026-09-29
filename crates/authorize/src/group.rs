//! 群组管理
//!
//! 对齐参考实现的群组模式：
//!
//! - 群组由 Leader 创建，群组 ID 形如 `group-{sessionId}`（仅允许
//!   `[A-Za-z0-9-]`，长度 1~128）；
//! - Leader 先向鉴权服务写入成员的群组 ACL，再通过收件箱（`inbox.topic`）
//!   或直接 RPC 发送邀请；
//! - 成员加入后由 Leader 侧队列/交换机承载群组任务流量。

use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use uuid::Uuid;
use tokio::sync::RwLock;
use crate::{AuthorizeError, MemoryAclManager};

/// 默认邀请有效期（秒）
pub const DEFAULT_INVITATION_TIMEOUT_SECONDS: i64 = 300;

/// 允许的时钟偏移（秒）
pub const INVITATION_CLOCK_SKEW_SECONDS: i64 = 30;

/// 群组 ID 最大长度
pub const MAX_GROUP_ID_LENGTH: usize = 128;

/// 群组管理器 trait
#[async_trait]
pub trait GroupManager: Send + Sync {
    /// 创建群组
    async fn create(&self, leader_aic: &str) -> Result<Group, AuthorizeError>;
    
    /// 邀请成员
    async fn invite(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError>;
    
    /// 加入群组
    async fn join(&self, group_id: &str, partner_aic: &str) -> Result<Group, AuthorizeError>;
    
    /// 离开群组
    ///
    /// 参考实现中成员不能自行退出，只有 Leader 可以解散群组。
    async fn leave(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError>;
}

/// 群组
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: String,
    pub leader_aic: String,
    pub partners: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Group {
    /// 群组交换机名（fanout）
    pub fn exchange_name(&self) -> String {
        format!("group_{}_{}", self.leader_aic, self.id)
    }

    /// 成员队列名
    pub fn member_queue_name(&self, member_aic: &str) -> String {
        format!("group_{}_{}_{}", self.leader_aic, self.id, member_aic)
    }

    /// 是否包含指定成员
    pub fn contains(&self, aic: &str) -> bool {
        self.partners.iter().any(|partner| partner == aic)
    }
}

/// 群组邀请
#[derive(Debug, Clone, PartialEq)]
pub struct GroupInvitation {
    /// 群组 ID
    pub group_id: String,

    /// 邀请方（Leader）AIC
    pub leader_aic: String,

    /// 被邀请方 AIC
    pub partner_aic: String,

    /// 邀请令牌
    pub invitation_token: String,

    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

impl GroupInvitation {
    /// 在允许时钟偏移的前提下判断是否过期
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now > self.expires_at + Duration::seconds(INVITATION_CLOCK_SKEW_SECONDS)
    }
}

/// 规范化群组 ID
///
/// 依据参考实现 `aip_group_runtime.normalize_group_id`：非 `[A-Za-z0-9-]`
/// 的字符替换为 `-`，去除首尾 `-`，并截断到 128 字符。
pub fn normalize_group_id(raw: &str) -> String {
    let normalized: String = raw
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' { ch } else { '-' })
        .collect();
    let trimmed = normalized.trim_matches('-');
    let truncated: String = trimmed.chars().take(MAX_GROUP_ID_LENGTH).collect();
    if truncated.is_empty() {
        "group".to_string()
    } else {
        truncated
    }
}

/// 校验群组 ID
pub fn validate_group_id(group_id: &str) -> Result<(), AuthorizeError> {
    lazy_static::lazy_static! {
        static ref RE_GROUP_ID: Regex =
            Regex::new(r"^[A-Za-z0-9-]{1,128}$").expect("群组 ID 正则有效");
    }

    if RE_GROUP_ID.is_match(group_id) {
        Ok(())
    } else {
        Err(AuthorizeError::GroupError(format!(
            "非法群组 ID: '{}'（仅允许 [A-Za-z0-9-]，长度 1~128）",
            group_id
        )))
    }
}

/// 内存群组管理器
#[derive(Debug)]
pub struct MemoryGroupManager {
    acl: Arc<MemoryAclManager>,
    groups: RwLock<HashMap<String, Group>>,
    invitations: RwLock<HashMap<String, GroupInvitation>>,
    invitation_ttl_seconds: i64,
}

impl MemoryGroupManager {
    /// 创建群组管理器
    pub fn new(acl: Arc<MemoryAclManager>) -> Self {
        Self {
            acl,
            groups: RwLock::new(HashMap::new()),
            invitations: RwLock::new(HashMap::new()),
            invitation_ttl_seconds: DEFAULT_INVITATION_TIMEOUT_SECONDS,
        }
    }

    /// 自定义邀请有效期（秒）
    ///
    /// 传入 `0` 或负数表示邀请立即过期（仍受
    /// [`INVITATION_CLOCK_SKEW_SECONDS`] 时钟偏移约束），便于演练/验证过期分支。
    pub fn with_invitation_ttl_seconds(mut self, seconds: i64) -> Self {
        self.invitation_ttl_seconds = seconds;
        self
    }

    /// 读取群组
    pub async fn get(&self, group_id: &str) -> Option<Group> {
        self.groups.read().await.get(group_id).cloned()
    }

    /// 列出全部群组
    pub async fn list(&self) -> Vec<Group> {
        let mut groups: Vec<Group> = self.groups.read().await.values().cloned().collect();
        groups.sort_by(|a, b| a.id.cmp(&b.id));
        groups
    }

    /// 以 `group-{session_id}` 形式创建群组
    pub async fn create_for_session(
        &self,
        leader_aic: &str,
        session_id: &str,
    ) -> Result<Group, AuthorizeError> {
        self.create_with_id(leader_aic, &normalize_group_id(&format!("group-{}", session_id)))
            .await
    }

    /// 指定群组 ID 创建群组
    pub async fn create_with_id(
        &self,
        leader_aic: &str,
        group_id: &str,
    ) -> Result<Group, AuthorizeError> {
        crate::acl::validate_aic(leader_aic)?;
        validate_group_id(group_id)?;

        // Leader 自身也写入 ACL，便于其 configure 自己的交换机
        self.acl.add_member(leader_aic, group_id, leader_aic).await?;

        let group = Group {
            id: group_id.to_string(),
            leader_aic: leader_aic.to_string(),
            partners: vec![leader_aic.to_string()],
            created_at: Utc::now(),
        };

        self.groups
            .write()
            .await
            .insert(group_id.to_string(), group.clone());

        Ok(group)
    }

    /// 生成邀请
    pub async fn create_invitation(
        &self,
        group_id: &str,
        partner_aic: &str,
    ) -> Result<GroupInvitation, AuthorizeError> {
        crate::acl::validate_aic(partner_aic)?;

        let group = self
            .get(group_id)
            .await
            .ok_or_else(|| AuthorizeError::GroupError(format!("群组不存在: {}", group_id)))?;

        let invitation = GroupInvitation {
            group_id: group_id.to_string(),
            leader_aic: group.leader_aic.clone(),
            partner_aic: partner_aic.to_string(),
            invitation_token: generate_invitation_token(),
            expires_at: Utc::now() + Duration::seconds(self.invitation_ttl_seconds),
        };

        self.invitations
            .write()
            .await
            .insert(invitation.invitation_token.clone(), invitation.clone());

        Ok(invitation)
    }

    /// 接受邀请并加入群组（邀请令牌一次性使用）
    pub async fn accept_invitation(
        &self,
        invitation_token: &str,
        partner_aic: &str,
    ) -> Result<Group, AuthorizeError> {
        // 先校验，只有成功接受时才消费令牌
        let invitation = self
            .invitations
            .read()
            .await
            .get(invitation_token)
            .cloned()
            .ok_or_else(|| {
                AuthorizeError::GroupError(format!("邀请不存在: {}", invitation_token))
            })?;

        if invitation.partner_aic != partner_aic {
            return Err(AuthorizeError::AuthorizationError(
                "邀请与响应方不匹配".to_string(),
            ));
        }

        if invitation.is_expired(Utc::now()) {
            return Err(AuthorizeError::GroupError(format!(
                "邀请已过期: {}",
                invitation_token
            )));
        }

        self.invitations.write().await.remove(invitation_token);

        self.join(&invitation.group_id, partner_aic).await
    }

    /// 拒绝邀请
    pub async fn reject_invitation(
        &self,
        invitation_token: &str,
        partner_aic: &str,
    ) -> Result<(), AuthorizeError> {
        let mut invitations = self.invitations.write().await;
        let invitation = invitations
            .remove(invitation_token)
            .ok_or_else(|| {
                AuthorizeError::GroupError(format!("邀请不存在: {}", invitation_token))
            })?;

        if invitation.partner_aic != partner_aic {
            return Err(AuthorizeError::AuthorizationError(
                "邀请与响应方不匹配".to_string(),
            ));
        }
        Ok(())
    }

    /// 解散群组：清空成员并删除群组 ACL
    pub async fn disband(&self, group_id: &str) -> Result<Vec<String>, AuthorizeError> {
        let group = self
            .groups
            .write()
            .await
            .remove(group_id)
            .ok_or_else(|| AuthorizeError::GroupError(format!("群组不存在: {}", group_id)))?;

        self.acl.remove_group(&group.leader_aic, group_id).await?;
        Ok(group.partners)
    }

    /// 由 Leader 强制移除成员（成员自身无权限退出）
    pub async fn force_remove(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError> {
        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| AuthorizeError::GroupError(format!("群组不存在: {}", group_id)))?;

        if partner_aic == group.leader_aic {
            return Err(AuthorizeError::GroupError(
                "不能移除群组 Leader".to_string(),
            ));
        }

        group.partners.retain(|partner| partner != partner_aic);
        let leader = group.leader_aic.clone();
        drop(groups);

        self.acl.remove_member(&leader, group_id, partner_aic).await
    }

    /// 清理已过期的邀请
    pub async fn prune_invitations(&self) -> usize {
        let now = Utc::now();
        let mut invitations = self.invitations.write().await;
        let before = invitations.len();
        invitations.retain(|_, invitation| !invitation.is_expired(now));
        before - invitations.len()
    }
}

#[async_trait]
impl GroupManager for MemoryGroupManager {
    async fn create(&self, leader_aic: &str) -> Result<Group, AuthorizeError> {
        let group_id = normalize_group_id(&format!("group-{}", Uuid::new_v4()));
        self.create_with_id(leader_aic, &group_id).await
    }

    async fn invite(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError> {
        self.create_invitation(group_id, partner_aic).await.map(|_| ())
    }

    async fn join(&self, group_id: &str, partner_aic: &str) -> Result<Group, AuthorizeError> {
        crate::acl::validate_aic(partner_aic)?;

        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| AuthorizeError::GroupError(format!("群组不存在: {}", group_id)))?;

        if !group.partners.iter().any(|partner| partner == partner_aic) {
            group.partners.push(partner_aic.to_string());
        }
        let updated = group.clone();
        let leader = group.leader_aic.clone();
        drop(groups);

        // 成员加入后写入群组 ACL（参考实现中由 Leader 预先写入）
        self.acl.add_member(&leader, group_id, partner_aic).await?;
        Ok(updated)
    }

    async fn leave(&self, group_id: &str, partner_aic: &str) -> Result<(), AuthorizeError> {
        let group = self
            .get(group_id)
            .await
            .ok_or_else(|| AuthorizeError::GroupError(format!("群组不存在: {}", group_id)))?;

        // 成员不能自行退出；Leader 离开即解散群组
        if partner_aic != group.leader_aic {
            return Err(AuthorizeError::AuthorizationError(
                "成员不能自行退出群组，需由 Leader 移除".to_string(),
            ));
        }

        self.disband(group_id).await.map(|_| ())
    }
}

/// 生成邀请令牌（对应参考实现的 `secrets.token_urlsafe(32)`）
fn generate_invitation_token() -> String {
    let first = Uuid::new_v4().as_simple().to_string();
    let second = Uuid::new_v4().as_simple().to_string();
    format!("{}{}", first, second)
}
