use super::*;
use std::cell::Cell;

#[derive(Default)]
struct Nodes {
    bytes: BTreeMap<String, Vec<u8>>,
    reads: Cell<usize>,
    prefetches: Cell<usize>,
}
impl BusinessIndexNodeReader for Nodes {
    fn prefetch_path(&self, _: &BusinessIndexRootV1, _: &str) -> Result<(), String> {
        self.prefetches.set(self.prefetches.get() + 1);
        Ok(())
    }
    fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.bytes.get(digest).cloned())
    }
}
fn fixture() -> (BusinessIndexRootV1, String, Nodes) {
    let key = hex::encode([7; 32]);
    let mut updates = vec![
        BusinessIndexLeafV1 {
            key_digest: metadata_key_digest(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: vec![1],
        },
        BusinessIndexLeafV1 {
            key_digest: key.clone(),
            kind: BusinessIndexLeafKindV1::Outcome,
            canonical_value: vec![2],
        },
    ];
    updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
    let mut nodes = Nodes::default();
    let delta = BusinessIndexDeltaV1::build(BusinessIndexRootV1::empty().unwrap(), updates, &nodes)
        .unwrap();
    nodes.bytes.extend(
        delta
            .node_writes
            .into_iter()
            .map(|node| (node.digest, node.bytes)),
    );
    (delta.new, key, nodes)
}

#[test]
fn business_path_bundle_reuses_canonical_present_absent_walk_without_recursive_prefetch() {
    let (root, key, nodes) = fixture();
    let path = root.path_nodes(&key, &nodes).unwrap();
    assert_eq!(path.nodes.len(), BUSINESS_INDEX_DEPTH + 1);
    assert_eq!(nodes.reads.get(), BUSINESS_INDEX_DEPTH + 1);
    assert_eq!(
        nodes.prefetches.get(),
        0,
        "server collection must not recursively prefetch"
    );
    assert_eq!(path.verify(&root, &key).unwrap().unwrap().key_digest, key);
    assert_eq!(
        nodes.reads.get(),
        BUSINESS_INDEX_DEPTH + 1,
        "verification uses only returned bytes"
    );
    let absent = hex::encode([8; 32]);
    let absence = root.path_nodes(&absent, &nodes).unwrap();
    assert!(absence.nodes.len() < BUSINESS_INDEX_DEPTH + 1);
    assert!(absence.verify(&root, &absent).unwrap().is_none());
    assert!(
        BusinessIndexPathV1 { nodes: vec![] }
            .verify(&root, &absent)
            .is_err()
    );
    assert!(
        BusinessIndexPathV1 { nodes: vec![] }
            .verify(&BusinessIndexRootV1::empty().unwrap(), &absent)
            .unwrap()
            .is_none()
    );
    root.get(&key, &nodes).unwrap();
    root.prove(&key, &nodes).unwrap();
    assert_eq!(
        nodes.prefetches.get(),
        2,
        "both public walks invoke the optional hook"
    );
}

#[test]
fn business_path_bundle_rejects_wrong_bindings_missing_extra_and_late_corruption() {
    let (root, key, nodes) = fixture();
    let path = root.path_nodes(&key, &nodes).unwrap();
    assert!(
        path.verify(&BusinessIndexRootV1::empty().unwrap(), &key)
            .is_err()
    );
    assert!(path.verify(&root, &hex::encode([8; 32])).is_err());
    let mut changed = path.clone();
    changed.nodes.last_mut().unwrap().bytes[0] ^= 1;
    assert!(changed.verify(&root, &key).is_err());
    let mut missing = path.clone();
    missing.nodes.pop();
    assert!(missing.verify(&root, &key).is_err());
    let mut duplicate = path.clone();
    duplicate.nodes[1] = duplicate.nodes[0].clone();
    assert!(duplicate.verify(&root, &key).is_err());
    let absent = hex::encode([8; 32]);
    let mut extra = root.path_nodes(&absent, &nodes).unwrap();
    extra.nodes.push(path.nodes.last().unwrap().clone());
    assert!(extra.verify(&root, &absent).is_err());
    let oversized = BusinessIndexPathV1 {
        nodes: vec![path.nodes[0].clone(); BUSINESS_INDEX_DEPTH + 2],
    };
    assert!(oversized.verify(&root, &key).is_err());
    assert!(
        serde_cbor::from_slice::<BusinessIndexPathV1>(&serde_cbor::to_vec(&oversized).unwrap())
            .is_err()
    );
}
