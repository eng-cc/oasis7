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
assert summary["window"]["executor"]["p50_call_ms"]["upper_bound_ms"] == 10
assert summary["window"]["executor"]["p95_call_ms"]["upper_bound_ms"] == 50
assert summary["window"]["router"]["match_calls_total_delta"] == 5
assert summary["window"]["router"]["p95_match_ms"]["upper_bound_ms"] == 25
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
assert summary["window"]["executor"]["p95_call_ms"]["upper_bound_ms"] == 50
assert summary["window"]["router"]["match_calls_total_delta"] == 4
assert summary["window"]["top_hotspot"] == "executor.entrypoint_call_ms_total"
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
check_build_timestamp_churn_keeps_runtime_window
check_single_sample_compat
check_missing_timestamp_is_rejected
check_unavailable_metrics_disable_window
