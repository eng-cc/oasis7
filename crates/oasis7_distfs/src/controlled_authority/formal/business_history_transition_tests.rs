use super::*;
use crate::controlled_authority::replicated_protocol::ArtifactRole;

#[test]
fn historical_business_material_rebuilds_hash_bound_parent_delta() {
    let (fixture, initial_envelope) = super::super::business_tests::fixture();
    let mut coordinator = fixture.coordinator();
    let parent_proof = coordinator.submit(fixture.initial.clone()).unwrap();
    let parent = verify_formal_qualified(&parent_proof, &fixture.authority, None, None).unwrap();
    let (input, frozen_material) = fixture.frozen(&parent, 177);
    let executable = coordinator.freeze(input, frozen_material).unwrap();
    let mut candidate = fixture.candidate(&executable);
    let mut updates = vec![
        BusinessIndexLeafV1 {
            key_digest: metadata_key_digest(),
            kind: BusinessIndexLeafKindV1::CommitMetadata,
            canonical_value: vec![177],
        },
        FormalHistoricalCutV1::from_proof(&parent_proof)
            .unwrap()
            .leaf()
            .unwrap(),
    ];
    updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
    let delta = BusinessIndexDeltaV1::build(
        initial_envelope.delta.new,
        updates,
        &coordinator.endpoints_mut().0.store,
    )
    .unwrap();
    let envelope = super::super::business_tests::envelope(delta);
    let bind = |candidate: &mut FormalCandidateMaterialV1,
                envelope: &FormalBusinessResultEnvelopeV1| {
        super::super::business_tests::replace_result(
            &mut candidate.material,
            envelope.encode().unwrap(),
        );
        candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
        candidate.candidate.record.result_root = envelope.delta.new_root.clone();
        candidate.candidate.writer_signature_hex = signed(
            canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
            4,
        );
    };
    bind(&mut candidate, &envelope);
    let verify = |candidate: &FormalCandidateMaterialV1, reader: &super::super::super::Store| {
        super::super::super::business_material::verify_committed_business_material(
            &candidate.candidate.record,
            &candidate.material,
            candidate.frozen_material.as_ref(),
            reader,
        )
    };
    verify(&candidate, &coordinator.endpoints_mut().0.store).unwrap();
    // Signed canonical roots and material hashes remain internally valid. The
    // old root-only verifier could not distinguish these malformed deltas.
    for corruption in 0..3 {
        let mut bad_envelope = envelope.clone();
        match corruption {
            0 => bad_envelope.delta.base_root = h(212),
            1 => {
                bad_envelope.delta.node_writes.pop();
            }
            _ => {
                let cut = bad_envelope
                    .delta
                    .updates
                    .iter_mut()
                    .find(|leaf| leaf.kind == BusinessIndexLeafKindV1::HistoricalCut)
                    .unwrap();
                let mut wrong = FormalHistoricalCutV1::decode_leaf(
                    cut,
                    &parent.record().world,
                    parent.record().position,
                )
                .unwrap();
                wrong.parent_record.result_root = h(213);
                wrong.parent_qualified.decision.record = wrong.parent_record.clone();
                *cut = wrong.leaf().unwrap();
            }
        }
        let mut bad = candidate.clone();
        bind(&mut bad, &bad_envelope);
        verify_record_material(&bad.candidate.record, &bad.material).unwrap();
        assert_eq!(
            bad_envelope.delta.new_root,
            bad.candidate.record.result_root
        );
        assert_eq!(
            bad_envelope.delta.new.canonical_result_root().unwrap(),
            bad.candidate.record.result_root
        );
        assert!(verify(&bad, &coordinator.endpoints_mut().0.store).is_err());
        assert!(coordinator.endpoints_mut().0.prepare(bad).is_err());
    }
    assert!(candidate.material.roots.contains_key(&ArtifactRole::Result));
    coordinator.submit(candidate).unwrap();
}
