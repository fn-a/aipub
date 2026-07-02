//! ACS (Agent Capability Specification) 协议测试
//!
//! 测试智能体能力描述的数据模型、验证、序列化等功能

use aipub::protocols::acs::*;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用 ACS
    fn create_test_acs() -> AgentCapabilitySpec {
        let mut security_schemes = HashMap::new();
        security_schemes.insert(
            "mtls".to_string(),
            SecurityScheme::MutualTls(MutualTlsSecurityScheme {
                r#type: "mutualTLS".to_string(),
                description: Some("双向TLS认证".to_string()),
                x_ca_challenge_base_url: None,
            }),
        );

        AgentCapabilitySpec {
            aic: "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD".to_string(),
            active: true,
            last_modified_time: "2025-03-15T16:30:00+08:00".to_string(),
            protocol_version: "02.01".to_string(),
            name: "测试智能体".to_string(),
            description: "这是一个测试智能体".to_string(),
            version: "1.0.0".to_string(),
            provider: AgentProvider::default(),
            security_schemes,
            end_points: vec![],
            capabilities: AgentCapabilities {
                streaming: true,
                notification: false,
                message_queue: vec![MqProtocolVersion::Mqtt50],
            },
            default_input_modes: vec![],
            default_output_modes: vec![],
            skills: vec![AgentSkill {
                id: "test-skill".to_string(),
                name: "测试技能".to_string(),
                description: "这是一个测试技能".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["test".to_string(), "demo".to_string()],
                examples: None,
                input_modes: None,
                output_modes: None,
            }],
            icon_url: None,
            documentation_url: None,
            web_app_url: None,
            entity_user_id: None,
            entity_meta: None,
        }
    }

    /// 测试 ACS 验证
    #[test]
    fn test_acs_validate() {
        let acs = create_test_acs();
        assert!(acs.validate().is_ok());
    }

    /// 测试 ACS 序列化和反序列化
    #[test]
    fn test_acs_serialization() {
        let acs = create_test_acs();
        
        // 序列化为 JSON
        let json = acs.to_json().unwrap();
        assert!(json.contains("\"aic\""));
        assert!(json.contains("\"active\""));
        assert!(json.contains("\"name\""));
        
        // 反序列化
        let deserialized = AgentCapabilitySpec::from_json(&json).unwrap();
        assert_eq!(acs, deserialized);
    }

    /// 测试 ACS 能力检查
    #[test]
    fn test_acs_capabilities() {
        let acs = create_test_acs();
        
        assert!(acs.supports_streaming());
        assert!(!acs.supports_notification());
        assert!(acs.supports_mq_protocol("MQTT"));
        assert!(!acs.supports_mq_protocol("Kafka"));
    }

    /// 测试 ACS 技能管理
    #[test]
    fn test_acs_skills() {
        let acs = create_test_acs();
        
        // 检查技能存在
        assert!(acs.has_skill("test-skill"));
        assert!(!acs.has_skill("nonexistent"));
        
        // 获取技能
        let skill = acs.get_skill("test-skill").unwrap();
        assert_eq!(skill.name, "测试技能");
        assert_eq!(skill.tags.len(), 2);
        
        // 检查标签
        assert!(acs.has_skill_with_tag("test"));
        assert!(acs.has_skill_with_tag("demo"));
        assert!(!acs.has_skill_with_tag("production"));
    }

    /// 测试 ACS 所有标签获取
    #[test]
    fn test_acs_get_all_tags() {
        let acs = create_test_acs();
        let tags = acs.get_all_tags();
        
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"test"));
        assert!(tags.contains(&"demo"));
    }
}

/// Agent Provider 测试
#[cfg(test)]
mod provider_tests {
    use super::*;

    #[test]
    fn test_provider_default() {
        let provider = AgentProvider::default();
        // 默认值可能为 None
        assert!(provider.country_code.is_none() || provider.country_code == Some("CN".to_string()));
    }

    #[test]
    fn test_provider_custom() {
        let provider = AgentProvider {
            country_code: Some("US".to_string()),
            organization: Some("Test Org".to_string()),
            department: Some("AI Dept".to_string()),
            url: Some("https://example.com".to_string()),
            name: Some("John Doe".to_string()),
            email: Some("john@example.com".to_string()),
            license: Some("MIT".to_string()),
        };

        assert_eq!(provider.country_code, Some("US".to_string()));
        assert_eq!(provider.organization, Some("Test Org".to_string()));
    }
}

/// Agent Capabilities 测试
#[cfg(test)]
mod capabilities_tests {
    use super::*;

    #[test]
    fn test_capabilities_default() {
        let capabilities = AgentCapabilities::default();
        assert!(!capabilities.streaming);
        assert!(!capabilities.notification);
        assert!(capabilities.message_queue.is_empty());
    }

    #[test]
    fn test_capabilities_custom() {
        let capabilities = AgentCapabilities {
            streaming: true,
            notification: true,
            message_queue: vec![
                MqProtocolVersion::Mqtt50,
                MqProtocolVersion::Kafka30,
            ],
        };

        assert!(capabilities.streaming);
        assert!(capabilities.notification);
        assert_eq!(capabilities.message_queue.len(), 2);
    }
}

/// Security Scheme 测试
#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_mutual_tls_security_scheme() {
        let scheme = MutualTlsSecurityScheme {
            r#type: "mutualTLS".to_string(),
            description: Some("双向TLS认证".to_string()),
            x_ca_challenge_base_url: Some("https://ca.example.com".to_string()),
        };

        assert_eq!(scheme.r#type, "mutualTLS");
        assert!(scheme.description.is_some());
    }

    #[test]
    fn test_security_scheme_serialization() {
        let scheme = SecurityScheme::MutualTls(MutualTlsSecurityScheme {
            r#type: "mutualTLS".to_string(),
            description: Some("双向TLS认证".to_string()),
            x_ca_challenge_base_url: None,
        });

        let json = serde_json::to_string(&scheme).unwrap();
        assert!(json.contains("\"type\":\"mutualTLS\""));

        let deserialized: SecurityScheme = serde_json::from_str(&json).unwrap();
        assert_eq!(scheme, deserialized);
    }
}

/// MQ Protocol Version 测试
#[cfg(test)]
mod mq_protocol_tests {
    use super::*;

    #[test]
    fn test_mq_protocol_version() {
        let mqtt = MqProtocolVersion::Mqtt50;
        assert_eq!(mqtt.protocol_name(), "MQTT");
        assert_eq!(mqtt.version(), "5.0");

        let kafka = MqProtocolVersion::Kafka30;
        assert_eq!(kafka.protocol_name(), "Kafka");
        assert_eq!(kafka.version(), "3.0");
    }

    #[test]
    fn test_mq_protocol_serialization() {
        let mqtt = MqProtocolVersion::Mqtt50;
        let json = serde_json::to_string(&mqtt).unwrap();
        assert_eq!(json, "\"mqtt:5.0\"");

        let deserialized: MqProtocolVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(mqtt, deserialized);
    }
}
