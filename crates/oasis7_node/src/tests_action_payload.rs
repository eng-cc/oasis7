use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::*;
use ed25519_dalek::{Signature, Signer, SigningKey};
use oasis7_distfs::{
    public_key_hex_from_signing_key_hex, sign_feedback_create_request, FeedbackCreateRequest,
};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};

#[path = "tests_action_payload_consensus_auth.rs"]
mod consensus_auth_tests;

const MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX: &str = "octransferauth:v1:";
const MAIN_TOKEN_CLAIM_AUTH_SIGNATURE_V1_PREFIX: &str = "occlaimauth:v1:";
const MAIN_TOKEN_GENESIS_AUTH_SIGNATURE_V1_PREFIX: &str = "ocgenesisauth:v1:";
const MAIN_TOKEN_TREASURY_AUTH_SIGNATURE_V1_PREFIX: &str = "octreasuryauth:v1:";
const MAIN_TOKEN_RESTRICTED_GRANT_ADMIN_REGISTRY_AUTH_SIGNATURE_V1_PREFIX: &str =
    "ocrestrictedgrantadminauth:v1:";
const DEFAULT_GENESIS_CONTROLLER_SLOT: &str = "msig.genesis.v1";
const DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT: &str = "msig.ecosystem_governance.v1";

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

fn token_auth_test_signer(seed: u8) -> (String, String) {
    let private_key_hex = format!("{seed:02x}").repeat(32);
    let public_key_hex =
        public_key_hex_from_signing_key_hex(private_key_hex.as_str()).expect("derive pubkey");
    (public_key_hex, private_key_hex)
}

#[derive(Serialize)]
struct TestConsensusActionPayloadEnvelope {
    version: u8,
    auth: Option<TestConsensusActionAuthEnvelope>,
    body: TestConsensusActionPayloadBody,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum TestConsensusActionAuthEnvelope {
    MainTokenAction(TestMainTokenActionAuthProof),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum TestMainTokenActionAuthScheme {
    Ed25519,
    ThresholdEd25519,
}

#[derive(Serialize)]
struct TestMainTokenActionParticipantSignature {
    public_key: String,
    signature: String,
}

#[derive(Serialize)]
struct TestMainTokenActionAuthProof {
    scheme: TestMainTokenActionAuthScheme,
    account_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold: Option<u16>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    participant_signatures: Vec<TestMainTokenActionParticipantSignature>,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum TestConsensusActionPayloadBody {
    RuntimeAction { action: JsonValue },
}

#[derive(Serialize)]
struct TestMainTokenActionSigningEnvelope<'a> {
    version: u8,
    operation: &'static str,
    account_id: &'a str,
    public_key: &'a str,
    action: TestMainTokenActionSigningPayload<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
enum TestMainTokenActionSigningPayload<'a> {
    TransferMainToken(TestTransferMainTokenSigningData<'a>),
    ClaimMainTokenVesting(TestClaimMainTokenVestingSigningData<'a>),
    InitializeMainTokenGenesis(TestInitializeMainTokenGenesisSigningData<'a>),
    DistributeMainTokenTreasury(TestDistributeMainTokenTreasurySigningData<'a>),
    UpdateRestrictedStarterClaimAdminRegistry(
        TestUpdateRestrictedStarterClaimAdminRegistrySigningData<'a>,
    ),
}

#[derive(Serialize)]
struct TestTransferMainTokenSigningData<'a> {
    from_account_id: &'a str,
    to_account_id: &'a str,
    amount: u64,
    nonce: u64,
}

#[derive(Serialize)]
struct TestClaimMainTokenVestingSigningData<'a> {
    bucket_id: &'a str,
    beneficiary: &'a str,
    nonce: u64,
}

#[derive(Serialize)]
struct TestInitializeMainTokenGenesisSigningData<'a> {
    allocations: &'a [JsonValue],
}

#[derive(Serialize)]
struct TestDistributeMainTokenTreasurySigningData<'a> {
    proposal_id: u64,
    distribution_id: &'a str,
    bucket_id: &'a str,
    distributions: &'a [JsonValue],
}

#[derive(Serialize)]
struct TestUpdateRestrictedStarterClaimAdminRegistrySigningData<'a> {
    controller_account_id: &'a str,
    next_admin_account_ids: &'a [JsonValue],
}

fn test_main_token_action_signature_prefix(action_kind: &str) -> &'static str {
    match action_kind {
        "TransferMainToken" => MAIN_TOKEN_TRANSFER_AUTH_SIGNATURE_V1_PREFIX,
        "ClaimMainTokenVesting" => MAIN_TOKEN_CLAIM_AUTH_SIGNATURE_V1_PREFIX,
        "InitializeMainTokenGenesis" => MAIN_TOKEN_GENESIS_AUTH_SIGNATURE_V1_PREFIX,
        "DistributeMainTokenTreasury" => MAIN_TOKEN_TREASURY_AUTH_SIGNATURE_V1_PREFIX,
        "UpdateRestrictedStarterClaimAdminRegistry" => {
            MAIN_TOKEN_RESTRICTED_GRANT_ADMIN_REGISTRY_AUTH_SIGNATURE_V1_PREFIX
        }
        other => panic!("unsupported test action kind {other}"),
    }
}

