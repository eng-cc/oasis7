use super::*;
use crate::viewer::runtime_live::recovery_receipt::RuntimeAuthoritativeRecoveryGeneration;

#[test]
fn rollback_receipt_freezes_full_signed_identity_and_evidence() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let _fork = commit_single_authoritative_batch(&mut server);
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "full-receipt",
        "nonce-full-receipt",
        &on_call,
        &governance,
    );
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "full-receipt".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let receipt =
        serde_json::to_value(ack.rollback_receipt.expect("receipt")).expect("receipt json");
    for field in [
        "canonical_intent_digest",
        "rollback_checkpoint",
        "replay_target",
        "journal_commitment",
        "target_state_root",
    ] {
        assert!(
            receipt.get(field).is_some(),
            "immutable receipt is missing {field}"
        );
    }
}

#[test]
fn readiness_metadata_status_unknown_installs_write_fence() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-readiness-unknown-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "readiness-unknown",
        "nonce-readiness-unknown",
        &on_call,
        &governance,
    );
    server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "readiness-unknown".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let readiness_before = server.rollback_readiness.clone();
    let root = recovery_dir.join(".distfs-state/sidecar-generations");
    std::fs::write(root.join(".test-fail-before-index-commit"), b"fail").expect("failpoint");
    std::fs::write(root.join("index.json"), b"not-json").expect("corrupt index");

    let error = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReevaluateRollbackReadiness {
            authorization_nonce: "nonce-readiness-unknown".to_string(),
            audit_evidence: None,
        })
        .expect_err("status unknown");

    assert_eq!(error.code, "rollback_persistence_status_unknown");
    assert!(
        server.authoritative_recovery_write_fence.is_some(),
        "indeterminate readiness metadata commit must fence writes"
    );
    assert_eq!(
        server.rollback_readiness, readiness_before,
        "unknown result retains pre-mutation projection"
    );
    server
        .resolve_authoritative_recovery_write_fence()
        .expect("resolve attempt");
    assert!(
        server.authoritative_recovery_write_fence.is_some(),
        "unknown readback must remain fenced"
    );
    let _ = std::fs::remove_dir_all(recovery_dir);
}

#[test]
fn rollback_player_action_census_deduplicates_multi_event_actions() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let _fork = commit_single_authoritative_batch(&mut server);
    for action_id in server
        .authoritative_batches
        .iter()
        .flat_map(|batch| batch.events.iter())
        .filter_map(|event| event.runtime_event.as_ref())
        .filter_map(|event| match event.caused_by.as_ref() {
            Some(crate::runtime::CausedBy::Action(action_id)) => Some(*action_id),
            _ => None,
        })
        .collect::<Vec<_>>()
    {
        server
            .runtime_action_players
            .insert(action_id, "player-census-a".to_string());
    }
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "action-census",
        "nonce-action-census",
        &on_call,
        &governance,
    );
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "action-census".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let dispositions = ack.rollback_receipt.expect("receipt").player_dispositions;
    let action_ids = dispositions
        .iter()
        .map(|entry| {
            entry
                .action_id
                .clone()
                .expect("every player disposition has action identity")
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        dispositions
            .iter()
            .all(|entry| entry.player_id.as_deref() == Some("player-census-a")),
        "every player disposition must preserve exact player attribution"
    );
    assert!(
        action_ids
            .iter()
            .all(|action_id| !action_id.starts_with("unknown:"))
    );
    assert_eq!(
        action_ids.len(),
        dispositions.len(),
        "one action must have exactly one disposition even when it emitted multiple events"
    );
}

#[test]
fn rollback_all_clear_is_blocked_without_explicit_verification_evidence() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "all-clear-evidence",
        "nonce-all-clear-evidence",
        &on_call,
        &governance,
    );
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "all-clear-evidence".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let receipt = ack.rollback_receipt.expect("receipt");
    assert!(
        !receipt.ready_for_all_clear,
        "all-clear must stay blocked until explicit root, epoch, drift, consensus, receipt, and census evidence is committed"
    );
    assert!(!receipt.readiness_blockers.is_empty());
}

