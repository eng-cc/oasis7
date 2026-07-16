use super::*;
use ed25519_dalek::{Signer, SigningKey};
use oasis7_distfs::assemble_snapshot;

const ISSUE160_SAMPLE_WORLD_ENV: &str = "OASIS7_VERIFY_ISSUE160_SAMPLE_WORLD";

const ROLLBACK_NOW_MS: u64 = 1_720_000_000_000;

fn rollback_test_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn rollback_authority_registry(
    on_call_key: &SigningKey,
    governance_key: &SigningKey,
) -> RollbackAuthorityRegistry {
    RollbackAuthorityRegistry::new([
        RollbackAuthorityRecord {
            authority_id: "on-call-alice".to_string(),
            role: RollbackAuthorityRole::OnCall,
            public_key_hex: hex::encode(on_call_key.verifying_key().to_bytes()),
            active: true,
        },
        RollbackAuthorityRecord {
            authority_id: "governance-bob".to_string(),
            role: RollbackAuthorityRole::Governance,
            public_key_hex: hex::encode(governance_key.verifying_key().to_bytes()),
            active: true,
        },
    ])
    .expect("valid rollback authority registry")
}

#[test]
fn snapshot_deserialization_rejects_rollback_registry_map_key_mismatch() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");

    let mut snapshot = serde_json::to_value(world.snapshot()).expect("encode snapshot");
    let records = snapshot["rollback_authority_registry"]["records"]
        .as_object_mut()
        .expect("serialized registry records");
    let record = records
        .remove("on-call-alice")
        .expect("on-call registry record");
    records.insert("attacker-controlled-map-key".to_string(), record);

    let encoded = serde_json::to_string(&snapshot).expect("encode malformed snapshot");
    Snapshot::from_json(&encoded)
        .expect_err("registry map keys must exactly match normalized authority ids");
}

fn signed_rollback_authorization(
    snapshot: &Snapshot,
    target_journal_len: usize,
    expected_target_state_root: &str,
    target_batch_id: Option<&str>,
    ticket: &str,
    reason: &str,
    nonce: &str,
    on_call_key: &SigningKey,
    governance_key: &SigningKey,
) -> RollbackAuthorizationEnvelope {
    let intent = RollbackIntent {
        schema_version: 1,
        rollback_ticket: ticket.to_string(),
        snapshot_hash: util::hash_json(snapshot).expect("snapshot hash"),
        snapshot_journal_len: snapshot.journal_len,
        target_journal_len,
        target_journal_commitment: None,
        expected_target_state_root: expected_target_state_root.to_string(),
        target_batch_id: target_batch_id.map(str::to_string),
        reason: reason.to_string(),
        issued_at_ms: ROLLBACK_NOW_MS - 1_000,
        expires_at_ms: ROLLBACK_NOW_MS + 60_000,
        nonce: nonce.to_string(),
    };
    let payload = intent
        .canonical_signing_payload()
        .expect("canonical rollback signing payload");
    RollbackAuthorizationEnvelope {
        intent,
        signatures: vec![
            RollbackApprovalSignature {
                authority_id: "on-call-alice".to_string(),
                role: RollbackAuthorityRole::OnCall,
                signature_scheme: "ed25519".to_string(),
                signature_hex: hex::encode(on_call_key.sign(&payload).to_bytes()),
            },
            RollbackApprovalSignature {
                authority_id: "governance-bob".to_string(),
                role: RollbackAuthorityRole::Governance,
                signature_scheme: "ed25519".to_string(),
                signature_hex: hex::encode(governance_key.sign(&payload).to_bytes()),
            },
        ],
    }
}

