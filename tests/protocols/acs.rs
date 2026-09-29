//! ACS (Agent Capability Specification) 协议测试
//!
//! 测试智能体能力描述的数据模型、验证、序列化等功能

use std::collections::HashMap;
use serde_json::{to_string, from_str};
use aipub::protocols::acs::*;

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

        let json = to_string(&scheme).unwrap();
        assert!(json.contains("\"type\":\"mutualTLS\""));

        let deserialized: SecurityScheme = from_str(&json).unwrap();
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
        let json = to_string(&mqtt).unwrap();
        assert_eq!(json, "\"mqtt:5.0\"");

        let deserialized: MqProtocolVersion = from_str(&json).unwrap();
        assert_eq!(mqtt, deserialized);
    }
}

/// ACS 能力描述校验测试
#[cfg(test)]
mod validation_tests {
    use std::collections::HashMap;
    use aipub::protocols::acs::{AgentCapabilities, AgentProvider, AgentSkill, MqProtocolVersion, MutualTlsSecurityScheme};
    use super::*;

    fn base_acs() -> AgentCapabilitySpec {
        AgentCapabilitySpec {
            aic: "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T".to_string(),
            active: true,
            last_modified_time: "2025-03-15T16:30:00+08:00".to_string(),
            protocol_version: "02.01".to_string(),
            name: "北京旅游助手".to_string(),
            description: "提供北京旅游规划服务".to_string(),
            version: "1.0.0".to_string(),
            icon_url: None,
            documentation_url: None,
            web_app_url: None,
            provider: AgentProvider::default(),
            security_schemes: HashMap::new(),
            end_points: vec![],
            capabilities: AgentCapabilities::default(),
            default_input_modes: vec![],
            default_output_modes: vec![],
            skills: vec![],
            entity_user_id: None,
            entity_meta: None,
        }
    }

    #[test]
    fn test_mq_protocol_version_accepts_spec_expressions() {
        for value in [
            "mqtt:3.1.1",
            "mqtt:5.*",
            "mqtt:>=5.0",
            "kafka:>=2.8 <4.0",
            "rabbitmq:4.2",
            "redis:7.0",
        ] {
            assert!(
                validate_mq_protocol_version(value).is_ok(),
                "应接受 {}",
                value
            );
        }

        for value in ["mqtt", "mqtt:", "foo:1.0", "mqtt:abc", "mqtt:1"] {
            assert!(
                validate_mq_protocol_version(value).is_err(),
                "应拒绝 {}",
                value
            );
        }
    }

    #[test]
    fn test_unknown_mq_version_round_trips() {
        let version = MqProtocolVersion::from_wire("rabbitmq:4.2");
        assert_eq!(version.as_wire(), "rabbitmq:4.2");
        assert_eq!(version.protocol_name(), "RabbitMQ");
        assert_eq!(version.version(), "4.2");

        let json = to_string(&version).unwrap();
        assert_eq!(json, "\"rabbitmq:4.2\"");
        let parsed: MqProtocolVersion = from_str(&json).unwrap();
        assert_eq!(parsed, version);
    }

    #[test]
    fn test_endpoint_requires_mutual_tls() {
        let mut acs = base_acs();
        acs.end_points = vec![AgentEndPoint {
            url: "https://localhost:9021/rpc".to_string(),
            transport: "JSONRPC".to_string(),
            security: None,
        }];
        assert_eq!(
            acs.validate().unwrap_err(),
            "endPoint must use mutualTLS security scheme"
        );

        let mut security = HashMap::new();
        security.insert("mtls".to_string(), vec![]);
        acs.end_points[0].security = Some(vec![security]);
        acs.security_schemes.insert(
            "mtls".to_string(),
            SecurityScheme::MutualTls(MutualTlsSecurityScheme {
                r#type: "mutualTLS".to_string(),
                description: None,
                x_ca_challenge_base_url: None,
            }),
        );
        assert!(acs.validate().is_ok());
    }

    #[test]
    fn test_endpoint_url_scheme_must_match_transport() {
        let mut acs = base_acs();
        let mut security = HashMap::new();
        security.insert("mtls".to_string(), vec![]);

        acs.security_schemes.insert(
            "mtls".to_string(),
            SecurityScheme::MutualTls(MutualTlsSecurityScheme {
                r#type: "mutualTLS".to_string(),
                description: None,
                x_ca_challenge_base_url: None,
            }),
        );

        acs.end_points = vec![AgentEndPoint {
            url: "amqp://localhost:5672/acps".to_string(),
            transport: "JSONRPC".to_string(),
            security: Some(vec![security.clone()]),
        }];
        assert!(acs.validate().is_err());

        acs.end_points[0].transport = "AMQP".to_string();
        assert!(acs.validate().is_ok());
    }

    #[test]
    fn test_apply_registry_metadata_replaces_placeholder() {
        let mut acs = base_acs();
        acs.aic = String::new();
        acs.end_points = vec![AgentEndPoint {
            url: "amqps://mq.example.com:5671/acps?inbox=inbox_{AIC}".to_string(),
            transport: "AMQP".to_string(),
            security: None,
        }];

        acs.apply_registry_metadata("1.2.156.3088.1.0001.00001.ABC123.000001.0H9T", true, "2025-03-15T16:30:00+08:00");

        assert_eq!(
            acs.end_points[0].url,
            "amqps://mq.example.com:5671/acps?inbox=inbox_1.2.156.3088.1.0001.00001.ABC123.000001.0H9T"
        );
        assert!(acs.active);
    }

    #[test]
    fn test_duplicate_skill_ids_rejected() {
        let mut acs = base_acs();
        let skill = AgentSkill {
            id: "skill.a".to_string(),
            name: "技能".to_string(),
            description: "描述".to_string(),
            version: "1.0.0".to_string(),
            tags: vec![],
            examples: None,
            input_modes: None,
            output_modes: None,
        };
        acs.skills = vec![skill.clone(), skill];
        assert!(acs.validate().unwrap_err().contains("技能 ID 重复"));
    }

    #[test]
    fn test_acs_hash_is_stable() {
        let acs = base_acs();
        assert_eq!(acs.acs_hash().unwrap().len(), 64);
        assert_eq!(acs.acs_hash().unwrap(), acs.acs_hash().unwrap());
    }

    #[test]
    fn test_is_discoverable() {
        let mut acs = base_acs();
        assert!(!acs.is_discoverable());

        acs.web_app_url = Some("https://example.com".to_string());
        assert!(acs.is_discoverable());

        acs.active = false;
        assert!(!acs.is_discoverable());
    }
}
