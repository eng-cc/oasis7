#!/usr/bin/env python3
"""Adversarial entry-point tests for ordered aggregate closeout and finalization."""
from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
import json
import pathlib
import sys
import tempfile
import unittest
from argparse import Namespace
from types import SimpleNamespace
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
TASK_PATH = ROOT / "scripts/pm/github-project-task.py"
FINALIZER_PATH = ROOT / "scripts/pm/finalize-aggregate-task.py"
UID = "task_" + "f" * 32
REPO = "eng-cc/oasis7"


def load_module(path: pathlib.Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def plan_for(uid: str = UID, deliveries: list[dict] | None = None) -> dict:
    candidate = {
        "change_id": "change-4035", "integration_base_oid": "1" * 40,
        "tested_tree_oid": "2" * 40, "configuration_digest": "sha256:" + "3" * 64,
        "entry": "aggregate-required", "environment": "linux-x86_64",
        "evidence_window": {"started_at": "2026-09-25T08:00:00Z", "ended_at": "2026-09-25T09:00:00Z"},
    }
    evidence = [{"obligation_id": "delivery-a", "status": "passed", "profile": "repository_required"}]
    return {
        "schema": "oasis7.aggregate-delivery-plan/v1", "task_uid": uid,
        "repository": REPO, "issue_number": 4035, "change_id": "change-4035",
        "required_deliveries": deliveries if deliveries is not None else [
            {"ordinal": 1, "obligation_id": "delivery-a", "task_uid": "task_" + "a" * 32,
             "issue_number": 4101, "pr_number": 5101,
             "pr_url": "https://github.com/eng-cc/oasis7/pull/5101", "depends_on": []},
            {"ordinal": 2, "obligation_id": "delivery-b", "task_uid": "task_" + "b" * 32,
             "issue_number": 4102, "pr_number": 5102,
             "pr_url": "https://github.com/eng-cc/oasis7/pull/5102", "depends_on": ["delivery-a"]},
        ],
        "candidate_selection": {
            "candidate_sha256": hashlib.sha256(canonical_bytes(candidate)).hexdigest(),
            "evidence_sha256": hashlib.sha256(canonical_bytes(evidence)).hexdigest(),
            **{key: candidate[key] for key in (
                "integration_base_oid", "tested_tree_oid", "configuration_digest",
                "entry", "environment", "evidence_window",
            )},
        },
    }


class OrderedAggregateCloseoutTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.task = load_module(TASK_PATH, "aggregate_closeout_task")
        cls.finalizer = load_module(FINALIZER_PATH, "aggregate_closeout_finalizer")

    def invoke_binder(self, *, permission="admin", deliveries=None, promoted_pr=None):
        plan = plan_for(deliveries=deliveries)
        marker = "<!-- oasis7-aggregate-delivery-plan/v1 -->\n"
        body = marker + canonical_bytes(plan).decode()
        with tempfile.TemporaryDirectory() as temp:
            plan_path = pathlib.Path(temp) / "plan.json"
            plan_path.write_text(json.dumps(plan), encoding="utf-8")
            record = {"task_uid": UID, "issue_number": 4035, "repository": REPO,
                      "completion_mode": "", "status": "committed", "workflow_phase": ""}
            issue = {"state": "open", "body": f"<!-- oasis7-pm-task -->\ntask_uid: {UID}\n"}
            events: list[tuple] = []
            writes: list[str] = []

            def run_text(command):
                if command[1:3] == ["api", f"repos/{REPO}/issues/comments/6001"]:
                    events.append(("comment_read",))
                    return json.dumps({"id": 6001, "body": body,
                                       "issue_url": f"https://api.github.com/repos/{REPO}/issues/4035",
                                       "user": {"login": "plan-author"}})
                if command[1:3] == ["api", f"repos/{REPO}/collaborators/plan-author/permission"]:
                    events.append(("permission_read",))
                    return json.dumps({"permission": permission})
                if command[1:3] == ["api", f"repos/{REPO}/issues/4035"]:
                    events.append(("issue_read",))
                    return json.dumps(issue)
                if command[1:3] == ["pr", "view"]:
                    number = int(command[3])
                    events.append(("pr_read", number))
                    is_promoted = promoted_pr == number
                    return json.dumps({"state": "OPEN", "isDraft": not is_promoted,
                                       "number": number,
                                       "url": f"https://github.com/{REPO}/pull/{number}"})
                raise AssertionError(f"unexpected command: {command}")

            def update_issue_body(repo, number, task):
                writes.append("issue")
                events.append(("issue_write",))
                issue["body"] = self.task.issue_body(task)

            def merge_mapping(*args, **kwargs):
                writes.append("mapping")
                events.append(("mapping_write",))

            args = Namespace(root=ROOT, repo=REPO, mapping=".pm/tasks.json", task_uid=UID,
                             plan=str(plan_path), comment_id=6001)
            with mock.patch.object(self.task, "require_record", return_value=(pathlib.Path(temp) / "tasks.json", {}, record)), \
                    mock.patch.object(self.task, "run_text", side_effect=run_text), \
                    mock.patch.object(self.task, "update_issue_body", side_effect=update_issue_body), \
                    mock.patch.object(self.task, "merge_task_mapping", side_effect=merge_mapping), \
                    contextlib.redirect_stdout(io.StringIO()):
                try:
                    self.task.command_bind_aggregate_plan(args)
                    outcome = None
                except SystemExit as exc:
                    outcome = str(exc)
            return outcome, events, writes

    def test_binder_requires_admin_and_reads_every_open_draft_before_writing(self):
        outcome, events, writes = self.invoke_binder()
        self.assertIsNone(outcome)
        self.assertEqual(writes, ["issue", "mapping"])
        self.assertEqual([name for name, *_ in events if name == "pr_read"], ["pr_read", "pr_read"])
        self.assertGreater(events.index(("issue_write",)), max(i for i, e in enumerate(events) if e[0] == "pr_read"))

        for permission in ("write", "maintain"):
            with self.subTest(permission=permission):
                outcome, events, writes = self.invoke_binder(permission=permission)
                self.assertIn("lacks repository admin authority", outcome or "")
                self.assertEqual(writes, [])
                self.assertFalse(any(event[0] == "pr_read" for event in events))

    def test_binder_fails_closed_for_empty_deliveries_and_promoted_child(self):
        outcome, _events, writes = self.invoke_binder(deliveries=[])
        self.assertIn("at least two required deliveries", outcome or "")
        self.assertEqual(writes, [])

        outcome, events, writes = self.invoke_binder(promoted_pr=5102)
        self.assertIn("all required PRs must be live drafts", outcome or "")
        self.assertEqual(writes, [])
        self.assertEqual([event for event in events if event[0] == "pr_read"],
                         [("pr_read", 5101), ("pr_read", 5102)])

    def test_aggregate_done_without_receipt_fails_before_effects(self):
        record = {"task_uid": UID, "status": "committed", "completion_mode": "ordered_delivery_aggregate"}
        args = Namespace(task_uid=UID, to_status="done", claim_json=json.dumps({
            "status": "verified", "allowed_to_claim": True,
        }), aggregate_receipt=None)
        side_effects = ["comment", "project", "issue", "mapping"]
        with mock.patch.object(self.task, "require_record", return_value=(pathlib.Path("tasks.json"), {}, record)), \
                mock.patch.object(self.task, "issue_comment", side_effect=lambda *a, **k: side_effects.pop(0)), \
                mock.patch.object(self.task, "update_done_project_fields", side_effect=lambda *a, **k: side_effects.pop(0)), \
                mock.patch.object(self.task, "update_issue_body", side_effect=lambda *a, **k: side_effects.pop(0)), \
                mock.patch.object(self.task, "merge_task_mapping", side_effect=lambda *a, **k: side_effects.pop(0)):
            with self.assertRaisesRegex(SystemExit, "ordered aggregate completion requires an aggregate receipt"):
                self.task.command_closeout_task(args)
        self.assertEqual(side_effects, ["comment", "project", "issue", "mapping"])

    def test_aggregate_terminal_phase_missing_exact_proofs_fails_before_effects(self):
        record = {"task_uid": UID, "issue_number": 4035, "status": "done",
                  "workflow_phase": "task_done", "completion_mode": "ordered_delivery_aggregate",
                  "aggregate_completion_receipt_sha256": "a" * 64}
        with tempfile.TemporaryDirectory() as temp:
            terminal = pathlib.Path(temp) / "terminal.json"
            terminal.write_text(json.dumps({"receipt_type": "oasis7_aggregate_terminal",
                                            "issuer": "aggregate-task-finalizer", "task_uid": UID,
                                            "aggregate_completion_receipt_sha256": "a" * 64}), encoding="utf-8")
            args = Namespace(task_uid=UID, phase="post_merge_done", receipt_json=str(terminal),
                             root=ROOT, role="tpm", aggregate_plan=None,
                             aggregate_candidate=None, aggregate_evidence=None,
                             aggregate_receipt=None, repo=REPO)
            effects = []
            with mock.patch.object(self.task, "require_record", return_value=(pathlib.Path(temp) / "tasks.json", {}, record)), \
                    mock.patch.object(self.task, "issue_comment", side_effect=lambda *a, **k: effects.append("comment")), \
                    mock.patch.object(self.task, "update_project_fields", side_effect=lambda *a, **k: effects.append("project")), \
                    mock.patch.object(self.task, "merge_task_mapping", side_effect=lambda *a, **k: effects.append("mapping")):
                with self.assertRaisesRegex(SystemExit, "aggregate terminal requires the exact plan/candidate/evidence/completion receipt"):
                    self.task.command_set_phase(args)
            self.assertEqual(effects, [])

    def run_finalizer_retry(self, phase: str, issue_state: str):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp).resolve()
            (root / ".git").mkdir()
            (root / ".pm/github-project-sync").mkdir(parents=True)
            common = root / ".git"
            (root / "scripts/pm").mkdir(parents=True)
            for name, value in (("plan.json", {"plan": True}), ("candidate.json", {"candidate": True}),
                                ("evidence.json", [{"evidence": True}]),
                                ("completion.json", {"plan_comment_id": 6001})):
                (root / name).write_text(json.dumps(value), encoding="utf-8")
            plan_path, candidate_path, evidence_path, receipt_path = [
                root / name for name in ("plan.json", "candidate.json", "evidence.json", "completion.json")
            ]
            completion_sha = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
            issue_number = 4035
            durable = common / "oasis7-workflow-receipts" / UID
            durable.mkdir(parents=True)
            terminal_path = durable / "aggregate-terminal-receipt.json"
            if phase == "post_merge_done":
                payload = {
                    "schema": "oasis7.aggregate-terminal/v1", "receipt_type": "oasis7_aggregate_terminal",
                    "issuer": "aggregate-task-finalizer", "task_uid": UID, "repository": REPO,
                    "issue_number": issue_number, "aggregate_completion_receipt_sha256": completion_sha,
                    "plan_comment_id": 6001, "observed_at": "2026-09-25T10:00:00Z",
                }
                terminal = {**payload, "receipt_sha256": self.finalizer.digest(payload)}
                terminal_path.write_text(json.dumps(terminal, sort_keys=True, indent=2) + "\n", encoding="utf-8")
                terminal_sha = hashlib.sha256(terminal_path.read_bytes()).hexdigest()
            else:
                terminal_sha = ""
            task = {"task_uid": UID, "completion_mode": "ordered_delivery_aggregate",
                    "status": "done", "workflow_phase": phase, "repository": REPO,
                    "issue_number": issue_number, "aggregate_completion_receipt_sha256": completion_sha,
                    "phase_receipt_sha256": {"post_merge_done": terminal_sha} if terminal_sha else {}}
            mapping_path = root / ".pm/github-project-sync/tasks.json"
            mapping_path.write_text(json.dumps({"version": 1, "tasks": {UID: task}}), encoding="utf-8")
            issue = {"state": issue_state, "body": f"task_uid: {UID}\n"}
            events: list[str] = []

            def fake_command(*argv):
                argv = tuple(str(arg) for arg in argv)
                if argv[:3] == ("git", "-C", str(root)) and argv[3:5] == ("rev-parse", "--show-toplevel"):
                    return str(root) + "\n"
                if argv[:3] == ("git", "-C", str(root)) and argv[3:5] == ("rev-parse", "--git-common-dir"):
                    return str(common) + "\n"
                if argv[:4] == ("gh", "issue", "view", str(issue_number)):
                    events.append("issue_view")
                    return json.dumps(issue)
                if len(argv) > 1 and argv[1].endswith("aggregate-task-completion.py") and argv[2] == "validate":
                    events.append("aggregate_validate")
                    return "{}\n"
                if len(argv) > 2 and argv[1].endswith("github-project-task.py") and argv[2] == "set-phase":
                    events.append("set_phase")
                    receipt_file = pathlib.Path(argv[argv.index("--receipt-json") + 1])
                    current = json.loads(mapping_path.read_text())
                    mapped_task = current["tasks"][UID]
                    mapped_task["workflow_phase"] = "post_merge_done"
                    mapped_task.setdefault("phase_receipt_sha256", {})["post_merge_done"] = hashlib.sha256(receipt_file.read_bytes()).hexdigest()
                    mapping_path.write_text(json.dumps(current), encoding="utf-8")
                    return "{}\n"
                if len(argv) > 3 and argv[1].endswith("github-project-workflow.py") and argv[3] == "audit":
                    events.append("project_audit")
                    return json.dumps({"status": "ok", "selected_count": 1})
                if argv[:3] == ("gh", "issue", "close"):
                    events.append("issue_close")
                    issue["state"] = "CLOSED"
                    return "closed\n"
                raise AssertionError(f"unexpected finalizer command: {argv}")

            argv = [str(FINALIZER_PATH), "--repo-root", str(root), "--task-uid", UID,
                    "--record", str(plan_path), "--candidate", str(candidate_path),
                    "--evidence", str(evidence_path), "--receipt", str(receipt_path), "--json"]
            with mock.patch.object(self.finalizer, "command", side_effect=fake_command), \
                    mock.patch.object(sys, "argv", argv), contextlib.redirect_stdout(io.StringIO()):
                result = self.finalizer.main()
            saved_task = json.loads(mapping_path.read_text())["tasks"][UID]
            return result, events, saved_task, issue, terminal_path.is_file()

    def test_finalizer_resumes_task_done_and_post_merge_open_retries(self):
        result, events, task, issue, terminal_exists = self.run_finalizer_retry("task_done", "OPEN")
        self.assertEqual(result, 0)
        self.assertEqual(task["workflow_phase"], "post_merge_done")
        self.assertEqual(issue["state"], "CLOSED")
        self.assertLess(events.index("set_phase"), events.index("project_audit"))
        self.assertLess(events.index("project_audit"), events.index("issue_close"))
        self.assertTrue(terminal_exists)

        result, events, task, issue, _terminal = self.run_finalizer_retry("post_merge_done", "OPEN")
        self.assertEqual(result, 0)
        self.assertEqual(task["workflow_phase"], "post_merge_done")
        self.assertEqual(issue["state"], "CLOSED")
        self.assertNotIn("set_phase", events)
        self.assertIn("aggregate_validate", events)
        self.assertIn("issue_close", events)

    def test_finalizer_closed_retry_is_idempotent_without_open_only_validator(self):
        result, events, task, issue, _terminal = self.run_finalizer_retry("post_merge_done", "CLOSED")
        self.assertEqual(result, 0)
        self.assertEqual(task["workflow_phase"], "post_merge_done")
        self.assertEqual(issue["state"], "CLOSED")
        self.assertNotIn("aggregate_validate", events)
        self.assertNotIn("set_phase", events)
        self.assertNotIn("issue_close", events)
        self.assertEqual(events.count("project_audit"), 1)

    def test_single_pr_done_closeout_remains_receipt_optional(self):
        record = {"task_uid": UID, "status": "committed", "issue_number": 4035,
                  "issue_url": f"https://github.com/{REPO}/issues/4035", "project_item_id": "PVTI_1",
                  "owner_role": "tpm", "module": "engineering", "priority": "P2",
                  "worktree_hint": str(ROOT), "pr_number": 5101,
                  "pr_url": f"https://github.com/{REPO}/pull/5101", "claim_verifications": []}
        args = Namespace(task_uid=UID, to_status="done", repo=REPO, role="tpm",
                         claim_json=json.dumps({"claim_type": "task_complete", "status": "verified",
                                                "allowed_to_claim": True, "verification_exit_code": 0}),
                         pr_receipt=None, aggregate_receipt=None, aggregate_plan=None,
                         aggregate_candidate=None, aggregate_evidence=None, json=False)
        effects = []
        with mock.patch.object(self.task, "require_record", return_value=(pathlib.Path("tasks.json"), {}, record)), \
                mock.patch.object(self.task, "synchronize_live_issue_traceability", return_value=frozenset()), \
                mock.patch.object(self.task, "issue_comment", side_effect=lambda *a, **k: "comment-url"), \
                mock.patch.object(self.task, "update_done_project_fields", side_effect=lambda *a, **k: effects.append("project") or 1), \
                mock.patch.object(self.task, "update_issue_body", side_effect=lambda *a, **k: effects.append("issue")), \
                mock.patch.object(self.task, "merge_task_mapping", side_effect=lambda *a, **k: effects.append("mapping")), \
                contextlib.redirect_stdout(io.StringIO()):
            result = self.task.command_closeout_task(args)
        self.assertEqual(result, 0)
        self.assertEqual(effects, ["project", "issue", "mapping"])


if __name__ == "__main__":
    unittest.main()
