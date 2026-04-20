use oasis7_wasm_abi::ModuleCallErrorCode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CALL_WALL_BUCKETS: &[(u64, &str)] = &[
    (1, "le_0001_ms"),
    (5, "le_0005_ms"),
    (10, "le_0010_ms"),
    (25, "le_0025_ms"),
    (50, "le_0050_ms"),
    (100, "le_0100_ms"),
    (250, "le_0250_ms"),
    (500, "le_0500_ms"),
    (1000, "le_1000_ms"),
];
const CALL_WALL_OVERFLOW_BUCKET: &str = "gt_1000_ms";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileCachePathKind {
    MemoryHit,
    DiskHit,
    CompileMiss,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmExecutorMetricsSnapshot {
    pub observed_since_unix_ms: i64,
    pub metrics_available: bool,
    pub degraded_reason: Option<String>,
    pub calls_total: u64,
    pub memory_cache_hits: u64,
    pub disk_cache_hits: u64,
    pub compile_misses: u64,
    pub failure_by_code: BTreeMap<String, u64>,
    pub compile_ms_total: u64,
    pub deserialize_ms_total: u64,
    pub instantiate_ms_total: u64,
    pub entrypoint_call_ms_total: u64,
    pub decode_ms_total: u64,
    pub call_wall_ms_buckets: BTreeMap<String, u64>,
}

impl WasmExecutorMetricsSnapshot {
    pub fn empty() -> Self {
        let mut call_wall_ms_buckets = BTreeMap::new();
        for (_, label) in CALL_WALL_BUCKETS {
            call_wall_ms_buckets.insert((*label).to_string(), 0);
        }
        call_wall_ms_buckets.insert(CALL_WALL_OVERFLOW_BUCKET.to_string(), 0);
        Self {
            observed_since_unix_ms: now_unix_ms(),
            metrics_available: true,
            degraded_reason: None,
            calls_total: 0,
            memory_cache_hits: 0,
            disk_cache_hits: 0,
            compile_misses: 0,
            failure_by_code: BTreeMap::new(),
            compile_ms_total: 0,
            deserialize_ms_total: 0,
            instantiate_ms_total: 0,
            entrypoint_call_ms_total: 0,
            decode_ms_total: 0,
            call_wall_ms_buckets,
        }
    }

    fn observe_call_bucket(&mut self, elapsed_ms: u64) {
        for (upper_bound_ms, label) in CALL_WALL_BUCKETS {
            if elapsed_ms <= *upper_bound_ms {
                *self
                    .call_wall_ms_buckets
                    .entry((*label).to_string())
                    .or_insert(0) += 1;
                return;
            }
        }
        *self
            .call_wall_ms_buckets
            .entry(CALL_WALL_OVERFLOW_BUCKET.to_string())
            .or_insert(0) += 1;
    }
}

pub type SharedWasmExecutorMetrics = Arc<Mutex<WasmExecutorMetricsSnapshot>>;

pub fn init_shared_wasm_executor_metrics() -> SharedWasmExecutorMetrics {
    Arc::new(Mutex::new(WasmExecutorMetricsSnapshot::empty()))
}

pub fn global_wasm_executor_metrics() -> SharedWasmExecutorMetrics {
    static GLOBAL: OnceLock<SharedWasmExecutorMetrics> = OnceLock::new();
    GLOBAL
        .get_or_init(init_shared_wasm_executor_metrics)
        .clone()
}

pub fn snapshot_wasm_executor_metrics(
    metrics: &SharedWasmExecutorMetrics,
) -> WasmExecutorMetricsSnapshot {
    match metrics.lock() {
        Ok(locked) => locked.clone(),
        Err(_) => WasmExecutorMetricsSnapshot {
            metrics_available: false,
            degraded_reason: Some("wasm executor metrics lock poisoned".to_string()),
            ..WasmExecutorMetricsSnapshot::empty()
        },
    }
}

pub fn snapshot_global_wasm_executor_metrics() -> WasmExecutorMetricsSnapshot {
    let shared = global_wasm_executor_metrics();
    snapshot_wasm_executor_metrics(&shared)
}

pub fn observe_wasm_executor_compile(
    metrics: &SharedWasmExecutorMetrics,
    cache_path: CompileCachePathKind,
    compile_ms: u64,
    deserialize_ms: u64,
) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    match cache_path {
        CompileCachePathKind::MemoryHit => {
            locked.memory_cache_hits = locked.memory_cache_hits.saturating_add(1);
        }
        CompileCachePathKind::DiskHit => {
            locked.disk_cache_hits = locked.disk_cache_hits.saturating_add(1);
        }
        CompileCachePathKind::CompileMiss => {
            locked.compile_misses = locked.compile_misses.saturating_add(1);
        }
    }
    locked.compile_ms_total = locked.compile_ms_total.saturating_add(compile_ms);
    locked.deserialize_ms_total = locked.deserialize_ms_total.saturating_add(deserialize_ms);
}

pub fn observe_wasm_executor_instantiate(metrics: &SharedWasmExecutorMetrics, instantiate_ms: u64) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.instantiate_ms_total = locked.instantiate_ms_total.saturating_add(instantiate_ms);
}

pub fn observe_wasm_executor_entrypoint_call(
    metrics: &SharedWasmExecutorMetrics,
    entrypoint_call_ms: u64,
) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.entrypoint_call_ms_total = locked
        .entrypoint_call_ms_total
        .saturating_add(entrypoint_call_ms);
}

pub fn observe_wasm_executor_decode(metrics: &SharedWasmExecutorMetrics, decode_ms: u64) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.decode_ms_total = locked.decode_ms_total.saturating_add(decode_ms);
}

pub fn observe_wasm_executor_call_result(
    metrics: &SharedWasmExecutorMetrics,
    total_call_ms: u64,
    code: Option<ModuleCallErrorCode>,
) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.calls_total = locked.calls_total.saturating_add(1);
    locked.observe_call_bucket(total_call_ms);
    if let Some(code) = code {
        *locked
            .failure_by_code
            .entry(module_call_error_code_label(code).to_string())
            .or_insert(0) += 1;
    }
}

fn module_call_error_code_label(code: ModuleCallErrorCode) -> &'static str {
    match code {
        ModuleCallErrorCode::SandboxUnavailable => "sandbox_unavailable",
        ModuleCallErrorCode::Trap => "trap",
        ModuleCallErrorCode::Interrupted => "interrupted",
        ModuleCallErrorCode::Timeout => "timeout",
        ModuleCallErrorCode::OutOfFuel => "out_of_fuel",
        ModuleCallErrorCode::OutputTooLarge => "output_too_large",
        ModuleCallErrorCode::EffectLimitExceeded => "effect_limit_exceeded",
        ModuleCallErrorCode::EmitLimitExceeded => "emit_limit_exceeded",
        ModuleCallErrorCode::InvalidOutput => "invalid_output",
        ModuleCallErrorCode::CapsDenied => "caps_denied",
        ModuleCallErrorCode::PolicyDenied => "policy_denied",
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(i64::MAX))
        .unwrap_or(0)
}
