#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/game-world-state-sync-commit-module-required.sh [options]

Run or render the GWSC module_required command set for game-world state sync
and commit closure.

Options:
  --dry-run          record commands without executing them
  --out-dir <path>   output root (default: .tmp/game_world_state_sync_commit_module_required)
  --skip-cargo       skip the cargo-based required checks
  --skip-matrix      skip the mixed-topology required matrix
  -h, --help         show help

Claim boundary:
  - This wrapper only claims `module_required`.
  - It does not claim multi-node world sync, release_full, or public_testnet ready.
USAGE
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 127
  fi
}

out_root=".tmp/game_world_state_sync_commit_module_required"
dry_run=0
skip_cargo=0
skip_matrix=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      dry_run=1
      shift
      ;;
    --out-dir)
      [[ $# -ge 2 && -n "${2:-}" ]] || { echo "error: --out-dir requires a value" >&2; exit 2; }
      out_root=$2
      shift 2
      ;;
    --skip-cargo)
      skip_cargo=1
      shift
      ;;
    --skip-matrix)
      skip_matrix=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_command jq

run_id=$(date +"%Y%m%d-%H%M%S")
run_dir="$out_root/${run_id}-module_required"
steps_dir="$run_dir/steps"
steps_ndjson="$run_dir/steps.ndjson"
summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"

mkdir -p "$steps_dir"
: > "$steps_ndjson"

step_ids=(
  oasis7_required_tests
  oasis7_node
  oasis7_net_lib
  oasis7_net_libp2p_lib
  oasis7_consensus_lib
  oasis7_distfs_lib
  p2p_mixed_topology_required
)

step_kinds=(
  cargo
  cargo
  cargo
  cargo
  cargo
  cargo
  matrix
)

base_commands=(
  "env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required"
  "env -u RUSTC_WRAPPER cargo test -p oasis7_node"
  "env -u RUSTC_WRAPPER cargo test -p oasis7_net --lib"
  "env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib"
  "env -u RUSTC_WRAPPER cargo test -p oasis7_consensus --lib"
  "env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib"
  "./scripts/p2p-mixed-topology-matrix.sh --tier required"
)

should_skip_step() {
  local kind=$1
  [[ "$kind" == "cargo" && "$skip_cargo" -eq 1 ]] || [[ "$kind" == "matrix" && "$skip_matrix" -eq 1 ]]
}

render_command() {
  local kind=$1
  local base_command=$2
  local step_dir=$3

  if [[ "$kind" == "matrix" ]]; then
    local command="$base_command --out-dir $step_dir/matrix"
    if [[ "$dry_run" -eq 1 ]]; then
      command="$command --dry-run"
    fi
    printf '%s\n' "$command"
  else
    printf '%s\n' "$base_command"
  fi
}

append_step_record() {
  local step_id=$1
  local kind=$2
  local base_command=$3
  local command=$4
  local status=$5
  local exit_code=$6
  local started_at=$7
  local ended_at=$8
  local stdout_log=$9
  local stderr_log=${10}

  jq -n \
    --arg step_id "$step_id" \
    --arg kind "$kind" \
    --arg base_command "$base_command" \
    --arg command "$command" \
    --arg status "$status" \
    --arg started_at "$started_at" \
    --arg ended_at "$ended_at" \
    --arg stdout_log "$stdout_log" \
    --arg stderr_log "$stderr_log" \
    --argjson exit_code "$exit_code" \
    '{
      step_id: $step_id,
      kind: $kind,
      base_command: $base_command,
      command: $command,
      status: $status,
      exit_code: $exit_code,
      started_at: $started_at,
      ended_at: $ended_at,
      stdout_log: $stdout_log,
      stderr_log: $stderr_log
    }' >> "$steps_ndjson"
}

echo "GWSC module_required"
echo "- dry_run: $dry_run"
echo "- output: $run_dir"
echo "- skip_cargo: $skip_cargo"
echo "- skip_matrix: $skip_matrix"

