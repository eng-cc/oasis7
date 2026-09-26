#!/usr/bin/env python3
"""Independent adversarial pagination checks for validation-only readback.

The live values below were independently read from GitHub at the time this
slice ran: eng-cc/oasis7 repository ID 1148737145 and rust.yml workflow ID
230018940. All URL traversal tests use mocked HTTP responses and issue no API
writes.
"""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

MODULE_PATH = Path(__file__).with_name("ci_reuse_validation_readback.py")
SPEC = importlib.util.spec_from_file_location("ci_reuse_validation_readback_pg2", MODULE_PATH)
readback = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = readback
SPEC.loader.exec_module(readback)

LIVE_REPOSITORY_ID = 1148737145
LIVE_WORKFLOW_ID = 230018940
REPOSITORY = "eng-cc/oasis7"
PAGE_SIZE = 100
SLUG_PATH = f"/repos/{REPOSITORY}/actions/workflows/{LIVE_WORKFLOW_ID}/runs"
ID_PATH = f"/repositories/{LIVE_REPOSITORY_ID}/actions/workflows/{LIVE_WORKFLOW_ID}/runs"


def response(payload: dict, *, link: str | None = None) -> bytes:
    headers = ["HTTP/2.0 200 OK", "Content-Type: application/json"]
    if link is not None:
        headers.append(f"Link: {link}")
    raw_headers = ("\r\n".join(headers) + "\r\n\r\n").encode("ascii")
    return raw_headers + json.dumps(payload, separators=(",", ":")).encode("utf-8")


def page(total: int, ids: list[int], *, link: str | None = None) -> bytes:
    return response({
        "total_count": total,
        "workflow_runs": [{"id": run_id, "display_title": f"run-{run_id}"} for run_id in ids],
    }, link=link)


def identity_responses() -> list[bytes]:
    return [
        response({
            "id": LIVE_REPOSITORY_ID,
            "name": "oasis7",
            "full_name": REPOSITORY,
            "owner": {"login": "eng-cc"},
            "default_branch": "main",
        }),
        response({"id": LIVE_WORKFLOW_ID, "path": readback.WORKFLOW_FILE, "state": "active"}),
    ]


def next_link(path: str, page_number: int, *, query: str | None = None) -> str:
    q = query if query is not None else f"per_page={PAGE_SIZE}&page={page_number}"
    return f'<https://api.github.com{path}?{q}>; rel="next"'


class LiveIdentityFixture:
    def __init__(self, *, repository_id: int = LIVE_REPOSITORY_ID,
                 workflow_id: int = LIVE_WORKFLOW_ID):
        self.repository_id = repository_id
        self.workflow_id = workflow_id
        self.calls: list[str] = []

    def get_json(self, endpoint: str):
        self.calls.append(endpoint)
        if endpoint == f"repos/{REPOSITORY}":
            return {
                "id": self.repository_id,
                "name": "oasis7",
                "full_name": REPOSITORY,
                "owner": {"login": "eng-cc"},
                "default_branch": "main",
            }
        if endpoint == f"repos/{REPOSITORY}/actions/workflows/rust.yml":
            return {"id": self.workflow_id, "path": readback.WORKFLOW_FILE, "state": "active"}
        raise AssertionError(f"unexpected endpoint: {endpoint}")


