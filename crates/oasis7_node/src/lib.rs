use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub use oasis7_consensus::node_consensus_action::NodeConsensusAction;
use oasis7_consensus::node_consensus_action::{
    compute_consensus_action_root as core_compute_consensus_action_root,
    drain_ordered_consensus_actions as core_drain_ordered_consensus_actions,
    merge_pending_consensus_actions as core_merge_pending_consensus_actions,
    validate_consensus_action_root as core_validate_consensus_action_root,
};
use oasis7_consensus::node_consensus_error::NodeConsensusError;
use oasis7_consensus::node_consensus_signature::{
    sign_attestation_message as core_sign_attestation_message,
    sign_commit_message as core_sign_commit_message,
    sign_proposal_message as core_sign_proposal_message,
    verify_attestation_message_signature as core_verify_attestation_message_signature,
    verify_commit_message_signature as core_verify_commit_message_signature,
    verify_proposal_message_signature as core_verify_proposal_message_signature,
    NodeConsensusMessageSigner,
};
use oasis7_consensus::node_pos::{
    advance_pending_attestations as core_advance_pending_attestations,
    insert_attestation as core_insert_attestation, propose_next_head as core_propose_next_head,
    NodePosDecision, NodePosError, NodePosPendingProposal, NodePosStatusAdapter,
};
use oasis7_distfs::{
    blake3_hex, FeedbackAnnounce, FeedbackAnnounceBridge, FeedbackStore, LocalCasStore,
};
#[cfg(not(target_arch = "wasm32"))]
use oasis7_net::{
    run_replica_maintenance_poll, ReplicaMaintenancePolicy, ReplicaMaintenancePollingPolicy,
    ReplicaMaintenancePollingState, ReplicaTransferExecutor, ReplicaTransferTask,
};
use oasis7_proto::distributed::DistributedErrorCode;
use oasis7_proto::distributed_dht as proto_dht;
use oasis7_proto::world_error::WorldError as ProtoWorldError;
use serde::Deserialize;

mod consensus_support;
mod error;
mod execution_hook;
mod feedback_runtime;
mod gossip_udp;
#[cfg(not(target_arch = "wasm32"))]
mod libp2p_replication_network;
#[cfg(target_arch = "wasm32")]
mod libp2p_replication_network_wasm;
mod network_bridge;
mod node_engine_core;
mod node_engine_network;
mod node_engine_replication;
mod node_engine_storage_challenge;
mod node_runtime_core;
mod pos_engine_gossip;
mod pos_schedule;
mod pos_state_store;
mod pos_validation;
mod replica_maintenance_support;
mod replication;
mod replication_probe_gate;
mod replication_state_reconcile;
mod runtime_util;
mod types;

pub use consensus_support::compute_consensus_action_root;
use consensus_support::{
    checked_consensus_successor, checked_replication_successor, dequeue_pending_consensus_actions,
    drain_ordered_consensus_actions, merge_pending_consensus_actions, node_consensus_error,
    node_pos_error, sign_attestation_message, sign_commit_message, sign_proposal_message,
    validate_consensus_action_root, verify_attestation_message_signature,
    verify_commit_message_signature, verify_proposal_message_signature,
};
pub use error::NodeError;
pub use execution_hook::{
    NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook,
};
use gossip_udp::{
    GossipAttestationMessage, GossipCommitMessage, GossipEndpoint, GossipMessage,
    GossipProposalMessage,
};
pub use gossip_udp::{
    GossipTrafficDirectionMetricsSnapshot, GossipTrafficLaneMetricsSnapshot,
    GossipTrafficMetricsSnapshot,
};
#[cfg(not(target_arch = "wasm32"))]
pub use libp2p_replication_network::{
    derive_libp2p_identity_keypair, Libp2pReplicationNetwork, Libp2pReplicationNetworkConfig,
};
#[cfg(target_arch = "wasm32")]
pub use libp2p_replication_network_wasm::{
    derive_libp2p_identity_keypair, Libp2pReplicationNetwork, Libp2pReplicationNetworkConfig,
};
pub use network_bridge::NodeReplicationNetworkHandle;
pub use oasis7_net::{
    Libp2pControlPlaneMetricsSnapshot, Libp2pReachabilitySnapshot, Libp2pTrafficMetricsSnapshot,
    LiveAutoNatStatus, LiveHolePunchState, LivePublicPortReachability, LiveTransportKind,
};
pub use replication::NodeReplicationConfig;
pub use types::{
    NodeAutoNatStatus, NodeCommittedActionBatch, NodeConfig, NodeConsensusMode,
    NodeConsensusSnapshot, NodeFeedbackP2pConfig, NodeFinalityLatencySnapshot, NodeGossipConfig,
    NodeHolePunchViability, NodeMainTokenControllerBindingConfig,
    NodeMainTokenControllerSignerPolicy, NodeNetworkPolicy, NodePeerCommittedHead,
    NodePendingConsensusActionsSnapshot, NodePendingProposalSnapshot, NodePosConfig,
    NodePublicPortReachability, NodeReachabilityAutoDetection, NodeReplicaMaintenanceConfig,
    NodeRole, NodeSnapshot, NodeUserMode, NodeUserModeRecommendation, PosConsensusStatus,
    PosValidator,
};

