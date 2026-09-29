//! ADP (Agent Discovery Protocol) 协议测试
//!
//! 测试智能体发现协议的请求、响应、过滤条件等功能

use aipub::protocols::adp::*;

#[cfg(test)]
mod tests {
    use serde_json::{to_string, from_str};
    use super::*;

    /// 测试查询类型
    #[test]
    fn test_query_type() {
        let qt = QueryType::Explicit;
        let json = to_string(&qt).unwrap();
        assert_eq!(json, "\"explicit\"");

        let deserialized: QueryType = from_str(&json).unwrap();
        assert_eq!(qt, deserialized);
    }

    /// 测试过滤运算符
    #[test]
    fn test_filter_operator() {
        let op = FilterOperator::Contains;
        let json = to_string(&op).unwrap();
        assert_eq!(json, "\"contains\"");

        let op = FilterOperator::Eq;
        let json = to_string(&op).unwrap();
        assert_eq!(json, "\"eq\"");
    }
}

/// Discovery Request 测试
#[cfg(test)]
mod request_tests {
    use serde_json::{json, to_string, from_str};
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
                    FilterCondition::eq("active", json!(true)),
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

        // 默认值对齐 ACPs-spec-ADP-v02.01 / acps-sdk 常量
        assert_eq!(request.get_effective_depth_limit(), 3); // 默认值
        assert_eq!(request.get_effective_fanout_limit(), 1); // 默认值
        assert_eq!(request.get_effective_each_timeout_ms(), 10_000); // 默认值
        assert_eq!(request.get_effective_total_timeout_ms(), 60_000); // 默认值
        // 剩余扇出额度缺省时等于有效扇出限制
        assert_eq!(request.get_effective_fanout_remaining(), 1);
    }

    #[test]
    fn test_discovery_request_serialization() {
        let request = DiscoveryRequest::new()
            .with_query("测试查询")
            .with_limit(10);

        let json = to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"测试查询\""));
        assert!(json.contains("\"limit\":10"));

        // 反序列化
        let deserialized: DiscoveryRequest = from_str(&json).unwrap();
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
    use serde_json::json;
    use super::*;

    #[test]
    fn test_filter_condition() {
        let condition = FilterCondition::eq("active", json!(true));
        assert_eq!(condition.field, "active");
        assert_eq!(condition.op, FilterOperator::Eq);
        assert_eq!(condition.value, Some(json!(true)));
    }

    #[test]
    fn test_filter_condition_helpers() {
        let eq_cond = FilterCondition::eq("field", json!(42));
        assert_eq!(eq_cond.op, FilterOperator::Eq);

        let contains_cond = FilterCondition::contains("name", "test");
        assert_eq!(contains_cond.op, FilterOperator::Contains);

        let exists_cond = FilterCondition::exists("field", true);
        assert_eq!(exists_cond.op, FilterOperator::Exists);

        let in_cond = FilterCondition::in_values("status", vec![
            json!("active"),
            json!("pending"),
        ]);
        assert_eq!(in_cond.op, FilterOperator::In);
    }

    #[test]
    fn test_discovery_filter() {
        let filter = DiscoveryFilter::and(vec![
            FilterCondition::eq("active", json!(true)),
            FilterCondition::contains("name", "北京"),
        ]);

        assert_eq!(filter.logic, Some("and".to_string()));
        assert!(filter.conditions.is_some());
        assert_eq!(filter.conditions.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_discovery_filter_or() {
        let filter = DiscoveryFilter::or(vec![
            FilterCondition::eq("type", json!("food")),
            FilterCondition::eq("type", json!("travel")),
        ]);

        assert_eq!(filter.logic, Some("or".to_string()));
    }

    #[test]
    fn test_discovery_filter_not() {
        let filter = DiscoveryFilter::not(
            FilterCondition::eq("deleted", json!(true))
        );

        assert_eq!(filter.logic, Some("not".to_string()));
    }

    #[test]
    fn test_discovery_filter_builder() {
        let filter = DiscoveryFilter::new()
            .add_condition(FilterCondition::eq("active", json!(true)))
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
    use serde_json::json;
    use std::collections::HashMap;
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
        let mut acs_map = HashMap::new();
        acs_map.insert(
            "aic-001".to_string(),
            json!({"name": "Agent 1"}),
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

/// DiscoveryFilter / FilterCondition 求值测试
#[cfg(test)]
mod filter_eval_tests {
    use serde_json::{json, Value};
    use aipub::protocols::adp::FilterOperator;
    use super::*;

    fn sample_acs() -> Value {
        json!({
            "aic": "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T",
            "active": true,
            "name": "北京旅游助手",
            "version": "1.0.0",
            "skills": [
                {
                    "id": "beijing-tour.recommend",
                    "name": "北京景点推荐",
                    "tags": ["旅游", "北京", "推荐"]
                },
                {
                    "id": "beijing-tour.food",
                    "name": "北京美食推荐",
                    "tags": ["美食", "北京"]
                }
            ],
            "provider": { "countryCode": "CN", "organization": "ACPs工作组" },
            "priority": 5
        })
    }

    #[test]
    fn test_eq_on_nested_and_top_level() {
        let acs = sample_acs();
        assert!(FilterCondition::eq("active", json!(true)).evaluate(&acs));
        assert!(FilterCondition::eq("provider.countryCode", json!("cn")).evaluate(&acs));
        assert!(!FilterCondition::eq("provider.countryCode", json!("US")).evaluate(&acs));
    }

    #[test]
    fn test_array_field_any_element_matches() {
        let acs = sample_acs();
        // 数组字段按元素匹配：skills.tags 中的任意标签命中即可
        let condition = FilterCondition::new(
            "skills.tags".to_string(),
            FilterOperator::Eq,
            Some(json!("美食")),
        );
        assert!(condition.evaluate(&acs));

        let condition = FilterCondition::new(
            "skills.tags".to_string(),
            FilterOperator::AnyOf,
            Some(json!(["美景", "美食"])),
        );
        assert!(condition.evaluate(&acs));

        let condition = FilterCondition::new(
            "skills.tags".to_string(),
            FilterOperator::AllOf,
            Some(json!(["北京", "美食"])),
        );
        assert!(condition.evaluate(&acs));

        let condition = FilterCondition::new(
            "skills.tags".to_string(),
            FilterOperator::NoneOf,
            Some(json!(["上海"])),
        );
        assert!(condition.evaluate(&acs));
    }

    #[test]
    fn test_string_operators_case_sensitivity() {
        let acs = sample_acs();
        assert!(FilterCondition::contains("name", "北京").evaluate(&acs));

        let insensitive = FilterCondition::new(
            "name".to_string(),
            FilterOperator::StartsWith,
            Some(json!("北京")),
        );
        assert!(insensitive.evaluate(&acs));

        let sensitive = FilterCondition::new(
            "name".to_string(),
            FilterOperator::StartsWithCs,
            Some(json!("北京")),
        );
        assert!(sensitive.evaluate(&acs));

        let sensitive_miss = FilterCondition::new(
            "provider.countryCode".to_string(),
            FilterOperator::EqCs,
            Some(json!("cn")),
        );
        assert!(!sensitive_miss.evaluate(&acs));
    }

    #[test]
    fn test_comparison_and_size_and_keys() {
        let acs = sample_acs();

        let gte = FilterCondition::new(
            "version".to_string(),
            FilterOperator::Gte,
            Some(json!("1.0.0")),
        );
        assert!(gte.evaluate(&acs));

        let size = FilterCondition::new(
            "skills".to_string(),
            FilterOperator::Size,
            Some(json!(2)),
        );
        assert!(size.evaluate(&acs));

        let size_gt = FilterCondition::new(
            "skills".to_string(),
            FilterOperator::SizeGt,
            Some(json!(1)),
        );
        assert!(size_gt.evaluate(&acs));

        let has_key = FilterCondition::new(
            "provider".to_string(),
            FilterOperator::HasAllKeys,
            Some(json!(["countryCode", "organization"])),
        );
        assert!(has_key.evaluate(&acs));

        let has_no_key = FilterCondition::new(
            "provider".to_string(),
            FilterOperator::HasNoKey,
            Some(json!("missing")),
        );
        assert!(has_no_key.evaluate(&acs));
    }

    #[test]
    fn test_exists_semantics() {
        let acs = sample_acs();
        assert!(FilterCondition::exists("active", true).evaluate(&acs));
        assert!(!FilterCondition::exists("missing", true).evaluate(&acs));
        assert!(FilterCondition::exists("missing", false).evaluate(&acs));
    }

    #[test]
    fn test_logic_combination() {
        let acs = sample_acs();

        let and_filter = DiscoveryFilter::and(vec![
            FilterCondition::eq("active", json!(true)),
            FilterCondition::contains("name", "北京"),
        ]);
        assert!(and_filter.evaluate(&acs));

        let and_miss = DiscoveryFilter::and(vec![
            FilterCondition::eq("active", json!(true)),
            FilterCondition::contains("name", "上海"),
        ]);
        assert!(!and_miss.evaluate(&acs));

        let or_filter = DiscoveryFilter::or(vec![
            FilterCondition::eq("active", json!(false)),
            FilterCondition::contains("name", "北京"),
        ]);
        assert!(or_filter.evaluate(&acs));

        let not_filter = DiscoveryFilter::not(FilterCondition::contains("name", "上海"));
        assert!(not_filter.evaluate(&acs));
    }

    #[test]
    fn test_nested_groups_and_depth() {
        let acs = sample_acs();

        let nested = DiscoveryFilter {
            conditions: Some(vec![FilterCondition::eq("active", json!(true))]),
            groups: Some(vec![DiscoveryFilter::or(vec![
                FilterCondition::contains("name", "上海"),
                FilterCondition::contains("name", "北京"),
            ])]),
            logic: Some("and".to_string()),
        };
        assert!(nested.evaluate(&acs));
        assert_eq!(nested.depth(), 2);

        let deeper = DiscoveryFilter {
            conditions: None,
            groups: Some(vec![nested.clone()]),
            logic: Some("and".to_string()),
        };
        assert_eq!(deeper.depth(), 3);
    }

    #[test]
    fn test_between_closed_interval() {
        let acs = sample_acs();

        // 闭区间：5 落在 [5, 10] 与 [1, 5] 内
        let between = FilterCondition::new(
            "priority".to_string(),
            FilterOperator::Between,
            Some(json!([5, 10])),
        );
        assert!(between.evaluate(&acs));

        let between = FilterCondition::new(
            "priority".to_string(),
            FilterOperator::Between,
            Some(json!([1, 5])),
        );
        assert!(between.evaluate(&acs));

        // 不在区间内
        let between = FilterCondition::new(
            "priority".to_string(),
            FilterOperator::Between,
            Some(json!([6, 10])),
        );
        assert!(!between.evaluate(&acs));
    }
}

/// ADP 请求校验与转发测试
#[cfg(test)]
mod validator_tests {
    use serde_json::json;
    use aipub::protocols::adp::{DiscoveryFilter, FilterCondition, FilterOperator};
    use super::*;

    #[test]
    fn test_explicit_requires_query() {
        let request = DiscoveryRequest {
            query_type: Some("explicit".to_string()),
            query: None,
            ..Default::default()
        };
        let error = request.validate().unwrap_err();
        assert_eq!(error.code, 40001);
    }

    #[test]
    fn test_invalid_limits_rejected() {
        let request = DiscoveryRequest::new()
            .with_query("查询")
            .with_query_type("explicit");
        let mut bad_depth = request.clone();
        bad_depth.forward_depth_limit = Some(9);
        assert_eq!(bad_depth.validate().unwrap_err().code, 40002);

        let mut bad_fanout = request.clone();
        bad_fanout.forward_fanout_limit = Some(9);
        assert_eq!(bad_fanout.validate().unwrap_err().code, 40005);

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_filter_depth_limit() {
        let deep = DiscoveryFilter {
            conditions: Some(vec![FilterCondition::eq("active", json!(true))]),
            groups: Some(vec![DiscoveryFilter {
                conditions: None,
                groups: Some(vec![DiscoveryFilter {
                    conditions: Some(vec![FilterCondition::eq("active", json!(true))]),
                    groups: Some(vec![DiscoveryFilter {
                        conditions: Some(vec![FilterCondition::eq("active", json!(true))]),
                        groups: None,
                        logic: None,
                    }]),
                    logic: None,
                }]),
                logic: None,
            }]),
            logic: None,
        };

        let request = DiscoveryRequest::new()
            .with_query("查询")
            .with_filter(deep);
        assert_eq!(request.validate().unwrap_err().code, 40004);
    }

    #[test]
    fn test_forward_chain_validation() {
        let request = DiscoveryRequest {
            forward_chain: Some(vec!["aic-a".to_string()]),
            ..Default::default()
        };

        // 环路
        let error = validate_forward_chain(&request, "aic-a", None).unwrap_err();
        assert_eq!(error.code, 50801);

        // 未超深
        assert!(validate_forward_chain(&request, "aic-b", Some("aic-a")).is_ok());

        // 发送方不匹配
        let error = validate_forward_chain(&request, "aic-b", Some("aic-c")).unwrap_err();
        assert_eq!(error.code, 50803);

        // 超深：链长 3 >= 默认深度 3
        let deep = DiscoveryRequest {
            forward_chain: Some(vec![
                "aic-a".to_string(),
                "aic-b".to_string(),
                "aic-c".to_string(),
            ]),
            ..Default::default()
        };
        let error = validate_forward_chain(&deep, "aic-d", None).unwrap_err();
        assert_eq!(error.code, 50802);
    }

    #[test]
    fn test_build_forwarded_request() {
        let request = DiscoveryRequest::new()
            .with_query("查询")
            .with_query_type("explicit")
            .with_limit(5);
        let forwarded = build_forwarded_request(&request, "aic-server", 4, 1500, None, None);

        assert_eq!(forwarded.forward_chain, Some(vec!["aic-server".to_string()]));
        assert_eq!(forwarded.forward_fanout_remaining, Some(4));
        assert_eq!(
            forwarded.forward_total_timeout_ms,
            Some(FORWARD_TOTAL_TIMEOUT_MS_DEFAULT - 1500)
        );
        assert_eq!(forwarded.forward_depth_limit, Some(FORWARD_DEPTH_LIMIT_DEFAULT));
        assert_eq!(
            forwarded.forward_fanout_limit,
            Some(FORWARD_FANOUT_LIMIT_DEFAULT)
        );
    }

    #[test]
    fn test_allocate_fanout_budget_equal() {
        // 参考实现 doctest: allocate_fanout_budget(4, 3) == [0, 0, 1]
        assert_eq!(allocate_fanout_budget(4, 3, None).unwrap(), vec![0, 0, 1]);
        assert_eq!(allocate_fanout_budget(3, 3, None).unwrap(), vec![0, 0, 0]);
        assert_eq!(allocate_fanout_budget(10, 3, None).unwrap(), vec![2, 2, 3]);
        assert_eq!(allocate_fanout_budget(2, 3, None).unwrap_err().code, 50804);
    }

    #[test]
    fn test_allocate_fanout_budget_weighted() {
        let weights = [3.0, 1.0];
        let allocation = allocate_fanout_budget(6, 2, Some(&weights)).unwrap();
        assert_eq!(allocation.iter().sum::<usize>(), 4);
        assert!(allocation[0] >= allocation[1]);

        assert_eq!(
            allocate_fanout_budget(6, 2, Some(&[1.0]))
                .unwrap_err()
                .code,
            40005
        );
    }

    #[test]
    fn test_trusted_target_and_continue() {
        let mut request = DiscoveryRequest::default();
        assert!(validate_trusted_target(&request, "aic-x"));

        request.forward_trusted_servers = Some(vec!["aic-y".to_string()]);
        assert!(!validate_trusted_target(&request, "aic-x"));
        assert!(validate_trusted_target(&request, "aic-y"));

        assert!(should_continue_forwarding(&request));

        request.forward_each_timeout_ms = Some(70000);
        assert!(!should_continue_forwarding(&request));
    }

    #[test]
    fn test_filter_logic_name() {
        let mut request = DiscoveryRequest::default();
        assert_eq!(filter_logic_name(&request), "and");

        request.filter = Some(DiscoveryFilter {
            conditions: Some(vec![FilterCondition::new(
                "tags".to_string(),
                FilterOperator::AnyOf,
                Some(json!(["a"])),
            )]),
            groups: None,
            logic: Some("or".to_string()),
        });
        assert_eq!(filter_logic_name(&request), "or");
    }
}
