#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/s10-five-node-game-soak.sh [options]

Status:
  active (2026-02-28): this script runs a five-node soak via oasis7_chain_runtime.

Options:
  --duration-secs <n>              soak duration seconds (default: 1800)
  --scenario <name>                scenario label used to derive world id (default: llm_bootstrap)
  --world-id <id>                  override world id (default: s10-<scenario>)
  --llm                            legacy compatibility flag (no effect, recorded only)
  --no-llm                         legacy compatibility flag (default)
  --reward-runtime-epoch-duration-secs <n>
                                   reward runtime epoch duration seconds (default: 60)
  --reward-points-per-credit <n>   reward points per credit (default: 100)
  --base-port <n>                  base port for node port allocation (default: 5810)
  --bind-host <host>               bind host for gossip/status endpoints (default: 127.0.0.1)
  --out-dir <path>                 output root (default: .tmp/s10_game_longrun)
  --startup-timeout-secs <n>       startup grace before monitor loop (default: 20)
  --poll-interval-secs <n>         monitor loop interval (default: 2)
  --curl-timeout-secs <n>          HTTP timeout for status polling (default: 2)
  --node-tick-ms <n>               worker poll/fallback interval milliseconds (default: 200)
  --pos-slot-duration-ms <n>       PoS slot duration in milliseconds (default: 12000)
  --pos-ticks-per-slot <n>         PoS logical ticks per slot (default: 10)
  --pos-proposal-tick-phase <n>    PoS proposal phase within slot tick window (default: 9)
  --pos-adaptive-tick-scheduler    enable PoS adaptive tick scheduler
  --pos-no-adaptive-tick-scheduler disable PoS adaptive scheduler (default)
  --pos-slot-clock-genesis-unix-ms <n>
                                   fixed PoS slot clock genesis unix ms (default: auto)
  --pos-max-past-slot-lag <n>      max accepted stale slot lag (default: 256)
  --max-stall-secs <n>             gate threshold for max no-progress window (default: 300)
  --max-lag-p95 <n>                gate threshold for p95(network_height - committed_height) (default: 12)
  --max-distfs-failure-ratio <r>   gate threshold for distfs failure ratio (default: 0.25)
  --max-settlement-apply-failure-ratio <r>
                                   gate threshold for settlement apply failure ratio (default: 0)
  --node-auto-attest-all           enable local auto-attesting all validators on all nodes
  --node-no-auto-attest-all        disable local auto-attesting all validators on all nodes
  --node-auto-attest-sequencer-only
                                   enable auto-attest only on sequencer (default)
  --preserve-node-state            keep existing output/node-distfs + output/chain-runtime state
  --no-prewarm                     skip cargo build prewarm
  --dry-run                        write config/commands only, do not start processes
  -h, --help                       show help

Topology:
  s10-sequencer (stake 35)
  s10-storage-a (stake 20)
  s10-storage-b (stake 20)
  s10-observer-a (stake 15)
  s10-observer-b (stake 10)

Output:
  <out-dir>/<timestamp>/
    run_config.json
    timeline.csv
    summary.json
    summary.md
    failures.md (only when failed)
    nodes/<node_id>/{command.txt,stdout.log,stderr.log}
USAGE
}

run() {
  echo "+ $*"
  "$@"
}

trim() {
  local value=$1
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

join_by() {
  local sep=$1
  shift || true
  local first=1
  local item
  for item in "$@"; do
    [[ -z "$item" ]] && continue
    if [[ "$first" -eq 1 ]]; then
      printf '%s' "$item"
      first=0
    else
      printf '%s%s' "$sep" "$item"
    fi
  done
}

ensure_positive_int() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^[0-9]+$ ]] || (( value <= 0 )); then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

ensure_non_negative_int() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

ensure_integer() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^-?[0-9]+$ ]]; then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

ensure_ratio_between_zero_and_one() {
  local flag=$1
  local value=$2
  if ! awk -v value="$value" 'BEGIN { exit !(value ~ /^([0-9]+(\.[0-9]+)?|\.[0-9]+)$/ && value >= 0.0 && value <= 1.0) }'; then
    echo "invalid $flag: $value (expected 0~1)" >&2
    exit 2
  fi
}

safe_int() {
  local value=$1
  if [[ "$value" =~ ^-?[0-9]+$ ]]; then
    printf '%s' "$value"
  else
    printf '0'
  fi
}

slugify() {
  local raw=$1
  local slug
  slug=$(printf '%s' "$raw" | tr -cs '[:alnum:]_-' '-')
  slug=${slug##-}
  slug=${slug%%-}
  printf '%s' "$slug"
}

duration_secs=1800
scenario="llm_bootstrap"
world_id_override=""
llm_enabled=0
reward_runtime_epoch_duration_secs=60
reward_points_per_credit=100
base_port=5810
bind_host="127.0.0.1"
out_root=".tmp/s10_game_longrun"
startup_timeout_secs=20
poll_interval_secs=2
curl_timeout_secs=2
node_tick_ms=200
pos_slot_duration_ms=12000
pos_ticks_per_slot=10
pos_proposal_tick_phase=9
pos_adaptive_tick_scheduler_enabled=0
pos_slot_clock_genesis_unix_ms=""
pos_max_past_slot_lag=256
max_stall_secs=300
max_lag_p95=12
max_distfs_failure_ratio="0.25"
max_settlement_apply_failure_ratio="0"
node_auto_attest_mode=1
isolate_node_state=1
prewarm=1
dry_run=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration-secs)
      duration_secs=${2:-}
      shift 2
      ;;
    --scenario)
      scenario=${2:-}
      shift 2
      ;;
    --world-id)
      world_id_override=${2:-}
      shift 2
      ;;
    --llm)
      llm_enabled=1
      shift
      ;;
    --no-llm)
      llm_enabled=0
      shift
      ;;
    --reward-runtime-epoch-duration-secs)
      reward_runtime_epoch_duration_secs=${2:-}
      shift 2
      ;;
    --reward-points-per-credit)
      reward_points_per_credit=${2:-}
      shift 2
      ;;
    --base-port)
      base_port=${2:-}
      shift 2
      ;;
    --bind-host)
      bind_host=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --startup-timeout-secs)
      startup_timeout_secs=${2:-}
      shift 2
      ;;
    --poll-interval-secs)
      poll_interval_secs=${2:-}
      shift 2
      ;;
    --curl-timeout-secs)
      curl_timeout_secs=${2:-}
      shift 2
      ;;
    --node-tick-ms)
      node_tick_ms=${2:-}
      shift 2
      ;;
    --pos-slot-duration-ms)
      pos_slot_duration_ms=${2:-}
      shift 2
      ;;
    --pos-ticks-per-slot)
      pos_ticks_per_slot=${2:-}
      shift 2
      ;;
    --pos-proposal-tick-phase)
      pos_proposal_tick_phase=${2:-}
      shift 2
      ;;
    --pos-adaptive-tick-scheduler)
      pos_adaptive_tick_scheduler_enabled=1
      shift
      ;;
    --pos-no-adaptive-tick-scheduler)
      pos_adaptive_tick_scheduler_enabled=0
      shift
      ;;
    --pos-slot-clock-genesis-unix-ms)
      pos_slot_clock_genesis_unix_ms=${2:-}
      shift 2
      ;;
    --pos-max-past-slot-lag)
      pos_max_past_slot_lag=${2:-}
      shift 2
      ;;
    --max-stall-secs)
      max_stall_secs=${2:-}
      shift 2
      ;;
    --max-lag-p95)
      max_lag_p95=${2:-}
      shift 2
      ;;
    --max-distfs-failure-ratio)
      max_distfs_failure_ratio=${2:-}
      shift 2
      ;;
    --max-settlement-apply-failure-ratio)
      max_settlement_apply_failure_ratio=${2:-}
      shift 2
      ;;
    --node-auto-attest-all)
      node_auto_attest_mode=2
      shift
      ;;
    --node-no-auto-attest-all)
      node_auto_attest_mode=0
      shift
      ;;
    --node-auto-attest-sequencer-only)
      node_auto_attest_mode=1
      shift
      ;;
    --preserve-node-state)
      isolate_node_state=0
      shift
      ;;
    --no-prewarm)
      prewarm=0
      shift
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
      usage
      exit 2
      ;;
  esac
