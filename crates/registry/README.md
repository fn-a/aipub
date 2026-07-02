# Registry - 智能体注册服务

基于 ATR (Agent Trust Registry) 协议实现的智能体注册服务。

## 功能特性

- ✅ 智能体注册和注销
- ✅ ACS 能力描述管理
- ✅ EAB 凭据管理
- ✅ mTLS 证书验证
- ✅ 本体和实体关系管理

## 架构设计

```text
┌─────────────────────────────────────────┐
│          Registry API Layer             │
│  - HTTP API                             │
│  - mTLS Endpoint                        │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Registration Service           │
│  - ACS Validation                       │
│  - AIC Verification                     │
│  - EAB Credential Check                 │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Storage Layer                  │
│  - PostgreSQL (Metadata)                │
│  - Redis (Cache)                        │
└─────────────────────────────────────────┘
```

## 注册流程

### 1. 申请 EAB 凭据

```bash
POST /api/v1/eab
{
  "accountId": "user-001",
  "aic": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353"
}

Response:
{
  "keyId": "eab-001",
  "hmacKey": "secret-key-here"
}
```

### 2. 提交注册申请

```bash
POST /api/v1/agents
Authorization: EAB keyId="eab-001", hmacKey="secret-key-here"
{
  "aic": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353",
  "name": "北京旅游助手",
  "description": "提供北京旅游规划服务",
  "version": "1.0.0",
  "provider": { ... },
  "securitySchemes": { ... },
  "capabilities": { ... },
  "skills": [ ... ]
}
```

### 3. 派生实体注册（mTLS）

对于本体派生的实体，可以通过 mTLS 自动注册：

```bash
POST /api/v1/agents/derived
# 使用本体证书进行 mTLS 认证
{
  "instanceSerial": "000000001",
  "name": "北京旅游助手实例-001",
  "entityMeta": { "location": "Beijing" }
}
```

## 本体和实体

### 本体 (Ontology)

本体是智能体的模板定义，实例序列号全为 0：

```
AIC: 1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353
                                             ^^^^^^^^^
                                             实例序列号全0
```

### 实体 (Entity)

实体是本体的具体实例，实例序列号非全 0：

```
AIC: 1.2.156.3088.1.K9.12345.E789ABCDEF.000000001.2353
                                             ^^^^^^^^^
                                             实例序列号非0
```

### 关系管理

```rust
// 获取本体的所有实体
let entities = registry.list_entities(ontology_aic).await?;

// 获取实体的本体
let ontology = registry.get_ontology(entity_aic).await?;
```

## EAB 凭据

EAB (External Account Binding) 凭据用于将智能体注册与外部账号绑定：

```text
Account (user-001) ──绑定──> AIC (本体)
                            │
                            ├──> Entity 001
                            ├──> Entity 002
                            └──> Entity 003
```

## 数据库设计

```sql
-- 智能体表
CREATE TABLE agents (
    aic VARCHAR(100) PRIMARY KEY,
    acs JSONB NOT NULL,
    active BOOLEAN DEFAULT true,
    ontology_aic VARCHAR(100),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_agents_ontology ON agents(ontology_aic);
CREATE INDEX idx_agents_active ON agents(active);

-- EAB 凭据表
CREATE TABLE eab_credentials (
    key_id VARCHAR(100) PRIMARY KEY,
    hmac_key VARCHAR(200) NOT NULL,
    account_id VARCHAR(100) NOT NULL,
    aic VARCHAR(100) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    revoked_at TIMESTAMP
);

CREATE INDEX idx_eab_account ON eab_credentials(account_id);
CREATE INDEX idx_eab_aic ON eab_credentials(aic);
```

## 配置

```toml
[registry]
port = 8081
auto_approval = false
require_eab = true
enable_mtls = true

[registry.storage]
type = "postgres"
url = "postgresql://localhost/registry"

[registry.ca]
url = "https://ca.example.com"
```

## 参考

- [ACPs-spec-ATR](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/04-ACPs-spec-ATR)
- [ACPs-Registry-Server](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/registry-server)
