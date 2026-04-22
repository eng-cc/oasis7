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

#[test]
fn pos_engine_snapshot_surfaces_pending_and_queue_metrics() {
    let config =
        NodeConfig::new("node-a", "world-pending-metrics", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 2;
    engine.next_height = 4;
    engine.last_committed_at_ms = Some(8_100);
    engine.inbound_rejected_proposal_future_slot = 2;
    engine.inbound_rejected_proposal_stale_slot = 1;
    engine.inbound_rejected_attestation_future_slot = 3;
    engine.inbound_rejected_attestation_stale_slot = 4;
    engine.inbound_rejected_attestation_epoch_mismatch = 5;
    engine.last_inbound_timing_reject_reason =
        Some("attestation target_epoch mismatch".to_string());
    engine.pending_consensus_actions.insert(
        10,
        NodeConsensusAction::from_payload(10, "player-a", vec![0x0a]).expect("queued-a action"),
    );
    engine.pending_consensus_actions.insert(
        11,
        NodeConsensusAction::from_payload(11, "player-a", vec![0x0b]).expect("queued-b action"),
    );
    engine.pending = Some(PendingProposal {
        height: 3,
        slot: 8,
        epoch: 1,
        proposer_id: "node-b".to_string(),
        block_hash: "pending-h3".to_string(),
        action_root: empty_action_root(),
        committed_actions: vec![
            NodeConsensusAction::from_payload(12, "player-a", vec![0x0c])
                .expect("reserved-a action"),
            NodeConsensusAction::from_payload(13, "player-a", vec![0x0d])
                .expect("reserved-b action"),
        ],
        attestations: std::collections::BTreeMap::from([(
            "node-a".to_string(),
            oasis7_consensus::node_pos::NodePosAttestation {
                validator_id: "node-a".to_string(),
                approve: true,
                source_epoch: 0,
                target_epoch: 1,
                voted_at_ms: 8_150,
                reason: Some("auto attestation".to_string()),
            },
        )]),
        approved_stake: 34,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let snapshot = engine.snapshot_from_decision(&PosDecision {
        height: 3,
        slot: 8,
        epoch: 1,
        status: PosConsensusStatus::Pending,
        block_hash: "proposal-h3".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 34,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    });

    assert_eq!(snapshot.last_committed_at_ms, Some(8_100));
    assert_eq!(snapshot.pending_consensus_actions.queued_action_count, 2);
    assert_eq!(
        snapshot
            .pending_consensus_actions
            .reserved_requeue_action_count,
        2
    );
    assert_eq!(
        snapshot.pending_consensus_actions.available_capacity,
        engine.pending_consensus_action_capacity()
    );
    let pending = snapshot
        .pending_proposal
        .as_ref()
        .expect("pending proposal");
    assert_eq!(pending.height, 3);
    assert_eq!(pending.proposer_id, "node-b");
    assert_eq!(pending.action_count, 2);
    assert_eq!(pending.attestation_count, 1);
    assert_eq!(pending.approved_stake, 34);
    assert_eq!(snapshot.inbound_rejected_proposal_future_slot, 2);
    assert_eq!(snapshot.inbound_rejected_proposal_stale_slot, 1);
    assert_eq!(snapshot.inbound_rejected_attestation_future_slot, 3);
    assert_eq!(snapshot.inbound_rejected_attestation_stale_slot, 4);
    assert_eq!(snapshot.inbound_rejected_attestation_epoch_mismatch, 5);
    assert_eq!(
        snapshot.last_inbound_timing_reject_reason.as_deref(),
        Some("attestation target_epoch mismatch")
    );
}
