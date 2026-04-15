use std::fmt;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use oasis7_distfs::{
    build_feedback_announce_from_receipt, FeedbackAppendRequest, FeedbackCreateRequest,
    FeedbackMutationReceipt, FeedbackStore, FeedbackTombstoneRequest, LocalCasStore,
};
use oasis7_proto::distributed_dht as proto_dht;
use oasis7_proto::world_error::WorldError as ProtoWorldError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::runtime_util::now_unix_ms;
use crate::{
    NodeCommittedActionBatch, NodeCommittedActionBatchesHandle, NodeConfig, NodeConsensusAction,
    NodeConsensusSnapshot, NodeError, NodeExecutionHook, NodeMainTokenControllerBindingConfig,
    NodeMainTokenControllerSignerPolicy, NodeReplicationNetworkHandle, NodeRuntime,
};

#[derive(Debug, Clone)]
pub(super) struct RuntimeState {
    pub(super) tick_count: u64,
    pub(super) last_tick_unix_ms: Option<i64>,
    pub(super) replica_maintenance_last_polled_at_ms: Option<i64>,
    pub(super) consensus: NodeConsensusSnapshot,
    pub(super) last_error: Option<String>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            tick_count: 0,
            last_tick_unix_ms: None,
            replica_maintenance_last_polled_at_ms: None,
            consensus: NodeConsensusSnapshot::default(),
            last_error: None,
        }
    }
}

const CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION: u8 = 1;
const MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION: u8 = 1;
const MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX: &str = "octransferauth:v1:";
const MAIN_TOKEN_CLAIM_AUTH_SIGNATURE_V1_PREFIX: &str = "occlaimauth:v1:";
const MAIN_TOKEN_GENESIS_AUTH_SIGNATURE_V1_PREFIX: &str = "ocgenesisauth:v1:";
const MAIN_TOKEN_TREASURY_AUTH_SIGNATURE_V1_PREFIX: &str = "octreasuryauth:v1:";
const MAIN_TOKEN_RESTRICTED_CLAIM_LIVEOPS_POOL_TOP_UP_AUTH_SIGNATURE_V1_PREFIX: &str =
    "ocrestrictedclaimliveopspoolauth:v1:";
