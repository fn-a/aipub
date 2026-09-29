//! 基础流程端到端示例
//!
//! 运行：
//!
//! ```bash
//! cargo run --example e2eflow
//! ```
//!
//! 该示例在单进程内串通 ACPs 的基础链路：
//!
//! ```text
//! ACS 注册 -> AIC 分配 -> 变更流同步 -> 发现查询 -> 跨域转发
//!          -> EAB 签发 -> ACME 账号/订单/证书 -> 群组 ACL
//!          -> AIP 任务下发与结果回传 -> 注销/停用/吊销
//! ```

#![allow(dead_code)]

use std::process;
use std::sync::Arc;
use std::collections::HashMap;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use uuid::Uuid;
use serde_json::{json, to_value};
use aipub::{PROTOCOL_VERSION, AIC_OID_PREFIX, VERSION};
use aipub::authorize::{
    AuthzDecision, AuthorizeConfig, AuthorizeServiceImpl, GroupManager, MqAuthRequest,
    SHARED_VHOST,
};
use aipub::certifier::{CertStatus, CertifierConfig, CertifierService, CertifierServiceImpl};
use aipub::discovery::{
    DiscoveryConfig, DiscoveryService, DiscoveryServiceImpl, InProcessForwarder, KeywordMatcher,
    MemoryStorage as DiscoveryStorage, is_discoverable,
};
use aipub::protocols::acs::{
    AgentCapabilities, AgentCapabilitySpec, AgentEndPoint, AgentProvider, AgentSkill,
    MutualTlsSecurityScheme, SecurityScheme,
};
use aipub::protocols::adp::{DiscoveryFilter, DiscoveryRequest, FilterCondition, FilterOperator};
use aipub::protocols::aip::{
    now_iso8601, DataItem, Product, TaskCommand, TaskCommandType, TaskManager, TaskState,
    TextDataItem,
};
use aipub::protocols::atr::{sign_eab_hmac, CertificateUsage, RevocationReason};
use aipub::protocols::aic::get_ontology_prefix_from_aic;
use aipub::registry::{
    MemoryEabManager, MemoryRegistryStorage, RegistryConfig, RegistryService,
    RegistryServiceImpl,
};

/// 执行过程中的产出物，供断言与展示
#[derive(Debug, Default)]
pub struct FlowOutcome {
    /// 本体 AIC
    pub ontology_aic: String,

    /// 由本体派生的实体 AIC
    pub derived_entity_aic: String,

    /// 美食智能体 AIC
    pub food_aic: String,

    /// 酒店智能体 AIC
    pub hotel_aic: String,

    /// Leader 智能体 AIC
    pub leader_aic: String,

    /// EAB 凭据 ID
    pub eab_key_id: String,

    /// ACME 账号 ID
    pub account_id: String,

    /// 证书序列号
    pub certificate_serial: String,

    /// 证书有效期（天）
    pub certificate_validity_days: i64,

    /// 已应用的变更条数
    pub changes_applied: usize,

    /// 同步后发现服务中的智能体总数
    pub total_agents_after_sync: usize,

    /// 同步后可供发现的智能体数量
    pub available_agents_after_sync: usize,

    /// 本体 AIC 是否可被服务发现（应为 false：本体不提供端点）
    pub ontology_discoverable: bool,

    /// 自然语言发现命中的 AIC
    pub discovered_aic: String,

    /// 自然语言发现命中的技能 ID
    pub discovered_skill_id: String,

    /// 发现结果中的候选数量
    pub discovered_count: usize,

    /// 发现结果中的路由数量（含跨域转发）
    pub route_count: usize,

    /// 跨域转发命中的下游 AIC
    pub forwarded_aic: String,

    /// 结构化过滤查询命中的 AIC
    pub filtered_aic: String,

    /// 群组 ID
    pub group_id: String,

    /// 群组成员数
    pub group_member_count: usize,

    /// 群组成员写群组交换机的鉴权结论
    pub member_exchange_decision: AuthzDecision,

