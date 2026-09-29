//! AIC 生成与派生
//!
//! 对齐参考实现 `registry-server/app/utils/aic.py`：
//!
//! - 第 5~9 级由「当年内毫秒时间戳 + 随机盐」派生出的高熵序列经 Base36 编码得到；
//! - 第 10 级由本体码（1-9 级）规范化后的 ASCII 字节序列追加 SALT，
//!   经 CRC-16/CCITT-FALSE 计算后以 Base36 编码为固定 4 位；
//! - 本体 AIC 的第 9 级为全 0；实体 AIC 的第 9 级非全 0。
//!
//! ## 示例
//!
//! ```rust
//! use protocols::aic::{generate_aic, generate_ontology_aic, validate_aic_format};
//!
//! let salt = b"\x00\x49\x4F\x41";
//!
//! // 生成实体 AIC（第 9 级非全 0）
//! let entity = generate_aic(salt).unwrap();
//! assert!(validate_aic_format(&entity).unwrap().is_entity());
//!
//! // 生成本体 AIC（第 9 级全 0）
//! let ontology = generate_ontology_aic(salt).unwrap();
//! assert!(validate_aic_format(&ontology).unwrap().is_ontology());
//! ```

use sha2::{Digest, Sha256};
use uuid::Uuid;
use chrono::{Datelike, FixedOffset, TimeZone, Utc};
use super::constants::*;
use super::utils::{base36_encode_u128, is_base36, normalize_aic};
use super::{validate_aic_format_with_prefix, AicInfo};

/// 序列号用途标签：本体序列号
const KIND_ONTOLOGY: &[u8] = b"ONT";

/// 序列号用途标签：新实体实例序列号
const KIND_INSTANCE: &[u8] = b"INS";

/// 序列号用途标签：由本体派生的实体序列号
const KIND_ENTITY: &[u8] = b"ENT";

/// 使用固定 SALT 计算 AIC 第 10 级校验码（4 位 Base36）
///
/// 输入为本体码（1-9 级）规范化后的 ASCII 字节序列，末尾追加 `salt`。
///
/// # 示例
///
/// ```rust
/// use protocols::aic::calculate_checksum;
///
/// // ACPs-spec-AIC-v02.01 给出的示例（无盐）
/// let checksum = calculate_checksum("1.2.156.1234.1.1.34C2.478BDF.3GF546", b"");
/// assert_eq!(checksum, "0H9T");
/// ```
pub fn calculate_checksum(body_1_9: &str, salt: &[u8]) -> String {
    let normalized = normalize_aic(body_1_9);
    let mut data = normalized.into_bytes();
    data.extend_from_slice(salt);
    let crc = super::utils::crc16_ccitt_false(&data);
    super::utils::base36_encode(crc as u64, 4)
}

/// 生成实体 AIC（第 5~9 级默认值，第 9 级非全 0）
///
/// 等价于参考实现的 `generate_aic()`。
pub fn generate_aic(salt: &[u8]) -> Result<String, String> {
    AicGenerator::new(salt.to_vec()).generate_entity_aic()
}

/// 生成本体 AIC（第 9 级为全 0）
///
/// 等价于参考实现的 `generate_ontology_aic()`。
pub fn generate_ontology_aic(salt: &[u8]) -> Result<String, String> {
    AicGenerator::new(salt.to_vec()).generate_ontology_aic()
}

/// 由实体 AIC 派生对应的本体 AIC（第 9 级置全 0 并重算 CRC）
///
/// 等价于参考实现的 `get_ontology_aic_from_entity()`。
pub fn get_ontology_aic_from_entity(entity_aic: &str, salt: &[u8]) -> Option<String> {
    AicGenerator::new(salt.to_vec()).ontology_from_entity(entity_aic)
}

/// 由本体 AIC 派生新的实体 AIC（保留 1~8 级，重生成第 9 级并重算 CRC）
///
/// 等价于参考实现的 `generate_entity_aic_from_ontology()`。
pub fn generate_entity_aic_from_ontology(ontology_aic: &str, salt: &[u8]) -> Option<String> {
    AicGenerator::new(salt.to_vec()).entity_from_ontology(ontology_aic)
}

/// 取得 AIC 的 1-9 级拼接串（本体码）
pub fn get_aic_body(aic: &str) -> Option<String> {
    super::parse_aic(aic).map(|info| info.body())
}

/// 取得用于数据库前缀查询的派生实体前缀（1-8 级 + '.'）
///
/// 等价于参考实现的 `get_derived_entity_like_prefix()`。
pub fn get_derived_entity_like_prefix(aic: &str) -> Option<String> {
    super::parse_aic(aic).map(|info| format!("{}.", info.ontology_prefix()))
}

