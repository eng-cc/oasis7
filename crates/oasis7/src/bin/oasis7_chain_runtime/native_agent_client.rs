//! Opt-in first-party executor composition. World access remains signed HTTP.
use oasis7_agent_harness::{
    NativeTaskRunConfig, NativeTaskRunReport, NativeTaskRunner, NativeTaskStopReason,
    SdkNativeTaskHost,
};
use oasis7_game_client::device_client::Client;
use oasis7_model_provider::{ModelProviderConfig, OpenAiChatCompletionClient};
use serde::Serialize;
use std::{
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

pub(super) const PROFILE: &str = "native_task_industry_v1";
const PROFILE_ENV: &str = "OASIS7_AGENT_EXECUTION_PROFILE";
#[derive(Clone, Serialize)]
struct ExecutorStatus {
    profile: &'static str,
    execution_location: &'static str,
    phase: &'static str,
    reason: &'static str,
    model: String,
    max_model_calls: u32,
    reserved_model_calls: Option<u32>,
    committed_turns: u32,
    observed_at_unix_ms: i64,
}
static STATUS: OnceLock<Arc<Mutex<ExecutorStatus>>> = OnceLock::new();

pub(super) fn selected() -> Result<bool, String> {
    parse_profile(std::env::var(PROFILE_ENV).ok().as_deref())
}
fn parse_profile(value: Option<&str>) -> Result<bool, String> {
    match value {
        None | Some("legacy_simulator_v1") => Ok(false),
        Some(PROFILE) => Ok(true),
        Some(_) => {
            Err("unknown Agent execution profile; choose an explicit supported profile".into())
        }
    }
}
pub(super) struct PreparedExecutor {
    client: Arc<Client>,
    model: OpenAiChatCompletionClient,
    config: NativeTaskRunConfig,
}
impl PreparedExecutor {
    /// Loads operator-owned credentials and config only. No inference or world
    /// operation is performed before the local World Service is listening.
    pub(super) fn from_environment(
        enabled: bool,
        status_port: u16,
        world_id: &str,
        service_key: &str,
    ) -> Result<Option<Self>, String> {
        if !enabled {
            return Ok(None);
        }
        let required = |name: &str| {
            std::env::var(name).map_err(|_| format!("native executor requires {name}"))
        };
        let device_path = required("OASIS7_AGENT_DEVICE_CONFIG")?;
        if !Path::new(&device_path).is_absolute() {
            return Err("native executor requires an absolute device configuration path".into());
        }
        let client = Client::open(Path::new(&device_path))?;
        let remote = client.remote.config();
        let endpoint = reqwest::Url::parse(&remote.endpoint)
            .map_err(|_| "invalid native executor endpoint")?;
        let host = endpoint
            .host_str()
            .unwrap_or_default()
            .trim_matches(['[', ']']);
        if status_port == 0
            || endpoint.scheme() != "http"
            || !host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
            || endpoint.port_or_known_default() != Some(status_port)
            || !matches!(endpoint.path(), "" | "/")
            || client.world.world_id != world_id
            || remote.trusted_service_public_key != service_key
        {
            return Err("native executor device must target this node's literal-loopback World Service and service identity".into());
        }
        let provider = ModelProviderConfig::from_default_sources_for_agent(&client.agent)?;
        let max_model_calls = required("OASIS7_AGENT_MAX_MODEL_CALLS")?
            .parse::<u32>()
            .map_err(|_| "invalid explicit native model budget")?;
        let max_turns = std::env::var("OASIS7_AGENT_MAX_TURNS")
            .unwrap_or_else(|_| "16".into())
            .parse::<u32>()
            .map_err(|_| "invalid native turn limit")?;
        let config = NativeTaskRunConfig {
            run_id: required("OASIS7_AGENT_RUN_ID")?,
            model: provider.model.clone(),
            system_prompt: native_system_prompt(&provider.system_prompt),
            max_model_calls,
            max_turns,
            model_timeout_ms: provider.timeout_ms.min(120_000),
            max_tool_calls_per_call: u32::try_from(provider.max_module_calls)
                .map_err(|_| "invalid native tool limit")?,
            ..Default::default()
        };
        config.validate()?;
        let model = OpenAiChatCompletionClient::from_config(&provider)
            .map_err(|_| "native model configuration rejected")?;
        Ok(Some(Self {
            client: Arc::new(client),
            model,
            config,
        }))
    }
    /// The node process owns this worker. Closing a Viewer leaves it running;
    /// node exit ends it. Stop/goal/executor control remains Runtime authority.
    pub(super) fn start(self) -> Result<(), String> {
        let status = Arc::new(Mutex::new(ExecutorStatus {
            profile: PROFILE,
            execution_location: "node_process",
            phase: "starting",
            reason: "initializing_authorized_task",
            model: self.config.model.clone(),
            max_model_calls: self.config.max_model_calls,
            reserved_model_calls: None,
            committed_turns: 0,
            observed_at_unix_ms: super::now_unix_ms(),
        }));
        STATUS
            .set(status.clone())
            .map_err(|_| "native executor already configured in this node process")?;
        let host = Arc::new(SdkNativeTaskHost::new(self.client));
        let runner = NativeTaskRunner::new(host, Box::new(self.model), self.config)?;
        std::thread::Builder::new()
            .name("native-agent-client".into())
            .spawn(move || {
                if let Ok(mut state) = status.lock() {
                    state.phase = "running";
                    state.reason = "bounded_execution";
                }
                let result = runner.run();
                if let Ok(mut state) = status.lock() {
                    state.observed_at_unix_ms = super::now_unix_ms();
                    match result {
                        Ok(report) => apply_report(&mut state, &report),
                        Err(_) => {
                            state.phase = "blocked";
                            state.reason = "client_or_model_unavailable";
                        }
                    }
                }
            })
            .map_err(|_| "native executor worker unavailable")?;
        super::info!(
            profile = PROFILE,
            execution_location = "node_process",
            "first-party Agent client started with explicit bounded budget"
        );
        Ok(())
    }
}
fn native_system_prompt(style: &str) -> String {
    if style.trim().is_empty() {
        NativeTaskRunConfig::default().system_prompt
    } else {
        style.to_owned()
    }
}
fn apply_report(status: &mut ExecutorStatus, report: &NativeTaskRunReport) {
    status.reserved_model_calls = Some(report.model_calls_spent);
    status.committed_turns = report.turns_completed;
    let (phase, reason) = match report.stop_reason {
        NativeTaskStopReason::Completed => ("stopped", "turn_limit_reached"),
        NativeTaskStopReason::BudgetExhausted => ("blocked", "model_budget_exhausted"),
        NativeTaskStopReason::PendingOriginal => ("blocked", "original_result_pending"),
        NativeTaskStopReason::RejectedOriginal => ("blocked", "original_operation_rejected"),
        NativeTaskStopReason::ExpiredOriginal | NativeTaskStopReason::ContextExpired => {
            ("blocked", "context_expired")
        }
        NativeTaskStopReason::ControlChanged => ("stopped", "task_control_changed"),
        NativeTaskStopReason::ModelFailed => ("blocked", "model_unavailable"),
        NativeTaskStopReason::ModelTimeout => ("blocked", "model_timeout"),
        NativeTaskStopReason::InvalidModelOutput => ("blocked", "unsupported_model_output"),
        NativeTaskStopReason::Aborted => ("stopped", "executor_aborted"),
        NativeTaskStopReason::HostFailure => ("blocked", "client_unavailable"),
    };
    status.phase = phase;
    status.reason = reason;
}
pub(super) fn snapshot() -> serde_json::Value {
    match STATUS.get() {
        Some(status) => match status.lock() {
            Ok(status) => serde_json::to_value(&*status)
                .unwrap_or_else(|_| serde_json::json!({"phase":"unknown"})),
            Err(_) => serde_json::json!({"phase":"unknown","reason":"status_unavailable"}),
        },
        None => {
            serde_json::json!({"phase":"not_selected","profile":null,"execution_location":null})
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_requires_explicit_selection_and_does_not_fall_back() {
        assert!(!parse_profile(None).unwrap());
        assert!(!parse_profile(Some("legacy_simulator_v1")).unwrap());
        assert!(parse_profile(Some(PROFILE)).unwrap());
        for profile in ["", "builtin", "native_task", "unknown"] {
            assert!(parse_profile(Some(profile)).is_err());
        }
        assert!(!native_system_prompt("").is_empty());
        assert_eq!(native_system_prompt("operator style"), "operator style");
    }
    #[test]
    fn budget_and_turn_limit_status_do_not_claim_goal_completion() {
        let mut status = ExecutorStatus {
            profile: PROFILE,
            execution_location: "node_process",
            phase: "running",
            reason: "bounded_execution",
            model: "fixture".into(),
            max_model_calls: 2,
            reserved_model_calls: None,
            committed_turns: 0,
            observed_at_unix_ms: 0,
        };
        let mut report = NativeTaskRunReport {
            run_id: "run".into(),
            model_calls_spent: 2,
            turns_completed: 1,
            last_operation: None,
            stop_reason: NativeTaskStopReason::BudgetExhausted,
        };
        apply_report(&mut status, &report);
        assert_eq!(
            (status.phase, status.reason),
            ("blocked", "model_budget_exhausted")
        );
        report.stop_reason = NativeTaskStopReason::Completed;
        apply_report(&mut status, &report);
        assert_eq!(
            (status.phase, status.reason),
            ("stopped", "turn_limit_reached")
        );
    }
}
