//! AIP (Agent Interaction Protocol) 协议测试
//!
//! 测试智能体交互协议的任务命令、任务结果、状态管理等功能

use aipub::protocols::aip::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试任务状态
    #[test]
    fn test_task_state() {
        // 测试终态判断
        assert!(!TaskState::Working.is_terminal());
        assert!(!TaskState::Accepted.is_terminal());
        assert!(TaskState::Completed.is_terminal());
        assert!(TaskState::Failed.is_terminal());
        assert!(TaskState::Canceled.is_terminal());
        assert!(TaskState::Rejected.is_terminal());

        // 测试活跃状态
        assert!(TaskState::Working.is_active());
        assert!(TaskState::Accepted.is_active());
        assert!(!TaskState::Completed.is_active());
        assert!(!TaskState::Failed.is_active());
    }

    /// 测试任务状态序列化
    #[test]
    fn test_task_state_serialization() {
        let state = TaskState::Working;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"working\"");

        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }

    /// 测试任务命令类型
    #[test]
    fn test_task_command_type() {
        let start = TaskCommandType::Start;
        let json = serde_json::to_string(&start).unwrap();
        assert_eq!(json, "\"start\"");

        let deserialized: TaskCommandType = serde_json::from_str(&json).unwrap();
        assert_eq!(start, deserialized);
    }
}

/// Task Command 测试
#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn test_task_command_start() {
        let cmd = TaskCommand::start(
            "cmd-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "leader-001".to_string(),
        );

        assert_eq!(cmd.id, "cmd-001");
        assert_eq!(cmd.command, TaskCommandType::Start);
        assert_eq!(cmd.sender_role, "leader");
        assert_eq!(cmd.sender_id, "leader-001");
        assert!(cmd.task_id.is_none());
    }

    #[test]
    fn test_task_command_get() {
        let cmd = TaskCommand::get(
            "cmd-002".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "leader-001".to_string(),
            "task-001".to_string(),
        );

        assert_eq!(cmd.command, TaskCommandType::Get);
        assert_eq!(cmd.task_id, Some("task-001".to_string()));
    }

    #[test]
    fn test_task_command_cancel() {
        let cmd = TaskCommand::cancel(
            "cmd-003".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "leader-001".to_string(),
            "task-001".to_string(),
        );

        assert_eq!(cmd.command, TaskCommandType::Cancel);
        assert_eq!(cmd.task_id, Some("task-001".to_string()));
    }

    #[test]
    fn test_task_command_builder() {
        let cmd = TaskCommand::start(
            "cmd-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "leader-001".to_string(),
        )
        .with_task_id("task-001".to_string())
        .with_group_id("group-001".to_string());

        assert_eq!(cmd.task_id, Some("task-001".to_string()));
        assert_eq!(cmd.group_id, Some("group-001".to_string()));
    }
}

/// Task Result 测试
#[cfg(test)]
mod result_tests {
    use super::*;

    #[test]
    fn test_task_result() {
        let status = TaskStatus {
            state: TaskState::Working,
            state_changed_at: "2025-03-15T16:30:00+08:00".to_string(),
            data_items: None,
        };

        let result = TaskResult::new(
            "result-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "partner-001".to_string(),
            "task-001".to_string(),
            status,
        );

        assert_eq!(result.id, "result-001");
        assert_eq!(result.task_id, "task-001");
        assert_eq!(result.sender_role, "partner");
        assert_eq!(result.sender_id, "partner-001");
        assert_eq!(result.status.state, TaskState::Working);
        assert!(!result.is_completed());
    }

    #[test]
    fn test_task_result_completed() {
        let status = TaskStatus {
            state: TaskState::Completed,
            state_changed_at: "2025-03-15T16:30:00+08:00".to_string(),
            data_items: None,
        };

        let result = TaskResult::new(
            "result-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "partner-001".to_string(),
            "task-001".to_string(),
            status,
        );

        assert!(result.is_completed());
    }

    #[test]
    fn test_task_result_builder() {
        let status = TaskStatus {
            state: TaskState::Completed,
            state_changed_at: "2025-03-15T16:30:00+08:00".to_string(),
            data_items: None,
        };

        let product = Product {
            id: "product-001".to_string(),
            name: Some("Result".to_string()),
            description: None,
            data_items: vec![],
        };

        let result = TaskResult::new(
            "result-001".to_string(),
            "2025-03-15T16:30:00+08:00".to_string(),
            "partner-001".to_string(),
            "task-001".to_string(),
            status,
        )
        .with_products(vec![product])
        .with_group_id("group-001".to_string());

        assert!(result.products.is_some());
        assert_eq!(result.products.as_ref().unwrap().len(), 1);
        assert_eq!(result.group_id, Some("group-001".to_string()));
    }
}

/// Data Item 测试
#[cfg(test)]
mod data_item_tests {
    use super::*;

    #[test]
    fn test_text_data_item() {
        let item = DataItem::Text(TextDataItem {
            text: "Hello".to_string(),
            metadata: None,
        });

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));

        let deserialized: DataItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_file_data_item() {
        let item = DataItem::File(FileDataItem {
            name: Some("test.txt".to_string()),
            mime_type: Some("text/plain".to_string()),
            uri: Some("file:///test.txt".to_string()),
            bytes: None,
            metadata: None,
        });

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"name\":\"test.txt\""));
    }

    #[test]
    fn test_structured_data_item() {
        let item = DataItem::Data(StructuredDataItem {
            data: serde_json::json!({"key": "value"}),
            metadata: None,
        });

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"data\""));
        assert!(json.contains("\"key\":\"value\""));
    }
}
