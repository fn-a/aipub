//! Authorize ACL 与消息队列鉴权测试
//!
//! 测试用户/vhost/资源/主题四类判定的规则与拒绝原因

use aipub::authorize::*;

#[cfg(test)]
mod tests {
    use super::*;

    const LEADER: &str = "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T";
    const MEMBER: &str = "1.2.156.3088.1.0001.00001.ABC123.000002.0H9T";
    const OTHER: &str = "1.2.156.3088.1.0001.00001.ABC123.000003.0H9T";
    const GROUP: &str = "group-1";

    async fn manager_with_member() -> MemoryAclManager {
        let manager = MemoryAclManager::new();
        manager.add_member(LEADER, GROUP, MEMBER).await.unwrap();
        manager
    }

    #[tokio::test]
    async fn test_user_and_vhost_auth() {
        let acl = MemoryAclManager::new();
        assert!(authorize_mq_request(&MqAuthRequest::user(LEADER), &acl)
            .await
            .is_allowed());
        assert!(!authorize_mq_request(&MqAuthRequest::user("not-an-aic"), &acl)
            .await
            .is_allowed());

        assert!(
            authorize_mq_request(&MqAuthRequest::vhost(LEADER, SHARED_VHOST), &acl)
                .await
                .is_allowed()
        );
        assert!(
            !authorize_mq_request(&MqAuthRequest::vhost(LEADER, "/"), &acl)
                .await
                .is_allowed()
        );
    }

    #[tokio::test]
    async fn test_inbox_queue_owner_only() {
        let acl = MemoryAclManager::new();
        let inbox = format!("inbox_{}", MEMBER);

        assert!(authorize_mq_request(
            &MqAuthRequest::resource(MEMBER, SHARED_VHOST, "queue", inbox.clone(), "read"),
            &acl
        )
        .await
        .is_allowed());

        let decision = authorize_mq_request(
            &MqAuthRequest::resource(OTHER, SHARED_VHOST, "queue", inbox, "read"),
            &acl,
        )
        .await;
        assert_eq!(decision.reason(), Some("foreign-inbox"));
    }

    #[tokio::test]
    async fn test_group_exchange_and_queue_rules() {
        let acl = manager_with_member().await;
        let exchange = format!("group_{}_{}", LEADER, GROUP);
        let queue = format!("group_{}_{}_{}", LEADER, GROUP, MEMBER);

        // 仅 Leader 可 configure 交换机
        assert!(authorize_mq_request(
            &MqAuthRequest::resource(LEADER, SHARED_VHOST, "exchange", exchange.clone(), "configure"),
            &acl
        )
        .await
        .is_allowed());
        assert!(!authorize_mq_request(
            &MqAuthRequest::resource(MEMBER, SHARED_VHOST, "exchange", exchange.clone(), "configure"),
            &acl
        )
        .await
        .is_allowed());

        // 活跃成员可写交换机
        assert!(authorize_mq_request(
            &MqAuthRequest::resource(MEMBER, SHARED_VHOST, "exchange", exchange, "write"),
            &acl
        )
        .await
        .is_allowed());

        // 非成员不可访问成员队列
        let decision = authorize_mq_request(
            &MqAuthRequest::resource(OTHER, SHARED_VHOST, "queue", queue.clone(), "read"),
            &acl,
        )
        .await;
        assert_eq!(decision.reason(), Some("read-not-member"));

        // 成员可读自己的队列
        assert!(authorize_mq_request(
            &MqAuthRequest::resource(MEMBER, SHARED_VHOST, "queue", queue, "read"),
            &acl
        )
        .await
        .is_allowed());
    }

    #[tokio::test]
    async fn test_default_exchange_blocked() {
        let acl = MemoryAclManager::new();
        let decision = authorize_mq_request(
            &MqAuthRequest::resource(LEADER, SHARED_VHOST, "exchange", DEFAULT_EXCHANGE, "write"),
            &acl,
        )
        .await;
        assert_eq!(decision.reason(), Some("blocked-default-exchange"));
    }

    #[tokio::test]
    async fn test_topic_routing_rules() {
        let acl = MemoryAclManager::new();
        let allow = authorize_mq_request(
            &MqAuthRequest::topic(
                MEMBER,
                SHARED_VHOST,
                INBOX_TOPIC_EXCHANGE,
                "read",
                format!("inbox_{}", MEMBER),
            ),
            &acl,
        )
        .await;
        assert!(allow.is_allowed());

        let deny = authorize_mq_request(
            &MqAuthRequest::topic(
                MEMBER,
                SHARED_VHOST,
                INBOX_TOPIC_EXCHANGE,
                "read",
                format!("inbox_{}", OTHER),
            ),
            &acl,
        )
        .await;
        assert_eq!(deny.reason(), Some("foreign-inbox-routing-key"));

        // 写入任意合法 AIC 的收件箱
        assert!(authorize_mq_request(
            &MqAuthRequest::topic(
                LEADER,
                SHARED_VHOST,
                INBOX_TOPIC_EXCHANGE,
                "write",
                format!("inbox_{}", MEMBER),
            ),
            &acl
        )
        .await
        .is_allowed());
    }

    #[tokio::test]
    async fn test_group_membership_management() {
        let acl = manager_with_member().await;
        assert!(acl.is_member(LEADER, GROUP, MEMBER).await);
        assert_eq!(acl.members(LEADER, GROUP).await, vec![MEMBER.to_string()]);
        assert_eq!(acl.groups_of_leader(LEADER).await, vec![GROUP.to_string()]);

        acl.remove_member(LEADER, GROUP, MEMBER).await.unwrap();
        assert!(!acl.is_member(LEADER, GROUP, MEMBER).await);

        assert!(acl.remove_group(LEADER, GROUP).await.unwrap());
        assert!(!acl.remove_group(LEADER, GROUP).await.unwrap());
    }

    #[tokio::test]
    async fn test_permission_triples() {
        let acl = MemoryAclManager::new();
        acl.add_permission(LEADER, "topic:audit", "read").await.unwrap();
        assert!(acl.check_permission(LEADER, "topic:audit", "read").await.unwrap());
        assert!(!acl.check_permission(LEADER, "topic:audit", "write").await.unwrap());

        acl.remove_permission(LEADER, "topic:audit", "read").await.unwrap();
        assert!(!acl.check_permission(LEADER, "topic:audit", "read").await.unwrap());
    }
}
