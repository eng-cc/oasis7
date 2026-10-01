#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

check_no_reset_window() {
  local out_dir="$tmp_root/no-reset"
  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir fixtures/wasm_metrics_monitor/no_reset \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["window"]["available"] is True
assert summary["window"]["window_reset_detected"] is False
assert summary["window"]["executor"]["calls_total_delta"] == 5
assert summary["window"]["executor"]["compile_ms_total_delta"] == 40
assert summary["window"]["router"]["match_calls_total_delta"] == 5
assert summary["window"]["top_hotspot"] == "executor.entrypoint_call_ms_total"
assert summary["window"]["module_hotspot_source"] == "not_reported"
assert summary["window"]["top_module_hotspot"] == "not_reported"
PY
}

check_module_hotspot_window() {
  local sample_dir="$tmp_root/module-hotspots"
  local out_dir="$tmp_root/module-hotspots-out"
  mkdir -p "$sample_dir"
  python3 - "$sample_dir/001.json" "$sample_dir/002.json" <<'PY'
import json
import sys
from pathlib import Path

samples = [
    ("fixtures/wasm_metrics_monitor/no_reset/001.json", sys.argv[1], [
        {
            "module_id": "m.alpha",
            "calls_total": 10,
            "wall_ms_total": 100,
            "failure_count": 0,
            "share_ppm": 625000,
        },
        {
            "module_id": "m.beta",
            "calls_total": 5,
            "wall_ms_total": 60,
            "failure_count": 1,
            "share_ppm": 375000,
        },
    ]),
    ("fixtures/wasm_metrics_monitor/no_reset/002.json", sys.argv[2], [
        {
            "module_id": "m.alpha",
            "calls_total": 12,
            "wall_ms_total": 130,
            "failure_count": 0,
            "share_ppm": 342105,
        },
        {
            "module_id": "m.beta",
            "calls_total": 8,
            "wall_ms_total": 145,
            "failure_count": 2,
            "share_ppm": 381579,
        },
        {
            "module_id": "m.gamma",
            "calls_total": 1,
            "wall_ms_total": 105,
            "failure_count": 0,
            "share_ppm": 276316,
        },
    ]),
]
for src, dst, module_hotspots in samples:
    payload = json.loads(Path(src).read_text())
    payload["wasm"]["executor"]["module_hotspots"] = module_hotspots
    Path(dst).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
PY

  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir "$sample_dir" \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" "$out_dir/latest_summary.md" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
markdown = Path(sys.argv[2]).read_text()
module_hotspots = summary["window"]["latest_module_hotspots"]
assert summary["window"]["module_hotspot_source"] == "reported"
assert summary["window"]["top_module_hotspot"] == "m.beta"
assert module_hotspots[0]["module_id"] == "m.beta"
assert module_hotspots[0]["wall_ms_total"] == 145
assert module_hotspots[0]["failure_count"] == 2
assert module_hotspots[1]["module_id"] == "m.alpha"
assert module_hotspots[2]["module_id"] == "m.gamma"
assert "## Module Hotspots" in markdown
assert "scope: `latest_cumulative_bounded_top_n`" in markdown
assert "`m.beta`: wall_ms_total=`145`" in markdown
PY
}

check_reported_empty_module_hotspots() {
  local sample_dir="$tmp_root/reported-empty-module-hotspots"
  local out_dir="$tmp_root/reported-empty-module-hotspots-out"
  mkdir -p "$sample_dir"
  python3 - "$sample_dir/001.json" "$sample_dir/002.json" <<'PY'
import json
import sys
from pathlib import Path

for src, dst in [
    ("fixtures/wasm_metrics_monitor/no_reset/001.json", sys.argv[1]),
    ("fixtures/wasm_metrics_monitor/no_reset/002.json", sys.argv[2]),
]:
    payload = json.loads(Path(src).read_text())
    payload["wasm"]["executor"]["module_hotspots"] = []
    Path(dst).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
PY

  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir "$sample_dir" \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" "$out_dir/latest_summary.md" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
markdown = Path(sys.argv[2]).read_text()
assert summary["window"]["module_hotspot_source"] == "reported"
assert summary["window"]["top_module_hotspot"] == "none"
assert summary["window"]["latest_module_hotspots"] == []
assert "- none" in markdown
PY
}

