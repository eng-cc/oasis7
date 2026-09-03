#!/usr/bin/env python3
"""Prepare, externally sign, assemble, and independently verify identity v2.

This executable is intentionally a small file-boundary tool.  It never accepts
private-key material or an arbitrary provider command/endpoint.  ``prepare``
freezes the exact canonical bytes, ``sign`` resolves a non-secret provider ID
from the pinned registry, ``assemble`` checks the detached signature before
emitting an envelope, and ``verify`` independently checks every binding before
writing an admission receipt.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, NoReturn


PREFIX = b"OASIS7-IDENTITY-RECEIPT-V2\0"
RAW_SCHEMA = "oasis7.identity_receipt.v1"
PAYLOAD_SCHEMA = "oasis7.identity_receipt.v2"
TRUST_SCHEMA = "oasis7.identity_v2_trust_config.v1"
REGISTRY_SCHEMA = "oasis7.identity_v2_provider_registry.v1"
CONTEXT_SCHEMA = "oasis7.identity_v2_context.v1"
INTENT_SCHEMA = "oasis7.clean_room_plan_intent.v1"
ALGORITHM = "ed25519"
DOMAIN = "oasis7.identity_receipt.v2/signature/v1"
DEPLOYED_TRUST_CONFIG = Path("/operator/truth/identity-v2-trust-config.json")
DEPLOYED_PROVIDER_REGISTRY = Path("/operator/truth/identity-v2-provider-registry.json")
DEPLOYED_GOVERNANCE_ROOT = Path("/operator/truth/governance-root.json")
# Identity-v2 is not provisioned in this repository.  Deployment authority
# must install these independent pins before any admission-capable command can
# run; caller/registry-provided digests are consistency checks only.
PINNED_TRUST_CONFIG_SHA256: str | None = None
PINNED_PROVIDER_REGISTRY_SHA256: str | None = None
ADAPTER_MODULE_NAME = "oasis7_identity_v2_adapter"
ADAPTER_PATH = Path(__file__).with_name("p2p-public-testnet-full-network-clean-room-adapter.py")
PAYLOAD_FIELDS = frozenset(
    {
        "domain_separator",
        "schema_version",
        "signer_id",
        "verifier_id",
        "trust_root_id",
        "task_uid",
        "head_oid",
        "frozen_head_oid",
        "plan_digest",
        "context_digest",
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
        "node_id",
        "peer_id",
        "key_sha256",
        "key_size_bytes",
        "key_mode",
        "key_uid",
        "key_gid",
        "signed_payload_sha256",
    }
)
RAW_FIELDS = frozenset(
    {
        "schema_version",
        "node_id",
        "peer_id",
        "key_path",
        "key_sha256",
        "key_size_bytes",
        "key_mode",
        "key_uid",
        "key_gid",
    }
)
CONTEXT_FIELDS = frozenset(
    {
        "schema_version",
        "network_id",
        "task_uid",
        "head_oid",
        "capture_window_id",
        "capture_start",
        "capture_end",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    }
)
INTENT_FIELDS = frozenset({"schema_version", "context_digest", "adapter_action", "nodes"})
NODE_FIELDS = frozenset({"node_name", "node_id", "peer_id", "role", "reset_surface_ids"})
ATTESTATION_FIELDS = frozenset(
    {
        "schema_version",
        "provider_id",
        "request_id",
        "signer_id",
        "public_key_sha256",
        "algorithm",
        "canonical_payload_sha256",
        "signature_sha256",
        "context_digest",
        "rotation_epoch",
        "capture_window_id",
        "issued_at",
        "detached_provider_authentication_proof",
    }
)
HEX64 = re.compile(r"^[0-9a-f]{64}$")
OID = re.compile(r"^[0-9a-f]{40}$")
SAFE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$")
UTC = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")


class ToolError(Exception):
    pass


def fail(message: str) -> NoReturn:
    raise ToolError(message)


def _reject_constant(value: str) -> NoReturn:
    fail(f"non-finite JSON value {value} is forbidden")


def _object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def canonical(value: Any) -> bytes:
    try:
        return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":"), allow_nan=False).encode(
            "utf-8"
        )
    except (TypeError, ValueError) as error:
        fail(f"cannot canonicalize JSON: {error.__class__.__name__}")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _regular(path_value: str | Path, label: str) -> Path:
    path = Path(path_value)
    if path.is_symlink() or not path.is_file():
        fail(f"{label} must be a regular non-symlink file")
    return path


def _regular_ancestor(path: Path, label: str) -> None:
    # Inspect the path as supplied.  Resolving first would hide a symlinked
    # ancestor and turn an operator path-control failure into a false pass.
    current = path.parent
    while True:
        # macOS exposes the system temporary tree through the conventional
        # ``/var`` (and occasionally ``/tmp``) alias.  These fixed aliases are
        # safe OS paths; operator/deployment ancestors remain symlink-intolerant.
        if current.is_symlink() and current not in {Path("/var"), Path("/tmp")}:
            fail(f"{label} has an invalid ancestor")
        if not current.is_dir():
            fail(f"{label} has an invalid ancestor")
        if current.parent == current:
            break
        current = current.parent


def read_bytes(path_value: str | Path, label: str) -> bytes:
    path = _regular(path_value, label)
    _regular_ancestor(path, label)
    try:
        return path.read_bytes()
    except OSError as error:
        fail(f"cannot read {label}: {error.__class__.__name__}")


def read_json(path_value: str | Path, label: str, *, require_canonical: bool = True) -> dict[str, Any]:
    raw = read_bytes(path_value, label)
    if raw.startswith(b"\xef\xbb\xbf"):
        fail(f"{label} must not contain a BOM")
    try:
        text = raw.decode("utf-8")
        value = json.loads(
            text,
            object_pairs_hook=_object_pairs,
            parse_constant=_reject_constant,
        )
    except ToolError:
        raise
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"{label} is not valid UTF-8 JSON: {error.__class__.__name__}")
    if not isinstance(value, dict):
        fail(f"{label} must contain a JSON object")
    if require_canonical and canonical(value) != raw:
        fail(f"{label} is not canonical JSON")
    return value


def require_exact_fields(value: dict[str, Any], fields: frozenset[str], label: str) -> None:
    if set(value) != fields:
        missing = sorted(fields - set(value))
        extra = sorted(set(value) - fields)
        fail(f"{label} fields are not exact (missing={missing}, extra={extra})")


def require_string(value: Any, label: str, *, safe: bool = False) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{label} must be a non-empty string")
    if safe and SAFE_ID.fullmatch(value) is None:
        fail(f"{label} is not a safe identifier")
    return value


def require_hex(value: Any, label: str, pattern: re.Pattern[str] = HEX64) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        fail(f"{label} has invalid lowercase hexadecimal shape")
    return value


def require_integer(value: Any, label: str, *, positive: bool = False) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0 or (positive and value == 0):
        fail(f"{label} must be a {'positive' if positive else 'non-negative'} integer")
    return value


def parse_timestamp(value: Any, label: str) -> datetime:
    text = require_string(value, label)
    if UTC.fullmatch(text) is None:
        fail(f"{label} must be UTC RFC3339 seconds")
    try:
        return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        fail(f"{label} is not a valid UTC timestamp")


def validate_raw(raw_bytes: bytes) -> dict[str, Any]:
    try:
        value = json.loads(
            raw_bytes.decode("utf-8"), object_pairs_hook=_object_pairs, parse_constant=_reject_constant
        )
    except ToolError:
        raise
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"raw-v1 is not valid UTF-8 JSON: {error.__class__.__name__}")
    if not isinstance(value, dict):
        fail("raw-v1 must contain a JSON object")
    require_exact_fields(value, RAW_FIELDS, "raw-v1")
    if value.get("schema_version") != RAW_SCHEMA:
        fail("raw-v1 schema is unsupported")
    for field in ("node_id", "peer_id", "key_path"):
        require_string(value.get(field), f"raw-v1.{field}")
    require_hex(value.get("key_sha256"), "raw-v1.key_sha256")
    require_integer(value.get("key_size_bytes"), "raw-v1.key_size_bytes", positive=True)
    require_integer(value.get("key_mode"), "raw-v1.key_mode")
    require_integer(value.get("key_uid"), "raw-v1.key_uid")
    require_integer(value.get("key_gid"), "raw-v1.key_gid")
    if value["key_mode"] != 0o600:
        fail("raw-v1.key_mode must be 0600")
    return value


def validate_trust(trust: dict[str, Any], label: str = "trust-config") -> dict[str, Any]:
    require_exact_fields(
        trust,
        frozenset({"schema_version", "network_id", "trust_root_id", "verifier_id", "algorithm", "rotation_epoch", "allowlist", "revocations"}),
        label,
    )
    if trust["schema_version"] != TRUST_SCHEMA or trust["algorithm"] != ALGORITHM:
        fail(f"{label} schema or algorithm is unsupported")
    for field in ("network_id", "trust_root_id", "verifier_id", "rotation_epoch"):
        require_string(trust.get(field), f"{label}.{field}", safe=(field == "rotation_epoch"))
    allowlist = trust["allowlist"]
    if not isinstance(allowlist, list) or not allowlist:
        fail(f"{label}.allowlist must be non-empty")
    seen: set[str] = set()
    for index, entry in enumerate(allowlist):
        item_label = f"{label}.allowlist[{index}]"
        if not isinstance(entry, dict):
            fail(f"{item_label} must be an object")
        require_exact_fields(
            entry,
            frozenset({"signer_id", "public_key_ref", "public_key_sha256", "status", "valid_from", "valid_until"}),
            item_label,
        )
        signer = require_string(entry.get("signer_id"), f"{item_label}.signer_id", safe=True)
        if signer in seen:
            fail(f"{label} contains duplicate signer_id")
        seen.add(signer)
        require_string(entry.get("public_key_ref"), f"{item_label}.public_key_ref")
        require_hex(entry.get("public_key_sha256"), f"{item_label}.public_key_sha256")
        if entry.get("status") not in {"active", "retired", "revoked"}:
            fail(f"{item_label}.status is unsupported")
        start = parse_timestamp(entry.get("valid_from"), f"{item_label}.valid_from")
        end = parse_timestamp(entry.get("valid_until"), f"{item_label}.valid_until")
        if end <= start:
            fail(f"{item_label} validity interval is inverted")
    revocations = trust["revocations"]
    if not isinstance(revocations, list):
        fail(f"{label}.revocations must be an array")
    revocation_seen: set[str] = set()
    for index, item in enumerate(revocations):
        item_label = f"{label}.revocations[{index}]"
        if not isinstance(item, dict):
            fail(f"{item_label} must be an object")
        require_exact_fields(item, frozenset({"signer_id", "effective_at", "reason"}), item_label)
        signer = require_string(item.get("signer_id"), f"{item_label}.signer_id", safe=True)
        if signer in revocation_seen:
            fail(f"{label} contains duplicate revocation")
        revocation_seen.add(signer)
        parse_timestamp(item.get("effective_at"), f"{item_label}.effective_at")
        require_string(item.get("reason"), f"{item_label}.reason")
    return trust


def resolved_equal(first: str | Path, second: str | Path) -> bool:
    try:
        return Path(first).resolve() == Path(second).resolve()
    except OSError:
        return False


def validate_registry(
    registry: dict[str, Any], registry_path: Path, trust_path: Path, *, label: str = "provider-registry"
) -> tuple[dict[str, Any], dict[str, Any]]:
    require_exact_fields(registry, frozenset({"schema_version", "trust_config_path", "trust_config_sha256", "providers", "verifier"}), label)
    if registry["schema_version"] != REGISTRY_SCHEMA:
        fail(f"{label} schema is unsupported")
    if not resolved_equal(registry["trust_config_path"], trust_path):
        fail(f"{label} trust-config path is not the pinned path")
    trust_bytes = read_bytes(trust_path, "trust-config")
    require_hex(registry.get("trust_config_sha256"), f"{label}.trust_config_sha256")
    if registry["trust_config_sha256"] != sha256_bytes(trust_bytes):
        fail(f"{label} trust-config digest mismatch")
    trust = validate_trust(read_json(trust_path, "trust-config"))
    providers = registry["providers"]
    if not isinstance(providers, list) or not providers:
        fail(f"{label}.providers must be a non-empty array")
    ids: set[str] = set()
    for index, provider in enumerate(providers):
        item_label = f"{label}.providers[{index}]"
        if not isinstance(provider, dict):
            fail(f"{item_label} must be an object")
        require_exact_fields(
            provider,
            frozenset({"provider_id", "adapter_path", "adapter_sha256", "signer_id", "public_key_ref", "public_key_sha256", "algorithm"}),
            item_label,
        )
        provider_id = require_string(provider.get("provider_id"), f"{item_label}.provider_id", safe=True)
        if provider_id in ids:
            fail(f"{label} contains duplicate provider_id")
        ids.add(provider_id)
        require_string(provider.get("adapter_path"), f"{item_label}.adapter_path")
        adapter = _regular(provider["adapter_path"], f"{item_label}.adapter_path")
        if not os.access(adapter, os.X_OK):
            fail(f"{item_label}.adapter_path is not executable")
        require_hex(provider.get("adapter_sha256"), f"{item_label}.adapter_sha256")
        if provider["adapter_sha256"] != sha256_bytes(read_bytes(adapter, f"{item_label}.adapter_path")):
            fail(f"{item_label}.adapter_sha256 mismatch")
        if provider.get("algorithm") != ALGORITHM:
            fail(f"{item_label}.algorithm is unsupported")
        require_string(provider.get("signer_id"), f"{item_label}.signer_id", safe=True)
        require_string(provider.get("public_key_ref"), f"{item_label}.public_key_ref")
        public_key = _regular(provider["public_key_ref"], f"{item_label}.public_key_ref")
        public_bytes = read_bytes(public_key, f"{item_label}.public_key_ref")
        if len(public_bytes) != 32:
            fail(f"{item_label}.public_key_ref must contain a raw 32-byte Ed25519 key")
        require_hex(provider.get("public_key_sha256"), f"{item_label}.public_key_sha256")
        if provider["public_key_sha256"] != sha256_bytes(public_bytes):
            fail(f"{item_label}.public_key_sha256 mismatch")
        trusted = [item for item in trust["allowlist"] if item["signer_id"] == provider["signer_id"]]
        if len(trusted) != 1 or not resolved_equal(trusted[0]["public_key_ref"], public_key):
            fail(f"{item_label} is not bound to the trust allowlist")
        if trusted[0]["public_key_sha256"] != provider["public_key_sha256"]:
            fail(f"{item_label} public key is not bound to the trust allowlist")
    verifier = registry["verifier"]
    if not isinstance(verifier, dict):
        fail(f"{label}.verifier must be an object")
    require_exact_fields(verifier, frozenset({"executable_path", "executable_sha256"}), f"{label}.verifier")
    require_string(verifier.get("executable_path"), f"{label}.verifier.executable_path")
    verifier_path = _regular(verifier["executable_path"], f"{label}.verifier.executable_path")
    require_hex(verifier.get("executable_sha256"), f"{label}.verifier.executable_sha256")
    if verifier["executable_sha256"] != sha256_bytes(read_bytes(verifier_path, f"{label}.verifier.executable_path")):
        fail(f"{label}.verifier.executable_sha256 mismatch")
    return trust, registry


def _load_adapter_module() -> Any:
    """Load the repository-owned adapter that owns the live root validator."""

    existing = sys.modules.get(ADAPTER_MODULE_NAME)
    if existing is not None:
        return existing
    if ADAPTER_PATH.is_symlink() or not ADAPTER_PATH.is_file():
        fail("repository-owned trust-root validator is unavailable")
    spec = importlib.util.spec_from_file_location(ADAPTER_MODULE_NAME, ADAPTER_PATH)
    if spec is None or spec.loader is None:
        fail("repository-owned trust-root validator cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[ADAPTER_MODULE_NAME] = module
    try:
        spec.loader.exec_module(module)
    except Exception as error:
        sys.modules.pop(ADAPTER_MODULE_NAME, None)
        fail(f"repository-owned trust-root validator failed to load: {error.__class__.__name__}")
    return module


def _validate_governance_root_pin() -> None:
    """Reuse the adapter's code-owned path/content/metadata root validator."""

    adapter = _load_adapter_module()
    adapter_root = Path(adapter.CANONICAL_TRUST_ROOT_PATH)
    if adapter_root.resolve() != DEPLOYED_GOVERNANCE_ROOT.resolve():
        fail("identity-v2 and adapter trust-root paths are not the same code-owned path")
    try:
        adapter.validate_live_trust_root_file()
    except SystemExit as error:
        fail(f"canonical governance-root validation failed: {error}")
    except Exception as error:
        fail(f"canonical governance-root validation failed: {error.__class__.__name__}")


