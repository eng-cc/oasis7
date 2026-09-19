use super::*;

#[path = "prompt_control_enhanced_preview.rs"]
mod prompt_control_enhanced_preview_tests;

#[test]
fn runtime_prompt_control_enhanced_apply_is_idempotent_per_authority_epoch() {
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
    let (public_key, private_key) = test_signer(41);
    let registration = register_runtime_session(
        &mut server,
        "player-enhanced",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let mut request = crate::viewer::PromptControlApplyRequest {
        agent_id: agent_id.clone(),
        player_id: "player-enhanced".to_string(),
        public_key: None,
        auth: None,
        strong_auth_grant: None,
        expected_version: Some(0),
        updated_by: Some("player-enhanced".to_string()),
        system_prompt_override: Some(Some("enhanced system".to_string())),
        short_term_goal_override: None,
        long_term_goal_override: None,
        request_id: Some("enhanced-apply-1".to_string()),
        session_epoch: registration.session_epoch,
        binding_epoch: registration.binding_epoch,
        expected_authority_epoch: Some(authority_epoch.clone()),
        ..Default::default()
    };
    request = signed_prompt_control_apply_request(
        request,
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let first = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: request.clone(),
            },
            &negotiated,
        )
        .expect("enhanced apply");
    assert_eq!(
        first.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Applied)
    );
    assert_eq!(first.request_id.as_deref(), Some("enhanced-apply-1"));
    assert_eq!(
        first.authority_epoch.as_deref(),
        Some(authority_epoch.as_str())
    );
    assert_eq!(first.mutation_count, Some(1));

    let replay = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect("same signed request replays from result ledger");
    assert!(replay.idempotent_replay);
    assert_eq!(replay.version, first.version);
    assert!(server.pending_virtual_events.iter().any(|event| matches!(
        event.kind,
        crate::simulator::WorldEventKind::AgentPromptUpdated { .. }
    )));
}

#[test]
fn runtime_prompt_control_rejects_zero_result_limits_before_bootstrap() {
    let cache_error = match ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_prompt_result_cache_capacity(0),
    ) {
        Err(error) => error,
        Ok(_) => panic!("zero result cache capacity must fail closed"),
    };
    assert!(format!("{cache_error:?}").contains("cache capacity"));

    let receipt_error = match ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_prompt_result_receipt_max_bytes(0),
    ) {
        Err(error) => error,
        Ok(_) => panic!("zero receipt limit must fail closed"),
    };
    assert!(format!("{receipt_error:?}").contains("receipt max bytes"));
}

#[test]
fn runtime_prompt_control_enhanced_requires_negotiated_result_capability() {
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
    let request = crate::viewer::PromptControlApplyRequest {
        agent_id,
        player_id: "player-a".to_string(),
        request_id: Some("capability-required".to_string()),
        ..Default::default()
    };
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: Vec::new(),
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Preview { request },
            &negotiated,
        )
        .expect_err("enhanced request without capability must fail");
    assert_eq!(error.code, "prompt_control_capability_required");
}

#[test]
fn runtime_prompt_control_selected_capability_rejects_legacy_shaped_requests() {
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
    let (public_key, private_key) = test_signer(47);
    let registration = register_runtime_session(
        &mut server,
        "player-shape",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let legacy_shaped = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-shape".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-shape".to_string()),
            system_prompt_override: Some(Some("must not apply".to_string())),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: legacy_shaped,
            },
            &negotiated,
        )
        .expect_err("selected enhanced capability must reject legacy shape");
    assert_eq!(error.code, "prompt_control_field_required");
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Rejected)
    );
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-shape"),
        Some(&1)
    );

    let partial = crate::viewer::PromptControlApplyRequest {
        agent_id,
        player_id: "player-shape".to_string(),
        request_id: Some("partial-shape".to_string()),
        expected_version: Some(0),
        ..Default::default()
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Preview { request: partial },
            &negotiated,
        )
        .expect_err("partial enhanced shape must fail before proof verification");
    assert_eq!(error.code, "prompt_control_field_required");
    assert_eq!(error.request_id.as_deref(), Some("partial-shape"));
}

#[test]
fn runtime_prompt_control_result_capability_is_not_advertised_in_script_mode() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Script),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();
    server
        .handle_request(
            ViewerRequest::HelloV2 {
                client: "script-capability-probe".to_string(),
                version: crate::viewer::VIEWER_PROTOCOL_VERSION,
                capabilities: vec![
                    crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string(),
                ],
            },
            &mut session,
            &mut writer,
        )
        .expect("handle script v2 hello");
    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(25));
    let ViewerResponse::HelloAck {
        capabilities,
        authority_epoch,
        ..
    } = responses.first().expect("missing script hello ack")
    else {
        panic!("expected hello ack, got {responses:?}");
    };
    assert!(
        !capabilities
            .iter()
            .any(|capability| capability
                == crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY)
    );
    assert!(authority_epoch.is_none());
    assert!(
        !crate::viewer::protocol::viewer_protocol_supports_prompt_control_result(
            &session.negotiated_protocol
        )
    );
}

