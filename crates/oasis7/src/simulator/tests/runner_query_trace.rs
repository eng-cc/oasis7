use super::*;

struct TracedQuoteAgent {
    id: String,
    trace_emitted: bool,
    query_results: Vec<AgentQueryResult>,
}

impl TracedQuoteAgent {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            trace_emitted: false,
            query_results: Vec::new(),
        }
    }

    fn query(&self) -> AgentQuery {
        AgentQuery::EvaluateMicroDepotQuote(MicroDepotQuoteRequest {
            agent_id: self.id.clone(),
            facility_id: "depot-measured".to_string(),
            target_id: "loc-measured".to_string(),
            action_kind: MicroDepotActionKind::Repair,
            base_cost_class: MicroDepotPressureClass::Medium,
            base_risk_class: MicroDepotPressureClass::Low,
            blocker_type: Some("supply_missing".to_string()),
        })
    }
}

impl AgentBehavior for TracedQuoteAgent {
    fn agent_id(&self) -> &str {
        &self.id
    }

    fn decide(&mut self, _observation: &Observation) -> AgentDecision {
        AgentDecision::Query(self.query())
    }

    fn on_query_result(&mut self, result: &AgentQueryResult) {
        self.query_results.push(result.clone());
    }

    fn take_decision_trace(&mut self) -> Option<AgentDecisionTrace> {
        if self.trace_emitted {
            return None;
        }
        self.trace_emitted = true;
        Some(AgentDecisionTrace {
            agent_id: self.id.clone(),
            time: 0,
            decision: AgentDecision::Query(self.query()),
            llm_input: Some("evaluate micro-depot quote".to_string()),
            llm_output: Some("query preview".to_string()),
            llm_error: None,
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: vec![LlmEffectIntentTrace {
                intent_id: "quote-module-call".to_string(),
                kind: "llm.prompt.module_call".to_string(),
                params: serde_json::json!({ "module": "regional.micro_depot" }),
                cap_ref: "llm.prompt.module_access".to_string(),
                origin: "llm_agent".to_string(),
            }],
            llm_effect_receipts: vec![LlmEffectReceiptTrace {
                intent_id: "quote-module-call".to_string(),
                status: "ok".to_string(),
                payload: serde_json::json!({ "preview": "available" }),
                cost_cents: None,
            }],
            llm_step_trace: vec![],
            llm_prompt_section_trace: vec![],
            llm_chat_messages: vec![],
        })
    }
}

fn traced_query_kernel() -> WorldKernel {
    let mut kernel =
        super::micro_depot_measured_supply::installed_micro_depot_with_measured_supply(5, 4, 2);
    kernel
        .observe("agent-measured")
        .expect("warm quote-agent observation");
    kernel
}

fn measured_service_action() -> Action {
    Action::ServiceMicroDepotRepair {
        agent_id: "agent-measured".to_string(),
        facility_id: "depot-measured".to_string(),
        target_id: "loc-measured".to_string(),
        base_cost_class: MicroDepotPressureClass::Medium,
        base_risk_class: MicroDepotPressureClass::Low,
        blocker_type: Some("supply_missing".to_string()),
    }
}

fn assert_query_is_checkpoint_pure(
    kernel: &WorldKernel,
    snapshot_before: &WorldSnapshot,
    journal_before: &WorldJournal,
) {
    assert_eq!(kernel.pending_actions(), 0);
    assert_eq!(kernel.time(), snapshot_before.time);
    assert_eq!(kernel.journal_snapshot(), *journal_before);
    assert_eq!(
        kernel.snapshot(),
        *snapshot_before,
        "query trace must not perturb canonical or replay checkpoint state"
    );
}

#[test]
fn traced_query_tick_preserves_canonical_history_and_next_service_identity() {
    let mut kernel = traced_query_kernel();
    let mut control = traced_query_kernel();
    let snapshot_before = kernel.snapshot();
    let journal_before = kernel.journal_snapshot();
    let mut runner: AgentRunner<TracedQuoteAgent> = AgentRunner::new();
    runner.register(TracedQuoteAgent::new("agent-measured"));

    let query_tick = runner.tick(&mut kernel).expect("query tick");

    assert!(matches!(query_tick.decision, AgentDecision::Query(_)));
    assert!(query_tick.action_result.is_none());
    assert!(query_tick.query_result.is_some());
    assert_eq!(
        runner
            .get("agent-measured")
            .expect("registered")
            .behavior
            .query_results
            .len(),
        1
    );
    assert_query_is_checkpoint_pure(&kernel, &snapshot_before, &journal_before);

    let service_id_after_query =
        kernel.submit_action_from_agent("agent-measured", measured_service_action());
    let control_service_id =
        control.submit_action_from_agent("agent-measured", measured_service_action());
    assert_eq!(service_id_after_query, control_service_id);
    assert_eq!(kernel.step(), control.step());
}

#[test]
fn traced_query_decide_only_preserves_canonical_history_and_feedback() {
    let mut kernel = traced_query_kernel();
    let snapshot_before = kernel.snapshot();
    let journal_before = kernel.journal_snapshot();
    let mut runner: AgentRunner<TracedQuoteAgent> = AgentRunner::new();
    runner.register(TracedQuoteAgent::new("agent-measured"));

    let query_tick = runner
        .tick_decide_only(&mut kernel)
        .expect("query decision");

    assert!(matches!(query_tick.decision, AgentDecision::Query(_)));
    assert!(query_tick.action_result.is_none());
    assert!(query_tick.query_result.is_some());
    assert_eq!(
        runner
            .get("agent-measured")
            .expect("registered")
            .behavior
            .query_results
            .len(),
        1
    );
    assert_query_is_checkpoint_pure(&kernel, &snapshot_before, &journal_before);
}

#[test]
fn traced_query_batch_excludes_trace_from_durable_intents_and_history() {
    let mut kernel = traced_query_kernel();
    let mut control = traced_query_kernel();
    let snapshot_before = kernel.snapshot();
    let journal_before = kernel.journal_snapshot();
    let mut runner: AgentRunner<TracedQuoteAgent> = AgentRunner::new();
    runner.register(TracedQuoteAgent::new("agent-measured"));

    let results = runner.tick_collect_intents_and_commit(&mut kernel, 1);

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].decision, AgentDecision::Query(_)));
    assert!(results[0].action_result.is_none());
    assert!(results[0].query_result.is_some());
    assert_eq!(
        runner
            .get("agent-measured")
            .expect("registered")
            .behavior
            .query_results
            .len(),
        1
    );
    assert_query_is_checkpoint_pure(&kernel, &snapshot_before, &journal_before);

    let service_id_after_batch =
        kernel.submit_action_from_agent("agent-measured", measured_service_action());
    let control_service_id =
        control.submit_action_from_agent("agent-measured", measured_service_action());
    assert_eq!(service_id_after_batch, control_service_id);
    assert_eq!(kernel.step(), control.step());
}