def _authority_scope(trust_path: Path, registry_path: Path, registry: dict[str, Any]) -> str:
    """Require the code-owned deployed trust path and governance-root pin."""

    if trust_path.resolve() != DEPLOYED_TRUST_CONFIG.resolve():
        fail("trust-config is not the canonical deployed authority path")
    if registry_path.resolve() != DEPLOYED_PROVIDER_REGISTRY.resolve():
        fail("provider registry is not the canonical deployed authority path")
    if not PINNED_TRUST_CONFIG_SHA256 or not PINNED_PROVIDER_REGISTRY_SHA256:
        fail("identity-v2 trust-config/provider-registry anchors are not provisioned")
    if sha256_bytes(read_bytes(trust_path, "trust-config")) != PINNED_TRUST_CONFIG_SHA256:
        fail("identity-v2 trust-config content digest is not independently pinned")
    if sha256_bytes(read_bytes(registry_path, "provider-registry")) != PINNED_PROVIDER_REGISTRY_SHA256:
        fail("identity-v2 provider-registry content digest is not independently pinned")
    _validate_governance_root_pin()
    return "deployed-governance-root"


def find_provider(registry: dict[str, Any], provider_id: str) -> dict[str, Any]:
    require_string(provider_id, "provider-ref", safe=True)
    matches = [provider for provider in registry["providers"] if provider["provider_id"] == provider_id]
    if len(matches) != 1:
        fail("provider-ref is not an allowlisted provider ID")
    return matches[0]