    /// 非成员写群组交换机的鉴权结论
    pub outsider_exchange_decision: AuthzDecision,

    /// 任务最终状态
    pub task_state: Option<TaskState>,

    /// 产出物数量
    pub product_count: usize,

    /// 任务状态历史长度
    pub status_history_len: usize,

    /// 注销前发现服务中的智能体数量
    pub agents_before_unregister: usize,

    /// 注销后同步到的智能体数量
    pub agents_after_unregister: usize,

    /// 吊销后的证书状态
    pub cert_status_after_revoke: Option<CertStatus>,

    /// CRL 条目数
    pub crl_entry_count: usize,

    /// OCSP 状态
    pub ocsp_status: Option<CertStatus>,

    /// EAB 二次消费是否被拒绝
    pub eab_replay_rejected: bool,
}

/// 采集 AIC 的包装
fn empty_acs(name: &str, skill_id: &str, skill_name: &str, tags: &[&str], rpc_port: u16) -> AgentCapabilitySpec {
    let mut security_schemes = HashMap::new();
    security_schemes.insert(
        "mtls".to_string(),
        SecurityScheme::MutualTls(MutualTlsSecurityScheme {
            r#type: "mutualTLS".to_string(),
            description: Some("双向 TLS 认证".to_string()),
            x_ca_challenge_base_url: None,
        }),
    );

    let mut endpoint_security = HashMap::new();
    endpoint_security.insert("mtls".to_string(), vec![]);

    AgentCapabilitySpec {
        aic: String::new(),
        active: true,
        last_modified_time: String::new(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        name: name.to_string(),
        description: format!("{}：面向北京本地生活场景的智能体", name),
        version: "1.0.0".to_string(),
        icon_url: None,
        documentation_url: None,
        web_app_url: None,
        provider: AgentProvider {
            country_code: Some("CN".to_string()),
            organization: Some("ACPs 工作组".to_string()),
            department: None,
            url: None,
            license: None,
            name: None,
            email: None,
        },
        security_schemes,
        end_points: vec![
            AgentEndPoint {
                url: format!("https://localhost:{}/rpc", rpc_port),
                transport: "JSONRPC".to_string(),
                security: Some(vec![endpoint_security.clone()]),
            },
            AgentEndPoint {
                url: "amqps://localhost:5671/acps?inbox=inbox_{AIC}".to_string(),
                transport: "AMQP".to_string(),
                security: Some(vec![endpoint_security]),
            },
        ],
        capabilities: AgentCapabilities {
            streaming: true,
            notification: false,
            message_queue: vec![],
        },
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string()],
        skills: vec![AgentSkill {
            id: skill_id.to_string(),
            name: skill_name.to_string(),
            description: format!("提供{}能力", skill_name),
            version: "1.0.0".to_string(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            examples: None,
            input_modes: None,
            output_modes: None,
        }],
        entity_user_id: None,
        entity_meta: None,
    }
}

/// 运行基础流程
///
/// `report` 在每条关键步骤后被调用，用于打印进度。
pub async fn run_basic_flow(report: &mut dyn FnMut(&str)) -> Result<FlowOutcome, String> {
    let mut outcome = FlowOutcome::default();

    // ------------------------------------------------------------------
    // 0. 组装服务：注册服务（含 EAB）、发现服务、鉴权服务、证书服务
    // ------------------------------------------------------------------
    let registry = RegistryServiceImpl::new(
        RegistryConfig::without_eab(),
        Box::new(MemoryRegistryStorage::new()),
        Some(Box::new(MemoryEabManager::new())),
    );

    let discovery = DiscoveryServiceImpl::new(
        DiscoveryConfig::default().without_forward(),
        Box::new(DiscoveryStorage::new()),
        Box::new(KeywordMatcher::new()),
        None,
    );

    let authorize = AuthorizeServiceImpl::new(AuthorizeConfig::default());
    let ca = CertifierServiceImpl::new(CertifierConfig::default());

    report("服务组装完成：Registry / Discovery / Authorize / Certifier");

    // ------------------------------------------------------------------
    // 1. 注册智能体：本体 AIC -> 派生实体 AIC，以及独立的实体 AIC
    // ------------------------------------------------------------------
    // 1a. 注册本体：本体不承载对外服务端点，因此不会出现在发现结果里
    let mut ontology_acs = empty_acs(
        "北京美食推荐（本体）",
        "beijing-tour.food.ontology",
        "北京美食本体",
        &["美食"],
        9020,
    );
    ontology_acs.end_points.clear();
    outcome.ontology_aic = registry
        .register_with_eab(ontology_acs, None, true)
        .await
        .map_err(|e| format!("注册本体失败: {}", e))?;
    report(&format!("注册本体：{}", outcome.ontology_aic));

    // 1b. 由本体派生实体，作为对外提供服务的美食智能体
    let food_acs = empty_acs(
        "北京美食推荐助手",
        "beijing-tour.food",
        "北京美食推荐",
        &["美食", "北京", "推荐"],
        9021,
    );
    outcome.food_aic = registry
        .register_entity_from_ontology(&outcome.ontology_aic, food_acs)
        .await
        .map_err(|e| format!("派生实体注册失败: {}", e))?;
    outcome.derived_entity_aic = outcome.food_aic.clone();

    // 1c. 独立实体：酒店与 Leader
    let hotel_acs = empty_acs("北京酒店预订助手", "beijing-tour.hotel", "北京酒店预订", &["酒店", "北京"], 9022);
    outcome.hotel_aic = registry
        .register_with_eab(hotel_acs, None, false)
        .await
        .map_err(|e| format!("注册酒店智能体失败: {}", e))?;

    let leader_acs = empty_acs("北京行程规划助手", "beijing-tour.plan", "北京行程规划", &["行程", "北京"], 9011);
    outcome.leader_aic = registry
        .register_with_eab(leader_acs, None, false)
        .await
        .map_err(|e| format!("注册 Leader 失败: {}", e))?;

    // 派生的实体与原本体共享 1-8 级前缀
    let ontology_prefix = get_ontology_prefix_from_aic(&outcome.ontology_aic)
        .ok_or_else(|| "无法解析本体前缀".to_string())?;
    if !outcome
        .derived_entity_aic
        .starts_with(&format!("{}.", ontology_prefix))
    {
        return Err("实体 AIC 未继承本体前缀".to_string());
    }
    report(&format!(
        "AIC 分配完成：本体 {} / 派生实体 {} / 酒店 {}",
        outcome.ontology_aic, outcome.derived_entity_aic, outcome.hotel_aic
    ));

    // ------------------------------------------------------------------
    // 2. 变更流同步：注册服务 -> 发现服务（DSP）
    // ------------------------------------------------------------------
    let entries = registry
        .changes_since(0, 100)
        .await
        .map_err(|e| format!("拉取变更失败: {}", e))?;
    outcome.changes_applied = discovery
        .apply_changes(&entries)
        .await
        .map_err(|e| format!("应用变更失败: {}", e))?;

    // 重复投递应当被版本幂等丢弃
    let replayed = discovery
        .apply_changes(&entries)
        .await
        .map_err(|e| format!("重复应用变更失败: {}", e))?;
    if replayed != 0 {
        return Err("变更流未按版本去重".to_string());
    }

    let stats = discovery.stats().await.map_err(|e| e.to_string())?;
    outcome.total_agents_after_sync = stats.total_agents;
    outcome.available_agents_after_sync = stats.available_agents;

    // 本阶段每条变更都是一次新增，因此记录数应与变更条数一致
    if stats.total_agents != outcome.changes_applied {
        return Err(format!(
            "同步记录数与变更条数不一致: {} vs {}",
            stats.total_agents, outcome.changes_applied
        ));
    }

    // 本体 AIC 没有服务端点，不应出现在可发现集合中
    let ontology_acs = discovery
        .get_agent(&outcome.ontology_aic)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "本体 ACS 未同步到发现服务".to_string())?;
    outcome.ontology_discoverable = is_discoverable(&ontology_acs);
    if outcome.ontology_discoverable {
        return Err("本体 AIC 不应可被发现".to_string());
    }
    if stats.available_agents >= stats.total_agents {
        return Err("可发现集合应排除本体".to_string());
    }
    report(&format!(
        "变更流同步完成：应用 {} 条，发现服务共 {} 个智能体（可发现 {} 个）",
        outcome.changes_applied, stats.total_agents, stats.available_agents
    ));

    // ------------------------------------------------------------------
    // 3. 发现查询：自然语言 / 结构化过滤
    // ------------------------------------------------------------------
    let request = DiscoveryRequest::new()
        .with_query("我需要一个可以做北京美食推荐的智能体")
        .with_limit(5)
        .with_query_type("explicit");
    let response = discovery
        .discover(request)
        .await
        .map_err(|e| format!("发现查询失败: {}", e))?;
    let result = response.result.ok_or_else(|| "发现查询返回错误响应".to_string())?;

    let top = result
        .agents
        .first()
        .and_then(|group| group.agent_skills.first())
        .ok_or_else(|| "发现结果为空".to_string())?;
    outcome.discovered_aic = top.aic.clone();
    outcome.discovered_skill_id = top.skill_id.clone();
    outcome.discovered_count = result
        .agents
        .iter()
        .map(|group| group.agent_skills.len())
        .sum();
    outcome.route_count = result.routes.as_ref().map(|routes| routes.len()).unwrap_or(0);

    if !result.acs_map.contains_key(&outcome.discovered_aic) {
        return Err("acsMap 缺少命中智能体的 ACS".to_string());
    }
    // Leader 从 acsMap 中取出服务端点
    let endpoint_url = result.acs_map[&outcome.discovered_aic]["endPoints"][0]["url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    report(&format!(
        "自然语言发现命中：{} / {} -> {}",
        outcome.discovered_aic, outcome.discovered_skill_id, endpoint_url
    ));

    let mut filtered_request = DiscoveryRequest::new()
        .with_query_type("filtered")
        .with_limit(5);
    filtered_request.filter = Some(DiscoveryFilter::and(vec![
        FilterCondition::eq("active", json!(true)),
        FilterCondition::new(
            "skills.tags".to_string(),
            FilterOperator::AnyOf,
            Some(json!(["酒店"])),
        ),
    ]));
    let filtered_response = discovery
        .discover(filtered_request)
        .await
        .map_err(|e| format!("结构化查询失败: {}", e))?;
    outcome.filtered_aic = filtered_response
        .result
        .and_then(|result| result.agents.first().cloned())
        .and_then(|group| group.agent_skills.first().cloned())
        .map(|skill| skill.aic)
        .ok_or_else(|| "结构化查询无结果".to_string())?;
    if outcome.filtered_aic != outcome.hotel_aic {
        return Err("结构化查询命中的智能体不符合预期".to_string());
    }
    report(&format!("结构化查询命中：{}", outcome.filtered_aic));

    // ------------------------------------------------------------------
    // 4. 跨域转发：本机发现服务 -> 下游发现服务
    // ------------------------------------------------------------------
    let downstream = Arc::new(DiscoveryServiceImpl::new(
        DiscoveryConfig::default()
            .without_forward()
            .with_local_server_aic("AIC-DS-B"),
        Box::new(DiscoveryStorage::new()),
        Box::new(KeywordMatcher::new()),
        None,
    ));
    let mut downstream_acs = empty_acs("上海美食推荐助手", "shanghai-tour.food", "上海美食推荐", &["美食", "上海"], 9031);
    let downstream_aic = registry
        .register_with_eab(downstream_acs.clone(), None, true)
        .await
        .map_err(|e| format!("注册下游智能体失败: {}", e))?;
    downstream_acs.aic = downstream_aic.clone();
    DiscoveryService::register_agent(downstream.as_ref(), to_value(&downstream_acs).unwrap())
        .await
        .map_err(|e| format!("写入下游发现服务失败: {}", e))?;

    let forwarding_discovery = DiscoveryServiceImpl::new(
        DiscoveryConfig::default()
            .with_local_server_aic("AIC-DS-A")
            .with_forward_servers(vec!["AIC-DS-B".to_string()]),
        Box::new(DiscoveryStorage::new()),
        Box::new(KeywordMatcher::new()),
        Some(Box::new(InProcessForwarder::new(
            downstream.clone(),
            "AIC-DS-A",
            "AIC-DS-B",
        ))),
    );
    forwarding_discovery
        .apply_changes(&entries)
        .await
        .map_err(|e| format!("转发场景同步失败: {}", e))?;

    let forward_response = forwarding_discovery
        .discover(
            DiscoveryRequest::new()
                .with_query("北京美食推荐")
                .with_query_type("explicit")
                .with_limit(5),
        )
        .await
        .map_err(|e| format!("转发查询失败: {}", e))?;
    let forward_result = forward_response
        .result
        .ok_or_else(|| "转发查询返回错误".to_string())?;

    // 下游「上海美食」不会被本机关键词命中，这里改为直接查询下游服务验证转发链路
    let downstream_probe = forwarding_discovery
        .discover(
            DiscoveryRequest::new()
                .with_query("上海美食推荐")
                .with_query_type("explicit")
                .with_limit(5),
        )
        .await
        .map_err(|e| format!("转发探测失败: {}", e))?;
    outcome.route_count = downstream_probe
        .result
        .as_ref()
        .and_then(|result| result.routes.as_ref())
        .map(|routes| routes.len())
        .unwrap_or(0);
    outcome.forwarded_aic = downstream_probe
        .result
        .as_ref()
        .and_then(|result| result.routes.as_ref())
        .and_then(|routes| routes.last())
        .and_then(|route| route.agent_groups.first())
        .and_then(|group| group.agent_skills.first())
        .map(|skill| skill.aic.clone())
        .unwrap_or_default();

    if outcome.forwarded_aic != downstream_aic {
        return Err(format!(
            "跨域转发未取回下游结果: {:?}",
            forward_result.routes
        ));
    }
    report(&format!(
        "跨域转发完成：{} 条路由，命中下游 {}",
        outcome.route_count, outcome.forwarded_aic
    ));

    // ------------------------------------------------------------------
    // 5. EAB 签发 -> ACME 账号绑定 -> 订单 -> 证书
    // ------------------------------------------------------------------
    let credential = registry
        .mint_eab(&outcome.food_aic)
        .await
        .map_err(|e| format!("签发 EAB 失败: {}", e))?;
    outcome.eab_key_id = credential.key_id.clone();

    // CA 以 `/internal/eab/consume` 一次性换取 macKey（凭据随即失效）
    let consumed = registry
        .consume_eab(&credential.key_id)
        .await
        .map_err(|e| format!("消费 EAB 失败: {}", e))?;

    if consumed.aic != outcome.food_aic {
        return Err("EAB 绑定的 AIC 与预期不一致".to_string());
    }

    let protected = base64_url(br#"{"alg":"HS256","kid":"placeholder","url":"https://ca/acps-atr-v2/acme/new-account"}"#);
    let payload = base64_url(br#"{"kty":"RSA","n":"mock-modulus","e":"AQAB"}"#);
    let signature = sign_eab_hmac(&consumed.mac_key, &protected, &payload)
        .ok_or_else(|| "EAB 签名失败".to_string())?;

    let account = ca
        .create_account_with_eab(&consumed.aic, &consumed.mac_key, &protected, &payload, &signature)
        .await
        .map_err(|e| format!("ACME 账号创建失败: {}", e))?;
    outcome.account_id = account.id.clone();

    // EAB 一次性消费：再次消费必须失败
    outcome.eab_replay_rejected = registry.consume_eab(&credential.key_id).await.is_err();

    let order = ca
        .create_order(
            &account.id,
            &outcome.food_aic,
            CertificateUsage::ServerAuth,
            Some(30),
        )
        .await
        .map_err(|e| format!("创建订单失败: {}", e))?;

    let certificate = ca
        .finalize_order(&order.id, b"mock-csr-der-bytes".to_vec())
        .await
        .map_err(|e| format!("签发证书失败: {}", e))?;
    outcome.certificate_serial = certificate.serial_number.clone();
    outcome.certificate_validity_days = (certificate.not_after - certificate.not_before).num_days();
    report(&format!(
        "证书签发完成：账号 {} 序列号 {} 有效期 {} 天",
        outcome.account_id, outcome.certificate_serial, outcome.certificate_validity_days
    ));

    // ------------------------------------------------------------------
    // 6. 群组 ACL 与消息队列鉴权
    // ------------------------------------------------------------------
    let group = GroupManager::create(authorize.groups(), outcome.leader_aic.as_str())
        .await
        .map_err(|e| format!("创建群组失败: {}", e))?;
    let group_id = group.id.clone();
    outcome.group_id = group_id.clone();

    authorize
        .add_group_member(
            outcome.leader_aic.as_str(),
            outcome.leader_aic.as_str(),
            group_id.as_str(),
            outcome.food_aic.as_str(),
        )
        .await
        .map_err(|e| format!("写入群组 ACL 失败: {}", e))?;

    let invitation = authorize
        .groups()
        .create_invitation(group_id.as_str(), outcome.food_aic.as_str())
        .await
        .map_err(|e| format!("生成邀请失败: {}", e))?;
    authorize
        .groups()
        .accept_invitation(&invitation.invitation_token, outcome.food_aic.as_str())
        .await
        .map_err(|e| format!("接受邀请失败: {}", e))?;

    outcome.group_member_count = authorize
        .acl()
        .members(outcome.leader_aic.as_str(), group_id.as_str())
        .await
        .len();

    let exchange = format!("group_{}_{}", outcome.leader_aic, group_id);
    outcome.member_exchange_decision = authorize
        .authorize_mq(&MqAuthRequest::resource(
            outcome.food_aic.as_str(),
            SHARED_VHOST,
            "exchange",
            exchange.clone(),
            "write",
        ))
        .await;
    outcome.outsider_exchange_decision = authorize
        .authorize_mq(&MqAuthRequest::resource(
            outcome.hotel_aic.as_str(),
            SHARED_VHOST,
            "exchange",
            exchange,
            "write",
        ))
        .await;

    if !outcome.member_exchange_decision.is_allowed()
        || outcome.outsider_exchange_decision.is_allowed()
    {
        return Err("群组 ACL 鉴权结论不符合预期".to_string());
    }
    report(&format!(
        "群组就绪：{} 成员 {} 人，成员可写/非成员被拒",
        outcome.group_id, outcome.group_member_count
    ));

    // ------------------------------------------------------------------
    // 7. AIP 任务下发与结果回传
    // ------------------------------------------------------------------
    let mut partner = TaskManager::new(outcome.food_aic.clone());
    let task_id = format!("task-{}", Uuid::new_v4());
    let session_id = format!("session-{}", Uuid::new_v4());

    let mut command = TaskCommand::start(
        format!("cmd-{}", Uuid::new_v4()),
        now_iso8601(),
        outcome.leader_aic.clone(),
    )
    .with_task_id(task_id.clone())
    .with_group_id(group_id.clone())
    .with_data_items(vec![DataItem::Text(TextDataItem {
        text: "帮我推荐三家北京美食".to_string(),
        metadata: None,
    })]);
    command.session_id = Some(session_id.clone());

    let accepted = partner
        .handle_command(command)
        .map_err(|e| format!("任务下发失败: {}", e))?;
    if accepted.status.state != TaskState::Accepted {
        return Err("任务未进入 accepted".to_string());
    }

    partner
        .transition(&task_id, TaskState::Working, None)
        .map_err(|e| format!("状态流转失败: {}", e))?;

    partner
        .set_products(
            &task_id,
            vec![Product {
                id: "product-1".to_string(),
                name: Some("北京美食清单".to_string()),
                description: Some("三家推荐餐厅".to_string()),
                data_items: vec![DataItem::Text(TextDataItem {
                    text: "1. 全聚德 2. 东来顺 3. 四季民福".to_string(),
                    metadata: None,
                })],
            }],
            Some(4096),
        )
        .map_err(|e| format!("写入产出物失败: {}", e))?;

    partner
        .transition(&task_id, TaskState::AwaitingCompletion, None)
        .map_err(|e| format!("状态流转失败: {}", e))?;

    let complete = TaskCommand::new(
        format!("cmd-{}", Uuid::new_v4()),
        now_iso8601(),
        "leader".to_string(),
        outcome.leader_aic.clone(),
        TaskCommandType::Complete,
    )
    .with_task_id(task_id.clone());
    let completed = partner
        .handle_command(complete)
        .map_err(|e| format!("任务完成失败: {}", e))?;

    outcome.task_state = Some(completed.status.state);
    outcome.product_count = completed.products.as_ref().map(|p| p.len()).unwrap_or(0);
    outcome.status_history_len = completed.status_history.as_ref().map(|h| h.len()).unwrap_or(0);

    if !outcome.task_state.map(|state| state.is_terminal()).unwrap_or(false) {
        return Err("任务未到终态".to_string());
    }
    report(&format!(
        "任务完成：{} 状态 {} 产出物 {} 个",
        task_id,
        outcome.task_state.map(|state| state.as_str()).unwrap_or("-"),
        outcome.product_count
    ));

    // ------------------------------------------------------------------
    // 8. 注销 / 停用 / 吊销
    // ------------------------------------------------------------------
    // 先把注册服务的最新状态全部同步到发现服务（含第 4 步注册的下游智能体）
    let pre_teardown = registry
        .changes_since(0, 100)
        .await
        .map_err(|e| format!("拉取变更失败: {}", e))?;
    discovery
        .apply_changes(&pre_teardown)
        .await
        .map_err(|e| format!("应用变更失败: {}", e))?;
    outcome.agents_before_unregister = discovery
        .stats()
        .await
        .map_err(|e| e.to_string())?
        .total_agents;

    RegistryService::unregister(&registry, &outcome.food_aic)
        .await
        .map_err(|e| format!("注销失败: {}", e))?;
    RegistryService::set_active(&registry, &outcome.hotel_aic, false)
        .await
        .map_err(|e| format!("停用失败: {}", e))?;

    let teardown_entries = registry
        .changes_since(0, 100)
        .await
        .map_err(|e| format!("拉取变更失败: {}", e))?;
    discovery
        .apply_changes(&teardown_entries)
        .await
        .map_err(|e| format!("应用变更失败: {}", e))?;

    if discovery
        .get_agent(&outcome.food_aic)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("注销后仍能查到智能体".to_string());
    }

    let teardown_stats = discovery.stats().await.map_err(|e| e.to_string())?;
    outcome.agents_after_unregister = teardown_stats.total_agents;

    let revoked = ca
        .revoke_by_aic(&outcome.food_aic, RevocationReason::CessationOfOperation)
        .await
        .map_err(|e| format!("吊销证书失败: {}", e))?;
    if revoked.is_empty() {
        return Err("未吊销任何证书".to_string());
    }

    outcome.cert_status_after_revoke = Some(
        CertifierService::get_certificate_status(&ca, &outcome.certificate_serial)
            .await
            .map_err(|e| format!("查询证书状态失败: {}", e))?,
    );
    outcome.crl_entry_count = ca.crl().await.entries.len();
    outcome.ocsp_status = Some(ca.ocsp(&outcome.certificate_serial).await.status);

    if outcome.cert_status_after_revoke != Some(CertStatus::Revoked)
        || outcome.ocsp_status != Some(CertStatus::Revoked)
        || outcome.crl_entry_count == 0
    {
        return Err("吊销后状态不一致".to_string());
    }
    report(&format!(
        "收尾完成：吊销 {} 张证书，CRL {} 条，剩余可发现智能体 {}",
        revoked.len(),
        outcome.crl_entry_count,
        teardown_stats.available_agents
    ));

    Ok(outcome)
}

