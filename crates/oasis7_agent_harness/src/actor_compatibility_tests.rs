use super::tests::request;
use crate::FeedbackHistoryProjection;
use crate::authority::test_support::FixtureAuthority;
use crate::{
    AsyncAgentRunner, AsyncAgentRunnerError, AsyncTurnId, CompatibilityDecisionProvider,
    CompatibilityTurnOutcome, CompatibilityTurnRequest, DecisionProvider, DecisionProviderError,
    TurnHandoffDispatch,
};
use oasis7_agent_api::{
    ContinuousAgentRequestContextV1, ContinuousAgentResponseContextV1,
    ContinuousAgentTurnContextV1, DecisionResponse, GoalSnapshotV1, MemoryContextSnapshotV1,
};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn reduced_context(
    agent: &str,
    session: &str,
    turn: &str,
    request: &str,
) -> ContinuousAgentTurnContextV1 {
    ContinuousAgentTurnContextV1 {
        agent_id: agent.into(),
        agent_session_id: session.into(),
        agent_turn_id: turn.into(),
        decision_request_id: request.into(),
        request_digest: format!("blake3:{}", "a".repeat(64)).into(),
        memory_snapshot: MemoryContextSnapshotV1::empty("compatibility-test"),
        goal_snapshot: GoalSnapshotV1::empty(),
        continuation: None,
    }
}

fn history_feedback(
    agent: &str,
    session: &str,
    turn: &str,
    request: &str,
) -> oasis7_agent_api::FeedbackEnvelopeV1 {
    oasis7_agent_api::FeedbackEnvelopeV1 {
        feedback_id: "restored-feedback".into(),
        feedback_seq: 1,
        agent_subject: agent.into(),
        agent_session_id: session.into(),
        agent_turn_id: turn.into(),
        decision_request_id: request.into(),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "rejected".into(),
        request_digest: format!("blake3:{}", "a".repeat(64)).into(),
        reject_reason: Some("stale_base".into()),
        provenance: "runtime_authoritative".into(),
    }
}

struct CompatProvider(Arc<AtomicUsize>);

impl CompatibilityDecisionProvider<String, String, String> for CompatProvider {
    fn decide_compatibility(
        &mut self,
        request: CompatibilityTurnRequest<String>,
    ) -> Result<String, DecisionProviderError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(request.input)
    }

    fn push_compatibility_feedback(
        &mut self,
        _feedback: String,
    ) -> Result<(), DecisionProviderError> {
        Ok(())
    }
}

