use super::egui_right_panel_player_experience::PlayerGuideStep;
use super::egui_right_panel_player_guide::{
    build_player_mission_loop_snapshot, build_player_mission_remaining_hint,
    build_player_post_onboarding_snapshot, player_control_result_summary,
    player_control_stage_color, player_control_stage_label,
    player_control_stage_shows_recovery_actions, player_mission_hud_anchor_y,
    player_mission_hud_compact_mode, player_mission_hud_minimap_reserved_bottom,
    player_mission_hud_show_command_action, player_mission_hud_show_minimap,
    player_post_onboarding_status_label, PlayerGuideProgressSnapshot, PlayerPostOnboardingStatus,
};
use super::egui_right_panel_player_micro_loop::{
    build_player_micro_loop_snapshot, format_due_timer_line,
};
use crate::web_test_api::WebTestApiControlFeedbackSnapshot;
use oasis7::simulator::{
    PlayerAgentClaimQuoteSnapshot, PlayerAgentClaimSnapshot, PlayerGameplayGoalKind,
    PlayerGameplaySnapshot, PlayerGameplayStageId, PlayerGameplayStageStatus, WorldEvent,
    WorldEventKind,
};

fn sample_post_onboarding_viewer_state(gameplay: PlayerGameplaySnapshot) -> crate::ViewerState {
    let mut state = super::sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 0,
        config: oasis7::simulator::WorldConfig::default(),
        model: oasis7::simulator::WorldModel::default(),
        runtime_snapshot: None,
        player_gameplay: Some(gameplay),
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
    };
    state.snapshot = Some(snapshot);
    state
}

fn no_selection() -> crate::ViewerSelection {
    crate::ViewerSelection { current: None }
}

fn selected_agent(agent_id: &str) -> crate::ViewerSelection {
    crate::ViewerSelection {
        current: Some(crate::SelectionInfo {
            entity: bevy::prelude::Entity::from_bits(7),
            kind: crate::SelectionKind::Agent,
            id: agent_id.to_string(),
            name: Some(agent_id.to_string()),
        }),
    }
}

#[test]
fn build_player_mission_loop_snapshot_open_panel_requires_open_action() {
    let progress = PlayerGuideProgressSnapshot {
        connect_world_done: true,
        open_panel_done: false,
        select_target_done: false,
        explore_ready: false,
    };
    let snapshot = build_player_mission_loop_snapshot(
        PlayerGuideStep::OpenPanel,
        progress,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.completed_steps, 1);
    assert_eq!(
        snapshot.objective,
        "Open the right control panel to unlock actions"
    );
    assert_eq!(
        snapshot.completion_condition,
        "Completion: right panel is visible"
    );
    assert_eq!(snapshot.eta, "ETA: about 5s");
    assert_eq!(snapshot.short_goals[0].label, "Open control panel");
    assert!(!snapshot.short_goals[0].complete);
    assert_eq!(snapshot.short_goals[1].label, "Lock one target");
    assert!(!snapshot.short_goals[1].complete);
    assert_eq!(snapshot.action_label, "Do next step: Open panel");
    assert!(snapshot.action_opens_panel);
}

#[test]
fn build_player_mission_loop_snapshot_reports_progress_and_objective() {
    let progress = PlayerGuideProgressSnapshot {
        connect_world_done: true,
        open_panel_done: true,
        select_target_done: true,
        explore_ready: true,
    };
    let snapshot = build_player_mission_loop_snapshot(
        PlayerGuideStep::ExploreAction,
        progress,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.completed_steps, 4);
    assert_eq!(
        snapshot.objective,
        "Send one command and confirm new world feedback"
    );
    assert_eq!(
        snapshot.completion_condition,
        "Completion: at least one new world feedback appears"
    );
    assert_eq!(snapshot.eta, "ETA: about 20s");
    assert_eq!(snapshot.short_goals[0].label, "Send first order");
    assert!(snapshot.short_goals[0].complete);
    assert_eq!(snapshot.short_goals[1].label, "Confirm world feedback");
    assert!(snapshot.short_goals[1].complete);
    assert_eq!(snapshot.action_label, "Do next step: Open command and play");
    assert!(!snapshot.action_opens_panel);
}

