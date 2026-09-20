use super::*;

#[test]
fn runtime_prompt_control_hosted_local_mock_preview_apply_and_rollback_use_result_capability() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
    let (backend_public_key, backend_private_key) = test_signer(90);
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(
            HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV,
            backend_public_key.as_str(),
        );
    }

    let mut server = hosted_local_mock_runtime_server();
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    server
        .world
        .install_test_provider_capability_fixture(agent_id.as_str())
        .expect("install proof-bearing local-mock capability context");
    prime_hosted_local_mock_provider_context(&mut server, agent_id.as_str());
    assert!(!server.world.capability_invocation_contexts().is_empty());

    let (public_key, private_key) = test_signer(91);
    let registration = register_runtime_session(
        &mut server,
        "player-hosted-local-mock-paths",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(1_000);
    let strong_auth_grant = |operation: &str| {
        crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
            operation,
            "player-hosted-local-mock-paths",
            public_key.as_str(),
            agent_id.as_str(),
            issued_at_unix_ms,
            issued_at_unix_ms.saturating_add(60_000),
            backend_public_key.as_str(),
            backend_private_key.as_str(),
        )
        .expect("sign Hosted local-mock strong-auth grant")
    };

    let mut preview_request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-local-mock-paths".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-hosted-local-mock-paths".to_string()),
            system_prompt_override: Some(Some("preview-local-mock".to_string())),
            request_id: Some("hosted-local-mock-preview".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(authority_epoch.clone()),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Preview,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    preview_request.strong_auth_grant = Some(strong_auth_grant("prompt_control_preview"));
    let preview = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Preview {
                request: preview_request,
            },
            &negotiated,
        )
        .expect("Hosted local-mock Preview request must reach enhanced handler");
    assert_eq!(
        preview.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Accepted)
    );

    let mut apply_request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-local-mock-paths".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-hosted-local-mock-paths".to_string()),
            system_prompt_override: Some(Some("apply-local-mock".to_string())),
            short_term_goal_override: Some(Some("apply-local-mock-goal".to_string())),
            request_id: Some("hosted-local-mock-apply".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(authority_epoch.clone()),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        3,
        public_key.as_str(),
        private_key.as_str(),
    );
    apply_request.strong_auth_grant = Some(strong_auth_grant("prompt_control_apply"));
    let apply = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: apply_request,
            },
            &negotiated,
        )
        .expect("Hosted local-mock Apply request must reach enhanced handler");
    assert_eq!(
        apply.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Applied)
    );
    assert_eq!(apply.version, 1);
    assert_eq!(
        server
            .llm_sidecar
            .prompt_profiles
            .get(agent_id.as_str())
            .and_then(|profile| profile.short_term_goal_override.as_deref()),
        Some("apply-local-mock-goal")
    );
    prime_hosted_local_mock_provider_context(&mut server, agent_id.as_str());
    assert_eq!(
        server
            .llm_sidecar
            .provider_test_goal_summary(agent_id.as_str())
            .as_deref(),
        Some("apply-local-mock-goal")
    );

    let mut rollback_request = signed_prompt_control_rollback_request(
        crate::viewer::PromptControlRollbackRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-local-mock-paths".to_string(),
            expected_version: Some(1),
            updated_by: Some("player-hosted-local-mock-paths".to_string()),
            to_version: 0,
            request_id: Some("hosted-local-mock-rollback".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(authority_epoch),
            ..Default::default()
        },
        4,
        public_key.as_str(),
        private_key.as_str(),
    );
    rollback_request.strong_auth_grant = Some(strong_auth_grant("prompt_control_rollback"));
    let rollback = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Rollback {
                request: rollback_request,
            },
            &negotiated,
        )
        .expect("Hosted local-mock Rollback request must reach enhanced handler");
    assert_eq!(
        rollback.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Applied)
    );
    assert_eq!(rollback.rolled_back_to_version, Some(0));
}
