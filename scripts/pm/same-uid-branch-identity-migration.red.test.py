#!/usr/bin/env python3
"""RED contract for same-UID branch identity migration bootstrap epochs."""

from __future__ import annotations

import base64
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts" / "pm" / "migrate-task-branch-identity.py"
BOOTSTRAP_HELPER = ROOT / "scripts" / "pm" / "bootstrap-task-snapshot.py"
UID = "task_11111111111111111111111111111111"


def git_executable() -> str:
    executable = shutil.which("git")
    if executable is None and sys.platform == "win32":
        candidate = pathlib.Path("C:/Program Files/Git/cmd/git.exe")
        if candidate.is_file():
            executable = str(candidate)
    if executable is None:
        raise RuntimeError("same-UID migration test setup cannot find git")
    return executable


class SameUidBranchIdentityMigrationRedTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name) / "repo"
        self.root.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        (self.root / "README.md").write_text("base\n", encoding="utf-8")
        self.git("add", "README.md")
        self.git("commit", "-m", "base")
        self.git("switch", "-c", "task/engineering-old")
        (self.root / "IMPLEMENTATION").write_text("preserve this exact head\n", encoding="utf-8")
        self.git("add", "IMPLEMENTATION")
        self.git("commit", "-m", "implementation")
        self.old_head = self.git("rev-parse", "HEAD")
        self.mapping = self.root / ".pm" / "github-project-sync" / "tasks.json"
        self.snapshot = self.root / ".pm" / "scratch" / UID / "bootstrap-task-snapshot.json"
        self.snapshot.parent.mkdir(parents=True)
        self.snapshot_bytes = b'{"digest":"sha256:old-snapshot","epoch":1}\n'
        self.snapshot.write_bytes(self.snapshot_bytes)
        self.replacement = self.root.parent / "replacement-worktree"
        self.replacement_branch = "task/engineering-replacement"
        self.authoritative_remote = self.root.parent / "authoritative.git"
        subprocess.run(
            [git_executable(), "init", "--bare", str(self.authoritative_remote)],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.git("remote", "add", "origin", str(self.authoritative_remote))
        self.remote_repository_map = {
            str(self.authoritative_remote.resolve()): "eng-cc/oasis7",
        }
        self.moved_comparison_oid = self.git(
            "commit-tree", "HEAD^{tree}", "-p", "refs/heads/main", "-m", "comparison moved"
        )
        self.write_mapping()
        self.old_mapping_bytes = self.mapping.read_bytes()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str, cwd: pathlib.Path | None = None) -> str:
        return subprocess.run(
            [git_executable(), "-C", str(cwd or self.root), *args],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        ).stdout.strip()

    def write_mapping(self, **changes: object) -> None:
        task = {
            "task_uid": UID,
            "title": "same UID migration bootstrap fixture",
            "issue_number": 2660,
            "issue_url": "https://example.invalid/issues/2660",
            "project_item_id": "PVTI_test",
            "status": "committed",
            "workflow_phase": "execution",
            "owner_role": "repository_health_engineer",
            "repository": "eng-cc/oasis7",
            "canonical_worktree": str(self.root.resolve()),
            "task_branch": "task/engineering-old",
            "default_branch": "main",
            "bootstrap_epoch": 1,
            "acceptance": ["same UID migration preserves the implementation head"],
            "phase_receipts": {"ci": "old-ci-receipt"},
            "evidence": {"review": "old-review-ledger"},
            "pr_number": 2661,
            "pr_url": "https://example.invalid/pull/2661",
            "claim_verifications": [{"claim_type": "task_complete"}],
        }
        task.update(changes)
        self.mapping.parent.mkdir(parents=True, exist_ok=True)
        self.mapping.write_text(
            json.dumps({"version": 1, "project": {"owner": "eng-cc", "number": 1}, "tasks": {UID: task}}),
            encoding="utf-8",
        )

    def mapping_record(self) -> dict[str, object]:
        return json.loads(self.mapping.read_text(encoding="utf-8"))["tasks"][UID]

    def replace_mapping_record(self, record: dict[str, object]) -> None:
        mapping = json.loads(self.mapping.read_text(encoding="utf-8"))
        mapping["tasks"][UID] = record
        self.mapping.write_text(json.dumps(mapping), encoding="utf-8")

    def remove_bootstrap_epoch(self) -> None:
        record = self.mapping_record()
        record.pop("bootstrap_epoch")
        self.replace_mapping_record(record)

    def run_helper(
        self,
        *,
        crash_after: str | None = None,
        comparison_oid_after_branch_created: str | None = None,
        repo_root: pathlib.Path | None = None,
        tasks_json: pathlib.Path | None = None,
        replacement_worktree: pathlib.Path | None = None,
        replacement_branch: str | None = None,
    ) -> subprocess.CompletedProcess[str]:
        environment = dict(os.environ)
        environment["OASIS7_PM_TEST_REMOTE_REPOSITORY_MAP"] = json.dumps(self.remote_repository_map)
        if crash_after:
            environment["OASIS7_PM_TEST_MIGRATION_CRASH_AFTER"] = crash_after
        if comparison_oid_after_branch_created:
            environment["OASIS7_PM_TEST_MIGRATION_COMPARISON_OID_AFTER_BRANCH_CREATED"] = (
                comparison_oid_after_branch_created
            )
        return subprocess.run(
            [
                sys.executable,
                str(HELPER),
                "--repo-root", str(repo_root or self.root),
                "--task-uid", UID,
                "--tasks-json", str(tasks_json or self.mapping),
                "--replacement-worktree", str(replacement_worktree or self.replacement),
                "--replacement-branch", replacement_branch or self.replacement_branch,
                "--comparison-ref", "refs/heads/main",
                "--issuer", "tpm",
                "--reason", "same UID canonical worktree collision",
                "--json",
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
        )

    def assert_precommit_failure_preserves_old_authority(
        self, result: subprocess.CompletedProcess[str], before_mapping: bytes, before_snapshot: bytes
    ) -> None:
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.mapping.read_bytes(), before_mapping)
        self.assertEqual(self.snapshot.read_bytes(), before_snapshot)

    def add_remote_collision(self, *, fetch: bool) -> None:
        self.git("push", "origin", f"{self.old_head}:refs/heads/{self.replacement_branch}")
        if fetch:
            self.git("fetch", "origin", self.replacement_branch)
        else:
            # A successful push updates the tracking ref. Remove only that local
            # observation so the bare remote remains authoritative but unfetched.
            self.git("update-ref", "-d", f"refs/remotes/origin/{self.replacement_branch}")

    def assert_migrated(self, result: subprocess.CompletedProcess[str]) -> dict[str, object]:
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["task_uid"], UID)
        self.assertEqual(receipt["old_epoch"], 1)
        self.assertEqual(receipt["new_epoch"], 2)
        self.assertEqual(receipt["implementation_head"], self.old_head)
        self.assertEqual(receipt["comparison_ref"], "refs/heads/main")
        self.assertEqual(receipt["comparison_oid"], self.git("rev-parse", "refs/heads/main"))
        return receipt

    def run_replacement_bootstrap(self) -> subprocess.CompletedProcess[str]:
        return self.run_bootstrap(self.replacement)

    def run_bootstrap(self, root: pathlib.Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(BOOTSTRAP_HELPER),
                "validate-or-create",
                "--repo-root",
                str(root),
                "--task-uid",
                UID,
                "--producer",
                "tpm",
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_migration_archives_old_epoch_preserves_exact_head_and_invalidates_authority(self) -> None:
        receipt = self.assert_migrated(self.run_helper())
        record = self.mapping_record()
        self.assertEqual(record["canonical_worktree"], str(self.replacement.resolve()))
        self.assertEqual(record["task_branch"], self.replacement_branch)
        self.assertEqual(record["bootstrap_epoch"], 2)
        self.assertEqual(record["workflow_phase"], "bootstrap")
        self.assertEqual(record["workflow_state"], "action_required")
        self.assertEqual(self.git("rev-parse", "HEAD", cwd=self.replacement), self.old_head)
        self.assertEqual(self.git("rev-parse", self.replacement_branch), self.old_head)

        history = record["historical_epochs"]["1"]
        self.assertEqual(history["task_record"]["canonical_worktree"], str(self.root.resolve()))
        self.assertEqual(history["task_record"]["task_branch"], "task/engineering-old")
        archived_snapshot = pathlib.Path(history["snapshot_path"])
        self.assertEqual(archived_snapshot.read_bytes(), self.snapshot_bytes)
        self.assertEqual(history["snapshot_sha256"], hashlib.sha256(self.snapshot_bytes).hexdigest())
        self.assertEqual(base64.b64decode(history["snapshot_bytes_b64"]), self.snapshot_bytes)
        self.assertEqual(receipt["historical_artifact_digests"]["snapshot_sha256"], history["snapshot_sha256"])
        self.assertEqual(receipt["old_common_dir"], str(self.root.resolve() / ".git"))
        self.assertEqual(receipt["new_common_dir"], str(self.root.resolve() / ".git"))
        self.assertEqual(receipt["old_worktree"], history["task_record"]["canonical_worktree"])
        self.assertEqual(receipt["old_branch"], history["task_record"]["task_branch"])
        self.assertEqual(receipt["new_worktree"], record["canonical_worktree"])
        self.assertEqual(receipt["new_branch"], record["task_branch"])
        self.assertEqual(receipt["implementation_head"], record["branch_identity_migration"]["implementation_head"])
        self.assertEqual(receipt["comparison_ref"], record["branch_identity_migration"]["comparison_ref"])
        self.assertEqual(receipt["comparison_oid"], record["branch_identity_migration"]["comparison_oid"])
        self.assertEqual(receipt, record["branch_identity_migration_receipt"])

        self.assertEqual(record["phase_receipts"], {})
        self.assertEqual(record["evidence"], {})
        self.assertNotIn("pr_number", record)
        self.assertNotIn("claim_verifications", record)
        invalidated = record["invalidated_authority"]
        self.assertEqual(invalidated["migration_receipt_sha256"], receipt["digest"])
        self.assertEqual(receipt["invalidated_authority"]["reason"], invalidated["reason"])
        self.assertEqual(receipt["invalidated_authority"]["fields"], invalidated["fields"])
        self.assertIn("phase_receipts", invalidated["fields"])
        self.assertIn("bootstrap_snapshot", invalidated["fields"])

    def test_migration_materializes_replacement_mapping_for_normal_bootstrap(self) -> None:
        self.assert_migrated(self.run_helper())

        replacement_mapping = self.replacement / ".pm" / "github-project-sync" / "tasks.json"
        replacement_record = json.loads(replacement_mapping.read_text(encoding="utf-8"))["tasks"][UID]
        self.assertEqual(replacement_record["canonical_worktree"], str(self.replacement.resolve()))
        self.assertEqual(replacement_record["task_branch"], self.replacement_branch)
        self.assertEqual(replacement_record["bootstrap_epoch"], 2)

        bootstrap = self.run_replacement_bootstrap()
        self.assertEqual(bootstrap.returncode, 0, bootstrap.stderr)
        self.assertEqual(json.loads(bootstrap.stdout)["status"], "created")

    def test_sequential_migration_advances_epoch_preserves_history_and_is_idempotent(self) -> None:
        first = self.assert_migrated(self.run_helper())
        replacement_two = self.root.parent / "replacement-worktree-two"
        branch_two = "task/engineering-replacement-two"
        epoch_two_mapping = self.replacement / ".pm" / "github-project-sync" / "tasks.json"
        (self.replacement / "POST_MIGRATION_CHANGE").write_text("ordinary later implementation\n", encoding="utf-8")
        self.git("add", "POST_MIGRATION_CHANGE", cwd=self.replacement)
        self.git("commit", "-m", "ordinary later implementation", cwd=self.replacement)
        advanced_head = self.git("rev-parse", "HEAD", cwd=self.replacement)

        second = self.run_helper(
            repo_root=self.replacement,
            tasks_json=epoch_two_mapping,
            replacement_worktree=replacement_two,
            replacement_branch=branch_two,
        )
        self.assertEqual(second.returncode, 0, second.stderr)
        second_receipt = json.loads(second.stdout)
        self.assertEqual(second_receipt["old_epoch"], 2)
        self.assertEqual(second_receipt["new_epoch"], 3)
        self.assertEqual(second_receipt["implementation_head"], advanced_head)
        self.assertGreater(second_receipt["journal_revision"], first["journal_revision"])

        final_mapping = replacement_two / ".pm" / "github-project-sync" / "tasks.json"
        final_record = json.loads(final_mapping.read_text(encoding="utf-8"))["tasks"][UID]
        self.assertEqual(final_record["bootstrap_epoch"], 3)
        self.assertEqual(final_record["canonical_worktree"], str(replacement_two.resolve()))
        self.assertEqual(final_record["task_branch"], branch_two)
        self.assertEqual(self.git("rev-parse", "HEAD", cwd=replacement_two), advanced_head)
        self.assertEqual(set(final_record["historical_epochs"]), {"1", "2"})
        self.assertEqual(
            final_record["historical_epochs"]["2"]["task_record"]["branch_identity_migration_receipt"], first
        )
        bootstrap = self.run_bootstrap(replacement_two)
        self.assertEqual(bootstrap.returncode, 0, bootstrap.stderr)
        self.assertEqual(json.loads(bootstrap.stdout)["status"], "created")

        repeated = self.run_helper(
            repo_root=self.replacement,
            tasks_json=epoch_two_mapping,
            replacement_worktree=replacement_two,
            replacement_branch=branch_two,
        )
        self.assertEqual(repeated.returncode, 0, repeated.stderr)
        self.assertEqual(json.loads(repeated.stdout), second_receipt)
        self.assertEqual(
            self.git("worktree", "list", "--porcelain").count(f"branch refs/heads/{branch_two}"), 1
        )

    def test_sequential_migration_rejects_ambiguous_prior_branch_request(self) -> None:
        self.assert_migrated(self.run_helper())
        replacement_two = self.root.parent / "replacement-worktree-two"
        epoch_two_mapping = self.replacement / ".pm" / "github-project-sync" / "tasks.json"
        before_mapping = epoch_two_mapping.read_bytes()

        ambiguous = self.run_helper(
            repo_root=self.replacement,
            tasks_json=epoch_two_mapping,
            replacement_worktree=replacement_two,
            replacement_branch=self.replacement_branch,
        )

        self.assertNotEqual(ambiguous.returncode, 0, ambiguous.stdout)
        self.assertIn("requested active identity", ambiguous.stderr)
        self.assertEqual(epoch_two_mapping.read_bytes(), before_mapping)
        self.assertFalse(replacement_two.exists())

    def test_sequential_migration_rejects_non_ancestor_rewritten_history(self) -> None:
        self.assert_migrated(self.run_helper())
        replacement_two = self.root.parent / "replacement-worktree-two"
        branch_two = "task/engineering-replacement-two"
        epoch_two_mapping = self.replacement / ".pm" / "github-project-sync" / "tasks.json"
        before_mapping = epoch_two_mapping.read_bytes()
        self.git("reset", "--hard", "main", cwd=self.replacement)

        rewritten = self.run_helper(
            repo_root=self.replacement,
            tasks_json=epoch_two_mapping,
            replacement_worktree=replacement_two,
            replacement_branch=branch_two,
        )

        self.assertNotEqual(rewritten.returncode, 0, rewritten.stdout)
        self.assertIn("requested active identity", rewritten.stderr)
        self.assertEqual(epoch_two_mapping.read_bytes(), before_mapping)
        self.assertFalse(replacement_two.exists())

    def test_legacy_record_without_bootstrap_epoch_migrates_as_epoch_one(self) -> None:
        self.remove_bootstrap_epoch()

        receipt = self.assert_migrated(self.run_helper())

        history = self.mapping_record()["historical_epochs"]["1"]
        self.assertEqual(history["task_record"]["bootstrap_epoch"], 1)
        self.assertEqual(receipt["old_epoch"], 1)
        self.assertEqual(receipt["new_epoch"], 2)

    def test_legacy_record_without_bootstrap_epoch_recovers_after_mapping_commit(self) -> None:
        self.remove_bootstrap_epoch()

        interrupted = self.run_helper(crash_after="mapping_committed")

        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)
        self.assertIn("injected crash after mapping committed", interrupted.stderr)
        receipt = self.assert_migrated(self.run_helper())
        history = self.mapping_record()["historical_epochs"]["1"]
        self.assertEqual(history["task_record"]["bootstrap_epoch"], 1)
        self.assertEqual(receipt["old_epoch"], 1)
        self.assertEqual(receipt["new_epoch"], 2)

    def test_existing_conflicting_branch_fails_before_commit_and_keeps_old_epoch_active(self) -> None:
        self.git("branch", self.replacement_branch, "main")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("branch uniqueness", result.stderr)
        self.assertEqual(self.mapping.read_bytes(), before_mapping)
        self.assertEqual(self.snapshot.read_bytes(), before_snapshot)
        self.assertFalse(self.replacement.exists())
        journal = self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json"
        self.assertTrue(journal.exists(), "failure must preserve a resumable migration journal")
        saved_journal = json.loads(journal.read_text(encoding="utf-8"))
        self.assertEqual(saved_journal["state"], "capability_blocked")
        self.assertEqual(saved_journal["resume_command"].split()[0], sys.executable)

    def test_remote_tracking_replacement_branch_collision_preserves_old_epoch(self) -> None:
        self.add_remote_collision(fetch=True)
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("branch uniqueness", result.stderr)
        self.assertFalse(self.replacement.exists())
        journal = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertTrue(journal["collision"]["remote_branch"])

    def test_unfetched_remote_replacement_branch_collision_fails_closed(self) -> None:
        self.add_remote_collision(fetch=False)
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("remote branch uniqueness", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_no_configured_remote_is_capability_blocked_without_mutation(self) -> None:
        self.git("remote", "remove", "origin")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("authoritative remote", result.stderr)
        self.assertIn("capability_blocked", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_only_foreign_remote_repository_is_rejected_without_mutation(self) -> None:
        foreign = self.root.parent / "foreign.git"
        subprocess.run(
            [git_executable(), "init", "--bare", str(foreign)],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.git("remote", "set-url", "origin", str(foreign))
        self.remote_repository_map[str(foreign.resolve())] = "other-owner/other-repository"
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("repository identity mismatch", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_authoritative_test_remote_normalizes_to_task_repository_identity(self) -> None:
        result = self.run_helper()

        receipt = self.assert_migrated(result)
        journal = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertEqual(journal.get("authoritative_repository"), "eng-cc/oasis7")
        self.assertEqual(journal.get("authoritative_remote_names"), ["origin"])
        self.assertEqual(receipt.get("repository"), "eng-cc/oasis7")

    def test_source_branch_identity_mismatch_preserves_old_authority(self) -> None:
        self.write_mapping(task_branch="task/foreign-owner")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("source branch identity", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_journal_proven_replacement_in_foreign_common_dir_is_rejected(self) -> None:
        self.git("branch", self.replacement_branch, self.old_head)
        subprocess.run(
            [git_executable(), "clone", "--quiet", str(self.root), str(self.replacement)],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.git("switch", self.replacement_branch, cwd=self.replacement)
        journal_path = self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json"
        journal_path.write_text(json.dumps({
            "schema": "oasis7_branch_identity_migration/v1",
            "task_uid": UID,
            "revision": 1,
            "state": "replacement_branch_created",
            "replacement_worktree": str(self.replacement.resolve()),
            "replacement_branch": self.replacement_branch,
            "implementation_head": self.old_head,
        }), encoding="utf-8")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("branch uniqueness", result.stderr)
        replacement_common = (self.replacement / self.git("rev-parse", "--git-common-dir", cwd=self.replacement)).resolve()
        source_common = (self.root / self.git("rev-parse", "--git-common-dir")).resolve()
        self.assertNotEqual(replacement_common, source_common)

    def test_non_positive_bootstrap_epoch_is_rejected_without_mutation(self) -> None:
        self.write_mapping(bootstrap_epoch=0)
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("bootstrap epoch must be a positive integer", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_malformed_bootstrap_epoch_is_rejected_without_traceback(self) -> None:
        self.write_mapping(bootstrap_epoch="not-an-integer")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("bootstrap epoch must be a positive integer", result.stderr)
        self.assertNotIn("Traceback", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_malformed_journal_is_rejected_without_traceback_or_authority_mutation(self) -> None:
        journal_path = self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json"
        journal_path.write_text("{not-json\n", encoding="utf-8")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("migration journal", result.stderr)
        self.assertNotIn("Traceback", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_non_positive_journal_revision_is_rejected_without_authority_mutation(self) -> None:
        journal_path = self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json"
        journal_path.write_text(json.dumps({
            "schema": "oasis7_branch_identity_migration/v1",
            "task_uid": UID,
            "revision": 0,
            "state": "intent",
        }), encoding="utf-8")
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper()

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("journal revision must be a positive integer", result.stderr)
        self.assertFalse(self.replacement.exists())

    def test_comparison_ref_movement_after_branch_creation_fails_before_mapping_commit(self) -> None:
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        result = self.run_helper(comparison_oid_after_branch_created=self.moved_comparison_oid)

        self.assert_precommit_failure_preserves_old_authority(result, before_mapping, before_snapshot)
        self.assertIn("comparison", result.stderr)
        self.assertEqual(self.git("rev-parse", "refs/heads/main"), self.moved_comparison_oid)
        journal = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertEqual(journal["state"], "replacement_branch_created")
        self.assertEqual(journal["comparison_oid"], self.git("rev-parse", f"{self.moved_comparison_oid}^"))
        self.assertEqual(self.git("rev-parse", self.replacement_branch), self.old_head)

    def test_crash_after_archive_before_mapping_commit_retries_one_epoch(self) -> None:
        before_mapping = self.mapping.read_bytes()
        before_snapshot = self.snapshot.read_bytes()

        interrupted = self.run_helper(crash_after="historical_snapshot_archived")

        self.assert_precommit_failure_preserves_old_authority(interrupted, before_mapping, before_snapshot)
        self.assertIn("injected crash after historical snapshot archived", interrupted.stderr)
        archive = (
            self.root / ".pm" / "scratch" / UID / "historical-epochs" / "1" /
            "bootstrap-task-snapshot.json"
        )
        self.assertEqual(archive.read_bytes(), self.snapshot_bytes)

        receipt = self.assert_migrated(self.run_helper())
        self.assertEqual(self.mapping_record()["bootstrap_epoch"], 2)
        self.assertEqual(receipt["old_epoch"], 1)
        self.assertEqual(receipt["new_epoch"], 2)
        self.assertEqual(
            self.git("worktree", "list", "--porcelain").count(f"branch refs/heads/{self.replacement_branch}"),
            1,
        )

    def test_crash_after_mapping_commit_reuses_receipt_and_finishes_snapshot_cleanup(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")

        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)
        self.assertIn("injected crash after mapping committed", interrupted.stderr)
        committed_record = self.mapping_record()
        self.assertEqual(committed_record["bootstrap_epoch"], 2)
        self.assertTrue(self.snapshot.exists(), "post-commit injection must precede active snapshot cleanup")
        committed_receipt = committed_record["branch_identity_migration_receipt"]
        journal_before_retry = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertNotEqual(journal_before_retry["state"], "committed")

        retried_receipt = self.assert_migrated(self.run_helper())

        self.assertEqual(retried_receipt, committed_receipt)
        self.assertFalse(self.snapshot.exists())
        journal_after_retry = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertEqual(journal_after_retry["state"], "committed")
        self.assertEqual(journal_after_retry["receipt"], committed_receipt)

    def test_mapping_commit_recovery_replaces_stale_replacement_mapping(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)

        replacement_mapping = self.replacement / ".pm" / "github-project-sync" / "tasks.json"
        replacement_mapping.write_bytes(self.old_mapping_bytes)
        self.assertEqual(
            json.loads(replacement_mapping.read_text(encoding="utf-8"))["tasks"][UID]["bootstrap_epoch"], 1
        )

        self.assert_migrated(self.run_helper())
        replacement_record = json.loads(replacement_mapping.read_text(encoding="utf-8"))["tasks"][UID]
        self.assertEqual(replacement_record["bootstrap_epoch"], 2)
        self.assertEqual(replacement_record["canonical_worktree"], str(self.replacement.resolve()))

    def test_pre_fix_committed_archive_without_bootstrap_epoch_recovers_as_epoch_one(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)

        committed_record = self.mapping_record()
        historical_record = committed_record["historical_epochs"]["1"]["task_record"]
        historical_record.pop("bootstrap_epoch")
        self.replace_mapping_record(committed_record)

        receipt = self.assert_migrated(self.run_helper())

        self.assertEqual(receipt["old_epoch"], 1)
        self.assertFalse(self.snapshot.exists())

    def test_committed_archive_with_malformed_bootstrap_epoch_remains_fail_closed(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)

        committed_record = self.mapping_record()
        committed_record["historical_epochs"]["1"]["task_record"]["bootstrap_epoch"] = None
        self.replace_mapping_record(committed_record)

        retried = self.run_helper()

        self.assertNotEqual(retried.returncode, 0, retried.stdout)
        self.assertIn("committed historical task record disagrees", retried.stderr)
        self.assertTrue(self.snapshot.exists())

    def test_committed_archive_with_conflicting_bootstrap_epoch_remains_fail_closed(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)

        committed_record = self.mapping_record()
        committed_record["historical_epochs"]["1"]["task_record"]["bootstrap_epoch"] = 2
        self.replace_mapping_record(committed_record)

        retried = self.run_helper()

        self.assertNotEqual(retried.returncode, 0, retried.stdout)
        self.assertIn("committed historical task record disagrees", retried.stderr)
        self.assertTrue(self.snapshot.exists())

    def test_committed_journal_recovery_rejects_corrupted_common_dir(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)
        self.assert_migrated(self.run_helper())
        committed_record = self.mapping_record()
        committed_record["branch_identity_migration"]["new_common_dir"] = str(
            self.root.parent / "foreign-common-dir"
        )
        self.replace_mapping_record(committed_record)

        retried = self.run_helper()

        self.assertNotEqual(retried.returncode, 0, retried.stdout)
        self.assertIn("committed migration record disagrees", retried.stderr)
        journal = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertEqual(journal["state"], "committed")

    def test_mapping_commit_recovery_rejects_corrupted_invalidation_set(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)
        committed_record = self.mapping_record()
        committed_record["invalidated_authority"]["fields"] = ["phase_receipts"]
        self.replace_mapping_record(committed_record)

        retried = self.run_helper()

        self.assertNotEqual(retried.returncode, 0, retried.stdout)
        self.assertIn("committed invalidated authority disagrees", retried.stderr)
        self.assertTrue(self.snapshot.exists())
        journal = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertNotEqual(journal["state"], "committed")

    def test_mapping_commit_recovery_rejects_migration_payload_digest_mismatch(self) -> None:
        interrupted = self.run_helper(crash_after="mapping_committed")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)
        committed_record = self.mapping_record()
        committed_record["branch_identity_migration"]["issuer"] = "unauthorized-replacement"
        self.replace_mapping_record(committed_record)

        retried = self.run_helper()

        self.assertNotEqual(retried.returncode, 0, retried.stdout)
        self.assertIn("committed migration record digest is invalid", retried.stderr)
        self.assertTrue(self.snapshot.exists())
        journal = json.loads(
            (self.root / ".pm" / "scratch" / UID / "branch-identity-migration.json").read_text(encoding="utf-8")
        )
        self.assertNotEqual(journal["state"], "committed")

    def test_retry_after_post_branch_creation_crash_reuses_one_branch_and_one_epoch(self) -> None:
        interrupted = self.run_helper(crash_after="replacement_branch_created")
        self.assertNotEqual(interrupted.returncode, 0, interrupted.stdout)
        self.assertIn("injected crash", interrupted.stderr)
        self.assertEqual(self.mapping_record()["bootstrap_epoch"], 1)
        self.assertEqual(self.git("rev-parse", self.replacement_branch), self.old_head)

        receipt = self.assert_migrated(self.run_helper())
        record = self.mapping_record()
        self.assertEqual(record["bootstrap_epoch"], 2)
        self.assertEqual(self.git("worktree", "list", "--porcelain").count(f"branch refs/heads/{self.replacement_branch}"), 1)
        self.assertEqual(receipt["journal_revision"], 1)


if __name__ == "__main__":
    unittest.main()
