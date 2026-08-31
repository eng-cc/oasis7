#!/usr/bin/env python3
"""TDD contract tests for the external full-network clean-room adapter.

These tests deliberately use the planner's synthetic, authenticated fixture.  No
provider transport is configured and no credential is read.  The adapter is
expected to remain a dry-run boundary until a separately governed transport is
supplied.
"""

from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PLANNER_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.py"
PLANNER_TEST_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.test.py"
ADAPTER_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room-adapter.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FakeTransport:
    def __init__(self) -> None:
        self.mutations: list[str] = []

    def mutate(self, operation: str, node: dict[str, object] | None) -> None:
        self.mutations.append(operation)
        raise AssertionError("dry-run must not call provider mutation")


class FullNetworkCleanRoomAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.planner = load_module("full_network_clean_room", PLANNER_PATH)
        fixture_module = load_module("full_network_clean_room_fixture", PLANNER_TEST_PATH)
        fixture = fixture_module.FullNetworkCleanRoomPlanTests()
        fixture.setUp()
        request = fixture._input()
        self.plan = self.planner.build_plan(request)
        self.adapter = load_module("full_network_clean_room_adapter", ADAPTER_PATH)

    def _authority(self) -> dict[str, object]:
        execution = copy.deepcopy(self.plan["truth"]["execution"])
        return {
            "schema_version": self.adapter.AUTHORITY_SCHEMA,
            "repository": "eng-cc/oasis7",
            "task_uid": self.plan["task_uid"],
            "frozen_head_oid": self.plan["head_oid"],
            "plan_digest": self.plan["plan_digest"],
            "adapter_id": self.adapter.CANONICAL_ADAPTER_ID,
            "network_id": self.adapter.CANONICAL_NETWORK_ID,
            "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
            "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
            "apply_authorized": False,
            "receipt": {
                "schema_version": self.adapter.CRYPTO_RECEIPT_SCHEMA,
                "authenticated": True,
                "verified": True,
                "signer_id": "governance-signer",
                "signed_payload_sha256": "a" * 64,
                "signature_hex": "b" * 128,
                "canonical_digest": "c" * 64,
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "bindings": {
                    "task_uid": self.plan["task_uid"],
                    "frozen_head_oid": self.plan["head_oid"],
                    "plan_digest": self.plan["plan_digest"],
                    "execution": execution,
                    "forensic_backup": copy.deepcopy(self.plan["forensic_backup"]),
                    "package_commit": self.plan["truth"]["package"]["commit"],
                    "checkpoint_id": self.plan["truth"]["checkpoint"]["checkpoint_id"],
                    "checkpoint_manifest_hash": self.plan["truth"]["checkpoint"]["manifest_hash"],
                },
            },
        }

    @staticmethod
    def _write_ledger(path: Path, rows: list[dict[str, object]] | None = None) -> None:
        path.write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in (rows or [])),
            encoding="utf-8",
        )
        path.chmod(0o600)

    def test_rejects_fake_head_signature_and_peer(self) -> None:
        authority = self._authority()
        authority["frozen_head_oid"] = "f" * 40
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

        authority = self._authority()
        authority["receipt"]["signature_hex"] = "0" * 128
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

        plan = copy.deepcopy(self.plan)
        plan["nodes"][0]["identity_receipt"]["peer_id"] = "12D3KooWattacker"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_plan(plan)

    def test_external_verifier_is_required_for_apply_and_binds_execution_truth(self) -> None:
        authority = self._authority()
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.execute(
                self.plan,
                authority,
                journal_path=Path(tempfile.mkdtemp()) / "journal.json",
                ledger_path=Path(tempfile.mkdtemp()) / "nonce.jsonl",
                dry_run=False,
            )

        calls: list[str] = []

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            calls.append(receipt["bindings"]["plan_digest"])
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory) / "nonce.jsonl"
            self._write_ledger(ledger)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=Path(directory) / "journal.json",
                    ledger_path=ledger,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
        self.assertEqual(calls, [self.plan["plan_digest"]])

    def test_rejects_remote_path_escape_and_pinned_host_drift(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["nodes"][0]["persistent_state_paths"][0] = "/outside/state"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_plan(plan)

        plan = copy.deepcopy(self.plan)
        plan["nodes"][0]["host_binding"]["target"] = "root@attacker.invalid"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_plan(plan)

        node = self.plan["nodes"][0]
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_remote_preflight(
                self.plan,
                node,
                {
                    "node_root": node["node_root"],
                    "persistent_state_paths": ["/outside/state"],
                    "symlink_free": True,
                    "free_bytes": 10**12,
                    "required_bytes": 1,
                    "free_inodes": 10**6,
                    "required_inodes": 1,
                    "host_target": node["host_binding"]["target"],
                    "known_hosts_path": node["host_binding"]["known_hosts_path"],
                    "known_host_fingerprint": node["host_binding"]["known_host_fingerprint"],
                },
            )

    def test_remote_preflight_requires_exact_pin_symlink_and_capacity_evidence(self) -> None:
        node = self.plan["nodes"][0]
        required_bytes, required_inodes = self.adapter.capacity_requirement(self.plan, node)
        evidence = {
            "node": node["name"],
            "node_root": node["node_root"],
            "persistent_state_paths": list(node["persistent_state_paths"]),
            "symlink_free": True,
            "free_bytes": required_bytes,
            "required_bytes": required_bytes,
            "free_inodes": required_inodes,
            "required_inodes": required_inodes,
            "host_target": node["host_binding"]["target"],
            "known_hosts_path": node["host_binding"]["known_hosts_path"],
            "known_host_fingerprint": node["host_binding"]["known_host_fingerprint"],
            "known_hosts_regular": True,
            "known_hosts_owner_uid": os.getuid(),
            "known_hosts_mode": "0600",
        }
        result = self.adapter.validate_remote_preflight(self.plan, node, evidence)
        self.assertTrue(result["known_hosts_pinned"])
        for field, bad_value in (("symlink_free", False), ("free_bytes", required_bytes - 1)):
            invalid = dict(evidence, **{field: bad_value})
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_remote_preflight(self.plan, node, invalid)

    def test_credential_ledger_is_regular_0600_and_rejects_nonce_replay(self) -> None:
        nonce = self.plan["credential_nonce_ledger"]["reserved_nonces"][0]
        row = {
            "schema_version": self.adapter.NONCE_ROW_SCHEMA,
            "transaction_id": self.plan["transaction_id"],
            "nonce": nonce,
            "one_shot": True,
        }
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory) / "nonce.jsonl"
            self._write_ledger(ledger, [row])
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_credential_ledger(self.plan, ledger)

            ledger = Path(directory) / "duplicate.jsonl"
            self._write_ledger(ledger, [dict(row, nonce="fresh-nonce"), dict(row, nonce="fresh-nonce")])
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_credential_ledger(self.plan, ledger)

            ledger = Path(directory) / "replay.jsonl"
            self._write_ledger(ledger)
            self.adapter.reserve_nonce(ledger, self.plan["transaction_id"], "one-shot-test")
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.reserve_nonce(ledger, self.plan["transaction_id"], "one-shot-test")

            ledger.chmod(0o644)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_credential_ledger(self.plan, ledger)

    def test_journal_resume_is_bound_and_ambiguous_state_is_terminal(self) -> None:
        authority = self._authority()
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            ledger = Path(directory) / "nonce.jsonl"
            self._write_ledger(ledger)
            result = self.adapter.execute(
                self.plan,
                authority,
                journal_path=journal,
                ledger_path=ledger,
                dry_run=True,
            )
            self.assertEqual(result["status"], "dry-run-complete")
            resumed = self.adapter.resume_transaction(
                self.plan,
                authority,
                journal,
                ledger_path=ledger,
                dry_run=True,
            )
            self.assertTrue(resumed["resumed"])

            record = json.loads(journal.read_text(encoding="utf-8"))
            record["status"] = "in-flight"
            record["journal_digest"] = self.adapter.journal_digest(record)
            journal.write_text(json.dumps(record) + "\n", encoding="utf-8")
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=ledger,
                    dry_run=True,
                )
            terminal = json.loads(journal.read_text(encoding="utf-8"))
            self.assertEqual(terminal["status"], "terminal-failure")

    def test_dry_run_has_deterministic_order_and_never_mutates_provider(self) -> None:
        authority = self._authority()
        transport = FakeTransport()
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory) / "nonce.jsonl"
            self._write_ledger(ledger)
            result = self.adapter.execute(
                self.plan,
                authority,
                journal_path=Path(directory) / "journal.json",
                ledger_path=ledger,
                transport=transport,
                dry_run=True,
            )
        self.assertEqual(result["operations"], self.plan["global_order"])
        self.assertEqual(transport.mutations, [])
        serialized = json.dumps(result, sort_keys=True)
        self.assertNotRegex(serialized, r"(?i)(nonce-|password=|secret-value|private.?key)")
        self.assertEqual(
            {value["receipt"]["status"] for value in result["nodes"].values()},
            {"planned"},
        )

    def test_transport_boundary_never_receives_credential_seams(self) -> None:
        node = self.plan["nodes"][0]
        transport_node = self.adapter._transport_node(node)
        transport_plan = self.adapter._transport_plan(self.plan)
        self.assertNotIn("credential_seam", transport_node)
        self.assertNotIn("credential_nonce_ledger", transport_plan)
        self.assertTrue(all("credential_seam" not in item for item in transport_plan["nodes"]))


if __name__ == "__main__":
    unittest.main()
