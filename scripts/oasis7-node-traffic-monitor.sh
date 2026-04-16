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
  --top-n <n>                      top kind/topic/protocol rows (default: 5)
  --loop                           run forever instead of sampling once
  -h, --help                       show help

Artifacts:
  <out-dir>/history.ndjson
  <out-dir>/latest_summary.json
  <out-dir>/latest_summary.md

Notes:
  - Samples are append-only. The summary logic compares only records whose
    `traffic.*.observed_since_unix_ms` matches the latest successful sample, so
    node restarts shrink the covered window instead of producing bogus deltas.
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
  python3 - "$history_path" "$summary_json_path" "$summary_md_path" "$window_minutes" "$top_n" <<'PY'
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def load_records(path: Path):
    decoder = json.JSONDecoder()
    text = path.read_text(encoding="utf-8")
    index = 0
    records = []
    while index < len(text):
        while index < len(text) and text[index].isspace():
            index += 1
        if index >= len(text):
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


def summarize_lane(lane_name, current_lane, baseline_lane, top_n):
    if current_lane is None:
        return {"available": False}
    counter_key = counter_key_for_lane(lane_name)
    result = {
        "available": True,
        "counter_key": counter_key,
        "scope": current_lane.get("scope"),
        "observed_since_unix_ms": current_lane.get("observed_since_unix_ms"),
        "totals": delta_lane_entry(
            current_lane.get("totals"), (baseline_lane or {}).get("totals"), counter_key
        ),
    }
    if lane_name == "udp_gossip":
        result["top_kinds"] = top_entries(
            delta_named_map(
                current_lane.get("by_kind"), (baseline_lane or {}).get("by_kind"), counter_key
            ),
            counter_key,
            top_n,
        )
    else:
        result["gossip"] = delta_lane_entry(
            current_lane.get("gossip"), (baseline_lane or {}).get("gossip"), counter_key
        )
        result["request"] = delta_lane_entry(
            current_lane.get("request"), (baseline_lane or {}).get("request"), counter_key
        )
        result["response"] = delta_lane_entry(
            current_lane.get("response"), (baseline_lane or {}).get("response"), counter_key
        )
        result["top_topics"] = top_entries(
            delta_named_map(
                current_lane.get("by_topic"), (baseline_lane or {}).get("by_topic"), counter_key
            ),
            counter_key,
            top_n,
        )
        result["top_protocols"] = top_entries(
            delta_named_map(
                current_lane.get("by_protocol"),
                (baseline_lane or {}).get("by_protocol"),
                counter_key,
            ),
            counter_key,
            top_n,
        )
    return result


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


history_path = Path(sys.argv[1])
summary_json_path = Path(sys.argv[2])
summary_md_path = Path(sys.argv[3])
window_minutes = int(sys.argv[4])
top_n = int(sys.argv[5])
records = load_records(history_path)
generated_at = datetime.now(tz=timezone.utc).isoformat()

successful = [record for record in records if record.get("status_fetch_ok") is True]
summary = {
    "ok": True,
    "generated_at": generated_at,
    "history_path": str(history_path),
    "history_record_count": len(records),
    "window_minutes_requested": window_minutes,
    "top_n": top_n,
}

if not successful:
    summary["node"] = {
        "available": False,
        "sample_count_total": len(records),
        "sample_count_successful": 0,
        "latest_fetch_error": (records[-1] if records else {}).get("fetch_error"),
    }
