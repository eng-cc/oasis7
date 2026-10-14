use super::{fixtures::*, *};
use crate::controlled_authority::formal::business_index::*;

#[test]
fn committed_historical_recovery_requires_exact_independent_permission_and_predecessor() {
    for branch in [
        FormalCompletionBranchV1::ReplayFrozen,
        FormalCompletionBranchV1::CompletePrepared,
    ] {
        let (intent, original, predecessor, original_frozen, record) =
            recovery_fixture_with_profile(branch, None, 10, 2);
        let recovery = verified_recovery(&intent, &original);
        let completed = proof(
            record,
            recovery.successor(),
            branch,
            Some(recovery.certificate_digest().into()),
            Some(original_frozen),
            if branch == FormalCompletionBranchV1::ReplayFrozen {
                10
            } else {
                4
            },
            11,
            12,
        );
        let committed =
            verify_formal_qualified(&completed, &original, Some(&predecessor), Some(&recovery))
                .unwrap();
        let leaf = FormalHistoricalCutV1::from_proof(&completed)
            .unwrap()
            .leaf()
            .unwrap();
        let key = hex::decode(&leaf.key_digest).unwrap();
        let siblings: Vec<_> = (1..=256)
            .rev()
            .map(|depth| BusinessIndexRootV1::empty_subtree_hash(depth).unwrap())
            .collect();
        let mut node = BusinessIndexNodeV1::leaf(leaf.clone())
            .unwrap()
            .digest()
            .unwrap();
        for (offset, sibling) in siblings.iter().enumerate() {
            let depth = 255 - offset;
            node = if (key[depth / 8] >> (7 - depth % 8)) & 1 == 1 {
                BusinessIndexNodeV1::branch(depth, sibling.clone(), node)
            } else {
                BusinessIndexNodeV1::branch(depth, node, sibling.clone())
            }
            .unwrap()
            .digest()
            .unwrap();
        }
        let descriptor = BusinessIndexRootV1 {
            node_root: node,
            leaf_count: 1,
        };
        let inclusion = BusinessIndexInclusionV1 {
            leaf,
            siblings,
            leaf_count: 1,
        };
        let next_frozen = frozen(recovery.successor(), &committed, None);
        let mut next = execution(recovery.successor(), &next_frozen);
        next.result_root = descriptor.canonical_result_root().unwrap();
        let next_proof = proof(
            next,
            recovery.successor(),
            FormalCompletionBranchV1::Normal,
            None,
            Some(next_frozen),
            10,
            11,
            12,
        );
        let next_commit =
            verify_formal_qualified(&next_proof, recovery.successor(), Some(&committed), None)
                .unwrap();
        let (certificate, _, _, _) = initial_with_profile(2);
        let trust = InitialActivationTrustV1 {
            issuer_public_key: public(9),
            world: original.definition().world.clone(),
            initial_state_root: certificate.body.initial_state_root.clone(),
            governing_manifest_digest: certificate.body.governing_manifest_digest.clone(),
            recovery_policy_digest: original.policy().digest().unwrap(),
        };
        let initial_proof = initial_with_profile(2).2;
        let origin =
            VerifiedFormalReadAnchorV1::from_initial(&certificate, &trust, &initial_proof).unwrap();
        let anchor = origin
            .after_verified_recovery_commit(&recovery, committed.clone())
            .unwrap()
            .after_verified_commit(recovery.successor().clone(), next_commit)
            .unwrap();
        let delegated = VerifiedHistoricalCutInclusionV2::verify(
            &anchor,
            &descriptor,
            &inclusion,
            committed.record().position,
        )
        .unwrap();
        let cut = delegated
            .qualify_with_verified_recovery(&anchor, &recovery, &predecessor)
            .unwrap();
        assert_eq!(cut.proof(), &completed);
        let replay_cut = delegated
            .qualify_with_verified_recovery_contexts(&anchor, &[(&recovery, &predecessor)])
            .unwrap();
        assert_eq!(replay_cut.proof(), cut.proof());
        assert!(
            delegated
                .qualify_with_verified_recovery_contexts(&anchor, &[(&recovery, &committed)])
                .is_err()
        );
        assert_eq!(cut.commit().head(), committed.head());
        assert!(VerifiedFormalReadAnchorV1::from_committed_historical_cut(&anchor, &cut).is_ok());
        assert!(
            delegated
                .qualify_with_verified_recovery(&anchor, &recovery, &committed)
                .is_err()
        );
        assert!(
            delegated
                .qualify_with_verified_recovery(&origin, &recovery, &predecessor)
                .is_err()
        );
        assert!(
            VerifiedCommittedHistoricalCutV1::verify(
                &anchor,
                &descriptor,
                &inclusion,
                committed.record().position
            )
            .is_err()
        );
        let mut bad_proof = completed.clone();
        bad_proof.primary_complete.signature_hex = h(200);
        assert!(
            verify_formal_qualified(&bad_proof, &original, Some(&predecessor), Some(&recovery))
                .is_err()
        );
        assert!(
            delegated
                .qualify_with_verified_recovery_lineage(&anchor, &[])
                .is_err()
        );
        let wrong_permission = verified_recovery(&alternate_recovery_fixture(), &initial().1);
        assert!(
            delegated
                .qualify_with_verified_recovery(&anchor, &wrong_permission, &predecessor)
                .is_err()
        );
        let mut corrupt = inclusion.clone();
        corrupt.leaf.canonical_value.push(0);
        assert!(
            VerifiedHistoricalCutInclusionV2::verify(
                &anchor,
                &descriptor,
                &corrupt,
                committed.record().position
            )
            .is_err()
        );
    }
}
