//! AIC 完整验证器（包含 CRC 校验）

use super::{validate_aic_format, AicInfo};
use sha2::{Sha256, Digest};

/// AIC 完整验证器（包含 CRC 校验）
///
/// CRC 校验需要 SALT，SALT 由各 ARSP 内部维护。
///
/// # 示例
///
/// ```rust
/// use aipub::protocols::aic::AicValidator;
///
/// let validator = AicValidator::new(Some(b"\x12\x34".to_vec()));
/// let aic = "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353";
///
/// if validator.validate(aic) {
///     println!("AIC 有效");
/// } else {
///     println!("AIC 无效: {}", validator.last_error().unwrap());
/// }
/// ```
pub struct AicValidator {
    /// CRC 计算的盐值
    salt: Option<Vec<u8>>,
    
    /// 最后一次验证的错误信息
    last_error: Option<String>,
}

impl AicValidator {
    /// 创建新的验证器
    ///
    /// # 参数
    ///
    /// - `salt`: CRC 计算的盐值
    ///   - `Some(bytes)`: 使用指定的盐值
    ///   - `None`: 不验证 CRC，仅做格式验证
    pub fn new(salt: Option<Vec<u8>>) -> Self {
        Self {
            salt,
            last_error: None,
        }
    }
    
    /// 从十六进制字符串创建验证器
    ///
    /// # 参数
    ///
    /// - `hex_str`: 十六进制字符串（支持 "0x" 前缀）
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let hex_str = hex_str.trim();
        let hex_str = if hex_str.to_lowercase().starts_with("0x") {
            &hex_str[2..]
        } else {
            hex_str
        };
        
        let hex_str = if hex_str.len() % 2 != 0 {
            format!("0{}", hex_str)
        } else {
            hex_str.to_string()
        };
        
        let bytes = hex::decode(&hex_str)
            .map_err(|e| format!("解析十六进制失败: {}", e))?;
        
        Ok(Self::new(Some(bytes)))
    }
    
    /// 是否配置了 SALT
    pub fn has_salt(&self) -> bool {
        self.salt.is_some()
    }
    
    /// 获取最后一次验证的错误信息
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
    
    /// 计算 CRC 校验码
    ///
    /// # 参数
    ///
    /// - `body_1_9`: AIC 的 1-9 级（本体码）
    ///
    /// # 返回
    ///
    /// 4 位 Base36 校验码，未配置 SALT 时返回 None
    pub fn calculate_checksum(&self, body_1_9: &str) -> Option<String> {
        let salt = self.salt.as_ref()?;
        
        let normalized = body_1_9
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();
        
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hasher.update(salt);
        let hash = hasher.finalize();
        
        // 取前 2 字节作为 CRC
        let crc = u16::from_be_bytes([hash[0], hash[1]]);
        
        // 转换为 Base36 (4位)
        Some(super::utils::base36_encode(crc as u64, 4))
    }
    
    /// 验证 AIC
    ///
    /// # 参数
    ///
    /// - `aic`: 待验证的 AIC 字符串
    /// - `require_crc`: 是否必须验证 CRC
    ///   - `true`: 必须验证 CRC（需要配置 SALT）
    ///   - `false`: 如果有 SALT 则验证 CRC，否则仅验证格式
    ///
    /// # 返回
    ///
    /// AIC 是否有效
    pub fn validate(&mut self, aic: &str, require_crc: bool) -> bool {
        self.last_error = None;
        
        // 格式验证
        let info = match validate_aic_format(aic) {
            Ok(info) => info,
            Err(e) => {
                self.last_error = Some(e);
                return false;
            }
        };
        
        // CRC 验证
        match &self.salt {
            None => {
                if require_crc {
                    self.last_error = Some("未配置 SALT，无法验证 CRC".to_string());
                    return false;
                }
                // 无 SALT 且不要求 CRC，格式正确即通过
                true
            }
            Some(_) => {
                // 有 SALT，验证 CRC
                let expected_crc = match self.calculate_checksum(&info.body()) {
                    Some(crc) => crc,
                    None => {
                        self.last_error = Some("计算 CRC 失败".to_string());
                        return false;
                    }
                };
                
                if expected_crc != info.checksum {
                    self.last_error = Some(format!(
                        "CRC 校验失败，期望 {}，实际 {}",
                        expected_crc, info.checksum
                    ));
                    return false;
                }
                
                true
            }
        }
    }
    
    /// 验证 AIC 并返回详细信息
    ///
    /// # 返回
    ///
    /// (is_valid, error_message, aic_info) 元组
    pub fn validate_with_detail(&mut self, aic: &str) -> (bool, Option<String>, Option<AicInfo>) {
        let is_valid = self.validate(aic, false);
        let info = if is_valid || self.last_error.is_some() {
            super::parse_aic(aic)
        } else {
            None
        };
        
        (is_valid, self.last_error.clone(), info)
    }
}

impl Default for AicValidator {
    fn default() -> Self {
        Self::new(None)
    }
}