#[test]
fn build_player_mission_remaining_hint_reports_tick_gap_after_feedback() {
    let progress = PlayerGuideProgressSnapshot {
        connect_world_done: true,
        open_panel_done: true,
        select_target_done: true,
        explore_ready: true,
    };
    let mut state = super::sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    state.metrics = Some(oasis7::simulator::RunnerMetrics {
        total_ticks: 12,
        ..oasis7::simulator::RunnerMetrics::default()
    });
    let hint = build_player_mission_remaining_hint(
        PlayerGuideStep::ExploreAction,
        progress,
        &state,
        crate::i18n::UiLocale::EnUs,
    );
    assert_eq!(hint, "Remaining: advance about 8 more ticks (goal tick=20)");
}

#[test]
fn build_player_mission_remaining_hint_reports_connection_waiting_message() {
    let progress = PlayerGuideProgressSnapshot {
        connect_world_done: false,
        open_panel_done: false,
        select_target_done: false,
        explore_ready: false,
    };
    let state = super::sample_viewer_state(crate::ConnectionStatus::Connecting, Vec::new());
    let hint = build_player_mission_remaining_hint(
        PlayerGuideStep::ConnectWorld,
        progress,
        &state,
        crate::i18n::UiLocale::EnUs,
    );
    assert_eq!(
        hint,
        "Remaining: wait until the status chip shows Connected"
    );
}

#[test]
fn player_mission_hud_compact_mode_tracks_panel_visibility() {
    assert!(player_mission_hud_compact_mode(false));
    assert!(!player_mission_hud_compact_mode(true));
}

#[test]
fn player_mission_hud_anchor_avoids_onboarding_overlap() {
    assert_eq!(player_mission_hud_anchor_y(false, false, false), 96.0);
    assert_eq!(player_mission_hud_anchor_y(true, false, false), 136.0);
    assert_eq!(player_mission_hud_anchor_y(true, true, false), 214.0);
    assert_eq!(player_mission_hud_anchor_y(true, true, true), 298.0);
    assert_eq!(player_mission_hud_anchor_y(true, false, true), 136.0);
    assert_eq!(player_mission_hud_anchor_y(false, true, true), 96.0);
}

#[test]
fn player_mission_hud_command_action_only_visible_when_hidden() {
    assert!(player_mission_hud_show_command_action(true));
    assert!(!player_mission_hud_show_command_action(false));
}

#[test]
fn player_mission_hud_minimap_is_visible_only_in_world_first_mode() {
    assert!(player_mission_hud_show_minimap(true));
    assert!(!player_mission_hud_show_minimap(false));
}

#[test]
fn player_mission_hud_minimap_reserves_chatter_space() {
    assert_eq!(player_mission_hud_minimap_reserved_bottom(true), 188.0);
    assert_eq!(player_mission_hud_minimap_reserved_bottom(false), 0.0);
}

#[test]
fn player_control_stage_label_maps_core_states() {
    assert_eq!(
        player_control_stage_label("received", crate::i18n::UiLocale::EnUs),
        "Received"
    );
    assert_eq!(
        player_control_stage_label("executing", crate::i18n::UiLocale::EnUs),
        "Executing"
    );
    assert_eq!(
        player_control_stage_label("blocked", crate::i18n::UiLocale::EnUs),
        "Blocked"
    );
    assert_eq!(
        player_control_stage_label("completed_advanced", crate::i18n::UiLocale::ZhCn),
        "已完成（有推进）"
    );
    assert_eq!(
        player_control_stage_label("completed_no_progress", crate::i18n::UiLocale::EnUs),
        "Completed (no progress)"
    );
    assert_eq!(
        player_control_stage_label("applied", crate::i18n::UiLocale::EnUs),
        "Completed (advanced)"
    );
}

#[test]
fn player_control_stage_shows_recovery_actions_only_for_no_progress_completion() {
    assert!(player_control_stage_shows_recovery_actions(
        "completed_no_progress"
    ));
    assert!(!player_control_stage_shows_recovery_actions(
        "completed_advanced"
    ));
    assert!(!player_control_stage_shows_recovery_actions("executing"));
    assert!(!player_control_stage_shows_recovery_actions("blocked"));
}

#[test]
fn player_control_stage_color_distinguishes_warning_and_positive_states() {
    let positive = player_control_stage_color("completed_advanced");
    let warning = player_control_stage_color("completed_no_progress");
    let blocked = player_control_stage_color("blocked");
    assert_ne!(positive, warning);
    assert_ne!(warning, blocked);
}

