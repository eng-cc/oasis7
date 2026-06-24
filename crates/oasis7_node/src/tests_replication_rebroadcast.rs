#[test]
fn replicated_commit_head_network_rebroadcast_is_independent_from_local_commit_broadcast() {
    let world_id = "world-replicated-head-independent-broadcast";
    let dir = temp_dir("replicated-head-independent-broadcast");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 89)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Storage)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_network_policy(NodeNetworkPolicy {
            deployment_mode: oasis7_proto::distributed_dht::PeerDeploymentMode::Private,
            node_role_claim: oasis7_proto::distributed_dht::PeerNodeRole::ValidatorCore,
        })
        .expect("validator-core policy")
        .with_replication(signed_replication_config(dir.clone(), 89));
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("replication runtime");
    let replicated_decision = PosDecision {
        height: 3,
        slot: 3,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "replicated-block-3".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication
        .build_local_commit_message(
            "node-a",
            world_id,
            4_003,
            &replicated_decision,
            Some("replicated-exec-block-3"),
            Some("replicated-exec-state-3"),
        )
        .expect("build replicated commit")
        .expect("replicated commit");
    engine.committed_height = 3;
    engine.replication_persisted_height = 3;
    engine.last_committed_block_hash = Some("replicated-block-3".to_string());
    engine.remember_execution_binding(
        3,
        "replicated-exec-block-3".to_string(),
        "replicated-exec-state-3".to_string(),
    );

    let network = Arc::new(TestInMemoryNetwork::default());
    let handle = NodeReplicationNetworkHandle::new(network.clone());
    let endpoint = ConsensusNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
        .expect("consensus endpoint");
    let local_decision = PosDecision {
        block_hash: "replicated-block-3".to_string(),
        ..replicated_decision.clone()
    };

    engine
        .broadcast_local_commit_network(&endpoint, "node-a", world_id, 5_000, &local_decision)
        .expect("broadcast local commit");
    engine
        .broadcast_replicated_commit_head_network(
            &endpoint,
            "node-a",
            world_id,
            5_000,
            Some(&replication),
        )
        .expect("broadcast replicated commit head");

    let commit_topic = super::network_bridge::default_consensus_commit_topic(world_id);
    let payloads = network
        .retained
        .lock()
        .expect("lock retained")
        .get(commit_topic.as_str())
        .cloned()
        .unwrap_or_default();
    let commits = payloads
        .iter()
        .filter_map(|payload| serde_json::from_slice::<GossipCommitMessage>(payload).ok())
        .map(|commit| (commit.block_hash, commit.committed_at_ms))
        .collect::<Vec<_>>();

    assert_eq!(
        commits,
        vec![
            ("replicated-block-3".to_string(), 5_000),
            ("replicated-block-3".to_string(), 4_003)
        ],
        "replicated commit head should rebroadcast even when local commit used the same height in the same tick"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn replicated_commit_head_network_rebroadcast_rejects_same_height_conflict() {
    let world_id = "world-replicated-head-conflict";
    let dir = temp_dir("replicated-head-conflict");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 91)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Storage)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_network_policy(NodeNetworkPolicy {
            deployment_mode: oasis7_proto::distributed_dht::PeerDeploymentMode::Private,
            node_role_claim: oasis7_proto::distributed_dht::PeerNodeRole::ValidatorCore,
        })
        .expect("validator-core policy")
        .with_replication(signed_replication_config(dir.clone(), 91));
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("replication runtime");
    let replicated_decision = PosDecision {
        height: 3,
        slot: 3,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "replicated-conflict-block-3".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication
        .build_local_commit_message(
            "node-a",
            world_id,
            4_203,
            &replicated_decision,
            Some("replicated-conflict-exec-block-3"),
            Some("replicated-conflict-exec-state-3"),
        )
        .expect("build replicated commit")
        .expect("replicated commit");
    engine.committed_height = 3;
    engine.replication_persisted_height = 3;
    engine.last_committed_block_hash = Some("local-conflict-block-3".to_string());

    let network = Arc::new(TestInMemoryNetwork::default());
    let handle = NodeReplicationNetworkHandle::new(network.clone());
    let endpoint = ConsensusNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
        .expect("consensus endpoint");
    let local_decision = PosDecision {
        block_hash: "local-conflict-block-3".to_string(),
        ..replicated_decision.clone()
    };

    engine
        .broadcast_local_commit_network(&endpoint, "node-a", world_id, 5_200, &local_decision)
        .expect("broadcast local commit");
    let err = engine
        .broadcast_replicated_commit_head_network(
            &endpoint,
            "node-a",
            world_id,
            5_200,
            Some(&replication),
        )
        .expect_err("conflicting replicated head should fail closed");

    assert!(
        matches!(
            err,
            NodeError::Replication { ref reason }
                if reason.contains("replicated commit head conflicts with local committed head")
                    && reason.contains("local-conflict-block-3")
                    && reason.contains("replicated-conflict-block-3")
        ),
        "unexpected error: {err:?}"
    );
    let commit_topic = super::network_bridge::default_consensus_commit_topic(world_id);
    let payloads = network
        .retained
        .lock()
        .expect("lock retained")
        .get(commit_topic.as_str())
        .cloned()
        .unwrap_or_default();
    assert_eq!(payloads.len(), 1, "conflicting replicated head must not publish");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn replicated_commit_head_network_rebroadcast_allows_legacy_restored_local_hash() {
    let world_id = "world-replicated-head-legacy-local";
    let dir = temp_dir("replicated-head-legacy-local");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 92)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Storage)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_network_policy(NodeNetworkPolicy {
            deployment_mode: oasis7_proto::distributed_dht::PeerDeploymentMode::Private,
            node_role_claim: oasis7_proto::distributed_dht::PeerNodeRole::ValidatorCore,
        })
        .expect("validator-core policy")
        .with_replication(signed_replication_config(dir.clone(), 92));
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("replication runtime");
    let replicated_decision = PosDecision {
        height: 3,
        slot: 3,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "real-replicated-block-3".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication
        .build_local_commit_message(
            "node-a",
            world_id,
            4_303,
            &replicated_decision,
            Some("real-replicated-exec-block-3"),
            Some("real-replicated-exec-state-3"),
        )
        .expect("build replicated commit")
        .expect("replicated commit");
    engine.committed_height = 3;
    engine.replication_persisted_height = 3;
    engine.last_committed_block_hash = Some("legacy-height-3".to_string());

    let network = Arc::new(TestInMemoryNetwork::default());
    let handle = NodeReplicationNetworkHandle::new(network.clone());
    let endpoint = ConsensusNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
        .expect("consensus endpoint");

    engine
        .broadcast_replicated_commit_head_network(
            &endpoint,
            "node-a",
            world_id,
            5_300,
            Some(&replication),
        )
        .expect("legacy synthetic local hash should not reject real persisted commit head");

    let commit_topic = super::network_bridge::default_consensus_commit_topic(world_id);
    let payloads = network
        .retained
        .lock()
        .expect("lock retained")
        .get(commit_topic.as_str())
        .cloned()
        .unwrap_or_default();
    let commits = payloads
        .iter()
        .filter_map(|payload| serde_json::from_slice::<GossipCommitMessage>(payload).ok())
        .map(|commit| commit.block_hash)
        .collect::<Vec<_>>();

    assert_eq!(commits, vec!["real-replicated-block-3".to_string()]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn replicated_commit_head_gossip_rebroadcast_is_independent_from_local_commit_broadcast() {
    let world_id = "world-replicated-head-independent-gossip";
    let dir = temp_dir("replicated-head-independent-gossip");
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 90)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Storage)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 90));
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("replication runtime");
    let replicated_decision = PosDecision {
        height: 3,
        slot: 3,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "replicated-gossip-block-3".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication
        .build_local_commit_message(
            "node-a",
            world_id,
            4_103,
            &replicated_decision,
            Some("replicated-gossip-exec-block-3"),
            Some("replicated-gossip-exec-state-3"),
        )
        .expect("build replicated commit")
        .expect("replicated commit");
    engine.committed_height = 3;
    engine.replication_persisted_height = 3;
    engine.last_committed_block_hash = Some("replicated-gossip-block-3".to_string());
    engine.remember_execution_binding(
        3,
        "replicated-gossip-exec-block-3".to_string(),
        "replicated-gossip-exec-state-3".to_string(),
    );

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b = GossipEndpoint::bind(&gossip_config(addr_b, Vec::new())).expect("endpoint b");
    let local_decision = PosDecision {
        block_hash: "replicated-gossip-block-3".to_string(),
        ..replicated_decision.clone()
    };

    engine
        .broadcast_local_commit(&endpoint_a, "node-a", world_id, 5_100, &local_decision)
        .expect("broadcast local commit");
    engine
        .broadcast_replicated_commit_head_gossip(
            &endpoint_a,
            "node-a",
            world_id,
            5_100,
            Some(&replication),
        )
        .expect("broadcast replicated commit head");
    thread::sleep(Duration::from_millis(20));

    let commits = endpoint_b
        .drain_messages()
        .expect("drain endpoint b")
        .into_iter()
        .filter_map(|received| match received.message {
            GossipMessage::Commit(commit) => Some((commit.block_hash, commit.committed_at_ms)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        commits,
        vec![
            ("replicated-gossip-block-3".to_string(), 5_100),
            ("replicated-gossip-block-3".to_string(), 4_103)
        ],
        "replicated commit head should gossip even when local commit used the same height in the same tick"
    );

    let _ = fs::remove_dir_all(&dir);
}