done

ensure_positive_int "--duration-secs" "$duration_secs"
ensure_positive_int "--reward-runtime-epoch-duration-secs" "$reward_runtime_epoch_duration_secs"
ensure_positive_int "--reward-points-per-credit" "$reward_points_per_credit"
ensure_positive_int "--base-port" "$base_port"
ensure_positive_int "--startup-timeout-secs" "$startup_timeout_secs"
ensure_positive_int "--poll-interval-secs" "$poll_interval_secs"
ensure_positive_int "--curl-timeout-secs" "$curl_timeout_secs"
ensure_positive_int "--node-tick-ms" "$node_tick_ms"
ensure_positive_int "--pos-slot-duration-ms" "$pos_slot_duration_ms"
ensure_positive_int "--pos-ticks-per-slot" "$pos_ticks_per_slot"
ensure_non_negative_int "--pos-proposal-tick-phase" "$pos_proposal_tick_phase"
ensure_non_negative_int "--pos-max-past-slot-lag" "$pos_max_past_slot_lag"
if (( pos_proposal_tick_phase >= pos_ticks_per_slot )); then
  echo "--pos-proposal-tick-phase must be less than --pos-ticks-per-slot" >&2
  exit 2
fi
if [[ -n "$pos_slot_clock_genesis_unix_ms" ]]; then
  ensure_integer "--pos-slot-clock-genesis-unix-ms" "$pos_slot_clock_genesis_unix_ms"
fi
ensure_non_negative_int "--max-stall-secs" "$max_stall_secs"
ensure_non_negative_int "--max-lag-p95" "$max_lag_p95"
ensure_ratio_between_zero_and_one "--max-distfs-failure-ratio" "$max_distfs_failure_ratio"
ensure_ratio_between_zero_and_one "--max-settlement-apply-failure-ratio" "$max_settlement_apply_failure_ratio"

scenario=$(trim "$scenario")
if [[ -z "$scenario" ]]; then
  echo "--scenario cannot be empty" >&2
  exit 2
fi

world_id_override=$(trim "$world_id_override")
if [[ -n "$world_id_override" ]]; then
  world_id="$world_id_override"
else
  scenario_slug=$(slugify "$scenario")
  if [[ -z "$scenario_slug" ]]; then
    scenario_slug="default"
  fi
  world_id="s10-${scenario_slug}"
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required for metrics aggregation but not found in PATH" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for endpoint polling but not found in PATH" >&2
  exit 1
fi

if [[ "$prewarm" -eq 1 ]] && [[ "$dry_run" -eq 0 ]]; then
  run env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_chain_runtime
fi

chain_bin="$repo_root/target/debug/oasis7_chain_runtime"
if [[ "$dry_run" -eq 0 ]] && [[ ! -x "$chain_bin" ]]; then
  echo "oasis7_chain_runtime binary not found: $chain_bin" >&2
  echo "run with prewarm enabled or build it manually first" >&2
  exit 1
fi

timestamp=$(date +%Y%m%d-%H%M%S)
run_dir="$out_root/$timestamp"
run mkdir -p "$run_dir"