#[test]
fn player_control_result_summary_uses_player_facing_language() {
    let executing = player_control_result_summary(
        &WebTestApiControlFeedbackSnapshot {
            action: "step".to_string(),
            stage: "executing".to_string(),
            reason: None,
            hint: None,
            effect: "running".to_string(),
            delta_logical_time: 0,
            delta_event_seq: 0,
            delta_trace_count: 0,
        },
        crate::i18n::UiLocale::EnUs,
    );
    let blocked = player_control_result_summary(
        &WebTestApiControlFeedbackSnapshot {
            action: "step".to_string(),
            stage: "blocked".to_string(),
            reason: Some("line stalled".to_string()),
            hint: Some("restore energy".to_string()),
            effect: "blocked".to_string(),
            delta_logical_time: 0,
            delta_event_seq: 0,
            delta_trace_count: 0,
        },
        crate::i18n::UiLocale::ZhCn,
    );

    assert!(executing.contains("executing"));
    assert!(blocked.contains("阻塞"));
    assert!(blocked.contains("代价"));
}

#[test]
fn player_micro_loop_snapshot_exposes_due_timer_lines() {
    let mut state = super::sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![
            WorldEvent {
                id: 1,
                time: 8,
                kind: WorldEventKind::RuntimeEvent {
                    kind: "runtime.gameplay.governance_proposal_opened".to_string(),
                    domain_kind: Some(
                        "proposal_key=gov-main title=budget closes_at=20".to_string(),
                    ),
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 2,
                time: 9,
                kind: WorldEventKind::RuntimeEvent {
                    kind: "runtime.action_accepted".to_string(),
                    domain_kind: Some(
                        "action_id=7 action_kind=open_governance actor_id=agent-1 eta_ticks=4"
                            .to_string(),
                    ),
                },
                runtime_event: None,
            },
        ],
    );
    state.metrics = Some(oasis7::simulator::RunnerMetrics {
        total_ticks: 10,
        ..oasis7::simulator::RunnerMetrics::default()
    });
    let snapshot = build_player_micro_loop_snapshot(&state, crate::i18n::UiLocale::EnUs);
    assert_eq!(snapshot.action_status.headline, "Recent Action: Accepted");
    let timer_line = format_due_timer_line(&snapshot.due_timers[0], crate::i18n::UiLocale::EnUs);
    assert!(timer_line.contains("Governance gov-main"));
    assert!(timer_line.contains("T-10"));
}

#[test]
fn build_player_post_onboarding_snapshot_defaults_to_first_capability_goal() {
    let state = super::sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.status, PlayerPostOnboardingStatus::Active);
    assert_eq!(
        snapshot.title,
        "PostOnboarding: Establish Your First Sustainable Capability"
    );
    assert!(snapshot
        .objective
        .contains("first sustainable industrial result"));
    assert_eq!(snapshot.progress_percent, 20);
    assert!(snapshot.blocker_detail.is_none());
}

#[test]
fn build_player_post_onboarding_snapshot_surfaces_factory_blockers() {
    let state = super::sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![super::sample_runtime_event(
            7,
            7,
            "runtime.economy.factory_production_blocked",
            "factory=factory.alpha recipe=recipe.motor requester=agent.alpha reason=material_shortage detail=material_shortage:iron_ingot",
        )],
    );
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.status, PlayerPostOnboardingStatus::Blocked);
    assert!(snapshot
        .blocker_detail
        .expect("blocked detail")
        .contains("missing materials"));
    assert!(snapshot.next_step.contains("replenish upstream materials"));
}

#[test]
fn build_player_post_onboarding_snapshot_uses_feedback_blocker_when_no_runtime_blocker_exists() {
    let state = super::sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let feedback = WebTestApiControlFeedbackSnapshot {
        action: "step".to_string(),
        stage: "completed_no_progress".to_string(),
        reason: Some("line stalled".to_string()),
        hint: Some("retry after restoring energy".to_string()),
        effect: "no visible change".to_string(),
        delta_logical_time: 0,
        delta_event_seq: 0,
        delta_trace_count: 0,
    };
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        Some(&feedback),
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.status, PlayerPostOnboardingStatus::Blocked);
    assert!(snapshot
        .blocker_detail
        .expect("blocked detail")
        .contains("power or energy"));
    assert!(snapshot.next_step.contains("restore energy first"));
}

