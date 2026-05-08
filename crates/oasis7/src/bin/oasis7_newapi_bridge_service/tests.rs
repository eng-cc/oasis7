use std::env;
use std::fs;
use std::io::{ErrorKind, Read};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use super::api::{read_http_request, write_http_response, HttpRequest};
use super::model::{
    BindBridgeUserRequest, BridgeBindingStatus, BridgeLedgerState, CreateDepositRouteRequest,
    DepositRouteStatus, OperatorReviewRequest,
};
use super::service::{
    BridgePricingRuleConfig, BridgeService, BridgeServiceConfig, CreditTargetType,
};
use super::store::BridgeStateStore;
use super::{dispatch_request, parse_cli_options};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn bind_user_persists_and_reuses_existing_binding() {
    let test_service = test_service("bind-reuse", 900);
    let created = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("create binding");
    assert!(!created.reused_existing_binding);
    assert_eq!(created.binding_status, BridgeBindingStatus::Active);

    let reused = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            2_000,
        )
        .expect("reuse binding");
    assert!(reused.reused_existing_binding);
    assert_eq!(reused.bridge_user_id, created.bridge_user_id);

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.bindings.len(), 1);
    let persisted = fs::read(test_service.state_path.as_path()).expect("read state file");
    let persisted_json: Value =
        serde_json::from_slice(persisted.as_slice()).expect("parse state file");
    assert_eq!(
        persisted_json
            .get("bindings")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
}

#[test]
fn bind_user_rejects_conflicting_active_binding() {
    let test_service = test_service("bind-conflict", 900);
    test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("initial binding");

    let err = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-2".to_string(),
            },
            2_000,
        )
        .expect_err("conflict expected");
    assert_eq!(err.status_code, 409);
    assert_eq!(err.code, "binding_conflict");
}

#[test]
fn create_deposit_route_persists_and_reuses_active_route() {
    let test_service = test_service("route-reuse", 900);
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");

    let first = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id.clone(),
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("route");
    assert!(!first.reused_existing_route);
    assert_eq!(first.route_status, DepositRouteStatus::Issued);
    assert_eq!(first.deposit_account_id, "oc:bridge:000001");

    let reused = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-2".to_string()),
                topup_plan_id: None,
            },
            2_100,
        )
        .expect("reuse route");
    assert!(reused.reused_existing_route);
    assert_eq!(reused.route_id, first.route_id);

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.routes.len(), 1);
}

#[test]
fn reconcile_promotes_confirmed_deposit_into_reconciled_credit() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    let test_service = test_service_with_endpoints(
        "reconcile-happy",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        2,
    );
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");
    let route = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("route");

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-1".to_string(),
            action_id: 11,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: route.deposit_account_id.clone(),
            amount: 100,
            submitted_at_unix_ms: 5_000,
            updated_at_unix_ms: 5_100,
            block_height: Some(10),
        }],
    });

    let first = test_service
        .service
        .reconcile_once(6_000)
        .expect("first reconcile");
    assert_eq!(first.reconciled_credit_count, 0);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger.len(), 1);
    assert_eq!(
        snapshot.ledger[0].state,
        BridgeLedgerState::PendingConfirmations
    );

    chain_server.set_committed_height(11);
    let second = test_service
        .service
        .reconcile_once(7_000)
        .expect("second reconcile");
    assert_eq!(second.updated_deposit_count, 1);
    assert_eq!(second.reconciled_credit_count, 1);

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::Reconciled);
    assert_eq!(snapshot.ledger[0].total_credit_units, 15);
    assert_eq!(credit_server.recorded_requests().len(), 1);
}

#[test]
fn reconcile_marks_underpay_for_manual_review() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    let test_service = test_service_with_endpoints(
        "reconcile-underpay",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        1,
    );
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");
    let route = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("route");

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-underpay".to_string(),
            action_id: 12,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: route.deposit_account_id.clone(),
            amount: 99,
            submitted_at_unix_ms: 5_000,
            updated_at_unix_ms: 5_100,
            block_height: Some(10),
        }],
    });

    let reconcile = test_service
        .service
        .reconcile_once(6_000)
        .expect("reconcile");
    assert_eq!(reconcile.manual_review_count, 1);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::ManualReview);
    assert_eq!(
        snapshot.ledger[0].review_reason.as_deref(),
        Some("underpay")
    );
    assert!(credit_server.recorded_requests().is_empty());
}

