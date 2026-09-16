use super::*;
use crate::NodeExecutionBootstrap;

struct GapWaitingExecutionHook;

#[test]
fn local_execution_bootstrap_starts_consensus_at_successor_height() {
    let config = NodeConfig::new("node-local", "world-local-bootstrap", NodeRole::Sequencer)
        .expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let bootstrap = NodeExecutionBootstrap {
        height: 2,
        consensus_block_hash: "local-finality-h2".to_string(),
        execution_block_hash: "local-execution-h2".to_string(),
        execution_state_root: "local-state-root-h2".to_string(),
    };

    engine
        .apply_local_execution_bootstrap(&bootstrap)
        .expect("apply local execution boundary");

    assert_eq!(engine.committed_height, 2);
    assert_eq!(engine.next_height, 3);
    assert_eq!(engine.last_execution_height, 2);
    assert_eq!(engine.replication_persisted_height, 2);
    engine
        .validate_local_execution_bootstrap(&bootstrap)
        .expect("validate local execution boundary");
}

impl NodeExecutionHook for GapWaitingExecutionHook {
    fn on_commit(
        &mut self,
        _context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err(format!(
            "{}: last_applied=0 incoming=8 predecessor=7",
            EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE
        ))
    }
}

#[test]
fn committed_execution_waits_for_gap_sync_when_predecessor_record_is_missing() {
    let config =
        NodeConfig::new("node-b", "world-gap-sync-exec-wait", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let decision = PosDecision {
        height: 8,
        slot: 8,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-8".to_string(),
        action_root: compute_consensus_action_root(&[]).expect("empty action root"),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let mut hook = GapWaitingExecutionHook;

    engine
        .apply_committed_execution(
            &config.node_id,
            &config.world_id,
            8_000,
            &decision,
            Some(&mut hook),
        )
        .expect("defer gap execution");

    assert_eq!(engine.last_execution_height, 0);
    assert!(engine.last_execution_block_hash.is_none());
    assert!(engine.last_execution_state_root.is_none());
}

#[test]
fn sequencer_committed_execution_does_not_wait_for_gap_sync() {
    let config =
        NodeConfig::new("node-b", "world-gap-sync-exec-fail", NodeRole::Sequencer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let decision = PosDecision {
        height: 8,
        slot: 8,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-8".to_string(),
        action_root: compute_consensus_action_root(&[]).expect("empty action root"),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let mut hook = GapWaitingExecutionHook;

    let err = engine
        .apply_committed_execution(
            &config.node_id,
            &config.world_id,
            8_000,
            &decision,
            Some(&mut hook),
        )
        .expect_err("sequencer must fail when execution cannot bridge the predecessor gap");

    assert!(
        matches!(err, NodeError::Execution { reason } if reason.contains("missing predecessor record"))
    );
    assert_eq!(engine.last_execution_height, 0);
    assert!(engine.last_execution_block_hash.is_none());
    assert!(engine.last_execution_state_root.is_none());
}
