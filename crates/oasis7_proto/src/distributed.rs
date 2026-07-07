//! Distributed runtime protocol types and naming conventions.

use serde::{Deserialize, Serialize};

pub const WIRE_ENCODING_CBOR: &str = "cbor";

pub const GOSSIPSUB_PREFIX: &str = "aw";
pub const TOPIC_ACTION_SUFFIX: &str = "action";
pub const TOPIC_BLOCK_SUFFIX: &str = "block";
pub const TOPIC_HEAD_SUFFIX: &str = "head";
pub const TOPIC_EVENT_SUFFIX: &str = "event";
pub const TOPIC_MEMBERSHIP_SUFFIX: &str = "membership";
pub const TOPIC_MEMBERSHIP_REVOKE_SUFFIX: &str = "membership.revoke";
pub const TOPIC_MEMBERSHIP_RECONCILE_SUFFIX: &str = "membership.reconcile";

pub const RR_PROTOCOL_PREFIX: &str = "/aw/rr/1.0.0";
pub const RR_GET_WORLD_HEAD: &str = "/aw/rr/1.0.0/get_world_head";
pub const RR_GET_BLOCK: &str = "/aw/rr/1.0.0/get_block";
pub const RR_GET_SNAPSHOT: &str = "/aw/rr/1.0.0/get_snapshot";
pub const RR_GET_JOURNAL_SEGMENT: &str = "/aw/rr/1.0.0/get_journal_segment";
pub const RR_GET_RECEIPT_SEGMENT: &str = "/aw/rr/1.0.0/get_receipt_segment";
pub const RR_FETCH_BLOB: &str = "/aw/rr/1.0.0/fetch_blob";
pub const RR_GET_MODULE_MANIFEST: &str = "/aw/rr/1.0.0/get_module_manifest";
pub const RR_GET_MODULE_ARTIFACT: &str = "/aw/rr/1.0.0/get_module_artifact";

pub const DHT_WORLD_PREFIX: &str = "/aw/world";
pub const DHT_MEMBERSHIP_SUFFIX: &str = "membership";
pub const DHT_PEER_DISCOVERY_SUFFIX: &str = "peer-discovery";
pub const DHT_PEER_RECORDS_SUFFIX: &str = "peer-records";

pub fn gossipsub_topic(world_id: &str, suffix: &str) -> String {
    format!("{GOSSIPSUB_PREFIX}.{world_id}.{suffix}")
}

pub fn topic_action(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_ACTION_SUFFIX)
}

pub fn topic_block(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_BLOCK_SUFFIX)
}

pub fn topic_head(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_HEAD_SUFFIX)
}

pub fn topic_event(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_EVENT_SUFFIX)
}

pub fn topic_membership(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_MEMBERSHIP_SUFFIX)
}

pub fn topic_membership_revocation(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_MEMBERSHIP_REVOKE_SUFFIX)
}

pub fn topic_membership_reconcile(world_id: &str) -> String {
    gossipsub_topic(world_id, TOPIC_MEMBERSHIP_RECONCILE_SUFFIX)
}

pub fn dht_world_head_key(world_id: &str) -> String {
    format!("{DHT_WORLD_PREFIX}/{world_id}/head")
}

pub fn dht_provider_key(world_id: &str, content_hash: &str) -> String {
    format!("{DHT_WORLD_PREFIX}/{world_id}/providers/{content_hash}")
}

pub fn dht_membership_key(world_id: &str) -> String {
    format!("{DHT_WORLD_PREFIX}/{world_id}/{DHT_MEMBERSHIP_SUFFIX}")
}

pub fn dht_peer_discovery_key(world_id: &str) -> String {
    format!("{DHT_WORLD_PREFIX}/{world_id}/{DHT_PEER_DISCOVERY_SUFFIX}")
}

pub fn rendezvous_namespace(world_id: &str, network_id: &str) -> String {
    format!("aw-{world_id}-{network_id}")
}