#[test]
fn compatibility_admission_does_not_authorize_production_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    runner
        .register_compatibility("agent-a", CompatProvider(Arc::clone(&calls)))
        .unwrap();

    let turn = runner
        .start_compatibility_turn(CompatibilityTurnRequest {
            agent_subject: "agent-a".into(),
            turn_context: Some(reduced_context("agent-a", "session-a", "turn-a", "req-a")),
            input: "legacy request".into(),
        })
        .expect("the explicitly non-production lane remains available");
    assert_eq!(
        runner
            .notify_runtime_feedback(
                "agent-a",
                oasis7_agent_api::FeedbackEnvelopeV1 {
                    feedback_id: "forbidden".into(),
                    feedback_seq: 1,
                    agent_subject: "agent-a".into(),
                    agent_session_id: "session-a".into(),
                    agent_turn_id: "turn-a".into(),
                    decision_request_id: "req-a".into(),
                    candidate_action_id: Some(1),
                    runtime_receipt_id: Some("caller-controlled".into()),
                    status: "committed".into(),
                    request_digest: format!("blake3:{}", "a".repeat(64)).into(),
                    reject_reason: None,
                    provenance: "runtime_authoritative".into(),
                },
            )
            .expect_err("production Runtime feedback cannot cross into compatibility")
            .code(),
        "feedback_lane_mismatch"
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    let outcome: CompatibilityTurnOutcome<String> = loop {
        if let Some(outcome) = runner
            .poll_compatibility_completed()
            .unwrap()
            .into_iter()
            .find(|outcome| outcome.turn_id == turn)
        {
            break outcome;
        }
        assert!(
            Instant::now() < deadline,
            "compatibility decision completes"
        );
        thread::yield_now();
    };
    assert_eq!(outcome.result.unwrap(), "legacy request");
    assert!(runner.awaiting_request(turn).is_none());
    runner
        .notify_compatibility_feedback(turn, "legacy callback".into())
        .expect("a callback for the exact compatibility turn remains available");
    runner
        .start_turn(request("agent-a", "prod-request", "prod-turn"))
        .expect_err("compatibility admission must not set production restore state");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn recovered_pending_partition_fences_compatibility_replay_and_restore_is_busy_safe() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    runner
        .register_compatibility("agent-a", CompatProvider(Arc::clone(&calls)))
        .unwrap();
    let mut history = FixtureAuthority::default();
    history.feedback_history.push(FeedbackHistoryProjection {
        feedback: oasis7_agent_api::FeedbackEnvelopeV1 {
            feedback_id: "feedback-a".into(),
            feedback_seq: 1,
            agent_subject: "agent-a".into(),
            agent_session_id: "session-a".into(),
            agent_turn_id: "turn-history".into(),
            decision_request_id: "request-history".into(),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "pending".into(),
            request_digest: format!("blake3:{}", "b".repeat(64)).into(),
            reject_reason: None,
            provenance: "runtime_authoritative".into(),
        },
        acknowledged: false,
        lineage: None,
    });
    runner.restore_feedback_history(&history).unwrap();
    let blocked = runner.start_compatibility_turn(CompatibilityTurnRequest {
        agent_subject: "agent-a".into(),
        turn_context: Some(reduced_context(
            "agent-a",
            "session-a",
            "new-turn",
            "new-request",
        )),
        input: "must not dispatch".into(),
    });
    assert!(matches!(blocked, Err(AsyncAgentRunnerError::Cognition(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let admitted = runner.start_compatibility_turn(CompatibilityTurnRequest {
        agent_subject: "agent-a".into(),
        turn_context: None,
        input: "identity-free legacy request".into(),
    });
    let _ = admitted;
    assert_eq!(
        runner
            .restore_feedback_history(&history)
            .expect_err("host recovery cannot replace fences while a compat actor is active")
            .code(),
        "feedback_recovery_active_turn"
    );
}

#[test]
fn recovered_feedback_identity_collision_fences_compatibility_feedback_on_runner() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    runner
        .register_compatibility("agent-a", CompatProvider(Arc::clone(&calls)))
        .unwrap();
    let history_feedback = history_feedback("agent-a", "session-a", "old-turn", "old-request");
    let mut authority = FixtureAuthority::default();
    authority.feedback_history.push(FeedbackHistoryProjection {
        feedback: history_feedback.clone(),
        acknowledged: true,
        lineage: None,
    });
    runner.restore_feedback_history(&authority).unwrap();

    let context = reduced_context("agent-a", "session-a", "new-turn", "new-request");
    let turn = runner
        .start_compatibility_turn(CompatibilityTurnRequest {
            agent_subject: "agent-a".into(),
            turn_context: Some(context.clone()),
            input: "replay collision candidate".into(),
        })
        .expect("the recovered terminal history does not block a new identity");
    let deadline = Instant::now() + Duration::from_secs(2);
    while runner.poll_compatibility_completed().unwrap().is_empty() {
        assert!(
            Instant::now() < deadline,
            "compatibility decision completes"
        );
        thread::yield_now();
    }

    let mut reused = history_feedback;
    reused.agent_turn_id = context.agent_turn_id;
    reused.decision_request_id = context.decision_request_id;
    reused.request_digest = context.request_digest;
    assert_eq!(
        runner
            .accept_compatibility_feedback(turn, reused)
            .expect_err("restored feedback identity must fence the reduced callback")
            .code(),
        "feedback_id_conflict"
    );
    assert!(runner.contains_feedback("agent-a", "session-a", "restored-feedback"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_host_recovery_keeps_reduced_compatibility_admission_but_not_production() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    runner
        .register_compatibility("agent-a", CompatProvider(Arc::clone(&calls)))
        .unwrap();
    let authority = FixtureAuthority {
        feedback_history_error: Some(oasis7_agent_api::CognitionError::new(
            "fixture_history_unavailable",
            "validated host feedback history is unavailable",
        )),
        ..FixtureAuthority::default()
    };
    assert_eq!(
        runner
            .restore_feedback_history(&authority)
            .expect_err("failed recovery cannot claim the production lane")
            .code(),
        "fixture_history_unavailable"
    );
    let turn = runner
        .start_compatibility_turn(CompatibilityTurnRequest {
            agent_subject: "agent-a".into(),
            turn_context: None,
            input: "legacy contract remains admitted".into(),
        })
        .expect("legacy reduced admission is not globally fenced by restoration");
    runner
        .start_turn(request("agent-a", "request-a", "turn-a"))
        .expect_err("production remains fenced after failed history recovery");
    let deadline = Instant::now() + Duration::from_secs(2);
    let result = loop {
        if let Some(outcome) = runner
            .poll_compatibility_completed()
            .unwrap()
            .into_iter()
            .find(|outcome| outcome.turn_id == turn)
        {
            break outcome.result;
        }
        assert!(
            Instant::now() < deadline,
            "compatibility decision completes"
        );
        thread::yield_now();
    };
    assert_eq!(result.unwrap(), "legacy contract remains admitted");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_compatibility_turn_cannot_be_retried_or_completed_with_production_feedback() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    runner
        .register_compatibility("agent-a", CompatProvider(Arc::clone(&calls)))
        .unwrap();
    let context = reduced_context("agent-a", "session-a", "turn-a", "request-a");
    let turn = runner
        .start_compatibility_turn(CompatibilityTurnRequest {
            agent_subject: "agent-a".into(),
            turn_context: Some(context),
            input: "legacy request".into(),
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while runner.poll_compatibility_completed().unwrap().is_empty() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert!(runner.awaiting_request(turn).is_none());
    runner
        .retry_awaiting_turn(turn, &request("agent-a", "request-a", "turn-a"))
        .expect_err("a compat completion has no production retry lineage");
    runner
        .accept_feedback(
            turn,
            oasis7_agent_api::FeedbackEnvelopeV1 {
                feedback_id: "feedback-a".into(),
                feedback_seq: 1,
                agent_subject: "agent-a".into(),
                agent_session_id: "session-a".into(),
                agent_turn_id: "turn-a".into(),
                decision_request_id: "request-a".into(),
                candidate_action_id: Some(1),
                runtime_receipt_id: Some("caller-supplied-is-not-proof".into()),
                status: "committed".into(),
                request_digest: format!("blake3:{}", "a".repeat(64)).into(),
                reject_reason: None,
                provenance: "runtime_authoritative".into(),
            },
        )
        .expect_err("a compat outcome cannot be promoted by production feedback");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[allow(dead_code)]
struct ProductionOnlyProvider;

impl DecisionProvider<Value, Value> for ProductionOnlyProvider {
    fn provider_id(&self) -> &str {
        "production-only"
    }

    fn decide(
        &mut self,
        request: &ContinuousAgentRequestContextV1,
    ) -> Result<ContinuousAgentResponseContextV1<Value, Value>, DecisionProviderError> {
        let mut response = ContinuousAgentResponseContextV1 {
            base_decision_response: DecisionResponse {
                decision: oasis7_agent_api::ProviderDecision::Wait,
                module_command: None,
                provider_error: None,
                diagnostics: Default::default(),
                trace_payload: Default::default(),
                memory_write_intents: Vec::new(),
            },
            context_discriminator: request.context_discriminator.clone(),
            context_version: request.context_version,
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            retry_seq: request.retry_seq,
            transport_attempt: request.transport_attempt,
            request_digest: request.request_digest.clone(),
            response_digest: Default::default(),
        };
        response.response_digest =
            crate::provider::cognition_response_digest(&response.base_decision_response);
        Ok(response)
    }
}

#[allow(dead_code)]
fn assert_production_only_registration_still_works() {
    let mut runner = AsyncAgentRunner::<Value, Value>::with_default_capacity();
    runner.register("agent-a", ProductionOnlyProvider).unwrap();
    let _ = runner.restore_feedback_history(&FixtureAuthority::default());
}

struct HandoffProvider {
    decisions: Arc<AtomicUsize>,
    callbacks: Arc<AtomicUsize>,
}

impl CompatibilityDecisionProvider<String, String, String> for HandoffProvider {
    fn decide_compatibility(
        &mut self,
        request: CompatibilityTurnRequest<String>,
    ) -> Result<String, DecisionProviderError> {
        self.decisions.fetch_add(1, Ordering::SeqCst);
        Ok(request.input)
    }

    fn push_compatibility_feedback(
        &mut self,
        _feedback: String,
    ) -> Result<(), DecisionProviderError> {
        self.callbacks.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn completed_compatibility_turn(
    runner: &mut AsyncAgentRunner<Value, Value, String, String, String>,
    turn_id: AsyncTurnId,
) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if runner
            .poll_compatibility_completed()
            .unwrap()
            .iter()
            .any(|outcome| outcome.turn_id == turn_id)
        {
            return;
        }
        assert!(Instant::now() < deadline, "provider completion is polled");
        thread::yield_now();
    }
}

#[test]
fn failed_handoff_dispatch_restores_only_the_exact_awaiting_turn() {
    let decisions = Arc::new(AtomicUsize::new(0));
    let callbacks = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    runner
        .register_compatibility(
            "agent-a",
            HandoffProvider {
                decisions: Arc::clone(&decisions),
                callbacks: Arc::clone(&callbacks),
            },
        )
        .unwrap();
    let old_context = reduced_context("agent-a", "session-a", "turn-a", "request-a");
    let old_turn = runner
        .start_compatibility_turn(CompatibilityTurnRequest {
            agent_subject: "agent-a".into(),
            turn_context: Some(old_context),
            input: "original".into(),
        })
        .unwrap();
    completed_compatibility_turn(&mut runner, old_turn);

    let capability = runner.begin_turn_handoff(old_turn).unwrap();
    runner
        .actors
        .get("agent-a")
        .unwrap()
        .accepting_commands
        .store(false, Ordering::Release);
    let next_context = reduced_context("agent-a", "session-a", "turn-b", "request-b");
    let next_request = CompatibilityTurnRequest {
        agent_subject: "agent-a".into(),
        turn_context: Some(next_context),
        input: "successor".into(),
    };
    assert_eq!(
        runner
            .dispatch_turn_handoff(
                capability,
                TurnHandoffDispatch::Compatibility(next_request.clone()),
            )
            .unwrap_err()
            .code(),
        "actor_unavailable"
    );
    assert_eq!(decisions.load(Ordering::SeqCst), 1);
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);
    assert_eq!(
        runner.awaiting_compatibility.get("agent-a"),
        Some(&old_turn)
    );
    assert!(runner.compatibility_turns.contains_key(&old_turn));

    runner
        .actors
        .get("agent-a")
        .unwrap()
        .accepting_commands
        .store(true, Ordering::Release);
    let retry_capability = runner.begin_turn_handoff(old_turn).unwrap();
    let next_turn = runner
        .dispatch_turn_handoff(
            retry_capability,
            TurnHandoffDispatch::Compatibility(next_request),
        )
        .expect("the exact predecessor is released once when the retry dispatch succeeds");
    assert!(!runner.compatibility_turns.contains_key(&old_turn));
    completed_compatibility_turn(&mut runner, next_turn);
    assert_eq!(decisions.load(Ordering::SeqCst), 2);
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);
}

#[test]
fn handoff_capability_rejects_another_runner_and_a_stale_turn_version() {
    let calls_a = Arc::new(AtomicUsize::new(0));
    let calls_b = Arc::new(AtomicUsize::new(0));
    let callbacks = Arc::new(AtomicUsize::new(0));
    let mut runner_a = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    let mut runner_b = AsyncAgentRunner::<Value, Value, String, String, String>::new(1).unwrap();
    for (runner, calls) in [(&mut runner_a, &calls_a), (&mut runner_b, &calls_b)] {
        runner
            .register_compatibility(
                "agent-a",
                HandoffProvider {
                    decisions: Arc::clone(calls),
                    callbacks: Arc::clone(&callbacks),
                },
            )
            .unwrap();
    }
    let first_turn = runner_a
        .start_compatibility_turn(CompatibilityTurnRequest {
            agent_subject: "agent-a".into(),
            turn_context: Some(reduced_context(
                "agent-a",
                "session-a",
                "turn-a",
                "request-a",
            )),
            input: "original".into(),
        })
        .unwrap();
    completed_compatibility_turn(&mut runner_a, first_turn);
    let foreign_runner_capability = runner_a.begin_turn_handoff(first_turn).unwrap();
    assert_eq!(
        runner_b
            .dispatch_turn_handoff(
                foreign_runner_capability,
                TurnHandoffDispatch::Compatibility(CompatibilityTurnRequest {
                    agent_subject: "agent-a".into(),
                    turn_context: Some(reduced_context(
                        "agent-a",
                        "session-a",
                        "turn-b",
                        "request-b",
                    )),
                    input: "must not dispatch".into(),
                }),
            )
            .unwrap_err()
            .code(),
        "handoff_runner_mismatch"
    );
    assert_eq!(
        runner_a.awaiting_compatibility.get("agent-a"),
        Some(&first_turn)
    );
    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(calls_b.load(Ordering::SeqCst), 0);

    let stale_capability = runner_a.begin_turn_handoff(first_turn).unwrap();
    runner_a.release_compatibility_turn(first_turn).unwrap();
    assert_eq!(
        runner_a
            .dispatch_turn_handoff(
                stale_capability,
                TurnHandoffDispatch::Compatibility(CompatibilityTurnRequest {
                    agent_subject: "agent-a".into(),
                    turn_context: Some(reduced_context(
                        "agent-a",
                        "session-a",
                        "turn-c",
                        "request-c",
                    )),
                    input: "stale predecessor".into(),
                }),
            )
            .unwrap_err()
            .code(),
        "handoff_identity_stale"
    );
    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);
}
