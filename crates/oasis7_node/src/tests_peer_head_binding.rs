use super::*;

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
