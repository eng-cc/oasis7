#!/usr/bin/env python3
"""Regression coverage for canonical role/slice ordering in archive closure.

The producer stores batch pairs in canonical (role, slice_id) order while a
review plan preserves its selected role order. These fixtures carry no live
review or publication authority.
"""
from __future__ import annotations
import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
BASE_TEST_PATH = ROOT / "scripts" / "pm" / "publication_helper_archive_closure.test.py"
BASE_SPEC = importlib.util.spec_from_file_location("archive_closure_fixture", BASE_TEST_PATH)
assert BASE_SPEC is not None and BASE_SPEC.loader is not None
BASE = importlib.util.module_from_spec(BASE_SPEC)
sys.modules[BASE_SPEC.name] = BASE
BASE_SPEC.loader.exec_module(BASE)

PLAN_PATH = ROOT / "scripts" / "pm" / "review-plan.py"
PLAN_SPEC = importlib.util.spec_from_file_location("archive_order_review_plan", PLAN_PATH)
assert PLAN_SPEC is not None and PLAN_SPEC.loader is not None
REVIEW_PLAN = importlib.util.module_from_spec(PLAN_SPEC)
sys.modules[PLAN_SPEC.name] = REVIEW_PLAN
PLAN_SPEC.loader.exec_module(REVIEW_PLAN)

TASK_UID = BASE.TASK_UID
HEAD = BASE.HEAD
PRIOR_HEAD = BASE.PRIOR_HEAD
RED_SCRATCH = ROOT / ".pm" / "scratch" / TASK_UID / "archive-order-red"
CURRENT_IDS = ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222"]
PRIOR_IDS = [BASE.PRIOR_SLICE, "44444444-4444-4444-8444-444444444444"]


