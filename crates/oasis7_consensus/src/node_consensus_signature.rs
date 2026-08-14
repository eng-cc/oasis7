use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::Serialize;

use super::node_consensus_action::NodeConsensusAction;
use super::node_consensus_error::NodeConsensusError;
use super::node_consensus_message::{
    NodeGossipAttestationMessage, NodeGossipCommitMessage, NodeGossipProposalMessage,
};

#[derive(Debug, Clone)]
pub struct NodeConsensusMessageSigner {
    signing_key: SigningKey,
    public_key_hex: String,
}

impl NodeConsensusMessageSigner {
    pub fn new(
        signing_key: SigningKey,
        public_key_hex: String,
    ) -> Result<Self, NodeConsensusError> {
        let expected = hex::encode(signing_key.verifying_key().to_bytes());
        if expected != public_key_hex {
            return Err(NodeConsensusError {
                reason: "consensus signing public key does not match private key".to_string(),
            });
        }
        Ok(Self {
            signing_key,
            public_key_hex,
        })
    }

    pub fn sign_domain_payload(&self, payload: &[u8]) -> String {
        let signature: Signature = self.signing_key.sign(payload);
        hex::encode(signature.to_bytes())
    }
}

pub fn sign_commit_message(
    message: &mut NodeGossipCommitMessage,
    signer: &NodeConsensusMessageSigner,
) -> Result<(), NodeConsensusError> {
    message.public_key_hex = Some(signer.public_key_hex.clone());
    let payload = commit_signing_bytes(message)?;
    let signature: Signature = signer.signing_key.sign(&payload);
    message.signature_hex = Some(hex::encode(signature.to_bytes()));
    Ok(())
}

pub fn sign_proposal_message(
    message: &mut NodeGossipProposalMessage,
    signer: &NodeConsensusMessageSigner,
) -> Result<(), NodeConsensusError> {
    message.public_key_hex = Some(signer.public_key_hex.clone());
    let payload = proposal_signing_bytes(message)?;
    let signature: Signature = signer.signing_key.sign(&payload);
    message.signature_hex = Some(hex::encode(signature.to_bytes()));
    Ok(())
}

pub fn sign_attestation_message(
    message: &mut NodeGossipAttestationMessage,
    signer: &NodeConsensusMessageSigner,
) -> Result<(), NodeConsensusError> {
    message.public_key_hex = Some(signer.public_key_hex.clone());
    let payload = attestation_signing_bytes(message)?;
    let signature: Signature = signer.signing_key.sign(&payload);
    message.signature_hex = Some(hex::encode(signature.to_bytes()));
    Ok(())
}

pub fn verify_commit_message_signature(
    message: &NodeGossipCommitMessage,
    enforce: bool,
) -> Result<(), NodeConsensusError> {
    verify_message_signature(
        message.public_key_hex.as_deref(),
        message.signature_hex.as_deref(),
        commit_signing_bytes(message)?,
        enforce,
        "commit",
    )
}

pub fn verify_proposal_message_signature(
    message: &NodeGossipProposalMessage,
    enforce: bool,
) -> Result<(), NodeConsensusError> {
    verify_message_signature(
        message.public_key_hex.as_deref(),
        message.signature_hex.as_deref(),
        proposal_signing_bytes(message)?,
        enforce,
        "proposal",
    )
}

pub fn verify_attestation_message_signature(
    message: &NodeGossipAttestationMessage,
    enforce: bool,
) -> Result<(), NodeConsensusError> {
    verify_message_signature(
        message.public_key_hex.as_deref(),
        message.signature_hex.as_deref(),
        attestation_signing_bytes(message)?,
        enforce,
        "attestation",
    )
}

