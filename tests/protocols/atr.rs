//! ATR (Agent Trust Registry) 支撑算法测试
//!
//! 测试 EAB 的 HMAC-SHA256 签名/校验、吊销原因码映射、证书用途与有效期约束

use aipub::protocols::atr::*;

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    use super::*;

    #[test]
    fn test_hmac_sha256_rfc4231_case2() {
        let expected = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
        assert_eq!(
            hex::encode(hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            expected
        );
    }

    #[test]
    fn test_eab_sign_and_verify() {
        let mac_key = generate_mac_key();
        assert_eq!(mac_key.len(), 43);

        let protected =
            URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","kid":"abc","url":"https://ca/new-account"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"kty":"RSA","n":"...","e":"AQAB"}"#);

        let signature = sign_eab_hmac(&mac_key, &protected, &payload).unwrap();
        assert!(verify_eab_hmac(&mac_key, &protected, &payload, &signature));

        let tampered = URL_SAFE_NO_PAD.encode(br#"{"kty":"RSA","n":"tampered"}"#);
        assert!(!verify_eab_hmac(&mac_key, &protected, &tampered, &signature));
        assert!(!verify_eab_hmac(&generate_mac_key(), &protected, &payload, &signature));
        assert!(!verify_eab_hmac("not-base64!", &protected, &payload, &signature));
    }

    #[test]
    fn test_revocation_reason_mapping() {
        assert_eq!(RevocationReason::from_code(5), Some(RevocationReason::CessationOfOperation));
        assert_eq!(RevocationReason::from_code(6), None);
        assert_eq!(RevocationReason::Superseded.code(), 4);
        assert_eq!(RevocationReason::CaCompromise.name(), "cACompromise");
        assert_eq!(
            RevocationReason::from_label("续期替换"),
            Some(RevocationReason::Superseded)
        );
        assert_eq!(RevocationReason::from_label("unknown"), None);
    }

    #[test]
    fn test_certificate_usage_wire_values() {
        assert_eq!(CertificateUsage::ClientAuth.as_str(), "clientAuth");
        assert_eq!(CertificateUsage::ServerAuth.as_str(), "serverAuth");
        assert_eq!(
            CertificateUsage::from_str("serverAuth"),
            Some(CertificateUsage::ServerAuth)
        );
        assert_eq!(CertificateUsage::from_str("other"), None);
    }

    #[test]
    fn test_normalize_validity_days() {
        assert_eq!(normalize_validity_days(None), DEFAULT_CERT_VALIDITY_DAYS);
        assert_eq!(normalize_validity_days(Some(0)), 1);
        assert_eq!(normalize_validity_days(Some(90)), 90);
        assert_eq!(normalize_validity_days(Some(99999)), MAX_CERT_VALIDITY_DAYS);
    }
}
