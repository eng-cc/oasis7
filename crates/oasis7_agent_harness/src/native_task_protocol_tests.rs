use super::*;

// FakeHost is the trusted boundary in these runner tests. SDK signed-history
// matching is tested separately; an arbitrary provider JSON is never promoted.
fn stale_feedback() -> oasis7_game_client::AuthorizedFeedbackRecord {
    oasis7_game_client::AuthorizedFeedbackRecord {
        feedback: oasis7_agent_api::FeedbackEnvelopeV1 {
            feedback_id: "runtime-feedback-one".into(),
            feedback_seq: 1,
            agent_subject: "agent".into(),
            agent_session_id: "session".into(),
            agent_turn_id: "turn".into(),
            decision_request_id: "decision".into(),
            candidate_action_id: Some(7),
            runtime_receipt_id: None,
            status: "rejected".into(),
            request_digest: oasis7_agent_api::Digest32::from(format!("blake3:{}", "a".repeat(64))),
            reject_reason: Some("stale_base".into()),
            provenance: "runtime_authoritative".into(),
        },
        original_envelope_digest: format!("blake3:{}", "b".repeat(64)),
        delivery_state: "pending".into(),
        receipt_lineage: None,
    }
}
#[test]
fn matched_stale_original_uses_new_slot_new_model_decision_and_preserves_budget_on_restart() {
    let host = Arc::new(FakeHost::new());
    let prompts = Arc::new(Mutex::new(Vec::new()));
    {
        let mut s = host.state.lock().unwrap();
        s.effects.push_back(NativeTaskRecovery::Rejected);
        s.known_feedback
            .insert("operator-run:0".into(), stale_feedback());
    }
    let p = FakeProvider {
        batches: vec![
            vec![call("move", json!({"to":[1,2,3]}))],
            vec![call("move", json!({"to":[4,5,6]}))],
        ],
        prompts: prompts.clone(),
    };
    let c = config();
    let report = NativeTaskRunner::new(host.clone(), Box::new(p), c.clone())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::Completed);
    assert_eq!(report.turns_completed, 1);
    assert_eq!(report.model_calls_spent, 2);
    let prompts = prompts.lock().unwrap();
    let second: Value = serde_json::from_str(&prompts[1]).unwrap();
    assert_eq!(second["operation"], "operator-run:1");
    let prior = &second["prior_authorized_rejections"][0];
    assert_eq!(prior["original_operation"], "operator-run:0");
    assert_eq!(
        prior["authorized_feedback"]["original_envelope_digest"],
        stale_feedback().original_envelope_digest
    );
    assert_eq!(
        prior["authorized_feedback"]["feedback"]["reject_reason"],
        "stale_base"
    );
    drop(prompts);
    let count = host.events().len();
    let restart = NativeTaskRunner::new(host.clone(), provider(vec![]), c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(restart.turns_completed, 1);
    assert_eq!(restart.model_calls_spent, 2);
    assert_eq!(restart.stop_reason, NativeTaskStopReason::Completed);
    assert!(
        host.events()[count..]
            .iter()
            .all(|event| event.starts_with("recover:"))
    );
}
#[test]
fn stale_rejection_with_exhausted_budget_never_prepares_another_context() {
    let host = Arc::new(FakeHost::new());
    {
        let mut s = host.state.lock().unwrap();
        s.spent = 2;
        s.recover
            .insert("operator-run:0".into(), NativeTaskRecovery::Rejected);
        s.known_feedback
            .insert("operator-run:0".into(), stale_feedback());
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::BudgetExhausted);
    assert_eq!(report.turns_completed, 0);
    assert!(
        host.events()
            .iter()
            .all(|event| event.starts_with("recover:"))
    );
}
struct ProtocolProvider {
    turns: Vec<ModelProviderTurn>,
    pending: Vec<crate::ModelProviderToolCall>,
    results: Arc<Mutex<Vec<crate::ModelProviderToolResult>>>,
    allowed: Arc<AtomicBool>,
}
impl ModelProvider for ProtocolProvider {
    fn provider_id(&self) -> &str {
        "protocol-fixture"
    }
    fn complete(
        &mut self,
        r: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        Ok(response(r, self.turns.clone()))
    }
    fn pending_tool_calls(&self) -> Vec<crate::ModelProviderToolCall> {
        self.pending.clone()
    }
    fn record_tool_results(
        &mut self,
        results: &[crate::ModelProviderToolResult],
    ) -> Result<(), ModelProviderError> {
        *self.results.lock().unwrap() = results.to_vec();
        Ok(())
    }
    fn check_execution(&self) -> Result<(), ModelProviderError> {
        if self.allowed.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(ModelProviderError::new("lease_stopped", "stopped", false))
        }
    }
}
fn protocol_provider() -> ProtocolProvider {
    ProtocolProvider {
        turns: vec![
            call("observe", json!({})),
            call("move", json!({"to":[1,2,3]})),
        ],
        pending: vec![
            crate::ModelProviderToolCall {
                call_id: "supplier-read".into(),
                name: "observe".into(),
                arguments: json!({}),
            },
            crate::ModelProviderToolCall {
                call_id: "supplier-effect".into(),
                name: "move".into(),
                arguments: json!({"to":[1,2,3]}),
            },
        ],
        results: Arc::new(Mutex::new(Vec::new())),
        allowed: Arc::new(AtomicBool::new(true)),
    }
}
#[test]
fn supplier_call_ids_receive_complete_ordered_results_not_world_operation_ids() {
    let host = Arc::new(FakeHost::new());
    let p = protocol_provider();
    let results = p.results.clone();
    let report = NativeTaskRunner::new(host, Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::Completed);
    let results = results.lock().unwrap();
    assert_eq!(
        results
            .iter()
            .map(|r| r.call_id.as_str())
            .collect::<Vec<_>>(),
        ["supplier-read", "supplier-effect"]
    );
    assert!(
        results
            .iter()
            .all(|r| r.content["observed_result"] == "fresh observation sentinel")
    );
}
#[test]
fn supplier_call_id_mismatch_duplicate_and_reordering_fail_before_first_tool() {
    for defect in [
        "argument",
        "name",
        "duplicate",
        "reordered",
        "missing",
        "empty",
    ] {
        let host = Arc::new(FakeHost::new());
        let mut p = protocol_provider();
        match defect {
            "argument" => p.pending[1].arguments = json!({"to":[9,9,9]}),
            "name" => p.pending[1].name = "observe".into(),
            "duplicate" => p.pending[1].call_id = p.pending[0].call_id.clone(),
            "reordered" => p.pending.reverse(),
            "missing" => {
                p.pending.pop();
            }
            "empty" => p.pending[0].call_id.clear(),
            _ => unreachable!(),
        }
        let report = NativeTaskRunner::new(host.clone(), Box::new(p), config())
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(
            report.stop_reason,
            NativeTaskStopReason::InvalidModelOutput,
            "{defect}"
        );
        assert!(!host.events().iter().any(|e| e.starts_with("tool:")));
    }
}
#[test]
fn execution_lease_stop_between_read_and_effect_prevents_world_action() {
    let p = protocol_provider();
    let results = p.results.clone();
    let mut h = FakeHost::new();
    h.execution_allowed = Some(p.allowed.clone());
    let host = Arc::new(h);
    let report = NativeTaskRunner::new(host.clone(), Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::ControlChanged);
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.ends_with(":read"))
            .count(),
        1
    );
    assert!(!host.events().iter().any(|e| e.ends_with(":effect")));
    let results = results.lock().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].call_id, "supplier-read");
    assert_eq!(results[1].call_id, "supplier-effect");
    assert_eq!(results[1].content["status"], "not_executed");
}

