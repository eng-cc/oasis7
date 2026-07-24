#!/usr/bin/env python3
"""Behavior contract for deterministic, reusable review planning."""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("review-plan.py")
TASK = "task_" + "1" * 32
HEAD = "a" * 40
EVIDENCE = "b" * 64


class ReviewPlanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.out = self.root / "plan.json"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_plan(self, *extra: str, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", HEAD, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--out", str(self.out), *extra],
            text=True,
            capture_output=True,
        )
        if ok and result.returncode != 0:
            self.fail(f"command failed: {result.stderr}")
        if not ok and result.returncode == 0:
            self.fail(f"command unexpectedly passed: {result.stdout}")
        return result

    def plan(self, *extra: str) -> dict[str, object]:
        return json.loads(self.run_plan(*extra).stdout)

    def test_explicit_document_risk_class_selects_the_minimum_deterministic_roles(self) -> None:
        plan = self.plan()
        self.assertEqual(
            ["repository_health_engineer", "qa_engineer"],
            plan["roles"],
        )
        self.assertEqual(plan["roles"], [item["role"] for item in plan["expected_slices"]])
        self.assertEqual(2, len(plan["packet_refs"]))

    def test_unchanged_identity_reuses_stable_slice_uuids_and_epoch(self) -> None:
        first = self.plan()
        retry = self.plan()
        self.assertTrue(retry["reused"])
        self.assertEqual(first["epoch"], retry["epoch"])
        self.assertEqual(first["expected_slices"], retry["expected_slices"])
        self.assertEqual(first["packet_refs"], retry["packet_refs"])
        for item in retry["expected_slices"]:
            self.assertRegex(item["slice_id"],
                             r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")

    def test_unchanged_identity_reuses_batch_with_a_different_explicit_plan_path(self) -> None:
        first = self.plan()
        alternate_out = self.root / "alternate-plan.json"
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", HEAD, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--out", str(alternate_out)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        retry = json.loads(result.stdout)
        self.assertTrue(retry["reused"])
        self.assertEqual(first["epoch"], retry["epoch"])
        self.assertEqual(first["expected_slices"], retry["expected_slices"])
        self.assertEqual(first["batch_path"], retry["batch_path"])

    def test_preflight_only_materializes_incomplete_artifacts_without_collection(self) -> None:
        artifacts = self.root / "preflight"
        plan = self.plan("--preflight-dir", str(artifacts))
        self.assertEqual("incomplete", plan["preflight"]["status"])
        self.assertFalse(Path(plan["collection_path"]).exists())
        for artifact in plan["preflight"]["artifact_paths"]:
            returned = json.loads(Path(artifact).read_text(encoding="utf-8"))
            self.assertEqual("incomplete", returned["status"])
            self.assertEqual(plan["epoch"], returned["epoch"])
            self.assertNotEqual("passed", returned["disposition"])

    def test_preflight_fails_closed_when_an_existing_ledger_is_tampered(self) -> None:
        artifacts = self.root / "preflight-tampered-ledger"
        plan = self.plan("--preflight-dir", str(artifacts))
        ledger = Path(plan["preflight"]["ledger_path"])
        ledger.write_text("", encoding="utf-8")
        for artifact in plan["preflight"]["artifact_paths"]:
            returned = json.loads(Path(artifact).read_text(encoding="utf-8"))
            self.assertEqual("incomplete", returned["status"])

        result = self.run_plan("--preflight-dir", str(artifacts), ok=False)
        self.assertRegex(result.stderr.lower(), r"ledger|inconsistent")

    def test_head_evidence_or_role_drift_never_reuses_a_previous_plan(self) -> None:
        first = self.plan()
        variations = (
            ("--head", "c" * 40),
            ("--evidence-digest", "d" * 64),
            ("--change-class", "domain-semantic-doc", "--domain-role", "runtime_engineer"),
        )
        for index, variation in enumerate(variations):
            with self.subTest(variation=variation):
                drifted_out = self.root / f"drift-{index}.json"
                args = list(variation) + ["--out", str(drifted_out)]
                result = subprocess.run(
                    [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
                     "--head", HEAD, "--evidence-digest", EVIDENCE,
                     "--change-class", "workflow-doc", *args],
                    text=True,
                    capture_output=True,
                )
                self.assertEqual(0, result.returncode, result.stderr)
                drifted = json.loads(result.stdout)
                self.assertFalse(drifted["reused"])
                self.assertNotEqual(first["epoch"], drifted["epoch"])


if __name__ == "__main__":
    unittest.main()
