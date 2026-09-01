//! LIVE-1 RED fixtures for the async AgentActor/simulator boundary.
//!
//! These tests intentionally exercise the current simulator entry point as a
//! negative control.  A provider that never returns must not hold the world
//! execution worker; the target implementation is expected to move this call
//! behind a bounded actor/mailbox seam while preserving the existing provider
//! DTOs.

use super::*;
use crate::runtime::{AgentCognitionMailbox, AgentDecisionEnvelopeV1};
use crate::simulator::AsyncAgentRunner;
use crate::simulator::{
    ContinuousAgentTurnContextV1, Digest32, GoalSnapshotV1, MemoryContextSnapshotV1,
};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const AGENT_ID: &str = "agent-live-1";

fn envelope(agent_id: &str, turn: &str) -> AgentDecisionEnvelopeV1 {
    serde_json::from_value(json!({
        "schema_version": "agent-decision-envelope.v1",
        "world_id": "simulator-live-world",
        "agent_id": agent_id,
        "branch_id": "main",
        "finality_epoch": 7,
        "finality_block_hash": format!("blake3:{}", "a".repeat(64)),
        "finality_status": "verified",
        "agent_session_id": format!("session.{agent_id}"),
        "agent_turn_id": turn,
        "decision_request_id": format!("request.{turn}"),
        "retry_seq": 0,
        "base_tick": 0,
        "base_world_hash": "blake3:simulator-live-parent",
        "reorg_epoch": 3,
        "runtime_manifest_hash": "blake3:simulator-live-manifest",
        "capability_snapshot_hash": "blake3:simulator-live-capability",
        "authority_context_hash": "blake3:simulator-live-authority",
        "observation_digest": "blake3:simulator-live-observation",
        "context_digest": "blake3:simulator-live-context",
        "issued_at_tick": 0,
        "valid_until_tick": 10,
        "preconditions": [],
        "decision_kind": "wait",
        "action": {"type": "wait"},
        "request_digest": "blake3:simulator-live-request",
        "decision_digest": "blake3:simulator-live-decision",
        "envelope_digest": format!("blake3:simulator-live-{turn}"),
        "provider_invocation_key": format!("blake3:provider-{turn}"),
        "envelope_idempotency_key": format!("blake3:idempotency-{turn}"),
        "origin_intent_ref": null,
        "source": "provider"
    }))
    .expect("decode live mailbox envelope")
}

#[test]
fn world_tick_does_not_wait_for_an_outstanding_provider() {
    let mut runner = AsyncAgentRunner::blocking_provider_fixture(AGENT_ID);
    let before = runner.logical_tick();
    runner.start_turn(AGENT_ID).expect("start async turn");

    let progress = runner
        .step_world_without_waiting_for_provider()
        .expect("world worker must make bounded progress");
    assert_eq!(progress.logical_tick, before + 1);
    assert!(
        runner.provider_is_still_in_flight(AGENT_ID),
        "the provider remains outstanding while the world tick completes"
    );
}

#[test]
fn mailbox_is_bounded_and_enforces_one_active_turn_per_agent() {
    let mut mailbox = AgentCognitionMailbox::with_capacity(1, 1);
    assert!(mailbox.try_enqueue(envelope(AGENT_ID, "turn-a")).is_ok());

    let same_agent = mailbox
        .try_enqueue(envelope(AGENT_ID, "turn-b"))
        .expect_err("same agent cannot re-enter while active");
    assert_eq!(same_agent.code(), "agent_busy");

    let different_agent = mailbox
        .try_enqueue(envelope("agent-live-2", "turn-a"))
        .expect_err("bounded mailbox must reject a full queue immediately");
    assert_eq!(different_agent.code(), "mailbox_full");

    assert_eq!(
        mailbox.try_dequeue().expect("first turn").agent_id,
        AGENT_ID
    );
    mailbox
        .try_enqueue(envelope("agent-live-2", "turn-a"))
        .expect("capacity release admits the next agent");
}

struct ReleaseOnDrop(Arc<AtomicBool>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

struct ParallelWaitBehavior {
    agent_id: String,
    started: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl ParallelWaitBehavior {
    fn new(agent_id: &str, started: Arc<AtomicBool>, release: Arc<AtomicBool>) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            started,
            release,
        }
    }
}

