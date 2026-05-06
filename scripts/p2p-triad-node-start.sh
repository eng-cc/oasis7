#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="${APP_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_FILE="${ENV_FILE:-$APP_ROOT/config/node.env}"
[[ -f "$ENV_FILE" ]] || { echo "missing env file: $ENV_FILE" >&2; exit 1; }

source "$ENV_FILE"

ensure_integer() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^-?[0-9]+$ ]]; then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

RELEASE_LINK="${RELEASE_LINK:-$APP_ROOT/current}"
BIN="${BIN:-$RELEASE_LINK/bin/oasis7_chain_runtime}"
[[ -x "$BIN" ]] || { echo "missing runtime binary: $BIN" >&2; exit 1; }

mkdir -p \
  "$APP_ROOT/logs" \
  "$APP_ROOT/data" \
  "$(dirname "$CONFIG_PATH")" \
  "$EXECUTION_WORLD_DIR" \
  "$EXECUTION_RECORDS_DIR" \
  "$STORAGE_ROOT"

IFS="," read -r -a validators <<< "${NODE_VALIDATORS_CSV:-}"
IFS="," read -r -a validator_signers <<< "${NODE_VALIDATOR_SIGNERS_CSV:-}"
IFS="," read -r -a peers <<< "${NODE_GOSSIP_PEERS_CSV:-}"
IFS="," read -r -a replication_listens <<< "${REPLICATION_NETWORK_LISTEN_ADDRS_CSV:-}"
IFS="," read -r -a replication_peers <<< "${REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV:-}"
IFS="," read -r -a replication_remote_writers <<< "${REPLICATION_REMOTE_WRITERS_CSV:-}"

cmd=(
  "$BIN"
  --node-id "$NODE_ID"
  --world-id "$WORLD_ID"
  --storage-profile "$STORAGE_PROFILE"
  --status-bind "$STATUS_BIND"
  --node-role "$NODE_ROLE"
  --node-tick-ms "$NODE_TICK_MS"
  --pos-slot-duration-ms "$POS_SLOT_DURATION_MS"
  --pos-ticks-per-slot "$POS_TICKS_PER_SLOT"
  --pos-proposal-tick-phase "$POS_PROPOSAL_TICK_PHASE"
  --pos-max-past-slot-lag "$POS_MAX_PAST_SLOT_LAG"
  --config "$CONFIG_PATH"
  --execution-world-dir "$EXECUTION_WORLD_DIR"
  --execution-records-dir "$EXECUTION_RECORDS_DIR"
  --storage-root "$STORAGE_ROOT"
  --reward-runtime-epoch-duration-secs "$REWARD_RUNTIME_EPOCH_DURATION_SECS"
  --reward-points-per-credit "$REWARD_POINTS_PER_CREDIT"
  --node-gossip-bind "$NODE_GOSSIP_BIND"
)

if [[ "${POS_ADAPTIVE_TICK_SCHEDULER:-0}" == "1" ]]; then
  cmd+=(--pos-adaptive-tick-scheduler)
else
  cmd+=(--pos-no-adaptive-tick-scheduler)
fi

if [[ "${REWARD_RUNTIME_ENABLE:-1}" == "1" ]]; then
  cmd+=(--reward-runtime-enable)
else
  cmd+=(--reward-runtime-disable)
fi

if [[ "${REWARD_RUNTIME_AUTO_REDEEM:-0}" == "1" ]]; then
  cmd+=(--reward-runtime-auto-redeem)
else
  cmd+=(--reward-runtime-no-auto-redeem)
fi

if [[ -n "${POS_SLOT_CLOCK_GENESIS_UNIX_MS:-}" ]]; then
  ensure_integer "POS_SLOT_CLOCK_GENESIS_UNIX_MS" "$POS_SLOT_CLOCK_GENESIS_UNIX_MS"
  cmd+=(--pos-slot-clock-genesis-unix-ms "$POS_SLOT_CLOCK_GENESIS_UNIX_MS")
fi

if [[ -n "${NODE_AUTO_ATTEST_FLAG:-}" ]]; then
  cmd+=("$NODE_AUTO_ATTEST_FLAG")
fi

for validator in "${validators[@]}"; do
  [[ -n "$validator" ]] && cmd+=(--node-validator "$validator")
done

for signer in "${validator_signers[@]}"; do
  [[ -n "$signer" ]] && cmd+=(--node-validator-signer-public-key "$signer")
done

for peer in "${peers[@]}"; do
  [[ -n "$peer" ]] && cmd+=(--node-gossip-peer "$peer")
done

for listen in "${replication_listens[@]}"; do
  [[ -n "$listen" ]] && cmd+=(--replication-network-listen "$listen")
done