#[test]
fn runtime_prompt_control_enhanced_applied_then_stale_is_serialized() {
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
    let (public_key, private_key) = test_signer(42);
    let registration = register_runtime_session(
        &mut server,
        "player-serialized",
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
    let make_request = |request_id: &str, nonce: u64, text: &str| {
        signed_prompt_control_apply_request(
            crate::viewer::PromptControlApplyRequest {
                agent_id: agent_id.clone(),
                player_id: "player-serialized".to_string(),
                expected_version: Some(0),
                updated_by: Some("player-serialized".to_string()),
                system_prompt_override: Some(Some(text.to_string())),
                request_id: Some(request_id.to_string()),
                session_epoch: registration.session_epoch,
                binding_epoch: registration.binding_epoch,
                expected_authority_epoch: Some(authority_epoch.clone()),
                ..Default::default()
            },
            crate::viewer::PromptControlAuthIntent::Apply,
            nonce,
            public_key.as_str(),
            private_key.as_str(),
        )
    };
    let first = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: make_request("serialized-a", 2, "first"),
            },
            &negotiated,
        )
        .expect("first operation applies");
    assert_eq!(
        first.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Applied)
    );
    assert_eq!(
        first.applied_scope,
        Some(crate::viewer::protocol::PromptControlApplicationScope::RuntimeInstance)
    );
    assert_eq!(
        first.persistence_scope,
        Some(crate::viewer::protocol::PromptControlApplicationScope::None)
    );
    assert_eq!(
        first.sync_scope,
        Some(crate::viewer::protocol::PromptControlApplicationScope::None)
    );

    let mut stale_request = make_request("serialized-b", 3, "second");
    stale_request.expected_version = Some(0);
    stale_request.auth = None;
    stale_request = signed_prompt_control_apply_request(
        stale_request,
        crate::viewer::PromptControlAuthIntent::Apply,
        3,
        public_key.as_str(),
        private_key.as_str(),
    );
    let stale = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: stale_request.clone(),
            },
            &negotiated,
        )
        .expect_err("second baseline request must become stale");
    assert_eq!(stale.code, "version_conflict");
    assert_eq!(
        stale.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Stale)
    );
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-serialized"),
        Some(&3)
    );
    let stale_replay = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: stale_request.clone(),
            },
            &negotiated,
        )
        .expect_err("same stale request must replay its terminal result");
    assert_eq!(stale_replay.code, "version_conflict");
    assert!(stale_replay.idempotent_replay);
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-serialized"),
        Some(&3)
    );

    let mut conflict_request = stale_request;
    conflict_request.system_prompt_override = Some(Some("different operation".to_string()));
    conflict_request.auth = None;
    conflict_request = signed_prompt_control_apply_request(
        conflict_request,
        crate::viewer::PromptControlAuthIntent::Apply,
        4,
        public_key.as_str(),
        private_key.as_str(),
    );
    let conflict = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: conflict_request,
            },
            &negotiated,
        )
        .expect_err("same request id with a different digest must conflict");
    assert_eq!(conflict.code, "request_id_conflict");
    assert!(!conflict.idempotent_replay);
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-serialized"),
        Some(&3)
    );
}

#[test]
fn runtime_prompt_control_hosted_control_loss_precedes_missing_grant() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true),
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
    let rebound_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .find(|candidate| *candidate != &agent_id)
        .cloned()
        .expect("second seed agent");
    let (public_key, private_key) = test_signer(48);
    let registration = register_runtime_session(
        &mut server,
        "player-hosted-loss",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-loss".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-hosted-loss".to_string()),
            system_prompt_override: Some(Some("lost control".to_string())),
            request_id: Some("hosted-control-loss".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(server.prompt_control_authority.authority_epoch.clone()),
            strong_auth_grant: None,
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let rebound = register_runtime_session_with_options(
        &mut server,
        "player-hosted-loss",
        Some(rebound_agent_id.as_str()),
        true,
        3,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(rebound.binding_epoch, Some(1));
    assert_eq!(
        server
            .prompt_control_authority
            .binding_epoch(agent_id.as_str()),
        registration.binding_epoch.unwrap_or_default() + 1
    );

    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect_err("lost hosted control must be reported before grant status");
    assert_eq!(error.code, "control_lost");
    assert_eq!(error.reason_code.as_deref(), Some("control_lost"));
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        error.value_visibility,
        Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
    );
    assert!(error.player_id.is_none());
    assert!(error.binding_epoch.is_none());
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
    clear_hosted_strong_auth_env();
}

