#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-real-env-traffic-monitor.sh [options]

Sample `/v1/chain/status.traffic` from the local observer + ECS sequencer/storage
triad, persist samples into a reusable history file, and derive recent-window
traffic summaries from cumulative counters.

Options:
  --samples <n>                    number of samples to collect this run (default: 3)
  --interval-secs <n>              sleep between samples (default: 20)
  --window-minutes <n>             recent summary window in minutes (default: 10)
  --top-n <n>                      top protocol/topic/kind entries per node (default: 5)
  --ssh-timeout-secs <n>           SSH connect timeout in seconds (default: 8)
  --out-dir <path>                 output root (default: .tmp/p2p_real_env_traffic_monitor)
  --history-path <path>            override persistent history file path
  --summary-only                   skip sampling, recompute summary from history only

  --observer-status-url <url>      local observer status endpoint
                                   (default: http://127.0.0.1:5633/v1/chain/status)

  --sequencer-target <user@host>   remote sequencer SSH target
                                   (default: root@39.104.204.172)
  --sequencer-status-url <url>     remote sequencer status endpoint
                                   (default: http://127.0.0.1:5631/v1/chain/status)

  --storage-target <user@host>     remote storage SSH target
                                   (default: root@39.104.205.67)
  --storage-status-url <url>       remote storage status endpoint
                                   (default: http://127.0.0.1:5632/v1/chain/status)

Environment:
  P2PARCH6_SEQ_SSH_PASSWORD        optional sequencer SSH password for sshpass
  P2PARCH6_STORAGE_SSH_PASSWORD    optional storage SSH password for sshpass

Artifacts:
  <out-dir>/history.ndjson         persistent sample history (default)
  <out-dir>/latest_summary.json    latest recent-window summary
  <out-dir>/latest_summary.md      latest recent-window summary in Markdown
  <out-dir>/runs/<timestamp>/...   per-run raw samples, config, and summary copies

Notes:
  - Counter deltas are derived only from samples whose `observed_since_unix_ms`
    matches the latest sample for that node, so process restarts/reset windows
    shrink coverage instead of producing negative deltas.
  - Use repeated invocations (cron/systemd timer) or a longer single run to
    accumulate enough history for a full 10-minute answer.
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

run_ssh() {
  local target=$1
  local password=${2:-}
  shift 2
  local cmd=(
    ssh
    -o StrictHostKeyChecking=no
    -o ConnectTimeout="$ssh_timeout_secs"
    -o ServerAliveInterval=5
    "$target"
    "$@"
  )
  if [[ -n "$password" ]]; then
    SSHPASS="$password" sshpass -e "${cmd[@]}"
  else
    "${cmd[@]}"
  fi
}

capture_status() {
  local label=$1
  local mode=$2
  local status_url=$3
  local sample_index=$4
  local target=${5:-}
  local password=${6:-}
  local sample_dir="$run_dir/raw/$label/sample-$(printf '%03d' "$sample_index")"
  mkdir -p "$sample_dir"
  local status_file="$sample_dir/status.json"

  if [[ "$mode" == "local" ]]; then
    if curl -fsS "$status_url" >"$status_file" 2>"$sample_dir/status.stderr.log"; then
      return 0
    fi
    printf '{"ok":false,"fetch_error":"curl_failed"}\n' >"$status_file"
    return 0
  fi

  if run_ssh "$target" "$password" "curl -fsS '$status_url'" >"$status_file" 2>"$sample_dir/status.stderr.log"; then
    return 0
  fi
  printf '{"ok":false,"fetch_error":"ssh_or_curl_failed"}\n' >"$status_file"
}

append_sample_record() {
  local label=$1
  local mode=$2
  local status_url=$3
  local sample_dir="$run_dir/raw/$label/sample-$(printf '%03d' "$sample_index")"

  jq -c -n \
    --arg run_id "$run_id" \
    --arg label "$label" \
    --arg mode "$mode" \
    --arg status_url "$status_url" \
    --arg sample_index "$sample_index" \
    --arg captured_at "$captured_at" \
    --arg captured_at_unix_ms "$captured_at_unix_ms" \
    --slurpfile status "$sample_dir/status.json" \
    '{
      run_id: $run_id,
      sample_index: ($sample_index | tonumber),
      captured_at: $captured_at,
      captured_at_unix_ms: ($captured_at_unix_ms | tonumber),
      label: $label,
      mode: $mode,
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
    }' >> "$run_samples_ndjson"
}

samples=3
interval_secs=20
window_minutes=10
top_n=5
ssh_timeout_secs=8
out_root=".tmp/p2p_real_env_traffic_monitor"
history_path=""
summary_only=0

observer_status_url="http://127.0.0.1:5633/v1/chain/status"

sequencer_target="root@39.104.204.172"
sequencer_status_url="http://127.0.0.1:5631/v1/chain/status"

storage_target="root@39.104.205.67"
storage_status_url="http://127.0.0.1:5632/v1/chain/status"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --samples)
      samples=${2:-}
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
    --top-n)
      top_n=${2:-}
      shift 2
      ;;
    --ssh-timeout-secs)
      ssh_timeout_secs=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --history-path)
      history_path=${2:-}
      shift 2
      ;;
    --summary-only)
      summary_only=1
      shift
      ;;
    --observer-status-url)
      observer_status_url=${2:-}
      shift 2
      ;;
    --sequencer-target)
      sequencer_target=${2:-}
      shift 2
      ;;
    --sequencer-status-url)
      sequencer_status_url=${2:-}
      shift 2
      ;;
    --storage-target)
      storage_target=${2:-}
      shift 2
      ;;
    --storage-status-url)
      storage_status_url=${2:-}
      shift 2
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

