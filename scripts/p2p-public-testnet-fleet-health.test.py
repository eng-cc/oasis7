#!/usr/bin/env python3
"""RED contract for the cross-platform public-testnet fleet-health collector."""

from __future__ import annotations

import json
import hashlib
import importlib.util
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from unittest.mock import patch


ROOT_DIR = Path(__file__).resolve().parent.parent
COLLECTOR = ROOT_DIR / "scripts" / "p2p-public-testnet-fleet-health.py"
RUNBOOK = ROOT_DIR / "doc" / "p2p" / "blockchain" / "public-testnet-governed-bootstrap.runbook.md"
INVENTORY = ROOT_DIR / "doc" / "testing" / "evidence" / "public-testnet-five-node-inventory-2026-06-23.md"
NO_CHECKPOINT = object()


def load_collector_module() -> Any:
    spec = importlib.util.spec_from_file_location("p2p_public_testnet_fleet_health", COLLECTOR)
    if spec is None or spec.loader is None:
        raise AssertionError("fleet-health collector module could not be loaded")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def status(
    *,
    head: int = 42,
    ready: bool = True,
    decision: str = "ready",
    checkpoint: object = NO_CHECKPOINT,
) -> dict[str, Any]:
    result = {
        "running": True,
        "last_error": None,
        "readiness": {"status": "ready" if ready else "not_ready", "failed_gates": []},
        "consensus": {
            "committed_height": head,
            "network_committed_height": head,
            "last_execution_height": head,
            "network_head": {"decision": decision},
        },
    }
    if checkpoint is not NO_CHECKPOINT:
        result["chain_proof"] = {"latest_execution_checkpoint": checkpoint}
    return result


def checkpoint(*, height: int = 42, checkpoint_id: str = "checkpoint-v2", manifest_hash: str = "a" * 64) -> dict[str, Any]:
    return {
        "schema_version": 2,
        "checkpoint_id": checkpoint_id,
        "height": height,
        "manifest_hash": manifest_hash,
    }


class FleetHealthFixture:
    def __init__(self, responses: dict[str, tuple[dict[str, Any], float]]) -> None:
        self.responses = responses
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), self._handler())
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def _handler(self) -> type[BaseHTTPRequestHandler]:
        responses = self.responses

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802
                payload, delay = responses.get(self.path, ({"error": "missing fixture"}, 0.0))
                if delay:
                    time.sleep(delay)
                encoded = json.dumps(payload).encode("utf-8")
                self.send_response(200 if self.path in responses else 404)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(encoded)))
                self.end_headers()
                self.wfile.write(encoded)

            def log_message(self, _format: str, *_args: object) -> None:
                pass

        return Handler

    def __enter__(self) -> "FleetHealthFixture":
        self.thread.start()
        return self

    def __exit__(self, *_args: object) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def endpoint(self, name: str) -> str:
        host, port = self.server.server_address
        return f"http://{host}:{port}/{name}"


