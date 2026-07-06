use std::fs;
use std::path::Path;

use oasis7_proto::distributed::{
    BlobRef, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1, HeadConsensusEvidenceV1,
    WIRE_ENCODING_CBOR, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_V1_SCHEMA, WorldBlock,
    WorldHeadAnnounce, WorldHeadProofV1,
};
use serde::Serialize;
use serde_json::json;

pub(crate) fn canonical_blake3_hex<T: Serialize>(value: &T) -> String {
    let payload = serde_cbor::to_vec(value).expect("encode canonical cbor");
    blake3::hash(payload.as_slice()).to_hex().to_string()
}

pub(crate) fn sample_world_head_proof() -> WorldHeadProofV1 {
    sample_world_head_proof_at(42, "prev-block-41")
}

pub(crate) fn sample_world_head_proof_at(height: u64, prev_block_hash: &str) -> WorldHeadProofV1 {
    let state_root = format!("state-root-{height}");
    let action_root = format!("action-root-{height}");
    let journal_ref = format!("journal-ref-{height}");
    let snapshot_ref = format!("snapshot-ref-{height}");
    let block = WorldBlock {
        world_id: "world-a".to_string(),
        height,
        prev_block_hash: prev_block_hash.to_string(),
        action_root: action_root.clone(),
        event_root: format!("event-root-{height}"),
        state_root: state_root.clone(),
        journal_ref: journal_ref.clone(),
        snapshot_ref: snapshot_ref.clone(),
        receipts_root: format!("receipts-root-{height}"),
        proposer_id: "validator-a".to_string(),
        timestamp_ms: 1_772_467_200_000 + height as i64,
        signature: "block-signature-evidence-only".to_string(),
    };
    let block_hash = canonical_blake3_hex(&block);
    WorldHeadProofV1 {
        schema_version: WORLD_HEAD_PROOF_V1_SCHEMA,
        world_id: "world-a".to_string(),
        height,
        timestamp_ms: 1_772_467_200_000 + height as i64,
        head: WorldHeadAnnounce {
            world_id: "world-a".to_string(),
            height,
            block_hash: block_hash.clone(),
            state_root: state_root.clone(),
            timestamp_ms: 1_772_467_200_000 + height as i64,
            signature: "head-signature-evidence-only".to_string(),
        },
        block,
        snapshot_manifest_ref: BlobRef {
            content_hash: snapshot_ref.clone(),
            size_bytes: 120,
            codec: WIRE_ENCODING_CBOR.to_string(),
            links: vec!["snapshot-chunk-1".to_string()],
        },
        journal_segments_ref: BlobRef {
            content_hash: journal_ref.clone(),
            size_bytes: 80,
            codec: WIRE_ENCODING_CBOR.to_string(),
            links: vec!["journal-segment-1".to_string()],
        },
        consensus: HeadConsensusEvidenceV1 {
            consensus_status: "committed".to_string(),
            proposer_id: "validator-a".to_string(),
            quorum_threshold: 2,
            validator_count: 3,
            vote_count: 2,
            approver_ids: vec!["validator-a".to_string(), "validator-b".to_string()],
            evidence_hash: "consensus-evidence-42".to_string(),
        },
        execution: ExecutionBindingEvidenceV1 {
            execution_height: height,
            node_block_hash: block_hash,
            execution_block_hash: format!("execution-block-{height}"),
            execution_state_root: state_root.clone(),
            action_root,
        },
        checkpoint: Some(CheckpointClosureEvidenceV1 {
            checkpoint_height: height,
            execution_block_hash: format!("execution-block-{height}"),
            execution_state_root: state_root.clone(),
            manifest_ref: format!("checkpoint-manifest-{height}"),
            manifest_hash: format!("checkpoint-manifest-hash-{height}"),
            pinned_refs: vec![snapshot_ref, journal_ref, state_root],
        }),
        claim_boundary: WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1.to_string(),
    }
}

pub(crate) fn sample_world_head_proof_window() -> Vec<WorldHeadProofV1> {
    let first = sample_world_head_proof_at(40, "prev-block-39");
    let second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
    let third = sample_world_head_proof_at(42, second.head.block_hash.as_str());
    vec![first, second, third]
}

pub(crate) fn write_sample_head_json(path: &Path) -> Result<serde_json::Value, String> {
    let proof = sample_world_head_proof();
    fs::write(
        path,
        serde_json::to_vec_pretty(&proof)
            .map_err(|err| format!("encode sample proof json: {err}"))?,
    )
    .map_err(|err| format!("write sample proof {}: {err}", path.display()))?;
    Ok(json!({
        "schema_version": "oasis7.world_head_proof_verifier_fixture.v1",
        "status": "sample_written",
        "proof_path": path,
        "proof_hash": proof.proof_hash()?,
        "world_id": proof.world_id,
        "height": proof.height,
        "head": {
            "block_hash": proof.head.block_hash,
            "state_root": proof.head.state_root
        }
    }))
}

pub(crate) fn write_sample_window_json(dir: &Path) -> Result<serde_json::Value, String> {
    let proofs = sample_world_head_proof_window();
    fs::create_dir_all(dir)
        .map_err(|err| format!("create sample proof window dir {}: {err}", dir.display()))?;
    let mut entries = Vec::new();
    for (index, proof) in proofs.iter().enumerate() {
        let proof_file_name = format!("proof-{index}.json");
        let proof_path = dir.join(&proof_file_name);
        fs::write(
            &proof_path,
            serde_json::to_vec_pretty(proof)
                .map_err(|err| format!("encode sample window proof json: {err}"))?,
        )
        .map_err(|err| format!("write sample window proof {}: {err}", proof_path.display()))?;
        entries.push(json!({
            "proof": proof_file_name,
            "proof_ref": format!("sample-proof-ref-{}", proof.height),
            "format": "json",
            "expect_hash": proof.proof_hash()?,
        }));
    }
    let first = proofs.first().expect("sample proof window first proof");
    let last = proofs.last().expect("sample proof window last proof");
    let window_path = dir.join("window.json");
    let manifest = json!({
        "schema_version": "oasis7.world_head_proof_window.v1",
        "window_id": "sample-window-40-42",
        "world_id": first.world_id,
        "from_height": first.height,
        "to_height": last.height,
        "trusted_anchor": {
            "height": first.height - 1,
            "block_hash": first.block.prev_block_hash,
            "state_root": "trusted-anchor-state-root"
        },
        "proofs": entries,
        "observed_head": {
            "height": last.height,
            "block_hash": last.head.block_hash,
            "state_root": last.head.state_root
        }
    });
    fs::write(
        &window_path,
        serde_json::to_vec_pretty(&manifest)
            .map_err(|err| format!("encode sample window manifest json: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "write sample window manifest {}: {err}",
            window_path.display()
        )
    })?;
    Ok(json!({
        "schema_version": "oasis7.world_head_proof_window_verifier_fixture.v1",
        "status": "sample_window_written",
        "window_path": window_path,
        "world_id": first.world_id,
        "from_height": first.height,
        "to_height": last.height,
        "trusted_anchor": {
            "height": first.height - 1,
            "block_hash": first.block.prev_block_hash
        },
        "observed_head": {
            "height": last.height,
            "block_hash": last.head.block_hash,
            "state_root": last.head.state_root
        }
    }))
}
