#!/usr/bin/env python3
"""Fail-closed adapter boundary for the governed five-node clean-room plan.

The planner in ``p2p-public-testnet-full-network-clean-room.py`` is
intentionally provider-free.  This module is the equally deliberate boundary
between that plan and a separately governed provider transport.  It can write
only a local, durable transaction journal in dry-run mode.  A real transport,
an independently verified receipt, and an apply authority are required before
any mutating callback is even eligible to run.

The transport interface is intentionally tiny and data-oriented::

    inspect_node(node) -> read-only preflight evidence plus a signed,
        independently verifier-bound receipt
    preflight(operation, node-or-None) -> authenticated preflight receipt
    verify(operation, node-or-None) -> authenticated verification receipt
    health(operation) -> authenticated fleet-health receipt
    verify_fresh_root_probe(plan) -> authenticated probe receipt
    mutate(operation, node-or-None) -> sanitized operation receipt
    reobserve_failed_state(plan, attempted_mutating_operations, failed_operation) -> signed receipt
    rollback_clean_redeploy(plan, attempted_mutating_operations, failed_state_receipt) -> sanitized receipt

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
PROVIDER_RECEIPT_SCHEMA = "oasis7.clean_room_provider_receipt.v1"
NO_BACKUP_AUTHORITY_SCHEMA = "oasis7.no_backup_authority.v1"
RECOVERY_RECEIPT_SCHEMA = "oasis7.recovery_receipt.v1"
IDENTITY_RECEIPT_SCHEMA = "oasis7.identity_receipt.v1"
JOURNAL_SCHEMA = "oasis7.clean_room_mutation_journal.v1"
NODE_RECEIPT_SCHEMA = "oasis7.clean_room_node_receipt.v1"
NONCE_ROW_SCHEMA = "oasis7.clean_room_adapter_nonce.v1"
REPOSITORY = "eng-cc/oasis7"
CANONICAL_ADAPTER_ID = "external-clean-room-adapter"
CANONICAL_NETWORK_ID = "oasis7-public-testnet-governed-20260606"
CANONICAL_VERIFIER_ID = "governed-receipt-verifier"
CANONICAL_TRUST_ROOT_ID = "oasis7-public-testnet-governance-root-v1"
CANONICAL_TRUST_ROOT_PATH = "/operator/truth/governance-root.json"
CANONICAL_TRUST_ROOT_DIGEST = hashlib.sha256(
    f"{CANONICAL_TRUST_ROOT_ID}:{CANONICAL_TRUST_ROOT_PATH}".encode()
).hexdigest()
# Deployment supplies this code-owned digest for the pinned regular file;
# tests validate the path/content/owner/mode contract without reading a live
# operator filesystem.
CANONICAL_TRUST_ROOT_OWNER_UID = os.getuid()
CANONICAL_TRUST_ROOT_MODE = "0600"
CANONICAL_SIGNER_ALLOWLIST = frozenset({"governance-signer"})
CANONICAL_PROBE_PEER_ID = "validator-pair"
CANONICAL_FLEET_PEER_ID = "fleet"
PHASE_RECEIPT_SCHEMAS = {
    "preflight": "oasis7.clean_room_preflight_receipt.v1",
    "backup": "oasis7.clean_room_backup_receipt.v1",
    "apply": "oasis7.clean_room_apply_receipt.v1",
    "verify": "oasis7.clean_room_verify_receipt.v1",
    "fresh-root-probe": "oasis7.clean_room_fresh_root_probe_receipt.v1",
    "fleet-health": "oasis7.clean_room_fleet_health_receipt.v1",
    "reobserve": "oasis7.clean_room_reobserve_receipt.v1",
    "rollback": "oasis7.clean_room_rollback_receipt.v1",
}
CANONICAL_PROVIDER_UID = {
    "storage-205": 0,
    "sequencer-204": 0,
    "linux-lan-observer": 0,
    "windows-observer": 0,
    "macos-observer": 0,
}
OID_RE = re.compile(r"^[0-9a-fA-F]{40,64}$")
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
SAFE_NONCE_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{7,255}$")
SECRET_KEY_RE = re.compile(r"(?:password|secret|token|private[_-]?key|sshpass)", re.I)
SECRET_FIELD_NAMES = frozenset(
    {"nonce", "credential", "credentials", "environment_name", "argv", "command", "command_line"}
)
SECRET_VALUE_RE = re.compile(r"(?:password|secret|token|private[_ -]?key|sshpass)", re.I)
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

# Provider callbacks receive only these DTO fields.  Keeping the allowlist in
# the adapter makes a future planner field opt-in rather than an accidental
# transport disclosure.
TRANSPORT_NODE_FIELDS = frozenset(
    {
        "name",
        "node_id",
        "role",
        "platform",
        "node_root",
        "service_manager",
        "service",
        "host_binding",
        "endpoints",
        "persistent_state_paths",
        "identity_receipt",
        "bindings",
    }
)
TRANSPORT_PLAN_FIELDS = frozenset(
    {
        "schema_version",
        "task_uid",
        "head_oid",
        "plan_digest",
        "transaction_id",
        "capture_window_id",
        "capture_window",
        "node_order",
        "global_order",
        "canonical_host_inventory",
        "canonical_endpoint_inventory",
        "nodes",
        "surfaces",
        "truth",
        "execution",
        "forensic_backup",
        "rollback",
        "fresh_root_probe",
        "observer_gate",
        "operation_journal",
        "operation_journal_contract",
        "adapter_verification",
    }
)


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
        if set(identity) - {
            "schema_version", "authenticated", "verified", "signer_id", "verifier_id",
            "trust_root_id", "signed_payload_sha256", "signature_hex", "canonical_digest",
            "node_id", "peer_id", "key_sha256", "key_size_bytes", "key_mode", "key_uid", "key_gid",
        }:
            _fail(f"{name} identity receipt contains an unsafe field")
        if (
            identity.get("schema_version") != IDENTITY_RECEIPT_SCHEMA
            or identity.get("authenticated") is not True
            or identity.get("verified") is not True
            or identity.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
            or identity.get("verifier_id") != CANONICAL_VERIFIER_ID
            or identity.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
            or identity.get("node_id") != node["node_id"]
        ):
            _fail(f"{name} identity receipt is not independently authenticated")
        _reject_secret_fields(identity, f"{name} identity receipt")
        _nonzero_hex(identity.get("signed_payload_sha256"), HEX64_RE, f"{name} identity payload")
        _nonzero_hex(identity.get("signature_hex"), SIGNATURE_RE, f"{name} identity signature")
        _nonzero_hex(identity.get("canonical_digest"), HEX64_RE, f"{name} identity digest")
        if identity.get("peer_id") != CANONICAL_PEER_REGISTRY[name]:
            _fail(f"{name} peer is not in the code-owned peer registry")
        if identity.get("key_uid") != CANONICAL_PROVIDER_UID[name]:
            _fail(f"{name} provider identity uid is not code-owned")
        key_size = identity.get("key_size_bytes")
        if not isinstance(key_size, int) or isinstance(key_size, bool) or key_size <= 0:
            _fail(f"{name} identity key size is malformed")
        _nonzero_hex(identity.get("key_sha256"), HEX64_RE, f"{name} identity key digest")
        if identity.get("key_mode") != "0600":
            _fail(f"{name} identity key mode is not 0600")
        for owner in ("key_uid", "key_gid"):
            value = identity.get(owner)
            if not isinstance(value, int) or isinstance(value, bool) or value < 0:
                _fail(f"{name} identity {owner} is malformed")
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
    mode = forensic.get("mode")
    if (
        forensic.get("task_uid") != plan["task_uid"]
        or forensic.get("frozen_head_oid") != plan["head_oid"]
    ):
        _fail("forensic backup task or frozen-head binding drifted")
    if mode not in {"forensic-backup", "operator-authorized-no-backup"}:
        _fail("forensic backup mode is unsupported")
    if mode == "forensic-backup":
        if (
            forensic.get("required_before_reset") is not True
            or forensic.get("immutable") is not True
            or forensic.get("receipt_required_per_node") is not True
            or forensic.get("operator_authorized") is not False
            or forensic.get("current_authorization") is not False
            or forensic.get("authority") is not None
        ):
            _fail("forensic-backup mode has an unsafe authority or reset combination")
    else:
        if (
            forensic.get("required_before_reset") is not False
            or forensic.get("immutable") is not False
            or forensic.get("receipt_required_per_node") is not False
            or forensic.get("operator_authorized") is not True
            or forensic.get("current_authorization") is not True
        ):
            _fail("operator-authorized-no-backup mode has an unsafe reset combination")
    rollback = _object(plan.get("rollback"), "rollback")
    if (
        rollback.get("policy") != "clean-redeploy"
        or rollback.get("restore_old_state") is not False
        or rollback.get("cross_node_state_copy") is not False
    ):
        _fail("rollback is not clean-redeploy-only")
    ledger_contract = _object(plan.get("credential_nonce_ledger"), "credential nonce ledger contract")
    ledger_path = _string(ledger_contract.get("path"), "credential nonce ledger path")
    if not Path(ledger_path).is_absolute():
        _fail("credential nonce ledger path must be absolute")
    ledger_receipt = _object(ledger_contract.get("receipt"), "credential nonce ledger receipt")
    if _object(ledger_receipt.get("bindings"), "credential nonce ledger receipt bindings").get("path") != ledger_path:
        _fail("credential nonce ledger receipt path binding drifted")
    capture_window = _object(plan.get("capture_window"), "transaction capture window")
    if set(capture_window) != {"id", "starts_at", "ends_at"}:
        _fail("transaction capture window contains an unsafe field")
    if capture_window.get("id") != plan["capture_window_id"]:
        _fail("transaction capture window id binding drifted")
    window_start = _parse_utc(capture_window.get("starts_at"), "transaction capture window starts_at")
    window_end = _parse_utc(capture_window.get("ends_at"), "transaction capture window ends_at")
    if window_end <= window_start:
        _fail("transaction capture window is inverted")
    if (
        capture_window.get("starts_at") != ledger_contract.get("issued_at")
        or capture_window.get("ends_at") != ledger_contract.get("expires_at")
    ):
        _fail("transaction capture window is not bound to the nonce ledger lease")
    for node in nodes:
        seam = _object(node.get("credential_seam"), f"{node['name']} credential seam")
        if seam.get("ledger_path") != ledger_path:
            _fail(f"{node['name']} credential ledger path is not the canonical path")
    journal_contract = _object(
        plan.get("operation_journal_contract"), "operation journal contract"
    )
    if (
        journal_contract.get("authoritative") is not False
        or journal_contract.get("apply_usable") is not False
        or journal_contract.get("adapter_owned") is not True
        or journal_contract.get("durable_receipt_required") is not True
        or journal_contract.get("planner_output_is_not_apply_proof") is not True
    ):
        _fail("operation journal contract is not the adapter-owned fail-closed contract")
    adapter_verification = _object(
        plan.get("adapter_verification"), "planner adapter verification"
    )
    if (
        adapter_verification.get("schema_version") != "oasis7.clean_room_adapter_verification.v1"
        or adapter_verification.get("adapter_id") != CANONICAL_ADAPTER_ID
        or adapter_verification.get("transaction_id") != plan["transaction_id"]
        or adapter_verification.get("capture_window_id") != plan["capture_window_id"]
        or adapter_verification.get("live_receipts_verified") is not True
        or adapter_verification.get("credential_nonce_ledger_verified") is not True
        or adapter_verification.get("backup_or_no_backup_authority_verified") is not True
        or adapter_verification.get("apply_authority_granted") is not False
        or adapter_verification.get("durable_journal_authoritative") is not False
        or adapter_verification.get("durable_journal_receipt_required") is not True
    ):
        _fail("planner adapter verification does not grant safe apply prerequisites")
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
    _validate_no_backup_authority(plan)
    return {"nodes": by_name, "planner": planner, "plan_digest": actual_digest}


def _parse_utc(value: Any, label: str) -> dt.datetime:
    raw = _string(value, label)
    try:
        parsed = dt.datetime.fromisoformat(raw.replace("Z", "+00:00"))
    except ValueError:
        _fail(f"{label} must be an ISO-8601 timestamp")
    if parsed.tzinfo is None:
        _fail(f"{label} must include a timezone")
    return parsed.astimezone(dt.timezone.utc)


def _capture_window_bounds(plan: dict[str, Any]) -> tuple[dt.datetime, dt.datetime]:
    window = _object(plan.get("capture_window"), "transaction capture window")
    if window.get("id") != plan["capture_window_id"]:
        _fail("transaction capture window id binding drifted")
    start = _parse_utc(window.get("starts_at"), "transaction capture window starts_at")
    end = _parse_utc(window.get("ends_at"), "transaction capture window ends_at")
    if end <= start:
        _fail("transaction capture window is inverted")
    return start, end


def _validate_no_backup_authority(plan: dict[str, Any]) -> None:
    """Require a current, signed operator decision when no backup is used."""
    forensic = _object(plan.get("forensic_backup"), "forensic backup")
    mode = forensic.get("mode")
    authority = forensic.get("authority")
    if mode != "operator-authorized-no-backup":
        if authority is not None:
            _fail("forensic backup authority is only valid in no-backup mode")
        return
    if authority is None:
        _fail("operator-authorized-no-backup requires a signed current authority")
    if forensic.get("operator_authorized") is not True:
        _fail("no-backup mode is not operator-authorized")
    if forensic.get("current_authorization") is not True:
        _fail("no-backup mode lacks current authorization")
    if forensic.get("repository") != REPOSITORY or forensic.get("action") != "full-network-clean-room":
        _fail("no-backup authority repository or action is not governed")
    if forensic.get("targets") != list(plan["node_order"]):
        _fail("no-backup authority targets are not the deterministic managed-node set")
    for field in ("transaction_id", "capture_window_id", "actor", "issued_at", "expires_at"):
        if field not in forensic:
            _fail(f"no-backup authority is missing {field}")
    if forensic["transaction_id"] != plan["transaction_id"] or forensic["capture_window_id"] != plan["capture_window_id"]:
        _fail("no-backup authority transaction binding drifted")
    actor = _string(forensic["actor"], "no-backup authority actor")
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{1,127}", actor):
        _fail("no-backup authority actor is malformed")
    issued_at = _parse_utc(forensic["issued_at"], "no-backup authority issued_at")
    expires_at = _parse_utc(forensic["expires_at"], "no-backup authority expires_at")
    if issued_at > dt.datetime.now(dt.timezone.utc) or expires_at <= issued_at or expires_at <= dt.datetime.now(dt.timezone.utc):
        _fail("no-backup authority is expired or inverted")
    receipt = _object(authority, "no-backup authority receipt")
    if receipt.get("schema_version") != NO_BACKUP_AUTHORITY_SCHEMA:
        _fail("no-backup authority receipt schema is unsupported")
    if receipt.get("authenticated") is not True or receipt.get("verified") is not True:
        _fail("no-backup authority receipt is not authenticated and verified")
    if receipt.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST:
        _fail("no-backup authority signer is not code-owned")
    if receipt.get("verifier_id") != CANONICAL_VERIFIER_ID or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID:
        _fail("no-backup authority verifier or trust root is not code-owned")
    allowed = {
        "schema_version", "authenticated", "verified", "signer_id", "verifier_id", "trust_root_id",
        "signed_payload_sha256", "signature_hex", "canonical_digest", "bindings",
    }
    if set(receipt) - allowed:
        _fail("no-backup authority receipt contains an unsafe field")
    _reject_secret_fields(receipt, "no-backup authority receipt")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, "no-backup authority signed payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, "no-backup authority signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, "no-backup authority canonical digest")
    expected_bindings = {
        "repository": REPOSITORY,
        "action": "full-network-clean-room",
        "targets": list(plan["node_order"]),
        "task_uid": plan["task_uid"],
        "transaction_id": plan["transaction_id"],
        "capture_window_id": plan["capture_window_id"],
        "frozen_head_oid": plan["head_oid"],
        "actor": actor,
        "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
        "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
        "current_authorization": True,
    }
    if _object(receipt.get("bindings"), "no-backup authority bindings") != expected_bindings:
        _fail("no-backup authority receipt bindings are not exact")


def validate_authority(plan: dict[str, Any], authority: dict[str, Any]) -> dict[str, Any]:
    """Validate external authority without accepting caller-owned identities."""
    validate_plan(plan)
    authority = _object(authority, "adapter authority")
    _reject_secret_fields(authority, "adapter authority")
    if set(authority) - {
        "schema_version", "repository", "task_uid", "frozen_head_oid", "plan_digest",
        "adapter_id", "network_id", "verifier_id", "trust_root_id", "trust_root_path",
        "trust_root_digest", "trust_root_file", "apply_authorized", "receipt", "provenance_helper",
    }:
        _fail("adapter authority contains an unsafe field")
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
        "trust_root_path": CANONICAL_TRUST_ROOT_PATH,
        "trust_root_digest": CANONICAL_TRUST_ROOT_DIGEST,
    }
    for field, expected_value in expected.items():
        if authority.get(field) != expected_value:
            _fail(f"adapter authority {field} is not bound to the frozen plan")
    trust_root_file = _object(authority.get("trust_root_file"), "pinned trust-root file contract")
    if set(trust_root_file) != {"path", "sha256", "owner_uid", "mode", "regular_file"}:
        _fail("pinned trust-root file contract contains an unsafe field")
    if (
        trust_root_file.get("path") != CANONICAL_TRUST_ROOT_PATH
        or trust_root_file.get("sha256") != CANONICAL_TRUST_ROOT_DIGEST
        or trust_root_file.get("owner_uid") != CANONICAL_TRUST_ROOT_OWNER_UID
        or trust_root_file.get("mode") != CANONICAL_TRUST_ROOT_MODE
        or trust_root_file.get("regular_file") is not True
    ):
        _fail("pinned trust-root file path, content digest, owner, or mode drifted")
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
    if set(receipt) - {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "signed_payload_sha256",
        "signature_hex",
        "canonical_digest",
        "verifier_id",
        "trust_root_id",
        "bindings",
    }:
        _fail("adapter authority receipt contains an unsafe field")
    _reject_secret_fields(receipt, "adapter authority receipt")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, "adapter authority signed payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, "adapter authority signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, "adapter authority canonical digest")
    bindings = _object(receipt.get("bindings"), "adapter authority receipt bindings")
    expected_bindings = {
        "task_uid": plan["task_uid"],
        "frozen_head_oid": plan["head_oid"],
        "plan_digest": plan["plan_digest"],
        "execution": plan["truth"]["execution"],
        "ledger_path": plan["credential_nonce_ledger"]["path"],
        "apply_authorized": authority["apply_authorized"],
        "forensic_backup": plan["forensic_backup"],
        "package_commit": plan["truth"]["package"]["commit"],
        "checkpoint_id": plan["truth"]["checkpoint"]["checkpoint_id"],
        "checkpoint_manifest_hash": plan["truth"]["checkpoint"]["manifest_hash"],
        "trust_root_path": CANONICAL_TRUST_ROOT_PATH,
        "trust_root_digest": CANONICAL_TRUST_ROOT_DIGEST,
        "trust_root_file": copy.deepcopy(trust_root_file),
    }
    if bindings != expected_bindings:
        _fail("adapter authority receipt binding set is not exact")
    return {"apply_authorized": authority.get("apply_authorized") is True, "receipt": receipt}


def validate_live_trust_root_file() -> dict[str, Any]:
    """Check the code-owned trust-root file at the apply boundary.

    The authority envelope is not sufficient: an apply must observe the live
    regular file immediately before any provider operation.  Path, content,
    owner, and mode are deployment-pinned constants, never caller inputs.
    """
    path = Path(CANONICAL_TRUST_ROOT_PATH)
    try:
        link_metadata = path.lstat()
    except OSError:
        _fail("code-owned trust-root file is unavailable")
    if not stat.S_ISREG(link_metadata.st_mode):
        _fail("code-owned trust-root file is not a regular file")
    descriptor: int | None = None
    try:
        descriptor = os.open(
            str(path), os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
        )
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode):
            _fail("code-owned trust-root file is not a regular file")
        if metadata.st_uid != CANONICAL_TRUST_ROOT_OWNER_UID:
            _fail("code-owned trust-root file owner drifted")
        if stat.S_IMODE(metadata.st_mode) != int(CANONICAL_TRUST_ROOT_MODE, 8):
            _fail("code-owned trust-root file mode drifted")
        digest_builder = hashlib.sha256()
        while True:
            chunk = os.read(descriptor, 1024 * 1024)
            if not chunk:
                break
            digest_builder.update(chunk)
        digest = digest_builder.hexdigest()
    except OSError:
        _fail("code-owned trust-root file content is unreadable")
    finally:
        if descriptor is not None:
            os.close(descriptor)
    if digest != CANONICAL_TRUST_ROOT_DIGEST:
        _fail("code-owned trust-root file content digest drifted")
    return {
        "path": CANONICAL_TRUST_ROOT_PATH,
        "sha256": digest,
        "owner_uid": metadata.st_uid,
        "mode": CANONICAL_TRUST_ROOT_MODE,
        "regular_file": True,
    }


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
    if trust_root_path != Path(CANONICAL_TRUST_ROOT_PATH):
        _fail("provenance trust-root path is not the code-owned path")
    if reference.get("trust_root_digest") != CANONICAL_TRUST_ROOT_DIGEST:
        _fail("provenance trust-root digest is not code-owned")
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
        if nonce in plan_nonces:
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


def _remote_evidence_digest(evidence: dict[str, Any]) -> str:
    """Digest only the read-only evidence, excluding its signed receipt."""
    return hashlib.sha256(_canonical_bytes(evidence, omit="receipt")).hexdigest()


def validate_remote_preflight(
    plan: dict[str, Any],
    node: dict[str, Any],
    evidence: dict[str, Any],
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Validate pinned remote evidence and its signed verifier-bound receipt."""
    validate_plan(plan)
    node = _object(node, "remote preflight node")
    evidence = _object(evidence, "remote preflight evidence")
    if set(evidence) - {
        "node",
        "node_id",
        "provider_uid",
        "node_root",
        "persistent_state_paths",
        "symlink_free",
        "free_bytes",
        "required_bytes",
        "free_inodes",
        "required_inodes",
        "host_target",
        "known_hosts_path",
        "known_host_fingerprint",
        "known_hosts_regular",
        "known_hosts_owner_uid",
        "known_hosts_mode",
        "receipt",
    }:
        _fail("remote preflight evidence contains an unsafe field")
    _reject_secret_fields(evidence, "remote preflight evidence")
    name = _string(node.get("name"), "remote preflight node name")
    if evidence.get("node") != name:
        _fail(f"{name} preflight evidence node binding drifted")
    if evidence.get("node_id") != node["node_id"]:
        _fail(f"{name} preflight identity binding drifted")
    if evidence.get("provider_uid") != CANONICAL_PROVIDER_UID[name]:
        _fail(f"{name} provider uid is not the governed uid")
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
    if verifier is None:
        _fail(f"{name} remote preflight requires an independent receipt verifier")
    receipt = _validate_provider_receipt(
        plan,
        f"preflight:{name}",
        name,
        evidence.get("receipt"),
        verifier,
        evidence=evidence,
    )
    return {
        "node": name,
        "node_root": node["node_root"],
        "path_count": len(paths),
        "symlink_free": True,
        "known_hosts_pinned": True,
        "capacity_verified": True,
        "receipt": receipt,
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


def _reject_symlink_ancestors(path: Path) -> None:
    """Reject symlinked path components before creating or opening a journal."""
    raw_parts = Path(os.fspath(path)).parts
    if ".." in raw_parts:
        _fail("transaction journal path must not contain parent traversal")
    absolute = Path(os.path.abspath(os.fspath(path)))
    # macOS exposes /var as a system alias for /private/var.  Normalize that
    # host-owned alias without resolving any caller-controlled descendant.
    if (
        len(absolute.parts) > 1
        and absolute.parts[1] == "var"
        and os.path.realpath("/var") == "/private/var"
    ):
        absolute = Path(os.path.realpath("/var")).joinpath(*absolute.parts[2:])
    current = Path(absolute.anchor or os.curdir)
    for component in absolute.parts[1:]:
        current /= component
        try:
            metadata = os.lstat(current)
        except FileNotFoundError:
            # No later component can exist below a missing ancestor.
            break
        except OSError:
            _fail("transaction journal path metadata is unavailable")
        if stat.S_ISLNK(metadata.st_mode):
            _fail("transaction journal path must not contain a symlink ancestor")
        if current != absolute and not stat.S_ISDIR(metadata.st_mode):
            _fail("transaction journal path ancestor is not a directory")


def _write_journal(path: Path, record: dict[str, Any]) -> None:
    path = Path(path)
    _reject_symlink_ancestors(path)
    if path.is_symlink():
        _fail("transaction journal must not be a symlink")
    if path.exists():
        try:
            metadata = path.stat()
        except OSError:
            _fail("transaction journal metadata is unavailable")
        if (
            not stat.S_ISREG(metadata.st_mode)
            or metadata.st_uid != os.getuid()
            or stat.S_IMODE(metadata.st_mode) != 0o600
        ):
            _fail("transaction journal owner or mode is invalid")
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
    _reject_symlink_ancestors(path)
    if path.is_symlink() or not path.is_file():
        _fail("transaction journal must be an existing regular file")
    try:
        metadata = path.stat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or metadata.st_uid != os.getuid()
            or stat.S_IMODE(metadata.st_mode) != 0o600
        ):
            _fail("transaction journal owner or mode is invalid")
        record = _object(json.loads(path.read_text(encoding="utf-8")), "transaction journal")
    except (OSError, json.JSONDecodeError):
        _fail("transaction journal is unreadable")
    if record.get("schema_version") != JOURNAL_SCHEMA or record.get("journal_digest") != journal_digest(record):
        _fail("transaction journal digest or schema is invalid")
    return record


