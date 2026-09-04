#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("provider_bridge_contract_smoke.py")
LIVE_GATE_PATH = Path(__file__).with_name("provider-bridge-live-gate.sh")


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
                    "chain_resource_manifest_schema_version": "oasis7.world_resource_manifest.v1",
                    "chain_resource_delta_schema_version": "oasis7.world_resource_delta.v1",
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
        if self.path == "/v1/world-simulator/feedback-context":
            self.server.feedback_calls += 1
            self.write_json({"ok": True})
            return
        if self.path == "/v1/world-simulator/decision-context":
            self.server.target_decision_calls += 1
            response = {
                "decision": "wait",
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
            if self.server.empty_target_response:
                response = {}
            elif (
                self.server.quota_error_after is not None
                and self.server.target_decision_calls > self.server.quota_error_after
            ):
                response["provider_error"] = {
                    "code": self.server.quota_error_code,
                    "message": self.server.quota_error_message,
                    "retryable": False,
                }
            self.write_json(
                {
                    "base_decision_response": response,
                    "context_discriminator": body["context_discriminator"],
                    "context_version": body["context_version"],
                    "agent_session_id": body["agent_session_id"],
                    "agent_turn_id": body["agent_turn_id"],
                    "decision_request_id": body["decision_request_id"],
                    "retry_seq": body["retry_seq"],
                    "transport_attempt": body["transport_attempt"],
                    "request_digest": body["request_digest"],
                    "response_digest": "blake3:" + "b" * 64,
                }
            )
            return
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
        empty_target_response=False,
    ):
        self.health_ok = health_ok
        self.quota_error_after = quota_error_after
        self.quota_error_code = quota_error_code
        self.quota_error_message = quota_error_message
        self.empty_target_response = empty_target_response

    def __enter__(self):
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), MockProviderHandler)
        self.server.requests = []
        self.server.health_ok = self.health_ok
        self.server.quota_error_after = self.quota_error_after
        self.server.quota_error_code = self.quota_error_code
        self.server.quota_error_message = self.quota_error_message
        self.server.empty_target_response = self.empty_target_response
        self.server.decision_calls = 0
        self.server.target_decision_calls = 0
        self.server.feedback_calls = 0
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


def target_payload_file(directory, count=1):
    requests = []
    for index in range(1, count + 1):
        request = smoke.build_target_context_request(index, timeout_ms=5000)
        requests.append(
            {
                "decision_context": request,
                "feedback_context": {
                    "feedback_id": f"feedback.smoke-{index}",
                    "feedback_seq": 1,
                    "agent_subject": request["agent_subject"],
                    "agent_session_id": request["agent_session_id"],
                    "agent_turn_id": request["agent_turn_id"],
                    "decision_request_id": request["decision_request_id"],
                    "candidate_action_id": None,
                    "runtime_receipt_id": None,
                    "status": "pending",
                    "request_digest": request["request_digest"],
                    "reject_reason": None,
                    "provenance": "runtime_authoritative",
                },
            }
        )
    path = Path(directory) / "target-context.json"
    path.write_text(json.dumps({"requests": requests}), encoding="utf-8")
    return str(path)


def valid_target_response(request):
    return {
        "base_decision_response": {"decision": "wait", "provider_error": None},
        "context_discriminator": smoke.CONTINUOUS_CONTEXT_DISCRIMINATOR,
        "context_version": smoke.CONTINUOUS_CONTEXT_VERSION,
        "agent_session_id": request["agent_session_id"],
        "agent_turn_id": request["agent_turn_id"],
        "decision_request_id": request["decision_request_id"],
        "retry_seq": request["retry_seq"],
        "transport_attempt": request["transport_attempt"],
        "request_digest": request["request_digest"],
        "response_digest": "blake3:" + "b" * 64,
    }


def valid_target_feedback(request):
    return {
        "feedback_id": "feedback.smoke-1",
        "feedback_seq": 1,
        "agent_subject": request["agent_subject"],
        "agent_session_id": request["agent_session_id"],
        "agent_turn_id": request["agent_turn_id"],
        "decision_request_id": request["decision_request_id"],
        "status": "pending",
        "request_digest": request["request_digest"],
        "provenance": "runtime_authoritative",
    }


