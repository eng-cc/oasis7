//! Runtime cognition economy transition, idempotency and crash-prefix tests.

use super::super::*;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn quote(id: &str, resource: &str, amount: u64) -> CognitionLeaseQuoteV1 {
    CognitionLeaseQuoteV1::new(id, resource, amount)
}

fn request(key: &str, amount: u64) -> CognitionLeaseRequestV1 {
    CognitionLeaseRequestV1::new(
        key,
        "account-agent-a",
        "agent-a",
        "session-a",
        "turn-a",
        "request-a",
        "request-digest-a",
        quote("quote-a", "cognition_units", amount),
    )
}

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-cognition-economy-{prefix}-{nanos}"))
}

fn economy_digest_for_test<T: Serialize>(domain: &str, payload: &T) -> String {
    let bytes = oasis7_wasm_abi::encode_canonical_cbor(&(domain, payload))
        .expect("economy test payload is canonically encodable");
    format!("blake3:{}", blake3::hash(&bytes))
}

fn refresh_event_digest_for_test(event: &mut CognitionEconomyEventV1) {
    let mut value = serde_json::to_value(&*event).expect("economy event serializes");
    value
        .as_object_mut()
        .expect("economy event is an object")
        .remove("event_digest");
    event.event_digest = economy_digest_for_test("oasis7.cognition.economy.event.v1", &value);
}

fn refresh_head_digest_for_test(economy: &mut CognitionEconomyStateV1) {
    economy.head_digest = economy_digest_for_test(
        "oasis7.cognition.economy.journal-head.v1",
        &(economy.head_seq, &economy.journal),
    );
}

#[test]
fn reserve_is_runtime_authoritative_and_idempotent() {
    let mut economy = CognitionEconomyStateV1::new();
    economy
        .set_resource_balance("account-agent-a", "cognition_units", 10)
        .expect("seed explicit balance");
    let first = economy
        .reserve(request("stable-key", 7), 4)
        .expect("reserve");
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        3
    );
    assert_eq!(
        economy.reserved_balance("account-agent-a", "cognition_units"),
        7
    );
    assert_eq!(economy.journal.len(), 1);

    let replay = economy
        .reserve(request("stable-key", 7), 5)
        .expect("replay exact reserve");
    assert_eq!(replay, first);
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        3
    );
    assert_eq!(
        economy.reserved_balance("account-agent-a", "cognition_units"),
        7
    );
    assert_eq!(economy.journal.len(), 1, "replay must not append a debit");

    let conflicting = economy.reserve(request("stable-key", 6), 5);
    assert_eq!(
        conflicting
            .expect_err("same key with changed quote must conflict")
            .code(),
        "cognition_idempotency_conflict"
    );
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        3
    );
    assert_eq!(
        economy.reserved_balance("account-agent-a", "cognition_units"),
        7
    );
}

#[test]
fn settlement_release_and_refund_preserve_conservation_and_terminality() {
    let mut economy = CognitionEconomyStateV1::new();
    economy
        .set_resource_balance("account-agent-a", "cognition_units", 20)
        .expect("seed explicit balance");
    let lease = economy
        .reserve(request("settle-key", 8), 1)
        .expect("reserve");
    let settlement = economy.settle(&lease.lease_id, 5, 2).expect("settle");
    assert_eq!(settlement.status, CognitionLeaseStatusV1::Settled);
    assert_eq!(settlement.consumed_amount, 5);
    assert_eq!(settlement.refunded_amount, 3);
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        15
    );
    assert_eq!(
        economy.reserved_balance("account-agent-a", "cognition_units"),
        0
    );

    let settlement_replay = economy
        .settle(&lease.lease_id, 5, 99)
        .expect("settle replay");
    assert_eq!(settlement_replay, settlement);
    let changed_settlement = economy.settle(&lease.lease_id, 4, 99);
    assert_eq!(
        changed_settlement
            .expect_err("changed settlement retry must conflict")
            .code(),
        "cognition_settlement_idempotency_conflict"
    );
    let refund = economy
        .refund(&lease.lease_id, 3)
        .expect("refund settled lease");
    assert_eq!(refund.status, CognitionLeaseStatusV1::Refunded);
    assert_eq!(refund.refunded_amount, 5);
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        20
    );
    assert_eq!(
        economy.reserved_balance("account-agent-a", "cognition_units"),
        0
    );
    assert_eq!(
        economy.refund(&lease.lease_id, 4).expect("refund replay"),
        refund
    );

    let released = economy
        .reserve(request("release-key", 6), 5)
        .expect("second reserve");
    let release = economy.release(&released.lease_id, 6).expect("release");
    assert_eq!(release.status, CognitionLeaseStatusV1::Released);
    assert_eq!(release.refunded_amount, 6);
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        20
    );
    assert_eq!(
        economy
            .release(&released.lease_id, 7)
            .expect("release replay"),
        release
    );
}

