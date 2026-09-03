#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/cargo-dev-lib.sh"

RUN_ID="provider_parity_$(date +%Y%m%d_%H%M%S)"
SCENARIO="llm_bootstrap"
SCENARIO_ID="P0-001"
PARITY_TIER="P0"
SAMPLES=3
TICKS=20
TIMEOUT_MS=15000
OUT_DIR=""
PROVIDER_BASE_URL="http://127.0.0.1:5841"
PROVIDER_AUTH_TOKEN=""
AGENT_PROVIDER_CONNECT_TIMEOUT_MS=15000
AGENT_PROVIDER_PROFILE="oasis7_p0_low_freq_npc"
PROVIDER_EXECUTION_MODE="headless_agent"
RUN_BUILTIN=1
RUN_PROVIDER=1

usage() {
  cat <<'USAGE'
Usage: ./scripts/provider-parity-p0.sh [options]

Run a repeatable P0 parity batch for builtin and/or the loopback provider.
This script emits protocol-aligned artifacts under output/provider_parity/<run_id>/.

Options:
  --run-id <id>                         Override benchmark run id
  --scenario <name>                     Scenario name (default: llm_bootstrap)
  --scenario-id <P0-001..P0-005>        Parity scenario id (default: P0-001)
  --parity-tier <P0|P1|P2>              Tier label (default: P0)
  --samples <n>                         Sample count per provider (default: 3)
  --ticks <n>                           Ticks per sample (default: 20)
  --timeout-ms <n>                      Timeout budget per sample (default: 15000)
  --out-dir <path>                      Artifact root (default: output/provider_parity/<run_id>)
  --agent-provider-url <url>             local provider local HTTP base URL
  --agent-provider-auth-token <token>         local provider bearer token
  --agent-provider-connect-timeout-ms <n>     local provider connect timeout (default: 15000)
  --agent-provider-profile <id>          local provider gameplay profile/skill id
  --execution-mode <mode>                local provider execution mode (default: headless_agent)
  --builtin-only                        Run only builtin provider
  --provider-only                       Run only the local provider-backed loopback provider
  -h, --help                            Show help

Notes:
  - builtin runs require the usual builtin LLM env (for example OPENAI_API_KEY).
  - provider runs require a real local provider exposing /v1/provider/info, /v1/provider/health,
    /v1/world-simulator/decision-context and /v1/world-simulator/feedback-context.
    These target outer-cognition routes are recorded in T4/T5 provider artifacts.
    Bare /v1/world-simulator/decision and /v1/world-simulator/feedback are legacy compatibility-only
    routes and are excluded from target cognition proof.
  - This script prepares T4/T5 parity evidence; it does not auto-sign QA/producer scorecards.
    benchmark_status is execution/sample coverage only. When both providers are present,
    parity_status and release_gate expose the machine-readable PRD gate disposition;
    profile/fixture binding and relative wait/latency class are required for a passed gate.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run-id)
      RUN_ID="${2:-}"
      shift 2
      ;;
    --scenario)
      SCENARIO="${2:-}"
      shift 2
      ;;
    --scenario-id)
      SCENARIO_ID="${2:-}"
      shift 2
      ;;
    --parity-tier)
      PARITY_TIER="${2:-}"
      shift 2
      ;;
    --samples)
      SAMPLES="${2:-}"
      shift 2
      ;;
    --ticks)
      TICKS="${2:-}"
      shift 2
      ;;
    --timeout-ms)
      TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --agent-provider-url)
      PROVIDER_BASE_URL="${2:-}"
      shift 2
      ;;
    --agent-provider-auth-token)
      PROVIDER_AUTH_TOKEN="${2:-}"
      shift 2
      ;;
    --agent-provider-connect-timeout-ms)
      AGENT_PROVIDER_CONNECT_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --agent-provider-profile)
      AGENT_PROVIDER_PROFILE="${2:-}"
      shift 2
      ;;
    --execution-mode)
      PROVIDER_EXECUTION_MODE="${2:-}"
      shift 2
      ;;
    --builtin-only)
      RUN_BUILTIN=1
      RUN_PROVIDER=0
      shift
      ;;
    --provider-only)
      RUN_BUILTIN=0
      RUN_PROVIDER=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