for peer in "${replication_peers[@]}"; do
  [[ -n "$peer" ]] && cmd+=(--replication-network-peer "$peer")
done

for writer in "${replication_remote_writers[@]}"; do
  [[ -n "$writer" ]] && cmd+=(--replication-remote-writer-public-key "$writer")
done

traffic_profile="${TRAFFIC_PROFILE:-}"
if [[ -n "$traffic_profile" ]]; then
  cmd+=(--traffic-profile "$traffic_profile")
fi

traffic_monitor_enable="${TRAFFIC_MONITOR_ENABLE:-0}"
traffic_monitor_interval_secs="${TRAFFIC_MONITOR_INTERVAL_SECS:-60}"
traffic_monitor_window_minutes="${TRAFFIC_MONITOR_WINDOW_MINUTES:-10}"
traffic_monitor_history_retention_minutes="${TRAFFIC_MONITOR_HISTORY_RETENTION_MINUTES:-}"
traffic_monitor_top_n="${TRAFFIC_MONITOR_TOP_N:-5}"
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
traffic_monitor_script="${TRAFFIC_MONITOR_SCRIPT_PATH:-$script_dir/oasis7-node-traffic-monitor.sh}"
traffic_monitor_output_dir="${TRAFFIC_MONITOR_OUTPUT_DIR:-$APP_ROOT/output/traffic-monitor}"
traffic_monitor_status_url="${TRAFFIC_MONITOR_STATUS_URL:-http://$STATUS_BIND/v1/chain/status}"

runtime_log="$APP_ROOT/logs/chain-runtime.log"
startup_log="$APP_ROOT/logs/startup.log"
last_command_file="$APP_ROOT/logs/last-command.sh"
monitor_command_file="$APP_ROOT/logs/last-traffic-monitor-command.sh"
monitor_supervisor_log="$APP_ROOT/logs/traffic-monitor-supervisor.log"

printf '%s\n' "$(date -Is) starting $NODE_ROLE node $NODE_ID" >> "$startup_log"
printf '%q ' "${cmd[@]}" > "$last_command_file"
printf '\n' >> "$last_command_file"

monitor_cmd=()
if [[ "$traffic_monitor_enable" == "1" ]]; then
  [[ -x "$traffic_monitor_script" ]] || {
    echo "traffic monitor enabled but missing executable: $traffic_monitor_script" >&2
    exit 1
  }
  mkdir -p "$traffic_monitor_output_dir"
  monitor_cmd=(
    "$traffic_monitor_script"
    --loop
    --status-url "$traffic_monitor_status_url"
    --node-label "$NODE_ID"
    --out-dir "$traffic_monitor_output_dir"
    --interval-secs "$traffic_monitor_interval_secs"
    --window-minutes "$traffic_monitor_window_minutes"
    --top-n "$traffic_monitor_top_n"
  )
  if [[ -n "$traffic_monitor_history_retention_minutes" ]]; then
    monitor_cmd+=(
      --history-retention-minutes
      "$traffic_monitor_history_retention_minutes"
    )
  fi
  printf '%q ' "${monitor_cmd[@]}" > "$monitor_command_file"
  printf '\n' >> "$monitor_command_file"
fi

if [[ "${OASIS7_NODE_START_DRY_RUN:-0}" == "1" ]]; then
  printf 'runtime command:\n'
  printf '  %q' "${cmd[@]}"
  printf '\n'
  if (( ${#monitor_cmd[@]} > 0 )); then
    printf 'traffic monitor command:\n'
    printf '  %q' "${monitor_cmd[@]}"
    printf '\n'
  fi
  exit 0
fi

runtime_pid=""
monitor_pid=""

cleanup() {
  trap - EXIT TERM INT
  if [[ -n "$monitor_pid" ]]; then
    kill "$monitor_pid" 2>/dev/null || true
    wait "$monitor_pid" 2>/dev/null || true
  fi
  if [[ -n "$runtime_pid" ]]; then
    kill "$runtime_pid" 2>/dev/null || true
    wait "$runtime_pid" 2>/dev/null || true
  fi
}

trap cleanup EXIT TERM INT

"${cmd[@]}" >> "$runtime_log" 2>&1 &
runtime_pid=$!

if (( ${#monitor_cmd[@]} > 0 )); then
  printf '%s\n' "$(date -Is) starting traffic monitor for $NODE_ID" >> "$startup_log"
  "${monitor_cmd[@]}" >> "$monitor_supervisor_log" 2>&1 &
  monitor_pid=$!
fi

set +e
wait "$runtime_pid"
runtime_status=$?
set -e

if [[ -n "$monitor_pid" ]]; then
  kill "$monitor_pid" 2>/dev/null || true
  wait "$monitor_pid" 2>/dev/null || true
fi

exit "$runtime_status"
