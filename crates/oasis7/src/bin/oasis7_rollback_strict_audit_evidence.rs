use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use oasis7::viewer::{RollbackStrictAuditEvidenceInput, build_unsigned_strict_audit_evidence};

#[derive(Debug)]
struct Options {
    audit_report: PathBuf,
    manifest: PathBuf,
    output: Option<PathBuf>,
    canonical_payload_output: Option<PathBuf>,
    authority_id: String,
    rollback_ticket: String,
    receipt_id: String,
    canonical_intent_digest: String,
    recovery_snapshot_hash: String,
    candidate_state_root: String,
    reorg_epoch: u64,
    issued_at_ms: u64,
    expires_at_ms: u64,
    nonce: String,
    private_key_file: Option<PathBuf>,
    signature_file: Option<PathBuf>,
    signer_public_key_file: Option<PathBuf>,
}

fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let options = parse_options(args)?;
    let report = std::fs::read(&options.audit_report)
        .map_err(|error| format!("read audit report failed: {error}"))?;
    let manifest = std::fs::read(&options.manifest)
        .map_err(|error| format!("read manifest failed: {error}"))?;
    let mut evidence = build_unsigned_strict_audit_evidence(RollbackStrictAuditEvidenceInput {
        authority_id: options.authority_id,
        rollback_ticket: options.rollback_ticket,
        receipt_id: options.receipt_id,
        canonical_intent_digest: options.canonical_intent_digest,
        recovery_snapshot_hash: options.recovery_snapshot_hash,
        reorg_epoch: options.reorg_epoch,
        candidate_state_root: options.candidate_state_root,
        audit_report_bytes: report,
        manifest_bytes: manifest,
        issued_at_ms: options.issued_at_ms,
        expires_at_ms: options.expires_at_ms,
        nonce: options.nonce,
    });
    let payload = evidence
        .canonical_signing_payload()
        .map_err(|error| format!("construct canonical payload failed: {error}"))?;
    if let Some(path) = options.canonical_payload_output.as_deref() {
        write_private(path, &payload)?;
    }

    match (
        options.private_key_file.as_deref(),
        options.signature_file.as_deref(),
    ) {
        (Some(key_path), None) => {
            let signing_key = read_signing_key(key_path)?;
            evidence.signature_hex = hex::encode(signing_key.sign(&payload).to_bytes());
        }
        (None, Some(signature_path)) => {
            let public_path = options.signer_public_key_file.as_deref().ok_or_else(|| {
                "--signer-public-key-file is required with --signature-file".to_string()
            })?;
            let signature = read_signature(signature_path)?;
            let public_key = read_verifying_key(public_path)?;
            public_key
                .verify(&payload, &signature)
                .map_err(|_| "external signer signature does not verify".to_string())?;
            evidence.signature_hex = hex::encode(signature.to_bytes());
        }
        (None, None) if options.canonical_payload_output.is_some() && options.output.is_none() => {
            return Ok(());
        }
        (Some(_), Some(_)) => {
            return Err("choose exactly one of --private-key-file or --signature-file".to_string());
        }
        _ => {
            return Err(
                "signed output requires --private-key-file or --signature-file".to_string(),
            );
        }
    }
    let output = options
        .output
        .as_deref()
        .ok_or_else(|| "--output is required for signed evidence".to_string())?;
    let encoded = serde_json::to_vec_pretty(&evidence)
        .map_err(|error| format!("encode evidence failed: {error}"))?;
    write_private(output, &encoded)
}

