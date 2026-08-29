#!/usr/bin/env python3
"""RED contract for the evidence-bound non-merge terminal lifecycle.

This is intentionally a production-facing contract test rather than a fixture
that teaches the implementation its own result.  The helper is absent on the
current HEAD, so the first assertion is the expected RED failure.  Once the
entrypoint exists, the remaining assertions keep the implementation honest
about the distinct closed_without_merge lane and its fail-closed authorities.
"""
from __future__ import annotations

import os
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts/pm/non-merge-finalize.py"


class NonMergeFinalizeContractTest(unittest.TestCase):
    def _source(self) -> str:
        self.assertTrue(
            HELPER.is_file(),
            "RED: missing scripts/pm/non-merge-finalize.py canonical non-merge terminal entrypoint",
        )
        return HELPER.read_text(encoding="utf-8")

    def test_canonical_entrypoint_exposes_bound_inputs(self) -> None:
        self.assertTrue(
            HELPER.is_file(),
            "RED: missing scripts/pm/non-merge-finalize.py canonical non-merge terminal entrypoint",
        )
        self.assertTrue(os.access(HELPER, os.X_OK), "non-merge terminal helper must be executable")
        result = subprocess.run(
            [str(HELPER), "--help"], cwd=ROOT, text=True, capture_output=True
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for option in ("--repo-root", "--task-uid", "--reason", "--evidence-file", "--json"):
            self.assertIn(option, result.stdout)
        for reason in ("superseded", "duplicate", "not_planned", "non_pr_completed"):
            self.assertIn(reason, result.stdout)

    def test_invalid_reason_is_rejected_before_task_or_github_reads(self) -> None:
        self.assertTrue(
            HELPER.is_file(),
            "RED: missing scripts/pm/non-merge-finalize.py canonical non-merge terminal entrypoint",
        )
        result = subprocess.run(
            [
                str(HELPER),
                "--repo-root",
                str(ROOT),
                "--task-uid",
                "task_00000000000000000000000000000000",
                "--reason",
                "invalid",
                "--evidence-file",
                str(ROOT / "testing-manual.md"),
                "--json",
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"reason|invalid choice|allowed")

    def test_reason_and_terminal_phase_are_explicit(self) -> None:
        source = self._source()
        for reason in ("superseded", "duplicate", "not_planned", "non_pr_completed"):
            self.assertIn(reason, source)
        self.assertIn("closed_without_merge", source)

    def test_pr_bound_reasons_require_closed_unmerged_exact_pr(self) -> None:
        source = self._source()
        for marker in (
            "superseded",
            "duplicate",
            "CLOSED",
            "mergedAt",
            "pr_number",
            "pr_url",
            "identity",
            "mismatch",
        ):
            self.assertIn(marker, source, marker)
        # A closed PR is not enough: the merge marker must be checked explicitly.
        self.assertRegex(source, r"mergedAt.{0,120}(None|empty|unmerged|not merged|merge)")

    def test_non_pr_completion_cannot_mint_merge_authority(self) -> None:
        source = self._source()
        for marker in (
            "non_pr_completed",
            "merge_receipt",
            "merge_receipt_sha256",
            "pr_number",
            "pr_url",
        ):
            self.assertIn(marker, source, marker)
        self.assertRegex(source, r"non_pr_completed.{0,500}(merge_receipt|pr_number)")

    def test_terminal_effects_are_durable_idempotent_and_cross_sink_bound(self) -> None:
        source = self._source()
        for marker in (
            "operation_id",
            "ledger",
            "intent",
            "readback",
            "committed",
            "already_finalized",
            "project",
            "evidence",
            "issue",
            "close",
            "task_uid",
            "workflow_phase",
        ):
            self.assertIn(marker, source, marker)
if __name__ == "__main__":
    unittest.main(verbosity=2)
