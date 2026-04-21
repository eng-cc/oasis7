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

check_no_reset_window
check_reset_window
check_single_sample_compat
