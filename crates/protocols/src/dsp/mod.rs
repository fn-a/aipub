//! 变更流（DSP - Data Sync Protocol）
//!
//! 注册服务在 ACS 发生变化时追加一条变更记录，发现服务按 `seq` 增量拉取
//! 并应用。对应参考实现的 `/acps-dsp-v2/changes` 与 `/snapshots` 能力。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{Utc, SecondsFormat};

/// 变更操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeOp {
    /// 新增或更新
    Upsert,

    /// 删除
    Delete,
}

impl ChangeOp {
    /// 线上取值
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeOp::Upsert => "upsert",
            ChangeOp::Delete => "delete",
        }
    }
}

/// 变更类型（当前仅支持 ACS）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    /// 智能体能力描述
    Acs,
}

impl ChangeType {
    /// 线上取值
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::Acs => "acs",
        }
    }
}

/// 变更记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeEntry {
    /// 全局递增序号
    pub seq: u64,

    /// 变更时间（RFC 3339）
    pub ts: String,

    /// 操作类型
    pub op: ChangeOp,

    /// 变更类型
    #[serde(rename = "type")]
    pub change_type: ChangeType,

    /// 对象标识（AIC）
    pub id: String,

    /// 对象版本号，发现服务仅在版本严格更大时应用
    pub version: u64,

    /// 变更负载（删除时为 null）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

impl ChangeEntry {
    /// 构造新增/更新记录
    pub fn upsert(seq: u64, aic: &str, version: u64, payload: Value) -> Self {
        Self {
            seq,
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            op: ChangeOp::Upsert,
            change_type: ChangeType::Acs,
            id: aic.to_string(),
            version,
            payload: Some(payload),
        }
    }

    /// 构造删除记录
    pub fn delete(seq: u64, aic: &str, version: u64) -> Self {
        Self {
            seq,
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            op: ChangeOp::Delete,
            change_type: ChangeType::Acs,
            id: aic.to_string(),
            version,
            payload: None,
        }
    }

    /// 是否为删除操作
    pub fn is_delete(&self) -> bool {
        matches!(self.op, ChangeOp::Delete)
    }
}

/// 变更流
///
/// 内存实现，按 `seq` 单调递增追加，支持增量拉取与全量快照。
#[derive(Debug, Default)]
pub struct ChangeFeed {
    entries: Vec<ChangeEntry>,
    next_seq: u64,
    /// 保留的最大记录数（0 表示不限制）
    retention: usize,
}

impl ChangeFeed {
    /// 创建变更流
    pub fn new() -> Self {
        Self {
            entries: vec![],
            next_seq: 1,
            retention: 0,
        }
    }

    /// 设置保留上限（超出后裁剪最旧记录）
    pub fn with_retention(mut self, retention: usize) -> Self {
        self.retention = retention;
        self
    }

    /// 下一个可用序号
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// 当前记录数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 追加一条变更并返回其序号
    pub fn append(&mut self, op: ChangeOp, aic: &str, version: u64, payload: Option<Value>) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;

        let entry = match op {
            ChangeOp::Upsert => ChangeEntry::upsert(seq, aic, version, payload.unwrap_or(Value::Null)),
            ChangeOp::Delete => ChangeEntry::delete(seq, aic, version),
        };
        self.entries.push(entry);

        if self.retention > 0 && self.entries.len() > self.retention {
            let overflow = self.entries.len() - self.retention;
            self.entries.drain(0..overflow);
        }

        seq
    }

    /// 拉取 `seq` 之后的变更（`seq` 为 0 表示从头拉取）
    pub fn changes_since(&self, seq: u64, limit: usize) -> Vec<ChangeEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.seq > seq)
            .take(limit)
            .cloned()
            .collect()
    }

    /// 全量快照
    pub fn snapshot(&self) -> Vec<ChangeEntry> {
        self.entries.clone()
    }

    /// 最新序号
    pub fn latest_seq(&self) -> u64 {
        self.entries.last().map(|entry| entry.seq).unwrap_or(0)
    }
}