#[test]
fn load_from_dir_falls_back_to_json_when_distfs_sidecar_is_invalid() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();

    let dir = temp_dir("persist-distfs-fallback");
    world.save_to_dir(&dir).expect("save world");
    fs::write(
        dir.join("snapshot.manifest.json"),
        b"{\"manifest\":\"broken\"}",
    )
    .expect("tamper sidecar");

    let restored = World::load_from_dir(&dir).expect("fallback to legacy json");
    assert_eq!(restored.state(), world.state());
    let audit_value: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("distfs.recovery.audit.json")).expect("read distfs fallback audit"),
    )
    .expect("decode distfs fallback audit");
    assert_eq!(
        audit_value.get("status").and_then(|value| value.as_str()),
        Some("fallback_json")
    );
    assert!(
        audit_value
            .get("reason")
            .and_then(|value| value.as_str())
            .map(|reason| reason.contains("distfs_restore_failed"))
            .unwrap_or(false)
    );
    assert!(
        audit_value
            .get("timestamp_ms")
            .and_then(|value| value.as_i64())
            .is_some()
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_json_without_era_fields_keeps_backward_compatibility() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-legacy".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("step");

    let snapshot = world.snapshot();
    let mut value = serde_json::to_value(&snapshot).expect("encode snapshot");
    let object = value.as_object_mut().expect("snapshot object");
    object.remove("event_id_era");
    object.remove("action_id_era");
    object.remove("intent_id_era");
    object.remove("proposal_id_era");

    let legacy_json = serde_json::to_string(&value).expect("legacy json");
    let restored = Snapshot::from_json(&legacy_json).expect("decode legacy snapshot");
    assert_eq!(restored.event_id_era, 0);
    assert_eq!(restored.action_id_era, 0);
    assert_eq!(restored.intent_id_era, 0);
    assert_eq!(restored.proposal_id_era, 0);
}

#[test]
fn sample_issue160_execution_world_matches_tick_consensus_after_distfs_restore() {
    if std::env::var_os(ISSUE160_SAMPLE_WORLD_ENV).is_none() {
        return;
    }

    let dir = std::path::Path::new(
        "output/chain-runtime/viewer-live-node-playtest-issue160-trust-refresh-fix1/reward-runtime-execution-world",
    );
    assert!(
        dir.exists(),
        "set {ISSUE160_SAMPLE_WORLD_ENV}=1 only when the sample world exists at {}",
        dir.display()
    );

    let snapshot_json = Snapshot::load_json(dir.join("snapshot.json")).expect("load snapshot json");
    let journal = Journal::load_json(dir.join("journal.json")).expect("load journal json");
    let manifest: oasis7_proto::distributed::SnapshotManifest =
        crate::runtime::util::read_json_from_path(dir.join("snapshot.manifest.json").as_path())
            .expect("load distfs snapshot manifest");
    let store = LocalCasStore::new(dir.join(".distfs-state"));
    let snapshot_distfs: Snapshot =
        assemble_snapshot(&manifest, &store).expect("assemble distfs snapshot");

    let world_from_json =
        World::from_snapshot(snapshot_json.clone(), journal.clone()).expect("world from json");
    let world_from_distfs =
        World::from_snapshot(snapshot_distfs.clone(), journal).expect("world from distfs");

    let json_tick_root = snapshot_json
        .tick_consensus_records
        .last()
        .map(|record| record.block.header.state_root.clone())
        .expect("json tick consensus record");
    let distfs_tick_root = snapshot_distfs
        .tick_consensus_records
        .last()
        .map(|record| record.block.header.state_root.clone())
        .expect("distfs tick consensus record");
    let json_world_tick_root = world_from_json
        .latest_tick_consensus_record()
        .map(|record| record.block.header.state_root.clone())
        .expect("json world tick root");
    let distfs_world_tick_root = world_from_distfs
        .latest_tick_consensus_record()
        .map(|record| record.block.header.state_root.clone())
        .expect("distfs world tick root");

    assert_eq!(json_tick_root, json_world_tick_root);
    assert_eq!(distfs_tick_root, distfs_world_tick_root);
}

