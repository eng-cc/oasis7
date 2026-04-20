use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use oasis7_distfs::{
    apply_replication_record, blake3_hex, build_replication_record_with_epoch, BlobStore as _,
    FileReplicationRecord, LocalCasStore, SingleWriterReplicationGuard,
    StorageChallengeProbeConfig, StorageChallengeProbeReport,
};
use oasis7_proto::world_error::WorldError;
use serde::{Deserialize, Serialize};

use crate::{NodeConsensusAction, NodeError, PosConsensusStatus, PosDecision};

const REPLICATION_VERSION: u8 = 1;
const COMMIT_FILE_PREFIX: &str = "consensus/commits";
const COMMIT_MESSAGE_DIR: &str = "replication_commit_messages";
const DEFAULT_WRITER_EPOCH: u64 = 1;
const DEFAULT_MAX_HOT_COMMIT_MESSAGES: usize = 4096;

pub(crate) const REPLICATION_FETCH_COMMIT_PROTOCOL: &str =
    "/aw/node/replication/fetch-commit/1.0.0";
pub(crate) const REPLICATION_FETCH_BLOB_PROTOCOL: &str = "/aw/node/replication/fetch-blob/1.0.0";

mod commit_retention;
#[path = "replication_support.rs"]
mod support;

use self::commit_retention::{
    build_commit_message_retention_plan, has_commit_message_cold_index,
    load_commit_message_cold_index_from_root, write_commit_message_cold_index_to_root,
};
use self::support::{
    distfs_error_to_node_error, fetch_blob_request_signing_bytes,
    fetch_commit_request_signing_bytes, load_json_or_default,
    normalize_replication_public_key_hex_for_config,
    normalize_replication_public_key_hex_for_request, sign_fetch_blob_request,
    sign_fetch_commit_request, sign_replication_message, signing_key_from_hex,
    verify_replication_message_signature, verify_signed_fetch_request, write_json_pretty,
};
pub(crate) use self::support::{load_blob_from_root, load_commit_message_from_root};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeReplicationConfig {
    pub root_dir: PathBuf,
    signing_private_key_hex: Option<String>,
    signing_public_key_hex: Option<String>,
    enforce_signature: bool,
    remote_writer_allowlist: BTreeSet<String>,
    max_hot_commit_messages: usize,
}

impl NodeReplicationConfig {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, NodeError> {
        let root_dir = root_dir.into();
        if root_dir.as_os_str().is_empty() {
            return Err(NodeError::InvalidConfig {
                reason: "replication root_dir cannot be empty".to_string(),
            });
        }
        Ok(Self {
            root_dir,
            signing_private_key_hex: None,
            signing_public_key_hex: None,
            enforce_signature: false,
            remote_writer_allowlist: BTreeSet::new(),
            max_hot_commit_messages: DEFAULT_MAX_HOT_COMMIT_MESSAGES,
        })
    }

    pub fn with_signing_keypair(
        mut self,
        private_key_hex: impl Into<String>,
        public_key_hex: impl Into<String>,
    ) -> Result<Self, NodeError> {
        let private_key_hex = private_key_hex.into();
        let public_key_hex = public_key_hex.into();
        let signing_key = signing_key_from_hex(private_key_hex.as_str())?;
        let expected_public = hex::encode(signing_key.verifying_key().to_bytes());
        if expected_public != public_key_hex {
            return Err(NodeError::InvalidConfig {
                reason: "replication signing public key does not match private key".to_string(),
            });
        }
        self.signing_private_key_hex = Some(private_key_hex);
        self.signing_public_key_hex = Some(public_key_hex);
        self.enforce_signature = true;
        Ok(self)
    }

    pub fn with_remote_writer_allowlist(
        mut self,
        remote_writer_allowlist: Vec<String>,
    ) -> Result<Self, NodeError> {
        let mut normalized = BTreeSet::new();
        for writer_id in remote_writer_allowlist {
            let normalized_writer_id = normalize_replication_public_key_hex_for_config(
                writer_id.as_str(),
                "replication remote_writer_allowlist entry",
            )?;
            normalized.insert(normalized_writer_id);
        }
        self.remote_writer_allowlist = normalized;
        Ok(self)
    }

