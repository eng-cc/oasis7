use super::*;
use crate::{ModelProviderError, ModelProviderTrace, ModelProviderTurn, ModelProviderUsage};
use oasis7_agent_api::game::{DecimalU64, GameExecutorBindingV1};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Default)]
struct State {
    spent: u32,
    binding: Option<Value>,
    corrupt: bool,
    recover: BTreeMap<String, NativeTaskRecovery>,
    events: Vec<String>,
}
struct FakeHost {
    state: Mutex<State>,
    started: Arc<AtomicBool>,
    drift: Option<&'static str>,
    context_drift: bool,
}
fn fence() -> GameTaskFenceV1 {
    GameTaskFenceV1 {
        world_id: "world".into(),
        genesis_digest: "genesis".into(),
        branch_id: "main".into(),
        reorg_epoch: DecimalU64(0),
        agent_id: "agent".into(),
        owner_binding: "owner".into(),
        owner_generation: DecimalU64(1),
        owner_public_key: "a".repeat(64),
        task_id: "task".into(),
        task_revision: DecimalU64(1),
        task_status: GameTaskStatusV1::Active,
        latest_goal_revision: DecimalU64(0),
        active_goal_revision: DecimalU64(0),
        active_goal_reference: None,
        executor_epoch: DecimalU64(1),
        executor_binding: Some(GameExecutorBindingV1 {
            executor_id: "executor".into(),
            executor_public_key: "b".repeat(64),
            delegation_generation: DecimalU64(1),
            executor_epoch: DecimalU64(1),
        }),
    }
}
impl FakeHost {
    fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            started: Arc::new(AtomicBool::new(false)),
            drift: None,
            context_drift: false,
        }
    }
    fn events(&self) -> Vec<String> {
        self.state.lock().unwrap().events.clone()
    }
}
impl NativeTaskHost for FakeHost {
    fn initialize_run(&self, c: &NativeTaskRunConfig) -> Result<u32, String> {
        let mut s = self.state.lock().unwrap();
        if s.corrupt {
            return Err("corrupt budget".into());
        }
        if s.binding.as_ref().is_some_and(|b| b != &c.binding()) {
            return Err("immutable budget conflict".into());
        }
        s.binding = Some(c.binding());
        Ok(s.spent)
    }
    fn reserve_model_call(&self, c: &NativeTaskRunConfig, operation: &str) -> Result<u32, String> {
        let mut s = self.state.lock().unwrap();
        if s.corrupt || s.spent >= c.max_model_calls {
            return Err("budget unavailable".into());
        }
        s.spent += 1;
        s.events.push(format!("reserve:{operation}"));
        Ok(s.spent)
    }
    fn recover(&self, op: &str) -> Result<NativeTaskRecovery, String> {
        let mut s = self.state.lock().unwrap();
        s.events.push(format!("recover:{op}"));
        Ok(s.recover
            .get(op)
            .copied()
            .unwrap_or(NativeTaskRecovery::Vacant))
    }
    fn prepare(&self, op: &str) -> Result<NativeTaskContext, String> {
        self.state
            .lock()
            .unwrap()
            .events
            .push(format!("prepare:{op}"));
        Ok(NativeTaskContext {
            operation: op.into(),
            fence: {
                let mut f = fence();
                if self.context_drift {
                    f.task_revision = DecimalU64(2);
                }
                f
            },
            valid_until_tick: 20,
            model_context: json!({"goal":{"goal_text":"complete private goal ".repeat(80),"soft_constraints":["preserve all soft constraints"]},"actor_observation":{"ore":10},"capability_catalog":{"native":true}}),
        })
    }
    fn current_control(&self) -> Result<NativeTaskControl, String> {
        let mut control = NativeTaskControl {
            fence: fence(),
            logical_tick: 10,
        };
        if self.started.load(Ordering::SeqCst) {
            match self.drift {
                Some("stop") => control.fence.task_status = GameTaskStatusV1::Suspended,
                Some("goal") => control.fence.latest_goal_revision = DecimalU64(1),
                Some("owner") => control.fence.owner_generation = DecimalU64(2),
                Some("expiry") => control.logical_tick = 21,
                _ => (),
            }
        }
        Ok(control)
    }
    fn invoke(
        &self,
        c: &NativeTaskContext,
        t: &NativeTaskTool,
    ) -> Result<NativeTaskToolResult, String> {
        let mut s = self.state.lock().unwrap();
        s.events.push(format!(
            "tool:{}:{}",
            c.operation,
            if t.side_effect() { "effect" } else { "read" }
        ));
        let effect = t.side_effect().then_some(NativeTaskRecovery::Settled);
        if effect.is_some() {
            s.recover.insert(
                c.operation.clone(),
                if matches!(t, NativeTaskTool::Abort(_)) {
                    NativeTaskRecovery::Aborted
                } else {
                    NativeTaskRecovery::Settled
                },
            );
        }
        Ok(NativeTaskToolResult {
            value: json!({"observed_result":"fresh observation sentinel"}),
            effect,
        })
    }
}
fn config() -> NativeTaskRunConfig {
    NativeTaskRunConfig {
        run_id: "operator-run".into(),
        model: "fake".into(),
        system_prompt: "Preserve operator style without replacing the authorized private goal."
            .into(),
        max_turns: 1,
        max_model_calls: 2,
        max_tool_calls_per_call: 8,
        model_timeout_ms: 1000,
        control_poll_ms: 1,
    }
}
struct FakeProvider {
    batches: Vec<Vec<ModelProviderTurn>>,
    prompts: Arc<Mutex<Vec<String>>>,
}
impl ModelProvider for FakeProvider {
    fn provider_id(&self) -> &str {
        "fake"
    }
    fn complete(
        &mut self,
        r: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        self.prompts.lock().unwrap().push(r.user_prompt.clone());
        Ok(response(r, self.batches.remove(0)))
    }
}
fn response(r: &ModelProviderRequest, turns: Vec<ModelProviderTurn>) -> ModelProviderResponse {
    ModelProviderResponse {
        request_digest: r.request_digest.clone(),
        turns,
        output: String::new(),
        model: None,
        usage: ModelProviderUsage::default(),
        trace: ModelProviderTrace::default(),
    }
}
fn call(name: &str, args: Value) -> ModelProviderTurn {
    ModelProviderTurn::ToolCall {
        name: name.into(),
        arguments: args,
    }
}
fn provider(batches: Vec<Vec<ModelProviderTurn>>) -> Box<dyn ModelProvider> {
    Box::new(FakeProvider {
        batches,
        prompts: Arc::new(Mutex::new(Vec::new())),
    })
}
#[test]
fn full_authorized_input_and_tool_results_reach_next_physical_call_without_shadow_world() {
    let host = Arc::new(FakeHost::new());
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let p = FakeProvider {
        batches: vec![
            vec![call("observe", json!({}))],
            vec![call("move", json!({"to":[1,2,3]}))],
        ],
        prompts: prompts.clone(),
    };
    let report = NativeTaskRunner::new(host.clone(), Box::new(p), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::Completed);
    assert_eq!(report.model_calls_spent, 2);
    assert_eq!(report.turns_completed, 1);
    let prompts = prompts.lock().unwrap();
    assert!(prompts[0].contains(&"complete private goal ".repeat(80)));
    assert!(prompts[0].contains("preserve all soft constraints"));
    assert!(prompts[1].contains("fresh observation sentinel"));
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.ends_with(":effect"))
            .count(),
        1
    );
}
#[test]
fn all_model_turns_validate_before_any_outbound_tool_and_no_wait_fallback() {
    assert!(NativeTaskTool::parse("wait", &json!({"ticks":1})).is_ok());
    for args in [
        json!({"ticks":0}),
        json!({"ticks":17}),
        json!({"ticks":1,"extend_ttl":true}),
    ] {
        assert!(NativeTaskTool::parse("wait", &args).is_err());
    }
    for turns in [
        vec![
            call("move", json!({"to":[1,2,3]})),
            call(
                "build",
                json!({"site":"s","factory":"f","kind":"factory.smelter.mk1"}),
            ),
        ],
        vec![call("observe", json!({})), call("unsupported", json!({}))],
        vec![call("move", json!({"to":[1,2,3],"grant":"injected"}))],
    ] {
        let host = Arc::new(FakeHost::new());
        let report = NativeTaskRunner::new(host.clone(), provider(vec![turns]), config())
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(report.stop_reason, NativeTaskStopReason::InvalidModelOutput);
        assert!(!host.events().iter().any(|e| e.starts_with("tool:")));
    }
}
#[test]
fn reserved_calls_survive_restart_and_limits_cannot_be_topped_up() {
    let host = Arc::new(FakeHost::new());
    let mut c = config();
    c.max_model_calls = 1;
    let first = NativeTaskRunner::new(
        host.clone(),
        provider(vec![vec![call("observe", json!({}))]]),
        c.clone(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert_eq!(first.stop_reason, NativeTaskStopReason::BudgetExhausted);
    let prepares = host
        .events()
        .iter()
        .filter(|e| e.starts_with("prepare:"))
        .count();
    let second = NativeTaskRunner::new(host.clone(), provider(vec![]), c.clone())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(second.model_calls_spent, 1);
    assert_eq!(second.stop_reason, NativeTaskStopReason::BudgetExhausted);
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.starts_with("prepare:"))
            .count(),
        prepares
    );
    c.max_model_calls = 2;
    assert!(
        NativeTaskRunner::new(host, provider(vec![]), c)
            .unwrap()
            .run()
            .is_err()
    );
}
#[test]
fn unknown_rejected_expired_and_aborted_originals_never_prepare_replacement_contexts() {
    for (state, reason) in [
        (
            NativeTaskRecovery::Pending,
            NativeTaskStopReason::PendingOriginal,
        ),
        (
            NativeTaskRecovery::Rejected,
            NativeTaskStopReason::RejectedOriginal,
        ),
        (
            NativeTaskRecovery::Expired,
            NativeTaskStopReason::ExpiredOriginal,
        ),
        (NativeTaskRecovery::Aborted, NativeTaskStopReason::Aborted),
    ] {
        let host = Arc::new(FakeHost::new());
        host.state
            .lock()
            .unwrap()
            .recover
            .insert("operator-run:0".into(), state);
        let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(report.stop_reason, reason);
        assert!(
            !host
                .events()
                .iter()
                .any(|e| e.starts_with("prepare:") || e.starts_with("reserve:"))
        );
    }
}
struct BlockedProvider {
    started: Arc<AtomicBool>,
    release: mpsc::Receiver<()>,
    finished: mpsc::Sender<()>,
}
impl ModelProvider for BlockedProvider {
    fn provider_id(&self) -> &str {
        "blocked"
    }
    fn complete(
        &mut self,
        r: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        self.started.store(true, Ordering::SeqCst);
        self.release.recv_timeout(Duration::from_secs(2)).unwrap();
        self.finished.send(()).unwrap();
        Ok(response(r, vec![call("move", json!({"to":[1,2,3]}))]))
    }
}
#[test]
fn independent_control_poll_discards_late_model_result_after_stop_goal_owner_or_expiry() {
    for drift in ["stop", "goal", "owner", "expiry"] {
        let mut h = FakeHost::new();
        h.drift = Some(drift);
        let host = Arc::new(h);
        let (tx, rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let p = BlockedProvider {
            started: host.started.clone(),
            release: rx,
            finished: done_tx,
        };
        let report = NativeTaskRunner::new(host.clone(), Box::new(p), config())
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(
            report.stop_reason,
            if drift == "expiry" {
                NativeTaskStopReason::ContextExpired
            } else {
                NativeTaskStopReason::ControlChanged
            }
        );
        assert_eq!(report.model_calls_spent, 1);
        tx.send(()).unwrap();
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(!host.events().iter().any(|e| e.starts_with("tool:")));
    }
}
#[test]
fn corrupt_budget_and_disabled_default_never_start_model_io() {
    let host = Arc::new(FakeHost::new());
    host.state.lock().unwrap().corrupt = true;
    assert!(
        NativeTaskRunner::new(host.clone(), provider(vec![]), config())
            .unwrap()
            .run()
            .is_err()
    );
    assert!(host.events().is_empty());
    assert!(NativeTaskRunner::new(host, provider(vec![]), NativeTaskRunConfig::default()).is_err());
}
#[test]
fn recovery_scans_later_unknown_before_new_context_and_stable_ids_are_bounded() {
    let host = Arc::new(FakeHost::new());
    host.state
        .lock()
        .unwrap()
        .recover
        .insert("operator-run:1".into(), NativeTaskRecovery::Pending);
    let mut c = config();
    c.max_turns = 2;
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.last_operation.as_deref(), Some("operator-run:1"));
    assert!(!host.events().iter().any(|e| e.starts_with("prepare:")));
    let mut c = config();
    c.run_id = "a".repeat(129);
    assert!(c.validate().is_err());
    c.run_id = "a".repeat(128);
    assert!(c.validate().is_ok());
    assert_eq!(c.operation_id(127).len(), 132);
}

#[test]
fn a_new_context_cannot_adopt_a_different_fence_under_the_existing_budget() {
    let mut h = FakeHost::new();
    h.context_drift = true;
    let host = Arc::new(h);
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::ControlChanged);
    assert!(
        !host
            .events()
            .iter()
            .any(|event| event.starts_with("reserve:") || event.starts_with("tool:"))
    );
}

#[test]
fn model_deadline_consumes_reservation_and_discards_late_effect() {
    let host = Arc::new(FakeHost::new());
    let (release, wait) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    let p = BlockedProvider {
        started: host.started.clone(),
        release: wait,
        finished,
    };
    let mut c = config();
    c.model_timeout_ms = 10;
    let report = NativeTaskRunner::new(host.clone(), Box::new(p), c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::ModelTimeout);
    assert_eq!(report.model_calls_spent, 1);
    release.send(()).unwrap();
    done.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(!host.events().iter().any(|event| event.starts_with("tool:")));
}
