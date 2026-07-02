//! AIC 常量定义

/// Base36 字母表（0-9, A-Z）
pub const BASE36_ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// AIC OID 前缀 (由国家 OID 注册中心分配)
pub const AIC_OID_PREFIX: &str = "1.2.156.3088";

/// AIC 各段的长度约束
pub const SEGMENT_CONSTRAINTS: [(usize, usize); 6] = [
    (1, 1),   // Version (第5级)
    (1, 6),   // ARSP (第6级)
    (1, 6),   // Vendor (第7级)
    (1, 9),   // Ontology SN (第8级)
    (1, 9),   // Instance SN (第9级)
    (4, 4),   // CRC (第10级)
];
