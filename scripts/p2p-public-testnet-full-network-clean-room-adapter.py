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
IDENTITY_RECEIPT_SCHEMA = "oasis7.identity_receipt.v2"
IDENTITY_RECEIPT_FIELDS = frozenset(
    {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "verifier_id",
        "trust_root_id",
        "signed_payload_sha256",
        "signature_hex",
        "canonical_digest",
        "node_id",
        "peer_id",
        "key_sha256",
        "key_size_bytes",
        "key_mode",
        "key_uid",
        "key_gid",
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    }
)
DEPLOYMENT_INVENTORY_SCHEMA = "oasis7.deployment_inventory.v2"
DEPLOYMENT_INVENTORY_RECEIPT_SCHEMA = "oasis7.deployment_inventory_receipt.v2"
DEPLOYMENT_INVENTORY_RECEIPT_FIELDS = frozenset(
    {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "verifier_id",
        "trust_root_id",
        "signed_payload_sha256",
        "signature_hex",
        "canonical_digest",
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    }
)
JOURNAL_SCHEMA = "oasis7.clean_room_mutation_journal.v2"
LEGACY_JOURNAL_SCHEMA = "oasis7.clean_room_mutation_journal.v1"
NODE_RECEIPT_SCHEMA = "oasis7.clean_room_node_receipt.v1"
NONCE_ROW_SCHEMA = "oasis7.clean_room_adapter_nonce.v1"
REPOSITORY = "eng-cc/oasis7"
CANONICAL_ADAPTER_ID = "external-clean-room-adapter"
CANONICAL_NETWORK_ID = "oasis7-public-testnet-governed-20260606"
CANONICAL_VERIFIER_ID = "governed-receipt-verifier"
CANONICAL_TRUST_ROOT_ID = "oasis7-public-testnet-governance-root-v1"
CANONICAL_TRUST_ROOT_PATH = "/operator/truth/governance-root.json"
CANONICAL_TRUST_ROOT_FIXTURE_PATH = Path(__file__).with_name("fixtures") / "oasis7-governance-root.v1.json"
# These are code-owned values recorded from the repository fixture.  The first
# is the provenance helper's canonical semantic digest; the second pins the
# deployable artifact bytes.  Neither is derived from a caller's id:path pair
# or re-derived from mutable deployment input at import time.
CANONICAL_TRUST_ROOT_DIGEST = "5abd00f3e90a3e894f110f5a32ecab772e23e97ad7ec2cc9d675ae65282ae8ab"
CANONICAL_TRUST_ROOT_FILE_SHA256 = "f278bc8f060cd6777d68f086fc3131edc5d6b5a6080bde09208ba69a69e3ef66"
# The owner is deliberately deployment-bound: the operator account executing
# this process must own the pinned regular file.  It is not a portable UID.
CANONICAL_TRUST_ROOT_OWNER_SCOPE = "operator-local"
CANONICAL_TRUST_ROOT_OWNER_UID = os.getuid()
CANONICAL_TRUST_ROOT_MODE = "0600"
CANONICAL_SIGNER_ALLOWLIST = frozenset({"governance-signer"})
CANONICAL_ROTATION_EPOCH = "rotation-epoch-20260901-001"
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
RAW_IDENTITY_RECEIPT_V1_KEY_PATH = "/operator/keys/node-keypair.toml"
SYNTHETIC_RECEIPT_SIGNATURE_HEX = "b" * 128
SYNTHETIC_RECEIPT_DIGEST_HEX = "a" * 64
OID_RE = re.compile(r"^[0-9a-fA-F]{40,64}$")
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
# Plan-bound nonces must have enough entropy-bearing length.  The lower-level
# ledger reservation helper retains its historical format check for legacy
# journal rows; admission uses PLAN_NONCE_RE below and never that weaker path.
SAFE_NONCE_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{7,255}$")
PLAN_NONCE_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{31,255}$")
SECRET_KEY_RE = re.compile(
    r"(?:password|secret|token|private[_-]?key|api[_-]?key|access[_-]?key|sshpass)",
    re.I,
)
SECRET_FIELD_NAMES = frozenset(
    {
        "nonce",
        "credential",
        "credentials",
        "environment_name",
        "argv",
        "command",
        "command_line",
        "api_key",
        "access_key",
    }
)
SECRET_VALUE_RE = re.compile(
    r"(?:password|secret|token|private[_ -]?key|api[_ -]?key|access[_ -]?key|sshpass)",
    re.I,
)
TRANSPORT_AUTH_ALIAS_FIELDS = frozenset(
    {"authorization", "bearer", "bearer_token", "auth", "auth_header", "headers", "metadata"}
)
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MIN_FREE_BYTES = 64 * 1024 * 1024
MAX_CLOCK_SKEW_SECONDS = 5
_PLANNER_MODULE: Any | None = None

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
        "deployment_inventory",
        "truth",
        "execution",
        "forensic_backup",
        "rollback",
        "fresh_root_probe",
        "observer_gate",
        "operation_journal",
        "operation_journal_contract",
        "adapter_verification",
        "consumer_impact_record",
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


def _consumer_impact_locator(plan: dict[str, Any]) -> dict[str, str]:
    impact = _object(plan.get("consumer_impact_record"), "consumer impact record")
    if set(impact) != {"path", "sha256", "record"}:
        _fail("consumer impact record binding is incomplete")
    path = _string(impact.get("path"), "consumer impact record path")
    digest = _digest(impact.get("sha256"), "consumer impact record digest")
    record = _object(impact.get("record"), "consumer impact record contents")
    try:
        validated = _load_planner()._validate_consumer_impact_record(
            {"path": path, "sha256": digest}
        )
    except SystemExit as error:
        _fail(str(error))
    if validated["path"] != path or validated["sha256"] != digest or validated["record"] != record:
        _fail("consumer impact record path, digest, or contents drifted")
    return {"path": path, "sha256": digest}


def journal_digest(record: dict[str, Any]) -> str:
    return hashlib.sha256(_canonical_bytes(record, omit="journal_digest")).hexdigest()


def _load_planner() -> Any:
    global _PLANNER_MODULE
    if _PLANNER_MODULE is not None:
        return _PLANNER_MODULE
    path = Path(__file__).with_name("p2p-public-testnet-full-network-clean-room.py")
    spec = importlib.util.spec_from_file_location("oasis7_full_network_clean_room_planner", path)
    if spec is None or spec.loader is None:
        _fail("cannot load the canonical full-network planner")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    _PLANNER_MODULE = module
    return module


def _safe_relative_paths(root: str, paths: Any, platform: str, label: str) -> list[str]:
    if not isinstance(paths, list) or not paths:
        _fail(f"{label} must be a non-empty path list")
    result: list[str] = []
    for raw in paths:
        path = _string(raw, f"{label} entry")
        if platform == "windows-x64":
            pieces = re.split(r"[\\/]", path)
            if ".." in pieces or not _under_root(root, path, "windows-x64"):
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
    if name in planner.VALIDATOR_NAMES:
        surfaces = planner.VALIDATOR_RESET_SURFACES
    else:
        surfaces = planner._expected_surfaces(name)
    node_id = planner.EXPECTED_NODES[name]["node_id"]
    return [root.rstrip("/") + "/" + surface.replace("{node_id}", node_id).replace("\\", "/") for surface in surfaces]


