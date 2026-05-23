use super::*;

struct PanicExecutionHook {
    called: Arc<std::sync::atomic::AtomicBool>,
}

impl NodeExecutionHook for PanicExecutionHook {
    fn on_commit(
        &mut self,
        _context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        panic!("panic execution hook");
    }
}

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
fn pos_engine_non_expected_proposer_does_not_open_local_proposal() {
    let validators = multi_validators();
    let probe_config = NodeConfig::new("node-a", "world-non-proposer-probe", NodeRole::Sequencer)
        .expect("probe config")
        .with_pos_validators(validators.clone())
        .expect("probe validators");
    let probe_engine = PosNodeEngine::new(&probe_config).expect("probe engine");
    let expected = probe_engine
        .expected_proposer(0)
        .expect("expected proposer for slot 0");
    let non_proposer = validators
        .iter()
        .map(|validator| validator.validator_id.as_str())
        .find(|validator_id| *validator_id != expected)
        .expect("non proposer");

    let config = NodeConfig::new(
        non_proposer,
        "world-non-proposer-probe",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_pos_validators(validators)
    .expect("validators")
    .with_auto_attest_all_validators(true);
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

    assert_eq!(snapshot.consensus_snapshot.latest_height, 0);
    assert_eq!(snapshot.consensus_snapshot.committed_height, 0);
    assert_eq!(engine.committed_height, 0);
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_non_expected_proposer_keeps_pending_consensus_actions_queued() {
    let validators = multi_validators();
    let probe_config = NodeConfig::new("node-a", "world-non-proposer-queue", NodeRole::Sequencer)
        .expect("probe config")
        .with_pos_validators(validators.clone())
        .expect("probe validators");
    let probe_engine = PosNodeEngine::new(&probe_config).expect("probe engine");
    let expected = probe_engine
        .expected_proposer(0)
        .expect("expected proposer for slot 0");
    let non_proposer = validators
        .iter()
        .map(|validator| validator.validator_id.as_str())
        .find(|validator_id| *validator_id != expected)
        .expect("non proposer");

    let config = NodeConfig::new(
        non_proposer,
        "world-non-proposer-queue",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_pos_validators(validators)
    .expect("validators")
    .with_auto_attest_all_validators(true);
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let queued = NodeConsensusAction::from_payload(7, config.player_id.clone(), vec![7_u8])
        .expect("queued action");
    engine.pending_consensus_actions.insert(7, queued);

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

    assert_eq!(snapshot.consensus_snapshot.latest_height, 0);
    assert_eq!(snapshot.consensus_snapshot.committed_height, 0);
    assert_eq!(engine.pending_consensus_actions.len(), 1);
    assert!(engine.pending_consensus_actions.contains_key(&7));
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
        opened_at_ms: 100,
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
        opened_at_ms: 200,
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
        .restore_state_snapshot(snapshot, None)
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
fn sequencer_commit_binding_rejects_missing_execution_hashes() {
    let config = NodeConfig::new("node-b", "world-missing-exec-binding", NodeRole::Sequencer)
        .expect("config");
    let engine = PosNodeEngine::new(&config).expect("engine");

    let err = engine
        .commit_execution_binding_for_height(1)
        .expect_err("sequencer commit binding must require execution hashes");

    assert!(
        matches!(err, NodeError::Consensus { reason } if reason.contains("missing execution binding"))
    );
}

#[test]
fn sequencer_restore_state_snapshot_rejects_committed_head_ahead_of_execution() {
    let config =
        NodeConfig::new("node-a", "world-restore-gap", NodeRole::Sequencer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 9;
    engine.network_committed_height = 10;
    engine.next_height = 11;
    engine.next_slot = 3;

    let snapshot = super::pos_state_store::PosNodeStateSnapshot {
        next_height: 6,
        next_slot: 5,
        last_observed_slot: 5,
        missed_slot_count: 0,
        last_observed_tick: 5,
        missed_tick_count: 0,
        committed_height: 5,
        network_committed_height: 5,
        last_broadcast_proposal_height: 0,
        last_broadcast_local_attestation_height: 0,
        last_broadcast_committed_height: 0,
        last_committed_block_hash: Some("committed-5".to_string()),
        last_execution_height: 3,
        last_execution_block_hash: Some("exec-3".to_string()),
        last_execution_state_root: Some("state-3".to_string()),
    };

    let err = engine
        .restore_state_snapshot(snapshot, None)
        .expect_err("sequencer snapshot must fail when committed head is ahead of execution");

    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("committed_height=5") && reason.contains("last_execution_height=3"))
    );
    assert_eq!(engine.committed_height, 9);
    assert_eq!(engine.network_committed_height, 10);
    assert_eq!(engine.next_height, 11);
    assert_eq!(engine.next_slot, 3);
}

#[test]
fn restore_state_snapshot_clamps_future_clock_state_when_fixed_genesis_is_configured() {
    let mut config = NodeConfig::new("node-a", "world-restore-clock-clamp", NodeRole::Sequencer)
        .expect("config");
    config.pos_config.slot_duration_ms = 100;
    config.pos_config.ticks_per_slot = 10;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let snapshot = super::pos_state_store::PosNodeStateSnapshot {
        next_height: 4,
        next_slot: 77,
        last_observed_slot: 77,
        missed_slot_count: 0,
        last_observed_tick: 777,
        missed_tick_count: 0,
        committed_height: 3,
        network_committed_height: 3,
        last_broadcast_proposal_height: 3,
        last_broadcast_local_attestation_height: 3,
        last_broadcast_committed_height: 3,
        last_committed_block_hash: Some("committed-3".to_string()),
        last_execution_height: 3,
        last_execution_block_hash: Some("exec-3".to_string()),
        last_execution_state_root: Some("state-3".to_string()),
    };

    engine
        .restore_state_snapshot(snapshot, Some(1_230))
        .expect("fixed genesis restore should clamp future clock state");

    assert_eq!(engine.next_height, 4);
    assert_eq!(engine.committed_height, 3);
    assert_eq!(engine.network_committed_height, 3);
    assert_eq!(engine.next_slot, 2);
    assert_eq!(engine.last_observed_slot, 2);
    assert_eq!(engine.last_observed_tick, 23);
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

#[test]
fn runtime_stop_cleans_up_after_worker_join_failure() {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("bind socket");
    let addr = socket.local_addr().expect("addr");
    drop(socket);

    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let config = NodeConfig::new("node-a", "world-join-failure", NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval")
        .with_gossip_optional(addr, Vec::new());
    let mut runtime = NodeRuntime::new(config).with_execution_hook(PanicExecutionHook {
        called: Arc::clone(&called),
    });
    runtime.start().expect("start");

    let hook_panicked = wait_until(Instant::now() + Duration::from_secs(1), || {
        called.load(std::sync::atomic::Ordering::SeqCst)
    });
    assert!(hook_panicked, "execution hook did not run before stop");

    let err = runtime
        .stop()
        .expect_err("stop should surface thread join failure");
    assert!(matches!(err, NodeError::ThreadJoinFailed { .. }));
    assert!(!runtime.snapshot().running);

    let config_retry = NodeConfig::new("node-b", "world-join-failure", NodeRole::Observer)
        .expect("retry config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("retry tick interval")
        .with_gossip_optional(addr, Vec::new());
    let mut retry_runtime = NodeRuntime::new(config_retry);
    retry_runtime
        .start()
        .expect("start retry runtime on released socket");
    retry_runtime.stop().expect("stop retry runtime");
}
