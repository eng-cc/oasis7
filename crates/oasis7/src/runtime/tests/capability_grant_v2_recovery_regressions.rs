//! Recovery regression coverage for pending capability contexts.

use super::super::*;
use super::capability_grant_v2::*;
use serde_json::json;

#[test]
fn capability_recovery_requires_commit_for_new_context_after_historical_receipt() {
    let mut world = fixture_world();
    let historical_grant = signed_grant(grant_json(json!({
        "grant_nonce": "historical-receipt-grant"
    })));
    install_budget_for_grant(&mut world, &historical_grant, 128);
    let (historical_catalog, historical_response) = prepared_invocation(
        &world,
        &historical_grant,
        catalog_json(json!({})),
        response_json(json!({
            "response_nonce": "response-history"
        })),
    );
    install_invocation_context(
        &mut world,
        &historical_grant,
        &historical_catalog,
        &historical_response,
    );
    execute_without_invocation_context(
        &mut world,
        historical_grant,
        historical_catalog,
        historical_response,
        &mut RecordingSandbox::default(),
    )
    .expect("historical command should leave a durable receipt");

    let crashed_grant = signed_grant(grant_json(json!({
        "grant_nonce": "crashed-command-grant"
    })));
    install_budget_for_grant(&mut world, &crashed_grant, 128);
    let (crashed_catalog, crashed_response) = prepared_invocation(
        &world,
        &crashed_grant,
        catalog_json(json!({})),
        response_json(json!({
            "response_nonce": "response-crashed"
        })),
    );
    install_invocation_context(
        &mut world,
        &crashed_grant,
        &crashed_catalog,
        &crashed_response,
    );
    let stale_snapshot = world.snapshot();
    assert!(!stale_snapshot.capability_authorization_receipts.is_empty());
    assert!(
        stale_snapshot
            .capability_invocation_contexts
            .values()
            .any(|context| { context.response_nonce == "response-crashed" })
    );

    let mut journal = world.journal().clone();
    execute_without_invocation_context(
        &mut world,
        crashed_grant,
        crashed_catalog,
        crashed_response,
        &mut RecordingSandbox::default(),
    )
    .expect("the simulated command should produce a pre-commit journal tail");
    let command_commit = world.journal().events.last().expect("command commit event");
    assert!(matches!(
        command_commit.body,
        WorldEventBody::CapabilityAuthorization(
            CapabilityAuthorizationEvent::CommandCommitted { .. }
        )
    ));
    journal.events.extend(
        world.journal().events[stale_snapshot.journal_len..world.journal().events.len() - 1]
            .iter()
            .cloned(),
    );

    let error = World::from_snapshot(stale_snapshot, journal)
        .expect_err("a new pending context without its command commit must fail closed");
    assert!(matches!(error, WorldError::JournalMismatch));
}

#[test]
fn capability_recovery_requires_a_commit_for_each_pending_context() {
    let mut world = fixture_world();
    let first_grant = signed_grant(grant_json(json!({
        "grant_nonce": "pending-context-one"
    })));
    let second_grant = signed_grant(grant_json(json!({
        "grant_nonce": "pending-context-two"
    })));
    install_budget_for_grant(&mut world, &first_grant, 128);
    install_budget_for_grant(&mut world, &second_grant, 128);

    let base_head = world
        .journal()
        .events
        .last()
        .map(|event| event.id)
        .unwrap_or(0);
    let expected_context_head = base_head.saturating_add(2);
    let mut first_catalog = prepared_catalog(&world, &first_grant, catalog_json(json!({})));
    first_catalog.world_head = expected_context_head;
    first_catalog.snapshot_id = first_catalog
        .canonical_hash()
        .expect("compute first pending context catalog hash");
    let first_response = prepared_response(
        response_json(json!({
            "response_nonce": "response-pending-one"
        })),
        &first_catalog,
    );
    let mut second_catalog = prepared_catalog(&world, &second_grant, catalog_json(json!({})));
    second_catalog.world_head = expected_context_head;
    second_catalog.snapshot_id = second_catalog
        .canonical_hash()
        .expect("compute second pending context catalog hash");
    let second_response = prepared_response(
        response_json(json!({
            "response_nonce": "response-pending-two"
        })),
        &second_catalog,
    );
    install_invocation_context(&mut world, &first_grant, &first_catalog, &first_response);
    install_invocation_context(&mut world, &second_grant, &second_catalog, &second_response);
    let stale_snapshot = world.snapshot();

    let mut sandbox = RecordingSandbox::default();
    execute_without_invocation_context(
        &mut world,
        first_grant,
        first_catalog,
        first_response,
        &mut sandbox,
    )
    .expect("the first pending context should commit");
    let mut journal = world.journal().clone();
    let next_event_id = journal
        .events
        .last()
        .map(|event| event.id.saturating_add(1))
        .expect("first command has a journal tail");
    journal.append(WorldEvent {
        id: next_event_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::ModuleStateUpdated(oasis7_wasm_abi::ModuleStateUpdate {
            module_id: MODULE_ID.to_string(),
            trace_id: "trusted-command-response-pending-two".to_string(),
            state: vec![0x78],
        }),
    });
    assert!(journal.events.iter().any(|event| matches!(
        &event.body,
        WorldEventBody::CapabilityAuthorization(
            CapabilityAuthorizationEvent::CommandCommitted { receipt, .. }
        ) if receipt.response_nonce.as_deref() == Some("response-pending-one")
    )));
    assert!(journal.events.iter().any(|event| matches!(
        &event.body,
        WorldEventBody::ModuleStateUpdated(update)
            if update.trace_id == "trusted-command-response-pending-two"
    )));

    let error = World::from_snapshot(stale_snapshot, journal)
        .expect_err("one matching commit must not discharge another pending context");
    assert!(matches!(error, WorldError::JournalMismatch));
}
