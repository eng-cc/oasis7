#!/usr/bin/env python3
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


SOURCE = Path(__file__).with_name("subagent-task-packet.py")
SNAPSHOT_HELPER = Path(__file__).with_name("bootstrap-task-snapshot.py")
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
        shutil.copy2(SNAPSHOT_HELPER, self.repo / "scripts/pm/bootstrap-task-snapshot.py")
        for path in ("AGENTS.md", "doc/engineering/workflow/source-of-truth.md", ".agents/roles/qa_engineer.md", "scope.txt"):
            (self.repo / path).write_text(path + "\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "base"], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.repo), "switch", "-c", "task/packet"], check=True, capture_output=True)
        self.write_mapping()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def write_mapping(self, **changes: object) -> None:
        task = {"task_uid": TASK_UID, "canonical_worktree": str(self.repo.resolve()), "task_branch": "task/packet", "default_branch": "main", "owner_role": "qa_engineer", "issue_number": 1, "issue_url": "https://example.invalid/issues/1", "repository": "example/repo", "project_item_id": "PVTI_test", "status": "committed", "title": "review dispatch admission", "acceptance": ["reject stale review admission"]}
        task.update(changes)
        path = self.repo / ".pm/github-project-sync/tasks.json"
        path.write_text(json.dumps({"version": 1, "project": {"owner": "example", "number": 1}, "tasks": {TASK_UID: task}}), encoding="utf-8")

    def command(self, *extra: str) -> list[str]:
        return ["python3", "scripts/pm/subagent-task-packet.py", *extra]

    def create_args(self) -> list[str]:
        return ["create", "--task-uid", TASK_UID, "--slice-id", "qa-review", "--role", "qa_engineer", "--slice-type", "review", "--owner-role", "qa_engineer", "--integration-owner", "tpm", "--integration-order", "1/1", "--packet-producer", "tpm", "--context-delivery-mode", "minimal_head_bound_task_packet", "--intended-model-configuration", "inherit current parent selection", "--actual-dispatched-model-reasoning", "inherited/unverified", "--actual-runtime-evidence-reason", "dispatch surface does not report inherited runtime", "--role-activation", "message_assigned_adapter_inactive", "--base", "main", "--user-intent", "review packet behavior", "--work-item", "validate the bounded helper", "--non-goals", "no product changes", "--acceptance-target", "focused tests pass", "--governance-ref", "AGENTS.md", "--governance-ref", "doc/engineering/workflow/source-of-truth.md", "--governance-ref", ".agents/roles/qa_engineer.md", "--scoped-ref", "scope.txt", "--evidence-summary", "scope.txt is the only task surface", "--collaboration-boundary", "read only except assigned files", "--write-scope", "scripts/pm/**", "--return-contract", "patch and test evidence", "--validation-command", "python3 scripts/pm/subagent-task-packet.test.py", "--formal-sink", "https://example.invalid/issues/1"]

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
        self.assertEqual("inherited/unverified", packet["slice"]["actual_dispatched_model_reasoning"])
        self.assertEqual("message_assigned_adapter_inactive", packet["slice"]["role_activation"])
        self.assertNotIn("embedded_docs", packet)
        self.invoke(["validate", packet_path])

    def test_missing_mandatory_fields_fail(self) -> None:
        args = self.create_args()
        index = args.index("--work-item")
        del args[index:index + 2]
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("--work-item", result.stderr)

        for option in ("--packet-producer", "--context-delivery-mode", "--integration-order", "--intended-model-configuration", "--actual-dispatched-model-reasoning", "--actual-runtime-evidence-reason", "--role-activation"):
            args = self.create_args()
            index = args.index(option)
            del args[index:index + 2]
            result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
            self.assertNotEqual(0, result.returncode)
            self.assertIn(option, result.stderr)

        args = self.create_args()
        args[args.index("message_assigned_adapter_inactive")] = "unsupported_activation"
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("invalid choice", result.stderr)

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

    def create_snapshot(self) -> Path:
        snapshot = self.repo / ".pm/scratch" / TASK_UID / "bootstrap-task-snapshot.json"
        result = subprocess.run(
            ["python3", "scripts/pm/bootstrap-task-snapshot.py", "create",
             "--repo-root", str(self.repo), "--task-uid", TASK_UID,
             "--request-identity", "review dispatch admission", "--producer", "tpm"],
            cwd=self.repo, text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        return snapshot

    def create_review_plan(self, packet_path: str, **changes: object) -> Path:
        base_sha = self.git("rev-parse", "main")
        head = self.git("rev-parse", "HEAD")
        plan = {
            "schema": "oasis7-review-plan/v1", "task_uid": TASK_UID,
            "frozen_head": head, "comparison_ref": "main", "comparison_oid": base_sha,
            "expected_slices": [{"role": "qa_engineer", "slice_id": "qa-review"}],
            "packet_refs": [{"role": "qa_engineer", "slice_id": "qa-review", "packet_ref": packet_path}],
        }
        plan.update(changes)
        path = self.repo / ".pm/scratch" / TASK_UID / "review-plans" / "admission.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(plan), encoding="utf-8")
        return path

    def git(self, *args: str) -> str:
        return subprocess.check_output(["git", "-C", str(self.repo), *args], text=True).strip()

    def review_admission(self, packet: str, plan: Path, snapshot: Path, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            self.command("review-admission", "--packet", packet, "--review-plan", str(plan), "--bootstrap-snapshot", str(snapshot)),
            cwd=self.repo, text=True, capture_output=True,
        )
        if ok:
            self.assertEqual(0, result.returncode, result.stderr)
        else:
            self.assertNotEqual(0, result.returncode, result.stdout)
        return result

    def test_review_admission_requires_current_cross_bound_packet_plan_and_snapshot(self) -> None:
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        plan = self.create_review_plan(packet)
        admitted = self.review_admission(packet, plan, snapshot)
        self.assertEqual("admitted", json.loads(admitted.stdout)["status"])

        cases = {
            "plan task mismatch": {"task_uid": "task_22222222222222222222222222222222"},
            "plan head mismatch": {"frozen_head": "a" * 40},
            "plan role mismatch": {"expected_slices": [{"role": "runtime_engineer", "slice_id": "qa-review"}]},
            "plan slice mismatch": {"expected_slices": [{"role": "qa_engineer", "slice_id": "other-slice"}]},
            "plan packet ref mismatch": {"packet_refs": [{"role": "qa_engineer", "slice_id": "qa-review", "packet_ref": "scope.txt"}]},
            "comparison oid mismatch": {"comparison_oid": "b" * 40},
        }
        for name, changes in cases.items():
            with self.subTest(name=name):
                bad_plan = self.create_review_plan(packet, **changes)
                self.review_admission(packet, bad_plan, snapshot, ok=False)

        plan = self.create_review_plan(packet)
        payload = json.loads(snapshot.read_text(encoding="utf-8"))
        payload["producer"] = "tampered"
        snapshot.write_text(json.dumps(payload), encoding="utf-8")
        self.review_admission(packet, plan, snapshot, ok=False)

    def test_review_admission_invalidates_after_head_or_comparison_ref_changes(self) -> None:
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        plan = self.create_review_plan(packet)
        self.review_admission(packet, plan, snapshot)

        original_base = self.git("rev-parse", "main")
        moved_base = self.git("commit-tree", "HEAD^{tree}", "-p", original_base, "-m", "moved comparison")
        self.git("update-ref", "main", moved_base)
        self.review_admission(packet, plan, snapshot, ok=False)
        self.git("update-ref", "main", original_base)

        (self.repo / "advance.txt").write_text("advance\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "advance.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "advance"], check=True, capture_output=True)
        self.review_admission(packet, plan, snapshot, ok=False)


if __name__ == "__main__":
    unittest.main()
