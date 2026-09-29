//! 基础流程端到端集成测试
//!
//! 复用 `examples/e2eflow.rs` 中的场景，断言链路每一步的结果。

use aipub::certifier::CertStatus;
use aipub::protocols::aic;
use aipub::protocols::aip::TaskState;

#[path = "../examples/e2eflow.rs"]
mod support;

use support::run_basic_flow;

#[tokio::test]
async fn test_basic_flow_end_to_end() {
    let mut steps: Vec<String> = Vec::new();
    let outcome = {
        let mut report = |message: &str| steps.push(message.to_string());
        run_basic_flow(&mut report)
            .await
            .expect("基础流程应当完整跑通")
    };

    assert!(!steps.is_empty(), "应当记录流程步骤");

    // === 1. AIC 分配与本体/实体关系 ===
    assert!(!outcome.ontology_aic.is_empty());
    assert!(!outcome.derived_entity_aic.is_empty());
    assert_ne!(outcome.ontology_aic, outcome.derived_entity_aic);
    assert!(aic::is_ontology_aic(&outcome.ontology_aic));
    assert!(aic::is_entity_aic(&outcome.derived_entity_aic));
    assert!(aic::is_entity_aic(&outcome.hotel_aic));
    // 派生实体继承本体的 1-8 级前缀
    let prefix = aic::get_ontology_prefix_from_aic(&outcome.ontology_aic).unwrap();
    assert!(outcome
        .derived_entity_aic
        .starts_with(&format!("{}.", prefix)));

    // === 2. 变更流同步 ===
    assert!(outcome.changes_applied >= 4, "应同步全部注册变更");
    assert_eq!(
        outcome.total_agents_after_sync, outcome.changes_applied,
        "每条变更对应一个智能体记录"
    );
    assert!(
        outcome.available_agents_after_sync < outcome.total_agents_after_sync,
        "本体不应计入可发现集合"
    );
    assert!(!outcome.ontology_discoverable, "本体 AIC 不应可被发现");

    // === 3. 发现查询 ===
    assert_eq!(
        outcome.discovered_aic, outcome.food_aic,
        "自然语言查询应命中最相关的智能体"
    );
    assert_eq!(outcome.discovered_skill_id, "beijing-tour.food");
    assert!(outcome.discovered_count >= 1);
    assert_eq!(
        outcome.filtered_aic, outcome.hotel_aic,
        "结构化过滤应命中酒店智能体"
    );

    // === 4. 跨域转发 ===
    assert!(
        outcome.route_count >= 2,
        "应同时包含本地路由与下游路由，实际 {}",
        outcome.route_count
    );
    assert!(!outcome.forwarded_aic.is_empty());

    // === 5. EAB / ACME / 证书 ===
    assert!(!outcome.eab_key_id.is_empty());
    assert!(!outcome.account_id.is_empty());
    assert_eq!(outcome.certificate_serial.len(), 40);
    assert_eq!(outcome.certificate_validity_days, 30, "应遵循 requestedValidity");
    assert!(outcome.eab_replay_rejected, "EAB 凭据必须一次性使用");

    // === 6. 群组与鉴权 ===
    assert!(outcome.group_id.starts_with("group-"));
    assert_eq!(outcome.group_member_count, 2, "Leader 与成员各一条 ACL");
    assert!(outcome.member_exchange_decision.is_allowed());
    assert!(!outcome.outsider_exchange_decision.is_allowed());

    // === 7. AIP 任务 ===
    assert_eq!(outcome.task_state, Some(TaskState::Completed));
    assert!(outcome.task_state.unwrap().is_terminal());
    assert_eq!(outcome.product_count, 1);
    assert!(outcome.status_history_len >= 4, "accepted/working/awaiting/completed");

    // === 8. 收尾 ===
    assert_eq!(
        outcome.agents_after_unregister + 1,
        outcome.agents_before_unregister,
        "注销后应减少 1 个智能体"
    );
    assert_eq!(outcome.cert_status_after_revoke, Some(CertStatus::Revoked));
    assert_eq!(outcome.ocsp_status, Some(CertStatus::Revoked));
    assert!(outcome.crl_entry_count >= 1);
}