fn test_main_token_action_operation(action_kind: &str) -> &'static str {
    match action_kind {
        "TransferMainToken" => "transfer_main_token",
        "ClaimMainTokenVesting" => "claim_main_token_vesting",
        "InitializeMainTokenGenesis" => "initialize_main_token_genesis",
        "DistributeMainTokenTreasury" => "distribute_main_token_treasury",
        "UpdateRestrictedStarterClaimAdminRegistry" => {
            "update_restricted_starter_claim_admin_registry"
        }
        other => panic!("unsupported test action kind {other}"),
    }
}

fn main_token_account_id_from_public_key(public_key_hex: &str) -> String {
    format!("oc:pk:{}", public_key_hex.trim().to_ascii_lowercase())
}

fn test_main_token_signing_action(action: &JsonValue) -> TestMainTokenActionSigningPayload<'_> {
    let action_kind = action
        .get("type")
        .and_then(JsonValue::as_str)
        .expect("action kind");
    let data = action.get("data").expect("action data");
    match action_kind {
        "TransferMainToken" => {
            TestMainTokenActionSigningPayload::TransferMainToken(TestTransferMainTokenSigningData {
                from_account_id: data
                    .get("from_account_id")
                    .and_then(JsonValue::as_str)
                    .expect("transfer from_account_id"),
                to_account_id: data
                    .get("to_account_id")
                    .and_then(JsonValue::as_str)
                    .expect("transfer to_account_id"),
                amount: data
                    .get("amount")
                    .and_then(JsonValue::as_u64)
                    .expect("transfer amount"),
                nonce: data
                    .get("nonce")
                    .and_then(JsonValue::as_u64)
                    .expect("transfer nonce"),
            })
        }
        "ClaimMainTokenVesting" => TestMainTokenActionSigningPayload::ClaimMainTokenVesting(
            TestClaimMainTokenVestingSigningData {
                bucket_id: data
                    .get("bucket_id")
                    .and_then(JsonValue::as_str)
                    .expect("claim bucket_id"),
                beneficiary: data
                    .get("beneficiary")
                    .and_then(JsonValue::as_str)
                    .expect("claim beneficiary"),
                nonce: data
                    .get("nonce")
                    .and_then(JsonValue::as_u64)
                    .expect("claim nonce"),
            },
        ),
        "InitializeMainTokenGenesis" => {
            TestMainTokenActionSigningPayload::InitializeMainTokenGenesis(
                TestInitializeMainTokenGenesisSigningData {
                    allocations: data
                        .get("allocations")
                        .and_then(JsonValue::as_array)
                        .map(Vec::as_slice)
                        .expect("genesis allocations"),
                },
            )
        }
        "DistributeMainTokenTreasury" => {
            TestMainTokenActionSigningPayload::DistributeMainTokenTreasury(
                TestDistributeMainTokenTreasurySigningData {
                    proposal_id: data
                        .get("proposal_id")
                        .and_then(JsonValue::as_u64)
                        .expect("treasury proposal_id"),
                    distribution_id: data
                        .get("distribution_id")
                        .and_then(JsonValue::as_str)
                        .expect("treasury distribution_id"),
                    bucket_id: data
                        .get("bucket_id")
                        .and_then(JsonValue::as_str)
                        .expect("treasury bucket_id"),
                    distributions: data
                        .get("distributions")
                        .and_then(JsonValue::as_array)
                        .map(Vec::as_slice)
                        .expect("treasury distributions"),
                },
            )
        }
        "UpdateRestrictedStarterClaimAdminRegistry" => {
            TestMainTokenActionSigningPayload::UpdateRestrictedStarterClaimAdminRegistry(
                TestUpdateRestrictedStarterClaimAdminRegistrySigningData {
                    controller_account_id: data
                        .get("controller_account_id")
                        .and_then(JsonValue::as_str)
                        .expect("restricted claim admin registry controller_account_id"),
                    next_admin_account_ids: data
                        .get("next_admin_account_ids")
                        .and_then(JsonValue::as_array)
                        .map(Vec::as_slice)
                        .expect("restricted claim admin registry next_admin_account_ids"),
                },
            )
        }
        other => panic!("unsupported test action kind {other}"),
    }
}

