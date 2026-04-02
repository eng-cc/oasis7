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

    #[test]
    fn error_response_sets_retryable_from_code() {
        let response = ErrorResponse::from_code(DistributedErrorCode::ErrBusy, "busy");
        assert!(response.retryable);
        assert_eq!(response.code, DistributedErrorCode::ErrBusy);
    }
}
