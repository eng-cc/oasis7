#!/usr/bin/env python3
"""Aggregate five independently retained identity-v2 evidence maps.

Each input is the one-node map produced by the identity-v2 receipt sidecar.
The aggregator only joins already retained bytes: it never copies, rewrites,
or deletes an input artifact.  The full-network planner remains the canonical
validator for the resulting five-node map, including all artifact bindings.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
from typing import Any, NoReturn, Sequence


ROOT = Path(__file__).resolve().parents[1]
PLANNER_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.py"


def _load_planner():
    spec = importlib.util.spec_from_file_location("identity_v2_aggregate_planner", PLANNER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load canonical planner: {PLANNER_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PLANNER = _load_planner()
NODE_ORDER = tuple(PLANNER.NODE_ORDER)
IDENTITY_V2_EVIDENCE_SCHEMA = PLANNER.IDENTITY_V2_EVIDENCE_SCHEMA
CANONICAL_NETWORK_ID = PLANNER.CANONICAL_NETWORK_ID
ARTIFACT_FIELDS = tuple(sorted(PLANNER.IDENTITY_V2_EVIDENCE_ARTIFACT_FIELDS))
MAP_FIELDS = {"schema_version", "network_id", "task_uid", "head_oid", "context", "plan_intent", "entries"}
ENTRY_FIELDS = {"node_name", "node_id", "peer_id", *PLANNER.IDENTITY_V2_EVIDENCE_ARTIFACT_FIELDS}


def die(message: str) -> NoReturn:
    raise SystemExit(f"error: identity-v2 evidence aggregate: {message}")


def _secure_map_descriptor(path: Path, label: str) -> tuple[Path, bytes]:
    """Use the planner's exact secure descriptor checks for an input map."""
    try:
        payload = path.read_bytes()
    except OSError as error:
        die(f"{label} is unreadable: {error.__class__.__name__}")
    descriptor = {"path": str(path), "sha256": hashlib.sha256(payload).hexdigest(), "size_bytes": len(payload)}
    try:
        return PLANNER._evidence_descriptor(descriptor, label)
    except SystemExit as error:
        die(str(error))


def _canonical_descriptor(value: Any, label: str) -> dict[str, Any]:
    """Validate a retained artifact and normalize only digest spelling."""
    try:
        path, payload = PLANNER._evidence_descriptor(value, label)
    except SystemExit as error:
        die(str(error))
    return {
        "path": str(path),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "size_bytes": len(payload),
    }


def _read_input_map(path: Path, index: int) -> dict[str, Any]:
    path = path.absolute()
    if path.is_symlink() or not path.is_file():
        die(f"input map {index} is not a regular non-symlink file")
    _, payload = _secure_map_descriptor(path, f"input map {index}")
    try:
        value = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        die(f"input map {index} JSON is malformed: {error.__class__.__name__}")
    if not isinstance(value, dict) or set(value) != MAP_FIELDS:
        die(f"input map {index} fields are not exact")
    if value.get("schema_version") != IDENTITY_V2_EVIDENCE_SCHEMA:
        die(f"input map {index} schema is unsupported")
    if value.get("network_id") != CANONICAL_NETWORK_ID:
        die(f"input map {index} network is not the governed deployment network")
    task_uid = value.get("task_uid")
    if not isinstance(task_uid, str) or not task_uid.strip():
        die(f"input map {index} task_uid is malformed")
    try:
        head_oid = PLANNER.require_oid(value.get("head_oid"), f"input map {index}.head_oid")
    except SystemExit as error:
        die(str(error))
    value["head_oid"] = head_oid
    context_descriptor = _canonical_descriptor(value.get("context"), f"input map {index} context")
    intent_descriptor = _canonical_descriptor(value.get("plan_intent"), f"input map {index} plan intent")
    entries = value.get("entries")
    if not isinstance(entries, list) or len(entries) != 1:
        die(f"input map {index} must contain exactly one node entry")
    entry = entries[0]
    if not isinstance(entry, dict) or set(entry) != ENTRY_FIELDS:
        die(f"input map {index} node entry fields are not exact")
    name = entry.get("node_name")
    if not isinstance(name, str) or name not in NODE_ORDER:
        die(f"input map {index} node is unexpected")
    expected = PLANNER.EXPECTED_NODES[name]
    if entry.get("node_id") != expected["node_id"] or entry.get("peer_id") != PLANNER.CANONICAL_PEER_REGISTRY[name]:
        die(f"input map {index} {name} identity binding does not match the canonical registry")
    normalized_entry = {
        "node_name": name,
        "node_id": entry["node_id"],
        "peer_id": entry["peer_id"],
    }
    for artifact in ARTIFACT_FIELDS:
        normalized_entry[artifact] = _canonical_descriptor(entry.get(artifact), f"input map {index} {name} {artifact}")
    value["context"] = context_descriptor
    value["plan_intent"] = intent_descriptor
    value["entries"] = [normalized_entry]
    return value


def _descriptor_key(value: dict[str, Any]) -> tuple[str, int, str]:
    return (value["sha256"], value["size_bytes"], value["path"])