#[test]
fn build_player_post_onboarding_snapshot_unlocks_branches_after_first_output() {
    let state = super::sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![super::sample_runtime_event(
            12,
            12,
            "runtime.economy.recipe_completed",
            "factory=factory.alpha recipe=recipe.motor requester=agent.alpha batches=1 outputs=motor_mk1x2",
        )],
    );
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.status, PlayerPostOnboardingStatus::BranchReady);
    assert_eq!(
        player_post_onboarding_status_label(snapshot.status, crate::i18n::UiLocale::EnUs),
        "Branch Ready"
    );
    assert!(snapshot
        .branch_hint
        .expect("branch hint")
        .contains("Production Expansion"));
    assert_eq!(snapshot.progress_percent, 100);
}

#[test]
fn build_player_post_onboarding_snapshot_prefers_canonical_player_gameplay_snapshot() {
    let state = sample_post_onboarding_viewer_state(PlayerGameplaySnapshot {
        stage_id: PlayerGameplayStageId::PostOnboarding,
        stage_status: PlayerGameplayStageStatus::BranchReady,
        goal_id: "post_onboarding.choose_first_expansion_tradeoff".to_string(),
        goal_kind: PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff,
        goal_title: "Choose the first expansion tradeoff".to_string(),
        objective: "canonical objective".to_string(),
        progress_detail: "canonical progress".to_string(),
        progress_percent: 92,
        blocker_kind: None,
        blocker_detail: None,
        next_step_hint: "canonical next step".to_string(),
        branch_hint: Some("Tradeoffs unlocked: throughput expansion".to_string()),
        available_actions: Vec::new(),
        recent_feedback: None,
        agent_claim: None,
    });
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.status, PlayerPostOnboardingStatus::BranchReady);
    assert_eq!(
        snapshot.title,
        "Next Stage: Choose the First Expansion Tradeoff"
    );
    assert_eq!(snapshot.objective, "canonical objective");
    assert_eq!(snapshot.progress_detail, "canonical progress");
    assert_eq!(snapshot.next_step, "canonical next step");
    assert_eq!(
        snapshot.branch_hint.as_deref(),
        Some("Tradeoffs unlocked: throughput expansion / input resilience / logistics reach")
    );
}

#[test]
fn build_player_post_onboarding_snapshot_surfaces_first_claim_onboarding() {
    let state = sample_post_onboarding_viewer_state(PlayerGameplaySnapshot {
        stage_id: PlayerGameplayStageId::PostOnboarding,
        stage_status: PlayerGameplayStageStatus::Active,
        goal_id: "post_onboarding.recover_capability".to_string(),
        goal_kind: PlayerGameplayGoalKind::RecoverCapability,
        goal_title: "Recover sustainable capability".to_string(),
        objective: "canonical objective".to_string(),
        progress_detail: "canonical progress".to_string(),
        progress_percent: 32,
        blocker_kind: None,
        blocker_detail: None,
        next_step_hint: "canonical next step".to_string(),
        branch_hint: None,
        available_actions: Vec::new(),
        recent_feedback: None,
        agent_claim: Some(PlayerAgentClaimSnapshot {
            claimer_agent_id: "player-agent".to_string(),
            current_epoch: 7,
            reputation_tier: 0,
            claim_cap: 1,
            owned_claim_count: 0,
            liquid_main_token_balance: 0,
            restricted_starter_claim_balance: 650,
            slot_1_eligible_claim_balance: 650,
            next_claim_quote: Some(PlayerAgentClaimQuoteSnapshot {
                slot_index: 1,
                reputation_tier: 0,
                claim_cap: 1,
                owned_claim_count: 0,
                activation_fee_amount: 100,
                claim_bond_amount: 200,
                upkeep_per_epoch: 25,
                total_upfront_amount: 325,
                transferable_liquid_balance: 0,
                restricted_starter_claim_balance: 650,
                eligible_claim_balance: 650,
                release_cooldown_epochs: 2,
                grace_epochs: 2,
                idle_warning_epochs: 7,
                forced_idle_reclaim_epochs: 10,
                forced_reclaim_penalty_bps: 2000,
                blocked_reason: None,
            }),
            first_agent_claim_approval_request: None,
            owned_claims: Vec::new(),
        }),
    });
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &selected_agent("agent-slot-1"),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    let claim = snapshot.claim_onboarding.expect("claim onboarding");
    assert_eq!(claim.target_agent_id.as_deref(), Some("agent-slot-1"));
    assert!(claim.ready_to_prepare);
    assert!(claim.ready_to_submit);
    assert!(claim.summary.contains("Slot 1 quote"));
    assert!(claim.guidance.contains("agent-slot-1"));
}

