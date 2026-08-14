use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use oasis7_proto::distributed_checkpoint_lineage::{
    CheckpointLineageEnvelopeV1, CheckpointLineageHeadV1, CheckpointLineageValidatorV1,
    CheckpointLineageVoteV1, checkpoint_lineage_descriptor_digest,
    checkpoint_lineage_vote_signing_payload,
};
use serde_json::Value;

use crate::NodeError;
use crate::execution_hook::NodeExecutionCheckpointDescriptor;
use crate::gossip_udp::GossipCommitMessage;
use crate::replication_state_reconcile::ReplicationCommitPayload;

#[derive(Debug, Clone, Default)]
pub(crate) struct CheckpointLineageState {
    pub(crate) votes: BTreeMap<String, BTreeMap<String, CheckpointLineageVoteV1>>,
    pub(crate) envelopes: BTreeMap<String, CheckpointLineageEnvelopeV1>,
}

pub(crate) fn checkpoint_lineage_cache_key(
    envelope: &CheckpointLineageEnvelopeV1,
) -> Result<String, String> {
    let bytes = serde_cbor::to_vec(&(
        envelope.schema_version,
        envelope.claim_boundary.as_str(),
        envelope.world_id.as_str(),
        (
            envelope.checkpoint.height,
            envelope.checkpoint.block_hash.as_str(),
            envelope.checkpoint.state_root.as_str(),
            envelope.checkpoint.execution_block_hash.as_str(),
            envelope.checkpoint.execution_state_root.as_str(),
            envelope.checkpoint.descriptor_digest.as_str(),
            envelope.checkpoint.manifest_size,
        ),
        (
            envelope.head.height,
            envelope.head.block_hash.as_str(),
            envelope.head.state_root.as_str(),
            envelope.head.execution_block_hash.as_str(),
            envelope.head.execution_state_root.as_str(),
        ),
        envelope.validator_set_hash.as_str(),
        envelope.total_stake,
        envelope.required_stake,
        envelope.round_id.as_str(),
    ))
    .map_err(|err| format!("encode checkpoint lineage cache key: {err}"))?;
    Ok(oasis7_distfs::blake3_hex(bytes.as_slice()))
}

pub(crate) fn checkpoint_lineage_authority(
    validators: &BTreeMap<String, u64>,
    validator_signers: &BTreeMap<String, String>,
) -> Result<Vec<CheckpointLineageValidatorV1>, String> {
    validators
        .iter()
        .map(|(validator_id, stake_weight)| {
            let public_key_hex = validator_signers
                .get(validator_id)
                .filter(|key| !key.trim().is_empty())
                .ok_or_else(|| format!("validator signer binding missing: {validator_id}"))?;
            Ok(CheckpointLineageValidatorV1 {
                validator_id: validator_id.clone(),
                stake_weight: *stake_weight,
                public_key_hex: public_key_hex.clone(),
            })
        })
        .collect()
}

