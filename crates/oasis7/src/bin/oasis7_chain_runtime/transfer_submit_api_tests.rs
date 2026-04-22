use super::super::explorer_p0_api::{
    ExplorerBlocksResponse, ExplorerSearchResponse, ExplorerTxResponse, ExplorerTxsResponse,
};
use super::explorer_p1_api::{
    ExplorerAddressResponse, ExplorerAssetsResponse, ExplorerContractResponse,
    ExplorerContractsResponse, ExplorerMempoolResponse,
};
use super::{
    build_transfer_submit_action_payload, maybe_handle_transfer_submit_request,
    parse_transfer_submit_request, verify_transfer_submit_request_auth,
    ChainExplorerOverviewResponse, ChainTransferAccountsResponse, ChainTransferHistoryResponse,
    ChainTransferStatusResponse, ChainTransferSubmitRequest, ChainTransferSubmitResponse,
    TransferLifecycleStatus,
};
use crate::transfer_submit_api::preflight_validate_transfer_request;
use ed25519_dalek::SigningKey;
use oasis7::consensus_action_payload::{
    decode_consensus_action_payload, sign_main_token_runtime_action_auth,
    ConsensusActionPayloadBody,
};
use oasis7::runtime::{
    main_token_account_id_from_node_public_key, Action, EconomicContractState,
    EconomicContractStatus, MainTokenAccountBalance, MainTokenConfig, MainTokenSupplyState, World,
    WorldState,
};
use oasis7::simulator::ResourceKind;
use oasis7_node::{
    NodeConfig, NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook, NodeRole,
    NodeRuntime,
};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs};

#[path = "transfer_submit_api_explorer_tests.rs"]
mod explorer_tests;

fn tcp_stream_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let bind = listener.local_addr().expect("read local addr");
    let client = TcpStream::connect(bind).expect("connect loopback client");
    let (server, _) = listener.accept().expect("accept loopback connection");
    (server, client)
}

#[derive(Debug)]
struct NoopExecutionHook;

impl NodeExecutionHook for NoopExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("noop-block-{}", context.height),
            execution_state_root: format!("noop-root-{}", context.height),
        })
    }
}

fn decode_http_json_response<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> (u16, T) {
    let boundary = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response must include HTTP body separator");
    let header = std::str::from_utf8(&bytes[..boundary]).expect("response header utf-8");
    let status = header
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
        .expect("response status code");
    let payload =
        serde_json::from_slice::<T>(&bytes[(boundary + 4)..]).expect("response json payload");
    (status, payload)
}

fn make_temp_dir(label: &str) -> std::path::PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_transfer_submit_api_tests_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn reset_transfer_state_for_tests() {
    super::with_transfer_tracker(|tracker| {
        tracker.by_action_id.clear();
        tracker.action_order.clear();
    });
    super::NEXT_TRANSFER_ACTION_ID.store(1, Ordering::Relaxed);
    super::super::explorer_p0_api::reset_store_for_tests();
}

fn lock_transfer_test_state() -> std::sync::MutexGuard<'static, ()> {
    static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_GUARD.get_or_init(|| Mutex::new(()));
    let guard = guard
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    reset_transfer_state_for_tests();
    guard
}

fn transfer_test_signer(seed: u8) -> (String, String) {
    let private_key = [seed; 32];
    let signing_key = SigningKey::from_bytes(&private_key);
    (
        hex::encode(signing_key.verifying_key().to_bytes()),
        hex::encode(private_key),
    )
}

fn build_signed_transfer_request_with_accounts(
    from_account_id: String,
    to_account_id: String,
    amount: u64,
    nonce: u64,
    public_key: String,
    private_key: String,
) -> ChainTransferSubmitRequest {
    let mut request = ChainTransferSubmitRequest {
        from_account_id,
        to_account_id,
        amount,
        nonce,
        public_key,
        signature: String::new(),
    };
    let action = Action::TransferMainToken {
        from_account_id: request.from_account_id.clone(),
        to_account_id: request.to_account_id.clone(),
        amount: request.amount,
        nonce: request.nonce,
    };
    request.signature = sign_main_token_runtime_action_auth(
        &action,
        request.from_account_id.as_str(),
        request.public_key.as_str(),
        private_key.as_str(),
    )
    .expect("sign transfer request")
    .signature
    .expect("single signer transfer proof signature");
    request
}

