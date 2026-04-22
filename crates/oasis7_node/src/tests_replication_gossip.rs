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
fn runtime_gossip_seeds_reverse_path_for_private_observer() {
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

    let config_a = NodeConfig::new("node-a", "world-private-observer-seed", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_validators(validators.clone())
        .expect("validators a")
        .with_auto_attest_all_validators(true)
        .with_gossip_optional(addr_a, Vec::new());
    let config_b = NodeConfig::new("node-b", "world-private-observer-seed", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_validators(validators)
        .expect("validators b")
        .with_gossip_optional(addr_b, vec![addr_a]);

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    let mut runtime_b = NodeRuntime::new(config_b);
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");

    let synced = wait_until(Instant::now() + Duration::from_secs(8), || {
        let snapshot_b = runtime_b.snapshot();
        snapshot_b.consensus.network_committed_height >= 1
            && snapshot_b.consensus.known_peer_heads >= 1
    });
    assert!(
        synced,
        "private observer did not learn reverse gossip path in asymmetric static-peer topology"
    );

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
    let synced = wait_until(Instant::now() + Duration::from_secs(8), || {
        let snapshot_a = runtime_a.snapshot();
        let snapshot_b = runtime_b.snapshot();
        snapshot_a.consensus.committed_height >= 1
            && snapshot_b.consensus.network_committed_height >= 1
            && snapshot_b.consensus.known_peer_heads >= 1
    });

    let snapshot_a = runtime_a.snapshot();
    let snapshot_b = runtime_b.snapshot();
    assert!(
        synced,
        "network consensus peer heads did not converge: a_committed={} a_last_error={:?} a_peer_heads={:?} b_network_committed={} b_known_peer_heads={} b_last_error={:?} b_peer_heads={:?}",
        snapshot_a.consensus.committed_height,
        snapshot_a.last_error,
        snapshot_a.consensus.peer_heads,
        snapshot_b.consensus.network_committed_height,
        snapshot_b.consensus.known_peer_heads,
        snapshot_b.last_error,
        snapshot_b.consensus.peer_heads
    );
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
