use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use oasis7::simulator::{
    evaluate_provider_compatibility, initialize_kernel, provider_phase1_required_actions,
    provider_phase1_required_capabilities, Action, ActionCatalogEntry, ActionResult, AgentBehavior,
    AgentDecision, AgentDecisionTrace, AgentRunner, LlmAgentBehavior, Observation,
    OpenAiChatCompletionClient, ProviderLoopbackAdapter, ProviderLoopbackHttpClient,
    ProviderCompatibilityStatus, ProviderExecutionMode, RuntimePerfSnapshot, WorldConfig,
    WorldEvent, WorldInitConfig, WorldScenario, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};

const DEFAULT_PROTOCOL_VERSION: &str = "2026-03-12";
const DEFAULT_ADAPTER_VERSION: &str = "provider_phase1_adapter_v1";
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_TICKS: u64 = 20;
const DEFAULT_PROVIDER_CONNECT_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_PROVIDER_AGENT_PROFILE: &str = "oasis7_p0_low_freq_npc";
const DEFAULT_MAX_MOVE_DISTANCE_CM_PER_TICK: i64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BenchProviderKind {
    Builtin,
    ProviderLoopbackHttp,
}

impl BenchProviderKind {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "builtin" => Some(Self::Builtin),
            "provider_loopback_http" | "provider_local_bridge" => {
                Some(Self::ProviderLoopbackHttp)
            }
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::ProviderLoopbackHttp => "provider_loopback_http",
        }
    }

    fn summary_suffix(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    provider: BenchProviderKind,
    scenario: WorldScenario,
    scenario_id: String,
    parity_tier: String,
    benchmark_run_id: String,
    fixture_id: Option<String>,
    protocol_version: String,
    adapter_version: String,
    ticks: u64,
    timeout_ms: u64,
    out_dir: PathBuf,
    provider_base_url: Option<String>,
    provider_auth_token: Option<String>,
    agent_provider_connect_timeout_ms: u64,
    agent_provider_profile: String,
    execution_mode: ProviderExecutionMode,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            provider: BenchProviderKind::Builtin,
            scenario: WorldScenario::LlmBootstrap,
            scenario_id: "P0-001".to_string(),
            parity_tier: "P0".to_string(),
            benchmark_run_id: "manual".to_string(),
            fixture_id: None,
            protocol_version: DEFAULT_PROTOCOL_VERSION.to_string(),
            adapter_version: DEFAULT_ADAPTER_VERSION.to_string(),
            ticks: DEFAULT_TICKS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            out_dir: PathBuf::from("output/provider_parity/manual"),
            provider_base_url: None,
            provider_auth_token: None,
            agent_provider_connect_timeout_ms: DEFAULT_PROVIDER_CONNECT_TIMEOUT_MS,
            agent_provider_profile: DEFAULT_PROVIDER_AGENT_PROFILE.to_string(),
            execution_mode: ProviderExecutionMode::HeadlessAgent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProviderRunInfo {
    provider_kind: String,
    provider_version: String,
    adapter_version: String,
    protocol_version: String,
    compatibility_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supported_action_sets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_queue_depth: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FixtureRefs {
    initial_world_snapshot_ref: String,
    observation_sequence_ref: String,
    goal_definition: String,
    action_catalog_ref: String,
    player_context_ref: String,
    memory_fixture_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StepTraceRecord {
    benchmark_run_id: String,
    mode: String,
    observation_schema_version: String,
    action_schema_version: String,
    environment_class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback_reason: Option<String>,
    parity_tier: String,
    scenario_id: String,
    fixture_id: String,
    provider_kind: String,
    provider_version: String,
    adapter_version: String,
    protocol_version: String,
    step_index: u64,
    agent_id: String,
    decision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    retry_count: u32,
    trace_present: bool,
    trace_message_count: usize,
    trace_tool_call_count: usize,
    context_drift_flag: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_success: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct SampleSummary {
    benchmark_run_id: String,
    mode: String,
    observation_schema_version: String,
    action_schema_version: String,
    environment_class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback_reason: Option<String>,
    parity_tier: String,
    scenario_id: String,
    fixture_id: String,
    provider_kind: String,
    provider_version: String,
    adapter_version: String,
    protocol_version: String,
    scenario: String,
    seed: String,
    status: String,
    goal_completed: bool,
    completion_time_ms: u64,
    decision_steps: u64,
    invalid_action_count: u64,
    timeout_count: u64,
    recoverable_error_count: u64,
    fatal_error_count: u64,
    trace_completeness_ratio_ppm: u64,
    median_latency_ms: u64,
    p95_latency_ms: u64,
    context_drift_count: u64,
    action_kind_counts: BTreeMap<String, u64>,
    error_counts: BTreeMap<String, u64>,
    fixture_refs: FixtureRefs,
    provider: ProviderRunInfo,
    notes: Vec<String>,
    runtime_perf: RuntimePerfSnapshot,
}

enum BenchBehavior {
    Builtin(BuiltinParityBehavior),
    ProviderBacked(ProviderBackedLoopbackBehavior),
}

struct BuiltinParityBehavior {
    inner: LlmAgentBehavior<OpenAiChatCompletionClient>,
    scenario_id: String,
    pending_trace: Option<AgentDecisionTrace>,
}

struct ProviderBackedLoopbackBehavior {
    inner: oasis7::simulator::ProviderBackedAgentBehavior<ProviderLoopbackAdapter>,
}

impl AgentBehavior for BenchBehavior {
    fn agent_id(&self) -> &str {
        match self {
            Self::Builtin(inner) => inner.agent_id(),
            Self::ProviderBacked(inner) => inner.agent_id(),
        }
    }

    fn decide(&mut self, observation: &Observation) -> AgentDecision {
        match self {
            Self::Builtin(inner) => inner.decide(observation),
            Self::ProviderBacked(inner) => inner.decide(observation),
        }
    }

    fn on_action_result(&mut self, result: &ActionResult) {
        match self {
            Self::Builtin(inner) => inner.on_action_result(result),
            Self::ProviderBacked(inner) => inner.on_action_result(result),
        }
    }

    fn on_event(&mut self, event: &WorldEvent) {
        match self {
            Self::Builtin(inner) => inner.on_event(event),
            Self::ProviderBacked(inner) => inner.on_event(event),
        }
    }

    fn take_decision_trace(&mut self) -> Option<AgentDecisionTrace> {
        match self {
            Self::Builtin(inner) => inner.take_decision_trace(),
            Self::ProviderBacked(inner) => inner.take_decision_trace(),
        }
    }
}

impl AgentBehavior for BuiltinParityBehavior {
    fn agent_id(&self) -> &str {
        self.inner.agent_id()
    }

    fn decide(&mut self, observation: &Observation) -> AgentDecision {
        let original_decision = self.inner.decide(observation);
        let mut trace = self.inner.take_decision_trace();
        let (decision, guardrail_note) = apply_builtin_parity_guardrail(
            self.scenario_id.as_str(),
            self.inner.agent_id(),
            observation,
            original_decision.clone(),
        );
        if let Some(note) = guardrail_note {
            if let Some(trace) = trace.as_mut() {
                trace.decision = decision.clone();
                trace.llm_step_trace.push(oasis7::simulator::LlmStepTrace {
                    step_index: trace.llm_step_trace.len(),
                    step_type: "builtin_parity_guardrail".to_string(),
                    input_summary: decision_label(&original_decision),
                    output_summary: decision_label(&decision),
                    status: note,
                });
            }
        }
        self.pending_trace = trace;
        decision
    }

    fn on_action_result(&mut self, result: &ActionResult) {
        self.inner.on_action_result(result);
    }

    fn on_event(&mut self, event: &WorldEvent) {
        self.inner.on_event(event);
    }

    fn take_decision_trace(&mut self) -> Option<AgentDecisionTrace> {
        self.pending_trace.take()
    }
}

fn apply_builtin_parity_guardrail(
    scenario_id: &str,
    agent_id: &str,
    observation: &Observation,
    decision: AgentDecision,
) -> (AgentDecision, Option<String>) {
    if scenario_id != "P0-001" {
        return (decision, None);
    }
    let Some(preferred_location) = preferred_patrol_move_target(observation) else {
        return (decision, None);
    };
    if decision_is_valid_patrol_move(&decision, observation) {
        return (decision, None);
    }
    (
        AgentDecision::Act(Action::MoveAgent {
            agent_id: agent_id.to_string(),
            to: preferred_location.clone(),
        }),
        Some(format!(
            "builtin_parity_guardrail: reroute {} -> move_agent({})",
            decision_label(&decision),
            preferred_location,
        )),
    )
}

fn decision_is_valid_patrol_move(decision: &AgentDecision, observation: &Observation) -> bool {
    let current_location_id = estimated_current_location_id(observation);
    matches!(
        decision,
        AgentDecision::Act(Action::MoveAgent { to, .. })
            if observation.visible_locations.iter().any(|location| {
                location.location_id == *to
                    && location.distance_cm > 0
                    && location.distance_cm <= DEFAULT_MAX_MOVE_DISTANCE_CM_PER_TICK
                    && Some(location.location_id.as_str()) != current_location_id
            })
    )
}

fn preferred_patrol_move_target(observation: &Observation) -> Option<String> {
    let current_location_id = estimated_current_location_id(observation);
    observation
        .visible_locations
        .iter()
        .filter(|location| {
            location.distance_cm > 0
                && location.distance_cm <= DEFAULT_MAX_MOVE_DISTANCE_CM_PER_TICK
                && Some(location.location_id.as_str()) != current_location_id
        })
        .min_by_key(|location| location.distance_cm)
        .map(|location| location.location_id.clone())
}

fn estimated_current_location_id(observation: &Observation) -> Option<&str> {
    observation
        .visible_locations
        .iter()
        .min_by_key(|location| location.distance_cm)
        .map(|location| location.location_id.as_str())
}

impl AgentBehavior for ProviderBackedLoopbackBehavior {
    fn agent_id(&self) -> &str {
        self.inner.agent_id()
    }

    fn decide(&mut self, observation: &Observation) -> AgentDecision {
        self.inner.decide(observation)
    }

    fn on_action_result(&mut self, result: &ActionResult) {
        self.inner.on_action_result(result);
    }

    fn on_event(&mut self, event: &WorldEvent) {
        self.inner.on_event(event);
    }

    fn take_decision_trace(&mut self) -> Option<AgentDecisionTrace> {
        self.inner.take_decision_trace()
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let options = match parse_options(args.iter().skip(1).map(String::as_str)) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            print_help();
            process::exit(1);
        }
    };

    let config = WorldConfig::default();
    let init = WorldInitConfig::from_scenario(options.scenario, &config);
    let (mut kernel, init_report) = match initialize_kernel(config, init) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to initialize world: {err:?}");
            process::exit(1);
        }
    };
    let seed = init_report.seed.to_string();
    let fixture_id = options
        .fixture_id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", options.scenario.as_str(), seed));

    let provider = match prepare_provider_info(&options) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to prepare provider: {err}");
            process::exit(1);
        }
    };

    let raw_dir = options.out_dir.join("raw");
    let summary_dir = options.out_dir.join("summary");
    if let Err(err) = fs::create_dir_all(&raw_dir) {
        eprintln!("failed to create raw dir {}: {err}", raw_dir.display());
        process::exit(1);
    }
    if let Err(err) = fs::create_dir_all(&summary_dir) {
        eprintln!(
            "failed to create summary dir {}: {err}",
            summary_dir.display()
        );
        process::exit(1);
    }

    let raw_path = raw_dir.join(format!(
        "{}.{}.jsonl",
        sanitize_filename(fixture_id.as_str()),
        options.provider.summary_suffix()
    ));
    let summary_path = summary_dir.join(format!(
        "{}.{}.json",
        sanitize_filename(options.scenario_id.as_str()),
        options.provider.summary_suffix()
    ));

    let mut runner: AgentRunner<BenchBehavior> = AgentRunner::new();
    let mut agent_ids: Vec<String> = kernel.model().agents.keys().cloned().collect();
    agent_ids.sort();
    if agent_ids.is_empty() {
        eprintln!("no agents in scenario {}", options.scenario.as_str());
        process::exit(1);
    }

    for agent_id in &agent_ids {
        let behavior =
            match build_behavior(agent_id.as_str(), &options, fixture_id.as_str(), &provider) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to build behavior for {agent_id}: {err}");
                    process::exit(1);
                }
            };
        runner.register(behavior);
    }

    let run_started_at = Instant::now();
    let mut step_records = Vec::new();
    let mut notes = Vec::new();
    let mut action_kind_counts = BTreeMap::new();
    let mut error_counts = BTreeMap::new();
    let mut invalid_action_count = 0_u64;
    let mut timeout_count = 0_u64;
    let mut recoverable_error_count = 0_u64;
    let mut fatal_error_count = 0_u64;
    let mut context_drift_count = 0_u64;
    let mut trace_present_count = 0_u64;
    let mut decision_steps = 0_u64;
    let mut latencies = Vec::new();

    for step_index in 1..=options.ticks {
        let Some(result) = runner.tick(&mut kernel) else {
            notes.push(format!("step {step_index}: runner returned no result"));
            continue;
        };
        decision_steps += 1;
        let action_ref = action_ref_from_decision(&result.decision);
        if let Some(action_ref) = action_ref.as_ref() {
            let entry = action_kind_counts.entry(action_ref.clone()).or_insert(0);
            *entry += 1;
        }

        let trace_present = result.decision_trace.is_some();
        if trace_present {
            trace_present_count += 1;
        }
        let latency_ms = result
            .decision_trace
            .as_ref()
            .and_then(|trace| trace.llm_diagnostics.as_ref())
            .and_then(|diagnostics| diagnostics.latency_ms);
        if let Some(latency_ms) = latency_ms {
            latencies.push(latency_ms);
        }

        let mut error_code = classify_trace_error(
            result.decision_trace.as_ref(),
            result.action_result.as_ref(),
        );
        if let Some(code) = error_code.as_ref() {
            let entry = error_counts.entry(code.clone()).or_insert(0);
            *entry += 1;
            match code.as_str() {
                "timeout" => {
                    timeout_count += 1;
                    recoverable_error_count += 1;
                }
                "provider_unreachable" | "invalid_action_schema" | "action_rejected" => {
                    recoverable_error_count += 1;
                }
                "context_drift" => {
                    context_drift_count += 1;
                }
                "session_cross_talk" => {
                    fatal_error_count += 1;
                }
                "trace_missing" => {}
                _ => {
                    fatal_error_count += 1;
                }
            }
        }

        let action_success = result.action_result.as_ref().map(|value| value.success);
        if matches!(action_success, Some(false)) {
            invalid_action_count += 1;
            let entry = error_counts
                .entry("action_rejected".to_string())
                .or_insert(0);
            *entry += 1;
            if error_code.is_none() {
                error_code = Some("action_rejected".to_string());
            }
        }

        if let Some(result_action) = result.action_result.as_ref() {
            if let Some(reject_reason) = result_action.reject_reason() {
                notes.push(format!(
                    "step {step_index}: action rejected for agent {} with {:?}",
                    result.agent_id, reject_reason
                ));
            }
        }

        step_records.push(StepTraceRecord {
            benchmark_run_id: options.benchmark_run_id.clone(),
            mode: options.execution_mode.as_str().to_string(),
            observation_schema_version: DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
            action_schema_version: DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string(),
            environment_class: execution_environment_class(options.execution_mode).to_string(),
            fallback_reason: provider.fallback_reason.clone(),
            parity_tier: options.parity_tier.clone(),
            scenario_id: options.scenario_id.clone(),
            fixture_id: fixture_id.clone(),
            provider_kind: options.provider.as_str().to_string(),
            provider_version: provider.provider_version.clone(),
            adapter_version: options.adapter_version.clone(),
            protocol_version: options.protocol_version.clone(),
            step_index,
            agent_id: result.agent_id.clone(),
            decision: decision_label(&result.decision),
            action_ref,
            latency_ms,
            error_code,
            retry_count: result
                .decision_trace
                .as_ref()
                .and_then(|trace| trace.llm_diagnostics.as_ref())
                .map(|diagnostics| diagnostics.retry_count)
                .unwrap_or(0),
            trace_present,
            trace_message_count: result
                .decision_trace
                .as_ref()
                .map(|trace| trace.llm_chat_messages.len())
                .unwrap_or(0),
            trace_tool_call_count: result
                .decision_trace
                .as_ref()
                .map(|trace| trace.llm_step_trace.len())
                .unwrap_or(0),
            context_drift_flag: false,
            action_success,
        });
    }

    let goal_completed = scenario_goal_completed(
        options.scenario_id.as_str(),
        &action_kind_counts,
        &error_counts,
        invalid_action_count,
    );
    let status = derive_status(goal_completed, &error_counts, &notes);
    let trace_completeness_ratio_ppm = ratio_ppm(trace_present_count, decision_steps);
    let summary = SampleSummary {
        benchmark_run_id: options.benchmark_run_id.clone(),
        mode: options.execution_mode.as_str().to_string(),
        observation_schema_version: DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string(),
        environment_class: execution_environment_class(options.execution_mode).to_string(),
        fallback_reason: provider.fallback_reason.clone(),
        parity_tier: options.parity_tier.clone(),
        scenario_id: options.scenario_id.clone(),
        fixture_id,
        provider_kind: options.provider.as_str().to_string(),
        provider_version: provider.provider_version.clone(),
        adapter_version: options.adapter_version.clone(),
        protocol_version: options.protocol_version.clone(),
        scenario: options.scenario.as_str().to_string(),
        seed,
        status,
        goal_completed,
        completion_time_ms: run_started_at.elapsed().as_millis().min(u64::MAX as u128) as u64,
        decision_steps,
        invalid_action_count,
        timeout_count,
        recoverable_error_count,
        fatal_error_count,
        trace_completeness_ratio_ppm,
        median_latency_ms: percentile_u64(&latencies, 50.0),
        p95_latency_ms: percentile_u64(&latencies, 95.0),
        context_drift_count,
        action_kind_counts,
        error_counts,
        fixture_refs: FixtureRefs {
            initial_world_snapshot_ref: format!(
                "scenario://{}/snapshot",
                options.scenario.as_str()
            ),
            observation_sequence_ref: format!(
                "scenario://{}/observations",
                options.scenario.as_str()
            ),
            goal_definition: format!("parity://{}/{}", options.parity_tier, options.scenario_id),
            action_catalog_ref: "catalog://provider/phase1".to_string(),
            player_context_ref: "player://default".to_string(),
            memory_fixture_ref: "memory://default".to_string(),
        },
        provider,
        notes,
        runtime_perf: runner.runtime_perf_snapshot(),
    };

    if let Err(err) = write_jsonl(raw_path.as_path(), &step_records) {
        eprintln!("failed to write raw trace jsonl: {err}");
        process::exit(1);
    }
    if let Err(err) = write_json(summary_path.as_path(), &summary) {
        eprintln!("failed to write summary json: {err}");
        process::exit(1);
    }

    println!("provider: {}", options.provider.as_str());
    println!("scenario: {}", options.scenario.as_str());
    println!("scenario_id: {}", options.scenario_id);
    println!("benchmark_run_id: {}", options.benchmark_run_id);
    println!("summary_json: {}", summary_path.display());
    println!("raw_jsonl: {}", raw_path.display());
    println!("status: {}", summary.status);
    println!(
        "goal_completed: {}",
        if summary.goal_completed { 1 } else { 0 }
    );
    println!("decision_steps: {}", summary.decision_steps);
    println!("invalid_action_count: {}", summary.invalid_action_count);
    println!("timeout_count: {}", summary.timeout_count);
    println!(
        "trace_completeness_ratio_ppm: {}",
        summary.trace_completeness_ratio_ppm
    );
    println!("median_latency_ms: {}", summary.median_latency_ms);
    println!("p95_latency_ms: {}", summary.p95_latency_ms);
}