/// 取得第 9 级实例序列号
pub fn get_instance_serial(aic: &str) -> Option<String> {
    super::get_aic_segment(aic, 9)
}

/// AIC 生成器
///
/// 持有 CRC 计算所需的 SALT 以及第 5~7 级的编码，用于批量生成同一 ARSP
/// 下的本体 AIC 与实体 AIC。
///
/// # 示例
///
/// ```rust
/// use protocols::aic::{validate_aic_format, AicGenerator};
///
/// let generator = AicGenerator::new(b"\x00\x49\x4F\x41".to_vec());
/// let ontology = generator.generate_ontology_aic().unwrap();
/// let entity = generator.entity_from_ontology(&ontology).unwrap();
///
/// assert!(validate_aic_format(&ontology).unwrap().is_ontology());
/// assert!(validate_aic_format(&entity).unwrap().is_entity());
/// assert_ne!(ontology, entity);
/// ```
#[derive(Debug, Clone)]
pub struct AicGenerator {
    /// CRC 计算盐值
    salt: Vec<u8>,

    /// 第 5 级版本号
    protocol_version: String,

    /// 第 6 级 ARSP 序号
    arsp: String,

    /// 第 7 级 Vendor 序号
    vendor: String,

    /// 第 8 级本体序列号长度
    ontology_serial_len: usize,

    /// 第 9 级实例序列号长度
    instance_serial_len: usize,
}

impl AicGenerator {
    /// 使用默认第 5~7 级编码创建生成器
    pub fn new(salt: Vec<u8>) -> Self {
        Self {
            salt,
            protocol_version: AIC_PROTOCOL_VERSION.to_string(),
            arsp: DEFAULT_ARSP_CODE.to_string(),
            vendor: DEFAULT_VENDOR_CODE.to_string(),
            ontology_serial_len: DEFAULT_ONTOLOGY_SERIAL_LEN,
            instance_serial_len: DEFAULT_INSTANCE_SERIAL_LEN,
        }
    }

    /// 从十六进制字符串创建生成器（支持 "0x" 前缀）
    pub fn from_hex_salt(hex_str: &str) -> Result<Self, String> {
        Ok(Self::new(parse_hex_salt(hex_str)))
    }

    /// 自定义第 5~7 级编码
    pub fn with_codes(
        mut self,
        protocol_version: &str,
        arsp: &str,
        vendor: &str,
    ) -> Result<Self, String> {
        self.protocol_version =
            ensure_base36_segment("protocol_version", protocol_version, 1, 1)?;
        self.arsp = ensure_base36_segment("arsp", arsp, 1, 6)?;
        self.vendor = ensure_base36_segment("vendor", vendor, 1, 6)?;
        Ok(self)
    }

    /// 自定义本体/实体序列号长度（规范允许 1~9 位）
    pub fn with_serial_lengths(
        mut self,
        ontology_serial_len: usize,
        instance_serial_len: usize,
    ) -> Result<Self, String> {
        if !(1..=9).contains(&ontology_serial_len) {
            return Err("本体序列号长度必须在 1~9 之间".to_string());
        }
        if !(1..=9).contains(&instance_serial_len) {
            return Err("实例序列号长度必须在 1~9 之间".to_string());
        }
        self.ontology_serial_len = ontology_serial_len;
        self.instance_serial_len = instance_serial_len;
        Ok(self)
    }

    /// 当前配置使用的 SALT
    pub fn salt(&self) -> &[u8] {
        &self.salt
    }

    /// 使用当前 SALT 计算校验码
    pub fn checksum(&self, body_1_9: &str) -> String {
        calculate_checksum(body_1_9, &self.salt)
    }

    /// 生成实体 AIC（第 9 级非全 0）
    pub fn generate_entity_aic(&self) -> Result<String, String> {
        let ontology_serial =
            generate_nonzero_base36(KIND_ONTOLOGY, self.ontology_serial_len);
        let instance_serial =
            generate_nonzero_base36(KIND_INSTANCE, self.instance_serial_len);
        Ok(self.build(&ontology_serial, &instance_serial))
    }

    /// 生成本体 AIC（第 9 级全 0）
    pub fn generate_ontology_aic(&self) -> Result<String, String> {
        let ontology_serial =
            generate_nonzero_base36(KIND_ONTOLOGY, self.ontology_serial_len);
        let instance_serial = "0".repeat(self.instance_serial_len);
        Ok(self.build(&ontology_serial, &instance_serial))
    }