#[derive(Debug, Serialize)]
struct CommitSigningPayload<'a> {
    version: u8,
    world_id: &'a str,
    node_id: &'a str,
    player_id: &'a str,
    height: u64,
    slot: u64,
    epoch: u64,
    block_hash: &'a str,
    action_root: &'a str,
    actions: &'a [NodeConsensusAction],
    committed_at_ms: i64,
    execution_block_hash: Option<&'a str>,
    execution_state_root: Option<&'a str>,
    public_key_hex: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ProposalSigningPayload<'a> {
    version: u8,
    world_id: &'a str,
    node_id: &'a str,
    player_id: &'a str,
    proposer_id: &'a str,
    height: u64,
    slot: u64,
    epoch: u64,
    block_hash: &'a str,
    action_root: &'a str,
    actions: &'a [NodeConsensusAction],
    proposed_at_ms: i64,
    public_key_hex: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct AttestationSigningPayload<'a> {
    version: u8,
    world_id: &'a str,
    node_id: &'a str,
    player_id: &'a str,
    validator_id: &'a str,
    height: u64,
    slot: u64,
    epoch: u64,
    block_hash: &'a str,
    approve: bool,
    source_epoch: u64,
    target_epoch: u64,
    voted_at_ms: i64,
    reason: Option<&'a str>,
    public_key_hex: Option<&'a str>,
}

fn commit_signing_bytes(message: &NodeGossipCommitMessage) -> Result<Vec<u8>, NodeConsensusError> {
    to_canonical_cbor(&CommitSigningPayload {
        version: message.version,
        world_id: &message.world_id,
        node_id: &message.node_id,
        player_id: &message.player_id,
        height: message.height,
        slot: message.slot,
        epoch: message.epoch,
        block_hash: &message.block_hash,
        action_root: &message.action_root,
        actions: message.actions.as_slice(),
        committed_at_ms: message.committed_at_ms,
        execution_block_hash: message.execution_block_hash.as_deref(),
        execution_state_root: message.execution_state_root.as_deref(),
        public_key_hex: message.public_key_hex.as_deref(),
    })
}

fn proposal_signing_bytes(
    message: &NodeGossipProposalMessage,
) -> Result<Vec<u8>, NodeConsensusError> {
    to_canonical_cbor(&ProposalSigningPayload {
        version: message.version,
        world_id: &message.world_id,
        node_id: &message.node_id,
        player_id: &message.player_id,
        proposer_id: &message.proposer_id,
        height: message.height,
        slot: message.slot,
        epoch: message.epoch,
        block_hash: &message.block_hash,
        action_root: &message.action_root,
        actions: message.actions.as_slice(),
        proposed_at_ms: message.proposed_at_ms,
        public_key_hex: message.public_key_hex.as_deref(),
    })
}

fn attestation_signing_bytes(
    message: &NodeGossipAttestationMessage,
) -> Result<Vec<u8>, NodeConsensusError> {
    to_canonical_cbor(&AttestationSigningPayload {
        version: message.version,
        world_id: &message.world_id,
        node_id: &message.node_id,
        player_id: &message.player_id,
        validator_id: &message.validator_id,
        height: message.height,
        slot: message.slot,
        epoch: message.epoch,
        block_hash: &message.block_hash,
        approve: message.approve,
        source_epoch: message.source_epoch,
        target_epoch: message.target_epoch,
        voted_at_ms: message.voted_at_ms,
        reason: message.reason.as_deref(),
        public_key_hex: message.public_key_hex.as_deref(),
    })
}

fn verify_message_signature(
    public_key_hex: Option<&str>,
    signature_hex: Option<&str>,
    payload: Vec<u8>,
    enforce: bool,
    label: &str,
) -> Result<(), NodeConsensusError> {
    let (Some(public_key_hex), Some(signature_hex)) = (public_key_hex, signature_hex) else {
        if enforce {
            return Err(NodeConsensusError {
                reason: format!("{label} signature missing"),
            });
        }
        return Ok(());
    };

    let public_key_bytes = decode_hex_array::<32>(public_key_hex, "consensus public key")?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "consensus signature")?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key_bytes).map_err(|err| NodeConsensusError {
            reason: format!("parse consensus public key failed: {err}"),
        })?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(&payload, &signature)
        .map_err(|err| NodeConsensusError {
            reason: format!("verify {label} signature failed: {err}"),
        })
}

fn to_canonical_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, NodeConsensusError> {
    serde_cbor::to_vec(value).map_err(|err| NodeConsensusError {
        reason: format!("serialize consensus signing payload failed: {err}"),
    })
}

fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], NodeConsensusError> {
    let bytes = hex::decode(raw).map_err(|_| NodeConsensusError {
        reason: format!("{label} must be valid hex"),
    })?;
    bytes.try_into().map_err(|_| NodeConsensusError {
        reason: format!("{label} must be {N}-byte hex"),
    })
}
