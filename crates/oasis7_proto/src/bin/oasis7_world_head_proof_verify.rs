use std::fs;
use std::path::PathBuf;

use oasis7_proto::distributed::{
    BlobRef, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1, HeadConsensusEvidenceV1,
    WIRE_ENCODING_CBOR, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_HASH_DOMAIN_V1,
    WORLD_HEAD_PROOF_V1_SCHEMA, WorldBlock, WorldHeadAnnounce, WorldHeadProofV1,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFormat {
    Cbor,
    Json,
}

#[derive(Debug)]
struct Args {
    proof_path: PathBuf,
    proof_ref: Option<String>,
    format: InputFormat,
    expect_hash: Option<String>,
    expect_world_id: Option<String>,
    expect_height: Option<u64>,
    write_sample_json: Option<PathBuf>,
    emit_json: bool,
}

fn usage() -> &'static str {
    "Usage: oasis7_world_head_proof_verify --proof <path> [--format cbor|json] [--expect-hash <hash>] [--expect-world-id <id>] [--expect-height <height>] [--json]"
}

fn parse_args<I>(args: I) -> Result<Args, String>
where
    I: IntoIterator<Item = String>,
{
    let mut proof_path = None;
    let mut proof_ref = None;
    let mut format = InputFormat::Cbor;
    let mut expect_hash = None;
    let mut expect_world_id = None;
    let mut expect_height = None;
    let mut write_sample_json = None;
    let mut emit_json = false;
    let mut iter = args.into_iter();
    let _program = iter.next();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--proof" => {
                proof_path = Some(PathBuf::from(
                    iter.next()
                        .ok_or_else(|| "--proof requires a path".to_string())?,
                ));
            }
            "--format" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--format requires cbor or json".to_string())?;
                format = match raw.as_str() {
                    "cbor" => InputFormat::Cbor,
                    "json" => InputFormat::Json,
                    _ => return Err(format!("unsupported --format: {raw}")),
                };
            }
            "--proof-ref" => {
                proof_ref = Some(
                    iter.next()
                        .ok_or_else(|| "--proof-ref requires a value".to_string())?,
                );
            }
            "--expect-hash" => {
                expect_hash = Some(
                    iter.next()
                        .ok_or_else(|| "--expect-hash requires a value".to_string())?,
                );
            }
            "--expect-world-id" => {
                expect_world_id = Some(
                    iter.next()
                        .ok_or_else(|| "--expect-world-id requires a value".to_string())?,
                );
            }
            "--expect-height" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--expect-height requires a value".to_string())?;
                expect_height = Some(
                    raw.parse::<u64>()
                        .map_err(|err| format!("invalid --expect-height {raw}: {err}"))?,
                );
            }
            "--write-sample-json" => {
                write_sample_json =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--write-sample-json requires a path".to_string()
                    })?));
            }
            "--json" => {
                emit_json = true;
            }
            "-h" | "--help" => {
                return Err(usage().to_string());
            }
            _ => return Err(format!("unknown argument: {arg}\n{}", usage())),
        }
    }
    if write_sample_json.is_some() {
        return Ok(Args {
            proof_path: PathBuf::new(),
            proof_ref,
            format,
            expect_hash,
            expect_world_id,
            expect_height,
            write_sample_json,
            emit_json,
        });
    }
    Ok(Args {
        proof_path: proof_path.ok_or_else(|| "--proof is required".to_string())?,
        proof_ref,
        format,
        expect_hash,
        expect_world_id,
        expect_height,
        write_sample_json,
        emit_json,
    })
}

fn decode_proof(bytes: &[u8], format: InputFormat) -> Result<WorldHeadProofV1, String> {
    match format {
        InputFormat::Cbor => serde_cbor::from_slice(bytes)
            .map_err(|err| format!("decode WorldHeadProofV1 cbor: {err}")),
        InputFormat::Json => serde_json::from_slice(bytes)
            .map_err(|err| format!("decode WorldHeadProofV1 json: {err}")),
    }
}

