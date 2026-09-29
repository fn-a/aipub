//! DSP (Data Sync Protocol) 变更流测试
//!
//! 测试变更记录的追加、增量拉取、保留上限与序列化

use aipub::protocols::dsp::*;

#[cfg(test)]
mod tests {
    use serde_json::{json, to_value, from_value};
    use super::*;

    #[test]
    fn test_append_and_incremental_pull() {
        let mut feed = ChangeFeed::new();
        assert_eq!(feed.append(ChangeOp::Upsert, "aic-1", 1, Some(json!({"aic": "aic-1"}))), 1);
        assert_eq!(feed.append(ChangeOp::Upsert, "aic-2", 1, Some(json!({"aic": "aic-2"}))), 2);
        assert_eq!(feed.append(ChangeOp::Delete, "aic-1", 2, None), 3);

        let first = feed.changes_since(0, 10);
        assert_eq!(first.len(), 3);
        assert_eq!(first[0].seq, 1);

        let incremental = feed.changes_since(2, 10);
        assert_eq!(incremental.len(), 1);
        assert!(incremental[0].is_delete());
        assert_eq!(feed.latest_seq(), 3);
    }

    #[test]
    fn test_retention_trimming() {
        let mut feed = ChangeFeed::new().with_retention(2);
        feed.append(ChangeOp::Upsert, "aic-1", 1, None);
        feed.append(ChangeOp::Upsert, "aic-2", 1, None);
        feed.append(ChangeOp::Upsert, "aic-3", 1, None);

        assert_eq!(feed.len(), 2);
        let entries = feed.snapshot();
        assert_eq!(entries[0].seq, 2);
    }

    #[test]
    fn test_change_entry_serialization() {
        let entry = ChangeEntry::upsert(7, "aic-1", 3, json!({"aic": "aic-1"}));
        let value = to_value(&entry).unwrap();
        assert_eq!(value["seq"], 7);
        assert_eq!(value["op"], "upsert");
        assert_eq!(value["type"], "acs");
        assert_eq!(value["id"], "aic-1");
        assert_eq!(value["version"], 3);

        let restore: ChangeEntry = from_value(value).unwrap();
        assert_eq!(restore, entry);
    }
}
