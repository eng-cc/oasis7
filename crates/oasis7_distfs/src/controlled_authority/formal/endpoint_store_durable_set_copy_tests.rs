use super::*;

#[test]
fn healthy_copy_enqueues_every_signed_business_delta_write() {
    use crate::controlled_authority::formal::business_index::{
        BusinessIndexDeltaV1, BusinessIndexLeafKindV1, BusinessIndexLeafV1,
        BusinessIndexNodeKindV1, BusinessIndexNodeReader, BusinessIndexNodeV1, BusinessIndexRootV1,
    };
    use crate::controlled_authority::formal::business_material::{
        FORMAL_BUSINESS_ENVELOPE_DOMAIN_V1, FormalBusinessResultEnvelopeV1,
    };
    use crate::controlled_authority::replicated_protocol::{
        ArtifactObject, ArtifactRole, ClosedRecord,
    };
    use std::collections::{BTreeMap, BTreeSet};

    struct Empty;
    impl BusinessIndexNodeReader for Empty {
        fn read_node(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(None)
        }
    }

    fn delta(prefix: u8, count: u8) -> BusinessIndexDeltaV1 {
        let updates = (1..=count)
            .map(|suffix| BusinessIndexLeafV1 {
                key_digest: format!("{}{:02x}", format!("{prefix:02x}").repeat(31), suffix),
                kind: BusinessIndexLeafKindV1::Outcome,
                canonical_value: vec![prefix, suffix],
            })
            .collect();
        BusinessIndexDeltaV1::build(BusinessIndexRootV1::empty().unwrap(), updates, &Empty).unwrap()
    }

    fn reachable_writes(delta: &BusinessIndexDeltaV1) -> BTreeSet<String> {
        let by_digest: BTreeMap<_, _> = delta
            .node_writes
            .iter()
            .map(|write| (write.digest.as_str(), write.bytes.as_slice()))
            .collect();
        let empty =
            crate::controlled_authority::formal::business_index::empty_subtree_hashes().unwrap();
        let mut found = BTreeSet::new();
        let mut pending = vec![(delta.new.node_root.clone(), 0usize)];
        while let Some((digest, depth)) = pending.pop() {
            assert!(depth <= BUSINESS_INDEX_DEPTH);
            if digest == empty[depth] || !found.insert(digest.clone()) {
                continue;
            }
            let bytes = by_digest.get(digest.as_str()).unwrap_or_else(|| {
                panic!("empty base delta omitted reachable synthesized node {digest}")
            });
            match BusinessIndexNodeV1::decode(bytes).unwrap().kind {
                BusinessIndexNodeKindV1::Branch {
                    depth: branch_depth,
                    left_hash,
                    right_hash,
                } => {
                    assert_eq!(usize::from(branch_depth), depth);
                    pending.push((left_hash, depth + 1));
                    pending.push((right_hash, depth + 1));
                }
                BusinessIndexNodeKindV1::Leaf(_) => assert_eq!(depth, BUSINESS_INDEX_DEPTH),
            }
        }
        found
    }

    let normal = delta(0x20, 8);
    let retained = delta(0x80, 8);
    for transition in [&normal, &retained] {
        let reachable = reachable_writes(transition);
        assert!(
            transition
                .node_writes
                .iter()
                .any(|write| !reachable.contains(&write.digest)),
            "multiple same-delta updates should leave signed intermediate writes outside the final root"
        );
    }

    #[derive(serde::Serialize)]
    struct Snapshot {
        state: State,
    }
    #[derive(serde::Serialize)]
    struct State {
        retained_history_root: Option<RetainedRoot>,
    }
    #[derive(serde::Serialize)]
    struct RetainedRoot {
        schema_version: u32,
        index_root: BusinessIndexRootV1,
        root_digest: String,
    }
    let retained_root_bytes = serde_cbor::to_vec(
        &serde_cbor::value::to_value((
            "oasis7.formal.retained-history.root.v1",
            1u32,
            &retained.new,
        ))
        .unwrap(),
    )
    .unwrap();
    let snapshot = serde_cbor::to_vec(&Snapshot {
        state: State {
            retained_history_root: Some(RetainedRoot {
                schema_version: 1,
                index_root: retained.new.clone(),
                root_digest: blake3::hash(&retained_root_bytes).to_hex().to_string(),
            }),
        },
    })
    .unwrap();
    let envelope = FormalBusinessResultEnvelopeV1 {
        schema_version: 1,
        domain: FORMAL_BUSINESS_ENVELOPE_DOMAIN_V1.into(),
        execution_result_cbor: serde_cbor::to_vec(&"fixture result").unwrap(),
        tree_descriptor_cbor: serde_cbor::to_vec(&normal.new).unwrap(),
        delta: normal.clone(),
        retained_history_delta: Some(retained.clone()),
    }
    .encode()
    .unwrap();
    let result_digest = blake3::hash(&envelope).to_hex().to_string();
    let snapshot_digest = blake3::hash(&snapshot).to_hex().to_string();
    let material = ClosedRecord {
        payload_digest: blake3::hash(b"input").to_hex().to_string(),
        before_state_root: snapshot_digest.clone(),
        after_state_root: snapshot_digest.clone(),
        roots: BTreeMap::from([
            (ArtifactRole::Result, result_digest.clone()),
            (ArtifactRole::Snapshot, snapshot_digest.clone()),
        ]),
        objects: vec![
            ArtifactObject {
                content_hash: result_digest,
                bytes: envelope,
                references: vec![],
            },
            ArtifactObject {
                content_hash: snapshot_digest,
                bytes: snapshot,
                references: vec![],
            },
        ],
    };
    let mut work = Vec::new();
    enqueue_material(&material, 2, true, &mut work, None).unwrap();
    let queued: BTreeMap<_, _> = work
        .into_iter()
        .filter_map(|visit| match visit {
            Visit::Raw(digest, bytes) => Some((digest, bytes)),
            _ => None,
        })
        .collect();
    for write in normal.node_writes.iter().chain(&retained.node_writes) {
        assert_eq!(queued.get(&write.digest), Some(&write.bytes));
    }
}

