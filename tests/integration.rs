//! 集成测试 - 测试多个协议模块与服务组件的协作
//!
//! 本文件同时作为各模块测试的入口，声明以下子模块：
//!
//! - `protocols`：AIC / ACS / AIP / ADP / ATR / DSP 协议测试
//! - `registry`：注册服务测试
//! - `discovery`：发现服务测试
//! - `certifier`：证书服务测试
//! - `authorize`：鉴权服务测试

mod authorize;
mod certifier;
mod discovery;
mod protocols;
mod registry;

/// 测试 AIC 和 ACS 的集成
#[cfg(test)]
mod aic_acs_integration {
    use std::collections::HashMap;
    use aipub::protocols::{aic::*, acs::*};

    #[test]
    fn test_aic_acs_workflow() {
        // 1. 验证 AIC
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
        let aic_info = validate_aic_format(aic).unwrap();
        
        // 2. 使用 AIC 创建 ACS
        let mut security_schemes = HashMap::new();
        security_schemes.insert(
            "mtls".to_string(),
            SecurityScheme::MutualTls(MutualTlsSecurityScheme {
                r#type: "mutualTLS".to_string(),
                description: None,
                x_ca_challenge_base_url: None,
            }),
        );

        let acs = AgentCapabilitySpec {
            aic: aic_info.normalized.clone(),
            active: true,
            last_modified_time: "2025-03-15T16:30:00+08:00".to_string(),
            protocol_version: "02.01".to_string(),
            name: "测试智能体".to_string(),
            description: "测试描述".to_string(),
            version: "1.0.0".to_string(),
            provider: AgentProvider::default(),
            security_schemes,
            capabilities: AgentCapabilities::default(),
            end_points: vec![],
            default_input_modes: vec![],
            default_output_modes: vec![],
            skills: vec![],
            icon_url: None,
            documentation_url: None,
            web_app_url: None,
            entity_user_id: None,
            entity_meta: None,
        };

        // 3. 验证 ACS
        assert!(acs.validate().is_ok());
        
        // 4. 序列化 ACS
        let json = acs.to_json().unwrap();
        assert!(json.contains(&aic_info.normalized));
    }
}

/// 测试 AIP 和 ADP 的集成
#[cfg(test)]
mod aip_adp_integration {
    use aipub::protocols::{aip::*, adp::*};

    #[test]
    fn test_aip_adp_workflow() {
        // 1. 创建发现请求
        let discovery_request = DiscoveryRequest::new()
            .with_query("我需要一个北京美食推荐的智能体")
            .with_limit(5);

        // 2. 模拟发现响应
        let response = DiscoveryResponse::success(DiscoveryResult::new());
        assert!(response.is_success());
        assert_eq!(discovery_request.limit, Some(5));
        assert_eq!(
            discovery_request.query.as_deref(),
            Some("我需要一个北京美食推荐的智能体")
        );

        // 3. 创建任务命令
        let task_command = TaskCommand::start(
            "cmd-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "leader-001".to_string(),
        );

        // 4. 创建任务结果
        let task_status = TaskStatus {
            state: TaskState::Working,
            state_changed_at: "2025-03-15T16:30:01+08:00".to_string(),
            data_items: None,
        };

        let task_result = TaskResult::new(
            "result-001".to_string(),
            "2025-03-15T16:30:01+08:00".to_string(),
            "partner-001".to_string(),
            "task-001".to_string(),
            task_status,
        );

        // 验证整个流程
        assert_eq!(task_command.command, TaskCommandType::Start);
        assert_eq!(task_result.status.state, TaskState::Working);
    }
}

/// 测试完整工作流
#[cfg(test)]
mod full_workflow {
    use std::collections::HashMap;
    use serde_json::json;
    use aipub::protocols::{aic::*, acs::*, aip::*, adp::*};

    #[test]
    fn test_complete_agent_workflow() {
        // 步骤 1: 创建和验证 AIC
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
        let aic_info = validate_aic_format(aic).unwrap();
        assert!(aic_info.is_ontology());

        // 步骤 2: 创建 ACS
        let mut security_schemes = HashMap::new();
        security_schemes.insert(
            "mtls".to_string(),
            SecurityScheme::MutualTls(MutualTlsSecurityScheme {
                r#type: "mutualTLS".to_string(),
                description: None,
                x_ca_challenge_base_url: None,
            }),
        );

        let acs = AgentCapabilitySpec {
            aic: aic.to_string(),
            active: true,
            last_modified_time: "2025-03-15T16:30:00+08:00".to_string(),
            protocol_version: "02.01".to_string(),
            name: "北京旅游助手".to_string(),
            description: "提供北京旅游规划服务".to_string(),
            version: "1.0.0".to_string(),
            provider: AgentProvider {
                country_code: Some("CN".to_string()),
                organization: Some("ACPs工作组".to_string()),
                ..Default::default()
            },
            security_schemes,
            capabilities: AgentCapabilities {
                streaming: true,
                notification: false,
                message_queue: vec![MqProtocolVersion::Mqtt50],
            },
            end_points: vec![],
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![AgentSkill {
                id: "beijing-tour.recommend".to_string(),
                name: "北京景点推荐".to_string(),
                description: "推荐北京旅游景点".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["旅游".to_string(), "北京".to_string()],
                examples: None,
                input_modes: None,
                output_modes: None,
            }],
            icon_url: None,
            documentation_url: None,
            web_app_url: None,
            entity_user_id: None,
            entity_meta: None,
        };

        assert!(acs.validate().is_ok());
        assert!(acs.supports_streaming());
        assert!(acs.has_skill("beijing-tour.recommend"));

        // 步骤 3: 发现智能体
        let discovery_request = DiscoveryRequest::new()
            .with_query("北京旅游推荐")
            .with_limit(10)
            .with_filter(
                DiscoveryFilter::and(vec![
                    FilterCondition::eq("active", json!(true)),
                    FilterCondition::contains("name", "北京"),
                ])
            );

        assert!(discovery_request.filter.is_some());

        // 步骤 4: 创建任务
        let task_command = TaskCommand::start(
            "cmd-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "leader-001".to_string(),
        );

        // 步骤 5: 返回结果
        let task_status = TaskStatus {
            state: TaskState::Completed,
            state_changed_at: "2025-03-15T16:35:00+08:00".to_string(),
            data_items: None,
        };

        assert_eq!(task_command.command, TaskCommandType::Start);
        assert_eq!(task_command.sender_role, "leader");
        assert_eq!(task_command.sender_id, "leader-001");

        let task_result = TaskResult::new(
            "result-001".to_string(),
            "2025-03-15T16:35:00+08:00".to_string(),
            "partner-001".to_string(),
            "task-001".to_string(),
            task_status,
        );

        assert!(task_result.is_completed());
    }
}