#[test]
fn build_player_post_onboarding_snapshot_requests_target_before_first_claim() {
    let state = sample_post_onboarding_viewer_state(PlayerGameplaySnapshot {
        stage_id: PlayerGameplayStageId::PostOnboarding,
        stage_status: PlayerGameplayStageStatus::Active,
        goal_id: "post_onboarding.recover_capability".to_string(),
        goal_kind: PlayerGameplayGoalKind::RecoverCapability,
        goal_title: "Recover sustainable capability".to_string(),
        objective: "canonical objective".to_string(),
        progress_detail: "canonical progress".to_string(),
        progress_percent: 32,
        blocker_kind: None,
        blocker_detail: None,
        next_step_hint: "canonical next step".to_string(),
        branch_hint: None,
        available_actions: Vec::new(),
        recent_feedback: None,
        agent_claim: Some(PlayerAgentClaimSnapshot {
            claimer_agent_id: "player-agent".to_string(),
            current_epoch: 7,
            reputation_tier: 0,
            claim_cap: 1,
            owned_claim_count: 0,
            liquid_main_token_balance: 0,
            restricted_starter_claim_balance: 650,
            slot_1_eligible_claim_balance: 650,
            next_claim_quote: Some(PlayerAgentClaimQuoteSnapshot {
                slot_index: 1,
                reputation_tier: 0,
                claim_cap: 1,
                owned_claim_count: 0,
                activation_fee_amount: 100,
                claim_bond_amount: 200,
                upkeep_per_epoch: 25,
                total_upfront_amount: 325,
                transferable_liquid_balance: 0,
                restricted_starter_claim_balance: 650,
                eligible_claim_balance: 650,
                release_cooldown_epochs: 2,
                grace_epochs: 2,
                idle_warning_epochs: 7,
                forced_idle_reclaim_epochs: 10,
                forced_reclaim_penalty_bps: 2000,
                blocked_reason: None,
            }),
            first_agent_claim_approval_request: None,
            owned_claims: Vec::new(),
        }),
    });
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    let claim = snapshot.claim_onboarding.expect("claim onboarding");
    assert!(!claim.ready_to_prepare);
    assert!(!claim.ready_to_submit);
    assert!(claim.target_agent_id.is_none());
    assert!(claim.guidance.contains("Select an unclaimed agent"));
}

#[test]
fn build_player_post_onboarding_snapshot_uses_canonical_blocker_fields() {
    let state = sample_post_onboarding_viewer_state(PlayerGameplaySnapshot {
        stage_id: PlayerGameplayStageId::PostOnboarding,
        stage_status: PlayerGameplayStageStatus::Blocked,
        goal_id: "post_onboarding.recover_capability".to_string(),
        goal_kind: PlayerGameplayGoalKind::RecoverCapability,
        goal_title: "Recover sustainable capability".to_string(),
        objective: "canonical blocked objective".to_string(),
        progress_detail: "canonical blocked progress".to_string(),
        progress_percent: 68,
        blocker_kind: Some("material_shortage".to_string()),
        blocker_detail: Some("material_shortage:iron_ingot".to_string()),
        next_step_hint: "canonical blocker next step".to_string(),
        branch_hint: None,
        available_actions: Vec::new(),
        recent_feedback: None,
        agent_claim: None,
    });
    let snapshot = build_player_post_onboarding_snapshot(
        &state,
        &no_selection(),
        None,
        crate::i18n::UiLocale::EnUs,
    );

    assert_eq!(snapshot.status, PlayerPostOnboardingStatus::Blocked);
    assert!(snapshot
        .blocker_detail
        .expect("blocked detail")
        .contains("missing materials"));
    assert!(snapshot.next_step.contains("replenish upstream materials"));
}