[[ -n "$RUN_ID" ]] || { echo "error: --run-id cannot be empty" >&2; exit 1; }
[[ "$SAMPLES" =~ ^[0-9]+$ ]] || { echo "error: --samples must be numeric" >&2; exit 1; }
[[ "$TICKS" =~ ^[0-9]+$ ]] || { echo "error: --ticks must be numeric" >&2; exit 1; }
[[ "$TIMEOUT_MS" =~ ^[0-9]+$ ]] || { echo "error: --timeout-ms must be numeric" >&2; exit 1; }
[[ "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" =~ ^[0-9]+$ ]] || { echo "error: --agent-provider-connect-timeout-ms must be numeric" >&2; exit 1; }
[[ -n "$AGENT_PROVIDER_PROFILE" ]] || { echo "error: --agent-provider-profile cannot be empty" >&2; exit 1; }
[[ "$PROVIDER_EXECUTION_MODE" == "headless_agent" || "$PROVIDER_EXECUTION_MODE" == "player_parity" ]] || { echo "error: --execution-mode must be headless_agent or player_parity" >&2; exit 1; }

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="output/provider_parity/$RUN_ID"
fi
mkdir -p "$OUT_DIR/raw" "$OUT_DIR/summary" "$OUT_DIR/samples"

run_sample() {
  local provider=$1
  local sample_index=$2
  local sample_dir="$OUT_DIR/samples/$provider/sample_$sample_index"
  mkdir -p "$sample_dir"

  local cmd=(oasis7_cargo_dev run -p oasis7 --bin oasis7_provider_parity_bench --
    --provider "$provider"
    --scenario "$SCENARIO"
    --scenario-id "$SCENARIO_ID"
    --parity-tier "$PARITY_TIER"
    --benchmark-run-id "$RUN_ID"
    --fixture-id "${SCENARIO_ID}_sample_${sample_index}"
    --ticks "$TICKS"
    --timeout-ms "$TIMEOUT_MS"
    --out-dir "$sample_dir")

  if [[ "$provider" == "provider_loopback_http" ]]; then
    cmd+=(--agent-provider-url "$PROVIDER_BASE_URL")
    if [[ -n "$PROVIDER_AUTH_TOKEN" ]]; then
      cmd+=(--agent-provider-auth-token "$PROVIDER_AUTH_TOKEN")
    fi
    cmd+=(--agent-provider-connect-timeout-ms "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS")
    cmd+=(--agent-provider-profile "$AGENT_PROVIDER_PROFILE")
    cmd+=(--execution-mode "$PROVIDER_EXECUTION_MODE")
  fi

  echo "+ ${cmd[*]}"
  "${cmd[@]}" | tee "$sample_dir/run.log"
}

if [[ "${PROVIDER_PARITY_P0_AGGREGATE_ONLY:-0}" != "1" ]]; then
  if (( RUN_BUILTIN )); then
    for sample_index in $(seq 1 "$SAMPLES"); do
      run_sample builtin "$sample_index"
    done
  fi

  if (( RUN_PROVIDER )); then
    for sample_index in $(seq 1 "$SAMPLES"); do
      run_sample provider_loopback_http "$sample_index"
    done
  fi
fi

python3 - "$OUT_DIR" "$RUN_ID" "$SCENARIO_ID" "$PARITY_TIER" "$SAMPLES" "$RUN_BUILTIN" "$RUN_PROVIDER" "$AGENT_PROVIDER_PROFILE" "$TIMEOUT_MS" "$TICKS" <<'PY'
import csv
import json
import math
import pathlib
import statistics
import sys

out_dir = pathlib.Path(sys.argv[1])
run_id = sys.argv[2]
scenario_id = sys.argv[3]
parity_tier = sys.argv[4]
requested_samples = int(sys.argv[5])
run_builtin = int(sys.argv[6])
run_provider = int(sys.argv[7])
configured_agent_profile = sys.argv[8]
requested_timeout_ms = int(sys.argv[9])
requested_ticks = int(sys.argv[10])

