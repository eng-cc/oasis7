use super::gossip_udp::{GossipCommitMessage, GossipEndpoint, GossipMessage};
use super::*;
use oasis7_consensus::node_consensus_signature::{
    sign_attestation_message, sign_commit_message, sign_proposal_message,
    verify_commit_message_signature, NodeConsensusMessageSigner as ConsensusMessageSigner,
};
use oasis7_distfs::{FileStore as _, LocalCasStore, SingleWriterReplicationGuard};
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::{
    self as proto_dht, MembershipDirectorySnapshot, ProviderRecord, SignedPeerRecord,
};
use oasis7_proto::distributed_net::NetworkSubscription;
use oasis7_proto::world_error::WorldError;
use ed25519_dalek::{Signer as _, SigningKey};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "tests_commit_execution_hashes.rs"]
mod commit_execution_hashes_tests;

fn multi_validators() -> Vec<PosValidator> {
    vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 40,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 35,
        },
        PosValidator {
            validator_id: "node-c".to_string(),
            stake: 25,
        },
    ]
}

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-node-tests-{prefix}-{unique}"))
}

fn deterministic_keypair_hex(seed: u8) -> (String, String) {
    let bytes = [seed; 32];
    let signing_key = SigningKey::from_bytes(&bytes);
    (
        hex::encode(signing_key.to_bytes()),
        hex::encode(signing_key.verifying_key().to_bytes()),
    )
}

fn empty_action_root() -> String {
    compute_consensus_action_root(&[]).expect("empty action root")
}

fn gossip_config(
    bind_addr: std::net::SocketAddr,
    peers: Vec<std::net::SocketAddr>,
) -> NodeGossipConfig {
    NodeGossipConfig {
        bind_addr,
        peers,
        max_dynamic_peers: 1024,
        dynamic_peer_ttl_ms: 10 * 60 * 1000,
    }
}

fn signed_replication_config(root_dir: PathBuf, seed: u8) -> NodeReplicationConfig {
    let (private_hex, public_hex) = deterministic_keypair_hex(seed);
    NodeReplicationConfig::new(root_dir)
        .expect("replication config")
        .with_signing_keypair(private_hex, public_hex)
        .expect("signing keypair")
}

fn signed_pos_config_with_signer_seeds(
    validators: Vec<PosValidator>,
    signer_seeds: &[(&str, u8)],
) -> NodePosConfig {
    let seed_map = signer_seeds
        .iter()
        .map(|(validator_id, seed)| ((*validator_id).to_string(), *seed))
        .collect::<HashMap<_, _>>();
    let mut signer_map = BTreeMap::new();
    for validator in &validators {
        let seed = seed_map
            .get(validator.validator_id.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "missing signer seed for validator {}",
                    validator.validator_id
                )
            });
        let (_, public_key_hex) = deterministic_keypair_hex(*seed);
        signer_map.insert(validator.validator_id.clone(), public_key_hex);
    }
    NodePosConfig::ethereum_like(validators)
        .with_validator_signer_public_keys(signer_map)
        .expect("signed pos config")
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

fn signed_fetch_commit_request_for_test(
    world_id: &str,
    height: u64,
    signer_seed: u8,
) -> super::replication::FetchCommitRequest {
    let (private_hex, public_hex) = deterministic_keypair_hex(signer_seed);
    let signing_key_bytes: [u8; 32] = hex::decode(private_hex)
        .expect("private key decode")
        .try_into()
        .expect("private key len");
    let signing_key = SigningKey::from_bytes(&signing_key_bytes);
    let mut request = super::replication::FetchCommitRequest {
        world_id: world_id.to_string(),
        height,
        requester_public_key_hex: Some(public_hex),
        requester_signature_hex: None,
    };
    let payload = FetchCommitRequestSigningPayload {
        version: 1,
        world_id: request.world_id.as_str(),
        height: request.height,
        requester_public_key_hex: request.requester_public_key_hex.as_deref(),
    };
    let payload_bytes = serde_json::to_vec(&payload).expect("encode fetch-commit signing payload");
    let signature = signing_key.sign(payload_bytes.as_slice());
    request.requester_signature_hex = Some(hex::encode(signature.to_bytes()));
    request
}