fn encode_signed_main_token_runtime_payload(
    action: JsonValue,
    account_id: &str,
    seed: u8,
) -> Vec<u8> {
    let (public_key_hex, private_key_hex) = token_auth_test_signer(seed);
    let signing_key = SigningKey::from_bytes(
        &hex::decode(private_key_hex)
            .expect("decode private key")
            .try_into()
            .expect("32 bytes"),
    );
    let action_kind = action
        .get("type")
        .and_then(JsonValue::as_str)
        .expect("action kind");
    let signing_payload = serde_json::to_vec(&TestMainTokenActionSigningEnvelope {
        version: 1,
        operation: test_main_token_action_operation(action_kind),
        account_id,
        public_key: public_key_hex.as_str(),
        action: test_main_token_signing_action(&action),
    })
    .expect("encode signing payload");
    let signature: Signature = signing_key.sign(signing_payload.as_slice());
    let proof = TestMainTokenActionAuthProof {
        scheme: TestMainTokenActionAuthScheme::Ed25519,
        account_id: account_id.to_string(),
        public_key: Some(public_key_hex),
        signature: Some(format!(
            "{}{}",
            test_main_token_action_signature_prefix(action_kind),
            hex::encode(signature.to_bytes())
        )),
        threshold: None,
        participant_signatures: Vec::new(),
    };
    serde_cbor::to_vec(&TestConsensusActionPayloadEnvelope {
        version: 1,
        auth: Some(TestConsensusActionAuthEnvelope::MainTokenAction(proof)),
        body: TestConsensusActionPayloadBody::RuntimeAction { action },
    })
    .expect("encode signed payload")
}

fn encode_threshold_signed_main_token_runtime_payload(
    action: JsonValue,
    account_id: &str,
    threshold: u16,
    seeds: &[u8],
) -> Vec<u8> {
    let action_kind = action
        .get("type")
        .and_then(JsonValue::as_str)
        .expect("action kind");
    let mut participant_signatures = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let (public_key_hex, private_key_hex) = token_auth_test_signer(*seed);
        let signing_key = SigningKey::from_bytes(
            &hex::decode(private_key_hex)
                .expect("decode private key")
                .try_into()
                .expect("32 bytes"),
        );
        let signing_payload = serde_json::to_vec(&TestMainTokenActionSigningEnvelope {
            version: 1,
            operation: test_main_token_action_operation(action_kind),
            account_id,
            public_key: public_key_hex.as_str(),
            action: test_main_token_signing_action(&action),
        })
        .expect("encode threshold signing payload");
        let signature: Signature = signing_key.sign(signing_payload.as_slice());
        participant_signatures.push(TestMainTokenActionParticipantSignature {
            public_key: public_key_hex,
            signature: format!(
                "{}{}",
                test_main_token_action_signature_prefix(action_kind),
                hex::encode(signature.to_bytes())
            ),
        });
    }
    serde_cbor::to_vec(&TestConsensusActionPayloadEnvelope {
        version: 1,
        auth: Some(TestConsensusActionAuthEnvelope::MainTokenAction(
            TestMainTokenActionAuthProof {
                scheme: TestMainTokenActionAuthScheme::ThresholdEd25519,
                account_id: account_id.to_string(),
                public_key: None,
                signature: None,
                threshold: Some(threshold),
                participant_signatures,
            },
        )),
        body: TestConsensusActionPayloadBody::RuntimeAction { action },
    })
    .expect("encode threshold signed payload")
}

