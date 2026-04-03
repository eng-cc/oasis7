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
