//! # AIC - Agent Identity Code (智能体身份码)
//!
//! 基于 ACPs-spec-AIC-v02.01 规范实现的智能体身份码工具模块。
//!
//! ## AIC 结构说明
//!
//! AIC 采用 10 级 OID 结构，以 '.' 分隔：
//!
//! ```text
//! 1-4:   OID 前缀 (1.2.156.3088)
//! 5:     版本号 (Base36, 1位)
//! 6:     ARSP 注册服务商序号 (Base36, 1-6位)
//! 7:     Vendor 供应商序号 (Base36, 1-6位)
//! 8:     Ontology SN 本体序列号 (Base36, 1-9位)
//! 9:     Instance SN 实例序列号 (Base36, 1-9位, 全0表示本体AIC)
//! 10:    CRC校验码 (Base36, 固定4位)
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use protocols::aic::{validate_aic_format, parse_aic, AicInfo};
//!
//! // 基础格式验证
//! let aic = "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353";
//! if let Ok(info) = validate_aic_format(aic) {
//!     println!("AIC 有效");
//!     println!("是否为本体: {}", info.is_ontology());
//! }
//!
//! // 解析 AIC
//! if let Some(info) = parse_aic(aic) {
//!     println!("ARSP: {}", info.arsp);
//!     println!("Vendor: {}", info.vendor);
//! }
//! ```

mod validator;
mod utils;
mod constants;

pub use validator::*;
pub use utils::*;
pub use constants::*;

use serde::{Deserialize, Serialize};
use std::fmt;

/// AIC 解析结果
///
/// 包含 AIC 的各个组成部分和元信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AicInfo {
    /// 原始 AIC 字符串
    pub raw: String,
    
    /// 规范化后的 AIC（大写，无空白）
    pub normalized: String,
    
    /// OID 前缀 (1-4级)
    pub prefix: String,
    
    /// 版本号 (第5级)
    pub version: String,
    
    /// 注册服务商序号 (第6级)
    pub arsp: String,
    
    /// 供应商序号 (第7级)
    pub vendor: String,
    
    /// 本体序列号 (第8级)
    pub ontology_serial: String,
    
    /// 实例序列号 (第9级)
    pub instance_serial: String,
    
    /// CRC 校验码 (第10级)
    pub checksum: String,
}

impl AicInfo {
    /// 判断是否为本体 AIC
    ///
    /// 本体 AIC 的实例序列号全为 0
    pub fn is_ontology(&self) -> bool {
        self.instance_serial.chars().all(|c| c == '0')
    }
    
    /// 判断是否为实体 AIC
    ///
    /// 实体 AIC 的实例序列号非全 0
    pub fn is_entity(&self) -> bool {
        !self.is_ontology()
    }
    
    /// 获取本体码（1-9级，用于 CRC 计算）
    pub fn body(&self) -> String {
        format!(
            "{}.{}.{}.{}.{}.{}",
            self.prefix,
            self.version,
            self.arsp,
            self.vendor,
            self.ontology_serial,
            self.instance_serial
        )
    }
    
    /// 获取本体前缀（1-8级）
    ///
    /// 可用于匹配同一本体的所有实体
    pub fn ontology_prefix(&self) -> String {
        format!(
            "{}.{}.{}.{}.{}",
            self.prefix,
            self.version,
            self.arsp,
            self.vendor,
            self.ontology_serial
        )
    }
}

impl fmt::Display for AicInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AicInfo {{ aic: {}, is_ontology: {}, arsp: {}, vendor: {} }}",
            self.normalized,
            self.is_ontology(),
            self.arsp,
            self.vendor
        )
    }
}

