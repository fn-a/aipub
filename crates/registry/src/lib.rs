//! # Registry - 智能体注册服务
//!
//! 基于 ATR (Agent Trust Registry) 协议实现的智能体注册服务。
//!
//! ## 功能特性
//!
//! - 智能体注册和注销
//! - ACS 能力描述管理
//! - AIC 分配（本体 AIC / 实体 AIC）
//! - EAB 凭据签发与一次性消费
//! - 变更流（DSP），供发现服务增量同步
//!
//! ## 端到端用法
//!
//! ```rust,no_run
//! use registry::{
//!     MemoryEabManager, MemoryRegistryStorage, RegistryConfig, RegistryService,
//!     RegistryServiceImpl,
//! };
//! use protocols::acs::AgentCapabilitySpec;
//!
//! # async fn demo(acs: AgentCapabilitySpec) {
//! let service = RegistryServiceImpl::new(
//!     RegistryConfig::default(),
//!     Box::new(MemoryRegistryStorage::new()),
//!     Some(Box::new(MemoryEabManager::new())),
//! );
//!
//! // 注册（默认要求 EAB 时请改用 register_with_eab）
//! let aic = service.register(acs).await.unwrap();
//! # let _ = aic;
//! # }
//! ```

use thiserror::Error;

/// 变更流类型（定义于 `protocols::dsp`，此处重新导出便于使用）
pub use protocols::dsp::{ChangeEntry, ChangeFeed, ChangeOp, ChangeType};
use protocols::aic::*;
use protocols::PROTOCOL_VERSION;

pub mod service;
pub mod storage;
pub mod eab;

pub use service::*;
pub use storage::*;
pub use eab::*;

/// Registry 服务错误类型
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("注册错误: {0}")]
    RegistrationError(String),
    
    #[error("验证错误: {0}")]
    VerificationError(String),
    
    #[error("存储错误: {0}")]
    StorageError(String),
    
    #[error("无效的 AIC: {0}")]
    InvalidAic(String),
    
    #[error("无效的 ACS: {0}")]
    InvalidAcs(String),
    
    #[error("EAB 凭据错误: {0}")]
    EabError(String),
    
    #[error("智能体不存在: {0}")]
    AgentNotFound(String),
    
    #[error("智能体已存在: {0}")]
    AlreadyExists(String),
    
    #[error("AIC 生成失败: {0}")]
    AicGenerationFailed(String),
}

/// Registry 服务配置
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// 服务端口
    pub port: u16,
    
    /// 是否启用自动审批
    pub auto_approval: bool,
    
    /// 是否需要 EAB 凭据
    pub require_eab: bool,
    
    /// 是否启用 mTLS
    pub enable_mtls: bool,

    /// AIC CRC 计算盐值（十六进制字符串，支持 "0x" 前缀）
    ///
    /// 对应参考实现的环境变量 `AIC_CRC_SALT`。
    pub aic_crc_salt_hex: String,

    /// AIC 第 5 级版本号
    pub aic_protocol_version: String,

    /// AIC 第 6 级 ARSP 序号
    pub aic_arsp_code: String,

    /// AIC 第 7 级 Vendor 序号
    pub aic_vendor_code: String,

    /// 默认 ACS 协议版本
    pub default_protocol_version: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            port: 8081,
            auto_approval: false,
            require_eab: true,
            enable_mtls: true,
            // 参考实现测试环境使用的盐值（ASCII "IOA"）
            aic_crc_salt_hex: "0x00494F41".to_string(),
            aic_protocol_version: AIC_PROTOCOL_VERSION.to_string(),
            aic_arsp_code: DEFAULT_ARSP_CODE.to_string(),
            aic_vendor_code: DEFAULT_VENDOR_CODE.to_string(),
            default_protocol_version: PROTOCOL_VERSION.to_string(),
        }
    }
}

impl RegistryConfig {
    /// 免 EAB 的开发/测试配置
    pub fn without_eab() -> Self {
        Self {
            require_eab: false,
            ..Default::default()
        }
    }
}