    pub fn with_max_hot_commit_messages(
        mut self,
        max_hot_commit_messages: usize,
    ) -> Result<Self, NodeError> {
        if max_hot_commit_messages == 0 {
            return Err(NodeError::InvalidConfig {
                reason: "replication max_hot_commit_messages must be positive".to_string(),
            });
        }
        self.max_hot_commit_messages = max_hot_commit_messages;
        Ok(self)
    }

    pub fn max_hot_commit_messages(&self) -> usize {
        self.max_hot_commit_messages
    }

    pub(crate) fn with_default_remote_writer_allowlist(
        mut self,
        defaults: impl IntoIterator<Item = String>,
    ) -> Result<Self, NodeError> {
        if !self.remote_writer_allowlist.is_empty() {
            return Ok(self);
        }
        let mut normalized = BTreeSet::new();
        for writer_id in defaults {
            let normalized_writer_id = normalize_replication_public_key_hex_for_config(
                writer_id.as_str(),
                "replication remote writer default",
            )?;
            normalized.insert(normalized_writer_id);
        }
        self.remote_writer_allowlist = normalized;
        Ok(self)
    }

    fn signing_keypair(&self) -> Result<Option<ReplicationSigningKey>, NodeError> {
        match (
            self.signing_private_key_hex.as_deref(),
            self.signing_public_key_hex.as_deref(),
        ) {
            (Some(private_key_hex), Some(public_key_hex)) => {
                let signing_key = signing_key_from_hex(private_key_hex)?;
                let expected_public = hex::encode(signing_key.verifying_key().to_bytes());
                if expected_public != public_key_hex {
                    return Err(NodeError::InvalidConfig {
                        reason: "replication signing public key does not match private key"
                            .to_string(),
                    });
                }
                Ok(Some(ReplicationSigningKey {
                    signing_key,
                    public_key_hex: public_key_hex.to_string(),
                }))
            }
            (None, None) => Ok(None),
            _ => Err(NodeError::InvalidConfig {
                reason: "replication signing keypair must include both private/public".to_string(),
            }),
        }
    }

    pub(crate) fn consensus_signer(&self) -> Result<Option<(SigningKey, String)>, NodeError> {
        Ok(self
            .signing_keypair()?
            .map(|key| (key.signing_key, key.public_key_hex)))
    }

    pub(crate) fn enforce_consensus_signature(&self) -> bool {
        self.enforce_signature || self.signing_private_key_hex.is_some()
    }

    pub(crate) fn remote_writer_allowlist(&self) -> &BTreeSet<String> {
        &self.remote_writer_allowlist
    }

    pub(crate) fn authorize_fetch_commit_request(
        &self,
        request: &FetchCommitRequest,
    ) -> Result<(), NodeError> {
        let signing_payload = fetch_commit_request_signing_bytes(request)?;
        self.authorize_fetch_request(
            request.requester_public_key_hex.as_deref(),
            request.requester_signature_hex.as_deref(),
            signing_payload.as_slice(),
            "fetch-commit request",
        )
    }

    pub(crate) fn authorize_fetch_blob_request(
        &self,
        request: &FetchBlobRequest,
    ) -> Result<(), NodeError> {
        let signing_payload = fetch_blob_request_signing_bytes(request)?;
        self.authorize_fetch_request(
            request.requester_public_key_hex.as_deref(),
            request.requester_signature_hex.as_deref(),
            signing_payload.as_slice(),
            "fetch-blob request",
        )
    }

