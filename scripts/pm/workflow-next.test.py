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
BOOTSTRAP = ROOT / "scripts" / "pm" / "bootstrap-task-snapshot.py"
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
        self.default_root = Path(self.tmp.name) / "default"
        subprocess.run(
            ["git", "-C", str(self.root), "worktree", "add", "-q", "-b", "main", str(self.default_root), "HEAD"],
            check=True,
        )
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
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        snapshot.unlink(missing_ok=True)
        self.mapping.write_text(json.dumps({"version": 1, "project": {"owner": "fixture", "number": 1, "id": "PROJECT1"}, "tasks": {UID: task}}))
        subprocess.run(
            [sys.executable, str(BOOTSTRAP), "create", "--repo-root", str(self.root),
             "--task-uid", UID, "--request-identity", "Workflow next fixture", "--producer", "fixture"],
            check=True,
            capture_output=True,
        )
        task.update(updates)
        self.mapping.write_text(json.dumps({"version": 1, "project": {"owner": "fixture", "number": 1, "id": "PROJECT1"}, "tasks": {UID: task}}))

    def install_terminal_proof(self, phase: str, *, producer_shaped_merge: bool = False,
                               non_merge_reason: str = "non_pr_completed",
                               non_merge_receipt_overrides: dict[str, object] | None = None) -> None:
        receipt_root = self.root / ".git/oasis7-workflow-receipts" / UID
        receipt_root.mkdir(parents=True, exist_ok=True)

        def write(name: str, value: dict) -> tuple[dict, str]:
            path = receipt_root / name
            path.write_text(json.dumps(value, sort_keys=True) + "\n")
            return value, hashlib.sha256(path.read_bytes()).hexdigest()

        mapping = json.loads(self.mapping.read_text())
        task = mapping["tasks"][UID]
        if phase in {"task_done", "main_sync", "post_merge_done"}:
            merge = {
                "receipt_type": "oasis7_pr_merge",
                "issuer": "github_live_query",
                "evidence_mode": "production",
                "repository": "fixture/repo",
                "default_branch": "main",
                "pr_number": task.get("pr_number") or 7,
                "pr_url": task.get("pr_url") or "https://github.com/fixture/repo/pull/7",
                "state": "MERGED",
                "merged_at": "2026-08-30T00:00:00Z",
                "head_oid": "a" * 40,
                "base_ref": "main",
                "observed_at": "2026-08-30T00:00:00Z",
            }
            if not producer_shaped_merge:
                merge["task_uid"] = UID
            merge, merge_digest = write("merge-receipt.json", merge)
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
                evidence_digest = "a" * 64
                task["closed_without_merge_reason"] = non_merge_reason
                task["closed_without_merge_evidence_sha256"] = evidence_digest
                if non_merge_reason in {"superseded", "duplicate"}:
                    task.update({"headRefOid": "a" * 40, "headRefName": task["task_branch"]})
                terminal_value = {
                    "receipt_type": "oasis7_closed_without_merge",
                    "schema_version": 1, "issuer": "non-merge-finalize",
                    "task_uid": UID, "repository": "fixture/repo",
                    "issue_number": task["issue_number"],
                    "project_item_id": task["project_item_id"],
                    "project_identity": {key: str(mapping["project"][key])
                                         for key in ("owner", "number", "id")},
                    "reason": non_merge_reason,
                    "evidence_sha256": evidence_digest,
                    "evidence": {"text": "fixture terminal evidence"},
                    "previous_status": "done" if non_merge_reason == "non_pr_completed" else "committed",
                    "previous_workflow_phase": "task_done" if non_merge_reason == "non_pr_completed" else "execution",
                    "pr_number": task.get("pr_number"), "pr_url": task.get("pr_url"),
                    "pr_state": "CLOSED" if task.get("pr_number") else None,
                    "mergedAt": None,
                }
                if non_merge_reason in {"superseded", "duplicate"}:
                    terminal_value.update({
                        "headRefOid": "a" * 40,
                        "headRefName": task["task_branch"],
                    })
                terminal_value.update(non_merge_receipt_overrides or {})
                terminal, terminal_digest = write(
                    "closed-without-merge-receipt.json", terminal_value,
                )
                ledger_name = "non-merge-finalizer-ledger.json"
            else:
                terminal, terminal_digest = write("terminal-cleanup-receipt.json", {
                    "receipt_type": "oasis7_terminal_cleanup",
                    "issuer": "post-merge-cleanup", "task_uid": UID,
                    "repository": "fixture/repo",
                    "worktree": str(task.get("canonical_worktree") or self.root),
                    "branch": task.get("task_branch") or "task/fixture",
                })
                ledger_name = "finalizer-ledger.json"
            if phase == "closed_without_merge":
                bound_head = {
                    key: task[key] for key in (
                        "headRefOid", "headRefName", "headRepositoryOwner", "headRepositoryName",
                    ) if key in task
                }

                def operation_id(effect: str) -> str:
                    payload = {
                        "task_uid": UID,
                        "phase": "closed_without_merge",
                        "effect": effect,
                        "pr_head": bound_head,
                    }
                    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
                    return hashlib.sha256(encoded).hexdigest()

                previous_body = (
                    "- status: `done`\n"
                    "- workflow_phase: `closed_without_merge`\n"
                )
                close_state_reason = "COMPLETED" if non_merge_reason == "non_pr_completed" else "NOT_PLANNED"
                issue_readback = {
                    "state": "CLOSED",
                    "stateReason": close_state_reason,
                    "body": previous_body,
                    "number": task["issue_number"],
                    "url": task["issue_url"],
                }
                operations = {
                    "issue_body_update": {
                        "operation_id": operation_id("issue_body_update"),
                        "effect": "issue_body_update", "readback": issue_readback,
                        "committed": True,
                    },
                    "project_update": {
                        "operation_id": operation_id("project_update"),
                        "effect": "project_update",
                        "readback": {"Status": "Done", "PM Status": "done", "Workflow Phase": "done"},
                        "committed": True,
                    },
                    "evidence_comment": {
                        "operation_id": operation_id("evidence_comment"),
                        "effect": "evidence_comment",
                        "readback": f"https://github.com/fixture/repo/issues/{task['issue_number']}#issuecomment-99",
                        "committed": True,
                    },
                    "issue_close": {
                        "operation_id": operation_id("issue_close"),
                        "effect": "issue_close", "readback": issue_readback,
                        "committed": True,
                    },
                }
                write(ledger_name, {
                    "schema": "oasis7_non_merge_finalizer_ledger_v1",
                    "task_uid": UID,
                    "operations": operations,
                })
            else:
                write(ledger_name, {"schema": "oasis7_finalizer_ledger_v1",
                                    "task_uid": UID, "revision": 1,
                                    "operations": {"terminal": {"committed": True}}})
            write("terminal-tombstone.json", {
                "schema": "oasis7_terminal_tombstone_v1", "task_uid": UID,
                "repository": "fixture/repo", "issue_number": task["issue_number"],
                "pr_number": task.get("pr_number"),
                "canonical_worktree": task["canonical_worktree"],
                "task_branch": task["task_branch"], "workflow_phase": phase,
                "terminal_receipt_sha256": terminal_digest,
                "checkout_recreation_forbidden": True,
            })
            task.setdefault("phase_receipts", {})[phase] = terminal
            task.setdefault("phase_receipt_sha256", {})[phase] = terminal_digest
        self.mapping.write_text(json.dumps(mapping))

    def rewrite_non_merge_terminal_receipt(
        self, *, remove: tuple[str, ...] = (),
        overrides: dict[str, object] | None = None,
        tombstone_overrides: dict[str, object] | None = None,
    ) -> None:
        receipt_root = self.root / ".git/oasis7-workflow-receipts" / UID
        receipt_path = receipt_root / "closed-without-merge-receipt.json"
        receipt = json.loads(receipt_path.read_text())
        for key in remove:
            receipt.pop(key, None)
        receipt.update(overrides or {})
        receipt_path.write_text(json.dumps(receipt, sort_keys=True) + "\n")
        digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()

        mapping = json.loads(self.mapping.read_text())
        task = mapping["tasks"][UID]
        task.setdefault("phase_receipts", {})["closed_without_merge"] = receipt
        task.setdefault("phase_receipt_sha256", {})["closed_without_merge"] = digest
        task["closed_without_merge_receipt"] = receipt
        self.mapping.write_text(json.dumps(mapping))

        tombstone_path = receipt_root / "terminal-tombstone.json"
        tombstone = json.loads(tombstone_path.read_text())
        tombstone["terminal_receipt_sha256"] = digest
        tombstone.update(tombstone_overrides or {})
        tombstone_path.write_text(json.dumps(tombstone, sort_keys=True) + "\n")

    def run_query(self, *extra: str, repo_root: Path | None = None) -> tuple[int, dict]:
        query_root = repo_root or self.root
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--repo-root", str(query_root), "--task-uid", UID, "--json", *extra],
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

    def test_non_pr_terminal_receipt_and_ledger_use_producer_schema(self) -> None:
        self.write_mapping(
            status="done", workflow_phase="closed_without_merge",
            completion_mode="non_pr_task",
        )
        self.install_terminal_proof("closed_without_merge")

        code, payload = self.run_query()

        self.assertEqual(code, 0, payload)
        self.assertEqual(payload["identity_status"], "bound", payload)
        self.assertEqual(payload["workflow_phase"], "closed_without_merge", payload)
        self.assertEqual(payload["next_action"], "completed", payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertEqual(payload["blockers"], [], payload)

    def test_non_pr_bound_non_merge_reasons_use_non_merge_producer_schema(self) -> None:
        for reason in ("not_planned", "superseded", "duplicate"):
            with self.subTest(reason=reason):
                updates: dict[str, object] = {}
                if reason in {"superseded", "duplicate"}:
                    updates.update({
                        "pr_number": 7,
                        "pr_url": "https://github.com/fixture/repo/pull/7",
                    })
                self.write_mapping(
                    status="done", workflow_phase="closed_without_merge", **updates,
                )
                self.install_terminal_proof(
                    "closed_without_merge", non_merge_reason=reason,
                )

                code, payload = self.run_query()

                self.assertEqual(code, 0, payload)
                self.assertEqual(payload["identity_status"], "bound", payload)
                self.assertEqual(payload["workflow_phase"], "closed_without_merge", payload)
                self.assertEqual(payload["next_action"], "completed", payload)
                self.assertEqual(payload["next_command"], [], payload)
                self.assertEqual(payload["blockers"], [], payload)

    def test_non_merge_terminal_producer_schema_still_checks_optional_worktree_and_branch(self) -> None:
        self.write_mapping(status="done", workflow_phase="closed_without_merge")
        self.install_terminal_proof(
            "closed_without_merge", non_merge_reason="not_planned",
            non_merge_receipt_overrides={
                "worktree": str(self.root / "forged-task"),
                "branch": "task/forged",
            },
        )

        code, payload = self.run_query()

        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("terminal receipt worktree identity drift" in item
                            for item in payload["blockers"]), payload)
        self.assertTrue(any("terminal receipt branch identity drift" in item
                            for item in payload["blockers"]), payload)

    def test_non_merge_receipt_requires_issue_and_project_identity_for_each_reason(self) -> None:
        for reason in ("superseded", "duplicate", "not_planned", "non_pr_completed"):
            for missing in ("issue_number", "project_item_id", "project_identity"):
                with self.subTest(reason=reason, missing=missing):
                    updates: dict[str, object] = {}
                    if reason in {"superseded", "duplicate"}:
                        updates.update({
                            "pr_number": 7,
                            "pr_url": "https://github.com/fixture/repo/pull/7",
                        })
                    if reason == "non_pr_completed":
                        updates["completion_mode"] = "non_pr_task"
                    self.write_mapping(
                        status="done", workflow_phase="closed_without_merge", **updates,
                    )
                    self.install_terminal_proof(
                        "closed_without_merge", non_merge_reason=reason,
                    )
                    self.rewrite_non_merge_terminal_receipt(remove=(missing,))

                    code, payload = self.run_query()

                    self.assertNotEqual(code, 0, payload)
                    self.assertTrue(any("terminal receipt" in item.lower()
                                        and ("identity" in item.lower() or "project" in item.lower())
                                        for item in payload["blockers"]), payload)

    def test_duplicate_receipt_requires_exact_closed_unmerged_pr_authority(self) -> None:
        cases = (
            ({"pr_number": 8, "pr_url": "https://github.com/fixture/repo/pull/8"}, "PR"),
            ({"pr_state": "MERGED", "mergedAt": "2026-09-24T00:00:00Z"}, "merged"),
            ({"headRefOid": "f" * 40}, "head"),
        )
        for overrides, marker in cases:
            with self.subTest(marker=marker):
                self.write_mapping(
                    status="done", workflow_phase="closed_without_merge",
                    pr_number=7, pr_url="https://github.com/fixture/repo/pull/7",
                )
                self.install_terminal_proof(
                    "closed_without_merge", non_merge_reason="duplicate",
                )
                self.rewrite_non_merge_terminal_receipt(overrides=overrides)

                code, payload = self.run_query()

                self.assertNotEqual(code, 0, payload)
                self.assertTrue(any("pr" in item.lower() or "merged" in item.lower()
                                    for item in payload["blockers"]), payload)

    def test_non_merge_receipt_reason_and_evidence_digest_match_task_truth(self) -> None:
        cases = (
            ({"evidence_sha256": "b" * 64}, "evidence digest"),
            ({}, "reason"),
        )
        for overrides, marker in cases:
            with self.subTest(marker=marker):
                self.write_mapping(status="done", workflow_phase="closed_without_merge")
                self.install_terminal_proof(
                    "closed_without_merge", non_merge_reason="not_planned",
                )
                self.rewrite_non_merge_terminal_receipt(overrides=overrides)
                if marker == "reason":
                    mapping = json.loads(self.mapping.read_text())
                    mapping["tasks"][UID]["closed_without_merge_reason"] = "superseded"
                    self.mapping.write_text(json.dumps(mapping))

                code, payload = self.run_query()

                self.assertNotEqual(code, 0, payload)
                self.assertTrue(any(marker in item.lower() for item in payload["blockers"]), payload)

    def test_non_merge_tombstone_binds_task_terminal_identity(self) -> None:
        self.write_mapping(status="done", workflow_phase="closed_without_merge")
        self.install_terminal_proof(
            "closed_without_merge", non_merge_reason="not_planned",
        )
        self.rewrite_non_merge_terminal_receipt(
            tombstone_overrides={"issue_number": 12, "task_branch": "task/foreign"},
        )

        code, payload = self.run_query()

        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("tombstone" in item.lower()
                            and "identity" in item.lower() for item in payload["blockers"]), payload)

    def test_non_merge_ledger_requires_committed_issue_close_operation(self) -> None:
        cases = (
            ("issue_close", "operation_id", "c" * 64),
            ("issue_close", "readback", None),
            ("issue_body_update", "operation", None),
            ("migrated ledger", "migration marker", None),
        )
        for effect, mutation, value in cases:
            with self.subTest(effect=effect, mutation=mutation):
                self.write_mapping(status="done", workflow_phase="closed_without_merge")
                self.install_terminal_proof(
                    "closed_without_merge", non_merge_reason="not_planned",
                )
                ledger_path = self.root / ".git/oasis7-workflow-receipts" / UID / "non-merge-finalizer-ledger.json"
                ledger = json.loads(ledger_path.read_text())
                if mutation == "migration marker":
                    for missing_effect in ("issue_body_update", "project_update", "evidence_comment"):
                        ledger["operations"].pop(missing_effect)
                    ledger["migrated_from"] = "unverified-source.json"
                    ledger["migrated_from_sha256"] = "d" * 64
                elif mutation == "operation":
                    ledger["operations"].pop(effect)
                elif value is None:
                    ledger["operations"][effect].pop(mutation)
                else:
                    ledger["operations"][effect][mutation] = value
                ledger_path.write_text(json.dumps(ledger, sort_keys=True) + "\n")

                code, payload = self.run_query()

                self.assertNotEqual(code, 0, payload)
                self.assertTrue(any("ledger" in item.lower() or effect in item.lower()
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

    def test_terminal_command_targets_canonical_default_worktree(self) -> None:
        self.write_mapping(
            status="done",
            workflow_phase="main_sync",
            pr_url="https://github.com/fixture/repo/pull/7",
            pr_number=7,
        )
        self.install_terminal_proof("main_sync")
        code, payload = self.run_query()
        self.assertEqual(code, 0, payload)
        self.assertEqual(payload["command_cwd"], str(self.default_root.resolve()), payload)
        command = payload["next_command"]
        self.assertEqual(command[command.index("--repo-root") + 1], str(self.default_root.resolve()), payload)

    def test_snapshot_immutable_fields_use_strict_bootstrap_validator(self) -> None:
        self.write_mapping(status="committed", workflow_phase="execution")
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        saved = json.loads(snapshot.read_text())
        saved["task"]["project"]["item_id"] = "FORGED_ITEM"
        saved["task"]["acceptance"] = ["forged acceptance"]
        saved["request"]["acceptance"] = ["forged acceptance"]
        saved["digest"] = "sha256:" + hashlib.sha256(
            json.dumps({k: v for k, v in saved.items() if k != "digest"},
                       ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        snapshot.write_text(json.dumps(saved))
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("snapshot" in item.lower() and "drift" in item.lower()
                            for item in payload["blockers"]), payload)

    def test_terminal_receipt_worktree_and_branch_identity_is_bound(self) -> None:
        self.write_mapping(
            status="done",
            workflow_phase="post_merge_done",
            pr_url="https://github.com/fixture/repo/pull/7",
            pr_number=7,
        )
        self.install_terminal_proof("post_merge_done")
        terminal_path = self.root / ".git/oasis7-workflow-receipts" / UID / "terminal-cleanup-receipt.json"
        terminal = json.loads(terminal_path.read_text())
        terminal["worktree"] = str(self.root / "forged-task")
        terminal["branch"] = "task/forged"
        terminal_path.write_text(json.dumps(terminal, sort_keys=True) + "\n")
        mapping = json.loads(self.mapping.read_text())
        mapping["tasks"][UID]["phase_receipts"]["post_merge_done"] = terminal
        mapping["tasks"][UID]["phase_receipt_sha256"]["post_merge_done"] = hashlib.sha256(
            terminal_path.read_bytes()
        ).hexdigest()
        tombstone_path = terminal_path.with_name("terminal-tombstone.json")
        tombstone = json.loads(tombstone_path.read_text())
        tombstone["terminal_receipt_sha256"] = mapping["tasks"][UID]["phase_receipt_sha256"]["post_merge_done"]
        tombstone_path.write_text(json.dumps(tombstone, sort_keys=True) + "\n")
        self.mapping.write_text(json.dumps(mapping))
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("terminal receipt" in item.lower() and
                            ("worktree" in item.lower() or "branch" in item.lower())
                            for item in payload["blockers"]), payload)

    def test_post_merge_terminal_query_uses_default_worktree_after_task_checkout_removal(self) -> None:
        task_worktree = Path(self.tmp.name) / "retired-task-worktree"
        subprocess.run([
            "git", "-C", str(self.root), "worktree", "add", "-qb", "task/retired", str(task_worktree),
        ], check=True)
        self.write_mapping(
            status="done", workflow_phase="post_merge_done",
            canonical_worktree=str(task_worktree), task_branch="task/retired",
            pr_url="https://github.com/fixture/repo/pull/7", pr_number=7,
        )
        self.install_terminal_proof("post_merge_done")
        default_mapping = self.default_root / ".pm/github-project-sync/tasks.json"
        default_mapping.parent.mkdir(parents=True, exist_ok=True)
        default_mapping.write_bytes(self.mapping.read_bytes())
        subprocess.run(["git", "-C", str(self.root), "worktree", "remove", "--force", str(task_worktree)], check=True)
        code, payload = self.run_query(repo_root=self.default_root)
        self.assertEqual(code, 0, payload)
        self.assertEqual(payload["identity_status"], "bound", payload)
        self.assertEqual(payload["workflow_phase"], "post_merge_done", payload)
        self.assertEqual(payload["next_action"], "completed", payload)
        self.assertEqual(payload["next_command"], [], payload)
        self.assertEqual(payload["blockers"], [], payload)

    def test_producer_shaped_merge_receipt_without_task_uid_is_accepted(self) -> None:
        self.write_mapping(
            status="done", workflow_phase="post_merge_done",
            pr_url="https://github.com/fixture/repo/pull/7", pr_number=7,
        )
        self.install_terminal_proof("post_merge_done", producer_shaped_merge=True)
        code, payload = self.run_query()
        self.assertEqual(code, 0, payload)
        self.assertEqual(payload["identity_status"], "bound", payload)
        self.assertEqual(payload["next_action"], "completed", payload)
        self.assertEqual(payload["blockers"], [], payload)

    def test_producer_shaped_merge_keeps_digest_repository_pr_and_chain_guards(self) -> None:
        cases = (
            ("task_uid", "task_22222222222222222222222222222222", "task/repository identity drift"),
            ("repository", "fixture/other-repo", "task/repository identity drift"),
            ("pr_number", 8, "PR number identity drift"),
            ("pr_url", "https://github.com/fixture/repo/pull/8", "PR URL identity drift"),
        )
        for field, value, marker in cases:
            with self.subTest(field=field):
                self.write_mapping(
                    status="done", workflow_phase="post_merge_done",
                    pr_url="https://github.com/fixture/repo/pull/7", pr_number=7,
                )
                self.install_terminal_proof("post_merge_done", producer_shaped_merge=True)
                merge_path = self.root / ".git/oasis7-workflow-receipts" / UID / "merge-receipt.json"
                merge = json.loads(merge_path.read_text())
                merge[field] = value
                merge_path.write_text(json.dumps(merge, sort_keys=True) + "\n")
                mapping = json.loads(self.mapping.read_text())
                mapping["tasks"][UID]["merge_receipt"] = merge
                mapping["tasks"][UID]["merge_receipt_sha256"] = hashlib.sha256(
                    merge_path.read_bytes()
                ).hexdigest()
                self.mapping.write_text(json.dumps(mapping))
                code, payload = self.run_query()
                self.assertNotEqual(code, 0, payload)
                self.assertTrue(any(marker in item for item in payload["blockers"]), payload)

        self.write_mapping(
            status="done", workflow_phase="post_merge_done",
            pr_url="https://github.com/fixture/repo/pull/7", pr_number=7,
        )
        self.install_terminal_proof("post_merge_done", producer_shaped_merge=True)
        merge_path = self.root / ".git/oasis7-workflow-receipts" / UID / "merge-receipt.json"
        merge = json.loads(merge_path.read_text())
        merge["observed_at"] = "2026-08-30T00:00:01Z"
        merge_path.write_text(json.dumps(merge, sort_keys=True) + "\n")
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("merge digest mismatch" in item for item in payload["blockers"]), payload)

        self.write_mapping(
            status="done", workflow_phase="post_merge_done",
            pr_url="https://github.com/fixture/repo/pull/7", pr_number=7,
        )
        self.install_terminal_proof("post_merge_done", producer_shaped_merge=True)
        sync_path = self.root / ".git/oasis7-workflow-receipts" / UID / "main-sync-receipt.json"
        sync = json.loads(sync_path.read_text())
        sync["task_uid"] = "task_22222222222222222222222222222222"
        sync_path.write_text(json.dumps(sync, sort_keys=True) + "\n")
        mapping = json.loads(self.mapping.read_text())
        mapping["tasks"][UID]["phase_receipts"]["main_sync"] = sync
        mapping["tasks"][UID]["phase_receipt_sha256"]["main_sync"] = hashlib.sha256(
            sync_path.read_bytes()
        ).hexdigest()
        self.mapping.write_text(json.dumps(mapping))
        code, payload = self.run_query()
        self.assertNotEqual(code, 0, payload)
        self.assertTrue(any("main-sync receipt task/repository identity drift" in item
                            for item in payload["blockers"]), payload)


if __name__ == "__main__":
    unittest.main()
