use crate::simulator::{
    AgentBehavior, AgentDecision, AgentDecisionTrace, AsyncAgentTurnOutcome, AsyncTurnFeedback,
    AsyncTurnId, AsyncTurnLifecycle, AsyncWorldEffect, FeedbackEnvelopeV1, MemoryWriteStore,
    TurnEngine, TurnEngineError, TurnEnginePhase, TurnRequest,
};

use super::agent_cognition_identity::{
    production_observation, production_request_fixture, production_turn_context, request_from_value,
};
use serde_json::json;

struct WaitBehavior;

impl AgentBehavior for WaitBehavior {
    fn agent_id(&self) -> &str {
        "agent-1"
    }

    fn decide(&mut self, _observation: &crate::simulator::Observation) -> AgentDecision {
        AgentDecision::Wait
    }
}

fn request() -> TurnRequest {
    let mut fixture = production_request_fixture(1, 60_000);
    fixture["retry_seq"] = json!(1);
    let request_context = request_from_value(fixture);
    let turn_context = production_turn_context(&request_context);
    TurnRequest::new(
        production_observation("agent-1", 42),
        turn_context,
        request_context,
    )
}

fn completed_wait_turn() -> (TurnEngine, AsyncTurnId, AsyncAgentTurnOutcome) {
    let mut engine = TurnEngine::with_default_capacity();
    engine.register(WaitBehavior).expect("register actor");
    let turn_id = engine.start_turn(request()).expect("start turn");
    let completion = loop {
        if let Some(completion) = engine.poll_completed().expect("poll actor").pop() {
            break completion;
        }
        std::thread::yield_now();
    };
    (engine, turn_id, completion.outcome)
}

#[test]
fn turn_engine_keeps_one_in_flight_turn_and_exposes_pending_phase() {
    let mut engine = TurnEngine::with_default_capacity();
    engine.register(WaitBehavior).expect("register actor");
    let turn_id = engine.start_turn(request()).expect("start turn");
    assert_eq!(engine.phase(turn_id), Some(TurnEnginePhase::InFlight));
    assert_eq!(engine.active_turn_count(), 1);
    let second = engine.start_turn(request());
    assert!(second.is_err(), "same agent must remain single-flight");

    let completion = loop {
        if let Some(completion) = engine.poll_completed().expect("poll actor").pop() {
            break completion;
        }
        std::thread::yield_now();
    };
    assert_eq!(completion.turn_id, turn_id);
    assert_eq!(completion.phase, TurnEnginePhase::Pending);
    assert_eq!(completion.outcome.agent_id, "agent-1");
    assert_eq!(completion.outcome.lifecycle, AsyncTurnLifecycle::Completed);
    assert_eq!(completion.outcome.feedback, AsyncTurnFeedback::Wait);
    assert_eq!(completion.outcome.world_effect, AsyncWorldEffect::NoEffect);
    assert_eq!(
        engine.active_turn_count(),
        1,
        "Runtime disposition is pending"
    );
}

#[test]
fn committed_feedback_requires_runtime_world_readback_proof() {
    let (mut engine, turn_id, outcome) = completed_wait_turn();
    let mut feedback = outcome
        .feedback_for_runtime_status("committed", Some("receipt-1"))
        .expect("build committed feedback");
    feedback.candidate_action_id = Some(7);
    feedback.provenance = "runtime_authoritative".to_string();
    let error = engine
        .accept_runtime_feedback(turn_id, feedback, None, &mut MemoryWriteStore::default())
        .expect_err("caller receipt alone cannot authorize committed feedback");
    assert_eq!(error, TurnEngineError::RuntimeReadbackRequired(turn_id));
}

#[test]
fn stale_base_feedback_enters_stale_phase() {
    let (mut engine, turn_id, outcome) = completed_wait_turn();
    let mut feedback = outcome
        .feedback_for_runtime_status("rejected", None)
        .expect("build rejected feedback");
    feedback.reject_reason = Some("stale_base".to_string());
    feedback.provenance = "runtime_authoritative".to_string();
    let phase = engine
        .accept_runtime_feedback(
            turn_id,
            FeedbackEnvelopeV1 { ..feedback },
            None,
            &mut MemoryWriteStore::default(),
        )
        .expect("accept authoritative stale feedback");
    assert_eq!(phase, TurnEnginePhase::Stale);
    assert_eq!(engine.phase(turn_id), Some(TurnEnginePhase::Stale));
}

#[allow(dead_code)]
fn _outcome_shape_is_stable(outcome: AsyncAgentTurnOutcome) -> Option<AgentDecisionTrace> {
    outcome.decision_trace
}