#[test]
fn recovery_generation_round_trip_preserves_revoked_session_policy_and_metadata() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-viewer-session-recovery-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    server
        .session_policy
        .register_session("player-restart", "revoked-key")
        .expect("register session");
    server
        .session_policy
        .rotate_session("player-restart", "revoked-key", "replacement-key")
        .expect("rotate session");
    server.record_session_revoke_metadata(
        "player-restart",
        "revoked-key",
        RuntimeSessionRevokeMetadata {
            revoke_reason: Some("rotation".to_string()),
            revoked_by: Some("security-ops".to_string()),
        },
    );

    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "persist-session-policy",
        "nonce-persist-session-policy",
        &on_call,
        &governance,
    );
    server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "persist-session-policy".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("persist recovery generation");

    let metadata = RuntimeWorld::load_authoritative_recovery_metadata(&recovery_dir)
        .expect("load metadata")
        .expect("committed metadata");
    let mut generation: RuntimeAuthoritativeRecoveryGeneration =
        serde_json::from_slice(&metadata).expect("decode generation");
    assert!(
        generation
            .session_policy
            .register_session("player-restart", "revoked-key")
            .is_err(),
        "revoked key must remain rejected after generation restore"
    );
    assert!(generation.session_revoke_metadata.iter().any(|entry| {
        entry.player_id == "player-restart"
            && entry.session_pubkey == "revoked-key"
            && entry.metadata.revoke_reason.as_deref() == Some("rotation")
            && entry.metadata.revoked_by.as_deref() == Some("security-ops")
    }));

    let legacy_value = serde_json::json!({
        "schema_version": generation.schema_version,
        "ack": generation.ack,
        "authoritative_batches": generation.authoritative_batches,
        "next_authoritative_batch_id": generation.next_authoritative_batch_id,
        "authoritative_challenges": generation.authoritative_challenges,
        "next_authoritative_challenge_id": generation.next_authoritative_challenge_id,
        "stable_checkpoints": generation.stable_checkpoints,
        "reorg_epoch": generation.reorg_epoch
    });
    let migrated: RuntimeAuthoritativeRecoveryGeneration =
        serde_json::from_value(legacy_value).expect("older generation uses migration defaults");
    assert!(migrated.session_revoke_metadata.is_empty());
    let _ = std::fs::remove_dir_all(recovery_dir);
}

#[test]
fn readiness_reevaluation_stays_blocked_without_fresh_strict_audit_evidence() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-viewer-readiness-recovery-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "readiness-transition",
        "nonce-readiness-transition",
        &on_call,
        &governance,
    );
    let (blocked, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "readiness-transition".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let blocked_receipt = blocked.rollback_receipt.expect("receipt");
    assert!(!blocked_receipt.ready_for_all_clear);

    let (reevaluated, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReevaluateRollbackReadiness {
            authorization_nonce: "nonce-readiness-transition".to_string(),
            audit_evidence: None,
        })
        .expect("reevaluate readiness");
    let receipt = reevaluated.rollback_receipt.expect("reevaluated receipt");
    assert!(!receipt.ready_for_all_clear);
    assert!(
        receipt
            .readiness_blockers
            .iter()
            .any(|blocker| blocker == "fresh_strict_audit_evidence_required")
    );

    let metadata = RuntimeWorld::load_authoritative_recovery_metadata(&recovery_dir)
        .expect("load metadata")
        .expect("committed metadata");
    let generation: RuntimeAuthoritativeRecoveryGeneration =
        serde_json::from_slice(&metadata).expect("decode generation");
    let evidence = generation
        .rollback_readiness
        .get("nonce-readiness-transition")
        .expect("persisted readiness evidence");
    assert!(!evidence.ready);
    assert!(!evidence.canonical_intent_digest.is_empty());
    assert!(!evidence.affected_census_digest.is_empty());

    let mut audit_evidence = crate::viewer::protocol::RollbackStrictAuditEvidence {
        rollback_ticket: blocked_receipt.rollback_ticket.clone(),
        receipt_id: blocked_receipt.receipt_id.clone(),
        canonical_intent_digest: blocked_receipt.canonical_intent_digest.clone(),
        recovery_snapshot_hash: blocked_receipt.snapshot_hash.clone(),
        reorg_epoch: blocked_receipt.committed_reorg_epoch,
        candidate_state_root: server
            .world
            .current_state_root_hash()
            .expect("current root"),
        strict_registry_audit_passed: true,
        strict_manifest_audit_passed: true,
        evidence_digest: "strict-audit-digest".to_string(),
        observed_at_ms: crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms(),
    };
    let mut mismatched_audit = audit_evidence.clone();
    mismatched_audit.candidate_state_root = "not-the-current-candidate-root".to_string();
    let (root_mismatch, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReevaluateRollbackReadiness {
            authorization_nonce: "nonce-readiness-transition".to_string(),
            audit_evidence: Some(mismatched_audit),
        })
        .expect("audit root mismatch returns blocked readiness");
    assert!(
        root_mismatch
            .rollback_receipt
            .expect("root mismatch receipt")
            .readiness_blockers
            .iter()
            .any(|blocker| blocker == "fresh_strict_audit_evidence_required"),
        "strict audit evidence must bind the observed candidate world root"
    );
    audit_evidence.candidate_state_root = server
        .world
        .current_state_root_hash()
        .expect("restored root");
    audit_evidence.observed_at_ms =
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms();
    let (ready, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReevaluateRollbackReadiness {
            authorization_nonce: "nonce-readiness-transition".to_string(),
            audit_evidence: Some(audit_evidence),
        })
        .expect("reevaluate with fresh strict audit evidence");
    assert!(
        ready
            .rollback_receipt
            .expect("ready receipt")
            .ready_for_all_clear
    );

    let metadata = RuntimeWorld::load_authoritative_recovery_metadata(&recovery_dir)
        .expect("reload metadata")
        .expect("committed metadata");
    let restarted: RuntimeAuthoritativeRecoveryGeneration =
        serde_json::from_slice(&metadata).expect("decode restarted generation");
    let restarted_readiness = restarted
        .rollback_readiness
        .get("nonce-readiness-transition")
        .expect("restart preserves readiness evidence");
    assert!(restarted_readiness.ready);
    assert_eq!(
        restarted_readiness.strict_audit_evidence_digest,
        "strict-audit-digest"
    );
    assert_eq!(
        restarted_readiness.candidate_state_root, blocked_receipt.target_state_root,
        "persisted all-clear evidence records the observed candidate root"
    );
    let _ = std::fs::remove_dir_all(recovery_dir);
}

