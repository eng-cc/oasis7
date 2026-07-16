use super::*;

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
fn rollback_player_action_census_deduplicates_multi_event_actions() {
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
