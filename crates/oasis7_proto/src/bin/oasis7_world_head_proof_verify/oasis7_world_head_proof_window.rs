use std::fs;
use std::path::{Path, PathBuf};

use oasis7_proto::distributed::{WORLD_HEAD_PROOF_HASH_DOMAIN_V1, WorldHeadProofV1};
use serde::Deserialize;
use serde_json::json;

pub(crate) struct ProofWindowExpectations<'a> {
    pub(crate) expect_world_id: Option<&'a str>,
    pub(crate) expect_height: Option<u64>,
    pub(crate) expect_from_height: Option<u64>,
    pub(crate) expect_to_height: Option<u64>,
    pub(crate) expect_anchor_hash: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct ProofWindowManifest {
    schema_version: String,
    #[serde(default)]
    window_id: String,
    #[serde(default)]
    world_id: String,
    from_height: u64,
    to_height: u64,
    #[serde(default)]
    trusted_anchor: Option<ProofWindowAnchor>,
    proofs: Vec<ProofWindowEntry>,
    #[serde(default)]
    observed_head: Option<ObservedHead>,
}

#[derive(Debug, Deserialize)]
struct ProofWindowAnchor {
    height: u64,
    block_hash: String,
    #[serde(default)]
    state_root: String,
}

#[derive(Debug, Deserialize)]
struct ProofWindowEntry {
    proof: PathBuf,
    #[serde(default)]
    proof_ref: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    expect_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ObservedHead {
    height: u64,
    block_hash: String,
    state_root: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProofWindowInputFormat {
    Cbor,
    Json,
}

fn decode_proof_from_path(
    path: &Path,
    format: ProofWindowInputFormat,
) -> Result<WorldHeadProofV1, String> {
    let bytes = fs::read(path).map_err(|err| format!("read proof {}: {err}", path.display()))?;
    match format {
        ProofWindowInputFormat::Cbor => serde_cbor::from_slice(bytes.as_slice())
            .map_err(|err| format!("decode WorldHeadProofV1 cbor: {err}")),
        ProofWindowInputFormat::Json => serde_json::from_slice(bytes.as_slice())
            .map_err(|err| format!("decode WorldHeadProofV1 json: {err}")),
    }
}

fn input_format_from_entry(entry: &ProofWindowEntry) -> Result<ProofWindowInputFormat, String> {
    match entry.format.as_deref().unwrap_or("cbor") {
        "cbor" => Ok(ProofWindowInputFormat::Cbor),
        "json" => Ok(ProofWindowInputFormat::Json),
        raw => Err(format!("unsupported proof window entry format: {raw}")),
    }
}

fn resolve_window_entry_path(window_path: &Path, entry_path: &Path) -> PathBuf {
    if entry_path.is_absolute() {
        return entry_path.to_path_buf();
    }
    window_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(entry_path)
}

fn validate_window_consensus(proof: &WorldHeadProofV1) -> Result<(), String> {
    if proof.consensus.quorum_threshold == 0 {
        return Err(format!(
            "proof window height {} quorum_threshold must be positive",
            proof.height
        ));
    }
    if proof.consensus.validator_count < proof.consensus.quorum_threshold {
        return Err(format!(
            "proof window height {} validator_count below quorum_threshold",
            proof.height
        ));
    }
    if proof.consensus.vote_count < proof.consensus.quorum_threshold {
        return Err(format!(
            "proof window height {} vote_count below quorum_threshold",
            proof.height
        ));
    }
    if proof.consensus.vote_count > proof.consensus.validator_count {
        return Err(format!(
            "proof window height {} vote_count exceeds validator_count",
            proof.height
        ));
    }
    let unique_approvers = proof
        .consensus
        .approver_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if unique_approvers.len() != proof.consensus.approver_ids.len() {
        return Err(format!(
            "proof window height {} duplicate approvers are not allowed",
            proof.height
        ));
    }
    if unique_approvers.len() < proof.consensus.quorum_threshold as usize {
        return Err(format!(
            "proof window height {} unique approvers below quorum_threshold",
            proof.height
        ));
    }
    Ok(())
}

pub(crate) fn verify_proof_window(
    window_path: &Path,
    expectations: ProofWindowExpectations<'_>,
) -> Result<serde_json::Value, String> {
    let manifest: ProofWindowManifest = serde_json::from_slice(
        fs::read(window_path)
            .map_err(|err| format!("read proof window {}: {err}", window_path.display()))?
            .as_slice(),
    )
    .map_err(|err| format!("decode proof window {}: {err}", window_path.display()))?;
    if manifest.schema_version != "oasis7.world_head_proof_window.v1" {
        return Err(format!(
            "unsupported proof window schema: {}",
            manifest.schema_version
        ));
    }
    if manifest.proofs.is_empty() {
        return Err("proof window must include at least one proof".to_string());
    }
    if manifest.from_height == 0 || manifest.to_height < manifest.from_height {
        return Err(format!(
            "invalid proof window height range: from={} to={}",
            manifest.from_height, manifest.to_height
        ));
    }
    if let Some(expected) = expectations.expect_from_height {
        if manifest.from_height != expected {
            return Err(format!(
                "proof window from_height mismatch: expected={} actual={}",
                expected, manifest.from_height
            ));
        }
    }
    if let Some(expected) = expectations.expect_to_height.or(expectations.expect_height) {
        if manifest.to_height != expected {
            return Err(format!(
                "proof window to_height mismatch: expected={} actual={}",
                expected, manifest.to_height
            ));
        }
    }
    if let (Some(anchor), Some(expected)) = (
        manifest.trusted_anchor.as_ref(),
        expectations.expect_anchor_hash,
    ) {
        if anchor.block_hash != expected {
            return Err(format!(
                "proof window anchor hash mismatch: expected={} actual={}",
                expected, anchor.block_hash
            ));
        }
    }

    let mut previous: Option<WorldHeadProofV1> = None;
    let mut proof_refs = Vec::new();
    let mut proof_hashes = Vec::new();
    let mut checkpoint_bound_count = 0_u64;
    let mut world_id = manifest.world_id.trim().to_string();

    for (index, entry) in manifest.proofs.iter().enumerate() {
        let proof_path = resolve_window_entry_path(window_path, entry.proof.as_path());
        let proof = decode_proof_from_path(proof_path.as_path(), input_format_from_entry(entry)?)?;
        proof.validate_contract()?;
        validate_window_consensus(&proof)?;
        let proof_hash = proof.proof_hash()?;
        if let Some(expected_hash) = entry.expect_hash.as_deref() {
            if proof_hash != expected_hash {
                return Err(format!(
                    "proof window entry {index} hash mismatch: expected={expected_hash} actual={proof_hash}"
                ));
            }
        }
        if entry.proof_ref.trim().is_empty() {
            return Err(format!("proof window entry {index} proof_ref missing"));
        }
        if world_id.is_empty() {
            world_id = proof.world_id.clone();
        }
        if proof.world_id != world_id {
            return Err(format!(
                "proof window world_id mismatch at height {}: expected={} actual={}",
                proof.height, world_id, proof.world_id
            ));
        }
        if let Some(expected) = expectations.expect_world_id {
            if proof.world_id != expected {
                return Err(format!(
                    "proof window expected world_id mismatch at height {}: expected={} actual={}",
                    proof.height, expected, proof.world_id
                ));
            }
        }
        let expected_height = manifest.from_height + index as u64;
        if proof.height != expected_height {
            return Err(format!(
                "proof window height gap at index {index}: expected={} actual={}",
                expected_height, proof.height
            ));
        }
        if index == 0 {
            if let Some(anchor) = manifest.trusted_anchor.as_ref() {
                if anchor.height + 1 != proof.height {
                    return Err(format!(
                        "proof window anchor height mismatch: anchor={} first_proof={}",
                        anchor.height, proof.height
                    ));
                }
                if anchor.block_hash != proof.block.prev_block_hash {
                    return Err(format!(
                        "proof window anchor hash mismatch: anchor={} first_prev={}",
                        anchor.block_hash, proof.block.prev_block_hash
                    ));
                }
            }
        } else if let Some(previous) = &previous {
            if proof.block.prev_block_hash != previous.head.block_hash {
                return Err(format!(
                    "proof window prev_block_hash mismatch at height {}: expected={} actual={}",
                    proof.height, previous.head.block_hash, proof.block.prev_block_hash
                ));
            }
            if proof.timestamp_ms < previous.timestamp_ms {
                return Err(format!(
                    "proof window timestamp regressed at height {}",
                    proof.height
                ));
            }
        }
        if proof.checkpoint.is_some() {
            checkpoint_bound_count += 1;
        }
        proof_refs.push(entry.proof_ref.trim().to_string());
        proof_hashes.push(proof_hash);
        previous = Some(proof);
    }

    let last_proof = previous.expect("proof window proof");
    if last_proof.height != manifest.to_height {
        return Err(format!(
            "proof window to_height mismatch: manifest={} actual={}",
            manifest.to_height, last_proof.height
        ));
    }
    if let Some(observed) = &manifest.observed_head {
        if observed.height != last_proof.height {
            return Err(format!(
                "proof window observed head height mismatch: observed={} verified={}",
                observed.height, last_proof.height
            ));
        }
        if observed.block_hash != last_proof.head.block_hash {
            return Err(format!(
                "proof window observed head hash mismatch: observed={} verified={}",
                observed.block_hash, last_proof.head.block_hash
            ));
        }
        if observed.state_root != last_proof.head.state_root {
            return Err(format!(
                "proof window observed state root mismatch: observed={} verified={}",
                observed.state_root, last_proof.head.state_root
            ));
        }
    }

    Ok(json!({
        "schema_version": "oasis7.world_head_proof_window_verifier.v1",
        "status": "pass",
        "verifier_mode": "proof_window_continuity",
        "proof_contract": "WorldHeadProofV1",
        "window_contract": "WorldHeadProofWindowV1",
        "hash_domain": WORLD_HEAD_PROOF_HASH_DOMAIN_V1,
        "claim_boundary": "proof_window_continuity_evidence_only_not_full_light_client_or_mainnet_readiness",
        "window_id": manifest.window_id,
        "world_id": world_id,
        "from_height": manifest.from_height,
        "to_height": manifest.to_height,
        "proof_count": manifest.proofs.len(),
        "trusted_anchor": manifest.trusted_anchor.as_ref().map(|anchor| json!({
            "height": anchor.height,
            "block_hash": anchor.block_hash,
            "state_root": anchor.state_root
        })),
        "proof_refs": proof_refs,
        "proof_hashes": proof_hashes,
        "head": {
            "height": last_proof.height,
            "block_hash": last_proof.head.block_hash,
            "state_root": last_proof.head.state_root
        },
        "checkpoint_bound_count": checkpoint_bound_count,
        "continuity": {
            "height_contiguous": true,
            "prev_hash_linked": true,
            "world_id_consistent": true,
            "quorum_threshold_checked": true,
            "observed_head_matched": manifest.observed_head.is_some()
        },
        "does_not_claim": [
            "mainnet-grade finality",
            "state proof",
            "receipt proof",
            "DA sampling",
            "full light client",
            "multi-client consensus equivalence"
        ]
    }))
}
