#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import pathlib
import re
import tempfile
import time
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "github_pr_snapshot_under_test", ROOT / "scripts/pm/github_pr_snapshot.py"
)
assert SPEC and SPEC.loader
SNAPSHOT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SNAPSHOT)
API = SNAPSHOT._github_api_module()
OBS_SPEC = importlib.util.spec_from_file_location(
    "github_observation_for_snapshot_test", ROOT / "scripts/pm/github_observation.py"
)
assert OBS_SPEC and OBS_SPEC.loader
OBS = importlib.util.module_from_spec(OBS_SPEC)
OBS_SPEC.loader.exec_module(OBS)


REPOSITORY = "eng-cc/oasis7"
NUMBER = 145


def connection(nodes):
    return {"pageInfo": {"hasNextPage": False, "endCursor": None}, "nodes": nodes}


def fixture_data(threads=None):
    threads = threads if threads is not None else [
        {"id": "PRRT_1", "isResolved": False, "isOutdated": False, "path": "a.py", "line": 1,
         "originalLine": 1, "startLine": None, "originalStartLine": None,
         "comments": connection([{"id": "C_1", "body": "first", "createdAt": "2026-01-01T00:00:00Z",
                                   "url": "https://example.test/1", "author": {"login": "reviewer"}}])},
        {"id": "PRRT_2", "isResolved": False, "isOutdated": True, "path": "b.py", "line": 2,
         "originalLine": 2, "startLine": None, "originalStartLine": None,
         "comments": connection([{"id": "C_2", "body": "second", "createdAt": "2026-01-02T00:00:00Z",
                                   "url": "https://example.test/2", "author": {"login": "reviewer"}}])},
    ]
    pull_request = {
        "number": NUMBER,
        "url": f"https://github.com/{REPOSITORY}/pull/{NUMBER}",
        "state": "OPEN",
        "isDraft": False,
        "body": "PR body",
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "BLOCKED",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefName": "task/test",
        "headRefOid": "0123456789abcdef",
        "baseRefName": "main",
        "baseRefOid": "fedcba9876543210",
        "comments": connection([]),
        "reviews": connection([{"id": "R_1", "body": "review body", "state": "COMMENTED"}]),
        "reviewThreads": connection(threads),
        "commits": {"nodes": [{"commit": {"oid": "0123456789abcdef",
                      "statusCheckRollup": {"contexts": connection([])}}}]},
    }
    return {
        "viewer": {"login": "fixture-user"},
        "rateLimit": {"cost": 1, "remaining": 4999, "used": 1,
                       "resetAt": "2026-10-01T13:00:00Z", "limit": 5000},
        "repository": {"nameWithOwner": REPOSITORY, "pullRequest": pull_request},
    }


