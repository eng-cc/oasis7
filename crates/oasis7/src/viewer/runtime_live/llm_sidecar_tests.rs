use super::*;

#[test]
fn bind_agent_player_emits_unbind_before_rebind_for_same_agent() {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar
        .agent_player_bindings
        .insert("agent-1".to_string(), "player-a".to_string());
    sidecar
        .player_agent_bindings
        .insert("player-a".to_string(), "agent-1".to_string());
    sidecar
        .agent_public_key_bindings
        .insert("agent-1".to_string(), "pubkey-a".to_string());

    let events = sidecar
        .bind_agent_player("agent-1", "player-b", Some("pubkey-b"), false)
        .expect("rebind should succeed");
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        WorldEventKind::AgentPlayerUnbound {
            agent_id,
            player_id,
            public_key
        } if agent_id == "agent-1"
            && player_id == "player-a"
            && public_key.as_deref() == Some("pubkey-a")
    ));
    assert!(matches!(
        &events[1],
        WorldEventKind::AgentPlayerBound {
            agent_id,
            player_id,
            public_key
        } if agent_id == "agent-1"
            && player_id == "player-b"
            && public_key.as_deref() == Some("pubkey-b")
    ));
    assert_eq!(
        sidecar
            .agent_player_bindings
            .get("agent-1")
            .map(String::as_str),
        Some("player-b")
    );
    assert_eq!(
        sidecar
            .player_agent_bindings
            .get("player-b")
            .map(String::as_str),
        Some("agent-1")
    );
    assert!(!sidecar.player_agent_bindings.contains_key("player-a"));
    assert_eq!(
        sidecar
            .agent_public_key_bindings
            .get("agent-1")
            .map(String::as_str),
        Some("pubkey-b")
    );
}

#[test]
fn sync_shadow_kernel_accepts_empty_synthetic_runtime_snapshot() {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    let world = RuntimeWorld::default();

    sidecar
        .sync_shadow_kernel(&world, &WorldConfig::default())
        .expect("empty synthetic runtime snapshot should sync");

    let shadow = sidecar.shadow_kernel.as_ref().expect("shadow kernel");
    assert!(shadow.journal().is_empty());
}

#[test]
fn sync_shadow_kernel_preserves_generated_seed_locations() {
    let seed_pos = GeoPos::new(7, 8, 9);
    let mut seed_model = WorldModel::default();
    seed_model.locations.insert(
        "frag-shadow".to_string(),
        Location::new("frag-shadow", "shadow fragment", seed_pos),
    );
    let mut sidecar =
        RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm).with_runtime_seed_model(&seed_model);
    let world = RuntimeWorld::default();

    sidecar
        .sync_shadow_kernel(&world, &WorldConfig::default())
        .expect("runtime seed shadow sync");

    let shadow = sidecar.shadow_kernel.as_ref().expect("shadow kernel");
    assert!(
        shadow
            .snapshot()
            .model
            .locations
            .contains_key("frag-shadow")
    );
}