use feedback_runtime::{
    maybe_ingest_runtime_feedback_announces, maybe_publish_runtime_feedback_announces,
};
use network_bridge::{ConsensusNetworkEndpoint, ReplicationNetworkEndpoint};
use node_runtime_core::RuntimeState;
use pos_state_store::PosNodeStateStore;
use pos_validation::{normalize_consensus_public_key_hex, validated_pos_state};
use replica_maintenance_support::maybe_run_runtime_replica_maintenance_poll;
use replication::{
    load_blob_from_root, load_commit_message_from_root, FetchBlobRequest, FetchBlobResponse,
    FetchCommitRequest, FetchCommitResponse, ReplicationRuntime, REPLICATION_FETCH_BLOB_PROTOCOL,
    REPLICATION_FETCH_COMMIT_PROTOCOL,
};
use replication_probe_gate::{
    replication_request_waitable_connection_gap, request_fetch_blob_with_route_fallback,
};
use replication_state_reconcile::{
    parse_replication_commit_payload, parse_replication_commit_payload_view,
    reconcile_engine_with_persisted_replication, NodeEngineTickResult,
    ReplicationCommitPayloadView,
};
use runtime_util::{lock_state, now_unix_ms};

const STORAGE_GATE_NETWORK_SAMPLES_PER_CHECK: usize = 3;
const STORAGE_GATE_NETWORK_MIN_MATCHES_CAP: usize = 2;
const STORAGE_GATE_NETWORK_WARMUP_HEIGHT: u64 = 32;
const STORAGE_GATE_FALLBACK_SAMPLES_PER_CHECK: usize = 3;
const STORAGE_CHALLENGE_SUCCESS_CACHE_MAX_AGE_HEIGHTS: u64 = 2;
const REPLICATION_GAP_SYNC_MAX_RETRIES_PER_HEIGHT: usize = 3;
const REPLICATION_FETCH_BLOB_GENERIC_ROUTE_ATTEMPTS: usize = 3;
const EXECUTION_BINDING_HISTORY_LIMIT: usize = 256;
const FINALITY_LATENCY_HISTORY_LIMIT: usize = 128;

fn required_network_blob_matches(sample_count: usize) -> usize {
    sample_count
        .min(STORAGE_GATE_NETWORK_MIN_MATCHES_CAP)
        .max(1)
}

impl NodePosStatusAdapter for PosConsensusStatus {
    fn pending() -> Self {
        PosConsensusStatus::Pending
    }

    fn committed() -> Self {
        PosConsensusStatus::Committed
    }

