//! Profile 2 cut enforcement against the endpoint's held/authenticated parent.
use super::*;
use crate::controlled_authority::formal::business_history::FormalHistoricalCutV1;
use crate::controlled_authority::formal::business_index::*;
#[test]
fn historical_cut_is_exact_native_parent_and_old_position_leaf_is_immutable() {
    let (fixture, initial_envelope) = super::business_tests::fixture();
    let mut coordinator = fixture.coordinator();
    let proof = coordinator.submit(fixture.initial.clone()).unwrap();
    let parent = verify_formal_qualified(&proof, &fixture.authority, None, None).unwrap();
    let (input, material) = fixture.frozen(&parent, 109);
    let executable = coordinator.freeze(input, material).unwrap();
    let plain = fixture.candidate(&executable);
    let canonical_cut = FormalHistoricalCutV1::from_proof(&proof)
        .unwrap()
        .leaf()
        .unwrap();
    let decoded =
        FormalHistoricalCutV1::decode_leaf(&canonical_cut, &parent.record().world, 1).unwrap();
    assert_eq!(decoded.parent_qualified, proof);
    let make = |cuts: Vec<BusinessIndexLeafV1>, endpoint: &FileFormalEndpoint| {
        let mut updates = vec![BusinessIndexLeafV1 {
            key_digest: metadata_key_digest(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: b"next metadata".to_vec(),
        }];
        updates.extend(cuts);
        updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
        let delta = BusinessIndexDeltaV1::build(
            initial_envelope.delta.new.clone(),
            updates,
            &endpoint.store,
        )
        .unwrap();
        let envelope = super::business_tests::envelope(delta);
        let mut candidate = plain.clone();
        super::business_tests::replace_result(&mut candidate.material, envelope.encode().unwrap());
        candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
        candidate.candidate.record.result_root = envelope.delta.new_root;
        candidate.candidate.writer_signature_hex = signed(
            canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
            4,
        );
        candidate
    };
    let (endpoint, _) = coordinator.endpoints_mut();
    let missing = make(vec![], endpoint);
    assert!(endpoint.prepare(missing).is_err());
    let mut altered = canonical_cut.clone();
    altered.canonical_value.push(0);
    let invalid = make(vec![altered], endpoint);
    assert!(endpoint.prepare(invalid).is_err());
    let mut shadow = decoded.clone();
    shadow.parent_qualified.primary_complete.signature_hex = "00".repeat(64);
    let invalid = make(vec![shadow.leaf().unwrap()], endpoint);
    assert!(endpoint.prepare(invalid).is_err());
    let mut second = canonical_cut.clone();
    second.key_digest = h(208);
    let invalid = make(vec![canonical_cut.clone(), second], endpoint);
    assert!(endpoint.prepare(invalid).is_err());
    let valid = make(vec![canonical_cut.clone()], endpoint);
    let result_hash = &valid.material.roots[&ArtifactRole::Result];
    let committed_envelope = FormalBusinessResultEnvelopeV1::decode(
        &valid
            .material
            .objects
            .iter()
            .find(|object| &object.content_hash == result_hash)
            .unwrap()
            .bytes,
    )
    .unwrap();
    coordinator.submit(valid).unwrap();
    let (endpoint, _) = coordinator.endpoints_mut();
    let mut changed = canonical_cut;
    changed.canonical_value.push(0);
    let mut updates = vec![
        BusinessIndexLeafV1 {
            key_digest: metadata_key_digest(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: b"third metadata".to_vec(),
        },
        changed,
    ];
    updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
    assert!(
        BusinessIndexDeltaV1::build(committed_envelope.delta.new, updates, &endpoint.store)
            .is_err()
    );
}