#[test]
fn provider_lineage_persists_and_restores_pending_lifecycle_markers() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut first = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    first.configure_provider_lineage_store(path.clone());
    first
        .provider_session_ids
        .insert("agent-0".to_string(), "session-7".to_string());
    first.provider_context_seq.insert("agent-0".to_string(), 4);
    let request_context = crate::simulator::ContinuousAgentRequestContextV1 {
        base_decision_request: crate::simulator::DecisionRequest {
            observation: crate::simulator::ObservationEnvelope {
                agent_id: "agent-0".to_string(),
                world_time: 12,
                mode: crate::simulator::ProviderExecutionMode::PlayerParity,
                observation_schema_version:
                    crate::simulator::DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
                action_schema_version: crate::simulator::DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION
                    .to_string(),
                environment_class: Some("runtime_live".to_string()),
                fallback_reason: None,
                observation: crate::simulator::ProviderObservation::default(),
                recent_event_summary: Vec::new(),
                memory_summary: None,
                action_catalog: Vec::new(),
                module_command_catalog: Vec::new(),
                timeout_budget_ms: 100,
            },
            provider_config_ref: None,
            agent_profile: None,
            fixture_id: None,
            replay_id: None,
            capability_catalog: None,
            capability_invocation_context: None,
            timeout_budget_ms: 100,
        },
        context_discriminator: crate::simulator::CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR.to_string(),
        context_version: crate::simulator::CONTINUOUS_AGENT_CONTEXT_VERSION,
        protocol_version: "oasis7.continuous-agent-request.v1".to_string(),
        agent_session_id: "session-7".to_string(),
        agent_turn_id: "turn-active".to_string(),
        decision_request_id: "request-active".to_string(),
        retry_seq: 4,
        transport_attempt: 2,
        agent_subject: "agent-0".to_string(),
        runtime_binding: crate::simulator::RuntimeBindingV1 {
            world_id: "world".to_string(),
            branch_id: "main".to_string(),
            finality_epoch: 0,
            finality_block_hash: None,
            finality_status: "pending".to_string(),
            base_tick: 12,
            base_world_hash: crate::simulator::Digest32::default(),
            reorg_epoch: 0,
            runtime_manifest_hash: crate::simulator::Digest32::default(),
        },
        observation_digest: crate::simulator::Digest32::default(),
        capability_catalog_digest: crate::simulator::Digest32::default(),
        capability_invocation_context_digest: crate::simulator::Digest32::default(),
        memory_snapshot_digest: crate::simulator::Digest32::default(),
        goal_snapshot_digest: crate::simulator::Digest32::default(),
        continuation_digest: crate::simulator::Digest32::default(),
        adapter_protocol_version: "test".to_string(),
        budget_contract: crate::simulator::BudgetContractV1 {
            max_latency_ms: 100,
            max_repair_attempts: 0,
        },
        request_digest: crate::simulator::Digest32::default(),
    };
    let turn_context = crate::simulator::ContinuousAgentTurnContextV1 {
        agent_id: "agent-0".to_string(),
        agent_session_id: "session-7".to_string(),
        agent_turn_id: "turn-active".to_string(),
        decision_request_id: "request-active".to_string(),
        request_digest: crate::simulator::Digest32::default(),
        memory_snapshot: crate::simulator::MemoryContextSnapshotV1::empty("agent-0"),
        goal_snapshot: crate::simulator::GoalSnapshotV1::empty(),
        continuation: None,
    };
    let active_context = cognition_context::ProviderContextState {
        turn_context,
        request_context,
    };
    first
        .provider_contexts
        .insert("agent-0".to_string(), active_context.clone());
    first
        .provider_retry_contexts
        .insert("agent-0".to_string(), active_context.clone());
    first
        .provider_active_turns
        .insert("agent-0".to_string(), active_context);
    first.provider_agent_ids.insert("agent-0".to_string());
    first.provider_wait_until.insert("agent-0".to_string(), 19);
    first.provider_terminal_states.insert(
        "agent-0".to_string(),
        super::lineage_persistence::ProviderTerminalState {
            agent_turn_id: "turn-1".to_string(),
            decision_request_id: "request-1".to_string(),
            status: "rejected".to_string(),
            reject_reason: Some("no_effect".to_string()),
            feedback_id: Some("feedback-1".to_string()),
        },
    );
    first.schedule_provider_stale_replan("agent-0", "turn-2", "request-4");
    first.mark_provider_transport_exhausted("agent-0".to_string());
    first
        .persist_provider_lineage()
        .expect("persist provider lineage");

    let mut restored = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored.configure_provider_lineage_store(path.clone());
    restored
        .restore_provider_lineage(&RuntimeWorld::default())
        .expect("restore provider lineage");

    assert_eq!(
        restored.provider_session_ids.get("agent-0"),
        Some(&"session-7".to_string())
    );
    assert_eq!(restored.provider_context_seq.get("agent-0"), Some(&4));
    assert_eq!(
        restored
            .provider_contexts
            .get("agent-0")
            .map(|context| context.request_context.transport_attempt),
        Some(2)
    );
    assert!(restored.provider_retry_contexts.contains_key("agent-0"));
    assert!(restored.provider_active_turns.contains_key("agent-0"));
    assert_eq!(restored.provider_wait_until.get("agent-0"), Some(&19));
    assert_eq!(
        restored
            .provider_terminal_states
            .get("agent-0")
            .and_then(|state| state.reject_reason.as_deref()),
        Some("no_effect")
    );
    assert!(restored.provider_stale_replans.contains_key("agent-0"));
    assert!(restored.provider_transport_exhausted.contains("agent-0"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn stale_provider_replans_stop_at_the_bounded_budget() {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    for count in 1..=3 {
        assert!(sidecar.schedule_provider_stale_replan(
            "agent-0",
            format!("turn-{count}").as_str(),
            format!("request-{count}").as_str(),
        ));
        sidecar.mark_provider_stale_replan_dispatched("agent-0");
    }
    assert!(!sidecar.schedule_provider_stale_replan("agent-0", "turn-4", "request-4"));
    assert_eq!(
        sidecar.provider_stale_replan_exhausted_agent().as_deref(),
        Some("agent-0")
    );
}

#[test]
fn provider_lineage_hydrates_sequences_and_session_from_runtime_projection() {
    let mut world = RuntimeWorld::default();
    world
        .enqueue_runtime_feedback(crate::simulator::FeedbackEnvelopeV1 {
            feedback_id: "runtime-feedback:agent-0:7".to_string(),
            feedback_seq: 7,
            agent_subject: "agent-0".to_string(),
            agent_session_id: "persisted-session".to_string(),
            agent_turn_id: "persisted-session-turn-12".to_string(),
            decision_request_id: "persisted-session-request-12".to_string(),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "rejected".to_string(),
            request_digest: crate::simulator::h_v1(
                "oasis7.cognition.request.v1",
                &"persisted-session-request-12",
            ),
            reject_reason: Some("no_effect".to_string()),
            provenance: "runtime_authoritative".to_string(),
        })
        .expect("persist feedback lineage");

    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.hydrate_provider_lineage(&world);

    assert_eq!(
        sidecar
            .provider_session_ids
            .get("agent-0")
            .map(String::as_str),
        Some("persisted-session")
    );
    assert_eq!(sidecar.provider_context_seq.get("agent-0"), Some(&2));
    assert_eq!(sidecar.provider_feedback_seq.get("agent-0"), Some(&8));
}
