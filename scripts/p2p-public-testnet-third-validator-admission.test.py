#!/usr/bin/env python3
"""PG-01 GREEN contract for governed public-testnet triad admission.

This file is intentionally tests-only.  It freezes the approved triad
deployment truth before any validator-47 cutover decision:

* three unique validators with equal 100 stake (300 total, 200 required);
* a separate three-signer governance domain with a 2-of-3 / 6667-bps
  threshold;
* validator-47 is both a validator and an authorized checkpoint/full-storage
  provider;
* validator-47 is staged with its service and listeners rendered but never
  started or enabled; and
* pair-only and generic fleet-health output cannot be final triad evidence.

The implementation now supplies this governed triad contract.  Keep these
assertions strict so future changes cannot regress authority, provider closure,
no-start safety, or the distinction between pair-only and triad evidence.
"""

from __future__ import annotations

import hashlib
import http.server
from importlib.machinery import SourceFileLoader
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "scripts" / "public-testnet-validator-triad-inventory.v1.json"
SERVICE_READBACK = ROOT / "scripts" / "service-readback"
BOOTSTRAP = ROOT / "scripts" / "p2p-public-testnet-bootstrap-fresh-validator-host.sh"
STAGE = ROOT / "scripts" / "p2p-public-testnet-build-deployment-stage.sh"
FLEET_HEALTH = ROOT / "scripts" / "p2p-public-testnet-fleet-health.py"

