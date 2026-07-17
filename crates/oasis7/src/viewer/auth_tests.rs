use super::*;

fn test_signer_with_seed(seed: u8) -> (String, String) {
    let private_key = [seed; 32];
    let signing_key = SigningKey::from_bytes(&private_key);
    (
        hex::encode(signing_key.verifying_key().to_bytes()),
        hex::encode(private_key),
    )
}

fn test_signer() -> (String, String) {
    test_signer_with_seed(7)
}

#[test]
fn prompt_control_apply_auth_sign_and_verify_roundtrip() {
    let (public_key, private_key) = test_signer();
    let request = PromptControlApplyRequest {
        agent_id: "agent-0".to_string(),
        player_id: "player-a".to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        expected_version: Some(3),
        updated_by: Some("player-a".to_string()),
        system_prompt_override: Some(Some("system".to_string())),
        short_term_goal_override: Some(None),
        long_term_goal_override: None,
    };
    let proof = sign_prompt_control_apply_auth_proof(
        PromptControlAuthIntent::Apply,
        &request,
        11,
        public_key.as_str(),
        private_key.as_str(),
    )
    .expect("sign proof");
    let verified =
        verify_prompt_control_apply_auth_proof(PromptControlAuthIntent::Apply, &request, &proof)
            .expect("verify proof");
    assert_eq!(verified.player_id, "player-a");
    assert_eq!(verified.public_key, public_key);
    assert_eq!(verified.nonce, 11);
}

#[test]
fn prompt_control_apply_auth_verify_rejects_tamper() {
    let (public_key, private_key) = test_signer();
    let request = PromptControlApplyRequest {
        agent_id: "agent-0".to_string(),
        player_id: "player-a".to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        expected_version: Some(3),
        updated_by: Some("player-a".to_string()),
        system_prompt_override: Some(Some("system".to_string())),
        short_term_goal_override: None,
        long_term_goal_override: None,
    };
    let proof = sign_prompt_control_apply_auth_proof(
        PromptControlAuthIntent::Apply,
        &request,
        12,
        public_key.as_str(),
        private_key.as_str(),
    )
    .expect("sign proof");

    let mut tampered = request.clone();
    tampered.system_prompt_override = Some(Some("tampered".to_string()));
    let err =
        verify_prompt_control_apply_auth_proof(PromptControlAuthIntent::Apply, &tampered, &proof)
            .expect_err("tampered payload must fail");
    assert!(err.contains("verify auth signature failed"));
}

