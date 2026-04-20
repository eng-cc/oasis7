mod report;
mod spec;

pub use report::{
    render_markdown, CaseActualSummary, CaseResultSummary, ExecutorMetricsDelta, ObserveSummary,
    PerfStats, RouterMetricsDelta, RouterProbeResultSummary,
};
pub use spec::{
    load_spec, CaseExpectationSpec, CaseRequestSpec, ExpectedEmitSpec, ModuleObserveSpec,
    ObserveCaseSpec, ResolvedModuleObserveSpec, RouterProbeInputSpec, RouterProbeSpec,
    TickLifecycleExpectation,
};

use oasis7_wasm_abi::{
    ModuleCallFailure, ModuleCallInput, ModuleCallOrigin, ModuleCallRequest, ModuleContext,
    ModuleOutput, ModuleSandbox, ModuleTickLifecycleDirective,
};
use oasis7_wasm_executor::{
    init_shared_wasm_executor_metrics, snapshot_wasm_executor_metrics, WasmExecutor,
    WasmExecutorConfig, WasmExecutorMetricsSnapshot,
};
use oasis7_wasm_router::{
    module_subscribes_to_action, module_subscribes_to_event, prepare_subscriptions,
    prepared_module_subscribes_to_action, prepared_module_subscribes_to_event,
    snapshot_global_wasm_router_metrics, WasmRouterMetricsSnapshot,
};
use report::ActualEmitSummary;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tools_wasm_build_suite::{run_build, BuildRequest};

