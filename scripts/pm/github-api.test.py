#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("github_api_under_test", ROOT / "scripts/pm/github_api.py")
assert SPEC and SPEC.loader
API = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(API)


class FakeClock:
    def __init__(self, now: float | None = None):
        self.now = time.time() if now is None else now
        self.sleeps: list[float] = []

    def __call__(self) -> float:
        return self.now

    def sleep(self, seconds: float) -> None:
        self.sleeps.append(seconds)
        self.now += seconds


class FakeTransport:
    def __init__(self, responses):
        self.responses = list(responses)
        self.calls: list[tuple[str, str, dict[str, str], bytes | None, float]] = []

    def __call__(self, method, url, headers, body, timeout):
        self.calls.append((method, url, dict(headers), body, timeout))
        if not self.responses:
            raise AssertionError("unexpected HTTP send")
        response = self.responses.pop(0)
        if isinstance(response, Exception):
            raise response
        return response


def response(status=200, payload=None, headers=None):
    payload = {} if payload is None else payload
    return API.HTTPResponse(status, headers or {}, json.dumps(payload))


class GitHubAPITests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.state_root = pathlib.Path(self.temp.name) / "state"
        self.clock = FakeClock()
        API._PROCESS_PAUSES.clear()

    def tearDown(self):
        self.temp.cleanup()

    def client(self, transport, token="test-token"):
        return API.GitHubAPIClient(token, transport=transport, state_root=self.state_root,
                                   clock=self.clock, sleeper=self.clock.sleep, random_value=lambda: 0)

    def test_one_graphql_transport_call_records_real_rate_limit_cost(self):
        transport = FakeTransport([response(payload={"data": {
            "repository": {"nameWithOwner": "owner/repo"},
            "rateLimit": {"cost": 3, "remaining": 477, "used": 23, "limit": 500,
                          "resetAt": "2026-10-01T13:00:00Z"},
        }})])
        client = self.client(transport)
        data = client.graphql("query Snapshot { repository { nameWithOwner } }", operation="snapshot")
        self.assertEqual(data["repository"]["nameWithOwner"], "owner/repo")
        self.assertEqual(len(transport.calls), 1)
        observed = client.rate_limit_snapshot(max_age_seconds=1)
        self.assertEqual(observed["remaining"], 477)
        self.assertEqual(observed["cost"], 3)

    def test_http_200_rate_limited_response_persists_pause_and_stops_later_calls(self):
        transport = FakeTransport([response(payload={"data": None, "errors": [{
            "message": "API rate limit exceeded", "extensions": {"code": "RATE_LIMITED"},
        }]})])
        client = self.client(transport, "primary-token")
        with self.assertRaises(API.APIError) as caught:
            client.graphql("query Snapshot { viewer { login } }", operation="snapshot")
        self.assertEqual(caught.exception.workflow_status, "external_wait")
        self.assertEqual(caught.exception.exit_code, 75)
        self.assertFalse(caught.exception.mutation_started)
        self.assertEqual(len(transport.calls), 1)

        API._PROCESS_PAUSES.clear()
        second_transport = FakeTransport([])
        second_client = self.client(second_transport, "primary-token")
        with self.assertRaises(API.APIError) as blocked:
            second_client.rest("GET", "repos/owner/repo", operation="repo")
        self.assertEqual(blocked.exception.workflow_status, "external_wait")
        self.assertEqual(len(second_transport.calls), 0)

    def test_ordinary_403_is_permission_failure_even_with_retry_after(self):
        transport = FakeTransport([response(403, {"message": "Resource not accessible by integration"},
                                             {"Retry-After": "5"})])
        client = self.client(transport)
        with self.assertRaises(API.APIError) as caught:
            client.rest("GET", "repos/owner/repo", operation="repo")
        self.assertEqual(caught.exception.kind, "permission_denied")
        self.assertEqual(caught.exception.workflow_status, "capability_blocked")
        self.assertEqual(len(transport.calls), 1)

    def test_secondary_rate_limit_403_without_retry_after_is_external_wait(self):
        transport = FakeTransport([response(403, {"message": "You have exceeded a secondary rate limit. Please wait."})])
        with self.assertRaises(API.APIError) as caught:
            self.client(transport, "secondary-403-token").rest("GET", "repos/owner/repo", operation="repo")
        self.assertEqual(caught.exception.kind, "secondary_rate_limit")
        self.assertEqual(caught.exception.workflow_status, "external_wait")
        self.assertEqual(caught.exception.exit_code, 75)
        self.assertGreaterEqual(caught.exception.retry_after_seconds or 0, 60)
        self.assertEqual(len(transport.calls), 1)

    def test_secondary_429_uses_minimum_cross_call_cooldown(self):
        transport = FakeTransport([response(429, {"message": "secondary throttle"})])
        client = self.client(transport, "secondary-token")
        with self.assertRaises(API.APIError) as caught:
            client.rest("GET", "repos/owner/repo", operation="repo")
        self.assertEqual(caught.exception.kind, "secondary_rate_limit")
        self.assertEqual(caught.exception.retry_after_seconds, 60)
        self.assertEqual(caught.exception.exit_code, 75)

    def test_query_server_failures_make_at_most_three_total_sends(self):
        transport = FakeTransport([
            response(503, {"message": "temporary failure"}),
            response(503, {"message": "temporary failure"}),
            response(503, {"message": "temporary failure"}),
        ])
        client = self.client(transport)
        with self.assertRaises(API.APIError):
            client.graphql("query Read { viewer { login } }", operation="read")
        self.assertEqual(len(transport.calls), 3)
        self.assertEqual(len(self.clock.sleeps), 2)

    def test_mutation_server_failure_is_uncertain_and_never_replayed(self):
        transport = FakeTransport([response(502, {"message": "upstream timeout"})])
        client = self.client(transport)
        with self.assertRaises(API.APIError) as caught:
            client.graphql("mutation Update { updateProjectV2ItemFieldValue(input: {}) { clientMutationId } }",
                           operation="update")
        self.assertTrue(caught.exception.uncertain)
        self.assertTrue(caught.exception.mutation_started)
        self.assertEqual(caught.exception.workflow_status, "uncertain")
        self.assertEqual(len(transport.calls), 1)

    def test_graphql_partial_mutation_data_is_uncertain_and_never_replayed(self):
        transport = FakeTransport([response(payload={"data": {"resolveReviewThread": {"thread": {"id": "T1"}}},
                                             "errors": [{"message": "partial result"}]})])
        with self.assertRaises(API.APIError) as caught:
            self.client(transport, "partial-mutation-token").graphql(
                "mutation Resolve { resolveReviewThread(input: {}) { thread { id } } }",
                operation="resolve_review_thread")
        self.assertTrue(caught.exception.uncertain)
        self.assertTrue(caught.exception.mutation_started)
        self.assertTrue(caught.exception.details["partial_data"])
        self.assertEqual(caught.exception.workflow_status, "uncertain")
        self.assertEqual(len(transport.calls), 1)

    def test_graphql_non_list_errors_envelope_fails_closed_for_query_and_mutation(self):
        malformed = {"data": {"change": {"id": "possibly-applied"}},
                     "errors": {"message": "partial"}}
        query_transport = FakeTransport([response(payload=malformed)])
        with self.assertRaises(API.APIError) as query_error:
            self.client(query_transport, "malformed-errors-query").graphql(
                "query Read { viewer { login } }", operation="read")
        self.assertEqual(query_error.exception.kind, "malformed_response")
        self.assertFalse(query_error.exception.mutation_started)
        self.assertFalse(query_error.exception.uncertain)
        self.assertEqual(len(query_transport.calls), 1)

        mutation_transport = FakeTransport([response(payload=malformed)])
        with self.assertRaises(API.APIError) as mutation_error:
            self.client(mutation_transport, "malformed-errors-mutation").graphql(
                "mutation Change { change { id } }", operation="change")
        self.assertEqual(mutation_error.exception.kind, "malformed_response")
        self.assertTrue(mutation_error.exception.mutation_started)
        self.assertTrue(mutation_error.exception.uncertain)
        self.assertEqual(mutation_error.exception.workflow_status, "uncertain")
        self.assertEqual(len(mutation_transport.calls), 1)

    def test_graphql_partial_errors_and_missing_data_fail_closed(self):
        for payload, expected in [
            ({"data": {"repository": None}, "errors": [{"message": "partial", "extensions": None}]}, "graphql_error"),
            ({"errors": [{"message": "no data"}]}, "graphql_error"),
            ({"data": None}, "malformed_response"),
        ]:
            transport = FakeTransport([response(payload=payload)])
            client = self.client(transport, f"bad-shape-{len(payload)}-{expected}")
            with self.assertRaises(API.APIError) as caught:
                client.graphql("query Strict { repository { nameWithOwner } }", operation="strict")
            self.assertEqual(caught.exception.kind, expected)

    def test_malformed_json_fails_closed(self):
        transport = FakeTransport([API.HTTPResponse(200, {}, b"not-json")])
        with self.assertRaises(API.APIError) as caught:
            self.client(transport).graphql("query Strict { viewer { login } }", operation="strict")
        self.assertEqual(caught.exception.kind, "malformed_response")

    def test_rate_limit_guard_shares_one_measured_budget_probe(self):
        transport = FakeTransport([response(payload={"data": {"rateLimit": {
            "cost": 1, "remaining": 480, "used": 20, "limit": 500,
            "resetAt": "2026-10-01T13:00:00Z",
        }}})])
        client = self.client(transport)
        first = client.rate_limit_guard(minimum_remaining=100, operation="broad_guard")
        second = client.rate_limit_guard(minimum_remaining=100, operation="broad_guard")
        self.assertEqual(first["status"], "ok")
        self.assertEqual(second["status"], "ok")
        self.assertEqual(len(transport.calls), 1)

    def test_rate_limit_guard_blocks_known_low_positive_budget(self):
        transport = FakeTransport([response(payload={"data": {"rateLimit": {
            "cost": 1, "remaining": 40, "used": 460, "limit": 500,
            "resetAt": "2026-10-01T13:00:00Z",
        }}})])
        result = self.client(transport).rate_limit_guard(minimum_remaining=100, operation="broad_guard")
        self.assertEqual(result["status"], "capability_blocked")
        self.assertEqual(result["reason"], "graphql_budget_insufficient")
        self.assertEqual(len(transport.calls), 1)

    def test_rate_limit_guard_reuses_fresh_zero_budget_without_network(self):
        transport = FakeTransport([])
        client = self.client(transport, "zero-budget-token")
        client.write_state("budget", client._rate_state_key, {
            "schema": "oasis7.github-api-budget/v1", "remaining": 0, "used": 500, "limit": 500,
            "cost": 1, "resetAt": None, "observed_at_epoch": self.clock.now,
            "observed_at": "now", "pause_until_epoch": 0,
        })
        API._PROCESS_PAUSES.clear()
        result = client.rate_limit_guard(minimum_remaining=100, max_age_seconds=300, operation="broad_guard")
        self.assertEqual(result["status"], "external_wait")
        self.assertEqual(result["reason"], "graphql_budget_exhausted")
        self.assertEqual(len(transport.calls), 0)

    def test_confirmed_same_viewer_shares_pause_but_not_credential_scope(self):
        token_a_transport = FakeTransport([
            response(payload={"data": {"viewer": {"id": "MDQ6VXNlcjEyMw=="},
                                        "rateLimit": {"cost": 1, "remaining": 450, "used": 50,
                                                      "limit": 500, "resetAt": "2030-01-01T00:00:00Z"}}}),
            response(403, {"message": "API rate limit exceeded"}),
        ])
        token_b_transport = FakeTransport([response(payload={
            "data": {"viewer": {"id": "MDQ6VXNlcjEyMw=="},
                     "rateLimit": {"cost": 1, "remaining": 449, "used": 51, "limit": 500,
                                   "resetAt": "2030-01-01T00:00:00Z"}}})])
        token_a = self.client(token_a_transport, "permission-a-token")
        token_b = self.client(token_b_transport, "permission-b-token")
        token_a.graphql("query Who { viewer { id } rateLimit { remaining } }", operation="identity")
        token_b.graphql("query Who { viewer { id } rateLimit { remaining } }", operation="identity")
        with self.assertRaises(API.APIError) as rate_limited:
            token_a.rest("GET", "repos/owner/repo", operation="trigger_limit")
        self.assertEqual(rate_limited.exception.workflow_status, "external_wait")
        self.assertEqual(len(token_a_transport.calls), 2)
        with self.assertRaises(API.APIError) as shared_pause:
            token_b.rest("GET", "repos/owner/repo", operation="blocked_by_shared_pause")
        self.assertEqual(shared_pause.exception.workflow_status, "external_wait")
        self.assertEqual(len(token_b_transport.calls), 1)
        self.assertNotEqual(token_a.credential_scope_digest, token_b.credential_scope_digest)

    def test_successful_zero_remaining_response_pauses_later_requests(self):
        transport = FakeTransport([response(payload={"data": {"viewer": {"login": "person"},
                                                               "rateLimit": {"cost": 1, "remaining": 0,
                                                                             "used": 500, "limit": 500,
                                                                             "resetAt": "2030-01-01T00:00:00Z"}}})])
        client = self.client(transport, "success-zero-token")
        client.graphql("query Read { viewer { login } rateLimit { remaining } }", operation="read")
        with self.assertRaises(API.APIError) as caught:
            client.rest("GET", "repos/owner/repo", operation="repo")
        self.assertEqual(caught.exception.workflow_status, "external_wait")
        self.assertEqual(len(transport.calls), 1)

    def test_env_token_precedence_and_explicit_constructor(self):
        with mock.patch.dict(os.environ, {"GH_TOKEN": "first-secret", "GITHUB_TOKEN": "second-secret"}):
            with mock.patch.object(API.subprocess, "run", side_effect=AssertionError("gh must not be called")):
                resolved = API.GitHubAPIClient.from_gh(state_root=self.state_root, transport=FakeTransport([]))
        self.assertEqual(resolved.token, "first-secret")
        explicit = API.GitHubAPIClient("explicit-secret", state_root=self.state_root, transport=FakeTransport([]))
        self.assertEqual(explicit.token, "explicit-secret")

    def test_telemetry_redacts_token_and_request_content_and_stats_count_sends(self):
        marker = "private-comment-body-value"
        token = "ghs_private_token_value"
        transport = FakeTransport([response(payload={"data": {"viewer": {"login": "person"}}})])
        client = self.client(transport, token)
        client.graphql("query Read($body: String!) { viewer { login } }", {"body": marker},
                       operation="read", context={"script": "test-script", "operation": "read",
                                                  "task_uid": "task_0123456789abcdef0123456789abcdef",
                                                  "pr_number": 123})
        client.log_cache_event("read", context={"script": "test-script", "operation": "read",
                                                "task_uid": "task_0123456789abcdef0123456789abcdef",
                                                "pr_number": 123}, cache_hit=True)
        lines = list(client.telemetry_dir.glob("*.jsonl"))
        payload = b"\n".join(path.read_bytes() for path in lines)
        self.assertNotIn(token.encode(), payload)
        self.assertNotIn(marker.encode(), payload)
        self.assertNotIn(b"query Read", payload)
        stats = API._stats(client, 3600)
        self.assertEqual(stats["coverage"], "instrumented_paths_only")
        self.assertEqual(stats["groups"][0]["requests"], 1)
        self.assertEqual(stats["groups"][0]["unknown_cost"], 1)
        self.assertEqual(stats["groups"][0]["cache_hits"], 1)

    def test_telemetry_preserves_sanitized_operation_correlation_context(self):
        context = {"script": "test-script", "operation": "budget-probe",
                   "operation_id": "op-123", "parent_operation_id": "op-parent",
                   "phase": "selected_task_audit", "api_family": "graphql",
                   "operation_name": "RateLimitBudget"}
        client = self.client(FakeTransport([response(payload={"data": {"rateLimit": {
            "cost": 1, "remaining": 450, "used": 50, "limit": 500,
            "resetAt": "2026-10-01T13:00:00Z"}}})]))
        client.graphql("query Budget { rateLimit { remaining } }", operation="budget-probe", context=context)
        lines = list(client.telemetry_dir.glob("*.jsonl"))
        event = json.loads(lines[0].read_text(encoding="utf-8").splitlines()[0])
        for key, value in context.items():
            if key in {"script", "operation"}:
                continue
            self.assertEqual(event[key], value)
        self.assertNotIn("query", event)
        self.assertNotIn("variables", event)

    def test_stats_and_status_do_not_send_http(self):
        transport = FakeTransport([])
        client = self.client(transport)
        self.assertEqual(API._stats(client, 3600)["groups"], [])
        self.assertEqual(API._status(client)["status"], "unknown")
        self.assertEqual(len(transport.calls), 0)

    def test_corrupt_shared_throttle_state_fails_closed(self):
        client = self.client(FakeTransport([]), "corrupt-state-token")
        path = client.state_path("budget", client._rate_state_key)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("{broken", encoding="utf-8")
        with self.assertRaises(API.APIError) as caught:
            client.graphql("query Read { viewer { login } }", operation="read")
        self.assertEqual(caught.exception.kind, "shared_state_unavailable")


if __name__ == "__main__":
    unittest.main(verbosity=2)