class FakeClock:
    def __init__(self):
        self.now = time.time()

    def __call__(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds


class SnapshotTransport:
    def __init__(self, *, threads=None, search_result=None):
        self.threads = threads
        self.search_result = search_result or {
            "issueCount": 1,
            "pageInfo": {"hasNextPage": False, "endCursor": None},
            "nodes": [{"__typename": "PullRequest", "number": NUMBER,
                       "url": f"https://github.com/{REPOSITORY}/pull/{NUMBER}",
                       "headRefName": "feature/branch", "repository": {"nameWithOwner": REPOSITORY}}],
        }
        self.calls = []
        self.force_uncertain_thread = None
        self.force_malformed_thread = None
        self.rest_responses = []

    def __call__(self, method, url, headers, body, timeout):
        if url != API.GRAPHQL_URL:
            self.calls.append((method, url, {}))
            if not self.rest_responses:
                raise AssertionError("unexpected REST request")
            return self.rest_responses.pop(0)
        payload = json.loads(body or b"{}")
        query = payload.get("query", "")
        self.calls.append((method, url, payload))
        if "mutation ResolvePRReviewThread" in query:
            thread_id = payload.get("variables", {}).get("threadId")
            for thread in self.threads:
                if thread["id"] == thread_id:
                    thread["isResolved"] = True
            if self.force_uncertain_thread == thread_id:
                return API.HTTPResponse(503, {}, json.dumps({"message": "fixture timeout after apply"}))
            if self.force_malformed_thread == thread_id:
                return API.HTTPResponse(200, {}, b"[]")
            data = {"resolveReviewThread": {"thread": {"id": thread_id, "isResolved": True}}}
        elif "PRReviewThreadReadback" in query:
            data = {"repository": {"nameWithOwner": REPOSITORY,
                     "pullRequest": {"number": NUMBER,
                                     "reviewThreads": connection([
                                         {"id": thread["id"], "isResolved": thread["isResolved"]}
                                         for thread in self.threads])}}}
        elif "PRSelectorByHead" in query:
            data = {"search": self.search_result}
        elif "GitHubPRIdentity" in query:
            data = fixture_data(self.threads)
            data["repository"]["pullRequest"].pop("comments")
            data["repository"]["pullRequest"].pop("reviews")
            data["repository"]["pullRequest"].pop("reviewThreads")
            data["repository"]["pullRequest"].pop("commits")
        else:
            data = fixture_data(self.threads)
        if isinstance(data.get("viewer"), dict):
            viewer_selection = re.search(r"viewer\s*\{([^}]*)\}", query)
            if viewer_selection and re.search(r"\bid\b", viewer_selection.group(1)):
                data["viewer"]["id"] = "MDQ6VXNlcjEyMw=="
            else:
                data["viewer"].pop("id", None)
        if "GitHubPRSnapshot" in query and "body" not in query:
            pr = data["repository"]["pullRequest"]
            pr.pop("body", None)
            for review in pr.get("reviews", {}).get("nodes", []):
                review.pop("body", None)
            for comment in pr.get("comments", {}).get("nodes", []):
                comment.pop("body", None)
            for thread in pr.get("reviewThreads", {}).get("nodes", []):
                for comment in thread.get("comments", {}).get("nodes", []):
                    comment.pop("body", None)
        result = {"data": data}
        return API.HTTPResponse(200, {
            "X-RateLimit-Limit": "5000",
            "X-RateLimit-Remaining": "4999",
            "X-RateLimit-Used": "1",
            "X-RateLimit-Reset": str(int(time.time()) + 3600),
        }, json.dumps(result))

    def client(self, state_root, token="fixture-token"):
        clock = FakeClock()
        return API.GitHubAPIClient(
            token, transport=self, state_root=state_root,
            clock=clock, sleeper=clock.sleep, random_value=lambda: 0,
        )


class GitHubPRSnapshotTests(unittest.TestCase):
    def setUp(self):
        API._PROCESS_PAUSES.clear()
        self.temp = tempfile.TemporaryDirectory()
        self.state_root = pathlib.Path(self.temp.name) / "api-state"

    def tearDown(self):
        self.temp.cleanup()

    def test_flat_projection_is_strict_and_summary_omits_comment_body_fields(self):
        transport = SnapshotTransport()
        snapshot = SNAPSHOT.fetch_pr_snapshot(transport.client(self.state_root), REPOSITORY, NUMBER)
        self.assertEqual(snapshot["repository"], REPOSITORY)
        self.assertEqual(snapshot["number"], NUMBER)
        self.assertEqual(snapshot["threads"][0]["id"], "PRRT_1")
        self.assertTrue(snapshot["snapshot_metadata"]["complete"])
        self.assertEqual(len(transport.calls), 1)

        summary_query = SNAPSHOT._query(False)
        self.assertNotIn("body", summary_query)
        summary_transport = SnapshotTransport()
        summary = SNAPSHOT.fetch_pr_snapshot(
            summary_transport.client(self.state_root / "summary"), REPOSITORY, NUMBER,
            include_comment_bodies=False,
        )
        self.assertNotIn("body", summary)
        self.assertNotIn("body", summary["reviews"][0])

    def test_null_required_connection_fails_but_known_empty_connection_is_valid(self):
        valid = fixture_data()
        valid["repository"]["pullRequest"]["comments"] = connection([])
        normal_transport = SnapshotTransport()
        normal = normal_transport.client(self.state_root)
        self.assertEqual(SNAPSHOT.fetch_pr_snapshot(normal, REPOSITORY, NUMBER)["comments"], [])

        for broken_value in (None, {"pageInfo": {"hasNextPage": False}, "nodes": []}):
            broken = fixture_data()
            broken["repository"]["pullRequest"]["comments"] = broken_value
            fake = type("Client", (), {"graphql": lambda self, *args, **kwargs: broken})()
            with self.assertRaises(API.APIError) as caught:
                SNAPSHOT.fetch_pr_snapshot(fake, REPOSITORY, NUMBER)
            self.assertEqual(caught.exception.kind, "malformed_response")

    def test_incomplete_snapshot_refuses_before_any_review_mutation(self):
        threads = fixture_data()["repository"]["pullRequest"]["reviewThreads"]["nodes"]
        threads[0]["comments"]["pageInfo"]["hasNextPage"] = True
        transport = SnapshotTransport(threads=threads)
        client = transport.client(self.state_root)
        with self.assertRaises(API.APIError) as caught:
            SNAPSHOT.closeout_review_threads(client, REPOSITORY, NUMBER, thread_ids=["PRRT_1"])
        self.assertEqual(caught.exception.kind, "incomplete_snapshot")
        self.assertEqual(len(transport.calls), 1)
        self.assertNotIn("mutation ResolvePRReviewThread", transport.calls[0][2]["query"])

    def test_selector_local_forms_cost_no_lookup_and_branch_costs_one_query(self):
        transport = SnapshotTransport()
        client = transport.client(self.state_root)
        self.assertEqual(SNAPSHOT.resolve_pr_selector(client, "#145", "git@github.com:eng-cc/oasis7.git")["number"], 145)
        self.assertEqual(SNAPSHOT.resolve_pr_selector(client, "https://github.com/eng-cc/oasis7/pull/145", None)["selector_kind"],
                         "pull_request_url")
        self.assertEqual(SNAPSHOT.resolve_pr_selector(client, "eng-cc/oasis7#145", REPOSITORY)["selector_kind"],
                         "repository_number")
        self.assertEqual(transport.calls, [])
        branch = SNAPSHOT.resolve_pr_selector(client, "feature/branch", REPOSITORY)
        self.assertEqual(branch["selector_kind"], "branch")
        self.assertEqual(branch["number"], NUMBER)
        self.assertEqual(len(transport.calls), 1)
        self.assertEqual(transport.calls[0][2]["variables"]["query"],
                         "repo:eng-cc/oasis7 is:pr head:feature/branch")

    def test_omitted_selector_uses_current_branch_and_origin_with_one_lookup(self):
        transport = SnapshotTransport(search_result={
            "issueCount": 1,
            "pageInfo": {"hasNextPage": False, "endCursor": None},
            "nodes": [{"__typename": "PullRequest", "number": NUMBER,
                       "url": f"https://github.com/{REPOSITORY}/pull/{NUMBER}",
                       "headRefName": "task/current", "repository": {"nameWithOwner": REPOSITORY}}],
        })
        client = transport.client(self.state_root)

        def git_output(*args):
            if args == ("branch", "--show-current"):
                return "task/current"
            if args == ("remote", "get-url", "origin"):
                return "git@github.com:eng-cc/oasis7.git"
            raise AssertionError(args)

        with mock.patch.object(SNAPSHOT, "_git_output", side_effect=git_output):
            resolved = SNAPSHOT.resolve_pr_selector(client, None, None)
        self.assertEqual(resolved["selector_kind"], "branch")
        self.assertEqual(resolved["repository"], REPOSITORY)
        self.assertEqual(len(transport.calls), 1)

    def test_closeout_sends_one_pre_read_n_mutations_and_one_post_read(self):
        threads = fixture_data()["repository"]["pullRequest"]["reviewThreads"]["nodes"]
        transport = SnapshotTransport(threads=threads)
        result, selected = SNAPSHOT.closeout_review_threads(
            transport.client(self.state_root), REPOSITORY, NUMBER,
            thread_ids=["PRRT_1", "PRRT_2"],
        )
        self.assertEqual(selected, ["PRRT_1", "PRRT_2"])
        self.assertTrue(all(item["isResolved"] for item in result["threads"]))
        operations = [call[2]["query"].split("(", 1)[0].strip().split()[1]
                      for call in transport.calls]
        self.assertEqual(operations, ["GitHubPRSnapshot", "ResolvePRReviewThread",
                                      "ResolvePRReviewThread", "GitHubPRSnapshot"])
        self.assertEqual(len(transport.calls), 4)

    def test_uncertain_mutation_is_read_back_once_and_never_replayed(self):
        threads = fixture_data()["repository"]["pullRequest"]["reviewThreads"]["nodes"]
        transport = SnapshotTransport(threads=threads)
        transport.force_uncertain_thread = "PRRT_1"
        result, selected = SNAPSHOT.closeout_review_threads(
            transport.client(self.state_root), REPOSITORY, NUMBER, thread_ids=["PRRT_1"]
        )
        self.assertEqual(selected, ["PRRT_1"])
        self.assertTrue(result["threads"][0]["isResolved"])
        queries = [call[2]["query"] for call in transport.calls]
        self.assertEqual(sum("mutation ResolvePRReviewThread" in query for query in queries), 1)
        self.assertEqual(sum("PRReviewThreadReadback" in query for query in queries), 1)
        self.assertEqual(len(transport.calls), 4)

    def test_malformed_successful_mutation_is_read_back_once_and_never_replayed(self):
        threads = fixture_data()["repository"]["pullRequest"]["reviewThreads"]["nodes"]
        transport = SnapshotTransport(threads=threads)
        transport.force_malformed_thread = "PRRT_1"
        result, selected = SNAPSHOT.closeout_review_threads(
            transport.client(self.state_root), REPOSITORY, NUMBER, thread_ids=["PRRT_1"]
        )
        self.assertEqual(selected, ["PRRT_1"])
        self.assertTrue(result["threads"][0]["isResolved"])
        queries = [call[2]["query"] for call in transport.calls if call[2]]
        self.assertEqual(sum("mutation ResolvePRReviewThread" in query for query in queries), 1)
        self.assertEqual(sum("PRReviewThreadReadback" in query for query in queries), 1)
        self.assertEqual(len(transport.calls), 4)

    def test_identity_read_is_a_distinct_uncached_request(self):
        transport = SnapshotTransport()
        client = transport.client(self.state_root)
        SNAPSHOT.fetch_pr_snapshot(client, REPOSITORY, NUMBER)
        identity = SNAPSHOT.fetch_pr_identity(client, REPOSITORY, NUMBER)
        self.assertEqual(identity["headRefOid"], "0123456789abcdef")
        self.assertEqual(len(transport.calls), 2)
        self.assertIn("GitHubPRSnapshot", transport.calls[0][2]["query"])
        self.assertIn("GitHubPRIdentity", transport.calls[1][2]["query"])

    def test_actual_snapshot_identity_confirms_shared_account_pause(self):
        API._PROCESS_PAUSES.clear()
        token_a_transport = SnapshotTransport()
        token_b_transport = SnapshotTransport()
        token_a = token_a_transport.client(self.state_root, "credential-a")
        token_b = token_b_transport.client(self.state_root, "credential-b")

        snapshot = SNAPSHOT.fetch_pr_snapshot(token_a, REPOSITORY, NUMBER)
        identity = SNAPSHOT.fetch_pr_identity(token_b, REPOSITORY, NUMBER)
        self.assertEqual(snapshot["viewer"].get("id"), "MDQ6VXNlcjEyMw==")
        self.assertEqual(identity["identity_metadata"]["viewer"].get("id"), "MDQ6VXNlcjEyMw==")
        self.assertEqual(token_a._account_scope_state_key, token_b._account_scope_state_key)
        self.assertNotEqual(token_a.credential_scope_digest, token_b.credential_scope_digest)
        cache_context = {"effective_policy": "frozen"}
        self.assertNotEqual(
            OBS.observation_key(token_a, REPOSITORY, NUMBER, task_uid="fixture-task",
                                context=cache_context),
            OBS.observation_key(token_b, REPOSITORY, NUMBER, task_uid="fixture-task",
                                context=cache_context),
        )

        token_a_transport.rest_responses.append(API.HTTPResponse(
            429, {"Retry-After": "120"}, json.dumps({"message": "secondary rate limit"}),
        ))
        with self.assertRaises(API.APIError) as limited:
            token_a.rest("GET", "repos/owner/repo", operation="trigger_secondary_limit")
        self.assertEqual(limited.exception.workflow_status, "external_wait")
        token_b_calls_before = len(token_b_transport.calls)
        with self.assertRaises(API.APIError) as shared_pause:
            token_b.rest("GET", "repos/owner/repo", operation="same_account_pause")
        self.assertEqual(shared_pause.exception.workflow_status, "external_wait")
        self.assertEqual(len(token_b_transport.calls), token_b_calls_before)


if __name__ == "__main__":
    unittest.main(verbosity=2)
