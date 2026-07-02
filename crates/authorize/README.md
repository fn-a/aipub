# Authorize - 鉴权服务

面向群组交互模式的鉴权与群组 ACL 服务。

## 功能特性

- ✅ 消息队列认证
- ✅ 群组访问控制 (ACL)
- ✅ 智能体权限管理
- ✅ 群组邀请和加入

## 架构设计

```text
┌─────────────────────────────────────────┐
│          Authorize API Layer            │
│  - HTTP Auth Backend                    │
│  - Group Management API                 │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Authorization Engine           │
│  - ACL Checker                          │
│  - Group Manager                        │
│  - Permission Manager                   │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Storage Layer                  │
│  - PostgreSQL (ACL/Groups)              │
│  - Redis (Cache)                        │
└─────────────────────────────────────────┘
```

## 消息队列认证

作为 RabbitMQ/Redis 的 HTTP Auth Backend：

```bash
POST /auth/mq
{
  "username": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000001.2353",
  "password": "certificate-thumbprint",
  "vhost": "/",
  "resource": "queue",
  "name": "group-001",
  "permission": "write"
}

Response:
{
  "allow": true,
  "tags": ["partner"]
}
```

## 群组管理

### 创建群组

```bash
POST /api/v1/groups
Authorization: Bearer {token}
{
  "leaderAic": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000001.2353"
}

Response:
{
  "groupId": "group-001",
  "leaderAic": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000001.2353",
  "createdAt": "2025-03-15T16:30:00Z"
}
```

### 邀请成员

```bash
POST /api/v1/groups/{groupId}/invite
Authorization: Bearer {token}
{
  "partnerAic": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000002.2353"
}
```

### 加入群组

```bash
POST /api/v1/groups/{groupId}/join
Authorization: Bearer {token}
{
  "partnerAic": "1.2.156.3088.1.K9.12345.E789ABCDEF.000000002.2353"
}
```

## ACL 规则

```sql
CREATE TABLE acl_rules (
    id SERIAL PRIMARY KEY,
    aic VARCHAR(100) NOT NULL,
    resource VARCHAR(200) NOT NULL,
    action VARCHAR(50) NOT NULL,
    allowed BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW()
);

-- 示例规则
INSERT INTO acl_rules (aic, resource, action, allowed) VALUES
('1.2.156.3088.1.K9.12345.E789ABCDEF.000000001.2353', 'group-001', 'write', true),
('1.2.156.3088.1.K9.12345.E789ABCDEF.000000002.2353', 'group-001', 'read', true);
```

## 配置

```toml
[authorize]
port = 8083
enable_group = true
default_permission = "read"

[authorize.storage]
type = "postgres"
url = "postgresql://localhost/authorize"

[authorize.mq]
type = "rabbitmq"
url = "amqp://localhost"
```

## 参考

- [ACPs-MQ-Auth-Server](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/mq-auth-server)
