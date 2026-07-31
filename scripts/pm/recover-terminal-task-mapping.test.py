#!/usr/bin/env python3
# This recovery fixture must remain compatible with POSIX and native Windows Python.
from __future__ import annotations

import datetime as dt
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts/pm/recover-terminal-task-mapping.py"
UID = "task_11111111111111111111111111111111"


class RecoveryTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.repo = self.root / "repo"
        self.worktree = self.root / "task-worktree"
        self.mapping = self.repo / ".pm/github-project-sync/tasks.json"
        subprocess.run(["git", "init", "-q", "-b", "main", str(self.repo)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.name", "Test"], check=True)
        (self.repo / "file").write_text("base\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "file"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-qm", "base"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "worktree", "add", "-qb", "task/recovery", str(self.worktree)], check=True)
        self.mapping.parent.mkdir(parents=True)
        self.mapping.write_text('{"version":1,"tasks":{}}\n', encoding="utf-8")
        self.record = {
            "task_uid": UID, "status": "done", "repository": "fixture/repo",
            "default_branch": "main", "canonical_worktree": str(self.worktree),
            "task_branch": "task/recovery", "pr_number": 7,
            "pr_url": "https://example.invalid/pull/7",
        }
        retained = self.worktree / ".pm/github-project-sync/tasks.json"
        retained.parent.mkdir(parents=True)
        retained.write_text(json.dumps({"version": 1, "tasks": {UID: self.record}}) + "\n", encoding="utf-8")
        self.receipt = self.root / "merge-receipt.json"
        self.receipt.write_text(json.dumps({
            "receipt_type": "oasis7_pr_merge", "issuer": "github_live_query",
            "evidence_mode": "production", "repository": "fixture/repo",
            "default_branch": "main", "pr_number": 7,
            "pr_url": "https://example.invalid/pull/7", "state": "MERGED",
            "merged_at": "2026-01-01T00:00:00Z", "head_oid": "a" * 40,
            "base_ref": "main", "observed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        }) + "\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def invoke(self, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run([
            sys.executable, str(HELPER), "--repo-root", str(self.repo),
            "--mapping", str(self.mapping), "--task-uid", UID,
            "--main-ref", "main", "--pr-receipt", str(self.receipt),
        ], text=True, capture_output=True)
        self.assertEqual(0 if ok else 1, result.returncode, result.stderr)
        return result

    def write_retained(self) -> None:
        path = self.worktree / ".pm/github-project-sync/tasks.json"
        path.write_text(json.dumps({"version": 1, "tasks": {UID: self.record}}) + "\n", encoding="utf-8")

    def test_imports_complete_registered_terminal_record(self) -> None:
        self.assertIn("imported", self.invoke().stdout)
        imported = json.loads(self.mapping.read_text(encoding="utf-8"))["tasks"][UID]
        self.assertEqual(self.record, imported)

    def test_rejects_incomplete_identity(self) -> None:
        del self.record["pr_url"]
        self.write_retained()
        self.assertIn("missing pr_url", self.invoke(ok=False).stderr)

    def test_rejects_receipt_identity_mismatch(self) -> None:
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["repository"] = "foreign/repo"
        self.receipt.write_text(json.dumps(receipt) + "\n", encoding="utf-8")
        self.assertIn("repository disagrees", self.invoke(ok=False).stderr)

    def test_rejects_unregistered_canonical_worktree(self) -> None:
        self.record["canonical_worktree"] = str(self.root / "unregistered")
        self.write_retained()
        self.assertIn("not registered", self.invoke(ok=False).stderr)

    def test_rejects_conflicting_registered_terminal_records(self) -> None:
        second = self.root / "second-task-worktree"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-qb",
            "task/recovery-two", str(second),
        ], check=True)
        conflict = dict(
            self.record, canonical_worktree=str(second),
            task_branch="task/recovery-two",
        )
        path = second / ".pm/github-project-sync/tasks.json"
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps({"version": 1, "tasks": {UID: conflict}}) + "\n", encoding="utf-8")
        self.assertIn("conflicting terminal task identities", self.invoke(ok=False).stderr)

    def test_does_not_import_over_existing_incomplete_entry(self) -> None:
        self.mapping.write_text(json.dumps({"version": 1, "tasks": {UID: None}}) + "\n", encoding="utf-8")
        before = self.mapping.read_bytes()
        self.assertIn("existing terminal task record is incomplete", self.invoke(ok=False).stderr)
        self.assertEqual(before, self.mapping.read_bytes())
        mapping = json.loads(self.mapping.read_text(encoding="utf-8"))
        self.assertIsNone(mapping["tasks"][UID])

    def test_transaction_rejects_present_null_added_after_discovery(self) -> None:
        spec = importlib.util.spec_from_file_location("recovery_helper", HELPER)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)

        class RacingStore:
            @staticmethod
            def transact_json(path, update, default):
                data = json.loads(path.read_text(encoding="utf-8"))
                data["tasks"][UID] = None
                update(data)

        before = self.mapping.read_bytes()
        with self.assertRaisesRegex(SystemExit, "conflicting task record"):
            helper.import_recovered(RacingStore, self.mapping, UID, self.record)
        self.assertEqual(before, self.mapping.read_bytes())


if __name__ == "__main__":
    unittest.main()