#[test]
fn hosted_prompt_control_strong_auth_grant_roundtrip() {
    let (player_public_key, _) = test_signer();
    let (backend_public_key, backend_private_key) = test_signer_with_seed(9);
    let request = PromptControlApplyRequest {
        agent_id: "agent-0".to_string(),
        player_id: "player-a".to_string(),
        public_key: Some(player_public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        expected_version: Some(3),
        updated_by: Some("player-a".to_string()),
        system_prompt_override: Some(Some("system".to_string())),
        short_term_goal_override: None,
        long_term_goal_override: None,
    };
    let grant = sign_hosted_prompt_control_strong_auth_grant(
        "prompt_control_apply",
        "player-a",
        player_public_key.as_str(),
        "agent-0",
        100,
        200,
        backend_public_key.as_str(),
        backend_private_key.as_str(),
    )
    .expect("sign hosted strong-auth grant");
    verify_hosted_prompt_control_apply_strong_auth_grant(
        PromptControlAuthIntent::Apply,
        &request,
        &grant,
        backend_public_key.as_str(),
        150,
    )
    .expect("verify hosted strong-auth grant");
}

#[test]
fn hosted_prompt_control_strong_auth_grant_rejects_request_mismatch() {
    let (player_public_key, _) = test_signer();
    let (backend_public_key, backend_private_key) = test_signer_with_seed(10);
    let request = PromptControlApplyRequest {
        agent_id: "agent-0".to_string(),
        player_id: "player-a".to_string(),
        public_key: Some(player_public_key.clone()),
        auth: None,
        strong_auth_grant: None,
        expected_version: Some(3),
        updated_by: Some("player-a".to_string()),
        system_prompt_override: Some(Some("system".to_string())),
        short_term_goal_override: None,
        long_term_goal_override: None,
    };
    let grant = sign_hosted_prompt_control_strong_auth_grant(
        "prompt_control_apply",
        "player-a",
        player_public_key.as_str(),
        "agent-1",
        100,
        200,
        backend_public_key.as_str(),
        backend_private_key.as_str(),
    )
    .expect("sign hosted strong-auth grant");
    let err = verify_hosted_prompt_control_apply_strong_auth_grant(
        PromptControlAuthIntent::Apply,
        &request,
        &grant,
        backend_public_key.as_str(),
        150,
    )
    .expect_err("mismatched hosted strong-auth grant must fail");
    assert!(err.contains("agent_id"));
}

#[test]
fn agent_chat_auth_verify_rejects_player_mismatch() {
    let (public_key, private_key) = test_signer();
    let request = AgentChatRequest {
        agent_id: "agent-0".to_string(),
        message: "hello".to_string(),
        player_id: Some("player-a".to_string()),
        public_key: Some(public_key.clone()),
        auth: None,
        intent_tick: Some(9),
        intent_seq: Some(15),
    };
    let mut proof =
        sign_agent_chat_auth_proof(&request, 15, public_key.as_str(), private_key.as_str())
            .expect("sign proof");
    proof.player_id = "player-b".to_string();
    let err = verify_agent_chat_auth_proof(&request, &proof).expect_err("player mismatch");
    assert!(err.contains("player_id"));
}

#[test]
fn agent_chat_auth_verify_rejects_invalid_signature_prefix() {
    let (public_key, private_key) = test_signer();
    let request = AgentChatRequest {
        agent_id: "agent-0".to_string(),
        message: "hello".to_string(),
        player_id: Some("player-a".to_string()),
        public_key: Some(public_key.clone()),
        auth: None,
        intent_tick: None,
        intent_seq: Some(16),
    };
    let mut proof =
        sign_agent_chat_auth_proof(&request, 16, public_key.as_str(), private_key.as_str())
            .expect("sign proof");
    proof.signature = "badprefix:deadbeef".to_string();
    let err = verify_agent_chat_auth_proof(&request, &proof).expect_err("invalid prefix");
    assert!(err.contains("awviewauth:v1"));
}

#[test]
fn agent_chat_auth_verify_rejects_zero_intent_seq() {
    let (public_key, private_key) = test_signer();
    let request = AgentChatRequest {
        agent_id: "agent-0".to_string(),
        message: "hello".to_string(),
        player_id: Some("player-a".to_string()),
        public_key: Some(public_key.clone()),
        auth: None,
        intent_tick: Some(1),
        intent_seq: Some(0),
    };
    let err = sign_agent_chat_auth_proof(&request, 17, public_key.as_str(), private_key.as_str())
        .expect_err("zero intent_seq should fail");
    assert!(err.contains("intent_seq"));
}

#[test]
fn gameplay_action_auth_sign_and_verify_roundtrip() {
    let (public_key, private_key) = test_signer();
    let request = GameplayActionRequest {
        action_id: "build_factory_smelter_mk1".to_string(),
        target_agent_id: "agent-0".to_string(),
        actor_agent_id: None,
        player_id: "player-a".to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
    };
    let proof =
        sign_gameplay_action_auth_proof(&request, 21, public_key.as_str(), private_key.as_str())
            .expect("sign proof");
    let verified = verify_gameplay_action_auth_proof(&request, &proof).expect("verify proof");
    assert_eq!(verified.player_id, "player-a");
    assert_eq!(verified.public_key, public_key);
    assert_eq!(verified.nonce, 21);
}

#[test]
fn gameplay_action_auth_verify_rejects_tampered_action_id() {
    let (public_key, private_key) = test_signer();
    let request = GameplayActionRequest {
        action_id: "build_factory_smelter_mk1".to_string(),
        target_agent_id: "agent-0".to_string(),
        actor_agent_id: None,
        player_id: "player-a".to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
    };
    let proof =
        sign_gameplay_action_auth_proof(&request, 22, public_key.as_str(), private_key.as_str())
            .expect("sign proof");
    let mut tampered = request.clone();
    tampered.action_id = "schedule_recipe_smelter_iron_ingot".to_string();
    let err = verify_gameplay_action_auth_proof(&tampered, &proof).expect_err("tamper must fail");
    assert!(err.contains("verify auth signature failed"));
}

#[test]
fn session_register_auth_verify_rejects_tampered_requested_agent_id() {
    let (public_key, private_key) = test_signer();
    let request = AuthoritativeSessionRegisterRequest {
        player_id: "player-a".to_string(),
        public_key: Some(public_key.clone()),
        registration_grant: None,
        auth: None,
        requested_agent_id: Some("agent-0".to_string()),
        force_rebind: false,
    };
    let proof =
        sign_session_register_auth_proof(&request, 31, public_key.as_str(), private_key.as_str())
            .expect("sign proof");

    let mut tampered = request.clone();
    tampered.requested_agent_id = Some("agent-1".to_string());
    let err = verify_session_register_auth_proof(&tampered, &proof)
        .expect_err("tampered requested_agent_id must fail");
    assert!(err.contains("verify auth signature failed"));
}

#[test]
fn session_register_auth_verify_rejects_tampered_force_rebind() {
    let (public_key, private_key) = test_signer();
    let request = AuthoritativeSessionRegisterRequest {
        player_id: "player-a".to_string(),
        public_key: Some(public_key.clone()),
        registration_grant: None,
        auth: None,
        requested_agent_id: Some("agent-0".to_string()),
        force_rebind: false,
    };
    let proof =
        sign_session_register_auth_proof(&request, 32, public_key.as_str(), private_key.as_str())
            .expect("sign proof");

    let mut tampered = request.clone();
    tampered.force_rebind = true;
    let err = verify_session_register_auth_proof(&tampered, &proof)
        .expect_err("tampered force_rebind must fail");
    assert!(err.contains("verify auth signature failed"));
}

#[test]
fn hosted_session_register_rejects_arbitrary_self_signed_player_claim() {
    let (attacker_public_key, attacker_private_key) = test_signer_with_seed(41);
    let request = AuthoritativeSessionRegisterRequest {
        player_id: "hosted-player-account-00000001".to_string(),
        public_key: Some(attacker_public_key.clone()),
        registration_grant: None,
        auth: None,
        requested_agent_id: None,
        force_rebind: false,
    };
    let proof = sign_session_register_auth_proof(
        &request,
        33,
        attacker_public_key.as_str(),
        attacker_private_key.as_str(),
    )
    .expect("attacker can self-sign its claim");

    let error = verify_session_register_auth_proof(&request, &proof)
        .expect_err("hosted player claims require an issuer-authenticated registration grant");
    assert!(
        error.contains("registration grant"),
        "unexpected error: {error}"
    );
}

#[test]
fn hosted_session_register_grant_is_bound_and_single_use_across_ledger_reload() {
    let (issuer_public_key, issuer_private_key) = test_signer_with_seed(51);
    let (public_key, private_key) = test_signer_with_seed(52);
    let ledger_path = std::env::temp_dir().join(format!(
        "oasis7-registration-replay-{}-{}.json",
        std::process::id(),
        now_unix_ms()
    ));
    unsafe {
        std::env::set_var(
            HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV,
            &issuer_public_key,
        );
        std::env::set_var(HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV, &ledger_path);
    }
    let grant = issue_hosted_registration_grant(
        "hosted-player-account-test",
        public_key.as_str(),
        "device-1",
        "nonce-1",
        now_unix_ms(),
        issuer_private_key.as_str(),
    )
    .expect("issue registration grant");
    let request = AuthoritativeSessionRegisterRequest {
        player_id: "hosted-player-account-test".to_string(),
        public_key: Some(public_key.clone()),
        registration_grant: Some(grant),
        auth: None,
        requested_agent_id: None,
        force_rebind: false,
    };
    let proof =
        sign_session_register_auth_proof(&request, 91, public_key.as_str(), private_key.as_str())
            .expect("sign session request");

    let mut mismatched_request = request.clone();
    mismatched_request.player_id = "hosted-player-account-other".to_string();
    let mismatched_proof = sign_session_register_auth_proof(
        &mismatched_request,
        92,
        public_key.as_str(),
        private_key.as_str(),
    )
    .expect("sign mismatched session request");
    let mismatch = verify_session_register_auth_proof(&mismatched_request, &mismatched_proof)
        .expect_err("grant binding mismatch rejected");
    assert!(mismatch.contains("binding"), "unexpected error: {mismatch}");

    let expired_grant = issue_hosted_registration_grant(
        "hosted-player-account-test",
        public_key.as_str(),
        "device-1",
        "nonce-expired",
        now_unix_ms()
            .saturating_sub(HOSTED_REGISTRATION_GRANT_TTL_MS)
            .saturating_sub(1),
        issuer_private_key.as_str(),
    )
    .expect("issue expired registration grant");
    let mut expired_request = request.clone();
    expired_request.registration_grant = Some(expired_grant);
    let expired_proof = sign_session_register_auth_proof(
        &expired_request,
        93,
        public_key.as_str(),
        private_key.as_str(),
    )
    .expect("sign expired session request");
    let expired = verify_session_register_auth_proof(&expired_request, &expired_proof)
        .expect_err("expired grant rejected");
    assert!(expired.contains("expired"), "unexpected error: {expired}");

    verify_session_register_auth_proof(&request, &proof).expect("first use accepted");
    let replay = verify_session_register_auth_proof(&request, &proof).expect_err("replay rejected");
    assert!(replay.contains("replay"), "unexpected error: {replay}");
    let _ = std::fs::remove_file(ledger_path);
    unsafe {
        std::env::remove_var(HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV);
        std::env::remove_var(HOSTED_REGISTRATION_REPLAY_LEDGER_PATH_ENV);
    }
}
