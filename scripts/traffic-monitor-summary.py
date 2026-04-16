#!/usr/bin/env python3
import argparse
import json
from collections import deque
from datetime import datetime, timezone
from pathlib import Path


def parse_args():
    parser = argparse.ArgumentParser(
        description="Summarize Oasis7 traffic-monitor history with bounded retention."
    )
    parser.add_argument(
        "--layout",
        choices=("single-node", "triad"),
        required=True,
        help="summary layout to produce",
    )
    parser.add_argument("--history-path", required=True)
    parser.add_argument("--summary-json", required=True)
    parser.add_argument("--summary-md", required=True)
    parser.add_argument("--window-minutes", type=int, required=True)
    parser.add_argument("--history-retention-minutes", type=int, required=True)
    parser.add_argument("--top-n", type=int, required=True)
    parser.add_argument("--run-id")
    parser.add_argument("--run-dir")
    parser.add_argument("--label", action="append", default=[])
    return parser.parse_args()


def record_ts_ms(record):
    value = record.get("captured_at_unix_ms")
    if value is None:
        return None
    return int(value)


def history_cutoff_ms(latest_ts_ms, retention_minutes):
    return latest_ts_ms - retention_minutes * 60 * 1000


def prune_and_load_records(path, retention_minutes):
    records = deque()
    latest_ts_ms = None
    total_records = 0

    with path.open("r", encoding="utf-8") as handle:
        for raw_line in handle:
            line = raw_line.strip()
            if not line:
                continue
            record = json.loads(line)
            total_records += 1
            ts_ms = record_ts_ms(record)
            if ts_ms is None:
                records.append(record)
                continue
            if latest_ts_ms is None or ts_ms > latest_ts_ms:
                latest_ts_ms = ts_ms
            records.append(record)
            cutoff_ms = history_cutoff_ms(latest_ts_ms, retention_minutes)
            while records:
                oldest_ts_ms = record_ts_ms(records[0])
                if oldest_ts_ms is None or oldest_ts_ms >= cutoff_ms:
                    break
                records.popleft()

    pruned_records = list(records)
    if len(pruned_records) != total_records:
        temp_path = path.with_name(path.name + ".tmp")
        with temp_path.open("w", encoding="utf-8") as handle:
            for record in pruned_records:
                handle.write(json.dumps(record, ensure_ascii=False))
                handle.write("\n")
        temp_path.replace(path)
    return pruned_records, total_records


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
        result[name] = delta_lane_entry(
            current_map.get(name), baseline_map.get(name), counter_key
        )
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
        "scope": current_lane.get("scope"),
        "observed_since_unix_ms": current_lane.get("observed_since_unix_ms"),
        "counter_key": counter_key,
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