fn execution_environment_class(mode: ProviderExecutionMode) -> &'static str {
    match mode {
        ProviderExecutionMode::PlayerParity => "player_parity_linux",
        ProviderExecutionMode::HeadlessAgent => "headless_linux",
    }
}

fn prepare_provider_info(options: &CliOptions) -> Result<ProviderRunInfo, String> {
    match options.provider {
        BenchProviderKind::Builtin => Ok(ProviderRunInfo {
            provider_kind: options.provider.as_str().to_string(),
            provider_version: "builtin_llm_env".to_string(),
            adapter_version: options.adapter_version.clone(),
            protocol_version: options.protocol_version.clone(),
            compatibility_status: ProviderCompatibilityStatus::Ready
                .as_str()
                .to_string(),
            fallback_reason: None,
            capabilities: provider_phase1_required_capabilities()
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            supported_action_sets: provider_phase1_required_actions()
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            provider_status: None,
            provider_last_error: None,
            provider_queue_depth: None,
            agent_profile: None,
        }),
        BenchProviderKind::ProviderLoopbackHttp => {
            let base_url = options.provider_base_url.as_deref().ok_or_else(|| {
                "--agent-provider-url is required for provider_loopback_http".to_string()
            })?;
            let client = ProviderLoopbackHttpClient::new(
                base_url,
                options.provider_auth_token.as_deref(),
                options.agent_provider_connect_timeout_ms,
            )
            .map_err(|err| err.to_string())?;
            let info = client.provider_info().map_err(|err| err.to_string())?;
            let health = client.provider_health().map_err(|err| err.to_string())?;
            let compatibility = evaluate_provider_compatibility(&info, Some(&health));
            Ok(ProviderRunInfo {
                provider_kind: options.provider.as_str().to_string(),
                provider_version: info.version.unwrap_or_else(|| "unknown".to_string()),
                adapter_version: options.adapter_version.clone(),
                protocol_version: info
                    .protocol_version
                    .unwrap_or_else(|| options.protocol_version.clone()),
                compatibility_status: compatibility.status.as_str().to_string(),
                fallback_reason: compatibility.fallback_reason,
                capabilities: info.capabilities,
                supported_action_sets: info.supported_action_sets,
                provider_status: health.status,
                provider_last_error: health.last_error,
                provider_queue_depth: health.queue_depth,
                agent_profile: Some(options.agent_provider_profile.clone()),
            })
        }
    }
}

