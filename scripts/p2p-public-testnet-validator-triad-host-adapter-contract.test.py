#!/usr/bin/env python3
"""Contract tests for the governed triad staggered host-adapter boundary.

The adapter is intentionally mocked at the subprocess boundary.  These tests
exercise the repository-owned executor's real binding, receipt validation, and
durable callback journal without contacting a host, SSH endpoint, or secret.
"""

from __future__ import annotations

import base64
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
ADAPTER_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-triad-host-adapter.py"


def load_executor():
    spec = importlib.util.spec_from_file_location("triad_host_adapter_contract_executor", EXECUTOR)
    if spec is None or spec.loader is None:
        raise RuntimeError("unable to load governed validator executor")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


MODULE = load_executor()


def load_adapter():
    spec = importlib.util.spec_from_file_location("triad_host_adapter_contract_adapter", ADAPTER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("unable to load governed triad host adapter")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ADAPTER = load_adapter()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


TRIAD_ROLLBACK_CONTRACT = {
    "strategy": "staggered-target-only-cleanup",
    "required_on_gate_failure": True,
    "target_only_cleanup": True,
    "restore_deleted_chain_state": False,
    "restore_only_forensic_snapshot": True,
}


class GovernedTriadHostAdapterContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="oasis7-triad-host-adapter-contract-")
        self.root = Path(self.tmp.name)
        self.runtime = self.root / "oasis7_chain_runtime"
        self.runtime.write_bytes(b"runtime-fixture\n")
        self.runtime_sha256 = sha256(self.runtime)
        self.package_sha256 = "b" * 64
        self.package_commit = "c" * 40
        self.package_run_id = "fixture-run-001"
        self.identity_receipts: list[dict[str, str]] = []
        for role in MODULE.MUTATION_ORDER:
            path = self.root / f"identity-{role}.json"
            path.write_text(json.dumps({"role": role}) + "\n", encoding="utf-8")
            self.identity_receipts.append(
                {
                    "path": str(path),
                    "sha256": sha256(path),
                    "role": role,
                    "node_id": f"node-{role}",
                    "peer_id": f"peer-{role}",
                }
            )
        self.sequencer_proof = self.root / "sequencer-rebuild-proof.json"
        self.sequencer_proof.write_text("{\"fixture\":true}\n", encoding="utf-8")
        self.adapter = self.root / "mock-governed-host-adapter.py"
        self._write_mock_adapter()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def _base_plan(self) -> dict[str, object]:
        nodes = {}
        for role in MODULE.MUTATION_ORDER:
            node_root = self.root / role
            node_root.mkdir(exist_ok=True)
            nodes[role] = {"role": role, "root": str(node_root)}
        proof = {
            "identity_receipts": self.identity_receipts,
            "storage_health_url": "http://127.0.0.1/healthz",
            "sequencer_health_url": "http://127.0.0.1/healthz",
            "sequencer_rebuild_proof": {
                "sha256": sha256(self.sequencer_proof),
                "role": "sequencer-204",
                "node_id": "node-sequencer-204",
                "peer_id": "peer-sequencer-204",
            },
            "sequencer_rebuild_proof_path": str(self.sequencer_proof),
        }
        live_nodes = {
            role: {
                "role": role,
                "root": str(nodes[role]["root"]),
                "active": True,
                "running": True,
                "service_state": "running",
                "independently_observed": True,
            }
            for role in MODULE.MUTATION_ORDER
        }
        live_observations = {
            key: {"observation_state": "live", "nodes": copy.deepcopy(live_nodes)}
            for key in (
                "before-staggered-preflight",
                "before-staggered-storage",
                "after-staggered-storage",
                "before-staggered-sequencer",
                "after-staggered-sequencer",
            )
        }
        return {
            "execution_mode": MODULE.TRIAD_STAGGERED_EXECUTION_MODE,
            "transaction_id": "triad-host-adapter-contract-001",
            "plan_digest": "a" * 64,
            "nodes": nodes,
            "startup_order": list(MODULE.MUTATION_ORDER),
            "package": {
                "runtime_sha256": self.runtime_sha256,
                "package_sha256": self.package_sha256,
                "commit": self.package_commit,
                "run_id": self.package_run_id,
            },
            "proof": proof,
            "observer_gate": {"status": "hold"},
            "rollback": dict(TRIAD_ROLLBACK_CONTRACT),
            "staggered_live_observations": live_observations,
        }

    def _write_mock_adapter(self) -> None:
        reset_surfaces = repr(list(MODULE.RESET_SURFACES))
        reset_digest = repr(MODULE.reset_surface_digest())
        listeners = repr({role: sorted(MODULE.EXPECTED_LISTENERS[role]) for role in MODULE.MUTATION_ORDER})
        self.adapter.write_text(
            f"""#!{os.sys.executable}
import datetime as dt
import json
import os
import sys
from pathlib import Path

transaction_path = Path(sys.argv[sys.argv.index('--transaction') + 1])
phase = sys.argv[sys.argv.index('--phase') + 1]
transaction = json.loads(transaction_path.read_text(encoding='utf-8'))
if os.environ.get('O7_ADAPTER_FAIL_PHASE') == phase:
    raise SystemExit(17)
binding = transaction['adapter_binding']
roles = ['storage-205', 'sequencer-204']
reset_surfaces = {reset_surfaces}
reset_digest = {reset_digest}
listeners = {listeners}
runtime_sha256 = transaction['package']['runtime_sha256']

def node(role, *, active=True, running=True):
    return {{
        'role': role,
        'root': transaction['nodes'][role]['root'],
        'active': active,
        'running': running,
        'service_state': 'running' if running else 'stopped',
        'independently_observed': True,
        'healthz_ok': True,
        'nrestarts': 0,
        'oom_panic_segfault': False,
        'runtime_sha256': runtime_sha256,
        'listeners': listeners[role],
        'full_chain_status_called': False,
    }}

receipt = {{
    'schema_version': 'oasis7.validator_pair_rebuild_host_receipt.v2',
    'phase': phase,
    'execution_mode': 'triad_staggered',
    'transaction_id': transaction['transaction_id'],
    'plan_digest': binding['plan_digest'],
    'evidence_bindings': binding['evidence_bindings'],
    'repository_executable': binding['repository_executable'],
    'captured_at': dt.datetime.now(dt.timezone.utc).isoformat().replace('+00:00', 'Z'),
    'mutation_order': roles,
    'startup_order': transaction['startup_order'],
    'observer_mutation': False,
    'max_simultaneously_stopped_validators': 1,
    'identity_receipts': binding['evidence_bindings']['identity_receipts'],
    'sequencer_rebuild_proof': binding['evidence_bindings']['sequencer_rebuild_proof'],
    'package': transaction['package'],
    'nodes': {{role: node(role) for role in roles}},
}}
if phase == 'staggered-preflight':
    receipt.update({{'staggered_phase': 'preflight', 'live_baseline': True}})
    for value in receipt['nodes'].values():
        value['preflight_observer_mutation'] = False
elif phase in ('staggered-storage', 'staggered-sequencer'):
    target = 'storage-205' if phase == 'staggered-storage' else 'sequencer-204'
    peer = 'sequencer-204' if target == 'storage-205' else 'storage-205'
    receipt.update({{
        'staggered_phase': 'member_cutover',
        'target_role': target,
        'live_peer_role': peer,
        'live_peer_readback': True,
        'rebuilt_member_readiness': True,
        'target_stopped_before_reset': {{
            'active': False,
            'running': False,
            'service_state': 'stopped',
            'independently_observed': True,
        }},
        'reset_started_after_target_stop': True,
    }})
    receipt['nodes'][target]['post_delete_absence'] = {{
        'absent': True,
        'target_set': reset_surfaces,
        'target_set_sha256': reset_digest,
    }}
elif phase == 'staggered-rollback':
    failed_role = 'storage-205'
    peer = 'sequencer-204'
    receipt.update({{
        'staggered_phase': 'target_only_cleanup',
        'failed_role': failed_role,
        'target_only_cleanup': True,
        'restore_deleted_chain_state': False,
    }})
    receipt['nodes'][failed_role] = node(failed_role, active=False, running=False)
else:
    raise SystemExit('unsupported phase fixture')
if os.environ.get('O7_ADAPTER_BAD_IDENTITY') == '1':
    receipt['evidence_bindings'] = {{}}
if os.environ.get('O7_ADAPTER_BAD_STOP_ORDER') == '1':
    receipt['target_stopped_before_reset']['running'] = True
if os.environ.get('O7_ADAPTER_BAD_PEER_STOPPED') == '1':
    peer = receipt.get('live_peer_role', 'sequencer-204')
    receipt['nodes'][peer]['active'] = False
    receipt['nodes'][peer]['running'] = False
    receipt['nodes'][peer]['service_state'] = 'stopped'
if os.environ.get('O7_ADAPTER_BAD_ROLLBACK') == '1':
    receipt['restore_deleted_chain_state'] = True
if os.environ.get('O7_ADAPTER_BAD_PACKAGE_HASH') == '1':
    receipt['package']['package_sha256'] = 'd' * 64
if os.environ.get('O7_ADAPTER_BAD_PACKAGE_COMMIT') == '1':
    receipt['package']['commit'] = 'e' * 40
if os.environ.get('O7_ADAPTER_BAD_PACKAGE_RUN_ID') == '1':
    receipt['package']['run_id'] = 'wrong-run-id'
print(json.dumps(receipt, ensure_ascii=True, sort_keys=True))
""",
            encoding="utf-8",
        )
        self.adapter.chmod(0o700)

    def _invoke(self, phase: str) -> tuple[dict[str, object], dict[str, object], Path]:
        transaction = self._base_plan()
        path = self.root / f"{phase}.transaction.json"
        path.write_text(json.dumps(transaction, sort_keys=True) + "\n", encoding="utf-8")
        receipt = MODULE.run_host_adapter(self.adapter, path, transaction, phase)
        persisted = json.loads(path.read_text(encoding="utf-8"))
        return receipt, persisted, path

    def _invoke_with_env_failure(self, phase: str, **environment: str) -> tuple[SystemExit, dict[str, object]]:
        transaction = self._base_plan()
        path = self.root / f"{phase}.failure.transaction.json"
        path.write_text(json.dumps(transaction, sort_keys=True) + "\n", encoding="utf-8")
        with patch.dict(os.environ, environment, clear=False):
            with self.assertRaises(SystemExit) as raised:
                MODULE.run_host_adapter(self.adapter, path, transaction, phase)
        return raised.exception, json.loads(path.read_text(encoding="utf-8"))

    def test_mock_adapter_covers_all_staggered_phases_and_durable_receipts(self) -> None:
        for phase in MODULE.TRIAD_STAGGERED_PHASES:
            receipt, persisted, _ = self._invoke(phase)
            self.assertEqual(receipt["phase"], phase)
            self.assertEqual(receipt["package"]["package_sha256"], self.package_sha256)
            self.assertEqual(receipt["package"]["commit"], self.package_commit)
            self.assertEqual(receipt["package"]["run_id"], self.package_run_id)
            callback = persisted["adapter_callback"]
            self.assertEqual(callback["status"], "completed")
            self.assertEqual(callback["phase"], phase)
            self.assertEqual(callback["receipt_digest"], MODULE._json_payload_digest(receipt))

    def test_identity_and_repository_binding_are_strict(self) -> None:
        error, persisted = self._invoke_with_env_failure(
            "staggered-storage", O7_ADAPTER_BAD_IDENTITY="1"
        )
        self.assertRegex(str(error), r"(?i)(binding|evidence|provenance|identity)")
        self.assertEqual(persisted["adapter_callback"]["status"], "failed")
        self.assertRegex(persisted["adapter_callback"]["error"], r"(?i)(binding|evidence|provenance|identity)")

    def test_canonical_host_pin_rejects_wrong_public_fingerprint(self) -> None:
        key_blob = base64.b64encode(b"mock-ed25519-public-key").decode("ascii")
        fingerprint = "SHA256:" + base64.b64encode(hashlib.sha256(base64.b64decode(key_blob)).digest()).decode("ascii").rstrip("=")
        known_hosts = self.root / "known-hosts"
        known_hosts.write_text(f"39.104.205.67 ssh-ed25519 {key_blob}\n", encoding="utf-8")
        accepted = MODULE._direct_known_host_pin(
            known_hosts, "root@39.104.205.67", fingerprint, "storage-205"
        )
        self.assertEqual(accepted["computed"], fingerprint)
        with self.assertRaisesRegex(SystemExit, r"(?i)(expected public pin|fingerprint)"):
            MODULE._direct_known_host_pin(
                known_hosts, "root@39.104.205.67", "SHA256:" + "A" * 43, "storage-205"
            )

    def test_stop_before_reset_and_peer_preservation_are_fail_closed(self) -> None:
        error, persisted = self._invoke_with_env_failure(
            "staggered-storage", O7_ADAPTER_BAD_STOP_ORDER="1"
        )
        self.assertRegex(str(error), r"(?i)(stopped before reset|reset-after-stop)")
        self.assertEqual(persisted["adapter_callback"]["status"], "failed")

        error, persisted = self._invoke_with_env_failure(
            "staggered-sequencer", O7_ADAPTER_BAD_PEER_STOPPED="1"
        )
        self.assertRegex(str(error), r"(?i)(live peer|running|health)")
        self.assertEqual(persisted["adapter_callback"]["status"], "failed")

    def test_peer_health_failure_precedes_destructive_script_generation(self) -> None:
        class SyntheticHealthTransport:
            def command(self, role: str, remote: str, timeout: int = 60) -> str:
                del role, timeout
                if "service-readback" in remote:
                    # These values are deliberately synthetic.  A production
                    # adapter must not treat them as /healthz authority.
                    return json.dumps(
                        {
                            "independently_observed": True,
                            "active": True,
                            "running": True,
                            "service_state": "running",
                            "listeners": [],
                            "healthz_ok": True,
                            "ready": True,
                            "last_error": None,
                            "nrestarts": 0,
                            "oom_panic_segfault": False,
                        }
                    )
                if "/healthz" in remote:
                    raise SystemExit("health probe unavailable")
                if "sha256sum" in remote:
                    return "a" * 64
                raise AssertionError(f"unexpected remote command: {remote}")

        transaction = {
            "nodes": {
                "storage-205": {"root": "/fixture/storage"},
                "sequencer-204": {"root": "/fixture/sequencer"},
            },
            "proof": {
                "storage_health_url": "http://127.0.0.1/healthz",
                "sequencer_health_url": "http://127.0.0.1/healthz",
            },
        }
        inventory = {
            "nodes": {
                "storage-205": {"service": "oasis7-triad-storage.service"},
                "sequencer-204": {"service": "oasis7-triad-sequencer.service"},
            }
        }
        control_calls: list[object] = []

        def unexpected_control_script(*_args: object, **_kwargs: object) -> tuple[str, list[object]]:
            control_calls.append(True)
            raise AssertionError("destructive control script was generated before health proof")

        with (
            patch.object(ADAPTER, "validate_transaction", return_value=(inventory, {}, {}, {})),
            patch.object(ADAPTER, "credential_fds", return_value={}),
            patch.object(ADAPTER, "FixedSSH", return_value=SyntheticHealthTransport()),
            patch.object(ADAPTER, "require_remote_backup"),
            patch.object(ADAPTER, "control_script", unexpected_control_script),
        ):
            with self.assertRaisesRegex(SystemExit, r"(?i)health"):
                ADAPTER.run_phase(transaction, "staggered-storage")
        self.assertEqual(control_calls, [])

    def test_preflight_proves_each_member_through_transaction_bound_healthz(self) -> None:
        class HealthyTransport:
            def __init__(self, runtime_sha256: str) -> None:
                self.calls: list[tuple[str, str]] = []
                self.runtime_sha256 = runtime_sha256

            def command(self, role: str, remote: str, timeout: int = 60) -> str:
                del timeout
                self.calls.append((role, remote))
                if "service-readback" in remote:
                    return json.dumps(
                        {
                            "independently_observed": True,
                            "active": True,
                            "running": True,
                            "service_state": "running",
                            "listeners": [],
                        }
                    )
                if "/healthz" in remote:
                    return json.dumps(
                        {
                            "ok": True,
                            "ready": True,
                            "last_error": None,
                            "nrestarts": 0,
                            "oom_panic_segfault": False,
                        }
                    )
                if "sha256sum" in remote:
                    return self.runtime_sha256
                raise AssertionError(f"unexpected remote command: {remote}")

        transaction = self._base_plan()
        inventory = {
            "nodes": {
                "storage-205": {"service": "oasis7-triad-storage.service"},
                "sequencer-204": {"service": "oasis7-triad-sequencer.service"},
            }
        }
        transport = HealthyTransport(self.runtime_sha256)

        with (
            patch.object(ADAPTER, "validate_transaction", return_value=(inventory, self.root / "known-hosts", {}, {})),
            patch.object(ADAPTER, "credential_fds", return_value={}),
            patch.object(ADAPTER, "FixedSSH", return_value=transport),
            patch.object(ADAPTER, "base_receipt", return_value={}),
        ):
            receipt = ADAPTER.run_phase(transaction, "staggered-preflight")

        self.assertEqual(receipt["staggered_phase"], "preflight")
        self.assertTrue(receipt["live_baseline"])
        self.assertEqual(set(receipt["nodes"]), set(MODULE.MUTATION_ORDER))
        for node in receipt["nodes"].values():
            self.assertTrue(node["healthz_ok"])
            self.assertTrue(node["ready"])
            self.assertIsNone(node["last_error"])
            self.assertEqual(node["nrestarts"], 0)
            self.assertFalse(node["oom_panic_segfault"])
            self.assertFalse(node["preflight_observer_mutation"])
        for role, expected_url in (
            ("storage-205", transaction["proof"]["storage_health_url"]),
            ("sequencer-204", transaction["proof"]["sequencer_health_url"]),
        ):
            role_calls = [remote for called_role, remote in transport.calls if called_role == role]
            self.assertEqual(sum("service-readback" in remote for remote in role_calls), 1)
            health_calls = [remote for remote in role_calls if "/healthz" in remote]
            self.assertEqual(len(health_calls), 1)
            self.assertIn(expected_url, health_calls[0])

    def test_package_hash_and_commit_are_bound_and_fail_closed(self) -> None:
        for variable, pattern in (
            ("O7_ADAPTER_BAD_PACKAGE_HASH", r"(?i)(package|sha256|hash)"),
            ("O7_ADAPTER_BAD_PACKAGE_COMMIT", r"(?i)(package|commit|provenance)"),
            ("O7_ADAPTER_BAD_PACKAGE_RUN_ID", r"(?i)(package|run|provenance)"),
        ):
            with self.subTest(variable=variable):
                error, persisted = self._invoke_with_env_failure(
                    "staggered-storage", **{variable: "1"}
                )
                self.assertRegex(str(error), pattern)
                self.assertEqual(persisted["adapter_callback"]["status"], "failed")

    def test_rollback_is_target_only_and_rejects_restore(self) -> None:
        receipt, persisted, _ = self._invoke("staggered-rollback")
        self.assertTrue(receipt["target_only_cleanup"])
        self.assertFalse(receipt["restore_deleted_chain_state"])
        self.assertEqual(persisted["adapter_callback"]["status"], "completed")

        error, persisted = self._invoke_with_env_failure(
            "staggered-rollback", O7_ADAPTER_BAD_ROLLBACK="1"
        )
        self.assertRegex(str(error), r"(?i)(target-only|restore|rollback)")
        self.assertEqual(persisted["adapter_callback"]["status"], "failed")

    def test_adapter_nonzero_exit_is_durable_and_fail_closed(self) -> None:
        error, persisted = self._invoke_with_env_failure(
            "staggered-storage", O7_ADAPTER_FAIL_PHASE="staggered-storage"
        )
        self.assertRegex(str(error), r"(?i)(host adapter failed|exit 17)")
        callback = persisted["adapter_callback"]
        self.assertEqual(callback["status"], "failed")
        self.assertIn("exit 17", callback["error"])
        self.assertNotIn("receipt", callback)


if __name__ == "__main__":
    unittest.main(verbosity=2)
