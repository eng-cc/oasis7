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
  "execution_authority": "simulator_world_kernel",
  "runtime_certification_status": "not_certified",
  "runtime_certification_reason": "local simulator smoke; unified Runtime execution and receipt authority is not wired",
  "goal_completed": true,
  "decision_steps": 10,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "metric_schema_version": "recoverable_error_resolution_rate.v1",
  "sample_id": "P0-001_sample_$sample_index",
  "trace_validity": "valid",
  "recovery_events": [],
  "recoverable_error_resolution_rate": {
    "numerator": 0,
    "denominator": 0,
    "value": null,
    "zero_case": "not_applicable",
    "gate_status": "not_evaluable"
  },
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
  "execution_authority": "simulator_world_kernel",
  "runtime_certification_status": "not_certified",
  "runtime_certification_reason": "local simulator smoke; unified Runtime execution and receipt authority is not wired",
  "goal_completed": $goal_completed,
  "decision_steps": 10,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "metric_schema_version": "recoverable_error_resolution_rate.v1",
  "sample_id": "$fixture_id",
  "trace_validity": "valid",
  "recovery_events": [],
  "recoverable_error_resolution_rate": {
    "numerator": 0,
    "denominator": 0,
    "value": null,
    "zero_case": "not_applicable",
    "gate_status": "not_evaluable"
  },
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
  "execution_authority": "simulator_world_kernel",
  "runtime_certification_status": "not_certified",
  "runtime_certification_reason": "local simulator smoke; unified Runtime execution and receipt authority is not wired",
  "goal_completed": true,
  "decision_steps": 10,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "metric_schema_version": "recoverable_error_resolution_rate.v1",
  "sample_id": "P0-001_sample_$sample_index",
  "trace_validity": "valid",
  "recovery_events": [],
  "recoverable_error_resolution_rate": {
    "numerator": 0,
    "denominator": 0,
    "value": null,
    "zero_case": "not_applicable",
    "gate_status": "not_evaluable"
  },
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
  "execution_authority": "simulator_world_kernel",
  "runtime_certification_status": "not_certified",
  "runtime_certification_reason": "local simulator smoke; unified Runtime execution and receipt authority is not wired",
  "goal_completed": true,
  "decision_steps": 1,
  "invalid_action_count": 0,
  "timeout_count": 0,
  "recoverable_error_count": 0,
  "metric_schema_version": "recoverable_error_resolution_rate.v1",
  "sample_id": "P0-001_sample_1",
  "trace_validity": "valid",
  "recovery_events": [],
  "recoverable_error_resolution_rate": {
    "numerator": 0,
    "denominator": 0,
    "value": null,
    "zero_case": "not_applicable",
    "gate_status": "not_evaluable"
  },
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
assert provider["benchmark_status"] == "passed"
assert provider["execution_authority"] == "simulator_world_kernel"
assert provider["runtime_certification_status"] == "not_certified"
assert provider["runtime_certification_errors"] == []
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
assert provider["parity_gate"]["checks"]["runtime_certification"]["passed"] is False

rows = {
    row["metric"]: row
    for row in csv.DictReader((out_dir / "summary" / "combined.csv").open())
}
assert rows["runtime_perf.tick.p95_ms_peak"]["builtin"] == "7.25"
assert rows["runtime_perf.tick.p95_ms_peak"]["provider_loopback_http"] == "8.5"
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["builtin"] == "20"
assert rows["runtime_perf.tick.over_budget_ratio_ppm_peak"]["provider_loopback_http"] == "30"
PY

python3 - "$tmp_root/out/samples/provider_loopback_http/sample_1/summary/sample.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
sample = json.loads(path.read_text())
sample["runtime_certification_status"] = "certified"
path.write_text(json.dumps(sample), encoding="utf-8")
PY

PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
  --run-id forged-certification-test \
  --scenario-id P0-001 \
  --samples 2 \
  --out-dir "$tmp_root/out"

python3 - "$tmp_root/out" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
provider = json.loads((out_dir / "summary" / "P0-001.provider_loopback_http.json").read_text())
assert provider["runtime_certification_status"] == "blocked"
assert provider["benchmark_status"] == "blocked"
assert provider["parity_status"] == "blocked"
assert any("cannot assert Runtime certification" in error for error in provider["runtime_certification_errors"])
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
assert provider["parity_status"] == "blocked"
assert provider["relative_wait_gap_median_ms"] == 591
assert provider["relative_wait_gap_p95_ms"] == 1600
assert provider["latency_class"] == "B"
assert provider["release_gate"] == "blocked"
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
assert provider["parity_status"] == "blocked"
assert provider["release_gate"] == "blocked"
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