def _acquire_transaction_lock(journal_path: Path) -> Any:
    """Serialize a transaction and leave the lock durable for inspection."""
    lock_path = Path(f"{journal_path}.lock")
    _reject_symlink_ancestors(lock_path)
    if lock_path.is_symlink():
        _fail("transaction lock must not be a symlink")
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        import fcntl

        flags = os.O_CREAT | os.O_RDWR | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(str(lock_path), flags, 0o600)
        handle = os.fdopen(descriptor, "a+", encoding="utf-8")
        os.fchmod(handle.fileno(), 0o600)
        metadata = lock_path.stat()
        if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
            handle.close()
            _fail("transaction lock owner or mode is invalid")
        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            handle.close()
            _fail("transaction is already locked")
        return handle
    except AdapterError:
        raise
    except (OSError, ImportError):
        _fail("transaction lock cannot be acquired")


def _release_transaction_lock(handle: Any) -> None:
    try:
        import fcntl

        fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
    finally:
        handle.close()


def _persist_terminal(journal_path: Path, record: dict[str, Any]) -> None:
    """Persist a terminal record, with a separate durable emergency receipt."""
    try:
        _write_journal(Path(journal_path), record)
        return
    except Exception as error:
        emergency = dict(record)
        emergency["status"] = "reconciliation-blocked"
        emergency["emergency_receipt"] = True
        emergency["journal_write_error"] = error.__class__.__name__
        try:
            _write_journal(Path(f"{journal_path}.emergency.json"), emergency)
        except Exception as emergency_error:
            _fail(
                "terminal journal write failed and emergency reconciliation receipt failed: "
                f"{emergency_error.__class__.__name__}"
            )
        _fail("terminal journal write failed; emergency reconciliation receipt persisted")


