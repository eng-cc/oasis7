//! Runtime cognition economy transition, idempotency and crash-prefix tests.

use super::super::*;
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
