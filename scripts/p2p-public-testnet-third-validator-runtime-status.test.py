#!/usr/bin/env python3
"""PG-02R-A contract for production-compatible triad readiness.

The non-projection portion of every response in this test is loaded from the
repository's serialized ``ChainStatusResponse`` sample.  The sample is not
replaced with the synthetic ``validator``/``provider`` objects used by the
original PG-01 test.  Instead, this file freezes the versioned projection that
the runtime must emit before a triad claim can be ready.

The collector must accept only a node-emitted projection. Historical
ChainStatusResponse samples without that projection must remain blocked; this
prevents a collector-side adapter from fabricating membership or provider
readiness.
"""

from __future__ import annotations

import copy
import hashlib
import http.server
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
COLLECTOR = ROOT / "scripts" / "p2p-public-testnet-fleet-health.py"
STATUS_SAMPLE = (
    ROOT
    / "doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05/chain-status-sample.json"
)
INVENTORY = ROOT / "scripts/public-testnet-validator-triad-inventory.v1.json"
REGISTRY = ROOT / "doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"

PROJECTION_SCHEMA = "oasis7.chain_validator_provider_status.v1"
WORLD_ID = "oasis7-public-testnet-governed-20260606"
NODE_IDS = {
    "sequencer-204": "triad-testnet-sequencer",
    "storage-205": "triad-testnet-storage",
    "validator-47": "triad-testnet-validator-47",
}
NODE_ROLES = {
    "sequencer-204": "sequencer",
    "storage-205": "storage",
    # The runtime role remains the current serialized role; validator
    # membership belongs to the versioned validator projection below.
    "validator-47": "storage",
}
SIGNERS = {
    "triad-testnet-sequencer": "e01e5c34dee2da3087653bc4cec02be01632f56250a800994c96ea44ae6f3690",
    "triad-testnet-storage": "1f530cae002d7adb9a6c3dd8f4bc861226f112f88fdd252b28b6494019e21c33",
    "triad-testnet-validator-47": "cf8c9c2b5637d20d0efa585f0fb7f503b19a1aaba02fb807637e67ed40919fc2",
}
TRIAD_STAKES = {node_id: 100 for node_id in NODE_IDS.values()}
VALIDATOR_47_PEER_ID = "12D3KooWCdQLY6Qm9sWqPqEhJTmPdY3Ykw1w5QnTh7qmSgYDazQZ"
TRIAD_REGISTRY_SEMANTIC_DIGEST = hashlib.sha256(
    json.dumps(
        {
            "signer_bindings": {
                f"governance.finality.v1.{node_id}": SIGNERS[node_id].lower()
                for node_id in sorted(SIGNERS)
            },
            "slot_id": "governance.finality.v1",
            "threshold": 2,
            "threshold_bps": 0,
            "validator_stakes": {
                f"governance.finality.v1.{node_id}": 100 for node_id in sorted(SIGNERS)
            },
        },
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
).hexdigest()

CURRENT_STATUS_REQUIRED_TOP_LEVEL_KEYS = {
    "ok",
    "observed_at_unix_ms",
    "node_id",
    "world_id",
    "role",
    "running",
    "liveness",
    "readiness",
    "sync",
    "worker_poll_count",
    "tick_count",
    "last_tick_unix_ms",
    "consensus",
    "chain_proof",
    "last_error",
    "execution_world_dir",
    "network_tier",
    "world_resource",
    "p2p",
    "observability",
    "release_security_policy",
    "reward_runtime",
    "storage",
    "wasm",
    "traffic",
    "transactions",
    "replication",
}
CURRENT_STATUS_OPTIONAL_TOP_LEVEL_KEYS = {
    # These fields were added after the checked-in 2026-07 serialization
    # sample.  Their absence in that historical sample must not turn this
    # behavior RED into a fixture/setup failure.
    "runtime_perf",
    "execution_bridge_commit_timing",
    "module_tick_routing",
}
CURRENT_STATUS_CONSENSUS_KEYS = {
    "slot",
    "epoch",
    "ticks_per_slot",
    "tick_phase",
    "proposal_tick_phase",
    "last_observed_slot",
    "missed_slot_count",
    "last_observed_tick",
    "missed_tick_count",
    "adaptive_tick_scheduler_enabled",
    "latest_height",
    "committed_height",
    "last_committed_at_ms",
    "last_commit_age_ms",
    "network_committed_height",
    "replication_enabled",
    "replication_persisted_height",
    "replication_gap_sync_blocked_height",
    "replication_gap_sync_blocked_reason",
    "replication_gap_sync_repair_attempt_height",
    "replication_gap_sync_repair_attempt_summary",
    "storage_challenge_network_degraded_height",
    "storage_challenge_network_degraded_reason",
    "state_sync_fallback_required",
    "state_sync_snapshot_available",
    "state_sync_trusted_checkpoint_required_height",
    "state_sync_fallback_reason",
    "consensus_participation_held",
    "consensus_participation_hold_reason",
    "recent_finality_latency",
    "pending_proposal",
    "pending_consensus_actions",
    "inbound_timing_rejections",
    "last_status",
    "last_block_hash",
    "last_execution_height",
    "last_execution_block_hash",
    "last_execution_state_root",
    "known_peer_heads",
    "validator_set_hash",
    "validator_stake_root",
    "validator_stake_proof_count",
    "misbehavior_evidence_count",
    "slashing_intent_count",
    "pending_slashing_intent_count",
    "slashing_receipt_count",
    "applied_slashing_receipt_count",
    "quarantined_validator_count",
    "slashable_stake_total",
    "network_head",
}


def canonical_digest(value: Any) -> str:
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected JSON object in {path}")
    return value


def current_status_sample() -> dict[str, Any]:
    """Load and structurally pin the real serialized status fixture."""
    value = load_json(STATUS_SAMPLE)
    actual_required_keys = set(value) - CURRENT_STATUS_OPTIONAL_TOP_LEVEL_KEYS
    if actual_required_keys != CURRENT_STATUS_REQUIRED_TOP_LEVEL_KEYS:
        raise AssertionError(
            "ChainStatusResponse fixture drifted; update this contract only with a reviewed schema change: "
            f"missing={sorted(CURRENT_STATUS_REQUIRED_TOP_LEVEL_KEYS - actual_required_keys)} "
            f"extra={sorted(actual_required_keys - CURRENT_STATUS_REQUIRED_TOP_LEVEL_KEYS)}"
        )
    if set(value["consensus"]) != CURRENT_STATUS_CONSENSUS_KEYS:
        raise AssertionError("serialized consensus shape drifted; this test must track the runtime contract")
    if value["world_id"] != WORLD_ID:
        raise AssertionError("repository status fixture is not the governed public-testnet sample")
    return value


def triad_registry() -> dict[str, Any]:
    """Build the candidate three-validator registry from stable signer fixtures."""
    value = copy.deepcopy(load_json(REGISTRY))
    validators = value.get("validators")
    if not isinstance(validators, list):
        raise AssertionError("validator registry fixture has no validators")
    validators.append(
        {
            "node_id": NODE_IDS["validator-47"],
            "scheme": "ed25519",
            "finality_signer_public_key": SIGNERS[NODE_IDS["validator-47"]],
            "stake": 100,
        }
    )
    value["quorum"] = {
        "numerator": 2,
        "denominator": 3,
        "total_stake": 300,
        "required_stake": 200,
    }
    value["governance"] = {"signer_count": 3, "threshold": 2, "threshold_bps": 6667}
    return value


# ``registry_sha256`` is the exact-byte digest of the generated deployment
# registry bound by the immutable inventory.  The source registry is retained
# as a separate build-time input, while the runtime also emits the canonical
# effective-registry semantic digest below.
TRIAD_REGISTRY_DIGEST = "8bfb4411f3895ab5f1a2a3de1bcaa08ce97567202d4198444b323ef437a88f78"
INVENTORY_DIGEST = hashlib.sha256(INVENTORY.read_bytes()).hexdigest()


def projected_status(node_name: str) -> dict[str, Any]:
    """Return current serialized status plus the required runtime projection."""
    status = current_status_sample()
    # The checked-in sample is a real sequencer capture.  Adapt only the
    # serialized node identity fields so the same production shape can stand
    # in for each endpoint in this bounded collector contract.
    status["node_id"] = NODE_IDS[node_name]
    status["role"] = NODE_ROLES[node_name]
    status["p2p"] = copy.deepcopy(status["p2p"])
    status["p2p"]["node_role_claim"] = (
        "validator_core" if node_name == "sequencer-204" else "full_storage"
    )
    status["network_tier"] = copy.deepcopy(status["network_tier"])
    status["network_tier"]["target_validator_count"] = 3
    status["network_tier"]["network_id"] = WORLD_ID
    status["network_tier"]["chain_id"] = WORLD_ID
    status["world_resource"] = copy.deepcopy(status["world_resource"])
    status["world_resource"]["world_id"] = WORLD_ID
    status["world_resource"]["chain_id"] = WORLD_ID
    status["chain_proof"] = copy.deepcopy(status["chain_proof"])
    status["chain_proof"]["latest_world_head_proof"] = copy.deepcopy(
        status["chain_proof"]["latest_world_head_proof"]
    )
    status["chain_proof"]["latest_world_head_proof"]["world_id"] = WORLD_ID
    status["consensus"] = copy.deepcopy(status["consensus"])
    status["consensus"]["validator_set_hash"] = "runtime-validator-set-hash"
    status["consensus"]["validator_stake_root"] = canonical_digest(
        [(node_id, TRIAD_STAKES[node_id], SIGNERS[node_id]) for node_id in sorted(TRIAD_STAKES)]
    )
    status["consensus"]["validator_stake_proof_count"] = 3

    node_id = NODE_IDS[node_name]
    world_resource = status["world_resource"]
    proof = status["chain_proof"]["latest_world_head_proof"]
    status["chain_proof"]["latest_execution_checkpoint"] = {
        "schema_version": 2,
        "checkpoint_id": proof["world_head_proof_ref"],
        "height": proof["height"],
        "manifest_hash": world_resource["seed_manifest_hash"],
    }
    status["validator"] = {
        "schema_version": PROJECTION_SCHEMA,
        "role": "validator",
        "membership": "active",
        "stake": TRIAD_STAKES[node_id],
        "signer_binding": node_id,
        "signer_public_key_hex": SIGNERS[node_id],
        "stake_proof": {
            "validator_id": node_id,
            "player_id": node_id,
            "stake": TRIAD_STAKES[node_id],
            "signer_public_key_hex": SIGNERS[node_id],
            "leaf_hash": f"leaf-{node_id}",
            "proof": [],
        },
        "validator_set_hash": status["consensus"]["validator_set_hash"],
        "stake_root": status["consensus"]["validator_stake_root"],
        "registry_ref": "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
        "registry_sha256": TRIAD_REGISTRY_DIGEST,
        "registry_semantic_sha256": TRIAD_REGISTRY_SEMANTIC_DIGEST,
        "inventory_ref": str(INVENTORY.relative_to(ROOT)),
        "inventory_sha256": INVENTORY_DIGEST,
    }
    checkpoint = {
        "schema_version": 2,
        "checkpoint_id": proof["world_head_proof_ref"],
        "height": proof["height"],
        "manifest_hash": world_resource["seed_manifest_hash"],
        "proof_hash": proof["proof_hash"],
        "world_id": WORLD_ID,
        "chain_id": WORLD_ID,
    }
    is_validator_47 = node_name == "validator-47"
    status["provider"] = {
        "schema_version": PROJECTION_SCHEMA,
        "node_id": node_id,
        "provider_id": VALIDATOR_47_PEER_ID if is_validator_47 else f"peer-{node_id}",
        "checkpoint": is_validator_47,
        "full_storage": is_validator_47,
        "checkpoint_proof": checkpoint if is_validator_47 else None,
        "full_storage_proof": (
            {
                "status": "ready",
                "provider_id": VALIDATOR_47_PEER_ID,
                "world_id": WORLD_ID,
                "chain_id": WORLD_ID,
                "manifest_hash": world_resource["seed_manifest_hash"],
                "height": proof["height"],
            }
            if is_validator_47
            else None
        ),
    }
    return status


class JsonFixtureServer:
    def __init__(self, responses: dict[str, dict[str, Any]]) -> None:
        self.responses = responses

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802 - stdlib callback name
                payload = self.server.responses.get(self.path)  # type: ignore[attr-defined]
                if payload is None:
                    self.send_response(404)
                    self.end_headers()
                    return
                encoded = json.dumps(payload, sort_keys=True).encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(encoded)))
                self.end_headers()
                self.wfile.write(encoded)

            def log_message(self, _format: str, *_args: object) -> None:
                pass

        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.responses = responses  # type: ignore[attr-defined]
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def __enter__(self) -> "JsonFixtureServer":
        self.thread.start()
        return self

    def __exit__(self, *_args: object) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def endpoint(self, name: str) -> str:
        host, port = self.server.server_address
        return f"http://{host}:{port}/{name}"


