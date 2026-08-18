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
import time
import uuid
from pathlib import Path
from typing import Any, NoReturn


SCHEMA = "oasis7.validator_pair_rebuild_transaction.v1"
MUTATION_ORDER = ["storage-205", "sequencer-204"]
STARTUP_ORDER = ["sequencer-204", "storage-205"]
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


def hash_tree(path: Path) -> tuple[str, int, int]:
    """Hash a tree without following symlinks and return digest, bytes, files."""
    digest = hashlib.sha256()
    total = 0
    count = 0
    if path.is_symlink():
        target = os.readlink(path)
        digest.update(f"L\0{target}\n".encode())
        return digest.hexdigest(), 0, 1
    if path.is_file():
        return sha256_file(path), path.stat().st_size, 1
    if not path.exists():
        return hashlib.sha256(b"<missing>").hexdigest(), 0, 0
    for child in sorted(path.rglob("*"), key=lambda item: item.relative_to(path).as_posix()):
        rel = child.relative_to(path).as_posix()
        if child.is_symlink():
            fail(f"governed tree contains symlink: {child}")
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
            count += 1
    return digest.hexdigest(), total, count


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


def capacity_for(capacity: dict[str, Any], role: str, required: int, required_inodes: int) -> dict[str, Any]:
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
    return {
        "free_bytes": free_bytes,
        "free_inodes": free_inodes,
        "same_filesystem": True,
        "required_bytes": required,
        "required_inodes": required_inodes,
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
        "verified": True,
    }


def reject_full_status(url: str | None, label: str) -> None:
    if url and "/v1/chain/status" in url:
        fail(f"204 {label} must use a bounded proof endpoint; full /v1/chain/status is forbidden")


