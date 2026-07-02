//! # Certifier - 证书服务 (CA)
//!
//! 基于 ACME 协议实现的智能体证书颁发服务。
//!
//! ## 功能特性
//!
//! - X.509 证书颁发
//! - EAB 凭据验证
//! - CRL (证书吊销列表) 管理
//! - OCSP (在线证书状态协议) 响应

pub mod service;
pub mod certificate;
pub mod crl;
pub mod ocsp;

pub use service::*;
pub use certificate::*;
pub use crl::*;
pub use ocsp::*;

use thiserror::Error;

/// Certifier 服务错误类型
#[derive(Debug, Error)]
pub enum CertifierError {
    #[error("证书错误: {0}")]
    CertificateError(String),
    
    #[error("签名错误: {0}")]
    SignatureError(String),
    
    #[error("验证错误: {0}")]
    VerificationError(String),
    
    #[error("无效的 CSR: {0}")]
    InvalidCsr(String),
    
    #[error("无效的 EAB: {0}")]
    InvalidEab(String),
}

/// Certifier 服务配置
#[derive(Debug, Clone)]
pub struct CertifierConfig {
    /// 服务端口
    pub port: u16,
    
    /// CA 证书路径
    pub ca_cert_path: String,
    
    /// CA 私钥路径
    pub ca_key_path: String,
    
    /// 证书有效期（天）
    pub cert_validity_days: u32,
}

impl Default for CertifierConfig {
    fn default() -> Self {
        Self {
            port: 8082,
            ca_cert_path: "ca.crt".to_string(),
            ca_key_path: "ca.key".to_string(),
            cert_validity_days: 365,
        }
    }
}
