#[path = "tests_network_gap_sync_budget.rs"]
mod network_gap_sync_budget_tests;
#[path = "tests_network_gap_sync_execution_failures.rs"]
mod network_gap_sync_execution_failure_tests;
#[path = "tests_network_gap_sync_high_checkpoint_probe.rs"]
mod network_gap_sync_high_checkpoint_probe_tests;
#[path = "tests_network_gap_sync_not_found.rs"]
mod network_gap_sync_not_found_tests;
#[path = "tests_network_gap_sync_peer_head.rs"]
mod network_gap_sync_peer_head_tests;
#[path = "tests_network_gap_sync_provider_routing.rs"]
mod network_gap_sync_provider_routing_tests;
#[path = "tests_network_gap_sync_successor_probe.rs"]
mod network_gap_sync_successor_probe_tests;
#[path = "tests_network_gap_sync.rs"]
mod network_gap_sync_tests;
#[path = "tests_storage_challenge_blob_cache.rs"]
mod storage_challenge_blob_cache_tests;

#[test]
fn pos_engine_rejects_commit_without_execution_hashes_when_required() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let config = NodeConfig::new("node-b", "world-commit-exec-required", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 60,
            },
            PosValidator {
                validator_id: "node-b".to_string(),
                stake: 40,
            },
        ])
        .expect("validators")
        .with_require_peer_execution_hashes(true)
        .with_gossip_optional(addr_b, vec![addr_a]);
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    endpoint_a
        .broadcast_commit(&GossipCommitMessage {
            version: 1,
            world_id: config.world_id.clone(),
            node_id: "node-a".to_string(),
            player_id: "node-a".to_string(),
            height: 4,
            slot: 4,
            epoch: 0,
            block_hash: "block-4".to_string(),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: 4_000,
            execution_block_hash: None,
            execution_state_root: None,
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("broadcast commit");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config.node_id, &config.world_id, None, 0)
        .expect("ingest");
    assert!(
        !engine.peer_heads.contains_key("node-a"),
        "peer head with missing execution hashes must be rejected"
    );
}

#[test]
fn pos_engine_rejects_commit_when_execution_binding_mismatches_local() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let config = NodeConfig::new("node-b", "world-commit-exec-mismatch", NodeRole::Observer)
        .expect("config")
        .with_require_peer_execution_hashes(true)
        .with_gossip_optional(addr_b, vec![addr_a]);
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut hook = RecordingExecutionHook::new(calls);
    let tick = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            Some(&mut hook),
        )
        .expect("tick");
    assert_eq!(tick.consensus_snapshot.committed_height, 1);
    assert_eq!(engine.last_execution_height, 1);

    endpoint_a
        .broadcast_commit(&GossipCommitMessage {
            version: 1,
            world_id: config.world_id.clone(),
            node_id: "node-a".to_string(),
            player_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            block_hash: "block-peer-1".to_string(),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: 1_100,
            execution_block_hash: Some("exec-block-mismatch".to_string()),
            execution_state_root: Some("exec-state-mismatch".to_string()),
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("broadcast commit");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config.node_id, &config.world_id, None, 0)
        .expect("ingest");
    assert!(
        !engine.peer_heads.contains_key("node-a"),
        "peer head with mismatched execution binding must be rejected"
    );
}