def validate_context(context: dict[str, Any], *, now: datetime | None = None) -> dict[str, Any]:
    require_exact_fields(context, CONTEXT_FIELDS, "context")
    if context["schema_version"] != CONTEXT_SCHEMA:
        fail("context schema is unsupported")
    for field in ("network_id", "task_uid", "capture_window_id", "rotation_epoch"):
        require_string(context.get(field), f"context.{field}", safe=(field in {"task_uid", "capture_window_id", "rotation_epoch"}))
    head = require_string(context.get("head_oid"), "context.head_oid")
    if OID.fullmatch(head) is None:
        fail("context.head_oid must be 40 lowercase hexadecimal characters")
    start = parse_timestamp(context.get("capture_start"), "context.capture_start")
    end = parse_timestamp(context.get("capture_end"), "context.capture_end")
    issued = parse_timestamp(context.get("issued_at"), "context.issued_at")
    expires = parse_timestamp(context.get("expires_at"), "context.expires_at")
    if not start <= issued < expires <= end:
        fail("context capture/freshness ordering is invalid")
    current = now or datetime.now(timezone.utc)
    if expires <= current:
        fail("context freshness window is stale")
    return context


def validate_intent(intent: dict[str, Any], context: dict[str, Any]) -> dict[str, Any]:
    require_exact_fields(intent, INTENT_FIELDS, "plan-intent")
    if intent["schema_version"] != INTENT_SCHEMA:
        fail("plan-intent schema is unsupported")
    if intent["context_digest"] != sha256_bytes(canonical(context)):
        fail("plan-intent context digest mismatch")
    require_string(intent.get("adapter_action"), "plan-intent.adapter_action", safe=True)
    nodes = intent["nodes"]
    if not isinstance(nodes, list) or not nodes:
        fail("plan-intent.nodes must be a non-empty array")
    names: set[str] = set()
    peers: set[str] = set()
    previous = ""
    for index, node in enumerate(nodes):
        label = f"plan-intent.nodes[{index}]"
        if not isinstance(node, dict):
            fail(f"{label} must be an object")
        require_exact_fields(node, NODE_FIELDS, label)
        name = require_string(node.get("node_name"), f"{label}.node_name", safe=True)
        if name in names or name < previous:
            fail("plan-intent nodes must be sorted and unique")
        names.add(name)
        previous = name
        require_string(node.get("node_id"), f"{label}.node_id", safe=True)
        peer = require_string(node.get("peer_id"), f"{label}.peer_id", safe=True)
        if peer in peers:
            fail("plan-intent peers must be unique")
        peers.add(peer)
        require_string(node.get("role"), f"{label}.role", safe=True)
        surfaces = node.get("reset_surface_ids")
        if not isinstance(surfaces, list) or not all(isinstance(item, str) and item for item in surfaces):
            fail(f"{label}.reset_surface_ids must contain strings")
        if surfaces != sorted(set(surfaces)):
            fail(f"{label}.reset_surface_ids must be sorted and unique")
    return intent