class PublicationHelperArchiveOrderTests(unittest.TestCase):
    def setUp(self) -> None:
        RED_SCRATCH.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="archive-order-", dir=RED_SCRATCH)
        self.original_roles = BASE.ROLES
        self.original_slices = BASE.SLICES
        self.original_batch = BASE.ArchiveClosureFixture._batch

    def tearDown(self) -> None:
        BASE.ROLES = self.original_roles
        BASE.SLICES = self.original_slices
        BASE.ArchiveClosureFixture._batch = self.original_batch
        self.temp.cleanup()

    def fixture(self, *, current_ordered: bool) -> BASE.ArchiveClosureFixture:
        # Preserve role selection order in the plan while matching the public
        # review-batch-epoch producer's canonical sort in the batch.
        BASE.ROLES = (["qa_engineer", "repository_health_engineer"] if current_ordered
                      else ["repository_health_engineer", "qa_engineer"])
        BASE.SLICES = CURRENT_IDS
        original_batch = self.original_batch
        def canonical_batch(instance, head, slices, evidence):
            ordered = sorted(slices, key=lambda row: (row["role"], row["slice_id"]))
            return original_batch(instance, head, ordered, evidence)
        BASE.ArchiveClosureFixture._batch = canonical_batch
        return BASE.ArchiveClosureFixture(Path(self.temp.name) / "task")

    def collect(self, fixture: BASE.ArchiveClosureFixture):
        collector = getattr(BASE.HELPER, "_collect_review_archive_closure", None)
        self.assertTrue(callable(collector), "canonical archive closure collector is unavailable")
        plan = json.loads(fixture.plan_path.read_text(encoding="utf-8"))
        return collector(fixture.root, TASK_UID, HEAD, fixture.plan_path)

    def expand_prior_order_difference(self, fixture: BASE.ArchiveClosureFixture) -> Path:
        """Replace the one-slice prior context with a valid two-role ordered plan."""
        prior_roles = ["runtime_engineer", "qa_engineer"]
        prior_slices = [
            {"role": role, "slice_id": slice_id}
            for role, slice_id in zip(prior_roles, PRIOR_IDS)
        ]
        evidence = "d" * 64
        prior_epoch, prior_batch = fixture._batch(PRIOR_HEAD, prior_slices, evidence)
        prior_batch_path = fixture.scratch / "review-batches" / f"{prior_epoch}.json"
        fixture.put_json(prior_batch_path, prior_batch)

        prior_rows = []
        for role, slice_id in zip(prior_roles, PRIOR_IDS):
            return_path = fixture.scratch / "review-returns" / f"{slice_id}.json"
            returned = {
                "schema": "oasis7-review-return/v1", "task_uid": TASK_UID,
                "role": role, "slice_id": slice_id, "head": PRIOR_HEAD,
                "epoch": prior_epoch, "status": "completed", "disposition": "no_findings",
                "findings": [], "residual_risk": "fixture only",
            }
            return_raw = BASE.canonical(returned) + b"\n"
            fixture.put(return_path, return_raw)
            prior_rows.append({
                "role": role, "slice_id": slice_id, "task_uid": TASK_UID,
                "head": PRIOR_HEAD, "epoch": prior_epoch, "status": "completed",
                "activation": "message-assigned", "context_delivery": "minimal-task-packet",
                "actual_runtime": "inherit current parent selection", "scope_verdict": "approved",
                "risk_verdict": "approved", "findings": "no_findings", "residual_risk": "fixture only",
                "artifact_digest": BASE.digest(return_raw), "artifacts": [fixture.rel(return_path)],
            })
        prior_ledger_path = fixture.scratch / "review-plans" / "preflight" / f"{prior_epoch}.jsonl"
        prior_ledger_raw = b"".join((json.dumps(row, sort_keys=True) + "\n").encode() for row in prior_rows)
        fixture.put(prior_ledger_path, prior_ledger_raw)

        prior_collection_path = prior_batch_path.with_name(f"{prior_epoch}.collection.json")
        prior_collection = {
            "schema": "oasis7-review-collection/v1", "status": "passed",
            "epoch": prior_epoch, "task_uid": TASK_UID, "frozen_head": PRIOR_HEAD,
            "ledger_digest": BASE.digest(prior_ledger_raw), "roles": sorted(prior_roles),
        }
        collection_raw = BASE.canonical(prior_collection) + b"\n"
        fixture.put(prior_collection_path, collection_raw)

        prior_plan_path = fixture.scratch / "review-plans" / f"{prior_epoch}.json"
        prior_plan = {
            "schema": "oasis7-review-plan/v2", "task_uid": TASK_UID,
            "frozen_head": PRIOR_HEAD, "epoch": prior_epoch, "roles": prior_roles,
            "expected_slices": prior_slices, "batch_path": fixture.rel(prior_batch_path),
            "collection_path": fixture.rel(prior_collection_path),
            "preflight": {"status": "incomplete", "ledger_path": fixture.rel(prior_ledger_path)},
            "source_review_digest": evidence, "integration_ci_digest": "e" * 64,
        }
        prior_plan_raw = BASE.canonical(prior_plan) + b"\n"
        fixture.put(prior_plan_path, prior_plan_raw)
        context = {
            "schema": "oasis7-review-context/v1", "authority": "context_only",
            "task_uid": TASK_UID, "current_head_oid": HEAD, "prior_head_oid": PRIOR_HEAD,
            "prior_plan_path": fixture.rel(prior_plan_path),
            "prior_plan_digest": BASE.digest(prior_plan_raw), "prior_epoch": prior_epoch,
            "prior_source_review_digest": evidence, "prior_integration_ci_digest": "e" * 64,
            "prior_roles": prior_roles, "prior_collection_path": fixture.rel(prior_collection_path),
            "prior_collection_digest": BASE.digest(collection_raw),
            "prior_collection_ledger_digest": BASE.digest(prior_ledger_raw),
            "delta_paths": [], "delta_paths_digest": BASE.digest(BASE.canonical([])),
            "delta_patch_digest": "f" * 64,
        }

        plan = json.loads(fixture.plan_path.read_text(encoding="utf-8"))
        plan["incremental_review_context"] = context
        plan_raw = BASE.canonical(plan) + b"\n"
        fixture.plan_path.write_bytes(plan_raw)
        packet_values = {}
        for ref in plan["packet_refs"]:
            packet_path = fixture.root / ref["packet_ref"]
            packet = json.loads(packet_path.read_text(encoding="utf-8"))
            packet["review_context"] = context
            packet.pop("packet_digest", None)
            packet["packet_digest"] = BASE.digest(BASE.canonical(packet))
            packet_raw = BASE.canonical(packet) + b"\n"
            packet_path.write_bytes(packet_raw)
            packet_values[(packet["slice"]["role"], packet["slice"]["slice_id"])] = (packet, packet_raw)

        handoff_path = fixture.scratch / "review-handoffs" / f"{plan['epoch']}.json"
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        handoff["plan_sha256"] = BASE.digest(plan_raw)
        for row in handoff["rows"]:
            packet, _raw = packet_values[(row["role"], row["slice_id"])]
            row["packet_digest"] = packet["packet_digest"]
        handoff.pop("handoff_digest", None)
        handoff["handoff_digest"] = BASE.digest(BASE.canonical(handoff))
        handoff_path.write_bytes(BASE.canonical(handoff) + b"\n")

        origin_path = fixture.scratch / "publication-helper-review-origin.json"
        origin = json.loads(origin_path.read_text(encoding="utf-8"))
        origin["review_plan"] = {"value": plan, "raw_sha256": BASE.digest(plan_raw)}
        for row in origin["packets"]:
            packet, packet_raw = packet_values[(row["role"], row["slice_id"])]
            row["value"] = packet
            row["raw_sha256"] = BASE.digest(packet_raw)
        fixture.put_json(origin_path, origin)
        return prior_plan_path

    def test_current_accepts_canonical_batch_order_for_plan_role_order(self) -> None:
        fixture = self.fixture(current_ordered=False)
        plan = json.loads(fixture.plan_path.read_text(encoding="utf-8"))
        batch = json.loads((fixture.root / plan["batch_path"]).read_text(encoding="utf-8"))
        self.assertNotEqual(plan["expected_slices"], batch["expected_slices"])
        self.assertEqual(sorted(plan["expected_slices"], key=lambda row: (row["role"], row["slice_id"])),
                         batch["expected_slices"])
        try:
            rows = self.collect(fixture)
        except ValueError as exc:
            self.fail(f"valid current plan/batch slice identity was rejected: {exc}")
        self.assertTrue(rows)

    def test_prior_accepts_canonical_batch_order_for_prior_role_order(self) -> None:
        fixture = self.fixture(current_ordered=True)
        prior_plan_path = self.expand_prior_order_difference(fixture)
        prior_plan = json.loads(prior_plan_path.read_text(encoding="utf-8"))
        prior_batch = json.loads((fixture.root / prior_plan["batch_path"]).read_text(encoding="utf-8"))
        self.assertNotEqual(prior_plan["expected_slices"], prior_batch["expected_slices"])
        self.assertEqual(sorted(prior_plan["expected_slices"], key=lambda row: (row["role"], row["slice_id"])),
                         prior_batch["expected_slices"])
        try:
            rows = self.collect(fixture)
        except ValueError as exc:
            self.fail(f"valid prior plan/batch slice identity was rejected: {exc}")
        self.assertTrue(rows)

    def test_rejects_current_mismatched_slice_identity(self) -> None:
        fixture = self.fixture(current_ordered=True)
        plan = json.loads(fixture.plan_path.read_text(encoding="utf-8"))
        batch_path = fixture.root / plan["batch_path"]
        batch = json.loads(batch_path.read_text(encoding="utf-8"))
        batch["expected_slices"][0]["slice_id"] = "55555555-5555-4555-8555-555555555555"
        batch_path.write_bytes(BASE.canonical(batch) + b"\n")
        with self.assertRaises((ValueError, OSError, SystemExit)):
            self.collect(fixture)

    def test_rejects_prior_mismatched_slice_identity(self) -> None:
        fixture = self.fixture(current_ordered=True)
        prior_plan_path = self.expand_prior_order_difference(fixture)
        prior_plan = json.loads(prior_plan_path.read_text(encoding="utf-8"))
        prior_batch_path = fixture.root / prior_plan["batch_path"]
        prior_batch = json.loads(prior_batch_path.read_text(encoding="utf-8"))
        prior_batch["expected_slices"][0]["slice_id"] = "55555555-5555-4555-8555-555555555555"
        prior_batch_path.write_bytes(BASE.canonical(prior_batch) + b"\n")
        with self.assertRaises((ValueError, OSError, SystemExit)):
            self.collect(fixture)

    def test_rejects_duplicate_current_role_slice_pair(self) -> None:
        fixture = self.fixture(current_ordered=True)
        plan = json.loads(fixture.plan_path.read_text(encoding="utf-8"))
        batch_path = fixture.root / plan["batch_path"]
        batch = json.loads(batch_path.read_text(encoding="utf-8"))
        duplicate = copy.deepcopy(plan["expected_slices"][0])
        plan["expected_slices"].append(duplicate)
        plan["roles"].append(duplicate["role"])
        batch["expected_slices"].append(copy.deepcopy(duplicate))
        fixture.plan_path.write_bytes(BASE.canonical(plan) + b"\n")
        batch_path.write_bytes(BASE.canonical(batch) + b"\n")
        with self.assertRaises((ValueError, OSError, SystemExit)):
            self.collect(fixture)


if __name__ == "__main__":
    unittest.main(verbosity=2)