pub(crate) fn verify_checkpoint_lineage_vote(
    envelope: &CheckpointLineageEnvelopeV1,
    vote: &CheckpointLineageVoteV1,
    authority: &[CheckpointLineageValidatorV1],
) -> Result<u64, String> {
    envelope.validate_contract()?;
    if vote.signature_scheme != "ed25519" {
        return Err(format!(
            "unsupported checkpoint lineage vote signature scheme: {}",
            vote.signature_scheme
        ));
    }
    let validator = authority
        .iter()
        .find(|candidate| candidate.validator_id == vote.validator_id)
        .ok_or_else(|| {
            format!(
                "checkpoint lineage vote validator is not configured: {}",
                vote.validator_id
            )
        })?;
    let payload = checkpoint_lineage_vote_signing_payload(envelope)?;
    let evidence_hash = oasis7_distfs::blake3_hex(payload.as_slice());
    if vote.signature_evidence_hash != evidence_hash {
        return Err(format!(
            "checkpoint lineage vote evidence hash mismatch for {}",
            vote.validator_id
        ));
    }
    let public_key_bytes = hex::decode(validator.public_key_hex.as_str()).map_err(|err| {
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
    Ok(validator.stake_weight)
}

/// Extract the source-authored lineage sidecar from an already validated
/// replication payload.  This helper never creates or changes the envelope;
/// malformed or absent sidecars are reported so callers can fail closed.
pub(crate) fn lineage_envelope_from_payload(
    payload: &[u8],
) -> Result<Option<CheckpointLineageEnvelopeV1>, NodeError> {
    let value: Value = serde_json::from_slice(payload).map_err(|err| NodeError::Replication {
        reason: format!("decode replication payload for lineage sidecar: {err}"),
    })?;
    let Some(sidecar) = value.get("lineage_envelope") else {
        return Ok(None);
    };
    if sidecar.is_null() {
        return Ok(None);
    }
    serde_json::from_value(sidecar.clone())
        .map(Some)
        .map_err(|err| NodeError::Replication {
            reason: format!("decode checkpoint lineage sidecar: {err}"),
        })
}

/// Build the exact H identity that was authenticated by the consensus commit
/// signature.  A lineage envelope must name this H; callers must not derive H
/// from transport connectivity or an unsigned world-head response.
pub(crate) fn lineage_head_from_commit(
    commit: &GossipCommitMessage,
) -> Option<CheckpointLineageHeadV1> {
    Some(CheckpointLineageHeadV1 {
        height: commit.height,
        block_hash: commit.block_hash.clone(),
        state_root: commit.execution_state_root.clone()?,
        execution_block_hash: commit.execution_block_hash.clone()?,
        execution_state_root: commit.execution_state_root.clone()?,
    })
}

/// Verify a source-authored C->H certificate against the local validator
/// authority and the exact checkpoint descriptor carried by a replication
/// payload.  This helper intentionally does not read manifests or create
/// votes: the envelope must already be present in the signed payload.
pub(crate) fn verify_checkpoint_lineage_for_descriptor(
    envelope: &CheckpointLineageEnvelopeV1,
    world_id: &str,
    payload: &ReplicationCommitPayload,
    descriptor: &NodeExecutionCheckpointDescriptor,
    expected_head: &CheckpointLineageHeadV1,
    validators: &BTreeMap<String, u64>,
    validator_signers: &BTreeMap<String, String>,
    validator_set_hash: &str,
    total_stake: u64,
    required_stake: u64,
    quarantined_validators: &BTreeSet<String>,
) -> Result<(), String> {
    let execution_block_hash = payload
        .execution_block_hash
        .as_deref()
        .ok_or_else(|| "lineage payload execution block hash is missing".to_string())?;
    let execution_state_root = payload
        .execution_state_root
        .as_deref()
        .ok_or_else(|| "lineage payload execution state root is missing".to_string())?;
    if descriptor.height != payload.height
        || descriptor.execution_block_hash != execution_block_hash
        || descriptor.execution_state_root != execution_state_root
    {
        return Err(
            "lineage checkpoint descriptor does not match payload execution binding".into(),
        );
    }

    let blobs = descriptor
        .blobs
        .iter()
        .map(|blob| (blob.content_hash.clone(), blob.size_bytes))
        .collect::<Vec<_>>();
    let descriptor_digest = checkpoint_lineage_descriptor_digest(
        world_id,
        descriptor.height,
        payload.block_hash.as_str(),
        descriptor.execution_block_hash.as_str(),
        descriptor.execution_state_root.as_str(),
        descriptor.manifest_ref.as_str(),
        descriptor.manifest_size_bytes,
        blobs.as_slice(),
    )?;
    let checkpoint = &envelope.checkpoint;
    if checkpoint.height != descriptor.height
        || checkpoint.block_hash != payload.block_hash
        || checkpoint.state_root != execution_state_root
        || checkpoint.execution_block_hash != descriptor.execution_block_hash
        || checkpoint.execution_state_root != descriptor.execution_state_root
        || checkpoint.descriptor_digest != descriptor_digest
        || checkpoint.manifest_size != descriptor.manifest_size_bytes
    {
        return Err("lineage checkpoint C identity or descriptor digest mismatch".into());
    }
    if envelope
        .votes
        .iter()
        .any(|vote| quarantined_validators.contains(vote.validator_id.as_str()))
    {
        return Err("lineage envelope contains a quarantined validator vote".into());
    }

    let authority = checkpoint_lineage_authority(validators, validator_signers)?;
    envelope
        .verify_against_authority(
            world_id,
            expected_head,
            authority.as_slice(),
            validator_set_hash,
            total_stake,
            required_stake,
        )
        .map(|_| ())
}
