#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import time
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("github_api_concurrency_test", ROOT / "scripts/pm/github_api.py")
assert SPEC and SPEC.loader
API = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(API)
PROCESS_CODE = "\n".join([
    "import importlib.util,json,pathlib,sys,time",
    "api_path,state_root,counter,ready=map(pathlib.Path,sys.argv[1:])",
    "spec=importlib.util.spec_from_file_location('api_child',api_path)",
    "api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)",
    "def update(path):",
    "    with api._FILE_LOCK.locked_file(str(path)+'.lock',timeout=6):",
    "        value=int(path.read_text() or '0') if path.exists() else 0",
    "        path.write_text(str(value+1))",
    "def transport(method,url,headers,body,timeout):",
    "    update(counter);time.sleep(0.8)",
    "    data={'viewer':{'login':'confirmed'},'rateLimit':{'cost':1,'remaining':400,'used':100,'limit':500,'resetAt':'2030-01-01T00:00:00Z'}}",
    "    return api.HTTPResponse(200,{},json.dumps({'data':data}))",
    "client=api.GitHubAPIClient('recovery-shared-token',transport=transport,state_root=state_root)",
    "update(ready);deadline=time.monotonic()+8",
    "while int(ready.read_text() or '0')<8:",
    "    if time.monotonic()>deadline:raise RuntimeError('recovery barrier timed out')",
    "    time.sleep(0.005)",
    "try:",
    "    data=client.graphql('query Recovery { viewer { login } }',operation='recovery')",
    "    result={'status':'passed','login':data['viewer']['login']}",
    "except api.APIError as exc:",
    "    result=exc.as_dict()",
    "print(json.dumps(result,sort_keys=True))",
])