if (( summary_only == 0 )); then
  ensure_positive_int --samples "$samples"
  ensure_positive_int --interval-secs "$interval_secs"
fi
ensure_positive_int --window-minutes "$window_minutes"
ensure_positive_int --top-n "$top_n"
ensure_positive_int --ssh-timeout-secs "$ssh_timeout_secs"

if [[ -z "$history_path" ]]; then
  history_path="$out_root/history.ndjson"
fi

run_id=$(date +"%Y%m%d-%H%M%S")
run_dir="$out_root/runs/$run_id"
run_samples_ndjson="$run_dir/samples.ndjson"
run_summary_json="$run_dir/summary.json"
run_summary_md="$run_dir/summary.md"
latest_summary_json="$out_root/latest_summary.json"
latest_summary_md="$out_root/latest_summary.md"
config_json="$run_dir/config.json"

mkdir -p "$run_dir/raw/observer_local" "$run_dir/raw/sequencer_ecs" "$run_dir/raw/storage_ecs"
mkdir -p "$(dirname "$history_path")"
: > "$run_samples_ndjson"
touch "$history_path"

seq_password=${P2PARCH6_SEQ_SSH_PASSWORD:-}
storage_password=${P2PARCH6_STORAGE_SSH_PASSWORD:-}

jq -n \
  --arg run_id "$run_id" \
  --arg run_dir "$run_dir" \
  --arg history_path "$history_path" \
  --arg latest_summary_json "$latest_summary_json" \
  --arg latest_summary_md "$latest_summary_md" \
  --arg observer_status_url "$observer_status_url" \
  --arg sequencer_target "$sequencer_target" \
  --arg sequencer_status_url "$sequencer_status_url" \
  --arg storage_target "$storage_target" \
  --arg storage_status_url "$storage_status_url" \
  --argjson samples "$samples" \
  --argjson interval_secs "$interval_secs" \
  --argjson window_minutes "$window_minutes" \
  --argjson top_n "$top_n" \
  --argjson ssh_timeout_secs "$ssh_timeout_secs" \
  --argjson summary_only "$summary_only" \
  '{
    run_id: $run_id,
    run_dir: $run_dir,
    history_path: $history_path,
    latest_summary_json: $latest_summary_json,
    latest_summary_md: $latest_summary_md,
    samples: $samples,
    interval_secs: $interval_secs,
    window_minutes: $window_minutes,
    top_n: $top_n,
    ssh_timeout_secs: $ssh_timeout_secs,
    summary_only: ($summary_only == 1),
    nodes: {
      observer_local: {
        mode: "local",
        status_url: $observer_status_url
      },
      sequencer_ecs: {
        mode: "remote",
        target: $sequencer_target,
        status_url: $sequencer_status_url
      },
      storage_ecs: {
        mode: "remote",
        target: $storage_target,
        status_url: $storage_status_url
      }
    }
  }' > "$config_json"

