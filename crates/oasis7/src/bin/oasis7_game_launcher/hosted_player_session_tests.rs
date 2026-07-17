use super::*;

#[test]
fn hosted_player_session_issue_returns_structured_grant() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let response = issuer.issue(DeploymentMode::HostedPublicJoin);
    assert!(response.ok);
    assert_eq!(response.error_code, None);
    assert_eq!(response.deployment_mode, "hosted_public_join");
    let grant = response.grant.expect("grant");
    assert!(grant.player_id.starts_with("hosted-player-"));
    assert!(
        grant
            .device_session_id
            .starts_with("hosted-device-session-")
    );
    assert_eq!(grant.auth_mode, "browser_local_ephemeral_ed25519");
    assert_eq!(grant.release_token.len(), 64);
    assert!(
        grant
            .release_token
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    assert_eq!(response.admission.active_player_sessions, 1);
    assert_eq!(response.admission.effective_player_sessions, 1);
    assert_eq!(response.admission.issued_players_total, 1);
    assert_eq!(response.admission.issued_in_current_window, 1);
}

#[test]
fn hosted_player_session_issue_for_player_reuses_stable_player_id() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let first = issuer.issue_for_player(DeploymentMode::HostedPublicJoin, "stable-player-1");
    let second = issuer.issue_for_player(DeploymentMode::HostedPublicJoin, "stable-player-1");
    let first_grant = first.grant.expect("first grant");
    let second_grant = second.grant.expect("second grant");
    assert_eq!(first_grant.player_id, "stable-player-1");
    assert_eq!(second_grant.player_id, "stable-player-1");
    assert_ne!(first_grant.release_token, second_grant.release_token);
}

#[test]
fn hosted_player_session_release_token_is_opaque_256_bit_credential() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let first = issuer
        .issue(DeploymentMode::HostedPublicJoin)
        .grant
        .expect("first grant");
    let second = issuer
        .issue(DeploymentMode::HostedPublicJoin)
        .grant
        .expect("second grant");

    for token in [&first.release_token, &second.release_token] {
        assert_eq!(token.len(), 64, "release token must encode 256 random bits");
        assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(!token.contains("hosted-release"));
    }
    assert_ne!(first.release_token, second.release_token);
}

#[test]
fn hosted_player_session_reassociates_returning_runtime_player_at_capacity_after_restart() {
    let mut restarted = HostedPlayerSessionIssuer::default();
    restarted.observe_runtime_active_players([
        "returning-player",
        "runtime-player-2",
        "runtime-player-3",
        "runtime-player-4",
        "runtime-player-5",
        "runtime-player-6",
        "runtime-player-7",
        "runtime-player-8",
    ]);

    let response = restarted.issue_for_player(DeploymentMode::HostedPublicJoin, "returning-player");
    assert!(
        response.ok,
        "returning player must reclaim its existing runtime slot: {response:?}"
    );
    assert_eq!(
        response.grant.expect("reassociated grant").player_id,
        "returning-player"
    );
    assert_eq!(response.admission.effective_player_sessions, 8);
}

#[test]
fn hosted_player_session_ledger_recovers_lease_rate_and_revocation_after_restart() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-hosted-session-ledger-{}-{}.json",
        std::process::id(),
        now_unix_ms()
    ));
    let grant = {
        let mut issuer =
            HostedPlayerSessionIssuer::with_ledger_path(path.clone()).expect("create ledger");
        issuer
            .issue_for_player(DeploymentMode::HostedPublicJoin, "durable-player")
            .grant
            .expect("issue durable lease")
    };
    let persisted = fs::read_to_string(&path).expect("read persisted ledger");
    assert!(
        !persisted.contains(grant.release_token.as_str()),
        "durable ledger must not persist the bearer credential in plaintext"
    );

    let mut restarted =
        HostedPlayerSessionIssuer::with_ledger_path(path.clone()).expect("reload ledger");
    let admission = restarted.admission(DeploymentMode::HostedPublicJoin);
    assert_eq!(admission.admission.active_player_sessions, 1);
    assert_eq!(admission.admission.issued_in_current_window, 1);
    assert!(
        restarted
            .refresh(
                DeploymentMode::HostedPublicJoin,
                grant.player_id.as_str(),
                grant.release_token.as_str(),
                None,
            )
            .ok
    );
    assert!(
        restarted
            .release(
                DeploymentMode::HostedPublicJoin,
                grant.player_id.as_str(),
                grant.release_token.as_str(),
            )
            .ok
    );

    let mut released =
        HostedPlayerSessionIssuer::with_ledger_path(path.clone()).expect("reload release");
    assert_eq!(
        released
            .admission(DeploymentMode::HostedPublicJoin)
            .admission
            .active_player_sessions,
        0
    );
    assert_eq!(
        released
            .refresh(
                DeploymentMode::HostedPublicJoin,
                grant.player_id.as_str(),
                grant.release_token.as_str(),
                None,
            )
            .error_code
            .as_deref(),
        Some("release_token_invalid")
    );
    let _ = fs::remove_file(path);
}