fn encode_unsigned_runtime_payload(action: JsonValue) -> Vec<u8> {
    serde_cbor::to_vec(&TestConsensusActionPayloadEnvelope {
        version: 1,
        auth: None,
        body: TestConsensusActionPayloadBody::RuntimeAction { action },
    })
    .expect("encode unsigned payload")
}

fn configured_controller_binding(
    genesis_threshold: u16,
    genesis_seeds: &[u8],
    ecosystem_threshold: u16,
    ecosystem_seeds: &[u8],
) -> NodeMainTokenControllerBindingConfig {
    let genesis_public_keys = genesis_seeds
        .iter()
        .map(|seed| token_auth_test_signer(*seed).0)
        .collect::<Vec<_>>();
    let ecosystem_public_keys = ecosystem_seeds
        .iter()
        .map(|seed| token_auth_test_signer(*seed).0)
        .collect::<Vec<_>>();
    NodeMainTokenControllerBindingConfig::default()
        .with_controller_signer_policy(
            DEFAULT_GENESIS_CONTROLLER_SLOT,
            genesis_threshold,
            genesis_public_keys,
        )
        .expect("genesis controller signer policy")
        .with_controller_signer_policy(
            DEFAULT_ECOSYSTEM_TREASURY_CONTROLLER_SLOT,
            ecosystem_threshold,
            ecosystem_public_keys,
        )
        .expect("ecosystem controller signer policy")
}

#[test]
fn runtime_execution_hook_receives_sorted_committed_actions() {
    let config = NodeConfig::new("node-action", "world-action", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let hook = RecordingExecutionHook::new(Arc::clone(&calls));
    let mut runtime = NodeRuntime::new(config).with_execution_hook(hook);

    let payload_b = serde_cbor::to_vec(&serde_json::json!({"kind": "b"})).expect("payload b");
    let payload_a = serde_cbor::to_vec(&serde_json::json!({"kind": "a"})).expect("payload a");
    runtime
        .submit_consensus_action_payload(2, payload_b)
        .expect("submit action b");
    runtime
        .submit_consensus_action_payload(1, payload_a)
        .expect("submit action a");

    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(120));
    runtime.stop().expect("stop");

    let execution_calls = calls.lock().expect("lock calls");
    let with_actions = execution_calls
        .iter()
        .find(|call| !call.committed_actions.is_empty())
        .expect("at least one commit should carry actions");
    let ordered_ids: Vec<u64> = with_actions
        .committed_actions
        .iter()
        .map(|action| action.action_id)
        .collect();
    assert_eq!(ordered_ids, vec![1, 2]);
    let computed_root =
        compute_consensus_action_root(with_actions.committed_actions.as_slice()).expect("root");
    assert_eq!(computed_root, with_actions.action_root);
}

