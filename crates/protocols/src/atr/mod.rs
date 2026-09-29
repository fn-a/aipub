//! # ATR - Agent Trust Registry 支撑算法与常量
//!
//! 本模块提供 EAB (External Account Binding) 的 HMAC 校验算法与证书相关的
//! 协议常量，供注册服务（签发/消费 EAB）与证书服务（校验 ACME
//! `externalAccountBinding`）共享。
//!
//! ## EAB 校验流程
//!
//! ACME `newAccount` 请求中携带内层 JWS：
//!
//! ```text
//! protected = base64url({"alg":"HS256","kid":"<keyId>","url":"<new-account URL>"})
//! payload   = base64url(<账号 JWK>)
//! signature = base64url(HMAC-SHA256(base64url_decode(macKey), "<protected>.<payload>"))
//! ```
//!
//! 证书服务用 `/internal/eab/consume` 一次消费拿到的 `macKey` 做校验。

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// EAB 凭据默认有效期（小时）
pub const EAB_TTL_HOURS: i64 = 24;

/// 证书默认有效期（天）
pub const DEFAULT_CERT_VALIDITY_DAYS: u32 = 49;

/// 证书有效期上限（天）
pub const MAX_CERT_VALIDITY_DAYS: u32 = 1825;

/// ACME 订单/授权有效期（小时）
pub const ACME_ORDER_TTL_HOURS: i64 = 24;

/// CRL / OCSP 下次更新时间间隔（小时）
pub const CRL_NEXT_UPDATE_HOURS: i64 = 24;

/// CA 签发者标识
pub const CA_ISSUER_LABEL: &str = "Agent Trusted Registration CA";

/// 证书用途
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CertificateUsage {
    /// 客户端认证
    ClientAuth,

    /// 服务端认证
    ServerAuth,
}

impl CertificateUsage {
    /// 线上取值（ACME identifier 的 `usage` 字段）
    pub fn as_str(&self) -> &'static str {
        match self {
            CertificateUsage::ClientAuth => "clientAuth",
            CertificateUsage::ServerAuth => "serverAuth",
        }
    }

    /// 由线上取值解析
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "clientAuth" => Some(CertificateUsage::ClientAuth),
            "serverAuth" => Some(CertificateUsage::ServerAuth),
            _ => None,
        }
    }

    /// X.509 扩展密钥用法名称（供后续接入真实签发时使用）
    pub fn extended_key_usage(&self) -> &'static str {
        match self {
            CertificateUsage::ClientAuth => "clientAuth",
            CertificateUsage::ServerAuth => "serverAuth",
        }
    }
}

/// 证书吊销原因
///
/// 对应参考实现 `ca-server` 实现的 RFC 5280 原因码（仅 0~5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RevocationReason {
    /// 0 - 未指定
    Unspecified,

    /// 1 - 私钥泄露
    KeyCompromise,

    /// 2 - CA 泄露
    CaCompromise,

    /// 3 - 隶属关系变更
    AffiliationChanged,

    /// 4 - 被取代
    Superseded,

    /// 5 - 停止使用
    CessationOfOperation,
}

impl Default for RevocationReason {
    fn default() -> Self {
        RevocationReason::Unspecified
    }
}

impl RevocationReason {
    /// 数值原因码
    pub fn code(&self) -> u8 {
        match self {
            RevocationReason::Unspecified => 0,
            RevocationReason::KeyCompromise => 1,
            RevocationReason::CaCompromise => 2,
            RevocationReason::AffiliationChanged => 3,
            RevocationReason::Superseded => 4,
            RevocationReason::CessationOfOperation => 5,
        }
    }

    /// 名称（RFC 5280 拼写）
    pub fn name(&self) -> &'static str {
        match self {
            RevocationReason::Unspecified => "unspecified",
            RevocationReason::KeyCompromise => "keyCompromise",
            RevocationReason::CaCompromise => "cACompromise",
            RevocationReason::AffiliationChanged => "affiliationChanged",
            RevocationReason::Superseded => "superseded",
            RevocationReason::CessationOfOperation => "cessationOfOperation",
        }
    }

    /// 由数值原因码解析（仅 0~5 有效）
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(RevocationReason::Unspecified),
            1 => Some(RevocationReason::KeyCompromise),
            2 => Some(RevocationReason::CaCompromise),
            3 => Some(RevocationReason::AffiliationChanged),
            4 => Some(RevocationReason::Superseded),
            5 => Some(RevocationReason::CessationOfOperation),
            _ => None,
        }
    }

    /// 由参考实现的字符串形式解析（含中文别名「续期替换」）
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "续期替换" => Some(RevocationReason::Superseded),
            "unspecified" => Some(RevocationReason::Unspecified),
            "keyCompromise" => Some(RevocationReason::KeyCompromise),
            "cACompromise" | "caCompromise" => Some(RevocationReason::CaCompromise),
            "affiliationChanged" => Some(RevocationReason::AffiliationChanged),
            "superseded" => Some(RevocationReason::Superseded),
            "cessationOfOperation" => Some(RevocationReason::CessationOfOperation),
            _ => None,
        }
    }
}

/// 计算 HMAC-SHA256
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;

    let mut normalized_key = if key.len() > BLOCK_SIZE {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    normalized_key.resize(BLOCK_SIZE, 0);

    let mut inner_pad = vec![0x36u8; BLOCK_SIZE];
    let mut outer_pad = vec![0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        inner_pad[i] ^= normalized_key[i];
        outer_pad[i] ^= normalized_key[i];
    }

    let mut inner = Sha256::new();
    inner.update(&inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&outer_pad);
    outer.update(inner_digest);
    outer.finalize().to_vec()
}

/// 生成 32 字节随机 macKey（base64url 无填充）
pub fn generate_mac_key() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

/// 生成内层 JWS 签名段（base64url 无填充）
pub fn sign_eab_hmac(mac_key: &str, protected: &str, payload: &str) -> Option<String> {
    let key = URL_SAFE_NO_PAD.decode(mac_key).ok()?;
    let signing_input = format!("{}.{}", protected, payload);
    Some(URL_SAFE_NO_PAD.encode(hmac_sha256(&key, signing_input.as_bytes())))
}

/// 校验 ACME `externalAccountBinding` 内部 JWS 的 HMAC-SHA256 签名
pub fn verify_eab_hmac(
    mac_key: &str,
    protected: &str,
    payload: &str,
    signature: &str,
) -> bool {
    let Ok(key) = URL_SAFE_NO_PAD.decode(mac_key) else {
        return false;
    };
    let Ok(expected_signature) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };

    let signing_input = format!("{}.{}", protected, payload);
    let actual = hmac_sha256(&key, signing_input.as_bytes());

    constant_time_eq(&actual, &expected_signature)
}

/// 常量时间字节比较
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// 校验并归一化证书有效期（天）
pub fn normalize_validity_days(requested: Option<u32>) -> u32 {
    let requested = requested.unwrap_or(DEFAULT_CERT_VALIDITY_DAYS).max(1);
    requested.min(MAX_CERT_VALIDITY_DAYS)
}