class PaginationAdversarialTests(unittest.TestCase):
    def fetch(self, responses: list[bytes]):
        with patch.object(
            readback.subprocess, "check_output", side_effect=identity_responses() + responses,
        ) as mocked:
            pages = readback.GitHubReadOnly().workflow_run_pages(
                LIVE_WORKFLOW_ID, LIVE_REPOSITORY_ID,
            )
        return pages, mocked

    def test_live_identity_resolves_repository_id_and_workflow_id(self):
        api = LiveIdentityFixture()
        self.assertEqual(
            (LIVE_WORKFLOW_ID, "main", LIVE_REPOSITORY_ID),
            readback._live_workflow(api),
        )
        self.assertEqual([
            f"repos/{REPOSITORY}",
            f"repos/{REPOSITORY}/actions/workflows/rust.yml",
        ], api.calls)

    def test_producer_compatible_one_arg_call_refreshes_live_ids_then_follows_alias(self):
        repository = {
            "id": LIVE_REPOSITORY_ID,
            "name": "oasis7",
            "full_name": REPOSITORY,
            "owner": {"login": "eng-cc"},
            "default_branch": "main",
        }
        workflow = {"id": LIVE_WORKFLOW_ID, "path": readback.WORKFLOW_FILE, "state": "active"}
        with patch.object(readback.subprocess, "check_output", side_effect=[
            response(repository),
            response(workflow),
            page(2, [101], link=next_link(ID_PATH, 2)),
            page(2, [102]),
        ]) as mocked:
            pages = readback.GitHubReadOnly().workflow_run_pages(LIVE_WORKFLOW_ID)
        self.assertEqual([101, 102], [run["id"] for run in readback.contract.collect_workflow_runs(pages)])
        self.assertEqual(4, mocked.call_count)
        endpoints = [call.args[0][-1] for call in mocked.call_args_list]
        self.assertEqual([
            f"repos/{REPOSITORY}",
            f"repos/{REPOSITORY}/actions/workflows/rust.yml",
            f"repos/{REPOSITORY}/actions/workflows/{LIVE_WORKFLOW_ID}/runs?per_page=100",
            f"repositories/{LIVE_REPOSITORY_ID}/actions/workflows/{LIVE_WORKFLOW_ID}/runs?per_page=100&page=2",
        ], endpoints)

    def test_wrong_supplied_live_repository_id_is_rejected_before_run_listing(self):
        repository = {
            "id": LIVE_REPOSITORY_ID,
            "name": "oasis7",
            "full_name": REPOSITORY,
            "owner": {"login": "eng-cc"},
            "default_branch": "main",
        }
        workflow = {"id": LIVE_WORKFLOW_ID, "path": readback.WORKFLOW_FILE, "state": "active"}
        with patch.object(readback.subprocess, "check_output", side_effect=[
            response(repository), response(workflow),
        ]) as mocked:
            with self.assertRaisesRegex(readback.ReadbackError, "differs from live repository"):
                readback.GitHubReadOnly().workflow_run_pages(
                    LIVE_WORKFLOW_ID, LIVE_REPOSITORY_ID + 1,
                )
        self.assertEqual(2, mocked.call_count, "mismatched live repository ID must stop before run listing")

    def test_numeric_repository_alias_continues_exact_live_workflow(self):
        link = next_link(ID_PATH, 2)
        pages, mocked = self.fetch([
            page(2, [101], link=link),
            page(2, [102]),
        ])
        self.assertEqual([101, 102], [run["id"] for run in readback.contract.collect_workflow_runs(pages)])
        self.assertEqual([True, False], [p["has_next"] for p in pages])
        self.assertEqual(4, mocked.call_count)
        first_endpoint = mocked.call_args_list[2].args[0][-1]
        second_endpoint = mocked.call_args_list[3].args[0][-1]
        self.assertEqual(f"repos/{REPOSITORY}/actions/workflows/{LIVE_WORKFLOW_ID}/runs?per_page=100", first_endpoint)
        self.assertEqual(f"repositories/{LIVE_REPOSITORY_ID}/actions/workflows/{LIVE_WORKFLOW_ID}/runs?per_page=100&page=2", second_endpoint)

    def assert_link_rejected(self, link: str, message: str) -> None:
        with patch.object(
            readback.subprocess, "check_output",
            side_effect=identity_responses() + [page(2, [1], link=link)],
        ) as mocked:
            with self.assertRaisesRegex(readback.ReadbackError, message):
                readback.GitHubReadOnly().workflow_run_pages(LIVE_WORKFLOW_ID, LIVE_REPOSITORY_ID)
        self.assertEqual(3, mocked.call_count, "invalid next link must stop before another GET")

    def test_wrong_repository_numeric_id_is_rejected(self):
        wrong_path = f"/repositories/{LIVE_REPOSITORY_ID + 1}/actions/workflows/{LIVE_WORKFLOW_ID}/runs"
        self.assert_link_rejected(next_link(wrong_path, 2), "canonical API endpoint")

    def test_wrong_workflow_id_in_numeric_alias_is_rejected(self):
        wrong_path = f"/repositories/{LIVE_REPOSITORY_ID}/actions/workflows/{LIVE_WORKFLOW_ID + 1}/runs"
        self.assert_link_rejected(next_link(wrong_path, 2), "canonical API endpoint")

    def test_wrong_endpoint_suffix_is_rejected(self):
        wrong_path = f"/repositories/{LIVE_REPOSITORY_ID}/actions/workflows/{LIVE_WORKFLOW_ID}/jobs"
        self.assert_link_rejected(next_link(wrong_path, 2), "canonical API endpoint")

    def test_wrong_slug_repository_is_rejected(self):
        wrong_path = f"/repos/another-owner/oasis7/actions/workflows/{LIVE_WORKFLOW_ID}/runs"
        self.assert_link_rejected(next_link(wrong_path, 2), "canonical API endpoint")

    def test_wrong_host_is_rejected(self):
        link = f'<https://api.github.evil/repositories/{LIVE_REPOSITORY_ID}/actions/workflows/{LIVE_WORKFLOW_ID}/runs?per_page=100&page=2>; rel="next"'
        self.assert_link_rejected(link, "canonical API endpoint")

    def test_changed_or_extra_query_is_rejected(self):
        bad = next_link(ID_PATH, 2, query="per_page=100&page=2&event=workflow_dispatch")
        self.assert_link_rejected(bad, "unfiltered page query")

    def test_duplicate_query_parameters_are_rejected(self):
        bad = next_link(ID_PATH, 2, query="per_page=100&per_page=100&page=2")
        self.assert_link_rejected(bad, "unfiltered page query")

    def test_page_skip_and_repeat_are_rejected(self):
        for target in (3, 1):
            with self.subTest(target_page=target):
                self.assert_link_rejected(next_link(ID_PATH, target), "following page")

    def test_two_page_cycle_is_rejected_before_repeating_a_get(self):
        first = next_link(ID_PATH, 2)
        back_to_first = next_link(SLUG_PATH, 1)
        with patch.object(readback.subprocess, "check_output", side_effect=(
            identity_responses() + [
                page(2, [101], link=first),
                page(2, [102], link=back_to_first),
            ]
        )) as mocked:
            with self.assertRaisesRegex(readback.ReadbackError, "following page"):
                readback.GitHubReadOnly().workflow_run_pages(LIVE_WORKFLOW_ID, LIVE_REPOSITORY_ID)
        self.assertEqual(4, mocked.call_count, "cycle must be rejected before requesting page 1 again")

    def test_count_drift_across_valid_alias_pages_fails_contract(self):
        pages, _ = self.fetch([
            page(2, [101], link=next_link(ID_PATH, 2)),
            page(3, [102]),
        ])
        with self.assertRaisesRegex(readback.contract.ContractError, "total_count changed"):
            readback.contract.collect_workflow_runs(pages)

    def test_duplicate_run_ids_across_alias_pages_fail_contract(self):
        pages, _ = self.fetch([
            page(2, [101], link=next_link(ID_PATH, 2)),
            page(2, [101]),
        ])
        with self.assertRaisesRegex(readback.contract.ContractError, "duplicate run ID"):
            readback.contract.collect_workflow_runs(pages)

    def test_truncated_history_without_next_link_fails_contract(self):
        pages, _ = self.fetch([page(2, [101])])
        with self.assertRaisesRegex(readback.contract.ContractError, "does not equal unique returned runs"):
            readback.contract.collect_workflow_runs(pages)

    def test_multiple_next_links_are_rejected(self):
        link = ", ".join([next_link(ID_PATH, 2), next_link(SLUG_PATH, 2)])
        self.assert_link_rejected(link, "multiple next links")


if __name__ == "__main__":
    unittest.main()
