#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

all_steps=(candidate_bundle ci_full sync_m1 sync_m4 sync_m5 web_strict s9 s10)

usage() {
  cat <<'USAGE'
Usage: ./scripts/release-gate.sh [options]

Purpose:
  Run release gate closure for external publish:
  - ci full tier
  - builtin wasm sync checks (m1/m4/m5)
  - primary-entry + software-safe web strict loop
  - S9/S10 longrun gate

Options:
  --out-dir <path>               Output root (default: .tmp/release_gate)
  --candidate-bundle <path>      Validate one release candidate bundle before running gate
  --quick                        Quick profile: shorter S9/S10 durations
  --dry-run                      Print and record commands only (no execution)
  --dry-run-fail-step <step>     Simulate a failure step in dry-run mode for hint validation
  --skip-ci-full                 Skip `./scripts/ci-tests.sh full`
  --skip-sync                    Skip sync-m1/m4/m5 checks
  --skip-web-strict              Skip web strict loop
  --skip-s9                      Skip S9 soak gate
  --skip-s10                     Skip S10 soak gate
  --web-scenario <name>          Scenario for web strict loop (default: llm_bootstrap)
  --web-headed                   Run web loop in headed mode
  --s9-duration-secs <n>         S9 release gate duration (default: 300)
  --s9-out-dir <path>            S9 output root (default: .tmp/release_gate_p2p)
  --s9-dry-run                   Pass --dry-run to S9 script
  --s10-duration-secs <n>        S10 release gate duration (default: 300)
  --s10-out-dir <path>           S10 output root (default: .tmp/release_gate_s10)
  --s10-dry-run                  Pass --dry-run to S10 script
  -h, --help                     Show help

Steps:
  candidate_bundle | ci_full | sync_m1 | sync_m4 | sync_m5 | web_strict | s9 | s10
USAGE
}

ensure_positive_int() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^[0-9]+$ ]] || (( value <= 0 )); then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

is_supported_step() {
  local step=$1
  local candidate=""
  for candidate in "${all_steps[@]}"; do
    if [[ "$candidate" == "$step" ]]; then
      return 0
    fi
  done
  return 1
}

format_cmd() {
  local formatted=""
  local token=""
  for token in "$@"; do
    local quoted=""
    printf -v quoted '%q' "$token"
    if [[ -z "$formatted" ]]; then
      formatted="$quoted"
    else
      formatted="$formatted $quoted"
    fi
  done
  printf '%s' "$formatted"
}

declare -A step_status=()
declare -A step_note=()
declare -A step_log=()
declare -A step_cmd=()

dry_run=0
dry_run_fail_step=""
out_dir=".tmp/release_gate"
quick=0
candidate_bundle=""

skip_ci_full=0
skip_sync=0
skip_web_strict=0
skip_s9=0
skip_s10=0

web_scenario="llm_bootstrap"
web_headed=0

s9_duration_secs=300
s9_duration_user_set=0
s9_out_dir=".tmp/release_gate_p2p"
s9_dry_run=0

s10_duration_secs=300
s10_duration_user_set=0
s10_out_dir=".tmp/release_gate_s10"
s10_dry_run=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --candidate-bundle)
      candidate_bundle=${2:-}
      shift 2
      ;;
    --quick)
      quick=1
      shift
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --dry-run-fail-step)
      dry_run_fail_step=${2:-}
      shift 2
      ;;
    --skip-ci-full)
      skip_ci_full=1
      shift
      ;;
    --skip-sync)
      skip_sync=1
      shift
      ;;
    --skip-web-strict)
      skip_web_strict=1
      shift
      ;;
    --skip-s9)
      skip_s9=1
      shift
      ;;
    --skip-s10)
      skip_s10=1
      shift
      ;;
    --web-scenario)
      web_scenario=${2:-}
      shift 2
      ;;
    --web-headed)
      web_headed=1
      shift
      ;;
    --s9-duration-secs)
      s9_duration_secs=${2:-}
      s9_duration_user_set=1
      shift 2
      ;;
    --s9-out-dir)
      s9_out_dir=${2:-}
      shift 2
      ;;
    --s9-dry-run)
      s9_dry_run=1
      shift
      ;;
    --s10-duration-secs)
      s10_duration_secs=${2:-}
      s10_duration_user_set=1
      shift 2
      ;;
    --s10-out-dir)
      s10_out_dir=${2:-}
      shift 2
      ;;
    --s10-dry-run)
      s10_dry_run=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$quick" -eq 1 ]]; then
  if [[ "$s9_duration_user_set" -eq 0 ]]; then
    s9_duration_secs=60
  fi
  if [[ "$s10_duration_user_set" -eq 0 ]]; then
    s10_duration_secs=60
  fi
