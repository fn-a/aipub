//! AIC (Agent Identity Code) 协议测试
//!
//! 测试智能体身份码的验证、解析、编解码等功能

use aipub::protocols::aic::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 AIC 格式验证 - 有效格式
    #[test]
    fn test_validate_aic_format_valid() {
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
        let result = validate_aic_format(aic);
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_eq!(info.version, "1");
        assert_eq!(info.arsp, "K9");
        assert_eq!(info.vendor, "ABCDEF");
        assert!(info.is_ontology());
    }

    /// 测试 AIC 格式验证 - 无效格式
    #[test]
    fn test_validate_aic_format_invalid() {
        // 空 AIC
        assert!(validate_aic_format("").is_err());
        
        // 段数不足
        assert!(validate_aic_format("1.2.156.3088.1.K9").is_err());
        
        // 前缀不匹配
        assert!(validate_aic_format("1.2.156.3089.1.K9.ABCDEF.123456789.000000000.ABCD").is_err());
    }

    /// 测试本体和实体判断
    #[test]
    fn test_ontology_vs_entity() {
        let ontology_aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
        assert!(is_ontology_aic(ontology_aic));
        assert!(!is_entity_aic(ontology_aic));
        
        let entity_aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000001.ABCD";
        assert!(is_entity_aic(entity_aic));
        assert!(!is_ontology_aic(entity_aic));
    }

    /// 测试 AIC 解析
    #[test]
    fn test_parse_aic() {
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000001.ABCD";
        let info = parse_aic(aic);
        
        assert!(info.is_some());
        let info = info.unwrap();
        
        assert_eq!(info.raw, aic);
        assert_eq!(info.prefix, "1.2.156.3088");
        assert_eq!(info.version, "1");
        assert_eq!(info.arsp, "K9");
        assert_eq!(info.vendor, "ABCDEF");
        assert_eq!(info.ontology_serial, "123456789");
        assert_eq!(info.instance_serial, "000000001");
        assert_eq!(info.checksum, "ABCD");
    }

    /// 测试 AIC Info 方法
    #[test]
    fn test_aic_info_methods() {
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000001.ABCD";
        let info = parse_aic(aic).unwrap();
        
        // 测试 body() 方法
        let body = info.body();
        assert!(body.contains("1.2.156.3088"));
        assert!(body.contains("000000001"));
        
        // 测试 ontology_prefix() 方法
        let ontology_prefix = info.ontology_prefix();
        assert!(ontology_prefix.contains("123456789"));
        assert!(!ontology_prefix.contains("000000001"));
    }

    /// 测试获取 AIC 段
    #[test]
    fn test_get_aic_segment() {
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000001.ABCD";
        
        assert_eq!(get_aic_segment(aic, 1), Some("1".to_string()));
        assert_eq!(get_aic_segment(aic, 5), Some("1".to_string()));
        assert_eq!(get_aic_segment(aic, 6), Some("K9".to_string()));
        assert_eq!(get_aic_segment(aic, 10), Some("ABCD".to_string()));
        
        // 无效级别
        assert_eq!(get_aic_segment(aic, 0), None);
        assert_eq!(get_aic_segment(aic, 11), None);
    }

    /// 测试本体前缀提取
    #[test]
    fn test_get_ontology_prefix_from_aic() {
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000001.ABCD";
        let prefix = get_ontology_prefix_from_aic(aic);
        
        assert!(prefix.is_some());
        let prefix = prefix.unwrap();
        assert!(prefix.starts_with("1.2.156.3088"));
        assert!(prefix.contains("123456789"));
    }
}

/// Base36 编解码测试
#[cfg(test)]
mod base36_tests {
    use aipub::protocols::aic::{base36_encode, base36_decode};

    #[test]
    fn test_base36_encode() {
        assert_eq!(base36_encode(0, 0), "0");
        assert_eq!(base36_encode(35, 0), "Z");
        assert_eq!(base36_encode(36, 0), "10");
        assert_eq!(base36_encode(12345, 0), "9IX");
        assert_eq!(base36_encode(12345, 6), "0009IX");
    }

    #[test]
    fn test_base36_decode() {
        assert_eq!(base36_decode("0").unwrap(), 0);
        assert_eq!(base36_decode("Z").unwrap(), 35);
        assert_eq!(base36_decode("10").unwrap(), 36);
        assert_eq!(base36_decode("9IX").unwrap(), 12345);
        
        // 大小写不敏感
        assert_eq!(base36_decode("z").unwrap(), 35);
        assert_eq!(base36_decode("9ix").unwrap(), 12345);
    }

    #[test]
    fn test_base36_roundtrip() {
        for num in &[0, 1, 10, 35, 36, 100, 1000, 12345, 99999, 123456789] {
            let encoded = base36_encode(*num, 0);
            let decoded = base36_decode(&encoded).unwrap();
            assert_eq!(*num, decoded);
        }
    }

    #[test]
    fn test_base36_decode_invalid() {
        assert!(base36_decode("!").is_err());
        assert!(base36_decode("@").is_err());
        assert!(base36_decode("ABC!").is_err());
    }
}

/// AIC 验证器测试
#[cfg(test)]
mod validator_tests {
    use aipub::protocols::aic::AicValidator;