#[test]
fn runtime_dequeues_actions_with_engine_capacity_limit() {
    let config = NodeConfig::new("node-cap", "world-cap", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval")
        .with_max_pending_consensus_actions(16)
        .expect("runtime pending limit")
        .with_max_engine_pending_consensus_actions(1)
        .expect("engine pending limit");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let hook = RecordingExecutionHook::new(Arc::clone(&calls));
    let mut runtime = NodeRuntime::new(config).with_execution_hook(hook);

    for action_id in 1..=3 {
        runtime
            .submit_consensus_action_payload(action_id, vec![action_id as u8])
            .expect("submit action");
    }

    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(240));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(
        snapshot.last_error.is_none(),
        "engine should not saturate while runtime dequeues incrementally: {:?}",
        snapshot.last_error
    );

    let calls = calls.lock().expect("lock calls");
    let committed_ids = calls
        .iter()
        .flat_map(|call| call.committed_actions.iter().map(|action| action.action_id))
        .collect::<Vec<_>>();
    assert!(
        committed_ids.windows(2).all(|pair| pair[0] <= pair[1]),
        "committed action ids should remain monotonic: {committed_ids:?}"
    );
    assert!(
        committed_ids.iter().copied().eq([1, 2, 3]),
        "all queued actions should be eventually committed exactly once: {committed_ids:?}"
    );
}