/// 验证 AIC 格式是否符合规范（不验证 CRC 校验码）
///
/// 此函数不需要 SALT，适用于基础验证。
///
/// # 参数
///
/// - `aic`: 待验证的 AIC 字符串
///
/// # 返回
///
/// - `Ok(AicInfo)`: 格式有效，返回解析后的信息
/// - `Err(String)`: 格式无效，返回错误描述
///
/// # 示例
///
/// ```rust
/// use protocols::aic::validate_aic_format;
///
/// let aic = "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353";
/// match validate_aic_format(aic) {
///     Ok(info) => println!("AIC 有效: {}", info),
///     Err(e) => println!("AIC 无效: {}", e),
/// }
/// ```
pub fn validate_aic_format(aic: &str) -> Result<AicInfo, String> {
    if aic.trim().is_empty() {
        return Err("AIC 不能为空".to_string());
    }
    
    let normalized = normalize_aic(aic);
    let parts: Vec<&str> = normalized.split('.').collect();
    
    if parts.len() != 10 {
        return Err(format!("AIC 必须包含 10 段，当前为 {} 段", parts.len()));
    }
    
    // 验证前缀（1-4级）
    let prefix = format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);
    if prefix != AIC_OID_PREFIX {
        return Err(format!(
            "前缀不匹配，期望 {}，实际 {}",
            AIC_OID_PREFIX, prefix
        ));
    }
    
    // 验证 1-4 级为纯数字
    for i in 0..4 {
        if !parts[i].chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("第 {} 级必须为纯数字，实际为 '{}'", i + 1, parts[i]));
        }
    }
    
    // 验证 5-10 级为 Base36 且符合长度约束
    let constraints = [
        (1, 1),   // Version (第5级)
        (1, 6),   // ARSP (第6级)
        (1, 6),   // Vendor (第7级)
        (1, 9),   // Ontology SN (第8级)
        (1, 9),   // Instance SN (第9级)
        (4, 4),   // CRC (第10级)
    ];
    
    for (i, (min_len, max_len)) in constraints.iter().enumerate() {
        let level = i + 5;
        let seg = parts[level - 1];
        
        if !is_base36(seg) {
            return Err(format!("第 {} 级必须为 Base36 字符，实际为 '{}'", level, seg));
        }
        
        let len = seg.len();
        if len < *min_len || len > *max_len {
            return Err(format!(
                "第 {} 级长度必须在 {}-{} 之间，实际为 {}",
                level, min_len, max_len, len
            ));
        }
    }
    
    let result = AicInfo {
        raw: aic.to_string(),
        normalized: normalized.clone(),
        prefix,
        version: parts[4].to_string(),
        arsp: parts[5].to_string(),
        vendor: parts[6].to_string(),
        ontology_serial: parts[7].to_string(),
        instance_serial: parts[8].to_string(),
        checksum: parts[9].to_string(),
    };
    
    Ok(result)
}

/// 解析 AIC 字符串，返回结构化信息
///
/// 注意：此函数不验证 CRC 校验码，仅做格式解析。
///
/// # 参数
///
/// - `aic`: AIC 字符串
///
/// # 返回
///
/// - `Some(AicInfo)`: 解析成功
/// - `None`: 解析失败
pub fn parse_aic(aic: &str) -> Option<AicInfo> {
    validate_aic_format(aic).ok()
}

/// 获取 AIC 指定级别的内容
///
/// # 参数
///
/// - `aic`: AIC 字符串
/// - `level`: 级别（1-10）
///
/// # 返回
///
/// - `Some(String)`: 指定级别的内容
/// - `None`: 失败
pub fn get_aic_segment(aic: &str, level: usize) -> Option<String> {
    if level < 1 || level > 10 {
        return None;
    }
    
    let normalized = normalize_aic(aic);
    let parts: Vec<&str> = normalized.split('.').collect();
    
    if parts.len() != 10 {
        return None;
    }
    
    Some(parts[level - 1].to_string())
}

/// 判断是否为本体 AIC
///
/// 本体 AIC 的第 9 级（实例序列号）全为 0。
pub fn is_ontology_aic(aic: &str) -> bool {
    parse_aic(aic).map(|info| info.is_ontology()).unwrap_or(false)
}

/// 判断是否为实体 AIC
///
/// 实体 AIC 的第 9 级（实例序列号）非全 0。
pub fn is_entity_aic(aic: &str) -> bool {
    parse_aic(aic).map(|info| info.is_entity()).unwrap_or(false)
}

/// 从 AIC 提取本体前缀（1-8级）
///
/// 可用于数据库 LIKE 查询，匹配同一本体的所有实体。
pub fn get_ontology_prefix_from_aic(aic: &str) -> Option<String> {
    parse_aic(aic).map(|info| info.ontology_prefix())
}