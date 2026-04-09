use crate::runtime::{FactoryProductionStatus, IndustryStage, WorldState};
use crate::simulator::persist::{
    PlayerAgentClaimSnapshot, PlayerGameplayAction, PlayerGameplayGoalKind,
    PlayerGameplayRecentFeedback, PlayerGameplaySnapshot, PlayerGameplayStageId,
    PlayerGameplayStageStatus,
};
use crate::viewer::{ControlCompletionAck, ControlCompletionStatus, ViewerControl};

use super::player_gameplay::extend_available_actions;

fn blocked_control_hint(error_code: Option<&str>) -> String {
    match error_code {
        Some("llm_mode_required" | "llm_init_failed") => {
            "enable --llm and configure a reachable LLM provider before retrying gameplay controls"
                .to_string()
        }
        _ => {
            "inspect the runtime failure, repair the broken world/module state, then retry the control"
                .to_string()
        }
    }
}

pub(super) fn player_gameplay_feedback_from_control_ack(
    mode: &ViewerControl,
    ack: &ControlCompletionAck,
) -> PlayerGameplayRecentFeedback {
    let action = match mode {
        ViewerControl::Pause => "pause",
        ViewerControl::Play => "play",
        ViewerControl::Step { .. } => "step",
        ViewerControl::Seek { .. } => "seek",
    }
    .to_string();
    let (stage, reason, hint) = match ack.status {
        ControlCompletionStatus::Advanced => ("completed_advanced".to_string(), None, None),
        ControlCompletionStatus::TimeoutNoProgress => (
            "completed_no_progress".to_string(),
            Some("latest live control did not create forward progress".to_string()),
            Some(
                "inspect blockers or restore energy/material flow before stepping again"
                    .to_string(),
            ),
        ),
        ControlCompletionStatus::Blocked => (
            "blocked".to_string(),
            Some(ack.error_message.clone().unwrap_or_else(|| {
                "latest live control was blocked before runtime advance".to_string()
            })),
            Some(blocked_control_hint(ack.error_code.as_deref())),
        ),
    };
    let effect = match ack.status {
        ControlCompletionStatus::Advanced => format!(
            "world advanced: logicalTime +{}, eventSeq +{}",
            ack.delta_logical_time, ack.delta_event_seq
        ),
        ControlCompletionStatus::TimeoutNoProgress => format!(
            "no visible world delta: logicalTime +{}, eventSeq +{}",
            ack.delta_logical_time, ack.delta_event_seq
        ),
        ControlCompletionStatus::Blocked => format!(
            "gameplay blocked before requested advance completed: logicalTime +{}, eventSeq +{}",
            ack.delta_logical_time, ack.delta_event_seq
        ),
    };
    PlayerGameplayRecentFeedback {
        action,
        stage,
        effect,
        reason,
        hint,
        delta_logical_time: ack.delta_logical_time,
        delta_event_seq: ack.delta_event_seq,
    }
}

