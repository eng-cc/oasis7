use super::*;

use crate::gossip_udp::GossipCheckpointLineageVoteMessage;
use crate::replication_checkpoint_lineage::{
    checkpoint_lineage_authority, checkpoint_lineage_cache_key, lineage_head_from_commit,
    verify_checkpoint_lineage_vote,
};
use oasis7_proto::distributed_checkpoint_lineage::{
    CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1, CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
    CheckpointLineageCheckpointV1, CheckpointLineageEnvelopeV1, CheckpointLineageHeadV1,
    CheckpointLineageVoteV1, checkpoint_lineage_vote_signing_payload,
};

impl PosNodeEngine {
    pub(super) fn authenticated_lineage_head_for(
        &self,
        expected_head: &CheckpointLineageHeadV1,
    ) -> Option<CheckpointLineageHeadV1> {
        if let Some(commit) = self.latest_validated_peer_commit.as_ref() {
            let validator_id = self.validator_id_for_peer_head(commit.node_id.as_str());
            if commit.height >= REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
                && commit.public_key_hex.is_some()
                && commit.signature_hex.is_some()
                && validator_id
                    .as_deref()
                    .map(|id| !self.quarantined_validators.contains(id))
                    .unwrap_or(false)
                && lineage_head_from_commit(commit).as_ref() == Some(expected_head)
            {
                return lineage_head_from_commit(commit);
            }
        }
        self.peer_heads.iter().find_map(|(node_id, head)| {
            let validator_id = self.validator_id_for_peer_head(node_id.as_str())?;
            if self.quarantined_validators.contains(&validator_id)
                || head.height < REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
                || head.public_key_hex.is_none()
                || head.signature_hex.is_none()
            {
                return None;
            }
            let lineage_head = CheckpointLineageHeadV1 {
                height: head.height,
                block_hash: head.block_hash.clone(),
                state_root: head.execution_state_root.clone()?,
                execution_block_hash: head.execution_block_hash.clone()?,
                execution_state_root: head.execution_state_root.clone()?,
            };
            (lineage_head == *expected_head).then_some(lineage_head)
        })
    }

    pub(super) fn sync_missing_replication_commits(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication: Option<&mut ReplicationRuntime>,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        self.sync_missing_replication_commits_with_progress(
            endpoint,
            node_id,
            world_id,
            replication,
            execution_hook,
            None,
            true,
        )
    }