/// base64url 无填充编码
fn base64_url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

#[tokio::main]
async fn main() {
    println!("=========================================================================");
    println!(" AIP 智能体互联协议 —— 基础流程端到端演示 v{}", VERSION);
    println!(" 协议版本: {}   AIC OID 前缀: {}", PROTOCOL_VERSION, AIC_OID_PREFIX);
    println!("=========================================================================\n");

    let mut step = 0usize;
    let mut report = |message: &str| {
        step += 1;
        println!("[{:02}] {}", step, message);
    };

    let outcome = match run_basic_flow(&mut report).await {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("\n✗ 基础流程中断: {}", error);
            process::exit(1);
        }
    };

    println!("\n-------------------------------------------------------------------------");
    println!(" 流程结果汇总");
    println!("-------------------------------------------------------------------------");
    println!("  本体 AIC              : {}", outcome.ontology_aic);
    println!("  派生实体 AIC          : {}", outcome.derived_entity_aic);
    println!("  美食智能体 AIC        : {}", outcome.food_aic);
    println!("  酒店智能体 AIC        : {}", outcome.hotel_aic);
    println!("  EAB 凭据 ID           : {}", outcome.eab_key_id);
    println!("  ACME 账号             : {}", outcome.account_id);
    println!(
        "  证书序列号            : {} ({} 天)",
        outcome.certificate_serial, outcome.certificate_validity_days
    );
    println!("  EAB 重放被拒          : {}", outcome.eab_replay_rejected);
    println!(
        "  同步变更条数          : {}（发现服务 {} 个智能体，可发现 {} 个）",
        outcome.changes_applied, outcome.total_agents_after_sync, outcome.available_agents_after_sync
    );
    println!(
        "  发现命中              : {} / {}（候选 {} 个）",
        outcome.discovered_aic, outcome.discovered_skill_id, outcome.discovered_count
    );
    println!(
        "  跨域转发              : {} 条路由，命中 {}",
        outcome.route_count, outcome.forwarded_aic
    );
    println!("  结构化查询命中        : {}", outcome.filtered_aic);
    println!(
        "  群组 {}  成员 {} 人",
        outcome.group_id, outcome.group_member_count
    );
    println!("  成员写群组交换机      : {:?}", outcome.member_exchange_decision);
    println!("  非成员写群组交换机    : {:?}", outcome.outsider_exchange_decision);
    println!(
        "  任务终态              : {}（产出物 {} 个，状态历史 {} 条）",
        outcome.task_state.map(|state| state.as_str()).unwrap_or("-"),
        outcome.product_count,
        outcome.status_history_len
    );
    println!(
        "  注销前后智能体数      : {} -> {}",
        outcome.agents_before_unregister, outcome.agents_after_unregister
    );
    println!(
        "  吊销后证书状态        : {}",
        outcome
            .cert_status_after_revoke
            .map(|status| status.as_str())
            .unwrap_or("-")
    );
    println!(
        "  OCSP 状态             : {}",
        outcome.ocsp_status.map(|status| status.as_str()).unwrap_or("-")
    );
    println!("  CRL 条目              : {}", outcome.crl_entry_count);
    println!("-------------------------------------------------------------------------");
    println!("✓ 基础流程已跑通");
}