def managed_triad_status(module: Any, name: str, *, inventory_sha256: object) -> dict[str, Any]:
    """Build a complete node-emitted projection for the managed triad path."""
    authority = module.triad_inventory_authority()
    if authority is None:
        raise AssertionError("the checked-in triad authority must load for this fixture")
    inventory, _inventory_digest = authority
    node = inventory["nodes"][name]
    node_id = node["node_id"]
    world_id = inventory["authority"]["world_id"]
    registry_ref = inventory["authority"]["registry_ref"]
    registry_digest = inventory["authority"]["generated_registry_sha256"]
    registry_semantic_digest = inventory["authority"]["generated_registry_semantic_sha256"]
    signer = node["finality_signer_public_key"]
    validator_set_hash = "fixture-validator-set"
    stake_root = "fixture-stake-root"
    manifest_hash = "f" * 64
    checkpoint_id = "fixture-checkpoint-42"
    proof_hash = "fixture-proof-42"
    is_validator_47 = name == "validator-47"
    provider_id = node.get("libp2p_peer_id", f"peer-{node_id}")
    return {
        "node_id": node_id,
        "world_id": world_id,
        "role": "sequencer" if name == "sequencer-204" else "storage",
        "running": True,
        "last_error": None,
        "readiness": {"status": "ready", "failed_gates": []},
        "consensus": {
            "committed_height": 42,
            "network_committed_height": 42,
            "last_execution_height": 42,
            "network_head": {"decision": "ready"},
            "validator_set_hash": validator_set_hash,
            "validator_stake_root": stake_root,
        },
        "network_tier": {
            "tier": "public_testnet",
            "network_id": world_id,
            "chain_id": world_id,
            "target_validator_count": 3,
        },
        "world_resource": {
            "world_id": world_id,
            "chain_id": world_id,
            "seed_manifest_hash": manifest_hash,
        },
        "p2p": {
            "node_role_claim": "validator_core" if name == "sequencer-204" else "full_storage"
        },
        "chain_proof": {
            "latest_execution_checkpoint": {
                "schema_version": 2,
                "checkpoint_id": checkpoint_id,
                "height": 42,
                "manifest_hash": manifest_hash,
            },
            "latest_world_head_proof": {
                "checkpoint_ref": checkpoint_id,
                "height": 42,
                "proof_hash": proof_hash,
                "world_id": world_id,
            },
        },
        "validator": {
            "schema_version": module.TRIAD_STATUS_PROJECTION_SCHEMA,
            "role": "validator",
            "membership": "active",
            "stake": 100,
            "signer_binding": node_id,
            "signer_public_key_hex": signer,
            "stake_proof": {
                "validator_id": node_id,
                "player_id": node_id,
                "stake": 100,
                "signer_public_key_hex": signer,
                "leaf_hash": f"leaf-{node_id}",
                "proof": [],
            },
            "validator_set_hash": validator_set_hash,
            "stake_root": stake_root,
            "registry_ref": registry_ref,
            "registry_sha256": registry_digest,
            "registry_semantic_sha256": registry_semantic_digest,
            "inventory_ref": module.TRIAD_INVENTORY_RELATIVE,
            "inventory_sha256": inventory_sha256,
        },
        "provider": {
            "schema_version": module.TRIAD_STATUS_PROJECTION_SCHEMA,
            "node_id": node_id,
            "provider_id": provider_id,
            "checkpoint": is_validator_47,
            "full_storage": is_validator_47,
            "checkpoint_proof": (
                {
                    "schema_version": 2,
                    "checkpoint_id": checkpoint_id,
                    "height": 42,
                    "manifest_hash": manifest_hash,
                    "proof_hash": proof_hash,
                    "world_id": world_id,
                    "chain_id": world_id,
                }
                if is_validator_47
                else None
            ),
            "full_storage_proof": (
                {
                    "status": "ready",
                    "provider_id": provider_id,
                    "world_id": world_id,
                    "chain_id": world_id,
                    "manifest_hash": manifest_hash,
                    "height": 42,
                }
                if is_validator_47
                else None
            ),
        },
    }


