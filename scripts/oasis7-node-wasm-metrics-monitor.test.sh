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
check_reset_window
check_single_sample_compat
check_missing_timestamp_is_rejected
check_unavailable_metrics_disable_window