pub(super) fn build_player_gameplay_snapshot(
    state: &WorldState,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
    gameplay_enabled: bool,
    gameplay_disabled_reason: Option<&str>,
    supports_agent_chat: bool,
    agent_claim: Option<PlayerAgentClaimSnapshot>,
) -> PlayerGameplaySnapshot {
    let first_agent_id = state.agents.keys().next().cloned();
    let mut available_actions = base_available_actions(
        first_agent_id.as_deref(),
        gameplay_enabled,
        gameplay_disabled_reason,
        supports_agent_chat,
    );
    if gameplay_enabled {
        extend_available_actions(state, first_agent_id.as_deref(), &mut available_actions);
    }
    if !gameplay_enabled {
        let disabled_reason = gameplay_disabled_reason
            .unwrap_or("gameplay requires runtime live server running with --llm");
        return PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::FirstSessionLoop,
            stage_status: PlayerGameplayStageStatus::Blocked,
            goal_id: "first_session_loop.configure_llm_access".to_string(),
            goal_kind: PlayerGameplayGoalKind::CreateFirstWorldFeedback,
            goal_title: "Configure LLM access before entering the world".to_string(),
            objective:
                "This world requires an active LLM provider before gameplay controls are allowed."
                    .to_string(),
            progress_detail:
                "Gameplay is blocked until runtime live is running with an initialized LLM provider."
                    .to_string(),
            progress_percent: 0,
            blocker_kind: Some("llm_required".to_string()),
            blocker_detail: Some(disabled_reason.to_string()),
            next_step_hint:
                "Enable --llm and configure a reachable provider before retrying play, step, or gameplay actions."
                    .to_string(),
            branch_hint: None,
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
        };
    }
    let latest_blocker = state.factories.iter().find_map(|(factory_id, factory)| {
        let kind = factory.production.current_blocker_kind.as_ref()?;
        let detail = factory
            .production
            .current_blocker_detail
            .clone()
            .unwrap_or_else(|| format!("factory={factory_id}"));
        Some((kind.clone(), detail))
    });
    let blocked_feedback = recent_feedback.and_then(|feedback| {
        matches!(feedback.stage.as_str(), "blocked" | "completed_no_progress").then(|| {
            (
                "no_progress".to_string(),
                feedback.reason.clone().unwrap_or_else(|| {
                    "latest command did not create forward progress".to_string()
                }),
            )
        })
    });

    let has_first_session_feedback = recent_feedback
        .is_some_and(|feedback| feedback.delta_logical_time > 0 || feedback.delta_event_seq > 0);
    let has_material_flow = state.industry_progress.completed_material_transits > 0;
    let has_factory_ready = !state.factories.is_empty();
    let has_recipe_running = state
        .factories
        .values()
        .any(|factory| factory.production.status == FactoryProductionStatus::Running);
    let has_first_output = state.industry_progress.completed_recipe_jobs > 0;
    let has_blocked_history = state
        .factories
        .values()
        .any(|factory| factory.production.last_blocked_at.is_some());
    let has_recovery_history = state
        .factories
        .values()
        .any(|factory| factory.production.last_resumed_at.is_some());
    let industry_stage = state.industry_progress.stage;

    if !has_first_session_feedback
        && !has_material_flow
        && !has_factory_ready
        && !has_recipe_running
        && !has_first_output
        && latest_blocker.is_none()
    {
        available_actions[0].label =
            "Advance 1 step to create the first world feedback".to_string();
        return PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::FirstSessionLoop,
            stage_status: PlayerGameplayStageStatus::Active,
            goal_id: "first_session_loop.create_first_world_feedback".to_string(),
            goal_kind: PlayerGameplayGoalKind::CreateFirstWorldFeedback,
            goal_title: "Create the first visible world feedback".to_string(),
            objective: "Advance the world once and confirm that your action produces a visible state or event delta.".to_string(),
            progress_detail: "You are still in the initial action loop; the first feedback has not been confirmed yet.".to_string(),
            progress_percent: 0,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Request a snapshot, advance 1 step, then inspect the new delta and events.".to_string(),
            branch_hint: None,
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
        };
    }

    if let Some((blocker_kind, blocker_detail)) = latest_blocker.or(blocked_feedback) {
        let (progress_detail, progress_percent) = if has_first_output {
            (
                "Stage progress: the first line already produced output, but the current stoppage still blocks resilient production."
                    .to_string(),
                84,
            )
        } else {
            (
                "Stage progress: you are in the management phase, but the primary line is blocked."
                    .to_string(),
                68,
            )
        };
        return PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Blocked,
            goal_id: "post_onboarding.recover_capability".to_string(),
            goal_kind: PlayerGameplayGoalKind::RecoverCapability,
            goal_title: "Recover sustainable capability".to_string(),
            objective:
                "Recover the blocked line or capability chain instead of repeating one-off actions."
                    .to_string(),
            progress_detail,
            progress_percent,
            blocker_kind: Some(blocker_kind.clone()),
            blocker_detail: Some(blocker_detail.clone()),
            next_step_hint: blocker_next_step(blocker_kind.as_str(), blocker_detail.as_str()),
            branch_hint: None,
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
        };
    }

    if has_first_output {
        match industry_stage {
            IndustryStage::Bootstrap => {
                let (progress_detail, next_step_hint, progress_percent) = if has_recovery_history {
                    (
                        "Stage progress: the first line already recovered once; keep it producing until the first expansion tradeoff is justified."
                            .to_string(),
                        "Advance again and decide whether the next gain should come from more throughput, stronger inputs, or wider logistics reach."
                            .to_string(),
                        88,
                    )
                } else if has_blocked_history {
                    (
                        "Stage progress: the first line produced output, but it still needs one clean recovery beat before expansion becomes the right call."
                            .to_string(),
                        "Keep advancing until the line recovers from the next stoppage and proves it can resume without manual babysitting."
                            .to_string(),
                        82,
                    )
                } else {
                    (
                        "Stage progress: the first output exists; now harden the line until it survives its first real stoppage or exposes a repeatable recovery loop."
                            .to_string(),
                        "Advance 1-2 more times and watch whether the line stays stable, stalls, or recovers into repeatable output."
                            .to_string(),
                        80,
                    )
                };
                return PlayerGameplaySnapshot {
                    stage_id: PlayerGameplayStageId::PostOnboarding,
                    stage_status: PlayerGameplayStageStatus::Active,
                    goal_id: "post_onboarding.stabilize_first_line_after_output".to_string(),
                    goal_kind: PlayerGameplayGoalKind::StabilizeFirstLine,
                    goal_title: "Harden your first output into resilient production".to_string(),
                    objective: "One visible output is not enough. Keep the first line alive until it survives interruption and resumes as a repeatable capability.".to_string(),
                    progress_detail,
                    progress_percent,
                    blocker_kind: None,
                    blocker_detail: None,
                    next_step_hint,
                    branch_hint: None,
                    available_actions,
                    recent_feedback: recent_feedback.cloned(),
                    agent_claim,
                };
            }
            IndustryStage::ScaleOut => {
                return PlayerGameplaySnapshot {
                    stage_id: PlayerGameplayStageId::PostOnboarding,
                    stage_status: PlayerGameplayStageStatus::BranchReady,
                    goal_id: "post_onboarding.choose_first_expansion_tradeoff".to_string(),
                    goal_kind: PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff,
                    goal_title: "Choose the first expansion tradeoff".to_string(),
                    objective: "The first line is stable enough to grow. Pick whether the next investment should buy more throughput, stronger resilience, or wider logistics reach.".to_string(),
                    progress_detail: "Stage progress: bootstrap is complete and the first expansion tradeoff is now unlocked.".to_string(),
                    progress_percent: 92,
                    blocker_kind: None,
                    blocker_detail: None,
                    next_step_hint: "Advance again and commit to one tradeoff: add capacity, protect upstream inputs, or widen distribution coverage.".to_string(),
                    branch_hint: Some(
                        "Tradeoffs unlocked: throughput expansion / input resilience / logistics reach"
                            .to_string(),
                    ),
                    available_actions,
                    recent_feedback: recent_feedback.cloned(),
                    agent_claim,
                };
            }
            IndustryStage::Governance => {
                return PlayerGameplaySnapshot {
                    stage_id: PlayerGameplayStageId::PostOnboarding,
                    stage_status: PlayerGameplayStageStatus::BranchReady,
                    goal_id: "post_onboarding.choose_midloop_path".to_string(),
                    goal_kind: PlayerGameplayGoalKind::ChooseMidLoopPath,
                    goal_title: "Choose your mid-loop path".to_string(),
                    objective: "Your first sustainable industrial capability is online. Expand it into stable organizational momentum.".to_string(),
                    progress_detail: "Stage progress: the first expansion tradeoff is behind you and wider mid-loop branches are now meaningful.".to_string(),
                    progress_percent: 100,
                    blocker_kind: None,
                    blocker_detail: None,
                    next_step_hint: "Keep advancing and either expand production, push governance, or secure a critical node.".to_string(),
                    branch_hint: Some(
                        "Branches unlocked: production expansion / governance influence / conflict security"
                            .to_string(),
                    ),
                    available_actions,
                    recent_feedback: recent_feedback.cloned(),
                    agent_claim,
                };
            }
        }
    }

    if has_recipe_running {
        return PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Active,
            goal_id: "post_onboarding.stabilize_first_line".to_string(),
            goal_kind: PlayerGameplayGoalKind::StabilizeFirstLine,
            goal_title: "Stabilize your first line".to_string(),
            objective: "Keep the first production line moving until it yields stable output or exposes a clear blocker.".to_string(),
            progress_detail: "Stage progress: the first line is running; now watch for output and stoppage reasons.".to_string(),
            progress_percent: 72,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Advance 1-2 more times and watch for output, recovery, or blocker feedback.".to_string(),
            branch_hint: None,
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
        };
    }

    if has_factory_ready {
        return PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Active,
            goal_id: "post_onboarding.start_factory_run".to_string(),
            goal_kind: PlayerGameplayGoalKind::StartFactoryRun,
            goal_title: "Start your first factory run".to_string(),
            objective: "Turn the factory you built into a running, repeatable capability.".to_string(),
            progress_detail: "Stage progress: the factory is ready; one visible production push remains.".to_string(),
            progress_percent: 54,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Keep advancing until the factory starts a recipe, yields output, or returns a blocker.".to_string(),
            branch_hint: None,
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
        };
    }

    if has_material_flow {
        return PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Active,
            goal_id: "post_onboarding.turn_material_flow_into_output".to_string(),
            goal_kind: PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput,
            goal_title: "Turn material flow into output".to_string(),
            objective: "Do not stop at one-off harvesting; push the resource flow into visible output.".to_string(),
            progress_detail: "Stage progress: base resources are moving; now convert them into the first sustainable capability.".to_string(),
            progress_percent: 38,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Keep harvesting, refining, building, or starting the first recipe until stable output appears.".to_string(),
            branch_hint: None,
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
        };
    }

    PlayerGameplaySnapshot {
        stage_id: PlayerGameplayStageId::PostOnboarding,
        stage_status: PlayerGameplayStageStatus::Active,
        goal_id: "post_onboarding.establish_first_capability".to_string(),
        goal_kind: PlayerGameplayGoalKind::EstablishFirstCapability,
        goal_title: "Establish your first sustainable capability".to_string(),
        objective: "The first-session action loop is complete. Create your first sustainable industrial result instead of repeating the tutorial.".to_string(),
        progress_detail: "Stage progress: you have moved from 'can operate' into the start of 'can manage'.".to_string(),
        progress_percent: 20,
        blocker_kind: None,
        blocker_detail: None,
        next_step_hint: "Advance 2-3 more times and prioritize the first output, the first stable line, or one clear recovery signal.".to_string(),
        branch_hint: None,
        available_actions,
        recent_feedback: recent_feedback.cloned(),
        agent_claim,
    }
}