check_reset_window() {
  local out_dir="$tmp_root/reset"
  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir fixtures/wasm_metrics_monitor/reset \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["window"]["available"] is True
assert summary["window"]["window_reset_detected"] is True
assert summary["sample_overview"]["reset_event_count"] == 1
assert summary["window"]["executor"]["calls_total_delta"] == 4
assert summary["window"]["executor"]["compile_ms_total_delta"] == 12
assert summary["window"]["router"]["match_calls_total_delta"] == 4
assert summary["window"]["top_hotspot"] == "executor.entrypoint_call_ms_total"
PY
}

check_exclusive_bucket_percentiles() {
  local sample_root="$tmp_root/exclusive-percentiles"
  local out_root="$tmp_root/exclusive-percentiles-out"
  mkdir -p "$sample_root"
  python3 - "$sample_root" <<'PY'
import json
import sys
from pathlib import Path

sample_root = Path(sys.argv[1])
template = json.loads(Path("fixtures/wasm_metrics_monitor/no_reset/001.json").read_text())
labels = [
    "le_0001_ms",
    "le_0005_ms",
    "le_0010_ms",
    "le_0025_ms",
    "le_0050_ms",
    "le_0100_ms",
    "le_0250_ms",
    "le_0500_ms",
    "le_1000_ms",
    "gt_1000_ms",
]
zero = [0] * len(labels)
rank_boundary = [3, 7, 0, 0, 0, 0, 0, 0, 5, 5]
next_finite = [9, 1, 0, 0, 0, 0, 0, 0, 0, 0]
overflow_only = [0] * 9 + [4]
reset_window = [1, 1, 0, 0, 0, 0, 0, 0, 0, 2]
large_rank = [4_503_599_627_370_497, 4_503_599_627_370_498] + [0] * 8
float_rank_boundary = [4_503_599_627_370_500, 4_503_599_627_370_501] + [0] * 8


def write_sample(scenario, name, observed_at, observed_since, executor_counts, match_counts):
    payload = json.loads(json.dumps(template))
    payload["observed_at_unix_ms"] = observed_at
    wasm = payload["wasm"]
    wasm["observed_since_unix_ms"] = observed_since
    executor = wasm["executor"]
    router = wasm["router"]
    executor["observed_since_unix_ms"] = observed_since
    router["observed_since_unix_ms"] = observed_since
    executor["calls_total"] = sum(executor_counts)
    executor["call_wall_ms_buckets"] = dict(zip(labels, executor_counts))
    router["match_calls_total"] = sum(match_counts)
    router["match_ms_buckets"] = dict(zip(labels, match_counts))
    router["prepare_calls_total"] = 0
    router["prepare_ms_buckets"] = dict(zip(labels, zero))
    path = sample_root / scenario / f"{name}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")


for scenario, counts in [
    ("rank-boundary", rank_boundary),
    ("next-finite", next_finite),
    ("no-traffic", zero),
    ("overflow-only", overflow_only),
    ("large-rank", large_rank),
    ("float-rank-boundary", float_rank_boundary),
]:
    write_sample(scenario, "001", 1700000001000, 1700000000000, zero, zero)
    write_sample(scenario, "002", 1700000002000, 1700000000000, counts, counts)

write_sample("reset", "001", 1700000001000, 1700000000000, overflow_only[:-1] + [100], overflow_only[:-1] + [100])
write_sample("reset", "002", 1700000002000, 1700000002000, zero, zero)
write_sample("reset", "003", 1700000003000, 1700000002000, reset_window, reset_window)
PY

  for scenario in rank-boundary next-finite no-traffic overflow-only reset large-rank float-rank-boundary; do
    bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
      --status-sample-dir "$sample_root/$scenario" \
      --node-label test-node \
      --out-dir "$out_root/$scenario"
  done

  python3 - "$out_root" <<'PY'
