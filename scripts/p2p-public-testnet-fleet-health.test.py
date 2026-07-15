#!/usr/bin/env python3
"""RED contract for the cross-platform public-testnet fleet-health collector."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parent.parent
COLLECTOR = ROOT_DIR / "scripts" / "p2p-public-testnet-fleet-health.py"


def status(*, head: int = 42, ready: bool = True, decision: str = "ready") -> dict[str, Any]:
    return {
        "running": True,
        "last_error": None,
        "readiness": {"status": "ready" if ready else "not_ready", "failed_gates": []},
        "consensus": {
            "committed_height": head,
            "network_committed_height": head,
            "last_execution_height": head,
            "network_head": {"decision": decision},
        },
    }


class FleetHealthFixture:
    def __init__(self, responses: dict[str, tuple[dict[str, Any], float]]) -> None:
        self.responses = responses
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), self._handler())
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def _handler(self) -> type[BaseHTTPRequestHandler]:
        responses = self.responses

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self) -> None:  # noqa: N802
                payload, delay = responses.get(self.path, ({"error": "missing fixture"}, 0.0))
                if delay:
                    time.sleep(delay)
                encoded = json.dumps(payload).encode("utf-8")
                self.send_response(200 if self.path in responses else 404)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(encoded)))
                self.end_headers()
                self.wfile.write(encoded)

            def log_message(self, _format: str, *_args: object) -> None:
                pass

        return Handler

    def __enter__(self) -> "FleetHealthFixture":
        self.thread.start()
        return self

    def __exit__(self, *_args: object) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def endpoint(self, name: str) -> str:
        host, port = self.server.server_address
        return f"http://{host}:{port}/{name}"


class FleetHealthCollectorContractTest(unittest.TestCase):
    def run_collector(
        self,
        fixture: FleetHealthFixture,
        output: Path,
        *,
        max_span_seconds: float = 1.0,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(COLLECTOR),
                "--sequencer",
                "sequencer",
                "--node",
                f"sequencer={fixture.endpoint('sequencer')}",
                "--node",
                f"storage={fixture.endpoint('storage')}",
                "--node",
                f"observer={fixture.endpoint('observer')}",
                "--max-capture-span-seconds",
                str(max_span_seconds),
                "--output",
                str(output),
            ],
            cwd=ROOT_DIR,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_ready_fleet_writes_timestamped_json_evidence(self) -> None:
        responses = {f"/{name}": (status(), 0.0) for name in ("sequencer", "storage", "observer")}
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            result = self.run_collector(fixture, output)

            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "ready")
            self.assertEqual(evidence["max_capture_span_seconds"], 1.0)
            self.assertIn("capture_started_at", evidence)
            self.assertIn("capture_finished_at", evidence)
            self.assertEqual(evidence["sequencer"], "sequencer")
            self.assertEqual(set(evidence["nodes"]), {"sequencer", "storage", "observer"})
            for node in evidence["nodes"].values():
                self.assertIn("captured_at", node)
                self.assertEqual(node["consensus"]["committed_height"], 42)

    def test_over_span_fails_closed_and_writes_failure_evidence(self) -> None:
        responses = {
            "/sequencer": (status(), 0.0),
            "/storage": (status(), 0.08),
            "/observer": (status(), 0.0),
        }
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            result = self.run_collector(fixture, output, max_span_seconds=0.01)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("capture_span_exceeded", result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(evidence["verdict"], "blocked")
            self.assertIn("capture_span_exceeded", evidence["failed_gates"])

    def test_any_head_projection_divergence_fails_closed(self) -> None:
        for field in (
            "committed_height",
            "network_committed_height",
            "last_execution_height",
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                divergent = status(head=42)
                divergent["consensus"][field] = 41
                responses = {
                    "/sequencer": (status(head=42), 0.0),
                    "/storage": (divergent, 0.0),
                    "/observer": (status(head=42), 0.0),
                }
                with FleetHealthFixture(responses) as fixture:
                    output = Path(temp_dir) / "fleet-health.json"
                    result = self.run_collector(fixture, output)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn("head_mismatch", result.stderr)
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertIn("head_mismatch", evidence["failed_gates"])

    def test_non_ready_network_head_fails_closed(self) -> None:
        responses = {
            "/sequencer": (status(), 0.0),
            "/storage": (status(decision="degraded"), 0.0),
            "/observer": (status(), 0.0),
        }
        with tempfile.TemporaryDirectory() as temp_dir, FleetHealthFixture(responses) as fixture:
            output = Path(temp_dir) / "fleet-health.json"
            result = self.run_collector(fixture, output)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("network_head_not_ready", result.stderr)
            evidence = json.loads(output.read_text(encoding="utf-8"))
            self.assertIn("network_head_not_ready", evidence["failed_gates"])

    def test_node_readiness_error_and_failed_gates_each_fail_closed(self) -> None:
        invalid_statuses = {
            "node_not_ready": {"readiness": {"status": "not_ready", "failed_gates": []}},
            "failed_gates_nonempty": {"readiness": {"status": "ready", "failed_gates": ["stale"]}},
            "last_error_present": {"last_error": "replication stalled"},
        }
        for expected_gate, replacement in invalid_statuses.items():
            with self.subTest(gate=expected_gate), tempfile.TemporaryDirectory() as temp_dir:
                invalid = status()
                invalid.update(replacement)
                responses = {
                    "/sequencer": (status(), 0.0),
                    "/storage": (invalid, 0.0),
                    "/observer": (status(), 0.0),
                }
                with FleetHealthFixture(responses) as fixture:
                    output = Path(temp_dir) / "fleet-health.json"
                    result = self.run_collector(fixture, output)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_gate, result.stderr)
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertIn(expected_gate, evidence["failed_gates"])


if __name__ == "__main__":
    unittest.main()
