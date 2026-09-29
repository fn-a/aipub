# Authorize - 鉴权服务

面向群组交互模式的鉴权与群组 ACL 服务。

## 当前实现状态

库内已实现 RabbitMQ HTTP 认证后端的判定规则（`/auth/user|vhost|resource|topic`）、
群组 ACL 与邀请/加入/移除/解散。鉴权结论与参考实现一致：返回纯文本
`allow` 或 `deny`（不返回 JSON，也不返回 tags）。

⚠️ 参考实现**不校验密码**：任何合法 AIC 用户名配合任意密码都能通过 PLAIN 认证，
真正的访问控制来自 mTLS 客户端证书与群组 ACL。本节以下描述的 HTTP 接口尚未提供。

## 功能特性

- ✅ 消息队列认证（用户名须为合法 AIC，vhost 须为 `acps`）
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

作为 RabbitMQ 的 HTTP Auth Backend（表单编码 POST，返回纯文本 `allow` / `deny`）：

```bash
POST /auth/resource
username=1.2.156.3088.1.0001.00001.ABC123.000001.176F
&vhost=acps
&resource=exchange
&name=group_1.2.156.3088.1.0001.00001.ABC123.000001.176F_group-001
&permission=write

Response: allow
```

判定规则（与参考实现一致）：

- `amq.default` 交换机一律拒绝；
- `inbox.topic` 交换机一律允许，读取时路由键必须等于 `inbox_{username}`；
- `inbox_{AIC}` 队列仅其所有者可访问；
- `group_{leader}_{group}` 交换机仅 Leader 可 configure，读写需要 ACL 成员身份；
- `group_{leader}_{group}_{member}` 队列：configure/write 允许 Leader 或活跃成员，read 仅活跃成员。

## 群组管理

### 管理群组成员

Leader 以 mTLS 客户端证书身份调用（证书 CN 必须等于路径中的 `leader_aic`）：

```bash
# 添加成员（事实上等同于创建群组）
PUT /groups/{leader_aic}/{group_id}/members/{member_aic}

# 移除成员
DELETE /groups/{leader_aic}/{group_id}/members/{member_aic}

# 删除整个群组 ACL
DELETE /groups/{leader_aic}/{group_id}

# 强制关闭成员连接
DELETE /groups/{leader_aic}/{group_id}/members/{member_aic}/connection
```

成功返回 204；证书 CN 缺失返回 403，与路径 Leader 不一致返回 403。

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
('1.2.156.3088.1.0001.00001.ABC123.000001.176F', 'group-001', 'write', true),
('1.2.156.3088.1.0001.00001.ABC123.000002.0ABP', 'group-001', 'read', true);
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
