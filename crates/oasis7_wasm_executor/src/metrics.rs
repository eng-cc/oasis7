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
const MODULE_HOTSPOT_LIMIT: usize = 10;
const MODULE_TIMING_MAX_TRACKED: usize = 256;
const MODULE_TIMING_OVERFLOW_KEY: &str = "__overflow__";
const MODULE_HOTSPOT_ID_MAX_BYTES: usize = 120;

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
    #[serde(default)]
    pub module_hotspots: Vec<WasmExecutorModuleHotspot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmExecutorModuleHotspot {
    pub module_id: String,
    pub calls_total: u64,
    pub wall_ms_total: u64,
    pub failure_count: u64,
    pub share_ppm: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WasmExecutorModuleTiming {
    calls_total: u64,
    wall_ms_total: u64,
    failure_count: u64,
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
            module_hotspots: Vec::new(),
        }
    }

    fn observe_call_bucket(&mut self, elapsed_ms: u64) {
        for (upper_bound_ms, label) in CALL_WALL_BUCKETS {
            if elapsed_ms <= *upper_bound_ms {
                if let Some(bucket) = self.call_wall_ms_buckets.get_mut(*label) {
                    *bucket = bucket.saturating_add(1);
                } else {
                    self.call_wall_ms_buckets.insert((*label).to_string(), 1);
                }
                return;
            }
        }
        if let Some(bucket) = self.call_wall_ms_buckets.get_mut(CALL_WALL_OVERFLOW_BUCKET) {
            *bucket = bucket.saturating_add(1);
        } else {
            self.call_wall_ms_buckets
                .insert(CALL_WALL_OVERFLOW_BUCKET.to_string(), 1);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExecutorMetricsState {
    snapshot: WasmExecutorMetricsSnapshot,
    module_timings: BTreeMap<String, WasmExecutorModuleTiming>,
}

impl WasmExecutorMetricsState {
    fn empty() -> Self {
        Self {
            snapshot: WasmExecutorMetricsSnapshot::empty(),
            module_timings: BTreeMap::new(),
        }
    }

    fn observe_module_call(&mut self, module_id: &str, total_call_ms: u64, failed: bool) {
        let key = self.module_timing_key(module_id);
        let timing = self.module_timings.entry(key).or_default();
        timing.calls_total = timing.calls_total.saturating_add(1);
        timing.wall_ms_total = timing.wall_ms_total.saturating_add(total_call_ms);
        if failed {
            timing.failure_count = timing.failure_count.saturating_add(1);
        }
        self.refresh_module_hotspots();
    }

    fn module_timing_key(&self, module_id: &str) -> String {
        let trimmed = module_id.trim();
        let normalized = if trimmed.is_empty() {
            "(unknown)"
        } else {
            trimmed
        };
        if self.module_timings.contains_key(normalized)
            || self.module_timings.len() < MODULE_TIMING_MAX_TRACKED
        {
            normalized.to_string()
        } else {
            MODULE_TIMING_OVERFLOW_KEY.to_string()
        }
    }

    fn refresh_module_hotspots(&mut self) {
        let total_wall_ms = self
            .module_timings
            .values()
            .map(|timing| timing.wall_ms_total)
            .sum::<u64>();
        let mut hotspots = self
            .module_timings
            .iter()
            .filter(|(_, timing)| timing.calls_total > 0)
            .map(|(module_id, timing)| WasmExecutorModuleHotspot {
                module_id: bounded_module_hotspot_id(module_id),
                calls_total: timing.calls_total,
                wall_ms_total: timing.wall_ms_total,
                failure_count: timing.failure_count,
                share_ppm: if total_wall_ms == 0 {
                    0
                } else {
                    ((timing.wall_ms_total as u128 * 1_000_000_u128) / total_wall_ms as u128)
                        .try_into()
                        .unwrap_or(u64::MAX)
                },
            })
            .collect::<Vec<_>>();
        hotspots.sort_by(|left, right| {
            right
                .wall_ms_total
                .cmp(&left.wall_ms_total)
                .then_with(|| right.calls_total.cmp(&left.calls_total))
                .then_with(|| left.module_id.cmp(&right.module_id))
        });
        hotspots.truncate(MODULE_HOTSPOT_LIMIT);
        self.snapshot.module_hotspots = hotspots;
    }
}

fn bounded_module_hotspot_id(module_id: &str) -> String {
    if module_id.len() <= MODULE_HOTSPOT_ID_MAX_BYTES {
        return module_id.to_string();
    }

    let suffix = "...";
    let max_prefix_bytes = MODULE_HOTSPOT_ID_MAX_BYTES.saturating_sub(suffix.len());
    let mut end = 0;
    for (index, ch) in module_id.char_indices() {
        let next = index + ch.len_utf8();
        if next > max_prefix_bytes {
            break;
        }
        end = next;
    }
    format!("{}{}", &module_id[..end], suffix)
}

pub type SharedWasmExecutorMetrics = Arc<Mutex<WasmExecutorMetricsState>>;

pub fn init_shared_wasm_executor_metrics() -> SharedWasmExecutorMetrics {
    Arc::new(Mutex::new(WasmExecutorMetricsState::empty()))
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
        Ok(locked) => locked.snapshot.clone(),
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

#[cfg(feature = "wasmtime")]
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
            locked.snapshot.memory_cache_hits = locked.snapshot.memory_cache_hits.saturating_add(1);
        }
        CompileCachePathKind::DiskHit => {
            locked.snapshot.disk_cache_hits = locked.snapshot.disk_cache_hits.saturating_add(1);
        }
        CompileCachePathKind::CompileMiss => {
            locked.snapshot.compile_misses = locked.snapshot.compile_misses.saturating_add(1);
        }
    }
    locked.snapshot.compile_ms_total = locked.snapshot.compile_ms_total.saturating_add(compile_ms);
    locked.snapshot.deserialize_ms_total = locked
        .snapshot
        .deserialize_ms_total
        .saturating_add(deserialize_ms);
}

#[cfg(feature = "wasmtime")]
pub fn observe_wasm_executor_instantiate(metrics: &SharedWasmExecutorMetrics, instantiate_ms: u64) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.snapshot.instantiate_ms_total = locked
        .snapshot
        .instantiate_ms_total
        .saturating_add(instantiate_ms);
}

#[cfg(feature = "wasmtime")]
pub fn observe_wasm_executor_entrypoint_call(
    metrics: &SharedWasmExecutorMetrics,
    entrypoint_call_ms: u64,
) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.snapshot.entrypoint_call_ms_total = locked
        .snapshot
        .entrypoint_call_ms_total
        .saturating_add(entrypoint_call_ms);
}

#[cfg(feature = "wasmtime")]
pub fn observe_wasm_executor_decode(metrics: &SharedWasmExecutorMetrics, decode_ms: u64) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.snapshot.decode_ms_total = locked.snapshot.decode_ms_total.saturating_add(decode_ms);
}

pub fn observe_wasm_executor_call_result(
    metrics: &SharedWasmExecutorMetrics,
    module_id: &str,
    total_call_ms: u64,
    code: Option<ModuleCallErrorCode>,
) {
    let Ok(mut locked) = metrics.lock() else {
        return;
    };
    locked.snapshot.calls_total = locked.snapshot.calls_total.saturating_add(1);
    locked.snapshot.observe_call_bucket(total_call_ms);
    locked.observe_module_call(module_id, total_call_ms, code.is_some());
    if let Some(code) = code {
        *locked
            .snapshot
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
