use super::*;
use oasis7::capability_invocation_context::CapabilityInvocationContext;
use oasis7::simulator::{
    COGNITION_CAPABILITY_CATALOG_DOMAIN, COGNITION_CAPABILITY_INVOCATION_CONTEXT_DOMAIN,
};
use oasis7_wasm_abi::CapabilityCatalogSnapshot;

#[derive(Debug, Clone)]
struct RecordingInvoker {
    response: Result<AgentInvocationOutput, String>,
    invocations: Arc<Mutex<Vec<AgentInvocation>>>,
}

impl AgentInvoker for RecordingInvoker {
    fn invoke(&self, invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
        self.invocations
            .lock()
            .expect("recording invocations lock")
            .push(invocation);
        self.response.clone()
    }
}

#[test]
fn continuous_feedback_holds_gaps_and_drains_in_order_with_bounded_diagnostics() {
    let state = ProviderState::new(CliOptions {
        mode: ProviderMode::Mock,
        ..CliOptions::default()
    })
    .expect("build mock provider state");
    let accepted_context = continuous_context("session-gap", 1);
    state
        .handle_continuous_decision(
            accepted_context.clone(),
            None,
            &RecordingInvoker {
                response: Ok(AgentInvocationOutput {
                    prompt: "prompt".to_string(),
                    text: r#"{"decision":"wait"}"#.to_string(),
                    provider_version: Some("provider/test".to_string()),
                    duration_ms: Some(1),
                    prompt_tokens: None,
                    completion_tokens: None,
                    total_tokens: None,
                    route_note: None,
                    upstream_trace: None,
                }),
                invocations: Arc::new(Mutex::new(Vec::new())),
            },
        )
        .expect("register the feedback lineage before sequencing it");
    let feedback = |seq: u64| FeedbackEnvelopeV1 {
        feedback_id: format!("feedback-gap-{seq}"),
        feedback_seq: seq,
        agent_subject: accepted_context.agent_subject.clone(),
        agent_session_id: accepted_context.agent_session_id.clone(),
        agent_turn_id: accepted_context.agent_turn_id.clone(),
        decision_request_id: accepted_context.decision_request_id.clone(),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "pending".to_string(),
        request_digest: accepted_context.request_digest.clone(),
        reject_reason: None,
        provenance: "runtime_authoritative".to_string(),
    };

    assert_eq!(
        state
            .record_continuous_feedback(feedback(2))
            .expect_err("out-of-order feedback must be held"),
        "feedback_sequence_gap"
    );
    state
        .record_continuous_feedback(feedback(1))
        .expect("first feedback drains the held successor");
    let partitions = state.recent_feedback.lock().expect("feedback lock");
    let partition = &partitions[&("agent-1".to_string(), "session-gap".to_string())];
    assert_eq!(partition.next_seq, 2);
    assert!(partition.held.is_empty());
    assert_eq!(partition.recent.len(), 2);
    drop(partitions);
    for seq in 4..=11 {
        assert_eq!(
            state
                .record_continuous_feedback(feedback(seq))
                .expect_err("bounded gap remains diagnostic"),
            "feedback_sequence_gap"
        );
    }
    let mut held_collision = feedback(4);
    held_collision.feedback_id = "feedback-gap-collision".to_string();
    held_collision.reject_reason = Some("different-envelope".to_string());
    assert_eq!(
        state
            .record_continuous_feedback(held_collision)
            .expect_err("held sequence collision must fail closed"),
        "feedback_identity_collision"
    );
    assert_eq!(
        state
            .record_continuous_feedback(feedback(13))
            .expect_err("bounded gap overflow must remain diagnostic"),
        "feedback_sequence_overflow"
    );
}

fn feedback_for_context(
    context: &ContinuousAgentRequestContextV1,
    feedback_id: &str,
    feedback_seq: u64,
) -> FeedbackEnvelopeV1 {
    FeedbackEnvelopeV1 {
        feedback_id: feedback_id.to_string(),
        feedback_seq,
        agent_subject: context.agent_subject.clone(),
        agent_session_id: context.agent_session_id.clone(),
        agent_turn_id: context.agent_turn_id.clone(),
        decision_request_id: context.decision_request_id.clone(),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "pending".to_string(),
        request_digest: context.request_digest.clone(),
        reject_reason: None,
        provenance: "runtime_authoritative".to_string(),
    }
}

fn recording_invoker() -> RecordingInvoker {
    RecordingInvoker {
        response: Ok(AgentInvocationOutput {
            prompt: "prompt".to_string(),
            text: r#"{"decision":"wait"}"#.to_string(),
            provider_version: Some("provider/test".to_string()),
            duration_ms: Some(1),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            route_note: None,
            upstream_trace: None,
        }),
        invocations: Arc::new(Mutex::new(Vec::new())),
    }
}