fn build_signed_transfer_request(
    seed: u8,
    to_seed: u8,
    amount: u64,
    nonce: u64,
) -> ChainTransferSubmitRequest {
    let (public_key, private_key) = transfer_test_signer(seed);
    let (to_public_key, _) = transfer_test_signer(to_seed);
    build_signed_transfer_request_with_accounts(
        main_token_account_id_from_node_public_key(public_key.as_str()),
        main_token_account_id_from_node_public_key(to_public_key.as_str()),
        amount,
        nonce,
        public_key,
        private_key,
    )
}

fn serialize_transfer_request(request: &ChainTransferSubmitRequest) -> Vec<u8> {
    serde_json::to_vec(request).expect("serialize transfer request")
}

fn seed_world_for_explorer_p1(temp_dir: &Path) {
    let mut state = WorldState::default();
    state.main_token_config = MainTokenConfig {
        symbol: "OC".to_string(),
        decimals: 9,
        ..MainTokenConfig::default()
    };
    state.main_token_supply = MainTokenSupplyState {
        total_supply: 1_000_000,
        circulating_supply: 500_000,
        total_issued: 600_000,
        total_burned: 100_000,
    };
    state.main_token_balances.insert(
        "player:alice".to_string(),
        MainTokenAccountBalance {
            account_id: "player:alice".to_string(),
            liquid_balance: 1200,
            vested_balance: 300,
            restricted_starter_claim_balance: 125,
        },
    );
    state
        .main_token_transfer_nonces
        .insert("player:alice".to_string(), 7);
    state.economic_contracts.insert(
        "contract:alpha".to_string(),
        EconomicContractState {
            contract_id: "contract:alpha".to_string(),
            creator_agent_id: "player:alice".to_string(),
            counterparty_agent_id: "player:bob".to_string(),
            settlement_kind: ResourceKind::Electricity,
            settlement_amount: 88,
            reputation_stake: 5,
            expires_at: 1234,
            description: "power swap".to_string(),
            status: EconomicContractStatus::Accepted,
            accepted_at: Some(1000),
            settled_at: None,
            settlement_success: None,
            transfer_amount: 0,
            tax_amount: 0,
            settlement_notes: None,
        },
    );

    let world = World::new_with_state(state);
    world
        .save_to_dir(temp_dir)
        .expect("save seeded world for explorer p1 tests");
}

#[test]
fn parse_transfer_submit_request_rejects_same_account() {
    let _guard = lock_transfer_test_state();
    let (public_key, private_key) = transfer_test_signer(7);
    let account_id = main_token_account_id_from_node_public_key(public_key.as_str());
    let request = build_signed_transfer_request_with_accounts(
        account_id.clone(),
        account_id,
        7,
        1,
        public_key,
        private_key,
    );
    let body = serialize_transfer_request(&request);
    let err = parse_transfer_submit_request(body.as_slice())
        .expect_err("same source and target must fail");
    assert!(err.contains("cannot be the same"));
}

#[test]
fn build_transfer_submit_action_payload_encodes_runtime_action() {
    let _guard = lock_transfer_test_state();
    let request = build_signed_transfer_request(7, 8, 7, 2);
    let body = serialize_transfer_request(&request);
    let request = parse_transfer_submit_request(body.as_slice()).expect("request should parse");
    let payload = build_transfer_submit_action_payload(&request).expect("payload");
    let decoded = decode_consensus_action_payload(payload.as_slice()).expect("decode payload");
    match decoded {
        ConsensusActionPayloadBody::RuntimeAction { action } => match action {
            Action::TransferMainToken {
                from_account_id,
                to_account_id,
                amount,
                nonce,
            } => {
                let expected = build_signed_transfer_request(7, 8, 7, 2);
                assert_eq!(from_account_id, expected.from_account_id);
                assert_eq!(to_account_id, expected.to_account_id);
                assert_eq!(amount, 7);
                assert_eq!(nonce, 2);
            }
            other => panic!("expected TransferMainToken action, got {other:?}"),
        },
        other => panic!("expected runtime action payload, got {other:?}"),
    }
}