fn build_behavior(
    agent_id: &str,
    options: &CliOptions,
    fixture_id: &str,
    provider: &ProviderRunInfo,
) -> Result<BenchBehavior, String> {
    match options.provider {
        BenchProviderKind::Builtin => {
            if options.execution_mode != ProviderExecutionMode::HeadlessAgent {
                return Err(
                    "--execution-mode=player_parity is only supported with --provider provider_loopback_http"
                        .to_string(),
                );
            }
            let mut behavior =
                LlmAgentBehavior::from_env(agent_id.to_string()).map_err(|err| err.to_string())?;
            if let Some(goal) = builtin_parity_short_term_goal(options.scenario_id.as_str()) {
                behavior.apply_prompt_overrides(None, Some(goal), None);
            }
            Ok(BenchBehavior::Builtin(BuiltinParityBehavior {
                inner: behavior,
                scenario_id: options.scenario_id.clone(),
                pending_trace: None,
            }))
        }
        BenchProviderKind::ProviderLoopbackHttp => {
            let base_url = options.provider_base_url.as_deref().ok_or_else(|| {
                "--agent-provider-url is required for provider_loopback_http".to_string()
            })?;
            let adapter = ProviderLoopbackAdapter::new(
                base_url,
                options.provider_auth_token.as_deref(),
                options.agent_provider_connect_timeout_ms,
            )
            .map_err(|err| err.to_string())?;
            let mut behavior = oasis7::simulator::ProviderBackedAgentBehavior::new(
                agent_id.to_string(),
                adapter,
                phase1_action_catalog(),
            )
            .with_provider_config_ref(format!(
                "provider://loopback-http/parity/{}/{}",
                options.benchmark_run_id, agent_id
            ))
            .with_agent_profile(options.agent_provider_profile.clone())
            .with_execution_mode(options.execution_mode)
            .with_environment_class(execution_environment_class(options.execution_mode))
            .with_fixture_id(fixture_id)
            .with_replay_id(format!("{}:{}", options.benchmark_run_id, fixture_id));
            if let Some(fallback_reason) = provider.fallback_reason.as_deref() {
                behavior = behavior.with_fallback_reason(fallback_reason);
            }
            if let Some(memory_summary) = parity_memory_summary(options.scenario_id.as_str()) {
                behavior = behavior.with_memory_summary(memory_summary);
            }
            Ok(BenchBehavior::ProviderBacked(ProviderBackedLoopbackBehavior {
                inner: behavior,
            }))
        }
    }
}

