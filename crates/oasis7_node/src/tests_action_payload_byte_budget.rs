use super::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// RED contract: one configured byte budget must cover the public submit buffer
// and engine pending map together; the existing count/per-action limits remain
// independent safety checks.
struct ByteBudgetExecutionHook;

fn accepted_action_payload() -> Vec<u8> {
    let payload = serde_cbor::to_vec(&serde_json::json!({"aaaaaaa": "bbbbbb"}))
        .expect("encode accepted legacy runtime action payload");
    assert_eq!(
        payload.len(),
        16,
        "fixture must stay within the 16-byte limit"
    );
    payload
}

impl NodeExecutionHook for ByteBudgetExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("execution-block-{}", context.height),
            execution_state_root: format!("execution-state-{}", context.height),
        })
    }
}

#[test]
fn runtime_public_action_queue_enforces_aggregate_payload_byte_budget() {
    const QUEUE_BYTE_BUDGET: usize = 24;
    const ACTION_BYTES: usize = 16;
    let config = NodeConfig::new(
        "node-public-byte-budget",
        "world-public-byte-budget",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_pending_consensus_actions(8)
    .expect("pending action count")
    .with_max_consensus_action_payload_bytes(ACTION_BYTES)
    .expect("per-action payload limit")
    .with_max_pending_consensus_action_queue_bytes(QUEUE_BYTE_BUDGET)
    .expect("aggregate public+engine byte budget");
    let runtime = NodeRuntime::new(config);
    runtime
        .submit_consensus_action_payload(1, accepted_action_payload())
        .expect("first action should fit aggregate budget");
    let err = runtime
        .submit_consensus_action_payload(2, accepted_action_payload())
        .expect_err("projected public queue bytes must be rejected before exceeding budget");
    assert!(
        err.to_string().contains("byte budget"),
        "unexpected aggregate queue rejection: {err}"
    );
    assert_eq!(
        runtime
            .snapshot()
            .consensus
            .pending_consensus_actions
            .submit_buffer_payload_bytes,
        ACTION_BYTES,
        "rejected action must not be retained in the public queue"
    );
}

#[test]
fn pos_engine_action_queue_enforces_aggregate_payload_byte_budget() {
    const QUEUE_BYTE_BUDGET: usize = 24;
    const ACTION_BYTES: usize = 16;
    let config = NodeConfig::new(
        "node-engine-byte-budget",
        "world-engine-byte-budget",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_engine_pending_consensus_actions(8)
    .expect("engine action count")
    .with_max_consensus_action_payload_bytes(ACTION_BYTES)
    .expect("per-action payload limit")
    .with_max_pending_consensus_action_queue_bytes(QUEUE_BYTE_BUDGET)
    .expect("aggregate public+engine byte budget");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let incoming = (1..=2)
        .map(|action_id| {
            NodeConsensusAction::from_payload(
                action_id,
                config.player_id.clone(),
                vec![action_id as u8; ACTION_BYTES],
            )
            .expect("action")
        })
        .collect();
    let err = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            incoming,
            None,
        )
        .expect_err("projected engine queue bytes must be rejected before exceeding budget");
    assert!(
        err.to_string().contains("byte budget"),
        "unexpected aggregate engine queue rejection: {err}"
    );
}

#[test]
fn runtime_retained_committed_batches_enforce_aggregate_payload_byte_budget() {
    // The retained window must reject/drop before projected raw action payload
    // bytes exceed this budget, while preserving the existing count window.
    const RETAINED_BYTE_BUDGET: usize = 24;
    const ACTION_BYTES: usize = 16;
    let config = NodeConfig::new(
        "node-retained-byte-budget",
        "world-retained-byte-budget",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick interval")
    .with_max_engine_pending_consensus_actions(1)
    .expect("engine action count")
    .with_max_consensus_action_payload_bytes(ACTION_BYTES)
    .expect("per-action payload limit")
    .with_max_committed_action_batches(8)
    .expect("retained batch count")
    .with_max_committed_action_batch_bytes(RETAINED_BYTE_BUDGET)
    .expect("aggregate retained batch byte budget");
    let mut runtime = NodeRuntime::new(config).with_execution_hook(ByteBudgetExecutionHook);
    for action_id in 1..=4 {
        runtime
            .submit_consensus_action_payload(action_id, accepted_action_payload())
            .expect("submit action");
    }

    runtime.start().expect("start");
    let deadline = Instant::now() + Duration::from_secs(2);
    while runtime.snapshot().consensus.committed_height < 4 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    runtime.stop().expect("stop");

    let batches = runtime.drain_committed_action_batches();
    let retained_payload_bytes: usize = batches
        .iter()
        .flat_map(|batch| batch.actions.iter())
        .map(|action| action.payload_cbor.len())
        .sum();
    assert!(
        retained_payload_bytes <= RETAINED_BYTE_BUDGET,
        "retained committed payload bytes exceeded budget: bytes={retained_payload_bytes} budget={RETAINED_BYTE_BUDGET}"
    );
}

#[test]
fn pending_proposal_handoff_keeps_aggregate_reservation_until_payload_drop() {
    const ACTION_BYTES: usize = 16;
    const QUEUE_BYTE_BUDGET: usize = ACTION_BYTES;
    let config = NodeConfig::new(
        "node-pending-handoff-byte-budget",
        "world-pending-handoff-byte-budget",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_engine_pending_consensus_actions(8)
    .expect("engine action count")
    .with_max_consensus_action_payload_bytes(ACTION_BYTES)
    .expect("per-action payload limit")
    .with_max_pending_consensus_action_queue_bytes(QUEUE_BYTE_BUDGET)
    .expect("aggregate byte budget");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let action =
        NodeConsensusAction::from_payload(1, config.player_id.clone(), accepted_action_payload())
            .expect("action");
    merge_pending_consensus_actions_with_budget(
        &mut engine.pending_consensus_actions,
        vec![action.clone()],
        engine.max_pending_consensus_actions,
        engine.pending_consensus_action_queue_bytes.as_ref(),
        engine.max_pending_consensus_action_queue_bytes,
        false,
    )
    .expect("initial action reservation");

    let committed_actions = engine
        .drain_proposable_consensus_actions(1)
        .expect("proposal handoff");
    assert_eq!(committed_actions, vec![action.clone()]);
    engine.pending = Some(PendingProposal {
        height: 1,
        slot: 0,
        epoch: 0,
        opened_at_ms: 1,
        proposer_id: config.node_id.clone(),
        block_hash: "pending-byte-budget-block".to_string(),
        action_root: compute_consensus_action_root(&committed_actions).expect("action root"),
        committed_actions,
        attestations: BTreeMap::new(),
        approved_stake: 0,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let next =
        NodeConsensusAction::from_payload(2, config.player_id.clone(), accepted_action_payload())
            .expect("next action");
    let err = merge_pending_consensus_actions_with_budget(
        &mut engine.pending_consensus_actions,
        vec![next],
        engine.max_pending_consensus_actions,
        engine.pending_consensus_action_queue_bytes.as_ref(),
        engine.max_pending_consensus_action_queue_bytes,
        false,
    )
    .expect_err("pending proposal payload must remain charged until dropped");
    assert!(
        err.to_string().contains("byte budget"),
        "unexpected handoff rejection: {err}"
    );
    assert_eq!(
        engine
            .pending_consensus_action_queue_bytes
            .load(Ordering::Acquire),
        ACTION_BYTES,
        "proposal handoff must retain the aggregate reservation"
    );
}

#[test]
fn pending_merge_conflict_rolls_back_map_and_reservation_atomically() {
    const ACTION_BYTES: usize = 16;
    let config = NodeConfig::new(
        "node-atomic-merge-byte-budget",
        "world-atomic-merge-byte-budget",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_engine_pending_consensus_actions(8)
    .expect("engine action count")
    .with_max_consensus_action_payload_bytes(ACTION_BYTES)
    .expect("per-action payload limit");
    let mut pending = BTreeMap::new();
    let existing =
        NodeConsensusAction::from_payload(1, config.player_id.clone(), vec![0x11; ACTION_BYTES])
            .expect("existing action");
    pending.insert(existing.action_id, existing.clone());
    let queue_bytes = AtomicUsize::new(ACTION_BYTES);
    let new_action =
        NodeConsensusAction::from_payload(2, config.player_id.clone(), vec![0x22; ACTION_BYTES])
            .expect("new action");
    let conflicting =
        NodeConsensusAction::from_payload(1, config.player_id, vec![0x33; ACTION_BYTES])
            .expect("conflicting action");

    let err = merge_pending_consensus_actions_with_budget(
        &mut pending,
        vec![new_action, conflicting],
        8,
        &queue_bytes,
        ACTION_BYTES * 3,
        false,
    )
    .expect_err("conflicting merge must fail");
    assert!(
        err.to_string().contains("conflicting consensus action"),
        "unexpected conflict error: {err}"
    );
    assert_eq!(
        pending,
        BTreeMap::from([(existing.action_id, existing)]),
        "failed merge must not leave a partially inserted action"
    );
    assert_eq!(
        queue_bytes.load(Ordering::Acquire),
        ACTION_BYTES,
        "failed merge must restore the pre-merge reservation"
    );
}

#[test]
fn incoming_reserved_validation_failure_releases_reserved_bytes() {
    const ACTION_BYTES: usize = 16;
    let config = NodeConfig::new(
        "node-validation-reservation-byte-budget",
        "world-validation-reservation-byte-budget",
        NodeRole::Observer,
    )
    .expect("config");
    let mut malformed =
        NodeConsensusAction::from_payload(1, config.player_id, vec![0x44; ACTION_BYTES])
            .expect("action");
    malformed.payload_hash = "tampered-payload-hash".to_string();
    let mut pending = BTreeMap::new();
    let queue_bytes = AtomicUsize::new(ACTION_BYTES);

    let err = merge_pending_consensus_actions_with_budget(
        &mut pending,
        vec![malformed],
        8,
        &queue_bytes,
        ACTION_BYTES * 2,
        true,
    )
    .expect_err("malformed pre-reserved action must fail validation");
    assert!(
        err.to_string().contains("payload hash mismatch"),
        "unexpected validation error: {err}"
    );
    assert_eq!(
        queue_bytes.load(Ordering::Acquire),
        0,
        "validation failure must release incoming_already_reserved bytes"
    );
    assert!(
        pending.is_empty(),
        "invalid action must not enter pending map"
    );
}

#[test]
fn incoming_reserved_byte_budget_failure_releases_reserved_bytes() {
    const ACTION_BYTES: usize = 16;
    let config = NodeConfig::new(
        "node-byte-budget-reservation-failure",
        "world-byte-budget-reservation-failure",
        NodeRole::Observer,
    )
    .expect("config");
    let action = NodeConsensusAction::from_payload(1, config.player_id, vec![0x55; ACTION_BYTES])
        .expect("action");
    let mut pending = BTreeMap::new();
    let queue_bytes = AtomicUsize::new(ACTION_BYTES);

    let err = merge_pending_consensus_actions_with_budget(
        &mut pending,
        vec![action],
        8,
        &queue_bytes,
        ACTION_BYTES / 2,
        true,
    )
    .expect_err("invalid pre-reserved byte budget must fail closed");
    assert!(
        err.to_string().contains("reservation invalid"),
        "unexpected reservation error: {err}"
    );
    assert_eq!(
        queue_bytes.load(Ordering::Acquire),
        0,
        "byte-budget failure must release incoming_already_reserved bytes"
    );
    assert!(
        pending.is_empty(),
        "failed reservation must not mutate pending map"
    );
}
