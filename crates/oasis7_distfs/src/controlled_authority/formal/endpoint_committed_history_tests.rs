use super::*;
use crate::controlled_authority::formal::business_index::*;
#[test]
fn committed_historical_cut_uses_current_canonical_inclusion_and_keeps_main_anchor() {
    let (fixture, initial_envelope) = super::business_tests::fixture();
    let mut coordinator = fixture.coordinator();
    let initial = coordinator.submit(fixture.initial.clone()).unwrap();
    let body = fixture.authority.initial_body().cloned().unwrap();
    let certificate = SignedInitialActivationV1 {
        signature_hex: signed(initial_activation_signing_bytes(&body).unwrap(), 9),
        body,
    };
    let trust = InitialActivationTrustV1 {
        issuer_public_key: public(9),
        world: fixture.authority.definition().world.clone(),
        initial_state_root: certificate.body.initial_state_root.clone(),
        governing_manifest_digest: certificate.body.governing_manifest_digest.clone(),
        recovery_policy_digest: fixture.authority.policy().digest().unwrap(),
    };
    let origin = VerifiedFormalReadAnchorV1::from_initial(&certificate, &trust, &initial).unwrap();
    let mut anchor = origin.clone();
    let mut previous_proof = initial.clone();
    let mut tree = initial_envelope.delta.new;
    let mut proofs = vec![initial];
    for nonce in [121, 122] {
        let (input, material) = fixture.frozen(anchor.head(), nonce);
        let frozen = coordinator.freeze(input, material).unwrap();
        let mut candidate = fixture.candidate(&frozen);
        let mut updates = vec![
            BusinessIndexLeafV1 {
                key_digest: metadata_key_digest(),
                kind: BusinessIndexLeafKindV1::CommitMetadata,
                canonical_value: vec![nonce],
            },
            FormalHistoricalCutV1::from_proof(&previous_proof)
                .unwrap()
                .leaf()
                .unwrap(),
        ];
        updates.sort_by(|a, b| a.key_digest.cmp(&b.key_digest));
        let delta =
            BusinessIndexDeltaV1::build(tree, updates, &coordinator.endpoints_mut().0.store)
                .unwrap();
        tree = delta.new.clone();
        let envelope = super::business_tests::envelope(delta);
        super::business_tests::replace_result(&mut candidate.material, envelope.encode().unwrap());
        candidate.candidate.record.artifact_roots = candidate.material.roots.clone();
        candidate.candidate.record.result_root = envelope.delta.new_root;
        candidate.candidate.writer_signature_hex = signed(
            canonical_candidate_signing_bytes(&candidate.candidate.record).unwrap(),
            4,
        );
        let proof = coordinator.submit(candidate).unwrap();
        let commit =
            verify_formal_qualified(&proof, &fixture.authority, Some(anchor.head()), None).unwrap();
        anchor = anchor
            .after_verified_commit(fixture.authority.clone(), commit)
            .unwrap();
        previous_proof = proof.clone();
        proofs.push(proof);
    }
    let current_head = anchor.head().head();
    for position in [1, 2] {
        let key =
            formal_historical_cut_key(&fixture.authority.definition().world, position).unwrap();
        let inclusion = tree
            .prove(&key, &coordinator.endpoints_mut().0.store)
            .unwrap();
        let cut =
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &inclusion, position).unwrap();
        assert_eq!(
            cut.commit().record(),
            &proofs[position as usize - 1].decision.record
        );
        assert_eq!(cut.proof(), &proofs[position as usize - 1]);
        assert_eq!(cut.delegating_head(), &current_head);
        let FormalQueryOutcomeV1::Recorded(state) = coordinator
            .endpoints_mut()
            .0
            .query(&cut.commit().record().request_digest)
            .unwrap()
        else {
            panic!("published exact old query");
        };
        let restored = verify_committed_historical_material(
            &cut,
            (*state).clone(),
            &coordinator.endpoints_mut().0.store,
        )
        .unwrap();
        assert_eq!(restored.commit().record(), cut.commit().record());
        let mut corrupt = (*state).clone();
        if let FormalRequestStateV1::Qualified { proof, .. } = &mut corrupt {
            proof.primary_complete.signature_hex = h(1);
        }
        assert!(
            verify_committed_historical_material(
                &cut,
                corrupt,
                &coordinator.endpoints_mut().0.store
            )
            .is_err()
        );
        let mut corrupt = *state;
        if let FormalRequestStateV1::Qualified { candidate, .. } = &mut corrupt {
            candidate.material.objects[0].bytes.push(0);
        }
        assert!(
            verify_committed_historical_material(
                &cut,
                corrupt,
                &coordinator.endpoints_mut().0.store
            )
            .is_err()
        );

        assert_eq!(anchor.head().head(), current_head);
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&origin, &tree, &inclusion, position).is_err()
        );
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &inclusion, position + 1)
                .is_err()
        );
        let mut corrupt = inclusion.clone();
        corrupt.siblings[0] = h(203);
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &corrupt, position).is_err()
        );
        let mut corrupt = inclusion.clone();
        corrupt.leaf.canonical_value.push(0);
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &tree, &corrupt, position).is_err()
        );
        let mut count = tree.clone();
        count.leaf_count += 1;
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(&anchor, &count, &inclusion, position)
                .is_err()
        );
    }
}

#[path = "business_history_transition_tests.rs"]
mod transition_tests;
