//! # AIP 智能体互联协议
//!
//! 基于 ACPs (Agent Collaboration Protocols) 规范的 Rust 实现。
//!
//! ## 项目介绍
//!
//! 本项目实现了智能体互联协议体系，包括：
//!
//! - **AIC**: Agent Identity Code - 智能体身份码
//! - **ACS**: Agent Capability Specification - 智能体能力描述
//! - **AIP**: Agent Interaction Protocol - 智能体交互协议
//! - **ADP**: Agent Discovery Protocol - 智能体发现协议
//!
//! 以及配套的服务组件：
//!
//! - **Discovery**: 智能体发现服务
//! - **Registry**: 智能体注册服务
//! - **Certifier**: 证书服务 (CA)
//! - **Authorize**: 鉴权服务
//!
//! ## 快速开始
//!
//! ```rust
//! use aipub::{
//!     // 协议相关
//!     validate_aic_format, parse_aic, AicInfo,
//!     AgentCapabilitySpec, TaskCommand, TaskState,
//!     DiscoveryRequest, DiscoveryResponse,
//! };
//!
//! // 验证 AIC
//! let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
//! if let Ok(info) = validate_aic_format(aic) {
//!     println!("AIC 有效: {}", info);
//! }
//!
//! // 创建发现请求
//! let request = DiscoveryRequest::new()
//!     .with_query("我需要一个北京美食推荐的智能体")
//!     .with_limit(10);
//! ```

// 重新导出所有子 crate
pub use protocols;
pub use discovery;
pub use registry;
pub use certifier;
pub use authorize;

// 重新导出常用类型，提供便捷访问
pub use protocols::{
    // AIC 相关
    aic::{validate_aic_format, parse_aic, AicInfo, AicValidator},
    
    // ACS 相关
    acs::{AgentCapabilitySpec, AgentProvider, AgentCapabilities, AgentSkill},
    
    // AIP 相关
    aip::{TaskCommand, TaskResult, TaskState, TaskCommandType},
    
    // ADP 相关
    adp::{DiscoveryRequest, DiscoveryResponse, DiscoveryResult, DiscoveryFilter},
};

/// 协议版本
pub const PROTOCOL_VERSION: &str = "02.01";

/// AIC OID 前缀
pub const AIC_OID_PREFIX: &str = "1.2.156.3088";

/// 项目版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");