#[test]
fn verify_transfer_submit_request_auth_accepts_live_browser_captured_signature() {
    let _guard = lock_transfer_test_state();
    let request = ChainTransferSubmitRequest {
        from_account_id:
            "oc:pk:fded5085f1e8099257b7bfb2346eb6bd4194c3351d8f97686b18cfcc5969e0a3"
                .to_string(),
        to_account_id: "oc:pk:1111111111111111111111111111111111111111111111111111111111111111"
            .to_string(),
        amount: 1,
        nonce: 1,
        public_key: "fded5085f1e8099257b7bfb2346eb6bd4194c3351d8f97686b18cfcc5969e0a3"
            .to_string(),
        signature:
            "octransferauth:v1:9200ab505c80b5d27fb1a4e79624a083f2047669404d8c7d562e7d6b7da9154fcf7b85e96ab4c02bd5f2910845e4a7ce791d87398e0e3b9004bb022749d6830b"
                .to_string(),
    };
    let body = serialize_transfer_request(&request);
    let parsed = parse_transfer_submit_request(body.as_slice()).expect("request should parse");
    let action = Action::TransferMainToken {
        from_account_id: parsed.from_account_id.clone(),
        to_account_id: parsed.to_account_id.clone(),
        amount: parsed.amount,
        nonce: parsed.nonce,
    };
    let expected_signature = sign_main_token_runtime_action_auth(
        &action,
        parsed.from_account_id.as_str(),
        parsed.public_key.as_str(),
        "c7a149783d4d97d4b36f6f97ae43eb71af7fe595b7f717d329c96be3e58fdc29",
    )
    .expect("runtime helper should sign")
    .signature
    .expect("runtime helper signature");
    assert_eq!(
        parsed.signature, expected_signature,
        "runtime helper signature drift"
    );
    verify_transfer_submit_request_auth(&parsed).expect("browser-captured signature should verify");
}

#[test]
fn transfer_submit_handler_returns_invalid_request_for_bad_payload() {
    let _guard = lock_transfer_test_state();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-transfer-submit-bad",
            "world-transfer-submit-bad",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));

    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let (public_key, private_key) = transfer_test_signer(9);
    let account_id = main_token_account_id_from_node_public_key(public_key.as_str());
    let request = build_signed_transfer_request_with_accounts(
        account_id.clone(),
        account_id,
        7,
        2,
        public_key,
        private_key,
    );
    let body = serde_json::to_string(&request).expect("serialize request");
    let request = format!(
        "POST /v1/chain/transfer/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let handled = maybe_handle_transfer_submit_request(
        &mut server_stream,
        request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/transfer/submit",
        "node-transfer-submit-bad",
        "world-transfer-submit-bad",
        Path::new("."),
    )
    .expect("handler should process request");
    assert!(handled);
    drop(server_stream);

    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    let (status, response): (u16, ChainTransferSubmitResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 400);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("invalid_request"));
}

