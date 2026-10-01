#!/usr/bin/env python3
from __future__ import annotations

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