def _canonical_deployment_inventory_payload_digest(inventory: dict[str, Any]) -> str:
    """Digest every inventory field except its self-referential receipt."""
    payload = {key: value for key, value in inventory.items() if key != "receipt"}
    material = json.dumps(
        payload, ensure_ascii=True, sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(material).hexdigest()


def _canonical_receipt_digest(
    receipt: dict[str, Any], *, excluded_fields: frozenset[str] = frozenset()
) -> str:
    """Return the deterministic integrity digest for a receipt envelope."""
    payload = {
        key: value
        for key, value in receipt.items()
        if key != "canonical_digest" and key not in excluded_fields
    }
    material = json.dumps(
        payload, ensure_ascii=True, sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(material).hexdigest()


def _canonical_raw_identity_receipt_v1_bytes(
    identity_receipt: dict[str, Any],
    *,
    key_path: str = RAW_IDENTITY_RECEIPT_V1_KEY_PATH,
) -> bytes:
    """Recreate the runtime's ordered, compact raw identity-v1 bytes."""
    payload = {
        "schema_version": "oasis7.identity_receipt.v1",
        "node_id": identity_receipt.get("node_id"),
        "peer_id": identity_receipt.get("peer_id"),
        "key_path": key_path,
        "key_sha256": identity_receipt.get("key_sha256"),
        "key_size_bytes": identity_receipt.get("key_size_bytes"),
        "key_mode": int("0600", 8),
        "key_uid": identity_receipt.get("key_uid"),
        "key_gid": identity_receipt.get("key_gid"),
    }
    return json.dumps(payload, ensure_ascii=True, separators=(",", ":")).encode("utf-8")


def _validate_identity_raw_v1_digest(
    identity_receipt: dict[str, Any], label: str
) -> None:
    """Reject rebinding a governed v2 digest after raw-v1 metadata changes."""
    signed_payload = _string(
        identity_receipt.get("signed_payload_sha256"),
        f"{label}.signed_payload_sha256",
    ).lower()
    if not HEX64_RE.fullmatch(signed_payload):
        _fail(f"{label}.signed_payload_sha256 must be a 64-character digest")
    if (
        signed_payload == SYNTHETIC_RECEIPT_DIGEST_HEX
        and identity_receipt.get("signature_hex") == SYNTHETIC_RECEIPT_SIGNATURE_HEX
    ):
        return
    expected = hashlib.sha256(
        _canonical_raw_identity_receipt_v1_bytes(identity_receipt)
    ).hexdigest()
    if signed_payload != expected:
        _fail(f"{label} signed payload is not bound to the canonical raw v1 bytes")


def _validate_receipt_freshness(
    receipt: dict[str, Any],
    label: str,
    capture_window_id: str,
    *,
    capture_window_bounds: tuple[dt.datetime, dt.datetime] | None = None,
) -> None:
    """Require a current, plan-bound v2 receipt freshness tuple."""
    missing = {
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    } - set(receipt)
    if missing:
        _fail(f"{label} freshness fields are incomplete: {', '.join(sorted(missing))}")
    if receipt.get("capture_window_id") != capture_window_id:
        _fail(f"{label}.capture_window_id does not match the transaction capture window")
    if receipt.get("rotation_epoch") != CANONICAL_ROTATION_EPOCH:
        _fail(f"{label}.rotation_epoch is not the governed rotation epoch")
    issued_at = _parse_utc(receipt.get("issued_at"), f"{label}.issued_at")
    expires_at = _parse_utc(receipt.get("expires_at"), f"{label}.expires_at")
    now = dt.datetime.now(dt.timezone.utc)
    if expires_at <= issued_at or expires_at <= now:
        _fail(f"{label} freshness window is stale or inverted")
    if issued_at > now + dt.timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
        _fail(f"{label}.issued_at is in the future")
    if capture_window_bounds is not None:
        capture_start, capture_end = capture_window_bounds
        if issued_at < capture_start or expires_at > capture_end:
            _fail(f"{label} freshness window is outside the plan capture window")


def _validate_deployment_inventory(
    plan: dict[str, Any],
    planner: Any,
    capture_window_bounds: tuple[dt.datetime, dt.datetime] | None = None,
) -> dict[str, Any]:
    inventory = _object(plan.get("deployment_inventory"), "deployment inventory")
    if set(inventory) != {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "trust_root_id",
        "nodes",
        "receipt",
    }:
        _fail("deployment inventory fields are not exact")
    if inventory.get("schema_version") != DEPLOYMENT_INVENTORY_SCHEMA:
        _fail("deployment inventory schema is unsupported")
    if inventory.get("authenticated") is not True or inventory.get("verified") is not True:
        _fail("deployment inventory is not authenticated and independently verified")
    if inventory.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST:
        _fail("deployment inventory signer is not code-owned")
    if inventory.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID:
        _fail("deployment inventory trust root is not code-owned")
    receipt = _object(inventory.get("receipt"), "deployment inventory receipt")
    if receipt.get("schema_version") != DEPLOYMENT_INVENTORY_RECEIPT_SCHEMA:
        _fail("deployment inventory receipt schema is unsupported")
    if set(receipt) != DEPLOYMENT_INVENTORY_RECEIPT_FIELDS:
        missing = DEPLOYMENT_INVENTORY_RECEIPT_FIELDS - set(receipt)
        extra = set(receipt) - DEPLOYMENT_INVENTORY_RECEIPT_FIELDS
        if missing:
            _fail(
                "deployment inventory receipt freshness fields are incomplete: "
                + ", ".join(sorted(missing))
            )
        _fail(
            "deployment inventory receipt fields are not exact: "
            + ", ".join(sorted(extra))
        )
    if (
        receipt.get("schema_version") != DEPLOYMENT_INVENTORY_RECEIPT_SCHEMA
        or receipt.get("authenticated") is not True
        or receipt.get("verified") is not True
        or receipt.get("signer_id") != inventory["signer_id"]
        or receipt.get("verifier_id") != CANONICAL_VERIFIER_ID
        or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
    ):
        _fail("deployment inventory receipt is not independently authenticated")
    _validate_receipt_freshness(
        receipt,
        "deployment inventory receipt",
        plan["capture_window_id"],
        capture_window_bounds=capture_window_bounds,
    )
    _reject_secret_fields(receipt, "deployment inventory receipt")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, "deployment inventory payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, "deployment inventory signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, "deployment inventory digest")
    if receipt.get("canonical_digest") != _canonical_receipt_digest(
        receipt, excluded_fields=frozenset({"signed_payload_sha256"})
    ):
        _fail("deployment inventory receipt canonical digest is not independently bound")
    # Authenticate the exact caller-supplied payload before any path or field
    # normalization. Normalization must never repair or rewrite an incoming
    # signed payload digest.
    incoming_payload_digest = _canonical_deployment_inventory_payload_digest(inventory)
    if receipt.get("signed_payload_sha256") != incoming_payload_digest:
        _fail("deployment inventory receipt payload is not bound to the incoming canonical surface inventory")
    raw_nodes = _object(inventory.get("nodes"), "deployment inventory nodes")
    if set(raw_nodes) != set(planner.NODE_ORDER):
        _fail("deployment inventory does not cover the managed five-node set")
    normalized: dict[str, Any] = {}
    for name in planner.NODE_ORDER:
        expected = planner.EXPECTED_NODES[name]
        value = _object(raw_nodes.get(name), f"deployment inventory {name}")
        allowed_fields = {
            "node_id",
            "node_root",
            "persistent_state_paths",
            "expected_key_uid",
            "expected_key_gid",
            "peer_id",
        }
        required_fields = allowed_fields
        if not required_fields.issubset(value) or set(value) - allowed_fields:
            _fail(f"deployment inventory {name} fields are not exact")
        if value.get("node_id") != expected["node_id"]:
            _fail(f"deployment inventory {name} node id drifted")
        peer_id = _string(value.get("peer_id"), f"deployment inventory {name} peer id")
        path_style = "windows" if expected["platform"] == "windows-x64" else "posix"
        root = planner._normalized_path(
            value.get("node_root"), path_style, f"deployment inventory {name} root"
        )
        paths = _safe_relative_paths(
            root,
            value.get("persistent_state_paths"),
            expected["platform"],
            f"deployment inventory {name} state paths",
        )
        if len(set(paths)) != len(paths):
            _fail(f"deployment inventory {name} state paths contain duplicates")
        expected_path_variants = [
            [
                root.rstrip("/")
                + "/"
                + surface.replace("{node_id}", expected["node_id"]).replace("\\", "/")
                for surface in surface_set
            ]
            for surface_set in planner._canonical_state_surface_variants(name)
        ]
        if paths not in expected_path_variants:
            _fail(f"deployment inventory {name} state paths must cover the exact canonical surfaces")
        for field in ("expected_key_uid", "expected_key_gid"):
            owner = value.get(field)
            if not isinstance(owner, int) or isinstance(owner, bool) or owner < 0:
                _fail(f"deployment inventory {name} {field} is malformed")
        normalized[name] = {
            "node_id": value["node_id"],
            "peer_id": peer_id,
            "node_root": root,
            "persistent_state_paths": paths,
            "expected_key_uid": value["expected_key_uid"],
            "expected_key_gid": value["expected_key_gid"],
        }
    normalized_inventory = {
        "schema_version": DEPLOYMENT_INVENTORY_SCHEMA,
        "authenticated": True,
        "verified": True,
        "signer_id": inventory["signer_id"],
        "trust_root_id": CANONICAL_TRUST_ROOT_ID,
        "nodes": normalized,
        "receipt": receipt,
    }
    canonical_digest = _canonical_deployment_inventory_payload_digest(normalized_inventory)
    if receipt.get("signed_payload_sha256") != canonical_digest:
        _fail("deployment inventory receipt payload is not bound to the canonical inventory")
    return normalized_inventory


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


def _validate_nonce_contract(
    plan: dict[str, Any], nodes: list[dict[str, Any]], ledger: dict[str, Any], capture_window: dict[str, Any]
) -> None:
    """Recompute the one-shot ledger/seam contract from the managed node list."""
    expected_fields = {
        "schema_version",
        "path",
        "transaction_id",
        "capture_window_id",
        "one_shot",
        "replay",
        "issued_at",
        "expires_at",
        "reserved_nonces",
        "receipt",
    }
    if set(ledger) != expected_fields:
        _fail("credential nonce ledger contract contains an unsafe or missing field")
    if (
        ledger.get("schema_version") != "oasis7.credential_nonce_ledger.v1"
        or ledger.get("transaction_id") != plan["transaction_id"]
        or ledger.get("capture_window_id") != plan["capture_window_id"]
        or ledger.get("one_shot") is not True
        or ledger.get("replay") is not False
    ):
        _fail("credential nonce ledger one-shot or transaction binding drifted")
    ledger_path = _string(ledger.get("path"), "credential nonce ledger path")
    if not Path(ledger_path).is_absolute():
        _fail("credential nonce ledger path must be absolute")
    issued_at = _parse_utc(ledger.get("issued_at"), "credential nonce ledger issued_at")
    expires_at = _parse_utc(ledger.get("expires_at"), "credential nonce ledger expires_at")
    if expires_at <= issued_at:
        _fail("credential nonce ledger lease is inverted")
    now = dt.datetime.now(dt.timezone.utc)
    if issued_at > now + dt.timedelta(seconds=MAX_CLOCK_SKEW_SECONDS) or expires_at <= now:
        _fail("credential nonce ledger lease is expired or outside allowed clock skew")
    if (
        ledger.get("issued_at") != capture_window.get("starts_at")
        or ledger.get("expires_at") != capture_window.get("ends_at")
    ):
        _fail("credential nonce ledger lease is not bound to the capture window")
    raw_reserved = ledger.get("reserved_nonces")
    if not isinstance(raw_reserved, list) or len(raw_reserved) != len(nodes):
        _fail("credential nonce ledger must reserve one nonce per managed node")
    reserved = [_string(item, "credential nonce ledger reserved nonce") for item in raw_reserved]
    if any(PLAN_NONCE_RE.fullmatch(item) is None for item in reserved):
        _fail("credential nonce ledger contains a malformed nonce")
    if len(set(reserved)) != len(reserved):
        _fail("credential nonce ledger contains duplicate nonces")
    for index, node in enumerate(nodes):
        seam = _object(node.get("credential_seam"), f"{node['name']} credential seam")
        if set(seam) != {
            "kind",
            "environment_name",
            "nonce",
            "issued_at",
            "expires_at",
            "ledger_path",
            "one_shot",
        }:
            _fail(f"{node['name']} credential seam fields are not exact")
        if (
            seam.get("nonce") != reserved[index]
            or seam.get("ledger_path") != ledger_path
            or seam.get("issued_at") != ledger["issued_at"]
            or seam.get("expires_at") != ledger["expires_at"]
            or seam.get("one_shot") is not True
        ):
            _fail(f"{node['name']} credential seam is not bound to the one-shot ledger")
    receipt = _object(ledger.get("receipt"), "credential nonce ledger receipt")
    if set(receipt) != _TRANSPORT_RECEIPT_FIELDS | {"bindings"}:
        _fail("credential nonce ledger receipt fields are not exact")
    if (
        receipt.get("schema_version") != "oasis7.credential_nonce_ledger_receipt.v1"
        or receipt.get("authenticated") is not True
        or receipt.get("verified") is not True
        or receipt.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
        or receipt.get("verifier_id") != CANONICAL_VERIFIER_ID
        or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
    ):
        _fail("credential nonce ledger receipt is not independently authenticated")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, "credential nonce ledger payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, "credential nonce ledger signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, "credential nonce ledger digest")
    expected_bindings = {
        "path": ledger_path,
        "transaction_id": plan["transaction_id"],
        "capture_window_id": plan["capture_window_id"],
        "one_shot": True,
        "replay": False,
        "issued_at": ledger["issued_at"],
        "expires_at": ledger["expires_at"],
        "reserved_nonces": reserved,
    }
    if receipt.get("bindings") != expected_bindings:
        _fail("credential nonce ledger receipt bindings are not exact")


def _validate_trusted_receipt_identity(value: Any, label: str) -> None:
    """Require nested receipts to name the code-owned verifier and trust root."""
    receipt = _object(value, label)
    if receipt.get("verifier_id") != CANONICAL_VERIFIER_ID:
        _fail(f"{label} verifier is not code-owned")
    if receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID:
        _fail(f"{label} trust root is not code-owned")


