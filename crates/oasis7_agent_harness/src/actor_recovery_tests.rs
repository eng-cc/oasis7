use super::tests::{WaitProvider, request};
use crate::authority::test_support::FixtureAuthority;
use crate::{AsyncAgentRunner, FeedbackHistoryProjection};
use oasis7_agent_api::{CognitionError, Digest32, FeedbackEnvelopeV1};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn feedback(agent: &str, session: &str, seq: u64, feedback_id: &str) -> FeedbackEnvelopeV1 {
    FeedbackEnvelopeV1 {
        feedback_id: feedback_id.to_string(),
        feedback_seq: seq,
        agent_subject: agent.to_string(),
        agent_session_id: session.to_string(),
        agent_turn_id: format!("turn-{seq}"),
        decision_request_id: format!("request-{seq}"),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "rejected".to_string(),
        request_digest: Digest32(format!("blake3:{}", "a".repeat(64))),
        reject_reason: Some("stale_base".to_string()),
        provenance: "runtime_authoritative".to_string(),
    }
}

#[test]
fn feedback_history_must_be_restored_before_runner_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value>::with_default_capacity();
    runner
        .register(
            "agent-a",
            WaitProvider {
                calls: Arc::clone(&calls),
                delay: Duration::ZERO,
            },
        )
        .unwrap();

    runner
        .start_turn(request("agent-a", "request-a", "turn-a"))
        .expect_err("an un-restored private cognition store must fence dispatch");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn restored_feedback_history_fences_reused_sequence_on_runner() {
    let mut authority = FixtureAuthority::default();
    authority.feedback_history.push(FeedbackHistoryProjection {
        feedback: feedback("agent-a", "session-agent-a", 1, "restored-feedback"),
        acknowledged: true,
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value>::with_default_capacity();
    runner
        .register(
            "agent-a",
            WaitProvider {
                calls: Arc::clone(&calls),
                delay: Duration::ZERO,
            },
        )
        .unwrap();
    runner
        .restore_feedback_history(&authority)
        .expect("host-validated history restores into runner's private store");
    runner
        .restore_feedback_history(&authority)
        .expect("a fully initialized history restore is idempotent");
    assert_eq!(authority.history_reads.load(Ordering::SeqCst), 1);
    assert!(
        runner
            .cognition
            .contains_feedback("agent-a", "session-agent-a", "restored-feedback")
    );

    let request = request("agent-a", "request-live", "turn-live");
    let turn_id = runner.start_turn(request.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while runner.awaiting_outcome(turn_id).is_none() {
        runner.poll_completed().unwrap();
        assert!(
            Instant::now() < deadline,
            "provider result must become pollable"
        );
        thread::yield_now();
    }

    let mut reused_sequence = feedback(
        "agent-a",
        request.agent_session_id.as_str(),
        1,
        "different-feedback",
    );
    reused_sequence.agent_turn_id = request.agent_turn_id.clone();
    reused_sequence.decision_request_id = request.decision_request_id.clone();
    reused_sequence.request_digest = request.request_digest.clone();
    assert_eq!(
        runner
            .accept_feedback(turn_id, reused_sequence)
            .expect_err("restored sequence identity must fence a colliding feedback")
            .code(),
        "feedback_identity_collision"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn runner_refresh_rejects_active_turn_without_mutating_recovered_fences() {
    let mut partial_authority = FixtureAuthority::default();
    let pending = feedback("agent-blocked", "session-agent-blocked", 1, "pending");
    partial_authority
        .feedback_history
        .push(FeedbackHistoryProjection {
            feedback: pending.clone(),
            acknowledged: false,
        });
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value>::with_default_capacity();
    runner
        .register(
            "agent-healthy",
            WaitProvider {
                calls: Arc::clone(&calls),
                delay: Duration::from_millis(250),
            },
        )
        .unwrap();
    runner
        .restore_feedback_history(&partial_authority)
        .expect("incomplete partition is imported as a local fence");
    assert!(
        runner
            .cognition
            .feedback_recovery_blocked("agent-blocked", "session-agent-blocked")
    );

    runner
        .start_turn(request("agent-healthy", "request-healthy", "turn-healthy"))
        .expect("an unrelated recovered partition remains admissible");
    let deadline = Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "provider call should begin");
        thread::yield_now();
    }
    assert!(runner.provider_is_still_in_flight("agent-healthy"));
    assert_eq!(
        runner
            .restore_feedback_history(&FixtureAuthority::default())
            .expect_err("refresh cannot replace verifier state during an active turn")
            .code(),
        "feedback_recovery_active_turn"
    );
    assert!(runner.provider_is_still_in_flight("agent-healthy"));
    assert!(
        runner
            .cognition
            .feedback_recovery_blocked("agent-blocked", "session-agent-blocked")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_refresh_fences_runner_until_a_new_snapshot_is_validated() {
    let mut authority = FixtureAuthority::default();
    authority.feedback_history.push(FeedbackHistoryProjection {
        feedback: feedback("agent-blocked", "session-agent-blocked", 1, "pending"),
        acknowledged: false,
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value>::with_default_capacity();
    runner
        .register(
            "agent-healthy",
            WaitProvider {
                calls: Arc::clone(&calls),
                delay: Duration::ZERO,
            },
        )
        .unwrap();
    runner
        .restore_feedback_history(&authority)
        .expect("partial host history is loaded with its session fence");

    authority.feedback_history_error = Some(CognitionError::new(
        "fixture_history_unavailable",
        "history unavailable during refresh",
    ));
    assert_eq!(
        runner
            .restore_feedback_history(&authority)
            .expect_err("failed refresh cannot authorize dispatch from a stale snapshot")
            .code(),
        "fixture_history_unavailable"
    );
    assert_eq!(
        runner
            .start_turn(request("agent-healthy", "request-healthy", "turn-healthy"))
            .expect_err("a failed refresh fences all new dispatch until recovery succeeds")
            .code(),
        "feedback_recovery_required"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    authority.feedback_history_error = None;
    runner
        .restore_feedback_history(&authority)
        .expect("a later validated snapshot restores admission");
    runner
        .start_turn(request("agent-healthy", "request-healthy", "turn-healthy"))
        .expect("dispatch resumes after successful recovery");
    assert_eq!(authority.history_reads.load(Ordering::SeqCst), 3);
}

#[test]
fn failed_history_read_keeps_runner_fenced_until_retry_succeeds() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut runner = AsyncAgentRunner::<Value, Value>::with_default_capacity();
    runner
        .register(
            "agent-a",
            WaitProvider {
                calls: Arc::clone(&calls),
                delay: Duration::ZERO,
            },
        )
        .unwrap();
    let mut authority = FixtureAuthority {
        feedback_history_error: Some(CognitionError::new(
            "fixture_history_unavailable",
            "history unavailable",
        )),
        ..FixtureAuthority::default()
    };
    assert_eq!(
        runner
            .restore_feedback_history(&authority)
            .expect_err("failed host validation cannot be treated as restored")
            .code(),
        "fixture_history_unavailable"
    );
    assert_eq!(
        runner
            .start_turn(request("agent-a", "request-a", "turn-a"))
            .expect_err("dispatch remains fenced after failed history read")
            .code(),
        "feedback_recovery_required"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    authority.feedback_history_error = None;
    runner
        .restore_feedback_history(&authority)
        .expect("a later host read can recover the uninitialized store");
    runner
        .start_turn(request("agent-a", "request-a", "turn-a"))
        .expect("dispatch follows a successful recovery");
    assert_eq!(authority.history_reads.load(Ordering::SeqCst), 2);
}
