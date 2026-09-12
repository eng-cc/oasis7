"""Canonical producer helpers for repository-owned aggregate leaf results."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
import re
from typing import Any, Callable


MARKER = "oasis7-loop-leaf-result"
SCHEMA = "oasis7.loop-leaf-result/v1"
UID = re.compile(r"task_[0-9a-f]{32}\Z")
OID = re.compile(r"[0-9a-f]{40}\Z")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")

# Keep this allowlist aligned with claim-ready.sh's repository-owned profiles.
# The fixture profile is retained solely for deterministic local tests; live
# readers still require github_live_query authority before using a result.
VERIFICATION_PROFILE_MODES = {
    "codex_subagent_role_fit": frozenset({"live_nonfinal", "detached_frozen_tree"}),
    "workflow_behavior": frozenset({"live_nonfinal", "detached_frozen_tree"}),
    "repository_required": frozenset({"live_nonfinal", "detached_frozen_tree"}),
    "fixture_repository_state": frozenset({"fixture"}),
}
VERIFICATION_FIELDS = frozenset({
    "profile",
    "mode",
    "frozen_source_head",
    "frozen_source_tree",
    "repository_fingerprint_before",
    "repository_fingerprint_after",
    "verification_epoch_stable",
    "verification_exit_code",
})


def verification_projection_errors(verification: Any) -> list[str]:
    """Validate the closed repository-owned claim-ready projection."""
    if not isinstance(verification, dict):
        return ["leaf result verification is invalid"]
    errors: list[str] = []
    unexpected = sorted(set(verification) - VERIFICATION_FIELDS)
    errors.extend(f"leaf result verification field is not supported: {field}" for field in unexpected)
    profile = verification.get("profile")
    modes = VERIFICATION_PROFILE_MODES.get(profile) if isinstance(profile, str) else None
    if modes is None:
        errors.append("leaf result verification profile is not repository-owned")
    mode = verification.get("mode")
    if not isinstance(mode, str) or not mode.strip():
        errors.append("leaf result verification mode is missing")
    elif modes is not None and mode not in modes:
        errors.append("leaf result verification mode is not supported for profile")
    for field, pattern in (
        ("frozen_source_head", OID),
        ("frozen_source_tree", OID),
        ("repository_fingerprint_before", re.compile(r"[0-9a-f]{64}\Z")),
        ("repository_fingerprint_after", re.compile(r"[0-9a-f]{64}\Z")),
    ):
        if field in verification and (not isinstance(verification[field], str) or not pattern.fullmatch(verification[field])):
            errors.append(f"leaf result verification {field} is invalid")
    if verification.get("verification_exit_code") != 0:
        errors.append("leaf result verification exit code is not zero")
    if verification.get("verification_epoch_stable") is not True:
        errors.append("leaf result verification epoch is not stable")
    return errors


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def configuration_projection(candidate: dict[str, Any]) -> dict[str, Any]:
    return {
        key: deepcopy(candidate.get(key))
        for key in (
            "entry",
            "environment",
            "effective_policy_identity",
            "effective_helper_identity",
            "effective_workflow_identity",
            "consumed_contracts",
        )
    }


def verification_digest(verification: dict[str, Any]) -> str:
    return canonical_digest(verification)


def leaf_evidence_digest(
    task_uid: str, status: str, candidate: dict[str, Any], verification_digest_value: str,
) -> str:
    return canonical_digest({
        "task_uid": task_uid,
        "status": status,
        "candidate": deepcopy(candidate),
        "verification_digest": verification_digest_value,
    })


def build_leaf_result(
    *, task_uid: str, change_id: str, obligation_id: str, mapping_slot: str,
    candidate: dict[str, Any], verification: dict[str, Any], status: str = "passed",
) -> dict[str, Any]:
    """Build a result only from the repository-owned verification projection."""
    if not isinstance(task_uid, str) or not UID.fullmatch(task_uid):
        raise ValueError("leaf result task_uid is invalid")
    if not isinstance(change_id, str) or not change_id.strip():
        raise ValueError("leaf result change_id is invalid")
    if not isinstance(obligation_id, str) or not obligation_id.strip():
        raise ValueError("leaf result obligation_id is invalid")
    if not isinstance(mapping_slot, str) or not mapping_slot.strip():
        raise ValueError("leaf result mapping_slot is invalid")
    if status != "passed":
        raise ValueError("leaf result status must be passed")
    if not isinstance(candidate, dict):
        raise ValueError("leaf result candidate is invalid")
    if not isinstance(verification, dict):
        raise ValueError("leaf result verification is invalid")
    verification_errors = verification_projection_errors(verification)
    if verification_errors:
        raise ValueError("; ".join(verification_errors))
    if candidate.get("configuration_digest") != canonical_digest(configuration_projection(candidate)):
        raise ValueError("leaf result configuration_digest does not match effective metadata")
    result = {
        "marker": MARKER,
        "schema": SCHEMA,
        "task_uid": task_uid,
        "change_id": change_id,
        "obligation_id": obligation_id,
        "mapping_slot": mapping_slot,
        "status": status,
        "candidate": deepcopy(candidate),
        "verification": deepcopy(verification),
    }
    result["verification_digest"] = verification_digest(result["verification"])
    result["evidence_digest"] = leaf_evidence_digest(
        task_uid, status, result["candidate"], result["verification_digest"]
    )
    return result


def canonical_body(result: dict[str, Any]) -> str:
    if result.get("marker") != MARKER or result.get("schema") != SCHEMA:
        raise ValueError("leaf result marker/schema is invalid")
    return canonical_bytes(result).decode("utf-8")


def leaf_result_locator(repository: str, issue_number: int, comment_id: int, body: str) -> dict[str, Any]:
    if not isinstance(repository, str) or not repository.strip():
        raise ValueError("leaf result repository is invalid")
    if type(issue_number) is not int or issue_number < 1 or type(comment_id) is not int or comment_id < 1:
        raise ValueError("leaf result Issue/comment identity is invalid")
    if not isinstance(body, str):
        raise ValueError("leaf result body is invalid")
    return {
        "repository": repository,
        "issue_number": issue_number,
        "comment_id": comment_id,
        "body_digest": "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest(),
    }


def publish_leaf_result(
    result: dict[str, Any], publish: Callable[[str], dict[str, Any]], readback: Callable[[dict[str, Any]], Any],
) -> dict[str, Any]:
    """Publish and immediately read back the exact canonical result body."""
    body = canonical_body(result)
    identity = publish(body)
    if not isinstance(identity, dict):
        raise ValueError("leaf result publication returned no identity")
    locator = leaf_result_locator(
        str(identity.get("repository") or ""),
        identity.get("issue_number"),
        identity.get("comment_id"),
        body,
    )
    observed = readback(locator)
    if not isinstance(observed, dict) or observed.get("comment", {}).get("body") != body:
        raise ValueError("leaf result publication readback mismatch")
    locator["body_digest"] = "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest()
    return locator