#[test]
fn hosted_player_session_issue_is_disabled_for_trusted_local_only() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let response = issuer.issue(DeploymentMode::TrustedLocalOnly);
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("player_session_issue_disabled")
    );
    assert!(response.grant.is_none());
}

#[test]
fn hosted_player_session_issue_enforces_max_player_sessions() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    for _ in 0..8 {
        let response = issuer.issue(DeploymentMode::HostedPublicJoin);
        assert!(response.ok);
    }
    let response = issuer.issue(DeploymentMode::HostedPublicJoin);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("world_full"));
    assert_eq!(response.admission.active_player_sessions, 8);
    assert_eq!(response.admission.effective_player_sessions, 8);
}

#[test]
fn hosted_player_session_issue_counts_runtime_only_occupancy_toward_world_full() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    issuer.observe_runtime_active_players([
        "runtime-player-1",
        "runtime-player-2",
        "runtime-player-3",
        "runtime-player-4",
        "runtime-player-5",
        "runtime-player-6",
        "runtime-player-7",
        "runtime-player-8",
    ]);

    let response = issuer.issue(DeploymentMode::HostedPublicJoin);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("world_full"));
    assert_eq!(response.admission.active_player_sessions, 0);
    assert_eq!(response.admission.runtime_bound_player_sessions, 8);
    assert_eq!(response.admission.runtime_only_player_sessions, 8);
    assert_eq!(response.admission.effective_player_sessions, 8);
}

#[test]
fn hosted_player_session_release_frees_active_slot() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");
    let release = issuer.release(
        DeploymentMode::HostedPublicJoin,
        grant.player_id.as_str(),
        grant.release_token.as_str(),
    );
    assert!(release.ok);
    assert_eq!(release.admission.active_player_sessions, 0);
    assert_eq!(release.admission.released_players_total, 1);
}

#[test]
fn hosted_player_session_admission_reports_current_snapshot() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let _ = issuer.issue(DeploymentMode::HostedPublicJoin);
    let response = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert!(response.ok);
    assert_eq!(response.admission.active_player_sessions, 1);
    assert_eq!(response.admission.effective_player_sessions, 1);
    assert_eq!(response.admission.max_player_sessions, 8);
    assert_eq!(response.admission.runtime_bound_player_sessions, 0);
    assert_eq!(response.admission.runtime_only_player_sessions, 0);
    assert_eq!(response.admission.runtime_probe_status, "not_started");
    assert_eq!(response.admission.runtime_probe_error, None);
    assert_eq!(response.admission.slot_lease_ttl_ms, SLOT_LEASE_TTL_MS);
    assert_eq!(
        response.admission.pending_registration_ttl_ms,
        PENDING_REGISTRATION_TTL_MS
    );
}

#[test]
fn hosted_player_session_refresh_keeps_slot_alive() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let token = issue.grant.expect("grant").release_token;
    let response = issuer.refresh(
        DeploymentMode::HostedPublicJoin,
        "hosted-player-test",
        token.as_str(),
        None,
    );
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("player_id_mismatch"));

    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");
    let response = issuer.refresh(
        DeploymentMode::HostedPublicJoin,
        grant.player_id.as_str(),
        grant.release_token.as_str(),
        None,
    );
    assert!(response.ok);
    assert_eq!(response.admission.active_player_sessions, 1);
    assert_eq!(response.admission.effective_player_sessions, 1);
}

#[test]
fn hosted_player_session_refresh_rotates_registration_grant_for_new_browser_key() {
    let issuer_private_key = [71_u8; 32];
    let browser_key = ed25519_dalek::SigningKey::from_bytes(&[72_u8; 32]);
    unsafe {
        std::env::set_var(
            oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV,
            hex::encode(issuer_private_key),
        );
    }
    let mut issuer = HostedPlayerSessionIssuer::default();
    let grant = issuer
        .issue_for_player(
            DeploymentMode::HostedPublicJoin,
            "hosted-player-account-refresh",
        )
        .grant
        .expect("initial lease");

    let response = issuer.refresh(
        DeploymentMode::HostedPublicJoin,
        grant.player_id.as_str(),
        grant.release_token.as_str(),
        Some(hex::encode(browser_key.verifying_key().to_bytes()).as_str()),
    );

    unsafe {
        std::env::remove_var(oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV);
    }
    assert!(
        response.ok,
        "refresh must rotate registration grant: {response:?}"
    );
    assert!(response.registration_grant.is_some());
    assert!(response.device_session_id.is_some());
}

#[test]
fn hosted_player_session_release_requires_matching_player_id() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");

    let missing_player_id = issuer.release(
        DeploymentMode::HostedPublicJoin,
        "",
        grant.release_token.as_str(),
    );
    assert!(!missing_player_id.ok);
    assert_eq!(
        missing_player_id.error_code.as_deref(),
        Some("player_id_required")
    );

    let mismatch = issuer.release(
        DeploymentMode::HostedPublicJoin,
        "hosted-player-other",
        grant.release_token.as_str(),
    );
    assert!(!mismatch.ok);
    assert_eq!(mismatch.error_code.as_deref(), Some("player_id_mismatch"));

    let ok = issuer.release(
        DeploymentMode::HostedPublicJoin,
        grant.player_id.as_str(),
        grant.release_token.as_str(),
    );
    assert!(ok.ok);
}