fi

if [[ -n "$dry_run_fail_step" ]]; then
  if [[ "$dry_run" -ne 1 ]]; then
    echo "error: --dry-run-fail-step requires --dry-run" >&2
    exit 2
  fi
  if ! is_supported_step "$dry_run_fail_step"; then
    echo "error: unsupported --dry-run-fail-step: $dry_run_fail_step" >&2
    echo "supported: ${all_steps[*]}" >&2
    exit 2
  fi
fi

ensure_positive_int "--s9-duration-secs" "$s9_duration_secs"
ensure_positive_int "--s10-duration-secs" "$s10_duration_secs"

timestamp=$(date '+%Y%m%d-%H%M%S')
run_dir="$out_dir/$timestamp"
summary_path="$run_dir/release-gate-summary.md"
mkdir -p "$run_dir"

selected_steps=()
if [[ -n "$candidate_bundle" ]]; then
  selected_steps+=(candidate_bundle)
fi
if [[ "$skip_ci_full" -eq 0 ]]; then
  selected_steps+=(ci_full)
fi
if [[ "$skip_sync" -eq 0 ]]; then
  selected_steps+=(sync_m1 sync_m4 sync_m5)
fi
if [[ "$skip_web_strict" -eq 0 ]]; then
  selected_steps+=(web_strict)
fi
if [[ "$skip_s9" -eq 0 ]]; then
  selected_steps+=(s9)
fi
if [[ "$skip_s10" -eq 0 ]]; then
  selected_steps+=(s10)
fi

if [[ "${#selected_steps[@]}" -eq 0 ]]; then
  echo "error: no steps selected for release gate" >&2
  echo "hint: remove at least one --skip-* option or run with defaults" >&2
  exit 2
fi

step=""
for step in "${all_steps[@]}"; do
  step_status["$step"]="skipped"
  step_note["$step"]="not scheduled"
  step_log["$step"]="$run_dir/$step.log"
  step_cmd["$step"]=""
done

run_step() {
  local step_name=$1
  shift
  local -a cmd=("$@")
  local cmd_rendered=""
  local step_log_path="${step_log[$step_name]}"
  local code=0

  cmd_rendered="$(format_cmd "${cmd[@]}")"
  step_cmd["$step_name"]="$cmd_rendered"

  {
    echo "step=$step_name"
    echo "started_at=$(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "command=$cmd_rendered"
  } >"$step_log_path"

  if [[ "$dry_run" -eq 1 ]]; then
    echo "+ $cmd_rendered (dry-run)"
    echo "mode=dry_run" >>"$step_log_path"
    if [[ -n "$dry_run_fail_step" && "$dry_run_fail_step" == "$step_name" ]]; then
      step_status["$step_name"]="failed"
      step_note["$step_name"]="simulated_dry_run_failure"
      echo "result=simulated_failure" >>"$step_log_path"
      return 1
    fi
    step_status["$step_name"]="passed"
    step_note["$step_name"]="dry_run"
    echo "result=dry_run_pass" >>"$step_log_path"
    return 0
  fi

  set +e
  {
    echo "+ $cmd_rendered"
    "${cmd[@]}"
  } > >(tee -a "$step_log_path") 2>&1
  code=$?
  set -e

  if [[ "$code" -eq 0 ]]; then
    step_status["$step_name"]="passed"
    step_note["$step_name"]="ok"
    echo "result=ok" >>"$step_log_path"
    return 0
  fi

  step_status["$step_name"]="failed"
  step_note["$step_name"]="exit_code=$code"
  echo "result=failed" >>"$step_log_path"
  echo "exit_code=$code" >>"$step_log_path"
  return 1
}