fn signed_fetch_blob_request_for_test(
    content_hash: &str,
    signer_seed: u8,
) -> super::replication::FetchBlobRequest {
    let (private_hex, public_hex) = deterministic_keypair_hex(signer_seed);
    let signing_key_bytes: [u8; 32] = hex::decode(private_hex)
        .expect("private key decode")
        .try_into()
        .expect("private key len");
    let signing_key = SigningKey::from_bytes(&signing_key_bytes);
    let mut request = super::replication::FetchBlobRequest {
        content_hash: content_hash.to_string(),
        requester_public_key_hex: Some(public_hex),
        requester_signature_hex: None,
    };
    let payload = FetchBlobRequestSigningPayload {
        version: 1,
        content_hash: request.content_hash.as_str(),
        requester_public_key_hex: request.requester_public_key_hex.as_deref(),
    };
    let payload_bytes = serde_json::to_vec(&payload).expect("encode fetch-blob signing payload");
    let signature = signing_key.sign(payload_bytes.as_slice());
    request.requester_signature_hex = Some(hex::encode(signature.to_bytes()));
    request
}

#[derive(Clone)]
struct RecordingExecutionHook {
    calls: Arc<Mutex<Vec<NodeExecutionCommitContext>>>,
}

impl RecordingExecutionHook {
    fn new(calls: Arc<Mutex<Vec<NodeExecutionCommitContext>>>) -> Self {
        Self { calls }
    }
}

impl NodeExecutionHook for RecordingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.calls
            .lock()
            .expect("lock execution calls")
            .push(context.clone());
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("exec-block-{:020}", context.height),
            execution_state_root: format!("exec-state-{:020}", context.height),
        })
    }
}

fn with_noop_execution_hook(runtime: NodeRuntime) -> NodeRuntime {
    let calls: Arc<Mutex<Vec<NodeExecutionCommitContext>>> = Arc::new(Mutex::new(Vec::new()));
    runtime.with_execution_hook(RecordingExecutionHook::new(calls))
}

