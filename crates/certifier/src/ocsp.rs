//! OCSP (在线证书状态协议) 响应

use chrono::{DateTime, Duration, Utc};
use protocols::atr::{RevocationReason, CRL_NEXT_UPDATE_HOURS};
use crate::CertStatus;

/// OCSP 响应
#[derive(Debug, Clone, PartialEq)]
pub struct OcspResponse {
    /// 证书序列号
    pub serial_number: String,

    /// 证书状态
    pub status: CertStatus,

    /// 本次响应时间
    pub this_update: DateTime<Utc>,

    /// 下次更新时间
    pub next_update: DateTime<Utc>,

    /// 吊销时间
    pub revoked_at: Option<DateTime<Utc>>,

    /// 吊销原因
    pub revocation_reason: Option<RevocationReason>,
}

impl OcspResponse {
    /// 构造响应（`next_update` 默认 +24h）
    pub fn new(
        serial_number: impl Into<String>,
        status: CertStatus,
        revoked_at: Option<DateTime<Utc>>,
        revocation_reason: Option<RevocationReason>,
    ) -> Self {
        let this_update = Utc::now();
        Self {
            serial_number: serial_number.into(),
            status,
            this_update,
            next_update: this_update + Duration::hours(CRL_NEXT_UPDATE_HOURS),
            revoked_at,
            revocation_reason,
        }
    }

    /// 构造「未知」响应
    pub fn unknown(serial_number: impl Into<String>) -> Self {
        Self::new(serial_number, CertStatus::Unknown, None, None)
    }

    /// 响应缓存时长（秒），对应 `Cache-Control: max-age`
    pub fn max_age_seconds(&self) -> i64 {
        (self.next_update - self.this_update).num_seconds().max(0)
    }

    /// 是否为「良好」状态
    pub fn is_good(&self) -> bool {
        self.status == CertStatus::Valid
    }
}
