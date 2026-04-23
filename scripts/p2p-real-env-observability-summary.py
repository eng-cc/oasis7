#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

NODE_LABELS = ("observer_local", "sequencer_ecs", "storage_ecs")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Merge triad snapshot/host/traffic/wasm monitoring into one summary."
    )
    parser.add_argument("--snapshot-summary", required=True)
    parser.add_argument("--host-summary", required=True)
    parser.add_argument("--traffic-summary", required=True)
    parser.add_argument("--observer-wasm-summary", required=True)
    parser.add_argument("--sequencer-wasm-summary", required=True)
    parser.add_argument("--storage-wasm-summary", required=True)
    parser.add_argument("--summary-json", required=True)
    parser.add_argument("--summary-md", required=True)
    parser.add_argument("--run-id")
    parser.add_argument("--run-dir")
    return parser.parse_args()


def load_json(path: str) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def fmt_num(value):
    if value is None:
        return "n/a"
    if isinstance(value, float):
        return f"{value:.2f}"
    return f"{int(value):,}"


def fmt_percent(value):
    if value is None:
        return "n/a"
    return f"{float(value):.2f}%"


def fmt_bytes(value):
    if value is None:
        return "n/a"
    amount = float(value)
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if amount < 1024.0 or unit == "TiB":
            return f"{amount:.2f} {unit}" if unit != "B" else f"{int(amount)} B"
        amount /= 1024.0
    return f"{int(value)} B"


def derive_node_summary(label: str, snapshot: dict, host: dict, traffic: dict, wasm: dict) -> dict:
    snapshot_node = ((snapshot.get("nodes") or {}).get(label)) or {}
    host_node = ((host.get("nodes") or {}).get(label)) or {}
    traffic_node = ((traffic.get("nodes") or {}).get(label)) or {}
    wasm_window = wasm.get("window") or {}
    latest_wasm = wasm.get("latest") or {}

    roles = snapshot_node.get("roles") or []
    role = roles[0] if roles else (traffic_node.get("latest") or {}).get("role")
    latest_host = host_node.get("latest") or {}
    latest_runtime = latest_host.get("runtime") or {}
    latest_storage = latest_host.get("storage") or {}
    latest_traffic = traffic_node.get("latest") or {}
    libp2p = (traffic_node.get("traffic") or {}).get("libp2p_replication") or {}
    payload_bytes = int(libp2p.get("total_payload_bytes") or 0) + int(
        (((traffic_node.get("traffic") or {}).get("udp_gossip") or {}).get("total_payload_bytes") or 0)
    )

    alerts = []
    alerts.extend(snapshot.get("failure_signatures") or [])
    alerts.extend(host_node.get("alerts") or [])
    if latest_wasm.get("metrics_available") is not True:
      alerts.append("wasm_metrics_unavailable")
    if (latest_wasm.get("degraded_reason") or ""):
      alerts.append("wasm_degraded")
    if wasm_window.get("window_reset_detected") is True:
      alerts.append("wasm_counter_reset_detected")

    return {
        "label": label,
        "role": role,
        "node_id": ((snapshot_node.get("node_ids") or [None])[0]),
        "snapshot": {
            "sample_count": snapshot_node.get("sample_count"),
            "healthz_all_ok": snapshot_node.get("healthz_all_ok"),
            "status_fetch_all_ok": snapshot_node.get("status_fetch_all_ok"),
            "committed_height_first": ((snapshot_node.get("heights") or {}).get("first_committed_height")),
            "committed_height_last": ((snapshot_node.get("heights") or {}).get("last_committed_height")),
            "known_peer_heads_max": ((snapshot_node.get("peers") or {}).get("max_known_peer_heads")),
            "last_errors": snapshot_node.get("last_errors") or [],
        },
        "host": {
            "hostname": latest_host.get("hostname"),
            "cpu_cores": latest_host.get("cpu_cores"),
            "runtime_cpu_percent": latest_host.get("runtime_cpu_percent"),
            "runtime_cpu_core_ratio": latest_host.get("runtime_cpu_core_ratio"),
            "runtime_threads": latest_runtime.get("nlwp"),
            "loadavg_1m": latest_host.get("loadavg_1m"),
            "load_per_core_ratio_1m": latest_host.get("load_per_core_ratio_1m"),
            "mem_available_percent": latest_host.get("mem_available_percent"),
            "storage_used_percent": latest_storage.get("used_percent"),
            "status": host_node.get("status") or {},
        },
        "traffic": {
            "payload_total_bytes": payload_bytes,
            "window_covered_minutes": ((traffic_node.get("window") or {}).get("covered_minutes")),
            "libp2p_total_wire_bytes": libp2p.get("total_wire_bytes"),
            "control_plane_total_events": ((libp2p.get("control_plane") or {}).get("total_events")),
            "control_plane_total_wire_bytes": ((libp2p.get("control_plane") or {}).get("total_wire_bytes")),
            "last_error": latest_traffic.get("last_error"),
        },
        "wasm": {
            "metrics_available": latest_wasm.get("metrics_available"),
            "degraded_reason": latest_wasm.get("degraded_reason"),
            "window_available": wasm_window.get("available"),
            "window_reset_detected": wasm_window.get("window_reset_detected"),
            "top_hotspot": wasm_window.get("top_hotspot"),
            "executor_calls_total_delta": (((wasm_window.get("executor") or {}).get("calls_total_delta"))),
        },
        "alerts": sorted(set(alerts)),
    }