#[test]
fn preflight_transfer_rejects_restricted_only_balance() {
    let _guard = lock_transfer_test_state();
    let temp_dir = env::temp_dir().join(format!(
        "oasis7-transfer-preflight-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let mut world = World::new();
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 125,
        circulating_supply: 125,
        ..MainTokenSupplyState::default()
    });
    world
        .set_main_token_account_balance_with_restricted("player:starter", 0, 0, 125)
        .expect("seed restricted-only account");
    world
        .save_to_dir(temp_dir.as_path())
        .expect("save execution world");

    let request = ChainTransferSubmitRequest {
        from_account_id: "player:starter".to_string(),
        to_account_id: "player:receiver".to_string(),
        amount: 1,
        nonce: 1,
        public_key: "test-public-key".to_string(),
        signature: "test-signature".to_string(),
    };

    let err = preflight_validate_transfer_request(temp_dir.as_path(), &request)
        .expect_err("restricted-only transfer should fail");
    assert_eq!(err.0, "insufficient_balance");
    assert!(err.1.contains("transferable_balance=0"));
    assert!(err.1.contains("restricted_starter_claim_balance=125"));
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn transfer_accounts_endpoint_exposes_restricted_balance_separately() {
    let _guard = lock_transfer_test_state();
    let temp_dir = make_temp_dir("transfer_accounts_restricted");
    seed_world_for_explorer_p1(temp_dir.as_path());
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-transfer-accounts-restricted",
            "world-transfer-accounts-restricted",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));

    let (mut server, mut client) = tcp_stream_pair();
    let request = "GET /v1/chain/transfer/accounts HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n";
    let handled = maybe_handle_transfer_submit_request(
        &mut server,
        request.as_bytes(),
        &runtime,
        "GET",
        "/v1/chain/transfer/accounts",
        "node-transfer-accounts-restricted",
        "world-transfer-accounts-restricted",
        temp_dir.as_path(),
    )
    .expect("accounts request should be handled");
    assert!(handled);
    drop(server);

    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    let mut response_bytes = Vec::new();
    client
        .read_to_end(&mut response_bytes)
        .expect("read accounts response");
    let (status, response): (u16, ChainTransferAccountsResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 200);
    assert!(response.ok);
    let alice = response
        .accounts
        .iter()
        .find(|account| account.account_id == "player:alice")
        .expect("alice account entry");
    assert_eq!(alice.liquid_balance, 1200);
    assert_eq!(alice.vested_balance, 300);
    assert_eq!(alice.restricted_starter_claim_balance, 125);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn transfer_tracker_metrics_status_summarizes_lifecycle_and_latency() {
    let _guard = lock_transfer_test_state();
    super::with_transfer_tracker(|tracker| {
        tracker.by_action_id.insert(
            11,
            super::TrackedTransfer {
                action_id: 11,
                from_account_id: "player:a".to_string(),
                to_account_id: "player:b".to_string(),
                amount: 10,
                nonce: 1,
                status: TransferLifecycleStatus::Accepted,
                submitted_at_unix_ms: 1_500,
                updated_at_unix_ms: 1_500,
                error_code: None,
                error: None,
            },
        );
        tracker.by_action_id.insert(
            12,
            super::TrackedTransfer {
                action_id: 12,
                from_account_id: "player:a".to_string(),
                to_account_id: "player:c".to_string(),
                amount: 20,
                nonce: 2,
                status: TransferLifecycleStatus::Pending,
                submitted_at_unix_ms: 800,
                updated_at_unix_ms: 1_200,
                error_code: None,
                error: None,
            },
        );
        tracker.by_action_id.insert(
            13,
            super::TrackedTransfer {
                action_id: 13,
                from_account_id: "player:a".to_string(),
                to_account_id: "player:d".to_string(),
                amount: 30,
                nonce: 3,
                status: TransferLifecycleStatus::Confirmed,
                submitted_at_unix_ms: 100,
                updated_at_unix_ms: 200,
                error_code: None,
                error: None,
            },
        );
        tracker.by_action_id.insert(
            14,
            super::TrackedTransfer {
                action_id: 14,
                from_account_id: "player:a".to_string(),
                to_account_id: "player:e".to_string(),
                amount: 40,
                nonce: 4,
                status: TransferLifecycleStatus::Confirmed,
                submitted_at_unix_ms: 100,
                updated_at_unix_ms: 500,
                error_code: None,
                error: None,
            },
        );
        tracker.by_action_id.insert(
            15,
            super::TrackedTransfer {
                action_id: 15,
                from_account_id: "player:a".to_string(),
                to_account_id: "player:f".to_string(),
                amount: 50,
                nonce: 5,
                status: TransferLifecycleStatus::Confirmed,
                submitted_at_unix_ms: 100,
                updated_at_unix_ms: 1_000,
                error_code: None,
                error: None,
            },
        );
        tracker.by_action_id.insert(
            16,
            super::TrackedTransfer {
                action_id: 16,
                from_account_id: "player:a".to_string(),
                to_account_id: "player:g".to_string(),
                amount: 60,
                nonce: 6,
                status: TransferLifecycleStatus::Timeout,
                submitted_at_unix_ms: 50,
                updated_at_unix_ms: 1_500,
                error_code: Some("timeout".to_string()),
                error: Some("expired".to_string()),
            },
        );
        tracker.action_order.extend([11, 12, 13, 14, 15, 16]);
        let metrics = tracker.metrics_status(2_000);
        assert_eq!(metrics.tracked_records, 6);
        assert_eq!(metrics.accepted_count, 1);
        assert_eq!(metrics.pending_count, 1);
        assert_eq!(metrics.confirmed_count, 3);
        assert_eq!(metrics.failed_count, 0);
        assert_eq!(metrics.timeout_count, 1);
        assert_eq!(metrics.inflight_count, 2);
        assert_eq!(metrics.oldest_inflight_age_ms, Some(1_200));
        assert_eq!(metrics.recent_confirmation_latency.sample_count, 3);
        assert_eq!(
            metrics.recent_confirmation_latency.avg_latency_ms,
            Some(466)
        );
        assert_eq!(
            metrics.recent_confirmation_latency.max_latency_ms,
            Some(900)
        );
        assert_eq!(
            metrics.recent_confirmation_latency.p50_latency_ms,
            Some(400)
        );
        assert_eq!(
            metrics.recent_confirmation_latency.p95_latency_ms,
            Some(400)
        );
    });
}

