//! Certifier 服务核心实现

use crate::{CertifierConfig, CertifierError};
use async_trait::async_trait;

/// Certifier 服务 trait
#[async_trait]
pub trait CertifierService: Send + Sync {
    /// 申请证书
    async fn issue_certificate(
        &self,
        csr: Vec<u8>,
        aic: &str,
    ) -> Result<Certificate, CertifierError>;
    
    /// 吊销证书
    async fn revoke_certificate(&self, serial_number: &str) -> Result<(), CertifierError>;
    
    /// 获取证书状态
    async fn get_certificate_status(&self, serial_number: &str) -> Result<CertStatus, CertifierError>;
}

/// 证书
#[derive(Debug, Clone)]
pub struct Certificate {
    pub serial_number: String,
    pub cert_pem: String,
    pub chain_pem: String,
    pub not_before: chrono::DateTime<chrono::Utc>,
    pub not_after: chrono::DateTime<chrono::Utc>,
}

/// 证书状态
#[derive(Debug, Clone)]
pub enum CertStatus {
    Valid,
    Revoked,
    Expired,
    Unknown,
}

/// Certifier 服务实现
pub struct CertifierServiceImpl {
    config: CertifierConfig,
}

impl CertifierServiceImpl {
    pub fn new(config: CertifierConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl CertifierService for CertifierServiceImpl {
    async fn issue_certificate(
        &self,
        _csr: Vec<u8>,
        _aic: &str,
    ) -> Result<Certificate, CertifierError> {
        // TODO: 实现证书颁发
        // 1. 验证 CSR
        // 2. 验证 EAB 凭据
        // 3. 颁发证书
        // 4. 返回证书和链
        
        Err(CertifierError::CertificateError("Not implemented".to_string()))
    }
    
    async fn revoke_certificate(&self, _serial_number: &str) -> Result<(), CertifierError> {
        // TODO: 实现证书吊销
        Ok(())
    }
    
    async fn get_certificate_status(&self, _serial_number: &str) -> Result<CertStatus, CertifierError> {
        // TODO: 实现证书状态查询
        Ok(CertStatus::Unknown)
    }
}