def summarize_record_set(records, window_minutes, top_n):
    successful = [record for record in records if record.get("status_fetch_ok") is True]
    if not successful:
        latest_any = records[-1] if records else None
        return {
            "available": False,
            "sample_count_total": len(records),
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
    return {
        "available": True,
        "sample_count_total": len(records),
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
            "full_window_covered": coverage_minutes >= max(0.0, window_minutes - 0.01),
        },
        "latest": {
            "node_label": latest.get("node_label"),
            "node_id": latest.get("node_id"),
            "world_id": latest.get("world_id"),
            "role": latest.get("role"),
            "running": latest.get("running"),
            "status_url": latest.get("status_url"),
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


def render_traffic_totals(name, lane):
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


def render_single_node_markdown(summary, history_path, generated_at):
    lines = [
        "# Oasis7 Node Traffic Monitor Summary",
        "",
        f"- Generated at: `{generated_at}`",
        f"- History file: `{history_path}`",
        f"- History retention: `{summary['history_retention_minutes']}` minutes",
        f"- History record count after prune: `{summary['history_record_count']}`",
        f"- History records pruned this run: `{summary['history_pruned_count']}`",
        f"- Requested window: `{summary['window_minutes_requested']}` minutes",
        f"- Top contributors per map: `{summary['top_n']}`",
        "",
    ]

    node = summary["node"]
    if not node.get("available"):
        lines.append(
            f"- No successful samples yet. latest_fetch_error=`{node.get('latest_fetch_error')}`"
        )
        return lines

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

    lines.append(render_traffic_totals("UDP gossip", udp))
    if udp.get("available"):
        lines.extend(render_top_block("Top UDP kinds", udp.get("top_kinds"), "datagrams"))

    lines.append(render_traffic_totals("Libp2p replication", libp2p))
    if libp2p.get("available"):
        lines.append(
            "- Libp2p lanes: "
            + f"gossip +{fmt_bytes(libp2p['gossip']['inbound']['payload_bytes'] + libp2p['gossip']['outbound']['payload_bytes'])}, "
            + f"request +{fmt_bytes(libp2p['request']['inbound']['payload_bytes'] + libp2p['request']['outbound']['payload_bytes'])}, "
            + f"response +{fmt_bytes(libp2p['response']['inbound']['payload_bytes'] + libp2p['response']['outbound']['payload_bytes'])}"
        )
        lines.extend(
            render_top_block(
                "Top libp2p protocols", libp2p.get("top_protocols"), "messages"
            )
        )
        lines.extend(
            render_top_block("Top libp2p topics", libp2p.get("top_topics"), "messages")
        )
    return lines


def render_triad_markdown(summary, history_path, generated_at, labels):
    lines = [
        "# P2P Real Env Traffic Monitor Summary",
        "",
        f"- Generated at: `{generated_at}`",
        f"- History file: `{history_path}`",
        f"- History retention: `{summary['history_retention_minutes']}` minutes",
        f"- History record count after prune: `{summary['history_record_count']}`",
        f"- History records pruned this run: `{summary['history_pruned_count']}`",
        f"- Requested window: `{summary['window_minutes_requested']}` minutes",
        f"- Top contributors per map: `{summary['top_n']}`",
        "",
    ]

    for label in labels:
        node = summary["nodes"][label]
        lines.append(f"## {label}")
        if not node.get("available"):
            lines.append(
                f"- No successful samples yet. latest_fetch_error=`{node.get('latest_fetch_error')}`"
            )
            lines.append("")
            continue

        window = node["window"]
        latest = node["latest"]
        consensus = node["consensus"]
        errors = node["recent_errors"]
        lines.extend(
            [
                f"- Node: `{latest.get('node_id')}` role=`{latest.get('role')}` running=`{latest.get('running')}`",
                f"- Window coverage: `{window['covered_minutes']}` / `{window['requested_minutes']}` minutes across `{window['compatible_sample_count']}` compatible samples",
                f"- Baseline sample: `{window['baseline_captured_at']}` | Latest sample: `{window['latest_captured_at']}`",
                f"- Restart/counter reset inside requested window: `{window['restart_or_counter_reset_detected_within_window']}`",
                f"- Height delta: committed `+{fmt_num(consensus['committed_height']['delta'])}` (`{fmt_num(consensus['committed_height']['baseline'])}` -> `{fmt_num(consensus['committed_height']['latest'])}`), network `+{fmt_num(consensus['network_committed_height']['delta'])}` (`{fmt_num(consensus['network_committed_height']['baseline'])}` -> `{fmt_num(consensus['network_committed_height']['latest'])}`)",
                f"- Known peer heads: `{fmt_num(consensus['known_peer_heads']['baseline'])}` -> `{fmt_num(consensus['known_peer_heads']['latest'])}`",
                f"- Recent errors: storage `+{fmt_num(errors['storage_recent_errors_count']['delta'])}` (`{fmt_num(errors['storage_recent_errors_count']['baseline'])}` -> `{fmt_num(errors['storage_recent_errors_count']['latest'])}`), reward runtime `+{fmt_num(errors['reward_runtime_recent_errors_count']['delta'])}` (`{fmt_num(errors['reward_runtime_recent_errors_count']['baseline'])}` -> `{fmt_num(errors['reward_runtime_recent_errors_count']['latest'])}`)",
                f"- Last error: `{latest.get('last_error')}`",
                render_traffic_totals("UDP gossip", node["traffic"]["udp_gossip"]),
                render_traffic_totals(
                    "Libp2p replication", node["traffic"]["libp2p_replication"]
                ),
            ]
        )

        udp = node["traffic"]["udp_gossip"]
        if udp.get("available"):
            lines.extend(render_top_block("Top UDP kinds", udp.get("top_kinds"), "datagrams"))

        libp2p = node["traffic"]["libp2p_replication"]
        if libp2p.get("available"):
            lines.append(
                "- Libp2p lanes: "
                + f"gossip +{fmt_bytes(libp2p['gossip']['inbound']['payload_bytes'] + libp2p['gossip']['outbound']['payload_bytes'])}, "
                + f"request +{fmt_bytes(libp2p['request']['inbound']['payload_bytes'] + libp2p['request']['outbound']['payload_bytes'])}, "
                + f"response +{fmt_bytes(libp2p['response']['inbound']['payload_bytes'] + libp2p['response']['outbound']['payload_bytes'])}"
            )
            lines.extend(
                render_top_block(
                    "Top libp2p protocols", libp2p.get("top_protocols"), "messages"
                )
            )
            lines.extend(
                render_top_block("Top libp2p topics", libp2p.get("top_topics"), "messages")
            )
        lines.append("")
    return lines


def main():
    args = parse_args()
    history_path = Path(args.history_path)
    summary_json_path = Path(args.summary_json)
    summary_md_path = Path(args.summary_md)
    generated_at = datetime.now(tz=timezone.utc).isoformat()

    records, total_records = prune_and_load_records(
        history_path, args.history_retention_minutes
    )
    history_pruned_count = max(0, total_records - len(records))

    common = {
        "ok": True,
        "generated_at": generated_at,
        "history_path": str(history_path),
        "history_retention_minutes": args.history_retention_minutes,
        "history_record_count_before_prune": total_records,
        "history_record_count": len(records),
        "history_pruned_count": history_pruned_count,
        "window_minutes_requested": args.window_minutes,
        "top_n": args.top_n,
    }

    if args.layout == "single-node":
        summary = dict(common)
        summary["node"] = summarize_record_set(records, args.window_minutes, args.top_n)
        markdown_lines = render_single_node_markdown(summary, history_path, generated_at)
    else:
        labels = args.label or ["observer_local", "sequencer_ecs", "storage_ecs"]
        records_by_label = {label: [] for label in labels}
        for record in records:
            label = record.get("label")
            if label in records_by_label:
                records_by_label[label].append(record)

        summary = dict(common)
        summary["run_id"] = args.run_id
        summary["run_dir"] = args.run_dir
        summary["nodes"] = {}
        for label in labels:
            summary["nodes"][label] = summarize_record_set(
                records_by_label[label], args.window_minutes, args.top_n
            )
            summary["nodes"][label]["label"] = label
        markdown_lines = render_triad_markdown(
            summary, history_path, generated_at, labels
        )

    summary_json_path.write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    summary_md_path.write_text("\n".join(markdown_lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
