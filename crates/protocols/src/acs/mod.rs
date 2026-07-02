//! # ACS - Agent Capability Specification (智能体能力描述)
//!
//! 基于 ACPs-spec-ACS-v02.01 规范定义的智能体能力描述数据模型。
//!
//! ACS 完整描述了智能体的身份信息、功能能力、技术特性和服务接口，
//! 用于智能体的注册、发现、匹配和协作。
//!
//! ## 主要结构
//!
//! - [`AgentCapabilitySpec`]: ACS 根对象
//! - [`AgentProvider`]: 智能体提供者信息
//! - [`AgentCapabilities`]: 智能体技术能力配置
//! - [`AgentSkill`]: 智能体技能定义
//! - [`AgentEndPoint`]: 智能体服务端点配置
//! - [`SecurityScheme`]: 安全方案定义

mod models;
mod security;
mod capabilities;

pub use models::*;
pub use security::*;
pub use capabilities::*;

use serde::{Deserialize, Serialize};

/// 消息队列协议版本枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MqProtocolVersion {
    /// MQTT 3.1.1
    #[serde(rename = "mqtt:3.1.1")]
    Mqtt311,
    
    /// MQTT 5.0
    #[serde(rename = "mqtt:5.0")]
    Mqtt50,
    
    /// AMQP 0.9.1
    #[serde(rename = "amqp:0.9.1")]
    Amqp091,
    
    /// AMQP 1.0
    #[serde(rename = "amqp:1.0")]
    Amqp10,
    
    /// Kafka 3.0
    #[serde(rename = "kafka:3.0")]
    Kafka30,
    
    /// Redis 7.0
    #[serde(rename = "redis:7.0")]
    Redis70,
    
    /// RabbitMQ 3.11
    #[serde(rename = "rabbitmq:3.11")]
    Rabbitmq311,
}

impl MqProtocolVersion {
    /// 获取协议名称
    pub fn protocol_name(&self) -> &str {
        match self {
            MqProtocolVersion::Mqtt311 | MqProtocolVersion::Mqtt50 => "MQTT",
            MqProtocolVersion::Amqp091 | MqProtocolVersion::Amqp10 => "AMQP",
            MqProtocolVersion::Kafka30 => "Kafka",
            MqProtocolVersion::Redis70 => "Redis",
            MqProtocolVersion::Rabbitmq311 => "RabbitMQ",
        }
    }
    
    /// 获取版本号
    pub fn version(&self) -> &str {
        match self {
            MqProtocolVersion::Mqtt311 => "3.1.1",
            MqProtocolVersion::Mqtt50 => "5.0",
            MqProtocolVersion::Amqp091 => "0.9.1",
            MqProtocolVersion::Amqp10 => "1.0",
            MqProtocolVersion::Kafka30 => "3.0",
            MqProtocolVersion::Redis70 => "7.0",
            MqProtocolVersion::Rabbitmq311 => "3.11",
        }
    }
}