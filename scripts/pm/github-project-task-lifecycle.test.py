#!/usr/bin/env python3
"""RED contract for gate-owned and terminal move-task transitions."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
from argparse import Namespace
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/pm/github-project-task.py"
SPEC = importlib.util.spec_from_file_location("github_project_task_lifecycle", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


UID = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"


def mapping_record(*, status: str, phase: str) -> dict[str, object]:
    return {
        "task_uid": UID,
        "title": "Lifecycle move contract",
        "owner_role": "tpm",
        "module": "engineering",
        "status": status,
        "workflow_phase": phase,
        "priority": "P2",
        "worktree_hint": "/tmp/lifecycle-move-worktree",
        "issue_url": "https://github.com/eng-cc/oasis7/issues/2001",
        "issue_number": 2001,
        "project_item_id": "ITEM_ID",
    }


def args(root: pathlib.Path, target: str) -> Namespace:
    return Namespace(
        root=root,
        mapping=".pm/github-project-sync/tasks.json",
        repo="eng-cc/oasis7",
        project_owner="eng-cc",
        project_number=1,
        task_uid=UID,
        to_status=target,
        json=True,
    )


def record_pr_args(root: pathlib.Path) -> Namespace:
    return Namespace(
        root=root,
        mapping=".pm/github-project-sync/tasks.json",
        repo="eng-cc/oasis7",
        project_owner="eng-cc",
        project_number=1,
        task_uid=UID,
        pr_url="https://github.com/eng-cc/oasis7/pull/2001",
        role="tpm",
        validation_command="record-pr lifecycle contract",
        draft_candidate=False,
        json=True,
    )


def record_pr_identity(root: pathlib.Path) -> dict[str, object]:
    return {
        "repository": "eng-cc/oasis7",
        "canonical_worktree": str(root.resolve()),
        "worktree_hint": str(root.resolve()),
        "task_branch": "task/lifecycle-move-contract",
        "default_branch": "main",
    }


def record_pr_live_issue(record: dict[str, object]) -> dict[str, object]:
    return {
        "task_uid": UID,
        "issue_number": 2001,
        "issue_url": "https://github.com/eng-cc/oasis7/issues/2001",
        "owner_role": record["owner_role"],
        "module": record["module"],
        "priority": record["priority"],
        "status": record["status"],
        "workflow_phase": record["workflow_phase"],
        "worktree_hint": record["worktree_hint"],
    }


def record_pr_live_pr(**overrides: object) -> dict[str, object]:
    pr: dict[str, object] = {
        "number": 2001,
        "html_url": "https://github.com/eng-cc/oasis7/pull/2001",
        "state": "open",
        "merged_at": None,
        "draft": False,
        "head": {
            "ref": "task/lifecycle-move-contract",
            "sha": "a" * 40,
            "repo": {"full_name": "eng-cc/oasis7"},
        },
        "base": {
            "ref": "main",
            "repo": {"full_name": "eng-cc/oasis7"},
        },
    }
    pr.update(overrides)
    return pr


class MoveTaskLifecycleContract(unittest.TestCase):
    def write_mapping(self, root: pathlib.Path, record: dict[str, object]) -> pathlib.Path:
        path = root / ".pm/github-project-sync/tasks.json"
        path.parent.mkdir(parents=True)
        MODULE.save_mapping(path, {"version": 1, "tasks": {UID: record}})
        return path

    @staticmethod
    def digest(path: pathlib.Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def test_gate_owned_statuses_require_canonical_lifecycle_writer(self) -> None:
        for target in ("ready", "pr_watch"):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                mapping_path = self.write_mapping(root, mapping_record(status="committed", phase="execution"))
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                ):
                    with self.assertRaisesRegex(
                        MODULE._CommandExit,
                        rf"move-task: {target} is owned by the canonical",
                    ):
                        MODULE.command_move_task(args(root, target))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                update_project.assert_not_called()

    def test_terminal_idempotent_done_preserves_fine_phase(self) -> None:
        for phase in ("post_merge_done", "closed_without_merge"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="done", phase=phase)
                record.update(
                    {
                        "last_closed_at": "2026-08-30T12:00:00+08:00",
                        "claim_verifications": [
                            {
                                "claim_type": "task_complete",
                                "status": "verified",
                                "allowed_to_claim": True,
                                "verification_exit_code": 0,
                            }
                        ],
                    }
                )
                mapping_path = self.write_mapping(root, record)
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                    mock.patch("builtins.print"),
                ):
                    self.assertEqual(0, MODULE.command_move_task(args(root, "done")))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                update_project.assert_not_called()
                persisted = json.loads(mapping_path.read_text(encoding="utf-8"))["tasks"][UID]
                self.assertEqual(phase, persisted["workflow_phase"])

    def test_terminal_task_cannot_be_reclassified(self) -> None:
        for phase in ("post_merge_done", "closed_without_merge"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="done", phase=phase)
                record.update({"last_closed_at": "2026-08-30T12:00:00+08:00"})
                mapping_path = self.write_mapping(root, record)
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                ):
                    with self.assertRaisesRegex(MODULE._CommandExit, "terminal task cannot be reclassified"):
                        MODULE.command_move_task(args(root, "deferred"))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                update_project.assert_not_called()

    def test_record_pr_requires_ready_pre_pr_ready_authority(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            mapping_path = self.write_mapping(root, mapping_record(status="committed", phase="execution"))
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
            ):
                with self.assertRaisesRegex(
                    MODULE._CommandExit,
                    "record-pr: non-draft pr_watch transition requires task truth at ready/pre_pr_ready",
                ):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            merge_mapping.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_rejects_second_pr_without_mutating_task_truth(self) -> None:
        for existing_field in ("pr_url", "pull_request_url", "pr_number"):
          with self.subTest(existing_field=existing_field), tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record[existing_field] = 1999 if existing_field == "pr_number" else "https://github.com/eng-cc/oasis7/pull/1999"
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment") as comment,
                mock.patch.object(MODULE, "update_project_fields") as update_project,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "different PR .*already bound"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_rejects_foreign_repository_url_with_matching_number(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record["pr_number"] = 2001
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            request = record_pr_args(root)
            request.pr_url = "https://github.com/other/repo/pull/2001"
            with (
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment") as comment,
                mock.patch.object(MODULE, "update_project_fields") as update_project,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "PR URL repository mismatch"):
                    MODULE.command_record_pr(request)
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_rejects_replaced_canonical_branch_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value={
                    **record_pr_identity(root),
                    "task_branch": "task/lifecycle-move-contract-repaired",
                }),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True) as live_pr,
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "canonical task branch identity mismatch"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            live_pr.assert_not_called()
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_rejects_stale_live_task_truth_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            live_issue = record_pr_live_issue(record)
            live_issue["workflow_phase"] = "execution"
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=live_issue),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True) as live_pr,
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "live task Issue workflow_phase differs from cached task truth"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            live_pr.assert_not_called()
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_rejects_stale_live_pr_head_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(
                    head={
                        "ref": "task/lifecycle-move-contract",
                        "sha": "b" * 40,
                        "repo": {"full_name": "eng-cc/oasis7"},
                    },
                ), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "live PR head does not match canonical task HEAD"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_rejects_foreign_live_pr_branch_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(
                    head={
                        "ref": "task/lifecycle-move-contract",
                        "sha": "a" * 40,
                        "repo": {"full_name": "outside/fork"},
                    },
                ), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "live PR head repository does not match task repository"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_cannot_rewrite_terminal_task(self) -> None:
        for phase in ("post_merge_done", "closed_without_merge"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                mapping_path = self.write_mapping(root, mapping_record(status="done", phase=phase))
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                    mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                ):
                    with self.assertRaisesRegex(
                        MODULE._CommandExit,
                        "record-pr: terminal task cannot be reclassified",
                    ):
                        MODULE.command_record_pr(record_pr_args(root))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                comment.assert_not_called()
                merge_mapping.assert_not_called()
                update_project.assert_not_called()

    def test_record_pr_replay_after_pr_watch_is_rejected_without_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            mapping_path = self.write_mapping(root, mapping_record(status="pr_watch", phase="pr_watch"))
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
            ):
                with self.assertRaisesRegex(
                    MODULE._CommandExit,
                    "record-pr: non-draft pr_watch transition requires task truth at ready/pre_pr_ready",
                ):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            merge_mapping.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_preserves_authoritative_ready_writer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body"),
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url"),
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "update_project_fields", return_value=0),
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
            ):
                result = MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(0, result)
            merge_mapping.assert_called_once()


if __name__ == "__main__":
    unittest.main(verbosity=2)