#[test]
fn pos_engine_rejects_tick_when_engine_pending_limit_exceeded() {
    let config = NodeConfig::new(
        "node-engine-limit",
        "world-engine-limit",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_engine_pending_consensus_actions(1)
    .expect("engine pending limit");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let action_1 = NodeConsensusAction::from_payload(1, config.player_id.clone(), vec![1_u8])
        .expect("action 1");
    let action_2 = NodeConsensusAction::from_payload(2, config.player_id.clone(), vec![2_u8])
        .expect("action 2");
    let err = engine
        .tick(
            &config.node_id,
            &config.world_id,
            1_000,
            None,
            None,
            None,
            None,
            vec![action_1, action_2],
            None,
        )
        .expect_err("engine should reject merged queue over capacity");
    assert!(matches!(err, NodeError::Consensus { .. }));
    assert!(
        err.to_string().contains("engine buffer saturated"),
        "unexpected error: {err}"
    );
}

#[test]
fn pos_engine_pending_capacity_reserves_rejected_proposal_actions() {
    let config = NodeConfig::new(
        "node-capacity-reserve",
        "world-capacity-reserve",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_engine_pending_consensus_actions(4)
    .expect("engine pending limit");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let queued = NodeConsensusAction::from_payload(1, config.player_id.clone(), vec![1_u8])
        .expect("queued action");
    let reserved_a = NodeConsensusAction::from_payload(2, config.player_id.clone(), vec![2_u8])
        .expect("reserved action a");
    let reserved_b = NodeConsensusAction::from_payload(3, config.player_id.clone(), vec![3_u8])
        .expect("reserved action b");
    let action_root = compute_consensus_action_root(&[reserved_a.clone(), reserved_b.clone()])
        .expect("action root");

    engine
        .pending_consensus_actions
        .insert(queued.action_id, queued);
    engine.pending = Some(PendingProposal {
        height: 1,
        slot: 0,
        epoch: 0,
        opened_at_ms: 1,
        proposer_id: config.node_id.clone(),
        block_hash: "pending-block".to_string(),
        action_root,
        committed_actions: vec![reserved_a, reserved_b],
        attestations: BTreeMap::new(),
        approved_stake: 0,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
    });

    assert_eq!(
        engine.pending_consensus_action_capacity(),
        1,
        "capacity should reserve space for requeueing actions from the pending proposal"
    );
}

#[test]
fn pos_engine_apply_rejected_decision_surfaces_requeue_overflow_instead_of_dropping() {
    let config = NodeConfig::new(
        "node-requeue-overflow",
        "world-requeue-overflow",
        NodeRole::Observer,
    )
    .expect("config")
    .with_max_engine_pending_consensus_actions(2)
    .expect("engine pending limit");
    let mut engine = PosNodeEngine::new(&config).expect("engine");

    let queued = NodeConsensusAction::from_payload(1, config.player_id.clone(), vec![1_u8])
        .expect("queued action");
    engine
        .pending_consensus_actions
        .insert(queued.action_id, queued);

    let rejected_a = NodeConsensusAction::from_payload(2, config.player_id.clone(), vec![2_u8])
        .expect("rejected action a");
    let rejected_b = NodeConsensusAction::from_payload(3, config.player_id.clone(), vec![3_u8])
        .expect("rejected action b");
    let action_root = compute_consensus_action_root(&[rejected_a.clone(), rejected_b.clone()])
        .expect("action root");
    let decision = PosDecision {
        height: 7,
        slot: 6,
        epoch: 0,
        status: PosConsensusStatus::Rejected,
        block_hash: "rejected-block".to_string(),
        action_root,
        committed_actions: vec![rejected_a, rejected_b],
        approved_stake: 0,
        rejected_stake: 100,
        required_stake: 67,
        total_stake: 100,
    };

    let err = engine
        .apply_decision(&decision)
        .expect_err("requeue overflow must return an explicit error");
    let reason = err.to_string();
    assert!(
        reason.contains("requeue rejected consensus actions failed"),
        "error should describe requeue failure context: {reason}"
    );
    assert!(
        reason.contains("engine buffer saturated"),
        "error should preserve the saturation reason: {reason}"
    );
    assert_eq!(
        engine.pending_consensus_actions.len(),
        1,
        "existing queued actions should remain intact when requeue fails"
    );
    assert_eq!(
        engine.next_height, 1,
        "engine height should not advance when rejected actions cannot be requeued"
    );
}

#[test]
fn submit_consensus_action_payload_rejects_zero_action_id() {
    let runtime = NodeRuntime::new(
        NodeConfig::new("node-action-id", "world-action-id", NodeRole::Observer).expect("config"),
    );
    let err = runtime
        .submit_consensus_action_payload(0, vec![0_u8])
        .expect_err("zero action id must fail");
    assert!(matches!(err, NodeError::Consensus { .. }));
}

#[test]
fn submit_consensus_action_payload_as_player_rejects_player_mismatch() {
    let runtime = NodeRuntime::new(
        NodeConfig::new("node-action-id", "world-action-id", NodeRole::Observer).expect("config"),
    );
    let err = runtime
        .submit_consensus_action_payload_as_player("other-player", 1, vec![1_u8, 2, 3])
        .expect_err("mismatched player must fail");
    assert!(matches!(err, NodeError::Consensus { .. }));
}

#[test]
fn submit_consensus_action_payload_rejects_payload_over_limit() {
    let config = NodeConfig::new("node-limit", "world-limit", NodeRole::Observer)
        .expect("config")
        .with_max_consensus_action_payload_bytes(4)
        .expect("payload limit");
    let runtime = NodeRuntime::new(config);
    let err = runtime
        .submit_consensus_action_payload(1, vec![1_u8, 2, 3, 4, 5])
        .expect_err("oversized payload must fail");
    assert!(matches!(err, NodeError::Consensus { .. }));
    assert!(
        err.to_string().contains("payload too large"),
        "unexpected error: {err}"
    );
}

#[test]
fn submit_consensus_action_payload_rejects_queue_saturation() {
    let config = NodeConfig::new("node-queue", "world-queue", NodeRole::Observer)
        .expect("config")
        .with_max_pending_consensus_actions(1)
        .expect("queue limit");
    let runtime = NodeRuntime::new(config);
    let payload = encode_unsigned_runtime_payload(json!({
        "type": "RegisterAgent",
        "data": {
            "agent_id": "queue-agent",
            "pos": {
                "x": 0.0,
                "y": 0.0,
                "z": 0.0
            }
        }
    }));
    runtime
        .submit_consensus_action_payload(1, payload.clone())
        .expect("first action should be accepted");
    let err = runtime
        .submit_consensus_action_payload(2, payload)
        .expect_err("second action must fail after queue reaches limit");
    assert!(matches!(err, NodeError::Consensus { .. }));
    assert!(
        err.to_string().contains("queue saturated"),
        "unexpected error: {err}"
    );
}

#[test]
fn submit_feedback_rejects_when_feedback_p2p_not_configured() {
    let runtime = NodeRuntime::new(
        NodeConfig::new(
            "node-feedback-off",
            "world-feedback-off",
            NodeRole::Observer,
        )
        .expect("config"),
    );
    let signing_key_hex =
        "1212121212121212121212121212121212121212121212121212121212121212".to_string();
    let author_public_key_hex =
        public_key_hex_from_signing_key_hex(signing_key_hex.as_str()).expect("derive pubkey");
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_millis() as i64;
    let mut request = FeedbackCreateRequest {
        feedback_id: "fb-node-runtime-off".to_string(),
        author_public_key_hex,
        submit_ip: "127.0.0.1".to_string(),
        category: "bug".to_string(),
        platform: "web".to_string(),
        game_version: "0.1.0".to_string(),
        content: "feedback disabled".to_string(),
        attachments: vec![],
        nonce: "nonce-feedback-off".to_string(),
        timestamp_ms: now_ms,
        expires_at_ms: now_ms + 120_000,
        signature_hex: String::new(),
    };
    request.signature_hex =
        sign_feedback_create_request(&request, signing_key_hex.as_str()).expect("sign request");

    let err = runtime
        .submit_feedback(request)
        .expect_err("feedback submit should fail when feedback_p2p is not configured");
    assert!(matches!(err, NodeError::Replication { .. }));
    assert!(
        err.to_string().contains("feedback_p2p is not configured"),
        "unexpected error: {err}"
    );
}

#[test]
fn role_parse_roundtrip() {
    for role in [NodeRole::Sequencer, NodeRole::Storage, NodeRole::Observer] {
        let parsed = NodeRole::from_str(role.as_str()).expect("parse role");
        assert_eq!(parsed, role);
    }
}

#[test]
fn config_rejects_invalid_pos_config() {
    let result = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("base config")
        .with_pos_config(NodePosConfig::ethereum_like(vec![]));
    assert!(matches!(result, Err(NodeError::InvalidConfig { .. })));
}

#[test]
fn config_rejects_zero_slot_duration() {
    let mut pos_config = NodePosConfig::ethereum_like(vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }]);
    pos_config.slot_duration_ms = 0;
    let result = NodeConfig::new("node-a", "world-slot-config", NodeRole::Observer)
        .expect("base config")
        .with_pos_config(pos_config);
    assert!(
        matches!(result, Err(NodeError::InvalidConfig { reason }) if reason.contains("slot_duration_ms"))
    );
}

