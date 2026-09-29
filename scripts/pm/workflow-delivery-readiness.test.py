#!/usr/bin/env python3
"""Focused tests for project-bound artifact readiness and live CI identity."""
from __future__ import annotations

import importlib.util
import json
from types import SimpleNamespace
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


def check_row(*, check_id: int = CHECK, run_id: int = RUN, head: str = HEAD,
              pr_numbers: list[int] | None = None, conclusion: str = "success") -> dict:
    return {
        "id": check_id,
        "name": "required-gate",
        "app": {"id": DELIVERY.REQUIRED_GATE_APP_ID},
        "pull_requests": [{"number": number} for number in (pr_numbers if pr_numbers is not None else [PR])],
        "details_url": f"https://github.com/{REPO}/actions/runs/{run_id}/job/{JOB}",
        "head_sha": head,
        "status": "completed",
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
                run_conclusions: dict[int, str] | None = None):
        run_prs = run_prs or {}
        run_conclusions = run_conclusions or {}
        runs = {int(row["details_url"].split("/runs/")[1].split("/")[0]): row for row in checks}

        def gh_json(path: str) -> dict:
            if path == f"repos/{REPO}/pulls/{PR}":
                return self.pr
            for run_id, row in runs.items():
                if path == f"repos/{REPO}/actions/runs/{run_id}":
                    return {
                        "id": run_id, "run_attempt": 2, "head_sha": HEAD,
                        "head_branch": "feature", "path": ".github/workflows/rust.yml",
                        "event": "pull_request",
                        "pull_requests": [{"number": n} for n in run_prs.get(run_id, [PR])],
                        "status": "completed",
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
                        "status": "completed", "conclusion": row["conclusion"],
                        "steps": [{"name": "tests", "conclusion": row["conclusion"]}],
                    }]
            raise AssertionError((path, key))

        with patch.object(DELIVERY, "_git_value", return_value=self.local_head), \
                patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                patch.object(DELIVERY, "_pages", side_effect=pages):
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
        with patch.object(DELIVERY, "_current_pr_process_waiver", return_value={"applicable": False}):
            projection, blockers = self.project([wrong])
        codes = {row["code"] for row in blockers}
        self.assertIn("SOURCE_NOT_PUBLISHED", codes)
        self.assertIn("CURRENT_CHECK_FAILED", codes)
        self.assertEqual(projection["remote_pr_head_oid"], HEAD)

    def test_empty_run_association_requires_process_exception_and_keeps_source_blocker(self) -> None:
        check = check_row()
        verified_replacement = {"check_run_id": CHECK + 5, "run_id": RUN + 5}
        with patch.object(DELIVERY, "_current_pr_process_waiver", return_value={
            "applicable": True, "verified_replacement_ci": verified_replacement,
        }):
            projection, blockers = self.project([check], run_prs={RUN: []})
        self.assertIn("SOURCE_NOT_PUBLISHED", {row["code"] for row in blockers})
        self.assertEqual(projection["replacement_ci_identity"], verified_replacement)

    def test_process_waiver_does_not_rewrite_actual_failed_test_result(self) -> None:
        failed = check_row(conclusion="failure")
        replacement = {"check_run_id": CHECK + 5, "run_id": RUN + 5, "run_attempt": 1}
        with patch.object(DELIVERY, "_current_pr_process_waiver", return_value={
            "applicable": True, "verified_replacement_ci": replacement,
        }):
            projection, blockers = self.project([failed])
        self.assertEqual(projection["required_ci"]["conclusion"], "failure")
        self.assertEqual(projection["failure_phase"], {"job": "required-gate", "step": "tests"})
        self.assertEqual(projection["replacement_ci_identity"], replacement)
        self.assertNotIn("CURRENT_CHECK_FAILED", {row["code"] for row in blockers})

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


if __name__ == "__main__":
    unittest.main()
