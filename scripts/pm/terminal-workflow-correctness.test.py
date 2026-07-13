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
        for phase in ("task_done", "main_sync", "post_merge_done"):
            values = sync.project_field_values({"status": "done", "workflow_phase": phase})
            self.assertEqual("done", values["Workflow Phase"], phase)

        workflow = load("github_project_workflow", ROOT / "scripts/pm/github-project-workflow.py")
        values = workflow.expected_project_values({"status": "done", "workflow_phase": "task_done"})
        self.assertEqual("done", values["Workflow Phase"])

    def test_generated_pr_link_does_not_auto_close_task(self) -> None:
        text = (ROOT / "scripts/prepare-task-pr.sh").read_text(encoding="utf-8")
        self.assertNotIn("Closes #$TASK_ISSUE_NUMBER", text)
        self.assertIn("Refs #$TASK_ISSUE_NUMBER", text)


if __name__ == "__main__":
    unittest.main(verbosity=2)