#[test]
fn hosted_player_session_runtime_reconcile_releases_seen_players_missing_from_runtime() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");

    issuer.observe_runtime_active_players([grant.player_id.as_str()]);
    let admission = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert_eq!(admission.admission.active_player_sessions, 1);
    assert_eq!(admission.admission.effective_player_sessions, 1);
    assert_eq!(admission.admission.runtime_bound_player_sessions, 1);
    assert_eq!(admission.admission.runtime_only_player_sessions, 0);
    assert_eq!(admission.admission.runtime_probe_status, "ok");

    issuer.observe_runtime_active_players(std::iter::empty::<&str>());
    let admission = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert_eq!(admission.admission.active_player_sessions, 0);
    assert_eq!(admission.admission.effective_player_sessions, 0);
    assert_eq!(admission.admission.runtime_bound_player_sessions, 0);
    assert_eq!(admission.admission.runtime_only_player_sessions, 0);
    assert_eq!(admission.admission.released_players_total, 1);

    let refresh = issuer.refresh(
        DeploymentMode::HostedPublicJoin,
        grant.player_id.as_str(),
        grant.release_token.as_str(),
        None,
    );
    assert!(!refresh.ok);
    assert_eq!(refresh.error_code.as_deref(), Some("session_revoked"));
}

#[test]
fn hosted_player_session_runtime_probe_failure_surfaces_in_admission() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    issuer.record_runtime_probe_failure("connect runtime live failed".to_string());
    let response = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert_eq!(response.admission.runtime_probe_status, "error");
    assert_eq!(
        response.admission.runtime_probe_error.as_deref(),
        Some("connect runtime live failed")
    );
    assert!(response.admission.last_runtime_probe_unix_ms.is_some());
}

#[test]
fn hosted_player_session_admission_reports_runtime_only_occupancy_separately() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");
    issuer.observe_runtime_active_players([grant.player_id.as_str(), "runtime-player-extra"]);

    let response = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert!(response.ok);
    assert_eq!(response.admission.active_player_sessions, 1);
    assert_eq!(response.admission.runtime_bound_player_sessions, 2);
    assert_eq!(response.admission.runtime_only_player_sessions, 1);
    assert_eq!(response.admission.effective_player_sessions, 2);
}

#[test]
fn hosted_player_session_pending_registration_slots_expire_before_full_lease_ttl() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");
    let token = grant.release_token;
    let stale_seen_at = now_unix_ms()
        .saturating_sub(PENDING_REGISTRATION_TTL_MS)
        .saturating_sub(1);
    issuer
        .last_seen_unix_ms_by_release_token
        .insert(release_token_digest(token.as_str()), stale_seen_at);

    let response = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert!(response.ok);
    assert_eq!(response.admission.active_player_sessions, 0);
    assert_eq!(response.admission.effective_player_sessions, 0);
    assert_eq!(response.admission.released_players_total, 1);
}

#[test]
fn hosted_player_session_runtime_seen_slots_keep_full_lease_ttl() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");
    issuer.observe_runtime_active_players([grant.player_id.as_str()]);
    let token = grant.release_token;
    let still_alive_seen_at = now_unix_ms()
        .saturating_sub(PENDING_REGISTRATION_TTL_MS)
        .saturating_sub(1);
    issuer
        .last_seen_unix_ms_by_release_token
        .insert(release_token_digest(token.as_str()), still_alive_seen_at);

    let response = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert!(response.ok);
    assert_eq!(response.admission.active_player_sessions, 1);
    assert_eq!(response.admission.effective_player_sessions, 1);
}

#[test]
fn hosted_player_session_runtime_probe_refreshes_runtime_bound_slot_before_expiry_prune() {
    let mut issuer = HostedPlayerSessionIssuer::default();
    let issue = issuer.issue(DeploymentMode::HostedPublicJoin);
    let grant = issue.grant.expect("grant");
    issuer.observe_runtime_active_players([grant.player_id.as_str()]);

    let stale_seen_at = now_unix_ms()
        .saturating_sub(SLOT_LEASE_TTL_MS)
        .saturating_sub(1);
    issuer.last_seen_unix_ms_by_release_token.insert(
        release_token_digest(grant.release_token.as_str()),
        stale_seen_at,
    );

    issuer.observe_runtime_active_players([grant.player_id.as_str()]);

    let admission = issuer.admission(DeploymentMode::HostedPublicJoin);
    assert!(admission.ok);
    assert_eq!(admission.admission.active_player_sessions, 1);
    assert_eq!(admission.admission.runtime_bound_player_sessions, 1);
    assert_eq!(admission.admission.runtime_only_player_sessions, 0);
    assert_eq!(admission.admission.effective_player_sessions, 1);
    assert_eq!(admission.admission.released_players_total, 0);

    let refresh = issuer.refresh(
        DeploymentMode::HostedPublicJoin,
        grant.player_id.as_str(),
        grant.release_token.as_str(),
        None,
    );
    assert!(refresh.ok);
}