fn builtin_parity_short_term_goal(scenario_id: &str) -> Option<String> {
    parity_memory_summary(scenario_id).map(str::to_string)
}

fn parity_memory_summary(scenario_id: &str) -> Option<&'static str> {
    match scenario_id {
        "P0-001" => Some(
            "goal=巡游移动; prefer move_agent to the nearest visible non-current location; do not idle when a legal move is available",
        ),
        "P0-002" => Some(
            "goal=近邻观察; prefer inspect_target on a visible agent or location before waiting",
        ),
        "P0-003" => Some(
            "goal=简单对话; prefer speak_to_nearby with a short nearby message instead of idle waiting",
        ),
        "P0-004" => Some(
            "goal=简单交互; prefer one legal simple_interact on a visible target before waiting",
        ),
        "P0-005" => Some(
            "goal=拒绝路径恢复; after one recoverable failure, prefer a legal recovery action or short wait_ticks with an explicit recovery attempt",
        ),
        _ => None,
    }
}

fn phase1_action_catalog() -> Vec<ActionCatalogEntry> {
    vec![
        ActionCatalogEntry::new("wait", "yield current turn without acting"),
        ActionCatalogEntry::new("wait_ticks", "sleep for a bounded number of ticks"),
        ActionCatalogEntry::new("move_agent", "move to a neighboring location"),
        ActionCatalogEntry::new("speak_to_nearby", "emit a lightweight nearby speech event"),
        ActionCatalogEntry::new(
            "inspect_target",
            "emit a lightweight target inspection event",
        ),
        ActionCatalogEntry::new(
            "simple_interact",
            "emit a lightweight single-step interaction event",
        ),
    ]
}

