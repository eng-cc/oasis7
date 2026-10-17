use super::*;
use crate::controlled_authority::replicated_protocol::{
    ArtifactObject, ArtifactRole, ClosedRecord,
};
use crate::snapshot_delta::{SnapshotArtifactV1, SnapshotDeltaV1};
use std::collections::BTreeMap;

fn delta_material() -> ClosedRecord {
    let base = BTreeMap::from([("cursor", 1u64)]);
    let target = BTreeMap::from([("cursor", 2u64)]);
    let base_bytes = serde_cbor::to_vec(&base).unwrap();
    let target_bytes = serde_cbor::to_vec(&target).unwrap();
    let base_ref = blake3::hash(&base_bytes).to_hex().to_string();
    let artifact = SnapshotArtifactV1::new(
        base_ref.clone(),
        1,
        SnapshotDeltaV1::between(&base, &target).unwrap(),
    )
    .unwrap();
    let bytes = artifact.encode().unwrap();
    let root = blake3::hash(&bytes).to_hex().to_string();
    let mut objects = vec![
        ArtifactObject {
            content_hash: root.clone(),
            bytes,
            references: vec![base_ref.clone()],
        },
        ArtifactObject {
            content_hash: base_ref.clone(),
            bytes: base_bytes,
            references: vec![],
        },
    ];
    objects.sort_by(|a, b| a.content_hash.cmp(&b.content_hash));
    let mut roots = fixtures::initial().3.record().artifact_roots.clone();
    for hash in roots.values_mut() {
        *hash = root.clone();
    }
    ClosedRecord {
        payload_digest: root,
        before_state_root: base_ref,
        after_state_root: blake3::hash(&target_bytes).to_hex().to_string(),
        roots,
        objects,
    }
}
#[test]
fn snapshot_delta_records_have_explicit_protocol_versions_and_expanded_state_roots() {
    let material = delta_material();
    let mut record = fixtures::initial().3.record().clone();
    record.kind = FormalRecordKindV1::Execution;
    record.schema_version = 3;
    record.position = 2;
    record.frozen_reservation_id = Some(h(22));
    record.payload_digest = material.payload_digest.clone();
    record.before_state_root = material.before_state_root.clone();
    record.after_state_root = material.after_state_root.clone();
    record.artifact_roots = material.roots.clone();
    verify_record_material(&record, &material).unwrap();
    let mut legacy = record.clone();
    legacy.schema_version = 1;
    assert!(verify_record_material(&legacy, &material).is_err());
    let mut wrong = record.clone();
    wrong.after_state_root = h(90);
    let mut wrong_material = material.clone();
    wrong_material.after_state_root = h(90);
    assert!(verify_record_material(&wrong, &wrong_material).is_err());
    let mut transition = record;
    transition.schema_version = 4;
    transition.durable_set_transition_digest = Some(h(91));
    verify_record_material(&transition, &material).unwrap();
}
#[test]
fn frozen_v2_resolves_exact_prestate_and_legacy_v1_never_reinterprets_delta() {
    let (_, authority, _, cut) = fixtures::initial();
    let mut body = fixtures::frozen(&authority, &cut, None).reservation.body;
    let mut material = delta_material();
    material.before_state_root = material.after_state_root.clone();
    body.schema_version = 2;
    body.pre_state_root = material.after_state_root.clone();
    body.payload_digest = material.payload_digest.clone();
    body.runtime_manifest_digest = material.roots[&ArtifactRole::ExecutionManifest].clone();
    body.frozen_material_digest = frozen_material_digest(&material).unwrap();
    verify_frozen_material(&body, &material).unwrap();
    body.schema_version = 1;
    assert!(verify_frozen_material(&body, &material).is_err());
}
