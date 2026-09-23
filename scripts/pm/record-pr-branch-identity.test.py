#!/usr/bin/env python3
"""Regression coverage for repaired-PR branch identity admission."""
from __future__ import annotations

import importlib.util
import json
import pathlib
import subprocess
import tempfile
import types
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/pm/github-project-task.py"
SPEC = importlib.util.spec_from_file_location("github_project_task_under_test", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class RecordPrBranchIdentityTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.worktree = self.root / "task-worktree"
        subprocess.run(["git", "init", "-q", str(self.worktree)], check=True)
        subprocess.run(["git", "-C", str(self.worktree), "checkout", "-qb", "task/canonical"], check=True)
        subprocess.run(["git", "-C", str(self.worktree), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.worktree), "config", "user.name", "Test"], check=True)
        (self.worktree / "file").write_text("task\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.worktree), "add", "file"], check=True)
        subprocess.run(["git", "-C", str(self.worktree), "commit", "-qm", "task"], check=True)
        self.head = subprocess.check_output(
            ["git", "-C", str(self.worktree), "rev-parse", "HEAD"], text=True
        ).strip()
        self.args = types.SimpleNamespace(repo="eng-cc/oasis7")
        self.record = {
            "task_branch": "task/canonical",
            "default_branch": "main",
            "canonical_worktree": str(self.worktree),
        }
        self.original_run_text = MODULE.run_text

    def tearDown(self) -> None:
        MODULE.run_text = self.original_run_text
        self.temp.cleanup()

    def fake_run_text(self, command: list[str]) -> str:
        if command[:3] == ["gh", "pr", "view"]:
            return json.dumps({
                "number": 7,
                "headRefName": "task/repaired",
                "headRefOid": self.head,
                "headRepository": {"nameWithOwner": "eng-cc/oasis7"},
                "baseRefName": "main",
                "state": "OPEN",
            })
        return self.original_run_text(command)

    def test_repaired_branch_is_rejected_before_task_write(self) -> None:
        MODULE.run_text = self.fake_run_text
        with self.assertRaises(MODULE._CommandExit) as caught:
            MODULE.validate_record_pr_identity(self.args, self.record, 7)
        self.assertIn("differs from canonical task branch", str(caught.exception))

    def test_exact_canonical_branch_and_head_are_admitted(self) -> None:
        def exact_run_text(command: list[str]) -> str:
            if command[:3] == ["gh", "pr", "view"]:
                return json.dumps({
                    "number": 7,
                    "headRefName": "task/canonical",
                    "headRefOid": self.head,
                    "headRepository": {"nameWithOwner": "eng-cc/oasis7"},
                    "baseRefName": "main",
                    "state": "OPEN",
                })
            return self.original_run_text(command)

        MODULE.run_text = exact_run_text
        live = MODULE.validate_record_pr_identity(self.args, self.record, 7)
        self.assertEqual(live["headRefName"], "task/canonical")

    def test_foreign_head_repository_is_rejected(self) -> None:
        def foreign_run_text(command: list[str]) -> str:
            if command[:3] == ["gh", "pr", "view"]:
                return json.dumps({
                    "number": 7,
                    "headRefName": "task/canonical",
                    "headRefOid": self.head,
                    "headRepository": {"nameWithOwner": "fork/oasis7"},
                    "baseRefName": "main",
                    "state": "OPEN",
                })
            return self.original_run_text(command)

        MODULE.run_text = foreign_run_text
        with self.assertRaises(MODULE._CommandExit) as caught:
            MODULE.validate_record_pr_identity(self.args, self.record, 7)
        self.assertIn("head repository differs", str(caught.exception))

    def test_worktree_on_wrong_branch_is_rejected_even_when_task_ref_remains(self) -> None:
        subprocess.run(["git", "-C", str(self.worktree), "checkout", "-qb", "wrong/branch"], check=True)
        def canonical_pr_run_text(command: list[str]) -> str:
            if command[:3] == ["gh", "pr", "view"]:
                return json.dumps({
                    "number": 7,
                    "headRefName": "task/canonical",
                    "headRefOid": self.head,
                    "headRepository": {"nameWithOwner": "eng-cc/oasis7"},
                    "baseRefName": "main",
                    "state": "OPEN",
                })
            return self.original_run_text(command)
        MODULE.run_text = canonical_pr_run_text
        with self.assertRaises(MODULE._CommandExit) as caught:
            MODULE.validate_record_pr_identity(self.args, self.record, 7)
        self.assertIn("worktree is registered to a different branch", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
