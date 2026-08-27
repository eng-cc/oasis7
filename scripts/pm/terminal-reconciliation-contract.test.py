#!/usr/bin/env python3
import os
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "scripts/pm/terminal-task-audit.py"
FINALIZER = ROOT / "scripts/pm/post-merge-finalize.py"
CANONICAL = ROOT / "doc/engineering/workflow/source-of-truth.md"


class TerminalReconciliationContractTest(unittest.TestCase):
    def test_repo_exposes_machine_readable_terminal_audit(self) -> None:
        self.assertTrue(AUDIT.is_file(), "missing terminal-task-audit.py")
        self.assertTrue(os.access(AUDIT, os.X_OK), "terminal audit must be executable")
        help_result = subprocess.run(
            [str(AUDIT), "--help"], cwd=ROOT, text=True, capture_output=True
        )
        self.assertEqual(help_result.returncode, 0, help_result.stderr)
        for marker in ("--task-uid", "--json", "--resume-finalizer"):
            self.assertIn(marker, help_result.stdout)

    def test_finalizer_persists_terminal_tombstone(self) -> None:
        source = FINALIZER.read_text(encoding="utf-8")
        for marker in (
            "terminal-tombstone.json",
            "oasis7_terminal_tombstone_v1",
            "checkout_recreation_forbidden",
        ):
            self.assertIn(marker, source)

    def test_canonical_truth_defines_cross_sink_terminal_invariant(self) -> None:
        canonical = CANONICAL.read_text(encoding="utf-8")
        for marker in (
            "terminal tombstone",
            "checkout_recreation_forbidden",
            "terminal-task-audit.py",
            "`done` is not terminal reconciliation",
        ):
            self.assertIn(marker, canonical)


if __name__ == "__main__":
    unittest.main()