providers = []
if run_builtin:
    providers.append("builtin")
if run_provider:
    providers.append("provider_loopback_http")

summary_dir = out_dir / "summary"
summary_dir.mkdir(parents=True, exist_ok=True)

aggregate = {}
def nested_get(payload, path, default=0):
    current = payload
    for key in path:
        if not isinstance(current, dict):
            return default
        current = current.get(key)
        if current is None:
            return default
    return current

def runtime_perf_tick_peak(samples, path):
    values = []
    for sample in samples:
        tick = nested_get(sample, ("runtime_perf", "tick"), {})
        if not isinstance(tick, dict):
            continue
        samples_total = tick.get("samples_total", 0)
        if not isinstance(samples_total, (int, float)) or samples_total <= 0:
            continue
        value = nested_get(sample, path, 0)
        if isinstance(value, bool):
            continue
        if isinstance(value, (int, float)) and math.isfinite(value):
            values.append(value)
    return max(values) if values else None

def runtime_perf_tick_coverage_sample_count(samples):
    count = 0
    for sample in samples:
        tick = nested_get(sample, ("runtime_perf", "tick"), {})
        if not isinstance(tick, dict):
            continue
        samples_total = tick.get("samples_total", 0)
        if isinstance(samples_total, bool):
            continue
        if isinstance(samples_total, (int, float)) and samples_total > 0:
            count += 1
    return count

