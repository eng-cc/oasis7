use std::fs;
use std::path::Path;

use oasis7_proto::distributed::{
    BlobRef, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1, HeadConsensusEvidenceV1,
    WIRE_ENCODING_CBOR, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_V1_SCHEMA,
    WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1, WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1,
    WORLD_STATE_RECEIPT_PROOF_HASH_DOMAIN_V1, WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA, WorldBlock,
    WorldHeadAnnounce, WorldHeadProofV1, WorldStateReceiptProofKindV1,
    WorldStateReceiptProofNodeV1, WorldStateReceiptProofSiblingSideV1,
    WorldStateReceiptProofStatusV1, WorldStateReceiptProofSubjectV1, WorldStateReceiptProofV1,
    compute_world_state_receipt_root,
};
use serde::Serialize;
use serde_json::json;

pub(crate) fn write_sample_state_receipt_json(path: &Path) -> Result<serde_json::Value, String> {
    let proof = sample_world_state_receipt_proof();
    fs::write(
        path,
        serde_json::to_vec_pretty(&proof)
            .map_err(|err| format!("encode sample state receipt proof json: {err}"))?,
    )
    .map_err(|err| format!("write sample state receipt proof {}: {err}", path.display()))?;
    Ok(json!({
        "schema_version": "oasis7.world_state_receipt_proof_verifier_fixture.v1",
        "status": "sample_written",
        "proof_path": path,
        "proof_contract": "WorldStateReceiptProofV1",
        "proof_hash": proof.proof_hash()?,
        "head_proof_hash": proof.head_proof_hash,
        "world_id": proof.world_id,
        "height": proof.height,
        "proof_kind": proof.proof_kind,
        "proof_status": proof.proof_status,
        "root_hash": proof.root_hash,
    }))
}

pub(crate) fn verify_state_receipt_proof_path(
    path: &Path,
    format: &str,
    expect_hash: Option<&str>,
    expect_world_id: Option<&str>,
    expect_height: Option<u64>,
    proof_ref: Option<String>,
) -> Result<serde_json::Value, String> {
    let proof = decode_state_receipt_proof_from_path(path, format)?;
    proof.validate_contract()?;
    let proof_hash = proof.proof_hash()?;
    if let Some(expected) = expect_hash {
        if proof_hash != expected {
            return Err(format!(
                "state receipt proof hash mismatch: expected={expected} actual={proof_hash}"
            ));
        }
    }
    if let Some(expected) = expect_world_id {
        if proof.world_id != expected {
            return Err(format!(
                "world_id mismatch: expected={expected} actual={}",
                proof.world_id
            ));
        }
    }
    if let Some(expected) = expect_height {
        if proof.height != expected {
            return Err(format!(
                "height mismatch: expected={expected} actual={}",
                proof.height
            ));
        }
    }
    Ok(json!({
        "schema_version": "oasis7.world_state_receipt_proof_verifier.v1",
        "status": "pass",
        "proof_contract": "WorldStateReceiptProofV1",
        "hash_domain": WORLD_STATE_RECEIPT_PROOF_HASH_DOMAIN_V1,
        "claim_boundary": WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1,
        "proof_hash": proof_hash,
        "proof_ref": proof_ref.unwrap_or_default(),
        "head_proof_hash": proof.head_proof_hash,
        "world_id": proof.world_id,
        "height": proof.height,
        "proof_kind": proof.proof_kind,
        "proof_status": proof.proof_status,
        "subject": proof.subject,
        "root_hash": proof.root_hash,
        "leaf_hash": proof.leaf_hash,
        "proof_path_nodes": proof.proof_path.len(),
        "head": {
            "block_hash": proof.head_proof.head.block_hash,
            "state_root": proof.head_proof.block.state_root,
            "receipts_root": proof.head_proof.block.receipts_root
        },
        "does_not_claim": [
            "mainnet-grade finality",
            "full light client",
            "validator-set finality",
            "DA sampling",
            "multi-client consensus equivalence",
            "live runtime arbitrary state proof availability"
        ]
    }))
}

pub(crate) fn decode_state_receipt_proof_from_path(
    path: &Path,
    format: &str,
) -> Result<WorldStateReceiptProofV1, String> {
    let bytes = fs::read(path)
        .map_err(|err| format!("read state receipt proof {}: {err}", path.display()))?;
    match format {
        "cbor" => serde_cbor::from_slice(bytes.as_slice())
            .map_err(|err| format!("decode WorldStateReceiptProofV1 cbor: {err}")),
        "json" => serde_json::from_slice(bytes.as_slice())
            .map_err(|err| format!("decode WorldStateReceiptProofV1 json: {err}")),
        _ => Err(format!("unsupported state receipt proof format: {format}")),
    }
}

pub(crate) fn sample_world_state_receipt_proof() -> WorldStateReceiptProofV1 {
    let mut head_proof = sample_world_head_proof();
    let subject = WorldStateReceiptProofSubjectV1::ResourceState {
        namespace: "inventory".to_string(),
        resource_id: "agent-1/bag".to_string(),
        value_hash: "resource-value-hash-42".to_string(),
        value_codec: WIRE_ENCODING_CBOR.to_string(),
        absence_marker_hash: String::new(),
    };
    let proof_status = WorldStateReceiptProofStatusV1::Included;
    let leaf_hash = subject.leaf_hash(proof_status).expect("leaf hash");
    let proof_path =
        sample_state_receipt_path(head_proof.block.state_root.as_str(), leaf_hash.as_str());
    let state_root = compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
        .expect("state root");
    head_proof.block.state_root = state_root.clone();
    head_proof.head.state_root = state_root.clone();
    head_proof.execution.execution_state_root = state_root.clone();
    head_proof
        .checkpoint
        .as_mut()
        .expect("checkpoint")
        .execution_state_root = state_root.clone();
    head_proof.head.block_hash = canonical_blake3_hex(&head_proof.block);
    head_proof.execution.node_block_hash = head_proof.head.block_hash.clone();
    let head_proof_hash = head_proof.proof_hash().expect("head proof hash");
    WorldStateReceiptProofV1 {
        schema_version: WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA,
        world_id: head_proof.world_id.clone(),
        height: head_proof.height,
        head_proof,
        head_proof_hash,
        proof_kind: WorldStateReceiptProofKindV1::ResourceState,
        proof_status,
        root_hash: state_root,
        subject,
        leaf_hash,
        proof_path,
        claim_boundary: WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string(),
    }
}

fn sample_state_receipt_path(
    root_seed: &str,
    leaf_hash: &str,
) -> Vec<WorldStateReceiptProofNodeV1> {
    let sibling_hash = canonical_blake3_hex(&(
        WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1,
        leaf_hash,
        root_seed,
    ));
    vec![WorldStateReceiptProofNodeV1 {
        sibling_hash,
        sibling_side: WorldStateReceiptProofSiblingSideV1::Right,
    }]
}

fn sample_world_head_proof() -> WorldHeadProofV1 {
    sample_world_head_proof_at(42, "prev-block-41")
}

fn sample_world_head_proof_at(height: u64, prev_block_hash: &str) -> WorldHeadProofV1 {
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

fn canonical_blake3_hex<T: Serialize>(value: &T) -> String {
    let payload = serde_cbor::to_vec(value).expect("encode canonical cbor");
    blake3::hash(payload.as_slice()).to_hex().to_string()
}
