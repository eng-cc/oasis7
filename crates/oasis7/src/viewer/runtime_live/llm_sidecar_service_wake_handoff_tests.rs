use super::*;
use crate::world_service::{
    client::{WorldServiceAgentSignerConfig, WorldServiceClientConfig},
    projection::WorldServiceProjection,
    wire::SchedulerOperationV1,
};
use oasis7_client_api::world_service::WorldIdentity;

fn service_cognition() -> RuntimeProviderActionContext {
    let mut request = super::super::super::tests::test_provider_context(
        "agent-handoff-test",
        "turn-handoff-test",
        "request-handoff-test",
        1,
    );
    let request_context = &mut request.request_context;
    let digest = |domain: &str| crate::simulator::h_v1(domain, &"handoff-fixture");
    request_context.runtime_binding.base_world_hash = digest("handoff-fixture-world");
    request_context.runtime_binding.runtime_manifest_hash = digest("handoff-fixture-manifest");
    request_context.observation_digest = digest("handoff-fixture-observation");
    request_context.capability_catalog_digest = digest("handoff-fixture-catalog");
    request_context.capability_invocation_context_digest = digest("handoff-fixture-invocation");
    request_context.memory_snapshot_digest = digest("handoff-fixture-memory");
    request_context.goal_snapshot_digest = digest("handoff-fixture-goal");
    request_context.continuation_digest = digest("handoff-fixture-continuation");
    request_context.request_digest = request_context.request_digest();
    request.turn_context.request_digest = request_context.request_digest.clone();
    let response_context = request.request_context.clone();
    let response = crate::simulator::DecisionResponse::wait("handoff-test-provider");
    RuntimeProviderActionContext {
        request,
        response: crate::simulator::ContinuousAgentResponseContextV1 {
            response_digest: crate::simulator::cognition_response_digest(&response),
            base_decision_response: response,
            context_discriminator: crate::simulator::CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR
                .to_string(),
            context_version: crate::simulator::CONTINUOUS_AGENT_CONTEXT_VERSION,
            agent_session_id: response_context.agent_session_id,
            agent_turn_id: response_context.agent_turn_id,
            decision_request_id: response_context.decision_request_id,
            retry_seq: response_context.retry_seq,
            transport_attempt: response_context.transport_attempt,
            request_digest: response_context.request_digest,
        },
        cognition_lease: None,
        memory_write_intents: Vec::new(),
    }
}

fn wake_for(
    cognition: &RuntimeProviderActionContext,
    wake_id: &str,
) -> crate::runtime::SchedulerWakeV1 {
    let request = &cognition.request.request_context;
    let binding = &request.runtime_binding;
    crate::runtime::SchedulerWakeV1 {
        schema_version: crate::runtime::SchedulerWakeV1::SCHEMA_VERSION.to_string(),
        wake_id: wake_id.to_string(),
        continuation_id: format!("continuation-{wake_id}"),
        world_id: binding.world_id.clone(),
        branch_id: binding.branch_id.clone(),
        finality_epoch: binding.finality_epoch,
        finality_block_hash: binding
            .finality_block_hash
            .as_ref()
            .map(ToString::to_string),
        finality_status: binding.finality_status.clone(),
        reorg_epoch: binding.reorg_epoch,
        runtime_manifest_hash: binding.runtime_manifest_hash.to_string(),
        agent_id: request.agent_subject.clone(),
        agent_session_id: request.agent_session_id.clone(),
        agent_turn_id: request.agent_turn_id.clone(),
        decision_request_id: request.decision_request_id.clone(),
        request_digest: request.request_digest.to_string(),
        next_wake_tick: 12,
        eligible_since_tick: 12,
        starvation_deadline_tick: 20,
        initial_priority: 0,
        wake_seq: 1,
        retry_seq: 0,
        status: "pending".into(),
        pending_reason: "capacity_available".into(),
    }
}