fn wait_until(deadline: Instant, mut predicate: impl FnMut() -> bool) -> bool {
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

#[derive(Clone, Default)]
struct TestInMemoryNetwork {
    retained: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    subscribers: Arc<Mutex<Vec<TestNetworkInbox>>>,
    handlers: Arc<
        Mutex<HashMap<String, Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>>>,
    >,
}

type TestNetworkInbox = Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>;

impl TestInMemoryNetwork {
    fn clear_topic(&self, topic: &str) {
        self.retained
            .lock()
            .expect("lock retained")
            .insert(topic.to_string(), Vec::new());
        let subscribers = self.subscribers.lock().expect("lock subscribers");
        for inbox in subscribers.iter() {
            inbox
                .lock()
                .expect("lock subscriber inbox")
                .insert(topic.to_string(), Vec::new());
        }
    }
}

#[derive(Clone)]
struct ProviderAwareTestNetwork {
    inner: TestInMemoryNetwork,
    storage_root: PathBuf,
    required_provider_id: String,
    provider_attempts: Arc<Mutex<Vec<Vec<String>>>>,
}

impl ProviderAwareTestNetwork {
    fn new(storage_root: PathBuf, required_provider_id: impl Into<String>) -> Self {
        Self {
            inner: TestInMemoryNetwork::default(),
            storage_root,
            required_provider_id: required_provider_id.into(),
            provider_attempts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn provider_attempts(&self) -> Vec<Vec<String>> {
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .clone()
    }

    fn clear_topic(&self, topic: &str) {
        self.inner.clear_topic(topic);
    }
}

#[derive(Clone)]
struct ProviderFallbackTestNetwork {
    inner: TestInMemoryNetwork,
    storage_root: PathBuf,
    provider_attempts: Arc<Mutex<Vec<Vec<String>>>>,
    generic_attempts: Arc<Mutex<usize>>,
}

impl ProviderFallbackTestNetwork {
    fn new(storage_root: PathBuf) -> Self {
        Self {
            inner: TestInMemoryNetwork::default(),
            storage_root,
            provider_attempts: Arc::new(Mutex::new(Vec::new())),
            generic_attempts: Arc::new(Mutex::new(0)),
        }
    }

    fn provider_attempts(&self) -> Vec<Vec<String>> {
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .clone()
    }

    fn generic_attempts(&self) -> usize {
        *self
            .generic_attempts
            .lock()
            .expect("lock generic attempts")
    }

    fn clear_topic(&self, topic: &str) {
        self.inner.clear_topic(topic);
    }
}

#[derive(Clone)]
struct TestReplicaMaintenanceDht {
    source_provider_id: String,
    local_provider_id: String,
    providers_by_hash: Arc<Mutex<HashMap<String, Vec<ProviderRecord>>>>,
    provider_seed_count: Arc<Mutex<usize>>,
    published: Arc<Mutex<Vec<(String, String, String)>>>,
    heads: Arc<Mutex<HashMap<String, WorldHeadAnnounce>>>,
    memberships: Arc<Mutex<HashMap<String, MembershipDirectorySnapshot>>>,
}

impl TestReplicaMaintenanceDht {
    fn new(source_provider_id: impl Into<String>, local_provider_id: impl Into<String>) -> Self {
        Self {
            source_provider_id: source_provider_id.into(),
            local_provider_id: local_provider_id.into(),
            providers_by_hash: Arc::new(Mutex::new(HashMap::new())),
            provider_seed_count: Arc::new(Mutex::new(0)),
            published: Arc::new(Mutex::new(Vec::new())),
            heads: Arc::new(Mutex::new(HashMap::new())),
            memberships: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn published_records(&self) -> Vec<(String, String, String)> {
        self.published.lock().expect("lock published").clone()
    }

    fn seed_provider(&self, content_hash: &str, provider_id: &str) {
        self.providers_by_hash
            .lock()
            .expect("lock providers_by_hash")
            .insert(
                content_hash.to_string(),
                vec![ProviderRecord {
                    provider_id: provider_id.to_string(),
                    last_seen_ms: 1_000,
                    storage_total_bytes: None,
                    storage_available_bytes: None,
                    uptime_ratio_per_mille: None,
                    challenge_pass_ratio_per_mille: None,
                    load_ratio_per_mille: None,
                    p50_read_latency_ms: None,
                }],
            );
    }
}

impl proto_dht::DistributedDht<WorldError> for TestReplicaMaintenanceDht {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        provider_id: &str,
    ) -> Result<(), WorldError> {
        self.published.lock().expect("lock published").push((
            world_id.to_string(),
            content_hash.to_string(),
            provider_id.to_string(),
        ));
        Ok(())
    }

    fn get_providers(
        &self,
        _world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let mut providers_by_hash = self
            .providers_by_hash
            .lock()
            .expect("lock providers_by_hash");
        if let Some(cached) = providers_by_hash.get(content_hash) {
            return Ok(cached.clone());
        }

        let mut seed_count = self
            .provider_seed_count
            .lock()
            .expect("lock provider_seed_count");
        let providers = if *seed_count == 0 {
            vec![ProviderRecord {
                provider_id: self.source_provider_id.clone(),
                last_seen_ms: 1_000,
                storage_total_bytes: None,
                storage_available_bytes: None,
                uptime_ratio_per_mille: None,
                challenge_pass_ratio_per_mille: None,
                load_ratio_per_mille: None,
                p50_read_latency_ms: None,
            }]
        } else {
            vec![ProviderRecord {
                provider_id: self.local_provider_id.clone(),
                last_seen_ms: 1_100,
                storage_total_bytes: None,
                storage_available_bytes: None,
                uptime_ratio_per_mille: None,
                challenge_pass_ratio_per_mille: None,
                load_ratio_per_mille: None,
                p50_read_latency_ms: None,
            }]
        };
        *seed_count = seed_count.saturating_add(1);
        providers_by_hash.insert(content_hash.to_string(), providers.clone());
        Ok(providers)
    }

    fn put_world_head(&self, world_id: &str, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        self.heads
            .lock()
            .expect("lock heads")
            .insert(world_id.to_string(), head.clone());
        Ok(())
    }

    fn get_world_head(&self, world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        Ok(self
            .heads
            .lock()
            .expect("lock heads")
            .get(world_id)
            .cloned())
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        self.memberships
            .lock()
            .expect("lock memberships")
            .insert(world_id.to_string(), snapshot.clone());
        Ok(())
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        Ok(self
            .memberships
            .lock()
            .expect("lock memberships")
            .get(world_id)
            .cloned())
    }

    fn put_peer_record(&self, _world_id: &str, _record: &SignedPeerRecord) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_peer_record(&self, _world_id: &str, _peer_id: &str) -> Result<Option<SignedPeerRecord>, WorldError> {
        Ok(None)
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for TestInMemoryNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.retained
            .lock()
            .expect("lock retained")
            .entry(topic.to_string())
            .or_default()
            .push(payload.to_vec());
        let subscribers = self.subscribers.lock().expect("lock subscribers");
        for inbox in subscribers.iter() {
            let mut topic_inbox = inbox.lock().expect("lock subscriber inbox");
            topic_inbox
                .entry(topic.to_string())
                .or_default()
                .push(payload.to_vec());
        }
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        let inbox = Arc::new(Mutex::new(HashMap::<String, Vec<Vec<u8>>>::new()));
        let retained = self.retained.lock().expect("lock retained");
        let seeded = retained.get(topic).cloned().unwrap_or_default();
        drop(retained);
        {
            let mut topic_inbox = inbox.lock().expect("lock subscriber inbox");
            topic_inbox.insert(topic.to_string(), seeded);
        }
        self.subscribers
            .lock()
            .expect("lock subscribers")
            .push(Arc::clone(&inbox));
        Ok(NetworkSubscription::new(topic.to_string(), inbox))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let handlers = self.handlers.lock().expect("lock handlers");
        let Some(handler) = handlers.get(protocol) else {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        };
        handler(payload)
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.handlers
            .lock()
            .expect("lock handlers")
            .insert(protocol.to_string(), Arc::from(handler));
        Ok(())
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for ProviderAwareTestNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        self.request_with_providers(protocol, payload, &[])
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .push(providers.to_vec());
        if protocol != super::replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            return self.inner.request_with_providers(protocol, payload, providers);
        }
        if !providers
            .iter()
            .any(|provider_id| provider_id == &self.required_provider_id)
        {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!(
                    "provider selection missing required provider {}",
                    self.required_provider_id
                ),
            });
        }
        let request = serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("decode fetch blob request failed: {err}"),
            })?;
        let blob = super::replication::load_blob_from_root(
            self.storage_root.as_path(),
            request.content_hash.as_str(),
        )
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("load local blob failed: {err}"),
        })?;
        let response = super::replication::FetchBlobResponse {
            found: blob.is_some(),
            blob,
        };
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch blob response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for ProviderFallbackTestNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        self.request_with_providers(protocol, payload, &[])
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if protocol != super::replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            return self.inner.request_with_providers(protocol, payload, providers);
        }
        if !providers.is_empty() {
            self.provider_attempts
                .lock()
                .expect("lock provider attempts")
                .push(providers.to_vec());
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: "simulated provider route unavailable".to_string(),
            });
        }
        *self
            .generic_attempts
            .lock()
            .expect("lock generic attempts") += 1;
        let request = serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("decode fetch blob request failed: {err}"),
            })?;
        let blob = super::replication::load_blob_from_root(
            self.storage_root.as_path(),
            request.content_hash.as_str(),
        )
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("load local blob failed: {err}"),
        })?;
        let response = super::replication::FetchBlobResponse {
            found: blob.is_some(),
            blob,
        };
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch blob response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

