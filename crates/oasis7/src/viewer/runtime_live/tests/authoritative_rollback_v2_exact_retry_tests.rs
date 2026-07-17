use super::*;

fn signed_v2_request(
    server: &ViewerRuntimeLiveServer,
    checkpoint_batch_id: &str,
    target_batch_id: &str,
    target_state_root: &str,
    reason: &str,
    nonce: &str,
    on_call: &SigningKey,
    governance: &SigningKey,
) -> AuthoritativeRollbackV2Request {
    let checkpoint = server
        .stable_checkpoints
        .iter()
        .find(|entry| entry.batch_id == checkpoint_batch_id)
        .expect("rollback checkpoint");
    let target = server
        .stable_checkpoints
        .iter()
        .find(|entry| entry.batch_id == target_batch_id)
        .expect("replay target");
    let commitment = crate::runtime::rollback_journal_commitment(
        &checkpoint.snapshot,
        &target.journal,
        target.journal.len(),
    )
    .expect("replay commitment");
    let now_ms = crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms();
    let intent = RollbackIntentV2 {
        schema_version: 2,
        rollback_ticket: "ROLLBACK-V2-EXACT-RETRY-OUTER-REASON".to_string(),
        rollback_checkpoint: RollbackCheckpointRef {
            batch_id: checkpoint_batch_id.to_string(),
            snapshot_hash: compute_runtime_snapshot_hash(&checkpoint.snapshot)
                .expect("checkpoint snapshot hash"),
            snapshot_journal_len: checkpoint.snapshot.journal_len,
        },
        replay_target: RollbackReplayTarget {
            batch_id: target_batch_id.to_string(),
            target_journal_len: target.journal.len(),
            expected_target_state_root: target_state_root.to_string(),
            journal_commitment: commitment,
        },
        expected_reorg_epoch: server.reorg_epoch,
        max_replay_events: usize::MAX,
        max_replay_bytes: usize::MAX,
        reason: reason.to_string(),
        issued_at_ms: now_ms.saturating_sub(1_000),
        expires_at_ms: now_ms.saturating_add(60_000),
        nonce: nonce.to_string(),
    };
    let payload = intent.canonical_signing_payload().expect("v2 payload");
    AuthoritativeRollbackV2Request {
        reason: reason.to_string(),
        approval: RollbackAuthorizationEnvelopeV2 {
            intent,
            signatures: vec![
                RollbackApprovalSignature {
                    authority_id: "on-call-alice".to_string(),
                    role: RollbackAuthorityRole::OnCall,
                    signature_scheme: "ed25519".to_string(),
                    signature_hex: hex::encode(on_call.sign(&payload).to_bytes()),
                },
                RollbackApprovalSignature {
                    authority_id: "governance-bob".to_string(),
                    role: RollbackAuthorityRole::Governance,
                    signature_scheme: "ed25519".to_string(),
                    signature_hex: hex::encode(governance.sign(&payload).to_bytes()),
                },
            ],
        },
    }
}

#[test]
fn committed_v2_exact_retry_rejects_mismatched_outer_request_reason_atomically() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-v2-exact-retry-outer-reason-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let nonce = "nonce-v2-exact-retry-outer-reason";
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    let checkpoint = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(checkpoint.final_height)
        .expect("finalize checkpoint");
    let target = commit_single_authoritative_batch(&mut server);
    server
        .advance_authoritative_batch_finality(target.final_height)
        .expect("finalize replay target");
    let _fork = commit_single_authoritative_batch(&mut server);
    let (on_call, governance) = configure_rollback_authorities(&mut server);
    let request = signed_v2_request(
        &server,
        &checkpoint.batch_id,
        &target.batch_id,
        &target.state_root,
        "signed-v2-reason",
        nonce,
        &on_call,
        &governance,
    );
    let snapshot_before = server.world.snapshot();
    let journal_before = server.world.journal().clone();
    let outcome_before = server.world.rollback_nonce_outcome(nonce);
    let readiness_before = server.rollback_readiness.clone();
    let generation_before = RuntimeWorld::load_authoritative_recovery_generation(&recovery_dir)
        .expect("load generation before retry");

    let mut retry = request;
    retry.reason = "unsigned-outer-reason-mismatch".to_string();
    let error = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RollbackV2 { request: retry })
        .expect_err("v2 request must reject an outer reason outside the signed intent");
    assert_eq!(error.code, "rollback_authorization_invalid");
    assert!(error.message.contains("reason"));
    assert_eq!(server.world.snapshot(), snapshot_before);
    assert_eq!(server.world.journal(), &journal_before);
    assert_eq!(server.world.rollback_nonce_outcome(nonce), outcome_before);
    assert_eq!(server.rollback_readiness, readiness_before);
    assert!(server.get_rollback_receipt(nonce.to_string()).is_err());
    let generation_after = RuntimeWorld::load_authoritative_recovery_generation(&recovery_dir)
        .expect("load generation after retry");
    assert_eq!(generation_after.is_some(), generation_before.is_some());
    let _ = std::fs::remove_dir_all(recovery_dir);
}
