#!/usr/bin/env python3
"""Collect one bounded public-testnet fleet-health evidence window."""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import Request, urlopen


MANAGED_FIVE_NODE_NAMES = frozenset(
    {
        "sequencer",
        "storage",
        "linux-lan-observer",
        "windows-observer",
        "macos-observer",
    }
)
MANAGED_FIVE_NODE_SEQUENCER = "sequencer"
MANAGED_FIVE_NODE_STORAGE = "storage"
MIN_CHECKPOINT_SCHEMA_VERSION = 2
MAX_PROVIDER_CHECKPOINT_HEIGHT_DELTA = 1
MANAGED_TRIAD_NODE_IDS = {
    "sequencer-204": "triad-testnet-sequencer",
    "storage-205": "triad-testnet-storage",
    "validator-47": "triad-testnet-validator-47",
}
MANAGED_TRIAD_NAMES = frozenset(MANAGED_TRIAD_NODE_IDS)
MANAGED_TRIAD_SEQUENCER = "sequencer-204"
MANAGED_TRIAD_TOTAL_STAKE = 300
MANAGED_TRIAD_REQUIRED_STAKE = 200
MANAGED_TRIAD_QUORUM = {"numerator": 2, "denominator": 3}
MANAGED_TRIAD_GOVERNANCE = {
    "signer_count": 3,
    "threshold": 2,
    "threshold_bps": 6667,
}


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


def provider_checkpoint(status: dict[str, Any]) -> tuple[int, str, int, str] | None:
    chain_proof = status.get("chain_proof")
    if not isinstance(chain_proof, dict):
        return None
    checkpoint = chain_proof.get("latest_execution_checkpoint")
    if not isinstance(checkpoint, dict):
        return None
    schema_version = checkpoint.get("schema_version")
    checkpoint_id = checkpoint.get("checkpoint_id")
    height = checkpoint.get("height")
    manifest_hash = checkpoint.get("manifest_hash")
    if (
        isinstance(schema_version, bool)
        or not isinstance(schema_version, int)
        or isinstance(height, bool)
        or not isinstance(height, int)
        or not isinstance(checkpoint_id, str)
        or not isinstance(manifest_hash, str)
    ):
        return None
    return schema_version, checkpoint_id, height, manifest_hash


def provider_checkpoint_gates(captured: dict[str, dict[str, Any]]) -> list[str]:
    checkpoints: dict[str, tuple[int, str, int, str]] = {}
    gates: list[str] = []
    for name in (MANAGED_FIVE_NODE_SEQUENCER, MANAGED_FIVE_NODE_STORAGE):
        checkpoint = provider_checkpoint(captured[name])
        if checkpoint is None:
            gates.append("provider_checkpoint_missing")
            continue
        schema_version, checkpoint_id, height, manifest_hash = checkpoint
        if schema_version < MIN_CHECKPOINT_SCHEMA_VERSION:
            gates.append("provider_checkpoint_schema_invalid")
        if not checkpoint_id.strip() or not re.fullmatch(r"[0-9a-fA-F]{64}", manifest_hash):
            gates.append("provider_checkpoint_identity_invalid")
        if height <= 0:
            gates.append("provider_checkpoint_height_invalid")
        checkpoints[name] = checkpoint
    if len(checkpoints) == 2:
        _, sequencer_id, sequencer_height, sequencer_hash = checkpoints[MANAGED_FIVE_NODE_SEQUENCER]
        _, storage_id, storage_height, storage_hash = checkpoints[MANAGED_FIVE_NODE_STORAGE]
        if sequencer_id != storage_id or sequencer_hash.lower() != storage_hash.lower():
            gates.append("provider_checkpoint_identity_mismatch")
        if abs(sequencer_height - storage_height) > MAX_PROVIDER_CHECKPOINT_HEIGHT_DELTA:
            gates.append("provider_checkpoint_height_incompatible")
    return gates


def triad_validator_gates(name: str, status: dict[str, Any]) -> list[str]:
    """Require an active, canonical validator identity for managed triad mode."""
    gates: list[str] = []
    expected_node_id = MANAGED_TRIAD_NODE_IDS[name]
    if status.get("node_id") != expected_node_id:
        gates.append("validator_identity_mismatch")

    validator = status.get("validator")
    if not isinstance(validator, dict):
        return gates + ["validator_metadata_missing"]
    if validator.get("role") != "validator":
        gates.append("validator_role_invalid")
    if validator.get("membership") != "active":
        gates.append("validator_membership_invalid")
    stake = validator.get("stake")
    if isinstance(stake, bool) or not isinstance(stake, int) or stake != 100:
        gates.append("validator_stake_mismatch")
    if validator.get("signer_binding") != expected_node_id:
        gates.append("validator_signer_binding_mismatch")
    return gates


def triad_provider_gates(status: dict[str, Any]) -> list[str]:
    """Require validator-47 to close both checkpoint and full-storage roles."""
    provider = status.get("provider")
    if not isinstance(provider, dict):
        return ["validator_47_provider_metadata_missing"]
    gates: list[str] = []
    if provider.get("checkpoint") is not True:
        gates.append("validator_47_provider_checkpoint_missing")
    if provider.get("full_storage") is not True:
        gates.append("validator_47_provider_full_storage_missing")
    return gates


