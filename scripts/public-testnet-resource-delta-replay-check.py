#!/usr/bin/env python3
"""Validate public_testnet committed resource-delta replay evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


MANIFEST_SCHEMA = "oasis7.world_resource_manifest.v1"
DELTA_SCHEMA = "oasis7.world_resource_delta.v1"


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise SystemExit(f"{path}: expected JSON object")
    return value


def is_non_empty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def normalize_chunk_status(value: Any) -> str:
    return str(value or "").strip().lower()


def node_report(
    *,
    name: str,
    status_path: Path,
    snapshot_path: Path,
    expected_world_id: str,
    expected_chain_id: str,
) -> dict[str, Any]:
    status = load_json(status_path)
    snapshot = load_json(snapshot_path)
    consensus = status.get("consensus") or {}
    world_resource = status.get("world_resource") or {}
    manifest = (
        snapshot.get("chain_resource_manifest")
        or snapshot.get("latest_chain_resource_manifest")
        or {}
    )
    delta = snapshot.get("latest_chain_resource_delta") or {}
    generated_chunks = manifest.get("generated_chunks") or {}
    chunk_statuses = [
        normalize_chunk_status(chunk.get("chunk_status"))
        for chunk in generated_chunks.values()
        if isinstance(chunk, dict)
    ]
    provisional_statuses = {"provisional", "chainpending", "chain_pending"}
    checks = {
        "status_world_resource_ready": world_resource.get("readiness_status") == "ready",
        "status_failed_gates_empty": world_resource.get("failed_gates") == [],
        "status_manifest_schema_current": world_resource.get("schema_version") == MANIFEST_SCHEMA,
        "status_delta_schema_current": world_resource.get("delta_schema_version") == DELTA_SCHEMA,
        "status_world_id_matches": world_resource.get("world_id") == expected_world_id,
        "status_chain_id_matches": world_resource.get("chain_id") == expected_chain_id,
        "status_world_seed_present": bool(world_resource.get("world_seed")),
        "status_height_matches_consensus": world_resource.get("latest_resource_commit_height")
        == consensus.get("committed_height"),
        "status_hash_matches_consensus": world_resource.get("latest_resource_commit_hash")
        == consensus.get("last_block_hash"),
        "status_committed_chunks_present": (world_resource.get("committed_chunk_count") or 0) > 0,
        "status_no_provisional_chunks": world_resource.get("provisional_chunk_count") == 0,
        "status_no_pending_delta": world_resource.get("pending_delta_count") == 0,
        "manifest_schema_current": manifest.get("schema_version") == MANIFEST_SCHEMA,
        "manifest_world_id_matches": manifest.get("world_id") == expected_world_id,
        "manifest_chain_id_matches": manifest.get("chain_id") == expected_chain_id,
        "manifest_hash_matches_status": manifest.get("manifest_hash")
        == world_resource.get("seed_manifest_hash"),
        "manifest_committed_chunk_present": "committed" in chunk_statuses,
        "manifest_no_provisional_or_pending_chunks": not any(
            status in provisional_statuses for status in chunk_statuses
        ),
        "delta_present": bool(delta),
        "delta_schema_current": delta.get("schema_version") == DELTA_SCHEMA,
        "delta_world_id_matches": delta.get("world_id") == expected_world_id,
        "delta_chain_id_matches": delta.get("chain_id") == expected_chain_id,
        "delta_replay_committed": delta.get("replay_status") == "committed",
        "delta_base_hash_present": is_non_empty_string(delta.get("base_manifest_hash")),
        "delta_resulting_hash_matches_manifest": delta.get("resulting_manifest_hash")
        == manifest.get("manifest_hash"),
        "delta_height_matches_consensus": delta.get("block_height")
        == consensus.get("committed_height"),
        "delta_ordering_height_matches_consensus": (delta.get("ordering_key") or {}).get("height")
        == consensus.get("committed_height"),
        "delta_height_matches_status": delta.get("block_height")
        == world_resource.get("last_delta_commit_height"),
        "delta_id_matches_status": delta.get("delta_id") == world_resource.get("last_delta_id"),
        "delta_commit_hash_present": is_non_empty_string(delta.get("commit_block_hash")),
    }
    failures = [name for name, passed in checks.items() if not passed]
    return {
        "node": name,
        "pass": not failures,
        "failures": failures,
        "status_path": str(status_path),
        "snapshot_path": str(snapshot_path),
        "summary": {
            "world_id": world_resource.get("world_id"),
            "chain_id": world_resource.get("chain_id"),
            "height": consensus.get("committed_height"),
            "commit_hash": consensus.get("last_block_hash"),
            "delta_commit_hash": delta.get("commit_block_hash"),
            "manifest_hash": manifest.get("manifest_hash"),
            "delta_id": delta.get("delta_id"),
            "delta_replay_status": delta.get("replay_status"),
            "delta_entry_count": len(delta.get("entries") or []),
            "committed_chunk_count": world_resource.get("committed_chunk_count"),
            "provisional_chunk_count": world_resource.get("provisional_chunk_count"),
            "pending_delta_count": world_resource.get("pending_delta_count"),
            "failed_gates": world_resource.get("failed_gates"),
        },
        "checks": checks,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-world-id", required=True)
    parser.add_argument("--expected-chain-id", required=True)
    parser.add_argument(
        "--node",
        action="append",
        nargs=3,
        metavar=("NAME", "STATUS_JSON", "SNAPSHOT_JSON"),
        required=True,
        help="node name plus live status JSON and execution-world resource JSON",
    )
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    nodes = [
        node_report(
            name=name,
            status_path=Path(status_path),
            snapshot_path=Path(snapshot_path),
            expected_world_id=args.expected_world_id,
            expected_chain_id=args.expected_chain_id,
        )
        for name, status_path, snapshot_path in args.node
    ]
    report = {
        "pass": all(node["pass"] for node in nodes),
        "expected_world_id": args.expected_world_id,
        "expected_chain_id": args.expected_chain_id,
        "nodes": nodes,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
