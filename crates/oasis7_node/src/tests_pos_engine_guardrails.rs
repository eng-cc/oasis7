use super::*;

#[test]
fn pos_engine_commits_single_validator_head() {
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let snapshot = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("tick");
    assert_eq!(snapshot.consensus_snapshot.mode, NodeConsensusMode::Pos);
    assert_eq!(snapshot.consensus_snapshot.latest_height, 1);
    assert_eq!(snapshot.consensus_snapshot.committed_height, 1);
    assert_eq!(
        snapshot.consensus_snapshot.last_status,
        Some(PosConsensusStatus::Committed)
    );
    assert_eq!(snapshot.consensus_snapshot.slot, 1);
}

#[test]
fn pos_engine_generates_chain_hashed_block_ids() {
    let config = NodeConfig::new("node-a", "world-hash", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let first = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("first tick");
    let second = engine
        .tick(
            &config.node_id,
            &config.world_id,
            2_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("second tick");

    let first_hash = first
        .consensus_snapshot
        .last_block_hash
        .as_deref()
        .expect("first hash should exist");
    let second_hash = second
        .consensus_snapshot
        .last_block_hash
        .as_deref()
        .expect("second hash should exist");
    assert_eq!(first_hash.len(), 64);
    assert_eq!(second_hash.len(), 64);
    assert!(first_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert!(second_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_ne!(first_hash, second_hash);
    assert!(!first_hash.contains(":h"));
}

#[test]
fn pos_engine_stays_pending_without_peer_votes_when_auto_attest_disabled() {
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(multi_validators())
        .expect("validators")
        .with_auto_attest_all_validators(false);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    for offset in 0..12 {
        let snapshot = engine
            .tick(
                &config.node_id,
                &config.world_id,
                2_000 + offset,
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
            )
            .expect("tick");
        assert_eq!(snapshot.consensus_snapshot.committed_height, 0);
    }
}

#[test]
fn pos_engine_apply_decision_rejects_height_overflow_without_state_mutation() {
    let config =
        NodeConfig::new("node-a", "world-overflow-apply", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 41;
    engine.network_committed_height = 43;
    engine.next_height = 44;
    engine.pending = Some(PendingProposal {
        height: 44,
        slot: 7,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "pending-block".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let decision = PosDecision {
        height: u64::MAX,
        slot: 8,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "overflow-block".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };

    let err = engine
        .apply_decision(&decision)
        .expect_err("height overflow must fail");
    assert!(
        matches!(err, NodeError::Consensus { reason } if reason.contains("decision.height overflow"))
    );
    assert_eq!(engine.committed_height, 41);
    assert_eq!(engine.network_committed_height, 43);
    assert_eq!(engine.next_height, 44);
    assert_eq!(
        engine
            .pending
            .as_ref()
            .map(|proposal| proposal.block_hash.as_str()),
        Some("pending-block")
    );
    assert!(engine.last_committed_block_hash.is_none());
}

#[test]
fn pos_engine_ingest_proposal_rejects_slot_overflow_without_partial_state() {
    let config =
        NodeConfig::new("node-a", "world-overflow-proposal", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.next_height = 5;
    engine.next_slot = 3;

    let message = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: config.node_id.clone(),
        player_id: config.player_id.clone(),
        proposer_id: config.node_id.clone(),
        height: 8,
        slot: u64::MAX,
        epoch: 0,
        block_hash: "proposal-overflow".to_string(),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_234,
        public_key_hex: None,
        signature_hex: None,
    };

    let err = engine
        .ingest_proposal_message(config.world_id.as_str(), &message, u64::MAX)
        .expect_err("slot overflow must fail");
    assert!(
        matches!(err, NodeError::Consensus { reason } if reason.contains("proposal.slot overflow"))
    );
    assert_eq!(engine.next_height, 5);
    assert_eq!(engine.next_slot, 3);
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_restore_state_snapshot_rejects_overflow_without_partial_state() {
    let config =
        NodeConfig::new("node-a", "world-overflow-restore", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 9;
    engine.network_committed_height = 10;
    engine.next_height = 11;
    engine.next_slot = 3;
    engine.pending = Some(PendingProposal {
        height: 11,
        slot: 3,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "pending-restore".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let snapshot = super::pos_state_store::PosNodeStateSnapshot {
        next_height: 0,
        next_slot: 77,
        last_observed_slot: 77,
        missed_slot_count: 0,
        last_observed_tick: 77,
        missed_tick_count: 0,
        committed_height: u64::MAX,
        network_committed_height: u64::MAX,
        last_broadcast_proposal_height: 0,
        last_broadcast_local_attestation_height: 0,
        last_broadcast_committed_height: 0,
        last_committed_block_hash: Some("unexpected".to_string()),
        last_execution_height: 0,
        last_execution_block_hash: None,
        last_execution_state_root: None,
    };

    let err = engine
        .restore_state_snapshot(snapshot)
        .expect_err("committed height overflow must fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("committed_height"))
    );
    assert_eq!(engine.committed_height, 9);
    assert_eq!(engine.network_committed_height, 10);
    assert_eq!(engine.next_height, 11);
    assert_eq!(engine.next_slot, 3);
    assert_eq!(
        engine
            .pending
            .as_ref()
            .map(|proposal| proposal.block_hash.as_str()),
        Some("pending-restore")
    );
}

#[test]
fn sequencer_commit_requires_execution_hook() {
    let config = NodeConfig::new("sequencer-a", "world-a", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let mut runtime = NodeRuntime::new(config);
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(80));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.consensus.committed_height, 0);
    assert!(snapshot
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("execution hook is required"));
}

#[test]
fn runtime_start_and_stop_updates_snapshot() {
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let mut runtime = NodeRuntime::new(config);
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(40));

    let running = runtime.snapshot();
    assert!(running.running);
    assert!(running.tick_count >= 2);
    assert!(running.last_tick_unix_ms.is_some());
    assert_eq!(running.consensus.mode, NodeConsensusMode::Pos);
    assert!(running.consensus.committed_height >= 1);
    assert_eq!(
        running.consensus.last_status,
        Some(PosConsensusStatus::Committed)
    );
    assert!(running.last_error.is_none());

    runtime.stop().expect("stop");
    let stopped = runtime.snapshot();
    assert!(!stopped.running);
    assert!(stopped.tick_count >= running.tick_count);
}