if (( summary_only == 0 )); then
  for ((sample_index = 1; sample_index <= samples; sample_index++)); do
    captured_at=$(date -Iseconds)
    captured_at_unix_ms=$(( $(date +%s) * 1000 ))
    echo "sample $sample_index/$samples @ $captured_at"

    capture_status observer_local local "$observer_status_url" "$sample_index"
    append_sample_record observer_local local "$observer_status_url"

    capture_status sequencer_ecs remote "$sequencer_status_url" "$sample_index" "$sequencer_target" "$seq_password"
    append_sample_record sequencer_ecs remote "$sequencer_status_url"

    capture_status storage_ecs remote "$storage_status_url" "$sample_index" "$storage_target" "$storage_password"
    append_sample_record storage_ecs remote "$storage_status_url"

    if (( sample_index < samples )); then
      sleep "$interval_secs"
    fi
  done

  cat "$run_samples_ndjson" >> "$history_path"
fi

if [[ ! -s "$history_path" ]]; then
  echo "history file is empty: $history_path" >&2
  exit 1
fi

python3 - "$history_path" "$latest_summary_json" "$latest_summary_md" "$window_minutes" "$top_n" "$run_id" "$run_dir" <<'PY'
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def load_records(path: Path):
    records = []
    decoder = json.JSONDecoder()
    text = path.read_text(encoding="utf-8")
    index = 0
    text_len = len(text)
    while index < text_len:
        while index < text_len and text[index].isspace():
            index += 1
        if index >= text_len:
            break
        record, next_index = decoder.raw_decode(text, index)
        records.append(record)
        index = next_index
    return records


def clamp_delta(current, baseline):
    if current is None:
        current = 0
    if baseline is None:
        baseline = 0
    return max(0, int(current) - int(baseline))


def counter_key_for_lane(lane_name):
    return "datagrams" if lane_name == "udp_gossip" else "messages"


def delta_direction(current, baseline, counter_key):
    current = current or {}
    baseline = baseline or {}
    return {
        counter_key: clamp_delta(current.get(counter_key), baseline.get(counter_key)),
        "payload_bytes": clamp_delta(
            current.get("payload_bytes"), baseline.get("payload_bytes")
        ),
    }


def delta_lane_entry(current, baseline, counter_key):
    current = current or {}
    baseline = baseline or {}
    return {
        "inbound": delta_direction(current.get("inbound"), baseline.get("inbound"), counter_key),
        "outbound": delta_direction(
            current.get("outbound"), baseline.get("outbound"), counter_key
        ),
    }


def delta_named_map(current_map, baseline_map, counter_key):
    current_map = current_map or {}
    baseline_map = baseline_map or {}
    result = {}
    for name in sorted(set(current_map) | set(baseline_map)):
        result[name] = delta_lane_entry(current_map.get(name), baseline_map.get(name), counter_key)
    return result


def top_entries(delta_map, counter_key, top_n):
    items = []
    for name, entry in (delta_map or {}).items():
      inbound = (entry or {}).get("inbound") or {}
      outbound = (entry or {}).get("outbound") or {}
      total_count = int(inbound.get(counter_key, 0)) + int(outbound.get(counter_key, 0))
      total_payload = int(inbound.get("payload_bytes", 0)) + int(
          outbound.get("payload_bytes", 0)
      )
      if total_count == 0 and total_payload == 0:
          continue
      items.append(
          {
              "name": name,
              "total_count": total_count,
              "total_payload_bytes": total_payload,
              "inbound": inbound,
              "outbound": outbound,
          }
      )
    items.sort(
        key=lambda item: (
            item["total_payload_bytes"],
            item["total_count"],
            item["name"],
        ),
        reverse=True,
    )
    return items[:top_n]