#[test]
fn reconcile_retries_credit_adapter_with_stable_idempotency_key() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    credit_server.fail_first_requests(1);
    let test_service = test_service_with_endpoints(
        "reconcile-retry",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        1,
    );
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");
    let route = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("route");

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-retry".to_string(),
            action_id: 13,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: route.deposit_account_id.clone(),
            amount: 100,
            submitted_at_unix_ms: 5_000,
            updated_at_unix_ms: 5_100,
            block_height: Some(10),
        }],
    });

    let first = test_service
        .service
        .reconcile_once(6_000)
        .expect("first reconcile");
    assert_eq!(first.failed_credit_count, 1);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::Failed);
    assert_eq!(snapshot.ledger[0].credit_attempt_count, 1);

    let second = test_service
        .service
        .reconcile_once(7_000)
        .expect("second reconcile");
    assert_eq!(second.reconciled_credit_count, 1);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::Reconciled);
    let requests = credit_server.recorded_requests();
    assert_eq!(requests.len(), 2);
    let first_key = requests[0]
        .get("idempotency_key")
        .and_then(Value::as_str)
        .expect("first idempotency key");
    let second_key = requests[1]
        .get("idempotency_key")
        .and_then(Value::as_str)
        .expect("second idempotency key");
    assert_eq!(first_key, second_key);
}

#[test]
fn reconcile_tracks_same_tx_hash_actions_as_distinct_ledger_rows() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    let test_service = test_service_with_endpoints(
        "reconcile-multi-action",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        1,
    );
    let deposit_account_id = issue_default_route(&test_service);

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![
            MockChainTx {
                tx_hash: "tx-shared".to_string(),
                action_id: 31,
                from_account_id: "oc:pk:sender-1".to_string(),
                to_account_id: deposit_account_id.clone(),
                amount: 100,
                submitted_at_unix_ms: 5_000,
                updated_at_unix_ms: 5_100,
                block_height: Some(10),
            },
            MockChainTx {
                tx_hash: "tx-shared".to_string(),
                action_id: 32,
                from_account_id: "oc:pk:sender-1".to_string(),
                to_account_id: deposit_account_id,
                amount: 100,
                submitted_at_unix_ms: 5_200,
                updated_at_unix_ms: 5_300,
                block_height: Some(10),
            },
        ],
    });

    let reconcile = test_service
        .service
        .reconcile_once(6_000)
        .expect("reconcile");
    assert_eq!(reconcile.observed_new_deposit_count, 2);
    assert_eq!(reconcile.reconciled_credit_count, 1);
    assert_eq!(reconcile.manual_review_count, 1);

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger.len(), 2);
    assert_eq!(
        snapshot.ledger[0].chain_tx_id,
        snapshot.ledger[1].chain_tx_id
    );
    assert_ne!(
        snapshot.ledger[0].chain_action_id,
        snapshot.ledger[1].chain_action_id
    );
    assert_ne!(
        snapshot.ledger[0].idempotency_key,
        snapshot.ledger[1].idempotency_key
    );
    assert_eq!(credit_server.recorded_requests().len(), 1);
}

#[test]
fn operator_review_can_close_manual_review_row() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    let test_service = test_service_with_endpoints(
        "operator-review",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        1,
    );
    let deposit_account_id = issue_default_route(&test_service);

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![
            MockChainTx {
                tx_hash: "tx-a".to_string(),
                action_id: 21,
                from_account_id: "oc:pk:sender-1".to_string(),
                to_account_id: deposit_account_id.clone(),
                amount: 100,
                submitted_at_unix_ms: 5_000,
                updated_at_unix_ms: 5_100,
                block_height: Some(10),
            },
            MockChainTx {
                tx_hash: "tx-b".to_string(),
                action_id: 22,
                from_account_id: "oc:pk:sender-1".to_string(),
                to_account_id: deposit_account_id,
                amount: 100,
                submitted_at_unix_ms: 5_200,
                updated_at_unix_ms: 5_300,
                block_height: Some(10),
            },
        ],
    });

    test_service
        .service
        .reconcile_once(6_000)
        .expect("reconcile");
    let manual_review_id = test_service
        .service
        .snapshot()
        .ledger
        .iter()
        .find(|entry| entry.state == BridgeLedgerState::ManualReview)
        .map(|entry| entry.bridge_deposit_id.clone())
        .expect("manual review entry");
    let response = test_service
        .service
        .apply_operator_review(
            manual_review_id.as_str(),
            OperatorReviewRequest {
                resolution: "close".to_string(),
                operator_note: Some("handled offline".to_string()),
            },
            7_000,
        )
        .expect("apply review");
    assert_eq!(response.state, BridgeLedgerState::Closed);
}

