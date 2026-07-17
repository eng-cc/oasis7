use super::*;

#[cfg(unix)]
#[test]
fn hosted_registration_persist_failure_rolls_back_binding_and_keeps_grant_retryable() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = lock_test_llm_env();
    let replay_dir = runtime_live_temp_dir("hosted-grant-persist-failure");
    std::fs::create_dir_all(&replay_dir).expect("create replay directory");
    let replay_ledger = replay_dir.join("registration-replay.json");
    std::fs::write(&replay_ledger, b"[]").expect("seed readable replay ledger");
    std::fs::set_permissions(&replay_dir, std::fs::Permissions::from_mode(0o500))
        .expect("make replay directory unwritable");

    let issuer_private_key = hex::encode([94_u8; 32]);
    let issuer_public_key =
        crate::viewer::derive_hosted_registration_issuer_public_key(issuer_private_key.as_str())
            .expect("derive issuer public key");
    unsafe {
        oasis7::env_mut::set_var(
            crate::viewer::HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV,
            issuer_public_key,
        );
        oasis7::env_mut::set_var(
            crate::viewer::HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV,
            replay_ledger.as_os_str(),
        );
    }

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let hosted_player_id = "hosted-player-account-persist-failure";
    let (hosted_public_key, hosted_private_key) = test_signer(95);
    let registration_grant = crate::viewer::issue_hosted_registration_grant(
        hosted_player_id,
        hosted_public_key.as_str(),
        "device-persist-failure",
        "nonce-persist-failure",
        test_now_unix_ms(),
        issuer_private_key.as_str(),
    )
    .expect("issue registration grant");
    let request = |nonce| {
        signed_session_register_request(
            crate::viewer::AuthoritativeSessionRegisterRequest {
                player_id: hosted_player_id.to_string(),
                public_key: None,
                registration_grant: Some(registration_grant.clone()),
                auth: None,
                requested_agent_id: Some(agent_id.clone()),
                force_rebind: false,
            },
            nonce,
            hosted_public_key.as_str(),
            hosted_private_key.as_str(),
        )
    };

    let error = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: request(1),
        })
        .expect_err("replay-ledger persistence must fail");
    assert!(
        error.message.contains("registration replay ledger"),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        server.llm_sidecar.bound_agent_for_player(hosted_player_id),
        None,
        "failed registration must roll back the committed binding"
    );
    assert!(
        server
            .session_policy
            .validate_known_session_key(hosted_player_id, hosted_public_key.as_str())
            .is_err(),
        "failed registration must not commit the active session key"
    );
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get(hosted_player_id),
        None,
        "failed registration must not consume the request auth nonce"
    );

    std::fs::set_permissions(&replay_dir, std::fs::Permissions::from_mode(0o700))
        .expect("restore replay directory permissions");
    let _ = std::fs::remove_file(&replay_ledger);
    std::fs::write(&replay_ledger, b"[]").expect("restore writable replay ledger");
    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: request(2),
        })
        .expect("the same grant must remain retryable after rollback");
    assert_eq!(ack.status, AuthoritativeRecoveryStatus::SessionRegistered);

    unsafe {
        oasis7::env_mut::remove_var(crate::viewer::HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV);
        oasis7::env_mut::remove_var(crate::viewer::HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV);
    }
    let _ = std::fs::remove_dir_all(replay_dir);
}

