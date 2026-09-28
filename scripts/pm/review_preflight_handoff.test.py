#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
BATCH_SCRIPT = HERE / "review-batch-epoch.py"
HANDOFF_SCRIPT = HERE / "review_preflight_handoff.py"
TASK = "task_" + "1" * 32
ROLE = "qa_engineer"
SLICE = "11111111-1111-4111-8111-111111111111"
HEAD = "a" * 40
SCOPE_OID = "c" * 40
REPOSITORY = "eng-cc/oasis7"
ACTIVATION = "message-assigned"
CONTEXT_DELIVERY = "minimal-task-packet"
MODEL_REASONING = "inherited/unverified"
RUNTIME_REASON = (
    "message-assigned fallback; adapter inactive on this surface; "
    "actual runtime/model/reasoning unverified"
)


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def load_handoff_module():
    spec = importlib.util.spec_from_file_location("review_preflight_handoff_under_test", HANDOFF_SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HANDOFF = load_handoff_module()


class ReviewPreflightHandoffTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.task_root = self.root / ".pm" / "scratch" / TASK

    def tearDown(self) -> None:
        self.temp.cleanup()

    def command_json(self, *args: str) -> dict[str, object]:
        result = subprocess.run(
            [sys.executable, str(BATCH_SCRIPT), "--root", str(self.root), *args],
            text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        return json.loads(result.stdout)

    def make_fixture(self) -> dict[str, object]:
        expected_slices = [{"role": ROLE, "slice_id": SLICE}]
        source_identity = {
            "task_uid": TASK,
            "bootstrap_epoch": 1,
            "repository": REPOSITORY,
            "pr_number": 1,
            "source_head_oid": HEAD,
            "source_scope_oid": SCOPE_OID,
            "changed_paths_digest": "b" * 64,
            "ordered_role_ids": [ROLE],
            "role_contract_digest": "d" * 64,
            "review_policy_digest": "e" * 64,
            "input_contract_digest": "f" * 64,
        }
        source_digest = digest(source_identity)
        batch_path = self.task_root / "review-batches" / "batch.json"
        batch_path.parent.mkdir(parents=True, exist_ok=True)
        batch = self.command_json(
            "create", "--task-uid", TASK, "--head", HEAD,
            "--evidence-digest", source_digest, "--slice", f"{ROLE}={SLICE}",
            "--out", str(batch_path),
        )
        epoch = str(batch["epoch"])
        preflight_dir = self.task_root / "review-plans" / "preflight"
        preflight = self.command_json(
            "preflight", "--batch", str(batch_path), "--out-dir", str(preflight_dir)
        )
        return_path = preflight_dir / f"{SLICE}.json"
        returned = json.loads(return_path.read_text(encoding="utf-8"))
        returned.update({
            "status": "completed",
            "activation": ACTIVATION,
            "context_delivery": CONTEXT_DELIVERY,
            "actual_runtime": f"{MODEL_REASONING}: {RUNTIME_REASON}",
            "scope_verdict": "approved",
            "risk_verdict": "approved",
            "disposition": "no_findings",
            "findings": [],
            "residual_risk": "fixture risk",
        })
        return_path.write_text(json.dumps(returned, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

        packet_path = self.task_root / "slice-packets" / f"{SLICE}.json"
        packet_path.parent.mkdir(parents=True, exist_ok=True)
        packet: dict[str, object] = {
            "schema": "oasis7-subagent-task-packet/v1",
            "identity": {"task_uid": TASK, "head": HEAD, "base_sha": SCOPE_OID},
            "slice": {
                "slice_id": SLICE,
                "role": ROLE,
                "role_activation": "message_assigned_adapter_inactive",
                "context_delivery_mode": "minimal_head_bound_task_packet",
                "actual_dispatched_model_reasoning": MODEL_REASONING,
                "actual_runtime_evidence_reason": RUNTIME_REASON,
            },
        }
        packet["packet_digest"] = digest(packet)
        packet_path.write_text(json.dumps(packet, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

        applicability_identity = {
            key: source_identity[key] for key in (
                "changed_paths_digest", "input_contract_digest", "ordered_role_ids",
                "role_contract_digest", "review_policy_digest",
            )
        }
        plan_path = self.task_root / "review-plans" / "plan.json"
        plan_path.parent.mkdir(parents=True, exist_ok=True)
        plan = {
            "schema": "oasis7-review-plan/v2",
            "task_uid": TASK,
            "frozen_head": HEAD,
            "comparison_ref": "refs/heads/main",
            "comparison_oid": SCOPE_OID,
            "source_scope_oid": SCOPE_OID,
            "source_review_identity": source_identity,
            "source_review_digest": source_digest,
            "relevant_evidence_digest": source_digest,
            "professional_review_applicability": {
                "identity": applicability_identity,
                "identity_digest": digest(applicability_identity),
                "verified": True,
            },
            "epoch": epoch,
            "batch_path": str(batch_path),
            "collection_path": str(batch_path.with_name("batch.collection.json")),
            "roles": [ROLE],
            "expected_slices": expected_slices,
            "packet_refs": [{
                "role": ROLE, "slice_id": SLICE,
                "packet_ref": packet_path.relative_to(self.root).as_posix(),
            }],
            "preflight": {"status": "incomplete", "ledger_path": preflight["ledger_path"]},
        }
        plan_path.write_text(json.dumps(plan, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        created = HANDOFF.create_handoff(self.root, plan_path)
        return {
            "batch": batch,
            "batch_path": batch_path,
            "epoch": epoch,
            "handoff_path": Path(str(created["handoff_path"])),
            "plan": plan,
            "plan_path": plan_path,
            "packet_path": packet_path,
            "return_path": return_path,
            "source_digest": source_digest,
        }

    def rebind_return_and_handoff(self, fixture: dict[str, object], field: str, value: str) -> None:
        return_path = Path(str(fixture["return_path"]))
        returned = json.loads(return_path.read_text(encoding="utf-8"))
        returned[field] = value
        raw = json.dumps(returned, ensure_ascii=False, sort_keys=True).encode() + b"\n"
        return_path.write_bytes(raw)
        handoff_path = Path(str(fixture["handoff_path"]))
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        row = handoff["rows"][0]
        row["return_sha256"] = hashlib.sha256(raw).hexdigest()
        row["findings_digest"] = digest(returned["findings"])
        handoff["handoff_digest"] = digest({key: item for key, item in handoff.items() if key != "handoff_digest"})
        handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

    def test_accepts_return_metadata_bound_to_slice_packet(self) -> None:
        fixture = self.make_fixture()
        validated = HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))
        returned = validated["returns"][(ROLE, SLICE)][2]
        self.assertEqual(ACTIVATION, returned["activation"])
        self.assertEqual(CONTEXT_DELIVERY, returned["context_delivery"])
        self.assertEqual(f"{MODEL_REASONING}: {RUNTIME_REASON}", returned["actual_runtime"])

    def test_rejects_return_activation_conflicting_with_slice_packet(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_and_handoff(fixture, "activation", "named-role-adapter")
        with self.assertRaisesRegex(HANDOFF.ContractError, "activation|packet|metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_return_context_conflicting_with_slice_packet(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_and_handoff(fixture, "context_delivery", "full-history")
        with self.assertRaisesRegex(HANDOFF.ContractError, "context|packet|metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_return_runtime_conflicting_with_slice_packet(self) -> None:
        fixture = self.make_fixture()
        self.rebind_return_and_handoff(fixture, "actual_runtime", "gpt-6-astra: high")
        with self.assertRaisesRegex(HANDOFF.ContractError, "runtime|packet|metadata"):
            HANDOFF.validate_handoff(self.root, Path(str(fixture["handoff_path"])))

    def test_rejects_comparison_oid_different_from_source_review_scope(self) -> None:
        fixture = self.make_fixture()
        plan_path = Path(str(fixture["plan_path"]))
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
        plan["comparison_oid"] = "9" * 40
        plan_raw = json.dumps(plan, ensure_ascii=False, sort_keys=True).encode() + b"\n"
        plan_path.write_bytes(plan_raw)

        handoff_path = Path(str(fixture["handoff_path"]))
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        handoff["comparison_oid"] = plan["comparison_oid"]
        handoff["plan_sha256"] = hashlib.sha256(plan_raw).hexdigest()
        handoff["handoff_digest"] = digest({key: item for key, item in handoff.items() if key != "handoff_digest"})
        handoff_path.write_text(json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")

        batch = json.loads(Path(str(fixture["batch_path"])).read_text(encoding="utf-8"))
        self.assertEqual(str(fixture["source_digest"]), batch["relevant_evidence_digest"])
        self.assertEqual(
            digest({key: batch[key] for key in ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}),
            batch["epoch"],
        )
        with self.assertRaisesRegex(HANDOFF.ContractError, "comparison|scope"):
            HANDOFF.validate_handoff(self.root, handoff_path)


if __name__ == "__main__":
    unittest.main()