#[test]
fn runtime_prompt_control_binding_loss_is_hidden_before_nonce_or_version() {
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
    let (public_key, private_key) = test_signer(43);
    let registration = register_runtime_session(
        &mut server,
        "player-binding-loss",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-binding-loss".to_string(),
            expected_version: Some(999),
            updated_by: Some("player-binding-loss".to_string()),
            system_prompt_override: Some(Some("should-not-apply".to_string())),
            request_id: Some("binding-loss".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(authority_epoch),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let rebound = register_runtime_session_with_options(
        &mut server,
        "player-binding-loss",
        Some(agent_id.as_str()),
        true,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(rebound.binding_epoch, Some(2));
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect_err("stale binding must block before expected version and nonce");
    assert_eq!(error.code, "control_lost");
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        error.value_visibility,
        Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
    );
    assert!(error.player_id.is_none());
    assert!(error.binding_epoch.is_none());
    assert!(error.current_version.is_none());
    assert!(error.digest.is_none());
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-binding-loss"),
        Some(&2)
    );
    assert_eq!(
        server
            .llm_sidecar
            .prompt_profiles
            .get(agent_id.as_str())
            .map(|p| p.version),
        None
    );
}

#[test]
fn runtime_prompt_control_binding_loss_precedes_authority_and_version_mismatch() {
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
    let (public_key, private_key) = test_signer(49);
    let registration = register_runtime_session(
        &mut server,
        "player-binding-authority-race",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let old_authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-binding-authority-race".to_string(),
            // This must never be reached after binding loss.
            expected_version: Some(999),
            updated_by: Some("player-binding-authority-race".to_string()),
            system_prompt_override: Some(Some("must stay unchanged".to_string())),
            request_id: Some("binding-before-authority-race".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(old_authority_epoch),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );

    let rebound = register_runtime_session_with_options(
        &mut server,
        "player-binding-authority-race",
        Some(agent_id.as_str()),
        true,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        rebound.binding_epoch,
        Some(registration.binding_epoch.unwrap_or_default() + 1)
    );
    // Simulate the same request crossing a runtime-authority fence after the
    // binding was revoked/rebound. Binding loss must still win the redaction
    // ordering, rather than being hidden as result_unknown.
    server.prompt_control_authority.authority_epoch = "authority-after-binding-loss".to_string();

    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect_err("binding loss must precede authority and version mismatch");
    assert_eq!(error.code, "control_lost");
    assert_eq!(error.reason_code.as_deref(), Some("control_lost"));
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        error.value_visibility,
        Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
    );
    assert_eq!(error.agent_id.as_deref(), Some(agent_id.as_str()));
    assert!(error.player_id.is_none());
    assert!(error.current_version.is_none());
    assert!(error.digest.is_none());
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-binding-authority-race"),
        Some(&2),
        "rebind registration nonce is retained; rejected prompt must not consume nonce 3"
    );
    assert_eq!(server.prompt_control_authority.result_ledger.len(), 0);
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
}

#[test]
fn runtime_prompt_control_conflicting_rebind_precedes_authority_and_version_mismatch() {
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
    let (player_a_public_key, player_a_private_key) = test_signer(52);
    let player_a_registration = register_runtime_session(
        &mut server,
        "player-conflict-a",
        Some(agent_id.as_str()),
        1,
        player_a_public_key.as_str(),
        player_a_private_key.as_str(),
    );
    let old_authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-conflict-a".to_string(),
            expected_version: Some(999),
            updated_by: Some("player-conflict-a".to_string()),
            system_prompt_override: Some(Some(
                "must not apply after conflicting rebind".to_string(),
            )),
            request_id: Some("conflicting-rebind-before-authority-race".to_string()),
            session_epoch: player_a_registration.session_epoch,
            binding_epoch: player_a_registration.binding_epoch,
            expected_authority_epoch: Some(old_authority_epoch),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        player_a_public_key.as_str(),
        player_a_private_key.as_str(),
    );

    let (player_b_public_key, player_b_private_key) = test_signer(53);
    let player_b_registration = register_runtime_session_with_options(
        &mut server,
        "player-conflict-b",
        Some(agent_id.as_str()),
        true,
        1,
        player_b_public_key.as_str(),
        player_b_private_key.as_str(),
    );
    assert_eq!(
        player_b_registration.binding_epoch,
        Some(player_a_registration.binding_epoch.unwrap_or_default() + 1)
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_player_bindings
            .get(agent_id.as_str()),
        Some(&"player-conflict-b".to_string())
    );
    let pending_events_before_request = server.pending_virtual_events.len();
    server.prompt_control_authority.authority_epoch =
        "authority-after-conflicting-rebind".to_string();

    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect_err("conflicting rebind must precede authority and version mismatch");
    assert_eq!(error.code, "control_lost");
    assert_eq!(error.reason_code.as_deref(), Some("control_lost"));
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        error.value_visibility,
        Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
    );
    assert_eq!(error.agent_id.as_deref(), Some(agent_id.as_str()));
    assert!(error.player_id.is_none());
    assert!(error.expected_version.is_none());
    assert!(error.current_version.is_none());
    assert!(error.digest.is_none());
    assert!(error.operation_digest.is_none());
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-conflict-a"),
        Some(&1),
        "conflicting rebind rejection must not consume player A nonce 2"
    );
    assert_eq!(
        server.pending_virtual_events.len(),
        pending_events_before_request
    );
    assert_eq!(server.prompt_control_authority.result_ledger.len(), 0);
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
}