def _journal_record(
    plan: dict[str, Any],
    status: str,
    next_index: int,
    completed: list[str],
    error: str | None = None,
    node_receipts: dict[str, dict[str, Any]] | None = None,
    provider_receipts: list[dict[str, Any]] | None = None,
    rollback_status: str = "not-started",
    rollback_receipt: dict[str, Any] | None = None,
    execution_mode: str = "dry-run",
    rollback_error: str | None = None,
    rollback_reobservation_receipt: dict[str, Any] | None = None,
    failed_operation: str | None = None,
    failed_state_digest: str | None = None,
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
        "execution_mode": execution_mode,
        "next_operation_index": next_index,
        "completed_operations": list(completed),
        "operations": list(plan["global_order"]),
        "rollback_policy": "clean-redeploy",
        "restore_old_state": False,
        "cross_node_state_copy": False,
        "node_receipts": copy.deepcopy(node_receipts or {}),
        "provider_receipts": copy.deepcopy(provider_receipts or []),
        "rollback_status": rollback_status,
        "rollback_receipt": copy.deepcopy(rollback_receipt),
        "rollback_reobservation_receipt": copy.deepcopy(rollback_reobservation_receipt),
    }
    if error is not None:
        record["terminal_error"] = error
    if rollback_error is not None:
        record["rollback_error"] = rollback_error
    if failed_operation is not None:
        record["failed_operation"] = failed_operation
    if failed_state_digest is not None:
        record["failed_state_digest"] = failed_state_digest
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
        if receipt.get("status") not in {"planned", "completed"}:
            _fail(f"{name} node receipt status is unsupported")
        operation_count = receipt.get("operation_count")
        if not isinstance(operation_count, int) or operation_count < 0:
            _fail(f"{name} node receipt operation_count is malformed")
        if receipt.get("status") == "completed":
            if not isinstance(receipt.get("last_operation"), str) or not receipt["last_operation"].endswith(name):
                _fail(f"{name} node receipt last operation is malformed")
        elif "last_operation" in receipt:
            _fail(f"{name} planned node receipt must not claim a last operation")
        result[name] = receipt
    return result