#[test]
fn transfer_submit_handler_rejects_missing_signature() {
    let _guard = lock_transfer_test_state();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-transfer-submit-missing-signature",
            "world-transfer-submit-missing-signature",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));

    let (public_key, _) = transfer_test_signer(11);
    let from_account_id = main_token_account_id_from_node_public_key(public_key.as_str());
    let (to_public_key, _) = transfer_test_signer(12);
    let to_account_id = main_token_account_id_from_node_public_key(to_public_key.as_str());
    let body = format!(
        r#"{{"from_account_id":"{from_account_id}","to_account_id":"{to_account_id}","amount":7,"nonce":2,"public_key":"{public_key}"}}"#
    );

    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let request = format!(
        "POST /v1/chain/transfer/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    maybe_handle_transfer_submit_request(
        &mut server_stream,
        request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/transfer/submit",
        "node-transfer-submit-missing-signature",
        "world-transfer-submit-missing-signature",
        Path::new("."),
    )
    .expect("handler should process request");
    drop(server_stream);

    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    let (status, response): (u16, ChainTransferSubmitResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 400);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("invalid_request"));
}

#[test]
fn transfer_submit_handler_rejects_invalid_signature() {
    let _guard = lock_transfer_test_state();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-transfer-submit-invalid-signature",
            "world-transfer-submit-invalid-signature",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));

    let mut request = build_signed_transfer_request(13, 14, 7, 2);
    request.signature = format!("{}{}", "octransferauth:v1:", "f".repeat(128));
    let body = serde_json::to_string(&request).expect("serialize request");

    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let request = format!(
        "POST /v1/chain/transfer/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    maybe_handle_transfer_submit_request(
        &mut server_stream,
        request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/transfer/submit",
        "node-transfer-submit-invalid-signature",
        "world-transfer-submit-invalid-signature",
        Path::new("."),
    )
    .expect("handler should process request");
    drop(server_stream);

    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    let (status, response): (u16, ChainTransferSubmitResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 400);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("invalid_signature"));
}

#[test]
fn transfer_submit_handler_rejects_account_auth_mismatch() {
    let _guard = lock_transfer_test_state();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-transfer-submit-auth-mismatch",
            "world-transfer-submit-auth-mismatch",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));

    let mut request = build_signed_transfer_request(15, 16, 7, 2);
    let (mismatch_public_key, _) = transfer_test_signer(17);
    request.from_account_id =
        main_token_account_id_from_node_public_key(mismatch_public_key.as_str());
    let body = serde_json::to_string(&request).expect("serialize request");

    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let request = format!(
        "POST /v1/chain/transfer/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    maybe_handle_transfer_submit_request(
        &mut server_stream,
        request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/transfer/submit",
        "node-transfer-submit-auth-mismatch",
        "world-transfer-submit-auth-mismatch",
        Path::new("."),
    )
    .expect("handler should process request");
    drop(server_stream);

    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    let (status, response): (u16, ChainTransferSubmitResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 400);
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("account_auth_mismatch")
    );
}

