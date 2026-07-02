//! # AIP 智能体互联协议核心库
//!
//! 本库实现了 ACPs (Agent Collaboration Protocols) 智能体协作协议体系的核心协议，
//! 包括智能体身份码 (AIC)、能力描述 (ACS)、交互协议 (AIP) 和发现协议 (ADP)。
//!
//! ## 模块结构
//!
//! - [`aic`]: Agent Identity Code - 智能体身份码
//! - [`acs`]: Agent Capability Specification - 智能体能力描述
//! - [`aip`]: Agent Interaction Protocol - 智能体交互协议
//! - [`adp`]: Agent Discovery Protocol - 智能体发现协议
//!
//! ## 快速开始
//!
//! ```rust
//! use aipub::protocols::aic::validate_aic_format;
//! use aipub::protocols::acs::AgentCapabilitySpec;
//! use aipub::protocols::adp::DiscoveryRequest;
//!
//! // 验证 AIC 格式
//! let aic = "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353";
//! let result = validate_aic_format(aic);
//! ```

pub mod aic;
pub mod acs;
pub mod aip;
pub mod adp;

pub use aic::*;
pub use acs::*;
pub use aip::*;
pub use adp::*;

/// 协议版本
pub const PROTOCOL_VERSION: &str = "02.01";

/// AIC OID 前缀 (由国家 OID 注册中心分配)
pub const AIC_OID_PREFIX: &str = "1.2.156.3088";