fn verify(args: Args) -> Result<serde_json::Value, String> {
    if let Some(path) = args.write_sample_json {
        let proof = sample_world_head_proof();
        fs::write(
            &path,
            serde_json::to_vec_pretty(&proof)
                .map_err(|err| format!("encode sample proof json: {err}"))?,
        )
        .map_err(|err| format!("write sample proof {}: {err}", path.display()))?;
        return Ok(json!({
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
        }));
    }
    let bytes = fs::read(&args.proof_path)
        .map_err(|err| format!("read proof {}: {err}", args.proof_path.display()))?;
    let proof = decode_proof(bytes.as_slice(), args.format)?;
    proof.validate_contract()?;
    let proof_hash = proof.proof_hash()?;
    if let Some(expected) = args.expect_hash.as_deref() {
        if proof_hash != expected {
            return Err(format!(
                "proof hash mismatch: expected={expected} actual={proof_hash}"
            ));
        }
    }
    if let Some(expected) = args.expect_world_id.as_deref() {
        if proof.world_id != expected {
            return Err(format!(
                "world_id mismatch: expected={expected} actual={}",
                proof.world_id
            ));
        }
    }
    if let Some(expected) = args.expect_height {
        if proof.height != expected {
            return Err(format!(
                "height mismatch: expected={expected} actual={}",
                proof.height
            ));
        }
    }
    Ok(json!({
        "schema_version": "oasis7.world_head_proof_verifier.v1",
        "status": "pass",
        "proof_contract": "WorldHeadProofV1",
        "hash_domain": WORLD_HEAD_PROOF_HASH_DOMAIN_V1,
        "claim_boundary": WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1,
        "proof_hash": proof_hash,
        "proof_ref": args.proof_ref.unwrap_or_default(),
        "world_id": proof.world_id,
        "height": proof.height,
        "head": {
            "block_hash": proof.head.block_hash,
            "state_root": proof.head.state_root
        },
        "checkpoint_bound": proof.checkpoint.is_some(),
        "does_not_claim": [
            "mainnet-grade finality",
            "state proof",
            "receipt proof",
            "DA sampling",
            "full light client"
        ]
    }))
}

fn canonical_blake3_hex<T: Serialize>(value: &T) -> String {
    let payload = serde_cbor::to_vec(value).expect("encode canonical cbor");
    blake3::hash(payload.as_slice()).to_hex().to_string()
}

fn sample_world_head_proof() -> WorldHeadProofV1 {
    let block = WorldBlock {
        world_id: "world-a".to_string(),
        height: 42,
        prev_block_hash: "prev-block-41".to_string(),
        action_root: "action-root-42".to_string(),
        event_root: "event-root-42".to_string(),
        state_root: "state-root-42".to_string(),
        journal_ref: "journal-ref-42".to_string(),
        snapshot_ref: "snapshot-ref-42".to_string(),
        receipts_root: "receipts-root-42".to_string(),
        proposer_id: "validator-a".to_string(),
        timestamp_ms: 1_772_467_200_000,
        signature: "block-signature-evidence-only".to_string(),
    };
    let block_hash = canonical_blake3_hex(&block);
    WorldHeadProofV1 {
        schema_version: WORLD_HEAD_PROOF_V1_SCHEMA,
        world_id: "world-a".to_string(),
        height: 42,
        timestamp_ms: 1_772_467_200_000,
        head: WorldHeadAnnounce {
            world_id: "world-a".to_string(),
            height: 42,
            block_hash: block_hash.clone(),
            state_root: "state-root-42".to_string(),
            timestamp_ms: 1_772_467_200_000,
            signature: "head-signature-evidence-only".to_string(),
        },
        block,
        snapshot_manifest_ref: BlobRef {
            content_hash: "snapshot-ref-42".to_string(),
            size_bytes: 120,
            codec: WIRE_ENCODING_CBOR.to_string(),
            links: vec!["snapshot-chunk-1".to_string()],
        },
        journal_segments_ref: BlobRef {
            content_hash: "journal-ref-42".to_string(),
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
            execution_height: 42,
            node_block_hash: block_hash,
            execution_block_hash: "execution-block-42".to_string(),
            execution_state_root: "state-root-42".to_string(),
            action_root: "action-root-42".to_string(),
        },
        checkpoint: Some(CheckpointClosureEvidenceV1 {
            checkpoint_height: 42,
            execution_block_hash: "execution-block-42".to_string(),
            execution_state_root: "state-root-42".to_string(),
            manifest_ref: "checkpoint-manifest-42".to_string(),
            manifest_hash: "checkpoint-manifest-hash-42".to_string(),
            pinned_refs: vec![
                "snapshot-ref-42".to_string(),
                "journal-ref-42".to_string(),
                "state-root-42".to_string(),
            ],
        }),
        claim_boundary: WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1.to_string(),
    }
}