#[test]
fn rollback_to_snapshot_resets_state() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();
    let snapshot = world.snapshot();
    let snapshot_state_root = world
        .current_state_root_hash()
        .expect("snapshot state root");

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(9, 9),
    });
    world.step().unwrap();
    assert_eq!(
        world.state().agents.get("agent-1").unwrap().state.pos,
        pos(9, 9)
    );

    let journal = world.journal().clone();
    let approval = signed_rollback_authorization(
        &snapshot,
        snapshot.journal_len,
        snapshot_state_root.as_str(),
        None,
        "ROLLBACK-LOW-LEVEL-TEST",
        "test-rollback",
        "nonce-low-level-1",
        &on_call_key,
        &governance_key,
    );
    world
        .rollback_to_snapshot(
            snapshot.clone(),
            journal,
            "test-rollback",
            None,
            approval,
            ROLLBACK_NOW_MS,
        )
        .unwrap();

    assert_eq!(world.state(), &snapshot.state);
    let last = world.journal().events.last().unwrap();
    assert!(matches!(last.body, WorldEventBody::RollbackApplied(_)));
}

#[test]
fn rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world
        .bind_node_identity("relay.node.1", "relay-public-key-1")
        .expect("bind relay identity");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("step");

    let stable_snapshot = world.snapshot();
    let stable_journal = world.journal().clone();
    let stable_state_root = world.current_state_root_hash().expect("stable state root");

    world
        .record_tick_consensus_propagation_for_tick(0, "relay.node.1")
        .expect("inject propagation record that breaks parent ordering");
    let drift = world
        .first_tick_consensus_drift()
        .expect("drift report should be present");
    assert_eq!(drift.tick, 0);
    assert!(
        drift.reason.contains("parent hash mismatch"),
        "unexpected drift reason: {}",
        drift.reason
    );
    world
        .verify_tick_consensus_chain()
        .expect_err("drifted chain should fail verification");

    let approval = signed_rollback_authorization(
        &stable_snapshot,
        stable_journal.len(),
        stable_state_root.as_str(),
        None,
        "ROLLBACK-2313",
        "reconcile-after-drift",
        "nonce-reconcile-1",
        &on_call_key,
        &governance_key,
    );
    world
        .rollback_to_snapshot_with_reconciliation(
            stable_snapshot,
            stable_journal,
            "reconcile-after-drift",
            None,
            approval,
            ROLLBACK_NOW_MS,
        )
        .expect("rollback with reconciliation");

    assert!(
        world.first_tick_consensus_drift().is_none(),
        "drift should be fully reconciled after rollback"
    );
    world
        .verify_tick_consensus_chain()
        .expect("reconciled chain should verify");

    let rollback = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::RollbackApplied(rollback) => Some(rollback),
            _ => None,
        })
        .expect("rollback audit event");
    assert_eq!(rollback.rollback_ticket, "ROLLBACK-2313");
    assert_eq!(rollback.on_call_authority_id, "on-call-alice");
    assert_eq!(rollback.governance_authority_id, "governance-bob");
    assert_eq!(rollback.authorization_nonce, "nonce-reconcile-1");
}

#[test]
fn rollback_replays_authoritative_journal_suffix_to_the_target_state() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("create stable snapshot state");
    let stable_snapshot = world.snapshot();

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(9, 9),
    });
    world.step().expect("create authoritative catch-up suffix");
    let target_state = world.state().clone();
    let target_journal = world.journal().clone();
    let target_state_root = world.current_state_root_hash().expect("target state root");
    assert!(target_journal.len() > stable_snapshot.journal_len);
    let approval = signed_rollback_authorization(
        &stable_snapshot,
        target_journal.len(),
        target_state_root.as_str(),
        None,
        "ROLLBACK-2313-REPLAY-TARGET",
        "replay-to-authoritative-target",
        "nonce-replay-target-1",
        &on_call_key,
        &governance_key,
    );

    world
        .rollback_to_snapshot_with_reconciliation(
            stable_snapshot,
            target_journal,
            "replay-to-authoritative-target",
            None,
            approval,
            ROLLBACK_NOW_MS,
        )
        .expect("rollback and deterministic catch-up must succeed");

    assert_eq!(
        world.state(),
        &target_state,
        "rollback must replay the authoritative post-snapshot suffix to its target state"
    );
}