import json
import sys
from pathlib import Path

out_root = Path(sys.argv[1])
failures = []


def load(scenario):
    return json.loads((out_root / scenario / "latest_summary.json").read_text())


def check(name, operation):
    try:
        operation()
    except AssertionError as error:
        failures.append(f"{name}: {error}")


def check_bound(value, expected):
    assert isinstance(value, dict), f"expected bucket object, got {value!r}"
    assert set(value) == {"bucket_label", "upper_bound_ms", "lower_bound_ms", "samples"}, value
    assert type(value["bucket_label"]) is str, value
    assert value["upper_bound_ms"] is None or type(value["upper_bound_ms"]) is int, value
    assert type(value["lower_bound_ms"]) is int, value
    assert type(value["samples"]) is int, value
    assert value == expected, f"got {value!r}, expected {expected!r}"


def assert_true(value):
    assert value is True, value


def assert_equal(value, expected):
    assert value == expected, f"got {value!r}, expected {expected!r}"


rank_boundary = load("rank-boundary")
check(
    "rank-boundary executor p50",
    lambda: check_bound(
        rank_boundary["window"]["executor"]["p50_call_ms"],
        {"bucket_label": "le_0005_ms", "upper_bound_ms": 5, "lower_bound_ms": 1, "samples": 20},
    ),
)
check(
    "rank-boundary executor p95 overflow",
    lambda: check_bound(
        rank_boundary["window"]["executor"]["p95_call_ms"],
        {"bucket_label": "gt_1000_ms", "upper_bound_ms": None, "lower_bound_ms": 1000, "samples": 20},
    ),
)
check(
    "rank-boundary router p50",
    lambda: check_bound(
        rank_boundary["window"]["router"]["p50_match_ms"],
        {"bucket_label": "le_0005_ms", "upper_bound_ms": 5, "lower_bound_ms": 1, "samples": 20},
    ),
)
check(
    "rank-boundary router p95 overflow",
    lambda: check_bound(
        rank_boundary["window"]["router"]["p95_match_ms"],
        {"bucket_label": "gt_1000_ms", "upper_bound_ms": None, "lower_bound_ms": 1000, "samples": 20},
    ),
)

next_finite = load("next-finite")
check(
    "next-finite executor p50",
    lambda: check_bound(
        next_finite["window"]["executor"]["p50_call_ms"],
        {"bucket_label": "le_0001_ms", "upper_bound_ms": 1, "lower_bound_ms": 0, "samples": 10},
    ),
)
check(
    "next-finite executor p95",
    lambda: check_bound(
        next_finite["window"]["executor"]["p95_call_ms"],
        {"bucket_label": "le_0005_ms", "upper_bound_ms": 5, "lower_bound_ms": 1, "samples": 10},
    ),
)
check(
    "next-finite router p50",
    lambda: check_bound(
        next_finite["window"]["router"]["p50_match_ms"],
        {"bucket_label": "le_0001_ms", "upper_bound_ms": 1, "lower_bound_ms": 0, "samples": 10},
    ),
)
check(
    "next-finite router p95",
    lambda: check_bound(
        next_finite["window"]["router"]["p95_match_ms"],
        {"bucket_label": "le_0005_ms", "upper_bound_ms": 5, "lower_bound_ms": 1, "samples": 10},
    ),
)

no_traffic = load("no-traffic")
check("no-traffic window availability", lambda: assert_true(no_traffic["window"]["available"]))
for path in [
    ("executor", "p50_call_ms"),
    ("executor", "p95_call_ms"),
    ("router", "p50_match_ms"),
    ("router", "p95_match_ms"),
]:
    check(
        f"no-traffic {path[0]}.{path[1]} null",
        lambda path=path: assert_equal(no_traffic["window"][path[0]][path[1]], None),
    )