class ThirdValidatorRuntimeStatusContractTest(unittest.TestCase):
    def run_health(
        self,
        fixture: JsonFixtureServer,
        output: Path,
    ) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(COLLECTOR), "--sequencer", "sequencer-204"]
        for name in NODE_IDS:
            command.extend(["--node", f"{name}={fixture.endpoint(name)}"])
        command.extend(
            ["--managed-triad", "--max-capture-span-seconds", "1", "--output", str(output)]
        )
        return subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)

    def test_current_chain_status_shape_without_projection_stays_blocked(self) -> None:
        """A historical status must never be upgraded by the collector."""
        statuses = {}
        for name in NODE_IDS:
            status = current_status_sample()
            status["node_id"] = NODE_IDS[name]
            status["role"] = NODE_ROLES[name]
            statuses[name] = status
        with tempfile.TemporaryDirectory(prefix="oasis7-pg02r-a-current-status-") as temp_dir, JsonFixtureServer(
            {f"/{name}": statuses[name] for name in NODE_IDS}
        ) as fixture:
            output = Path(temp_dir) / "triad-health.json"
            result = self.run_health(fixture, output)
            self.assertNotEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "blocked")
            self.assertEqual(evidence["scope"], "managed_triad")
            self.assertEqual(evidence["status_projection"]["schema_version"], PROJECTION_SCHEMA)
            self.assertIn("validator_metadata_missing", evidence["failed_gates"])
            self.assertIn("validator_47_provider_metadata_missing", evidence["failed_gates"])

    def test_versioned_projection_is_accepted_only_when_node_emits_all_bindings(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        for name, status in statuses.items():
            self.assertEqual(
                set(status) - {"validator", "provider"} - CURRENT_STATUS_OPTIONAL_TOP_LEVEL_KEYS,
                CURRENT_STATUS_REQUIRED_TOP_LEVEL_KEYS,
            )
            self.assertEqual(status["world_id"], WORLD_ID)
            self.assertEqual(status["network_tier"]["network_id"], WORLD_ID)
            self.assertEqual(status["network_tier"]["chain_id"], WORLD_ID)
            self.assertEqual(status["network_tier"]["tier"], "public_testnet")
            self.assertEqual(status["world_resource"]["chain_id"], WORLD_ID)
            self.assertEqual(status["chain_proof"]["latest_world_head_proof"]["world_id"], WORLD_ID)
            self.assertEqual(status["validator"]["membership"], "active")
            self.assertEqual(status["validator"]["stake"], 100)
            self.assertEqual(status["validator"]["signer_binding"], NODE_IDS[name])
            self.assertEqual(status["validator"]["signer_public_key_hex"], SIGNERS[NODE_IDS[name]])
            self.assertEqual(status["validator"]["registry_sha256"], TRIAD_REGISTRY_DIGEST)
            self.assertEqual(status["validator"]["inventory_sha256"], INVENTORY_DIGEST)
            self.assertEqual(status["provider"]["schema_version"], PROJECTION_SCHEMA)
            self.assertEqual(status["provider"]["node_id"], NODE_IDS[name])
            expected_provider_id = (
                VALIDATOR_47_PEER_ID if name == "validator-47" else f"peer-{NODE_IDS[name]}"
            )
            self.assertEqual(status["provider"]["provider_id"], expected_provider_id)
            if name == "validator-47":
                self.assertTrue(status["provider"]["checkpoint"])
                self.assertTrue(status["provider"]["full_storage"])
                checkpoint = status["provider"]["checkpoint_proof"]
                self.assertEqual(checkpoint["schema_version"], 2)
                self.assertEqual(checkpoint["world_id"], WORLD_ID)
                self.assertEqual(checkpoint["chain_id"], WORLD_ID)
                self.assertEqual(checkpoint["manifest_hash"], status["world_resource"]["seed_manifest_hash"])
                self.assertEqual(
                    status["provider"]["full_storage_proof"]["manifest_hash"],
                    status["world_resource"]["seed_manifest_hash"],
                )
            else:
                self.assertFalse(status["provider"]["checkpoint"])
                self.assertFalse(status["provider"]["full_storage"])
                self.assertIsNone(status["provider"]["checkpoint_proof"])
                self.assertIsNone(status["provider"]["full_storage_proof"])

        with tempfile.TemporaryDirectory(prefix="oasis7-pg02r-a-projection-") as temp_dir, JsonFixtureServer(
            {f"/{name}": statuses[name] for name in NODE_IDS}
        ) as fixture:
            output = Path(temp_dir) / "triad-health.json"
            result = self.run_health(fixture, output)
            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "ready")
            self.assertEqual(
                evidence["status_projection"]["collector_policy"],
                "node_emitted_only_fail_closed",
            )

    def assert_identity_rejected(
        self,
        statuses: dict[str, dict[str, Any]],
        expected_gate: str,
    ) -> None:
        with tempfile.TemporaryDirectory(prefix="oasis7-pg02r-a-negative-") as temp_dir, JsonFixtureServer(
            {f"/{name}": statuses[name] for name in NODE_IDS}
        ) as fixture:
            output = Path(temp_dir) / "triad-health.json"
            result = self.run_health(fixture, output)
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn(expected_gate, result.stderr)
            self.assertTrue(output.exists(), "identity failures must emit diagnostic blocked evidence")
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertIn(expected_gate, evidence["failed_gates"])
            self.assertNotEqual(evidence.get("verdict"), "ready")

    def test_wrong_world_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["validator-47"]["world_id"] = "oasis7-wrong-world"
        self.assert_identity_rejected(statuses, "world_identity_mismatch")

    def test_self_consistent_wrong_node_identity_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        status = statuses["validator-47"]
        attacker_id = "attacker-validator-47"
        status["node_id"] = attacker_id
        status["validator"]["signer_binding"] = attacker_id
        status["validator"]["stake_proof"]["validator_id"] = attacker_id
        status["provider"]["node_id"] = attacker_id
        self.assert_identity_rejected(statuses, "validator_identity_mismatch")

    def test_self_consistent_wrong_stake_root_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        status = statuses["validator-47"]
        wrong_root = "0" * 64
        status["consensus"]["validator_stake_root"] = wrong_root
        status["validator"]["stake_root"] = wrong_root
        self.assert_identity_rejected(statuses, "validator_stake_root_mismatch")

    def test_self_consistent_wrong_finality_signer_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        status = statuses["validator-47"]
        wrong_signer = "11" * 32
        status["validator"]["signer_public_key_hex"] = wrong_signer
        status["validator"]["stake_proof"]["signer_public_key_hex"] = wrong_signer
        self.assert_identity_rejected(statuses, "signer_identity_mismatch")

    def test_self_consistent_wrong_peer_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        status = statuses["validator-47"]
        wrong_peer = "12D3KooWAttackerPeer"
        status["provider"]["provider_id"] = wrong_peer
        status["provider"]["full_storage_proof"]["provider_id"] = wrong_peer
        self.assert_identity_rejected(statuses, "provider_identity_mismatch")

    def test_all_consistent_wrong_world_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        for status in statuses.values():
            status["world_id"] = "oasis7-unapproved-world"
            status["network_tier"]["network_id"] = "oasis7-unapproved-world"
            status["network_tier"]["chain_id"] = "oasis7-unapproved-world"
            status["world_resource"]["world_id"] = "oasis7-unapproved-world"
            status["world_resource"]["chain_id"] = "oasis7-unapproved-world"
            status["chain_proof"]["latest_world_head_proof"]["world_id"] = "oasis7-unapproved-world"
            if status["provider"]["checkpoint_proof"] is not None:
                status["provider"]["checkpoint_proof"]["world_id"] = "oasis7-unapproved-world"
                status["provider"]["full_storage_proof"]["world_id"] = "oasis7-unapproved-world"
        self.assert_identity_rejected(statuses, "world_identity_mismatch")

    def test_wrong_chain_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["storage-205"]["network_tier"]["chain_id"] = "oasis7-wrong-chain"
        self.assert_identity_rejected(statuses, "chain_identity_mismatch")

    def test_all_consistent_wrong_chain_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        wrong_chain = "oasis7-unapproved-chain"
        for status in statuses.values():
            status["network_tier"]["chain_id"] = wrong_chain
            status["world_resource"]["chain_id"] = wrong_chain
            if status["provider"]["checkpoint_proof"] is not None:
                status["provider"]["checkpoint_proof"]["chain_id"] = wrong_chain
                status["provider"]["full_storage_proof"]["chain_id"] = wrong_chain
        self.assert_identity_rejected(statuses, "chain_identity_mismatch")

    def test_wrong_manifest_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["storage-205"]["world_resource"]["seed_manifest_hash"] = "0" * 64
        self.assert_identity_rejected(statuses, "manifest_identity_mismatch")

    def test_wrong_registry_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["sequencer-204"]["validator"]["registry_sha256"] = "0" * 64
        self.assert_identity_rejected(statuses, "registry_identity_mismatch")

    def test_all_consistent_wrong_registry_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        for status in statuses.values():
            status["validator"]["registry_semantic_sha256"] = "0" * 64
        self.assert_identity_rejected(statuses, "registry_semantic_identity_mismatch")

    def test_all_consistent_wrong_registry_digest_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        for status in statuses.values():
            status["validator"]["registry_sha256"] = "0" * 64
        self.assert_identity_rejected(statuses, "registry_authority_digest_mismatch")

    def test_wrong_inventory_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["sequencer-204"]["validator"]["inventory_sha256"] = "0" * 64
        self.assert_identity_rejected(statuses, "inventory_identity_mismatch")

    def test_wrong_signer_binding_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["storage-205"]["validator"]["signer_public_key_hex"] = "0" * 64
        self.assert_identity_rejected(statuses, "signer_identity_mismatch")

    def test_wrong_provider_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["validator-47"]["provider"]["provider_id"] = "triad-testnet-storage"
        self.assert_identity_rejected(statuses, "provider_identity_mismatch")

    def test_validator_provider_uses_storage_full_storage_role_pair(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["validator-47"]["p2p"]["node_role_claim"] = "validator_core"
        self.assert_identity_rejected(statuses, "p2p_role_invalid")

    def test_wrong_checkpoint_proof_is_not_triad_ready(self) -> None:
        statuses = {name: projected_status(name) for name in NODE_IDS}
        statuses["validator-47"]["provider"]["checkpoint_proof"]["proof_hash"] = "0" * 64
        self.assert_identity_rejected(statuses, "provider_checkpoint_proof_mismatch")


if __name__ == "__main__":
    unittest.main()
