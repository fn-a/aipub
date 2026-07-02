# AIP 智能体互联协议 (Agent Interconnection Protocol)

**中文** | [English](README.en.md)

基于 GB/Z 185《人工智能 智能体互联》国家标准的 Rust 实现。

## 项目介绍

本项目是基于 ACPs (Agent Collaboration Protocols) 智能体协作协议体系的 Rust 实现版本，参考 Python 实现进行开发。ACPs 是为保障异构智能体之间高效协作、支持多样化智能体互联应用而设计与实现的标准化交互协议体系。

### 协议体系

| 协议 | 说明 | 模块路径 |
|------|------|---------|
| **AIC** | Agent Identity Code - 智能体身份码 | `aipub::protocols::aic` |
| **ACS** | Agent Capability Specification - 智能体能力描述 | `aipub::protocols::acs` |
| **AIP** | Agent Interaction Protocol - 智能体交互协议 | `aipub::protocols::aip` |
| **ADP** | Agent Discovery Protocol - 智能体发现协议 | `aipub::protocols::adp` |

### 服务组件

| 服务 | 说明 | 模块路径 |
|------|------|---------|
| **Discovery** | 智能体发现服务 | `aipub::discovery` |
| **Registry** | 智能体注册服务 | `aipub::registry` |
| **Certifier** | 证书服务 (CA) | `aipub::certifier` |
| **Authorize** | 鉴权服务 | `aipub::authorize` |

## 快速开始

### 添加依赖

```bash
cargo add aipub
```

### 基本使用

```rust
use aipub::{
   validate_aic_format, parse_aic, AicInfo,
   AgentCapabilitySpec, TaskCommand, TaskState,
   DiscoveryRequest, DiscoveryResponse,
};

// 验证 AIC
let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
if let Ok(info) = validate_aic_format(aic) {
   println!("AIC 有效: {}", info);
   println!("是否为本体: {}", info.is_ontology());
}

// 创建发现请求
let request = DiscoveryRequest::new()
   .with_query("我需要一个北京美食推荐的智能体")
   .with_limit(10);
```

### 运行示例

```bash
# 运行基本示例
cargo run --example basic

# 运行测试
cargo test
```

## 项目结构

```
aipub/
├── Cargo.toml                 # Workspace + Package 配置
├── src/
│   └── lib.rs                 # 统一导出入口 ✅
├── examples/
│   └── basic.rs               # 基本使用示例 ✅
├── tests/                     # 测试用例 ✅
└── crates/
    ├── protocols/             # 核心协议库 ✅
    │   ├── src/
    │   │   ├── aic/           # 智能体身份码
    │   │   ├── acs/           # 智能体能力描述
    │   │   ├── aip/           # 智能体交互协议
    │   │   └── adp/           # 智能体发现协议
    │   └── README.md
    │
    ├── discovery/             # 发现服务框架 🚧
    │   └── README.md
    ├── registry/              # 注册服务框架 🚧
    │   └── README.md
    ├── certifier/             # 证书服务框架 🚧
    │   └── README.md
    └── authorize/             # 鉴权服务框架 🚧
        └── README.md
```

## 协议规范

### AIC - 智能体身份码

AIC (Agent Identity Code) 是智能体的全局唯一身份标识符，采用 10 级 OID 结构：

```
1.2.156.3088.{version}.{arsp}.{vendor}.{ontology_sn}.{instance_sn}.{crc}
```

- **1-4级**: OID 前缀 `1.2.156.3088` (由国家 OID 注册中心分配)
- **5级**: 版本号 (Base36, 1位)
- **6级**: ARSP 注册服务商序号 (Base36, 1-6位)
- **7级**: Vendor 供应商序号 (Base36, 1-6位)
- **8级**: Ontology SN 本体序列号 (Base36, 1-9位)
- **9级**: Instance SN 实例序列号 (Base36, 1-9位, 全0表示本体AIC)
- **10级**: CRC校验码 (Base36, 固定4位)

