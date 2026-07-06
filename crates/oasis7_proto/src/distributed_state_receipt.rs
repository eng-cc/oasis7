//! Bounded state/resource/receipt proof contract anchored to `WorldHeadProofV1`.

use serde::{Deserialize, Serialize};

use crate::distributed::WorldHeadProofV1;

pub const WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA: u16 = 1;
pub const WORLD_STATE_RECEIPT_PROOF_HASH_DOMAIN_V1: &str = "oasis7.world_state_receipt_proof.v1";
pub const WORLD_STATE_RECEIPT_LEAF_HASH_DOMAIN_V1: &str =
    "oasis7.world_state_receipt_proof.leaf.v1";
pub const WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1: &str =
    "oasis7.world_state_receipt_proof.node.v1";
pub const WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1: &str =
    "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness";

fn world_state_receipt_proof_v1_schema() -> u16 {
    WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA
}

fn world_state_receipt_proof_claim_boundary_v1() -> String {
    WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldStateReceiptProofKindV1 {
    ResourceState,
    QueryResult,
    Receipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldStateReceiptProofStatusV1 {
    Included,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldStateReceiptProofSiblingSideV1 {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldStateReceiptProofNodeV1 {
    pub sibling_hash: String,
    pub sibling_side: WorldStateReceiptProofSiblingSideV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "subject_kind", rename_all = "snake_case")]
pub enum WorldStateReceiptProofSubjectV1 {
    ResourceState {
        namespace: String,
        resource_id: String,
        value_hash: String,
        value_codec: String,
        #[serde(default)]
        absence_marker_hash: String,
    },
    QueryResult {
        namespace: String,
        query_id: String,
        query_hash: String,
        result_hash: String,
        result_codec: String,
        #[serde(default)]
        absence_marker_hash: String,
    },
    Receipt {
        action_id: String,
        receipt_hash: String,
        status: String,
        result_hash: String,
        #[serde(default)]
        event_root: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldStateReceiptProofV1 {
    #[serde(default = "world_state_receipt_proof_v1_schema")]
    pub schema_version: u16,
    pub world_id: String,
    pub height: u64,
    pub head_proof: WorldHeadProofV1,
    pub head_proof_hash: String,
    pub proof_kind: WorldStateReceiptProofKindV1,
    pub proof_status: WorldStateReceiptProofStatusV1,
    pub root_hash: String,
    pub subject: WorldStateReceiptProofSubjectV1,
    pub leaf_hash: String,
    pub proof_path: Vec<WorldStateReceiptProofNodeV1>,
    #[serde(default = "world_state_receipt_proof_claim_boundary_v1")]
    pub claim_boundary: String,
}

impl WorldStateReceiptProofSubjectV1 {
    pub fn subject_kind(&self) -> WorldStateReceiptProofKindV1 {
        match self {
            WorldStateReceiptProofSubjectV1::ResourceState { .. } => {
                WorldStateReceiptProofKindV1::ResourceState
            }
            WorldStateReceiptProofSubjectV1::QueryResult { .. } => {
                WorldStateReceiptProofKindV1::QueryResult
            }
            WorldStateReceiptProofSubjectV1::Receipt { .. } => {
                WorldStateReceiptProofKindV1::Receipt
            }
        }
    }

    fn validate_for_status(&self, status: WorldStateReceiptProofStatusV1) -> Result<(), String> {
        match self {
            WorldStateReceiptProofSubjectV1::ResourceState {
                namespace,
                resource_id,
                value_hash,
                value_codec,
                absence_marker_hash,
            } => {
                require_non_empty("resource namespace", namespace)?;
                require_non_empty("resource_id", resource_id)?;
                match status {
                    WorldStateReceiptProofStatusV1::Included => {
                        require_non_empty("resource value_hash", value_hash)?;
                        require_non_empty("resource value_codec", value_codec)?;
                    }
                    WorldStateReceiptProofStatusV1::Absent => {
                        require_non_empty("resource absence_marker_hash", absence_marker_hash)?;
                    }
                }
            }
            WorldStateReceiptProofSubjectV1::QueryResult {
                namespace,
                query_id,
                query_hash,
                result_hash,
                result_codec,
                absence_marker_hash,
            } => {
                require_non_empty("query namespace", namespace)?;
                require_non_empty("query_id", query_id)?;
                require_non_empty("query_hash", query_hash)?;
                match status {
                    WorldStateReceiptProofStatusV1::Included => {
                        require_non_empty("query result_hash", result_hash)?;
                        require_non_empty("query result_codec", result_codec)?;
                    }
                    WorldStateReceiptProofStatusV1::Absent => {
                        require_non_empty("query absence_marker_hash", absence_marker_hash)?;
                    }
                }
            }
            WorldStateReceiptProofSubjectV1::Receipt {
                action_id,
                receipt_hash,
                status: receipt_status,
                result_hash,
                ..
            } => {
                if status != WorldStateReceiptProofStatusV1::Included {
                    return Err("receipt proof does not support absence status".to_string());
                }
                require_non_empty("receipt action_id", action_id)?;
                require_non_empty("receipt_hash", receipt_hash)?;
                require_non_empty("receipt status", receipt_status)?;
                require_non_empty("receipt result_hash", result_hash)?;
            }
        }
        Ok(())
    }

    pub fn leaf_hash(&self, status: WorldStateReceiptProofStatusV1) -> Result<String, String> {
        self.validate_for_status(status)?;
        canonical_blake3_hex(&(WORLD_STATE_RECEIPT_LEAF_HASH_DOMAIN_V1, status, self))
            .map_err(|err| format!("encode world state receipt leaf: {err}"))
    }
}

impl WorldStateReceiptProofV1 {
    pub fn validate_contract(&self) -> Result<(), String> {
        if self.schema_version != WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA {
            return Err(format!(
                "unsupported world state receipt proof schema: {}",
                self.schema_version
            ));
        }
        if self.claim_boundary != WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1 {
            return Err(format!(
                "unexpected world state receipt proof claim boundary: {}",
                self.claim_boundary
            ));
        }
        self.head_proof.validate_contract()?;
        let computed_head_hash = self.head_proof.proof_hash()?;
        if self.head_proof_hash != computed_head_hash {
            return Err(format!(
                "head proof hash mismatch: expected={} actual={}",
                self.head_proof_hash, computed_head_hash
            ));
        }
        if self.world_id != self.head_proof.world_id {
            return Err(format!(
                "world_id mismatch: proof={} head_proof={}",
                self.world_id, self.head_proof.world_id
            ));
        }
        if self.height != self.head_proof.height {
            return Err(format!(
                "height mismatch: proof={} head_proof={}",
                self.height, self.head_proof.height
            ));
        }
        if self.proof_kind != self.subject.subject_kind() {
            return Err(format!(
                "proof kind mismatch: proof={:?} subject={:?}",
                self.proof_kind,
                self.subject.subject_kind()
            ));
        }
        let expected_root = match self.proof_kind {
            WorldStateReceiptProofKindV1::ResourceState
            | WorldStateReceiptProofKindV1::QueryResult => &self.head_proof.block.state_root,
            WorldStateReceiptProofKindV1::Receipt => &self.head_proof.block.receipts_root,
        };
        require_non_empty("root_hash", &self.root_hash)?;
        if self.root_hash != *expected_root {
            return Err(format!(
                "root hash mismatch for {:?}: proof={} head_proof={}",
                self.proof_kind, self.root_hash, expected_root
            ));
        }
        require_non_empty("leaf_hash", &self.leaf_hash)?;
        if self.proof_path.is_empty() {
            return Err("proof_path must not be empty".to_string());
        }
        for (index, node) in self.proof_path.iter().enumerate() {
            require_non_empty(
                format!("proof_path[{index}].sibling_hash").as_str(),
                &node.sibling_hash,
            )?;
        }
        let computed_leaf_hash = self.subject.leaf_hash(self.proof_status)?;
        if self.leaf_hash != computed_leaf_hash {
            return Err(format!(
                "leaf hash mismatch: proof={} computed={}",
                self.leaf_hash, computed_leaf_hash
            ));
        }
        let computed_root =
            compute_world_state_receipt_root(self.leaf_hash.as_str(), self.proof_path.as_slice())?;
        if computed_root != self.root_hash {
            return Err(format!(
                "computed root mismatch: proof={} computed={computed_root}",
                self.root_hash
            ));
        }
        Ok(())
    }

    pub fn proof_hash(&self) -> Result<String, String> {
        self.validate_contract()?;
        canonical_blake3_hex(&(WORLD_STATE_RECEIPT_PROOF_HASH_DOMAIN_V1, self))
            .map_err(|err| format!("encode world state receipt proof: {err}"))
    }
}

pub fn compute_world_state_receipt_root(
    leaf_hash: &str,
    proof_path: &[WorldStateReceiptProofNodeV1],
) -> Result<String, String> {
    require_non_empty("leaf_hash", leaf_hash)?;
    if proof_path.is_empty() {
        return Err("proof_path must not be empty".to_string());
    }
    let mut current = leaf_hash.to_string();
    for node in proof_path {
        require_non_empty("proof_path sibling_hash", &node.sibling_hash)?;
        current = match node.sibling_side {
            WorldStateReceiptProofSiblingSideV1::Left => canonical_blake3_hex(&(
                WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1,
                node.sibling_hash.as_str(),
                current.as_str(),
            )),
            WorldStateReceiptProofSiblingSideV1::Right => canonical_blake3_hex(&(
                WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1,
                current.as_str(),
                node.sibling_hash.as_str(),
            )),
        }
        .map_err(|err| format!("encode world state receipt proof node: {err}"))?;
    }
    Ok(current)
}

fn require_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

fn canonical_blake3_hex<T: Serialize>(value: &T) -> Result<String, serde_cbor::Error> {
    let payload = serde_cbor::to_vec(value)?;
    Ok(blake3::hash(&payload).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::{
        BlobRef, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1, HeadConsensusEvidenceV1,
        WIRE_ENCODING_CBOR, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_V1_SCHEMA,
        WorldBlock, WorldHeadAnnounce,
    };

    fn sample_world_head_proof() -> WorldHeadProofV1 {
        let block = WorldBlock {
            world_id: "w1".to_string(),
            height: 7,
            prev_block_hash: "prev-block".to_string(),
            action_root: "action-root-7".to_string(),
            event_root: "event-root-7".to_string(),
            state_root: "state-root-7".to_string(),
            journal_ref: "journal-ref-7".to_string(),
            snapshot_ref: "snapshot-ref-7".to_string(),
            receipts_root: "receipts-root-7".to_string(),
            proposer_id: "validator-a".to_string(),
            timestamp_ms: 123_456,
            signature: "block-sig".to_string(),
        };
        let block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&block).expect("block hash");
        WorldHeadProofV1 {
            schema_version: WORLD_HEAD_PROOF_V1_SCHEMA,
            world_id: "w1".to_string(),
            height: 7,
            timestamp_ms: 123_456,
            head: WorldHeadAnnounce {
                world_id: "w1".to_string(),
                height: 7,
                block_hash,
                state_root: "state-root-7".to_string(),
                timestamp_ms: 123_456,
                signature: "head-sig".to_string(),
            },
            block,
            snapshot_manifest_ref: BlobRef {
                content_hash: "snapshot-ref-7".to_string(),
                size_bytes: 120,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["snapshot-chunk-1".to_string()],
            },
            journal_segments_ref: BlobRef {
                content_hash: "journal-ref-7".to_string(),
                size_bytes: 80,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["journal-segment-1".to_string()],
            },
            consensus: HeadConsensusEvidenceV1 {
                consensus_status: "committed".to_string(),
                proposer_id: "validator-a".to_string(),
                quorum_threshold: 2,
                validator_count: 3,
                vote_count: 2,
                approver_ids: vec!["validator-a".to_string(), "validator-b".to_string()],
                evidence_hash: "consensus-evidence-7".to_string(),
            },
            execution: ExecutionBindingEvidenceV1 {
                execution_height: 7,
                node_block_hash: String::new(),
                execution_block_hash: "execution-block-7".to_string(),
                execution_state_root: "state-root-7".to_string(),
                action_root: "action-root-7".to_string(),
            },
            checkpoint: Some(CheckpointClosureEvidenceV1 {
                checkpoint_height: 7,
                execution_block_hash: "execution-block-7".to_string(),
                execution_state_root: "state-root-7".to_string(),
                manifest_ref: "checkpoint-manifest-7".to_string(),
                manifest_hash: "checkpoint-manifest-hash-7".to_string(),
                pinned_refs: vec![
                    "snapshot-ref-7".to_string(),
                    "journal-ref-7".to_string(),
                    "state-root-7".to_string(),
                ],
            }),
            claim_boundary: WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    fn sample_valid_world_head_proof() -> WorldHeadProofV1 {
        let mut proof = sample_world_head_proof();
        proof.execution.node_block_hash = proof.head.block_hash.clone();
        proof
    }

    fn sample_proof_path_for_root(
        root_hash: &str,
        leaf_hash: &str,
    ) -> Vec<WorldStateReceiptProofNodeV1> {
        let sibling_hash = canonical_blake3_hex(&(
            WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1,
            leaf_hash,
            root_hash,
        ))
        .expect("sibling hash");
        vec![WorldStateReceiptProofNodeV1 {
            sibling_hash,
            sibling_side: WorldStateReceiptProofSiblingSideV1::Right,
        }]
    }

    fn sample_resource_state_proof() -> WorldStateReceiptProofV1 {
        let mut head_proof = sample_valid_world_head_proof();
        let subject = WorldStateReceiptProofSubjectV1::ResourceState {
            namespace: "inventory".to_string(),
            resource_id: "agent-1/bag".to_string(),
            value_hash: "resource-value-hash".to_string(),
            value_codec: WIRE_ENCODING_CBOR.to_string(),
            absence_marker_hash: String::new(),
        };
        let leaf_hash = subject
            .leaf_hash(WorldStateReceiptProofStatusV1::Included)
            .expect("leaf hash");
        let proof_path =
            sample_proof_path_for_root(head_proof.block.state_root.as_str(), leaf_hash.as_str());
        head_proof.block.state_root =
            compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("state root");
        head_proof.head.state_root = head_proof.block.state_root.clone();
        head_proof.execution.execution_state_root = head_proof.block.state_root.clone();
        head_proof
            .checkpoint
            .as_mut()
            .expect("checkpoint")
            .execution_state_root = head_proof.block.state_root.clone();
        head_proof.head.block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&head_proof.block)
                .expect("block hash");
        head_proof.execution.node_block_hash = head_proof.head.block_hash.clone();
        let head_proof_hash = head_proof.proof_hash().expect("head proof hash");
        WorldStateReceiptProofV1 {
            schema_version: WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA,
            world_id: head_proof.world_id.clone(),
            height: head_proof.height,
            head_proof,
            head_proof_hash,
            proof_kind: WorldStateReceiptProofKindV1::ResourceState,
            proof_status: WorldStateReceiptProofStatusV1::Included,
            root_hash: compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("root hash"),
            subject,
            leaf_hash,
            proof_path,
            claim_boundary: WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    fn sample_query_absence_proof() -> WorldStateReceiptProofV1 {
        let mut head_proof = sample_valid_world_head_proof();
        let subject = WorldStateReceiptProofSubjectV1::QueryResult {
            namespace: "inventory".to_string(),
            query_id: "agent-1/missing-slot".to_string(),
            query_hash: "query-hash-7".to_string(),
            result_hash: String::new(),
            result_codec: String::new(),
            absence_marker_hash: "absence-marker-7".to_string(),
        };
        let leaf_hash = subject
            .leaf_hash(WorldStateReceiptProofStatusV1::Absent)
            .expect("leaf hash");
        let proof_path =
            sample_proof_path_for_root(head_proof.block.state_root.as_str(), leaf_hash.as_str());
        head_proof.block.state_root =
            compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("state root");
        head_proof.head.state_root = head_proof.block.state_root.clone();
        head_proof.execution.execution_state_root = head_proof.block.state_root.clone();
        head_proof
            .checkpoint
            .as_mut()
            .expect("checkpoint")
            .execution_state_root = head_proof.block.state_root.clone();
        head_proof.head.block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&head_proof.block)
                .expect("block hash");
        head_proof.execution.node_block_hash = head_proof.head.block_hash.clone();
        let head_proof_hash = head_proof.proof_hash().expect("head proof hash");
        WorldStateReceiptProofV1 {
            schema_version: WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA,
            world_id: head_proof.world_id.clone(),
            height: head_proof.height,
            head_proof,
            head_proof_hash,
            proof_kind: WorldStateReceiptProofKindV1::QueryResult,
            proof_status: WorldStateReceiptProofStatusV1::Absent,
            root_hash: compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("root hash"),
            subject,
            leaf_hash,
            proof_path,
            claim_boundary: WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    fn sample_resource_absence_proof() -> WorldStateReceiptProofV1 {
        let mut head_proof = sample_valid_world_head_proof();
        let subject = WorldStateReceiptProofSubjectV1::ResourceState {
            namespace: "inventory".to_string(),
            resource_id: "agent-1/missing-slot".to_string(),
            value_hash: String::new(),
            value_codec: String::new(),
            absence_marker_hash: "resource-absence-marker-7".to_string(),
        };
        let leaf_hash = subject
            .leaf_hash(WorldStateReceiptProofStatusV1::Absent)
            .expect("leaf hash");
        let proof_path =
            sample_proof_path_for_root(head_proof.block.state_root.as_str(), leaf_hash.as_str());
        head_proof.block.state_root =
            compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("state root");
        head_proof.head.state_root = head_proof.block.state_root.clone();
        head_proof.execution.execution_state_root = head_proof.block.state_root.clone();
        head_proof
            .checkpoint
            .as_mut()
            .expect("checkpoint")
            .execution_state_root = head_proof.block.state_root.clone();
        head_proof.head.block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&head_proof.block)
                .expect("block hash");
        head_proof.execution.node_block_hash = head_proof.head.block_hash.clone();
        let head_proof_hash = head_proof.proof_hash().expect("head proof hash");
        WorldStateReceiptProofV1 {
            schema_version: WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA,
            world_id: head_proof.world_id.clone(),
            height: head_proof.height,
            head_proof,
            head_proof_hash,
            proof_kind: WorldStateReceiptProofKindV1::ResourceState,
            proof_status: WorldStateReceiptProofStatusV1::Absent,
            root_hash: compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("root hash"),
            subject,
            leaf_hash,
            proof_path,
            claim_boundary: WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    fn sample_query_result_proof() -> WorldStateReceiptProofV1 {
        let mut head_proof = sample_valid_world_head_proof();
        let subject = WorldStateReceiptProofSubjectV1::QueryResult {
            namespace: "inventory".to_string(),
            query_id: "agent-1/bag-query".to_string(),
            query_hash: "query-hash-agent-1-bag".to_string(),
            result_hash: "query-result-hash-agent-1-bag".to_string(),
            result_codec: WIRE_ENCODING_CBOR.to_string(),
            absence_marker_hash: String::new(),
        };
        let leaf_hash = subject
            .leaf_hash(WorldStateReceiptProofStatusV1::Included)
            .expect("leaf hash");
        let proof_path =
            sample_proof_path_for_root(head_proof.block.state_root.as_str(), leaf_hash.as_str());
        head_proof.block.state_root =
            compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("state root");
        head_proof.head.state_root = head_proof.block.state_root.clone();
        head_proof.execution.execution_state_root = head_proof.block.state_root.clone();
        head_proof
            .checkpoint
            .as_mut()
            .expect("checkpoint")
            .execution_state_root = head_proof.block.state_root.clone();
        head_proof.head.block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&head_proof.block)
                .expect("block hash");
        head_proof.execution.node_block_hash = head_proof.head.block_hash.clone();
        let head_proof_hash = head_proof.proof_hash().expect("head proof hash");
        WorldStateReceiptProofV1 {
            schema_version: WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA,
            world_id: head_proof.world_id.clone(),
            height: head_proof.height,
            head_proof,
            head_proof_hash,
            proof_kind: WorldStateReceiptProofKindV1::QueryResult,
            proof_status: WorldStateReceiptProofStatusV1::Included,
            root_hash: compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("root hash"),
            subject,
            leaf_hash,
            proof_path,
            claim_boundary: WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    fn sample_receipt_proof() -> WorldStateReceiptProofV1 {
        let mut head_proof = sample_valid_world_head_proof();
        let subject = WorldStateReceiptProofSubjectV1::Receipt {
            action_id: "action-7".to_string(),
            receipt_hash: "receipt-hash-7".to_string(),
            status: "committed".to_string(),
            result_hash: "receipt-result-7".to_string(),
            event_root: "event-root-7".to_string(),
        };
        let leaf_hash = subject
            .leaf_hash(WorldStateReceiptProofStatusV1::Included)
            .expect("leaf hash");
        let proof_path =
            sample_proof_path_for_root(head_proof.block.receipts_root.as_str(), leaf_hash.as_str());
        head_proof.block.receipts_root =
            compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("receipt root");
        head_proof.head.block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&head_proof.block)
                .expect("block hash");
        head_proof.execution.node_block_hash = head_proof.head.block_hash.clone();
        let head_proof_hash = head_proof.proof_hash().expect("head proof hash");
        WorldStateReceiptProofV1 {
            schema_version: WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA,
            world_id: head_proof.world_id.clone(),
            height: head_proof.height,
            head_proof,
            head_proof_hash,
            proof_kind: WorldStateReceiptProofKindV1::Receipt,
            proof_status: WorldStateReceiptProofStatusV1::Included,
            root_hash: compute_world_state_receipt_root(leaf_hash.as_str(), proof_path.as_slice())
                .expect("root hash"),
            subject,
            leaf_hash,
            proof_path,
            claim_boundary: WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    #[test]
    fn world_state_receipt_proof_v1_validates_resource_inclusion() {
        let proof = sample_resource_state_proof();

        proof.validate_contract().expect("valid proof");
        assert_eq!(
            proof.proof_hash().expect("proof hash"),
            proof.proof_hash().expect("stable proof hash")
        );
    }

    #[test]
    fn world_state_receipt_proof_v1_validates_receipt_inclusion() {
        let proof = sample_receipt_proof();

        proof.validate_contract().expect("valid receipt proof");
        assert_eq!(proof.root_hash, proof.head_proof.block.receipts_root);
    }

    #[test]
    fn world_state_receipt_proof_v1_validates_query_absence() {
        let proof = sample_query_absence_proof();

        proof
            .validate_contract()
            .expect("valid query absence proof");
        assert_eq!(proof.proof_kind, WorldStateReceiptProofKindV1::QueryResult);
        assert_eq!(proof.proof_status, WorldStateReceiptProofStatusV1::Absent);
        assert_eq!(proof.root_hash, proof.head_proof.block.state_root);
    }

    #[test]
    fn world_state_receipt_proof_v1_validates_resource_absence() {
        let proof = sample_resource_absence_proof();

        proof
            .validate_contract()
            .expect("valid resource absence proof");
        assert_eq!(
            proof.proof_kind,
            WorldStateReceiptProofKindV1::ResourceState
        );
        assert_eq!(proof.proof_status, WorldStateReceiptProofStatusV1::Absent);
        assert_eq!(proof.root_hash, proof.head_proof.block.state_root);
    }

    #[test]
    fn world_state_receipt_proof_v1_validates_query_result_inclusion() {
        let proof = sample_query_result_proof();

        proof.validate_contract().expect("valid query result proof");
        assert_eq!(proof.proof_kind, WorldStateReceiptProofKindV1::QueryResult);
        assert_eq!(proof.proof_status, WorldStateReceiptProofStatusV1::Included);
        assert_eq!(proof.root_hash, proof.head_proof.block.state_root);
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_head_hash_mismatch() {
        let mut proof = sample_resource_state_proof();
        proof.head_proof_hash = "wrong-head-proof-hash".to_string();

        let err = proof.validate_contract().expect_err("tamper rejected");
        assert!(err.contains("head proof hash mismatch"), "{err}");
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_wrong_root_kind() {
        let mut proof = sample_receipt_proof();
        proof.root_hash = proof.head_proof.block.state_root.clone();

        let err = proof.validate_contract().expect_err("root kind rejected");
        assert!(err.contains("root hash mismatch"), "{err}");
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_empty_proof_path() {
        let mut proof = sample_resource_state_proof();
        proof.proof_path.clear();

        let err = proof.validate_contract().expect_err("empty path rejected");
        assert!(err.contains("proof_path must not be empty"), "{err}");
    }

    #[test]
    fn world_state_receipt_proof_v1_accepts_repeated_sibling_hashes() {
        let mut proof = sample_resource_state_proof();
        proof
            .proof_path
            .push(proof.proof_path.first().expect("sample proof node").clone());
        proof.root_hash =
            compute_world_state_receipt_root(proof.leaf_hash.as_str(), proof.proof_path.as_slice())
                .expect("root hash");
        proof.head_proof.block.state_root = proof.root_hash.clone();
        proof.head_proof.head.state_root = proof.root_hash.clone();
        proof.head_proof.execution.execution_state_root = proof.root_hash.clone();
        proof
            .head_proof
            .checkpoint
            .as_mut()
            .expect("checkpoint")
            .execution_state_root = proof.root_hash.clone();
        proof.head_proof.head.block_hash =
            crate::distributed::test_support::canonical_blake3_hex(&proof.head_proof.block)
                .expect("block hash");
        proof.head_proof.execution.node_block_hash = proof.head_proof.head.block_hash.clone();
        proof.head_proof_hash = proof.head_proof.proof_hash().expect("head proof hash");

        proof
            .validate_contract()
            .expect("repeated sibling hashes are allowed when the ordered path reaches the root");
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_leaf_tamper() {
        let mut proof = sample_resource_state_proof();
        proof.leaf_hash = "wrong-leaf".to_string();

        let err = proof.validate_contract().expect_err("leaf tamper rejected");
        assert!(err.contains("leaf hash mismatch"), "{err}");
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_subject_tamper() {
        let mut proof = sample_receipt_proof();
        proof.subject = WorldStateReceiptProofSubjectV1::Receipt {
            action_id: "wrong-action".to_string(),
            receipt_hash: "receipt-hash-7".to_string(),
            status: "committed".to_string(),
            result_hash: "receipt-result-7".to_string(),
            event_root: "event-root-7".to_string(),
        };

        let err = proof
            .validate_contract()
            .expect_err("subject tamper rejected");
        assert!(err.contains("leaf hash mismatch"), "{err}");
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_claim_boundary_mismatch() {
        let mut proof = sample_resource_state_proof();
        proof.claim_boundary = "too-broad".to_string();

        let err = proof.validate_contract().expect_err("boundary rejected");
        assert!(
            err.contains("unexpected world state receipt proof claim boundary"),
            "{err}"
        );
    }

    #[test]
    fn world_state_receipt_proof_v1_rejects_receipt_absence() {
        let mut proof = sample_receipt_proof();
        proof.proof_status = WorldStateReceiptProofStatusV1::Absent;

        let err = proof
            .validate_contract()
            .expect_err("receipt absence rejected");
        assert!(
            err.contains("receipt proof does not support absence status"),
            "{err}"
        );
    }
}
