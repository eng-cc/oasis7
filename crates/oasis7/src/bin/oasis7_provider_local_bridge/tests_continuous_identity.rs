use super::*;

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
    let feedback = |seq: u64| FeedbackEnvelopeV1 {
        feedback_id: format!("feedback-gap-{seq}"),
        feedback_seq: seq,
        agent_subject: "agent-gap".to_string(),
        agent_session_id: "session-gap".to_string(),
        agent_turn_id: "turn-gap".to_string(),
        decision_request_id: "request-gap".to_string(),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "pending".to_string(),
        request_digest: Digest32::from(
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
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
    let partition = &partitions[&("agent-gap".to_string(), "session-gap".to_string())];
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

fn continuous_context(
    agent_session_id: &str,
    transport_attempt: u64,
) -> ContinuousAgentRequestContextV1 {
    let mut context = ContinuousAgentRequestContextV1 {
        base_decision_request: sample_request(),
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
        capability_catalog_digest: Digest32::from(format!("blake3:{}", "e".repeat(64))),
        capability_invocation_context_digest: Digest32::from(format!("blake3:{}", "f".repeat(64))),
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
