#!/usr/bin/env python3
"""Project adapters delegate broad budget decisions to the shared client."""
import importlib.util
import pathlib


path = pathlib.Path(__file__).with_name("github-project-sync.py")
spec = importlib.util.spec_from_file_location("sync", path)
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)


class FakeClient:
    def __init__(self, result):
        self.result = result
        self.calls = []

    def rate_limit_guard(self, **kwargs):
        self.calls.append(kwargs)
        return self.result


expected_call = {
    "minimum_remaining": 100,
    "max_age_seconds": 300,
    "operation": "project_broad_preflight",
    "context": {"script": "github-project-sync.py"},
}
for result in (
    {"status": "capability_blocked", "reason": "graphql_budget_insufficient"},
    {"status": "capability_blocked", "reason": "graphql_budget_unknown"},
    {"status": "external_wait", "reason": "primary_rate_limit"},
    {"status": "ok", "remaining": 100},
):
    client = FakeClient(result)
    received = []

    def injected(token=None):
        received.append(token)
        return client

    sync.github_api_client = injected
    assert sync.broad_rate_limit_guard("explicit-token") == result
    assert received == ["explicit-token"]
    assert client.calls == [expected_call]

print("graphql-rate-limit-guard.test: OK")
