//! AIC 工具函数

use std::collections::HashMap;
use regex::Regex;
use super::constants::BASE36_ALPHABET;

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

/// CRC-16/CCITT-FALSE 校验算法
///
/// 参数：`poly=0x1021`、`init=0xFFFF`、`refin/refout=false`、`xorout=0x0000`。
///
/// AIC 第 10 级校验码即由本体码（1-9 级）的 ASCII 字节序列追加 SALT 后，
/// 经本算法计算，再以 Base36 编码为固定 4 位。
///
/// # 示例
///
/// ```rust
/// use protocols::aic::crc16_ccitt_false;
///
/// // CRC-16/CCITT-FALSE("123456789") == 0x29B1
/// assert_eq!(crc16_ccitt_false(b"123456789"), 0x29B1);
/// ```
pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// 将 u128 编码为固定长度 Base36 字符串（超出长度时截取低位，不足左侧补 0）
pub fn base36_encode_u128(num: u128, length: usize) -> String {
    if length == 0 {
        return base36_encode(num as u64, 0);
    }

    let mut digits = vec![0u8; length];
    let mut n = num;
    let modulus = 36u128;
    for i in (0..length).rev() {
        digits[i] = (n % modulus) as u8;
        n /= modulus;
    }

    digits
        .into_iter()
        .map(|d| BASE36_ALPHABET.chars().nth(d as usize).unwrap())
        .collect()
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
/// use protocols::aic::base36_encode;
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
/// use protocols::aic::base36_decode;
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