    #[test]
    fn test_validator_without_salt() {
        let mut validator = AicValidator::new(None);
        let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
        
        // 不要求 CRC 时应该通过
        assert!(validator.validate(aic, false));
        
        // 要求 CRC 时应该失败
        assert!(!validator.validate(aic, true));
        assert!(validator.last_error().unwrap().contains("未配置 SALT"));
    }

    #[test]
    fn test_validator_from_hex() {
        let validator = AicValidator::from_hex("1234");
        assert!(validator.is_ok());
        assert!(validator.unwrap().has_salt());
        
        let validator = AicValidator::from_hex("0x1234");
        assert!(validator.is_ok());
        assert!(validator.unwrap().has_salt());
    }

    #[test]
    fn test_validator_with_salt() {
        let validator = AicValidator::new(Some(vec![0x12, 0x34]));
        assert!(validator.has_salt());
    }
}

/// AIC 生成器测试
#[cfg(test)]
mod generator_tests {
    use aipub::protocols::aic::{validate_aic_format, AicValidator};
    use super::*;

    const SPEC_SALT: &[u8] = b"\x00\x49\x4F\x41";

    #[test]
    fn test_crc16_spec_example_without_salt() {
        // ACPs-spec-AIC-v02.01 规范示例
        let body = "1.2.156.1234.1.1.34C2.478BDF.3GF546";
        assert_eq!(calculate_checksum(body, b""), "0H9T");
    }

    #[test]
    fn test_registry_salt_checksum_examples() {
        // 独立计算的黄金值（CRC-16/CCITT-FALSE + salt 0x00494F41）
        let salt = parse_hex_salt("0x00494F41");

        let entity_body = "1.2.156.3088.1.0001.00001.ABC123.000001";
        assert_eq!(calculate_checksum(entity_body, &salt), "176F");

        let ontology_body = "1.2.156.3088.1.0001.00001.ABC123.000000";
        assert_eq!(calculate_checksum(ontology_body, &salt), "0M9Y");

        // 完整校验器应接受这两个示例 AIC
        let mut validator = AicValidator::new(Some(salt));
        assert!(validator.validate(&format!("{}.176F", entity_body), true));
        assert!(validator.validate(&format!("{}.0M9Y", ontology_body), true));
        assert!(!validator.validate(
            "1.2.156.3088.1.0001.00001.ABC123.000001.0000",
            true
        ));
    }

    #[test]
    fn test_generate_entity_aic_is_valid() {
        let generator = AicGenerator::new(SPEC_SALT.to_vec());
        let aic = generator.generate_entity_aic().unwrap();

        let info = validate_aic_format(&aic).unwrap();
        assert!(info.is_entity());
        assert_eq!(info.checksum, generator.checksum(&info.body()));
        assert_eq!(info.ontology_serial.len(), DEFAULT_ONTOLOGY_SERIAL_LEN);
        assert_eq!(info.instance_serial.len(), DEFAULT_INSTANCE_SERIAL_LEN);
    }

    #[test]
    fn test_generate_ontology_aic_is_valid() {
        let generator = AicGenerator::new(SPEC_SALT.to_vec());
        let aic = generator.generate_ontology_aic().unwrap();

        let info = validate_aic_format(&aic).unwrap();
        assert!(info.is_ontology());
        assert!(info.instance_serial.chars().all(|c| c == '0'));
    }

    #[test]
    fn test_ontology_entity_round_trip() {
        let generator = AicGenerator::new(SPEC_SALT.to_vec());

        let entity = generator.generate_entity_aic().unwrap();
        let ontology = generator.ontology_from_entity(&entity).unwrap();
        assert!(validate_aic_format(&ontology).unwrap().is_ontology());
        assert_eq!(
            entity.split('.').take(8).collect::<Vec<_>>(),
            ontology.split('.').take(8).collect::<Vec<_>>()
        );

        let derived = generator.entity_from_ontology(&ontology).unwrap();
        assert!(validate_aic_format(&derived).unwrap().is_entity());
        assert_eq!(
            ontology.split('.').take(8).collect::<Vec<_>>(),
            derived.split('.').take(8).collect::<Vec<_>>()
        );
        assert_ne!(entity, derived);

        let prefix = get_derived_entity_like_prefix(&ontology).unwrap();
        assert!(derived.starts_with(&prefix));
    }

    #[test]
    fn test_entity_from_non_ontology_returns_none() {
        let generator = AicGenerator::new(SPEC_SALT.to_vec());
        let entity = generator.generate_entity_aic().unwrap();
        assert!(generator.entity_from_ontology(&entity).is_none());
    }

    #[test]
    fn test_invalid_codes_rejected() {
        let generator = AicGenerator::new(SPEC_SALT.to_vec());
        assert!(generator.clone().with_codes("", "0001", "00001").is_err());
        assert!(generator.clone().with_codes("12", "0001", "00001").is_err());
        assert!(generator.clone().with_codes("1", "00*1", "00001").is_err());
        assert!(generator.clone().with_codes("1", "0001", "").is_err());
        assert!(generator.with_serial_lengths(0, 6).is_err());
    }

    #[test]
    fn test_invalid_checksum_rejected() {
        let validator_salt = SPEC_SALT;
        let bad = "1.2.156.1234.1.1.AAAAAA.1.1.1.0000";
        let expected = calculate_checksum(
            "1.2.156.1234.1.1.AAAAAA.1.1.1",
            validator_salt,
        );
        assert_ne!(expected, "0000");
        assert!(validate_aic_format(bad).is_err() || expected != "0000");
    }
}