fn configured_sidecar(
    cognition: &RuntimeProviderActionContext,
    wakes: Vec<crate::runtime::SchedulerWakeV1>,
    lineage_path: std::path::PathBuf,
) -> RuntimeLlmSidecar {
    let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    let binding = cognition.request.request_context.runtime_binding.clone();
    let mut projection =
        WorldServiceProjection::from_world(&RuntimeWorld::new(), None).expect("projection");
    projection.runtime_binding = Some(binding.clone());
    projection.scheduler_wakes = wakes;
    sidecar.provider_service_projection = Some(projection);
    sidecar.provider_service_config = Some(WorldServiceClientConfig {
        endpoint: "http://127.0.0.1:1".into(),
        trusted_service_public_key: hex::encode([12; 32]),
        expected_world: WorldIdentity {
            world_id: binding.world_id,
            genesis_digest: "0".repeat(64),
        },
        scope_id: "public".into(),
        read_private_key_hex: hex::encode([13; 32]),
        timeout: std::time::Duration::from_millis(10),
        max_response_bytes: 4096,
    });
    sidecar.provider_service_signer = Some(WorldServiceAgentSignerConfig {
        private_key_hex: hex::encode([14; 32]),
        delegation_generation: 3,
    });
    sidecar.provider_lineage_store = Some(lineage_path);
    sidecar.provider_service_lineage_store_explicit = true;
    sidecar.provider_service_required = true;
    sidecar
}

fn handoff_operation(wake_id: &str) -> SchedulerOperationV1 {
    let digest = |name: &str| crate::simulator::h_v1("oasis7.test.handoff.v1", &name).to_string();
    SchedulerOperationV1::HandoffWake {
        wake_id: wake_id.to_string(),
        disposition: crate::runtime::CognitionWakeDispositionV1::Terminal {
            status: crate::runtime::ContinuationStatusV1::Completed,
            reason: "provider_action_committed".into(),
        },
        current_context: crate::runtime::CognitionContextDigestsV1 {
            baseline_observation_digest: digest("observation"),
            goal_digest: digest("goal"),
            policy_digest: digest("policy"),
            precondition_digest: digest("precondition"),
        },
    }
}

fn lineage_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "oasis7-service-wake-{label}-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}

#[test]
fn service_handoff_selects_exact_request_and_reuses_signed_checkpoint() {
    let cognition = service_cognition();
    let target = wake_for(&cognition, "wake-exact-handoff");
    let mut unrelated = target.clone();
    unrelated.wake_id = "wake-other-turn".into();
    unrelated.continuation_id = "continuation-other-turn".into();
    unrelated.agent_turn_id = "turn-other".into();
    unrelated.request_digest = crate::simulator::h_v1("other-request", &"different").to_string();
    let path = lineage_path("exact-checkpoint");
    let mut sidecar = configured_sidecar(&cognition, vec![unrelated, target.clone()], path.clone());
    let request = &cognition.request.request_context;
    let phase = format!("handoff:{}", target.wake_id);
    let operation = handoff_operation(&target.wake_id);
    let (checkpoint, existed) = sidecar
        .prepare_service_scheduler_checkpoint(request, &phase, operation.clone(), None)
        .expect("persist exact signed handoff checkpoint");
    assert!(!existed);
    let checkpoint_bytes = serde_json::to_vec(&checkpoint).expect("checkpoint bytes");

    let (selected_wake, selected_operation) = sidecar
        .committed_service_wake_handoff(&cognition)
        .expect("select exact wake")
        .expect("matching wake exists");
    assert_eq!(selected_wake, target);
    assert_eq!(selected_operation, operation);
    let (retried, existed) = sidecar
        .prepare_service_scheduler_checkpoint(request, &phase, selected_operation, None)
        .expect("reuse exact signed handoff checkpoint");
    assert!(existed);
    assert_eq!(
        serde_json::to_vec(&retried).expect("retried bytes"),
        checkpoint_bytes,
        "recovery must not replace the signed request payload"
    );
    let persisted = std::fs::read(&path).expect("durable lineage checkpoint");
    assert!(
        String::from_utf8_lossy(&persisted).contains(&target.wake_id),
        "the selected handoff request remains persisted"
    );
    std::fs::remove_file(path).expect("remove fixture checkpoint");
}