#[test]
fn continuous_feedback_requires_an_accepted_request_response_lineage() {
    let state = ProviderState::new(CliOptions {
        mode: ProviderMode::Mock,
        ..CliOptions::default()
    })
    .expect("build mock provider state");
    let context = continuous_context("session-correlation", 1);

    assert_eq!(
        state
            .record_continuous_feedback(feedback_for_context(&context, "forged-before-request", 1,))
            .expect_err("unknown feedback must not enter the prompt"),
        "unknown_feedback"
    );

    state
        .handle_continuous_decision(context.clone(), None, &recording_invoker())
        .expect("accepted response registers feedback lineage");

    let mut mismatched = feedback_for_context(&context, "mismatched-turn", 1);
    mismatched.agent_turn_id = "forged-turn".to_string();
    assert_eq!(
        state
            .record_continuous_feedback(mismatched)
            .expect_err("feedback with mismatched turn must fail closed"),
        "feedback_correlation_mismatch"
    );

    let mut forged_digest = feedback_for_context(&context, "forged-digest", 1);
    forged_digest.request_digest = Digest32::from(format!("blake3:{}", "f".repeat(64)));
    assert_eq!(
        state
            .record_continuous_feedback(forged_digest)
            .expect_err("feedback with a forged request digest must fail closed"),
        "feedback_digest_mismatch"
    );

    state
        .record_continuous_feedback(feedback_for_context(&context, "accepted-feedback", 1))
        .expect("correlated feedback is accepted");
}

#[test]
fn continuous_response_lineage_rejects_a_forged_duplicate_response() {
    let state = ProviderState::new(CliOptions {
        mode: ProviderMode::Mock,
        ..CliOptions::default()
    })
    .expect("build mock provider state");
    let context = continuous_context("session-response", 1);
    let mut forged_response = state
        .handle_continuous_decision(context.clone(), None, &recording_invoker())
        .expect("register response lineage");
    forged_response.response_digest = Digest32::from(format!("blake3:{}", "f".repeat(64)));
    assert_eq!(
        super::super::feedback_state::remember_accepted_response(
            &state,
            &context,
            &forged_response
        )
        .expect_err("a changed response must not replace accepted lineage"),
        "response_identity_collision"
    );
}

