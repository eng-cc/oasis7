use super::*;
use sha2::{Digest, Sha256};

fn install_viewer_recovery_precommit_failure(dir: &std::path::Path) {
    let generation_root = dir.join(".distfs-state/sidecar-generations");
    std::fs::create_dir_all(&generation_root).expect("create generation root");
    std::fs::write(
        generation_root.join(".test-fail-before-index-commit"),
        b"fail",
    )
    .expect("install precommit failpoint");
}

fn persist_session_side_effect_projection(
    recovery_dir: &std::path::Path,
) -> (ViewerRuntimeLiveServer, String, String, Vec<u8>) {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.to_path_buf()));
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("agent");
    let player_id = "player-session-projection".to_string();
    let cached_ack = crate::viewer::AgentChatAck {
        agent_id: agent_id.clone(),
        accepted_at_tick: server.world.state().time,
        message_len: "persisted chat".len(),
        player_id: Some("cached-player".to_string()),
        intent_tick: Some(server.world.state().time),
        intent_seq: Some(77),
        idempotent_replay: false,
    };
    server.llm_sidecar.record_chat_intent_ack(
        "cached-player",
        agent_id.as_str(),
        77,
        Some(server.world.state().time),
        "persisted chat",
        None,
        &cached_ack,
    );
    let (public_key, private_key) = test_signer(96);
    register_runtime_session(
        &mut server,
        player_id.as_str(),
        Some(agent_id.as_str()),
        9,
        public_key.as_str(),
        private_key.as_str(),
    );
    let metadata = RuntimeWorld::load_authoritative_recovery_metadata(recovery_dir)
        .expect("load recovery metadata")
        .expect("committed recovery metadata");
    (server, agent_id, player_id, metadata)
}

fn assert_session_side_effect_projection(
    server: &ViewerRuntimeLiveServer,
    agent_id: &str,
    player_id: &str,
) {
    assert_eq!(
        server.llm_sidecar.agent_player_bindings.get(agent_id),
        Some(&player_id.to_string())
    );
    assert_eq!(
        server.llm_sidecar.player_agent_bindings.get(player_id),
        Some(&agent_id.to_string())
    );
    assert!(
        server
            .llm_sidecar
            .agent_public_key_bindings
            .contains_key(agent_id)
    );
    assert_eq!(
        server.llm_sidecar.player_auth_last_nonce.get(player_id),
        Some(&9)
    );
    assert!(
        server
            .llm_sidecar
            .find_chat_intent_replay(
                "cached-player",
                agent_id,
                77,
                Some(server.world.state().time),
                "persisted chat",
                None,
            )
            .expect("lookup cached chat acknowledgement")
            .is_some()
    );
    assert!(!server.pending_virtual_events.is_empty());
}

#[test]
fn committed_fence_readback_restores_complete_session_side_effect_projection() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-session-projection-readback-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let (mut server, agent_id, player_id, metadata) =
        persist_session_side_effect_projection(&recovery_dir);
    let encoded: serde_json::Value =
        serde_json::from_slice(&metadata).expect("decode recovery envelope");
    assert!(encoded.get("session_side_effects").is_some());

    server.llm_sidecar.agent_player_bindings.clear();
    server.llm_sidecar.player_agent_bindings.clear();
    server.llm_sidecar.agent_public_key_bindings.clear();
    server.llm_sidecar.player_auth_last_nonce.clear();
    server.llm_sidecar.player_chat_intent_acks.clear();
    server.pending_virtual_events.clear();
    server.authoritative_recovery_write_fence =
        Some(hex::encode(Sha256::digest(metadata.as_slice())));
    server
        .resolve_authoritative_recovery_write_fence()
        .expect("resolve committed generation");

    assert_session_side_effect_projection(&server, agent_id.as_str(), player_id.as_str());
    let _ = std::fs::remove_dir_all(recovery_dir);
}

#[test]
fn recovery_restart_restores_complete_session_side_effect_projection() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-session-projection-restart-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let (_source, agent_id, player_id, metadata) =
        persist_session_side_effect_projection(&recovery_dir);
    let mut restarted =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("restarted server");
    restarted.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    restarted.authoritative_recovery_write_fence =
        Some(hex::encode(Sha256::digest(metadata.as_slice())));
    restarted
        .resolve_authoritative_recovery_write_fence()
        .expect("restore committed generation after restart");

    assert_session_side_effect_projection(&restarted, agent_id.as_str(), player_id.as_str());
    let _ = std::fs::remove_dir_all(recovery_dir);
}

