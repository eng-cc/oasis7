use oasis7_distfs::{
    BlobStore as _, apply_replication_record, blake3_hex, build_replication_record_with_epoch,
};

use crate::{
    NodeError, NodeExecutionCheckpointBlob, NodeExecutionCheckpointBlobRef,
    NodeExecutionCheckpointBundle, NodeExecutionCheckpointDescriptor,
};

use super::ReplicationRuntime;
use super::support::{distfs_error_to_node_error, sign_replication_message};
use super::{GossipReplicationMessage, ReplicatedCommitPayload};

impl ReplicationRuntime {
    pub(crate) fn store_execution_checkpoint_bundle(
        &self,
        bundle: &NodeExecutionCheckpointBundle,
    ) -> Result<NodeExecutionCheckpointDescriptor, NodeError> {
        let manifest_ref = blake3_hex(bundle.manifest_json.as_slice());
        self.store_blob_by_hash(manifest_ref.as_str(), bundle.manifest_json.as_slice())?;
        let mut blob_refs = Vec::with_capacity(bundle.blobs.len());
        for blob in &bundle.blobs {
            let actual = blake3_hex(blob.bytes.as_slice());
            if actual != blob.content_hash {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint blob hash mismatch expected={} actual={}",
                        blob.content_hash, actual
                    ),
                });
            }
            self.store_blob_by_hash(blob.content_hash.as_str(), blob.bytes.as_slice())?;
            blob_refs.push(NodeExecutionCheckpointBlobRef {
                content_hash: blob.content_hash.clone(),
                size_bytes: blob.bytes.len() as u64,
            });
        }
        let descriptor = NodeExecutionCheckpointDescriptor {
            height: bundle.height,
            execution_block_hash: bundle.execution_block_hash.clone(),
            execution_state_root: bundle.execution_state_root.clone(),
            manifest_ref,
            manifest_size_bytes: bundle.manifest_json.len() as u64,
            blobs: blob_refs,
        };
        self.pin_execution_checkpoint_descriptor(&descriptor)?;
        Ok(descriptor)
    }

    pub(crate) fn load_execution_checkpoint_bundle(
        &self,
        descriptor: &NodeExecutionCheckpointDescriptor,
    ) -> Result<Option<NodeExecutionCheckpointBundle>, NodeError> {
        let Some(manifest_json) = self.load_blob_by_hash(descriptor.manifest_ref.as_str())? else {
            return Ok(None);
        };
        if manifest_json.len() as u64 != descriptor.manifest_size_bytes {
            return Err(NodeError::Replication {
                reason: format!(
                    "execution checkpoint manifest size mismatch expected={} actual={}",
                    descriptor.manifest_size_bytes,
                    manifest_json.len()
                ),
            });
        }
        let mut blobs = Vec::with_capacity(descriptor.blobs.len());
        for blob_ref in &descriptor.blobs {
            let Some(bytes) = self.load_blob_by_hash(blob_ref.content_hash.as_str())? else {
                return Ok(None);
            };
            if bytes.len() as u64 != blob_ref.size_bytes {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint blob size mismatch hash={} expected={} actual={}",
                        blob_ref.content_hash,
                        blob_ref.size_bytes,
                        bytes.len()
                    ),
                });
            }
            blobs.push(NodeExecutionCheckpointBlob {
                content_hash: blob_ref.content_hash.clone(),
                bytes,
            });
        }
        Ok(Some(NodeExecutionCheckpointBundle {
            height: descriptor.height,
            execution_block_hash: descriptor.execution_block_hash.clone(),
            execution_state_root: descriptor.execution_state_root.clone(),
            manifest_json,
            blobs,
        }))
    }

    pub(crate) fn pin_execution_checkpoint_descriptor(
        &self,
        descriptor: &NodeExecutionCheckpointDescriptor,
    ) -> Result<(), NodeError> {
        self.store
            .pin(descriptor.manifest_ref.as_str())
            .map_err(distfs_error_to_node_error)?;
        for blob_ref in &descriptor.blobs {
            self.store
                .pin(blob_ref.content_hash.as_str())
                .map_err(distfs_error_to_node_error)?;
        }
        Ok(())
    }

    pub(crate) fn attach_execution_checkpoint_descriptor_to_message(
        &mut self,
        local_node_id: &str,
        message: &GossipReplicationMessage,
        checkpoint: &NodeExecutionCheckpointBundle,
    ) -> Result<GossipReplicationMessage, NodeError> {
        let mut payload = serde_json::from_slice::<ReplicatedCommitPayload>(
            message.payload.as_slice(),
        )
        .map_err(|err| NodeError::Replication {
            reason: format!(
                "decode replication payload for checkpoint descriptor attach failed: {}",
                err
            ),
        })?;
        if payload.height != checkpoint.height {
            return Err(NodeError::Replication {
                reason: format!(
                    "execution checkpoint attach height mismatch payload={} checkpoint={}",
                    payload.height, checkpoint.height
                ),
            });
        }
        match (
            payload.execution_block_hash.as_deref(),
            payload.execution_state_root.as_deref(),
        ) {
            (Some(block_hash), Some(state_root))
                if block_hash == checkpoint.execution_block_hash
                    && state_root == checkpoint.execution_state_root => {}
            _ => {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint attach binding mismatch at height {}",
                        payload.height
                    ),
                });
            }
        }
        if self.signer.is_none() && message.signature_hex.is_some() {
            return Err(NodeError::Replication {
                reason: "cannot re-sign augmented replication checkpoint message without signer"
                    .to_string(),
            });
        }

        // A storage/full-storage provider can receive an upstream commit that
        // already carries a checkpoint descriptor while its own replication
        // store has none of the referenced closure.  Persist the locally
        // exported, binding-checked bundle and re-sign the refreshed message
        // instead of returning the upstream descriptor unchanged.
        payload.execution_checkpoint = Some(self.store_execution_checkpoint_bundle(checkpoint)?);
        let payload_bytes = serde_json::to_vec(&payload).map_err(|err| NodeError::Replication {
            reason: format!("serialize augmented replication payload failed: {}", err),
        })?;
        let mut augmented = message.clone();
        augmented.payload = payload_bytes;
        if let Some(signer) = &self.signer {
            if message.record.writer_id != signer.public_key_hex {
                let writer_id = signer.public_key_hex.clone();
                let (writer_epoch, sequence) = self.next_local_record_position(&writer_id)?;
                let path = format!("consensus/commits/{:020}.json", payload.height);
                augmented.record = build_replication_record_with_epoch(
                    message.world_id.as_str(),
                    writer_id.as_str(),
                    writer_epoch,
                    sequence,
                    path.as_str(),
                    augmented.payload.as_slice(),
                    message.record.updated_at_ms,
                )
                .map_err(distfs_error_to_node_error)?;
                apply_replication_record(
                    &self.store,
                    &mut self.guard,
                    &augmented.record,
                    augmented.payload.as_slice(),
                )
                .map_err(distfs_error_to_node_error)?;
                self.writer_state.writer_epoch = augmented.record.writer_epoch;
                self.writer_state.last_sequence = augmented.record.sequence;
                self.writer_state.last_replicated_height =
                    self.writer_state.last_replicated_height.max(payload.height);
                self.persist_state(local_node_id)?;
            } else {
                augmented.record.content_hash = blake3_hex(augmented.payload.as_slice());
                augmented.record.size_bytes = augmented.payload.len() as u64;
            }
        } else {
            augmented.record.content_hash = blake3_hex(augmented.payload.as_slice());
            augmented.record.size_bytes = augmented.payload.len() as u64;
        }
        self.store
            .put(
                augmented.record.content_hash.as_str(),
                augmented.payload.as_slice(),
            )
            .map_err(distfs_error_to_node_error)?;
        self.store
            .pin(augmented.record.content_hash.as_str())
            .map_err(distfs_error_to_node_error)?;
        augmented.signature_hex = None;
        if let Some(signer) = &self.signer {
            augmented.public_key_hex = Some(signer.public_key_hex.clone());
            augmented.signature_hex = Some(sign_replication_message(&augmented, signer)?);
        }
        Ok(augmented)
    }
}
