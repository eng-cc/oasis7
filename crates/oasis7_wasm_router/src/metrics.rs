use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const WALL_BUCKETS: &[(u64, &str)] = &[
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
const WALL_OVERFLOW_BUCKET: &str = "gt_1000_ms";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRouterMetricsSnapshot {
    pub observed_since_unix_ms: i64,
    pub metrics_available: bool,
    pub degraded_reason: Option<String>,
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

impl WasmRouterMetricsSnapshot {
    pub fn empty() -> Self {
        Self {
            observed_since_unix_ms: now_unix_ms(),
            metrics_available: true,
            degraded_reason: None,
            prepare_calls_total: 0,
            prepare_ms_total: 0,
            match_calls_total: 0,
            match_ms_total: 0,
            parse_fallbacks: 0,
            prepared_hits: 0,
            regex_compile_ms_total: 0,
            prepare_ms_buckets: empty_bucket_map(),
            match_ms_buckets: empty_bucket_map(),
        }
    }
}

pub type SharedWasmRouterMetrics = Arc<Mutex<WasmRouterMetricsSnapshot>>;

pub fn snapshot_global_wasm_router_metrics() -> WasmRouterMetricsSnapshot {
    let shared = global_wasm_router_metrics();
    let snapshot = match shared.lock() {
        Ok(locked) => locked.clone(),
        Err(_) => WasmRouterMetricsSnapshot {
            metrics_available: false,
            degraded_reason: Some("wasm router metrics lock poisoned".to_string()),
            ..WasmRouterMetricsSnapshot::empty()
        },
    };
    snapshot
}

pub fn observe_wasm_router_prepare(prepare_ms: u64) {
    let shared = global_wasm_router_metrics();
    let Ok(mut locked) = shared.lock() else {
        return;
    };
    locked.prepare_calls_total = locked.prepare_calls_total.saturating_add(1);
    locked.prepare_ms_total = locked.prepare_ms_total.saturating_add(prepare_ms);
    observe_bucket(&mut locked.prepare_ms_buckets, prepare_ms);
}

pub fn observe_wasm_router_match(match_ms: u64, prepared_hit: bool, parse_fallback: bool) {
    let shared = global_wasm_router_metrics();
    let Ok(mut locked) = shared.lock() else {
        return;
    };
    locked.match_calls_total = locked.match_calls_total.saturating_add(1);
    locked.match_ms_total = locked.match_ms_total.saturating_add(match_ms);
    if prepared_hit {
        locked.prepared_hits = locked.prepared_hits.saturating_add(1);
    }
    if parse_fallback {
        locked.parse_fallbacks = locked.parse_fallbacks.saturating_add(1);
    }
    observe_bucket(&mut locked.match_ms_buckets, match_ms);
}

pub fn observe_wasm_router_regex_compile(regex_compile_ms: u64) {
    let shared = global_wasm_router_metrics();
    let Ok(mut locked) = shared.lock() else {
        return;
    };
    locked.regex_compile_ms_total = locked
        .regex_compile_ms_total
        .saturating_add(regex_compile_ms);
}

fn global_wasm_router_metrics() -> SharedWasmRouterMetrics {
    static GLOBAL: OnceLock<SharedWasmRouterMetrics> = OnceLock::new();
    GLOBAL
        .get_or_init(|| Arc::new(Mutex::new(WasmRouterMetricsSnapshot::empty())))
        .clone()
}

fn empty_bucket_map() -> BTreeMap<String, u64> {
    let mut buckets = BTreeMap::new();
    for (_, label) in WALL_BUCKETS {
        buckets.insert((*label).to_string(), 0);
    }
    buckets.insert(WALL_OVERFLOW_BUCKET.to_string(), 0);
    buckets
}

fn observe_bucket(buckets: &mut BTreeMap<String, u64>, elapsed_ms: u64) {
    for (upper_bound_ms, label) in WALL_BUCKETS {
        if elapsed_ms <= *upper_bound_ms {
            *buckets.entry((*label).to_string()).or_insert(0) += 1;
            return;
        }
    }
    *buckets.entry(WALL_OVERFLOW_BUCKET.to_string()).or_insert(0) += 1;
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(i64::MAX))
        .unwrap_or(0)
}
