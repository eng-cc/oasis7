#!/usr/bin/env python3
import os
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "scripts/pm/terminal-task-audit.py"
FINALIZER = ROOT / "scripts/pm/post-merge-finalize.py"
GUARD = ROOT / "scripts/pm/terminal-tombstone-guard.py"
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

    def test_checkout_creator_consumes_terminal_tombstone(self) -> None:
        creator = (ROOT / "scripts/new-task-worktree.sh").read_text(encoding="utf-8")
        self.assertIn("terminal-tombstone-guard.py", creator)
        with tempfile.TemporaryDirectory() as scratch:
            repo = Path(scratch) / "repo"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            common = Path(subprocess.check_output(
                ["git", "-C", str(repo), "rev-parse", "--git-common-dir"], text=True
            ).strip())
            if not common.is_absolute():
                common = repo / common
            receipt_root = common / "oasis7-workflow-receipts" / ("task_" + "0" * 32)
            receipt_root.mkdir(parents=True)
            retired = str(Path(scratch) / "retired")
            (receipt_root / "terminal-tombstone.json").write_text(json.dumps({
                "schema": "oasis7_terminal_tombstone_v1",
                "task_uid": "task_" + "0" * 32,
                "canonical_worktree": retired,
                "task_branch": "task/retired",
                "checkout_recreation_forbidden": True,
            }), encoding="utf-8")
            blocked = subprocess.run([
                str(GUARD), "--repo-root", str(repo), "--worktree", retired,
                "--branch", "task/other",
            ], text=True, capture_output=True)
            self.assertNotEqual(blocked.returncode, 0)
            allowed = subprocess.run([
                str(GUARD), "--repo-root", str(repo), "--worktree", str(Path(scratch) / "new"),
                "--branch", "task/new",
            ], text=True, capture_output=True)
            self.assertEqual(allowed.returncode, 0, allowed.stderr)
            (receipt_root / "terminal-tombstone.json").write_text("{broken", encoding="utf-8")
            corrupt = subprocess.run([
                str(GUARD), "--repo-root", str(repo), "--worktree", str(Path(scratch) / "new"),
                "--branch", "task/new",
            ], text=True, capture_output=True)
            self.assertNotEqual(corrupt.returncode, 0)
            self.assertIn("invalid terminal tombstone", corrupt.stderr)

    def test_audit_validates_receipt_chain_and_physical_absence(self) -> None:
        source = AUDIT.read_text(encoding="utf-8")
        for marker in ("merge_receipt_sha256", "main_sync_receipt_sha256",
                       "phase_receipt_sha256", "pathlib.Path(worktree).exists()",
                       "finalizer_ledger_committed", "project_terminal"):
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
