#!/usr/bin/env python3
"""TDD contract tests for the external full-network clean-room adapter.

These tests deliberately use the planner's synthetic, authenticated fixture.  No
provider transport is configured and no credential is read.  The adapter is
expected to remain a dry-run boundary until a separately governed transport is
supplied.
"""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
PLANNER_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.py"
PLANNER_TEST_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.test.py"
ADAPTER_PATH = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room-adapter.py"
PROVENANCE_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-pair-provenance.py"


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


class ApplyTransport:
    def __init__(
        self,
        adapter,
        plan: dict[str, object],
        *,
        invalid_operation: str | None = None,
        invalid_signature: bool = False,
        peer_mismatch: bool = False,
        rollback_failure: bool = False,
        side_effect_operation: str | None = None,
    ) -> None:
        self.adapter = adapter
        self.plan = plan
        self.invalid_operation = invalid_operation
        self.invalid_signature = invalid_signature
        self.peer_mismatch = peer_mismatch
        self.rollback_failure = rollback_failure
        self.side_effect_operation = side_effect_operation
        self.operations: list[str] = []
        self.rollback_operations: list[str] = []
        self.rollback_reobservations: list[str] = []
        self.rollback_started: list[str] = []
        self.failed_operation: str | None = None

    def inspect_node(self, node: dict[str, object]) -> dict[str, object]:
        name = node["name"]
        original = next(item for item in self.plan["nodes"] if item["name"] == name)
        required_bytes, required_inodes = self.adapter.capacity_requirement(self.plan, original)
        binding = original["host_binding"]
        evidence = {
            "node": name,
            "node_id": original["node_id"],
            "provider_uid": self.adapter.CANONICAL_PROVIDER_UID[name],
            "node_root": original["node_root"],
            "persistent_state_paths": list(original["persistent_state_paths"]),
            "symlink_free": True,
            "free_bytes": required_bytes,
            "required_bytes": required_bytes,
            "free_inodes": required_inodes,
            "required_inodes": required_inodes,
            "host_target": binding["target"],
            "known_hosts_path": binding["known_hosts_path"],
            "known_host_fingerprint": binding["known_host_fingerprint"],
            "known_hosts_regular": True,
            "known_hosts_owner_uid": os.getuid(),
            "known_hosts_mode": "0600",
        }
        evidence["receipt"] = self._receipt(f"preflight:{name}", original, evidence=evidence)
        return evidence

    def _receipt(
        self,
        operation: str,
        node: dict[str, object] | None,
        *,
        evidence: dict[str, object] | None = None,
    ) -> dict[str, object]:
        node_name = node["name"] if node is not None else None
        peer_id = (
            self.adapter.CANONICAL_PEER_REGISTRY[node_name]
            if node_name is not None
            else ("validator-pair" if operation == "fresh-root-probe" else "fleet")
        )
        if self.peer_mismatch and node_name is not None:
            peer_id = "12D3KooWattacker"
        bindings = {
            "task_uid": self.plan["task_uid"],
            "frozen_head_oid": self.plan["head_oid"],
            "plan_digest": self.plan["plan_digest"],
            "transaction_id": self.plan["transaction_id"],
            "capture_window_id": self.plan["capture_window_id"],
            "operation": operation,
            "node": node_name,
            "peer_id": peer_id,
            "ledger_path": self.plan["credential_nonce_ledger"]["path"],
            "consumer_impact_record": {
                "path": self.plan["consumer_impact_record"]["path"],
                "sha256": self.plan["consumer_impact_record"]["sha256"],
            },
        }
        if evidence is not None:
            bindings["evidence_sha256"] = self.adapter._remote_evidence_digest(evidence)
        receipt: dict[str, object] = {
            "schema_version": self.adapter.PHASE_RECEIPT_SCHEMAS[
                self.adapter._receipt_phase(operation)
            ],
            "authenticated": True,
            "verified": True,
            "signer_id": "governance-signer",
            "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
            "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
            "signed_payload_sha256": "a" * 64,
            "signature_hex": "b" * 128,
            "canonical_digest": "c" * 64,
            "transaction_id": self.plan["transaction_id"],
            "capture_window_id": self.plan["capture_window_id"],
            "operation": operation,
            "node": node_name,
            "peer_id": peer_id,
            "bindings": bindings,
            "phase": self.adapter._receipt_phase(operation),
            "captured_at": "2026-09-01T00:00:00Z",
            "replayed": False,
            "observer_mutation": False,
            "status": (
                "completed"
                if self.adapter._receipt_phase(operation)
                in {"backup", "apply", "rollback", "reobserve"}
                else "verified"
            ),
        }
        if self.adapter._receipt_phase(operation) in {"backup", "apply"}:
            receipt["seed_eligible"] = False
            receipt["backup_manifest"] = {
                "node": node_name,
                "sha256": "d" * 64,
                "size_bytes": 256,
                "verified": True,
                "seed_eligible": False,
            }
        if self.invalid_signature and operation == "preflight:storage-205":
            receipt["signature_hex"] = "0" * 128
        if operation == "fresh-root-probe":
            checkpoint = self.plan["truth"]["checkpoint"]
            receipt["replayed"] = False
            receipt["checkpoint_manifest_hash"] = checkpoint["manifest_hash"]
            receipt["checkpoint_id"] = checkpoint["checkpoint_id"]
            receipt["height"] = checkpoint["height"]
            receipt["package_commit"] = self.plan["truth"]["package"]["commit"]
            receipt["execution_block_hash"] = checkpoint["execution_block_hash"]
            receipt["execution_state_root"] = checkpoint["execution_state_root"]
            receipt["blob_closure"] = copy.deepcopy(self.plan["truth"]["execution"])
            receipt["runtime"] = {
                "sha256": self.plan["truth"]["package"]["runtime_sha256"],
                "size_bytes": self.plan["truth"]["package"]["runtime_size_bytes"],
            }
            receipt["connected_provider"] = {
                "verified": True,
                "providers": [
                    {
                        "node": name,
                        "node_id": next(item for item in self.plan["nodes"] if item["name"] == name)["node_id"],
                        "peer_id": self.adapter.CANONICAL_PEER_REGISTRY[name],
                        "provider_uid": self.adapter.CANONICAL_PROVIDER_UID[name],
                    }
                    for name in ("storage-205", "sequencer-204")
                ],
            }
            receipt["recovery_receipt"] = {
                "schema_version": "oasis7.recovery_receipt.v1",
                "authenticated": True,
                "verified": True,
                "signer_id": "governance-signer",
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signed_payload_sha256": "a" * 64,
                "signature_hex": "b" * 128,
                "canonical_digest": "c" * 64,
                "bindings": {
                    "task_uid": self.plan["task_uid"],
                    "transaction_id": self.plan["transaction_id"],
                    "capture_window_id": self.plan["capture_window_id"],
                    "checkpoint_id": checkpoint["checkpoint_id"],
                    "checkpoint_manifest_hash": checkpoint["manifest_hash"],
                },
            }
        if operation in {"reobserve-failed-state", "rollback-clean-redeploy"}:
            receipt["failed_operation"] = self.failed_operation or "stop:storage-205"
            receipt["failed_state_digest"] = "d" * 64
            receipt["rollback_steps"] = list(self.plan["rollback"]["steps"])
            receipt["reobserved"] = True
        if operation == "fleet-health":
            receipt["fleet_health_closure"] = {
                "verified": True,
                "nodes": list(self.plan["node_order"]),
                "healthy": True,
            }
        return receipt

    def verify_fresh_root_probe(self, plan: dict[str, object]) -> dict[str, object]:
        self.operations.append("fresh-root-probe")
        return self._receipt("fresh-root-probe", None)

    def preflight(self, operation: str, node: dict[str, object] | None) -> dict[str, object]:
        self.operations.append(operation)
        if operation == self.invalid_operation:
            self.failed_operation = operation
            return {"schema_version": "caller-owned.invalid"}
        return self._receipt(operation, node)

    def verify(self, operation: str, node: dict[str, object] | None) -> dict[str, object]:
        self.operations.append(operation)
        if operation == self.invalid_operation:
            self.failed_operation = operation
            return {"schema_version": "caller-owned.invalid"}
        return self._receipt(operation, node)

    def health(self, operation: str) -> dict[str, object]:
        self.operations.append(operation)
        if operation == self.invalid_operation:
            self.failed_operation = operation
            return {"schema_version": "caller-owned.invalid"}
        return self._receipt(operation, None)

    def mutate(self, operation: str, node: dict[str, object] | None) -> dict[str, object]:
        self.operations.append(operation)
        if operation == self.side_effect_operation:
            self.failed_operation = operation
            raise RuntimeError("provider side effect then throw")
        if operation == self.invalid_operation:
            self.failed_operation = operation
            return {"schema_version": "caller-owned.invalid"}
        return self._receipt(operation, node)

    def reobserve_failed_state(
        self, plan: dict[str, object], started: list[str], failed_operation: str
    ) -> dict[str, object]:
        self.rollback_reobservations.append(failed_operation)
        self.failed_operation = failed_operation
        return self._receipt("reobserve-failed-state", None)

    def rollback_clean_redeploy(
        self,
        plan: dict[str, object],
        started: list[str],
        failed_state: dict[str, object] | None = None,
    ) -> dict[str, object]:
        self.rollback_operations.append("rollback")
        self.rollback_started = list(started)
        if self.rollback_failure:
            raise RuntimeError("rollback transport unavailable")
        return self._receipt("rollback-clean-redeploy", None)


class FullNetworkCleanRoomAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.planner = load_module("full_network_clean_room", PLANNER_PATH)
        self.adapter = load_module("full_network_clean_room_adapter", ADAPTER_PATH)
        self.fixture_module = load_module("full_network_clean_room_fixture", PLANNER_TEST_PATH)
        self.fixture = self.fixture_module.FullNetworkCleanRoomPlanTests()
        self.fixture.setUp()
        self._test_directory = tempfile.TemporaryDirectory()
        self.ledger_path = Path(self._test_directory.name) / "nonce.jsonl"
        self._write_ledger(self.ledger_path)
        self.plan = self._bind_test_ledger(self.planner.build_plan(self.fixture._input()))
        self.live_trust_root_patcher = mock.patch.object(
            self.adapter,
            "validate_live_trust_root_file",
            return_value={
                "path": self.adapter.CANONICAL_TRUST_ROOT_PATH,
                "sha256": self.adapter.CANONICAL_TRUST_ROOT_FILE_SHA256,
                "root_digest": self.adapter.CANONICAL_TRUST_ROOT_DIGEST,
                "owner_scope": self.adapter.CANONICAL_TRUST_ROOT_OWNER_SCOPE,
                "owner_uid": os.getuid(),
                "mode": "0600",
                "regular_file": True,
            },
        )
        self.live_trust_root_patcher.start()

    def tearDown(self) -> None:
        self.live_trust_root_patcher.stop()
        self._test_directory.cleanup()

    def _bind_test_ledger(self, plan: dict[str, object]) -> dict[str, object]:
        plan["credential_nonce_ledger"]["path"] = str(self.ledger_path)
        plan["credential_nonce_ledger"]["receipt"]["bindings"]["path"] = str(self.ledger_path)
        for node in plan["nodes"]:
            node["credential_seam"]["ledger_path"] = str(self.ledger_path)
        plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
        return plan

    def _authority(
        self, apply_authorized: bool = False, plan: dict[str, object] | None = None
    ) -> dict[str, object]:
        plan = plan or self.plan
        execution = copy.deepcopy(plan["truth"]["execution"])
        return {
            "schema_version": self.adapter.AUTHORITY_SCHEMA,
            "repository": "eng-cc/oasis7",
            "task_uid": plan["task_uid"],
            "frozen_head_oid": plan["head_oid"],
            "plan_digest": plan["plan_digest"],
            "adapter_id": self.adapter.CANONICAL_ADAPTER_ID,
            "network_id": self.adapter.CANONICAL_NETWORK_ID,
            "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
            "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
            "trust_root_path": self.adapter.CANONICAL_TRUST_ROOT_PATH,
            "trust_root_digest": self.adapter.CANONICAL_TRUST_ROOT_DIGEST,
            "consumer_impact_record": copy.deepcopy(plan["consumer_impact_record"]),
            "trust_root_file": {
                "path": self.adapter.CANONICAL_TRUST_ROOT_PATH,
                "sha256": self.adapter.CANONICAL_TRUST_ROOT_FILE_SHA256,
                "root_digest": self.adapter.CANONICAL_TRUST_ROOT_DIGEST,
                "owner_scope": self.adapter.CANONICAL_TRUST_ROOT_OWNER_SCOPE,
                "owner_uid": os.getuid(),
                "mode": "0600",
                "regular_file": True,
            },
            "apply_authorized": apply_authorized,
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
                    "task_uid": plan["task_uid"],
                    "frozen_head_oid": plan["head_oid"],
                    "plan_digest": plan["plan_digest"],
                    "execution": execution,
                    "ledger_path": plan["credential_nonce_ledger"]["path"],
                    "apply_authorized": apply_authorized,
                    "forensic_backup": copy.deepcopy(plan["forensic_backup"]),
                    "package_commit": plan["truth"]["package"]["commit"],
                    "checkpoint_id": plan["truth"]["checkpoint"]["checkpoint_id"],
                    "checkpoint_manifest_hash": plan["truth"]["checkpoint"]["manifest_hash"],
                    "trust_root_path": self.adapter.CANONICAL_TRUST_ROOT_PATH,
                    "trust_root_digest": self.adapter.CANONICAL_TRUST_ROOT_DIGEST,
                    "trust_root_file": {
                        "path": self.adapter.CANONICAL_TRUST_ROOT_PATH,
                        "sha256": self.adapter.CANONICAL_TRUST_ROOT_FILE_SHA256,
                        "root_digest": self.adapter.CANONICAL_TRUST_ROOT_DIGEST,
                        "owner_scope": self.adapter.CANONICAL_TRUST_ROOT_OWNER_SCOPE,
                        "owner_uid": os.getuid(),
                        "mode": "0600",
                        "regular_file": True,
                    },
                    "consumer_impact_record": {
                        "path": plan["consumer_impact_record"]["path"],
                        "sha256": plan["consumer_impact_record"]["sha256"],
                    },
                },
            },
        }

    def _no_backup_plan(self) -> dict[str, object]:
        request = self.fixture._input()
        request["backup_policy"] = {
            "mode": "operator-authorized-no-backup",
            "operator_authorized": True,
            "current_authorization": True,
            "repository": "eng-cc/oasis7",
            "action": "full-network-clean-room",
            "targets": list(self.planner.NODE_ORDER),
            "transaction_id": request["transaction_id"],
            "capture_window_id": request["capture_window_id"],
            "actor": "ops-actor",
            "issued_at": "2026-08-30T00:00:00Z",
            "expires_at": "2099-01-01T00:00:00Z",
            "task_uid": request["task_uid"],
            "frozen_head_oid": request["head_oid"],
            "reason": "immutable provider backup unavailable",
            "authority": self.fixture._no_backup_receipt(
                request, "2099-01-01T00:00:00Z"
            ),
        }
        return self._bind_test_ledger(self.planner.build_plan(request))

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
        plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_plan(plan)

        for field, bad_value in (
            ("schema_version", "oasis7.attacker_identity.v1"),
            ("signature_hex", "0" * 128),
            ("key_sha256", "0" * 64),
            ("key_mode", "0644"),
            ("verifier_id", "caller-verifier"),
            ("trust_root_id", "caller-root"),
        ):
            plan = copy.deepcopy(self.plan)
            plan["nodes"][0]["identity_receipt"][field] = bad_value
            plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
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
            ledger = self.ledger_path
            self._write_ledger(ledger)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=Path(directory) / "journal.json",
                    ledger_path=self.ledger_path,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
        self.assertEqual(calls, [self.plan["plan_digest"]])

    def test_apply_validates_every_provider_receipt_and_persists_sanitized_receipts(self) -> None:
        authority = self._authority(apply_authorized=True)
        verifier_calls: list[str] = []

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            verifier_calls.append(receipt["bindings"]["operation"] if "operation" in receipt["bindings"] else "authority")
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan)
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            result = self.adapter.execute(
                self.plan,
                authority,
                journal_path=journal,
                ledger_path=self.ledger_path,
                transport=transport,
                dry_run=False,
                provenance_verifier=verifier,
            )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(result["status"], "complete")
        self.assertEqual(transport.operations, self.plan["global_order"])
        self.assertEqual(len(record["provider_receipts"]), len(self.plan["global_order"]))
        self.assertIsNone(record["rollback_receipt"])
        self.assertEqual(record["execution_mode"], "apply")
        self.assertEqual(record["rollback_status"], "not-needed")
        preflight_receipts = record["preflight_evidence_receipts"]
        self.assertEqual(len(preflight_receipts), len(self.plan["nodes"]))
        self.assertEqual(
            [receipt["operation"] for receipt in preflight_receipts],
            [f"preflight:{name}" for name in self.plan["node_order"]],
        )
        for receipt in preflight_receipts:
            self.assertRegex(receipt["bindings"]["evidence_sha256"], r"^[0-9a-f]{64}$")
        self.assertIn("authority", verifier_calls)
        self.assertIn("fresh-root-probe", verifier_calls)

    def test_apply_persists_preflight_complete_checkpoint_before_first_operation(self) -> None:
        authority = self._authority(apply_authorized=True)
        statuses: list[str] = []
        original_write = self.adapter._write_journal

        def observe_write(path: Path, record: dict[str, object]) -> None:
            statuses.append(record["status"])
            original_write(path, record)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with mock.patch.object(self.adapter, "_write_journal", side_effect=observe_write):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=journal,
                    ledger_path=self.ledger_path,
                    transport=ApplyTransport(self.adapter, self.plan),
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertIn("preflight-complete", statuses)
        self.assertLess(statuses.index("preflight-complete"), statuses.index("in-flight"))
        self.assertEqual(record["preflight_status"], "complete")
        self.assertEqual(record["nonce_reservation_state"]["reserved_count"], len(self.plan["nodes"]))
        self.assertTrue(record["nonce_reservation_state"]["complete"])

    def test_resume_from_prepared_or_preflight_checkpoint_reconciles_without_double_use(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        for interrupted_status in ("prepared", "preflight-complete"):
            with self.subTest(interrupted_status=interrupted_status), tempfile.TemporaryDirectory() as directory:
                self._write_ledger(self.ledger_path)
                journal = Path(directory) / "journal.json"
                original_write = self.adapter._write_journal

                def interrupt(path: Path, record: dict[str, object]) -> None:
                    original_write(path, record)
                    if record["status"] == interrupted_status:
                        raise KeyboardInterrupt

                with mock.patch.object(self.adapter, "_write_journal", side_effect=interrupt):
                    with self.assertRaises(KeyboardInterrupt):
                        self.adapter.execute(
                            self.plan,
                            authority,
                            journal_path=journal,
                            ledger_path=self.ledger_path,
                            transport=ApplyTransport(self.adapter, self.plan),
                            dry_run=False,
                            provenance_verifier=verifier,
                        )
                checkpoint = json.loads(journal.read_text(encoding="utf-8"))
                self.assertEqual(checkpoint["status"], interrupted_status)
                rows_before = len(self.ledger_path.read_text(encoding="utf-8").splitlines())
                transport = ApplyTransport(self.adapter, self.plan)
                original_inspect = transport.inspect_node

                def inspect(node: dict[str, object]) -> dict[str, object]:
                    if interrupted_status == "preflight-complete":
                        raise AssertionError("resume must reuse the durable preflight checkpoint")
                    return original_inspect(node)

                transport.inspect_node = inspect
                result = self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
                rows_after = len(self.ledger_path.read_text(encoding="utf-8").splitlines())
                self.assertEqual(result["status"], "complete")
                expected_rows = (
                    rows_before
                    if interrupted_status == "preflight-complete"
                    else rows_before + len(self.plan["nodes"])
                )
                self.assertEqual(rows_after, expected_rows)

    def test_resume_after_partial_nonce_reservation_reconciles_missing_once(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        original_reserve = self.adapter.reserve_nonce
        reservations = 0

        def reserve_then_interrupt(path: Path, transaction_id: str, nonce: str) -> None:
            nonlocal reservations
            if reservations == 1:
                raise KeyboardInterrupt
            original_reserve(path, transaction_id, nonce)
            reservations += 1

        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with mock.patch.object(self.adapter, "reserve_nonce", side_effect=reserve_then_interrupt):
                with self.assertRaises(KeyboardInterrupt):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=journal,
                        ledger_path=self.ledger_path,
                        transport=ApplyTransport(self.adapter, self.plan),
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
            checkpoint = json.loads(journal.read_text(encoding="utf-8"))
            self.assertEqual(checkpoint["status"], "prepared")
            self.assertEqual(checkpoint["nonce_reservation_state"]["reserved_count"], 0)
            self.assertEqual(len(self.ledger_path.read_text(encoding="utf-8").splitlines()), 1)
            result = self.adapter.resume_transaction(
                self.plan,
                authority,
                journal,
                ledger_path=self.ledger_path,
                transport=ApplyTransport(self.adapter, self.plan),
                dry_run=False,
                provenance_verifier=verifier,
            )
            self.assertEqual(result["status"], "complete")
            self.assertEqual(
                len(self.ledger_path.read_text(encoding="utf-8").splitlines()),
                len(self.plan["nodes"]),
            )

    def test_preflight_checkpoint_fails_closed_if_ledger_reservation_is_missing(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            original_write = self.adapter._write_journal

            def interrupt_after_checkpoint(path: Path, record: dict[str, object]) -> None:
                original_write(path, record)
                if record["status"] == "preflight-complete":
                    raise KeyboardInterrupt

            with mock.patch.object(self.adapter, "_write_journal", side_effect=interrupt_after_checkpoint):
                with self.assertRaises(KeyboardInterrupt):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=journal,
                        ledger_path=self.ledger_path,
                        transport=ApplyTransport(self.adapter, self.plan),
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
            rows = self.ledger_path.read_text(encoding="utf-8").splitlines()
            self._write_ledger(self.ledger_path, [json.loads(row) for row in rows[:-1]])
            transport = ApplyTransport(self.adapter, self.plan)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            self.assertEqual(transport.operations, [])

    def test_preflight_evidence_receipts_must_follow_canonical_node_order(self) -> None:
        transport = ApplyTransport(self.adapter, self.plan)
        receipts = [
            transport._receipt(f"preflight:{name}", next(node for node in self.plan["nodes"] if node["name"] == name))
            for name in self.plan["node_order"]
        ]
        for receipt in receipts:
            receipt["bindings"]["evidence_sha256"] = "a" * 64
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_journal_preflight_evidence_receipts(
                self.plan,
                list(reversed(receipts)),
            )

    def test_prepared_journal_rejects_preflight_receipts_instead_of_appending(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan)
        receipt = transport.inspect_node(self.plan["nodes"][0])["receipt"]
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            self.adapter._write_journal(
                journal,
                self.adapter._journal_record(
                    self.plan,
                    "prepared",
                    0,
                    [],
                    execution_mode="apply",
                    preflight_evidence_receipts=[receipt],
                    preflight_status="pending",
                    nonce_reservation_state=self.adapter._nonce_reservation_state(self.plan, 0),
                ),
            )
            with self.assertRaisesRegex(self.adapter.AdapterError, "prepared journal must not contain preflight evidence"):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
        self.assertEqual(transport.operations, [])

    def test_v1_journal_is_rejected_with_migration_reconciliation_message(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            self.adapter.execute(
                self.plan,
                self._authority(),
                journal_path=journal,
                ledger_path=self.ledger_path,
                dry_run=True,
            )
            record = json.loads(journal.read_text(encoding="utf-8"))
            record["schema_version"] = "oasis7.clean_room_mutation_journal.v1"
            record["journal_digest"] = self.adapter.journal_digest(record)
            journal.write_text(json.dumps(record) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(self.adapter.AdapterError, "v1.*migration.*reconciliation"):
                self.adapter._read_journal(journal)

    def test_nonce_append_then_error_terminal_state_uses_ledger_readback(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        original_reserve = self.adapter.reserve_nonce

        def append_then_error(path: Path, transaction_id: str, nonce: str) -> None:
            original_reserve(path, transaction_id, nonce)
            raise RuntimeError("append succeeded before callback error")

        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with mock.patch.object(self.adapter, "reserve_nonce", side_effect=append_then_error):
                with self.assertRaises(self.adapter.AdapterError):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=journal,
                        ledger_path=self.ledger_path,
                        transport=ApplyTransport(self.adapter, self.plan),
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(record["status"], "terminal-failure")
        self.assertEqual(record["nonce_reservation_state"]["reserved_count"], 1)

    def test_repository_trust_root_fixture_matches_provenance_helper_without_monkeypatch(self) -> None:
        provenance = load_module("validator_pair_provenance_fixture", PROVENANCE_PATH)
        fixture = ROOT / "scripts" / "fixtures" / "oasis7-governance-root.v1.json"
        self.assertTrue(fixture.is_file())
        loaded = provenance.load_trust_root(fixture)
        self.assertEqual(loaded["root_digest"], self.adapter.CANONICAL_TRUST_ROOT_DIGEST)
        self.assertEqual(
            hashlib.sha256(fixture.read_bytes()).hexdigest(),
            self.adapter.CANONICAL_TRUST_ROOT_FILE_SHA256,
        )

    def test_backup_failure_with_no_mutation_candidates_does_not_call_clean_redeploy(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(
            self.adapter,
            self.plan,
            invalid_operation="forensic-backup:storage-205",
        )
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(transport.rollback_operations, [])
        self.assertEqual(transport.rollback_reobservations, [])
        self.assertEqual(record["rollback_status"], "not-needed")
        self.assertEqual(record["backup_status"], "backup-failed")
        self.assertEqual(record["backup_error"], "AdapterError")

    def test_apply_executes_code_owned_live_trust_root_check_before_remote_observation(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan, invalid_operation="preflight:storage-205")
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.object(
                self.adapter,
                "validate_live_trust_root_file",
                create=True,
                return_value={"path": self.adapter.CANONICAL_TRUST_ROOT_PATH},
            ) as live_check:
                with self.assertRaises(self.adapter.AdapterError):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=Path(directory) / "journal.json",
                        ledger_path=self.ledger_path,
                        transport=transport,
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
                live_check.assert_called_once_with()

    def test_read_only_failure_with_no_rollback_candidates_never_calls_rollback(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan, invalid_operation="preflight:storage-205")
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with mock.patch.object(
                self.adapter,
                "validate_live_trust_root_file",
                create=True,
                return_value={"path": self.adapter.CANONICAL_TRUST_ROOT_PATH},
            ):
                with self.assertRaises(self.adapter.AdapterError):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=journal,
                        ledger_path=self.ledger_path,
                        transport=transport,
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(transport.rollback_operations, [])
        self.assertEqual(transport.rollback_reobservations, [])
        self.assertEqual(record["rollback_status"], "not-needed")

    def test_journal_rejects_symlink_in_any_ancestor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            real_parent = root / "real-parent"
            real_parent.mkdir()
            symlink_parent = root / "symlink-parent"
            symlink_parent.symlink_to(real_parent, target_is_directory=True)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter._write_journal(
                    symlink_parent / "nested" / "journal.json",
                    {"schema_version": self.adapter.JOURNAL_SCHEMA},
                )

    def test_live_trust_root_file_checks_content_owner_mode_and_symlink(self) -> None:
        self.live_trust_root_patcher.stop()
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory) / "trust-root.json"
                content = (ROOT / "scripts" / "fixtures" / "oasis7-governance-root.v1.json").read_bytes()
                semantic_digest = json.loads(content.decode("utf-8"))["root_digest"]
                file_digest = hashlib.sha256(content).hexdigest()
                root.write_bytes(content)
                root.chmod(0o600)
                with mock.patch.object(self.adapter, "CANONICAL_TRUST_ROOT_PATH", str(root)), mock.patch.object(
                    self.adapter,
                    "CANONICAL_TRUST_ROOT_DIGEST",
                    semantic_digest,
                ), mock.patch.object(
                    self.adapter,
                    "CANONICAL_TRUST_ROOT_FILE_SHA256",
                    file_digest,
                ):
                    result = self.adapter.validate_live_trust_root_file()
                    self.assertEqual(result["sha256"], file_digest)
                    root.chmod(0o644)
                    with self.assertRaises(self.adapter.AdapterError):
                        self.adapter.validate_live_trust_root_file()
                    root.chmod(0o600)
                    with mock.patch.object(
                        self.adapter, "CANONICAL_TRUST_ROOT_OWNER_UID", os.getuid() + 1
                    ):
                        with self.assertRaises(self.adapter.AdapterError):
                            self.adapter.validate_live_trust_root_file()
                    root.unlink()
                    root.symlink_to(Path(directory) / "missing-root.json")
                    with self.assertRaises(self.adapter.AdapterError):
                        self.adapter.validate_live_trust_root_file()
                    real_parent = Path(directory) / "real-parent"
                    real_parent.mkdir()
                    nested_root = real_parent / "trust-root.json"
                    nested_root.write_bytes(content)
                    nested_root.chmod(0o600)
                    symlink_parent = Path(directory) / "symlink-parent"
                    symlink_parent.symlink_to(real_parent, target_is_directory=True)
                    with mock.patch.object(
                        self.adapter,
                        "CANONICAL_TRUST_ROOT_PATH",
                        str(symlink_parent / "trust-root.json"),
                    ), mock.patch.object(
                        self.adapter,
                        "CANONICAL_TRUST_ROOT_DIGEST",
                        semantic_digest,
                    ), mock.patch.object(
                        self.adapter,
                        "CANONICAL_TRUST_ROOT_FILE_SHA256",
                        file_digest,
                    ):
                        with self.assertRaises(self.adapter.AdapterError):
                            self.adapter.validate_live_trust_root_file()
        finally:
            self.live_trust_root_patcher.start()

    def test_completed_apply_resume_revalidates_authority_verifier_and_receipt_signatures(self) -> None:
        authority = self._authority(apply_authorized=True)
        verifier_calls: list[str] = []

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            verifier_calls.append(
                receipt["bindings"].get("operation", "authority")
            )
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan)
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            self.adapter.execute(
                self.plan,
                authority,
                journal_path=journal,
                ledger_path=self.ledger_path,
                transport=transport,
                dry_run=False,
                provenance_verifier=verifier,
            )

            verifier_calls.clear()
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    self._authority(apply_authorized=False),
                    journal,
                    ledger_path=self.ledger_path,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            self.assertEqual(verifier_calls, [])

            record = json.loads(journal.read_text(encoding="utf-8"))
            record["provider_receipts"][0]["signature_hex"] = "0" * 128
            record["journal_digest"] = self.adapter.journal_digest(record)
            journal.write_text(json.dumps(record) + "\n", encoding="utf-8")
            verifier_calls.clear()
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            self.assertIn("authority", verifier_calls)

    def test_completed_apply_resume_revalidates_evidence_bound_preflight_receipt(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            self.adapter.execute(
                self.plan,
                authority,
                journal_path=journal,
                ledger_path=self.ledger_path,
                transport=ApplyTransport(self.adapter, self.plan),
                dry_run=False,
                provenance_verifier=verifier,
            )
            record = json.loads(journal.read_text(encoding="utf-8"))
            record["preflight_evidence_receipts"][0]["signature_hex"] = "0" * 128
            record["journal_digest"] = self.adapter.journal_digest(record)
            journal.write_text(json.dumps(record) + "\n", encoding="utf-8")
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    dry_run=False,
                    provenance_verifier=verifier,
                )

    def test_operator_no_backup_apply_requires_signed_current_authority(self) -> None:
        plan = self._no_backup_plan()
        authority = self._authority(apply_authorized=True, plan=plan)
        plan["forensic_backup"]["authority"] = None
        plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
        authority = self._authority(apply_authorized=True, plan=plan)
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.execute(
                plan,
                authority,
                journal_path=Path(tempfile.mkdtemp()) / "journal.json",
                ledger_path=self.ledger_path,
                dry_run=True,
            )

    def test_fresh_root_probe_requires_signed_full_closure(self) -> None:
        transport = ApplyTransport(self.adapter, self.plan)
        receipt = transport._receipt("fresh-root-probe", None)
        receipt["blob_closure"]["json_index_consistency"]["verified"] = False
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(
                self.plan,
                "fresh-root-probe",
                None,
                receipt,
                None,
            )

    def test_every_provider_receipt_is_fresh_and_inside_transaction_capture_window(self) -> None:
        transport = ApplyTransport(self.adapter, self.plan)
        receipt = transport._receipt("stop:storage-205", self.plan["nodes"][0])
        receipt["replayed"] = True
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(
                self.plan, "stop:storage-205", "storage-205", receipt, None
            )

        receipt = transport._receipt("stop:storage-205", self.plan["nodes"][0])
        receipt["captured_at"] = "2020-01-01T00:00:00Z"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(
                self.plan, "stop:storage-205", "storage-205", receipt, None
            )

    def test_remote_preflight_requires_signed_verifier_checked_evidence_receipt(self) -> None:
        node = self.plan["nodes"][0]
        required_bytes, required_inodes = self.adapter.capacity_requirement(self.plan, node)
        evidence = {
            "node": node["name"],
            "node_id": node["node_id"],
            "provider_uid": self.adapter.CANONICAL_PROVIDER_UID[node["name"]],
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
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_remote_preflight(self.plan, node, evidence)

    def test_no_backup_authority_rejects_future_issue_and_requires_independent_verifier(self) -> None:
        plan = self._no_backup_plan()
        plan["forensic_backup"]["issued_at"] = "2099-01-01T00:00:00Z"
        plan["forensic_backup"]["authority"]["bindings"]["issued_at"] = "2099-01-01T00:00:00Z"
        plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_plan(plan)

        plan = self._no_backup_plan()
        authority = self._authority(apply_authorized=False, plan=plan)
        calls: list[str] = []

        def verifier(plan_dto: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            calls.append(receipt["schema_version"])
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        with tempfile.TemporaryDirectory() as directory:
            self.adapter.execute(
                plan,
                authority,
                journal_path=Path(directory) / "journal.json",
                ledger_path=self.ledger_path,
                dry_run=True,
                provenance_verifier=verifier,
            )
        self.assertEqual(
            calls,
            [self.adapter.CRYPTO_RECEIPT_SCHEMA, self.adapter.NO_BACKUP_AUTHORITY_SCHEMA],
        )

    def test_no_backup_authority_binds_task_head_transaction_and_capture(self) -> None:
        for field, bad_value in (
            ("task_uid", "task_attacker"),
            ("frozen_head_oid", "f" * 40),
            ("transaction_id", "other-transaction"),
            ("capture_window_id", "other-window"),
        ):
            plan = self._no_backup_plan()
            plan["forensic_backup"][field] = bad_value
            plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_plan(plan)

        plan = self._no_backup_plan()
        plan["forensic_backup"]["authority"]["bindings"]["frozen_head_oid"] = "f" * 40
        plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_plan(plan)

    def test_forensic_backup_mode_cannot_bypass_required_reset_gate(self) -> None:
        for field, bad_value in (
            ("task_uid", "task_attacker"),
            ("frozen_head_oid", "f" * 40),
            ("required_before_reset", False),
            ("immutable", False),
            ("receipt_required_per_node", False),
            ("operator_authorized", True),
            ("current_authorization", True),
            ("mode", "operator-authorized-no-backup"),
        ):
            plan = copy.deepcopy(self.plan)
            plan["forensic_backup"][field] = bad_value
            plan["plan_digest"] = self.adapter.canonical_plan_digest(plan)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_plan(plan)

    def test_provider_receipts_require_phase_schema_and_capture_contract(self) -> None:
        transport = ApplyTransport(self.adapter, self.plan)
        receipt = transport._receipt("stop:storage-205", self.plan["nodes"][1])
        for field, bad_value in (
            ("schema_version", self.adapter.PROVIDER_RECEIPT_SCHEMA),
            ("phase", "verify"),
            ("captured_at", None),
            ("observer_mutation", True),
            ("status", "planned"),
            ("seed_eligible", True),
        ):
            invalid = copy.deepcopy(receipt)
            invalid[field] = bad_value
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter._validate_provider_receipt(
                    self.plan, "stop:storage-205", "storage-205", invalid, None
                )

        backup = transport._receipt("forensic-backup:storage-205", self.plan["nodes"][1])
        backup["backup_manifest"]["seed_eligible"] = True
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(
                self.plan, "forensic-backup:storage-205", "storage-205", backup, None
            )

        health = transport._receipt("fleet-health", None)
        health["fleet_health_closure"]["nodes"] = ["storage-205"]
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(self.plan, "fleet-health", None, health, None)

    def test_rejects_invalid_provider_signature_or_peer_before_advancing(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        for kwargs in (
            {"invalid_operation": "preflight:storage-205"},
            {"invalid_signature": True},
            {"peer_mismatch": True},
        ):
            transport = ApplyTransport(self.adapter, self.plan, **kwargs)
            self._write_ledger(self.ledger_path)
            with tempfile.TemporaryDirectory() as directory:
                journal = Path(directory) / "journal.json"
                with self.assertRaises(self.adapter.AdapterError):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=journal,
                        ledger_path=self.ledger_path,
                        transport=transport,
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
                record = json.loads(journal.read_text(encoding="utf-8"))
            self.assertEqual(record["status"], "terminal-failure")
            self.assertEqual(record["execution_mode"], "apply")
            self.assertTrue(
                all(self.adapter._rollback_candidate(operation) for operation in transport.rollback_started)
            )

    def test_apply_failure_persists_node_and_rollback_receipts(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan, invalid_operation="stop:storage-205")
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(record["status"], "terminal-failure")
        self.assertTrue(record["provider_receipts"])
        self.assertEqual(record["rollback_status"], "completed")
        self.assertEqual(record["rollback_receipt"]["operation"], "rollback-clean-redeploy")
        self.assertEqual(transport.rollback_reobservations, ["stop:storage-205"])
        self.assertEqual(record["rollback_receipt"]["failed_operation"], "stop:storage-205")
        self.assertEqual(
            record["rollback_receipt"]["rollback_steps"], self.plan["rollback"]["steps"]
        )

    def test_rollback_failure_is_reconciliation_blocked_and_durable(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(
            self.adapter,
            self.plan,
            invalid_operation="stop:storage-205",
            rollback_failure=True,
        )
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
            record = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(record["rollback_status"], "reconciliation-blocked")
        self.assertIsNone(record["rollback_receipt"])
        self.assertEqual(record["rollback_error"], "RuntimeError")

    def test_side_effect_then_throw_includes_current_operation_in_rollback(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(
            self.adapter,
            self.plan,
            side_effect_operation="rebuild:storage-205",
        )
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=journal,
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                    provenance_verifier=verifier,
                )
        self.assertIn("rebuild:storage-205", transport.rollback_started)

    def test_journal_write_failure_rolls_back_current_started_operation(self) -> None:
        authority = self._authority(apply_authorized=True)

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        transport = ApplyTransport(self.adapter, self.plan)
        original_write = self.adapter._write_journal
        injected = False

        def write_with_one_failure(path: Path, record: dict[str, object]) -> None:
            nonlocal injected
            if (
                not injected
                and record.get("status") == "running"
                and "start:sequencer-204" in record.get("completed_operations", [])
            ):
                injected = True
                raise self.adapter.AdapterError("injected journal failure")
            original_write(path, record)

        self.adapter._write_journal = write_with_one_failure
        try:
            with tempfile.TemporaryDirectory() as directory:
                journal = Path(directory) / "journal.json"
                with self.assertRaises(self.adapter.AdapterError):
                    self.adapter.execute(
                        self.plan,
                        authority,
                        journal_path=journal,
                        ledger_path=self.ledger_path,
                        transport=transport,
                        dry_run=False,
                        provenance_verifier=verifier,
                    )
                record = json.loads(journal.read_text(encoding="utf-8"))
        finally:
            self.adapter._write_journal = original_write
        self.assertTrue(injected)
        self.assertIn("start:sequencer-204", transport.rollback_started)
        self.assertEqual(record["rollback_status"], "completed")

    def test_transport_and_receipt_boundaries_reject_secret_fields(self) -> None:
        node = copy.deepcopy(self.plan["nodes"][0])
        node["identity_receipt"]["nested"] = {"password": "must-not-cross"}
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._transport_node(node)
        node = copy.deepcopy(self.plan["nodes"][0])
        node["identity_receipt"]["nested"] = {"safe_label": "PRIVATE KEY material"}
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._transport_node(node)

        plan = copy.deepcopy(self.plan)
        plan["unexpected_provider_field"] = "must-not-cross"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._transport_plan(plan)

        transport = ApplyTransport(self.adapter, self.plan)
        receipt = transport._receipt("stop:storage-205", self.plan["nodes"][0])
        receipt["bindings"]["secret"] = "must-not-persist"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(
                self.plan,
                "stop:storage-205",
                "storage-205",
                receipt,
                None,
            )

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
                    "node_id": node["node_id"],
                    "provider_uid": self.adapter.CANONICAL_PROVIDER_UID[node["name"]],
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
            "node_id": node["node_id"],
            "provider_uid": self.adapter.CANONICAL_PROVIDER_UID[node["name"]],
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
        evidence["receipt"] = ApplyTransport(self.adapter, self.plan)._receipt(
            f"preflight:{node['name']}", node, evidence=evidence
        )

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        result = self.adapter.validate_remote_preflight(self.plan, node, evidence, verifier)
        self.assertTrue(result["known_hosts_pinned"])
        tampered = copy.deepcopy(evidence)
        tampered["free_bytes"] += 1
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_remote_preflight(self.plan, node, tampered, verifier)
        for field, bad_value in (("symlink_free", False), ("free_bytes", required_bytes - 1)):
            invalid = dict(evidence, **{field: bad_value})
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_remote_preflight(self.plan, node, invalid, verifier)

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

            cross_transaction = Path(directory) / "cross-transaction.jsonl"
            self._write_ledger(
                cross_transaction,
                [dict(row, transaction_id="different-transaction")],
            )
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_credential_ledger(self.plan, cross_transaction)

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
            ledger = self.ledger_path
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

    def test_terminal_journal_failure_persists_emergency_reconciliation_receipt(self) -> None:
        original_write = self.adapter._write_journal
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"

            def fail_primary(path: Path, record: dict[str, object]) -> None:
                if path == journal:
                    raise self.adapter.AdapterError("injected primary journal failure")
                original_write(path, record)

            self.adapter._write_journal = fail_primary
            try:
                with self.assertRaises(self.adapter.AdapterError):
                    self.adapter._persist_terminal(
                        journal,
                        {
                            "schema_version": self.adapter.JOURNAL_SCHEMA,
                            "status": "terminal-failure",
                            "transaction_id": self.plan["transaction_id"],
                        },
                    )
            finally:
                self.adapter._write_journal = original_write

            emergency = Path(f"{journal}.emergency.json")
            record = json.loads(emergency.read_text(encoding="utf-8"))
            self.assertEqual(record["status"], "reconciliation-blocked")
            self.assertTrue(record["emergency_receipt"])
            self.assertEqual(record["journal_write_error"], "AdapterError")

    def test_dry_run_has_deterministic_order_and_never_mutates_provider(self) -> None:
        authority = self._authority()
        transport = FakeTransport()
        with tempfile.TemporaryDirectory() as directory:
            ledger = self.ledger_path
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

        captured: dict[str, object] = {}

        def verifier(plan: dict[str, object], receipt: dict[str, object]) -> dict[str, object]:
            captured.update(plan)
            return {
                "verified": True,
                "bindings": receipt["bindings"],
                "verifier_id": self.adapter.CANONICAL_VERIFIER_ID,
                "trust_root_id": self.adapter.CANONICAL_TRUST_ROOT_ID,
                "signer_id": "governance-signer",
            }

        with tempfile.TemporaryDirectory() as directory:
            self.adapter.execute(
                self.plan,
                self._authority(),
                journal_path=Path(directory) / "journal.json",
                ledger_path=self.ledger_path,
                dry_run=True,
                provenance_verifier=verifier,
            )
        self.assertNotIn("credential_nonce_ledger", captured)
        self.assertNotIn("authority", captured)
        captured_text = json.dumps(captured, sort_keys=True)
        self.assertNotIn("storage-nonce-", captured_text)
        self.assertNotIn("PUBLIC_TESTNET_", captured_text)

    def test_trust_root_path_and_digest_are_code_owned(self) -> None:
        authority = self._authority()
        authority["trust_root_path"] = "/caller/selected/trust-root.json"
        authority["trust_root_digest"] = "a" * 64
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

        authority = self._authority()
        authority["trust_root_file"]["owner_uid"] = os.getuid() + 1
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

        authority = self._authority()
        authority["trust_root_file"]["mode"] = "0644"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

        authority = self._authority()
        authority["trust_root_file"]["root_digest"] = "a" * 64
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

        authority = self._authority()
        authority["trust_root_file"]["owner_scope"] = "caller-selected"
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter.validate_authority(self.plan, authority)

    def test_journal_execution_mode_and_ledger_path_are_exact_bindings(self) -> None:
        authority = self._authority()
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal.json"
            self.adapter.execute(
                self.plan,
                authority,
                journal_path=journal,
                ledger_path=self.ledger_path,
                dry_run=True,
            )
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    dry_run=False,
                )
            record = json.loads(journal.read_text(encoding="utf-8"))
            record["execution_mode"] = "apply"
            record["journal_digest"] = self.adapter.journal_digest(record)
            journal.write_text(json.dumps(record) + "\n", encoding="utf-8")
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.resume_transaction(
                    self.plan,
                    authority,
                    journal,
                    ledger_path=self.ledger_path,
                    dry_run=True,
                )

            journal.chmod(0o644)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=journal,
                    ledger_path=self.ledger_path,
                    dry_run=True,
                )

            alternate_ledger = Path(directory) / "alternate.jsonl"
            self._write_ledger(alternate_ledger)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=Path(directory) / "alternate-journal.json",
                    ledger_path=alternate_ledger,
                    dry_run=True,
                )

    def test_consumer_impact_binding_covers_transport_authority_journal_and_receipts(self) -> None:
        impact_locator = {
            "path": self.plan["consumer_impact_record"]["path"],
            "sha256": self.plan["consumer_impact_record"]["sha256"],
        }
        self.assertEqual(
            self.adapter._transport_plan(self.plan)["consumer_impact_record"],
            self.plan["consumer_impact_record"],
        )
        authority = self._authority()
        self.adapter.validate_authority(self.plan, authority)
        self.assertEqual(
            authority["receipt"]["bindings"]["consumer_impact_record"], impact_locator
        )
        node = next(node for node in self.plan["nodes"] if node["name"] == "storage-205")
        receipt = ApplyTransport(self.adapter, self.plan)._receipt("preflight:storage-205", node)
        validated = self.adapter._validate_provider_receipt(
            self.plan, "preflight:storage-205", "storage-205", receipt, None
        )
        self.assertEqual(validated["bindings"]["consumer_impact_record"], impact_locator)
        journal = self.adapter._journal_record(self.plan, "dry-run-complete", 0, [])
        self.assertEqual(journal["consumer_impact_record"], impact_locator)

    def test_consumer_impact_change_fails_before_any_provider_callback(self) -> None:
        impact_path = self.fixture._impact_path
        impact_path.write_text(
            json.dumps({"impact": "active"}), encoding="utf-8"
        )
        transport = mock.Mock()
        authority = self._authority()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.execute(
                    self.plan,
                    authority,
                    journal_path=Path(directory) / "journal.json",
                    ledger_path=self.ledger_path,
                    transport=transport,
                    dry_run=False,
                )
        transport.inspect_node.assert_not_called()
        transport.preflight.assert_not_called()
        transport.mutate.assert_not_called()

    def test_provider_receipt_without_consumer_impact_binding_is_rejected(self) -> None:
        node = next(node for node in self.plan["nodes"] if node["name"] == "storage-205")
        receipt = ApplyTransport(self.adapter, self.plan)._receipt("preflight:storage-205", node)
        del receipt["bindings"]["consumer_impact_record"]
        with self.assertRaises(self.adapter.AdapterError):
            self.adapter._validate_provider_receipt(
                self.plan, "preflight:storage-205", "storage-205", receipt, None
            )

    def test_nonce_ledger_symlinked_ancestor_is_rejected_before_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            real_parent = root / "real-parent"
            real_parent.mkdir()
            linked_parent = root / "linked-parent"
            linked_parent.symlink_to(real_parent, target_is_directory=True)
            ledger = linked_parent / "nonce.jsonl"
            self._write_ledger(ledger)
            with self.assertRaises(self.adapter.AdapterError):
                self.adapter.validate_credential_ledger(self.plan, ledger)


if __name__ == "__main__":
    unittest.main()
