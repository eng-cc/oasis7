#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-longrun-soak.sh [options]

Status:
  active (2026-02-28): this script runs p2p soak topologies via oasis7_chain_runtime.

Options:
  --profile <name>                 soak_smoke | soak_endurance | soak_release (default: soak_smoke)
  --duration-secs <n>              override per-topology soak duration seconds
  --topologies <csv>               comma-separated topologies: triad,triad_distributed
  --scenario <name>                legacy compatibility label (recorded only, default: triad_p2p_bootstrap)
  --llm                            legacy compatibility flag (no effect, recorded only)
  --no-llm                         legacy compatibility flag (default)
  --base-port <n>                  base port for per-topology allocation (default: 5610)
  --bind-host <host>               bind host for gossip/status endpoints (default: 127.0.0.1)
  --out-dir <path>                 output root (default: .tmp/p2p_longrun)
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
  --chaos-plan <path>              JSON chaos plan for restart/pause/disconnect injections
  --chaos-continuous-enable        continuously inject chaos events during soak window
  --chaos-continuous-interval-secs <n>
                                   interval seconds between continuous chaos events (default: 30)
  --chaos-continuous-start-sec <n> start second offset for continuous chaos injection (default: 30)
  --chaos-continuous-max-events <n>
                                   max continuous events per topology, 0 = unlimited (default: 0)
  --chaos-continuous-actions <csv> comma-separated actions from restart,pause,disconnect (default: restart,pause)
  --chaos-continuous-seed <n>      deterministic seed for continuous chaos selection (default: unix timestamp)
  --chaos-continuous-restart-down-secs <n>
                                   down seconds for generated restart events (default: 1)
  --chaos-continuous-pause-duration-secs <n>
                                   pause duration seconds for generated pause/disconnect events (default: 2)
  --reward-runtime-epoch-duration-secs <n>
                                   reward runtime epoch duration seconds (default: 60)
  --reward-points-per-credit <n>   reward points per credit (default: 100)
  --feedback-events-enable          continuously inject feedback submit events during soak
  --feedback-events-interval-secs <n>
                                   interval seconds between feedback submissions (default: 60)
  --feedback-events-start-sec <n>   start second offset for feedback submissions (default: 30)
  --feedback-events-max-events <n>
                                   max feedback events per topology, 0 = unlimited (default: 0)
  --max-stall-secs <n>             gate threshold for max no-progress window
  --max-lag-p95 <n>                gate threshold for p95(network_height - committed_height)
  --max-distfs-failure-ratio <r>   gate threshold for distfs failure ratio (0~1)
  --no-prewarm                     skip cargo build prewarm
  --dry-run                        render topology commands only, do not run node processes
  -h, --help                       show help

Profiles:
  soak_smoke      default duration 1500s, default topologies triad,triad_distributed
  soak_endurance  default duration 10800s, default topologies triad_distributed
  soak_release    default duration 28800s, default topologies triad_distributed

Output:
  <out-dir>/<timestamp>/
    run_config.json
    timeline.csv
    summary.json
    summary.md
    failures.md (only when failed)
    chaos_events.log
    feedback_events.log
    <topology>/nodes/<node_label>/{stdout.log,stderr.log,command.txt}
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

ensure_supported_topology() {
  local value=$1
  case "$value" in
    triad|triad_distributed) ;;
    *)
      echo "unsupported topology: $value (expected triad|triad_distributed)" >&2
      exit 2
      ;;
  esac
}

ensure_supported_chaos_action() {
  local value=$1
  case "$value" in
    restart|pause|disconnect) ;;
    *)
      echo "unsupported chaos action: $value (expected restart|pause|disconnect)" >&2
      exit 2
      ;;
  esac
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

chaos_rng_state=1
chaos_rng_seed() {
  local seed=$1
  local normalized=$((seed % 2147483647))
  if (( normalized <= 0 )); then
    normalized=$((normalized + 2147483646))
  fi
  chaos_rng_state=$normalized
}

chaos_rng_next() {
  chaos_rng_state=$(( (chaos_rng_state * 48271) % 2147483647 ))
  printf '%s' "$chaos_rng_state"
}

profile="soak_smoke"
duration_secs=""
topologies_csv=""
scenario="triad_p2p_bootstrap"
llm_enabled=0
base_port=5610
bind_host="127.0.0.1"
out_root=".tmp/p2p_longrun"
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
chaos_plan_path=""
chaos_continuous_enabled=0
chaos_continuous_interval_secs=30
chaos_continuous_start_sec=30
chaos_continuous_max_events=0
chaos_continuous_actions_csv="restart,pause"
chaos_continuous_seed=""
chaos_continuous_restart_down_secs=1
chaos_continuous_pause_duration_secs=2
reward_runtime_epoch_duration_secs=60
reward_points_per_credit=100
feedback_events_enabled=0
feedback_events_interval_secs=60
feedback_events_start_sec=30
feedback_events_max_events=0
max_stall_secs=""
max_lag_p95=""
max_distfs_failure_ratio=""
prewarm=1
dry_run=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      profile=${2:-}
      shift 2
      ;;
    --duration-secs)
      duration_secs=${2:-}
      shift 2
      ;;
    --topologies)
      topologies_csv=${2:-}
      shift 2
      ;;
    --scenario)
      scenario=${2:-}
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
    --chaos-plan)
      chaos_plan_path=${2:-}
      shift 2
      ;;
    --chaos-continuous-enable)
      chaos_continuous_enabled=1
      shift
      ;;
    --chaos-continuous-interval-secs)
      chaos_continuous_interval_secs=${2:-}
      shift 2
      ;;
    --chaos-continuous-start-sec)
      chaos_continuous_start_sec=${2:-}
      shift 2
      ;;
    --chaos-continuous-max-events)
      chaos_continuous_max_events=${2:-}
      shift 2
      ;;
    --chaos-continuous-actions)
      chaos_continuous_actions_csv=${2:-}
      shift 2
      ;;
    --chaos-continuous-seed)
      chaos_continuous_seed=${2:-}
      shift 2
      ;;
    --chaos-continuous-restart-down-secs)
      chaos_continuous_restart_down_secs=${2:-}
      shift 2
      ;;
    --chaos-continuous-pause-duration-secs)
      chaos_continuous_pause_duration_secs=${2:-}
      shift 2
      ;;
    --reward-runtime-epoch-duration-secs)
      reward_runtime_epoch_duration_secs=${2:-}
      shift 2
      ;;
    --reward-points-per-credit)
      reward_points_per_credit=${2:-}
      shift 2
      ;;
    --feedback-events-enable)
      feedback_events_enabled=1
      shift
      ;;
    --feedback-events-interval-secs)
      feedback_events_interval_secs=${2:-}
      shift 2
      ;;
    --feedback-events-start-sec)
      feedback_events_start_sec=${2:-}
      shift 2
      ;;
    --feedback-events-max-events)
      feedback_events_max_events=${2:-}
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

case "$profile" in
  soak_smoke)
    default_duration_secs=1500
    default_topologies_csv="triad,triad_distributed"
    default_max_stall_secs=300
    default_max_lag_p95=12
    default_max_distfs_failure_ratio="0.25"
    ;;
  soak_endurance)
    default_duration_secs=10800
    default_topologies_csv="triad_distributed"
    default_max_stall_secs=420
    default_max_lag_p95=8
    default_max_distfs_failure_ratio="0.15"
    ;;
  soak_release)
    default_duration_secs=28800
    default_topologies_csv="triad_distributed"
    default_max_stall_secs=600
    default_max_lag_p95=5
    default_max_distfs_failure_ratio="0.10"
    ;;
  *)
    echo "invalid --profile: $profile (expected soak_smoke|soak_endurance|soak_release)" >&2
    exit 2
    ;;
esac

if [[ -z "$duration_secs" ]]; then
  duration_secs=$default_duration_secs
fi
if [[ -z "$topologies_csv" ]]; then
  topologies_csv=$default_topologies_csv
fi
if [[ -z "$max_stall_secs" ]]; then
  max_stall_secs=$default_max_stall_secs
fi
if [[ -z "$max_lag_p95" ]]; then
  max_lag_p95=$default_max_lag_p95
fi
if [[ -z "$max_distfs_failure_ratio" ]]; then
  max_distfs_failure_ratio=$default_max_distfs_failure_ratio
fi

ensure_positive_int "--duration-secs" "$duration_secs"
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
ensure_non_negative_int "--chaos-continuous-start-sec" "$chaos_continuous_start_sec"
ensure_non_negative_int "--chaos-continuous-max-events" "$chaos_continuous_max_events"
ensure_non_negative_int "--chaos-continuous-restart-down-secs" "$chaos_continuous_restart_down_secs"
ensure_non_negative_int "--chaos-continuous-pause-duration-secs" "$chaos_continuous_pause_duration_secs"
ensure_positive_int "--reward-runtime-epoch-duration-secs" "$reward_runtime_epoch_duration_secs"
ensure_positive_int "--reward-points-per-credit" "$reward_points_per_credit"
ensure_non_negative_int "--feedback-events-start-sec" "$feedback_events_start_sec"
ensure_non_negative_int "--feedback-events-max-events" "$feedback_events_max_events"
if [[ "$chaos_continuous_enabled" -eq 1 ]]; then
  ensure_positive_int "--chaos-continuous-interval-secs" "$chaos_continuous_interval_secs"
fi
if [[ "$feedback_events_enabled" -eq 1 ]]; then
  ensure_positive_int "--feedback-events-interval-secs" "$feedback_events_interval_secs"
fi

scenario=$(trim "$scenario")
if [[ -z "$scenario" ]]; then
  echo "--scenario cannot be empty" >&2
  exit 2
fi

if [[ -n "$chaos_plan_path" ]]; then
  if [[ ! -f "$chaos_plan_path" ]]; then
    echo "chaos plan file not found: $chaos_plan_path" >&2
    exit 2
  fi
  if ! jq -e '(.events // []) | type == "array"' "$chaos_plan_path" >/dev/null; then
    echo "invalid chaos plan format: expected JSON object with .events array" >&2
    exit 2
  fi
fi

