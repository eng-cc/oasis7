#!/usr/bin/env python3
"""Fail-closed adapter boundary for the governed five-node clean-room plan.

The planner in ``p2p-public-testnet-full-network-clean-room.py`` is
intentionally provider-free.  This module is the equally deliberate boundary
between that plan and a separately governed provider transport.  It can write
only a local, durable transaction journal in dry-run mode.  A real transport,
an independently verified receipt, and an apply authority are required before
any mutating callback is even eligible to run.

The transport interface is intentionally tiny and data-oriented::

    inspect_node(node) -> read-only preflight evidence
    verify_fresh_root_probe(plan) -> authenticated probe receipt
    mutate(operation, node-or-None) -> sanitized operation receipt
    rollback_clean_redeploy(plan, started_operations) -> sanitized receipt

No shell command, credential, or provider-specific implementation belongs in
this file.  In particular, old-state restore and cross-node copy are not
transport operations accepted by this adapter.
"""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import ntpath
import os
from pathlib import Path, PurePosixPath
import posixpath
import re
import stat
import tempfile
from typing import Any, Callable, NoReturn


PLAN_SCHEMA = "oasis7.public_testnet_full_network_clean_room_plan.v1"
ADAPTER_SCHEMA = "oasis7.clean_room_mutation_adapter.v1"
AUTHORITY_SCHEMA = "oasis7.clean_room_mutation_authority.v1"
CRYPTO_RECEIPT_SCHEMA = "oasis7.crypto_verifier_receipt.v1"
JOURNAL_SCHEMA = "oasis7.clean_room_mutation_journal.v1"
NODE_RECEIPT_SCHEMA = "oasis7.clean_room_node_receipt.v1"
NONCE_ROW_SCHEMA = "oasis7.clean_room_adapter_nonce.v1"
REPOSITORY = "eng-cc/oasis7"
CANONICAL_ADAPTER_ID = "external-clean-room-adapter"
CANONICAL_NETWORK_ID = "oasis7-public-testnet-governed-20260606"
CANONICAL_VERIFIER_ID = "governed-receipt-verifier"
CANONICAL_TRUST_ROOT_ID = "oasis7-public-testnet-governance-root-v1"
CANONICAL_SIGNER_ALLOWLIST = frozenset({"governance-signer"})
OID_RE = re.compile(r"^[0-9a-fA-F]{40,64}$")
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
SAFE_NONCE_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{7,255}$")
SECRET_KEY_RE = re.compile(r"(?:password|secret|token|private[_-]?key|sshpass)", re.I)
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MIN_FREE_BYTES = 64 * 1024 * 1024

# This registry is code-owned.  It is not derived from plan input, a host
# response, or a caller-supplied peer list.
CANONICAL_PEER_REGISTRY = {
    "storage-205": "12D3KooWtriadtestnetstorage",
    "sequencer-204": "12D3KooWtriadtestnetsequencer",
    "linux-lan-observer": "12D3KooWtriadtestnetlocal",
    "windows-observer": "12D3KooWtriadtestnetwindowsobserver",
    "macos-observer": "12D3KooWtriadtestnetfourthlocal",
}


class AdapterError(RuntimeError):
    """A fail-closed adapter contract violation."""


def _fail(message: str) -> NoReturn:
    raise AdapterError(f"full-network clean-room adapter: {message}")


def _object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(f"{label} must be an object")
    return value


def _string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        _fail(f"{label} must be a non-empty string")
    return value


def _bool(value: Any, label: str) -> bool:
    if not isinstance(value, bool):
        _fail(f"{label} must be boolean")
    return value


def _digest(value: Any, label: str) -> str:
    raw = _string(value, label)
    if HEX64_RE.fullmatch(raw) is None:
        _fail(f"{label} must be a 64-character hexadecimal digest")
    return raw.lower()


def _oid(value: Any, label: str) -> str:
    raw = _string(value, label)
    if OID_RE.fullmatch(raw) is None:
        _fail(f"{label} must be a commit oid")
    return raw.lower()


def _nonzero_hex(value: Any, pattern: re.Pattern[str], label: str) -> str:
    raw = _string(value, label)
    if pattern.fullmatch(raw) is None or not any(character != "0" for character in raw):
        _fail(f"{label} is malformed or empty")
    return raw.lower()


def _canonical_bytes(value: dict[str, Any], *, omit: str | None = None) -> bytes:
    body = {key: item for key, item in value.items() if key != omit}
    return json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()


def canonical_plan_digest(plan: dict[str, Any]) -> str:
    """Return the planner-compatible digest, excluding ``plan_digest``."""
    return hashlib.sha256(_canonical_bytes(plan, omit="plan_digest")).hexdigest()


def journal_digest(record: dict[str, Any]) -> str:
    return hashlib.sha256(_canonical_bytes(record, omit="journal_digest")).hexdigest()


