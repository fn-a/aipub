//! Authorize 群组管理测试
//!
//! 测试群组创建、邀请/接受/拒绝、成员移除、解散与群组 ID 规范化

use aipub::authorize::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;

    const LEADER: &str = "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T";
    const MEMBER: &str = "1.2.156.3088.1.0001.00001.ABC123.000002.0H9T";
    const OTHER: &str = "1.2.156.3088.1.0001.00001.ABC123.000003.0H9T";

    fn build() -> (Arc<MemoryAclManager>, MemoryGroupManager) {
        let acl = Arc::new(MemoryAclManager::new());
        let manager = MemoryGroupManager::new(acl.clone());
        (acl, manager)
    }

    #[test]
    fn test_normalize_group_id() {
        assert_eq!(normalize_group_id("group-abc 123"), "group-abc-123");
        assert_eq!(normalize_group_id("---group---"), "group");
        assert_eq!(normalize_group_id("中文会话"), "group");
        assert_eq!(normalize_group_id(&"a".repeat(200)).len(), 128);
    }

    #[test]
    fn test_validate_group_id() {
        assert!(validate_group_id("group-1").is_ok());
        assert!(validate_group_id("").is_err());
        assert!(validate_group_id("group_1").is_err());
        assert!(validate_group_id(&"a".repeat(129)).is_err());
    }

    #[tokio::test]
    async fn test_create_invitation_accept_flow() {
        let (acl, manager) = build();

        let group = manager.create_for_session(LEADER, "session-1").await.unwrap();
        assert_eq!(group.id, "group-session-1");
        assert_eq!(group.exchange_name(), format!("group_{}_group-session-1", LEADER));
        assert_eq!(
            group.member_queue_name(MEMBER),
            format!("group_{}_group-session-1_{}", LEADER, MEMBER)
        );

        let invitation = manager.create_invitation(&group.id, MEMBER).await.unwrap();
        assert!(!invitation.invitation_token.is_empty());

        let joined = manager
            .accept_invitation(&invitation.invitation_token, MEMBER)
            .await
            .unwrap();
        assert!(joined.contains(MEMBER));
        assert!(acl.is_member(LEADER, &group.id, MEMBER).await);
        assert_eq!(acl.members(LEADER, &group.id).await.len(), 2);
    }

    #[tokio::test]
    async fn test_invitation_token_is_single_use_and_bound() {
        let (_acl, manager) = build();
        let group = manager.create_with_id(LEADER, "group-1").await.unwrap();
        let invitation = manager.create_invitation(&group.id, MEMBER).await.unwrap();

        // 响应方不匹配
        assert!(manager
            .accept_invitation(&invitation.invitation_token, OTHER)
            .await
            .is_err());

        // 正常接受
        manager
            .accept_invitation(&invitation.invitation_token, MEMBER)
            .await
            .unwrap();

        // 拒绝不存在的邀请
        assert!(manager
            .reject_invitation(&invitation.invitation_token, MEMBER)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_expired_invitation_rejected() {
        let (_acl, manager) = build();
        // 负数表示立即过期（仍受 30s 时钟偏移约束）
        let manager = manager.with_invitation_ttl_seconds(-60);
        let group = manager.create_with_id(LEADER, "group-1").await.unwrap();
        let invitation = manager.create_invitation(&group.id, MEMBER).await.unwrap();

        assert!(manager
            .accept_invitation(&invitation.invitation_token, MEMBER)
            .await
            .is_err());
        assert_eq!(manager.prune_invitations().await, 1);
    }

    #[tokio::test]
    async fn test_member_cannot_leave_but_leader_can_remove() {
        let (acl, manager) = build();
        let group = manager.create_with_id(LEADER, "group-1").await.unwrap();
        manager.join(&group.id, MEMBER).await.unwrap();

        assert!(manager.leave(&group.id, MEMBER).await.is_err());

        manager.force_remove(&group.id, MEMBER).await.unwrap();
        assert!(!manager.get(&group.id).await.unwrap().contains(MEMBER));
        assert!(!acl.is_member(LEADER, &group.id, MEMBER).await);

        // Leader 不可被移除
        assert!(manager.force_remove(&group.id, LEADER).await.is_err());
    }

    #[tokio::test]
    async fn test_disband_clears_acl() {
        let (acl, manager) = build();
        let group = manager.create_with_id(LEADER, "group-1").await.unwrap();
        manager.join(&group.id, MEMBER).await.unwrap();

        let members = manager.disband(&group.id).await.unwrap();
        assert!(members.contains(&MEMBER.to_string()));
        assert!(manager.get(&group.id).await.is_none());
        assert_eq!(acl.group_count().await, 0);
    }
}