#[test]
fn healthy_copy_revalidates_same_object_wrong_history_head_and_index_depth() {
    let head = FormalHeadRefV1 {
        position: 3,
        record_id: "11".repeat(32),
    };
    let node = HistoryNode {
        head: head.clone(),
        request: "22".repeat(32),
        entry: "33".repeat(32),
        ancestors: vec![],
    };
    let bytes = serde_cbor::to_vec(&node).unwrap();
    let id = blake3::hash(&bytes).to_hex().to_string();
    let good = Visit::History(id.clone(), head.clone());
    let wrong_head = FormalHeadRefV1 {
        record_id: "44".repeat(32),
        ..head
    };
    let bad = Visit::History(id.clone(), wrong_head.clone());
    let mut verified_edges = std::collections::HashSet::new();
    assert!(verified_edges.insert(good.edge_digest().unwrap()));
    check_history_head(&decode(&bytes).unwrap(), &node.head).unwrap();
    // Identical object/hash but a later edge with a different recordID must not
    // hit the earlier verification marker. Its real structural check rejects.
    assert!(verified_edges.insert(bad.edge_digest().unwrap()));
    assert!(check_history_head(&decode(&bytes).unwrap(), &wrong_head).is_err());
    let index = Index::Fork {
        bit: 8,
        left: "55".repeat(32),
        right: "66".repeat(32),
    };
    let bytes = serde_cbor::to_vec(&index).unwrap();
    let id = blake3::hash(&bytes).to_hex().to_string();
    let good = Visit::Index(id.clone(), Some(7));
    let bad = Visit::Index(id, Some(8));
    assert_ne!(good.edge_digest().unwrap(), bad.edge_digest().unwrap());
    let Index::Fork { bit, .. } = decode(&bytes).unwrap() else {
        panic!("fork");
    };
    check_index_depth(bit, Some(7)).unwrap();
    assert!(check_index_depth(bit, Some(8)).is_err());
    assert!(check_index_depth(256, None).is_err());
    assert_ne!(
        Visit::Entry(id_for_entry(), None).edge_digest().unwrap(),
        Visit::Entry(id_for_entry(), Some(node.head))
            .edge_digest()
            .unwrap()
    );
    assert_ne!(
        Visit::Business("77".repeat(32), 1).edge_digest().unwrap(),
        Visit::Business("77".repeat(32), 2).edge_digest().unwrap()
    );
}
fn id_for_entry() -> String {
    "88".repeat(32)
}