def summarize_lane(lane_name, current_lane, baseline_lane, top_n):
    if current_lane is None:
        return {"available": False}
    counter_key = counter_key_for_lane(lane_name)
    summary = {
        "available": True,
        "scope": current_lane.get("scope"),
        "observed_since_unix_ms": current_lane.get("observed_since_unix_ms"),
        "counter_key": counter_key,
        "totals": delta_lane_entry(
            current_lane.get("totals"), (baseline_lane or {}).get("totals"), counter_key
        ),
    }
    if lane_name == "udp_gossip":
        summary["top_kinds"] = top_entries(
            delta_named_map(
                current_lane.get("by_kind"), (baseline_lane or {}).get("by_kind"), counter_key
            ),
            counter_key,
            top_n,
        )
    else:
        summary["gossip"] = delta_lane_entry(
            current_lane.get("gossip"), (baseline_lane or {}).get("gossip"), counter_key
        )
        summary["request"] = delta_lane_entry(
            current_lane.get("request"), (baseline_lane or {}).get("request"), counter_key
        )
        summary["response"] = delta_lane_entry(
            current_lane.get("response"), (baseline_lane or {}).get("response"), counter_key
        )
        summary["top_topics"] = top_entries(
            delta_named_map(
                current_lane.get("by_topic"), (baseline_lane or {}).get("by_topic"), counter_key
            ),
            counter_key,
            top_n,
        )
        summary["top_protocols"] = top_entries(
            delta_named_map(
                current_lane.get("by_protocol"),
                (baseline_lane or {}).get("by_protocol"),
                counter_key,
            ),
            counter_key,
            top_n,
        )
    return summary


def lane_observed_since(record, lane_name):
    traffic = (record or {}).get("traffic") or {}
    lane = traffic.get(lane_name) or {}
    return lane.get("observed_since_unix_ms")


def compatible_with_latest(record, latest):
    if record.get("status_fetch_ok") is not True:
        return False
    latest_node = latest.get("node_id")
    if latest_node and record.get("node_id") not in (None, latest_node):
        return False
    for lane_name in ("udp_gossip", "libp2p_replication"):
        latest_since = lane_observed_since(latest, lane_name)
        if latest_since is None:
            continue
        if lane_observed_since(record, lane_name) != latest_since:
            return False
    return True


def summarize_node(label, node_records, window_minutes, top_n):
    successful = [record for record in node_records if record.get("status_fetch_ok") is True]
    if not successful:
        latest_any = node_records[-1] if node_records else None
        return {
            "label": label,
            "available": False,
            "sample_count_total": len(node_records),
            "sample_count_successful": 0,
            "latest_fetch_error": (latest_any or {}).get("fetch_error"),
        }

    latest = successful[-1]
    window_start_ms = int(latest["captured_at_unix_ms"]) - window_minutes * 60 * 1000
    in_window = [
        record for record in successful if int(record["captured_at_unix_ms"]) >= window_start_ms
    ]
    compatible = [record for record in in_window if compatible_with_latest(record, latest)]
    if compatible:
        baseline = compatible[0]
    else:
        baseline = latest
        compatible = [latest]

    restart_detected = any(not compatible_with_latest(record, latest) for record in in_window)
    coverage_minutes = round(
        max(
            0,
            int(latest["captured_at_unix_ms"]) - int(baseline["captured_at_unix_ms"]),
        )
        / 60000.0,
        2,
    )
    requested_covered = coverage_minutes >= max(0.0, window_minutes - 0.01)

    traffic_latest = latest.get("traffic") or {}
    traffic_baseline = baseline.get("traffic") or {}

    summary = {
        "label": label,
        "available": True,
        "sample_count_total": len(node_records),
        "sample_count_successful": len(successful),
        "window": {
            "requested_minutes": window_minutes,
            "covered_minutes": coverage_minutes,
            "sample_count_in_window": len(in_window),
            "compatible_sample_count": len(compatible),
            "baseline_captured_at": baseline.get("captured_at"),
            "baseline_captured_at_unix_ms": baseline.get("captured_at_unix_ms"),
            "latest_captured_at": latest.get("captured_at"),
            "latest_captured_at_unix_ms": latest.get("captured_at_unix_ms"),
            "restart_or_counter_reset_detected_within_window": restart_detected,
            "full_window_covered": requested_covered,
        },
        "latest": {
            "node_id": latest.get("node_id"),
            "world_id": latest.get("world_id"),
            "role": latest.get("role"),
            "running": latest.get("running"),
            "status_observed_at_unix_ms": latest.get("status_observed_at_unix_ms"),
            "last_error": latest.get("last_error"),
        },
        "consensus": {
            "committed_height": {
                "baseline": baseline.get("consensus", {}).get("committed_height"),
                "latest": latest.get("consensus", {}).get("committed_height"),
                "delta": clamp_delta(
                    latest.get("consensus", {}).get("committed_height"),
                    baseline.get("consensus", {}).get("committed_height"),
                ),
            },
            "network_committed_height": {
                "baseline": baseline.get("consensus", {}).get("network_committed_height"),
                "latest": latest.get("consensus", {}).get("network_committed_height"),
                "delta": clamp_delta(
                    latest.get("consensus", {}).get("network_committed_height"),
                    baseline.get("consensus", {}).get("network_committed_height"),
                ),
            },
            "known_peer_heads": {
                "baseline": baseline.get("consensus", {}).get("known_peer_heads"),
                "latest": latest.get("consensus", {}).get("known_peer_heads"),
            },
        },
        "recent_errors": {
            "storage_recent_errors_count": {
                "baseline": baseline.get("storage_recent_errors_count"),
                "latest": latest.get("storage_recent_errors_count"),
                "delta": clamp_delta(
                    latest.get("storage_recent_errors_count"),
                    baseline.get("storage_recent_errors_count"),
                ),
            },
            "reward_runtime_recent_errors_count": {
                "baseline": baseline.get("reward_runtime_recent_errors_count"),
                "latest": latest.get("reward_runtime_recent_errors_count"),
                "delta": clamp_delta(
                    latest.get("reward_runtime_recent_errors_count"),
                    baseline.get("reward_runtime_recent_errors_count"),
                ),
            },
        },
        "traffic": {
            "udp_gossip": summarize_lane(
                "udp_gossip",
                traffic_latest.get("udp_gossip"),
                traffic_baseline.get("udp_gossip"),
                top_n,
            ),
            "libp2p_replication": summarize_lane(
                "libp2p_replication",
                traffic_latest.get("libp2p_replication"),
                traffic_baseline.get("libp2p_replication"),
                top_n,
            ),
        },
    }
    return summary


