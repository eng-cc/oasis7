#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest import mock

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
        task = load(
            "github_project_task_record_pr_outcomes",
            ROOT / "scripts/pm/github-project-task.py",
        )
        sync = load(
            "github_project_sync_record_pr_outcomes",
            ROOT / "scripts/pm/github-project-sync.py",
        )
        fixtures = load(
            "github_project_task_lifecycle_fixtures",
            ROOT / "scripts/pm/github-project-task-lifecycle.test.py",
        )
        uid = fixtures.UID
        repo = "eng-cc/oasis7"
        pr_url = f"https://github.com/{repo}/pull/2001"
        head = "a" * 40
        branch = "task/lifecycle-move-contract"

        def invoke_case(
            initial_status: str,
            initial_phase: str,
            *,
            draft_candidate: bool = False,
            existing_ready_update: bool = False,
            wrong_live_task_linkage: bool = False,
        ) -> tuple[dict[str, object], dict[str, object], dict[str, object], dict[str, object]]:
            with tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory).resolve()
                record = fixtures.mapping_record(status=initial_status, phase=initial_phase)
                record.update(fixtures.record_pr_identity(root))
                if existing_ready_update:
                    record.update({"pr_url": pr_url, "pr_number": 2001})
                live_issue = fixtures.record_pr_live_issue(record)
                comments: list[dict[str, str]] = []
                publication_binding_path = None
                live_pr = fixtures.record_pr_live_pr(draft=draft_candidate)
                if existing_ready_update:
                    _publication_module, _intent, publication_binding_path, body, comments = (
                        fixtures.existing_ready_publication_fixture(root)
                    )
                    live_pr["body"] = body
                    if wrong_live_task_linkage:
                        live_pr["body"] = str(live_pr["body"]).replace(
                            f"Task: {uid}", f"Task: task_{'b' * 32}",
                        )
                request = fixtures.record_pr_args(
                    root, existing_ready_update=existing_ready_update,
                    publication_binding_json=publication_binding_path,
                )
                request.draft_candidate = draft_candidate
                mapping_path = root / request.mapping
                mapping_path.parent.mkdir(parents=True)
                task.save_mapping(mapping_path, {"version": 1, "tasks": {uid: record}})
                projected: dict[str, object] = {}
                issue_task: dict[str, object] = {}
                committed: dict[str, object] = {}
                printed: list[str] = []

                with (
                    mock.patch.object(task, "github_issue_record", return_value=live_issue),
                    mock.patch.object(task, "github_issue_comments", return_value=comments),
                    mock.patch.object(
                        task,
                        "authoritative_repository_identity",
                        return_value={
                            "repository": repo,
                            "canonical_worktree": str(root),
                            "task_branch": branch,
                            "default_branch": "main",
                        },
                    ),
                    mock.patch.object(task, "github_pull_request", return_value=live_pr),
                    mock.patch.object(task, "run_text", return_value=head),
                    mock.patch.object(task, "synchronize_live_issue_traceability", return_value=[]),
                    mock.patch.object(
                        task,
                        "update_project_fields",
                        side_effect=lambda _args, value, *_a, **_k: projected.update(value) or 1,
                    ) as update_project,
                    mock.patch.object(
                        task,
                        "update_issue_body",
                        side_effect=lambda _repo, _number, value: issue_task.update(value),
                    ) as update_issue,
                    mock.patch.object(
                        task,
                        "issue_comment",
                        return_value="https://github.com/eng-cc/oasis7/issues/2001#issuecomment-2001",
                    ) as issue_comment,
                    mock.patch.object(
                        task,
                        "verified_issue_comment",
                        return_value="https://github.com/eng-cc/oasis7/issues/2001#issuecomment-2002",
                    ) as verified_issue_comment,
                    mock.patch.object(
                        task,
                        "merge_task_mapping",
                        side_effect=lambda _path, _uid, value, **_k: committed.update(value),
                    ) as merge_mapping,
                    mock.patch(
                        "builtins.print",
                        side_effect=lambda *values, **kwargs: printed.append(values[0]) if not kwargs else None,
                    ),
                ):
                    if wrong_live_task_linkage:
                        with self.assertRaisesRegex(
                            task._CommandExit,
                            "live ready-update PR lacks exact unique Task/Refs identity",
                        ):
                            task.command_record_pr(request)
                        update_project.assert_not_called()
                        update_issue.assert_not_called()
                        issue_comment.assert_not_called()
                        verified_issue_comment.assert_not_called()
                        merge_mapping.assert_not_called()
                        return {}, {}, {}, {}
                    self.assertEqual(0, task.command_record_pr(request))

                self.assertTrue(projected, "Project writer received the command's resulting task projection")
                self.assertTrue(issue_task, "Issue writer received the command's resulting task projection")
                self.assertTrue(committed, "mapping writer received the command's resulting task record")
                self.assertEqual(1, len(printed))
                return committed, projected, issue_task, json.loads(printed[0])

        cases = (
            ("ordinary-ready", "ready", "pre_pr_ready", False, False, "pr_watch", "pr_watch", "PR Watch"),
            ("draft-candidate", "committed", "execution", True, False, "committed", "verification", "In Progress"),
            ("C1-ready", "ready", "pre_pr_ready", False, True, "ready", "pre_pr_ready", "Ready / PR"),
            ("C1-pr-watch", "pr_watch", "pr_watch", False, True, "pr_watch", "pr_watch", "PR Watch"),
        )
        for name, status, phase, draft, c1, expected_status, expected_phase, project_status in cases:
            with self.subTest(route=name):
                committed, projected, issue_task, payload = invoke_case(
                    status, phase, draft_candidate=draft, existing_ready_update=c1,
                )
                self.assertEqual(expected_status, committed["status"])
                self.assertEqual(expected_phase, committed["workflow_phase"])
                self.assertEqual(expected_status, projected["status"])
                self.assertEqual(expected_phase, projected["workflow_phase"])
                self.assertEqual(expected_status, issue_task["status"])
                self.assertEqual(expected_phase, issue_task["workflow_phase"])
                self.assertEqual(expected_status, payload["status"])
                self.assertEqual(expected_phase, payload["workflow_phase"])
                self.assertEqual(project_status, sync.project_field_values(projected)["Status"])
                self.assertEqual(expected_status, sync.project_field_values(projected)["PM Status"])
                self.assertEqual(expected_phase, sync.project_field_values(projected)["Workflow Phase"])
                self.assertEqual(pr_url, sync.project_field_values(projected)["PR"])
                if c1:
                    self.assertIsNotNone(payload["publication_binding_comment_url"])
                else:
                    self.assertIsNone(payload["publication_binding_comment_url"])
        with self.subTest(route="C1-wrong-live-Task-linkage"):
            self.assertEqual(({}, {}, {}, {}), invoke_case(
                "ready", "pre_pr_ready", existing_ready_update=True,
                wrong_live_task_linkage=True,
            ))


if __name__ == "__main__":
    unittest.main(verbosity=2)