#[test]
fn rollback_replay_rejects_tampered_suffix_atomically_without_consuming_nonce() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("create stable snapshot state");
    let stable_snapshot = world.snapshot();
    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(9, 9),
    });
    world.step().expect("create authoritative suffix");
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let target_state_root = world.current_state_root_hash().expect("target state root");
    let mut tampered_journal = journal_before.clone();
    tampered_journal.events[stable_snapshot.journal_len].id = stable_snapshot.last_event_id;
    let approval = signed_rollback_authorization(
        &stable_snapshot,
        tampered_journal.len(),
        target_state_root.as_str(),
        None,
        "ROLLBACK-2313-TAMPERED-REPLAY",
        "reject-tampered-replay",
        "nonce-tampered-replay-1",
        &on_call_key,
        &governance_key,
    );

    world
        .rollback_to_snapshot_with_reconciliation(
            stable_snapshot,
            tampered_journal,
            "reject-tampered-replay",
            None,
            approval,
            ROLLBACK_NOW_MS,
        )
        .expect_err("tampered replay suffix must be rejected before commit");

    assert_eq!(world.snapshot(), snapshot_before);
    assert_eq!(world.journal(), &journal_before);
    assert!(
        !world
            .snapshot()
            .consumed_rollback_nonces
            .contains("nonce-tampered-replay-1")
    );
}

#[test]
fn rollback_replay_target_range_and_state_root_failures_are_atomic() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    for invalid_case in ["target_range", "target_state_root"] {
        let mut world = World::new();
        world
            .set_rollback_authority_registry(rollback_authority_registry(
                &on_call_key,
                &governance_key,
            ))
            .expect("configure rollback authorities");
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: pos(0, 0),
        });
        world.step().expect("create stable snapshot state");
        let stable_snapshot = world.snapshot();
        world.submit_action(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: pos(9, 9),
        });
        world.step().expect("create replay target");
        let target_journal = world.journal().clone();
        let target_state_root = world.current_state_root_hash().expect("target state root");
        let snapshot_before = world.snapshot();
        let journal_before = world.journal().clone();
        let nonce = format!("nonce-{invalid_case}");
        let approval = signed_rollback_authorization(
            &stable_snapshot,
            if invalid_case == "target_range" {
                target_journal.len() + 1
            } else {
                target_journal.len()
            },
            if invalid_case == "target_state_root" {
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            } else {
                target_state_root.as_str()
            },
            None,
            "ROLLBACK-2313-TARGET-VALIDATION",
            "validate-signed-replay-target",
            nonce.as_str(),
            &on_call_key,
            &governance_key,
        );

        let error = world
            .rollback_to_snapshot_with_reconciliation(
                stable_snapshot,
                target_journal,
                "validate-signed-replay-target",
                None,
                approval,
                ROLLBACK_NOW_MS,
            )
            .expect_err("invalid signed replay target must fail atomically");
        assert!(
            matches!(
                (invalid_case, error),
                (
                    "target_range",
                    WorldError::RollbackReplayTargetInvalid { .. }
                ) | (
                    "target_state_root",
                    WorldError::RollbackTargetStateRootMismatch { .. }
                )
            ),
            "unexpected error for {invalid_case}"
        );
        assert_eq!(world.snapshot(), snapshot_before);
        assert_eq!(world.journal(), &journal_before);
        assert!(!world.snapshot().consumed_rollback_nonces.contains(&nonce));
    }
}