fn action_ref_from_decision(decision: &AgentDecision) -> Option<String> {
    match decision {
        AgentDecision::Wait => Some("wait".to_string()),
        AgentDecision::WaitTicks(_) => Some("wait_ticks".to_string()),
        AgentDecision::Act(action) => Some(action_ref_from_action(action).to_string()),
    }
}

fn action_ref_from_action(action: &Action) -> &'static str {
    match action {
        Action::MoveAgent { .. } => "move_agent",
        Action::SpeakToNearby { .. } => "speak_to_nearby",
        Action::InspectTarget { .. } => "inspect_target",
        Action::SimpleInteract { .. } => "simple_interact",
        _ => "other",
    }
}

fn decision_label(decision: &AgentDecision) -> String {
    match decision {
        AgentDecision::Wait => "wait".to_string(),
        AgentDecision::WaitTicks(ticks) => format!("wait_ticks:{ticks}"),
        AgentDecision::Act(action) => format!("act:{}", action_ref_from_action(action)),
    }
}

fn classify_trace_error(
    trace: Option<&AgentDecisionTrace>,
    action_result: Option<&ActionResult>,
) -> Option<String> {
    if let Some(result) = action_result {
        if !result.success {
            return Some("action_rejected".to_string());
        }
    }
    let err =
        trace.and_then(|value| value.llm_error.as_deref().or(value.parse_error.as_deref()))?;
    let lowered = err.to_ascii_lowercase();
    if lowered.contains("timeout") {
        Some("timeout".to_string())
    } else if lowered.contains("provider_unreachable") || lowered.contains("unreachable") {
        Some("provider_unreachable".to_string())
    } else if lowered.contains("invalid_action_schema") || lowered.contains("schema") {
        Some("invalid_action_schema".to_string())
    } else if lowered.contains("session_cross_talk") || lowered.contains("cross talk") {
        Some("session_cross_talk".to_string())
    } else if lowered.contains("context_drift") || lowered.contains("drift") {
        Some("context_drift".to_string())
    } else {
        None
    }
}

