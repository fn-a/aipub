//! Authorize 服务核心实现
//!
//! 同时覆盖两类鉴权面：
//!
//! - 消息队列认证后端（`/auth/user|vhost|resource|topic`），供 RabbitMQ 调用；
//! - 群组 ACL 管理（`PUT|DELETE /groups/{leader}/{group}/members/{member}`），
//!   由 Leader 以 mTLS 客户端证书身份调用。

use std::sync::Arc;
use async_trait::async_trait;
use crate::{AuthorizeConfig, AuthorizeError, GroupManager};

/// Authorize 服务 trait
#[async_trait]
pub trait AuthorizeService: Send + Sync {
    /// 验证消息队列连接
    async fn authenticate_mq(
        &self,
        username: &str,
        password: &str,
    ) -> Result<bool, AuthorizeError>;
    
    /// 检查 ACL 权限
    async fn check_acl(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthorizeError>;
    
    /// 创建群组
    async fn create_group(&self, leader_aic: &str) -> Result<String, AuthorizeError>;
    
    /// 邀请智能体加入群组
    async fn invite_to_group(
        &self,
        group_id: &str,
        partner_aic: &str,
    ) -> Result<(), AuthorizeError>;
}

/// Authorize 服务实现
pub struct AuthorizeServiceImpl {
    config: AuthorizeConfig,
    acl: Arc<crate::MemoryAclManager>,
    groups: crate::MemoryGroupManager,
}

impl AuthorizeServiceImpl {
    /// 创建服务实例
    pub fn new(config: AuthorizeConfig) -> Self {
        let acl = Arc::new(crate::MemoryAclManager::new());
        let groups = crate::MemoryGroupManager::new(acl.clone());
        Self { config, acl, groups }
    }

    /// 当前配置
    pub fn config(&self) -> &AuthorizeConfig {
        &self.config
    }

    /// ACL 存储
    pub fn acl(&self) -> Arc<crate::MemoryAclManager> {
        self.acl.clone()
    }

    /// 群组管理器
    pub fn groups(&self) -> &crate::MemoryGroupManager {
        &self.groups
    }

    /// 处理消息队列鉴权请求（对应 `POST /auth/*`）
    pub async fn authorize_mq(
        &self,
        request: &crate::MqAuthRequest,
    ) -> crate::AuthzDecision {
        crate::authorize_mq_request(request, &self.acl).await
    }

    /// Leader 以 mTLS 身份管理群组成员
    ///
    /// `caller_aic` 为客户端证书 CN，必须与路径中的 `leader_aic` 一致。
    pub async fn add_group_member(
        &self,
        caller_aic: &str,
        leader_aic: &str,
        group_id: &str,
        member_aic: &str,
    ) -> Result<(), AuthorizeError> {
        self.ensure_leader_caller(caller_aic, leader_aic)?;
        self.acl.add_member(leader_aic, group_id, member_aic).await
    }

    /// 移除群组成员
    pub async fn remove_group_member(
        &self,
        caller_aic: &str,
        leader_aic: &str,
        group_id: &str,
        member_aic: &str,
    ) -> Result<(), AuthorizeError> {
        self.ensure_leader_caller(caller_aic, leader_aic)?;
        self.groups.force_remove(group_id, member_aic).await
    }

    /// 删除整个群组 ACL
    pub async fn delete_group_acl(
        &self,
        caller_aic: &str,
        leader_aic: &str,
        group_id: &str,
    ) -> Result<bool, AuthorizeError> {
        self.ensure_leader_caller(caller_aic, leader_aic)?;
        self.acl.remove_group(leader_aic, group_id).await
    }

    /// 校验调用方证书 CN 与路径中的 Leader AIC 一致
    fn ensure_leader_caller(
        &self,
        caller_aic: &str,
        leader_aic: &str,
    ) -> Result<(), AuthorizeError> {
        if caller_aic.trim().is_empty() {
            return Err(AuthorizeError::AuthenticationError(
                "CLIENT_CERTIFICATE_REQUIRED".to_string(),
            ));
        }
        if caller_aic != leader_aic {
            return Err(AuthorizeError::AuthorizationError(
                "LEADER_AIC_MISMATCH".to_string(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl AuthorizeService for AuthorizeServiceImpl {
    async fn authenticate_mq(
        &self,
        username: &str,
        _password: &str,
    ) -> Result<bool, AuthorizeError> {
        // 与参考实现一致：仅校验用户名是否为合法 AIC，不校验密码
        Ok(crate::acl::validate_aic(username).is_ok())
    }
    
    async fn check_acl(
        &self,
        aic: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthorizeError> {
        use crate::AclManager;

        // 优先匹配显式权限三元组，其次回落到群组成员关系
        if self.acl.check_permission(aic, resource, action).await? {
            return Ok(true);
        }

        if let Some((leader, group)) = resource
            .strip_prefix(crate::GROUP_RESOURCE_PREFIX)
            .and_then(|rest| rest.split_once('_'))
        {
            return Ok(self.acl.is_member(leader, group, aic).await);
        }

        Ok(false)
    }
    
    async fn create_group(&self, leader_aic: &str) -> Result<String, AuthorizeError> {
        let group = self.groups.create(leader_aic).await?;
        Ok(group.id)
    }
    
    async fn invite_to_group(
        &self,
        group_id: &str,
        partner_aic: &str,
    ) -> Result<(), AuthorizeError> {
        self.groups.invite(group_id, partner_aic).await
    }
}
