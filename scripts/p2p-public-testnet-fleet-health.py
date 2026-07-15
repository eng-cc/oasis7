#!/usr/bin/env python3
"""Collect one bounded public-testnet fleet-health evidence window."""

from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import Request, urlopen


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def parse_node(raw: str) -> tuple[str, str]:
    name, separator, url = raw.partition("=")
    if not separator or not name or not url:
        raise argparse.ArgumentTypeError("node must be NAME=URL")
    return name, url


def read_json(url: str) -> dict[str, Any]:
    request = Request(url, headers={"Accept": "application/json"})
    with urlopen(request, timeout=10) as response:  # noqa: S310 - operator-provided status URL
        payload = json.loads(response.read().decode("utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("status payload is not a JSON object")
    return payload


def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def node_gates(status: dict[str, Any], sequencer_consensus: dict[str, Any]) -> list[str]:
    gates: list[str] = []
    readiness = status.get("readiness")
    if status.get("running") is not True or not isinstance(readiness, dict) or readiness.get("status") != "ready":
        gates.append("node_not_ready")
    if readiness.get("failed_gates") if isinstance(readiness, dict) else None:
        gates.append("failed_gates_nonempty")
    if status.get("last_error") is not None:
        gates.append("last_error_present")

    consensus = status.get("consensus")
    if not isinstance(consensus, dict):
        return gates + ["head_mismatch", "network_head_not_ready"]
    for field in ("committed_height", "network_committed_height", "last_execution_height"):
        expected = sequencer_consensus.get(field)
        if expected is None or consensus.get(field) != expected:
            gates.append("head_mismatch")
            break
    network_head = consensus.get("network_head")
    if not isinstance(network_head, dict) or network_head.get("decision") != "ready":
        gates.append("network_head_not_ready")
    return gates


def main() -> int:
    parser = argparse.ArgumentParser(description="Collect bounded public-testnet fleet-health evidence.")
    parser.add_argument("--node", action="append", required=True, type=parse_node, metavar="NAME=URL")
    parser.add_argument("--sequencer", required=True, help="Name of the sequencer node supplied by --node.")
    parser.add_argument("--max-capture-span-seconds", required=True, type=float)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if args.max_capture_span_seconds < 0:
        parser.error("--max-capture-span-seconds must be non-negative")
    nodes = dict(args.node)
    if len(nodes) != len(args.node):
        parser.error("--node names must be unique")
    if args.sequencer not in nodes:
        parser.error("--sequencer must name one supplied --node")

    started_at = utc_now()
    started_monotonic = time.monotonic()
    captured: dict[str, dict[str, Any]] = {}
    failed_gates: list[str] = []
    for name, url in nodes.items():
        node_evidence: dict[str, Any] = {"url": url, "captured_at": utc_now()}
        try:
            node_evidence.update(read_json(url))
        except (OSError, URLError, ValueError, json.JSONDecodeError) as error:
            node_evidence["collection_error"] = str(error)
            failed_gates.append("collection_failed")
        captured[name] = node_evidence

    capture_span_seconds = time.monotonic() - started_monotonic
    finished_at = utc_now()
    if capture_span_seconds > args.max_capture_span_seconds:
        failed_gates.append("capture_span_exceeded")

    sequencer_consensus = captured[args.sequencer].get("consensus", {})
    if not isinstance(sequencer_consensus, dict):
        sequencer_consensus = {}
    for name, node_evidence in captured.items():
        if "collection_error" not in node_evidence:
            failed_gates.extend(node_gates(node_evidence, sequencer_consensus))
        elif name == args.sequencer:
            failed_gates.extend(["head_mismatch", "network_head_not_ready"])

    unique_gates = sorted(set(failed_gates))
    evidence = {
        "capture_finished_at": finished_at,
        "capture_span_seconds": capture_span_seconds,
        "capture_started_at": started_at,
        "failed_gates": unique_gates,
        "max_capture_span_seconds": args.max_capture_span_seconds,
        "nodes": captured,
        "sequencer": args.sequencer,
        "verdict": "ready" if not unique_gates else "blocked",
    }
    write_evidence(args.output, evidence)
    if unique_gates:
        print("fleet_health_blocked=" + ",".join(unique_gates), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