pub fn dht_peer_record_key(world_id: &str, peer_id: &str) -> String {
    format!("{DHT_WORLD_PREFIX}/{world_id}/{DHT_PEER_RECORDS_SUFFIX}/{peer_id}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBlock {
    pub world_id: String,
    pub height: u64,
    pub prev_block_hash: String,
    pub action_root: String,
    pub event_root: String,
    pub state_root: String,
    pub journal_ref: String,
    pub snapshot_ref: String,
    pub receipts_root: String,
    pub proposer_id: String,
    pub timestamp_ms: i64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub world_id: String,
    pub action_id: String,
    pub actor_id: String,
    pub action_kind: String,
    pub payload_cbor: Vec<u8>,
    pub payload_hash: String,
    pub nonce: u64,
    pub timestamp_ms: i64,
    #[serde(default)]
    pub intent_batch_hash: String,
    #[serde(default)]
    pub idempotency_key: String,
    #[serde(default)]
    pub zone_id: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBatch {
    pub world_id: String,
    pub batch_id: String,
    pub actions: Vec<ActionEnvelope>,
    pub proposer_id: String,
    pub timestamp_ms: i64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldHeadAnnounce {
    pub world_id: String,
    pub height: u64,
    pub block_hash: String,
    pub state_root: String,
    pub timestamp_ms: i64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAnnounce {
    pub world_id: String,
    pub height: u64,
    pub block_hash: String,
    pub prev_block_hash: String,
    pub state_root: String,
    pub event_root: String,
    pub timestamp_ms: i64,
    pub signature: String,
}

pub const WORLD_HEAD_PROOF_V1_SCHEMA: u16 = 1;
pub const WORLD_HEAD_PROOF_HASH_DOMAIN_V1: &str = "oasis7.world_head_proof.v1";
pub const WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1: &str =
    "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness";

pub use crate::distributed_finality::{
    WORLD_FINALITY_GOVERNANCE_SET_HASH_DOMAIN_V1, WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1,
    WORLD_FINALITY_PROOF_HASH_DOMAIN_V1, WORLD_FINALITY_PROOF_V1_SCHEMA,
    WORLD_FINALITY_VALIDATOR_SET_HASH_DOMAIN_V1,
    WORLD_FINALITY_VALIDATOR_SET_TRANSITION_GOVERNANCE_SIGNING_DOMAIN_V1,
    WORLD_FINALITY_VALIDATOR_SET_TRANSITION_SIGNING_DOMAIN_V1,
    WORLD_FINALITY_VOTE_SIGNING_DOMAIN_V1, WorldFinalityCommitmentV1,
    WorldFinalityGovernanceSignerV1, WorldFinalityMisbehaviorEvidenceV1, WorldFinalityProofV1,
    WorldFinalityValidatorSetTransitionApprovalV1,
    WorldFinalityValidatorSetTransitionGovernanceApprovalV1,
    WorldFinalityValidatorSetTransitionGovernanceCertificateV1,
    WorldFinalityValidatorSetTransitionV1, WorldFinalityValidatorV1, WorldFinalityVoteV1,
    compute_world_finality_governance_set_hash, compute_world_finality_validator_set_hash,
    world_finality_validator_set_transition_governance_signing_payload,
    world_finality_validator_set_transition_signing_payload, world_finality_vote_signing_payload,
};
pub use crate::distributed_state_receipt::{
    WORLD_STATE_RECEIPT_LEAF_HASH_DOMAIN_V1, WORLD_STATE_RECEIPT_NODE_HASH_DOMAIN_V1,
    WORLD_STATE_RECEIPT_PROOF_CLAIM_BOUNDARY_V1, WORLD_STATE_RECEIPT_PROOF_HASH_DOMAIN_V1,
    WORLD_STATE_RECEIPT_PROOF_V1_SCHEMA, WorldStateReceiptProofKindV1,
    WorldStateReceiptProofNodeV1, WorldStateReceiptProofSiblingSideV1,
    WorldStateReceiptProofStatusV1, WorldStateReceiptProofSubjectV1, WorldStateReceiptProofV1,
    compute_world_state_receipt_root,
};

fn world_head_proof_v1_schema() -> u16 {
    WORLD_HEAD_PROOF_V1_SCHEMA
}

fn world_head_proof_claim_boundary_v1() -> String {
    WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadConsensusEvidenceV1 {
    pub consensus_status: String,
    pub proposer_id: String,
    #[serde(default)]
    pub quorum_threshold: u64,
    #[serde(default)]
    pub validator_count: u64,
    #[serde(default)]
    pub vote_count: u64,
    #[serde(default)]
    pub approver_ids: Vec<String>,
    #[serde(default)]
    pub evidence_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionBindingEvidenceV1 {
    pub execution_height: u64,
    pub node_block_hash: String,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub action_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointClosureEvidenceV1 {
    pub checkpoint_height: u64,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub manifest_ref: String,
    #[serde(default)]
    pub manifest_hash: String,
    #[serde(default)]
    pub pinned_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldHeadProofV1 {
    #[serde(default = "world_head_proof_v1_schema")]
    pub schema_version: u16,
    pub world_id: String,
    pub height: u64,
    pub timestamp_ms: i64,
    pub head: WorldHeadAnnounce,
    pub block: WorldBlock,
    pub snapshot_manifest_ref: BlobRef,
    pub journal_segments_ref: BlobRef,
    pub consensus: HeadConsensusEvidenceV1,
    pub execution: ExecutionBindingEvidenceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<CheckpointClosureEvidenceV1>,
    #[serde(default = "world_head_proof_claim_boundary_v1")]
    pub claim_boundary: String,
}

impl WorldHeadProofV1 {
    pub fn validate_contract(&self) -> Result<(), String> {
        if self.schema_version != WORLD_HEAD_PROOF_V1_SCHEMA {
            return Err(format!(
                "unsupported world head proof schema: {}",
                self.schema_version
            ));
        }
        if self.claim_boundary != WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1 {
            return Err(format!(
                "unexpected world head proof claim boundary: {}",
                self.claim_boundary
            ));
        }
        if self.world_id != self.head.world_id || self.world_id != self.block.world_id {
            return Err(format!(
                "world_id mismatch: proof={} head={} block={}",
                self.world_id, self.head.world_id, self.block.world_id
            ));
        }
        if self.height != self.head.height || self.height != self.block.height {
            return Err(format!(
                "height mismatch: proof={} head={} block={}",
                self.height, self.head.height, self.block.height
            ));
        }
        if self.timestamp_ms != self.head.timestamp_ms
            || self.timestamp_ms != self.block.timestamp_ms
        {
            return Err(format!(
                "timestamp mismatch: proof={} head={} block={}",
                self.timestamp_ms, self.head.timestamp_ms, self.block.timestamp_ms
            ));
        }
        let computed_block_hash = canonical_blake3_hex(&self.block)
            .map_err(|err| format!("compute world head proof block hash: {err}"))?;
        if self.head.block_hash != computed_block_hash {
            return Err(format!(
                "head block hash mismatch: head={} block={}",
                self.head.block_hash, computed_block_hash
            ));
        }
        if self.head.state_root != self.block.state_root {
            return Err(format!(
                "head state_root mismatch: head={} block={}",
                self.head.state_root, self.block.state_root
            ));
        }
        if self.block.snapshot_ref != self.snapshot_manifest_ref.content_hash {
            return Err(format!(
                "snapshot_ref mismatch: block={} proof={}",
                self.block.snapshot_ref, self.snapshot_manifest_ref.content_hash
            ));
        }
        if self.block.journal_ref != self.journal_segments_ref.content_hash {
            return Err(format!(
                "journal_ref mismatch: block={} proof={}",
                self.block.journal_ref, self.journal_segments_ref.content_hash
            ));
        }
        if self.execution.execution_height != self.height {
            return Err(format!(
                "execution height mismatch: proof={} execution={}",
                self.height, self.execution.execution_height
            ));
        }
        if self.execution.node_block_hash.trim().is_empty() {
            return Err("execution node block hash must not be empty".to_string());
        }
        if self.execution.execution_state_root != self.block.state_root {
            return Err(format!(
                "execution state_root mismatch: execution={} block={}",
                self.execution.execution_state_root, self.block.state_root
            ));
        }
        if self.execution.action_root != self.block.action_root {
            return Err(format!(
                "execution action_root mismatch: execution={} block={}",
                self.execution.action_root, self.block.action_root
            ));
        }
        if self.consensus.consensus_status != "committed" {
            return Err(format!(
                "world head proof requires committed consensus status, got {}",
                self.consensus.consensus_status
            ));
        }
        if self.consensus.proposer_id != self.block.proposer_id {
            return Err(format!(
                "consensus proposer mismatch: consensus={} block={}",
                self.consensus.proposer_id, self.block.proposer_id
            ));
        }
        if let Some(checkpoint) = &self.checkpoint {
            if checkpoint.manifest_ref.is_empty() {
                return Err("checkpoint manifest_ref must not be empty".to_string());
            }
            if !checkpoint
                .pinned_refs
                .iter()
                .any(|reference| reference == &self.snapshot_manifest_ref.content_hash)
            {
                return Err(format!(
                    "checkpoint pinned refs missing snapshot manifest ref: {}",
                    self.snapshot_manifest_ref.content_hash
                ));
            }
            if !checkpoint
                .pinned_refs
                .iter()
                .any(|reference| reference == &self.journal_segments_ref.content_hash)
            {
                return Err(format!(
                    "checkpoint pinned refs missing journal segments ref: {}",
                    self.journal_segments_ref.content_hash
                ));
            }
            if checkpoint.checkpoint_height != self.height {
                return Err(format!(
                    "checkpoint height mismatch: proof={} checkpoint={}",
                    self.height, checkpoint.checkpoint_height
                ));
            }
            if checkpoint.execution_block_hash != self.execution.execution_block_hash {
                return Err(format!(
                    "checkpoint execution block mismatch: checkpoint={} execution={}",
                    checkpoint.execution_block_hash, self.execution.execution_block_hash
                ));
            }
            if checkpoint.execution_state_root != self.execution.execution_state_root {
                return Err(format!(
                    "checkpoint execution state mismatch: checkpoint={} execution={}",
                    checkpoint.execution_state_root, self.execution.execution_state_root
                ));
            }
        }
        Ok(())
    }

    pub fn proof_hash(&self) -> Result<String, String> {
        self.validate_contract()?;
        canonical_blake3_hex(&(WORLD_HEAD_PROOF_HASH_DOMAIN_V1, self))
            .map_err(|err| format!("encode world head proof: {err}"))
    }
}

fn canonical_blake3_hex<T: Serialize>(value: &T) -> Result<String, serde_cbor::Error> {
    let payload = serde_cbor::to_vec(value)?;
    Ok(blake3::hash(&payload).to_hex().to_string())
}

#[cfg(test)]
pub(crate) mod test_support {
    use serde::Serialize;

    pub(crate) fn canonical_blake3_hex<T: Serialize>(
        value: &T,
    ) -> Result<String, serde_cbor::Error> {
        super::canonical_blake3_hex(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRef {
    pub content_hash: String,
    pub size_bytes: u64,
    pub codec: String,
    #[serde(default)]
    pub links: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChunkRef {
    pub chunk_id: String,
    pub content_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub world_id: String,
    pub epoch: u64,
    pub chunks: Vec<StateChunkRef>,
    pub state_root: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageChallengeSampleSource {
    LocalStoreIndex,
    ReplicationCommit,
    GossipReplicaHint,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageChallengeFailureReason {
    MissingSample,
    HashMismatch,
    Timeout,
    ReadIoError,
    SignatureInvalid,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageChallengeProofSemantics {
    pub node_id: String,
    pub sample_source: StorageChallengeSampleSource,
    pub sample_reference: String,
    #[serde(default)]
    pub failure_reason: Option<StorageChallengeFailureReason>,
    #[serde(default)]
    pub proof_kind_hint: String,
    #[serde(default)]
    pub vrf_seed_hint: Option<String>,
    #[serde(default)]
    pub post_commitment_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetWorldHeadRequest {
    pub world_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetWorldHeadResponse {
    pub head: WorldHeadAnnounce,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBlockRequest {
    pub world_id: String,
    pub height: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBlockResponse {
    pub block: WorldBlock,
    pub journal_ref: String,
    pub snapshot_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSnapshotRequest {
    pub world_id: String,
    pub epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSnapshotResponse {
    pub manifest: SnapshotManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchBlobRequest {
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchBlobResponse {
    pub blob: Vec<u8>,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetJournalSegmentRequest {
    pub world_id: String,
    pub from_event_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetJournalSegmentResponse {
    pub segment: BlobRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetReceiptSegmentRequest {
    pub world_id: String,
    pub from_event_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetReceiptSegmentResponse {
    pub segment: BlobRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetModuleManifestRequest {
    pub module_id: String,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetModuleManifestResponse {
    pub manifest_ref: BlobRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetModuleArtifactRequest {
    pub wasm_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetModuleArtifactResponse {
    pub artifact_ref: BlobRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DistributedErrorCode {
    ErrNotFound,
    ErrBadRequest,
    ErrInvalidHash,
    ErrStateMismatch,
    ErrUnsupported,
    ErrUnauthorized,
    ErrBusy,
    ErrRateLimited,
    ErrTimeout,
    ErrNotAvailable,
}

impl DistributedErrorCode {
    pub fn retryable(self) -> bool {
        matches!(
            self,
            DistributedErrorCode::ErrBusy
                | DistributedErrorCode::ErrRateLimited
                | DistributedErrorCode::ErrTimeout
                | DistributedErrorCode::ErrNotAvailable
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: DistributedErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl ErrorResponse {
    pub fn from_code(code: DistributedErrorCode, message: impl Into<String>) -> Self {
        let retryable = code.retryable();
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_helpers_match_expected_format() {
        assert_eq!(topic_action("w1"), "aw.w1.action");
        assert_eq!(topic_block("w1"), "aw.w1.block");
        assert_eq!(topic_head("w1"), "aw.w1.head");
        assert_eq!(topic_event("w1"), "aw.w1.event");
        assert_eq!(topic_membership("w1"), "aw.w1.membership");
        assert_eq!(topic_membership_revocation("w1"), "aw.w1.membership.revoke");
        assert_eq!(
            topic_membership_reconcile("w1"),
            "aw.w1.membership.reconcile"
        );
    }

    #[test]
    fn dht_key_helpers_match_expected_format() {
        assert_eq!(dht_world_head_key("w1"), "/aw/world/w1/head");
        assert_eq!(
            dht_provider_key("w1", "hash"),
            "/aw/world/w1/providers/hash"
        );
        assert_eq!(dht_membership_key("w1"), "/aw/world/w1/membership");
    }

    #[test]
    fn cbor_round_trip_action_envelope() {
        let envelope = ActionEnvelope {
            world_id: "w1".to_string(),
            action_id: "a1".to_string(),
            actor_id: "agent-1".to_string(),
            action_kind: "move".to_string(),
            payload_cbor: vec![1, 2, 3],
            payload_hash: "hash".to_string(),
            nonce: 7,
            timestamp_ms: 123,
            intent_batch_hash: String::new(),
            idempotency_key: String::new(),
            zone_id: String::new(),
            signature: "sig".to_string(),
        };
        let encoded = serde_cbor::to_vec(&envelope).expect("encode action envelope");
        let decoded: ActionEnvelope =
            serde_cbor::from_slice(&encoded).expect("decode action envelope");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn cbor_round_trip_head_announce() {
        let head = WorldHeadAnnounce {
            world_id: "w1".to_string(),
            height: 9,
            block_hash: "b1".to_string(),
            state_root: "s1".to_string(),
            timestamp_ms: 999,
            signature: "sig".to_string(),
        };
        let encoded = serde_cbor::to_vec(&head).expect("encode head");
        let decoded: WorldHeadAnnounce = serde_cbor::from_slice(&encoded).expect("decode head");
        assert_eq!(decoded, head);
    }

    #[test]
    fn cbor_round_trip_storage_challenge_proof_semantics() {
        let semantics = StorageChallengeProofSemantics {
            node_id: "node-a".to_string(),
            sample_source: StorageChallengeSampleSource::LocalStoreIndex,
            sample_reference: "distfs://node-a/tick/10".to_string(),
            failure_reason: Some(StorageChallengeFailureReason::HashMismatch),
            proof_kind_hint: "reserved".to_string(),
            vrf_seed_hint: Some("seed-1".to_string()),
            post_commitment_hint: Some("commit-1".to_string()),
        };
        let encoded = serde_cbor::to_vec(&semantics).expect("encode semantics");
        let decoded: StorageChallengeProofSemantics =
            serde_cbor::from_slice(&encoded).expect("decode semantics");
        assert_eq!(decoded, semantics);
    }

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
        let block_hash = canonical_blake3_hex(&block).expect("block hash");
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

    #[test]
    fn world_head_proof_v1_round_trip_and_hash_validates_contract() {
        let proof = sample_valid_world_head_proof();

        proof.validate_contract().expect("valid proof");
        let first_hash = proof.proof_hash().expect("proof hash");
        let encoded = serde_cbor::to_vec(&proof).expect("encode proof");
        let decoded: WorldHeadProofV1 = serde_cbor::from_slice(&encoded).expect("decode proof");

        assert_eq!(decoded, proof);
        assert_eq!(
            decoded.proof_hash().expect("decoded proof hash"),
            first_hash
        );
    }

    #[test]
    fn world_head_proof_v1_rejects_tampered_head_block_hash() {
        let mut proof = sample_valid_world_head_proof();
        proof.head.block_hash = "wrong-block-hash".to_string();

        let err = proof.validate_contract().expect_err("tamper rejected");
        assert!(err.contains("head block hash mismatch"), "{err}");
    }

    #[test]
    fn world_head_proof_v1_rejects_tampered_block_state_root() {
        let mut proof = sample_valid_world_head_proof();
        proof.block.state_root = "wrong-state-root".to_string();

        let err = proof.validate_contract().expect_err("tamper rejected");
        assert!(err.contains("head block hash mismatch"), "{err}");
    }

    #[test]
    fn world_head_proof_v1_rejects_execution_state_mismatch() {
        let mut proof = sample_valid_world_head_proof();
        proof.execution.execution_state_root = "wrong-execution-state".to_string();

        let err = proof.validate_contract().expect_err("tamper rejected");
        assert!(err.contains("execution state_root mismatch"), "{err}");
    }

    #[test]
    fn world_head_proof_v1_rejects_checkpoint_state_mismatch() {
        let mut proof = sample_valid_world_head_proof();
        proof
            .checkpoint
            .as_mut()
            .expect("checkpoint")
            .execution_state_root = "wrong-checkpoint-state".to_string();

        let err = proof.validate_contract().expect_err("tamper rejected");
        assert!(err.contains("checkpoint execution state mismatch"), "{err}");
    }

    #[test]
    fn error_response_sets_retryable_from_code() {
        let response = ErrorResponse::from_code(DistributedErrorCode::ErrBusy, "busy");
        assert!(response.retryable);
        assert_eq!(response.code, DistributedErrorCode::ErrBusy);
    }
}
