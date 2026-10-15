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

#[test]
fn current_native_embedded_nodes_avoid_source_reads_but_preserve_missing_node_fallback_and_hash_checks()
 {
    use crate::controlled_authority::formal::business_index::BusinessIndexNodeReader;
    let mut embedded = super::super::native_history::EmbeddedBusinessNodes::default();
    let bytes = b"authenticated current material node";
    let id = blake3::hash(bytes).to_hex().to_string();
    embedded.insert_write(&id, bytes).unwrap();
    assert!(embedded.insert_write(&id, b"different bytes").is_err());
    let missing_bytes = b"authenticated node outside current and parent material";
    let missing = blake3::hash(missing_bytes).to_hex().to_string();
    let reads = std::cell::Cell::new(0);
    let mut load = |digest: &str| {
        reads.set(reads.get() + 1);
        assert_eq!(digest, missing);
        Ok(missing_bytes.to_vec())
    };
    let reader = NativeBusinessReader {
        load: std::cell::RefCell::new(&mut load),
        embedded: &embedded,
    };
    for _ in 0..8 {
        assert_eq!(reader.read_node(&id).unwrap(), Some(bytes.to_vec()));
    }
    assert_eq!(reads.get(), 0);
    assert_eq!(
        reader.read_node(&missing).unwrap(),
        Some(missing_bytes.to_vec())
    );
    assert_eq!(reads.get(), 1);
}