#[test]
fn quote_digest_and_expiry_are_immutable_runtime_inputs() {
    let mut forged = quote("quote-forged", "cognition_units", 2);
    forged.amount = 3;
    assert_eq!(
        forged
            .validate()
            .expect_err("mutated quote must fail digest validation")
            .code(),
        "cognition_quote_invalid"
    );

    let expired = request("expired-key", 1);
    let expired = CognitionLeaseRequestV1 {
        quote: quote("expired", "cognition_units", 1).with_valid_until_tick(2),
        ..expired
    };
    let mut economy = CognitionEconomyStateV1::new();
    economy
        .set_resource_balance("account-agent-a", "cognition_units", 5)
        .expect("seed balance");
    assert_eq!(
        economy
            .reserve(expired, 3)
            .expect_err("expired quote must not reserve")
            .code(),
        "cognition_quote_expired"
    );
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        5
    );
    assert_eq!(economy.journal.len(), 0);
}

#[test]
fn boundary_amount_validation_returns_error_without_panicking() {
    let amount = u64::MAX;
    let request = request("max-amount-key", amount);
    let lease = CognitionLeaseV1 {
        schema_version: COGNITION_LEASE_SCHEMA_VERSION.to_string(),
        lease_id: request.derived_lease_id(),
        idempotency_key: request.idempotency_key.clone(),
        account_id: request.account_id.clone(),
        agent_id: request.agent_id.clone(),
        agent_session_id: request.agent_session_id.clone(),
        agent_turn_id: request.agent_turn_id.clone(),
        decision_request_id: request.decision_request_id.clone(),
        request_digest: request.request_digest.clone(),
        quote: request.quote.clone(),
        reserved_amount: amount,
        settled_amount: amount,
        refunded_amount: amount,
        status: CognitionLeaseStatusV1::Settled,
        reserved_at_tick: 1,
        closed_at_tick: Some(2),
        receipt_id: Some(format!("blake3:{}", "0".repeat(64))),
    };
    let lease_result = std::panic::catch_unwind(|| lease.validate());
    assert!(
        lease_result.is_ok(),
        "lease boundary validation must return an error instead of panicking"
    );
    assert!(lease_result.unwrap().is_err());

    let mut receipt = CognitionReceiptV1 {
        schema_version: COGNITION_RECEIPT_SCHEMA_VERSION.to_string(),
        receipt_id: format!("blake3:{}", "1".repeat(64)),
        receipt_digest: String::new(),
        lease_id: lease.lease_id,
        idempotency_key: lease.idempotency_key,
        account_id: lease.account_id,
        agent_id: lease.agent_id,
        agent_session_id: lease.agent_session_id,
        agent_turn_id: lease.agent_turn_id,
        decision_request_id: lease.decision_request_id,
        request_digest: lease.request_digest,
        quote: lease.quote,
        reserved_amount: amount,
        consumed_amount: amount,
        refunded_amount: amount,
        status: CognitionLeaseStatusV1::Settled,
        issued_at_tick: 2,
    };
    receipt.receipt_digest = receipt.recompute_digest();
    let receipt_result = std::panic::catch_unwind(|| receipt.validate());
    assert!(
        receipt_result.is_ok(),
        "receipt boundary validation must return an error instead of panicking"
    );
    assert!(receipt_result.unwrap().is_err());
}

#[test]
fn validation_rejects_receipt_amount_mismatch_with_lease_transition() {
    let mut economy = CognitionEconomyStateV1::new();
    economy
        .set_resource_balance("account-agent-a", "cognition_units", 20)
        .expect("seed balance");
    let lease = economy
        .reserve(request("amount-linkage-key", 8), 1)
        .expect("reserve");
    economy.settle(&lease.lease_id, 5, 2).expect("settle");

    let stored_lease = economy
        .leases
        .get_mut(&lease.lease_id)
        .expect("stored lease");
    stored_lease.settled_amount = 4;
    stored_lease.refunded_amount = 4;

    assert!(
        economy.validate().is_err(),
        "receipt amounts must match the lease transition"
    );
}