impl AgentBehavior for ParallelWaitBehavior {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn decide(&mut self, _observation: &Observation) -> AgentDecision {
        self.started.store(true, Ordering::Release);
        while !self.release.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
        }
        AgentDecision::Wait
    }
}

#[test]
fn mailbox_capacity_is_per_actor_and_does_not_cap_parallel_turns() {
    let release = Arc::new(AtomicBool::new(false));
    let first_started = Arc::new(AtomicBool::new(false));
    let second_started = Arc::new(AtomicBool::new(false));
    let mut runner = AsyncAgentRunner::new(1).expect("create one-slot actor mailboxes");
    let _release_guard = ReleaseOnDrop(Arc::clone(&release));

    runner
        .register(ParallelWaitBehavior::new(
            "agent-live-1",
            Arc::clone(&first_started),
            Arc::clone(&release),
        ))
        .expect("register first actor");
    runner
        .register(ParallelWaitBehavior::new(
            "agent-live-2",
            Arc::clone(&second_started),
            Arc::clone(&release),
        ))
        .expect("register second actor");

    runner
        .start_turn("agent-live-1")
        .expect("first actor uses its mailbox slot");
    runner
        .start_turn("agent-live-2")
        .expect("second actor uses its own mailbox slot");
    let reentrant = runner
        .start_turn("agent-live-1")
        .expect_err("a single actor still has one active turn");
    assert_eq!(reentrant.code(), "agent_busy");
    assert_eq!(runner.active_turn_count(), 2);

    for _ in 0..500 {
        if first_started.load(Ordering::Acquire) && second_started.load(Ordering::Acquire) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        first_started.load(Ordering::Acquire) && second_started.load(Ordering::Acquire),
        "both actors must enter decide while their turns overlap"
    );
    assert!(runner.provider_is_still_in_flight("agent-live-1"));
    assert!(runner.provider_is_still_in_flight("agent-live-2"));

    release.store(true, Ordering::Release);
    let mut completed = 0;
    for _ in 0..500 {
        completed += runner.poll_completed().expect("poll parallel actors").len();
        if completed == 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        completed, 2,
        "both parallel turns must complete after release"
    );
}

#[test]
fn builtin_and_provider_backed_wait_use_the_same_lifecycle_outcome() {
    let mut builtin = AsyncAgentRunner::builtin_fixture(AGENT_ID);
    let mut provider_backed = AsyncAgentRunner::provider_backed_fixture(AGENT_ID);

    let builtin_outcome = builtin.run_one_turn().expect("builtin turn outcome");
    let provider_outcome = provider_backed
        .run_one_turn()
        .expect("provider-backed turn outcome");
    assert_eq!(builtin_outcome.lifecycle, provider_outcome.lifecycle);
    assert_eq!(builtin_outcome.feedback, provider_outcome.feedback);
    assert_eq!(builtin_outcome.world_effect, provider_outcome.world_effect);
}

#[test]
fn completed_provider_turn_remains_single_flight_until_runtime_terminal_feedback() {
    let mut runner = AsyncAgentRunner::builtin_fixture(AGENT_ID);
    let context = ContinuousAgentTurnContextV1 {
        agent_id: AGENT_ID.to_string(),
        agent_session_id: "session.agent-live-1".to_string(),
        agent_turn_id: "turn.agent-live-1".to_string(),
        decision_request_id: "request.agent-live-1".to_string(),
        request_digest: Digest32::from(
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        memory_snapshot: MemoryContextSnapshotV1::empty("session_private"),
        goal_snapshot: GoalSnapshotV1::empty(),
        continuation: None,
    };
    runner
        .start_turn_with_context(AGENT_ID, context.clone())
        .expect("open turn");
    for _ in 0..1024 {
        if !runner.poll_completed().expect("poll turn").is_empty() {
            break;
        }
        std::thread::yield_now();
    }
    let error = runner
        .start_turn_with_context(AGENT_ID, context)
        .expect_err("provider completion is not Runtime terminal feedback");
    assert_eq!(error.code(), "agent_busy");
}
