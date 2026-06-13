#!/usr/bin/env python3

import importlib.util
import io
import json
import os
import tempfile
import threading
import unittest
import urllib.error
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("letai_provider_cli.py")


def load_cli_module():
    spec = importlib.util.spec_from_file_location("letai_provider_cli", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


cli = load_cli_module()


class EnvPatch:
    def __init__(self, **updates):
        self.updates = updates
        self.previous = {}

    def __enter__(self):
        for key, value in self.updates.items():
            self.previous[key] = os.environ.get(key)
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value

    def __exit__(self, exc_type, exc, tb):
        for key, value in self.previous.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


class MockLetaiHandler(BaseHTTPRequestHandler):
    server_version = "MockLetai/1.0"

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        self.server.requests.append(
            {
                "path": self.path,
                "authorization": self.headers.get("Authorization"),
                "body": json.loads(body.decode("utf-8")),
            }
        )
        payload = {
            "model": "mock-model",
            "choices": [{"message": {"content": "{\"decision\":\"wait\"}"}}],
            "usage": {
                "prompt_tokens": 3,
                "completion_tokens": 2,
                "total_tokens": 5,
            },
        }
        encoded = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def log_message(self, format, *args):
        return


class MockLetaiServer:
    def __enter__(self):
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), MockLetaiHandler)
        self.server.requests = []
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        host, port = self.server.server_address
        self.base_url = f"http://{host}:{port}/v1"
        return self

    def __exit__(self, exc_type, exc, tb):
        self.server.shutdown()
        self.thread.join(timeout=5)
        self.server.server_close()

    @property
    def requests(self):
        return list(self.server.requests)


def write_state(payload):
    handle = tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False)
    with handle:
        json.dump(payload, handle)
    return handle.name


