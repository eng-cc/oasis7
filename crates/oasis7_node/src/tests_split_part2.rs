#[path = "tests_network_gap_sync.rs"]
mod network_gap_sync_tests;

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
    thread::sleep(Duration::from_millis(180));
    runtime.stop().expect("stop first");
    let first = runtime.snapshot();
    assert!(first.last_error.is_none());
    assert!(first.consensus.committed_height >= 8);
    assert!(first.consensus.last_execution_height >= 8);

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
    thread::sleep(Duration::from_millis(40));
    runtime.stop().expect("stop second");
    let second = runtime.snapshot();
    assert!(second.last_error.is_none());
    assert!(second.consensus.committed_height > first.consensus.committed_height);
    assert!(second.consensus.last_execution_height > first.consensus.last_execution_height);
    assert!(second.consensus.last_observed_slot >= first.consensus.last_observed_slot);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn config_with_gossip_rejects_empty_peers() {
    let bind_socket = UdpSocket::bind("127.0.0.1:0").expect("bind");
    let bind_addr = bind_socket.local_addr().expect("addr");
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("config")
        .with_gossip(bind_addr, vec![]);
    assert!(matches!(config, Err(NodeError::InvalidConfig { .. })));
}

#[test]
fn gossip_endpoint_learns_inbound_peer_for_followup_broadcasts() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let world_id = "world-gossip-discovery";
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Observer)
        .expect("config a")
        .with_pos_validators(vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 50,
            },
            PosValidator {
                validator_id: "node-b".to_string(),
                stake: 50,
            },
        ])
        .expect("validators")
        .with_gossip_optional(addr_a, Vec::new());
    let mut engine_a = PosNodeEngine::new(&config_a).expect("engine a");

    let endpoint_a = GossipEndpoint::bind(&gossip_config(addr_a, Vec::new())).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    endpoint_b
        .broadcast_commit(&GossipCommitMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: "node-b".to_string(),
            player_id: "node-b".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            block_hash: "block-b-1".to_string(),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: 1_000,
            execution_block_hash: None,
            execution_state_root: None,
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("broadcast to a");
    thread::sleep(Duration::from_millis(20));
    engine_a
        .ingest_peer_messages(&endpoint_a, "node-a", world_id, None, 0)
        .expect("ingest from b");

    endpoint_a
        .broadcast_commit(&GossipCommitMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: "node-a".to_string(),
            player_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            block_hash: "block-a-2".to_string(),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: 2_000,
            execution_block_hash: None,
            execution_state_root: None,
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("rebroadcast to discovered peer");
    thread::sleep(Duration::from_millis(20));

    let echoed = endpoint_b.drain_messages().expect("drain endpoint b");
    assert!(echoed.iter().any(|received| {
        matches!(
            &received.message,
            GossipMessage::Commit(commit) if commit.node_id == "node-a" && commit.height == 2
        )
    }));
}

#[test]
fn gossip_endpoint_enforces_dynamic_peer_capacity() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let socket_c = UdpSocket::bind("127.0.0.1:0").expect("bind c");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    let addr_c = socket_c.local_addr().expect("addr c");
    drop(socket_a);
    drop(socket_b);
    drop(socket_c);

    let endpoint_a = GossipEndpoint::bind(&NodeGossipConfig {
        bind_addr: addr_a,
        peers: Vec::new(),
        max_dynamic_peers: 1,
        dynamic_peer_ttl_ms: 60_000,
    })
    .expect("endpoint a");
    let endpoint_b = GossipEndpoint::bind(&gossip_config(addr_b, Vec::new())).expect("endpoint b");
    let endpoint_c = GossipEndpoint::bind(&gossip_config(addr_c, Vec::new())).expect("endpoint c");

    endpoint_a.remember_peer(addr_b).expect("remember b");
    thread::sleep(Duration::from_millis(2));
    endpoint_a.remember_peer(addr_c).expect("remember c");

    endpoint_a
        .broadcast_commit(&GossipCommitMessage {
            version: 1,
            world_id: "world-peer-cap".to_string(),
            node_id: "node-a".to_string(),
            player_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            block_hash: "block-a-1".to_string(),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: 1_000,
            execution_block_hash: None,
            execution_state_root: None,
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("broadcast from a");
    thread::sleep(Duration::from_millis(20));

    let to_b = endpoint_b.drain_messages().expect("drain b");
    let to_c = endpoint_c.drain_messages().expect("drain c");
    assert!(
        !to_b.iter().any(|received| {
            matches!(
                &received.message,
                GossipMessage::Commit(commit)
                    if commit.node_id == "node-a" && commit.height == 1
            )
        }),
        "oldest dynamic peer should be evicted when capacity is full"
    );
    assert!(
        to_c.iter().any(|received| {
            matches!(
                &received.message,
                GossipMessage::Commit(commit)
                    if commit.node_id == "node-a" && commit.height == 1
            )
        }),
        "most recent dynamic peer should remain routable"
    );
}

#[test]
fn gossip_endpoint_expires_dynamic_peers_by_ttl() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let endpoint_a = GossipEndpoint::bind(&NodeGossipConfig {
        bind_addr: addr_a,
        peers: Vec::new(),
        max_dynamic_peers: 4,
        dynamic_peer_ttl_ms: 20,
    })
    .expect("endpoint a");
    let endpoint_b = GossipEndpoint::bind(&gossip_config(addr_b, Vec::new())).expect("endpoint b");

    endpoint_a.remember_peer(addr_b).expect("remember b");
    thread::sleep(Duration::from_millis(40));

    endpoint_a
        .broadcast_commit(&GossipCommitMessage {
            version: 1,
            world_id: "world-peer-ttl".to_string(),
            node_id: "node-a".to_string(),
            player_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            block_hash: "block-a-2".to_string(),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: 2_000,
            execution_block_hash: None,
            execution_state_root: None,
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("broadcast from a");
    thread::sleep(Duration::from_millis(20));

    let to_b = endpoint_b.drain_messages().expect("drain b");
    assert!(
        !to_b.iter().any(|received| {
            matches!(
                &received.message,
                GossipMessage::Commit(commit)
                    if commit.node_id == "node-a" && commit.height == 2
            )
        }),
        "expired dynamic peer should not receive broadcasts"
    );
}

#[test]
fn runtime_gossip_tracks_peer_committed_heads() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];

    let config_a = NodeConfig::new("node-a", "world-sync", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_validators(validators.clone())
        .expect("validators a")
        .with_auto_attest_all_validators(true)
        .with_gossip_optional(addr_a, vec![addr_b]);
    let config_b = NodeConfig::new("node-b", "world-sync", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_validators(validators)
        .expect("validators b")
        .with_auto_attest_all_validators(true)
        .with_gossip_optional(addr_b, vec![addr_a]);

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    let mut runtime_b = NodeRuntime::new(config_b);
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");
    let synced = wait_until(Instant::now() + Duration::from_secs(8), || {
        let snapshot_a = runtime_a.snapshot();
        let snapshot_b = runtime_b.snapshot();
        snapshot_a.consensus.network_committed_height >= 1
            && snapshot_b.consensus.network_committed_height >= 1
            && snapshot_a.consensus.known_peer_heads >= 1
            && snapshot_b.consensus.known_peer_heads >= 1
    });
    assert!(synced, "runtime gossip did not observe peer heads in time");

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");
}

#[test]
fn runtime_gossip_tracks_peer_heads_when_replication_network_consensus_is_disabled() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let config_a = NodeConfig::new("node-a", "world-sync-fallback-network", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_validators(validators.clone())
        .expect("validators a")
        .with_auto_attest_all_validators(true)
        .with_gossip_optional(addr_a, vec![addr_b]);
    let config_b = NodeConfig::new("node-b", "world-sync-fallback-network", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_validators(validators)
        .expect("validators b")
        .with_auto_attest_all_validators(true)
        .with_gossip_optional(addr_b, vec![addr_a]);

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)))
        .with_replication_network_consensus_enabled(false);
    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)))
        .with_replication_network_consensus_enabled(false);
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");

    let synced = wait_until(Instant::now() + Duration::from_secs(8), || {
        let snapshot_a = runtime_a.snapshot();
        let snapshot_b = runtime_b.snapshot();
        snapshot_a.consensus.network_committed_height >= 1
            && snapshot_b.consensus.network_committed_height >= 1
            && snapshot_a.consensus.known_peer_heads >= 1
            && snapshot_b.consensus.known_peer_heads >= 1
    });
    assert!(
        synced,
        "runtime gossip did not observe peer heads when replication-network consensus was disabled"
    );

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");
}

