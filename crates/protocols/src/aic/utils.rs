//! AIC 工具函数

use super::constants::BASE36_ALPHABET;
use regex::Regex;
use std::collections::HashMap;

lazy_static::lazy_static! {
    /// Base36 字符到索引的映射
    static ref BASE36_INDEX: HashMap<char, usize> = {
        BASE36_ALPHABET
            .chars()
            .enumerate()
            .map(|(i, ch)| (ch, i))
            .collect()
    };
    
    /// Base36 正则表达式
    static ref RE_BASE36: Regex = Regex::new(r"^[0-9A-Z]+$").unwrap();
}

/// 规范化 AIC：去除空白并转为大写
pub fn normalize_aic(aic: &str) -> String {
    aic.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

/// 判断字符串是否为有效的 Base36
pub fn is_base36(s: &str) -> bool {
    RE_BASE36.is_match(s)
}

/// 将非负整数编码为 Base36 字符串（大写）
///
/// # 参数
///
/// - `num`: 非负整数
/// - `length`: 目标长度，不足左侧补 0，0 表示不补零
///
/// # 返回
///
/// Base36 编码字符串
///
/// # 错误
///
/// 如果 num 为负数，返回 None
///
/// # 示例
///
/// ```rust
/// use aipub::protocols::aic::base36_encode;
///
/// assert_eq!(base36_encode(0, 0), "0");
/// assert_eq!(base36_encode(35, 0), "Z");
/// assert_eq!(base36_encode(36, 0), "10");
/// assert_eq!(base36_encode(12345, 6), "0009IX");
/// ```
pub fn base36_encode(num: u64, length: usize) -> String {
    if num == 0 {
        return if length > 0 {
            "0".repeat(length)
        } else {
            "0".to_string()
        };
    }
    
    let mut n = num;
    let mut digits = Vec::new();
    
    while n > 0 {
        let rem = (n % 36) as usize;
        digits.push(BASE36_ALPHABET.chars().nth(rem).unwrap());
        n /= 36;
    }
    
    digits.reverse();
    let encoded: String = digits.into_iter().collect();
    
    if length > 0 && encoded.len() < length {
        format!("{}{}", "0".repeat(length - encoded.len()), encoded)
    } else {
        encoded
    }
}

/// 将 Base36 字符串解码为整数
///
/// # 参数
///
/// - `s`: Base36 字符串（大小写不敏感）
///
/// # 返回
///
/// - `Ok(u64)`: 解码后的整数
/// - `Err(String)`: 包含非 Base36 字符
///
/// # 示例
///
/// ```rust
/// use aipub::protocols::aic::base36_decode;
///
/// assert_eq!(base36_decode("0").unwrap(), 0);
/// assert_eq!(base36_decode("Z").unwrap(), 35);
/// assert_eq!(base36_decode("10").unwrap(), 36);
/// ```
pub fn base36_decode(s: &str) -> Result<u64, String> {
    if s.trim().is_empty() {
        return Ok(0);
    }
    
    let mut val: u64 = 0;
    for ch in s.trim().to_uppercase().chars() {
        if let Some(&idx) = BASE36_INDEX.get(&ch) {
            val = val * 36 + idx as u64;
        } else {
            return Err(format!("非 Base36 字符: {}", ch));
        }
    }
    
    Ok(val)
}