#[test]
fn runtime_prompt_control_session_loss_precedes_authority_and_version_mismatch() {
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
    let (public_key, private_key) = test_signer(50);
    let registration = register_runtime_session(
        &mut server,
        "player-session-authority-race",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let old_authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-session-authority-race".to_string(),
            expected_version: Some(999),
            updated_by: Some("player-session-authority-race".to_string()),
            system_prompt_override: Some(Some("must stay unchanged".to_string())),
            request_id: Some("session-before-authority-race".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(old_authority_epoch),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );

    let (revoke_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RevokeSession {
            request: AuthoritativeSessionRevokeRequest {
                player_id: "player-session-authority-race".to_string(),
                session_pubkey: Some(public_key.clone()),
                revoke_reason: "authority-ordering-test".to_string(),
                revoked_by: Some("runtime-test".to_string()),
            },
        })
        .expect("revoke session");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        revoke_ack.status,
        AuthoritativeRecoveryStatus::SessionRevoked
    );
    server.prompt_control_authority.authority_epoch = "authority-after-session-loss".to_string();

    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect_err("session loss must precede authority and version mismatch");
    assert_eq!(error.code, "control_lost");
    assert_eq!(error.reason_code.as_deref(), Some("control_lost"));
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        error.value_visibility,
        Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
    );
    assert_eq!(error.agent_id.as_deref(), Some(agent_id.as_str()));
    assert!(error.player_id.is_none());
    assert!(error.current_version.is_none());
    assert!(error.digest.is_none());
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
    assert_eq!(server.prompt_control_authority.result_ledger.len(), 0);
}

#[test]
fn runtime_prompt_control_restart_fences_old_authority_result() {
    let _guard = lock_test_llm_env();
    let mut old_server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("old runtime server");
    let agent_id = old_server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(44);
    let registration = register_runtime_session(
        &mut old_server,
        "player-restart",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let old_authority_epoch = old_server.prompt_control_authority.authority_epoch.clone();
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-restart".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-restart".to_string()),
            system_prompt_override: Some(Some("old-authority".to_string())),
            request_id: Some("restart-retry".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(old_authority_epoch.clone()),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let mut new_server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("new runtime server");
    assert_ne!(
        new_server.prompt_control_authority.authority_epoch,
        old_authority_epoch
    );
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = new_server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply { request },
            &negotiated,
        )
        .expect_err("old authority retry must be fenced");
    assert_eq!(error.code, "prompt_control_result_unknown");
    assert_eq!(
        error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        error.value_visibility,
        Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
    );
    assert!(error.agent_id.is_none());
    assert!(error.player_id.is_none());
    assert!(error.operation_digest.is_none());
    assert!(error.expected_version.is_none());
    assert!(new_server.pending_virtual_events.is_empty());
}

#[test]
fn runtime_prompt_control_cache_and_receipt_limits_refuse_before_nonce() {
    let _guard = lock_test_llm_env();
    let mut full_server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_prompt_result_cache_capacity(1),
    )
    .expect("runtime server");
    let agent_id = full_server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(45);
    let registration = register_runtime_session(
        &mut full_server,
        "player-cache",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let authority_epoch = full_server.prompt_control_authority.authority_epoch.clone();
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let make_request = |request_id: &str, nonce: u64, expected_version: u64| {
        signed_prompt_control_apply_request(
            crate::viewer::PromptControlApplyRequest {
                agent_id: agent_id.clone(),
                player_id: "player-cache".to_string(),
                expected_version: Some(expected_version),
                updated_by: Some("player-cache".to_string()),
                system_prompt_override: Some(Some(request_id.to_string())),
                request_id: Some(request_id.to_string()),
                session_epoch: registration.session_epoch,
                binding_epoch: registration.binding_epoch,
                expected_authority_epoch: Some(authority_epoch.clone()),
                ..Default::default()
            },
            crate::viewer::PromptControlAuthIntent::Apply,
            nonce,
            public_key.as_str(),
            private_key.as_str(),
        )
    };
    full_server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: make_request("cache-first", 2, 0),
            },
            &negotiated,
        )
        .expect("first cache entry");
    let full_error = full_server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: make_request("cache-second", 3, 1),
            },
            &negotiated,
        )
        .expect_err("new request must be blocked when ledger is full");
    assert_eq!(full_error.code, "result_cache_full");
    assert_eq!(
        full_error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
    );
    assert_eq!(
        full_server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-cache"),
        Some(&2)
    );
    assert_eq!(
        full_server
            .llm_sidecar
            .prompt_profiles
            .get(agent_id.as_str())
            .map(|p| p.version),
        Some(1)
    );
    assert_eq!(
        full_error.next_step.as_deref(),
        Some("refresh_authority_and_retry_with_new_request_id")
    );

    let mut receipt_server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_prompt_result_receipt_max_bytes(1),
    )
    .expect("runtime server");
    let receipt_agent_id = receipt_server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (receipt_public_key, receipt_private_key) = test_signer(46);
    let receipt_registration = register_runtime_session(
        &mut receipt_server,
        "player-receipt",
        Some(receipt_agent_id.as_str()),
        1,
        receipt_public_key.as_str(),
        receipt_private_key.as_str(),
    );
    let receipt_request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: receipt_agent_id.clone(),
            player_id: "player-receipt".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-receipt".to_string()),
            system_prompt_override: Some(Some("oversize".to_string())),
            request_id: Some("receipt-oversize".to_string()),
            session_epoch: receipt_registration.session_epoch,
            binding_epoch: receipt_registration.binding_epoch,
            expected_authority_epoch: Some(
                receipt_server
                    .prompt_control_authority
                    .authority_epoch
                    .clone(),
            ),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        receipt_public_key.as_str(),
        receipt_private_key.as_str(),
    );
    let receipt_error = receipt_server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Apply {
                request: receipt_request,
            },
            &negotiated,
        )
        .expect_err("oversize receipt must be refused");
    assert_eq!(receipt_error.code, "result_receipt_too_large");
    assert_eq!(
        receipt_error.status,
        Some(crate::viewer::protocol::PromptControlResultStatus::Rejected)
    );
    assert_eq!(
        receipt_server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-receipt"),
        Some(&1)
    );
    assert_eq!(
        receipt_server
            .llm_sidecar
            .prompt_profiles
            .get(receipt_agent_id.as_str())
            .map(|p| p.version),
        None
    );
}

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