for provider in providers:
    sample_files = sorted((out_dir / "samples" / provider).glob("sample_*/summary/*.json"))
    samples = [json.loads(path.read_text()) for path in sample_files]
    valid_samples = [s for s in samples if s.get("status") != "invalid_fixture"]
    sample_status_counts = {}
    for sample in samples:
      status = str(sample.get("status") or "missing").strip().lower()
      sample_status_counts[status] = sample_status_counts.get(status, 0) + 1
    successful_sample_statuses = {"passed"}
    sample_statuses_complete = bool(valid_samples) and all(
      str(sample.get("status") or "").strip().lower() in successful_sample_statuses
      for sample in valid_samples
    )
    sample_outcomes_complete = sample_statuses_complete and all(
      sample.get("goal_completed") is True for sample in valid_samples
    )
    completion_rate = 0.0 if not valid_samples else sum(1 for s in valid_samples if s["goal_completed"]) / len(valid_samples)
    total_decision_steps = sum(s["decision_steps"] for s in valid_samples)
    invalid_action_rate = 0.0 if total_decision_steps == 0 else sum(s["invalid_action_count"] for s in valid_samples) / total_decision_steps
    timeout_rate = 0.0 if total_decision_steps == 0 else sum(s["timeout_count"] for s in valid_samples) / total_decision_steps
    recoverable_total = sum(s["recoverable_error_count"] for s in valid_samples)
    recoverable_resolution_rate = 1.0 if recoverable_total == 0 else sum(1 for s in valid_samples if s["goal_completed"]) / len(valid_samples)
    median_wait = 0 if not valid_samples else round(statistics.median(s["median_latency_ms"] for s in valid_samples))
    p95_wait = 0 if not valid_samples else max(s["p95_latency_ms"] for s in valid_samples)
    trace_completeness = 0.0 if not valid_samples else sum(s["trace_completeness_ratio_ppm"] for s in valid_samples) / len(valid_samples) / 1_000_000.0
    context_drift_count = sum(s.get("context_drift_count", 0) for s in valid_samples)
    runtime_perf_tick_p95_ms_peak = runtime_perf_tick_peak(valid_samples, ("runtime_perf", "tick", "p95_ms"))
    runtime_perf_tick_over_budget_ratio_ppm_peak = runtime_perf_tick_peak(valid_samples, ("runtime_perf", "tick", "over_budget_ratio_ppm"))
    runtime_perf_tick_coverage_sample_count_value = runtime_perf_tick_coverage_sample_count(valid_samples)
    warnings = []
    if valid_samples and runtime_perf_tick_coverage_sample_count_value == 0:
      warnings.append("runtime_perf_missing")
    error_codes = {}
    for sample in samples:
      for code, count in sample.get("error_counts", {}).items():
        error_codes[code] = error_codes.get(code, 0) + count
    benchmark_status = "insufficient_data"
    if len(valid_samples) >= requested_samples:
      if error_codes.get("session_cross_talk", 0) > 0 or "blocked" in sample_status_counts:
        benchmark_status = "blocked"
      elif not sample_outcomes_complete:
        benchmark_status = "failed"
      else:
        benchmark_status = "passed"

    metadata_source = valid_samples[0] if valid_samples else (samples[0] if samples else {})
    provider_metadata = metadata_source.get("provider", {})
    if not isinstance(provider_metadata, dict):
      provider_metadata = {}
    cognition_lane = provider_metadata.get("cognition_lane", "unknown")
    decision_route = provider_metadata.get("decision_route", "unknown")
    feedback_route = provider_metadata.get("feedback_route", "unknown")
    profile_values = []
    for sample in valid_samples:
      sample_provider = sample.get("provider")
      if isinstance(sample_provider, dict):
        profile = sample_provider.get("agent_profile")
        if isinstance(profile, str) and profile.strip():
          profile_values.append(profile.strip())
    if provider == "builtin":
      profile_binding_status = "configured_baseline"
      observed_agent_profile = configured_agent_profile
    else:
      observed_agent_profile = profile_values[0] if profile_values else None
      profile_binding_status = (
        "matched"
        if valid_samples and len(profile_values) == len(valid_samples)
        and all(profile == configured_agent_profile for profile in profile_values)
        else "missing_or_mismatched"
      )
    target_route_contract_valid = bool(valid_samples) and all(
      isinstance(sample.get("provider"), dict)
      and sample["provider"].get("cognition_lane") == "target_outer_context_v1"
      and sample["provider"].get("decision_route") == "/v1/world-simulator/decision-context"
      and sample["provider"].get("feedback_route") == "/v1/world-simulator/feedback-context"
      for sample in valid_samples
    )
    aggregated = {
      "benchmark_run_id": run_id,
      "parity_tier": parity_tier,
      "scenario_id": scenario_id,
      "provider_kind": provider,
      "mode": metadata_source.get("mode", "unknown"),
      "observation_schema_version": metadata_source.get("observation_schema_version", "unknown"),
      "action_schema_version": metadata_source.get("action_schema_version", "unknown"),
      "environment_class": metadata_source.get("environment_class", "unknown"),
      "fallback_reason": metadata_source.get("fallback_reason"),
      "requested_timeout_ms": requested_timeout_ms,
      "requested_ticks": requested_ticks,
      "agent_profile": observed_agent_profile,
      "profile_binding_status": profile_binding_status,
      "sample_count": len(samples),
      "valid_samples": len(valid_samples),
      "invalid_fixture": len(samples) - len(valid_samples),
      "sample_status_counts": sample_status_counts,
      "completion_rate": completion_rate,
      "invalid_action_rate": invalid_action_rate,
      "timeout_rate": timeout_rate,
      "recoverable_error_resolution_rate": recoverable_resolution_rate,
      "median_extra_wait_ms": median_wait,
      "p95_extra_wait_ms": p95_wait,
      "median_latency_ms": median_wait,
      "p95_latency_ms": p95_wait,
      "relative_wait_gap_median_ms": None,
      "relative_wait_gap_p95_ms": None,
      "latency_class": None,
      "trace_completeness": trace_completeness,
      "context_drift_count": context_drift_count,
      "runtime_perf": {
        "tick": {
          "coverage_sample_count": runtime_perf_tick_coverage_sample_count_value,
          "p95_ms_peak": runtime_perf_tick_p95_ms_peak,
          "over_budget_ratio_ppm_peak": runtime_perf_tick_over_budget_ratio_ppm_peak,
        },
      },
      "warnings": warnings,
      "benchmark_status": benchmark_status,
      "parity_status": "blocked",
      "release_gate": "blocked",
      "parity_gate": {
        "status": "blocked",
        "passed": False,
        "checks": {},
      },
      "error_counts": error_codes,
      "provider_version": valid_samples[0]["provider_version"] if valid_samples else "unknown",
      "adapter_version": valid_samples[0]["adapter_version"] if valid_samples else "unknown",
      "protocol_version": valid_samples[0]["protocol_version"] if valid_samples else "unknown",
      "cognition_lane": cognition_lane,
      "decision_route": decision_route,
      "feedback_route": feedback_route,
      "sample_summaries": [str(path) for path in sample_files],
    }
    if provider == "provider_loopback_http" and not target_route_contract_valid:
      aggregated["warnings"].append("target_outer_cognition_route_missing")
      aggregated["benchmark_status"] = "blocked"
    aggregate[provider] = aggregated

