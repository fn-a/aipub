//! AIP 任务命令

use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::{DataItem, TaskCommandType};

/// 任务命令
///
/// Leader 向 Partner 发送的任务相关命令
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCommand {
    /// 消息类型
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// 消息ID
    pub id: String,
    
    /// 发送时间
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
    pub mentions: Option<super::Mentions>,
    
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
    
    /// 命令类型
    pub command: TaskCommandType,
    
    /// 命令参数
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "commandParams")]
    pub command_params: Option<Value>,
    
    /// 任务ID（新建任务时可不填）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskId")]
    pub task_id: Option<String>,
}

impl TaskCommand {
    /// 创建新的任务命令
    pub fn new(
        id: String,
        sent_at: String,
        sender_role: String,
        sender_id: String,
        command: TaskCommandType,
    ) -> Self {
        Self {
            message_type: "task-command".to_string(),
            id,
            sent_at,
            sender_role,
            sender_id,
            mentions: None,
            data_items: None,
            group_id: None,
            session_id: None,
            command,
            command_params: None,
            task_id: None,
        }
    }
    
    /// 创建 Start 命令
    pub fn start(id: String, sent_at: String, sender_id: String) -> Self {
        Self::new(
            id,
            sent_at,
            "leader".to_string(),
            sender_id,
            TaskCommandType::Start,
        )
    }
    
    /// 创建 Get 命令
    pub fn get(id: String, sent_at: String, sender_id: String, task_id: String) -> Self {
        let mut cmd = Self::new(
            id,
            sent_at,
            "leader".to_string(),
            sender_id,
            TaskCommandType::Get,
        );
        cmd.task_id = Some(task_id);
        cmd
    }
    
    /// 创建 Cancel 命令
    pub fn cancel(id: String, sent_at: String, sender_id: String, task_id: String) -> Self {
        let mut cmd = Self::new(
            id,
            sent_at,
            "leader".to_string(),
            sender_id,
            TaskCommandType::Cancel,
        );
        cmd.task_id = Some(task_id);
        cmd
    }
    
    /// 设置任务ID
    pub fn with_task_id(mut self, task_id: String) -> Self {
        self.task_id = Some(task_id);
        self
    }
    
    /// 设置数据项
    pub fn with_data_items(mut self, data_items: Vec<DataItem>) -> Self {
        self.data_items = Some(data_items);
        self
    }
    
    /// 设置群组ID
    pub fn with_group_id(mut self, group_id: String) -> Self {
        self.group_id = Some(group_id);
        self
    }
}

/// Get 命令参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCommandParams {
    /// 最后命令发送时间
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastCommandSentAt")]
    pub last_command_sent_at: Option<String>,
    
    /// 最后状态变更时间
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastStateChangedAt")]
    pub last_state_changed_at: Option<String>,
}

/// Start 命令参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartCommandParams {
    /// 超时时间（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    
    /// 最大产出物字节数
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxProductsBytes")]
    pub max_products_bytes: Option<u64>,
}
