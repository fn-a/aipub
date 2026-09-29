//! Certifier 服务核心实现
//!
//! 结构对齐参考实现 `ca-server` 的 ACME 流程（directory -> new-account(EAB)
//! -> new-order -> finalize -> cert -> revoke），但**不实现真实 X.509 签名**：
//! 证书材料由内存 CA 生成可解析的占位 PEM，用于打通流程与验证状态机。
//! 真实密码学签发（rcgen/openssl + CSR 解析）属于后续规划。

use std::collections::HashMap;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;
use protocols::atr::{
    normalize_validity_days, verify_eab_hmac, CertificateUsage, RevocationReason, CA_ISSUER_LABEL,
    DEFAULT_CERT_VALIDITY_DAYS, ACME_ORDER_TTL_HOURS,
};
use crate::certificate::{CertStatus, Certificate};
use crate::crl::{Crl, CrlEntry};
use crate::ocsp::OcspResponse;
use crate::{CertifierConfig, CertifierError};

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

/// ACME 账号
#[derive(Debug, Clone, PartialEq)]
pub struct AcmeAccount {
    pub id: String,
    pub aic: String,
    pub status: AccountStatus,
    pub created_at: DateTime<Utc>,
}

/// ACME 账号状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Valid,
    Deactivated,
    Revoked,
}

/// ACME 订单
#[derive(Debug, Clone, PartialEq)]
pub struct AcmeOrder {
    pub id: String,
    pub account_id: String,
    pub aic: String,
    pub usage: CertificateUsage,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,

    /// 完成签发后的证书序列号
    pub certificate_serial: Option<String>,

    /// 请求的有效期（天）
    pub requested_validity_days: Option<u32>,
}

/// ACME 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    Ready,
    Processing,
    Valid,
    Invalid,
}

impl OrderStatus {
    /// 线上取值
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Ready => "ready",
            OrderStatus::Processing => "processing",
            OrderStatus::Valid => "valid",
            OrderStatus::Invalid => "invalid",
        }
    }
}

/// Certifier 服务实现
pub struct CertifierServiceImpl {
    config: CertifierConfig,
    state: Mutex<CaState>,
}

/// CA 内部状态
#[derive(Debug, Default)]
struct CaState {
    accounts: HashMap<String, AcmeAccount>,
    orders: HashMap<String, AcmeOrder>,
    certificates: HashMap<String, Certificate>,
    crl_version: u64,
}

impl CertifierServiceImpl {
    /// 创建服务实例
    pub fn new(config: CertifierConfig) -> Self {
        Self {
            config,
            state: Mutex::new(CaState::default()),
        }
    }

    /// 当前配置
    pub fn config(&self) -> &CertifierConfig {
        &self.config
    }

    /// 信任根（本实现返回占位 PEM）
    pub fn trust_bundle(&self) -> String {
        pseudo_pem("TRUST BUNDLE", &format!("subject={},mock=true", CA_ISSUER_LABEL))
    }

    /// 创建 ACME 账号并校验 EAB 绑定
    ///
    /// `mac_key` 为注册服务 `/internal/eab/consume` 一次消费得到的密钥；
    /// `protected` / `payload` / `signature` 为 ACME `externalAccountBinding`
    /// 内层 JWS 的三段。
    pub async fn create_account_with_eab(
        &self,
        aic: &str,
        mac_key: &str,
        protected: &str,
        payload: &str,
        signature: &str,
    ) -> Result<AcmeAccount, CertifierError> {
        if !verify_eab_hmac(mac_key, protected, payload, signature) {
            return Err(CertifierError::InvalidEab(
                "EXTERNAL_ACCOUNT_REQUIRED: EAB 签名校验失败".to_string(),
            ));
        }

        let account = AcmeAccount {
            id: format!("acct-{}", Uuid::new_v4().simple()),
            aic: aic.to_string(),
            status: AccountStatus::Valid,
            created_at: Utc::now(),
        };

        self.state
            .lock()
            .await
            .accounts
            .insert(account.id.clone(), account.clone());

        Ok(account)
    }

    /// 读取账号
    pub async fn get_account(&self, account_id: &str) -> Option<AcmeAccount> {
        self.state.lock().await.accounts.get(account_id).cloned()
    }