def sample_identity(sample):
    run_id = sample.get("benchmark_run_id")
    parity_tier = sample.get("parity_tier")
    scenario_id = sample.get("scenario_id")
    fixture_id = sample.get("fixture_id")
    seed = sample.get("seed")
    if any(
      not isinstance(value, str) or not value.strip()
      for value in (run_id, parity_tier, scenario_id, fixture_id)
    ) or seed is None:
      return None
    return (run_id.strip(), parity_tier.strip(), scenario_id.strip(), fixture_id.strip(), str(seed))

def build_fixture_binding_status(left, right):
    if not left or not right:
      return "missing_provider"
    left_identities = {
      identity for identity in (sample_identity(sample) for sample in left)
      if identity is not None
    }
    right_identities = {
      identity for identity in (sample_identity(sample) for sample in right)
      if identity is not None
    }
    if len(left_identities) != len(left) or len(right_identities) != len(right):
      return "missing_or_invalid_identity"
    return "matched" if left_identities == right_identities else "mismatched"

def make_gate_check(value, limit, *, passed):
    return {"value": value, "limit": limit, "passed": bool(passed)}

builtin = aggregate.get("builtin")
provider_summary = aggregate.get("provider_loopback_http")
fixture_binding_status = "missing_provider"
if builtin is not None and provider_summary is not None:
    builtin_samples = []
    for path in sorted((out_dir / "samples" / "builtin").glob("sample_*/summary/*.json")):
      sample = json.loads(path.read_text())
      if sample.get("status") != "invalid_fixture":
        builtin_samples.append(sample)
    provider_samples = []
    for path in sorted((out_dir / "samples" / "provider_loopback_http").glob("sample_*/summary/*.json")):
      sample = json.loads(path.read_text())
      if sample.get("status") != "invalid_fixture":
        provider_samples.append(sample)
    fixture_binding_status = build_fixture_binding_status(builtin_samples, provider_samples)
else:
    fixture_binding_status = "missing_provider"

for summary in aggregate.values():
    summary["fixture_binding_status"] = fixture_binding_status