    fn authorize_fetch_request(
        &self,
        requester_public_key_hex: Option<&str>,
        requester_signature_hex: Option<&str>,
        signing_payload: &[u8],
        request_label: &str,
    ) -> Result<(), NodeError> {
        let require_auth = self.enforce_consensus_signature()
            || !self.remote_writer_allowlist.is_empty()
            || requester_public_key_hex.is_some()
            || requester_signature_hex.is_some();
        if !require_auth {
            return Ok(());
        }
        if self.remote_writer_allowlist.is_empty() {
            return Err(NodeError::Replication {
                reason: format!(
                    "{request_label} authorization failed: replication remote writer allowlist is empty while signature enforcement is enabled"
                ),
            });
        }
        let requester_public_key_hex =
            requester_public_key_hex.ok_or_else(|| NodeError::Replication {
                reason: format!(
                    "{request_label} authorization failed: missing requester_public_key_hex"
                ),
            })?;
        let requester_signature_hex =
            requester_signature_hex.ok_or_else(|| NodeError::Replication {
                reason: format!(
                    "{request_label} authorization failed: missing requester_signature_hex"
                ),
            })?;
        let normalized_requester_public_key_hex = normalize_replication_public_key_hex_for_request(
            requester_public_key_hex,
            &format!("{request_label} requester public key"),
        )?;
        verify_signed_fetch_request(
            normalized_requester_public_key_hex.as_str(),
            requester_signature_hex,
            signing_payload,
            request_label,
        )?;
        if !self
            .remote_writer_allowlist
            .contains(normalized_requester_public_key_hex.as_str())
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "{request_label} requester is not authorized: requester_public_key_hex={normalized_requester_public_key_hex}"
                ),
            });
        }
        Ok(())
    }

    fn store_root(&self) -> PathBuf {
        self.root_dir.join("store")
    }

    fn guard_state_path(&self) -> PathBuf {
        self.root_dir.join("replication_guard.json")
    }

    fn writer_state_path(&self, node_id: &str) -> PathBuf {
        self.root_dir
            .join(format!("replication_writer_state_{node_id}.json"))
    }

    fn commit_message_path(&self, height: u64) -> PathBuf {
        self.root_dir
            .join(COMMIT_MESSAGE_DIR)
            .join(format!("{:020}.json", height))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchCommitRequest {
    pub world_id: String,
    pub height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_public_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchCommitResponse {
    pub found: bool,
    pub message: Option<GossipReplicationMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchBlobRequest {
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_public_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchBlobResponse {
    pub found: bool,
    pub blob: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GossipReplicationMessage {
    pub version: u8,
    pub world_id: String,
    pub node_id: String,
    pub record: FileReplicationRecord,
    pub payload: Vec<u8>,
    pub public_key_hex: Option<String>,
    pub signature_hex: Option<String>,
}

#[derive(Debug, Clone)]
struct ReplicationSigningKey {
    signing_key: SigningKey,
    public_key_hex: String,
}

#[derive(Debug)]
pub(crate) struct ReplicationRuntime {
    config: NodeReplicationConfig,
    store: LocalCasStore,
    guard: SingleWriterReplicationGuard,
    writer_state: LocalWriterState,
    signer: Option<ReplicationSigningKey>,
    enforce_signature: bool,
    remote_writer_allowlist: BTreeSet<String>,
}

fn default_writer_epoch() -> u64 {
    DEFAULT_WRITER_EPOCH
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalWriterState {
    #[serde(default = "default_writer_epoch")]
    writer_epoch: u64,
    last_sequence: u64,
    last_replicated_height: u64,
}

impl Default for LocalWriterState {
    fn default() -> Self {
        Self {
            writer_epoch: DEFAULT_WRITER_EPOCH,
            last_sequence: 0,
            last_replicated_height: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReplicatedCommitPayload {
    world_id: String,
    node_id: String,
    height: u64,
    slot: u64,
    epoch: u64,
    block_hash: String,
    action_root: String,
    actions: Vec<NodeConsensusAction>,
    committed_at_ms: i64,
    execution_block_hash: Option<String>,
    execution_state_root: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReplicationSigningPayload<'a> {
    version: u8,
    world_id: &'a str,
    node_id: &'a str,
    record: &'a FileReplicationRecord,
    payload: &'a [u8],
    public_key_hex: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct FetchCommitRequestSigningPayload<'a> {
    version: u8,
    world_id: &'a str,
    height: u64,
    requester_public_key_hex: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct FetchBlobRequestSigningPayload<'a> {
    version: u8,
    content_hash: &'a str,
    requester_public_key_hex: Option<&'a str>,
}

impl ReplicationRuntime {
    pub(crate) fn new(config: &NodeReplicationConfig, node_id: &str) -> Result<Self, NodeError> {
        fs::create_dir_all(&config.root_dir).map_err(|err| NodeError::Replication {
            reason: format!(
                "create replication root {} failed: {}",
                config.root_dir.display(),
                err
            ),
        })?;

        let guard = load_json_or_default::<SingleWriterReplicationGuard>(
            config.guard_state_path().as_path(),
        )?;
        let mut writer_state =
            load_json_or_default::<LocalWriterState>(config.writer_state_path(node_id).as_path())?;
        if writer_state.writer_epoch == 0 {
            writer_state.writer_epoch = DEFAULT_WRITER_EPOCH;
        }
        if writer_state.last_sequence == 0
            && writer_state.last_replicated_height == 0
            && writer_state.writer_epoch == DEFAULT_WRITER_EPOCH
        {
            writer_state.writer_epoch = seeded_writer_epoch();
        }
        let signer = config.signing_keypair()?;

        Ok(Self {
            config: config.clone(),
            store: LocalCasStore::new(config.store_root()),
            guard,
            writer_state,
            enforce_signature: config.enforce_signature || signer.is_some(),
            remote_writer_allowlist: config.remote_writer_allowlist().clone(),
            signer,
        })
    }

    pub(crate) fn build_local_commit_message(
        &mut self,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        decision: &PosDecision,
        execution_block_hash: Option<&str>,
        execution_state_root: Option<&str>,
    ) -> Result<Option<GossipReplicationMessage>, NodeError> {
        if !matches!(decision.status, PosConsensusStatus::Committed) {
            return Ok(None);
        }
        if decision.height <= self.writer_state.last_replicated_height {
            return Ok(None);
        }
        if execution_block_hash.is_some() != execution_state_root.is_some() {
            return Err(NodeError::Replication {
                reason: "replication execution hash binding requires both block/state".to_string(),
            });
        }

        let payload = ReplicatedCommitPayload {
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            height: decision.height,
            slot: decision.slot,
            epoch: decision.epoch,
            block_hash: decision.block_hash.clone(),
            action_root: decision.action_root.clone(),
            actions: decision.committed_actions.clone(),
            committed_at_ms: now_ms,
            execution_block_hash: execution_block_hash.map(str::to_string),
            execution_state_root: execution_state_root.map(str::to_string),
        };
        let payload_bytes = serde_json::to_vec(&payload).map_err(|err| NodeError::Replication {
            reason: format!("serialize local replication payload failed: {}", err),
        })?;
        let writer_id = self
            .signer
            .as_ref()
            .map(|signer| signer.public_key_hex.as_str())
            .unwrap_or(node_id);
        let (writer_epoch, sequence) = self.next_local_record_position(writer_id)?;
        let path = format!("{COMMIT_FILE_PREFIX}/{:020}.json", decision.height);
        let record = build_replication_record_with_epoch(
            world_id,
            writer_id,
            writer_epoch,
            sequence,
            path.as_str(),
            &payload_bytes,
            now_ms,
        )
        .map_err(distfs_error_to_node_error)?;

        apply_replication_record(&self.store, &mut self.guard, &record, &payload_bytes)
            .map_err(distfs_error_to_node_error)?;

        self.writer_state.writer_epoch = record.writer_epoch;
        self.writer_state.last_sequence = record.sequence;
        self.writer_state.last_replicated_height = decision.height;
        self.persist_state(node_id)?;

        let mut message = GossipReplicationMessage {
            version: REPLICATION_VERSION,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            record,
            payload: payload_bytes,
            public_key_hex: self
                .signer
                .as_ref()
                .map(|signer| signer.public_key_hex.clone()),
            signature_hex: None,
        };

        if let Some(signer) = &self.signer {
            let signature_hex = sign_replication_message(&message, signer)?;
            message.signature_hex = Some(signature_hex);
        }
        self.persist_commit_message(decision.height, &message)?;

        Ok(Some(message))
    }

    pub(crate) fn apply_remote_message(
        &mut self,
        node_id: &str,
        world_id: &str,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        if !self.should_process_remote_message(node_id, world_id, message) {
            return Ok(());
        }
        self.validate_remote_message_signature_binding(message)?;
        self.validate_remote_message_payload_hash(message)?;

        if self.is_stale_remote_record(&message.record) {
            return Ok(());
        }

        apply_replication_record(
            &self.store,
            &mut self.guard,
            &message.record,
            &message.payload,
        )
        .map_err(distfs_error_to_node_error)?;

        if let Some(height) = commit_height_from_payload(message.payload.as_slice()) {
            self.persist_commit_message(height, message)?;
        }

        write_json_pretty(self.config.guard_state_path().as_path(), &self.guard)
    }

    pub(crate) fn validate_remote_message_for_observe(
        &self,
        node_id: &str,
        world_id: &str,
        message: &GossipReplicationMessage,
    ) -> Result<bool, NodeError> {
        if !self.should_process_remote_message(node_id, world_id, message) {
            return Ok(false);
        }
        self.validate_remote_message_signature_binding(message)?;
        self.validate_remote_message_payload_hash(message)?;
        Ok(true)
    }

    fn persist_state(&self, node_id: &str) -> Result<(), NodeError> {
        write_json_pretty(self.config.guard_state_path().as_path(), &self.guard)?;
        write_json_pretty(
            self.config.writer_state_path(node_id).as_path(),
            &self.writer_state,
        )
    }

    fn next_local_record_position(&self, writer_id: &str) -> Result<(u64, u64), NodeError> {
        let guard_epoch = self.guard.writer_epoch.max(DEFAULT_WRITER_EPOCH);
        let state_epoch = self.writer_state.writer_epoch.max(DEFAULT_WRITER_EPOCH);
        match self.guard.writer_id.as_deref() {
            Some(existing_writer) if existing_writer == writer_id => {
                let writer_epoch = guard_epoch.max(state_epoch);
                let guard_sequence = if self.guard.writer_epoch == writer_epoch {
                    self.guard.last_sequence
                } else {
                    0
                };
                let writer_state_sequence = if self.writer_state.writer_epoch == writer_epoch {
                    self.writer_state.last_sequence
                } else {
                    0
                };
                let sequence = checked_replication_counter_increment(
                    guard_sequence.max(writer_state_sequence),
                    "sequence",
                    "advancing local replication record for existing writer",
                )?;
                Ok((writer_epoch, sequence))
            }
            Some(_) => {
                let writer_epoch = checked_replication_counter_increment(
                    guard_epoch.max(state_epoch),
                    "writer_epoch",
                    "switching local replication writer",
                )?;
                Ok((writer_epoch, 1))
            }
            None => {
                let writer_epoch = state_epoch;
                let sequence = if self.writer_state.writer_epoch == writer_epoch {
                    checked_replication_counter_increment(
                        self.writer_state.last_sequence,
                        "sequence",
                        "advancing local replication record without guard writer",
                    )?
                } else {
                    1
                };
                Ok((writer_epoch, sequence))
            }
        }
    }

    fn is_stale_remote_record(&self, record: &FileReplicationRecord) -> bool {
        let local_epoch = self.guard.writer_epoch.max(DEFAULT_WRITER_EPOCH);
        if record.writer_epoch < local_epoch {
            return true;
        }
        if record.writer_epoch == local_epoch
            && self.guard.writer_id.as_deref() == Some(record.writer_id.as_str())
            && record.sequence <= self.guard.last_sequence
        {
            return true;
        }
        false
    }

    fn persist_commit_message(
        &self,
        height: u64,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        write_json_pretty(self.config.commit_message_path(height).as_path(), message)?;
        self.prune_hot_commit_messages()
    }

    fn prune_hot_commit_messages(&self) -> Result<(), NodeError> {
        let retention_plan = build_commit_message_retention_plan(
            self.config.root_dir.as_path(),
            self.config.max_hot_commit_messages,
        )?;
        let had_cold_index = has_commit_message_cold_index(self.config.root_dir.as_path());

        let mut offloaded = Vec::new();
        for candidate in retention_plan.offload_candidates {
            let bytes = fs::read(&candidate.path).map_err(|err| NodeError::Replication {
                reason: format!("read {} failed: {}", candidate.path.display(), err),
            })?;
            let content_hash = blake3_hex(bytes.as_slice());
            self.store
                .put(content_hash.as_str(), bytes.as_slice())
                .map_err(distfs_error_to_node_error)?;
            offloaded.push((candidate.height, content_hash, candidate.path));
        }
        if offloaded.is_empty() && !had_cold_index {
            return Ok(());
        }

        let mut cold_index =
            load_commit_message_cold_index_from_root(self.config.root_dir.as_path())?;
        for (height, content_hash, _) in &offloaded {
            cold_index.by_height.insert(*height, content_hash.clone());
        }
        cold_index.refresh_metadata(&retention_plan.hot_window);
        write_commit_message_cold_index_to_root(self.config.root_dir.as_path(), &cold_index)?;

        for (_, _, path) in offloaded {
            fs::remove_file(&path).map_err(|err| NodeError::Replication {
                reason: format!("remove {} failed: {}", path.display(), err),
            })?;
        }
        Ok(())
    }

    pub(crate) fn load_commit_message_by_height(
        &self,
        world_id: &str,
        height: u64,
    ) -> Result<Option<GossipReplicationMessage>, NodeError> {
        load_commit_message_from_root(self.config.root_dir.as_path(), world_id, height)
    }

    pub(crate) fn latest_persisted_commit_height(&self, world_id: &str) -> Result<u64, NodeError> {
        let hot_height = build_commit_message_retention_plan(
            self.config.root_dir.as_path(),
            self.config.max_hot_commit_messages,
        )?
        .hot_window
        .latest_height
        .unwrap_or(0);
        let cold_height = load_commit_message_cold_index_from_root(self.config.root_dir.as_path())?
            .by_height
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0);
        let mut candidate = hot_height.max(cold_height);
        while candidate > 0 {
            if self
                .load_commit_message_by_height(world_id, candidate)?
                .is_some()
            {
                return Ok(candidate);
            }
            candidate -= 1;
        }
        Ok(0)
    }

    pub(crate) fn load_blob_by_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Vec<u8>>, NodeError> {
        load_blob_from_root(self.config.root_dir.as_path(), content_hash)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn store_blob_by_hash(
        &self,
        content_hash: &str,
        blob: &[u8],
    ) -> Result<(), NodeError> {
        self.store
            .put(content_hash, blob)
            .map_err(distfs_error_to_node_error)
    }

    pub(crate) fn build_fetch_commit_request(
        &self,
        world_id: &str,
        height: u64,
    ) -> Result<FetchCommitRequest, NodeError> {
        let mut request = FetchCommitRequest {
            world_id: world_id.to_string(),
            height,
            requester_public_key_hex: self
                .signer
                .as_ref()
                .map(|signer| signer.public_key_hex.clone()),
            requester_signature_hex: None,
        };
        if let Some(signer) = &self.signer {
            request.requester_signature_hex = Some(sign_fetch_commit_request(&request, signer)?);
        }
        Ok(request)
    }

    pub(crate) fn build_fetch_blob_request(
        &self,
        content_hash: &str,
    ) -> Result<FetchBlobRequest, NodeError> {
        let mut request = FetchBlobRequest {
            content_hash: content_hash.to_string(),
            requester_public_key_hex: self
                .signer
                .as_ref()
                .map(|signer| signer.public_key_hex.clone()),
            requester_signature_hex: None,
        };
        if let Some(signer) = &self.signer {
            request.requester_signature_hex = Some(sign_fetch_blob_request(&request, signer)?);
        }
        Ok(request)
    }

    pub(crate) fn probe_storage_challenges(
        &self,
        world_id: &str,
        node_id: &str,
        observed_at_unix_ms: i64,
    ) -> Result<StorageChallengeProbeReport, NodeError> {
        let config = StorageChallengeProbeConfig::default();
        self.store
            .probe_storage_challenges(world_id, node_id, observed_at_unix_ms, &config)
            .map_err(distfs_error_to_node_error)
    }

    pub(crate) fn recent_replicated_content_hashes(
        &self,
        world_id: &str,
        max_samples: usize,
    ) -> Result<Vec<String>, NodeError> {
        Ok(self
            .recent_replicated_content_refs(world_id, max_samples)?
            .into_iter()
            .map(|(_, content_hash)| content_hash)
            .collect())
    }

    pub(crate) fn recent_replicated_content_refs(
        &self,
        world_id: &str,
        max_samples: usize,
    ) -> Result<Vec<(u64, String)>, NodeError> {
        if max_samples == 0 || self.writer_state.last_replicated_height == 0 {
            return Ok(Vec::new());
        }

        let mut samples = Vec::with_capacity(max_samples);
        let mut seen = BTreeSet::new();
        let mut height = self.writer_state.last_replicated_height;
        while height > 0 && samples.len() < max_samples {
            if let Some(message) = self.load_commit_message_by_height(world_id, height)? {
                let content_hash = message.record.content_hash.trim();
                if !content_hash.is_empty() && seen.insert(content_hash.to_string()) {
                    samples.push((height, content_hash.to_string()));
                }
            }
            height -= 1;
        }
        Ok(samples)
    }

    pub(crate) fn replicated_content_refs_from_height(
        &self,
        world_id: &str,
        start_height: u64,
        max_samples: usize,
    ) -> Result<Vec<(u64, String)>, NodeError> {
        if max_samples == 0 || self.writer_state.last_replicated_height == 0 {
            return Ok(Vec::new());
        }

        let mut height = start_height.max(1);
        let latest_height = self.writer_state.last_replicated_height;
        if height > latest_height {
            return Ok(Vec::new());
        }

        let mut samples = Vec::with_capacity(max_samples);
        let mut seen = BTreeSet::new();
        while height <= latest_height && samples.len() < max_samples {
            if let Some(message) = self.load_commit_message_by_height(world_id, height)? {
                let content_hash = message.record.content_hash.trim();
                if !content_hash.is_empty() && seen.insert(content_hash.to_string()) {
                    samples.push((height, content_hash.to_string()));
                }
            }
            height = match height.checked_add(1) {
                Some(next_height) => next_height,
                None => break,
            };
        }
        Ok(samples)
    }

    fn should_process_remote_message(
        &self,
        node_id: &str,
        world_id: &str,
        message: &GossipReplicationMessage,
    ) -> bool {
        if message.version != REPLICATION_VERSION {
            return false;
        }
        if message.node_id == node_id {
            return false;
        }
        if message.world_id != world_id || message.record.world_id != world_id {
            return false;
        }
        true
    }

    fn validate_remote_message_signature_binding(
        &self,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        if self.enforce_signature
            || !self.remote_writer_allowlist.is_empty()
            || message.signature_hex.is_some()
            || message.public_key_hex.is_some()
        {
            verify_replication_message_signature(message)?;
            if let Some(public_key_hex) = message.public_key_hex.as_deref() {
                if message.record.writer_id != public_key_hex {
                    return Err(NodeError::Replication {
                        reason: "replication writer_id does not match signature public key"
                            .to_string(),
                    });
                }
            }
        }
        self.validate_remote_writer_authorization(message.record.writer_id.as_str())?;
        Ok(())
    }

    fn validate_remote_writer_authorization(&self, writer_id: &str) -> Result<(), NodeError> {
        if self.remote_writer_allowlist.is_empty() {
            if self.enforce_signature {
                return Err(NodeError::Replication {
                    reason: "replication remote writer allowlist is empty while signature enforcement is enabled"
                        .to_string(),
                });
            }
            return Ok(());
        }
        if !self.remote_writer_allowlist.contains(writer_id) {
            return Err(NodeError::Replication {
                reason: format!(
                    "replication remote writer is not authorized: writer_id={writer_id}"
                ),
            });
        }
        Ok(())
    }

    fn validate_remote_message_payload_hash(
        &self,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        let computed_hash = blake3_hex(message.payload.as_slice());
        if computed_hash != message.record.content_hash {
            return Err(NodeError::Replication {
                reason: format!(
                    "replication payload hash mismatch expected={} actual={}",
                    message.record.content_hash, computed_hash
                ),
            });
        }
        Ok(())
    }
}

fn checked_replication_counter_increment(
    current: u64,
    field: &str,
    context: &str,
) -> Result<u64, NodeError> {
    current
        .checked_add(1)
        .ok_or_else(|| NodeError::Replication {
            reason: format!("{field} overflow while {context}: current={current}"),
        })
}

fn seeded_writer_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(DEFAULT_WRITER_EPOCH)
        .max(DEFAULT_WRITER_EPOCH)
}

fn commit_height_from_payload(payload: &[u8]) -> Option<u64> {
    serde_json::from_slice::<ReplicatedCommitPayload>(payload)
        .ok()
        .map(|parsed| parsed.height)
        .filter(|height| *height > 0)
}

#[cfg(test)]
mod tests;
