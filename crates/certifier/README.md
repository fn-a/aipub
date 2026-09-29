# Certifier - 证书服务 (CA)

基于 ACME 协议实现的智能体证书颁发服务。

## 当前实现状态

库内已实现 ACME 流程状态机（`create_account_with_eab` → `create_order` →
`finalize_order`）、EAB 的 HMAC-SHA256 校验、吊销、CRL 与 OCSP 查询。

⚠️ **尚未接入真实密码学**：证书材料由内存 CA 生成显式标注为占位内容的 PEM，
不解析/校验 CSR 签名，CRL/OCSP 也不是 DER 编码。真实 X.509 签发、JWS 验签与
nonce 管理属于后续规划（见根 README）。本节以下描述的 HTTP 接口尚未提供。

## 功能特性

- ✅ 证书签发流程（ACME 状态机，占位证书材料）
- ✅ EAB 凭据验证（HMAC-SHA256）
- ✅ CRL (证书吊销列表) 管理
- ✅ OCSP (在线证书状态协议) 响应
- 🚧 真实 X.509 签发与 CSR 解析

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
    { "type": "aic", "value": "1.2.156.3088.1.0001.00001.ABC123.000000.0M9Y" }
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
Subject: CN=1.2.156.3088.1.0001.00001.ABC123.000001.176F
Issuer: CN=Agent Trusted Registration CA
Not Before: 2025-01-01 00:00:00 UTC
Not After: 2025-02-19 00:00:00 UTC   # 默认 49 天
Extensions:
  - Subject Alternative Name: URI:acps://{AIC}（以及 ACS certificate.altNames 中声明的 DNS/IP）
  - Key Usage: Digital Signature, Key Encipherment (critical)
  - Extended Key Usage: clientAuth 或 serverAuth（二选一，critical）
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
# 未声明 requestedValidity 时的默认有效期（ATR 规范默认 49 天，上限 1825 天）
cert_validity_days = 49
max_cert_validity_days = 1825

[certifier.storage]
type = "postgres"
url = "postgresql://localhost/certifier"

[certifier.hsm]
enabled = false
```

## 参考

- [ACPs-CA-Server](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/ca-server)
- [RFC 8555 - ACME Protocol](https://tools.ietf.org/html/rfc8555)