if builtin is not None and provider_summary is not None:
    builtin_median = builtin["median_latency_ms"]
    builtin_p95 = builtin["p95_latency_ms"]
    provider_median = provider_summary["median_latency_ms"]
    provider_p95 = provider_summary["p95_latency_ms"]
    if isinstance(builtin_median, (int, float)) and isinstance(provider_median, (int, float)):
      relative_median = max(0, provider_median - builtin_median)
    else:
      relative_median = None
    if isinstance(builtin_p95, (int, float)) and isinstance(provider_p95, (int, float)):
      relative_p95 = max(0, provider_p95 - builtin_p95)
    else:
      relative_p95 = None
    builtin["relative_wait_gap_median_ms"] = 0
    builtin["relative_wait_gap_p95_ms"] = 0
    provider_summary["relative_wait_gap_median_ms"] = relative_median
    provider_summary["relative_wait_gap_p95_ms"] = relative_p95
    # Keep the established extra-wait fields as the Local Provider's absolute
    # observed waits.  Relative gaps are the behavior gate; latency class is
    # the rollout gate over the provider's absolute waits.
    provider_summary["median_extra_wait_ms"] = provider_median
    provider_summary["p95_extra_wait_ms"] = provider_p95

    if provider_median is not None and provider_p95 is not None:
      if provider_median <= 500 and provider_p95 <= 1500:
        latency_class = "A"
      elif provider_median <= 15000 and provider_p95 <= 20000:
        latency_class = "B"
      else:
        latency_class = "C"
    else:
      latency_class = None
    provider_summary["latency_class"] = latency_class
    builtin["latency_class"] = None

    completion_gap = abs(provider_summary["completion_rate"] - builtin["completion_rate"])
    builtin_invalid_rate = builtin["invalid_action_rate"]
    provider_invalid_rate = provider_summary["invalid_action_rate"]
    invalid_action_pass = (
      provider_invalid_rate <= 0.03
      and provider_invalid_rate <= builtin_invalid_rate * 2
    )
    checks = {
      "sample_status": make_gate_check(
        provider_summary["benchmark_status"], "passed", passed=(
          builtin["benchmark_status"] == "passed"
          and provider_summary["benchmark_status"] == "passed"
        )
      ),
      "fixture_binding": make_gate_check(fixture_binding_status, "matched", passed=fixture_binding_status == "matched"),
      "profile_binding": make_gate_check(
        provider_summary["profile_binding_status"], "matched", passed=provider_summary["profile_binding_status"] == "matched"
      ),
      "completion_rate_gap": make_gate_check(completion_gap, 0.05, passed=completion_gap <= 0.05),
      "invalid_action_rate": make_gate_check(provider_invalid_rate, {"absolute": 0.03, "relative_to_builtin": 2}, passed=invalid_action_pass),
      "timeout_rate": make_gate_check(provider_summary["timeout_rate"], 0.02, passed=provider_summary["timeout_rate"] <= 0.02),
      "relative_wait_gap_median_ms": make_gate_check(relative_median, 5000, passed=relative_median is not None and relative_median <= 5000),
      "relative_wait_gap_p95_ms": make_gate_check(relative_p95, 8000, passed=relative_p95 is not None and relative_p95 <= 8000),
      "trace_completeness": make_gate_check(provider_summary["trace_completeness"], 0.95, passed=provider_summary["trace_completeness"] >= 0.95),
      "recoverable_error_resolution_rate": make_gate_check(provider_summary["recoverable_error_resolution_rate"], 0.90, passed=provider_summary["recoverable_error_resolution_rate"] >= 0.90),
    }
    gate_passed = all(check["passed"] for check in checks.values())
    parity_status = "passed" if gate_passed else (
      "blocked" if fixture_binding_status != "matched" or provider_summary["profile_binding_status"] != "matched" or builtin["benchmark_status"] != "passed" or provider_summary["benchmark_status"] in {"insufficient_data", "blocked"}
      else "failed"
    )
    provider_summary["parity_gate"] = {
      "status": parity_status,
      "passed": gate_passed,
      "checks": checks,
    }
    provider_summary["parity_status"] = parity_status
    if parity_status == "blocked":
      provider_summary["release_gate"] = "blocked"
    elif latency_class == "C":
      provider_summary["release_gate"] = "blocked"
    elif parity_status != "passed" or latency_class == "B":
      provider_summary["release_gate"] = "experimental_only"
    else:
      provider_summary["release_gate"] = "default_candidate"
    builtin["parity_gate"] = provider_summary["parity_gate"]
    builtin["parity_status"] = parity_status
    builtin["release_gate"] = provider_summary["release_gate"]

for provider, aggregated in aggregate.items():
    out_path = summary_dir / f"{scenario_id}.{provider}.json"
    out_path.write_text(json.dumps(aggregated, ensure_ascii=False, indent=2) + "\n")