def _load_planner() -> Any:
    path = Path(__file__).with_name("p2p-public-testnet-full-network-clean-room.py")
    spec = importlib.util.spec_from_file_location("oasis7_full_network_clean_room_planner", path)
    if spec is None or spec.loader is None:
        _fail("cannot load the canonical full-network planner")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _safe_relative_paths(root: str, paths: Any, platform: str, label: str) -> list[str]:
    if not isinstance(paths, list) or not paths:
        _fail(f"{label} must be a non-empty path list")
    result: list[str] = []
    for raw in paths:
        path = _string(raw, f"{label} entry")
        if platform == "windows-x64":
            pieces = re.split(r"[\\/]", path)
            if ".." in pieces or not ntpath.normcase(ntpath.normpath(path)).startswith(
                ntpath.normcase(ntpath.normpath(root))
            ):
                _fail(f"{label} contains a path outside its canonical root")
        else:
            pieces = PurePosixPath(path).parts
            if ".." in pieces or not posixpath.normpath(path).startswith(posixpath.normpath(root).rstrip("/") + "/"):
                _fail(f"{label} contains a path outside its canonical root")
        result.append(path)
    return result


def _expected_paths(planner: Any, node: dict[str, Any]) -> list[str]:
    name = node["name"]
    path_style = "windows" if planner.EXPECTED_NODES[name]["platform"] == "windows-x64" else "posix"
    root = planner._normalized_path(
        planner.EXPECTED_NODES[name]["node_root"], path_style, f"{name}.expected_node_root"
    )
    surfaces = planner.VALIDATOR_RESET_SURFACES if name in planner.VALIDATOR_NAMES else planner.OBSERVER_RESET_SURFACES
    node_id = planner.EXPECTED_NODES[name]["node_id"]
    return [root.rstrip("/") + "/" + surface.replace("{node_id}", node_id).replace("\\", "/") for surface in surfaces]


def _under_root(root: str, path: str, platform: str) -> bool:
    if platform == "windows-x64":
        root_norm = ntpath.normcase(ntpath.normpath(root))
        path_norm = ntpath.normcase(ntpath.normpath(path))
        try:
            relative = ntpath.relpath(path_norm, root_norm)
            return relative not in ("..", ".") and not relative.startswith(".." + ntpath.sep)
        except ValueError:
            return False
    root_norm = posixpath.normpath(root)
    path_norm = posixpath.normpath(path)
    return path_norm != root_norm and path_norm.startswith(root_norm.rstrip("/") + "/")


def validate_plan(plan: dict[str, Any]) -> dict[str, Any]:
    """Validate immutable planner output and all code-owned inventories."""
    plan = _object(plan, "plan")
    if plan.get("schema_version") != PLAN_SCHEMA:
        _fail("plan schema is unsupported")
    actual_digest = _digest(plan.get("plan_digest"), "plan_digest")
    if actual_digest != canonical_plan_digest(plan):
        _fail("plan digest does not match the frozen plan contents")
    planner = _load_planner()
    if plan.get("node_order") != list(planner.NODE_ORDER):
        _fail("plan node order is not the code-owned five-node order")
    if plan.get("canonical_host_inventory") != planner.CANONICAL_HOST_INVENTORY:
        _fail("plan host inventory is not code-owned")
    if plan.get("canonical_endpoint_inventory") != planner.CANONICAL_ENDPOINT_INVENTORY:
        _fail("plan endpoint inventory is not code-owned")
    execution = _object(plan.get("execution"), "plan execution")
    if execution.get("mode") != "plan-only" or execution.get("provider_mutation_performed") is not False:
        _fail("plan is not an unperformed plan-only artifact")
    if (
        execution.get("plan_is_apply_proof") is not False
        or execution.get("apply_requires_fresh_adapter_receipt") is not True
    ):
        _fail("plan cannot be used as apply proof")
    surfaces = _object(plan.get("surfaces"), "plan surfaces")
    if surfaces.get("validators") != list(planner.VALIDATOR_RESET_SURFACES):
        _fail("validator reset surfaces are not the canonical eight")
    if surfaces.get("observers") != list(planner.OBSERVER_RESET_SURFACES):
        _fail("observer reset surfaces are not the canonical seven")
    nodes = plan.get("nodes")
    if (
        not isinstance(nodes, list)
        or {node.get("name") for node in nodes if isinstance(node, dict)} != set(planner.NODE_ORDER)
    ):
        _fail("plan does not contain exactly the canonical five nodes")
    by_name: dict[str, dict[str, Any]] = {}
    for node_value in nodes:
        node = _object(node_value, "plan node")
        name = _string(node.get("name"), "plan node name")
        if name in by_name:
            _fail("plan contains duplicate node names")
        by_name[name] = node
        expected = planner.EXPECTED_NODES[name]
        for field, expected_value in expected.items():
            if field == "node_root":
                expected_value = planner._normalized_path(
                    expected_value,
                    "windows" if expected["platform"] == "windows-x64" else "posix",
                    f"{name}.expected_node_root",
                )
            if node.get(field) != expected_value:
                _fail(f"{name} {field} is not the code-owned value")
        binding = _object(node.get("host_binding"), f"{name} host binding")
        if binding != planner.CANONICAL_HOST_INVENTORY[name]:
            _fail(f"{name} known-host target or pin is not code-owned")
        if _object(node.get("endpoints"), f"{name} endpoints") != planner.CANONICAL_ENDPOINT_INVENTORY[name]:
            _fail(f"{name} endpoint binding is not code-owned")
        identity = _object(node.get("identity_receipt"), f"{name} identity receipt")
        if identity.get("peer_id") != CANONICAL_PEER_REGISTRY[name]:
            _fail(f"{name} peer is not in the code-owned peer registry")
        if identity.get("authenticated") is not True or identity.get("verified") is not True:
            _fail(f"{name} identity receipt is not authenticated and verified")
        paths = _safe_relative_paths(
            node["node_root"],
            node.get("persistent_state_paths"),
            node["platform"],
            f"{name} state paths",
        )
        if paths != _expected_paths(planner, node):
            _fail(f"{name} state paths do not match its exact reset surfaces")
    forensic = _object(plan.get("forensic_backup"), "forensic backup")
    if (
        forensic.get("restore_old_state") is not False
        or forensic.get("cross_node_state_copy") is not False
        or forensic.get("seed_eligible") is not False
    ):
        _fail("old-state restore or cross-node copy is not an adapter operation")
    rollback = _object(plan.get("rollback"), "rollback")
    if (
        rollback.get("policy") != "clean-redeploy"
        or rollback.get("restore_old_state") is not False
        or rollback.get("cross_node_state_copy") is not False
    ):
        _fail("rollback is not clean-redeploy-only")
    global_order = plan.get("global_order")
    if not isinstance(global_order, list):
        _fail("plan global order is missing")
    try:
        planner._validate_global_order(global_order)
        expected_order = planner._global_order(bool(forensic.get("required_before_reset")))
    except SystemExit as error:
        _fail(str(error))
    if global_order != expected_order:
        _fail("plan global order is not deterministic for its backup mode")
    probe = _object(plan.get("fresh_root_probe"), "fresh-root probe")
    if (
        probe.get("transaction_id") != plan.get("transaction_id")
        or probe.get("capture_window_id") != plan.get("capture_window_id")
    ):
        _fail("fresh-root probe transaction binding is not exact")
    if probe.get("replayed") is not False or probe.get("post_validator_verify") is not True:
        _fail("fresh-root probe is replayed or lacks post-validator verification")
    return {"nodes": by_name, "planner": planner, "plan_digest": actual_digest}