for i in "${!step_ids[@]}"; do
  step_id=${step_ids[$i]}
  kind=${step_kinds[$i]}
  base_command=${base_commands[$i]}
  step_dir="$steps_dir/$step_id"
  stdout_log="$step_dir/stdout.log"
  stderr_log="$step_dir/stderr.log"
  mkdir -p "$step_dir"

  command=$(render_command "$kind" "$base_command" "$step_dir")
  printf '%s\n' "$base_command" > "$step_dir/base_command.txt"
  printf '%s\n' "$command" > "$step_dir/command.txt"

  started_at=$(date -Iseconds)
  status="dry_run"
  exit_code=0

  if should_skip_step "$kind"; then
    status="skipped"
    printf 'skipped by wrapper option\n' > "$stdout_log"
    : > "$stderr_log"
    echo "+ skip [$step_id]: $base_command"
  elif [[ "$dry_run" -eq 1 ]]; then
    printf 'dry-run only\n' > "$stdout_log"
    : > "$stderr_log"
    echo "+ dry-run [$step_id]: $command"
  else
    echo "+ [$step_id] $command"
    if bash -lc "$command" >"$stdout_log" 2>"$stderr_log"; then
      status="ok"
      exit_code=0
    else
      status="failed"
      exit_code=$?
    fi
  fi

  ended_at=$(date -Iseconds)
  append_step_record "$step_id" "$kind" "$base_command" "$command" "$status" "$exit_code" "$started_at" "$ended_at" "$stdout_log" "$stderr_log"
done

generated_at=$(date -Iseconds)

jq -s \
  --arg generated_at "$generated_at" \
  --arg run_dir "$run_dir" \
  --arg summary_md "$summary_md" \
  --argjson dry_run "$dry_run" \
  --argjson skip_cargo "$skip_cargo" \
  --argjson skip_matrix "$skip_matrix" \
  '{
    generated_at: $generated_at,
    run_dir: $run_dir,
    summary_md: $summary_md,
    claim_boundary: "module_required",
    does_not_claim: [
      "multi-node world sync",
      "release_full",
      "public_testnet ready"
    ],
    dry_run: ($dry_run == 1),
    skips: {
      cargo: ($skip_cargo == 1),
      matrix: ($skip_matrix == 1)
    },
    steps: .,
    totals: {
      step_count: length,
      cargo_step_count: (map(select(.kind == "cargo")) | length),
      matrix_step_count: (map(select(.kind == "matrix")) | length),
      ok_count: (map(select(.status == "ok")) | length),
      failed_count: (map(select(.status == "failed")) | length),
      dry_run_count: (map(select(.status == "dry_run")) | length),
      skipped_count: (map(select(.status == "skipped")) | length)
    },
    overall_status: (
      if (map(select(.status == "failed")) | length) > 0 then
        "failed"
      elif (map(select(.status == "dry_run")) | length) > 0 then
        "dry_run"
      elif (map(select(.status == "ok")) | length) > 0 then
        "ok"
      else
        "skipped"
      end
    ),
    evidence_contract: {
      module_required_ready: (
        ($dry_run == 0)
        and ((map(select(.status == "failed")) | length) == 0)
        and ((map(select(.status == "skipped")) | length) == 0)
        and ((map(select(.status == "ok")) | length) == length)
      ),
      boundary_note: "module_required only; stronger GWSC claims require module_full/integration_required/release_full evidence"
    }
  }' "$steps_ndjson" > "$summary_json"

{
  echo "# GWSC module_required Summary"
  echo
  echo "- generated_at: \`$(jq -r '.generated_at' "$summary_json")\`"
  echo "- overall_status: \`$(jq -r '.overall_status' "$summary_json")\`"
  echo "- claim_boundary: \`$(jq -r '.claim_boundary' "$summary_json")\`"
  echo "- does_not_claim: \`$(jq -r '.does_not_claim | join(", ")' "$summary_json")\`"
  echo "- run_dir: \`$run_dir\`"
  echo
  echo "| step | kind | status | command |"
  echo "| --- | --- | --- | --- |"
  jq -r '.steps[] | [ .step_id, .kind, .status, .command ] | @tsv' "$summary_json" |
    while IFS=$'\t' read -r step_id kind status command; do
      echo "| $step_id | $kind | $status | \`$command\` |"
    done
} > "$summary_md"

echo "- summary: $summary_json"
echo "- summary_md: $summary_md"

if [[ "$(jq -r '.overall_status' "$summary_json")" == "failed" ]]; then
  exit 1
fi
