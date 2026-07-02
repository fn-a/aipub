//! ADP (Agent Discovery Protocol) 协议测试
//!
//! 测试智能体发现协议的请求、响应、过滤条件等功能

use aipub::protocols::adp::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试查询类型
    #[test]
    fn test_query_type() {
        let qt = QueryType::Explicit;
        let json = serde_json::to_string(&qt).unwrap();
        assert_eq!(json, "\"explicit\"");

        let deserialized: QueryType = serde_json::from_str(&json).unwrap();
        assert_eq!(qt, deserialized);
    }

    /// 测试过滤运算符
    #[test]
    fn test_filter_operator() {
        let op = FilterOperator::Contains;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, "\"contains\"");

        let op = FilterOperator::Eq;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, "\"eq\"");
    }
}

/// Discovery Request 测试
#[cfg(test)]
mod request_tests {
    use super::*;

    #[test]
    fn test_discovery_request() {
        let request = DiscoveryRequest::new()
            .with_query("我需要一个北京美食推荐的智能体")
            .with_limit(10);

        assert_eq!(request.query, Some("我需要一个北京美食推荐的智能体".to_string()));
        assert_eq!(request.limit, Some(10));
        assert_eq!(request.query_type, Some("explicit".to_string()));
    }

    #[test]
    fn test_discovery_request_with_filter() {
        let request = DiscoveryRequest::new()
            .with_query("测试查询")
            .with_limit(5)
            .with_filter(
                DiscoveryFilter::and(vec![
                    FilterCondition::eq("active", serde_json::json!(true)),
                    FilterCondition::contains("name", "北京"),
                ])
            );

        assert!(request.filter.is_some());
        let filter = request.filter.unwrap();
        assert_eq!(filter.logic, Some("and".to_string()));
        assert!(filter.conditions.is_some());
        assert_eq!(filter.conditions.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_discovery_request_effective_values() {
        let request = DiscoveryRequest::new()
            .with_query("测试")
            .with_limit(10);

        assert_eq!(request.get_effective_depth_limit(), 3); // 默认值
        assert_eq!(request.get_effective_fanout_limit(), 10); // 默认值
        assert_eq!(request.get_effective_each_timeout_ms(), 5000); // 默认值
        assert_eq!(request.get_effective_total_timeout_ms(), 30000); // 默认值
    }

    #[test]
    fn test_discovery_request_serialization() {
        let request = DiscoveryRequest::new()
            .with_query("测试查询")
            .with_limit(10);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"测试查询\""));
        assert!(json.contains("\"limit\":10"));

        // 反序列化
        let deserialized: DiscoveryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.query, deserialized.query);
        assert_eq!(request.limit, deserialized.limit);
    }
}

/// Discovery Response 测试
#[cfg(test)]
mod response_tests {
    use super::*;

    #[test]
    fn test_discovery_response_success() {
        let result = DiscoveryResult::new();
        let response = DiscoveryResponse::success(result);

        assert!(response.is_success());
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_discovery_response_failure() {
        let response = DiscoveryResponse::failure(40001, "MissingQuery");

        assert!(!response.is_success());
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, 40001);
        assert_eq!(error.message, "MissingQuery");
    }
}

/// Discovery Filter 测试
#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_condition() {
        let condition = FilterCondition::eq("active", serde_json::json!(true));
        assert_eq!(condition.field, "active");
        assert_eq!(condition.op, FilterOperator::Eq);
        assert_eq!(condition.value, Some(serde_json::json!(true)));
    }

    #[test]
    fn test_filter_condition_helpers() {
        let eq_cond = FilterCondition::eq("field", serde_json::json!(42));
        assert_eq!(eq_cond.op, FilterOperator::Eq);

        let contains_cond = FilterCondition::contains("name", "test");
        assert_eq!(contains_cond.op, FilterOperator::Contains);

        let exists_cond = FilterCondition::exists("field", true);
        assert_eq!(exists_cond.op, FilterOperator::Exists);

        let in_cond = FilterCondition::in_values("status", vec![
            serde_json::json!("active"),
            serde_json::json!("pending"),
        ]);
        assert_eq!(in_cond.op, FilterOperator::In);
    }

    #[test]
    fn test_discovery_filter() {
        let filter = DiscoveryFilter::and(vec![
            FilterCondition::eq("active", serde_json::json!(true)),
            FilterCondition::contains("name", "北京"),
        ]);

        assert_eq!(filter.logic, Some("and".to_string()));
        assert!(filter.conditions.is_some());
        assert_eq!(filter.conditions.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_discovery_filter_or() {
        let filter = DiscoveryFilter::or(vec![
            FilterCondition::eq("type", serde_json::json!("food")),
            FilterCondition::eq("type", serde_json::json!("travel")),
        ]);

        assert_eq!(filter.logic, Some("or".to_string()));
    }

    #[test]
    fn test_discovery_filter_not() {
        let filter = DiscoveryFilter::not(
            FilterCondition::eq("deleted", serde_json::json!(true))
        );

        assert_eq!(filter.logic, Some("not".to_string()));
    }

    #[test]
    fn test_discovery_filter_builder() {
        let filter = DiscoveryFilter::new()
            .add_condition(FilterCondition::eq("active", serde_json::json!(true)))
            .add_condition(FilterCondition::contains("name", "test"))
            .with_logic("and");

        assert_eq!(filter.logic, Some("and".to_string()));
        assert!(filter.conditions.is_some());
        assert_eq!(filter.conditions.as_ref().unwrap().len(), 2);
    }
}

/// Discovery Result 测试
#[cfg(test)]
mod result_tests {
    use super::*;

    #[test]
    fn test_discovery_result() {
        let result = DiscoveryResult::new();
        assert!(result.acs_map.is_empty());
        assert!(result.agents.is_empty());
        assert!(result.routes.is_none());
    }

    #[test]
    fn test_discovery_result_with_data() {
        let mut acs_map = std::collections::HashMap::new();
        acs_map.insert(
            "aic-001".to_string(),
            serde_json::json!({"name": "Agent 1"}),
        );

        let result = DiscoveryResult {
            acs_map,
            agents: vec![DiscoveryAgentGroup {
                group: "default".to_string(),
                agent_skills: vec![DiscoveryAgentSkill {
                    aic: "aic-001".to_string(),
                    skill_id: "skill-001".to_string(),
                    ranking: 1,
                    memo: None,
                }],
            }],
            routes: None,
        };

        assert_eq!(result.acs_map.len(), 1);
        assert_eq!(result.agents.len(), 1);
        assert_eq!(result.agents[0].agent_skills.len(), 1);
    }
}
