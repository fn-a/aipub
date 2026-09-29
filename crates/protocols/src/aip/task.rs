//! AIP 任务运行时
//!
//! 实现 ACPs-spec-AIP-v02.01 的任务状态机与命令处理语义，
//! 对齐参考实现 `acps-sdk/acps_sdk/aip/aip_rpc_server.py` 的 DefaultHandlers。

use std::collections::HashMap;
use uuid::Uuid;
use serde_json::to_string;
use chrono::{Utc, SecondsFormat};
use super::{
    AipErrorCode, DataItem, Product, RpcError, TaskCommand, TaskCommandType, TaskResult, TaskState,
    TaskStatus,
};

/// 任务历史记录上限
pub const DEFAULT_MAX_HISTORY: usize = 1000;

/// 任务记录
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRecord {
    /// 任务 ID
    pub task_id: String,

    /// 会话 ID
    pub session_id: Option<String>,

    /// 群组 ID
    pub group_id: Option<String>,

    /// 当前状态
    pub status: TaskStatus,

    /// 产出物
    pub products: Vec<Product>,

    /// 命令历史
    pub command_history: Vec<TaskCommand>,

    /// 状态历史
    pub status_history: Vec<TaskStatus>,
}

impl TaskRecord {
    /// 创建任务记录（初始状态由调用方指定，`start` 命令默认 `accepted`）
    pub fn new(task_id: impl Into<String>, state: TaskState) -> Self {
        let task_id = task_id.into();
        let status = TaskStatus {
            state,
            state_changed_at: now_iso8601(),
            data_items: None,
        };
        Self {
            task_id,
            session_id: None,
            group_id: None,
            status: status.clone(),
            products: vec![],
            command_history: vec![],
            status_history: vec![status],
        }
    }

    /// 转换为对外返回的任务结果
    pub fn to_result(
        &self,
        message_id: impl Into<String>,
        sent_at: impl Into<String>,
        sender_id: impl Into<String>,
    ) -> TaskResult {
        let mut result = TaskResult::new(
            message_id.into(),
            sent_at.into(),
            sender_id.into(),
            self.task_id.clone(),
            self.status.clone(),
        );
        result.session_id = self.session_id.clone();
        result.group_id = self.group_id.clone();
        if !self.products.is_empty() {
            result.products = Some(self.products.clone());
        }
        if !self.command_history.is_empty() {
            result.command_history = Some(self.command_history.clone());
        }
        if !self.status_history.is_empty() {
            result.status_history = Some(self.status_history.clone());
        }
        result
    }
}

/// 任务管理器（进程内实现，对应参考实现的 `TaskManager`）
#[derive(Debug)]
pub struct TaskManager {
    tasks: HashMap<String, TaskRecord>,
    server_id: String,
    max_history: usize,
}

impl TaskManager {
    /// 创建任务管理器
    ///
    /// `server_id` 为 Partner 自身 AIC，用于填充 `TaskResult.senderId`。
    pub fn new(server_id: impl Into<String>) -> Self {
        Self {
            tasks: HashMap::new(),
            server_id: server_id.into(),
            max_history: DEFAULT_MAX_HISTORY,
        }
    }

    /// 自定义历史记录上限
    pub fn with_max_history(mut self, max_history: usize) -> Self {
        self.max_history = max_history.max(1);
        self
    }

    /// 当前任务数
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// 是否没有任何任务
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// 读取任务记录
    pub fn get(&self, task_id: &str) -> Option<&TaskRecord> {
        self.tasks.get(task_id)
    }

    /// 是否已存在任务
    pub fn contains(&self, task_id: &str) -> bool {
        self.tasks.contains_key(task_id)
    }

    /// 删除任务
    pub fn remove(&mut self, task_id: &str) -> Option<TaskRecord> {
        self.tasks.remove(task_id)
    }