fn scenario_goal_completed(
    scenario_id: &str,
    action_kind_counts: &BTreeMap<String, u64>,
    error_counts: &BTreeMap<String, u64>,
    invalid_action_count: u64,
) -> bool {
    match scenario_id {
        "P0-001" => action_kind_counts.get("move_agent").copied().unwrap_or(0) >= 3,
        "P0-002" => {
            action_kind_counts
                .get("inspect_target")
                .copied()
                .unwrap_or(0)
                >= 1
        }
        "P0-003" => {
            action_kind_counts
                .get("speak_to_nearby")
                .copied()
                .unwrap_or(0)
                >= 2
        }
        "P0-004" => {
            action_kind_counts
                .get("simple_interact")
                .copied()
                .unwrap_or(0)
                >= 1
                && invalid_action_count == 0
        }
        "P0-005" => !error_counts.is_empty() && invalid_action_count == 0,
        _ => action_kind_counts.values().copied().sum::<u64>() > 0,
    }
}

fn derive_status(
    goal_completed: bool,
    error_counts: &BTreeMap<String, u64>,
    notes: &[String],
) -> String {
    if error_counts.contains_key("session_cross_talk") {
        return "blocked".to_string();
    }
    if notes.iter().any(|note| note.contains("invalid_fixture")) {
        return "invalid_fixture".to_string();
    }
    if goal_completed {
        "passed".to_string()
    } else {
        "failed".to_string()
    }
}

