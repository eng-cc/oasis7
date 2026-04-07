use super::*;

#[test]
fn runtime_openclaw_compat_snapshot_exposes_agent_execution_debug_contexts() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "openclaw_local_http");
    std::env::set_var(VIEWER_OPENCLAW_BASE_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_OPENCLAW_EXECUTION_MODE_ENV, "player_parity");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let snapshot = server.compat_snapshot();
    let context = snapshot
        .model
        .agent_execution_debug_contexts
        .get(agent_id.as_str())
        .expect("debug context in snapshot");
    assert_eq!(
        context.provider_mode.as_deref(),
        Some("openclaw_local_http")
    );
    assert_eq!(context.compatibility_status.as_deref(), Some("ready"));
    assert_eq!(context.execution_mode.as_deref(), Some("player_parity"));
    assert_eq!(
        context.observation_schema_version.as_deref(),
        Some(DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION)
    );
    assert_eq!(
        context.action_schema_version.as_deref(),
        Some(DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION)
    );
    assert_eq!(context.environment_class.as_deref(), Some("runtime_live"));
    assert_eq!(
        context.capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(
        context.supported_action_sets,
        vec![
            "wait".to_string(),
            "wait_ticks".to_string(),
            "move_agent".to_string(),
            "speak_to_nearby".to_string(),
            "inspect_target".to_string(),
            "simple_interact".to_string(),
        ]
    );
    assert_eq!(context.fallback_reason, None);
    assert_eq!(
        context.agent_profile.as_deref(),
        Some("oasis7_p0_low_freq_npc")
    );
    clear_runtime_openclaw_env();
}

#[test]
fn runtime_openclaw_compat_snapshot_tracks_alias_fallback_reason() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "agent_direct_connect");
    std::env::set_var(VIEWER_OPENCLAW_BASE_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let snapshot = server.compat_snapshot();
    let context = snapshot
        .model
        .agent_execution_debug_contexts
        .get(agent_id.as_str())
        .expect("debug context in snapshot");
    assert_eq!(context.compatibility_status.as_deref(), Some("degraded"));
    assert_eq!(
        context.fallback_reason.as_deref(),
        Some("provider_mode_alias:agent_direct_connect")
    );
    clear_runtime_openclaw_env();
}

#[test]
fn compat_snapshot_exposes_player_gameplay_snapshot() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let mut server = server;
    let snapshot = server.compat_snapshot();
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::FirstSessionLoop
    );
    assert_eq!(gameplay.goal_id, "first_session_loop.configure_llm_access");
    assert_eq!(
        gameplay.stage_status,
        crate::simulator::PlayerGameplayStageStatus::Blocked
    );
    assert_eq!(gameplay.blocker_kind.as_deref(), Some("llm_required"));
    assert_eq!(
        gameplay.available_actions[0].protocol_action,
        "request_snapshot"
    );
    assert!(gameplay
        .available_actions
        .iter()
        .any(|action| action.action_id == "advance_step"
            && action.disabled_reason.as_deref().is_some()));
    assert!(!gameplay
        .available_actions
        .iter()
        .any(|action| action.action_id == "build_factory_smelter_mk1"));
    assert!(!gameplay
        .available_actions
        .iter()
        .any(|action| action.action_id == "chat_first_agent"));
    assert!(gameplay.recent_feedback.is_none());
}

