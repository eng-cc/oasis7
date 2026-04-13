use super::*;

impl PosNodeEngine {
    fn expected_player_for_validator(&self, validator_id: &str) -> Result<&str, NodeError> {
        self.validator_players
            .get(validator_id)
            .map(String::as_str)
            .ok_or_else(|| NodeError::Consensus {
                reason: format!("validator player binding missing for {}", validator_id),
            })
    }

    fn normalized_message_player_id(
        &self,
        validator_id: &str,
        message_player_id: &str,
    ) -> Result<String, NodeError> {
        let expected = self.expected_player_for_validator(validator_id)?;
        let normalized = message_player_id.trim();
        if normalized.is_empty() || normalized == "legacy" {
            return Ok(expected.to_string());
        }
        Ok(normalized.to_string())
    }

    pub(super) fn validate_message_player_binding(
        &self,
        validator_id: &str,
        message_player_id: &str,
        label: &str,
    ) -> Result<(), NodeError> {
        let expected = self.expected_player_for_validator(validator_id)?;
        let normalized = self.normalized_message_player_id(validator_id, message_player_id)?;
        if normalized != expected {
            return Err(NodeError::Consensus {
                reason: format!(
                    "{label} player_id mismatch validator_id={} expected={} actual={}",
                    validator_id, expected, normalized
                ),
            });
        }
        Ok(())
    }

    pub(super) fn validate_message_signer_binding(
        &self,
        validator_id: &str,
        message_public_key_hex: Option<&str>,
        label: &str,
    ) -> Result<(), NodeError> {
        let Some(expected_public_key) = self.validator_signers.get(validator_id) else {
            return Ok(());
        };
        let Some(actual_raw) = message_public_key_hex else {
            return Err(NodeError::Consensus {
                reason: format!(
                    "{label} signer binding missing public_key_hex for validator_id={validator_id}"
                ),
            });
        };
        let actual_public_key = normalize_consensus_public_key_hex(
            actual_raw,
            format!("{label}.public_key_hex").as_str(),
        )?;
        if &actual_public_key != expected_public_key {
            return Err(NodeError::Consensus {
                reason: format!(
                    "{label} signer binding mismatch validator_id={} expected={} actual={}",
                    validator_id, expected_public_key, actual_public_key
                ),
            });
        }
        Ok(())
    }

