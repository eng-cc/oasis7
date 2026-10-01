#!/usr/bin/env python3
"""Behavioral request budgets for the real PR lifecycle load path."""

from __future__ import annotations

import importlib.util
import json
import pathlib
import tempfile
import unittest
import contextlib
import io
import subprocess
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


API = load_module("api_budget_test_github_api", ROOT / "scripts/pm/github_api.py")
GATE = load_module("api_budget_test_gate", ROOT / "scripts/pm/pr-lifecycle-gate.py")
SNAPSHOT = load_module("api_budget_test_snapshot", ROOT / "scripts/pm/github_pr_snapshot.py")
OBSERVATION = load_module("api_budget_test_observation", ROOT / "scripts/pm/github_observation.py")


def connection(nodes=None, *, more=False):
    return {"pageInfo": {"hasNextPage": more, "endCursor": "cursor" if more else None},
            "nodes": nodes or []}


def snapshot_payload(number=42, *, identity_only=False):
    pr = {
        "number": number, "url": f"https://github.com/eng-cc/oasis7/pull/{number}",
        "state": "OPEN", "isDraft": False, "body": "Task: task_11111111111111111111111111111111",
        "headRefName": "codex/task", "headRefOid": "b" * 40,
        "baseRefName": "main", "baseRefOid": "a" * 40,
    }
    if not identity_only:
        pr.update({
            "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN", "reviewDecision": "APPROVED",
            "comments": connection(), "reviews": connection(), "reviewThreads": connection(),
            "commits": {"nodes": [{"commit": {"statusCheckRollup": {"contexts": connection()}}}]},
        })
    return {"data": {
        "viewer": {"login": "test-user"},
        "rateLimit": {"cost": 1, "remaining": 499, "used": 1, "limit": 500,
                      "resetAt": "2026-10-01T13:00:00Z"},
        "repository": {"nameWithOwner": "eng-cc/oasis7", "pullRequest": pr},
    }}


class CountingTransport:
    def __init__(self, *, not_modified=False):
        self.calls = []
        self.not_modified = not_modified

    def __call__(self, method, url, headers, body, timeout):
        self.calls.append((method, url, body, dict(headers), timeout))
        if url.endswith("/graphql"):
            query = json.loads(body.decode("utf-8"))["query"]
            if self.not_modified:
                return API.HTTPResponse(304, {}, "")
            return API.HTTPResponse(200, {}, json.dumps(
                snapshot_payload(identity_only="GitHubPRIdentity" in query),
            ))
        if "/branches/main/protection" in url:
            return API.HTTPResponse(404, {}, json.dumps({"message": "Not Found"}))
        if "/rulesets?" in url:
            return API.HTTPResponse(200, {}, "[]")
        raise AssertionError(f"unexpected REST request: {url}")


class GraphqlBudgetContracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.transport = CountingTransport()
        self.client = API.GitHubAPIClient(
            "test-token", transport=self.transport,
            state_root=pathlib.Path(self.temp.name) / "state",
            clock=lambda: 1790860000.0, sleeper=lambda _seconds: None,
            random_value=lambda: 0,
        )

    def tearDown(self):
        self.temp.cleanup()

    def test_actual_load_live_batches_all_pr_surfaces_and_policy_reads(self):
        with mock.patch.object(GATE.subprocess, "check_output",
                               side_effect=AssertionError("hot path invoked gh subprocess")):
            data = GATE.load_live(
                "42", client=self.client, repository_hint="eng-cc/oasis7",
                number_hint=42, effective_root=ROOT,
                task_uid="task_11111111111111111111111111111111",
            )
        graphql = [call for call in self.transport.calls if call[1].endswith("/graphql")]
        self.assertEqual(len(graphql), 1)
        self.assertEqual(data["number"], 42)
        self.assertEqual(data["repository"], "eng-cc/oasis7")
        self.assertEqual(set(data["snapshot_metadata"]["connections"]), {
            "comments", "reviews", "reviewThreads", "statusCheckRollup", "thread_comments",
        })
        self.assertTrue(data["snapshot_metadata"]["complete"])
        self.assertEqual(data["policy_discovery"]["status"], "resolved")
        self.assertFalse(any("gh" in str(call) for call in self.transport.calls))

    def test_formal_end_identity_is_a_second_fresh_graphql_read(self):
        data = GATE.load_live(
            "42", client=self.client, repository_hint="eng-cc/oasis7", number_hint=42,
            effective_root=ROOT, task_uid="task_11111111111111111111111111111111",
        )
        data["merge_hold"] = {"kind": "normal_pr_ci_watch", "active": False}
        with mock.patch.object(GATE, "local_loop_admission", return_value={"status": "legacy"}), \
             mock.patch.object(GATE, "live_integration_admission", return_value=None), \
             mock.patch.object(GATE.subprocess, "check_output",
                               side_effect=AssertionError("formal hot path invoked gh subprocess")):
            result = GATE.production_decision(
                data, False, ROOT, "task_11111111111111111111111111111111", str(ROOT),
                api_client=self.client,
            )
        graphql = [call for call in self.transport.calls if call[1].endswith("/graphql")]
        self.assertEqual(len(graphql), 2)
        self.assertIn("GitHubPRIdentity", json.loads(graphql[1][2].decode("utf-8"))["query"])
        self.assertTrue(result["ready_for_merge"], result)
        self.assertIn("readiness_receipt", result)

    def test_formal_load_ignores_a_seeded_observation_cache_entry(self):
        task_uid = "task_11111111111111111111111111111111"
        context = {"task_uid": task_uid, "repository": "eng-cc/oasis7", "pr_number": 42}
        key = OBSERVATION.observation_key(
            self.client, "eng-cc/oasis7", 42, task_uid=task_uid,
            context=context, query_version=SNAPSHOT.QUERY_VERSION,
        )
        self.client.write_state("observations", key, {
            "schema": OBSERVATION.SCHEMA, "query_version": SNAPSHOT.QUERY_VERSION,
            "snapshot_digest": "candidate-digest", "next_read_at_epoch": 9999999999,
            "observation": {"status": "observed", "candidate_ready": True},
        })
        GATE.load_live("42", client=self.client, repository_hint="eng-cc/oasis7",
                       number_hint=42, effective_root=ROOT, task_uid=task_uid)
        graphql = [call for call in self.transport.calls if call[1].endswith("/graphql")]
        self.assertEqual(len(graphql), 1, "formal gate must always perform its own fresh snapshot")

    def test_candidate_cache_key_includes_effective_gate_candidate_logic(self):
        task_uid = "task_11111111111111111111111111111111"
        with tempfile.TemporaryDirectory() as temp:
            effective = pathlib.Path(temp)
            scripts = effective / "scripts/pm"
            scripts.mkdir(parents=True)
            for name in ("github_api.py", "portable_file_lock.py", "github_observation.py",
                         "github_pr_snapshot.py", "pr-lifecycle-gate.py"):
                (scripts / name).write_bytes((ROOT / "scripts/pm" / name).read_bytes())
            task = {
                "repository": "eng-cc/oasis7", "issue_number": 2198, "pr_number": 42,
                "pr_url": "https://github.com/eng-cc/oasis7/pull/42",
                "merge_hold": {"kind": "normal_pr_ci_watch", "active": False},
            }
            first = GATE._run_observation(
                "42", task_root=effective, task=task, task_uid=task_uid,
                effective_root=effective, client=self.client, watch_mode=False,
            )
            self.assertFalse(first["cache_hit"])
            gate_path = scripts / "pr-lifecycle-gate.py"
            gate_path.write_bytes(gate_path.read_bytes() + b"\n# candidate semantics changed\n")
            second = GATE._run_observation(
                "42", task_root=effective, task=task, task_uid=task_uid,
                effective_root=effective, client=self.client, watch_mode=False,
            )
            self.assertFalse(second["cache_hit"], "changing candidate evaluator must create a new observation key")
            lock_path = scripts / "portable_file_lock.py"
            lock_path.write_bytes(lock_path.read_bytes() + b"\n# lock behavior changed\n")
            third = GATE._run_observation(
                "42", task_root=effective, task=task, task_uid=task_uid,
                effective_root=effective, client=self.client, watch_mode=False,
            )
            self.assertFalse(third["cache_hit"], "changing an effective helper dependency must create a new key")
            graphql = [call for call in self.transport.calls if call[1].endswith("/graphql")]
            self.assertEqual(len(graphql), 3)

    def test_hot_requests_have_no_conditional_headers_or_secret_telemetry(self):
        GATE.load_live("42", client=self.client, repository_hint="eng-cc/oasis7",
                       number_hint=42, effective_root=ROOT)
        graphql = [call for call in self.transport.calls if call[1].endswith("/graphql")]
        self.assertEqual(len(graphql), 1)
        headers = {key.casefold() for key in graphql[0][3]}
        self.assertNotIn("if-none-match", headers)
        self.assertNotIn("if-modified-since", headers)
        telemetry = b"\n".join(path.read_bytes() for path in self.client.telemetry_dir.glob("*.jsonl"))
        self.assertNotIn(b"test-token", telemetry)
        self.assertNotIn(b"Authorization", telemetry)

    def test_bound_selector_mismatch_fails_before_any_http(self):
        with self.assertRaises(ValueError):
            GATE.load_live("#43", client=self.client, repository_hint="eng-cc/oasis7",
                            number_hint=42, effective_root=ROOT)
        self.assertEqual(self.transport.calls, [])

    def test_missing_task_uid_is_rejected_before_client_creation_or_http(self):
        with tempfile.TemporaryDirectory() as temp:
            old_argv = GATE.sys.argv
            GATE.sys.argv = ["pr-lifecycle-gate.py", "42", "--root", temp, "--json"]
            try:
                with mock.patch.object(GATE, "_github_api_client",
                                       side_effect=AssertionError("client created before UID validation")), \
                     contextlib.redirect_stdout(io.StringIO()):
                    status = GATE.main()
            finally:
                GATE.sys.argv = old_argv
        self.assertEqual(status, 2)
        self.assertEqual(self.transport.calls, [])

    def test_observe_cli_shares_cache_but_never_emits_formal_authority(self):
        task_uid = "task_11111111111111111111111111111111"
        with tempfile.TemporaryDirectory() as temp:
            task_root = pathlib.Path(temp)
            mapping_path = task_root / ".pm/github-project-sync/tasks.json"
            mapping_path.parent.mkdir(parents=True)
            mapping_path.write_text(json.dumps({"tasks": {task_uid: {
                "repository": "eng-cc/oasis7", "issue_number": 2198,
                "pr_number": 42, "pr_url": "https://github.com/eng-cc/oasis7/pull/42",
                "canonical_worktree": str(task_root),
                "merge_hold": {"kind": "normal_pr_ci_watch", "active": False},
            }}}), encoding="utf-8")
            for expected_cache_hit in (False, True):
                old_argv = GATE.sys.argv
                GATE.sys.argv = [
                    "pr-lifecycle-gate.py", "42", "--root", str(task_root),
                    "--tool-root", str(ROOT), "--task-uid", task_uid, "--observe", "--json",
                ]
                output = io.StringIO()
                try:
                    with mock.patch.object(GATE, "_github_api_client", return_value=self.client), \
                         contextlib.redirect_stdout(output):
                        status = GATE.main()
                finally:
                    GATE.sys.argv = old_argv
                self.assertEqual(status, 0)
                observed = json.loads(output.getvalue())
                self.assertEqual(observed["evidence_mode"], "observation")
                self.assertTrue(observed["candidate_ready"])
                self.assertFalse(observed["ready_for_merge"])
                self.assertTrue(observed["requires_live_gate"])
                self.assertEqual(observed["cache_hit"], expected_cache_hit)
                self.assertNotIn("readiness_receipt", observed)
                self.assertEqual(observed.get("formal_gate_command"), None)
            old_argv = GATE.sys.argv
            GATE.sys.argv = [
                "pr-lifecycle-gate.py", "42", "--root", str(task_root),
                "--tool-root", str(ROOT), "--task-uid", task_uid,
                "--observe", "--watch", "--json",
            ]
            output = io.StringIO()
            try:
                with mock.patch.object(GATE, "_github_api_client", return_value=self.client), \
                     contextlib.redirect_stdout(output):
                    status = GATE.main()
            finally:
                GATE.sys.argv = old_argv
            watched = json.loads(output.getvalue())
            self.assertEqual(status, 0)
            self.assertTrue(watched["candidate_ready"])
            self.assertFalse(watched["ready_for_merge"])
            self.assertNotIn("readiness_receipt", watched)
            self.assertIn("--task-uid", watched["formal_gate_command"])
            graphql = [call for call in self.transport.calls if call[1].endswith("/graphql")]
            self.assertEqual(len(graphql), 1, "second identical observation must use the shared cache")
            self.assertEqual(len(self.transport.calls), len(graphql), "observation must not issue REST writes")
            for method, _url, body, _headers, _timeout in graphql:
                self.assertEqual(method, "POST")
                query = json.loads(body.decode("utf-8"))["query"]
                self.assertNotRegex(query, r"(?i)\bmutation\b", "observation is read-only")

    def test_304_is_not_treated_as_a_valid_conditional_snapshot(self):
        transport = CountingTransport(not_modified=True)
        client = API.GitHubAPIClient(
            "test-token", transport=transport,
            state_root=pathlib.Path(self.temp.name) / "not-modified-state",
            clock=lambda: 1790860000.0, sleeper=lambda _seconds: None,
            random_value=lambda: 0,
        )
        with self.assertRaises(API.APIError) as caught:
            GATE.load_live("42", client=client, repository_hint="eng-cc/oasis7",
                           number_hint=42, effective_root=ROOT)
        self.assertEqual(caught.exception.status_code, 304)
        self.assertEqual(len(transport.calls), 1)
        self.assertTrue(all(call[1].endswith("/graphql") for call in transport.calls))

    def test_one_task_refresh_retains_its_independent_project_budget_contract(self):
        source = (ROOT / "scripts/pm/github-project-task.py").read_text(encoding="utf-8")
        # This secondary contract is retained until Project E's behavioral tests
        # cover its injected client; it is not used as PR-watch budget evidence.
        self.assertIn("def command_refresh_task", source)

    def test_terminal_closeout_runs_pre_and_post_selected_task_audits_around_mutation(self):
        result = subprocess.run(
            ["bash", str(ROOT / "scripts/pm/task-closeout-audit-order.test.sh")],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("task-closeout-audit-order.test: OK", result.stdout)


if __name__ == "__main__":
    unittest.main()