declare -a chaos_continuous_actions=()
if [[ "$chaos_continuous_enabled" -eq 1 ]]; then
  if [[ -z "$chaos_continuous_seed" ]]; then
    chaos_continuous_seed=$(date +%s)
  fi
  ensure_non_negative_int "--chaos-continuous-seed" "$chaos_continuous_seed"

  mapfile -t chaos_continuous_actions < <(printf '%s' "$chaos_continuous_actions_csv" | tr ',' '\n' | sed '/^$/d')
  if (( ${#chaos_continuous_actions[@]} == 0 )); then
    echo "--chaos-continuous-actions resolved to empty list" >&2
    exit 2
  fi
  for i in "${!chaos_continuous_actions[@]}"; do
    chaos_continuous_actions[$i]=$(trim "${chaos_continuous_actions[$i]}")
    ensure_supported_chaos_action "${chaos_continuous_actions[$i]}"
  done
else
  chaos_continuous_seed=0
fi

mapfile -t topologies < <(printf '%s' "$topologies_csv" | tr ',' '\n' | sed '/^$/d')
if (( ${#topologies[@]} == 0 )); then
  echo "--topologies resolved to empty list" >&2
  exit 2
fi
for i in "${!topologies[@]}"; do
  topologies[$i]=$(trim "${topologies[$i]}")
  ensure_supported_topology "${topologies[$i]}"
done

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

run_config_json="$run_dir/run_config.json"
summary_md="$run_dir/summary.md"
timeline_csv="$run_dir/timeline.csv"
summary_json="$run_dir/summary.json"
failures_md="$run_dir/failures.md"
chaos_events_log="$run_dir/chaos_events.log"
feedback_events_log="$run_dir/feedback_events.log"
topology_summary_ndjson="$run_dir/.topology_summary.ndjson"

jq -n \
  --arg profile "$profile" \
  --arg scenario "$scenario" \
  --arg bind_host "$bind_host" \
  --arg out_dir "$out_root" \
  --arg topologies_csv "$topologies_csv" \
  --arg chaos_plan_path "$chaos_plan_path" \
  --arg chaos_continuous_actions_csv "$chaos_continuous_actions_csv" \
  --argjson llm_enabled "$llm_enabled" \
  --argjson duration_secs "$duration_secs" \
  --argjson base_port "$base_port" \
  --argjson startup_timeout_secs "$startup_timeout_secs" \
  --argjson poll_interval_secs "$poll_interval_secs" \
  --argjson curl_timeout_secs "$curl_timeout_secs" \
  --argjson node_tick_ms "$node_tick_ms" \
  --argjson pos_slot_duration_ms "$pos_slot_duration_ms" \
  --argjson pos_ticks_per_slot "$pos_ticks_per_slot" \
  --argjson pos_proposal_tick_phase "$pos_proposal_tick_phase" \
  --argjson pos_adaptive_tick_scheduler_enabled "$pos_adaptive_tick_scheduler_enabled" \
  --arg pos_slot_clock_genesis_unix_ms "$pos_slot_clock_genesis_unix_ms" \
  --argjson pos_max_past_slot_lag "$pos_max_past_slot_lag" \
  --argjson max_stall_secs "$max_stall_secs" \
  --argjson max_lag_p95 "$max_lag_p95" \
  --argjson max_distfs_failure_ratio "$max_distfs_failure_ratio" \
  --argjson chaos_continuous_enabled "$chaos_continuous_enabled" \
  --argjson chaos_continuous_interval_secs "$chaos_continuous_interval_secs" \
  --argjson chaos_continuous_start_sec "$chaos_continuous_start_sec" \
  --argjson chaos_continuous_max_events "$chaos_continuous_max_events" \
  --argjson chaos_continuous_seed "$chaos_continuous_seed" \
  --argjson chaos_continuous_restart_down_secs "$chaos_continuous_restart_down_secs" \
  --argjson chaos_continuous_pause_duration_secs "$chaos_continuous_pause_duration_secs" \
  --argjson reward_runtime_epoch_duration_secs "$reward_runtime_epoch_duration_secs" \
  --argjson reward_points_per_credit "$reward_points_per_credit" \
  --argjson feedback_events_enabled "$feedback_events_enabled" \
  --argjson feedback_events_interval_secs "$feedback_events_interval_secs" \
  --argjson feedback_events_start_sec "$feedback_events_start_sec" \
  --argjson feedback_events_max_events "$feedback_events_max_events" \
  --argjson dry_run "$dry_run" \
  --argjson topologies "$(printf '%s\n' "${topologies[@]}" | jq -R -s 'split("\n") | map(select(length > 0))')" \
  '{
    profile: $profile,
    scenario_compat: $scenario,
    llm_enabled_compat: ($llm_enabled == 1),
    duration_secs: $duration_secs,
    bind_host: $bind_host,
    base_port: $base_port,
    startup_timeout_secs: $startup_timeout_secs,
    poll_interval_secs: $poll_interval_secs,
    curl_timeout_secs: $curl_timeout_secs,
    node_tick_ms: $node_tick_ms,
    pos_config: {
      slot_duration_ms: $pos_slot_duration_ms,
      ticks_per_slot: $pos_ticks_per_slot,
      proposal_tick_phase: $pos_proposal_tick_phase,
      adaptive_tick_scheduler_enabled: ($pos_adaptive_tick_scheduler_enabled == 1),
      slot_clock_genesis_unix_ms: (if $pos_slot_clock_genesis_unix_ms == "" then null else ($pos_slot_clock_genesis_unix_ms | tonumber) end),
      max_past_slot_lag: $pos_max_past_slot_lag
    },
    reward_runtime_epoch_duration_secs: $reward_runtime_epoch_duration_secs,
    reward_points_per_credit: $reward_points_per_credit,
    feedback_events: {
      enabled: ($feedback_events_enabled == 1),
      interval_secs: $feedback_events_interval_secs,
      start_sec: $feedback_events_start_sec,
      max_events: $feedback_events_max_events
    },
    thresholds: {
      max_stall_secs: $max_stall_secs,
      max_lag_p95: $max_lag_p95,
      max_distfs_failure_ratio: $max_distfs_failure_ratio
    },
    topologies: $topologies,
    topologies_csv: $topologies_csv,
    chaos: {
      enabled: ($chaos_plan_path != "" or $chaos_continuous_enabled == 1),
      plan_path: (if $chaos_plan_path == "" then null else $chaos_plan_path end),
      continuous_enabled: ($chaos_continuous_enabled == 1),
      continuous_interval_secs: $chaos_continuous_interval_secs,
      continuous_start_sec: $chaos_continuous_start_sec,
      continuous_max_events: $chaos_continuous_max_events,
      continuous_actions_csv: $chaos_continuous_actions_csv,
      continuous_seed: (if $chaos_continuous_enabled == 1 then $chaos_continuous_seed else null end),
      continuous_restart_down_secs: $chaos_continuous_restart_down_secs,
      continuous_pause_duration_secs: $chaos_continuous_pause_duration_secs
    },
    dry_run: ($dry_run == 1),
    compatibility_notes: [
      "--scenario/--llm are accepted but no longer affect oasis7_chain_runtime topology",
      "node_tick_ms is worker poll/fallback interval; PoS slot timing is configured by pos_config.*"
    ]
  }' > "$run_config_json"

{
  echo "# P2P Longrun Soak Summary"
  echo
  echo "- run_dir: \`$run_dir\`"
  echo "- profile: \`$profile\`"
  echo "- duration_secs_per_topology: \`$duration_secs\`"
  echo "- scenario_compat: \`$scenario\`"
  echo "- llm_enabled_compat: \`$llm_enabled\`"
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
  echo "- reward_runtime_epoch_duration_secs: \`$reward_runtime_epoch_duration_secs\`"
  echo "- reward_points_per_credit: \`$reward_points_per_credit\`"
  if [[ "$feedback_events_enabled" -eq 1 ]]; then
    echo "- feedback_events: \`enabled\` (interval=${feedback_events_interval_secs}s, start=${feedback_events_start_sec}s, max_events=${feedback_events_max_events})"
  else
    echo "- feedback_events: \`disabled\`"
  fi
  echo "- max_stall_secs: \`$max_stall_secs\`"
  echo "- max_lag_p95: \`$max_lag_p95\`"
  echo "- max_distfs_failure_ratio: \`$max_distfs_failure_ratio\`"
  if [[ -n "$chaos_plan_path" ]]; then
    echo "- chaos_plan: \`$chaos_plan_path\`"
  else
    echo "- chaos_plan: \`disabled\`"
  fi
  if [[ "$chaos_continuous_enabled" -eq 1 ]]; then
    echo "- chaos_continuous: \`enabled\` (interval=${chaos_continuous_interval_secs}s, start=${chaos_continuous_start_sec}s, max_events=${chaos_continuous_max_events}, actions=${chaos_continuous_actions_csv}, seed=${chaos_continuous_seed})"
    echo "- chaos_continuous_durations: \`restart_down=${chaos_continuous_restart_down_secs}s,pause=${chaos_continuous_pause_duration_secs}s\`"
  else
    echo "- chaos_continuous: \`disabled\`"
  fi
  echo
  echo "| topology | status | process_status | metric_gate | reports | started_at | ended_at | notes |"
  echo "|---|---|---|---|---|---|---|---|"
} > "$summary_md"

echo "topology,node,epoch_index,observed_at_unix_ms,committed_height,network_committed_height,lag,total_checks,failed_checks,distfs_failure_ratio,invariant_ok,last_block_hash,last_execution_block_hash,last_execution_state_root,report_path" > "$timeline_csv"
: > "$topology_summary_ndjson"
echo "timestamp|topology|event_id|phase|action|node|detail" > "$chaos_events_log"
echo "timestamp|topology|event_id|phase|node|category|detail" > "$feedback_events_log"

append_summary_row() {
  local topology=$1
  local status=$2
  local process_status=$3
  local metric_gate=$4
  local reports=$5
  local started_at=$6
  local ended_at=$7
  local notes=$8
  echo "| $topology | $status | $process_status | $metric_gate | $reports | $started_at | $ended_at | $notes |" >> "$summary_md"
}

active_cleanup_done=0
declare -a active_pids=()
declare -a active_nodes=()
declare -A node_cmd_file_by_name=()
declare -A node_stdout_log_by_name=()
declare -A node_stderr_log_by_name=()
declare -A node_status_url_by_name=()
declare -A node_balances_url_by_name=()
declare -A node_runtime_id_by_name=()

declare -A chaos_exempt_secs_by_topology=()
declare -A chaos_events_executed_by_topology=()
declare -A chaos_plan_events_executed_by_topology=()
declare -A chaos_continuous_events_executed_by_topology=()
declare -A feedback_events_executed_by_topology=()
declare -A feedback_events_success_by_topology=()
declare -A feedback_events_failed_by_topology=()

stop_active_processes() {
  if [[ "$active_cleanup_done" -eq 1 ]]; then
    return 0
  fi
  active_cleanup_done=1

  local pid
  for pid in "${active_pids[@]}"; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
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
      break
    fi
    if (( $(date +%s) >= deadline )); then
      for pid in "${active_pids[@]}"; do
        if kill -0 "$pid" >/dev/null 2>&1; then
          kill -9 "$pid" >/dev/null 2>&1 || true
        fi
      done
      break
    fi
    sleep 1
  done

  for pid in "${active_pids[@]}"; do
    wait "$pid" >/dev/null 2>&1 || true
  done
}

cleanup_on_exit() {
  stop_active_processes
}
trap cleanup_on_exit EXIT

reset_active_topology_state() {
  active_cleanup_done=0
  active_pids=()
  active_nodes=()
  node_cmd_file_by_name=()
  node_stdout_log_by_name=()
  node_stderr_log_by_name=()
  node_status_url_by_name=()
  node_balances_url_by_name=()
  node_runtime_id_by_name=()
}

find_node_index_by_name() {
  local target_node=$1
  local idx
  for idx in "${!active_nodes[@]}"; do
    if [[ "${active_nodes[$idx]}" == "$target_node" ]]; then
      printf '%s' "$idx"
      return 0
    fi
  done
  return 1
}

log_chaos_event() {
  local topology=$1
  local event_id=$2
  local phase=$3
  local action=$4
  local node=$5
  local detail=$6
  local ts
  ts=$(date '+%Y-%m-%d %H:%M:%S %Z')
  echo "$ts|$topology|$event_id|$phase|$action|$node|$detail" >> "$chaos_events_log"
}

log_feedback_event() {
  local topology=$1
  local event_id=$2
  local phase=$3
  local node=$4
  local category=$5
  local detail=$6
  local ts
  ts=$(date '+%Y-%m-%d %H:%M:%S %Z')
  echo "$ts|$topology|$event_id|$phase|$node|$category|$detail" >> "$feedback_events_log"
}

execute_feedback_submit_event() {
  local topology=$1
  local event_id=$2
  local node_name=$3
  local category=$4
  local at_sec=$5
  local status_url=${node_status_url_by_name[$node_name]:-}
  local submit_url payload response_with_code response_code response_body
  local response_ok feedback_id response_event_id error_text

  if [[ -z "$status_url" ]]; then
    log_feedback_event "$topology" "$event_id" "failed" "$node_name" "$category" "status_url_not_found"
    return 1
  fi
  submit_url="${status_url%/v1/chain/status}/v1/chain/feedback/submit"
  payload=$(jq -cn \
    --arg category "$category" \
    --arg title "soak-feedback-${topology}-${event_id}" \
    --arg description "p2p-longrun feedback event topology=${topology} node=${node_name} at_sec=${at_sec}" \
    --arg platform "p2p_longrun_soak" \
    --arg game_version "soak" \
    '{category: $category, title: $title, description: $description, platform: $platform, game_version: $game_version}')

  log_feedback_event "$topology" "$event_id" "start" "$node_name" "$category" "at_sec=$at_sec,url=$submit_url"

  response_with_code=$(curl -sS --max-time "$curl_timeout_secs" \
    -X POST "$submit_url" \
    -H 'Content-Type: application/json' \
    --data "$payload" \
    -w $'\n%{http_code}' 2>/dev/null || true)
  if [[ -z "$response_with_code" ]]; then
    log_feedback_event "$topology" "$event_id" "failed" "$node_name" "$category" "empty_http_response"
    return 1
  fi

  response_code=${response_with_code##*$'\n'}
  response_body=${response_with_code%$'\n'*}
  response_ok=$(jq -r '.ok // false' <<< "$response_body" 2>/dev/null || echo "false")
  if [[ "$response_code" =~ ^2[0-9][0-9]$ ]] && [[ "$response_ok" == "true" ]]; then
    feedback_id=$(jq -r '.feedback_id // empty' <<< "$response_body" 2>/dev/null || true)
    response_event_id=$(jq -r '.event_id // empty' <<< "$response_body" 2>/dev/null || true)
    log_feedback_event \
      "$topology" \
      "$event_id" \
      "completed" \
      "$node_name" \
      "$category" \
      "http=${response_code},feedback_id=${feedback_id:-none},event_id=${response_event_id:-none}"
    return 0
  fi

  error_text=$(jq -r '.error // empty' <<< "$response_body" 2>/dev/null || true)
  log_feedback_event \
    "$topology" \
    "$event_id" \
    "failed" \
    "$node_name" \
    "$category" \
    "http=${response_code},error=${error_text:-unknown}"
  return 1
}

relaunch_node_from_saved_command() {
  local node_name=$1
  local idx cmd_txt stdout_log stderr_log cmd_line

  if ! idx=$(find_node_index_by_name "$node_name"); then
    echo "node not found for relaunch: $node_name" >&2
    return 1
  fi

  cmd_txt=${node_cmd_file_by_name[$node_name]:-}
  stdout_log=${node_stdout_log_by_name[$node_name]:-}
  stderr_log=${node_stderr_log_by_name[$node_name]:-}
  if [[ -z "$cmd_txt" ]] || [[ -z "$stdout_log" ]] || [[ -z "$stderr_log" ]]; then
    echo "missing node command metadata for relaunch: $node_name" >&2
    return 1
  fi

  cmd_line=$(tr -d '\n' < "$cmd_txt")
  if [[ -z "$cmd_line" ]]; then
    echo "empty command file for relaunch: $cmd_txt" >&2
    return 1
  fi

  echo "+ $cmd_line >> $stdout_log 2>> $stderr_log"
  bash -lc "$cmd_line" >>"$stdout_log" 2>>"$stderr_log" &
  active_pids[$idx]=$!
  return 0
}

execute_chaos_event() {
  local topology=$1
  local event_id=$2
  local action=$3
  local node_name=$4
  local at_sec=$5
  local down_secs=$6
  local duration_secs=$7
  local idx pid

  if ! idx=$(find_node_index_by_name "$node_name"); then
    log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "node_not_found"
    return 1
  fi
  pid=${active_pids[$idx]}

  case "$action" in
    restart)
      log_chaos_event "$topology" "$event_id" "start" "$action" "$node_name" "at_sec=$at_sec,down_secs=$down_secs,pid=$pid"
      if ! kill -0 "$pid" >/dev/null 2>&1; then
        log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "pid_not_alive=$pid"
        return 1
      fi
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" >/dev/null 2>&1 || true
      if (( down_secs > 0 )); then
        sleep "$down_secs"
      fi
      if ! relaunch_node_from_saved_command "$node_name"; then
        log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "relaunch_failed"
        return 1
      fi
      log_chaos_event "$topology" "$event_id" "completed" "$action" "$node_name" "new_pid=${active_pids[$idx]}"
      ;;
    pause|disconnect)
      log_chaos_event "$topology" "$event_id" "start" "$action" "$node_name" "at_sec=$at_sec,duration_secs=$duration_secs,pid=$pid"
      if ! kill -0 "$pid" >/dev/null 2>&1; then
        log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "pid_not_alive=$pid"
        return 1
      fi
      if ! kill -STOP "$pid" >/dev/null 2>&1; then
        log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "sigstop_failed"
        return 1
      fi
      if (( duration_secs > 0 )); then
        sleep "$duration_secs"
      fi
      if ! kill -CONT "$pid" >/dev/null 2>&1; then
        log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "sigcont_failed"
        return 1
      fi
      log_chaos_event "$topology" "$event_id" "completed" "$action" "$node_name" "pid=$pid"
      ;;
    *)
      log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_name" "unknown_action"
      return 1
      ;;
  esac

  return 0
}

launch_node() {
  local topology_dir=$1
  local node_name=$2
  local status_url=$3
  local balances_url=$4
  local runtime_id=$5
  shift 5

  local node_dir="$topology_dir/nodes/$node_name"
  local stdout_log="$node_dir/stdout.log"
  local stderr_log="$node_dir/stderr.log"
  local cmd_txt="$node_dir/command.txt"

  run mkdir -p "$node_dir"

  printf '%q ' "$@" > "$cmd_txt"
  printf '\n' >> "$cmd_txt"

  echo "+ $* > $stdout_log 2> $stderr_log"
  "$@" >"$stdout_log" 2>"$stderr_log" &
  local pid=$!

  node_cmd_file_by_name["$node_name"]="$cmd_txt"
  node_stdout_log_by_name["$node_name"]="$stdout_log"
  node_stderr_log_by_name["$node_name"]="$stderr_log"
  node_status_url_by_name["$node_name"]="$status_url"
  node_balances_url_by_name["$node_name"]="$balances_url"
  node_runtime_id_by_name["$node_name"]="$runtime_id"

  active_pids+=("$pid")
  active_nodes+=("$node_name")
}

wait_for_topology_ready() {
  local deadline=$(( $(date +%s) + startup_timeout_secs ))
  while :; do
    local all_ready=1
    local idx node_name
    for idx in "${!active_nodes[@]}"; do
      node_name=${active_nodes[$idx]}
      if ! kill -0 "${active_pids[$idx]}" >/dev/null 2>&1; then
        echo "node exited before startup ready: $node_name" >&2
        return 1
      fi
      if ! curl -fsS --max-time "$curl_timeout_secs" "${node_status_url_by_name[$node_name]%/v1/chain/status}/healthz" >/dev/null 2>&1; then
        all_ready=0
      fi
    done
    if [[ "$all_ready" -eq 1 ]]; then
      return 0
    fi
    if (( $(date +%s) >= deadline )); then
      echo "startup timeout waiting for /healthz on topology" >&2
      return 1
    fi
    sleep 1
  done
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
analysis_chaos_exempt_secs=0
analysis_effective_max_stall_secs=0
analysis_status_samples_ok=0
analysis_balances_samples_ok=0
analysis_running_false_samples=0
analysis_last_error_samples=0
analysis_balance_load_error_samples=0
analysis_peer_zero_samples=0
analysis_http_failure_samples=0
analysis_minted_non_empty_samples=0
analysis_reward_runtime_available_samples=0
analysis_monotonic_ok=true
analysis_monotonic_violation_nodes=""
analysis_consensus_hash_consistent=true
analysis_consensus_hash_mismatch_count=0
analysis_consensus_hash_mismatch_heights=""
analysis_consensus_hash_samples=0
analysis_consensus_hash_missing_samples=0
analysis_lag_values_file=""
analysis_runtime_errors_file=""
analysis_consensus_hash_mismatch_file=""
analysis_topology_dir=""
best_height=-1
last_progress_epoch_sec=0
declare -A analysis_node_prev_committed=()
declare -A analysis_node_monotonic_violations=()
declare -A analysis_node_distfs_total_checks_max=()
declare -A analysis_node_distfs_failed_checks_max=()
declare -A analysis_node_settlement_apply_attempts_max=()
declare -A analysis_node_settlement_apply_failures_max=()
declare -A analysis_node_reward_minted_count_max=()
declare -A analysis_consensus_hash_mismatch_heights_map=()
declare -A analysis_committed_height_block_hash=()
declare -A analysis_execution_height_block_hash=()
declare -A analysis_execution_height_state_root=()

reset_topology_analysis() {
  local topology=$1
  local topology_dir=$2
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
  analysis_chaos_exempt_secs=${chaos_exempt_secs_by_topology[$topology]:-0}
  analysis_effective_max_stall_secs=$((max_stall_secs + analysis_chaos_exempt_secs))
  analysis_status_samples_ok=0
  analysis_balances_samples_ok=0
  analysis_running_false_samples=0
  analysis_last_error_samples=0
  analysis_balance_load_error_samples=0
  analysis_peer_zero_samples=0
  analysis_http_failure_samples=0
  analysis_minted_non_empty_samples=0
  analysis_reward_runtime_available_samples=0
  analysis_monotonic_ok=true
  analysis_monotonic_violation_nodes=""
  analysis_consensus_hash_consistent=true
  analysis_consensus_hash_mismatch_count=0
  analysis_consensus_hash_mismatch_heights=""
  analysis_consensus_hash_samples=0
  analysis_consensus_hash_missing_samples=0
  analysis_lag_values_file="$topology_dir/.lag_values.txt"
  analysis_runtime_errors_file="$topology_dir/.runtime_errors.tsv"
  analysis_consensus_hash_mismatch_file="$topology_dir/.consensus_hash_mismatch.tsv"
  analysis_topology_dir="$topology_dir"
  : > "$analysis_lag_values_file"
  : > "$analysis_runtime_errors_file"
  : > "$analysis_consensus_hash_mismatch_file"
  best_height=-1
  last_progress_epoch_sec=$(date +%s)
  analysis_node_prev_committed=()
  analysis_node_monotonic_violations=()
  analysis_node_distfs_total_checks_max=()
  analysis_node_distfs_failed_checks_max=()
  analysis_node_settlement_apply_attempts_max=()
  analysis_node_settlement_apply_failures_max=()
  analysis_node_reward_minted_count_max=()
  analysis_consensus_hash_mismatch_heights_map=()
  analysis_committed_height_block_hash=()
  analysis_execution_height_block_hash=()
  analysis_execution_height_state_root=()
}

record_consensus_hash_sample() {
  local scope=$1
  local height=$2
  local hash_value=$3
  local node_name=$4
  local map_name=$5

  if (( height <= 0 )); then
    return 0
  fi

  analysis_consensus_hash_samples=$((analysis_consensus_hash_samples + 1))
  if [[ -z "$hash_value" ]]; then
    analysis_consensus_hash_missing_samples=$((analysis_consensus_hash_missing_samples + 1))
    return 0
  fi

  local -n hash_map_ref="$map_name"
  local expected=${hash_map_ref[$height]:-}
  if [[ -z "$expected" ]]; then
    hash_map_ref["$height"]=$hash_value
    return 0
  fi

  if [[ "$expected" != "$hash_value" ]]; then
    analysis_consensus_hash_consistent=false
    analysis_consensus_hash_mismatch_count=$((analysis_consensus_hash_mismatch_count + 1))
    analysis_consensus_hash_mismatch_heights_map["$height"]=1
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$(date +%s)" \
      "$scope" \
      "$height" \
      "$node_name" \
      "$expected" \
      "$hash_value" >> "$analysis_consensus_hash_mismatch_file"
  fi
}

append_topology_timeline_sample() {
  local topology=$1
  local node_name=$2
  local epoch_index=$3
  local observed_at_unix_ms=$4
  local committed_height=$5
  local network_committed_height=$6
  local lag=$7
  local invariant_ok=$8
  local last_block_hash=${9}
  local last_execution_block_hash=${10}
  local last_execution_state_root=${11}
  local report_path=${12}

  jq -rRn \
    --arg topology "$topology" \
    --arg node "$node_name" \
    --arg epoch "$epoch_index" \
    --arg observed "$observed_at_unix_ms" \
    --arg committed "$committed_height" \
    --arg network "$network_committed_height" \
    --arg lag "$lag" \
    --arg invariant "$invariant_ok" \
    --arg last_block_hash "$last_block_hash" \
    --arg last_execution_block_hash "$last_execution_block_hash" \
    --arg last_execution_state_root "$last_execution_state_root" \
    --arg report "$report_path" \
    '[
      $topology,
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
      $last_block_hash,
      $last_execution_block_hash,
      $last_execution_state_root,
      $report
    ] | @csv' >> "$timeline_csv"
}

poll_topology_node_once() {
  local topology=$1
  local node_name=$2
  local status_url=${node_status_url_by_name[$node_name]}
  local balances_url=${node_balances_url_by_name[$node_name]}

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
  local last_block_hash=""
  local last_execution_height=0
  local last_execution_block_hash=""
  local last_execution_state_root=""
  local rr_metrics_available="false"
  local rr_last_error=""
  local rr_distfs_total_checks=0
  local rr_distfs_failed_checks=0
  local rr_settlement_apply_attempts=0
  local rr_settlement_apply_failures=0
  local rr_minted_cumulative_count=0
  local rr_invariant_ok="true"

  if [[ "$status_ok" -eq 1 ]]; then
    observed_at_unix_ms=$(safe_int "$(jq -r '.observed_at_unix_ms // 0' <<< "$status_json")")
    epoch_index=$(safe_int "$(jq -r '.consensus.epoch // 0' <<< "$status_json")")
    committed_height=$(safe_int "$(jq -r '.consensus.committed_height // 0' <<< "$status_json")")
    network_committed_height=$(safe_int "$(jq -r '.consensus.network_committed_height // 0' <<< "$status_json")")
    running=$(jq -r '.running // false' <<< "$status_json")
    known_peer_heads=$(safe_int "$(jq -r '.consensus.known_peer_heads // 0' <<< "$status_json")")
    last_block_hash=$(jq -r '.consensus.last_block_hash // empty' <<< "$status_json")
    last_execution_height=$(safe_int "$(jq -r '.consensus.last_execution_height // 0' <<< "$status_json")")
    last_execution_block_hash=$(jq -r '.consensus.last_execution_block_hash // empty' <<< "$status_json")
    last_execution_state_root=$(jq -r '.consensus.last_execution_state_root // empty' <<< "$status_json")
    last_error=$(jq -r '.last_error // empty' <<< "$status_json")
    rr_metrics_available=$(jq -r '.reward_runtime.metrics_available // false' <<< "$status_json")
    rr_last_error=$(jq -r '.reward_runtime.last_error // empty' <<< "$status_json")
    rr_distfs_total_checks=$(safe_int "$(jq -r '.reward_runtime.distfs_total_checks // 0' <<< "$status_json")")
    rr_distfs_failed_checks=$(safe_int "$(jq -r '.reward_runtime.distfs_failed_checks // 0' <<< "$status_json")")
    rr_settlement_apply_attempts=$(safe_int "$(jq -r '.reward_runtime.settlement_apply_attempts_total // 0' <<< "$status_json")")
    rr_settlement_apply_failures=$(safe_int "$(jq -r '.reward_runtime.settlement_apply_failures_total // 0' <<< "$status_json")")
    rr_minted_cumulative_count=$(safe_int "$(jq -r '.reward_runtime.cumulative_minted_record_count // 0' <<< "$status_json")")
    rr_invariant_ok=$(jq -r '.reward_runtime.invariant_ok // true' <<< "$status_json")
  fi

  local reward_mint_record_count balance_load_error
  reward_mint_record_count=0
  balance_load_error=""
  if [[ "$balances_ok" -eq 1 ]]; then
    reward_mint_record_count=$(safe_int "$(jq -r '.reward_mint_record_count // 0' <<< "$balances_json")")
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

  if [[ "$status_ok" -eq 1 ]]; then
    record_consensus_hash_sample \
      "committed_block_hash" \
      "$committed_height" \
      "$last_block_hash" \
      "$node_name" \
      "analysis_committed_height_block_hash"
    record_consensus_hash_sample \
      "execution_block_hash" \
      "$last_execution_height" \
      "$last_execution_block_hash" \
      "$node_name" \
      "analysis_execution_height_block_hash"
    record_consensus_hash_sample \
      "execution_state_root" \
      "$last_execution_height" \
      "$last_execution_state_root" \
      "$node_name" \
      "analysis_execution_height_state_root"
  fi

  append_topology_timeline_sample \
    "$topology" \
    "$node_name" \
    "$epoch_index" \
    "$observed_at_unix_ms" \
    "$committed_height" \
    "$network_committed_height" \
    "$lag" \
    "$invariant_ok" \
    "$last_block_hash" \
    "$last_execution_block_hash" \
    "$last_execution_state_root" \
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
    analysis_last_error_samples=$((analysis_last_error_samples + 1))
    printf '%s\t%s\n' "$node_name" "$last_error" >> "$analysis_runtime_errors_file"
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
    printf '%s\t%s\n' "$node_name" "$rr_last_error" >> "$analysis_runtime_errors_file"
  fi
  if [[ "$rr_invariant_ok" != "true" ]]; then
    analysis_invariant_all_ok=false
  fi
  local prev_distfs_total=${analysis_node_distfs_total_checks_max[$node_name]:-0}
  if (( rr_distfs_total_checks > prev_distfs_total )); then
    analysis_node_distfs_total_checks_max["$node_name"]=$rr_distfs_total_checks
  fi
  local prev_distfs_failed=${analysis_node_distfs_failed_checks_max[$node_name]:-0}
  if (( rr_distfs_failed_checks > prev_distfs_failed )); then
    analysis_node_distfs_failed_checks_max["$node_name"]=$rr_distfs_failed_checks
  fi
  local prev_settlement_attempts=${analysis_node_settlement_apply_attempts_max[$node_name]:-0}
  if (( rr_settlement_apply_attempts > prev_settlement_attempts )); then
    analysis_node_settlement_apply_attempts_max["$node_name"]=$rr_settlement_apply_attempts
  fi
  local prev_settlement_failures=${analysis_node_settlement_apply_failures_max[$node_name]:-0}
  if (( rr_settlement_apply_failures > prev_settlement_failures )); then
    analysis_node_settlement_apply_failures_max["$node_name"]=$rr_settlement_apply_failures
  fi
  local prev_minted=${analysis_node_reward_minted_count_max[$node_name]:-0}
  if (( rr_minted_cumulative_count > prev_minted )); then
    analysis_node_reward_minted_count_max["$node_name"]=$rr_minted_cumulative_count
  fi
  if (( effective_minted_record_count > 0 )); then
    analysis_minted_non_empty_samples=$((analysis_minted_non_empty_samples + 1))
  fi

  echo "$lag" >> "$analysis_lag_values_file"

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

  local prev_committed=${analysis_node_prev_committed[$node_name]:-}
  if [[ -n "$prev_committed" ]] && (( committed_height < prev_committed )); then
    analysis_node_monotonic_violations["$node_name"]=1
  fi
  analysis_node_prev_committed["$node_name"]=$committed_height
}

poll_topology_all_nodes() {
  local topology=$1
  local node_name
  for node_name in "${active_nodes[@]}"; do
    poll_topology_node_once "$topology" "$node_name"
  done
}

compute_topology_lag_p95() {
  local lag_count
  lag_count=$(wc -l < "$analysis_lag_values_file" | tr -d ' ')
  if [[ -z "$lag_count" ]]; then
    lag_count=0
  fi
  if (( lag_count <= 0 )); then
    analysis_lag_p95=0
    return 0
  fi
  local sorted_lag_file="$analysis_topology_dir/.lag_values.sorted.txt"
  sort -n "$analysis_lag_values_file" > "$sorted_lag_file"
  local p95_rank=$(( (95 * lag_count + 99) / 100 ))
  if (( p95_rank < 1 )); then
    p95_rank=1
  fi
  analysis_lag_p95=$(sed -n "${p95_rank}p" "$sorted_lag_file")
  analysis_lag_p95=$(safe_int "$analysis_lag_p95")
}

finalize_topology_metric_gate() {
  local chaos_event_count=${1:-0}
  compute_topology_lag_p95

  if (( ${#analysis_node_monotonic_violations[@]} > 0 )); then
    analysis_monotonic_ok=false
    analysis_monotonic_violation_nodes=$(printf '%s\n' "${!analysis_node_monotonic_violations[@]}" | sort | paste -sd ',' -)
  fi
  if (( ${#analysis_consensus_hash_mismatch_heights_map[@]} > 0 )); then
    analysis_consensus_hash_mismatch_heights=$(printf '%s\n' "${!analysis_consensus_hash_mismatch_heights_map[@]}" | sort -n | paste -sd ',' -)
  fi

  analysis_distfs_total_checks=0
  analysis_distfs_failed_checks=0
  analysis_settlement_apply_attempts=0
  analysis_settlement_apply_failures=0
  analysis_distfs_failure_ratio="0.000000"
  analysis_settlement_apply_failure_ratio="0.000000"
  local node_name
  for node_name in "${active_nodes[@]}"; do
    analysis_distfs_total_checks=$((analysis_distfs_total_checks + ${analysis_node_distfs_total_checks_max[$node_name]:-0}))
    analysis_distfs_failed_checks=$((analysis_distfs_failed_checks + ${analysis_node_distfs_failed_checks_max[$node_name]:-0}))
    analysis_settlement_apply_attempts=$((analysis_settlement_apply_attempts + ${analysis_node_settlement_apply_attempts_max[$node_name]:-0}))
    analysis_settlement_apply_failures=$((analysis_settlement_apply_failures + ${analysis_node_settlement_apply_failures_max[$node_name]:-0}))
  done
  if (( analysis_distfs_total_checks > 0 )); then
    analysis_distfs_failure_ratio=$(awk -v failed="$analysis_distfs_failed_checks" -v total="$analysis_distfs_total_checks" 'BEGIN { printf "%.6f", failed / total }')
  fi
  if (( analysis_settlement_apply_attempts > 0 )); then
    analysis_settlement_apply_failure_ratio=$(awk -v failed="$analysis_settlement_apply_failures" -v total="$analysis_settlement_apply_attempts" 'BEGIN { printf "%.6f", failed / total }')
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
    if (( chaos_event_count > 0 )); then
      gate_warnings+=("running_false_samples=${analysis_running_false_samples}")
    else
      gate_failures+=("running_false_samples=${analysis_running_false_samples}")
    fi
  fi
  if (( analysis_last_error_samples > 0 )); then
    gate_failures+=("last_error_samples=${analysis_last_error_samples}")
  fi
  if (( analysis_max_stall_secs_observed > analysis_effective_max_stall_secs )); then
    gate_failures+=("stall=${analysis_max_stall_secs_observed}s>max_${analysis_effective_max_stall_secs}s")
  fi
  if (( analysis_lag_p95 > max_lag_p95 )); then
    gate_failures+=("lag_p95=${analysis_lag_p95}>max_${max_lag_p95}")
  fi
  if (( analysis_distfs_total_checks > 0 )) && awk -v ratio="$analysis_distfs_failure_ratio" -v max="$max_distfs_failure_ratio" 'BEGIN { exit !(ratio > max) }'; then
    gate_failures+=("distfs_failure_ratio=${analysis_distfs_failure_ratio}>max_${max_distfs_failure_ratio}")
  fi
  if [[ "$analysis_monotonic_ok" != "true" ]]; then
    if (( chaos_event_count > 0 )); then
      gate_warnings+=("committed_height_not_monotonic nodes=${analysis_monotonic_violation_nodes}")
    else
      gate_failures+=("committed_height_not_monotonic nodes=${analysis_monotonic_violation_nodes}")
    fi
  fi
  if [[ "$analysis_invariant_all_ok" != "true" ]]; then
    gate_failures+=("reward_asset_invariant_violation")
  fi
  if [[ "$analysis_consensus_hash_consistent" != "true" ]]; then
    if [[ -z "$analysis_consensus_hash_mismatch_heights" ]]; then
      gate_failures+=("consensus_hash_divergence count=${analysis_consensus_hash_mismatch_count}")
    else
      gate_failures+=("consensus_hash_divergence count=${analysis_consensus_hash_mismatch_count} heights=${analysis_consensus_hash_mismatch_heights}")
    fi
  fi

  if (( analysis_reward_runtime_available_samples <= 0 )); then
    gate_data_warnings+=("reward_runtime_metrics_not_ready")
  elif (( analysis_distfs_total_checks <= 0 )); then
    gate_warnings+=("distfs_checks_zero")
  fi
  if (( analysis_consensus_hash_samples <= 0 )); then
    gate_data_warnings+=("consensus_hash_samples_missing")
  elif (( analysis_consensus_hash_missing_samples > 0 )); then
    gate_warnings+=("consensus_hash_missing_samples=${analysis_consensus_hash_missing_samples}")
  fi
  if (( analysis_settlement_apply_attempts <= 0 )); then
    gate_warnings+=("settlement_apply_attempts_zero")
  fi
  if (( analysis_balance_load_error_samples > 0 )); then
    gate_warnings+=("balances_load_error_samples=${analysis_balance_load_error_samples}")
  fi
  if (( analysis_peer_zero_samples > 0 )); then
    gate_warnings+=("known_peer_heads_zero_samples=${analysis_peer_zero_samples}")
  fi
  if (( analysis_http_failure_samples > 0 )); then
    if (( chaos_event_count > 0 )); then
      gate_warnings+=("http_failure_samples=${analysis_http_failure_samples}")
    else
      gate_failures+=("http_failure_samples=${analysis_http_failure_samples}")
    fi
  fi
  if (( analysis_minted_non_empty_samples <= 0 )); then
    gate_warnings+=("minted_records_empty")
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

run_topology() {
  local topology=$1
  local index=$2

  local started_at ended_at status notes
  started_at=$(date '+%Y-%m-%d %H:%M:%S %Z')
  status="ok"
  notes="-"

  reset_active_topology_state

  local topology_dir="$run_dir/$topology"
  run mkdir -p "$topology_dir/nodes"

  local -a node_labels=("sequencer" "storage" "observer")
  local -a node_roles=("sequencer" "storage" "observer")
  local -a node_stakes=(50 30 20)
  local -a node_runtime_ids=()
  local -a node_gossip_addrs=()
  local -a node_status_binds=()
  local -a node_replication_listens=()
  local -a validator_specs=()
  local case_base_port=$((base_port + index * 100))
  local topology_slug
  topology_slug=$(slugify "$topology")
  local world_id="p2p-${topology_slug}-${timestamp}"

  local i
  for i in "${!node_labels[@]}"; do
    local label=${node_labels[$i]}
    local runtime_id="soak-${topology_slug}-${timestamp}-${label}"
    local gossip_addr="$bind_host:$((case_base_port + i + 1))"
    local status_bind="$bind_host:$((case_base_port + i + 21))"
    local replication_listen="/ip4/${bind_host}/tcp/$((case_base_port + i + 41))"
    node_runtime_ids+=("$runtime_id")
    node_gossip_addrs+=("$gossip_addr")
    node_status_binds+=("$status_bind")
    node_replication_listens+=("$replication_listen")
    validator_specs+=("${runtime_id}:${node_stakes[$i]}")
  done

  if [[ "$dry_run" -eq 1 ]]; then
    for i in "${!node_labels[@]}"; do
      local label=${node_labels[$i]}
      local role=${node_roles[$i]}
      local runtime_id=${node_runtime_ids[$i]}
      local status_bind=${node_status_binds[$i]}
      local gossip_addr=${node_gossip_addrs[$i]}
      local -a cmd=(
        "$chain_bin"
        --node-id "$runtime_id"
        --world-id "$world_id"
        --status-bind "$status_bind"
        --node-role "$role"
        --node-tick-ms "$node_tick_ms"
        --pos-slot-duration-ms "$pos_slot_duration_ms"
        --pos-ticks-per-slot "$pos_ticks_per_slot"
        --pos-proposal-tick-phase "$pos_proposal_tick_phase"
        --pos-max-past-slot-lag "$pos_max_past_slot_lag"
        --reward-runtime-epoch-duration-secs "$reward_runtime_epoch_duration_secs"
        --reward-points-per-credit "$reward_points_per_credit"
        --node-gossip-bind "$gossip_addr"
        --replication-network-listen "${node_replication_listens[$i]}"
      )
      if [[ "$pos_adaptive_tick_scheduler_enabled" -eq 1 ]]; then
        cmd+=(--pos-adaptive-tick-scheduler)
      else
        cmd+=(--pos-no-adaptive-tick-scheduler)
      fi
      if [[ -n "$pos_slot_clock_genesis_unix_ms" ]]; then
        cmd+=(--pos-slot-clock-genesis-unix-ms "$pos_slot_clock_genesis_unix_ms")
      fi
      if [[ "$role" == "sequencer" ]]; then
        cmd+=(--node-auto-attest-all)
      else
        cmd+=(--node-no-auto-attest-all)
      fi
      local validator
      for validator in "${validator_specs[@]}"; do
        cmd+=(--node-validator "$validator")
      done
      local peer_idx
      for peer_idx in "${!node_labels[@]}"; do
        if (( peer_idx == i )); then
          continue
        fi
        cmd+=(--node-gossip-peer "${node_gossip_addrs[$peer_idx]}")
        cmd+=(--replication-network-peer "${node_replication_listens[$peer_idx]}")
      done

      local node_dir="$topology_dir/nodes/$label"
      local cmd_txt="$node_dir/command.txt"
      run mkdir -p "$node_dir"
      printf '%q ' "${cmd[@]}" > "$cmd_txt"
      printf '\n' >> "$cmd_txt"
      echo "+ dry-run command[$topology/$label]: ${cmd[*]}"
    done

    append_summary_row "$topology" "dry_run" "dry_run" "dry_run" "0" "$started_at" "$started_at" "commands_rendered_only"
    jq -n \
      --arg topology "$topology" \
      --arg started_at "$started_at" \
      '{
        topology: $topology,
        status: "dry_run",
        process_status: "dry_run",
        started_at: $started_at,
        ended_at: $started_at,
        notes: "commands_rendered_only",
        report_samples: 0,
        chaos_events: 0,
        chaos_plan_events: 0,
        chaos_continuous_events: 0,
        feedback_events: 0,
        feedback_events_success: 0,
        feedback_events_failed: 0,
        metric_gate: {
          status: "dry_run",
          notes: "commands_rendered_only"
        },
        metrics: {
          max_stall_secs_observed: 0,
          chaos_exempt_secs: 0,
          effective_max_stall_secs: 0,
          lag_p95: 0,
          distfs_failure_ratio: "0.000000",
          distfs_total_checks: 0,
          distfs_failed_checks: 0,
          settlement_apply_failure_ratio: "0.000000",
          settlement_apply_attempts: 0,
          settlement_apply_failures: 0,
          invariant_all_ok: true,
          status_samples_ok: 0,
          balances_samples_ok: 0,
          running_false_samples: 0,
          last_error_samples: 0,
          reward_runtime_available_samples: 0,
          minted_non_empty_samples: 0,
          committed_height_monotonic: true,
          committed_height_monotonic_violation_nodes: [],
          consensus_hash_consistent: true,
          consensus_hash_samples: 0,
          consensus_hash_missing_samples: 0,
          consensus_hash_mismatch_count: 0,
          consensus_hash_mismatch_heights: [],
          consensus_hash_mismatch_file: ""
        }
      }' >> "$topology_summary_ndjson"
    return 0
  fi

  for i in "${!node_labels[@]}"; do
    local label=${node_labels[$i]}
    local role=${node_roles[$i]}
    local runtime_id=${node_runtime_ids[$i]}
    local status_bind=${node_status_binds[$i]}
    local gossip_addr=${node_gossip_addrs[$i]}
    local status_url="http://${status_bind}/v1/chain/status"
    local balances_url="http://${status_bind}/v1/chain/balances"

    local -a cmd=(
      "$chain_bin"
      --node-id "$runtime_id"
      --world-id "$world_id"
      --status-bind "$status_bind"
      --node-role "$role"
      --node-tick-ms "$node_tick_ms"
      --pos-slot-duration-ms "$pos_slot_duration_ms"
      --pos-ticks-per-slot "$pos_ticks_per_slot"
      --pos-proposal-tick-phase "$pos_proposal_tick_phase"
      --pos-max-past-slot-lag "$pos_max_past_slot_lag"
      --reward-runtime-epoch-duration-secs "$reward_runtime_epoch_duration_secs"
      --reward-points-per-credit "$reward_points_per_credit"
      --node-gossip-bind "$gossip_addr"
      --replication-network-listen "${node_replication_listens[$i]}"
    )
    if [[ "$pos_adaptive_tick_scheduler_enabled" -eq 1 ]]; then
      cmd+=(--pos-adaptive-tick-scheduler)
    else
      cmd+=(--pos-no-adaptive-tick-scheduler)
    fi
    if [[ -n "$pos_slot_clock_genesis_unix_ms" ]]; then
      cmd+=(--pos-slot-clock-genesis-unix-ms "$pos_slot_clock_genesis_unix_ms")
    fi

    if [[ "$role" == "sequencer" ]]; then
      cmd+=(--node-auto-attest-all)
    else
      cmd+=(--node-no-auto-attest-all)
    fi

    local validator
    for validator in "${validator_specs[@]}"; do
      cmd+=(--node-validator "$validator")
    done

    local peer_idx
    for peer_idx in "${!node_labels[@]}"; do
      if (( peer_idx == i )); then
        continue
      fi
      cmd+=(--node-gossip-peer "${node_gossip_addrs[$peer_idx]}")
      cmd+=(--replication-network-peer "${node_replication_listens[$peer_idx]}")
    done

    launch_node "$topology_dir" "$label" "$status_url" "$balances_url" "$runtime_id" "${cmd[@]}"
  done

  local -a chaos_event_ids=()
  local -a chaos_event_at_secs=()
  local -a chaos_event_nodes=()
  local -a chaos_event_actions=()
  local -a chaos_event_down_secs=()
  local -a chaos_event_duration_secs=()
  local -a chaos_event_done=()
  local continuous_enabled=0
  local continuous_next_at_sec=0
  local continuous_generated=0
  local feedback_enabled=0
  local feedback_next_at_sec=0
  local feedback_generated=0
  local feedback_node_cursor=0

  if [[ -n "$chaos_plan_path" ]]; then
    local event_id at_sec node action down_secs event_duration_secs_raw
    while IFS=$'\t' read -r event_id at_sec node action down_secs event_duration_secs_raw; do
      [[ -z "$event_id" ]] && continue
      [[ "$at_sec" =~ ^[0-9]+$ ]] || at_sec=0
      [[ "$down_secs" =~ ^[0-9]+$ ]] || down_secs=0
      [[ "$event_duration_secs_raw" =~ ^[0-9]+$ ]] || event_duration_secs_raw=0

      if [[ -z "$node" ]]; then
        log_chaos_event "$topology" "$event_id" "skipped" "$action" "none" "missing_node"
        continue
      fi

      local node_resolved="$node"
      local label_idx
      for label_idx in "${!node_labels[@]}"; do
        if [[ "$node" == "${node_runtime_ids[$label_idx]}" ]]; then
          node_resolved=${node_labels[$label_idx]}
          break
        fi
      done

      case "$action" in
        restart|pause|disconnect) ;;
        "")
          action="restart"
          ;;
        *)
          log_chaos_event "$topology" "$event_id" "failed" "$action" "$node_resolved" "unsupported_action"
          status="chaos_plan_invalid"
          notes="invalid chaos action for event=$event_id"
          break
          ;;
      esac

      chaos_event_ids+=("$event_id")
      chaos_event_at_secs+=("$at_sec")
      chaos_event_nodes+=("$node_resolved")
      chaos_event_actions+=("$action")
      chaos_event_down_secs+=("$down_secs")
      chaos_event_duration_secs+=("$event_duration_secs_raw")
      chaos_event_done+=("0")
    done < <(
      jq -r --arg topology "$topology" '
        (.events // [] | to_entries[]) as $entry
        | ($entry.value) as $event
        | ($event.topology // "all") as $event_topology
        | select($event_topology == "all" or $event_topology == $topology)
        | [
            ($event.id // ("event-" + ($entry.key | tostring))),
            ($event.at_sec // 0),
            ($event.node // ""),
            ($event.action // "restart"),
            ($event.down_secs // 0),
            ($event.duration_secs // 0)
          ] | @tsv
      ' "$chaos_plan_path"
    )
  fi

  if [[ "$chaos_continuous_enabled" -eq 1 ]]; then
    continuous_enabled=1
    continuous_next_at_sec=$chaos_continuous_start_sec
    chaos_rng_seed $((chaos_continuous_seed + (index + 1) * 104729))
  fi
  if [[ "$feedback_events_enabled" -eq 1 ]]; then
    feedback_enabled=1
    feedback_next_at_sec=$feedback_events_start_sec
  fi

  if ! wait_for_topology_ready; then
    status="startup_failed"
    notes="failed to reach /healthz across all nodes"
  fi

  reset_topology_analysis "$topology" "$topology_dir"

  if [[ "$status" == "ok" ]]; then
    local started_epoch_sec
    started_epoch_sec=$(date +%s)
    local deadline=$((started_epoch_sec + duration_secs))
    while (( $(date +%s) < deadline )); do
      local now_sec elapsed_sec
      now_sec=$(date +%s)
      elapsed_sec=$((now_sec - started_epoch_sec))

      local event_idx
      for event_idx in "${!chaos_event_ids[@]}"; do
        if [[ "${chaos_event_done[$event_idx]}" == "1" ]]; then
          continue
        fi
        local event_at=${chaos_event_at_secs[$event_idx]}
        if (( elapsed_sec < event_at )); then
          continue
        fi

        local event_id=${chaos_event_ids[$event_idx]}
        local event_node=${chaos_event_nodes[$event_idx]}
        local event_action=${chaos_event_actions[$event_idx]}
        local event_down_secs=${chaos_event_down_secs[$event_idx]}
        local event_duration_secs=${chaos_event_duration_secs[$event_idx]}

        chaos_event_done[$event_idx]="1"
        if ! execute_chaos_event "$topology" "$event_id" "$event_action" "$event_node" "$event_at" "$event_down_secs" "$event_duration_secs"; then
          status="chaos_failed"
          notes="chaos_event=${event_id} failed"
          break 2
        fi

        local exempt_secs=0
        if [[ "$event_action" == "restart" ]]; then
          exempt_secs=$event_down_secs
        else
          exempt_secs=$event_duration_secs
        fi
        chaos_exempt_secs_by_topology["$topology"]=$(( ${chaos_exempt_secs_by_topology[$topology]:-0} + exempt_secs ))
        chaos_events_executed_by_topology["$topology"]=$(( ${chaos_events_executed_by_topology[$topology]:-0} + 1 ))
        chaos_plan_events_executed_by_topology["$topology"]=$(( ${chaos_plan_events_executed_by_topology[$topology]:-0} + 1 ))
      done

      if [[ "$continuous_enabled" -eq 1 ]]; then
        while (( elapsed_sec >= continuous_next_at_sec )); do
          if (( chaos_continuous_max_events > 0 && continuous_generated >= chaos_continuous_max_events )); then
            continuous_enabled=0
            break
          fi

          local rand node_idx action_idx
          local generated_event_id generated_node generated_action
          local generated_down_secs generated_duration_secs

          rand=$(chaos_rng_next)
          node_idx=$((rand % ${#active_nodes[@]}))
          generated_node=${active_nodes[$node_idx]}

          rand=$(chaos_rng_next)
          action_idx=$((rand % ${#chaos_continuous_actions[@]}))
          generated_action=${chaos_continuous_actions[$action_idx]}

          generated_down_secs=0
          generated_duration_secs=0
          if [[ "$generated_action" == "restart" ]]; then
            generated_down_secs=$chaos_continuous_restart_down_secs
          else
            generated_duration_secs=$chaos_continuous_pause_duration_secs
          fi

          generated_event_id="continuous-${topology}-${continuous_generated}"
          if ! execute_chaos_event "$topology" "$generated_event_id" "$generated_action" "$generated_node" "$continuous_next_at_sec" "$generated_down_secs" "$generated_duration_secs"; then
            status="chaos_failed"
            notes="chaos_event=${generated_event_id} failed"
            break 2
          fi

          local generated_exempt_secs=0
          if [[ "$generated_action" == "restart" ]]; then
            generated_exempt_secs=$generated_down_secs
          else
            generated_exempt_secs=$generated_duration_secs
          fi
          chaos_exempt_secs_by_topology["$topology"]=$(( ${chaos_exempt_secs_by_topology[$topology]:-0} + generated_exempt_secs ))
          chaos_events_executed_by_topology["$topology"]=$(( ${chaos_events_executed_by_topology[$topology]:-0} + 1 ))
          chaos_continuous_events_executed_by_topology["$topology"]=$(( ${chaos_continuous_events_executed_by_topology[$topology]:-0} + 1 ))

          continuous_generated=$((continuous_generated + 1))
          continuous_next_at_sec=$((continuous_next_at_sec + chaos_continuous_interval_secs))
        done
      fi

      if [[ "$feedback_enabled" -eq 1 ]]; then
        while (( elapsed_sec >= feedback_next_at_sec )); do
          if (( feedback_events_max_events > 0 && feedback_generated >= feedback_events_max_events )); then
            feedback_enabled=0
            break
          fi

          local feedback_node feedback_category feedback_event_id
          feedback_node=${active_nodes[$feedback_node_cursor]}
          if (( feedback_generated % 2 == 0 )); then
            feedback_category="bug"
          else
            feedback_category="suggestion"
          fi
          feedback_event_id="feedback-${topology}-${feedback_generated}"

          feedback_events_executed_by_topology["$topology"]=$(( ${feedback_events_executed_by_topology[$topology]:-0} + 1 ))
          if execute_feedback_submit_event "$topology" "$feedback_event_id" "$feedback_node" "$feedback_category" "$feedback_next_at_sec"; then
            feedback_events_success_by_topology["$topology"]=$(( ${feedback_events_success_by_topology[$topology]:-0} + 1 ))
          else
            feedback_events_failed_by_topology["$topology"]=$(( ${feedback_events_failed_by_topology[$topology]:-0} + 1 ))
          fi

          feedback_generated=$((feedback_generated + 1))
          feedback_node_cursor=$(( (feedback_node_cursor + 1) % ${#active_nodes[@]} ))
          feedback_next_at_sec=$((feedback_next_at_sec + feedback_events_interval_secs))
        done
      fi

      poll_topology_all_nodes "$topology"

      local idx
      for idx in "${!active_pids[@]}"; do
        if ! kill -0 "${active_pids[$idx]}" >/dev/null 2>&1; then
          status="process_exit"
          notes="node=${active_nodes[$idx]} exited during soak"
          break 2
        fi
      done

      sleep "$poll_interval_secs"
    done

    if [[ "$status" == "ok" ]]; then
      poll_topology_all_nodes "$topology"
    fi
  fi

  stop_active_processes

  local chaos_event_count=${chaos_events_executed_by_topology[$topology]:-0}
  local chaos_plan_event_count=${chaos_plan_events_executed_by_topology[$topology]:-0}
  local chaos_continuous_event_count=${chaos_continuous_events_executed_by_topology[$topology]:-0}
  local chaos_exempt_secs=${chaos_exempt_secs_by_topology[$topology]:-0}
  local feedback_event_count=${feedback_events_executed_by_topology[$topology]:-0}
  local feedback_event_success_count=${feedback_events_success_by_topology[$topology]:-0}
  local feedback_event_failed_count=${feedback_events_failed_by_topology[$topology]:-0}

  analysis_chaos_exempt_secs=$chaos_exempt_secs
  analysis_effective_max_stall_secs=$((max_stall_secs + analysis_chaos_exempt_secs))
  finalize_topology_metric_gate "$chaos_event_count"

  local process_status=$status
  local final_status=$status
  local -a notes_parts=()
  if [[ "$notes" != "-" ]]; then
    notes_parts+=("$notes")
  fi
  if (( chaos_event_count > 0 )); then
    notes_parts+=("chaos_events=${chaos_event_count}")
    notes_parts+=("chaos_plan_events=${chaos_plan_event_count}")
    notes_parts+=("chaos_continuous_events=${chaos_continuous_event_count}")
    notes_parts+=("chaos_exempt_secs=${chaos_exempt_secs}")
  fi
  if (( feedback_event_count > 0 )); then
    notes_parts+=("feedback_events=${feedback_event_count}")
    notes_parts+=("feedback_success=${feedback_event_success_count}")
    notes_parts+=("feedback_failed=${feedback_event_failed_count}")
  fi

  if [[ "$analysis_gate_status" == "fail" ]]; then
    notes_parts+=("metric_gate=$analysis_gate_notes")
    if [[ "$final_status" == "ok" ]]; then
      final_status="metric_gate_failed"
    fi
  elif [[ "$analysis_gate_status" == "insufficient_data" ]]; then
    notes_parts+=("metric_data=$analysis_gate_notes")
    if [[ "$profile" != "soak_smoke" ]] && [[ "$final_status" == "ok" ]]; then
      notes_parts+=("profile_${profile}_requires_metrics")
      final_status="metric_gate_failed"
    fi
  fi

  if (( ${#notes_parts[@]} == 0 )); then
    notes="-"
  else
    notes=$(join_by "; " "${notes_parts[@]}")
  fi

  ended_at=$(date '+%Y-%m-%d %H:%M:%S %Z')
  status=$final_status

  append_summary_row "$topology" "$status" "$process_status" "$analysis_gate_status" "$analysis_report_count" "$started_at" "$ended_at" "$notes"

  jq -n \
    --arg topology "$topology" \
    --arg status "$status" \
    --arg process_status "$process_status" \
    --arg started_at "$started_at" \
    --arg ended_at "$ended_at" \
    --arg notes "$notes" \
    --arg gate_status "$analysis_gate_status" \
    --arg gate_notes "$analysis_gate_notes" \
    --argjson report_samples "$analysis_report_count" \
    --argjson chaos_events "$chaos_event_count" \
    --argjson chaos_plan_events "$chaos_plan_event_count" \
    --argjson chaos_continuous_events "$chaos_continuous_event_count" \
    --argjson feedback_events "$feedback_event_count" \
    --argjson feedback_events_success "$feedback_event_success_count" \
    --argjson feedback_events_failed "$feedback_event_failed_count" \
    --argjson max_stall_secs_observed "$analysis_max_stall_secs_observed" \
    --argjson chaos_exempt_secs "$analysis_chaos_exempt_secs" \
    --argjson effective_max_stall_secs "$analysis_effective_max_stall_secs" \
    --argjson lag_p95 "$analysis_lag_p95" \
    --argjson distfs_failure_ratio "$analysis_distfs_failure_ratio" \
    --argjson distfs_total_checks "$analysis_distfs_total_checks" \
    --argjson distfs_failed_checks "$analysis_distfs_failed_checks" \
    --argjson settlement_apply_failure_ratio "$analysis_settlement_apply_failure_ratio" \
    --argjson settlement_apply_attempts "$analysis_settlement_apply_attempts" \
    --argjson settlement_apply_failures "$analysis_settlement_apply_failures" \
    --argjson invariant_all_ok "$analysis_invariant_all_ok" \
    --argjson status_samples_ok "$analysis_status_samples_ok" \
    --argjson balances_samples_ok "$analysis_balances_samples_ok" \
    --argjson running_false_samples "$analysis_running_false_samples" \
    --argjson last_error_samples "$analysis_last_error_samples" \
    --argjson reward_runtime_available_samples "$analysis_reward_runtime_available_samples" \
    --argjson minted_non_empty_samples "$analysis_minted_non_empty_samples" \
    --argjson monotonic_ok "$analysis_monotonic_ok" \
    --arg monotonic_violation_nodes "$analysis_monotonic_violation_nodes" \
    --argjson consensus_hash_consistent "$analysis_consensus_hash_consistent" \
    --argjson consensus_hash_samples "$analysis_consensus_hash_samples" \
    --argjson consensus_hash_missing_samples "$analysis_consensus_hash_missing_samples" \
    --argjson consensus_hash_mismatch_count "$analysis_consensus_hash_mismatch_count" \
    --arg consensus_hash_mismatch_heights "$analysis_consensus_hash_mismatch_heights" \
    --arg consensus_hash_mismatch_file "$analysis_consensus_hash_mismatch_file" \
    '{
      topology: $topology,
      status: $status,
      process_status: $process_status,
      started_at: $started_at,
      ended_at: $ended_at,
      notes: $notes,
      report_samples: $report_samples,
      chaos_events: $chaos_events,
      chaos_plan_events: $chaos_plan_events,
      chaos_continuous_events: $chaos_continuous_events,
      feedback_events: $feedback_events,
      feedback_events_success: $feedback_events_success,
      feedback_events_failed: $feedback_events_failed,
      metric_gate: {
        status: $gate_status,
        notes: $gate_notes
      },
      metrics: {
        max_stall_secs_observed: $max_stall_secs_observed,
        chaos_exempt_secs: $chaos_exempt_secs,
        effective_max_stall_secs: $effective_max_stall_secs,
        lag_p95: $lag_p95,
        distfs_failure_ratio: $distfs_failure_ratio,
        distfs_total_checks: $distfs_total_checks,
        distfs_failed_checks: $distfs_failed_checks,
        settlement_apply_failure_ratio: $settlement_apply_failure_ratio,
        settlement_apply_attempts: $settlement_apply_attempts,
        settlement_apply_failures: $settlement_apply_failures,
        invariant_all_ok: $invariant_all_ok,
        status_samples_ok: $status_samples_ok,
        balances_samples_ok: $balances_samples_ok,
        running_false_samples: $running_false_samples,
        last_error_samples: $last_error_samples,
        reward_runtime_available_samples: $reward_runtime_available_samples,
        minted_non_empty_samples: $minted_non_empty_samples,
        committed_height_monotonic: $monotonic_ok,
        committed_height_monotonic_violation_nodes: (
          if $monotonic_violation_nodes == "" then []
          else ($monotonic_violation_nodes | split(","))
          end
        ),
        consensus_hash_consistent: $consensus_hash_consistent,
        consensus_hash_samples: $consensus_hash_samples,
        consensus_hash_missing_samples: $consensus_hash_missing_samples,
        consensus_hash_mismatch_count: $consensus_hash_mismatch_count,
        consensus_hash_mismatch_heights: (
          if $consensus_hash_mismatch_heights == "" then []
          else ($consensus_hash_mismatch_heights | split(","))
          end
        ),
        consensus_hash_mismatch_file: $consensus_hash_mismatch_file
      }
    }' >> "$topology_summary_ndjson"

  if [[ "$status" != "ok" ]]; then
    echo "topology run failed: $topology ($notes)" >&2
    return 1
  fi
  return 0
}

write_summary_json() {
  local overall_status_code=$1
  local generated_at
  generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

  jq -s \
    --arg generated_at "$generated_at" \
    --arg run_dir "$run_dir" \
    --arg profile "$profile" \
    --arg scenario "$scenario" \
    --arg chaos_plan_path "${chaos_plan_path:-}" \
    --arg chaos_continuous_actions_csv "$chaos_continuous_actions_csv" \
    --argjson llm_enabled "$llm_enabled" \
    --argjson duration_secs "$duration_secs" \
    --argjson max_stall_secs "$max_stall_secs" \
    --argjson max_lag_p95 "$max_lag_p95" \
    --argjson max_distfs_failure_ratio "$max_distfs_failure_ratio" \
    --argjson chaos_continuous_enabled "$chaos_continuous_enabled" \
    --argjson chaos_continuous_interval_secs "$chaos_continuous_interval_secs" \
    --argjson chaos_continuous_start_sec "$chaos_continuous_start_sec" \
    --argjson chaos_continuous_max_events "$chaos_continuous_max_events" \
    --argjson chaos_continuous_seed "$chaos_continuous_seed" \
    --argjson chaos_continuous_restart_down_secs "$chaos_continuous_restart_down_secs" \
    --argjson chaos_continuous_pause_duration_secs "$chaos_continuous_pause_duration_secs" \
    --argjson dry_run "$dry_run" \
    --arg timeline_csv "$timeline_csv" \
    --arg summary_md "$summary_md" \
    --arg chaos_events_log "$chaos_events_log" \
    --arg feedback_events_log "$feedback_events_log" \
    --arg run_config_json "$run_config_json" \
    --argjson feedback_events_enabled "$feedback_events_enabled" \
    --argjson feedback_events_interval_secs "$feedback_events_interval_secs" \
    --argjson feedback_events_start_sec "$feedback_events_start_sec" \
    --argjson feedback_events_max_events "$feedback_events_max_events" \
    --argjson overall_status_code "$overall_status_code" \
    '
      . as $topologies |
      {
        generated_at_utc: $generated_at,
        run_dir: $run_dir,
        profile: $profile,
        scenario_compat: $scenario,
        llm_enabled_compat: ($llm_enabled == 1),
        chaos_plan: (if $chaos_plan_path == "" then null else $chaos_plan_path end),
        chaos_continuous: {
          enabled: ($chaos_continuous_enabled == 1),
          interval_secs: $chaos_continuous_interval_secs,
          start_sec: $chaos_continuous_start_sec,
          max_events: $chaos_continuous_max_events,
          actions_csv: $chaos_continuous_actions_csv,
          seed: (if $chaos_continuous_enabled == 1 then $chaos_continuous_seed else null end),
          restart_down_secs: $chaos_continuous_restart_down_secs,
          pause_duration_secs: $chaos_continuous_pause_duration_secs
        },
        feedback_events: {
          enabled: ($feedback_events_enabled == 1),
          interval_secs: $feedback_events_interval_secs,
          start_sec: $feedback_events_start_sec,
          max_events: $feedback_events_max_events
        },
        dry_run: ($dry_run == 1),
        duration_secs_per_topology: $duration_secs,
        thresholds: {
          max_stall_secs: $max_stall_secs,
          max_lag_p95: $max_lag_p95,
          max_distfs_failure_ratio: $max_distfs_failure_ratio
        },
        artifacts: {
          run_config_json: $run_config_json,
          timeline_csv: $timeline_csv,
          summary_md: $summary_md,
          chaos_events_log: $chaos_events_log,
          feedback_events_log: $feedback_events_log
        },
        totals: {
          topology_count: ($topologies | length),
          topology_ok_count: ($topologies | map(select(.status == "ok" or .status == "dry_run")) | length),
          topology_failed_count: ($topologies | map(select(.status != "ok" and .status != "dry_run")) | length),
          report_samples_total: ($topologies | map(.report_samples) | add // 0),
          chaos_plan_events_total: ($topologies | map(.chaos_plan_events) | add // 0),
          chaos_continuous_events_total: ($topologies | map(.chaos_continuous_events) | add // 0),
          chaos_events_total: ($topologies | map(.chaos_events) | add // 0),
          feedback_events_total: ($topologies | map(.feedback_events) | add // 0),
          feedback_events_success_total: ($topologies | map(.feedback_events_success) | add // 0),
          feedback_events_failed_total: ($topologies | map(.feedback_events_failed) | add // 0)
        },
        gate_failures: (
          $topologies
          | map(select(.metric_gate.status == "fail") | {
              topology,
              reason: .metric_gate.notes
            })
        ),
        topologies: $topologies,
        overall_status: (
          if $dry_run == 1 then "dry_run"
          elif $overall_status_code == 0 then "ok"
          else "failed"
          end
        )
      }
    ' "$topology_summary_ndjson" > "$summary_json"
}

append_summary_metrics_section() {
  local topology_count=0
  local topology_ok_count=0
  local topology_failed_count=0
  local report_samples_total=0
  local chaos_plan_events_total=0
  local chaos_continuous_events_total=0
  local chaos_events_total=0
  local feedback_events_total=0
  local feedback_events_success_total=0
  local feedback_events_failed_total=0

  if [[ -f "$summary_json" ]]; then
    topology_count=$(jq -r '.totals.topology_count // 0' "$summary_json")
    topology_ok_count=$(jq -r '.totals.topology_ok_count // 0' "$summary_json")
    topology_failed_count=$(jq -r '.totals.topology_failed_count // 0' "$summary_json")
    report_samples_total=$(jq -r '.totals.report_samples_total // 0' "$summary_json")
    chaos_plan_events_total=$(jq -r '.totals.chaos_plan_events_total // 0' "$summary_json")
    chaos_continuous_events_total=$(jq -r '.totals.chaos_continuous_events_total // 0' "$summary_json")
    chaos_events_total=$(jq -r '.totals.chaos_events_total // 0' "$summary_json")
    feedback_events_total=$(jq -r '.totals.feedback_events_total // 0' "$summary_json")
    feedback_events_success_total=$(jq -r '.totals.feedback_events_success_total // 0' "$summary_json")
    feedback_events_failed_total=$(jq -r '.totals.feedback_events_failed_total // 0' "$summary_json")
  fi

  {
    echo
    echo "## Metrics Artifacts"
    echo
    echo "- timeline_csv: \`$timeline_csv\`"
    echo "- summary_json: \`$summary_json\`"
    echo "- chaos_events_log: \`$chaos_events_log\`"
    echo "- feedback_events_log: \`$feedback_events_log\`"
    echo "- topology_count: \`$topology_count\` (ok=\`$topology_ok_count\`, failed=\`$topology_failed_count\`)"
    echo "- report_samples_total: \`$report_samples_total\`"
    echo "- chaos_plan_events_total: \`$chaos_plan_events_total\`"
    echo "- chaos_continuous_events_total: \`$chaos_continuous_events_total\`"
    echo "- chaos_events_total: \`$chaos_events_total\`"
    echo "- feedback_events_total: \`$feedback_events_total\` (success=\`$feedback_events_success_total\`, failed=\`$feedback_events_failed_total\`)"
    echo
    echo "## Gate Metrics"
    echo
    echo "| topology | gate | reports | chaos_plan | chaos_continuous | chaos_events | feedback_events | feedback_success | feedback_failed | chaos_exempt_s | max_stall_s | max_stall_s_effective | lag_p95 | distfs_ratio | settlement_apply_ratio | invariant_all_ok | reward_runtime_samples | minted_samples | consensus_hash_consistent | consensus_hash_mismatch_count | consensus_hash_missing_samples |"
    echo "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    if [[ -f "$summary_json" ]]; then
      while IFS=$'\t' read -r topology gate reports chaos_plan chaos_continuous chaos_events feedback_events feedback_success feedback_failed chaos_exempt stall stall_effective lag ratio settlement_ratio invariant rr_samples minted consensus_consistent consensus_mismatch consensus_missing; do
        echo "| $topology | $gate | $reports | $chaos_plan | $chaos_continuous | $chaos_events | $feedback_events | $feedback_success | $feedback_failed | $chaos_exempt | $stall | $stall_effective | $lag | $ratio | $settlement_ratio | $invariant | $rr_samples | $minted | $consensus_consistent | $consensus_mismatch | $consensus_missing |"
      done < <(jq -r '.topologies[] | [ .topology, .metric_gate.status, .report_samples, .chaos_plan_events, .chaos_continuous_events, .chaos_events, .feedback_events, .feedback_events_success, .feedback_events_failed, .metrics.chaos_exempt_secs, .metrics.max_stall_secs_observed, .metrics.effective_max_stall_secs, .metrics.lag_p95, .metrics.distfs_failure_ratio, .metrics.settlement_apply_failure_ratio, .metrics.invariant_all_ok, .metrics.reward_runtime_available_samples, .metrics.minted_non_empty_samples, .metrics.consensus_hash_consistent, .metrics.consensus_hash_mismatch_count, .metrics.consensus_hash_missing_samples ] | @tsv' "$summary_json")
    fi
  } >> "$summary_md"
}

write_failures_md() {
  local overall_status_code=$1
  if (( overall_status_code == 0 )) || [[ "$dry_run" -eq 1 ]]; then
    rm -f "$failures_md"
    return 0
  fi

  {
    echo "# P2P Longrun Soak Failures"
    echo
    echo "- run_dir: \`$run_dir\`"
    echo "- profile: \`$profile\`"
    echo "- scenario_compat: \`$scenario\`"
    echo
    echo "## Failed Topologies"
    if [[ -f "$summary_json" ]]; then
      jq -r '.topologies[] | select(.status != "ok") | "- topology=\(.topology) status=\(.status) process=\(.process_status) gate=\(.metric_gate.status) reports=\(.report_samples) notes=\(.notes)"' "$summary_json"
    fi
  } > "$failures_md"
}

overall_status=0
for idx in "${!topologies[@]}"; do
  topology="${topologies[$idx]}"
  echo "== topology: $topology =="
  if ! run_topology "$topology" "$idx"; then
    overall_status=1
    break
  fi
done

write_summary_json "$overall_status"
append_summary_metrics_section
write_failures_md "$overall_status"

echo "soak run completed:"
echo "  run_dir: $run_dir"
echo "  summary: $summary_md"
echo "  summary_json: $summary_json"
echo "  timeline_csv: $timeline_csv"
echo "  chaos_events_log: $chaos_events_log"
echo "  feedback_events_log: $feedback_events_log"
if [[ -f "$failures_md" ]]; then
  echo "  failures: $failures_md"
fi

if [[ "$dry_run" -eq 1 ]]; then
  exit 0
fi

exit "$overall_status"
