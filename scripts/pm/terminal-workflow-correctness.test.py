#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


def load(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class TerminalWorkflowCorrectness(unittest.TestCase):
    def test_interrupt_fixture_targets_child_not_ambient_parent(self) -> None:
        fixture = (ROOT / "scripts/pm/github-project-task.test.sh").read_text(encoding="utf-8")
        workflow_eval = (ROOT / "scripts/pm/workflow-behavior-eval.sh").read_text(encoding="utf-8")
        self.assertNotIn('kill -TERM "$PPID"', fixture)
        self.assertIn("GH_INTERRUPT_TARGET", fixture)
        self.assertIn("run_interrupt_isolated", workflow_eval)

    def test_project_uses_coarse_done_for_internal_terminal_phases(self) -> None:
        sync = load("github_project_sync", ROOT / "scripts/pm/github-project-sync.py")
        workflow = load("github_project_workflow", ROOT / "scripts/pm/github-project-workflow.py")
        for phase in ("task_done", "main_sync", "post_merge_done"):
            task = {"status": "done", "workflow_phase": phase}
            written = sync.project_field_values(task)
            audited = workflow.expected_project_values(task)
            for field in ("Status", "PM Status", "Workflow Phase"):
                self.assertEqual(written[field], audited[field], f"{phase}: {field}")
            self.assertEqual("done", written["Workflow Phase"], phase)
        self.assertEqual("In Progress", sync.project_field_values(
            {"status": "done", "workflow_phase": "task_done"})["Status"])
        self.assertEqual("In Progress", sync.project_field_values(
            {"status": "done", "workflow_phase": "main_sync"})["Status"])
        self.assertEqual("Done", sync.project_field_values(
            {"status": "done", "workflow_phase": "post_merge_done"})["Status"])

    def test_generated_pr_link_does_not_auto_close_task(self) -> None:
        text = (ROOT / "scripts/prepare-task-pr.sh").read_text(encoding="utf-8")
        self.assertNotIn("Closes #$TASK_ISSUE_NUMBER", text)
        self.assertIn("Refs #$TASK_ISSUE_NUMBER", text)

    def test_workflow_lint_uses_canonical_pre_pr_ready_evidence(self) -> None:
        for relative in ("scripts/pm/workflow-lint.sh", "scripts/pm/audit-pr-watch-issues.py"):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("Evidence Phase: pre_pr_ready", text, relative)
            self.assertNotIn("Evidence Phase: close", text, relative)

    def test_record_pr_advances_status_and_phase_together(self) -> None:
        text = (ROOT / "scripts/pm/github-project-task.py").read_text(encoding="utf-8")
        start = text.index("def command_record_pr")
        end = text.index("\ndef command_", start + 1)
        command = text[start:end]
        self.assertIn('record["status"] = "pr_watch"', command)
        self.assertIn('record["workflow_phase"] = "pr_watch"', command)


if __name__ == "__main__":
    unittest.main(verbosity=2)