#[test]
fn operator_review_invalid_resolution_lists_resolve_alias() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    let test_service = test_service_with_endpoints(
        "operator-review-invalid-resolution",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        1,
    );
    let deposit_account_id = issue_default_route(&test_service);

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-invalid-resolution".to_string(),
            action_id: 33,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: deposit_account_id,
            amount: 99,
            submitted_at_unix_ms: 5_000,
            updated_at_unix_ms: 5_100,
            block_height: Some(10),
        }],
    });
    test_service
        .service
        .reconcile_once(6_000)
        .expect("reconcile");
    let manual_review_id = test_service
        .service
        .snapshot()
        .ledger
        .iter()
        .find(|entry| entry.state == BridgeLedgerState::ManualReview)
        .map(|entry| entry.bridge_deposit_id.clone())
        .expect("manual review entry");

    let err = test_service
        .service
        .apply_operator_review(
            manual_review_id.as_str(),
            OperatorReviewRequest {
                resolution: "invalid".to_string(),
                operator_note: None,
            },
            7_000,
        )
        .expect_err("invalid resolution");
    assert_eq!(err.status_code, 400);
    assert_eq!(err.code, "invalid_resolution");
    assert!(err.message.contains("mark_resolved"));
    assert!(err.message.contains("resolve"));
    assert!(err.message.contains("close"));
}

#[test]
fn create_deposit_route_expires_old_route_before_reissuing() {
    let test_service = test_service("route-expire", 1);
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");

    let first = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id.clone(),
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("first route");
    let second = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-2".to_string()),
                topup_plan_id: None,
            },
            3_500,
        )
        .expect("second route");
    assert_ne!(second.route_id, first.route_id);
    assert_eq!(second.deposit_account_id, "oc:bridge:000002");

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.routes.len(), 2);
    assert_eq!(snapshot.routes[0].status, DepositRouteStatus::Expired);
    assert_eq!(snapshot.routes[1].status, DepositRouteStatus::Issued);
}

#[test]
fn dispatch_request_handles_bind_and_route_http_contract() {
    let chain_server = MockChainServer::spawn();
    let credit_server = MockCreditServer::spawn();
    let test_service = test_service_with_endpoints(
        "http-contract",
        900,
        Some(chain_server.base_url.clone()),
        Some(credit_server.base_url.clone()),
        1,
    );
    let bind_response = dispatch_request(
        &test_service.service,
        HttpRequest {
            method: "POST".to_string(),
            path: "/v1/bridge/bind".to_string(),
            body: serde_json::to_vec(&serde_json::json!({
                "newapi_user_ref": "user-1",
                "oasis_sender_account_id": "oc:pk:sender-1"
            }))
            .expect("encode bind request"),
        },
    )
    .expect("dispatch bind");
    assert_eq!(bind_response.status_code, 200);
    let bind_json: Value =
        serde_json::from_slice(bind_response.body.as_slice()).expect("parse bind response");
    let bridge_user_id = bind_json
        .get("bridge_user_id")
        .and_then(Value::as_str)
        .expect("bridge_user_id")
        .to_string();

    let route_response = dispatch_request(
        &test_service.service,
        HttpRequest {
            method: "POST".to_string(),
            path: "/v1/bridge/deposit-route".to_string(),
            body: serde_json::to_vec(&serde_json::json!({
                "bridge_user_id": bridge_user_id,
                "pricing_version": "pv-1"
            }))
            .expect("encode route request"),
        },
    )
    .expect("dispatch route");
    assert_eq!(route_response.status_code, 200);
    let route_json: Value =
        serde_json::from_slice(route_response.body.as_slice()).expect("parse route response");
    assert_eq!(
        route_json.get("route_status").and_then(Value::as_str),
        Some("issued")
    );

    let reconcile_response = dispatch_request(
        &test_service.service,
        HttpRequest {
            method: "POST".to_string(),
            path: "/v1/bridge/reconcile".to_string(),
            body: Vec::new(),
        },
    )
    .expect("dispatch reconcile");
    assert_eq!(reconcile_response.status_code, 200);
}

