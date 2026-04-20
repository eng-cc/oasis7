use super::*;

#[test]
fn runtime_execution_hook_updates_consensus_snapshot() {
    let config = NodeConfig::new("node-exec", "world-exec", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let hook = RecordingExecutionHook::new(Arc::clone(&calls));
    let mut runtime = NodeRuntime::new(config).with_execution_hook(hook);
    runtime.start().expect("start");
    std::thread::sleep(Duration::from_millis(120));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(snapshot.consensus.committed_height >= 1);
    assert!(snapshot.consensus.last_execution_height >= 1);
    assert!(snapshot.consensus.last_execution_block_hash.is_some());
    assert!(snapshot.consensus.last_execution_state_root.is_some());

    let execution_calls = calls.lock().expect("lock calls");
    assert!(!execution_calls.is_empty());
    assert!(execution_calls
        .iter()
        .all(|call| call.world_id == "world-exec" && call.node_id == "node-exec"));
}

#[test]
fn pos_engine_signature_enforced_rejects_unsigned_proposal() {
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
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 203), ("node-b", 201)]);
    let config_b = NodeConfig::new("node-b", "world-sig-enforced", NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(temp_dir("sig-enforced"), 201));
    let mut engine = PosNodeEngine::new(&config_b).expect("engine");

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let unsigned_proposal = GossipProposalMessage {
        version: 1,
        world_id: config_b.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: format!("{}:h1:s0:p{}", config_b.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    endpoint_a
        .broadcast_proposal(&unsigned_proposal)
        .expect("broadcast unsigned proposal");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config_b.node_id, &config_b.world_id, None, 0)
        .expect("ingest");
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_rejects_future_slot_proposal_from_peer_messages() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let config = NodeConfig::new(
        "node-b",
        "world-future-slot-peer-reject",
        NodeRole::Observer,
    )
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
    .expect("validators");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 9,
        epoch: 0,
        block_hash: format!("{}:h1:s9:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    endpoint_a
        .broadcast_proposal(&proposal)
        .expect("broadcast proposal");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config.node_id, &config.world_id, None, 2)
        .expect("ingest");
    assert!(engine.pending.is_none());
    assert_eq!(engine.inbound_rejected_proposal_future_slot, 1);
}

#[test]
fn pos_engine_signature_enforced_accepts_signed_proposal_and_attestation() {
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
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 203), ("node-b", 202)]);
    let config_b = NodeConfig::new("node-b", "world-sig-accept", NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(temp_dir("sig-accept"), 202));
    let mut engine = PosNodeEngine::new(&config_b).expect("engine");

    let (proposal_private_hex, proposal_public_hex) = deterministic_keypair_hex(203);
    let proposal_signing_key = SigningKey::from_bytes(
        &hex::decode(proposal_private_hex)
            .expect("proposal private decode")
            .try_into()
            .expect("proposal private len"),
    );
    let proposal_signer =
        ConsensusMessageSigner::new(proposal_signing_key, proposal_public_hex).expect("signer");

    let (attestation_private_hex, attestation_public_hex) = deterministic_keypair_hex(202);
    let attestation_signing_key = SigningKey::from_bytes(
        &hex::decode(attestation_private_hex)
            .expect("attestation private decode")
            .try_into()
            .expect("attestation private len"),
    );
    let attestation_signer =
        ConsensusMessageSigner::new(attestation_signing_key, attestation_public_hex)
            .expect("attestation signer");

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let mut proposal = GossipProposalMessage {
        version: 1,
        world_id: config_b.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: format!("{}:h1:s0:p{}", config_b.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 2_000,
        public_key_hex: None,
        signature_hex: None,
    };
    sign_proposal_message(&mut proposal, &proposal_signer).expect("sign proposal");
    endpoint_a
        .broadcast_proposal(&proposal)
        .expect("broadcast signed proposal");
    thread::sleep(Duration::from_millis(20));

    let mut attestation = GossipAttestationMessage {
        version: 1,
        world_id: config_b.world_id.clone(),
        node_id: "node-b".to_string(),
        player_id: "node-b".to_string(),
        validator_id: "node-b".to_string(),
        height: proposal.height,
        slot: proposal.slot,
        epoch: proposal.epoch,
        block_hash: proposal.block_hash.clone(),
        approve: true,
        source_epoch: 0,
        target_epoch: 0,
        voted_at_ms: 2_001,
        reason: Some("signed attestation".to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    sign_attestation_message(&mut attestation, &attestation_signer).expect("sign attestation");
    endpoint_a
        .broadcast_attestation(&attestation)
        .expect("broadcast signed attestation");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config_b.node_id, &config_b.world_id, None, 0)
        .expect("ingest");
    let pending = engine.pending.as_ref().expect("pending exists");
    assert_eq!(pending.height, 1);
    assert!(pending.attestations.contains_key("node-a"));
    assert!(pending.attestations.contains_key("node-b"));
}
