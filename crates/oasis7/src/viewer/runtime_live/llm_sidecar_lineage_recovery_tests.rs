use super::*;

fn bound_provider_lease_test_world(agent_ids: &[&str]) -> RuntimeWorld {
    let mut world = RuntimeWorld::new();
    world
        .bind_cognition_runtime(
            "lease-recovery-world",
            "lease-recovery-branch",
            0,
            None,
            "pending",
            0,
        )
        .expect("Runtime cognition binding");
    for agent_id in agent_ids {
        world.submit_action(RuntimeAction::RegisterAgent {
            agent_id: (*agent_id).to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
    }
    world.step().expect("register Runtime agents");
    for agent_id in agent_ids {
        world
            .install_test_provider_capability_fixture(agent_id)
            .expect("install Runtime provider capability fixture");
    }
    world
}

fn reserve_test_provider_lease(
    world: &mut RuntimeWorld,
    context: &cognition_context::ProviderContextState,
) -> crate::runtime::CognitionLeaseV1 {
    crate::viewer::runtime_live::control_plane::llm_sidecar::async_support::reserve_provider_cognition_lease(
        world, context,
    )
    .expect("reserve Runtime cognition lease")
}

fn valid_test_provider_context(
    world: &RuntimeWorld,
    agent_id: &str,
    agent_turn_id: &str,
    decision_request_id: &str,
) -> cognition_context::ProviderContextState {
    let mut context = test_provider_context(agent_id, agent_turn_id, decision_request_id, 1);
    let request = &mut context.request_context;
    request.runtime_binding.base_world_hash =
        crate::simulator::h_v1("oasis7.cognition.test.world.v1", &agent_id);
    request.runtime_binding.runtime_manifest_hash =
        crate::simulator::h_v1("oasis7.cognition.test.manifest.v1", &agent_id);
    request.observation_digest = crate::simulator::h_v1(
        "oasis7.cognition.test.observation.v1",
        &request.base_decision_request.observation,
    );
    request.capability_catalog_digest =
        crate::simulator::h_v1("oasis7.cognition.test.capability-catalog.v1", &agent_id);
    request.capability_invocation_context_digest =
        crate::simulator::h_v1("oasis7.cognition.test.capability-invocation.v1", &agent_id);
    request.memory_snapshot_digest =
        crate::simulator::h_v1("oasis7.cognition.test.memory.v1", &agent_id);
    request.goal_snapshot_digest =
        crate::simulator::h_v1("oasis7.cognition.test.goal.v1", &agent_id);
    request.continuation_digest =
        crate::simulator::h_v1("oasis7.cognition.test.continuation.v1", &agent_id);
    request.runtime_binding = world
        .current_cognition_runtime_binding()
        .expect("Runtime cognition binding");
    request.request_digest = request.request_digest();
    context.turn_context.request_digest = request.request_digest.clone();
    context
}

#[test]
fn provider_lineage_restore_fences_cross_request_lease_before_dispatch() {
    let mut world = bound_provider_lease_test_world(&["agent-a"]);
    let old_context = valid_test_provider_context(&world, "agent-a", "turn-old", "request-old");
    let new_context = valid_test_provider_context(&world, "agent-a", "turn-new", "request-new");
    let old_lease = reserve_test_provider_lease(&mut world, &old_context);
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-cross-lease-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut first = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    first.configure_provider_lineage_store(path.clone());
    first.provider_agent_ids.insert("agent-a".to_string());
    first
        .provider_contexts
        .insert("agent-a".to_string(), new_context.clone());
    first
        .provider_active_turns
        .insert("agent-a".to_string(), new_context);
    first
        .provider_cognition_leases
        .insert("agent-a".to_string(), old_lease);
    first
        .persist_provider_lineage()
        .expect("persist cross-request provider lease");
    let economy_before = world
        .cognition_economy()
        .expect("read economy before restore");

    let mut restarted = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restarted.configure_provider_lineage_store(path.clone());
    restarted.hydrate_provider_lineage(&world);
    assert!(
        restarted
            .provider_lineage_recovery_pending
            .as_deref()
            .is_some_and(|reason| reason.contains("provider cognition lease")),
        "cross-request lease must create a durable recovery fence"
    );
    assert!(
        restarted.provider_cognition_leases.is_empty(),
        "invalid lease must not become an installed provider authority"
    );
    let mut kernel = WorldKernel::new();
    let error = restarted
        .prepare_provider_request_contexts(&mut world, &mut kernel, "lease-recovery-world")
        .expect_err("recovery fence must block provider dispatch");
    assert!(error.contains("provider lineage recovery fenced"));
    assert_eq!(
        world
            .cognition_economy()
            .expect("read economy after restore"),
        economy_before,
        "restore fencing must not mutate Runtime economy"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn provider_lease_release_fences_cross_request_without_economic_mutation() {
    let mut world = bound_provider_lease_test_world(&["agent-a"]);
    let old_context = valid_test_provider_context(&world, "agent-a", "turn-old", "request-old");
    let new_context = valid_test_provider_context(&world, "agent-a", "turn-new", "request-new");
    let old_lease = reserve_test_provider_lease(&mut world, &old_context);
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar
        .provider_contexts
        .insert("agent-a".to_string(), new_context);
    sidecar
        .provider_cognition_leases
        .insert("agent-a".to_string(), old_lease);
    sidecar.provider_wait_until.insert("agent-a".to_string(), 0);
    let economy_before = world
        .cognition_economy()
        .expect("read economy before release");

    let error = sidecar
        .release_due_provider_waits(&mut world)
        .expect_err("cross-request lease must be rejected before release");
    assert!(error.contains("cognition lease identity mismatch"));
    assert_eq!(
        world
            .cognition_economy()
            .expect("read economy after release"),
        economy_before,
        "identity rejection must not emit a Runtime economic receipt"
    );
    assert!(
        sidecar.provider_cognition_leases.contains_key("agent-a"),
        "rejected lease remains available for durable recovery inspection"
    );
}

#[test]
fn provider_lease_reserve_fences_stale_runtime_binding_without_economic_mutation() {
    let mut world = bound_provider_lease_test_world(&["agent-a"]);
    let mut context = valid_test_provider_context(&world, "agent-a", "turn-stale", "request-stale");
    context.request_context.runtime_binding.branch_id = "stale-branch".to_string();
    context.request_context.request_digest = context.request_context.request_digest();
    context.turn_context.request_digest = context.request_context.request_digest.clone();
    let economy_before = world
        .cognition_economy()
        .expect("read economy before stale reserve");

    let error = crate::viewer::runtime_live::control_plane::llm_sidecar::async_support::reserve_provider_cognition_lease(
        &mut world,
        &context,
    )
        .expect_err("stale Runtime binding must be rejected before reserve");
    assert!(error.contains("Runtime binding changed before lease reserve"));
    assert_eq!(
        world
            .cognition_economy()
            .expect("read economy after stale reserve"),
        economy_before,
        "stale binding rejection must not reserve balance or emit a receipt"
    );
}

#[test]
fn malformed_runtime_wake_projection_fences_provider_dispatch_and_persists_recovery() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-viewer-provider-lineage-runtime-wake-fence-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let valid_world = RuntimeWorld::default();
    let mut snapshot = valid_world.snapshot();
    snapshot.cognition["scheduler_state"] = serde_json::json!({
        "schema_version": "scheduler.v1",
        "in_flight": "malformed"
    });
    let mut malformed_world = RuntimeWorld::from_snapshot(snapshot, valid_world.journal().clone())
        .expect("malformed scheduler projection remains loadable for adapter fault injection");

    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    sidecar.configure_provider_lineage_store(path.clone());
    sidecar.provider_agent_ids.insert("agent-0".to_string());
    sidecar.hydrate_provider_lineage(&malformed_world);

    let recovery_reason = sidecar
        .provider_lineage_recovery_pending
        .as_deref()
        .expect("unreadable Runtime wake projection must retain a recovery fence")
        .to_string();
    assert!(recovery_reason.contains("Runtime cognition wake read failed"));
    assert!(
        !sidecar.provider_lineage_hydrated,
        "wake projection failure must not be marked as successfully hydrated"
    );
    let checkpoint: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).expect("read persisted wake projection recovery fence"),
    )
    .expect("decode persisted wake projection recovery fence");
    assert_eq!(
        checkpoint["provider_lineage_recovery_pending"],
        serde_json::json!(recovery_reason)
    );

    let mut kernel = WorldKernel::new();
    let error = sidecar
        .prepare_provider_request_contexts(&mut malformed_world, &mut kernel, "fault-world")
        .expect_err("provider admission must stop behind the Runtime recovery fence");
    assert!(error.contains("provider lineage recovery fenced"));

    let mut restarted = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restarted.configure_provider_lineage_store(path.clone());
    restarted.hydrate_provider_lineage(&malformed_world);
    assert!(
        restarted
            .provider_lineage_recovery_pending
            .as_deref()
            .is_some_and(|reason| reason.contains("Runtime cognition wake read failed")),
        "restart must retain an actionable recovery reason"
    );
    assert!(
        restarted.provider_lineage_hydrated,
        "restart may mark the persisted recovery fence hydrated, but admission remains blocked"
    );

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
    assert!(
        restored
            .provider_transport_exhausted
            .contains("agent-missing"),
        "missing active identity must enter the durable Runtime terminalization path"
    );
    assert!(
        restored
            .provider_transport_exhausted
            .contains("agent-mismatch"),
        "mismatched active identity must enter the durable Runtime terminalization path"
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
