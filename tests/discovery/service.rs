//! Discovery 服务测试
//!
//! 测试四种查询模式、可发现门控、注册/更新/注销与变更流同步

use aipub::discovery::*;

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use aipub::discovery::matcher::KeywordMatcher;
    use aipub::discovery::storage::MemoryStorage;
    use aipub::discovery::Storage;
    use aipub::protocols::adp::{DiscoveryFilter, DiscoveryRequest, FilterOperator, FilterCondition};
    use aipub::protocols::dsp::ChangeEntry;
    use super::*;

    fn acs(aic: &str, name: &str, active: bool, skill_id: &str, tags: Vec<&str>) -> Value {
        json!({
            "aic": aic,
            "active": active,
            "name": name,
            "description": "提供本地生活服务",
            "endPoints": [{
                "url": format!("https://localhost:9021/rpc"),
                "transport": "JSONRPC",
                "security": [{"mtls": []}]
            }],
            "skills": [{
                "id": skill_id,
                "name": name,
                "description": "本地服务",
                "tags": tags,
                "version": "1.0.0"
            }]
        })
    }

    async fn build_service() -> DiscoveryServiceImpl {
        let storage = MemoryStorage::new();
        storage
            .store(acs("aic-food", "北京美食推荐", true, "beijing.food", vec!["美食", "北京"]))
            .await
            .unwrap();
        storage
            .store(acs("aic-hotel", "北京酒店预订", true, "beijing.hotel", vec!["酒店", "北京"]))
            .await
            .unwrap();
        storage
            .store(acs("aic-offline", "停用智能体", false, "offline", vec!["北京"]))
            .await
            .unwrap();
        storage
            .store(json!({
                "aic": "aic-noentry",
                "active": true,
                "name": "无入口智能体",
                "endPoints": [],
                "skills": []
            }))
            .await
            .unwrap();

        DiscoveryServiceImpl::new(
            DiscoveryConfig::default(),
            Box::new(storage),
            Box::new(KeywordMatcher::new()),
            None,
        )
    }

    #[tokio::test]
    async fn test_explicit_query_ranks_relevant_agent_first() {
        let service = build_service().await;
        let request = DiscoveryRequest::new()
            .with_query("我需要一个北京美食推荐的智能体")
            .with_limit(10);

        let response = service.discover(request).await.unwrap();
        let result = response.result.expect("应返回结果");

        assert_eq!(result.agents.len(), 1);
        let skills = &result.agents[0].agent_skills;
        assert!(!skills.is_empty());
        assert_eq!(skills[0].aic, "aic-food");
        assert_eq!(skills[0].skill_id, "beijing.food");
        assert_eq!(skills[0].ranking, 1);
        assert!(result.acs_map.contains_key("aic-food"));
        assert!(result.routes.is_some());
    }

    #[tokio::test]
    async fn test_inactive_and_entryless_agents_are_excluded() {
        let service = build_service().await;
        let request = DiscoveryRequest::new()
            .with_query("北京")
            .with_query_type("exploratory")
            .with_limit(10);

        let response = service.discover(request).await.unwrap();
        let result = response.result.unwrap();
        let aics: Vec<&str> = result
            .agents
            .iter()
            .flat_map(|group| group.agent_skills.iter())
            .map(|skill| skill.aic.as_str())
            .collect();

        assert!(!aics.contains(&"aic-offline"));
        assert!(!aics.contains(&"aic-noentry"));
        assert!(aics.contains(&"aic-food"));
    }

    #[tokio::test]
    async fn test_filtered_query_ignores_query_and_uses_filter() {
        let service = build_service().await;
        let mut request = DiscoveryRequest::new()
            .with_query("北京美食")
            .with_query_type("filtered")
            .with_limit(10);
        request.filter = Some(DiscoveryFilter::and(vec![FilterCondition::new(
            "skills.tags".to_string(),
            FilterOperator::AnyOf,
            Some(json!(["酒店"])),
        )]));

        let response = service.discover(request).await.unwrap();
        let result = response.result.unwrap();
        let skills = &result.agents[0].agent_skills;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].aic, "aic-hotel");
        assert_eq!(skills[0].memo.as_deref(), Some("Filtered query result"));
        assert_eq!(result.agents[0].group, "北京美食");
    }

    #[tokio::test]
    async fn test_filtered_without_filter_returns_error_body() {
        let service = build_service().await;
        let request = DiscoveryRequest::new().with_query_type("filtered");
        let response = service.discover(request).await.unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, 40004);
    }

    #[tokio::test]
    async fn test_explicit_without_query_returns_error_body() {
        let service = build_service().await;
        let request = DiscoveryRequest {
            query_type: Some("explicit".to_string()),
            query: None,
            ..Default::default()
        };
        let response = service.discover(request).await.unwrap();
        assert_eq!(response.error.unwrap().code, 40001);
    }

    #[tokio::test]
    async fn test_trending_returns_all_and_marks_memo() {
        let service = build_service().await;
        let request = DiscoveryRequest::new()
            .with_query_type("trending")
            .with_limit(10);

        let response = service.discover(request).await.unwrap();
        let result = response.result.unwrap();
        assert_eq!(result.agents[0].group, "trending");
        assert_eq!(result.agents[0].agent_skills.len(), 2);
        assert!(result.agents[0]
            .agent_skills
            .iter()
            .all(|skill| skill.memo.as_deref() == Some("trending")));
    }

    #[tokio::test]
    async fn test_explicit_falls_back_to_exploratory_when_weak() {
        let service = build_service().await;
        // 查询词完全无法命中时降级为探索性结果
        let request = DiscoveryRequest::new()
            .with_query("量子计算编译器")
            .with_limit(10);

        let response = service.discover(request).await.unwrap();
        let result = response.result.unwrap();
        assert!(!result.agents[0].agent_skills.is_empty());
        assert!(result.agents[0]
            .agent_skills
            .iter()
            .all(|skill| skill.memo.as_deref() == Some("exploratory")));
    }

    #[tokio::test]
    async fn test_register_update_unregister_agent() {
        let service = build_service().await;

        service
            .register_agent(acs("aic-new", "新智能体", true, "new.skill", vec!["新"]))
            .await
            .unwrap();
        assert!(service.get_agent("aic-new").await.unwrap().is_some());

        service
            .update_agent("aic-new", acs("aic-new", "新智能体v2", true, "new.skill", vec!["新"]))
            .await
            .unwrap();
        assert_eq!(
            service.get_agent("aic-new").await.unwrap().unwrap()["name"],
            "新智能体v2"
        );

        service.unregister_agent("aic-new").await.unwrap();
        assert!(service.get_agent("aic-new").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_apply_changes_from_registry() {
        let service = build_service().await;
        let entries = vec![
            ChangeEntry::upsert(1, "aic-sync", 1, acs("aic-sync", "同步智能体", true, "sync", vec!["同步"])),
            ChangeEntry::upsert(2, "aic-sync", 1, acs("aic-sync", "旧", true, "sync", vec![])),
        ];

        let applied = service.apply_changes(&entries).await.unwrap();
        assert_eq!(applied, 1);
        assert_eq!(
            service.get_agent("aic-sync").await.unwrap().unwrap()["name"],
            "同步智能体"
        );

        let stats = service.stats().await.unwrap();
        assert!(stats.total_agents >= 5);
    }
}
