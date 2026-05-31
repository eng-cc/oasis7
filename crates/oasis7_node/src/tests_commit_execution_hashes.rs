use super::*;

#[test]
fn commit_signature_covers_execution_hashes() {
    let (private_hex, public_hex) = deterministic_keypair_hex(204);
    let signing_key = SigningKey::from_bytes(
        &hex::decode(private_hex)
            .expect("private decode")
            .try_into()
            .expect("private len"),
    );
    let signer = ConsensusMessageSigner::new(signing_key, public_hex).expect("signer");

    let mut commit = GossipCommitMessage {
        version: 1,
        world_id: "world-signature-exec".to_string(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        height: 7,
        slot: 3,
        epoch: 0,
        block_hash: "block-7".to_string(),
        action_root: empty_action_root(),
        actions: Vec::new(),
        committed_at_ms: 3_000,
        execution_block_hash: Some("exec-block-7".to_string()),
        execution_state_root: Some("exec-state-7".to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    sign_commit_message(&mut commit, &signer).expect("sign commit");
    verify_commit_message_signature(&commit, true).expect("verify signed commit");

    let mut tampered = commit.clone();
    tampered.execution_state_root = Some("exec-state-tampered".to_string());
    let err = verify_commit_message_signature(&tampered, true).expect_err("tamper must fail");
    assert!(err.reason.contains("verify commit signature failed"));

    let mut tampered_action_root = commit.clone();
    tampered_action_root.action_root = "tampered-action-root".to_string();
    let err =
        verify_commit_message_signature(&tampered_action_root, true).expect_err("tamper must fail");
    assert!(err.reason.contains("verify commit signature failed"));
}

#[test]
fn pos_engine_ingests_commit_execution_hashes() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let config = NodeConfig::new("node-b", "world-commit-exec-head", NodeRole::Observer)
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
            execution_block_hash: Some("exec-block-4".to_string()),
            execution_state_root: Some("exec-state-4".to_string()),
            public_key_hex: None,
            signature_hex: None,
        })
        .expect("broadcast commit");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config.node_id, &config.world_id, None, 0)
        .expect("ingest");
    let head = engine
        .peer_heads
        .get("node-a")
        .expect("peer head should exist");
    assert_eq!(head.height, 4);
    assert_eq!(head.execution_block_hash.as_deref(), Some("exec-block-4"));
    assert_eq!(head.execution_state_root.as_deref(), Some("exec-state-4"));
}

#[test]
fn pos_engine_quarantines_validator_on_commit_equivocation() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let config = NodeConfig::new("node-b", "world-commit-equivocation", NodeRole::Observer)
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
        .with_gossip_optional(addr_b, vec![addr_a]);
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let first = GossipCommitMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        height: 8,
        slot: 8,
        epoch: 0,
        block_hash: "block-8-a".to_string(),
        action_root: empty_action_root(),
        actions: Vec::new(),
        committed_at_ms: 8_000,
        execution_block_hash: Some("exec-block-8-a".to_string()),
        execution_state_root: Some("exec-state-8-a".to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    let second = GossipCommitMessage {
        block_hash: "block-8-b".to_string(),
        execution_block_hash: Some("exec-block-8-b".to_string()),
        execution_state_root: Some("exec-state-8-b".to_string()),
        committed_at_ms: 8_100,
        ..first.clone()
    };

    endpoint_a
        .broadcast_commit(&first)
        .expect("broadcast first");
    endpoint_a
        .broadcast_commit(&second)
        .expect("broadcast second");
    thread::sleep(Duration::from_millis(30));

    engine
        .ingest_peer_messages(&endpoint_b, &config.node_id, &config.world_id, None, 0)
        .expect("ingest");

    assert!(!engine.peer_heads.contains_key("node-a"));
    assert!(engine.quarantined_validators.contains("node-a"));
    assert_eq!(engine.misbehavior_evidence.len(), 1);
    let evidence = engine
        .misbehavior_evidence
        .values()
        .next()
        .expect("evidence");
    assert_eq!(evidence.kind, "commit_equivocation");
    assert_eq!(evidence.validator_id, "node-a");
    assert_eq!(evidence.height, 8);
    assert_eq!(evidence.slashable_stake, 60);
    assert_eq!(evidence.total_stake, 100);

    let snapshot = engine.snapshot_from_decision(&PosDecision {
        height: 8,
        slot: 8,
        epoch: 0,
        status: PosConsensusStatus::Pending,
        block_hash: "pending".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 0,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    });
    assert_eq!(snapshot.known_peer_heads, 0);
    assert_eq!(snapshot.quarantined_validators, vec!["node-a".to_string()]);
    assert_eq!(snapshot.misbehavior_evidence.len(), 1);
    assert!(snapshot.misbehavior_evidence[0].quarantined);
    assert!(!snapshot.misbehavior_evidence[0].evidence_hash.is_empty());
    assert_eq!(snapshot.slashing_intents.len(), 1);
    assert_eq!(
        snapshot.slashing_intents[0].governance_method,
        "apply_identity_penalty"
    );
    assert_eq!(snapshot.slashing_intents[0].target_agent_id, "node-a");
    assert_eq!(snapshot.slashing_intents[0].slash_stake, 60);
    assert_eq!(
        snapshot.slashing_intents[0].evidence_hash,
        snapshot.misbehavior_evidence[0].evidence_hash
    );
    assert!(!snapshot.slashing_intents[0].enforced);
    assert!(!snapshot.validator_set_hash.is_empty());
    assert!(!snapshot.validator_stake_root.is_empty());
    assert_eq!(snapshot.validator_stake_proofs.len(), 2);
    assert_eq!(
        snapshot.misbehavior_evidence[0].validator_stake_root,
        snapshot.validator_stake_root
    );
    assert!(snapshot
        .validator_stake_proofs
        .iter()
        .all(|proof| !proof.leaf_hash.is_empty()));
}
