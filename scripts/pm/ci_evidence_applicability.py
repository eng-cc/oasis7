#!/usr/bin/env python3
"""Pure, conservative applicability decisions for CI evidence reuse.

This is the C0 decision contract only.  It does not read GitHub, run tests,
publish evidence, or authorize promotion.  Until the effective policy enables
``input-scope-reuse/v1``, it returns ``disabled`` and leaves legacy gates in
control.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass, field
import re
from typing import Any

from ci_ready_receipt_identity import (
    INPUT_SCOPE_REUSE_CAPABILITY,
    REQUIRED_PLAN_V2_SCHEMA,
    read_required_plan_capabilities,
)
from ci_input_scope import aggregate_product_corpus_results, validate_input_scope_snapshot
from ci_input_scope import validate_planner_inventory_binding
from integration_executor_contract import (
    EFFECTIVE_POLICY_IDENTITY_SCHEMA,
    effective_policy_digest,
)

_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
_OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_COMMON_IDENTITY_FIELDS = (
    "repository", "task_uid", "pr_number", "source_head_oid", "source_scope_oid",
)
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
    identity: dict[str, Any] = field(default_factory=dict)
    effective_policy_identity: dict[str, Any] | None = None
    evidence_locators: tuple[dict[str, Any], ...] = ()
    item_decisions: tuple[dict[str, Any], ...] = ()
    reasons: tuple[str, ...] = ()

    def to_dict(self) -> dict[str, Any]:
        value = asdict(self)
        for field in (
            "reused_units", "required_test_units", "required_review_roles", "blockers",
            "evidence_locators", "item_decisions", "reasons",
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
    identity: dict[str, Any] | None = None,
    effective_policy_identity: dict[str, Any] | None = None,
    evidence_locators: tuple[dict[str, Any], ...] = (),
    item_decisions: tuple[dict[str, Any], ...] = (),
    reasons: tuple[str, ...] = (),
) -> ApplicabilityDecision:
    return ApplicabilityDecision(
        source_review=source_review,
        test_evidence=test_evidence,
        merge_readiness=merge_readiness,
        reused_units=tuple(sorted(set(reused_units))),
        required_test_units=tuple(sorted(set(required_test_units))),
        required_review_roles=tuple(dict.fromkeys(required_review_roles)),
        blockers=tuple(sorted(set(blockers))),
        identity=dict(identity or {}),
        effective_policy_identity=(
            dict(effective_policy_identity) if effective_policy_identity is not None else None
        ),
        evidence_locators=tuple(dict(item) for item in evidence_locators),
        item_decisions=tuple(dict(item) for item in item_decisions),
        reasons=tuple(sorted(set(reasons))),
    )


def _blocked(
    code: str, *, identity: dict[str, Any] | None = None,
    effective_policy_identity: dict[str, Any] | None = None,
    item_decisions: tuple[dict[str, Any], ...] = (),
) -> ApplicabilityDecision:
    return _decision(
        _BLOCKED, _BLOCKED, _BLOCKED, blockers=(code,),
        identity=identity, effective_policy_identity=effective_policy_identity,
        item_decisions=item_decisions, reasons=(code,),
    )


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
    source_scope_oid = _identity(value["source_scope_oid"], f"{label}.source_scope_oid")
    return {
        "repository": repository,
        "task_uid": task_uid,
        "pr_number": pr_number,
        "source_head_oid": source_head_oid,
        "source_scope_oid": source_scope_oid,
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


def _prior_target(value: Any) -> str | None:
    if value is None:
        return None
    return _identity(value, "target_snapshot.prior_assessed_target_oid")


def _review_locator(record: dict[str, Any]) -> dict[str, Any]:
    locator = record.get("evidence_locator")
    if (not isinstance(locator, dict) or set(locator) != {"kind", "id"}
            or not isinstance(locator["kind"], str) or not locator["kind"].strip()
            or not isinstance(locator["id"], (str, int)) or isinstance(locator["id"], bool)
            or not str(locator["id"]).strip()
            or (isinstance(locator["id"], int) and locator["id"] <= 0)):
        raise ValueError("review evidence locator is missing or malformed")
    return {"kind": locator["kind"], "id": locator["id"]}


def _test_locator(record: dict[str, Any], unit_id: str) -> dict[str, Any]:
    return {
        "kind": "github-check-artifact",
        "id": {
            "unit_id": unit_id,
            "run_id": record["run_id"],
            "run_attempt": record["run_attempt"],
            "check_app_id": _positive_numeric_id(record["check_app_id"], "test.check_app_id"),
            "check_run_id": record["check_run_id"],
            "artifact_id": record["artifact_id"],
        },
    }


def _inventory_locator(binding: dict[str, Any], label: str) -> dict[str, Any]:
    return {
        "kind": "trusted-planner-inventory",
        "id": {
            "label": label,
            "repository": binding["authority"]["repository"],
            "workflow_ref": binding["authority"]["workflow_ref"],
            "planner_authority_oid": binding["authority"]["planner_authority_oid"],
            "planner_config_sha256": binding["authority"]["planner_config_sha256"],
            "run_id": binding["producer"]["run_id"],
            "run_attempt": binding["producer"]["run_attempt"],
            "artifact_id": binding["producer"]["artifact_id"],
            "check_app_id": binding["producer"]["check_app_id"],
            "check_run_id": binding["producer"]["check_run_id"],
            "target_oid": binding["target_oid"],
            "target_tree_oid": binding["target_tree_oid"],
            "inventory_digest": binding["inventory_digest"],
        },
    }


def _item_decision(kind: str, item_id: str, disposition: str, reason: str,
                   locator: dict[str, Any] | None = None) -> dict[str, Any]:
    return {
        "kind": kind,
        "id": item_id,
        "disposition": disposition,
        "reason": reason,
        "evidence_locator": dict(locator) if locator is not None else None,
    }


def evaluate_evidence_applicability(
    source_plan: Any,
    evidence_set: Any,
    target_snapshot: Any,
    effective_policy: Any,
    *,
    trusted_source_inventory: Any = None,
    trusted_target_inventory: Any = None,
) -> ApplicabilityDecision:
    """Decide whether prior review/test evidence applies to a target snapshot.

    The live reader supplies both planner inventory bindings out of band. The
    target's assessed Q is separate from the commit/tree used to build its
    input inventory (M/T). Reuse remains disabled unless the v2 envelope and
    trusted effective policy both select the capability.
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
        identity: dict[str, Any] = {}
        policy_identity = None
        try:
            policy_identity = {
                "schema": EFFECTIVE_POLICY_IDENTITY_SCHEMA,
                "digest": effective_policy_digest(effective_policy),
            }
        except (TypeError, ValueError):
            pass
        try:
            source = _common_identity(source_plan, "source plan")
            target = _common_identity(target_snapshot, "target snapshot")
            identity = {
                "source_head_oid": source["source_head_oid"],
                "source_scope_oid": source["source_scope_oid"],
                "assessed_target_oid": _identity(
                    target_snapshot.get("target_oid"), "target_snapshot.target_oid",
                ),
                "prior_assessed_target_oid": _prior_target(
                    target_snapshot.get("prior_assessed_target_oid"),
                ),
            }
            if source != target:
                identity["identity_mismatch"] = True
        except (TypeError, ValueError):
            pass
        return _decision(
            _DISABLED, _DISABLED, _DISABLED, identity=identity,
            effective_policy_identity=policy_identity,
            reasons=("CAPABILITY_DISABLED",),
        )
    try:
        expected_check_app_id = _positive_numeric_id(
            effective_policy.get("check_app_id"), "effective_policy.check_app_id",
        )
        policy_identity = {
            "schema": EFFECTIVE_POLICY_IDENTITY_SCHEMA,
            "digest": effective_policy_digest(effective_policy),
        }
    except (TypeError, ValueError):
        return _blocked("EFFECTIVE_POLICY_INVALID")

    if not isinstance(source_plan, dict) or source_plan.get("schema") != REQUIRED_PLAN_V2_SCHEMA:
        return _blocked("UNSUPPORTED_PROTOCOL")
    if not isinstance(target_snapshot, dict):
        return _blocked("TARGET_SNAPSHOT_INVALID")
    if not isinstance(evidence_set, dict):
        return _blocked("EVIDENCE_SET_INVALID", effective_policy_identity=policy_identity)

    decision_identity: dict[str, Any] = {}
    try:
        source_identity = _common_identity(source_plan, "source plan")
        target_identity = _common_identity(target_snapshot, "target snapshot")
        assessed_target_oid = _identity(
            target_snapshot.get("target_oid"), "target_snapshot.target_oid",
        )
        prior_assessed_target_oid = _prior_target(
            target_snapshot.get("prior_assessed_target_oid"),
        )
        decision_identity = {
            "source_head_oid": source_identity["source_head_oid"],
            "source_scope_oid": source_identity["source_scope_oid"],
            "assessed_target_oid": assessed_target_oid,
            "prior_assessed_target_oid": prior_assessed_target_oid,
        }
        if source_identity != target_identity:
            return _blocked(
                "TASK_SOURCE_IDENTITY_MISMATCH", identity=decision_identity,
                effective_policy_identity=policy_identity,
            )

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
        source_inventory = validate_planner_inventory_binding(
            source_plan.get("planner_inventory_issuer"), trusted_source_inventory,
        )
        if source_inventory["unit_ids"] != sorted(source_units):
            raise ValueError("source required units disagree with trusted planner inventory")
        source_roles = set(_string_list(
            source_plan.get("required_review_roles"), "source_plan.required_review_roles",
        ))
        input_scope = validate_input_scope_snapshot(
            target_snapshot.get("input_scope"),
            trusted_planner_inventory=trusted_target_inventory,
        )
        input_scope_commit_oid = _identity(
            target_snapshot.get("input_scope_commit_oid"),
            "target_snapshot.input_scope_commit_oid",
        )
        input_scope_tree_oid = _identity(
            target_snapshot.get("input_scope_tree_oid"),
            "target_snapshot.input_scope_tree_oid",
        )
        if (input_scope["target_oid"] != input_scope_commit_oid
                or input_scope["target_tree_oid"] != input_scope_tree_oid):
            raise ValueError("target input scope commit/tree disagree with the explicit M/T identity")
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
        return _blocked(
            "APPLICABILITY_INPUT_INVALID",
            identity=decision_identity,
            effective_policy_identity=policy_identity,
        )

    reviews = evidence_set.get("reviews", [])
    tests = evidence_set.get("tests", [])
    if not isinstance(reviews, list) or not isinstance(tests, list):
        return _blocked(
            "EVIDENCE_SET_INVALID", identity=decision_identity,
            effective_policy_identity=policy_identity,
        )
    if any(not isinstance(item, dict) for item in reviews + tests):
        return _blocked(
            "EVIDENCE_RECORD_INVALID", identity=decision_identity,
            effective_policy_identity=policy_identity,
        )

    corpus_result = aggregate_product_corpus_results(
        input_scope, tests, trusted_planner_inventory=trusted_target_inventory,
    )
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
    item_decisions: list[dict[str, Any]] = []
    evidence_locators: list[dict[str, Any]] = [
        _inventory_locator(source_inventory, "source"),
        _inventory_locator(trusted_target_inventory, "target"),
    ]

    # A changed review applicability digest starts a new review epoch.  C0
    # deliberately does not try to reconstruct semantic review closure.
    for role in target_roles:
        if role not in source_roles or source_review_digest != target_review_digest:
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "revalidate",
                "ROLE_OR_REVIEW_APPLICABILITY_CHANGED",
            ))
            continue
        matching = [item for item in reviews if item.get("role_id") == role]
        if not matching:
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "revalidate", "REVIEW_EVIDENCE_MISSING",
            ))
            continue
        if len(matching) != 1:
            blockers.append("REVIEW_EVIDENCE_AMBIGUOUS")
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "blocked", "REVIEW_EVIDENCE_AMBIGUOUS",
            ))
            continue
        record = matching[0]
        try:
            if not _matches_common_identity(record, source_identity, "review evidence"):
                blockers.append("REVIEW_IDENTITY_MISMATCH")
                required_roles.append(role)
                item_decisions.append(_item_decision(
                    "review", role, "blocked", "REVIEW_IDENTITY_MISMATCH",
                ))
                continue
            record_digest = _digest(
                record.get("applicability_digest"), "review.applicability_digest",
            )
            locator = _review_locator(record)
        except (TypeError, ValueError):
            blockers.append("REVIEW_PROVENANCE_INVALID")
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "blocked", "REVIEW_PROVENANCE_INVALID",
            ))
            continue
        evidence_locators.append(locator)
        status = record.get("status")
        if record_digest != target_review_digest:
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "revalidate", "REVIEW_APPLICABILITY_CHANGED", locator,
            ))
        elif status == "passed":
            item_decisions.append(_item_decision(
                "review", role, "reusable", "REVIEW_EVIDENCE_MATCHED", locator,
            ))
        elif isinstance(status, str) and status in {"failed", "blocked", "pending"}:
            blockers.append("REVIEW_EVIDENCE_NOT_ACCEPTED")
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "blocked", "REVIEW_EVIDENCE_NOT_ACCEPTED", locator,
            ))
        else:
            blockers.append("REVIEW_EVIDENCE_STATUS_UNSUPPORTED")
            required_roles.append(role)
            item_decisions.append(_item_decision(
                "review", role, "blocked", "REVIEW_EVIDENCE_STATUS_UNSUPPORTED", locator,
            ))

    for unit in target_units:
        if unit not in source_units:
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "revalidate", "TEST_UNIT_ADDED",
            ))
            continue
        matching = [item for item in tests if item.get("unit_id") == unit]
        if not matching:
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "revalidate", "TEST_EVIDENCE_MISSING",
            ))
            continue
        if len(matching) != 1:
            blockers.append("TEST_EVIDENCE_AMBIGUOUS")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_EVIDENCE_AMBIGUOUS",
            ))
            continue
        record = matching[0]
        try:
            if not _matches_common_identity(record, source_identity, "test evidence"):
                blockers.append("TEST_IDENTITY_MISMATCH")
                required_units.append(unit)
                item_decisions.append(_item_decision(
                    "test", unit, "blocked", "TEST_IDENTITY_MISMATCH",
                ))
                continue
            record_digest = _digest(record.get("input_digest"), "test.input_digest")
            inventory_digest = _digest(
                record.get("inventory_digest"), "test.inventory_digest",
            )
            check_app_id = _positive_numeric_id(record.get("check_app_id"), "test.check_app_id")
            _positive_int(record.get("run_id"), "test.run_id")
            _positive_int(record.get("run_attempt"), "test.run_attempt")
            _positive_int(record.get("check_run_id"), "test.check_run_id")
            _positive_int(record.get("artifact_id"), "test.artifact_id")
            locator = _test_locator(record, unit)
        except (TypeError, ValueError):
            blockers.append("TEST_PROVENANCE_INVALID")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_PROVENANCE_INVALID",
            ))
            continue
        evidence_locators.append(locator)
        if inventory_digest != source_inventory["inventory_digest"]:
            blockers.append("TEST_INVENTORY_IDENTITY_MISMATCH")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_INVENTORY_IDENTITY_MISMATCH", locator,
            ))
            continue
        if check_app_id != expected_check_app_id:
            blockers.append("TEST_APP_IDENTITY_MISMATCH")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_APP_IDENTITY_MISMATCH", locator,
            ))
            continue
        status = record.get("status")
        if record.get("effective_policy_identity") != policy_identity:
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "revalidate", "TEST_EFFECTIVE_POLICY_CHANGED", locator,
            ))
        elif record_digest != fingerprints.get(unit):
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "revalidate", "TEST_INPUT_FINGERPRINT_CHANGED", locator,
            ))
        elif status == "passed":
            reused_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "reusable", "TEST_EVIDENCE_MATCHED", locator,
            ))
        elif isinstance(status, str) and status in {"failed", "blocked", "pending"}:
            blockers.append("TEST_EVIDENCE_NOT_ACCEPTED")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_EVIDENCE_NOT_ACCEPTED", locator,
            ))
        else:
            blockers.append("TEST_EVIDENCE_STATUS_UNSUPPORTED")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_EVIDENCE_STATUS_UNSUPPORTED", locator,
            ))

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
        identity=decision_identity,
        effective_policy_identity=policy_identity,
        evidence_locators=tuple(evidence_locators),
        item_decisions=tuple(item_decisions),
        reasons=tuple(
            [item["reason"] for item in item_decisions] + blockers
        ),
    )