#[test]
fn config_rejects_zero_ticks_per_slot() {
    let mut pos_config = NodePosConfig::ethereum_like(vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }]);
    pos_config.ticks_per_slot = 0;
    let result = NodeConfig::new("node-a", "world-tick-config", NodeRole::Observer)
        .expect("base config")
        .with_pos_config(pos_config);
    assert!(
        matches!(result, Err(NodeError::InvalidConfig { reason }) if reason.contains("ticks_per_slot"))
    );
}

#[test]
fn config_rejects_out_of_range_proposal_tick_phase() {
    let mut pos_config = NodePosConfig::ethereum_like(vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }]);
    pos_config.ticks_per_slot = 10;
    pos_config.proposal_tick_phase = 10;
    let result = NodeConfig::new("node-a", "world-tick-phase", NodeRole::Observer)
        .expect("base config")
        .with_pos_config(pos_config);
    assert!(
        matches!(result, Err(NodeError::InvalidConfig { reason }) if reason.contains("proposal_tick_phase"))
    );
}

#[test]
fn feedback_p2p_config_rejects_zero_limits() {
    let err = NodeFeedbackP2pConfig::default()
        .with_max_incoming_announces_per_tick(0)
        .expect_err("incoming announce limit must reject zero");
    assert!(matches!(err, NodeError::InvalidConfig { .. }));

    let err = NodeFeedbackP2pConfig::default()
        .with_max_outgoing_announces_per_tick(0)
        .expect_err("outgoing announce limit must reject zero");
    assert!(matches!(err, NodeError::InvalidConfig { .. }));
}

