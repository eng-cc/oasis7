use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use super::*;

pub(super) const STRICT_AUDIT_FUTURE_SKEW_MS: u64 = 30_000;

pub(super) fn strict_audit_evidence_digest(report: &[u8], manifest: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"oasis7:rollback-strict-audit-artifacts:v1\0");
    hasher.update((report.len() as u64).to_be_bytes());
    hasher.update(report);
    hasher.update((manifest.len() as u64).to_be_bytes());
    hasher.update(manifest);
    hex::encode(hasher.finalize())
}

pub(super) fn verify_strict_audit_evidence(
    world: &RuntimeWorld,
    audit: &crate::viewer::protocol::RollbackStrictAuditEvidence,
    now_ms: u64,
) -> Result<(), String> {
    if audit.schema_version != 1
        || audit.signature_scheme != "ed25519"
        || audit.authority_id.trim().is_empty()
        || audit.nonce.trim().is_empty()
        || audit.audit_report_bytes.is_empty()
        || audit.manifest_bytes.is_empty()
        || !audit.strict_registry_audit_passed
        || !audit.strict_manifest_audit_passed
        || audit.issued_at_ms > now_ms.saturating_add(STRICT_AUDIT_FUTURE_SKEW_MS)
        || audit.expires_at_ms < now_ms
        || audit.expires_at_ms <= audit.issued_at_ms
    {
        return Err("strict audit evidence envelope is incomplete or expired".to_string());
    }
    if strict_audit_evidence_digest(&audit.audit_report_bytes, &audit.manifest_bytes)
        != audit.evidence_digest
    {
        return Err("strict audit artifact digest mismatch".to_string());
    }
    let report: serde_json::Value = serde_json::from_slice(&audit.audit_report_bytes)
        .map_err(|_| "strict audit report bytes are not valid JSON".to_string())?;
    if !matches!(
        report
            .get("overall_status")
            .and_then(serde_json::Value::as_str),
        Some("pass" | "ready_for_ops_drill")
    ) || report
        .get("overall_single_failure_tolerance_pass")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
        || report
            .get("manifest_match_pass")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || report
            .get("rollback_blockers")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|blockers| !blockers.is_empty())
    {
        return Err("strict audit report does not prove a blocker-free pass".to_string());
    }
    let snapshot = world.snapshot();
    let authority = snapshot
        .rollback_authority_registry
        .records()
        .find(|record| {
            record.authority_id == audit.authority_id
                && record.active
                && record.role == crate::runtime::RollbackAuthorityRole::Governance
        })
        .ok_or_else(|| "strict audit signer is not the active governance authority".to_string())?;
    let public_key: [u8; 32] = hex::decode(authority.public_key_hex.trim())
        .map_err(|_| "invalid strict audit authority public key".to_string())?
        .try_into()
        .map_err(|_| "invalid strict audit authority public key length".to_string())?;
    let signature: [u8; 64] = hex::decode(audit.signature_hex.trim())
        .map_err(|_| "invalid strict audit signature".to_string())?
        .try_into()
        .map_err(|_| "invalid strict audit signature length".to_string())?;
    let payload = audit
        .canonical_signing_payload()
        .map_err(|err| format!("strict audit canonical payload failed: {err}"))?;
    VerifyingKey::from_bytes(&public_key)
        .map_err(|_| "invalid strict audit authority public key".to_string())?
        .verify(&payload, &Signature::from_bytes(&signature))
        .map_err(|_| "strict audit signature verification failed".to_string())
}