    fn rejected() -> Self {
        PosConsensusStatus::Rejected
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GapSyncHeightOutcome {
    Synced {
        block_hash: String,
        committed_at_ms: i64,
    },
    NotFound,
}

pub struct NodeRuntime {
    config: NodeConfig,
    replication_network: Option<NodeReplicationNetworkHandle>,
    replication_network_consensus_enabled: bool,
    gossip_endpoint: Option<Arc<GossipEndpoint>>,
    feedback_store: Option<Arc<FeedbackStore>>,
    pending_feedback_announces: Arc<Mutex<Vec<FeedbackAnnounce>>>,
    execution_hook: Option<std::sync::Arc<std::sync::Mutex<Box<dyn NodeExecutionHook>>>>,
    replica_maintenance_dht:
        Option<Arc<dyn proto_dht::DistributedDht<ProtoWorldError> + Send + Sync>>,
    pending_consensus_actions: Arc<Mutex<Vec<NodeConsensusAction>>>,
    committed_action_batches: Arc<(Mutex<Vec<NodeCommittedActionBatch>>, Condvar)>,
    running: Arc<AtomicBool>,
    state: Arc<Mutex<RuntimeState>>,
    stop_tx: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct NodeCommittedActionBatchesHandle {
    state: Arc<(Mutex<Vec<NodeCommittedActionBatch>>, Condvar)>,
}

impl NodeCommittedActionBatchesHandle {
    pub fn wait_for_batches(&self, timeout: Duration) -> bool {
        let (batches_lock, signal) = &*self.state;
        let batches = batches_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !batches.is_empty() {
            return true;
        }
        let (batches, _) = signal
            .wait_timeout_while(batches, timeout, |pending| pending.is_empty())
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        !batches.is_empty()
    }
}

impl NodeRuntime {
    pub fn start(&mut self) -> Result<(), NodeError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(NodeError::AlreadyRunning {
                node_id: self.config.node_id.clone(),
            });
        }

        {
            let mut state = lock_state(&self.state);
            *state = RuntimeState::default();
        }
        {
            let (committed_lock, committed_signal) = &*self.committed_action_batches;
            let mut committed = committed_lock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            committed.clear();
            committed_signal.notify_all();
        }

        let mut engine = match PosNodeEngine::new(&self.config) {
            Ok(engine) => engine,
            Err(err) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(err);
            }
        };
        let effective_replication_config = self
            .config
            .replication
            .as_ref()
            .map(|config| {
                config.clone().with_default_remote_writer_allowlist(
                    self.config
                        .pos_config
                        .validator_signer_public_keys
                        .values()
                        .cloned(),
                )
            })
            .transpose()?;
        let pos_state_store = effective_replication_config
            .as_ref()
            .map(PosNodeStateStore::from_replication);
        if let Some(store) = pos_state_store.as_ref() {
            match store.load() {
                Ok(Some(snapshot)) => {
                    if let Err(err) = engine.restore_state_snapshot(snapshot) {
                        self.running.store(false, Ordering::SeqCst);
                        return Err(err);
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    self.running.store(false, Ordering::SeqCst);
                    return Err(err);
                }
            }
        }
        let gossip = if let Some(config) = &self.config.gossip {
            match GossipEndpoint::bind(config) {
                Ok(endpoint) => Some(Arc::new(endpoint)),
                Err(err) => {
                    self.running.store(false, Ordering::SeqCst);
                    return Err(err);
                }
            }
        } else {
            None
        };
        self.gossip_endpoint = gossip.clone();
        let mut replication = if let Some(config) = effective_replication_config.as_ref() {
            match ReplicationRuntime::new(config, &self.config.node_id) {
                Ok(runtime) => Some(runtime),
                Err(err) => {
                    self.running.store(false, Ordering::SeqCst);
                    return Err(err);
                }
            }
        } else {
            None
        };
        if let Some(replication_runtime) = replication.as_ref() {
            if let Err(err) = reconcile_engine_with_persisted_replication(
                &mut engine,
                replication_runtime,
                self.config.world_id.as_str(),
            ) {
                self.running.store(false, Ordering::SeqCst);
                return Err(err);
            }
            if let Some(store) = pos_state_store.as_ref() {
                if let Err(err) = store.save_engine_state(&engine) {
                    self.running.store(false, Ordering::SeqCst);
                    return Err(err);
                }
            }
        }
        if let (Some(network), Some(replication_config)) = (
            &self.replication_network,
            effective_replication_config.as_ref(),
        ) {
            if let Err(err) = register_replication_fetch_handlers(
                network,
                replication_config,
                self.config.world_id.as_str(),
                &self.config.network_policy,
            ) {
                self.running.store(false, Ordering::SeqCst);
                return Err(err);
            }
        }
        let mut replication_network = if let Some(network) = &self.replication_network {
            let subscribe = !matches!(self.config.role, NodeRole::Sequencer);
            match ReplicationNetworkEndpoint::new(
                network,
                &self.config.world_id,
                subscribe,
                &self.config.network_policy,
            ) {
                Ok(endpoint) => Some(endpoint),
                Err(err) => {
                    self.running.store(false, Ordering::SeqCst);
                    return Err(err);
                }
            }
        } else {
            None
        };
        let mut consensus_network = if let Some(network) = &self.replication_network {
            if self.replication_network_consensus_enabled {
                match ConsensusNetworkEndpoint::new(
                    network,
                    &self.config.world_id,
                    true,
                    &self.config.network_policy,
                ) {
                    Ok(endpoint) => Some(endpoint),
                    Err(err) => {
                        self.running.store(false, Ordering::SeqCst);
                        return Err(err);
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let feedback_p2p = self.config.feedback_p2p.clone();
        let feedback_store = if let Some(feedback_config) = feedback_p2p.as_ref() {
            let Some(replication_config) = effective_replication_config.as_ref() else {
                self.running.store(false, Ordering::SeqCst);
                return Err(NodeError::InvalidConfig {
                    reason: "feedback_p2p requires replication config".to_string(),
                });
            };
            Some(FeedbackStore::new(
                LocalCasStore::new(replication_config.root_dir.join("store")),
                feedback_config.store.clone(),
            ))
        } else {
            None
        };
        let feedback_store = feedback_store.map(Arc::new);
        self.feedback_store = feedback_store.clone();
        let feedback_bridge = if feedback_p2p.is_some() {
            let Some(network) = &self.replication_network else {
                self.running.store(false, Ordering::SeqCst);
                return Err(NodeError::InvalidConfig {
                    reason: "feedback_p2p requires replication network".to_string(),
                });
            };
            if !self.config.network_policy.allows_lane_operation(
                oasis7_proto::distributed_net::NetworkLane::BlobState,
                oasis7_proto::distributed_net::NetworkLaneOperation::Publish,
            ) || !self.config.network_policy.allows_lane_operation(
                oasis7_proto::distributed_net::NetworkLane::BlobState,
                oasis7_proto::distributed_net::NetworkLaneOperation::Subscribe,
            ) {
                self.running.store(false, Ordering::SeqCst);
                return Err(NodeError::InvalidConfig {
                    reason: format!(
                        "feedback_p2p requires blob/state lane publish+subscribe access for node_role_claim={}",
                        self.config.network_policy.node_role_claim
                    ),
                });
            }
            match FeedbackAnnounceBridge::new(
                self.config.world_id.as_str(),
                network.clone_network(),
            ) {
                Ok(bridge) => Some(bridge),
                Err(err) => {
                    self.running.store(false, Ordering::SeqCst);
                    return Err(NodeError::Replication {
                        reason: format!(
                            "feedback announce bridge initialization failed: {:?}",
                            err
                        ),
                    });
                }
            }
        } else {
            None
        };
        let tick_interval = self.config.tick_interval;
        let worker_name = format!("aw-node-{}", self.config.node_id);
        let running = Arc::clone(&self.running);
        let state = Arc::clone(&self.state);
        let execution_hook = self.execution_hook.clone();
        let replica_maintenance = self.config.replica_maintenance;
        let replica_maintenance_dht = self.replica_maintenance_dht.clone();
        let pending_consensus_actions = Arc::clone(&self.pending_consensus_actions);
        let pending_feedback_announces = Arc::clone(&self.pending_feedback_announces);
        let committed_action_batches = Arc::clone(&self.committed_action_batches);
        let node_id = self.config.node_id.clone();
        let world_id = self.config.world_id.clone();
        let max_committed_action_batches = self.config.max_committed_action_batches.max(1);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let worker = thread::Builder::new()
            .name(worker_name)
            .spawn(move || {
                loop {
                    let wait_duration =
                        engine.next_tick_wait_duration(now_unix_ms(), tick_interval);
                    match stop_rx.recv_timeout(wait_duration) {
                        Ok(()) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            let now_ms = now_unix_ms();
                            let last_polled_at_ms = {
                                let mut current = lock_state(&state);
                                current.tick_count = current.tick_count.saturating_add(1);
                                current.last_tick_unix_ms = Some(now_ms);
                                current.replica_maintenance_last_polled_at_ms
                            };

                            let queued_actions = {
                                let mut pending = pending_consensus_actions
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                                dequeue_pending_consensus_actions(
                                    &mut pending,
                                    engine.pending_consensus_action_capacity(),
                                )
                            };
                            let feedback_publish_result = maybe_publish_runtime_feedback_announces(
                                feedback_p2p.as_ref(),
                                pending_feedback_announces.as_ref(),
                                feedback_bridge.as_ref(),
                            );
                            let feedback_ingest_result = maybe_ingest_runtime_feedback_announces(
                                feedback_p2p.as_ref(),
                                feedback_store.as_deref(),
                                feedback_bridge.as_ref(),
                                replication.as_ref(),
                                replication_network.as_ref(),
                            );

                            let tick_result = if let Some(execution_hook) = execution_hook.as_ref()
                            {
                                match execution_hook.lock() {
                                    Ok(mut hook) => engine.tick(
                                        &node_id,
                                        &world_id,
                                        now_ms,
                                        gossip.as_deref(),
                                        replication.as_mut(),
                                        replication_network.as_mut(),
                                        consensus_network.as_mut(),
                                        queued_actions,
                                        Some(hook.as_mut()),
                                    ),
                                    Err(_) => Err(NodeError::Execution {
                                        reason: "execution hook lock poisoned".to_string(),
                                    }),
                                }
                            } else {
                                engine.tick(
                                    &node_id,
                                    &world_id,
                                    now_ms,
                                    gossip.as_deref(),
                                    replication.as_mut(),
                                    replication_network.as_mut(),
                                    consensus_network.as_mut(),
                                    queued_actions,
                                    None,
                                )
                            };
                            let maintenance_result = if tick_result.is_ok() {
                                maybe_run_runtime_replica_maintenance_poll(
                                    replica_maintenance,
                                    node_id.as_str(),
                                    world_id.as_str(),
                                    now_ms,
                                    last_polled_at_ms,
                                    replica_maintenance_dht.as_deref(),
                                    replication_network.as_ref(),
                                    replication.as_ref(),
                                )
                            } else {
                                Ok(last_polled_at_ms)
                            };

                            let mut current = lock_state(&state);
                            match tick_result {
                                Ok(tick) => {
                                    current.consensus = tick.consensus_snapshot;
                                    current.last_error = None;
                                    if let Err(err) = feedback_publish_result.as_ref() {
                                        current.last_error = Some(err.to_string());
                                    }
                                    if let Err(err) = feedback_ingest_result.as_ref() {
                                        if current.last_error.is_none() {
                                            current.last_error = Some(err.to_string());
                                        }
                                    }
                                    match maintenance_result {
                                        Ok(polled_at_ms) => {
                                            current.replica_maintenance_last_polled_at_ms =
                                                polled_at_ms;
                                        }
                                        Err(err) => {
                                            if current.last_error.is_none() {
                                                current.last_error = Some(err.to_string());
                                            }
                                        }
                                    }
                                    if let Some(batch) = tick.committed_action_batch {
                                        let (committed_lock, committed_signal) =
                                            &*committed_action_batches;
                                        let mut committed = committed_lock
                                            .lock()
                                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                                        let retained = max_committed_action_batches - 1;
                                        if committed.len() > retained {
                                            let overflow = committed.len() - retained;
                                            committed.drain(..overflow);
                                        }
                                        committed.push(batch);
                                        committed_signal.notify_all();
                                    }
                                    if let Some(store) = pos_state_store.as_ref() {
                                        if let Err(err) = store.save_engine_state(&engine) {
                                            if current.last_error.is_none() {
                                                current.last_error = Some(err.to_string());
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    current.last_error = Some(err.to_string());
                                }
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                running.store(false, Ordering::SeqCst);
            })
            .map_err(|err| {
                self.running.store(false, Ordering::SeqCst);
                NodeError::ThreadSpawnFailed {
                    reason: err.to_string(),
                }
            })?;

        self.stop_tx = Some(stop_tx);
        self.worker = Some(worker);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), NodeError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(NodeError::NotRunning {
                node_id: self.config.node_id.clone(),
            });
        }
        let (_, committed_signal) = &*self.committed_action_batches;
        committed_signal.notify_all();
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| NodeError::ThreadJoinFailed {
                node_id: self.config.node_id.clone(),
            })?;
        }
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn snapshot(&self) -> NodeSnapshot {
        let state = lock_state(&self.state);
        let mut snapshot = NodeSnapshot {
            node_id: self.config.node_id.clone(),
            player_id: self.config.player_id.clone(),
            world_id: self.config.world_id.clone(),
            role: self.config.role,
            running: self.running.load(Ordering::SeqCst),
            tick_count: state.tick_count,
            last_tick_unix_ms: state.last_tick_unix_ms,
            consensus: state.consensus.clone(),
            last_error: state.last_error.clone(),
        };
        let pending_submit_buffer = self
            .pending_consensus_actions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        snapshot
            .consensus
            .pending_consensus_actions
            .submit_buffer_action_count = pending_submit_buffer.len();
        snapshot
            .consensus
            .pending_consensus_actions
            .submit_buffer_payload_bytes = pending_submit_buffer
            .iter()
            .map(|action| action.payload_cbor.len())
            .sum();
        snapshot
            .consensus
            .pending_consensus_actions
            .submit_buffer_max_capacity = self.config.max_pending_consensus_actions;
        snapshot
    }

    pub fn gossip_traffic_snapshot(&self) -> Option<GossipTrafficMetricsSnapshot> {
        self.gossip_endpoint
            .as_ref()
            .map(|endpoint| endpoint.traffic_metrics_snapshot())
    }
}

impl Drop for NodeRuntime {
    fn drop(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

fn register_replication_fetch_handlers(
    handle: &NodeReplicationNetworkHandle,
    replication: &NodeReplicationConfig,
    world_id: &str,
    network_policy: &NodeNetworkPolicy,
) -> Result<(), NodeError> {
    let network = handle.clone_network();
    if network_policy.allows_lane_operation(
        oasis7_proto::distributed_net::NetworkLane::Sync,
        oasis7_proto::distributed_net::NetworkLaneOperation::Serve,
    ) {
        let commit_root_dir = replication.root_dir.clone();
        let commit_world_id = world_id.to_string();
        let commit_replication_config = replication.clone();
        network
            .register_handler(
                REPLICATION_FETCH_COMMIT_PROTOCOL,
                Box::new(move |payload| {
                    let request =
                        serde_json::from_slice::<FetchCommitRequest>(payload).map_err(|err| {
                            network_bad_request(format!(
                                "decode fetch-commit request failed: {}",
                                err
                            ))
                        })?;
                    if request.world_id != commit_world_id {
                        return Err(network_bad_request(format!(
                            "fetch-commit world mismatch: expected={}, got={}",
                            commit_world_id, request.world_id
                        )));
                    }
                    commit_replication_config
                        .authorize_fetch_commit_request(&request)
                        .map_err(|err| {
                            network_bad_request(format!(
                                "fetch-commit authorization failed: {}",
                                err
                            ))
                        })?;
                    let message = load_commit_message_from_root(
                        commit_root_dir.as_path(),
                        commit_world_id.as_str(),
                        request.height,
                    )
                    .map_err(network_internal_error)?;
                    let response = FetchCommitResponse {
                        found: message.is_some(),
                        message,
                    };
                    serde_json::to_vec(&response).map_err(|err| {
                        network_internal_error(NodeError::Replication {
                            reason: format!("encode fetch-commit response failed: {}", err),
                        })
                    })
                }),
            )
            .map_err(network_replication_error)?;
    }

    if network_policy.allows_lane_operation(
        oasis7_proto::distributed_net::NetworkLane::BlobState,
        oasis7_proto::distributed_net::NetworkLaneOperation::Serve,
    ) {
        let blob_root_dir = replication.root_dir.clone();
        let blob_replication_config = replication.clone();
        network
            .register_handler(
                REPLICATION_FETCH_BLOB_PROTOCOL,
                Box::new(move |payload| {
                    let request =
                        serde_json::from_slice::<FetchBlobRequest>(payload).map_err(|err| {
                            network_bad_request(format!(
                                "decode fetch-blob request failed: {}",
                                err
                            ))
                        })?;
                    blob_replication_config
                        .authorize_fetch_blob_request(&request)
                        .map_err(|err| {
                            network_bad_request(format!("fetch-blob authorization failed: {}", err))
                        })?;
                    let blob =
                        load_blob_from_root(blob_root_dir.as_path(), request.content_hash.as_str())
                            .map_err(network_internal_error)?;
                    let response = FetchBlobResponse {
                        found: blob.is_some(),
                        blob,
                    };
                    serde_json::to_vec(&response).map_err(|err| {
                        network_internal_error(NodeError::Replication {
                            reason: format!("encode fetch-blob response failed: {}", err),
                        })
                    })
                }),
            )
            .map_err(network_replication_error)?;
    }

    Ok(())
}

fn network_bad_request(message: impl Into<String>) -> ProtoWorldError {
    ProtoWorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrBadRequest,
        message: message.into(),
        retryable: false,
    }
}

fn network_internal_error(err: NodeError) -> ProtoWorldError {
    ProtoWorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrNotAvailable,
        message: err.to_string(),
        retryable: true,
    }
}

fn network_replication_error(err: ProtoWorldError) -> NodeError {
    NodeError::Replication {
        reason: format!("replication network error: {err:?}"),
    }
}

#[derive(Debug, Clone)]
struct PosNodeEngine {
    validators: BTreeMap<String, u64>,
    validator_players: BTreeMap<String, String>,
    validator_signers: BTreeMap<String, String>,
    total_stake: u64,
    required_stake: u64,
    epoch_length_slots: u64,
    slot_duration_ms: u64,
    ticks_per_slot: u64,
    proposal_tick_phase: u64,
    adaptive_tick_scheduler_enabled: bool,
    slot_clock_genesis_unix_ms: Option<i64>,
    max_past_slot_lag: u64,
    last_observed_tick: u64,
    missed_tick_count: u64,
    last_observed_slot: u64,
    missed_slot_count: u64,
    local_validator_id: String,
    node_player_id: String,
    gossip_reverse_path_seeding_enabled: bool,
    last_gossip_reverse_path_seed_at_ms: Option<i64>,
    allow_local_proposals: bool,
    require_execution_on_commit: bool,
    next_height: u64,
    next_slot: u64,
    committed_height: u64,
    network_committed_height: u64,
    replication_persisted_height: u64,
    storage_challenge_fallback_height: u64,
    recent_storage_challenge_successes: BTreeMap<String, u64>,
    pending: Option<PendingProposal>,
    auto_attest_all_validators: bool,
    last_broadcast_proposal_height: u64,
    last_broadcast_local_attestation_height: u64,
    last_broadcast_committed_height: u64,
    replicate_local_commits: bool,
    require_peer_execution_hashes: bool,
    consensus_signer: Option<NodeConsensusMessageSigner>,
    enforce_consensus_signature: bool,
    peer_heads: BTreeMap<String, PeerCommittedHead>,
    last_committed_at_ms: Option<i64>,
    last_committed_block_hash: Option<String>,
    inbound_rejected_proposal_future_slot: u64,
    inbound_rejected_proposal_stale_slot: u64,
    inbound_rejected_attestation_future_slot: u64,
    inbound_rejected_attestation_stale_slot: u64,
    inbound_rejected_attestation_epoch_mismatch: u64,
    last_inbound_timing_reject_reason: Option<String>,
    last_execution_height: u64,
    last_execution_block_hash: Option<String>,
    last_execution_state_root: Option<String>,
    recent_finality_latency_ms: VecDeque<i64>,
    execution_bindings: BTreeMap<u64, (String, String)>,
    pending_consensus_actions: BTreeMap<u64, NodeConsensusAction>,
    max_pending_consensus_actions: usize,
}

type PendingProposal = NodePosPendingProposal<NodeConsensusAction, PosConsensusStatus>;
type PosDecision = NodePosDecision<NodeConsensusAction, PosConsensusStatus>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PeerCommittedHead {
    height: u64,
    block_hash: String,
    committed_at_ms: i64,
    execution_block_hash: Option<String>,
    execution_state_root: Option<String>,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_action_payload;
#[cfg(test)]
mod tests_gossip_player;
#[cfg(test)]
mod tests_hardening;