#[test]
fn dispatch_request_returns_405_for_known_path_with_wrong_method() {
    let test_service = test_service("http-405", 900);
    let response = dispatch_request(
        &test_service.service,
        HttpRequest {
            method: "GET".to_string(),
            path: "/v1/bridge/bind".to_string(),
            body: Vec::new(),
        },
    )
    .expect("dispatch 405");
    assert_eq!(response.status_code, 405);
    let payload: Value = serde_json::from_slice(response.body.as_slice()).expect("parse 405");
    assert_eq!(
        payload
            .get("error")
            .and_then(|value| value.get("code"))
            .and_then(Value::as_str),
        Some("method_not_allowed")
    );
}

#[test]
fn store_reloads_persisted_bindings_and_routes() {
    let test_service = test_service("reload-state", 900);
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");
    test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("route");

    let reloaded_store =
        BridgeStateStore::new(test_service.state_path.clone()).expect("reload bridge state store");
    let snapshot = reloaded_store.snapshot();
    assert_eq!(snapshot.bindings.len(), 1);
    assert_eq!(snapshot.routes.len(), 1);
    assert_eq!(snapshot.ledger.len(), 0);
}

#[test]
fn parse_cli_options_rejects_zero_route_ttl() {
    let err = parse_cli_options(vec!["--route-ttl-seconds".to_string(), "0".to_string()])
        .expect_err("ttl validation");
    assert!(err.contains("greater than 0"));
}

#[test]
fn parse_cli_options_rejects_route_ttl_that_overflows_milliseconds() {
    let err = parse_cli_options(vec![
        "--route-ttl-seconds".to_string(),
        u64::MAX.to_string(),
    ])
    .expect_err("ttl overflow validation");
    assert!(err.contains("too large") || err.contains("supported millisecond range"));
}

#[test]
fn create_deposit_route_rejects_ttl_overflow_from_service_config() {
    let test_service = test_service("ttl-overflow", u64::MAX);
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");
    let err = test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect_err("ttl overflow");
    assert_eq!(err.status_code, 500);
    assert_eq!(err.code, "internal_error");
}

#[test]
fn parse_cli_options_accepts_bridge_automation_flags() {
    let options = parse_cli_options(vec![
        "--chain-base-url".to_string(),
        "http://127.0.0.1:5121".to_string(),
        "--pricing-rule".to_string(),
        "pv-1:100:15:2".to_string(),
        "--credit-adapter-url".to_string(),
        "http://127.0.0.1:6001/v1/admin/credits/apply".to_string(),
        "--credit-target-type".to_string(),
        "quota".to_string(),
        "--reconcile-interval-seconds".to_string(),
        "15".to_string(),
    ])
    .expect("parse options");
    assert_eq!(
        options.chain_base_url.as_deref(),
        Some("http://127.0.0.1:5121")
    );
    assert_eq!(options.pricing_rules.len(), 1);
    assert_eq!(options.reconcile_interval_seconds, 15);
}

#[test]
fn reconcile_requires_chain_base_url_configuration() {
    let test_service = test_service("chain-config-missing", 900);
    issue_default_route(&test_service);

    let err = test_service
        .service
        .reconcile_once(6_000)
        .expect_err("missing chain config");
    assert_eq!(err.status_code, 503);
    assert_eq!(err.code, "chain_explorer_not_configured");
    assert!(err.message.contains("--chain-base-url"));
}

#[test]
fn reconcile_requires_credit_adapter_url_configuration() {
    let chain_server = MockChainServer::spawn();
    let test_service = test_service_with_endpoints(
        "credit-config-missing",
        900,
        Some(chain_server.base_url.clone()),
        None,
        1,
    );
    let deposit_account_id = issue_default_route(&test_service);
    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-credit-config".to_string(),
            action_id: 34,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: deposit_account_id,
            amount: 100,
            submitted_at_unix_ms: 5_000,
            updated_at_unix_ms: 5_100,
            block_height: Some(10),
        }],
    });

    let err = test_service
        .service
        .reconcile_once(6_000)
        .expect_err("missing credit adapter config");
    assert_eq!(err.status_code, 503);
    assert_eq!(err.code, "credit_adapter_not_configured");
    assert!(err.message.contains("--credit-adapter-url"));
}

#[test]
fn write_http_response_uses_upstream_status_texts() {
    assert_http_status_line(502, "HTTP/1.1 502 Bad Gateway\r\n");
    assert_http_status_line(503, "HTTP/1.1 503 Service Unavailable\r\n");
}

struct TestBridgeService {
    service: BridgeService,
    state_path: PathBuf,
}