TRIAD_NODE_IDS = {
    "sequencer-204": "triad-testnet-sequencer",
    "storage-205": "triad-testnet-storage",
    "validator-47": "triad-testnet-validator-47",
}
TRIAD_STAKES = [100, 100, 100]
TRIAD_TOTAL_STAKE = 300
TRIAD_REQUIRED_STAKE = 200
GOVERNANCE_SIGNER_COUNT = 3
GOVERNANCE_THRESHOLD = 2
GOVERNANCE_THRESHOLD_BPS = 6667
VALIDATOR_47_SERVICE = "oasis7-triad-validator-47.service"
VALIDATOR_47_PORTS = {"6634", "6834"}
TRIAD_SIGNERS = {
    "triad-testnet-sequencer": "e01e5c34dee2da3087653bc4cec02be01632f56250a800994c96ea44ae6f3690",
    "triad-testnet-storage": "1f530cae002d7adb9a6c3dd8f4bc861226f112f88fdd252b28b6494019e21c33",
    "triad-testnet-validator-47": "302e6f629a3d148623fb96ef5dad4ef9530e1e5f56d095d9c59d76ce530c1f73",
}
TRIAD_ROOT_PUBLIC_KEY = "4eb958cb0376568df7c7abf3d3708b5b5ae52e2928b533e41a203c58271a6a25"
TRIAD_VALIDATOR_47_PEER_ID = "12D3KooWRHQrqchtg87SpBHiojGaVtfoGcPsnUfiNSpJd5J7UJbU"
TRIAD_VALIDATOR_47_FIXTURE_SIGNER = TRIAD_SIGNERS["triad-testnet-validator-47"]
TRIAD_REGISTRY_FIXTURE_DIGEST = "1290818e16b4d5f6fa1929a18e0d6c73295d0ff86e8c7ee8ee58e670874199fd"
TRIAD_INVENTORY_DIGEST = hashlib.sha256(INVENTORY.read_bytes()).hexdigest()
TRIAD_WORLD_ID = "oasis7-public-testnet-governed-20260606"
TRIAD_MANIFEST_HASH = hashlib.sha256(b"oasis7-test-fixture-triad-manifest").hexdigest()
TRIAD_REGISTRY_SEMANTIC_DIGEST = hashlib.sha256(
    json.dumps(
        {
            "signer_bindings": {
                f"governance.finality.v1.{node_id}": signer
                for node_id, signer in sorted(TRIAD_SIGNERS.items())
            },
            "slot_id": "governance.finality.v1",
            "threshold": 2,
            "threshold_bps": 0,
            "validator_stakes": {
                f"governance.finality.v1.{node_id}": 100
                for node_id in sorted(TRIAD_SIGNERS)
            },
        },
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
).hexdigest()


def load_python_module(path: Path, name: str) -> Any:
    # ``service-readback`` intentionally has no ``.py`` suffix.  Use the
    # source loader explicitly so the test reaches the triad registry
    # assertion instead of failing in the harness itself.
    loader = SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def triad_status(
    node_id: str,
    *,
    role: str = "validator",
    provider: bool = False,
    head: int = 42,
) -> dict[str, Any]:
    """Return a deterministic status fixture for validator-aware health."""
    return {
        "node_id": node_id,
        "running": True,
        "last_error": None,
        "readiness": {"status": "ready", "failed_gates": []},
        "consensus": {
            "committed_height": head,
            "network_committed_height": head,
            "last_execution_height": head,
            "network_head": {"decision": "ready"},
        },
        "validator": {
            "role": role,
            "membership": "active",
            "stake": 100,
            "signer_binding": node_id,
        },
        "provider": {
            "checkpoint": provider,
            "full_storage": provider,
        },
    }


def node_emitted_triad_status(node_name: str, *, provider: bool = False, head: int = 42) -> dict[str, Any]:
    """Return a complete node-emitted projection fixture for managed triad.

    This deliberately keeps the historical ``triad_status`` fixture above as
    a negative-path fixture.  The collector must not upgrade that old shape
    by guessing validator/provider state; only this complete versioned
    projection may exercise the ready path.
    """
    node_id = TRIAD_NODE_IDS[node_name]
    runtime_role = "sequencer" if node_name == "sequencer-204" else "storage"
    p2p_role = "validator_core" if node_name == "sequencer-204" else "full_storage"
    signer = TRIAD_SIGNERS[node_id]
    validator_set_hash = "runtime-validator-set-hash"
    stake_root = hashlib.sha256(b"oasis7-test-fixture-triad-stake-root").hexdigest()
    checkpoint_id = "checkpoint-42"
    proof_hash = hashlib.sha256(b"oasis7-test-fixture-world-head-proof").hexdigest()
    base = {
        "node_id": node_id,
        "world_id": TRIAD_WORLD_ID,
        "role": runtime_role,
        "running": True,
        "last_error": None,
        "readiness": {"status": "ready", "failed_gates": []},
        "consensus": {
            "committed_height": head,
            "network_committed_height": head,
            "last_execution_height": head,
            "network_head": {"decision": "ready"},
            "validator_set_hash": validator_set_hash,
            "validator_stake_root": stake_root,
        },
        "network_tier": {
            "tier": "public_testnet",
            "network_id": TRIAD_WORLD_ID,
            "chain_id": TRIAD_WORLD_ID,
            "target_validator_count": 3,
        },
        "world_resource": {
            "world_id": TRIAD_WORLD_ID,
            "chain_id": TRIAD_WORLD_ID,
            "seed_manifest_hash": TRIAD_MANIFEST_HASH,
        },
        "p2p": {"node_role_claim": p2p_role},
        "chain_proof": {
            "latest_execution_checkpoint": {
                "schema_version": 2,
                "checkpoint_id": checkpoint_id,
                "height": head,
                "manifest_hash": TRIAD_MANIFEST_HASH,
            },
            "latest_world_head_proof": {
                "checkpoint_ref": checkpoint_id,
                "height": head,
                "proof_hash": proof_hash,
                "world_id": TRIAD_WORLD_ID,
            },
        },
        "validator": {
            "schema_version": "oasis7.chain_validator_provider_status.v1",
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
                "leaf_hash": hashlib.sha256(f"leaf:{node_id}".encode("utf-8")).hexdigest(),
                "proof": [],
            },
            "validator_set_hash": validator_set_hash,
            "stake_root": stake_root,
            "registry_ref": "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
            "registry_sha256": TRIAD_REGISTRY_FIXTURE_DIGEST,
            "registry_semantic_sha256": TRIAD_REGISTRY_SEMANTIC_DIGEST,
            "inventory_ref": "scripts/public-testnet-validator-triad-inventory.v1.json",
            "inventory_sha256": TRIAD_INVENTORY_DIGEST,
        },
        "provider": {
            "schema_version": "oasis7.chain_validator_provider_status.v1",
            "node_id": node_id,
            "provider_id": (
                "12D3KooWRHQrqchtg87SpBHiojGaVtfoGcPsnUfiNSpJd5J7UJbU"
                if node_id == TRIAD_NODE_IDS["validator-47"]
                else f"peer-{node_id}"
            ),
            "checkpoint": provider,
            "full_storage": provider,
            "checkpoint_proof": (
                {
                    "schema_version": 2,
                    "checkpoint_id": checkpoint_id,
                    "height": head,
                    "manifest_hash": TRIAD_MANIFEST_HASH,
                    "proof_hash": proof_hash,
                    "world_id": TRIAD_WORLD_ID,
                    "chain_id": TRIAD_WORLD_ID,
                }
                if provider
                else None
            ),
            "full_storage_proof": (
                {
                    "status": "ready",
                    "provider_id": (
                        "12D3KooWRHQrqchtg87SpBHiojGaVtfoGcPsnUfiNSpJd5J7UJbU"
                        if node_id == TRIAD_NODE_IDS["validator-47"]
                        else f"peer-{node_id}"
                    ),
                    "world_id": TRIAD_WORLD_ID,
                    "chain_id": TRIAD_WORLD_ID,
                    "manifest_hash": TRIAD_MANIFEST_HASH,
                    "height": head,
                }
                if provider
                else None
            ),
        },
    }
    return base


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

    def url(self, path: str) -> str:
        host, port = self.server.server_address
        return f"http://{host}:{port}{path}"