#[test]
fn compat_snapshot_exposes_player_agent_claim_overview() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");

    server
        .world
        .set_governance_execution_policy(crate::runtime::GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..crate::runtime::GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    server
        .world
        .set_agent_reputation_score(primary_agent_id.as_str(), 0)
        .expect("set reputation");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 1_000,
            circulating_supply: 1_000,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance(primary_agent_id.as_str(), 1_000, 0)
        .expect("seed main token balance");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-target".to_string(),
            pos: crate::geometry::GeoPos::new(0.0, 0.0, 0.0),
        });
    server.world.step().expect("register claim target");
    server
        .world
        .submit_action(crate::runtime::Action::ClaimAgent {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-target".to_string(),
        });
    server.world.step().expect("claim target");
    server
        .world
        .submit_action(crate::runtime::Action::ReleaseAgentClaim {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-target".to_string(),
        });
    server.world.step().expect("request release");

    let snapshot = server.compat_snapshot();
    let claim = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .expect("player agent claim snapshot");
    assert_eq!(claim.claimer_agent_id, primary_agent_id);
    assert_eq!(claim.claim_cap, 1);
    assert_eq!(claim.owned_claim_count, 1);
    assert_eq!(claim.current_epoch, snapshot.time);
    assert_eq!(claim.restricted_starter_claim_balance, 0);
    assert_eq!(claim.slot_1_eligible_claim_balance, 650);

    let quote = claim.next_claim_quote.as_ref().expect("next claim quote");
    assert_eq!(quote.transferable_liquid_balance, 650);
    assert_eq!(quote.restricted_starter_claim_balance, 0);
    assert_eq!(
        quote.blocked_reason.as_deref(),
        Some("agent claim cap exceeded: owned=1 cap=1")
    );

    let owned = claim.owned_claims.first().expect("owned claim entry");
    assert_eq!(owned.target_agent_id, "agent-claim-target");
    assert_eq!(owned.status, "release_cooldown");
    assert_eq!(owned.claim_bond_locked_restricted_amount, 0);
    assert_eq!(owned.claim_bond_locked_liquid_amount, 200);
    assert!(owned.release_ready_in_epochs.is_some());
    assert!(owned.forced_reclaim_in_epochs.is_some());
}

#[test]
fn compat_snapshot_flags_restricted_balance_as_ineligible_for_slot_2() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");

    server
        .world
        .set_governance_execution_policy(crate::runtime::GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..crate::runtime::GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    server
        .world
        .set_agent_reputation_score(primary_agent_id.as_str(), 10)
        .expect("set reputation");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 650,
            circulating_supply: 650,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance_with_restricted(primary_agent_id.as_str(), 0, 0, 650)
        .expect("seed restricted-only claim balance");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-slot2-target-a".to_string(),
            pos: crate::geometry::GeoPos::new(0.0, 0.0, 0.0),
        });
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-slot2-target-b".to_string(),
            pos: crate::geometry::GeoPos::new(0.0, 0.0, 0.0),
        });
    server.world.step().expect("register claim targets");
    server
        .world
        .submit_action(crate::runtime::Action::ClaimAgent {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-slot2-target-a".to_string(),
        });
    server
        .world
        .step()
        .expect("claim slot 1 using restricted balance");

    let snapshot = server.compat_snapshot();
    let claim = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .expect("player agent claim snapshot");
    let quote = claim.next_claim_quote.as_ref().expect("next claim quote");
    assert_eq!(quote.slot_index, 2);
    assert_eq!(quote.transferable_liquid_balance, 0);
    assert_eq!(quote.restricted_starter_claim_balance, 325);
    assert_eq!(quote.eligible_claim_balance, 0);
    assert_eq!(
        quote.blocked_reason.as_deref(),
        Some(
            "restricted_balance_not_eligible_for_slot slot=2 liquid=0 restricted=325 required=488"
        )
    );
}

#[test]
fn compat_snapshot_promotes_to_post_onboarding_after_control_feedback() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "step".to_string(),
        stage: "completed_advanced".to_string(),
        effect: "world advanced: logicalTime +1, eventSeq +1".to_string(),
        reason: None,
        hint: None,
        delta_logical_time: 1,
        delta_event_seq: 1,
    });
    let snapshot = server.compat_snapshot();
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::PostOnboarding
    );
    assert!(gameplay.goal_id.starts_with("post_onboarding."));
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "completed_advanced"
    );
}
