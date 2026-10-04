#!/usr/bin/env python3
"""Repository-owned validation for accepted task_complete claim evidence."""
from __future__ import annotations

import base64
import hashlib
import json
import re
from datetime import datetime
from typing import Any, Iterable

from loop_leaf_result import VERIFICATION_PROFILE_MODES, verification_projection_errors


CLAIM_FIELDS = frozenset({
    "claim_type", "verify_command", "verified_at", "verification_exit_code",
    "status", "allowed_to_claim", "claim_message", "blocked_phrase",
    "success_phrase", "task_uid", "repository_fingerprint_before",
    "repository_fingerprint_after", "verification_epoch_stable",
    "verification_mode", "frozen_source_head", "frozen_source_tree",
    "comparison_ref", "verification_profile", "repository_head",
    "repository_index_sha256",
})
PRODUCTION_PROFILES = frozenset({
    "codex_subagent_role_fit", "workflow_behavior", "repository_required",
})
OID = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
UID = re.compile(r"task_[0-9a-f]{32}\Z")


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def canonical_claim_digest(claim: dict[str, Any]) -> str:
    return "sha256:" + hashlib.sha256(canonical_json_bytes(claim)).hexdigest()


def _parse_timestamp(value: Any) -> datetime:
    if not isinstance(value, str) or not value.strip():
        raise ValueError("task_complete claim verified_at is missing")
    try:
        result = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("task_complete claim verified_at is invalid") from exc
    if result.tzinfo is None:
        raise ValueError("task_complete claim verified_at has no timezone")
    return result


def validate_task_complete_claim(
    repository: str,
    task_uid: str,
    claim: Any,
    *,
    expected_head: str | None = None,
) -> str:
    """Validate the closed claim-ready projection and return its canonical digest.

    This is historical-safe: it validates the frozen identity carried by the
    accepted claim rather than comparing it to today's checkout. Callers which
    accept a claim bind `expected_head` to the source revision under review.
    """
    if not isinstance(claim, dict) or set(claim) != CLAIM_FIELDS:
        raise ValueError("task_complete claim fields are missing or unsupported")
    if not UID.fullmatch(task_uid) or claim.get("task_uid") != task_uid:
        raise ValueError("task_complete claim Task UID mismatch")
    if (claim.get("claim_type") != "task_complete"
            or claim.get("status") != "verified"
            or claim.get("allowed_to_claim") is not True
            or type(claim.get("verification_exit_code")) is not int
            or claim.get("verification_exit_code") != 0
            or claim.get("verification_epoch_stable") is not True):
        raise ValueError("task_complete claim is not an accepted verified result")
    profile = claim.get("verification_profile")
    if profile not in PRODUCTION_PROFILES:
        raise ValueError("task_complete claim requires a repository-owned production profile")
    command = {
        "codex_subagent_role_fit": f"./scripts/pm/verify-codex-subagent-role-fit.sh --task-uid {task_uid}",
        "workflow_behavior": "./scripts/pm/workflow-behavior-eval.sh",
        "repository_required": "true",
    }[profile]
    if claim.get("verify_command") != command:
        raise ValueError("task_complete claim profile/command is not repository-owned")
    mode = claim.get("verification_mode")
    if mode not in VERIFICATION_PROFILE_MODES.get(profile, frozenset()):
        raise ValueError("task_complete claim verification mode is unsupported")
    if profile == "fixture_repository_state":
        raise ValueError("task_complete claim fixture profile is not production evidence")

    verification = {
        "profile": profile,
        "mode": mode,
        "frozen_source_head": claim.get("frozen_source_head"),
        "frozen_source_tree": claim.get("frozen_source_tree"),
        "repository_fingerprint_before": claim.get("repository_fingerprint_before"),
        "repository_fingerprint_after": claim.get("repository_fingerprint_after"),
        "verification_epoch_stable": claim.get("verification_epoch_stable"),
        "verification_exit_code": claim.get("verification_exit_code"),
    }
    if verification_projection_errors(verification):
        raise ValueError("task_complete claim verification projection is incomplete or unsupported")
    source_head = claim.get("frozen_source_head")
    if not isinstance(source_head, str) or not OID.fullmatch(source_head):
        raise ValueError("task_complete claim frozen source head is invalid")
    if claim.get("repository_head") != source_head:
        raise ValueError("task_complete claim repository head differs from frozen source head")
    if claim.get("frozen_source_tree") is not None and not OID.fullmatch(str(claim.get("frozen_source_tree"))):
        raise ValueError("task_complete claim frozen source tree is invalid")
    for key in ("repository_fingerprint_before", "repository_fingerprint_after", "repository_index_sha256"):
        value = claim.get(key)
        if not isinstance(value, str) or not SHA256.fullmatch(value):
            raise ValueError(f"task_complete claim {key} is invalid")
    if claim.get("repository_fingerprint_before") != claim.get("repository_fingerprint_after"):
        raise ValueError("task_complete claim verification fingerprints differ")
    if not isinstance(repository, str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", repository):
        raise ValueError("task_complete claim repository identity is invalid")
    if expected_head is not None and (not OID.fullmatch(expected_head) or source_head != expected_head):
        raise ValueError("task_complete claim is stale or does not match accepted source head")
    _parse_timestamp(claim.get("verified_at"))
    return canonical_claim_digest(claim)


def _unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("task_complete claim Issue evidence contains duplicate JSON keys")
        result[key] = value
    return result


def issue_claim_history(issue_body: Any) -> list[dict[str, Any]]:
    """Decode the exact claim history projection written to the Task Issue."""
    if not isinstance(issue_body, str):
        raise ValueError("task_complete claim Issue body is unavailable")
    matches = re.findall(r"^- claim_verifications_b64: `([^`]+)`$", issue_body.replace("\r\n", "\n"), re.MULTILINE)
    if len(matches) != 1:
        raise ValueError("task_complete claim Issue history is missing or ambiguous")
    encoded = matches[0]
    try:
        raw = base64.b64decode(encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True)
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_json_object)
    except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError("task_complete claim Issue history is invalid") from exc
    if not isinstance(value, list) or any(not isinstance(item, dict) for item in value):
        raise ValueError("task_complete claim Issue history is malformed")
    return value


