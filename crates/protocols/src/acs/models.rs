//! ACS 核心数据模型

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Error, Value, to_string, from_str, to_value, from_value};
use super::{MqProtocolVersion, SecurityScheme};

/// 智能体能力描述规范的根对象
///
/// 这是 ACS（Agent Capability Specification）的核心数据结构，
/// 完整描述了智能体的身份信息、功能能力、技术特性和服务接口。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilitySpec {
    // === 身份信息（由注册服务分配和维护）===
    
    /// 智能体唯一身份标识符，由注册服务分配
    pub aic: String,
    
    /// 智能体的激活状态，由注册服务维护
    pub active: bool,
    
    /// 智能体能力描述的最后修改时间（ISO 8601格式）
    #[serde(rename = "lastModifiedTime")]
    pub last_modified_time: String,
    
    // === 协议版本 ===
    
    /// 此智能体支持的ACPs协议版本
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    
    // === 基本描述信息 ===
    
    /// 智能体名称
    pub name: String,
    
    /// 智能体详细描述
    pub description: String,
    
    /// 智能体版本号（语义化版本）
    pub version: String,
    
    // === 附加信息（可选）===
    
    /// 智能体图标URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
    
    /// 智能体文档URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "documentationUrl")]
    pub documentation_url: Option<String>,
    
    /// 智能体Web应用URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "webAppUrl")]
    pub web_app_url: Option<String>,
    
    // === 提供者信息 ===
    
    /// 智能体服务提供者信息
    pub provider: AgentProvider,
    
    // === 安全方案 ===
    
    /// 安全方案声明（键是方案名称）
    #[serde(rename = "securitySchemes")]
    pub security_schemes: HashMap<String, SecurityScheme>,
    
    // === 服务端点 ===
    
    /// 智能体端点配置列表
    #[serde(default)]
    #[serde(rename = "endPoints")]
    pub end_points: Vec<AgentEndPoint>,
    
    // === 技术能力 ===
    
    /// 智能体支持的可选能力声明
    pub capabilities: AgentCapabilities,
    
    // === 输入输出格式 ===
    
    /// 默认支持输入MIME类型集合
    #[serde(default)]
    #[serde(rename = "defaultInputModes")]
    pub default_input_modes: Vec<String>,
    
    /// 默认支持输出MIME类型集合
    #[serde(default)]
    #[serde(rename = "defaultOutputModes")]
    pub default_output_modes: Vec<String>,
    
    // === 技能列表 ===
    
    /// 智能体技能集合
    #[serde(default)]
    pub skills: Vec<AgentSkill>,
    
    // === 实体相关信息（可选）===
    
    /// 实体的用户关联ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "entityUserId")]
    pub entity_user_id: Option<String>,
    
    /// 实体的额外元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "entityMeta")]
    pub entity_meta: Option<HashMap<String, Value>>,
}

impl AgentCapabilitySpec {
    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, Error> {
        to_string(self)
    }
    
    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, Error> {
        from_str(json)
    }
    
    /// 序列化为 JSON Value
    pub fn to_value(&self) -> Result<Value, Error> {
        to_value(self)
    }
    
    /// 从 JSON Value 反序列化
    pub fn from_value(value: Value) -> Result<Self, Error> {
        from_value(value)
    }
}

/// 智能体服务提供者信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    /// 国家或地区代码（ISO 3166-1 alpha-2）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
    
    /// 组织名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    
    /// 部门或院系名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    
    /// 官方网站URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    
    /// 法律备案信息或许可证
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    
    /// 联系人姓名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// 联系人邮箱
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// 智能体技术能力配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    /// 是否支持 SSE 流式响应
    pub streaming: bool,
    
    /// 是否支持异步推送通知
    pub notification: bool,
    
    /// 支持的消息队列能力配置
    #[serde(default)]
    #[serde(rename = "messageQueue")]
    pub message_queue: Vec<MqProtocolVersion>,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            streaming: false,
            notification: false,
            message_queue: vec![],
        }
    }
}

/// 智能体技能定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    /// 技能唯一标识符
    pub id: String,
    
    /// 技能名称
    pub name: String,
    
    /// 技能详细描述
    pub description: String,
    
    /// 技能版本号
    pub version: String,
    
    /// 技能标签集合
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// 技能示例
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
    
    /// 技能支持的输入MIME类型
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "inputModes")]
    pub input_modes: Option<Vec<String>>,
    
    /// 技能支持的输出MIME类型
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "outputModes")]
    pub output_modes: Option<Vec<String>>,
}

/// 智能体服务端点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEndPoint {
    /// 端点URL地址
    pub url: String,
    
    /// 传输协议类型（JSONRPC / HTTP_JSON）
    pub transport: String,
    
    /// 安全要求配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
}