write_recovery_sample() {
  local root=$1
  local mode=$2
  local sample_index=${3:-1}
  python3 - "$root" "$mode" "$sample_index" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
sample_index = int(sys.argv[3])
run_id = f"recovery-{mode}"
sample_id = f"P0-005_sample_{sample_index}"
ORIGIN_DIGEST = "blake3:" + "0" * 64
SECOND_ORIGIN_DIGEST = "blake3:" + "1" * 64
RECOVERY_DIGEST = "blake3:" + "2" * 64

def error(error_id, seq, turn, chain):
    return {
        "event_kind": "recoverable_error",
        "event_seq": seq,
        "error_id": error_id,
        "error_code": "timeout",
        "sample_id": sample_id,
        "agent_id": "agent-1",
        "agent_session_id": "session-1",
        "recovery_chain_id": chain,
        "agent_turn_id": turn,
        "decision_request_id": f"request-{error_id}",
        "request_digest": ORIGIN_DIGEST if error_id == "error-1" else SECOND_ORIGIN_DIGEST,
    }

def resolved(error_id, seq, origin_turn, chain, *, authority_ref=True, origin_request_digest=ORIGIN_DIGEST):
    event = {
        "event_kind": "recovery_resolved",
        "event_seq": seq,
        "error_id": error_id,
        "sample_id": sample_id,
        "agent_id": "agent-1",
        "agent_session_id": "session-1",
        "recovery_chain_id": chain,
        "agent_turn_id": "turn-recovery",
        "decision_request_id": f"request-{error_id}-retry",
        "request_digest": RECOVERY_DIGEST,
        "retry_seq": 2,
        "origin_turn_id": origin_turn,
        "origin_request_digest": origin_request_digest,
        "authority": "runtime_or_fixture_host",
        "runtime_outcome": "action_committed",
    }
    if authority_ref:
        event["authority_ref"] = "fixture-host://recovery/action-1"
    return event

if mode == "happy":
    events = [error("error-1", 1, "turn-1", "chain-1"), resolved("error-1", 2, "turn-1", "chain-1")]
    status, goal_completed = "passed", True
elif mode == "retry-too-low":
    events = [error("error-1", 1, "turn-1", "chain-1"), resolved("error-1", 2, "turn-1", "chain-1")]
    events[1]["retry_seq"] = 1
    status, goal_completed = "passed", True
elif mode in {"timeout-only", "goal-flag-only"}:
    events = [error("error-1", 1, "turn-1", "chain-1")]
    status, goal_completed = ("passed", True) if mode == "goal-flag-only" else ("failed", False)
elif mode == "partial":
    events = [
        error("error-1", 1, "turn-1", "chain-1"),
        error("error-2", 2, "turn-2", "chain-2"),
        resolved("error-1", 3, "turn-1", "chain-1"),
    ]
    status, goal_completed = "failed", False
elif mode == "zero":
    events = []
    status, goal_completed = "passed", True
elif mode == "malformed":
    events = [
        error("error-1", 1, "turn-1", "chain-1"),
        resolved(
            "error-1", 2, "turn-1", "chain-1", origin_request_digest="blake3:origin"
        ),
    ]
    status, goal_completed = "passed", True
elif mode == "wrong-origin":
    events = [
        error("error-1", 1, "turn-1", "chain-1"),
        resolved(
            "error-1", 2, "turn-1", "chain-1", origin_request_digest=SECOND_ORIGIN_DIGEST
        ),
    ]
    status, goal_completed = "passed", True
elif mode == "missing-identity":
    events = [error("error-1", 1, "turn-1", "chain-1"), resolved("error-1", 2, "turn-1", "chain-1")]
    for field in ("sample_id", "decision_request_id", "request_digest", "retry_seq"):
        events[1].pop(field)
    status, goal_completed = "passed", True
else:
    raise SystemExit(f"unknown recovery fixture mode: {mode}")

denominator = sum(event["event_kind"] == "recoverable_error" for event in events)
numerator = sum(event["event_kind"] == "recovery_resolved" for event in events)
metric = {
    "numerator": numerator,
    "denominator": denominator,
    "value": None if denominator == 0 else numerator / denominator,
    "zero_case": "not_applicable" if denominator == 0 else None,
    "gate_status": "not_evaluable" if denominator == 0 else "evaluable",
}
sample = {
    "benchmark_run_id": run_id,
    "parity_tier": "P0",
    "scenario_id": "P0-005",
    "fixture_id": sample_id,
    "sample_id": sample_id,
    "seed": "seed-1",
    "status": status,
    "execution_authority": "simulator_world_kernel",
    "runtime_certification_status": "not_certified",
    "runtime_certification_reason": "fixture ledger only; no Runtime certification is asserted",
    "goal_completed": goal_completed,
    "decision_steps": 10,
    "invalid_action_count": 0,
    "timeout_count": denominator,
    "recoverable_error_count": denominator,
    "fatal_error_count": 0,
    "trace_completeness_ratio_ppm": 1_000_000,
    "median_latency_ms": 11,
    "p95_latency_ms": 22,
    "metric_schema_version": "recoverable_error_resolution_rate.v1",
    "trace_validity": "valid",
    "recovery_events": events,
    "recoverable_error_resolution_rate": metric,
    "error_counts": {"timeout": denominator} if denominator else {},
    "provider_version": "test-provider",
    "adapter_version": "test-adapter",
    "protocol_version": "test-protocol",
    "provider": {
        "agent_profile": "oasis7_p0_low_freq_npc",
        "cognition_lane": "target_outer_context_v1",
        "decision_route": "/v1/world-simulator/decision-context",
        "feedback_route": "/v1/world-simulator/feedback-context",
    },
    "runtime_perf": {"tick": {"samples_total": 1, "p95_ms": 1, "over_budget_ratio_ppm": 0}},
}
summary_path = root / "samples" / "provider_loopback_http" / f"sample_{sample_index}" / "summary" / "sample.json"
summary_path.parent.mkdir(parents=True, exist_ok=True)
summary_path.write_text(json.dumps(sample), encoding="utf-8")
PY
}