no_traffic_markdown = (out_root / "no-traffic" / "latest_summary.md").read_text()
check("no-traffic Markdown n/a", lambda: assert_true("executor.p50_call_ms: `n/a`" in no_traffic_markdown))

overflow_only = load("overflow-only")
for path in [
    ("executor", "p50_call_ms"),
    ("executor", "p95_call_ms"),
    ("router", "p50_match_ms"),
    ("router", "p95_match_ms"),
]:
    check(
        f"overflow-only {path[0]}.{path[1]}",
        lambda path=path: check_bound(
            overflow_only["window"][path[0]][path[1]],
            {"bucket_label": "gt_1000_ms", "upper_bound_ms": None, "lower_bound_ms": 1000, "samples": 4},
        ),
    )
overflow_markdown = (out_root / "overflow-only" / "latest_summary.md").read_text()
check("overflow Markdown bound", lambda: assert_true(">1000ms" in overflow_markdown))

reset = load("reset")
check("reset flag", lambda: assert_true(reset["window"]["window_reset_detected"]))
check("reset event count", lambda: assert_equal(reset["sample_overview"]["reset_event_count"], 1))
check("reset executor call delta", lambda: assert_equal(reset["window"]["executor"]["calls_total_delta"], 4))
check("reset router match delta", lambda: assert_equal(reset["window"]["router"]["match_calls_total_delta"], 4))
check(
    "reset executor p50 post-reset",
    lambda: check_bound(
        reset["window"]["executor"]["p50_call_ms"],
        {"bucket_label": "le_0005_ms", "upper_bound_ms": 5, "lower_bound_ms": 1, "samples": 4},
    ),
)
check(
    "reset executor p95 post-reset",
    lambda: check_bound(
        reset["window"]["executor"]["p95_call_ms"],
        {"bucket_label": "gt_1000_ms", "upper_bound_ms": None, "lower_bound_ms": 1000, "samples": 4},
    ),
)
check(
    "reset router p50 post-reset",
    lambda: check_bound(
        reset["window"]["router"]["p50_match_ms"],
        {"bucket_label": "le_0005_ms", "upper_bound_ms": 5, "lower_bound_ms": 1, "samples": 4},
    ),
)
check(
    "reset router p95 post-reset",
    lambda: check_bound(
        reset["window"]["router"]["p95_match_ms"],
        {"bucket_label": "gt_1000_ms", "upper_bound_ms": None, "lower_bound_ms": 1000, "samples": 4},
    ),
)

large_rank = load("large-rank")
check(
    "large-count exact integer p50 rank",
    lambda: check_bound(
        large_rank["window"]["executor"]["p50_call_ms"],
        {
            "bucket_label": "le_0005_ms",
            "upper_bound_ms": 5,
            "lower_bound_ms": 1,
            "samples": 9_007_199_254_740_995,
        },
    ),
)

float_rank_boundary = load("float-rank-boundary")
for path in [
    ("executor", "p50_call_ms"),
    ("router", "p50_match_ms"),
]:
    check(
        f"integer-ceil precision boundary {path[0]}.{path[1]}",
        lambda path=path: check_bound(
            float_rank_boundary["window"][path[0]][path[1]],
            {
                "bucket_label": "le_0005_ms",
                "upper_bound_ms": 5,
                "lower_bound_ms": 1,
                "samples": 9_007_199_254_741_001,
            },
        ),
    )


if failures:
    for failure in failures:
        print(f"FAIL: {failure}", file=sys.stderr)
    raise SystemExit(1)
PY
}

check_build_timestamp_churn_keeps_runtime_window() {
  local sample_dir="$tmp_root/build-timestamp-churn"
  local out_dir="$tmp_root/build-timestamp-churn-out"
  mkdir -p "$sample_dir"
  cp fixtures/wasm_metrics_monitor/no_reset/001.json "$sample_dir/001.json"
  python3 - "$sample_dir/002.json" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path("fixtures/wasm_metrics_monitor/no_reset/002.json").read_text())
