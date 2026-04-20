use oasis7_wasm_abi::ModuleOutput;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use tools_wasm_build_suite::BuildTimingSnapshot;

#[derive(Debug, Clone, Serialize)]
pub struct ObserveSummary {
    pub schema_version: u32,
    pub generated_at_unix_ms: i64,
    pub spec_path: String,
    pub module_id: String,
    pub manifest_path: String,
    pub packaged_wasm_path: String,
    pub build_metadata_path: String,
    pub build_receipt_path: String,
    pub wasm_hash_sha256: String,
    pub build_timing: BuildTimingSnapshot,
    pub case_results: Vec<CaseResultSummary>,
    pub router_probe_results: Vec<RouterProbeResultSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResultSummary {
    pub name: String,
    pub repeat: u32,
    pub request_entrypoint: String,
    pub perf: PerfStats,
    pub executor_delta: ExecutorMetricsDelta,
    pub router_delta: RouterMetricsDelta,
    pub actual: CaseActualSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouterProbeResultSummary {
    pub name: String,
    pub repeat: u32,
    pub use_prepared: bool,
    pub matched: bool,
    pub perf: PerfStats,
    pub router_delta: RouterMetricsDelta,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerfStats {
    pub runs: usize,
    pub total_wall_ms: u64,
    pub min_wall_ms: u64,
    pub avg_wall_ms: u64,
    pub p50_wall_ms: u64,
    pub p95_wall_ms: u64,
    pub max_wall_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseActualSummary {
    pub success: bool,
    pub failure_code: Option<String>,
    pub failure_detail: Option<String>,
    pub emit_count: usize,
    pub effect_count: usize,
    pub tick_lifecycle: Option<JsonValue>,
    pub new_state_json: Option<JsonValue>,
    pub emits: Vec<ActualEmitSummary>,
    pub last_output: Option<ModuleOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActualEmitSummary {
    pub kind: String,
    pub payload_json: JsonValue,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutorMetricsDelta {
    pub calls_total: u64,
    pub memory_cache_hits: u64,
    pub disk_cache_hits: u64,
    pub compile_misses: u64,
    pub compile_ms_total: u64,
    pub deserialize_ms_total: u64,
    pub instantiate_ms_total: u64,
    pub entrypoint_call_ms_total: u64,
    pub decode_ms_total: u64,
    pub failure_by_code: BTreeMap<String, u64>,
    pub call_wall_ms_buckets: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouterMetricsDelta {
    pub prepare_calls_total: u64,
    pub prepare_ms_total: u64,
    pub match_calls_total: u64,
    pub match_ms_total: u64,
    pub parse_fallbacks: u64,
    pub prepared_hits: u64,
    pub regex_compile_ms_total: u64,
    pub prepare_ms_buckets: BTreeMap<String, u64>,
    pub match_ms_buckets: BTreeMap<String, u64>,
}

impl PerfStats {
    pub fn from_samples(samples: &[u64]) -> Self {
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let total_wall_ms = sorted.iter().copied().sum::<u64>();
        let runs = sorted.len();
        let min_wall_ms = *sorted.first().unwrap_or(&0);
        let max_wall_ms = *sorted.last().unwrap_or(&0);
        let avg_wall_ms = if runs == 0 {
            0
        } else {
            total_wall_ms / u64::try_from(runs).unwrap_or(1)
        };
        let p50_wall_ms = percentile(&sorted, 50);
        let p95_wall_ms = percentile(&sorted, 95);
        Self {
            runs,
            total_wall_ms,
            min_wall_ms,
            avg_wall_ms,
            p50_wall_ms,
            p95_wall_ms,
            max_wall_ms,
        }
    }
}

pub fn render_markdown(summary: &ObserveSummary) -> String {
    let mut lines = vec![
        "# Oasis7 WASM Module Observe Summary".to_string(),
        String::new(),
        format!("- Module: `{}`", summary.module_id),
        format!("- Spec: `{}`", summary.spec_path),
        format!("- Manifest: `{}`", summary.manifest_path),
        format!("- Wasm hash: `{}`", summary.wasm_hash_sha256),
        format!(
            "- Build timing: `total={}ms cargo={}ms canonicalize={}ms hash={}ms receipt={}ms metadata={}ms`",
            summary.build_timing.total_build_wall_ms,
            summary.build_timing.cargo_build_ms,
            summary.build_timing.canonicalize_ms,
            summary.build_timing.hash_ms,
            summary.build_timing.receipt_write_ms,
            summary.build_timing.metadata_write_ms
        ),
        String::new(),
        "## Cases".to_string(),
    ];

    for case in &summary.case_results {
        lines.push(format!(
            "- `{}`: success=`{}` repeat=`{}` perf=`avg={}ms p95={}ms max={}ms` emits=`{}` effects=`{}` compile_miss_delta=`{}` memory_hit_delta=`{}`",
            case.name,
            case.actual.success,
            case.repeat,
            case.perf.avg_wall_ms,
            case.perf.p95_wall_ms,
            case.perf.max_wall_ms,
            case.actual.emit_count,
            case.actual.effect_count,
            case.executor_delta.compile_misses,
            case.executor_delta.memory_cache_hits
        ));
        if let Some(code) = &case.actual.failure_code {
            lines.push(format!("  failure_code=`{code}`"));
        }
    }

    lines.push(String::new());
    lines.push("## Router Probes".to_string());
    if summary.router_probe_results.is_empty() {
        lines.push("- none".to_string());
    } else {
        for probe in &summary.router_probe_results {
            lines.push(format!(
                "- `{}`: matched=`{}` prepared=`{}` repeat=`{}` perf=`avg={}ms p95={}ms` prepared_hits_delta=`{}` parse_fallbacks_delta=`{}`",
                probe.name,
                probe.matched,
                probe.use_prepared,
                probe.repeat,
                probe.perf.avg_wall_ms,
                probe.perf.p95_wall_ms,
                probe.router_delta.prepared_hits,
                probe.router_delta.parse_fallbacks
            ));
        }
    }

    lines.join("\n") + "\n"
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let last_index = sorted.len().saturating_sub(1);
    let scaled = last_index.saturating_mul(p);
    let index = (scaled + 99) / 100;
    sorted[index.min(last_index)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_stats_calculates_percentiles() {
        let stats = PerfStats::from_samples(&[10, 20, 30, 40, 50]);
        assert_eq!(stats.runs, 5);
        assert_eq!(stats.total_wall_ms, 150);
        assert_eq!(stats.avg_wall_ms, 30);
        assert_eq!(stats.p50_wall_ms, 30);
        assert_eq!(stats.p95_wall_ms, 50);
    }
}