#[test]
fn rollback_reconciliation_drift_rejection_leaves_world_unchanged() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world
        .bind_node_identity("relay.node.1", "relay-public-key-1")
        .expect("bind relay identity");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("step");
    world
        .record_tick_consensus_propagation_for_tick(0, "relay.node.1")
        .expect("inject propagation record that breaks parent ordering");
    assert!(world.first_tick_consensus_drift().is_some());
    let drifted_snapshot = world.snapshot();
    let drifted_journal = world.journal().clone();
    let drifted_state_root = world.current_state_root_hash().expect("drifted state root");

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(9, 9),
    });
    world
        .step()
        .expect("mutate original world after target snapshot");
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    assert_ne!(
        snapshot_before, drifted_snapshot,
        "test precondition: rejected rollback candidate must differ from the current world"
    );
    let approval = signed_rollback_authorization(
        &drifted_snapshot,
        drifted_journal.len(),
        drifted_state_root.as_str(),
        None,
        "ROLLBACK-2313-DRIFT",
        "reject-drifted-candidate",
        "nonce-drift-rejected-1",
        &on_call_key,
        &governance_key,
    );

    let error = world
        .rollback_to_snapshot_with_reconciliation(
            drifted_snapshot,
            drifted_journal,
            "reject-drifted-candidate",
            None,
            approval,
            ROLLBACK_NOW_MS,
        )
        .expect_err("drifted rollback candidate must be rejected");
    assert!(
        matches!(
            error,
            WorldError::RollbackReconciliationFailed { ref reason, .. }
                if reason.contains("tick consensus parent hash mismatch")
        ),
        "expected drift validation rejection, got {error:?}"
    );

    assert_eq!(world.snapshot(), snapshot_before);
    assert_eq!(world.journal(), &journal_before);
    assert!(
        !world
            .snapshot()
            .consumed_rollback_nonces
            .contains("nonce-drift-rejected-1")
    );
}

#[test]
fn rollback_rejects_tampered_expired_or_same_key_authorization_before_mutation() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    for invalid_case in [
        "tampered_ticket",
        "tampered_target_journal_len",
        "tampered_target_state_root",
        "expired",
        "same_key_wrong_role",
        "target_mismatch",
    ] {
        let mut world = World::new();
        world
            .set_rollback_authority_registry(rollback_authority_registry(
                &on_call_key,
                &governance_key,
            ))
            .expect("configure rollback authorities");
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: pos(0, 0),
        });
        world.step().expect("step");
        let stable_snapshot = world.snapshot();
        let stable_journal = world.journal().clone();
        let stable_state_root = world.current_state_root_hash().expect("stable state root");

        world.submit_action(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: pos(9, 9),
        });
        world.step().expect("mutate after snapshot");
        let state_before = world.state().clone();
        let journal_before = world.journal().clone();

        let mut approval = signed_rollback_authorization(
            &stable_snapshot,
            stable_journal.len(),
            stable_state_root.as_str(),
            Some("batch-stable-1"),
            "ROLLBACK-2313",
            "invalid-authorization-must-not-mutate",
            invalid_case,
            &on_call_key,
            &governance_key,
        );
        match invalid_case {
            "tampered_ticket" => approval.intent.rollback_ticket.push_str("-tampered"),
            "tampered_target_journal_len" => approval.intent.target_journal_len += 1,
            "tampered_target_state_root" => {
                approval.intent.expected_target_state_root = "f".repeat(64)
            }
            "expired" => approval.intent.expires_at_ms = ROLLBACK_NOW_MS - 1,
            "same_key_wrong_role" => {
                let payload = approval
                    .intent
                    .canonical_signing_payload()
                    .expect("canonical payload");
                approval.signatures[1].authority_id = "on-call-alice".to_string();
                approval.signatures[1].role = RollbackAuthorityRole::Governance;
                approval.signatures[1].signature_hex =
                    hex::encode(on_call_key.sign(&payload).to_bytes());
            }
            "target_mismatch" => {}
            _ => unreachable!(),
        }

        let expected_target_batch_id = if invalid_case == "target_mismatch" {
            Some("batch-other")
        } else {
            Some("batch-stable-1")
        };

        world
            .rollback_to_snapshot_with_reconciliation(
                stable_snapshot,
                stable_journal,
                "invalid-authorization-must-not-mutate",
                expected_target_batch_id,
                approval,
                ROLLBACK_NOW_MS,
            )
            .expect_err("invalid rollback approval must be rejected");

        assert_eq!(
            world.state(),
            &state_before,
            "state mutated before rejection"
        );
        assert_eq!(
            world.journal(),
            &journal_before,
            "journal mutated before rejection"
        );
    }
}