for recovery_mode in happy retry-too-low timeout-only goal-flag-only partial zero malformed wrong-origin missing-identity; do
  recovery_root="$tmp_root/recovery-$recovery_mode/out"
  write_recovery_sample "$recovery_root" "$recovery_mode"
  PROVIDER_PARITY_P0_AGGREGATE_ONLY=1 ./scripts/provider-parity-p0.sh \
    --run-id "recovery-$recovery_mode" \
    --scenario-id P0-005 \
    --samples 1 \
    --provider-only \
    --out-dir "$recovery_root"
done

python3 - "$tmp_root" <<'PY'
import json
import pathlib
import sys

tmp_root = pathlib.Path(sys.argv[1])

def summary(mode):
    path = tmp_root / f"recovery-{mode}" / "out" / "summary" / "P0-005.provider_loopback_http.json"
    return json.loads(path.read_text())

happy = summary("happy")
assert happy["recoverable_error_resolution_rate"] == {
    "numerator": 1, "denominator": 1, "value": 1.0, "zero_case": None, "gate_status": "evaluable"
}
assert happy["benchmark_status"] == "passed"

retry_too_low = summary("retry-too-low")
assert retry_too_low["recoverable_error_resolution_rate"]["gate_status"] == "blocked"
assert retry_too_low["recoverable_error_resolution_rate"]["value"] is None
assert retry_too_low["recoverable_error_resolution_rate"]["denominator"] == 1
assert retry_too_low["benchmark_status"] == "blocked"
assert retry_too_low["recovery_ledger_status"] == "blocked"

for mode in ("timeout-only", "goal-flag-only"):
    result = summary(mode)
    metric = result["recoverable_error_resolution_rate"]
    assert metric["numerator"] == 0 and metric["denominator"] == 1 and metric["value"] == 0.0
    assert result["benchmark_status"] == "failed"

partial = summary("partial")
assert partial["recoverable_error_resolution_rate"]["numerator"] == 1
assert partial["recoverable_error_resolution_rate"]["denominator"] == 2
assert partial["recoverable_error_resolution_rate"]["value"] == 0.5
assert partial["benchmark_status"] == "failed"

zero = summary("zero")
assert zero["recoverable_error_resolution_rate"] == {
    "numerator": 0, "denominator": 0, "value": None,
    "zero_case": "not_applicable", "gate_status": "not_evaluable"
}
assert zero["benchmark_status"] == "failed"

malformed = summary("malformed")
assert malformed["recoverable_error_resolution_rate"]["gate_status"] == "blocked"
assert malformed["recoverable_error_resolution_rate"]["value"] is None
assert malformed["recoverable_error_resolution_rate"]["denominator"] == 1
assert malformed["benchmark_status"] == "blocked"
assert malformed["recovery_ledger_status"] == "blocked"

wrong_origin = summary("wrong-origin")
assert wrong_origin["recoverable_error_resolution_rate"]["gate_status"] == "blocked"
assert wrong_origin["recoverable_error_resolution_rate"]["value"] is None
assert wrong_origin["recoverable_error_resolution_rate"]["denominator"] == 1
assert wrong_origin["benchmark_status"] == "blocked"
assert wrong_origin["recovery_ledger_status"] == "blocked"

missing_identity = summary("missing-identity")
assert missing_identity["recoverable_error_resolution_rate"]["gate_status"] == "blocked"
assert missing_identity["recoverable_error_resolution_rate"]["value"] is None
assert missing_identity["benchmark_status"] == "blocked"
assert missing_identity["recovery_ledger_status"] == "blocked"
PY

echo "provider parity runtime perf aggregation smoke checks passed"