    pub(super) fn broadcast_local_proposal(
        &mut self,
        endpoint: &GossipEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let Some(proposal) = self.pending.as_ref() else {
            return Ok(());
        };
        if proposal.proposer_id != node_id {
            return Ok(());
        }
        if proposal.height <= self.last_broadcast_proposal_height {
            return Ok(());
        }
        let mut message = GossipProposalMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: self.node_player_id.clone(),
            proposer_id: proposal.proposer_id.clone(),
            height: proposal.height,
            slot: proposal.slot,
            epoch: proposal.epoch,
            block_hash: proposal.block_hash.clone(),
            action_root: proposal.action_root.clone(),
            actions: proposal.committed_actions.clone(),
            proposed_at_ms: now_ms,
            public_key_hex: None,
            signature_hex: None,
        };
        if let Some(signer) = self.consensus_signer.as_ref() {
            sign_proposal_message(&mut message, signer)?;
        }
        endpoint.broadcast_proposal(&message)?;
        self.last_broadcast_proposal_height = proposal.height;
        Ok(())
    }

    pub(super) fn broadcast_local_proposal_network(
        &mut self,
        endpoint: &ConsensusNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        if !endpoint.allows_publish() {
            return Ok(());
        }
        let Some(proposal) = self.pending.as_ref() else {
            return Ok(());
        };
        if proposal.proposer_id != node_id {
            return Ok(());
        }
        if proposal.height <= self.last_broadcast_proposal_height {
            return Ok(());
        }
        let mut message = GossipProposalMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: self.node_player_id.clone(),
            proposer_id: proposal.proposer_id.clone(),
            height: proposal.height,
            slot: proposal.slot,
            epoch: proposal.epoch,
            block_hash: proposal.block_hash.clone(),
            action_root: proposal.action_root.clone(),
            actions: proposal.committed_actions.clone(),
            proposed_at_ms: now_ms,
            public_key_hex: None,
            signature_hex: None,
        };
        if let Some(signer) = self.consensus_signer.as_ref() {
            sign_proposal_message(&mut message, signer)?;
        }
        endpoint.publish_proposal(&message)?;
        self.last_broadcast_proposal_height = proposal.height;
        Ok(())
    }

    pub(super) fn broadcast_local_attestation(
        &mut self,
        endpoint: &GossipEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let Some(proposal) = self.pending.as_ref() else {
            return Ok(());
        };
        let Some(attestation) = proposal.attestations.get(node_id) else {
            return Ok(());
        };
        if proposal.height <= self.last_broadcast_local_attestation_height {
            return Ok(());
        }

        let mut message = GossipAttestationMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: self.node_player_id.clone(),
            validator_id: attestation.validator_id.clone(),
            height: proposal.height,
            slot: proposal.slot,
            epoch: proposal.epoch,
            block_hash: proposal.block_hash.clone(),
            approve: attestation.approve,
            source_epoch: attestation.source_epoch,
            target_epoch: attestation.target_epoch,
            voted_at_ms: now_ms,
            reason: attestation.reason.clone(),
            public_key_hex: None,
            signature_hex: None,
        };
        if let Some(signer) = self.consensus_signer.as_ref() {
            sign_attestation_message(&mut message, signer)?;
        }
        endpoint.broadcast_attestation(&message)?;
        self.last_broadcast_local_attestation_height = proposal.height;
        Ok(())
    }

    pub(super) fn broadcast_local_attestation_network(
        &mut self,
        endpoint: &ConsensusNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        if !endpoint.allows_publish() {
            return Ok(());
        }
        let Some(proposal) = self.pending.as_ref() else {
            return Ok(());
        };
        let Some(attestation) = proposal.attestations.get(node_id) else {
            return Ok(());
        };
        if proposal.height <= self.last_broadcast_local_attestation_height {
            return Ok(());
        }

        let mut message = GossipAttestationMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: self.node_player_id.clone(),
            validator_id: attestation.validator_id.clone(),
            height: proposal.height,
            slot: proposal.slot,
            epoch: proposal.epoch,
            block_hash: proposal.block_hash.clone(),
            approve: attestation.approve,
            source_epoch: attestation.source_epoch,
            target_epoch: attestation.target_epoch,
            voted_at_ms: now_ms,
            reason: attestation.reason.clone(),
            public_key_hex: None,
            signature_hex: None,
        };
        if let Some(signer) = self.consensus_signer.as_ref() {
            sign_attestation_message(&mut message, signer)?;
        }
        endpoint.publish_attestation(&message)?;
        self.last_broadcast_local_attestation_height = proposal.height;
        Ok(())
    }

    pub(super) fn broadcast_local_commit(
        &mut self,
        endpoint: &GossipEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        decision: &PosDecision,
    ) -> Result<(), NodeError> {
        if !matches!(decision.status, PosConsensusStatus::Committed) {
            return Ok(());
        }
        if decision.height <= self.last_broadcast_committed_height {
            return Ok(());
        }
        let (execution_block_hash, execution_state_root) =
            self.commit_execution_binding_for_height(decision.height)?;
        let mut message = GossipCommitMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: self.node_player_id.clone(),
            height: decision.height,
            slot: decision.slot,
            epoch: decision.epoch,
            block_hash: decision.block_hash.clone(),
            action_root: decision.action_root.clone(),
            actions: decision.committed_actions.clone(),
            committed_at_ms: now_ms,
            execution_block_hash: execution_block_hash.map(str::to_string),
            execution_state_root: execution_state_root.map(str::to_string),
            public_key_hex: None,
            signature_hex: None,
        };
        if let Some(signer) = self.consensus_signer.as_ref() {
            sign_commit_message(&mut message, signer)?;
        }
        endpoint.broadcast_commit(&message)?;
        self.last_broadcast_committed_height = decision.height;
        Ok(())
    }

    pub(super) fn broadcast_local_commit_network(
        &mut self,
        endpoint: &ConsensusNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        decision: &PosDecision,
    ) -> Result<(), NodeError> {
        if !endpoint.allows_publish() {
            return Ok(());
        }
        if !matches!(decision.status, PosConsensusStatus::Committed) {
            return Ok(());
        }
        if decision.height <= self.last_broadcast_committed_height {
            return Ok(());
        }
        let (execution_block_hash, execution_state_root) =
            self.commit_execution_binding_for_height(decision.height)?;
        let mut message = GossipCommitMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            player_id: self.node_player_id.clone(),
            height: decision.height,
            slot: decision.slot,
            epoch: decision.epoch,
            block_hash: decision.block_hash.clone(),
            action_root: decision.action_root.clone(),
            actions: decision.committed_actions.clone(),
            committed_at_ms: now_ms,
            execution_block_hash: execution_block_hash.map(str::to_string),
            execution_state_root: execution_state_root.map(str::to_string),
            public_key_hex: None,
            signature_hex: None,
        };
        if let Some(signer) = self.consensus_signer.as_ref() {
            sign_commit_message(&mut message, signer)?;
        }
        endpoint.publish_commit(&message)?;
        self.last_broadcast_committed_height = decision.height;
        Ok(())
    }
}