payload["wasm"]["build"]["observed_since_unix_ms"] = 1700000003000
Path(sys.argv[1]).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
PY

  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir "$sample_dir" \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["window"]["available"] is True
assert summary["window"]["window_reset_detected"] is False
assert summary["sample_overview"]["reset_event_count"] == 0
assert summary["window"]["window_sample_count"] == 2
assert summary["window"]["executor"]["calls_total_delta"] == 5
assert summary["window"]["executor"]["compile_ms_total_delta"] == 40
assert summary["window"]["router"]["match_calls_total_delta"] == 5
PY
}

check_single_sample_compat() {
  local out_dir="$tmp_root/single"
  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-json-path fixtures/wasm_metrics_monitor/no_reset/001.json \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["window"]["available"] is False
assert summary["latest"]["node_id"] == "node-a"
assert summary["status_source"] == "file"
PY
}

check_missing_timestamp_is_rejected() {
  local sample_dir="$tmp_root/missing-timestamp"
  local out_dir="$tmp_root/missing-timestamp-out"
  mkdir -p "$sample_dir"
  cp fixtures/wasm_metrics_monitor/no_reset/001.json "$sample_dir/001.json"
  python3 - "$sample_dir/002.json" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path("fixtures/wasm_metrics_monitor/no_reset/002.json").read_text())
payload.pop("observed_at_unix_ms", None)
Path(sys.argv[1]).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
PY

  if bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir "$sample_dir" \
    --node-label test-node \
    --out-dir "$out_dir" >"$tmp_root/missing-timestamp.stdout" 2>"$tmp_root/missing-timestamp.stderr"; then
    echo "expected missing timestamp sample-dir run to fail" >&2
    exit 1
  fi

  grep -q "missing observed_at_unix_ms" "$tmp_root/missing-timestamp.stderr"
}

check_unavailable_metrics_disable_window() {
  local sample_dir="$tmp_root/unavailable"
  local out_dir="$tmp_root/unavailable-out"
  mkdir -p "$sample_dir"
  python3 - "$sample_dir/001.json" "$sample_dir/002.json" <<'PY'
import json
import sys
from pathlib import Path

for src, dst in [
    ("fixtures/wasm_metrics_monitor/no_reset/001.json", sys.argv[1]),
    ("fixtures/wasm_metrics_monitor/no_reset/002.json", sys.argv[2]),
]:
    payload = json.loads(Path(src).read_text())
    payload["wasm"]["metrics_available"] = False
    payload["wasm"]["degraded_reason"] = "metrics disabled for test"
    payload["wasm"]["executor"]["metrics_available"] = False
    payload["wasm"]["executor"]["degraded_reason"] = "executor metrics disabled for test"
    payload["wasm"]["router"]["metrics_available"] = False
    payload["wasm"]["router"]["degraded_reason"] = "router metrics disabled for test"
    Path(dst).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
PY

  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir "$sample_dir" \
    --node-label test-node \
    --out-dir "$out_dir"

  python3 - "$out_dir/latest_summary.json" <<'PY'
import json
import sys
from pathlib import Path

summary = json.loads(Path(sys.argv[1]).read_text())
assert summary["window"]["available"] is False
assert "baseline sample does not expose available wasm executor/router metrics; window delta output is disabled" in summary["window"]["notes"]
assert "latest sample does not expose available wasm executor/router metrics; window delta output is disabled" in summary["window"]["notes"]
PY
}

check_no_reset_window
check_module_hotspot_window
check_reported_empty_module_hotspots
check_reset_window
check_exclusive_bucket_percentiles
check_build_timestamp_churn_keeps_runtime_window
check_single_sample_compat
check_missing_timestamp_is_rejected
check_unavailable_metrics_disable_window
