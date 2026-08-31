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
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any, NoReturn


INPUT_SCHEMA = "oasis7.public_testnet_full_network_clean_room_input.v1"
PLAN_SCHEMA = "oasis7.public_testnet_full_network_clean_room_plan.v1"
OID_RE = re.compile(r"^[0-9a-fA-F]{40,64}$")
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
SAFE_NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$")
SECRET_KEY_RE = re.compile(r"(?:password|secret|token|private[_-]?key|sshpass)")

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
    require_hex(receipt.get("signed_payload_sha256"), f"{label}.signed_payload_sha256")
    signature = receipt.get("signature_hex")
    if not isinstance(signature, str) or SIGNATURE_RE.fullmatch(signature) is None:
        die(f"{label}.signature_hex must be a complete Ed25519 signature")
    require_hex(receipt.get("canonical_digest"), f"{label}.canonical_digest")
    return receipt


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
    return tuple(item.replace("{node_id}", node_id) for item in OBSERVER_RESET_SURFACES)


def _validate_state_paths(node: dict[str, Any], expected: dict[str, str]) -> list[str]:
    name = require_string(node.get("name"), "node.name")
    platform = expected["platform"]
    path_style = "windows" if platform == "windows-x64" else "posix"
    root = _normalized_path(node.get("node_root"), path_style, f"{name}.node_root")
    raw_paths = node.get("persistent_state_paths")
    if not isinstance(raw_paths, list) or len(raw_paths) != (8 if expected["role"] == "validator" else 7):
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


def _validate_authority(request: dict[str, Any]) -> dict[str, Any]:
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
    raw_signers = authority.get("signer_allowlist")
    if not isinstance(raw_signers, list) or not raw_signers:
        die("authority signer allowlist is missing")
    signer_allowlist = {require_string(item, "authority.signer_allowlist[]") for item in raw_signers}
    if len(signer_allowlist) != len(raw_signers):
        die("authority signer allowlist contains duplicates")
    receipt = validate_authenticated_receipt(
        authority.get("receipt"), "authority.receipt", signer_allowlist
    )
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
    require_string(verifier.get("executable_path"), "crypto_verifier_receipt.executable_path")
    require_hex(verifier.get("executable_sha256"), "crypto_verifier_receipt.executable_sha256")
    return {
        "task_uid": task_uid,
        "head_oid": head_oid,
        "frozen_head_oid": head_oid,
        "signer_allowlist": sorted(signer_allowlist),
        "crypto_verifier_receipt": verifier,
        "receipt": receipt,
    }


