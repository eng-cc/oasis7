//! Predicate regressions run with genuinely restored protected-RPC tokens.
//! Private mutations below exist only in tests; production has no raw constructor.
use super::*;

pub(in super::super) fn assert_bridge_negatives(
    bridge: &VerifiedNativeCheckpointRestoreV1,
    fresh: &VerifiedNativeCheckpointRestoreV1,
) {
    assert_eq!(fresh.minimum(), &bridge.commit().head());
    let joined = fresh
        .clone()
        .with_verified_ancestry_bridge(bridge, bridge)
        .unwrap();
    assert_eq!(joined.minimum(), bridge.minimum());
    assert_eq!(joined.checkpoint(), fresh.checkpoint());
    assert_eq!(joined.pending(), fresh.pending());
    assert_eq!(joined.proof(), fresh.proof());
    assert_eq!(joined.candidate(), fresh.candidate());
    for corruption in 0..7 {
        let mut invalid = fresh.clone();
        match corruption {
            0 => invalid.minimum.record_id = "00".repeat(32),
            1 => invalid.proof.primary_complete.signature_hex = "00".repeat(64),
            2 => invalid.candidate.material.objects[0].bytes.push(0),
            3 => invalid.instance_digest = "00".repeat(32),
            4 => invalid.resource_binding = "unrelated-protected-resource".into(),
            5 => invalid.checkpoint.revision = bridge.checkpoint.revision.saturating_sub(1),
            _ => invalid.checkpoint.head.qualified = false,
        }
        assert!(
            invalid
                .with_verified_ancestry_bridge(bridge, bridge)
                .is_err(),
            "case {corruption}"
        );
    }
    let mut same_revision_fork = fresh.clone();
    same_revision_fork.checkpoint.revision = bridge.checkpoint.revision;
    same_revision_fork.checkpoint.commitment = "00".repeat(32);
    assert!(
        same_revision_fork
            .with_verified_ancestry_bridge(bridge, bridge)
            .is_err()
    );
    let mut wrong_origin = bridge.clone();
    wrong_origin.minimum.record_id = "00".repeat(32);
    assert!(
        fresh
            .clone()
            .with_verified_ancestry_bridge(bridge, &wrong_origin)
            .is_err()
    );
}