    /// 创建订单
    ///
    /// 与参考实现一致：订单创建即为 `ready`（无需 challenge 步骤）。
    pub async fn create_order(
        &self,
        account_id: &str,
        aic: &str,
        usage: CertificateUsage,
        requested_validity_days: Option<u32>,
    ) -> Result<AcmeOrder, CertifierError> {
        let state = self.state.lock().await;
        let account = state
            .accounts
            .get(account_id)
            .ok_or_else(|| CertifierError::CertificateError("ACCOUNT_NOT_FOUND".to_string()))?;

        if account.status != AccountStatus::Valid {
            return Err(CertifierError::CertificateError(
                "UNAUTHORIZED: 账号不可用".to_string(),
            ));
        }
        if account.aic != aic {
            return Err(CertifierError::CertificateError(
                "INVALID_IDENTIFIER: 订单标识与账号绑定 AIC 不一致".to_string(),
            ));
        }
        drop(state);

        let now = Utc::now();
        let order = AcmeOrder {
            id: format!("order-{}", Uuid::new_v4().simple()),
            account_id: account_id.to_string(),
            aic: aic.to_string(),
            usage,
            status: OrderStatus::Ready,
            created_at: now,
            expires_at: now + Duration::hours(ACME_ORDER_TTL_HOURS),
            certificate_serial: None,
            requested_validity_days,
        };

        self.state
            .lock()
            .await
            .orders
            .insert(order.id.clone(), order.clone());

        Ok(order)
    }

    /// 读取订单
    pub async fn get_order(&self, order_id: &str) -> Option<AcmeOrder> {
        self.state.lock().await.orders.get(order_id).cloned()
    }

    /// 完成订单并签发证书
    pub async fn finalize_order(
        &self,
        order_id: &str,
        csr: Vec<u8>,
    ) -> Result<Certificate, CertifierError> {
        let order = self
            .get_order(order_id)
            .await
            .ok_or_else(|| CertifierError::CertificateError("ORDER_NOT_FOUND".to_string()))?;

        if order.status != OrderStatus::Ready {
            return Err(CertifierError::CertificateError(format!(
                "ORDER_NOT_READY: 当前状态为 {}",
                order.status.as_str()
            )));
        }

        {
            let mut state = self.state.lock().await;
            if let Some(stored) = state.orders.get_mut(order_id) {
                stored.status = OrderStatus::Processing;
            }
        }

        let mut certificate = self
            .issue_certificate_internal(
                csr,
                &order.aic,
                order.usage,
                order.requested_validity_days,
            )
            .await?;

        certificate.status = CertStatus::Valid;

        {
            let mut state = self.state.lock().await;
            if let Some(stored) = state.orders.get_mut(order_id) {
                stored.status = OrderStatus::Valid;
                stored.certificate_serial = Some(certificate.serial_number.clone());
            }
            state
                .certificates
                .insert(certificate.serial_number.clone(), certificate.clone());
        }

        Ok(certificate)
    }

    /// 按序列号读取证书
    pub async fn get_certificate(&self, serial_number: &str) -> Option<Certificate> {
        self.state.lock().await.certificates.get(serial_number).cloned()
    }

    /// 列出全部证书
    pub async fn list_certificates(&self) -> Vec<Certificate> {
        let mut certificates: Vec<Certificate> = self
            .state
            .lock()
            .await
            .certificates
            .values()
            .cloned()
            .collect();
        certificates.sort_by(|a, b| a.serial_number.cmp(&b.serial_number));
        certificates
    }

    /// 按 AIC 吊销全部有效证书（对应 `/ca/revoke-notify`）
    pub async fn revoke_by_aic(
        &self,
        aic: &str,
        reason: RevocationReason,
    ) -> Result<Vec<String>, CertifierError> {
        let serials: Vec<String> = self
            .state
            .lock()
            .await
            .certificates
            .values()
            .filter(|certificate| certificate.aic == aic && certificate.status == CertStatus::Valid)
            .map(|certificate| certificate.serial_number.clone())
            .collect();

        for serial in &serials {
            self.revoke_with_reason(serial, reason).await?;
        }

        Ok(serials)
    }

    /// 按原因吊销证书
    pub async fn revoke_with_reason(
        &self,
        serial_number: &str,
        reason: RevocationReason,
    ) -> Result<(), CertifierError> {
        let mut state = self.state.lock().await;
        let certificate = state
            .certificates
            .get_mut(serial_number)
            .ok_or_else(|| CertifierError::CertificateError("CERTIFICATE_NOT_FOUND".to_string()))?;

        if certificate.status == CertStatus::Revoked {
            return Err(CertifierError::CertificateError(
                "ALREADY_REVOKED".to_string(),
            ));
        }

        certificate.status = CertStatus::Revoked;
        certificate.revoked_at = Some(Utc::now());
        certificate.revocation_reason = Some(reason);

        state.crl_version += 1;
        Ok(())
    }

    /// 生成 CRL
    pub async fn crl(&self) -> Crl {
        let state = self.state.lock().await;
        let mut entries: Vec<CrlEntry> = state
            .certificates
            .values()
            .filter_map(|certificate| match (certificate.status, certificate.revoked_at) {
                (CertStatus::Revoked, Some(revoked_at)) => Some(CrlEntry {
                    serial_number: certificate.serial_number.clone(),
                    revoked_at,
                    reason: certificate.revocation_reason.unwrap_or_default(),
                }),
                _ => None,
            })
            .collect();
        entries.sort_by(|a, b| a.serial_number.cmp(&b.serial_number));

        Crl::new(state.crl_version, entries)
    }

