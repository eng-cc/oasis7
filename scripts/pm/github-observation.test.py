#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import threading
import time
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


API = load_module("github_api_for_observation_test", ROOT / "scripts/pm/github_api.py")
OBS = load_module("github_observation_under_test", ROOT / "scripts/pm/github_observation.py")
GATE = load_module("pr_gate_for_observation_test", ROOT / "scripts/pm/pr-lifecycle-gate.py")


class FakeClock:
    def __init__(self):
        self.now = time.time()

    def __call__(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds


class FakeTransport:
    def __init__(self, count=20):
        self.calls = []
        self.count = count

    def __call__(self, method, url, headers, body, timeout):
        self.calls.append((method, url, body))
        if len(self.calls) > self.count:
            raise AssertionError("observation sent more requests than the test budget")
        return API.HTTPResponse(200, {}, json.dumps({"data": {
            "viewer": {"login": "observer"},
            "rateLimit": {"cost": 1, "remaining": 450, "used": 50, "limit": 500,
                          "resetAt": "2030-01-01T00:00:00Z"},
        }}))


class ObservationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.clock = FakeClock()
        self.transport = FakeTransport()
        API._PROCESS_PAUSES.clear()
        self.client = API.GitHubAPIClient("observation-test-token", transport=self.transport,
                                          state_root=pathlib.Path(self.temp.name) / "state",
                                          clock=self.clock, sleeper=self.clock.sleep,
                                          random_value=lambda: 0)
        self.data = self.snapshot()
        self.fetch_count = 0

    def tearDown(self):
        self.temp.cleanup()

    @staticmethod
    def snapshot(body="PR body", remaining=450):
        return {
            "repository": "owner/repo", "number": 42, "url": "https://github.com/owner/repo/pull/42",
            "state": "OPEN", "isDraft": False, "body": body, "headRefName": "feature",
            "headRefOid": "a" * 40, "baseRefName": "main", "baseRefOid": "b" * 40,
            "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN", "reviewDecision": "APPROVED",
            "comments": [{"id": "IC_1", "body": "review comment", "createdAt": "2026-10-01T00:00:00Z",
                           "author": {"login": "reviewer"}}],
            "reviews": [{"id": "R_1", "body": "approved", "state": "APPROVED",
                          "submittedAt": "2026-10-01T00:00:00Z", "author": {"login": "reviewer"}}],
            "threads": [{"id": "T_1", "isResolved": True}],
            "statusCheckRollup": [{"databaseId": 7, "name": "CI", "conclusion": "SUCCESS",
                                    "checkSuite": {"app": {"databaseId": 1}}}],
            "policy_discovery": {"status": "resolved", "source": "repository_rulesets",
                                 "required_status_checks": [{"context": "CI", "app_id": 1}],
                                 "active_rule_types": ["required_status_checks"]},
            "snapshot_metadata": {"complete": True, "query_version": "oasis7-pr-snapshot/v1",
                                  "connections": {"comments": {"complete": True}}},
        }

    def fetch(self):
        self.fetch_count += 1
        self.client.graphql("query Observation { viewer { login } rateLimit { remaining } }",
                            operation="pr_snapshot_observation",
                            context={"script": "github-observation.test.py", "operation": "pr_snapshot_observation",
                                     "api_family": "graphql", "phase": "observation"})
        return self.data

    def evaluate(self, _data):
        return True

    def test_observation_is_derived_only_and_never_a_readiness_receipt(self):
        result = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate,
                                  task_uid="task_0123456789abcdef0123456789abcdef",
                                  context={"effective_policy": "policy-a"})
        self.assertEqual(result["evidence_mode"], "observation")
        self.assertTrue(result["candidate_ready"])
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(result["requires_live_gate"])
        self.assertNotIn("readiness_receipt", result)
        observation_files = list(self.client.observations_dir.glob("*.json"))
        self.assertEqual(len(observation_files), 1)
        raw_cache = observation_files[0].read_text(encoding="utf-8")
        self.assertNotIn("PR body", raw_cache)
        self.assertNotIn("review comment", raw_cache)

    def test_fresh_observation_cache_avoids_transport_and_tracks_context_isolation(self):
        task = "task_0123456789abcdef0123456789abcdef"
        first = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate,
                                 task_uid=task, context={"effective_policy": "policy-a"})
        cached = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate,
                                  task_uid=task, context={"effective_policy": "policy-a"})
        self.assertFalse(first["cache_hit"])
        self.assertTrue(cached["cache_hit"])
        self.assertEqual(len(self.transport.calls), 1)
        changed_policy = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate,
                                          task_uid=task, context={"effective_policy": "policy-b"})
        changed_task = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate,
                                         task_uid="task_1123456789abcdef0123456789abcdef",
                                         context={"effective_policy": "policy-a"})
        self.assertFalse(changed_policy["cache_hit"])
        self.assertFalse(changed_task["cache_hit"])
        self.assertEqual(len(self.transport.calls), 3)

    def test_observation_uses_latest_review_and_keeps_current_blockers(self):
        task_uid = "task_0123456789abcdef0123456789abcdef"
        task = {"merge_hold": {"kind": "normal_pr_ci_watch", "active": False}}
        evaluate = lambda data: GATE._observation_candidate(data, task, task_uid)

        superseded = self.snapshot(body=f"Task: {task_uid}")
        superseded["comments"] = [{"id": "IC_OK", "body": "looks good"}]
        superseded["reviews"] = [
            {"id": "R_old", "state": "CHANGES_REQUESTED", "body": "", "author": {"login": "alice"},
             "submittedAt": "2026-09-29T10:00:00Z"},
            {"id": "R_new", "state": "APPROVED", "body": "", "author": {"login": "alice"},
             "submittedAt": "2026-09-30T10:00:00Z"},
        ]
        self.data = superseded
        candidate = OBS.observe_once(
            self.client, "owner/repo", 42, self.fetch, evaluate, task_uid=task_uid,
            context={"review_history": "superseded_changes_requested"},
        )
        self.assertTrue(candidate["candidate_ready"])
        self.assertFalse(candidate["ready_for_merge"])
        self.assertTrue(candidate["requires_live_gate"])
        self.assertNotIn("readiness_receipt", candidate)

        current_request = copy.deepcopy(superseded)
        current_request["reviews"] = [
            {"id": "R_current", "state": "CHANGES_REQUESTED", "body": "", "author": {"login": "alice"},
             "submittedAt": "2026-10-01T10:00:00Z"},
        ]
        self.data = current_request
        blocked_review = OBS.observe_once(
            self.client, "owner/repo", 42, self.fetch, evaluate, task_uid=task_uid,
            context={"review_history": "current_changes_requested"},
        )
        self.assertFalse(blocked_review["candidate_ready"])
        self.assertFalse(blocked_review["ready_for_merge"])

        unresolved = copy.deepcopy(superseded)
        unresolved["threads"] = [{"id": "T_current", "isResolved": False}]
        self.data = unresolved
        blocked_thread = OBS.observe_once(
            self.client, "owner/repo", 42, self.fetch, evaluate, task_uid=task_uid,
            context={"review_history": "unresolved_thread"},
        )
        self.assertFalse(blocked_thread["candidate_ready"])
        self.assertFalse(blocked_thread["ready_for_merge"])

    def test_observation_ignores_actionable_body_from_superseded_review(self):
        task_uid = "task_0123456789abcdef0123456789abcdef"
        task = {"merge_hold": {"kind": "normal_pr_ci_watch", "active": False}}
        data = self.snapshot(body=f"Task: {task_uid}")
        data["comments"] = [{"id": "IC_OK", "body": "looks good"}]
        data["reviews"] = [
            {"id": "R_old", "state": "CHANGES_REQUESTED", "body": "[P1] Fix this error",
             "author": {"login": "alice"}, "submittedAt": "2026-09-29T10:00:00Z"},
            {"id": "R_new", "state": "APPROVED", "body": "Resolved",
             "author": {"login": "alice"}, "submittedAt": "2026-09-30T10:00:00Z"},
        ]

        candidate = GATE._observation_candidate(data, task, task_uid)

        self.assertTrue(candidate)

        current_request = copy.deepcopy(data)
        current_request["reviews"][-1] = {
            "id": "R_current", "state": "CHANGES_REQUESTED", "body": "[P1] Fix this error",
            "author": {"login": "alice"}, "submittedAt": "2026-10-01T10:00:00Z",
        }
        self.assertFalse(GATE._observation_candidate(current_request, task, task_uid))

    def test_business_change_resets_interval_and_volatile_rate_fields_do_not_change_digest(self):
        result1 = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate)
        self.clock.now += result1["retry_after_seconds"] + 1
        first_calls = len(self.transport.calls)
        self.data["rateLimit"] = {"remaining": 2, "cost": 90, "resetAt": "2031-01-01T00:00:00Z"}
        result2 = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate)
        self.assertEqual(first_calls + 1, len(self.transport.calls))
        self.assertFalse(result2["changed"])
        self.assertEqual(result2["retry_after_seconds"], 120)
        self.clock.now += result2["retry_after_seconds"] + 1
        self.data["body"] = "updated PR body"
        result3 = OBS.observe_once(self.client, "owner/repo", 42, self.fetch, self.evaluate)
        self.assertTrue(result3["changed"])
        self.assertEqual(result3["retry_after_seconds"], 60)

    def test_incomplete_snapshot_is_rejected_without_candidate_readiness(self):
        def incomplete():
            self.client.graphql("query Incomplete { viewer { login } }", operation="incomplete")
            data = self.snapshot()
            data["snapshot_metadata"]["complete"] = False
            return data

        result = OBS.observe_once(self.client, "owner/repo", 42, incomplete, self.evaluate)
        self.assertEqual(result["status"], "capability_blocked")
        self.assertFalse(result["candidate_ready"])
        self.assertNotIn("readiness_receipt", result)

    def test_corrupt_observation_cache_falls_back_to_one_fresh_non_authorizing_read(self):
        key = OBS.observation_key(self.client, "owner/repo", 42,
                                  task_uid="task_0123456789abcdef0123456789abcdef",
                                  context={"effective_policy": "policy-a"})
        path = self.client.state_path("observations", key)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("{broken", encoding="utf-8")
        result = OBS.observe_once(self.client, "owner/repo", 42, self.fetch,
                                  lambda _data: False,
                                  task_uid="task_0123456789abcdef0123456789abcdef",
                                  context={"effective_policy": "policy-a"})
        self.assertEqual(result["status"], "observed")
        self.assertFalse(result["candidate_ready"])
        self.assertFalse(result["ready_for_merge"])
        self.assertFalse(result["cache_hit"])
        self.assertEqual(len(self.transport.calls), 1)
        telemetry = b"\n".join(path.read_bytes() for path in self.client.telemetry_dir.glob("*.jsonl"))
        self.assertIn(b"observation_cache_corrupt", telemetry)

    def test_old_observation_schema_is_rejected_before_current_fetch(self):
        key = OBS.observation_key(self.client, "owner/repo", 42,
                                  task_uid="task_0123456789abcdef0123456789abcdef",
                                  context={"effective_policy": "policy-a"})
        self.client.write_state("observations", key, {
            "schema": "oasis7.github-pr-observation/v0", "query_version": "old-query",
            "snapshot_digest": "old", "next_read_at_epoch": self.clock.now + 1000,
            "observation": {"status": "observed", "candidate_ready": True,
                            "ready_for_merge": False, "requires_live_gate": True},
        })
        result = OBS.observe_once(self.client, "owner/repo", 42, self.fetch,
                                  lambda _data: False,
                                  task_uid="task_0123456789abcdef0123456789abcdef",
                                  context={"effective_policy": "policy-a"})
        self.assertEqual(result["status"], "observed")
        self.assertFalse(result["candidate_ready"])
        self.assertFalse(result["cache_hit"])
        self.assertEqual(len(self.transport.calls), 1)

    def test_concurrent_observers_single_flight_under_shared_file_lock(self):
        entered = threading.Event()
        release = threading.Event()
        fetch_calls = []

        def slow_fetch():
            fetch_calls.append(1)
            entered.set()
            release.wait(timeout=2)
            self.client.graphql("query Concurrent { viewer { login } }", operation="concurrent")
            return self.data

        results = []
        first = threading.Thread(target=lambda: results.append(
            OBS.observe_once(self.client, "owner/repo", 42, slow_fetch, self.evaluate)))
        second = threading.Thread(target=lambda: results.append(
            OBS.observe_once(self.client, "owner/repo", 42, slow_fetch, self.evaluate)))
        first.start()
        self.assertTrue(entered.wait(timeout=2))
        second.start()
        time.sleep(0.05)
        release.set()
        first.join(timeout=2)
        second.join(timeout=2)
        self.assertEqual(len(fetch_calls), 1)
        self.assertEqual(len(results), 2)
        self.assertEqual(len(self.transport.calls), 1)
        self.assertEqual(sum(bool(result.get("cache_hit")) for result in results), 1)

    def test_eight_subprocess_observers_share_one_network_snapshot(self):
        state_root = pathlib.Path(self.temp.name) / "multi-process-state"
        counter = pathlib.Path(self.temp.name) / "network-sends"
        script = r'''import importlib.util, json, pathlib, sys, time
api_path, obs_path, state_root, counter = map(pathlib.Path, sys.argv[1:])
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module
api = load("api_child", api_path)
obs = load("obs_child", obs_path)
client = None
def transport(method, url, headers, body, timeout):
    count_path = pathlib.Path(counter)
    with api._FILE_LOCK.locked_file(str(count_path) + ".lock", timeout=2):
        count = int(count_path.read_text() or "0") if count_path.exists() else 0
        count_path.write_text(str(count + 1))
    time.sleep(0.15)
    payload = {"data": {"viewer": {"id": "123"}, "rateLimit": {
        "cost": 1, "remaining": 450, "used": 50, "limit": 500,
        "resetAt": "2030-01-01T00:00:00Z"}}}
    return api.HTTPResponse(200, {}, json.dumps(payload))
client = api.GitHubAPIClient("shared-account-token", transport=transport, state_root=state_root)
def fetch():
    client.graphql("query Snapshot { viewer { id } rateLimit { remaining } }", operation="snapshot")
    return {"repository": "owner/repo", "number": 42, "state": "OPEN", "isDraft": False,
            "headRefOid": "a" * 40, "body": "review-needed", "comments": [], "reviews": [],
            "threads": [], "statusCheckRollup": [],
            "snapshot_metadata": {"complete": True, "query_version": "oasis7-pr-snapshot/v1"}}
result = obs.observe_once(client, "owner/repo", 42, fetch, lambda _: False,
                          task_uid="task_0123456789abcdef0123456789abcdef",
                          context={"effective_policy": "frozen"})
print(json.dumps(result, sort_keys=True))
'''
        command = [sys.executable, "-c", script, str(ROOT / "scripts/pm/github_api.py"),
                   str(ROOT / "scripts/pm/github_observation.py"), str(state_root), str(counter)]
        processes = [subprocess.Popen(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                     for _ in range(8)]
        outputs = []
        for process in processes:
            stdout, stderr = process.communicate(timeout=10)
            self.assertEqual(process.returncode, 0, stderr)
            outputs.append(json.loads(stdout))
        self.assertEqual(counter.read_text(encoding="utf-8"), "1")
        self.assertEqual(sum(not item["cache_hit"] for item in outputs), 1)
        self.assertTrue(all(item["status"] == "observed" for item in outputs))

    def test_watch_preserves_bounded_unchanged_policy(self):
        result = OBS.watch(self.client, "owner/repo", 42, self.fetch,
                           lambda _data: False, max_polls=4, max_unchanged_polls=1,
                           sleeper=self.clock.sleep)
        self.assertEqual(result["status"], "external_wait")
        self.assertEqual(result["reason"], "stable_pr_watch_unchanged_budget_exhausted")
        self.assertFalse(result["ready_for_merge"])
        self.assertEqual(len(self.transport.calls), 2)

    def test_watch_returns_on_meaningful_business_change_before_sleeping_through_repeat(self):
        task_uid = "task_0123456789abcdef0123456789abcdef"
        old = self.snapshot(body=f"Task: {task_uid}")
        old["comments"] = [{"id": "C_1", "body": "status: waiting", "author": {"login": "bot"}}]
        changed = copy.deepcopy(old)
        changed["comments"][0]["body"] = "status: resolved"
        snapshots = [old, changed, changed]

        def fetch_sequence():
            self.data = snapshots[min(self.fetch_count, len(snapshots) - 1)]
            return self.fetch()

        result = OBS.watch(
            self.client, "owner/repo", 42, fetch_sequence, lambda _data: False,
            task_uid=task_uid, min_interval=1, max_interval=4, max_polls=4,
            max_unchanged_polls=1, sleeper=self.clock.sleep,
        )
        self.assertEqual(result["status"], "observed")
        self.assertTrue(result["changed"])
        self.assertFalse(result["candidate_ready"])
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(result["requires_live_gate"])
        self.assertNotIn("readiness_receipt", result)
        self.assertEqual(self.fetch_count, 2)
        self.assertEqual(len(self.transport.calls), 2)

    def test_watch_returns_on_hold_context_change_with_same_pr_projection(self):
        task_uid = "task_0123456789abcdef0123456789abcdef"
        task = {"merge_hold": {"kind": "user_requested_merge_hold", "active": True}}
        context = {"merge_hold": copy.deepcopy(task["merge_hold"])}
        sleeps = []
        fetches = []

        def fetch_same_pr():
            fetches.append(True)
            data = self.snapshot(body=f"Task: {task_uid}")
            return data

        def sleep_and_change_hold(seconds):
            sleeps.append(seconds)
            task["merge_hold"] = {"kind": "manual_packaging_ci_hold", "active": True}
            context["merge_hold"] = copy.deepcopy(task["merge_hold"])

        result = OBS.watch(
            self.client, "owner/repo", 42, fetch_same_pr,
            lambda data: GATE._observation_candidate(data, task, task_uid),
            task_uid=task_uid, context=context, min_interval=1, max_interval=4,
            max_polls=4, max_unchanged_polls=3, sleeper=sleep_and_change_hold,
        )
        self.assertEqual(result["status"], "observed")
        self.assertTrue(result["changed"])
        self.assertFalse(result["candidate_ready"])
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(result["requires_live_gate"])
        self.assertNotIn("readiness_receipt", result)
        self.assertEqual(len(fetches), 2)
        self.assertEqual(len(self.transport.calls), 0)
        self.assertEqual(len(sleeps), 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