fn parse_options(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut values = BTreeMap::new();
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        if !flag.starts_with("--") {
            return Err(format!("unexpected argument {flag}"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        if values.insert(flag.clone(), value).is_some() {
            return Err(format!("duplicate option {flag}"));
        }
    }
    let required = |flag: &str| {
        values
            .get(flag)
            .cloned()
            .ok_or_else(|| format!("missing required option {flag}"))
    };
    let parse_u64 = |flag: &str| -> Result<u64, String> {
        required(flag)?
            .parse()
            .map_err(|_| format!("{flag} must be an unsigned integer"))
    };
    Ok(Options {
        audit_report: required("--audit-report")?.into(),
        manifest: required("--manifest")?.into(),
        output: values.get("--output").map(PathBuf::from),
        canonical_payload_output: values.get("--canonical-payload-output").map(PathBuf::from),
        authority_id: required("--authority-id")?,
        rollback_ticket: required("--rollback-ticket")?,
        receipt_id: required("--receipt-id")?,
        canonical_intent_digest: required("--canonical-intent-digest")?,
        recovery_snapshot_hash: required("--recovery-snapshot-hash")?,
        candidate_state_root: required("--candidate-state-root")?,
        reorg_epoch: parse_u64("--reorg-epoch")?,
        issued_at_ms: parse_u64("--issued-at-ms")?,
        expires_at_ms: parse_u64("--expires-at-ms")?,
        nonce: required("--nonce")?,
        private_key_file: values.get("--private-key-file").map(PathBuf::from),
        signature_file: values.get("--signature-file").map(PathBuf::from),
        signer_public_key_file: values.get("--signer-public-key-file").map(PathBuf::from),
    })
}

fn read_hex<const N: usize>(path: &Path, label: &str) -> Result<[u8; N], String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("read {label} failed: {error}"))?;
    hex::decode(text.trim())
        .map_err(|_| format!("{label} must be hex"))?
        .try_into()
        .map_err(|_| format!("{label} must contain exactly {N} bytes"))
}

fn read_signing_key(path: &Path) -> Result<SigningKey, String> {
    Ok(SigningKey::from_bytes(&read_hex(path, "private key")?))
}

fn read_signature(path: &Path) -> Result<Signature, String> {
    Ok(Signature::from_bytes(&read_hex(path, "signature")?))
}

fn read_verifying_key(path: &Path) -> Result<VerifyingKey, String> {
    VerifyingKey::from_bytes(&read_hex(path, "public key")?)
        .map_err(|_| "public key is invalid".to_string())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|error| format!("write output failed: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("secure output permissions failed: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_signer_preserves_exact_artifacts_without_emitting_private_key() {
        let root = std::env::temp_dir().join(format!(
            "oasis7-strict-audit-evidence-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create temp dir");
        let report = root.join("audit.json");
        let manifest = root.join("manifest.json");
        let key = root.join("governance.key");
        let output = root.join("evidence.json");
        let report_bytes = br#"{"overall_status":"ready_for_ops_drill","overall_single_failure_tolerance_pass":true,"manifest_match_pass":true,"rollback_blockers":[]}"#;
        let manifest_bytes = br#"{"entries":[{"slot_id":"governance.rollback.v1"}]}"#;
        let signing_key = SigningKey::from_bytes(&[73; 32]);
        std::fs::write(&report, report_bytes).expect("write report");
        std::fs::write(&manifest, manifest_bytes).expect("write manifest");
        std::fs::write(&key, hex::encode(signing_key.to_bytes())).expect("write key");
        run([
            "--audit-report",
            report.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--authority-id",
            "governance-bob",
            "--rollback-ticket",
            "ROLLBACK-2313",
            "--receipt-id",
            "receipt-1",
            "--canonical-intent-digest",
            "intent-1",
            "--recovery-snapshot-hash",
            "snapshot-1",
            "--candidate-state-root",
            "root-1",
            "--reorg-epoch",
            "4",
            "--issued-at-ms",
            "100",
            "--expires-at-ms",
            "200",
            "--nonce",
            "audit-1",
            "--private-key-file",
            key.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_string))
        .expect("construct evidence");
        let encoded = std::fs::read(&output).expect("read evidence");
        let evidence: oasis7_proto::viewer::RollbackStrictAuditEvidence =
            serde_json::from_slice(&encoded).expect("decode evidence");
        assert_eq!(evidence.audit_report_bytes, report_bytes);
        assert_eq!(evidence.manifest_bytes, manifest_bytes);
        assert!(!String::from_utf8_lossy(&encoded).contains(&hex::encode(signing_key.to_bytes())));
        signing_key
            .verifying_key()
            .verify(
                &evidence.canonical_signing_payload().expect("payload"),
                &Signature::from_slice(&hex::decode(&evidence.signature_hex).unwrap()).unwrap(),
            )
            .expect("valid signature");
        let _ = std::fs::remove_dir_all(root);
    }
}