    /// OCSP 查询
    pub async fn ocsp(&self, serial_number: &str) -> OcspResponse {
        let now = Utc::now();
        let state = self.state.lock().await;

        match state.certificates.get(serial_number) {
            Some(certificate) => OcspResponse::new(
                serial_number,
                certificate.effective_status(now),
                certificate.revoked_at,
                certificate.revocation_reason,
            ),
            None => OcspResponse::unknown(serial_number),
        }
    }

    /// 内部签发实现
    async fn issue_certificate_internal(
        &self,
        csr: Vec<u8>,
        aic: &str,
        usage: CertificateUsage,
        requested_validity_days: Option<u32>,
    ) -> Result<Certificate, CertifierError> {
        if aic.trim().is_empty() {
            return Err(CertifierError::InvalidCsr(
                "INVALID_CERTIFICATE_FORMAT: 证书 CN 必须为 AIC".to_string(),
            ));
        }

        let now = Utc::now();
        let validity_days = normalize_validity_days(
            requested_validity_days.or(Some(self.config.cert_validity_days)),
        );
        let not_after = now + Duration::days(validity_days as i64);

        let serial_number = generate_serial_number();
        let csr_fingerprint = fingerprint(&csr);
        let subject = format!("CN={},O={}", aic, CA_ISSUER_LABEL);

        let cert_pem = pseudo_pem(
            "CERTIFICATE",
            &format!(
                "subject={};issuer={};serial={};notBefore={};notAfter={};eku={};san=URI:acps://{};csrSha256={}",
                subject,
                CA_ISSUER_LABEL,
                serial_number,
                now.to_rfc3339(),
                not_after.to_rfc3339(),
                usage.extended_key_usage(),
                aic,
                csr_fingerprint
            ),
        );

        let chain_pem = format!(
            "{}\n{}",
            cert_pem,
            pseudo_pem("CERTIFICATE", &format!("subject={},role=intermediate", CA_ISSUER_LABEL))
        );

        Ok(Certificate {
            serial_number,
            cert_pem,
            chain_pem,
            not_before: now,
            not_after,
            aic: aic.to_string(),
            usage,
            csr_fingerprint,
            status: CertStatus::Valid,
            revoked_at: None,
            revocation_reason: None,
        })
    }
}

#[async_trait]
impl CertifierService for CertifierServiceImpl {
    async fn issue_certificate(
        &self,
        csr: Vec<u8>,
        aic: &str,
    ) -> Result<Certificate, CertifierError> {
        let certificate = self
            .issue_certificate_internal(
                csr,
                aic,
                CertificateUsage::ClientAuth,
                Some(self.config.cert_validity_days),
            )
            .await?;

        self.state
            .lock()
            .await
            .certificates
            .insert(certificate.serial_number.clone(), certificate.clone());

        Ok(certificate)
    }
    
    async fn revoke_certificate(&self, serial_number: &str) -> Result<(), CertifierError> {
        self.revoke_with_reason(serial_number, RevocationReason::Unspecified)
            .await
    }
    
    async fn get_certificate_status(&self, serial_number: &str) -> Result<CertStatus, CertifierError> {
        let now = Utc::now();
        let state = self.state.lock().await;
        match state.certificates.get(serial_number) {
            Some(certificate) if certificate.status == CertStatus::Revoked => Ok(CertStatus::Revoked),
            Some(certificate) if certificate.is_expired(now) => Ok(CertStatus::Expired),
            Some(_) => Ok(CertStatus::Valid),
            None => Ok(CertStatus::Unknown),
        }
    }
}

/// 生成 159 位随机序列号（十六进制大写）
fn generate_serial_number() -> String {
    let bytes = [
        Uuid::new_v4().as_bytes()[..16].to_vec(),
        Uuid::new_v4().as_bytes()[..4].to_vec(),
    ]
    .concat();

    let mut unique = bytes;
    // 首字节最高位置 0，保证为正数
    unique[0] &= 0x7F;
    hex::encode_upper(unique)
}

/// 计算 SHA-256 指纹
fn fingerprint(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// 生成占位 PEM（显式标注 mock，避免被误认为真实证书）
fn pseudo_pem(label: &str, body: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    let encoded = STANDARD.encode(body.as_bytes());
    format!(
        "-----BEGIN {}-----\n{}\n-----END {}-----",
        label, encoded, label
    )
}

/// 默认证书有效期（天），供外部引用
pub const DEFAULT_VALIDITY_DAYS: u32 = DEFAULT_CERT_VALIDITY_DAYS;
