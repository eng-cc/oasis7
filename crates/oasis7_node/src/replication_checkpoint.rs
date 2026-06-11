use oasis7_distfs::blake3_hex;

use crate::{
    NodeError, NodeExecutionCheckpointBlob, NodeExecutionCheckpointBlobRef,
    NodeExecutionCheckpointBundle, NodeExecutionCheckpointDescriptor,
};

use super::support::distfs_error_to_node_error;
use super::ReplicationRuntime;

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
}
