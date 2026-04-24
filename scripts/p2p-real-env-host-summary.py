#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

NODE_LABELS = ("observer_local", "sequencer_ecs", "storage_ecs")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Summarize triad host/process monitoring samples."
    )
    parser.add_argument("--history-path", required=True)
    parser.add_argument("--summary-json", required=True)
    parser.add_argument("--summary-md", required=True)
    parser.add_argument("--run-id")
    parser.add_argument("--run-dir")
    return parser.parse_args()


def load_records(path: Path) -> list[dict]:
    records: list[dict] = []
    if not path.is_file():
        return records
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        records.append(json.loads(line))
    return records


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


def to_float(value):
    if value in (None, ""):
        return None
    return float(value)


def to_int(value):
    if value in (None, ""):
        return None
    return int(value)


def ratio_percent(part, total):
    if part in (None, "") or total in (None, ""):
        return None
    total_value = float(total)
    if total_value <= 0:
        return None
    return round(float(part) * 100.0 / total_value, 2)


def summarize_node(records: list[dict]) -> dict:
    if not records:
        return {"available": False, "sample_count": 0}

    latest = max(records, key=lambda item: int(item.get("captured_at_unix_ms") or 0))
    latest_host = latest.get("host") or {}
    latest_runtime = latest.get("runtime") or {}
    cpu_cores = to_int(latest_host.get("cpu_cores")) or 0

    runtime_cpu_values = [
        to_float((record.get("runtime") or {}).get("pcpu"))
        for record in records
        if (record.get("runtime") or {}).get("present") is True
    ]
    runtime_mem_values = [
        to_float((record.get("runtime") or {}).get("pmem"))
        for record in records
        if (record.get("runtime") or {}).get("present") is True
    ]
    load1_values = [to_float((record.get("host") or {}).get("loadavg_1m")) for record in records]
    load_ratio_values = [
        (load / cpu_cores) if load is not None and cpu_cores > 0 else None
        for load in load1_values
    ]
    mem_available_values = [
        ratio_percent(
            (record.get("host") or {}).get("mem_available_bytes"),
            (record.get("host") or {}).get("mem_total_bytes"),
        )
        for record in records
    ]
    storage_used_values = [
        to_float((record.get("storage") or {}).get("used_percent")) for record in records
    ]

    latest_runtime_cpu = to_float(latest_runtime.get("pcpu"))
    latest_runtime_cpu_ratio = (
        round(latest_runtime_cpu / (cpu_cores * 100.0), 4)
        if latest_runtime_cpu is not None and cpu_cores > 0
        else None
    )
    latest_load1 = to_float(latest_host.get("loadavg_1m"))
    latest_load_ratio = round(latest_load1 / cpu_cores, 4) if latest_load1 is not None and cpu_cores > 0 else None
    latest_mem_available_percent = ratio_percent(
        latest_host.get("mem_available_bytes"),
        latest_host.get("mem_total_bytes"),
    )
    latest_storage_used_percent = to_float((latest.get("storage") or {}).get("used_percent"))

    alerts = []
    runtime_cpu_status = "unavailable"
    if latest_runtime.get("present") is True and latest_runtime_cpu_ratio is not None:
        if latest_runtime_cpu_ratio >= 0.75:
            runtime_cpu_status = "hot"
            alerts.append("runtime_cpu_hot")
        elif latest_runtime_cpu_ratio >= 0.50:
            runtime_cpu_status = "elevated"
        else:
            runtime_cpu_status = "normal"
    elif latest_runtime.get("present") is not True:
        alerts.append("runtime_process_missing")

    load_status = "unavailable"
    if latest_load_ratio is not None:
        if latest_load_ratio >= 1.0:
            load_status = "hot"
            alerts.append("host_load_hot")
        elif latest_load_ratio >= 0.75:
            load_status = "elevated"
        else:
            load_status = "normal"

    memory_status = "unavailable"
    if latest_mem_available_percent is not None:
        if latest_mem_available_percent < 15.0:
            memory_status = "low"
            alerts.append("memory_available_low")
        elif latest_mem_available_percent < 25.0:
            memory_status = "tight"
        else:
            memory_status = "normal"

    storage_status = "unavailable"
    if latest_storage_used_percent is not None:
        if latest_storage_used_percent >= 90.0:
            storage_status = "critical"
            alerts.append("storage_usage_critical")
        elif latest_storage_used_percent >= 85.0:
            storage_status = "high"
            alerts.append("storage_usage_high")
        else:
            storage_status = "normal"

    service_state = latest.get("service") or {}
    if service_state.get("active_state") != "active" or service_state.get("sub_state") != "running":
        alerts.append("service_not_running")

    return {
        "available": True,
        "sample_count": len(records),
        "latest": {
            "captured_at": latest.get("captured_at"),
            "hostname": latest_host.get("hostname"),
            "cpu_cores": cpu_cores,
            "loadavg_1m": latest_load1,
            "loadavg_5m": to_float(latest_host.get("loadavg_5m")),
            "loadavg_15m": to_float(latest_host.get("loadavg_15m")),
            "load_per_core_ratio_1m": latest_load_ratio,
            "mem_total_bytes": to_int(latest_host.get("mem_total_bytes")),
            "mem_available_bytes": to_int(latest_host.get("mem_available_bytes")),
            "mem_available_percent": latest_mem_available_percent,
            "storage": latest.get("storage"),
            "service": service_state,
            "wrapper": latest.get("wrapper"),
            "runtime": latest_runtime,
            "runtime_cpu_percent": latest_runtime_cpu,
            "runtime_cpu_core_ratio": latest_runtime_cpu_ratio,
        },
        "peaks": {
            "max_runtime_cpu_percent": max((value for value in runtime_cpu_values if value is not None), default=None),
            "max_runtime_cpu_core_ratio": max(
                (
                    round(value / (cpu_cores * 100.0), 4)
                    for value in runtime_cpu_values
                    if value is not None and cpu_cores > 0
                ),
                default=None,
            ),
            "max_runtime_mem_percent": max((value for value in runtime_mem_values if value is not None), default=None),
            "max_loadavg_1m": max((value for value in load1_values if value is not None), default=None),
            "max_load_per_core_ratio_1m": max((value for value in load_ratio_values if value is not None), default=None),
            "min_mem_available_percent": min(
                (value for value in mem_available_values if value is not None),
                default=None,
            ),
            "max_storage_used_percent": max(
                (value for value in storage_used_values if value is not None),
                default=None,
            ),
        },
        "status": {
            "runtime_cpu": runtime_cpu_status,
            "host_load": load_status,
            "memory": memory_status,
            "storage": storage_status,
        },
        "alerts": sorted(set(alerts)),
    }


