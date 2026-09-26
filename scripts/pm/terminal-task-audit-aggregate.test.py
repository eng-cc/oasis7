#!/usr/bin/env python3
"""Focused aggregate terminal-audit and resume-finalizer contract tests."""
from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import pathlib
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from types import SimpleNamespace
from unittest.mock import patch


ROOT = pathlib.Path(__file__).resolve().parents[2]
UID = "task_11111111111111111111111111111111"
REPOSITORY = "eng-cc/oasis7"
ISSUE_NUMBER = 11
PLAN_MARKER = "<!-- oasis7-aggregate-delivery-plan/v1 -->"


def load_module(path: pathlib.Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


AUDIT = load_module(ROOT / "scripts/pm/terminal-task-audit.py", "terminal_task_audit_aggregate")
AGGREGATE = load_module(ROOT / "scripts/pm/aggregate-task-completion.py", "aggregate_task_completion_for_audit")


def raw_digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def json_bytes(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


class AggregateTerminalAudit(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name) / "repo"
        self.root.mkdir()
        self.mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        self.mapping_path.parent.mkdir(parents=True)
        self.receipt_root = pathlib.Path(self.temp.name) / "durable" / UID
        self.receipt_root.mkdir(parents=True)
        self.input_root = pathlib.Path(self.temp.name) / "inputs"
        self.input_root.mkdir()
        self.plan_path, self.candidate_path, self.evidence_path, self.completion_path = self._write_inputs()
        self.terminal_path, self.journal_path, self.issue, self.project_item = self._write_terminal_state()

    def _write_inputs(self):
        selection = {
            "candidate_sha256": "0" * 64,
            "evidence_sha256": "0" * 64,
            "integration_base_oid": "a" * 40,
            "tested_tree_oid": "b" * 40,
            "configuration_digest": "c" * 64,
            "entry": "server",
            "environment": "test",
            "evidence_window": {
                "started_at": "2026-09-25T12:00:00Z",
                "ended_at": "2026-09-25T12:10:00Z",
            },
        }
        candidate = {
            "change_id": "change-aggregate-audit",
            "integration_base_oid": selection["integration_base_oid"],
            "tested_tree_oid": selection["tested_tree_oid"],
            "configuration_digest": selection["configuration_digest"],
            "entry": selection["entry"],
            "environment": selection["environment"],
            "evidence_window": selection["evidence_window"],
        }
        evidence = [{"kind": "integration-test", "result": "passed"}]
        selection["candidate_sha256"] = AGGREGATE.canonical_digest(candidate)
        selection["evidence_sha256"] = AGGREGATE.canonical_digest(evidence)
        plan = {
            "schema": AGGREGATE.PLAN_SCHEMA,
            "task_uid": UID,
            "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER,
            "change_id": "change-aggregate-audit",
            "required_deliveries": [
                {
                    "ordinal": 1, "obligation_id": "delivery-one",
                    "task_uid": "task_22222222222222222222222222222222",
                    "issue_number": 21, "pr_number": 31,
                    "pr_url": "https://github.com/eng-cc/oasis7/pull/31", "depends_on": [],
                },
                {
                    "ordinal": 2, "obligation_id": "delivery-two",
                    "task_uid": "task_33333333333333333333333333333333",
                    "issue_number": 22, "pr_number": 32,
                    "pr_url": "https://github.com/eng-cc/oasis7/pull/32", "depends_on": ["delivery-one"],
                },
            ],
            "candidate_selection": selection,
        }
        AGGREGATE.validate_plan(plan, UID)
        plan_body = PLAN_MARKER + "\n" + json_bytes(plan).decode("utf-8")
        comment_sha = "sha256:" + raw_digest(plan_body.encode("utf-8"))
        rows = []
        for index, delivery in enumerate(plan["required_deliveries"]):
            rows.append({
                **delivery,
                "merge_commit_oid": "d" * 40,
                "head_oid": "e" * 40,
                "base_ref": "main",
                "merged_at": (
                    "2026-09-25T12:30:00Z" if index == 0 else "2026-09-25T12:31:00Z"
                ),
                "task_complete_claim_sha256": "sha256:" + "1" * 64,
                "merge_receipt_sha256": "sha256:" + "2" * 64,
                "main_sync_receipt_sha256": "sha256:" + "3" * 64,
                "terminal_receipt_sha256": "sha256:" + "4" * 64,
                "terminal_tombstone_sha256": "sha256:" + "5" * 64,
            })
        receipt = {
            "schema": AGGREGATE.RECEIPT_SCHEMA,
            "receipt_type": AGGREGATE.RECEIPT_TYPE,
            "issuer": "github_live_query",
            "evidence_mode": "production",
            "status": "verified",
            "claim_type": "task_complete",
            "task_uid": UID,
            "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER,
            "plan_comment_id": 6001,
            "plan_body_sha256": comment_sha,
            "plan_sha256": AGGREGATE.canonical_digest(plan, prefix=True),
            "candidate_sha256": selection["candidate_sha256"],
            "evidence_sha256": selection["evidence_sha256"],
            "candidate_selection": selection,
            "effective_validator_commit": "f" * 40,
            "observed_at": "2026-09-25T12:40:00Z",
            "deliveries": rows,
        }
        receipt["receipt_sha256"] = AGGREGATE.canonical_digest(receipt, prefix=True)
        AGGREGATE.validate_receipt(
            receipt,
            now=AGGREGATE._timestamp(receipt["observed_at"], "receipt observed_at"),
        )
        self.plan = plan
        self.candidate = candidate
        self.evidence = evidence
        self.receipt = receipt
        self.comment_sha = comment_sha
        paths = (
            self.input_root / "plan.json", self.input_root / "candidate.json",
            self.input_root / "evidence.json", self.input_root / "aggregate-receipt.json",
        )
        for path, value in zip(paths, (plan, candidate, evidence, receipt)):
            path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        self.input_paths = paths
        return paths

    def _write_terminal_state(self):
        plan, candidate, evidence, completion = self.input_paths
        completion_sha = raw_digest(completion.read_bytes())
        payload = {
            "schema": "oasis7.aggregate-terminal/v1",
            "receipt_type": "oasis7_aggregate_terminal",
            "issuer": "aggregate-task-finalizer",
            "task_uid": UID,
            "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER,
            "aggregate_completion_receipt_sha256": completion_sha,
            "plan_comment_id": self.receipt["plan_comment_id"],
            "observed_at": "2026-09-25T12:41:00Z",
        }
        terminal = {**payload, "receipt_sha256": raw_digest(json_bytes(payload))}
        terminal_path = self.receipt_root / "aggregate-terminal-receipt.json"
        terminal_path.write_text(json.dumps(terminal, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        terminal_sha = raw_digest(terminal_path.read_bytes())
        journal = {
            "schema": "oasis7.aggregate-terminal-effects/v1",
            "task_uid": UID,
            "terminal_receipt_sha256": terminal_sha,
            "phase_readback": True,
            "project_readback": True,
            "issue_closed_readback": True,
        }
        journal_path = self.receipt_root / "aggregate-terminal-effects.json"
        journal_path.write_text(json.dumps(journal, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        record = {
            "task_uid": UID,
            "status": "done",
            "workflow_phase": "post_merge_done",
            "completion_mode": "ordered_delivery_aggregate",
            "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER,
            "project_item_id": "ITEM1",
            "aggregate_plan_comment_id": str(self.receipt["plan_comment_id"]),
            "aggregate_plan_sha256": self.comment_sha,
            "aggregate_completion_receipt": self.receipt,
            "aggregate_completion_receipt_sha256": completion_sha,
            "phase_receipts": {"post_merge_done": terminal},
            "phase_receipt_sha256": {"post_merge_done": terminal_sha},
        }
        mapping = {
            "project": {"owner": "fixture-owner", "number": 7, "repo": REPOSITORY},
            "tasks": {UID: record},
        }
        self.mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        issue_body = "\n".join((
            f"task_uid: {UID}",
            "- status: `done`",
            "- workflow_phase: `task_done`",
            f"- completion_mode: `ordered_delivery_aggregate`",
            f"- aggregate_plan_comment_id: `{self.receipt['plan_comment_id']}`",
            f"- aggregate_plan_sha256: `{self.comment_sha}`",
            f"- aggregate_completion_receipt_sha256: `{completion_sha}`",
        ))
        issue = {
            "number": ISSUE_NUMBER,
            "url": f"https://github.com/{REPOSITORY}/issues/{ISSUE_NUMBER}",
            "state": "CLOSED", "body": issue_body, "projectItems": [],
        }
        comment = {
            "id": self.receipt["plan_comment_id"],
            "body": PLAN_MARKER + "\n" + json_bytes(self.plan).decode("utf-8"),
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE_NUMBER}",
        }
        project_item = {
            "id": "ITEM1",
            "_project_number": 7,
            "_project_owner": "fixture-owner",
            "content": {
                "body": issue_body,
                "number": ISSUE_NUMBER,
                "url": f"https://github.com/{REPOSITORY}/issues/{ISSUE_NUMBER}",
            },
            "Status": "Done",
            "PM Status": "done",
            "Workflow Phase": "done",
            "_field_values_has_next_page": False,
        }
        self.comment = comment
        return terminal_path, journal_path, issue, project_item

    def _run_audit(self, *, issue=None, project_item=None):
        issue = self.issue if issue is None else issue
        project_item = self.project_item if project_item is None else project_item
        def fake_run(command, **kwargs):
            self.assertIn("canonical-receipt-root.py", command[1])
            return SimpleNamespace(returncode=0, stdout=f"{self.receipt_root}\n", stderr="")
        with (
            patch.object(
                AUDIT, "run_json",
                side_effect=lambda command: self.comment if command[:2] == ["gh", "api"] else issue,
            ),
            patch.object(AUDIT, "registered_worktrees", return_value={}),
            patch.object(AUDIT, "_read_project_items", return_value={"ITEM1": project_item}),
            patch.object(AUDIT, "_load_aggregate_module", return_value=AGGREGATE),
            patch.object(AUDIT.subprocess, "run", side_effect=fake_run),
        ):
            return AUDIT.audit(
                self.root, UID,
                aggregate_plan=self.input_paths[0], aggregate_candidate=self.input_paths[1],
                aggregate_evidence=self.input_paths[2], aggregate_receipt=self.input_paths[3],
            )

    def test_reconciles_aggregate_terminal_without_single_pr_or_cleanup_proof(self):
        # Child revalidation performs its own live GitHub reads; this fixture
        # isolates the terminal-audit contract, while the adjacent tests cover
        # the revalidator invocation and fail-closed behavior.
        with patch.object(AGGREGATE, "validate_terminal_receipt", return_value=self.receipt):
            report = self._run_audit()
        self.assertEqual(report["status"], "reconciled", report)
        for check in (
            "aggregate_completion_receipt_valid", "aggregate_terminal_receipt_valid",
            "aggregate_child_proofs_live", "aggregate_effect_journal_valid", "issue_closed", "project_item_identity",
            "project_field_values_complete", "project_terminal",
        ):
            self.assertTrue(report["checks"][check], report)
        self.assertNotIn("pr_merged", report["checks"])
        self.assertNotIn("terminal_tombstone_valid", report["checks"])

    def test_closed_aggregate_audit_revalidates_every_child_proof(self):
        with patch.object(AGGREGATE, "validate_terminal_receipt", return_value=None) as validate:
            report = self._run_audit()

        self.assertTrue(report["checks"].get("aggregate_child_proofs_live"), report)
        validate.assert_called_once_with(
            self.root, UID, self.plan, self.candidate, self.evidence, self.receipt,
        )

    def test_closed_aggregate_audit_fails_closed_when_child_proof_drifted(self):
        with patch.object(
            AGGREGATE, "validate_terminal_receipt",
            side_effect=AGGREGATE.ReceiptError("child PR terminal proof no longer reconciles"),
        ):
            report = self._run_audit()

        self.assertEqual("drifted", report["status"], report)
        self.assertFalse(report["checks"].get("aggregate_child_proofs_live"), report)
        self.assertIn("aggregate_child_proofs_live", report["drift"])

    def test_closed_aggregate_audit_rejects_live_coordinator_lifecycle_drift(self):
        cases = (
            ("status drift", "- status: `done`", "- status: `committed`"),
            ("phase drift", "- workflow_phase: `task_done`", "- workflow_phase: `execution`"),
        )
        for name, old, new in cases:
            with self.subTest(name=name):
                issue = {**self.issue, "body": self.issue["body"].replace(old, new)}
                with patch.object(AGGREGATE, "validate_terminal_receipt", return_value=None):
                    report = self._run_audit(issue=issue)
                self.assertEqual("drifted", report["status"], report)
                self.assertIs(
                    False, report["checks"].get("aggregate_coordinator_lifecycle"), report,
                )
                self.assertIn("aggregate_coordinator_lifecycle", report["drift"])

    def test_rejects_typed_receipt_journal_and_live_identity_drift(self):
        terminal = json.loads(self.terminal_path.read_text(encoding="utf-8"))
        terminal["receipt_type"] = "oasis7_terminal_cleanup"
        self.terminal_path.write_text(json.dumps(terminal), encoding="utf-8")
        report = self._run_audit()
        self.assertFalse(report["checks"]["aggregate_terminal_receipt_valid"], report)

        self._write_terminal_state()
        journal = json.loads(self.journal_path.read_text(encoding="utf-8"))
        journal["issue_closed_readback"] = False
        self.journal_path.write_text(json.dumps(journal), encoding="utf-8")
        report = self._run_audit()
        self.assertFalse(report["checks"]["aggregate_effect_journal_valid"], report)

        self._write_terminal_state()
        bad_issue = {**self.issue, "state": "OPEN"}
        report = self._run_audit(issue=bad_issue)
        self.assertFalse(report["checks"]["issue_closed"], report)
        bad_project = {**self.project_item, "Status": "In Progress"}
        report = self._run_audit(project_item=bad_project)
        self.assertFalse(report["checks"]["project_terminal"], report)

    def test_rejects_duplicate_singular_pr_bindings_on_aggregate_issue(self):
        issue = {
            **self.issue,
            "body": self.issue["body"] + "\npr_number: 31\npr_number: 32\n",
        }
        report = self._run_audit(issue=issue)
        self.assertFalse(report["checks"]["aggregate_plan_live"], report)
        self.assertFalse(report["checks"]["issue_closed"], report)

    def test_rejects_malformed_duplicate_route_fields_on_closed_aggregate_issue(self):
        malformed_fields = (
            ("completion mode", "- completion_mode : `pr_task`"),
            ("aggregate plan pointer", "- aggregate_plan_comment_id : `9999`"),
            ("singular PR number", "- pr_number : `9999`"),
            ("singular PR URL", "- pr_url : `https://github.com/eng-cc/oasis7/pull/9999`"),
        )
        for label, malformed_field in malformed_fields:
            with self.subTest(field=label):
                issue = {
                    **self.issue,
                    "body": self.issue["body"] + "\n" + malformed_field + "\n",
                }
                # Keep the child replay valid so this specifically exercises
                # the terminal auditor's own live coordinator field parsing.
                with patch.object(AGGREGATE, "validate_terminal_receipt", return_value=None):
                    report = self._run_audit(issue=issue)
                self.assertEqual("drifted", report["status"], report)
                self.assertFalse(report["checks"]["aggregate_plan_live"], report)

    def test_aggregate_audit_requires_the_exact_four_input_files(self):
        with patch.object(
            AUDIT.subprocess, "run",
            return_value=SimpleNamespace(returncode=0, stdout=f"{self.receipt_root}\n", stderr=""),
        ):
            with self.assertRaisesRegex(SystemExit, "aggregate plan, candidate, evidence and receipt"):
                AUDIT.audit(self.root, UID)

    def test_resume_uses_the_route_specific_finalizer_and_exact_inputs(self):
        drifted = {
            "status": "drifted",
            "checks": {"completion_route_identity": True},
            "task": {"completion_mode": "ordered_delivery_aggregate"},
        }
        reconciled = {
            "status": "reconciled",
            "checks": {"completion_route_identity": True},
            "task": {"completion_mode": "ordered_delivery_aggregate"},
        }
        with (
            patch.object(AUDIT.subprocess, "check_output", return_value=f"{self.root}\n"),
            patch.object(AUDIT, "audit", side_effect=(drifted, reconciled)) as audit_call,
            patch.object(AUDIT.subprocess, "run", return_value=SimpleNamespace(returncode=0, stdout="", stderr="")) as run,
            redirect_stdout(io.StringIO()),
        ):
            result = AUDIT.main([
                "--repo-root", str(self.root), "--task-uid", UID, "--resume-finalizer",
                "--aggregate-plan", str(self.input_paths[0]),
                "--aggregate-candidate", str(self.input_paths[1]),
                "--aggregate-evidence", str(self.input_paths[2]),
                "--aggregate-receipt", str(self.input_paths[3]), "--json",
            ])
        self.assertEqual(result, 0)
        self.assertEqual(audit_call.call_count, 2)
        command = run.call_args.args[0]
        self.assertEqual(command[1], str((self.root / "scripts/pm/finalize-aggregate-task.py").resolve()))
        self.assertEqual(command[command.index("--record") + 1], str(self.input_paths[0].resolve()))
        self.assertEqual(command[command.index("--candidate") + 1], str(self.input_paths[1].resolve()))
        self.assertEqual(command[command.index("--evidence") + 1], str(self.input_paths[2].resolve()))
        self.assertEqual(command[command.index("--receipt") + 1], str(self.input_paths[3].resolve()))
        self.assertNotIn("--pr", command)

    def test_resume_refuses_absent_or_false_live_completion_route(self):
        args = [
            "--repo-root", str(self.root), "--task-uid", UID, "--resume-finalizer",
            "--aggregate-plan", str(self.input_paths[0]),
            "--aggregate-candidate", str(self.input_paths[1]),
            "--aggregate-evidence", str(self.input_paths[2]),
            "--aggregate-receipt", str(self.input_paths[3]), "--json",
        ]
        for checks in ({}, {"completion_route_identity": False}):
            with self.subTest(checks=checks):
                drifted = {
                    "status": "drifted",
                    "checks": checks,
                    "task": {"completion_mode": "ordered_delivery_aggregate"},
                }
                with (
                    patch.object(AUDIT.subprocess, "check_output", return_value=f"{self.root}\n"),
                    patch.object(AUDIT, "audit", return_value=drifted),
                    patch.object(AUDIT.subprocess, "run") as run,
                    redirect_stdout(io.StringIO()),
                ):
                    with self.assertRaisesRegex(
                        SystemExit, "refusing resume without a verified live completion route",
                    ):
                        AUDIT.main(args)
                run.assert_not_called()


if __name__ == "__main__":
    unittest.main(verbosity=2)