#[test]
fn pos_engine_commits_single_validator_head() {
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let snapshot = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("tick");
    assert_eq!(snapshot.consensus_snapshot.mode, NodeConsensusMode::Pos);
    assert_eq!(snapshot.consensus_snapshot.latest_height, 1);
    assert_eq!(snapshot.consensus_snapshot.committed_height, 1);
    assert_eq!(
        snapshot.consensus_snapshot.last_status,
        Some(PosConsensusStatus::Committed)
    );
    assert_eq!(snapshot.consensus_snapshot.slot, 1);
}

#[test]
fn pos_engine_generates_chain_hashed_block_ids() {
    let config = NodeConfig::new("node-a", "world-hash", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let first = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("first tick");
    let second = engine
        .tick(
            &config.node_id,
            &config.world_id,
            2_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("second tick");

    let first_hash = first
        .consensus_snapshot
        .last_block_hash
        .as_deref()
        .expect("first hash should exist");
    let second_hash = second
        .consensus_snapshot
        .last_block_hash
        .as_deref()
        .expect("second hash should exist");
    assert_eq!(first_hash.len(), 64);
    assert_eq!(second_hash.len(), 64);
    assert!(first_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert!(second_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_ne!(first_hash, second_hash);
    assert!(!first_hash.contains(":h"));
}

#[test]
fn pos_engine_stays_pending_without_peer_votes_when_auto_attest_disabled() {
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("config")
        .with_pos_validators(multi_validators())
        .expect("validators")
        .with_auto_attest_all_validators(false);
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    for offset in 0..12 {
        let snapshot = engine
            .tick(
                &config.node_id,
                &config.world_id,
                2_000 + offset,
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
            )
            .expect("tick");
        assert_eq!(snapshot.consensus_snapshot.committed_height, 0);
    }
}

#[test]
fn pos_engine_apply_decision_rejects_height_overflow_without_state_mutation() {
    let config =
        NodeConfig::new("node-a", "world-overflow-apply", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 41;
    engine.network_committed_height = 43;
    engine.next_height = 44;
    engine.pending = Some(PendingProposal {
        height: 44,
        slot: 7,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "pending-block".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let decision = PosDecision {
        height: u64::MAX,
        slot: 8,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "overflow-block".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };

    let err = engine
        .apply_decision(&decision)
        .expect_err("height overflow must fail");
    assert!(
        matches!(err, NodeError::Consensus { reason } if reason.contains("decision.height overflow"))
    );
    assert_eq!(engine.committed_height, 41);
    assert_eq!(engine.network_committed_height, 43);
    assert_eq!(engine.next_height, 44);
    assert_eq!(
        engine
            .pending
            .as_ref()
            .map(|proposal| proposal.block_hash.as_str()),
        Some("pending-block")
    );
    assert!(engine.last_committed_block_hash.is_none());
}

#[test]
fn pos_engine_record_synced_replication_height_rejects_overflow_without_partial_state() {
    let config =
        NodeConfig::new("node-a", "world-overflow-sync", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 9;
    engine.next_height = 10;
    engine.pending = Some(PendingProposal {
        height: 10,
        slot: 1,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "pending-sync".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let err = engine
        .record_synced_replication_height(u64::MAX, "overflow-block".to_string(), 7_700)
        .expect_err("height overflow must fail");
    assert!(matches!(err, NodeError::Replication { reason } if reason.contains("height overflow")));
    assert_eq!(engine.committed_height, 9);
    assert_eq!(engine.next_height, 10);
    assert_eq!(
        engine
            .pending
            .as_ref()
            .map(|proposal| proposal.block_hash.as_str()),
        Some("pending-sync")
    );
    assert!(engine.last_committed_block_hash.is_none());
    assert!(engine.last_committed_at_ms.is_none());
}

#[test]
fn pos_engine_ingest_proposal_rejects_slot_overflow_without_partial_state() {
    let config =
        NodeConfig::new("node-a", "world-overflow-proposal", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.next_height = 5;
    engine.next_slot = 3;

    let message = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: config.node_id.clone(),
        player_id: config.player_id.clone(),
        proposer_id: config.node_id.clone(),
        height: 8,
        slot: u64::MAX,
        epoch: 0,
        block_hash: "proposal-overflow".to_string(),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_234,
        public_key_hex: None,
        signature_hex: None,
    };

    let err = engine
        .ingest_proposal_message(config.world_id.as_str(), &message, u64::MAX)
        .expect_err("slot overflow must fail");
    assert!(
        matches!(err, NodeError::Consensus { reason } if reason.contains("proposal.slot overflow"))
    );
    assert_eq!(engine.next_height, 5);
    assert_eq!(engine.next_slot, 3);
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_restore_state_snapshot_rejects_overflow_without_partial_state() {
    let config =
        NodeConfig::new("node-a", "world-overflow-restore", NodeRole::Observer).expect("config");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 9;
    engine.network_committed_height = 10;
    engine.next_height = 11;
    engine.next_slot = 3;
    engine.pending = Some(PendingProposal {
        height: 11,
        slot: 3,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        block_hash: "pending-restore".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        attestations: std::collections::BTreeMap::new(),
        approved_stake: 100,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    let snapshot = super::pos_state_store::PosNodeStateSnapshot {
        next_height: 0,
        next_slot: 77,
        last_observed_slot: 77,
        missed_slot_count: 0,
        last_observed_tick: 77,
        missed_tick_count: 0,
        committed_height: u64::MAX,
        network_committed_height: u64::MAX,
        last_broadcast_proposal_height: 0,
        last_broadcast_local_attestation_height: 0,
        last_broadcast_committed_height: 0,
        last_committed_block_hash: Some("unexpected".to_string()),
        last_execution_height: 0,
        last_execution_block_hash: None,
        last_execution_state_root: None,
    };

    let err = engine
        .restore_state_snapshot(snapshot)
        .expect_err("committed height overflow must fail");
    assert!(
        matches!(err, NodeError::Replication { reason } if reason.contains("committed_height"))
    );
    assert_eq!(engine.committed_height, 9);
    assert_eq!(engine.network_committed_height, 10);
    assert_eq!(engine.next_height, 11);
    assert_eq!(engine.next_slot, 3);
    assert_eq!(
        engine
            .pending
            .as_ref()
            .map(|proposal| proposal.block_hash.as_str()),
        Some("pending-restore")
    );
}

#[test]
fn sequencer_commit_requires_execution_hook() {
    let config = NodeConfig::new("sequencer-a", "world-a", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let mut runtime = NodeRuntime::new(config);
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(80));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.consensus.committed_height, 0);
    assert!(snapshot
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("execution hook is required"));
}

#[test]
fn runtime_start_and_stop_updates_snapshot() {
    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let mut runtime = NodeRuntime::new(config);
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(40));

    let running = runtime.snapshot();
    assert!(running.running);
    assert!(running.tick_count >= 2);
    assert!(running.last_tick_unix_ms.is_some());
    assert_eq!(running.consensus.mode, NodeConsensusMode::Pos);
    assert!(running.consensus.committed_height >= 1);
    assert_eq!(
        running.consensus.last_status,
        Some(PosConsensusStatus::Committed)
    );
    assert!(running.last_error.is_none());

    runtime.stop().expect("stop");
    let stopped = runtime.snapshot();
    assert!(!stopped.running);
    assert!(stopped.tick_count >= running.tick_count);
}

#[test]
fn config_rejects_non_positive_replica_maintenance_poll_interval() {
    let err = NodeConfig::new("node-maint", "world-maint", NodeRole::Observer)
        .expect("config")
        .with_replica_maintenance(NodeReplicaMaintenanceConfig {
            poll_interval_ms: 0,
            ..NodeReplicaMaintenanceConfig::default()
        })
        .expect_err("non-positive poll interval should be rejected");
    assert!(
        matches!(err, NodeError::InvalidConfig { reason } if reason.contains("replica_maintenance.poll_interval_ms"))
    );
}

#[test]
fn runtime_replica_maintenance_poll_executes_local_target_tasks() {
    let replication_root = temp_dir("runtime-replica-maintenance");
    let config = NodeConfig::new("node-a", "world-maint", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval")
        .with_require_execution_on_commit(false)
        .with_replication_root(replication_root.clone())
        .expect("replication root")
        .with_replica_maintenance(NodeReplicaMaintenanceConfig {
            max_content_hash_samples_per_round: 2,
            target_replicas_per_blob: 2,
            max_repairs_per_round: 2,
            max_rebalances_per_round: 0,
            poll_interval_ms: 10,
            ..NodeReplicaMaintenanceConfig::default()
        })
        .expect("replica maintenance");
    let network = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new("source-a", "node-a"));
    let mut runtime = NodeRuntime::new(config)
        .with_replication_network(NodeReplicationNetworkHandle::new(network))
        .with_replica_maintenance_dht(dht.clone());
    runtime.start().expect("start");
    let committed_ready = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 2
    });
    let published_ready = wait_until(Instant::now() + Duration::from_secs(2), || {
        !dht.published_records().is_empty()
    });
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(snapshot.last_error.is_none(), "{:?}", snapshot.last_error);
    assert!(
        committed_ready,
        "expected at least 2 committed heights before maintenance checks"
    );
    assert!(published_ready, "expected maintenance publish records");
    let published = dht.published_records();
    assert!(
        !published.is_empty(),
        "expected maintenance publish records"
    );
    assert!(published
        .iter()
        .any(|(_, _, provider_id)| provider_id == "node-a"));

    let _ = fs::remove_dir_all(replication_root);
}

#[test]
fn runtime_replica_maintenance_poll_skips_without_dht() {
    let replication_root = temp_dir("runtime-replica-maintenance-no-dht");
    let config = NodeConfig::new("node-a", "world-maint-no-dht", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval")
        .with_require_execution_on_commit(false)
        .with_replication_root(replication_root.clone())
        .expect("replication root")
        .with_replica_maintenance(NodeReplicaMaintenanceConfig {
            poll_interval_ms: 10,
            ..NodeReplicaMaintenanceConfig::default()
        })
        .expect("replica maintenance");
    let network = Arc::new(TestInMemoryNetwork::default());
    let mut runtime = NodeRuntime::new(config)
        .with_replication_network(NodeReplicationNetworkHandle::new(network));
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(120));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(snapshot.last_error.is_none(), "{:?}", snapshot.last_error);

    let _ = fs::remove_dir_all(replication_root);
}

#[test]
fn runtime_execution_hook_updates_consensus_snapshot() {
    let config = NodeConfig::new("node-exec", "world-exec", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let hook = RecordingExecutionHook::new(Arc::clone(&calls));
    let mut runtime = NodeRuntime::new(config).with_execution_hook(hook);
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(120));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(snapshot.consensus.committed_height >= 1);
    assert!(snapshot.consensus.last_execution_height >= 1);
    assert!(snapshot.consensus.last_execution_block_hash.is_some());
    assert!(snapshot.consensus.last_execution_state_root.is_some());

    let execution_calls = calls.lock().expect("lock calls");
    assert!(!execution_calls.is_empty());
    assert!(execution_calls
        .iter()
        .all(|call| call.world_id == "world-exec" && call.node_id == "node-exec"));
}

#[test]
fn pos_engine_signature_enforced_rejects_unsigned_proposal() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 203), ("node-b", 201)]);
    let config_b = NodeConfig::new("node-b", "world-sig-enforced", NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(temp_dir("sig-enforced"), 201));
    let mut engine = PosNodeEngine::new(&config_b).expect("engine");

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let unsigned_proposal = GossipProposalMessage {
        version: 1,
        world_id: config_b.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: format!("{}:h1:s0:p{}", config_b.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    endpoint_a
        .broadcast_proposal(&unsigned_proposal)
        .expect("broadcast unsigned proposal");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config_b.node_id, &config_b.world_id, None, 0)
        .expect("ingest");
    assert!(engine.pending.is_none());
}

#[test]
fn pos_engine_rejects_future_slot_proposal_from_peer_messages() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let config = NodeConfig::new(
        "node-b",
        "world-future-slot-peer-reject",
        NodeRole::Observer,
    )
    .expect("config")
    .with_pos_validators(vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ])
    .expect("validators");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let proposal = GossipProposalMessage {
        version: 1,
        world_id: config.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 9,
        epoch: 0,
        block_hash: format!("{}:h1:s9:p{}", config.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 1_000,
        public_key_hex: None,
        signature_hex: None,
    };
    endpoint_a
        .broadcast_proposal(&proposal)
        .expect("broadcast proposal");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config.node_id, &config.world_id, None, 2)
        .expect("ingest");
    assert!(engine.pending.is_none());
    assert_eq!(engine.inbound_rejected_proposal_future_slot, 1);
}

#[test]
fn pos_engine_signature_enforced_accepts_signed_proposal_and_attestation() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 203), ("node-b", 202)]);
    let config_b = NodeConfig::new("node-b", "world-sig-accept", NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(temp_dir("sig-accept"), 202));
    let mut engine = PosNodeEngine::new(&config_b).expect("engine");

    let (proposal_private_hex, proposal_public_hex) = deterministic_keypair_hex(203);
    let proposal_signing_key = SigningKey::from_bytes(
        &hex::decode(proposal_private_hex)
            .expect("proposal private decode")
            .try_into()
            .expect("proposal private len"),
    );
    let proposal_signer =
        ConsensusMessageSigner::new(proposal_signing_key, proposal_public_hex).expect("signer");

    let (attestation_private_hex, attestation_public_hex) = deterministic_keypair_hex(202);
    let attestation_signing_key = SigningKey::from_bytes(
        &hex::decode(attestation_private_hex)
            .expect("attestation private decode")
            .try_into()
            .expect("attestation private len"),
    );
    let attestation_signer =
        ConsensusMessageSigner::new(attestation_signing_key, attestation_public_hex)
            .expect("attestation signer");

    let endpoint_a =
        GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("endpoint a");
    let endpoint_b =
        GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint b");

    let mut proposal = GossipProposalMessage {
        version: 1,
        world_id: config_b.world_id.clone(),
        node_id: "node-a".to_string(),
        player_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height: 1,
        slot: 0,
        epoch: 0,
        block_hash: format!("{}:h1:s0:p{}", config_b.world_id, "node-a"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        proposed_at_ms: 2_000,
        public_key_hex: None,
        signature_hex: None,
    };
    sign_proposal_message(&mut proposal, &proposal_signer).expect("sign proposal");
    endpoint_a
        .broadcast_proposal(&proposal)
        .expect("broadcast signed proposal");
    thread::sleep(Duration::from_millis(20));

    let mut attestation = GossipAttestationMessage {
        version: 1,
        world_id: config_b.world_id.clone(),
        node_id: "node-b".to_string(),
        player_id: "node-b".to_string(),
        validator_id: "node-b".to_string(),
        height: proposal.height,
        slot: proposal.slot,
        epoch: proposal.epoch,
        block_hash: proposal.block_hash.clone(),
        approve: true,
        source_epoch: 0,
        target_epoch: 0,
        voted_at_ms: 2_001,
        reason: Some("signed attestation".to_string()),
        public_key_hex: None,
        signature_hex: None,
    };
    sign_attestation_message(&mut attestation, &attestation_signer).expect("sign attestation");
    endpoint_a
        .broadcast_attestation(&attestation)
        .expect("broadcast signed attestation");
    thread::sleep(Duration::from_millis(20));

    engine
        .ingest_peer_messages(&endpoint_b, &config_b.node_id, &config_b.world_id, None, 0)
        .expect("ingest");
    let pending = engine.pending.as_ref().expect("pending exists");
    assert_eq!(pending.height, 1);
    assert!(pending.attestations.contains_key("node-a"));
    assert!(pending.attestations.contains_key("node-b"));
}
