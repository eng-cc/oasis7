#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import sys
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("provider_bridge_contract_smoke.py")


def load_smoke_module():
    spec = importlib.util.spec_from_file_location("provider_bridge_contract_smoke", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


smoke = load_smoke_module()


class MockProviderHandler(BaseHTTPRequestHandler):
    server_version = "MockProviderBridge/1.0"

    def do_GET(self):
        self.server.requests.append(
            {
                "method": "GET",
                "path": self.path,
                "authorization": self.headers.get("Authorization"),
            }
        )
        if self.path == "/v1/provider/info":
            self.write_json(
                {
                    "provider_id": "provider_local_bridge",
                    "name": "Mock Provider Bridge",
                    "version": "0.1.0",
                    "protocol_version": "world-simulator-provider-loopback-http-v1",
                    "capabilities": ["decision", "feedback"],
                    "supported_action_sets": ["wait", "move_agent"],
                }
            )
            return
        if self.path == "/v1/provider/health":
            self.write_json(
                {
                    "ok": self.server.health_ok,
                    "status": "ok" if self.server.health_ok else "degraded",
                    "last_error": None if self.server.health_ok else "gateway health returned HTTP 401",
                }
            )
            return
        self.send_error(404)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = json.loads(self.rfile.read(length).decode("utf-8"))
        self.server.requests.append(
            {
                "method": "POST",
                "path": self.path,
                "authorization": self.headers.get("Authorization"),
                "body": body,
            }
        )
        if self.path != "/v1/world-simulator/decision":
            self.send_error(404)
            return
        self.server.decision_calls += 1
        if (
            self.server.quota_error_after is not None
            and self.server.decision_calls > self.server.quota_error_after
        ):
            self.write_json(
                {
                    "decision": {"decision": "wait"},
                    "provider_error": {
                        "code": self.server.quota_error_code,
                        "message": self.server.quota_error_message,
                        "retryable": False,
                    },
                    "diagnostics": {
                        "provider_id": "provider_local_bridge",
                        "provider_version": "mock-model",
                        "latency_ms": 2,
                    },
                    "trace_payload": {
                        "provider_id": "provider_local_bridge",
                        "provider_version": "mock-model",
                        "schema_repair_count": 0,
                    },
                }
            )
            return
        self.write_json(
            {
                "decision": {"decision": "wait"},
                "provider_error": None,
                "diagnostics": {
                    "provider_id": "provider_local_bridge",
                    "provider_version": "mock-model",
                    "latency_ms": 1,
                },
                "trace_payload": {
                    "provider_id": "provider_local_bridge",
                    "provider_version": "mock-model",
                    "schema_repair_count": 0,
                },
            }
        )

    def write_json(self, payload):
        encoded = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def log_message(self, format, *args):
        return


class MockProviderServer:
    def __init__(
        self,
        *,
        health_ok=False,
        quota_error_after=None,
        quota_error_code="quota_exhausted",
        quota_error_message="mock quota exhausted",
    ):
        self.health_ok = health_ok
        self.quota_error_after = quota_error_after
        self.quota_error_code = quota_error_code
        self.quota_error_message = quota_error_message

    def __enter__(self):
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), MockProviderHandler)
        self.server.requests = []
        self.server.health_ok = self.health_ok
        self.server.quota_error_after = self.quota_error_after
        self.server.quota_error_code = self.quota_error_code
        self.server.quota_error_message = self.quota_error_message
        self.server.decision_calls = 0
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        host, port = self.server.server_address
        self.base_url = f"http://{host}:{port}"
        return self

    def __exit__(self, exc_type, exc, tb):
        self.server.shutdown()
        self.thread.join(timeout=5)
        self.server.server_close()

    @property
    def requests(self):
        return list(self.server.requests)


class ProviderBridgeContractSmokeTests(unittest.TestCase):
    def test_public_ingress_contract_allows_degraded_health_and_decision_success(self):
        with MockProviderServer(health_ok=False) as server:
            summary = smoke.run_smoke(
                smoke.SmokeOptions(
                    base_url=server.base_url,
                    auth_token="newapi_user_ref:smoke-user",
                    timeout_ms=5000,
                    decision_count=1,
                    min_successes=1,
                    expect_provider_error_code_substr="",
                    require_health_ok=False,
                )
            )

        self.assertEqual(summary["status"], "pass")
        self.assertEqual(summary["health_status"], "degraded")
        self.assertEqual(summary["decision_successes"], 1)
        self.assertEqual(summary["decisions"][0]["provider_error_code"], None)
        self.assertTrue(
            all(
                request["authorization"] == "Bearer newapi_user_ref:smoke-user"
                for request in server.requests
            )
        )

    def test_low_quota_exhaustion_requires_matching_provider_error(self):
        with MockProviderServer(health_ok=True, quota_error_after=2) as server:
            summary = smoke.run_smoke(
                smoke.SmokeOptions(
                    base_url=server.base_url,
                    auth_token="newapi_user_ref:lowquota",
                    timeout_ms=5000,
                    decision_count=3,
                    min_successes=2,
                    expect_provider_error_code_substr="quota",
                    require_health_ok=True,
                )
            )

        self.assertEqual(summary["decision_successes"], 2)
        self.assertEqual(summary["matching_provider_errors"], 1)
        self.assertEqual(summary["decisions"][2]["provider_error_code"], "quota_exhausted")

    def test_low_quota_exhaustion_matches_provider_error_message(self):
        with MockProviderServer(
            health_ok=True,
            quota_error_after=1,
            quota_error_code="provider_gateway_unreachable",
            quota_error_message="upstream returned insufficient_user_quota",
        ) as server:
            summary = smoke.run_smoke(
                smoke.SmokeOptions(
                    base_url=server.base_url,
                    auth_token="newapi_user_ref:lowquota",
                    timeout_ms=5000,
                    decision_count=2,
                    min_successes=1,
                    expect_provider_error_code_substr="quota",
                    require_health_ok=True,
                )
            )

        self.assertEqual(summary["decision_successes"], 1)
        self.assertEqual(summary["matching_provider_errors"], 1)
        self.assertEqual(
            summary["decisions"][1]["provider_error_code"],
            "provider_gateway_unreachable",
        )
        self.assertIn("insufficient_user_quota", summary["decisions"][1]["provider_error_message"])

    def test_decision_payload_uses_configured_timeout_budget(self):
        request = smoke.build_decision_request(index=7, timeout_ms=4321)

        self.assertEqual(request["timeout_budget_ms"], 4321)
        self.assertEqual(request["observation"]["timeout_budget_ms"], 4321)

    def test_low_quota_exhaustion_fails_without_provider_error(self):
        with MockProviderServer(health_ok=True) as server:
            with self.assertRaisesRegex(RuntimeError, "expected provider_error code or message"):
                smoke.run_smoke(
                    smoke.SmokeOptions(
                        base_url=server.base_url,
                        auth_token="newapi_user_ref:lowquota",
                        timeout_ms=5000,
                        decision_count=3,
                        min_successes=2,
                        expect_provider_error_code_substr="quota",
                        require_health_ok=True,
                    )
                )


if __name__ == "__main__":
    unittest.main()
