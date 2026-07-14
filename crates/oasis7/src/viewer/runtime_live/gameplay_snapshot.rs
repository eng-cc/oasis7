use crate::runtime::{
    FactoryProductionStatus, IndustryStage, WorldEvent as RuntimeWorldEvent,
    WorldEventBody as RuntimeWorldEventBody, WorldState,
};
use crate::simulator::persist::{
    PlayerAgentClaimSnapshot, PlayerGameplayAction, PlayerGameplayBranchCommitment,
    PlayerGameplayCausalityKind, PlayerGameplayExecutionState, PlayerGameplayGoalKind,
    PlayerGameplayRecentFeedback, PlayerGameplaySnapshot, PlayerGameplayStageId,
    PlayerGameplayStageStatus,
};
use crate::viewer::{ControlCompletionAck, ControlCompletionStatus, ViewerControl};

pub(super) use super::gameplay_snapshot_helpers::apply_runtime_snapshot_empty_entities_blocker;
use super::gameplay_snapshot_helpers::{
    base_available_actions, blocker_next_step, primary_factory_for_player_gameplay,
};
use super::gameplay_snapshot_lane::apply_small_player_lane_truth;
use super::player_gameplay::extend_available_actions;
use crate::viewer::ACTION_CLAIM_FIRST_AGENT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlayerGameplayCausalitySignal {
    pub kind: PlayerGameplayCausalityKind,
    pub detail: String,
}

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

fn first_session_runtime_sync_blocker(
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> Option<(String, String, String)> {
    let feedback = recent_feedback?;
    if feedback.action != "chain_sync" {
        return None;
    }
    if !matches!(feedback.stage.as_str(), "blocked" | "completed_no_progress") {
        return None;
    }
    let detail = feedback.reason.clone().unwrap_or_else(|| {
        "committed runtime sync did not expose a usable world snapshot".to_string()
    });
    let kind = if detail.contains("execution world is not ready") {
        "execution_world_not_ready".to_string()
    } else {
        "runtime_sync_unavailable".to_string()
    };
    let hint = feedback.hint.clone().unwrap_or_else(|| {
        "repair the runtime sync path, then refresh gameplay to confirm the committed world is available"
            .to_string()
    });
    Some((kind, detail, hint))
}

pub(super) fn player_gameplay_causality_from_runtime_events(
    new_events: &[RuntimeWorldEvent],
) -> Option<PlayerGameplayCausalitySignal> {
    let mut override_detail = None;
    let mut override_fallback = None;
    for runtime_event in new_events {
        match &runtime_event.body {
            RuntimeWorldEventBody::RuleDecisionRecorded(record)
                if record.override_action.is_some() =>
            {
                let notes = if record.notes.is_empty() {
                    "no rule note supplied".to_string()
                } else {
                    record.notes.join("; ")
                };
                override_detail = Some(format!(
                    "rule module {} redirected the accepted action before execution: {}",
                    record.module_id, notes
                ));
            }
            RuntimeWorldEventBody::ActionOverridden(record) => {
                override_fallback = Some(format!(
                    "the acting agent followed an overridden plan instead of the original action: {:?} -> {:?}",
                    record.original_action, record.override_action
                ));
            }
            _ => {}
        }
    }
    override_fallback.map(|fallback| PlayerGameplayCausalitySignal {
        kind: PlayerGameplayCausalityKind::AgentOverride,
        detail: override_detail.unwrap_or(fallback),
    })
}

pub(super) fn player_gameplay_feedback_from_control_ack(
    mode: &ViewerControl,
    ack: &ControlCompletionAck,
) -> PlayerGameplayRecentFeedback {
    let (action, intent_summary) = match mode {
        ViewerControl::Pause => (
            "pause".to_string(),
            "pause live world advancement".to_string(),
        ),
        ViewerControl::Play => (
            "play".to_string(),
            "continue advancing the live world".to_string(),
        ),
        ViewerControl::Step { count } => (
            "step".to_string(),
            format!("advance the live world by {count} step(s)"),
        ),
        ViewerControl::Seek { tick } => (
            "seek".to_string(),
            format!("seek the live world to tick {tick}"),
        ),
    };
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
        intent_summary: Some(intent_summary),
        target_agent_id: None,
        reason,
        hint,
        delta_logical_time: ack.delta_logical_time,
        delta_event_seq: ack.delta_event_seq,
    }
}

fn player_gameplay_intent_scope(action: &str) -> Option<&'static str> {
    if action.starts_with("gameplay_action:") {
        Some("gameplay_action")
    } else if action.starts_with("prompt_control.") {
        Some("prompt_control")
    } else if action == "agent_chat" {
        Some("agent_chat")
    } else if matches!(action, "play" | "pause" | "step" | "seek") {
        Some("world_control")
    } else if action == "chain_sync" {
        Some("world_sync")
    } else {
        None
    }
}

fn player_gameplay_intent_summary(feedback: &PlayerGameplayRecentFeedback) -> Option<String> {
    feedback.intent_summary.clone().or_else(|| {
        if feedback.action.is_empty() {
            None
        } else {
            Some(feedback.action.replace('_', " "))
        }
    })
}