#[test]
fn service_handoff_recovers_mirror_only_wake_from_exact_signed_checkpoint() {
    let cognition = service_cognition();
    let target = wake_for(&cognition, "wake-mirror-recovery");
    let path = lineage_path("mirror-recovery");
    let mut sidecar = configured_sidecar(&cognition, Vec::new(), path.clone());
    sidecar
        .pending_runtime_wakes
        .insert(target.wake_id.clone(), target.clone());

    let request = &cognition.request.request_context;
    let phase = format!("handoff:{}", target.wake_id);
    let operation = handoff_operation(&target.wake_id);
    let (checkpoint, existed) = sidecar
        .prepare_service_scheduler_checkpoint(request, &phase, operation.clone(), None)
        .expect("persist original signed handoff checkpoint alongside durable wake mirror");
    assert!(!existed);
    let checkpoint_bytes = serde_json::to_vec(&checkpoint).expect("checkpoint bytes");

    let (selected_wake, selected_operation) = sidecar
        .committed_service_wake_handoff(&cognition)
        .expect("recovered mirror is fenced by its original checkpoint")
        .expect("exact durable mirror is restored");
    assert_eq!(selected_wake, target);
    assert_eq!(selected_operation, operation);
    assert_eq!(
        serde_json::to_vec(
            &sidecar.provider_scheduler_pending[&format!(
                "{}:handoff:{}",
                request.provider_invocation_key(),
                target.wake_id
            )]
        )
        .expect("checkpoint bytes after recovery"),
        checkpoint_bytes,
        "recovery reuses the original signed request without replacement"
    );
    assert_eq!(
        sidecar
            .provider_service_projection
            .as_ref()
            .expect("verified projection")
            .scheduler_wakes,
        Vec::<crate::runtime::SchedulerWakeV1>::new(),
        "mirror recovery does not synthesize a projection wake"
    );
    std::fs::remove_file(path).expect("remove fixture checkpoint");
}

#[test]
fn service_handoff_fences_mirror_only_wake_without_signed_checkpoint() {
    let cognition = service_cognition();
    let target = wake_for(&cognition, "wake-mirror-without-checkpoint");
    let path = lineage_path("mirror-without-checkpoint");
    let mut sidecar = configured_sidecar(&cognition, Vec::new(), path.clone());
    sidecar
        .pending_runtime_wakes
        .insert(target.wake_id.clone(), target);

    let error = sidecar
        .committed_service_wake_handoff(&cognition)
        .expect_err("a mirror without the original signed checkpoint fails closed");
    assert!(error.contains("absent from verified projection"), "{error}");
    assert!(sidecar.provider_scheduler_pending.is_empty());
    assert!(
        !path.exists(),
        "the fenced recovery does not sign or persist work"
    );
}

