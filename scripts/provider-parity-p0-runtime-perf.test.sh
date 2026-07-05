#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-provider-parity-runtime-perf.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT

write_sample() {
  local provider=$1
  local sample_index=$2
  local tick_p95_ms=$3
  local over_budget_ratio_ppm=$4
  local summary_dir="$tmp_root/out/samples/$provider/sample_$sample_index/summary"
  mkdir -p "$summary_dir"
  cat >"$summary_dir/sample.json" <<JSON
{
  "status": "ok",
  "goal_completed": true,
  "decision_steps": 10,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "median_latency_ms": 11,
  "p95_latency_ms": 22,
  "trace_completeness_ratio_ppm": 1000000,
  "provider_version": "test-provider",
  "adapter_version": "test-adapter",
  "protocol_version": "test-protocol",
  "runtime_perf": {
    "tick": {
      "samples_total": 1,
      "p95_ms": $tick_p95_ms,
      "over_budget_ratio_ppm": $over_budget_ratio_ppm
    }
  }
}
JSON
}

write_sample_without_runtime_perf() {
  local provider=$1
  local sample_index=$2
  local summary_dir="$tmp_root/missing/out/samples/$provider/sample_$sample_index/summary"
  mkdir -p "$summary_dir"
  cat >"$summary_dir/sample.json" <<JSON
{
  "status": "ok",
  "goal_completed": true,
  "decision_steps": 10,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "median_latency_ms": 11,
  "p95_latency_ms": 22,
  "trace_completeness_ratio_ppm": 1000000,
  "provider_version": "test-provider",
  "adapter_version": "test-adapter",
  "protocol_version": "test-protocol"
}
JSON
}

write_sample builtin 1 4.5 10
write_sample builtin 2 7.25 20
write_sample provider_loopback_http 1 8.5 30
write_sample provider_loopback_http 2 6.0 15

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id runtime-perf-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/out"

python3 - "$tmp_root/out" <<'PY'
import csv
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
builtin = json.loads((out_dir / "summary" / "P0-001.builtin.json").read_text())
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())

assert builtin["runtime_perf"]["tick"]["p95_ms_peak"] == 7.25
assert builtin["runtime_perf"]["tick"]["over_budget_ratio_ppm_peak"] == 20
assert provider["runtime_perf"]["tick"]["p95_ms_peak"] == 8.5
assert provider["runtime_perf"]["tick"]["over_budget_ratio_ppm_peak"] == 30

rows = {
    row["metric"]: row
    for row in csv.DictReader((out_dir / "summary" / "combined.csv").open())
}
assert rows["runtime_perf.tick.p95_ms_peak"]["builtin"] == "7.25"
assert rows["runtime_perf.tick.p95_ms_peak"]["provider_loopback_http"] == "8.5"
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["builtin"] == "20"
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["provider_loopback_http"] == "30"
PY

write_sample_without_runtime_perf builtin 1
write_sample_without_runtime_perf builtin 2

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id runtime-perf-missing-test \
  --scenario-id P0-001 \
  --samples 2 \
  --builtin-only \
  --out-dir "$tmp_root/missing/out"

python3 - "$tmp_root/missing/out" <<'PY'
import csv
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
builtin = json.loads((out_dir / "summary" / "P0-001.builtin.json").read_text())

assert builtin["runtime_perf"]["tick"]["p95_ms_peak"] is None
assert builtin["runtime_perf"]["tick"]["over_budget_ratio_ppm_peak"] is None

rows = {
    row["metric"]: row
    for row in csv.DictReader((out_dir / "summary" / "combined.csv").open())
}
assert rows["runtime_perf.tick.p95_ms_peak"]["builtin"] == ""
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["builtin"] == ""
PY

echo "provider parity runtime perf aggregation smoke checks passed"