#[test]
fn transfer_status_and_history_endpoint_report_confirmed_record() {
    let _guard = lock_transfer_test_state();
    let config = NodeConfig::new(
        "node-transfer-query-ok",
        "world-transfer-query-ok",
        NodeRole::Sequencer,
    )
    .expect("node config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick interval");
    let mut node_runtime = NodeRuntime::new(config).with_execution_hook(NoopExecutionHook);
    node_runtime.start().expect("start node runtime");
    let runtime = Arc::new(Mutex::new(node_runtime));

    let (mut submit_server, mut submit_client) = tcp_stream_pair();
    let submit_request = build_signed_transfer_request(21, 22, 3, 8);
    let submit_body = serde_json::to_string(&submit_request).expect("serialize request");
    let submit_http = format!(
        "POST /v1/chain/transfer/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        submit_body.len(),
        submit_body
    );
    maybe_handle_transfer_submit_request(
        &mut submit_server,
        submit_http.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/transfer/submit",
        "node-transfer-query-ok",
        "world-transfer-query-ok",
        Path::new("."),
    )
    .expect("submit should be handled");
    drop(submit_server);

    let mut submit_response_bytes = Vec::new();
    submit_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    submit_client
        .read_to_end(&mut submit_response_bytes)
        .expect("read submit response");
    let (_, submit_response): (u16, ChainTransferSubmitResponse) =
        decode_http_json_response(&submit_response_bytes);
    assert_eq!(
        submit_response.lifecycle_status,
        Some(TransferLifecycleStatus::Accepted)
    );
    let action_id = submit_response.action_id.expect("action_id");

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observed_confirmed = false;
    while Instant::now() < deadline {
        let (mut status_server, mut status_client) = tcp_stream_pair();
        let status_http = format!(
            "GET /v1/chain/transfer/status?action_id={} HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n",
            action_id
        );
        maybe_handle_transfer_submit_request(
            &mut status_server,
            status_http.as_bytes(),
            &runtime,
            "GET",
            "/v1/chain/transfer/status",
            "node-transfer-query-ok",
            "world-transfer-query-ok",
            Path::new("."),
        )
        .expect("status should be handled");
        drop(status_server);

        status_client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set timeout");
        let mut status_response_bytes = Vec::new();
        status_client
            .read_to_end(&mut status_response_bytes)
            .expect("read status response");
        let (_, status_response): (u16, ChainTransferStatusResponse) =
            decode_http_json_response(&status_response_bytes);
        let status = status_response.status.expect("status payload");
        if status.status == TransferLifecycleStatus::Confirmed {
            observed_confirmed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(80));
    }
    assert!(
        observed_confirmed,
        "status should eventually become confirmed"
    );

    let (mut history_server, mut history_client) = tcp_stream_pair();
    let history_http = format!(
        "GET /v1/chain/transfer/history?action_id={} HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n",
        action_id
    );
    maybe_handle_transfer_submit_request(
        &mut history_server,
        history_http.as_bytes(),
        &runtime,
        "GET",
        "/v1/chain/transfer/history",
        "node-transfer-query-ok",
        "world-transfer-query-ok",
        Path::new("."),
    )
    .expect("history should be handled");
    drop(history_server);

    history_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    let mut history_response_bytes = Vec::new();
    history_client
        .read_to_end(&mut history_response_bytes)
        .expect("read history response");
    let (_, history_response): (u16, ChainTransferHistoryResponse) =
        decode_http_json_response(&history_response_bytes);
    assert!(history_response.ok);
    assert_eq!(history_response.total, 1);
    assert_eq!(history_response.items[0].action_id, action_id);

    runtime
        .lock()
        .expect("lock runtime for stop")
        .stop()
        .expect("stop node runtime");
}

