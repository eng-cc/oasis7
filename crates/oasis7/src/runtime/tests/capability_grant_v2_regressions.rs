//! Regression coverage for persisted capability authorization and replay.

use super::super::*;
use super::capability_grant_v2::*;
use oasis7_wasm_abi::{ModuleCallCaller, ModuleCallInput, ModuleEffectIntent, ModuleOutput};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn authorization_nonce_same_hash_is_idempotent_but_different_hash_conflicts_without_effect() {
    let mut world = fixture_world();
    let grant = signed_grant(grant_json(json!({})));
    let (catalog, response) = prepared_invocation(
        &world,
        &grant,
        catalog_json(json!({})),
        response_json(json!({})),
    );
    install_invocation_context(&mut world, &grant, &catalog, &response);
    let mut sandbox = RecordingSandbox::default();
    let first = execute_without_invocation_context(
        &mut world,
        grant.clone(),
        catalog.clone(),
        response.clone(),
        &mut sandbox,
    )
    .expect("first exact request");
    let second = execute_without_invocation_context(
        &mut world,
        grant.clone(),
        catalog.clone(),
        response.clone(),
        &mut sandbox,
    )
    .expect("same request is idempotent");
    assert_eq!(first.receipt_id, second.receipt_id);
    assert_eq!(second.decision, "idempotent");
    assert_eq!(sandbox.calls, 1);

    let mut conflict_response = response;
    conflict_response.envelope.payload = vec![123, 34, 120, 34, 125];
    let error = execute_without_invocation_context(
        &mut world,
        grant,
        catalog,
        conflict_response,
        &mut sandbox,
    )
    .expect_err("same nonce with a different request hash must conflict");
    assert!(matches!(error, WorldError::CapabilityNonceConflict { .. }));
    assert_eq!(sandbox.calls, 1);
}

#[test]
#[allow(deprecated)]
fn capability_authority_install_rejects_unverified_finality_metadata() {
    let mut world = World::new();
    let signing_key = capability_issuer_signing_key();
    let record = CapabilityAuthorityRecord {
        issuer_id: ISSUER_ID.to_string(),
        issuer_kind: "governance".to_string(),
        key_id: "governance-key-1".to_string(),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
        issuer_key_epoch: 3,
        governance_epoch: 9,
        finalized_receipt_id: "unverified-receipt".to_string(),
        authority_rotation_receipt_id: None,
        world_id: WORLD_ID.to_string(),
        branch_id: BRANCH_ID.to_string(),
        finality_epoch: 4,
        finality_block_hash: "forged-finality-block".to_string(),
        finality_status: "finalized".to_string(),
        revocation_epoch: 2,
        revoked_grant_ids: BTreeSet::new(),
        superseded_by: BTreeMap::new(),
    };

    let error = world
        .install_capability_authority_record(record)
        .expect_err("caller-supplied finalized metadata must not create a trust root");
    assert!(matches!(
        error,
        WorldError::CapabilityAuthorizationDenied { .. }
    ));
    assert!(
        world
            .capability_revocation_state()
            .authority_records
            .is_empty()
    );
}

#[test]
fn capability_invocation_context_allows_two_distinct_nonces_for_one_grant() {
    let mut world = fixture_world();
    let grant = signed_grant(grant_json(json!({"issued_at_tick": 0})));
    let catalog = prepared_catalog(&world, &grant, catalog_json(json!({})));
    let response = prepared_response(response_json(json!({})), &catalog);
    let mut second_response = response.clone();
    second_response.response_nonce = "response-2".to_string();

    install_invocation_context(&mut world, &grant, &catalog, &response);
    install_invocation_context(&mut world, &grant, &catalog, &second_response);

    assert_eq!(world.capability_invocation_contexts().len(), 2);
    assert!(
        world
            .capability_invocation_contexts()
            .values()
            .any(|context| context.response_nonce == "response-1")
    );
    assert!(
        world
            .capability_invocation_contexts()
            .values()
            .any(|context| context.response_nonce == "response-2")
    );
}

