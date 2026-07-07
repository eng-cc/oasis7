use std::fs;
use std::path::{Path, PathBuf};

use oasis7_proto::distributed::{
    WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_HASH_DOMAIN_V1, WorldHeadProofV1,
};
use serde_json::json;

#[path = "oasis7_world_head_proof_verify/oasis7_finality_proof.rs"]
mod oasis7_finality_proof;
#[path = "oasis7_world_head_proof_verify/oasis7_state_receipt_proof.rs"]
mod oasis7_state_receipt_proof;
#[path = "oasis7_world_head_proof_verify/oasis7_world_head_proof_samples.rs"]
mod oasis7_world_head_proof_samples;
#[path = "oasis7_world_head_proof_verify/oasis7_world_head_proof_window.rs"]
mod oasis7_world_head_proof_window;

use oasis7_finality_proof::{verify_finality_proof_path, write_sample_finality_json};
use oasis7_state_receipt_proof::{
    verify_state_receipt_proof_path, write_sample_state_receipt_json,
};
use oasis7_world_head_proof_samples::{write_sample_head_json, write_sample_window_json};
use oasis7_world_head_proof_window::{ProofWindowExpectations, verify_proof_window};

#[cfg(test)]
use oasis7_finality_proof::sample_world_finality_proof;
#[cfg(test)]
use oasis7_proto::distributed::{
    WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1, WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1,
};
#[cfg(test)]
use oasis7_state_receipt_proof::sample_world_state_receipt_proof;
#[cfg(test)]
use oasis7_world_head_proof_samples::{
    canonical_blake3_hex, sample_world_head_proof, sample_world_head_proof_at,
    sample_world_head_proof_window,
};
#[cfg(test)]
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFormat {
    Cbor,
    Json,
}

impl InputFormat {
    fn as_str(self) -> &'static str {
        match self {
            InputFormat::Cbor => "cbor",
            InputFormat::Json => "json",
        }
    }
}

#[derive(Debug)]
struct Args {
    proof_path: PathBuf,
    proof_window_path: Option<PathBuf>,
    state_receipt_proof_path: Option<PathBuf>,
    finality_proof_path: Option<PathBuf>,
    proof_ref: Option<String>,
    format: InputFormat,
    expect_hash: Option<String>,
    expect_world_id: Option<String>,
    expect_height: Option<u64>,
    expect_from_height: Option<u64>,
    expect_to_height: Option<u64>,
    expect_anchor_hash: Option<String>,
    expect_governance_set_hash: Option<String>,
    write_sample_json: Option<PathBuf>,
    write_sample_window_json: Option<PathBuf>,
    write_sample_state_receipt_json: Option<PathBuf>,
    write_sample_finality_json: Option<PathBuf>,
    emit_json: bool,
}

fn usage() -> &'static str {
    "Usage: oasis7_world_head_proof_verify (--proof <path>|--proof-window <manifest.json>|--state-receipt-proof <path>|--finality-proof <path>) [--format cbor|json] [--expect-hash <hash>] [--expect-world-id <id>] [--expect-height <height>] [--expect-from-height <height>] [--expect-to-height <height>] [--expect-anchor-hash <hash>] [--expect-governance-set-hash <hash>] [--write-sample-json <path>] [--write-sample-window-json <dir>] [--write-sample-state-receipt-json <path>] [--write-sample-finality-json <path>] [--json]"
}

