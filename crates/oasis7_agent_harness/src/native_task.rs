//! Bounded native task execution against authenticated remote World Service.
//! No local World, clock advancement, RuntimeAuthority, or model SDK lives here.
use crate::{ModelProvider, ModelProviderRequest, ModelProviderResponse, ModelProviderToolResult};
use oasis7_agent_api::game::{GameTaskFenceV1, GameTaskStatusV1};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
#[path = "native_task_sdk.rs"]
mod sdk;
#[path = "native_task_tools.rs"]
mod tools;
pub use sdk::SdkNativeTaskHost;
pub use tools::NativeTaskTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTaskRunConfig {
    pub run_id: String,
    pub model: String,
    /// Operator system style; complete native goal remains authorized context.
    pub system_prompt: String,
    pub max_turns: u32,
    /// Explicit total physical calls across restarts; zero disables model I/O.
    pub max_model_calls: u32,
    pub max_tool_calls_per_call: u32,
    pub model_timeout_ms: u64,
    pub control_poll_ms: u64,
}
impl Default for NativeTaskRunConfig {
    fn default() -> Self {
        Self {
            run_id: String::new(),
            model: String::new(),
            system_prompt: "Operate the authorized remote native industry task.".into(),
            max_turns: 1,
            max_model_calls: 0,
            max_tool_calls_per_call: 8,
            model_timeout_ms: 30_000,
            control_poll_ms: 100,
        }
    }
}
impl NativeTaskRunConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.run_id.trim().is_empty()
            || self.run_id.len() > 128
            || self
                .run_id
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
            || self.model.trim().is_empty()
            || self.model.len() > 256
            || self.system_prompt.trim().is_empty()
            || self.system_prompt.len() > 32 * 1024
            || !(1..=128).contains(&self.max_turns)
            || !(1..=1024).contains(&self.max_model_calls)
            || !(1..=32).contains(&self.max_tool_calls_per_call)
            || !(10..=120_000).contains(&self.model_timeout_ms)
            || !(1..=1000).contains(&self.control_poll_ms)
            || self.control_poll_ms > self.model_timeout_ms
        {
            return Err("native run requires bounded explicit identity, model and budgets".into());
        }
        Ok(())
    }
    pub fn binding(&self) -> Value {
        serde_json::json!({"profile":"native_task_industry_v1", "config_digest":
            oasis7_agent_api::h_v1("oasis7.native-task.run-config.v1", self)})
    }
    pub fn operation_id(&self, index: u32) -> String {
        format!("{}:{index}", self.run_id)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTaskRecovery {
    Vacant,
    Ready,
    Settled,
    Pending,
    Rejected,
    Expired,
    Aborted,
}
#[derive(Debug, Clone)]
pub struct NativeTaskControl {
    pub fence: GameTaskFenceV1,
    pub logical_tick: u64,
}
#[derive(Debug, Clone)]
pub struct NativeTaskContext {
    pub operation: String,
    pub fence: GameTaskFenceV1,
    pub valid_until_tick: u64,
    /// Complete authorized goal, soft constraints, observation and catalog.
    pub model_context: Value,
}
#[derive(Debug, Clone)]
pub struct NativeTaskToolResult {
    pub value: Value,
    pub effect: Option<NativeTaskRecovery>,
}
/// A host is an authenticated service adapter, never a local execution authority.
/// Implementations must preserve immutable originals and fail on corrupt state.
pub trait NativeTaskHost: Send + Sync + 'static {
    fn initialize_run(&self, config: &NativeTaskRunConfig) -> Result<u32, String>;
    fn reserve_model_call(
        &self,
        config: &NativeTaskRunConfig,
        operation: &str,
    ) -> Result<u32, String>;
    fn recover(&self, operation: &str) -> Result<NativeTaskRecovery, String>;
    /// Return only an authoritative stale-base rejection strictly matched to the
    /// signed original operation. Absence never authorizes a replacement.
    fn known_rejection_feedback(
        &self,
        _operation: &str,
    ) -> Result<Option<oasis7_game_client::AuthorizedFeedbackRecord>, String> {
        Ok(None)
    }
    /// Settle only the original prepared context using the fixed public
    /// transport_exhausted failure outcome. Never prepare, replace or cancel an
    /// existing action. Settled requires a canonically committed termination;
    /// uncertain/rejected/expired originals retain their recovery state.
    fn settle_exhausted_context(&self, operation: &str) -> Result<NativeTaskRecovery, String>;
    fn prepare(&self, operation: &str) -> Result<NativeTaskContext, String>;
    fn current_control(&self) -> Result<NativeTaskControl, String>;
    /// The exact authenticated fence bound when this run budget was initialized.
    fn initial_fence(&self) -> Result<GameTaskFenceV1, String> {
        Ok(self.current_control()?.fence)
    }
    fn invoke(
        &self,
        context: &NativeTaskContext,
        tool: &NativeTaskTool,
    ) -> Result<NativeTaskToolResult, String>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTaskStopReason {
    Completed,
    BudgetExhausted,
    PendingOriginal,
    RejectedOriginal,
    ExpiredOriginal,
    ControlChanged,
    ContextExpired,
    ModelFailed,
    ModelTimeout,
    InvalidModelOutput,
    Aborted,
    HostFailure,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTaskRunReport {
    pub run_id: String,
    pub model_calls_spent: u32,
    pub turns_completed: u32,
    pub last_operation: Option<String>,
    pub stop_reason: NativeTaskStopReason,
}
pub struct NativeTaskRunner<H: NativeTaskHost> {
    host: Arc<H>,
    provider: Option<Box<dyn ModelProvider>>,
    config: NativeTaskRunConfig,
}
impl<H: NativeTaskHost> NativeTaskRunner<H> {
    pub fn new(
        host: Arc<H>,
        provider: Box<dyn ModelProvider>,
        config: NativeTaskRunConfig,
    ) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            host,
            provider: Some(provider),
            config,
        })
    }
    /// Scan every original operation before preparing any new context. A pending
    /// or unknown original halts this run; it never produces a replacement ID.
    pub fn run(mut self) -> Result<NativeTaskRunReport, String> {
        let spent = self.host.initialize_run(&self.config)?;
        let run_fence = self.host.initial_fence()?;
        let mut report = NativeTaskRunReport {
            run_id: self.config.run_id.clone(),
            model_calls_spent: spent,
            turns_completed: 0,
            last_operation: None,
            stop_reason: NativeTaskStopReason::Completed,
        };
        let mut recovered = Vec::new();
        let mut rejected_feedback = Vec::new();
        // Rejected originals consume immutable slots, never successful effects.
        // Bound recovery work independently of a model's willingness to finish.
        let slots = self
            .config
            .max_turns
            .saturating_add(self.config.max_model_calls)
            .min(128);
        for index in 0..slots {
            let operation = self.config.operation_id(index);
            let state = self.host.recover(&operation)?;
            if state == NativeTaskRecovery::Rejected
                && self.collect_stale_feedback(&operation, &mut rejected_feedback)?
            {
                recovered.push(state);
                continue;
            }
            if let Some(reason) = terminal(state) {
                report.last_operation = Some(operation);
                report.stop_reason = reason;
                return Ok(report);
            }
            if state == NativeTaskRecovery::Settled {
                report.turns_completed += 1;
            }
            recovered.push(state);
        }
        for (index, state) in recovered.into_iter().enumerate() {
            if report.turns_completed >= self.config.max_turns {
                break;
            }
            let operation = self.config.operation_id(index as u32);
            report.last_operation = Some(operation.clone());
            if matches!(
                state,
                NativeTaskRecovery::Settled | NativeTaskRecovery::Rejected
            ) {
                continue;
            }
            if report.model_calls_spent >= self.config.max_model_calls {
                report.stop_reason = if state == NativeTaskRecovery::Ready {
                    self.settle_exhausted_context(&operation, &run_fence)?
                } else {
                    NativeTaskStopReason::BudgetExhausted
                };
                break;
            }
            let initial = self.host.current_control()?;
            if initial.fence != run_fence || initial.fence.task_status != GameTaskStatusV1::Active {
                report.stop_reason = NativeTaskStopReason::ControlChanged;
                break;
            }
            if self
                .provider
                .as_ref()
                .ok_or("native provider unavailable")?
                .check_execution()
                .is_err()
            {
                report.stop_reason = NativeTaskStopReason::ControlChanged;
                break;
            }
            let context = self.host.prepare(&operation)?;
            if context.operation != operation || context.fence != run_fence {
                report.stop_reason = NativeTaskStopReason::ControlChanged;
                return Ok(report);
            }
            let mut transcript = Vec::<Value>::new();
            loop {
                if let Some(reason) = self.control_reason(&context)? {
                    report.stop_reason = reason;
                    return Ok(report);
                }
                if report.model_calls_spent >= self.config.max_model_calls {
                    report.stop_reason = self.settle_exhausted_context(&operation, &run_fence)?;
                    return Ok(report);
                }
                let request =
                    tools::request(&self.config, &context, &transcript, &rejected_feedback)?;
                report.model_calls_spent =
                    self.host.reserve_model_call(&self.config, &operation)?;
                // Recheck after durable reservation too. A canceled reservation
                // remains spent; no crashed/unknown call can be topped up.
                if let Some(reason) = self.control_reason(&context)? {
                    report.stop_reason = reason;
                    return Ok(report);
                }
                let response = match self.complete(&context, &request)? {
                    Ok(response) => response,
                    Err(reason) => {
                        report.stop_reason = reason;
                        return Ok(report);
                    }
                };
                let turns =
                    match tools::validate_turns(&response, self.config.max_tool_calls_per_call) {
                        Ok(turns) => turns,
                        Err(_) => {
                            report.stop_reason = NativeTaskStopReason::InvalidModelOutput;
                            return Ok(report);
                        }
                    };
                // Protocol call IDs are separate from World operation IDs.
                // Match all names/arguments before invoking even the first read.
                let pending = self
                    .provider
                    .as_ref()
                    .ok_or("native provider unavailable")?
                    .pending_tool_calls();
                if tools::validate_call_ids(&response, &pending).is_err() {
                    report.stop_reason = NativeTaskStopReason::InvalidModelOutput;
                    return Ok(report);
                }
                let mut finished = false;
                let mut effect_result = None;
                let mut supplier_results = Vec::new();
                for (index, tool) in turns.into_iter().enumerate() {
                    if let Some(reason) = self.control_reason(&context)? {
                        // Complete protocol causality without pretending that
                        // canceled calls executed or made a World effect.
                        let _ = self.record_supplier_results(
                            &pending,
                            supplier_results,
                            "execution_stopped",
                        );
                        report.stop_reason = reason;
                        return Ok(report);
                    }
                    let result = match self.host.invoke(&context, &tool) {
                        Ok(result) => result,
                        Err(_) => {
                            if let Some(call) = pending.get(index) {
                                supplier_results.push(ModelProviderToolResult {
                                    call_id: call.call_id.clone(),
                                    content: serde_json::json!({"status":"execution_unknown","reason":"host_failure"}),
                                });
                            }
                            let _ = self.record_supplier_results(
                                &pending,
                                supplier_results,
                                "host_failure",
                            );
                            report.stop_reason = NativeTaskStopReason::HostFailure;
                            return Ok(report);
                        }
                    };
                    if result.effect.is_some() != tool.side_effect() {
                        report.stop_reason = NativeTaskStopReason::HostFailure;
                        return Ok(report);
                    }
                    transcript.push(serde_json::json!({"tool":tool,"result":result.value}));
                    if let Some(call) = pending.get(index) {
                        supplier_results.push(ModelProviderToolResult {
                            call_id: call.call_id.clone(),
                            content: result.value.clone(),
                        });
                    }
                    if let Some(effect) = result.effect {
                        effect_result = Some((effect, matches!(tool, NativeTaskTool::Abort(_))));
                        finished = true;
                    }
                }
                if self
                    .record_supplier_results(&pending, supplier_results, "not_executed")
                    .is_err()
                {
                    report.stop_reason = NativeTaskStopReason::ModelFailed;
                    return Ok(report);
                }
                if let Some((effect, abort)) = effect_result {
                    if effect == NativeTaskRecovery::Rejected
                        && self.collect_stale_feedback(&operation, &mut rejected_feedback)?
                    {
                        break;
                    }
                    if let Some(reason) = terminal(effect) {
                        report.stop_reason = reason;
                        return Ok(report);
                    }
                    if effect != NativeTaskRecovery::Settled {
                        report.stop_reason = NativeTaskStopReason::HostFailure;
                        return Ok(report);
                    }
                    report.turns_completed += 1;
                    if abort {
                        report.stop_reason = NativeTaskStopReason::Aborted;
                        return Ok(report);
                    }
                }
                if finished {
                    break;
                }
            }
        }
        if report.turns_completed < self.config.max_turns
            && report.stop_reason == NativeTaskStopReason::Completed
        {
            report.stop_reason = NativeTaskStopReason::BudgetExhausted;
        }
        Ok(report)
    }
    fn record_supplier_results(
        &mut self,
        calls: &[crate::ModelProviderToolCall],
        mut results: Vec<ModelProviderToolResult>,
        remaining_reason: &str,
    ) -> Result<(), String> {
        if calls.is_empty() {
            return Ok(());
        }
        for call in calls.iter().skip(results.len()) {
            results.push(ModelProviderToolResult {
                call_id: call.call_id.clone(),
                content: serde_json::json!({"status":"not_executed","reason":remaining_reason}),
            });
        }
        self.provider
            .as_mut()
            .ok_or("native provider unavailable")?
            .record_tool_results(&results)
            .map_err(|_| "native supplier tool result recording failed".into())
    }
    fn collect_stale_feedback(
        &self,
        operation: &str,
        feedback: &mut Vec<Value>,
    ) -> Result<bool, String> {
        let Some(record) = self.host.known_rejection_feedback(operation)? else {
            return Ok(false);
        };
        feedback
            .push(serde_json::json!({"original_operation":operation,"authorized_feedback":record}));
        Ok(true)
    }
    fn settle_exhausted_context(
        &self,
        operation: &str,
        run_fence: &GameTaskFenceV1,
    ) -> Result<NativeTaskStopReason, String> {
        if self
            .provider
            .as_ref()
            .is_some_and(|provider| provider.check_execution().is_err())
        {
            return Ok(NativeTaskStopReason::ControlChanged);
        }
        let current = self.host.current_control()?;
        if &current.fence != run_fence || current.fence.task_status != GameTaskStatusV1::Active {
            return Ok(NativeTaskStopReason::ControlChanged);
        }
        let state = self.host.settle_exhausted_context(operation)?;
        Ok(
            terminal(state).unwrap_or(if state == NativeTaskRecovery::Settled {
                NativeTaskStopReason::BudgetExhausted
            } else {
                NativeTaskStopReason::HostFailure
            }),
        )
    }
    fn control_reason(
        &self,
        context: &NativeTaskContext,
    ) -> Result<Option<NativeTaskStopReason>, String> {
        if self
            .provider
            .as_ref()
            .is_some_and(|provider| provider.check_execution().is_err())
        {
            return Ok(Some(NativeTaskStopReason::ControlChanged));
        }
        let current = self.host.current_control()?;
        if current.fence != context.fence || current.fence.task_status != GameTaskStatusV1::Active {
            return Ok(Some(NativeTaskStopReason::ControlChanged));
        }
        if current.logical_tick > context.valid_until_tick {
            return Ok(Some(NativeTaskStopReason::ContextExpired));
        }
        Ok(None)
    }
    fn complete(
        &mut self,
        context: &NativeTaskContext,
        request: &ModelProviderRequest,
    ) -> Result<Result<ModelProviderResponse, NativeTaskStopReason>, String> {
        let mut provider = self
            .provider
            .take()
            .ok_or("native provider worker already active")?;
        let physical_request = request.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = provider.complete_checked(&physical_request);
            let _ = tx.send((provider, result));
        });
        let start = Instant::now();
        loop {
            if let Some(reason) = self.control_reason(context)? {
                return Ok(Err(reason));
            }
            let timeout = Duration::from_millis(self.config.model_timeout_ms);
            let Some(remaining) = timeout.checked_sub(start.elapsed()) else {
                return Ok(Err(NativeTaskStopReason::ModelTimeout));
            };
            match rx.recv_timeout(remaining.min(Duration::from_millis(self.config.control_poll_ms)))
            {
                Ok((provider, response)) => {
                    self.provider = Some(provider);
                    if let Some(reason) = self.control_reason(context)? {
                        return Ok(Err(reason));
                    }
                    return Ok(response.map_err(|_| NativeTaskStopReason::ModelFailed));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Ok(Err(NativeTaskStopReason::ModelFailed));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
            }
        }
    }
}
fn terminal(state: NativeTaskRecovery) -> Option<NativeTaskStopReason> {
    match state {
        NativeTaskRecovery::Pending => Some(NativeTaskStopReason::PendingOriginal),
        NativeTaskRecovery::Rejected => Some(NativeTaskStopReason::RejectedOriginal),
        NativeTaskRecovery::Expired => Some(NativeTaskStopReason::ExpiredOriginal),
        NativeTaskRecovery::Aborted => Some(NativeTaskStopReason::Aborted),
        _ => None,
    }
}
#[cfg(test)]
#[path = "native_task_tests.rs"]
mod tests;