def _aggregate(input_paths: Sequence[Path]) -> dict[str, Any]:
    if len(input_paths) != len(NODE_ORDER):
        die(f"exactly {len(NODE_ORDER)} --input-map arguments are required")
    resolved_paths = [path.absolute() for path in input_paths]
    if len(set(resolved_paths)) != len(resolved_paths):
        die("duplicate input-map paths are not allowed")
    values = [_read_input_map(path, index) for index, path in enumerate(resolved_paths)]
    first = values[0]
    task_uid = first["task_uid"]
    head_oid = first["head_oid"]
    context = first["context"]
    intent = first["plan_intent"]
    by_name: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(values):
        if value["task_uid"] != task_uid or value["head_oid"] != head_oid or value["network_id"] != CANONICAL_NETWORK_ID:
            die(f"input map {index} has mixed task/head/network binding")
        if value["context"]["sha256"] != context["sha256"] or value["context"]["size_bytes"] != context["size_bytes"]:
            die(f"input map {index} has mixed context bytes")
        if value["plan_intent"]["sha256"] != intent["sha256"] or value["plan_intent"]["size_bytes"] != intent["size_bytes"]:
            die(f"input map {index} has mixed plan-intent bytes")
        entry = value["entries"][0]
        name = entry["node_name"]
        if name in by_name:
            die(f"duplicate node entry: {name}")
        by_name[name] = entry
    if set(by_name) != set(NODE_ORDER):
        missing = sorted(set(NODE_ORDER) - set(by_name))
        unexpected = sorted(set(by_name) - set(NODE_ORDER))
        die(f"node set is incomplete or unexpected (missing={missing}, unexpected={unexpected})")

    # Pick a shared context/intent pair by content and path, not CLI order.
    # Sidecars may retain equivalent bytes in different transactions; keeping
    # the pair together avoids introducing a new cross-transaction pairing.
    shared = sorted(
        values,
        key=lambda value: (
            _descriptor_key(value["context"]),
            _descriptor_key(value["plan_intent"]),
            value["entries"][0]["node_name"],
        ),
    )[0]
    aggregate = {
        "schema_version": IDENTITY_V2_EVIDENCE_SCHEMA,
        "network_id": CANONICAL_NETWORK_ID,
        "task_uid": task_uid,
        "head_oid": head_oid,
        "context": copy.deepcopy(shared["context"]),
        "plan_intent": copy.deepcopy(shared["plan_intent"]),
        "entries": [copy.deepcopy(by_name[name]) for name in NODE_ORDER],
    }
    try:
        context_bytes = PLANNER._evidence_descriptor(aggregate["context"], "aggregate context")[1]
        context_value = json.loads(context_bytes.decode("utf-8"))
        capture_window_id = context_value.get("capture_window_id") if isinstance(context_value, dict) else None
        request = {
            "authority": {"task_uid": task_uid, "head_oid": head_oid},
            "capture_window_id": capture_window_id,
        }
        # This is the canonical planner path: it verifies every descriptor,
        # artifact binding, identity registry binding, and cross-pairing.
        PLANNER._identity_v2_evidence_map(aggregate, request)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        die(f"aggregate context JSON is malformed: {error.__class__.__name__}")
    except (SystemExit, KeyError, TypeError, ValueError, AttributeError) as error:
        die(f"canonical planner validation failed: {error}")
    return aggregate


def _check_output_parent(path: Path) -> None:
    if not path.is_absolute():
        die("--out must be an absolute path")
    parent = path.parent
    if parent.is_symlink() or not parent.is_dir():
        die("output parent must be an existing regular directory")
    current = parent
    while True:
        if current.is_symlink() and current not in {Path("/tmp"), Path("/var")}:
            die("output path has a symlinked ancestor")
        if current.parent == current:
            break
        current = current.parent
    metadata = parent.stat()
    current_uid = getattr(os, "getuid", lambda: None)()
    if current_uid is not None and hasattr(metadata, "st_uid") and metadata.st_uid != current_uid:
        die("output parent is not owned by the current user")
    if os.name != "nt" and stat.S_IMODE(metadata.st_mode) != 0o700:
        die("output parent must have mode 0700")
    if path.exists() or path.is_symlink():
        if path.is_symlink() or not path.is_file():
            die("output path must not be a symlink or non-regular file")
        output_metadata = path.stat()
        if current_uid is not None and hasattr(output_metadata, "st_uid") and output_metadata.st_uid != current_uid:
            die("existing output is not owned by the current user")
        if os.name != "nt" and stat.S_IMODE(output_metadata.st_mode) != 0o600:
            die("existing output must have mode 0600")


def _write_atomic(value: dict[str, Any], output: Path) -> None:
    _check_output_parent(output)
    payload = (json.dumps(value, ensure_ascii=True, sort_keys=True, indent=2) + "\n").encode("utf-8")
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=output.parent, prefix=f".{output.name}.", suffix=".partial", delete=False
        ) as handle:
            temporary = Path(handle.name)
            if hasattr(os, "fchmod"):
                os.fchmod(handle.fileno(), 0o600)
            else:
                temporary.chmod(0o600)
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, output)
        temporary = None
        if os.name != "nt":
            directory_fd = os.open(output.parent, os.O_RDONLY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
    except OSError as error:
        die(f"secure atomic output failed: {error.__class__.__name__}")
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except OSError:
                pass


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-map", action="append", required=True, metavar="PATH")
    parser.add_argument("--out", required=True, metavar="PATH")
    args = parser.parse_args(argv)
    input_paths = [Path(value) for value in args.input_map]
    if len(input_paths) != len(NODE_ORDER):
        parser.error(f"exactly {len(NODE_ORDER)} --input-map arguments are required")
    output = Path(args.out)
    resolved_inputs = {path.absolute() for path in input_paths}
    if output.absolute() in resolved_inputs:
        parser.error("--out must not replace an input map")
    aggregate = _aggregate(input_paths)
    _write_atomic(aggregate, output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BrokenPipeError:
        raise SystemExit(1)