#[test]
fn service_handoff_cleanup_removes_wake_and_checkpoint_atomically() {
    let cognition = service_cognition();
    let target = wake_for(&cognition, "wake-cleanup-atomic");
    let path = lineage_path("atomic-cleanup");
    let mut sidecar = configured_sidecar(&cognition, vec![target.clone()], path.clone());
    sidecar
        .pending_runtime_wakes
        .insert(target.wake_id.clone(), target.clone());
    let request = &cognition.request.request_context;
    let checkpoint_id = format!(
        "{}:handoff:{}",
        request.provider_invocation_key(),
        target.wake_id
    );
    sidecar
        .prepare_service_scheduler_checkpoint(
            request,
            &format!("handoff:{}", target.wake_id),
            handoff_operation(&target.wake_id),
            None,
        )
        .expect("persist exact handoff checkpoint");

    let blocker = path.with_extension("blocker");
    std::fs::write(&blocker, b"not a directory").expect("persistence blocker");
    sidecar.provider_lineage_store = Some(blocker.join("checkpoint.json"));
    assert!(
        sidecar
            .complete_committed_service_wake_handoff(&cognition, &target.wake_id)
            .is_err()
    );
    assert_eq!(
        sidecar.pending_runtime_wakes.get(&target.wake_id),
        Some(&target),
        "failed persistence restores the wake mirror"
    );
    assert!(
        sidecar
            .provider_scheduler_pending
            .contains_key(&checkpoint_id)
    );

    sidecar.provider_lineage_store = Some(path.clone());
    sidecar
        .complete_committed_service_wake_handoff(&cognition, &target.wake_id)
        .expect("persist both durable removals together");
    assert!(!sidecar.pending_runtime_wakes.contains_key(&target.wake_id));
    assert!(
        !sidecar
            .provider_scheduler_pending
            .contains_key(&checkpoint_id)
    );
    let persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("durable lineage"))
            .expect("lineage JSON");
    assert!(
        persisted["pending_runtime_wakes"]
            .get(&target.wake_id)
            .is_none()
    );
    assert!(
        persisted["provider_scheduler_pending"]
            .get(&checkpoint_id)
            .is_none()
    );
    std::fs::remove_file(path).expect("remove fixture checkpoint");
    std::fs::remove_file(blocker).expect("remove fixture blocker");
}

#[test]
fn service_handoff_does_not_consume_an_unrelated_same_agent_wake() {
    let cognition = service_cognition();
    let mut target = wake_for(&cognition, "wake-other-request");
    target.agent_turn_id = "turn-other".into();
    target.request_digest = crate::simulator::h_v1("other-request", &"different").to_string();
    let path = lineage_path("unrelated-wake");
    let mut sidecar = configured_sidecar(&cognition, vec![target.clone()], path.clone());
    assert!(
        sidecar
            .committed_service_wake_handoff(&cognition)
            .expect("unrelated wake is ignored")
            .is_none()
    );
    assert!(sidecar.provider_scheduler_pending.is_empty());
    assert_eq!(
        sidecar
            .provider_service_projection
            .as_ref()
            .expect("projection")
            .scheduler_wakes,
        vec![target]
    );
    assert!(!path.exists(), "no checkpoint is written for another turn");
}

#[test]
fn service_handoff_does_not_consume_request_wake_from_other_runtime_binding() {
    let cognition = service_cognition();
    let mut target = wake_for(&cognition, "wake-other-world-binding");
    target.world_id = "another-world".into();
    let path = lineage_path("other-world-binding");
    let mut sidecar = configured_sidecar(&cognition, vec![target.clone()], path.clone());

    assert!(
        sidecar
            .committed_service_wake_handoff(&cognition)
            .expect("wake with another binding is ignored")
            .is_none()
    );
    assert!(sidecar.provider_scheduler_pending.is_empty());
    assert_eq!(
        sidecar
            .provider_service_projection
            .as_ref()
            .expect("projection")
            .scheduler_wakes,
        vec![target]
    );
    assert!(!path.exists(), "no checkpoint is written for another world");
}

#[test]
fn service_handoff_fences_multiple_exact_wakes_without_selecting_one() {
    let cognition = service_cognition();
    let target = wake_for(&cognition, "wake-duplicate-exact");
    let mut duplicate = target.clone();
    duplicate.wake_id = "wake-duplicate-exact-2".into();
    duplicate.continuation_id = "continuation-wake-duplicate-exact-2".into();
    let path = lineage_path("ambiguous-exact-wakes");
    let mut sidecar = configured_sidecar(&cognition, vec![target, duplicate], path.clone());
    let error = sidecar
        .committed_service_wake_handoff(&cognition)
        .expect_err("ambiguous exact target wakes are fenced");
    assert!(error.contains("ambiguous"), "{error}");
    assert!(sidecar.provider_scheduler_pending.is_empty());
    assert!(
        !path.exists(),
        "ambiguous wake selection has no side effect"
    );
}
