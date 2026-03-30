use super::*;
use crate::viewer::runtime_live::authoritative::{
    is_valid_root_hash, RuntimeBatchChallengeState,
};

fn commit_single_authoritative_batch(
    server: &mut ViewerRuntimeLiveServer,
) -> AuthoritativeBatchFinality {
    let journal_start = server.world.journal().events.len();
    server.script.enqueue(&mut server.world);
    server.world.step().expect("runtime step");

    let mut mapped_events = Vec::new();
    for runtime_event in &server.world.journal().events[journal_start..] {
        mapped_events.push(map_runtime_event(runtime_event, &server.snapshot_config));
    }
    mapped_events.extend(server.pending_virtual_events.drain(..));

    server
        .register_authoritative_batch(mapped_events.as_slice())
        .expect("register authoritative batch")
}

#[test]
fn runtime_authoritative_batch_commit_records_required_roots() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let batch = commit_single_authoritative_batch(&mut server);
    assert_eq!(batch.finality_state, AuthoritativeFinalityState::Pending);
    assert!(!batch.batch_id.is_empty());
    assert!(is_valid_root_hash(batch.state_root.as_str()));
    assert!(is_valid_root_hash(batch.data_root.as_str()));
    assert_eq!(server.authoritative_batches.len(), 1);
}

#[test]
fn runtime_authoritative_batch_finality_is_monotonic_and_final_only_gates_settlement() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    assert!(!pending.settlement_ready);
    assert!(!pending.ranking_ready);

    let confirmed_updates = server
        .advance_authoritative_batch_finality(pending.confirm_height)
        .expect("advance to confirmed");
    assert_eq!(confirmed_updates.len(), 1);
    let confirmed = &confirmed_updates[0];
    assert_eq!(
        confirmed.finality_state,
        AuthoritativeFinalityState::Confirmed
    );
    assert!(!confirmed.settlement_ready);
    assert!(!confirmed.ranking_ready);

    let final_updates = server
        .advance_authoritative_batch_finality(pending.final_height)
        .expect("advance to final");
    assert_eq!(final_updates.len(), 1);
    let final_update = &final_updates[0];
    assert_eq!(
        final_update.finality_state,
        AuthoritativeFinalityState::Final
    );
    assert!(final_update.settlement_ready);
    assert!(final_update.ranking_ready);

    let no_regression = server
        .advance_authoritative_batch_finality(pending.confirm_height)
        .expect("finality should be monotonic");
    assert!(no_regression.is_empty());
    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_eq!(stored.finality_state, AuthoritativeFinalityState::Final);
}

#[test]
fn runtime_authoritative_batch_data_root_mismatch_blocks_confirmation() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let tampered_root = "f".repeat(64);
    let batch = server
        .authoritative_batches
        .back_mut()
        .expect("stored batch for tamper");
    batch.data_root = tampered_root;

    let updates = server
        .advance_authoritative_batch_finality(pending.final_height.saturating_add(10))
        .expect("advance finality");
    assert!(updates.is_empty());

    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_eq!(stored.finality_state, AuthoritativeFinalityState::Pending);
    let wire = stored.as_wire(&server.settlement_ranking_gate);
    assert!(!wire.settlement_ready);
    assert!(!wire.ranking_ready);
}

#[test]
fn runtime_authoritative_challenge_submit_opens_challenge_and_blocks_finality_progress() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (_, maybe_batch_update) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id.clone(),
                watcher_id: "watcher-1".to_string(),
                recomputed_state_root: pending.state_root.clone(),
                recomputed_data_root: pending.data_root.clone(),
                challenge_id: Some("challenge-1".to_string()),
            },
        })
        .expect("submit challenge");
    let batch_update = maybe_batch_update.expect("batch update");
    assert!(batch_update.challenge_open);
    assert_eq!(
        batch_update.active_challenge_id.as_deref(),
        Some("challenge-1")
    );

    let updates = server
        .advance_authoritative_batch_finality(pending.final_height.saturating_add(10))
        .expect("advance while challenged");
    assert!(updates.is_empty());
    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_ne!(stored.finality_state, AuthoritativeFinalityState::Final);
}

#[test]
fn runtime_authoritative_challenge_resolve_no_fraud_unblocks_finality_without_slash() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (submit_ack, _) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id.clone(),
                watcher_id: "watcher-2".to_string(),
                recomputed_state_root: pending.state_root.clone(),
                recomputed_data_root: pending.data_root.clone(),
                challenge_id: None,
            },
        })
        .expect("submit challenge");
    assert_eq!(submit_ack.status, AuthoritativeChallengeStatus::Challenged);

    let (resolve_ack, maybe_batch_update) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id.clone(),
                resolved_by: Some("arbiter-1".to_string()),
            },
        })
        .expect("resolve challenge");
    assert_eq!(
        resolve_ack.status,
        AuthoritativeChallengeStatus::ResolvedNoFraud
    );
    assert!(!resolve_ack.slash_applied);
    let batch_update = maybe_batch_update.expect("batch update");
    assert!(!batch_update.challenge_open);
    assert!(!batch_update.slashed);

    let final_updates = server
        .advance_authoritative_batch_finality(pending.final_height)
        .expect("advance after resolve");
    assert!(final_updates.iter().any(|update| {
        update.batch_id == pending.batch_id
            && update.finality_state == AuthoritativeFinalityState::Final
            && !update.slashed
    }));
}