### ACS - 智能体能力描述

ACS (Agent Capability Specification) 完整描述智能体的身份信息、功能能力、技术特性和服务接口。

主要包含：
- 身份信息 (aic, active, lastModifiedTime)
- 基本描述 (name, description, version)
- 提供者信息 (provider)
- 安全方案 (securitySchemes)
- 服务端点 (endPoints)
- 技术能力 (capabilities)
- 技能列表 (skills)

### AIP - 智能体交互协议

AIP (Agent Interaction Protocol) 定义了智能体之间的交互模式：

- **任务命令** (TaskCommand): Leader 向 Partner 发送的命令
- **任务结果** (TaskResult): Partner 向 Leader 返回的结果
- **任务状态** (TaskState): accepted, working, completed, failed 等
- **数据项** (DataItem): text, file, structured data

### ADP - 智能体发现协议

ADP (Agent Discovery Protocol) 定义了智能体发现流程：

- **发现请求** (DiscoveryRequest): 包含查询、过滤条件、转发配置
- **发现响应** (DiscoveryResponse): 返回匹配的智能体列表和 ACS 数据
- **过滤条件** (DiscoveryFilter): 支持丰富的运算符和逻辑组合
- **转发机制**: 支持跨域发现和链式转发

## 技术特性

### Rust 实现优势

- ✅ **类型安全**: 利用 Rust 类型系统确保协议数据的正确性
- ✅ **零成本抽象**: 高性能的协议处理
- ✅ **内存安全**: 无需 GC，编译时保证内存安全
- ✅ **并发安全**: 利用 async/await 和 tokio 实现高效并发
- ✅ **错误处理**: 使用 Result 和 thiserror 提供清晰的错误信息

### 协议实现亮点

- ✅ **完整的序列化支持**: 使用 serde 实现 JSON 序列化/反序列化
- ✅ **Builder 模式**: 提供流畅的 API 构建体验
- ✅ **验证逻辑**: 内置数据验证，确保协议合规性
- ✅ **详细文档**: 每个模块都有详细的文档和示例
- ✅ **测试覆盖**: 核心功能都有完整的测试覆盖

## 后续规划

### 短期目标

1. 完善 Discovery 服务实现
   - 实现具体的存储后端（PostgreSQL + Redis）
   - 实现关键词匹配器
   - 实现 HTTP API 接口

2. 完善 Registry 服务实现
   - 实现具体的存储后端
   - 实现 EAB 凭据验证
   - 实现 HTTP API 接口

### 中期目标

3. 完善 Certifier 服务实现
   - 集成 OpenSSL 或 rustls
   - 实现 ACME 协议
   - 实现证书颁发和吊销

4. 完善 Authorize 服务实现
   - 实现 ACL 规则引擎
   - 实现群组管理
   - 实现消息队列认证后端

5. 向量相似度匹配
   - 集成向量数据库（如 Qdrant/Milvus）
   - 实现语义相似度匹配

### 长期目标

6. gRPC 支持
7. 消息队列集成（RabbitMQ, Redis Streams, Kafka）
8. 监控和可观测性（Prometheus, OpenTelemetry）
9. 部署和运维支持（Docker, Kubernetes）

## 无法直接转换的部分

以下部分由于 Rust 的限制，使用了伪代码和详细注释：

1. **动态类型处理**: 使用 `serde_json::Value` + 注释说明
2. **异步生成器**: 使用 `Stream` trait + 注释说明
3. **元编程**: 使用过程宏 + 注释说明

## 参考资源

- [GB/Z 185《人工智能 智能体互联》国家标准](https://std.samr.gov.cn/gb/search/gbDetailed?id=52A05F8113453761E06397BE0A0AFEB5)
- [ACPs-community Python 实现](https://atomgit.com/AIP-PUB/ACPs-community)
- [ACPs 协议规范文档](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs)

## 许可证

[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)