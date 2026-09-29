//! 证书模型与工具
//!
//! 真实 X.509 签发（CSR 解析、密钥校验、ASN.1 编码）尚未接入，
//! 当前由 [`crate::CertifierServiceImpl`] 生成显式标注为 mock 的占位证书材料，
//! 用于打通 ACME 流程与状态机。

use std::fmt::{Display, Formatter, Result};
use chrono::{DateTime, Utc};
use protocols::atr::{CertificateUsage, RevocationReason};

/// 证书
#[derive(Debug, Clone, PartialEq)]
pub struct Certificate {
    /// 证书序列号（十六进制大写）
    pub serial_number: String,

    /// 证书 PEM（当前为占位内容）
    pub cert_pem: String,

    /// 证书链 PEM（叶子 + 中间证书）
    pub chain_pem: String,

    /// 生效时间
    pub not_before: DateTime<Utc>,

    /// 失效时间
    pub not_after: DateTime<Utc>,

    /// 绑定的 AIC（证书 CN）
    pub aic: String,

    /// 证书用途
    pub usage: CertificateUsage,

    /// CSR 指纹（SHA-256 十六进制）
    pub csr_fingerprint: String,

    /// 状态
    pub status: CertStatus,

    /// 吊销时间
    pub revoked_at: Option<DateTime<Utc>>,

    /// 吊销原因
    pub revocation_reason: Option<RevocationReason>,
}

impl Certificate {
    /// 是否已过期
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.not_after
    }

    /// 证书 SAN 中的 URI（`URI:acps://{AIC}`）
    pub fn san_uri(&self) -> String {
        format!("acps://{}", self.aic)
    }

    /// 证书主题（`CN={AIC}`）
    pub fn subject(&self) -> String {
        format!("CN={}", self.aic)
    }

    /// 按当前时间计算有效状态
    pub fn effective_status(&self, now: DateTime<Utc>) -> CertStatus {
        if self.status == CertStatus::Revoked {
            CertStatus::Revoked
        } else if self.is_expired(now) {
            CertStatus::Expired
        } else {
            CertStatus::Valid
        }
    }
}

/// 证书状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertStatus {
    /// 有效
    Valid,

    /// 已吊销
    Revoked,

    /// 已过期
    Expired,

    /// 未知（序列号不存在）
    Unknown,
}

impl CertStatus {
    /// 线上取值
    pub fn as_str(&self) -> &'static str {
        match self {
            CertStatus::Valid => "valid",
            CertStatus::Revoked => "revoked",
            CertStatus::Expired => "expired",
            CertStatus::Unknown => "unknown",
        }
    }
}

impl Display for CertStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.as_str())
    }
}
