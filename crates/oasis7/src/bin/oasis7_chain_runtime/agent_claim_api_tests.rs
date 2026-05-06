use super::{
    maybe_handle_agent_claim_request, parse_approval_request_status, parse_approval_requests_query,
    reset_agent_claim_api_state_for_tests, ChainAgentClaimActionResponse,
    ChainAgentClaimApprovalRequestsResponse, ChainAgentClaimSubmitRequest,
    ChainFirstAgentClaimApprovalApproveRequest, ChainFirstAgentClaimApprovalRejectRequest,
    ChainFirstAgentClaimApprovalRequestSubmit,
};
use crate::cli::DEFAULT_NODE_ID;
use ed25519_dalek::SigningKey;
use oasis7::geometry::GeoPos;
use oasis7::runtime::{
    Action, FirstAgentClaimApprovalRequestStatus, GovernanceExecutionPolicy,
    GovernanceMainTokenControllerRegistry, GovernanceThresholdSignerPolicy, MainTokenSupplyState,
    World, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
    MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
};
use oasis7_node::{
    NodeConfig, NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook, NodeRole,
    NodeRuntime,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn agent_claim_api_test_guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-agent-claim-api-{prefix}-{unique}"))
}

fn wait_for_committed_height(runtime: &Arc<Mutex<NodeRuntime>>, minimum_height: u64) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        let height = runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .snapshot()
            .consensus
            .committed_height;
        if height >= minimum_height {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let height = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .snapshot()
        .consensus
        .committed_height;
    panic!("timed out waiting for committed height >= {minimum_height}, got {height}");
}

fn started_runtime(node_id: &str) -> Arc<Mutex<NodeRuntime>> {
    let config = NodeConfig::new(node_id, "world-agent-claim-api", NodeRole::Sequencer)
        .expect("node config")
        .with_tick_interval(Duration::from_millis(20))
        .expect("tick interval");
    let mut node_runtime = NodeRuntime::new(config).with_execution_hook(NoopExecutionHook);
    node_runtime.start().expect("start node runtime");
    Arc::new(Mutex::new(node_runtime))
}

fn stop_runtime(runtime: &Arc<Mutex<NodeRuntime>>) {
    runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .stop()
        .expect("stop node runtime");
}

fn register_agent(world: &mut World, agent_id: &str) {
    world.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        pos: GeoPos::new(0, 0, 0),
    });
    world.step().expect("register agent");
}

fn configure_claim_world() -> World {
    let mut world = World::new();
    world
        .set_governance_execution_policy(GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    register_agent(&mut world, "alice");
    register_agent(&mut world, "bob");
    world
        .set_agent_reputation_score("alice", 0)
        .expect("set reputation");
    world.set_main_token_supply(MainTokenSupplyState {
        total_supply: 2_000,
        circulating_supply: 0,
        ..MainTokenSupplyState::default()
    });
    world
}

fn allowlist_restricted_grant_admins(world: &mut World, admin_account_ids: &[&str]) {
    let mut controller_signer_policies = BTreeMap::from([(
        "msig.genesis.v1".to_string(),
        GovernanceThresholdSignerPolicy {
            threshold: 1,
            allowed_public_keys: BTreeSet::from([hex::encode(
                SigningKey::from_bytes(&[1; 32]).verifying_key().to_bytes(),
            )]),
        },
    )]);
    controller_signer_policies.insert(
        "msig.ecosystem_governance.v1".to_string(),
        GovernanceThresholdSignerPolicy {
            threshold: 1,
            allowed_public_keys: BTreeSet::from([hex::encode(
                SigningKey::from_bytes(&[2; 32]).verifying_key().to_bytes(),
            )]),
        },
    );
    for (index, account_id) in admin_account_ids.iter().enumerate() {
        controller_signer_policies.insert(
            (*account_id).to_string(),
            GovernanceThresholdSignerPolicy {
                threshold: 1,
                allowed_public_keys: BTreeSet::from([format!("{:064x}", index + 7)]),
            },
        );
    }
    world
        .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
            genesis_controller_account_id: "msig.genesis.v1".to_string(),
            treasury_bucket_controller_slots: BTreeMap::from([(
                MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
                "msig.ecosystem_governance.v1".to_string(),
            )]),
            restricted_starter_claim_admin_account_ids: admin_account_ids
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            controller_signer_policies,
        })
        .expect("set restricted grant admin registry");
}

fn save_world(world: &World, prefix: &str) -> PathBuf {
    let dir = unique_temp_dir(prefix);
    fs::create_dir_all(&dir).expect("create temp dir");
    world.save_to_dir(&dir).expect("save world");
    dir
}

