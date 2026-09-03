#!/usr/bin/env python3
"""Build a governed, plan-only clean-room transaction for the five-node fleet.

This module deliberately has no provider transport or mutation implementation.
It turns an operator-supplied, already authenticated evidence envelope into a
deterministic transaction plan.  The plan describes forensic preservation and
clean redeploy; a forensic backup is never an eligible state seed and rollback
never restores old chain state.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any, NoReturn


INPUT_SCHEMA = "oasis7.public_testnet_full_network_clean_room_input.v1"
PLAN_SCHEMA = "oasis7.public_testnet_full_network_clean_room_plan.v1"
DEPLOYMENT_INVENTORY_SCHEMA = "oasis7.deployment_inventory.v2"
IDENTITY_RECEIPT_SCHEMA = "oasis7.identity_receipt.v2"
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
OID_RE = re.compile(r"^[0-9a-fA-F]{40,64}$")
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
SAFE_NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$")
SECRET_KEY_RE = re.compile(
    r"(?:password|secret|token|private[_-]?key|api[_-]?key|access[_-]?key|sshpass)",
    re.I,
)

NODE_ORDER = (
    "storage-205",
    "sequencer-204",
    "linux-lan-observer",
    "windows-observer",
    "macos-observer",
)
VALIDATOR_NAMES = frozenset({"storage-205", "sequencer-204"})
OBSERVER_NAMES = frozenset(set(NODE_ORDER) - VALIDATOR_NAMES)

# These are relative reset surfaces, not a seed/copy source.  The validator
# list is intentionally byte-for-byte aligned with the pair rebuild executor.
VALIDATOR_RESET_SURFACES = (
    "data/execution-records",
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/storage",
    "data/runtime-root",
    "data/replication-root",
    "output/chain-runtime",
    "output/node-distfs",
)
OBSERVER_RESET_SURFACES = (
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/execution-records",
    "data/storage",
    "data/replication-root",
    "data/runtime-root",
    "output/chain-runtime/{node_id}/reward-runtime-execution-bridge-state.json",
)
# The Linux LAN observer is installed at a root-level stack layout.  Its
# replication root may fall back to output/node-distfs/<node_id> when the
# optional REPLICATION_ROOT env binding is absent, so both governed locations
# are retained in the emitted state inventory.
LINUX_OBSERVER_PERSISTENT_STATE_SURFACES = (
    "world",
    "world-simulator-mirror",
    "execution-records",
    "store",
    "replication-root",
    "runtime-root",
    "output/chain-runtime/{node_id}/reward-runtime-execution-bridge-state.json",
    "output/node-distfs/{node_id}",
)

EXPECTED_NODES: dict[str, dict[str, str]] = {
    "storage-205": {
        "node_id": "triad-testnet-storage",
        "role": "validator",
        "platform": "linux-x64",
        "node_root": "/opt/oasis7/p2p-testnet",
        "service_manager": "systemd",
        "service": "oasis7-triad-storage.service",
    },
    "sequencer-204": {
        "node_id": "triad-testnet-sequencer",
        "role": "validator",
        "platform": "linux-x64",
        "node_root": "/opt/oasis7/p2p-testnet",
        "service_manager": "systemd",
        "service": "oasis7-triad-sequencer.service",
    },
    "linux-lan-observer": {
        "node_id": "triad-testnet-local",
        "role": "observer",
        "platform": "linux-x64",
        "node_root": "/opt/oasis7/p2p-testnet-local",
        "service_manager": "systemd",
        "service": "oasis7-testnet-observer.service",
    },
    "windows-observer": {
        "node_id": "triad-testnet-windows-observer",
        "role": "observer",
        "platform": "windows-x64",
        "node_root": r"C:\oasis7-deploy",
        "service_manager": "scheduled-task",
        "service": "Oasis7Observer",
    },
    "macos-observer": {
        "node_id": "triad-testnet-fourth-local",
        "role": "observer",
        "platform": "macos-arm64",
        "node_root": "/Applications/oasis7",
        "service_manager": "launchd",
        "service": "oasis7.testnet.fourth",
    },
}

# This registry is code-owned.  Identity receipts may attest a deployment
# peer, but caller-supplied unique values cannot redefine the managed fleet.
CANONICAL_PEER_REGISTRY = {
    "storage-205": "12D3KooWtriadtestnetstorage",
    "sequencer-204": "12D3KooWtriadtestnetsequencer",
    "linux-lan-observer": "12D3KooWtriadtestnetlocal",
    "windows-observer": "12D3KooWtriadtestnetwindowsobserver",
    "macos-observer": "12D3KooWtriadtestnetfourthlocal",
}

# Connection inventory is code-owned.  Callers may provide evidence for these
# exact bindings, but cannot select a different host, known_hosts file, or pin.
# Observer targets are operator aliases; no credential material is represented.
CANONICAL_HOST_INVENTORY: dict[str, dict[str, str]] = {
    "sequencer-204": {
        "target": "root@39.104.204.172",
        "known_hosts_path": "/opt/oasis7/p2p-testnet/config/public-testnet-validator-pair-known-hosts",
        "known_host_fingerprint": "SHA256:7NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
    },
    "storage-205": {
        "target": "root@39.104.205.67",
        "known_hosts_path": "/opt/oasis7/p2p-testnet/config/public-testnet-validator-pair-known-hosts",
        "known_host_fingerprint": "SHA256:1SVgiaT5JLCw8PsPpVfLE9UyWNf82IJDZsiE7LAa1gI",
    },
    "linux-lan-observer": {
        "target": "observer@linux-lan",
        "known_hosts_path": "/operator/known-hosts",
        "known_host_fingerprint": "SHA256:2NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
    },
    "windows-observer": {
        "target": "observer@windows-lan",
        "known_hosts_path": "/operator/known-hosts",
        "known_host_fingerprint": "SHA256:3NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
    },
    "macos-observer": {
        "target": "observer@macos-lan",
        "known_hosts_path": "/operator/known-hosts",
        "known_host_fingerprint": "SHA256:4NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
    },
}

CANONICAL_ENDPOINT_INVENTORY: dict[str, dict[str, str]] = {
    "sequencer-204": {
        "healthz": "http://127.0.0.1:6631/healthz",
        "evidence": "http://127.0.0.1:6631/v1/chain/rebuild-proof",
    },
    "storage-205": {
        "healthz": "http://127.0.0.1:6632/healthz",
        "evidence": "http://127.0.0.1:6632/v1/chain/status",
    },
    "linux-lan-observer": {
        "healthz": "http://127.0.0.1:6633/healthz",
        "evidence": "http://127.0.0.1:6633/v1/chain/status",
    },
    "windows-observer": {
        "healthz": "http://127.0.0.1:5121/healthz",
        "evidence": "http://127.0.0.1:5121/v1/chain/status",
    },
    "macos-observer": {
        "healthz": "http://127.0.0.1:19083/healthz",
        "evidence": "http://127.0.0.1:19083/v1/chain/status",
    },
}

CANONICAL_SIGNER_ALLOWLIST = frozenset({"governance-signer"})
CANONICAL_VERIFIER_ID = "governed-receipt-verifier"
CANONICAL_TRUST_ROOT_ID = "oasis7-public-testnet-governance-root-v1"
CANONICAL_ROTATION_EPOCH = "rotation-epoch-20260901-001"
CANONICAL_ADAPTER_ID = "external-clean-room-adapter"
CANONICAL_NETWORK_ID = "oasis7-public-testnet-governed-20260606"
CONSUMER_IMPACT_FIELDS = frozenset(
    {
        "impact",
        "evidence_source",
        "timestamp",
        "validators_already_stopped",
        "outage_update_channel",
        "recovery_update_checkpoint",
        "producer_wording_approval",
        "decision",
    }
)
CONSUMER_IMPACT_REFERENCE_FIELDS = frozenset({"path", "sha256"})
CONSUMER_IMPACT_MAX_AGE_SECONDS = 24 * 60 * 60
MAX_CLOCK_SKEW_SECONDS = 5
# The runtime producer remains the raw v1 metadata producer.  The v2 envelope
# carries the digest of those exact bytes; this path is the canonical key-path
# binding used when reconstructing the bytes for admission checks.
RAW_IDENTITY_RECEIPT_V1_KEY_PATH = "/operator/keys/node-keypair.toml"


def die(message: str) -> NoReturn:
    raise SystemExit(f"error: full-network clean-room: {message}")


def require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        die(f"{label} must be an object")
    return value


def require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        die(f"{label} must be a non-empty string")
    return value


def require_hex(value: Any, label: str) -> str:
    if not isinstance(value, str) or HEX64_RE.fullmatch(value) is None:
        die(f"{label} must be a 64-character hexadecimal digest")
    return value.lower()


def require_oid(value: Any, label: str) -> str:
    if not isinstance(value, str) or OID_RE.fullmatch(value) is None:
        die(f"{label} must be a hexadecimal commit oid")
    return value.lower()


def _parse_utc(value: Any, label: str) -> datetime:
    raw = require_string(value, label)
    try:
        parsed = datetime.fromisoformat(raw.replace("Z", "+00:00"))
    except ValueError:
        die(f"{label} must be an RFC3339 timestamp")
    if parsed.tzinfo is None:
        die(f"{label} must include a timezone")
    return parsed.astimezone(timezone.utc)


def _impact_locator(value: dict[str, Any]) -> dict[str, str]:
    return {"path": value["path"], "sha256": value["sha256"]}


def _validate_consumer_impact_record(value: Any, label: str = "consumer_impact_record") -> dict[str, Any]:
    reference = require_object(value, label)
    if set(reference) != CONSUMER_IMPACT_REFERENCE_FIELDS:
        die(f"{label} must contain only path and sha256")
    raw_path = require_string(reference.get("path"), f"{label}.path")
    path = _normalized_path(raw_path, "posix", f"{label}.path")
    digest = require_hex(reference.get("sha256"), f"{label}.sha256")
    path_obj = Path(path)
    if path_obj.is_symlink() or not path_obj.is_file():
        die(f"{label}.path must reference a regular immutable record file")
    try:
        payload = path_obj.read_bytes()
    except OSError as error:
        die(f"cannot read {label}.path: {error}")
    actual_digest = hashlib.sha256(payload).hexdigest()
    if actual_digest != digest:
        die(f"{label} path/sha256 binding mismatch")
    try:
        record = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        die(f"{label}.path does not contain valid JSON: {error}")
    record = require_object(record, f"{label}.record")
    if set(record) != CONSUMER_IMPACT_FIELDS:
        die(f"{label}.record schema fields are not exact")
    impact = require_string(record.get("impact"), f"{label}.record.impact")
    if impact not in {"active", "none", "unknown"}:
        die(f"{label}.record.impact is unsupported")
    require_string(record.get("evidence_source"), f"{label}.record.evidence_source")
    timestamp = _parse_utc(record.get("timestamp"), f"{label}.record.timestamp")
    age = (datetime.now(timezone.utc) - timestamp).total_seconds()
    if age < -MAX_CLOCK_SKEW_SECONDS:
        die(f"{label}.record.timestamp is in the future")
    if age > CONSUMER_IMPACT_MAX_AGE_SECONDS:
        die(f"{label}.record.timestamp is stale")
    if not isinstance(record.get("validators_already_stopped"), bool):
        die(f"{label}.record.validators_already_stopped must be boolean")
    for field in (
        "outage_update_channel",
        "recovery_update_checkpoint",
        "producer_wording_approval",
    ):
        field_value = require_string(record.get(field), f"{label}.record.{field}")
        if impact in {"active", "unknown"} and field_value.strip().lower() == "n/a":
            die(f"{label}.record.{field} must be bound for active or unknown impact")
    if record.get("decision") != "proceed":
        die(f"{label}.record.decision must be proceed")
    return {
        "path": path,
        "sha256": digest,
        "record": record,
    }


def _validate_transaction_context(request: dict[str, Any]) -> dict[str, str]:
    transaction_id = require_string(request.get("transaction_id"), "transaction_id")
    capture_window_id = require_string(request.get("capture_window_id"), "capture_window_id")
    for value, label in (
        (transaction_id, "transaction_id"),
        (capture_window_id, "capture_window_id"),
    ):
        if SAFE_NAME_RE.fullmatch(value) is None:
            die(f"{label} is not a safe transaction identifier")
    return {"transaction_id": transaction_id, "capture_window_id": capture_window_id}


def reject_secret_fields(value: Any, label: str = "input") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if SECRET_KEY_RE.search(str(key)):
                die(f"{label} contains a credential-bearing field: {key}")
            reject_secret_fields(child, f"{label}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            reject_secret_fields(child, f"{label}[{index}]")


def validate_authenticated_receipt(
    value: Any, label: str, allowed_signers: set[str] | None = None
) -> dict[str, Any]:
    receipt = require_object(value, label)
    schema = require_string(receipt.get("schema_version"), f"{label}.schema_version")
    if not schema.startswith("oasis7."):
        die(f"{label}.schema_version is not repository-owned")
    if receipt.get("authenticated") is not True or receipt.get("verified") is not True:
        die(f"{label} must be authenticated and independently verified")
    signer_id = require_string(receipt.get("signer_id"), f"{label}.signer_id")
    if allowed_signers is not None and signer_id not in allowed_signers:
        die(f"{label}.signer_id is not in the governed signer allowlist")
    signed_payload = require_hex(receipt.get("signed_payload_sha256"), f"{label}.signed_payload_sha256")
    if not any(character != "0" for character in signed_payload):
        die(f"{label}.signed_payload_sha256 must not be empty")
    signature = receipt.get("signature_hex")
    if (
        not isinstance(signature, str)
        or SIGNATURE_RE.fullmatch(signature) is None
        or not any(character != "0" for character in signature)
    ):
        die(f"{label}.signature_hex must be a complete Ed25519 signature")
    canonical_digest = require_hex(receipt.get("canonical_digest"), f"{label}.canonical_digest")
    if not any(character != "0" for character in canonical_digest):
        die(f"{label}.canonical_digest must not be empty")
    return receipt


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
    """Return the code-owned integrity digest for a receipt envelope.

    The self-referential canonical digest and any explicitly rotatable identity
    fields are excluded.  The signed payload digest remains independent and is
    validated against the incoming inventory before normalization.
    """
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
    """Recreate the runtime's ordered, compact raw identity-v1 bytes.

    The v1 object is input metadata only.  It is never admitted directly; a
    v2 envelope must bind these exact bytes through ``signed_payload_sha256``.
    """
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
    """Ensure the v2 payload digest cannot be rebound after raw-v1 drift."""
    signed_payload = require_hex(
        identity_receipt.get("signed_payload_sha256"),
        f"{label}.signed_payload_sha256",
    )
    expected = hashlib.sha256(
        _canonical_raw_identity_receipt_v1_bytes(identity_receipt)
    ).hexdigest()
    if signed_payload != expected:
        die(f"{label} signed payload is not bound to the canonical raw v1 bytes")


def _validate_receipt_freshness(
    receipt: dict[str, Any],
    label: str,
    capture_window_id: str,
    *,
    capture_window_bounds: tuple[datetime, datetime] | None = None,
) -> None:
    """Require a current, plan-bound v2 receipt freshness tuple."""
    missing = {
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    } - set(receipt)
    if missing:
        die(f"{label} freshness fields are incomplete: {', '.join(sorted(missing))}")
    if receipt.get("capture_window_id") != capture_window_id:
        die(f"{label}.capture_window_id does not match the transaction capture window")
    if receipt.get("rotation_epoch") != CANONICAL_ROTATION_EPOCH:
        die(f"{label}.rotation_epoch is not the governed rotation epoch")
    issued_at = _parse_utc(receipt.get("issued_at"), f"{label}.issued_at")
    expires_at = _parse_utc(receipt.get("expires_at"), f"{label}.expires_at")
    now = datetime.now(timezone.utc)
    if expires_at <= issued_at or expires_at <= now:
        die(f"{label} freshness window is stale or inverted")
    if issued_at > now + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
        die(f"{label}.issued_at is in the future")
    if capture_window_bounds is not None:
        capture_start, capture_end = capture_window_bounds
        if issued_at < capture_start or expires_at > capture_end:
            die(f"{label} freshness window is outside the plan capture window")


def _capture_window_bounds_from_credential_ledger(
    raw_ledger: Any, capture_window_id: str
) -> tuple[datetime, datetime]:
    """Read the plan's immutable capture interval before receipt admission.

    The nonce-ledger validator performs the complete lease and replay checks
    later in ``build_plan``.  Receipt validation needs the two signed lease
    timestamps earlier, however, so this small preflight only establishes the
    interval and its transaction-window binding; it never normalizes or
    replaces the ledger.
    """
    ledger = require_object(raw_ledger, "credential_nonce_ledger")
    if ledger.get("capture_window_id") != capture_window_id:
        die("credential_nonce_ledger capture window binding mismatch")
    starts_at = _parse_utc(ledger.get("issued_at"), "credential_nonce_ledger.issued_at")
    ends_at = _parse_utc(ledger.get("expires_at"), "credential_nonce_ledger.expires_at")
    if ends_at <= starts_at:
        die("credential_nonce_ledger capture window is inverted")
    now = datetime.now(timezone.utc)
    if ends_at <= now:
        die("credential_nonce_ledger lease is expired")
    if starts_at > now + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
        die("credential_nonce_ledger.issued_at is in the future")
    return starts_at, ends_at


def _normalized_path(raw: Any, platform: str, label: str) -> str:
    value = require_string(raw, label)
    if platform == "windows":
        parsed = PureWindowsPath(value)
        if not parsed.is_absolute():
            die(f"{label} must be absolute")
        normalized = str(parsed).replace("\\", "/")
        if ".." in PurePosixPath(normalized).parts:
            die(f"{label} contains parent traversal")
        return normalized.rstrip("/")
    parsed = PurePosixPath(value)
    if not parsed.is_absolute() or ".." in parsed.parts:
        die(f"{label} must be an absolute, traversal-free path")
    return str(parsed).rstrip("/")


def _path_under(root: str, path: str, platform: str) -> bool:
    left = root.lower() if platform == "windows" else root
    right = path.lower() if platform == "windows" else path
    return right.startswith(left + "/")


def _expected_surfaces(name: str) -> tuple[str, ...]:
    if name in VALIDATOR_NAMES:
        return VALIDATOR_RESET_SURFACES
    node_id = EXPECTED_NODES[name]["node_id"]
    if name == "linux-lan-observer":
        return tuple(
            item.replace("{node_id}", node_id)
            for item in LINUX_OBSERVER_PERSISTENT_STATE_SURFACES
        )
    return tuple(item.replace("{node_id}", node_id) for item in OBSERVER_RESET_SURFACES)


def _canonical_state_surface_variants(name: str) -> tuple[tuple[str, ...], ...]:
    """Return the code-owned complete surface set(s) for a node role/layout."""
    canonical = _expected_surfaces(name)
    if name == "macos-observer":
        node_id = EXPECTED_NODES[name]["node_id"]
        stack_layout = tuple(
            item.replace("{node_id}", node_id)
            for item in LINUX_OBSERVER_PERSISTENT_STATE_SURFACES
        )
        # macOS has an authenticated legacy data layout and a governed stack
        # layout; both are complete inventories, while neither permits a
        # sparse or caller-invented surface list.
        return (canonical, stack_layout)
    return (canonical,)


def _validate_deployment_inventory(
    raw: Any,
    allowed_signers: set[str],
    capture_window_id: str,
    *,
    capture_window_bounds: tuple[datetime, datetime] | None = None,
) -> dict[str, Any]:
    if raw is None:
        die("deployment_inventory is required and must be independently authenticated")
    inventory = require_object(raw, "deployment_inventory")
    if set(inventory) != {
        "schema_version",
        "authenticated",
        "verified",
        "signer_id",
        "trust_root_id",
        "nodes",
        "receipt",
    }:
        die("deployment_inventory fields are not exact")
    if inventory.get("schema_version") != DEPLOYMENT_INVENTORY_SCHEMA:
        die("deployment_inventory schema is unsupported")
    if inventory.get("authenticated") is not True or inventory.get("verified") is not True:
        die("deployment_inventory must be authenticated and independently verified")
    signer_id = require_string(inventory.get("signer_id"), "deployment_inventory.signer_id")
    if signer_id not in allowed_signers:
        die("deployment_inventory signer is not in the governed signer allowlist")
    if inventory.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID:
        die("deployment_inventory trust root is not code-owned")
    receipt = validate_authenticated_receipt(
        inventory.get("receipt"), "deployment_inventory.receipt", allowed_signers
    )
    if receipt.get("schema_version") != "oasis7.deployment_inventory_receipt.v2":
        die("deployment_inventory receipt schema is unsupported")
    if set(receipt) != DEPLOYMENT_INVENTORY_RECEIPT_FIELDS:
        missing = DEPLOYMENT_INVENTORY_RECEIPT_FIELDS - set(receipt)
        extra = set(receipt) - DEPLOYMENT_INVENTORY_RECEIPT_FIELDS
        if missing:
            die(
                "deployment_inventory receipt freshness fields are incomplete: "
                + ", ".join(sorted(missing))
            )
        die(
            "deployment_inventory receipt fields are not exact: "
            + ", ".join(sorted(extra))
        )
    if (
        receipt.get("signer_id") != signer_id
        or receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
        or receipt.get("verifier_id") != CANONICAL_VERIFIER_ID
    ):
        die("deployment_inventory receipt signer, verifier, or trust root drifted")
    _validate_receipt_freshness(
        receipt,
        "deployment_inventory.receipt",
        capture_window_id,
        capture_window_bounds=capture_window_bounds,
    )
    # Authenticate the exact caller-supplied payload before any path or field
    # normalization. Normalization must never repair or rewrite an incoming
    # signed payload digest.
    incoming_payload_digest = _canonical_deployment_inventory_payload_digest(inventory)
    if receipt.get("signed_payload_sha256") != incoming_payload_digest:
        die("deployment_inventory receipt payload is not bound to the incoming canonical surface inventory")
    raw_nodes = require_object(inventory.get("nodes"), "deployment_inventory.nodes")
    if set(raw_nodes) != set(NODE_ORDER):
        die("deployment_inventory must cover the exact managed five-node set")
    normalized_nodes: dict[str, dict[str, Any]] = {}
    for name in NODE_ORDER:
        expected = EXPECTED_NODES[name]
        raw_node = require_object(raw_nodes.get(name), f"deployment_inventory.nodes.{name}")
        allowed_fields = {
            "node_id",
            "node_root",
            "persistent_state_paths",
            "expected_key_uid",
            "expected_key_gid",
            "peer_id",
        }
        required_fields = allowed_fields
        if not required_fields.issubset(raw_node) or set(raw_node) - allowed_fields:
            die(f"deployment_inventory.nodes.{name} fields are not complete")
        if raw_node["node_id"] != expected["node_id"]:
            die(f"deployment_inventory.nodes.{name}.node_id does not match the governed identity")
        peer_id = require_string(raw_node.get("peer_id"), f"deployment_inventory.nodes.{name}.peer_id")
        platform = expected["platform"]
        path_style = "windows" if platform == "windows-x64" else "posix"
        root = _normalized_path(
            raw_node.get("node_root"),
            path_style,
            f"deployment_inventory.nodes.{name}.node_root",
        )
        expected_uid = raw_node["expected_key_uid"]
        expected_gid = raw_node["expected_key_gid"]
        for field, value in (("expected_key_uid", expected_uid), ("expected_key_gid", expected_gid)):
            if isinstance(value, bool) or not isinstance(value, int) or value < 0:
                die(f"deployment_inventory.nodes.{name}.{field} must be a non-negative integer")
        raw_paths = raw_node.get("persistent_state_paths")
        if not isinstance(raw_paths, list) or not raw_paths:
            die(f"deployment_inventory.nodes.{name}.persistent_state_paths must be a non-empty list")
        paths = [
            _normalized_path(path, path_style, f"deployment_inventory.nodes.{name}.persistent_state_paths[{index}]")
            for index, path in enumerate(raw_paths)
        ]
        if len(set(paths)) != len(paths) or any(not _path_under(root, path, path_style) for path in paths):
            die(f"deployment_inventory.nodes.{name}.persistent_state_paths escape or duplicate node_root")
        expected_path_variants = [
            [
                _normalized_path(
                    f"{root}/{surface}",
                    path_style,
                    f"deployment_inventory.nodes.{name}.canonical_surface[{index}]",
                )
                for index, surface in enumerate(surface_set)
            ]
            for surface_set in _canonical_state_surface_variants(name)
        ]
        if paths not in expected_path_variants:
            die(f"deployment_inventory.nodes.{name}.persistent_state_paths must cover the exact canonical surfaces")
        normalized_nodes[name] = {
            "node_id": expected["node_id"],
            "peer_id": peer_id,
            "node_root": root,
            "persistent_state_paths": paths,
            "expected_key_uid": expected_uid,
            "expected_key_gid": expected_gid,
        }
    normalized_receipt = dict(receipt)
    normalized_receipt["canonical_digest"] = _canonical_receipt_digest(
        receipt, excluded_fields=frozenset({"signed_payload_sha256"})
    )
    normalized_inventory = {
        "schema_version": DEPLOYMENT_INVENTORY_SCHEMA,
        "authenticated": True,
        "verified": True,
        "signer_id": signer_id,
        "trust_root_id": CANONICAL_TRUST_ROOT_ID,
        "nodes": normalized_nodes,
        "receipt": normalized_receipt,
    }
    canonical_digest = _canonical_deployment_inventory_payload_digest(normalized_inventory)
    if receipt.get("signed_payload_sha256") != canonical_digest:
        die("deployment_inventory receipt payload is not bound to the canonical inventory")
    return normalized_inventory


def _validate_state_paths(
    node: dict[str, Any],
    expected: dict[str, str],
    governed: dict[str, Any] | None = None,
) -> list[str]:
    name = require_string(node.get("name"), "node.name")
    platform = expected["platform"]
    path_style = "windows" if platform == "windows-x64" else "posix"
    if governed is not None:
        root = _normalized_path(governed["node_root"], path_style, f"{name}.deployment_node_root")
        actual = [
            _normalized_path(path, path_style, f"{name}.deployment_state_path[{index}]")
            for index, path in enumerate(governed["persistent_state_paths"])
        ]
        if not actual or len(set(actual)) != len(actual):
            die(f"{name}.deployment persistent_state_paths contain duplicates or are empty")
        if any(not _path_under(root, path, path_style) for path in actual):
            die(f"{name}.deployment persistent_state_paths escape node_root")
        return actual
    root = _normalized_path(node.get("node_root"), path_style, f"{name}.node_root")
    raw_paths = node.get("persistent_state_paths")
    allowed_lengths = {8 if expected["role"] == "validator" else 7}
    if name == "linux-lan-observer":
        allowed_lengths.add(8)
    if not isinstance(raw_paths, list) or len(raw_paths) not in allowed_lengths:
        die(f"{name}.persistent_state_paths must enumerate the exact governed surface set")
    actual = [
        _normalized_path(path, path_style, f"{name}.persistent_state_paths[{index}]")
        for index, path in enumerate(raw_paths)
    ]
    normalized_root = root
    expected_paths = [
        _normalized_path(
            f"{normalized_root}/{surface}",
            "windows" if platform == "windows-x64" else "posix",
            f"{name}.expected_surface[{index}]",
        )
        for index, surface in enumerate(_expected_surfaces(name))
    ]
    if actual != expected_paths:
        die(f"{name}.persistent_state_paths do not match the exact governed surface order")
    if any(not _path_under(normalized_root, path, path_style) for path in actual):
        die(f"{name}.persistent_state_paths escape node_root")
    if len(set(actual)) != len(actual):
        die(f"{name}.persistent_state_paths contain duplicates")
    return actual


def _validate_authority(
    request: dict[str, Any], consumer_impact_record: dict[str, Any]
) -> dict[str, Any]:
    task_uid = require_string(request.get("task_uid"), "task_uid")
    if not SAFE_NAME_RE.fullmatch(task_uid.replace("_", "-")):
        die("task_uid is not a safe identifier")
    head_oid = require_oid(request.get("head_oid"), "head_oid")
    authority = require_object(request.get("authority"), "authority")
    if authority.get("authorized") is not True:
        die("authority is not explicitly authorized")
    if authority.get("task_uid") != task_uid or str(authority.get("head_oid", "")).lower() != head_oid:
        die("authority task/head binding mismatch")
    if str(authority.get("frozen_head_oid", "")).lower() != head_oid:
        die("authority frozen head binding mismatch")
    authority_impact = require_object(
        authority.get("consumer_impact_record"), "authority.consumer_impact_record"
    )
    if authority_impact != _impact_locator(consumer_impact_record):
        die("authority consumer-impact path/sha256 binding mismatch")
    raw_signers = authority.get("signer_allowlist")
    if not isinstance(raw_signers, list) or not raw_signers:
        die("authority signer allowlist is missing")
    signer_allowlist = {require_string(item, "authority.signer_allowlist[]") for item in raw_signers}
    if len(signer_allowlist) != len(raw_signers):
        die("authority signer allowlist contains duplicates")
    if signer_allowlist != CANONICAL_SIGNER_ALLOWLIST:
        die("authority signer allowlist does not match the code-owned trust root")
    receipt = validate_authenticated_receipt(
        authority.get("receipt"), "authority.receipt", signer_allowlist
    )
    if receipt.get("schema_version") != "oasis7.clean_room_authority.v1":
        die("authority receipt schema is unsupported")
    receipt_bindings = require_object(receipt.get("bindings"), "authority.receipt.bindings")
    if (
        receipt_bindings.get("task_uid") != task_uid
        or str(receipt_bindings.get("head_oid", "")).lower() != head_oid
        or receipt_bindings.get("signer_allowlist") != sorted(CANONICAL_SIGNER_ALLOWLIST)
        or receipt_bindings.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
        or receipt_bindings.get("verifier_id") != CANONICAL_VERIFIER_ID
        or receipt_bindings.get("consumer_impact_record")
        != _impact_locator(consumer_impact_record)
    ):
        die("authority receipt task/head/trust-root/consumer-impact binding mismatch")
    if (
        "frozen_head_oid" in receipt_bindings
        and str(receipt_bindings.get("frozen_head_oid", "")).lower() != head_oid
    ):
        die("authority receipt frozen-head binding mismatch")
    authority_context_fields = {
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    }
    present_context = authority_context_fields.intersection(receipt_bindings)
    if present_context:
        if present_context != authority_context_fields:
            die("authority receipt freshness binding is incomplete")
        if receipt_bindings.get("capture_window_id") != request.get("capture_window_id"):
            die("authority receipt capture-window binding mismatch")
        if receipt_bindings.get("rotation_epoch") != CANONICAL_ROTATION_EPOCH:
            die("authority receipt rotation epoch is not code-owned")
        issued_at = _parse_utc(receipt_bindings.get("issued_at"), "authority.receipt.bindings.issued_at")
        expires_at = _parse_utc(receipt_bindings.get("expires_at"), "authority.receipt.bindings.expires_at")
        now = datetime.now(timezone.utc)
        if expires_at <= issued_at or expires_at <= now:
            die("authority receipt is stale or has an inverted freshness window")
        if issued_at > now + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
            die("authority receipt is issued in the future")
    trust_root = require_object(authority.get("trust_root"), "authority.trust_root")
    validate_authenticated_receipt(trust_root, "authority.trust_root", signer_allowlist)
    if (
        trust_root.get("schema_version") != "oasis7.governed_trust_root_receipt.v1"
        or trust_root.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
        or trust_root.get("verifier_id") != CANONICAL_VERIFIER_ID
        or trust_root.get("signer_allowlist") != sorted(CANONICAL_SIGNER_ALLOWLIST)
    ):
        die("authority trust-root identity is not code-owned")
    trust_bindings = require_object(trust_root.get("bindings"), "authority.trust_root.bindings")
    if trust_bindings != receipt_bindings:
        die("authority trust-root and authority receipt bindings disagree")
    verifier = require_object(
        authority.get("crypto_verifier_receipt"), "authority.crypto_verifier_receipt"
    )
    validate_authenticated_receipt(
        verifier, "authority.crypto_verifier_receipt", signer_allowlist
    )
    if verifier.get("schema_version") != "oasis7.crypto_verifier_receipt.v1":
        die("crypto verifier receipt schema is unsupported")
    if verifier.get("verified") is not True or verifier.get("algorithm") != "ed25519":
        die("crypto verifier receipt is not a verified Ed25519 verifier")
    if verifier.get("scope") != "all-plan-receipts":
        die("crypto verifier receipt scope must cover all plan receipts")
    require_string(verifier.get("verifier_id"), "crypto_verifier_receipt.verifier_id")
    if verifier.get("verifier_id") != CANONICAL_VERIFIER_ID:
        die("crypto verifier identity is not code-owned")
    require_string(verifier.get("executable_path"), "crypto_verifier_receipt.executable_path")
    require_hex(verifier.get("executable_sha256"), "crypto_verifier_receipt.executable_sha256")
    return {
        "task_uid": task_uid,
        "head_oid": head_oid,
        "frozen_head_oid": head_oid,
        "consumer_impact_record": consumer_impact_record,
        "signer_allowlist": sorted(signer_allowlist),
        "trust_root": trust_root,
        "crypto_verifier_receipt": verifier,
        "receipt": receipt,
    }


def _positive_size(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        die(f"{label} must be a positive integer")
    return value


def _validate_execution_bindings(value: Any, label: str) -> dict[str, Any]:
    execution = require_object(value, label)
    records = require_object(execution.get("execution_records_root"), f"{label}.execution_records_root")
    require_string(records.get("path"), f"{label}.execution_records_root.path")
    require_hex(records.get("sha256"), f"{label}.execution_records_root.sha256")
    _positive_size(records.get("size_bytes"), f"{label}.execution_records_root.size_bytes")

    cas = require_object(execution.get("cas"), f"{label}.cas")
    require_string(cas.get("root"), f"{label}.cas.root")
    require_hex(cas.get("blake3"), f"{label}.cas.blake3")
    _positive_size(cas.get("size_bytes"), f"{label}.cas.size_bytes")

    world_head = require_object(execution.get("world_head"), f"{label}.world_head")
    require_string(world_head.get("path"), f"{label}.world_head.path")
    require_hex(world_head.get("sha256"), f"{label}.world_head.sha256")
    _positive_size(world_head.get("size_bytes"), f"{label}.world_head.size_bytes")
    world_head_height = world_head.get("height")
    if isinstance(world_head_height, bool) or not isinstance(world_head_height, int) or world_head_height <= 0:
        die(f"{label}.world_head.height must be a positive integer")
    require_hex(world_head.get("block_hash"), f"{label}.world_head.block_hash")
    require_hex(world_head.get("state_root"), f"{label}.world_head.state_root")

    sidecar = require_object(
        execution.get("generated_world_sidecar"), f"{label}.generated_world_sidecar"
    )
    require_string(sidecar.get("path"), f"{label}.generated_world_sidecar.path")
    require_hex(sidecar.get("sha256"), f"{label}.generated_world_sidecar.sha256")
    _positive_size(sidecar.get("size_bytes"), f"{label}.generated_world_sidecar.size_bytes")
    require_string(sidecar.get("provenance_path"), f"{label}.generated_world_sidecar.provenance_path")
    require_hex(
        sidecar.get("provenance_sha256"),
        f"{label}.generated_world_sidecar.provenance_sha256",
    )
    _positive_size(
        sidecar.get("provenance_size_bytes"),
        f"{label}.generated_world_sidecar.provenance_size_bytes",
    )

    consistency = require_object(
        execution.get("json_index_consistency"), f"{label}.json_index_consistency"
    )
    if consistency.get("verified") is not True:
        die(f"{label}.json_index_consistency must be independently verified")
    for field in ("snapshot_sha256", "journal_sha256", "index_sha256"):
        require_hex(consistency.get(field), f"{label}.json_index_consistency.{field}")
    for field in ("snapshot_size_bytes", "journal_size_bytes", "index_size_bytes"):
        _positive_size(consistency.get(field), f"{label}.json_index_consistency.{field}")
    return {
        "execution_records_root": {
            "path": records["path"],
            "sha256": records["sha256"].lower(),
            "size_bytes": records["size_bytes"],
        },
        "cas": {
            "root": cas["root"],
            "blake3": cas["blake3"].lower(),
            "size_bytes": cas["size_bytes"],
        },
        "world_head": {
            "path": world_head["path"],
            "sha256": world_head["sha256"].lower(),
            "size_bytes": world_head["size_bytes"],
            "height": world_head_height,
            "block_hash": world_head["block_hash"].lower(),
            "state_root": world_head["state_root"].lower(),
        },
        "generated_world_sidecar": {
            "path": sidecar["path"],
            "sha256": sidecar["sha256"].lower(),
            "size_bytes": sidecar["size_bytes"],
            "provenance_path": sidecar["provenance_path"],
            "provenance_sha256": sidecar["provenance_sha256"].lower(),
            "provenance_size_bytes": sidecar["provenance_size_bytes"],
        },
        "json_index_consistency": {
            "verified": True,
            "snapshot_sha256": consistency["snapshot_sha256"].lower(),
            "snapshot_size_bytes": consistency["snapshot_size_bytes"],
            "journal_sha256": consistency["journal_sha256"].lower(),
            "journal_size_bytes": consistency["journal_size_bytes"],
            "index_sha256": consistency["index_sha256"].lower(),
            "index_size_bytes": consistency["index_size_bytes"],
        },
    }


def _validate_verifier_bindings(
    verifier: dict[str, Any], execution: dict[str, Any]
) -> None:
    bindings = require_object(verifier.get("bindings"), "crypto_verifier_receipt.bindings")
    if bindings != execution:
        die("crypto verifier receipt execution/world/CAS/index binding mismatch")


def _validate_external_truth_binding(
    authority: dict[str, Any], truth: dict[str, Any]
) -> None:
    expected_network = truth["genesis"]["network_id"]
    authority_network = authority["receipt"]["bindings"].get("network_id")
    trust_network = authority["trust_root"]["bindings"].get("network_id")
    if authority_network != expected_network or trust_network != expected_network:
        die("external authority/trust-root network binding mismatch")


def _validate_truth(truth: Any, allowed_signers: set[str]) -> dict[str, Any]:
    value = require_object(truth, "truth")
    package = require_object(value.get("package"), "truth.package")
    for field in ("package_id", "package_dir", "provenance_path", "package_version"):
        require_string(package.get(field), f"truth.package.{field}")
    package_commit = require_oid(package.get("commit"), "truth.package.commit")
    require_hex(package.get("runtime_sha256"), "truth.package.runtime_sha256")
    _positive_size(package.get("runtime_size_bytes"), "truth.package.runtime_size_bytes")
    package_genesis_sha = require_hex(package.get("genesis_sha256"), "truth.package.genesis_sha256")
    package_world_sha = require_hex(package.get("world_sha256"), "truth.package.world_sha256")
    require_hex(package.get("provenance_sha256"), "truth.package.provenance_sha256")
    _positive_size(package.get("provenance_size_bytes"), "truth.package.provenance_size_bytes")
    validate_authenticated_receipt(package.get("receipt"), "truth.package.receipt", allowed_signers)
    platforms = require_object(package.get("platforms"), "truth.package.platforms")
    expected_platforms = {"linux-x64", "windows-x64", "macos-arm64"}
    if set(platforms) != expected_platforms:
        die("truth.package.platforms must cover the exact managed platform set")
    for platform, raw_platform in platforms.items():
        platform_value = require_object(raw_platform, f"truth.package.platforms.{platform}")
        require_oid(platform_value.get("commit"), f"truth.package.platforms.{platform}.commit")
        if platform_value["commit"].lower() != package_commit:
            die(f"truth.package.platforms.{platform}.commit binding mismatch")
        for field in (
            "package_sha256",
            "world_sha256",
            "world_provenance_sha256",
        ):
            require_hex(platform_value.get(field), f"truth.package.platforms.{platform}.{field}")
        for field in (
            "package_size_bytes",
            "world_size_bytes",
            "world_provenance_size_bytes",
        ):
            _positive_size(platform_value.get(field), f"truth.package.platforms.{platform}.{field}")

    genesis = require_object(value.get("genesis"), "truth.genesis")
    for field in ("network_id", "chain_id", "world_id", "path"):
        require_string(genesis.get(field), f"truth.genesis.{field}")
    if any(genesis.get(field) != CANONICAL_NETWORK_ID for field in ("network_id", "chain_id", "world_id")):
        die("truth genesis network identity is not the code-owned public-testnet network")
    _positive_size(genesis.get("size_bytes"), "truth.genesis.size_bytes")
    genesis_sha = require_hex(genesis.get("sha256"), "truth.genesis.sha256")
    if genesis_sha != package_genesis_sha:
        die("package/genesis digest binding mismatch")
    validate_authenticated_receipt(genesis.get("receipt"), "truth.genesis.receipt", allowed_signers)

    world = require_object(value.get("world"), "truth.world")
    for field in ("world_id", "generation", "path", "provenance_path"):
        require_string(world.get(field), f"truth.world.{field}")
    _positive_size(world.get("size_bytes"), "truth.world.size_bytes")
    world_sha = require_hex(world.get("sha256"), "truth.world.sha256")
    require_hex(world.get("provenance_sha256"), "truth.world.provenance_sha256")
    _positive_size(world.get("provenance_size_bytes"), "truth.world.provenance_size_bytes")
    if world_sha != package_world_sha or world.get("world_id") != genesis.get("world_id"):
        die("package/world/genesis binding mismatch")
    validate_authenticated_receipt(world.get("receipt"), "truth.world.receipt", allowed_signers)
    for platform, raw_platform in platforms.items():
        platform_value = require_object(raw_platform, f"truth.package.platforms.{platform}")
        if (
            platform_value["world_sha256"].lower() != world_sha
            or platform_value["world_provenance_sha256"].lower() != world["provenance_sha256"].lower()
        ):
            die(f"truth.package.platforms.{platform} world provenance binding mismatch")

    execution = _validate_execution_bindings(value.get("execution"), "truth.execution")
    checkpoint = require_object(value.get("checkpoint"), "truth.checkpoint")
    checkpoint_id = require_string(checkpoint.get("checkpoint_id"), "truth.checkpoint.checkpoint_id")
    manifest_hash = require_hex(checkpoint.get("manifest_hash"), "truth.checkpoint.manifest_hash")
    height = checkpoint.get("height")
    if isinstance(height, bool) or not isinstance(height, int) or height <= 0:
        die("truth.checkpoint.height must be a positive integer")
    require_string(checkpoint.get("receipt_path"), "truth.checkpoint.receipt_path")
    _positive_size(checkpoint.get("size_bytes"), "truth.checkpoint.size_bytes")
    require_hex(checkpoint.get("execution_block_hash"), "truth.checkpoint.execution_block_hash")
    require_hex(checkpoint.get("execution_state_root"), "truth.checkpoint.execution_state_root")
    require_hex(checkpoint.get("sha256"), "truth.checkpoint.sha256")
    validate_authenticated_receipt(checkpoint.get("receipt"), "truth.checkpoint.receipt", allowed_signers)
    world_head = execution["world_head"]
    if (
        world_head["height"] != height
        or world_head["block_hash"] != checkpoint["execution_block_hash"].lower()
        or world_head["state_root"] != checkpoint["execution_state_root"].lower()
    ):
        die("truth world-head/checkpoint execution binding mismatch")
    return {
        "package": {
            **package,
            "commit": package_commit,
            "genesis_sha256": package_genesis_sha,
            "world_sha256": package_world_sha,
        },
        "genesis": {**genesis, "sha256": genesis_sha},
        "world": {**world, "sha256": world_sha},
        "execution": execution,
        "checkpoint": {**checkpoint, "manifest_hash": manifest_hash, "height": height},
    }


def _validate_probe(
    probe: Any,
    truth: dict[str, Any],
    allowed_signers: set[str],
    context: dict[str, str],
) -> dict[str, Any]:
    value = require_object(probe, "fresh_root_probe")
    schema = require_string(value.get("schema_version"), "fresh_root_probe.schema_version")
    if schema != "oasis7.fresh_root_probe.v1":
        die("fresh_root_probe schema is unsupported")
    if value.get("authenticated") is not True or value.get("verified") is not True:
        die("fresh_root_probe must be authenticated and verified")
    if value.get("transaction_id") != context["transaction_id"]:
        die("fresh_root_probe transaction binding mismatch")
    if value.get("capture_window_id") != context["capture_window_id"]:
        die("fresh_root_probe capture-window binding mismatch")
    if value.get("replayed") is not False or value.get("post_validator_verify") is not True:
        die("fresh_root_probe must be a post-validator, non-replayed capture")
    if value.get("package_commit", "").lower() != truth["package"]["commit"]:
        die("fresh_root_probe package binding mismatch")
    checkpoint = truth["checkpoint"]
    if (
        value.get("checkpoint_id") != checkpoint["checkpoint_id"]
        or str(value.get("manifest_hash", "")).lower() != checkpoint["manifest_hash"]
        or value.get("height") != checkpoint["height"]
    ):
        die("fresh_root_probe checkpoint binding mismatch")
    outputs = require_object(
        value.get("validator_verify_outputs"), "fresh_root_probe.validator_verify_outputs"
    )
    if set(outputs) != set(VALIDATOR_NAMES):
        die("fresh_root_probe validator verify outputs must cover both validators")
    for name in sorted(VALIDATOR_NAMES):
        output = require_object(outputs.get(name), f"fresh_root_probe.validator_verify_outputs.{name}")
        validate_authenticated_receipt(
            output,
            f"fresh_root_probe.validator_verify_outputs.{name}",
            allowed_signers,
        )
        if output.get("schema_version") != "oasis7.validator_verify_output.v1":
            die(f"{name} validator verify output schema is unsupported")
        if output.get("node") != name:
            die(f"{name} validator verify output node binding mismatch")
        if output.get("transaction_id") != context["transaction_id"]:
            die(f"{name} validator verify output transaction binding mismatch")
        if output.get("capture_window_id") != context["capture_window_id"]:
            die(f"{name} validator verify output capture-window binding mismatch")
        if output.get("package_commit", "").lower() != truth["package"]["commit"]:
            die(f"{name} validator verify output package binding mismatch")
        if (
            output.get("checkpoint_id") != checkpoint["checkpoint_id"]
            or str(output.get("manifest_hash", "")).lower() != checkpoint["manifest_hash"]
            or output.get("height") != checkpoint["height"]
        ):
            die(f"{name} validator verify output checkpoint binding mismatch")
        require_hex(output.get("output_sha256"), f"{name}.validator_verify_output.output_sha256")
    validate_authenticated_receipt(value.get("receipt"), "fresh_root_probe.receipt", allowed_signers)
    return value


def _validate_host_and_endpoints(
    node: dict[str, Any], expected: dict[str, str]
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    name = str(node.get("name"))
    host = require_object(node.get("host_binding"), f"{name}.host_binding")
    canonical_host = CANONICAL_HOST_INVENTORY[name]
    target = require_string(host.get("target"), f"{name}.host_binding.target")
    if target != canonical_host["target"]:
        die(f"{name}.host_binding.target does not match the code-owned canonical inventory")
    if "/v1/chain/status" in target:
        die(f"{name}.host_binding.target must not contain a status endpoint")
    known_hosts_path = require_string(host.get("known_hosts_path"), f"{name}.host_binding.known_hosts_path")
    if known_hosts_path != canonical_host["known_hosts_path"]:
        die(f"{name}.host_binding.known_hosts_path does not match the code-owned canonical inventory")
    if not known_hosts_path.startswith("/") or ".." in PurePosixPath(known_hosts_path).parts:
        die(f"{name}.host_binding.known_hosts_path must be an absolute operator path")
    fingerprint = require_string(host.get("known_host_fingerprint"), f"{name}.host_binding.known_host_fingerprint")
    if fingerprint != canonical_host["known_host_fingerprint"]:
        die(f"{name}.host_binding.known_host_fingerprint does not match the code-owned canonical inventory")
    if re.fullmatch(r"SHA256:[A-Za-z0-9+/]{20,}", fingerprint) is None:
        die(f"{name}.host_binding.known_host_fingerprint is malformed")

    endpoints = require_object(node.get("endpoints"), f"{name}.endpoints")
    canonical_endpoints = CANONICAL_ENDPOINT_INVENTORY[name]
    healthz = require_string(endpoints.get("healthz"), f"{name}.endpoints.healthz")
    evidence = require_string(endpoints.get("evidence"), f"{name}.endpoints.evidence")
    if healthz != canonical_endpoints["healthz"] or evidence != canonical_endpoints["evidence"]:
        die(f"{name}.endpoints do not match the code-owned canonical endpoint inventory")
    if not healthz.startswith(("http://", "https://")) or not healthz.endswith("/healthz"):
        die(f"{name}.endpoints.healthz must be an HTTP /healthz endpoint")
    if not evidence.startswith(("http://", "https://")):
        die(f"{name}.endpoints.evidence must be an HTTP endpoint")
    if name == "sequencer-204":
        if not evidence.endswith("/v1/chain/rebuild-proof"):
            die("sequencer-204 evidence endpoint must be the bounded rebuild-proof endpoint")
    elif not evidence.endswith("/v1/chain/status"):
        die(f"{name} evidence endpoint must be /v1/chain/status")

    seam = require_object(node.get("credential_seam"), f"{name}.credential_seam")
    if seam.get("kind") != "temporary-fd-or-environment":
        die(f"{name}.credential_seam must use a temporary fd or environment")
    environment_name = require_string(seam.get("environment_name"), f"{name}.credential_seam.environment_name")
    if re.fullmatch(r"[A-Z][A-Z0-9_]{2,127}", environment_name) is None:
        die(f"{name}.credential_seam.environment_name is malformed")
    nonce = require_string(seam.get("nonce"), f"{name}.credential_seam.nonce")
    if re.fullmatch(r"[A-Za-z0-9._-]{32,}", nonce) is None:
        die(f"{name}.credential_seam.nonce must be an unpredictable bound nonce")
    issued_at = _parse_utc(seam.get("issued_at"), f"{name}.credential_seam.issued_at")
    expires_at = _parse_utc(seam.get("expires_at"), f"{name}.credential_seam.expires_at")
    if (
        issued_at > datetime.now(timezone.utc) + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS)
        or expires_at <= issued_at
        or expires_at <= datetime.now(timezone.utc)
    ):
        die(f"{name}.credential_seam nonce lease is expired or inverted")
    ledger_path = require_string(seam.get("ledger_path"), f"{name}.credential_seam.ledger_path")
    if not ledger_path.startswith("/") or ".." in PurePosixPath(ledger_path).parts:
        die(f"{name}.credential_seam.ledger_path must be an absolute operator path")
    if seam.get("one_shot") is not True:
        die(f"{name}.credential_seam must be one-shot")
    return (
        {"target": target, "known_hosts_path": known_hosts_path, "known_host_fingerprint": fingerprint},
        {
            "healthz": canonical_endpoints["healthz"],
            "evidence": canonical_endpoints["evidence"],
        },
        {
            "kind": seam["kind"],
            "environment_name": environment_name,
            "nonce": nonce,
            "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
            "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
            "ledger_path": ledger_path,
            "one_shot": True,
        },
    )


def _validate_nodes(
    nodes: Any,
    truth: dict[str, Any],
    allowed_signers: set[str],
    deployment_inventory: dict[str, Any],
    inventory_overrides_observed_layout: bool,
    capture_window_id: str,
    *,
    capture_window_bounds: tuple[datetime, datetime] | None = None,
) -> dict[str, dict[str, Any]]:
    if not isinstance(nodes, list) or len(nodes) != len(NODE_ORDER):
        die("nodes must contain exactly the five managed nodes")
    by_name: dict[str, dict[str, Any]] = {}
    seen_peer_ids: set[str] = set()
    for index, raw in enumerate(nodes):
        node = require_object(raw, f"nodes[{index}]")
        name = require_string(node.get("name"), f"nodes[{index}].name")
        if name not in EXPECTED_NODES or name in by_name:
            die(f"nodes contains an unexpected or duplicate node: {name}")
        expected = EXPECTED_NODES[name]
        governed = deployment_inventory["nodes"][name]
        for field in ("node_id", "role", "platform", "service_manager", "service"):
            if node.get(field) != expected[field]:
                die(f"{name}.{field} does not match the governed identity/service contract")
        identity_receipt = dict(
            require_object(node.get("identity_receipt"), f"{name}.identity_receipt")
        )
        if identity_receipt.get("schema_version") != IDENTITY_RECEIPT_SCHEMA:
            die(f"{name}.identity_receipt schema is unsupported")
        if set(identity_receipt) != IDENTITY_RECEIPT_FIELDS:
            missing = IDENTITY_RECEIPT_FIELDS - set(identity_receipt)
            extra = set(identity_receipt) - IDENTITY_RECEIPT_FIELDS
            if missing:
                die(
                    f"{name}.identity_receipt freshness fields are incomplete: "
                    + ", ".join(sorted(missing))
                )
            die(
                f"{name}.identity_receipt fields are not exact: "
                + ", ".join(sorted(extra))
            )
        if identity_receipt.get("node_id") != expected["node_id"]:
            die(f"{name}.identity_receipt node_id binding mismatch")
        validate_authenticated_receipt(identity_receipt, f"{name}.identity_receipt", allowed_signers)
        _validate_receipt_freshness(
            identity_receipt,
            f"{name}.identity_receipt",
            capture_window_id,
            capture_window_bounds=capture_window_bounds,
        )
        peer_id = require_string(identity_receipt.get("peer_id"), f"{name}.identity_receipt.peer_id")
        expected_peer_id = governed.get("peer_id", CANONICAL_PEER_REGISTRY[name])
        if peer_id != expected_peer_id:
            die(f"{name}.identity_receipt.peer_id does not match authenticated deployment inventory")
        if peer_id in seen_peer_ids:
            die(f"{name}.identity_receipt.peer_id duplicates another managed node")
        seen_peer_ids.add(peer_id)
        require_hex(identity_receipt.get("key_sha256"), f"{name}.identity_receipt.key_sha256")
        key_size = identity_receipt.get("key_size_bytes")
        if isinstance(key_size, bool) or not isinstance(key_size, int) or key_size <= 0:
            die(f"{name}.identity_receipt.key_size_bytes must be positive")
        if identity_receipt.get("key_mode") != "0600":
            die(f"{name}.identity_receipt.key_mode must be 0600")
        for owner in ("key_uid", "key_gid"):
            owner_value = identity_receipt.get(owner)
            if isinstance(owner_value, bool) or not isinstance(owner_value, int) or owner_value < 0:
                die(f"{name}.identity_receipt.{owner} must be a non-negative integer")
        _validate_identity_raw_v1_digest(identity_receipt, f"{name}.identity_receipt")
        expected_uid = governed["expected_key_uid"]
        expected_gid = governed["expected_key_gid"]
        if identity_receipt["key_uid"] != expected_uid or identity_receipt["key_gid"] != expected_gid:
            die(f"{name}.identity_receipt uid/gid do not match independently authenticated deployment inventory")
        # The deployment inventory is the independently authenticated source
        # for mutable identity fields (including service UID/GID). Recompute
        # the derived identity canonical digest after those fields are checked;
        # the inventory receipt's signed payload remains immutable and is
        # validated before normalization above.
        identity_receipt["canonical_digest"] = _canonical_receipt_digest(
            identity_receipt, excluded_fields=frozenset({"peer_id"})
        )
        path_style = "windows" if expected["platform"] == "windows-x64" else "posix"
        if inventory_overrides_observed_layout:
            expected_root = _normalized_path(
                governed["node_root"], path_style, f"{name}.deployment_node_root"
            )
            governed_paths = governed
        else:
            expected_root = _normalized_path(
                expected["node_root"], path_style, f"{name}.expected_node_root"
            )
            governed_paths = None
        host_binding, endpoints, credential_seam = _validate_host_and_endpoints(node, expected)
        normalized_paths = _validate_state_paths(node, expected, governed_paths)
        by_name[name] = {
            "name": name,
            "node_id": expected["node_id"],
            "role": expected["role"],
            "platform": expected["platform"],
            "node_root": expected_root,
            "service_manager": expected["service_manager"],
            "service": expected["service"],
            "persistent_state_paths": normalized_paths,
            "identity_receipt": identity_receipt,
            "host_binding": host_binding,
            "endpoints": endpoints,
            "credential_seam": credential_seam,
            "bindings": {
                "package_commit": truth["package"]["commit"],
                "package_platform": truth["package"]["platforms"][expected["platform"]],
                "genesis_sha256": truth["genesis"]["sha256"],
                "world_sha256": truth["world"]["sha256"],
                "checkpoint_id": truth["checkpoint"]["checkpoint_id"],
                "checkpoint_manifest_hash": truth["checkpoint"]["manifest_hash"],
                "checkpoint_height": truth["checkpoint"]["height"],
            },
        }
    if set(by_name) != set(NODE_ORDER):
        die("managed node set is incomplete")
    return by_name


def _validate_credential_nonce_ledger(
    raw_ledger: Any,
    nodes: dict[str, dict[str, Any]],
    context: dict[str, str],
    allowed_signers: set[str],
) -> dict[str, Any]:
    ledger = require_object(raw_ledger, "credential_nonce_ledger")
    if ledger.get("schema_version") != "oasis7.credential_nonce_ledger.v1":
        die("credential_nonce_ledger schema is unsupported")
    path = require_string(ledger.get("path"), "credential_nonce_ledger.path")
    if not path.startswith("/") or ".." in PurePosixPath(path).parts:
        die("credential_nonce_ledger.path must be an absolute operator path")
    if ledger.get("transaction_id") != context["transaction_id"]:
        die("credential_nonce_ledger transaction binding mismatch")
    if ledger.get("capture_window_id") != context["capture_window_id"]:
        die("credential_nonce_ledger capture-window binding mismatch")
    if ledger.get("one_shot") is not True or ledger.get("replay") is not False:
        die("credential_nonce_ledger must be a non-replayed one-shot ledger")
    issued_at = _parse_utc(ledger.get("issued_at"), "credential_nonce_ledger.issued_at")
    expires_at = _parse_utc(ledger.get("expires_at"), "credential_nonce_ledger.expires_at")
    if (
        issued_at > datetime.now(timezone.utc) + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS)
        or expires_at <= issued_at
        or expires_at <= datetime.now(timezone.utc)
    ):
        die("credential_nonce_ledger is expired or inverted")
    raw_reserved = ledger.get("reserved_nonces")
    if not isinstance(raw_reserved, list) or len(raw_reserved) != len(NODE_ORDER):
        die("credential_nonce_ledger must reserve one nonce per managed node")
    reserved = [require_string(item, "credential_nonce_ledger.reserved_nonces[]") for item in raw_reserved]
    if any(re.fullmatch(r"[A-Za-z0-9._-]{32,}", item) is None for item in reserved):
        die("credential_nonce_ledger contains a malformed nonce")
    if len(set(reserved)) != len(reserved):
        die("credential_nonce_ledger contains duplicate nonces")
    expected = [nodes[name]["credential_seam"]["nonce"] for name in NODE_ORDER]
    if reserved != expected:
        die("credential_nonce_ledger reservations do not match node nonce seams in order")
    for name in NODE_ORDER:
        seam = nodes[name]["credential_seam"]
        if seam["ledger_path"] != path or seam["one_shot"] is not True:
            die(f"{name}.credential_seam ledger binding mismatch")
        if seam["issued_at"] != issued_at.isoformat().replace("+00:00", "Z"):
            die(f"{name}.credential_seam issued-at binding mismatch")
        if seam["expires_at"] != expires_at.isoformat().replace("+00:00", "Z"):
            die(f"{name}.credential_seam expiry binding mismatch")
    validate_authenticated_receipt(
        ledger.get("receipt"), "credential_nonce_ledger.receipt", allowed_signers
    )
    receipt_bindings = require_object(
        ledger["receipt"].get("bindings"), "credential_nonce_ledger.receipt.bindings"
    )
    expected_receipt_bindings = {
        "path": path,
        "transaction_id": context["transaction_id"],
        "capture_window_id": context["capture_window_id"],
        "one_shot": True,
        "replay": False,
        "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
        "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
        "reserved_nonces": reserved,
    }
    if receipt_bindings != expected_receipt_bindings:
        die("credential_nonce_ledger receipt binding mismatch")
    return {
        "schema_version": ledger["schema_version"],
        "path": path,
        "transaction_id": context["transaction_id"],
        "capture_window_id": context["capture_window_id"],
        "one_shot": True,
        "replay": False,
        "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
        "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
        "reserved_nonces": reserved,
        "receipt": ledger["receipt"],
    }


def _validate_adapter_verification(
    raw_adapter: Any,
    context: dict[str, str],
    allowed_signers: set[str],
) -> dict[str, Any]:
    adapter = require_object(raw_adapter, "adapter_verification")
    validate_authenticated_receipt(adapter, "adapter_verification", allowed_signers)
    if adapter.get("schema_version") != "oasis7.clean_room_adapter_verification.v1":
        die("adapter_verification schema is unsupported")
    if adapter.get("adapter_id") != CANONICAL_ADAPTER_ID:
        die("adapter_verification adapter identity is not code-owned")
    if adapter.get("transaction_id") != context["transaction_id"]:
        die("adapter_verification transaction binding mismatch")
    if adapter.get("capture_window_id") != context["capture_window_id"]:
        die("adapter_verification capture-window binding mismatch")
    for field in (
        "live_receipts_verified",
        "credential_nonce_ledger_verified",
        "backup_or_no_backup_authority_verified",
    ):
        if adapter.get(field) is not True:
            die(f"adapter_verification.{field} must be true before any apply adapter")
    if adapter.get("apply_authority_granted") is not False:
        die("adapter_verification cannot grant apply authority")
    if adapter.get("durable_journal_authoritative") is not False:
        die("adapter_verification durable journal cannot be apply authority")
    if adapter.get("durable_journal_receipt_required") is not True:
        die("adapter_verification must require a durable adapter receipt")
    validate_authenticated_receipt(
        adapter.get("receipt"), "adapter_verification.receipt", allowed_signers
    )
    return {
        "schema_version": adapter["schema_version"],
        "authenticated": True,
        "verified": True,
        "adapter_id": CANONICAL_ADAPTER_ID,
        "transaction_id": context["transaction_id"],
        "capture_window_id": context["capture_window_id"],
        "live_receipts_verified": True,
        "credential_nonce_ledger_verified": True,
        "backup_or_no_backup_authority_verified": True,
        "apply_authority_granted": False,
        "durable_journal_authoritative": False,
        "durable_journal_receipt_required": True,
        "receipt": adapter["receipt"],
    }


def _validate_no_old_state_copy(request: dict[str, Any]) -> None:
    recovery = request.get("recovery")
    if recovery is None:
        return
    value = require_object(recovery, "recovery")
    if value.get("restore_old_state") is True or value.get("restore_state") is True:
        die("old-state restore is forbidden; rollback is clean-redeploy only")
    if (
        value.get("cross_node_state_copy") is True
        or value.get("copy_old_state") is True
        or value.get("source_node") is not None
        or value.get("copy_from_node") is not None
    ):
        die("cross-node state copy is forbidden")
    if value.get("seed_eligible") is True:
        die("forensic backup cannot be seed-eligible")


def _validate_backup_policy(
    request: dict[str, Any],
    authority: dict[str, Any],
    allowed_signers: set[str],
    context: dict[str, str],
    consumer_impact_record: dict[str, Any],
) -> dict[str, Any]:
    raw_policy = request.get("backup_policy")
    if raw_policy is None:
        return {
            "mode": "forensic-backup",
            "required_before_reset": True,
            "operator_authorized": False,
            "authority": None,
        }
    policy = require_object(raw_policy, "backup_policy")
    mode = require_string(policy.get("mode"), "backup_policy.mode")
    if mode == "forensic-backup":
        return {
            "mode": mode,
            "required_before_reset": True,
            "operator_authorized": False,
            "authority": None,
        }
    if mode != "operator-authorized-no-backup":
        die("backup_policy.mode is unsupported")
    if policy.get("operator_authorized") is not True:
        die("operator-authorized-no-backup requires explicit operator_authorized=true")
    if policy.get("current_authorization") is not True:
        die("operator-authorized-no-backup requires current_authorization=true")
    if policy.get("repository") != "eng-cc/oasis7":
        die("backup_policy.repository is not the governed repository")
    if policy.get("action") != "full-network-clean-room":
        die("backup_policy.action is not the governed clean-room action")
    targets = policy.get("targets")
    if targets != list(NODE_ORDER):
        die("backup_policy.targets must cover every managed node in deterministic order")
    if policy.get("transaction_id") != context["transaction_id"]:
        die("backup_policy transaction binding mismatch")
    if policy.get("capture_window_id") != context["capture_window_id"]:
        die("backup_policy capture-window binding mismatch")
    actor = require_string(policy.get("actor"), "backup_policy.actor")
    if SAFE_NAME_RE.fullmatch(actor) is None:
        die("backup_policy.actor is not a safe operator identifier")
    issued_at = _parse_utc(policy.get("issued_at"), "backup_policy.issued_at")
    expires_at = _parse_utc(policy.get("expires_at"), "backup_policy.expires_at")
    if (
        issued_at > datetime.now(timezone.utc) + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS)
        or expires_at <= issued_at
        or expires_at <= datetime.now(timezone.utc)
    ):
        die("backup_policy authorization is expired or inverted")
    if (
        policy.get("task_uid") != authority["task_uid"]
        or str(policy.get("frozen_head_oid", "")).lower() != authority["head_oid"]
    ):
        die("backup_policy authority task/frozen-head binding mismatch")
    require_string(policy.get("reason"), "backup_policy.reason")
    no_backup_receipt = validate_authenticated_receipt(
        policy.get("authority"), "backup_policy.authority", allowed_signers
    )
    if (
        no_backup_receipt.get("verifier_id") != CANONICAL_VERIFIER_ID
        or no_backup_receipt.get("trust_root_id") != CANONICAL_TRUST_ROOT_ID
    ):
        die("backup_policy authority verifier or trust-root identity mismatch")
    receipt_bindings = require_object(
        no_backup_receipt.get("bindings"), "backup_policy.authority.bindings"
    )
    expected_bindings = {
        "repository": "eng-cc/oasis7",
        "action": "full-network-clean-room",
        "targets": list(NODE_ORDER),
        "task_uid": authority["task_uid"],
        "transaction_id": context["transaction_id"],
        "capture_window_id": context["capture_window_id"],
        "frozen_head_oid": authority["head_oid"],
        "actor": actor,
        "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
        "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
        "current_authorization": True,
        "consumer_impact_record": _impact_locator(consumer_impact_record),
    }
    if receipt_bindings != expected_bindings:
        die("backup_policy authority receipt binding mismatch")
    return {
        "mode": mode,
        "required_before_reset": False,
        "operator_authorized": True,
        "current_authorization": True,
        "authority": no_backup_receipt,
        "repository": "eng-cc/oasis7",
        "action": "full-network-clean-room",
        "targets": list(NODE_ORDER),
        "transaction_id": context["transaction_id"],
        "capture_window_id": context["capture_window_id"],
        "actor": actor,
        "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
        "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
    }


def _global_order(backup_required: bool) -> list[str]:
    order: list[str] = []
    phases = ["preflight"]
    if backup_required:
        phases.append("forensic-backup")
    for phase in phases:
        order.extend(f"{phase}:{name}" for name in NODE_ORDER)
    for phase in ("stop", "delete", "rebuild"):
        order.extend(f"{phase}:{name}" for name in ("storage-205", "sequencer-204"))
    order.extend(("start:sequencer-204", "verify:sequencer-204", "start:storage-205", "verify:storage-205"))
    order.append("fresh-root-probe")
    for name in ("linux-lan-observer", "windows-observer", "macos-observer"):
        for phase in ("stop", "delete", "rebuild"):
            order.append(f"{phase}:{name}")
        order.extend((f"start:{name}", f"verify:{name}"))
    order.append("fleet-health")
    return order


def _validate_global_order(global_order: list[str]) -> None:
    if len(set(global_order)) != len(global_order):
        die("global operation order contains duplicate entries")
    positions = {entry: index for index, entry in enumerate(global_order)}
    required = {
        "fresh-root-probe",
        "start:sequencer-204",
        "verify:sequencer-204",
        "start:storage-205",
        "verify:storage-205",
    }
    required.update(
        f"{phase}:{name}"
        for phase in ("stop", "delete", "rebuild", "start", "verify")
        for name in ("linux-lan-observer", "windows-observer", "macos-observer")
    )
    if not required.issubset(positions):
        die("global operation order is incomplete")
    probe_index = positions["fresh-root-probe"]
    for name in ("linux-lan-observer", "windows-observer", "macos-observer"):
        for phase in ("stop", "delete", "rebuild"):
            if positions[f"{phase}:{name}"] <= probe_index:
                die("observer destructive phases must follow the fresh-root probe")
        if not (
            positions[f"stop:{name}"]
            < positions[f"delete:{name}"]
            < positions[f"rebuild:{name}"]
            < positions[f"start:{name}"]
            < positions[f"verify:{name}"]
        ):
            die(f"{name} operation order is not stop/delete/rebuild/start/verify")
    if not (
        positions["verify:sequencer-204"] < probe_index
        and positions["verify:storage-205"] < probe_index
    ):
        die("fresh-root probe must follow both validator verify outputs")


def _operation_journal(
    global_order: list[str],
    nodes: dict[str, dict[str, Any]],
    truth: dict[str, Any],
    context: dict[str, str],
    consumer_impact_record: dict[str, Any],
) -> list[dict[str, Any]]:
    journal: list[dict[str, Any]] = []
    for sequence, entry in enumerate(global_order, 1):
        phase, _, node = entry.partition(":")
        record: dict[str, Any] = {
            "sequence": sequence,
            "phase": phase,
            "node": node or None,
            "operation": entry,
            "transaction_id": context["transaction_id"],
            "capture_window_id": context["capture_window_id"],
            "consumer_impact_record": _impact_locator(consumer_impact_record),
            "package_commit": truth["package"]["commit"],
            "checkpoint_id": truth["checkpoint"]["checkpoint_id"],
            "checkpoint_manifest_hash": truth["checkpoint"]["manifest_hash"],
            "checkpoint_height": truth["checkpoint"]["height"],
        }
        if node:
            node_value = nodes[node]
            record.update(
                {
                    "target_root": node_value["node_root"],
                    "host_target": node_value["host_binding"]["target"],
                    "known_hosts_path": node_value["host_binding"]["known_hosts_path"],
                    "known_host_fingerprint": node_value["host_binding"]["known_host_fingerprint"],
                    "surface_set_sha256": hashlib.sha256(
                        json.dumps(
                            node_value["persistent_state_paths"],
                            ensure_ascii=True,
                            separators=(",", ":"),
                        ).encode()
                    ).hexdigest(),
                }
            )
        journal.append(record)
    return journal


def build_plan(request: dict[str, Any]) -> dict[str, Any]:
    request = require_object(request, "clean-room input")
    reject_secret_fields(request)
    if request.get("schema_version") != INPUT_SCHEMA:
        die("input schema is unsupported")
    context = _validate_transaction_context(request)
    consumer_impact_record = _validate_consumer_impact_record(
        request.get("consumer_impact_record")
    )
    authority = _validate_authority(request, consumer_impact_record)
    _validate_no_old_state_copy(request)
    allowed_signers = set(authority["signer_allowlist"])
    truth = _validate_truth(request.get("truth"), allowed_signers)
    _validate_external_truth_binding(authority, truth)
    _validate_verifier_bindings(authority["crypto_verifier_receipt"], truth["execution"])
    probe = _validate_probe(request.get("fresh_root_probe"), truth, allowed_signers, context)
    capture_window_bounds = _capture_window_bounds_from_credential_ledger(
        request.get("credential_nonce_ledger"), context["capture_window_id"]
    )
    has_explicit_inventory = request.get("deployment_inventory") is not None
    deployment_inventory = _validate_deployment_inventory(
        request.get("deployment_inventory"),
        allowed_signers,
        context["capture_window_id"],
        capture_window_bounds=capture_window_bounds,
    )
    nodes = _validate_nodes(
        request.get("nodes"),
        truth,
        allowed_signers,
        deployment_inventory,
        has_explicit_inventory,
        context["capture_window_id"],
        capture_window_bounds=capture_window_bounds,
    )
    credential_nonce_ledger = _validate_credential_nonce_ledger(
        request.get("credential_nonce_ledger"), nodes, context, allowed_signers
    )
    adapter_verification = _validate_adapter_verification(
        request.get("adapter_verification"), context, allowed_signers
    )
    backup_policy = _validate_backup_policy(
        request, authority, allowed_signers, context, consumer_impact_record
    )
    global_order = _global_order(backup_policy["required_before_reset"])
    _validate_global_order(global_order)
    plan: dict[str, Any] = {
        "schema_version": PLAN_SCHEMA,
        "task_uid": authority["task_uid"],
        "head_oid": authority["head_oid"],
        "transaction_id": context["transaction_id"],
        "capture_window_id": context["capture_window_id"],
        # The one-shot credential lease is the externally captured transaction
        # window; every provider receipt must land inside these exact bounds.
        "capture_window": {
            "id": context["capture_window_id"],
            "starts_at": credential_nonce_ledger["issued_at"],
            "ends_at": credential_nonce_ledger["expires_at"],
        },
        "authority": authority,
        "consumer_impact_record": consumer_impact_record,
        "execution": {
            "mode": "plan-only",
            "provider_mutation_performed": False,
            "provider_mutation_boundary": "external-governed-adapter-required",
            "plan_is_apply_proof": False,
            "apply_requires_fresh_adapter_receipt": True,
        },
        "node_order": list(NODE_ORDER),
        "global_order": global_order,
        "operation_journal": _operation_journal(
            global_order, nodes, truth, context, consumer_impact_record
        ),
        "operation_journal_contract": {
            "authoritative": False,
            "apply_usable": False,
            "adapter_owned": True,
            "durable_receipt_required": True,
            "planner_output_is_not_apply_proof": True,
        },
        "canonical_host_inventory": CANONICAL_HOST_INVENTORY,
        "canonical_endpoint_inventory": CANONICAL_ENDPOINT_INVENTORY,
        "surfaces": {
            "validators": list(VALIDATOR_RESET_SURFACES),
            "observers": list(OBSERVER_RESET_SURFACES),
            "validator_count": 8,
            "observer_count": 8,
            "observers_by_node": {
                name: nodes[name]["persistent_state_paths"] for name in OBSERVER_NAMES
            },
        },
        "deployment_inventory": deployment_inventory,
        "truth": {
            "package": truth["package"],
            "genesis": truth["genesis"],
            "world": truth["world"],
            "execution": truth["execution"],
            "checkpoint": truth["checkpoint"],
        },
        "fresh_root_probe": probe,
        "credential_nonce_ledger": credential_nonce_ledger,
        "adapter_verification": adapter_verification,
        "nodes": [nodes[name] for name in NODE_ORDER],
        "forensic_backup": {
            "mode": backup_policy["mode"],
            "task_uid": authority["task_uid"],
            "frozen_head_oid": authority["head_oid"],
            "required_before_reset": backup_policy["required_before_reset"],
            "operator_authorized": backup_policy["operator_authorized"],
            "current_authorization": backup_policy.get("current_authorization", False),
            "immutable": backup_policy["required_before_reset"],
            "seed_eligible": False,
            "cross_node_state_copy": False,
            "restore_old_state": False,
            "receipt_required_per_node": backup_policy["required_before_reset"],
            "authority": backup_policy["authority"],
            "repository": backup_policy.get("repository"),
            "action": backup_policy.get("action"),
            "targets": backup_policy.get("targets"),
            "transaction_id": backup_policy.get("transaction_id", context["transaction_id"]),
            "capture_window_id": backup_policy.get("capture_window_id", context["capture_window_id"]),
            "actor": backup_policy.get("actor"),
            "issued_at": backup_policy.get("issued_at"),
            "expires_at": backup_policy.get("expires_at"),
        },
        "observer_gate": {
            "required_before": ["windows-observer", "macos-observer"],
            "fresh_root_probe_required": True,
            "checkpoint_receipt_required": True,
            "fail_closed": True,
        },
        "rollback": {
            "policy": "clean-redeploy",
            "steps": [
                "stop-started-nodes",
                "preserve-failed-state-for-forensics",
                "reinstall-exact-package-and-truth",
                "rerun-fresh-root-probe",
            ],
            "stop_started_nodes": True,
            "preserve_failed_state_for_forensics": True,
            "restore_old_state": False,
            "cross_node_state_copy": False,
            "reinstall_exact_package_and_truth": True,
            "rerun_fresh_root_probe": True,
            "provider_mutation_requires_external_authority": True,
        },
    }
    material = json.dumps(plan, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
    plan["plan_digest"] = hashlib.sha256(material).hexdigest()
    return plan


def load_json(path: Path) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file():
        die(f"input must be a regular file: {path}")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        die(f"cannot read input JSON: {error}")
    return require_object(value, "clean-room input")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Generate a deterministic, plan-only full-network clean-room transaction. "
            "This command never mutates providers or copies old state."
        )
    )
    parser.add_argument("--input", required=True, type=Path, help="authenticated five-node truth envelope")
    parser.add_argument("--out", type=Path, help="optional plan output path")
    parser.add_argument("--json", action="store_true", help="print the complete machine-readable plan")
    args = parser.parse_args(argv)
    plan = build_plan(load_json(args.input).copy())
    if args.out is not None:
        output = args.out.expanduser()
        if output.is_symlink():
            die(f"plan output must not be a symlink: {output}")
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(plan, ensure_ascii=True, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.json:
        print(json.dumps(plan, ensure_ascii=True, indent=2, sort_keys=True))
    else:
        print(f"plan_digest={plan['plan_digest']}")
        print(f"plan_mode={plan['execution']['mode']}")
        print(f"plan_output={args.out if args.out is not None else 'stdout-summary'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