#[test]
fn durable_receipt_does_not_fabricate_player_compensation_for_system_events() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let _fork = commit_single_authoritative_batch(&mut server);
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "compensation-status",
        "nonce-compensation-status",
        &on_call,
        &governance,
    );
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "compensation-status".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let dispositions = ack.rollback_receipt.expect("receipt").player_dispositions;
    assert!(
        dispositions.is_empty(),
        "runtime-authored system events must remain journal evidence, not fake player compensation"
    );
}

#[test]
fn compensation_projection_exposes_safe_owner_ticket_and_state() {
    let dispositions = crate::viewer::runtime_live::recovery_receipt::viewer_dispositions(
        "ROLLBACK-SAFE-COMPENSATION",
        "nonce-safe-compensation",
        &[crate::runtime::RollbackEventDisposition {
            source_batch_id: "batch-player-action".to_string(),
            source_event_id: 77,
            status: crate::runtime::RollbackDispositionStatus::CompensationRequired,
            compensation: Some(crate::runtime::RollbackCompensationCaseRef {
                owner_id: "player-support-queue".to_string(),
                ticket_id: "public-case-77".to_string(),
                state: crate::runtime::RollbackCompensationState::InProgress,
            }),
            player_id: Some("player-77".to_string()),
            action_id: Some("action-77".to_string()),
        }],
    );
    let disposition = dispositions.into_iter().next().expect("disposition");
    let compensation = disposition.compensation.expect("safe compensation status");
    assert_eq!(
        compensation.case_id,
        disposition.compensation_case_id.unwrap()
    );
    assert_eq!(compensation.responsible_party, "player-support-queue");
    assert_eq!(compensation.ticket_reference, "public-case-77");
    assert_eq!(
        serde_json::to_value(compensation.state).expect("serialize compensation state"),
        serde_json::json!("in_progress")
    );
}

#[test]
fn rejected_attributed_actions_create_accountable_public_compensation_cases() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let _fork = commit_single_authoritative_batch(&mut server);
    for action_id in server
        .authoritative_batches
        .iter()
        .skip(1)
        .flat_map(|batch| batch.events.iter())
        .filter_map(|event| event.runtime_event.as_ref())
        .filter_map(|event| match event.caused_by.as_ref() {
            Some(crate::runtime::CausedBy::Action(action_id)) => Some(*action_id),
            _ => None,
        })
        .collect::<Vec<_>>()
    {
        server
            .runtime_action_players
            .insert(action_id, "player-private-42".to_string());
    }
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "accountable-compensation",
        "nonce-accountable-compensation",
        &on_call,
        &governance,
    );
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "accountable-compensation".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let dispositions = ack.rollback_receipt.expect("receipt").player_dispositions;
    assert!(
        !dispositions.is_empty(),
        "attributed fork actions are censused"
    );
    for disposition in dispositions {
        assert_eq!(
            disposition.disposition,
            crate::viewer::protocol::PlayerActionDisposition::CompensationRequired
        );
        let compensation = disposition.compensation.expect("accountable case");
        assert!(!compensation.responsible_party.trim().is_empty());
        assert!(!compensation.ticket_reference.trim().is_empty());
        assert!(!compensation.responsible_party.contains("private"));
        assert!(
            !compensation
                .ticket_reference
                .contains("nonce-accountable-compensation")
        );
        assert!(!compensation.ticket_reference.contains("player-private-42"));
    }
}

#[test]
fn missing_action_attribution_is_explicit_fail_closed_census_evidence() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let first = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize");
    let _fork = commit_single_authoritative_batch(&mut server);
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let approval = signed_rollback_envelope(
        &server,
        &first.batch_id,
        "missing-attribution",
        "nonce-missing-attribution",
        &on_call,
        &governance,
    );
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "missing-attribution".to_string(),
                requested_by: None,
                approval: Some(approval),
            },
        })
        .expect("rollback");
    let receipt = ack.rollback_receipt.expect("receipt");
    assert!(!receipt.ready_for_all_clear);
    assert!(
        receipt
            .readiness_blockers
            .iter()
            .any(|blocker| blocker == "player_attribution_incomplete"),
        "missing attribution must be explicit persisted blocker evidence"
    );
}
