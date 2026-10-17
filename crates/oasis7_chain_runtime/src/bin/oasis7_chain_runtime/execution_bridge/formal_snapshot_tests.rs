use super::*;
use oasis7::runtime::{World, blake3_hex};
fn material(root: String, objects: &BTreeMap<String, ArtifactObject>) -> ClosedRecord {
    ClosedRecord {
        payload_digest: "00".repeat(32),
        before_state_root: "00".repeat(32),
        after_state_root: "00".repeat(32),
        roots: BTreeMap::from([(ArtifactRole::Snapshot, root)]),
        objects: objects.values().cloned().collect(),
    }
}
#[test]
fn sparse_snapshot_history_uses_bounded_checkpoints_and_exact_expanded_state_roots() {
    let mut before = World::new().snapshot();
    before
        .module_artifact_bytes
        .insert("unchanged-module".into(), vec![7; 65536]);
    let mut objects = BTreeMap::new();
    let raw = to_cbor(&before).unwrap();
    let root = insert(&mut objects, raw, vec![]);
    let mut frozen = material(root, &objects);
    for step in 1..=40 {
        let mut after = before.clone();
        after.next_action_id += 1;
        after.module_tick_schedule.insert("instance-a".into(), step);
        let raw_after = to_cbor(&after).unwrap();
        let state_root = blake3_hex(&raw_after);
        let mut inherited = BTreeMap::new();
        let base_ref = inherit(&mut inherited, &frozen, &to_cbor(&before).unwrap()).unwrap();
        let current = material(base_ref, &inherited);
        let snapshot_ref =
            append(&mut inherited, &current, &before, &after, raw_after.clone()).unwrap();
        frozen = material(snapshot_ref.clone(), &inherited);
        assert_eq!(bytes(&frozen).unwrap(), raw_after);
        assert_eq!(blake3_hex(&bytes(&frozen).unwrap()), state_root);
        let (_, depth) =
            oasis7::runtime::resolve_snapshot_artifact_with_depth(&snapshot_ref, &|hash| {
                capture::object(&frozen, hash).map(<[u8]>::to_vec)
            })
            .unwrap();
        assert_eq!(depth, step as usize % (MAX_SNAPSHOT_DELTA_DEPTH + 1));
        if depth > 0 {
            assert_ne!(snapshot_ref, state_root);
        }
        assert!(inherited.len() <= MAX_SNAPSHOT_DELTA_DEPTH + 2);
        before = after;
    }
}
#[test]
fn snapshot_closure_rejects_wrong_prestate_missing_base_and_declared_edge_mismatch() {
    let before = World::new().snapshot();
    let mut after = before.clone();
    after.next_action_id += 1;
    let mut objects = BTreeMap::new();
    let root = insert(&mut objects, to_cbor(&before).unwrap(), vec![]);
    let frozen = material(root.clone(), &objects);
    assert_eq!(
        append(
            &mut objects.clone(),
            &frozen,
            &before,
            &before,
            to_cbor(&before).unwrap()
        )
        .unwrap(),
        root
    );
    assert!(inherit(&mut BTreeMap::new(), &frozen, &to_cbor(&after).unwrap()).is_err());
    let artifact = SnapshotArtifactV1::new(
        root.clone(),
        1,
        SnapshotDeltaV1::between(&before, &after).unwrap(),
    )
    .unwrap();
    let encoded = artifact.encode().unwrap();
    let artifact_ref = insert(&mut objects, encoded.clone(), vec![root.clone()]);
    let complete = material(artifact_ref.clone(), &objects);
    assert_eq!(complete.snapshot_bytes().unwrap(), to_cbor(&after).unwrap());
    let mut bad = complete.clone();
    bad.objects.retain(|object| object.content_hash != root);
    assert!(bad.snapshot_bytes().is_err());
    let mut bad = complete;
    bad.objects
        .iter_mut()
        .find(|object| object.content_hash == artifact_ref)
        .unwrap()
        .references
        .clear();
    assert!(bad.snapshot_bytes().is_err());
}
