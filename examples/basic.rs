//! AIP 智能体互联协议使用示例

use aipub::{
    protocols::{
        aic::{validate_aic_format, parse_aic, AicValidator},
        acs::{AgentCapabilitySpec, AgentProvider, AgentCapabilities, AgentSkill, SecurityScheme, MutualTlsSecurityScheme},
        aip::{TaskCommand, TaskResult, TaskState, TaskStatus},
        adp::{DiscoveryRequest, DiscoveryResponse, DiscoveryFilter, FilterCondition},
    },
    PROTOCOL_VERSION, AIC_OID_PREFIX,
};
use std::collections::HashMap;

fn main() {
    println!("AIP 智能体互联协议 v{}", aipub::VERSION);
    println!("协议版本: {}", PROTOCOL_VERSION);
    println!("AIC OID 前缀: {}", AIC_OID_PREFIX);
    println!();
    
    // 示例 1: AIC 验证和解析
    example_aic();
    
    // 示例 2: ACS 创建和序列化
    example_acs();
    
    // 示例 3: AIP 任务交互
    example_aip();
    
    // 示例 4: ADP 发现查询
    example_adp();
}

/// AIC 示例
fn example_aic() {
    println!("=== AIC 示例 ===");
    
    let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
    
    // 验证格式
    match validate_aic_format(aic) {
        Ok(info) => {
            println!("✓ AIC 格式有效");
            println!("  - 版本: {}", info.version);
            println!("  - ARSP: {}", info.arsp);
            println!("  - Vendor: {}", info.vendor);
            println!("  - 是否为本体: {}", info.is_ontology());
            println!("  - 本体前缀: {}", info.ontology_prefix());
        }
        Err(e) => {
            println!("✗ AIC 格式无效: {}", e);
        }
    }
    
    // 完整验证（需要 SALT）
    let validator = AicValidator::from_hex("1234").unwrap();
    println!("  - 验证器已配置 SALT: {}", validator.has_salt());
    
    println!();
}

/// ACS 示例
fn example_acs() {
    println!("=== ACS 示例 ===");
    
    // 创建安全方案
    let mut security_schemes = HashMap::new();
    security_schemes.insert(
        "mtls".to_string(),
        SecurityScheme::MutualTls(MutualTlsSecurityScheme {
            r#type: "mutualTLS".to_string(),
            description: Some("双向TLS认证".to_string()),
            x_ca_challenge_base_url: None,
        }),
    );
    
    // 创建 ACS
    let acs = AgentCapabilitySpec {
        aic: "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD".to_string(),
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
        skills: vec![AgentSkill {
            id: "beijing-tour.recommend".to_string(),
            name: "北京景点推荐".to_string(),
            description: "推荐北京城区旅游景点".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["旅游".to_string(), "北京".to_string(), "推荐".to_string()],
            examples: None,
            input_modes: None,
            output_modes: None,
        }],
        icon_url: None,
        documentation_url: None,
        web_app_url: None,
        end_points: vec![],
        default_input_modes: vec![],
        default_output_modes: vec![],
        entity_user_id: None,
        entity_meta: None,
    };
    
    // 验证 ACS
    match acs.validate() {
        Ok(()) => println!("✓ ACS 验证通过"),
        Err(e) => println!("✗ ACS 验证失败: {}", e),
    }
    
    // 序列化
    match acs.to_json() {
        Ok(json) => {
            println!("✓ ACS 序列化成功 ({} bytes)", json.len());
            println!("  - 支持流式: {}", acs.supports_streaming());
            println!("  - 支持通知: {}", acs.supports_notification());
            println!("  - 技能数量: {}", acs.skills.len());
        }
        Err(e) => println!("✗ 序列化失败: {}", e),
    }
    
    println!();
}

/// AIP 示例
fn example_aip() {
    println!("=== AIP 示例 ===");
    
    // 创建 Start 命令
    let cmd = TaskCommand::start(
        "cmd-001".to_string(),
        "2025-03-15T16:30:00+08:00".to_string(),
        "leader-001".to_string(),
    );
    
    println!("✓ 创建任务命令:");
    println!("  - ID: {}", cmd.id);
    println!("  - 类型: {:?}", cmd.command);
    println!("  - 发送者: {}", cmd.sender_id);
    
    // 创建任务结果
    let status = TaskStatus {
        state: TaskState::Working,
        state_changed_at: "2025-03-15T16:30:01+08:00".to_string(),
        data_items: None,
    };
    
    let result = TaskResult::new(
        "result-001".to_string(),
        "2025-03-15T16:30:01+08:00".to_string(),
        "partner-001".to_string(),
        "task-001".to_string(),
        status,
    );
    
    println!("✓ 创建任务结果:");
    println!("  - 任务ID: {}", result.task_id);
    println!("  - 状态: {:?}", result.status.state);
    println!("  - 是否完成: {}", result.is_completed());
    
    println!();
}

/// ADP 示例
fn example_adp() {
    println!("=== ADP 示例 ===");
    
    // 创建发现请求
    let request = DiscoveryRequest::new()
        .with_query("我需要一个可以做北京美食推荐的智能体")
        .with_limit(10)
        .with_filter(
            DiscoveryFilter::and(vec![
                FilterCondition::eq("active", serde_json::json!(true)),
                FilterCondition::contains("name", "北京"),
            ])
        );
    
    println!("✓ 创建发现请求:");
    println!("  - 查询: {:?}", request.query);
    println!("  - 限制: {:?}", request.limit);
    println!("  - 过滤条件: {}", request.filter.is_some());
    
    // 创建响应
    let response = DiscoveryResponse::success(Default::default());
    
    println!("✓ 创建发现响应:");
    println!("  - 是否成功: {}", response.is_success());
    
    println!();
}
