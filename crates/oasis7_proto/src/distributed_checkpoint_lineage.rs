//! Domain-separated quorum evidence binding a retained checkpoint (C) to a
//! signed network head (H).
//!
//! This is deliberately a sidecar proof.  It is not a transport route
//! attestation, a provider endorsement, or a replacement for
//! `WorldHeadProofV1`.

use std::collections::BTreeSet;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1: u16 = 1;
pub const CHECKPOINT_LINEAGE_VOTE_DOMAIN_V1: &str = "oasis7.checkpoint_lineage_vote.v1";
pub const CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1: &str =
    "checkpoint_lineage_evidence_only_not_light_client_or_mainnet_readiness";
pub const CHECKPOINT_LINEAGE_DESCRIPTOR_BINDING_DOMAIN_V1: &str =
    "oasis7.checkpoint_descriptor_binding.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointLineageVoteV1 {
    pub validator_id: String,
    pub signature_scheme: String,
    pub signature_evidence_hash: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointLineageCheckpointV1 {
    pub height: u64,
    pub block_hash: String,
    pub state_root: String,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub descriptor_digest: String,
    pub manifest_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointLineageHeadV1 {
    pub height: u64,
    pub block_hash: String,
    pub state_root: String,
    pub execution_block_hash: String,
    pub execution_state_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointLineageEnvelopeV1 {
    pub schema_version: u16,
    pub claim_boundary: String,
    pub world_id: String,
    pub checkpoint: CheckpointLineageCheckpointV1,
    pub head: CheckpointLineageHeadV1,
    pub validator_set_hash: String,
    pub total_stake: u64,
    pub required_stake: u64,
    pub round_id: String,
    pub votes: Vec<CheckpointLineageVoteV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointLineageValidatorV1 {
    pub validator_id: String,
    pub stake_weight: u64,
    pub public_key_hex: String,
}

#[derive(Serialize)]
struct CheckpointLineageVoteSigningTuple<'a> {
    domain: &'a str,
    schema_version: u16,
    claim_boundary: &'a str,
    world_id: &'a str,
    checkpoint_height: u64,
    checkpoint_block_hash: &'a str,
    checkpoint_state_root: &'a str,
    checkpoint_execution_block_hash: &'a str,
    checkpoint_execution_state_root: &'a str,
    descriptor_digest: &'a str,
    manifest_size: u64,
    head_height: u64,
    head_block_hash: &'a str,
    head_state_root: &'a str,
    head_execution_block_hash: &'a str,
    head_execution_state_root: &'a str,
    validator_set_hash: &'a str,
    total_stake: u64,
    required_stake: u64,
    round_id: &'a str,
}

pub fn checkpoint_lineage_vote_signing_payload(
    envelope: &CheckpointLineageEnvelopeV1,
) -> Result<Vec<u8>, String> {
    serde_cbor::to_vec(&CheckpointLineageVoteSigningTuple {
        domain: CHECKPOINT_LINEAGE_VOTE_DOMAIN_V1,
        schema_version: envelope.schema_version,
        claim_boundary: envelope.claim_boundary.as_str(),
        world_id: envelope.world_id.as_str(),
        checkpoint_height: envelope.checkpoint.height,
        checkpoint_block_hash: envelope.checkpoint.block_hash.as_str(),
        checkpoint_state_root: envelope.checkpoint.state_root.as_str(),
        checkpoint_execution_block_hash: envelope.checkpoint.execution_block_hash.as_str(),
        checkpoint_execution_state_root: envelope.checkpoint.execution_state_root.as_str(),
        descriptor_digest: envelope.checkpoint.descriptor_digest.as_str(),
        manifest_size: envelope.checkpoint.manifest_size,
        head_height: envelope.head.height,
        head_block_hash: envelope.head.block_hash.as_str(),
        head_state_root: envelope.head.state_root.as_str(),
        head_execution_block_hash: envelope.head.execution_block_hash.as_str(),
        head_execution_state_root: envelope.head.execution_state_root.as_str(),
        validator_set_hash: envelope.validator_set_hash.as_str(),
        total_stake: envelope.total_stake,
        required_stake: envelope.required_stake,
        round_id: envelope.round_id.as_str(),
    })
    .map_err(|err| format!("encode checkpoint lineage vote payload: {err}"))
}

/// Compute the descriptor digest bound into the envelope.  The ordered blob
/// list is part of the v1 contract; callers must not replace it with a route,
/// provider, or message-content hash.
pub fn checkpoint_lineage_descriptor_digest(
    world_id: &str,
    checkpoint_height: u64,
    checkpoint_block_hash: &str,
    execution_block_hash: &str,
    execution_state_root: &str,
    manifest_hash: &str,
    manifest_size: u64,
    blobs: &[(String, u64)],
) -> Result<String, String> {
    if blobs
        .iter()
        .any(|(hash, size)| hash.trim().is_empty() || *size == 0)
    {
        return Err("checkpoint descriptor blob hash/size is invalid".to_string());
    }
    let bytes = serde_cbor::to_vec(&(
        CHECKPOINT_LINEAGE_DESCRIPTOR_BINDING_DOMAIN_V1,
        world_id,
        checkpoint_height,
        checkpoint_block_hash,
        execution_block_hash,
        execution_state_root,
        manifest_hash,
        manifest_size,
        blobs,
    ))
    .map_err(|err| format!("encode checkpoint descriptor binding: {err}"))?;
    Ok(blake3::hash(bytes.as_slice()).to_hex().to_string())
}

impl CheckpointLineageEnvelopeV1 {
    pub fn validate_contract(&self) -> Result<(), String> {
        if self.schema_version != CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1 {
            return Err(format!(
                "unsupported checkpoint lineage envelope schema: {}",
                self.schema_version
            ));
        }
        if self.claim_boundary != CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1 {
            return Err("unexpected checkpoint lineage claim boundary".to_string());
        }
        require_non_empty("world_id", &self.world_id)?;
        require_non_empty("checkpoint.block_hash", &self.checkpoint.block_hash)?;
        require_non_empty("checkpoint.state_root", &self.checkpoint.state_root)?;
        require_non_empty(
            "checkpoint.execution_block_hash",
            &self.checkpoint.execution_block_hash,
        )?;
        require_non_empty(
            "checkpoint.execution_state_root",
            &self.checkpoint.execution_state_root,
        )?;
        require_non_empty(
            "checkpoint.descriptor_digest",
            &self.checkpoint.descriptor_digest,
        )?;
        require_non_empty("head.block_hash", &self.head.block_hash)?;
        require_non_empty("head.state_root", &self.head.state_root)?;
        require_non_empty("head.execution_block_hash", &self.head.execution_block_hash)?;
        require_non_empty("head.execution_state_root", &self.head.execution_state_root)?;
        require_non_empty("validator_set_hash", &self.validator_set_hash)?;
        require_non_empty("round_id", &self.round_id)?;
        if self.checkpoint.height == 0 || self.checkpoint.height > self.head.height {
            return Err(format!(
                "checkpoint height must be positive and <= head height: checkpoint={} head={}",
                self.checkpoint.height, self.head.height
            ));
        }
        if self.head.height == 0 {
            return Err("head height must be positive".to_string());
        }
        if self.total_stake == 0
            || self.required_stake == 0
            || self.required_stake > self.total_stake
        {
            return Err(format!(
                "invalid checkpoint lineage stake threshold: total={} required={}",
                self.total_stake, self.required_stake
            ));
        }
        if self.votes.is_empty() {
            return Err("checkpoint lineage envelope must include votes".to_string());
        }
        Ok(())
    }

    pub fn verify_votes(&self, validators: &[CheckpointLineageValidatorV1]) -> Result<u64, String> {
        self.validate_contract()?;
        let payload = checkpoint_lineage_vote_signing_payload(self)?;
        let evidence_hash = blake3::hash(payload.as_slice()).to_hex().to_string();
        let mut seen = BTreeSet::new();
        let mut signed_stake = 0_u64;
        for vote in &self.votes {
            if !seen.insert(vote.validator_id.as_str()) {
                return Err(format!(
                    "duplicate checkpoint lineage vote: {}",
                    vote.validator_id
                ));
            }
            let validator = validators
                .iter()
                .find(|candidate| candidate.validator_id == vote.validator_id)
                .ok_or_else(|| {
                    format!(
                        "checkpoint lineage vote validator is not configured: {}",
                        vote.validator_id
                    )
                })?;
            if vote.signature_scheme != "ed25519" {
                return Err(format!(
                    "unsupported checkpoint lineage vote signature scheme: {}",
                    vote.signature_scheme
                ));
            }
            if vote.signature_evidence_hash != evidence_hash {
                return Err(format!(
                    "checkpoint lineage vote evidence hash mismatch for {}",
                    vote.validator_id
                ));
            }
            let public_key_bytes =
                hex::decode(validator.public_key_hex.as_str()).map_err(|err| {
                    format!(
                        "decode checkpoint lineage signer {}: {err}",
                        vote.validator_id
                    )
                })?;
            let public_key_bytes: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
                format!(
                    "checkpoint lineage signer {} key must be 32 bytes",
                    vote.validator_id
                )
            })?;
            let signature_bytes = hex::decode(vote.signature_hex.as_str()).map_err(|err| {
                format!(
                    "decode checkpoint lineage signature {}: {err}",
                    vote.validator_id
                )
            })?;
            let signature = Signature::from_slice(signature_bytes.as_slice()).map_err(|err| {
                format!(
                    "decode checkpoint lineage signature {}: {err}",
                    vote.validator_id
                )
            })?;
            VerifyingKey::from_bytes(&public_key_bytes)
                .map_err(|err| format!("decode checkpoint lineage verifier: {err}"))?
                .verify(payload.as_slice(), &signature)
                .map_err(|err| {
                    format!(
                        "verify checkpoint lineage vote {}: {err}",
                        vote.validator_id
                    )
                })?;
            signed_stake = signed_stake
                .checked_add(validator.stake_weight)
                .ok_or_else(|| "checkpoint lineage signed stake overflow".to_string())?;
        }
        if signed_stake < self.required_stake {
            return Err(format!(
                "checkpoint lineage signed stake below threshold: signed={} required={}",
                signed_stake, self.required_stake
            ));
        }
        if signed_stake > self.total_stake {
            return Err(format!(
                "checkpoint lineage signed stake exceeds declared total: signed={} total={}",
                signed_stake, self.total_stake
            ));
        }
        Ok(signed_stake)
    }

    /// Verify the envelope against the node's configured authority.  The
    /// validator-set hash is intentionally supplied by the caller: this
    /// protocol does not invent a set identifier or silently substitute the
    /// separate world-finality hash contract.
    pub fn verify_against_authority(
        &self,
        expected_world_id: &str,
        expected_head: &CheckpointLineageHeadV1,
        validators: &[CheckpointLineageValidatorV1],
        expected_validator_set_hash: &str,
        expected_total_stake: u64,
        expected_required_stake: u64,
    ) -> Result<u64, String> {
        self.validate_contract()?;
        if self.world_id != expected_world_id {
            return Err(format!(
                "checkpoint lineage world mismatch: expected={} actual={}",
                expected_world_id, self.world_id
            ));
        }
        if &self.head != expected_head {
            return Err("checkpoint lineage head identity mismatch".to_string());
        }
        if self.validator_set_hash != expected_validator_set_hash {
            return Err("checkpoint lineage validator-set hash mismatch".to_string());
        }
        if self.total_stake != expected_total_stake
            || self.required_stake != expected_required_stake
        {
            return Err(format!(
                "checkpoint lineage stake authority mismatch: envelope=({}, {}) configured=({}, {})",
                self.total_stake,
                self.required_stake,
                expected_total_stake,
                expected_required_stake
            ));
        }
        self.verify_votes(validators)
    }
}

