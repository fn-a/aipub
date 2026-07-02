# Protocols - AIP 核心协议库

基于 ACPs (Agent Collaboration Protocols) 规范实现的 Rust 核心协议库。

## 模块结构

### AIC - Agent Identity Code (智能体身份码)

智能体全局唯一身份标识符，采用 10 级 OID 结构。

```rust
use aipub::protocols::aic::{validate_aic_format, parse_aic, AicInfo, AicValidator};

// 基础格式验证
let aic = "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353";
if let Ok(info) = validate_aic_format(aic) {
    println!("AIC 有效");
    println!("是否为本体: {}", info.is_ontology());
    println!("ARSP: {}", info.arsp);
}

// 完整验证（包含 CRC）
let validator = AicValidator::from_hex("1234").unwrap();
if validator.validate(aic, true) {
    println!("AIC 完全有效");
}
```

**AIC 结构说明：**

```
1.2.156.3088.{version}.{arsp}.{vendor}.{ontology_sn}.{instance_sn}.{crc}
```

- 1-4级: OID 前缀 `1.2.156.3088` (由国家 OID 注册中心分配)
- 5级: 版本号 (Base36, 1位)
- 6级: ARSP 注册服务商序号 (Base36, 1-6位)
- 7级: Vendor 供应商序号 (Base36, 1-6位)
- 8级: Ontology SN 本体序列号 (Base36, 1-9位)
- 9级: Instance SN 实例序列号 (Base36, 1-9位, 全0表示本体AIC)
- 10级: CRC校验码 (Base36, 固定4位)

### ACS - Agent Capability Specification (智能体能力描述)

完整描述智能体的身份信息、功能能力、技术特性和服务接口。

```rust
use aipub::protocols::acs::{
    AgentCapabilitySpec, AgentProvider, AgentCapabilities,
    AgentSkill, SecurityScheme, MutualTlsSecurityScheme
};
use std::collections::HashMap;

let mut security_schemes = HashMap::new();
security_schemes.insert(
    "mtls".to_string(),
    SecurityScheme::MutualTls(MutualTlsSecurityScheme {
        r#type: "mutualTLS".to_string(),
        description: Some("双向TLS认证".to_string()),
        x_ca_challenge_base_url: None,
    }),
);

let acs = AgentCapabilitySpec {
    aic: "1.2.156.3088.1.K9.12345.E789ABCDEF.000000000.2353".to_string(),
    active: true,
    last_modified_time: "2025-03-15T16:30:00+08:00".to_string(),
    protocol_version: "02.01".to_string(),
    name: "北京旅游助手".to_string(),
    description: "提供北京旅游规划服务".to_string(),
    version: "1.0.0".to_string(),
    provider: AgentProvider {
        country_code: Some("CN".to_string()),
        organization: Some("ACPs工作组".to_string()),
        ..Default::default()
    },
    security_schemes,
    capabilities: AgentCapabilities {
        streaming: true,
        notification: false,
        message_queue: vec![],
    },
    ..Default::default()
};

// 验证 ACS
acs.validate()?;

// 序列化
let json = acs.to_json()?;
```

**主要字段：**

- `aic`: 智能体身份码
- `active`: 激活状态
- `name`: 智能体名称
- `description`: 智能体描述
- `version`: 版本号
- `provider`: 提供者信息
- `security_schemes`: 安全方案
- `end_points`: 服务端点
- `capabilities`: 技术能力
- `skills`: 技能列表

### AIP - Agent Interaction Protocol (智能体交互协议)

定义智能体之间的交互模式，包括任务命令、任务结果等。

```rust
use aipub::protocols::aip::{
    TaskCommand, TaskResult, TaskState, TaskCommandType,
    DataItem, TextDataItem
};

// 创建 Start 命令
let cmd = TaskCommand::start(
    "cmd-001".to_string(),
    "2025-03-15T16:30:00+08:00".to_string(),
    "leader-001".to_string(),
);

// 创建任务结果
let status = TaskStatus {
    state: TaskState::Working,
    state_changed_at: "2025-03-15T16:30:00+08:00".to_string(),
    data_items: None,
};

let result = TaskResult::new(
    "result-001".to_string(),
    "2025-03-15T16:30:00+08:00".to_string(),
    "partner-001".to_string(),
    "task-001".to_string(),
    status,
);
```

**任务状态：**

- `Accepted`: 已接受
- `Working`: 工作中
- `AwaitingInput`: 等待输入
- `Completed`: 已完成
- `Failed`: 失败
- `Canceled`: 已取消

**命令类型：**

- `Start`: 启动任务
- `Get`: 获取状态
- `Continue`: 继续任务
- `Cancel`: 取消任务
- `Complete`: 完成任务

### ADP - Agent Discovery Protocol (智能体发现协议)

定义智能体发现流程，支持查询、过滤和跨域发现。

```rust
use aipub::protocols::adp::{
    DiscoveryRequest, DiscoveryResponse, DiscoveryFilter,
    FilterCondition, QueryType
};
use serde_json::json;

// 创建发现请求
let request = DiscoveryRequest::new()
    .with_query("我需要一个北京美食推荐的智能体")
    .with_limit(10)
    .with_filter(
        DiscoveryFilter::and(vec![
            FilterCondition::eq("active", json!(true)),
            FilterCondition::contains("name", "北京"),
        ])
    );

// 处理响应
if let Some(result) = response.result {
    for agent_group in &result.agents {
        for agent_skill in &agent_group.agent_skills {
            println!("发现智能体: {} - 技能: {}", 
                agent_skill.aic, 
                agent_skill.skill_id
            );
        }
    }
}
```

**查询类型：**

- `explicit`: 明确查询（有明确意图）
- `exploratory`: 探索性查询
- `trending`: 热门查询
- `filtered`: 过滤查询

**过滤运算符：**

- 等值: `eq`, `ne`
- 比较: `gt`, `gte`, `lt`, `lte`, `between`
- 集合: `in`, `nin`
- 字符串: `contains`, `startsWith`, `endsWith`
- 数组: `anyOf`, `allOf`, `noneOf`, `size`

## 特性

- ✅ 完整的 AIC 验证和解析（包含 Base36 编解码和 CRC 校验）
- ✅ ACS 数据模型和验证
- ✅ AIP 任务交互模型
- ✅ ADP 发现协议和过滤条件
- ✅ 完整的序列化/反序列化支持
- ✅ 类型安全的 Rust 实现

## 参考

- [ACPs-spec-AIC-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/02-ACPs-spec-AIC)
- [ACPs-spec-ACS-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/03-ACPs-spec-ACS)
- [ACPs-spec-AIP-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/07-ACPs-spec-AIP)
- [ACPs-spec-ADP-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/06-ACPs-spec-ADP)