#[test]
fn explorer_overview_and_transaction_queries_return_expected_payloads() {
    let _guard = lock_transfer_test_state();
    let config = NodeConfig::new(
        "node-transfer-explorer-ok",
        "world-transfer-explorer-ok",
        NodeRole::Sequencer,
    )
    .expect("node config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick interval");
    let mut node_runtime = NodeRuntime::new(config).with_execution_hook(NoopExecutionHook);
    node_runtime.start().expect("start node runtime");
    let runtime = Arc::new(Mutex::new(node_runtime));

    let (mut submit_server, mut submit_client) = tcp_stream_pair();
    let submit_request = build_signed_transfer_request(31, 32, 4, 9);
    let submit_body = serde_json::to_string(&submit_request).expect("serialize request");
    let submit_http = format!(
        "POST /v1/chain/transfer/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        submit_body.len(),
        submit_body
    );
    maybe_handle_transfer_submit_request(
        &mut submit_server,
        submit_http.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/transfer/submit",
        "node-transfer-explorer-ok",
        "world-transfer-explorer-ok",
        Path::new("."),
    )
    .expect("submit should be handled");
    drop(submit_server);

    let mut submit_response_bytes = Vec::new();
    submit_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    submit_client
        .read_to_end(&mut submit_response_bytes)
        .expect("read submit response");
    let (_, submit_response): (u16, ChainTransferSubmitResponse) =
        decode_http_json_response(&submit_response_bytes);
    let action_id = submit_response.action_id.expect("action_id");

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut confirmed = false;
    while Instant::now() < deadline {
        let (mut status_server, mut status_client) = tcp_stream_pair();
        let status_http = format!(
            "GET /v1/chain/transfer/status?action_id={} HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n",
            action_id
        );
        maybe_handle_transfer_submit_request(
            &mut status_server,
            status_http.as_bytes(),
            &runtime,
            "GET",
            "/v1/chain/transfer/status",
            "node-transfer-explorer-ok",
            "world-transfer-explorer-ok",
            Path::new("."),
        )
        .expect("status should be handled");
        drop(status_server);

        status_client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set timeout");
        let mut status_response_bytes = Vec::new();
        status_client
            .read_to_end(&mut status_response_bytes)
            .expect("read status response");
        let (_, status_response): (u16, ChainTransferStatusResponse) =
            decode_http_json_response(&status_response_bytes);
        if status_response
            .status
            .as_ref()
            .is_some_and(|record| record.status == TransferLifecycleStatus::Confirmed)
        {
            confirmed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(80));
    }
    assert!(confirmed, "status should eventually become confirmed");

    let (mut overview_server, mut overview_client) = tcp_stream_pair();
    let overview_http = "GET /v1/chain/explorer/overview HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n";
    maybe_handle_transfer_submit_request(
        &mut overview_server,
        overview_http.as_bytes(),
        &runtime,
        "GET",
        "/v1/chain/explorer/overview",
        "node-transfer-explorer-ok",
        "world-transfer-explorer-ok",
        Path::new("."),
    )
    .expect("overview should be handled");
    drop(overview_server);

    overview_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    let mut overview_response_bytes = Vec::new();
    overview_client
        .read_to_end(&mut overview_response_bytes)
        .expect("read overview response");
    let (_, overview): (u16, ChainExplorerOverviewResponse) =
        decode_http_json_response(&overview_response_bytes);
    assert!(overview.ok);
    assert_eq!(overview.node_id, "node-transfer-explorer-ok");
    assert_eq!(overview.world_id, "world-transfer-explorer-ok");
    assert!(overview.transfer_total >= 1);
    assert!(overview.transfer_confirmed >= 1);
    assert!(overview.latest_height >= 1);

    let (mut txs_server, mut txs_client) = tcp_stream_pair();
    let txs_http =
        "GET /v1/chain/explorer/transactions?status=confirmed&limit=10 HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n";
    maybe_handle_transfer_submit_request(
        &mut txs_server,
        txs_http.as_bytes(),
        &runtime,
        "GET",
        "/v1/chain/explorer/transactions",
        "node-transfer-explorer-ok",
        "world-transfer-explorer-ok",
        Path::new("."),
    )
    .expect("transactions should be handled");
    drop(txs_server);

    txs_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    let mut txs_response_bytes = Vec::new();
    txs_client
        .read_to_end(&mut txs_response_bytes)
        .expect("read transactions response");
    let (_, txs): (u16, ChainTransferHistoryResponse) =
        decode_http_json_response(&txs_response_bytes);
    assert!(txs.ok);
    assert_eq!(txs.status_filter, Some(TransferLifecycleStatus::Confirmed));
    assert!(txs.items.iter().any(|item| item.action_id == action_id));

    let (mut tx_server, mut tx_client) = tcp_stream_pair();
    let tx_http = format!(
        "GET /v1/chain/explorer/transaction?action_id={} HTTP/1.1\r\nHost: 127.0.0.1:5121\r\n\r\n",
        action_id
    );
    maybe_handle_transfer_submit_request(
        &mut tx_server,
        tx_http.as_bytes(),
        &runtime,
        "GET",
        "/v1/chain/explorer/transaction",
        "node-transfer-explorer-ok",
        "world-transfer-explorer-ok",
        Path::new("."),
    )
    .expect("transaction detail should be handled");
    drop(tx_server);

    tx_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");
    let mut tx_response_bytes = Vec::new();
    tx_client
        .read_to_end(&mut tx_response_bytes)
        .expect("read transaction detail response");
    let (_, tx_detail): (u16, ChainTransferStatusResponse) =
        decode_http_json_response(&tx_response_bytes);
    assert!(tx_detail.ok);
    assert_eq!(tx_detail.action_id, action_id);
    assert_eq!(
        tx_detail.status.as_ref().map(|item| item.status),
        Some(TransferLifecycleStatus::Confirmed)
    );

    runtime
        .lock()
        .expect("lock runtime for stop")
        .stop()
        .expect("stop node runtime");
}
