#!/usr/bin/env python3
"""Focused tests for project-bound artifact readiness and live CI identity."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import types
from types import SimpleNamespace
from typing import Any
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("workflow-delivery-readiness.py")
SPEC = importlib.util.spec_from_file_location("workflow_delivery_readiness", SCRIPT)
assert SPEC and SPEC.loader
DELIVERY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DELIVERY)

TASK_UID = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
HEAD = "a" * 40
REPO = "eng-cc/oasis7"
PR = 901
RUN = 902
CHECK = 903
JOB = 9901
UPSTREAM_UID = "task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"


def complete_delivery_proof(*, review: dict | None = None) -> dict:
    locator = {
        "pr_number": 55, "head_oid": HEAD, "check_run_id": CHECK,
        "run_id": RUN, "run_attempt": 2,
    }
    return {
        "declaration": {
            "schema": DELIVERY.ARTIFACT_DEPENDENCY_SCHEMA,
            "repository": REPO,
            "task_uid": TASK_UID,
            "task_issue_number": 900,
            "upstream_task_uid": UPSTREAM_UID,
            "artifacts": [{"path": "doc/accepted.md", "sha256": "d" * 64}],
            "source_ci": locator,
        },
        "declaration_authority": {"permission": "admin", "body_valid": True},
        "downstream": {"task_uid": TASK_UID, "issue_number": 900},
        "upstream_issue_number": 901,
        "upstream_issue": {
            "task_uid": UPSTREAM_UID, "status": "done", "workflow_phase": "task_done",
            "pr_number": 55, "merge_hold_active": False,
        },
        "pr": {
            "repository": REPO, "number": 55, "issue_number": 901,
            "task_uid": UPSTREAM_UID, "state": "closed", "merged": True,
            "merge_commit_oid": "c" * 40, "head_oid": HEAD,
            "base_ref": "main", "default_branch": "main",
        },
        "review": review or {
            "task_uid": UPSTREAM_UID, "source_head_oid": HEAD,
            "passed": True, "admin_author": True,
            "findings_disposition": "no_findings",
            "roles": ["agent_engineer", "qa_engineer"],
            "finding_disposition_evidence": (
                "agent_engineer: no_findings; qa_engineer: no_findings"
            ),
        },
        "source_ci": {
            "check_name": "required-gate", "app_id": DELIVERY.REQUIRED_GATE_APP_ID,
            "check_run_id": CHECK, "run_id": RUN, "run_attempt": 2,
            "pr_number": 55, "head_oid": HEAD,
            "pr_association_verified": True,
            "status": "completed", "conclusion": "success",
        },
        "local_input": {
            "repository": REPO, "head_contains_merge_commit": True,
            "artifacts_match": True,
        },
    }


def check_row(*, check_id: int = CHECK, run_id: int = RUN, head: str = HEAD,
              pr_numbers: list[int] | None = None, status: str = "completed",
              conclusion: str | None = "success") -> dict:
    return {
        "id": check_id,
        "name": "required-gate",
        "app": {"id": DELIVERY.REQUIRED_GATE_APP_ID},
        "pull_requests": [{"number": number} for number in (pr_numbers if pr_numbers is not None else [PR])],
        "details_url": f"https://github.com/{REPO}/actions/runs/{run_id}/job/{JOB}",
        "head_sha": head,
        "status": status,
        "conclusion": conclusion,
    }


def mocked_ci(*, check: dict, run_pr_numbers: list[int] | None = None,
              run_branch: str = "feature", run_head: str = HEAD,
              run_path: str = ".github/workflows/rust.yml", job_id: int = JOB,
              job_check_id: int | None = None, job_conclusion: str = "success"):
    run_id = int(check["details_url"].split("/runs/")[1].split("/")[0])
    check_id = check["id"]
    job_check_id = check_id if job_check_id is None else job_check_id

    def gh_json(path: str) -> dict:
        if path == f"repos/{REPO}/check-runs/{check_id}":
            return check
        if path == f"repos/{REPO}/actions/runs/{run_id}":
            return {
                "id": run_id, "run_attempt": 2, "head_sha": run_head,
                "head_branch": run_branch, "path": run_path, "event": "pull_request",
                "pull_requests": [{"number": n} for n in (run_pr_numbers if run_pr_numbers is not None else [PR])],
                "status": "completed", "conclusion": "success",
            }
        raise AssertionError(path)

    def pages(path: str, key: str | None = None) -> list:
        if path == f"repos/{REPO}/actions/runs/{run_id}/jobs":
            return [{
                "id": job_id, "name": "required-gate", "run_attempt": 2,
                "check_run_url": f"https://api.github.com/repos/{REPO}/check-runs/{job_check_id}",
                "status": "completed", "conclusion": job_conclusion,
            }]
        raise AssertionError((path, key))

    return gh_json, pages


class DeclaredSourceCiTest(unittest.TestCase):
    def test_exact_ci_locator_checks_distinct_job_and_check_ids(self) -> None:
        check = check_row()
        gh_json, pages = mocked_ci(check=check, job_id=JOB, job_check_id=CHECK)
        with patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages):
            result = DELIVERY._verify_declared_ci(
                REPO,
                {"pr_number": PR, "head_oid": HEAD, "check_run_id": CHECK,
                 "run_id": RUN, "run_attempt": 2},
                PR, HEAD, "feature",
            )
        self.assertTrue(result["pr_association_verified"])
        self.assertEqual(result["check_run_id"], CHECK)
        self.assertNotEqual(JOB, CHECK)

    def test_same_sha_from_another_pr_cannot_be_selected(self) -> None:
        check = check_row(pr_numbers=[PR + 1])
        gh_json, pages = mocked_ci(check=check, run_pr_numbers=[PR + 1])
        with patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages):
            with self.assertRaisesRegex(ValueError, "PR association"):
                DELIVERY._verify_declared_ci(
                    REPO,
                    {"pr_number": PR, "head_oid": HEAD, "check_run_id": CHECK,
                     "run_id": RUN, "run_attempt": 2},
                    PR, HEAD, "feature",
                )

    def test_empty_associations_need_exact_authorized_binding(self) -> None:
        check = check_row(pr_numbers=[])
        gh_json, pages = mocked_ci(check=check, run_pr_numbers=[])
        locator = {"pr_number": PR, "head_oid": HEAD, "check_run_id": CHECK,
                   "run_id": RUN, "run_attempt": 2}
        with patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages):
            with self.assertRaisesRegex(ValueError, "PR association"):
                DELIVERY._verify_declared_ci(REPO, locator, PR, HEAD, "feature")
        with patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages):
            result = DELIVERY._verify_declared_ci(
                REPO, locator, PR, HEAD, "feature", allow_empty_pr_association=True,
            )
        self.assertEqual(result["pr_association_basis"], "authenticated_task_evidence_and_exact_run_identity")

    def test_nonempty_conflicting_association_is_never_waived(self) -> None:
        check = check_row(pr_numbers=[PR, PR + 1])
        gh_json, pages = mocked_ci(check=check, run_pr_numbers=[PR])
        with patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages):
            with self.assertRaisesRegex(ValueError, "PR association"):
                DELIVERY._verify_declared_ci(
                    REPO,
                    {"pr_number": PR, "head_oid": HEAD, "check_run_id": CHECK,
                     "run_id": RUN, "run_attempt": 2},
                    PR, HEAD, "feature", allow_empty_pr_association=True,
                )


class CurrentPrProjectionTest(unittest.TestCase):
    def setUp(self) -> None:
        self.local_head = "b" * 40
        self.task = {"task_uid": TASK_UID, "repository": REPO, "issue_number": 900, "pr_number": str(PR)}
        self.pr = {"number": PR, "body": f"Task: {TASK_UID}\nRefs #900",
                   "head": {"sha": HEAD, "ref": "feature"}}

    def project(self, checks: list[dict], *, run_prs: dict[int, list[int]] | None = None,
                run_statuses: dict[int, str] | None = None,
                run_conclusions: dict[int, str | None] | None = None,
                job_statuses: dict[int, str] | None = None,
                job_conclusions: dict[int, str | None] | None = None,
                read_error_paths: set[str] | None = None,
                process_waiver: dict[str, object] | None = None):
        run_prs = run_prs or {}
        run_statuses = run_statuses or {}
        run_conclusions = run_conclusions or {}
        job_statuses = job_statuses or {}
        job_conclusions = job_conclusions or {}
        read_error_paths = read_error_paths or set()
        runs = {int(row["details_url"].split("/runs/")[1].split("/")[0]): row for row in checks}

        def gh_json(path: str) -> dict:
            if path in read_error_paths:
                raise OSError("synthetic live check read unavailable")
            if path == f"repos/{REPO}/pulls/{PR}":
                return self.pr
            for run_id, row in runs.items():
                if path == f"repos/{REPO}/actions/runs/{run_id}":
                    return {
                        "id": run_id, "run_attempt": 2, "head_sha": HEAD,
                        "head_branch": "feature", "path": ".github/workflows/rust.yml",
                        "event": "pull_request",
                        "pull_requests": [{"number": n} for n in run_prs.get(run_id, [PR])],
                        "status": run_statuses.get(run_id, "completed"),
                        "conclusion": run_conclusions.get(run_id, "success"),
                    }
            raise AssertionError(path)

        def pages(path: str, key: str | None = None) -> list:
            if path == f"repos/{REPO}/commits/{HEAD}/check-runs":
                return {"check_runs": checks}.get(key, [])
            for run_id, row in runs.items():
                if path == f"repos/{REPO}/actions/runs/{run_id}/jobs":
                    check_id = row["id"]
                    return [{
                        "id": JOB, "name": "required-gate", "run_attempt": 2,
                        "check_run_url": f"https://api.github.com/repos/{REPO}/check-runs/{check_id}",
                        "status": job_statuses.get(run_id, "completed"),
                        "conclusion": job_conclusions.get(run_id, row["conclusion"]),
                        "steps": [{"name": "tests", "conclusion": job_conclusions.get(run_id, row["conclusion"])}],
                    }]
            raise AssertionError((path, key))

        with patch.object(DELIVERY, "_git_value", return_value=self.local_head), \
                patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages), \
                patch.object(DELIVERY, "_current_pr_process_waiver",
                             return_value=process_waiver or {"applicable": False}):
            return DELIVERY.read_current_pr_projection(Path("."), self.task)

    def test_later_matching_success_supersedes_old_failed_attempt(self) -> None:
        failed = check_row(check_id=CHECK, run_id=RUN, conclusion="failure")
        passed = check_row(check_id=CHECK + 2, run_id=RUN + 2, conclusion="success")
        projection, blockers = self.project([failed, passed], run_conclusions={RUN: "failure"})
        self.assertEqual(projection["required_ci"]["run_id"], RUN + 2)
        self.assertNotIn("CURRENT_CHECK_FAILED", {row["code"] for row in blockers})
        self.assertIn("SOURCE_NOT_PUBLISHED", {row["code"] for row in blockers})

    def test_wrong_pr_same_head_is_not_selected_and_cannot_hide_local_head_blocker(self) -> None:
        wrong = check_row(pr_numbers=[PR + 1])
        projection, blockers = self.project([wrong])
        codes = {row["code"] for row in blockers}
        self.assertIn("SOURCE_NOT_PUBLISHED", codes)
        self.assertIn("TASK_BINDING_CONFLICT", codes)
        self.assertEqual(projection["remote_pr_head_oid"], HEAD)

    def test_empty_run_association_requires_process_exception_and_keeps_source_blocker(self) -> None:
        check = check_row()
        verified_replacement = {"check_run_id": CHECK + 5, "run_id": RUN + 5}
        projection, blockers = self.project([check], run_prs={RUN: []}, process_waiver={
            "applicable": True, "verified_replacement_ci": verified_replacement,
        })
        self.assertIn("SOURCE_NOT_PUBLISHED", {row["code"] for row in blockers})
        self.assertEqual(projection["replacement_ci_identity"], verified_replacement)

    def test_process_waiver_does_not_rewrite_actual_failed_test_result(self) -> None:
        failed = check_row(conclusion="failure")
        replacement = {"check_run_id": CHECK + 5, "run_id": RUN + 5, "run_attempt": 1}
        projection, blockers = self.project([failed], process_waiver={
            "applicable": True, "verified_replacement_ci": replacement,
        })
        self.assertEqual(projection["required_ci"]["conclusion"], "failure")
        self.assertEqual(projection["failure_phase"], {"job": "required-gate", "step": "tests"})
        self.assertEqual(projection["replacement_ci_identity"], replacement)
        self.assertNotIn("CURRENT_CHECK_FAILED", {row["code"] for row in blockers})

    def test_nonterminal_check_run_or_job_waits_without_rerun_advice(self) -> None:
        cases = [
            ("check", check_row(status="in_progress", conclusion=None), {}, {}, {}, {RUN: "success"}),
            ("check-requested", check_row(status="requested", conclusion=None), {}, {}, {}, {RUN: "success"}),
            ("check-waiting", check_row(status="waiting", conclusion=None), {}, {}, {}, {RUN: "success"}),
            ("check-pending", check_row(status="pending", conclusion=None), {}, {}, {}, {RUN: "success"}),
            ("job-queued", check_row(), {}, {}, {RUN: "queued"}, {RUN: None}),
            ("job", check_row(), {}, {}, {RUN: "in_progress"}, {RUN: None}),
        ]
        for state, check, run_statuses, run_conclusions, job_statuses, job_conclusions in cases:
            with self.subTest(state=state):
                projection, blockers = self.project(
                    [check], run_statuses=run_statuses,
                    run_conclusions=run_conclusions, job_statuses=job_statuses,
                    job_conclusions=job_conclusions,
                )
                current = next(row for row in blockers if row["code"].startswith("CURRENT_CHECK_"))
                self.assertEqual(current["code"], "CURRENT_CHECK_PENDING", current)
                self.assertEqual(current["next_action_kind"], "wait_for_current_required_check", current)
                self.assertIn("merge", current["blocks_actions"], current)
                self.assertIn("complete", current["blocks_actions"], current)
                self.assertNotIn("consume_artifact", current["blocks_actions"], current)
                self.assertNotIn("rerun_applicable_check", current["allowed_actions"], current)
                self.assertIsNone(projection["failure_phase"], projection)

    def test_unknown_or_missing_required_check_status_is_unavailable(self) -> None:
        cases = [
            ("unknown-check", check_row(status="not-a-check-status", conclusion=None), {}, {}),
            ("missing-check", check_row(status=None, conclusion=None), {}, {}),
            ("pending-check-with-conclusion", check_row(status="waiting", conclusion="success"), {}, {}),
            ("unknown-job", check_row(), {RUN: "waiting"}, {RUN: None}),
            ("missing-job", check_row(), {RUN: None}, {RUN: None}),
            ("pending-job-with-conclusion", check_row(), {RUN: "queued"}, {RUN: "success"}),
        ]
        for name, check, job_statuses, job_conclusions in cases:
            with self.subTest(name=name):
                projection, blockers = self.project(
                    [check], job_statuses=job_statuses,
                    job_conclusions=job_conclusions,
                )
                current = next(row for row in blockers if row["code"].startswith("CURRENT_CHECK_"))
                self.assertEqual(current["code"], "CURRENT_CHECK_UNAVAILABLE", current)
                self.assertEqual(current["next_action_kind"], "restore_current_check_readback", current)
                self.assertNotIn("rerun_applicable_check", current["allowed_actions"], current)
                self.assertEqual(projection["read_status"], "uncertain", projection)
                self.assertIsNone(projection["failure_phase"], projection)

    def test_nonterminal_workflow_run_after_successful_required_gate_does_not_block(self) -> None:
        projection, blockers = self.project(
            [check_row()], run_statuses={RUN: "in_progress"},
            run_conclusions={RUN: None},
        )

        self.assertNotIn(
            "CURRENT_CHECK_PENDING", {row["code"] for row in blockers},
        )
        self.assertEqual(projection["required_ci"]["conclusion"], "success")

    def test_completed_unsuccessful_check_is_failed_and_allows_repair_then_rerun(self) -> None:
        failed = check_row(conclusion="failure")
        projection, blockers = self.project(
            [failed], run_conclusions={RUN: "failure"}, job_conclusions={RUN: "failure"},
        )

        current = next(row for row in blockers if row["code"].startswith("CURRENT_CHECK_"))
        self.assertEqual(current["code"], "CURRENT_CHECK_FAILED", current)
        self.assertEqual(current["next_action_kind"], "rerun_current_required_check", current)
        self.assertIn("diagnose", current["allowed_actions"], current)
        self.assertIn("rerun_applicable_check", current["allowed_actions"], current)
        self.assertIn("merge", current["blocks_actions"], current)
        self.assertIn("complete", current["blocks_actions"], current)
        self.assertNotIn("consume_artifact", current["blocks_actions"], current)
        self.assertEqual(projection["failure_phase"], {"job": "required-gate", "step": "tests"})

    def test_unreadable_current_ci_is_unavailable_not_reported_as_test_failure(self) -> None:
        check = check_row()
        run_path = f"repos/{REPO}/actions/runs/{RUN}"
        projection, blockers = self.project([check], read_error_paths={run_path})

        current = next(row for row in blockers if row["code"].startswith("CURRENT_CHECK_"))
        self.assertEqual(current["code"], "CURRENT_CHECK_UNAVAILABLE", current)
        self.assertEqual(current["next_action_kind"], "restore_current_check_readback", current)
        self.assertIn("merge", current["blocks_actions"], current)
        self.assertIn("complete", current["blocks_actions"], current)
        self.assertNotIn("consume_artifact", current["blocks_actions"], current)
        self.assertIn("inspect", current["allowed_actions"], current)
        self.assertNotIn("rerun_applicable_check", current["allowed_actions"], current)
        self.assertEqual(projection["read_status"], "uncertain", projection)
        self.assertIsNone(projection["failure_phase"], projection)

    def test_final_workflow_projection_exposes_waiver_and_replacement_without_rewriting_failure(self) -> None:
        original_ci = {"check_run_id": CHECK, "run_id": RUN, "conclusion": "failure"}
        process_exception = {"status": "applied", "decision": "waive_process_check", "comment_id": 44}
        replacement = {"check_run_id": CHECK + 1, "run_id": RUN + 1, "conclusion": "success"}
        live_projection = {
            "local_candidate_head_oid": HEAD,
            "remote_pr_head_oid": HEAD,
            "required_ci": original_ci,
            "process_exception": process_exception,
            "replacement_ci_identity": replacement,
            "failure_phase": {"job": "required-gate", "step": "tests"},
        }
        with patch.object(DELIVERY, "read_explicit_edge", return_value={}), \
                patch.object(DELIVERY, "read_current_pr_projection", return_value=(live_projection, [])):
            final_projection = DELIVERY.workflow_projection(Path("."), self.task, [])
        self.assertEqual(final_projection["process_exception"], process_exception)
        self.assertEqual(final_projection["replacement_ci_identity"], replacement)
        self.assertEqual(final_projection["ci_identity"], original_ci)


class LegacyProjectionTest(unittest.TestCase):
    def test_missing_artifact_declaration_retains_legacy_terminal_semantics(self) -> None:
        result = DELIVERY.derive_delivery_readiness(TASK_UID, {})
        self.assertIsNone(result["delivery_ready"])
        self.assertEqual(result["action_blockers"], [])

    def test_task_without_typed_edge_does_not_require_project_resolution(self) -> None:
        task = {"task_uid": TASK_UID, "repository": REPO, "issue_number": 900}
        issue = {"number": 900, "body": f"task_uid: {TASK_UID}"}
        with patch.object(DELIVERY, "_gh_json", return_value=issue), \
                patch.object(DELIVERY, "_pages", return_value=[]), \
                patch.object(DELIVERY, "_resolve_project_task_issue", side_effect=AssertionError("unexpected Project lookup")):
            self.assertEqual(DELIVERY.read_explicit_edge(Path("."), task), {})

    def test_noncanonical_repository_preserves_unclassified_legacy_delivery(self) -> None:
        task = {"task_uid": "task_" + "b" * 32, "repository": "fixture/repo", "issue_number": 11}
        with patch.object(DELIVERY, "_gh_json", side_effect=AssertionError("must not query GitHub")), \
                patch.object(DELIVERY, "_pages", side_effect=AssertionError("must not query GitHub")):
            self.assertEqual(DELIVERY.read_explicit_edge(Path("."), task), {})
        projected = DELIVERY.derive_delivery_readiness(task["task_uid"], {})
        self.assertIsNone(projected["delivery_ready"])


class NoFindingsAggregateTest(unittest.TestCase):
    def test_complete_no_findings_role_aggregate_is_accepted_without_relabeling(self) -> None:
        proof = complete_delivery_proof()
        result = DELIVERY.derive_delivery_readiness(TASK_UID, proof)

        self.assertTrue(result["delivery_ready"], result)
        self.assertEqual(proof["review"]["findings_disposition"], "no_findings")
        self.assertNotIn(
            "REVIEW_FINDING_BLOCKING",
            {row["code"] for row in result["action_blockers"]},
        )

    def test_incomplete_duplicate_or_malformed_no_findings_aggregate_is_blocked(self) -> None:
        cases = {
            "partial role evidence": {
                "finding_disposition_evidence": "agent_engineer: no_findings",
            },
            "duplicate role": {
                "roles": ["agent_engineer", "agent_engineer"],
                "finding_disposition_evidence": (
                    "agent_engineer: no_findings; agent_engineer: no_findings"
                ),
            },
            "malformed role": {
                "roles": ["agent_engineer", "not a role"],
                "finding_disposition_evidence": (
                    "agent_engineer: no_findings; not a role: no_findings"
                ),
            },
            "malformed disposition": {"findings_disposition": "no-findings"},
            "extra role evidence": {
                "finding_disposition_evidence": (
                    "agent_engineer: no_findings; qa_engineer: no_findings; "
                    "runtime_engineer: no_findings"
                ),
            },
        }
        for label, updates in cases.items():
            with self.subTest(case=label):
                proof = complete_delivery_proof()
                proof["review"].update(updates)
                result = DELIVERY.derive_delivery_readiness(TASK_UID, proof)

                self.assertFalse(result["delivery_ready"], result)
                self.assertIn(
                    "REVIEW_FINDING_BLOCKING",
                    {row["code"] for row in result["action_blockers"]},
                )

    def test_unresolved_review_findings_block_delivery(self) -> None:
        proof = complete_delivery_proof(review={
            "task_uid": UPSTREAM_UID,
            "source_head_oid": HEAD,
            "passed": True,
            "admin_author": True,
            "findings_disposition": "findings",
            "roles": ["agent_engineer", "qa_engineer"],
            "finding_disposition_evidence": (
                "agent_engineer: no_findings; qa_engineer: findings"
            ),
        })

        result = DELIVERY.derive_delivery_readiness(TASK_UID, proof)

        self.assertFalse(result["delivery_ready"], result)
        self.assertIn(
            "REVIEW_FINDING_BLOCKING",
            {row["code"] for row in result["action_blockers"]},
        )


class AuthoritativeEvidenceSelectionTest(unittest.TestCase):
    declaration_marker = DELIVERY.ARTIFACT_DEPENDENCY_MARKER
    declaration_schema = DELIVERY.ARTIFACT_DEPENDENCY_SCHEMA
    consuming_issue_url = f"https://api.github.com/repos/{REPO}/issues/900"
    source_issue_url = f"https://api.github.com/repos/{REPO}/issues/901"

    def declaration_comment(self, comment_id: int, login: str, *,
                            task_uid: str = TASK_UID, issue_number: int = 900,
                            body_override: str | None = None) -> dict:
        record = {
            "schema": self.declaration_schema,
            "repository": REPO,
            "task_uid": task_uid,
            "task_issue_number": issue_number,
            "upstream_task_uid": UPSTREAM_UID,
            "artifacts": [{"path": "doc/accepted.md", "sha256": "d" * 64}],
            "source_ci": {
                "pr_number": 55, "head_oid": HEAD, "check_run_id": CHECK,
                "run_id": RUN, "run_attempt": 2,
            },
        }
        body = body_override or (
            self.declaration_marker + "\n" +
            json.dumps(record, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
        )
        return {
            "id": comment_id, "issue_url": self.consuming_issue_url,
            "user": {"login": login}, "body": body,
        }

    def duplicate_key_declaration_comment(self, comment_id: int, login: str,
                                          key: str, second_value: Any) -> dict:
        comment = self.declaration_comment(comment_id, login)
        record = json.loads(comment["body"].split("\n", 1)[1])
        pairs = []
        for name in sorted(record):
            pairs.append((name, record[name]))
            if name == key:
                pairs.append((name, second_value))
        raw = ",".join(
            f"{json.dumps(name, ensure_ascii=True)}:{json.dumps(value, ensure_ascii=True)}"
            for name, value in pairs
        )
        comment["body"] = self.declaration_marker + "\n{" + raw + "}"
        return comment

    def review_comment(self, comment_id: int, login: str, updated_at: str, *,
                       task_uid: str = UPSTREAM_UID, head_oid: str = HEAD) -> dict:
        body = "\n".join((
            "- Pre-PR Local Role Review: passed",
            f"- Task UID: `{task_uid}`",
            f"- Source Head: `{head_oid}`",
            "- Review Roles: agent_engineer, qa_engineer",
            "- Review Findings Disposition: no_findings",
            "- Finding Disposition Evidence: agent_engineer: no_findings; qa_engineer: no_findings",
        ))
        return {
            "id": comment_id, "issue_url": self.source_issue_url,
            "created_at": updated_at, "updated_at": updated_at,
            "user": {"login": login}, "body": body,
        }

    def test_known_nonadmin_and_wrong_binding_do_not_make_declaration_ambiguous(self) -> None:
        another_task = "task_" + "c" * 32
        unrelated = self.declaration_comment(
            1, "synthetic-admin", task_uid=another_task,
        )
        wrong_issue = self.declaration_comment(7, "synthetic-admin")
        wrong_issue["issue_url"] = f"https://api.github.com/repos/{REPO}/issues/999"
        another_type = self.declaration_comment(8, "synthetic-admin")
        other_type_record = json.loads(another_type["body"].split("\n", 1)[1])
        other_type_record["schema"] = "oasis7.other-typed-evidence/v1"
        another_type["body"] = (
            self.declaration_marker + "\n" +
            json.dumps(other_type_record, ensure_ascii=True, sort_keys=True,
                       separators=(",", ":"))
        )
        nonadmin = self.declaration_comment(2, "synthetic-public")
        duplicate_nonadmin = self.duplicate_key_declaration_comment(
            9, "synthetic-public", "schema", "oasis7.other-typed-evidence/v1",
        )
        malformed_nonadmin = self.declaration_comment(
            3, "synthetic-public", body_override=self.declaration_marker + "\n{bad-json",
        )
        misframed_nonadmin = self.declaration_comment(
            4, "synthetic-public", body_override="prefix " + self.declaration_marker + "\n{}",
        )
        incomplete_record = {
            "schema": self.declaration_schema,
            "repository": REPO,
        }
        incomplete_nonadmin = self.declaration_comment(
            5, "synthetic-public", body_override=(
                self.declaration_marker + "\n" + json.dumps(
                    incomplete_record, ensure_ascii=True, sort_keys=True,
                    separators=(",", ":"),
                )
            ),
        )
        authorized = self.declaration_comment(6, "synthetic-admin")
        with patch.object(
            DELIVERY, "_is_admin",
            side_effect=lambda _repo, login: login == "synthetic-admin",
        ):
            selected = DELIVERY._typed_record(
                [unrelated, wrong_issue, another_type, nonadmin, duplicate_nonadmin, malformed_nonadmin,
                 misframed_nonadmin, incomplete_nonadmin, authorized], self.declaration_marker,
                self.declaration_schema, self.consuming_issue_url,
                repository=REPO, task_uid=TASK_UID, issue_number=900,
            )

        self.assertIsNotNone(selected)
        assert selected is not None
        self.assertEqual(selected[0]["id"], 6)

    def test_duplicate_schema_and_task_binding_keys_block_admin_declarations(self) -> None:
        cases = {
            "schema": "oasis7.other-typed-evidence/v1",
            "task_uid": "task_" + "e" * 32,
            "task_issue_number": 999,
            "repository": "other/repository",
        }
        for key, second_value in cases.items():
            with self.subTest(key=key):
                comment = self.duplicate_key_declaration_comment(
                    30, "synthetic-admin", key, second_value,
                )
                with patch.object(DELIVERY, "_is_admin", return_value=True):
                    with self.assertRaisesRegex(ValueError, "duplicate|JSON|malformed"):
                        DELIVERY._typed_record(
                            [comment], self.declaration_marker, self.declaration_schema,
                            self.consuming_issue_url, repository=REPO,
                            task_uid=TASK_UID, issue_number=900,
                        )

    def test_duplicate_declaration_keys_with_unknown_permission_fail_closed(self) -> None:
        comment = self.duplicate_key_declaration_comment(
            31, "synthetic-uncertain", "schema", "oasis7.other-typed-evidence/v1",
        )
        with patch.object(DELIVERY, "_is_admin", side_effect=ValueError("permission unavailable")):
            with self.assertRaisesRegex(ValueError, "permission|unavailable"):
                DELIVERY._typed_record(
                    [comment], self.declaration_marker, self.declaration_schema,
                    self.consuming_issue_url, repository=REPO,
                    task_uid=TASK_UID, issue_number=900,
                )

    def test_duplicate_authorized_declarations_are_ambiguous(self) -> None:
        comments = [
            self.declaration_comment(1, "synthetic-admin"),
            self.declaration_comment(2, "synthetic-admin"),
        ]
        with patch.object(DELIVERY, "_is_admin", return_value=True):
            with self.assertRaisesRegex(ValueError, "duplicate|ambiguous"):
                DELIVERY._typed_record(
                    comments, self.declaration_marker, self.declaration_schema,
                    self.consuming_issue_url, repository=REPO,
                    task_uid=TASK_UID, issue_number=900,
                )

    def test_malformed_marker_framed_admin_candidate_blocks_when_binding_is_unknown(self) -> None:
        comment = self.declaration_comment(
            7, "synthetic-admin", body_override=self.declaration_marker + "\n{bad-json",
        )
        with patch.object(DELIVERY, "_is_admin", return_value=True):
            with self.assertRaisesRegex(ValueError, "malformed|invalid|canonical"):
                DELIVERY._typed_record(
                    [comment], self.declaration_marker, self.declaration_schema,
                    self.consuming_issue_url, repository=REPO,
                    task_uid=TASK_UID, issue_number=900,
                )

    def test_incomplete_bound_admin_declaration_blocks(self) -> None:
        comment = self.declaration_comment(8, "synthetic-admin")
        record = json.loads(comment["body"].split("\n", 1)[1])
        record.pop("task_issue_number")
        comment["body"] = (
            self.declaration_marker + "\n" +
            json.dumps(record, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
        )
        with patch.object(DELIVERY, "_is_admin", return_value=True):
            with self.assertRaisesRegex(ValueError, "field|schema|canonical|malformed"):
                DELIVERY._typed_record(
                    [comment], self.declaration_marker, self.declaration_schema,
                    self.consuming_issue_url, repository=REPO,
                    task_uid=TASK_UID, issue_number=900,
                )

    def test_unreadable_permission_for_exact_bound_declaration_blocks(self) -> None:
        comment = self.declaration_comment(5, "synthetic-uncertain")
        with patch.object(DELIVERY, "_is_admin", side_effect=ValueError("permission unavailable")):
            with self.assertRaisesRegex(ValueError, "permission|unavailable"):
                DELIVERY._typed_record(
                    [comment], self.declaration_marker, self.declaration_schema,
                    self.consuming_issue_url, repository=REPO,
                    task_uid=TASK_UID, issue_number=900,
                )

    def test_newer_nonadmin_source_review_does_not_mask_authenticated_packet(self) -> None:
        authorized = self.review_comment(10, "synthetic-admin", "2026-09-28T00:00:00Z")
        latest_admin = self.review_comment(13, "synthetic-admin", "2026-09-29T12:00:00Z")
        newer_nonadmin = self.review_comment(11, "synthetic-public", "2026-09-30T00:00:00Z")
        with patch.object(
            DELIVERY, "_is_admin",
            side_effect=lambda _repo, login: login == "synthetic-admin",
        ):
            selected = DELIVERY._latest_authenticated_review(
                [authorized, latest_admin, newer_nonadmin], REPO,
                UPSTREAM_UID, HEAD, 901,
            )

        self.assertEqual(selected["id"], 13)

    def test_unreadable_permission_for_exact_source_review_blocks_selection(self) -> None:
        candidate = self.review_comment(12, "synthetic-uncertain", "2026-09-29T00:00:00Z")
        with patch.object(DELIVERY, "_is_admin", side_effect=ValueError("permission unavailable")):
            with self.assertRaisesRegex(ValueError, "permission|unavailable"):
                DELIVERY._latest_authenticated_review(
                    [candidate], REPO, UPSTREAM_UID, HEAD, 901,
                )


class PermissionNotFoundRecoveryTest(unittest.TestCase):
    def setUp(self) -> None:
        self.issue_url = f"https://api.github.com/repos/{REPO}/issues/900"

    def exception_comment(self, comment_id: int, login: str, created_at: str) -> dict:
        payload = {
            "schema": "oasis7-process-exception/v1",
            "task_uid": TASK_UID,
            "action": "current_pr_validation",
            "head_oid": HEAD,
            "scope": "required-gate",
            "decision": "waive_process_check",
            "reason": "synthetic test decision",
            "authorized_decision": "synthetic authorization",
            "replacement_evidence": {"pr_number": PR, "head_oid": HEAD,
                                      "check_run_id": CHECK, "run_id": RUN, "run_attempt": 1},
        }
        body = DELIVERY.PROCESS_EXCEPTIONS.MARKER + "\n" + json.dumps(payload, sort_keys=True, separators=(",", ":"))
        return {"id": comment_id, "issue_url": self.issue_url, "created_at": created_at,
                "updated_at": created_at, "user": {"login": login}, "body": body}

    @staticmethod
    def permission_api(args, **_kwargs):
        login = args[-1].split("/")[-2]
        status = (200 if login == "synthetic-admin" else 404 if login == "synthetic-public"
                  else 403 if login == "synthetic-forbidden" else 503)
        body = (json.dumps({"permission": "admin", "user": {"login": login}})
                if status == 200 else json.dumps({"message": "permission read failed", "status": str(status)}))
        response = f"HTTP/2.0 {status} {'OK' if status == 200 else 'Unavailable' if status == 503 else 'Not Found'}\nContent-Type: application/json\n\n{body}"
        return SimpleNamespace(returncode=0 if status == 200 else 1, stdout=response,
                               stderr="" if status == 200 else f"gh: HTTP {status}")

    def resolve(self, comments):
        return DELIVERY.PROCESS_EXCEPTIONS.resolve_process_exception(
            comments,
            repository=REPO,
            issue_number=900,
            task_uid=TASK_UID,
            action="current_pr_validation",
            head_oid=HEAD,
            scope="required-gate",
            live_admin_by_login=lambda login: DELIVERY._is_admin(REPO, login),
            replacement_validator=lambda _evidence, _record: True,
            process_check_waivable=True,
        )

    def test_http_404_public_comment_does_not_poison_older_admin_exception(self) -> None:
        comments = [
            self.exception_comment(1, "synthetic-admin", "2026-09-29T00:00:01Z"),
            self.exception_comment(2, "synthetic-public", "2026-09-29T00:00:02Z"),
        ]
        with patch.object(DELIVERY.subprocess, "run", side_effect=self.permission_api):
            result = self.resolve(comments)
        self.assertEqual(result["status"], "applied")
        self.assertEqual(result["comment_id"], 1)

    def test_http_503_permission_uncertainty_still_blocks_newer_matching_record(self) -> None:
        comments = [
            self.exception_comment(1, "synthetic-admin", "2026-09-29T00:00:01Z"),
            self.exception_comment(2, "synthetic-temporary-failure", "2026-09-29T00:00:02Z"),
        ]
        with patch.object(DELIVERY.subprocess, "run", side_effect=self.permission_api):
            result = self.resolve(comments)
        self.assertEqual(result["status"], "rejected")
        self.assertIn("permission is unreadable", result["reason"])

    def test_http_403_permission_uncertainty_is_not_treated_as_nonadmin(self) -> None:
        comments = [
            self.exception_comment(1, "synthetic-admin", "2026-09-29T00:00:01Z"),
            self.exception_comment(2, "synthetic-forbidden", "2026-09-29T00:00:02Z"),
        ]
        with patch.object(DELIVERY.subprocess, "run", side_effect=self.permission_api):
            result = self.resolve(comments)
        self.assertEqual(result["status"], "rejected")
        self.assertIn("permission is unreadable", result["reason"])


class ResourceDependencyConsumerTest(unittest.TestCase):
    """Exercise resource waits through the consumer and real local Git readback."""

    consuming_issue_url = f"https://api.github.com/repos/{REPO}/issues/900"
    upstream_issue_url = f"https://api.github.com/repos/{REPO}/issues/901"
    upstream_pr_url = f"https://github.com/{REPO}/pull/55"
    upstream_branch = "task/upstream-resource"

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="workflow-resource-consumer-")
        self.temp_root = Path(self.temp.name)
        self.repo = self.temp_root / "repo"
        self._git("init", "-q", "-b", "main", str(self.repo))
        self._git("-C", str(self.repo), "config", "user.email", "test@example.invalid")
        self._git("-C", str(self.repo), "config", "user.name", "Resource Consumer Test")
        self._git("-C", str(self.repo), "remote", "add", "origin", f"https://github.com/{REPO}.git")
        artifact = self.repo / "doc/accepted.md"
        artifact.parent.mkdir(parents=True)
        artifact.write_text("accepted upstream artifact\n", encoding="utf-8")
        self._git("-C", str(self.repo), "add", "doc/accepted.md")
        self._git("-C", str(self.repo), "commit", "-qm", "accepted upstream artifact")
        self.head_oid = self._git("-C", str(self.repo), "rev-parse", "HEAD")
        common = Path(self._git("-C", str(self.repo), "rev-parse", "--git-common-dir"))
        self.common_dir = (self.repo / common if not common.is_absolute() else common).resolve()

        self.upstream_record = {
            "task_uid": UPSTREAM_UID,
            "repository": REPO,
            "issue_number": 901,
            "pr_number": 55,
            "pr_url": self.upstream_pr_url,
            "status": "done",
            "workflow_phase": "post_merge_done",
            "task_branch": self.upstream_branch,
        }
        mapping = self.repo / ".pm/github-project-sync/tasks.json"
        mapping.parent.mkdir(parents=True)
        mapping.write_text(
            json.dumps({"version": 1, "tasks": {UPSTREAM_UID: self.upstream_record}}) + "\n",
            encoding="utf-8",
        )
        self.receipt_root = DELIVERY._canonical_receipt_root(self.repo, UPSTREAM_UID)
        self.receipt_root.mkdir(parents=True, exist_ok=True)
        self.local_branch_resource_id = {
            "full_ref": f"refs/heads/{self.upstream_branch}",
            "expected_oid": self.head_oid,
        }
        self.remote_branch_resource_id = {
            "remote_repository": REPO,
            "full_ref": f"refs/heads/{self.upstream_branch}",
            "expected_oid": self.head_oid,
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def _git(*args: str) -> str:
        result = subprocess.run(["git", *args], text=True, capture_output=True, check=True)
        return result.stdout.strip()

    def resource_record(self, *, resource_id: dict[str, Any] | None = None,
                        upstream_uid: str = UPSTREAM_UID,
                        kind: str = "local_branch") -> dict[str, Any]:
        if kind not in {"local_branch", "remote_branch"}:
            raise AssertionError(f"unsupported fixture resource kind: {kind}")
        identities = {
            "local_branch": self.local_branch_resource_id,
            "remote_branch": self.remote_branch_resource_id,
        }
        return {
            "schema": DELIVERY.RESOURCE_DEPENDENCY_SCHEMA,
            "repository": REPO,
            "task_uid": TASK_UID,
            "task_issue_number": 900,
            "upstream_task_uid": upstream_uid,
            "resources": [{
                "kind": kind,
                "resource_id": resource_id or identities[kind],
                "required_state": "released",
            }],
        }

    def marker_comment(self, marker: str, record: dict[str, Any], comment_id: int) -> dict[str, Any]:
        return {
            "id": comment_id,
            "issue_url": self.consuming_issue_url,
            "user": {"login": "synthetic-admin"},
            "body": marker + "\n" + json.dumps(
                record, ensure_ascii=(marker == DELIVERY.ARTIFACT_DEPENDENCY_MARKER),
                sort_keys=True, separators=(",", ":"),
            ),
        }

    def artifact_comment(self) -> dict[str, Any]:
        locator = {
            "pr_number": 55, "head_oid": self.head_oid, "check_run_id": CHECK,
            "run_id": RUN, "run_attempt": 2,
        }
        declaration = {
            "schema": DELIVERY.ARTIFACT_DEPENDENCY_SCHEMA,
            "repository": REPO,
            "task_uid": TASK_UID,
            "task_issue_number": 900,
            "upstream_task_uid": UPSTREAM_UID,
            "artifacts": [{
                "path": "doc/accepted.md",
                "sha256": hashlib.sha256(b"accepted upstream artifact\n").hexdigest(),
            }],
            "source_ci": locator,
        }
        return self.marker_comment(
            DELIVERY.ARTIFACT_DEPENDENCY_MARKER,
            declaration,
            101,
        )

    def accepted_source_ci(self) -> dict[str, Any]:
        return {
            "check_name": "required-gate",
            "app_id": DELIVERY.REQUIRED_GATE_APP_ID,
            "check_run_id": CHECK,
            "run_id": RUN,
            "run_attempt": 2,
            "pr_number": 55,
            "head_oid": self.head_oid,
            "pr_association_verified": True,
            "status": "completed",
            "conclusion": "success",
        }

    def write_cleanup_snapshot(self, state: str, *, recorded_resource_id: dict[str, Any] | None = None) -> None:
        path = SCRIPT.with_name("resource-cleanup-executor.py")
        spec = importlib.util.spec_from_file_location("readiness_cleanup_executor_fixture", path)
        if spec is None or spec.loader is None:
            raise AssertionError(f"cannot load cleanup fixture helper {path}")
        executor = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(executor)

        revision = 1
        identities = {
            "worktree": {
                "path": str((self.temp_root / "removed-upstream-worktree").resolve()),
                "common_dir": str(self.common_dir),
                "admin_dir": str((self.common_dir / "worktrees" / "removed-upstream-worktree").resolve()),
                "instance_id": "00000000-0000-4000-8000-000000000001",
                "expected_head_oid": self.head_oid,
            },
            "local_branch": recorded_resource_id or self.local_branch_resource_id,
            "remote_branch": {
                "remote_repository": REPO,
                "full_ref": f"refs/heads/{self.upstream_branch}",
                "expected_oid": self.head_oid,
            },
        }
        operations = {
            "worktree": "git_worktree_remove",
            "local_branch": "git_update_ref_cas",
            "remote_branch": "git_push_delete_with_lease",
        }
        journal = self.receipt_root / "resource-cleanup-journal.jsonl"
        rows = []
        for kind in executor.RESOURCE_KINDS:
            identity = identities[kind]
            operation = operations[kind]
            operation_id = hashlib.sha256(executor._canonical({
                "task_uid": UPSTREAM_UID,
                "revision": revision,
                "kind": kind,
                "resource_id": identity,
                "operation": operation,
            })).hexdigest()
            executor._append_journal(journal, executor._event(
                UPSTREAM_UID, REPO, revision, "intent", kind=kind,
                resource_id=identity, operation=operation, operation_id=operation_id,
            ))
            rows.append({
                "kind": kind,
                "resource_id": identity,
                "state": state,
                "operation": operation,
                "reason": "fixture cleanup observation",
                "readback": {"fixture": True},
                "observed_at": executor._now(),
            })

        snapshot = {
            "schema": executor.SNAPSHOT_SCHEMA,
            "task_uid": UPSTREAM_UID,
            "repository": REPO,
            "revision": revision,
            "observed_at": executor._now(),
            "resources": rows,
        }
        raw = json.dumps(snapshot, ensure_ascii=False, sort_keys=True, indent=2).encode("utf-8") + b"\n"
        executor._atomic_write(self.receipt_root / "resource-cleanup.json", raw)
        executor._append_journal(journal, executor._event(
            UPSTREAM_UID, REPO, revision, "record", record_sha256=hashlib.sha256(raw).hexdigest(),
        ))

    def read_proof(self, resource_comments: list[dict[str, Any]] | None = None,
                   *, include_artifact: bool = True,
                   permission_check: Any | None = None) -> dict[str, Any]:
        downstream_body = f"task_uid: {TASK_UID}\n"
        upstream_body = "\n".join((
            f"task_uid: {UPSTREAM_UID}",
            "- status: `done`",
            "- workflow_phase: `post_merge_done`",
            "- merge_hold_active: `false`",
        ))
        downstream_issue = {"number": 900, "url": self.consuming_issue_url, "body": downstream_body}
        upstream_issue = {"number": 901, "url": self.upstream_issue_url, "body": upstream_body}
        pr = {
            "number": 55, "state": "closed", "merged": True,
            "merge_commit_sha": self.head_oid,
            "head": {"sha": self.head_oid, "ref": self.upstream_branch},
            "base": {"ref": "main"},
            "body": f"Task: {UPSTREAM_UID}\nRefs #901",
        }
        comments = list(resource_comments or [])
        if include_artifact:
            comments.insert(0, self.artifact_comment())

        def gh_json(path: str) -> dict[str, Any]:
            if path == f"repos/{REPO}/issues/900":
                return {"number": 900, "body": downstream_body}
            if path == f"repos/{REPO}/pulls/55":
                return pr
            if path == f"repos/{REPO}":
                return {"default_branch": "main"}
            raise AssertionError(f"unexpected GitHub JSON read: {path}")

        def pages(path: str, _key: str | None = None) -> list[dict[str, Any]]:
            if path == f"repos/{REPO}/issues/900/comments":
                return comments
            if path == f"repos/{REPO}/issues/901/comments":
                return []
            if path == f"repos/{REPO}/pulls/55/files":
                return [{"filename": "doc/accepted.md"}]
            raise AssertionError(f"unexpected GitHub page read: {path}")

        project_reader = SimpleNamespace(_live_task_pr=lambda _issue, _uid: (55, self.upstream_pr_url))
        review = {
            "id": 202,
            "updated_at": "2026-09-30T00:00:00Z",
            "user": {"login": "synthetic-admin"},
            "body": "\n".join((
                "- Pre-PR Local Role Review: passed",
                f"- Task UID: `{UPSTREAM_UID}`",
                f"- Source Head: `{self.head_oid}`",
                "- Review Roles: agent_engineer, qa_engineer",
                "- Review Findings Disposition: no_findings",
                "- Finding Disposition Evidence: agent_engineer: no_findings; qa_engineer: no_findings",
            )),
        }
        loop_terminal = types.ModuleType("loop_terminal")
        loop_terminal.read_shared_terminal_proof = lambda *_args, **_kwargs: {
            "status": "passed", "protocol_version": 2, "head_oid": self.head_oid,
        }
        permission_patch = (
            patch.object(DELIVERY, "_is_admin", side_effect=permission_check)
            if permission_check is not None else
            patch.object(DELIVERY, "_is_admin", return_value=True)
        )
        with (
            patch.object(DELIVERY, "_gh_json", side_effect=gh_json),
            patch.object(DELIVERY, "_pages", side_effect=pages),
            patch.object(DELIVERY, "_resolve_project_task_issue",
                         side_effect=lambda uid, _number=None: downstream_issue if uid == TASK_UID else upstream_issue),
            patch.object(DELIVERY, "_project_readback", return_value=project_reader),
            permission_patch,
            patch.object(DELIVERY, "_verify_declared_ci", return_value=self.accepted_source_ci()),
            patch.object(DELIVERY, "_latest_authenticated_review", return_value=review),
            patch.dict(sys.modules, {"loop_terminal": loop_terminal}),
        ):
            return DELIVERY.read_explicit_edge(
                self.repo, {"task_uid": TASK_UID, "repository": REPO, "issue_number": 900},
            )

    def readiness(self, resource_comments: list[dict[str, Any]] | None = None,
                  *, include_artifact: bool = True,
                  permission_check: Any | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
        proof = self.read_proof(
            resource_comments, include_artifact=include_artifact,
            permission_check=permission_check,
        )
        return DELIVERY.derive_delivery_readiness(TASK_UID, proof), proof

    def resource_blocker(self, result: dict[str, Any]) -> dict[str, Any]:
        blockers = [row for row in result["action_blockers"] if row["code"] == "RESOURCE_RELEASE_PENDING"]
        self.assertEqual(len(blockers), 1, result)
        return blockers[0]

    def test_released_exact_local_branch_allows_matching_artifact_consume(self) -> None:
        self.write_cleanup_snapshot("removed")
        resource_comment = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 102,
        )

        result, proof = self.readiness([resource_comment])

        self.assertTrue(result["delivery_ready"], result)
        self.assertEqual(result["resource_wait_state"], "released", result)
        self.assertTrue(proof["resource_release"]["ready"], proof)
        self.assertTrue(proof["resource_release"]["rows"][0]["released"], proof)
        self.assertFalse(any(row["code"] == "RESOURCE_RELEASE_PENDING" for row in result["action_blockers"]))

    def test_retained_and_present_resources_block_only_artifact_consume(self) -> None:
        resource_comment = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 102,
        )
        for label, state, create_branch in (
            ("retained snapshot", "retained", False),
            ("present after removed snapshot", "removed", True),
        ):
            with self.subTest(case=label):
                for name in ("resource-cleanup.json", "resource-cleanup-journal.jsonl"):
                    (self.receipt_root / name).unlink(missing_ok=True)
                if create_branch:
                    self._git("-C", str(self.repo), "branch", self.upstream_branch, self.head_oid)
                self.write_cleanup_snapshot(state)

                result, proof = self.readiness([resource_comment])

                self.assertTrue(result["delivery_ready"], result)
                self.assertEqual(result["resource_wait_state"], "pending", result)
                blocker = self.resource_blocker(result)
                self.assertEqual(blocker["blocks_actions"], ["consume_artifact"])
                self.assertIn("inspect", blocker["allowed_actions"])
                self.assertFalse(proof["resource_release"]["ready"], proof)
                if create_branch:
                    self.assertIn("present or its name has been reused", " ".join(proof["resource_release"]["reasons"]))
                    self._git("-C", str(self.repo), "branch", "-D", self.upstream_branch)
                else:
                    self.assertIn("cleanup state is retained", " ".join(proof["resource_release"]["reasons"]))

    def test_reused_and_mismatched_local_branch_identity_blocks_consume(self) -> None:
        self.write_cleanup_snapshot("removed")
        self._git("-C", str(self.repo), "branch", self.upstream_branch, self.head_oid)
        resource_comment = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 102,
        )
        result, proof = self.readiness([resource_comment])
        self.assertFalse(proof["resource_release"]["ready"], proof)
        self.assertIn("present or its name has been reused", " ".join(proof["resource_release"]["reasons"]))
        self.assertEqual(self.resource_blocker(result)["blocks_actions"], ["consume_artifact"])
        self._git("-C", str(self.repo), "branch", "-D", self.upstream_branch)

        changed_id = {**self.local_branch_resource_id, "expected_oid": "b" * 40}
        mismatched_comment = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(resource_id=changed_id), 103,
        )
        mismatched, _proof = self.readiness([mismatched_comment])
        self.assertEqual(mismatched["resource_wait_state"], "blocked", mismatched)
        self.assertIn("differs from the accepted delivery", self.resource_blocker(mismatched)["reason"])

    def test_remote_404_requires_repository_read_permission_before_release(self) -> None:
        self.write_cleanup_snapshot("removed")
        resource_comment = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER,
            self.resource_record(kind="remote_branch"),
            105,
        )
        real_run = DELIVERY.subprocess.run

        def github_read(permission: bool):
            calls: list[str] = []

            def run(args, *positional, **keywords):
                if args[:2] != ["gh", "api"] or "--include" not in args:
                    return real_run(args, *positional, **keywords)
                endpoint = args[-1]
                calls.append(endpoint)
                if endpoint == f"repos/{REPO}/branches?per_page=1":
                    status, payload = ((200, [{"name": "main"}]) if permission else
                                       (404, {"message": "Not Found"}))
                elif endpoint == f"repos/{REPO}/git/ref/heads/{self.upstream_branch}":
                    status, payload = 404, {"message": "Not Found"}
                else:
                    raise AssertionError(f"unexpected remote cleanup read: {args!r}")
                stdout = f"HTTP/2 {status} {'OK' if status == 200 else 'Not Found'}\r\n\r\n"
                stdout += json.dumps(payload)
                return subprocess.CompletedProcess(args, 0, stdout, "")
            return run, calls

        for label, permission, expected_state in (
            ("permission-hidden ref", False, "pending"),
            ("authorized genuine absence", True, "released"),
        ):
            with self.subTest(case=label):
                run, calls = github_read(permission)
                with patch.object(DELIVERY.subprocess, "run", side_effect=run):
                    result, proof = self.readiness([resource_comment])
                self.assertTrue(result["delivery_ready"], result)
                self.assertEqual(result["resource_wait_state"], expected_state, result)
                row = proof["resource_release"]["rows"][0]
                self.assertEqual(row["released"], permission, proof["resource_release"])
                self.assertIn(f"repos/{REPO}/branches?per_page=1", calls)
                if not permission:
                    self.assertEqual(self.resource_blocker(result)["blocks_actions"], ["consume_artifact"])

    def test_malformed_ambiguous_and_missing_resource_evidence_fail_closed(self) -> None:
        malformed = {
            "id": 102, "issue_url": self.consuming_issue_url,
            "user": {"login": "synthetic-admin"},
            "body": DELIVERY.RESOURCE_DEPENDENCY_MARKER + "\n{bad-json",
        }
        ambiguous = [
            self.marker_comment(DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 102),
            self.marker_comment(DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 103),
        ]
        self.write_cleanup_snapshot("removed")
        for label, comments in (("malformed marker", [malformed]), ("duplicate declaration", ambiguous)):
            with self.subTest(case=label):
                result, _proof = self.readiness(comments)
                self.assertEqual(result["resource_wait_state"], "blocked", result)
                self.assertEqual(self.resource_blocker(result)["blocks_actions"], ["consume_artifact"])

        for name in ("resource-cleanup.json", "resource-cleanup-journal.jsonl"):
            (self.receipt_root / name).unlink(missing_ok=True)
        valid = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 104,
        )
        missing, proof = self.readiness([valid])
        self.assertEqual(missing["resource_wait_state"], "pending", missing)
        self.assertIn("readback failed", " ".join(proof["resource_release"]["reasons"]))
        self.assertEqual(self.resource_blocker(missing)["blocks_actions"], ["consume_artifact"])

    def test_possible_admin_resource_candidate_with_unknown_schema_fails_closed(self) -> None:
        unknown_schema = self.resource_record()
        unknown_schema["schema"] = "oasis7.unregistered-resource/v1"
        missing_schema = self.resource_record()
        del missing_schema["schema"]
        nonadmin_schema = dict(unknown_schema)
        wrong_task_schema = {**unknown_schema, "task_uid": "task_" + "c" * 32}

        def permission_unavailable_for_candidate(_repo: str, login: str) -> bool:
            if login == "synthetic-uncertain":
                raise ValueError("permission unavailable")
            return True

        def known_nonadmin_candidate(_repo: str, login: str) -> bool:
            return login != "synthetic-public"

        cases = (
            (
                "unknown schema with admin", unknown_schema, "synthetic-admin",
                lambda _repo, _login: True, "blocked",
            ),
            (
                "missing schema with admin", missing_schema, "synthetic-admin",
                lambda _repo, _login: True, "blocked",
            ),
            (
                "unknown permission for possible match", unknown_schema, "synthetic-uncertain",
                permission_unavailable_for_candidate,
                "blocked",
            ),
            (
                "known nonadmin", nonadmin_schema, "synthetic-public",
                known_nonadmin_candidate, "not_applicable",
            ),
            (
                "proven different Task binding", wrong_task_schema, "synthetic-admin",
                lambda _repo, _login: True, "not_applicable",
            ),
        )
        self.write_cleanup_snapshot("removed")
        for index, (label, record, author, permission_check, expected_state) in enumerate(cases, start=120):
            with self.subTest(case=label):
                comment = self.marker_comment(
                    DELIVERY.RESOURCE_DEPENDENCY_MARKER, record, index,
                )
                comment["user"]["login"] = author
                result, proof = self.readiness(
                    [comment], permission_check=permission_check,
                )

                self.assertEqual(result["resource_wait_state"], expected_state, result)
                if expected_state == "blocked":
                    self.assertEqual(
                        self.resource_blocker(result)["blocks_actions"], ["consume_artifact"],
                    )
                    self.assertTrue(proof.get("resource_dependency_error"), proof)
                else:
                    self.assertIsNone(proof.get("resource_dependency_error"), proof)

    def test_resource_wait_without_artifact_fails_closed_and_no_wait_preserves_artifact(self) -> None:
        self.write_cleanup_snapshot("removed")
        resource_comment = self.marker_comment(
            DELIVERY.RESOURCE_DEPENDENCY_MARKER, self.resource_record(), 102,
        )
        no_artifact, _proof = self.readiness([resource_comment], include_artifact=False)
        self.assertIsNone(no_artifact["delivery_ready"], no_artifact)
        self.assertEqual(no_artifact["resource_wait_state"], "blocked", no_artifact)
        self.assertIn("no unique matching artifact", self.resource_blocker(no_artifact)["reason"])

        ordinary_artifact, ordinary_proof = self.readiness()
        self.assertTrue(ordinary_artifact["delivery_ready"], ordinary_artifact)
        self.assertEqual(ordinary_artifact["resource_wait_state"], "not_applicable", ordinary_artifact)
        self.assertIsNone(ordinary_proof["resource_dependency"], ordinary_proof)


if __name__ == "__main__":
    unittest.main()