def _claim_comment_body(task_uid: str, claim: dict[str, Any]) -> str:
    return "\n".join((
        "<!-- oasis7-pm-claim-verification -->",
        f"Task UID: {task_uid}",
        f"Claim Type: {claim['claim_type']}",
        f"Verified At: {claim['verified_at']}",
        f"Verification Exit Code: {claim['verification_exit_code']}",
        f"Verification Status: {claim['status']}",
        f"Verify Command: {claim['verify_command']}",
        f"Claim Message: {claim.get('claim_message') or ''}",
        "",
    ))


def _validate_claim_comment(
    repository: str,
    issue_number: int,
    task_uid: str,
    claim: dict[str, Any],
    comments: Iterable[dict[str, Any]],
    *,
    require_latest: bool,
    issue_updated_at: Any = None,
) -> dict[str, Any]:
    comments = list(comments)
    issue_url = f"https://github.com/{repository}/issues/{issue_number}"
    api_issue_url = f"https://api.github.com/repos/{repository}/issues/{issue_number}"
    expected_body = _claim_comment_body(task_uid, claim)
    matches = []
    for comment in comments:
        if not isinstance(comment, dict) or comment.get("body") != expected_body:
            continue
        comment_id = comment.get("id")
        if type(comment_id) is not int or comment_id <= 0:
            continue
        if (comment.get("html_url") != f"{issue_url}#issuecomment-{comment_id}"
                or comment.get("issue_url") != api_issue_url):
            continue
        try:
            created = _parse_timestamp(comment.get("created_at"))
            updated = _parse_timestamp(comment.get("updated_at"))
            verified = _parse_timestamp(claim.get("verified_at"))
        except ValueError:
            continue
        if created < verified or updated != created:
            continue
        matches.append(comment)
    if len(matches) != 1:
        raise ValueError("task_complete claim live comment is missing, ambiguous, or edited")
    if require_latest:
        def created_at(item: dict[str, Any]) -> datetime:
            return _parse_timestamp(item.get("created_at"))
        try:
            latest = max(comments, key=created_at, default=None)
        except ValueError as exc:
            raise ValueError("task_complete claim Issue comment timestamps are invalid") from exc
        if latest is not matches[0]:
            raise ValueError("task_complete claim comment is not the latest Issue update")
        try:
            issue_updated = _parse_timestamp(issue_updated_at)
            comment_updated = _parse_timestamp(matches[0].get("updated_at"))
        except ValueError as exc:
            raise ValueError("task_complete claim Issue/comment timestamps are invalid") from exc
        if issue_updated != comment_updated:
            raise ValueError("task_complete claim comment is stale or superseded")
    return matches[0]


def validate_task_complete_claim_for_closeout(
    repository: str,
    task_uid: str,
    claim: Any,
    *,
    issue_number: int,
    issue_state: str,
    issue_url: str,
    issue_updated_at: Any,
    comments: Iterable[dict[str, Any]],
) -> str:
    """Validate the same accepted claim projection at ordinary/aggregate closeout."""
    if type(issue_number) is not int or issue_number <= 0:
        raise ValueError("task_complete claim Issue identity is invalid")
    canonical_issue_url = f"https://github.com/{repository}/issues/{issue_number}"
    if issue_url != canonical_issue_url or str(issue_state).upper() != "OPEN":
        raise ValueError("task_complete claim requires the exact open task Issue")
    digest = validate_task_complete_claim(repository, task_uid, claim)
    _validate_claim_comment(repository, issue_number, task_uid, claim, comments,
                            require_latest=True, issue_updated_at=issue_updated_at)
    return digest


def select_historical_task_complete_claim(
    repository: str,
    task_uid: str,
    task_record: dict[str, Any],
    live_issue: dict[str, Any],
    comments: Iterable[dict[str, Any]],
    *,
    accepted_head: str,
) -> tuple[dict[str, Any], str, dict[str, Any]]:
    """Select and replay the one accepted claim bound to H after worktree removal."""
    issue_number = live_issue.get("number")
    if type(issue_number) is not int or issue_number <= 0:
        raise ValueError("task_complete claim Issue identity is invalid")
    mapping_claims = task_record.get("claim_verifications")
    issue_claims = issue_claim_history(live_issue.get("body"))
    if not isinstance(mapping_claims, list) or any(not isinstance(item, dict) for item in mapping_claims):
        raise ValueError("task_complete claim mapping history is unavailable")
    if mapping_claims != issue_claims:
        raise ValueError("task_complete claim mapping/Issue history mismatch")
    candidates = [claim for claim in mapping_claims
                  if claim.get("claim_type") == "task_complete"
                  and claim.get("frozen_source_head") == accepted_head]
    if len(candidates) != 1:
        raise ValueError("terminal delivery accepted task_complete claim is missing, ambiguous, or invalid")
    claim = candidates[0]
    digest = validate_task_complete_claim(repository, task_uid, claim, expected_head=accepted_head)
    comment = _validate_claim_comment(repository, issue_number, task_uid, claim, comments,
                                      require_latest=False)
    return claim, digest, comment