#[test]
fn trusted_executor_replays_stale_snapshot_authz_tail_exactly_once() {
    let mut world = fixture_world();
    let grant = signed_grant(grant_json(json!({
        "issued_at_tick": 0,
        "scope": {
            "entity_selector": null,
            "resource_selector": null
        }
    })));
    install_budget_for_grant(&mut world, &grant, 128);
    let (catalog, response) = prepared_invocation(
        &world,
        &grant,
        catalog_json(json!({})),
        response_json(json!({
            "subject": {
                "kind": "agent",
                "agent_id": SUBJECT_ID,
                "owner_binding": "owner-7",
                "generation": 1
            }
        })),
    );
    install_invocation_context(&mut world, &grant, &catalog, &response);
    let stale_snapshot = world.snapshot();
    let mut sandbox = RecordingSandbox::default();
    let first = execute_without_invocation_context(
        &mut world,
        grant.clone(),
        catalog.clone(),
        response.clone(),
        &mut sandbox,
    )
    .expect("first command should commit before the simulated crash");
    let committed_journal = world.journal().clone();

    let mut restored = World::from_snapshot(stale_snapshot, committed_journal)
        .expect("stale snapshot plus journal tail should recover");
    assert_eq!(
        restored.capability_grants_v2().get(&grant.grant_id),
        world.capability_grants_v2().get(&grant.grant_id)
    );
    assert_eq!(
        restored.capability_nonce_records(),
        world.capability_nonce_records()
    );
    assert_eq!(
        restored.capability_authorization_receipts(),
        world.capability_authorization_receipts()
    );
    assert_eq!(
        restored.capability_budget_accounts(),
        world.capability_budget_accounts()
    );

    let mut retry_sandbox = RecordingSandbox::default();
    let retry = execute_without_invocation_context(
        &mut restored,
        grant,
        catalog,
        response,
        &mut retry_sandbox,
    )
    .expect("replayed command must be idempotent, not re-executed");
    assert_eq!(retry.decision, "idempotent");
    assert_eq!(retry.receipt_id, first.receipt_id);
    assert_eq!(retry_sandbox.calls, 0);
}