class FleetHealthCollectorContractTest(unittest.TestCase):
    def test_triad_authority_rejects_self_consistent_noncanonical_source_or_peer_refs(self) -> None:
        module = load_collector_module()
        inventory = json.loads(
            (ROOT_DIR / "scripts" / "public-testnet-validator-triad-inventory.v1.json").read_text(
                encoding="utf-8"
            )
        )
        cases = {
            "source_registry": (
                "source_registry_ref",
                "source_registry_sha256",
                module.TRIAD_BOOTSTRAP_PEER_RELATIVE,
                module.TRIAD_BOOTSTRAP_PEER_SHA256,
            ),
            "bootstrap_peer": (
                "bootstrap_peer_ref",
                "bootstrap_peer_sha256",
                module.TRIAD_SOURCE_REGISTRY_RELATIVE,
                module.TRIAD_SOURCE_REGISTRY_SHA256,
            ),
        }
        for case, (ref_key, digest_key, ref, digest) in cases.items():
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temp_dir:
                mutated = json.loads(json.dumps(inventory))
                mutated["authority"][ref_key] = ref
                mutated["authority"][digest_key] = digest
                staged_inventory = Path(temp_dir) / "inventory.json"
                staged_bytes = (json.dumps(mutated, indent=2) + "\n").encode("utf-8")
                staged_inventory.write_bytes(staged_bytes)
                with (
                    patch.object(module, "TRIAD_INVENTORY_PATH", staged_inventory),
                    patch.object(module, "TRIAD_INVENTORY_SHA256", hashlib.sha256(staged_bytes).hexdigest()),
                ):
                    self.assertIsNone(module.triad_inventory_authority())

    def run_collector(
        self,
        fixture: FleetHealthFixture,
        output: Path,
        *,
        max_span_seconds: float | str = 1.0,
        nodes: list[tuple[str, str]] | None = None,
        managed_triad: bool = False,
        managed_five_node: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        nodes = nodes or [
            ("sequencer", fixture.endpoint("sequencer")),
            ("storage", fixture.endpoint("storage")),
            ("observer", fixture.endpoint("observer")),
        ]
        command = [
            sys.executable,
            str(COLLECTOR),
            "--sequencer",
            "sequencer-204" if managed_triad else "sequencer",
        ]
        for name, endpoint in nodes:
            command.extend(["--node", f"{name}={endpoint}"])
        if managed_triad:
            command.append("--managed-triad")
        if managed_five_node:
            command.append("--managed-five-node")
        command.extend([
            "--max-capture-span-seconds",
            str(max_span_seconds),
            "--output",
            str(output),
        ])
        return subprocess.run(
            command,
            cwd=ROOT_DIR,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_managed_triad_non_string_inventory_digest_writes_bounded_blocked_evidence(self) -> None:
        module = load_collector_module()
        node_names = ("sequencer-204", "storage-205", "validator-47")
        inventory_authority = module.triad_inventory_authority()
        self.assertIsNotNone(inventory_authority)
        _inventory, inventory_digest = inventory_authority

        for malformed_digest in (None, 17, []):
            with self.subTest(malformed_digest=malformed_digest), tempfile.TemporaryDirectory() as temp_dir:
                statuses = {
                    name: managed_triad_status(
                        module,
                        name,
                        inventory_sha256=malformed_digest if name == "sequencer-204" else inventory_digest,
                    )
                    for name in node_names
                }
                responses = {f"/{name}": (statuses[name], 0.0) for name in node_names}
                with FleetHealthFixture(responses) as fixture:
                    output = Path(temp_dir) / "triad-health.json"
                    result = self.run_collector(
                        fixture,
                        output,
                        nodes=[(name, fixture.endpoint(name)) for name in node_names],
                        managed_triad=True,
                    )

                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertNotIn("Traceback", result.stderr)
                self.assertTrue(output.exists(), "malformed node status must still emit blocked evidence")
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertEqual(evidence["scope"], "managed_triad")
                self.assertEqual(evidence["verdict"], "blocked")
                self.assertIn("inventory_authority_digest_mismatch", evidence["failed_gates"])

    def test_ready_fleet_writes_timestamped_json_evidence(self) -> None:
        responses = {f"/{name}": (status(), 0.0) for name in ("sequencer", "storage", "observer")}
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            result = self.run_collector(fixture, output)

            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "ready")
            self.assertEqual(evidence["max_capture_span_seconds"], 1.0)
            self.assertIn("capture_started_at", evidence)
            self.assertIn("capture_finished_at", evidence)
            self.assertEqual(evidence["sequencer"], "sequencer")
            self.assertEqual(set(evidence["nodes"]), {"sequencer", "storage", "observer"})
            for node in evidence["nodes"].values():
                self.assertIn("captured_at", node)
                self.assertEqual(node["consensus"]["committed_height"], 42)

    def test_over_span_fails_closed_and_writes_failure_evidence(self) -> None:
        responses = {
            "/sequencer": (status(), 0.0),
            "/storage": (status(), 0.08),
            "/observer": (status(), 0.0),
        }
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            result = self.run_collector(fixture, output, max_span_seconds=0.01)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("capture_span_exceeded", result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "blocked")
            self.assertIn("capture_span_exceeded", evidence["failed_gates"])

    def test_non_finite_capture_span_is_rejected_without_ready_evidence(self) -> None:
        responses = {f"/{name}": (status(), 0.0) for name in ("sequencer", "storage", "observer")}
        for non_finite_span in ("nan", "inf"):
            with (
                self.subTest(max_capture_span_seconds=non_finite_span),
                tempfile.TemporaryDirectory() as temp_dir,
                FleetHealthFixture(responses) as fixture,
            ):
                output = Path(temp_dir) / "fleet-health.json"
                result = self.run_collector(
                    fixture,
                    output,
                    max_span_seconds=non_finite_span,
                )

                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertFalse(
                    output.exists(),
                    "non-finite capture span must not produce ready evidence",
                )

    def test_any_head_projection_divergence_fails_closed(self) -> None:
        for field in (
            "committed_height",
            "network_committed_height",
            "last_execution_height",
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                divergent = status(head=42)
                divergent["consensus"][field] = 41
                responses = {
                    "/sequencer": (status(head=42), 0.0),
                    "/storage": (divergent, 0.0),
                    "/observer": (status(head=42), 0.0),
                }
                with FleetHealthFixture(responses) as fixture:
                    output = Path(temp_dir) / "fleet-health.json"
                    result = self.run_collector(fixture, output)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn("head_mismatch", result.stderr)
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertIn("head_mismatch", evidence["failed_gates"])

    def test_non_ready_network_head_fails_closed(self) -> None:
        responses = {
            "/sequencer": (status(), 0.0),
            "/storage": (status(decision="degraded"), 0.0),
            "/observer": (status(), 0.0),
        }
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            result = self.run_collector(fixture, output)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("network_head_not_ready", result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertIn("network_head_not_ready", evidence["failed_gates"])

    def test_node_readiness_error_and_failed_gates_each_fail_closed(self) -> None:
        invalid_statuses = {
            "node_not_ready": {"readiness": {"status": "not_ready", "failed_gates": []}},
            "failed_gates_nonempty": {"readiness": {"status": "ready", "failed_gates": ["stale"]}},
            "last_error_present": {"last_error": "replication stalled"},
        }
        for expected_gate, replacement in invalid_statuses.items():
            with self.subTest(gate=expected_gate), tempfile.TemporaryDirectory() as temp_dir:
                invalid = status()
                invalid.update(replacement)
                responses = {
                    "/sequencer": (status(), 0.0),
                    "/storage": (invalid, 0.0),
                    "/observer": (status(), 0.0),
                }
                with FleetHealthFixture(responses) as fixture:
                    output = Path(temp_dir) / "fleet-health.json"
                    result = self.run_collector(fixture, output)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_gate, result.stderr)
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertIn(expected_gate, evidence["failed_gates"])

    def test_managed_five_node_mode_accepts_only_the_canonical_full_fleet(self) -> None:
        names = ("sequencer", "storage", "linux-lan-observer", "windows-observer", "macos-observer")
        responses = {f"/{name}": (status(), 0.0) for name in names}
        responses["/sequencer"] = (status(checkpoint=checkpoint(height=42)), 0.0)
        responses["/storage"] = (status(checkpoint=checkpoint(height=43)), 0.0)
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            nodes = [(name, fixture.endpoint(name)) for name in names]
            result = self.run_collector(fixture, output, nodes=nodes, managed_five_node=True)

            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "ready")
            self.assertEqual(set(evidence["nodes"]), set(names))

    def test_managed_five_node_mode_requires_compatible_v2_provider_checkpoints(self) -> None:
        names = ("sequencer", "storage", "linux-lan-observer", "windows-observer", "macos-observer")
        cases: dict[str, tuple[object | None, object | None, str]] = {
            "null": (None, checkpoint(), "provider_checkpoint_missing"),
            "v1": ({**checkpoint(), "schema_version": 1}, checkpoint(), "provider_checkpoint_schema_invalid"),
            "malformed_identity": ({**checkpoint(), "checkpoint_id": " "}, checkpoint(), "provider_checkpoint_identity_invalid"),
            "malformed_hash": ({**checkpoint(), "manifest_hash": "not-a-checkpoint-hash"}, checkpoint(), "provider_checkpoint_identity_invalid"),
            "one_provider_only": (checkpoint(), None, "provider_checkpoint_missing"),
            "height_incompatibility": (checkpoint(height=42), checkpoint(height=44), "provider_checkpoint_height_incompatible"),
            "identity_mismatch": (checkpoint(), checkpoint(checkpoint_id="other"), "provider_checkpoint_identity_mismatch"),
        }
        for case, (sequencer_checkpoint, storage_checkpoint, expected_gate) in cases.items():
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temp_dir:
                responses = {f"/{name}": (status(), 0.0) for name in names}
                responses["/sequencer"] = (status(checkpoint=sequencer_checkpoint), 0.0)
                responses["/storage"] = (status(checkpoint=storage_checkpoint), 0.0)
                with FleetHealthFixture(responses) as fixture:
                    output = Path(temp_dir) / "fleet-health.json"
                    nodes = [(name, fixture.endpoint(name)) for name in names]
                    result = self.run_collector(fixture, output, nodes=nodes, managed_five_node=True)

                self.assertNotEqual(result.returncode, 0)
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertIn(expected_gate, evidence["failed_gates"])

    def test_managed_five_node_mode_rejects_omission_rename_and_duplicate_before_evidence(self) -> None:
        names = ("sequencer", "storage", "linux-lan-observer", "windows-observer", "macos-observer")
        responses = {f"/{name}": (status(), 0.0) for name in names}
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            cases = {
                "omission": [(name, fixture.endpoint(name)) for name in names if name != "macos-observer"],
                "rename": [
                    ("sequencer", fixture.endpoint("sequencer")),
                    ("storage", fixture.endpoint("storage")),
                    ("linux-lan-observer", fixture.endpoint("linux-lan-observer")),
                    ("windows-observer", fixture.endpoint("windows-observer")),
                    ("local-macos-observer", fixture.endpoint("macos-observer")),
                ],
                "duplicate": [
                    ("sequencer", fixture.endpoint("sequencer")),
                    ("storage", fixture.endpoint("storage")),
                    ("linux-lan-observer", fixture.endpoint("linux-lan-observer")),
                    ("windows-observer", fixture.endpoint("windows-observer")),
                    ("windows-observer", fixture.endpoint("macos-observer")),
                ],
            }
            for name, nodes in cases.items():
                with self.subTest(case=name):
                    output = Path(temp_dir) / f"{name}.json"
                    result = self.run_collector(fixture, output, nodes=nodes, managed_five_node=True)

                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("managed five-node", result.stderr)
                    self.assertFalse(output.exists(), "invalid managed fleet must not write evidence")

    def test_deployment_closure_docs_require_the_managed_five_node_preset(self) -> None:
        runbook = RUNBOOK.read_text(encoding="utf-8")
        inventory = INVENTORY.read_text(encoding="utf-8")

        self.assertIn("--managed-five-node", runbook)
        self.assertIn("--managed-five-node", inventory)
        for identity in (
            "sequencer",
            "storage",
            "linux-lan-observer",
            "windows-observer",
            "macos-observer",
        ):
            self.assertIn(identity, inventory)


if __name__ == "__main__":
    unittest.main()
