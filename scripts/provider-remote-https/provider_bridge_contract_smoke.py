#!/usr/bin/env python3

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import sys
import time
from typing import Optional, Tuple
import urllib.error
import urllib.request


DEFAULT_TIMEOUT_MS = 15000


@dataclass(frozen=True)
class SmokeOptions:
    base_url: str
    auth_token: str
    timeout_ms: int
    decision_count: int
    min_successes: int
    expect_provider_error_code_substr: str
    require_health_ok: bool


def normalize_base_url(raw: str) -> str:
    value = raw.strip().rstrip("/")
    if not value:
        raise RuntimeError("--base-url is required")
    return value


def make_headers(auth_token: str, content_type: bool = False) -> dict[str, str]:
    headers = {
        "User-Agent": "oasis7-provider-bridge-contract-smoke/1.0",
    }
    if auth_token:
        headers["Authorization"] = f"Bearer {auth_token}"
    if content_type:
        headers["Content-Type"] = "application/json"
    return headers


def request_json(
    base_url: str,
    path: str,
    *,
    method: str = "GET",
    auth_token: str = "",
    payload: Optional[dict] = None,
    timeout_ms: int = DEFAULT_TIMEOUT_MS,
) -> Tuple[int, dict, float]:
    data = None
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        f"{base_url}{path}",
        data=data,
        headers=make_headers(auth_token, content_type=payload is not None),
        method=method,
    )
    started = time.time()
    try:
        with urllib.request.urlopen(request, timeout=max(timeout_ms, 1000) / 1000.0) as response:
            body = response.read().decode("utf-8")
            status = response.status
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path} returned HTTP {exc.code}: {body}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"{method} {path} failed: {exc}") from exc
    elapsed = time.time() - started
    try:
        decoded = json.loads(body)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"{method} {path} returned non-JSON body: {body[:200]}") from exc
    if not isinstance(decoded, dict):
        raise RuntimeError(f"{method} {path} returned non-object JSON")
    return status, decoded, elapsed


def build_decision_request(index: int, timeout_ms: int) -> dict:
    return {
        "observation": {
            "agent_id": f"smoke-agent-{index}",
            "world_time": index,
            "mode": "headless_agent",
            "observation_schema_version": "oc_dual_obs_v1",
            "action_schema_version": "oc_dual_act_v1",
            "environment_class": "provider_bridge_contract_smoke",
            "observation": {
                "self_state": {
                    "location_ref": "loc-1",
                    "pose_hint": "grid_pose=(0, 0, 0)",
                    "status_flags": [],
                    "resource_summary": {},
                },
                "mission_context": {
                    "goal_summary": "return a minimal wait decision for provider bridge smoke"
                },
                "nearby_entities": [],
                "recent_events": [],
                "local_navigation_graph": [],
                "hazard_summary": [],
                "interaction_targets": [],
            },
            "recent_event_summary": [],
            "action_catalog": [
                {"action_ref": "wait", "summary": "do nothing this tick"},
                {"action_ref": "move_agent", "summary": "move to a visible location"},
            ],
            "timeout_budget_ms": timeout_ms,
        },
        "provider_config_ref": "provider://remote-https",
        "agent_profile": "oasis7_p0_low_freq_npc",
        "fixture_id": "provider_bridge_contract_smoke",
        "timeout_budget_ms": timeout_ms,
    }


def provider_error_code(response: dict) -> str:
    provider_error = response.get("provider_error")
    if not isinstance(provider_error, dict):
        return ""
    return str(provider_error.get("code") or "").strip()


def provider_error_message(response: dict) -> str:
    provider_error = response.get("provider_error")
    if not isinstance(provider_error, dict):
        return ""
    return str(provider_error.get("message") or "").strip()


def provider_version(response: dict) -> str:
    diagnostics = response.get("diagnostics")
    if isinstance(diagnostics, dict):
        value = str(diagnostics.get("provider_version") or "").strip()
        if value:
            return value
    trace = response.get("trace_payload")
    if isinstance(trace, dict):
        return str(trace.get("provider_version") or "").strip()
    return ""


