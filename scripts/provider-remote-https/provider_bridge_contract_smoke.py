#!/usr/bin/env python3

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import sys
import time
from typing import Optional, Tuple
import urllib.error
import urllib.request


_SHARED_VALIDATOR_DIR = Path(__file__).resolve().parent
if str(_SHARED_VALIDATOR_DIR) not in sys.path:
    sys.path.insert(0, str(_SHARED_VALIDATOR_DIR))

from provider_response_validator import (  # noqa: E402
    CANONICAL_DIGEST_RE,
    CONTINUOUS_CONTEXT_DISCRIMINATOR,
    CONTINUOUS_CONTEXT_VERSION,
    SUPPORTED_DECISIONS,
    _require_digest,
    _require_object,
    validate_base_decision_response,
    validate_target_context_response,
)


DEFAULT_TIMEOUT_MS = 15000
CHAIN_RESOURCE_MANIFEST_SCHEMA_V1 = "oasis7.world_resource_manifest.v1"
CHAIN_RESOURCE_DELTA_SCHEMA_V1 = "oasis7.world_resource_delta.v1"
TARGET_DECISION_PATH = "/v1/world-simulator/decision-context"
TARGET_FEEDBACK_PATH = "/v1/world-simulator/feedback-context"
LEGACY_DECISION_PATH = "/v1/world-simulator/decision"


@dataclass(frozen=True)
class SmokeOptions:
    base_url: str
    auth_token: str
    timeout_ms: int
    decision_count: int
    min_successes: int
    expect_provider_error_code_substr: str
    require_health_ok: bool
    target_context_payload_file: Optional[str] = None
    legacy_compatibility_only: bool = False


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
    """Build the old bare DTO for an explicitly selected legacy smoke only."""
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


def build_target_context_request(index: int, timeout_ms: int) -> dict:
    """Build a shape-valid fixture for the local mock server tests.

    Live target smoke must use a Runtime-issued payload file.  The fixture is
    intentionally not used as a production default because its digest values
    are shape-only placeholders and the bridge must verify canonical hashes.
    """
    fake_digest = "blake3:" + ("a" * 64)
    base = build_decision_request(index, timeout_ms)
    base["capability_catalog"] = {
        "snapshot_id": f"catalog.smoke-{index}",
        "world_id": "world-bridge",
        "world_head": index,
        "branch_id": "main",
        "finality_epoch": 1,
        "logical_tick": index,
        "module_registry_hash": fake_digest,
        "policy_hash": fake_digest,
        "revocation_epoch": 0,
        "subject": {
            "kind": "agent",
            "agent_id": f"smoke-agent-{index}",
            "owner_binding": "smoke-owner",
            "generation": 1,
        },
        "presenter": {
            "presenter_id": "provider-bridge-smoke",
            "presenter_kind": "provider",
            "session_id": f"smoke-session-{index}",
        },
        "audience": {
            "world_id": "world-bridge",
            "branch_id": "main",
            "finality_epoch": 1,
            "target_kind": "world",
            "target_id": None,
        },
        "entries": [],
        "valid_until_tick": 100,
    }
    base["capability_invocation_context"] = {
        "grant_id": f"grant.smoke-{index}",
        "subject": base["capability_catalog"]["subject"],
        "presenter": base["capability_catalog"]["presenter"],
        "audience": base["capability_catalog"]["audience"],
        "catalog_snapshot_id": base["capability_catalog"]["snapshot_id"],
        "module_id": "",
        "module_version": "",
        "response_nonce": f"nonce.smoke-{index}",
    }
    return {
        "base_decision_request": base,
        "context_discriminator": CONTINUOUS_CONTEXT_DISCRIMINATOR,
        "context_version": CONTINUOUS_CONTEXT_VERSION,
        "protocol_version": "continuous-agent-v1",
        "agent_session_id": f"smoke-session-{index}",
        "agent_turn_id": f"smoke-turn-{index}",
        "decision_request_id": f"smoke-request-{index}",
        "retry_seq": 1,
        "transport_attempt": 1,
        "agent_subject": f"smoke-agent-{index}",
        "runtime_binding": {
            "world_id": "world-bridge",
            "branch_id": "main",
            "finality_epoch": 1,
            "finality_block_hash": fake_digest,
            "finality_status": "verified",
            "base_tick": index,
            "base_world_hash": fake_digest,
            "reorg_epoch": 0,
            "runtime_manifest_hash": fake_digest,
        },
        "observation_digest": fake_digest,
        "capability_catalog_digest": fake_digest,
        "capability_invocation_context_digest": fake_digest,
        "memory_snapshot_digest": fake_digest,
        "goal_snapshot_digest": fake_digest,
        "continuation_digest": fake_digest,
        "adapter_protocol_version": "loopback-http-v1",
        "budget_contract": {
            "max_latency_ms": timeout_ms,
            "max_repair_attempts": 1,
        },
        "request_digest": fake_digest,
    }


