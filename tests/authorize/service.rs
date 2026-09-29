//! Authorize 服务测试
//!
//! 测试 MQ 认证、群组 ACL 接口（调用方证书 CN 校验）与权限三元组

use aipub::authorize::*;

#[cfg(test)]
mod tests {
    use super::*;

    const LEADER: &str = "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T";
    const MEMBER: &str = "1.2.156.3088.1.0001.00001.ABC123.000002.0H9T";
    const OTHER: &str = "1.2.156.3088.1.0001.00001.ABC123.000003.0H9T";

    #[tokio::test]
    async fn test_authenticate_mq_ignores_password() {
        let service = AuthorizeServiceImpl::new(AuthorizeConfig::default());
        assert!(service.authenticate_mq(LEADER, "any-password").await.unwrap());
        assert!(!service.authenticate_mq("bad-aic", "any").await.unwrap());
    }

    #[tokio::test]
    async fn test_full_group_flow() {
        let service = AuthorizeServiceImpl::new(AuthorizeConfig::default());

        let group_id = service.create_group(LEADER).await.unwrap();
        assert!(group_id.starts_with("group-"));

        service
            .invite_to_group(&group_id, MEMBER)
            .await
            .unwrap();

        let groups = service.groups().list().await;
        assert_eq!(groups.len(), 1);

        // 邀请 -> 接受
        let invitation = service
            .groups()
            .create_invitation(&group_id, MEMBER)
            .await
            .unwrap();
        service
            .groups()
            .accept_invitation(&invitation.invitation_token, MEMBER)
            .await
            .unwrap();

        // 成员可写群组交换机
        let decision = service
            .authorize_mq(&MqAuthRequest::resource(
                MEMBER,
                SHARED_VHOST,
                "exchange",
                format!("group_{}_{}", LEADER, group_id),
                "write",
            ))
            .await;
        assert!(decision.is_allowed());

        // 非成员被拒
        let decision = service
            .authorize_mq(&MqAuthRequest::resource(
                OTHER,
                SHARED_VHOST,
                "exchange",
                format!("group_{}_{}", LEADER, group_id),
                "write",
            ))
            .await;
        assert_eq!(decision.reason(), Some("not-group-member"));
    }

    #[tokio::test]
    async fn test_group_acl_requires_matching_certificate_cn() {
        let service = AuthorizeServiceImpl::new(AuthorizeConfig::default());
        let group_id = service.create_group(LEADER).await.unwrap();

        // 证书 CN 缺失
        assert!(service
            .add_group_member("", LEADER, &group_id, MEMBER)
            .await
            .is_err());

        // 证书 CN 与路径 Leader 不一致
        assert!(service
            .add_group_member(MEMBER, LEADER, &group_id, MEMBER)
            .await
            .is_err());

        // 正常写入
        service
            .add_group_member(LEADER, LEADER, &group_id, MEMBER)
            .await
            .unwrap();
        assert!(service.acl().is_member(LEADER, &group_id, MEMBER).await);

        // 删除成员
        service
            .remove_group_member(LEADER, LEADER, &group_id, MEMBER)
            .await
            .unwrap();
        assert!(!service.acl().is_member(LEADER, &group_id, MEMBER).await);

        // 删除群组 ACL
        assert!(service
            .delete_group_acl(LEADER, LEADER, &group_id)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_check_acl_permission_triples() {
        let service = AuthorizeServiceImpl::new(AuthorizeConfig::default());
        service
            .acl()
            .add_permission(LEADER, "topic:audit", "read")
            .await
            .unwrap();

        assert!(service
            .check_acl(LEADER, "topic:audit", "read")
            .await
            .unwrap());
        assert!(!service
            .check_acl(LEADER, "topic:audit", "write")
            .await
            .unwrap());
        assert!(!service
            .check_acl(OTHER, "topic:other", "read")
            .await
            .unwrap());
    }
}