#[test]
fn healthy_copy_empty_sentinel_keeps_each_edge_depth_context() {
    let table = empty_subtree_hashes().unwrap();
    for depth in 0..=BUSINESS_INDEX_DEPTH {
        assert!(is_empty_business_edge(&table, &table[depth], depth).unwrap());
        if depth < BUSINESS_INDEX_DEPTH {
            // Identical digest under another depth is not an empty sentinel;
            // the walker must continue into the actual node/depth validation.
            assert!(!is_empty_business_edge(&table, &table[depth], depth + 1).unwrap());
        }
    }
    assert!(is_empty_business_edge(&table, &table[0], BUSINESS_INDEX_DEPTH + 1).is_err());
}

#[cfg(unix)]
#[test]
fn healthy_copy_business_batches_are_bounded_and_logically_verified() {
    use std::os::unix::fs::MetadataExt;
    let directory = std::env::temp_dir().join(format!(
        "oasis7-copy-business-{}",
        fencing::fresh_challenge().unwrap()
    ));
    super::directory(&directory).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    let mut batch = CopyPackBatch::default();
    let mut writes = Vec::new();
    for index in 0..300 {
        let node = BusinessIndexNodeV1::branch(
            index % BUSINESS_INDEX_DEPTH,
            format!("{:064x}", index + 1),
            "11".repeat(32),
        )
        .unwrap();
        let bytes = node.encode().unwrap();
        let id = blake3::hash(&bytes).to_hex().to_string();
        batch.push(&directory, &id, &bytes).unwrap();
        assert!(batch.objects.len() <= COPY_PACK_OBJECTS);
        assert!(batch.bytes <= COPY_PACK_BYTES);
        writes.push((id, bytes));
    }
    batch.flush(&directory).unwrap();
    assert!(batch.objects.is_empty());
    assert_eq!(batch.bytes, 0);
    let markers = directory.join("readback-markers");
    super::directory(&markers).unwrap();
    let mut physical = std::collections::BTreeSet::new();
    for (id, expected) in &writes {
        let path = directory.join(id);
        let metadata = fs::metadata(&path).unwrap();
        physical.insert((metadata.dev(), metadata.ino()));
        assert_eq!(
            crate::controlled_authority::formal::business_pack::read_object_durable_once(
                &path, id, &markers
            )
            .unwrap()
            .unwrap(),
            *expected
        );
    }
    assert_eq!(
        physical.len(),
        2,
        "300 small Business nodes use two bounded packs"
    );
    fs::write(directory.join(&writes[0].0), b"corrupt shared pack").unwrap();
    assert!(
        crate::controlled_authority::formal::business_pack::read_object_durable_once(
            &directory.join(&writes[0].0),
            &writes[0].0,
            &markers
        )
        .is_err()
    );
    let mut retry = CopyPackBatch::default();
    retry.push(&directory, &writes[0].0, &writes[0].1).unwrap();
    assert!(
        retry.flush(&directory).is_err(),
        "existing immutable corruption cannot be overwritten"
    );
}
