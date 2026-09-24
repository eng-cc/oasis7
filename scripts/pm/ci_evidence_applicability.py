#!/usr/bin/env python3
"""Pure, conservative applicability decisions for CI evidence reuse.

This is the C0 decision contract only.  It does not read GitHub, run tests,
publish evidence, or authorize promotion.  Until the effective policy enables
``input-scope-reuse/v1``, it returns ``disabled`` and leaves legacy gates in
control.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass
import re
from typing import Any

from ci_ready_receipt_identity import (
    INPUT_SCOPE_REUSE_CAPABILITY,
    REQUIRED_PLAN_V2_SCHEMA,
    read_required_plan_capabilities,
)
from ci_input_scope import aggregate_product_corpus_results, validate_input_scope_snapshot

_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_COMMON_IDENTITY_FIELDS = ("repository", "task_uid", "pr_number", "source_head_oid")
_BLOCKED = "blocked"
_REVALIDATE = "revalidate"
_REUSABLE = "reusable"
_DISABLED = "disabled"


@dataclass(frozen=True)
class ApplicabilityDecision:
    """Separated review, test-evidence, and merge-readiness observations."""

    source_review: str
    test_evidence: str
    merge_readiness: str
    reused_units: tuple[str, ...]
    required_test_units: tuple[str, ...]
    required_review_roles: tuple[str, ...]
    blockers: tuple[str, ...]

    def to_dict(self) -> dict[str, Any]:
        value = asdict(self)
        for field in (
            "reused_units", "required_test_units", "required_review_roles", "blockers",
        ):
            value[field] = list(value[field])
        return value


def _decision(
    source_review: str,
    test_evidence: str,
    merge_readiness: str,
    *,
    reused_units: tuple[str, ...] = (),
    required_test_units: tuple[str, ...] = (),
    required_review_roles: tuple[str, ...] = (),
    blockers: tuple[str, ...] = (),
) -> ApplicabilityDecision:
    return ApplicabilityDecision(
        source_review=source_review,
        test_evidence=test_evidence,
        merge_readiness=merge_readiness,
        reused_units=tuple(sorted(set(reused_units))),
        required_test_units=tuple(sorted(set(required_test_units))),
        required_review_roles=tuple(dict.fromkeys(required_review_roles)),
        blockers=tuple(sorted(set(blockers))),
    )


def _blocked(code: str) -> ApplicabilityDecision:
    return _decision(_BLOCKED, _BLOCKED, _BLOCKED, blockers=(code,))


def _identity(value: Any, field: str) -> str:
    if not isinstance(value, str) or not _OID_RE.fullmatch(value):
        raise ValueError(f"{field} must be a lowercase object ID")
    return value


def _digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or not _DIGEST_RE.fullmatch(value):
        raise ValueError(f"{field} must be a prefixed SHA-256 digest")
    return value


def _common_identity(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} identity must be an object")
    missing = [field for field in _COMMON_IDENTITY_FIELDS if field not in value]
    if missing:
        raise ValueError(f"{label} identity is missing: " + ",".join(missing))
    repository = value["repository"]
    if (not isinstance(repository, str)
            or not re.fullmatch(r"[^/\s]+/[^/\s]+", repository)):
        raise ValueError(f"{label} repository identity is invalid")
    task_uid = value["task_uid"]
    if not isinstance(task_uid, str) or not _UID_RE.fullmatch(task_uid):
        raise ValueError(f"{label} task UID is invalid")
    pr_number = value["pr_number"]
    if type(pr_number) is not int or pr_number < 1:
        raise ValueError(f"{label} PR number is invalid")
    source_head_oid = _identity(value["source_head_oid"], f"{label}.source_head_oid")
    return {
        "repository": repository,
        "task_uid": task_uid,
        "pr_number": pr_number,
        "source_head_oid": source_head_oid,
    }


def _string_list(value: Any, field: str) -> tuple[str, ...]:
    if (not isinstance(value, list)
            or any(not isinstance(item, str) or not item.strip() for item in value)
            or len(value) != len(set(value))):
        raise ValueError(f"{field} must be a unique string list")
    return tuple(value)


def _positive_int(value: Any, field: str) -> int:
    if type(value) is not int or value <= 0:
        raise ValueError(f"{field} must be a positive integer")
    return value


def _positive_numeric_id(value: Any, field: str) -> str:
    """Normalize a positive GitHub numeric ID without accepting bools."""
    if isinstance(value, bool):
        raise ValueError(f"{field} must be a positive numeric ID")
    if isinstance(value, int) and value > 0:
        return str(value)
    if isinstance(value, str) and re.fullmatch(r"[1-9][0-9]*", value):
        return value
    raise ValueError(f"{field} must be a positive numeric ID")


def _matches_common_identity(record: Any, expected: dict[str, Any], label: str) -> bool:
    actual = _common_identity(record, label)
    return all(actual[field] == expected[field] for field in _COMMON_IDENTITY_FIELDS)


def evaluate_evidence_applicability(
    source_plan: Any,
    evidence_set: Any,
    target_snapshot: Any,
    effective_policy: Any,
) -> ApplicabilityDecision:
    """Decide whether prior review/test evidence applies to a target snapshot.

    The v2 envelope must explicitly require the known capability.  Reuse is
    still off unless ``effective_policy.enabled_capabilities`` explicitly
    includes it and the policy binds the trusted ``check_app_id``. Evidence
    must carry that same app ID. Identity/provenance uncertainty blocks; known
    missing or changed obligations are returned as required revalidation work.
    """
    try:
        capabilities = read_required_plan_capabilities(source_plan)
    except (TypeError, ValueError):
        return _blocked("UNSUPPORTED_PROTOCOL")

    # Existing v1 receipts stay on the established legacy path.  This pure
    # helper cannot infer that a legacy evidence item has an input closure.
    if capabilities is None:
        return _decision(_DISABLED, _DISABLED, _DISABLED)

    if not isinstance(effective_policy, dict):
        return _blocked("EFFECTIVE_POLICY_INVALID")
    enabled = effective_policy.get("enabled_capabilities", [])
    if (not isinstance(enabled, list)
            or any(not isinstance(item, str) or not item for item in enabled)
            or enabled != sorted(set(enabled))):
        return _blocked("EFFECTIVE_POLICY_INVALID")
    unsupported_policy = sorted(set(enabled) - {INPUT_SCOPE_REUSE_CAPABILITY})
    if unsupported_policy:
        return _blocked("EFFECTIVE_POLICY_UNSUPPORTED_CAPABILITY")
    if INPUT_SCOPE_REUSE_CAPABILITY not in enabled:
        # Disabled means the existing full/legacy gate remains authoritative.
        return _decision(_DISABLED, _DISABLED, _DISABLED)
    try:
        expected_check_app_id = _positive_numeric_id(
            effective_policy.get("check_app_id"), "effective_policy.check_app_id",
        )
    except ValueError:
        return _blocked("EFFECTIVE_POLICY_INVALID")

    if not isinstance(source_plan, dict) or source_plan.get("schema") != REQUIRED_PLAN_V2_SCHEMA:
        return _blocked("UNSUPPORTED_PROTOCOL")
    if not isinstance(target_snapshot, dict):
        return _blocked("TARGET_SNAPSHOT_INVALID")
    if not isinstance(evidence_set, dict):
        return _blocked("EVIDENCE_SET_INVALID")

    try:
        source_identity = _common_identity(source_plan, "source plan")
        target_identity = _common_identity(target_snapshot, "target snapshot")
        _identity(target_snapshot.get("target_oid"), "target_snapshot.target_oid")
        if source_identity != target_identity:
            return _blocked("TASK_SOURCE_IDENTITY_MISMATCH")

        source_review_digest = _digest(
            source_plan.get("review_applicability_digest"),
            "source_plan.review_applicability_digest",
        )
        target_review_digest = _digest(
            target_snapshot.get("review_applicability_digest"),
            "target_snapshot.review_applicability_digest",
        )
        source_units = set(_string_list(
            source_plan.get("required_test_units"), "source_plan.required_test_units",
        ))
        source_roles = set(_string_list(
            source_plan.get("required_review_roles"), "source_plan.required_review_roles",
        ))
        input_scope = validate_input_scope_snapshot(target_snapshot.get("input_scope"))
        target_units = tuple(input_scope["required_test_units"])
        projected_target_units = _string_list(
            target_snapshot.get("required_test_units"), "target_snapshot.required_test_units",
        )
        if projected_target_units != target_units:
            raise ValueError("target required units disagree with the complete input-scope snapshot")
        target_roles = _string_list(
            target_snapshot.get("required_review_roles"), "target_snapshot.required_review_roles",
        )
        fingerprints = input_scope["input_fingerprints"]
    except (TypeError, ValueError, KeyError):
        return _blocked("APPLICABILITY_INPUT_INVALID")

    reviews = evidence_set.get("reviews", [])
    tests = evidence_set.get("tests", [])
    if not isinstance(reviews, list) or not isinstance(tests, list):
        return _blocked("EVIDENCE_SET_INVALID")
    if any(not isinstance(item, dict) for item in reviews + tests):
        return _blocked("EVIDENCE_RECORD_INVALID")

    corpus_result = aggregate_product_corpus_results(input_scope, tests)
    if corpus_result["status"] == "blocked":
        corpus_blockers = tuple(
            "TEST_" + code for code in corpus_result["blockers"]
        )
    else:
        corpus_blockers = ()

    blockers: list[str] = list(corpus_blockers)
    reused_units: list[str] = []
    required_units: list[str] = []
    required_roles: list[str] = []

    # A changed review applicability digest starts a new review epoch.  C0
    # deliberately does not try to reconstruct semantic review closure.
    for role in target_roles:
        if role not in source_roles or source_review_digest != target_review_digest:
            required_roles.append(role)
            continue
        matching = [item for item in reviews if item.get("role_id") == role]
        if not matching:
            required_roles.append(role)
            continue
        if len(matching) != 1:
            blockers.append("REVIEW_EVIDENCE_AMBIGUOUS")
            required_roles.append(role)
            continue
        record = matching[0]
        try:
            if not _matches_common_identity(record, source_identity, "review evidence"):
                blockers.append("REVIEW_IDENTITY_MISMATCH")
                required_roles.append(role)
                continue
            record_digest = _digest(
                record.get("applicability_digest"), "review.applicability_digest",
            )
        except (TypeError, ValueError):
            blockers.append("REVIEW_PROVENANCE_INVALID")
            required_roles.append(role)
            continue
        status = record.get("status")
        if record_digest != target_review_digest:
            required_roles.append(role)
        elif status == "passed":
            continue
        elif isinstance(status, str) and status in {"failed", "blocked", "pending"}:
            blockers.append("REVIEW_EVIDENCE_NOT_ACCEPTED")
            required_roles.append(role)
        else:
            blockers.append("REVIEW_EVIDENCE_STATUS_UNSUPPORTED")
            required_roles.append(role)

    for unit in target_units:
        if unit not in source_units:
            required_units.append(unit)
            continue
        matching = [item for item in tests if item.get("unit_id") == unit]
        if not matching:
            required_units.append(unit)
            continue
        if len(matching) != 1:
            blockers.append("TEST_EVIDENCE_AMBIGUOUS")
            required_units.append(unit)
            continue
        record = matching[0]
        try:
            if not _matches_common_identity(record, source_identity, "test evidence"):
                blockers.append("TEST_IDENTITY_MISMATCH")
                required_units.append(unit)
                continue
            record_digest = _digest(record.get("input_digest"), "test.input_digest")
            check_app_id = _positive_numeric_id(record.get("check_app_id"), "test.check_app_id")
            _positive_int(record.get("run_id"), "test.run_id")
            _positive_int(record.get("run_attempt"), "test.run_attempt")
            _positive_int(record.get("check_run_id"), "test.check_run_id")
            _positive_int(record.get("artifact_id"), "test.artifact_id")
        except (TypeError, ValueError):
            blockers.append("TEST_PROVENANCE_INVALID")
            required_units.append(unit)
            continue
        if check_app_id != expected_check_app_id:
            blockers.append("TEST_APP_IDENTITY_MISMATCH")
            required_units.append(unit)
            continue
        status = record.get("status")
        if record_digest != fingerprints.get(unit):
            required_units.append(unit)
        elif status == "passed":
            reused_units.append(unit)
        elif isinstance(status, str) and status in {"failed", "blocked", "pending"}:
            blockers.append("TEST_EVIDENCE_NOT_ACCEPTED")
            required_units.append(unit)
        else:
            blockers.append("TEST_EVIDENCE_STATUS_UNSUPPORTED")
            required_units.append(unit)

    source_review = (
        _BLOCKED if any(item.startswith("REVIEW_") for item in blockers)
        else _REVALIDATE if required_roles else _REUSABLE
    )
    test_evidence = (
        _BLOCKED if any(item.startswith("TEST_") for item in blockers)
        else _REVALIDATE if required_units else _REUSABLE
    )
    merge_readiness = (
        _BLOCKED if blockers
        else _REVALIDATE if source_review == _REVALIDATE or test_evidence == _REVALIDATE
        else _REUSABLE
    )
    return _decision(
        source_review,
        test_evidence,
        merge_readiness,
        reused_units=tuple(reused_units),
        required_test_units=tuple(required_units),
        required_review_roles=tuple(required_roles),
        blockers=tuple(blockers),
    )
