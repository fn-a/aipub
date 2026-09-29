//! EAB (External Account Binding) 凭据管理
//!
//! 对应参考实现中注册服务的 EAB 签发/消费（`/acps-atr-v2/eab/{aic}` 与
//! `/internal/eab/consume`）以及 CA 侧的 HMAC 校验：
//!
//! - 注册服务负责**签发**与**一次性消费**，自己不校验 HMAC；
//! - CA 用消费得到的 `macKey` 以 HMAC-SHA256 校验 ACME `newAccount` 里的
//!   `externalAccountBinding` 内部 JWS。
//!
//! 具体的 HMAC 算法定义在 [`protocols::atr`]，注册服务与证书服务共用。

use std::sync::Mutex;
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use protocols::atr::{generate_mac_key, constant_time_eq, EAB_TTL_HOURS};
use crate::RegistryError;

/// EAB 凭据默认有效期（小时）
pub const DEFAULT_EAB_TTL_HOURS: i64 = EAB_TTL_HOURS;

/// EAB 凭据
#[derive(Debug, Clone, PartialEq)]
pub struct EabCredential {
    /// 凭据 ID（ACME 内层 JWS 的 `kid`）
    pub key_id: String,

    /// HMAC 密钥（base64url 无填充）
    pub mac_key: String,

    /// 账号标识
    pub account_id: String,

    /// 绑定的智能体 AIC
    pub aic: String,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 过期时间
    pub expires_at: DateTime<Utc>,

    /// 是否已被消费
    pub is_consumed: bool,

    /// 消费时间
    pub consumed_at: Option<DateTime<Utc>>,
}

impl EabCredential {
    /// 是否已过期
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// 是否可用（未消费且未过期）
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        !self.is_consumed && !self.is_expired(now)
    }
}

/// EAB 管理器 trait
#[async_trait]
pub trait EabManager: Send + Sync {
    /// 创建 EAB 凭据
    async fn create(&self, account_id: &str, aic: &str) -> Result<EabCredential, RegistryError>;

    /// 校验 EAB 凭据（键值匹配、未消费、未过期）
    async fn verify(&self, key_id: &str, hmac_key: &str) -> Result<bool, RegistryError>;

    /// 一次性消费 EAB 凭据，成功返回 macKey 与绑定的 AIC
    async fn consume(&self, key_id: &str) -> Result<EabCredential, RegistryError>;

    /// 撤销 EAB 凭据
    async fn revoke(&self, key_id: &str) -> Result<(), RegistryError>;
}

/// 内存 EAB 管理器
#[derive(Debug)]
pub struct MemoryEabManager {
    credentials: Mutex<HashMap<String, EabCredential>>,
    ttl_hours: i64,
}

impl MemoryEabManager {
    /// 创建管理器（使用默认 24 小时有效期）
    pub fn new() -> Self {
        Self {
            credentials: Mutex::new(HashMap::new()),
            ttl_hours: DEFAULT_EAB_TTL_HOURS,
        }
    }

    /// 自定义有效期（小时）
    ///
    /// 传入 `0` 表示凭据立即过期，便于演练/验证过期分支。
    pub fn with_ttl_hours(mut self, hours: i64) -> Self {
        self.ttl_hours = hours.max(0);
        self
    }

    /// 当前凭据数量
    pub fn len(&self) -> usize {
        self.credentials.lock().expect("EAB 锁未中毒").len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 读取凭据
    pub fn get(&self, key_id: &str) -> Option<EabCredential> {
        self.credentials
            .lock()
            .expect("EAB 锁未中毒")
            .get(key_id)
            .cloned()
    }
}

impl Default for MemoryEabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EabManager for MemoryEabManager {
    async fn create(&self, account_id: &str, aic: &str) -> Result<EabCredential, RegistryError> {
        let now = Utc::now();
        let credential = EabCredential {
            key_id: Uuid::new_v4().simple().to_string(),
            mac_key: generate_mac_key(),
            account_id: account_id.to_string(),
            aic: aic.to_string(),
            created_at: now,
            expires_at: now + Duration::hours(self.ttl_hours),
            is_consumed: false,
            consumed_at: None,
        };

        self.credentials
            .lock()
            .expect("EAB 锁未中毒")
            .insert(credential.key_id.clone(), credential.clone());

        Ok(credential)
    }

    async fn verify(&self, key_id: &str, hmac_key: &str) -> Result<bool, RegistryError> {
        let credentials = self.credentials.lock().expect("EAB 锁未中毒");
        let Some(credential) = credentials.get(key_id) else {
            return Ok(false);
        };

        Ok(credential.is_usable(Utc::now())
            && constant_time_eq(
                credential.mac_key.as_bytes(),
                hmac_key.as_bytes(),
            ))
    }

    async fn consume(&self, key_id: &str) -> Result<EabCredential, RegistryError> {
        let mut credentials = self.credentials.lock().expect("EAB 锁未中毒");
        let credential = credentials
            .get_mut(key_id)
            .ok_or_else(|| RegistryError::EabError(format!("EAB_NOT_FOUND: {}", key_id)))?;

        let now = Utc::now();
        if credential.is_consumed {
            return Err(RegistryError::EabError(format!(
                "EAB_ALREADY_CONSUMED: {}",
                key_id
            )));
        }
        if credential.is_expired(now) {
            return Err(RegistryError::EabError(format!("EAB_EXPIRED: {}", key_id)));
        }

        credential.is_consumed = true;
        credential.consumed_at = Some(now);
        Ok(credential.clone())
    }

    async fn revoke(&self, key_id: &str) -> Result<(), RegistryError> {
        let mut credentials = self.credentials.lock().expect("EAB 锁未中毒");
        if credentials.remove(key_id).is_none() {
            return Err(RegistryError::EabError(format!("EAB_NOT_FOUND: {}", key_id)));
        }
        Ok(())
    }
}