fn pending_request_world_dir() -> PathBuf {
    let mut world = configure_claim_world();
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);
    world
        .set_main_token_treasury_balance(
            MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
            1_000,
        )
        .expect("seed treasury");
    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit approval request");
    save_world(&world, "pending")
}

fn approved_request_world_dir() -> PathBuf {
    let mut world = configure_claim_world();
    allowlist_restricted_grant_admins(&mut world, &["liveops"]);
    world
        .set_main_token_treasury_balance(
            MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
            1_000,
        )
        .expect("seed treasury");
    world.submit_action(Action::SubmitFirstAgentClaimApprovalRequest {
        claimer_agent_id: "alice".to_string(),
    });
    world.step().expect("submit approval request");
    world.submit_action(Action::ApproveFirstAgentClaimApprovalRequest {
        operator_account_id: "liveops".to_string(),
        request_id: 1,
        expires_at_epoch: 10,
    });
    world.step().expect("approve approval request");
    save_world(&world, "approved")
}

fn build_http_request(path: &str, method: &str, body: &str) -> Vec<u8> {
    format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

#[test]
fn approval_request_status_filter_parses_known_values() {
    assert_eq!(
        parse_approval_request_status("pending").expect("pending"),
        FirstAgentClaimApprovalRequestStatus::Pending
    );
    assert_eq!(
        parse_approval_request_status("PeNdInG").expect("mixed-case pending"),
        FirstAgentClaimApprovalRequestStatus::Pending
    );
    assert_eq!(
        parse_approval_request_status("approved").expect("approved"),
        FirstAgentClaimApprovalRequestStatus::Approved
    );
    assert_eq!(
        parse_approval_request_status("APPROVED").expect("uppercase approved"),
        FirstAgentClaimApprovalRequestStatus::Approved
    );
    assert_eq!(
        parse_approval_request_status("rejected").expect("rejected"),
        FirstAgentClaimApprovalRequestStatus::Rejected
    );
    assert_eq!(
        parse_approval_request_status("Rejected").expect("mixed-case rejected"),
        FirstAgentClaimApprovalRequestStatus::Rejected
    );
}

#[test]
fn approval_requests_query_percent_decodes_filters() {
    let query = parse_approval_requests_query(
        "/v1/chain/agent-claim/approval-requests?status=PeNdInG&claimer_agent_id=ali%63e",
    )
    .expect("parse approval requests query");
    assert_eq!(query.claimer_agent_id_filter.as_deref(), Some("alice"));
    assert_eq!(
        query.status_filter,
        Some(FirstAgentClaimApprovalRequestStatus::Pending)
    );
}

#[test]
fn approval_request_submit_handler_rejects_missing_claimer_in_preflight() {
    let _guard = agent_claim_api_test_guard();
    reset_agent_claim_api_state_for_tests();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-agent-claim-bad-submit",
            "world-agent-claim-bad-submit",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));
    let world_dir = pending_request_world_dir();

    let request = ChainFirstAgentClaimApprovalRequestSubmit {
        claimer_agent_id: "unknown-agent".to_string(),
    };
    let body = serde_json::to_string(&request).expect("serialize request");
    let http_request = build_http_request(
        "/v1/chain/agent-claim/approval-request/submit",
        "POST",
        &body,
    );
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let handled = maybe_handle_agent_claim_request(
        &mut server_stream,
        &http_request,
        &runtime,
        "POST",
        "/v1/chain/agent-claim/approval-request/submit",
        "/v1/chain/agent-claim/approval-request/submit",
        DEFAULT_NODE_ID,
        "world-agent-claim-bad-submit",
        world_dir.as_path(),
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
    let (status, response): (u16, ChainAgentClaimActionResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 409);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("action_rejected"));
    assert!(response
        .error
        .as_deref()
        .is_some_and(|error| error.contains("claimer_agent_id not found")));
}

#[test]
fn approval_requests_list_handler_returns_pending_request_rows() {
    let _guard = agent_claim_api_test_guard();
    reset_agent_claim_api_state_for_tests();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-agent-claim-list",
            "world-agent-claim-list",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));
    let world_dir = pending_request_world_dir();
    let http_request = build_http_request(
        "/v1/chain/agent-claim/approval-requests?status=PeNdInG&claimer_agent_id=ali%63e",
        "GET",
        "",
    );
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let handled = maybe_handle_agent_claim_request(
        &mut server_stream,
        &http_request,
        &runtime,
        "GET",
        "/v1/chain/agent-claim/approval-requests?status=pending",
        "/v1/chain/agent-claim/approval-requests",
        DEFAULT_NODE_ID,
        "world-agent-claim-list",
        world_dir.as_path(),
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
    let (status, response): (u16, ChainAgentClaimApprovalRequestsResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 200);
    assert!(response.ok);
    assert_eq!(
        response.status_filter,
        Some(FirstAgentClaimApprovalRequestStatus::Pending)
    );
    assert_eq!(response.total, 1);
    assert_eq!(response.items[0].request_id, 1);
    assert_eq!(response.items[0].claimer_agent_id, "alice");
}