class ProviderBridgeContractSmokeTests(unittest.TestCase):
    def test_public_ingress_contract_allows_degraded_health_and_decision_success(self):
        with tempfile.TemporaryDirectory() as directory:
            payload_path = target_payload_file(directory)
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
                        target_context_payload_file=payload_path,
                    )
                )

        self.assertEqual(summary["status"], "pass")
        self.assertEqual(summary["lane"], "target_cognition")
        self.assertEqual(summary["feedback_successes"], 1)
        self.assertEqual(summary["health_status"], "degraded")
        self.assertEqual(
            summary["chain_resource_manifest_schema_version"],
            "oasis7.world_resource_manifest.v1",
        )
        self.assertEqual(
            summary["chain_resource_delta_schema_version"],
            "oasis7.world_resource_delta.v1",
        )
        self.assertEqual(summary["decision_successes"], 1)
        self.assertEqual(
            [request["path"] for request in server.requests if request["method"] == "POST"],
            ["/v1/world-simulator/decision-context", "/v1/world-simulator/feedback-context"],
        )
        self.assertEqual(summary["decisions"][0]["provider_error_code"], None)
        self.assertTrue(
            all(
                request["authorization"] == "Bearer newapi_user_ref:smoke-user"
                for request in server.requests
            )
        )

    def test_low_quota_exhaustion_requires_matching_provider_error(self):
        with tempfile.TemporaryDirectory() as directory:
            payload_path = target_payload_file(directory, count=3)
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
                        target_context_payload_file=payload_path,
                    )
                )

        self.assertEqual(summary["decision_successes"], 2)
        self.assertEqual(summary["matching_provider_errors"], 1)
        self.assertEqual(summary["decisions"][2]["provider_error_code"], "quota_exhausted")

    def test_low_quota_exhaustion_matches_provider_error_message(self):
        with tempfile.TemporaryDirectory() as directory:
            payload_path = target_payload_file(directory, count=2)
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
                        target_context_payload_file=payload_path,
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

    def test_target_lane_fails_closed_without_runtime_pair_artifact(self):
        with MockProviderServer(health_ok=True) as server:
            with self.assertRaisesRegex(RuntimeError, "target cognition smoke requires"):
                smoke.run_smoke(
                    smoke.SmokeOptions(
                        base_url=server.base_url,
                        auth_token="newapi_user_ref:missing-pair",
                        timeout_ms=5000,
                        decision_count=1,
                        min_successes=1,
                        expect_provider_error_code_substr="",
                        require_health_ok=True,
                    )
                )

    def test_target_lane_rejects_empty_inner_decision_response(self):
        with tempfile.TemporaryDirectory() as directory:
            payload_path = target_payload_file(directory)
            with MockProviderServer(health_ok=True, empty_target_response=True) as server:
                with self.assertRaisesRegex(
                    RuntimeError, "target provider response base decision"
                ):
                    smoke.run_smoke(
                        smoke.SmokeOptions(
                            base_url=server.base_url,
                            auth_token="newapi_user_ref:empty-inner",
                            timeout_ms=5000,
                            decision_count=1,
                            min_successes=1,
                            expect_provider_error_code_substr="",
                            require_health_ok=True,
                            target_context_payload_file=payload_path,
                        )
                    )

    def test_shared_target_response_validator_rejects_untagged_response(self):
        with self.assertRaisesRegex(RuntimeError, "supported decision tag"):
            smoke.validate_base_decision_response({"provider_error": None})

    def test_shared_target_response_validator_rejects_malformed_response(self):
        with self.assertRaisesRegex(RuntimeError, "requires code"):
            smoke.validate_base_decision_response(
                {"provider_error": {"message": "malformed"}}
            )

    def test_shared_target_response_validator_rejects_missing_wrapper_digest(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        response = valid_target_response(request)
        response.pop("response_digest")
        with self.assertRaisesRegex(RuntimeError, "response_digest"):
            smoke.validate_target_context_response(response, request)

    def test_shared_target_response_validator_rejects_wrapper_identity_drift(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        response = valid_target_response(request)
        response["agent_turn_id"] = "wrong-turn"
        with self.assertRaisesRegex(RuntimeError, "agent_turn_id does not echo"):
            smoke.validate_target_context_response(response, request)

    def test_shared_target_response_validator_rejects_malformed_wrapper(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        response = valid_target_response(request)
        response["base_decision_response"] = []
        with self.assertRaisesRegex(RuntimeError, "base must be a JSON object"):
            smoke.validate_target_context_response(response, request)

    def test_target_outer_validators_reject_unknown_authority_fields_with_stable_code(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        request["unknown_authority_field"] = True
        with self.assertRaisesRegex(RuntimeError, r"^unknown_context_field:"):
            smoke.validate_target_context_request(request)

        response = valid_target_response(smoke.build_target_context_request(index=1, timeout_ms=5000))
        response["unknown_authority_field"] = True
        with self.assertRaisesRegex(RuntimeError, r"^unknown_context_field:"):
            smoke.validate_target_context_response(response, request={})

        feedback = valid_target_feedback(smoke.build_target_context_request(index=1, timeout_ms=5000))
        feedback["unknown_authority_field"] = True
        with self.assertRaisesRegex(RuntimeError, r"^unknown_context_field:"):
            smoke.validate_target_context_feedback(feedback, request)

    def test_target_feedback_rejects_unverifiable_authority_and_invalid_reasons(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        for mutate in (
            lambda feedback: feedback.update(
                status="committed", candidate_action_id=7, runtime_receipt_id="receipt-forged"
            ),
            lambda feedback: feedback.update(candidate_action_id=7),
            lambda feedback: feedback.update(runtime_receipt_id="receipt-forged"),
        ):
            feedback = valid_target_feedback(request)
            mutate(feedback)
            with self.assertRaisesRegex(RuntimeError, "feedback_disposition_unverifiable"):
                smoke.validate_target_context_feedback(feedback, request)

        for status in ("rejected", "failed"):
            feedback = valid_target_feedback(request)
            feedback.update(status=status, reject_reason="forged_reason")
            with self.assertRaisesRegex(RuntimeError, "feedback_disposition_reason_invalid"):
                smoke.validate_target_context_feedback(feedback, request)

    def test_target_request_requires_strict_positive_integer_lineage(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        for field in ("retry_seq", "transport_attempt"):
            for invalid in (True, False, 0, -1, "1", None):
                with self.subTest(field=field, invalid=invalid):
                    malformed = dict(request)
                    malformed[field] = invalid
                    with self.assertRaisesRegex(RuntimeError, "positive integer"):
                        smoke.validate_target_context_request(malformed)

    def test_target_feedback_requires_strict_positive_integer_sequence(self):
        request = smoke.build_target_context_request(index=1, timeout_ms=5000)
        for invalid in (True, False, 0, -1, "1", None):
            with self.subTest(invalid=invalid):
                feedback = valid_target_feedback(request)
                feedback["feedback_seq"] = invalid
                with self.assertRaisesRegex(RuntimeError, "positive integer"):
                    smoke.validate_target_context_feedback(feedback, request)

    def test_live_gate_uses_complete_shared_target_response_validator(self):
        live_gate = LIVE_GATE_PATH.read_text(encoding="utf-8")
        self.assertIn("response_validator_source", live_gate)
        self.assertIn(
            "base = validate_target_context_response(response, request)",
            live_gate,
        )

    def test_target_lane_rejects_feedback_identity_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            request = smoke.build_target_context_request(index=1, timeout_ms=5000)
            feedback = {
                "feedback_id": "feedback.smoke-1",
                "feedback_seq": 1,
                "agent_subject": request["agent_subject"],
                "agent_session_id": request["agent_session_id"],
                "agent_turn_id": request["agent_turn_id"],
                "decision_request_id": "wrong-request",
                "status": "pending",
                "request_digest": request["request_digest"],
                "provenance": "runtime_authoritative",
            }
            path = Path(directory) / "target-context.json"
            path.write_text(
                json.dumps(
                    {"requests": [{"decision_context": request, "feedback_context": feedback}]}
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(RuntimeError, "feedback decision_request_id"):
                smoke.load_target_context_pairs(str(path), 1)

    def test_low_quota_exhaustion_fails_without_provider_error(self):
        with tempfile.TemporaryDirectory() as directory:
            payload_path = target_payload_file(directory, count=3)
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
                            target_context_payload_file=payload_path,
                        )
                    )

    def test_bare_route_is_explicit_legacy_only(self):
        with MockProviderServer(health_ok=True) as server:
            summary = smoke.run_smoke(
                smoke.SmokeOptions(
                    base_url=server.base_url,
                    auth_token="newapi_user_ref:legacy",
                    timeout_ms=5000,
                    decision_count=1,
                    min_successes=1,
                    expect_provider_error_code_substr="",
                    require_health_ok=True,
                    legacy_compatibility_only=True,
                )
            )
        self.assertEqual(summary["lane"], "legacy_compatibility_only")
        self.assertEqual(summary["feedback_successes"], 0)
        self.assertEqual(summary["decision_path"], "/v1/world-simulator/decision")
        self.assertEqual(
            [request["path"] for request in server.requests if request["method"] == "POST"],
            ["/v1/world-simulator/decision"],
        )


if __name__ == "__main__":
    unittest.main()
