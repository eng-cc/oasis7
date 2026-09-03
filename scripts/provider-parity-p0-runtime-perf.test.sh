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
  local cognition_lane="builtin_host_runner"
  local decision_route="builtin"
  local feedback_route="builtin"
  local agent_profile=""
  if [[ "$provider" == "provider_loopback_http" ]]; then
    cognition_lane="target_outer_context_v1"
    decision_route="/v1/world-simulator/decision-context"
    feedback_route="/v1/world-simulator/feedback-context"
    agent_profile="oasis7_p0_low_freq_npc"
  fi
  local summary_dir="$tmp_root/out/samples/$provider/sample_$sample_index/summary"
  mkdir -p "$summary_dir"
  cat >"$summary_dir/sample.json" <<JSON
{
  "benchmark_run_id": "runtime-perf-test",
  "parity_tier": "P0",
  "scenario_id": "P0-001",
  "fixture_id": "P0-001_sample_$sample_index",
  "seed": "seed-1",
  "status": "passed",
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
  "provider": {
    "agent_profile": "$agent_profile",
    "cognition_lane": "$cognition_lane",
    "decision_route": "$decision_route",
    "feedback_route": "$feedback_route"
  },
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

write_sample_with_status() {
  local provider=$1
  local sample_index=$2
  local status=$3
  local goal_completed=$4
  local median_latency_ms=$5
  local p95_latency_ms=$6
  local agent_profile=${7:-}
  local fixture_id=${8:-"P0-001_sample_$sample_index"}
  if [[ "$provider" == "provider_loopback_http" && -z "$agent_profile" ]]; then
    agent_profile="oasis7_p0_low_freq_npc"
  fi
  local sample_dir="$tmp_root/status/out/samples/$provider/sample_$sample_index/summary"
  mkdir -p "$sample_dir"
  cat >"$sample_dir/sample.json" <<JSON
{
  "benchmark_run_id": "status-test",
  "parity_tier": "P0",
  "scenario_id": "P0-001",
  "fixture_id": "$fixture_id",
  "seed": "seed-1",
  "status": "$status",
  "goal_completed": $goal_completed,
  "decision_steps": 10,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "median_latency_ms": $median_latency_ms,
  "p95_latency_ms": $p95_latency_ms,
  "trace_completeness_ratio_ppm": 1000000,
  "provider_version": "test-provider",
  "adapter_version": "test-adapter",
  "protocol_version": "test-protocol",
  "provider": {
    "agent_profile": "$agent_profile",
    "cognition_lane": "target_outer_context_v1",
    "decision_route": "/v1/world-simulator/decision-context",
    "feedback_route": "/v1/world-simulator/feedback-context"
  },
  "runtime_perf": {
    "tick": {
      "samples_total": 1,
      "p95_ms": 1,
      "over_budget_ratio_ppm": 0
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
  "status": "passed",
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

write_sample_with_legacy_routes() {
  local summary_dir="$tmp_root/legacy/out/samples/provider_loopback_http/sample_1/summary"
  mkdir -p "$summary_dir"
  cat >"$summary_dir/sample.json" <<'JSON'
{
  "status": "passed",
  "goal_completed": true,
  "decision_steps": 1,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "median_latency_ms": 11,
  "p95_latency_ms": 22,
  "trace_completeness_ratio_ppm": 1000000,
  "provider_version": "legacy-test-provider",
  "adapter_version": "legacy-test-adapter",
  "protocol_version": "legacy-test-protocol",
  "provider": {
    "cognition_lane": "legacy_compatibility",
    "decision_route": "/v1/world-simulator/decision",
    "feedback_route": "/v1/world-simulator/feedback"
  }
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
assert provider["cognition_lane"] == "target_outer_context_v1"
assert provider["decision_route"] == "/v1/world-simulator/decision-context"
assert provider["feedback_route"] == "/v1/world-simulator/feedback-context"
assert builtin["agent_profile"] == "oasis7_p0_low_freq_npc"
assert provider["agent_profile"] == "oasis7_p0_low_freq_npc"
assert provider["relative_wait_gap_median_ms"] == 0
assert provider["relative_wait_gap_p95_ms"] == 0
assert provider["latency_class"] == "A"
assert builtin["latency_class"] is None
assert provider["parity_status"] == "passed"
assert provider["release_gate"] == "default_candidate"

rows = {
    row["metric"]: row
    for row in csv.DictReader((out_dir / "summary" / "combined.csv").open())
}
assert rows["runtime_perf.tick.p95_ms_peak"]["builtin"] == "7.25"
assert rows["runtime_perf.tick.p95_ms_peak"]["provider_loopback_http"] == "8.5"
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["builtin"] == "20"
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["provider_loopback_http"] == "30"
PY

write_sample_with_status builtin 1 passed true 10 20
write_sample_with_status provider_loopback_http 1 passed true 601 1620

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id status-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/status/out"

python3 - "$tmp_root/status/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
builtin = json.loads((out_dir / "summary" / "P0-001.builtin.json").read_text())
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert builtin["benchmark_status"] == "insufficient_data"
assert provider["benchmark_status"] == "insufficient_data"
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
PY

write_sample_with_status builtin 1 passed true 10 20
write_sample_with_status builtin 2 passed false 10 20
write_sample_with_status provider_loopback_http 2 failed false 11 21

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id failed-sample-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/status/out"

python3 - "$tmp_root/status/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
builtin = json.loads((out_dir / "summary" / "P0-001.builtin.json").read_text())
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert builtin["benchmark_status"] == "failed"
assert provider["benchmark_status"] == "failed"
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
PY

write_sample_with_status builtin 1 passed true 10 20
write_sample_with_status builtin 2 passed true 10 20
write_sample_with_status provider_loopback_http 1 passed true 601 1620
write_sample_with_status provider_loopback_http 2 passed true 601 1620

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id latency-class-b-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/status/out"

python3 - "$tmp_root/status/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert provider["parity_status"] == "passed"
assert provider["relative_wait_gap_median_ms"] == 591
assert provider["relative_wait_gap_p95_ms"] == 1600
assert provider["latency_class"] == "B"
assert provider["release_gate"] == "experimental_only"
PY

write_sample_with_status provider_loopback_http 2 passed true 11 21 oasis7_p0_low_freq_npc mismatched-fixture

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id fixture-binding-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/status/out"

python3 - "$tmp_root/status/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert provider["fixture_binding_status"] == "mismatched"
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
PY

write_sample_with_status provider_loopback_http 2 passed true 11 21 wrong_profile

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id profile-binding-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/status/out"

python3 - "$tmp_root/status/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert provider["profile_binding_status"] == "missing_or_mismatched"
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
PY

write_sample_with_status builtin 1 passed true 10 20
write_sample_with_status builtin 2 passed true 10 20
write_sample_with_status provider_loopback_http 1 passed true 6010 8030
write_sample_with_status provider_loopback_http 2 passed true 6010 8030

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id relative-wait-threshold-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/status/out"

python3 - "$tmp_root/status/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert provider["relative_wait_gap_median_ms"] == 6000
assert provider["relative_wait_gap_p95_ms"] == 8010
assert provider["parity_status"] == "failed"
assert provider["release_gate"] == "experimental_only"
assert provider["parity_gate"]["checks"]["relative_wait_gap_median_ms"]["passed"] is False
assert provider["parity_gate"]["checks"]["relative_wait_gap_p95_ms"]["passed"] is False
PY

write_sample_with_legacy_routes

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id legacy-route-test \
  --scenario-id P0-001 \
  --samples 1 \
  --provider-only \
  --out-dir "$tmp_root/legacy/out"

python3 - "$tmp_root/legacy/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert provider["benchmark_status"] == "blocked"
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
assert "target_outer_cognition_route_missing" in provider["warnings"]
assert provider["decision_route"] == "/v1/world-simulator/decision"
assert provider["feedback_route"] == "/v1/world-simulator/feedback"
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
assert builtin["runtime_perf"]["tick"]["coverage_sample_count"] == 0
assert "runtime_perf_missing" in builtin["warnings"]

rows = {
    row["metric"]: row
    for row in csv.DictReader((out_dir / "summary" / "combined.csv").open())
}
assert rows["runtime_perf.tick.p95_ms_peak"]["builtin"] == ""
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["builtin"] == ""
assert rows["runtime_perf.tick.coverage_sample_count"]["builtin"] == "0"
assert rows["warnings"]["builtin"] == "runtime_perf_missing"
PY

echo "provider parity runtime perf aggregation smoke checks passed"