fn parse_args<I>(args: I) -> Result<Args, String>
where
    I: IntoIterator<Item = String>,
{
    let mut proof_path = None;
    let mut proof_window_path = None;
    let mut state_receipt_proof_path = None;
    let mut finality_proof_path = None;
    let mut proof_ref = None;
    let mut format = InputFormat::Cbor;
    let mut expect_hash = None;
    let mut expect_world_id = None;
    let mut expect_height = None;
    let mut expect_from_height = None;
    let mut expect_to_height = None;
    let mut expect_anchor_hash = None;
    let mut expect_governance_set_hash = None;
    let mut write_sample_json = None;
    let mut write_sample_window_json = None;
    let mut write_sample_state_receipt_json = None;
    let mut write_sample_finality_json = None;
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
            "--proof-window" => {
                proof_window_path =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--proof-window requires a path".to_string()
                    })?));
            }
            "--state-receipt-proof" => {
                state_receipt_proof_path =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--state-receipt-proof requires a path".to_string()
                    })?));
            }
            "--finality-proof" => {
                finality_proof_path =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--finality-proof requires a path".to_string()
                    })?));
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
            "--expect-from-height" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--expect-from-height requires a value".to_string())?;
                expect_from_height = Some(
                    raw.parse::<u64>()
                        .map_err(|err| format!("invalid --expect-from-height {raw}: {err}"))?,
                );
            }
            "--expect-to-height" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--expect-to-height requires a value".to_string())?;
                expect_to_height = Some(
                    raw.parse::<u64>()
                        .map_err(|err| format!("invalid --expect-to-height {raw}: {err}"))?,
                );
            }
            "--expect-anchor-hash" => {
                expect_anchor_hash = Some(
                    iter.next()
                        .ok_or_else(|| "--expect-anchor-hash requires a value".to_string())?,
                );
            }
            "--expect-governance-set-hash" => {
                expect_governance_set_hash =
                    Some(iter.next().ok_or_else(|| {
                        "--expect-governance-set-hash requires a value".to_string()
                    })?);
            }
            "--write-sample-json" => {
                write_sample_json =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--write-sample-json requires a path".to_string()
                    })?));
            }
            "--write-sample-window-json" => {
                write_sample_window_json = Some(PathBuf::from(iter.next().ok_or_else(|| {
                    "--write-sample-window-json requires a directory".to_string()
                })?));
            }
            "--write-sample-state-receipt-json" => {
                write_sample_state_receipt_json =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--write-sample-state-receipt-json requires a path".to_string()
                    })?));
            }
            "--write-sample-finality-json" => {
                write_sample_finality_json =
                    Some(PathBuf::from(iter.next().ok_or_else(|| {
                        "--write-sample-finality-json requires a path".to_string()
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
            proof_window_path,
            state_receipt_proof_path,
            finality_proof_path,
            proof_ref,
            format,
            expect_hash,
            expect_world_id,
            expect_height,
            expect_from_height,
            expect_to_height,
            expect_anchor_hash,
            expect_governance_set_hash,
            write_sample_json,
            write_sample_window_json,
            write_sample_state_receipt_json,
            write_sample_finality_json,
            emit_json,
        });
    }
    if write_sample_window_json.is_some() {
        return Ok(Args {
            proof_path: PathBuf::new(),
            proof_window_path,
            state_receipt_proof_path,
            finality_proof_path,
            proof_ref,
            format,
            expect_hash,
            expect_world_id,
            expect_height,
            expect_from_height,
            expect_to_height,
            expect_anchor_hash,
            expect_governance_set_hash,
            write_sample_json,
            write_sample_window_json,
            write_sample_state_receipt_json,
            write_sample_finality_json,
            emit_json,
        });
    }
    if write_sample_state_receipt_json.is_some() {
        return Ok(Args {
            proof_path: PathBuf::new(),
            proof_window_path,
            state_receipt_proof_path,
            finality_proof_path,
            proof_ref,
            format,
            expect_hash,
            expect_world_id,
            expect_height,
            expect_from_height,
            expect_to_height,
            expect_anchor_hash,
            expect_governance_set_hash,
            write_sample_json,
            write_sample_window_json,
            write_sample_state_receipt_json,
            write_sample_finality_json,
            emit_json,
        });
    }
    if write_sample_finality_json.is_some() {
        return Ok(Args {
            proof_path: PathBuf::new(),
            proof_window_path,
            state_receipt_proof_path,
            finality_proof_path,
            proof_ref,
            format,
            expect_hash,
            expect_world_id,
            expect_height,
            expect_from_height,
            expect_to_height,
            expect_anchor_hash,
            expect_governance_set_hash,
            write_sample_json,
            write_sample_window_json,
            write_sample_state_receipt_json,
            write_sample_finality_json,
            emit_json,
        });
    }
    let selected_modes = [
        proof_path.is_some(),
        proof_window_path.is_some(),
        state_receipt_proof_path.is_some(),
        finality_proof_path.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if selected_modes > 1 {
        return Err(
            "--proof, --proof-window, --state-receipt-proof, and --finality-proof cannot be combined".to_string(),
        );
    }
    Ok(Args {
        proof_path: match (
            &proof_path,
            &proof_window_path,
            &state_receipt_proof_path,
            &finality_proof_path,
        ) {
            (Some(path), None, None, None) => path.clone(),
            (None, Some(_), None, None)
            | (None, None, Some(_), None)
            | (None, None, None, Some(_)) => PathBuf::new(),
            (None, None, None, None) => {
                return Err(
                    "--proof, --proof-window, --state-receipt-proof, or --finality-proof is required".to_string(),
                );
            }
            _ => unreachable!("selected_modes rejected combined proof modes"),
        },
        proof_window_path,
        state_receipt_proof_path,
        finality_proof_path,
        proof_ref,
        format,
        expect_hash,
        expect_world_id,
        expect_height,
        expect_from_height,
        expect_to_height,
        expect_anchor_hash,
        expect_governance_set_hash,
        write_sample_json,
        write_sample_window_json,
        write_sample_state_receipt_json,
        write_sample_finality_json,
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

fn decode_proof_from_path(path: &Path, format: InputFormat) -> Result<WorldHeadProofV1, String> {
    let bytes = fs::read(path).map_err(|err| format!("read proof {}: {err}", path.display()))?;
    decode_proof(bytes.as_slice(), format)
}

fn verify(args: Args) -> Result<serde_json::Value, String> {
    if let Some(path) = args.write_sample_json {
        return write_sample_head_json(path.as_path());
    }
    if let Some(dir) = args.write_sample_window_json {
        return write_sample_window_json(dir.as_path());
    }
    if let Some(path) = args.write_sample_state_receipt_json {
        return write_sample_state_receipt_json(path.as_path());
    }
    if let Some(path) = args.write_sample_finality_json {
        return write_sample_finality_json(path.as_path());
    }
    if let Some(window_path) = args.proof_window_path.clone() {
        return verify_proof_window(
            window_path.as_path(),
            ProofWindowExpectations {
                expect_world_id: args.expect_world_id.as_deref(),
                expect_height: args.expect_height,
                expect_from_height: args.expect_from_height,
                expect_to_height: args.expect_to_height,
                expect_anchor_hash: args.expect_anchor_hash.as_deref(),
            },
        );
    }
    if let Some(path) = args.state_receipt_proof_path.clone() {
        return verify_state_receipt_proof_path(
            path.as_path(),
            args.format.as_str(),
            args.expect_hash.as_deref(),
            args.expect_world_id.as_deref(),
            args.expect_height,
            args.proof_ref,
        );
    }
    if let Some(path) = args.finality_proof_path.clone() {
        return verify_finality_proof_path(
            path.as_path(),
            args.format.as_str(),
            args.expect_hash.as_deref(),
            args.expect_world_id.as_deref(),
            args.expect_height,
            args.expect_governance_set_hash.as_deref(),
            args.proof_ref,
        );
    }
    let proof = decode_proof_from_path(args.proof_path.as_path(), args.format)?;
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

    fn base_args() -> Args {
        Args {
            proof_path: PathBuf::new(),
            proof_window_path: None,
            state_receipt_proof_path: None,
            finality_proof_path: None,
            format: InputFormat::Cbor,
            expect_hash: None,
            proof_ref: None,
            expect_world_id: None,
            expect_height: None,
            expect_from_height: None,
            expect_to_height: None,
            expect_anchor_hash: None,
            expect_governance_set_hash: None,
            write_sample_json: None,
            write_sample_window_json: None,
            write_sample_state_receipt_json: None,
            write_sample_finality_json: None,
            emit_json: true,
        }
    }

    fn write_json(path: &Path, value: &impl Serialize) {
        fs::write(
            path,
            serde_json::to_vec_pretty(value).expect("encode json fixture"),
        )
        .expect("write json fixture");
    }

    fn write_window_fixture(
        label: &str,
        proofs: &[WorldHeadProofV1],
        observed_head: Option<&WorldHeadProofV1>,
    ) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "oasis7-proof-window-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create proof window temp dir");
        let mut entries = Vec::new();
        for (index, proof) in proofs.iter().enumerate() {
            let proof_path = dir.join(format!("proof-{index}.json"));
            write_json(&proof_path, proof);
            entries.push(json!({
                "proof": proof_path.file_name().expect("proof file name").to_string_lossy(),
                "proof_ref": format!("proof-ref-{}", proof.height),
                "format": "json",
                "expect_hash": proof.proof_hash().expect("proof hash")
            }));
        }
        let observed = observed_head.map(|proof| {
            json!({
                "height": proof.height,
                "block_hash": proof.head.block_hash,
                "state_root": proof.head.state_root
            })
        });
        let window_path = dir.join("window.json");
        let manifest = json!({
            "schema_version": "oasis7.world_head_proof_window.v1",
            "window_id": label,
            "world_id": "world-a",
            "from_height": proofs.first().expect("first proof").height,
            "to_height": proofs.last().expect("last proof").height,
            "trusted_anchor": {
                "height": proofs.first().expect("first proof").height - 1,
                "block_hash": proofs.first().expect("first proof").block.prev_block_hash,
                "state_root": "trusted-anchor-state-root"
            },
            "proofs": entries,
            "observed_head": observed
        });
        write_json(&window_path, &manifest);
        window_path
    }

    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_slice(&fs::read(path).expect("read json fixture"))
            .expect("decode json fixture")
    }

    fn rewrite_json(path: &Path, value: &serde_json::Value) {
        fs::write(
            path,
            serde_json::to_vec_pretty(value).expect("encode rewritten json fixture"),
        )
        .expect("rewrite json fixture");
    }

    fn sample_proof_window() -> Vec<WorldHeadProofV1> {
        sample_world_head_proof_window()
    }

    fn refresh_proof_block_hash(proof: &mut WorldHeadProofV1) {
        let block_hash = canonical_blake3_hex(&proof.block);
        proof.head.block_hash = block_hash.clone();
        proof.execution.node_block_hash = block_hash;
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
            ..base_args()
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
            ..base_args()
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
            ..base_args()
        })
        .expect_err("tamper rejected");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("execution state_root mismatch"), "{err}");
    }

    #[test]
    fn verifies_valid_state_receipt_json_proof_and_expectations() {
        let proof = sample_world_state_receipt_proof();
        let proof_hash = proof.proof_hash().expect("proof hash");
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-state-receipt-proof-{}-valid.json",
            std::process::id()
        ));
        write_json(&proof_path, &proof);
        let summary = verify(Args {
            proof_path: PathBuf::new(),
            state_receipt_proof_path: Some(proof_path.clone()),
            format: InputFormat::Json,
            expect_hash: Some(proof_hash.clone()),
            proof_ref: Some("state-receipt-proof-ref-42".to_string()),
            expect_world_id: Some("world-a".to_string()),
            expect_height: Some(42),
            ..base_args()
        })
        .expect("verify state receipt proof");
        let _ = fs::remove_file(&proof_path);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["proof_contract"], "WorldStateReceiptProofV1");
        assert_eq!(summary["proof_hash"], proof_hash);
        assert_eq!(summary["proof_ref"], "state-receipt-proof-ref-42");
        assert_eq!(
            summary["claim_boundary"],
            WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1
        );
    }

    #[test]
    fn rejects_state_receipt_hash_mismatch() {
        let proof = sample_world_state_receipt_proof();
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-state-receipt-proof-{}-hash-mismatch.cbor",
            std::process::id()
        ));
        fs::write(
            &proof_path,
            serde_cbor::to_vec(&proof).expect("encode state receipt proof cbor"),
        )
        .expect("write proof");
        let err = verify(Args {
            proof_path: PathBuf::new(),
            state_receipt_proof_path: Some(proof_path.clone()),
            format: InputFormat::Cbor,
            expect_hash: Some("wrong-hash".to_string()),
            ..base_args()
        })
        .expect_err("hash mismatch");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("state receipt proof hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_state_receipt_root_tamper() {
        let mut proof = sample_world_state_receipt_proof();
        proof.root_hash = "wrong-root".to_string();
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-state-receipt-proof-{}-root-tamper.json",
            std::process::id()
        ));
        write_json(&proof_path, &proof);
        let err = verify(Args {
            proof_path: PathBuf::new(),
            state_receipt_proof_path: Some(proof_path.clone()),
            format: InputFormat::Json,
            ..base_args()
        })
        .expect_err("root tamper rejected");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("root hash mismatch"), "{err}");
    }

    #[test]
    fn verifies_valid_finality_json_proof_and_expectations() {
        let proof = sample_world_finality_proof();
        let proof_hash = proof.proof_hash().expect("proof hash");
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-finality-proof-{}-valid.json",
            std::process::id()
        ));
        write_json(&proof_path, &proof);
        let summary = verify(Args {
            proof_path: PathBuf::new(),
            finality_proof_path: Some(proof_path.clone()),
            format: InputFormat::Json,
            expect_hash: Some(proof_hash.clone()),
            proof_ref: Some("finality-proof-ref-42".to_string()),
            expect_world_id: Some("world-a".to_string()),
            expect_height: Some(42),
            ..base_args()
        })
        .expect("verify finality proof");
        let _ = fs::remove_file(&proof_path);
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["verifier_mode"], "validator_set_finality");
        assert_eq!(summary["proof_contract"], "WorldFinalityProofV1");
        assert_eq!(summary["proof_hash"], proof_hash);
        assert_eq!(summary["proof_ref"], "finality-proof-ref-42");
        assert_eq!(
            summary["claim_boundary"],
            WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1
        );
        assert_eq!(summary["validator_set"]["validator_count"], 3);
        assert_eq!(summary["finality"]["stake_threshold_checked"], true);
    }

    #[test]
    fn rejects_finality_hash_mismatch() {
        let proof = sample_world_finality_proof();
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-finality-proof-{}-hash-mismatch.cbor",
            std::process::id()
        ));
        fs::write(
            &proof_path,
            serde_cbor::to_vec(&proof).expect("encode finality proof cbor"),
        )
        .expect("write proof");
        let err = verify(Args {
            proof_path: PathBuf::new(),
            finality_proof_path: Some(proof_path.clone()),
            format: InputFormat::Cbor,
            expect_hash: Some("wrong-hash".to_string()),
            ..base_args()
        })
        .expect_err("hash mismatch");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("finality proof hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_finality_commitment_tamper() {
        let mut proof = sample_world_finality_proof();
        proof.finality_commitments[0].votes.pop();
        let proof_path = std::env::temp_dir().join(format!(
            "oasis7-finality-proof-{}-below-threshold.json",
            std::process::id()
        ));
        write_json(&proof_path, &proof);
        let err = verify(Args {
            proof_path: PathBuf::new(),
            finality_proof_path: Some(proof_path.clone()),
            format: InputFormat::Json,
            ..base_args()
        })
        .expect_err("below threshold rejected");
        let _ = fs::remove_file(&proof_path);
        assert!(err.contains("signed stake below threshold"), "{err}");
    }

    #[test]
    fn verifies_contiguous_world_head_proof_window() {
        let proofs = sample_proof_window();
        let window_path = write_window_fixture("valid", proofs.as_slice(), proofs.last());
        let summary = verify(Args {
            proof_path: PathBuf::new(),
            proof_window_path: Some(window_path.clone()),
            expect_world_id: Some("world-a".to_string()),
            expect_from_height: Some(40),
            expect_to_height: Some(42),
            expect_anchor_hash: Some("prev-block-39".to_string()),
            ..base_args()
        })
        .expect("verify proof window");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert_eq!(summary["status"], "pass");
        assert_eq!(summary["verifier_mode"], "proof_window_continuity");
        assert_eq!(summary["from_height"], 40);
        assert_eq!(summary["to_height"], 42);
        assert_eq!(summary["proof_count"], 3);
        assert_eq!(summary["trusted_anchor"]["height"], 39);
        assert_eq!(
            summary["claim_boundary"],
            "proof_window_continuity_evidence_only_not_full_light_client_or_mainnet_readiness"
        );
    }

    #[test]
    fn rejects_proof_window_height_gap() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let second = sample_world_head_proof_at(42, first.head.block_hash.as_str());
        let window_path = write_window_fixture("height-gap", &[first, second], None);
        let err = verify(Args {
            proof_path: PathBuf::new(),
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("height gap rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("proof window height gap"), "{err}");
    }

    #[test]
    fn rejects_proof_window_prev_hash_fork() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let second = sample_world_head_proof_at(41, "forked-prev-hash");
        let window_path = write_window_fixture("fork", &[first, second], None);
        let err = verify(Args {
            proof_path: PathBuf::new(),
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("prev hash fork rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("prev_block_hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_proof_window_below_quorum() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.vote_count = 1;
        let window_path = write_window_fixture("below-quorum", &[first, second], None);
        let err = verify(Args {
            proof_path: PathBuf::new(),
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("quorum violation rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("vote_count below quorum_threshold"), "{err}");
    }

    #[test]
    fn rejects_proof_window_world_id_mismatch() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.world_id = "world-b".to_string();
        second.head.world_id = "world-b".to_string();
        second.block.world_id = "world-b".to_string();
        refresh_proof_block_hash(&mut second);
        let window_path = write_window_fixture("world-id-mismatch", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("world id mismatch rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("proof window world_id mismatch"), "{err}");
    }

    #[test]
    fn rejects_proof_window_observed_head_mismatch() {
        let proofs = sample_proof_window();
        let mut observed = proofs.last().expect("last proof").clone();
        observed.head.block_hash = "wrong-observed-head".to_string();
        let window_path = write_window_fixture("observed-head-mismatch", &proofs, Some(&observed));
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("observed head mismatch rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("observed head hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_proof_window_timestamp_rollback() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.timestamp_ms = first.timestamp_ms - 1;
        second.head.timestamp_ms = first.head.timestamp_ms - 1;
        second.block.timestamp_ms = first.block.timestamp_ms - 1;
        refresh_proof_block_hash(&mut second);
        let window_path = write_window_fixture("timestamp-rollback", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("timestamp rollback rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("timestamp regressed"), "{err}");
    }

    #[test]
    fn rejects_proof_window_empty_proof_ref() {
        let proofs = sample_proof_window();
        let window_path = write_window_fixture("empty-proof-ref", &proofs, None);
        let mut manifest = read_json(&window_path);
        manifest["proofs"][1]["proof_ref"] = json!("");
        rewrite_json(&window_path, &manifest);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("empty proof ref rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("proof_ref missing"), "{err}");
    }

    #[test]
    fn rejects_proof_window_entry_hash_mismatch() {
        let proofs = sample_proof_window();
        let window_path = write_window_fixture("entry-hash-mismatch", &proofs, None);
        let mut manifest = read_json(&window_path);
        manifest["proofs"][1]["expect_hash"] = json!("wrong-hash");
        rewrite_json(&window_path, &manifest);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("entry hash mismatch rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_proof_window_anchor_mismatch() {
        let proofs = sample_proof_window();
        let window_path = write_window_fixture("anchor-mismatch", &proofs, None);
        let mut manifest = read_json(&window_path);
        manifest["trusted_anchor"]["block_hash"] = json!("wrong-anchor");
        rewrite_json(&window_path, &manifest);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("anchor mismatch rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("anchor hash mismatch"), "{err}");
    }

    #[test]
    fn rejects_proof_window_missing_expected_anchor() {
        let proofs = sample_proof_window();
        let window_path = write_window_fixture("missing-expected-anchor", &proofs, None);
        let mut manifest = read_json(&window_path);
        manifest
            .as_object_mut()
            .expect("window manifest object")
            .remove("trusted_anchor");
        rewrite_json(&window_path, &manifest);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            expect_anchor_hash: Some("prev-block-39".to_string()),
            ..base_args()
        })
        .expect_err("missing expected anchor rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("trusted_anchor missing"), "{err}");
    }

    #[test]
    fn rejects_proof_window_quorum_zero() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.quorum_threshold = 0;
        let window_path = write_window_fixture("quorum-zero", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("zero quorum rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("quorum_threshold must be positive"), "{err}");
    }

    #[test]
    fn rejects_proof_window_validator_count_below_threshold() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.validator_count = 1;
        let window_path = write_window_fixture("validator-count-low", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("validator count rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(
            err.contains("validator_count below quorum_threshold"),
            "{err}"
        );
    }

    #[test]
    fn rejects_proof_window_vote_count_above_validator_count() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.vote_count = 4;
        let window_path = write_window_fixture("vote-count-high", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("vote count rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("vote_count exceeds validator_count"), "{err}");
    }

    #[test]
    fn rejects_proof_window_duplicate_approvers_below_threshold() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.approver_ids = vec!["validator-a".to_string(), "validator-a".to_string()];
        let window_path = write_window_fixture("duplicate-approvers", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("duplicate approvers rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("duplicate approvers"), "{err}");
    }

    #[test]
    fn rejects_proof_window_duplicate_approvers_even_above_threshold() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.vote_count = 3;
        second.consensus.approver_ids = vec![
            "validator-a".to_string(),
            "validator-a".to_string(),
            "validator-b".to_string(),
        ];
        let window_path = write_window_fixture(
            "duplicate-approvers-above-threshold",
            &[first, second],
            None,
        );
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("duplicate approvers rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(err.contains("duplicate approvers"), "{err}");
    }

    #[test]
    fn rejects_proof_window_unique_approvers_below_threshold() {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let mut second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        second.consensus.approver_ids = vec!["validator-a".to_string()];
        let window_path = write_window_fixture("unique-approvers-low", &[first, second], None);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("unique approvers rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(
            err.contains("unique approvers below quorum_threshold"),
            "{err}"
        );
    }

    #[test]
    fn rejects_proof_window_unsupported_entry_format() {
        let proofs = sample_proof_window();
        let window_path = write_window_fixture("unsupported-entry-format", &proofs, None);
        let mut manifest = read_json(&window_path);
        manifest["proofs"][0]["format"] = json!("yaml");
        rewrite_json(&window_path, &manifest);
        let err = verify(Args {
            proof_window_path: Some(window_path.clone()),
            ..base_args()
        })
        .expect_err("unsupported format rejected");
        let _ = fs::remove_dir_all(window_path.parent().expect("window parent"));
        assert!(
            err.contains("unsupported proof window entry format"),
            "{err}"
        );
    }

    #[test]
    fn rejects_combined_proof_and_proof_window_args() {
        let err = parse_args([
            "verify".to_string(),
            "--proof".to_string(),
            "proof.cbor".to_string(),
            "--finality-proof".to_string(),
            "finality.json".to_string(),
        ])
        .expect_err("combined proof modes rejected");
        assert!(
            err.contains(
                "--proof, --proof-window, --state-receipt-proof, and --finality-proof cannot be combined"
            ),
            "{err}"
        );
    }

    #[test]
    fn rejects_missing_proof_mode_args() {
        let err = parse_args(["verify".to_string()]).expect_err("missing proof mode rejected");
        assert!(
            err.contains(
                "--proof, --proof-window, --state-receipt-proof, or --finality-proof is required"
            ),
            "{err}"
        );
    }
}