def build_plan(args: argparse.Namespace) -> dict[str, Any]:
    package_dir = Path(args.package_dir).resolve()
    provenance_path = Path(args.provenance).resolve()
    helper = load_provenance_helper()
    try:
        provenance_summary = helper.validate_receipt(provenance_path, package_dir)
    except SystemExit as error:
        fail(str(error).removeprefix("error: validator-pair provenance: "))
    runtime_source = helper.find_runtime(package_dir)
    impact = validate_impact(Path(args.consumer_impact_record).resolve())
    nodes = parse_nodes(args.node)
    capacity = load_json(Path(args.capacity_json).resolve(), "capacity evidence")
    package_bytes = package_size(package_dir)
    governed = provenance_summary["governed"]
    governed_bytes = sum(int(item.get("size_bytes", item.get("total_bytes", 0))) for item in governed.values())
    capacities: dict[str, Any] = {}
    for role, node in nodes.items():
        for surface in RESET_SURFACES:
            if (Path(node["root"]) / surface).is_symlink():
                fail(f"reset surface must not be a symlink: {role}/{surface}")
        _, full_backup_bytes, full_backup_entries = hash_tree(Path(node["root"]))
        required = int((full_backup_bytes + package_bytes + governed_bytes) * 1.20) + 1
        governed_entries = sum(int(item.get("file_count", 1)) for item in governed.values())
        package_entries = hash_tree(package_dir)[2]
        required_inodes = max(128, int((full_backup_entries + package_entries + governed_entries) * 1.20) + 16)
        capacities[role] = capacity_for(capacity, role, required, required_inodes)
    reject_full_status(args.sequencer_proof_url, "proof URL")
    reject_full_status(args.sequencer_health_url, "health URL")
    txid = f"pair-rebuild-{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())}-{uuid.uuid4().hex[:10]}"
    plan: dict[str, Any] = {
        "schema_version": SCHEMA,
        "transaction_id": txid,
        "phase": "planned",
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
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
        },
        "observer_gate": {
            "status": "hold",
            "required_before_observer_mutation": True,
            "receipt": str(Path(args.observer_receipt).resolve()) if args.observer_receipt else None,
        },
        "rollback": {"strategy": "same-filesystem-full-snapshot", "required_on_gate_failure": True},
    }
    plan["canonical_digest"] = canonical_digest(plan)
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
    for key, value in plan["network"]["governed"].items():
        source = Path(value["path"])
        governed_destination = governed_root / key
        if governed_destination.exists() or governed_destination.is_symlink():
            remove_entry(governed_destination)
        copy_entry(source, governed_destination)
        actual_sha, total, _ = hash_tree(governed_destination)
        expected_sha = value.get("sha256") or value.get("sha256_tree")
        if actual_sha != expected_sha or total != int(value.get("size_bytes", value.get("total_bytes", 0))):
            fail(f"governed {key} identity mismatch after staging {node['role']}")
        staged[key] = str(governed_destination)
    return {
        "staged_release": str(release.parent),
        "runtime": str(runtime_destination),
        "governed": staged,
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


def validate_host_receipt(receipt: dict[str, Any], plan: dict[str, Any]) -> dict[str, Any]:
    if receipt.get("schema_version") != "oasis7.validator_pair_rebuild_host_receipt.v1":
        fail("host adapter returned an unsupported receipt schema")
    if receipt.get("transaction_id") != plan.get("transaction_id"):
        fail("host adapter receipt transaction identity mismatch")
    if receipt.get("mutation_order") != MUTATION_ORDER or receipt.get("startup_order") != STARTUP_ORDER:
        fail("host adapter receipt order mismatch")
    nodes = receipt.get("nodes")
    if not isinstance(nodes, dict) or set(nodes) != set(MUTATION_ORDER):
        fail("host adapter receipt must cover both validator roles")
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
    return receipt


def run_host_adapter(adapter: Path, transaction_path: Path, plan: dict[str, Any]) -> dict[str, Any]:
    if adapter.is_symlink() or not adapter.is_file():
        fail(f"host adapter must be a regular file: {adapter}")
    adapter = adapter.resolve()
    try:
        result = subprocess.run(
            [str(adapter), "--transaction", str(transaction_path)],
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
    return validate_host_receipt(receipt, plan)


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


def apply_transaction(path: Path, host_adapter: Path | None = None) -> dict[str, Any]:
    plan = load_json(path, "transaction")
    if plan.get("schema_version") != SCHEMA or plan.get("phase") != "planned":
        fail("apply requires a planned validator-pair transaction")
    if plan.get("canonical_digest") != canonical_digest(plan):
        fail("transaction canonical digest mismatch")
    if host_adapter is None:
        fail("apply requires a governed host adapter for startup and health gates")
    helper = load_provenance_helper()
    package = plan.get("package")
    try:
        helper.validate_receipt(Path(package["provenance"]), Path(package["directory"]))
    except SystemExit as error:
        fail(str(error).removeprefix("error: validator-pair provenance: "))
    plan["capacity_apply"] = {
        role: refresh_capacity(Path(plan["nodes"][role]["root"]), plan["capacity"][role], role)
        for role in MUTATION_ORDER
    }
    backups: dict[str, Any] = {}
    staged: dict[str, Any] = {}
    try:
        for role in MUTATION_ORDER:
            backups[role] = snapshot_node(plan["nodes"][role], plan["transaction_id"])
            plan["nodes"][role]["backup"] = backups[role]
        plan["backup"] = backups
        # Mutation is deliberately storage-205 then sequencer-204.
        for role in MUTATION_ORDER:
            staged[role] = install_local(plan["nodes"][role], plan, backups[role])
        plan["staged"] = staged
        plan["host_receipt"] = run_host_adapter(host_adapter, path, plan)
        plan["phase"] = "applied"
        plan["applied_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        plan["rollback"] = {"strategy": "same-filesystem-full-snapshot", "required_on_gate_failure": True, "status": "available"}
        plan["canonical_digest"] = canonical_digest(plan)
        write_json(path, plan)
        return plan
    except BaseException as error:
        plan["backup"] = backups
        plan["staged"] = staged
        plan["phase"] = "rollback_required"
        plan["failure"] = str(error)
        rollback_errors: list[str] = []
        for role in reversed(MUTATION_ORDER):
            if role in backups:
                try:
                    restore_node(backups[role], plan.get("transaction_id"))
                except BaseException as rollback_error:
                    rollback_errors.append(f"{role}: {rollback_error}")
        plan["phase"] = "rollback_failed" if rollback_errors else "rolled_back"
        plan["rollback"] = {
            "strategy": "same-filesystem-full-snapshot",
            "status": "failed" if rollback_errors else "verified",
            "errors": rollback_errors,
        }
        plan["canonical_digest"] = canonical_digest(plan)
        write_json(path, plan)
        if rollback_errors:
            fail("rollback failed: " + "; ".join(rollback_errors))
        if isinstance(error, SystemExit):
            raise
        fail(str(error))


def rollback_transaction(path: Path) -> dict[str, Any]:
    transaction = load_json(path, "transaction")
    if transaction.get("schema_version") != SCHEMA:
        fail("unsupported transaction schema")
    if transaction.get("canonical_digest") != canonical_digest(transaction):
        fail("transaction canonical digest mismatch")
    backups = transaction.get("backup")
    if not isinstance(backups, dict):
        fail("transaction does not contain full backup receipts")
    results: dict[str, Any] = {}
    for role in reversed(MUTATION_ORDER):
        if role not in backups:
            fail(f"backup missing for {role}")
        results[role] = restore_node(backups[role], transaction.get("transaction_id"))
    transaction["phase"] = "rolled_back"
    transaction["rollback"] = {"strategy": "same-filesystem-full-snapshot", "status": "verified", "results": results}
    transaction["rolled_back_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    transaction["canonical_digest"] = canonical_digest(transaction)
    write_json(path, transaction)
    return transaction


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="mode", required=True)
    plan = sub.add_parser("plan")
    plan.add_argument("--package-dir", required=True)
    plan.add_argument("--provenance", required=True)
    plan.add_argument("--consumer-impact-record", required=True)
    plan.add_argument("--capacity-json", required=True)
    plan.add_argument("--node", action="append", required=True)
    plan.add_argument("--storage-health-url")
    plan.add_argument("--sequencer-health-url")
    plan.add_argument("--sequencer-proof-url")
    plan.add_argument("--observer-receipt")
    plan.add_argument("--out-dir")
    apply = sub.add_parser("apply")
    apply.add_argument("--transaction", required=True)
    apply.add_argument("--host-adapter")
    rollback = sub.add_parser("rollback")
    rollback.add_argument("--transaction", required=True)
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
        print(json.dumps(rollback_transaction(Path(args.transaction).resolve()), ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    main()