#[test]
fn approval_request_approve_handler_accepts_allowlisted_operator() {
    let _guard = agent_claim_api_test_guard();
    reset_agent_claim_api_state_for_tests();
    let runtime = started_runtime("node-agent-claim-approve");
    let world_dir = pending_request_world_dir();

    let request = ChainFirstAgentClaimApprovalApproveRequest {
        operator_account_id: "liveops".to_string(),
        request_id: 1,
        expires_at_epoch: 10,
    };
    let body = serde_json::to_string(&request).expect("serialize request");
    let http_request = build_http_request(
        "/v1/chain/agent-claim/approval-request/approve",
        "POST",
        &body,
    );
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let handled = maybe_handle_agent_claim_request(
        &mut server_stream,
        &http_request,
        &runtime,
        "POST",
        "/v1/chain/agent-claim/approval-request/approve",
        "/v1/chain/agent-claim/approval-request/approve",
        DEFAULT_NODE_ID,
        "world-agent-claim-approve",
        world_dir.as_path(),
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
    let (status, response): (u16, ChainAgentClaimActionResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 200);
    assert!(response.ok);
    assert_eq!(response.action_id, Some(1));
    assert_eq!(
        response
            .preview
            .as_ref()
            .and_then(|preview| preview.approval_status),
        Some(FirstAgentClaimApprovalRequestStatus::Approved)
    );
    assert_eq!(
        response
            .preview
            .as_ref()
            .and_then(|preview| preview.request_id),
        Some(1)
    );

    wait_for_committed_height(&runtime, 1);
    stop_runtime(&runtime);
}

#[test]
fn approval_request_reject_handler_rejects_non_admin_operator() {
    let _guard = agent_claim_api_test_guard();
    reset_agent_claim_api_state_for_tests();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-agent-claim-reject",
            "world-agent-claim-reject",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));
    let world_dir = pending_request_world_dir();

    let request = ChainFirstAgentClaimApprovalRejectRequest {
        operator_account_id: "qa".to_string(),
        request_id: 1,
        reason: "not allowed".to_string(),
    };
    let body = serde_json::to_string(&request).expect("serialize request");
    let http_request = build_http_request(
        "/v1/chain/agent-claim/approval-request/reject",
        "POST",
        &body,
    );
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let handled = maybe_handle_agent_claim_request(
        &mut server_stream,
        &http_request,
        &runtime,
        "POST",
        "/v1/chain/agent-claim/approval-request/reject",
        "/v1/chain/agent-claim/approval-request/reject",
        DEFAULT_NODE_ID,
        "world-agent-claim-reject",
        world_dir.as_path(),
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
    let (status, response): (u16, ChainAgentClaimActionResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 409);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("action_rejected"));
    assert!(response
        .error
        .as_deref()
        .is_some_and(|error| error.contains("not allowlisted admin")));
}

#[test]
fn agent_claim_submit_handler_accepts_slot_1_claim_after_approval() {
    let _guard = agent_claim_api_test_guard();
    reset_agent_claim_api_state_for_tests();
    let runtime = started_runtime("node-agent-claim-submit");
    let world_dir = approved_request_world_dir();

    let request = ChainAgentClaimSubmitRequest {
        claimer_agent_id: "alice".to_string(),
        target_agent_id: "bob".to_string(),
    };
    let body = serde_json::to_string(&request).expect("serialize request");
    let http_request = build_http_request("/v1/chain/agent-claim/submit", "POST", &body);
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let handled = maybe_handle_agent_claim_request(
        &mut server_stream,
        &http_request,
        &runtime,
        "POST",
        "/v1/chain/agent-claim/submit",
        "/v1/chain/agent-claim/submit",
        DEFAULT_NODE_ID,
        "world-agent-claim-submit",
        world_dir.as_path(),
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
    let (status, response): (u16, ChainAgentClaimActionResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 200);
    assert!(response.ok);
    assert_eq!(
        response
            .preview
            .as_ref()
            .and_then(|preview| preview.target_agent_id.as_deref()),
        Some("bob")
    );
    assert_eq!(
        response
            .preview
            .as_ref()
            .and_then(|preview| preview.slot_index),
        Some(1)
    );

    wait_for_committed_height(&runtime, 1);
    stop_runtime(&runtime);
}