#[test]
fn continuous_feedback_cursor_and_lineage_survive_provider_restart() {
    let path = std::env::temp_dir().join(format!(
        "oasis7-provider-feedback-restart-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    let context = continuous_context("session-restart", 1);
    let state =
        ProviderState::new_with_feedback_state_path(CliOptions::default(), Some(path.clone()))
            .expect("first bridge state");
    state
        .handle_continuous_decision(context.clone(), None, &recording_invoker())
        .expect("register first response");
    state
        .record_continuous_feedback(feedback_for_context(&context, "restart-feedback-1", 1))
        .expect("persist first feedback");
    drop(state);

    let restarted =
        ProviderState::new_with_feedback_state_path(CliOptions::default(), Some(path.clone()))
            .expect("restarted bridge state");
    restarted
        .record_continuous_feedback(feedback_for_context(&context, "restart-feedback-2", 2))
        .expect("persisted cursor accepts the next sequence after restart");

    let _ = std::fs::remove_file(path);
}

fn continuous_context(
    agent_session_id: &str,
    transport_attempt: u64,
) -> ContinuousAgentRequestContextV1 {
    let mut base_decision_request = sample_request();
    let (catalog, invocation) = production_capabilities(agent_session_id);
    base_decision_request.capability_catalog = Some(catalog.clone());
    base_decision_request.capability_invocation_context = Some(invocation.clone());
    let mut context = ContinuousAgentRequestContextV1 {
        base_decision_request,
        context_discriminator: oasis7::simulator::CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR
            .to_string(),
        context_version: oasis7::simulator::CONTINUOUS_AGENT_CONTEXT_VERSION,
        protocol_version: "continuous-agent-v1".to_string(),
        agent_session_id: agent_session_id.to_string(),
        agent_turn_id: "turn-bridge".to_string(),
        decision_request_id: format!("request-{agent_session_id}"),
        retry_seq: 1,
        transport_attempt,
        agent_subject: "agent-1".to_string(),
        runtime_binding: RuntimeBindingV1 {
            world_id: "world-bridge".to_string(),
            branch_id: "main".to_string(),
            finality_epoch: 1,
            finality_block_hash: Some(Digest32::from(format!("blake3:{}", "a".repeat(64)))),
            finality_status: "verified".to_string(),
            base_tick: 7,
            base_world_hash: Digest32::from(format!("blake3:{}", "b".repeat(64))),
            reorg_epoch: 0,
            runtime_manifest_hash: Digest32::from(format!("blake3:{}", "c".repeat(64))),
        },
        observation_digest: Digest32::from(format!("blake3:{}", "d".repeat(64))),
        capability_catalog_digest: oasis7::simulator::h_v1(
            COGNITION_CAPABILITY_CATALOG_DOMAIN,
            &catalog,
        ),
        capability_invocation_context_digest: oasis7::simulator::h_v1(
            COGNITION_CAPABILITY_INVOCATION_CONTEXT_DOMAIN,
            &invocation,
        ),
        memory_snapshot_digest: Digest32::from(format!("blake3:{}", "1".repeat(64))),
        goal_snapshot_digest: Digest32::from(format!("blake3:{}", "2".repeat(64))),
        continuation_digest: Digest32::from(format!("blake3:{}", "3".repeat(64))),
        adapter_protocol_version: "loopback-http-v1".to_string(),
        budget_contract: oasis7::simulator::BudgetContractV1 {
            max_latency_ms: 7_000,
            max_repair_attempts: 1,
        },
        request_digest: Digest32::default(),
    };
    context.request_digest = context.request_digest();
    context
}

fn production_capabilities(
    session_id: &str,
) -> (CapabilityCatalogSnapshot, CapabilityInvocationContext) {
    let subject = serde_json::json!({
        "kind": "agent",
        "agent_id": "agent-1",
        "owner_binding": "owner-1",
        "generation": 1
    });
    let presenter = serde_json::json!({
        "presenter_id": "provider-1",
        "presenter_kind": "provider",
        "session_id": session_id
    });
    let audience = serde_json::json!({
        "world_id": "world-bridge",
        "branch_id": "main",
        "finality_epoch": 1,
        "target_kind": "world",
        "target_id": null
    });
    let catalog: CapabilityCatalogSnapshot = serde_json::from_value(serde_json::json!({
        "snapshot_id": format!("catalog.{session_id}"),
        "world_id": "world-bridge",
        "world_head": 7,
        "branch_id": "main",
        "finality_epoch": 1,
        "logical_tick": 7,
        "module_registry_hash": "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "policy_hash": "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "revocation_epoch": 0,
        "subject": subject,
        "presenter": presenter,
        "audience": audience,
        "entries": [],
        "valid_until_tick": 100
    }))
    .expect("decode production capability catalog");
    let invocation = CapabilityInvocationContext {
        grant_id: format!("grant.{session_id}"),
        subject: catalog.subject.clone(),
        presenter: catalog.presenter.clone(),
        audience: catalog.audience.clone(),
        catalog_snapshot_id: catalog.snapshot_id.clone(),
        module_id: String::new(),
        module_version: String::new(),
        response_nonce: format!("nonce.{session_id}"),
    };
    (catalog, invocation)
}

#[test]
fn continuous_bridge_uses_outer_identity_for_session_and_invocation_key() {
    let state = ProviderState::new(CliOptions::default()).expect("build provider state");
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let invoker = RecordingInvoker {
        response: Ok(AgentInvocationOutput {
            prompt: "prompt".to_string(),
            text: r#"{"decision":"wait"}"#.to_string(),
            provider_version: Some("provider/test".to_string()),
            duration_ms: Some(1),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            route_note: None,
            upstream_trace: None,
        }),
        invocations: Arc::clone(&invocations),
    };
    let first = continuous_context("session-one", 1);
    let retry = continuous_context("session-one", 2);
    let other_outer = continuous_context("session-two", 1);
    let first_key = first.provider_invocation_key().to_string();
    let other_key = other_outer.provider_invocation_key().to_string();
    state
        .handle_continuous_decision(first.clone(), None, &invoker)
        .expect("first continuous invocation");
    state
        .handle_continuous_decision(retry, None, &invoker)
        .expect("transport retry continuous invocation");
    state
        .handle_continuous_decision(other_outer.clone(), None, &invoker)
        .expect("different outer identity invocation");

    let invocations = invocations.lock().expect("recording invocations lock");
    assert_eq!(invocations.len(), 3);
    assert_eq!(
        invocations[0].provider_invocation_key.as_deref(),
        Some(first_key.as_str())
    );
    assert_eq!(
        invocations[0].agent_session_id.as_deref(),
        Some("session-one")
    );
    assert_eq!(
        invocations[0].idempotency_key,
        invocations[1].idempotency_key
    );
    assert_eq!(invocations[0].session_key, invocations[1].session_key);
    assert_ne!(
        invocations[0].idempotency_key,
        invocations[2].idempotency_key
    );
    assert_ne!(invocations[0].session_key, invocations[2].session_key);
    assert_eq!(
        invocations[2].provider_invocation_key.as_deref(),
        Some(other_key.as_str())
    );
}
