#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/oasis7-node-traffic-monitor.sh [options]

Persist local `/v1/chain/status.traffic` samples for one node and derive a recent
window summary from cumulative counters.

Options:
  --status-url <url>               status endpoint to sample
                                   (default: http://127.0.0.1:5633/v1/chain/status)
  --node-label <label>             label written into summary/history
                                   (default: local_node)
  --out-dir <path>                 output root (default: .tmp/oasis7_node_traffic_monitor)
  --history-path <path>            override persistent history file
  --summary-json <path>            override latest summary json path
  --summary-md <path>              override latest summary markdown path
  --interval-secs <n>              loop interval for --loop mode (default: 60)
  --window-minutes <n>             recent summary window in minutes (default: 10)
  --history-retention-minutes <n>  keep only recent history for summary+buffer
                                   (default: max(window+30, 120))
  --top-n <n>                      top kind/topic/protocol rows (default: 5)
  --loop                           run forever instead of sampling once
  -h, --help                       show help

Artifacts:
  <out-dir>/history.ndjson
  <out-dir>/latest_summary.json
  <out-dir>/latest_summary.md

Notes:
  - History is pruned to a bounded retention window before each summary, so
    long-lived `--loop` usage does not grow `history.ndjson` without bound.
  - The summary logic compares only records whose
    `traffic.*.observed_since_unix_ms` matches the latest successful sample, so
    node restarts shrink the covered window instead of producing bogus deltas.
  - `latest_summary.json` now keeps full delta detail maps (`by_kind`,
    `by_topic`, `by_protocol`) in addition to the top-N markdown summary.
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

status_url="http://127.0.0.1:5633/v1/chain/status"
node_label="local_node"
out_dir=".tmp/oasis7_node_traffic_monitor"
history_path=""
summary_json_path=""
summary_md_path=""
interval_secs=60
window_minutes=10
history_retention_minutes=""
top_n=5
loop_mode=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status-url)
      status_url=${2:-}
      shift 2
      ;;
    --node-label)
      node_label=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --history-path)
      history_path=${2:-}
      shift 2
      ;;
    --summary-json)
      summary_json_path=${2:-}
      shift 2
      ;;
    --summary-md)
      summary_md_path=${2:-}
      shift 2
      ;;
    --interval-secs)
      interval_secs=${2:-}
      shift 2
      ;;
    --window-minutes)
      window_minutes=${2:-}
      shift 2
      ;;
    --history-retention-minutes)
      history_retention_minutes=${2:-}
      shift 2
      ;;
    --top-n)
      top_n=${2:-}
      shift 2
      ;;
    --loop)
      loop_mode=1
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

ensure_positive_int --interval-secs "$interval_secs"
ensure_positive_int --window-minutes "$window_minutes"
if [[ -n "$history_retention_minutes" ]]; then
  ensure_positive_int --history-retention-minutes "$history_retention_minutes"
else
  history_retention_minutes=$(( window_minutes + 30 ))
  if (( history_retention_minutes < 120 )); then
    history_retention_minutes=120
  fi
fi
ensure_positive_int --top-n "$top_n"

if [[ -z "$history_path" ]]; then
  history_path="$out_dir/history.ndjson"
fi
if [[ -z "$summary_json_path" ]]; then
  summary_json_path="$out_dir/latest_summary.json"
fi
if [[ -z "$summary_md_path" ]]; then
  summary_md_path="$out_dir/latest_summary.md"
fi

mkdir -p "$out_dir"
mkdir -p "$(dirname "$history_path")" "$(dirname "$summary_json_path")" "$(dirname "$summary_md_path")"
touch "$history_path"

summary_script="$repo_root/scripts/traffic-monitor-summary.py"
[[ -f "$summary_script" ]] || {
  echo "missing summary helper: $summary_script" >&2
  exit 1
}

sample_once() {
  local sample_tmp
  sample_tmp=$(mktemp)
  local captured_at
  local captured_at_unix_ms
  captured_at=$(date -Iseconds)
  captured_at_unix_ms=$(( $(date +%s) * 1000 ))

  if ! curl -fsS "$status_url" >"$sample_tmp" 2>"$sample_tmp.stderr"; then
    printf '{"ok":false,"fetch_error":"curl_failed"}\n' >"$sample_tmp"
  fi

  jq -c -n \
    --arg captured_at "$captured_at" \
    --arg captured_at_unix_ms "$captured_at_unix_ms" \
    --arg node_label "$node_label" \
    --arg status_url "$status_url" \
    --slurpfile status "$sample_tmp" \
    '{
      captured_at: $captured_at,
      captured_at_unix_ms: ($captured_at_unix_ms | tonumber),
      node_label: $node_label,
      status_url: $status_url,
      status_fetch_ok: ($status[0].ok // false),
      fetch_error: ($status[0].fetch_error // null),
      node_id: ($status[0].node_id // null),
      world_id: ($status[0].world_id // null),
      role: ($status[0].role // null),
      running: ($status[0].running // null),
      status_observed_at_unix_ms: ($status[0].observed_at_unix_ms // null),
      last_error: ($status[0].last_error // null),
      consensus: {
        committed_height: ($status[0].consensus.committed_height // null),
        network_committed_height: ($status[0].consensus.network_committed_height // null),
        known_peer_heads: ($status[0].consensus.known_peer_heads // null)
      },
      storage_recent_errors_count: ($status[0].storage.recent_errors_count // null),
      reward_runtime_recent_errors_count: ($status[0].reward_runtime.recent_errors_count // null),
      traffic: {
        udp_gossip: ($status[0].traffic.udp_gossip // null),
        libp2p_replication: ($status[0].traffic.libp2p_replication // null)
      }
    }' >> "$history_path"

  rm -f "$sample_tmp" "$sample_tmp.stderr"
}

write_summary() {
  python3 "$summary_script" \
    --layout single-node \
    --history-path "$history_path" \
    --summary-json "$summary_json_path" \
    --summary-md "$summary_md_path" \
    --window-minutes "$window_minutes" \
    --history-retention-minutes "$history_retention_minutes" \
    --top-n "$top_n"
}

while true; do
  sample_once
  write_summary
  if (( loop_mode == 0 )); then
    break
  fi
  sleep "$interval_secs"
done
