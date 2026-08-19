#!/usr/bin/env python3
"""Governed, fail-closed validator-pair rebuild transaction executor.

The executor is deliberately local-first.  A plan contains immutable package,
world and capacity evidence and can be reviewed without contacting a host.
``apply`` and ``rollback`` operate on explicitly supplied local node roots;
``apply`` additionally requires a governed host adapter that returns a
transaction-bound startup/health receipt.  An orchestration layer may map the
same transaction to a remote host, but this tool never guesses credentials or
silently falls back to SSH.  No observer is ever touched by this pair
transaction.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from typing import Any, NoReturn


PLAN_SCHEMA = "oasis7.validator_pair_rebuild_plan.v1"
SCHEMA = "oasis7.validator_pair_rebuild_transaction.v1"
MUTATION_ORDER = ["storage-205", "sequencer-204"]
STARTUP_ORDER = ["sequencer-204", "storage-205"]
IDENTITY_METADATA_FIELDS = (
    "key_path",
    "key_sha256",
    "key_size_bytes",
    "key_mode",
    "key_uid",
    "key_gid",
)
IDENTITY_EXPECTED_REQUIRED_FIELDS = ("key_mode", "key_uid", "key_gid")
EXPECTED_LISTENERS = {
    "storage-205": {"6632", "6832"},
    "sequencer-204": {"6631", "6831"},
}
RESET_SURFACES = [
    "data/execution-records",
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/storage",
    "data/runtime-root",
    "data/replication-root",
    "output/chain-runtime",
    "output/node-distfs",
]


def fail(message: str) -> NoReturn:
    raise SystemExit(f"error: validator-pair rebuild: {message}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_digest(value: dict[str, Any]) -> str:
    body = {key: item for key, item in value.items() if key != "canonical_digest"}
    return hashlib.sha256(
        json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def load_provenance_helper():
    helper_path = Path(__file__).with_name("p2p-public-testnet-validator-pair-provenance.py")
    spec = importlib.util.spec_from_file_location("oasis7_pair_provenance", helper_path)
    if spec is None or spec.loader is None:
        fail("cannot load provenance validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value


def path_kind(path: Path) -> str:
    if path.is_symlink():
        return "symlink"
    if path.is_dir():
        return "directory"
    if path.is_file():
        return "file"
    return "missing"


def inventory_tree(path: Path) -> dict[str, Any]:
    """Hash a tree without following symlinks and count every inode class."""
    digest = hashlib.sha256()
    total = 0
    # Count the directory root as well as its children: snapshot manifests
    # and copytree both account for that inode.
    entry_count = 1
    link_count = 0
    dir_count = 1
    file_count = 0
    if path.is_symlink():
        target = os.readlink(path)
        digest.update(f"L\0{target}\n".encode())
        return {
            "sha256": digest.hexdigest(),
            "total_bytes": 0,
            "entry_count": 1,
            "link_count": 1,
            "dir_count": 0,
            "file_count": 0,
        }
    if path.is_file():
        return {
            "sha256": sha256_file(path),
            "total_bytes": path.stat().st_size,
            "entry_count": 1,
            "link_count": 0,
            "dir_count": 0,
            "file_count": 1,
        }
    if not path.exists():
        return {
            "sha256": hashlib.sha256(b"<missing>").hexdigest(),
            "total_bytes": 0,
            "entry_count": 0,
            "link_count": 0,
            "dir_count": 0,
            "file_count": 0,
        }
    if not path.is_dir():
        fail(f"unsupported path entry: {path} ({stat.filemode(path.lstat().st_mode)})")
    for child in sorted(path.rglob("*"), key=lambda item: item.relative_to(path).as_posix()):
        rel = child.relative_to(path).as_posix()
        if child.is_symlink():
            target = os.readlink(child)
            digest.update(rel.encode("utf-8"))
            digest.update(b"\0L\0")
            digest.update(target.encode("utf-8"))
            digest.update(b"\n")
            entry_count += 1
            link_count += 1
        elif child.is_dir():
            # Directory entries consume inodes even when empty.  Do not add
            # them to the digest so governed-world file digests stay compatible.
            entry_count += 1
            dir_count += 1
        elif child.is_file():
            child_sha = sha256_file(child)
            size = child.stat().st_size
            # Keep the directory digest byte-for-byte compatible with the
            # provenance helper's governed world tree digest.
            digest.update(rel.encode("utf-8"))
            digest.update(b"\0")
            digest.update(child_sha.encode("ascii"))
            digest.update(b"\0")
            digest.update(str(size).encode("ascii"))
            digest.update(b"\n")
            total += size
            entry_count += 1
            file_count += 1
        else:
            fail(f"unsupported path entry: {child} ({stat.filemode(child.lstat().st_mode)})")
    return {
        "sha256": digest.hexdigest(),
        "total_bytes": total,
        "entry_count": entry_count,
        "link_count": link_count,
        "dir_count": dir_count,
        "file_count": file_count,
    }


def hash_tree(path: Path) -> tuple[str, int, int]:
    """Compatibility tuple for callers that need digest, bytes, inode count."""
    inventory = inventory_tree(path)
    return inventory["sha256"], inventory["total_bytes"], inventory["entry_count"]


INVENTORY_FIELDS = ("entry_count", "link_count", "dir_count", "file_count", "total_bytes")


def inventory_summary(inventory: dict[str, Any], label: str) -> dict[str, int]:
    """Return the complete count/byte contract without the implementation hash."""
    if not isinstance(inventory, dict):
        fail(f"{label} inventory must be an object")
    result: dict[str, int] = {}
    for field in INVENTORY_FIELDS:
        try:
            raw_value = inventory[field] if field != "total_bytes" else inventory.get("total_bytes", inventory.get("size_bytes"))
            value = int(raw_value)
        except (KeyError, TypeError, ValueError):
            fail(f"{label} inventory field is malformed: {field}")
        if value < 0:
            fail(f"{label} inventory field must be non-negative: {field}")
        result[field] = value
    if result["entry_count"] != result["link_count"] + result["dir_count"] + result["file_count"]:
        fail(f"{label} inventory entry count does not equal its inode classes")
    return result


def tree_size(path: Path) -> int:
    return hash_tree(path)[1]


def manifest_tree(root: Path) -> list[dict[str, Any]]:
    """Return complete metadata for a root, including empty directories."""
    if not root.exists():
        return []
    entries: list[dict[str, Any]] = []
    for item in sorted([root, *root.rglob("*")], key=lambda value: value.relative_to(root.parent).as_posix()):
        rel = item.relative_to(root).as_posix() if item != root else "."
        info: dict[str, Any] = {"path": rel, "kind": path_kind(item)}
        try:
            item_stat = item.lstat()
            info.update(
                {
                    "mode": stat.S_IMODE(item_stat.st_mode),
                    "uid": item_stat.st_uid,
                    "gid": item_stat.st_gid,
                }
            )
            if item.is_symlink():
                info["target"] = os.readlink(item)
            elif item.is_file():
                info["size_bytes"] = item_stat.st_size
                info["sha256"] = sha256_file(item)
        except OSError as error:
            fail(f"cannot inventory {item}: {error}")
        entries.append(info)
    return entries


def copy_entry(source: Path, destination: Path) -> None:
    if source.is_symlink():
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.symlink_to(os.readlink(source))
    elif source.is_dir():
        shutil.copytree(source, destination, symlinks=True)
    elif source.is_file():
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination, follow_symlinks=False)
    else:
        fail(f"cannot copy unsupported path: {source}")


def without_backup_entries(entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        entry
        for entry in entries
        if entry["path"] == "." or (entry["path"] != "backups" and not entry["path"].startswith("backups/"))
    ]


def apply_manifest_metadata(root: Path, entries: list[dict[str, Any]]) -> None:
    """Restore mode/owner metadata without ever reading key contents."""
    for entry in sorted(entries, key=lambda item: item["path"].count("/"), reverse=True):
        path = root if entry["path"] == "." else root / entry["path"]
        if not path.exists() and not path.is_symlink():
            fail(f"backup metadata path missing: {path}")
        try:
            if path.is_symlink():
                os.lchown(path, int(entry["uid"]), int(entry["gid"]))
            else:
                os.chown(path, int(entry["uid"]), int(entry["gid"]), follow_symlinks=False)
                os.chmod(path, int(entry["mode"]), follow_symlinks=False)
        except OSError as error:
            fail(f"cannot restore backup metadata for {path}: {error}")


def remove_entry(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.is_dir():
        shutil.rmtree(path)


def parse_node(raw: str) -> tuple[str, str, Path]:
    role, separator, target = raw.partition("=")
    if separator != "=" or role not in MUTATION_ORDER:
        fail(f"node must be role=local:/absolute/path: {raw}")
    scheme, separator, path = target.partition(":")
    if scheme != "local" or separator != ":" or not path:
        fail(f"only explicit local node roots are supported: {raw}")
    unresolved_root = Path(path).expanduser()
    if unresolved_root.is_symlink():
        fail(f"node root must not be a symlink: {unresolved_root}")
    root = unresolved_root.resolve()
    if not root.is_dir():
        fail(f"node root does not exist: {root}")
    return role, "local", root


def validate_current_link(root: Path, role: str) -> None:
    """Allow only the canonical root/current -> releases/<version> link."""
    current = root / "current"
    if not current.is_symlink():
        return
    target = os.readlink(current)
    if os.path.isabs(target):
        fail(f"{role} current symlink must be relative and canonical")
    resolved = (current.parent / target).resolve()
    releases_path = root / "releases"
    if releases_path.is_symlink() or not releases_path.is_dir():
        fail(f"{role} releases root must be a real directory")
    releases = releases_path.resolve()
    try:
        resolved.relative_to(releases)
    except ValueError:
        fail(f"{role} current symlink escapes releases: {current} -> {target}")
    if not resolved.is_dir():
        fail(f"{role} current symlink target is not a release directory: {resolved}")


def validate_node_symlinks(root: Path, role: str) -> None:
    validate_current_link(root, role)
    for item in root.rglob("*"):
        if item.is_symlink() and item.relative_to(root).as_posix() != "current":
            fail(f"{role} node root contains an unauthorized symlink: {item}")


def parse_nodes(raw_nodes: list[str]) -> dict[str, dict[str, Any]]:
    nodes: dict[str, dict[str, Any]] = {}
    for raw in raw_nodes:
        role, transport, root = parse_node(raw)
        if role in nodes:
            fail(f"duplicate node role: {role}")
        nodes[role] = {"role": role, "transport": transport, "root": str(root)}
    if set(nodes) != set(MUTATION_ORDER):
        fail(f"both node roles are required: {', '.join(MUTATION_ORDER)}")
    return {role: nodes[role] for role in MUTATION_ORDER}


def package_size(package_dir: Path) -> int:
    if not package_dir.is_dir():
        fail(f"package directory does not exist: {package_dir}")
    return tree_size(package_dir)


def validate_impact(path: Path) -> dict[str, Any]:
    impact = load_json(path, "consumer impact record")
    if impact.get("decision") != "proceed":
        fail("consumer impact decision must be proceed")
    for field in (
        "impact",
        "evidence_source",
        "timestamp",
        "validators_already_stopped",
        "outage_update_channel",
        "recovery_update_checkpoint",
        "producer_wording_approval",
    ):
        if field not in impact:
            fail(f"consumer impact record missing {field}")
    if impact.get("impact") not in {"active", "none", "unknown"}:
        fail("consumer impact must be active, none, or unknown")
    if not isinstance(impact.get("evidence_source"), str) or not impact["evidence_source"].strip():
        fail("consumer impact evidence_source must be non-empty")
    try:
        timestamp = str(impact["timestamp"]).replace("Z", "+00:00")
        parsed_timestamp = dt.datetime.fromisoformat(timestamp)
    except (TypeError, ValueError):
        fail("consumer impact timestamp must be RFC3339")
    if parsed_timestamp.tzinfo is None:
        fail("consumer impact timestamp must include a timezone")
    if impact.get("validators_already_stopped") is not True:
        fail("validators must be stopped before a pair rebuild plan")
    for field in ("outage_update_channel", "recovery_update_checkpoint", "producer_wording_approval"):
        value = impact.get(field)
        if not isinstance(value, str) or not value.strip():
            fail(f"consumer impact {field} must be non-empty")
        if impact["impact"] in {"active", "unknown"} and value.strip().lower() == "n/a":
            fail(f"consumer impact {field} cannot be n/a for active/unknown impact")
    return {"decision": "proceed", "evidence_source": impact["evidence_source"], "timestamp": impact["timestamp"]}


def capacity_for(
    capacity: dict[str, Any],
    role: str,
    required: int,
    required_inodes: int,
    inventory: dict[str, Any],
) -> dict[str, Any]:
    value = capacity.get(role)
    if not isinstance(value, dict):
        fail(f"capacity evidence missing {role}")
    try:
        free_bytes = int(value["free_bytes"])
        free_inodes = int(value["free_inodes"])
    except (KeyError, TypeError, ValueError):
        fail(f"capacity evidence malformed for {role}")
    if free_bytes < required:
        fail(f"capacity gate failed for {role}: {free_bytes} < {required} bytes")
    if free_inodes < required_inodes:
        fail(f"capacity gate failed for {role}: {free_inodes} < {required_inodes} free inodes")
    if value.get("same_filesystem") is not True:
        fail(f"same-filesystem backup gate failed for {role}")
    inventory_value = inventory_summary(inventory, f"{role} node")
    return {
        "free_bytes": free_bytes,
        "free_inodes": free_inodes,
        "same_filesystem": True,
        "required_bytes": required,
        "required_inodes": required_inodes,
        "inventory": inventory_value,
        "verified": True,
    }


def refresh_capacity(root: Path, planned: dict[str, Any], role: str) -> dict[str, Any]:
    try:
        usage = shutil.disk_usage(root)
        statvfs = os.statvfs(root)
        free_bytes = int(usage.free)
        free_inodes = int(statvfs.f_favail)
    except OSError as error:
        fail(f"cannot recapture apply-time capacity for {role}: {error}")
    required_bytes = int(planned["required_bytes"])
    required_inodes = int(planned["required_inodes"])
    actual_inventory = inventory_summary(inventory_tree(root), f"{role} apply-time node")
    planned_inventory = inventory_summary(planned.get("inventory"), f"{role} planned node")
    if actual_inventory != planned_inventory:
        fail(f"apply-time inventory changed for {role}: {actual_inventory} != {planned_inventory}")
    if free_bytes < required_bytes:
        fail(f"apply-time capacity gate failed for {role}: {free_bytes} < {required_bytes} bytes")
    if free_inodes < required_inodes:
        fail(f"apply-time capacity gate failed for {role}: {free_inodes} < {required_inodes} free inodes")
    return {
        "planned_free_bytes": int(planned["free_bytes"]),
        "planned_free_inodes": int(planned["free_inodes"]),
        "apply_free_bytes": free_bytes,
        "apply_free_inodes": free_inodes,
        "required_bytes": required_bytes,
        "required_inodes": required_inodes,
        "same_filesystem": planned.get("same_filesystem") is True,
        "inventory": actual_inventory,
        "verified": True,
    }


def reject_full_status(url: str | None, label: str) -> None:
    if url and "/v1/chain/status" in url:
        fail(f"204 {label} must use a bounded proof endpoint; full /v1/chain/status is forbidden")


def attestation_body(value: dict[str, Any]) -> bytes:
    body = {key: item for key, item in value.items() if key not in {"signature_ref", "public_key_ref"}}
    return json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(f"{label} must be an object")
    return value


def _require_exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    actual = set(value)
    if actual != expected:
        fail(f"{label} fields mismatch: missing={sorted(expected - actual)} extra={sorted(actual - expected)}")


def _require_hex(value: Any, length: int, label: str) -> str:
    if not isinstance(value, str) or len(value) != length or any(char not in "0123456789abcdefABCDEF" for char in value):
        fail(f"{label} must be {length}-hex")
    return value.lower()


def _require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        fail(f"{label} must be non-empty")
    return value


def _runtime_claim_bytes(raw: dict[str, Any], proof: dict[str, Any]) -> bytes:
    _require_exact_keys(
        raw,
        {
            "schema_version",
            "observed_at_unix_ms",
            "ok",
            "liveness",
            "readiness",
            "heights",
            "network_head",
            "checkpoint",
            "local_peer_id",
            "connected_peers",
            "connected_peer_count",
            "proof",
        },
        "raw rebuild proof",
    )
    if raw.get("schema_version") != "oasis7.rebuild_status.v1":
        fail("raw rebuild proof schema is unsupported")
    if not isinstance(raw.get("observed_at_unix_ms"), int) or isinstance(raw["observed_at_unix_ms"], bool):
        fail("raw rebuild proof observed_at_unix_ms must be an integer")
    if not isinstance(raw.get("ok"), bool):
        fail("raw rebuild proof ok must be boolean")
    liveness = _require_object(raw["liveness"], "raw rebuild proof liveness")
    _require_exact_keys(liveness, {"running", "last_error"}, "raw rebuild proof liveness")
    if not isinstance(liveness.get("running"), bool) or (
        liveness.get("last_error") is not None and not isinstance(liveness.get("last_error"), str)
    ):
        fail("raw rebuild proof liveness fields are malformed")
    readiness = _require_object(raw["readiness"], "raw rebuild proof readiness")
    _require_exact_keys(readiness, {"status", "failed_gates"}, "raw rebuild proof readiness")
    if readiness.get("status") not in {"ready", "not_ready"} or not isinstance(readiness.get("failed_gates"), list) or not all(
        isinstance(item, str) for item in readiness["failed_gates"]
    ):
        fail("raw rebuild proof readiness fields are malformed")
    if raw["ok"] != (readiness["status"] == "ready"):
        fail("raw rebuild proof ok/readiness mismatch")
    heights = _require_object(raw["heights"], "raw rebuild proof heights")
    _require_exact_keys(heights, {"committed_height", "network_committed_height", "last_execution_height"}, "raw rebuild proof heights")
    if not all(isinstance(heights.get(key), int) and not isinstance(heights.get(key), bool) and heights[key] >= 0 for key in heights):
        fail("raw rebuild proof heights are malformed")
    network_head = _require_object(raw["network_head"], "raw rebuild proof network_head")
    _require_exact_keys(
        network_head,
        {
            "source",
            "decision",
            "height",
            "block_hash",
            "execution_block_hash",
            "execution_state_root",
            "observed_peer_count",
            "fresh_peer_count",
        },
        "raw rebuild proof network_head",
    )
    if not isinstance(network_head.get("source"), str) or not isinstance(network_head.get("decision"), str):
        fail("raw rebuild proof network_head source/decision are malformed")
    for key in ("height", "observed_peer_count", "fresh_peer_count"):
        if network_head.get(key) is not None and (
            not isinstance(network_head[key], int) or isinstance(network_head[key], bool) or network_head[key] < 0
        ):
            fail(f"raw rebuild proof network_head {key} is malformed")
    for key in ("block_hash", "execution_block_hash", "execution_state_root"):
        if network_head.get(key) is not None and not isinstance(network_head[key], str):
            fail(f"raw rebuild proof network_head {key} is malformed")
    checkpoint = raw["checkpoint"]
    if checkpoint is not None:
        checkpoint = _require_object(checkpoint, "raw rebuild proof checkpoint")
        _require_exact_keys(
            checkpoint,
            {"schema_version", "checkpoint_id", "world_id", "height", "execution_block_hash", "execution_state_root", "manifest_hash"},
            "raw rebuild proof checkpoint",
        )
        if not isinstance(checkpoint.get("schema_version"), int) or isinstance(checkpoint["schema_version"], bool) or checkpoint["schema_version"] < 0:
            fail("raw rebuild proof checkpoint schema_version is malformed")
        for key in ("checkpoint_id", "world_id", "execution_block_hash", "execution_state_root", "manifest_hash"):
            _require_string(checkpoint.get(key), f"raw rebuild proof checkpoint {key}")
        if not isinstance(checkpoint.get("height"), int) or isinstance(checkpoint["height"], bool) or checkpoint["height"] < 0:
            fail("raw rebuild proof checkpoint height is malformed")
    local_peer_id = _require_string(raw.get("local_peer_id"), "raw rebuild proof local_peer_id")
    connected_peers = raw.get("connected_peers")
    if not isinstance(connected_peers, list) or len(connected_peers) > 64 or not all(isinstance(item, str) for item in connected_peers):
        fail("raw rebuild proof connected_peers are malformed")
    connected_peer_count = raw.get("connected_peer_count")
    if not isinstance(connected_peer_count, int) or isinstance(connected_peer_count, bool) or connected_peer_count < len(connected_peers):
        fail("raw rebuild proof connected_peer_count is malformed")
    _require_exact_keys(
        proof,
        {"schema_version", "signer_id", "signer_public_key_hex", "signed_payload_sha256", "signature_hex"},
        "raw rebuild proof envelope",
    )
    if proof.get("schema_version") != "oasis7.rebuild_proof.v1":
        fail("raw rebuild proof envelope schema is unsupported")
    signer_id = _require_string(proof.get("signer_id"), "raw rebuild proof signer_id")
    signer_public_key_hex = _require_hex(proof.get("signer_public_key_hex"), 64, "raw rebuild proof signer_public_key_hex")
    signed_payload_sha256 = _require_hex(proof.get("signed_payload_sha256"), 64, "raw rebuild proof signed_payload_sha256")
    signature_hex = _require_hex(proof.get("signature_hex"), 128, "raw rebuild proof signature_hex")
    ordered_claims = {
        "schema_version": raw["schema_version"],
        "observed_at_unix_ms": raw["observed_at_unix_ms"],
        "node_id": signer_id,
        "world_id": checkpoint["world_id"] if checkpoint is not None else "",
        "ok": raw["ok"],
        "liveness": {"running": liveness["running"], "last_error": liveness["last_error"]},
        "readiness": {"status": readiness["status"], "failed_gates": readiness["failed_gates"]},
        "heights": {
            "committed_height": heights["committed_height"],
            "network_committed_height": heights["network_committed_height"],
            "last_execution_height": heights["last_execution_height"],
        },
        "network_head": {
            "source": network_head["source"],
            "decision": network_head["decision"],
            "height": network_head["height"],
            "block_hash": network_head["block_hash"],
            "execution_block_hash": network_head["execution_block_hash"],
            "execution_state_root": network_head["execution_state_root"],
            "observed_peer_count": network_head["observed_peer_count"],
            "fresh_peer_count": network_head["fresh_peer_count"],
        },
        "checkpoint": (
            {
                "schema_version": checkpoint["schema_version"],
                "checkpoint_id": checkpoint["checkpoint_id"],
                "world_id": checkpoint["world_id"],
                "height": checkpoint["height"],
                "execution_block_hash": checkpoint["execution_block_hash"],
                "execution_state_root": checkpoint["execution_state_root"],
                "manifest_hash": checkpoint["manifest_hash"],
            }
            if checkpoint is not None
            else None
        ),
        "local_peer_id": local_peer_id,
        "connected_peers": connected_peers,
        "connected_peer_count": connected_peer_count,
    }
    claims = json.dumps(ordered_claims, ensure_ascii=True, separators=(",", ":")).encode()
    actual_digest = hashlib.sha256(claims).hexdigest()
    if signed_payload_sha256 != actual_digest:
        fail("raw rebuild proof signed payload digest mismatch")
    # The deployed Rust verifier performs the Ed25519 signature and trust-root
    # check before emitting this receipt. The executor replays the canonical
    # claim digest and binding here without depending on a host OpenSSL flavor.
    return {
        "signer_id": signer_id,
        "signer_public_key_hex": signer_public_key_hex,
        "signed_payload_sha256": signed_payload_sha256,
        "local_peer_id": local_peer_id,
        "signature_hex": signature_hex,
    }


def _run_governed_runtime_verifier(
    verifier: Path,
    raw_proof_path: Path,
    expected: dict[str, Any],
    expected_runtime: Path | None = None,
) -> None:
    if verifier.is_symlink() or not verifier.is_file() or not (verifier.stat().st_mode & 0o111):
        fail("governed runtime verifier must be an executable regular file")
    if expected_runtime is not None:
        if sha256_file(verifier) != sha256_file(expected_runtime) or verifier.stat().st_size != expected_runtime.stat().st_size:
            fail("governed runtime verifier identity differs from the verified package runtime")
    try:
        result = subprocess.run(
            [
                str(verifier),
                "verify-rebuild-proof",
                "--proof",
                str(raw_proof_path),
                "--trusted-signer-id",
                str(expected["signer_id"]),
                "--trusted-signer-public-key-hex",
                str(expected["signer_public_key_hex"]),
            ],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"governed runtime verifier failed: {error.__class__.__name__}")
    if result.returncode != 0:
        fail(f"governed runtime verifier rejected raw proof (exit {result.returncode})")
    try:
        observed = json.loads(result.stdout)
    except json.JSONDecodeError:
        fail("governed runtime verifier must return one JSON receipt")
    if not isinstance(observed, dict):
        fail("governed runtime verifier receipt must be an object")
    fields = {
        "schema_version",
        "proof_schema_version",
        "signer_id",
        "signer_public_key_hex",
        "signed_payload_sha256",
        "local_peer_id",
        "proof_sha256",
        "verified",
    }
    if set(observed) != fields:
        fail("governed runtime verifier receipt fields mismatch")
    for key, value in expected.items():
        if observed.get(key) != value:
            fail(f"governed runtime verifier receipt mismatch: {key}")


def _validate_identity_receipt_metadata(
    value: dict[str, Any],
    label: str,
    expected_role: str | None,
) -> dict[str, Any]:
    """Validate remote key metadata without dereferencing the reported path.

    Identity receipts are captured on the validator host.  Their ``key_path``
    is an audit reference, not a controller-local path; opening, stat-ing, or
    hashing it here could accidentally inspect an unrelated local key.  The
    host-side capture therefore supplies the immutable metadata tuple and the
    controller only validates its shape and security contract before binding
    it into the transaction evidence.
    """
    key_path = value.get("key_path")
    key_sha = value.get("key_sha256")
    key_size = value.get("key_size_bytes")
    key_mode = value.get("key_mode")
    key_uid = value.get("key_uid")
    key_gid = value.get("key_gid")
    node_id = value.get("node_id")
    peer_id = value.get("peer_id")
    receipt_role = value.get("role")
    if receipt_role is not None and receipt_role != expected_role:
        fail(f"{label} runtime identity metadata role binding mismatch")
    if (
        not isinstance(key_path, str)
        or not key_path.strip()
        or "\x00" in key_path
        or not Path(key_path).is_absolute()
    ):
        fail(f"{label} runtime identity metadata key path must be lexically absolute")
    if not isinstance(key_sha, str) or len(key_sha) != 64 or any(
        char not in "0123456789abcdefABCDEF" for char in key_sha
    ):
        fail(f"{label} runtime identity metadata key digest is invalid")
    if isinstance(key_size, bool) or not isinstance(key_size, int) or key_size <= 0:
        fail(f"{label} runtime identity metadata key size is invalid")
    if isinstance(key_mode, bool) or not isinstance(key_mode, int) or key_mode != 0o600:
        fail(f"{label} runtime identity metadata key mode must be 0600")
    for field, number in (("key_uid", key_uid), ("key_gid", key_gid)):
        if isinstance(number, bool) or not isinstance(number, int) or number < 0 or number > 0xFFFFFFFF:
            fail(f"{label} runtime identity metadata {field} is invalid")
    if not isinstance(node_id, str) or not node_id.strip() or not isinstance(peer_id, str) or not peer_id.strip():
        fail(f"{label} runtime identity metadata node/peer binding is incomplete")
    if expected_role not in MUTATION_ORDER:
        fail(f"{label} runtime identity metadata role binding is incomplete")
    return {
        "role": expected_role,
        "peer_id": peer_id,
        "node_id": node_id,
        "key_path": key_path,
        "key_sha256": key_sha.lower(),
        "key_size_bytes": key_size,
        "key_mode": key_mode,
        "key_uid": key_uid,
        "key_gid": key_gid,
    }


def _expected_identity_metadata(raw: dict[str, Any], label: str) -> dict[str, Any] | None:
    """Read the governed ownership tuple from an identity manifest entry.

    The runtime receipt is a host observation.  The manifest entry supplies
    the deployment-truth expectations; accepting the receipt's own uid/gid
    as its authority would make an integer-but-wrong owner indistinguishable
    from the expected service account.  Optional path/hash/size expectations
    are accepted only when the deployment truth governs those values.
    """
    nested = raw.get("expected_key_metadata")
    if nested is not None and not isinstance(nested, dict):
        fail(f"{label} expected key metadata must be an object")
    if isinstance(nested, dict):
        expected = dict(nested)
    else:
        expected = {
            key: raw[f"expected_{key}"]
            for key in IDENTITY_METADATA_FIELDS
            if f"expected_{key}" in raw
        }
    if not expected:
        return None
    unknown = set(expected) - set(IDENTITY_METADATA_FIELDS)
    if unknown:
        fail(f"{label} expected key metadata has unsupported fields: {', '.join(sorted(unknown))}")
    missing = [key for key in IDENTITY_EXPECTED_REQUIRED_FIELDS if key not in expected]
    if missing:
        fail(f"{label} expected key metadata is missing: {', '.join(missing)}")
    key_mode = expected.get("key_mode")
    key_uid = expected.get("key_uid")
    key_gid = expected.get("key_gid")
    if isinstance(key_mode, bool) or not isinstance(key_mode, int) or key_mode != 0o600:
        fail(f"{label} expected key mode must be 0600")
    for field, number in (("key_uid", key_uid), ("key_gid", key_gid)):
        if isinstance(number, bool) or not isinstance(number, int) or number < 0 or number > 0xFFFFFFFF:
            fail(f"{label} expected key {field} is invalid")
    optional = {key for key in ("key_path", "key_sha256", "key_size_bytes") if key in expected}
    if optional and optional != {"key_path", "key_sha256", "key_size_bytes"}:
        fail(f"{label} expected key path/hash/size must be complete when governed")
    if "key_path" in expected:
        key_path = expected["key_path"]
        if not isinstance(key_path, str) or not key_path.strip() or "\x00" in key_path or not Path(key_path).is_absolute():
            fail(f"{label} expected key path must be lexically absolute")
        key_sha = expected["key_sha256"]
        if not isinstance(key_sha, str) or len(key_sha) != 64 or any(
            char not in "0123456789abcdefABCDEF" for char in key_sha
        ):
            fail(f"{label} expected key digest is invalid")
        key_size = expected["key_size_bytes"]
        if isinstance(key_size, bool) or not isinstance(key_size, int) or key_size <= 0:
            fail(f"{label} expected key size is invalid")
    return {
        key: expected[key]
        for key in IDENTITY_METADATA_FIELDS
        if key in expected
    } | ({"key_sha256": expected["key_sha256"].lower()} if "key_sha256" in expected else {})


def _expected_identity_binding(raw: dict[str, Any], label: str) -> dict[str, str] | None:
    expected_node_id = raw.get("expected_node_id")
    expected_peer_id = raw.get("expected_peer_id")
    if expected_node_id is None and expected_peer_id is None:
        return None
    if not isinstance(expected_node_id, str) or not expected_node_id.strip() or not isinstance(expected_peer_id, str) or not expected_peer_id.strip():
        fail(f"{label} expected node/peer binding is incomplete")
    return {"node_id": expected_node_id, "peer_id": expected_peer_id}


def verify_signed_attestation(
    path: Path,
    trusted_root: dict[str, Any],
    label: str,
    expected_role: str | None = None,
    expected_proof_path: Path | None = None,
    expected_proof_verifier: Path | None = None,
    expected_runtime: Path | None = None,
    expected_identity_metadata: dict[str, Any] | None = None,
) -> dict[str, Any]:
    value = load_json(path, label)
    if value.get("schema_version") == "oasis7.identity_receipt.v1":
        if expected_identity_metadata is None:
            fail(f"{label} governed expected key metadata is required")
        metadata = _validate_identity_receipt_metadata(value, label, expected_role)
        for key, expected_value in expected_identity_metadata.items():
            if metadata.get(key) != expected_value:
                fail(f"{label} key metadata does not match governed deployment truth: {key}")
        return {
            "path": str(path.resolve()),
            "sha256": sha256_file(path),
            "schema_version": value["schema_version"],
            **metadata,
        }
    if value.get("schema_version") == "oasis7.rebuild_proof_verification.v1":
        if value.get("verified") is not True or value.get("proof_schema_version") != "oasis7.rebuild_proof.v1":
            fail(f"{label} runtime verifier receipt is not verified")
        trusted = [entry for entry in trusted_root.get("allowlist", []) if entry.get("signer_id") == value.get("signer_id")]
        public_key_hex = value.get("signer_public_key_hex")
        if not isinstance(public_key_hex, str) or not any(entry.get("public_key_hex", "").lower() == public_key_hex.lower() for entry in trusted):
            fail(f"{label} runtime verifier signer is not trust-root bound")
        local_peer_id = value.get("local_peer_id")
        proof_sha256 = value.get("proof_sha256")
        if not isinstance(local_peer_id, str) or not local_peer_id.strip():
            fail(f"{label} runtime verifier peer binding is required")
        if not isinstance(proof_sha256, str) or len(proof_sha256) != 64 or any(char not in "0123456789abcdefABCDEF" for char in proof_sha256):
            fail(f"{label} runtime verifier proof digest is required")
        if expected_proof_path is None:
            fail(f"{label} raw signed proof path is required")
        raw_proof_path = expected_proof_path.resolve()
        if raw_proof_path.is_symlink() or not raw_proof_path.is_file():
            fail(f"{label} raw signed proof must be a regular file")
        if sha256_file(raw_proof_path) != proof_sha256.lower():
            fail(f"{label} runtime verifier receipt is bound to a different raw proof")
        raw_proof = load_json(raw_proof_path, f"{label} raw signed proof")
        raw_summary = _runtime_claim_bytes(
            raw_proof,
            _require_object(raw_proof.get("proof"), f"{label} raw proof envelope"),
        )
        if raw_summary["signer_id"] != value.get("signer_id"):
            fail(f"{label} runtime verifier signer does not match raw proof")
        if raw_summary["signer_public_key_hex"] != public_key_hex.lower():
            fail(f"{label} runtime verifier public key does not match raw proof")
        if raw_summary["signed_payload_sha256"] != str(value.get("signed_payload_sha256", "")).lower():
            fail(f"{label} runtime verifier payload digest does not match raw proof")
        if raw_summary["local_peer_id"] != local_peer_id:
            fail(f"{label} runtime verifier peer binding does not match raw proof")
        if expected_proof_verifier is None:
            fail(f"{label} governed runtime verifier path is required")
        expected_receipt = {
            "schema_version": "oasis7.rebuild_proof_verification.v1",
            "proof_schema_version": "oasis7.rebuild_proof.v1",
            "signer_id": value["signer_id"],
            "signer_public_key_hex": public_key_hex.lower(),
            "signed_payload_sha256": str(value.get("signed_payload_sha256", "")).lower(),
            "local_peer_id": local_peer_id,
            "proof_sha256": proof_sha256.lower(),
            "verified": True,
        }
        _run_governed_runtime_verifier(
            expected_proof_verifier,
            raw_proof_path,
            expected_receipt,
            expected_runtime,
        )
        return {
            "path": str(path.resolve()),
            "sha256": sha256_file(path),
            "schema_version": value["schema_version"],
            "role": expected_role,
            "peer_id": local_peer_id,
            "node_id": value.get("signer_id"),
            "signer_id": value["signer_id"],
            "public_key_hex": public_key_hex,
            "proof_sha256": proof_sha256.lower(),
            "signed_payload_sha256": value.get("signed_payload_sha256"),
        }
    if value.get("schema_version") not in {"oasis7.validator_identity_receipt.v1", "oasis7.validator_pair_rebuild_proof.v1"}:
        fail(f"{label} has unsupported schema")
    if expected_role is not None and value.get("role") != expected_role:
        fail(f"{label} role binding mismatch")
    if value.get("trust_root_digest") != trusted_root.get("root_digest"):
        fail(f"{label} trust-root binding mismatch")
    signature_ref = value.get("signature_ref")
    public_key_ref = value.get("public_key_ref")
    signer_id = value.get("signer_id")
    algorithm = value.get("algorithm")
    if not all(isinstance(item, str) and item.strip() for item in (signature_ref, public_key_ref, signer_id, algorithm)):
        fail(f"{label} signed identity fields are required")
    signature_path = Path(signature_ref)
    public_key_path = Path(public_key_ref)
    if signature_path.is_symlink() or public_key_path.is_symlink() or not signature_path.is_file() or not public_key_path.is_file():
        fail(f"{label} signature/public key refs must be regular files")
    public_key_sha = sha256_file(public_key_path)
    if value.get("public_key_sha256") != public_key_sha:
        fail(f"{label} public key hash mismatch")
    trusted = {tuple((entry.get(key) for key in ("signer_id", "algorithm", "public_key_sha256"))) for entry in trusted_root.get("allowlist", [])}
    if (signer_id, algorithm, public_key_sha) not in trusted:
        fail(f"{label} signer is not in the governed trusted signer allowlist")
    with tempfile.TemporaryDirectory(prefix="oasis7-attestation-") as temp_dir:
        payload_path = Path(temp_dir) / "payload"
        payload_path.write_bytes(attestation_body(value))
        try:
            verify_command = (
                ["openssl", "dgst", "-sha256", "-verify", str(public_key_path), "-signature", str(signature_path), str(payload_path)]
                if algorithm in {"openssl-rsa-sha256", "rsa-sha256"}
                else ["openssl", "pkeyutl", "-verify", "-pubin", "-inkey", str(public_key_path), "-sigfile", str(signature_path), "-rawin", "-in", str(payload_path)]
                if algorithm in {"openssl-ed25519", "ed25519"}
                else None
            )
            if verify_command is None:
                fail(f"{label} unsupported signature algorithm")
            result = subprocess.run(
                verify_command,
                check=False,
                capture_output=True,
                timeout=10,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            fail(f"{label} signature verifier unavailable: {error.__class__.__name__}")
    if result.returncode != 0:
        fail(f"{label} detached signature verification failed")
    return {
        "path": str(path.resolve()),
        "sha256": sha256_file(path),
        "schema_version": value["schema_version"],
        "role": value.get("role"),
        "peer_id": value.get("peer_id"),
        "node_id": value.get("node_id"),
        "signer_id": signer_id,
        "public_key_sha256": public_key_sha,
    }


def validate_signed_gates(
    args: argparse.Namespace,
    provenance_summary: dict[str, Any],
    expected_runtime: Path | None = None,
) -> dict[str, Any]:
    trusted_root = provenance_summary.get("trusted_root")
    if not isinstance(trusted_root, dict):
        fail("governed trusted signer root is required for signed identity gates")
    if not args.identity_receipts or not args.sequencer_rebuild_proof:
        fail("identity receipts and signed 204 rebuild proof are required")
    identity_path = Path(args.identity_receipts).resolve()
    identities = load_json(identity_path, "identity receipts")
    raw_receipts = identities.get("receipts")
    if not isinstance(raw_receipts, list) or len(raw_receipts) != 2:
        fail("identity receipts must cover both validator roles")
    summaries = []
    for raw in raw_receipts:
        expected_role = raw.get("role") if isinstance(raw, dict) else None
        receipt_path = Path(raw.get("path")) if isinstance(raw, dict) and isinstance(raw.get("path"), str) else Path(raw) if isinstance(raw, str) else None
        if receipt_path is None:
            fail("identity receipt path must be a string")
        expected_metadata = _expected_identity_metadata(raw, "identity manifest entry") if isinstance(raw, dict) else None
        expected_binding = _expected_identity_binding(raw, "identity manifest entry") if isinstance(raw, dict) else None
        receipt_path = receipt_path.resolve()
        receipt_value = load_json(receipt_path, "identity receipt")
        if receipt_value.get("schema_version") == "oasis7.identity_receipt.v1" and expected_metadata is None:
            fail("identity manifest must carry governed key metadata for oasis7.identity_receipt.v1")
        if receipt_value.get("schema_version") == "oasis7.identity_receipt.v1" and expected_binding is None:
            fail("identity manifest must carry governed node/peer binding for oasis7.identity_receipt.v1")
        summary = verify_signed_attestation(
            receipt_path,
            trusted_root,
            "identity receipt",
            expected_role,
            expected_identity_metadata=expected_metadata,
        )
        summaries.append(summary)
        if expected_binding is not None and any(summary.get(key) != value for key, value in expected_binding.items()):
            fail("identity receipt node/peer does not match governed deployment truth")
    roles = {item.get("role") for item in summaries}
    if roles != set(MUTATION_ORDER):
        fail("identity receipts must cover storage-205 and sequencer-204")
    registry_path = Path(provenance_summary["governed"]["registry"]["path"])
    registry = load_json(registry_path, "governed validator registry")
    registered_nodes = {
        str(entry.get("node_id"))
        for entry in registry.get("validators", [])
        if isinstance(entry, dict) and isinstance(entry.get("node_id"), str)
    }
    identity_nodes = {item.get("node_id") for item in summaries}
    if not registered_nodes or identity_nodes != registered_nodes:
        fail("identity receipts do not match the governed validator registry")
    bootstrap_path = Path(provenance_summary["governed"]["bootstrap"]["path"])
    bootstrap_text = bootstrap_path.read_text(encoding="utf-8")
    for item in summaries:
        if not isinstance(item.get("peer_id"), str) or item["peer_id"] not in bootstrap_text:
            fail("identity receipt peer id is absent from governed bootstrap truth")
    raw_proof_path = Path(args.sequencer_rebuild_proof).resolve()
    verification_path = (
        Path(args.sequencer_rebuild_proof_verification).resolve()
        if args.sequencer_rebuild_proof_verification
        else None
    )
    proof_summary = verify_signed_attestation(
        verification_path or raw_proof_path,
        trusted_root,
        "signed 204 rebuild proof",
        "sequencer-204",
        raw_proof_path if verification_path else None,
        Path(args.sequencer_proof_verifier).resolve() if args.sequencer_proof_verifier else None,
        expected_runtime,
    )
    if verification_path:
        raw_proof = load_json(raw_proof_path, "raw signed 204 rebuild proof")
        if raw_proof.get("schema_version") != "oasis7.rebuild_status.v1" or not isinstance(raw_proof.get("proof"), dict):
            fail("raw signed 204 rebuild proof has unsupported schema")
        raw_envelope = raw_proof["proof"]
        if raw_envelope.get("signer_id") != proof_summary.get("signer_id"):
            fail("runtime verifier signer does not match raw proof signer")
        if raw_envelope.get("signer_public_key_hex", "").lower() != str(proof_summary.get("public_key_hex", "")).lower():
            fail("runtime verifier public key does not match raw proof signer")
        if raw_envelope.get("signed_payload_sha256") != proof_summary.get("signed_payload_sha256"):
            fail("runtime verifier payload digest does not match raw proof")
        if raw_proof.get("local_peer_id") != proof_summary.get("peer_id"):
            fail("runtime verifier peer binding does not match raw proof")
    sequencer_identity = next(item for item in summaries if item.get("role") == "sequencer-204")
    if proof_summary.get("node_id") != sequencer_identity.get("node_id") or proof_summary.get("peer_id") != sequencer_identity.get("peer_id"):
        fail("signed 204 rebuild proof identity does not match sequencer identity receipt")
    return {"identity_receipts": summaries, "sequencer_rebuild_proof": proof_summary}


def build_plan(args: argparse.Namespace) -> dict[str, Any]:
    package_dir = Path(args.package_dir).resolve()
    provenance_path = Path(args.provenance).resolve()
    helper = load_provenance_helper()
    try:
        provenance_summary = helper.validate_receipt(provenance_path, package_dir, Path(args.trust_root).resolve())
    except SystemExit as error:
        fail(str(error).removeprefix("error: validator-pair provenance: "))
    runtime_source = helper.find_runtime(package_dir)
    impact = validate_impact(Path(args.consumer_impact_record).resolve())
    nodes = parse_nodes(args.node)
    capacity = load_json(Path(args.capacity_json).resolve(), "capacity evidence")
    package_bytes = package_size(package_dir)
    governed = provenance_summary["governed"]
    signed_gates = validate_signed_gates(args, provenance_summary, runtime_source)
    governed_bytes = sum(int(item.get("size_bytes", item.get("total_bytes", 0))) for item in governed.values())
    capacities: dict[str, Any] = {}
    for role, node in nodes.items():
        validate_node_symlinks(Path(node["root"]), role)
        for surface in RESET_SURFACES:
            if (Path(node["root"]) / surface).is_symlink():
                fail(f"reset surface must not be a symlink: {role}/{surface}")
        node_inventory = inventory_tree(Path(node["root"]))
        full_backup_bytes = node_inventory["total_bytes"]
        full_backup_entries = node_inventory["entry_count"]
        required = int((full_backup_bytes + package_bytes + governed_bytes) * 1.20) + 1
        governed_entries = sum(
            inventory_summary(item, f"governed.{key}")["entry_count"] for key, item in governed.items()
        )
        package_entries = inventory_tree(package_dir)["entry_count"]
        required_inodes = max(128, int((full_backup_entries + package_entries + governed_entries) * 1.20) + 16)
        capacities[role] = capacity_for(capacity, role, required, required_inodes, node_inventory)
    reject_full_status(args.sequencer_proof_url, "proof URL")
    reject_full_status(args.sequencer_health_url, "health URL")
    plan: dict[str, Any] = {
        "schema_version": PLAN_SCHEMA,
        "phase": "planned",
        "mutation_order": MUTATION_ORDER,
        "startup_order": STARTUP_ORDER,
        "package": {
            "directory": str(package_dir),
            "provenance": str(provenance_path),
            "version": provenance_summary["package"]["package_version"],
            "run_id": provenance_summary["package"]["run_id"],
            "commit": provenance_summary["package"]["commit"],
            "runtime_sha256": provenance_summary["package"]["runtime_sha256"],
            "runtime_size_bytes": provenance_summary["package"]["runtime_size_bytes"],
            "runtime_relpath": runtime_source.relative_to(package_dir).as_posix(),
        },
        "provenance": {
            "path": str(provenance_path),
            "binding_digest": provenance_summary["binding_digest"],
            "signature": provenance_summary["signature"],
            "trusted_root": provenance_summary["trusted_root"],
        },
        "network": {
            "network_id": provenance_summary["network_id"],
            "chain_id": provenance_summary["chain_id"],
            "governed": provenance_summary["governed"],
        },
        "consumer_impact": impact,
        "nodes": nodes,
        "capacity": capacities,
        "proof": {
            "storage_health_url": args.storage_health_url,
            "sequencer_health_url": args.sequencer_health_url,
            "sequencer_proof_url": args.sequencer_proof_url,
            "full_204_status_forbidden": True,
            "identity_receipts_path": str(Path(args.identity_receipts).resolve()) if args.identity_receipts else None,
            "sequencer_rebuild_proof_path": str(Path(args.sequencer_rebuild_proof).resolve()) if args.sequencer_rebuild_proof else None,
            "sequencer_rebuild_proof_verification_path": str(Path(args.sequencer_rebuild_proof_verification).resolve()) if args.sequencer_rebuild_proof_verification else None,
            "sequencer_proof_verifier_path": str(Path(args.sequencer_proof_verifier).resolve()) if args.sequencer_proof_verifier else None,
            **signed_gates,
        },
        "observer_gate": {
            "status": "hold",
            "required_before_observer_mutation": True,
            "receipt": str(Path(args.observer_receipt).resolve()) if args.observer_receipt else None,
        },
        "rollback": {"strategy": "same-filesystem-full-snapshot", "required_on_gate_failure": True},
    }
    plan["plan_digest"] = hashlib.sha256(
        json.dumps(plan, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    return plan


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def snapshot_node(node: dict[str, Any], transaction_id: str) -> dict[str, Any]:
    root = Path(node["root"])
    backup_root = root / "backups" / transaction_id
    if backup_root.exists():
        fail(f"backup already exists for {node['role']}: {backup_root}")
    backup_root.mkdir(parents=True, exist_ok=False)
    try:
        snapshot = backup_root / "snapshot"
        snapshot.mkdir()
        source_manifest = without_backup_entries(manifest_tree(root))
        for child in sorted(root.iterdir(), key=lambda item: item.name):
            if child.name == "backups":
                continue
            copy_entry(child, snapshot / child.name)
        apply_manifest_metadata(snapshot, source_manifest)
        manifest = source_manifest
        if manifest_tree(snapshot) != manifest:
            fail(f"backup snapshot manifest verification failed for {root}")
        manifest_path = backup_root / "manifest.json"
        manifest_path.write_text(json.dumps({"schema_version": "oasis7.validator_pair_rebuild_backup.v1", "root": str(root), "entries": manifest}, indent=2) + "\n", encoding="utf-8")
        digest = sha256_file(manifest_path)
        return {
            "root": str(root),
            "backup_root": str(backup_root),
            "snapshot": str(snapshot),
            "manifest": str(manifest_path),
            "manifest_sha256": digest,
            "manifest_entries": len(manifest),
            "verified": bool(manifest),
        }
    except BaseException:
        try:
            if backup_root.exists() or backup_root.is_symlink():
                remove_entry(backup_root)
        except BaseException as cleanup_error:
            fail(f"snapshot failed for {root}; partial backup cleanup failed: {cleanup_error}")
        raise


def clear_node_preserving_backup(root: Path, backup_root: Path) -> None:
    for child in list(root.iterdir()):
        if child == backup_root or child == backup_root.parent:
            continue
        remove_entry(child)


def install_local(node: dict[str, Any], plan: dict[str, Any], backup: dict[str, Any]) -> dict[str, Any]:
    root = Path(node["root"])
    package_dir = Path(plan["package"]["directory"])
    package_version = str(plan["package"]["version"])
    backup_root = Path(backup["backup_root"])
    # Rebuild reset surfaces only after the complete root snapshot is durable.
    for surface in RESET_SURFACES:
        path = root / surface
        if path.exists() or path.is_symlink():
            remove_entry(path)
        path.mkdir(parents=True, exist_ok=True)
    release = root / "releases" / package_version / "bin"
    release.mkdir(parents=True, exist_ok=True)
    runtime = package_dir / plan["package"]["runtime_relpath"]
    if not runtime.is_file() or runtime.is_symlink():
        fail(f"package runtime disappeared before apply: {package_dir}")
    runtime_destination = release / "oasis7_chain_runtime"
    shutil.copy2(runtime, runtime_destination)
    if sha256_file(runtime_destination) != plan["package"]["runtime_sha256"] or runtime_destination.stat().st_size != int(plan["package"]["runtime_size_bytes"]):
        fail(f"runtime identity mismatch after staging {node['role']}")
    current = root / "current"
    if current.exists() or current.is_symlink():
        remove_entry(current)
    current.symlink_to(release.parent, target_is_directory=True)
    governed_root = root / "staged-governed"
    governed_root.mkdir(parents=True, exist_ok=True)
    staged: dict[str, str] = {}
    governed_inventory: dict[str, dict[str, int]] = {}
    for key, value in plan["network"]["governed"].items():
        source = Path(value["path"])
        governed_destination = governed_root / key
        if governed_destination.exists() or governed_destination.is_symlink():
            remove_entry(governed_destination)
        copy_entry(source, governed_destination)
        actual_tree = inventory_tree(governed_destination)
        actual_sha = actual_tree["sha256"] if value.get("kind") == "directory" else actual_tree["sha256"]
        total = actual_tree["total_bytes"]
        expected_sha = value.get("sha256") or value.get("sha256_tree")
        if actual_sha != expected_sha or total != int(value.get("size_bytes", value.get("total_bytes", 0))):
            fail(f"governed {key} identity mismatch after staging {node['role']}")
        expected_inventory = inventory_summary(value, f"governed.{key}")
        actual_inventory = inventory_summary(actual_tree, f"staged governed.{key}")
        if actual_inventory != expected_inventory:
            fail(f"governed {key} inventory mismatch after staging {node['role']}")
        staged[key] = str(governed_destination)
        governed_inventory[key] = actual_inventory
    return {
        "staged_release": str(release.parent),
        "runtime": str(runtime_destination),
        "governed": staged,
        "governed_inventory": governed_inventory,
        "service_action": "local dry-run; service manager delegated to governed host adapter",
        "startup_order_position": STARTUP_ORDER.index(node["role"]) + 1,
        "post_apply_gates": {
            "status": "deferred_to_governed_host_adapter",
            "healthz": "not_run_local",
            "nrestarts": "not_run_local",
            "oom_panic_segfault": "not_run_local",
        },
        "backup_root": str(backup_root),
    }


def validate_host_receipt(receipt: dict[str, Any], plan: dict[str, Any], phase: str) -> dict[str, Any]:
    if receipt.get("schema_version") not in {"oasis7.validator_pair_rebuild_host_receipt.v1", "oasis7.validator_pair_rebuild_host_receipt.v2"}:
        fail("host adapter returned an unsupported receipt schema")
    _validate_adapter_binding(receipt, plan, phase)
    if receipt.get("phase") != phase:
        fail(f"host adapter phase receipt mismatch: expected {phase}")
    if receipt.get("transaction_id") != plan.get("transaction_id"):
        fail("host adapter receipt transaction identity mismatch")
    if receipt.get("mutation_order") != MUTATION_ORDER or receipt.get("startup_order") != STARTUP_ORDER:
        fail("host adapter receipt order mismatch")
    nodes = receipt.get("nodes")
    if not isinstance(nodes, dict) or set(nodes) != set(MUTATION_ORDER):
        fail("host adapter receipt must cover both validator roles")
    if phase == "quiesce":
        for role in MUTATION_ORDER:
            value = nodes[role]
            if value.get("active") is not False or value.get("running") is not False:
                fail(f"host adapter quiesce gate failed for {role}")
        return receipt
    if phase == "backup":
        for role in MUTATION_ORDER:
            if nodes[role].get("backup_verified") is not True:
                fail(f"host adapter backup gate failed for {role}")
        return receipt
    if phase == "rollback":
        for role in MUTATION_ORDER:
            if nodes[role].get("rollback_verified") is not True:
                fail(f"host adapter rollback gate failed for {role}")
        return receipt
    for role in MUTATION_ORDER:
        value = nodes[role]
        if not isinstance(value, dict):
            fail(f"host adapter receipt malformed for {role}")
        if value.get("active") is not True or value.get("running") is not True or value.get("healthz_ok") is not True:
            fail(f"host adapter health gate failed for {role}")
        if value.get("nrestarts") != 0:
            fail(f"host adapter restart gate failed for {role}")
        if value.get("oom_panic_segfault") is not False:
            fail(f"host adapter OOM/panic/segfault gate failed for {role}")
        if value.get("runtime_sha256") != plan["package"]["runtime_sha256"] or int(value.get("runtime_size_bytes", -1)) != int(plan["package"]["runtime_size_bytes"]):
            fail(f"host adapter runtime identity mismatch for {role}")
        if not isinstance(value.get("listeners"), list) or not value["listeners"]:
            fail(f"host adapter listener gate missing for {role}")
        actual_listeners = {str(listener) for listener in value["listeners"]}
        if not EXPECTED_LISTENERS[role].issubset(actual_listeners):
            fail(f"host adapter listener identity mismatch for {role}")
        if role == "sequencer-204" and value.get("full_chain_status_called") is not False:
            fail("204 host adapter receipt must prove full /v1/chain/status was not called")
    identity_receipts = receipt.get("identity_receipts")
    proof = receipt.get("sequencer_rebuild_proof")
    if not isinstance(identity_receipts, list) or not isinstance(proof, dict):
        fail("host adapter must consume signed identity receipts and the signed 204 rebuild proof")
    expected_evidence = _expected_adapter_evidence(plan)
    expected_identities = expected_evidence["identity_receipts"]
    if len(identity_receipts) != len(expected_identities):
        fail("host adapter identity receipt set is incomplete")
    observed_by_role: dict[str, dict[str, Any]] = {}
    for value in identity_receipts:
        if not isinstance(value, dict) or value.get("role") in observed_by_role:
            fail("host adapter identity receipt set contains duplicate or malformed roles")
        observed_by_role[str(value.get("role"))] = value
    if set(observed_by_role) != set(MUTATION_ORDER):
        fail("host adapter identity receipt set must contain both canonical roles")
    trusted_root = plan["provenance"].get("trusted_root")
    if not isinstance(trusted_root, dict):
        fail("host adapter receipt has no governed trust root")
    for expected in expected_identities:
        value = observed_by_role[expected["role"]]
        identity_keys = (
            "path",
            "sha256",
            "role",
            "node_id",
            "peer_id",
            "key_path",
            "key_sha256",
            "key_size_bytes",
            "key_mode",
            "key_uid",
            "key_gid",
        )
        for key in identity_keys:
            if key not in expected:
                continue
            if value.get(key) != expected[key]:
                fail(f"host adapter identity evidence binding mismatch for {expected['role']}: {key}")
        identity_metadata = {
            key: expected[key]
            for key in IDENTITY_METADATA_FIELDS
            if key in expected
        }
        verified_identity = verify_signed_attestation(
            Path(expected["path"]),
            trusted_root,
            "host identity receipt",
            expected["role"],
            expected_identity_metadata=identity_metadata or None,
        )
        for key in identity_keys:
            if key not in expected:
                continue
            if verified_identity.get(key) != expected[key]:
                fail(f"host identity receipt content mismatch for {expected['role']}: {key}")
    expected_proof = expected_evidence["sequencer_rebuild_proof"]
    for key in expected_proof:
        if proof.get(key) != expected_proof.get(key):
            fail(f"host adapter signed proof evidence binding mismatch: {key}")
    proof_path = Path(expected_proof["path"]).resolve()
    verification_path = (
        Path(expected_proof["verification_path"]).resolve()
        if isinstance(expected_proof.get("verification_path"), str)
        else Path(plan["proof"]["sequencer_rebuild_proof_verification_path"]).resolve()
        if plan.get("proof", {}).get("sequencer_rebuild_proof_verification_path")
        else None
    )
    verify_signed_attestation(
        verification_path or proof_path,
        trusted_root,
        "host signed 204 rebuild proof",
        "sequencer-204",
        proof_path if verification_path else None,
        Path(plan["proof"]["sequencer_proof_verifier_path"]).resolve()
        if verification_path and plan.get("proof", {}).get("sequencer_proof_verifier_path")
        else None,
        Path(plan["package"]["directory"]) / plan["package"]["runtime_relpath"]
        if verification_path
        else None,
    )
    if receipt.get("sequencer_proof_url") != plan.get("proof", {}).get("sequencer_proof_url"):
        fail("host adapter proof endpoint binding mismatch")
    if plan.get("observer_gate", {}).get("status") != "hold":
        fail("observer gate must remain hold during validator pair rebuild")
    if receipt.get("observer_mutation") is not False:
        fail("host adapter must prove observer_hold; observer mutation is forbidden")
    return receipt


def run_host_adapter(adapter: Path, transaction_path: Path, plan: dict[str, Any], phase: str) -> dict[str, Any]:
    if adapter.is_symlink() or not adapter.is_file():
        fail(f"host adapter must be a regular file: {adapter}")
    adapter = adapter.resolve()
    phase_window_started_at = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    plan["adapter_binding"] = _build_adapter_binding(plan, phase, phase_window_started_at)
    plan["canonical_digest"] = canonical_digest(plan)
    write_json(transaction_path, plan)
    try:
        result = subprocess.run(
            [str(adapter), "--phase", phase, "--transaction", str(transaction_path)],
            check=False,
            text=True,
            capture_output=True,
            timeout=300,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"host adapter execution failed: {error.__class__.__name__}")
    if result.returncode != 0:
        fail(f"host adapter failed with exit {result.returncode}")
    try:
        receipt = json.loads(result.stdout)
    except json.JSONDecodeError:
        fail("host adapter must return one JSON receipt on stdout")
    if not isinstance(receipt, dict):
        fail("host adapter receipt must be a JSON object")
    return validate_host_receipt(receipt, plan, phase)


def restore_node(backup: dict[str, Any], transaction_id: str | None = None) -> dict[str, Any]:
    root = Path(backup["root"])
    backup_root = Path(backup["backup_root"])
    snapshot = Path(backup["snapshot"])
    manifest_path = Path(backup["manifest"])
    if not snapshot.is_dir() or not manifest_path.is_file():
        fail(f"rollback snapshot missing: {backup_root}")
    if transaction_id is not None:
        expected_backup_root = root / "backups" / transaction_id
        if backup_root.resolve() != expected_backup_root.resolve():
            fail(f"rollback backup identity mismatch for {root}")
    if backup_root.resolve().parent != root.resolve() / "backups":
        fail(f"rollback backup root escapes node root: {backup_root}")
    if manifest_path.resolve().parent != backup_root.resolve() or snapshot.resolve().parent != backup_root.resolve():
        fail(f"rollback backup paths escape backup root: {backup_root}")
    expected_manifest_sha = backup.get("manifest_sha256")
    if not isinstance(expected_manifest_sha, str) or expected_manifest_sha != sha256_file(manifest_path):
        fail(f"rollback manifest digest mismatch: {manifest_path}")
    try:
        manifest_payload = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"rollback manifest unreadable: {error}")
    if manifest_payload.get("schema_version") != "oasis7.validator_pair_rebuild_backup.v1":
        fail(f"rollback manifest schema mismatch: {manifest_path}")
    if manifest_payload.get("root") != str(root):
        fail(f"rollback manifest root mismatch: {manifest_path}")
    expected = manifest_payload.get("entries")
    if not isinstance(expected, list) or not expected:
        fail(f"rollback manifest entries missing: {manifest_path}")
    snapshot_manifest = without_backup_entries(manifest_tree(snapshot))
    if snapshot_manifest != expected:
        fail(f"rollback snapshot manifest verification failed for {root}")
    clear_node_preserving_backup(root, backup_root)
    for child in sorted(snapshot.iterdir(), key=lambda item: item.name):
        copy_entry(child, root / child.name)
    apply_manifest_metadata(root, expected)
    restored_manifest = without_backup_entries(manifest_tree(root))
    if restored_manifest != expected:
        fail(f"rollback manifest verification failed for {root}")
    return {"verified": True, "restored_manifest_entries": len(restored_manifest)}


def _parse_timestamp(value: Any, label: str) -> dt.datetime:
    if not isinstance(value, str) or not value.strip():
        fail(f"{label} must be an RFC3339 timestamp")
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        fail(f"{label} must be an RFC3339 timestamp")
    if parsed.tzinfo is None:
        fail(f"{label} must include a timezone")
    return parsed.astimezone(dt.timezone.utc)


def _bound_regular_file(path_value: Any, expected_sha256: Any, label: str) -> dict[str, str]:
    if not isinstance(path_value, str) or not path_value.strip():
        fail(f"{label} path is required")
    if not isinstance(expected_sha256, str) or len(expected_sha256) != 64:
        fail(f"{label} digest is required")
    raw_path = Path(path_value)
    if raw_path.is_symlink():
        fail(f"{label} must be a regular file")
    path = raw_path.resolve()
    if not path.is_file():
        fail(f"{label} must be a regular file")
    actual_sha256 = sha256_file(path)
    if actual_sha256 != expected_sha256.lower():
        fail(f"{label} digest mismatch")
    return {"path": str(path), "sha256": actual_sha256}


def _expected_adapter_evidence(plan: dict[str, Any]) -> dict[str, Any]:
    """Build the immutable evidence contract copied into each host receipt.

    The plan's signed-gate summaries are the authority.  Host adapters may
    report observations, but they cannot substitute a different trusted file
    or a different validator identity.
    """
    proof = plan.get("proof")
    if not isinstance(proof, dict):
        fail("plan signed evidence is missing")
    raw_identities = proof.get("identity_receipts")
    raw_proof = proof.get("sequencer_rebuild_proof")
    if not isinstance(raw_identities, list) or not isinstance(raw_proof, dict):
        fail("plan signed evidence summaries are missing")
    identities: list[dict[str, Any]] = []
    for summary in raw_identities:
        if not isinstance(summary, dict):
            fail("plan identity evidence summary is malformed")
        bound_file = _bound_regular_file(summary.get("path"), summary.get("sha256"), "plan identity evidence")
        role = summary.get("role")
        node_id = summary.get("node_id")
        peer_id = summary.get("peer_id")
        if role not in MUTATION_ORDER or not isinstance(node_id, str) or not node_id.strip() or not isinstance(peer_id, str) or not peer_id.strip():
            fail("plan identity evidence role/node/peer binding is incomplete")
        metadata_keys = ("key_path", "key_sha256", "key_size_bytes", "key_mode", "key_uid", "key_gid")
        metadata_present = {key for key in metadata_keys if key in summary}
        if metadata_present and metadata_present != set(metadata_keys):
            fail("plan identity evidence metadata tuple is incomplete")
        identity = {**bound_file, "role": role, "node_id": node_id, "peer_id": peer_id}
        if metadata_present:
            identity.update(_validate_identity_receipt_metadata(summary, "plan identity evidence", role))
        identities.append(identity)
    if {item["role"] for item in identities} != set(MUTATION_ORDER) or len(identities) != len(MUTATION_ORDER):
        fail("plan identity evidence must contain each canonical validator role exactly once")
    identities.sort(key=lambda item: MUTATION_ORDER.index(item["role"]))
    raw_proof_path_value = proof.get("sequencer_rebuild_proof_path")
    verification_path_value = proof.get("sequencer_rebuild_proof_verification_path")
    if not isinstance(raw_proof_path_value, str) or not raw_proof_path_value.strip():
        fail("plan signed 204 raw proof path is required")
    if verification_path_value:
        raw_proof_sha256 = raw_proof.get("proof_sha256")
        verification_sha256 = raw_proof.get("sha256")
    else:
        raw_proof_sha256 = raw_proof.get("sha256")
        verification_sha256 = None
    proof_file = _bound_regular_file(raw_proof_path_value, raw_proof_sha256, "plan signed 204 rebuild proof")
    proof_role = raw_proof.get("role")
    proof_node_id = raw_proof.get("node_id")
    proof_peer_id = raw_proof.get("peer_id")
    if proof_role != "sequencer-204" or not isinstance(proof_node_id, str) or not proof_node_id.strip() or not isinstance(proof_peer_id, str) or not proof_peer_id.strip():
        fail("plan signed 204 rebuild proof identity binding is incomplete")
    verification: dict[str, str] | None = None
    if verification_path_value:
        verification = _bound_regular_file(
            verification_path_value,
            verification_sha256,
            "plan signed 204 rebuild proof verification receipt",
        )
    proof_binding: dict[str, Any] = {
        **proof_file,
        "verification_path": verification["path"] if verification else None,
        "verification_sha256": verification["sha256"] if verification else None,
        "role": proof_role,
        "node_id": proof_node_id,
        "peer_id": proof_peer_id,
    }
    for key in ("signer_id", "signed_payload_sha256", "proof_sha256"):
        if key in raw_proof:
            proof_binding[key] = raw_proof[key]
    return {"identity_receipts": identities, "sequencer_rebuild_proof": proof_binding}


def _build_adapter_binding(plan: dict[str, Any], phase: str, phase_window_started_at: str) -> dict[str, Any]:
    plan_digest = plan.get("plan_digest")
    transaction_id = plan.get("transaction_id")
    if not isinstance(plan_digest, str) or len(plan_digest) != 64:
        fail("adapter binding requires the immutable plan digest")
    if not isinstance(transaction_id, str) or not transaction_id.strip():
        fail("adapter binding requires the transaction id")
    _parse_timestamp(phase_window_started_at, "adapter phase window")
    return {
        "schema_version": "oasis7.validator_pair_rebuild_adapter_binding.v1",
        "plan_digest": plan_digest,
        "transaction_id": transaction_id,
        "phase": phase,
        "phase_window_started_at": phase_window_started_at,
        "evidence_bindings": _expected_adapter_evidence(plan),
    }


def _validate_adapter_binding(receipt: dict[str, Any], plan: dict[str, Any], phase: str) -> None:
    binding = plan.get("adapter_binding")
    if not isinstance(binding, dict) or binding.get("schema_version") != "oasis7.validator_pair_rebuild_adapter_binding.v1":
        fail("host adapter binding was not persisted before invocation")
    if binding.get("plan_digest") != plan.get("plan_digest") or binding.get("transaction_id") != plan.get("transaction_id") or binding.get("phase") != phase:
        fail("host adapter binding identity mismatch")
    expected_evidence = _expected_adapter_evidence(plan)
    if binding.get("evidence_bindings") != expected_evidence:
        fail("persisted host adapter binding no longer matches plan evidence")
    if receipt.get("plan_digest") != binding["plan_digest"] or receipt.get("transaction_id") != binding["transaction_id"] or receipt.get("phase") != binding["phase"]:
        fail("host adapter receipt plan/transaction/phase binding mismatch")
    if receipt.get("evidence_bindings") != expected_evidence:
        fail("host adapter receipt evidence binding differs from the persisted plan evidence")
    captured_at = _parse_timestamp(receipt.get("captured_at"), "host adapter captured_at")
    phase_started = _parse_timestamp(binding["phase_window_started_at"], "adapter phase window")
    now = dt.datetime.now(dt.timezone.utc)
    if phase_started > now + dt.timedelta(seconds=5) or now - phase_started > dt.timedelta(seconds=300):
        fail("host adapter phase window is stale or in the future")
    if captured_at < phase_started or captured_at > now + dt.timedelta(seconds=5) or now - captured_at > dt.timedelta(seconds=300):
        fail("host adapter receipt freshness is outside the current phase window")


def apply_transaction(path: Path, host_adapter: Path | None = None) -> dict[str, Any]:
    plan = load_json(path, "transaction")
    if plan.get("schema_version") != PLAN_SCHEMA or plan.get("phase") != "planned":
        fail("apply requires a planned validator-pair rebuild plan")
    expected_plan_digest = hashlib.sha256(
        json.dumps({key: value for key, value in plan.items() if key != "plan_digest"}, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    if plan.get("plan_digest") != expected_plan_digest:
        fail("plan digest mismatch")
    if host_adapter is None:
        fail("apply requires a governed host adapter for startup and health gates")
    helper = load_provenance_helper()
    package = plan.get("package")
    try:
        helper.validate_receipt(Path(package["provenance"]), Path(package["directory"]), Path(plan["provenance"]["trusted_root"]["path"]))
    except SystemExit as error:
        fail(str(error).removeprefix("error: validator-pair provenance: "))
    transaction: dict[str, Any] = dict(plan)
    transaction["schema_version"] = SCHEMA
    transaction["transaction_id"] = f"pair-rebuild-{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())}-{uuid.uuid4().hex[:10]}"
    transaction["created_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    transaction["phase"] = "prepared"
    transaction["canonical_digest"] = canonical_digest(transaction)
    write_json(path, transaction)
    transaction["capacity_apply"] = {
        role: refresh_capacity(Path(plan["nodes"][role]["root"]), plan["capacity"][role], role)
        for role in MUTATION_ORDER
    }
    transaction["canonical_digest"] = canonical_digest(transaction)
    write_json(path, transaction)
    backups: dict[str, Any] = {}
    staged: dict[str, Any] = {}
    try:
        transaction["quiesce_receipt"] = run_host_adapter(host_adapter, path, transaction, "quiesce")
        transaction["phase"] = "quiesced"
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        for role in MUTATION_ORDER:
            backups[role] = snapshot_node(transaction["nodes"][role], transaction["transaction_id"])
            transaction["nodes"][role]["backup"] = backups[role]
        transaction["backup"] = backups
        transaction["backup_receipt"] = run_host_adapter(host_adapter, path, transaction, "backup")
        transaction["phase"] = "backed_up"
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        # Mutation is deliberately storage-205 then sequencer-204.
        for role in MUTATION_ORDER:
            staged[role] = install_local(transaction["nodes"][role], transaction, backups[role])
        transaction["staged"] = staged
        transaction["phase"] = "staged"
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        transaction["host_receipt"] = run_host_adapter(host_adapter, path, transaction, "apply")
        transaction["phase"] = "applied"
        transaction["applied_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        transaction["rollback"] = {"strategy": "same-filesystem-full-snapshot", "required_on_gate_failure": True, "status": "available"}
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        return transaction
    except BaseException as error:
        transaction["backup"] = backups
        transaction["staged"] = staged
        transaction["phase"] = "rollback_required"
        transaction["failure"] = str(error)
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        rollback_errors: list[str] = []
        try:
            transaction["rollback_receipt"] = run_host_adapter(host_adapter, path, transaction, "rollback")
        except BaseException as rollback_callback_error:
            rollback_errors.append(f"host-adapter: {rollback_callback_error}")
        for role in reversed(MUTATION_ORDER):
            if role in backups:
                try:
                    restore_node(backups[role], transaction.get("transaction_id"))
                except BaseException as rollback_error:
                    rollback_errors.append(f"{role}: {rollback_error}")
        transaction["phase"] = "rollback_failed" if rollback_errors else "rolled_back"
        transaction["rollback"] = {
            "strategy": "same-filesystem-full-snapshot",
            "status": "failed" if rollback_errors else "verified",
            "errors": rollback_errors,
        }
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        if rollback_errors:
            fail("rollback failed: " + "; ".join(rollback_errors))
        if isinstance(error, SystemExit):
            raise
        fail(str(error))


def rollback_transaction(path: Path, host_adapter: Path | None = None) -> dict[str, Any]:
    transaction = load_json(path, "transaction")
    if transaction.get("schema_version") != SCHEMA:
        fail("unsupported transaction schema")
    if transaction.get("canonical_digest") != canonical_digest(transaction):
        fail("transaction canonical digest mismatch")
    backups = transaction.get("backup")
    if not isinstance(backups, dict):
        fail("transaction does not contain full backup receipts")
    if host_adapter is None:
        fail("rollback requires a governed host adapter quiesce/rollback callback")
    transaction["phase"] = "rollback_required"
    transaction["canonical_digest"] = canonical_digest(transaction)
    write_json(path, transaction)
    try:
        transaction["rollback_quiesce_receipt"] = run_host_adapter(host_adapter, path, transaction, "quiesce")
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        results: dict[str, Any] = {}
        for role in reversed(MUTATION_ORDER):
            if role not in backups:
                fail(f"backup missing for {role}")
            results[role] = restore_node(backups[role], transaction.get("transaction_id"))
        transaction["rollback_receipt"] = run_host_adapter(host_adapter, path, transaction, "rollback")
        transaction["phase"] = "rolled_back"
        transaction["rollback"] = {"strategy": "same-filesystem-full-snapshot", "status": "verified", "results": results}
        transaction["rolled_back_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        return transaction
    except BaseException as error:
        transaction["phase"] = "rollback_failed"
        transaction["failure"] = str(error)
        transaction["rollback"] = {"strategy": "same-filesystem-full-snapshot", "status": "failed", "errors": [str(error)]}
        transaction["canonical_digest"] = canonical_digest(transaction)
        write_json(path, transaction)
        if isinstance(error, SystemExit):
            raise
        fail(str(error))


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="mode", required=True)
    plan = sub.add_parser("plan")
    plan.add_argument("--package-dir", required=True)
    plan.add_argument("--provenance", required=True)
    plan.add_argument("--trust-root", required=True)
    plan.add_argument("--consumer-impact-record", required=True)
    plan.add_argument("--capacity-json", required=True)
    plan.add_argument("--node", action="append", required=True)
    plan.add_argument("--storage-health-url")
    plan.add_argument("--sequencer-health-url")
    plan.add_argument("--sequencer-proof-url", required=True)
    plan.add_argument("--observer-receipt")
    plan.add_argument("--identity-receipts", required=True)
    plan.add_argument("--sequencer-rebuild-proof", required=True)
    plan.add_argument("--sequencer-rebuild-proof-verification")
    plan.add_argument("--sequencer-proof-verifier")
    plan.add_argument("--out-dir")
    apply = sub.add_parser("apply")
    apply.add_argument("--transaction", required=True)
    apply.add_argument("--host-adapter")
    rollback = sub.add_parser("rollback")
    rollback.add_argument("--transaction", required=True)
    rollback.add_argument("--host-adapter", required=True)
    return root


def main() -> int:
    args = parser().parse_args()
    if args.mode == "plan":
        plan = build_plan(args)
        if args.out_dir:
            write_json(Path(args.out_dir).resolve() / "transaction.json", plan)
        print(json.dumps(plan, ensure_ascii=True, sort_keys=True))
    elif args.mode == "apply":
        host_adapter = Path(args.host_adapter).expanduser() if args.host_adapter else None
        print(json.dumps(apply_transaction(Path(args.transaction).resolve(), host_adapter), ensure_ascii=True, sort_keys=True))
    else:
        print(json.dumps(rollback_transaction(Path(args.transaction).resolve(), Path(args.host_adapter).expanduser()), ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    main()
