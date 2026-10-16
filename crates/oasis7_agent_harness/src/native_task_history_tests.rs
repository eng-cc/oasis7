use super::*;

struct HistoryProvider {
    batches: Vec<Vec<ModelProviderTurn>>,
    prompts: Arc<Mutex<Vec<String>>>,
    results: Arc<Mutex<Vec<crate::ModelProviderToolResult>>>,
    pending: Vec<crate::ModelProviderToolCall>,
    structured: bool,
    reject_preflight: bool,
    allowed: Option<Arc<AtomicBool>>,
    omit_ids: bool,
}
impl ModelProvider for HistoryProvider {
    fn provider_id(&self) -> &str {
        "bounded-history-fixture"
    }
    fn history_mode(&self) -> crate::ModelProviderHistory {
        if self.structured {
            crate::ModelProviderHistory::ProviderConversation
        } else {
            crate::ModelProviderHistory::CallerSnapshot
        }
    }
    fn check_execution(&self) -> Result<(), ModelProviderError> {
        if self
            .allowed
            .as_ref()
            .is_some_and(|a| !a.load(Ordering::SeqCst))
        {
            Err(ModelProviderError::new("stopped", "stopped", false))
        } else {
            Ok(())
        }
    }
    fn preflight(&self, request: &ModelProviderRequest) -> Result<(), ModelProviderError> {
        request.validate()?;
        if let Some(allowed) = &self.allowed {
            allowed.store(false, Ordering::SeqCst);
        }
        if self.reject_preflight {
            Err(ModelProviderError::new(
                "input_limit",
                "complete input cannot fit",
                false,
            ))
        } else {
            Ok(())
        }
    }
    fn complete(
        &mut self,
        r: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        self.prompts.lock().unwrap().push(r.user_prompt.clone());
        let turns = self.batches.remove(0);
        self.pending = if self.structured && !self.omit_ids {
            turns
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let ModelProviderTurn::ToolCall { name, arguments } = t else {
                        panic!("typed tool")
                    };
                    crate::ModelProviderToolCall {
                        call_id: format!("supplier-{}-{i}", self.prompts.lock().unwrap().len()),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    }
                })
                .collect()
        } else {
            vec![]
        };
        Ok(response(r, turns))
    }
    fn pending_tool_calls(&self) -> Vec<crate::ModelProviderToolCall> {
        self.pending.clone()
    }
    fn record_tool_results(
        &mut self,
        r: &[crate::ModelProviderToolResult],
    ) -> Result<(), ModelProviderError> {
        assert_eq!(self.pending.len(), r.len());
        for (call, result) in self.pending.iter().zip(r) {
            assert_eq!(call.call_id, result.call_id);
        }
        self.results.lock().unwrap().extend_from_slice(r);
        self.pending.clear();
        Ok(())
    }
}
fn history_provider(batches: Vec<Vec<ModelProviderTurn>>, structured: bool) -> HistoryProvider {
    HistoryProvider {
        batches,
        structured,
        prompts: Arc::new(Mutex::new(vec![])),
        results: Arc::new(Mutex::new(vec![])),
        pending: vec![],
        reject_preflight: false,
        allowed: None,
        omit_ids: false,
    }
}
#[test]
fn provider_owned_history_sends_context_once_and_tools_only_by_supplier_id() {
    let host = Arc::new(FakeHost::new());
    let p = history_provider(
        vec![
            vec![call("observe", json!({}))],
            vec![call("industry", json!({}))],
            vec![call("move", json!({"to":[1,2,3]}))],
        ],
        true,
    );
    let prompts = p.prompts.clone();
    let results = p.results.clone();
    let mut c = config();
    c.max_model_calls = 3;
    let r = NativeTaskRunner::new(host, Box::new(p), c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(r.stop_reason, NativeTaskStopReason::Completed);
    let prompts = prompts.lock().unwrap();
    assert!(prompts[0].contains(&"complete private goal ".repeat(80)));
    for text in &prompts[1..] {
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(v["operation"], "operator-run:0");
        assert!(v.get("authorized_context").is_none());
        assert!(!text.contains("fresh observation sentinel"));
        assert!(text.len() < 512);
    }
    assert_eq!(results.lock().unwrap().len(), 3);
}
#[test]
fn snapshot_provider_still_receives_complete_context_and_accumulated_results() {
    let host = Arc::new(FakeHost::new());
    let p = history_provider(
        vec![
            vec![call("observe", json!({}))],
            vec![call("move", json!({"to":[1,2,3]}))],
        ],
        false,
    );
    let prompts = p.prompts.clone();
    let r = NativeTaskRunner::new(host, Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(r.stop_reason, NativeTaskStopReason::Completed);
    let prompts = prompts.lock().unwrap();
    for p in prompts.iter() {
        assert!(p.contains(&"complete private goal ".repeat(80)));
    }
    assert!(prompts[1].contains("fresh observation sentinel"));
}
#[test]
fn new_context_and_restarted_provider_receive_full_authorized_input() {
    let host = Arc::new(FakeHost::new());
    let p = history_provider(
        vec![
            vec![call("move", json!({"to":[1,2,3]}))],
            vec![call("observe", json!({}))],
            vec![call("move", json!({"to":[4,5,6]}))],
        ],
        true,
    );
    let prompts = p.prompts.clone();
    let mut c = config();
    c.max_turns = 2;
    c.max_model_calls = 3;
    let r = NativeTaskRunner::new(host, Box::new(p), c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(r.turns_completed, 2);
    let prompts = prompts.lock().unwrap();
    for i in [0, 1] {
        let v: Value = serde_json::from_str(&prompts[i]).unwrap();
        assert!(v.get("authorized_context").is_some());
        assert_eq!(v["operation"], format!("operator-run:{i}"));
    }
    assert!(
        serde_json::from_str::<Value>(&prompts[2])
            .unwrap()
            .get("authorized_context")
            .is_none()
    );
    // A fresh provider over an existing Ready slot cannot rely on lost in-memory history.
    let host = Arc::new(FakeHost::new());
    host.state
        .lock()
        .unwrap()
        .recover
        .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
    let p = history_provider(vec![vec![call("move", json!({"to":[1,2,3]}))]], true);
    let prompts = p.prompts.clone();
    assert_eq!(
        NativeTaskRunner::new(host, Box::new(p), config())
            .unwrap()
            .run()
            .unwrap()
            .stop_reason,
        NativeTaskStopReason::Completed
    );
    assert!(prompts.lock().unwrap()[0].contains(&"complete private goal ".repeat(80)));
}
#[test]
fn input_limit_preflight_preserves_original_and_spends_no_physical_call_budget() {
    let host = Arc::new(FakeHost::new());
    host.state
        .lock()
        .unwrap()
        .recover
        .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
    for _ in 0..2 {
        let mut p = history_provider(vec![], true);
        p.reject_preflight = true;
        let r = NativeTaskRunner::new(host.clone(), Box::new(p), config())
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(r.stop_reason, NativeTaskStopReason::InputLimit);
        assert_eq!(r.model_calls_spent, 0);
    }
    assert!(!host.events().iter().any(|e| e.starts_with("reserve:")
        || e.starts_with("tool:")
        || e.starts_with("settle:")
        || e == "prepare:operator-run:1"));
    assert_eq!(
        host.state.lock().unwrap().recover["operator-run:0"],
        NativeTaskRecovery::Ready
    );
}

#[test]
fn stop_during_preflight_cannot_reserve_or_execute() {
    let host = Arc::new(FakeHost::new());
    let mut p = history_provider(vec![], true);
    p.allowed = Some(Arc::new(AtomicBool::new(true)));
    let r = NativeTaskRunner::new(host.clone(), Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(r.stop_reason, NativeTaskStopReason::ControlChanged);
    assert_eq!(r.model_calls_spent, 0);
    assert!(
        !host
            .events()
            .iter()
            .any(|e| e.starts_with("reserve:") || e.starts_with("tool:"))
    );
}
#[test]
fn structured_history_without_supplier_ids_fails_before_tool_execution() {
    let host = Arc::new(FakeHost::new());
    let mut p = history_provider(vec![vec![call("move", json!({"to":[1,2,3]}))]], true);
    p.omit_ids = true;
    let r = NativeTaskRunner::new(host.clone(), Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(r.stop_reason, NativeTaskStopReason::InvalidModelOutput);
    assert_eq!(r.model_calls_spent, 1);
    assert!(!host.events().iter().any(|e| e.starts_with("tool:")));
}