def validate_template(template: dict[str, Any], raw: dict[str, Any], context: dict[str, Any], plan_digest: str, trust: dict[str, Any]) -> dict[str, Any]:
    require_exact_fields(template, PAYLOAD_FIELDS, "template")
    if template.get("domain_separator") != DOMAIN or template.get("schema_version") != PAYLOAD_SCHEMA:
        fail("template domain or schema is unsupported")
    signer = require_string(template.get("signer_id"), "template.signer_id", safe=True)
    matches = [entry for entry in trust["allowlist"] if entry["signer_id"] == signer]
    if len(matches) != 1:
        fail("template signer is not allowlisted")
    require_string(template.get("verifier_id"), "template.verifier_id")
    if template["verifier_id"] != trust["verifier_id"]:
        fail("template verifier is not governed")
    if template.get("trust_root_id") != trust["trust_root_id"]:
        fail("template trust root is not governed")
    for field in ("task_uid", "capture_window_id", "rotation_epoch", "node_id", "peer_id"):
        require_string(template.get(field), f"template.{field}", safe=(field in {"task_uid", "capture_window_id", "rotation_epoch"}))
    for field in ("head_oid", "frozen_head_oid"):
        if OID.fullmatch(require_string(template.get(field), f"template.{field}")) is None:
            fail(f"template.{field} must be a lowercase commit OID")
    if template["head_oid"] != template["frozen_head_oid"] or template["head_oid"] != context["head_oid"]:
        fail("template head binding mismatch")
    if template["task_uid"] != context["task_uid"] or template["capture_window_id"] != context["capture_window_id"] or template["rotation_epoch"] != context["rotation_epoch"]:
        fail("template context binding mismatch")
    if template["plan_digest"] != plan_digest or template["context_digest"] != sha256_bytes(canonical(context)):
        fail("template plan/context digest mismatch")
    issued = parse_timestamp(template.get("issued_at"), "template.issued_at")
    expires = parse_timestamp(template.get("expires_at"), "template.expires_at")
    if issued != parse_timestamp(context["issued_at"], "context.issued_at") or expires != parse_timestamp(context["expires_at"], "context.expires_at"):
        fail("template freshness binding mismatch")
    if not parse_timestamp(context["capture_start"], "context.capture_start") <= issued < expires <= parse_timestamp(context["capture_end"], "context.capture_end"):
        fail("template freshness is outside capture window")
    for field in ("key_size_bytes", "key_uid", "key_gid"):
        require_integer(template.get(field), f"template.{field}")
    require_hex(template.get("key_sha256"), "template.key_sha256")
    if template["key_sha256"] != raw["key_sha256"] or template["key_size_bytes"] != raw["key_size_bytes"] or template["key_uid"] != raw["key_uid"] or template["key_gid"] != raw["key_gid"] or template["key_mode"] != "0600" or raw["key_mode"] != 0o600:
        fail("template key metadata is not bound to raw-v1")
    if template["node_id"] != raw["node_id"] or template["peer_id"] != raw["peer_id"]:
        fail("template node/peer is not bound to raw-v1")
    require_hex(template.get("signed_payload_sha256"), "template.signed_payload_sha256")
    return template


def payload_bytes(payload: dict[str, Any]) -> bytes:
    require_exact_fields(payload, PAYLOAD_FIELDS, "payload")
    return PREFIX + canonical(payload)


def _atomic_write(path_value: str | Path, value: bytes, label: str) -> None:
    path = Path(path_value)
    if path.exists() and path.is_symlink():
        fail(f"{label} must not be a symlink")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f".{path.name}.", delete=False) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(value)
            temporary.flush()
            os.fsync(temporary.fileno())
        temporary_path.chmod(0o600)
        os.replace(temporary_path, path)
    except OSError as error:
        try:
            temporary_path.unlink(missing_ok=True)
        except (NameError, OSError):
            pass
        fail(f"cannot write {label}: {error.__class__.__name__}")


