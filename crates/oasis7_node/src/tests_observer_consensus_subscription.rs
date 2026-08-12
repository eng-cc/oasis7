#[test]
fn production_observer_runtime_enables_consensus_subscription_without_publish() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../oasis7/src/bin/oasis7_chain_runtime.rs"
    ));
    let attach = source
        .split_once("fn attach_default_replication_network")
        .map(|(_, body)| body)
        .expect("default replication attachment exists");
    assert!(
        attach.contains("with_replication_network_consensus_enabled(true)"),
        "production observer runtime must enable consensus subscription before checkpoint preflight"
    );

    let world_id = "world-observer-consensus-policy";
    let network = Arc::new(TestInMemoryNetwork::default());
    let config = NodeConfig::new("observer", world_id, NodeRole::Observer)
        .expect("observer config");
    let endpoint = ConsensusNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        true,
        &config.network_policy,
    )
    .expect("observer consensus subscription endpoint");
    assert!(!endpoint.allows_publish());
}

#[test]
fn observer_replication_runtime_ingests_signed_commit_before_checkpoint_preflight() {
    let world_id = "world-observer-consensus-preflight";
    let network = Arc::new(TestInMemoryNetwork::default());
    let (_, validator_public_key) = deterministic_keypair_hex(207);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 207)],
    );
    let mut commit = GossipCommitMessage {
        version: 1,
        world_id: world_id.to_string(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        height: 128,
        slot: 128,
        epoch: 0,
        block_hash: "block-128".to_string(),
        action_root: empty_action_root(),
        actions: Vec::new(),
        committed_at_ms: 128_000,
        execution_block_hash: None,
        execution_state_root: None,
        public_key_hex: None,
        signature_hex: None,
    };
    let signing_key = SigningKey::from_bytes(
        &hex::decode(deterministic_keypair_hex(207).0)
            .expect("decode validator private key")
            .try_into()
            .expect("validator private key length"),
    );
    let signer = ConsensusMessageSigner::new(signing_key, validator_public_key)
        .expect("validator commit signer");
    sign_commit_message(&mut commit, &signer).expect("sign validator commit");
    let commit_topic = super::network_bridge::default_consensus_commit_topic(world_id);
    oasis7_proto::distributed_net::DistributedNetwork::publish(
        network.as_ref(),
        commit_topic.as_str(),
        serde_json::to_vec(&commit)
            .expect("encode validator commit")
            .as_slice(),
    )
    .expect("publish validator commit");

    let config = NodeConfig::new("observer", world_id, NodeRole::Observer)
        .expect("observer config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("observer tick")
        .with_pos_config(pos_config)
        .expect("observer validators")
        .with_replication(signed_replication_config(temp_dir("observer-consensus-preflight"), 209));
    let mut runtime = NodeRuntime::new(config)
        .with_replication_network(NodeReplicationNetworkHandle::new(network))
        // Mirrors the live chain-runtime assembly: ObserverLight subscribes to
        // consensus gossip while its network policy still forbids publishing.
        .with_replication_network_consensus_enabled(true);
    runtime.start().expect("start observer runtime");
    let observed = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.known_peer_heads >= 1
    });
    let snapshot = runtime.snapshot();
    runtime.stop().expect("stop observer runtime");
    assert!(
        observed,
        "signed validator commit must populate peer_heads before replication checkpoint preflight: known_peer_heads={} peer_heads={:?} last_error={:?}",
        snapshot.consensus.known_peer_heads,
        snapshot.consensus.peer_heads,
        snapshot.last_error
    );
}
