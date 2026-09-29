//! Certifier 证书服务测试
//!
//! 测试证书签发、EAB 账号绑定、订单与 finalize 流程、有效期约束、吊销、CRL 与 OCSP

use aipub::certifier::*;

#[cfg(test)]
mod tests {
    use aipub::protocols::atr::{
        generate_mac_key, sign_eab_hmac, CertificateUsage, RevocationReason,
        DEFAULT_CERT_VALIDITY_DAYS, MAX_CERT_VALIDITY_DAYS,
    };
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    use super::*;

    const AIC: &str = "1.2.156.3088.1.0001.00001.ABC123.000001.0H9T";

    fn service() -> CertifierServiceImpl {
        CertifierServiceImpl::new(CertifierConfig::default())
    }

    async fn account(service: &CertifierServiceImpl) -> AcmeAccount {
        let mac_key = generate_mac_key();
        let protected = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","kid":"k1"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"kty":"RSA","n":"x","e":"AQAB"}"#);
        let signature = sign_eab_hmac(&mac_key, &protected, &payload).unwrap();

        service
            .create_account_with_eab(AIC, &mac_key, &protected, &payload, &signature)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_issue_and_query_certificate() {
        let service = service();
        let certificate = service
            .issue_certificate(b"mock-csr-bytes".to_vec(), AIC)
            .await
            .unwrap();

        assert_eq!(certificate.aic, AIC);
        assert_eq!(certificate.status, CertStatus::Valid);
        assert_eq!(certificate.usage, CertificateUsage::ClientAuth);
        assert_eq!(certificate.san_uri(), format!("acps://{}", AIC));
        assert!(certificate.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(certificate.chain_pem.contains("BEGIN CERTIFICATE"));
        assert_eq!(certificate.csr_fingerprint.len(), 64);
        assert!(certificate.not_after > certificate.not_before);

        assert_eq!(
            service
                .get_certificate_status(&certificate.serial_number)
                .await
                .unwrap(),
            CertStatus::Valid
        );
        assert_eq!(
            service.get_certificate_status("missing").await.unwrap(),
            CertStatus::Unknown
        );
    }

    #[tokio::test]
    async fn test_eab_account_then_order_then_finalize() {
        let service = service();
        let account = account(&service).await;
        assert_eq!(account.aic, AIC);

        let order = service
            .create_order(&account.id, AIC, CertificateUsage::ServerAuth, None)
            .await
            .unwrap();
        assert_eq!(order.status, OrderStatus::Ready);

        let certificate = service.finalize_order(&order.id, b"csr".to_vec()).await.unwrap();
        assert_eq!(certificate.usage, CertificateUsage::ServerAuth);

        // 有效期默认 49 天
        let days = (certificate.not_after - certificate.not_before).num_days();
        assert_eq!(days, DEFAULT_CERT_VALIDITY_DAYS as i64);

        let stored = service.get_order(&order.id).await.unwrap();
        assert_eq!(stored.status, OrderStatus::Valid);
        assert_eq!(
            stored.certificate_serial,
            Some(certificate.serial_number.clone())
        );
    }

    #[tokio::test]
    async fn test_eab_signature_is_verified() {
        let service = service();
        let mac_key = generate_mac_key();
        let protected = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","kid":"k1"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"kty":"RSA","n":"x","e":"AQAB"}"#);

        // 未签名
        assert!(service
            .create_account_with_eab(AIC, &mac_key, &protected, &payload, "bad-signature")
            .await
            .is_err());

        // 用其它密钥签名
        let signature = sign_eab_hmac(&generate_mac_key(), &protected, &payload).unwrap();
        assert!(service
            .create_account_with_eab(AIC, &mac_key, &protected, &payload, &signature)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_order_requires_matching_account_aic() {
        let service = service();
        let account = account(&service).await;

        let other = "1.2.156.3088.1.0001.00001.ABC123.000002.0H9T";
        assert!(service
            .create_order(&account.id, other, CertificateUsage::ClientAuth, None)
            .await
            .is_err());
        assert!(service
            .create_order("missing-account", AIC, CertificateUsage::ClientAuth, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_finalize_requires_ready_order() {
        let service = service();
        let account = account(&service).await;
        let order = service
            .create_order(&account.id, AIC, CertificateUsage::ClientAuth, None)
            .await
            .unwrap();

        service.finalize_order(&order.id, b"csr".to_vec()).await.unwrap();

        // 订单已 valid，不能重复 finalize
        let error = service
            .finalize_order(&order.id, b"csr".to_vec())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("ORDER_NOT_READY"));

        assert!(service.finalize_order("missing-order", vec![]).await.is_err());
    }

    #[tokio::test]
    async fn test_requested_validity_is_clamped() {
        let service = service();
        let account = account(&service).await;

        let order = service
            .create_order(&account.id, AIC, CertificateUsage::ClientAuth, Some(99999))
            .await
            .unwrap();
        let certificate = service.finalize_order(&order.id, b"csr".to_vec()).await.unwrap();
        assert_eq!(
            (certificate.not_after - certificate.not_before).num_days(),
            MAX_CERT_VALIDITY_DAYS as i64
        );
    }

    #[tokio::test]
    async fn test_revocation_crl_and_ocsp() {
        let service = service();
        let certificate = service
            .issue_certificate(b"csr".to_vec(), AIC)
            .await
            .unwrap();
        assert!(service.crl().await.entries.is_empty());

        service
            .revoke_with_reason(&certificate.serial_number, RevocationReason::KeyCompromise)
            .await
            .unwrap();

        assert_eq!(
            service
                .get_certificate_status(&certificate.serial_number)
                .await
                .unwrap(),
            CertStatus::Revoked
        );

        // 重复吊销
        assert!(service
            .revoke_with_reason(&certificate.serial_number, RevocationReason::Superseded)
            .await
            .is_err());

        let crl = service.crl().await;
        assert_eq!(crl.entries.len(), 1);
        assert_eq!(crl.entries[0].reason, RevocationReason::KeyCompromise);
        assert_eq!(crl.version, 1);
        assert!(crl.next_update > crl.generated_at);

        let ocsp = service.ocsp(&certificate.serial_number).await;
        assert_eq!(ocsp.status, CertStatus::Revoked);
        assert_eq!(ocsp.revocation_reason, Some(RevocationReason::KeyCompromise));

        let unknown = service.ocsp("missing").await;
        assert_eq!(unknown.status, CertStatus::Unknown);
    }

    #[tokio::test]
    async fn test_revoke_by_aic() {
        let service = service();
        let first = service.issue_certificate(b"csr-1".to_vec(), AIC).await.unwrap();
        let second = service.issue_certificate(b"csr-2".to_vec(), AIC).await.unwrap();

        let revoked = service
            .revoke_by_aic(AIC, RevocationReason::CessationOfOperation)
            .await
            .unwrap();
        assert_eq!(revoked.len(), 2);

        assert_eq!(
            service
                .get_certificate_status(&first.serial_number)
                .await
                .unwrap(),
            CertStatus::Revoked
        );
        assert_eq!(
            service
                .get_certificate_status(&second.serial_number)
                .await
                .unwrap(),
            CertStatus::Revoked
        );

        // 再次吊销时没有有效证书
        assert!(service
            .revoke_by_aic(AIC, RevocationReason::Unspecified)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_serial_number_is_positive_hex() {
        let service = service();
        let certificate = service.issue_certificate(b"csr".to_vec(), AIC).await.unwrap();
        let serial = certificate.serial_number;
        assert_eq!(serial.len(), 40);
        assert!(serial.chars().all(|c| c.is_ascii_hexdigit()));
        let first = u8::from_str_radix(&serial[..2], 16).unwrap();
        assert!(first < 0x80);
    }
}
