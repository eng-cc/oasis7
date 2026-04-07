#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-mixed-topology-matrix.sh [options]

Build or execute the P2PARCH-6 mixed-topology validation matrix.

Options:
  --tier <tier>                    required | full (default: required)
  --out-dir <path>                 output root (default: .tmp/p2p_mixed_topology)
  --shared-window-evidence-ref <path>
                                   attach same-window shared-network evidence ref
                                   (repeatable; summary only, does not execute it)
  --dedicated-lab-evidence-ref <path>
                                   attach dedicated sentry/NAT/live-lab evidence ref
                                   (repeatable; summary only, does not execute it)
  --pass-uplift-decision-ref <ref> attach producer/QA pass-uplift decision ref
  --dry-run                        render commands and summary only
  -h, --help                       show help

Notes:
  - `required` runs deterministic exact-coverage cases only.
  - `full` runs the required set plus proxy longrun cases that exercise
    distributed recovery without claiming a dedicated sentry/NAT lab.
  - External evidence refs are copied into the summary so downstream gate
    tooling can see which shared-window / dedicated-lab / pass-uplift inputs
    were present for this run.
  - Artifacts are written to:
      <out-dir>/<timestamp>-<tier>/{summary.json,summary.md,cases/*}
USAGE
}

tier="required"
out_root=".tmp/p2p_mixed_topology"
dry_run=0
shared_window_refs=()
dedicated_lab_refs=()
pass_uplift_decision_ref=""

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
    --shared-window-evidence-ref)
      shared_window_refs+=("${2:-}")
      shift 2
      ;;
    --dedicated-lab-evidence-ref)
      dedicated_lab_refs+=("${2:-}")
      shift 2
      ;;
    --pass-uplift-decision-ref)
      pass_uplift_decision_ref=${2:-}
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

write_json_array() {
  local out_file=$1
  shift
  if [[ $# -eq 0 ]]; then
    printf '[]\n' > "$out_file"
  else
    printf '%s\n' "$@" | jq -R . | jq -s . > "$out_file"
  fi
}

shared_window_refs_json="$run_dir/shared_window_refs.json"
dedicated_lab_refs_json="$run_dir/dedicated_lab_refs.json"
write_json_array "$shared_window_refs_json" "${shared_window_refs[@]}"
write_json_array "$dedicated_lab_refs_json" "${dedicated_lab_refs[@]}"

select_case() {
  local min_tier=$1
  if [[ "$tier" == "required" ]]; then
    [[ "$min_tier" == "required" ]]
  else
    return 0
  fi
}

case_table=$(cat <<'EOF'
nat_private_role_policy|required|exact|substrate_exact|home_nat|Validate private/home-NAT deployment-mode and role override plumbing.|Maps private validators/full nodes onto explicit P2P deployment policy instead of implicit public assumptions.|env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime parse_options_reads_explicit_p2p_policy_overrides -- --nocapture
validator_hidden_boundary|required|exact|substrate_exact|validator_hidden|Validate validator_hidden remains bound to validator_core semantics.|Prevents observer/runtime role drift from claiming validator-hidden transport semantics.|env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_network_policy_rejects_incompatible_runtime_role_combo -- --nocapture
relay_only_lane_budget|required|exact|substrate_exact|relay_only|Validate relay role stays confined to control-lane service surface.|Ensures relay/public ingress does not silently regain sync/blob request or serve rights.|env -u RUSTC_WRAPPER cargo test -p oasis7_node network_policy_limits_relay_to_control_lane -- --nocapture
cgnat_relay_path_ranking|required|exact|substrate_exact|cgnat|Validate CGNAT peers rank direct before hole-punch before relay fallback.|Uses signed peer-record transport ordering as the exact proxy for no-public-IP path selection.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p peer_record_transport_paths_rank_direct_before_hole_punch_before_relay -- --nocapture
bootstrap_poisoning_dedupe|required|exact|substrate_exact|bootstrap_poisoning|Validate poisoned bootstrap discovery does not permanently consume dial dedupe.|Covers discovery ingress quarantine against suspect records that later refresh healthy metadata.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p process_discovered_peer_record_does_not_poison_dial_dedupe_for_suspect_peer -- --nocapture
relay_budget_detection|required|exact|substrate_exact|relay_exhaustion|Validate relay budget overflow is detected and downgraded before quarantine consumption.|Exact peer-manager coverage for relay-budget and relay-domain concentration detection during health recompute.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p recompute_marks_relay_budget_and_domain_concentration -- --nocapture
path_failover_selection|required|exact|substrate_exact|path_failover|Validate direct-path failure falls back to hole-punch before relay.|Exact transport failover coverage for direct -> punched -> relay ordering.|env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p preferred_transport_path_skips_direct_and_falls_back_to_hole_punch_before_relay -- --nocapture
sentry_loss_proxy_longrun|full|proxy|executable_proxy|sentry_loss|Run triad_distributed ingress-loss proxy with disconnect/restart chaos.|Dedicated sentry live harness is not wired yet; triad_distributed ingress loss is the current executable proxy for sentry/anchor loss.|./scripts/p2p-longrun-soak.sh --profile soak_release --topologies triad_distributed --base-port 16610 --duration-secs 300 --max-stall-secs 240 --max-lag-p95 50 --max-distfs-failure-ratio 0.1 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-start-sec 30 --chaos-continuous-max-events 8 --chaos-continuous-actions disconnect,restart --chaos-continuous-seed 20260403 --chaos-continuous-restart-down-secs 1 --chaos-continuous-pause-duration-secs 2 --out-dir __RUN_DIR__/sentry-loss-proxy
mixed_topology_release_proxy|full|proxy|executable_proxy|mixed_topology|Run triad + triad_distributed release-profile proxy under mixed chaos.|Current runtime harness has no physical NAT/CGNAT lab; this proxy leaves a real distributed recovery command in the evidence bundle without overstating coverage.|./scripts/p2p-longrun-soak.sh --profile soak_release --topologies triad,triad_distributed --base-port 17610 --duration-secs 300 --max-stall-secs 240 --max-lag-p95 50 --max-distfs-failure-ratio 0.1 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-start-sec 30 --chaos-continuous-max-events 8 --chaos-continuous-actions restart,pause,disconnect --chaos-continuous-seed 20260403 --chaos-continuous-restart-down-secs 1 --chaos-continuous-pause-duration-secs 2 --out-dir __RUN_DIR__/mixed-topology-release-proxy
EOF
)

echo "p2p mixed-topology matrix"
echo "- tier: $tier"
echo "- dry_run: $dry_run"
echo "- output: $run_dir"

while IFS='|' read -r case_id min_tier coverage evidence_class scenario description note command_template; do
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
    --arg evidence_class "$evidence_class" \
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
      evidence_class: $evidence_class,
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
  --arg pass_uplift_decision_ref "$pass_uplift_decision_ref" \
  --argjson dry_run "$dry_run" \
  --slurpfile shared_window_refs "$shared_window_refs_json" \
  --slurpfile dedicated_lab_refs "$dedicated_lab_refs_json" \
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
    external_evidence: {
      shared_window_evidence_refs: $shared_window_refs[0],
      dedicated_lab_evidence_refs: $dedicated_lab_refs[0],
      pass_uplift_decision_ref: (
        if $pass_uplift_decision_ref == "" then
          null
        else
          $pass_uplift_decision_ref
        end
      )
    },
    evidence_contract: {
      executable_boundary: {
        required_exact_ready: (
          ($dry_run == 0)
          and ((map(select(.coverage == "exact")) | length) > 0)
          and ((map(select(.coverage == "exact" and .status == "ok")) | length) == (map(select(.coverage == "exact")) | length))
        ),
        full_proxy_ready: (
          ($tier == "full")
          and ($dry_run == 0)
          and ((map(select(.coverage == "proxy")) | length) > 0)
          and ((map(select(.coverage == "proxy" and .status == "ok")) | length) == (map(select(.coverage == "proxy")) | length))
        ),
        stronger_full_tier_truth_ready: (($dedicated_lab_refs[0] | length) > 0)
      },
      claim_readiness: {
        mixed_topology_full_tier_status: (
          if $tier == "required" then
            if $dry_run == 1 then
              "required_plan"
            elif any(.[]; .status == "failed") then
              "required_failed"
            else
              "required_exact_executed"
            end
          else
            if $dry_run == 1 then
              "full_proxy_plan"
            elif any(.[]; .status == "failed") then
              "full_failed"
            elif ((map(select(.coverage == "proxy" and .status == "ok")) | length) == (map(select(.coverage == "proxy")) | length)) then
              if (($dedicated_lab_refs[0] | length) > 0) then
                "full_proxy_executed_plus_dedicated_refs"
              else
                "full_proxy_executed"
              end
            else
              "required_exact_executed"
            end
          end
        ),
        shared_network_pass_inputs_ready: (
          ($tier == "full")
          and ($dry_run == 0)
          and ((map(select(.coverage == "proxy")) | length) > 0)
          and ((map(select(.coverage == "proxy" and .status == "ok")) | length) == (map(select(.coverage == "proxy")) | length))
          and (($shared_window_refs[0] | length) > 0)
          and ($pass_uplift_decision_ref != "")
        ),
        stronger_full_tier_truth_blockers: (
          []
          + (if $tier != "full" then ["run_full_tier_proxy_execution"] else [] end)
          + (if $dry_run == 1 then ["execute_full_tier_live_run"] else [] end)
          + (if any(.[]; .status == "failed") then ["fix_failed_matrix_cases"] else [] end)
          + (if (($dedicated_lab_refs[0] | length) == 0) then ["dedicated_sentry_or_nat_lab_evidence_ref"] else [] end)
        ),
        shared_network_pass_blockers: (
          []
          + (if $tier != "full" then ["run_full_tier_proxy_execution"] else [] end)
          + (if $dry_run == 1 then ["execute_full_tier_live_run"] else [] end)
          + (if any(.[]; .status == "failed") then ["fix_failed_matrix_cases"] else [] end)
          + (if (($shared_window_refs[0] | length) == 0) then ["same_window_shared_network_evidence_ref"] else [] end)
          + (if $pass_uplift_decision_ref == "" then ["producer_qa_pass_uplift_decision_ref"] else [] end)
        )
      }
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
  echo
  echo "## Evidence Contract"
  echo "- \`mixed_topology_full_tier_status\`: \`$(jq -r '.evidence_contract.claim_readiness.mixed_topology_full_tier_status' "$summary_json")\`"
  echo "- \`required_exact_ready\`: \`$(jq -r '.evidence_contract.executable_boundary.required_exact_ready' "$summary_json")\`"
  echo "- \`full_proxy_ready\`: \`$(jq -r '.evidence_contract.executable_boundary.full_proxy_ready' "$summary_json")\`"
  echo "- \`stronger_full_tier_truth_ready\`: \`$(jq -r '.evidence_contract.executable_boundary.stronger_full_tier_truth_ready' "$summary_json")\`"
  echo "- \`shared_network_pass_inputs_ready\`: \`$(jq -r '.evidence_contract.claim_readiness.shared_network_pass_inputs_ready' "$summary_json")\`"
  echo "- \`stronger_full_tier_truth_blockers\`: \`$(jq -r '.evidence_contract.claim_readiness.stronger_full_tier_truth_blockers | if length == 0 then "(none)" else join(", ") end' "$summary_json")\`"
  echo "- \`shared_network_pass_blockers\`: \`$(jq -r '.evidence_contract.claim_readiness.shared_network_pass_blockers | if length == 0 then "(none)" else join(", ") end' "$summary_json")\`"
  echo
  echo "## External Evidence Refs"
  echo "- \`shared_window_evidence_refs\`: \`$(jq -r '.external_evidence.shared_window_evidence_refs | if length == 0 then "(none)" else join(", ") end' "$summary_json")\`"
  echo "- \`dedicated_lab_evidence_refs\`: \`$(jq -r '.external_evidence.dedicated_lab_evidence_refs | if length == 0 then "(none)" else join(", ") end' "$summary_json")\`"
  echo "- \`pass_uplift_decision_ref\`: \`$(jq -r '.external_evidence.pass_uplift_decision_ref // "(none)"' "$summary_json")\`"
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
