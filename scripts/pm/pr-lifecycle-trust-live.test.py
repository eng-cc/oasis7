#!/usr/bin/env python3
"""Injected-transport trust regressions for the live lifecycle gate."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import pathlib
import tempfile
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
UID = "task_11111111111111111111111111111111"
REPOSITORY = "eng-cc/oasis7"
ISSUE = 2198
PR = 2198
HEAD = "a" * 40


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


API = load_module("pr_lifecycle_trust_live_api", ROOT / "scripts/pm/github_api.py")
GATE = load_module("pr_lifecycle_trust_live_gate", ROOT / "scripts/pm/pr-lifecycle-gate.py")


def connection(nodes=None, *, has_next=False):
    return {"pageInfo": {"hasNextPage": has_next, "endCursor": "cursor" if has_next else None},
            "nodes": nodes or []}


def snapshot_payload(*, comments=None, comments_incomplete=False):
    pr = {
        "number": PR,
        "url": f"https://github.com/{REPOSITORY}/pull/{PR}",
        "state": "OPEN",
        "isDraft": False,
        "body": f"Task: {UID}\nRefs #{ISSUE}",
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefName": "codex/task",
        "headRefOid": HEAD,
        "baseRefName": "main",
        "baseRefOid": "b" * 40,
        "comments": connection(comments, has_next=comments_incomplete),
        "reviews": connection(),
        "reviewThreads": connection(),
        "commits": {"nodes": [{"commit": {"oid": HEAD,
            "statusCheckRollup": {"contexts": connection()}}}]},
    }
    return {"data": {
        "viewer": {"login": "fixture-viewer"},
        "rateLimit": {"cost": 1, "remaining": 499, "used": 1, "limit": 500,
                      "resetAt": "2099-01-01T00:00:00Z"},
        "repository": {"nameWithOwner": REPOSITORY, "pullRequest": pr},
    }}


def active_hold_comment():
    body = "\n".join((
        "<!-- oasis7-merge-hold -->",
        f"- task_uid: `{UID}`",
        f"- repository: `{REPOSITORY}`",
        f"- issue_number: `{ISSUE}`",
        f"- pr_number: `{PR}`",
        f"- head_oid: `{HEAD}`",
        "- node_id: `merge_hold`",
        "- kind: `merge_hold`",
        "- disposition: `active`",
        "- hold_kind: `user_requested_merge_hold`",
        "- active: `true`",
        "- requester: `user`",
        "- reason: `do not merge`",
        "- resume_authority: `user`",
    ))
    return {
        "id": 501,
        "body": body,
        "user": {"login": "user"},
        "created_at": "2026-10-01T00:00:00Z",
        "html_url": f"https://github.com/{REPOSITORY}/issues/{ISSUE}#issuecomment-501",
    }


class InjectedTransport:
    def __init__(self, *, issue_comments=None, pr_comments=None, incomplete=False):
        self.issue_comments = issue_comments or []
        self.pr_comments = pr_comments or []
        self.incomplete = incomplete
        self.calls = []

    def __call__(self, method, url, headers, body, timeout):
        self.calls.append((method, url, body, dict(headers), timeout))
        if url.endswith("/graphql"):
            query = json.loads(body.decode("utf-8"))["query"]
            if "GitHubPRSnapshot" not in query:
                raise AssertionError(f"unexpected GraphQL operation: {query[:80]}")
            payload = snapshot_payload(comments=self.pr_comments,
                                       comments_incomplete=self.incomplete)
            return API.HTTPResponse(200, {}, json.dumps(payload))
        if "/branches/main/protection" in url:
            return API.HTTPResponse(404, {}, json.dumps({"message": "Not Found"}))
        if "/rulesets?" in url:
            return API.HTTPResponse(200, {}, "[]")
        if f"/issues/{ISSUE}/comments?" in url:
            return API.HTTPResponse(200, {}, json.dumps(self.issue_comments))
        raise AssertionError(f"unexpected GitHub request: {method} {url}")


class LiveLifecycleTrustTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name).resolve()
        self.mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        self.mapping_path.parent.mkdir(parents=True)
        self.record = {
            "task_uid": UID,
            "repository": REPOSITORY,
            "issue_number": ISSUE,
            "pr_number": PR,
            "pr_url": f"https://github.com/{REPOSITORY}/pull/{PR}",
            "canonical_worktree": str(self.root),
            "merge_hold": {
                "kind": "normal_pr_ci_watch", "active": False,
                "requester": "workflow", "reason": "normal", "resume_authority": "workflow",
            },
        }
        self.write_mapping()

    def tearDown(self):
        self.temp.cleanup()

    def write_mapping(self, extra=None):
        record = {**self.record, **(extra or {})}
        self.mapping_path.write_text(json.dumps({"version": 1, "tasks": {UID: record}}),
                                     encoding="utf-8")

    def run_gate(self, transport):
        client = API.GitHubAPIClient(
            "fixture-token", transport=transport,
            state_root=self.root / "api-state", clock=lambda: 1790860000.0,
            sleeper=lambda _seconds: None, random_value=lambda: 0,
        )
        old_argv = GATE.sys.argv
        GATE.sys.argv = [
            "pr-lifecycle-gate.py", str(PR), "--root", str(self.root),
            "--tool-root", str(ROOT), "--task-uid", UID, "--json",
        ]
        output = io.StringIO()
        try:
            with mock.patch.object(GATE, "_github_api_client", return_value=client), \
                 mock.patch.object(GATE, "local_loop_admission",
                                   side_effect=AssertionError("blocked gate reached loop admission")), \
                 contextlib.redirect_stdout(output):
                status = GATE.main()
        finally:
            GATE.sys.argv = old_argv
        return status, json.loads(output.getvalue()), transport

    def test_live_task_issue_hold_is_rebuilt_over_injected_shared_transport(self):
        transport = InjectedTransport(issue_comments=[active_hold_comment()])
        status, result, transport = self.run_gate(transport)

        self.assertEqual(status, 3, result)
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(any("user_requested_merge_hold" in item for item in result["blockers"]), result)
        self.assertNotIn("readiness_receipt", result)
        self.assertTrue(any(url.endswith("/graphql") for _method, url, *_rest in transport.calls))
        self.assertTrue(any("/issues/2198/comments?" in url for _method, url, *_rest in transport.calls))

    def test_caller_cached_disposition_cannot_hide_live_actionable_comment(self):
        self.write_mapping({"comment_dispositions": [{
            "node_id": "page-two", "head_oid": HEAD, "disposition": "addressed",
            "evidence": "caller-authored cache text",
        }]})
        transport = InjectedTransport(pr_comments=[{
            "id": "page-two", "body": "please fix the page-two issue",
            "url": f"https://github.com/{REPOSITORY}/pull/{PR}#issuecomment-2",
            "author": {"login": "reviewer"}, "authorAssociation": "NONE",
        }])
        status, result, _transport = self.run_gate(transport)

        self.assertEqual(status, 3, result)
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(any("issuecomment-2" in item for item in result["blockers"]), result)
        self.assertNotIn("readiness_receipt", result)

    def test_incomplete_live_pr_connection_fails_before_policy_or_receipt(self):
        transport = InjectedTransport(incomplete=True)
        status, result, transport = self.run_gate(transport)

        self.assertEqual(status, 2, result)
        self.assertEqual(result["status"], "capability_blocked")
        self.assertIn("incomplete or not fresh", result["error"])
        self.assertFalse(any("/branches/" in url or "/rulesets?" in url
                             for _method, url, *_rest in transport.calls))
        self.assertNotIn("readiness_receipt", result)


if __name__ == "__main__":
    unittest.main()
