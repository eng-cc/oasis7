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

#[test]
fn business_path_requires_both_complete_root_bound_paths_before_cache_install() {
    use oasis7_distfs::controlled_authority::formal::business_index::{
        BusinessIndexDeltaV1, BusinessIndexLeafKindV1, BusinessIndexLeafV1, metadata_key_digest,
    };
    struct Nodes(HashMap<String, Vec<u8>>);
    impl BusinessIndexNodeReader for Nodes {
        fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(self.0.get(digest).cloned())
        }
    }
    let key = metadata_key_digest();
    let delta = BusinessIndexDeltaV1::build(
        BusinessIndexRootV1::empty().unwrap(),
        vec![BusinessIndexLeafV1 {
            key_digest: key.clone(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: vec![1],
        }],
        &Nodes(HashMap::new()),
    )
    .unwrap();
    let nodes = Nodes(
        delta
            .node_writes
            .iter()
            .map(|node| (node.digest.clone(), node.bytes.clone()))
            .collect(),
    );
    let path = delta.new.path_nodes(&key, &nodes).unwrap();
    assert_eq!(path.nodes.len(), 257);
    assert_eq!(
        verify_dual_business_path(&delta.new, &key, path.clone(), path.clone()).unwrap(),
        path.nodes
    );
    let mut corrupt = path.clone();
    corrupt.nodes.last_mut().unwrap().bytes.push(0);
    assert!(verify_dual_business_path(&delta.new, &key, path.clone(), corrupt).is_err());
    let mut missing = path.clone();
    missing.nodes.pop();
    assert!(verify_dual_business_path(&delta.new, &key, path.clone(), missing).is_err());
    assert!(
        verify_dual_business_path(
            &BusinessIndexRootV1::empty().unwrap(),
            &key,
            path.clone(),
            path.clone()
        )
        .is_err()
    );
    let absent = hex::encode([7u8; 32]);
    let absence = delta.new.path_nodes(&absent, &nodes).unwrap();
    assert!(absence.verify(&delta.new, &absent).unwrap().is_none());
    assert!(verify_dual_business_path(&delta.new, &absent, absence.clone(), absence).is_ok());
    assert!(verify_dual_business_path(&delta.new, &absent, path.clone(), path).is_err());
}