const MAIN_TOKEN_RESTRICTED_GRANT_ADMIN_REGISTRY_AUTH_SIGNATURE_V1_PREFIX: &str =
    "ocrestrictedgrantadminauth:v1:";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LocalConsensusActionPayloadEnvelope {
    version: u8,
    #[serde(default)]
    auth: Option<LocalConsensusActionAuthEnvelope>,
    body: LocalConsensusActionPayloadBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum LocalConsensusActionAuthEnvelope {
    MainTokenAction(LocalMainTokenActionAuthProof),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LocalMainTokenActionAuthScheme {
    Ed25519,
    ThresholdEd25519,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalMainTokenActionParticipantSignature {
    public_key: String,
    signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalMainTokenActionAuthProof {
    scheme: LocalMainTokenActionAuthScheme,
    account_id: String,
    #[serde(default)]
    public_key: Option<String>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    threshold: Option<u16>,
    #[serde(default)]
    participant_signatures: Vec<LocalMainTokenActionParticipantSignature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum LocalConsensusActionPayloadBody {
    RuntimeAction {
        action: JsonValue,
    },
    SimulatorAction {
        action: JsonValue,
        #[serde(default)]
        submitter: JsonValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalMainTokenActionSigningEnvelope<'a> {
    version: u8,
    operation: &'static str,
    account_id: &'a str,
    public_key: &'a str,
    action: LocalMainTokenActionSigningPayload<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data")]
enum LocalMainTokenActionSigningPayload<'a> {
    TransferMainToken(LocalTransferMainTokenSigningData<'a>),
    ClaimMainTokenVesting(LocalClaimMainTokenVestingSigningData<'a>),
    InitializeMainTokenGenesis(LocalInitializeMainTokenGenesisSigningData<'a>),
    DistributeMainTokenTreasury(LocalDistributeMainTokenTreasurySigningData<'a>),
    TopUpRestrictedStarterClaimLiveopsPool(
        LocalTopUpRestrictedStarterClaimLiveopsPoolSigningData<'a>,
    ),
    UpdateRestrictedStarterClaimAdminRegistry(
        LocalUpdateRestrictedStarterClaimAdminRegistrySigningData<'a>,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalTransferMainTokenSigningData<'a> {
    from_account_id: &'a str,
    to_account_id: &'a str,
    amount: u64,
    nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalClaimMainTokenVestingSigningData<'a> {
    bucket_id: &'a str,
    beneficiary: &'a str,
    nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalInitializeMainTokenGenesisSigningData<'a> {
    allocations: &'a [JsonValue],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalDistributeMainTokenTreasurySigningData<'a> {
    proposal_id: u64,
    distribution_id: &'a str,
    bucket_id: &'a str,
    distributions: &'a [JsonValue],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalTopUpRestrictedStarterClaimLiveopsPoolSigningData<'a> {
    controller_account_id: &'a str,
    top_up_id: &'a str,
    amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LocalUpdateRestrictedStarterClaimAdminRegistrySigningData<'a> {
    controller_account_id: &'a str,
    next_admin_account_ids: &'a [JsonValue],
}

impl fmt::Debug for NodeRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRuntime")
            .field("config", &self.config)
            .field(
                "has_replication_network",
                &self.replication_network.is_some(),
            )
            .field(
                "replication_network_consensus_enabled",
                &self.replication_network_consensus_enabled,
            )
            .field("has_feedback_store", &self.feedback_store.is_some())
            .field("has_execution_hook", &self.execution_hook.is_some())
            .field("running", &self.running.load(Ordering::SeqCst))
            .finish()
    }
}

impl NodeRuntime {
    pub fn new(config: NodeConfig) -> Self {
        let feedback_store = config.feedback_p2p.as_ref().and_then(|feedback_config| {
            config.replication.as_ref().map(|replication_config| {
                Arc::new(FeedbackStore::new(
                    LocalCasStore::new(replication_config.root_dir.join("store")),
                    feedback_config.store.clone(),
                ))
            })
        });
        Self {
            config,
            replication_network: None,
            replication_network_consensus_enabled: true,
            gossip_endpoint: None,
            feedback_store,
            pending_feedback_announces: Arc::new(Mutex::new(Vec::new())),
            execution_hook: None,
            pending_consensus_actions: Arc::new(Mutex::new(Vec::new())),
            committed_action_batches: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            replica_maintenance_dht: None,
            state: Arc::new(Mutex::new(RuntimeState::default())),
            stop_tx: None,
            worker: None,
        }
    }

    pub fn with_replication_network(mut self, network: NodeReplicationNetworkHandle) -> Self {
        self.replication_network = Some(network);
        self
    }

    pub fn with_replication_network_consensus_enabled(mut self, enabled: bool) -> Self {
        self.replication_network_consensus_enabled = enabled;
        self
    }

    pub fn with_execution_hook<T>(mut self, hook: T) -> Self
    where
        T: NodeExecutionHook + 'static,
    {
        self.execution_hook = Some(Arc::new(Mutex::new(Box::new(hook))));
        self
    }

    pub fn with_replica_maintenance_dht(
        mut self,
        dht: Arc<dyn proto_dht::DistributedDht<ProtoWorldError> + Send + Sync>,
    ) -> Self {
        self.replica_maintenance_dht = Some(dht);
        self
    }

    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    pub fn submit_consensus_action_payload(
        &self,
        action_id: u64,
        payload_cbor: Vec<u8>,
    ) -> Result<(), NodeError> {
        self.submit_consensus_action_payload_as_player(
            self.config.player_id.clone(),
            action_id,
            payload_cbor,
        )
    }

    pub fn submit_consensus_action_payload_as_player(
        &self,
        player_id: impl Into<String>,
        action_id: u64,
        payload_cbor: Vec<u8>,
    ) -> Result<(), NodeError> {
        let player_id = player_id.into();
        let player_id = player_id.trim();
        if player_id.is_empty() {
            return Err(NodeError::Consensus {
                reason: "submitter player_id cannot be empty".to_string(),
            });
        }
        if player_id != self.config.player_id {
            return Err(NodeError::Consensus {
                reason: format!(
                    "submitter player_id mismatch expected={} actual={}",
                    self.config.player_id, player_id
                ),
            });
        }
        if payload_cbor.len() > self.config.max_consensus_action_payload_bytes {
            return Err(NodeError::Consensus {
                reason: format!(
                    "consensus action payload too large: bytes={} limit={}",
                    payload_cbor.len(),
                    self.config.max_consensus_action_payload_bytes
                ),
            });
        }
        validate_consensus_action_payload_auth(&self.config, payload_cbor.as_slice())?;
        let action = NodeConsensusAction::from_payload(
            action_id,
            self.config.player_id.clone(),
            payload_cbor,
        )
        .map_err(|err| NodeError::Consensus { reason: err.reason })?;
        let mut pending = self
            .pending_consensus_actions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if pending.len() >= self.config.max_pending_consensus_actions {
            return Err(NodeError::Consensus {
                reason: format!(
                    "pending consensus actions queue saturated: len={} limit={}",
                    pending.len(),
                    self.config.max_pending_consensus_actions
                ),
            });
        }
        pending.push(action);
        Ok(())
    }

    pub fn submit_feedback(
        &self,
        request: FeedbackCreateRequest,
    ) -> Result<FeedbackMutationReceipt, NodeError> {
        let store = self.require_feedback_store()?;
        let receipt = store
            .submit_feedback(request)
            .map_err(node_feedback_error)?;
        self.enqueue_feedback_announce(store, &receipt)?;
        Ok(receipt)
    }

    pub fn append_feedback(
        &self,
        request: FeedbackAppendRequest,
    ) -> Result<FeedbackMutationReceipt, NodeError> {
        let store = self.require_feedback_store()?;
        let receipt = store
            .append_feedback(request)
            .map_err(node_feedback_error)?;
        self.enqueue_feedback_announce(store, &receipt)?;
        Ok(receipt)
    }

    pub fn tombstone_feedback(
        &self,
        request: FeedbackTombstoneRequest,
    ) -> Result<FeedbackMutationReceipt, NodeError> {
        let store = self.require_feedback_store()?;
        let receipt = store
            .tombstone_feedback(request)
            .map_err(node_feedback_error)?;
        self.enqueue_feedback_announce(store, &receipt)?;
        Ok(receipt)
    }

    pub fn drain_committed_action_batches(&self) -> Vec<NodeCommittedActionBatch> {
        let (committed_lock, _) = &*self.committed_action_batches;
        let mut committed = committed_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *committed)
    }

    pub fn committed_action_batches_handle(&self) -> NodeCommittedActionBatchesHandle {
        NodeCommittedActionBatchesHandle {
            state: Arc::clone(&self.committed_action_batches),
        }
    }

    fn require_feedback_store(&self) -> Result<&Arc<FeedbackStore>, NodeError> {
        self.feedback_store
            .as_ref()
            .ok_or_else(|| NodeError::Replication {
                reason: "feedback_p2p is not configured".to_string(),
            })
    }

    fn enqueue_feedback_announce(
        &self,
        store: &FeedbackStore,
        receipt: &FeedbackMutationReceipt,
    ) -> Result<(), NodeError> {
        let now_ms = now_unix_ms();
        let announce = build_feedback_announce_from_receipt(
            store,
            self.config.world_id.as_str(),
            receipt,
            now_ms,
        )
        .map_err(node_feedback_error)?;
        let max_pending = self
            .config
            .feedback_p2p
            .as_ref()
            .map(|config| {
                config
                    .max_outgoing_announces_per_tick
                    .max(1)
                    .saturating_mul(64)
            })
            .unwrap_or(64);
        let mut pending = self
            .pending_feedback_announces
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if pending.len() >= max_pending {
            return Err(NodeError::Replication {
                reason: format!(
                    "feedback announce queue saturated: len={} limit={}",
                    pending.len(),
                    max_pending
                ),
            });
        }
        pending.push(announce);
        Ok(())
    }
}

fn validate_consensus_action_payload_auth(
    config: &NodeConfig,
    payload_cbor: &[u8],
) -> Result<(), NodeError> {
    let envelope = decode_local_consensus_action_payload_envelope(payload_cbor).map_err(|err| {
        NodeError::Consensus {
            reason: format!("decode consensus action payload auth envelope failed: {err}"),
        }
    })?;
    let LocalConsensusActionPayloadBody::RuntimeAction { action } = &envelope.body else {
        return Ok(());
    };
    if !local_main_token_action_auth_required(action) {
        return Ok(());
    }
    let Some(LocalConsensusActionAuthEnvelope::MainTokenAction(proof)) = envelope.auth.as_ref()
    else {
        return Err(NodeError::Consensus {
            reason: format!(
                "missing_main_token_auth for runtime action {}",
                local_runtime_action_kind(action).unwrap_or("unknown")
            ),
        });
    };
    verify_local_main_token_action_auth(action, proof, &config.main_token_controller_binding)
        .map_err(|err| NodeError::Consensus {
            reason: format!("main token action auth rejected: {err}"),
        })?;
    Ok(())
}

fn decode_local_consensus_action_payload_envelope(
    payload_cbor: &[u8],
) -> Result<LocalConsensusActionPayloadEnvelope, String> {
    match serde_cbor::from_slice::<LocalConsensusActionPayloadEnvelope>(payload_cbor) {
        Ok(envelope) => {
            if envelope.version != CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION {
                return Err(format!(
                    "unsupported consensus payload envelope version {}",
                    envelope.version
                ));
            }
            Ok(envelope)
        }
        Err(_) => Ok(LocalConsensusActionPayloadEnvelope {
            version: CONSENSUS_ACTION_PAYLOAD_ENVELOPE_VERSION,
            auth: None,
            body: LocalConsensusActionPayloadBody::RuntimeAction {
                action: serde_cbor::from_slice::<JsonValue>(payload_cbor)
                    .map_err(|err| format!("legacy runtime action decode failed: {err}"))?,
            },
        }),
    }
}

fn local_main_token_action_auth_required(action: &JsonValue) -> bool {
    matches!(
        local_runtime_action_kind(action),
        Some("TransferMainToken")
            | Some("ClaimMainTokenVesting")
            | Some("InitializeMainTokenGenesis")
            | Some("DistributeMainTokenTreasury")
            | Some("TopUpRestrictedStarterClaimLiveopsPool")
            | Some("UpdateRestrictedStarterClaimAdminRegistry")
    )
}

fn local_runtime_action_kind(action: &JsonValue) -> Option<&str> {
    action.get("type").and_then(JsonValue::as_str)
}

fn local_runtime_action_data(
    action: &JsonValue,
) -> Result<&serde_json::Map<String, JsonValue>, String> {
    action
        .get("data")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "runtime action missing object data".to_string())
}

fn verify_local_main_token_action_auth(
    action: &JsonValue,
    proof: &LocalMainTokenActionAuthProof,
    controller_binding: &NodeMainTokenControllerBindingConfig,
) -> Result<(), String> {
    let account_id =
        normalize_required_field(proof.account_id.as_str(), "main token auth account_id")?;
    match proof.scheme {
        LocalMainTokenActionAuthScheme::Ed25519 => {
            let public_key = normalize_public_key_field(
                proof.public_key.as_deref().ok_or_else(|| {
                    "main token auth public_key is required for ed25519".to_string()
                })?,
                "main token auth public key",
            )?;
            let payload = build_local_main_token_action_signing_payload(
                action,
                account_id.as_str(),
                public_key.as_str(),
            )?;
            validate_local_main_token_action_account_binding(
                action,
                account_id.as_str(),
                Some(public_key.as_str()),
                controller_binding,
            )?;
            enforce_local_controller_signer_policy(
                action,
                account_id.as_str(),
                controller_binding,
                1,
                &[public_key.as_str()],
            )?;
            verify_local_main_token_action_signature(
                action,
                public_key.as_str(),
                proof.signature.as_deref().ok_or_else(|| {
                    "main token auth signature is required for ed25519".to_string()
                })?,
                payload.as_slice(),
            )
        }
        LocalMainTokenActionAuthScheme::ThresholdEd25519 => {
            let threshold = proof.threshold.ok_or_else(|| {
                "main token auth threshold is required for threshold_ed25519".to_string()
            })?;
            if threshold == 0 {
                return Err("main token auth threshold must be > 0".to_string());
            }
            validate_local_main_token_action_account_binding(
                action,
                account_id.as_str(),
                None,
                controller_binding,
            )?;
            if proof.participant_signatures.len() < threshold as usize {
                return Err(format!(
                    "main token auth participant count below threshold: participants={} threshold={threshold}",
                    proof.participant_signatures.len()
                ));
            }
            let mut signer_public_keys = Vec::with_capacity(proof.participant_signatures.len());
            let mut seen = std::collections::BTreeSet::new();
            for participant in &proof.participant_signatures {
                let public_key = normalize_public_key_field(
                    participant.public_key.as_str(),
                    "main token auth participant public key",
                )?;
                if !seen.insert(public_key.clone()) {
                    return Err(format!(
                        "duplicate main token auth participant public key: {public_key}"
                    ));
                }
                let payload = build_local_main_token_action_signing_payload(
                    action,
                    account_id.as_str(),
                    public_key.as_str(),
                )?;
                verify_local_main_token_action_signature(
                    action,
                    public_key.as_str(),
                    participant.signature.as_str(),
                    payload.as_slice(),
                )?;
                signer_public_keys.push(public_key);
            }
            let signer_refs = signer_public_keys
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            enforce_local_controller_signer_policy(
                action,
                account_id.as_str(),
                controller_binding,
                threshold,
                signer_refs.as_slice(),
            )
        }
    }
}

fn build_local_main_token_action_signing_payload(
    action: &JsonValue,
    account_id: &str,
    public_key: &str,
) -> Result<Vec<u8>, String> {
    let envelope = LocalMainTokenActionSigningEnvelope {
        version: MAIN_TOKEN_ACTION_AUTH_PAYLOAD_VERSION,
        operation: local_main_token_action_operation(action)?,
        account_id,
        public_key,
        action: build_local_main_token_action_signing_action(action)?,
    };
    serde_json::to_vec(&envelope)
        .map_err(|err| format!("encode main token auth signing payload failed: {err}"))
}

fn build_local_main_token_action_signing_action(
    action: &JsonValue,
) -> Result<LocalMainTokenActionSigningPayload<'_>, String> {
    let action_kind = local_runtime_action_kind(action)
        .ok_or_else(|| "runtime action missing type".to_string())?;
    let data = local_runtime_action_data(action)?;
    match action_kind {
        "TransferMainToken" => Ok(LocalMainTokenActionSigningPayload::TransferMainToken(
            LocalTransferMainTokenSigningData {
                from_account_id: data
                    .get("from_account_id")
                    .and_then(JsonValue::as_str)
                    .ok_or_else(|| "transfer action missing from_account_id".to_string())?,
                to_account_id: data
                    .get("to_account_id")
                    .and_then(JsonValue::as_str)
                    .ok_or_else(|| "transfer action missing to_account_id".to_string())?,
                amount: data
                    .get("amount")
                    .and_then(JsonValue::as_u64)
                    .ok_or_else(|| "transfer action missing amount".to_string())?,
                nonce: data
                    .get("nonce")
                    .and_then(JsonValue::as_u64)
                    .ok_or_else(|| "transfer action missing nonce".to_string())?,
            },
        )),
        "ClaimMainTokenVesting" => Ok(LocalMainTokenActionSigningPayload::ClaimMainTokenVesting(
            LocalClaimMainTokenVestingSigningData {
                bucket_id: data
                    .get("bucket_id")
                    .and_then(JsonValue::as_str)
                    .ok_or_else(|| "claim action missing bucket_id".to_string())?,
                beneficiary: data
                    .get("beneficiary")
                    .and_then(JsonValue::as_str)
                    .ok_or_else(|| "claim action missing beneficiary".to_string())?,
                nonce: data
                    .get("nonce")
                    .and_then(JsonValue::as_u64)
                    .ok_or_else(|| "claim action missing nonce".to_string())?,
            },
        )),
        "InitializeMainTokenGenesis" => Ok(
            LocalMainTokenActionSigningPayload::InitializeMainTokenGenesis(
                LocalInitializeMainTokenGenesisSigningData {
                    allocations: data
                        .get("allocations")
                        .and_then(JsonValue::as_array)
                        .map(Vec::as_slice)
                        .ok_or_else(|| "genesis action missing allocations".to_string())?,
                },
            ),
        ),
        "DistributeMainTokenTreasury" => Ok(
            LocalMainTokenActionSigningPayload::DistributeMainTokenTreasury(
                LocalDistributeMainTokenTreasurySigningData {
                    proposal_id: data
                        .get("proposal_id")
                        .and_then(JsonValue::as_u64)
                        .ok_or_else(|| "treasury action missing proposal_id".to_string())?,
                    distribution_id: data
                        .get("distribution_id")
                        .and_then(JsonValue::as_str)
                        .ok_or_else(|| "treasury action missing distribution_id".to_string())?,
                    bucket_id: data
                        .get("bucket_id")
                        .and_then(JsonValue::as_str)
                        .ok_or_else(|| "treasury action missing bucket_id".to_string())?,
                    distributions: data
                        .get("distributions")
                        .and_then(JsonValue::as_array)
                        .map(Vec::as_slice)
                        .ok_or_else(|| "treasury action missing distributions".to_string())?,
                },
            ),
        ),
        "TopUpRestrictedStarterClaimLiveopsPool" => Ok(
            LocalMainTokenActionSigningPayload::TopUpRestrictedStarterClaimLiveopsPool(
                LocalTopUpRestrictedStarterClaimLiveopsPoolSigningData {
                    controller_account_id: data
                        .get("controller_account_id")
                        .and_then(JsonValue::as_str)
                        .ok_or_else(|| {
                            "restricted claim liveops pool top-up action missing controller_account_id"
                                .to_string()
                        })?,
                    top_up_id: data
                        .get("top_up_id")
                        .and_then(JsonValue::as_str)
                        .ok_or_else(|| {
                            "restricted claim liveops pool top-up action missing top_up_id"
                                .to_string()
                        })?,
                    amount: data
                        .get("amount")
                        .and_then(JsonValue::as_u64)
                        .ok_or_else(|| {
                            "restricted claim liveops pool top-up action missing amount"
                                .to_string()
                        })?,
                },
            ),
        ),
        "UpdateRestrictedStarterClaimAdminRegistry" => Ok(
            LocalMainTokenActionSigningPayload::UpdateRestrictedStarterClaimAdminRegistry(
                LocalUpdateRestrictedStarterClaimAdminRegistrySigningData {
                    controller_account_id: data
                        .get("controller_account_id")
                        .and_then(JsonValue::as_str)
                        .ok_or_else(|| {
                            "restricted claim admin registry action missing controller_account_id"
                                .to_string()
                        })?,
                    next_admin_account_ids: data
                        .get("next_admin_account_ids")
                        .and_then(JsonValue::as_array)
                        .map(Vec::as_slice)
                        .ok_or_else(|| {
                            "restricted claim admin registry action missing next_admin_account_ids"
                                .to_string()
                        })?,
                },
            ),
        ),
        other => Err(format!(
            "main token auth is not supported for action {other}"
        )),
    }
}

fn validate_local_main_token_action_account_binding(
    action: &JsonValue,
    account_id: &str,
    public_key: Option<&str>,
    controller_binding: &NodeMainTokenControllerBindingConfig,
) -> Result<(), String> {
    let action_kind = local_runtime_action_kind(action).unwrap_or("unknown");
    let data = local_runtime_action_data(action)?;
    match action_kind {
        "TransferMainToken" => {
            let from_account_id = data
                .get("from_account_id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| "transfer action missing from_account_id".to_string())?
                .trim();
            if account_id != from_account_id {
                return Err(format!(
                    "main token auth account_id does not match transfer from_account_id: expected={from_account_id} actual={account_id}"
                ));
            }
            let public_key = public_key.ok_or_else(|| {
                "main token auth public key is required for transfer binding".to_string()
            })?;
            let expected = main_token_account_id_from_public_key(public_key);
            if account_id != expected {
                return Err(format!(
                    "main token auth account_id does not match signer public key: expected={expected} actual={account_id}"
                ));
            }
        }
        "ClaimMainTokenVesting" => {
            let beneficiary = data
                .get("beneficiary")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| "claim action missing beneficiary".to_string())?
                .trim();
            if account_id != beneficiary {
                return Err(format!(
                    "main token auth account_id does not match claim beneficiary: expected={beneficiary} actual={account_id}"
                ));
            }
            if beneficiary.starts_with("oc:pk:") {
                let public_key = public_key.ok_or_else(|| {
                    "main token auth public key is required for oc:pk claim binding".to_string()
                })?;
                let expected = main_token_account_id_from_public_key(public_key);
                if account_id != expected {
                    return Err(format!(
                        "main token auth account_id does not match signer public key: expected={expected} actual={account_id}"
                    ));
                }
            }
        }
        "InitializeMainTokenGenesis" => {
            let expected = controller_binding.genesis_controller_account_id.trim();
            if account_id != expected {
                return Err(format!(
                    "main token auth account_id does not match genesis controller slot: expected={expected} actual={account_id}"
                ));
            }
        }
        "DistributeMainTokenTreasury" => {
            let bucket_id = data
                .get("bucket_id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| "treasury action missing bucket_id".to_string())?
                .trim();
            let expected = controller_binding
                .treasury_bucket_controller_slots
                .get(bucket_id)
                .map(String::as_str)
                .ok_or_else(|| {
                    format!(
                        "main token treasury controller slot is not configured for bucket {bucket_id}"
                    )
                })?;
            if account_id != expected {
                return Err(format!(
                    "main token auth account_id does not match treasury controller slot: bucket={bucket_id} expected={expected} actual={account_id}"
                ));
            }
        }
        "TopUpRestrictedStarterClaimLiveopsPool" => {
            let controller_account_id = data
                .get("controller_account_id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| {
                    "restricted claim liveops pool top-up action missing controller_account_id"
                        .to_string()
                })?
                .trim();
            if account_id != controller_account_id {
                return Err(format!(
                    "main token auth account_id does not match restricted claim liveops pool top-up controller_account_id: expected={controller_account_id} actual={account_id}"
                ));
            }
            let expected = controller_binding
                .treasury_bucket_controller_slots
                .get("ecosystem_pool")
                .map(String::as_str)
                .ok_or_else(|| {
                    "restricted claim liveops pool top-up controller slot is not configured for bucket ecosystem_pool".to_string()
                })?;
            if account_id != expected {
                return Err(format!(
                    "main token auth account_id does not match restricted claim liveops pool top-up controller slot: expected={expected} actual={account_id}"
                ));
            }
        }
        "UpdateRestrictedStarterClaimAdminRegistry" => {
            let controller_account_id = data
                .get("controller_account_id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| {
                    "restricted claim admin registry action missing controller_account_id"
                        .to_string()
                })?
                .trim();
            if account_id != controller_account_id {
                return Err(format!(
                    "main token auth account_id does not match restricted claim admin registry controller_account_id: expected={controller_account_id} actual={account_id}"
                ));
            }
            let expected = controller_binding
                .treasury_bucket_controller_slots
                .get("ecosystem_pool")
                .map(String::as_str)
                .ok_or_else(|| {
                    "restricted claim admin registry controller slot is not configured for bucket ecosystem_pool".to_string()
                })?;
            if account_id != expected {
                return Err(format!(
                    "main token auth account_id does not match restricted claim admin registry controller slot: expected={expected} actual={account_id}"
                ));
            }
        }
        other => {
            return Err(format!(
                "main token auth is not supported for action {other}"
            ))
        }
    }
    Ok(())
}

fn enforce_local_controller_signer_policy(
    action: &JsonValue,
    account_id: &str,
    controller_binding: &NodeMainTokenControllerBindingConfig,
    proof_threshold: u16,
    signer_public_keys: &[&str],
) -> Result<(), String> {
    let Some(policy) =
        local_controller_signer_policy_for_action(action, account_id, controller_binding)?
    else {
        return Ok(());
    };
    if policy.allowed_public_keys.is_empty() {
        return Err(format!(
            "main token controller signer allowlist is empty: controller_account_id={account_id}"
        ));
    }
    if policy.threshold != proof_threshold {
        return Err(format!(
            "main token controller signer threshold mismatch: controller_account_id={account_id} expected={} actual={proof_threshold}",
            policy.threshold
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    for public_key in signer_public_keys {
        if !unique.insert((*public_key).to_string()) {
            continue;
        }
        if !policy.allowed_public_keys.contains(*public_key) {
            return Err(format!(
                "main token controller signer is not allowlisted: controller_account_id={account_id} public_key={public_key}"
            ));
        }
    }
    if unique.len() < usize::from(policy.threshold) {
        return Err(format!(
            "main token controller signer threshold not met: controller_account_id={account_id} unique_signers={} threshold={}",
            unique.len(),
            policy.threshold
        ));
    }
    Ok(())
}

fn local_controller_signer_policy_for_action<'a>(
    action: &JsonValue,
    account_id: &str,
    controller_binding: &'a NodeMainTokenControllerBindingConfig,
) -> Result<Option<&'a NodeMainTokenControllerSignerPolicy>, String> {
    match local_runtime_action_kind(action) {
        Some("InitializeMainTokenGenesis")
        | Some("DistributeMainTokenTreasury")
        | Some("TopUpRestrictedStarterClaimLiveopsPool")
        | Some("UpdateRestrictedStarterClaimAdminRegistry") => {
            controller_binding
                .controller_signer_policies
                .get(account_id)
                .ok_or_else(|| {
                    format!(
                        "main token controller signer policy is not configured: controller_account_id={account_id}"
                    )
                })
                .map(Some)
        }
        Some("TransferMainToken") | Some("ClaimMainTokenVesting") => Ok(None),
        Some(other) => Err(format!("main token auth is not supported for action {other}")),
        None => Err("runtime action missing type".to_string()),
    }
}

fn verify_local_main_token_action_signature(
    action: &JsonValue,
    public_key_hex: &str,
    signature: &str,
    signing_payload: &[u8],
) -> Result<(), String> {
    let public_key_bytes = decode_hex_array::<32>(public_key_hex, "main token auth public key")?;
    let prefix = local_main_token_action_signature_prefix(action)?;
    let signature_hex = signature
        .strip_prefix(prefix)
        .ok_or_else(|| format!("main token auth signature is not {prefix}"))?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "main token auth signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|err| format!("parse main token auth public key failed: {err}"))?;
    verifying_key
        .verify(signing_payload, &Signature::from_bytes(&signature_bytes))
        .map_err(|err| format!("verify main token auth signature failed: {err}"))
}

fn local_main_token_action_operation(action: &JsonValue) -> Result<&'static str, String> {
    match local_runtime_action_kind(action) {
        Some("TransferMainToken") => Ok("transfer_main_token"),
        Some("ClaimMainTokenVesting") => Ok("claim_main_token_vesting"),
        Some("InitializeMainTokenGenesis") => Ok("initialize_main_token_genesis"),
        Some("DistributeMainTokenTreasury") => Ok("distribute_main_token_treasury"),
        Some("TopUpRestrictedStarterClaimLiveopsPool") => {
            Ok("top_up_restricted_starter_claim_liveops_pool")
        }
        Some("UpdateRestrictedStarterClaimAdminRegistry") => {
            Ok("update_restricted_starter_claim_admin_registry")
        }
        Some(other) => Err(format!(
            "main token auth is not supported for action {other}"
        )),
        None => Err("runtime action missing type".to_string()),
    }
}

fn local_main_token_action_signature_prefix(action: &JsonValue) -> Result<&'static str, String> {
    match local_runtime_action_kind(action) {
        Some("TransferMainToken") => Ok(MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX),
        Some("ClaimMainTokenVesting") => Ok(MAIN_TOKEN_CLAIM_AUTH_SIGNATURE_V1_PREFIX),
        Some("InitializeMainTokenGenesis") => Ok(MAIN_TOKEN_GENESIS_AUTH_SIGNATURE_V1_PREFIX),
        Some("DistributeMainTokenTreasury") => Ok(MAIN_TOKEN_TREASURY_AUTH_SIGNATURE_V1_PREFIX),
        Some("TopUpRestrictedStarterClaimLiveopsPool") => {
            Ok(MAIN_TOKEN_RESTRICTED_CLAIM_LIVEOPS_POOL_TOP_UP_AUTH_SIGNATURE_V1_PREFIX)
        }
        Some("UpdateRestrictedStarterClaimAdminRegistry") => {
            Ok(MAIN_TOKEN_RESTRICTED_GRANT_ADMIN_REGISTRY_AUTH_SIGNATURE_V1_PREFIX)
        }
        Some(other) => Err(format!(
            "main token auth is not supported for action {other}"
        )),
        None => Err("runtime action missing type".to_string()),
    }
}

fn normalize_required_field(raw: &str, label: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{label} is empty"));
    }
    Ok(value.to_string())
}

fn normalize_public_key_field(raw: &str, label: &str) -> Result<String, String> {
    let normalized = normalize_required_field(raw, label)?;
    let bytes = decode_hex_array::<32>(normalized.as_str(), label)?;
    Ok(hex::encode(bytes))
}

fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], String> {
    let bytes = hex::decode(raw).map_err(|err| format!("decode {label} failed: {err}"))?;
    if bytes.len() != N {
        return Err(format!(
            "{label} length mismatch: expected {N} bytes, got {}",
            bytes.len()
        ));
    }
    let mut fixed = [0_u8; N];
    fixed.copy_from_slice(bytes.as_slice());
    Ok(fixed)
}

fn main_token_account_id_from_public_key(public_key_hex: &str) -> String {
    format!("oc:pk:{}", public_key_hex.trim().to_ascii_lowercase())
}

fn node_feedback_error(err: ProtoWorldError) -> NodeError {
    NodeError::Replication {
        reason: format!("feedback operation failed: {err:?}"),
    }
}
