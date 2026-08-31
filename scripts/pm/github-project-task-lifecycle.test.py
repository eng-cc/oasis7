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


if __name__ == "__main__":
    unittest.main(verbosity=2)