class LetaiProviderCliNewapiStateTests(unittest.TestCase):
    def tearDown(self):
        for key in [
            "OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH",
            "OASIS7_REMOTE_LLM_ROUTE_LABEL",
            "OASIS7_REMOTE_LLM_BASE_URL",
            "OASIS7_REMOTE_LLM_MODEL",
            "OASIS7_REMOTE_LLM_API_KEY",
            "OASIS7_REMOTE_LLM_ROUTES_PATH",
            "OASIS7_REMOTE_LLM_STREAM",
        ]:
            os.environ.pop(key, None)

    def test_newapi_state_selector_drives_chat_completion_with_project_token(self):
        state_path = write_state(
            {
                "bindings": [
                    {
                        "bridge_user_id": "bridge-user-000001",
                        "newapi_user_ref": "user-1",
                        "status": "active",
                    }
                ],
                "project_bindings": [
                    {
                        "bridge_user_id": "bridge-user-000001",
                        "token_key": "token-key-000001",
                    }
                ],
            }
        )
        try:
            with MockLetaiServer() as server, EnvPatch(
                OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH=state_path,
                OASIS7_REMOTE_LLM_ROUTE_LABEL="newapi_user_ref:user-1",
                OASIS7_REMOTE_LLM_BASE_URL=server.base_url,
                OASIS7_REMOTE_LLM_MODEL="mock-model",
            ):
                result = cli.request_completion("choose next action", 5000, "agent-1")

            self.assertEqual(result["payloads"][0]["text"], "{\"decision\":\"wait\"}")
            self.assertEqual(len(server.requests), 1)
            request = server.requests[0]
            self.assertEqual(request["path"], "/v1/chat/completions")
            self.assertEqual(request["authorization"], "Bearer token-key-000001")
            self.assertEqual(request["body"]["model"], "mock-model")
            self.assertEqual(request["body"]["user"], "oasis7-provider:agent-1")
        finally:
            os.unlink(state_path)

    def test_newapi_state_selector_accepts_bridge_user_id(self):
        state_path = write_state(
            {
                "bindings": [
                    {
                        "bridge_user_id": "bridge-user-000042",
                        "newapi_user_ref": "user-42",
                        "status": "active",
                    }
                ],
                "project_bindings": [
                    {
                        "bridge_user_id": "bridge-user-000042",
                        "token_key": "token-key-000042",
                    }
                ],
            }
        )
        try:
            with EnvPatch(OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH=state_path):
                route = cli.load_newapi_bridge_state_route("bridge_user_id:bridge-user-000042")
            self.assertEqual(route, {"api_key": "token-key-000042"})
        finally:
            os.unlink(state_path)

    def test_newapi_state_selector_rejects_inactive_or_tokenless_binding(self):
        state_path = write_state(
            {
                "bindings": [
                    {
                        "bridge_user_id": "bridge-user-000001",
                        "newapi_user_ref": "user-1",
                        "status": "disabled",
                    },
                    {
                        "bridge_user_id": "bridge-user-000002",
                        "newapi_user_ref": "user-2",
                        "status": "active",
                    },
                ],
                "project_bindings": [
                    {
                        "bridge_user_id": "bridge-user-000002",
                        "token_key": "",
                    }
                ],
            }
        )
        try:
            with EnvPatch(OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH=state_path):
                with self.assertRaisesRegex(RuntimeError, "no active newapi bridge binding"):
                    cli.load_newapi_bridge_state_route("newapi_user_ref:user-1")
                with self.assertRaisesRegex(RuntimeError, "does not have a usable token_key"):
                    cli.load_newapi_bridge_state_route("newapi_user_ref:user-2")
        finally:
            os.unlink(state_path)

    def test_topup_retry_waits_for_quota_visibility_window(self):
        calls = []

        def fake_send_completion_request(request, timeout_ms, use_stream):
            calls.append(request.full_url)
            if len(calls) < 3:
                raise urllib.error.HTTPError(
                    request.full_url,
                    403,
                    "Forbidden",
                    {},
                    io.BytesIO(b'{"error":{"code":"insufficient_user_quota"}}'),
                )
            return (
                200,
                {"choices": [{"message": {"content": "{\"decision\":\"wait\"}"}}]},
                "{\"decision\":\"wait\"}",
                {"total_tokens": 5},
            )

        previous_send = cli.send_completion_request
        previous_sleep = cli.time.sleep
        cli.send_completion_request = fake_send_completion_request
        cli.time.sleep = lambda _seconds: None
        try:
            with EnvPatch(
                OASIS7_REMOTE_LLM_AUTO_TOPUP_RETRY_COUNT="3",
                OASIS7_REMOTE_LLM_AUTO_TOPUP_RETRY_DELAY_MS="25",
            ):
                status_code, _decoded, content, usage = cli.send_completion_request_after_topup(
                    {"model": "mock-model", "messages": []},
                    "https://api.example.test/v1",
                    "token-key",
                    5000,
                    False,
                )
        finally:
            cli.send_completion_request = previous_send
            cli.time.sleep = previous_sleep

        self.assertEqual(status_code, 200)
        self.assertEqual(content, "{\"decision\":\"wait\"}")
        self.assertEqual(usage["total_tokens"], 5)
        self.assertEqual(len(calls), 3)

    def test_completion_retry_recovers_from_empty_sse_content(self):
        calls = []

        def fake_send_completion_request(request, timeout_ms, use_stream):
            calls.append(request.full_url)
            if len(calls) == 1:
                raise RuntimeError("upstream SSE response did not contain assistant content")
            return (
                200,
                {"choices": [{"message": {"content": "{\"decision\":\"wait\"}"}}]},
                "{\"decision\":\"wait\"}",
                {"total_tokens": 5},
            )

        previous_send = cli.send_completion_request
        previous_sleep = cli.time.sleep
        cli.send_completion_request = fake_send_completion_request
        cli.time.sleep = lambda _seconds: None
        try:
            with EnvPatch(
                OASIS7_REMOTE_LLM_RETRY_COUNT="2",
                OASIS7_REMOTE_LLM_RETRY_DELAY_MS="25",
            ):
                status_code, _decoded, content, usage = cli.send_completion_request_with_retries(
                    {"model": "mock-model", "messages": []},
                    "https://api.example.test/v1",
                    "token-key",
                    5000,
                    True,
                )
        finally:
            cli.send_completion_request = previous_send
            cli.time.sleep = previous_sleep

        self.assertEqual(status_code, 200)
        self.assertEqual(content, "{\"decision\":\"wait\"}")
        self.assertEqual(usage["total_tokens"], 5)
        self.assertEqual(len(calls), 2)

    def test_completion_retry_classifies_remote_close_as_retryable(self):
        self.assertTrue(
            cli.is_retryable_completion_error(
                RuntimeError("Remote end closed connection without response")
            )
        )

    def test_gateway_call_timeout_is_already_milliseconds(self):
        params = json.dumps({"message": "choose", "agentId": "agent-1"})

        prompt, timeout_ms, agent_id = cli.parse_gateway_call(
            [
                "gateway",
                "call",
                "agent",
                "--params",
                params,
                "--timeout",
                "17000",
                "--expect-final",
                "--json",
            ]
        )

        self.assertEqual(prompt, "choose")
        self.assertEqual(timeout_ms, 17000)
        self.assertEqual(agent_id, "agent-1")

    def test_local_agent_accepts_bridge_compatibility_flags(self):
        prompt, timeout_ms, agent_id = cli.parse_local_agent(
            [
                "agent",
                "--agent",
                "letai-local",
                "--message",
                "choose",
                "--local",
                "--session-id",
                "ws-abc123",
                "--thinking",
                "off",
                "--timeout",
                "17000",
                "--json",
            ]
        )

        self.assertEqual(prompt, "choose")
        self.assertEqual(timeout_ms, 17000)
        self.assertEqual(agent_id, "letai-local")


if __name__ == "__main__":
    unittest.main()
