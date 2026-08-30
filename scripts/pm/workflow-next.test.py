#!/usr/bin/env python3
"""Table-driven contract tests for the read-only workflow-next query."""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "pm" / "workflow-next.py"
UID = "task_11111111111111111111111111111111"


class WorkflowNextTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "repo"
        (self.root / ".pm/github-project-sync").mkdir(parents=True)
        (self.root / ".pm/scratch" / UID).mkdir(parents=True)
        self.mapping = self.root / ".pm/github-project-sync/tasks.json"

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def write_mapping(self, **updates: object) -> None:
        task = {
            "task_uid": UID,
            "title": "Workflow next fixture",
            "repository": "fixture/repo",
            "issue_number": 11,
            "issue_url": "https://github.com/fixture/repo/issues/11",
            "project_item_id": "ITEM1",
            "owner_role": "repository_health_engineer",
            "canonical_worktree": str(self.root),
            "task_branch": "task/fixture",
            "default_branch": "main",
            "status": "candidate",
            "workflow_phase": "",
            "acceptance": ["query is deterministic"],
            "updated_at": "2026-08-30T00:00:00+00:00",
        }
        task.update(updates)
        self.mapping.write_text(json.dumps({"version": 1, "project": {"owner": "fixture", "number": 1}, "tasks": {UID: task}}))

    def run_query(self, *extra: str) -> tuple[int, dict]:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--repo-root", str(self.root), "--task-uid", UID, "--json", *extra],
            text=True,
            capture_output=True,
        )
        if not result.stdout.strip():
            raise AssertionError(f"workflow-next helper unavailable: {result.stderr.strip()}")
        payload = json.loads(result.stdout)
        return result.returncode, payload

    def test_phase_commands_and_terminal_classification(self) -> None:
        cases = [
            ({"status": "candidate", "workflow_phase": ""}, "bootstrap", "bootstrap-task-snapshot.py"),
            ({"status": "committed", "workflow_phase": "execution"}, "execution", "github-project-workflow.sh"),
            ({"status": "committed", "workflow_phase": "verification", "pr_url": "https://example.invalid/pull/7", "pr_number": 7}, "verification", "prepare-task-pr.sh"),
            ({"status": "pr_watch", "workflow_phase": "pr_watch", "pr_url": "https://example.invalid/pull/7", "pr_number": 7}, "pr_watch", "pr-lifecycle-gate.py"),
            ({"status": "done", "workflow_phase": "task_done", "pr_url": "https://example.invalid/pull/7", "pr_number": 7}, "task_done", "finalize-task.sh"),
            ({"status": "done", "workflow_phase": "main_sync", "pr_url": "https://example.invalid/pull/7", "pr_number": 7}, "main_sync", "finalize-task.sh"),
            ({"status": "done", "workflow_phase": "task_done", "completion_mode": "non_pr_task", "non_pr_completion_evidence": "completed"}, "task_done", "non-merge-finalize.py"),
        ]
        for updates, phase, command in cases:
            with self.subTest(updates=updates):
                self.write_mapping(**updates)
                if updates.get("completion_mode") == "non_pr_task":
                    (self.root / ".pm/scratch" / UID / "non-pr-completion-evidence.txt").write_text("completed")
                code, payload = self.run_query()
                self.assertEqual(code, 0, payload)
                self.assertEqual(payload["workflow_phase"], phase, payload)
                self.assertEqual(payload["blockers"], [], payload)
                self.assertTrue(payload["next_command"], payload)
                self.assertIn(command, " ".join(payload["next_command"]), payload)

    def test_stale_identity_and_ambiguous_phase_fail_closed(self) -> None:
        self.write_mapping(status="committed", workflow_phase="execution")
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        snapshot.write_text(json.dumps({
            "schema": "oasis7.bootstrap-task-snapshot/v1",
            "task": {"uid": UID, "project": {"item_id": "ITEM1"}},
            "repository": "fixture/other-repo",
            "git": {"worktree": str(self.root), "branch": "task/fixture"},
            "request": {"identity": "Workflow next fixture"},
        }))
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertTrue(any("stale identity" in item for item in payload["blockers"]), payload)

        self.write_mapping(status="committed", workflow_phase="unknown_phase")
        snapshot.unlink()
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertTrue(any("ambiguous" in item for item in payload["blockers"]), payload)

        self.write_mapping(status="done", workflow_phase="main_sync")
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertTrue(any("main-sync" in item for item in payload["blockers"]), payload)

        self.write_mapping(status="committed", workflow_phase="execution",
                           pr_number=7, pr_url="https://example.invalid/pull/8")
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertTrue(any("PR URL" in item for item in payload["blockers"]), payload)


if __name__ == "__main__":
    unittest.main()
