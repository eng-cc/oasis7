use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::Value;

use super::api::HttpRequest;
use super::model::{
    BindBridgeUserRequest, BridgeBindingStatus, CreateDepositRouteRequest, DepositRouteStatus,
};
use super::service::{BridgeService, BridgeServiceConfig};
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
    let test_service = test_service("http-contract", 900);
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
}

#[test]
fn parse_cli_options_rejects_zero_route_ttl() {
    let err = parse_cli_options(vec!["--route-ttl-seconds".to_string(), "0".to_string()])
        .expect_err("ttl validation");
    assert!(err.contains("greater than 0"));
}

struct TestBridgeService {
    service: BridgeService,
    state_path: PathBuf,
}

fn test_service(name: &str, route_ttl_seconds: u64) -> TestBridgeService {
    let state_path = temp_state_path(name);
    let store =
        Arc::new(BridgeStateStore::new(state_path.clone()).expect("create bridge state store"));
    TestBridgeService {
        service: BridgeService::new(
            store,
            BridgeServiceConfig {
                route_ttl_seconds,
                deposit_account_prefix: "oc:bridge:".to_string(),
            },
        ),
        state_path,
    }
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
