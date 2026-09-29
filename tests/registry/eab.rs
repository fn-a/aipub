//! Registry EAB (External Account Binding) 测试
//!
//! 测试 EAB 凭据的签发、一次性消费、过期与撤销

use aipub::registry::*;

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    use aipub::protocols::atr::{generate_mac_key, hmac_sha256, sign_eab_hmac, verify_eab_hmac};
    use super::*;

    #[tokio::test]
    async fn test_create_and_consume_once() {
        let manager = MemoryEabManager::new();
        let credential = manager.create("account-1", "aic-1").await.unwrap();

        assert_eq!(credential.aic, "aic-1");
        assert!(!credential.is_consumed);
        // base64url 无填充的 32 字节 = 43 字符
        assert_eq!(credential.mac_key.len(), 43);

        let consumed = manager.consume(&credential.key_id).await.unwrap();
        assert!(consumed.is_consumed);
        assert_eq!(consumed.mac_key, credential.mac_key);

        // 二次消费失败
        let error = manager.consume(&credential.key_id).await.unwrap_err();
        assert!(
            matches!(error, RegistryError::EabError(ref m) if m.starts_with("EAB_ALREADY_CONSUMED"))
        );
    }

    #[tokio::test]
    async fn test_consume_missing_and_verify() {
        let manager = MemoryEabManager::new();
        let credential = manager.create("account-1", "aic-1").await.unwrap();

        assert!(manager
            .verify(&credential.key_id, &credential.mac_key)
            .await
            .unwrap());
        assert!(!manager
            .verify(&credential.key_id, "wrong-key")
            .await
            .unwrap());
        assert!(!manager.verify("missing", "any").await.unwrap());

        let error = manager.consume("missing").await.unwrap_err();
        assert!(matches!(error, RegistryError::EabError(ref m) if m.starts_with("EAB_NOT_FOUND")));
    }

    #[tokio::test]
    async fn test_expired_credential_rejected() {
        // TTL 为 0 表示凭据立即过期
        let manager = MemoryEabManager::new().with_ttl_hours(0);
        let credential = manager.create("account-1", "aic-1").await.unwrap();

        let error = manager.consume(&credential.key_id).await.unwrap_err();
        assert!(matches!(error, RegistryError::EabError(ref m) if m.starts_with("EAB_EXPIRED")));
        assert!(!manager
            .verify(&credential.key_id, &credential.mac_key)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_revoke() {
        let manager = MemoryEabManager::new();
        let credential = manager.create("account-1", "aic-1").await.unwrap();

        manager.revoke(&credential.key_id).await.unwrap();
        assert!(manager.get(&credential.key_id).is_none());
        assert!(manager.revoke(&credential.key_id).await.is_err());
    }

    #[test]
    fn test_hmac_sha256_known_vector() {
        // RFC 4231 Test Case 2
        let expected = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
        assert_eq!(
            hex::encode(hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            expected
        );
    }

    #[tokio::test]
    async fn test_eab_jws_sign_and_verify() {
        let manager = MemoryEabManager::new();
        let mac_key = manager
            .create("account-1", "aic-1")
            .await
            .unwrap()
            .mac_key;

        let protected = URL_SAFE_NO_PAD
            .encode(br#"{"alg":"HS256","kid":"abc","url":"https://ca/new-account"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"kty":"RSA","n":"...","e":"AQAB"}"#);

        let signature = sign_eab_hmac(&mac_key, &protected, &payload).unwrap();
        assert!(verify_eab_hmac(&mac_key, &protected, &payload, &signature));

        // 篡改 payload 后校验失败
        let tampered = URL_SAFE_NO_PAD.encode(br#"{"kty":"RSA","n":"tampered","e":"AQAB"}"#);
        assert!(!verify_eab_hmac(&mac_key, &protected, &tampered, &signature));

        // 错误密钥校验失败
        let other_key = generate_mac_key();
        assert!(!verify_eab_hmac(&other_key, &protected, &payload, &signature));
    }
}
