//! Registry 服务核心实现
//!
//! 对齐参考实现 `registry-server` 的审批侧行为：
//!
//! 1. 校验/分配 AIC（本体 AIC 第 9 级全 0，实体 AIC 非全 0）；
//! 2. 校验 ACS（必填字段、mutualTLS 端点、URL scheme 与 transport 匹配等）；
//! 3. 需要时校验 EAB 凭据；
//! 4. 写入身份元数据（`aic`/`active`/`lastModifiedTime`）并替换 `{AIC}` 占位符；
//! 5. 落库并追加变更流记录，供发现服务增量同步。

use std::collections::HashMap;
use async_trait::async_trait;
use chrono::{DateTime, Utc, SecondsFormat};
use tokio::sync::Mutex;
use serde_json::to_value;
use protocols::acs::AgentCapabilitySpec;
use protocols::aic::{parse_hex_salt, AicGenerator, AicValidator};
use protocols::dsp::{ChangeEntry, ChangeFeed, ChangeOp};
use crate::{RegistryConfig, RegistryError};

/// Registry 服务 trait
#[async_trait]
pub trait RegistryService: Send + Sync {
    /// 注册智能体
    async fn register(&self, acs: AgentCapabilitySpec) -> Result<String, RegistryError>;
    
    /// 注销智能体
    async fn unregister(&self, aic: &str) -> Result<(), RegistryError>;
    
    /// 更新智能体 ACS
    async fn update(&self, aic: &str, acs: AgentCapabilitySpec) -> Result<(), RegistryError>;
    
    /// 获取智能体 ACS
    async fn get(&self, aic: &str) -> Result<Option<AgentCapabilitySpec>, RegistryError>;
    
    /// 激活/停用智能体
    async fn set_active(&self, aic: &str, active: bool) -> Result<(), RegistryError>;
}

/// 智能体注册元数据
#[derive(Debug, Clone)]
pub struct AgentMeta {
    /// ACS 版本号（每次改写自增）
    pub version: u64,

    /// 最近一次变更序号
    pub seq: u64,

    /// ACS 内容哈希
    pub hash: String,

    /// 最近更新时间
    pub updated_at: DateTime<Utc>,

    /// 是否为本体 AIC
    pub is_ontology: bool,
}

/// Registry 服务实现
pub struct RegistryServiceImpl {
    config: RegistryConfig,
    storage: Box<dyn crate::RegistryStorage>,
    eab_manager: Option<Box<dyn crate::EabManager>>,
    aic_generator: AicGenerator,
    changes: Mutex<ChangeFeed>,
    meta: Mutex<HashMap<String, AgentMeta>>,
}

impl RegistryServiceImpl {
    /// 创建服务实例
    pub fn new(
        config: RegistryConfig,
        storage: Box<dyn crate::RegistryStorage>,
        eab_manager: Option<Box<dyn crate::EabManager>>,
    ) -> Self {
        let aic_generator = AicGenerator::new(parse_hex_salt(&config.aic_crc_salt_hex));

        Self {
            config,
            storage,
            eab_manager,
            aic_generator,
            changes: Mutex::new(ChangeFeed::new()),
            meta: Mutex::new(HashMap::new()),
        }
    }

    /// 当前配置
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// AIC 生成器
    pub fn aic_generator(&self) -> &AicGenerator {
        &self.aic_generator
    }

    /// 列出全部智能体
    pub async fn list(&self) -> Result<Vec<AgentCapabilitySpec>, RegistryError> {
        self.storage.list().await
    }

    /// 读取注册元数据
    pub async fn meta(&self, aic: &str) -> Option<AgentMeta> {
        self.meta.lock().await.get(aic).cloned()
    }

    /// 拉取指定序号之后的变更
    pub async fn changes_since(
        &self,
        seq: u64,
        limit: usize,
    ) -> Result<Vec<ChangeEntry>, RegistryError> {
        Ok(self.changes.lock().await.changes_since(seq, limit))
    }

    /// 全量快照
    pub async fn snapshot(&self) -> Result<Vec<ChangeEntry>, RegistryError> {
        Ok(self.changes.lock().await.snapshot())
    }

    /// 最新变更序号
    pub async fn latest_seq(&self) -> u64 {
        self.changes.lock().await.latest_seq()
    }

