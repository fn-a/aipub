//! CRL (证书吊销列表)
//!
//! 只收录「已吊销且带吊销原因」的证书，与参考实现
//! `status=revoked AND revoked_at IS NOT NULL AND revocation_reason IS NOT NULL`
//! 的筛选条件一致。

use chrono::{DateTime, Duration, Utc};
use protocols::atr::{RevocationReason, CA_ISSUER_LABEL, CRL_NEXT_UPDATE_HOURS};

/// CRL 条目
#[derive(Debug, Clone, PartialEq)]
pub struct CrlEntry {
    /// 证书序列号
    pub serial_number: String,

    /// 吊销时间
    pub revoked_at: DateTime<Utc>,

    /// 吊销原因
    pub reason: RevocationReason,
}

/// 证书吊销列表
#[derive(Debug, Clone, PartialEq)]
pub struct Crl {
    /// CRL 版本号（每次吊销自增）
    pub version: u64,

    /// 生成时间
    pub generated_at: DateTime<Utc>,

    /// 下次更新时间（生成时间 + 24h）
    pub next_update: DateTime<Utc>,

    /// 签发者
    pub issuer: String,

    /// 条目列表
    pub entries: Vec<CrlEntry>,
}

impl Crl {
    /// 构造 CRL
    pub fn new(version: u64, entries: Vec<CrlEntry>) -> Self {
        let generated_at = Utc::now();
        Self {
            version,
            generated_at,
            next_update: generated_at + Duration::hours(CRL_NEXT_UPDATE_HOURS),
            issuer: CA_ISSUER_LABEL.to_string(),
            entries,
        }
    }

    /// 是否包含指定序列号
    pub fn contains(&self, serial_number: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.serial_number == serial_number)
    }
}
