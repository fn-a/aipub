# Protocols - AIP 核心协议库

基于 ACPs (Agent Collaboration Protocols) 规范实现的 Rust 核心协议库。

## 模块结构

### AIC - Agent Identity Code (智能体身份码)

智能体全局唯一身份标识符，采用 10 级 OID 结构。

```rust
use aipub::protocols::aic::{validate_aic_format, AicValidator, AicGenerator};

// 生成 AIC（SALT 由各 ARSP 内部维护，此处示例为 0x00494F41）
let generator = AicGenerator::from_hex_salt("0x00494F41").unwrap();
let aic = generator.generate_entity_aic().unwrap();

// 格式验证（不需要 SALT）
let info = validate_aic_format(&aic).unwrap();
println!("是否为本体: {}", info.is_ontology());
println!("ARSP: {}", info.arsp);

// 完整验证（含 CRC-16/CCITT-FALSE 校验）
let mut validator = AicValidator::new(Some(generator.salt().to_vec()));
assert!(validator.validate(&aic, true));
```

> 文档中出现的 AIC 字面量均按 SALT `0x00494F41` 计算校验码，可直接通过完整校验；
> 例如本体 AIC `1.2.156.3088.1.0001.00001.ABC123.000000.0M9Y`、
> 实体 AIC `1.2.156.3088.1.0001.00001.ABC123.000001.176F`。

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
use std::collections::HashMap;
use aipub::protocols::acs::{
    AgentCapabilitySpec, AgentProvider, AgentCapabilities,
    AgentSkill, SecurityScheme, MutualTlsSecurityScheme
};

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
    aic: "1.2.156.3088.1.0001.00001.ABC123.000000.0M9Y".to_string(),
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
use serde_json::json;
use aipub::protocols::adp::{
    DiscoveryRequest, DiscoveryResponse, DiscoveryFilter,
    FilterCondition, QueryType
};

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

**过滤运算符（共 35 种）：**

- 等值/存在: `eq`, `ne`, `exists`
- 比较: `gt`, `gte`, `lt`, `lte`, `between`（闭区间）
- 集合: `in`, `nin`
- 字符串: `contains`, `notContains`, `startsWith`, `endsWith`（默认大小写不敏感）
- 大小写敏感变体: `eqCs`, `neCs`, `inCs`, `ninCs`, `containsCs`, `notContainsCs`, `startsWithCs`, `endsWithCs`
- 数组: `anyOf`, `allOf`, `noneOf`, `size`, `sizeGt`, `sizeGte`, `sizeLt`, `sizeLte`
- 对象: `hasKey`, `hasNoKey`, `hasAnyKey`, `hasAllKeys`

求值语义：

- 字段路径支持点号嵌套（如 `skills.tags`），遇到数组时对每个元素求值，任一命中即为命中；
- 否定运算符（`ne`/`nin`/`notContains`/`noneOf`/`hasNoKey`）定义为对应正向运算符取反，
  因此在数组字段上表示「没有任何元素命中」；
- `size*` 直接作用于数组/字符串/对象本身；`exists` 表示「存在且非空」。

### 其它模块

- **ATR** (`protocols::atr`)：EAB 的 HMAC-SHA256 签名/校验、吊销原因码、
  证书用途与有效期常量
- **DSP** (`protocols::dsp`)：变更记录（`ChangeEntry`）与变更流（`ChangeFeed`），
  供注册服务到发现服务的增量同步

## 特性

- ✅ AIC 生成与解析（Base36 编解码、CRC-16/CCITT-FALSE 校验、本体/实体派生）
- ✅ ACS 数据模型与校验（protocolVersion、技能、mutualTLS 端点、URL scheme、MQ 版本正则）
- ✅ AIP 任务交互模型（任务状态机、TaskManager、JSON-RPC 请求/响应与错误码）
- ✅ ADP 发现协议（请求校验、35 种过滤运算符求值、错误码、转发链/扇出校验）
- ✅ ATR 支撑算法（EAB HMAC）与 DSP 变更流
- ✅ 完整的序列化/反序列化支持
- ✅ 类型安全的 Rust 实现

## 参考

- [ACPs-spec-AIC-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/02-ACPs-spec-AIC)
- [ACPs-spec-ACS-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/03-ACPs-spec-ACS)
- [ACPs-spec-AIP-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/07-ACPs-spec-AIP)
- [ACPs-spec-ADP-v02.01](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs/06-ACPs-spec-ADP)
