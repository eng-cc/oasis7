#!/usr/bin/env python3
"""Focused tests for independent V1 validation readback."""

from __future__ import annotations

import ast
import importlib.util
import inspect
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch
import zipfile


MODULE_PATH = Path(__file__).with_name("ci_reuse_validation_readback.py")
FIXTURE_ISSUE_NUMBER = 87
FIXTURE_PR_NUMBER = 143
SPEC = importlib.util.spec_from_file_location("ci_reuse_validation_readback", MODULE_PATH)
readback = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(readback)


def http_response(body: bytes, *, link: str | None = None, status: str = "200 OK") -> bytes:
    headers = [f"HTTP/2.0 {status}", "Content-Type: application/json"]
    if link is not None:
        headers.append(f"Link: {link}")
    return ("\r\n".join(headers) + "\r\n\r\n").encode("ascii") + body


def patch_live_workflow_metadata(api, *, repository_id=1148737145, workflow_id=230018940):
    responses = {
        "repos/eng-cc/oasis7": {
            "id": repository_id, "name": "oasis7", "full_name": "eng-cc/oasis7",
            "owner": {"login": "eng-cc"}, "default_branch": "main",
        },
        "repos/eng-cc/oasis7/actions/workflows/rust.yml": {
            "id": workflow_id, "path": readback.WORKFLOW_FILE, "state": "active",
        },
    }

    def get_json(endpoint):
        if endpoint not in responses:
            raise AssertionError(f"unexpected live identity read: {endpoint}")
        return responses[endpoint]

    return patch.object(api, "get_json", side_effect=get_json)


class SuccessorAdapterTests(unittest.TestCase):
    def test_v1_records_stay_on_existing_readback_resolver(self):
        api = Mock()
        comments = ({"id": 1, "body": "ordinary V1 records"},)
        identity = {
            "task_uid": "task_" + "1" * 32, "task_issue_number": 87,
            "pr_number": 143, "head_oid": "a" * 40, "integration_base_oid": "b" * 40,
        }
        authority = object()
        with patch.object(readback.contract, "resolve_records_for_readback", return_value=authority) as resolve:
            actual, predecessor = readback._resolve_records_with_predecessor(
                api, comments, identity, 17, "main", 18,
            )
        self.assertIs(actual, authority)
        self.assertIsNone(predecessor)
        resolve.assert_called_once_with(comments, expected_identity=identity)

    def test_v2_marker_requires_predecessor_observation_and_successor_resolver(self):
        api = Mock()
        identity = {
            "task_uid": "task_" + "1" * 32, "task_issue_number": 87,
            "pr_number": 143, "head_oid": "a" * 40, "integration_base_oid": "b" * 40,
        }
        comments = ({"id": 1, "body": readback.contract.SUCCESSOR_REQUEST_MARKER},)
        predecessor = {"run_id": 101, "run_terminal_updated_at": "2026-01-01T00:00:00Z"}
        authority = object()
        with patch.object(readback, "_predecessor_observation", return_value=predecessor) as observe, patch.object(
            readback.contract, "resolve_successor_records_for_readback", return_value=authority,
        ) as resolve:
            actual, actual_predecessor = readback._resolve_records_with_predecessor(
                api, comments, identity, 17, "main", 18,
            )
        self.assertIs(actual, authority)
        self.assertIs(actual_predecessor, predecessor)
        observe.assert_called_once_with(
            api, comments,
            {"task_uid": identity["task_uid"], "task_issue_number": 87, "pr_number": 143},
            17, "main", 18,
        )
        resolve.assert_called_once_with(comments, identity, predecessor)

    def test_predecessor_workflow_blob_is_hashed_at_exact_run_head(self):
        api = Mock()
        content = (
            "run-name: oasis7-ci|${{ github.event_name }}|${{ inputs.run_mode }}|"
            "${{ inputs.task_uid }}|${{ inputs.pr_number }}|${{ inputs.integration_base }}|"
            "${{ inputs.expected_head }}${{ inputs.request_key != '' && format('|{0}', inputs.request_key) || '' }}\n"
        ).encode()
        oid = readback.hashlib.sha1(
            b"blob " + str(len(content)).encode() + b"\0" + content,
        ).hexdigest()
        response = {
            "path": readback.WORKFLOW_FILE, "encoding": "base64",
            "content": readback.base64.b64encode(content).decode(), "sha": oid,
        }
        with patch.object(api, "get_json", return_value=response) as get_json:
            self.assertEqual((oid, readback.contract.body_digest(content)),
                             readback._workflow_blob_at(api, "c" * 40))
        get_json.assert_called_once_with(
            f"repos/{readback.REPOSITORY}/contents/{readback.WORKFLOW_FILE}?ref={'c' * 40}",
        )
        response["sha"] = "d" * 40
        with patch.object(api, "get_json", return_value=response):
            with self.assertRaises(readback.ReadbackError):
                readback._workflow_blob_at(api, "c" * 40)


