#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ./scripts/oasis7-node-observability-report.sh [options]

Summarize live node observability from `/v1/chain/status`, optionally folding in a
recent traffic-window summary produced by `oasis7-node-traffic-monitor.sh`.

Options:
  --status-url <url>            status endpoint to fetch
                                (default: http://127.0.0.1:5633/v1/chain/status)
  --status-json-path <path>     read status payload from a local JSON file instead of HTTP
  --traffic-summary-json <path> optional traffic summary json to attach
  --node-label <label>          label written into summary output
                                (default: local_node)
  --out-dir <path>              output root
                                (default: .tmp/oasis7_node_observability)
  --summary-json <path>         override latest summary json path
  --summary-md <path>           override latest summary markdown path
  -h, --help                    show help

Artifacts:
  <out-dir>/latest_summary.json
  <out-dir>/latest_summary.md
USAGE
}

status_url="http://127.0.0.1:5633/v1/chain/status"
status_json_path=""
traffic_summary_json=""
node_label="local_node"
out_dir=".tmp/oasis7_node_observability"
summary_json_path=""
summary_md_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status-url)
      status_url=${2:-}
      shift 2
      ;;
    --status-json-path)
      status_json_path=${2:-}
      shift 2
      ;;
    --traffic-summary-json)
      traffic_summary_json=${2:-}
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
    --summary-json)
      summary_json_path=${2:-}
      shift 2
      ;;
    --summary-md)
      summary_md_path=${2:-}
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

if [[ -z "$summary_json_path" ]]; then
  summary_json_path="$out_dir/latest_summary.json"
fi
if [[ -z "$summary_md_path" ]]; then
  summary_md_path="$out_dir/latest_summary.md"
fi

mkdir -p "$out_dir"
mkdir -p "$(dirname "$summary_json_path")" "$(dirname "$summary_md_path")"

status_tmp="$(mktemp)"
status_fetch_ok=1
fetch_error=""

cleanup() {
  rm -f "$status_tmp" "$status_tmp.stderr"
}
trap cleanup EXIT

if [[ -n "$status_json_path" ]]; then
  if [[ ! -f "$status_json_path" ]]; then
    echo "status json does not exist: $status_json_path" >&2
    exit 2
  fi
  cp "$status_json_path" "$status_tmp"
else
  if ! curl -fsS "$status_url" >"$status_tmp" 2>"$status_tmp.stderr"; then
    status_fetch_ok=0
    fetch_error="$(tr '\n' ' ' <"$status_tmp.stderr" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"
    printf '{}' >"$status_tmp"
  fi
fi

python3 - "$status_tmp" "$summary_json_path" "$summary_md_path" "$node_label" "$status_url" "$traffic_summary_json" "$status_fetch_ok" "$fetch_error" "$status_json_path" <<'PY'
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def load_json(path: str) -> dict:
    raw = Path(path).read_text(encoding="utf-8")
    if not raw.strip():
        return {}
    return json.loads(raw)


def fmt_bool(value):
    if value is None:
        return "n/a"
    return "yes" if value else "no"


def fmt_num(value):
    if value is None:
        return "n/a"
    return f"{int(value):,}"


status_path, summary_json_path, summary_md_path, node_label, status_url, traffic_summary_path, status_fetch_ok_raw, fetch_error, status_json_path = sys.argv[1:10]
generated_at = datetime.now(timezone.utc).astimezone().isoformat()
status_fetch_ok = status_fetch_ok_raw == "1"
status = load_json(status_path)
traffic_summary = None
traffic_summary_missing = False
if traffic_summary_path:
    traffic_path = Path(traffic_summary_path)
    if traffic_path.is_file():
        traffic_summary = load_json(str(traffic_path))
    else:
        traffic_summary_missing = True

observability = status.get("observability") or {}
consensus = status.get("consensus") or {}
storage = status.get("storage") or {}
reward_runtime = status.get("reward_runtime") or {}
alerts = observability.get("alerts") or []

summary = {
    "generated_at": generated_at,
    "node_label": node_label,
    "status_source": "file" if status_json_path else "http",
    "status_url": None if status_json_path else status_url,
    "status_json_path": status_json_path or None,
    "status_fetch_ok": status_fetch_ok,
    "fetch_error": None if status_fetch_ok else (fetch_error or "status fetch failed"),
    "latest": {
        "node_id": status.get("node_id"),
        "world_id": status.get("world_id"),
        "role": status.get("role"),
        "running": status.get("running"),
        "observed_at_unix_ms": status.get("observed_at_unix_ms"),
        "tick_count": status.get("tick_count"),
        "last_error": status.get("last_error"),
    },
    "observability": {
        "available": bool(observability),
        "status": observability.get("status"),
        "summary": observability.get("summary"),
        "connected_peer_count": observability.get("connected_peer_count"),
        "active_peer_count": observability.get("active_peer_count"),
        "candidate_peer_count": observability.get("candidate_peer_count"),
        "suspect_peer_count": observability.get("suspect_peer_count"),
        "blocked_peer_count": observability.get("blocked_peer_count"),
        "peer_with_issues_count": observability.get("peer_with_issues_count"),
        "known_peer_heads": observability.get("known_peer_heads"),
        "network_height_lag": observability.get("network_height_lag"),
        "recent_replication_error_count": observability.get("recent_replication_error_count"),
        "storage_degraded": observability.get("storage_degraded"),
        "reward_runtime_degraded": observability.get("reward_runtime_degraded"),
        "alerts": alerts,
    },
    "consensus": {
        "committed_height": consensus.get("committed_height"),
        "network_committed_height": consensus.get("network_committed_height"),
        "known_peer_heads": consensus.get("known_peer_heads"),
    },
    "storage": {
        "degraded_reason": storage.get("degraded_reason"),
        "last_gc_result": storage.get("last_gc_result"),
        "last_gc_error": storage.get("last_gc_error"),
        "orphan_blob_count": storage.get("orphan_blob_count"),
        "checkpoint_count": storage.get("checkpoint_count"),
    },
    "reward_runtime": {
        "enabled": reward_runtime.get("enabled"),
        "metrics_available": reward_runtime.get("metrics_available"),
        "invariant_ok": reward_runtime.get("invariant_ok"),
        "last_error": reward_runtime.get("last_error"),
        "latest_epoch_index": reward_runtime.get("latest_epoch_index"),
        "report_count": reward_runtime.get("report_count"),
    },
    "traffic_window": traffic_summary,
    "traffic_summary_missing": traffic_summary_missing,
}