#[test]
fn session_metadata_status_unknown_installs_write_fence() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-session-unknown-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("agent");
    let (old_public, old_private) = test_signer(94);
    register_runtime_session(
        &mut server,
        "player-unknown",
        Some(agent_id.as_str()),
        1,
        old_public.as_str(),
        old_private.as_str(),
    );
    let root = recovery_dir.join(".distfs-state/sidecar-generations");
    std::fs::write(root.join(".test-fail-before-index-commit"), b"fail").expect("failpoint");
    std::fs::write(root.join("index.json"), b"not-json").expect("corrupt index");
    let (new_public, _) = test_signer(95);

    let error = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RotateSession {
            request: AuthoritativeSessionRotateRequest {
                player_id: "player-unknown".to_string(),
                old_session_pubkey: old_public.clone(),
                new_session_pubkey: new_public.clone(),
                rotate_reason: "unknown".to_string(),
                rotated_by: Some("security".to_string()),
            },
        })
        .expect_err("status unknown");

    assert_eq!(error.code, "recovery_persistence_status_unknown");
    assert!(
        server.authoritative_recovery_write_fence.is_some(),
        "indeterminate session metadata commit must fence writes"
    );
    assert!(
        server
            .session_policy
            .validate_known_session_key("player-unknown", old_public.as_str())
            .is_ok()
    );
    assert!(
        server
            .session_policy
            .validate_known_session_key("player-unknown", new_public.as_str())
            .is_err()
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
fn session_mutations_roll_back_in_memory_when_recovery_persistence_fails() {
    let register_dir = std::env::temp_dir().join(format!(
        "oasis7-session-register-atomic-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    install_viewer_recovery_precommit_failure(&register_dir);
    let mut register_server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    register_server.set_authoritative_recovery_dir_override(Some(register_dir.clone()));
    let agent_id = register_server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (register_public, register_private) = test_signer(91);
    let register_request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: "player-register-atomic".to_string(),
            public_key: None,
            auth: None,
            requested_agent_id: Some(agent_id.clone()),
            force_rebind: false,
        },
        1,
        register_public.as_str(),
        register_private.as_str(),
    );
    register_server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: register_request,
        })
        .expect_err("register persistence failure");
    assert!(
        register_server
            .session_policy
            .validate_known_session_key("player-register-atomic", register_public.as_str())
            .is_err()
    );
    assert!(
        !register_server
            .llm_sidecar
            .agent_player_bindings
            .contains_key(agent_id.as_str())
    );
    assert!(
        !register_server
            .llm_sidecar
            .player_auth_last_nonce
            .contains_key("player-register-atomic")
    );

    let mutation_dir = std::env::temp_dir().join(format!(
        "oasis7-session-mutation-atomic-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    server.set_authoritative_recovery_dir_override(Some(mutation_dir.clone()));
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (old_public, old_private) = test_signer(92);
    register_runtime_session(
        &mut server,
        "player-mutation-atomic",
        Some(agent_id.as_str()),
        1,
        old_public.as_str(),
        old_private.as_str(),
    );
    let cached_ack = crate::viewer::AgentChatAck {
        agent_id: agent_id.clone(),
        accepted_at_tick: server.world.state().time,
        message_len: "persist me".len(),
        player_id: Some("player-mutation-atomic".to_string()),
        intent_tick: Some(server.world.state().time),
        intent_seq: Some(44),
        idempotent_replay: false,
    };
    server.llm_sidecar.record_chat_intent_ack(
        "player-mutation-atomic",
        agent_id.as_str(),
        44,
        Some(server.world.state().time),
        "persist me",
        Some(old_public.as_str()),
        &cached_ack,
    );
    install_viewer_recovery_precommit_failure(&mutation_dir);
    let (new_public, _) = test_signer(93);
    server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RotateSession {
            request: AuthoritativeSessionRotateRequest {
                player_id: "player-mutation-atomic".to_string(),
                old_session_pubkey: old_public.clone(),
                new_session_pubkey: new_public.clone(),
                rotate_reason: "atomic-rotate".to_string(),
                rotated_by: Some("security".to_string()),
            },
        })
        .expect_err("rotate persistence failure");
    assert!(
        server
            .session_policy
            .validate_known_session_key("player-mutation-atomic", old_public.as_str())
            .is_ok()
    );
    assert!(
        server
            .llm_sidecar
            .find_chat_intent_replay(
                "player-mutation-atomic",
                agent_id.as_str(),
                44,
                Some(server.world.state().time),
                "persist me",
                Some(old_public.as_str()),
            )
            .expect("lookup cached acknowledgement")
            .is_some(),
        "failed rotation must restore chat acknowledgement idempotency state"
    );
    assert!(
        server
            .session_policy
            .validate_known_session_key("player-mutation-atomic", new_public.as_str())
            .is_err()
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_public_key_bindings
            .get(agent_id.as_str())
            .map(String::as_str),
        Some(old_public.as_str())
    );
    assert!(
        !server
            .session_revoke_metadata
            .contains_key(&session_revoke_metadata_key(
                "player-mutation-atomic",
                old_public.as_str()
            ))
    );

    server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RevokeSession {
            request: AuthoritativeSessionRevokeRequest {
                player_id: "player-mutation-atomic".to_string(),
                session_pubkey: Some(old_public.clone()),
                revoke_reason: "atomic-revoke".to_string(),
                revoked_by: Some("security".to_string()),
            },
        })
        .expect_err("revoke persistence failure");
    assert!(
        server
            .session_policy
            .validate_known_session_key("player-mutation-atomic", old_public.as_str())
            .is_ok()
    );
    assert!(
        server
            .llm_sidecar
            .find_chat_intent_replay(
                "player-mutation-atomic",
                agent_id.as_str(),
                44,
                Some(server.world.state().time),
                "persist me",
                Some(old_public.as_str()),
            )
            .expect("lookup cached acknowledgement after revoke failure")
            .is_some(),
        "failed revoke must restore chat acknowledgement idempotency state"
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_player_bindings
            .get(agent_id.as_str())
            .map(String::as_str),
        Some("player-mutation-atomic")
    );
    assert!(
        !server
            .session_revoke_metadata
            .contains_key(&session_revoke_metadata_key(
                "player-mutation-atomic",
                old_public.as_str()
            ))
    );
    let _ = std::fs::remove_dir_all(register_dir);
    let _ = std::fs::remove_dir_all(mutation_dir);
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