#[derive(Debug, Clone)]
pub struct ObserveRunRequest {
    pub spec_path: PathBuf,
    pub out_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ObserveRunOutput {
    pub out_dir: PathBuf,
    pub summary_json_path: PathBuf,
    pub summary_md_path: PathBuf,
    pub summary: ObserveSummary,
}

pub fn run_observe(request: &ObserveRunRequest) -> Result<ObserveRunOutput, String> {
    let spec = load_spec(&request.spec_path)?;
    let out_dir = request.out_dir.clone().unwrap_or_else(|| {
        Path::new(".tmp")
            .join("wasm_module_observe")
            .join(spec.module.module_id.replace('.', "_"))
    });
    fs::create_dir_all(&out_dir).map_err(|err| {
        format!(
            "create output directory {} failed: {err}",
            out_dir.display()
        )
    })?;
    let build_out_dir = out_dir.join("build");
    fs::create_dir_all(&build_out_dir).map_err(|err| {
        format!(
            "create build output directory {} failed: {err}",
            build_out_dir.display()
        )
    })?;

    let build_output = run_build(&BuildRequest {
        module_id: spec.module.module_id.clone(),
        manifest_path: spec.module.manifest_path.clone(),
        out_dir: build_out_dir.clone(),
        target: spec.module.target.clone(),
        profile: spec.module.profile.clone(),
        dry_run: false,
    })
    .map_err(|err| format!("build module {} failed: {err}", spec.module.module_id))?;

    let wasm_hash_sha256 = build_output
        .wasm_hash_sha256
        .clone()
        .ok_or_else(|| "build output missing wasm hash".to_string())?;
    let build_timing = build_output
        .build_timing
        .clone()
        .ok_or_else(|| "build output missing build timing".to_string())?;
    let wasm_bytes = fs::read(&build_output.packaged_wasm_path).map_err(|err| {
        format!(
            "read packaged wasm {} failed: {err}",
            build_output.packaged_wasm_path.display()
        )
    })?;

    let metrics = init_shared_wasm_executor_metrics();
    let cache_dir = out_dir.join("compiled-cache");
    let mut executor = WasmExecutor::new_with_metrics(
        WasmExecutorConfig {
            compiled_cache_dir: Some(cache_dir),
            ..WasmExecutorConfig::default()
        },
        metrics.clone(),
    )
    .map_err(|err| format!("initialize wasm executor failed: {err}"))?;

    let case_results = spec
        .cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            run_case(
                &spec,
                &mut executor,
                &metrics,
                &build_output.packaged_wasm_path,
                wasm_hash_sha256.as_str(),
                Arc::<[u8]>::from(wasm_bytes.clone()),
                case,
                index,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let router_probe_results = spec
        .router_probes
        .iter()
        .map(|probe| run_router_probe(&spec, probe))
        .collect::<Result<Vec<_>, _>>()?;

    let summary = ObserveSummary {
        schema_version: spec.schema_version,
        generated_at_unix_ms: now_unix_ms(),
        spec_path: spec.spec_path.to_string_lossy().to_string(),
        module_id: spec.module.module_id.clone(),
        manifest_path: spec.module.manifest_path.to_string_lossy().to_string(),
        packaged_wasm_path: build_output
            .packaged_wasm_path
            .to_string_lossy()
            .to_string(),
        build_metadata_path: build_output.metadata_path.to_string_lossy().to_string(),
        build_receipt_path: build_output.receipt_path.to_string_lossy().to_string(),
        wasm_hash_sha256,
        build_timing,
        case_results,
        router_probe_results,
    };

    let summary_json_path = out_dir.join("summary.json");
    let summary_md_path = out_dir.join("summary.md");
    fs::write(
        &summary_json_path,
        serde_json::to_string_pretty(&summary)
            .map_err(|err| format!("serialize observe summary failed: {err}"))?
            + "\n",
    )
    .map_err(|err| {
        format!(
            "write summary json {} failed: {err}",
            summary_json_path.display()
        )
    })?;
    fs::write(&summary_md_path, render_markdown(&summary)).map_err(|err| {
        format!(
            "write summary markdown {} failed: {err}",
            summary_md_path.display()
        )
    })?;

    Ok(ObserveRunOutput {
        out_dir,
        summary_json_path,
        summary_md_path,
        summary,
    })
}

fn run_case(
    spec: &ResolvedModuleObserveSpec,
    executor: &mut WasmExecutor,
    metrics: &oasis7_wasm_executor::SharedWasmExecutorMetrics,
    packaged_wasm_path: &Path,
    wasm_hash_sha256: &str,
    wasm_bytes: Arc<[u8]>,
    case: &ObserveCaseSpec,
    index: usize,
) -> Result<CaseResultSummary, String> {
    let executor_before = snapshot_wasm_executor_metrics(metrics);
    let router_before = snapshot_global_wasm_router_metrics();
    let mut last_output: Option<ModuleOutput> = None;
    let mut last_failure: Option<ModuleCallFailure> = None;
    let mut samples = Vec::new();

    for run_index in 0..case.repeat {
        let request = build_case_request(
            spec,
            packaged_wasm_path,
            wasm_hash_sha256,
            wasm_bytes.clone(),
            case,
            index,
            run_index,
        )?;
        let started = Instant::now();
        let result = executor.call(&request);
        samples.push(elapsed_ms(started));
        match result {
            Ok(output) => {
                last_failure = None;
                last_output = Some(output);
            }
            Err(failure) => {
                last_output = None;
                last_failure = Some(failure);
            }
        }
    }

    validate_case(case, &last_output, &last_failure)?;
    let executor_after = snapshot_wasm_executor_metrics(metrics);
    let router_after = snapshot_global_wasm_router_metrics();
    let perf = PerfStats::from_samples(&samples);
    let actual = summarize_case_actual(last_output, last_failure)?;

    Ok(CaseResultSummary {
        name: case.name.clone(),
        repeat: case.repeat,
        request_entrypoint: case
            .request
            .entrypoint
            .clone()
            .unwrap_or_else(|| spec.module.entrypoint.clone()),
        perf,
        executor_delta: diff_executor_metrics(&executor_before, &executor_after),
        router_delta: diff_router_metrics(&router_before, &router_after),
        actual,
    })
}

fn run_router_probe(
    spec: &ResolvedModuleObserveSpec,
    probe: &RouterProbeSpec,
) -> Result<RouterProbeResultSummary, String> {
    let prepared = if probe.use_prepared {
        Some(
            prepare_subscriptions(&spec.subscriptions, &spec.module.module_id)
                .map_err(|err| format!("prepare router subscriptions failed: {err}"))?,
        )
    } else {
        None
    };
    let router_before = snapshot_global_wasm_router_metrics();
    let mut samples = Vec::new();
    let mut matched = false;
    for _ in 0..probe.repeat {
        let started = Instant::now();
        matched = match &probe.probe {
            RouterProbeInputSpec::Event {
                event_kind,
                payload_json,
            } => {
                if let Some(prepared) = prepared.as_ref() {
                    prepared_module_subscribes_to_event(prepared, event_kind, payload_json)
                } else {
                    module_subscribes_to_event(&spec.subscriptions, event_kind, payload_json)
                }
            }
            RouterProbeInputSpec::Action {
                stage,
                action_kind,
                payload_json,
            } => {
                if let Some(prepared) = prepared.as_ref() {
                    prepared_module_subscribes_to_action(
                        prepared,
                        *stage,
                        action_kind,
                        payload_json,
                    )
                } else {
                    module_subscribes_to_action(
                        &spec.subscriptions,
                        *stage,
                        action_kind,
                        payload_json,
                    )
                }
            }
        };
        samples.push(elapsed_ms(started));
    }
    if matched != probe.expect_match {
        return Err(format!(
            "router probe {} expected match={} got={matched}",
            probe.name, probe.expect_match
        ));
    }
    let router_after = snapshot_global_wasm_router_metrics();
    Ok(RouterProbeResultSummary {
        name: probe.name.clone(),
        repeat: probe.repeat,
        use_prepared: probe.use_prepared,
        matched,
        perf: PerfStats::from_samples(&samples),
        router_delta: diff_router_metrics(&router_before, &router_after),
    })
}

fn build_case_request(
    spec: &ResolvedModuleObserveSpec,
    _packaged_wasm_path: &Path,
    wasm_hash_sha256: &str,
    wasm_bytes: Arc<[u8]>,
    case: &ObserveCaseSpec,
    case_index: usize,
    run_index: u32,
) -> Result<ModuleCallRequest, String> {
    let limits = case
        .request
        .limits
        .clone()
        .unwrap_or_else(|| spec.module.limits.clone());
    let trace_id = case.request.trace_id.clone().unwrap_or_else(|| {
        format!(
            "observe-{}-{}-{}",
            spec.module.module_id,
            case_index + 1,
            run_index + 1
        )
    });
    let input = ModuleCallInput {
        ctx: ModuleContext {
            v: case.request.ctx_version.clone(),
            module_id: spec.module.module_id.clone(),
            trace_id: trace_id.clone(),
            time: case.request.time,
            origin: ModuleCallOrigin {
                kind: case.request.origin_kind.clone(),
                id: case.request.origin_id.clone(),
            },
            limits: limits.clone(),
            stage: case.request.stage.clone(),
            world_config_hash: case.request.world_config_hash.clone(),
            manifest_hash: case.request.manifest_hash.clone(),
            journal_height: case.request.journal_height,
            module_version: case.request.module_version.clone(),
            module_kind: case.request.module_kind.clone(),
            module_role: case.request.module_role.clone(),
        },
        event: encode_optional_json(&case.request.event_json)?,
        action: encode_optional_json(&case.request.action_json)?,
        state: encode_optional_json(&case.request.state_json)?,
    };
    let input_bytes = serde_cbor::to_vec(&input).map_err(|err| {
        format!(
            "encode module call input for case {} failed: {err}",
            case.name
        )
    })?;
    Ok(ModuleCallRequest {
        module_id: spec.module.module_id.clone(),
        wasm_hash: wasm_hash_sha256.to_string(),
        trace_id,
        entrypoint: case
            .request
            .entrypoint
            .clone()
            .unwrap_or_else(|| spec.module.entrypoint.clone()),
        input: input_bytes,
        limits,
        wasm_bytes,
    })
}

fn validate_case(
    case: &ObserveCaseSpec,
    output: &Option<ModuleOutput>,
    failure: &Option<ModuleCallFailure>,
) -> Result<(), String> {
    if case.expect.success {
        if let Some(failure) = failure {
            return Err(format!(
                "case {} expected success but failed with {}: {}",
                case.name,
                module_call_failure_code_label(&failure.code),
                failure.detail
            ));
        }
        let output = output
            .as_ref()
            .ok_or_else(|| format!("case {} missing output after successful run", case.name))?;
        if let Some(expected) = case.expect.emit_count {
            if output.emits.len() != expected {
                return Err(format!(
                    "case {} expected emit_count={} got={}",
                    case.name,
                    expected,
                    output.emits.len()
                ));
            }
        }
        if let Some(expected) = case.expect.effect_count {
            if output.effects.len() != expected {
                return Err(format!(
                    "case {} expected effect_count={} got={}",
                    case.name,
                    expected,
                    output.effects.len()
                ));
            }
        }
        if let Some(expected) = case.expect.new_state_present {
            if output.new_state.is_some() != expected {
                return Err(format!(
                    "case {} expected new_state_present={} got={}",
                    case.name,
                    expected,
                    output.new_state.is_some()
                ));
            }
        }
        if !case.expect.emits.is_empty() {
            if output.emits.len() != case.expect.emits.len() {
                return Err(format!(
                    "case {} expected {} emits but got {}",
                    case.name,
                    case.expect.emits.len(),
                    output.emits.len()
                ));
            }
            for (expected_emit, actual_emit) in case.expect.emits.iter().zip(output.emits.iter()) {
                if expected_emit.kind != actual_emit.kind {
                    return Err(format!(
                        "case {} expected emit kind {} got {}",
                        case.name, expected_emit.kind, actual_emit.kind
                    ));
                }
                if let Some(payload_json) = &expected_emit.payload_json {
                    if actual_emit.payload != *payload_json {
                        return Err(format!(
                            "case {} emit {} payload mismatch expected={} actual={}",
                            case.name, expected_emit.kind, payload_json, actual_emit.payload
                        ));
                    }
                }
            }
        }
        match (&case.expect.tick_lifecycle, &output.tick_lifecycle) {
            (Some(TickLifecycleExpectation::Absent), None) | (None, _) => {}
            (
                Some(TickLifecycleExpectation::Suspend),
                Some(ModuleTickLifecycleDirective::Suspend),
            ) => {}
            (
                Some(TickLifecycleExpectation::WakeAfterTicks { ticks: expected }),
                Some(ModuleTickLifecycleDirective::WakeAfterTicks { ticks }),
            ) if ticks == expected => {}
            (Some(expectation), actual) => {
                return Err(format!(
                    "case {} tick_lifecycle mismatch expected={} actual={}",
                    case.name,
                    tick_expectation_label(expectation),
                    tick_actual_label(actual.as_ref())
                ));
            }
        }
        if let Some(expected_state) = &case.expect.state_json {
            let actual_state =
                decode_optional_cbor_json(output.new_state.as_deref()).map_err(|err| {
                    format!(
                        "case {} failed to decode new_state as json-cbor: {err}",
                        case.name
                    )
                })?;
            if actual_state.as_ref() != Some(expected_state) {
                return Err(format!(
                    "case {} state_json mismatch expected={} actual={}",
                    case.name,
                    expected_state,
                    actual_state
                        .as_ref()
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_string())
                ));
            }
        }
        return Ok(());
    }

    let failure = failure
        .as_ref()
        .ok_or_else(|| format!("case {} expected failure but succeeded", case.name))?;
    if let Some(expected_code) = &case.expect.failure_code {
        let actual_code = module_call_failure_code_label(&failure.code);
        if actual_code != expected_code {
            return Err(format!(
                "case {} expected failure_code={} got={actual_code}",
                case.name, expected_code
            ));
        }
    }
    if let Some(expected_substring) = &case.expect.failure_detail_substring {
        if !failure.detail.contains(expected_substring) {
            return Err(format!(
                "case {} expected failure detail containing {:?} got {:?}",
                case.name, expected_substring, failure.detail
            ));
        }
    }
    Ok(())
}

fn summarize_case_actual(
    output: Option<ModuleOutput>,
    failure: Option<ModuleCallFailure>,
) -> Result<report::CaseActualSummary, String> {
    match (output, failure) {
        (Some(output), None) => {
            let tick_lifecycle = output
                .tick_lifecycle
                .as_ref()
                .map(|value| serde_json::to_value(value).expect("tick lifecycle is serializable"));
            let new_state_json = decode_optional_cbor_json(output.new_state.as_deref())?;
            let emits = output
                .emits
                .iter()
                .map(|emit| ActualEmitSummary {
                    kind: emit.kind.clone(),
                    payload_json: emit.payload.clone(),
                })
                .collect::<Vec<_>>();
            let emit_count = emits.len();
            let effect_count = output.effects.len();
            Ok(report::CaseActualSummary {
                success: true,
                failure_code: None,
                failure_detail: None,
                emit_count,
                effect_count,
                tick_lifecycle,
                new_state_json,
                emits,
                last_output: Some(output),
            })
        }
        (None, Some(failure)) => Ok(report::CaseActualSummary {
            success: false,
            failure_code: Some(module_call_failure_code_label(&failure.code).to_string()),
            failure_detail: Some(failure.detail),
            emit_count: 0,
            effect_count: 0,
            tick_lifecycle: None,
            new_state_json: None,
            emits: Vec::new(),
            last_output: None,
        }),
        _ => Err("case execution ended without output or failure".to_string()),
    }
}

fn encode_optional_json(value: &Option<JsonValue>) -> Result<Option<Vec<u8>>, String> {
    value
        .as_ref()
        .map(|value| serde_cbor::to_vec(value).map_err(|err| err.to_string()))
        .transpose()
}

fn decode_optional_cbor_json(bytes: Option<&[u8]>) -> Result<Option<JsonValue>, String> {
    bytes
        .map(|bytes| serde_cbor::from_slice(bytes).map_err(|err| err.to_string()))
        .transpose()
}

fn diff_executor_metrics(
    before: &WasmExecutorMetricsSnapshot,
    after: &WasmExecutorMetricsSnapshot,
) -> ExecutorMetricsDelta {
    ExecutorMetricsDelta {
        calls_total: after.calls_total.saturating_sub(before.calls_total),
        memory_cache_hits: after
            .memory_cache_hits
            .saturating_sub(before.memory_cache_hits),
        disk_cache_hits: after.disk_cache_hits.saturating_sub(before.disk_cache_hits),
        compile_misses: after.compile_misses.saturating_sub(before.compile_misses),
        compile_ms_total: after
            .compile_ms_total
            .saturating_sub(before.compile_ms_total),
        deserialize_ms_total: after
            .deserialize_ms_total
            .saturating_sub(before.deserialize_ms_total),
        instantiate_ms_total: after
            .instantiate_ms_total
            .saturating_sub(before.instantiate_ms_total),
        entrypoint_call_ms_total: after
            .entrypoint_call_ms_total
            .saturating_sub(before.entrypoint_call_ms_total),
        decode_ms_total: after.decode_ms_total.saturating_sub(before.decode_ms_total),
        failure_by_code: diff_btree_map(&before.failure_by_code, &after.failure_by_code),
        call_wall_ms_buckets: diff_btree_map(
            &before.call_wall_ms_buckets,
            &after.call_wall_ms_buckets,
        ),
    }
}

fn diff_router_metrics(
    before: &WasmRouterMetricsSnapshot,
    after: &WasmRouterMetricsSnapshot,
) -> RouterMetricsDelta {
    RouterMetricsDelta {
        prepare_calls_total: after
            .prepare_calls_total
            .saturating_sub(before.prepare_calls_total),
        prepare_ms_total: after
            .prepare_ms_total
            .saturating_sub(before.prepare_ms_total),
        match_calls_total: after
            .match_calls_total
            .saturating_sub(before.match_calls_total),
        match_ms_total: after.match_ms_total.saturating_sub(before.match_ms_total),
        parse_fallbacks: after.parse_fallbacks.saturating_sub(before.parse_fallbacks),
        prepared_hits: after.prepared_hits.saturating_sub(before.prepared_hits),
        regex_compile_ms_total: after
            .regex_compile_ms_total
            .saturating_sub(before.regex_compile_ms_total),
        prepare_ms_buckets: diff_btree_map(&before.prepare_ms_buckets, &after.prepare_ms_buckets),
        match_ms_buckets: diff_btree_map(&before.match_ms_buckets, &after.match_ms_buckets),
    }
}

fn diff_btree_map(
    before: &BTreeMap<String, u64>,
    after: &BTreeMap<String, u64>,
) -> BTreeMap<String, u64> {
    let mut keys = before.keys().cloned().collect::<Vec<_>>();
    for key in after.keys() {
        if !before.contains_key(key) {
            keys.push(key.clone());
        }
    }
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .map(|key| {
            let before_value = before.get(&key).copied().unwrap_or_default();
            let after_value = after.get(&key).copied().unwrap_or_default();
            (key, after_value.saturating_sub(before_value))
        })
        .collect()
}

fn module_call_failure_code_label(code: &oasis7_wasm_abi::ModuleCallErrorCode) -> &'static str {
    match code {
        oasis7_wasm_abi::ModuleCallErrorCode::Trap => "trap",
        oasis7_wasm_abi::ModuleCallErrorCode::Timeout => "timeout",
        oasis7_wasm_abi::ModuleCallErrorCode::OutOfFuel => "out_of_fuel",
        oasis7_wasm_abi::ModuleCallErrorCode::Interrupted => "interrupted",
        oasis7_wasm_abi::ModuleCallErrorCode::OutputTooLarge => "output_too_large",
        oasis7_wasm_abi::ModuleCallErrorCode::EffectLimitExceeded => "effect_limit_exceeded",
        oasis7_wasm_abi::ModuleCallErrorCode::EmitLimitExceeded => "emit_limit_exceeded",
        oasis7_wasm_abi::ModuleCallErrorCode::CapsDenied => "caps_denied",
        oasis7_wasm_abi::ModuleCallErrorCode::PolicyDenied => "policy_denied",
        oasis7_wasm_abi::ModuleCallErrorCode::SandboxUnavailable => "sandbox_unavailable",
        oasis7_wasm_abi::ModuleCallErrorCode::InvalidOutput => "invalid_output",
    }
}