fn player_gameplay_status_reason(
    gameplay: &PlayerGameplaySnapshot,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> Option<String> {
    gameplay
        .causality_detail
        .clone()
        .or_else(|| gameplay.blocker_detail.clone())
        .or_else(|| recent_feedback.and_then(|feedback| feedback.reason.clone()))
        .or_else(|| gameplay.blocker_kind.clone())
}

fn player_gameplay_last_world_change(
    gameplay: &PlayerGameplaySnapshot,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> Option<String> {
    recent_feedback
        .filter(|feedback| feedback.delta_logical_time > 0 || feedback.delta_event_seq > 0)
        .map(|feedback| feedback.effect.clone())
        .filter(|effect| !effect.trim().is_empty())
        .or_else(|| {
            matches!(
                gameplay.causality_kind,
                Some(PlayerGameplayCausalityKind::GoalProgressed)
            )
            .then(|| gameplay.progress_detail.clone())
        })
}

fn player_gameplay_primary_blocker(
    gameplay: &PlayerGameplaySnapshot,
    status_reason: Option<&String>,
) -> Option<String> {
    if gameplay.execution_state != PlayerGameplayExecutionState::Blocked
        && gameplay.stage_status != PlayerGameplayStageStatus::Blocked
    {
        return None;
    }
    gameplay
        .blocker_detail
        .clone()
        .or_else(|| gameplay.blocker_kind.clone())
        .or_else(|| status_reason.cloned())
}

fn player_gameplay_response_window_class(
    gameplay: &PlayerGameplaySnapshot,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> Option<String> {
    if let Some(feedback) = recent_feedback {
        return Some(match feedback.stage.as_str() {
            "accepted" | "submitted" | "queued" | "ack" => {
                "waiting_for_committed_progress".to_string()
            }
            "completed_no_progress" => "stalled_needs_escalation".to_string(),
            "blocked" => "blocked_needs_repair".to_string(),
            "rejected" => "request_rejected".to_string(),
            "completed_advanced" => "resolved".to_string(),
            _ => match gameplay.execution_state {
                PlayerGameplayExecutionState::Accepted => "waiting_for_committed_progress",
                PlayerGameplayExecutionState::Blocked => "blocked_needs_repair",
                PlayerGameplayExecutionState::Rejected => "request_rejected",
                PlayerGameplayExecutionState::Completed => "resolved",
                PlayerGameplayExecutionState::Executing => "watch_next_tick",
            }
            .to_string(),
        });
    }

    match gameplay.execution_state {
        PlayerGameplayExecutionState::Accepted => {
            Some("waiting_for_committed_progress".to_string())
        }
        PlayerGameplayExecutionState::Blocked => Some("blocked_needs_repair".to_string()),
        PlayerGameplayExecutionState::Rejected => Some("request_rejected".to_string()),
        PlayerGameplayExecutionState::Completed => Some("resolved".to_string()),
        PlayerGameplayExecutionState::Executing => None,
    }
}

fn player_gameplay_stalled_reason(
    gameplay: &PlayerGameplaySnapshot,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> Option<String> {
    recent_feedback
        .filter(|feedback| matches!(feedback.stage.as_str(), "completed_no_progress" | "blocked"))
        .and_then(|feedback| feedback.reason.clone())
        .or_else(|| {
            (gameplay.execution_state == PlayerGameplayExecutionState::Blocked)
                .then(|| {
                    gameplay
                        .blocker_detail
                        .clone()
                        .or_else(|| gameplay.blocker_kind.clone())
                })
                .flatten()
        })
}

fn player_gameplay_escalation_hint(
    gameplay: &PlayerGameplaySnapshot,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> Option<String> {
    recent_feedback
        .filter(|feedback| {
            matches!(
                feedback.stage.as_str(),
                "accepted" | "submitted" | "queued" | "ack" | "completed_no_progress" | "blocked"
            )
        })
        .and_then(|feedback| feedback.hint.clone())
        .or_else(|| {
            (gameplay.execution_state == PlayerGameplayExecutionState::Blocked
                || gameplay.execution_state == PlayerGameplayExecutionState::Accepted)
                .then(|| gameplay.next_step_hint.clone())
        })
}

fn player_gameplay_fallback_action(
    gameplay: &PlayerGameplaySnapshot,
    response_window_class: Option<&str>,
) -> Option<(String, String)> {
    let enabled_actions: Vec<&PlayerGameplayAction> = gameplay
        .available_actions
        .iter()
        .filter(|action| action.disabled_reason.is_none())
        .collect();
    if enabled_actions.is_empty() {
        return None;
    }

    let request_snapshot = enabled_actions
        .iter()
        .find(|action| action.protocol_action == "request_snapshot")
        .copied();
    let advance_step = enabled_actions
        .iter()
        .find(|action| action.action_id == "advance_step")
        .copied();
    let resume_play = enabled_actions
        .iter()
        .find(|action| action.action_id == "resume_play")
        .copied();

    let preferred = match response_window_class {
        Some("waiting_for_committed_progress") => advance_step.or(resume_play).or(request_snapshot),
        Some("stalled_needs_escalation") | Some("blocked_needs_repair") => {
            request_snapshot.or(advance_step).or(resume_play)
        }
        Some("request_rejected") => request_snapshot.or(advance_step).or(resume_play),
        _ => None,
    }
    .or_else(|| enabled_actions.first().copied())?;

    Some((preferred.action_id.clone(), preferred.label.clone()))
}

fn derive_player_gameplay_execution_state(
    stage_status: PlayerGameplayStageStatus,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
) -> PlayerGameplayExecutionState {
    if let Some(feedback) = recent_feedback {
        match feedback.stage.as_str() {
            "accepted" | "submitted" | "queued" | "ack" => {
                return PlayerGameplayExecutionState::Accepted;
            }
            "rejected" => return PlayerGameplayExecutionState::Rejected,
            "blocked" | "completed_no_progress" => return PlayerGameplayExecutionState::Blocked,
            "completed_advanced" => return PlayerGameplayExecutionState::Completed,
            _ => {}
        }
    }

    match stage_status {
        PlayerGameplayStageStatus::Blocked => PlayerGameplayExecutionState::Blocked,
        PlayerGameplayStageStatus::BranchReady => PlayerGameplayExecutionState::Completed,
        PlayerGameplayStageStatus::Active => PlayerGameplayExecutionState::Executing,
    }
}

fn derive_player_gameplay_causality(
    gameplay: &PlayerGameplaySnapshot,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
    causality_signal: Option<&PlayerGameplayCausalitySignal>,
) -> (Option<PlayerGameplayCausalityKind>, Option<String>) {
    if let Some(signal) = causality_signal {
        return (Some(signal.kind), Some(signal.detail.clone()));
    }

    match gameplay.execution_state {
        PlayerGameplayExecutionState::Accepted => (
            Some(PlayerGameplayCausalityKind::QueuedForExecution),
            Some(
                recent_feedback
                    .and_then(|feedback| feedback.hint.clone())
                    .unwrap_or_else(|| {
                        "the latest goal-affecting command is accepted and waiting for committed world progress"
                            .to_string()
                    }),
            ),
        ),
        PlayerGameplayExecutionState::Rejected => (
            Some(PlayerGameplayCausalityKind::RequestRejected),
            Some(
                recent_feedback
                    .and_then(|feedback| feedback.reason.clone())
                    .unwrap_or_else(|| {
                        "the latest goal-affecting request was rejected before execution"
                            .to_string()
                    }),
            ),
        ),
        PlayerGameplayExecutionState::Blocked => (
            Some(PlayerGameplayCausalityKind::WorldConstraint),
            gameplay
                .blocker_detail
                .clone()
                .or_else(|| recent_feedback.and_then(|feedback| feedback.reason.clone()))
                .or_else(|| {
                    gameplay
                        .blocker_kind
                        .as_ref()
                        .map(|kind| format!("current goal is blocked by {kind}"))
                }),
        ),
        PlayerGameplayExecutionState::Completed => (
            Some(PlayerGameplayCausalityKind::GoalProgressed),
            Some(
                recent_feedback
                    .map(|feedback| feedback.effect.clone())
                    .filter(|effect| !effect.trim().is_empty())
                    .unwrap_or_else(|| gameplay.progress_detail.clone()),
            ),
        ),
        PlayerGameplayExecutionState::Executing => (None, None),
    }
}

fn finalize_player_gameplay_snapshot(
    mut gameplay: PlayerGameplaySnapshot,
    industry_stage: IndustryStage,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
    causality_signal: Option<&PlayerGameplayCausalitySignal>,
) -> PlayerGameplaySnapshot {
    gameplay.branch_recommendations = branch_recommendations(
        industry_stage,
        gameplay.stage_status,
        gameplay.available_actions.as_slice(),
    );
    let effective_stage_status = effective_branch_stage_status(
        gameplay.stage_status,
        gameplay.branch_recommendations.as_slice(),
    );
    if effective_stage_status != gameplay.stage_status {
        gameplay.stage_status = effective_stage_status;
        gameplay.blocker_kind = Some("branch_commitment_unavailable".to_string());
        gameplay.blocker_detail = Some(
            "No executable branch commitment is currently available from the published actions."
                .to_string(),
        );
        gameplay.next_step_hint =
            "Restore the inputs or capability required by a published branch action, then inspect the choices again."
                .to_string();
        gameplay.branch_hint = None;
    }
    gameplay.execution_state =
        derive_player_gameplay_execution_state(gameplay.stage_status, recent_feedback);
    let (causality_kind, causality_detail) =
        derive_player_gameplay_causality(&gameplay, recent_feedback, causality_signal);
    gameplay.causality_kind = causality_kind;
    gameplay.causality_detail = causality_detail;
    let status_reason = player_gameplay_status_reason(&gameplay, recent_feedback);
    gameplay.accepted_intent_id = recent_feedback
        .map(|feedback| feedback.action.clone())
        .filter(|value| !value.is_empty());
    gameplay.intent_summary = recent_feedback.and_then(player_gameplay_intent_summary);
    gameplay.intent_scope = recent_feedback.and_then(|feedback| {
        player_gameplay_intent_scope(feedback.action.as_str()).map(str::to_string)
    });
    gameplay.intent_target = recent_feedback.and_then(|feedback| feedback.target_agent_id.clone());
    gameplay.status_reason = status_reason.clone();
    gameplay.last_world_change = player_gameplay_last_world_change(&gameplay, recent_feedback);
    gameplay.resume_anchor = Some(format!("{} ({})", gameplay.goal_title, gameplay.goal_id));
    gameplay.primary_blocker = player_gameplay_primary_blocker(&gameplay, status_reason.as_ref());
    gameplay.response_window_class =
        player_gameplay_response_window_class(&gameplay, recent_feedback);
    gameplay.stalled_reason = player_gameplay_stalled_reason(&gameplay, recent_feedback);
    gameplay.escalation_hint = player_gameplay_escalation_hint(&gameplay, recent_feedback);
    let fallback_action =
        player_gameplay_fallback_action(&gameplay, gameplay.response_window_class.as_deref());
    gameplay.fallback_action_id = fallback_action.as_ref().map(|(id, _)| id.clone());
    gameplay.fallback_action_label = fallback_action.map(|(_, label)| label);
    gameplay.resume_next_step = Some(gameplay.next_step_hint.clone());
    apply_small_player_lane_truth(&mut gameplay);
    gameplay
}

fn branch_recommendations(
    industry_stage: IndustryStage,
    stage_status: PlayerGameplayStageStatus,
    available_actions: &[PlayerGameplayAction],
) -> Vec<PlayerGameplayBranchCommitment> {
    if stage_status != PlayerGameplayStageStatus::BranchReady {
        return Vec::new();
    }

    let enabled = |action_id: &str| {
        available_actions
            .iter()
            .any(|action| action.action_id == action_id && action.disabled_reason.is_none())
    };
    let commitment =
        |action_id: &str,
         route_label: &str,
         immediate_gain: &str,
         future_beat_changed: &str,
         risk_or_lockin: &str,
         next_session_hook: &str| PlayerGameplayBranchCommitment {
            action_id: action_id.to_string(),
            route_label: route_label.to_string(),
            immediate_gain: immediate_gain.to_string(),
            future_beat_changed: future_beat_changed.to_string(),
            risk_or_lockin: risk_or_lockin.to_string(),
            next_session_hook: next_session_hook.to_string(),
        };

    let scale_out_candidates = [
        commitment(
            "schedule_recipe_smelter_alloy_plate",
            "Deepen the smelter line",
            "Convert the stable line into higher-tier alloy output.",
            "The next beat shifts from basic throughput to advanced inputs.",
            "Consumes line time that could have supplied basic materials.",
            "Return to turn the alloy output into the next production upgrade.",
        ),
        commitment(
            "build_factory_assembler_mk1",
            "Open a second production line",
            "Add the assembler capability and widen the real recipe surface.",
            "The next beat becomes coordinating two complementary factories.",
            "Commits construction materials before the new line produces value.",
            "Return to choose and run the assembler's first recipe.",
        ),
    ];
    let governance_candidates = [
        commitment(
            "schedule_recipe_assembler_module_rack",
            "Produce a module rack",
            "Create a governance-stage finished component.",
            "The next beat shifts toward assembling durable industrial modules.",
            "Uses advanced assembler inputs and occupies the current recipe slot.",
            "Return to connect the module rack to a factory-core production run.",
        ),
        commitment(
            "schedule_recipe_assembler_factory_core",
            "Produce a factory core",
            "Create infrastructure output from the mature assembler line.",
            "The next beat shifts toward deploying another industrial capability.",
            "Consumes a module rack and alloy plate before yielding the core.",
            "Return to decide where the new factory core creates the most leverage.",
        ),
        commitment(
            "schedule_recipe_smelter_alloy_plate",
            "Sustain alloy throughput",
            "Produce another alloy plate through the mature smelter line.",
            "The next beat keeps advanced assembler inputs supplied while other routes unlock.",
            "Uses the smelter queue and iron inputs without automatically advancing governance.",
            "Return to inspect the alloy output and choose the next advanced recipe.",
        ),
    ];

    let candidates = match industry_stage {
        IndustryStage::ScaleOut => scale_out_candidates.as_slice(),
        IndustryStage::Governance => governance_candidates.as_slice(),
        IndustryStage::Bootstrap => return Vec::new(),
    };
    candidates
        .into_iter()
        .filter(|candidate| enabled(candidate.action_id.as_str()))
        .take(3)
        .cloned()
        .collect()
}

fn effective_branch_stage_status(
    stage_status: PlayerGameplayStageStatus,
    recommendations: &[PlayerGameplayBranchCommitment],
) -> PlayerGameplayStageStatus {
    if stage_status == PlayerGameplayStageStatus::BranchReady && recommendations.is_empty() {
        PlayerGameplayStageStatus::Blocked
    } else {
        stage_status
    }
}

pub(super) fn build_player_gameplay_snapshot(
    state: &WorldState,
    controlled_agent_id: Option<&str>,
    confirmed_gameplay_progress: bool,
    recent_feedback: Option<&PlayerGameplayRecentFeedback>,
    causality_signal: Option<&PlayerGameplayCausalitySignal>,
    gameplay_enabled: bool,
    gameplay_disabled_reason: Option<&str>,
    supports_agent_chat: bool,
    first_agent_claim_target_available: bool,
    agent_claim: Option<PlayerAgentClaimSnapshot>,
) -> PlayerGameplaySnapshot {
    let mut available_actions = base_available_actions(
        controlled_agent_id,
        gameplay_enabled,
        gameplay_disabled_reason,
        supports_agent_chat,
    );
    if gameplay_enabled {
        extend_available_actions(
            state,
            controlled_agent_id,
            first_agent_claim_target_available,
            &mut available_actions,
        );
    }
    let industry_stage = state.industry_progress.stage;
    let finalize = |gameplay| {
        finalize_player_gameplay_snapshot(
            gameplay,
            industry_stage,
            recent_feedback,
            causality_signal,
        )
    };
    if !gameplay_enabled {
        let disabled_reason = gameplay_disabled_reason
            .unwrap_or("gameplay requires runtime live server running with --llm");
        return finalize(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::FirstSessionLoop,
            stage_status: PlayerGameplayStageStatus::Blocked,
            execution_state: PlayerGameplayExecutionState::Executing,
            accepted_intent_id: None,
            intent_summary: None,
            intent_scope: None,
            intent_target: None,
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
            status_reason: None,
            last_world_change: None,
            causality_kind: None,
            causality_detail: None,
            branch_hint: None,
            resume_anchor: None,
            primary_blocker: None,
            response_window_class: None,
            stalled_reason: None,
            escalation_hint: None,
            fallback_action_id: None,
            fallback_action_label: None,
            resume_next_step: None,
            branch_recommendations: Vec::new(),
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
            small_player_lane_id: None,
            leverage_class: None,
            same_loop_repeat_count: 0,
            grind_only_flag: false,
            major_power_dependency_status: None,
            recovery_path_kind: None,
            recovery_path_detail: None,
            requires_major_power_sponsorship: None,
            repair_available: None,
            rebuild_available: None,
            pivot_available: None,
        });
    }
    let primary_factory = primary_factory_for_player_gameplay(state);
    let latest_blocker = primary_factory.and_then(|factory| {
        let kind = factory.production.current_blocker_kind.as_ref()?;
        let detail = factory
            .production
            .current_blocker_detail
            .clone()
            .unwrap_or_else(|| format!("factory={}", factory.factory_id));
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
    let has_confirmed_world_progress = has_first_session_feedback || confirmed_gameplay_progress;
    let has_material_flow = state.industry_progress.completed_material_transits > 0;
    let has_factory_ready = primary_factory.is_some();
    let has_recipe_running = primary_factory
        .is_some_and(|factory| factory.production.status == FactoryProductionStatus::Running);
    let has_first_output = primary_factory.is_some_and(|factory| {
        factory.production.completed_jobs > 0 || factory.production.last_completed_at.is_some()
    });
    let has_blocked_history =
        primary_factory.is_some_and(|factory| factory.production.last_blocked_at.is_some());
    let has_recovery_history =
        primary_factory.is_some_and(|factory| factory.production.last_resumed_at.is_some());
    let same_loop_repeat_count = primary_factory
        .map(|factory| factory.production.same_recipe_repeat_count)
        .unwrap_or(0);
    if !has_confirmed_world_progress
        && !has_material_flow
        && !has_factory_ready
        && !has_recipe_running
        && !has_first_output
        && latest_blocker.is_none()
    {
        if let Some((blocker_kind, blocker_detail, next_step_hint)) =
            first_session_runtime_sync_blocker(recent_feedback)
        {
            let disabled_reason =
                "committed runtime sync is unavailable; refresh the snapshot or repair runtime bootstrap first"
                    .to_string();
            for action in &mut available_actions {
                if action.protocol_action == "request_snapshot"
                    || (state.agents.is_empty() && action.action_id == ACTION_CLAIM_FIRST_AGENT)
                {
                    continue;
                }
                action.disabled_reason = Some(disabled_reason.clone());
            }
            return finalize(PlayerGameplaySnapshot {
                stage_id: PlayerGameplayStageId::FirstSessionLoop,
                stage_status: PlayerGameplayStageStatus::Blocked,
                execution_state: PlayerGameplayExecutionState::Executing,
                accepted_intent_id: None,
                intent_summary: None,
                intent_scope: None,
                intent_target: None,
                goal_id: "first_session_loop.recover_runtime_sync".to_string(),
                goal_kind: PlayerGameplayGoalKind::CreateFirstWorldFeedback,
                goal_title: "Recover committed runtime sync".to_string(),
                objective: "Repair the committed runtime feed before retrying the first world-feedback loop.".to_string(),
                progress_detail:
                    "The first-session loop is blocked because the viewer cannot read a committed runtime world yet."
                        .to_string(),
                progress_percent: 0,
                blocker_kind: Some(blocker_kind),
                blocker_detail: Some(blocker_detail),
                next_step_hint,
                status_reason: None,
                last_world_change: None,
                causality_kind: None,
                causality_detail: None,
                branch_hint: None,
                resume_anchor: None,
                primary_blocker: None,
                response_window_class: None,
                stalled_reason: None,
                escalation_hint: None,
                fallback_action_id: None,
                fallback_action_label: None,
                resume_next_step: None,
                branch_recommendations: Vec::new(),
                available_actions,
                recent_feedback: recent_feedback.cloned(),
                agent_claim,
                small_player_lane_id: None,
                leverage_class: None,
                same_loop_repeat_count: 0,
                grind_only_flag: false,
                major_power_dependency_status: None,
                recovery_path_kind: None,
                recovery_path_detail: None,
                requires_major_power_sponsorship: None,
                repair_available: None,
                rebuild_available: None,
                pivot_available: None,
            });
        }
    }

    if !has_confirmed_world_progress
        && !has_material_flow
        && !has_factory_ready
        && !has_recipe_running
        && !has_first_output
        && latest_blocker.is_none()
    {
        if let Some(action) = available_actions
            .iter_mut()
            .find(|action| action.action_id == "advance_step")
        {
            action.label = "Advance 1 step to create the first world feedback".to_string();
        }
        return finalize(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::FirstSessionLoop,
            stage_status: PlayerGameplayStageStatus::Active,
            execution_state: PlayerGameplayExecutionState::Executing,
            accepted_intent_id: None,
            intent_summary: None,
            intent_scope: None,
            intent_target: None,
            goal_id: "first_session_loop.create_first_world_feedback".to_string(),
            goal_kind: PlayerGameplayGoalKind::CreateFirstWorldFeedback,
            goal_title: "Create the first visible world feedback".to_string(),
            objective: "Advance the world once and confirm that your action produces a visible state or event delta.".to_string(),
            progress_detail: "You are still in the initial action loop; the first feedback has not been confirmed yet.".to_string(),
            progress_percent: 0,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Request a snapshot, advance 1 step, then inspect the new delta and events.".to_string(),
            status_reason: None,
            last_world_change: None,
            causality_kind: None,
            causality_detail: None,
            branch_hint: None,
            resume_anchor: None,
            primary_blocker: None,
            response_window_class: None,
            stalled_reason: None,
            escalation_hint: None,
            fallback_action_id: None,
            fallback_action_label: None,
            resume_next_step: None,
            branch_recommendations: Vec::new(),
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
            small_player_lane_id: None,
            leverage_class: None,
            same_loop_repeat_count: 0,
            grind_only_flag: false,
            major_power_dependency_status: None,
            recovery_path_kind: None,
            recovery_path_detail: None,
            requires_major_power_sponsorship: None,
            repair_available: None,
            rebuild_available: None,
            pivot_available: None,
        });
    }

    let fallback_feedback_blocker = if latest_blocker.is_none()
        && primary_factory
            .is_none_or(|factory| factory.production.status == FactoryProductionStatus::Blocked)
    {
        blocked_feedback
    } else {
        None
    };

    if let Some((blocker_kind, blocker_detail)) = latest_blocker.or(fallback_feedback_blocker) {
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
        return finalize(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Blocked,
            execution_state: PlayerGameplayExecutionState::Executing,
            accepted_intent_id: None,
            intent_summary: None,
            intent_scope: None,
            intent_target: None,
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
            status_reason: None,
            last_world_change: None,
            causality_kind: None,
            causality_detail: None,
            branch_hint: None,
            resume_anchor: None,
            primary_blocker: None,
            response_window_class: None,
            stalled_reason: None,
            escalation_hint: None,
            fallback_action_id: None,
            fallback_action_label: None,
            resume_next_step: None,
            branch_recommendations: Vec::new(),
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
            small_player_lane_id: None,
            leverage_class: None,
            same_loop_repeat_count: 0,
            grind_only_flag: false,
            major_power_dependency_status: None,
            recovery_path_kind: None,
            recovery_path_detail: None,
            requires_major_power_sponsorship: None,
            repair_available: None,
            rebuild_available: None,
            pivot_available: None,
        });
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
                return finalize(PlayerGameplaySnapshot {
                    stage_id: PlayerGameplayStageId::PostOnboarding,
                    stage_status: PlayerGameplayStageStatus::Active,
                    execution_state: PlayerGameplayExecutionState::Executing,
                    accepted_intent_id: None,
                    intent_summary: None,
                    intent_scope: None,
                    intent_target: None,
                    goal_id: "post_onboarding.stabilize_first_line_after_output".to_string(),
                    goal_kind: PlayerGameplayGoalKind::StabilizeFirstLine,
                    goal_title: "Harden your first output into resilient production".to_string(),
                    objective: "One visible output is not enough. Keep the first line alive until it survives interruption and resumes as a repeatable capability.".to_string(),
                    progress_detail,
                    progress_percent,
                    blocker_kind: None,
                    blocker_detail: None,
                    next_step_hint,
                    status_reason: None,
                    last_world_change: None,
                    causality_kind: None,
                    causality_detail: None,
                    branch_hint: None,
                    resume_anchor: None,
                    primary_blocker: None,
                    response_window_class: None,
                    stalled_reason: None,
                    escalation_hint: None,
                    fallback_action_id: None,
                    fallback_action_label: None,
                    resume_next_step: None,
                    branch_recommendations: Vec::new(),
                    available_actions,
                    recent_feedback: recent_feedback.cloned(),
                    agent_claim,
                    small_player_lane_id: None,
                    leverage_class: None,
                    same_loop_repeat_count,
                    grind_only_flag: false,
                    major_power_dependency_status: None,
                    recovery_path_kind: None,
                    recovery_path_detail: None,
                    requires_major_power_sponsorship: None,
                    repair_available: None,
                    rebuild_available: None,
                    pivot_available: None,
                });
            }
            IndustryStage::ScaleOut => {
                return finalize(PlayerGameplaySnapshot {
                    stage_id: PlayerGameplayStageId::PostOnboarding,
                    stage_status: PlayerGameplayStageStatus::BranchReady,
                    execution_state: PlayerGameplayExecutionState::Executing,
                    accepted_intent_id: None,
                    intent_summary: None,
                    intent_scope: None,
                    intent_target: None,
                    goal_id: "post_onboarding.choose_first_expansion_tradeoff".to_string(),
                    goal_kind: PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff,
                    goal_title: "Choose the first expansion tradeoff".to_string(),
                    objective: "The first line is stable enough to grow. Pick whether the next investment should buy more throughput, stronger resilience, or wider logistics reach.".to_string(),
                    progress_detail: "Stage progress: bootstrap is complete and the first expansion tradeoff is now unlocked.".to_string(),
                    progress_percent: 92,
                    blocker_kind: None,
                    blocker_detail: None,
                    next_step_hint: "Advance again and commit to one tradeoff: add capacity, protect upstream inputs, or widen distribution coverage.".to_string(),
                    status_reason: None,
                    last_world_change: None,
                    causality_kind: None,
                    causality_detail: None,
                    branch_hint: Some(
                        "Tradeoffs unlocked: throughput expansion / input resilience / logistics reach"
                            .to_string(),
                    ),
                    resume_anchor: None,
                    primary_blocker: None,
                    response_window_class: None,
                    stalled_reason: None,
                    escalation_hint: None,
                    fallback_action_id: None,
                    fallback_action_label: None,
                    resume_next_step: None,
                    branch_recommendations: Vec::new(),
                    available_actions,
                    recent_feedback: recent_feedback.cloned(),
                    agent_claim,
                    small_player_lane_id: None,
                    leverage_class: None,
                    same_loop_repeat_count,
                    grind_only_flag: false,
                    major_power_dependency_status: None,
                    recovery_path_kind: None,
                    recovery_path_detail: None,
                    requires_major_power_sponsorship: None,
                    repair_available: None,
                    rebuild_available: None,
                    pivot_available: None,
                });
            }
            IndustryStage::Governance => {
                return finalize(PlayerGameplaySnapshot {
                    stage_id: PlayerGameplayStageId::PostOnboarding,
                    stage_status: PlayerGameplayStageStatus::BranchReady,
                    execution_state: PlayerGameplayExecutionState::Executing,
                    accepted_intent_id: None,
                    intent_summary: None,
                    intent_scope: None,
                    intent_target: None,
                    goal_id: "post_onboarding.choose_midloop_path".to_string(),
                    goal_kind: PlayerGameplayGoalKind::ChooseMidLoopPath,
                    goal_title: "Choose your mid-loop path".to_string(),
                    objective: "Your first sustainable industrial capability is online. Expand it into stable organizational momentum.".to_string(),
                    progress_detail: "Stage progress: the first expansion tradeoff is behind you and wider mid-loop branches are now meaningful.".to_string(),
                    progress_percent: 100,
                    blocker_kind: None,
                    blocker_detail: None,
                    next_step_hint: "Keep advancing and either expand production, push governance, or secure a critical node.".to_string(),
                    status_reason: None,
                    last_world_change: None,
                    causality_kind: None,
                    causality_detail: None,
                    branch_hint: Some(
                        "Branches unlocked: production expansion / governance influence / conflict security"
                            .to_string(),
                    ),
                    resume_anchor: None,
                    primary_blocker: None,
                    response_window_class: None,
                    stalled_reason: None,
                    escalation_hint: None,
                    fallback_action_id: None,
                    fallback_action_label: None,
                    resume_next_step: None,
                    branch_recommendations: Vec::new(),
                    available_actions,
                    recent_feedback: recent_feedback.cloned(),
                    agent_claim,
                    small_player_lane_id: None,
                    leverage_class: None,
                    same_loop_repeat_count: 0,
                    grind_only_flag: false,
                    major_power_dependency_status: None,
                    recovery_path_kind: None,
                    recovery_path_detail: None,
                    requires_major_power_sponsorship: None,
                    repair_available: None,
                    rebuild_available: None,
                    pivot_available: None,
                });
            }
        }
    }

    if has_recipe_running {
        return finalize(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Active,
            execution_state: PlayerGameplayExecutionState::Executing,
            accepted_intent_id: None,
            intent_summary: None,
            intent_scope: None,
            intent_target: None,
            goal_id: "post_onboarding.stabilize_first_line".to_string(),
            goal_kind: PlayerGameplayGoalKind::StabilizeFirstLine,
            goal_title: "Stabilize your first line".to_string(),
            objective: "Keep the first production line moving until it yields stable output or exposes a clear blocker.".to_string(),
            progress_detail: "Stage progress: the first line is running; now watch for output and stoppage reasons.".to_string(),
            progress_percent: 72,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Advance 1-2 more times and watch for output, recovery, or blocker feedback.".to_string(),
            status_reason: None,
            last_world_change: None,
            causality_kind: None,
            causality_detail: None,
            branch_hint: None,
            resume_anchor: None,
            primary_blocker: None,
            response_window_class: None,
            stalled_reason: None,
            escalation_hint: None,
            fallback_action_id: None,
            fallback_action_label: None,
            resume_next_step: None,
            branch_recommendations: Vec::new(),
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
            small_player_lane_id: None,
            leverage_class: None,
            same_loop_repeat_count,
            grind_only_flag: false,
            major_power_dependency_status: None,
            recovery_path_kind: None,
            recovery_path_detail: None,
            requires_major_power_sponsorship: None,
            repair_available: None,
            rebuild_available: None,
            pivot_available: None,
        });
    }

    if has_factory_ready {
        return finalize(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Active,
            execution_state: PlayerGameplayExecutionState::Executing,
            accepted_intent_id: None,
            intent_summary: None,
            intent_scope: None,
            intent_target: None,
            goal_id: "post_onboarding.start_factory_run".to_string(),
            goal_kind: PlayerGameplayGoalKind::StartFactoryRun,
            goal_title: "Start your first factory run".to_string(),
            objective: "Turn the factory you built into a running, repeatable capability.".to_string(),
            progress_detail: "Stage progress: the factory is ready; one visible production push remains.".to_string(),
            progress_percent: 54,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Keep advancing until the factory starts a recipe, yields output, or returns a blocker.".to_string(),
            status_reason: None,
            last_world_change: None,
            causality_kind: None,
            causality_detail: None,
            branch_hint: None,
            resume_anchor: None,
            primary_blocker: None,
            response_window_class: None,
            stalled_reason: None,
            escalation_hint: None,
            fallback_action_id: None,
            fallback_action_label: None,
            resume_next_step: None,
            branch_recommendations: Vec::new(),
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
            small_player_lane_id: None,
            leverage_class: None,
            same_loop_repeat_count,
            grind_only_flag: false,
            major_power_dependency_status: None,
            recovery_path_kind: None,
            recovery_path_detail: None,
            requires_major_power_sponsorship: None,
            repair_available: None,
            rebuild_available: None,
            pivot_available: None,
        });
    }

    if has_material_flow {
        return finalize(PlayerGameplaySnapshot {
            stage_id: PlayerGameplayStageId::PostOnboarding,
            stage_status: PlayerGameplayStageStatus::Active,
            execution_state: PlayerGameplayExecutionState::Executing,
            accepted_intent_id: None,
            intent_summary: None,
            intent_scope: None,
            intent_target: None,
            goal_id: "post_onboarding.turn_material_flow_into_output".to_string(),
            goal_kind: PlayerGameplayGoalKind::TurnMaterialFlowIntoOutput,
            goal_title: "Turn material flow into output".to_string(),
            objective: "Do not stop at one-off harvesting; push the resource flow into visible output.".to_string(),
            progress_detail: "Stage progress: base resources are moving; now convert them into the first sustainable capability.".to_string(),
            progress_percent: 38,
            blocker_kind: None,
            blocker_detail: None,
            next_step_hint: "Keep harvesting, refining, building, or starting the first recipe until stable output appears.".to_string(),
            status_reason: None,
            last_world_change: None,
            causality_kind: None,
            causality_detail: None,
            branch_hint: None,
            resume_anchor: None,
            primary_blocker: None,
            response_window_class: None,
            stalled_reason: None,
            escalation_hint: None,
            fallback_action_id: None,
            fallback_action_label: None,
            resume_next_step: None,
            branch_recommendations: Vec::new(),
            available_actions,
            recent_feedback: recent_feedback.cloned(),
            agent_claim,
            small_player_lane_id: None,
            leverage_class: None,
            same_loop_repeat_count: 0,
            grind_only_flag: false,
            major_power_dependency_status: None,
            recovery_path_kind: None,
            recovery_path_detail: None,
            requires_major_power_sponsorship: None,
            repair_available: None,
            rebuild_available: None,
            pivot_available: None,
        });
    }

    finalize(PlayerGameplaySnapshot {
        stage_id: PlayerGameplayStageId::PostOnboarding,
        stage_status: PlayerGameplayStageStatus::Active,
        execution_state: PlayerGameplayExecutionState::Executing,
        accepted_intent_id: None,
        intent_summary: None,
        intent_scope: None,
        intent_target: None,
        goal_id: "post_onboarding.establish_first_capability".to_string(),
        goal_kind: PlayerGameplayGoalKind::EstablishFirstCapability,
        goal_title: "Establish your first sustainable capability".to_string(),
        objective: "The first-session action loop is complete. Create your first sustainable industrial result instead of repeating the tutorial.".to_string(),
        progress_detail: "Stage progress: you have moved from 'can operate' into the start of 'can manage'.".to_string(),
        progress_percent: 20,
        blocker_kind: None,
        blocker_detail: None,
        next_step_hint: "Advance 2-3 more times and prioritize the first output, the first stable line, or one clear recovery signal.".to_string(),
        status_reason: None,
        last_world_change: None,
        causality_kind: None,
        causality_detail: None,
        branch_hint: None,
        resume_anchor: None,
        primary_blocker: None,
        response_window_class: None,
        stalled_reason: None,
        escalation_hint: None,
        fallback_action_id: None,
        fallback_action_label: None,
        resume_next_step: None,
        branch_recommendations: Vec::new(),
        available_actions,
        recent_feedback: recent_feedback.cloned(),
        agent_claim,
        small_player_lane_id: None,
        leverage_class: None,
        same_loop_repeat_count: 0,
        grind_only_flag: false,
        major_power_dependency_status: None,
        recovery_path_kind: None,
        recovery_path_detail: None,
        requires_major_power_sponsorship: None,
        repair_available: None,
        rebuild_available: None,
        pivot_available: None,
    })
}

#[cfg(test)]
mod branch_commitment_tests {
    use super::*;

    #[test]
    fn branch_ready_downgrades_when_every_published_candidate_is_disabled() {
        let actions = vec![PlayerGameplayAction {
            action_id: "schedule_recipe_smelter_alloy_plate".to_string(),
            label: "Schedule alloy plate".to_string(),
            protocol_action: "gameplay_action.submit".to_string(),
            target_agent_id: None,
            disabled_reason: Some("missing iron ingot".to_string()),
        }];
        let recommendations = branch_recommendations(
            IndustryStage::ScaleOut,
            PlayerGameplayStageStatus::BranchReady,
            actions.as_slice(),
        );

        assert!(recommendations.is_empty());
        assert_eq!(
            effective_branch_stage_status(
                PlayerGameplayStageStatus::BranchReady,
                recommendations.as_slice(),
            ),
            PlayerGameplayStageStatus::Blocked,
        );
    }
}
