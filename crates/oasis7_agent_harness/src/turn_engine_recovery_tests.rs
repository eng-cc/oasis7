use super::{TurnEngine, TurnEngineError, TurnRequest};
use crate::actor::tests::request;
use crate::authority::test_support::FixtureAuthority;
use crate::{
    AsyncAgentRunnerError, DecisionProvider, DecisionProviderError, FeedbackHistoryProjection,
    MemoryWriteStore,
};
use oasis7_agent_api::{
    CognitionError, ContinuousAgentRequestContextV1, ContinuousAgentResponseContextV1,
    ContinuousAgentTurnContextV1, DecisionResponse, Digest32, FeedbackEnvelopeV1, GoalSnapshotV1,
    MemoryContextSnapshotV1,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn context(request: &ContinuousAgentRequestContextV1) -> ContinuousAgentTurnContextV1 {
    ContinuousAgentTurnContextV1 {
        agent_id: request.agent_subject.clone(),
        agent_session_id: request.agent_session_id.clone(),
        agent_turn_id: request.agent_turn_id.clone(),
        decision_request_id: request.decision_request_id.clone(),
        request_digest: request.request_digest.clone(),
        memory_snapshot: MemoryContextSnapshotV1::empty("turn_private"),
        goal_snapshot: GoalSnapshotV1::empty(),
        continuation: None,
    }
}

struct CountingWaitProvider(Arc<AtomicUsize>);

impl DecisionProvider for CountingWaitProvider {
    fn provider_id(&self) -> &str {
        "counting-wait"
    }

    fn decide(
        &mut self,
        request: &ContinuousAgentRequestContextV1,
    ) -> Result<ContinuousAgentResponseContextV1, DecisionProviderError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let base = DecisionResponse::wait("wait");
        let mut response = ContinuousAgentResponseContextV1 {
            base_decision_response: base,
            context_discriminator: request.context_discriminator.clone(),
            context_version: request.context_version,
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            retry_seq: request.retry_seq,
            transport_attempt: request.transport_attempt,
            request_digest: request.request_digest.clone(),
            response_digest: Digest32::default(),
        };
        response.response_digest =
            crate::provider::cognition_response_digest(&response.base_decision_response);
        Ok(response)
    }
}

fn feedback(
    request: &ContinuousAgentRequestContextV1,
    feedback_seq: u64,
    feedback_id: &str,
) -> FeedbackEnvelopeV1 {
    FeedbackEnvelopeV1 {
        feedback_id: feedback_id.into(),
        feedback_seq,
        agent_subject: request.agent_subject.clone(),
        agent_session_id: request.agent_session_id.clone(),
        agent_turn_id: request.agent_turn_id.clone(),
        decision_request_id: request.decision_request_id.clone(),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "rejected".into(),
        request_digest: request.request_digest.clone(),
        reject_reason: Some("stale_base".into()),
        provenance: "runtime_authoritative".into(),
    }
}

#[test]
fn runtime_history_is_restored_before_engine_dispatch_and_fences_sequence_collision() {
    let request = request("agent-a", "request-a", "turn-a");
    let mut host = FixtureAuthority::default();
    host.feedback_history.push(FeedbackHistoryProjection {
        feedback: FeedbackEnvelopeV1 {
            feedback_id: "restored-feedback".into(),
            feedback_seq: 1,
            agent_subject: request.agent_subject.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: "turn-old".into(),
            decision_request_id: "request-old".into(),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "rejected".into(),
            request_digest: Digest32(format!("blake3:{}", "a".repeat(64))),
            reject_reason: Some("stale_base".into()),
            provenance: "runtime_authoritative".into(),
        },
        acknowledged: true,
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
    engine
        .register("agent-a", CountingWaitProvider(Arc::clone(&calls)))
        .unwrap();

    let turn_id = engine
        .start_turn(TurnRequest::new(context(&request), request.clone()), &host)
        .expect("engine restores host history before its first provider dispatch");
    assert_eq!(host.history_reads.load(Ordering::SeqCst), 1);
    let deadline = Instant::now() + Duration::from_secs(2);
    while engine.poll_completed().unwrap().is_empty() {
        assert!(
            Instant::now() < deadline,
            "provider result must become pollable"
        );
        std::thread::yield_now();
    }
    let collision = feedback(&request, 1, "different-feedback");
    let mut memory = MemoryWriteStore::default();
    let error = engine
        .accept_runtime_feedback(turn_id, collision, None, &host, &mut memory)
        .expect_err("the engine's restored private store must reject sequence reuse");
    assert!(matches!(
        error,
        TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(ref cognition))
            if cognition.code() == "feedback_identity_collision"
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_runtime_history_read_keeps_engine_dispatch_fenced_and_can_retry() {
    let request = request("agent-a", "request-a", "turn-a");
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
    engine
        .register("agent-a", CountingWaitProvider(Arc::clone(&calls)))
        .unwrap();
    let mut host = FixtureAuthority {
        feedback_history_error: Some(CognitionError::new(
            "fixture_history_unavailable",
            "history unavailable",
        )),
        ..FixtureAuthority::default()
    };
    let error = engine
        .start_turn(TurnRequest::new(context(&request), request.clone()), &host)
        .expect_err("history failure must happen before provider dispatch");
    assert!(matches!(
        error,
        TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(ref cognition))
            if cognition.code() == "fixture_history_unavailable"
    ));
    assert_eq!(engine.active_turn_count(), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    host.feedback_history_error = None;
    engine
        .start_turn(TurnRequest::new(context(&request), request), &host)
        .expect("engine can retry recovery before dispatch");
    assert_eq!(host.history_reads.load(Ordering::SeqCst), 2);
}

#[test]
fn blocked_runtime_history_fences_only_its_engine_session() {
    let blocked_request = request("agent-a", "request-blocked", "turn-blocked");
    let mut host = FixtureAuthority::default();
    host.feedback_history.push(FeedbackHistoryProjection {
        feedback: FeedbackEnvelopeV1 {
            feedback_id: "pending-feedback".into(),
            feedback_seq: 1,
            agent_subject: blocked_request.agent_subject.clone(),
            agent_session_id: blocked_request.agent_session_id.clone(),
            agent_turn_id: "turn-old".into(),
            decision_request_id: "request-old".into(),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "rejected".into(),
            request_digest: Digest32(format!("blake3:{}", "a".repeat(64))),
            reject_reason: Some("stale_base".into()),
            provenance: "runtime_authoritative".into(),
        },
        acknowledged: false,
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
    engine
        .register("agent-a", CountingWaitProvider(Arc::clone(&calls)))
        .unwrap();
    let blocked_error = engine
        .start_turn(
            TurnRequest::new(context(&blocked_request), blocked_request.clone()),
            &host,
        )
        .expect_err("an unacknowledged restored partition must fence its session");
    assert!(matches!(
        blocked_error,
        TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(ref cognition))
            if cognition.code() == "feedback_recovery_blocked"
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let mut healthy_request = request("agent-a", "request-healthy", "turn-healthy");
    healthy_request.agent_session_id = "session-healthy".into();
    healthy_request.request_digest = healthy_request.request_digest();
    healthy_request
        .validate_production_lane()
        .expect("healthy fixture request remains canonical");
    engine
        .start_turn(
            TurnRequest::new(context(&healthy_request), healthy_request),
            &host,
        )
        .expect("a healthy session of the same Agent remains admissible");
    let deadline = Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "provider call should begin");
        std::thread::yield_now();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.history_reads.load(Ordering::SeqCst), 2);
}
