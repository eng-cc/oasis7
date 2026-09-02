#!/usr/bin/env python3
"""Contract tests for the governed full-network clean-room plan."""

from __future__ import annotations

import importlib.util
import hashlib
import json
import subprocess
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch


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
        self._impact_directory = tempfile.TemporaryDirectory()
        self._impact_path = Path(self._impact_directory.name) / "consumer-impact.json"

    def tearDown(self) -> None:
        self._impact_directory.cleanup()

    def _consumer_impact_reference(self) -> dict[str, str]:
        record = {
            "impact": "none",
            "evidence_source": "test-fixture-direct-observation",
            "timestamp": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
            "validators_already_stopped": False,
            "outage_update_channel": "n/a",
            "recovery_update_checkpoint": "n/a",
            "producer_wording_approval": "n/a",
            "decision": "proceed",
        }
        payload = json.dumps(record, sort_keys=True, separators=(",", ":")).encode("utf-8")
        self._impact_path.write_bytes(payload)
        return {"path": str(self._impact_path), "sha256": hashlib.sha256(payload).hexdigest()}

    def _rewrite_consumer_impact(self, request: dict[str, object], **changes: object) -> None:
        record = json.loads(self._impact_path.read_text(encoding="utf-8"))
        record.update(changes)
        payload = json.dumps(record, sort_keys=True, separators=(",", ":")).encode("utf-8")
        self._impact_path.write_bytes(payload)
        reference = {"path": str(self._impact_path), "sha256": hashlib.sha256(payload).hexdigest()}
        request["consumer_impact_record"] = reference
        request["authority"]["consumer_impact_record"] = dict(reference)

    @staticmethod
    def _receipt(schema: str = "oasis7.authenticated_receipt.v1") -> dict[str, object]:
        return {
            "schema_version": schema,
            "verified": True,
            "authenticated": True,
            "signer_id": "governance-signer",
            "verifier_id": "governed-receipt-verifier",
            "trust_root_id": "oasis7-public-testnet-governance-root-v1",
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

    def _no_backup_receipt(self, request: dict[str, object], expires_at: str) -> dict[str, object]:
        receipt = self._receipt("oasis7.no_backup_authority.v1")
        receipt["bindings"] = {
            "repository": "eng-cc/oasis7",
            "action": "full-network-clean-room",
            "targets": list(self.module.NODE_ORDER),
            "task_uid": request["task_uid"],
            "transaction_id": request["transaction_id"],
            "capture_window_id": request["capture_window_id"],
            "frozen_head_oid": request["head_oid"],
            "actor": "ops-actor",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": expires_at,
            "current_authorization": True,
            "consumer_impact_record": {
                "path": request["consumer_impact_record"]["path"],
                "sha256": request["consumer_impact_record"]["sha256"],
            },
        }
        return receipt

    @staticmethod
    def _windows_state_path(surface: str) -> str:
        surface = surface.replace("{node_id}", "triad-testnet-windows-observer")
        return rf"C:\\oasis7-deploy\\{surface.replace('/', chr(92))}"

    def _deployment_inventory(
        self,
        nodes: list[dict[str, object]],
        *,
        expected_uid: int = 0,
        expected_gid: int = 0,
        include_layout: bool = True,
    ) -> dict[str, object]:
        inventory_nodes: dict[str, dict[str, object]] = {}
        for node in nodes:
            entry: dict[str, object] = {
                "node_id": node["node_id"],
                "expected_key_uid": expected_uid,
                "expected_key_gid": expected_gid,
                "peer_id": self.module.CANONICAL_PEER_REGISTRY[str(node["name"])],
            }
            if include_layout:
                entry.update(
                    {
                        "node_root": node["node_root"],
                        "persistent_state_paths": list(node["persistent_state_paths"]),
                    }
                )
                node_name = str(node["name"])
                path_style = (
                    "windows"
                    if self.module.EXPECTED_NODES[node_name]["platform"] == "windows-x64"
                    else "posix"
                )
                entry["node_root"] = self.module._normalized_path(
                    entry["node_root"], path_style, f"{node['name']}.node_root"
                )
                entry["persistent_state_paths"] = [
                    self.module._normalized_path(
                        path,
                        path_style,
                        f"{node['name']}.persistent_state_paths[{index}]",
                    )
                    for index, path in enumerate(entry["persistent_state_paths"])
                ]
            inventory_nodes[str(node["name"])] = entry
        inventory = {
            "schema_version": "oasis7.deployment_inventory.v1",
            "authenticated": True,
            "verified": True,
            "signer_id": "governance-signer",
            "trust_root_id": "oasis7-public-testnet-governance-root-v1",
            "nodes": inventory_nodes,
            "receipt": self._receipt("oasis7.deployment_inventory_receipt.v1"),
        }
        inventory["receipt"]["signed_payload_sha256"] = (
            self.module._canonical_deployment_inventory_payload_digest(inventory)
        )
        return inventory

    def _input(self) -> dict[str, object]:
        transaction_id = "txn-clean-room-001"
        capture_window_id = "capture-window-20260901-001"
        task_uid = "task_174f0a5a87394012b071171cc4a52372"
        head_oid = "e" * 40
        network_id = "oasis7-public-testnet-governed-20260606"
        authority_bindings = {
            "task_uid": task_uid,
            "head_oid": head_oid,
            "network_id": network_id,
            "signer_allowlist": ["governance-signer"],
            "trust_root_id": "oasis7-public-testnet-governance-root-v1",
            "verifier_id": "governed-receipt-verifier",
        }
        consumer_impact_record = self._consumer_impact_reference()
        authority_bindings["consumer_impact_record"] = dict(consumer_impact_record)
        execution_bindings = {
            "execution_records_root": {
                "path": "/operator/truth/execution-records/root",
                "sha256": "a" * 64,
                "size_bytes": 16384,
            },
            "cas": {
                "root": "/operator/truth/execution-cas",
                "blake3": "b" * 64,
                "size_bytes": 32768,
            },
            "world_head": {
                "path": "/operator/truth/world-head.json",
                "sha256": "c" * 64,
                "size_bytes": 1024,
                "height": 100,
                "block_hash": "8" * 64,
                "state_root": "9" * 64,
            },
            "generated_world_sidecar": {
                "path": "/operator/truth/world/.distfs-state/sidecar-generations/index.json",
                "sha256": "d" * 64,
                "size_bytes": 4096,
                "provenance_path": "/operator/truth/world/generated-world.provenance.json",
                "provenance_sha256": "4" * 64,
                "provenance_size_bytes": 256,
            },
            "json_index_consistency": {
                "verified": True,
                "snapshot_sha256": "e" * 64,
                "snapshot_size_bytes": 8192,
                "journal_sha256": "f" * 64,
                "journal_size_bytes": 16384,
                "index_sha256": "0" * 64,
                "index_size_bytes": 4096,
            },
        }
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
                "network_id": network_id,
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
            "execution": execution_bindings,
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
                    "known_hosts_path": "/opt/oasis7/p2p-testnet/config/public-testnet-validator-pair-known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:6631/healthz",
                    "evidence": "http://127.0.0.1:6631/v1/chain/rebuild-proof",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_SEQUENCER_SSHPASS",
                    "nonce": "sequencer-nonce-" + "x" * 32,
                    "issued_at": "2026-08-30T00:00:00Z",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "ledger_path": "/operator/credential-nonce-ledger.jsonl",
                    "one_shot": True,
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
                    "known_hosts_path": "/opt/oasis7/p2p-testnet/config/public-testnet-validator-pair-known-hosts",
                },
                "endpoints": {
                    "healthz": "http://127.0.0.1:6632/healthz",
                    "evidence": "http://127.0.0.1:6632/v1/chain/status",
                },
                "credential_seam": {
                    "kind": "temporary-fd-or-environment",
                    "environment_name": "PUBLIC_TESTNET_STORAGE_SSHPASS",
                    "nonce": "storage-nonce-" + "x" * 32,
                    "issued_at": "2026-08-30T00:00:00Z",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "ledger_path": "/operator/credential-nonce-ledger.jsonl",
                    "one_shot": True,
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
                    "issued_at": "2026-08-30T00:00:00Z",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "ledger_path": "/operator/credential-nonce-ledger.jsonl",
                    "one_shot": True,
                },
                "persistent_state_paths": [
                    f"/opt/oasis7/p2p-testnet-local/{surface.replace('{node_id}', 'triad-testnet-local')}"
                    for surface in self.module.LINUX_OBSERVER_PERSISTENT_STATE_SURFACES
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
                    "issued_at": "2026-08-30T00:00:00Z",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "ledger_path": "/operator/credential-nonce-ledger.jsonl",
                    "one_shot": True,
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
                    "issued_at": "2026-08-30T00:00:00Z",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "ledger_path": "/operator/credential-nonce-ledger.jsonl",
                    "one_shot": True,
                },
                "persistent_state_paths": [
                    f"/Applications/oasis7/{surface.replace('{node_id}', 'triad-testnet-fourth-local')}"
                    for surface in self.module.OBSERVER_RESET_SURFACES
                ],
                "identity_receipt": self._identity_receipt("triad-testnet-fourth-local"),
            },
        ]
        deployment_inventory = self._deployment_inventory(nodes)
        return {
            "schema_version": "oasis7.public_testnet_full_network_clean_room_input.v1",
            "transaction_id": transaction_id,
            "capture_window_id": capture_window_id,
            "task_uid": task_uid,
            "head_oid": head_oid,
            "authority": {
                "authorized": True,
                "task_uid": task_uid,
                "head_oid": head_oid,
                "frozen_head_oid": head_oid,
                "consumer_impact_record": consumer_impact_record,
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
                    "bindings": json.loads(json.dumps(execution_bindings)),
                },
                "trust_root": {
                    **self._receipt("oasis7.governed_trust_root_receipt.v1"),
                    "trust_root_id": "oasis7-public-testnet-governance-root-v1",
                    "verifier_id": "governed-receipt-verifier",
                    "signer_allowlist": ["governance-signer"],
                    "bindings": authority_bindings,
                },
                "receipt": {
                    **self._receipt("oasis7.clean_room_authority.v1"),
                    "bindings": authority_bindings,
                },
            },
            "consumer_impact_record": consumer_impact_record,
            "truth": truth,
            "fresh_root_probe": {
                "schema_version": "oasis7.fresh_root_probe.v1",
                "verified": True,
                "authenticated": True,
                "package_commit": "d" * 40,
                "checkpoint_id": "checkpoint-001",
                "manifest_hash": "5" * 64,
                "height": 100,
                "transaction_id": transaction_id,
                "capture_window_id": capture_window_id,
                "replayed": False,
                "post_validator_verify": True,
                "validator_verify_outputs": {
                    name: {
                        "schema_version": "oasis7.validator_verify_output.v1",
                        "authenticated": True,
                        "verified": True,
                        "signer_id": "governance-signer",
                        "signed_payload_sha256": "a" * 64,
                        "signature_hex": "b" * 128,
                        "canonical_digest": "c" * 64,
                        "node": name,
                        "transaction_id": transaction_id,
                        "capture_window_id": capture_window_id,
                        "package_commit": "d" * 40,
                        "checkpoint_id": "checkpoint-001",
                        "manifest_hash": "5" * 64,
                        "height": 100,
                        "output_sha256": "6" * 64,
                    }
                    for name in ("sequencer-204", "storage-205")
                },
                "receipt": self._receipt("oasis7.fresh_root_probe_receipt.v1"),
            },
            "credential_nonce_ledger": {
                "schema_version": "oasis7.credential_nonce_ledger.v1",
                "path": "/operator/credential-nonce-ledger.jsonl",
                "transaction_id": transaction_id,
                "capture_window_id": capture_window_id,
                "one_shot": True,
                "replay": False,
                "issued_at": "2026-08-30T00:00:00Z",
                "expires_at": "2099-01-01T00:00:00Z",
                "reserved_nonces": [
                    "storage-nonce-" + "x" * 32,
                    "sequencer-nonce-" + "x" * 32,
                    "linux-observer-nonce-" + "x" * 32,
                    "windows-observer-nonce-" + "x" * 32,
                    "macos-observer-nonce-" + "x" * 32,
                ],
                "receipt": {
                    **self._receipt("oasis7.credential_nonce_ledger_receipt.v1"),
                    "bindings": {
                        "path": "/operator/credential-nonce-ledger.jsonl",
                        "transaction_id": transaction_id,
                        "capture_window_id": capture_window_id,
                        "one_shot": True,
                        "replay": False,
                        "issued_at": "2026-08-30T00:00:00Z",
                        "expires_at": "2099-01-01T00:00:00Z",
                        "reserved_nonces": [
                            "storage-nonce-" + "x" * 32,
                            "sequencer-nonce-" + "x" * 32,
                            "linux-observer-nonce-" + "x" * 32,
                            "windows-observer-nonce-" + "x" * 32,
                            "macos-observer-nonce-" + "x" * 32,
                        ],
                    },
                },
            },
            "adapter_verification": {
                "schema_version": "oasis7.clean_room_adapter_verification.v1",
                "authenticated": True,
                "verified": True,
                "signer_id": "governance-signer",
                "signed_payload_sha256": "a" * 64,
                "signature_hex": "b" * 128,
                "canonical_digest": "c" * 64,
                "adapter_id": "external-clean-room-adapter",
                "transaction_id": transaction_id,
                "capture_window_id": capture_window_id,
                "live_receipts_verified": True,
                "credential_nonce_ledger_verified": True,
                "backup_or_no_backup_authority_verified": True,
                "apply_authority_granted": False,
                "durable_journal_authoritative": False,
                "durable_journal_receipt_required": True,
                "receipt": self._receipt("oasis7.clean_room_adapter_verification_receipt.v1"),
            },
            "deployment_inventory": deployment_inventory,
            "nodes": nodes,
        }

    def test_plan_emits_fixed_five_node_order_and_8_7_surfaces(self) -> None:
        plan = self.module.build_plan(self._input())

        self.assertEqual(
            plan["consumer_impact_record"]["path"], str(self._impact_path)
        )
        self.assertEqual(
            plan["authority"]["consumer_impact_record"], plan["consumer_impact_record"]
        )

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
        self.assertEqual(
            plan["capture_window"],
            {
                "id": plan["capture_window_id"],
                "starts_at": plan["credential_nonce_ledger"]["issued_at"],
                "ends_at": plan["credential_nonce_ledger"]["expires_at"],
            },
        )

    def test_linux_observer_surfaces_match_managed_reset_layout(self) -> None:
        plan = self.module.build_plan(self._input())
        observer = next(
            node for node in plan["nodes"] if node["name"] == "linux-lan-observer"
        )
        root = "/opt/oasis7/p2p-testnet-local"
        required_paths = {
            f"{root}/world",
            f"{root}/world-simulator-mirror",
            f"{root}/execution-records",
            f"{root}/store",
            f"{root}/replication-root",
            f"{root}/runtime-root",
            f"{root}/output/chain-runtime/triad-testnet-local/reward-runtime-execution-bridge-state.json",
            f"{root}/output/node-distfs/triad-testnet-local",
        }
        self.assertTrue(
            required_paths.issubset(set(observer["persistent_state_paths"])),
            observer["persistent_state_paths"],
        )

    def test_service_account_ownership_requires_independent_deployment_truth(self) -> None:
        """An observed equal uid/gid pair cannot define its own expectation."""
        request = self._input()
        inventory_nodes = {
            node["name"]: {
                "expected_key_uid": 1001,
                "expected_key_gid": 1001,
            }
            for node in request["nodes"]
        }
        request["deployment_inventory"] = {
            "schema_version": "oasis7.deployment_inventory.v1",
            "authenticated": True,
            "verified": True,
            "signer_id": "governance-signer",
            "trust_root_id": "oasis7-public-testnet-governance-root-v1",
            "nodes": inventory_nodes,
            "receipt": self._receipt("oasis7.deployment_inventory_receipt.v1"),
        }
        for node in request["nodes"]:
            node["identity_receipt"]["key_uid"] = 4242
            node["identity_receipt"]["key_gid"] = 4242

        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)(expected|deployment|uid|gid|owner)")

    def test_macos_observer_uses_authenticated_inventory_root_and_surfaces(self) -> None:
        """macOS reset targets must come from authenticated deployment inventory."""
        request = self._input()
        root = "/Users/operator/oasis7-fourth"
        declared_paths = [
            f"{root}/world",
            f"{root}/world-simulator-mirror",
            f"{root}/execution-records",
            f"{root}/store",
            f"{root}/replication-root",
            f"{root}/runtime-root",
            f"{root}/output/chain-runtime/triad-testnet-fourth-local/reward-runtime-execution-bridge-state.json",
            f"{root}/output/node-distfs/triad-testnet-fourth-local",
        ]
        request["deployment_inventory"] = self._explicit_inventory(request)
        request["deployment_inventory"]["nodes"]["macos-observer"].update(
            {"node_root": root, "persistent_state_paths": declared_paths}
        )
        request["deployment_inventory"]["receipt"]["signed_payload_sha256"] = (
            self.module._canonical_deployment_inventory_payload_digest(
                request["deployment_inventory"]
            )
        )

        plan = self.module.build_plan(request)
        macos = next(node for node in plan["nodes"] if node["name"] == "macos-observer")
        self.assertEqual(macos["node_root"], root)
        self.assertEqual(macos["persistent_state_paths"], declared_paths)

    def test_observer_surface_summary_matches_governed_node_inventory(self) -> None:
        """The exported observer summary must include the governed eight paths."""
        plan = self.module.build_plan(self._input())
        linux = next(
            node for node in plan["nodes"] if node["name"] == "linux-lan-observer"
        )
        summary = plan["surfaces"]
        self.assertEqual(summary["observer_count"], len(linux["persistent_state_paths"]))
        self.assertEqual(summary["observer_count"], 8)
        self.assertIn("observers_by_node", summary)
        self.assertEqual(
            summary["observers_by_node"]["linux-lan-observer"],
            linux["persistent_state_paths"],
        )

    def test_plan_requires_explicit_authenticated_deployment_inventory(self) -> None:
        """Release admission cannot silently replace absent deployment truth."""
        request = self._input()
        request.pop("deployment_inventory")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)deployment|inventory|missing|authenticated")

    def test_plan_allows_independent_authenticated_uid_and_gid_truth(self) -> None:
        """Deployment truth may authenticate distinct service UID and primary GID."""
        request = self._input()
        request["deployment_inventory"] = self._deployment_inventory(
            request["nodes"], expected_uid=1001, expected_gid=1002
        )
        for node in request["nodes"]:
            node["identity_receipt"]["key_uid"] = 1001
            node["identity_receipt"]["key_gid"] = 1002

        plan = self.module.build_plan(request)
        for node in plan["nodes"]:
            inventory_node = plan["deployment_inventory"]["nodes"][node["name"]]
            self.assertEqual(inventory_node["expected_key_uid"], 1001)
            self.assertEqual(inventory_node["expected_key_gid"], 1002)
            self.assertEqual(node["identity_receipt"]["key_uid"], 1001)
            self.assertEqual(node["identity_receipt"]["key_gid"], 1002)

    def test_plan_requires_authenticated_root_and_reset_surfaces_per_managed_node(self) -> None:
        """Inventory layout is required per node; code-owned defaults are not evidence."""
        for node_name in self.module.NODE_ORDER:
            for field in ("node_root", "persistent_state_paths"):
                with self.subTest(node=node_name, field=field):
                    request = self._input()
                    inventory = self._deployment_inventory(request["nodes"])
                    inventory["nodes"][node_name].pop(field)
                    request["deployment_inventory"] = inventory
                    with self.assertRaises(SystemExit) as raised:
                        self.module.build_plan(request)
                    self.assertRegex(str(raised.exception), r"(?i)deployment|inventory|root|surface|path")

    def test_plan_requires_complete_canonical_state_surfaces_per_managed_node(self) -> None:
        """Authenticated state inventory must cover every role/platform surface."""
        for node_name in self.module.NODE_ORDER:
            for omission in ("sparse", "nested"):
                with self.subTest(node=node_name, omission=omission):
                    request = self._input()
                    inventory = self._deployment_inventory(request["nodes"])
                    node = next(item for item in request["nodes"] if item["name"] == node_name)
                    full_paths = list(node["persistent_state_paths"])
                    if omission == "sparse":
                        incomplete_paths = [full_paths[0]]
                    else:
                        incomplete_paths = full_paths[:2] + full_paths[3:]
                    inventory["nodes"][node_name]["persistent_state_paths"] = incomplete_paths
                    node["persistent_state_paths"] = incomplete_paths
                    request["deployment_inventory"] = inventory
                    with self.assertRaises(SystemExit) as raised:
                        self.module.build_plan(request)
                    self.assertRegex(str(raised.exception), r"(?i)surface|canonical|complete|path")

    def test_plan_enforces_component_aware_windows_state_containment(self) -> None:
        """Windows roots are components: root and sibling-prefix paths are not surfaces."""
        windows_name = "windows-observer"
        windows_index = next(
            index for index, node in enumerate(self._input()["nodes"])
            if node["name"] == windows_name
        )
        for label, invalid_path in (
            ("exact-root", "C:/oasis7-deploy"),
            ("sibling-prefix", "C:/oasis7-deploy-evil/state"),
        ):
            with self.subTest(path=label):
                request = self._input()
                inventory = self._deployment_inventory(request["nodes"])
                inventory["nodes"][windows_name]["persistent_state_paths"] = [invalid_path]
                request["nodes"][windows_index]["persistent_state_paths"] = [invalid_path]
                request["deployment_inventory"] = inventory
                with self.assertRaises(SystemExit) as raised:
                    self.module.build_plan(request)
                self.assertRegex(str(raised.exception), r"(?i)root|surface|path|contain")

        # Existing complete inventory paths are genuine descendants and remain accepted.
        self.module.build_plan(self._input())

    def test_plan_rejects_duplicate_authenticated_peer_ids(self) -> None:
        """Distinct managed nodes cannot share one authenticated peer identity."""
        request = self._input()
        request["nodes"][1]["identity_receipt"]["peer_id"] = request["nodes"][0]["identity_receipt"]["peer_id"]
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)peer|identity|duplicate|unique")

    def test_plan_rejects_unique_peer_ids_outside_authenticated_registry(self) -> None:
        """Peer uniqueness alone cannot authorize a caller-supplied identity."""
        for node in self._input()["nodes"]:
            with self.subTest(node=node["name"]):
                request = self._input()
                target = next(item for item in request["nodes"] if item["name"] == node["name"])
                target["identity_receipt"]["peer_id"] = (
                    f"12D3KooWcaller-supplied-{node['name']}"
                )
                with self.assertRaises(SystemExit) as raised:
                    self.module.build_plan(request)
                self.assertRegex(str(raised.exception), r"(?i)peer|identity|registry|canonical|binding")

    def test_plan_requires_fresh_bound_consumer_impact_record(self) -> None:
        request = self._input()
        request.pop("consumer_impact_record")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)consumer|impact|record|binding")

        request = self._input()
        self._rewrite_consumer_impact(request, timestamp="2020-01-01T00:00:00Z")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)consumer|impact|stale|fresh|timestamp")

        request = self._input()
        self._rewrite_consumer_impact(request, impact="active", outage_update_channel="n/a")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)consumer|impact|outage|channel|n/a")

        request = self._input()
        request["consumer_impact_record"]["sha256"] = "0" * 64
        request["authority"]["consumer_impact_record"] = dict(request["consumer_impact_record"])
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)consumer|impact|sha|digest|binding")

    def test_observer_destructive_phases_follow_fresh_root_probe(self) -> None:
        plan = self.module.build_plan(self._input())
        order = plan["global_order"]
        probe_index = order.index("fresh-root-probe")
        for name in ("linux-lan-observer", "windows-observer", "macos-observer"):
            for phase in ("stop", "delete", "rebuild"):
                self.assertGreater(order.index(f"{phase}:{name}"), probe_index)

        invalid = list(order)
        invalid.remove("stop:windows-observer")
        invalid.insert(probe_index, "stop:windows-observer")
        with patch.object(self.module, "_global_order", return_value=invalid):
            with self.assertRaises(SystemExit) as raised:
                self.module.build_plan(self._input())
        self.assertRegex(str(raised.exception), r"(?i)order|probe|observer|destructive")

    def test_plan_rejects_attacker_endpoint_host_port_or_path(self) -> None:
        request = self._input()
        request["nodes"][0]["endpoints"]["healthz"] = "http://attacker.invalid:6631/healthz"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)canonical|endpoint|host|port|binding")

    def test_plan_requires_code_owned_external_trust_root_and_receipt_bindings(self) -> None:
        request = self._input()
        request["authority"]["trust_root"]["trust_root_id"] = "caller-owned-root"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)trust|root|authority|identity")

        request = self._input()
        request["authority"]["receipt"]["bindings"]["head_oid"] = "f" * 40
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)authority|head|receipt|binding")

        request = self._input()
        request["truth"]["genesis"]["network_id"] = "caller-owned-network"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)network|genesis|trust|code-owned")

    def test_plan_requires_adapter_live_receipt_and_never_treats_plan_as_apply_proof(self) -> None:
        request = self._input()
        request.pop("adapter_verification")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)adapter|live|receipt|ledger")

        request = self._input()
        request["adapter_verification"]["apply_authority_granted"] = True
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)apply|proof|adapter|authority")

    def test_plan_marks_operation_journal_non_authoritative_and_adapter_owned(self) -> None:
        plan = self.module.build_plan(self._input())
        contract = plan["operation_journal_contract"]
        self.assertFalse(contract["authoritative"])
        self.assertFalse(contract["apply_usable"])
        self.assertTrue(contract["adapter_owned"])
        self.assertTrue(contract["durable_receipt_required"])

    def test_plan_rejects_missing_fresh_root_probe_before_windows_or_macos(self) -> None:
        request = self._input()
        request.pop("fresh_root_probe")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)fresh[_-]?root.*probe")

    def test_plan_rejects_shaped_but_unverified_inventory_identity_and_authority_receipts(self) -> None:
        """Receipt-shaped fields are not independent verification evidence."""
        mutations = (
            (
                "deployment-inventory-signature",
                lambda request: request["deployment_inventory"]["receipt"].__setitem__(
                    "signature_hex", "0" * 128
                ),
            ),
            (
                "deployment-inventory-digest",
                lambda request: request["deployment_inventory"]["receipt"].__setitem__(
                    "canonical_digest", "0" * 64
                ),
            ),
            (
                "identity-signature",
                lambda request: request["nodes"][0]["identity_receipt"].__setitem__(
                    "signature_hex", "0" * 128
                ),
            ),
            (
                "identity-digest",
                lambda request: request["nodes"][0]["identity_receipt"].__setitem__(
                    "canonical_digest", "0" * 64
                ),
            ),
            (
                "authority-verifier-signature",
                lambda request: request["authority"]["crypto_verifier_receipt"].__setitem__(
                    "signature_hex", "0" * 128
                ),
            ),
        )
        for label, mutate in mutations:
            with self.subTest(receipt=label):
                request = self._input()
                mutate(request)
                with self.assertRaises(SystemExit) as raised:
                    self.module.build_plan(request)
                self.assertRegex(
                    str(raised.exception),
                    r"(?i)receipt|signature|digest|verified|authenticated|verifier",
                )

    def test_plan_rejects_authority_binding_context_drift_and_stale_or_future_receipts(self) -> None:
        """Authority receipts bind task/head/window/rotation and a bounded issue window."""
        valid = self._input()
        valid_context = {
            "capture_window_id": valid["capture_window_id"],
            "rotation_epoch": "rotation-epoch-20260901-001",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": "2099-01-01T00:00:00Z",
        }
        for container in (
            valid["authority"]["receipt"]["bindings"],
            valid["authority"]["trust_root"]["bindings"],
        ):
            container.update(valid_context)
        self.module.build_plan(valid)

        mutations = (
            ("task-mismatch", {"task_uid": "task-attacker"}),
            ("head-mismatch", {"head_oid": "f" * 40}),
            ("frozen-head-mismatch", {"frozen_head_oid": "f" * 40}),
            ("capture-window-mismatch", {"capture_window_id": "other-window"}),
            ("rotation-epoch-mismatch", {"rotation_epoch": "rotation-attacker"}),
            (
                "stale-authority",
                {"issued_at": "2020-01-01T00:00:00Z", "expires_at": "2020-01-02T00:00:00Z"},
            ),
            (
                "future-authority",
                {"issued_at": "2099-01-01T00:00:00Z", "expires_at": "2100-01-01T00:00:00Z"},
            ),
        )
        for label, updates in mutations:
            with self.subTest(binding=label):
                request = self._input()
                for container in (
                    request["authority"]["receipt"]["bindings"],
                    request["authority"]["trust_root"]["bindings"],
                ):
                    container.update(valid_context)
                    container.update(updates)
                with self.assertRaises(SystemExit) as raised:
                    self.module.build_plan(request)
                self.assertRegex(
                    str(raised.exception),
                    r"(?i)authority|binding|capture|rotation|task|head|stale|future|expir",
                )

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
        request["deployment_inventory"]["nodes"]["linux-lan-observer"]["persistent_state_paths"] = [
            "/operator/not-the-authenticated-observer-root"
        ]
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
            "current_authorization": True,
            "repository": "eng-cc/oasis7",
            "action": "full-network-clean-room",
            "targets": list(self.module.NODE_ORDER),
            "transaction_id": request["transaction_id"],
            "capture_window_id": request["capture_window_id"],
            "actor": "ops-actor",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": "2099-01-01T00:00:00Z",
            "task_uid": request["task_uid"],
            "frozen_head_oid": request["head_oid"],
            "reason": "immutable provider backup unavailable",
            "authority": self._no_backup_receipt(request, "2099-01-01T00:00:00Z"),
        }
        plan = self.module.build_plan(request)
        self.assertFalse(plan["forensic_backup"]["required_before_reset"])
        self.assertEqual(plan["forensic_backup"]["mode"], "operator-authorized-no-backup")
        self.assertFalse(plan["forensic_backup"]["immutable"])
        self.assertFalse(plan["forensic_backup"]["receipt_required_per_node"])

    def test_plan_rejects_caller_inventory_override(self) -> None:
        request = self._input()
        request["nodes"][0]["host_binding"]["known_hosts_path"] = "/operator/other-known-hosts"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)canonical|inventory|known.?hosts|host")

    def test_verifier_receipt_binds_execution_world_and_json_index_evidence(self) -> None:
        request = self._input()
        request["authority"]["crypto_verifier_receipt"]["bindings"]["cas"]["blake3"] = "0" * 64
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)verifier|execution|cas|binding")

        request = self._input()
        request["truth"]["execution"]["json_index_consistency"]["verified"] = False
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)json|index|consistency|execution")

    def test_probe_binds_transaction_capture_and_post_validator_outputs(self) -> None:
        request = self._input()
        request["fresh_root_probe"]["transaction_id"] = "different-transaction"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)transaction|capture|probe")

        request = self._input()
        request["fresh_root_probe"]["replayed"] = True
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)replay|probe")

        request = self._input()
        request["fresh_root_probe"]["validator_verify_outputs"].pop("storage-205")
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)validator|verify|output|probe")

    def test_no_backup_authority_binds_full_context_and_expiry(self) -> None:
        request = self._input()
        request["backup_policy"] = {
            "mode": "operator-authorized-no-backup",
            "operator_authorized": True,
            "current_authorization": True,
            "repository": "eng-cc/oasis7",
            "action": "full-network-clean-room",
            "targets": list(self.module.NODE_ORDER),
            "transaction_id": request["transaction_id"],
            "capture_window_id": request["capture_window_id"],
            "actor": "ops-actor",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": "2099-01-01T00:00:00Z",
            "task_uid": request["task_uid"],
            "frozen_head_oid": request["head_oid"],
            "reason": "immutable provider backup unavailable",
            "authority": self._no_backup_receipt(request, "2099-01-01T00:00:00Z"),
        }
        request["backup_policy"]["action"] = "other-action"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)backup|authority|action|binding")

        request["backup_policy"]["action"] = "full-network-clean-room"
        request["backup_policy"]["issued_at"] = "2099-01-01T00:00:00Z"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)expir|future|backup|authority")

        request = self._input()
        request["backup_policy"] = {
            "mode": "operator-authorized-no-backup",
            "operator_authorized": True,
            "current_authorization": True,
            "repository": "eng-cc/oasis7",
            "action": "full-network-clean-room",
            "targets": list(self.module.NODE_ORDER),
            "transaction_id": request["transaction_id"],
            "capture_window_id": request["capture_window_id"],
            "actor": "ops-actor",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": "2020-01-01T00:00:00Z",
            "task_uid": request["task_uid"],
            "frozen_head_oid": request["head_oid"],
            "reason": "immutable provider backup unavailable",
            "authority": self._no_backup_receipt(request, "2020-01-01T00:00:00Z"),
        }
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)expir|backup|authority")

        request = self._input()
        request["backup_policy"] = {
            "mode": "operator-authorized-no-backup",
            "operator_authorized": True,
            "current_authorization": True,
            "repository": "eng-cc/oasis7",
            "action": "full-network-clean-room",
            "targets": list(self.module.NODE_ORDER),
            "transaction_id": request["transaction_id"],
            "capture_window_id": request["capture_window_id"],
            "actor": "ops-actor",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": "2099-01-01T00:00:00Z",
            "task_uid": request["task_uid"],
            "frozen_head_oid": request["head_oid"],
            "reason": "immutable provider backup unavailable",
            "authority": self._no_backup_receipt(request, "2099-01-01T00:00:00Z"),
        }
        request["backup_policy"]["authority"]["bindings"]["actor"] = "different-actor"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)authority|receipt|binding")

    def test_credential_lease_rejects_future_issued_at(self) -> None:
        request = self._input()
        issued_at = "2099-01-01T00:00:00Z"
        expires_at = "2100-01-01T00:00:00Z"
        ledger = request["credential_nonce_ledger"]
        ledger["issued_at"] = issued_at
        ledger["expires_at"] = expires_at
        ledger["receipt"]["bindings"]["issued_at"] = issued_at
        ledger["receipt"]["bindings"]["expires_at"] = expires_at
        for node in request["nodes"]:
            node["credential_seam"]["issued_at"] = issued_at
            node["credential_seam"]["expires_at"] = expires_at

        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)future|issued|credential|lease")

    def test_credential_nonce_ledger_is_unique_live_and_one_shot(self) -> None:
        request = self._input()
        request["credential_nonce_ledger"]["reserved_nonces"][1] = request[
            "credential_nonce_ledger"
        ]["reserved_nonces"][0]
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)nonce|ledger|unique|duplicate")

        request = self._input()
        request["nodes"][0]["credential_seam"]["expires_at"] = "2020-01-01T00:00:00Z"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)nonce|credential|expir")

        request = self._input()
        request["credential_nonce_ledger"]["replay"] = True
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)one.?shot|replay|ledger|nonce")

        request = self._input()
        request["credential_nonce_ledger"]["receipt"]["bindings"]["capture_window_id"] = "other-window"
        with self.assertRaises(SystemExit) as raised:
            self.module.build_plan(request)
        self.assertRegex(str(raised.exception), r"(?i)receipt|ledger|binding")

    def test_operation_journal_binds_transaction_capture_targets_and_truth(self) -> None:
        plan = self.module.build_plan(self._input())
        self.assertTrue(plan["operation_journal"])
        for entry in plan["operation_journal"]:
            self.assertEqual(entry["transaction_id"], "txn-clean-room-001")
            self.assertEqual(entry["capture_window_id"], "capture-window-20260901-001")
            if entry["node"] is not None:
                self.assertIn("target_root", entry)
                self.assertEqual(entry["package_commit"], "d" * 40)

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
            self.assertFalse(plan["execution"]["plan_is_apply_proof"])
            self.assertFalse(plan["execution"]["provider_mutation_performed"])

    def _explicit_inventory(self, request: dict[str, object]) -> dict[str, object]:
        inventory = self._deployment_inventory(request["nodes"])
        for node in request["nodes"]:
            name = str(node["name"])
            inventory["nodes"][name]["peer_id"] = self.module.CANONICAL_PEER_REGISTRY[name]
        inventory["receipt"]["signed_payload_sha256"] = (
            self.module._canonical_deployment_inventory_payload_digest(inventory)
        )
        return inventory

    def test_plan_requires_explicit_peer_id_on_every_inventory_node(self) -> None:
        """Legacy omitted/partial peer identities must not enter authenticated plans."""
        for omission in ("all", "storage-205"):
            with self.subTest(omission=omission):
                request = self._input()
                inventory = self._explicit_inventory(request)
                if omission == "all":
                    for node in inventory["nodes"].values():
                        node.pop("peer_id")
                else:
                    inventory["nodes"][omission].pop("peer_id")
                request["deployment_inventory"] = inventory
                with self.assertRaises(SystemExit) as raised:
                    self.module.build_plan(request)
                self.assertRegex(str(raised.exception), r"(?i)peer|inventory|explicit|complete")

    def test_plan_validates_inventory_digest_before_normalization(self) -> None:
        """Inventory field mutations must fail against the incoming stale receipt."""
        mutations = ("node_root", "persistent_state_paths", "expected_key_uid", "expected_key_gid", "peer_id")
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                request = self._input()
                inventory = self._explicit_inventory(request)
                request["deployment_inventory"] = inventory
                name = "storage-205"
                inventory_node = inventory["nodes"][name]
                node = next(item for item in request["nodes"] if item["name"] == name)
                if mutation in {"node_root", "persistent_state_paths"}:
                    old_root = node["node_root"]
                    new_root = "/opt/oasis7/attacker-root"
                    node["node_root"] = new_root
                    node["persistent_state_paths"] = [
                        path.replace(old_root, new_root, 1)
                        for path in node["persistent_state_paths"]
                    ]
                    inventory_node["node_root"] = new_root
                    inventory_node["persistent_state_paths"] = list(node["persistent_state_paths"])
                elif mutation == "expected_key_uid":
                    inventory_node["expected_key_uid"] = 1001
                    node["identity_receipt"]["key_uid"] = 1001
                elif mutation == "expected_key_gid":
                    inventory_node["expected_key_gid"] = 1002
                    node["identity_receipt"]["key_gid"] = 1002
                else:
                    rotated_peer = "12D3KooWstale-inventory-peer"
                    inventory_node["peer_id"] = rotated_peer
                    node["identity_receipt"]["peer_id"] = rotated_peer
                with self.assertRaises(SystemExit) as raised:
                    self.module.build_plan(request)
                self.assertRegex(str(raised.exception), r"(?i)inventory|receipt|digest|binding|peer|canonical")

    def test_plan_accepts_fully_explicit_digest_bound_inventory(self) -> None:
        """The governed explicit inventory schema remains a valid admission path."""
        request = self._input()
        request["deployment_inventory"] = self._explicit_inventory(request)
        plan = self.module.build_plan(request)
        for name in self.module.NODE_ORDER:
            self.assertIn("peer_id", plan["deployment_inventory"]["nodes"][name])
        self.assertEqual(
            plan["deployment_inventory"]["receipt"]["signed_payload_sha256"],
            self.module._canonical_deployment_inventory_payload_digest(
                plan["deployment_inventory"]
            ),
        )


if __name__ == "__main__":
    unittest.main()