#[test]
fn hosted_registration_grant_rotates_reload_key_without_requiring_login() {
    let _guard = lock_test_llm_env();
    let replay_dir = runtime_live_temp_dir("hosted-grant-reload-key");
    let replay_ledger = replay_dir.join("registration-replay.json");
    let issuer_private_key = hex::encode([96_u8; 32]);
    let issuer_public_key =
        crate::viewer::derive_hosted_registration_issuer_public_key(issuer_private_key.as_str())
            .expect("derive issuer public key");
    unsafe {
        oasis7::env_mut::set_var(
            crate::viewer::HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV,
            issuer_public_key,
        );
        oasis7::env_mut::set_var(
            crate::viewer::HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV,
            replay_ledger.as_os_str(),
        );
    }

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_ids: Vec<_> = server
        .world
        .state()
        .agents
        .keys()
        .cloned()
        .take(2)
        .collect();
    assert_eq!(agent_ids.len(), 2, "two bases must seed two agents");
    let agent_id = agent_ids[0].clone();
    let other_agent_id = agent_ids[1].clone();
    let hosted_player_id = "hosted-player-account-reload-key";
    let (old_public_key, _) = test_signer(97);
    let (new_public_key, new_private_key) = test_signer(98);

    let initial_session_plan = server
        .session_policy
        .validate_session_registration(hosted_player_id, old_public_key.as_str(), false)
        .expect("plan pre-reload session");
    server
        .session_policy
        .commit_session_registration(initial_session_plan);
    server
        .llm_sidecar
        .bind_agent_player(
            agent_id.as_str(),
            hosted_player_id,
            Some(old_public_key.as_str()),
            false,
        )
        .expect("seed pre-reload player binding");

    let registration_grant = crate::viewer::issue_hosted_registration_grant(
        hosted_player_id,
        new_public_key.as_str(),
        "device-reload-key",
        "nonce-reload-key",
        test_now_unix_ms(),
        issuer_private_key.as_str(),
    )
    .expect("issue refreshed registration grant");
    let request = |requested_agent_id: &str, auth_nonce| {
        signed_session_register_request(
            crate::viewer::AuthoritativeSessionRegisterRequest {
                player_id: hosted_player_id.to_string(),
                public_key: None,
                registration_grant: Some(registration_grant.clone()),
                auth: None,
                requested_agent_id: Some(requested_agent_id.to_string()),
                force_rebind: false,
            },
            auth_nonce,
            new_public_key.as_str(),
            new_private_key.as_str(),
        )
    };

    let unauthorized_rebind = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: request(other_agent_id.as_str(), 2),
        })
        .expect_err("reload key rotation must not imply permission to change agents");
    assert_eq!(unauthorized_rebind.code, "player_bind_failed");
    assert!(
        !replay_ledger.exists(),
        "rejected agent change must not consume the reload grant"
    );

    let (ack, _) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: request(agent_id.as_str(), 3),
        })
        .expect("authenticated refresh grant should rotate the browser reload key");
    assert_eq!(ack.status, AuthoritativeRecoveryStatus::SessionRegistered);
    assert_eq!(ack.player_id.as_deref(), Some(hosted_player_id));
    assert_eq!(ack.agent_id.as_deref(), Some(agent_id.as_str()));
    assert!(
        server
            .session_policy
            .validate_known_session_key(hosted_player_id, old_public_key.as_str())
            .is_err(),
        "the pre-reload key must no longer authorize the hosted session"
    );
    assert!(
        server
            .session_policy
            .validate_known_session_key(hosted_player_id, new_public_key.as_str())
            .is_ok(),
        "the refreshed browser key must become the active session key"
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_public_key_bindings
            .get(agent_id.as_str())
            .map(String::as_str),
        Some(new_public_key.as_str()),
        "the existing player/agent binding must rotate to the refreshed browser key"
    );

    let replay_error = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: request(agent_id.as_str(), 4),
        })
        .expect_err("a committed reload grant must remain single use");
    assert_eq!(replay_error.code, "auth_invalid");
    assert!(replay_error.message.contains("registration grant replay"));

    unsafe {
        oasis7::env_mut::remove_var(crate::viewer::HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV);
        oasis7::env_mut::remove_var(crate::viewer::HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV);
    }
    let _ = std::fs::remove_dir_all(replay_dir);
}

#[test]
fn runtime_agent_chat_rejects_intent_seq_conflict_on_payload_change() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    seed_agent_chat_oc(&mut server, agent_id.as_str());
    let (public_key, private_key) = test_signer(22);
    let first = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(10),
            intent_seq: Some(6),
        },
        6,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        5,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    server
        .handle_agent_chat(first)
        .expect("first request accepted");

    let conflict = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "changed".to_string(),
            intent_tick: Some(10),
            intent_seq: Some(6),
        },
        6,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_agent_chat(conflict)
        .expect_err("same seq with different payload must fail");
    assert_eq!(err.code, "intent_seq_conflict");
}

#[test]
fn runtime_agent_chat_rejects_intent_seq_nonce_mismatch() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(23);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(3),
            intent_seq: Some(8),
        },
        9,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        8,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let err = server
        .handle_agent_chat(request)
        .expect_err("intent seq mismatch should fail");
    assert_eq!(err.code, "intent_seq_invalid");
}

