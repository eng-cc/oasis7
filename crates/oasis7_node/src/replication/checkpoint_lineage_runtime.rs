use super::*;

impl ReplicationRuntime {
    pub(crate) fn writer_last_replicated_height(&self) -> u64 {
        self.writer_state.last_replicated_height
    }

    pub(crate) fn load_commit_message_by_height(
        &self,
        world_id: &str,
        height: u64,
    ) -> Result<Option<GossipReplicationMessage>, NodeError> {
        load_commit_message_from_root(self.config.root_dir.as_path(), world_id, height)
    }

    pub(crate) fn persist_checkpoint_lineage_envelope(
        &self,
        envelope: &CheckpointLineageEnvelopeV1,
    ) -> Result<String, NodeError> {
        envelope
            .validate_contract()
            .map_err(|reason| NodeError::Replication { reason })?;
        let key = checkpoint_lineage_cache_key(envelope)
            .map_err(|reason| NodeError::Replication { reason })?;
        let root = self.config.root_dir.join("checkpoint-lineage");
        fs::create_dir_all(&root).map_err(|err| NodeError::Replication {
            reason: format!(
                "create checkpoint lineage cache {} failed: {err}",
                root.display()
            ),
        })?;
        let path = root.join(format!("{key}.json"));
        write_json_compact(path.as_path(), envelope)?;
        Ok(key)
    }

    pub(crate) fn load_checkpoint_lineage_envelope(
        &self,
        key: &str,
    ) -> Result<Option<CheckpointLineageEnvelopeV1>, NodeError> {
        if key.trim().is_empty()
            || key.len() != 64
            || !key.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(NodeError::Replication {
                reason: "invalid checkpoint lineage cache key".to_string(),
            });
        }
        let path = self
            .config
            .root_dir
            .join("checkpoint-lineage")
            .join(format!("{key}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path.as_path()).map_err(|err| NodeError::Replication {
            reason: format!(
                "read checkpoint lineage cache {} failed: {err}",
                path.display()
            ),
        })?;
        let envelope = serde_json::from_slice::<CheckpointLineageEnvelopeV1>(bytes.as_slice())
            .map_err(|err| NodeError::Replication {
                reason: format!(
                    "decode checkpoint lineage cache {} failed: {err}",
                    path.display()
                ),
            })?;
        envelope
            .validate_contract()
            .map_err(|reason| NodeError::Replication { reason })?;
        let computed_key = checkpoint_lineage_cache_key(&envelope)
            .map_err(|reason| NodeError::Replication { reason })?;
        if computed_key != key {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint lineage cache key mismatch: expected={key} actual={computed_key}"
                ),
            });
        }
        Ok(Some(envelope))
    }

    pub(crate) fn attach_checkpoint_lineage_envelope(
        &mut self,
        node_id: &str,
        world_id: &str,
        envelope: &CheckpointLineageEnvelopeV1,
    ) -> Result<Option<GossipReplicationMessage>, NodeError> {
        let Some(signer) = self.signer.clone() else {
            return Ok(None);
        };
        let Some(message) =
            self.load_commit_message_by_height(world_id, envelope.checkpoint.height)?
        else {
            return Ok(None);
        };
        if message.node_id != node_id
            || message.world_id != world_id
            || message.record.writer_id != signer.public_key_hex
        {
            return Ok(None);
        }
        let mut payload = serde_json::from_slice::<ReplicatedCommitPayload>(
            message.payload.as_slice(),
        )
        .map_err(|err| NodeError::Replication {
            reason: format!("decode source replication payload for lineage attach failed: {err}"),
        })?;
        if payload.world_id != world_id || payload.height != envelope.checkpoint.height {
            return Ok(None);
        }
        if let Some(existing) = payload.lineage_envelope.as_ref() {
            if existing == envelope {
                return Ok(Some(message));
            }
            return Err(NodeError::Replication {
                reason: format!(
                    "source checkpoint lineage envelope conflict at height {}",
                    envelope.checkpoint.height
                ),
            });
        }
        let Some(descriptor) = payload.execution_checkpoint.as_ref() else {
            return Ok(None);
        };
        if descriptor.height != payload.height
            || descriptor.execution_block_hash
                != payload.execution_block_hash.clone().unwrap_or_default()
            || descriptor.execution_state_root
                != payload.execution_state_root.clone().unwrap_or_default()
        {
            return Ok(None);
        }
        payload.lineage_envelope = Some(envelope.clone());
        let payload_bytes = serde_json::to_vec(&payload).map_err(|err| NodeError::Replication {
            reason: format!("serialize source lineage payload failed: {err}"),
        })?;
        let (writer_epoch, sequence) = self.next_local_record_position(&signer.public_key_hex)?;
        let path = format!("{COMMIT_FILE_PREFIX}/{:020}.json", payload.height);
        let record = build_replication_record_with_epoch(
            world_id,
            signer.public_key_hex.as_str(),
            writer_epoch,
            sequence,
            path.as_str(),
            payload_bytes.as_slice(),
            message.record.updated_at_ms,
        )
        .map_err(distfs_error_to_node_error)?;
        let mut amended = GossipReplicationMessage {
            version: REPLICATION_VERSION,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            record,
            payload: payload_bytes,
            public_key_hex: Some(signer.public_key_hex.clone()),
            signature_hex: None,
        };
        amended.signature_hex = Some(sign_replication_message(&amended, &signer)?);
        apply_replication_record(
            &self.store,
            &mut self.guard,
            &amended.record,
            amended.payload.as_slice(),
        )
        .map_err(distfs_error_to_node_error)?;
        self.writer_state.writer_epoch = amended.record.writer_epoch;
        self.writer_state.last_sequence = amended.record.sequence;
        self.writer_state.last_replicated_height =
            self.writer_state.last_replicated_height.max(payload.height);
        self.persist_state(node_id)?;
        self.persist_commit_message(payload.height, &amended)?;
        Ok(Some(amended))
    }
}