struct ClaimedFeedbackProvider;
impl ModelProvider for ClaimedFeedbackProvider {
    fn provider_id(&self) -> &str {
        "untrusted-feedback-claim"
    }
    fn complete(
        &mut self,
        request: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        let mut r = response(request, vec![call("move", json!({"to":[1,2,3]}))]);
        r.output = serde_json::to_string(&stale_feedback()).unwrap();
        Ok(r)
    }
}
#[test]
fn provider_claimed_stale_feedback_cannot_authorize_replanning() {
    let host = Arc::new(FakeHost::new());
    host.state
        .lock()
        .unwrap()
        .effects
        .push_back(NativeTaskRecovery::Rejected);
    let report = NativeTaskRunner::new(host.clone(), Box::new(ClaimedFeedbackProvider), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::RejectedOriginal);
    assert_eq!(report.model_calls_spent, 1);
    assert_eq!(report.turns_completed, 0);
    assert!(!host.events().iter().any(|e| e == "prepare:operator-run:1"));
}
#[test]
fn occupied_rejection_slots_report_limit_instead_of_completed() {
    let host = Arc::new(FakeHost::new());
    {
        let mut state = host.state.lock().unwrap();
        for i in 0..3 {
            let operation = config().operation_id(i);
            state
                .recover
                .insert(operation.clone(), NativeTaskRecovery::Rejected);
            state.known_feedback.insert(operation, stale_feedback());
        }
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::BudgetExhausted);
    assert_eq!(report.turns_completed, 0);
    assert_eq!(report.model_calls_spent, 0);
    assert!(host.events().iter().all(|e| e.starts_with("recover:")));
}
#[test]
fn successful_effect_goal_stops_at_four_even_with_free_slots_and_budget() {
    let host = Arc::new(FakeHost::new());
    let mut c = config();
    c.max_turns = 4;
    c.max_model_calls = 5;
    let p = provider(
        (0..4)
            .map(|i| vec![call("move", json!({"to":[i,2,3]}))])
            .collect(),
    );
    let report = NativeTaskRunner::new(host.clone(), p, c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::Completed);
    assert_eq!(report.turns_completed, 4);
    assert_eq!(report.model_calls_spent, 4);
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.ends_with(":effect"))
            .count(),
        4
    );
    assert!(!host.events().iter().any(|e| e == "prepare:operator-run:4"));
}

#[test]
fn stopped_model_lease_never_prepares_a_new_private_context() {
    let host = Arc::new(FakeHost::new());
    let p = protocol_provider();
    p.allowed.store(false, Ordering::SeqCst);
    let report = NativeTaskRunner::new(host.clone(), Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::ControlChanged);
    assert_eq!(report.model_calls_spent, 0);
    assert!(
        host.events()
            .iter()
            .all(|event| event.starts_with("recover:"))
    );
}