def render_markdown(summary: dict, history_path: str) -> list[str]:
    lines = [
        "# P2P Real Env Host Monitor Summary",
        "",
        f"- Generated at: `{summary['generated_at']}`",
        f"- History path: `{history_path}`",
        f"- Run id: `{summary.get('run_id') or 'n/a'}`",
        f"- Nodes with data: `{summary['aggregate']['node_count']}`",
        f"- Nodes with alerts: `{summary['aggregate']['alerted_node_count']}`",
        f"- Highest runtime CPU node: `{summary['aggregate']['highest_runtime_cpu_node'] or 'n/a'}`",
        "",
    ]
    for label in NODE_LABELS:
        node = summary["nodes"].get(label) or {"available": False}
        lines.append(f"## {label}")
        if node.get("available") is not True:
            lines.append("- unavailable")
            lines.append("")
            continue
        latest = node["latest"]
        peaks = node["peaks"]
        status = node["status"]
        storage = latest.get("storage") or {}
        lines.extend(
            [
                f"- Host: `{latest.get('hostname')}` cores=`{fmt_num(latest.get('cpu_cores'))}`",
                f"- Runtime CPU: latest `{fmt_percent(latest.get('runtime_cpu_percent'))}` core-ratio `{fmt_percent((latest.get('runtime_cpu_core_ratio') or 0) * 100) if latest.get('runtime_cpu_core_ratio') is not None else 'n/a'}`; peak `{fmt_percent(peaks.get('max_runtime_cpu_percent'))}`",
                f"- Loadavg 1m: latest `{fmt_num(latest.get('loadavg_1m'))}` ratio/core `{fmt_num(latest.get('load_per_core_ratio_1m'))}`; peak `{fmt_num(peaks.get('max_loadavg_1m'))}`",
                f"- Memory available: `{fmt_bytes(latest.get('mem_available_bytes'))}` / `{fmt_bytes(latest.get('mem_total_bytes'))}` ({fmt_percent(latest.get('mem_available_percent'))}); floor `{fmt_percent(peaks.get('min_mem_available_percent'))}`",
                f"- Storage `{storage.get('path') or 'n/a'}`: used `{fmt_percent(storage.get('used_percent'))}` (`{fmt_bytes(storage.get('used_bytes'))}` / `{fmt_bytes(storage.get('total_bytes'))}`)",
                f"- Service: active=`{(latest.get('service') or {}).get('active_state')}` sub=`{(latest.get('service') or {}).get('sub_state')}` main_pid=`{(latest.get('service') or {}).get('main_pid')}` runtime_pid=`{(latest.get('runtime') or {}).get('pid')}`",
                f"- Health status: runtime_cpu=`{status.get('runtime_cpu')}` load=`{status.get('host_load')}` memory=`{status.get('memory')}` storage=`{status.get('storage')}`",
                f"- Alerts: `{', '.join(node.get('alerts') or ['(none)'])}`",
                "",
            ]
        )
    return lines


def main():
    args = parse_args()
    history_path = Path(args.history_path)
    records = load_records(history_path)

    grouped: dict[str, list[dict]] = {label: [] for label in NODE_LABELS}
    for record in records:
        label = record.get("label")
        if label in grouped:
            grouped[label].append(record)

    node_summaries = {label: summarize_node(grouped[label]) for label in NODE_LABELS}
    alerted_nodes = [label for label, node in node_summaries.items() if node.get("alerts")]

    highest_runtime_cpu_node = None
    highest_runtime_cpu_value = -1.0
    for label, node in node_summaries.items():
        value = node.get("latest", {}).get("runtime_cpu_percent")
        if value is None:
            continue
        if value > highest_runtime_cpu_value:
            highest_runtime_cpu_value = value
            highest_runtime_cpu_node = label

    summary = {
        "generated_at": datetime.now(timezone.utc).astimezone().isoformat(),
        "run_id": args.run_id,
        "run_dir": args.run_dir,
        "history_path": str(history_path),
        "history_record_count": len(records),
        "nodes": node_summaries,
        "aggregate": {
            "node_count": len([node for node in node_summaries.values() if node.get("available") is True]),
            "alerted_node_count": len(alerted_nodes),
            "alerted_nodes": alerted_nodes,
            "highest_runtime_cpu_node": highest_runtime_cpu_node,
            "highest_runtime_cpu_percent": None if highest_runtime_cpu_value < 0 else highest_runtime_cpu_value,
        },
    }

    Path(args.summary_json).write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    Path(args.summary_md).write_text(
        "\n".join(render_markdown(summary, str(history_path))) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