#[test]
fn rollback_rejects_same_id_journal_body_substitution_before_mutation() {
    // The signed target must commit to event contents, not merely IDs, length, and state root.
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("create stable checkpoint");

    let stable_snapshot = world.snapshot();
    let stable_journal = world.journal().clone();
    let mut substituted_journal = stable_journal.clone();
    let stable_state_root = world.current_state_root_hash().expect("stable state root");
    let original_event = substituted_journal
        .events
        .first_mut()
        .expect("checkpoint journal event");
    original_event.body = WorldEventBody::SnapshotCreated(SnapshotMeta {
        journal_len: stable_snapshot.journal_len,
    });

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(9, 9),
    });
    world.step().expect("mutate live world after checkpoint");
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let mut approval = signed_rollback_authorization(
        &stable_snapshot,
        substituted_journal.len(),
        stable_state_root.as_str(),
        Some("batch-stable-1"),
        "ROLLBACK-2313-JOURNAL-COMMITMENT",
        "same-id-body-substitution-must-reject",
        "nonce-journal-body-substitution-1",
        &on_call_key,
        &governance_key,
    );
    approval.intent.schema_version = 2;
    approval.intent.target_journal_commitment = Some(
        rollback_journal_commitment(&stable_snapshot, &stable_journal, stable_journal.len())
            .expect("canonical stable journal commitment"),
    );
    let payload = approval
        .intent
        .canonical_signing_payload()
        .expect("canonical v2 rollback payload");
    approval.signatures[0].signature_hex = hex::encode(on_call_key.sign(&payload).to_bytes());
    approval.signatures[1].signature_hex = hex::encode(governance_key.sign(&payload).to_bytes());

    world
        .rollback_to_snapshot_with_reconciliation(
            stable_snapshot,
            substituted_journal,
            "same-id-body-substitution-must-reject",
            Some("batch-stable-1"),
            approval,
            ROLLBACK_NOW_MS,
        )
        .expect_err("signed journal commitment must reject same-ID event body substitution");

    assert_eq!(
        world.snapshot(),
        snapshot_before,
        "snapshot mutated on rejection"
    );
    assert_eq!(
        world.journal(),
        &journal_before,
        "journal mutated on rejection"
    );
}

