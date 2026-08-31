#!/usr/bin/env python3
"""Contract tests for the governed full-network clean-room plan."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.py"


def load_module():
    spec = importlib.util.spec_from_file_location("full_network_clean_room", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load clean-room module: {MODULE_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FullNetworkCleanRoomPlanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load_module()

    @staticmethod
    def _receipt(schema: str = "oasis7.authenticated_receipt.v1") -> dict[str, object]:
        return {
            "schema_version": schema,
            "verified": True,
            "authenticated": True,
            "signer_id": "governance-signer",
            "signed_payload_sha256": "a" * 64,
            "signature_hex": "b" * 128,
            "canonical_digest": "c" * 64,
        }

    def _identity_receipt(self, node_id: str) -> dict[str, object]:
        receipt = self._receipt("oasis7.identity_receipt.v1")
        receipt.update(
            {
                "node_id": node_id,
                "peer_id": f"12D3KooW{node_id.replace('-', '')}",
                "key_sha256": "7" * 64,
                "key_size_bytes": 128,
                "key_mode": "0600",
                "key_uid": 0,
                "key_gid": 0,
            }
        )
        return receipt

    @staticmethod
    def _windows_state_path(surface: str) -> str:
        surface = surface.replace("{node_id}", "triad-testnet-windows-observer")
        return rf"C:\\oasis7-deploy\\{surface.replace('/', chr(92))}"

    def _input(self) -> dict[str, object]:
        truth = {
            "package": {
                "package_id": "testnet-package-linux-windows-macos-001",
                "package_dir": "/operator/packages/testnet-package-linux-windows-macos-001",
                "provenance_path": "/operator/packages/testnet-package-linux-windows-macos-001/provenance.json",
                "provenance_sha256": "a" * 64,
                "provenance_size_bytes": 256,
                "commit": "d" * 40,
                "package_version": "0.0.0+testnet.001",
                "runtime_sha256": "1" * 64,
                "runtime_size_bytes": 1024,
                "genesis_sha256": "2" * 64,
                "world_sha256": "3" * 64,
                "platforms": {
                    platform: {
                        "package_sha256": "a" * 64,
                        "package_size_bytes": 4096,
                        "world_sha256": "3" * 64,
                        "world_size_bytes": 8192,
                        "world_provenance_sha256": "4" * 64,
                        "world_provenance_size_bytes": 256,
                        "commit": "d" * 40,
                    }
                    for platform in ("linux-x64", "windows-x64", "macos-arm64")
                },
                "receipt": self._receipt("oasis7.package_provenance.v1"),
            },
            "genesis": {
                "network_id": "oasis7-public-testnet-governed-20260606",
                "chain_id": "oasis7-public-testnet-governed-20260606",
                "world_id": "oasis7-public-testnet-governed-20260606",
                "path": "/operator/truth/genesis.json",
                "size_bytes": 2048,
                "sha256": "2" * 64,
                "receipt": self._receipt("oasis7.genesis_binding.v1"),
            },
            "world": {
                "world_id": "oasis7-public-testnet-governed-20260606",
                "generation": "gen-001",
                "path": "/operator/truth/world",
                "provenance_path": "/operator/truth/world-provenance.json",
                "size_bytes": 8192,
                "sha256": "3" * 64,
                "provenance_sha256": "4" * 64,
                "provenance_size_bytes": 256,
                "receipt": self._receipt("oasis7.world_binding.v1"),
            },
            "checkpoint": {
                "checkpoint_id": "checkpoint-001",
                "manifest_hash": "5" * 64,
                "height": 100,
                "receipt_path": "/operator/truth/checkpoint-receipt.json",
                "size_bytes": 512,
                "execution_block_hash": "8" * 64,
                "execution_state_root": "9" * 64,
                "sha256": "6" * 64,
                "receipt": self._receipt("oasis7.checkpoint_binding.v1"),
            },
        }
        nodes = [
            {
                "name": "sequencer-204",
                "node_id": "triad-testnet-sequencer",
                "role": "validator",
                "platform": "linux-x64",
                "node_root": "/opt/oasis7/p2p-testnet",
                "service_manager": "systemd",
                "service": "oasis7-triad-sequencer.service",
                "host_binding": {
                    "target": "root@39.104.204.172",
                    "known_host_fingerprint": "SHA256:7NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
                    "known_hosts_path": "/operator/known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:6631/healthz",
                    "evidence": "http://127.0.0.1:6631/v1/chain/rebuild-proof",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_SEQUENCER_SSHPASS",
                    "nonce": "sequencer-nonce-" + "x" * 32,
                },
                "persistent_state_paths": [
                    f"/opt/oasis7/p2p-testnet/{surface}"
                    for surface in self.module.VALIDATOR_RESET_SURFACES
                ],
                "identity_receipt": self._identity_receipt("triad-testnet-sequencer"),
            },
            {
                "name": "storage-205",
                "node_id": "triad-testnet-storage",
                "role": "validator",
                "platform": "linux-x64",
                "node_root": "/opt/oasis7/p2p-testnet",
                "service_manager": "systemd",
                "service": "oasis7-triad-storage.service",
                "host_binding": {
                    "target": "root@39.104.205.67",
                    "known_host_fingerprint": "SHA256:1SVgiaT5JLCw8PsPpVfLE9UyWNf82IJDZsiE7LAa1gI",
                    "known_hosts_path": "/operator/known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:6632/healthz",
                    "evidence": "http://127.0.0.1:6632/v1/chain/status",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_STORAGE_SSHPASS",
                    "nonce": "storage-nonce-" + "x" * 32,
                },
                "persistent_state_paths": [
                    f"/opt/oasis7/p2p-testnet/{surface}"
                    for surface in self.module.VALIDATOR_RESET_SURFACES
                ],
                "identity_receipt": self._identity_receipt("triad-testnet-storage"),
            },
            {
                "name": "linux-lan-observer",
                "node_id": "triad-testnet-local",
                "role": "observer",
                "platform": "linux-x64",
                "node_root": "/opt/oasis7/p2p-testnet-local",
                "service_manager": "systemd",
                "service": "oasis7-testnet-observer.service",
                "host_binding": {
                    "target": "observer@linux-lan",
                    "known_host_fingerprint": "SHA256:2NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
                    "known_hosts_path": "/operator/known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:6633/healthz",
                    "evidence": "http://127.0.0.1:6633/v1/chain/status",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_LINUX_OBSERVER_SSHPASS",
                    "nonce": "linux-observer-nonce-" + "x" * 32,
                },
                "persistent_state_paths": [
                    f"/opt/oasis7/p2p-testnet-local/{surface.replace('{node_id}', 'triad-testnet-local')}"
                    for surface in self.module.OBSERVER_RESET_SURFACES
                ],
                "identity_receipt": self._identity_receipt("triad-testnet-local"),
            },
            {
                "name": "windows-observer",
                "node_id": "triad-testnet-windows-observer",
                "role": "observer",
                "platform": "windows-x64",
                "node_root": r"C:\\oasis7-deploy",
                "service_manager": "scheduled-task",
                "service": "Oasis7Observer",
                "host_binding": {
                    "target": "observer@windows-lan",
                    "known_host_fingerprint": "SHA256:3NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
                    "known_hosts_path": "/operator/known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:5121/healthz",
                    "evidence": "http://127.0.0.1:5121/v1/chain/status",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_WINDOWS_OBSERVER_SSHPASS",
                    "nonce": "windows-observer-nonce-" + "x" * 32,
                },
                "persistent_state_paths": [
                    self._windows_state_path(surface)
                    for surface in self.module.OBSERVER_RESET_SURFACES
                ],
                "identity_receipt": self._identity_receipt("triad-testnet-windows-observer"),
            },
            {
                "name": "macos-observer",
                "node_id": "triad-testnet-fourth-local",
                "role": "observer",
                "platform": "macos-arm64",
                "node_root": "/Applications/oasis7",
                "service_manager": "launchd",
                "service": "oasis7.testnet.fourth",
                "host_binding": {
                    "target": "observer@macos-lan",
                    "known_host_fingerprint": "SHA256:4NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
                    "known_hosts_path": "/operator/known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:19083/healthz",
                    "evidence": "http://127.0.0.1:19083/v1/chain/status",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_MACOS_OBSERVER_SSHPASS",
                    "nonce": "macos-observer-nonce-" + "x" * 32,
                },
                "persistent_state_paths": [
                    f"/Applications/oasis7/{surface.replace('{node_id}', 'triad-testnet-fourth-local')}"
                    for surface in self.module.OBSERVER_RESET_SURFACES
                ],
                "identity_receipt": self._identity_receipt("triad-testnet-fourth-local"),
            },
        ]
        return {
            "schema_version": "oasis7.public_testnet_full_network_clean_room_input.v1",
            "task_uid": "task_174f0a5a87394012b071171cc4a52372",
            "head_oid": "e" * 40,
            "authority": {
                "authorized": True,
                "task_uid": "task_174f0a5a87394012b071171cc4a52372",
                "head_oid": "e" * 40,
                "frozen_head_oid": "e" * 40,
                "signer_allowlist": ["governance-signer"],
                "crypto_verifier_receipt": {
                    "schema_version": "oasis7.crypto_verifier_receipt.v1",
                    "authenticated": True,
                    "verified": True,
                    "signer_id": "governance-signer",
                    "signed_payload_sha256": "a" * 64,
                    "signature_hex": "b" * 128,
                    "canonical_digest": "c" * 64,
                    "algorithm": "ed25519",
                    "scope": "all-plan-receipts",
                    "verifier_id": "governed-receipt-verifier",
                    "executable_path": "/operator/bin/verify-receipt",
                    "executable_sha256": "f" * 64,
                },
                "receipt": self._receipt("oasis7.clean_room_authority.v1"),
            },
            "truth": truth,
            "fresh_root_probe": {
                "schema_version": "oasis7.fresh_root_probe.v1",
                "verified": True,
                "authenticated": True,
                "package_commit": "d" * 40,
                "checkpoint_id": "checkpoint-001",
                "manifest_hash": "5" * 64,
                "height": 100,
                "receipt": self._receipt("oasis7.fresh_root_probe_receipt.v1"),
            },
            "nodes": nodes,
        }

    def test_plan_emits_fixed_five_node_order_and_8_7_surfaces(self) -> None:
        plan = self.module.build_plan(self._input())

        self.assertEqual(
            plan["node_order"],
            [
                "storage-205",
                "sequencer-204",
                "linux-lan-observer",
                "windows-observer",
                "macos-observer",
            ],
        )
        self.assertEqual(len(plan["surfaces"]["validators"]), 8)
        self.assertEqual(len(plan["surfaces"]["observers"]), 7)
        self.assertEqual(plan["rollback"]["policy"], "clean-redeploy")
        self.assertEqual(
            plan["rollback"]["steps"],
            [
                "stop-started-nodes",
                "preserve-failed-state-for-forensics",
                "reinstall-exact-package-and-truth",
                "rerun-fresh-root-probe",
            ],
        )
        self.assertFalse(plan["rollback"]["restore_old_state"])
        self.assertFalse(plan["rollback"]["cross_node_state_copy"])
        self.assertEqual(plan["observer_gate"]["required_before"], ["windows-observer", "macos-observer"])
        self.assertLess(
            plan["global_order"].index("fresh-root-probe"),
            plan["global_order"].index("start:windows-observer"),
        )
        self.assertLess(
            plan["global_order"].index("start:windows-observer"),
            plan["global_order"].index("start:macos-observer"),
        )
        phases = [entry["phase"] for entry in plan["operation_journal"]]
        self.assertLess(phases.index("stop"), phases.index("delete"))
        self.assertLess(phases.index("delete"), phases.index("rebuild"))
        self.assertLess(phases.index("rebuild"), phases.index("start"))
        self.assertEqual(
            set(plan["truth"]["package"]["platforms"]),
            {"linux-x64", "windows-x64", "macos-arm64"},
        )

    def test_plan_rejects_missing_fresh_root_probe_before_windows_or_macos(self) -> None:
        request = self._input()
        request.pop("fresh_root_probe")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)fresh[_-]?root.*probe")

    def test_plan_rejects_old_state_restore_or_cross_node_copy(self) -> None:
        request = self._input()
        request["recovery"] = {"restore_old_state": True}
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)old.?state|seed|copy|forensic")

        request = self._input()
        request["recovery"] = {"source_node": "sequencer-204", "cross_node_state_copy": True}
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)cross.?node|copy|source")

    def test_plan_rejects_binding_drift_and_unverified_receipt(self) -> None:
        request = self._input()
        request["truth"]["checkpoint"]["manifest_hash"] = "f" * 64
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)probe|checkpoint|manifest|binding")

        request = self._input()
        request["truth"]["package"]["receipt"]["verified"] = False
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)receipt|verified|authenticated")

    def test_plan_rejects_service_path_or_identity_inventory_drift(self) -> None:
        request = self._input()
        request["nodes"][0]["service"] = "caller-owned.service"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)service|identity|contract")

        request = self._input()
        request["nodes"][2]["persistent_state_paths"] = request["nodes"][2]["persistent_state_paths"][:-1]
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)surface|persistent|state")

        request = self._input()
        request["nodes"][4]["identity_receipt"]["key_mode"] = "0644"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)key_mode|0600|identity")

    def test_plan_rejects_unbound_frozen_head_signer_or_verifier(self) -> None:
        request = self._input()
        request["authority"]["frozen_head_oid"] = "f" * 40
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)frozen|head|binding")

        request = self._input()
        request["authority"]["signer_allowlist"] = ["different-signer"]
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)signer|allow")

        request = self._input()
        request["authority"]["crypto_verifier_receipt"]["verified"] = False
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)verifier|crypt|receipt")

    def test_plan_rejects_endpoint_pin_or_nonce_drift(self) -> None:
        request = self._input()
        request["nodes"][0]["endpoints"]["evidence"] = "http://127.0.0.1:6631/v1/chain/status"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)endpoint|204|rebuild-proof|status")

        request = self._input()
        request["nodes"][1]["host_binding"]["known_host_fingerprint"] = "unbound-pin"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)host|fingerprint|pin")

        request = self._input()
        request["nodes"][2]["credential_seam"]["nonce"] = "too-short"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)nonce|credential|seam")

    def test_operator_authorized_no_backup_mode_is_explicit(self) -> None:
        request = self._input()
        request["backup_policy"] = {
            "mode": "operator-authorized-no-backup",
            "operator_authorized": True,
            "task_uid": request["task_uid"],
            "frozen_head_oid": request["head_oid"],
            "reason": "immutable provider backup unavailable",
            "authority": self._receipt("oasis7.no_backup_authority.v1"),
        }
        plan = self.module.build_plan(request)
        self.assertFalse(plan["forensic_backup"]["required_before_reset"])
        self.assertEqual(plan["forensic_backup"]["mode"], "operator-authorized-no-backup")
        self.assertFalse(plan["forensic_backup"]["immutable"])
        self.assertFalse(plan["forensic_backup"]["receipt_required_per_node"])

    def test_plan_is_deterministic_and_contains_no_secret_or_mutation_command(self) -> None:
        request = self._input()
        first = self.module.build_plan(request)
        shuffled = json.loads(json.dumps(request))
        shuffled["nodes"] = list(reversed(shuffled["nodes"]))
        second = self.module.build_plan(shuffled)
        self.assertEqual(first, second)
        serialized = json.dumps(first, sort_keys=True)
        self.assertNotRegex(serialized, r"(?i)(password=|secret-value|token-value|private.?key-bytes)")
        self.assertEqual(first["execution"]["mode"], "plan-only")
        self.assertFalse(first["execution"]["provider_mutation_performed"])

    def test_cli_writes_only_plan_artifact(self) -> None:
        request = self._input()
        with tempfile.TemporaryDirectory() as directory:
            input_path = Path(directory) / "input.json"
            output_path = Path(directory) / "plan.json"
            input_path.write_text(json.dumps(request), encoding="utf-8")
            result = subprocess.run(
                [
                    "python3",
                    str(MODULE_PATH),
                    "--input",
                    str(input_path),
                    "--out",
                    str(output_path),
                    "--json",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            plan = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(plan["execution"]["mode"], "plan-only")
            self.assertNotIn("apply", result.stdout.lower())


if __name__ == "__main__":
    unittest.main()
