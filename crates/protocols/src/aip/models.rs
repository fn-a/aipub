//! AIP 数据模型

use super::{TaskCommandType, TaskState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 数据项联合类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DataItem {
    /// 文本数据项
    Text(TextDataItem),
    
    /// 文件数据项
    File(FileDataItem),
    
    /// 结构化数据项
    Data(StructuredDataItem),
}

/// 文本数据项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextDataItem {
    /// 文本内容
    pub text: String,
    
    /// 元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 文件数据项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDataItem {
    /// 文件名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// MIME类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// 文件URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    
    /// Base64编码的字节数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<String>,
    
    /// 元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 结构化数据项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredDataItem {
    /// 数据内容
    pub data: Value,
    
    /// 元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 任务状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    /// 状态
    pub state: TaskState,
    
    /// 状态变更时间（ISO 8601）
    #[serde(rename = "stateChangedAt")]
    pub state_changed_at: String,
    
    /// 数据项
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "dataItems")]
    pub data_items: Option<Vec<DataItem>>,
}

/// 产出物
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    /// 产出物ID
    pub id: String,
    
    /// 产出物名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// 产出物描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// 数据项
    #[serde(rename = "dataItems")]
    pub data_items: Vec<DataItem>,
}

/// 消息基类
///
/// 所有交互数据对象都继承自此类。Message 是中立的、双向的、无状态的。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// 消息类型
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// 消息ID
    pub id: String,
    
    /// 发送时间（ISO 8601）
    #[serde(rename = "sentAt")]
    pub sent_at: String,
    
    /// 发送者角色
    #[serde(rename = "senderRole")]
    pub sender_role: String,
    
    /// 发送者ID
    #[serde(rename = "senderId")]
    pub sender_id: String,
    
    /// 提及
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Mentions>,
    
    /// 数据项
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "dataItems")]
    pub data_items: Option<Vec<DataItem>>,
    
    /// 群组ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "groupId")]
    pub group_id: Option<String>,
    
    /// 会话ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// 提及类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mentions {
    /// 提及所有人
    All(String),
    
    /// 提及特定智能体列表
    List(Vec<String>),
}
