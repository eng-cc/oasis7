#!/usr/bin/env python3
"""Collect one bounded public-testnet fleet-health evidence window."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import Request, urlopen


MANAGED_FIVE_NODE_NAMES = frozenset(
    {
        "sequencer",
        "storage",
        "linux-lan-observer",
        "windows-observer",
        "macos-observer",
    }
)
MANAGED_FIVE_NODE_SEQUENCER = "sequencer"
MANAGED_FIVE_NODE_STORAGE = "storage"
MIN_CHECKPOINT_SCHEMA_VERSION = 2
MAX_PROVIDER_CHECKPOINT_HEIGHT_DELTA = 1
TRIAD_STATUS_PROJECTION_SCHEMA = "oasis7.chain_validator_provider_status.v1"
TRIAD_INVENTORY_RELATIVE = "scripts/public-testnet-validator-triad-inventory.v1.json"
TRIAD_INVENTORY_PATH = Path(__file__).resolve().parent / "public-testnet-validator-triad-inventory.v1.json"
TRIAD_INVENTORY_SHA256 = "3313a899630e3013d623adfee252556a124c25d059406bcf98a541ae2fcdacd5"
TRIAD_SOURCE_REGISTRY_RELATIVE = (
    "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json"
)
TRIAD_SOURCE_REGISTRY_SHA256 = "a6bfa524e32f2f54c4665d58f18e87b5fa21845e17c14269be1cb1f978adb50f"
TRIAD_GENERATED_REGISTRY_SHA256 = "8bfb4411f3895ab5f1a2a3de1bcaa08ce97567202d4198444b323ef437a88f78"
TRIAD_GENERATED_REGISTRY_SEMANTIC_SHA256 = "aa6f6f7f367470d3b2c7282d489422d3eef14370446aa1fe8420fc94d776950d"
TRIAD_BOOTSTRAP_PEER_RELATIVE = (
    "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt"
)
TRIAD_BOOTSTRAP_PEER_SHA256 = "c7d0b977937adb5d27733ed0ad3e2212ccd0f3ac1b2273214e8cc57df110e5d6"
MANAGED_TRIAD_NODE_IDS = {
    "sequencer-204": "triad-testnet-sequencer",
    "storage-205": "triad-testnet-storage",
    "validator-47": "triad-testnet-validator-47",
}
MANAGED_TRIAD_RUNTIME_ROLES = {
    "sequencer-204": "sequencer",
    "storage-205": "storage",
    # Validator membership is PoS/governance state; storage is the supported
    # runtime role for a validator that also serves full-storage proofs.
    "validator-47": "storage",
}
MANAGED_TRIAD_P2P_ROLES = {
    "sequencer-204": "validator_core",
    "storage-205": "full_storage",
    "validator-47": "full_storage",
}
MANAGED_TRIAD_NAMES = frozenset(MANAGED_TRIAD_NODE_IDS)
MANAGED_TRIAD_SEQUENCER = "sequencer-204"
MANAGED_TRIAD_TOTAL_STAKE = 300
MANAGED_TRIAD_REQUIRED_STAKE = 200
MANAGED_TRIAD_QUORUM = {"numerator": 2, "denominator": 3}
MANAGED_TRIAD_GOVERNANCE = {
    "signer_count": 3,
    "threshold": 2,
    "threshold_bps": 6667,
}


def triad_inventory_authority() -> tuple[dict[str, Any], str] | None:
    """Read and semantically validate the immutable triad authority chain."""
    try:
        raw = TRIAD_INVENTORY_PATH.read_bytes()
        inventory = json.loads(raw.decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None
    if hashlib.sha256(raw).hexdigest() != TRIAD_INVENTORY_SHA256:
        return None
    if not isinstance(inventory, dict):
        return None
    if inventory.get("schema_version") != "oasis7.public_testnet_validator_triad_inventory.v1":
        return None
    if inventory.get("repository") != "eng-cc/oasis7":
        return None
    if inventory.get("network_tier") != "public_testnet":
        return None
    if inventory.get("topology") != "three_equal_validator":
        return None
    authority = inventory.get("authority")
    validator_set = inventory.get("validator_set")
    governance = inventory.get("governance")
    nodes = inventory.get("nodes")
    if (
        not isinstance(authority, dict)
        or not isinstance(validator_set, dict)
        or not isinstance(governance, dict)
        or not isinstance(nodes, dict)
        or set(nodes) != MANAGED_TRIAD_NAMES
    ):
        return None
    if (
        authority.get("world_id") != "oasis7-public-testnet-governed-20260606"
        or authority.get("chain_id") != authority.get("world_id")
        or authority.get("network_tier") != "public_testnet"
        or authority.get("registry_ref")
        != "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
        or authority.get("source_registry_ref") != TRIAD_SOURCE_REGISTRY_RELATIVE
        or authority.get("source_registry_sha256") != TRIAD_SOURCE_REGISTRY_SHA256
        or authority.get("generated_registry_sha256") != TRIAD_GENERATED_REGISTRY_SHA256
        or authority.get("generated_registry_semantic_sha256")
        != TRIAD_GENERATED_REGISTRY_SEMANTIC_SHA256
        or authority.get("bootstrap_peer_ref") != TRIAD_BOOTSTRAP_PEER_RELATIVE
        or authority.get("bootstrap_peer_sha256") != TRIAD_BOOTSTRAP_PEER_SHA256
        or validator_set
        != {
            "count": 3,
            "stakes": [100, 100, 100],
            "total_stake": 300,
            "required_stake": 200,
            "quorum": {"numerator": 2, "denominator": 3},
        }
        or governance != MANAGED_TRIAD_GOVERNANCE
    ):
        return None
    for name, expected_node_id in MANAGED_TRIAD_NODE_IDS.items():
        node = nodes.get(name)
        if not isinstance(node, dict):
            return None
        if node.get("node_id") != expected_node_id or node.get("stake") != 100:
            return None
        signer = node.get("finality_signer_public_key")
        if not isinstance(signer, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", signer):
            return None
        if name == "validator-47":
            if node.get("roles") != ["validator", "checkpoint_provider", "full_storage_provider"]:
                return None
            if not isinstance(node.get("libp2p_peer_id"), str) or not node["libp2p_peer_id"].strip():
                return None

    repo_root = Path(__file__).resolve().parent.parent

    def authority_file(raw_ref: object, expected_ref: str) -> bytes | None:
        if raw_ref != expected_ref or not isinstance(raw_ref, str):
            return None
        relative = Path(raw_ref)
        if relative.is_absolute():
            return None
        candidate = repo_root / relative
        if candidate.is_symlink() or not candidate.is_file():
            return None
        try:
            resolved = candidate.resolve()
            resolved.relative_to(repo_root)
            return resolved.read_bytes()
        except (OSError, ValueError):
            return None

    source_bytes = authority_file(authority.get("source_registry_ref"), TRIAD_SOURCE_REGISTRY_RELATIVE)
    peer_bytes = authority_file(authority.get("bootstrap_peer_ref"), TRIAD_BOOTSTRAP_PEER_RELATIVE)
    if source_bytes is None or peer_bytes is None:
        return None
    if hashlib.sha256(source_bytes).hexdigest() != TRIAD_SOURCE_REGISTRY_SHA256:
        return None
    if hashlib.sha256(peer_bytes).hexdigest() != TRIAD_BOOTSTRAP_PEER_SHA256:
        return None

    try:
        source_registry = json.loads(source_bytes.decode("utf-8"))
        peer_lines = [line.strip() for line in peer_bytes.decode("utf-8").splitlines() if line.strip()]
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None
    if not isinstance(source_registry, dict):
        return None
    if (
        source_registry.get("schema_version") != "oasis7.public_testnet_validator_triad_registry.v1"
        or source_registry.get("network_tier") != "public_testnet"
        or source_registry.get("topology") != "three_equal_validator"
        or source_registry.get("slot_id") != "governance.finality.v1"
        or source_registry.get("threshold") != 2
        or source_registry.get("threshold_bps") != 0
    ):
        return None
    source_validators = source_registry.get("validators")
    if not isinstance(source_validators, list) or len(source_validators) != 3:
        return None
    source_by_id = {
        item.get("node_id"): item
        for item in source_validators
        if isinstance(item, dict) and isinstance(item.get("node_id"), str)
    }
    if set(source_by_id) != {node["node_id"] for node in nodes.values()}:
        return None
    for node in nodes.values():
        source = source_by_id[node["node_id"]]
        for field in ("stake", "finality_signer_public_key"):
            if str(source.get(field, "")).lower() != str(node.get(field, "")).lower():
                return None
        if node["node_id"] == MANAGED_TRIAD_NODE_IDS["validator-47"]:
            for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
                if source.get(field) != node.get(field):
                    return None
    if triad_registry_semantic_digest(inventory) != authority["generated_registry_semantic_sha256"]:
        return None
    if len(peer_lines) != 3:
        return None
    for node in nodes.values():
        host = str(node.get("host", "")).removeprefix("root@")
        ports = node.get("ports")
        if not host or not isinstance(ports, list) or len(ports) < 2:
            return None
        prefix = f"/ip4/{host}/tcp/{ports[1]}/p2p/"
        matches = [line for line in peer_lines if line.startswith(prefix)]
        if len(matches) != 1:
            return None
        peer_id = node.get("libp2p_peer_id")
        if peer_id and matches[0] != f"{prefix}{peer_id}":
            return None
    return inventory, hashlib.sha256(raw).hexdigest()


def triad_registry_semantic_digest(inventory: dict[str, Any]) -> str:
    """Match the runtime's canonical effective-registry semantic digest."""
    nodes = inventory["nodes"]
    signer_bindings = {
        f"governance.finality.v1.{nodes[name]['node_id']}": nodes[name][
            "finality_signer_public_key"
        ].lower()
        for name in sorted(nodes)
    }
    validator_stakes = {node_id: 100 for node_id in sorted(signer_bindings)}
    return hashlib.sha256(
        json.dumps(
            {
                "signer_bindings": signer_bindings,
                "slot_id": "governance.finality.v1",
                "threshold": 2,
                "threshold_bps": 0,
                "validator_stakes": validator_stakes,
            },
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
    ).hexdigest()


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def parse_node(raw: str) -> tuple[str, str]:
    name, separator, url = raw.partition("=")
    if not separator or not name or not url:
        raise argparse.ArgumentTypeError("node must be NAME=URL")
    return name, url


def read_json(url: str) -> dict[str, Any]:
    request = Request(url, headers={"Accept": "application/json"})
    with urlopen(request, timeout=10) as response:  # noqa: S310 - operator-provided status URL
        payload = json.loads(response.read().decode("utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("status payload is not a JSON object")
    return payload


def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def node_gates(status: dict[str, Any], sequencer_consensus: dict[str, Any]) -> list[str]:
    gates: list[str] = []
    readiness = status.get("readiness")
    if status.get("running") is not True or not isinstance(readiness, dict) or readiness.get("status") != "ready":
        gates.append("node_not_ready")
    if readiness.get("failed_gates") if isinstance(readiness, dict) else None:
        gates.append("failed_gates_nonempty")
    if status.get("last_error") is not None:
        gates.append("last_error_present")

    consensus = status.get("consensus")
    if not isinstance(consensus, dict):
        return gates + ["head_mismatch", "network_head_not_ready"]
    for field in ("committed_height", "network_committed_height", "last_execution_height"):
        expected = sequencer_consensus.get(field)
        if expected is None or consensus.get(field) != expected:
            gates.append("head_mismatch")
            break
    network_head = consensus.get("network_head")
    if not isinstance(network_head, dict) or network_head.get("decision") != "ready":
        gates.append("network_head_not_ready")
    return gates


def provider_checkpoint(status: dict[str, Any]) -> tuple[int, str, int, str] | None:
    chain_proof = status.get("chain_proof")
    if not isinstance(chain_proof, dict):
        return None
    checkpoint = chain_proof.get("latest_execution_checkpoint")
    if not isinstance(checkpoint, dict):
        return None
    schema_version = checkpoint.get("schema_version")
    checkpoint_id = checkpoint.get("checkpoint_id")
    height = checkpoint.get("height")
    manifest_hash = checkpoint.get("manifest_hash")
    if (
        isinstance(schema_version, bool)
        or not isinstance(schema_version, int)
        or isinstance(height, bool)
        or not isinstance(height, int)
        or not isinstance(checkpoint_id, str)
        or not isinstance(manifest_hash, str)
    ):
        return None
    return schema_version, checkpoint_id, height, manifest_hash


def provider_checkpoint_gates(captured: dict[str, dict[str, Any]]) -> list[str]:
    checkpoints: dict[str, tuple[int, str, int, str]] = {}
    gates: list[str] = []
    for name in (MANAGED_FIVE_NODE_SEQUENCER, MANAGED_FIVE_NODE_STORAGE):
        checkpoint = provider_checkpoint(captured[name])
        if checkpoint is None:
            gates.append("provider_checkpoint_missing")
            continue
        schema_version, checkpoint_id, height, manifest_hash = checkpoint
        if schema_version < MIN_CHECKPOINT_SCHEMA_VERSION:
            gates.append("provider_checkpoint_schema_invalid")
        if not checkpoint_id.strip() or not re.fullmatch(r"[0-9a-fA-F]{64}", manifest_hash):
            gates.append("provider_checkpoint_identity_invalid")
        if height <= 0:
            gates.append("provider_checkpoint_height_invalid")
        checkpoints[name] = checkpoint
    if len(checkpoints) == 2:
        _, sequencer_id, sequencer_height, sequencer_hash = checkpoints[MANAGED_FIVE_NODE_SEQUENCER]
        _, storage_id, storage_height, storage_hash = checkpoints[MANAGED_FIVE_NODE_STORAGE]
        if sequencer_id != storage_id or sequencer_hash.lower() != storage_hash.lower():
            gates.append("provider_checkpoint_identity_mismatch")
        if abs(sequencer_height - storage_height) > MAX_PROVIDER_CHECKPOINT_HEIGHT_DELTA:
            gates.append("provider_checkpoint_height_incompatible")
    return gates


def triad_validator_gates(
    name: str,
    status: dict[str, Any],
    inventory_authority: tuple[dict[str, Any], str] | None,
) -> list[str]:
    """Require an active validator identity anchored to governed inventory."""
    gates: list[str] = []
    expected_node_id = MANAGED_TRIAD_NODE_IDS[name]
    if status.get("node_id") != expected_node_id:
        gates.append("validator_identity_mismatch")
    if status.get("role") != MANAGED_TRIAD_RUNTIME_ROLES[name]:
        gates.append("runtime_role_invalid")
    p2p = status.get("p2p")
    if not isinstance(p2p, dict) or p2p.get("node_role_claim") != MANAGED_TRIAD_P2P_ROLES[name]:
        gates.append("p2p_role_invalid")
    if inventory_authority is None:
        return gates + ["inventory_authority_unavailable"]

    inventory, inventory_sha256 = inventory_authority
    inventory_node = inventory["nodes"][name]
    expected_world_id = inventory["authority"]["world_id"]
    expected_chain_id = inventory["authority"]["chain_id"]
    expected_registry_sha256 = inventory["authority"]["generated_registry_sha256"]
    expected_signer = inventory_node["finality_signer_public_key"].lower()
    if status.get("world_id") != expected_world_id:
        gates.append("world_identity_mismatch")
    network_tier = status.get("network_tier")
    if not isinstance(network_tier, dict):
        gates.append("network_tier_identity_missing")
    else:
        if network_tier.get("tier") != "public_testnet":
            gates.append("network_tier_identity_mismatch")
        if network_tier.get("network_id") != expected_world_id:
            gates.append("network_identity_mismatch")
        if network_tier.get("chain_id") != expected_chain_id:
            gates.append("chain_identity_mismatch")
        if network_tier.get("target_validator_count") != 3:
            gates.append("validator_count_mismatch")
    world_resource = status.get("world_resource")
    if not isinstance(world_resource, dict):
        gates.append("world_resource_identity_missing")
    else:
        if world_resource.get("world_id") != expected_world_id:
            gates.append("world_identity_mismatch")
        if world_resource.get("chain_id") != expected_chain_id:
            gates.append("chain_identity_mismatch")
    chain_proof = status.get("chain_proof")
    latest_proof = chain_proof.get("latest_world_head_proof") if isinstance(chain_proof, dict) else None
    if not isinstance(latest_proof, dict) or latest_proof.get("world_id") != expected_world_id:
        gates.append("world_identity_mismatch")

    validator = status.get("validator")
    if not isinstance(validator, dict):
        return gates + ["validator_metadata_missing"]
    if validator.get("schema_version") != TRIAD_STATUS_PROJECTION_SCHEMA:
        gates.append("validator_projection_schema_invalid")
    if validator.get("role") != "validator":
        gates.append("validator_role_invalid")
    if validator.get("membership") != "active":
        gates.append("validator_membership_invalid")
    stake = validator.get("stake")
    if isinstance(stake, bool) or not isinstance(stake, int) or stake != inventory_node["stake"]:
        gates.append("validator_stake_mismatch")
    if validator.get("signer_binding") != expected_node_id:
        gates.append("validator_signer_binding_mismatch")
    signer = validator.get("signer_public_key_hex")
    if not isinstance(signer, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", signer):
        gates.append("signer_identity_missing")
    elif signer.lower() != expected_signer:
        gates.append("signer_identity_mismatch")
    stake_proof = validator.get("stake_proof")
    if not isinstance(stake_proof, dict):
        gates.append("validator_stake_proof_missing")
    else:
        if stake_proof.get("validator_id") != expected_node_id:
            gates.append("signer_identity_mismatch")
        if stake_proof.get("stake") != stake:
            gates.append("validator_stake_mismatch")
        if stake_proof.get("signer_public_key_hex") != signer:
            gates.append("signer_identity_mismatch")
        if not isinstance(stake_proof.get("player_id"), str) or not stake_proof["player_id"].strip():
            gates.append("validator_stake_proof_invalid")
        if not isinstance(stake_proof.get("leaf_hash"), str) or not stake_proof["leaf_hash"].strip():
            gates.append("validator_stake_proof_invalid")
        if not isinstance(stake_proof.get("proof"), list):
            gates.append("validator_stake_proof_invalid")
    consensus = status.get("consensus")
    if not isinstance(consensus, dict):
        gates.append("validator_consensus_source_missing")
    else:
        if validator.get("validator_set_hash") != consensus.get("validator_set_hash"):
            gates.append("validator_consensus_identity_mismatch")
        if validator.get("stake_root") != consensus.get("validator_stake_root"):
            gates.append("validator_consensus_identity_mismatch")
    for field in ("validator_set_hash", "stake_root"):
        value = validator.get(field)
        if not isinstance(value, str) or not value.strip():
            gates.append("validator_consensus_identity_missing")
    for field in ("registry_ref", "registry_sha256"):
        value = validator.get(field)
        if not isinstance(value, str) or not value.strip():
            gates.append("registry_identity_missing")
    actual_registry_sha256 = validator.get("registry_sha256")
    if (
        not isinstance(actual_registry_sha256, str)
        or actual_registry_sha256.lower() != expected_registry_sha256
    ):
        gates.append("registry_authority_digest_mismatch")
    if validator.get("registry_semantic_sha256") != inventory["authority"]["generated_registry_semantic_sha256"]:
        gates.append("registry_semantic_identity_mismatch")
    if validator.get("inventory_ref") != TRIAD_INVENTORY_RELATIVE:
        gates.append("inventory_authority_ref_mismatch")
    if validator.get("inventory_sha256", "").lower() != inventory_sha256:
        gates.append("inventory_authority_digest_mismatch")
    for field in ("inventory_ref", "inventory_sha256"):
        value = validator.get(field)
        if not isinstance(value, str) or not value.strip():
            gates.append("inventory_identity_missing")
    return gates


def triad_provider_gates(
    status: dict[str, Any],
    inventory_authority: tuple[dict[str, Any], str] | None,
) -> list[str]:
    """Require validator-47 to close both checkpoint and full-storage roles."""
    provider = status.get("provider")
    if not isinstance(provider, dict):
        return ["validator_47_provider_metadata_missing"]
    gates: list[str] = []
    if inventory_authority is None:
        return ["inventory_authority_unavailable"]
    inventory, _inventory_sha256 = inventory_authority
    inventory_node = inventory["nodes"]["validator-47"]
    expected_world_id = inventory["authority"]["world_id"]
    expected_chain_id = inventory["authority"]["chain_id"]
    if provider.get("schema_version") != TRIAD_STATUS_PROJECTION_SCHEMA:
        gates.append("validator_47_provider_schema_invalid")
    if provider.get("node_id") != MANAGED_TRIAD_NODE_IDS["validator-47"]:
        gates.append("provider_identity_mismatch")
    provider_id = provider.get("provider_id")
    if not isinstance(provider_id, str) or not provider_id.strip():
        gates.append("validator_47_provider_identity_missing")
    elif provider_id != inventory_node.get("libp2p_peer_id"):
        gates.append("provider_identity_mismatch")
    if provider.get("checkpoint") is not True:
        gates.append("validator_47_provider_checkpoint_missing")
    if provider.get("full_storage") is not True:
        gates.append("validator_47_provider_full_storage_missing")
    checkpoint = provider.get("checkpoint_proof")
    if not isinstance(checkpoint, dict):
        gates.append("validator_47_provider_checkpoint_proof_missing")
    else:
        if (
            isinstance(checkpoint.get("schema_version"), bool)
            or not isinstance(checkpoint.get("schema_version"), int)
            or checkpoint.get("schema_version", 0) < MIN_CHECKPOINT_SCHEMA_VERSION
        ):
            gates.append("provider_checkpoint_schema_invalid")
        for field in ("checkpoint_id", "proof_hash", "world_id", "chain_id", "manifest_hash"):
            value = checkpoint.get(field)
            if not isinstance(value, str) or not value.strip():
                gates.append("provider_checkpoint_identity_invalid")
        manifest_hash = checkpoint.get("manifest_hash")
        if isinstance(manifest_hash, str) and not re.fullmatch(r"[0-9a-fA-F]{64}", manifest_hash):
            gates.append("provider_checkpoint_identity_invalid")
        if (
            isinstance(checkpoint.get("height"), bool)
            or not isinstance(checkpoint.get("height"), int)
            or checkpoint.get("height", 0) <= 0
        ):
            gates.append("provider_checkpoint_height_invalid")
        world_resource = status.get("world_resource")
        expected_world_id = inventory["authority"]["world_id"]
        expected_chain_id = inventory["authority"]["chain_id"]
        expected_manifest_hash = (
            world_resource.get("seed_manifest_hash") if isinstance(world_resource, dict) else None
        )
        if checkpoint.get("world_id") != expected_world_id:
            gates.append("provider_checkpoint_world_identity_mismatch")
        if checkpoint.get("chain_id") != expected_chain_id:
            gates.append("provider_checkpoint_chain_identity_mismatch")
        if checkpoint.get("manifest_hash") != expected_manifest_hash:
            gates.append("provider_checkpoint_manifest_mismatch")
        chain_proof = status.get("chain_proof")
        latest_proof = (
            chain_proof.get("latest_world_head_proof")
            if isinstance(chain_proof, dict)
            else None
        )
        latest_checkpoint = (
            chain_proof.get("latest_execution_checkpoint")
            if isinstance(chain_proof, dict)
            else None
        )
        if not isinstance(latest_proof, dict) or not isinstance(latest_checkpoint, dict):
            gates.append("provider_checkpoint_source_missing")
        elif checkpoint.get("proof_hash") != latest_proof.get("proof_hash"):
            gates.append("provider_checkpoint_proof_mismatch")
        elif checkpoint.get("checkpoint_id") != latest_checkpoint.get("checkpoint_id"):
            gates.append("provider_checkpoint_proof_mismatch")
        elif checkpoint.get("height") != latest_checkpoint.get("height"):
            gates.append("provider_checkpoint_height_mismatch")
        elif checkpoint.get("manifest_hash") != latest_checkpoint.get("manifest_hash"):
            gates.append("provider_checkpoint_manifest_mismatch")
    full_storage = provider.get("full_storage_proof")
    if not isinstance(full_storage, dict):
        gates.append("validator_47_provider_full_storage_proof_missing")
    else:
        if full_storage.get("status") != "ready":
            gates.append("validator_47_provider_full_storage_not_ready")
        if full_storage.get("provider_id") != provider_id:
            gates.append("provider_identity_mismatch")
        world_resource = status.get("world_resource")
        if full_storage.get("world_id") != expected_world_id:
            gates.append("provider_full_storage_world_identity_mismatch")
        if (
            full_storage.get("chain_id") != expected_chain_id
        ):
            gates.append("provider_full_storage_chain_identity_mismatch")
        if (
            isinstance(world_resource, dict)
            and full_storage.get("manifest_hash") != world_resource.get("seed_manifest_hash")
        ):
            gates.append("provider_full_storage_manifest_mismatch")
        checkpoint = provider.get("checkpoint_proof")
        if isinstance(checkpoint, dict) and full_storage.get("height") != checkpoint.get("height"):
            gates.append("provider_full_storage_height_mismatch")
    return gates


def triad_projection_consistency_gates(
    captured: dict[str, dict[str, Any]],
    inventory_authority: tuple[dict[str, Any], str] | None,
) -> list[str]:
    """Cross-bind node-emitted validator/provider projections without inference."""
    gates: list[str] = []
    validator_set_hashes: set[str] = set()
    stake_roots: set[str] = set()
    registry_refs: set[str] = set()
    registry_digests: set[str] = set()
    inventory_refs: set[str] = set()
    inventory_digests: set[str] = set()
    world_ids: set[str] = set()
    chain_ids: set[str] = set()
    manifest_hashes: set[str] = set()
    network_ids: set[str] = set()
    network_tiers: set[str] = set()
    validator_counts: set[int] = set()
    if inventory_authority is None:
        gates.append("inventory_authority_unavailable")
        return gates
    inventory, inventory_sha256 = inventory_authority
    expected_world_id = inventory["authority"]["world_id"]
    expected_chain_id = inventory["authority"]["chain_id"]
    expected_registry_ref = inventory["authority"]["registry_ref"]
    expected_registry_sha256 = inventory["authority"]["generated_registry_sha256"]
    expected_registry_semantic_sha256 = inventory["authority"]["generated_registry_semantic_sha256"]
    for status in captured.values():
        validator = status.get("validator")
        if isinstance(validator, dict):
            for value, values in (
                (validator.get("validator_set_hash"), validator_set_hashes),
                (validator.get("stake_root"), stake_roots),
                (validator.get("registry_ref"), registry_refs),
                (validator.get("registry_sha256"), registry_digests),
                (validator.get("inventory_ref"), inventory_refs),
                (validator.get("inventory_sha256"), inventory_digests),
            ):
                if isinstance(value, str) and value.strip():
                    values.add(value.lower())
            if inventory_authority is not None:
                if validator.get("inventory_ref") != TRIAD_INVENTORY_RELATIVE:
                    gates.append("inventory_authority_ref_mismatch")
                actual_inventory_sha256 = validator.get("inventory_sha256")
                if (
                    not isinstance(actual_inventory_sha256, str)
                    or actual_inventory_sha256.lower() != inventory_sha256
                ):
                    gates.append("inventory_authority_digest_mismatch")
            if validator.get("registry_semantic_sha256") != expected_registry_semantic_sha256:
                gates.append("registry_semantic_identity_mismatch")
            registry_ref = validator.get("registry_ref")
            if not isinstance(registry_ref, str) or not (
                registry_ref == expected_registry_ref
                or registry_ref.replace("\\", "/").endswith("/" + expected_registry_ref)
            ):
                gates.append("registry_authority_ref_mismatch")
        world_id = status.get("world_id")
        if isinstance(world_id, str) and world_id.strip():
            world_ids.add(world_id)
            if world_id != expected_world_id:
                gates.append("world_identity_mismatch")
        network_tier = status.get("network_tier")
        if isinstance(network_tier, dict):
            tier = network_tier.get("tier")
            if isinstance(tier, str) and tier.strip():
                network_tiers.add(tier)
            network_id = network_tier.get("network_id")
            if isinstance(network_id, str) and network_id.strip():
                network_ids.add(network_id)
                if network_id != expected_world_id:
                    gates.append("network_identity_mismatch")
            target_validator_count = network_tier.get("target_validator_count")
            if (
                isinstance(target_validator_count, int)
                and not isinstance(target_validator_count, bool)
            ):
                validator_counts.add(target_validator_count)
            chain_id = network_tier.get("chain_id")
            if isinstance(chain_id, str) and chain_id.strip():
                chain_ids.add(chain_id)
                if chain_id != expected_chain_id:
                    gates.append("chain_identity_mismatch")
        world_resource = status.get("world_resource")
        if isinstance(world_resource, dict):
            chain_id = world_resource.get("chain_id")
            if isinstance(chain_id, str) and chain_id.strip():
                chain_ids.add(chain_id)
                if chain_id != expected_chain_id:
                    gates.append("chain_identity_mismatch")
            manifest_hash = world_resource.get("seed_manifest_hash")
            if isinstance(manifest_hash, str) and manifest_hash.strip():
                manifest_hashes.add(manifest_hash.lower())
    if len(validator_set_hashes) != 1:
        gates.append("validator_set_identity_mismatch")
    if len(stake_roots) != 1:
        gates.append("validator_stake_root_mismatch")
    if len(registry_refs) != 1 or len(registry_digests) != 1:
        gates.append("registry_identity_mismatch")
    if registry_digests != {expected_registry_sha256}:
        gates.append("registry_authority_digest_mismatch")
    if len(inventory_refs) != 1 or len(inventory_digests) != 1:
        gates.append("inventory_identity_mismatch")
    if len(world_ids) != 1:
        gates.append("world_identity_mismatch")
    if len(chain_ids) != 1:
        gates.append("chain_identity_mismatch")
    if len(manifest_hashes) != 1:
        gates.append("manifest_identity_mismatch")
    if network_tiers != {"public_testnet"} or len(network_ids) != 1 or validator_counts != {3}:
        gates.append("network_tier_identity_mismatch")
    if network_ids != {expected_world_id}:
        gates.append("network_identity_mismatch")
    if world_ids != {expected_world_id}:
        gates.append("world_identity_mismatch")
    if chain_ids != {expected_chain_id}:
        gates.append("chain_identity_mismatch")
    return gates


def triad_validator_set(captured: dict[str, dict[str, Any]]) -> dict[str, Any]:
    """Return the governed three-equal-validator projection in canonical order."""
    stakes = [
        captured[name].get("validator", {}).get("stake")
        if isinstance(captured[name].get("validator"), dict)
        else None
        for name in ("sequencer-204", "storage-205", "validator-47")
    ]
    return {
        "nodes": ["sequencer-204", "storage-205", "validator-47"],
        "stakes": stakes,
        "total_stake": MANAGED_TRIAD_TOTAL_STAKE,
        "required_stake": MANAGED_TRIAD_REQUIRED_STAKE,
        "quorum": MANAGED_TRIAD_QUORUM,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Collect bounded public-testnet fleet-health evidence.")
    parser.add_argument("--node", action="append", required=True, type=parse_node, metavar="NAME=URL")
    parser.add_argument("--sequencer", required=True, help="Name of the sequencer node supplied by --node.")
    parser.add_argument(
        "--managed-five-node",
        action="store_true",
        help=(
            "Require exactly the current managed public-testnet fleet: sequencer, storage, "
            "linux-lan-observer, windows-observer, and macos-observer."
        ),
    )
    parser.add_argument(
        "--managed-triad",
        action="store_true",
        help=(
            "Require exactly sequencer-204, storage-205, and validator-47 "
            "with three equal validators and validator-47 provider closure."
        ),
    )
    parser.add_argument("--max-capture-span-seconds", required=True, type=float)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if not math.isfinite(args.max_capture_span_seconds) or args.max_capture_span_seconds < 0:
        parser.error("--max-capture-span-seconds must be finite and non-negative")
    nodes = dict(args.node)
    if len(nodes) != len(args.node):
        if args.managed_triad:
            parser.error("managed triad closure requires each canonical node exactly once; duplicate --node name")
        if args.managed_five_node:
            parser.error("managed five-node closure requires each canonical node exactly once; duplicate --node name")
        parser.error("--node names must be unique")
    if args.sequencer not in nodes:
        parser.error("--sequencer must name one supplied --node")
    if args.managed_triad and args.managed_five_node:
        parser.error("managed triad and managed five-node closures are mutually exclusive")
    if args.managed_triad:
        supplied_names = frozenset(nodes)
        if supplied_names != MANAGED_TRIAD_NAMES:
            missing = sorted(MANAGED_TRIAD_NAMES - supplied_names)
            unknown = sorted(supplied_names - MANAGED_TRIAD_NAMES)
            details = []
            if missing:
                details.append("missing=" + ",".join(missing))
            if unknown:
                details.append("unknown=" + ",".join(unknown))
            parser.error(
                "managed triad closure requires exactly the canonical validator identities "
                "(" + "; ".join(details) + ")"
            )
        if args.sequencer != MANAGED_TRIAD_SEQUENCER:
            parser.error("managed triad closure requires --sequencer sequencer-204")
    elif frozenset(nodes) == MANAGED_TRIAD_NAMES:
        parser.error(
            "validator-aware --managed-triad is required for the canonical three-validator triad"
        )
    if args.managed_five_node:
        supplied_names = frozenset(nodes)
        if supplied_names != MANAGED_FIVE_NODE_NAMES:
            missing = sorted(MANAGED_FIVE_NODE_NAMES - supplied_names)
            unknown = sorted(supplied_names - MANAGED_FIVE_NODE_NAMES)
            details = []
            if missing:
                details.append("missing=" + ",".join(missing))
            if unknown:
                details.append("unknown=" + ",".join(unknown))
            parser.error(
                "managed five-node closure requires exactly the canonical node identities "
                "(" + "; ".join(details) + ")"
            )
        if args.sequencer != MANAGED_FIVE_NODE_SEQUENCER:
            parser.error("managed five-node closure requires --sequencer sequencer")

    started_at = utc_now()
    started_monotonic = time.monotonic()
    captured: dict[str, dict[str, Any]] = {}
    failed_gates: list[str] = []
    inventory_authority = triad_inventory_authority() if args.managed_triad else None
    if args.managed_triad and inventory_authority is None:
        failed_gates.append("inventory_authority_unavailable")
    for name, url in nodes.items():
        node_evidence: dict[str, Any] = {"url": url, "captured_at": utc_now()}
        try:
            node_evidence.update(read_json(url))
        except (OSError, URLError, ValueError, json.JSONDecodeError) as error:
            node_evidence["collection_error"] = str(error)
            failed_gates.append("collection_failed")
        captured[name] = node_evidence

    capture_span_seconds = time.monotonic() - started_monotonic
    finished_at = utc_now()
    if capture_span_seconds > args.max_capture_span_seconds:
        failed_gates.append("capture_span_exceeded")

    sequencer_consensus = captured[args.sequencer].get("consensus", {})
    if not isinstance(sequencer_consensus, dict):
        sequencer_consensus = {}
    for name, node_evidence in captured.items():
        if "collection_error" not in node_evidence:
            failed_gates.extend(node_gates(node_evidence, sequencer_consensus))
        elif name == args.sequencer:
            failed_gates.extend(["head_mismatch", "network_head_not_ready"])
    if args.managed_triad:
        for name in MANAGED_TRIAD_NAMES:
            node_evidence = captured[name]
            if "collection_error" not in node_evidence:
                failed_gates.extend(triad_validator_gates(name, node_evidence, inventory_authority))
        validator_47 = captured["validator-47"]
        if "collection_error" not in validator_47:
            failed_gates.extend(triad_provider_gates(validator_47, inventory_authority))
        if all("collection_error" not in captured[name] for name in MANAGED_TRIAD_NAMES):
            failed_gates.extend(triad_projection_consistency_gates(captured, inventory_authority))
    if args.managed_five_node:
        failed_gates.extend(provider_checkpoint_gates(captured))

    unique_gates = sorted(set(failed_gates))
    evidence = {
        "capture_finished_at": finished_at,
        "capture_span_seconds": capture_span_seconds,
        "capture_started_at": started_at,
        "failed_gates": unique_gates,
        "max_capture_span_seconds": args.max_capture_span_seconds,
        "nodes": captured,
        "sequencer": args.sequencer,
        "scope": (
            "managed_triad"
            if args.managed_triad
            else "managed_five_node"
            if args.managed_five_node
            else "generic"
        ),
        "verdict": "ready" if not unique_gates else "blocked",
    }
    if args.managed_triad:
        evidence["claim_mode"] = "three_equal_validator"
        evidence["validator_set"] = triad_validator_set(captured)
        evidence["governance"] = MANAGED_TRIAD_GOVERNANCE
        evidence["status_projection"] = {
            "schema_version": TRIAD_STATUS_PROJECTION_SCHEMA,
            "mode": "runtime_status_projection",
            "collector_policy": "node_emitted_only_fail_closed",
        }
    write_evidence(args.output, evidence)
    if unique_gates:
        print("fleet_health_blocked=" + ",".join(unique_gates), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