class ThirdValidatorAdmissionContractTest(unittest.TestCase):
    def test_triad_inventory_binds_validator_47_identity_provider_service_and_ports(self) -> None:
        self.assertTrue(INVENTORY.is_file(), f"missing governed triad inventory: {INVENTORY}")
        self.assertFalse(INVENTORY.is_symlink(), "triad inventory must not be a symlink")
        value = json.loads(INVENTORY.read_text(encoding="utf-8"))

        self.assertEqual(value["schema_version"], "oasis7.public_testnet_validator_triad_inventory.v1")
        self.assertEqual(value["repository"], "eng-cc/oasis7")
        self.assertEqual(value["network_tier"], "public_testnet")
        self.assertEqual(value["topology"], "three_equal_validator")
        self.assertEqual(value["validator_set"]["stakes"], TRIAD_STAKES)
        self.assertEqual(value["validator_set"]["total_stake"], TRIAD_TOTAL_STAKE)
        self.assertEqual(value["validator_set"]["required_stake"], TRIAD_REQUIRED_STAKE)
        self.assertEqual(value["validator_set"]["quorum"], {"numerator": 2, "denominator": 3})
        self.assertEqual(
            value["governance"],
            {
                "signer_count": GOVERNANCE_SIGNER_COUNT,
                "threshold": GOVERNANCE_THRESHOLD,
                "threshold_bps": GOVERNANCE_THRESHOLD_BPS,
            },
        )

        nodes = value["nodes"]
        self.assertEqual(set(nodes), set(TRIAD_NODE_IDS))
        validator_47 = nodes["validator-47"]
        self.assertEqual(validator_47["node_id"], TRIAD_NODE_IDS["validator-47"])
        self.assertEqual(validator_47["host"], "root@47.111.225.27")
        self.assertEqual(validator_47["roles"], ["validator", "checkpoint_provider", "full_storage_provider"])
        self.assertEqual(validator_47["service"], VALIDATOR_47_SERVICE)
        self.assertEqual(set(validator_47["ports"]), VALIDATOR_47_PORTS)

    def test_deployment_stage_registry_freezes_equal_stake_and_separate_governance_threshold(self) -> None:
        """The stage output must carry PoS and governance thresholds separately."""
        with tempfile.TemporaryDirectory(prefix="oasis7-pg01-stage-") as temp_dir:
            temp = Path(temp_dir)
            runtime = temp / "oasis7_chain_runtime"
            runtime.write_text(
                "#!/usr/bin/env bash\n"
                "set -eu\n"
                "[[ \"${1:-}\" == identity-receipt ]] || exit 64\n"
                "python3 - \"$3\" \"$5\" <<'PY'\n"
                "import hashlib, json, pathlib, sys\n"
                "config_dir = pathlib.Path(sys.argv[1])\n"
                "node_id = sys.argv[2]\n"
                "key_path = config_dir / 'node-keypair.toml'\n"
                "print(json.dumps({'schema_version':'oasis7.identity_receipt.v1', 'node_id':node_id, 'peer_id':'12D3KooWRHQrqchtg87SpBHiojGaVtfoGcPsnUfiNSpJd5J7UJbU', 'key_path':str(key_path), 'key_sha256':hashlib.sha256(key_path.read_bytes()).hexdigest(), 'key_size_bytes':key_path.stat().st_size, 'key_mode':384, 'key_uid':key_path.stat().st_uid, 'key_gid':key_path.stat().st_gid}))\n"
                "PY\n",
                encoding="utf-8",
            )
            runtime.chmod(0o755)
            identity_dir = temp / "identity"
            identity_dir.mkdir()
            identity_dir.chmod(0o700)
            identity_key = identity_dir / "node-keypair.toml"
            identity_key.write_text("already-staged-validator-47-key\n", encoding="utf-8")
            identity_key.chmod(0o600)
            (identity_dir / "identity-receipt.json").write_text(
                json.dumps(
                    {
                        "schema_version": "oasis7.identity_provision.v1",
                        "node_id": "triad-testnet-validator-47",
                        "root_public_key": TRIAD_ROOT_PUBLIC_KEY,
                        "finality_public_key": TRIAD_VALIDATOR_47_FIXTURE_SIGNER,
                        "libp2p_peer_id": TRIAD_VALIDATOR_47_PEER_ID,
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            (identity_dir / "identity-receipt.json").chmod(0o600)
            peers = temp / "bootstrap-peers.txt"
            peers.write_bytes(
                (ROOT / "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt").read_bytes()
            )
            output = temp / "stage"
            result = subprocess.run(
                [
                    str(STAGE),
                    "--runtime-build-ref",
                    str(runtime),
                    "--bootstrap-peers-file",
                    str(peers),
                    "--sequencer-finality-public-key",
                    TRIAD_SIGNERS["triad-testnet-sequencer"],
                    "--storage-finality-public-key",
                    TRIAD_SIGNERS["triad-testnet-storage"],
                    "--extra-validator",
                    f"triad-testnet-validator-47:{TRIAD_VALIDATOR_47_FIXTURE_SIGNER}:100",
                    "--validator-47-identity-dir",
                    str(identity_dir),
                    "--out-dir",
                    str(output),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            registry_path = output / "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
            registry = json.loads(registry_path.read_text(encoding="utf-8"))
            staged_identity = output / "identity"
            self.assertEqual(
                (staged_identity / "node-keypair.toml").read_bytes(),
                identity_key.read_bytes(),
            )
            self.assertEqual(
                (staged_identity / "identity-receipt.json").read_bytes(),
                (identity_dir / "identity-receipt.json").read_bytes(),
            )
            for identity_file in (staged_identity / "node-keypair.toml", staged_identity / "identity-receipt.json"):
                self.assertTrue(identity_file.is_file())
                self.assertFalse(identity_file.is_symlink())
                self.assertEqual(identity_file.stat().st_mode & 0o777, 0o600)
            staged_env = (output / "config/node.env").read_text(encoding="utf-8")
            self.assertIn("IDENTITY_KEY_PATH=config/node-keypair.toml", staged_env)
            self.assertIn("IDENTITY_RECEIPT_PATH=config/identity-receipt.json", staged_env)

        self.assertEqual([entry["stake"] for entry in registry["validators"]], TRIAD_STAKES)
        self.assertIn("quorum", registry, "staged triad registry missing quorum metadata")
        self.assertIn("governance", registry, "staged triad registry missing governance metadata")
        self.assertEqual(
            registry["quorum"],
            {
                "numerator": 2,
                "denominator": 3,
                "total_stake": TRIAD_TOTAL_STAKE,
                "required_stake": TRIAD_REQUIRED_STAKE,
            },
        )
        self.assertEqual(
            registry["governance"],
            {
                "signer_count": GOVERNANCE_SIGNER_COUNT,
                "threshold": GOVERNANCE_THRESHOLD,
                "threshold_bps": GOVERNANCE_THRESHOLD_BPS,
            },
        )

    def test_service_readback_exposes_validator_47_service_and_listeners(self) -> None:
        module = load_python_module(SERVICE_READBACK, "service_readback_pg01")
        self.assertIn("validator-47", module.SERVICES)
        self.assertEqual(module.SERVICES["validator-47"]["service"], VALIDATOR_47_SERVICE)
        self.assertEqual(set(module.SERVICES["validator-47"]["ports"]), VALIDATOR_47_PORTS)

    def test_validator_47_bootstrap_is_explicit_no_start_staging(self) -> None:
        source = BOOTSTRAP.read_text(encoding="utf-8")
        self.assertIn("validator-47", source)
        self.assertIn(VALIDATOR_47_SERVICE, source)
        self.assertNotIn("systemctl start", source)
        self.assertNotIn("systemctl enable", source)
        self.assertIn("no_service_started:true", source)

    def _run_health(
        self,
        fixture: JsonFixtureServer,
        output: Path,
        nodes: list[tuple[str, str]],
        *,
        managed_triad: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(FLEET_HEALTH), "--sequencer", "sequencer-204"]
        for name, endpoint in nodes:
            command.extend(["--node", f"{name}={endpoint}"])
        if managed_triad:
            command.append("--managed-triad")
        command.extend(["--max-capture-span-seconds", "1", "--output", str(output)])
        return subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)

    def test_validator_aware_health_accepts_only_three_equal_validator_provider_closure(self) -> None:
        responses = {
            "/sequencer-204": node_emitted_triad_status("sequencer-204"),
            "/storage-205": node_emitted_triad_status("storage-205"),
            "/validator-47": node_emitted_triad_status("validator-47", provider=True),
        }
        with tempfile.TemporaryDirectory(prefix="oasis7-pg01-health-") as temp_dir, JsonFixtureServer(responses) as fixture:
            output = Path(temp_dir) / "triad-health.json"
            result = self._run_health(
                fixture,
                output,
                [(name, fixture.url(f"/{name}")) for name in ("sequencer-204", "storage-205", "validator-47")],
                managed_triad=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))

        self.assertEqual(evidence["scope"], "managed_triad")
        self.assertEqual(evidence["claim_mode"], "three_equal_validator")
        self.assertEqual(evidence["verdict"], "ready")
        self.assertEqual(evidence["validator_set"]["stakes"], TRIAD_STAKES)
        self.assertEqual(evidence["validator_set"]["total_stake"], TRIAD_TOTAL_STAKE)
        self.assertEqual(evidence["validator_set"]["required_stake"], TRIAD_REQUIRED_STAKE)
        self.assertEqual(evidence["governance"]["threshold"], GOVERNANCE_THRESHOLD)
        self.assertEqual(evidence["governance"]["threshold_bps"], GOVERNANCE_THRESHOLD_BPS)
        self.assertTrue(evidence["nodes"]["validator-47"]["provider"]["checkpoint"])
        self.assertTrue(evidence["nodes"]["validator-47"]["provider"]["full_storage"])

    def test_historical_synthetic_status_stays_blocked(self) -> None:
        """The pre-projection fixture must never be upgraded by the collector."""
        responses = {
            "/sequencer-204": triad_status("triad-testnet-sequencer"),
            "/storage-205": triad_status("triad-testnet-storage", provider=True),
            "/validator-47": triad_status("triad-testnet-validator-47", provider=True),
        }
        with tempfile.TemporaryDirectory(prefix="oasis7-pg01-historical-health-") as temp_dir, JsonFixtureServer(responses) as fixture:
            output = Path(temp_dir) / "historical-health.json"
            result = self._run_health(
                fixture,
                output,
                [(name, fixture.url(f"/{name}")) for name in ("sequencer-204", "storage-205", "validator-47")],
                managed_triad=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("projection", result.stderr.lower())
            self.assertTrue(output.exists())
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "blocked")

    def test_pair_only_health_is_not_final_triad_evidence(self) -> None:
        responses = {
            "/sequencer-204": triad_status("triad-testnet-sequencer"),
            "/storage-205": triad_status("triad-testnet-storage", provider=True),
        }
        with tempfile.TemporaryDirectory(prefix="oasis7-pg01-pair-health-") as temp_dir, JsonFixtureServer(responses) as fixture:
            output = Path(temp_dir) / "pair-health.json"
            result = self._run_health(
                fixture,
                output,
                [(name, fixture.url(f"/{name}")) for name in ("sequencer-204", "storage-205")],
                managed_triad=True,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("managed triad", result.stderr.lower())
        self.assertFalse(output.exists(), "pair-only input must not emit triad evidence")

    def test_validator_only_health_is_not_provider_closure(self) -> None:
        responses = {
            "/sequencer-204": triad_status("triad-testnet-sequencer"),
            "/storage-205": triad_status("triad-testnet-storage", provider=True),
            "/validator-47": triad_status("triad-testnet-validator-47", provider=False),
        }
        with tempfile.TemporaryDirectory(prefix="oasis7-pg01-validator-only-") as temp_dir, JsonFixtureServer(responses) as fixture:
            output = Path(temp_dir) / "validator-only-health.json"
            result = self._run_health(
                fixture,
                output,
                [(name, fixture.url(f"/{name}")) for name in ("sequencer-204", "storage-205", "validator-47")],
                managed_triad=True,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("provider", result.stderr.lower())
        self.assertFalse(output.exists(), "validator-only input must not emit triad evidence")

    def test_generic_health_is_not_final_triad_evidence(self) -> None:
        responses = {
            "/sequencer-204": triad_status("triad-testnet-sequencer"),
            "/storage-205": triad_status("triad-testnet-storage", provider=True),
            "/validator-47": triad_status("triad-testnet-validator-47", provider=True),
        }
        with tempfile.TemporaryDirectory(prefix="oasis7-pg01-generic-health-") as temp_dir, JsonFixtureServer(responses) as fixture:
            output = Path(temp_dir) / "generic-health.json"
            result = self._run_health(
                fixture,
                output,
                [(name, fixture.url(f"/{name}")) for name in ("sequencer-204", "storage-205", "validator-47")],
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("validator-aware", result.stderr.lower())
        if output.exists():
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertNotEqual(evidence.get("verdict"), "ready")
            self.assertNotEqual(evidence.get("claim_mode"), "three_equal_validator")


if __name__ == "__main__":
    unittest.main()