    pub(super) fn build_local_checkpoint_lineage_vote(
        &self,
        world_id: &str,
        checkpoint: CheckpointLineageCheckpointV1,
        head: CheckpointLineageHeadV1,
        round_id: String,
    ) -> Result<GossipCheckpointLineageVoteMessage, NodeError> {
        if world_id.trim().is_empty() {
            return Err(NodeError::Replication {
                reason: "checkpoint lineage vote world binding mismatch".to_string(),
            });
        }
        if checkpoint.height == 0 || checkpoint.height > head.height {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint lineage vote requires C.height <= H.height: C={} H={}",
                    checkpoint.height, head.height
                ),
            });
        }
        let Some(signer) = self.consensus_signer.as_ref() else {
            return Err(NodeError::Replication {
                reason: "checkpoint lineage vote local consensus signer is unavailable".to_string(),
            });
        };
        let Some(expected_public_key) =
            self.validator_signers.get(self.local_validator_id.as_str())
        else {
            return Err(NodeError::Replication {
                reason: "checkpoint lineage vote local validator signer binding is unavailable"
                    .to_string(),
            });
        };
        if self.consensus_signer_public_key.as_deref() != Some(expected_public_key.as_str())
            || self
                .quarantined_validators
                .contains(&self.local_validator_id)
        {
            return Err(NodeError::Replication {
                reason: "checkpoint lineage vote local signer authority mismatch".to_string(),
            });
        }
        let mut envelope = CheckpointLineageEnvelopeV1 {
            schema_version: CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
            claim_boundary: CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1.to_string(),
            world_id: world_id.to_string(),
            checkpoint,
            head,
            validator_set_hash: self.validator_set_hash.clone(),
            total_stake: self.total_stake,
            required_stake: self.required_stake,
            round_id,
            votes: Vec::new(),
        };
        let payload = checkpoint_lineage_vote_signing_payload(&envelope).map_err(|reason| {
            NodeError::Replication {
                reason: format!("encode checkpoint lineage vote failed: {reason}"),
            }
        })?;
        let vote = CheckpointLineageVoteV1 {
            validator_id: self.local_validator_id.clone(),
            signature_scheme: "ed25519".to_string(),
            signature_evidence_hash: oasis7_distfs::blake3_hex(payload.as_slice()),
            signature_hex: signer.sign_domain_payload(payload.as_slice()),
        };
        envelope.votes = vec![vote.clone()];
        Ok(GossipCheckpointLineageVoteMessage {
            version: 1,
            world_id: world_id.to_string(),
            round_id: envelope.round_id,
            checkpoint: envelope.checkpoint,
            head: envelope.head,
            validator_set_hash: envelope.validator_set_hash,
            total_stake: envelope.total_stake,
            required_stake: envelope.required_stake,
            vote,
        })
    }

    pub(super) fn ingest_checkpoint_lineage_vote_message(
        &mut self,
        node_id: &str,
        world_id: &str,
        message: &GossipCheckpointLineageVoteMessage,
        mut replication: Option<&mut ReplicationRuntime>,
    ) -> Result<(), NodeError> {
        if message.version != 1 || message.world_id != world_id {
            return Ok(());
        }
        if self
            .quarantined_validators
            .contains(message.vote.validator_id.as_str())
        {
            return Ok(());
        }
        if message.validator_set_hash != self.validator_set_hash
            || message.total_stake != self.total_stake
            || message.required_stake != self.required_stake
        {
            return Ok(());
        }
        let envelope = CheckpointLineageEnvelopeV1 {
            schema_version:
                oasis7_proto::distributed_checkpoint_lineage::CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
            claim_boundary:
                oasis7_proto::distributed_checkpoint_lineage::CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1
                    .to_string(),
            world_id: message.world_id.clone(),
            checkpoint: message.checkpoint.clone(),
            head: message.head.clone(),
            validator_set_hash: message.validator_set_hash.clone(),
            total_stake: message.total_stake,
            required_stake: message.required_stake,
            round_id: message.round_id.clone(),
            votes: vec![message.vote.clone()],
        };
        let authority = checkpoint_lineage_authority(&self.validators, &self.validator_signers)
            .map_err(|reason| NodeError::Replication { reason })?;
        verify_checkpoint_lineage_vote(&envelope, &message.vote, authority.as_slice())
            .map_err(|reason| NodeError::Replication { reason })?;
        let key = checkpoint_lineage_cache_key(&envelope)
            .map_err(|reason| NodeError::Replication { reason })?;
        if let Some(replication_runtime) = replication.as_deref_mut() {
            if let Some(stored) = replication_runtime.load_checkpoint_lineage_envelope(&key)? {
                if stored
                    .verify_against_authority(
                        world_id,
                        &stored.head,
                        authority.as_slice(),
                        self.validator_set_hash.as_str(),
                        self.total_stake,
                        self.required_stake,
                    )
                    .is_ok()
                {
                    self.lineage_state
                        .envelopes
                        .insert(key.clone(), stored.clone());
                    let _ = replication_runtime
                        .attach_checkpoint_lineage_envelope(node_id, world_id, &stored)?;
                    return Ok(());
                }
            }
        }
        let votes = self.lineage_state.votes.entry(key.clone()).or_default();
        if let Some(previous) = votes.get(message.vote.validator_id.as_str()) {
            if previous != &message.vote {
                self.lineage_state.votes.remove(&key);
                self.lineage_state.envelopes.remove(&key);
                self.quarantined_validators
                    .insert(message.vote.validator_id.clone());
            }
            return Ok(());
        }
        votes.insert(message.vote.validator_id.clone(), message.vote.clone());
        let mut quorum_envelope = envelope.clone();
        quorum_envelope.votes = votes.values().cloned().collect();
        if quorum_envelope
            .verify_against_authority(
                world_id,
                &message.head,
                authority.as_slice(),
                self.validator_set_hash.as_str(),
                self.total_stake,
                self.required_stake,
            )
            .is_err()
        {
            return Ok(());
        }
        self.lineage_state
            .envelopes
            .insert(key, quorum_envelope.clone());
        if let Some(replication_runtime) = replication.as_deref_mut() {
            replication_runtime.persist_checkpoint_lineage_envelope(&quorum_envelope)?;
            let _ = replication_runtime.attach_checkpoint_lineage_envelope(
                node_id,
                world_id,
                &quorum_envelope,
            )?;
        }
        Ok(())
    }
}