def validate_target_context_request(request: dict) -> None:
    if request.get("context_discriminator") != CONTINUOUS_CONTEXT_DISCRIMINATOR:
        raise RuntimeError(
            "target decision context must use the continuous-agent outer wrapper"
        )
    if request.get("context_version") != CONTINUOUS_CONTEXT_VERSION:
        raise RuntimeError("target decision context version must be 1")
    for field in (
        "protocol_version",
        "agent_session_id",
        "agent_turn_id",
        "decision_request_id",
        "agent_subject",
        "runtime_binding",
        "budget_contract",
        "base_decision_request",
    ):
        value = request.get(field)
        if field.endswith("_id") or field in {"protocol_version", "agent_subject"}:
            if not isinstance(value, str) or not value.strip():
                raise RuntimeError(f"target decision context requires {field}")
        else:
            _require_object(value, f"target decision context {field}")
    for field in ("retry_seq", "transport_attempt"):
        value = request.get(field)
        if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
            raise RuntimeError(
                f"target decision context {field} requires a positive integer"
            )
    for field in (
        "observation_digest",
        "capability_catalog_digest",
        "capability_invocation_context_digest",
        "memory_snapshot_digest",
        "goal_snapshot_digest",
        "continuation_digest",
        "request_digest",
    ):
        _require_digest(request.get(field), f"target decision context {field}")
    runtime_binding = request["runtime_binding"]
    for field in ("world_id", "branch_id", "finality_status", "base_world_hash", "runtime_manifest_hash"):
        if not str(runtime_binding.get(field) or "").strip():
            raise RuntimeError(f"target Runtime binding requires {field}")
    _require_digest(runtime_binding.get("base_world_hash"), "target Runtime binding base_world_hash")
    _require_digest(
        runtime_binding.get("runtime_manifest_hash"),
        "target Runtime binding runtime_manifest_hash",
    )
    if runtime_binding.get("finality_status") == "verified":
        _require_digest(
            runtime_binding.get("finality_block_hash"),
            "verified target Runtime binding finality_block_hash",
        )
    base = request["base_decision_request"]
    observation = _require_object(base.get("observation"), "target base decision observation")
    if observation.get("agent_id") != request["agent_subject"]:
        raise RuntimeError("target decision subject does not match observation agent")
    if not isinstance(base.get("capability_catalog"), dict) or not isinstance(
        base.get("capability_invocation_context"), dict
    ):
        raise RuntimeError("target decision requires Runtime capability catalog and invocation context")
    catalog = base["capability_catalog"]
    invocation = base["capability_invocation_context"]
    if catalog.get("snapshot_id") != invocation.get("catalog_snapshot_id"):
        raise RuntimeError("target capability catalog/invocation snapshot mismatch")
    if catalog.get("subject") != invocation.get("subject"):
        raise RuntimeError("target capability catalog/invocation subject mismatch")
    if catalog.get("presenter") != invocation.get("presenter"):
        raise RuntimeError("target capability catalog/invocation presenter mismatch")
    if invocation.get("response_nonce", "").strip() == "":
        raise RuntimeError("target capability invocation requires response_nonce")