#[test]
fn trusted_executor_replays_effect_receipt_ack_after_closure_window_exactly_once() {
    let mut world = fixture_world();
    let grant = signed_grant(grant_json(json!({})));
    let (catalog, response) = prepared_invocation(
        &world,
        &grant,
        catalog_json(json!({})),
        response_json(json!({})),
    );
    install_invocation_context(&mut world, &grant, &catalog, &response);
    let effect_grant = signed_effect_grant();
    let mut sandbox = ConfiguredSandbox {
        calls: 0,
        output: ModuleOutput {
            new_state: Some(vec![0x99]),
            effects: vec![ModuleEffectIntent {
                kind: "weather.publish".to_string(),
                params: json!({"station": "station-1"}),
                cap_ref: effect_grant.grant_id.clone(),
                cap_slot: None,
            }],
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 64,
        },
    };

    let authorization =
        execute_without_invocation_context(&mut world, grant, catalog, response, &mut sandbox)
            .expect("effect-producing command should commit before the receipt window");
    let intent = world
        .take_next_effect()
        .expect("committed effect should be dispatched before receipt acknowledgement");
    let stale_snapshot = world.snapshot();
    let journal_before_receipt = world.journal().clone();
    let receipt = EffectReceipt {
        intent_id: intent.intent_id.clone(),
        status: "ok".to_string(),
        payload: json!({"published": true}),
        cost_cents: Some(1),
        signature: None,
    };

    world
        .ingest_receipt(receipt.clone())
        .expect("live receipt ingestion should close authorization and acknowledge effect");
    let mut closure_only_journal = world.journal().clone();
    let acknowledgement = closure_only_journal
        .events
        .pop()
        .expect("receipt acknowledgement event should be journaled last");
    assert!(matches!(
        acknowledgement.body,
        WorldEventBody::ReceiptAppended(_)
    ));
    assert_eq!(
        closure_only_journal.events.len(),
        journal_before_receipt.events.len() + 1,
        "the simulated crash preserves the authorization closure but not receipt acknowledgement"
    );
    assert!(closure_only_journal.events.iter().any(|event| matches!(
        &event.body,
        WorldEventBody::CapabilityAuthorization(
            CapabilityAuthorizationEvent::EffectReceiptCommitted { intent_id, .. }
        ) if intent_id == &intent.intent_id
    )));

    let mut recovered = World::from_snapshot(stale_snapshot, closure_only_journal)
        .expect("recovery should replay the durable authorization closure");
    let recovered_audit = recovered
        .capability_authorization_receipts()
        .get(&authorization.receipt_id)
        .expect("authorization audit receipt survives the crash window");
    assert_eq!(
        recovered_audit.committed_effect_receipt_id.as_deref(),
        Some(intent.intent_id.as_str())
    );
    assert!(
        !recovered
            .capability_effect_receipt_links()
            .contains_key(&intent.intent_id)
    );

    recovered
        .ingest_receipt(receipt)
        .expect("receipt acknowledgement should remain retryable after closure replay");
    assert_eq!(
        recovered
            .journal()
            .events
            .iter()
            .filter(|event| matches!(
                &event.body,
                WorldEventBody::CapabilityAuthorization(
                    CapabilityAuthorizationEvent::EffectReceiptCommitted { intent_id, .. }
                ) if intent_id == &intent.intent_id
            ))
            .count(),
        1,
        "receipt retry must not duplicate authorization closure"
    );
    assert_eq!(
        recovered
            .journal()
            .events
            .iter()
            .filter(|event| matches!(
                &event.body,
                WorldEventBody::ReceiptAppended(receipt) if receipt.intent_id == intent.intent_id
            ))
            .count(),
        1,
        "receipt acknowledgement is appended exactly once"
    );
    assert!(
        !recovered
            .capability_effect_receipt_links()
            .contains_key(&intent.intent_id)
    );
}

#[test]
fn trusted_executor_rejects_future_issued_grant_before_sandbox() {
    let mut world = fixture_world();
    let future_tick = world.state().time.saturating_add(1);
    let grant = signed_grant(grant_json(json!({
        "issued_at_tick": future_tick
    })));
    install_budget_for_grant(&mut world, &grant, 128);
    let (catalog, response) = prepared_invocation(
        &world,
        &grant,
        catalog_json(json!({})),
        response_json(json!({})),
    );
    install_invocation_context(&mut world, &grant, &catalog, &response);
    let mut sandbox = RecordingSandbox::default();

    let error =
        execute_without_invocation_context(&mut world, grant, catalog, response, &mut sandbox)
            .expect_err("future-issued grant must fail closed");
    assert!(matches!(
        error,
        WorldError::CapabilityAuthorizationDenied { .. }
    ));
    assert_eq!(sandbox.calls, 0);
}

#[test]
fn trusted_executor_rejects_command_selector_target_mismatch_before_sandbox() {
    let cases = [
        (
            "entity target mismatch",
            json!({
                "entity_id": "station-2",
                "resource_id": "weather.read"
            }),
        ),
        (
            "resource target mismatch",
            json!({
                "entity_id": "station-1",
                "resource_id": "weather.write"
            }),
        ),
    ];
    for (label, payload) in cases {
        let mut world = fixture_world();
        let grant = signed_grant(grant_json(json!({
            "issued_at_tick": 0,
            "scope": {
                "entity_selector": ["station-1"],
                "resource_selector": ["weather.read"]
            }
        })));
        install_budget_for_grant(&mut world, &grant, 128);
        let payload = serde_json::to_vec(&payload).expect("encode selector target payload");
        let (catalog, response) = prepared_invocation(
            &world,
            &grant,
            catalog_json(json!({})),
            response_json(json!({
                "envelope": {"payload": payload}
            })),
        );
        install_invocation_context(&mut world, &grant, &catalog, &response);
        let mut sandbox = RecordingSandbox::default();

        let error =
            execute_without_invocation_context(&mut world, grant, catalog, response, &mut sandbox)
                .expect_err(label);
        assert!(matches!(
            error,
            WorldError::CapabilityAuthorizationDenied { .. }
        ));
        assert_eq!(sandbox.calls, 0, "{label} must fail before sandbox");
    }
}