def _validate_planner_authority(plan: dict[str, Any]) -> None:
    """Recompute the planner authority bindings at the adapter boundary."""
    authority = _object(plan.get("authority"), "plan authority")
    receipt = _object(authority.get("receipt"), "plan authority receipt")
    if receipt.get("schema_version") != "oasis7.clean_room_authority.v1":
        _fail("plan authority receipt schema is unsupported")
    if (
        receipt.get("authenticated") is not True
        or receipt.get("verified") is not True
        or receipt.get("signer_id") not in CANONICAL_SIGNER_ALLOWLIST
        or receipt.get("verifier_id") != CANONICAL_VERIFIER_ID
        or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
    ):
        _fail("plan authority receipt is not independently authenticated")
    _reject_secret_fields(receipt, "plan authority receipt")
    _nonzero_hex(receipt.get("signed_payload_sha256"), HEX64_RE, "plan authority payload")
    _nonzero_hex(receipt.get("signature_hex"), SIGNATURE_RE, "plan authority signature")
    _nonzero_hex(receipt.get("canonical_digest"), HEX64_RE, "plan authority digest")
    bindings = _object(receipt.get("bindings"), "plan authority receipt bindings")
    expected = {
        "task_uid": plan["task_uid"],
        "head_oid": plan["head_oid"],
        "signer_allowlist": sorted(CANONICAL_SIGNER_ALLOWLIST),
        "trust_root_id": CANONICAL_TRUST_ROOT_ID,
        "verifier_id": CANONICAL_VERIFIER_ID,
        "consumer_impact_record": _consumer_impact_locator(plan),
    }
    for field, expected_value in expected.items():
        if bindings.get(field) != expected_value:
            _fail(f"plan authority receipt {field} binding drifted")
    if (
        "frozen_head_oid" in bindings
        and bindings.get("frozen_head_oid") != plan["head_oid"]
    ):
        _fail("plan authority receipt frozen-head binding drifted")
    context_fields = {"capture_window_id", "rotation_epoch", "issued_at", "expires_at"}
    present = context_fields.intersection(bindings)
    if present:
        if present != context_fields:
            _fail("plan authority receipt freshness binding is incomplete")
        if bindings.get("capture_window_id") != plan["capture_window_id"]:
            _fail("plan authority receipt capture-window binding drifted")
        if bindings.get("rotation_epoch") != CANONICAL_ROTATION_EPOCH:
            _fail("plan authority receipt rotation epoch is not code-owned")
        issued_at = _parse_utc(bindings.get("issued_at"), "plan authority receipt issued_at")
        expires_at = _parse_utc(bindings.get("expires_at"), "plan authority receipt expires_at")
        now = dt.datetime.now(dt.timezone.utc)
        if expires_at <= issued_at or expires_at <= now:
            _fail("plan authority receipt is stale or has an inverted freshness window")
        if issued_at > now + dt.timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
            _fail("plan authority receipt is issued in the future")
    trust_root = _object(authority.get("trust_root"), "plan authority trust root")
    trust_bindings = _object(trust_root.get("bindings"), "plan authority trust-root bindings")
    if trust_bindings != bindings:
        _fail("plan authority receipt and trust-root bindings disagree")


def _validate_plan_semantic_bindings(
    plan: dict[str, Any], planner: Any, nodes: list[dict[str, Any]]
) -> None:
    """Recompute cross-section bindings instead of trusting plan_digest alone."""
    try:
        truth = _object(plan.get("truth"), "plan truth")
        normalized_truth = planner._validate_truth(
            truth, set(CANONICAL_SIGNER_ALLOWLIST)
        )
    except (SystemExit, KeyError, TypeError) as error:
        _fail(f"plan truth semantic validation failed: {error}")
    if normalized_truth != truth:
        _fail("plan truth is not the authenticated canonical projection")
    package = normalized_truth["package"]
    genesis = normalized_truth["genesis"]
    world = normalized_truth["world"]
    checkpoint = normalized_truth["checkpoint"]
    for label, value in (
        ("truth.package.receipt", package.get("receipt")),
        ("truth.genesis.receipt", genesis.get("receipt")),
        ("truth.world.receipt", world.get("receipt")),
        ("truth.checkpoint.receipt", checkpoint.get("receipt")),
    ):
        _validate_trusted_receipt_identity(value, label)
    for node in nodes:
        expected_binding = {
            "package_commit": package["commit"],
            "package_platform": package["platforms"][node["platform"]],
            "genesis_sha256": genesis["sha256"],
            "world_sha256": world["sha256"],
            "checkpoint_id": checkpoint["checkpoint_id"],
            "checkpoint_manifest_hash": checkpoint["manifest_hash"],
            "checkpoint_height": checkpoint["height"],
        }
        if node.get("bindings") != expected_binding:
            _fail(f"{node['name']} semantic binding is not derived from authenticated truth")
    try:
        probe = _object(plan.get("fresh_root_probe"), "plan fresh-root probe")
        normalized_probe = planner._validate_probe(
            probe,
            normalized_truth,
            set(CANONICAL_SIGNER_ALLOWLIST),
            {
                "transaction_id": plan["transaction_id"],
                "capture_window_id": plan["capture_window_id"],
            },
        )
    except (SystemExit, KeyError, TypeError) as error:
        _fail(f"plan fresh-root probe semantic validation failed: {error}")
    if normalized_probe != plan["fresh_root_probe"]:
        _fail("plan fresh-root probe is not the authenticated canonical projection")
    _validate_trusted_receipt_identity(
        normalized_probe.get("receipt"), "fresh_root_probe.receipt"
    )
    _validate_trusted_receipt_identity(
        _object(plan.get("adapter_verification"), "planner adapter verification").get("receipt"),
        "adapter_verification.receipt",
    )
    expected_gate = {
        "required_before": ["windows-observer", "macos-observer"],
        "fresh_root_probe_required": True,
        "checkpoint_receipt_required": True,
        "fail_closed": True,
    }
    if plan.get("observer_gate") != expected_gate:
        _fail("plan observer gate is not the code-owned fail-closed contract")


