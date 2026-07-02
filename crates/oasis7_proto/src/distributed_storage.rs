use serde::{Deserialize, Serialize};

use crate::distributed::{
    BlobRef, BlockAnnounce, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1,
    HeadConsensusEvidenceV1, SnapshotManifest, WIRE_ENCODING_CBOR, WorldBlock, WorldHeadAnnounce,
    WorldHeadProofV1,
};

pub const DEFAULT_SNAPSHOT_CHUNK_BYTES: usize = 256 * 1024;
pub const DEFAULT_JOURNAL_EVENTS_PER_SEGMENT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentConfig {
    pub snapshot_chunk_bytes: usize,
    pub journal_events_per_segment: usize,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            snapshot_chunk_bytes: DEFAULT_SNAPSHOT_CHUNK_BYTES,
            journal_events_per_segment: DEFAULT_JOURNAL_EVENTS_PER_SEGMENT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalSegmentRef {
    pub from_event_id: u64,
    pub to_event_id: u64,
    pub content_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWriteConfig {
    pub segment: SegmentConfig,
    pub codec: String,
}

impl Default for ExecutionWriteConfig {
    fn default() -> Self {
        Self {
            segment: SegmentConfig::default(),
            codec: WIRE_ENCODING_CBOR.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWriteResult {
    pub block: WorldBlock,
    pub block_hash: String,
    pub block_ref: BlobRef,
    pub block_announce: BlockAnnounce,
    pub head_announce: WorldHeadAnnounce,
    pub snapshot_manifest: SnapshotManifest,
    pub snapshot_manifest_ref: BlobRef,
    pub journal_segments: Vec<JournalSegmentRef>,
    pub journal_segments_ref: BlobRef,
}

impl ExecutionWriteResult {
    pub fn world_head_proof_v1(
        &self,
        consensus: HeadConsensusEvidenceV1,
        execution: ExecutionBindingEvidenceV1,
        checkpoint: Option<CheckpointClosureEvidenceV1>,
    ) -> Result<WorldHeadProofV1, String> {
        let proof = WorldHeadProofV1 {
            schema_version: crate::distributed::WORLD_HEAD_PROOF_V1_SCHEMA,
            world_id: self.head_announce.world_id.clone(),
            height: self.head_announce.height,
            timestamp_ms: self.head_announce.timestamp_ms,
            head: self.head_announce.clone(),
            block: self.block.clone(),
            snapshot_manifest_ref: self.snapshot_manifest_ref.clone(),
            journal_segments_ref: self.journal_segments_ref.clone(),
            consensus,
            execution,
            checkpoint,
            claim_boundary: crate::distributed::WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        };
        proof.validate_contract()?;
        Ok(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::{
        StateChunkRef, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_HASH_DOMAIN_V1,
    };

    fn hash_cbor<T: Serialize>(value: &T) -> String {
        let encoded = serde_cbor::to_vec(value).expect("encode");
        blake3::hash(&encoded).to_hex().to_string()
    }

    fn sample_execution_write_result() -> ExecutionWriteResult {
        let block = WorldBlock {
            world_id: "w1".to_string(),
            height: 11,
            prev_block_hash: "prev-10".to_string(),
            action_root: "action-root-11".to_string(),
            event_root: "event-root-11".to_string(),
            state_root: "state-root-11".to_string(),
            journal_ref: "journal-ref-11".to_string(),
            snapshot_ref: "snapshot-ref-11".to_string(),
            receipts_root: "receipts-root-11".to_string(),
            proposer_id: "validator-a".to_string(),
            timestamp_ms: 11_000,
            signature: "block-signature".to_string(),
        };
        let block_hash = hash_cbor(&block);
        ExecutionWriteResult {
            block: block.clone(),
            block_hash: block_hash.clone(),
            block_ref: BlobRef {
                content_hash: block_hash.clone(),
                size_bytes: 256,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["snapshot-ref-11".to_string(), "journal-ref-11".to_string()],
            },
            block_announce: BlockAnnounce {
                world_id: "w1".to_string(),
                height: 11,
                block_hash: block_hash.clone(),
                prev_block_hash: "prev-10".to_string(),
                state_root: "state-root-11".to_string(),
                event_root: "event-root-11".to_string(),
                timestamp_ms: 11_000,
                signature: "block-announce-signature".to_string(),
            },
            head_announce: WorldHeadAnnounce {
                world_id: "w1".to_string(),
                height: 11,
                block_hash,
                state_root: "state-root-11".to_string(),
                timestamp_ms: 11_000,
                signature: "head-signature".to_string(),
            },
            snapshot_manifest: SnapshotManifest {
                world_id: "w1".to_string(),
                epoch: 11,
                chunks: vec![StateChunkRef {
                    chunk_id: "state-0".to_string(),
                    content_hash: "state-chunk-0".to_string(),
                    size_bytes: 64,
                }],
                state_root: "state-root-11".to_string(),
            },
            snapshot_manifest_ref: BlobRef {
                content_hash: "snapshot-ref-11".to_string(),
                size_bytes: 128,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["state-chunk-0".to_string()],
            },
            journal_segments: vec![JournalSegmentRef {
                from_event_id: 1,
                to_event_id: 2,
                content_hash: "journal-segment-0".to_string(),
                size_bytes: 72,
            }],
            journal_segments_ref: BlobRef {
                content_hash: "journal-ref-11".to_string(),
                size_bytes: 96,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["journal-segment-0".to_string()],
            },
        }
    }

    fn consensus() -> HeadConsensusEvidenceV1 {
        HeadConsensusEvidenceV1 {
            consensus_status: "committed".to_string(),
            proposer_id: "validator-a".to_string(),
            quorum_threshold: 2,
            validator_count: 3,
            vote_count: 2,
            approver_ids: vec!["validator-a".to_string(), "validator-b".to_string()],
            evidence_hash: "consensus-evidence-11".to_string(),
        }
    }

    fn execution(write: &ExecutionWriteResult) -> ExecutionBindingEvidenceV1 {
        ExecutionBindingEvidenceV1 {
            execution_height: write.head_announce.height,
            node_block_hash: write.head_announce.block_hash.clone(),
            execution_block_hash: "execution-block-11".to_string(),
            execution_state_root: write.block.state_root.clone(),
            action_root: write.block.action_root.clone(),
        }
    }

    fn checkpoint(write: &ExecutionWriteResult) -> CheckpointClosureEvidenceV1 {
        CheckpointClosureEvidenceV1 {
            checkpoint_height: write.head_announce.height,
            execution_block_hash: "execution-block-11".to_string(),
            execution_state_root: write.block.state_root.clone(),
            manifest_ref: "checkpoint-manifest-11".to_string(),
            manifest_hash: "checkpoint-manifest-hash-11".to_string(),
            pinned_refs: vec![
                write.snapshot_manifest_ref.content_hash.clone(),
                write.journal_segments_ref.content_hash.clone(),
                "state-root-11".to_string(),
            ],
        }
    }

    #[test]
    fn execution_write_result_builds_valid_world_head_proof_v1() {
        let write = sample_execution_write_result();

        let proof = write
            .world_head_proof_v1(consensus(), execution(&write), Some(checkpoint(&write)))
            .expect("proof");

        assert_eq!(proof.world_id, write.head_announce.world_id);
        assert_eq!(proof.height, write.head_announce.height);
        assert_eq!(proof.head.block_hash, write.head_announce.block_hash);
        assert_eq!(
            proof.block.snapshot_ref,
            write.snapshot_manifest_ref.content_hash
        );
        assert_eq!(
            proof.block.journal_ref,
            write.journal_segments_ref.content_hash
        );
        assert_eq!(proof.claim_boundary, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1);
        assert_ne!(
            proof.proof_hash().expect("proof hash"),
            hash_cbor(&(WORLD_HEAD_PROOF_HASH_DOMAIN_V1, "not-the-proof"))
        );
    }

    #[test]
    fn execution_write_result_rejects_mismatched_execution_root() {
        let write = sample_execution_write_result();
        let mut execution = execution(&write);
        execution.execution_state_root = "wrong-state-root".to_string();

        let err = write
            .world_head_proof_v1(consensus(), execution, Some(checkpoint(&write)))
            .expect_err("mismatch rejected");

        assert!(err.contains("execution state_root mismatch"), "{err}");
    }

    #[test]
    fn execution_write_result_rejects_checkpoint_without_snapshot_pin() {
        let write = sample_execution_write_result();
        let mut checkpoint = checkpoint(&write);
        checkpoint
            .pinned_refs
            .retain(|reference| reference != &write.snapshot_manifest_ref.content_hash);

        let err = write
            .world_head_proof_v1(consensus(), execution(&write), Some(checkpoint))
            .expect_err("missing pin rejected");

        assert!(
            err.contains("checkpoint pinned refs missing snapshot manifest ref"),
            "{err}"
        );
    }
}
