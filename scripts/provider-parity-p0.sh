#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

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
  - provider runs require a real local provider exposing /v1/provider/info, /health,
    /world-simulator/decision and /feedback.
  - This script prepares T4/T5 parity evidence; it does not auto-sign QA/producer scorecards.
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

  local cmd=(env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_provider_parity_bench --
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

python3 - "$OUT_DIR" "$RUN_ID" "$SCENARIO_ID" "$PARITY_TIER" "$SAMPLES" "$RUN_BUILTIN" "$RUN_PROVIDER" <<'PY'
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

providers = []
if run_builtin:
    providers.append("builtin")
if run_provider:
    providers.append("provider_loopback_http")

summary_dir = out_dir / "summary"
summary_dir.mkdir(parents=True, exist_ok=True)

aggregate = {}
for provider in providers:
    sample_files = sorted((out_dir / "samples" / provider).glob("sample_*/summary/*.json"))
    samples = [json.loads(path.read_text()) for path in sample_files]
    valid_samples = [s for s in samples if s["status"] != "invalid_fixture"]
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
    error_codes = {}
    for sample in samples:
      for code, count in sample.get("error_counts", {}).items():
        error_codes[code] = error_codes.get(code, 0) + count
    benchmark_status = "insufficient_data"
    if len(valid_samples) >= requested_samples:
      benchmark_status = "passed" if completion_rate > 0.0 else "failed"
      if error_codes.get("session_cross_talk", 0) > 0:
        benchmark_status = "blocked"

    metadata_source = valid_samples[0] if valid_samples else (samples[0] if samples else {})
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
      "sample_count": len(samples),
      "valid_samples": len(valid_samples),
      "invalid_fixture": len(samples) - len(valid_samples),
      "completion_rate": completion_rate,
      "invalid_action_rate": invalid_action_rate,
      "timeout_rate": timeout_rate,
      "recoverable_error_resolution_rate": recoverable_resolution_rate,
      "median_extra_wait_ms": median_wait,
      "p95_extra_wait_ms": p95_wait,
      "trace_completeness": trace_completeness,
      "context_drift_count": context_drift_count,
      "benchmark_status": benchmark_status,
      "error_counts": error_codes,
      "provider_version": valid_samples[0]["provider_version"] if valid_samples else "unknown",
      "adapter_version": valid_samples[0]["adapter_version"] if valid_samples else "unknown",
      "protocol_version": valid_samples[0]["protocol_version"] if valid_samples else "unknown",
      "sample_summaries": [str(path) for path in sample_files],
    }
    aggregate[provider] = aggregated
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
      "trace_completeness",
      "recoverable_error_resolution_rate",
      "context_drift_count",
      "benchmark_status",
    ]
    builtin = aggregate.get("builtin", {})
    provider_summary = aggregate.get("provider_loopback_http", {})
    for metric in metrics:
        left = builtin.get(metric, "")
        right = provider_summary.get(metric, "")
        if isinstance(left, (int, float)) and isinstance(right, (int, float)):
            gap = right - left
        else:
            gap = "compare_manually"
        writer.writerow([metric, left, right, gap])

failures_md = summary_dir / "failures.md"
with failures_md.open("w") as handle:
    handle.write(f"# Failures for {run_id}\n\n")
    for provider in providers:
        summary = aggregate.get(provider, {})
        handle.write(f"## {provider}\n")
        handle.write(f"- benchmark_status: {summary.get('benchmark_status', 'unknown')}\n")
        for code, count in sorted(summary.get("error_counts", {}).items()):
            handle.write(f"- {code}: {count}\n")
        if not summary.get("error_counts"):
            handle.write("- no error signatures recorded\n")
        handle.write("\n")

scorecard_links = out_dir / "scorecard-links.md"
with scorecard_links.open("w") as handle:
    handle.write(f"# Scorecard Links for {run_id}\n\n")
    handle.write("- QA 评分卡路径: doc/world-simulator/prd/acceptance/provider-agent-parity-score-card-2026-03-12.md\n")
    handle.write("- Producer 评分卡路径: doc/world-simulator/prd/acceptance/provider-agent-parity-score-card-2026-03-12.md\n")
    handle.write(f"- 自动 benchmark 证据路径: {summary_dir}\n")
    handle.write(f"- 样本输出根目录: {out_dir / 'samples'}\n")
PY

echo "artifacts written to: $OUT_DIR"
echo "combined csv: $OUT_DIR/summary/combined.csv"
echo "failures md: $OUT_DIR/summary/failures.md"
echo "scorecard links: $OUT_DIR/scorecard-links.md"
