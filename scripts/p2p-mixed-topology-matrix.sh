#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-mixed-topology-matrix.sh [options]

Build or execute the P2PARCH-6 mixed-topology validation matrix.

Options:
  --tier <tier>     required | full (default: required)
  --out-dir <path>  output root (default: .tmp/p2p_mixed_topology)
  --dry-run         render commands and summary only
  -h, --help        show help

Notes:
  - `required` runs deterministic exact-coverage cases only.
  - `full` runs the required set plus proxy longrun cases that exercise
    distributed recovery without claiming a dedicated sentry/NAT lab.
  - Artifacts are written to:
      <out-dir>/<timestamp>-<tier>/{summary.json,summary.md,cases/*}
USAGE
}

tier="required"
out_root=".tmp/p2p_mixed_topology"
dry_run=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tier)
      tier=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --dry-run)
      dry_run=1
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

case "$tier" in
  required|full) ;;
  *)
    echo "invalid --tier: $tier (expected required|full)" >&2
    exit 2
    ;;
esac

run_id=$(date +"%Y%m%d-%H%M%S")
run_dir="$out_root/${run_id}-${tier}"
cases_root="$run_dir/cases"
case_records="$run_dir/cases.ndjson"
summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"

mkdir -p "$cases_root"
: > "$case_records"

select_case() {
  local min_tier=$1
  if [[ "$tier" == "required" ]]; then
    [[ "$min_tier" == "required" ]]
  else
    return 0
  fi
}

case_table=$(cat <<'EOF'
nat_private_role_policy|required|exact|home_nat|Validate private/home-NAT deployment-mode and role override plumbing.|Maps private validators/full nodes onto explicit P2P deployment policy instead of implicit public assumptions.|env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime parse_options_reads_explicit_p2p_policy_overrides -- --nocapture
validator_hidden_boundary|required|exact|validator_hidden|Validate validator_hidden remains bound to validator_core semantics.|Prevents observer/runtime role drift from claiming validator-hidden transport semantics.|env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_network_policy_rejects_incompatible_runtime_role_combo -- --nocapture
relay_only_lane_budget|required|exact|relay_only|Validate relay role stays confined to control-lane service surface.|Ensures relay/public ingress does not silently regain sync/blob request or serve rights.|env -u RUSTC_WRAPPER cargo test -p oasis7_node network_policy_limits_relay_to_control_lane -- --nocapture
cgnat_relay_path_ranking|required|exact|cgnat|Validate CGNAT peers rank direct before hole-punch before relay fallback.|Uses signed peer-record transport ordering as the exact proxy for no-public-IP path selection.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p peer_record_transport_paths_rank_direct_before_hole_punch_before_relay -- --nocapture
bootstrap_poisoning_dedupe|required|exact|bootstrap_poisoning|Validate poisoned bootstrap discovery does not permanently consume dial dedupe.|Covers discovery ingress quarantine against suspect records that later refresh healthy metadata.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p process_discovered_peer_record_does_not_poison_dial_dedupe_for_suspect_peer -- --nocapture
relay_budget_detection|required|exact|relay_exhaustion|Validate relay budget overflow is detected and downgraded before quarantine consumption.|Exact peer-manager coverage for relay-budget and relay-domain concentration detection during health recompute.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p recompute_marks_relay_budget_and_domain_concentration -- --nocapture
path_failover_selection|required|exact|path_failover|Validate direct-path failure falls back to hole-punch before relay.|Exact transport failover coverage for direct -> punched -> relay ordering.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p preferred_transport_path_skips_direct_and_falls_back_to_hole_punch_before_relay -- --nocapture
sentry_loss_proxy_longrun|full|proxy|sentry_loss|Run triad_distributed ingress-loss proxy with disconnect/restart chaos.|Dedicated sentry live harness is not wired yet; triad_distributed ingress loss is the current executable proxy for sentry/anchor loss.|./scripts/p2p-longrun-soak.sh --profile soak_release --topologies triad_distributed --duration-secs 300 --no-prewarm --max-stall-secs 240 --max-lag-p95 50 --max-distfs-failure-ratio 0.1 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-start-sec 30 --chaos-continuous-max-events 8 --chaos-continuous-actions disconnect,restart --chaos-continuous-seed 20260403 --chaos-continuous-restart-down-secs 1 --chaos-continuous-pause-duration-secs 2 --out-dir __RUN_DIR__/sentry-loss-proxy
mixed_topology_release_proxy|full|proxy|mixed_topology|Run triad + triad_distributed release-profile proxy under mixed chaos.|Current runtime harness has no physical NAT/CGNAT lab; this proxy leaves a real distributed recovery command in the evidence bundle without overstating coverage.|./scripts/p2p-longrun-soak.sh --profile soak_release --topologies triad,triad_distributed --duration-secs 300 --no-prewarm --max-stall-secs 240 --max-lag-p95 50 --max-distfs-failure-ratio 0.1 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-start-sec 30 --chaos-continuous-max-events 8 --chaos-continuous-actions restart,pause,disconnect --chaos-continuous-seed 20260403 --chaos-continuous-restart-down-secs 1 --chaos-continuous-pause-duration-secs 2 --out-dir __RUN_DIR__/mixed-topology-release-proxy
EOF
)

echo "p2p mixed-topology matrix"
echo "- tier: $tier"
echo "- dry_run: $dry_run"
echo "- output: $run_dir"

while IFS='|' read -r case_id min_tier coverage scenario description note command_template; do
  [[ -z "$case_id" ]] && continue
  if ! select_case "$min_tier"; then
    continue
  fi

  case_dir="$cases_root/$case_id"
  mkdir -p "$case_dir"
  command=${command_template//__RUN_DIR__/$run_dir}
  printf '%s\n' "$command" > "$case_dir/command.txt"

  started_at=$(date -Iseconds)
  status="dry_run"
  exit_code=0

  if [[ "$dry_run" -eq 1 ]]; then
    printf 'dry-run only\n' > "$case_dir/stdout.log"
    : > "$case_dir/stderr.log"
    echo "+ dry-run [$case_id]: $command"
  else
    echo "+ [$case_id] $command"
    if bash -lc "$command" >"$case_dir/stdout.log" 2>"$case_dir/stderr.log"; then
      status="ok"
      exit_code=0
    else
      exit_code=$?
      status="failed"
    fi
  fi
  ended_at=$(date -Iseconds)

  jq -n \
    --arg case_id "$case_id" \
    --arg min_tier "$min_tier" \
    --arg coverage "$coverage" \
    --arg scenario "$scenario" \
    --arg description "$description" \
    --arg note "$note" \
    --arg command "$command" \
    --arg started_at "$started_at" \
    --arg ended_at "$ended_at" \
    --arg status "$status" \
    --arg stdout_log "$case_dir/stdout.log" \
    --arg stderr_log "$case_dir/stderr.log" \
    --argjson exit_code "$exit_code" \
    '{
      case_id: $case_id,
      min_tier: $min_tier,
      coverage: $coverage,
      scenario: $scenario,
      description: $description,
      note: $note,
      command: $command,
      started_at: $started_at,
      ended_at: $ended_at,
      status: $status,
      exit_code: $exit_code,
      stdout_log: $stdout_log,
      stderr_log: $stderr_log
    }' >> "$case_records"
done <<< "$case_table"

generated_at=$(date -Iseconds)

jq -s \
  --arg generated_at "$generated_at" \
  --arg tier "$tier" \
  --arg run_dir "$run_dir" \
  --arg summary_md "$summary_md" \
  --argjson dry_run "$dry_run" \
  '{
    generated_at: $generated_at,
    tier: $tier,
    dry_run: ($dry_run == 1),
    run_dir: $run_dir,
    summary_md: $summary_md,
    cases: .,
    totals: {
      case_count: length,
      exact_case_count: (map(select(.coverage == "exact")) | length),
      proxy_case_count: (map(select(.coverage == "proxy")) | length),
      passed_count: (map(select(.status == "ok")) | length),
      failed_count: (map(select(.status == "failed")) | length),
      dry_run_count: (map(select(.status == "dry_run")) | length)
    },
    overall_status: (
      if $dry_run == 1 then
        "dry_run"
      elif any(.[]; .status == "failed") then
        "failed"
      else
        "ok"
      end
    )
  }' "$case_records" > "$summary_json"

{
  echo "# P2P Mixed Topology Validation Matrix"
  echo
  echo "- generated_at: \`$generated_at\`"
  echo "- tier: \`$tier\`"
  echo "- dry_run: \`$dry_run\`"
  echo "- run_dir: \`$run_dir\`"
  echo "- overall_status: \`$(jq -r '.overall_status' "$summary_json")\`"
  echo
  echo "| case | scenario | coverage | status | description |"
  echo "| --- | --- | --- | --- | --- |"
  while IFS=$'\t' read -r case_id scenario coverage status description; do
    echo "| $case_id | $scenario | $coverage | $status | $description |"
  done < <(jq -r '.cases[] | [ .case_id, .scenario, .coverage, .status, .description ] | @tsv' "$summary_json")
  echo
  echo "## Coverage Notes"
  echo "- \`exact\`: deterministic cargo tests that directly cover the current substrate contracts."
  echo "- \`proxy\`: executable longrun drills that approximate mixed-topology recovery until a dedicated sentry/NAT lab harness exists."
} > "$summary_md"

echo "matrix summary:"
echo "  summary_json: $summary_json"
echo "  summary_md: $summary_md"

if [[ "$dry_run" -eq 1 ]]; then
  exit 0
fi

if [[ "$(jq -r '.overall_status' "$summary_json")" != "ok" ]]; then
  exit 1
fi