#[test]
fn runtime_authoritative_recovery_rotate_and_revoke_session_enforced_for_agent_chat() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    seed_agent_chat_oc(&mut server, agent_id.as_str());
    let (public_key_v1, private_key_v1) = test_signer(31);
    let (public_key_v2, private_key_v2) = test_signer(32);

    let first_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(1),
            intent_seq: Some(2),
        },
        2,
        public_key_v1.as_str(),
        private_key_v1.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        public_key_v1.as_str(),
        private_key_v1.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(register_ack.agent_id.as_deref(), Some(agent_id.as_str()));
    let _ = server
        .handle_agent_chat(first_request)
        .expect("first key should be accepted");

    let (rotate_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RotateSession {
            request: AuthoritativeSessionRotateRequest {
                player_id: "player-a".to_string(),
                old_session_pubkey: public_key_v1.clone(),
                new_session_pubkey: public_key_v2.clone(),
                rotate_reason: "security_rotation".to_string(),
                rotated_by: Some("ops".to_string()),
            },
        })
        .expect("rotate session");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        rotate_ack.status,
        AuthoritativeRecoveryStatus::SessionRotated
    );
    assert_eq!(
        rotate_ack.session_pubkey.as_deref(),
        Some(public_key_v1.as_str())
    );
    assert_eq!(
        rotate_ack.replaced_by_pubkey.as_deref(),
        Some(public_key_v2.as_str())
    );

    let stale_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "stale".to_string(),
            intent_tick: Some(2),
            intent_seq: Some(2),
        },
        2,
        public_key_v1.as_str(),
        private_key_v1.as_str(),
    );
    let stale_err = server
        .handle_agent_chat(stale_request)
        .expect_err("old key should be rejected after rotation");
    assert_eq!(stale_err.code, "session_revoked");

    let rotated_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "rotated".to_string(),
            intent_tick: Some(3),
            intent_seq: Some(1),
        },
        1,
        public_key_v2.as_str(),
        private_key_v2.as_str(),
    );
    let _ = server
        .handle_agent_chat(rotated_request)
        .expect("new key should be accepted");

    let (revoke_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RevokeSession {
            request: AuthoritativeSessionRevokeRequest {
                player_id: "player-a".to_string(),
                session_pubkey: Some(public_key_v2.clone()),
                revoke_reason: "compromised".to_string(),
                revoked_by: Some("ops".to_string()),
            },
        })
        .expect("revoke session");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        revoke_ack.status,
        AuthoritativeRecoveryStatus::SessionRevoked
    );
    assert_eq!(revoke_ack.revoke_reason.as_deref(), Some("compromised"));
    assert_eq!(revoke_ack.revoked_by.as_deref(), Some("ops"));

    let revoked_reconnect_err = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: "player-a".to_string(),
                session_pubkey: Some(public_key_v2.clone()),
                last_known_log_cursor: None,
                expected_reorg_epoch: None,
            },
        })
        .expect_err("reconnect should surface revoke metadata");
    assert_eq!(revoked_reconnect_err.code, "session_revoked");
    assert_eq!(
        revoked_reconnect_err.revoke_reason.as_deref(),
        Some("compromised")
    );
    assert_eq!(revoked_reconnect_err.revoked_by.as_deref(), Some("ops"));

    let revoked_register_request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: "player-a".to_string(),
            public_key: None,
            registration_grant: None,
            auth: None,
            requested_agent_id: Some(agent_id.clone()),
            force_rebind: false,
        },
        5,
        public_key_v2.as_str(),
        private_key_v2.as_str(),
    );
    let revoked_register_err = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: revoked_register_request,
        })
        .expect_err("register should surface revoke metadata");
    assert_eq!(revoked_register_err.code, "session_revoked");
    assert_eq!(
        revoked_register_err.revoke_reason.as_deref(),
        Some("compromised")
    );
    assert_eq!(revoked_register_err.revoked_by.as_deref(), Some("ops"));

    let revoked_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "revoked".to_string(),
            intent_tick: Some(4),
            intent_seq: Some(2),
        },
        2,
        public_key_v2.as_str(),
        private_key_v2.as_str(),
    );
    let revoked_err = server
        .handle_agent_chat(revoked_request)
        .expect_err("revoked key should be rejected");
    assert_eq!(revoked_err.code, "session_revoked");
}
