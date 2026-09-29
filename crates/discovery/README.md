# Discovery - 智能体发现服务

基于 ADP (Agent Discovery Protocol) 协议实现的智能体发现服务。

## 当前实现状态

库内已实现：内存存储（含变更流版本幂等应用）、关键词匹配与排名、
四种查询模式（explicit / exploratory / trending / filtered）、
35 种过滤运算符与 AND/OR/NOT 组合、跨域转发（环路/深度/扇出校验 + 结果聚合）。

⚠️ 存储为内存实现，向量检索与语义匹配尚未接入；本节以下描述的
HTTP 接口与 PostgreSQL/Redis/向量库方案属于后续规划。

## 功能特性

- ✅ 智能体能力查询和匹配
- ✅ 结构化过滤条件支持
- ✅ 跨域发现和转发
- 🚧 向量相似度匹配（计划中）
- 🚧 语义理解增强（计划中）

## 架构设计

```text
┌─────────────────────────────────────────┐
│          Discovery API Layer            │
│  (HTTP/gRPC/Message Queue)              │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Query Engine                   │
│  - Query Parser                         │
│  - Filter Evaluator                     │
│  - Matcher (Keyword/Vector/Semantic)    │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Storage Layer                  │
│  - PostgreSQL (Metadata)                │
│  - Redis (Cache)                        │
│  - Vector DB (Embeddings, Optional)     │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          Forwarder (Optional)           │
│  - Cross-domain Discovery               │
│  - Request Routing                      │
│  - Result Aggregation                   │
└─────────────────────────────────────────┘
```

## 使用示例

```rust
use aipub::discovery::{DiscoveryServiceImpl, DiscoveryConfig, MemoryStorage, SimpleMatcher};
use aipub::protocols::adp::DiscoveryRequest;

#[tokio::main]
async fn main() {
    // 创建服务
    let config = DiscoveryConfig::default();
    let storage = Box::new(MemoryStorage::new());
    let matcher = Box::new(SimpleMatcher::new());
    
    let service = DiscoveryServiceImpl::new(
        config,
        storage,
        matcher,
        None,
    );
    
    // 执行发现查询
    let request = DiscoveryRequest::new()
        .with_query("我需要一个北京美食推荐的智能体")
        .with_limit(10);
    
    let response = service.discover(request).await.unwrap();
    
    if let Some(result) = response.result {
        println!("发现 {} 个智能体", result.agents.len());
    }
}
```

## 查询类型

### 1. 明确查询 (Explicit)

用户有明确的意图，系统通过语义理解和关键词匹配查找智能体。

```json
{
  "type": "explicit",
  "query": "我需要一个可以做北京美食推荐的智能体",
  "limit": 10
}
```

### 2. 探索性查询 (Exploratory)

用户没有明确目标，系统推荐相关或热门智能体。

```json
{
  "type": "exploratory",
  "context": {
    "recentTurns": ["用户询问了北京旅游"],
    "userProfile": { "city": "北京" }
  }
}
```

### 3. 过滤查询 (Filtered)

仅通过结构化过滤条件查询，不进行语义匹配。

```json
{
  "type": "filtered",
  "filter": {
    "conditions": [
      { "field": "active", "op": "eq", "value": true },
      { "field": "skills.tags", "op": "anyOf", "value": ["美食", "推荐"] }
    ],
    "logic": "and"
  }
}
```

### 4. 热门查询 (Trending)

返回当前最热门或最常用的智能体。

```json
{
  "type": "trending",
  "limit": 20
}
```

## 过滤条件

支持丰富的过滤运算符：

### 等值和比较

```json
{ "field": "active", "op": "eq", "value": true }
{ "field": "version", "op": "gte", "value": "2.0.0" }
{ "field": "created_at", "op": "between", "value": ["2024-01-01", "2024-12-31"] }
```

### 字符串匹配

```json
{ "field": "name", "op": "contains", "value": "北京" }
{ "field": "description", "op": "startsWith", "value": "提供" }
```

### 集合运算

```json
{ "field": "skills.tags", "op": "anyOf", "value": ["美食", "旅游"] }
{ "field": "skills.tags", "op": "allOf", "value": ["推荐", "北京"] }
```

### 嵌套条件

```json
{
  "conditions": [
    { "field": "active", "op": "eq", "value": true }
  ],
  "groups": [
    {
      "conditions": [
        { "field": "name", "op": "contains", "value": "北京" },
        { "field": "name", "op": "contains", "value": "美食" }
      ],
      "logic": "or"
    }
  ],
  "logic": "and"
}
```

## 跨域发现

Discovery 服务支持跨域转发，实现跨注册中心的智能体发现：

```text
Discovery Server A ──转发──> Discovery Server B
        │                           │
        │                           │
        └──────聚合结果──────────────┘
```

转发配置：

```json
{
  "forwardDepthLimit": 3,
  "forwardFanoutLimit": 10,
  "forwardEachTimeoutMs": 5000,
  "forwardTotalTimeoutMs": 30000
}
```

## 存储后端

### PostgreSQL (推荐)

存储智能体元数据和 ACS：

```sql
CREATE TABLE agents (
    aic VARCHAR(100) PRIMARY KEY,
    acs JSONB NOT NULL,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_agents_active ON agents(active);
CREATE INDEX idx_agents_skills ON agents USING GIN ((acs->'skills'));
```

### Redis (缓存)

缓存热门查询结果和智能体 ACS：

```text
Key: agent:{aic}
Value: ACS JSON
TTL: 3600s

Key: query:{hash}
Value: DiscoveryResult JSON
TTL: 300s
```

### Vector DB (可选)

存储智能体和技能的向量嵌入，支持语义相似度匹配：

```text
Collection: agent_embeddings
Vector: [skill_name, skill_description, tags] -> embedding
Index: HNSW (Hierarchical Navigable Small World)
```

## 性能优化

1. **查询缓存**: 缓存热门查询结果
2. **索引优化**: 为常用过滤字段建立索引
3. **批量查询**: 批量获取 ACS 数据
4. **异步转发**: 并发转发跨域请求
5. **结果聚合**: 去重和排序优化

## 监控指标

- 查询 QPS 和延迟
- 缓存命中率
- 转发成功率和延迟
- 存储查询延迟

## 配置

```toml
[discovery]
port = 8080
enable_forward = true
max_forward_depth = 3
max_forward_fanout = 10
query_timeout_ms = 5000

[discovery.storage]
type = "postgres"
url = "postgresql://localhost/discovery"

[discovery.cache]
type = "redis"
url = "redis://localhost"
ttl = 3600
```

## 参考

- [ACPs-spec-ADP](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/06-ACPs-spec-ADP)
- [ACPs-Discovery-Server](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/discovery-server)