fn test_service(name: &str, route_ttl_seconds: u64) -> TestBridgeService {
    test_service_with_endpoints(name, route_ttl_seconds, None, None, 1)
}

fn test_service_with_endpoints(
    name: &str,
    route_ttl_seconds: u64,
    chain_base_url: Option<String>,
    credit_adapter_url: Option<String>,
    chain_confirmations_required: u64,
) -> TestBridgeService {
    let state_path = temp_state_path(name);
    let store =
        Arc::new(BridgeStateStore::new(state_path.clone()).expect("create bridge state store"));
    TestBridgeService {
        service: BridgeService::new(
            store,
            BridgeServiceConfig {
                route_ttl_seconds,
                deposit_account_prefix: "oc:bridge:".to_string(),
                chain_base_url,
                chain_timeout_ms: 2_000,
                chain_confirmations_required,
                pricing_rules: vec![BridgePricingRuleConfig {
                    pricing_version: "pv-1".to_string(),
                    oc_amount: 100,
                    credit_units: 10,
                    bonus_units: 5,
                }],
                credit_adapter_url,
                credit_adapter_auth_token: None,
                credit_adapter_timeout_ms: 2_000,
                credit_target_type: CreditTargetType::Quota,
                max_credit_attempts: 3,
            },
        ),
        state_path,
    }
}

fn issue_default_route(test_service: &TestBridgeService) -> String {
    let binding = test_service
        .service
        .bind_user(
            BindBridgeUserRequest {
                newapi_user_ref: "user-1".to_string(),
                oasis_sender_account_id: "oc:pk:sender-1".to_string(),
            },
            1_000,
        )
        .expect("binding");
    test_service
        .service
        .create_deposit_route(
            CreateDepositRouteRequest {
                bridge_user_id: binding.bridge_user_id,
                pricing_version: Some("pv-1".to_string()),
                topup_plan_id: None,
            },
            2_000,
        )
        .expect("route")
        .deposit_account_id
}

fn temp_state_path(name: &str) -> PathBuf {
    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(
        "oasis7-newapi-bridge-{name}-{}-{}.json",
        process::id(),
        nonce
    );
    let path = env::temp_dir().join(file_name);
    let _ = fs::remove_file(path.as_path());
    path
}

#[derive(Debug, Clone)]
struct MockChainTx {
    tx_hash: String,
    action_id: u64,
    from_account_id: String,
    to_account_id: String,
    amount: u64,
    submitted_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    block_height: Option<u64>,
}

#[derive(Debug, Clone)]
struct MockChainState {
    committed_height: u64,
    txs: Vec<MockChainTx>,
}

impl Default for MockChainState {
    fn default() -> Self {
        Self {
            committed_height: 0,
            txs: Vec::new(),
        }
    }
}

struct MockChainServer {
    base_url: String,
    state: Arc<Mutex<MockChainState>>,
    stop: Arc<AtomicBool>,
}

impl MockChainServer {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock chain server");
        listener
            .set_nonblocking(true)
            .expect("set mock chain server nonblocking");
        let base_url = format!("http://{}", listener.local_addr().expect("mock chain addr"));
        let state = Arc::new(Mutex::new(MockChainState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&state);
        let stop_for_thread = Arc::clone(&stop);
        thread::spawn(move || loop {
            if stop_for_thread.load(Ordering::Relaxed) {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_http_request(&mut stream).expect("read mock chain request");
                    handle_mock_chain_request(&mut stream, &state_for_thread, request);
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("mock chain accept failed: {err}"),
            }
        });
        Self {
            base_url,
            state,
            stop,
        }
    }

    fn set_state(&self, state: MockChainState) {
        *self.state.lock().expect("mock chain state lock") = state;
    }

    fn set_committed_height(&self, height: u64) {
        self.state
            .lock()
            .expect("mock chain state lock")
            .committed_height = height;
    }
}

