use super::*;

fn signed_peer_commit(
    signer_seed: u8,
    world_id: &str,
    node_id: &str,
    height: u64,
    execution: Option<(&str, &str)>,
) -> GossipCommitMessage {
    let (private_hex, public_hex) = deterministic_keypair_hex(signer_seed);
    let signing_key = SigningKey::from_bytes(
        &hex::decode(private_hex)
            .expect("private decode")
            .try_into()
            .expect("private len"),
    );
    let signer = ConsensusMessageSigner::new(signing_key, public_hex).expect("signer");
    let mut commit = GossipCommitMessage {
        version: 1,
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        player_id: node_id.to_string(),
        height,
        slot: height,
        epoch: 0,
        block_hash: format!("block-{height}"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        committed_at_ms: height as i64 * 1_000,
        execution_block_hash: execution.map(|(block, _)| block.to_string()),
        execution_state_root: execution.map(|(_, state)| state.to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    sign_commit_message(&mut commit, &signer).expect("sign peer commit");
    commit
}

fn signed_fallback_engine_for(world_id: &str, node_id: &str, signer_seed: u8) -> PosNodeEngine {
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 40,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 35,
        },
        PosValidator {
            validator_id: "node-c".to_string(),
            stake: 25,
        },
    ];
    let config = NodeConfig::new(node_id, world_id, NodeRole::Storage)
        .expect("config")
        .with_pos_config(signed_pos_config_with_signer_seeds(
            validators,
            &[("node-a", 41), ("node-b", 42), ("node-c", 43)],
        ))
        .expect("pos config")
        .with_replication(signed_replication_config(
            temp_dir("validated-peer-fallback"),
            signer_seed,
        ));
    PosNodeEngine::new(&config).expect("engine")
}

fn signed_fallback_engine(world_id: &str) -> PosNodeEngine {
    signed_fallback_engine_for(world_id, "node-b", 42)
}

#[test]
fn non_executing_follower_reuses_matching_validated_peer_execution_binding() {
    let config = NodeConfig::new("node-b", "world-peer-exec-binding", NodeRole::Observer)
        .expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 7;
    engine.last_committed_block_hash = Some("block-7".to_string());
    engine.observe_peer_committed_head(
        "node-a",
        PeerCommittedHead {
            height: 7,
            block_hash: "block-7".to_string(),
            committed_at_ms: 7_000,
            observed_at_ms: 7_001,
            execution_block_hash: Some("exec-block-7".to_string()),
            execution_state_root: Some("exec-state-7".to_string()),
            action_root: empty_action_root(),
            public_key_hex: None,
            signature_hex: None,
        },
    );

    assert_eq!(
        engine
            .commit_execution_binding_for_height(7)
            .expect("validated peer binding"),
        (Some("exec-block-7"), Some("exec-state-7"))
    );
}

#[test]
fn peer_head_execution_binding_enrichment_is_not_equivocation() {
    let config = NodeConfig::new("node-b", "world-peer-head-enrichment", NodeRole::Observer)
        .expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let head = |execution_block_hash, execution_state_root| PeerCommittedHead {
        height: 7,
        block_hash: "block-7".to_string(),
        committed_at_ms: 7_000,
        observed_at_ms: 7_001,
        execution_block_hash,
        execution_state_root,
        action_root: empty_action_root(),
        public_key_hex: None,
        signature_hex: None,
    };
    engine.observe_peer_committed_head("node-a", head(None, None));
    engine.observe_peer_committed_head(
        "node-a",
        head(Some("exec-block-7".into()), Some("exec-state-7".into())),
    );

    assert!(engine.quarantined_validators.is_empty());
    assert!(engine.misbehavior_evidence.is_empty());
    assert_eq!(
        engine
            .peer_heads
            .get("node-a")
            .and_then(|head| head.execution_block_hash.as_deref()),
        Some("exec-block-7")
    );
}

#[test]
fn validated_peer_fallback_rewrites_identity_and_preserves_signed_hashes() {
    let world_id = "world-validated-peer-fallback-signature";
    let mut engine = signed_fallback_engine(world_id);
    let source = signed_peer_commit(
        41,
        world_id,
        "node-a",
        9,
        Some(("exec-block-9", "exec-state-9")),
    );
    verify_commit_message_signature(&source, true).expect("source signature");
    engine.observe_peer_commit_message(&source);
    engine.network_committed_height = 9;

    let (_, fallback) = engine
        .validated_peer_commit_head_message("node-b", world_id)
        .expect("fallback result")
        .expect("fallback commit");

    assert_eq!(fallback.node_id, "node-b");
    assert_eq!(fallback.player_id, "node-b");
    assert_eq!(fallback.block_hash, source.block_hash);
    assert_eq!(fallback.action_root, source.action_root);
    assert_eq!(fallback.execution_block_hash, source.execution_block_hash);
    assert_eq!(fallback.execution_state_root, source.execution_state_root);
    assert_ne!(fallback.signature_hex, source.signature_hex);
    verify_commit_message_signature(&fallback, true).expect("rewritten signature");

    let mut receiver = signed_fallback_engine_for(world_id, "node-c", 43);
    receiver.observe_peer_commit_message(&fallback);
    receiver.observe_peer_commit_message(&fallback);
    assert!(receiver.quarantined_validators.is_empty());
    assert!(receiver.misbehavior_evidence.is_empty());
}

#[test]
fn validated_peer_fallback_is_monotonic_and_preserves_rich_metadata() {
    let world_id = "world-validated-peer-fallback-monotonic";
    let mut engine = signed_fallback_engine(world_id);
    let rich = signed_peer_commit(
        41,
        world_id,
        "node-a",
        12,
        Some(("exec-block-12", "exec-state-12")),
    );
    let sparse = signed_peer_commit(41, world_id, "node-a", 12, None);
    let delayed_lower = signed_peer_commit(
        43,
        world_id,
        "node-c",
        11,
        Some(("exec-block-11", "exec-state-11")),
    );

    engine.observe_peer_commit_message(&rich);
    engine.observe_peer_commit_message(&sparse);
    engine.observe_peer_commit_message(&delayed_lower);
    engine.network_committed_height = 12;

    let (_, fallback) = engine
        .validated_peer_commit_head_message("node-b", world_id)
        .expect("fallback result")
        .expect("fallback commit");
    assert_eq!(fallback.height, 12);
    assert_eq!(fallback.block_hash, "block-12");
    assert_eq!(
        fallback.execution_block_hash.as_deref(),
        Some("exec-block-12")
    );
    assert_eq!(
        fallback.execution_state_root.as_deref(),
        Some("exec-state-12")
    );
    assert!(engine.quarantined_validators.is_empty());
    assert!(engine.misbehavior_evidence.is_empty());
}

#[test]
fn quarantined_validated_peer_source_is_invalidated_and_not_rebroadcast() {
    let world_id = "world-validated-peer-fallback-quarantine";
    let mut engine = signed_fallback_engine(world_id);
    let first = signed_peer_commit(
        41,
        world_id,
        "node-a",
        15,
        Some(("exec-block-15", "exec-state-15")),
    );
    let mut conflicting = first.clone();
    conflicting.block_hash = "conflicting-block-15".to_string();
    let (private_hex, public_hex) = deterministic_keypair_hex(41);
    let signing_key = SigningKey::from_bytes(
        &hex::decode(private_hex)
            .expect("private decode")
            .try_into()
            .expect("private len"),
    );
    let signer = ConsensusMessageSigner::new(signing_key, public_hex).expect("signer");
    sign_commit_message(&mut conflicting, &signer).expect("sign conflict");

    engine.observe_peer_commit_message(&first);
    engine.observe_peer_commit_message(&conflicting);
    engine.network_committed_height = 15;

    assert!(engine.quarantined_validators.contains("node-a"));
    assert_eq!(engine.misbehavior_evidence.len(), 1);
    assert!(engine.latest_validated_peer_commit.is_none());
    assert!(
        engine
            .validated_peer_commit_head_message("node-b", world_id)
            .expect("fallback result")
            .is_none(),
        "quarantined source must not be re-signed or rebroadcast"
    );
}