def fmt_num(value):
    if value is None:
        return "n/a"
    return f"{int(value):,}"


def fmt_bytes(value):
    if value is None:
        return "n/a"
    units = ["B", "KiB", "MiB", "GiB", "TiB"]
    amount = float(value)
    for unit in units:
        if amount < 1024.0 or unit == units[-1]:
            if unit == "B":
                return f"{int(amount)} {unit}"
            return f"{amount:.2f} {unit}"
        amount /= 1024.0
    return f"{int(value)} B"


def fmt_traffic_totals(name, lane):
    if not lane.get("available"):
        return f"- {name}: unavailable"
    counter_key = lane["counter_key"]
    totals = lane["totals"]
    inbound = totals["inbound"]
    outbound = totals["outbound"]
    return (
        f"- {name}: inbound +{fmt_num(inbound[counter_key])} {counter_key}, "
        f"+{fmt_bytes(inbound['payload_bytes'])}; outbound +{fmt_num(outbound[counter_key])} "
        f"{counter_key}, +{fmt_bytes(outbound['payload_bytes'])}"
    )


def render_top_block(title, entries, counter_key):
    if not entries:
        return [f"- {title}: none"]
    lines = [f"- {title}:"]
    for entry in entries:
        inbound = entry["inbound"]
        outbound = entry["outbound"]
        lines.append(
            "  "
            + f"{entry['name']}: total +{fmt_bytes(entry['total_payload_bytes'])}, "
            + f"inbound +{fmt_num(inbound.get(counter_key))} / {fmt_bytes(inbound.get('payload_bytes'))}, "
            + f"outbound +{fmt_num(outbound.get(counter_key))} / {fmt_bytes(outbound.get('payload_bytes'))}"
        )
    return lines


history_path = Path(sys.argv[1])
summary_json_path = Path(sys.argv[2])
summary_md_path = Path(sys.argv[3])
window_minutes = int(sys.argv[4])
top_n = int(sys.argv[5])
run_id = sys.argv[6]
run_dir = sys.argv[7]
generated_at = datetime.now(tz=timezone.utc).isoformat()

records = load_records(history_path)
labels = ["observer_local", "sequencer_ecs", "storage_ecs"]
records_by_label = {label: [] for label in labels}
for record in records:
    label = record.get("label")
    if label in records_by_label:
        records_by_label[label].append(record)

summary = {
    "ok": True,
    "generated_at": generated_at,
    "run_id": run_id,
    "run_dir": run_dir,
    "history_path": str(history_path),
    "history_record_count": len(records),
    "window_minutes_requested": window_minutes,
    "top_n": top_n,
    "nodes": {},
}