#[test]
fn runtime_start_rejects_feedback_p2p_without_replication_config() {
    let config = NodeConfig::new("node-feedback", "world-feedback", NodeRole::Observer)
        .expect("config")
        .with_feedback_p2p(NodeFeedbackP2pConfig::default())
        .expect("feedback p2p config");
    let mut runtime = NodeRuntime::new(config);
    let err = runtime
        .start()
        .expect_err("feedback p2p without replication should fail");
    assert!(matches!(err, NodeError::InvalidConfig { .. }));
    assert!(
        err.to_string()
            .contains("feedback_p2p requires replication config"),
        "unexpected error: {err}"
    );
}

#[test]
fn config_accepts_extreme_supermajority_ratio_just_above_half() {
    let denominator = u64::MAX;
    let numerator = denominator / 2 + 1;
    let mut pos_config = NodePosConfig::ethereum_like(vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ]);
    pos_config.supermajority_numerator = numerator;
    pos_config.supermajority_denominator = denominator;

    let config = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("base config")
        .with_pos_config(pos_config)
        .expect("extreme ratio should be valid");
    assert_eq!(config.pos_config.supermajority_numerator, numerator);
    assert_eq!(config.pos_config.supermajority_denominator, denominator);
}

#[test]
fn config_rejects_duplicate_validator_player_bindings() {
    let mut validator_player_ids = BTreeMap::new();
    validator_player_ids.insert("node-a".to_string(), "player-1".to_string());
    validator_player_ids.insert("node-b".to_string(), "player-1".to_string());
    let result = NodePosConfig::ethereum_like(vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ])
    .with_validator_player_ids(validator_player_ids);
    assert!(matches!(result, Err(NodeError::InvalidConfig { .. })));
}

#[test]
fn runtime_drains_committed_action_batches_for_viewer_consumers() {
    let config = NodeConfig::new("node-drain", "world-drain", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let hook = RecordingExecutionHook::new(Arc::clone(&calls));
    let mut runtime = NodeRuntime::new(config).with_execution_hook(hook);

    let payload_b = serde_cbor::to_vec(&serde_json::json!({"kind": "b"})).expect("payload b");
    let payload_a = serde_cbor::to_vec(&serde_json::json!({"kind": "a"})).expect("payload a");
    runtime
        .submit_consensus_action_payload(2, payload_b)
        .expect("submit action b");
    runtime
        .submit_consensus_action_payload(1, payload_a)
        .expect("submit action a");

    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(120));
    runtime.stop().expect("stop");

    let batches = runtime.drain_committed_action_batches();
    assert!(!batches.is_empty());
    let with_actions = batches
        .iter()
        .find(|batch| !batch.actions.is_empty())
        .expect("at least one committed batch should carry actions");
    let ordered_ids: Vec<u64> = with_actions
        .actions
        .iter()
        .map(|action| action.action_id)
        .collect();
    assert_eq!(ordered_ids, vec![1, 2]);
}

#[test]
fn runtime_committed_batches_respect_hot_window_limit() {
    let config = NodeConfig::new(
        "node-batch-window",
        "world-batch-window",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick interval")
    .with_max_engine_pending_consensus_actions(1)
    .expect("engine pending limit")
    .with_max_committed_action_batches(2)
    .expect("batch window");
    let mut runtime = NodeRuntime::new(config).with_execution_hook(RecordingExecutionHook::new(
        Arc::new(Mutex::new(Vec::new())),
    ));
    for action_id in 1..=5 {
        runtime
            .submit_consensus_action_payload(action_id, vec![action_id as u8])
            .expect("submit action");
    }

    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(260));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(
        snapshot.consensus.committed_height >= 5,
        "expected >=5 committed heights, got {}",
        snapshot.consensus.committed_height
    );

    let batches = runtime.drain_committed_action_batches();
    assert_eq!(batches.len(), 2, "committed batch window must be capped");
    let retained_action_ids = batches
        .iter()
        .flat_map(|batch| batch.actions.iter().map(|action| action.action_id))
        .collect::<Vec<_>>();
    assert_eq!(retained_action_ids, vec![4, 5]);
}
