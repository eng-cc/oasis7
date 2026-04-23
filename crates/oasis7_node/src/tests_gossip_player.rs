use super::*;

fn empty_action_root() -> String {
    compute_consensus_action_root(&[]).expect("empty action root")
}

fn two_validators() -> Vec<PosValidator> {
    vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ]
}

#[test]
fn pos_engine_applies_gossiped_proposal_and_attestation() {
    let validators = two_validators();
    let config = NodeConfig::new("node-b", "world-gossip-proposal", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(validators)
        .expect("validators")
        .with_auto_attest_all_validators(false);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: format!("{}:h1:s0:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    engine
        .ingest_proposal_message(&config.world_id, &proposal, 0)
        .expect("ingest proposal");
    assert_eq!(
        engine.pending.as_ref().map(|pending| pending.opened_at_ms),
        Some(1_000)
    );

    let attestation = GossipAttestationMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-b".to_string(),
        player_id: "node-b".to_string(),
        validator_id: "node-b".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: proposal.block_hash.clone(),
        approve: true,
        source_epoch: 0,
        target_epoch: 0,
        voted_at_ms: 1_001,
        reason: Some("gossip attestation".to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    engine
        .ingest_attestation_message(&config.world_id, &attestation, 0)
        .expect("ingest attestation");

    let snapshot = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_002,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("tick");
    assert_eq!(snapshot.consensus_snapshot.committed_height, 1);
    assert_eq!(
        snapshot.consensus_snapshot.last_status,
        Some(PosConsensusStatus::Committed)
    );
}

#[test]
fn pos_engine_ignores_gossiped_proposal_when_player_binding_mismatches() {
    let validators = two_validators();
    let config = NodeConfig::new("node-b", "world-gossip-player-mismatch", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(validators)
        .expect("validators")
        .with_auto_attest_all_validators(false);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "other-player".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: format!("{}:h1:s0:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    engine
        .ingest_proposal_message(&config.world_id, &proposal, 0)
        .expect("ingest proposal");
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_rejects_gossiped_proposal_with_future_slot_window() {
    let config = NodeConfig::new("node-b", "world-gossip-future-slot", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(two_validators())
        .expect("validators");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 3,
        epoch: 0,
        block_hash: format!("{}:h1:s3:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };

    engine
        .ingest_proposal_message(&config.world_id, &proposal, 2)
        .expect("ingest proposal");
    assert!(engine.pending.is_none());
    assert_eq!(engine.inbound_rejected_proposal_future_slot, 1);
    assert!(engine
        .last_inbound_timing_reject_reason
        .as_deref()
        .unwrap_or_default()
        .contains("future"));
}

#[test]
fn pos_engine_rejects_gossiped_proposal_with_stale_slot_window() {
    let pos_config = NodePosConfig::ethereum_like(two_validators())
        .with_max_past_slot_lag(1)
        .expect("lag config");
    let config = NodeConfig::new("node-b", "world-gossip-stale-slot", NodeRole::Observer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 2,
        epoch: 0,
        block_hash: format!("{}:h1:s2:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };

    engine
        .ingest_proposal_message(&config.world_id, &proposal, 5)
        .expect("ingest proposal");
    assert!(engine.pending.is_none());
    assert_eq!(engine.inbound_rejected_proposal_stale_slot, 1);
    assert!(engine
        .last_inbound_timing_reject_reason
        .as_deref()
        .unwrap_or_default()
        .contains("stale"));
}

#[test]
fn pos_engine_rejects_gossiped_attestation_with_slot_window_and_epoch_mismatch() {
    let pos_config = NodePosConfig::ethereum_like(two_validators())
        .with_max_past_slot_lag(1)
        .expect("lag config");
    let config = NodeConfig::new(
        "node-b",
        "world-gossip-attestation-window",
        NodeRole::Observer,
    )
    .expect("config")
    .with_pos_config(pos_config)
    .expect("pos config")
    .with_auto_attest_all_validators(false);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 2,
        epoch: 0,
        block_hash: format!("{}:h1:s2:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    engine
        .ingest_proposal_message(&config.world_id, &proposal, 2)
        .expect("ingest proposal");
    assert!(engine.pending.is_some());

    let future_attestation = GossipAttestationMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-b".to_string(),
        player_id: "node-b".to_string(),
        validator_id: "node-b".to_string(),
        height: 1,
        slot: 3,
        epoch: 0,
        block_hash: proposal.block_hash.clone(),
        approve: true,
        source_epoch: 0,
        target_epoch: 0,
        voted_at_ms: 1_001,
        reason: Some("future attestation".to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    engine
        .ingest_attestation_message(&config.world_id, &future_attestation, 2)
        .expect("ingest future attestation");
    assert_eq!(engine.inbound_rejected_attestation_future_slot, 1);

    let stale_attestation = GossipAttestationMessage {
        slot: 2,
        voted_at_ms: 1_002,
        reason: Some("stale attestation".to_string()),
        ..future_attestation.clone()
    };
    engine
        .ingest_attestation_message(&config.world_id, &stale_attestation, 5)
        .expect("ingest stale attestation");
    assert_eq!(engine.inbound_rejected_attestation_stale_slot, 1);

    let wrong_target_epoch = GossipAttestationMessage {
        slot: 2,
        epoch: 0,
        target_epoch: 9,
        voted_at_ms: 1_003,
        reason: Some("wrong target epoch".to_string()),
        ..future_attestation
    };
    engine
        .ingest_attestation_message(&config.world_id, &wrong_target_epoch, 2)
        .expect("ingest wrong target epoch");
    assert_eq!(engine.inbound_rejected_attestation_epoch_mismatch, 1);
    let pending = engine.pending.as_ref().expect("pending exists");
    assert!(!pending.attestations.contains_key("node-b"));
}