#[test]
fn pos_engine_waits_when_next_slot_is_in_future() {
    let mut config =
        NodeConfig::new("node-a", "world-slot-wait", NodeRole::Observer).expect("config");
    config.pos_config.slot_duration_ms = 100;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
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
    assert_eq!(first.consensus_snapshot.committed_height, 1);

    let second = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_050,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("second tick");
    assert_eq!(second.consensus_snapshot.committed_height, 1);
    assert_eq!(
        second.consensus_snapshot.last_status,
        Some(PosConsensusStatus::Pending)
    );
    assert_eq!(second.consensus_snapshot.last_observed_slot, 0);
    assert_eq!(engine.next_height, 2);
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_aligns_missed_slots_to_wall_clock() {
    let mut config =
        NodeConfig::new("node-a", "world-slot-align", NodeRole::Observer).expect("config");
    config.pos_config.slot_duration_ms = 10;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    engine
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
            1_100,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("second tick");

    assert_eq!(engine.missed_slot_count, 9);
    assert_eq!(engine.last_observed_slot, 10);
    assert_eq!(second.consensus_snapshot.last_observed_slot, 10);
    assert_eq!(second.consensus_snapshot.missed_slot_count, 9);
    assert_eq!(second.consensus_snapshot.slot, 11);
}

#[test]
fn pos_engine_observed_slot_does_not_backtrack_on_clock_rewind() {
    let mut config =
        NodeConfig::new("node-a", "world-slot-monotonic", NodeRole::Observer).expect("config");
    config.pos_config.slot_duration_ms = 10;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    engine
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
    engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_200,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("second tick");
    let third = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_150,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("third tick");

    assert_eq!(engine.last_observed_slot, 20);
    assert_eq!(third.consensus_snapshot.last_observed_slot, 20);
    assert_eq!(third.consensus_snapshot.committed_height, 2);
}

#[test]
fn pos_engine_proposes_only_on_configured_tick_phase() {
    let mut config =
        NodeConfig::new("node-a", "world-phase-gate", NodeRole::Observer).expect("config");
    config.pos_config.slot_duration_ms = 100;
    config.pos_config.ticks_per_slot = 10;
    config.pos_config.proposal_tick_phase = 9;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let phase_zero = engine
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
        .expect("phase zero tick");
    assert_eq!(phase_zero.consensus_snapshot.committed_height, 0);
    assert_eq!(phase_zero.consensus_snapshot.tick_phase, 0);

    let phase_eight = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_080,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("phase eight tick");
    assert_eq!(phase_eight.consensus_snapshot.committed_height, 0);
    assert_eq!(phase_eight.consensus_snapshot.tick_phase, 8);

    let phase_nine = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_090,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("phase nine tick");
    assert_eq!(phase_nine.consensus_snapshot.committed_height, 1);
    assert_eq!(phase_nine.consensus_snapshot.tick_phase, 9);
}

fn phase_recovery_config(node_id: &str, world_id: &str) -> NodeConfig {
    let mut config = NodeConfig::new(node_id, world_id, NodeRole::Observer).expect("config");
    config.pos_config.slot_duration_ms = 100;
    config.pos_config.ticks_per_slot = 10;
    config.pos_config.proposal_tick_phase = 9;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    config
}

fn tick_phase_recovery(
    engine: &mut PosNodeEngine,
    config: &NodeConfig,
    now_ms: i64,
) -> NodeEngineTickResult {
    engine
        .tick(
            &config.node_id,
            &config.world_id,
            now_ms,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("phase recovery tick")
}

#[test]
fn pos_engine_recovers_proposal_after_skipping_configured_phase() {
    let config = phase_recovery_config("node-a", "world-phase-recovery");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let recovered = tick_phase_recovery(&mut engine, &config, 2_000);

    assert_eq!(recovered.consensus_snapshot.tick_phase, 0);
    assert_eq!(recovered.consensus_snapshot.missed_slot_count, 10);
    assert_eq!(recovered.consensus_snapshot.committed_height, 1);
}

#[test]
fn pos_engine_skipped_phase_recovery_is_non_sticky() {
    let config = phase_recovery_config("node-a", "world-phase-recovery-once");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let recovered = tick_phase_recovery(&mut engine, &config, 2_000);
    assert_eq!(recovered.consensus_snapshot.committed_height, 1);

    let next_slot_phase_zero = tick_phase_recovery(&mut engine, &config, 2_100);
    assert_eq!(next_slot_phase_zero.consensus_snapshot.tick_phase, 0);
    assert_eq!(next_slot_phase_zero.consensus_snapshot.committed_height, 1);

    let next_slot_configured_phase = tick_phase_recovery(&mut engine, &config, 2_190);
    assert_eq!(next_slot_configured_phase.consensus_snapshot.tick_phase, 9);
    assert_eq!(
        next_slot_configured_phase
            .consensus_snapshot
            .committed_height,
        2
    );
}

#[test]
fn pos_engine_pending_proposal_guard_consumes_skipped_phase_recovery() {
    let config = phase_recovery_config("node-a", "world-phase-recovery-pending");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.pending = Some(PendingProposal {
        height: 2,
        slot: 10,
        epoch: 0,
        opened_at_ms: 1_900,
        proposer_id: "node-b".to_string(),
        block_hash: "pending-future-block".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: BTreeMap::new(),
        approved_stake: 0,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let guarded = tick_phase_recovery(&mut engine, &config, 2_000);
    assert_eq!(guarded.consensus_snapshot.missed_slot_count, 10);
    assert_eq!(guarded.consensus_snapshot.committed_height, 0);
    assert!(engine.pending.is_some(), "pending guard must be exercised");

    engine.pending = None;
    let off_phase_retry = tick_phase_recovery(&mut engine, &config, 2_010);
    assert_eq!(off_phase_retry.consensus_snapshot.tick_phase, 1);
    assert_eq!(off_phase_retry.consensus_snapshot.committed_height, 0);

    let configured_phase = tick_phase_recovery(&mut engine, &config, 2_090);
    assert_eq!(configured_phase.consensus_snapshot.committed_height, 1);
}

#[derive(Clone)]
struct RecoveryEdgeSuccessorProbeNetwork {
    request_count: Arc<Mutex<usize>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for RecoveryEdgeSuccessorProbeNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        let mut request_count = self.request_count.lock().expect("lock request count");
        *request_count += 1;
        if *request_count == 1 {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
            });
        }
        serde_json::to_vec(&super::replication::FetchCommitResponse {
            found: false,
            message: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch commit response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

#[test]
fn pos_engine_successor_probe_guard_consumes_skipped_phase_recovery() {
    let dir_remote = temp_dir("phase-recovery-probe-remote");
    let dir_local = temp_dir("phase-recovery-probe-local");
    let world_id = "world-phase-recovery-probe";
    let request_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(RecoveryEdgeSuccessorProbeNetwork {
        request_count: Arc::clone(&request_count),
    });
    let (mut engine, mut replication, mut endpoint, _) =
        network_gap_sync_tests::build_fetch_commit_success_cache_fixture(
            world_id,
            dir_remote.as_path(),
            dir_local.as_path(),
            138,
            139,
            network,
        );
    engine.slot_duration_ms = 100;
    engine.ticks_per_slot = 10;
    engine.proposal_tick_phase = 9;
    engine.slot_clock_genesis_unix_ms = Some(1_000);
    engine.committed_height = 1;
    engine.network_committed_height = 0;
    engine.next_height = 2;
    engine.replication_enabled = false;

    let guarded = engine
        .tick(
            "node-a",
            world_id,
            2_000,
            None,
            Some(&mut replication),
            Some(&mut endpoint),
            None,
            Vec::new(),
            None,
        )
        .expect("successor probe guarded recovery tick");
    assert_eq!(guarded.consensus_snapshot.missed_slot_count, 10);
    assert_eq!(guarded.consensus_snapshot.committed_height, 1);
    assert_eq!(engine.last_replication_successor_probe_hold, Some(true));

    engine.last_replication_successor_probe_at_ms = None;
    let off_phase_retry = engine
        .tick(
            "node-a",
            world_id,
            2_010,
            None,
            Some(&mut replication),
            Some(&mut endpoint),
            None,
            Vec::new(),
            None,
        )
        .expect("released successor probe off-phase tick");
    assert_eq!(off_phase_retry.consensus_snapshot.tick_phase, 1);
    assert_eq!(off_phase_retry.consensus_snapshot.committed_height, 1);
    assert_eq!(engine.last_replication_successor_probe_hold, Some(false));
    assert_eq!(*request_count.lock().expect("lock request count"), 2);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn pos_engine_disabled_proposals_consume_skipped_phase_recovery() {
    let config = phase_recovery_config("node-a", "world-phase-recovery-disabled")
        .with_allow_local_proposals(false);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let guarded = tick_phase_recovery(&mut engine, &config, 2_000);
    assert_eq!(guarded.consensus_snapshot.missed_slot_count, 10);
    assert_eq!(guarded.consensus_snapshot.committed_height, 0);

    engine.allow_local_proposals = true;
    let off_phase_retry = tick_phase_recovery(&mut engine, &config, 2_010);
    assert_eq!(off_phase_retry.consensus_snapshot.tick_phase, 1);
    assert_eq!(off_phase_retry.consensus_snapshot.committed_height, 0);

    let configured_phase = tick_phase_recovery(&mut engine, &config, 2_090);
    assert_eq!(configured_phase.consensus_snapshot.committed_height, 1);
}

#[test]
fn pos_engine_participation_guard_consumes_skipped_phase_recovery() {
    let config = phase_recovery_config("node-a", "world-phase-recovery-participation");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.network_committed_height = 1;

    let guarded = tick_phase_recovery(&mut engine, &config, 2_000);
    assert_eq!(guarded.consensus_snapshot.missed_slot_count, 10);
    assert_eq!(guarded.consensus_snapshot.committed_height, 0);

    engine.network_committed_height = 0;
    let off_phase_retry = tick_phase_recovery(&mut engine, &config, 2_010);
    assert_eq!(off_phase_retry.consensus_snapshot.tick_phase, 1);
    assert_eq!(off_phase_retry.consensus_snapshot.committed_height, 0);

    let configured_phase = tick_phase_recovery(&mut engine, &config, 2_090);
    assert_eq!(configured_phase.consensus_snapshot.committed_height, 1);
}

#[test]
fn pos_engine_skipped_phase_recovery_keeps_expected_proposer_gate() {
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 50,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 50,
        },
    ];
    let probe_config = NodeConfig::new("node-a", "world-phase-proposer", NodeRole::Observer)
        .expect("probe config")
        .with_pos_validators(validators.clone())
        .expect("probe validators");
    let expected = PosNodeEngine::new(&probe_config)
        .expect("probe engine")
        .expected_proposer(10)
        .expect("slot 10 proposer");
    let non_proposer = validators
        .iter()
        .map(|validator| validator.validator_id.as_str())
        .find(|validator_id| *validator_id != expected.as_str())
        .expect("non-proposer validator");

    let mut config = NodeConfig::new(non_proposer, "world-phase-proposer", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(validators)
        .expect("validators")
        .with_auto_attest_all_validators(true);
    config.pos_config.slot_duration_ms = 100;
    config.pos_config.ticks_per_slot = 10;
    config.pos_config.proposal_tick_phase = 9;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let recovered = tick_phase_recovery(&mut engine, &config, 2_000);
    assert_eq!(recovered.consensus_snapshot.missed_slot_count, 10);
    assert_eq!(recovered.consensus_snapshot.committed_height, 0);
    assert!(engine.pending.is_none());
    assert_ne!(
        engine.expected_proposer(10).as_deref(),
        Some(config.node_id.as_str())
    );

    let configured_phase = tick_phase_recovery(&mut engine, &config, 2_090);
    assert_eq!(configured_phase.consensus_snapshot.committed_height, 0);
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_tracks_missed_logical_ticks() {
    let mut config =
        NodeConfig::new("node-a", "world-missed-tick", NodeRole::Observer).expect("config");
    config.pos_config.slot_duration_ms = 100;
    config.pos_config.ticks_per_slot = 10;
    config.pos_config.proposal_tick_phase = 9;
    config.pos_config.slot_clock_genesis_unix_ms = Some(1_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    engine
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
    let jumped = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_120,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("jumped tick");

    assert_eq!(engine.last_observed_tick, 12);
    assert_eq!(engine.missed_tick_count, 11);
    assert_eq!(jumped.consensus_snapshot.last_observed_tick, 12);
    assert_eq!(jumped.consensus_snapshot.missed_tick_count, 11);
    assert_eq!(jumped.consensus_snapshot.tick_phase, 2);
}

#[test]
fn replication_commit_payload_includes_execution_hashes() {
    let dir = temp_dir("replication-payload-exec");
    let config = NodeReplicationConfig::new(dir.clone()).expect("replication config");
    let mut replication =
        super::replication::ReplicationRuntime::new(&config, "node-a").expect("runtime");
    let decision = PosDecision {
        height: 1,
        slot: 0,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let message = replication
        .build_local_commit_message(
            "node-a",
            "world-repl-exec",
            5_000,
            &decision,
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build")
        .expect("message");
    let payload: serde_json::Value =
        serde_json::from_slice(&message.payload).expect("parse payload");
    assert_eq!(
        payload
            .get("execution_block_hash")
            .and_then(serde_json::Value::as_str),
        Some("exec-block-1")
    );
    assert_eq!(
        payload
            .get("execution_state_root")
            .and_then(serde_json::Value::as_str),
        Some("exec-state-1")
    );
    assert_eq!(
        payload
            .get("action_root")
            .and_then(serde_json::Value::as_str),
        Some(empty_action_root().as_str())
    );
    assert_eq!(
        payload
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(0)
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_rejects_double_start() {
    let config = NodeConfig::new("node-b", "world-b", NodeRole::Sequencer).expect("config");
    let mut runtime = NodeRuntime::new(config);
    runtime.start().expect("first start");
    let err = runtime.start().expect_err("second start must fail");
    assert!(matches!(err, NodeError::AlreadyRunning { .. }));
    runtime.stop().expect("stop");
}

#[test]
fn runtime_adaptive_tick_scheduler_reduces_tick_frequency() {
    let genesis_unix_ms = super::runtime_util::now_unix_ms();

    let mut adaptive_config =
        NodeConfig::new("node-adaptive", "world-adaptive", NodeRole::Observer).expect("config");
    adaptive_config.tick_interval = Duration::from_millis(1);
    adaptive_config.pos_config.slot_duration_ms = 200;
    adaptive_config.pos_config.ticks_per_slot = 10;
    adaptive_config.pos_config.proposal_tick_phase = 9;
    adaptive_config.pos_config.slot_clock_genesis_unix_ms = Some(genesis_unix_ms);
    adaptive_config.pos_config.adaptive_tick_scheduler_enabled = true;

    let mut fixed_config =
        NodeConfig::new("node-fixed", "world-fixed", NodeRole::Observer).expect("config");
    fixed_config.tick_interval = Duration::from_millis(1);
    fixed_config.pos_config.slot_duration_ms = 200;
    fixed_config.pos_config.ticks_per_slot = 10;
    fixed_config.pos_config.proposal_tick_phase = 9;
    fixed_config.pos_config.slot_clock_genesis_unix_ms = Some(genesis_unix_ms);
    fixed_config.pos_config.adaptive_tick_scheduler_enabled = false;

    let mut adaptive_runtime = NodeRuntime::new(adaptive_config);
    let mut fixed_runtime = NodeRuntime::new(fixed_config);
    adaptive_runtime.start().expect("start adaptive");
    fixed_runtime.start().expect("start fixed");
    thread::sleep(Duration::from_millis(140));

    let adaptive_snapshot = adaptive_runtime.snapshot();
    let fixed_snapshot = fixed_runtime.snapshot();

    adaptive_runtime.stop().expect("stop adaptive");
    fixed_runtime.stop().expect("stop fixed");

    assert!(
        fixed_snapshot.tick_count > adaptive_snapshot.tick_count + 20,
        "adaptive scheduler should significantly reduce tick frequency: adaptive={} fixed={}",
        adaptive_snapshot.tick_count,
        fixed_snapshot.tick_count
    );
}

#[test]
fn runtime_pos_state_persists_across_restart() {
    let dir = temp_dir("pos-state-restart");
    let build_config = || {
        NodeConfig::new("node-a", "world-pos-state", NodeRole::Sequencer)
            .expect("config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("tick")
            .with_replication_root(dir.clone())
            .expect("replication")
    };

    let mut runtime = NodeRuntime::new(build_config()).with_execution_hook(
        RecordingExecutionHook::new(Arc::new(Mutex::new(Vec::new()))),
    );
    runtime.start().expect("start first");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        let snapshot = runtime.snapshot();
        snapshot.consensus.committed_height >= 8 && snapshot.consensus.last_execution_height >= 8
    });
    runtime.stop().expect("stop first");
    let first = runtime.snapshot();
    assert!(first.last_error.is_none());
    assert!(
        reached
            && first.consensus.committed_height >= 8
            && first.consensus.last_execution_height >= 8,
        "runtime did not reach seed height before restart: committed={} execution={} last_error={:?}",
        first.consensus.committed_height,
        first.consensus.last_execution_height,
        first.last_error
    );

    let state_path = dir.join("node_pos_state.json");
    assert!(state_path.exists());
    let persisted = serde_json::from_slice::<super::pos_state_store::PosNodeStateSnapshot>(
        &fs::read(&state_path).expect("read pos state"),
    )
    .expect("parse pos state");
    assert!(persisted.committed_height >= first.consensus.committed_height);
    assert!(persisted.last_execution_height >= first.consensus.last_execution_height);
    assert!(persisted.last_observed_slot >= first.consensus.last_observed_slot);
    assert!(persisted.missed_slot_count >= first.consensus.missed_slot_count);
    assert!(persisted.last_execution_block_hash.is_some());
    assert!(persisted.last_execution_state_root.is_some());

    let mut runtime = NodeRuntime::new(build_config()).with_execution_hook(
        RecordingExecutionHook::new(Arc::new(Mutex::new(Vec::new()))),
    );
    runtime.start().expect("start second");
    let advanced = wait_until(Instant::now() + Duration::from_secs(2), || {
        let snapshot = runtime.snapshot();
        snapshot.consensus.committed_height > first.consensus.committed_height
            && snapshot.consensus.last_execution_height > first.consensus.last_execution_height
    });
    runtime.stop().expect("stop second");
    let second = runtime.snapshot();
    assert!(second.last_error.is_none());
    assert!(
        advanced
            && second.consensus.committed_height > first.consensus.committed_height
            && second.consensus.last_execution_height > first.consensus.last_execution_height,
        "runtime should advance after restart: first_committed={} second_committed={} first_execution={} second_execution={} last_error={:?}",
        first.consensus.committed_height,
        second.consensus.committed_height,
        first.consensus.last_execution_height,
        second.consensus.last_execution_height,
        second.last_error
    );
    assert!(second.consensus.last_observed_slot >= first.consensus.last_observed_slot);

    let _ = fs::remove_dir_all(&dir);
}
