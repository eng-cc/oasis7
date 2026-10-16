#!/usr/bin/env python3
"""RED tests for the triad adapter's exact-five identity-v2 admission.

The production adapter currently validates only receipt verdict fields.  This
fixture keeps the five-node admission contract local and secret-free, then
requires the adapter to bind every receipt to the governed node, task, head,
context, registry, provider proof, and verifier pins before admission.
"""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ADAPTER_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-triad-host-adapter.py"


def load_adapter():
    spec = importlib.util.spec_from_file_location("triad_identity_v2_admission_adapter", ADAPTER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("unable to load governed triad host adapter")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ADAPTER = load_adapter()

NETWORK_ID = "oasis7-public-testnet-governed-20260606"
TASK_UID = "task_cf4163ccd876402d8317b569019e12a6"
HEAD_OID = "a4e86e0efca339fb4e9179b1cfcffa793539da16"
CAPTURE_WINDOW_ID = "identity-v2-window-fixture-001"
ROTATION_EPOCH = "rotation-epoch-20260901-001"
PEER_REGISTRY_SHA256 = "1" * 64
PEER_REGISTRY_EPOCH = "managed-peer-registry-fixture-001"
CONTEXT_DIGEST = "5" * 64
TRUST_CONFIG_SHA256 = "2" * 64
PROVIDER_REGISTRY_SHA256 = "3" * 64
VERIFIER_EXECUTABLE_SHA256 = "4" * 64

NODES = (
    {
        "node_name": "storage-205",
        "node_id": "triad-testnet-storage",
        "peer_id": "peerstorage205",
        "role": "validator",
        "host": "root@39.104.205.67",
    },
    {
        "node_name": "sequencer-204",
        "node_id": "triad-testnet-sequencer",
        "peer_id": "peersequencer204",
        "role": "validator",
        "host": "root@39.104.204.172",
    },
    {
        "node_name": "linux-lan-observer",
        "node_id": "triad-testnet-local",
        "peer_id": "peerlinuxlanobserver",
        "role": "observer",
        "host": "observer@linux-lan",
    },
    {
        "node_name": "windows-observer",
        "node_id": "triad-testnet-windows-observer",
        "peer_id": "peerwindowsobserver",
        "role": "observer",
        "host": "observer@windows-lan",
    },
    {
        "node_name": "macos-observer",
        "node_id": "triad-testnet-fourth-local",
        "peer_id": "peermacosobserver",
        "role": "observer",
        "host": "observer@macos-lan",
    },
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class TriadIdentityV2AdmissionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="oasis7-triad-identity-v2-admission-")
        self.root = Path(self.temp.name)
        self.transaction = self._transaction()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _receipt(self, node: dict[str, str], *, historical_only: bool = False) -> dict[str, object]:
        node_index = next(index for index, value in enumerate(NODES, 1) if value["node_name"] == node["node_name"])
        request_token = f"{node_index:064x}"
        return {
            "schema_version": "oasis7.identity_v2_verification_receipt.v1",
            "mode": "historical_audit" if historical_only else "current_admission",
            "evaluation_time": "2026-09-20T00:00:00Z",
            "raw_v1_sha256": "a" * 64,
            "canonical_payload_sha256": "b" * 64,
            "envelope_sha256": "c" * 64,
            "signer_id": "governance-signer",
            "public_key_sha256": "d" * 64,
            "trust_config_sha256": TRUST_CONFIG_SHA256,
            "provider_registry_sha256": PROVIDER_REGISTRY_SHA256,
            "verifier_executable_sha256": VERIFIER_EXECUTABLE_SHA256,
            "network_id": NETWORK_ID,
            "proof_ref": f"proof-v1:{request_token}",
            "proof_claims_sha256": "e" * 64,
            "task_uid": TASK_UID,
            "head_oid": HEAD_OID,
            "node_id": node["node_id"],
            "peer_id": node["peer_id"],
            "capture_window_id": CAPTURE_WINDOW_ID,
            "rotation_epoch": ROTATION_EPOCH,
            "historical_only": historical_only,
            "apply_authorized": not historical_only,
            "authority_scope": "deployed-governance-root",
            "verified": True,
        }

    def _transaction(self) -> dict[str, object]:
        receipts: list[dict[str, object]] = []
        entries: list[dict[str, object]] = []
        for node in NODES:
            path = self.root / f"{node['node_name']}.verification.json"
            path.write_text(json.dumps(self._receipt(node), sort_keys=True) + "\n", encoding="utf-8")
            descriptor = {"path": str(path), "sha256": digest(path), "size_bytes": path.stat().st_size}
            receipts.append({**node, **descriptor})
            entries.append({**node, "verification": descriptor})
        return {
            "proof": {
                "identity_v2": {
                    "schema_version": "oasis7.identity_v2_evidence_map.v2",
                    "status": "verified",
                    "verified": True,
                    "network_id": NETWORK_ID,
                    "task_uid": TASK_UID,
                    "head_oid": HEAD_OID,
                    "capture_window_id": CAPTURE_WINDOW_ID,
                    "context_digest": CONTEXT_DIGEST,
                    "rotation_epoch": ROTATION_EPOCH,
                    "peer_registry_sha256": PEER_REGISTRY_SHA256,
                    "peer_registry_epoch": PEER_REGISTRY_EPOCH,
                    "context": {
                        "network_id": NETWORK_ID,
                        "task_uid": TASK_UID,
                        "head_oid": HEAD_OID,
                        "capture_window_id": CAPTURE_WINDOW_ID,
                        "context_digest": CONTEXT_DIGEST,
                        "peer_registry_sha256": PEER_REGISTRY_SHA256,
                        "peer_registry_epoch": PEER_REGISTRY_EPOCH,
                        "rotation_epoch": ROTATION_EPOCH,
                    },
                    "entries": entries,
                    "receipts": receipts,
                    "provider": {
                        "request_ids": [f"req-v2:{index:064x}" for index in range(1, len(NODES) + 1)],
                        "proof_refs": [f"proof-v1:{index:064x}" for index in range(1, len(NODES) + 1)],
                    },
                    "verifier": {
                        "trust_config_sha256": TRUST_CONFIG_SHA256,
                        "provider_registry_sha256": PROVIDER_REGISTRY_SHA256,
                        "verifier_executable_sha256": VERIFIER_EXECUTABLE_SHA256,
                    },
                }
            }
        }

    def _admission(self, transaction: dict[str, object] | None = None) -> dict[str, object]:
        value = transaction or self.transaction
        return value["proof"]["identity_v2"]  # type: ignore[index,return-value]

    def _assert_rejected(self, transaction: dict[str, object], message: str) -> None:
        with self.subTest(case=message), self.assertRaisesRegex(SystemExit, r"(?i)identity-v2|receipt|binding|authority|provider|verifier|registry"):
            ADAPTER.validate_identity_v2(transaction)

    def test_current_exact_five_control_is_admissible(self) -> None:
        """The fixture is a valid current-admission control, not a live claim."""
        ADAPTER.validate_identity_v2(self.transaction)

    def test_rejects_missing_extra_duplicate_and_unexpected_nodes(self) -> None:
        for mutation in ("missing", "extra", "duplicate", "unexpected"):
            changed = copy.deepcopy(self.transaction)
            admission = self._admission(changed)
            if mutation == "missing":
                admission["entries"].pop()
                admission["receipts"].pop()
            elif mutation == "extra":
                admission["entries"].append(copy.deepcopy(admission["entries"][0]))
                admission["receipts"].append(copy.deepcopy(admission["receipts"][0]))
            elif mutation == "duplicate":
                admission["entries"][1]["node_name"] = admission["entries"][0]["node_name"]
                admission["receipts"][1]["node_name"] = admission["receipts"][0]["node_name"]
            else:
                admission["entries"][0]["node_name"] = "attacker-node"
                admission["receipts"][0]["node_name"] = "attacker-node"
            self._assert_rejected(changed, mutation)

    def test_rejects_mixed_node_role_host_peer_task_head_context_or_registry(self) -> None:
        mutations = (
            ("node-id", lambda admission: admission["entries"][0].__setitem__("node_id", "wrong-node")),
            ("role", lambda admission: admission["entries"][0].__setitem__("role", "observer")),
            ("host", lambda admission: admission["entries"][0].__setitem__("host", "root@attacker")),
            ("peer", lambda admission: admission["entries"][0].__setitem__("peer_id", "wrong-peer")),
            ("task", lambda admission: admission.__setitem__("task_uid", "other-task")),
            ("head", lambda admission: admission.__setitem__("head_oid", "f" * 40)),
            ("context", lambda admission: admission["context"].__setitem__("capture_window_id", "other-window")),
            ("network", lambda admission: admission.__setitem__("network_id", "attacker-network")),
            ("epoch", lambda admission: admission.__setitem__("rotation_epoch", "old-epoch")),
            ("registry", lambda admission: admission.__setitem__("peer_registry_sha256", "f" * 64)),
            ("context digest", lambda admission: admission.__setitem__("context_digest", "f" * 64)),
        )
        for label, mutate in mutations:
            changed = copy.deepcopy(self.transaction)
            mutate(self._admission(changed))
            self._assert_rejected(changed, label)

    def test_rejects_historical_or_unauthorized_receipt(self) -> None:
        for field, value in (("historical_only", True), ("apply_authorized", False), ("mode", "historical_audit")):
            changed = copy.deepcopy(self.transaction)
            item = changed["proof"]["identity_v2"]["receipts"][0]  # type: ignore[index]
            item[field] = value
            path = Path(item["path"])
            receipt = json.loads(path.read_text(encoding="utf-8"))
            receipt[field] = value
            path.write_text(json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8")
            item["sha256"] = digest(path)
            self._assert_rejected(changed, field)

    def test_rejects_provider_replay_and_unpinned_verifier_or_registry(self) -> None:
        mutations = (
            ("provider request replay", lambda admission: admission["provider"]["request_ids"].__setitem__(1, admission["provider"]["request_ids"][0])),
            ("provider proof replay", lambda admission: admission["provider"]["proof_refs"].__setitem__(1, admission["provider"]["proof_refs"][0])),
            ("provider pin missing", lambda admission: admission["verifier"].pop("provider_registry_sha256")),
            ("verifier pin missing", lambda admission: admission["verifier"].pop("verifier_executable_sha256")),
            ("trust pin", lambda admission: admission["verifier"].__setitem__("trust_config_sha256", "f" * 64)),
            ("provider registry pin", lambda admission: admission["verifier"].__setitem__("provider_registry_sha256", "f" * 64)),
            ("verifier pin", lambda admission: admission["verifier"].__setitem__("verifier_executable_sha256", "f" * 64)),
        )
        for label, mutate in mutations:
            changed = copy.deepcopy(self.transaction)
            mutate(self._admission(changed))
            self._assert_rejected(changed, label)

    def test_rejects_cross_paired_receipt_metadata(self) -> None:
        changed = copy.deepcopy(self.transaction)
        first, second = changed["proof"]["identity_v2"]["receipts"][:2]  # type: ignore[index]
        first["path"], second["path"] = second["path"], first["path"]
        first["sha256"], second["sha256"] = second["sha256"], first["sha256"]
        self._assert_rejected(changed, "cross-paired receipt")


if __name__ == "__main__":
    unittest.main()