class GitHubPaginationTests(unittest.TestCase):
    def test_task_issue_discovery_follows_every_state_all_rest_page(self):
        first_issue = {"id": 9001, "number": 87, "body": "task issue one"}
        second_issue = {"id": 9002, "number": 88, "body": "task issue two"}
        first = readback.json.dumps([first_issue], separators=(",", ":")).encode("utf-8")
        second = readback.json.dumps([second_issue], separators=(",", ":")).encode("utf-8")
        next_link = (
            "<https://api.github.com/repositories/1148737145/issues?per_page=100&page=2&state=all&after=cursor-one>; rel=\"next\""
        )
        repository = {
            "id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
            "owner": {"login": "eng-cc"},
        }
        api = readback.GitHubReadOnly()
        with patch.object(api, "get_json", return_value=repository) as repo_read, patch.object(
            readback.subprocess, "check_output", side_effect=[
                http_response(first, link=next_link), http_response(second),
            ],
        ) as call:
            pages = api.task_issue_pages()

        self.assertEqual([[first_issue], [second_issue]], list(pages))
        self.assertEqual([
            "repos/eng-cc/oasis7/issues?state=all&per_page=100",
            "repositories/1148737145/issues?per_page=100&page=2&state=all&after=cursor-one",
        ], [item.args[0][-1] for item in call.call_args_list])
        repo_read.assert_called_once_with("repos/eng-cc/oasis7")

    def test_task_issue_pagination_rejects_a_repeated_opaque_cursor(self):
        repository = {
            "id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
            "owner": {"login": "eng-cc"},
        }
        body = readback.json.dumps([], separators=(",", ":")).encode("utf-8")
        first_link = (
            "<https://api.github.com/repositories/1148737145/issues?state=all&per_page=100&after=cursor-one&page=2>; rel=\"next\""
        )
        repeated_link = (
            "<https://api.github.com/repositories/1148737145/issues?state=all&per_page=100&after=cursor-one&page=3>; rel=\"next\""
        )
        api = readback.GitHubReadOnly()
        with patch.object(api, "get_json", return_value=repository), patch.object(
            readback.subprocess, "check_output", side_effect=[
                http_response(body, link=first_link), http_response(body, link=repeated_link),
            ],
        ) as request:
            with self.assertRaisesRegex(readback.ReadbackError, "advancing opaque cursor"):
                api.task_issue_pages()
        self.assertEqual(2, request.call_count)

    def test_task_issue_pagination_rejects_another_repository_numeric_alias(self):
        repository = {
            "id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
            "owner": {"login": "eng-cc"},
        }
        body = readback.json.dumps([], separators=(",", ":")).encode("utf-8")
        next_link = (
            "<https://api.github.com/repositories/1148737146/issues?state=all&per_page=100&page=2>; rel=\"next\""
        )
        api = readback.GitHubReadOnly()
        with patch.object(api, "get_json", return_value=repository), patch.object(
            readback.subprocess, "check_output", return_value=http_response(body, link=next_link),
        ) as request:
            with self.assertRaisesRegex(readback.ReadbackError, "canonical API endpoint"):
                api.task_issue_pages()
        request.assert_called_once()

    def test_producer_run_discovery_calls_the_compatible_one_argument_api(self):
        producer_path = Path(__file__).with_name("ci-reuse-validation.py")
        producer_tree = ast.parse(producer_path.read_text(encoding="utf-8"))
        calls = [
            node for node in ast.walk(producer_tree)
            if isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "workflow_run_pages"
        ]

        self.assertEqual(2, len(calls))
        self.assertTrue(all(len(call.args) == 1 and not call.keywords for call in calls))
        self.assertTrue(all(
            isinstance(call.args[0], ast.Name) and call.args[0].id == "workflow_id"
            for call in calls
        ))
        repository_id = inspect.signature(
            readback.GitHubReadOnly.workflow_run_pages,
        ).parameters["repository_id"]
        self.assertIsNone(repository_id.default)

    def test_live_workflow_binds_canonical_repository_id_and_workflow(self):
        api = readback.GitHubReadOnly()
        with patch.object(api, "get_json", side_effect=[
            {
                "id": 1148737145,
                "name": "oasis7",
                "full_name": "eng-cc/oasis7",
                "owner": {"login": "eng-cc"},
                "default_branch": "main",
            },
            {"id": 230018940, "path": readback.WORKFLOW_FILE, "state": "active"},
        ]) as get:
            identity = readback._live_workflow(api)

        self.assertEqual((230018940, "main", 1148737145), identity)
        self.assertEqual([
            "repos/eng-cc/oasis7",
            "repos/eng-cc/oasis7/actions/workflows/rust.yml",
        ], [call.args[0] for call in get.call_args_list])

        api = readback.GitHubReadOnly()
        with patch.object(api, "get_json", side_effect=[
            {"id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
             "owner": {"login": "eng-cc"}, "default_branch": "release/ready"},
            {"id": 230018940, "path": readback.WORKFLOW_FILE, "state": "active"},
        ]):
            self.assertEqual((230018940, "release/ready", 1148737145),
                             readback._live_workflow(api))

    def test_live_workflow_rejects_wrong_repository_or_workflow_identity(self):
        bad_repositories = [
            {"id": 1148737146, "name": "oasis7", "full_name": "other/oasis7",
             "owner": {"login": "other"}, "default_branch": "main"},
            {"id": True, "name": "oasis7", "full_name": "eng-cc/oasis7",
             "owner": {"login": "eng-cc"}, "default_branch": "main"},
            {"id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
             "owner": {"login": "eng-cc"}, "default_branch": ""},
        ]
        for repository in bad_repositories:
            with self.subTest(repository=repository):
                api = readback.GitHubReadOnly()
                with patch.object(api, "get_json", side_effect=[
                    repository,
                    {"id": 230018940, "path": readback.WORKFLOW_FILE, "state": "active"},
                ]):
                    with self.assertRaisesRegex(readback.ReadbackError, "canonical repository|default branch"):
                        readback._live_workflow(api)

        bad_workflows = [
            {"id": True, "path": readback.WORKFLOW_FILE, "state": "active"},
            {"id": 0, "path": readback.WORKFLOW_FILE, "state": "active"},
            {"id": 230018940, "path": ".github/workflows/other.yml", "state": "active"},
            {"id": 230018940, "path": readback.WORKFLOW_FILE, "state": "disabled"},
        ]
        for workflow in bad_workflows:
            with self.subTest(workflow=workflow):
                api = readback.GitHubReadOnly()
                with patch.object(api, "get_json", side_effect=[
                    {
                        "id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
                        "owner": {"login": "eng-cc"}, "default_branch": "main",
                    }, workflow,
                ]):
                    with self.assertRaisesRegex(readback.ReadbackError, "canonical rust.yml workflow"):
                        readback._live_workflow(api)

    def test_live_default_branch_is_dynamic_and_raw_path_is_retained(self):
        oid = "a" * 40
        row = {
            "id": 5, "workflow_id": 230018940,
            "path": readback.WORKFLOW_FILE, "head_branch": "release/ready",
            "head_sha": oid, "run_attempt": 1,
            "repository": {"full_name": readback.REPOSITORY},
            "head_repository": {"full_name": readback.REPOSITORY},
        }
        identity = readback._run_identity(row, 230018940, "release/ready")
        self.assertEqual(readback.WORKFLOW_FILE, identity["workflow_api_path"])
        self.assertEqual(f"{readback.WORKFLOW_FILE}@release/ready", identity["workflow_path"])
        self.assertEqual(
            f"{readback.REPOSITORY}/{readback.WORKFLOW_FILE}@refs/heads/release/ready",
            identity["workflow_ref"],
        )
        self.assertEqual("refs/heads/release/ready", identity["event_ref"])
        for raw_path in (".github/workflows/rust.yml@main", ".github/workflows/other.yml",
                         ".github/workflows/rust.yml@refs/heads/release/ready"):
            with self.subTest(raw_path=raw_path), self.assertRaises(readback.ReadbackError):
                readback._run_identity({**row, "path": raw_path}, 230018940, "release/ready")

    def test_run_pages_accept_authenticated_slug_to_numeric_repository_alias(self):
        workflow_id = 230018940
        repository_id = 1148737145
        rows = [
            {"id": 1, "display_title": "first"},
            {"id": 2, "display_title": "second"},
        ]
        first = readback.json.dumps(
            {"total_count": 2, "workflow_runs": rows[:1]}, separators=(",", ":"),
        ).encode("utf-8")
        second = readback.json.dumps(
            {"total_count": 2, "workflow_runs": rows[1:]}, separators=(",", ":"),
        ).encode("utf-8")
        next_link = (
            f'<https://api.github.com/repositories/{repository_id}/actions/workflows/'
            f'{workflow_id}/runs?per_page=100&page=2>; rel="next"'
        )
        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api) as identity_read, patch.object(
            readback.subprocess, "check_output", side_effect=[
                http_response(first, link=next_link), http_response(second),
            ],
        ) as call:
            # This is the exact one-argument call used by the validation producer.
            page_values = api.workflow_run_pages(workflow_id)

        self.assertEqual([[rows[0]], [rows[1]]], [page["runs"] for page in page_values])
        self.assertEqual([2, 2], [page["total_count"] for page in page_values])
        self.assertEqual([True, False], [page["has_next"] for page in page_values])
        self.assertEqual([
            "repos/eng-cc/oasis7",
            "repos/eng-cc/oasis7/actions/workflows/rust.yml",
        ], [call.args[0] for call in identity_read.call_args_list])
        self.assertEqual(
            [
                f"repos/eng-cc/oasis7/actions/workflows/{workflow_id}/runs?per_page=100",
                f"repositories/{repository_id}/actions/workflows/{workflow_id}/runs?per_page=100&page=2",
            ],
            [invocation.args[0][-1] for invocation in call.call_args_list],
        )

    def test_run_pages_reject_supplied_repository_id_that_differs_from_live_id(self):
        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api):
            with self.assertRaisesRegex(readback.ReadbackError, "differs from live repository"):
                api.workflow_run_pages(230018940, 1148737146)

    def test_run_pagination_rejects_noncanonical_repository_or_workflow_alias(self):
        workflow_id = 230018940
        repository_id = 1148737145
        bad_targets = [
            f"https://api.github.com/repositories/{repository_id + 1}/actions/workflows/{workflow_id}/runs?per_page=100&page=2",
            f"https://api.github.com/repositories/{repository_id}/actions/workflows/{workflow_id + 1}/runs?per_page=100&page=2",
            f"https://api.github.com/repos/other/oasis7/actions/workflows/{workflow_id}/runs?per_page=100&page=2",
        ]
        payload = readback.json.dumps(
            {"total_count": 2, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        for target in bad_targets:
            with self.subTest(target=target):
                api = readback.GitHubReadOnly()
                with patch_live_workflow_metadata(api), patch.object(
                    readback.subprocess, "check_output", return_value=http_response(
                        payload, link=f'<{target}>; rel="next"',
                    ),
                ):
                    with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
                        api.workflow_run_pages(workflow_id, repository_id)

    def test_run_pagination_rejects_extra_mutated_or_nonsequential_query(self):
        workflow_id = 230018940
        repository_id = 1148737145
        bad_queries = [
            "per_page=99&page=2",
            "per_page=100&page=2&event=workflow_dispatch",
            "per_page=100&page=3",
            "per_page=100&page=02",
            "page=2&per_page=100",
        ]
        payload = readback.json.dumps(
            {"total_count": 2, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        for query in bad_queries:
            with self.subTest(query=query):
                target = (
                    f"https://api.github.com/repositories/{repository_id}/actions/workflows/"
                    f"{workflow_id}/runs?{query}"
                )
                api = readback.GitHubReadOnly()
                with patch_live_workflow_metadata(api), patch.object(
                    readback.subprocess, "check_output", return_value=http_response(
                        payload, link=f'<{target}>; rel="next"',
                    ),
                ):
                    with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
                        api.workflow_run_pages(workflow_id, repository_id)

    def test_run_pagination_rejects_duplicate_next_relations_and_invalid_identity_values(self):
        workflow_id = 230018940
        repository_id = 1148737145
        payload = readback.json.dumps(
            {"total_count": 2, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        link = (
            f'<https://api.github.com/repositories/{repository_id}/actions/workflows/{workflow_id}'
            '/runs?per_page=100&page=2>; rel="next", '
            f'<https://api.github.com/repos/eng-cc/oasis7/actions/workflows/{workflow_id}'
            '/runs?per_page=100&page=2>; rel="next"'
        )
        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api), patch.object(
            readback.subprocess, "check_output", return_value=http_response(
                payload, link=link,
            ),
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "multiple next"):
                api.workflow_run_pages(workflow_id, repository_id)

        for invalid_workflow_id, invalid_repository_id in (
            (True, repository_id), (workflow_id, True), (workflow_id, 0),
        ):
            with self.subTest(
                workflow_id=invalid_workflow_id, repository_id=invalid_repository_id,
            ):
                with self.assertRaisesRegex(readback.ReadbackError, "ID is invalid"):
                    readback.GitHubReadOnly().workflow_run_pages(
                        invalid_workflow_id, invalid_repository_id,
                    )

    def test_unfiltered_run_pages_continue_past_search_ceiling(self):
        workflow_id = 230018940
        repository_id = 1148737145
        rows = [{"id": index, "display_title": f"unrelated-{index}"}
                for index in range(1, 1002)]
        pages = [rows[offset:offset + 100] for offset in range(0, len(rows), 100)]
        responses = []
        for index, batch in enumerate(pages, start=1):
            next_link = None
            if index < len(pages):
                next_link = (
                    f'<https://api.github.com/repositories/{repository_id}/actions/workflows/'
                    f'{workflow_id}/runs?per_page=100&page={index + 1}>; rel="next"'
                )
            payload = readback.json.dumps(
                {"total_count": len(rows), "workflow_runs": batch},
                separators=(",", ":"),
            ).encode("utf-8")
            responses.append(http_response(payload, link=next_link))

        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api), patch.object(
            readback.subprocess, "check_output", side_effect=responses,
        ) as call:
            page_values = api.workflow_run_pages(workflow_id, repository_id)

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
        target = {
            "id": 1001, "display_title": title,
            "status": "completed", "conclusion": "success",
        }
        different_action = {"id": 1002, "display_title": title + "|different-key"}
        rows = [{"id": index, "display_title": f"unrelated-{index}"}
                for index in range(1, 1001)] + [target, different_action]
        pages = [rows[offset:offset + 100] for offset in range(0, len(rows), 100)]
        responses = []
        for index, batch in enumerate(pages, start=1):
            next_link = None
            if index < len(pages):
                next_link = (
                    f'<https://api.github.com/repositories/1148737145/actions/workflows/'
                    f'{workflow_id}/runs?per_page=100&page={index + 1}>; rel="next"'
                )
            payload = readback.json.dumps(
                {"total_count": len(rows), "workflow_runs": batch},
                separators=(",", ":"),
            ).encode("utf-8")
            responses.append(http_response(payload, link=next_link))
        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api), patch.object(
            readback.subprocess, "check_output", side_effect=responses,
        ):
            pages = api.workflow_run_pages(workflow_id, 1148737145)
        complete = readback.contract.collect_workflow_runs(pages)
        self.assertEqual(1002, len(complete))
        self.assertEqual(
            target["id"], readback.contract.select_unique_run(complete, authority)["id"],
        )

    def test_pagination_cycle_fails_closed(self):
        workflow_id = 230018940
        payload = readback.json.dumps(
            {"total_count": 1, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        repeated = (
            f'<https://api.github.com/repositories/1148737145/actions/workflows/{workflow_id}'
            '/runs?per_page=100&page=1>; rel="next"'
        )
        response = http_response(payload, link=repeated)
        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api), patch.object(
            readback.subprocess, "check_output", return_value=response,
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
                api.workflow_run_pages(workflow_id, 1148737145)

    def test_unexpected_next_origin_fails_closed(self):
        workflow_id = 230018940
        payload = readback.json.dumps(
            {"total_count": 1, "workflow_runs": [{"id": 1, "display_title": "x"}]},
            separators=(",", ":"),
        ).encode("utf-8")
        link = '<https://example.invalid/page>; rel="next"'
        api = readback.GitHubReadOnly()
        with patch_live_workflow_metadata(api), patch.object(
            readback.subprocess, "check_output", return_value=http_response(payload, link=link),
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "pagination"):
                api.workflow_run_pages(workflow_id, 1148737145)

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

    def test_reader_cli_accepts_only_the_task_uid_lookup_key(self):
        with patch.object(readback.sys, "argv", ["reader.py", "--run-id", "700"]):
            with self.assertRaisesRegex(SystemExit, "requires --task-uid"):
                readback.main()
        with patch.object(readback.sys, "argv", ["reader.py", "--issue-number", "87"]):
            with self.assertRaisesRegex(SystemExit, "requires --task-uid"):
                readback.main()


def _marked(marker: str, value: dict) -> str:
    return marker + "\n" + readback.contract.canonical_json_bytes(value).decode("ascii")


def readback_fixture():
    """Build a complete live-shaped fake route without using caller IDs as authority."""
    c = readback.contract
    task_uid = "task_" + "a" * 32
    request_context = c.TrustedRequestContext(
        task_uid=task_uid, task_issue_number=FIXTURE_ISSUE_NUMBER,
        pr_number=FIXTURE_PR_NUMBER, head_oid="1" * 40, source_scope_oid="2" * 40,
        projection_digest="sha256:" + "3" * 64,
        planner_unit_ids=("required_gate_baseline", "workflow_governance"),
        planner_unit_obligations={
            "required_gate_baseline": ("required_gate_baseline:000:baseline",),
            "workflow_governance": ("workflow_governance:000:contracts",),
        },
    )
    authorization = {
        "schema": c.AUTHORIZATION_SCHEMA, "repository": c.REPOSITORY,
        "task_uid": task_uid, "task_issue_number": FIXTURE_ISSUE_NUMBER,
        "pr_number": FIXTURE_PR_NUMBER, "head_oid": request_context.head_oid,
        "integration_base_oid": "4" * 40, "source_scope_oid": request_context.source_scope_oid,
        "projection_digest": request_context.projection_digest,
        "validation_units": list(request_context.planner_unit_ids),
        "purpose": c.PURPOSE, "decision": c.REQUEST_DECISION,
    }
    authorization_body = _marked(c.AUTHORIZATION_MARKER, authorization)
    request = {
        "schema": c.REQUEST_SCHEMA, "repository": c.REPOSITORY,
        "task_uid": task_uid, "task_issue_number": FIXTURE_ISSUE_NUMBER,
        "pr_number": FIXTURE_PR_NUMBER, "head_oid": request_context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": request_context.source_scope_oid,
        "projection_digest": request_context.projection_digest,
        "validation_units": list(request_context.planner_unit_ids),
        "purpose": c.PURPOSE, "authorization_decision": c.REQUEST_DECISION,
        "authorization_source": {
            "issue_number": FIXTURE_ISSUE_NUMBER, "comment_id": 10,
            "body_digest": c.body_digest(authorization_body),
        },
        "authorized_actor": "approval-admin",
    }
    request["request_digest"] = c.request_digest(request)
    request_body = _marked(c.REQUEST_MARKER, request)
    pin = {
        "schema": c.PIN_SCHEMA, "repository": c.REPOSITORY, "task_uid": task_uid,
        "task_issue_number": FIXTURE_ISSUE_NUMBER, "pr_number": FIXTURE_PR_NUMBER,
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
        "workflow_api_path": c.WORKFLOW_PATH, "workflow_default_branch": "main",
        "workflow_path": c.WORKFLOW_PATH, "workflow_ref": c.WORKFLOW_REF,
        "event_ref": "refs/heads/main",
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
    task_body = (
        "<!-- oasis7-pm-task -->\n"
        f"task_uid: {task_uid}\n"
        f"- pr_url: `https://github.com/{c.REPOSITORY}/pull/{FIXTURE_PR_NUMBER}`\n"
        f"- pr_number: `{FIXTURE_PR_NUMBER}`\n"
    )
    issue_url = f"https://github.com/{c.REPOSITORY}/issues/{FIXTURE_ISSUE_NUMBER}"
    pr = {
        "number": FIXTURE_PR_NUMBER, "state": "open", "merged": False,
        "body": f"Task: {task_uid}\nRefs #{FIXTURE_ISSUE_NUMBER}\n",
        "head": {"sha": request_context.head_oid,
                 "repo": {"full_name": c.REPOSITORY, "id": 1148737145}},
        "base": {"sha": request["integration_base_oid"], "ref": "main",
                 "repo": {"full_name": c.REPOSITORY, "id": 1148737145}},
    }
    issue = {"number": FIXTURE_ISSUE_NUMBER, "body": task_body, "html_url": issue_url}
    project_issue = {"number": FIXTURE_ISSUE_NUMBER, "body": task_body,
                     "url": issue_url, "project_item_id": "PVTI_fixture"}
    return {
        "context": request_context, "request": request, "comments": comments,
        "run": run, "run_api": run_api, "check": check, "payload": payload,
        "payload_bytes": payload_bytes, "archive_bytes": archive_bytes,
        "artifact": artifact, "issue": issue, "project_issue": project_issue,
        "pr": pr,
    }


class ProjectTaskResolutionTests(unittest.TestCase):
    def _payload(self, *, issue_body: str | None = None, uid_field: str | None = None,
                 page_more: bool = False, issue_count: int = 1,
                 project_items: list[dict] | None = None) -> dict:
        uid = "task_" + "a" * 32
        issue_number = FIXTURE_ISSUE_NUMBER
        issue_url = f"https://github.com/{readback.REPOSITORY}/issues/{issue_number}"
        body = issue_body or (
            "<!-- oasis7-pm-task -->\n" + f"task_uid: {uid}\n"
            + f"- pr_url: `https://github.com/{readback.REPOSITORY}/pull/{FIXTURE_PR_NUMBER}`\n"
            + f"- pr_number: `{FIXTURE_PR_NUMBER}`\n"
        )
        item = {
            "id": "PVTI_fixture",
            "project": {"number": readback.PROJECT_NUMBER,
                        "owner": {"login": readback.PROJECT_OWNER}},
            "content": {"number": issue_number, "url": issue_url},
            "fieldValues": {
                "pageInfo": {"hasNextPage": False},
                "nodes": [{"text": uid_field or uid,
                           "field": {"name": "Task UID"}}],
            },
        }
        return {
            "data": {"search": {
                "issueCount": issue_count,
                "pageInfo": {"hasNextPage": page_more},
                "nodes": [{"number": issue_number, "url": issue_url, "body": body,
                           "projectItems": {"pageInfo": {"hasNextPage": False},
                                            "nodes": project_items if project_items is not None else [item]}}],
            }},
        }

    def test_project_task_resolution_requires_exact_complete_uid_and_issue(self):
        api = readback.GitHubReadOnly()
        uid = "task_" + "a" * 32
        with patch.object(api, "graphql", return_value=self._payload()):
            issue = api.resolve_project_task_issue(uid)
        self.assertEqual(FIXTURE_ISSUE_NUMBER, issue["number"])
        self.assertEqual("PVTI_fixture", issue["project_item_id"])

    def test_project_task_resolution_fails_closed_on_incomplete_or_ambiguous_mapping(self):
        uid = "task_" + "a" * 32
        invalid_payloads = [
            self._payload(issue_count=2),
            self._payload(page_more=True),
            self._payload(project_items=[]),
            self._payload(uid_field="task_" + "b" * 32),
            self._payload(project_items=[
                self._payload()["data"]["search"]["nodes"][0]["projectItems"]["nodes"][0],
                self._payload()["data"]["search"]["nodes"][0]["projectItems"]["nodes"][0],
            ]),
        ]
        for payload in invalid_payloads:
            api = readback.GitHubReadOnly()
            with self.subTest(payload=payload), patch.object(api, "graphql", return_value=payload):
                with self.assertRaises(readback.ReadbackError):
                    api.resolve_project_task_issue(uid)

    def test_project_read_permission_failure_is_not_replaced_by_issue_or_pr_inputs(self):
        api = readback.GitHubReadOnly()
        with patch.object(
            readback.subprocess, "check_output",
            side_effect=readback.subprocess.CalledProcessError(1, ["gh"]),
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "lacks read permission"):
                api.resolve_project_task_issue("task_" + "a" * 32)


class FakeReadbackAPI:
    def __init__(self, fixture):
        self.fixture = fixture
        self.run_listing_count = 0
        self.run_direct_count = 0
        self.comment_listing_count = 0
        self.project_lookup_count = 0
        self.change_project_on_final_read = False
        self.duplicate_second_run = False
        self.advance_second_attempt = False
        self.mutate_final_comment = False
        self.duplicate_artifact = False
        self.write_calls = 0

    def get_json(self, endpoint):
        if endpoint == f"repos/eng-cc/oasis7/issues/{FIXTURE_ISSUE_NUMBER}":
            return self.fixture["issue"]
        if endpoint == f"repos/eng-cc/oasis7/pulls/{FIXTURE_PR_NUMBER}":
            return self.fixture["pr"]
        if endpoint == "repos/eng-cc/oasis7":
            return {
                "id": 1148737145, "name": "oasis7", "full_name": "eng-cc/oasis7",
                "owner": {"login": "eng-cc"}, "default_branch": "main",
            }
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

    def resolve_project_task_issue(self, task_uid):
        self.project_lookup_count += 1
        if task_uid != self.fixture["context"].task_uid:
            raise readback.ReadbackError("Task UID is not bound to this Project item")
        value = dict(self.fixture["project_issue"])
        if self.change_project_on_final_read and self.project_lookup_count >= 2:
            value["project_item_id"] = "PVTI_changed"
        return value

    def issue_comment_pages(self, issue_number):
        if issue_number != FIXTURE_ISSUE_NUMBER:
            raise AssertionError("comment read used an unbound Task Issue number")
        self.comment_listing_count += 1
        comments = [dict(item) for item in self.fixture["comments"]]
        if self.mutate_final_comment and self.comment_listing_count >= 2:
            comments[-1]["body"] += " edited"
            comments[-1]["updated_at"] = "2026-01-01T00:02:30Z"
        return (comments,)

    def workflow_run_pages(self, workflow_id, repository_id):
        self.assert_workflow_id(workflow_id)
        if repository_id != 1148737145:
            raise AssertionError("repository ID mismatch")
        self.run_listing_count += 1
        rows = [dict(self.fixture["run_api"])]
        if self.duplicate_second_run and self.run_listing_count >= 2:
            duplicate = dict(rows[0])
            duplicate["id"] += 1
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
            return readback.read_validation(self.fixture["context"].task_uid, api)

    def setUp(self):
        self.fixture = readback_fixture()

    def test_full_live_readback_passes_without_accepting_caller_ids_or_writes(self):
        api = FakeReadbackAPI(self.fixture)
        envelope = self._run(api)
        self.assertEqual(readback.contract.READBACK_SCHEMA, envelope["schema"])
        self.assertEqual(self.fixture["run"]["id"], envelope["run_id"])
        self.assertEqual(2, api.comment_listing_count)
        self.assertEqual(2, api.project_lookup_count)
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
        with self.assertRaisesRegex(readback.ReadbackError, "latest R/A|run_attempt changed"):
            self._run(api)

    def test_final_issue_read_rejects_changed_pinned_comment(self):
        api = FakeReadbackAPI(self.fixture)
        api.mutate_final_comment = True
        with self.assertRaises(readback.contract.ContractError):
            self._run(api)

    def test_final_project_task_identity_must_remain_unchanged(self):
        api = FakeReadbackAPI(self.fixture)
        api.change_project_on_final_read = True
        with patch.object(
            readback, "_trusted_inventory",
            return_value=(self.fixture["context"], "6" * 40, "7" * 40),
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "Project-backed Task Issue identity"):
                readback.read_validation(self.fixture["context"].task_uid, api)

    def test_exact_attempt_rejects_a_second_artifact_alias(self):
        api = FakeReadbackAPI(self.fixture)
        api.duplicate_artifact = True
        with patch.object(
            readback, "_trusted_inventory",
            return_value=(self.fixture["context"], "6" * 40, "7" * 40),
        ):
            with self.assertRaisesRegex(readback.ReadbackError, "artifact"):
                readback.read_validation(self.fixture["context"].task_uid, api)

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
