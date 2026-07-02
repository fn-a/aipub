# Certifier - 证书服务 (CA)

基于 ACME 协议实现的智能体证书颁发服务。

## 功能特性

- ✅ X.509 证书颁发
- ✅ EAB 凭据验证
- ✅ CRL (证书吊销列表) 管理
- ✅ OCSP (在线证书状态协议) 响应

## 架构设计

```text
┌─────────────────────────────────────────┐
│          Certifier API Layer            │
│  - ACME Protocol                        │
│  - REST API                             │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Certificate Authority          │
│  - CSR Validation                       │
│  - Certificate Issuance                 │
│  - Key Management                       │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Storage Layer                  │
│  - PostgreSQL (Certificates)            │
│  - HSM (Keys, Optional)                 │
└─────────────────────────────────────────┘
```

## 证书申请流程

### 1. 创建账户

```bash
POST /acme/new-account
{
  "contact": ["mailto:admin@example.com"],
  "termsOfServiceAgreed": true
}
```

### 2. 申请证书

```bash
POST /acme/new-order
{
  "identifiers": [
    { "type": "aic", "value": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353" }
  ]
}
```

### 3. 提交 CSR

```bash
POST /acme/finalize/{order-id}
{
  "csr": "base64-encoded-csr"
}
```

### 4. 下载证书

```bash
GET /acme/cert/{cert-id}
```

## EAB 凭据

EAB (External Account Binding) 用于将证书申请与外部账号绑定：

```bash
POST /acme/new-account
{
  "contact": ["mailto:admin@example.com"],
  "termsOfServiceAgreed": true,
  "externalAccountBinding": {
    "keyId": "eab-001",
    "hmacKey": "secret-key"
  }
}
```

## 证书格式

智能体证书包含以下信息：

```text
Subject: CN=1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353
Issuer: CN=AIP CA
Not Before: 2025-01-01 00:00:00 UTC
Not After: 2026-01-01 00:00:00 UTC
Extensions:
  - Subject Alternative Name: AIC
  - Key Usage: Digital Signature, Key Encipherment
  - Extended Key Usage: Server Auth, Client Auth
```

## CRL 和 OCSP

### CRL 分发

```bash
GET /crl
```

返回证书吊销列表。

### OCSP 查询

```bash
POST /ocsp
Content-Type: application/ocsp-request

{ocsp-request-data}
```

返回证书状态响应。

## 配置

```toml
[certifier]
port = 8082
ca_cert_path = "/etc/aip/ca.crt"
ca_key_path = "/etc/aip/ca.key"
cert_validity_days = 365

[certifier.storage]
type = "postgres"
url = "postgresql://localhost/certifier"

[certifier.hsm]
enabled = false
```

## 参考

- [ACPs-CA-Server](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/ca-server)
- [RFC 8555 - ACME Protocol](https://tools.ietf.org/html/rfc8555)