fn require_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_payload_is_domain_separated_and_stable() {
        let envelope = CheckpointLineageEnvelopeV1 {
            schema_version: CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
            claim_boundary: CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1.to_string(),
            world_id: "world".to_string(),
            checkpoint: CheckpointLineageCheckpointV1 {
                height: 2,
                block_hash: "c-block".to_string(),
                state_root: "c-state".to_string(),
                execution_block_hash: "c-execution".to_string(),
                execution_state_root: "c-execution-state".to_string(),
                descriptor_digest: "descriptor".to_string(),
                manifest_size: 3,
            },
            head: CheckpointLineageHeadV1 {
                height: 4,
                block_hash: "h-block".to_string(),
                state_root: "h-state".to_string(),
                execution_block_hash: "h-execution".to_string(),
                execution_state_root: "h-execution-state".to_string(),
            },
            validator_set_hash: "set".to_string(),
            total_stake: 100,
            required_stake: 67,
            round_id: "round".to_string(),
            votes: vec![],
        };
        let first = checkpoint_lineage_vote_signing_payload(&envelope).unwrap();
        let second = checkpoint_lineage_vote_signing_payload(&envelope).unwrap();
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn rejects_checkpoint_above_head() {
        let mut envelope = CheckpointLineageEnvelopeV1 {
            schema_version: CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
            claim_boundary: CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1.to_string(),
            world_id: "world".to_string(),
            checkpoint: CheckpointLineageCheckpointV1 {
                height: 5,
                block_hash: "c-block".to_string(),
                state_root: "c-state".to_string(),
                execution_block_hash: "c-execution".to_string(),
                execution_state_root: "c-execution-state".to_string(),
                descriptor_digest: "descriptor".to_string(),
                manifest_size: 3,
            },
            head: CheckpointLineageHeadV1 {
                height: 4,
                block_hash: "h-block".to_string(),
                state_root: "h-state".to_string(),
                execution_block_hash: "h-execution".to_string(),
                execution_state_root: "h-execution-state".to_string(),
            },
            validator_set_hash: "set".to_string(),
            total_stake: 100,
            required_stake: 67,
            round_id: "round".to_string(),
            votes: vec![CheckpointLineageVoteV1 {
                validator_id: "v".to_string(),
                signature_scheme: "ed25519".to_string(),
                signature_evidence_hash: String::new(),
                signature_hex: String::new(),
            }],
        };
        assert!(envelope.validate_contract().is_err());
        envelope.checkpoint.height = 3;
        assert!(envelope.validate_contract().is_ok());
    }
}