#[test]
fn trusted_executor_rejects_effect_selector_target_mismatch_before_commit() {
    let effect_grant = signed_effect_grant_with_selectors();
    let mut world = fixture_world_with_revocations_and_budget_and_effect_grant(
        BTreeSet::new(),
        128,
        effect_grant.clone(),
    );
    let grant = signed_grant(grant_json(json!({})));
    let (catalog, response) = prepared_invocation(
        &world,
        &grant,
        catalog_json(json!({})),
        response_json(json!({})),
    );
    install_invocation_context(&mut world, &grant, &catalog, &response);
    let mut sandbox = ConfiguredSandbox {
        calls: 0,
        output: ModuleOutput {
            new_state: Some(vec![0xee]),
            effects: vec![ModuleEffectIntent {
                kind: "weather.publish".to_string(),
                params: json!({
                    "entity_id": "station-2",
                    "resource_id": "weather.read"
                }),
                cap_ref: effect_grant.grant_id,
                cap_slot: None,
            }],
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 0,
        },
    };

    let error =
        execute_without_invocation_context(&mut world, grant, catalog, response, &mut sandbox)
            .expect_err("effect target outside the selector must fail closed");
    assert!(matches!(
        error,
        WorldError::CapabilityAuthorizationDenied { .. }
    ));
    assert_eq!(world.pending_effects_len(), 0);
    assert!(world.capability_nonce_records().is_empty());
    assert!(world.capability_authorization_receipts().is_empty());
}

#[test]
fn trusted_executor_preserves_module_and_system_subject_provenance() {
    let cases = [
        (
            json!({
                "kind": "module",
                "module_id": MODULE_ID,
                "module_version": MODULE_VERSION,
                "instance_id": "weather-instance-1"
            }),
            ModuleCallCaller::Module {
                module_id: MODULE_ID.to_string(),
            },
        ),
        (
            json!({
                "kind": "system",
                "system_id": "system-weather",
                "epoch": 1
            }),
            ModuleCallCaller::System {
                system_id: "system-weather".to_string(),
            },
        ),
    ];
    for (subject, expected_caller) in cases {
        let mut world = fixture_world();
        let grant = signed_grant(grant_json(json!({
            "issued_at_tick": 0,
            "subject": subject,
            "scope": {
                "entity_selector": null,
                "resource_selector": null
            }
        })));
        install_budget_for_grant(&mut world, &grant, 128);
        let subject = serde_json::to_value(&grant.subject).expect("encode capability subject");
        let (catalog, response) = prepared_invocation(
            &world,
            &grant,
            catalog_json(json!({"subject": subject.clone()})),
            response_json(json!({"subject": subject})),
        );
        install_invocation_context(&mut world, &grant, &catalog, &response);
        let mut sandbox = ProvenanceSandbox::default();

        execute_without_invocation_context(&mut world, grant, catalog, response, &mut sandbox)
            .expect("subject-specific command should execute");
        assert_eq!(sandbox.requests.len(), 1);
        let input: ModuleCallInput = serde_cbor::from_slice(&sandbox.requests[0].input)
            .expect("decode trusted module call input");
        assert_eq!(input.ctx.caller, expected_caller);
    }
}