class GitHubAPIConcurrencyTests(unittest.TestCase):
    def test_stale_success_write_preserves_pause_written_before_lock(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("success-race-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            pause = {
                "schema": "oasis7.github-api-budget/v1", "remaining": 380,
                "used": 120, "limit": 500, "cost": 1,
                "resetAt": "2030-01-01T00:00:00Z", "observed_at": "concurrent-limit",
                "observed_at_epoch": now, "pause_reason": "secondary_rate_limit",
                "pause_until": int(now + 120), "pause_until_epoch": now + 120,
                "probe_until": 0, "probe_until_epoch": 0,
            }
            original_lock = client.file_lock

            @contextlib.contextmanager
            def write_external_pause_then_lock(key, *, timeout=2.0):
                # Model another process committing its throttle state after the
                # success response was read but before this writer takes lock.
                API._atomic_json(client._budget_path(), pause)
                with original_lock(key, timeout=timeout):
                    yield

            API._PROCESS_PAUSES.clear()
            client.file_lock = write_external_pause_then_lock
            client._record_success(410, 90, 500, "2030-01-01T00:00:00Z", 1, now,
                                   recovery_probe=False, graphql=True)

            observed = client.rate_limit_snapshot()
            self.assertEqual(observed["status"], "external_wait")
            self.assertEqual(observed["reason"], "secondary_rate_limit")
            self.assertGreaterEqual(observed["retry_after_seconds"], 119)

    def test_stale_success_does_not_replace_newer_observation(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("stale-observation-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            client.write_state("budget", client._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 380,
                "used": 120, "limit": 500, "cost": 1,
                "resetAt": "2030-01-01T00:00:00Z", "observed_at": "newer",
                "observed_at_epoch": now, "pause_reason": None,
                "pause_until": 0, "pause_until_epoch": 0,
                "probe_until": 0, "probe_until_epoch": 0,
            })
            API._PROCESS_PAUSES.clear()
            client._record_success(410, 90, 500, "2030-01-01T00:00:00Z", 1, now - 1,
                                   recovery_probe=False, graphql=True)

            state = client._load_budget_state(strict=True)
            self.assertEqual(state["remaining"], 380)
            self.assertEqual(state["observed_at"], "newer")

    def test_rate_limit_merge_keeps_the_longer_existing_pause(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("rate-limit-merge-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            client.write_state("budget", client._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 380,
                "used": 120, "limit": 500, "cost": 1,
                "resetAt": "2030-01-01T00:00:00Z", "observed_at": "older",
                "observed_at_epoch": now - 1, "pause_reason": "secondary_rate_limit",
                "pause_until": int(now + 180), "pause_until_epoch": now + 180,
                "probe_until": 0, "probe_until_epoch": 0,
            })
            API._PROCESS_PAUSES.clear()
            client._record_rate_limit("primary_rate_limit", 60, None, 0, 500, 500, 1,
                                      now, graphql=True)

            state = client._load_budget_state(strict=True)
            self.assertEqual(state["pause_reason"], "secondary_rate_limit")
            self.assertGreaterEqual(state["pause_until_epoch"], now + 179)
            self.assertEqual(state["remaining"], 0)

    def test_local_rate_limit_fallback_keeps_longer_pause_without_claiming_shared_state(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("rate-limit-fallback-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            client.write_state("budget", client._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 380,
                "used": 120, "limit": 500, "cost": 1,
                "resetAt": "2030-01-01T00:00:00Z", "observed_at": "older",
                "observed_at_epoch": now - 1, "pause_reason": "secondary_rate_limit",
                "pause_until": int(now + 180), "pause_until_epoch": now + 180,
                "probe_until": 0, "probe_until_epoch": 0,
            })
            API._PROCESS_PAUSES.clear()

            @contextlib.contextmanager
            def unavailable_lock(_key, *, timeout=2.0):
                raise TimeoutError("fixture lock timeout")
                yield

            client.file_lock = unavailable_lock
            client._record_rate_limit("primary_rate_limit", 60, None, 0, 500, 500, 1,
                                      now, graphql=True)
            state = API._PROCESS_PAUSES[client._rate_state_key]
            self.assertEqual(state["pause_reason"], "secondary_rate_limit")
            self.assertGreaterEqual(state["pause_until_epoch"], now + 179)
            self.assertIs(state["shared_persistence"], False)

    def test_stale_probe_success_cannot_clear_newer_pause_or_owner(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("probe-owner-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            client.write_state("budget", client._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 450,
                "used": 50, "limit": 500, "cost": 1,
                "observed_at": "current", "observed_at_epoch": now,
                "pause_reason": "secondary_rate_limit", "pause_until": int(now + 120),
                "pause_until_epoch": now + 120, "probe_until": int(now + 30),
                "probe_until_epoch": now + 30, "probe_owner": "current-owner",
            })
            API._PROCESS_PAUSES.clear()
            client._record_success(440, 60, 500, "2030-01-01T00:00:00Z", 1, now + 1,
                                   recovery_probe="stale-owner", graphql=True)

            state = client._load_budget_state(strict=True)
            self.assertEqual(state["pause_reason"], "secondary_rate_limit")
            self.assertGreaterEqual(state["pause_until_epoch"], now + 119)
            self.assertEqual(state["probe_owner"], "current-owner")

    def test_probe_finish_only_releases_matching_owner_and_preserves_pause(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("probe-finish-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            client.write_state("budget", client._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 450,
                "used": 50, "limit": 500, "cost": 1,
                "observed_at": "current", "observed_at_epoch": now,
                "pause_reason": "secondary_rate_limit", "pause_until": int(now + 120),
                "pause_until_epoch": now + 120, "probe_until": int(now + 30),
                "probe_until_epoch": now + 30, "probe_owner": "current-owner",
            })
            API._PROCESS_PAUSES.clear()
            client._finish_probe("stale-owner", success=False)
            stale_state = client._load_budget_state(strict=True)
            self.assertEqual(stale_state["probe_owner"], "current-owner")

            client._finish_probe("current-owner", success=False)
            state = client._load_budget_state(strict=True)
            self.assertIsNone(state["probe_owner"])
            self.assertGreater(state["probe_until_epoch"], now)
            self.assertEqual(state["pause_reason"], "secondary_rate_limit")
            self.assertGreaterEqual(state["pause_until_epoch"], now + 119)

    def test_active_probe_blocks_even_with_fresh_positive_budget(self):
        with tempfile.TemporaryDirectory() as temp:
            state_root = pathlib.Path(temp) / "state"
            client = API.GitHubAPIClient("probe-block-token", transport=lambda *_: None,
                                         state_root=state_root)
            now = time.time()
            client.write_state("budget", client._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 450,
                "used": 50, "limit": 500, "cost": 1,
                "observed_at": "current", "observed_at_epoch": now,
                "pause_until": 0, "pause_until_epoch": 0,
                "probe_until": int(now + 30), "probe_until_epoch": now + 30,
                "probe_owner": "current-owner",
            })
            API._PROCESS_PAUSES.clear()
            with self.assertRaises(API.APIError) as caught:
                client.graphql("query Read { viewer { login } }", operation="read")
            self.assertEqual(caught.exception.kind, "rate_limit_probe_pending")


    def test_eight_processes_release_exactly_one_recovery_probe(self):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            state_root, counter, ready = root / "state", root / "sends", root / "ready"
            seed = API.GitHubAPIClient("recovery-shared-token", transport=lambda *_: None,
                                       state_root=state_root)
            now = time.time()
            seed.write_state("budget", seed._rate_state_key, {
                "schema": "oasis7.github-api-budget/v1", "remaining": 0, "used": 500,
                "limit": 500, "cost": 1, "resetAt": None,
                "observed_at": "before-reset", "observed_at_epoch": now - 120,
                "pause_reason": "primary_rate_limit", "pause_until": int(now - 1),
                "pause_until_epoch": now - 1, "probe_until_epoch": 0,
            })
            API._PROCESS_PAUSES.clear()
            command = [sys.executable, "-c", PROCESS_CODE,
                       str(ROOT / "scripts/pm/github_api.py"), str(state_root), str(counter), str(ready)]
            processes = [subprocess.Popen(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                         for _ in range(8)]
            results = []
            for process in processes:
                stdout, stderr = process.communicate(timeout=15)
                self.assertEqual(process.returncode, 0, stderr)
                results.append(json.loads(stdout))
            self.assertEqual(counter.read_text(encoding="utf-8"), "1")
            self.assertEqual(sum(item.get("status") == "passed" for item in results), 1)
            self.assertEqual(sum(item.get("reason") == "rate_limit_probe_pending" for item in results), 7)


if __name__ == "__main__":
    unittest.main(verbosity=2)