#[test]
fn runtime_network_consensus_syncs_peer_heads_without_udp_gossip() {
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let config_a = NodeConfig::new("node-a", "world-network-consensus", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_validators(validators.clone())
        .expect("validators a")
        .with_auto_attest_all_validators(true);
    let config_b = NodeConfig::new("node-b", "world-network-consensus", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_validators(validators)
        .expect("validators b");

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");
    thread::sleep(Duration::from_millis(200));

    let snapshot_a = runtime_a.snapshot();
    let snapshot_b = runtime_b.snapshot();
    assert!(snapshot_a.consensus.committed_height >= 1);
    assert!(snapshot_b.consensus.network_committed_height >= 1);
    assert!(snapshot_b.consensus.known_peer_heads >= 1);

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");
}

#[test]
fn runtime_gossip_replication_syncs_distfs_commit_files() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let dir_a = temp_dir("replication-a");
    let dir_b = temp_dir("replication-b");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];

    let config_a = NodeConfig::new("node-a", "world-repl", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_validators(validators.clone())
        .expect("validators a")
        .with_gossip_optional(addr_a, vec![addr_b])
        .with_replication_root(dir_a.clone())
        .expect("replication a");
    let config_b = NodeConfig::new("node-b", "world-repl", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_validators(validators)
        .expect("validators b")
        .with_gossip_optional(addr_b, vec![addr_a])
        .with_replication_root(dir_b.clone())
        .expect("replication b");

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    let mut runtime_b = NodeRuntime::new(config_b);
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");
    thread::sleep(Duration::from_millis(220));

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");

    let store_b = LocalCasStore::new(dir_b.join("store"));
    let files = store_b.list_files().expect("list files");
    assert!(files
        .iter()
        .any(|item| item.path.starts_with("consensus/commits/")));
    assert!(dir_b.join("replication_guard.json").exists());

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_network_replication_syncs_distfs_commit_files() {
    let dir_a = temp_dir("network-repl-a");
    let dir_b = temp_dir("network-repl-b");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 71), ("node-b", 72)]);
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let config_a = NodeConfig::new("node-a", "world-network-repl", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir_a.clone(), 71));
    let config_b = NodeConfig::new("node-b", "world-network-repl", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 72));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");
    thread::sleep(Duration::from_millis(220));

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");

    let store_b = LocalCasStore::new(dir_b.join("store"));
    let files = store_b.list_files().expect("list files");
    assert!(files
        .iter()
        .any(|item| item.path.starts_with("consensus/commits/")));

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_network_replication_fetch_handlers_serve_commit_and_blob() {
    let dir_a = temp_dir("network-fetch-a");
    let validators = vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }];
    let pos_config = signed_pos_config_with_signer_seeds(validators, &[("node-a", 77)]);
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let config_a = NodeConfig::new("node-a", "world-network-fetch", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config)
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir_a.clone(), 77));
    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));

    runtime_a.start().expect("start a");
    thread::sleep(Duration::from_millis(180));
    let snapshot = runtime_a.snapshot();
    assert!(snapshot.consensus.committed_height >= 1);
    let target_height = snapshot.consensus.committed_height;

    let fetch_commit_request =
        signed_fetch_commit_request_for_test("world-network-fetch", target_height, 77);
    let fetch_commit_payload =
        serde_json::to_vec(&fetch_commit_request).expect("encode fetch commit request");
    let fetch_commit_response_payload = network
        .request(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            fetch_commit_payload.as_slice(),
        )
        .expect("fetch commit response");
    let fetch_commit_response: super::replication::FetchCommitResponse =
        serde_json::from_slice(&fetch_commit_response_payload).expect("decode fetch commit");
    assert!(fetch_commit_response.found);
    let commit_message = fetch_commit_response.message.expect("commit message");
    assert_eq!(commit_message.world_id, "world-network-fetch");
    assert_eq!(commit_message.record.world_id, "world-network-fetch");
    assert_eq!(
        commit_message.record.path,
        format!("consensus/commits/{:020}.json", target_height)
    );

    let fetch_blob_request =
        signed_fetch_blob_request_for_test(commit_message.record.content_hash.as_str(), 77);
    let fetch_blob_payload =
        serde_json::to_vec(&fetch_blob_request).expect("encode fetch blob request");
    let fetch_blob_response_payload = network
        .request(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            fetch_blob_payload.as_slice(),
        )
        .expect("fetch blob response");
    let fetch_blob_response: super::replication::FetchBlobResponse =
        serde_json::from_slice(&fetch_blob_response_payload).expect("decode fetch blob");
    assert!(fetch_blob_response.found);
    assert_eq!(
        fetch_blob_response.blob.expect("blob payload"),
        commit_message.payload
    );

    runtime_a.stop().expect("stop a");
    let _ = fs::remove_dir_all(&dir_a);
}
