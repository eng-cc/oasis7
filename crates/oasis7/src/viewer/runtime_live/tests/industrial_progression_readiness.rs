use super::industrial_progression::{
    build_first_smelter_via_gameplay_action, expect_player_gameplay,
    setup_industrial_gameplay_with_completed_jobs, setup_runtime_industrial_gameplay_session,
    smelter_schedule_action,
};
use super::*;
use crate::simulator::{PlayerGameplayGoalKind, PlayerGameplayStageStatus, ResourceKind};

fn submit_iron_ingot_schedule(
    server: &mut ViewerRuntimeLiveServer,
    agent_id: &str,
    public_key: &str,
    private_key: &str,
    nonce: u64,
) {
    server
        .handle_gameplay_action(signed_gameplay_action_request(
            crate::viewer::GameplayActionRequest {
                action_id: crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT.to_string(),
                target_agent_id: agent_id.to_string(),
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
            nonce,
            public_key,
            private_key,
        ))
        .expect("queue iron ingot schedule");
    server
        .world
        .step()
        .expect("settle rejected iron ingot schedule");
}

#[test]
fn runtime_gameplay_smelter_readiness_matches_submit_material_and_owner_power_checks() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(43);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        43,
    );
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund agent electricity");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, i64::MAX)
        .expect("fund agent data");

    let input_ledger = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory")
        .input_ledger
        .clone();
    server
        .world
        .set_ledger_material_balance(input_ledger, "iron_ore", 0)
        .expect("drain smelter iron ore");
    server
        .world
        .set_material_balance("iron_ore", 0)
        .expect("drain world iron ore");

    let gameplay = expect_player_gameplay(&mut server, "material readiness snapshot");
    let material_reason =
        smelter_schedule_action(&gameplay, crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT)
            .disabled_reason
            .as_deref()
            .expect("missing material should disable iron ingot scheduling");
    assert!(material_reason.contains("insufficient iron_ore"));
    assert!(material_reason.contains("replenish iron_ore"));

    submit_iron_ingot_schedule(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        44,
    );
    assert_eq!(
        server
            .world
            .state()
            .factories
            .get("factory.smelter.mk1")
            .and_then(|factory| factory.production.current_blocker_kind.as_deref()),
        Some("material_shortage"),
        "submit must reject the material shortage surfaced by readiness"
    );

    let input_ledger = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory")
        .input_ledger
        .clone();
    server
        .world
        .set_ledger_material_balance(input_ledger, "iron_ore", 400)
        .expect("restore smelter iron ore");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 0)
        .expect("drain factory-owner electricity");

    let gameplay = expect_player_gameplay(&mut server, "owner power readiness snapshot");
    let power_reason =
        smelter_schedule_action(&gameplay, crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT)
            .disabled_reason
            .as_deref()
            .expect("missing owner power should disable iron ingot scheduling");
    assert!(power_reason.contains("insufficient electricity"));
    assert!(power_reason.contains("replenish electricity"));

    submit_iron_ingot_schedule(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        45,
    );
    assert_eq!(
        server
            .world
            .state()
            .factories
            .get("factory.smelter.mk1")
            .and_then(|factory| factory.production.current_blocker_kind.as_deref()),
        Some("power_shortage"),
        "submit must reject the owner power shortage surfaced by readiness"
    );
}

#[test]
fn runtime_gameplay_action_unlocks_first_expansion_tradeoff_after_scale_out() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(41, 4);
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .expect("scale-out agent")
        .clone();
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund scale-out agent electricity");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, i64::MAX)
        .expect("fund scale-out agent data");
    server
        .world
        .set_material_balance("iron_ingot", 200)
        .expect("seed scale-out alloy iron ingot");
    server
        .world
        .set_material_balance("copper_wire", 200)
        .expect("seed scale-out alloy copper wire");
    server
        .world
        .set_resource_balance(ResourceKind::Electricity, 2_000);
    let gameplay = expect_player_gameplay(&mut server, "player gameplay after scale-out");
    assert_eq!(
        gameplay.goal_id,
        "post_onboarding.choose_first_expansion_tradeoff"
    );
    assert_eq!(
        gameplay.goal_kind,
        PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff
    );
    assert_eq!(
        gameplay.stage_status,
        PlayerGameplayStageStatus::BranchReady
    );
    assert_eq!(gameplay.progress_percent, 92);
    assert!(
        gameplay
            .branch_hint
            .as_deref()
            .is_some_and(|hint| hint.contains("throughput expansion"))
    );
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(
                |action| action.action_id == "schedule_recipe_smelter_alloy_plate"
                    && action.disabled_reason.is_none()
            )
    );
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "build_factory_assembler_mk1")
    );
    assert!((1..=3).contains(&gameplay.branch_recommendations.len()));
    assert_eq!(
        gameplay
            .branch_recommendations
            .iter()
            .map(|recommendation| recommendation.action_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "schedule_recipe_smelter_alloy_plate",
            "build_factory_assembler_mk1",
        ]
    );
    for recommendation in &gameplay.branch_recommendations {
        assert!(!recommendation.route_label.trim().is_empty());
        assert!(!recommendation.immediate_gain.trim().is_empty());
        assert!(!recommendation.future_beat_changed.trim().is_empty());
        assert_eq!(
            recommendation.future_beats.len(),
            2,
            "each actionable branch recommendation must expose exactly two future beats"
        );
        assert!(
            recommendation
                .future_beats
                .iter()
                .all(|beat| !beat.trim().is_empty()),
            "future beats must be player-readable"
        );
        assert_ne!(
            recommendation.future_beats[0].trim(),
            recommendation.future_beats[1].trim(),
            "future beats must describe substantively distinct changes"
        );
        assert!(!recommendation.risk_or_lockin.trim().is_empty());
        assert!(!recommendation.next_session_hook.trim().is_empty());
        assert!(
            gameplay
                .available_actions
                .iter()
                .any(|action| action.action_id == recommendation.action_id
                    && action.disabled_reason.is_none())
        );
    }
    assert_eq!(
        gameplay.small_player_lane_id.as_deref(),
        Some("local_operator")
    );
    assert_eq!(
        gameplay.leverage_class.as_deref(),
        Some("regional_specialization_option")
    );
    assert_eq!(gameplay.same_loop_repeat_count, 4);
    assert!(!gameplay.grind_only_flag);
    assert_eq!(
        gameplay.major_power_dependency_status.as_deref(),
        Some("independent_path_available")
    );
    assert_eq!(
        gameplay.recovery_path_kind.as_deref(),
        Some("repair_rebuild_or_pivot")
    );
    assert!(
        gameplay
            .recovery_path_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("repair"))
    );
}