#[test]
fn rollback_nonce_is_durable_and_cannot_be_replayed_through_an_old_snapshot() {
    let on_call_key = rollback_test_key(7);
    let governance_key = rollback_test_key(9);
    let mut world = World::new();
    world
        .set_rollback_authority_registry(rollback_authority_registry(&on_call_key, &governance_key))
        .expect("configure rollback authorities");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("step");
    let old_snapshot = world.snapshot();
    let old_journal = world.journal().clone();
    let old_state_root = world.current_state_root_hash().expect("old state root");
    let approval = signed_rollback_authorization(
        &old_snapshot,
        old_journal.len(),
        old_state_root.as_str(),
        None,
        "ROLLBACK-2313-REPLAY",
        "durable-replay-test",
        "nonce-durable-1",
        &on_call_key,
        &governance_key,
    );

    world
        .rollback_to_snapshot(
            old_snapshot.clone(),
            old_journal.clone(),
            "durable-replay-test",
            None,
            approval.clone(),
            ROLLBACK_NOW_MS,
        )
        .expect("first use succeeds");
    let recovery_metadata = RollbackOutcomeRecoveryMetadata {
        target_batch_id: "batch-target-3".to_string(),
        prior_reorg_epoch: 7,
        committed_reorg_epoch: 8,
        invalidated_batch_ids: vec!["batch-fork-4".to_string(), "batch-fork-5".to_string()],
        dispositions: vec![RollbackEventDisposition {
            source_batch_id: "batch-fork-4".to_string(),
            source_event_id: 41,
            status: RollbackDispositionStatus::RejectedFork,
            player_id: Some("player-1".to_string()),
            action_id: Some("action-9".to_string()),
        }],
    };
    world
        .record_rollback_outcome_recovery_metadata("nonce-durable-1", recovery_metadata.clone())
        .expect("complete durable rollback receipt metadata");

    let dir = temp_dir("persist-rollback-replay-state");
    world
        .save_to_dir(&dir)
        .expect("persist rollback replay state");
    let mut world = World::load_from_dir(&dir).expect("restore rollback replay state");
    let restored_outcome = world
        .rollback_nonce_outcome("nonce-durable-1")
        .expect("public lookup returns persisted rollback outcome");
    assert_eq!(
        restored_outcome.target_batch_id,
        recovery_metadata.target_batch_id
    );
    assert_eq!(restored_outcome.prior_reorg_epoch, 7);
    assert_eq!(restored_outcome.committed_reorg_epoch, 8);
    assert_eq!(
        restored_outcome.invalidated_batch_ids,
        recovery_metadata.invalidated_batch_ids
    );
    assert_eq!(
        restored_outcome.dispositions,
        recovery_metadata.dispositions
    );
    let restored_snapshot = world.snapshot();
    assert_eq!(
        restored_snapshot.rollback_authority_registry,
        rollback_authority_registry(&on_call_key, &governance_key)
    );
    assert!(
        restored_snapshot
            .consumed_rollback_nonces
            .contains("nonce-durable-1")
    );

    let state_after_first = world.state().clone();
    let journal_after_first = world.journal().clone();

    world
        .rollback_to_snapshot(
            old_snapshot.clone(),
            old_journal.clone(),
            "durable-replay-test",
            None,
            approval.clone(),
            ROLLBACK_NOW_MS,
        )
        .expect("exact retry must return the committed outcome without mutation");
    assert_eq!(world.state(), &state_after_first);
    assert_eq!(world.journal(), &journal_after_first);
    assert_eq!(
        world
            .rollback_nonce_outcome("nonce-durable-1")
            .expect("exact retry preserves stored outcome"),
        restored_outcome
    );

    let mut altered = approval;
    altered.intent.reason = "altered-intent".to_string();
    let error = world
        .rollback_to_snapshot(
            old_snapshot,
            old_journal,
            "altered-intent",
            None,
            altered,
            ROLLBACK_NOW_MS,
        )
        .expect_err("same nonce with altered canonical intent must conflict");
    assert!(matches!(error, WorldError::RollbackNonceConflict { .. }));
    assert_eq!(world.state(), &state_after_first);
    assert_eq!(world.journal(), &journal_after_first);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_retention_policy_prunes_old_entries() {
    let mut world = World::new();
    world.set_snapshot_retention(SnapshotRetentionPolicy { max_snapshots: 1 });

    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();
    let snap1 = world.create_snapshot().unwrap();

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(3, 3),
    });
    world.step().unwrap();
    let snap2 = world.create_snapshot().unwrap();

    assert_eq!(world.snapshot_catalog().records.len(), 1);
    let last_record = &world.snapshot_catalog().records[0];
    assert_eq!(last_record.snapshot_hash, util::hash_json(&snap2).unwrap());
    assert_ne!(last_record.snapshot_hash, util::hash_json(&snap1).unwrap());
}

#[test]
fn snapshot_file_pruning_removes_old_files() {
    let mut world = World::new();
    world.set_snapshot_retention(SnapshotRetentionPolicy { max_snapshots: 1 });

    let dir = std::env::temp_dir().join(format!(
        "oasis7-snapshots-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    world.save_snapshot_to_dir(&dir).unwrap();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();
    world.save_snapshot_to_dir(&dir).unwrap();

    let snapshots_dir = dir.join("snapshots");
    let file_count = fs::read_dir(&snapshots_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .count();
    assert_eq!(file_count, 1);

    let _ = fs::remove_dir_all(&dir);
}
