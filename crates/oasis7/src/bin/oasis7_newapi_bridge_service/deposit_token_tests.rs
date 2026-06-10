use super::*;

#[test]
fn create_deposit_route_persists_and_reuses_active_route() {
    let test_service = test_service("route-reuse", 900);
    let binding = bind_default_user(&test_service);

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
    assert_eq!(
        first.deposit_token.as_deref(),
        Some("bridge:deposit:route-000001")
    );

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
    assert_eq!(reused.deposit_token, first.deposit_token);

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.routes.len(), 1);
    assert_eq!(
        snapshot.routes[0].deposit_token.as_deref(),
        Some("bridge:deposit:route-000001")
    );
}

#[test]
fn reconcile_marks_missing_deposit_token_for_manual_review() {
    let chain_server = MockChainServer::spawn();
    let letai_server = MockLetaiServer::spawn();
    let test_service = test_service_with_endpoints(
        "reconcile-missing-deposit-token",
        900,
        Some(chain_server.base_url.clone()),
        Some(letai_server.base_url.clone()),
        1,
        None,
    );
    let deposit_account_id = issue_default_route(&test_service);

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-missing-token".to_string(),
            action_id: 18,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: deposit_account_id,
            amount: 100,
            memo: None,
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
    assert_eq!(reconcile.reconciled_credit_count, 0);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::ManualReview);
    assert_eq!(
        snapshot.ledger[0].review_reason.as_deref(),
        Some("missing_deposit_token")
    );
    assert_eq!(
        snapshot.ledger[0].expected_deposit_token.as_deref(),
        Some("bridge:deposit:route-000001")
    );
    assert_eq!(snapshot.ledger[0].observed_deposit_token, None);
    assert!(letai_server.recorded_topup_requests().is_empty());
}

#[test]
fn reconcile_marks_deposit_token_mismatch_for_manual_review() {
    let chain_server = MockChainServer::spawn();
    let letai_server = MockLetaiServer::spawn();
    let test_service = test_service_with_endpoints(
        "reconcile-token-mismatch",
        900,
        Some(chain_server.base_url.clone()),
        Some(letai_server.base_url.clone()),
        1,
        None,
    );
    let deposit_account_id = issue_default_route(&test_service);

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-token-mismatch".to_string(),
            action_id: 19,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: deposit_account_id,
            amount: 100,
            memo: Some("bridge:deposit:wrong-route".to_string()),
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
    assert_eq!(reconcile.reconciled_credit_count, 0);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::ManualReview);
    assert_eq!(
        snapshot.ledger[0].review_reason.as_deref(),
        Some("deposit_token_mismatch")
    );
    assert_eq!(
        snapshot.ledger[0].observed_deposit_token.as_deref(),
        Some("bridge:deposit:wrong-route")
    );
    assert!(letai_server.recorded_topup_requests().is_empty());
}

#[test]
fn reconcile_accepts_legacy_route_without_deposit_token() {
    let chain_server = MockChainServer::spawn();
    let letai_server = MockLetaiServer::spawn();
    let test_service = test_service_with_endpoints(
        "reconcile-legacy-token-none",
        900,
        Some(chain_server.base_url.clone()),
        Some(letai_server.base_url.clone()),
        1,
        None,
    );
    let deposit_account_id = issue_default_route(&test_service);
    test_service
        .service
        .store_mutate_test(|state| {
            state.routes[0].deposit_token = None;
            Ok(())
        })
        .expect("clear legacy route token");

    chain_server.set_state(MockChainState {
        committed_height: 10,
        txs: vec![MockChainTx {
            tx_hash: "tx-legacy-token-none".to_string(),
            action_id: 20,
            from_account_id: "oc:pk:sender-1".to_string(),
            to_account_id: deposit_account_id,
            amount: 100,
            memo: None,
            submitted_at_unix_ms: 5_000,
            updated_at_unix_ms: 5_100,
            block_height: Some(10),
        }],
    });

    let reconcile = test_service
        .service
        .reconcile_once(6_000)
        .expect("reconcile");
    assert_eq!(reconcile.manual_review_count, 0);
    assert_eq!(reconcile.reconciled_credit_count, 1);
    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.ledger[0].state, BridgeLedgerState::Reconciled);
    assert_eq!(snapshot.ledger[0].expected_deposit_token, None);
    assert_eq!(snapshot.ledger[0].observed_deposit_token, None);
    assert_eq!(letai_server.recorded_topup_requests().len(), 1);
}

#[test]
fn create_deposit_route_expires_old_route_before_reissuing() {
    let test_service = test_service("route-expire", 1);
    let binding = bind_default_user(&test_service);

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
    assert_ne!(second.deposit_token, first.deposit_token);
    assert_eq!(
        second.deposit_token.as_deref(),
        Some("bridge:deposit:route-000002")
    );

    let snapshot = test_service.service.snapshot();
    assert_eq!(snapshot.routes.len(), 2);
    assert_eq!(snapshot.routes[0].status, DepositRouteStatus::Expired);
    assert_eq!(snapshot.routes[1].status, DepositRouteStatus::Issued);
}
