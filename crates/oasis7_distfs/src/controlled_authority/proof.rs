use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::Serialize;

use super::*;

const SIGNING_DOMAIN: &str = "oasis7.controlled_authority.local_decision.sign.v1";
const HASH_DOMAIN: &str = "oasis7.controlled_authority.local_decision.proof.v1";

// Only closed structs/ordered vectors enter this encoding; no map iteration order.
fn domain_bytes<T: Serialize>(domain: &str, value: &T) -> Result<Vec<u8>, LocalJournalError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(domain.len() as u64).to_be_bytes());
    bytes.extend_from_slice(domain.as_bytes());
    bytes.extend_from_slice(&serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?);
    Ok(bytes)
}

pub(super) fn domain_hash<T: Serialize>(
    domain: &str,
    value: &T,
) -> Result<String, LocalJournalError> {
    Ok(blake3::hash(&domain_bytes(domain, value)?)
        .to_hex()
        .to_string())
}

pub(super) fn sign_local_decision(
    decision: LocalDecisionBodyV1,
    signer: &SigningKey,
) -> Result<ControlledAuthorityLocalDecisionProofV1, LocalJournalError> {
    let signature_hex = hex::encode(
        signer
            .sign(&domain_bytes(SIGNING_DOMAIN, &decision)?)
            .to_bytes(),
    );
    Ok(ControlledAuthorityLocalDecisionProofV1 {
        decision,
        signature_hex,
    })
}

/// Offline local evidence verification only. Trusted identity and parent are external inputs.
/// This verifies neither blob availability nor fsync nor activation/formal finality.
pub fn verify_local_decision_proof(
    proof: &ControlledAuthorityLocalDecisionProofV1,
    trusted: &TrustedLocalAuthority,
    trusted_parent: &LocalHeadAnchor,
) -> Result<LocalHeadAnchor, LocalJournalError> {
    trusted.validate()?;
    hash_hex(&trusted_parent.proof_hash, "trusted parent hash")?;
    if trusted_parent.position == 0 && trusted_parent != &trusted.genesis_anchor()? {
        return Err(invalid(
            "position-zero parent differs from trusted genesis anchor",
        ));
    }
    let body = &proof.decision;
    if body.schema_version != LOCAL_DECISION_SCHEMA
        || body.profile != LOCAL_AUTHORITY_PROFILE
        || body.scope != LOCAL_DECISION_SCOPE
        || &body.authority != trusted
    {
        return Err(invalid(
            "proof schema/profile/scope/trusted authority mismatch",
        ));
    }
    if trusted_parent.position.checked_add(1) != Some(body.position)
        || body.parent_hash != trusted_parent.proof_hash
    {
        return Err(invalid("proof position/parent mismatch"));
    }
    body.request.validate()?;
    body.prepared.validate()?;
    if proof.signature_hex.len() != 128
        || !proof
            .signature_hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(
            "signature requires canonical lowercase 64-byte hex",
        ));
    }
    let key: [u8; 32] = hex::decode(&trusted.signer_public_key_hex)
        .map_err(|e| invalid(e.to_string()))?
        .try_into()
        .map_err(|_| invalid("signer key size"))?;
    let signature: [u8; 64] = hex::decode(&proof.signature_hex)
        .map_err(|e| invalid(e.to_string()))?
        .try_into()
        .map_err(|_| invalid("signature size"))?;
    let key = VerifyingKey::from_bytes(&key).map_err(|e| invalid(e.to_string()))?;
    key.verify_strict(
        &domain_bytes(SIGNING_DOMAIN, body)?,
        &Signature::from_bytes(&signature),
    )
    .map_err(|e| invalid(format!("local decision signature: {e}")))?;
    Ok(LocalHeadAnchor {
        position: body.position,
        proof_hash: domain_hash(HASH_DOMAIN, proof)?,
    })
}
