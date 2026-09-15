#!/usr/bin/env python3
"""PG-01 RED contract for governed public-testnet triad admission.

This file is intentionally tests-only.  It freezes the approved triad
deployment truth before the PG-02 implementation work starts:

* three unique validators with equal 100 stake (300 total, 200 required);
* a separate three-signer governance domain with a 2-of-3 / 6667-bps
  threshold;
* validator-47 is both a validator and an authorized checkpoint/full-storage
  provider;
* validator-47 is staged with its service and listeners rendered but never
  started or enabled; and
* pair-only and generic fleet-health output cannot be final triad evidence.

The expected failures in the initial RED run are deliberately tied to missing
triad production behavior.  Do not weaken these assertions to make the RED
phase pass.
"""

from __future__ import annotations

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


def load_python_module(path: Path, name: str) -> Any:
    # ``service-readback`` intentionally has no ``.py`` suffix.  Use the
    # source loader explicitly so the test reaches the missing triad registry
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
            runtime.write_text("runtime\n", encoding="utf-8")
            peers = temp / "bootstrap-peers.txt"
            peers.write_text(
                "/ip4/127.0.0.1/tcp/6831/p2p/12D3KooWTestSequencer\n"
                "/ip4/127.0.0.1/tcp/6832/p2p/12D3KooWTestStorage\n",
                encoding="utf-8",
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
                    "65c27d898af9c528ebd6a3762373faef110bb7bb515dfa88c447f292474aac16",
                    "--storage-finality-public-key",
                    "858e97be96f238ef3f6e07ec36d4ba5f503755ecb232d06a80ef1ab8aaca44f6",
                    "--extra-validator",
                    "triad-testnet-validator-47:47aabbccddeeff00112233445566778899aabbccddeeff001122334455667788:100",
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
            "/sequencer-204": triad_status("triad-testnet-sequencer"),
            "/storage-205": triad_status("triad-testnet-storage", provider=True),
            "/validator-47": triad_status("triad-testnet-validator-47", provider=True),
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