fn base_available_actions(
    first_agent_id: Option<&str>,
    gameplay_enabled: bool,
    gameplay_disabled_reason: Option<&str>,
    supports_agent_chat: bool,
) -> Vec<PlayerGameplayAction> {
    let disabled_reason = (!gameplay_enabled).then(|| {
        gameplay_disabled_reason
            .unwrap_or("gameplay requires runtime live server running with --llm")
            .to_string()
    });
    let mut actions = vec![
        PlayerGameplayAction {
            action_id: "request_snapshot".to_string(),
            label: "Refresh gameplay snapshot".to_string(),
            protocol_action: "request_snapshot".to_string(),
            target_agent_id: None,
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: "advance_step".to_string(),
            label: "Advance 1 step".to_string(),
            protocol_action: "live_control.step".to_string(),
            target_agent_id: None,
            disabled_reason: disabled_reason.clone(),
        },
        PlayerGameplayAction {
            action_id: "resume_play".to_string(),
            label: "Resume live play".to_string(),
            protocol_action: "live_control.play".to_string(),
            target_agent_id: None,
            disabled_reason,
        },
    ];
    if supports_agent_chat {
        if let Some(agent_id) = first_agent_id {
            actions.push(PlayerGameplayAction {
                action_id: "chat_first_agent".to_string(),
                label: "Send one chat/command to the first available agent".to_string(),
                protocol_action: "agent_chat".to_string(),
                target_agent_id: Some(agent_id.to_string()),
                disabled_reason: None,
            });
        }
    }
    actions
}

fn blocker_next_step(kind: &str, detail: &str) -> String {
    let haystack = format!("{kind} {detail}");
    if haystack.contains("power") || haystack.contains("energy") {
        return "Restore energy first, then advance again to verify recovery.".to_string();
    }
    if haystack.contains("material") || haystack.contains("iron") || haystack.contains("input") {
        return "Replenish upstream materials, then advance again to confirm the line resumes."
            .to_string();
    }
    if haystack.contains("logistics") || haystack.contains("transit") {
        return "Repair the transport path or re-route the ledger flow before stepping again."
            .to_string();
    }
    "Inspect the blocker details, recover the line, then advance again to confirm progress."
        .to_string()
}
