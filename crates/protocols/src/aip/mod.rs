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

mod models;
mod command;
mod result;

pub use models::*;
pub use command::*;
pub use result::*;

use serde::{Deserialize, Serialize};

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
