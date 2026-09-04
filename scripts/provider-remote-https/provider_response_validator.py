"""Shared target-provider response shape validation.

This module is imported by the local contract smoke and embedded into the
remote loopback live gate. Keeping one validator prevents the two ingress
paths from disagreeing about whether a response is a usable decision.
"""

import re


CONTINUOUS_CONTEXT_DISCRIMINATOR = "oasis7.continuous-agent-context"
CONTINUOUS_CONTEXT_VERSION = 1
CANONICAL_DIGEST_RE = re.compile(r"^blake3:[0-9a-f]{64}$")

SUPPORTED_DECISIONS = {
    "wait",
    "wait_ticks",
    "act",
    "query",
    "module_command",
    "module_command_response",
}


def _require_object(value, label):
    if not isinstance(value, dict):
        raise RuntimeError(f"{label} must be a JSON object")
    return value


def _require_digest(value, label):
    if not isinstance(value, str) or not CANONICAL_DIGEST_RE.fullmatch(value):
        raise RuntimeError(f"{label} must be a canonical blake3: digest")


def validate_base_decision_response(response):
    """Require a non-empty, tagged, structurally valid inner response."""
    if not isinstance(response, dict) or not response:
        raise RuntimeError(
            "target provider response base decision response must contain a decision or provider_error"
        )
    provider_error = response.get("provider_error")
    decision = response.get("decision")
    if provider_error is not None:
        if not isinstance(provider_error, dict):
            raise RuntimeError("target provider response provider_error must be an object")
        if not isinstance(provider_error.get("code"), str) or not provider_error["code"].strip():
            raise RuntimeError("target provider response provider_error requires code")
        if not isinstance(provider_error.get("message"), str) or not provider_error["message"].strip():
            raise RuntimeError("target provider response provider_error requires message")
        if "retryable" in provider_error and not isinstance(provider_error["retryable"], bool):
            raise RuntimeError("target provider response provider_error requires boolean retryable")
        if decision is not None and decision != "wait":
            raise RuntimeError(
                "target provider response provider_error may only accompany the wait decision"
            )
        return

    if not isinstance(decision, str) or decision not in SUPPORTED_DECISIONS:
        raise RuntimeError(
            "target provider response base decision response requires a supported decision tag"
        )
    if decision == "wait_ticks":
        ticks = response.get("ticks")
        if isinstance(ticks, bool) or not isinstance(ticks, int) or ticks < 0:
            raise RuntimeError("target provider response wait_ticks requires non-negative integer ticks")
    elif decision == "act":
        if not isinstance(response.get("action_ref"), str) or not response["action_ref"].strip():
            raise RuntimeError("target provider response act requires action_ref")
        _require_object(response.get("action"), "target provider response act action")
    elif decision == "query":
        if not isinstance(response.get("query_ref"), str) or not response["query_ref"].strip():
            raise RuntimeError("target provider response query requires query_ref")
        _require_object(response.get("query"), "target provider response query")
    elif decision == "module_command":
        _require_object(response.get("module_command"), "target provider response module_command")
    elif decision == "module_command_response":
        _require_object(response.get("response"), "target provider response module response")


def validate_target_context_response(response, request):
    """Validate the complete outer response wrapper and its inner decision."""
    response = _require_object(response, "target provider response")
    request = _require_object(request, "target decision context request")
    if response.get("context_discriminator") != CONTINUOUS_CONTEXT_DISCRIMINATOR:
        raise RuntimeError("target provider response missing continuous-agent wrapper")
    if response.get("context_version") != CONTINUOUS_CONTEXT_VERSION:
        raise RuntimeError("target provider response context version is not 1")
    for field in (
        "agent_session_id",
        "agent_turn_id",
        "decision_request_id",
        "request_digest",
    ):
        if response.get(field) != request.get(field):
            raise RuntimeError(f"target provider response {field} does not echo request")
    if response.get("retry_seq") != request.get("retry_seq"):
        raise RuntimeError("target provider response retry lineage does not echo request")
    if response.get("transport_attempt") != request.get("transport_attempt"):
        raise RuntimeError("target provider response transport lineage does not echo request")
    _require_digest(response.get("request_digest"), "target provider response request_digest")
    _require_digest(response.get("response_digest"), "target provider response response_digest")
    base = _require_object(response.get("base_decision_response"), "target provider response base")
    validate_base_decision_response(base)
    return base