declare -a node_ids=(
  "s10-sequencer"
  "s10-storage-a"
  "s10-storage-b"
  "s10-observer-a"
  "s10-observer-b"
)
declare -a node_roles=(
  "sequencer"
  "storage"
  "storage"
  "observer"
  "observer"
)
declare -a node_stakes=(35 20 20 15 10)
node_count=${#node_ids[@]}

declare -a validator_specs=()
for idx in "${!node_ids[@]}"; do
  validator_specs+=("${node_ids[$idx]}:${node_stakes[$idx]}")
done

node_gossip_port() {
  local idx=$1
  printf '%s' $((base_port + idx + 1))
}

node_status_port() {
  local idx=$1
  printf '%s' $((base_port + idx + 21))
}

node_gossip_addr() {
  local idx=$1
  printf '%s:%s' "$bind_host" "$(node_gossip_port "$idx")"
}

node_status_bind_addr() {
  local idx=$1
  printf '%s:%s' "$bind_host" "$(node_status_port "$idx")"
}

node_replication_port() {
  local idx=$1
  printf '%s' $((base_port + idx + 41))
}

node_replication_listen_addr() {
  local idx=$1
  printf '/ip4/%s/tcp/%s' "$bind_host" "$(node_replication_port "$idx")"
}

node_status_url() {
  local idx=$1
  printf 'http://%s/v1/chain/status' "$(node_status_bind_addr "$idx")"
}

node_balances_url() {
  local idx=$1
  printf 'http://%s/v1/chain/balances' "$(node_status_bind_addr "$idx")"
}

node_healthz_url() {
  local idx=$1
  printf 'http://%s/healthz' "$(node_status_bind_addr "$idx")"
}

run_config_json="$run_dir/run_config.json"
summary_md="$run_dir/summary.md"
timeline_csv="$run_dir/timeline.csv"
summary_json="$run_dir/summary.json"
failures_md="$run_dir/failures.md"
lag_values_file="$run_dir/.lag_values.txt"
runtime_errors_tsv="$run_dir/.runtime_errors.tsv"
: > "$lag_values_file"
: > "$runtime_errors_tsv"

node_table_tsv="$run_dir/.nodes.tsv"
: > "$node_table_tsv"
for idx in "${!node_ids[@]}"; do
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "${node_ids[$idx]}" \
    "${node_roles[$idx]}" \
    "${node_stakes[$idx]}" \
    "$(node_gossip_addr "$idx")" \
    "$(node_status_bind_addr "$idx")" \
    "$(node_status_url "$idx")" \
    "$(node_balances_url "$idx")" >> "$node_table_tsv"
done

validators_json=$(printf '%s\n' "${validator_specs[@]}" | jq -R -s '
  split("\n")
  | map(select(length > 0) | split(":") | {
      validator_id: .[0],
      stake: (.[1] | tonumber)
    })
')
nodes_json=$(jq -R -s '
  split("\n")
  | map(select(length > 0) | split("\t") | {
      node_id: .[0],
      role: .[1],
      stake: (.[2] | tonumber),
      gossip_bind: .[3],
      status_bind: .[4],
      status_url: .[5],
      balances_url: .[6]
    })
' "$node_table_tsv")

node_auto_attest_mode_label="off"
if [[ "$node_auto_attest_mode" -eq 2 ]]; then
  node_auto_attest_mode_label="all"
elif [[ "$node_auto_attest_mode" -eq 1 ]]; then
  node_auto_attest_mode_label="sequencer_only"
fi

jq -n \
  --arg run_dir "$run_dir" \
  --arg scenario "$scenario" \
  --arg world_id "$world_id" \
  --arg bind_host "$bind_host" \
  --arg out_dir "$out_root" \
  --arg auto_attest_mode "$node_auto_attest_mode_label" \
  --argjson duration_secs "$duration_secs" \
  --argjson llm_enabled "$llm_enabled" \
  --argjson reward_runtime_epoch_duration_secs "$reward_runtime_epoch_duration_secs" \
  --argjson reward_points_per_credit "$reward_points_per_credit" \
  --argjson node_tick_ms "$node_tick_ms" \
  --argjson pos_slot_duration_ms "$pos_slot_duration_ms" \
  --argjson pos_ticks_per_slot "$pos_ticks_per_slot" \
  --argjson pos_proposal_tick_phase "$pos_proposal_tick_phase" \
  --argjson pos_adaptive_tick_scheduler_enabled "$pos_adaptive_tick_scheduler_enabled" \
  --arg pos_slot_clock_genesis_unix_ms "$pos_slot_clock_genesis_unix_ms" \
  --argjson pos_max_past_slot_lag "$pos_max_past_slot_lag" \
  --argjson base_port "$base_port" \
  --argjson startup_timeout_secs "$startup_timeout_secs" \
  --argjson poll_interval_secs "$poll_interval_secs" \
  --argjson curl_timeout_secs "$curl_timeout_secs" \
  --argjson max_stall_secs "$max_stall_secs" \
  --argjson max_lag_p95 "$max_lag_p95" \
  --argjson max_distfs_failure_ratio "$max_distfs_failure_ratio" \
  --argjson max_settlement_apply_failure_ratio "$max_settlement_apply_failure_ratio" \
  --argjson isolate_node_state "$isolate_node_state" \
  --argjson dry_run "$dry_run" \
  --argjson validators "$validators_json" \
  --argjson nodes "$nodes_json" \
  '{
    run_dir: $run_dir,
    scenario: $scenario,
    world_id: $world_id,
    llm_enabled_compat: ($llm_enabled == 1),
    reward_runtime_epoch_duration_secs: $reward_runtime_epoch_duration_secs,
    reward_points_per_credit: $reward_points_per_credit,
    node_tick_ms: $node_tick_ms,
    pos_config: {
      slot_duration_ms: $pos_slot_duration_ms,
      ticks_per_slot: $pos_ticks_per_slot,
      proposal_tick_phase: $pos_proposal_tick_phase,
      adaptive_tick_scheduler_enabled: ($pos_adaptive_tick_scheduler_enabled == 1),
      slot_clock_genesis_unix_ms: (if $pos_slot_clock_genesis_unix_ms == "" then null else ($pos_slot_clock_genesis_unix_ms | tonumber) end),
      max_past_slot_lag: $pos_max_past_slot_lag
    },
    duration_secs: $duration_secs,
    bind_host: $bind_host,
    base_port: $base_port,
    startup_timeout_secs: $startup_timeout_secs,
    poll_interval_secs: $poll_interval_secs,
    curl_timeout_secs: $curl_timeout_secs,
    thresholds: {
      max_stall_secs: $max_stall_secs,
      max_lag_p95: $max_lag_p95,
      max_distfs_failure_ratio: $max_distfs_failure_ratio,
      max_settlement_apply_failure_ratio: $max_settlement_apply_failure_ratio
    },
    node_auto_attest_mode: $auto_attest_mode,
    isolate_node_state: ($isolate_node_state == 1),
    dry_run: ($dry_run == 1),
    validators: $validators,
    nodes: $nodes,
    compatibility_notes: [
      "--llm/--no-llm are accepted but no longer affect oasis7_chain_runtime",
      "node_tick_ms is worker poll/fallback interval; PoS slot timing is configured by pos_config.*"
    ]
  }' > "$run_config_json"

{
  echo "# S10 Five-Node Real Game Soak Summary"
  echo
  echo "- run_dir: \`$run_dir\`"
  echo "- duration_secs: \`$duration_secs\`"
  echo "- scenario: \`$scenario\`"
  echo "- world_id: \`$world_id\`"
  echo "- llm_enabled_compat: \`$llm_enabled\`"
  echo "- reward_runtime_epoch_duration_secs: \`$reward_runtime_epoch_duration_secs\`"
  echo "- reward_points_per_credit: \`$reward_points_per_credit\`"
  echo "- node_tick_ms(worker_poll_fallback_ms): \`$node_tick_ms\`"
  echo "- pos_slot_duration_ms: \`$pos_slot_duration_ms\`"
  echo "- pos_ticks_per_slot: \`$pos_ticks_per_slot\`"
  echo "- pos_proposal_tick_phase: \`$pos_proposal_tick_phase\`"
  echo "- pos_adaptive_tick_scheduler_enabled: \`$pos_adaptive_tick_scheduler_enabled\`"
  if [[ -n "$pos_slot_clock_genesis_unix_ms" ]]; then
    echo "- pos_slot_clock_genesis_unix_ms: \`$pos_slot_clock_genesis_unix_ms\`"
  else
    echo "- pos_slot_clock_genesis_unix_ms: \`auto\`"
  fi
  echo "- pos_max_past_slot_lag: \`$pos_max_past_slot_lag\`"
  echo "- max_stall_secs: \`$max_stall_secs\`"
  echo "- max_lag_p95: \`$max_lag_p95\`"
  echo "- max_distfs_failure_ratio: \`$max_distfs_failure_ratio\`"
  echo "- max_settlement_apply_failure_ratio: \`$max_settlement_apply_failure_ratio\`"
  echo "- node_auto_attest_mode: \`$node_auto_attest_mode_label\`"
  echo "- isolate_node_state: \`$isolate_node_state\`"
  echo
  echo "| run | status | process_status | metric_gate | reports | started_at | ended_at | notes |"
  echo "|---|---|---|---|---|---|---|---|"
} > "$summary_md"

echo "node,epoch_index,observed_at_unix_ms,committed_height,network_committed_height,lag,total_checks,failed_checks,distfs_failure_ratio,invariant_ok,total_distributed_points,minted_record_count,settlement_apply_attempts_total,settlement_apply_failures_total,settlement_apply_failure_ratio,report_path" > "$timeline_csv"

append_summary_row() {
  local run_name=$1
  local status=$2
  local process_status=$3
  local metric_gate=$4
  local reports=$5
  local started_at=$6
  local ended_at=$7
  local notes=$8
  echo "| $run_name | $status | $process_status | $metric_gate | $reports | $started_at | $ended_at | $notes |" >> "$summary_md"
}

declare -a prepared_cmd=()
prepare_node_command() {
  local idx=$1
  local node_id=${node_ids[$idx]}
  local role=${node_roles[$idx]}
  local -a cmd=(
    "$chain_bin"
    --node-id "$node_id"
    --world-id "$world_id"
    --status-bind "$(node_status_bind_addr "$idx")"
    --node-role "$role"
    --node-tick-ms "$node_tick_ms"
    --pos-slot-duration-ms "$pos_slot_duration_ms"
    --pos-ticks-per-slot "$pos_ticks_per_slot"
    --pos-proposal-tick-phase "$pos_proposal_tick_phase"
    --pos-max-past-slot-lag "$pos_max_past_slot_lag"
    --reward-runtime-epoch-duration-secs "$reward_runtime_epoch_duration_secs"
    --reward-points-per-credit "$reward_points_per_credit"
    --node-gossip-bind "$(node_gossip_addr "$idx")"
    --replication-network-listen "$(node_replication_listen_addr "$idx")"
  )
  if [[ "$pos_adaptive_tick_scheduler_enabled" -eq 1 ]]; then
    cmd+=(--pos-adaptive-tick-scheduler)
  else
    cmd+=(--pos-no-adaptive-tick-scheduler)
  fi
  if [[ -n "$pos_slot_clock_genesis_unix_ms" ]]; then
    cmd+=(--pos-slot-clock-genesis-unix-ms "$pos_slot_clock_genesis_unix_ms")
  fi

  if [[ "$node_auto_attest_mode" -eq 2 ]] || { [[ "$node_auto_attest_mode" -eq 1 ]] && [[ "$role" == "sequencer" ]]; }; then
    cmd+=(--node-auto-attest-all)
  else
    cmd+=(--node-no-auto-attest-all)
  fi

  local validator
  for validator in "${validator_specs[@]}"; do
    cmd+=(--node-validator "$validator")
  done

  local peer_idx
  for peer_idx in "${!node_ids[@]}"; do
    if (( peer_idx == idx )); then
      continue
    fi
    cmd+=(--node-gossip-peer "$(node_gossip_addr "$peer_idx")")
    cmd+=(--replication-network-peer "$(node_replication_listen_addr "$peer_idx")")
  done

  prepared_cmd=("${cmd[@]}")
}

isolate_node_state_dirs() {
  local backup_root="$run_dir/node_state_backup"
  local node_id state_dir backup_dir
  for node_id in "${node_ids[@]}"; do
    for state_dir in \
      "$repo_root/output/node-distfs/$node_id" \
      "$repo_root/output/chain-runtime/$node_id"; do
      if [[ ! -e "$state_dir" ]]; then
        continue
      fi
      run mkdir -p "$backup_root"
      backup_dir="$backup_root/$(basename "$(dirname "$state_dir")")-${node_id}-$(date +%s)"
      while [[ -e "$backup_dir" ]]; do
        backup_dir="${backup_dir}-$RANDOM"
      done
      run mv "$state_dir" "$backup_dir"
    done
  done
}

active_cleanup_done=0
declare -a active_pids=()
declare -a active_nodes=()
declare -a active_waited=()
declare -a active_exit_statuses=()
captured_exit_status=0

capture_process_exit_status() {
  local idx=$1
  if [[ "${active_waited[$idx]:-0}" -eq 1 ]]; then
    captured_exit_status="${active_exit_statuses[$idx]:-0}"
    return 0
  fi

  local pid=${active_pids[$idx]}
  local status=0
  if wait "$pid"; then
    status=0
  else
    status=$?
  fi

  active_waited[$idx]=1
  active_exit_statuses[$idx]=$status
  captured_exit_status=$status

  local node_name=${active_nodes[$idx]}
  local node_dir="$run_dir/nodes/$node_name"
  local exit_status_file="$node_dir/exit-status.txt"
  {
    echo "node_id=$node_name"
    echo "pid=$pid"
    echo "exit_status=$status"
    if (( status >= 128 )); then
      echo "signal=$((status - 128))"
    fi
  } > "$exit_status_file"

  return 0
}

matching_node_pids() {
  local node_name=$1
  local idx=-1
  local candidate
  for candidate in "${!node_ids[@]}"; do
    if [[ "${node_ids[$candidate]}" == "$node_name" ]]; then
      idx=$candidate
      break
    fi
  done
  if (( idx < 0 )); then
    return 0
  fi
  local status_addr gossip_addr
  status_addr="$(node_status_bind_addr "$idx")"
  gossip_addr="$(node_gossip_addr "$idx")"
  pgrep -f "/target/debug/oasis7_chain_runtime --node-id ${node_name} .*--status-bind ${status_addr} .*--node-gossip-bind ${gossip_addr}( |$)" || true
}

wait_for_node_ports_to_close() {
  local deadline=$(( $(date +%s) + 8 ))
  local idx gossip_port status_port
  while :; do
    local any_open=0
    for idx in "${!node_ids[@]}"; do
      gossip_port=$((base_port + idx + 1))
      status_port=$((base_port + 20 + idx + 1))
      if curl -fsS --max-time 1 "http://${bind_host}:${status_port}/healthz" >/dev/null 2>&1; then
        any_open=1
        break
      fi
      if (exec 3<>"/dev/tcp/${bind_host}/${gossip_port}") >/dev/null 2>&1; then
        exec 3<&-
        exec 3>&-
        any_open=1
        break
      fi
    done
    if [[ "$any_open" -eq 0 ]]; then
      return 0
    fi
    if (( $(date +%s) >= deadline )); then
      return 1
    fi
    sleep 1
  done
}

stop_active_processes() {
  if [[ "$active_cleanup_done" -eq 1 ]]; then
    return 0
  fi
  active_cleanup_done=1

  local pid node_name
  for pid in "${active_pids[@]}"; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done
  for node_name in "${node_ids[@]}"; do
    while read -r pid; do
      [[ -z "$pid" ]] && continue
      if kill -0 "$pid" >/dev/null 2>&1; then
        kill "$pid" >/dev/null 2>&1 || true
      fi
      done < <(matching_node_pids "$node_name")
  done

  local deadline=$(( $(date +%s) + 8 ))
  while :; do
    local any_alive=0
    for pid in "${active_pids[@]}"; do
      if kill -0 "$pid" >/dev/null 2>&1; then
        any_alive=1
        break
      fi
    done
    if [[ "$any_alive" -eq 0 ]]; then
      for node_name in "${node_ids[@]}"; do
        if [[ -n "$(matching_node_pids "$node_name")" ]]; then
          any_alive=1
          break
        fi
      done
    fi
    if [[ "$any_alive" -eq 0 ]]; then
      break
    fi
    if (( $(date +%s) >= deadline )); then
      for pid in "${active_pids[@]}"; do
        if kill -0 "$pid" >/dev/null 2>&1; then
          kill -9 "$pid" >/dev/null 2>&1 || true
        fi
      done
      for node_name in "${node_ids[@]}"; do
        while read -r pid; do
          [[ -z "$pid" ]] && continue
          if kill -0 "$pid" >/dev/null 2>&1; then
            kill -9 "$pid" >/dev/null 2>&1 || true
          fi
        done < <(matching_node_pids "$node_name")
      done
      break
    fi
    sleep 1
  done

  local idx
  for idx in "${!active_pids[@]}"; do
    if [[ "${active_waited[$idx]:-0}" -eq 1 ]]; then
      continue
    fi
    capture_process_exit_status "$idx" >/dev/null 2>&1 || true
  done

  wait_for_node_ports_to_close || true
}

cleanup_on_exit() {
  stop_active_processes
}
trap cleanup_on_exit EXIT

launch_node() {
  local node_name=$1
  shift

  local node_dir="$run_dir/nodes/$node_name"
  local stdout_log="$node_dir/stdout.log"
  local stderr_log="$node_dir/stderr.log"
  local cmd_txt="$node_dir/command.txt"

  run mkdir -p "$node_dir"
  printf '%q ' "$@" > "$cmd_txt"
  printf '\n' >> "$cmd_txt"

  echo "+ $* > $stdout_log 2> $stderr_log"
  "$@" >"$stdout_log" 2>"$stderr_log" &
  local pid=$!
  active_pids+=("$pid")
  active_nodes+=("$node_name")
  active_waited+=(0)
  active_exit_statuses+=("")
}

wait_for_startup_ready() {
  local deadline=$(( $(date +%s) + startup_timeout_secs ))
  while :; do
    local all_ready=1
    local idx
    for idx in "${!node_ids[@]}"; do
      if ! kill -0 "${active_pids[$idx]}" >/dev/null 2>&1; then
        local exit_status
        capture_process_exit_status "$idx"
        exit_status=$captured_exit_status
        echo "node exited before startup ready: ${active_nodes[$idx]} exit_status=${exit_status}" >&2
        return 1
      fi
      if ! curl -fsS --max-time "$curl_timeout_secs" "$(node_healthz_url "$idx")" >/dev/null 2>&1; then
        all_ready=0
      fi
    done

    if [[ "$all_ready" -eq 1 ]]; then
      return 0
    fi
    if (( $(date +%s) >= deadline )); then
      echo "startup timeout waiting for /healthz on all nodes" >&2
      return 1
    fi
    sleep 1
  done
}

is_tolerable_bootstrap_fetch_commit_unavailable() {
  local node_name=$1
  local err=$2

  [[ "$node_name" == "s10-sequencer" ]] || return 1
  [[ "$err" == *"NetworkRequestFailed"* ]] || return 1
  [[ "$err" == *"NetworkProtocolUnavailable"* ]] || return 1
  [[ "$err" == *"/aw/node/replication/fetch-commit/1.0.0"* ]]
}

analysis_report_count=0
analysis_gate_status="insufficient_data"
analysis_gate_notes="no_samples"
analysis_max_stall_secs_observed=0
analysis_lag_p95=0
analysis_distfs_failure_ratio="0.000000"
analysis_distfs_total_checks=0
analysis_distfs_failed_checks=0
analysis_settlement_apply_failure_ratio="0.000000"
analysis_settlement_apply_attempts=0
analysis_settlement_apply_failures=0
analysis_invariant_all_ok=true
analysis_settlement_positive_samples=0
analysis_minted_non_empty_samples=0
analysis_monotonic_ok=true
analysis_monotonic_violation_nodes=""
analysis_status_samples_ok=0
analysis_balances_samples_ok=0
analysis_running_false_samples=0
analysis_last_error_samples=0
analysis_tolerated_bootstrap_fetch_commit_unavailable_samples=0
analysis_balance_load_error_samples=0
analysis_peer_zero_samples=0
analysis_http_failure_samples=0
analysis_reward_runtime_available_samples=0

best_height=-1
last_progress_epoch_sec=0
declare -A node_prev_committed=()
declare -A node_monotonic_violations=()
declare -A node_distfs_total_checks_max=()
declare -A node_distfs_failed_checks_max=()
declare -A node_settlement_apply_attempts_max=()
declare -A node_settlement_apply_failures_max=()
declare -A node_reward_minted_count_max=()

append_timeline_sample() {
  local node_name=$1
  local epoch_index=$2
  local observed_at_unix_ms=$3
  local committed_height=$4
  local network_committed_height=$5
  local lag=$6
  local invariant_ok=$7
  local total_distributed_points=$8
  local minted_record_count=$9
  local report_path=${10}

  jq -rRn \
    --arg node "$node_name" \
    --arg epoch "$epoch_index" \
    --arg observed "$observed_at_unix_ms" \
    --arg committed "$committed_height" \
    --arg network "$network_committed_height" \
    --arg lag "$lag" \
    --arg invariant "$invariant_ok" \
    --arg points "$total_distributed_points" \
    --arg minted "$minted_record_count" \
    --arg report "$report_path" \
    '[
      $node,
      ($epoch|tonumber),
      ($observed|tonumber),
      ($committed|tonumber),
      ($network|tonumber),
      ($lag|tonumber),
      0,
      0,
      "0.000000",
      ($invariant == "true"),
      ($points|tonumber),
      ($minted|tonumber),
      0,
      0,
      "0.000000",
      $report
    ] | @csv' >> "$timeline_csv"
}

poll_node_once() {
  local idx=$1
  local node_name=${node_ids[$idx]}
  local status_url
  status_url=$(node_status_url "$idx")
  local balances_url
  balances_url=$(node_balances_url "$idx")

  local status_json=""
  local balances_json=""
  local status_ok=0
  local balances_ok=0

  if status_json=$(curl -fsS --max-time "$curl_timeout_secs" "$status_url" 2>/dev/null); then
    status_ok=1
  fi
  if balances_json=$(curl -fsS --max-time "$curl_timeout_secs" "$balances_url" 2>/dev/null); then
    balances_ok=1
  fi

  local observed_at_unix_ms epoch_index committed_height network_committed_height running known_peer_heads last_error
  observed_at_unix_ms=$(( $(date +%s) * 1000 ))
  epoch_index=0
  committed_height=0
  network_committed_height=0
  running="false"
  known_peer_heads=0
  last_error=""
  local rr_metrics_available="false"
  local rr_last_error=""
  local rr_distfs_total_checks=0
  local rr_distfs_failed_checks=0
  local rr_settlement_apply_attempts=0
  local rr_settlement_apply_failures=0
  local rr_minted_cumulative_count=0
  local rr_invariant_ok="true"
  local rr_total_distributed_points=0

  if [[ "$status_ok" -eq 1 ]]; then
    observed_at_unix_ms=$(safe_int "$(jq -r '.observed_at_unix_ms // 0' <<< "$status_json")")
    epoch_index=$(safe_int "$(jq -r '.consensus.epoch // 0' <<< "$status_json")")
    committed_height=$(safe_int "$(jq -r '.consensus.committed_height // 0' <<< "$status_json")")
    network_committed_height=$(safe_int "$(jq -r '.consensus.network_committed_height // 0' <<< "$status_json")")
    running=$(jq -r '.running // false' <<< "$status_json")
    known_peer_heads=$(safe_int "$(jq -r '.consensus.known_peer_heads // 0' <<< "$status_json")")
    last_error=$(jq -r '.last_error // empty' <<< "$status_json")
    rr_metrics_available=$(jq -r '.reward_runtime.metrics_available // false' <<< "$status_json")
    rr_last_error=$(jq -r '.reward_runtime.last_error // empty' <<< "$status_json")
    rr_distfs_total_checks=$(safe_int "$(jq -r '.reward_runtime.distfs_total_checks // 0' <<< "$status_json")")
    rr_distfs_failed_checks=$(safe_int "$(jq -r '.reward_runtime.distfs_failed_checks // 0' <<< "$status_json")")
    rr_settlement_apply_attempts=$(safe_int "$(jq -r '.reward_runtime.settlement_apply_attempts_total // 0' <<< "$status_json")")
    rr_settlement_apply_failures=$(safe_int "$(jq -r '.reward_runtime.settlement_apply_failures_total // 0' <<< "$status_json")")
    rr_minted_cumulative_count=$(safe_int "$(jq -r '.reward_runtime.cumulative_minted_record_count // 0' <<< "$status_json")")
    rr_invariant_ok=$(jq -r '.reward_runtime.invariant_ok // true' <<< "$status_json")
    rr_total_distributed_points=$(safe_int "$(jq -r '.reward_runtime.latest_total_distributed_points // 0' <<< "$status_json")")
  fi

  local reward_mint_record_count node_power_credit_balance node_main_token_liquid_balance balance_load_error
  reward_mint_record_count=0
  node_power_credit_balance=0
  node_main_token_liquid_balance=0
  balance_load_error=""
  if [[ "$balances_ok" -eq 1 ]]; then
    reward_mint_record_count=$(safe_int "$(jq -r '.reward_mint_record_count // 0' <<< "$balances_json")")
    node_power_credit_balance=$(safe_int "$(jq -r '.node_power_credit_balance // 0' <<< "$balances_json")")
    node_main_token_liquid_balance=$(safe_int "$(jq -r '.node_main_token_liquid_balance // 0' <<< "$balances_json")")
    balance_load_error=$(jq -r '.load_error // empty' <<< "$balances_json")
  fi
  local effective_minted_record_count=$reward_mint_record_count
  if (( rr_minted_cumulative_count > effective_minted_record_count )); then
    effective_minted_record_count=$rr_minted_cumulative_count
  fi

  local lag=$((network_committed_height - committed_height))
  if (( lag < 0 )); then
    lag=0
  fi

  local invariant_ok="true"
  if [[ "$running" != "true" ]] || [[ -n "$last_error" ]]; then
    invariant_ok="false"
  fi

  append_timeline_sample \
    "$node_name" \
    "$epoch_index" \
    "$observed_at_unix_ms" \
    "$committed_height" \
    "$network_committed_height" \
    "$lag" \
    "$invariant_ok" \
    "$rr_total_distributed_points" \
    "$effective_minted_record_count" \
    "$status_url"

  analysis_report_count=$((analysis_report_count + 1))

  if [[ "$status_ok" -eq 1 ]]; then
    analysis_status_samples_ok=$((analysis_status_samples_ok + 1))
  else
    analysis_http_failure_samples=$((analysis_http_failure_samples + 1))
  fi
  if [[ "$balances_ok" -eq 1 ]]; then
    analysis_balances_samples_ok=$((analysis_balances_samples_ok + 1))
  else
    analysis_http_failure_samples=$((analysis_http_failure_samples + 1))
  fi

  if [[ "$running" != "true" ]]; then
    analysis_running_false_samples=$((analysis_running_false_samples + 1))
  fi
  if [[ -n "$last_error" ]]; then
    if is_tolerable_bootstrap_fetch_commit_unavailable "$node_name" "$last_error"; then
      analysis_tolerated_bootstrap_fetch_commit_unavailable_samples=$((analysis_tolerated_bootstrap_fetch_commit_unavailable_samples + 1))
    else
      analysis_last_error_samples=$((analysis_last_error_samples + 1))
      printf '%s\t%s\n' "$node_name" "$last_error" >> "$runtime_errors_tsv"
    fi
  fi
  if [[ -n "$balance_load_error" ]]; then
    analysis_balance_load_error_samples=$((analysis_balance_load_error_samples + 1))
  fi
  if (( known_peer_heads <= 0 )); then
    analysis_peer_zero_samples=$((analysis_peer_zero_samples + 1))
  fi
  if [[ "$rr_metrics_available" == "true" ]]; then
    analysis_reward_runtime_available_samples=$((analysis_reward_runtime_available_samples + 1))
  fi
  if [[ -n "$rr_last_error" ]]; then
    analysis_last_error_samples=$((analysis_last_error_samples + 1))
    printf '%s\t%s\n' "$node_name" "$rr_last_error" >> "$runtime_errors_tsv"
  fi
  if [[ "$rr_invariant_ok" != "true" ]]; then
    analysis_invariant_all_ok=false
  fi

  local prev_distfs_total=${node_distfs_total_checks_max[$node_name]:-0}
  if (( rr_distfs_total_checks > prev_distfs_total )); then
    node_distfs_total_checks_max["$node_name"]=$rr_distfs_total_checks
  fi
  local prev_distfs_failed=${node_distfs_failed_checks_max[$node_name]:-0}
  if (( rr_distfs_failed_checks > prev_distfs_failed )); then
    node_distfs_failed_checks_max["$node_name"]=$rr_distfs_failed_checks
  fi
  local prev_settlement_attempts=${node_settlement_apply_attempts_max[$node_name]:-0}
  if (( rr_settlement_apply_attempts > prev_settlement_attempts )); then
    node_settlement_apply_attempts_max["$node_name"]=$rr_settlement_apply_attempts
  fi
  local prev_settlement_failures=${node_settlement_apply_failures_max[$node_name]:-0}
  if (( rr_settlement_apply_failures > prev_settlement_failures )); then
    node_settlement_apply_failures_max["$node_name"]=$rr_settlement_apply_failures
  fi
  local prev_reward_minted=${node_reward_minted_count_max[$node_name]:-0}
  if (( rr_minted_cumulative_count > prev_reward_minted )); then
    node_reward_minted_count_max["$node_name"]=$rr_minted_cumulative_count
  fi

  if (( effective_minted_record_count > 0 )); then
    analysis_minted_non_empty_samples=$((analysis_minted_non_empty_samples + 1))
  fi
  if (( node_main_token_liquid_balance > 0 || node_power_credit_balance > 0 || rr_settlement_apply_attempts > 0 )); then
    analysis_settlement_positive_samples=$((analysis_settlement_positive_samples + 1))
  fi

  echo "$lag" >> "$lag_values_file"

  local now_sec
  now_sec=$(date +%s)
  if (( best_height < 0 )); then
    best_height=$committed_height
    last_progress_epoch_sec=$now_sec
  elif (( committed_height > best_height )); then
    best_height=$committed_height
    last_progress_epoch_sec=$now_sec
  fi
  local stall=$((now_sec - last_progress_epoch_sec))
  if (( stall > analysis_max_stall_secs_observed )); then
    analysis_max_stall_secs_observed=$stall
  fi

  local prev_committed=${node_prev_committed[$node_name]:-}
  if [[ -n "$prev_committed" ]] && (( committed_height < prev_committed )); then
    node_monotonic_violations["$node_name"]=1
  fi
  node_prev_committed["$node_name"]=$committed_height
}

poll_all_nodes_once() {
  local idx
  for idx in "${!node_ids[@]}"; do
    poll_node_once "$idx"
  done
}

compute_lag_p95() {
  local lag_count
  lag_count=$(wc -l < "$lag_values_file" | tr -d ' ')
  if [[ -z "$lag_count" ]]; then
    lag_count=0
  fi
  if (( lag_count <= 0 )); then
    analysis_lag_p95=0
    return 0
  fi

  local sorted_lag_file="$run_dir/.lag_values.sorted.txt"
  sort -n "$lag_values_file" > "$sorted_lag_file"
  local p95_rank=$(( (95 * lag_count + 99) / 100 ))
  if (( p95_rank < 1 )); then
    p95_rank=1
  fi
  analysis_lag_p95=$(sed -n "${p95_rank}p" "$sorted_lag_file")
  analysis_lag_p95=$(safe_int "$analysis_lag_p95")
}

finalize_metric_gate() {
  compute_lag_p95

  if (( ${#node_monotonic_violations[@]} > 0 )); then
    analysis_monotonic_ok=false
    analysis_monotonic_violation_nodes=$(printf '%s\n' "${!node_monotonic_violations[@]}" | sort | paste -sd ',' -)
  fi

  local -a gate_failures=()
  local -a gate_warnings=()
  local -a gate_data_warnings=()

  if (( analysis_report_count <= 0 )); then
    gate_failures+=("no_samples")
  fi

  if (( analysis_status_samples_ok <= 0 )); then
    gate_failures+=("status_endpoint_unreachable")
  fi

  if (( analysis_balances_samples_ok <= 0 )); then
    gate_failures+=("balances_endpoint_unreachable")
  fi

  if (( analysis_running_false_samples > 0 )); then
    gate_failures+=("running_false_samples=${analysis_running_false_samples}")
  fi

  if (( analysis_last_error_samples > 0 )); then
    gate_failures+=("last_error_samples=${analysis_last_error_samples}")
  fi

  if (( analysis_max_stall_secs_observed > max_stall_secs )); then
    gate_failures+=("stall=${analysis_max_stall_secs_observed}s>max_${max_stall_secs}s")
  fi

  if (( analysis_lag_p95 > max_lag_p95 )); then
    gate_failures+=("lag_p95=${analysis_lag_p95}>max_${max_lag_p95}")
  fi

  if (( analysis_minted_non_empty_samples <= 0 )); then
    gate_failures+=("minted_records_empty")
  fi

  if [[ "$analysis_monotonic_ok" != "true" ]]; then
    gate_failures+=("committed_height_not_monotonic nodes=${analysis_monotonic_violation_nodes}")
  fi

  analysis_distfs_total_checks=0
  analysis_distfs_failed_checks=0
  analysis_settlement_apply_attempts=0
  analysis_settlement_apply_failures=0
  local node_name
  for node_name in "${node_ids[@]}"; do
    analysis_distfs_total_checks=$((analysis_distfs_total_checks + ${node_distfs_total_checks_max[$node_name]:-0}))
    analysis_distfs_failed_checks=$((analysis_distfs_failed_checks + ${node_distfs_failed_checks_max[$node_name]:-0}))
    analysis_settlement_apply_attempts=$((analysis_settlement_apply_attempts + ${node_settlement_apply_attempts_max[$node_name]:-0}))
    analysis_settlement_apply_failures=$((analysis_settlement_apply_failures + ${node_settlement_apply_failures_max[$node_name]:-0}))
  done
  analysis_distfs_failure_ratio="0.000000"
  analysis_settlement_apply_failure_ratio="0.000000"
  if (( analysis_distfs_total_checks > 0 )); then
    analysis_distfs_failure_ratio=$(awk -v failed="$analysis_distfs_failed_checks" -v total="$analysis_distfs_total_checks" 'BEGIN { printf "%.6f", failed / total }')
    if awk -v ratio="$analysis_distfs_failure_ratio" -v max="$max_distfs_failure_ratio" 'BEGIN { exit !(ratio > max) }'; then
      gate_failures+=("distfs_failure_ratio=${analysis_distfs_failure_ratio}>max_${max_distfs_failure_ratio}")
    fi
  else
    gate_data_warnings+=("distfs_metrics_unavailable")
  fi
  if (( analysis_settlement_apply_attempts > 0 )); then
    analysis_settlement_apply_failure_ratio=$(awk -v failed="$analysis_settlement_apply_failures" -v total="$analysis_settlement_apply_attempts" 'BEGIN { printf "%.6f", failed / total }')
    if awk -v ratio="$analysis_settlement_apply_failure_ratio" -v max="$max_settlement_apply_failure_ratio" 'BEGIN { exit !(ratio > max) }'; then
      gate_failures+=("settlement_apply_failure_ratio=${analysis_settlement_apply_failure_ratio}>max_${max_settlement_apply_failure_ratio}")
    fi
  else
    gate_data_warnings+=("settlement_apply_metrics_unavailable")
  fi
  if [[ "$analysis_invariant_all_ok" != "true" ]]; then
    gate_failures+=("reward_asset_invariant_violation")
  fi

  if (( analysis_balance_load_error_samples > 0 )); then
    gate_warnings+=("balances_load_error_samples=${analysis_balance_load_error_samples}")
  fi
  if (( analysis_peer_zero_samples > 0 )); then
    gate_warnings+=("known_peer_heads_zero_samples=${analysis_peer_zero_samples}")
  fi
  if (( analysis_tolerated_bootstrap_fetch_commit_unavailable_samples > 2 )); then
    gate_failures+=("bootstrap_fetch_commit_protocol_unavailable_samples=${analysis_tolerated_bootstrap_fetch_commit_unavailable_samples}>max_2")
  elif (( analysis_tolerated_bootstrap_fetch_commit_unavailable_samples > 0 )); then
    gate_warnings+=("bootstrap_fetch_commit_protocol_unavailable_samples=${analysis_tolerated_bootstrap_fetch_commit_unavailable_samples}")
  fi
  if (( analysis_http_failure_samples > 0 )); then
    gate_warnings+=("http_failure_samples=${analysis_http_failure_samples}")
  fi
  if (( analysis_reward_runtime_available_samples <= 0 )); then
    gate_data_warnings+=("reward_runtime_metrics_not_ready")
  fi

  if (( ${#gate_failures[@]} > 0 )); then
    analysis_gate_status="fail"
    analysis_gate_notes=$(join_by "; " "${gate_failures[@]}")
    if (( ${#gate_data_warnings[@]} > 0 )); then
      analysis_gate_notes="${analysis_gate_notes}; $(join_by "; " "${gate_data_warnings[@]}")"
    fi
    if (( ${#gate_warnings[@]} > 0 )); then
      analysis_gate_notes="${analysis_gate_notes}; $(join_by "; " "${gate_warnings[@]}")"
    fi
  elif (( ${#gate_data_warnings[@]} > 0 )); then
    analysis_gate_status="insufficient_data"
    analysis_gate_notes=$(join_by "; " "${gate_data_warnings[@]}")
    if (( ${#gate_warnings[@]} > 0 )); then
      analysis_gate_notes="${analysis_gate_notes}; $(join_by "; " "${gate_warnings[@]}")"
    fi
  elif (( ${#gate_warnings[@]} > 0 )); then
    analysis_gate_status="pass"
    analysis_gate_notes=$(join_by "; " "${gate_warnings[@]}")
  else
    analysis_gate_status="pass"
    analysis_gate_notes="all_gates_passed"
  fi
}

write_summary_json() {
  local final_status=$1
  local process_status=$2
  local started_at=$3
  local ended_at=$4
  local notes=$5
  local overall_status_code=$6
  local generated_at
  generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

  jq -n \
    --arg generated_at "$generated_at" \
    --arg run_dir "$run_dir" \
    --arg scenario "$scenario" \
    --arg world_id "$world_id" \
    --arg summary_md "$summary_md" \
    --arg timeline_csv "$timeline_csv" \
    --arg run_config_json "$run_config_json" \
    --arg failures_md "$failures_md" \
    --arg final_status "$final_status" \
    --arg process_status "$process_status" \
    --arg started_at "$started_at" \
    --arg ended_at "$ended_at" \
    --arg notes "$notes" \
    --arg gate_status "$analysis_gate_status" \
    --arg gate_notes "$analysis_gate_notes" \
    --argjson llm_enabled "$llm_enabled" \
    --argjson duration_secs "$duration_secs" \
    --argjson max_stall_secs "$max_stall_secs" \
    --argjson max_lag_p95 "$max_lag_p95" \
    --argjson max_distfs_failure_ratio "$max_distfs_failure_ratio" \
    --argjson max_settlement_apply_failure_ratio "$max_settlement_apply_failure_ratio" \
    --argjson report_samples "$analysis_report_count" \
    --argjson max_stall_secs_observed "$analysis_max_stall_secs_observed" \
    --argjson lag_p95 "$analysis_lag_p95" \
    --argjson distfs_failure_ratio "$analysis_distfs_failure_ratio" \
    --argjson distfs_total_checks "$analysis_distfs_total_checks" \
    --argjson distfs_failed_checks "$analysis_distfs_failed_checks" \
    --argjson settlement_apply_failure_ratio "$analysis_settlement_apply_failure_ratio" \
    --argjson settlement_apply_attempts "$analysis_settlement_apply_attempts" \
    --argjson settlement_apply_failures "$analysis_settlement_apply_failures" \
    --argjson invariant_all_ok "$analysis_invariant_all_ok" \
    --argjson settlement_positive_samples "$analysis_settlement_positive_samples" \
    --argjson minted_non_empty_samples "$analysis_minted_non_empty_samples" \
    --argjson monotonic_ok "$analysis_monotonic_ok" \
    --arg monotonic_violation_nodes "$analysis_monotonic_violation_nodes" \
    --argjson status_samples_ok "$analysis_status_samples_ok" \
    --argjson balances_samples_ok "$analysis_balances_samples_ok" \
    --argjson running_false_samples "$analysis_running_false_samples" \
    --argjson last_error_samples "$analysis_last_error_samples" \
    --argjson tolerated_bootstrap_fetch_commit_unavailable_samples "$analysis_tolerated_bootstrap_fetch_commit_unavailable_samples" \
    --argjson balance_load_error_samples "$analysis_balance_load_error_samples" \
    --argjson peer_zero_samples "$analysis_peer_zero_samples" \
    --argjson http_failure_samples "$analysis_http_failure_samples" \
    --argjson reward_runtime_available_samples "$analysis_reward_runtime_available_samples" \
    --argjson overall_status_code "$overall_status_code" \
    '{
      generated_at_utc: $generated_at,
      run_dir: $run_dir,
      scenario: $scenario,
      world_id: $world_id,
      llm_enabled_compat: ($llm_enabled == 1),
      duration_secs: $duration_secs,
      thresholds: {
        max_stall_secs: $max_stall_secs,
        max_lag_p95: $max_lag_p95,
        max_distfs_failure_ratio: $max_distfs_failure_ratio,
        max_settlement_apply_failure_ratio: $max_settlement_apply_failure_ratio
      },
      artifacts: {
        run_config_json: $run_config_json,
        timeline_csv: $timeline_csv,
        summary_md: $summary_md,
        failures_md: (if $overall_status_code == 0 then null else $failures_md end)
      },
      run: {
        status: $final_status,
        process_status: $process_status,
        started_at: $started_at,
        ended_at: $ended_at,
        notes: $notes,
        report_samples: $report_samples,
        metric_gate: {
          status: $gate_status,
          notes: $gate_notes
        },
        metrics: {
          max_stall_secs_observed: $max_stall_secs_observed,
          lag_p95: $lag_p95,
          distfs_failure_ratio: $distfs_failure_ratio,
          distfs_total_checks: $distfs_total_checks,
          distfs_failed_checks: $distfs_failed_checks,
          settlement_apply_failure_ratio: $settlement_apply_failure_ratio,
          settlement_apply_attempts: $settlement_apply_attempts,
          settlement_apply_failures: $settlement_apply_failures,
          invariant_all_ok: $invariant_all_ok,
          settlement_positive_samples: $settlement_positive_samples,
          minted_non_empty_samples: $minted_non_empty_samples,
          status_samples_ok: $status_samples_ok,
          balances_samples_ok: $balances_samples_ok,
          running_false_samples: $running_false_samples,
          last_error_samples: $last_error_samples,
          tolerated_bootstrap_fetch_commit_unavailable_samples: $tolerated_bootstrap_fetch_commit_unavailable_samples,
          balance_load_error_samples: $balance_load_error_samples,
          known_peer_heads_zero_samples: $peer_zero_samples,
          http_failure_samples: $http_failure_samples,
          reward_runtime_available_samples: $reward_runtime_available_samples,
          committed_height_monotonic: $monotonic_ok,
          committed_height_monotonic_violation_nodes: (
            if $monotonic_violation_nodes == "" then []
            else ($monotonic_violation_nodes | split(","))
            end
          )
        }
      },
      overall_status: (if $overall_status_code == 0 then "ok" else "failed" end)
    }' > "$summary_json"
}

append_summary_metrics_section() {
  {
    echo
    echo "## Metrics Artifacts"
    echo
    echo "- timeline_csv: \`$timeline_csv\`"
    echo "- summary_json: \`$summary_json\`"
    echo "- report_samples: \`$analysis_report_count\`"
    echo
    echo "## Gate Metrics"
    echo
    echo "| metric | value |"
    echo "|---|---|"
    echo "| metric_gate | $analysis_gate_status |"
    echo "| metric_gate_notes | $analysis_gate_notes |"
    echo "| max_stall_secs_observed | $analysis_max_stall_secs_observed |"
    echo "| lag_p95 | $analysis_lag_p95 |"
    echo "| status_samples_ok | $analysis_status_samples_ok |"
    echo "| balances_samples_ok | $analysis_balances_samples_ok |"
  echo "| running_false_samples | $analysis_running_false_samples |"
  echo "| last_error_samples | $analysis_last_error_samples |"
  echo "| tolerated_bootstrap_fetch_commit_unavailable_samples | $analysis_tolerated_bootstrap_fetch_commit_unavailable_samples |"
  echo "| known_peer_heads_zero_samples | $analysis_peer_zero_samples |"
    echo "| minted_non_empty_samples | $analysis_minted_non_empty_samples |"
    echo "| settlement_positive_samples | $analysis_settlement_positive_samples |"
    echo "| reward_runtime_available_samples | $analysis_reward_runtime_available_samples |"
    echo "| distfs_failure_ratio | $analysis_distfs_failure_ratio |"
    echo "| settlement_apply_failure_ratio | $analysis_settlement_apply_failure_ratio |"
    echo "| committed_height_monotonic | $analysis_monotonic_ok |"
    echo "| committed_height_monotonic_violation_nodes | ${analysis_monotonic_violation_nodes:--} |"
  } >> "$summary_md"
}

write_failures_md() {
  local final_status=$1
  local process_status=$2
  local notes=$3
  local overall_status_code=$4

  if (( overall_status_code == 0 )); then
    rm -f "$failures_md"
    return 0
  fi

  {
    echo "# S10 Five-Node Real Game Soak Failures"
    echo
    echo "- run_dir: \`$run_dir\`"
    echo "- world_id: \`$world_id\`"
    echo "- scenario: \`$scenario\`"
    echo "- final_status: \`$final_status\`"
    echo "- process_status: \`$process_status\`"
    echo "- gate_status: \`$analysis_gate_status\`"
    echo "- notes: \`$notes\`"
    echo
    echo "## Gate Notes"
    echo
    echo "- \`$analysis_gate_notes\`"
    if [[ -s "$runtime_errors_tsv" ]]; then
      echo
      echo "## Runtime Errors"
      while IFS=$'\t' read -r node err; do
        echo "- node=\`$node\` error=\`$err\`"
      done < "$runtime_errors_tsv"
    fi
  } > "$failures_md"
}

if [[ "$dry_run" -eq 1 ]]; then
  for idx in "${!node_ids[@]}"; do
    node_name=${node_ids[$idx]}
    node_dir="$run_dir/nodes/$node_name"
    cmd_txt="$node_dir/command.txt"
    run mkdir -p "$node_dir"
    prepare_node_command "$idx"
    printf '%q ' "${prepared_cmd[@]}" > "$cmd_txt"
    printf '\n' >> "$cmd_txt"
    echo "+ dry-run command[$node_name]: ${prepared_cmd[*]}"
  done

  append_summary_row "five_node_real_game" "dry_run" "dry_run" "dry_run" "0" "-" "-" "commands_rendered_only"
  jq -n \
    --arg run_dir "$run_dir" \
    --arg scenario "$scenario" \
    --arg world_id "$world_id" \
    --arg summary_md "$summary_md" \
    --arg summary_json "$summary_json" \
    --arg timeline_csv "$timeline_csv" \
    --arg run_config_json "$run_config_json" \
    --argjson llm_enabled "$llm_enabled" \
    --argjson duration_secs "$duration_secs" \
    '{
      run_dir: $run_dir,
      scenario: $scenario,
      world_id: $world_id,
      llm_enabled_compat: ($llm_enabled == 1),
      duration_secs: $duration_secs,
      artifacts: {
        run_config_json: $run_config_json,
        timeline_csv: $timeline_csv,
        summary_md: $summary_md
      },
      run: {
        status: "dry_run",
        process_status: "dry_run",
        metric_gate: {
          status: "dry_run",
          notes: "commands_rendered_only"
        },
        report_samples: 0
      },
      overall_status: "dry_run"
    }' > "$summary_json"
  rm -f "$failures_md"

  echo "dry-run completed:"
  echo "  run_dir: $run_dir"
  echo "  run_config: $run_config_json"
  echo "  summary: $summary_md"
  echo "  summary_json: $summary_json"
  exit 0
fi

if [[ "$isolate_node_state" -eq 1 ]]; then
  isolate_node_state_dirs
fi

started_at=$(date '+%Y-%m-%d %H:%M:%S %Z')
run_status="ok"
run_notes="-"

for idx in "${!node_ids[@]}"; do
  prepare_node_command "$idx"
  launch_node "${node_ids[$idx]}" "${prepared_cmd[@]}"
done

if ! wait_for_startup_ready; then
  run_status="startup_failed"
  run_notes="failed to reach /healthz across all nodes"
fi

if [[ "$run_status" == "ok" ]]; then
  last_progress_epoch_sec=$(date +%s)
  started_epoch_sec=$(date +%s)
  deadline=$((started_epoch_sec + duration_secs))
  while (( $(date +%s) < deadline )); do
    poll_all_nodes_once

    for idx in "${!active_pids[@]}"; do
      if ! kill -0 "${active_pids[$idx]}" >/dev/null 2>&1; then
        capture_process_exit_status "$idx"
        exit_status=$captured_exit_status
        run_status="process_exit"
        run_notes="node=${active_nodes[$idx]} exited during soak exit_status=${exit_status}"
        break 2
      fi
    done
    sleep "$poll_interval_secs"
  done

  if [[ "$run_status" == "ok" ]]; then
    poll_all_nodes_once
  fi
fi

stop_active_processes
ended_at=$(date '+%Y-%m-%d %H:%M:%S %Z')
process_status=$run_status

finalize_metric_gate

final_status=$run_status
declare -a notes_parts=()
if [[ "$run_notes" != "-" ]]; then
  notes_parts+=("$run_notes")
fi
if [[ "$analysis_gate_status" == "fail" ]]; then
  notes_parts+=("metric_gate=$analysis_gate_notes")
  if [[ "$final_status" == "ok" ]]; then
    final_status="metric_gate_failed"
  fi
elif [[ "$analysis_gate_status" == "insufficient_data" ]]; then
  notes_parts+=("metric_data=$analysis_gate_notes")
fi

if (( ${#notes_parts[@]} == 0 )); then
  notes="-"
else
  notes=$(join_by "; " "${notes_parts[@]}")
fi

overall_status=0
if [[ "$final_status" != "ok" ]]; then
  overall_status=1
fi

append_summary_row "five_node_real_game" "$final_status" "$process_status" "$analysis_gate_status" "$analysis_report_count" "$started_at" "$ended_at" "$notes"
write_summary_json "$final_status" "$process_status" "$started_at" "$ended_at" "$notes" "$overall_status"
append_summary_metrics_section
write_failures_md "$final_status" "$process_status" "$notes" "$overall_status"

echo "S10 soak run completed:"
echo "  run_dir: $run_dir"
echo "  summary: $summary_md"
echo "  summary_json: $summary_json"
echo "  timeline_csv: $timeline_csv"
if [[ -f "$failures_md" ]]; then
  echo "  failures: $failures_md"
fi

exit "$overall_status"
