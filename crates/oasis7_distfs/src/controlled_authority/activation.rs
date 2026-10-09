//! Offline initial-activation prerequisite; never activates a runtime or authorizes an upgrade.
use super::replicated_protocol::{
    ArtifactRole, DurabilityEvidence, FixedTrust, HeadAnchor, ProtocolError, verify_evidence,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const DOMAIN: &str = "oasis7.controlled_authority.initial_activation.v1";
const MAX_ENVELOPE_BYTES: usize = 16 * 1024;

/// Independently authenticated issuer policy, supplied by the caller. Never derive
/// this value from the certificate, its signature, or a local configuration hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialActivationPolicy {
    pub trust: FixedTrust,
    pub issuer_public_key: String,
    pub initial_state_root: String,
    pub execution_manifest_root: String,
    pub activation_height: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialActivationBody {
    pub schema_version: u32,
    pub profile: String,
    pub profile_version: u32,
    pub trust: FixedTrust,
    pub position: u64,
    pub parent_hash: String,
    pub activation_height: u64,
    pub before_state_root: String,
    pub after_state_root: String,
    /// Exactly the eight non-Input roles. Input is the signed envelope itself.
    pub artifact_roots: BTreeMap<ArtifactRole, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedInitialActivation {
    pub body: InitialActivationBody,
    pub signature_hex: String,
}

/// Construction is restricted to successful verification. This is not a CommitRef,
/// runtime-readiness token, execution verifier, or proof of independent hardware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedInitialActivation {
    body: InitialActivationBody,
    qualified_head: HeadAnchor,
    envelope_digest: String,
}
impl VerifiedInitialActivation {
    pub fn body(&self) -> &InitialActivationBody {
        &self.body
    }
    pub fn qualified_head(&self) -> &HeadAnchor {
        &self.qualified_head
    }
    pub fn envelope_digest(&self) -> &str {
        &self.envelope_digest
    }
}
fn invalid(reason: &str) -> ProtocolError {
    ProtocolError::Invalid(reason.into())
}
fn hex_bytes<const N: usize>(value: &str) -> Result<[u8; N], ProtocolError> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("noncanonical activation key/hash/signature"));
    }
    hex::decode(value)
        .map_err(|_| invalid("activation hex"))?
        .try_into()
        .map_err(|_| invalid("activation hex length"))
}
/// Canonical domain-separated bytes for an authorized offline issuer. No signing
/// key or production signing operation is provided by this module.
pub fn initial_activation_signing_bytes(
    body: &InitialActivationBody,
) -> Result<Vec<u8>, ProtocolError> {
    let encoded = serde_cbor::to_vec(body).map_err(|_| invalid("activation serialization"))?;
    if encoded.len() > MAX_ENVELOPE_BYTES {
        return Err(invalid("activation body byte limit"));
    }
    let mut bytes = (DOMAIN.len() as u64).to_be_bytes().to_vec();
    bytes.extend_from_slice(DOMAIN.as_bytes());
    bytes.extend(encoded);
    Ok(bytes)
}

/// Verify original signed Input bytes and the complete existing two-endpoint proof.
/// The externally retained minimum must be the independently trusted genesis head:
/// any prior decision/activation requires a future explicit transition verifier.
pub fn verify_initial_activation(
    envelope_bytes: &[u8],
    evidence: &DurabilityEvidence,
    policy: &InitialActivationPolicy,
    minimum: &HeadAnchor,
) -> Result<VerifiedInitialActivation, ProtocolError> {
    if envelope_bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(invalid("activation envelope byte limit"));
    }
    policy.trust.validate()?;
    let t = &policy.trust;
    if t.writer_key == t.primary_key || t.writer_key == t.replica_key {
        return Err(invalid(
            "activation writer and endpoint keys must be distinct",
        ));
    }
    let issuer = VerifyingKey::from_bytes(&hex_bytes::<32>(&policy.issuer_public_key)?)
        .map_err(|_| invalid("activation issuer key"))?;
    hex_bytes::<32>(&policy.initial_state_root)?;
    hex_bytes::<32>(&policy.execution_manifest_root)?;
    let genesis = t.genesis_anchor()?;
    if minimum != &genesis {
        return Err(invalid(
            "initial activation requires external genesis minimum",
        ));
    }
    let envelope: SignedInitialActivation = serde_cbor::from_slice(envelope_bytes)
        .map_err(|_| invalid("activation envelope decoding"))?;
    // Require the single canonical representation, including no trailing bytes.
    if serde_cbor::to_vec(&envelope).map_err(|_| invalid("activation serialization"))?
        != envelope_bytes
    {
        return Err(invalid("noncanonical activation envelope"));
    }
    let b = &envelope.body;
    if b.schema_version != 1
        || b.profile != "controlled_single_authority"
        || b.profile_version != 1
        || &b.trust != t
        || b.position != 1
        || b.parent_hash != genesis.decision_hash
        || b.activation_height != policy.activation_height
        || b.before_state_root != policy.initial_state_root
        || b.artifact_roots.len() != 8
        || b.artifact_roots.contains_key(&ArtifactRole::Input)
        || b.artifact_roots.get(&ArtifactRole::ExecutionManifest)
            != Some(&policy.execution_manifest_root)
    {
        return Err(invalid("activation policy/profile/initial anchor mismatch"));
    }
    hex_bytes::<32>(&b.after_state_root)?;
    issuer
        .verify_strict(
            &initial_activation_signing_bytes(b)?,
            &Signature::from_bytes(&hex_bytes::<64>(&envelope.signature_hex)?),
        )
        .map_err(|_| invalid("untrusted activation issuer signature"))?;
    let proposal = &evidence.proposal.body;
    let record = &proposal.record;
    let digest = blake3::hash(envelope_bytes).to_hex().to_string();
    let non_input = record
        .roots
        .iter()
        .filter(|(role, _)| **role != ArtifactRole::Input)
        .map(|(role, hash)| (*role, hash.clone()))
        .collect::<BTreeMap<_, _>>();
    if proposal.position != b.position
        || proposal.parent_hash != b.parent_hash
        || record.payload_digest != digest
        || record.roots.get(&ArtifactRole::Input) != Some(&digest)
        || record.before_state_root != b.before_state_root
        || record.after_state_root != b.after_state_root
        || non_input != b.artifact_roots
        || !record
            .objects
            .iter()
            .any(|o| o.content_hash == digest && o.bytes == envelope_bytes)
    {
        return Err(invalid("activation input/closed record binding mismatch"));
    }
    let qualified_head = verify_evidence(evidence, t, minimum)?;
    Ok(VerifiedInitialActivation {
        body: envelope.body,
        qualified_head,
        envelope_digest: digest,
    })
}

#[cfg(test)]
#[path = "activation_tests.rs"]
mod tests;