Path(summary_json_path).write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

lines = [
    "# Oasis7 Node Observability Summary",
    "",
    f"- generated_at: `{generated_at}`",
    f"- node_label: `{node_label}`",
    f"- status_source: `{'file' if status_json_path else 'http'}`",
    f"- status_fetch_ok: `{fmt_bool(status_fetch_ok)}`",
]
if status_fetch_ok:
    lines.extend(
        [
            f"- node_id: `{status.get('node_id')}`",
            f"- world_id: `{status.get('world_id')}`",
            f"- role: `{status.get('role')}`",
            f"- running: `{fmt_bool(status.get('running'))}`",
            f"- last_error: `{status.get('last_error')}`",
        ]
    )
else:
    lines.append(f"- fetch_error: `{summary['fetch_error']}`")

lines.extend(
    [
        "",
        "## Live Health",
        f"- status: `{observability.get('status')}`",
        f"- summary: `{observability.get('summary')}`",
        f"- connected_peers: `{fmt_num(observability.get('connected_peer_count'))}`",
        f"- peer_health_counts: `active={fmt_num(observability.get('active_peer_count'))} candidate={fmt_num(observability.get('candidate_peer_count'))} suspect={fmt_num(observability.get('suspect_peer_count'))} blocked={fmt_num(observability.get('blocked_peer_count'))}`",
        f"- peers_with_issues: `{fmt_num(observability.get('peer_with_issues_count'))}`",
        f"- known_peer_heads: `{fmt_num(observability.get('known_peer_heads'))}`",
        f"- network_height_lag: `{fmt_num(observability.get('network_height_lag'))}`",
        f"- recent_replication_error_count: `{fmt_num(observability.get('recent_replication_error_count'))}`",
        f"- storage_degraded: `{fmt_bool(observability.get('storage_degraded'))}`",
        f"- reward_runtime_degraded: `{fmt_bool(observability.get('reward_runtime_degraded'))}`",
    ]
)

if alerts:
    lines.extend(["", "## Active Alerts"])
    for alert in alerts:
        lines.append(
            f"- [{alert.get('severity', 'unknown')}] `{alert.get('code', 'unknown')}`: {alert.get('summary', '')}"
        )
else:
    lines.extend(["", "## Active Alerts", "- none"])

lines.extend(
    [
        "",
        "## Storage / Reward",
        f"- storage_degraded_reason: `{storage.get('degraded_reason')}`",
        f"- storage_last_gc_result: `{storage.get('last_gc_result')}`",
        f"- storage_last_gc_error: `{storage.get('last_gc_error')}`",
        f"- reward_runtime_enabled: `{fmt_bool(reward_runtime.get('enabled'))}`",
        f"- reward_runtime_metrics_available: `{fmt_bool(reward_runtime.get('metrics_available'))}`",
        f"- reward_runtime_invariant_ok: `{fmt_bool(reward_runtime.get('invariant_ok'))}`",
        f"- reward_runtime_last_error: `{reward_runtime.get('last_error')}`",
    ]
)

lines.extend(["", "## Traffic Window"])
if traffic_summary is None:
    if traffic_summary_missing:
        lines.append(f"- traffic summary file missing: `{traffic_summary_path}`")
    else:
        lines.append("- no traffic summary attached")
else:
    latest = traffic_summary.get("latest") or {}
    window = traffic_summary.get("window") or {}
    lines.extend(
        [
            f"- covered_minutes: `{window.get('covered_minutes')}`",
            f"- full_window_covered: `{fmt_bool(window.get('full_window_covered'))}`",
            f"- restart_or_counter_reset_detected_within_window: `{fmt_bool(window.get('restart_or_counter_reset_detected_within_window'))}`",
            f"- latest_last_error: `{latest.get('last_error')}`",
        ]
    )
    traffic = traffic_summary.get("traffic") or {}
    for lane_name in ("udp_gossip", "libp2p_replication"):
        lane = traffic.get(lane_name) or {}
        totals = lane.get("totals") or {}
        inbound = totals.get("inbound") or {}
        outbound = totals.get("outbound") or {}
        counter_key = lane.get("counter_key") or ("datagrams" if lane_name == "udp_gossip" else "messages")
        lines.append(
            f"- {lane_name}: `in_{counter_key}={fmt_num(inbound.get(counter_key))} out_{counter_key}={fmt_num(outbound.get(counter_key))} in_payload={fmt_num(inbound.get('payload_bytes'))} out_payload={fmt_num(outbound.get('payload_bytes'))}`"
        )

Path(summary_md_path).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
