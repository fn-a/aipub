//! AIP (Agent Interaction Protocol) 协议测试
//!
//! 测试智能体交互协议的任务命令、任务结果、状态管理等功能

use serde_json::{json, to_string, from_str};
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
        let json = to_string(&state).unwrap();
        assert_eq!(json, "\"working\"");

        let deserialized: TaskState = from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }

    /// 测试任务命令类型
    #[test]
    fn test_task_command_type() {
        let start = TaskCommandType::Start;
        let json = to_string(&start).unwrap();
        assert_eq!(json, "\"start\"");

        let deserialized: TaskCommandType = from_str(&json).unwrap();
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

        let json = to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));

        let deserialized: DataItem = from_str(&json).unwrap();
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

        let json = to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"name\":\"test.txt\""));
    }

    #[test]
    fn test_structured_data_item() {
        let item = DataItem::Data(StructuredDataItem {
            data: json!({"key": "value"}),
            metadata: None,
        });

        let json = to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"data\""));
        assert!(json.contains("\"key\":\"value\""));
    }
}

/// AIP 任务状态机与管理器测试
#[cfg(test)]
mod task_tests {
    use aipub::protocols::aip::{TextDataItem, TaskCommand};
    use super::*;

    fn start_command(task_id: &str) -> TaskCommand {
        let mut command = TaskCommand::start(
            new_message_id("cmd"),
            now_iso8601(),
            "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T".to_string(),
        );
        command.task_id = Some(task_id.to_string());
        command.session_id = Some("session-1".to_string());
        command
    }

    #[test]
    fn test_state_transitions_per_spec() {
        assert!(TaskState::Accepted.can_transition_to(TaskState::Working));
        assert!(TaskState::Accepted.can_transition_to(TaskState::Canceled));
        assert!(!TaskState::Accepted.can_transition_to(TaskState::Completed));

        assert!(TaskState::Working.can_transition_to(TaskState::AwaitingInput));
        assert!(TaskState::Working.can_transition_to(TaskState::AwaitingCompletion));
        assert!(TaskState::Working.can_transition_to(TaskState::Failed));
        assert!(TaskState::Working.can_transition_to(TaskState::Canceled));

        assert!(TaskState::AwaitingInput.can_transition_to(TaskState::Working));
        assert!(TaskState::AwaitingInput.can_transition_to(TaskState::Canceled));

        assert!(TaskState::AwaitingCompletion.can_transition_to(TaskState::Completed));
        assert!(TaskState::AwaitingCompletion.can_transition_to(TaskState::Working));

        assert!(!TaskState::Completed.can_transition_to(TaskState::Working));
        assert!(!TaskState::Canceled.can_transition_to(TaskState::Working));
    }

    #[test]
    fn test_start_creates_task_at_accepted() {
        let mut manager = TaskManager::new("partner-aic");
        let result = manager.handle_command(start_command("task-1")).unwrap();

        assert_eq!(result.status.state, TaskState::Accepted);
        assert_eq!(result.task_id, "task-1");
        assert_eq!(result.sender_role, "partner");
        assert_eq!(result.sender_id, "partner-aic");
        assert_eq!(result.session_id.as_deref(), Some("session-1"));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_start_is_idempotent() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();
        manager
            .transition("task-1", TaskState::Working, None)
            .unwrap();

        let result = manager.handle_command(start_command("task-1")).unwrap();
        assert_eq!(result.status.state, TaskState::Working);
        assert_eq!(manager.get("task-1").unwrap().command_history.len(), 2);
    }

    #[test]
    fn test_start_requires_task_id() {
        let mut manager = TaskManager::new("partner-aic");
        let mut command = start_command("task-1");
        command.task_id = None;
        let error = manager.handle_command(command).unwrap_err();
        assert_eq!(error.code, -32602);
    }