    /// 清空全部任务
    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    /// 处理任务命令，返回任务结果
    ///
    /// 语义对齐参考实现：
    /// - `start`：任务已存在时仅追加命令历史并返回当前快照（幂等）；
    /// - `get`：只读，不改变状态；
    /// - `cancel`：非终态 -> `canceled`，终态下不改变；
    /// - `complete`：仅 `awaiting-completion` -> `completed`；
    /// - `continue`：仅 `awaiting-input`/`awaiting-completion` 且带非空文本数据项有效，不改变状态；
    /// - `re-stream`：流式重放，不改变状态。
    pub fn handle_command(&mut self, command: TaskCommand) -> Result<TaskResult, RpcError> {
        match command.command {
            TaskCommandType::Start => self.handle_start(command),
            TaskCommandType::Get => self.handle_get(command),
            TaskCommandType::Cancel => self.handle_cancel(command),
            TaskCommandType::Complete => self.handle_complete(command),
            TaskCommandType::Continue => self.handle_continue(command),
            TaskCommandType::ReStream => self.handle_get(command),
        }
    }

    /// 处理 `start`
    fn handle_start(&mut self, command: TaskCommand) -> Result<TaskResult, RpcError> {
        let task_id = command.task_id.clone().ok_or_else(|| {
            RpcError::invalid_params("A command to start a task must have a taskId.")
        })?;

        if let Some(record) = self.tasks.get_mut(&task_id) {
            record.push_command(command, self.max_history);
            let message_id = new_message_id("result");
            let sent_at = now_iso8601();
            return Ok(record.to_result(message_id, sent_at, self.server_id.clone()));
        }

        let mut record = TaskRecord::new(task_id, TaskState::Accepted);
        record.session_id = command.session_id.clone();
        record.group_id = command.group_id.clone();
        record.push_command(command, self.max_history);

        let message_id = new_message_id("result");
        let sent_at = now_iso8601();
        let result = record.to_result(message_id, sent_at, self.server_id.clone());
        self.tasks.insert(record.task_id.clone(), record);
        Ok(result)
    }

    /// 处理 `get` / `re-stream`
    fn handle_get(&mut self, command: TaskCommand) -> Result<TaskResult, RpcError> {
        let task_id = self.require_task_id(&command)?;
        let record = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| RpcError::task_not_found(&task_id))?;

        record.push_command(command, self.max_history);