for label in labels:
    summary["nodes"][label] = summarize_node(
        label, records_by_label[label], window_minutes, top_n
    )

summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
)

md_lines = [
    "# P2P Real Env Traffic Monitor Summary",
    "",
    f"- Generated at: `{generated_at}`",
    f"- History file: `{history_path}`",
    f"- History record count: `{len(records)}`",
    f"- Requested window: `{window_minutes}` minutes",
    f"- Top contributors per map: `{top_n}`",
    "",
]

for label in labels:
    node = summary["nodes"][label]
    md_lines.append(f"## {label}")
    if not node.get("available"):
        md_lines.append(
            f"- No successful samples yet. latest_fetch_error=`{node.get('latest_fetch_error')}`"
        )
        md_lines.append("")
        continue

    window = node["window"]
    latest = node["latest"]
    consensus = node["consensus"]
    errors = node["recent_errors"]
    md_lines.extend(
        [
            f"- Node: `{latest.get('node_id')}` role=`{latest.get('role')}` running=`{latest.get('running')}`",
            f"- Window coverage: `{window['covered_minutes']}` / `{window['requested_minutes']}` minutes across `{window['compatible_sample_count']}` compatible samples",
            f"- Baseline sample: `{window['baseline_captured_at']}` | Latest sample: `{window['latest_captured_at']}`",
            f"- Restart/counter reset inside requested window: `{window['restart_or_counter_reset_detected_within_window']}`",
            f"- Height delta: committed `+{fmt_num(consensus['committed_height']['delta'])}` (`{fmt_num(consensus['committed_height']['baseline'])}` -> `{fmt_num(consensus['committed_height']['latest'])}`), network `+{fmt_num(consensus['network_committed_height']['delta'])}` (`{fmt_num(consensus['network_committed_height']['baseline'])}` -> `{fmt_num(consensus['network_committed_height']['latest'])}`)",
            f"- Known peer heads: `{fmt_num(consensus['known_peer_heads']['baseline'])}` -> `{fmt_num(consensus['known_peer_heads']['latest'])}`",
            f"- Recent errors: storage `+{fmt_num(errors['storage_recent_errors_count']['delta'])}` (`{fmt_num(errors['storage_recent_errors_count']['baseline'])}` -> `{fmt_num(errors['storage_recent_errors_count']['latest'])}`), reward runtime `+{fmt_num(errors['reward_runtime_recent_errors_count']['delta'])}` (`{fmt_num(errors['reward_runtime_recent_errors_count']['baseline'])}` -> `{fmt_num(errors['reward_runtime_recent_errors_count']['latest'])}`)",
            f"- Last error: `{latest.get('last_error')}`",
            fmt_traffic_totals("UDP gossip", node["traffic"]["udp_gossip"]),
            fmt_traffic_totals(
                "Libp2p replication", node["traffic"]["libp2p_replication"]
            ),
        ]
    )

    udp = node["traffic"]["udp_gossip"]
    if udp.get("available"):
        md_lines.extend(render_top_block("Top UDP kinds", udp.get("top_kinds"), "datagrams"))

    libp2p = node["traffic"]["libp2p_replication"]
    if libp2p.get("available"):
        md_lines.append(
            "- Libp2p lanes: "
            + f"gossip +{fmt_bytes(libp2p['gossip']['inbound']['payload_bytes'] + libp2p['gossip']['outbound']['payload_bytes'])}, "
            + f"request +{fmt_bytes(libp2p['request']['inbound']['payload_bytes'] + libp2p['request']['outbound']['payload_bytes'])}, "
            + f"response +{fmt_bytes(libp2p['response']['inbound']['payload_bytes'] + libp2p['response']['outbound']['payload_bytes'])}"
        )
        md_lines.extend(
            render_top_block(
                "Top libp2p protocols", libp2p.get("top_protocols"), "messages"
            )
        )
        md_lines.extend(
            render_top_block("Top libp2p topics", libp2p.get("top_topics"), "messages")
        )
    md_lines.append("")

summary_md_path.write_text("\n".join(md_lines), encoding="utf-8")
PY

cp "$latest_summary_json" "$run_summary_json"
cp "$latest_summary_md" "$run_summary_md"

printf 'history: %s\nlatest_summary_json: %s\nlatest_summary_md: %s\nrun_dir: %s\n' \
  "$history_path" "$latest_summary_json" "$latest_summary_md" "$run_dir"