def validate_authority(plan: dict[str, Any], authority: dict[str, Any]) -> dict[str, Any]:
    """Validate external authority without accepting caller-owned identities."""
    validate_plan(plan)
    authority = _object(authority, "adapter authority")
    if authority.get("schema_version") != AUTHORITY_SCHEMA:
        _fail("adapter authority schema is unsupported")
    expected = {
        "repository": REPOSITORY,
        "task_uid": plan["task_uid"],
        "frozen_head_oid": plan["head_oid"],
        "plan_digest": plan["plan_digest"],
        "adapter_id": CANONICAL_ADAPTER_ID,
        "network_id": CANONICAL_NETWORK_ID,
        "verifier_id": CANONICAL_VERIFIER_ID,
        "trust_root_id": CANONICAL_TRUST_ROOT_ID,
    }
    for field, expected_value in expected.items():
        if authority.get(field) != expected_value:
            _fail(f"adapter authority {field} is not bound to the frozen plan")
    if not isinstance(authority.get("apply_authorized"), bool):
        _fail("adapter authority apply_authorized must be an explicit boolean")
    receipt = _object(authority.get("receipt"), "adapter authority receipt")
    if receipt.get("schema_version") != CRYPTO_RECEIPT_SCHEMA:
        _fail("adapter authority receipt schema is unsupported")
    if receipt.get("authenticated") is not True or receipt.get("verified") is not True:
        _fail("adapter authority receipt is not independently authenticated and verified")
    if receipt.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST:
        _fail("adapter authority signer is not in the code-owned allowlist")
    if receipt.get("verifier_id") != CANONICAL_VERIFIER_ID or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID:
        _fail("adapter authority receipt verifier or trust root is not code-owned")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, "adapter authority signed payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, "adapter authority signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, "adapter authority canonical digest")
    bindings = _object(receipt.get("bindings"), "adapter authority receipt bindings")
    if (
        bindings.get("task_uid") != plan["task_uid"]
        or bindings.get("frozen_head_oid") != plan["head_oid"]
        or bindings.get("plan_digest") != plan["plan_digest"]
    ):
        _fail("adapter authority receipt head or plan binding drifted")
    if bindings.get("execution") != plan["truth"]["execution"]:
        _fail("adapter authority receipt execution provenance binding drifted")
    if bindings.get("forensic_backup") != plan["forensic_backup"]:
        _fail("adapter authority receipt backup authority binding drifted")
    if (
        bindings.get("package_commit") != plan["truth"]["package"]["commit"]
        or bindings.get("checkpoint_id") != plan["truth"]["checkpoint"]["checkpoint_id"]
        or bindings.get("checkpoint_manifest_hash")
        != plan["truth"]["checkpoint"]["manifest_hash"]
    ):
        _fail("adapter authority receipt package or checkpoint binding drifted")
    return {"apply_authorized": authority.get("apply_authorized") is True, "receipt": receipt}


