//! # AIP - Agent Interaction Protocol (智能体交互协议)
//!
//! AIP v2 协议定义了智能体之间的交互模式，包括任务命令、任务结果、
//! 任务状态和数据项等核心概念。
//!
//! ## 主要结构
//!
//! - [`TaskCommand`]: 任务命令（Leader → Partner）
//! - [`TaskResult`]: 任务结果（Partner → Leader）
//! - [`TaskState`]: 任务状态枚举
//! - [`DataItem`]: 数据项（文本、文件、结构化数据）
//!
//! ## 交互模式
//!
//! AIP 支持两种交互模式：
//!
//! 1. **点对点模式**: Leader 与 Partner 直接通过 JSON-RPC 交互
//! 2. **群组模式**: Leader 通过消息队列与多个 Partner 协作

use serde::{Deserialize, Serialize};

mod models;
mod command;
mod result;
mod rpc;
mod task;

pub use models::*;
pub use command::*;
pub use result::*;
pub use rpc::*;
pub use task::*;

/// 任务状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    /// 已接受
    Accepted,
    
    /// 工作中
    Working,
    
    /// 等待输入
    AwaitingInput,
    
    /// 等待完成
    AwaitingCompletion,
    
    /// 已完成
    Completed,
    
    /// 已取消
    Canceled,
    
    /// 失败
    Failed,
    
    /// 已拒绝
    Rejected,
}

impl TaskState {
    /// 判断任务是否处于终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskState::Completed
                | TaskState::Canceled
                | TaskState::Failed
                | TaskState::Rejected
        )
    }
    
    /// 判断任务是否处于活跃状态
    pub fn is_active(&self) -> bool {
        !self.is_terminal()
    }

    /// 线上取值
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Accepted => "accepted",
            TaskState::Working => "working",
            TaskState::AwaitingInput => "awaiting-input",
            TaskState::AwaitingCompletion => "awaiting-completion",
            TaskState::Completed => "completed",
            TaskState::Canceled => "canceled",
            TaskState::Failed => "failed",
            TaskState::Rejected => "rejected",
        }
    }

    /// 由线上取值解析
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "accepted" => Some(TaskState::Accepted),
            "working" => Some(TaskState::Working),
            "awaiting-input" => Some(TaskState::AwaitingInput),
            "awaiting-completion" => Some(TaskState::AwaitingCompletion),
            "completed" => Some(TaskState::Completed),
            "canceled" => Some(TaskState::Canceled),
            "failed" => Some(TaskState::Failed),
            "rejected" => Some(TaskState::Rejected),
            _ => None,
        }
    }

    /// 状态机允许的流转
    ///
    /// 依据 `ACPs-spec-AIP-v02.01`：
    /// - `accepted` -> `working` / `canceled`
    /// - `working` -> `awaiting-input` / `awaiting-completion` / `failed` / `canceled`
    /// - `awaiting-input` -> `working` / `canceled`
    /// - `awaiting-completion` -> `completed` / `working` / `canceled`
    /// - 终态不可再流转（`rejected` 由新建任务时直接指定）
    pub fn can_transition_to(&self, next: TaskState) -> bool {
        if self.is_terminal() {
            return false;
        }
        if *self == next {
            return true;
        }

        match self {
            TaskState::Accepted => matches!(next, TaskState::Working | TaskState::Canceled),
            TaskState::Working => matches!(
                next,
                TaskState::AwaitingInput
                    | TaskState::AwaitingCompletion
                    | TaskState::Failed
                    | TaskState::Canceled
            ),
            TaskState::AwaitingInput => {
                matches!(next, TaskState::Working | TaskState::Canceled)
            }
            TaskState::AwaitingCompletion => matches!(
                next,
                TaskState::Completed | TaskState::Working | TaskState::Canceled
            ),
            _ => false,
        }
    }
}

/// 任务命令类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskCommandType {
    /// 获取任务状态
    Get,
    
    /// 启动任务
    Start,
    
    /// 继续任务
    Continue,
    
    /// 取消任务
    Cancel,
    
    /// 完成任务
    Complete,
    
    /// 重新流式传输
    #[serde(rename = "re-stream")]
    ReStream,
}