def _positive_size(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        die(f"{label} must be a positive integer")
    return value


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
    return {
        "package": {
            **package,
            "commit": package_commit,
            "genesis_sha256": package_genesis_sha,
            "world_sha256": package_world_sha,
        },
        "genesis": {**genesis, "sha256": genesis_sha},
        "world": {**world, "sha256": world_sha},
        "checkpoint": {**checkpoint, "manifest_hash": manifest_hash, "height": height},
    }


def _validate_probe(
    probe: Any, truth: dict[str, Any], allowed_signers: set[str]
) -> dict[str, Any]:
    value = require_object(probe, "fresh_root_probe")
    schema = require_string(value.get("schema_version"), "fresh_root_probe.schema_version")
    if schema != "oasis7.fresh_root_probe.v1":
        die("fresh_root_probe schema is unsupported")
    if value.get("authenticated") is not True or value.get("verified") is not True:
        die("fresh_root_probe must be authenticated and verified")
    if value.get("package_commit", "").lower() != truth["package"]["commit"]:
        die("fresh_root_probe package binding mismatch")
    checkpoint = truth["checkpoint"]
    if (
        value.get("checkpoint_id") != checkpoint["checkpoint_id"]
        or str(value.get("manifest_hash", "")).lower() != checkpoint["manifest_hash"]
        or value.get("height") != checkpoint["height"]
    ):
        die("fresh_root_probe checkpoint binding mismatch")
    validate_authenticated_receipt(value.get("receipt"), "fresh_root_probe.receipt", allowed_signers)
    return value


def _validate_host_and_endpoints(
    node: dict[str, Any], expected: dict[str, str]
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    name = str(node.get("name"))
    host = require_object(node.get("host_binding"), f"{name}.host_binding")
    target = require_string(host.get("target"), f"{name}.host_binding.target")
    if "/v1/chain/status" in target:
        die(f"{name}.host_binding.target must not contain a status endpoint")
    known_hosts_path = require_string(host.get("known_hosts_path"), f"{name}.host_binding.known_hosts_path")
    if not known_hosts_path.startswith("/") or ".." in PurePosixPath(known_hosts_path).parts:
        die(f"{name}.host_binding.known_hosts_path must be an absolute operator path")
    fingerprint = require_string(host.get("known_host_fingerprint"), f"{name}.host_binding.known_host_fingerprint")
    if re.fullmatch(r"SHA256:[A-Za-z0-9+/]{20,}", fingerprint) is None:
        die(f"{name}.host_binding.known_host_fingerprint is malformed")

    endpoints = require_object(node.get("endpoints"), f"{name}.endpoints")
    healthz = require_string(endpoints.get("healthz"), f"{name}.endpoints.healthz")
    evidence = require_string(endpoints.get("evidence"), f"{name}.endpoints.evidence")
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
    return (
        {"target": target, "known_hosts_path": known_hosts_path, "known_host_fingerprint": fingerprint},
        {"healthz": healthz, "evidence": evidence},
        {"kind": seam["kind"], "environment_name": environment_name, "nonce": nonce},
    )


def _validate_nodes(nodes: Any, truth: dict[str, Any], allowed_signers: set[str]) -> dict[str, dict[str, Any]]:
    if not isinstance(nodes, list) or len(nodes) != len(NODE_ORDER):
        die("nodes must contain exactly the five managed nodes")
    by_name: dict[str, dict[str, Any]] = {}
    for index, raw in enumerate(nodes):
        node = require_object(raw, f"nodes[{index}]")
        name = require_string(node.get("name"), f"nodes[{index}].name")
        if name not in EXPECTED_NODES or name in by_name:
            die(f"nodes contains an unexpected or duplicate node: {name}")
        expected = EXPECTED_NODES[name]
        for field in ("node_id", "role", "platform", "service_manager", "service"):
            if node.get(field) != expected[field]:
                die(f"{name}.{field} does not match the governed identity/service contract")
        identity_receipt = require_object(node.get("identity_receipt"), f"{name}.identity_receipt")
        if identity_receipt.get("node_id") != expected["node_id"]:
            die(f"{name}.identity_receipt node_id binding mismatch")
        validate_authenticated_receipt(identity_receipt, f"{name}.identity_receipt", allowed_signers)
        require_string(identity_receipt.get("peer_id"), f"{name}.identity_receipt.peer_id")
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
        if "node_root" in expected:
            expected_root = _normalized_path(
                expected["node_root"],
                "windows" if expected["platform"] == "windows-x64" else "posix",
                f"{name}.expected_node_root",
            )
            actual_root = _normalized_path(
                node.get("node_root"),
                "windows" if expected["platform"] == "windows-x64" else "posix",
                f"{name}.node_root",
            )
            root_mismatch = (
                actual_root.lower() != expected_root.lower()
                if expected["platform"] == "windows-x64"
                else actual_root != expected_root
            )
            if root_mismatch:
                die(f"{name}.node_root does not match the governed path inventory")
        host_binding, endpoints, credential_seam = _validate_host_and_endpoints(node, expected)
        _validate_state_paths(node, expected)
        by_name[name] = {
            "name": name,
            "node_id": expected["node_id"],
            "role": expected["role"],
            "platform": expected["platform"],
            "node_root": node["node_root"],
            "service_manager": expected["service_manager"],
            "service": expected["service"],
            "persistent_state_paths": list(node["persistent_state_paths"]),
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
    request: dict[str, Any], authority: dict[str, Any], allowed_signers: set[str]
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
    if (
        policy.get("task_uid") != authority["task_uid"]
        or str(policy.get("frozen_head_oid", "")).lower() != authority["head_oid"]
    ):
        die("backup_policy authority task/frozen-head binding mismatch")
    require_string(policy.get("reason"), "backup_policy.reason")
    authority = validate_authenticated_receipt(
        policy.get("authority"), "backup_policy.authority", allowed_signers
    )
    return {
        "mode": mode,
        "required_before_reset": False,
        "operator_authorized": True,
        "authority": authority,
    }


def _global_order(backup_required: bool) -> list[str]:
    order: list[str] = []
    phases = ["preflight"]
    if backup_required:
        phases.append("forensic-backup")
    phases.extend(("stop", "delete", "rebuild"))
    for phase in phases:
        order.extend(f"{phase}:{name}" for name in NODE_ORDER)
    order.extend(("start:sequencer-204", "verify:sequencer-204", "start:storage-205", "verify:storage-205"))
    order.append("fresh-root-probe")
    for name in ("linux-lan-observer", "windows-observer", "macos-observer"):
        order.extend((f"start:{name}", f"verify:{name}"))
    order.append("fleet-health")
    return order


def _operation_journal(global_order: list[str]) -> list[dict[str, Any]]:
    journal: list[dict[str, Any]] = []
    for sequence, entry in enumerate(global_order, 1):
        phase, _, node = entry.partition(":")
        journal.append(
            {
                "sequence": sequence,
                "phase": phase,
                "node": node or None,
                "operation": entry,
            }
        )
    return journal


def build_plan(request: dict[str, Any]) -> dict[str, Any]:
    request = require_object(request, "clean-room input")
    reject_secret_fields(request)
    if request.get("schema_version") != INPUT_SCHEMA:
        die("input schema is unsupported")
    authority = _validate_authority(request)
    _validate_no_old_state_copy(request)
    allowed_signers = set(authority["signer_allowlist"])
    truth = _validate_truth(request.get("truth"), allowed_signers)
    probe = _validate_probe(request.get("fresh_root_probe"), truth, allowed_signers)
    nodes = _validate_nodes(request.get("nodes"), truth, allowed_signers)
    backup_policy = _validate_backup_policy(request, authority, allowed_signers)
    global_order = _global_order(backup_policy["required_before_reset"])
    plan: dict[str, Any] = {
        "schema_version": PLAN_SCHEMA,
        "task_uid": authority["task_uid"],
        "head_oid": authority["head_oid"],
        "authority": authority,
        "execution": {
            "mode": "plan-only",
            "provider_mutation_performed": False,
            "provider_mutation_boundary": "external-governed-adapter-required",
        },
        "node_order": list(NODE_ORDER),
        "global_order": global_order,
        "operation_journal": _operation_journal(global_order),
        "surfaces": {
            "validators": list(VALIDATOR_RESET_SURFACES),
            "observers": list(OBSERVER_RESET_SURFACES),
            "validator_count": 8,
            "observer_count": 7,
        },
        "truth": {
            "package": truth["package"],
            "genesis": truth["genesis"],
            "world": truth["world"],
            "checkpoint": truth["checkpoint"],
        },
        "fresh_root_probe": probe,
        "nodes": [nodes[name] for name in NODE_ORDER],
        "forensic_backup": {
            "mode": backup_policy["mode"],
            "required_before_reset": backup_policy["required_before_reset"],
            "operator_authorized": backup_policy["operator_authorized"],
            "immutable": backup_policy["required_before_reset"],
            "seed_eligible": False,
            "cross_node_state_copy": False,
            "restore_old_state": False,
            "receipt_required_per_node": backup_policy["required_before_reset"],
            "authority": backup_policy["authority"],
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