def verify_repository_provenance_helper(
    plan: dict[str, Any], authority: dict[str, Any]
) -> dict[str, Any]:
    """Run the repository-owned detached provenance helper when requested.

    The helper path is code-owned; the receipt, package, and trust-root paths
    are merely references to externally provisioned evidence.  This function
    never accepts an executable path or a command from the authority envelope.
    """
    reference = _object(authority.get("provenance_helper"), "provenance helper reference")
    expected_helper = Path(__file__).with_name("p2p-public-testnet-validator-pair-provenance.py").resolve()
    helper_path = Path(_string(reference.get("helper_path"), "provenance helper path")).resolve()
    if helper_path != expected_helper:
        _fail("provenance helper path is not repository-owned")
    receipt_path = Path(_string(reference.get("receipt_path"), "provenance receipt path"))
    package_dir = Path(_string(reference.get("package_dir"), "provenance package path"))
    trust_root_path = Path(_string(reference.get("trust_root_path"), "provenance trust-root path"))
    if (
        receipt_path.is_symlink()
        or not receipt_path.is_file()
        or package_dir.is_symlink()
        or not package_dir.is_dir()
        or trust_root_path.is_symlink()
        or not trust_root_path.is_file()
    ):
        _fail("provenance evidence references must be present and non-symlinked")
    spec = importlib.util.spec_from_file_location("oasis7_repository_provenance_helper", helper_path)
    if spec is None or spec.loader is None:
        _fail("repository provenance helper cannot be loaded")
    helper = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(helper)
        result = helper.validate_receipt(receipt_path, package_dir, trust_root_path)
    except SystemExit:
        _fail("repository provenance helper rejected the evidence")
    except Exception as error:
        _fail(f"repository provenance helper failed: {error.__class__.__name__}")
    result = _object(result, "repository provenance helper result")
    package = _object(result.get("package"), "repository provenance package result")
    if (
        result.get("network_id") != CANONICAL_NETWORK_ID
        or result.get("chain_id") != CANONICAL_NETWORK_ID
        or package.get("commit") != plan["truth"]["package"]["commit"]
    ):
        _fail("repository provenance helper result is not bound to the plan truth")
    return {
        "verified": True,
        "verifier_id": CANONICAL_VERIFIER_ID,
        "trust_root_id": CANONICAL_TRUST_ROOT_ID,
        "signer_id": next(iter(CANONICAL_SIGNER_ALLOWLIST)),
        "bindings": authority["receipt"]["bindings"],
        "binding_digest": result.get("binding_digest"),
    }


def _node_nonce(plan: dict[str, Any], node: dict[str, Any]) -> str:
    seam = _object(node.get("credential_seam"), f"{node['name']} credential seam")
    nonce = _string(seam.get("nonce"), f"{node['name']} nonce")
    if SAFE_NONCE_RE.fullmatch(nonce) is None:
        _fail(f"{node['name']} nonce is malformed")
    return nonce


def _ledger_metadata(path: Path) -> os.stat_result:
    if path.is_symlink() or not path.is_file():
        _fail("credential nonce ledger must be an existing regular file")
    try:
        metadata = path.stat()
    except OSError:
        _fail("credential nonce ledger metadata is unavailable")
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
        _fail("credential nonce ledger owner or mode is invalid")
    try:
        path.resolve(strict=True).relative_to(REPOSITORY_ROOT)
    except ValueError:
        pass
    else:
        _fail("credential nonce ledger must be external to the repository")
    return metadata


def _read_ledger(path: Path) -> list[dict[str, Any]]:
    _ledger_metadata(path)
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        _fail("credential nonce ledger cannot be read")
    rows: list[dict[str, Any]] = []
    for number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            _fail(f"credential nonce ledger row {number} is malformed")
        row = _object(row, f"credential nonce ledger row {number}")
        if row.get("schema_version") != NONCE_ROW_SCHEMA or row.get("one_shot") is not True:
            _fail(f"credential nonce ledger row {number} is unsupported")
        _string(row.get("transaction_id"), f"credential nonce ledger row {number} transaction")
        nonce = _string(row.get("nonce"), f"credential nonce ledger row {number} nonce")
        if SAFE_NONCE_RE.fullmatch(nonce) is None:
            _fail(f"credential nonce ledger row {number} nonce is malformed")
        rows.append(row)
    return rows


def validate_credential_ledger(plan: dict[str, Any], path: Path) -> dict[str, int]:
    """Validate ownership, one-shot format, uniqueness, and replay state."""
    validate_plan(plan)
    rows = _read_ledger(Path(path))
    seen: set[str] = set()
    plan_nonces = {_node_nonce(plan, node) for node in plan["nodes"]}
    for row in rows:
        nonce = row["nonce"]
        if nonce in seen:
            _fail("credential nonce ledger contains a replayed nonce")
        seen.add(nonce)
        if nonce in plan_nonces and row["transaction_id"] == plan["transaction_id"]:
            _fail("credential nonce ledger already consumed a plan nonce")
    return {"rows": len(rows), "unique_nonces": len(seen)}