else:
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

    coverage_minutes = round(
        max(
            0,
            int(latest["captured_at_unix_ms"]) - int(baseline["captured_at_unix_ms"]),
        )
        / 60000.0,
        2,
    )
    restart_detected = any(not compatible_with_latest(record, latest) for record in in_window)

    traffic_latest = latest.get("traffic") or {}
    traffic_baseline = baseline.get("traffic") or {}
    summary["node"] = {
        "available": True,
        "sample_count_total": len(records),
        "sample_count_successful": len(successful),
        "window": {
            "requested_minutes": window_minutes,
            "covered_minutes": coverage_minutes,
            "sample_count_in_window": len(in_window),
            "compatible_sample_count": len(compatible),
            "baseline_captured_at": baseline.get("captured_at"),
            "latest_captured_at": latest.get("captured_at"),
            "restart_or_counter_reset_detected_within_window": restart_detected,
            "full_window_covered": coverage_minutes >= max(0.0, window_minutes - 0.01),
        },
        "latest": {
            "node_label": latest.get("node_label"),
            "node_id": latest.get("node_id"),
            "world_id": latest.get("world_id"),
            "role": latest.get("role"),
            "running": latest.get("running"),
            "status_url": latest.get("status_url"),
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

summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
)

lines = [
    "# Oasis7 Node Traffic Monitor Summary",
    "",
    f"- Generated at: `{generated_at}`",
    f"- History file: `{history_path}`",
    f"- History record count: `{len(records)}`",
    f"- Requested window: `{window_minutes}` minutes",
    f"- Top contributors per map: `{top_n}`",
    "",
]

node = summary["node"]
if not node.get("available"):
    lines.append(
        f"- No successful samples yet. latest_fetch_error=`{node.get('latest_fetch_error')}`"
    )
else:
    window = node["window"]
    latest = node["latest"]
    consensus = node["consensus"]
    errors = node["recent_errors"]
    udp = node["traffic"]["udp_gossip"]
    libp2p = node["traffic"]["libp2p_replication"]
    lines.extend(
        [
            f"- Node: `{latest['node_label']}` node_id=`{latest['node_id']}` role=`{latest['role']}` running=`{latest['running']}`",
            f"- Status URL: `{latest['status_url']}`",
            f"- Window coverage: `{window['covered_minutes']}` / `{window['requested_minutes']}` minutes across `{window['compatible_sample_count']}` compatible samples",
            f"- Baseline sample: `{window['baseline_captured_at']}` | Latest sample: `{window['latest_captured_at']}`",
            f"- Restart/counter reset inside requested window: `{window['restart_or_counter_reset_detected_within_window']}`",
            f"- Height delta: committed `+{fmt_num(consensus['committed_height']['delta'])}` (`{fmt_num(consensus['committed_height']['baseline'])}` -> `{fmt_num(consensus['committed_height']['latest'])}`), network `+{fmt_num(consensus['network_committed_height']['delta'])}` (`{fmt_num(consensus['network_committed_height']['baseline'])}` -> `{fmt_num(consensus['network_committed_height']['latest'])}`)",
            f"- Known peer heads: `{fmt_num(consensus['known_peer_heads']['baseline'])}` -> `{fmt_num(consensus['known_peer_heads']['latest'])}`",
            f"- Recent errors: storage `+{fmt_num(errors['storage_recent_errors_count']['delta'])}` (`{fmt_num(errors['storage_recent_errors_count']['baseline'])}` -> `{fmt_num(errors['storage_recent_errors_count']['latest'])}`), reward runtime `+{fmt_num(errors['reward_runtime_recent_errors_count']['delta'])}` (`{fmt_num(errors['reward_runtime_recent_errors_count']['baseline'])}` -> `{fmt_num(errors['reward_runtime_recent_errors_count']['latest'])}`)",
            f"- Last error: `{latest['last_error']}`",
        ]
    )

    if udp.get("available"):
        totals = udp["totals"]
        lines.append(
            f"- UDP gossip: inbound +{fmt_num(totals['inbound']['datagrams'])} datagrams, +{fmt_bytes(totals['inbound']['payload_bytes'])}; outbound +{fmt_num(totals['outbound']['datagrams'])} datagrams, +{fmt_bytes(totals['outbound']['payload_bytes'])}"
        )
        if udp.get("top_kinds"):
            lines.append("- Top UDP kinds:")
            for entry in udp["top_kinds"]:
                lines.append(
                    "  "
                    + f"{entry['name']}: total +{fmt_bytes(entry['total_payload_bytes'])}, inbound +{fmt_num(entry['inbound'].get('datagrams'))} / {fmt_bytes(entry['inbound'].get('payload_bytes'))}, outbound +{fmt_num(entry['outbound'].get('datagrams'))} / {fmt_bytes(entry['outbound'].get('payload_bytes'))}"
                )

    if libp2p.get("available"):
        totals = libp2p["totals"]
        lines.append(
            f"- Libp2p replication: inbound +{fmt_num(totals['inbound']['messages'])} messages, +{fmt_bytes(totals['inbound']['payload_bytes'])}; outbound +{fmt_num(totals['outbound']['messages'])} messages, +{fmt_bytes(totals['outbound']['payload_bytes'])}"
        )
        lines.append(
            "- Libp2p lanes: "
            + f"gossip +{fmt_bytes(libp2p['gossip']['inbound']['payload_bytes'] + libp2p['gossip']['outbound']['payload_bytes'])}, "
            + f"request +{fmt_bytes(libp2p['request']['inbound']['payload_bytes'] + libp2p['request']['outbound']['payload_bytes'])}, "
            + f"response +{fmt_bytes(libp2p['response']['inbound']['payload_bytes'] + libp2p['response']['outbound']['payload_bytes'])}"
        )
        if libp2p.get("top_protocols"):
            lines.append("- Top libp2p protocols:")
            for entry in libp2p["top_protocols"]:
                lines.append(
                    "  "
                    + f"{entry['name']}: total +{fmt_bytes(entry['total_payload_bytes'])}, inbound +{fmt_num(entry['inbound'].get('messages'))} / {fmt_bytes(entry['inbound'].get('payload_bytes'))}, outbound +{fmt_num(entry['outbound'].get('messages'))} / {fmt_bytes(entry['outbound'].get('payload_bytes'))}"
                )
        if libp2p.get("top_topics"):
            lines.append("- Top libp2p topics:")
            for entry in libp2p["top_topics"]:
                lines.append(
                    "  "
                    + f"{entry['name']}: total +{fmt_bytes(entry['total_payload_bytes'])}, inbound +{fmt_num(entry['inbound'].get('messages'))} / {fmt_bytes(entry['inbound'].get('payload_bytes'))}, outbound +{fmt_num(entry['outbound'].get('messages'))} / {fmt_bytes(entry['outbound'].get('payload_bytes'))}"
                )

summary_md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
}

while true; do
  sample_once
  write_summary
  if (( loop_mode == 0 )); then
    break
  fi
  sleep "$interval_secs"
done