fn main() {
    let args = match parse_args(std::env::args()) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };
    let emit_json = args.emit_json;
    match verify(args) {
        Ok(summary) => {
            if emit_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).expect("serialize verifier summary")
                );
            } else {
                println!("world head proof verified");
            }
        }
        Err(err) => {
            eprintln!("world head proof verification failed: {err}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_valid_world_head_proof() -> WorldHeadProofV1 {
        sample_world_head_proof()
    }

    #[test]
    fn verifies_valid_json_proof_and_expectations() {
        let proof = sample_valid_world_head_proof();
        let proof_hash = proof.proof_hash().expect("proof hash");
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-world-head-proof-{}-valid.json",
            std::process::id()
        ));
        fs::write(
            &proof_path,
            serde_json::to_vec_pretty(&proof).expect("encode proof json"),
        )
        .expect("write proof");
        let summary = verify(Args {
            proof_path: proof_path.clone(),
            format: InputFormat::Json,
            expect_hash: Some(proof_hash.clone()),
            proof_ref: Some("proof-ref-42".to_string()),
            expect_world_id: Some("world-a".to_string()),
            expect_height: Some(42),
            write_sample_json: None,
            emit_json: true,
        })
        .expect("verify proof");
        let _ = fs::remove_file(&proof_path);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["proof_hash"], proof_hash);
        assert_eq!(summary["proof_ref"], "proof-ref-42");
        assert_eq!(
            summary["claim_boundary"],
            WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1
        );
    }

    #[test]
    fn rejects_hash_mismatch() {
        let proof = sample_valid_world_head_proof();
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-world-head-proof-{}-hash-mismatch.cbor",
            std::process::id()
        ));
        fs::write(
            &proof_path,
            serde_cbor::to_vec(&proof).expect("encode proof cbor"),
        )
        .expect("write proof");
        let err = verify(Args {
            proof_path: proof_path.clone(),
            format: InputFormat::Cbor,
            expect_hash: Some("wrong-hash".to_string()),
            proof_ref: None,
            expect_world_id: None,
            expect_height: None,
            write_sample_json: None,
            emit_json: true,
        })
        .expect_err("hash mismatch");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("proof hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_contract_tamper() {
        let mut proof = sample_valid_world_head_proof();
        proof.execution.execution_state_root = "wrong-state-root".to_string();
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-world-head-proof-{}-tamper.json",
            std::process::id()
        ));
        fs::write(
            &proof_path,
            serde_json::to_vec_pretty(&proof).expect("encode proof json"),
        )
        .expect("write proof");
        let err = verify(Args {
            proof_path: proof_path.clone(),
            format: InputFormat::Json,
            expect_hash: None,
            proof_ref: None,
            expect_world_id: None,
            expect_height: None,
            write_sample_json: None,
            emit_json: true,
        })
        .expect_err("tamper rejected");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("execution state_root mismatch"), "{err}");
    }
}
