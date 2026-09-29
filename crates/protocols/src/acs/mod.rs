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

use std::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod models;
mod security;
mod capabilities;

pub use models::*;
pub use security::*;
pub use capabilities::*;

/// 端点 URL 中的 AIC 占位符，注册服务审批通过后会替换为真实 AIC
pub const AIC_URL_PLACEHOLDER: &str = "{AIC}";

/// 消息队列协议版本
///
/// 线上取值为自由字符串，参考实现的 `acsSchema.json` 用如下正则约束：
///
/// ```text
/// ^(mqtt|amqp|kafka|redis|rabbitmq):(>=)?\d+\.(\*|\d+(\.\d+)?)( <\d+\.\d+(\.\d+)?)?$
/// ```
///
/// 即支持精确版本（`rabbitmq:3.11`）、最小版本（`mqtt:>=5.0`）、
/// 区间（`kafka:>=2.8 <4.0`）与通配（`mqtt:5.*`）四种写法。
/// 未识别的取值会保留在 [`MqProtocolVersion::Other`] 中，由
/// [`AgentCapabilitySpec::validate`] 按正则校验。
#[derive(Debug, Clone, PartialEq)]
pub enum MqProtocolVersion {
    /// MQTT 3.1.1
    Mqtt311,

    /// MQTT 5.0
    Mqtt50,

    /// AMQP 0.9.1
    Amqp091,

    /// AMQP 1.0
    Amqp10,

    /// Kafka 3.0
    Kafka30,

    /// Redis 7.0
    Redis70,

    /// RabbitMQ 3.11
    Rabbitmq311,

    /// 其它符合规范正则的版本表达式
    Other(String),
}

impl MqProtocolVersion {
    /// 由线上字符串构造
    pub fn from_wire(value: &str) -> Self {
        match value {
            "mqtt:3.1.1" => MqProtocolVersion::Mqtt311,
            "mqtt:5.0" => MqProtocolVersion::Mqtt50,
            "amqp:0.9.1" => MqProtocolVersion::Amqp091,
            "amqp:1.0" => MqProtocolVersion::Amqp10,
            "kafka:3.0" => MqProtocolVersion::Kafka30,
            "redis:7.0" => MqProtocolVersion::Redis70,
            "rabbitmq:3.11" => MqProtocolVersion::Rabbitmq311,
            other => MqProtocolVersion::Other(other.to_string()),
        }
    }

    /// 线上取值
    pub fn as_wire(&self) -> &str {
        match self {
            MqProtocolVersion::Mqtt311 => "mqtt:3.1.1",
            MqProtocolVersion::Mqtt50 => "mqtt:5.0",
            MqProtocolVersion::Amqp091 => "amqp:0.9.1",
            MqProtocolVersion::Amqp10 => "amqp:1.0",
            MqProtocolVersion::Kafka30 => "kafka:3.0",
            MqProtocolVersion::Redis70 => "redis:7.0",
            MqProtocolVersion::Rabbitmq311 => "rabbitmq:3.11",
            MqProtocolVersion::Other(value) => value.as_str(),
        }
    }

    /// 获取协议名称（展示用，如 `MQTT`、`RabbitMQ`）
    pub fn protocol_name(&self) -> &str {
        match self.protocol_family() {
            "mqtt" => "MQTT",
            "amqp" => "AMQP",
            "kafka" => "Kafka",
            "redis" => "Redis",
            "rabbitmq" => "RabbitMQ",
            other => other,
        }
    }

    /// 获取协议族标识（线上前缀，如 `mqtt`、`rabbitmq`）
    pub fn protocol_family(&self) -> &str {
        match self {
            MqProtocolVersion::Mqtt311 | MqProtocolVersion::Mqtt50 => "mqtt",
            MqProtocolVersion::Amqp091 | MqProtocolVersion::Amqp10 => "amqp",
            MqProtocolVersion::Kafka30 => "kafka",
            MqProtocolVersion::Redis70 => "redis",
            MqProtocolVersion::Rabbitmq311 => "rabbitmq",
            MqProtocolVersion::Other(value) => value.split(':').next().unwrap_or(""),
        }
    }

    /// 获取版本表达式部分
    pub fn version(&self) -> &str {
        match self {
            MqProtocolVersion::Mqtt311 => "3.1.1",
            MqProtocolVersion::Mqtt50 => "5.0",
            MqProtocolVersion::Amqp091 => "0.9.1",
            MqProtocolVersion::Amqp10 => "1.0",
            MqProtocolVersion::Kafka30 => "3.0",
            MqProtocolVersion::Redis70 => "7.0",
            MqProtocolVersion::Rabbitmq311 => "3.11",
            MqProtocolVersion::Other(value) => {
                value.split_once(':').map(|(_, v)| v).unwrap_or("")
            }
        }
    }
}

impl fmt::Display for MqProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_wire())
    }
}

impl Serialize for MqProtocolVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_wire())
    }
}

impl<'de> Deserialize<'de> for MqProtocolVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(MqProtocolVersion::from_wire(&value))
    }
}