#[test]
fn validation_rejects_duplicate_terminal_journal_transition() {
    let mut economy = CognitionEconomyStateV1::new();
    economy
        .set_resource_balance("account-agent-a", "cognition_units", 20)
        .expect("seed balance");
    let lease = economy
        .reserve(request("duplicate-event-key", 8), 1)
        .expect("reserve");
    economy.settle(&lease.lease_id, 5, 2).expect("settle");

    let mut duplicate = economy.journal.last().cloned().expect("settle event");
    duplicate.journal_seq = economy.journal.len() as u64 + 1;
    duplicate.parent_event_digest = economy
        .journal
        .last()
        .expect("settle event")
        .event_digest
        .clone();
    refresh_event_digest_for_test(&mut duplicate);
    economy.journal.push(duplicate);
    economy.head_seq = economy.journal.len() as u64;
    refresh_head_digest_for_test(&mut economy);

    assert!(
        economy.validate().is_err(),
        "a lease must have one canonical terminal journal transition"
    );
}

#[test]
fn every_durable_crash_prefix_round_trips_without_scheduler_state() {
    let mut economy = CognitionEconomyStateV1::new();
    economy
        .set_resource_balance("account-agent-a", "cognition_units", 12)
        .expect("seed balance");
    let prefixes = [
        economy.clone(),
        {
            let mut prefix = economy.clone();
            prefix
                .reserve(request("prefix-key", 4), 7)
                .expect("reserve prefix");
            prefix
        },
        {
            let mut prefix = economy.clone();
            let lease = prefix
                .reserve(request("prefix-key", 4), 7)
                .expect("reserve prefix");
            prefix.settle(&lease.lease_id, 3, 8).expect("settle prefix");
            prefix
        },
    ];
    for prefix in prefixes {
        let encoded = prefix.snapshot_json().expect("encode crash prefix");
        let restored = CognitionEconomyStateV1::from_snapshot_json(encoded.clone())
            .expect("decode crash prefix");
        assert_eq!(restored, prefix);
        assert_eq!(restored.snapshot_json().expect("re-encode"), encoded);
        assert!(restored.validate().is_ok());
    }
}

#[test]
fn world_save_load_retains_typed_economy_projection_atomically() {
    let mut world = World::new();
    world
        .set_cognition_resource_balance("account-agent-a", "cognition_units", 9)
        .expect("seed world cognition balance");
    let scheduler_before = world.cognition().get("scheduler_state").cloned();
    let lease = world
        .reserve_cognition_lease(request("world-key", 6))
        .expect("world reserve");
    assert_eq!(
        world.cognition().get("scheduler_state").cloned(),
        scheduler_before
    );

    let dir = temp_dir("persist");
    world.save_to_dir(&dir).expect("save reserved world");
    let restored = World::load_from_dir(&dir).expect("load reserved world");
    assert_eq!(
        restored.cognition_economy().expect("typed economy"),
        world.cognition_economy().expect("typed economy")
    );
    assert_eq!(
        restored
            .cognition_economy()
            .expect("typed economy")
            .reserved_balance("account-agent-a", "cognition_units"),
        6
    );

    let mut restored = restored;
    restored
        .settle_cognition_lease(&lease.lease_id, 4)
        .expect("settle restored lease");
    let round_trip = temp_dir("round-trip");
    restored
        .save_to_dir(&round_trip)
        .expect("save settled world");
    let loaded = World::load_from_dir(&round_trip).expect("load settled world");
    let economy = loaded.cognition_economy().expect("typed settled economy");
    assert_eq!(
        economy.available_balance("account-agent-a", "cognition_units"),
        5
    );
    assert_eq!(
        economy.reserved_balance("account-agent-a", "cognition_units"),
        0
    );
    assert_eq!(economy.receipts.len(), 1);
    assert_eq!(
        economy.leases[&lease.lease_id].status,
        CognitionLeaseStatusV1::Settled
    );
    assert_eq!(economy.journal.len(), 2);
    assert_eq!(
        serde_json::to_value(economy).expect("json economy")["schema_version"],
        Value::String(COGNITION_ECONOMY_SCHEMA_VERSION.to_string())
    );

    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&round_trip);
}
