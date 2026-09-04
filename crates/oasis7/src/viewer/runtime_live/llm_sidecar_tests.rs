use super::*;

fn test_provider_context(
    agent_id: &str,
    agent_turn_id: &str,
    decision_request_id: &str,
    transport_attempt: u64,
) -> cognition_context::ProviderContextState {
    let request_context = crate::simulator::ContinuousAgentRequestContextV1 {
        base_decision_request: crate::simulator::DecisionRequest {
            observation: crate::simulator::ObservationEnvelope {
                agent_id: agent_id.to_string(),
                world_time: 12,
                mode: crate::simulator::ProviderExecutionMode::PlayerParity,
                observation_schema_version:
                    crate::simulator::DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
                action_schema_version: crate::simulator::DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION
                    .to_string(),
                environment_class: Some("runtime_live".to_string()),
                fallback_reason: None,
                observation: crate::simulator::ProviderObservation {
                    self_state: crate::simulator::ProviderSelfState {
                        location_ref: "loc-0".to_string(),
                        pose_hint: "origin".to_string(),
                        status_flags: Vec::new(),
                        resource_summary: BTreeMap::new(),
                    },
                    mission_context: crate::simulator::ProviderMissionContext {
                        goal_summary: "test goal".to_string(),
                        blocked_reason: None,
                    },
                    nearby_entities: Vec::new(),
                    recent_events: Vec::new(),
                    local_navigation_graph: Vec::new(),
                    hazard_summary: Vec::new(),
                    interaction_targets: Vec::new(),
                },
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
        agent_session_id: "session-test".to_string(),
        agent_turn_id: agent_turn_id.to_string(),
        decision_request_id: decision_request_id.to_string(),
        retry_seq: 1,
        transport_attempt,
        agent_subject: agent_id.to_string(),
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
        agent_id: agent_id.to_string(),
        agent_session_id: "session-test".to_string(),
        agent_turn_id: agent_turn_id.to_string(),
        decision_request_id: decision_request_id.to_string(),
        request_digest: crate::simulator::Digest32::default(),
        memory_snapshot: crate::simulator::MemoryContextSnapshotV1::empty(agent_id),
        goal_snapshot: crate::simulator::GoalSnapshotV1::empty(),
        continuation: None,
    };
    cognition_context::ProviderContextState {
        turn_context,
        request_context,
    }
}

fn test_recipe_completion_event(
    agent_id: &str,
    event_id: u64,
) -> (crate::runtime::WorldEvent, WorldEvent) {
    let runtime_event = crate::runtime::WorldEvent {
        id: event_id,
        time: 83,
        caused_by: None,
        body: crate::runtime::WorldEventBody::Domain(
            crate::runtime::DomainEvent::RecipeCompleted {
                job_id: 7,
                requester_agent_id: agent_id.to_string(),
                factory_id: "factory-smelter".to_string(),
                recipe_id: "recipe.iron-ingot".to_string(),
                accepted_batches: 1,
                produce: Vec::new(),
                byproducts: Vec::new(),
                output_ledger: crate::runtime::MaterialLedgerId::world(),
                bottleneck_tags: Vec::new(),
                logistics_route_ids: Vec::new(),
                logistics_path_ids: Vec::new(),
            },
        ),
    };
    let mapped_event = WorldEvent {
        id: event_id,
        time: 83,
        kind: WorldEventKind::RuntimeEvent {
            kind: "runtime.economy.recipe_completed".to_string(),
            domain_kind: Some("recipe=recipe.iron-ingot".to_string()),
        },
        runtime_event: Some(runtime_event.clone()),
    };
    (runtime_event, mapped_event)
}

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
fn provider_lineage_restore_requeues_retryable_held_outcome_as_restart_retry() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-held-retry-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut first = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    first.configure_provider_lineage_store(path.clone());
    first.provider_agent_ids.insert("agent-0".to_string());
    first.provider_contexts.insert(
        "agent-0".to_string(),
        test_provider_context("agent-0", "turn-retry", "request-retry", 1),
    );
    first.provider_held_decisions.insert(
        "agent-0".to_string(),
        async_support::RuntimeLlmDecision {
            agent_id: "agent-0".to_string(),
            decision: crate::simulator::AgentDecision::Wait,
            decision_trace: Some(crate::simulator::AgentDecisionTrace {
                agent_id: "agent-0".to_string(),
                time: 12,
                decision: crate::simulator::AgentDecision::Wait,
                llm_input: None,
                llm_output: Some(r#"{"provider_error":{"retryable":true}}"#.to_string()),
                llm_error: None,
                parse_error: None,
                llm_diagnostics: None,
                llm_effect_intents: Vec::new(),
                llm_effect_receipts: Vec::new(),
                llm_step_trace: Vec::new(),
                llm_prompt_section_trace: Vec::new(),
                llm_chat_messages: Vec::new(),
            }),
            cognition: None,
            memory_write_intents: Vec::new(),
            continuation_admitted: false,
        },
    );
    first
        .persist_provider_lineage()
        .expect("persist held retryable provider lineage");

    let mut restored = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored.configure_provider_lineage_store(path.clone());
    restored
        .restore_provider_lineage(&RuntimeWorld::default())
        .expect("restore held retryable provider lineage");

    assert!(!restored.provider_held_decisions.contains_key("agent-0"));
    assert!(restored.provider_completed_decisions.is_empty());
    let retry = restored
        .provider_retry_contexts
        .get("agent-0")
        .expect("retryable held outcome must be requeued after restart");
    assert_eq!(retry.request_context.agent_turn_id, "turn-retry");
    assert_eq!(retry.request_context.decision_request_id, "request-retry");
    assert_eq!(retry.request_context.transport_attempt, 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn provider_lineage_restore_requeues_orphaned_active_context_without_runtime_wake() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-orphan-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut first = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    first.configure_provider_lineage_store(path.clone());
    let context = test_provider_context("agent-0", "turn-orphaned", "request-orphaned", 1);
    first
        .provider_contexts
        .insert("agent-0".to_string(), context.clone());
    first
        .provider_active_turns
        .insert("agent-0".to_string(), context);
    first.provider_agent_ids.insert("agent-0".to_string());
    first
        .persist_provider_lineage()
        .expect("persist orphaned provider lineage");

    let mut restored = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored.configure_provider_lineage_store(path.clone());
    restored
        .restore_provider_lineage(&RuntimeWorld::default())
        .expect("restore orphaned provider lineage");

    assert!(
        !restored.provider_active_turns.contains_key("agent-0"),
        "process-local active marker must not survive a restart"
    );
    let retry = restored
        .provider_retry_contexts
        .get("agent-0")
        .expect("orphaned context must be eligible for bounded retry");
    assert_eq!(retry.request_context.agent_turn_id, "turn-orphaned");
    assert_eq!(
        retry.request_context.decision_request_id,
        "request-orphaned"
    );
    assert_eq!(retry.request_context.transport_attempt, 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn provider_lineage_restore_terminalizes_exhausted_orphan_without_retry_loop() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-exhausted-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut first = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    first.configure_provider_lineage_store(path.clone());
    let context = test_provider_context(
        "agent-0",
        "turn-exhausted",
        "request-exhausted",
        MAX_PROVIDER_TRANSPORT_ATTEMPTS,
    );
    first
        .provider_contexts
        .insert("agent-0".to_string(), context.clone());
    first
        .provider_active_turns
        .insert("agent-0".to_string(), context);
    first.provider_agent_ids.insert("agent-0".to_string());
    first
        .persist_provider_lineage()
        .expect("persist exhausted provider lineage");

    let mut restored = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored.configure_provider_lineage_store(path.clone());
    restored
        .restore_provider_lineage(&RuntimeWorld::default())
        .expect("restore exhausted provider lineage");

    assert!(!restored.provider_active_turns.contains_key("agent-0"));
    assert!(!restored.provider_retry_contexts.contains_key("agent-0"));
    assert!(restored.provider_transport_exhausted.contains("agent-0"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn provider_lineage_restore_quarantines_missing_or_mismatched_active_identity() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-quarantine-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut first = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    first.configure_provider_lineage_store(path.clone());
    let context = test_provider_context("agent-missing", "turn-missing", "request-missing", 1);
    first
        .provider_active_turns
        .insert("agent-missing".to_string(), context.clone());
    first.provider_agent_ids.insert("agent-missing".to_string());
    let mut mismatched = context.clone();
    mismatched.request_context.agent_turn_id = "turn-newer".to_string();
    mismatched.turn_context.agent_turn_id = "turn-newer".to_string();
    first
        .provider_contexts
        .insert("agent-mismatch".to_string(), mismatched.clone());
    first
        .provider_active_turns
        .insert("agent-mismatch".to_string(), context);
    first
        .provider_agent_ids
        .insert("agent-mismatch".to_string());
    first
        .persist_provider_lineage()
        .expect("persist quarantined provider lineage");

    let mut restored = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored.configure_provider_lineage_store(path.clone());
    restored
        .restore_provider_lineage(&RuntimeWorld::default())
        .expect("restore quarantined provider lineage");

    assert!(!restored.provider_active_turns.contains_key("agent-missing"));
    assert!(
        !restored
            .provider_active_turns
            .contains_key("agent-mismatch")
    );
    assert_eq!(
        restored
            .provider_recovery_pending
            .get("agent-missing")
            .map(|record| record.reason.as_str()),
        Some("active_context_missing")
    );
    assert_eq!(
        restored
            .provider_recovery_pending
            .get("agent-mismatch")
            .map(|record| record.reason.as_str()),
        Some("active_context_identity_mismatch")
    );
    assert!(restored.provider_contexts.contains_key("agent-mismatch"));
    assert!(
        !restored
            .provider_retry_contexts
            .contains_key("agent-mismatch")
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn recipe_completion_before_runner_registration_is_durable_and_deduplicated() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-world-event-startup-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let (runtime_event, mapped_event) = test_recipe_completion_event("agent-0", 111);
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.configure_provider_lineage_store(path.clone());
    sidecar.notify_recipe_completion_if_needed(&runtime_event, mapped_event.clone());
    sidecar.notify_recipe_completion_if_needed(&runtime_event, mapped_event);
    assert_eq!(sidecar.pending_provider_world_events.len(), 1);
    let checkpoint = std::fs::read_to_string(&path).expect("startup event checkpoint");
    assert!(checkpoint.contains("pending_provider_world_events"));
    assert!(checkpoint.contains("111"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn queued_recipe_completion_from_replaced_runtime_head_is_quarantined() {
    let (runtime_event, mapped_event) = test_recipe_completion_event("agent-0", 112);
    let old_binding = test_provider_context("agent-0", "turn-old", "request-old", 1)
        .request_context
        .runtime_binding;
    let mut new_binding = old_binding.clone();
    new_binding.branch_id = "replacement".to_string();
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.provider_lineage_binding = Some(new_binding);
    sidecar.notify_recipe_completion_with_binding(&runtime_event, mapped_event, Some(old_binding));
    assert!(sidecar.pending_provider_world_events.is_empty());
    assert_eq!(
        sidecar.provider_world_event_quarantine.len(),
        1,
        "a replacement Runtime head must not deliver an old completion"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn provider_world_event_for_missing_actor_is_quarantined_once() {
    let (runtime_event, mapped_event) = test_recipe_completion_event("agent-missing", 113);
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.runner = Some(RuntimeDecisionRunner::ProviderBacked(
        crate::simulator::AsyncAgentRunner::with_default_capacity(),
    ));

    sidecar.notify_recipe_completion_if_needed(&runtime_event, mapped_event);

    assert!(sidecar.pending_provider_world_events.is_empty());
    assert_eq!(sidecar.provider_world_event_quarantine.len(), 1);
    assert!(
        sidecar
            .provider_world_event_quarantine
            .values()
            .any(|reason| reason == "provider_actor_missing:agent-missing")
    );
}

#[test]
fn provider_feedback_compatibility_cursor_is_partitioned_by_session() {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    let first = test_provider_context("agent-0", "turn-a", "request-a", 1).request_context;
    let mut second = first.clone();
    second.agent_session_id = "session-other".to_string();
    let first_feedback = sidecar.provider_feedback_for_request(
        &first,
        None,
        "rejected",
        None,
        None,
        Some("no_effect".to_string()),
    );
    let second_feedback = sidecar.provider_feedback_for_request(
        &second,
        None,
        "rejected",
        None,
        None,
        Some("no_effect".to_string()),
    );
    assert_eq!(first_feedback.feedback_seq, 1);
    assert_eq!(second_feedback.feedback_seq, 1);
    assert_ne!(first_feedback.feedback_id, second_feedback.feedback_id);
    assert_eq!(sidecar.provider_feedback_seq_by_session.len(), 2);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn provider_world_event_mailbox_full_remains_durable_for_retry() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-world-event-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let (runtime_event, mapped_event) = test_recipe_completion_event("agent-0", 77);
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.configure_provider_lineage_store(path.clone());
    sidecar.runner = Some(RuntimeDecisionRunner::ProviderBacked(
        crate::simulator::AsyncAgentRunner::blocking_provider_fixture("agent-0"),
    ));
    let saturated = {
        let Some(RuntimeDecisionRunner::ProviderBacked(runner)) = sidecar.runner.as_mut() else {
            panic!("provider runner fixture");
        };
        runner
            .start_turn("agent-0")
            .expect("start blocking fixture turn");
        let capacity = runner.mailbox_capacity();
        let mut full = false;
        for _ in 0..capacity.saturating_add(1) {
            match runner.notify_world_event("agent-0", mapped_event.clone()) {
                Ok(()) => {}
                Err(crate::simulator::AsyncAgentRunnerError::FeedbackUnavailable(_)) => {
                    full = true;
                    break;
                }
                Err(error) => panic!("unexpected mailbox fixture error: {error}"),
            }
        }
        full
    };
    assert!(saturated, "fixture must exercise the full mailbox path");
    std::thread::sleep(std::time::Duration::from_millis(20));

    // The actor thread starts concurrently with the fixture setup. If it
    // consumes the first queued event before the sidecar call, refill the
    // bounded mailbox and exercise the same error path again.
    for _ in 0..4 {
        sidecar.notify_recipe_completion_if_needed(&runtime_event, mapped_event.clone());
        if sidecar.pending_provider_world_events.len() == 1 {
            break;
        }
        let Some(RuntimeDecisionRunner::ProviderBacked(runner)) = sidecar.runner.as_mut() else {
            panic!("provider runner fixture");
        };
        let capacity = runner.mailbox_capacity();
        let mut full = false;
        for _ in 0..capacity.saturating_add(1) {
            match runner.notify_world_event("agent-0", mapped_event.clone()) {
                Ok(()) => {}
                Err(crate::simulator::AsyncAgentRunnerError::FeedbackUnavailable(_)) => {
                    full = true;
                    break;
                }
                Err(error) => panic!("unexpected mailbox fixture error: {error}"),
            }
        }
        if !full {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    assert_eq!(sidecar.pending_provider_world_events.len(), 1);
    let checkpoint = std::fs::read_to_string(&path).expect("pending event checkpoint");
    assert!(checkpoint.contains("pending_provider_world_events"));
    assert!(checkpoint.contains("agent-0"));
    let mut restored = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored.configure_provider_lineage_store(path.clone());
    restored
        .restore_provider_lineage(&RuntimeWorld::default())
        .expect("restore pending provider world event");
    assert_eq!(restored.pending_provider_world_events.len(), 1);
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
