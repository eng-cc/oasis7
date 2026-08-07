use std::collections::BTreeMap;

use oasis7::simulator::{RuntimePerfBottleneck, RuntimePerfHealth};

fn runtime_perf_for_steady_window(
    recent_over_budget_count: u64,
    p95_total_ms: u64,
    latest_total_ms: u64,
    max_total_ms: u64,
) -> oasis7::simulator::RuntimePerfSnapshot {
    let timing = super::super::execution_bridge::ExecutionBridgeCommitTimingSnapshot {
        window_capacity: 128,
        recent_commit_count: 128,
        recent_over_budget_count,
        recent_over_budget_ratio_ppm: recent_over_budget_count * 1_000_000 / 128,
        p50_total_ms: Some(780),
        p95_total_ms: Some(p95_total_ms),
        latest_total_ms: Some(latest_total_ms),
        max_total_ms: Some(max_total_ms),
        slow_count: recent_over_budget_count,
        last_slow_stage: Some("cas_put".to_string()),
        stages: BTreeMap::new(),
    };

    super::super::status_payload::build_runtime_perf_snapshot_from_execution_bridge_timing(&timing)
        .expect("runtime perf snapshot")
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_accepts_one_isolated_steady_window_jitter() {
    let runtime_perf = runtime_perf_for_steady_window(1, 900, 900, 1_100);

    assert_eq!(runtime_perf.health, RuntimePerfHealth::Healthy);
    assert_eq!(runtime_perf.bottleneck, RuntimePerfBottleneck::None);
    assert_eq!(runtime_perf.action_execution.samples_window, 128);
    assert_eq!(runtime_perf.action_execution.p95_ms, 900.0);
    assert_eq!(runtime_perf.action_execution.over_budget_total, 1);
    assert_eq!(runtime_perf.action_execution.last_ms, 900.0);
    assert_eq!(runtime_perf.action_execution.max_ms, 1_100.0);
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_projects_latest_sample_separately_from_maximum()
 {
    let runtime_perf = runtime_perf_for_steady_window(1, 900, 900, 1_100);

    assert_eq!(runtime_perf.action_execution.last_ms, 900.0);
    assert_eq!(runtime_perf.action_execution.max_ms, 1_100.0);
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_rejects_second_steady_window_breach() {
    let runtime_perf = runtime_perf_for_steady_window(2, 900, 1_100, 1_100);

    assert_eq!(runtime_perf.health, RuntimePerfHealth::Warn);
    assert_eq!(
        runtime_perf.bottleneck,
        RuntimePerfBottleneck::ActionExecution
    );
    assert_eq!(runtime_perf.action_execution.over_budget_total, 2);
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_rejects_catastrophic_steady_window_outlier() {
    let runtime_perf = runtime_perf_for_steady_window(1, 900, 1_250, 1_250);

    assert_eq!(runtime_perf.health, RuntimePerfHealth::Warn);
    assert_eq!(
        runtime_perf.bottleneck,
        RuntimePerfBottleneck::ActionExecution
    );
    assert_eq!(runtime_perf.action_execution.max_ms, 1_250.0);
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_does_not_qualify_an_incomplete_window() {
    let timing = super::super::execution_bridge::ExecutionBridgeCommitTimingSnapshot {
        window_capacity: 128,
        recent_commit_count: 127,
        recent_over_budget_count: 0,
        recent_over_budget_ratio_ppm: 0,
        p50_total_ms: Some(780),
        p95_total_ms: Some(900),
        latest_total_ms: Some(900),
        max_total_ms: Some(900),
        slow_count: 0,
        last_slow_stage: None,
        stages: BTreeMap::new(),
    };

    let runtime_perf =
        super::super::status_payload::build_runtime_perf_snapshot_from_execution_bridge_timing(
            &timing,
        )
        .expect("runtime perf snapshot");

    assert_eq!(runtime_perf.health, RuntimePerfHealth::Warn);
    assert_eq!(
        runtime_perf.bottleneck,
        RuntimePerfBottleneck::ActionExecution
    );
    assert_eq!(runtime_perf.action_execution.samples_window, 127);
}