    /// 签发 EAB 凭据（对应 `POST /acps-atr-v2/eab/{aic}`）
    pub async fn mint_eab(&self, aic: &str) -> Result<crate::EabCredential, RegistryError> {
        let Some(manager) = self.eab_manager.as_ref() else {
            return Err(RegistryError::EabError("未启用 EAB 支持".to_string()));
        };

        // 仅已注册且激活的智能体可以签发
        match self.storage.get(aic).await? {
            Some(acs) if acs.active => {}
            Some(_) => return Err(RegistryError::EabError(format!("AIC_INACTIVE: {}", aic))),
            None => return Err(RegistryError::AgentNotFound(aic.to_string())),
        }

        manager.create(aic, aic).await
    }

    /// 一次性消费 EAB 凭据（对应 `POST /internal/eab/consume`）
    pub async fn consume_eab(
        &self,
        key_id: &str,
    ) -> Result<crate::EabCredential, RegistryError> {
        let Some(manager) = self.eab_manager.as_ref() else {
            return Err(RegistryError::EabError("未启用 EAB 支持".to_string()));
        };
        manager.consume(key_id).await
    }

    /// 带 EAB 校验的注册
    ///
    /// `eab` 为 `(key_id, mac_key)`；`is_ontology` 为真时生成本体 AIC。
    pub async fn register_with_eab(
        &self,
        acs: AgentCapabilitySpec,
        eab: Option<(&str, &str)>,
        is_ontology: bool,
    ) -> Result<String, RegistryError> {
        if self.config.require_eab {
            let Some((key_id, mac_key)) = eab else {
                return Err(RegistryError::EabError(
                    "注册需要提供 EAB 凭据".to_string(),
                ));
            };
            let Some(manager) = self.eab_manager.as_ref() else {
                return Err(RegistryError::EabError("未启用 EAB 支持".to_string()));
            };
            if !manager.verify(key_id, mac_key).await? {
                return Err(RegistryError::EabError(format!(
                    "EAB 凭据校验失败: {}",
                    key_id
                )));
            }
        }

        self.store_acs(acs, is_ontology).await
    }

    /// 由本体 AIC 注册实体（对应 ATR 的 `POST /acps-atr-v2/entity`）
    pub async fn register_entity_from_ontology(
        &self,
        ontology_aic: &str,
        mut entity_acs: AgentCapabilitySpec,
    ) -> Result<String, RegistryError> {
        let ontology = self
            .storage
            .get(ontology_aic)
            .await?
            .ok_or_else(|| RegistryError::AgentNotFound(ontology_aic.to_string()))?;

        if !ontology.active {
            return Err(RegistryError::VerificationError(format!(
                "ONTOLOGY_INACTIVE: {}",
                ontology_aic
            )));
        }

        let entity_aic = self
            .aic_generator
            .entity_from_ontology(ontology_aic)
            .ok_or_else(|| RegistryError::InvalidAic(ontology_aic.to_string()))?;

        // 实体继承本体的能力与技能，仅覆盖端点等差异
        entity_acs.aic = entity_aic.clone();
        self.store_acs(entity_acs, false).await
    }

    /// 校验并落库 ACS，同时追加变更流
    async fn store_acs(
        &self,
        mut acs: AgentCapabilitySpec,
        is_ontology: bool,
    ) -> Result<String, RegistryError> {
        // 1. AIC：为空则分配，非空则完整校验（含 CRC）
        let aic = if acs.aic.trim().is_empty() {
            let generated = if is_ontology {
                self.aic_generator.generate_ontology_aic()
            } else {
                self.aic_generator.generate_entity_aic()
            };
            generated.map_err(RegistryError::AicGenerationFailed)?
        } else {
            self.validate_aic(&acs.aic)?;
            acs.aic.clone()
        };

        // 2. 重复注册检查
        if let Some(existing) = self.storage.get(&aic).await? {
            if existing.active {
                return Err(RegistryError::AlreadyExists(aic));
            }
        }

        // 3. 写入身份元数据并解析 {AIC}
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
        acs.apply_registry_metadata(&aic, true, &now);

        if acs.protocol_version.trim().is_empty() {
            acs.protocol_version = self.config.default_protocol_version.clone();
        }

        // 4. ACS 校验
        acs.validate().map_err(RegistryError::InvalidAcs)?;

        // 5. 提交存储
        let hash = acs.acs_hash().map_err(RegistryError::InvalidAcs)?;
        let version = {
            let mut meta = self.meta.lock().await;
            let version = meta.get(&aic).map(|m| m.version + 1).unwrap_or(1);
            let seq = {
                let mut changes = self.changes.lock().await;
                let payload = to_value(&acs).map_err(|e| {
                    RegistryError::InvalidAcs(format!("ACS 序列化失败: {}", e))
                })?;
                changes.append(ChangeOp::Upsert, &aic, version, Some(payload))
            };

            meta.insert(
                aic.clone(),
                AgentMeta {
                    version,
                    seq,
                    hash,
                    updated_at: Utc::now(),
                    is_ontology,
                },
            );
            version
        };
        let _ = version;

        self.storage.store(acs).await?;
        Ok(aic)
    }

