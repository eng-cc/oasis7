use super::*;

#[test]
fn pos_engine_record_synced_replication_height_rejects_overflow_without_partial_state() {
    let config =
        NodeConfig::new("node-a", "world-overflow-sync", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 9;
    engine.next_height = 10;
    engine.pending = Some(PendingProposal {
        height: 10,
        slot: 1,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "pending-sync".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let err = engine
        .record_synced_replication_height(u64::MAX, "overflow-block".to_string(), 7_700)
        .expect_err("height overflow must fail");
    assert!(matches!(err, NodeError::Replication { reason } if reason.contains("height overflow")));
    assert_eq!(engine.committed_height, 9);
    assert_eq!(engine.next_height, 10);
    assert_eq!(
        engine
            .pending
            .as_ref()
            .map(|proposal| proposal.block_hash.as_str()),
        Some("pending-sync")
    );
    assert!(engine.last_committed_block_hash.is_none());
    assert!(engine.last_committed_at_ms.is_none());
}

#[test]
fn pos_engine_record_synced_replication_height_resets_stale_next_height() {
    let config =
        NodeConfig::new("node-a", "world-sync-next-height", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 1;
    engine.next_height = 9;
    engine.pending = Some(PendingProposal {
        height: 9,
        slot: 3,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "stale-pending".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    engine
        .record_synced_replication_height(2, "block-2".to_string(), 7_800)
        .expect("sync height");

    assert_eq!(engine.committed_height, 2);
    assert_eq!(engine.next_height, 3);
    assert_eq!(engine.last_committed_at_ms, Some(7_800));
    assert_eq!(engine.last_committed_block_hash.as_deref(), Some("block-2"));
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_snapshot_uses_last_committed_hash_for_pending_decision() {
    let config =
        NodeConfig::new("node-a", "world-pending-snapshot", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 2;
    engine.next_height = 3;
    engine.last_committed_block_hash = Some("committed-h2".to_string());

    let snapshot = engine.snapshot_from_decision(&PosDecision {
        height: 3,
        slot: 3,
        epoch: 0,
        status: PosConsensusStatus::Pending,
        block_hash: "proposal-h3".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 0,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    });

    assert_eq!(snapshot.committed_height, 2);
    assert_eq!(snapshot.latest_height, 3);
    assert_eq!(snapshot.last_status, Some(PosConsensusStatus::Pending));
    assert_eq!(snapshot.last_block_hash.as_deref(), Some("committed-h2"));
}