def validate_plan(plan: dict[str, Any]) -> dict[str, Any]:
    """Validate immutable planner output and all code-owned inventories."""
    plan = _object(plan, "plan")
    if plan.get("schema_version") != PLAN_SCHEMA:
        _fail("plan schema is unsupported")
    actual_digest = _digest(plan.get("plan_digest"), "plan_digest")
    if actual_digest != canonical_plan_digest(plan):
        _fail("plan digest does not match the frozen plan contents")
    planner = _load_planner()
    capture_window_bounds = _capture_window_bounds(plan)
    _consumer_impact_locator(plan)
    authority_plan = _object(plan.get("authority"), "plan authority")
    if authority_plan.get("consumer_impact_record") != plan["consumer_impact_record"]:
        _fail("plan authority is not bound to the consumer impact record")
    _validate_planner_authority(plan)
    if plan.get("node_order") != list(planner.NODE_ORDER):
        _fail("plan node order is not the code-owned five-node order")
    if plan.get("canonical_host_inventory") != planner.CANONICAL_HOST_INVENTORY:
        _fail("plan host inventory is not code-owned")
    if plan.get("canonical_endpoint_inventory") != planner.CANONICAL_ENDPOINT_INVENTORY:
        _fail("plan endpoint inventory is not code-owned")
    if getattr(planner, "CANONICAL_PEER_REGISTRY", None) != CANONICAL_PEER_REGISTRY:
        _fail("planner and adapter peer registries are not the same code-owned registry")
    deployment_inventory = _validate_deployment_inventory(
        plan, planner, capture_window_bounds
    )
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
    if surfaces.get("observer_count") != 8:
        _fail("observer surface summary must cover the governed eight surfaces")
    nodes = plan.get("nodes")
    if (
        not isinstance(nodes, list)
        or {node.get("name") for node in nodes if isinstance(node, dict)} != set(planner.NODE_ORDER)
    ):
        _fail("plan does not contain exactly the canonical five nodes")
    if [node.get("name") for node in nodes] != list(planner.NODE_ORDER):
        _fail("plan nodes are not in the code-owned five-node order")
    by_name: dict[str, dict[str, Any]] = {}
    seen_peer_ids: set[str] = set()
    for node_value in nodes:
        node = _object(node_value, "plan node")
        name = _string(node.get("name"), "plan node name")
        if name in by_name:
            _fail("plan contains duplicate node names")
        by_name[name] = node
        expected = planner.EXPECTED_NODES[name]
        for field, expected_value in expected.items():
            if field == "node_root":
                continue
            if node.get(field) != expected_value:
                _fail(f"{name} {field} is not the code-owned value")
        governed = deployment_inventory["nodes"][name]
        if node.get("node_id") != governed["node_id"]:
            _fail(f"{name} node id is not bound to deployment inventory")
        path_style = "windows" if expected["platform"] == "windows-x64" else "posix"
        normalized_root = planner._normalized_path(
            governed["node_root"], path_style, f"{name}.deployment_node_root"
        )
        if node.get("node_root") != normalized_root:
            _fail(f"{name} node root is not bound to deployment inventory")
        binding = _object(node.get("host_binding"), f"{name} host binding")
        if binding != planner.CANONICAL_HOST_INVENTORY[name]:
            _fail(f"{name} known-host target or pin is not code-owned")
        if _object(node.get("endpoints"), f"{name} endpoints") != planner.CANONICAL_ENDPOINT_INVENTORY[name]:
            _fail(f"{name} endpoint binding is not code-owned")
        identity = _object(node.get("identity_receipt"), f"{name} identity receipt")
        if identity.get("schema_version") != IDENTITY_RECEIPT_SCHEMA:
            _fail(f"{name} identity receipt schema is unsupported")
        if set(identity) != IDENTITY_RECEIPT_FIELDS:
            missing = IDENTITY_RECEIPT_FIELDS - set(identity)
            extra = set(identity) - IDENTITY_RECEIPT_FIELDS
            if missing:
                _fail(
                    f"{name} identity receipt freshness fields are incomplete: "
                    + ", ".join(sorted(missing))
                )
            _fail(
                f"{name} identity receipt fields are not exact: "
                + ", ".join(sorted(extra))
            )
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
        _validate_receipt_freshness(
            identity,
            f"{name} identity receipt",
            plan["capture_window_id"],
            capture_window_bounds=capture_window_bounds,
        )
        _reject_secret_fields(identity, f"{name} identity receipt")
        _nonzero_hex(identity.get("signed_payload_sha256"), HEX64_RE, f"{name} identity payload")
        _nonzero_hex(identity.get("signature_hex"), SIGNATURE_RE, f"{name} identity signature")
        _nonzero_hex(identity.get("canonical_digest"), HEX64_RE, f"{name} identity digest")
        if identity.get("canonical_digest") != _canonical_receipt_digest(
            identity, excluded_fields=frozenset({"peer_id"})
        ):
            _fail(f"{name} identity receipt canonical digest is not independently bound")
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
        if (
            identity["key_uid"] != governed["expected_key_uid"]
            or identity["key_gid"] != governed["expected_key_gid"]
        ):
            _fail(f"{name} identity uid/gid do not match independently authenticated deployment inventory")
        _validate_identity_raw_v1_digest(identity, f"{name} identity receipt")
        peer_id = _string(identity.get("peer_id"), f"{name} identity peer id")
        expected_peer_id = governed.get("peer_id", CANONICAL_PEER_REGISTRY[name])
        if peer_id != expected_peer_id:
            _fail(f"{name} identity peer id does not match authenticated deployment inventory")
        if peer_id in seen_peer_ids:
            _fail(f"{name} identity peer id duplicates another managed node")
        seen_peer_ids.add(peer_id)
        paths = _safe_relative_paths(
            node["node_root"],
            node.get("persistent_state_paths"),
            node["platform"],
            f"{name} state paths",
        )
        if paths != governed["persistent_state_paths"]:
            _fail(f"{name} state paths do not match its exact reset surfaces")
    expected_observers = {
        name: by_name[name]["persistent_state_paths"]
        for name in planner.OBSERVER_NAMES
    }
    if surfaces.get("observers_by_node") != expected_observers:
        _fail("observer surface summary is not bound to governed node inventory")
    _validate_plan_semantic_bindings(plan, planner, nodes)
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
    capture_window = _object(plan.get("capture_window"), "transaction capture window")
    if set(capture_window) != {"id", "starts_at", "ends_at"}:
        _fail("transaction capture window contains an unsafe field")
    if capture_window.get("id") != plan["capture_window_id"]:
        _fail("transaction capture window id binding drifted")
    window_start = _parse_utc(capture_window.get("starts_at"), "transaction capture window starts_at")
    window_end = _parse_utc(capture_window.get("ends_at"), "transaction capture window ends_at")
    if window_end <= window_start:
        _fail("transaction capture window is inverted")
    ledger_contract = _object(plan.get("credential_nonce_ledger"), "credential nonce ledger contract")
    _validate_nonce_contract(plan, nodes, ledger_contract, capture_window)
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
    return {
        "nodes": list(by_name.values()),
        "planner": planner,
        "plan_digest": actual_digest,
        "deployment_inventory": deployment_inventory,
        "capture_window_id": plan["capture_window_id"],
    }


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
    if (
        issued_at > dt.datetime.now(dt.timezone.utc) + dt.timedelta(seconds=MAX_CLOCK_SKEW_SECONDS)
        or expires_at <= issued_at
        or expires_at <= dt.datetime.now(dt.timezone.utc)
    ):
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
        "consumer_impact_record": _consumer_impact_locator(plan),
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
        "consumer_impact_record",
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
    if authority.get("consumer_impact_record") != plan["consumer_impact_record"]:
        _fail("adapter authority is not bound to the consumer impact record")
    trust_root_file = _object(authority.get("trust_root_file"), "pinned trust-root file contract")
    if set(trust_root_file) != {
        "path",
        "sha256",
        "root_digest",
        "owner_scope",
        "owner_uid",
        "mode",
        "regular_file",
    }:
        _fail("pinned trust-root file contract contains an unsafe field")
    if (
        trust_root_file.get("path") != CANONICAL_TRUST_ROOT_PATH
        or trust_root_file.get("sha256") != CANONICAL_TRUST_ROOT_FILE_SHA256
        or trust_root_file.get("root_digest") != CANONICAL_TRUST_ROOT_DIGEST
        or trust_root_file.get("owner_scope") != CANONICAL_TRUST_ROOT_OWNER_SCOPE
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
        "consumer_impact_record": _consumer_impact_locator(plan),
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
    _reject_symlink_ancestors(path, "trust-root")
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
        chunks: list[bytes] = []
        while True:
            chunk = os.read(descriptor, 1024 * 1024)
            if not chunk:
                break
            chunks.append(chunk)
            digest_builder.update(chunk)
        digest = digest_builder.hexdigest()
    except OSError:
        _fail("code-owned trust-root file content is unreadable")
    finally:
        if descriptor is not None:
            os.close(descriptor)
    if digest != CANONICAL_TRUST_ROOT_FILE_SHA256:
        _fail("code-owned trust-root file content digest drifted")
    try:
        root = json.loads(b"".join(chunks).decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        _fail("code-owned trust-root file is not valid JSON")
    if not isinstance(root, dict) or root.get("schema_version") != "oasis7.validator_pair_provenance_trust_root.v1":
        _fail("code-owned trust-root file schema is unsupported")
    root_digest = root.get("root_digest")
    if not isinstance(root_digest, str) or HEX64_RE.fullmatch(root_digest) is None:
        _fail("code-owned trust-root file root_digest is malformed")
    canonical_body = {key: value for key, value in root.items() if key != "root_digest"}
    canonical_digest = hashlib.sha256(
        json.dumps(canonical_body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if root_digest.lower() != canonical_digest or root_digest.lower() != CANONICAL_TRUST_ROOT_DIGEST:
        _fail("code-owned trust-root file canonical root_digest drifted")
    allowlist = root.get("allowlist")
    if not isinstance(allowlist, list) or not allowlist:
        _fail("code-owned trust-root file allowlist is missing")
    for entry in allowlist:
        if (
            not isinstance(entry, dict)
            or not isinstance(entry.get("signer_id"), str)
            or not entry["signer_id"].strip()
            or not isinstance(entry.get("algorithm"), str)
            or not entry["algorithm"].strip()
            or not isinstance(entry.get("public_key_sha256"), str)
            or HEX64_RE.fullmatch(entry["public_key_sha256"]) is None
        ):
            _fail("code-owned trust-root file allowlist entry is malformed")
        public_key_hex = entry.get("public_key_hex")
        if public_key_hex is not None and (
            not isinstance(public_key_hex, str) or re.fullmatch(r"[0-9a-fA-F]{64}", public_key_hex) is None
        ):
            _fail("code-owned trust-root file public key is malformed")
    return {
        "path": CANONICAL_TRUST_ROOT_PATH,
        "sha256": digest,
        "root_digest": root_digest.lower(),
        "owner_scope": CANONICAL_TRUST_ROOT_OWNER_SCOPE,
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
    _reject_symlink_ancestors(path, "credential nonce ledger")
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


def _nonce_reservation_state(
    plan: dict[str, Any], reserved_count: int, *, complete: bool | None = None
) -> dict[str, Any]:
    """Return journal-safe reservation state without exposing nonce values."""
    expected_count = len(plan["nodes"])
    if not isinstance(reserved_count, int) or reserved_count < 0 or reserved_count > expected_count:
        _fail("nonce reservation count is outside the plan boundary")
    return {
        "ledger_path": plan["credential_nonce_ledger"]["path"],
        "transaction_id": plan["transaction_id"],
        "expected_count": expected_count,
        "reserved_count": reserved_count,
        "one_shot": True,
        "complete": reserved_count == expected_count if complete is None else complete,
    }


def _read_plan_nonce_reservations(plan: dict[str, Any], path: Path) -> set[str]:
    """Read matching one-shot reservations without changing the ledger."""
    rows = _read_ledger(Path(path))
    expected = {_node_nonce(plan, node) for node in plan["nodes"]}
    reserved: set[str] = set()
    seen: set[str] = set()
    for row in rows:
        nonce = row["nonce"]
        if nonce in seen:
            _fail("credential nonce ledger contains a replayed nonce")
        seen.add(nonce)
        if nonce not in expected:
            continue
        if row["transaction_id"] != plan["transaction_id"]:
            _fail("plan nonce is reserved by a different transaction")
        reserved.add(nonce)
    return reserved


def _reservation_state_from_ledger(plan: dict[str, Any], path: Path) -> dict[str, Any]:
    """Derive the count-only audit state from the authoritative ledger."""
    reserved = _read_plan_nonce_reservations(plan, Path(path))
    expected = {_node_nonce(plan, node) for node in plan["nodes"]}
    return _nonce_reservation_state(
        plan, len(reserved), complete=reserved == expected
    )


def _reconcile_nonce_reservations(plan: dict[str, Any], path: Path) -> dict[str, Any]:
    """Reuse matching reservations and append only missing plan nonces.

    The ledger remains the sole nonce-value authority.  The returned state is
    deliberately count-only so journals never become a second credential
    store.
    """
    expected = {_node_nonce(plan, node) for node in plan["nodes"]}
    reserved = _read_plan_nonce_reservations(plan, Path(path))
    for node in plan["nodes"]:
        nonce = _node_nonce(plan, node)
        if nonce not in reserved:
            reserve_nonce(Path(path), plan["transaction_id"], nonce)
            reserved.add(nonce)
    return _nonce_reservation_state(plan, len(reserved), complete=len(reserved) == len(expected))


def _validate_committed_nonce_reservations(
    plan: dict[str, Any], path: Path, state: dict[str, Any]
) -> dict[str, Any]:
    """Require a preflight checkpoint's nonce reservations to still exist.

    A preflight-complete journal is allowed to resume only when the external
    one-shot ledger still contains every plan nonce under this transaction.
    It must never silently reserve a missing nonce after the checkpoint: that
    would make the checkpoint's evidence and credential boundary ambiguous.
    """
    expected = {_node_nonce(plan, node) for node in plan["nodes"]}
    reserved = _read_plan_nonce_reservations(plan, Path(path))
    if reserved != expected:
        _fail("preflight-complete checkpoint is missing a committed nonce reservation")
    if state.get("reserved_count") != len(reserved) or state.get("complete") is not True:
        _fail("preflight-complete checkpoint nonce state does not match the ledger")
    return _nonce_reservation_state(plan, len(reserved), complete=True)


def _validate_nonce_reservation_state(plan: dict[str, Any], raw: Any) -> dict[str, Any]:
    """Validate count-only journal state without treating it as ledger authority."""
    state = _object(raw, "transaction journal nonce reservation state")
    if set(state) != {
        "ledger_path",
        "transaction_id",
        "expected_count",
        "reserved_count",
        "one_shot",
        "complete",
    }:
        _fail("transaction journal nonce reservation state is incomplete")
    expected = _nonce_reservation_state(plan, len(plan["nodes"]))
    if (
        state.get("ledger_path") != expected["ledger_path"]
        or state.get("transaction_id") != expected["transaction_id"]
        or state.get("expected_count") != expected["expected_count"]
        or state.get("one_shot") is not True
    ):
        _fail("transaction journal nonce reservation state is not plan-bound")
    reserved_count = state.get("reserved_count")
    if not isinstance(reserved_count, int) or isinstance(reserved_count, bool) or not 0 <= reserved_count <= expected["expected_count"]:
        _fail("transaction journal nonce reservation count is malformed")
    if state.get("complete") is not (reserved_count == expected["expected_count"]):
        _fail("transaction journal nonce reservation completion flag drifted")
    return copy.deepcopy(state)


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
    expected_uid = _provider_uid(plan, name)
    if evidence.get("provider_uid") != expected_uid:
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


def _reject_symlink_ancestors(path: Path, label: str = "transaction journal") -> None:
    """Reject symlinked path components before creating/opening governed files."""
    raw_parts = Path(os.fspath(path)).parts
    if ".." in raw_parts:
        _fail(f"{label} path must not contain parent traversal")
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
            _fail(f"{label} path metadata is unavailable")
        if stat.S_ISLNK(metadata.st_mode):
            _fail(f"{label} path must not contain a symlink ancestor")
        if current != absolute and not stat.S_ISDIR(metadata.st_mode):
            _fail(f"{label} path ancestor is not a directory")


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
    schema_version = record.get("schema_version")
    if schema_version == LEGACY_JOURNAL_SCHEMA:
        _fail(
            "transaction journal schema v1 is unsupported; migration requires a "
            "new v2 journal or governed reconciliation before resuming"
        )
    if schema_version != JOURNAL_SCHEMA or record.get("journal_digest") != journal_digest(record):
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
    preflight_evidence_receipts: list[dict[str, Any]] | None = None,
    preflight_status: str = "pending",
    nonce_reservation_state: dict[str, Any] | None = None,
    backup_status: str = "not-needed",
    backup_error: str | None = None,
) -> dict[str, Any]:
    record: dict[str, Any] = {
        "schema_version": JOURNAL_SCHEMA,
        "adapter_schema": ADAPTER_SCHEMA,
        "task_uid": plan["task_uid"],
        "frozen_head_oid": plan["head_oid"],
        "plan_digest": plan["plan_digest"],
        "transaction_id": plan["transaction_id"],
        "capture_window_id": plan["capture_window_id"],
        "consumer_impact_record": _consumer_impact_locator(plan),
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
        "preflight_evidence_receipts": copy.deepcopy(preflight_evidence_receipts or []),
        "preflight_status": preflight_status,
        "nonce_reservation_state": copy.deepcopy(
            nonce_reservation_state or _nonce_reservation_state(plan, 0)
        ),
        "backup_status": backup_status,
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
    if backup_error is not None:
        record["backup_error"] = backup_error
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
        identity = _object(node.get("identity_receipt"), f"{node_name} identity receipt")
        return _string(identity.get("peer_id"), f"{node_name} deployment peer")
    if operation == "fresh-root-probe":
        return CANONICAL_PROBE_PEER_ID
    return CANONICAL_FLEET_PEER_ID


def _provider_uid(plan: dict[str, Any], node_name: str) -> int:
    node = next((item for item in plan["nodes"] if item["name"] == node_name), None)
    if node is None:
        _fail("provider evidence names an unknown node")
    identity = _object(node.get("identity_receipt"), f"{node_name} identity receipt")
    value = identity.get("key_uid")
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        _fail(f"{node_name} deployment uid is malformed")
    return value


def _verify_receipt_with_verifier(
    plan: dict[str, Any],
    receipt: dict[str, Any],
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None,
) -> None:
    if verifier is None:
        return
    # Revalidate the live consumer-impact record at the exact boundary where
    # an externally supplied verifier is about to run.  A receipt cannot
    # authorize work after the plan-bound impact record has drifted.
    transport_plan = _transport_plan(plan)
    _consumer_impact_locator(plan)
    try:
        # A provider verifier never needs nonce seams or other adapter-only
        # authority material.  Keep that boundary identical to the transport
        # DTO boundary.
        result = verifier(transport_plan, receipt)
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
    _consumer_impact_locator(plan)


def _verify_plan_receipts_with_verifier(
    plan: dict[str, Any],
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None,
) -> None:
    """Verify inventory and identity evidence through the existing seam.

    Planner receipts predate the transport receipt envelope and therefore do
    not carry provider-operation bindings.  The adapter projects each receipt
    into a verifier-only envelope with exact plan/node bindings; the original
    plan is never rewritten or sent as an apply proof.  Run every nested
    callback before surfacing the first failure so an operator receives a
    complete verification attempt for the governed inventory.
    """
    if verifier is None:
        _fail("deployment inventory and identity receipts require an independent verifier")
    inventory = _object(plan.get("deployment_inventory"), "deployment inventory")
    inventory_receipt = _object(inventory.get("receipt"), "deployment inventory receipt")
    evidence: list[dict[str, Any]] = [
        {
            "receipt": inventory_receipt,
            "bindings": {
                "kind": "deployment-inventory",
                "task_uid": plan["task_uid"],
                "frozen_head_oid": plan["head_oid"],
                "plan_digest": plan["plan_digest"],
                "consumer_impact_record": _consumer_impact_locator(plan),
            },
        }
    ]
    for node in plan["nodes"]:
        identity = _object(node.get("identity_receipt"), f"{node['name']} identity receipt")
        evidence.append(
            {
                "receipt": identity,
                "bindings": {
                    "kind": "identity",
                    "task_uid": plan["task_uid"],
                    "frozen_head_oid": plan["head_oid"],
                    "plan_digest": plan["plan_digest"],
                    "node": node["name"],
                    "node_id": node["node_id"],
                    "peer_id": identity["peer_id"],
                    "key_sha256": identity["key_sha256"],
                    "key_size_bytes": identity["key_size_bytes"],
                    "key_mode": identity["key_mode"],
                    "key_uid": identity["key_uid"],
                    "key_gid": identity["key_gid"],
                    "capture_window_id": identity["capture_window_id"],
                    "rotation_epoch": identity["rotation_epoch"],
                    "issued_at": identity["issued_at"],
                    "expires_at": identity["expires_at"],
                    "signed_payload_sha256": identity["signed_payload_sha256"],
                    "consumer_impact_record": _consumer_impact_locator(plan),
                },
            }
        )
    failures: list[str] = []
    for item in evidence:
        verifier_receipt = copy.deepcopy(item["receipt"])
        verifier_receipt["bindings"] = item["bindings"]
        try:
            _verify_receipt_with_verifier(plan, verifier_receipt, verifier)
        except AdapterError as error:
            failures.append(str(error))
    if failures:
        _fail(failures[0])


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
    # A provider callback may return a receipt only after the exact impact
    # source remains unchanged for the callback/receipt boundary.
    _consumer_impact_locator(plan)
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
        "consumer_impact_record",
    }
    _reject_secret_fields(receipt, f"{operation} provider receipt")
    if set(receipt) - allowed:
        _fail(f"{operation} provider receipt contains an unsafe field")
    if (
        "consumer_impact_record" in receipt
        and receipt["consumer_impact_record"] != plan["consumer_impact_record"]
    ):
        _fail(f"{operation} provider receipt consumer-impact record drifted")
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
    if captured_at > dt.datetime.now(dt.timezone.utc) + dt.timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
        _fail(f"{operation} captured_at is beyond the allowed future clock skew")
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
        "consumer_impact_record": _consumer_impact_locator(plan),
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
        elif receipt.get("seed_eligible") is not False:
            _fail(f"{operation} no-backup receipt is seed eligible")
        elif receipt.get("backup_manifest") is not None:
            manifest = _object(receipt["backup_manifest"], f"{operation} backup manifest")
            if manifest.get("seed_eligible") is not False:
                _fail(f"{operation} no-backup receipt contains a seed-eligible manifest")
    if operation == "fleet-health":
        closure = _object(receipt.get("fleet_health_closure"), "fleet-health closure")
        if (
            set(closure) != {"verified", "nodes", "healthy", "snapshot"}
            or closure.get("verified") is not True
            or closure.get("healthy") is not True
            or closure.get("nodes") != list(plan["node_order"])
        ):
            _fail("fleet-health receipt does not close the governed fleet")
        snapshot = _object(closure.get("snapshot"), "fleet-health final snapshot")
        if set(snapshot) != set(plan["node_order"]):
            _fail("fleet-health final snapshot does not cover the governed fleet")
        heights: dict[str, dict[str, int]] = {}
        for name in plan["node_order"]:
            node_snapshot = _object(snapshot.get(name), f"fleet-health snapshot {name}")
            if node_snapshot.get("running") is not True or node_snapshot.get("last_error") is not None:
                _fail(f"fleet-health snapshot {name} is not running and error-free")
            node_heights: dict[str, int] = {}
            for field in ("committed_height", "network_committed_height", "last_execution_height"):
                value = node_snapshot.get(field)
                if not isinstance(value, int) or isinstance(value, bool) or value < 0:
                    _fail(f"fleet-health snapshot {name} has malformed {field}")
                node_heights[field] = value
            heights[name] = node_heights
            connected_peers = node_snapshot.get("connected_peers")
            if not isinstance(connected_peers, list) or not all(
                isinstance(peer, str) and peer for peer in connected_peers
            ):
                _fail(f"fleet-health snapshot {name} connected_peers is malformed")
            if name not in {"storage-205", "sequencer-204"}:
                required_peers = {
                    _provider_peer(plan, "storage-205", "stop:storage-205"),
                    _provider_peer(plan, "sequencer-204", "stop:sequencer-204"),
                }
                if not required_peers.issubset(set(connected_peers)):
                    _fail(f"fleet-health snapshot {name} does not see both validator peers")
            elif name == "storage-205":
                required_peer = _provider_peer(plan, "sequencer-204", "stop:sequencer-204")
                if required_peer not in connected_peers:
                    _fail("fleet-health storage-205 does not see sequencer-204 validator peer")
            else:
                required_peer = _provider_peer(plan, "storage-205", "stop:storage-205")
                if required_peer not in connected_peers:
                    _fail("fleet-health sequencer-204 does not see storage-205 validator peer")
            readiness = _object(node_snapshot.get("readiness"), f"fleet-health snapshot {name} readiness")
            if readiness.get("ready") is not True or readiness.get("failed_gates") != []:
                _fail(f"fleet-health snapshot {name} readiness is not closed")
            consensus = _object(node_snapshot.get("consensus"), f"fleet-health snapshot {name} consensus")
            network_head = _object(
                consensus.get("network_head"),
                f"fleet-health snapshot {name} network head",
            )
            if network_head.get("decision") != "ready":
                _fail(f"fleet-health snapshot {name} network head is not ready")
        validator_head = heights["sequencer-204"]["committed_height"]
        for name, node_heights in heights.items():
            if any(value != validator_head for value in node_heights.values()):
                _fail(f"fleet-health snapshot {name} heights do not equal the sequencer head")
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
        identity = _object(node.get("identity_receipt"), f"{name} identity receipt")
        expected_providers.append(
            {
                "node": name,
                "node_id": node["node_id"],
                "peer_id": _string(identity.get("peer_id"), f"{name} deployment peer"),
                "provider_uid": _provider_uid(plan, name),
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


def _validate_journal_preflight_evidence_receipts(
    plan: dict[str, Any],
    raw: Any,
    verifier: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None,
) -> list[dict[str, Any]]:
    """Revalidate the signed evidence receipts captured by remote preflight.

    The evidence object is intentionally not replayed through a provider.  Its
    signed ``evidence_sha256`` binding is the durable read-only observation;
    resume must validate that exact preflight receipt rather than accepting a
    generic provider receipt for the same phase.
    """
    if not isinstance(raw, list):
        _fail("transaction journal preflight_evidence_receipts must be a list")
    if len(raw) > len(plan["node_order"]):
        _fail("transaction journal contains too many preflight evidence receipts")
    result: list[dict[str, Any]] = []
    seen_nodes: set[str] = set()
    for index, value in enumerate(raw):
        receipt = _object(value, "transaction journal preflight evidence receipt")
        node_name = receipt.get("node")
        if not isinstance(node_name, str) or node_name not in plan["node_order"]:
            _fail("transaction journal preflight evidence receipt names an unknown node")
        if node_name != plan["node_order"][index]:
            _fail("transaction journal preflight evidence receipts are out of canonical node order")
        if node_name in seen_nodes:
            _fail("transaction journal contains duplicate preflight evidence receipts")
        operation = f"preflight:{node_name}"
        if receipt.get("operation") != operation:
            _fail("transaction journal preflight evidence receipt operation drifted")
        bindings = _object(receipt.get("bindings"), "transaction journal preflight evidence bindings")
        if "evidence_sha256" not in bindings:
            _fail("transaction journal preflight evidence receipt is not evidence-bound")
        result.append(_validate_provider_receipt(plan, operation, node_name, receipt, verifier))
        seen_nodes.add(node_name)
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
        provenance_plan = copy.deepcopy(plan)
        _consumer_impact_locator(plan)
        result = verify_repository_provenance_helper(provenance_plan, authority)
        if len(receipts) != 1:
            _fail("no-backup authority requires the independent receipt verifier callback")
        validate_result(receipts[0], result)
        _consumer_impact_locator(plan)
    else:
        for receipt in receipts:
            transport_plan = _transport_plan(plan)
            _consumer_impact_locator(plan)
            try:
                result = verifier(transport_plan, receipt)
            except Exception as error:
                _fail(f"external provenance verifier failed: {error.__class__.__name__}")
            validate_result(receipt, result)
            _consumer_impact_locator(plan)
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


def _project_exact_object(
    value: Any, allowed: set[str], label: str
) -> dict[str, Any]:
    """Copy an object only when every nested schema key is explicitly known."""
    value = _object(value, label)
    if set(value) != allowed:
        _fail(f"{label} contains a field outside the recursive transport allowlist")
    return {key: copy.deepcopy(value[key]) for key in allowed}


def _project_transport_string_list(value: Any, label: str) -> list[str]:
    if not isinstance(value, list):
        _fail(f"{label} must be a list")
    return [_string(item, f"{label} entry") for item in value]


def _project_transport_list(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        _fail(f"{label} must be a list")
    return value


_TRANSPORT_RECEIPT_FIELDS = {
    "schema_version",
    "authenticated",
    "verified",
    "signer_id",
    "verifier_id",
    "trust_root_id",
    "signed_payload_sha256",
    "signature_hex",
    "canonical_digest",
}
_TRANSPORT_INVENTORY_RECEIPT_FIELDS = _TRANSPORT_RECEIPT_FIELDS | {
    "capture_window_id",
    "rotation_epoch",
    "issued_at",
    "expires_at",
}


def _project_transport_receipt(value: Any, label: str) -> dict[str, Any]:
    return _project_exact_object(value, _TRANSPORT_RECEIPT_FIELDS, label)


def _project_transport_truth(value: Any) -> dict[str, Any]:
    value = _object(value, "transport truth")
    if set(value) != {"package", "genesis", "world", "execution", "checkpoint"}:
        _fail("transport truth contains a field outside the recursive allowlist")
    package = _project_exact_object(
        value["package"],
        {
            "package_id", "package_dir", "provenance_path", "provenance_sha256",
            "provenance_size_bytes", "commit", "package_version", "runtime_sha256",
            "runtime_size_bytes", "genesis_sha256", "world_sha256", "platforms", "receipt",
        },
        "transport truth package",
    )
    platforms = _object(package["platforms"], "transport truth package platforms")
    if set(platforms) != {"linux-x64", "windows-x64", "macos-arm64"}:
        _fail("transport truth package platforms are not the managed set")
    package["platforms"] = {
        platform: _project_exact_object(
            platforms[platform],
            {
                "package_sha256", "package_size_bytes", "world_sha256", "world_size_bytes",
                "world_provenance_sha256", "world_provenance_size_bytes", "commit",
            },
            f"transport truth package platform {platform}",
        )
        for platform in ("linux-x64", "windows-x64", "macos-arm64")
    }
    package["receipt"] = _project_transport_receipt(package["receipt"], "transport truth package receipt")
    genesis = _project_exact_object(
        value["genesis"],
        {"network_id", "chain_id", "world_id", "path", "size_bytes", "sha256", "receipt"},
        "transport truth genesis",
    )
    genesis["receipt"] = _project_transport_receipt(genesis["receipt"], "transport truth genesis receipt")
    world = _project_exact_object(
        value["world"],
        {
            "world_id", "generation", "path", "provenance_path", "size_bytes", "sha256",
            "provenance_sha256", "provenance_size_bytes", "receipt",
        },
        "transport truth world",
    )
    world["receipt"] = _project_transport_receipt(world["receipt"], "transport truth world receipt")
    execution = _project_exact_object(
        value["execution"],
        {"execution_records_root", "cas", "world_head", "generated_world_sidecar", "json_index_consistency"},
        "transport truth execution",
    )
    execution["execution_records_root"] = _project_exact_object(
        execution["execution_records_root"], {"path", "sha256", "size_bytes"},
        "transport truth execution records",
    )
    execution["cas"] = _project_exact_object(
        execution["cas"], {"root", "blake3", "size_bytes"}, "transport truth execution cas"
    )
    execution["world_head"] = _project_exact_object(
        execution["world_head"],
        {"path", "sha256", "size_bytes", "height", "block_hash", "state_root"},
        "transport truth execution world head",
    )
    execution["generated_world_sidecar"] = _project_exact_object(
        execution["generated_world_sidecar"],
        {
            "path", "sha256", "size_bytes", "provenance_path", "provenance_sha256",
            "provenance_size_bytes",
        },
        "transport truth execution sidecar",
    )
    execution["json_index_consistency"] = _project_exact_object(
        execution["json_index_consistency"],
        {
            "verified", "snapshot_sha256", "snapshot_size_bytes", "journal_sha256",
            "journal_size_bytes", "index_sha256", "index_size_bytes",
        },
        "transport truth execution index consistency",
    )
    checkpoint = _project_exact_object(
        value["checkpoint"],
        {
            "checkpoint_id", "manifest_hash", "height", "receipt_path", "size_bytes",
            "execution_block_hash", "execution_state_root", "sha256", "receipt",
        },
        "transport truth checkpoint",
    )
    checkpoint["receipt"] = _project_transport_receipt(
        checkpoint["receipt"], "transport truth checkpoint receipt"
    )
    return {
        "package": package,
        "genesis": genesis,
        "world": world,
        "execution": execution,
        "checkpoint": checkpoint,
    }


def _project_transport_inventory(value: Any) -> dict[str, Any]:
    inventory = _project_exact_object(
        value,
        {
            "schema_version", "authenticated", "verified", "signer_id", "trust_root_id",
            "nodes", "receipt",
        },
        "transport deployment inventory",
    )
    nodes = _object(inventory["nodes"], "transport deployment inventory nodes")
    if set(nodes) != set(CANONICAL_PEER_REGISTRY):
        _fail("transport deployment inventory node set is not canonical")
    inventory["nodes"] = {
        name: _project_exact_object(
            nodes[name],
            {
                "node_id", "peer_id", "node_root", "persistent_state_paths",
                "expected_key_uid", "expected_key_gid",
            },
            f"transport deployment inventory {name}",
        )
        for name in CANONICAL_PEER_REGISTRY
    }
    inventory["receipt"] = _project_exact_object(
        inventory["receipt"],
        _TRANSPORT_INVENTORY_RECEIPT_FIELDS,
        "transport deployment inventory receipt",
    )
    return inventory


def _project_transport_surfaces(value: Any) -> dict[str, Any]:
    raw = _object(value, "transport surfaces")
    input_fields = {"validators", "observers", "validator_count", "observer_count", "observers_by_node"}
    if set(raw) - input_fields:
        _fail("transport surfaces contains a field outside the recursive allowlist")
    # The planner retains its generic observer reset list for local validation,
    # but provider truth must consume only the node-aware inventory.  Accepting
    # that compatibility field here while omitting it from the DTO prevents two
    # competing observer truths from crossing the transport boundary.
    surfaces = _project_exact_object(
        {key: raw[key] for key in input_fields if key != "observers"},
        {"validators", "validator_count", "observer_count", "observers_by_node"},
        "transport surfaces",
    )
    observers = _object(surfaces["observers_by_node"], "transport observer surfaces by node")
    if set(observers) != {"linux-lan-observer", "windows-observer", "macos-observer"}:
        _fail("transport observer surface map is not canonical")
    surfaces["observers_by_node"] = {
        name: _project_transport_string_list(
            observers[name], f"transport observer surfaces {name}"
        )
        for name in observers
    }
    surfaces["validators"] = _project_transport_string_list(
        surfaces["validators"], "transport validator surfaces"
    )
    return surfaces


def _project_transport_host_inventory(value: Any) -> dict[str, Any]:
    inventory = _object(value, "transport canonical host inventory")
    if set(inventory) != set(CANONICAL_PEER_REGISTRY):
        _fail("transport canonical host inventory node set is not canonical")
    return {
        name: _project_exact_object(
            inventory[name],
            {"target", "known_hosts_path", "known_host_fingerprint"},
            f"transport canonical host inventory {name}",
        )
        for name in CANONICAL_PEER_REGISTRY
    }


def _project_transport_endpoint_inventory(value: Any) -> dict[str, Any]:
    inventory = _object(value, "transport canonical endpoint inventory")
    if set(inventory) != set(CANONICAL_PEER_REGISTRY):
        _fail("transport canonical endpoint inventory node set is not canonical")
    return {
        name: _project_exact_object(
            inventory[name],
            {"healthz", "evidence"},
            f"transport canonical endpoint inventory {name}",
        )
        for name in CANONICAL_PEER_REGISTRY
    }


def _project_transport_capture_window(value: Any) -> dict[str, Any]:
    return _project_exact_object(
        value, {"id", "starts_at", "ends_at"}, "transport capture window"
    )


def _project_transport_execution(value: Any) -> dict[str, Any]:
    return _project_exact_object(
        value,
        {
            "mode",
            "provider_mutation_performed",
            "provider_mutation_boundary",
            "plan_is_apply_proof",
            "apply_requires_fresh_adapter_receipt",
        },
        "transport execution",
    )


def _project_transport_fresh_root_probe(value: Any) -> dict[str, Any]:
    probe = _project_exact_object(
        value,
        {
            "schema_version",
            "authenticated",
            "verified",
            "transaction_id",
            "capture_window_id",
            "replayed",
            "post_validator_verify",
            "package_commit",
            "checkpoint_id",
            "manifest_hash",
            "height",
            "validator_verify_outputs",
            "receipt",
        },
        "transport fresh-root probe",
    )
    outputs = _object(
        probe["validator_verify_outputs"],
        "transport fresh-root probe validator verify outputs",
    )
    if set(outputs) != {"storage-205", "sequencer-204"}:
        _fail("transport fresh-root probe validator output set is not canonical")
    output_fields = {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "signed_payload_sha256",
        "signature_hex",
        "canonical_digest",
        "node",
        "transaction_id",
        "capture_window_id",
        "package_commit",
        "checkpoint_id",
        "manifest_hash",
        "height",
        "output_sha256",
    }
    probe["validator_verify_outputs"] = {
        name: _project_exact_object(
            outputs[name], output_fields, f"transport fresh-root probe validator output {name}"
        )
        for name in ("storage-205", "sequencer-204")
    }
    probe["receipt"] = _project_transport_receipt(
        probe["receipt"], "transport fresh-root probe receipt"
    )
    return probe


def _project_transport_observer_gate(value: Any) -> dict[str, Any]:
    gate = _project_exact_object(
        value,
        {"required_before", "fresh_root_probe_required", "checkpoint_receipt_required", "fail_closed"},
        "transport observer gate",
    )
    required_before = gate["required_before"]
    if not isinstance(required_before, list):
        _fail("transport observer gate required_before must be a list")
    gate["required_before"] = [
        _string(node, "transport observer gate required_before entry")
        for node in required_before
    ]
    return gate


def _project_transport_journal_contract(value: Any) -> dict[str, Any]:
    return _project_exact_object(
        value,
        {
            "authoritative",
            "apply_usable",
            "adapter_owned",
            "durable_receipt_required",
            "planner_output_is_not_apply_proof",
        },
        "transport operation journal contract",
    )


def _project_transport_adapter_verification(value: Any) -> dict[str, Any]:
    verification = _project_exact_object(
        value,
        {
            "schema_version",
            "authenticated",
            "verified",
            "adapter_id",
            "transaction_id",
            "capture_window_id",
            "live_receipts_verified",
            "credential_nonce_ledger_verified",
            "backup_or_no_backup_authority_verified",
            "apply_authority_granted",
            "durable_journal_authoritative",
            "durable_journal_receipt_required",
            "receipt",
        },
        "transport adapter verification",
    )
    verification["receipt"] = _project_transport_receipt(
        verification["receipt"], "transport adapter verification receipt"
    )
    return verification


def _project_transport_consumer_impact(value: Any) -> dict[str, Any]:
    impact = _project_exact_object(
        value, {"path", "sha256", "record"}, "transport consumer impact record"
    )
    impact["record"] = _project_exact_object(
        impact["record"],
        {
            "impact",
            "evidence_source",
            "timestamp",
            "validators_already_stopped",
            "outage_update_channel",
            "recovery_update_checkpoint",
            "producer_wording_approval",
            "decision",
        },
        "transport consumer impact contents",
    )
    return impact


def _transport_node(node: dict[str, Any]) -> dict[str, Any]:
    """Project node inventory through a recursive allowlist; seams never cross."""
    node = _object(node, "transport node")
    if set(node) - TRANSPORT_NODE_FIELDS - {"credential_seam"}:
        _fail("transport node contains a field outside the allowlist")
    result = _project_exact_object(
        {key: value for key, value in node.items() if key != "credential_seam"},
        TRANSPORT_NODE_FIELDS,
        "transport node",
    )
    result["host_binding"] = _project_exact_object(
        result["host_binding"], {"target", "known_hosts_path", "known_host_fingerprint"},
        "transport node host binding",
    )
    result["endpoints"] = _project_exact_object(
        result["endpoints"], {"healthz", "evidence"}, "transport node endpoints"
    )
    result["identity_receipt"] = _project_exact_object(
        result["identity_receipt"],
        _TRANSPORT_RECEIPT_FIELDS | {
            "node_id", "peer_id", "key_sha256", "key_size_bytes", "key_mode", "key_uid", "key_gid",
            "capture_window_id", "rotation_epoch", "issued_at", "expires_at",
        },
        "transport node identity receipt",
    )
    result["persistent_state_paths"] = [
        _string(path, "transport node persistent state path")
        for path in result["persistent_state_paths"]
    ]
    result["bindings"] = _project_exact_object(
        result["bindings"],
        {
            "package_commit", "package_platform", "genesis_sha256", "world_sha256",
            "checkpoint_id", "checkpoint_manifest_hash", "checkpoint_height",
        },
        "transport node bindings",
    )
    _reject_secret_fields(result, "transport node")
    return result


def _reject_transport_auth_aliases(value: Any, label: str) -> None:
    """Keep provider DTOs free of auth/header/metadata aliases at any depth."""
    if isinstance(value, dict):
        for key, child in value.items():
            if str(key).lower() in TRANSPORT_AUTH_ALIAS_FIELDS:
                _fail(f"{label} contains an authorization-bearing field: {key}")
            _reject_transport_auth_aliases(child, f"{label}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _reject_transport_auth_aliases(child, f"{label}[{index}]")


def _transport_plan(plan: dict[str, Any]) -> dict[str, Any]:
    plan = _object(plan, "transport plan")
    if set(plan) - TRANSPORT_PLAN_FIELDS - {"authority", "credential_nonce_ledger"}:
        _fail("transport plan contains a field outside the allowlist")
    result = {
        "schema_version": _string(plan.get("schema_version"), "transport plan schema version"),
        "task_uid": _string(plan.get("task_uid"), "transport plan task uid"),
        "head_oid": _string(plan.get("head_oid"), "transport plan head oid"),
        "plan_digest": _string(plan.get("plan_digest"), "transport plan digest"),
        "transaction_id": _string(plan.get("transaction_id"), "transport plan transaction id"),
        "capture_window_id": _string(
            plan.get("capture_window_id"), "transport plan capture window id"
        ),
        "capture_window": _project_transport_capture_window(plan.get("capture_window")),
        "node_order": _project_transport_string_list(
            plan.get("node_order"), "transport plan node order"
        ),
        "global_order": _project_transport_string_list(
            plan.get("global_order"), "transport plan global order"
        ),
        "canonical_host_inventory": _project_transport_host_inventory(
            plan.get("canonical_host_inventory")
        ),
        "canonical_endpoint_inventory": _project_transport_endpoint_inventory(
            plan.get("canonical_endpoint_inventory")
        ),
        "nodes": [
            _transport_node(node)
            for node in _project_transport_list(plan.get("nodes"), "transport plan nodes")
        ],
        "surfaces": _project_transport_surfaces(plan.get("surfaces")),
        "deployment_inventory": _project_transport_inventory(plan.get("deployment_inventory")),
        "truth": _project_transport_truth(plan.get("truth")),
        "execution": _project_transport_execution(plan.get("execution")),
        "fresh_root_probe": _project_transport_fresh_root_probe(plan.get("fresh_root_probe")),
        "observer_gate": _project_transport_observer_gate(plan.get("observer_gate")),
        "operation_journal_contract": _project_transport_journal_contract(
            plan.get("operation_journal_contract")
        ),
        "adapter_verification": _project_transport_adapter_verification(
            plan.get("adapter_verification")
        ),
        "consumer_impact_record": _project_transport_consumer_impact(
            plan.get("consumer_impact_record")
        ),
    }
    _reject_transport_auth_aliases(result, "transport plan")
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
    resume_record: dict[str, Any] | None = None,
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
    # Re-check the live record before any externally supplied verifier callback
    # can run, keeping the consumer-impact gate ahead of all apply work.
    _consumer_impact_locator(plan)
    provenance_verified = _verify_provenance(plan, authority, provenance_verifier)
    if resume_record is None:
        ledger_summary = validate_credential_ledger(plan, Path(ledger_path))
    else:
        resume_nonce_state = _object(
            resume_record.get("nonce_reservation_state"), "resume nonce state"
        )
        ledger_summary = {"rows": resume_nonce_state.get("reserved_count", 0)}
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
            "consumer_impact_record": _consumer_impact_locator(plan),
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
    # Nested deployment/identity evidence is an apply-only trust boundary.
    # Place it after the existing authorization gate so an unauthorized caller
    # still fails before any newly introduced verifier callbacks run.
    _verify_plan_receipts_with_verifier(plan, provenance_verifier)
    # Re-check the code-owned trust root against the live filesystem after
    # authority/provenance validation and before journal/nonce/provider work.
    _consumer_impact_locator(plan)
    validate_live_trust_root_file()
    # One-shot reservations precede every remote observation.  Values stay in
    # the external ledger and never enter a journal, receipt, or log.
    resumed = resume_record is not None
    provider_receipts: list[dict[str, Any]] = []
    preflight_evidence_receipts: list[dict[str, Any]] = []
    rollback_receipt: dict[str, Any] | None = None
    rollback_reobservation_receipt: dict[str, Any] | None = None
    rollback_status = "not-started"
    backup_status = "pending" if plan["forensic_backup"]["required_before_reset"] is True else "not-needed"
    preflight_status = "pending"
    nonce_state = _nonce_reservation_state(plan, 0)
    completed: list[str] = []
    started: list[str] = []
    rollback_candidates: list[str] = []
    node_receipts: dict[str, dict[str, Any]] = {}
    if resume_record is None:
        try:
            _write_journal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "prepared",
                    0,
                    [],
                    execution_mode="apply",
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status=preflight_status,
                    nonce_reservation_state=nonce_state,
                    backup_status=backup_status,
                ),
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
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status=preflight_status,
                    nonce_reservation_state=nonce_state,
                    backup_status=backup_status,
                ),
            )
            _fail("initial transaction journal write failed; reconciliation is blocked")
    else:
        provider_receipts = copy.deepcopy(resume_record.get("provider_receipts", []))
        preflight_evidence_receipts = copy.deepcopy(resume_record.get("preflight_evidence_receipts", []))
        completed = list(resume_record.get("completed_operations", []))
        node_receipts = copy.deepcopy(resume_record.get("node_receipts", {}))
        backup_status = resume_record.get("backup_status", backup_status)
        preflight_status = resume_record.get("preflight_status", "pending")
        nonce_state = copy.deepcopy(resume_record.get("nonce_reservation_state", nonce_state))
        if resume_record.get("status") == "prepared" and preflight_evidence_receipts:
            _fail("prepared journal must not contain preflight evidence")
        if resume_record.get("status") == "preflight-complete":
            nonce_state = _validate_committed_nonce_reservations(
                plan, Path(ledger_path), nonce_state
            )
    if resume_record is None or resume_record.get("status") == "prepared":
        try:
            if resume_record is None:
                reserved_count = 0
                for node in (plan["nodes"] if isinstance(plan.get("nodes"), list) else []):
                    reserve_nonce(Path(ledger_path), plan["transaction_id"], _node_nonce(plan, node))
                    reserved_count += 1
                nonce_state = _nonce_reservation_state(
                    plan, reserved_count, complete=reserved_count == len(plan["nodes"])
                )
            else:
                nonce_state = _reconcile_nonce_reservations(plan, Path(ledger_path))
            for node in plan["nodes"]:
                # Inspect is an externally supplied provider callback.  Keep
                # the consumer-impact binding at the exact callback edge so
                # an in-flight record mutation cannot be observed only after
                # remote evidence has already been collected.
                transport_node = _transport_node(node)
                _consumer_impact_locator(plan)
                evidence = transport.inspect_node(transport_node)
                validated_evidence = validate_remote_preflight(plan, node, evidence, provenance_verifier)
                preflight_evidence_receipts.append(validated_evidence["receipt"])
            _write_journal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "preflight-complete",
                    0,
                    [],
                    node_receipts=node_receipts,
                    provider_receipts=provider_receipts,
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status="complete",
                    nonce_reservation_state=nonce_state,
                    rollback_status=rollback_status,
                    execution_mode="apply",
                    backup_status=backup_status,
                ),
            )
            preflight_status = "complete"
        except Exception as error:
            nonce_readback_error: AdapterError | None = None
            try:
                nonce_state = _reservation_state_from_ledger(
                    plan, Path(ledger_path)
                )
            except AdapterError as readback_error:
                nonce_readback_error = readback_error
            if nonce_readback_error is not None:
                _persist_terminal(
                    Path(journal_path),
                    _journal_record(
                        plan,
                        "terminal-failure",
                        0,
                        [],
                        error=error.__class__.__name__,
                        provider_receipts=provider_receipts,
                        preflight_evidence_receipts=preflight_evidence_receipts,
                        preflight_status=preflight_status,
                        nonce_reservation_state=nonce_state,
                        rollback_status="reconciliation-blocked",
                        rollback_error="nonce-ledger-readback-failed",
                        execution_mode="apply",
                        backup_status=backup_status,
                    ),
                )
                _fail("nonce reservation or remote preflight failed; authoritative ledger readback is blocked")
            _persist_terminal(
                Path(journal_path),
                _journal_record(
                    plan,
                    "terminal-failure",
                    0,
                    [],
                    error=error.__class__.__name__,
                    provider_receipts=provider_receipts,
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status=preflight_status,
                    nonce_reservation_state=nonce_state,
                    rollback_status="not-needed",
                    execution_mode="apply",
                    backup_status=backup_status,
                ),
            )
            _fail("nonce reservation or remote preflight failed; transaction is terminal")
    for index, operation in enumerate(operations):
        node_name: str | None = None
        in_flight_journal_written = False
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
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status=preflight_status,
                    nonce_reservation_state=nonce_state,
                    rollback_status=rollback_status,
                    rollback_receipt=rollback_receipt,
                    execution_mode="apply",
                    backup_status=backup_status,
                ),
            )
            in_flight_journal_written = True
            if operation == "fresh-root-probe":
                transport_plan = _transport_plan(plan)
                _consumer_impact_locator(plan)
                raw_receipt = transport.verify_fresh_root_probe(transport_plan)
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
                    _consumer_impact_locator(plan)
                    raw_receipt = transport.preflight(operation, transport_node)
                elif phase == "verify":
                    _consumer_impact_locator(plan)
                    raw_receipt = transport.verify(operation, transport_node)
                elif operation == "fleet-health":
                    _consumer_impact_locator(plan)
                    raw_receipt = transport.health(operation)
                else:
                    # Append only after the exact pre-callback binding check:
                    # if it fails, no provider mutation has begun and the
                    # rollback path must not invoke another callback.
                    _consumer_impact_locator(plan)
                    if _rollback_candidate(operation) and operation not in rollback_candidates:
                        rollback_candidates.append(operation)
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
            if operation.startswith("forensic-backup:"):
                completed_backup_operations = sum(
                    1 for completed_operation in completed + [operation]
                    if completed_operation.startswith("forensic-backup:")
                )
                if completed_backup_operations == len(plan["nodes"]):
                    backup_status = "completed"
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
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status=preflight_status,
                    nonce_reservation_state=nonce_state,
                    rollback_status=journal_rollback_status,
                    rollback_receipt=rollback_receipt,
                    execution_mode="apply",
                    backup_status=backup_status,
                ),
            )
        except Exception as error:
            rollback_error: str | None = None
            failed_operation = operation
            failed_state_digest: str | None = None
            if operation.startswith("forensic-backup:") and not rollback_candidates:
                # Backup is read-only evidence capture.  A failed backup has
                # no provider mutation to reconcile, so clean-redeploy must
                # not be invoked with an empty candidate set.
                backup_status = "backup-failed"
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
                        "not-needed",
                        rollback_receipt,
                        execution_mode="apply",
                        failed_operation=failed_operation,
                        preflight_evidence_receipts=preflight_evidence_receipts,
                        preflight_status=preflight_status,
                        nonce_reservation_state=nonce_state,
                        backup_status=backup_status,
                        backup_error=error.__class__.__name__,
                    ),
                )
                _fail("forensic backup failed; no clean-redeploy rollback is required")
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
                        preflight_evidence_receipts=preflight_evidence_receipts,
                        preflight_status=preflight_status,
                        nonce_reservation_state=nonce_state,
                        backup_status=backup_status,
                    ),
                )
                _fail("read-only provider operation failed; no rollback is required")
            if not rollback_candidates and not _read_only_operation(operation):
                # A consumer-impact mismatch at the pre-callback boundary is
                # fail-closed.  Preserve the durable in-flight journal and do
                # not make any further externally supplied callback, including
                # re-observation or clean-redeploy rollback.
                if in_flight_journal_written:
                    _fail("provider mutation was blocked before callback; governed reconciliation is required")
                # If the in-flight journal itself could not be written, retain
                # the pre-existing rollback contract: reconcile a possible
                # earlier provider transition before reporting the failure.
                if _rollback_candidate(operation):
                    rollback_candidates.append(operation)
            try:
                validate_authority(plan, authority)
                if provenance_verifier is None:
                    _fail("rollback requires the independent provider receipt verifier callback")
                if _verify_provenance(plan, authority, provenance_verifier) is not True:
                    _fail("rollback requires a fresh independent provenance verification")
                rollback_plan = _transport_plan(plan)
                rollback_candidates_snapshot = list(rollback_candidates)
                _consumer_impact_locator(plan)
                rollback_reobservation_receipt = transport.reobserve_failed_state(
                    rollback_plan, rollback_candidates_snapshot, failed_operation
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
                rollback_plan = _transport_plan(plan)
                rollback_candidates_snapshot = list(rollback_candidates)
                _consumer_impact_locator(plan)
                rollback_receipt = transport.rollback_clean_redeploy(
                    rollback_plan, rollback_candidates_snapshot, rollback_reobservation_receipt
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
                    preflight_evidence_receipts=preflight_evidence_receipts,
                    preflight_status=preflight_status,
                    nonce_reservation_state=nonce_state,
                    backup_status=backup_status,
                ),
            )
            if rollback_status == "reconciliation-blocked":
                _fail("provider operation and clean-redeploy rollback failed; reconciliation is blocked")
            _fail("provider operation failed; transaction is terminal and requires governed reconciliation")
    return {
        "schema_version": ADAPTER_SCHEMA,
        "status": "complete",
        "dry_run": False,
        "resumed": resumed,
        "plan_digest": plan["plan_digest"],
        "transaction_id": plan["transaction_id"],
        "consumer_impact_record": _consumer_impact_locator(plan),
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
        ("consumer_impact_record", _consumer_impact_locator(plan)),
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
    preflight_evidence_receipts = _validate_journal_preflight_evidence_receipts(
        plan,
        record.get("preflight_evidence_receipts", []),
        provenance_verifier if not dry_run else None,
    )
    preflight_status = record.get("preflight_status")
    if preflight_status not in {"pending", "complete"}:
        _fail("transaction journal preflight status is unsupported")
    nonce_reservation_state = _validate_nonce_reservation_state(
        plan, record.get("nonce_reservation_state")
    )
    backup_status = record.get("backup_status")
    if backup_status not in {"not-needed", "pending", "completed", "backup-failed"}:
        _fail("transaction journal backup status is unsupported")
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
    if status == "preflight-complete":
        if (
            expected_mode != "apply"
            or preflight_status != "complete"
            or not nonce_reservation_state["complete"]
            or len(preflight_evidence_receipts) != len(plan["node_order"])
            or completed
            or provider_receipts
            or next_index != 0
        ):
            _fail("preflight-complete journal is not a safe mutation-free checkpoint")
    if status == "prepared" and expected_mode == "apply":
        if preflight_evidence_receipts:
            _fail("prepared journal must not contain preflight evidence")
        if preflight_status != "pending" or completed or provider_receipts or next_index != 0:
            _fail("prepared journal has crossed an unsafe operation boundary")
    if (
        expected_mode == "apply"
        and status == "complete"
        and plan["forensic_backup"]["required_before_reset"] is True
        and backup_status != "completed"
    ):
        _fail("completed apply journal lacks completed forensic-backup evidence")
    if expected_mode == "apply" and status == "complete":
        if {receipt["node"] for receipt in preflight_evidence_receipts} != set(plan["node_order"]):
            _fail("completed apply journal lacks the exact remote preflight evidence closure")
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
            "consumer_impact_record": _consumer_impact_locator(plan),
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
    if status in {"prepared", "preflight-complete"} and not dry_run:
        return _execute_unlocked(
            plan,
            authority,
            journal_path=Path(journal_path),
            ledger_path=Path(ledger_path),
            transport=transport,
            dry_run=False,
            provenance_verifier=provenance_verifier,
            resume_record=record,
        )
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
                preflight_evidence_receipts=preflight_evidence_receipts,
                backup_status=backup_status,
            ),
        )
        return {
            "schema_version": ADAPTER_SCHEMA,
            "status": "dry-run-complete",
            "resumed": True,
            "dry_run": True,
            "plan_digest": plan["plan_digest"],
            "transaction_id": plan["transaction_id"],
            "consumer_impact_record": _consumer_impact_locator(plan),
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