    /// 完整校验 AIC（格式 + CRC）
    fn validate_aic(&self, aic: &str) -> Result<(), RegistryError> {
        let mut validator =
            AicValidator::new(Some(parse_hex_salt(&self.config.aic_crc_salt_hex)));
        if validator.validate(aic, true) {
            Ok(())
        } else {
            Err(RegistryError::InvalidAic(
                validator
                    .last_error()
                    .unwrap_or("AIC 校验失败")
                    .to_string(),
            ))
        }
    }

    /// 刷新 ACS 元数据并追加变更
    async fn refresh(
        &self,
        aic: &str,
        acs: AgentCapabilitySpec,
    ) -> Result<(), RegistryError> {
        let hash = acs.acs_hash().map_err(RegistryError::InvalidAcs)?;
        let mut meta = self.meta.lock().await;
        let version = meta.get(aic).map(|m| m.version + 1).unwrap_or(1);
        let is_ontology = meta.get(aic).map(|m| m.is_ontology).unwrap_or(false);

        let mut changes = self.changes.lock().await;
        let payload = to_value(&acs)
            .map_err(|e| RegistryError::InvalidAcs(format!("ACS 序列化失败: {}", e)))?;
        let seq = changes.append(ChangeOp::Upsert, aic, version, Some(payload));
        drop(changes);

        meta.insert(
            aic.to_string(),
            AgentMeta {
                version,
                seq,
                hash,
                updated_at: Utc::now(),
                is_ontology,
            },
        );
        drop(meta);

        self.storage.update(aic, acs).await
    }
}

#[async_trait]
impl RegistryService for RegistryServiceImpl {
    async fn register(&self, acs: AgentCapabilitySpec) -> Result<String, RegistryError> {
        self.register_with_eab(acs, None, false).await
    }
    
    async fn unregister(&self, aic: &str) -> Result<(), RegistryError> {
        if self.storage.get(aic).await?.is_none() {
            return Err(RegistryError::AgentNotFound(aic.to_string()));
        }

        let version = {
            let mut meta = self.meta.lock().await;
            let version = meta.get(aic).map(|m| m.version + 1).unwrap_or(1);
            let mut changes = self.changes.lock().await;
            changes.append(ChangeOp::Delete, aic, version, None);
            meta.remove(aic);
            version
        };
        let _ = version;

        self.storage.remove(aic).await
    }
    
    async fn update(&self, aic: &str, mut acs: AgentCapabilitySpec) -> Result<(), RegistryError> {
        let existing = self
            .storage
            .get(aic)
            .await?
            .ok_or_else(|| RegistryError::AgentNotFound(aic.to_string()))?;

        // 身份字段由注册服务维护
        acs.aic = existing.aic.clone();
        acs.active = existing.active;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
        acs.last_modified_time = now;
        acs.resolve_aic_placeholder();

        acs.validate().map_err(RegistryError::InvalidAcs)?;
        self.refresh(aic, acs).await
    }
    
    async fn get(&self, aic: &str) -> Result<Option<AgentCapabilitySpec>, RegistryError> {
        self.storage.get(aic).await
    }
    
    async fn set_active(&self, aic: &str, active: bool) -> Result<(), RegistryError> {
        let mut acs = self
            .storage
            .get(aic)
            .await?
            .ok_or_else(|| RegistryError::AgentNotFound(aic.to_string()))?;

        if acs.active == active {
            return Ok(());
        }

        acs.active = active;
        acs.last_modified_time = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
        self.refresh(aic, acs).await
    }
}