def render_markdown(summary: dict) -> list[str]:
    lines = [
        "# P2P Real Env Observability Summary",
        "",
        f"- Generated at: `{summary['generated_at']}`",
        f"- Run id: `{summary.get('run_id') or 'n/a'}`",
        f"- Snapshot claim_status: `{summary['snapshot'].get('claim_status')}`",
        f"- Overall verdict: `{summary['overall']['status']}`",
        f"- Overall alerts: `{', '.join(summary['overall'].get('alerts') or ['(none)'])}`",
        f"- Traffic total payload: `{fmt_bytes(((summary.get('traffic') or {}).get('aggregate') or {}).get('total_payload_bytes'))}`",
        "",
    ]
    aggregate = (summary.get("traffic") or {}).get("aggregate") or {}
    lane_distribution = aggregate.get("lane_distribution") or {}
    if lane_distribution:
        lines.append(
            "- Aggregate lanes: "
            + f"udp `{fmt_bytes(((lane_distribution.get('udp_gossip') or {}).get('payload_bytes')))}` ({fmt_percent(((lane_distribution.get('udp_gossip') or {}).get('share_percent')))}), "
            + f"libp2p `{fmt_bytes(((lane_distribution.get('libp2p_replication') or {}).get('payload_bytes')))}` ({fmt_percent(((lane_distribution.get('libp2p_replication') or {}).get('share_percent')))})."
        )
        lines.append("")
    for label in NODE_LABELS:
        node = (summary.get("nodes") or {}).get(label) or {}
        lines.extend(
            [
                f"## {label}",
                f"- role=`{node.get('role')}` node_id=`{node.get('node_id')}`",
                f"- heights=`{fmt_num((node.get('snapshot') or {}).get('committed_height_first'))} -> {fmt_num((node.get('snapshot') or {}).get('committed_height_last'))}` known_peer_heads_max=`{fmt_num((node.get('snapshot') or {}).get('known_peer_heads_max'))}`",
                f"- runtime_cpu=`{fmt_percent((node.get('host') or {}).get('runtime_cpu_percent'))}` core_ratio=`{fmt_percent((((node.get('host') or {}).get('runtime_cpu_core_ratio')) or 0) * 100) if (node.get('host') or {}).get('runtime_cpu_core_ratio') is not None else 'n/a'}` load1/core=`{fmt_num((node.get('host') or {}).get('load_per_core_ratio_1m'))}` mem_available=`{fmt_percent((node.get('host') or {}).get('mem_available_percent'))}` storage_used=`{fmt_percent((node.get('host') or {}).get('storage_used_percent'))}`",
                f"- traffic_payload=`{fmt_bytes((node.get('traffic') or {}).get('payload_total_bytes'))}` control_plane_events=`{fmt_num((node.get('traffic') or {}).get('control_plane_total_events'))}`",
                f"- wasm_metrics_available=`{(node.get('wasm') or {}).get('metrics_available')}` window_available=`{(node.get('wasm') or {}).get('window_available')}` hotspot=`{(node.get('wasm') or {}).get('top_hotspot')}`",
                f"- alerts=`{', '.join(node.get('alerts') or ['(none)'])}`",
                "",
            ]
        )
    return lines


def main():
    args = parse_args()
    snapshot = load_json(args.snapshot_summary)
    host = load_json(args.host_summary)
    traffic = load_json(args.traffic_summary)
    wasm = {
        "observer_local": load_json(args.observer_wasm_summary),
        "sequencer_ecs": load_json(args.sequencer_wasm_summary),
        "storage_ecs": load_json(args.storage_wasm_summary),
    }

    nodes = {
        label: derive_node_summary(label, snapshot, host, traffic, wasm[label])
        for label in NODE_LABELS
    }

    overall_alerts = []
    if snapshot.get("claim_status") != "pass_candidate":
        overall_alerts.extend(snapshot.get("failure_signatures") or [])
    overall_alerts.extend(host.get("aggregate", {}).get("alerted_nodes") or [])
    for label, node in nodes.items():
        for alert in node.get("alerts") or []:
            overall_alerts.append(f"{label}:{alert}")

    overall_status = "pass_candidate"
    if snapshot.get("claim_status") != "pass_candidate":
        overall_status = "blocked"
    elif host.get("aggregate", {}).get("alerted_node_count", 0) > 0:
        overall_status = "pass_with_resource_alerts"

    summary = {
        "generated_at": datetime.now(timezone.utc).astimezone().isoformat(),
        "run_id": args.run_id,
        "run_dir": args.run_dir,
        "snapshot_summary_path": args.snapshot_summary,
        "host_summary_path": args.host_summary,
        "traffic_summary_path": args.traffic_summary,
        "wasm_summary_paths": {
            "observer_local": args.observer_wasm_summary,
            "sequencer_ecs": args.sequencer_wasm_summary,
            "storage_ecs": args.storage_wasm_summary,
        },
        "snapshot": {
            "claim_status": snapshot.get("claim_status"),
            "failure_signatures": snapshot.get("failure_signatures") or [],
            "analysis": snapshot.get("analysis") or {},
        },
        "host": {
            "aggregate": host.get("aggregate") or {},
        },
        "traffic": {
            "aggregate": traffic.get("aggregate") or {},
        },
        "nodes": nodes,
        "overall": {
            "status": overall_status,
            "alerts": sorted(set(overall_alerts)),
        },
    }

    Path(args.summary_json).write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    Path(args.summary_md).write_text(
        "\n".join(render_markdown(summary)) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