def reserve_nonce(path: Path, transaction_id: str, nonce: str) -> None:
    """Atomically append a one-shot reservation before remote observation."""
    path = Path(path)
    _ledger_metadata(path)
    if SAFE_NONCE_RE.fullmatch(nonce) is None:
        _fail("nonce is malformed")
    try:
        import fcntl

        with path.open("r+", encoding="utf-8") as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            rows = _read_ledger(path)
            if any(row["nonce"] == nonce for row in rows):
                _fail("credential nonce has already been consumed")
            row = {
                "schema_version": NONCE_ROW_SCHEMA,
                "transaction_id": _string(transaction_id, "transaction_id"),
                "nonce": nonce,
                "one_shot": True,
                "reserved_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
            }
            handle.seek(0, os.SEEK_END)
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True, separators=(",", ":")) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
            _fsync_parent(path.parent)
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
    except AdapterError:
        raise
    except (OSError, ImportError):
        _fail("credential nonce ledger cannot be atomically reserved")


def capacity_requirement(plan: dict[str, Any], node: dict[str, Any]) -> tuple[int, int]:
    platform = node["platform"]
    package = plan["truth"]["package"]["platforms"][platform]
    required_bytes = sum(
        int(value)
        for value in (
            package["package_size_bytes"],
            package["world_size_bytes"],
            package["world_provenance_size_bytes"],
            plan["truth"]["genesis"]["size_bytes"],
            plan["truth"]["world"]["size_bytes"],
            plan["truth"]["checkpoint"]["size_bytes"],
        )
    ) + MIN_FREE_BYTES
    return required_bytes, len(node["persistent_state_paths"]) + 16


def validate_remote_preflight(plan: dict[str, Any], node: dict[str, Any], evidence: dict[str, Any]) -> dict[str, Any]:
    """Validate read-only remote path, host pin, symlink, and capacity evidence."""
    validate_plan(plan)
    node = _object(node, "remote preflight node")
    evidence = _object(evidence, "remote preflight evidence")
    name = _string(node.get("name"), "remote preflight node name")
    if evidence.get("node") not in (None, name):
        _fail(f"{name} preflight evidence node binding drifted")
    if evidence.get("node_root") != node["node_root"]:
        _fail(f"{name} remote root is not canonical")
    paths = _safe_relative_paths(
        node["node_root"],
        evidence.get("persistent_state_paths"),
        node["platform"],
        f"{name} remote state paths",
    )
    if paths != node["persistent_state_paths"]:
        _fail(f"{name} remote state path inventory drifted")
    if evidence.get("symlink_free") is not True:
        _fail(f"{name} remote state contains a symlink")
    binding = node["host_binding"]
    if (
        evidence.get("host_target") != binding["target"]
        or evidence.get("known_hosts_path") != binding["known_hosts_path"]
        or evidence.get("known_host_fingerprint") != binding["known_host_fingerprint"]
    ):
        _fail(f"{name} known-host target or fingerprint is not pinned")
    if evidence.get("known_hosts_regular") is not True:
        _fail(f"{name} known-hosts file is not a regular file")
    if evidence.get("known_hosts_owner_uid") != os.getuid():
        _fail(f"{name} known-hosts owner is not the adapter owner")
    if evidence.get("known_hosts_mode") != "0600":
        _fail(f"{name} known-hosts mode is not 0600")
    required_bytes, required_inodes = capacity_requirement(plan, node)
    if evidence.get("required_bytes") != required_bytes or evidence.get("required_inodes") != required_inodes:
        _fail(f"{name} capacity requirement is not code-owned")
    if not isinstance(evidence.get("free_bytes"), int) or evidence["free_bytes"] < required_bytes:
        _fail(f"{name} remote byte capacity is insufficient")
    if not isinstance(evidence.get("free_inodes"), int) or evidence["free_inodes"] < required_inodes:
        _fail(f"{name} remote inode capacity is insufficient")
    return {
        "node": name,
        "node_root": node["node_root"],
        "path_count": len(paths),
        "symlink_free": True,
        "known_hosts_pinned": True,
        "capacity_verified": True,
    }


def _fsync_parent(path: Path) -> None:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    try:
        fd = os.open(str(path), flags)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    except OSError:
        _fail("durable fsync failed")