def run_smoke(options: SmokeOptions) -> dict:
    base_url = normalize_base_url(options.base_url)
    info_status, info, info_elapsed = request_json(
        base_url,
        "/v1/provider/info",
        auth_token=options.auth_token,
        timeout_ms=options.timeout_ms,
    )
    health_status, health, health_elapsed = request_json(
        base_url,
        "/v1/provider/health",
        auth_token=options.auth_token,
        timeout_ms=options.timeout_ms,
    )
    if options.require_health_ok and health.get("ok") is not True:
        raise RuntimeError(f"provider health is not ok: {json.dumps(health, ensure_ascii=False)}")

    decision_results = []
    successes = 0
    matching_provider_errors = 0
    for index in range(1, options.decision_count + 1):
        status, response, elapsed = request_json(
            base_url,
            "/v1/world-simulator/decision",
            method="POST",
            auth_token=options.auth_token,
            payload=build_decision_request(index, options.timeout_ms),
            timeout_ms=options.timeout_ms,
        )
        error_code = provider_error_code(response)
        error_message = provider_error_message(response)
        if not error_code:
            successes += 1
        if (
            options.expect_provider_error_code_substr
            and options.expect_provider_error_code_substr.lower()
            in f"{error_code}\n{error_message}".lower()
        ):
            matching_provider_errors += 1
        decision_results.append(
            {
                "index": index,
                "http_status": status,
                "provider_error_code": error_code or None,
                "provider_error_message": error_message[:500] if error_message else None,
                "provider_version": provider_version(response) or None,
                "elapsed_ms": int(elapsed * 1000),
                "decision": response.get("decision", {}).get("decision")
                if isinstance(response.get("decision"), dict)
                else None,
            }
        )

    if successes < options.min_successes:
        raise RuntimeError(
            f"provider decision successes {successes} below required {options.min_successes}"
        )
    if options.expect_provider_error_code_substr and matching_provider_errors == 0:
        raise RuntimeError(
            "expected provider_error code or message containing "
            f"`{options.expect_provider_error_code_substr}` but none was observed"
        )

    return {
        "status": "pass",
        "base_url": base_url,
        "provider_id": info.get("provider_id"),
        "info_http_status": info_status,
        "info_elapsed_ms": int(info_elapsed * 1000),
        "health_http_status": health_status,
        "health_status": health.get("status"),
        "health_ok": health.get("ok"),
        "health_elapsed_ms": int(health_elapsed * 1000),
        "decision_count": options.decision_count,
        "decision_successes": successes,
        "matching_provider_errors": matching_provider_errors,
        "decisions": decision_results,
    }


def parse_args(argv) -> SmokeOptions:
    parser = argparse.ArgumentParser(
        description="Smoke a remote provider bridge contract through its HTTP ingress."
    )
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--auth-token", default="")
    parser.add_argument("--timeout-ms", type=int, default=DEFAULT_TIMEOUT_MS)
    parser.add_argument("--decision-count", type=int, default=1)
    parser.add_argument("--min-successes", type=int, default=1)
    parser.add_argument("--expect-provider-error-code-substr", default="")
    parser.add_argument("--require-health-ok", action="store_true")
    args = parser.parse_args(argv)
    if args.timeout_ms <= 0:
        raise RuntimeError("--timeout-ms must be positive")
    if args.decision_count <= 0:
        raise RuntimeError("--decision-count must be positive")
    if args.min_successes < 0:
        raise RuntimeError("--min-successes cannot be negative")
    if args.min_successes > args.decision_count:
        raise RuntimeError("--min-successes cannot exceed --decision-count")
    return SmokeOptions(
        base_url=args.base_url,
        auth_token=args.auth_token,
        timeout_ms=args.timeout_ms,
        decision_count=args.decision_count,
        min_successes=args.min_successes,
        expect_provider_error_code_substr=args.expect_provider_error_code_substr,
        require_health_ok=args.require_health_ok,
    )


def main(argv) -> int:
    try:
        summary = run_smoke(parse_args(argv))
    except RuntimeError as exc:
        print(f"provider bridge contract smoke failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(summary, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
