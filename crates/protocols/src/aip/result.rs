//! AIP 任务结果

use super::{DataItem, Product, TaskStatus};
use serde::{Deserialize, Serialize};

/// 任务结果
///
/// Partner 向 Leader 返回的任务状态和结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
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
    
    /// 任务ID
    #[serde(rename = "taskId")]
    pub task_id: String,
    
    /// 任务状态
    pub status: TaskStatus,
    
    /// 产出物列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<Product>>,
    
    /// 命令历史
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "commandHistory")]
    pub command_history: Option<Vec<super::TaskCommand>>,
    
    /// 状态历史
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "statusHistory")]
    pub status_history: Option<Vec<TaskStatus>>,
}

impl TaskResult {
    /// 创建新的任务结果
    pub fn new(
        id: String,
        sent_at: String,
        sender_id: String,
        task_id: String,
        status: TaskStatus,
    ) -> Self {
        Self {
            message_type: "task-result".to_string(),
            id,
            sent_at,
            sender_role: "partner".to_string(),
            sender_id,
            mentions: None,
            data_items: None,
            group_id: None,
            session_id: None,
            task_id,
            status,
            products: None,
            command_history: None,
            status_history: None,
        }
    }
    
    /// 设置产出物
    pub fn with_products(mut self, products: Vec<Product>) -> Self {
        self.products = Some(products);
        self
    }
    
    /// 设置群组ID
    pub fn with_group_id(mut self, group_id: String) -> Self {
        self.group_id = Some(group_id);
        self
    }
    
    /// 判断任务是否完成
    pub fn is_completed(&self) -> bool {
        self.status.state.is_terminal()
    }
}
