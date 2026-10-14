use super::*;

#[cfg(unix)]
#[path = "formal_live_protected_process_tests.rs"]
pub(crate) mod protected_process_tests;

#[test]
fn local_formal_node_profile_disables_network_attestation_and_requires_execution() {
    let config = local_single_authority_node_config("formal-node-test", "world-test").unwrap();
    assert_eq!(config.role, NodeRole::Sequencer);
    assert_eq!(
        config.network_policy,
        NodeNetworkPolicy::for_runtime_role(NodeRole::Sequencer)
    );
    assert!(config.allow_local_proposals);
    assert!(!config.auto_attest_all_validators);
    assert!(config.require_execution_on_commit);
    assert!(!config.require_peer_execution_hashes);
    assert!(config.gossip.is_none());
    assert!(config.replication.is_none());
    assert!(config.feedback_p2p.is_none());
}

#[test]
fn business_reader_requires_dual_replica_content_addressed_bytes() {
    let bytes = b"verified business node".to_vec();
    let digest = hex::encode(blake3::hash(&bytes).as_bytes());
    assert_eq!(
        verify_dual_business_node(&digest, Some(bytes.clone()), Some(bytes.clone())).unwrap(),
        Some(bytes.clone())
    );
    assert_eq!(
        verify_dual_business_node(&digest, None, None).unwrap(),
        None
    );
    assert!(verify_dual_business_node(&digest, Some(bytes.clone()), None).is_err());
    assert!(
        verify_dual_business_node(&digest, Some(bytes.clone()), Some(b"different".to_vec()))
            .is_err()
    );
    assert!(
        verify_dual_business_node(
            &digest,
            Some(b"different".to_vec()),
            Some(b"different".to_vec())
        )
        .is_err()
    );
    assert!(validate_business_node_digest(&digest.to_uppercase()).is_err());
}

#[test]
fn business_reader_cache_stays_bounded_and_evicts_oldest_verified_node() {
    let mut cache = BoundedBusinessNodeCache::new(5, 2);
    cache.insert("first".into(), b"1234".to_vec());
    cache.insert("second".into(), b"56".to_vec());
    assert_eq!(cache.bytes, 2);
    assert_eq!(cache.get("first"), None);
    assert_eq!(cache.get("second"), Some(b"56".to_vec()));
    cache.insert("too-large".into(), b"123456".to_vec());
    assert_eq!(cache.bytes, 2);
    assert!(cache.get("too-large").is_none());
    assert!(cache.bytes <= cache.max_bytes);

    let mut count_bounded = BoundedBusinessNodeCache::new(10, 1);
    count_bounded.insert("first".into(), b"1".to_vec());
    count_bounded.insert("second".into(), b"2".to_vec());
    assert!(count_bounded.get("first").is_none());
    assert_eq!(count_bounded.get("second"), Some(b"2".to_vec()));
    assert_eq!(count_bounded.entries.len(), 1);
}

#[test]
fn cache_seed_accepts_only_canonical_content_addressed_nodes() {
    let bytes = b"verified publication node".to_vec();
    let digest = hex::encode(blake3::hash(&bytes).as_bytes());
    let mut cache = BoundedBusinessNodeCache::new(1024, 8);
    seed_verified_business_writes(
        &mut cache,
        vec![BusinessIndexNodeWriteV1 {
            digest: digest.clone(),
            bytes: bytes.clone(),
        }],
    )
    .unwrap();
    assert_eq!(cache.get(&digest), Some(bytes));

    let mut bad_cache = BoundedBusinessNodeCache::new(1024, 8);
    assert!(
        seed_verified_business_writes(
            &mut bad_cache,
            vec![BusinessIndexNodeWriteV1 {
                digest: digest.clone(),
                bytes: b"not the object".to_vec(),
            }],
        )
        .is_err()
    );
    assert!(bad_cache.entries.is_empty());
}
