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
from ci_input_scope import (
    aggregate_product_corpus_results,
    planner_inventory_digest,
    validate_input_scope_snapshot,
    validate_planner_inventory_binding,
    validate_target_observation_binding,
)
from ci_required_artifact_v2 import (
    plan_artifact_name,
    request_key_for_identity,
    result_artifact_name,
    validate_request_identity,
)
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
_UNIT_POLICY_SCHEMA = "oasis7-required-unit-policy/v1"
_ACTIVE_REUSE_STATE = "active"
_INACTIVE_REUSE_STATES = frozenset({
    "disabled",
    "disabled-pending-independent-activation",
})
_TRUSTED_SOURCE_ATTEMPT_SCHEMA = "oasis7-ci-trusted-source-attempt/v1"


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


def _validate_trusted_source_attempt(
    value: Any, source_plan: dict[str, Any], source_inventory: dict[str, Any],
    source_units: set[str],
) -> dict[str, Any]:
    """Bind source test claims to the reader's exact live attempt locators."""
    fields = {
        "schema", "request_key", "workflow_run_id", "run_attempt", "check_app_id",
        "check_run_id", "job_id", "job_name", "plan_artifact_id",
        "plan_artifact_name", "result_artifacts",
    }
    if (not isinstance(value, dict) or set(value) != fields
            or value.get("schema") != _TRUSTED_SOURCE_ATTEMPT_SCHEMA):
        raise ValueError("trusted source attempt envelope is invalid")
    request_key = _digest(value.get("request_key"), "trusted_source_attempt.request_key")
    if request_key != source_plan.get("request_key"):
        raise ValueError("trusted source attempt request key mismatch")
    run_id = _positive_int(value.get("workflow_run_id"), "trusted_source_attempt.workflow_run_id")
    attempt = _positive_int(value.get("run_attempt"), "trusted_source_attempt.run_attempt")
    app_id = _positive_int(value.get("check_app_id"), "trusted_source_attempt.check_app_id")
    check_run_id = _positive_int(value.get("check_run_id"), "trusted_source_attempt.check_run_id")
    job_id = _positive_int(value.get("job_id"), "trusted_source_attempt.job_id")
    job_name = value.get("job_name")
    if not isinstance(job_name, str) or not job_name.strip():
        raise ValueError("trusted source attempt job name is invalid")
    plan_artifact_id = _positive_int(
        value.get("plan_artifact_id"), "trusted_source_attempt.plan_artifact_id",
    )
    plan_artifact = value.get("plan_artifact_name")
    if not isinstance(plan_artifact, str) or not plan_artifact:
        raise ValueError("trusted source attempt plan artifact name is invalid")
    if (type(source_plan.get("workflow_run_id")) is not int
            or source_plan.get("workflow_run_id") != run_id
            or type(source_plan.get("run_attempt")) is not int
            or source_plan.get("run_attempt") != attempt
            or type(source_plan.get("check_app_id")) is not int
            or source_plan.get("check_app_id") != app_id
            or type(source_plan.get("check_run_id")) is not int
            or source_plan.get("check_run_id") != check_run_id
            or type(source_plan.get("job_id")) is not int
            or source_plan.get("job_id") != job_id
            or source_plan.get("job_name") != job_name
            or source_plan.get("check_name") != "required-gate"
            or job_name != "required-gate"):
        raise ValueError("trusted source attempt differs from the source plan execution identity")
    producer = source_inventory.get("producer")
    if (not isinstance(producer, dict)
            or producer.get("run_id") != run_id
            or producer.get("run_attempt") != attempt
            or producer.get("check_app_id") != app_id
            or producer.get("check_run_id") != check_run_id
            or producer.get("artifact_id") != plan_artifact_id):
        raise ValueError("trusted source attempt differs from live planner inventory readback")
    if plan_artifact != plan_artifact_name(run_id, attempt):
        raise ValueError("trusted source attempt plan artifact name is invalid")

    raw_results = value.get("result_artifacts")
    if not isinstance(raw_results, list):
        raise ValueError("trusted source attempt result artifact set is invalid")
    results: list[dict[str, Any]] = []
    for artifact in raw_results:
        if not isinstance(artifact, dict) or set(artifact) != {"unit_id", "artifact_id", "name"}:
            raise ValueError("trusted source attempt result locator is invalid")
        unit_id = artifact.get("unit_id")
        artifact_id = _positive_int(
            artifact.get("artifact_id"), "trusted_source_attempt.result_artifact_id",
        )
        name = artifact.get("name")
        if not isinstance(unit_id, str) or not unit_id or not isinstance(name, str) or not name:
            raise ValueError("trusted source attempt result locator is invalid")
        if name != result_artifact_name(run_id, attempt, unit_id):
            raise ValueError("trusted source attempt result artifact name is invalid")
        results.append({"unit_id": unit_id, "artifact_id": artifact_id, "name": name})
    unit_ids = [item["unit_id"] for item in results]
    artifact_ids = [item["artifact_id"] for item in results]
    names = [item["name"] for item in results]
    if (unit_ids != sorted(source_units) or unit_ids != sorted(set(unit_ids))
            or len(artifact_ids) != len(set(artifact_ids))
            or plan_artifact_id in artifact_ids
            or len(names) != len(set(names))):
        raise ValueError("trusted source attempt result locators do not cover the exact unit inventory")
    return {
        "schema": _TRUSTED_SOURCE_ATTEMPT_SCHEMA,
        "request_key": request_key,
        "workflow_run_id": run_id,
        "run_attempt": attempt,
        "check_app_id": app_id,
        "check_run_id": check_run_id,
        "job_id": job_id,
        "job_name": job_name,
        "plan_artifact_id": plan_artifact_id,
        "plan_artifact_name": plan_artifact,
        "result_artifacts": results,
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


def _target_observation_locator(binding: dict[str, Any]) -> dict[str, Any]:
    """Locate local fresh-Q planner evidence without inventing a CI artifact."""
    authority = binding["authority"]
    invocation = binding["planner_invocation"]
    return {
        "kind": "trusted-local-target-observation",
        "id": {
            "repository": binding["repository"],
            "workflow_ref": authority["workflow_ref"],
            "planner_authority_oid": authority["planner_authority_oid"],
            "planner_config_sha256": authority["planner_config_sha256"],
            "planner_invocation_digest": invocation["digest"],
            "task_uid": binding["task_uid"],
            "pr_number": binding["pr_number"],
            "source_head_oid": binding["source_head_oid"],
            "source_scope_oid": binding["source_scope_oid"],
            "assessed_target_oid": binding["assessed_target_oid"],
            "input_scope_commit_oid": binding["input_scope_commit_oid"],
            "input_scope_tree_oid": binding["input_scope_tree_oid"],
            "effective_policy_identity": binding["effective_policy_identity"],
            "unit_ids": binding["unit_ids"],
            "inventory_digest": binding["inventory_digest"],
        },
    }


def _bound_unit_policies(
    inventory_record: Any,
    trusted_inventory: dict[str, Any],
    expected_units: set[str],
    label: str,
) -> dict[str, dict[str, Any]]:
    """Read per-unit reuse policy only from a digest-bound full inventory."""
    if not isinstance(inventory_record, dict):
        raise ValueError(f"{label} inventory record is missing")
    unit_specs = inventory_record.get("unit_specs")
    product_corpus = inventory_record.get("product_corpus")
    digest = planner_inventory_digest(
        unit_specs,
        product_corpus,
        trusted_inventory.get("target_oid", trusted_inventory.get("input_scope_commit_oid")),
        trusted_inventory.get("target_tree_oid", trusted_inventory.get("input_scope_tree_oid")),
    )
    if digest != trusted_inventory["inventory_digest"]:
        raise ValueError(f"{label} unit policies are not bound to the trusted inventory")
    if not isinstance(unit_specs, list):
        raise ValueError(f"{label} unit specs are malformed")
    unit_ids = [spec.get("unit_id") if isinstance(spec, dict) else None for spec in unit_specs]
    if (any(not isinstance(unit_id, str) or not unit_id for unit_id in unit_ids)
            or len(unit_ids) != len(set(unit_ids))
            or set(unit_ids) != expected_units):
        raise ValueError(f"{label} unit policies do not cover the exact trusted unit set")

    policies: dict[str, dict[str, Any]] = {}
    for spec in unit_specs:
        unit_id = spec["unit_id"]
        policy = spec.get("applicable_policy")
        environment = spec.get("environment_contract")
        if (not isinstance(policy, dict)
                or policy.get("schema") != _UNIT_POLICY_SCHEMA
                or not isinstance(policy.get("reuse_state"), str)
                or not policy["reuse_state"]
                or type(policy.get("reuse_eligible")) is not bool):
            raise ValueError(f"{label} unit {unit_id} has an invalid reuse policy")
        if not isinstance(environment, dict) or type(environment.get("reuse_eligible")) is not bool:
            raise ValueError(f"{label} unit {unit_id} has an invalid reuse environment policy")
        policies[unit_id] = {
            "reuse_state": policy["reuse_state"],
            "reuse_eligible": policy["reuse_eligible"],
            "environment_reuse_eligible": environment["reuse_eligible"],
        }
    return policies


def _reuse_policy_disposition(
    source_policy: dict[str, Any], target_policy: dict[str, Any],
) -> tuple[str, str] | None:
    recognized_states = {
        _ACTIVE_REUSE_STATE, *_INACTIVE_REUSE_STATES,
    }
    if (source_policy["reuse_state"] not in recognized_states
            or target_policy["reuse_state"] not in recognized_states):
        return _BLOCKED, "TEST_REUSE_POLICY_STATE_UNRECOGNIZED"
    if (source_policy["reuse_state"] in _INACTIVE_REUSE_STATES
            or target_policy["reuse_state"] in _INACTIVE_REUSE_STATES):
        return _REVALIDATE, "TEST_REUSE_POLICY_DISABLED"
    if not source_policy["reuse_eligible"] or not target_policy["reuse_eligible"]:
        return _REVALIDATE, "TEST_REUSE_POLICY_INELIGIBLE"
    if (not source_policy["environment_reuse_eligible"]
            or not target_policy["environment_reuse_eligible"]):
        return _REVALIDATE, "TEST_REUSE_ENVIRONMENT_INELIGIBLE"
    return None


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
    trusted_source_attempt: Any = None,
    trusted_target_observation: Any = None,
) -> ApplicabilityDecision:
    """Decide whether prior review/test evidence applies to a target snapshot.

    The reader supplies the source execution inventory and local target
    observation out of band. Source execution still requires live R/A/check/
    artifact readback. Fresh-Q observation is a distinct W-replayed local
    binding without artifact IDs. The target's assessed Q is separate from
    the commit/tree used to build its input inventory (M/T). Reuse remains
    disabled unless the v2 envelope and
    trusted effective policy both select the capability. Both plan records
    carry full `unit_specs` and `product_corpus` data; their canonical digest
    must match the respective live binding, and a test unit is reusable only
    when both its source and target policy explicitly mark reuse active and
    eligible.
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

        request_identity = validate_request_identity(source_plan.get("request_identity"))
        if source_plan.get("request_key") != request_key_for_identity(request_identity):
            raise ValueError("source plan request key differs from its request identity")
        for field in (
            "repository", "task_uid", "pr_number", "bootstrap_epoch", "source_head_oid",
            "source_projection_digest", "executor_contract_digest",
        ):
            if source_plan.get(field) != request_identity[field]:
                raise ValueError(f"source plan request identity differs at {field}")
        source_units_for_request = _string_list(
            source_plan.get("required_test_units"), "source_plan.required_test_units",
        )
        if source_units_for_request != tuple(request_identity["unit_ids"]):
            raise ValueError("source plan request identity differs at required test units")
        if source_plan.get("input_fingerprints") != request_identity["input_fingerprints"]:
            raise ValueError("source plan request identity differs at input fingerprints")
        if request_identity["applicability_mode"] == "snapshot_exact":
            exact_target_oid = request_identity["snapshot_target_oid"]
            tested_target_oid = _identity(
                source_plan.get("tested_commit_oid"), "source_plan.tested_commit_oid",
            )
            if tested_target_oid != exact_target_oid or assessed_target_oid != exact_target_oid:
                return _blocked(
                    "SNAPSHOT_EXACT_TARGET_MISMATCH", identity=decision_identity,
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
        try:
            source_attempt = _validate_trusted_source_attempt(
                trusted_source_attempt, source_plan, source_inventory, source_units,
            )
        except (TypeError, ValueError, KeyError):
            return _blocked(
                "SOURCE_ATTEMPT_BINDING_INVALID", identity=decision_identity,
                effective_policy_identity=policy_identity,
            )
        source_unit_policies = _bound_unit_policies(
            source_plan, source_inventory, source_units, "source",
        )
        source_roles = set(_string_list(
            source_plan.get("required_review_roles"), "source_plan.required_review_roles",
        ))
        input_scope = validate_input_scope_snapshot(
            target_snapshot.get("input_scope"),
            trusted_target_observation=trusted_target_observation,
        )
        target_observation = validate_target_observation_binding(
            input_scope.get("target_observation"), trusted_target_observation,
        )
        source_base_oid = _identity(
            source_plan.get("integration_base_oid"), "source_plan.integration_base_oid",
        )
        if (target_observation["planner_invocation"]["base_ref"] != source_base_oid
                or target_observation["planner_invocation"]["impact_projection_sha256"]
                   != source_plan.get("source_projection_digest")):
            raise ValueError("target planner invocation differs from immutable source B/projection")
        if target_snapshot.get("product_corpus") != input_scope["product_corpus"]:
            raise ValueError("target product corpus disagrees with its input-scope snapshot")
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
        target_pr_number = target_snapshot.get("pr_number")
        if (target_observation["repository"] != target_identity["repository"]
                or target_observation["task_uid"] != target_identity["task_uid"]
                or target_observation["pr_number"] != target_pr_number
                or target_observation["source_head_oid"] != target_identity["source_head_oid"]
                or target_observation["source_scope_oid"] != target_identity["source_scope_oid"]
                or target_observation["assessed_target_oid"] != assessed_target_oid
                or target_observation["input_scope_commit_oid"] != input_scope_commit_oid
                or target_observation["input_scope_tree_oid"] != input_scope_tree_oid
                or target_observation["effective_policy_identity"] != policy_identity):
            raise ValueError("trusted local target observation differs from Q, H/S, M/T, task or policy")
        decision_identity.update({
            "input_scope_commit_oid": input_scope_commit_oid,
            "input_scope_tree_oid": input_scope_tree_oid,
            "planner_authority_oid": target_observation["authority"]["planner_authority_oid"],
            "planner_config_sha256": target_observation["authority"]["planner_config_sha256"],
            "planner_invocation_digest": target_observation["planner_invocation"]["digest"],
            "target_inventory_digest": target_observation["inventory_digest"],
        })
        target_units = tuple(input_scope["required_test_units"])
        target_unit_policies = _bound_unit_policies(
            target_snapshot,
            target_observation,
            set(target_units),
            "target",
        )
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
        input_scope, tests, trusted_target_observation=trusted_target_observation,
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
        {"kind": "trusted-source-attempt", "id": source_attempt},
        _target_observation_locator(target_observation),
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
        trusted_result = next(
            item for item in source_attempt["result_artifacts"]
            if item["unit_id"] == unit
        )
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
            if (record.get("run_id") != source_attempt["workflow_run_id"]
                    or record.get("run_attempt") != source_attempt["run_attempt"]
                    or _positive_numeric_id(record.get("check_app_id"), "test.check_app_id")
                       != str(source_attempt["check_app_id"])
                    or record.get("check_run_id") != source_attempt["check_run_id"]
                    or record.get("artifact_id") != trusted_result["artifact_id"]
                    or record.get("artifact_name") != trusted_result["name"]):
                raise ValueError("test evidence differs from trusted source attempt locator")
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
            continue
        if record_digest != fingerprints.get(unit):
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "revalidate", "TEST_INPUT_FINGERPRINT_CHANGED", locator,
            ))
            continue
        if isinstance(status, str) and status in {"failed", "blocked", "pending"}:
            blockers.append("TEST_EVIDENCE_NOT_ACCEPTED")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_EVIDENCE_NOT_ACCEPTED", locator,
            ))
            continue
        if status != "passed":
            blockers.append("TEST_EVIDENCE_STATUS_UNSUPPORTED")
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, "blocked", "TEST_EVIDENCE_STATUS_UNSUPPORTED", locator,
            ))
            continue
        policy_decision = _reuse_policy_disposition(
            source_unit_policies[unit], target_unit_policies[unit],
        )
        if policy_decision is not None:
            disposition, reason = policy_decision
            if disposition == _BLOCKED:
                blockers.append(reason)
            required_units.append(unit)
            item_decisions.append(_item_decision(
                "test", unit, disposition, reason, locator,
            ))
            continue
        reused_units.append(unit)
        item_decisions.append(_item_decision(
            "test", unit, "reusable", "TEST_EVIDENCE_MATCHED", locator,
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
