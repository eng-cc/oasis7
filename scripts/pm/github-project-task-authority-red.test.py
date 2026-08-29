#!/usr/bin/env python3
"""RED contract for complete authoritative task-to-repository identity."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
import subprocess
import tempfile
import types
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/pm/github-project-task.py"
SPEC = importlib.util.spec_from_file_location("github_project_task_authority", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class AuthoritativeMappingContract(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = pathlib.Path(self.tmp.name) / "repo"
        self.worktree = pathlib.Path(self.tmp.name) / "canonical-worktree"
        subprocess.run(["git", "init", "-q", "-b", "main", str(self.repo)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.name", "Test"], check=True)
        (self.repo / "tracked").write_text("base\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "tracked"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-qm", "base"], check=True)
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-qb",
            "task/authority-contract", str(self.worktree),
        ], check=True, stdout=subprocess.DEVNULL)
        self.uid = "task_11111111111111111111111111111111"
        self.expected = {
            "repository": "eng-cc/oasis7",
            "canonical_worktree": str(self.worktree.resolve()),
            "task_branch": "task/authority-contract",
            "default_branch": "main",
        }

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def args(self) -> argparse.Namespace:
        return argparse.Namespace(
            root=self.worktree, mapping=".pm/github-project-sync/tasks.json",
            repo="eng-cc/oasis7", project_owner="eng-cc", project_number=1,
            owner_role="tpm", title="Authority mapping contract", module="engineering",
            priority="P2", source_signal=None, source_type=None, severity=None,
            source_ref=["doc/engineering/workflow/source-of-truth.md"], doc_ref=[],
            related_prd=[], acceptance=[], handoff_to=[],
            worktree_hint=str(self.worktree), json=True,
        )

    def assert_complete_identity(self, record: dict[str, object]) -> None:
        for key, expected in self.expected.items():
            with self.subTest(field=key):
                self.assertEqual(expected, record.get(key), record)

    def test_bootstrap_persists_complete_normalized_repository_identity(self) -> None:
        args = self.args()
        with (
            mock.patch.object(MODULE.uuid, "uuid4", return_value=types.SimpleNamespace(hex=self.uid.removeprefix("task_"))),
            mock.patch.object(MODULE, "create_issue", return_value="https://github.com/eng-cc/oasis7/issues/1"),
            mock.patch.object(MODULE, "add_project_item", return_value="ITEM_ID"),
            mock.patch.object(MODULE, "update_project_fields", return_value=0),
            mock.patch("builtins.print"),
        ):
            self.assertEqual(0, MODULE.command_new_task(args))
        mapping = MODULE.load_mapping(self.worktree / args.mapping)
        self.assert_complete_identity(mapping["tasks"][self.uid])

    def test_refresh_replaces_stale_identity_with_authoritative_git_facts(self) -> None:
        args = self.args()
        args.task_uid = self.uid
        mapping_path = self.worktree / args.mapping
        MODULE.save_mapping(mapping_path, {
            "version": 1,
            "tasks": {self.uid: {
                "task_uid": self.uid, "status": "committed",
                "repository": "wrong/repo", "canonical_worktree": "/tmp/wrong",
                "task_branch": "task/wrong", "default_branch": "trunk",
            }},
        })
        live = {
            "task_uid": self.uid, "title": "Authority mapping contract",
            "issue_number": 1, "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm", "module": "engineering", "status": "committed",
            "priority": "P2", "worktree_hint": str(self.worktree),
        }
        sync = types.SimpleNamespace(recover_project_mapping=lambda *_: {})
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch.object(MODULE, "load_sync_module", return_value=sync),
            mock.patch("builtins.print"),
        ):
            self.assertEqual(0, MODULE.command_refresh_task(args))
        mapping = MODULE.load_mapping(mapping_path)
        self.assert_complete_identity(mapping["tasks"][self.uid])

    def test_selected_refresh_preserves_closed_without_merge_workflow_phase(self) -> None:
        args = self.args()
        args.root = self.repo
        args.task_uid = self.uid
        mapping_path = self.repo / args.mapping
        MODULE.save_mapping(mapping_path, {
            "version": 1,
            "project": {"owner": "eng-cc", "number": 1, "id": "PROJECT_ID"},
            "tasks": {self.uid: {
                "task_uid": self.uid,
                "status": "done",
                "workflow_phase": "closed_without_merge",
                "project_item_id": "ITEM_ID",
                **self.expected,
            }},
        })
        live = {
            "task_uid": self.uid,
            "title": "Authority mapping contract",
            "issue_number": 1,
            "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm",
            "module": "engineering",
            "status": "done",
            "priority": "P2",
            "worktree_hint": str(self.worktree),
        }
        node = {
            "id": "ITEM_ID",
            "project": {
                "id": "PROJECT_ID",
                "number": 1,
                "owner": {"login": "eng-cc"},
            },
            "fieldValues": {
                "pageInfo": {"hasNextPage": False},
                "nodes": [
                    {"name": "Done", "field": {"name": "Status"}},
                    {"name": "done", "field": {"name": "PM Status"}},
                    {"name": "done", "field": {"name": "Workflow Phase"}},
                ],
            },
        }
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch.object(MODULE, "project_refresh_graphql",
                              return_value={"data": {"nodes": [node]}}),
            mock.patch("builtins.print"),
        ):
            self.assertEqual(0, MODULE.command_refresh_task(args))

        refreshed = MODULE.load_mapping(mapping_path)["tasks"][self.uid]
        self.assertEqual("done", refreshed["status"])
        self.assertEqual("closed_without_merge", refreshed["workflow_phase"])

    def test_refresh_resumes_after_intent_before_legacy_receipt_sidecar(self) -> None:
        args = self.args()
        args.root = self.repo
        args.task_uid = self.uid
        mapping_path = self.repo / args.mapping
        digest = "e" * 64
        project_identity = {"owner": "eng-cc", "number": "1", "id": "PROJECT_ID"}
        intent = {
            "schema": "oasis7_non_merge_closeout_intent_v1",
            "task_uid": self.uid,
            "repository": "eng-cc/oasis7",
            "issue_number": 1,
            "project_item_id": "ITEM_ID",
            "project_identity": project_identity,
            "reason": "superseded",
            "evidence_sha256": digest,
            "pr_number": 22,
            "pr_url": "https://github.com/eng-cc/oasis7/pulls/22",
            "headRefOid": "legacy-head",
            "headRefName": "task/authority-contract",
        }
        existing = {
            "task_uid": self.uid,
            "status": "committed",
            "workflow_phase": "execution",
            "repository": "eng-cc/oasis7",
            "issue_number": 1,
            "project_item_id": "ITEM_ID",
            "closed_without_merge_intent": intent,
            **self.expected,
        }
        MODULE.save_mapping(mapping_path, {
            "version": 1, "project": project_identity,
            "tasks": {self.uid: existing},
        })
        receipt_root = self.repo / ".git/oasis7-workflow-receipts" / self.uid
        receipt_root.mkdir(parents=True)
        (receipt_root / "closed-without-merge-receipt.json").write_text(json.dumps({
            "receipt_type": "oasis7_closed_without_merge",
            "schema_version": 1,
            "issuer": "non-merge-finalize",
            "task_uid": self.uid,
            "repository": "eng-cc/oasis7",
            "issue_number": 1,
            "project_item_id": "ITEM_ID",
            "reason": "superseded",
            "evidence_sha256": digest,
            "pr_number": 22,
            "pr_url": "https://github.com/eng-cc/oasis7/pulls/22",
            "pr_state": "CLOSED",
            "mergedAt": None,
        }) + "\n")

        self.assertEqual(
            "execution",
            MODULE.pending_non_merge_phase(
                self.repo, self.uid, existing, "eng-cc/oasis7", project_identity,
            ),
        )

    def test_refresh_accepts_no_pr_migrated_receipt_without_head_fields(self) -> None:
        project_identity = {"owner": "eng-cc", "number": "1", "id": "PROJECT_ID"}
        digest = "f" * 64
        intent = {
            "schema": "oasis7_non_merge_closeout_intent_v1",
            "task_uid": self.uid, "repository": "eng-cc/oasis7",
            "issue_number": 1, "project_item_id": "ITEM_ID",
            "project_identity": project_identity, "reason": "non_pr_completed",
            "evidence_sha256": digest, "pr_number": None, "pr_url": None,
        }
        existing = {
            "task_uid": self.uid, "status": "committed", "workflow_phase": "execution",
            "repository": "eng-cc/oasis7", "issue_number": 1,
            "project_item_id": "ITEM_ID", "closed_without_merge_intent": intent,
            **self.expected,
        }
        mapping_path = self.repo / ".pm/github-project-sync/tasks.json"
        MODULE.save_mapping(mapping_path, {
            "version": 1, "project": project_identity,
            "tasks": {self.uid: existing},
        })
        receipt_root = self.repo / ".git/oasis7-workflow-receipts" / self.uid
        receipt_root.mkdir(parents=True)
        canonical_path = receipt_root / "closed-without-merge-receipt.json"
        canonical_path.write_text(json.dumps({
            "receipt_type": "oasis7_closed_without_merge", "schema_version": 1,
            "issuer": "non-merge-finalize", "task_uid": self.uid,
            "repository": "eng-cc/oasis7", "issue_number": 1,
            "project_item_id": "ITEM_ID", "reason": "non_pr_completed",
            "evidence_sha256": digest, "pr_number": None, "pr_url": None,
            "pr_state": None, "mergedAt": None,
        }, sort_keys=True) + "\n")
        source_digest = hashlib.sha256(canonical_path.read_bytes()).hexdigest()
        (receipt_root / "closed-without-merge-receipt-migrated.json").write_text(json.dumps({
            "receipt_type": "oasis7_closed_without_merge", "schema_version": 1,
            "issuer": "non-merge-finalize", "task_uid": self.uid,
            "repository": "eng-cc/oasis7", "issue_number": 1,
            "project_item_id": "ITEM_ID", "project_identity": project_identity,
            "reason": "non_pr_completed", "evidence_sha256": digest,
            "evidence": {"text": "completed"},
            "pr_number": None, "pr_url": None, "pr_state": None, "mergedAt": None,
            "previous_status": "committed", "previous_workflow_phase": "execution",
            "migrated_from": "closed-without-merge-receipt.json",
            "migrated_from_sha256": source_digest,
        }, sort_keys=True) + "\n")

        self.assertEqual(
            "execution",
            MODULE.pending_non_merge_phase(
                self.repo, self.uid, existing, "eng-cc/oasis7", project_identity,
            ),
        )

    def test_refresh_rejects_missing_project_binding_without_mutation_and_allows_repaired_retry(self) -> None:
        args = self.args()
        args.root = self.repo
        args.task_uid = self.uid
        mapping_path = self.repo / args.mapping
        original = {
            "version": 1,
            "project": {"owner": "eng-cc", "number": 1, "id": "PROJECT_ID"},
            "tasks": {self.uid: {
                "task_uid": self.uid,
                "status": "done",
                "workflow_phase": "closed_without_merge",
                "project_item_id": "STALE_ITEM",
                **self.expected,
            }},
        }
        MODULE.save_mapping(mapping_path, original)
        live = {
            "task_uid": self.uid,
            "title": "Authority mapping contract",
            "issue_number": 1,
            "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm",
            "module": "engineering",
            "status": "done",
            "priority": "P2",
            "worktree_hint": str(self.worktree),
        }
        node = {
            "id": "ITEM_ID",
            "project": {
                "id": "PROJECT_ID",
                "number": 1,
                "owner": {"login": "eng-cc"},
            },
            "fieldValues": {
                "pageInfo": {"hasNextPage": False},
                "nodes": [],
            },
        }
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch.object(MODULE, "project_refresh_graphql",
                              return_value={"data": {"nodes": []}}),
            mock.patch("builtins.print"),
        ):
            with self.assertRaisesRegex(SystemExit, r"Project item|Project identity|binding"):
                MODULE.command_refresh_task(args)
        self.assertEqual(original, MODULE.load_mapping(mapping_path))

        repaired = json.loads(json.dumps(original))
        repaired["tasks"][self.uid]["project_item_id"] = "ITEM_ID"
        MODULE.save_mapping(mapping_path, repaired)
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch.object(MODULE, "project_refresh_graphql",
                              return_value={"data": {"nodes": [node]}}),
            mock.patch("builtins.print"),
        ):
            self.assertEqual(0, MODULE.command_refresh_task(args))
        refreshed = MODULE.load_mapping(mapping_path)["tasks"][self.uid]
        self.assertEqual("ITEM_ID", refreshed["project_item_id"])

    def test_default_root_refresh_preserves_registered_task_worktree_identity(self) -> None:
        args = self.args()
        args.root = self.repo
        args.task_uid = self.uid
        mapping_path = self.repo / args.mapping
        MODULE.save_mapping(mapping_path, {
            "version": 1,
            "tasks": {self.uid: {
                "task_uid": self.uid,
                "status": "done",
                **self.expected,
            }},
        })
        live = {
            "task_uid": self.uid,
            "title": "Authority mapping contract",
            "issue_number": 1,
            "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm",
            "module": "engineering",
            "status": "done",
            "priority": "P2",
            "worktree_hint": str(self.worktree),
        }
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch.object(MODULE, "project_refresh_graphql", return_value={"data": {"nodes": []}}),
            mock.patch("builtins.print"),
        ):
            self.assertEqual(0, MODULE.command_refresh_task(args))

        refreshed = MODULE.load_mapping(mapping_path)["tasks"][self.uid]
        self.assert_complete_identity(refreshed)
        self.assertNotEqual(str(self.repo.resolve()), refreshed["canonical_worktree"])
        self.assertNotEqual("main", refreshed["task_branch"])

    def test_refresh_rejects_conflicting_registered_task_identities_without_mutation(self) -> None:
        other = pathlib.Path(self.tmp.name) / "other-task-worktree"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-qb",
            "task/other-authority-contract", str(other),
        ], check=True, stdout=subprocess.DEVNULL)
        args = self.args()
        args.root = self.repo
        args.task_uid = self.uid
        mapping_path = self.repo / args.mapping
        original = {"version": 1, "tasks": {self.uid: {
            "task_uid": self.uid,
            "status": "committed",
            **self.expected,
        }}}
        MODULE.save_mapping(mapping_path, original)
        live = {
            "task_uid": self.uid,
            "title": "Authority mapping contract",
            "issue_number": 1,
            "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm",
            "module": "engineering",
            "status": "committed",
            "priority": "P2",
            "worktree_hint": str(other),
        }
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch("builtins.print"),
        ):
            with self.assertRaises(SystemExit):
                MODULE.command_refresh_task(args)
        self.assertEqual(original, MODULE.load_mapping(mapping_path))

    def test_default_root_refresh_rejects_missing_task_identity_without_mutation(self) -> None:
        args = self.args()
        args.root = self.repo
        args.task_uid = self.uid
        mapping_path = self.repo / args.mapping
        missing = pathlib.Path(self.tmp.name) / "missing-task-worktree"
        original = {"version": 1, "tasks": {self.uid: {
            "task_uid": self.uid,
            "status": "committed",
            "repository": "eng-cc/oasis7",
            "canonical_worktree": str(missing),
            "task_branch": "task/missing",
            "default_branch": "main",
        }}}
        MODULE.save_mapping(mapping_path, original)
        live = {
            "task_uid": self.uid,
            "title": "Authority mapping contract",
            "issue_number": 1,
            "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm",
            "module": "engineering",
            "status": "committed",
            "priority": "P2",
            "worktree_hint": str(missing),
        }
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch("builtins.print"),
        ):
            with self.assertRaises(SystemExit):
                MODULE.command_refresh_task(args)
        self.assertEqual(original, MODULE.load_mapping(mapping_path))

    def test_refresh_rejects_worktree_hint_from_different_git_common_dir(self) -> None:
        args = self.args()
        args.task_uid = self.uid
        mapping_path = self.worktree / args.mapping
        original = {"version": 1, "tasks": {self.uid: {
            "task_uid": self.uid, "status": "committed", **self.expected,
        }}}
        MODULE.save_mapping(mapping_path, original)

        foreign_repo = pathlib.Path(self.tmp.name) / "foreign-repo"
        foreign_worktree = pathlib.Path(self.tmp.name) / "foreign-worktree"
        subprocess.run(["git", "init", "-q", "-b", "main", str(foreign_repo)], check=True)
        subprocess.run(["git", "-C", str(foreign_repo), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(foreign_repo), "config", "user.name", "Test"], check=True)
        (foreign_repo / "tracked").write_text("foreign\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(foreign_repo), "add", "tracked"], check=True)
        subprocess.run(["git", "-C", str(foreign_repo), "commit", "-qm", "foreign"], check=True)
        subprocess.run([
            "git", "-C", str(foreign_repo), "worktree", "add", "-qb",
            "task/foreign", str(foreign_worktree),
        ], check=True, stdout=subprocess.DEVNULL)

        live = {
            "task_uid": self.uid, "title": "Authority mapping contract",
            "issue_number": 1, "issue_url": "https://github.com/eng-cc/oasis7/issues/1",
            "owner_role": "tpm", "module": "engineering", "status": "committed",
            "priority": "P2", "worktree_hint": str(foreign_worktree),
        }
        sync = types.SimpleNamespace(recover_project_mapping=lambda *_: {})
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=live),
            mock.patch.object(MODULE, "load_sync_module", return_value=sync),
            mock.patch("builtins.print"),
        ):
            with self.assertRaises(SystemExit):
                MODULE.command_refresh_task(args)
        self.assertEqual(original, MODULE.load_mapping(mapping_path))


class WindowsSubprocessEncodingContract(unittest.TestCase):
    def test_run_text_uses_utf8_for_windows_subprocess_output(self) -> None:
        completed = subprocess.CompletedProcess(
            ["gh", "issue", "view"], 0, stdout="\u4efb\u52a1\u8bc1\u636e\n", stderr=""
        )
        with mock.patch.object(MODULE.subprocess, "run", return_value=completed) as run:
            self.assertEqual("\u4efb\u52a1\u8bc1\u636e", MODULE.run_text(["gh", "issue", "view"]))
        self.assertEqual("utf-8", run.call_args.kwargs["encoding"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