    /// 由本体 AIC 派生实体 AIC
    pub fn entity_from_ontology(&self, ontology_aic: &str) -> Option<String> {
        let info = parse_and_check(&ontology_aic)?;
        if !info.is_ontology() {
            return None;
        }

        let instance_len = info.instance_serial.len();
        let instance_serial = generate_nonzero_base36(KIND_ENTITY, instance_len);
        let body = format!("{}.{}", info.ontology_prefix(), instance_serial);
        Some(format!("{}.{}", body, self.checksum(&body)))
    }

    /// 由实体 AIC 派生本体 AIC
    pub fn ontology_from_entity(&self, entity_aic: &str) -> Option<String> {
        let info = parse_and_check(&entity_aic)?;
        let instance_serial = "0".repeat(info.instance_serial.len());
        let body = format!("{}.{}", info.ontology_prefix(), instance_serial);
        Some(format!("{}.{}", body, self.checksum(&body)))
    }

    /// 拼接最终 AIC
    fn build(&self, ontology_serial: &str, instance_serial: &str) -> String {
        let body = format!(
            "{}.{}.{}.{}.{}.{}",
            AIC_OID_PREFIX,
            self.protocol_version,
            self.arsp,
            self.vendor,
            ontology_serial,
            instance_serial
        );
        format!("{}.{}", body, self.checksum(&body))
    }
}

/// 解析并校验 AIC 格式，前缀不符时返回 None
fn parse_and_check(aic: &str) -> Option<AicInfo> {
    validate_aic_format_with_prefix(aic, AIC_OID_PREFIX).ok()
}

/// 解析十六进制字符串为字节（支持 "0x" 前缀、奇数长度左补 0）
///
/// 解析失败时与参考实现保持一致，退化为 `b"\xff\xff"`。
pub fn parse_hex_salt(hex_str: &str) -> Vec<u8> {
    let trimmed = hex_str.trim();
    let without_prefix = if trimmed.to_lowercase().starts_with("0x") {
        &trimmed[2..]
    } else {
        trimmed
    };

    if without_prefix.is_empty() {
        return Vec::new();
    }

    let normalized = if without_prefix.len() % 2 != 0 {
        format!("0{}", without_prefix)
    } else {
        without_prefix.to_string()
    };

    // 与参考实现一致：解析失败时退化为 b"\xff\xff"
    hex::decode(&normalized).unwrap_or_else(|_| vec![0xFF, 0xFF])
}

/// 校验 Base36 段并返回规范化结果
fn ensure_base36_segment(
    name: &str,
    value: &str,
    min_len: usize,
    max_len: usize,
) -> Result<String, String> {
    let normalized = normalize_aic(value);
    if normalized.is_empty() {
        return Err(format!("{} 不能为空", name));
    }
    if !is_base36(&normalized) {
        return Err(format!("{} 必须仅包含 0-9 与 A-Z", name));
    }
    if normalized.len() < min_len || normalized.len() > max_len {
        return Err(format!(
            "{} 长度必须在 {}~{} 之间",
            name, min_len, max_len
        ));
    }
    Ok(normalized)
}

/// 取得北京时间当年内的毫秒数
fn milliseconds_of_year() -> u128 {
    let offset = FixedOffset::east_opt(8 * 3600).expect("东八区偏移有效");
    let now = Utc::now().with_timezone(&offset);
    let year_start = offset
        .with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0)
        .single()
        .unwrap_or(now);

    now.signed_duration_since(year_start)
        .num_milliseconds()
        .max(0) as u128
}

/// 生成 8 字节随机盐
fn random_salt_8() -> [u8; 8] {
    let uuid = Uuid::new_v4();
    let mut salt = [0u8; 8];
    salt.copy_from_slice(&uuid.as_bytes()[..8]);
    salt
}

/// 由「年内毫秒数 + 随机盐 + 用途标签」派生指定长度的 Base36 序列号
///
/// 参考实现使用 BLAKE2b-128，此处使用等强度的 SHA-256 截断至 128 位；
/// 序列号本身仅要求高熵且非全 0，不存在跨实现的校验约束。
fn serial_from_entropy(ms_in_year: u128, salt: &[u8], kind: &[u8], length: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind);
    hasher.update(ms_in_year.to_be_bytes());
    hasher.update(salt);
    let digest = hasher.finalize();

    let mut value_bytes = [0u8; 16];
    value_bytes.copy_from_slice(&digest[..16]);
    let value = u128::from_be_bytes(value_bytes);

    base36_encode_u128(value, length)
}

/// 生成非全 0 的 Base36 序列号
fn generate_nonzero_base36(kind: &[u8], length: usize) -> String {
    let ms_in_year = milliseconds_of_year();
    loop {
        let salt = random_salt_8();
        let serial = serial_from_entropy(ms_in_year, &salt, kind, length);
        if !serial.chars().all(|c| c == '0') {
            return serial;
        }
    }
}