def triad_validator_set(captured: dict[str, dict[str, Any]]) -> dict[str, Any]:
    """Return the governed three-equal-validator projection in canonical order."""
    stakes = [
        captured[name].get("validator", {}).get("stake")
        if isinstance(captured[name].get("validator"), dict)
        else None
        for name in ("sequencer-204", "storage-205", "validator-47")
    ]
    return {
        "nodes": ["sequencer-204", "storage-205", "validator-47"],
        "stakes": stakes,
        "total_stake": MANAGED_TRIAD_TOTAL_STAKE,
        "required_stake": MANAGED_TRIAD_REQUIRED_STAKE,
        "quorum": MANAGED_TRIAD_QUORUM,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Collect bounded public-testnet fleet-health evidence.")
    parser.add_argument("--node", action="append", required=True, type=parse_node, metavar="NAME=URL")
    parser.add_argument("--sequencer", required=True, help="Name of the sequencer node supplied by --node.")
    parser.add_argument(
        "--managed-five-node",
        action="store_true",
        help=(
            "Require exactly the current managed public-testnet fleet: sequencer, storage, "
            "linux-lan-observer, windows-observer, and macos-observer."
        ),
    )
    parser.add_argument(
        "--managed-triad",
        action="store_true",
        help=(
            "Require exactly sequencer-204, storage-205, and validator-47 "
            "with three equal validators and validator-47 provider closure."
        ),
    )
    parser.add_argument("--max-capture-span-seconds", required=True, type=float)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if not math.isfinite(args.max_capture_span_seconds) or args.max_capture_span_seconds < 0:
        parser.error("--max-capture-span-seconds must be finite and non-negative")
    nodes = dict(args.node)
    if len(nodes) != len(args.node):
        if args.managed_triad:
            parser.error("managed triad closure requires each canonical node exactly once; duplicate --node name")
        if args.managed_five_node:
            parser.error("managed five-node closure requires each canonical node exactly once; duplicate --node name")
        parser.error("--node names must be unique")
    if args.sequencer not in nodes:
        parser.error("--sequencer must name one supplied --node")
    if args.managed_triad and args.managed_five_node:
        parser.error("managed triad and managed five-node closures are mutually exclusive")
    if args.managed_triad:
        supplied_names = frozenset(nodes)
        if supplied_names != MANAGED_TRIAD_NAMES:
            missing = sorted(MANAGED_TRIAD_NAMES - supplied_names)
            unknown = sorted(supplied_names - MANAGED_TRIAD_NAMES)
            details = []
            if missing:
                details.append("missing=" + ",".join(missing))
            if unknown:
                details.append("unknown=" + ",".join(unknown))
            parser.error(
                "managed triad closure requires exactly the canonical validator identities "
                "(" + "; ".join(details) + ")"
            )
        if args.sequencer != MANAGED_TRIAD_SEQUENCER:
            parser.error("managed triad closure requires --sequencer sequencer-204")
    elif frozenset(nodes) == MANAGED_TRIAD_NAMES:
        parser.error(
            "validator-aware --managed-triad is required for the canonical three-validator triad"
        )
    if args.managed_five_node:
        supplied_names = frozenset(nodes)
        if supplied_names != MANAGED_FIVE_NODE_NAMES:
            missing = sorted(MANAGED_FIVE_NODE_NAMES - supplied_names)
            unknown = sorted(supplied_names - MANAGED_FIVE_NODE_NAMES)
            details = []
            if missing:
                details.append("missing=" + ",".join(missing))
            if unknown:
                details.append("unknown=" + ",".join(unknown))
            parser.error(
                "managed five-node closure requires exactly the canonical node identities "
                "(" + "; ".join(details) + ")"
            )
        if args.sequencer != MANAGED_FIVE_NODE_SEQUENCER:
            parser.error("managed five-node closure requires --sequencer sequencer")

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
    if args.managed_triad:
        for name in MANAGED_TRIAD_NAMES:
            node_evidence = captured[name]
            if "collection_error" not in node_evidence:
                failed_gates.extend(triad_validator_gates(name, node_evidence))
        validator_47 = captured["validator-47"]
        if "collection_error" not in validator_47:
            failed_gates.extend(triad_provider_gates(validator_47))
    if args.managed_five_node:
        failed_gates.extend(provider_checkpoint_gates(captured))

    unique_gates = sorted(set(failed_gates))
    if args.managed_triad and any(gate.startswith("validator_47_provider_") for gate in unique_gates):
        print("fleet_health_blocked=" + ",".join(unique_gates), file=sys.stderr)
        return 1
    evidence = {
        "capture_finished_at": finished_at,
        "capture_span_seconds": capture_span_seconds,
        "capture_started_at": started_at,
        "failed_gates": unique_gates,
        "max_capture_span_seconds": args.max_capture_span_seconds,
        "nodes": captured,
        "sequencer": args.sequencer,
        "scope": (
            "managed_triad"
            if args.managed_triad
            else "managed_five_node"
            if args.managed_five_node
            else "generic"
        ),
        "verdict": "ready" if not unique_gates else "blocked",
    }
    if args.managed_triad:
        evidence["claim_mode"] = "three_equal_validator"
        evidence["validator_set"] = triad_validator_set(captured)
        evidence["governance"] = MANAGED_TRIAD_GOVERNANCE
    write_evidence(args.output, evidence)
    if unique_gates:
        print("fleet_health_blocked=" + ",".join(unique_gates), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