fn ratio_ppm(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        0
    } else {
        numerator
            .saturating_mul(1_000_000)
            .saturating_div(denominator)
    }
}

fn percentile_u64(values: &[u64], percentile: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (((sorted.len() - 1) as f64) * percentile / 100.0).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn write_jsonl(path: &Path, records: &[StepTraceRecord]) -> Result<(), String> {
    let mut file =
        File::create(path).map_err(|err| format!("create {} failed: {err}", path.display()))?;
    for record in records {
        let line = serde_json::to_string(record)
            .map_err(|err| format!("serialize record failed: {err}"))?;
        writeln!(file, "{line}")
            .map_err(|err| format!("write {} failed: {err}", path.display()))?;
    }
    Ok(())
}

fn write_json(path: &Path, summary: &SampleSummary) -> Result<(), String> {
    let content = serde_json::to_string_pretty(summary)
        .map_err(|err| format!("serialize summary failed: {err}"))?;
    fs::write(path, format!("{content}\n"))
        .map_err(|err| format!("write {} failed: {err}", path.display()))
}

fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();

    while let Some(arg) = iter.next() {
        match arg {
            "--provider" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--provider requires a value".to_string())?;
                options.provider = BenchProviderKind::parse(raw)
                    .ok_or_else(|| format!("invalid --provider: {raw}"))?;
            }
            "--scenario" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--scenario requires a value".to_string())?;
                options.scenario = WorldScenario::parse(raw)
                    .ok_or_else(|| format!("invalid --scenario: {raw}"))?;
            }
            "--scenario-id" => {
                options.scenario_id = iter
                    .next()
                    .ok_or_else(|| "--scenario-id requires a value".to_string())?
                    .to_string();
            }
            "--parity-tier" => {
                options.parity_tier = iter
                    .next()
                    .ok_or_else(|| "--parity-tier requires a value".to_string())?
                    .to_string();
            }
            "--benchmark-run-id" => {
                options.benchmark_run_id = iter
                    .next()
                    .ok_or_else(|| "--benchmark-run-id requires a value".to_string())?
                    .to_string();
            }
            "--fixture-id" => {
                options.fixture_id = Some(
                    iter.next()
                        .ok_or_else(|| "--fixture-id requires a value".to_string())?
                        .to_string(),
                );
            }
            "--protocol-version" => {
                options.protocol_version = iter
                    .next()
                    .ok_or_else(|| "--protocol-version requires a value".to_string())?
                    .to_string();
            }
            "--adapter-version" => {
                options.adapter_version = iter
                    .next()
                    .ok_or_else(|| "--adapter-version requires a value".to_string())?
                    .to_string();
            }
            "--ticks" => {
                options.ticks = parse_u64(
                    iter.next()
                        .ok_or_else(|| "--ticks requires a value".to_string())?,
                    "--ticks",
                )?;
            }
            "--timeout-ms" => {
                options.timeout_ms = parse_u64(
                    iter.next()
                        .ok_or_else(|| "--timeout-ms requires a value".to_string())?,
                    "--timeout-ms",
                )?;
            }
            "--out-dir" => {
                options.out_dir = PathBuf::from(
                    iter.next()
                        .ok_or_else(|| "--out-dir requires a value".to_string())?,
                );
            }
            "--agent-provider-url" => {
                options.provider_base_url = Some(
                    iter.next()
                        .ok_or_else(|| "--agent-provider-url requires a value".to_string())?
                        .to_string(),
                );
            }
            "--agent-provider-auth-token" => {
                options.provider_auth_token = Some(
                    iter.next()
                        .ok_or_else(|| "--agent-provider-auth-token requires a value".to_string())?
                        .to_string(),
                );
            }
            "--agent-provider-connect-timeout-ms" => {
                options.agent_provider_connect_timeout_ms = parse_u64(
                    iter.next().ok_or_else(|| {
                        "--agent-provider-connect-timeout-ms requires a value".to_string()
                    })?,
                    "--agent-provider-connect-timeout-ms",
                )?;
            }
            "--agent-provider-profile" => {
                options.agent_provider_profile = iter
                    .next()
                    .ok_or_else(|| "--agent-provider-profile requires a value".to_string())?
                    .to_string();
            }
            "--execution-mode" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--execution-mode requires a value".to_string())?;
                options.execution_mode = ProviderExecutionMode::parse(raw).ok_or_else(|| {
                    format!(
                        "invalid --execution-mode `{raw}`: expected player_parity or headless_agent"
                    )
                })?;
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }

    if options.scenario_id.trim().is_empty() {
        return Err("--scenario-id cannot be empty".to_string());
    }
    if options.parity_tier.trim().is_empty() {
        return Err("--parity-tier cannot be empty".to_string());
    }
    if options.benchmark_run_id.trim().is_empty() {
        return Err("--benchmark-run-id cannot be empty".to_string());
    }
    if options.out_dir.as_os_str().is_empty() {
        return Err("--out-dir cannot be empty".to_string());
    }
    if options.provider == BenchProviderKind::ProviderLoopbackHttp
        && options
            .provider_base_url
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err("--agent-provider-url is required for provider_loopback_http".to_string());
    }
    if options.provider == BenchProviderKind::ProviderLoopbackHttp
        && options.agent_provider_profile.trim().is_empty()
    {
        return Err("--agent-provider-profile cannot be empty".to_string());
    }
    if options.provider == BenchProviderKind::Builtin
        && options.execution_mode != ProviderExecutionMode::HeadlessAgent
    {
        return Err(
            "--execution-mode=player_parity is only supported with --provider provider_loopback_http"
                .to_string(),
        );
    }
    Ok(options)
}

fn parse_u64(raw: &str, flag: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .map_err(|err| format!("invalid {flag}: {err}"))
}

fn print_help() {
    println!(
        "Usage: oasis7_provider_parity_bench [options]\n\n\
Run one parity benchmark sample for builtin or the loopback provider and emit\n\
raw jsonl + single-sample summary json following the parity benchmark contract.\n\n\
Options:\n\
  --provider <builtin|provider_loopback_http>\n\
                               supports provider_loopback_http and provider_local_bridge\n\
  --scenario <name>\n\
  --scenario-id <id>\n\
  --parity-tier <P0|P1|P2>\n\
  --benchmark-run-id <id>\n\
  --fixture-id <id>\n\
  --ticks <n>\n\
  --timeout-ms <n>\n\
  --out-dir <path>\n\
  --agent-provider-url <url>\n\
  --agent-provider-auth-token <token>\n\
  --agent-provider-connect-timeout-ms <n>\n\
  --agent-provider-profile <id>\n\
  --execution-mode <player_parity|headless_agent>\n\
  --protocol-version <str>\n\
  --adapter-version <str>\n\
  -h, --help\n"
    );
}

#[cfg(test)]
#[path = "oasis7_provider_parity_bench/tests.rs"]
mod tests;