#[test]
fn runtime_authoritative_challenge_resolve_fraud_slashes_and_blocks_finality() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (submit_ack, _) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id.clone(),
                watcher_id: "watcher-3".to_string(),
                recomputed_state_root: "f".repeat(64),
                recomputed_data_root: pending.data_root.clone(),
                challenge_id: None,
            },
        })
        .expect("submit challenge");

    let (resolve_ack, maybe_batch_update) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id,
                resolved_by: Some("arbiter-1".to_string()),
            },
        })
        .expect("resolve challenge");
    assert_eq!(
        resolve_ack.status,
        AuthoritativeChallengeStatus::ResolvedFraudSlashed
    );
    assert!(resolve_ack.slash_applied);
    assert_eq!(
        resolve_ack.slash_reason.as_deref(),
        Some("state_root_mismatch")
    );
    let batch_update = maybe_batch_update.expect("batch update");
    assert!(batch_update.slashed);
    assert!(!batch_update.challenge_open);

    let updates = server
        .advance_authoritative_batch_finality(pending.final_height.saturating_add(10))
        .expect("advance after slash");
    assert!(updates
        .iter()
        .all(|update| update.batch_id != pending.batch_id));
    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_eq!(
        stored.challenge_state,
        RuntimeBatchChallengeState::ResolvedFraudSlashed
    );
    assert_ne!(stored.finality_state, AuthoritativeFinalityState::Final);
}

#[test]
fn runtime_authoritative_challenge_duplicate_resolve_is_rejected() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (submit_ack, _) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id,
                watcher_id: "watcher-4".to_string(),
                recomputed_state_root: pending.state_root,
                recomputed_data_root: pending.data_root,
                challenge_id: Some("challenge-dup".to_string()),
            },
        })
        .expect("submit challenge");
    let _ = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id.clone(),
                resolved_by: None,
            },
        })
        .expect("first resolve");

    let err = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id,
                resolved_by: None,
            },
        })
        .expect_err("duplicate resolve should reject");
    assert_eq!(err.code, "challenge_already_resolved");
}

#[test]
fn runtime_authoritative_recovery_rollback_prunes_fork_batches() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let first = commit_single_authoritative_batch(&mut server);
    let updates = server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize first batch");
    assert!(updates.iter().any(|batch| {
        batch.batch_id == first.batch_id
            && batch.finality_state == AuthoritativeFinalityState::Final
    }));
    assert_eq!(server.stable_checkpoints.len(), 1);

    let second = commit_single_authoritative_batch(&mut server);
    assert_eq!(server.authoritative_batches.len(), 2);
    assert_eq!(server.authoritative_batches[1].batch_id, second.batch_id);

    let (ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id.clone()),
                reason: "test_reorg".to_string(),
                requested_by: Some("ops".to_string()),
            },
        })
        .expect("rollback to first stable batch");
    assert!(emit_snapshot_after_ack);
    assert_eq!(ack.status, AuthoritativeRecoveryStatus::RolledBack);
    assert_eq!(
        ack.stable_batch_id.as_deref(),
        Some(first.batch_id.as_str())
    );
    assert_eq!(server.reorg_epoch, 1);
    assert_eq!(server.authoritative_batches.len(), 1);
    assert_eq!(server.authoritative_batches[0].batch_id, first.batch_id);
}

#[test]
fn runtime_authoritative_recovery_reconnect_detects_reorg_epoch_mismatch() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let first = commit_single_authoritative_batch(&mut server);
    let _ = server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize first batch");
    let initial_cursor = latest_runtime_event_seq(&server.world);

    let (initial_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: "player-a".to_string(),
                session_pubkey: None,
                last_known_log_cursor: Some(initial_cursor),
                expected_reorg_epoch: Some(0),
            },
        })
        .expect("initial reconnect sync");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        initial_ack.status,
        AuthoritativeRecoveryStatus::CatchUpReady
    );
    assert_eq!(initial_ack.message.as_deref(), Some("delta_replay_allowed"));

    let _ = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "force_reorg".to_string(),
                requested_by: None,
            },
        })
        .expect("rollback");
    assert_eq!(server.reorg_epoch, 1);

    let (stale_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: "player-a".to_string(),
                session_pubkey: None,
                last_known_log_cursor: Some(initial_cursor),
                expected_reorg_epoch: Some(0),
            },
        })
        .expect("stale reconnect sync");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(stale_ack.status, AuthoritativeRecoveryStatus::CatchUpReady);
    assert!(stale_ack
        .message
        .as_deref()
        .is_some_and(|message| message.contains("snapshot_reload_required")));
}