def _provider_peer(plan: dict[str, Any], node_name: str | None, operation: str) -> str:
    if node_name is not None:
        node = next((item for item in plan["nodes"] if item["name"] == node_name), None)
        if node is None:
            _fail("provider receipt names an unknown node")
        return CANONICAL_PEER_REGISTRY[node_name]
    if operation == "fresh-root-probe":
        return CANONICAL_PROBE_PEER_ID
    return CANONICAL_FLEET_PEER_ID


def _verify_receipt_with_verifier(
    plan: dict[str, Any],
    receipt: dict[str, Any],
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None,
) -> None:
    if verifier is None:
        return
    try:
        # A provider verifier never needs nonce seams or other adapter-only
        # authority material.  Keep that boundary identical to the transport
        # DTO boundary.
        result = verifier(_transport_plan(plan), receipt)
    except Exception as error:
        _fail(f"provider receipt verifier failed: {error.__class__.__name__}")
    result = _object(result, "provider receipt verifier result")
    if (
        result.get("verified") is not True
        or result.get("bindings") != receipt["bindings"]
        or result.get("verifier_id") != CANONICAL_VERIFIER_ID
        or result.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
        or result.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
    ):
        _fail("provider receipt verifier did not verify the exact receipt binding")


def _validate_provider_receipt(
    plan: dict[str, Any],
    operation: str,
    node_name: str | None,
    raw: Any,
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None,
    *,
    evidence: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Validate and sanitize every provider receipt before phase advance."""
    receipt = _object(raw, f"{operation} provider receipt")
    allowed = {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "verifier_id",
        "trust_root_id",
        "signed_payload_sha256",
        "signature_hex",
        "canonical_digest",
        "transaction_id",
        "capture_window_id",
        "operation",
        "node",
        "peer_id",
        "bindings",
        "replayed",
        "checkpoint_manifest_hash",
        "checkpoint_id",
        "height",
        "package_commit",
        "execution_block_hash",
        "execution_state_root",
        "blob_closure",
        "runtime",
        "connected_provider",
        "recovery_receipt",
        "failed_operation",
        "failed_state_digest",
        "rollback_steps",
        "reobserved",
        "phase",
        "captured_at",
        "observer_mutation",
        "status",
        "backup_manifest",
        "seed_eligible",
        "fleet_health_closure",
    }
    _reject_secret_fields(receipt, f"{operation} provider receipt")
    if set(receipt) - allowed:
        _fail(f"{operation} provider receipt contains an unsafe field")
    phase = _receipt_phase(operation)
    if receipt.get("schema_version") != PHASE_RECEIPT_SCHEMAS[phase]:
        _fail(f"{operation} provider receipt schema is unsupported for phase {phase}")
    if receipt.get("authenticated") is not True or receipt.get("verified") is not True:
        _fail(f"{operation} provider receipt is not authenticated and verified")
    if receipt.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST:
        _fail(f"{operation} provider receipt signer is not code-owned")
    if receipt.get("verifier_id") != CANONICAL_VERIFIER_ID or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID:
        _fail(f"{operation} provider receipt verifier or trust root is not code-owned")
    if (
        receipt.get("phase") != phase
        or receipt.get("captured_at") is None
        or receipt.get("observer_mutation") is not False
        or receipt.get("replayed") is not False
        or receipt.get("status") != ("completed" if phase in {"backup", "apply", "reobserve", "rollback"} else "verified")
    ):
        _fail(f"{operation} provider receipt phase, capture, observer, or status binding drifted")
    captured_at = _parse_utc(receipt.get("captured_at"), f"{operation} captured_at")
    capture_start, capture_end = _capture_window_bounds(plan)
    if not capture_start <= captured_at <= capture_end:
        _fail(f"{operation} captured_at is outside the transaction capture window")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, f"{operation} signed payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, f"{operation} signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, f"{operation} canonical digest")
    if (
        receipt.get("transaction_id") != plan["transaction_id"]
        or receipt.get("capture_window_id") != plan["capture_window_id"]
    ):
        _fail(f"{operation} provider receipt transaction binding drifted")
    if receipt.get("operation") != operation or receipt.get("node") != node_name:
        _fail(f"{operation} provider receipt operation or node binding drifted")
    peer_id = _string(receipt.get("peer_id"), f"{operation} provider receipt peer")
    if peer_id != _provider_peer(plan, node_name, operation):
        _fail(f"{operation} provider receipt peer binding drifted")
    bindings = _object(receipt.get("bindings"), f"{operation} provider receipt bindings")
    expected_bindings = {
        "task_uid": plan["task_uid"],
        "frozen_head_oid": plan["head_oid"],
        "plan_digest": plan["plan_digest"],
        "transaction_id": plan["transaction_id"],
        "capture_window_id": plan["capture_window_id"],
        "operation": operation,
        "node": node_name,
        "peer_id": peer_id,
        "ledger_path": plan["credential_nonce_ledger"]["path"],
    }
    if evidence is not None:
        expected_bindings["evidence_sha256"] = _remote_evidence_digest(evidence)
    elif phase == "preflight" and "evidence_sha256" in bindings:
        # The journal retains the evidence digest inside the signed receipt;
        # the original evidence object is intentionally not replayed here.
        expected_bindings["evidence_sha256"] = _digest(
            bindings.get("evidence_sha256"), f"{operation} evidence digest"
        )
    if bindings != expected_bindings:
        _fail(f"{operation} provider receipt binding set is not exact")
    if operation == "fresh-root-probe":
        checkpoint = plan["truth"]["checkpoint"]
        if (
            receipt.get("replayed") is not False
            or receipt.get("checkpoint_manifest_hash") != checkpoint["manifest_hash"]
            or receipt.get("checkpoint_id") != checkpoint["checkpoint_id"]
            or receipt.get("height") != checkpoint["height"]
            or receipt.get("package_commit") != plan["truth"]["package"]["commit"]
            or receipt.get("execution_block_hash") != checkpoint["execution_block_hash"]
            or receipt.get("execution_state_root") != checkpoint["execution_state_root"]
        ):
            _fail("fresh-root probe receipt is replayed or checkpoint-unbound")
        _validate_fresh_probe_closure(plan, receipt)
    if operation in {"reobserve-failed-state", "rollback-clean-redeploy"}:
        if receipt.get("reobserved") is not True:
            _fail(f"{operation} receipt lacks a fresh failed-state re-observation")
        _string(receipt.get("failed_operation"), f"{operation} failed operation")
        _nonzero_hex(receipt.get("failed_state_digest"), HEX64_RE, f"{operation} failed state digest")
        if receipt.get("rollback_steps") != plan["rollback"]["steps"]:
            _fail(f"{operation} receipt clean-redeploy steps are not exact")
    if phase in {"backup", "apply"}:
        if plan["forensic_backup"]["required_before_reset"] is True:
            manifest = _object(receipt.get("backup_manifest"), f"{operation} backup manifest")
            if set(manifest) != {"node", "sha256", "size_bytes", "verified", "seed_eligible"}:
                _fail(f"{operation} backup manifest is incomplete")
            if manifest.get("node") != node_name or manifest.get("verified") is not True:
                _fail(f"{operation} backup manifest node or verification drifted")
            _nonzero_hex(manifest.get("sha256"), HEX64_RE, f"{operation} backup manifest digest")
            if not isinstance(manifest.get("size_bytes"), int) or manifest["size_bytes"] <= 0:
                _fail(f"{operation} backup manifest size is malformed")
            if manifest.get("seed_eligible") is not False or receipt.get("seed_eligible") is not False:
                _fail(f"{operation} backup is incorrectly seed eligible")
        elif receipt.get("backup_manifest") is not None:
            manifest = _object(receipt["backup_manifest"], f"{operation} backup manifest")
            if manifest.get("seed_eligible") is not False or receipt.get("seed_eligible") is not False:
                _fail(f"{operation} no-backup receipt contains a seed-eligible manifest")
    if operation == "fleet-health":
        closure = _object(receipt.get("fleet_health_closure"), "fleet-health closure")
        if (
            set(closure) != {"verified", "nodes", "healthy"}
            or closure.get("verified") is not True
            or closure.get("healthy") is not True
            or closure.get("nodes") != list(plan["node_order"])
        ):
            _fail("fleet-health receipt does not close the governed fleet")
    _verify_receipt_with_verifier(plan, receipt, verifier)
    return _sanitize_receipt(receipt, f"{operation} provider receipt")


def _validate_fresh_probe_closure(plan: dict[str, Any], receipt: dict[str, Any]) -> None:
    """Require one signed receipt to close execution blobs, runtime, peers, and recovery."""
    if receipt.get("blob_closure") != plan["truth"]["execution"]:
        _fail("fresh-root probe blob closure is not bound to execution truth")
    runtime = _object(receipt.get("runtime"), "fresh-root probe runtime closure")
    expected_runtime = {
        "sha256": plan["truth"]["package"]["runtime_sha256"],
        "size_bytes": plan["truth"]["package"]["runtime_size_bytes"],
    }
    if runtime != expected_runtime:
        _fail("fresh-root probe runtime hash or size is not bound to package truth")
    connected = _object(receipt.get("connected_provider"), "fresh-root connected provider")
    if connected.get("verified") is not True:
        _fail("fresh-root probe connected provider is not verified")
    expected_providers = []
    for name in ("storage-205", "sequencer-204"):
        node = next(item for item in plan["nodes"] if item["name"] == name)
        expected_providers.append(
            {
                "node": name,
                "node_id": node["node_id"],
                "peer_id": CANONICAL_PEER_REGISTRY[name],
                "provider_uid": CANONICAL_PROVIDER_UID[name],
            }
        )
    if connected.get("providers") != expected_providers or set(connected) != {"verified", "providers"}:
        _fail("fresh-root connected provider identity is not the governed pair")
    recovery = _object(receipt.get("recovery_receipt"), "fresh-root recovery receipt")
    allowed = {
        "schema_version", "authenticated", "verified", "signer_id", "verifier_id",
        "trust_root_id", "signed_payload_sha256", "signature_hex", "canonical_digest", "bindings",
    }
    if set(recovery) - allowed:
        _fail("fresh-root recovery receipt contains an unsafe field")
    if (
        recovery.get("schema_version") != RECOVERY_RECEIPT_SCHEMA
        or recovery.get("authenticated") is not True
        or recovery.get("verified") is not True
        or recovery.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
        or recovery.get("verifier_id") != CANONICAL_VERIFIER_ID
        or recovery.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
    ):
        _fail("fresh-root recovery receipt is not independently authenticated")
    _reject_secret_fields(recovery, "fresh-root recovery receipt")
    _nonzero_hex(recovery.get("signed_payload_sha256"), HEX64_RE, "fresh-root recovery payload")
    _nonzero_hex(recovery.get("signature_hex"), SIGNATURE_RE, "fresh-root recovery signature")
    _nonzero_hex(recovery.get("canonical_digest"), HEX64_RE, "fresh-root recovery digest")
    checkpoint = plan["truth"]["checkpoint"]
    expected_bindings = {
        "task_uid": plan["task_uid"],
        "transaction_id": plan["transaction_id"],
        "capture_window_id": plan["capture_window_id"],
        "checkpoint_id": checkpoint["checkpoint_id"],
        "checkpoint_manifest_hash": checkpoint["manifest_hash"],
    }
    if recovery.get("bindings") != expected_bindings:
        _fail("fresh-root recovery receipt bindings are not exact")


def _validate_journal_provider_receipts(
    plan: dict[str, Any], raw: Any, verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None
) -> list[dict[str, Any]]:
    if not isinstance(raw, list):
        _fail("transaction journal provider_receipts must be a list")
    result: list[dict[str, Any]] = []
    for receipt in raw:
        receipt_object = _object(receipt, "transaction journal provider receipt")
        operation = _string(receipt_object.get("operation"), "transaction journal receipt operation")
        node_name = receipt_object.get("node")
        if node_name is not None:
            node_name = _string(node_name, "transaction journal receipt node")
        result.append(_validate_provider_receipt(plan, operation, node_name, receipt_object, verifier))
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
    checkpoint = plan["truth"]["checkpoint"]
    _nonzero_hex(receipt.get("checkpoint_manifest_hash"), HEX64_RE, "live probe checkpoint manifest")
    if (
        receipt.get("checkpoint_manifest_hash") != checkpoint["manifest_hash"]
        or receipt.get("checkpoint_id") != checkpoint["checkpoint_id"]
        or receipt.get("height") != checkpoint["height"]
        or receipt.get("package_commit") != plan["truth"]["package"]["commit"]
        or receipt.get("execution_block_hash") != checkpoint["execution_block_hash"]
        or receipt.get("execution_state_root") != checkpoint["execution_state_root"]
    ):
        _fail("live fresh-root probe checkpoint binding drifted")
    _validate_fresh_probe_closure(plan, receipt)


def _verify_provenance(
    plan: dict[str, Any],
    authority: dict[str, Any],
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None,
) -> bool:
    def validate_result(receipt: dict[str, Any], result: Any) -> None:
        result = _object(result, "external provenance verifier result")
        if (
            result.get("verified") is not True
            or result.get("bindings") != receipt["bindings"]
            or result.get("verifier_id") != CANONICAL_VERIFIER_ID
            or result.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
            or result.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
        ):
            _fail("external provenance verifier did not verify exact execution bindings")

    receipts = [authority["receipt"]]
    forensic = _object(plan.get("forensic_backup"), "forensic backup")
    if forensic.get("mode") == "operator-authorized-no-backup":
        receipts.append(_object(forensic.get("authority"), "no-backup authority receipt"))
    if verifier is None:
        if "provenance_helper" not in authority:
            return False
        result = verify_repository_provenance_helper(plan, authority)
        if len(receipts) != 1:
            _fail("no-backup authority requires the independent receipt verifier callback")
        validate_result(receipts[0], result)
    else:
        for receipt in receipts:
            try:
                result = verifier(_transport_plan(plan), receipt)
            except Exception as error:
                _fail(f"external provenance verifier failed: {error.__class__.__name__}")
            validate_result(receipt, result)
    return True


def _reject_secret_fields(value: Any, label: str) -> None:
    """Reject secret-bearing fields recursively before any provider boundary."""
    if isinstance(value, dict):
        for key, child in value.items():
            key_text = str(key)
            if SECRET_KEY_RE.search(key_text) or key_text in SECRET_FIELD_NAMES:
                _fail(f"{label} contains a secret-bearing field")
            _reject_secret_fields(child, label)
    elif isinstance(value, list):
        for child in value:
            _reject_secret_fields(child, label)
    elif isinstance(value, str) and SECRET_VALUE_RE.search(value):
        _fail(f"{label} contains a secret-bearing value")


def _sanitize_receipt(value: Any, label: str) -> dict[str, Any]:
    value = _object(value, label)
    cleaned: dict[str, Any] = {}
    for key, child in value.items():
        if SECRET_KEY_RE.search(str(key)):
            continue
        if key in SECRET_FIELD_NAMES:
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
    node = _object(node, "transport node")
    if set(node) - TRANSPORT_NODE_FIELDS - {"credential_seam"}:
        _fail("transport node contains a field outside the allowlist")
    result = {key: copy.deepcopy(value) for key, value in node.items() if key != "credential_seam"}
    _reject_secret_fields(result, "transport node")
    return result


def _transport_plan(plan: dict[str, Any]) -> dict[str, Any]:
    plan = _object(plan, "transport plan")
    if set(plan) - TRANSPORT_PLAN_FIELDS - {"authority", "credential_nonce_ledger"}:
        _fail("transport plan contains a field outside the allowlist")
    result = {
        key: copy.deepcopy(value)
        for key, value in plan.items()
        if key not in {"authority", "credential_nonce_ledger"}
    }
    result["nodes"] = [_transport_node(node) for node in result["nodes"]]
    _reject_secret_fields(result, "transport plan")
    return result


def _mutating_operation(operation: str) -> bool:
    return operation.startswith(("forensic-backup:", "stop:", "delete:", "rebuild:", "start:"))


def _receipt_phase(operation: str) -> str:
    if operation.startswith("preflight:"):
        return "preflight"
    if operation.startswith("forensic-backup:"):
        return "backup"
    if operation.startswith(("stop:", "delete:", "rebuild:", "start:")):
        return "apply"
    if operation.startswith("verify:"):
        return "verify"
    if operation == "fresh-root-probe":
        return "fresh-root-probe"
    if operation == "fleet-health":
        return "fleet-health"
    if operation == "reobserve-failed-state":
        return "reobserve"
    if operation == "rollback-clean-redeploy":
        return "rollback"
    _fail(f"{operation} has no governed receipt phase")


def _rollback_candidate(operation: str) -> bool:
    """Only attempted stop/delete/rebuild/start operations can need rollback."""
    return operation.startswith(("stop:", "delete:", "rebuild:", "start:"))


def _read_only_operation(operation: str) -> bool:
    """Classify phases that cannot have changed provider state."""
    return operation.startswith(("preflight:", "verify:")) or operation in {
        "fresh-root-probe",
        "fleet-health",
    }


def _execute_unlocked(
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
    declared_ledger = Path(plan["credential_nonce_ledger"]["path"]).absolute()
    requested_ledger = Path(ledger_path).absolute()
    if requested_ledger != declared_ledger:
        _fail("requested credential nonce ledger is not the plan-bound canonical path")
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
        record = _journal_record(
            plan,
            "dry-run-complete",
            len(operations),
            operations,
            node_receipts=receipts,
            rollback_status="not-needed",
            execution_mode="dry-run",
        )
        try:
            _write_journal(Path(journal_path), record)
        except Exception as error:
            _persist_terminal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "terminal-failure",
                    len(operations),
                    operations,
                    error=error.__class__.__name__,
                    rollback_status="reconciliation-blocked",
                    rollback_error="dry-run-journal-write-failed",
                    execution_mode="dry-run",
                ),
            )
            _fail("dry-run journal write failed; emergency reconciliation receipt persisted")
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
            "provider_receipts": [],
            "rollback_status": "not-needed",
            "rollback_receipt": None,
            "nodes": {name: {"status": "planned", "receipt": receipts[name]} for name in plan["node_order"]},
        }
    required_callbacks = (
        "inspect_node",
        "preflight",
        "verify",
        "health",
        "verify_fresh_root_probe",
        "mutate",
        "reobserve_failed_state",
        "rollback_clean_redeploy",
    )
    if transport is None or any(not callable(getattr(transport, name, None)) for name in required_callbacks):
        _fail("apply requires all explicitly governed provider callbacks")
    if authority_summary["apply_authorized"] is not True:
        _fail("external apply authority is absent")
    if provenance_verifier is None:
        _fail("apply requires the independent provider receipt verifier callback")
    if not provenance_verified:
        _fail("apply requires an independently executed provenance verifier")
    # Re-check the code-owned trust root against the live filesystem after
    # authority/provenance validation and before journal/nonce/provider work.
    validate_live_trust_root_file()
    # One-shot reservations precede every remote observation.  Values stay in
    # the external ledger and never enter a journal, receipt, or log.
    provider_receipts: list[dict[str, Any]] = []
    rollback_receipt: dict[str, Any] | None = None
    rollback_reobservation_receipt: dict[str, Any] | None = None
    rollback_status = "not-started"
    try:
        _write_journal(
            Path(journal_path),
            _journal_record(plan, "prepared", 0, [], execution_mode="apply"),
        )
    except Exception as error:
        _persist_terminal(
            Path(journal_path),
            _journal_record(
                plan,
                "terminal-failure",
                0,
                [],
                error=error.__class__.__name__,
                execution_mode="apply",
                rollback_status="reconciliation-blocked",
                rollback_error="prepared-journal-write-failed",
            ),
        )
        _fail("initial transaction journal write failed; reconciliation is blocked")
    try:
        for node in (plan["nodes"] if isinstance(plan.get("nodes"), list) else []):
            reserve_nonce(Path(ledger_path), plan["transaction_id"], _node_nonce(plan, node))
        for node in plan["nodes"]:
            evidence = transport.inspect_node(_transport_node(node))
            validate_remote_preflight(plan, node, evidence, provenance_verifier)
    except Exception as error:
        _persist_terminal(
            Path(journal_path),
            _journal_record(
                plan,
                "terminal-failure",
                0,
                [],
                error=error.__class__.__name__,
                provider_receipts=provider_receipts,
                rollback_status="not-needed",
                execution_mode="apply",
            ),
        )
        _fail("nonce reservation or remote preflight failed; transaction is terminal")
    completed: list[str] = []
    started: list[str] = []
    rollback_candidates: list[str] = []
    node_receipts: dict[str, dict[str, Any]] = {}
    for index, operation in enumerate(operations):
        node_name: str | None = None
        try:
            # The in-flight journal write is itself protected by the rollback
            # handler.  A failure here must not strand an earlier successful
            # start/rebuild outside the durable transaction boundary.
            _write_journal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "in-flight",
                    index,
                    completed,
                    node_receipts=node_receipts,
                    provider_receipts=provider_receipts,
                    rollback_status=rollback_status,
                    rollback_receipt=rollback_receipt,
                    execution_mode="apply",
                ),
            )
            # Every provider callback is an attempted operation boundary. If
            # it performs a side effect and then throws, clean-redeploy must
            # still receive the current operation in its rollback set.
            if _rollback_candidate(operation) and operation not in rollback_candidates:
                rollback_candidates.append(operation)
            if operation == "fresh-root-probe":
                raw_receipt = transport.verify_fresh_root_probe(_transport_plan(plan))
                receipt = _validate_provider_receipt(
                    plan,
                    operation,
                    None,
                    raw_receipt,
                    provenance_verifier,
                )
                _validate_live_probe(plan, receipt)
            else:
                node_name = operation.partition(":")[2] or None
                node = next((item for item in plan["nodes"] if item["name"] == node_name), None)
                phase = operation.partition(":")[0]
                transport_node = _transport_node(node) if node is not None else None
                if phase == "preflight":
                    raw_receipt = transport.preflight(operation, transport_node)
                elif phase == "verify":
                    raw_receipt = transport.verify(operation, transport_node)
                elif operation == "fleet-health":
                    raw_receipt = transport.health(operation)
                else:
                    raw_receipt = transport.mutate(operation, transport_node)
                # A successful start/rebuild callback may have changed the
                # provider even if its receipt is malformed or the following
                # durable journal write fails.  Include that operation in the
                # clean-redeploy rollback set before any validation/write.
                if operation.startswith(("start:", "rebuild:")):
                    started.append(operation)
                receipt = _validate_provider_receipt(
                    plan,
                    operation,
                    node_name,
                    raw_receipt,
                    provenance_verifier,
                )
            provider_receipts.append(receipt)
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
            journal_rollback_status = "not-needed" if index + 1 == len(operations) else rollback_status
            _write_journal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "complete" if index + 1 == len(operations) else "running",
                    index + 1,
                    completed,
                    node_receipts=node_receipts,
                    provider_receipts=provider_receipts,
                    rollback_status=journal_rollback_status,
                    rollback_receipt=rollback_receipt,
                    execution_mode="apply",
                ),
            )
        except Exception as error:
            rollback_error: str | None = None
            failed_operation = operation
            failed_state_digest: str | None = None
            if not rollback_candidates and _read_only_operation(operation):
                # A failed preflight/verify/probe/health has no provider
                # mutation to reconcile.  Never call re-observe or rollback
                # transports for a read-only failure with an empty set.
                rollback_status = "not-needed"
                _persist_terminal(
                    Path(journal_path),
                    _journal_record(
                        plan,
                        "terminal-failure",
                        index,
                        completed,
                        error.__class__.__name__,
                        node_receipts,
                        provider_receipts,
                        rollback_status,
                        rollback_receipt,
                        execution_mode="apply",
                        failed_operation=failed_operation,
                    ),
                )
                _fail("read-only provider operation failed; no rollback is required")
            try:
                if _rollback_candidate(operation) and operation not in rollback_candidates:
                    rollback_candidates.append(operation)
                validate_authority(plan, authority)
                if provenance_verifier is None:
                    _fail("rollback requires the independent provider receipt verifier callback")
                if _verify_provenance(plan, authority, provenance_verifier) is not True:
                    _fail("rollback requires a fresh independent provenance verification")
                rollback_reobservation_receipt = transport.reobserve_failed_state(
                    _transport_plan(plan), list(rollback_candidates), failed_operation
                )
                rollback_reobservation_receipt = _validate_provider_receipt(
                    plan,
                    "reobserve-failed-state",
                    None,
                    rollback_reobservation_receipt,
                    provenance_verifier,
                )
                failed_state_digest = rollback_reobservation_receipt["failed_state_digest"]
                if rollback_reobservation_receipt["failed_operation"] != failed_operation:
                    _fail("rollback re-observation failed-operation binding drifted")
                rollback_receipt = transport.rollback_clean_redeploy(
                    _transport_plan(plan), list(rollback_candidates), rollback_reobservation_receipt
                )
                rollback_receipt = _validate_provider_receipt(
                    plan,
                    "rollback-clean-redeploy",
                    None,
                    rollback_receipt,
                    provenance_verifier,
                )
                if (
                    rollback_receipt["failed_operation"] != failed_operation
                    or rollback_receipt["failed_state_digest"] != failed_state_digest
                ):
                    _fail("clean-redeploy receipt is not bound to the re-observed failed state")
                rollback_status = "completed"
            except Exception as rollback_failure:
                rollback_receipt = None
                rollback_status = "reconciliation-blocked"
                rollback_error = rollback_failure.__class__.__name__
            _persist_terminal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "terminal-failure",
                    index,
                    completed,
                    error.__class__.__name__,
                    node_receipts,
                    provider_receipts,
                    rollback_status,
                    rollback_receipt,
                    execution_mode="apply",
                    rollback_error=rollback_error,
                    rollback_reobservation_receipt=rollback_reobservation_receipt,
                    failed_operation=failed_operation,
                    failed_state_digest=failed_state_digest,
                ),
            )
            if rollback_status == "reconciliation-blocked":
                _fail("provider operation and clean-redeploy rollback failed; reconciliation is blocked")
            _fail("provider operation failed; transaction is terminal and requires governed reconciliation")
    return {
        "schema_version": ADAPTER_SCHEMA,
        "status": "complete",
        "dry_run": False,
        "resumed": False,
        "plan_digest": plan["plan_digest"],
        "transaction_id": plan["transaction_id"],
        "operations": operations,
        "provider_mutation_performed": True,
        "provider_receipts": provider_receipts,
        "rollback_status": "not-needed",
        "rollback_receipt": None,
        "nodes": {name: {"status": "complete", "receipt": node_receipts.get(name)} for name in plan["node_order"]},
    }


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
    """Serialize one transaction while retaining the implementation boundary."""
    lock = _acquire_transaction_lock(Path(journal_path))
    try:
        return _execute_unlocked(
            plan,
            authority,
            journal_path=journal_path,
            ledger_path=ledger_path,
            transport=transport,
            dry_run=dry_run,
            provenance_verifier=provenance_verifier,
        )
    finally:
        _release_transaction_lock(lock)


def _resume_transaction_unlocked(
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
    authority_summary = validate_authority(plan, authority)
    provenance_verified = False
    if not dry_run:
        if authority_summary["apply_authorized"] is not True:
            _fail("completed apply resume requires current apply authority")
        if provenance_verifier is None:
            _fail("completed apply resume requires the independent provider receipt verifier callback")
        # A completed apply is not proof of current authority. Re-run the
        # independently owned verifier before trusting any persisted receipt.
        provenance_verified = _verify_provenance(plan, authority, provenance_verifier)
        if not provenance_verified:
            _fail("completed apply resume requires an independently executed provenance verifier")
    declared_ledger = Path(plan["credential_nonce_ledger"]["path"]).absolute()
    if Path(ledger_path).absolute() != declared_ledger:
        _fail("requested credential nonce ledger is not the plan-bound canonical path")
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
    expected_mode = "dry-run" if dry_run else "apply"
    if record.get("execution_mode") != expected_mode:
        _fail("transaction journal execution mode is not the requested mode")
    if record.get("operations") != plan["global_order"]:
        _fail("transaction journal operation order is not the frozen deterministic order")
    receipts = _validate_node_receipts(plan, record.get("node_receipts", {}))
    provider_receipts = _validate_journal_provider_receipts(
        plan,
        record.get("provider_receipts", []),
        provenance_verifier if not dry_run else None,
    )
    rollback_status = record.get("rollback_status")
    if rollback_status not in {"not-started", "not-needed", "completed", "reconciliation-blocked"}:
        _fail("transaction journal rollback status is unsupported")
    rollback_receipt_raw = record.get("rollback_receipt")
    rollback_reobservation_raw = record.get("rollback_reobservation_receipt")
    if rollback_status == "completed":
        rollback_reobservation = _validate_provider_receipt(
            plan,
            "reobserve-failed-state",
            None,
            rollback_reobservation_raw,
            provenance_verifier if not dry_run else None,
        )
        rollback_receipt = _validate_provider_receipt(
            plan,
            "rollback-clean-redeploy",
            None,
            rollback_receipt_raw,
            provenance_verifier if not dry_run else None,
        )
        if (
            record.get("failed_operation") != rollback_receipt["failed_operation"]
            or record.get("failed_state_digest") != rollback_receipt["failed_state_digest"]
        ):
            _fail("transaction journal rollback receipt is not bound to its failed state")
    elif rollback_status == "reconciliation-blocked":
        if rollback_receipt_raw is not None:
            _fail("reconciliation-blocked journal must not claim a completed rollback")
        if rollback_reobservation_raw is not None:
            rollback_reobservation = _validate_provider_receipt(
                plan,
                "reobserve-failed-state",
                None,
                rollback_reobservation_raw,
                provenance_verifier if not dry_run else None,
            )
            if (
                record.get("failed_operation") != rollback_reobservation["failed_operation"]
                or record.get("failed_state_digest") != rollback_reobservation["failed_state_digest"]
            ):
                _fail("reconciliation journal re-observation is not bound to its failed state")
    elif rollback_receipt_raw is not None or rollback_reobservation_raw is not None:
        _fail("transaction journal has a rollback receipt without a completed rollback")
    status = record.get("status")
    completed = record.get("completed_operations")
    next_index = record.get("next_operation_index")
    if (
        not isinstance(completed, list)
        or not isinstance(next_index, int)
        or next_index < 0
        or next_index > len(plan["global_order"])
        or completed != plan["global_order"][:next_index]
    ):
        _fail("transaction journal progress is not a deterministic operation prefix")
    if status in {"dry-run-complete", "complete"} and next_index != len(plan["global_order"]):
        _fail("completed transaction journal does not cover the full operation order")
    provider_operations = [receipt["operation"] for receipt in provider_receipts]
    if expected_mode == "dry-run":
        if provider_operations:
            _fail("dry-run journal must not contain provider receipts")
    elif provider_operations != completed:
        _fail("apply journal provider receipts do not cover its completed prefix")
    if rollback_status == "reconciliation-blocked" and not isinstance(record.get("rollback_error"), str):
        _fail("reconciliation-blocked journal lacks an explicit rollback error")
    if rollback_status != "reconciliation-blocked" and "rollback_error" in record:
        _fail("non-terminal rollback journal contains a rollback error")
    if (
        record.get("rollback_policy") != "clean-redeploy"
        or record.get("restore_old_state") is not False
        or record.get("cross_node_state_copy") is not False
    ):
        _fail("transaction journal contains an unsafe rollback policy")
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
            "provider_receipts": provider_receipts,
            "rollback_status": rollback_status,
            "rollback_receipt": rollback_receipt_raw,
            "nodes": {
                name: {"status": "already-complete", "receipt": receipts.get(name)}
                for name in plan["node_order"]
            },
        }
    if status == "in-flight":
        terminal = dict(record)
        terminal["status"] = "terminal-failure"
        terminal["terminal_error"] = "ambiguous in-flight journal requires governed reconciliation"
        _persist_terminal(Path(journal_path), terminal)
        _fail("transaction journal is ambiguous in-flight; governed reconciliation is required")
    if status == "terminal-failure":
        _fail("transaction journal is terminal-failure; governed reconciliation is required")
    if status in {"prepared", "running"} and dry_run:
        _write_journal(
            Path(journal_path),
            _journal_record(
                plan,
                "dry-run-complete",
                len(plan["global_order"]),
                list(plan["global_order"]),
                node_receipts=receipts,
                provider_receipts=provider_receipts,
                rollback_status="not-needed",
                rollback_receipt=None,
                execution_mode="dry-run",
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
            "provider_receipts": [],
            "rollback_status": "not-needed",
            "rollback_receipt": None,
            "nodes": {
                name: {"status": "planned", "receipt": receipts.get(name)}
                for name in plan["node_order"]
            },
        }
    _fail("transaction journal status is not resumable")


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
    """Serialize resume/reconciliation against the same transaction lock."""
    lock = _acquire_transaction_lock(Path(journal_path))
    try:
        return _resume_transaction_unlocked(
            plan,
            authority,
            journal_path,
            ledger_path=ledger_path,
            transport=transport,
            dry_run=dry_run,
            provenance_verifier=provenance_verifier,
        )
    finally:
        _release_transaction_lock(lock)


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
