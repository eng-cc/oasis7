#!/usr/bin/env python3
"""Table-driven contract tests for the read-only workflow-next query."""
from __future__ import annotations

import json
import hashlib
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
        subprocess.run(["git", "init", "-q", "-b", "task/fixture", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.email", "fixture@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.name", "Fixture"], check=True)
        (self.root / "README").write_text("fixture\n")
        subprocess.run(["git", "-C", str(self.root), "add", "README"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "fixture"], check=True)
        self.origin = self.root.parent / "origin.git"
        subprocess.run(["git", "init", "--bare", "-q", str(self.origin)], check=True)
        subprocess.run(
            ["git", "-C", str(self.root), "remote", "add", "origin", "https://github.com/fixture/repo.git"],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(self.root), "config", f"url.{self.origin}.insteadOf", "https://github.com/fixture/repo.git"],
            check=True,
        )
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
            "cache_refreshed_at": "2026-08-30T00:00:00+00:00",
            "acceptance": ["query is deterministic"],
            "updated_at": "2026-08-30T00:00:00+00:00",
        }
        task.update(updates)
        self.mapping.write_text(json.dumps({"version": 1, "project": {"owner": "fixture", "number": 1}, "tasks": {UID: task}}))
        snapshot = {
            "schema": "oasis7.bootstrap-task-snapshot/v1",
            "task": {"uid": UID, "project": {"item_id": "ITEM1"}},
            "repository": "fixture/repo",
            "git": {"worktree": str(self.root), "branch": "task/fixture"},
            "request": {"identity": "Workflow next fixture"},
            "producer": "fixture",
            "created_at": "2026-08-30T00:00:00Z",
        }
        snapshot["digest"] = "sha256:" + hashlib.sha256(
            json.dumps({k: v for k, v in snapshot.items()}, ensure_ascii=False,
                       sort_keys=True, separators=(",", ":")).encode("utf-8")
        ).hexdigest()
        (self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json").write_text(
            json.dumps(snapshot)
        )

    def install_terminal_proof(self, phase: str) -> None:
        receipt_root = self.root / ".git/oasis7-workflow-receipts" / UID
        receipt_root.mkdir(parents=True, exist_ok=True)

        def write(name: str, value: dict) -> tuple[dict, str]:
            path = receipt_root / name
            path.write_text(json.dumps(value, sort_keys=True) + "\n")
            return value, hashlib.sha256(path.read_bytes()).hexdigest()

        mapping = json.loads(self.mapping.read_text())
        task = mapping["tasks"][UID]
        if phase in {"task_done", "main_sync", "post_merge_done"}:
            merge, merge_digest = write("merge-receipt.json", {
                "receipt_type": "oasis7_merge_receipt", "task_uid": UID,
                "repository": "fixture/repo", "pr_number": 7,
            })
            task.update({"merge_receipt": merge, "merge_receipt_sha256": merge_digest})
        if phase in {"main_sync", "post_merge_done"}:
            sync, sync_digest = write("main-sync-receipt.json", {
                "receipt_type": "oasis7_main_sync", "task_uid": UID,
                "repository": "fixture/repo", "pr_number": 7,
            })
            task.setdefault("phase_receipts", {})["main_sync"] = sync
            task.setdefault("phase_receipt_sha256", {})["main_sync"] = sync_digest
        if phase in {"closed_without_merge", "post_merge_done"}:
            if phase == "closed_without_merge":
                terminal, terminal_digest = write("closed-without-merge-receipt.json", {
                    "receipt_type": "oasis7_closed_without_merge",
                    "schema_version": 1, "issuer": "non-merge-finalize",
                    "task_uid": UID, "repository": "fixture/repo",
                })
                ledger_name = "non-merge-finalizer-ledger.json"
            else:
                terminal, terminal_digest = write("terminal-cleanup-receipt.json", {
                    "receipt_type": "oasis7_terminal_cleanup",
                    "issuer": "post-merge-cleanup", "task_uid": UID,
                    "repository": "fixture/repo",
                })
                ledger_name = "finalizer-ledger.json"
            write(ledger_name, {"schema": "oasis7_finalizer_ledger_v1",
                                "task_uid": UID, "revision": 1,
                                "operations": {"terminal": {"committed": True}}})
            write("terminal-tombstone.json", {
                "schema": "oasis7_terminal_tombstone_v1", "task_uid": UID,
                "repository": "fixture/repo", "workflow_phase": phase,
                "terminal_receipt_sha256": terminal_digest,
                "checkout_recreation_forbidden": True,
            })
            task.setdefault("phase_receipts", {})[phase] = terminal
            task.setdefault("phase_receipt_sha256", {})[phase] = terminal_digest
        self.mapping.write_text(json.dumps(mapping))

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
            ({"status": "committed", "workflow_phase": "verification", "pr_url": "https://github.com/fixture/repo/pull/7", "pr_number": 7}, "verification", "prepare-task-pr.sh"),
            ({"status": "pr_watch", "workflow_phase": "pr_watch", "pr_url": "https://github.com/fixture/repo/pull/7", "pr_number": 7}, "pr_watch", "pr-lifecycle-gate.py"),
            ({"status": "done", "workflow_phase": "task_done", "pr_url": "https://github.com/fixture/repo/pull/7", "pr_number": 7}, "task_done", "finalize-task.sh"),
            ({"status": "done", "workflow_phase": "main_sync", "pr_url": "https://github.com/fixture/repo/pull/7", "pr_number": 7}, "main_sync", "finalize-task.sh"),
            ({"status": "done", "workflow_phase": "task_done", "completion_mode": "non_pr_task", "non_pr_completion_evidence": "completed"}, "task_done", "non-merge-finalize.py"),
        ]
        for updates, phase, command in cases:
            with self.subTest(updates=updates):
                self.write_mapping(**updates)
                if updates.get("completion_mode") == "non_pr_task":
                    evidence = self.root / ".pm/scratch" / UID / "non-pr-completion-evidence.txt"
                    evidence.write_text("completed\n")
                    mapping = json.loads(self.mapping.read_text())
                    mapping["tasks"][UID]["non_pr_completion_evidence_file"] = str(evidence)
                    mapping["tasks"][UID]["non_pr_completion_evidence_sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
                    self.mapping.write_text(json.dumps(mapping))
                if phase in {"task_done", "main_sync"}:
                    self.install_terminal_proof(phase)
                code, payload = self.run_query()
                self.assertEqual(code, 0, payload)
                self.assertEqual(payload["workflow_phase"], phase, payload)
                self.assertEqual(payload["blockers"], [], payload)
                self.assertTrue(payload["next_command"], payload)
                self.assertIn(command, " ".join(payload["next_command"]), payload)
                if phase == "task_done" and updates.get("pr_number"):
                    self.assertIn("--resume", payload["next_command"], payload)
                    self.assertNotIn("--preflight", payload["next_command"], payload)

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
                           pr_number=7, pr_url="https://github.com/fixture/repo/pull/8")
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertTrue(any("PR URL" in item for item in payload["blockers"]), payload)

    def test_mapping_and_evidence_identity_drift_fails_closed(self) -> None:
        mapping_cases = (
            ({"task_uid": "task_22222222222222222222222222222222"}, "task UID"),
            ({"repository": "not-a-repository"}, "repository"),
            ({"canonical_worktree": str(self.root / "missing")}, "worktree"),
        )
        for updates, marker in mapping_cases:
            with self.subTest(mapping=updates):
                self.write_mapping(status="committed", workflow_phase="execution", **updates)
                code, payload = self.run_query()
                self.assertNotEqual(code, 0, payload)
                self.assertEqual(payload["next_command"], [], payload)
                self.assertTrue(any(marker.lower() in item.lower() for item in payload["blockers"]), payload)

        self.write_mapping(status="committed", workflow_phase="execution")
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        snapshot.write_text(json.dumps({
            "schema": "oasis7.bootstrap-task-snapshot/v1",
            "task": {"project": {"item_id": "ITEM1"}},
            "repository": "fixture/repo",
            "git": {"worktree": str(self.root), "branch": "task/fixture"},
        }))
        with self.subTest(evidence="snapshot"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertTrue(any("snapshot" in item.lower() for item in payload["blockers"]), payload)

        snapshot.unlink()
        ledger = self.root / ".pm/scratch" / UID / "slice-ledger.jsonl"
        ledger.write_text(json.dumps({"role": "repository_health_engineer"}) + "\n")
        with self.subTest(evidence="ledger"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertTrue(any("ledger" in item.lower() for item in payload["blockers"]), payload)

        ledger.unlink()
        checkpoint = self.root / ".pm/tasks" / f"{UID}.workflow.json"
        checkpoint.parent.mkdir(parents=True)
        checkpoint.write_text(json.dumps({"repo": str(self.root), "phase": "execution"}))
        with self.subTest(evidence="checkpoint"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertTrue(any("checkpoint" in item.lower() for item in payload["blockers"]), payload)

    def test_repository_and_issue_identity_fail_closed(self) -> None:
        self.write_mapping(status="committed", workflow_phase="execution")

        self.write_mapping(
            status="committed",
            workflow_phase="execution",
            issue_url="https://example.invalid/issues/11",
        )
        with self.subTest(identity="malformed-issue-url"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertEqual(payload["next_command"], [], payload)
            self.assertTrue(any("Issue URL" in item for item in payload["blockers"]), payload)

        self.write_mapping(
            status="committed",
            workflow_phase="execution",
            pr_number=7,
            pr_url="https://example.invalid/pull/7",
        )
        with self.subTest(identity="unsupported-pr-url"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertEqual(payload["next_command"], [], payload)
            self.assertTrue(any("PR URL" in item for item in payload["blockers"]), payload)

        self.write_mapping(status="committed", workflow_phase="execution")
        checkpoint = self.root / ".pm/tasks" / f"{UID}.workflow.json"
        checkpoint.parent.mkdir(parents=True, exist_ok=True)
        checkpoint.write_text(json.dumps({
            "task_uid": UID,
            "schema": "tpm-production-supervisor/v2",
            "revision": 1,
            "repo": str(self.root),
            "repository": "https://github.com/fixture/repo.git",
            "phase": "execution",
            "status": "running",
            "capability_status": "blocked",
            "terminal_authority": {
                "task_uid": UID,
                "repository": "https://github.com/fixture/repo.git",
                "canonical_worktree": str(self.root),
                "task_branch": "task/fixture",
                "default_branch": "main",
            },
        }))
        with self.subTest(identity="checkpoint-github-url"):
            code, payload = self.run_query()
            self.assertEqual(code, 0, payload)
            self.assertEqual(payload["blockers"], [], payload)
            self.assertTrue(payload["next_command"], payload)

        checkpoint.write_text(json.dumps({
            "task_uid": UID,
            "repo": str(self.root),
            "repository": "fixture/other-repo",
            "phase": "execution",
            "terminal_authority": {
                "task_uid": UID,
                "repository": "fixture/repo",
                "canonical_worktree": str(self.root),
                "task_branch": "task/fixture",
                "default_branch": "main",
            },
        }))
        with self.subTest(identity="checkpoint-repository"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertEqual(payload["next_command"], [], payload)
            self.assertTrue(any("checkpoint" in item.lower() for item in payload["blockers"]), payload)

        checkpoint.write_text(json.dumps({
            "schema": "tpm-production-supervisor/v2",
            "revision": 1,
            "task_uid": UID,
            "repo": str(self.root),
            "repository": "fixture/repo",
            "phase": "execution",
            "status": "running",
            "capability_status": "blocked",
            "terminal_authority": {
                "task_uid": UID,
                "repository": "fixture/other-repo",
                "canonical_worktree": str(self.root),
                "task_branch": "task/fixture",
                "default_branch": "main",
            },
        }))
        with self.subTest(identity="checkpoint-terminal-repository"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertEqual(payload["next_command"], [], payload)
            self.assertTrue(any("terminal" in item.lower() and "repository" in item.lower()
                                for item in payload["blockers"]), payload)

        checkpoint.unlink()
        subprocess.run(["git", "-C", str(self.root), "remote", "remove", "origin"], check=True)
        with self.subTest(identity="missing-origin"):
            code, payload = self.run_query()
            self.assertNotEqual(code, 0, payload)
            self.assertEqual(payload["next_command"], [], payload)
            self.assertTrue(any("origin" in item.lower() for item in payload["blockers"]), payload)

    def test_phase_reducer_accepts_planning_blocked_and_deferred_as_action_required(self) -> None:
        cases = (
            ({"status": "candidate", "workflow_phase": "planning"}, "planning"),
            ({"status": "blocked", "workflow_phase": "blocked"}, "blocked"),
            ({"status": "deferred", "workflow_phase": ""}, "blocked"),
        )
        for updates, expected_phase in cases:
            with self.subTest(updates=updates):
                self.write_mapping(**updates)
                code, payload = self.run_query()
                self.assertEqual(code, 0, payload)
                self.assertEqual(payload["workflow_phase"], expected_phase, payload)
                self.assertEqual(payload["blockers"], [], payload)
                self.assertEqual(payload["next_command"], [], payload)
                self.assertEqual(payload["next_action"], "action_required", payload)

    def test_terminal_phases_require_receipt_ledger_and_tombstone_proof(self) -> None:
        self.write_mapping(status="done", workflow_phase="closed_without_merge")
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertTrue(any("terminal" in item.lower() or "receipt" in item.lower()
                            for item in payload["blockers"]), payload)

    def test_matching_invalid_snapshot_and_checkpoint_fail_closed(self) -> None:
        self.write_mapping(status="committed", workflow_phase="execution")
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        snapshot.write_text(json.dumps({
            "schema": "not-the-bootstrap-schema",
            "task": {"uid": UID},
            "repository": "fixture/repo",
            "git": {"worktree": str(self.root), "branch": "task/fixture"},
            "digest": "sha256:" + "0" * 64,
        }))
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("snapshot" in item.lower() for item in payload["blockers"]), payload)

        snapshot.unlink()
        checkpoint = self.root / ".pm/tasks" / f"{UID}.workflow.json"
        checkpoint.parent.mkdir(parents=True, exist_ok=True)
        checkpoint.write_text(json.dumps({
            "schema": "not-the-supervisor-schema",
            "revision": 1,
            "task_uid": UID,
            "repo": str(self.root),
            "repository": "fixture/repo",
            "phase": "execution",
            "status": "running",
        }))
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("checkpoint" in item.lower() for item in payload["blockers"]), payload)

    def test_next_command_declares_canonical_execution_cwd(self) -> None:
        self.write_mapping(status="committed", workflow_phase="execution")
        code, payload = self.run_query()
        self.assertEqual(code, 0, payload)
        self.assertEqual(payload["command_cwd"], str(self.root.resolve()), payload)


if __name__ == "__main__":
    unittest.main()