fn configure_hosted_local_mock_provider_env(contract: &str) {
    // SAFETY: Callers hold the canonical provider environment lock.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_backed");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "provider_local_mock");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_CONTRACT_ENV, contract);
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, "loopback_http");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "http://127.0.0.1:9");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
        oasis7::env_mut::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    }
}

fn hosted_local_mock_runtime_server() -> ViewerRuntimeLiveServer {
    ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true)
            .with_test_cognition_runtime_binding(
                "hosted-local-mock-branch",
                0,
                Some(
                    crate::simulator::h_v1(
                        "oasis7.viewer.test.finality-block.v1",
                        &"live-runtime-minimal",
                    )
                    .to_string(),
                ),
                "verified",
                0,
            ),
    )
    .expect("Hosted local-mock runtime server")
}

#[cfg(not(target_arch = "wasm32"))]
fn prime_hosted_local_mock_provider_context(server: &mut ViewerRuntimeLiveServer, agent_id: &str) {
    let world_id = server
        .world
        .current_cognition_runtime_binding()
        .expect("Hosted local-mock Runtime binding")
        .world_id
        .clone();
    server
        .llm_sidecar
        .prepare_provider_backed_test_context(
            &mut server.world,
            &WorldConfig::default(),
            world_id.as_str(),
            agent_id,
        )
        .expect("prepare proof-bearing Hosted local-mock provider context");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runtime_prompt_control_hosted_local_mock_provider_requires_runtime_context() {
    let _llm_guard = lock_test_llm_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.enable_hosted_local_mock_test_lane();
    sidecar
        .replace_provider_backed_runner_for_test("hosted-local-mock-agent")
        .expect("ProviderBacked test actor");
    let error = sidecar
        .apply_prompt_profile_to_driver(&crate::simulator::AgentPromptProfile {
            agent_id: "hosted-local-mock-agent".to_string(),
            short_term_goal_override: Some("must not project".to_string()),
            ..Default::default()
        })
        .expect_err("missing Runtime context must deny ProviderBacked prompt control");
    assert!(error.contains("without an admitted Hosted local-mock Runtime context"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runtime_prompt_control_hosted_local_mock_provider_rejects_rotated_runtime_binding() {
    let _llm_guard = lock_test_llm_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
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

    let mut rotated_binding = server
        .world
        .current_cognition_runtime_binding()
        .expect("current Runtime binding")
        .clone();
    rotated_binding.branch_id.push_str("-rotated");
    server
        .llm_sidecar
        .set_provider_test_binding(rotated_binding);
    let error = server
        .llm_sidecar
        .apply_prompt_profile_to_driver(&crate::simulator::AgentPromptProfile {
            agent_id,
            short_term_goal_override: Some("must not cross Runtime heads".to_string()),
            ..Default::default()
        })
        .expect_err("a context bound to the prior Runtime head must be denied");
    assert!(error.contains("without an admitted Hosted local-mock Runtime context"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runtime_prompt_control_hosted_local_mock_provider_refreshes_after_runtime_rebind() {
    let _llm_guard = lock_test_llm_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
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
        .expect("install initial proof-bearing local-mock capability context");
    prime_hosted_local_mock_provider_context(&mut server, agent_id.as_str());

    let old_binding = server
        .world
        .current_cognition_runtime_binding()
        .expect("initial Runtime binding")
        .clone();
    server.world = server.world.clone().with_cognition_scheduler(
        serde_json::from_value(serde_json::json!({
            "schema_version": "scheduler-policy.v1",
            "max_total_wakes_per_tick": 8,
            "max_wakes_per_agent_per_tick": 1,
            "aging_after_ticks": 2,
            "max_starvation_ticks": 4,
            "initial_priority": 0,
            "comparator": "deadline_due_desc,next_wake_tick_asc,effective_priority_desc,starvation_deadline_tick_asc,cursor_distance_asc,agent_id_asc,continuation_id_asc,wake_seq_asc",
            "service_order": "stable_round_robin"
        }))
        .expect("decode Runtime scheduler policy"),
        8,
    );
    let next_reorg_epoch = old_binding.reorg_epoch.saturating_add(1);
    server
        .world
        .invalidate_cognition_for_reorg(next_reorg_epoch)
        .expect("authorize Runtime binding rotation");
    server
        .world
        .bind_cognition_runtime(
            old_binding.world_id.clone(),
            old_binding.branch_id.clone(),
            old_binding.finality_epoch,
            old_binding
                .finality_block_hash
                .as_ref()
                .map(ToString::to_string),
            old_binding.finality_status.clone(),
            next_reorg_epoch,
        )
        .expect("bind the new authoritative Runtime head");
    server
        .world
        .install_test_provider_capability_fixture(agent_id.as_str())
        .expect("install proof-bearing context for the new Runtime head");

    // The sidecar still carries the old lineage marker. Refreshing through
    // the Runtime-owned preparation path must fence the old context, adopt
    // the new authoritative binding, and rebuild a usable provider context.
    server
        .llm_sidecar
        .refresh_provider_backed_test_context(
            &mut server.world,
            &WorldConfig::default(),
            old_binding.world_id.as_str(),
        )
        .expect("refresh provider context after Runtime rebind");
    let current_binding = server
        .world
        .current_cognition_runtime_binding()
        .expect("current Runtime binding");
    assert_eq!(
        server
            .llm_sidecar
            .provider_test_context_binding(agent_id.as_str()),
        Some(current_binding.clone()),
        "refresh must rebuild context instead of retaining the prior Runtime head"
    );
    let applied = server
        .llm_sidecar
        .apply_prompt_profile_to_driver(&crate::simulator::AgentPromptProfile {
            agent_id,
            short_term_goal_override: Some("new-runtime-head-goal".to_string()),
            ..Default::default()
        })
        .expect("refreshed current-head context must admit prompt control");
    assert_eq!(applied, ());
    assert_eq!(
        server.llm_sidecar.provider_test_binding(),
        Some(current_binding.clone())
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runtime_prompt_control_hosted_local_mock_provider_requires_initialized_runner() {
    let _llm_guard = lock_test_llm_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.enable_hosted_local_mock_test_lane();
    let error = sidecar
        .apply_prompt_profile_to_driver(&crate::simulator::AgentPromptProfile {
            agent_id: "hosted-local-mock-agent".to_string(),
            short_term_goal_override: Some("must not acknowledge without a driver".to_string()),
            ..Default::default()
        })
        .expect_err("Hosted Apply must fail closed before runner initialization");
    assert_eq!(error, "llm runner is not initialized");
}

fn signed_prompt_control_rollback_request(
    mut request: crate::viewer::PromptControlRollbackRequest,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::PromptControlRollbackRequest {
    request.public_key = Some(public_key_hex.to_string());
    request.auth = Some(
        crate::viewer::sign_prompt_control_rollback_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign rollback prompt auth"),
    );
    request
}

#[test]
fn runtime_prompt_control_hosted_local_mock_serialized_apply_returns_ack() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
    let (backend_public_key, backend_private_key) = test_signer(92);
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
    let (public_key, private_key) = test_signer(93);
    let registration = register_runtime_session(
        &mut server,
        "player-hosted-local-mock-wire",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();
    server
        .handle_request(
            ViewerRequest::HelloV2 {
                client: "hosted-local-mock-wire".to_string(),
                version: VIEWER_PROTOCOL_VERSION,
                capabilities: vec![
                    crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string(),
                ],
            },
            &mut session,
            &mut writer,
        )
        .expect("serialized-path HelloV2");
    let hello = read_available_runtime_live_responses(&peer, Duration::from_millis(25));
    assert!(matches!(
        hello.first(),
        Some(ViewerResponse::HelloAck { capabilities, .. })
            if capabilities.iter().any(|capability| capability
                == crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY)
    ));

    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(1_000);
    let mut request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-local-mock-wire".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-hosted-local-mock-wire".to_string()),
            system_prompt_override: Some(Some("wire-apply".to_string())),
            request_id: Some("hosted-local-mock-wire-apply".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(server.prompt_control_authority.authority_epoch.clone()),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    request.strong_auth_grant = Some(
        crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
            "prompt_control_apply",
            "player-hosted-local-mock-wire",
            public_key.as_str(),
            request.agent_id.as_str(),
            issued_at_unix_ms,
            issued_at_unix_ms.saturating_add(60_000),
            backend_public_key.as_str(),
            backend_private_key.as_str(),
        )
        .expect("wire strong-auth grant"),
    );
    let wire_request = ViewerRequest::PromptControl {
        command: Box::new(crate::viewer::PromptControlCommand::Apply { request }),
    };
    let decoded_request = serde_json::from_str::<ViewerRequest>(
        &serde_json::to_string(&wire_request).expect("serialize prompt-control request"),
    )
    .expect("deserialize prompt-control request");
    server
        .handle_request(decoded_request, &mut session, &mut writer)
        .expect("serialized Hosted local-mock Apply");
    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(25));
    match responses.as_slice() {
        [ViewerResponse::PromptControlAck { ack }] => {
            assert_eq!(
                ack.status,
                Some(crate::viewer::protocol::PromptControlResultStatus::Applied)
            );
            assert_eq!(ack.version, 1);
        }
        other => panic!("expected serialized prompt-control ack, got {other:?}"),
    }

    let mut rollback_request = signed_prompt_control_rollback_request(
        crate::viewer::PromptControlRollbackRequest {
            agent_id,
            player_id: "player-hosted-local-mock-wire".to_string(),
            expected_version: Some(1),
            updated_by: Some("player-hosted-local-mock-wire".to_string()),
            to_version: 0,
            request_id: Some("hosted-local-mock-wire-rollback".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(server.prompt_control_authority.authority_epoch.clone()),
            ..Default::default()
        },
        3,
        public_key.as_str(),
        private_key.as_str(),
    );
    rollback_request.strong_auth_grant = Some(
        crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
            "prompt_control_rollback",
            "player-hosted-local-mock-wire",
            public_key.as_str(),
            rollback_request.agent_id.as_str(),
            issued_at_unix_ms,
            issued_at_unix_ms.saturating_add(60_000),
            backend_public_key.as_str(),
            backend_private_key.as_str(),
        )
        .expect("wire rollback strong-auth grant"),
    );
    let rollback_wire_request = ViewerRequest::PromptControl {
        command: Box::new(crate::viewer::PromptControlCommand::Rollback {
            request: rollback_request,
        }),
    };
    let decoded_rollback_request = serde_json::from_str::<ViewerRequest>(
        &serde_json::to_string(&rollback_wire_request).expect("serialize rollback request"),
    )
    .expect("deserialize rollback request");
    server
        .handle_request(decoded_rollback_request, &mut session, &mut writer)
        .expect("serialized Hosted local-mock Rollback");
    let rollback_responses =
        read_available_runtime_live_responses(&peer, Duration::from_millis(25));
    match rollback_responses.as_slice() {
        [ViewerResponse::PromptControlAck { ack }] => {
            assert_eq!(
                ack.status,
                Some(crate::viewer::protocol::PromptControlResultStatus::Applied)
            );
            assert_eq!(ack.version, 2);
            assert_eq!(ack.rolled_back_to_version, Some(0));
        }
        other => panic!("expected serialized rollback ack, got {other:?}"),
    }
}

#[test]
fn runtime_prompt_control_hosted_local_mock_blocks_apply_and_rollback_when_context_preparation_fails()
 {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
    let (backend_public_key, backend_private_key) = test_signer(94);
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
    prime_hosted_local_mock_provider_context(&mut server, agent_id.as_str());
    let old_context_binding = server
        .llm_sidecar
        .provider_test_context_binding(agent_id.as_str())
        .expect("seed proof-bearing provider context");

    let (public_key, private_key) = test_signer(95);
    let registration = register_runtime_session(
        &mut server,
        "player-hosted-local-mock-prep-failure",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let authority_epoch = server.prompt_control_authority.authority_epoch.clone();
    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(1_000);
    let strong_auth_grant = |operation: &str| {
        crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
            operation,
            "player-hosted-local-mock-prep-failure",
            public_key.as_str(),
            agent_id.as_str(),
            issued_at_unix_ms,
            issued_at_unix_ms.saturating_add(60_000),
            backend_public_key.as_str(),
            backend_private_key.as_str(),
        )
        .expect("sign Hosted local-mock strong-auth grant")
    };

    let mut apply_request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-local-mock-prep-failure".to_string(),
            expected_version: Some(0),
            updated_by: Some("player-hosted-local-mock-prep-failure".to_string()),
            system_prompt_override: Some(Some("must not apply from stale context".to_string())),
            request_id: Some("hosted-local-mock-prep-failure-apply".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(authority_epoch.clone()),
            ..Default::default()
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    apply_request.strong_auth_grant = Some(strong_auth_grant("prompt_control_apply"));

    let mut rollback_request = signed_prompt_control_rollback_request(
        crate::viewer::PromptControlRollbackRequest {
            agent_id: agent_id.clone(),
            player_id: "player-hosted-local-mock-prep-failure".to_string(),
            expected_version: Some(1),
            updated_by: Some("player-hosted-local-mock-prep-failure".to_string()),
            to_version: 0,
            request_id: Some("hosted-local-mock-prep-failure-rollback".to_string()),
            session_epoch: registration.session_epoch,
            binding_epoch: registration.binding_epoch,
            expected_authority_epoch: Some(authority_epoch),
            ..Default::default()
        },
        3,
        public_key.as_str(),
        private_key.as_str(),
    );
    rollback_request.strong_auth_grant = Some(strong_auth_grant("prompt_control_rollback"));

    // Deliberately make the authoritative world/config tuple invalid after a valid
    // provider context exists. The old context must not be used as a fallback.
    server.config.world_id = "mismatched-world-for-prep-failure".to_string();
    let mut session = RuntimeLiveSession::new();
    session.negotiated_protocol = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let (mut writer, peer) = test_writer_pair();

    for command in [
        crate::viewer::PromptControlCommand::Apply {
            request: apply_request,
        },
        crate::viewer::PromptControlCommand::Rollback {
            request: rollback_request,
        },
    ] {
        server
            .handle_request(
                ViewerRequest::PromptControl {
                    command: Box::new(command),
                },
                &mut session,
                &mut writer,
            )
            .expect("prep failure should be returned as a PromptControlError");
        let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(25));
        match responses.as_slice() {
            [ViewerResponse::PromptControlError { error }] => {
                assert_eq!(error.code, "prompt_control_runtime_context_unavailable");
                assert_eq!(
                    error.status,
                    Some(crate::viewer::protocol::PromptControlResultStatus::Blocked)
                );
                assert_eq!(
                    error.value_visibility,
                    Some(crate::viewer::protocol::PromptControlValueVisibility::Hidden)
                );
            }
            other => panic!("expected blocked prep error, got {other:?}"),
        }
    }

    assert_eq!(
        server
            .llm_sidecar
            .provider_test_context_binding(agent_id.as_str()),
        Some(old_context_binding),
        "failed preparation must not authorize or replace the old context"
    );
    assert_eq!(
        server
            .llm_sidecar
            .prompt_profiles
            .get(agent_id.as_str())
            .map(|profile| profile.version),
        None,
        "blocked Apply/Rollback must not mutate the prompt profile"
    );
}

#[test]
fn runtime_prompt_control_hosted_local_mock_missing_context_and_wrong_tuple_fail_closed() {
    let _llm_guard = lock_test_llm_env();
    configure_hosted_local_mock_provider_env("wrong_contract");
    let sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    assert!(
        !sidecar.supports_prompt_control_result(),
        "wrong provider tuple must not enable PromptControl"
    );

    clear_runtime_provider_env();
    configure_hosted_local_mock_provider_env("worldsim_provider_v1");
    assert!(
        !RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm).supports_prompt_control_result(),
        "valid tuple without an installed Hosted Runtime fixture must remain disabled"
    );
    let mut empty_world = RuntimeWorld::new();
    let fixture_error = crate::viewer::runtime_live::control_plane::install_hosted_local_mock_test_capability_fixtures(
        &mut empty_world,
        true,
    )
    .expect_err("Hosted local-mock fixture installation must fail without a seeded Agent");
    assert!(fixture_error.contains("Runtime-seeded Agent"));
}

#[test]
fn runtime_prompt_control_production_provider_backed_handler_remains_denied() {
    let _llm_guard = lock_test_llm_env();
    // SAFETY: This test/setup code mutates process environment while holding
    // the canonical provider environment lock.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_backed");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "provider_local_bridge");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "http://127.0.0.1:9");
    }
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("production provider-backed runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let negotiated = crate::viewer::protocol::NegotiatedViewerProtocol {
        version: crate::viewer::VIEWER_PROTOCOL_VERSION,
        capabilities: vec![crate::viewer::protocol::PROMPT_CONTROL_RESULT_CAPABILITY.to_string()],
    };
    let error = server
        .handle_prompt_control_for_protocol(
            crate::viewer::PromptControlCommand::Preview {
                request: crate::viewer::PromptControlApplyRequest {
                    agent_id,
                    player_id: "production-provider-player".to_string(),
                    expected_version: Some(0),
                    updated_by: Some("production-provider-player".to_string()),
                    request_id: Some("production-provider-preview".to_string()),
                    session_epoch: Some(1),
                    binding_epoch: Some(0),
                    expected_authority_epoch: Some(
                        server.prompt_control_authority.authority_epoch.clone(),
                    ),
                    ..Default::default()
                },
            },
            &negotiated,
        )
        .expect_err("production provider-backed handler must remain denied");
    assert_eq!(error.code, "agent_provider_prompt_control_unsupported");
}