impl Drop for MockChainServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_mock_chain_request(
    stream: &mut TcpStream,
    state: &Arc<Mutex<MockChainState>>,
    request: HttpRequest,
) {
    let path = request
        .path
        .split('?')
        .next()
        .unwrap_or(request.path.as_str());
    let payload = match path {
        "/v1/chain/explorer/overview" => {
            let snapshot = state.lock().expect("mock chain state lock").clone();
            json!({
                "ok": true,
                "committed_height": snapshot.committed_height,
            })
        }
        "/v1/chain/explorer/txs" => {
            let account_id = query_param(request.path.as_str(), "account_id").unwrap_or_default();
            let snapshot = state.lock().expect("mock chain state lock").clone();
            let items = snapshot
                .txs
                .into_iter()
                .filter(|tx| tx.from_account_id == account_id || tx.to_account_id == account_id)
                .map(|tx| {
                    json!({
                        "tx_hash": tx.tx_hash,
                        "action_id": tx.action_id,
                        "from_account_id": tx.from_account_id,
                        "to_account_id": tx.to_account_id,
                        "amount": tx.amount,
                        "submitted_at_unix_ms": tx.submitted_at_unix_ms,
                        "updated_at_unix_ms": tx.updated_at_unix_ms,
                        "block_height": tx.block_height,
                    })
                })
                .collect::<Vec<_>>();
            json!({ "ok": true, "items": items })
        }
        other => json!({
            "ok": false,
            "error_code": "not_found",
            "error": format!("unknown mock chain path {other}"),
        }),
    };
    let body = serde_json::to_vec(&payload).expect("encode mock chain response");
    write_http_response(stream, 200, "application/json", body.as_slice(), false)
        .expect("write mock chain response");
}

#[derive(Default)]
struct MockCreditState {
    fail_remaining: usize,
    requests: Vec<Value>,
}

struct MockCreditServer {
    base_url: String,
    state: Arc<Mutex<MockCreditState>>,
    stop: Arc<AtomicBool>,
}

impl MockCreditServer {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock credit server");
        listener
            .set_nonblocking(true)
            .expect("set mock credit server nonblocking");
        let base_url = format!(
            "http://{}/v1/admin/credits/apply",
            listener.local_addr().expect("mock credit addr")
        );
        let state = Arc::new(Mutex::new(MockCreditState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let state_for_thread = Arc::clone(&state);
        let stop_for_thread = Arc::clone(&stop);
        thread::spawn(move || loop {
            if stop_for_thread.load(Ordering::Relaxed) {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_http_request(&mut stream).expect("read mock credit request");
                    handle_mock_credit_request(&mut stream, &state_for_thread, request);
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("mock credit accept failed: {err}"),
            }
        });
        Self {
            base_url,
            state,
            stop,
        }
    }

    fn fail_first_requests(&self, count: usize) {
        self.state
            .lock()
            .expect("mock credit state lock")
            .fail_remaining = count;
    }

    fn recorded_requests(&self) -> Vec<Value> {
        self.state
            .lock()
            .expect("mock credit state lock")
            .requests
            .clone()
    }
}

impl Drop for MockCreditServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_mock_credit_request(
    stream: &mut TcpStream,
    state: &Arc<Mutex<MockCreditState>>,
    request: HttpRequest,
) {
    let payload: Value =
        serde_json::from_slice(request.body.as_slice()).expect("decode credit body");
    let mut guard = state.lock().expect("mock credit state lock");
    guard.requests.push(payload.clone());
    if guard.fail_remaining > 0 {
        guard.fail_remaining -= 1;
        let body = serde_json::to_vec(&json!({
            "ok": false,
            "error": "temporary upstream failure"
        }))
        .expect("encode credit failure");
        write_http_response(stream, 502, "application/json", body.as_slice(), false)
            .expect("write credit failure");
        return;
    }
    let body = serde_json::to_vec(&json!({
        "ok": true,
        "applied": true,
        "echo": payload,
    }))
    .expect("encode credit success");
    write_http_response(stream, 200, "application/json", body.as_slice(), false)
        .expect("write credit success");
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let url = reqwest::Url::parse(format!("http://127.0.0.1{path}").as_str()).ok()?;
    for (name, value) in url.query_pairs() {
        if name == key {
            return Some(value.into_owned());
        }
    }
    None
}

fn assert_http_status_line(status_code: u16, expected_prefix: &str) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind status line listener");
    let addr = listener.local_addr().expect("status line listener addr");
    let expected_body = format!("status-{status_code}");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status line connection");
        write_http_response(
            &mut stream,
            status_code,
            "text/plain; charset=utf-8",
            expected_body.as_bytes(),
            false,
        )
        .expect("write status line response");
    });

    let mut client = TcpStream::connect(addr).expect("connect status line client");
    let mut raw_response = String::new();
    client
        .read_to_string(&mut raw_response)
        .expect("read status line response");
    server.join().expect("join status line thread");
    assert!(raw_response.starts_with(expected_prefix));
}
