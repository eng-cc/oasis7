use super::*;
use crate::controlled_authority::formal::business_index::{
    BusinessIndexDeltaV1, BusinessIndexLeafKindV1, BusinessIndexLeafV1, BusinessIndexRootV1,
};
use std::cell::{Cell, RefCell};

#[derive(Default)]
struct MemoryNodes {
    nodes: RefCell<BTreeMap<String, Vec<u8>>>,
    reads: Cell<usize>,
}

impl MemoryNodes {
    fn insert(&self, digest: String, bytes: Vec<u8>) {
        self.nodes.borrow_mut().insert(digest, bytes);
    }

    fn read_count(&self) -> usize {
        self.reads.get()
    }
}

impl BusinessIndexNodeReader for MemoryNodes {
    fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.nodes.borrow().get(digest).cloned())
    }
}

struct EmptyNodes;
impl BusinessIndexNodeReader for EmptyNodes {
    fn read_node(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
}

fn single_leaf_tree() -> (BusinessIndexRootV1, String, MemoryNodes) {
    let key = blake3::hash(b"candidate-local-memo-key")
        .to_hex()
        .to_string();
    let delta = BusinessIndexDeltaV1::build(
        BusinessIndexRootV1::empty().unwrap(),
        vec![BusinessIndexLeafV1 {
            key_digest: key.clone(),
            kind: BusinessIndexLeafKindV1::Outcome,
            canonical_value: b"verified immutable value".to_vec(),
        }],
        &EmptyNodes,
    )
    .unwrap();
    let nodes = MemoryNodes::default();
    for write in delta.node_writes {
        nodes.insert(write.digest, write.bytes);
    }
    (delta.new, key, nodes)
}

#[test]
fn candidate_memo_reuses_only_bounded_positive_node_bytes() {
    let (root, key, nodes) = single_leaf_tree();
    let memo = CandidateNodeMemo::new(&nodes);

    assert!(root.get(&key, &memo).unwrap().is_some());
    let first_reads = nodes.read_count();
    assert!(first_reads > 0);
    assert!(root.get(&key, &memo).unwrap().is_some());
    assert_eq!(nodes.read_count(), first_reads);

    let digest = blake3::hash(&vec![7; CANDIDATE_NODE_MEMO_MAX_BYTES + 1])
        .to_hex()
        .to_string();
    let oversized = vec![7; CANDIDATE_NODE_MEMO_MAX_BYTES + 1];
    nodes.insert(digest.clone(), oversized.clone());
    assert_eq!(memo.read_node(&digest).unwrap(), Some(oversized.clone()));
    assert_eq!(memo.read_node(&digest).unwrap(), Some(oversized));
    assert_eq!(nodes.read_count(), first_reads + 2);
}

#[test]
fn candidate_memo_does_not_cache_absence_or_corrupt_source_bytes() {
    let nodes = MemoryNodes::default();
    let good = b"valid immutable content".to_vec();
    let digest = blake3::hash(&good).to_hex().to_string();
    let memo = CandidateNodeMemo::new(&nodes);

    assert_eq!(memo.read_node(&digest).unwrap(), None);
    nodes.insert(digest.clone(), vec![0; good.len()]);
    assert!(memo.read_node(&digest).is_err());
    nodes.insert(digest.clone(), good.clone());
    assert_eq!(memo.read_node(&digest).unwrap(), Some(good));
    assert_eq!(nodes.read_count(), 3);
}

#[test]
fn a_new_candidate_memo_rechecks_mutated_source_cas() {
    let (root, key, nodes) = single_leaf_tree();
    assert!(
        root.get(&key, &CandidateNodeMemo::new(&nodes))
            .unwrap()
            .is_some()
    );

    let root_digest = root.node_root.clone();
    nodes.insert(root_digest, vec![0; 80]);
    assert!(root.get(&key, &CandidateNodeMemo::new(&nodes)).is_err());
}