fn tick_expectation_label(value: &TickLifecycleExpectation) -> String {
    match value {
        TickLifecycleExpectation::WakeAfterTicks { ticks } => format!("wake_after_ticks({ticks})"),
        TickLifecycleExpectation::Suspend => "suspend".to_string(),
        TickLifecycleExpectation::Absent => "absent".to_string(),
    }
}

fn tick_actual_label(value: Option<&ModuleTickLifecycleDirective>) -> String {
    match value {
        Some(ModuleTickLifecycleDirective::WakeAfterTicks { ticks }) => {
            format!("wake_after_ticks({ticks})")
        }
        Some(ModuleTickLifecycleDirective::Suspend) => "suspend".to_string(),
        None => "absent".to_string(),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_btree_map_subtracts_counters() {
        let before = BTreeMap::from([("a".to_string(), 1), ("b".to_string(), 2)]);
        let after = BTreeMap::from([
            ("a".to_string(), 3),
            ("b".to_string(), 2),
            ("c".to_string(), 5),
        ]);
        let diff = diff_btree_map(&before, &after);
        assert_eq!(diff.get("a"), Some(&2));
        assert_eq!(diff.get("b"), Some(&0));
        assert_eq!(diff.get("c"), Some(&5));
    }

    #[test]
    fn encode_and_decode_optional_json_round_trips() {
        let payload = Some(serde_json::json!({"a": 1, "b": [2, 3]}));
        let encoded = encode_optional_json(&payload).expect("encode");
        let decoded = decode_optional_cbor_json(encoded.as_deref()).expect("decode");
        assert_eq!(decoded, payload);
    }
}