def _write_journal(path: Path, record: dict[str, Any]) -> None:
    path = Path(path)
    if path.is_symlink():
        _fail("transaction journal must not be a symlink")
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = dict(record)
    payload["journal_digest"] = journal_digest(payload)
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            delete=False,
        ) as handle:
            temporary = Path(handle.name)
            os.fchmod(handle.fileno(), 0o600)
            handle.write(json.dumps(payload, ensure_ascii=True, sort_keys=True, separators=(",", ":")) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        _fsync_parent(path.parent)
    except OSError:
        try:
            temporary.unlink(missing_ok=True)
        except (OSError, UnboundLocalError):
            pass
        _fail("transaction journal write failed")


def _read_journal(path: Path) -> dict[str, Any]:
    path = Path(path)
    if path.is_symlink() or not path.is_file():
        _fail("transaction journal must be an existing regular file")
    try:
        record = _object(json.loads(path.read_text(encoding="utf-8")), "transaction journal")
    except (OSError, json.JSONDecodeError):
        _fail("transaction journal is unreadable")
    if record.get("schema_version") != JOURNAL_SCHEMA or record.get("journal_digest") != journal_digest(record):
        _fail("transaction journal digest or schema is invalid")
    return record


def _journal_record(
    plan: dict[str, Any],
    status: str,
    next_index: int,
    completed: list[str],
    error: str | None = None,
    node_receipts: dict[str, dict[str, Any]] | None = None,
) -> dict[str, Any]:
    record: dict[str, Any] = {
        "schema_version": JOURNAL_SCHEMA,
        "adapter_schema": ADAPTER_SCHEMA,
        "task_uid": plan["task_uid"],
        "frozen_head_oid": plan["head_oid"],
        "plan_digest": plan["plan_digest"],
        "transaction_id": plan["transaction_id"],
        "capture_window_id": plan["capture_window_id"],
        "status": status,
        "next_operation_index": next_index,
        "completed_operations": list(completed),
        "operations": list(plan["global_order"]),
        "rollback_policy": "clean-redeploy",
        "restore_old_state": False,
        "cross_node_state_copy": False,
        "node_receipts": copy.deepcopy(node_receipts or {}),
    }
    if error is not None:
        record["terminal_error"] = error
    return record


def _validate_node_receipts(plan: dict[str, Any], raw: Any) -> dict[str, dict[str, Any]]:
    raw = _object(raw, "transaction journal node receipts")
    if set(raw) - set(plan["node_order"]):
        _fail("transaction journal contains an unknown node receipt")
    allowed_keys = {
        "schema_version",
        "node",
        "transaction_id",
        "capture_window_id",
        "plan_digest",
        "status",
        "last_operation",
        "operation_count",
        "rollback_policy",
    }
    result: dict[str, dict[str, Any]] = {}
    for name, value in raw.items():
        receipt = _object(value, f"{name} node receipt")
        if set(receipt) - allowed_keys:
            _fail(f"{name} node receipt contains an unsafe field")
        if (
            receipt.get("schema_version") != NODE_RECEIPT_SCHEMA
            or receipt.get("node") != name
            or receipt.get("transaction_id") != plan["transaction_id"]
            or receipt.get("capture_window_id") != plan["capture_window_id"]
            or receipt.get("plan_digest") != plan["plan_digest"]
            or receipt.get("rollback_policy") != "clean-redeploy"
        ):
            _fail(f"{name} node receipt is not bound to this transaction")
        result[name] = receipt
    return result


def _validate_live_probe(plan: dict[str, Any], receipt: dict[str, Any]) -> None:
    receipt = _object(receipt, "live fresh-root probe receipt")
    if (
        receipt.get("authenticated") is not True
        or receipt.get("verified") is not True
        or receipt.get("replayed") is True
    ):
        _fail("live fresh-root probe is not authenticated, verified, and fresh")
    if (
        receipt.get("transaction_id") != plan["transaction_id"]
        or receipt.get("capture_window_id") != plan["capture_window_id"]
    ):
        _fail("live fresh-root probe transaction binding drifted")
    _nonzero_hex(receipt.get("checkpoint_manifest_hash"), HEX64_RE, "live probe checkpoint manifest")
    if receipt.get("checkpoint_manifest_hash") != plan["truth"]["checkpoint"]["manifest_hash"]:
        _fail("live fresh-root probe checkpoint binding drifted")


def _verify_provenance(
    plan: dict[str, Any],
    authority: dict[str, Any],
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None,
) -> bool:
    receipt = authority["receipt"]
    if verifier is None:
        if "provenance_helper" not in authority:
            return False
        result = verify_repository_provenance_helper(plan, authority)
    else:
        try:
            result = verifier(plan, receipt)
        except Exception as error:
            _fail(f"external provenance verifier failed: {error.__class__.__name__}")
    result = _object(result, "external provenance verifier result")
    if (
        result.get("verified") is not True
        or result.get("bindings") != receipt["bindings"]
        or result.get("verifier_id") != CANONICAL_VERIFIER_ID
        or result.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
        or result.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
    ):
        _fail("external provenance verifier did not verify exact execution bindings")
    return True


def _sanitize_receipt(value: Any, label: str) -> dict[str, Any]:
    value = _object(value, label)
    cleaned: dict[str, Any] = {}
    for key, child in value.items():
        if SECRET_KEY_RE.search(str(key)):
            continue
        if key in {"nonce", "credential", "environment_name", "argv", "command", "command_line"}:
            continue
        if isinstance(child, dict):
            cleaned[key] = _sanitize_receipt(child, label)
        elif isinstance(child, list):
            cleaned[key] = [(_sanitize_receipt(item, label) if isinstance(item, dict) else item) for item in child]
        else:
            cleaned[key] = child
    return cleaned


def _transport_node(node: dict[str, Any]) -> dict[str, Any]:
    """Pass inventory only; credential seams never cross the adapter API."""
    result = copy.deepcopy(node)
    result.pop("credential_seam", None)
    return result


def _transport_plan(plan: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(plan)
    result.pop("credential_nonce_ledger", None)
    result["nodes"] = [_transport_node(node) for node in result["nodes"]]
    return result


def _mutating_operation(operation: str) -> bool:
    return operation.startswith(("forensic-backup:", "stop:", "delete:", "rebuild:", "start:"))


def execute(
    plan: dict[str, Any],
    authority: dict[str, Any],
    *,
    journal_path: Path,
    ledger_path: Path,
    transport: Any = None,
    dry_run: bool = True,
    provenance_verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Plan or execute a transaction through an explicitly injected adapter.

    ``dry_run`` defaults to true.  The non-dry path fails before any provider
    callback unless all independent authority, nonce, preflight, and receipt
    contracts are present.  The current repository does not supply a provider
    transport; this is intentional and keeps this adapter safe by default.
    """
    validated = validate_plan(plan)
    authority_summary = validate_authority(plan, authority)
    provenance_verified = _verify_provenance(plan, authority, provenance_verifier)
    ledger_summary = validate_credential_ledger(plan, Path(ledger_path))
    operations = list(plan["global_order"])
    if dry_run:
        receipts = {
            name: {
                "schema_version": NODE_RECEIPT_SCHEMA,
                "node": name,
                "transaction_id": plan["transaction_id"],
                "capture_window_id": plan["capture_window_id"],
                "plan_digest": plan["plan_digest"],
                "status": "planned",
                "operation_count": sum(1 for operation in operations if operation.endswith(name)),
                "rollback_policy": "clean-redeploy",
            }
            for name in plan["node_order"]
        }
        record = _journal_record(plan, "dry-run-complete", len(operations), operations, node_receipts=receipts)
        _write_journal(Path(journal_path), record)
        return {
            "schema_version": ADAPTER_SCHEMA,
            "status": "dry-run-complete",
            "dry_run": True,
            "resumed": False,
            "plan_digest": plan["plan_digest"],
            "transaction_id": plan["transaction_id"],
            "operations": operations,
            "ledger_rows": ledger_summary["rows"],
            "provider_mutation_performed": False,
            "nodes": {name: {"status": "planned", "receipt": receipts[name]} for name in plan["node_order"]},
        }
    if transport is None or not hasattr(transport, "mutate") or not hasattr(transport, "inspect_node"):
        _fail("apply requires an explicitly governed provider transport")
    if authority_summary["apply_authorized"] is not True:
        _fail("external apply authority is absent")
    if not provenance_verified:
        _fail("apply requires an independently executed provenance verifier")
    # One-shot reservations precede every remote observation.  Values stay in
    # the external ledger and never enter a journal, receipt, or log.
    _write_journal(Path(journal_path), _journal_record(plan, "prepared", 0, []))
    try:
        for node in (plan["nodes"] if isinstance(plan.get("nodes"), list) else []):
            reserve_nonce(Path(ledger_path), plan["transaction_id"], _node_nonce(plan, node))
        for node in plan["nodes"]:
            evidence = transport.inspect_node(_transport_node(node))
            validate_remote_preflight(plan, node, evidence)
    except Exception as error:
        _write_journal(
            Path(journal_path),
            _journal_record(plan, "terminal-failure", 0, [], error=error.__class__.__name__),
        )
        _fail("nonce reservation or remote preflight failed; transaction is terminal")
    completed: list[str] = []
    started: list[str] = []
    node_receipts: dict[str, dict[str, Any]] = {}
    for index, operation in enumerate(operations):
        _write_journal(
            Path(journal_path),
            _journal_record(plan, "in-flight", index, completed, node_receipts=node_receipts),
        )
        try:
            if operation == "fresh-root-probe":
                _validate_live_probe(plan, transport.verify_fresh_root_probe(_transport_plan(plan)))
            else:
                node_name = operation.partition(":")[2] or None
                node = next((item for item in plan["nodes"] if item["name"] == node_name), None)
                receipt = transport.mutate(
                    operation,
                    _transport_node(node) if node is not None else None,
                )
                if _mutating_operation(operation):
                    _sanitize_receipt(receipt, f"{operation} receipt")
                if node_name is not None and operation.startswith("verify:"):
                    _sanitize_receipt(receipt, f"{operation} receipt")
                if node_name is not None:
                    node_receipts[node_name] = {
                        "schema_version": NODE_RECEIPT_SCHEMA,
                        "node": node_name,
                        "transaction_id": plan["transaction_id"],
                        "capture_window_id": plan["capture_window_id"],
                        "plan_digest": plan["plan_digest"],
                        "status": "completed",
                        "last_operation": operation,
                        "operation_count": sum(
                            1 for completed_operation in completed + [operation]
                            if completed_operation.endswith(node_name)
                        ),
                        "rollback_policy": "clean-redeploy",
                    }
            completed.append(operation)
            _write_journal(
                Path(journal_path),
                _journal_record(plan, "running", index + 1, completed, node_receipts=node_receipts),
            )
            if operation.startswith(("start:", "rebuild:")):
                started.append(operation)
        except Exception as error:
            try:
                rollback_receipt = transport.rollback_clean_redeploy(
                    _transport_plan(plan), list(started)
                )
                _sanitize_receipt(rollback_receipt, "clean-redeploy rollback receipt")
            except Exception:
                pass
            _write_journal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "terminal-failure",
                    index,
                    completed,
                    error.__class__.__name__,
                    node_receipts,
                ),
            )
            _fail("provider operation failed; transaction is terminal and requires governed reconciliation")
    _write_journal(Path(journal_path), _journal_record(plan, "complete", len(operations), completed))
    return {
        "schema_version": ADAPTER_SCHEMA,
        "status": "complete",
        "dry_run": False,
        "resumed": False,
        "plan_digest": plan["plan_digest"],
        "transaction_id": plan["transaction_id"],
        "operations": operations,
        "provider_mutation_performed": True,
        "nodes": {name: {"status": "complete", "receipt": node_receipts.get(name)} for name in plan["node_order"]},
    }


def resume_transaction(
    plan: dict[str, Any],
    authority: dict[str, Any],
    journal_path: Path,
    *,
    ledger_path: Path,
    transport: Any = None,
    dry_run: bool = True,
    provenance_verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Resume only a complete/planned journal; ambiguous state is terminal."""
    validate_plan(plan)
    validate_authority(plan, authority)
    record = _read_journal(Path(journal_path))
    for field, expected in (
        ("task_uid", plan["task_uid"]),
        ("frozen_head_oid", plan["head_oid"]),
        ("plan_digest", plan["plan_digest"]),
        ("transaction_id", plan["transaction_id"]),
        ("capture_window_id", plan["capture_window_id"]),
    ):
        if record.get(field) != expected:
            _fail("transaction journal is bound to a different task, head, plan, or capture window")
    if record.get("operations") != plan["global_order"]:
        _fail("transaction journal operation order is not the frozen deterministic order")
    receipts = _validate_node_receipts(plan, record.get("node_receipts", {}))
    if (
        record.get("rollback_policy") != "clean-redeploy"
        or record.get("restore_old_state") is not False
        or record.get("cross_node_state_copy") is not False
    ):
        _fail("transaction journal contains an unsafe rollback policy")
    status = record.get("status")
    if status == "dry-run-complete" or status == "complete":
        return {
            "schema_version": ADAPTER_SCHEMA,
            "status": status,
            "resumed": True,
            "dry_run": status == "dry-run-complete",
            "plan_digest": plan["plan_digest"],
            "transaction_id": plan["transaction_id"],
            "operations": list(plan["global_order"]),
            "provider_mutation_performed": status == "complete",
            "nodes": {
                name: {"status": "already-complete", "receipt": receipts.get(name)}
                for name in plan["node_order"]
            },
        }
    if status == "in-flight":
        terminal = dict(record)
        terminal["status"] = "terminal-failure"
        terminal["terminal_error"] = "ambiguous in-flight journal requires governed reconciliation"
        _write_journal(Path(journal_path), terminal)
        _fail("transaction journal is ambiguous in-flight; governed reconciliation is required")
    if status == "terminal-failure":
        _fail("transaction journal is terminal-failure; governed reconciliation is required")
    if status in {"prepared", "running"} and dry_run:
        completed = record.get("completed_operations")
        next_index = record.get("next_operation_index")
        if (
            not isinstance(completed, list)
            or not isinstance(next_index, int)
            or completed != plan["global_order"][:next_index]
        ):
            _fail("transaction journal progress is not a deterministic operation prefix")
        _write_journal(
            Path(journal_path),
            _journal_record(
                plan,
                "dry-run-complete",
                len(plan["global_order"]),
                list(plan["global_order"]),
                node_receipts=receipts,
            ),
        )
        return {
            "schema_version": ADAPTER_SCHEMA,
            "status": "dry-run-complete",
            "resumed": True,
            "dry_run": True,
            "plan_digest": plan["plan_digest"],
            "transaction_id": plan["transaction_id"],
            "operations": list(plan["global_order"]),
            "provider_mutation_performed": False,
            "nodes": {
                name: {"status": "planned", "receipt": receipts.get(name)}
                for name in plan["node_order"]
            },
        }
    _fail("transaction journal status is not resumable")


def _load_json(path: Path, label: str) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file():
        _fail(f"{label} must be a regular file")
    try:
        return _object(json.loads(path.read_text(encoding="utf-8")), label)
    except (OSError, json.JSONDecodeError):
        _fail(f"{label} is unreadable")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Execute or dry-run a governed full-network clean-room adapter transaction"
    )
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--authority", required=True, type=Path)
    parser.add_argument("--journal", required=True, type=Path)
    parser.add_argument("--ledger", required=True, type=Path)
    parser.add_argument(
        "--apply",
        action="store_true",
        help="require an injected provider transport; otherwise fail closed",
    )
    args = parser.parse_args(argv)
    result = execute(
        _load_json(args.plan, "plan"),
        _load_json(args.authority, "authority"),
        journal_path=args.journal,
        ledger_path=args.ledger,
        dry_run=not args.apply,
    )
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