def _clear_derived_output(path_value: str | Path, label: str) -> None:
    """Remove only a caller-named derived artifact before a new transaction.

    Signature, attestation, envelope, and verification files are disposable
    transaction outputs rather than authority inputs.  Clearing an old
    regular file prevents a failed retry from being mistaken for fresh
    evidence.  Symlinks and non-regular files are rejected, never followed or
    removed.
    """

    path = Path(path_value)
    if not path.exists():
        return
    if path.is_symlink() or not path.is_file():
        fail(f"{label} must be an absent regular output or a regular file")
    try:
        path.unlink()
    except OSError as error:
        fail(f"cannot clear stale {label}: {error.__class__.__name__}")


# RFC 8032 Ed25519 verifier.  It is used only for independent verification;
# signing remains in the external custody adapter.
Q = 2**255 - 19
L = 2**252 + 27742317777372353535851937790883648493
D = (-121665 * pow(121666, Q - 2, Q)) % Q
I = pow(2, (Q - 1) // 4, Q)


def _xrecover(y: int) -> int:
    xx = (y * y - 1) * pow(D * y * y + 1, Q - 2, Q) % Q
    x = pow(xx, (Q + 3) // 8, Q)
    if (x * x - xx) % Q:
        x = x * I % Q
    if (x * x - xx) % Q:
        fail("invalid Ed25519 point")
    return Q - x if x & 1 else x


BY = 4 * pow(5, Q - 2, Q) % Q
BX = _xrecover(BY)
B = (BX, BY)


def _edwards(first: tuple[int, int], second: tuple[int, int]) -> tuple[int, int]:
    x1, y1 = first
    x2, y2 = second
    denominator_x = pow(1 + D * x1 * x2 * y1 * y2, Q - 2, Q)
    denominator_y = pow(1 - D * x1 * x2 * y1 * y2, Q - 2, Q)
    return ((x1 * y2 + x2 * y1) * denominator_x % Q, (y1 * y2 + x1 * x2) * denominator_y % Q)


def _scalarmult(point: tuple[int, int], scalar: int) -> tuple[int, int]:
    result = (0, 1)
    current = point
    while scalar:
        if scalar & 1:
            result = _edwards(result, current)
        current = _edwards(current, current)
        scalar >>= 1
    return result


def _encodepoint(point: tuple[int, int]) -> bytes:
    x, y = point
    encoded = bytearray(y.to_bytes(32, "little"))
    encoded[31] |= (x & 1) << 7
    return bytes(encoded)


def _decodepoint(encoded: bytes) -> tuple[int, int]:
    if len(encoded) != 32:
        fail("invalid Ed25519 point length")
    value = int.from_bytes(encoded, "little")
    sign = value >> 255
    y = value & ((1 << 255) - 1)
    if y >= Q:
        fail("invalid Ed25519 point encoding")
    x = _xrecover(y)
    if (x & 1) != sign:
        x = Q - x
    return x, y


def verify_ed25519(public_key: bytes, message: bytes, signature: bytes) -> bool:
    if len(public_key) != 32 or len(signature) != 64:
        return False
    try:
        public_point = _decodepoint(public_key)
        r_bytes = signature[:32]
        r_point = _decodepoint(r_bytes)
        scalar = int.from_bytes(signature[32:], "little")
        if scalar >= L:
            return False
        challenge = int.from_bytes(hashlib.sha512(r_bytes + public_key + message).digest(), "little") % L
        left = _encodepoint(_scalarmult(B, scalar))
        right = _encodepoint(_edwards(r_point, _scalarmult(public_point, challenge)))
        return left == right
    except ToolError:
        return False


def _manifest_check(manifest: dict[str, Any], payload_path: Path, registry_path: Path) -> None:
    require_string(manifest.get("schema_version"), "prepare-manifest.schema_version")
    if manifest["schema_version"] != "oasis7.identity_v2_prepare_manifest.v1":
        fail("prepare manifest schema is unsupported")
    require_hex(manifest.get("canonical_payload_sha256"), "prepare-manifest.canonical_payload_sha256")
    require_integer(manifest.get("payload_size_bytes"), "prepare-manifest.payload_size_bytes", positive=True)
    require_string(manifest.get("payload_path"), "prepare-manifest.payload_path")
    require_string(manifest.get("provider_registry_path"), "prepare-manifest.provider_registry_path")
    require_string(manifest.get("trust_config_path"), "prepare-manifest.trust_config_path")
    require_string(manifest.get("provider_ref"), "prepare-manifest.provider_ref", safe=True)
    exact = read_bytes(payload_path, "payload")
    if len(exact) != manifest["payload_size_bytes"] or sha256_bytes(exact) != manifest["canonical_payload_sha256"]:
        fail("payload bytes do not match prepare manifest")
    if manifest.get("payload_path") and not resolved_equal(manifest["payload_path"], payload_path):
        fail("payload path is not the path frozen by prepare")
    if manifest.get("provider_registry_path") and not resolved_equal(manifest["provider_registry_path"], registry_path):
        fail("provider registry path is not the path frozen by prepare")
    require_hex(manifest.get("provider_registry_sha256"), "prepare-manifest.provider_registry_sha256")
    if manifest["provider_registry_sha256"] != sha256_bytes(read_bytes(registry_path, "provider-registry")):
        fail("provider registry bytes do not match prepare manifest")


def _provider_attestation(attestation: dict[str, Any], request: dict[str, Any], signature: bytes, payload: bytes) -> None:
    require_exact_fields(attestation, ATTESTATION_FIELDS, "provider-attestation")
    if attestation.get("schema_version") != "oasis7.identity_v2_provider_attestation.v1":
        fail("provider attestation schema is unsupported")
    for field in ("provider_id", "request_id", "signer_id", "context_digest", "rotation_epoch", "capture_window_id", "issued_at"):
        require_string(attestation.get(field), f"provider-attestation.{field}")
    if attestation["provider_id"] != request["provider_id"] or attestation["request_id"] != request["request_id"] or attestation["signer_id"] != request["signer_id"] or attestation["public_key_sha256"] != request["public_key_sha256"] or attestation["context_digest"] != request["context_digest"] or attestation["rotation_epoch"] != request["rotation_epoch"] or attestation["capture_window_id"] != request["capture_window_id"] or attestation["issued_at"] != request["issued_at"]:
        fail("provider attestation context mismatch")
    if attestation.get("algorithm") != ALGORITHM or attestation.get("canonical_payload_sha256") != sha256_bytes(payload) or attestation.get("signature_sha256") != sha256_bytes(signature):
        fail("provider attestation digest or algorithm mismatch")
    require_string(attestation.get("detached_provider_authentication_proof"), "provider-attestation.detached_provider_authentication_proof")


def command_prepare(args: argparse.Namespace) -> None:
    _clear_derived_output(args.payload_out, "payload output")
    _clear_derived_output(args.manifest_out, "prepare manifest output")
    raw_bytes = read_bytes(args.raw_v1, "raw-v1")
    raw = validate_raw(raw_bytes)
    context = validate_context(read_json(args.context, "context"))
    intent = validate_intent(read_json(args.plan_intent, "plan-intent"), context)
    trust_path = _regular(args.trust_config, "trust-config")
    registry_path = _regular(args.provider_registry, "provider-registry")
    trust, registry = validate_registry(read_json(registry_path, "provider-registry"), registry_path, trust_path)
    _authority_scope(trust_path, registry_path, registry)
    template = validate_template(read_json(args.template, "template"), raw, context, sha256_bytes(canonical(intent)), trust)
    if template["signed_payload_sha256"] != sha256_bytes(raw_bytes):
        fail("template raw-v1 digest mismatch")
    payload = payload_bytes(template)
    matching_providers = [provider for provider in registry["providers"] if provider["signer_id"] == template["signer_id"]]
    if len(matching_providers) != 1:
        fail("template signer has no unique provider registry binding")
    matching_provider = matching_providers[0]
    manifest = {
        "schema_version": "oasis7.identity_v2_prepare_manifest.v1",
        "canonical_payload_sha256": sha256_bytes(payload),
        "payload_size_bytes": len(payload),
        "payload_path": str(Path(args.payload_out).resolve()),
        "raw_v1_sha256": sha256_bytes(raw_bytes),
        "raw_v1_size_bytes": len(raw_bytes),
        "template_sha256": sha256_bytes(read_bytes(args.template, "template")),
        "context_digest": sha256_bytes(canonical(context)),
        "plan_digest": sha256_bytes(canonical(intent)),
        "task_uid": template["task_uid"],
        "head_oid": template["head_oid"],
        "frozen_head_oid": template["frozen_head_oid"],
        "node_id": template["node_id"],
        "peer_id": template["peer_id"],
        "capture_window_id": template["capture_window_id"],
        "rotation_epoch": template["rotation_epoch"],
        "issued_at": template["issued_at"],
        "expires_at": template["expires_at"],
        "signer_id": template["signer_id"],
        "verifier_id": template["verifier_id"],
        "trust_root_id": template["trust_root_id"],
        "algorithm": ALGORITHM,
        "trust_config_path": str(trust_path.resolve()),
        "trust_config_sha256": sha256_bytes(read_bytes(trust_path, "trust-config")),
        "provider_registry_path": str(registry_path.resolve()),
        "provider_registry_sha256": sha256_bytes(read_bytes(registry_path, "provider-registry")),
        "provider_ref": matching_provider["provider_id"],
        "public_key_sha256": matching_provider["public_key_sha256"],
        "verifier_executable_sha256": registry["verifier"]["executable_sha256"],
    }
    _atomic_write(args.payload_out, payload, "payload")
    _atomic_write(args.manifest_out, canonical(manifest), "prepare manifest")


def command_sign(args: argparse.Namespace) -> None:
    _clear_derived_output(args.signature_out, "signature output")
    _clear_derived_output(args.attestation_out, "attestation output")
    payload_path = _regular(args.payload, "payload")
    manifest_path = _regular(args.manifest, "prepare manifest")
    manifest = read_json(manifest_path, "prepare manifest")
    registry_path = _regular(args.provider_registry, "provider-registry")
    _manifest_check(manifest, payload_path, registry_path)
    trust_path = _regular(manifest["trust_config_path"], "trust-config")
    _, registry = validate_registry(read_json(registry_path, "provider-registry"), registry_path, trust_path)
    _authority_scope(trust_path, registry_path, registry)
    provider = find_provider(registry, args.provider_ref)
    payload = read_bytes(payload_path, "payload")
    request = {
        "provider_id": provider["provider_id"],
        "request_id": sha256_bytes(payload + canonical(manifest)),
        "signer_id": provider["signer_id"],
        "public_key_sha256": provider["public_key_sha256"],
        "payload_path": str(payload_path.resolve()),
        "canonical_payload_sha256": sha256_bytes(payload),
        "context_digest": manifest["context_digest"],
        "rotation_epoch": manifest["rotation_epoch"],
        "capture_window_id": manifest["capture_window_id"],
        "issued_at": manifest.get("issued_at", ""),
    }
    with tempfile.TemporaryDirectory(prefix="oasis7-identity-v2-provider-") as directory:
        temp = Path(directory)
        request_path = temp / "request.json"
        raw_signature_path = temp / "signature.raw"
        attestation_path = temp / "attestation.json"
        _atomic_write(request_path, canonical(request), "provider request")
        environment = {"PATH": os.environ.get("PATH", ""), "PYTHONIOENCODING": "utf-8"}
        try:
            completed = subprocess.run(
                [provider["adapter_path"], "--request", str(request_path), "--signature-out", str(raw_signature_path), "--attestation-out", str(attestation_path)],
                capture_output=True,
                env=environment,
                cwd=str(temp),
                timeout=30,
            )
        except (OSError, subprocess.TimeoutExpired):
            fail("provider custody adapter failed")
        if completed.returncode != 0:
            fail("provider custody adapter failed")
        signature = read_bytes(raw_signature_path, "provider signature")
        if len(signature) != 64:
            fail("provider signature must be exactly 64 bytes")
        attestation = read_json(attestation_path, "provider attestation")
        _provider_attestation(attestation, request, signature, payload)
        public_key = read_bytes(provider["public_key_ref"], "provider public key")
        if not verify_ed25519(public_key, payload, signature):
            fail("provider signature failed independent Ed25519 verification")
        _atomic_write(args.signature_out, signature.hex().encode("ascii"), "signature")
        _atomic_write(args.attestation_out, canonical(attestation), "provider attestation")


def _payload_from_file(payload_path: Path, manifest: dict[str, Any]) -> dict[str, Any]:
    payload = read_bytes(payload_path, "payload")
    _manifest_check(manifest, payload_path, Path(manifest["provider_registry_path"]))
    if not payload.startswith(PREFIX):
        fail("payload domain prefix mismatch")
    try:
        value = json.loads(payload[len(PREFIX) :].decode("utf-8"), object_pairs_hook=_object_pairs, parse_constant=_reject_constant)
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"payload body is not canonical JSON: {error.__class__.__name__}")
    if not isinstance(value, dict) or canonical(value) != payload[len(PREFIX) :]:
        fail("payload body is not canonical JSON")
    require_exact_fields(value, PAYLOAD_FIELDS, "payload")
    if sha256_bytes(payload) != manifest["canonical_payload_sha256"]:
        fail("payload digest mismatch")
    return value


def command_assemble(args: argparse.Namespace) -> None:
    _clear_derived_output(args.out, "envelope output")
    payload_path = _regular(args.payload, "payload")
    manifest = read_json(args.manifest, "prepare manifest")
    registry_path = _regular(args.provider_registry, "provider-registry")
    _manifest_check(manifest, payload_path, registry_path)
    trust_path = _regular(manifest["trust_config_path"], "trust-config")
    trust, registry = validate_registry(read_json(registry_path, "provider-registry"), registry_path, trust_path)
    authority_scope = _authority_scope(trust_path, registry_path, registry)
    payload = read_bytes(payload_path, "payload")
    fields = _payload_from_file(payload_path, manifest)
    signature_text = read_bytes(args.signature, "signature").decode("ascii")
    if not re.fullmatch(r"[0-9a-f]{128}", signature_text):
        fail("signature must be lowercase 128-hex detached Ed25519 bytes")
    signature = bytes.fromhex(signature_text)
    attestation = read_json(args.attestation, "provider attestation")
    provider = find_provider(registry, attestation.get("provider_id"))
    if manifest.get("provider_ref") != provider["provider_id"]:
        fail("provider attestation does not match the pinned prepare provider")
    request = {
        "provider_id": provider["provider_id"],
        "request_id": sha256_bytes(payload + canonical(manifest)),
        "signer_id": provider["signer_id"],
        "public_key_sha256": provider["public_key_sha256"],
        "context_digest": manifest["context_digest"],
        "rotation_epoch": manifest["rotation_epoch"],
        "capture_window_id": manifest["capture_window_id"],
        "issued_at": manifest.get("issued_at", ""),
    }
    _provider_attestation(attestation, request, signature, payload)
    public_key = read_bytes(provider["public_key_ref"], "provider public key")
    if not verify_ed25519(public_key, payload, signature):
        fail("detached signature failed independent Ed25519 verification")
    envelope = dict(fields)
    envelope["signature_hex"] = signature_text
    envelope["canonical_digest"] = sha256_bytes(canonical(envelope))
    envelope["authenticated"] = False
    envelope["verified"] = False
    _atomic_write(args.out, canonical(envelope), "unsigned identity envelope")


def validate_envelope(value: dict[str, Any]) -> dict[str, Any]:
    allowed = set(PAYLOAD_FIELDS) | {"signature_hex", "canonical_digest", "authenticated", "verified", "historical_only", "apply_authorized"}
    if not set(value) <= allowed or not PAYLOAD_FIELDS <= set(value):
        fail("identity envelope fields are not exact")
    require_hex(value.get("canonical_digest"), "envelope.canonical_digest")
    signature = value.get("signature_hex")
    if not isinstance(signature, str) or re.fullmatch(r"[0-9a-f]{128}", signature) is None:
        fail("envelope.signature_hex must be lowercase 128-hex")
    if not isinstance(value.get("authenticated"), bool) or not isinstance(value.get("verified"), bool):
        fail("envelope verdict fields must be booleans")
    for field in ("historical_only", "apply_authorized"):
        if field in value and not isinstance(value[field], bool):
            fail(f"envelope.{field} must be boolean")
    return value


def _validate_bindings(
    envelope: dict[str, Any], raw: dict[str, Any], raw_bytes: bytes, context: dict[str, Any], intent: dict[str, Any], trust: dict[str, Any]
) -> tuple[dict[str, Any], bytes, dict[str, Any]]:
    if envelope.get("domain_separator") != DOMAIN or envelope.get("schema_version") != PAYLOAD_SCHEMA:
        fail("envelope domain or schema is unsupported")
    if envelope["head_oid"] != envelope["frozen_head_oid"] or envelope["head_oid"] != context["head_oid"]:
        fail("envelope head binding mismatch")
    if envelope["task_uid"] != context["task_uid"] or envelope["capture_window_id"] != context["capture_window_id"] or envelope["rotation_epoch"] != context["rotation_epoch"]:
        fail("envelope context binding mismatch")
    if envelope["issued_at"] != context["issued_at"] or envelope["expires_at"] != context["expires_at"]:
        fail("envelope freshness binding mismatch")
    issued = parse_timestamp(envelope["issued_at"], "envelope.issued_at")
    expires = parse_timestamp(envelope["expires_at"], "envelope.expires_at")
    if not parse_timestamp(context["capture_start"], "context.capture_start") <= issued < expires <= parse_timestamp(context["capture_end"], "context.capture_end"):
        fail("envelope freshness is outside capture window")
    plan_digest = sha256_bytes(canonical(intent))
    if envelope["plan_digest"] != plan_digest or envelope["context_digest"] != sha256_bytes(canonical(context)):
        fail("envelope plan/context digest mismatch")
    if envelope["signed_payload_sha256"] != sha256_bytes(raw_bytes):
        fail("envelope raw-v1 digest mismatch")
    if envelope["node_id"] != raw["node_id"] or envelope["peer_id"] != raw["peer_id"]:
        fail("envelope node/peer binding mismatch")
    for field in ("key_sha256", "key_size_bytes", "key_uid", "key_gid"):
        if envelope[field] != raw[field]:
            fail(f"envelope {field} binding mismatch")
    if envelope["key_mode"] != "0600" or raw["key_mode"] != 0o600:
        fail("envelope key mode binding mismatch")
    signer_matches = [entry for entry in trust["allowlist"] if entry["signer_id"] == envelope["signer_id"]]
    if len(signer_matches) != 1:
        fail("envelope signer is not allowlisted")
    signed = {key: envelope[key] for key in PAYLOAD_FIELDS}
    exact_payload = payload_bytes(signed)
    if envelope["canonical_digest"] != sha256_bytes(canonical({**signed, "signature_hex": envelope["signature_hex"]})):
        fail("envelope canonical digest mismatch")
    return signer_matches[0], exact_payload, signed


def command_verify(args: argparse.Namespace) -> None:
    if args.mode not in {"current_admission", "historical_audit"}:
        fail("verification mode is unsupported")
    _clear_derived_output(args.out, "verified-envelope output")
    _clear_derived_output(args.verification_out, "verification receipt output")
    envelope_path = _regular(args.envelope, "identity envelope")
    envelope = validate_envelope(read_json(envelope_path, "identity envelope"))
    raw_bytes = read_bytes(args.raw_v1, "raw-v1")
    raw = validate_raw(raw_bytes)
    context = validate_context(read_json(args.context, "context"))
    intent = validate_intent(read_json(args.plan_intent, "plan-intent"), context)
    trust_path = _regular(args.trust_config, "trust-config")
    registry_path = _regular(args.provider_registry, "provider-registry")
    trust, registry = validate_registry(read_json(registry_path, "provider-registry"), registry_path, trust_path)
    authority_scope = _authority_scope(trust_path, registry_path, registry)
    signer, exact_payload, signed = _validate_bindings(envelope, raw, raw_bytes, context, intent, trust)
    provider = [entry for entry in registry["providers"] if entry["signer_id"] == signer["signer_id"]]
    if len(provider) != 1 or provider[0]["public_key_sha256"] != signer["public_key_sha256"]:
        fail("envelope signer has no matching pinned provider key")
    public_key = read_bytes(provider[0]["public_key_ref"], "provider public key")
    signature = bytes.fromhex(envelope["signature_hex"])
    if not verify_ed25519(public_key, exact_payload, signature):
        fail("identity envelope signature is invalid")
    issued = parse_timestamp(envelope["issued_at"], "envelope.issued_at")
    expires = parse_timestamp(envelope["expires_at"], "envelope.expires_at")
    valid_from = parse_timestamp(signer["valid_from"], "trust allowlist.valid_from")
    valid_until = parse_timestamp(signer["valid_until"], "trust allowlist.valid_until")
    if not valid_from <= issued < valid_until:
        fail("signer validity interval does not cover receipt issuance")
    capture_start = parse_timestamp(context["capture_start"], "context.capture_start")
    capture_end = parse_timestamp(context["capture_end"], "context.capture_end")
    now = datetime.now(timezone.utc)
    if expires <= now or issued >= expires or issued < capture_start or expires > capture_end:
        fail("identity envelope freshness is stale or inverted")
    current_status = signer["status"]
    historical_only = args.mode == "historical_audit"
    if not historical_only and current_status != "active":
        fail("current admission rejects retired or revoked signer")
    if not historical_only:
        for revocation in trust["revocations"]:
            if revocation["signer_id"] == signer["signer_id"] and parse_timestamp(revocation["effective_at"], "revocation.effective_at") <= now:
                fail("current admission rejects a currently revoked signer")
    if historical_only:
        for revocation in trust["revocations"]:
            if revocation["signer_id"] == signer["signer_id"] and parse_timestamp(revocation["effective_at"], "revocation.effective_at") <= issued:
                fail("historical signer was revoked before issuance")
    envelope_sha = sha256_bytes(read_bytes(envelope_path, "identity envelope"))
    output = dict(signed)
    output["signature_hex"] = envelope["signature_hex"]
    output["canonical_digest"] = envelope["canonical_digest"]
    output["authenticated"] = True
    output["verified"] = True
    output["historical_only"] = historical_only
    output["apply_authorized"] = not historical_only
    receipt = {
        "schema_version": "oasis7.identity_v2_verification_receipt.v1",
        "mode": args.mode,
        "evaluation_time": now.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "raw_v1_sha256": sha256_bytes(raw_bytes),
        "canonical_payload_sha256": sha256_bytes(exact_payload),
        "envelope_sha256": envelope_sha,
        "signer_id": signer["signer_id"],
        "public_key_sha256": signer["public_key_sha256"],
        "trust_config_sha256": sha256_bytes(read_bytes(trust_path, "trust-config")),
        "provider_registry_sha256": sha256_bytes(read_bytes(registry_path, "provider-registry")),
        "verifier_executable_sha256": registry["verifier"]["executable_sha256"],
        "task_uid": envelope["task_uid"],
        "head_oid": envelope["head_oid"],
        "node_id": envelope["node_id"],
        "peer_id": envelope["peer_id"],
        "capture_window_id": envelope["capture_window_id"],
        "rotation_epoch": envelope["rotation_epoch"],
        "historical_only": historical_only,
        "apply_authorized": not historical_only,
        "authority_scope": authority_scope,
        "verified": True,
    }
    _atomic_write(args.out, canonical(output), "verified identity envelope")
    _atomic_write(args.verification_out, canonical(receipt), "verification receipt")


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("prepare")
    for option, dest in (("--raw-v1", "raw_v1"), ("--template", "template"), ("--context", "context"), ("--plan-intent", "plan_intent"), ("--trust-config", "trust_config"), ("--provider-registry", "provider_registry"), ("--payload-out", "payload_out"), ("--manifest-out", "manifest_out")):
        prepare.add_argument(option, dest=dest, required=True)
    prepare.set_defaults(handler=command_prepare)
    sign = commands.add_parser("sign")
    for option, dest in (("--payload", "payload"), ("--manifest", "manifest"), ("--provider-registry", "provider_registry"), ("--provider-ref", "provider_ref"), ("--signature-out", "signature_out"), ("--attestation-out", "attestation_out")):
        sign.add_argument(option, dest=dest, required=True)
    sign.set_defaults(handler=command_sign)
    assemble = commands.add_parser("assemble")
    for option, dest in (("--payload", "payload"), ("--manifest", "manifest"), ("--signature", "signature"), ("--attestation", "attestation"), ("--provider-registry", "provider_registry"), ("--out", "out")):
        assemble.add_argument(option, dest=dest, required=True)
    assemble.set_defaults(handler=command_assemble)
    verify = commands.add_parser("verify")
    verify.add_argument("--mode", required=True, choices=("current_admission", "historical_audit"))
    for option, dest in (("--envelope", "envelope"), ("--raw-v1", "raw_v1"), ("--context", "context"), ("--plan-intent", "plan_intent"), ("--trust-config", "trust_config"), ("--provider-registry", "provider_registry"), ("--out", "out"), ("--verification-out", "verification_out")):
        verify.add_argument(option, dest=dest, required=True)
    verify.set_defaults(handler=command_verify)
    return root


def main(argv: list[str] | None = None) -> int:
    try:
        arguments = parser().parse_args(argv)
        arguments.handler(arguments)
        return 0
    except ToolError as error:
        print(f"error: identity-v2 signing tool: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