def validate_target_context_feedback(feedback: dict, request: dict) -> None:
    for field in (
        "feedback_id",
        "agent_subject",
        "agent_session_id",
        "agent_turn_id",
        "decision_request_id",
        "status",
        "provenance",
        "request_digest",
    ):
        if field not in feedback:
            raise RuntimeError(f"target feedback wrapper requires {field}")
    if feedback["provenance"] != "runtime_authoritative":
        raise RuntimeError("target feedback must carry Runtime-authoritative provenance")
    if feedback["status"] not in {"pending", "committed", "rejected", "failed"}:
        raise RuntimeError("target feedback status is outside the v1 registry")
    feedback_seq = feedback.get("feedback_seq")
    if isinstance(feedback_seq, bool) or not isinstance(feedback_seq, int) or feedback_seq <= 0:
        raise RuntimeError("target feedback feedback_seq requires a positive integer")
    _require_digest(feedback["request_digest"], "target feedback request_digest")
    for field in ("agent_subject", "agent_session_id", "agent_turn_id", "decision_request_id"):
        request_field = "agent_subject" if field == "agent_subject" else field
        if feedback[field] != request[request_field]:
            raise RuntimeError(f"target feedback {field} does not match decision context")
    if feedback["request_digest"] != request["request_digest"]:
        raise RuntimeError("target feedback request_digest does not match decision context")
    if feedback["status"] == "committed" and (
        feedback.get("candidate_action_id") is None
        or not str(feedback.get("runtime_receipt_id") or "").strip()
    ):
        raise RuntimeError("committed target feedback requires action and Runtime receipt")


def load_target_context_pairs(path: str, decision_count: int) -> list[tuple[dict, dict]]:
    try:
        with open(path, encoding="utf-8") as handle:
            payload = json.load(handle)
    except (OSError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"cannot read target context payload file {path}: {exc}") from exc
    root = _require_object(payload, "target context payload file")
    raw_pairs = root.get("requests")
    if not isinstance(raw_pairs, list) or len(raw_pairs) < decision_count:
        raise RuntimeError(
            "target context payload file must contain at least "
            f"{decision_count} paired requests"
        )
    pairs = []
    for index, raw_pair in enumerate(raw_pairs[:decision_count], start=1):
        pair = _require_object(raw_pair, f"target context payload request {index}")
        request = _require_object(pair.get("decision_context"), f"target request {index}")
        feedback = _require_object(pair.get("feedback_context"), f"target feedback {index}")
        validate_target_context_request(request)
        validate_target_context_feedback(feedback, request)
        pairs.append((request, feedback))
    return pairs


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


def decision_tag(response: dict) -> Optional[str]:
    value = response.get("decision")
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        nested = value.get("decision")
        return nested if isinstance(nested, str) else None
    return None


