#!/usr/bin/env python3
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


SOURCE = Path(__file__).with_name("subagent-task-packet.py")
TASK_UID = "task_11111111111111111111111111111111"


class PacketTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name) / "repo"
        subprocess.run(["git", "init", "-b", "main", str(self.repo)], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.name", "Test"], check=True)
        for path in ("scripts/pm", ".pm/github-project-sync", ".agents/roles", "doc/engineering/workflow"):
            (self.repo / path).mkdir(parents=True, exist_ok=True)
        shutil.copy2(SOURCE, self.repo / "scripts/pm/subagent-task-packet.py")
        for path in ("AGENTS.md", "doc/engineering/workflow/source-of-truth.md", ".agents/roles/qa_engineer.md", "scope.txt"):
            (self.repo / path).write_text(path + "\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "base"], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.repo), "switch", "-c", "task/packet"], check=True, capture_output=True)
        self.write_mapping()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def write_mapping(self, **changes: object) -> None:
        task = {"task_uid": TASK_UID, "canonical_worktree": str(self.repo.resolve()), "task_branch": "task/packet", "default_branch": "main", "owner_role": "qa_engineer", "issue_url": "https://example.invalid/issues/1", "repository": "example/repo", "project_item_id": "PVTI_test", "status": "committed"}
        task.update(changes)
        path = self.repo / ".pm/github-project-sync/tasks.json"
        path.write_text(json.dumps({"version": 1, "tasks": {TASK_UID: task}}), encoding="utf-8")

    def command(self, *extra: str) -> list[str]:
        return ["python3", "scripts/pm/subagent-task-packet.py", *extra]

    def create_args(self) -> list[str]:
        return ["create", "--task-uid", TASK_UID, "--slice-id", "qa-review", "--role", "qa_engineer", "--slice-type", "review", "--owner-role", "qa_engineer", "--integration-owner", "tpm", "--integration-order", "1/1", "--packet-producer", "tpm", "--context-delivery-mode", "minimal_head_bound_task_packet", "--base", "main", "--user-intent", "review packet behavior", "--work-item", "validate the bounded helper", "--non-goals", "no product changes", "--acceptance-target", "focused tests pass", "--governance-ref", "AGENTS.md", "--governance-ref", "doc/engineering/workflow/source-of-truth.md", "--governance-ref", ".agents/roles/qa_engineer.md", "--scoped-ref", "scope.txt", "--evidence-summary", "scope.txt is the only task surface", "--collaboration-boundary", "read only except assigned files", "--write-scope", "scripts/pm/**", "--return-contract", "patch and test evidence", "--validation-command", "python3 scripts/pm/subagent-task-packet.test.py", "--formal-sink", "https://example.invalid/issues/1"]

    def invoke(self, args: list[str], ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertEqual(0 if ok else 1, result.returncode, result.stdout + result.stderr)
        return result

    def test_create_and_validate(self) -> None:
        result = self.invoke(self.create_args())
        packet_path = result.stdout.splitlines()[0]
        packet = json.loads((self.repo / packet_path).read_text())
        self.assertEqual(subprocess.check_output(["git", "-C", str(self.repo), "rev-parse", "HEAD"], text=True).strip(), packet["identity"]["head"])
        self.assertEqual("tpm", packet["identity"]["packet_producer"])
        self.assertEqual("minimal_head_bound_task_packet", packet["slice"]["context_delivery_mode"])
        self.assertEqual("1/1", packet["slice"]["integration_order"])
        self.assertNotIn("embedded_docs", packet)
        self.invoke(["validate", packet_path])

    def test_missing_mandatory_fields_fail(self) -> None:
        args = self.create_args()
        index = args.index("--work-item")
        del args[index:index + 2]
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("--work-item", result.stderr)

        for option in ("--packet-producer", "--context-delivery-mode", "--integration-order"):
            args = self.create_args()
            index = args.index(option)
            del args[index:index + 2]
            result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
            self.assertNotEqual(0, result.returncode)
            self.assertIn(option, result.stderr)

    def test_delivery_mode_escalation_and_digest_tamper(self) -> None:
        args = self.create_args()
        args[args.index("minimal_head_bound_task_packet")] = "full_history_escalation"
        result = self.invoke(args, ok=False)
        self.assertIn("full_history_escalation_reason", result.stderr)

        args.extend(["--full-history-escalation-reason", "prior user authority is absent from scoped evidence"])
        result = self.invoke(args)
        packet_path = result.stdout.splitlines()[0]
        packet_file = self.repo / packet_path
        packet = json.loads(packet_file.read_text())
        packet["identity"]["packet_producer"] = "tampered"
        packet_file.write_text(json.dumps(packet), encoding="utf-8")
        self.assertIn("packet digest mismatch", self.invoke(["validate", packet_path], ok=False).stderr)

    def test_wrong_task_and_worktree_fail(self) -> None:
        args = self.create_args(); args[args.index(TASK_UID)] = "task_22222222222222222222222222222222"
        self.assertIn("not present", self.invoke(args, ok=False).stderr)
        self.write_mapping(canonical_worktree=str(self.repo.parent / "wrong"))
        self.assertIn("wrong worktree", self.invoke(self.create_args(), ok=False).stderr)

    def test_overwrite_and_stale_head_fail(self) -> None:
        result = self.invoke(self.create_args())
        packet_path = result.stdout.splitlines()[0]
        self.assertIn("refusing to overwrite", self.invoke(self.create_args(), ok=False).stderr)
        (self.repo / "new.txt").write_text("new\n")
        subprocess.run(["git", "-C", str(self.repo), "add", "new.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "advance"], check=True, capture_output=True)
        self.assertIn("stale or mismatched packet head", self.invoke(["validate", packet_path], ok=False).stderr)


if __name__ == "__main__":
    unittest.main()