        let message_id = new_message_id("result");
        let sent_at = now_iso8601();
        Ok(record.to_result(message_id, sent_at, self.server_id.clone()))
    }

    /// 处理 `cancel`
    fn handle_cancel(&mut self, command: TaskCommand) -> Result<TaskResult, RpcError> {
        let task_id = self.require_task_id(&command)?;
        let record = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| RpcError::task_not_found(&task_id))?;

        record.push_command(command, self.max_history);

        if record.status.state.is_active() {
            record.apply_state(TaskState::Canceled, None);
        }

        let message_id = new_message_id("result");
        let sent_at = now_iso8601();
        Ok(record.to_result(message_id, sent_at, self.server_id.clone()))
    }

    /// 处理 `complete`
    fn handle_complete(&mut self, command: TaskCommand) -> Result<TaskResult, RpcError> {
        let task_id = self.require_task_id(&command)?;
        let record = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| RpcError::task_not_found(&task_id))?;

        record.push_command(command, self.max_history);

        if record.status.state == TaskState::AwaitingCompletion {
            record.apply_state(TaskState::Completed, None);
        }

        let message_id = new_message_id("result");
        let sent_at = now_iso8601();
        Ok(record.to_result(message_id, sent_at, self.server_id.clone()))
    }

    /// 处理 `continue`
    fn handle_continue(&mut self, command: TaskCommand) -> Result<TaskResult, RpcError> {
        let task_id = self.require_task_id(&command)?;

        if !contains_non_blank_text(command.data_items.as_deref()) {
            return Err(RpcError::invalid_params(
                "A continue command must carry at least one non-blank text data item.",
            ));
        }

        let record = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| RpcError::task_not_found(&task_id))?;

        record.push_command(command, self.max_history);

        // 参考实现的默认处理器不改变状态
        let message_id = new_message_id("result");
        let sent_at = now_iso8601();
        Ok(record.to_result(message_id, sent_at, self.server_id.clone()))
    }

    /// 显式状态流转（供 Partner 业务逻辑调用）
    pub fn transition(
        &mut self,
        task_id: &str,
        state: TaskState,
        data_items: Option<Vec<DataItem>>,
    ) -> Result<TaskResult, RpcError> {
        let record = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| RpcError::task_not_found(task_id))?;

        if !record.status.state.can_transition_to(state) {
            return Err(RpcError::with_message(
                AipErrorCode::UnsupportedOperation,
                format!(
                    "任务 {} 不允许从 {} 流转到 {}",
                    task_id,
                    record.status.state.as_str(),
                    state.as_str()
                ),
            ));
        }

        record.apply_state(state, data_items);

        let message_id = new_message_id("result");
        let sent_at = now_iso8601();
        Ok(record.to_result(message_id, sent_at, self.server_id.clone()))
    }

    /// 写入产出物，可选校验总字节数上限
    pub fn set_products(
        &mut self,
        task_id: &str,
        products: Vec<Product>,
        max_products_bytes: Option<u64>,
    ) -> Result<(), RpcError> {
        if let Some(max_bytes) = max_products_bytes {
            let mut total: u64 = 0;
            for product in &products {
                for item in &product.data_items {
                    total += data_item_size(item);
                }
            }
            if total > max_bytes {
                return Err(RpcError::with_message(
                    AipErrorCode::UnsupportedOperation,
                    format!(
                        "Products size {} bytes exceeds maxProductsBytes={}.",
                        total, max_bytes
                    ),
                ));
            }
        }

        let record = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| RpcError::task_not_found(task_id))?;
        record.products = products;
        Ok(())
    }

    /// 将任务标记为失败（对应参考实现中处理器异常的处理方式）
    pub fn fail(&mut self, task_id: &str, reason: impl AsRef<str>) -> Result<TaskResult, RpcError> {
        let data_items = vec![DataItem::Text(super::TextDataItem {
            text: format!("Agent execution failed: {}", reason.as_ref()),
            metadata: None,
        })];
        self.transition(task_id, TaskState::Failed, Some(data_items))
    }

    /// 取出必填的任务 ID
    fn require_task_id(&self, command: &TaskCommand) -> Result<String, RpcError> {
        command
            .task_id
            .clone()
            .ok_or_else(|| RpcError::invalid_params("taskId is required for non-Start commands."))
    }
}

impl TaskRecord {
    /// 追加命令历史（超出上限时裁剪最旧记录）
    fn push_command(&mut self, command: TaskCommand, max_history: usize) {
        self.command_history.push(command);
        if self.command_history.len() > max_history {
            let overflow = self.command_history.len() - max_history;
            self.command_history.drain(0..overflow);
        }
    }

    /// 应用状态流转并记录历史
    fn apply_state(&mut self, state: TaskState, data_items: Option<Vec<DataItem>>) {
        let status = TaskStatus {
            state,
            state_changed_at: now_iso8601(),
            data_items,
        };
        self.status = status.clone();
        self.status_history.push(status);
    }
}

/// 检查数据项中是否存在非空文本
fn contains_non_blank_text(data_items: Option<&[DataItem]>) -> bool {
    data_items
        .map(|items| {
            items.iter().any(|item| match item {
                DataItem::Text(text) => !text.text.trim().is_empty(),
                _ => false,
            })
        })
        .unwrap_or(false)
}

/// 数据项字节数（文本按 UTF-8 长度，文件按 base64 解码长度估算）
fn data_item_size(item: &DataItem) -> u64 {
    match item {
        DataItem::Text(text) => text.text.len() as u64,
        DataItem::File(file) => file.bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0),
        DataItem::Data(data) => to_string(&data.data)
            .map(|s| s.len() as u64)
            .unwrap_or(0),
    }
}

/// 生成带前缀的消息 ID
pub fn new_message_id(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

/// 当前时间（UTC，RFC 3339）
pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
