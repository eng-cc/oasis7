#!/usr/bin/env python3
"""Focused tests for independent V1 validation readback."""

from __future__ import annotations

import importlib.util
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
import zipfile


MODULE_PATH = Path(__file__).with_name("ci_reuse_validation_readback.py")
SPEC = importlib.util.spec_from_file_location("ci_reuse_validation_readback", MODULE_PATH)
readback = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(readback)


def http_response(body: bytes, *, link: str | None = None, status: str = "200 OK") -> bytes:
    headers = [f"HTTP/2.0 {status}", "Content-Type: application/json"]
    if link is not None:
        headers.append(f"Link: {link}")
    return ("\r\n".join(headers) + "\r\n\r\n").encode("ascii") + body


class GitHubPaginationTests(unittest.TestCase):
    def test_unfiltered_run_pages_continue_past_search_ceiling(self):
        workflow_id = 230018940
        rows = [{"id": index, "display_title": f"unrelated-{index}"}
                for index in range(1, 1002)]
        pages = [rows[offset:offset + 100] for offset in range(0, len(rows), 100)]
        responses = []
        for index, batch in enumerate(pages, start=1):
            next_link = None
            if index < len(pages):
                next_link = (
                    f'<https://api.github.com/repos/eng-cc/oasis7/actions/workflows/'
                    f'{workflow_id}/runs?per_page=100&page={index + 1}>; rel="next"'
                )
            payload = readback.json.dumps(
                {"total_count": len(rows), "workflow_runs": batch},
                separators=(",", ":"),
            ).encode("utf-8")
            responses.append(http_response(payload, link=next_link))

        with patch.object(readback.subprocess, "check_output", side_effect=responses) as call:
            page_values = readback.GitHubReadOnly().workflow_run_pages(workflow_id)

        self.assertEqual(len(pages), len(page_values))
        self.assertEqual(1001, sum(len(page["runs"]) for page in page_values))
        complete = readback.contract.collect_workflow_runs(page_values)
        self.assertEqual(1001, len(complete))
        self.assertEqual(len(pages) - 1, sum(page["has_next"] for page in page_values))
        self.assertTrue(all(page["total_count"] == 1001 for page in page_values))
        self.assertEqual(len(pages), call.call_count)
        for invocation in call.call_args_list:
            argv = invocation.args[0]
            endpoint = argv[-1]
            self.assertIn("actions/workflows/230018940/runs?per_page=100", endpoint)
            self.assertNotRegex(endpoint, r"(?:actor|branch|check_suite_id|created|event|head_sha|status)=")
            self.assertIn("--include", argv)

    def test_unique_candidate_is_decided_after_all_pages_past_1000(self):
        workflow_id = 230018940
        fixture = readback_fixture()
        authority = readback.contract.resolve_records_for_readback(fixture["comments"])
        title = readback.contract.expected_run_title(authority)
        duplicate = {"id": 1002, "display_title": title + "|different-key"}
        rows = [{"id": index, "display_title": f"unrelated-{index}"}
                for index in range(1, 1001)] + [
                    {"id": 1001, "display_title": title}, duplicate,
                ]
        pages = [rows[offset:offset + 100] for offset in range(0, len(rows), 100)]
        responses = []
        for index, batch in enumerate(pages, start=1):
            next_link = None
            if index < len(pages):
                next_link = (
                    f'<https://api.github.com/repos/eng-cc/oasis7/actions/workflows/'
                    f'{workflow_id}/runs?per_page=100&page={index + 1}>; rel="next"'
                )
            payload = readback.json.dumps(
                {"total_count": len(rows), "workflow_runs": batch},
                separators=(",", ":"),
            ).encode("utf-8")
            responses.append(http_response(payload, link=next_link))
        with patch.object(readback.subprocess, "check_output", side_effect=responses):
            pages = readback.GitHubReadOnly().workflow_run_pages(workflow_id)
        complete = readback.contract.collect_workflow_runs(pages)
        self.assertEqual(1002, len(complete))
        with self.assertRaisesRegex(readback.contract.ContractError, "unique"):
            readback.contract.select_unique_run(complete, authority)

    def test_pagination_cycle_fails_closed(self):
        workflow_id = 230018940
        payload = readback.json.dumps(
            {"total_count": 1, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        repeated = (
            f'<https://api.github.com/repos/eng-cc/oasis7/actions/workflows/{workflow_id}'
            '/runs?per_page=100&page=1>; rel="next"'
        )
        response = http_response(payload, link=repeated)
        with patch.object(readback.subprocess, "check_output", return_value=response):
            with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
                readback.GitHubReadOnly().workflow_run_pages(workflow_id)

    def test_unexpected_next_origin_fails_closed(self):
        workflow_id = 230018940
        payload = readback.json.dumps(
            {"total_count": 1, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        link = '<https://example.invalid/page>; rel="next"'
        with patch.object(readback.subprocess, "check_output", return_value=http_response(payload, link=link)):
            with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
                readback.GitHubReadOnly().workflow_run_pages(workflow_id)

    def test_collection_count_and_page_state_must_be_complete(self):
        row = {"id": 1}
        with self.assertRaisesRegex(readback.ReadbackError, "total_count"):
            readback._collection_rows(
                ({"total_count": 2, "jobs": [row], "has_next": False},),
                "jobs", "jobs",
            )
        with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
            readback._collection_rows(
                ({"total_count": 1, "jobs": [row], "has_next": True},),
                "jobs", "jobs",
            )

    def test_reader_cli_rejects_caller_supplied_ids(self):
        with patch.object(readback.sys, "argv", ["reader.py", "--run-id", "700"]):
            with self.assertRaisesRegex(SystemExit, "accepts no arguments"):
                readback.main()


def _marked(marker: str, value: dict) -> str:
    return marker + "\n" + readback.contract.canonical_json_bytes(value).decode("ascii")


def readback_fixture():
    """Build a complete live-shaped fake route without using caller IDs as authority."""
    c = readback.contract
    task_uid = "task_" + "a" * 32
    request_context = c.TrustedRequestContext(
        task_uid=task_uid, head_oid="1" * 40, source_scope_oid="2" * 40,
        projection_digest="sha256:" + "3" * 64,
        planner_unit_ids=("required_gate_baseline", "workflow_governance"),
        planner_unit_obligations={
            "required_gate_baseline": ("required_gate_baseline:000:baseline",),
            "workflow_governance": ("workflow_governance:000:contracts",),
        },
    )
    authorization = {
        "schema": c.AUTHORIZATION_SCHEMA, "repository": c.REPOSITORY,
        "task_uid": task_uid, "task_issue_number": c.TASK_ISSUE_NUMBER,
        "pr_number": c.PR_NUMBER, "head_oid": request_context.head_oid,
        "integration_base_oid": "4" * 40, "source_scope_oid": request_context.source_scope_oid,
        "projection_digest": request_context.projection_digest,
        "validation_units": list(request_context.planner_unit_ids),
        "purpose": c.PURPOSE, "decision": c.REQUEST_DECISION,
    }
    authorization_body = _marked(c.AUTHORIZATION_MARKER, authorization)
    request = {
        "schema": c.REQUEST_SCHEMA, "repository": c.REPOSITORY,
        "task_uid": task_uid, "task_issue_number": c.TASK_ISSUE_NUMBER,
        "pr_number": c.PR_NUMBER, "head_oid": request_context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": request_context.source_scope_oid,
        "projection_digest": request_context.projection_digest,
        "validation_units": list(request_context.planner_unit_ids),
        "purpose": c.PURPOSE, "authorization_decision": c.REQUEST_DECISION,
        "authorization_source": {
            "issue_number": c.TASK_ISSUE_NUMBER, "comment_id": 10,
            "body_digest": c.body_digest(authorization_body),
        },
        "authorized_actor": "approval-admin",
    }
    request["request_digest"] = c.request_digest(request)
    request_body = _marked(c.REQUEST_MARKER, request)
    pin = {
        "schema": c.PIN_SCHEMA, "repository": c.REPOSITORY, "task_uid": task_uid,
        "task_issue_number": c.TASK_ISSUE_NUMBER, "pr_number": c.PR_NUMBER,
        "request_comment_id": 20, "request_body_digest": c.body_digest(request_body),
        "request_digest": request["request_digest"], "purpose": c.PIN_PURPOSE,
    }
    pin_body = _marked(c.PIN_MARKER, pin)
    comments = [
        {"id": 10, "body": authorization_body, "user": {"login": "approval-admin"},
         "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
        {"id": 20, "body": request_body, "user": {"login": "requester"},
         "created_at": "2026-01-01T00:01:00Z", "updated_at": "2026-01-01T00:01:00Z"},
        {"id": 30, "body": pin_body, "user": {"login": "pin-admin"},
         "created_at": "2026-01-01T00:02:00Z", "updated_at": "2026-01-01T00:02:00Z"},
    ]
    permissions = {
        "approval-admin": {"login": "approval-admin", "permission": "admin"},
        "pin-admin": {"login": "pin-admin", "permission": "admin"},
    }
    issued = c.resolve_records(comments, permissions)
    issued = c.bind_authority_context(issued, request_context)
    run = {
        "repository": c.REPOSITORY, "id": 700, "workflow_id": 900,
        "workflow_path": c.WORKFLOW_PATH, "workflow_ref": c.WORKFLOW_REF,
        "workflow_sha": "5" * 40, "event": "workflow_dispatch",
        "display_title": c.expected_run_title(issued),
        "dispatched_head_sha": "5" * 40, "run_attempt": 1,
        "status": "completed", "conclusion": "success",
        "created_at": "2026-01-01T00:03:00Z", "head_branch": "main",
        "head_repository": {"full_name": c.REPOSITORY},
        "tested_merge_oid": "6" * 40, "tested_tree_oid": "7" * 40,
    }
    run_api = {
        **run, "path": c.WORKFLOW_PATH, "head_sha": run["workflow_sha"],
        "repository": {"full_name": c.REPOSITORY},
    }
    check = {
        "name": c.CHECK_NAME, "id": 801, "app_id": c.GITHUB_ACTIONS_APP_ID,
        "head_sha": run["dispatched_head_sha"], "run_id": run["id"],
        "run_attempt": run["run_attempt"], "status": "completed", "conclusion": "success",
    }
    authority_record = c.build_authority_record(issued, run)
    payload = {
        **authority_record, "schema": c.PAYLOAD_SCHEMA,
        "run_attempt": run["run_attempt"], "check_name": check["name"],
        "check_run_id": check["id"], "check_app_id": check["app_id"],
        "selected_obligations": {key: list(value) for key, value in request_context.planner_unit_obligations.items()},
        "result_digests": {unit: "sha256:" + str(index) * 64
                           for index, unit in enumerate(request["validation_units"], start=1)},
        "authority_digest": c.authority_digest(authority_record),
        "capability_under_test": c.CAPABILITY,
        "tested_merge_oid": run["tested_merge_oid"], "tested_tree_oid": run["tested_tree_oid"],
        "event_inputs": c.expected_event_inputs(issued),
        "run_id": run["id"], "workflow_id": run["workflow_id"],
        "workflow_path": run["workflow_path"], "workflow_ref": run["workflow_ref"],
        "workflow_sha": run["workflow_sha"], "display_title": run["display_title"],
        "event": run["event"], "dispatched_head_sha": run["dispatched_head_sha"],
    }
    payload_bytes = c.canonical_json_bytes(payload)
    archive_io = io.BytesIO()
    with zipfile.ZipFile(archive_io, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr(c.PAYLOAD_MEMBER, payload_bytes)
    archive_bytes = archive_io.getvalue()
    artifact = {
        "id": 910,
        "name": c.artifact_name(issued.validation_id, run["id"], run["run_attempt"]),
        "expired": False,
        "workflow_run": {"id": run["id"], "head_sha": run["dispatched_head_sha"]},
    }
    task_body = "<!-- oasis7-pm-task -->\ntask_uid: " + task_uid + "\n"
    pr = {
        "number": c.PR_NUMBER, "state": "open", "merged": False,
        "body": f"Task: {task_uid}\nRefs #{c.TASK_ISSUE_NUMBER}\n",
        "head": {"sha": request_context.head_oid,
                 "repo": {"full_name": c.REPOSITORY, "id": 1234}},
        "base": {"sha": request["integration_base_oid"], "ref": "main",
                 "repo": {"full_name": c.REPOSITORY, "id": 1234}},
    }
    issue = {"number": c.TASK_ISSUE_NUMBER, "body": task_body}
    return {
        "context": request_context, "request": request, "comments": comments,
        "run": run, "run_api": run_api, "check": check, "payload": payload,
        "payload_bytes": payload_bytes, "archive_bytes": archive_bytes,
        "artifact": artifact, "issue": issue, "pr": pr,
    }


class FakeReadbackAPI:
    def __init__(self, fixture):
        self.fixture = fixture
        self.run_listing_count = 0
        self.run_direct_count = 0
        self.comment_listing_count = 0
        self.duplicate_second_run = False
        self.advance_second_attempt = False
        self.mutate_final_comment = False
        self.duplicate_artifact = False
        self.write_calls = 0

    def get_json(self, endpoint):
        if endpoint == f"repos/eng-cc/oasis7/issues/{readback.TASK_ISSUE_NUMBER}":
            return self.fixture["issue"]
        if endpoint == f"repos/eng-cc/oasis7/pulls/{readback.PR_NUMBER}":
            return self.fixture["pr"]
        if endpoint == "repos/eng-cc/oasis7":
            return {"default_branch": "main"}
        if endpoint == "repos/eng-cc/oasis7/actions/workflows/rust.yml":
            return {"id": self.fixture["run"]["workflow_id"],
                    "path": readback.WORKFLOW_FILE, "state": "active"}
        if endpoint == f"repos/eng-cc/oasis7/actions/runs/{self.fixture['run']['id']}":
            self.run_direct_count += 1
            value = dict(self.fixture["run_api"])
            if self.advance_second_attempt and self.run_direct_count >= 2:
                value["run_attempt"] = 2
            return value
        if endpoint == "repos/eng-cc/oasis7/check-runs/801":
            check = self.fixture["check"]
            return {"id": check["id"], "name": check["name"],
                    "app": {"id": check["app_id"]}, "head_sha": check["head_sha"],
                    "status": check["status"], "conclusion": check["conclusion"]}
        raise AssertionError(f"unexpected GitHub read: {endpoint}")

    def issue_comment_pages(self):
        self.comment_listing_count += 1
        comments = [dict(item) for item in self.fixture["comments"]]
        if self.mutate_final_comment and self.comment_listing_count >= 2:
            comments[-1]["body"] += " edited"
            comments[-1]["updated_at"] = "2026-01-01T00:02:30Z"
        return (comments,)

    def workflow_run_pages(self, workflow_id):
        self.assert_workflow_id(workflow_id)
        self.run_listing_count += 1
        rows = [dict(self.fixture["run_api"])]
        if self.duplicate_second_run and self.run_listing_count >= 2:
            duplicate = dict(rows[0])
            duplicate["id"] += 1
            duplicate["display_title"] += "|other"
            rows.append(duplicate)
        return ({"total_count": len(rows), "runs": rows, "has_next": False},)

    def assert_workflow_id(self, value):
        if value != self.fixture["run"]["workflow_id"]:
            raise AssertionError("workflow ID mismatch")

    def job_pages(self, run_id, attempt):
        check = self.fixture["check"]
        job = {
            "id": 802, "name": check["name"], "run_id": run_id,
            "run_attempt": attempt, "head_sha": check["head_sha"],
            "check_run_url": "https://api.github.com/repos/eng-cc/oasis7/check-runs/801",
        }
        return ({"total_count": 1, "jobs": [job], "has_next": False},)

    def artifact_pages(self, run_id):
        rows = [self.fixture["artifact"]]
        if self.duplicate_artifact:
            rows = [dict(rows[0]), {**rows[0], "id": rows[0]["id"] + 1,
                                    "name": rows[0]["name"] + "-duplicate"}]
        return ({"total_count": len(rows), "artifacts": rows, "has_next": False},)

    def get_bytes(self, endpoint):
        expected = f"repos/eng-cc/oasis7/actions/artifacts/{self.fixture['artifact']['id']}/zip"
        if endpoint != expected:
            raise AssertionError(f"unexpected GitHub binary read: {endpoint}")
        return self.fixture["archive_bytes"]


class IndependentReadbackTests(unittest.TestCase):
    def _run(self, api):
        with patch.object(
            readback, "_trusted_inventory",
            return_value=(self.fixture["context"], "6" * 40, "7" * 40),
        ):
            return readback.read_validation(api)

    def setUp(self):
        self.fixture = readback_fixture()

    def test_full_live_readback_passes_without_accepting_caller_ids_or_writes(self):
        api = FakeReadbackAPI(self.fixture)
        envelope = self._run(api)
        self.assertEqual(readback.contract.READBACK_SCHEMA, envelope["schema"])
        self.assertEqual(self.fixture["run"]["id"], envelope["run_id"])
        self.assertEqual(2, api.comment_listing_count)
        self.assertEqual(2, api.run_listing_count)
        self.assertEqual(2, api.run_direct_count)
        self.assertEqual(0, api.write_calls)

    def test_final_full_run_enumeration_rejects_a_second_candidate_id(self):
        api = FakeReadbackAPI(self.fixture)
        api.duplicate_second_run = True
        with self.assertRaisesRegex(readback.contract.ContractError, "unique"):
            self._run(api)

    def test_final_live_run_rejects_latest_attempt_advance(self):
        api = FakeReadbackAPI(self.fixture)
        api.advance_second_attempt = True
        with self.assertRaisesRegex(readback.ReadbackError, "latest R/A"):
            self._run(api)

    def test_final_issue_read_rejects_changed_pinned_comment(self):
        api = FakeReadbackAPI(self.fixture)
        api.mutate_final_comment = True
        with self.assertRaises(readback.contract.ContractError):
            self._run(api)

    def test_exact_attempt_rejects_a_second_artifact_alias(self):
        api = FakeReadbackAPI(self.fixture)
        api.duplicate_artifact = True
        with patch.object(
            readback, "_trusted_inventory",
            return_value=(self.fixture["context"], "6" * 40, "7" * 40),
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "artifact"):
                readback.read_validation(api)

    def test_exact_latest_attempt_ignores_an_artifact_from_an_older_attempt(self):
        c = readback.contract
        run = dict(self.fixture["run"])
        run["run_attempt"] = 2
        current = dict(self.fixture["artifact"])
        current["name"] = c.artifact_name(
            c.resolve_records_for_readback(self.fixture["comments"]).validation_id,
            run["id"], run["run_attempt"],
        )
        older = {**current, "id": current["id"] + 1,
                 "name": c.artifact_name(
                     c.resolve_records_for_readback(self.fixture["comments"]).validation_id,
                     run["id"], 1,
                 )}

        class API:
            def artifact_pages(self, run_id):
                if run_id != run["id"]:
                    raise AssertionError("artifact enumeration used another run")
                return ({"total_count": 2, "artifacts": [older, current], "has_next": False},)

        selected = readback._exact_artifact(
            API(), run, c.resolve_records_for_readback(self.fixture["comments"]),
        )
        self.assertEqual(current["id"], selected["id"])

    def test_zip_requires_one_fixed_member_and_payload_bytes_are_canonical(self):
        c = readback.contract
        archive_io = io.BytesIO()
        with zipfile.ZipFile(archive_io, "w") as archive:
            archive.writestr(c.PAYLOAD_MEMBER, b"{}")
            archive.writestr("extra.json", b"{}")
        with self.assertRaisesRegex(readback.ReadbackError, "exactly the fixed"):
            readback._payload_from_archive(archive_io.getvalue())
        with self.assertRaisesRegex(readback.ReadbackError, "canonical"):
            readback._parse_payload_bytes(b'{"a":1}\n')
        with self.assertRaisesRegex(readback.ReadbackError, "duplicate"):
            readback._parse_payload_bytes(b'{"a":1,"a":1}')


if __name__ == "__main__":
    unittest.main()