def require_provider_resource_schema(info: dict) -> None:
    manifest_schema = str(info.get("chain_resource_manifest_schema_version") or "").strip()
    delta_schema = str(info.get("chain_resource_delta_schema_version") or "").strip()
    if manifest_schema != CHAIN_RESOURCE_MANIFEST_SCHEMA_V1:
        raise RuntimeError(
            "provider resource manifest schema mismatch: "
            f"expected {CHAIN_RESOURCE_MANIFEST_SCHEMA_V1}, got {manifest_schema or '<missing>'}"
        )
    if delta_schema != CHAIN_RESOURCE_DELTA_SCHEMA_V1:
        raise RuntimeError(
            "provider resource delta schema mismatch: "
            f"expected {CHAIN_RESOURCE_DELTA_SCHEMA_V1}, got {delta_schema or '<missing>'}"
        )


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
    require_provider_resource_schema(info)
    if options.require_health_ok and health.get("ok") is not True:
        raise RuntimeError(f"provider health is not ok: {json.dumps(health, ensure_ascii=False)}")

    if options.legacy_compatibility_only and options.target_context_payload_file:
        raise RuntimeError(
            "--legacy-compatibility-only cannot be combined with "
            "--target-context-payload-file"
        )
    target_pairs = []
    if options.legacy_compatibility_only:
        lane = "legacy_compatibility_only"
    else:
        if not options.target_context_payload_file:
            raise RuntimeError(
                "target cognition smoke requires --target-context-payload-file; "
                "select --legacy-compatibility-only only for the bare compatibility route"
            )
        target_pairs = load_target_context_pairs(
            options.target_context_payload_file, options.decision_count
        )
        lane = "target_cognition"

    decision_results = []
    successes = 0
    matching_provider_errors = 0
    feedback_successes = 0
    for index in range(1, options.decision_count + 1):
        target_request = None
        target_feedback = None
        if options.legacy_compatibility_only:
            path = LEGACY_DECISION_PATH
            payload = build_decision_request(index, options.timeout_ms)
        else:
            target_request, target_feedback = target_pairs[index - 1]
            path = TARGET_DECISION_PATH
            payload = target_request
        status, response, elapsed = request_json(
            base_url,
            path,
            method="POST",
            auth_token=options.auth_token,
            payload=payload,
            timeout_ms=options.timeout_ms,
        )
        if target_request is not None:
            base_response = validate_target_context_response(response, target_request)
        else:
            base_response = response
        error_code = provider_error_code(base_response)
        error_message = provider_error_message(base_response)
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
                "provider_version": provider_version(base_response) or None,
                "elapsed_ms": int(elapsed * 1000),
                "decision": decision_tag(base_response),
            }
        )
        if target_request is not None:
            feedback_status, feedback_response, feedback_elapsed = request_json(
                base_url,
                TARGET_FEEDBACK_PATH,
                method="POST",
                auth_token=options.auth_token,
                payload=target_feedback,
                timeout_ms=options.timeout_ms,
            )
            if feedback_response.get("ok") is not True:
                raise RuntimeError(
                    "target feedback context was not acknowledged: "
                    f"{json.dumps(feedback_response, ensure_ascii=False)}"
                )
            feedback_successes += 1
            decision_results[-1]["feedback_http_status"] = feedback_status
            decision_results[-1]["feedback_elapsed_ms"] = int(feedback_elapsed * 1000)
            decision_results[-1]["decision"] = decision_tag(base_response)

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
        "lane": lane,
        "base_url": base_url,
        "provider_id": info.get("provider_id"),
        "chain_resource_manifest_schema_version": info.get(
            "chain_resource_manifest_schema_version"
        ),
        "chain_resource_delta_schema_version": info.get("chain_resource_delta_schema_version"),
        "info_http_status": info_status,
        "info_elapsed_ms": int(info_elapsed * 1000),
        "health_http_status": health_status,
        "health_status": health.get("status"),
        "health_ok": health.get("ok"),
        "health_elapsed_ms": int(health_elapsed * 1000),
        "decision_count": options.decision_count,
        "decision_successes": successes,
        "feedback_successes": feedback_successes,
        "decision_path": TARGET_DECISION_PATH
        if not options.legacy_compatibility_only
        else LEGACY_DECISION_PATH,
        "feedback_path": TARGET_FEEDBACK_PATH if not options.legacy_compatibility_only else None,
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
    parser.add_argument(
        "--target-context-payload-file",
        help=(
            "Runtime-issued JSON file containing paired decision_context and "
            "feedback_context wrappers; required for the target lane"
        ),
    )
    parser.add_argument(
        "--legacy-compatibility-only",
        action="store_true",
        help="explicitly test only the bare legacy DecisionRequest route",
    )
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
        target_context_payload_file=args.target_context_payload_file,
        legacy_compatibility_only=args.legacy_compatibility_only,
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
