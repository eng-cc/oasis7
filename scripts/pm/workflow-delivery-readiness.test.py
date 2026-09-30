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
            ("check", check_row(status="in_progress", conclusion=None), {}, {}, {}),
            ("job", check_row(), {}, {}, {RUN: "in_progress"}),
        ]
        for state, check, run_statuses, run_conclusions, job_statuses in cases:
            with self.subTest(state=state):
                projection, blockers = self.project(
                    [check], run_statuses=run_statuses,
                    run_conclusions=run_conclusions, job_statuses=job_statuses,
                    job_conclusions={RUN: None} if state == "job" else {},
                )
                current = next(row for row in blockers if row["code"].startswith("CURRENT_CHECK_"))
                self.assertEqual(current["code"], "CURRENT_CHECK_PENDING", current)
                self.assertEqual(current["next_action_kind"], "wait_for_current_required_check", current)
                self.assertIn("merge", current["blocks_actions"], current)
                self.assertIn("complete", current["blocks_actions"], current)
                self.assertNotIn("consume_artifact", current["blocks_actions"], current)
                self.assertNotIn("rerun_applicable_check", current["allowed_actions"], current)
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
                [unrelated, wrong_issue, another_type, nonadmin, malformed_nonadmin,
                 misframed_nonadmin, incomplete_nonadmin, authorized], self.declaration_marker,
                self.declaration_schema, self.consuming_issue_url,
                repository=REPO, task_uid=TASK_UID, issue_number=900,
            )

        self.assertIsNotNone(selected)
        assert selected is not None
        self.assertEqual(selected[0]["id"], 6)

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


if __name__ == "__main__":
    unittest.main()