    #[test]
    fn test_non_start_requires_task_id_and_existing_task() {
        let mut manager = TaskManager::new("partner-aic");

        let mut command = TaskCommand::get(
            new_message_id("cmd"),
            now_iso8601(),
            "leader-aic".to_string(),
            "task-x".to_string(),
        );
        command.task_id = None;
        assert_eq!(manager.handle_command(command).unwrap_err().code, -32602);

        let missing = TaskCommand::get(
            new_message_id("cmd"),
            now_iso8601(),
            "leader-aic".to_string(),
            "task-missing".to_string(),
        );
        let error = manager.handle_command(missing).unwrap_err();
        assert_eq!(error.code, -32001);
    }

    #[test]
    fn test_cancel_from_active_and_terminal_noop() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();
        manager
            .transition("task-1", TaskState::Working, None)
            .unwrap();

        let cancel = TaskCommand::cancel(
            new_message_id("cmd"),
            now_iso8601(),
            "leader-aic".to_string(),
            "task-1".to_string(),
        );
        let result = manager.handle_command(cancel.clone()).unwrap();
        assert_eq!(result.status.state, TaskState::Canceled);

        // 终态下取消不改变状态
        let result = manager.handle_command(cancel).unwrap();
        assert_eq!(result.status.state, TaskState::Canceled);
    }

    #[test]
    fn test_complete_only_from_awaiting_completion() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();

        let complete = TaskCommand::new(
            new_message_id("cmd"),
            now_iso8601(),
            "leader".to_string(),
            "leader-aic".to_string(),
            TaskCommandType::Complete,
        )
        .with_task_id("task-1".to_string());

        // working 状态下不生效
        manager
            .transition("task-1", TaskState::Working, None)
            .unwrap();
        let result = manager.handle_command(complete.clone()).unwrap();
        assert_eq!(result.status.state, TaskState::Working);

        manager
            .transition("task-1", TaskState::AwaitingCompletion, None)
            .unwrap();
        let result = manager.handle_command(complete).unwrap();
        assert_eq!(result.status.state, TaskState::Completed);
        assert!(result.status.state.is_terminal());
    }

    #[test]
    fn test_continue_requires_text_and_keeps_state() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();
        manager
            .transition("task-1", TaskState::Working, None)
            .unwrap();
        manager
            .transition("task-1", TaskState::AwaitingInput, None)
            .unwrap();

        let mut continue_command = TaskCommand::new(
            new_message_id("cmd"),
            now_iso8601(),
            "leader".to_string(),
            "leader-aic".to_string(),
            TaskCommandType::Continue,
        )
        .with_task_id("task-1".to_string());

        // 缺少文本数据项
        assert_eq!(
            manager.handle_command(continue_command.clone()).unwrap_err().code,
            -32602
        );

        continue_command.data_items = Some(vec![DataItem::Text(TextDataItem {
            text: "继续".to_string(),
            metadata: None,
        })]);

        let result = manager.handle_command(continue_command).unwrap();
        assert_eq!(result.status.state, TaskState::AwaitingInput);
    }

    #[test]
    fn test_transition_rejects_illegal_move() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();

        let error = manager
            .transition("task-1", TaskState::Completed, None)
            .unwrap_err();
        assert_eq!(error.code, -32004);
    }

    #[test]
    fn test_set_products_enforces_size_limit() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();

        let product = Product {
            id: "p1".to_string(),
            name: None,
            description: None,
            data_items: vec![DataItem::Text(TextDataItem {
                text: "a".repeat(32),
                metadata: None,
            })],
        };

        assert!(manager.set_products("task-1", vec![product.clone()], Some(8)).is_err());
        assert!(manager.set_products("task-1", vec![product], Some(64)).is_ok());
        assert_eq!(manager.get("task-1").unwrap().products.len(), 1);
    }

    #[test]
    fn test_fail_records_text_data_item() {
        let mut manager = TaskManager::new("partner-aic");
        manager.handle_command(start_command("task-1")).unwrap();
        manager
            .transition("task-1", TaskState::Working, None)
            .unwrap();

        let result = manager.fail("task-1", "boom").unwrap();
        assert_eq!(result.status.state, TaskState::Failed);

        let data_items = result.status.data_items.unwrap();
        match &data_items[0] {
            DataItem::Text(text) => assert!(text.text.contains("Agent execution failed: boom")),
            _ => panic!("期望文本数据项"),
        }
    }
}
