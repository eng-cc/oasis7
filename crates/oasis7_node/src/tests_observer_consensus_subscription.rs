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
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let config =
        NodeConfig::new("observer", world_id, NodeRole::Observer).expect("observer config");
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
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
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
        .with_replication(signed_replication_config(
            temp_dir("observer-consensus-preflight"),
            209,
        ));
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
        snapshot.consensus.known_peer_heads, snapshot.consensus.peer_heads, snapshot.last_error
    );
}

#[test]
fn testnet_250_default_consensus_topic_delivers_two_signed_validator_heads_before_preflight() {
    let world_id = "world-testnet-250-default-consensus-topic";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 50,
        },
        PosValidator {
            validator_id: "node-c".to_string(),
            stake: 50,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 250), ("node-c", 251)]);
    let publish_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("validator config")
        .with_pos_config(pos_config.clone())
        .expect("validator pos config");
    let publish_endpoint = ConsensusNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        world_id,
        false,
        &publish_config.network_policy,
    )
    .expect("validator consensus endpoint");
    for (node_id, seed, height) in [("node-a", 250_u8, 256_u64), ("node-c", 251, 256)] {
        let mut commit = GossipCommitMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: node_id.to_string(),
            height,
            slot: height,
            epoch: 0,
            block_hash: format!("block-{node_id}-{height}"),
            action_root: empty_action_root(),
            actions: Vec::new(),
            committed_at_ms: height as i64 * 1_000,
            execution_block_hash: Some(format!("exec-block-{node_id}-{height}")),
            execution_state_root: Some(format!("exec-state-{node_id}-{height}")),
            public_key_hex: None,
            signature_hex: None,
        };
        let (private_hex, public_hex) = deterministic_keypair_hex(seed);
        let signing_key = SigningKey::from_bytes(
            &hex::decode(private_hex)
                .expect("decode validator private key")
                .try_into()
                .expect("validator private key length"),
        );
        let signer =
            ConsensusMessageSigner::new(signing_key, public_hex).expect("validator commit signer");
        sign_commit_message(&mut commit, &signer).expect("sign validator commit");
        publish_endpoint
            .publish_commit(&commit)
            .expect("validator endpoint publishes signed commit");
    }

    let observer_config = NodeConfig::new("observer", world_id, NodeRole::Observer)
        .expect("observer config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("observer tick")
        .with_pos_config(pos_config)
        .expect("observer validators")
        .with_replication(signed_replication_config(
            temp_dir("testnet-250-consensus"),
            252,
        ));
    let mut runtime = NodeRuntime::new(observer_config)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)))
        // RED fixture: this is the pre-enable ObserverLight assembly. The
        // production chain runtime must turn this lane on before preflight.
        .with_replication_network_consensus_enabled(false);
    runtime.start().expect("start observer runtime");
    let converged = wait_until(Instant::now() + Duration::from_secs(2), || {
        let snapshot = runtime.snapshot();
        snapshot.consensus.known_peer_heads >= 2
            && snapshot.consensus.network_committed_height >= 256
    });
    let snapshot = runtime.snapshot();
    runtime.stop().expect("stop observer runtime");
    assert!(
        converged,
        "testnet.250 signed validator authority must arrive on the default consensus commit topic before checkpoint preflight: known_peer_heads={} network_committed_height={} peer_heads={:?} last_error={:?}",
        snapshot.consensus.known_peer_heads,
        snapshot.consensus.network_committed_height,
        snapshot.consensus.peer_heads,
        snapshot.last_error
    );
}

#[derive(Clone)]
struct Testnet250ConnectedOnlyNetwork {
    inner: Arc<TestInMemoryNetwork>,
    peers: Vec<String>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for Testnet250ConnectedOnlyNetwork
{
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        self.peers.clone()
    }

    fn known_peer_ids(&self) -> Vec<String> {
        self.peers.clone()
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

#[test]
fn testnet_250_connected_quorum_without_commit_publication_does_not_create_authority() {
    let world_id = "world-testnet-250-connected-without-publication";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(Testnet250ConnectedOnlyNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        peers: vec!["node-a".to_string(), "node-c".to_string()],
    });
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 50,
            },
            PosValidator {
                validator_id: "node-c".to_string(),
                stake: 50,
            },
        ],
        &[("node-a", 250), ("node-c", 251)],
    );
    let config = NodeConfig::new("observer", world_id, NodeRole::Observer)
        .expect("observer config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("observer tick")
        .with_pos_config(pos_config)
        .expect("observer validators")
        .with_replication(signed_replication_config(
            temp_dir("testnet-250-no-publication"),
            252,
        ));
    let mut runtime = NodeRuntime::new(config)
        .with_replication_network(NodeReplicationNetworkHandle::new(network))
        .with_replication_network_consensus_enabled(true);
    runtime.start().expect("start observer runtime");
    thread::sleep(Duration::from_millis(300));
    let snapshot = runtime.snapshot();
    runtime.stop().expect("stop observer runtime");
    assert_eq!(snapshot.consensus.known_peer_heads, 0);
    assert_eq!(snapshot.consensus.network_committed_height, 0);
    assert_eq!(snapshot.consensus.committed_height, 0);
}
