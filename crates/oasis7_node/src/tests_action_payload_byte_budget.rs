use super::*;
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
