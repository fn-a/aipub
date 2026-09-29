//! Discovery 存储层测试
//!
//! 测试 ACS 的存取、过滤查询、版本幂等应用与本体前缀检索

use aipub::discovery::*;

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use aipub::protocols::adp::{DiscoveryFilter, FilterCondition};
    use aipub::protocols::dsp::{ChangeEntry, ChangeOp};
    use super::*;

    fn acs(aic: &str, name: &str, active: bool) -> Value {
        json!({
            "aic": aic,
            "active": active,
            "name": name,
            "skills": [{ "id": "s1", "name": "技能", "tags": ["北京"] }]
        })
    }

    #[tokio::test]
    async fn test_store_get_query() {
        let storage = MemoryStorage::new();
        storage.store(acs("aic-1", "北京助手", true)).await.unwrap();
        storage.store(acs("aic-2", "上海助手", false)).await.unwrap();

        assert_eq!(storage.len().await, 2);
        assert!(storage.get("aic-1").await.unwrap().is_some());
        assert!(storage.get("missing").await.unwrap().is_none());

        let all = storage.query(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let filtered = storage
            .query(Some(DiscoveryFilter::and(vec![FilterCondition::eq(
                "active",
                json!(true),
            )])))
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["aic"], "aic-1");
    }

    #[tokio::test]
    async fn test_store_requires_aic() {
        let storage = MemoryStorage::new();
        let error = storage.store(json!({"name": "缺少 AIC"})).await.unwrap_err();
        assert!(matches!(error, DiscoveryError::StorageError(_)));
    }

    #[tokio::test]
    async fn test_apply_change_is_version_idempotent() {
        let storage = MemoryStorage::new();
        let entry = ChangeEntry::upsert(1, "aic-1", 3, acs("aic-1", "北京助手", true));

        assert!(storage.apply_change(&entry).await.unwrap());

        // 相同版本重复投递被丢弃
        assert!(!storage.apply_change(&entry).await.unwrap());

        // 更低版本被丢弃
        let stale = ChangeEntry::upsert(2, "aic-1", 2, acs("aic-1", "旧版本", true));
        assert!(!storage.apply_change(&stale).await.unwrap());
        assert_eq!(
            storage.get("aic-1").await.unwrap().unwrap()["name"],
            "北京助手"
        );

        // 更高版本被接受
        let newer = ChangeEntry::upsert(3, "aic-1", 4, acs("aic-1", "新版本", true));
        assert!(storage.apply_change(&newer).await.unwrap());
        assert_eq!(
            storage.get("aic-1").await.unwrap().unwrap()["name"],
            "新版本"
        );

        // 删除
        let delete = ChangeEntry::delete(4, "aic-1", 5);
        assert!(storage.apply_change(&delete).await.unwrap());
        assert!(storage.get("aic-1").await.unwrap().is_none());
        assert_eq!(delete.op, ChangeOp::Delete);
    }

    #[tokio::test]
    async fn test_update_bumps_version_and_requires_existing() {
        let storage = MemoryStorage::new();
        storage.store(acs("aic-1", "北京助手", true)).await.unwrap();
        storage
            .update("aic-1", acs("aic-1", "北京助手 v2", true))
            .await
            .unwrap();

        let records = storage.records().await.unwrap();
        assert_eq!(records[0].version, 2);

        assert!(storage
            .update("missing", acs("missing", "x", true))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_list_by_ontology_prefix() {
        let storage = MemoryStorage::new();
        storage
            .store(acs("1.2.156.3088.1.0001.00001.ABC123.000001.0H9T", "实体1", true))
            .await
            .unwrap();
        storage
            .store(acs("1.2.156.3088.1.0001.00001.ABC123.000002.0H9T", "实体2", true))
            .await
            .unwrap();
        storage
            .store(acs("1.2.156.3088.1.0001.00001.ZZZ999.000001.0H9T", "其它本体", true))
            .await
            .unwrap();

        let entities = storage
            .list_by_ontology_prefix("1.2.156.3088.1.0001.00001.ABC123")
            .await;
        assert_eq!(entities.len(), 2);
    }
}