combined_csv = summary_dir / "combined.csv"
with combined_csv.open("w", newline="") as handle:
    writer = csv.writer(handle)
    writer.writerow(["metric", "builtin", "provider_loopback_http", "gap_or_note"])
    metrics = [
      "completion_rate",
      "invalid_action_rate",
      "timeout_rate",
      "median_extra_wait_ms",
      "p95_extra_wait_ms",
      "mode",
      "observation_schema_version",
      "action_schema_version",
      "environment_class",
      "fallback_reason",
      "agent_profile",
      "profile_binding_status",
      "fixture_binding_status",
      "trace_completeness",
      "relative_wait_gap_median_ms",
      "relative_wait_gap_p95_ms",
      "latency_class",
      "recoverable_error_resolution_rate",
      "context_drift_count",
      "runtime_perf.tick.coverage_sample_count",
      "runtime_perf.tick.p95_ms_peak",
      "runtime_perf.tick.over_budget_ratio_ppm_peak",
      "warnings",
      "benchmark_status",
      "parity_status",
      "release_gate",
    ]
    builtin = aggregate.get("builtin", {})
    provider_summary = aggregate.get("provider_loopback_http", {})
    for metric in metrics:
        left = nested_get(builtin, metric.split("."), "") if metric.startswith("runtime_perf.") else builtin.get(metric, "")
        right = nested_get(provider_summary, metric.split("."), "") if metric.startswith("runtime_perf.") else provider_summary.get(metric, "")
        if isinstance(left, (int, float)) and isinstance(right, (int, float)):
            gap = right - left
        else:
            gap = "compare_manually"
        if isinstance(left, list):
            left = ";".join(str(item) for item in left)
        if isinstance(right, list):
            right = ";".join(str(item) for item in right)
        writer.writerow([metric, left, right, gap])

failures_md = summary_dir / "failures.md"
with failures_md.open("w") as handle:
    handle.write(f"# Failures for {run_id}\n\n")
    for provider in providers:
        summary = aggregate.get(provider, {})
        handle.write(f"## {provider}\n")
        handle.write(f"- benchmark_status: {summary.get('benchmark_status', 'unknown')}\n")
        handle.write(f"- parity_status: {summary.get('parity_status', 'unknown')}\n")
        handle.write(f"- release_gate: {summary.get('release_gate', 'unknown')}\n")
        handle.write(f"- agent_profile: {summary.get('agent_profile', 'unknown')}\n")
        handle.write(f"- profile_binding_status: {summary.get('profile_binding_status', 'unknown')}\n")
        handle.write(f"- fixture_binding_status: {summary.get('fixture_binding_status', 'unknown')}\n")
        handle.write(f"- latency_class: {summary.get('latency_class', 'unknown')}\n")
        handle.write(
            f"- relative_wait_gap_median_ms: {summary.get('relative_wait_gap_median_ms', 'unknown')}\n"
        )
        handle.write(
            f"- relative_wait_gap_p95_ms: {summary.get('relative_wait_gap_p95_ms', 'unknown')}\n"
        )
        for code, count in sorted(summary.get("error_counts", {}).items()):
            handle.write(f"- {code}: {count}\n")
        if not summary.get("error_counts"):
            handle.write("- no error signatures recorded\n")
        handle.write("\n")

scorecard_links = out_dir / "scorecard-links.md"
with scorecard_links.open("w") as handle:
    handle.write(f"# Scorecard Links for {run_id}\n\n")
    handle.write("- QA 评分卡路径: doc/world-simulator/prd/acceptance/provider-agent-parity-score-card.md\n")
    handle.write("- Producer 评分卡路径: doc/world-simulator/prd/acceptance/provider-agent-parity-score-card.md\n")
    handle.write(f"- 自动 benchmark 证据路径: {summary_dir}\n")
    handle.write(f"- 样本输出根目录: {out_dir / 'samples'}\n")
PY

echo "artifacts written to: $OUT_DIR"
echo "combined csv: $OUT_DIR/summary/combined.csv"
echo "failures md: $OUT_DIR/summary/failures.md"
echo "scorecard links: $OUT_DIR/scorecard-links.md"