write_summary() {
  local overall_label=$1
  {
    echo "# Release Gate Summary"
    echo ""
    echo "- Timestamp: $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "- Run dir: \`$run_dir\`"
    echo "- Dry run: \`$dry_run\`"
    echo "- Quick: \`$quick\`"
    echo "- Candidate bundle: \`${candidate_bundle:-none}\`"
    echo "- Overall: $overall_label"
    echo ""
    echo "## Step Status"
    for step in "${all_steps[@]}"; do
      echo "- $step: ${step_status[$step]} (${step_note[$step]})"
      if [[ -n "${step_cmd[$step]}" ]]; then
        echo "  - command: \`${step_cmd[$step]}\`"
      fi
      echo "  - log: \`${step_log[$step]}\`"
    done
  } >"$summary_path"
}

emit_failure_hints() {
  local failed_step_name=$1
  echo "error: release gate failed at step: $failed_step_name" >&2
  echo "hint: inspect step log: ${step_log[$failed_step_name]}" >&2
  if [[ -n "${step_cmd[$failed_step_name]}" ]]; then
    echo "hint: rerun step command: ${step_cmd[$failed_step_name]}" >&2
  fi
  echo "hint: gate summary: $summary_path" >&2
}

failed_step=""
for step in "${selected_steps[@]}"; do
  case "$step" in
    candidate_bundle)
      cmd=(
        ./scripts/release-candidate-bundle.sh
        validate
        --bundle "$candidate_bundle"
        --check-git-head
        --check-clean-worktree
      )
      ;;
    ci_full)
      cmd=(./scripts/ci-tests.sh full)
      ;;
    sync_m1)
      cmd=(./scripts/sync-m1-builtin-wasm-artifacts.sh --check)
      ;;
    sync_m4)
      cmd=(./scripts/sync-m4-builtin-wasm-artifacts.sh --check)
      ;;
    sync_m5)
      cmd=(./scripts/sync-m5-builtin-wasm-artifacts.sh --check)
      ;;
    web_strict)
      cmd=(./scripts/release-gate-web-strict.sh --scenario "$web_scenario" --out-dir "$run_dir/web_strict")
      if [[ "$web_headed" -eq 1 ]]; then
        cmd+=(--headed)
      fi
      ;;
    s9)
      cmd=(
        ./scripts/p2p-longrun-soak.sh
        --profile soak_release
        --topologies triad_distributed
        --duration-secs "$s9_duration_secs"
        --no-prewarm
        --max-stall-secs 240
        --max-lag-p95 50
        --max-distfs-failure-ratio 0.1
        --chaos-continuous-enable
        --chaos-continuous-interval-secs 30
        --chaos-continuous-start-sec 30
        --chaos-continuous-max-events 8
        --chaos-continuous-actions restart,pause
        --chaos-continuous-seed 1772284566
        --chaos-continuous-restart-down-secs 1
        --chaos-continuous-pause-duration-secs 2
        --out-dir "$s9_out_dir"
      )
      if [[ "$s9_dry_run" -eq 1 ]]; then
        cmd+=(--dry-run)
      fi
      ;;
    s10)
      cmd=(
        ./scripts/s10-five-node-game-soak.sh
        --duration-secs "$s10_duration_secs"
        --no-prewarm
        --max-stall-secs 240
        --max-lag-p95 50
        --out-dir "$s10_out_dir"
      )
      if [[ "$s10_dry_run" -eq 1 ]]; then
        cmd+=(--dry-run)
      fi
      ;;
    *)
      echo "error: internal unknown step: $step" >&2
      exit 2
      ;;
  esac

  if ! run_step "$step" "${cmd[@]}"; then
    failed_step="$step"
    break
  fi
done

overall_label="PASS"
if [[ -n "$failed_step" ]]; then
  overall_label="FAIL"
fi
write_summary "$overall_label"
echo "release gate summary: $summary_path"

if [[ -n "$failed_step" ]]; then
  emit_failure_hints "$failed_step"
  exit 1
fi
