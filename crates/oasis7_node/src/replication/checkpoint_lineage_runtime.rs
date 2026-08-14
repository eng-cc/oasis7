use super::*;
#[path = "checkpoint_lineage_retention.rs"]
mod checkpoint_lineage_retention;
use oasis7_proto::distributed_checkpoint_lineage::checkpoint_lineage_descriptor_digest;

impl ReplicationRuntime {
    pub(crate) fn persist_local_checkpoint_message_for_lineage(
        &self,
        node_id: &str,
        world_id: &str,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        self.ensure_checkpoint_lineage_healthy()?;
        if message.node_id != node_id
            || message.world_id != world_id
            || message.record.world_id != world_id
        {
            return Ok(());
        }
        let Some(signer) = self.signer.as_ref() else {
            return Ok(());
        };
        if message.record.writer_id != signer.public_key_hex
            || message.public_key_hex.as_deref() != Some(signer.public_key_hex.as_str())
            || message.signature_hex.is_none()
        {
            return Ok(());
        }
        let payload = serde_json::from_slice::<ReplicatedCommitPayload>(message.payload.as_slice())
            .map_err(|err| NodeError::Replication {
                reason: format!("decode local checkpoint lineage payload failed: {err}"),
            })?;
        if payload.world_id != world_id
            || payload.node_id != node_id
            || payload.execution_checkpoint.is_none()
        {
            return Ok(());
        }
        verify_replication_message_signature(message)?;
        if oasis7_distfs::blake3_hex(message.payload.as_slice()) != message.record.content_hash {
            return Err(NodeError::Replication {
                reason: "local checkpoint lineage payload hash mismatch".to_string(),
            });
        }
        let path = self
            .config
            .root_dir
            .join("checkpoint-lineage")
            .join(format!("source-{}.json", payload.height));
        if let Some(existing) =
            self.load_checkpoint_lineage_source_message_by_height(world_id, payload.height)?
        {
            if existing.record.content_hash != message.record.content_hash {
                return Err(NodeError::Replication {
                    reason: format!(
                        "local checkpoint lineage message conflict at height {}",
                        payload.height
                    ),
                });
            }
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| NodeError::Replication {
                reason: format!(
                    "create checkpoint lineage source cache {} failed: {err}",
                    parent.display()
                ),
            })?;
        }
        write_json_compact(path.as_path(), message)?;
        self.reconcile_checkpoint_lineage_retention()
    }

    pub(crate) fn load_checkpoint_lineage_source_message_by_height(
        &self,
        world_id: &str,
        height: u64,
    ) -> Result<Option<GossipReplicationMessage>, NodeError> {
        if height == 0 {
            return Ok(None);
        }
        let path = self
            .config
            .root_dir
            .join("checkpoint-lineage")
            .join(format!("source-{height}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path.as_path()).map_err(|err| NodeError::Replication {
            reason: format!(
                "read checkpoint lineage source cache {} failed: {err}",
                path.display()
            ),
        })?;
        let message = serde_json::from_slice::<GossipReplicationMessage>(bytes.as_slice())
            .map_err(|err| NodeError::Replication {
                reason: format!(
                    "decode checkpoint lineage source cache {} failed: {err}",
                    path.display()
                ),
            })?;
        if message.world_id != world_id {
            return Ok(None);
        }
        Ok(Some(message))
    }

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
        self.ensure_checkpoint_lineage_healthy()?;
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
        self.reconcile_checkpoint_lineage_retention()?;
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
        self.ensure_checkpoint_lineage_healthy()?;
        envelope
            .validate_contract()
            .map_err(|reason| NodeError::Replication { reason })?;
        let Some(signer) = self.signer.clone() else {
            return Ok(None);
        };
        let (message, source_cache_only) = if let Some(message) =
            self.load_commit_message_by_height(world_id, envelope.checkpoint.height)?
        {
            (message, false)
        } else if let Some(message) = self.load_checkpoint_lineage_source_message_by_height(
            world_id,
            envelope.checkpoint.height,
        )? {
            (message, true)
        } else {
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
        let descriptor_digest = checkpoint_lineage_descriptor_digest(
            world_id,
            descriptor.height,
            payload.block_hash.as_str(),
            descriptor.execution_block_hash.as_str(),
            descriptor.execution_state_root.as_str(),
            descriptor.manifest_ref.as_str(),
            descriptor.manifest_size_bytes,
            &descriptor
                .blobs
                .iter()
                .map(|blob| (blob.content_hash.clone(), blob.size_bytes))
                .collect::<Vec<_>>(),
        )
        .map_err(|reason| NodeError::Replication { reason })?;
        let checkpoint = &envelope.checkpoint;
        if checkpoint.height != descriptor.height
            || checkpoint.block_hash != payload.block_hash
            || checkpoint.state_root != descriptor.execution_state_root
            || checkpoint.execution_block_hash != descriptor.execution_block_hash
            || checkpoint.execution_state_root != descriptor.execution_state_root
            || checkpoint.descriptor_digest != descriptor_digest
            || checkpoint.manifest_size != descriptor.manifest_size_bytes
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
        if source_cache_only {
            let source_path = self
                .config
                .root_dir
                .join("checkpoint-lineage")
                .join(format!("source-{}.json", payload.height));
            write_json_compact(source_path.as_path(), &amended)?;
        } else {
            self.persist_commit_message(payload.height, &amended)?;
        }
        self.reconcile_checkpoint_lineage_retention()?;
        Ok(Some(amended))
    }
}
