#!/usr/bin/env python3
"""Stable review identity for a trusted CI-ready receipt."""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
import subprocess
import sys
from typing import Any

AUTHORITY_FIELDS = (
    "receipt_type", "issuer", "repository", "task_uid", "task_issue_number",
    "pr_number", "base_oid", "head_oid", "check_name", "check_app_id",
    "check_run_id", "planner_digest", "planner_config_sha256",
    "run_rust_baseline", "conclusion",
)

SOURCE_REVIEW_SCHEMA = "oasis7-review-plan/v2"
SOURCE_REVIEW_FIELDS = (
    "task_uid", "bootstrap_epoch", "repository", "pr_number", "source_head_oid",
    "source_scope_oid", "changed_paths_digest", "ordered_role_ids",
    "role_contract_digest", "review_policy_digest", "input_contract_digest",
)
# These fields describe the professional-review applicability projection.  They
# are deliberately separate from integration execution provenance: a target
# advance can change the integration base while leaving this projection intact.
REVIEW_APPLICABILITY_FIELDS = (
    "changed_paths_digest", "ordered_role_ids", "role_contract_digest",
    "review_policy_digest", "input_contract_digest",
)
SHADOW_REVIEW_SCHEMA = "oasis7-review-reuse-shadow/v1"
INTEGRATION_CI_FIELDS = (
    "repository", "task_uid", "pr_number", "source_head_oid", "integration_base_oid",
    "workflow_ref", "workflow_sha", "request_id", "request_created_at", "run_id",
    "run_attempt", "check_app_id", "check_run_id", "planner_digest", "tested_tree_oid",
    "conclusion",
)
# A new dispatch/run/check identity is expected during target revalidation. These
# fields are the complete trusted execution authority that must remain equivalent
# before a source review can be reused.
INTEGRATION_REUSE_FIELDS = (
    "repository", "task_uid", "pr_number", "source_head_oid",
    "workflow_ref", "workflow_sha", "check_app_id", "planner_digest", "tested_tree_oid",
)
# Projection metadata is the source-review boundary.  The complete integration
# identity above remains in every receipt for execution audit, but its
# run-specific workflow/planner/tree values must not silently invalidate a
# source review whose independently verified applicability is unchanged.
INTEGRATION_AUTHORITY_REUSE_FIELDS = tuple(
    field for field in INTEGRATION_REUSE_FIELDS
    if field not in {"workflow_sha", "planner_digest", "tested_tree_oid"}
)
PROJECTION_BINDING_FIELDS = (
    "impact_projection_schema", "impact_projection_digest",
    "impact_projection_planner_digest",
)
PROJECTION_SCHEMA = "oasis7-workflow-impact-projection/v2"


def _require_oid(value: Any, field: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{40,64}", value):
        raise ValueError(f"{field} must be a lowercase commit/tree object id")
    return value


def _require_digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{64}", value):
        raise ValueError(f"{field} must be a lowercase SHA-256 digest")
    return value


def source_review_identity(
    *, task_uid: str, bootstrap_epoch: int, repository: str, pr_number: int,
    source_head_oid: str, source_scope_oid: str, changed_paths_digest: str,
    ordered_role_ids: list[str], role_contract_digest: str,
    review_policy_digest: str, input_contract_digest: str,
) -> dict[str, Any]:
    """Build the immutable v2 professional-review identity.

    The source scope is deliberately separate from the CI integration base. The
    caller owns the role/policy/input digests and must provide the exact values
    used to construct the review packets.
    """
    if not isinstance(task_uid, str) or not re.fullmatch(r"task_[0-9a-f]{32}", task_uid):
        raise ValueError("task_uid must be a canonical task UID")
    if type(bootstrap_epoch) is not int or bootstrap_epoch <= 0:
        raise ValueError("bootstrap_epoch must be a positive integer")
    if not isinstance(repository, str) or not repository.strip():
        raise ValueError("repository must be non-empty")
    if type(pr_number) is not int or pr_number <= 0:
        raise ValueError("pr_number must be a positive integer")
    _require_oid(source_head_oid, "source_head_oid")
    _require_oid(source_scope_oid, "source_scope_oid")
    for field, value in (
        ("changed_paths_digest", changed_paths_digest),
        ("role_contract_digest", role_contract_digest),
        ("review_policy_digest", review_policy_digest),
        ("input_contract_digest", input_contract_digest),
    ):
        _require_digest(value, field)
    if (not isinstance(ordered_role_ids, list) or not ordered_role_ids
            or any(not isinstance(role, str) or not role.strip() for role in ordered_role_ids)
            or len(set(ordered_role_ids)) != len(ordered_role_ids)):
        raise ValueError("ordered_role_ids must be a non-empty unique role list")
    return {
        "task_uid": task_uid,
        "bootstrap_epoch": bootstrap_epoch,
        "repository": repository,
        "pr_number": pr_number,
        "source_head_oid": source_head_oid,
        "source_scope_oid": source_scope_oid,
        "changed_paths_digest": changed_paths_digest,
        "ordered_role_ids": list(ordered_role_ids),
        "role_contract_digest": role_contract_digest,
        "review_policy_digest": review_policy_digest,
        "input_contract_digest": input_contract_digest,
    }


def _validate_source_identity(identity: Any) -> dict[str, Any]:
    if not isinstance(identity, dict):
        raise ValueError("source review identity must be an object")
    missing = [field for field in SOURCE_REVIEW_FIELDS if field not in identity]
    if missing:
        raise ValueError("source review identity is missing: " + ",".join(missing))
    return source_review_identity(**{field: identity[field] for field in SOURCE_REVIEW_FIELDS})


def source_review_digest(identity: dict[str, Any]) -> str:
    canonical = json.dumps(_validate_source_identity(identity), sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(canonical).hexdigest()


def review_applicability_identity(source_identity: dict[str, Any]) -> dict[str, Any]:
    """Project source inputs that determine professional-review applicability.

    This projection intentionally excludes integration execution fields such as
    ``workflow_sha`` and ``tested_tree_oid``.  Those fields remain mandatory in
    the complete integration identity and the shadow audit output, but they are
    not evidence that the professional review's impact closure is unchanged.
    """
    source = _validate_source_identity(source_identity)
    return {field: source[field] for field in REVIEW_APPLICABILITY_FIELDS}


def _validate_review_applicability_identity(identity: Any) -> dict[str, Any]:
    if not isinstance(identity, dict):
        raise ValueError("review applicability identity must be an object")
    missing = [field for field in REVIEW_APPLICABILITY_FIELDS if field not in identity]
    if missing:
        raise ValueError("review applicability identity is missing: " + ",".join(missing))
    # Unknown projection members are unsafe: they may represent an unverified
    # impact input and must not be silently dropped from the comparison.
    unknown = sorted(set(identity) - set(REVIEW_APPLICABILITY_FIELDS))
    if unknown:
        raise ValueError("review applicability identity has unknown fields: " + ",".join(unknown))
    result = {field: identity[field] for field in REVIEW_APPLICABILITY_FIELDS}
    _require_digest(result["changed_paths_digest"], "changed_paths_digest")
    _require_digest(result["role_contract_digest"], "role_contract_digest")
    _require_digest(result["review_policy_digest"], "review_policy_digest")
    _require_digest(result["input_contract_digest"], "input_contract_digest")
    if (not isinstance(result["ordered_role_ids"], list)
            or not result["ordered_role_ids"]
            or any(not isinstance(role, str) or not role.strip()
                   for role in result["ordered_role_ids"])
            or len(set(result["ordered_role_ids"])) != len(result["ordered_role_ids"])):
        raise ValueError("ordered_role_ids must be a non-empty unique role list")
    return result


def review_applicability_digest(identity: dict[str, Any]) -> str:
    canonical = json.dumps(
        _validate_review_applicability_identity(identity),
        sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()


def validate_source_review_epoch(
    plan: dict[str, Any], *, root: pathlib.Path | str, task_uid: str,
) -> int:
    """Bind a v2 source review to the live canonical bootstrap epoch.

    The snapshot is only a locator for the request identity.  The repository's
    bootstrap validator rechecks its digest and immutable task identity against
    the current mapping, so a deleted, stale, or cross-generation snapshot
    cannot be accepted merely because it contains a plausible epoch integer.
    """
    if plan.get("schema") != SOURCE_REVIEW_SCHEMA:
        raise ValueError("canonical bootstrap epoch validation requires a v2 review plan")
    source = _validate_source_identity(plan.get("source_review_identity"))
    if source["task_uid"] != task_uid:
        raise ValueError("v2 source review task UID does not match the closeout task")

    root_path = pathlib.Path(root).resolve()
    snapshot_path = root_path / ".pm" / "scratch" / task_uid / "bootstrap-task-snapshot.json"
    try:
        snapshot = json.loads(snapshot_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"canonical bootstrap snapshot cannot be read: {exc}") from exc
    if not isinstance(snapshot, dict):
        raise ValueError("canonical bootstrap snapshot must be an object")
    request = snapshot.get("request")
    request_identity = request.get("identity") if isinstance(request, dict) else None
    if not isinstance(request_identity, str) or not request_identity:
        raise ValueError("canonical bootstrap snapshot request identity is missing")

    validator = pathlib.Path(__file__).with_name("bootstrap-task-snapshot.py")
    result = subprocess.run(
        [
            sys.executable,
            str(validator),
            "validate-epoch-identity",
            "--repo-root",
            str(root_path),
            "--task-uid",
            task_uid,
            "--request-identity",
            request_identity,
            "--snapshot",
            str(snapshot_path),
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "validator rejected snapshot"
        raise ValueError(f"canonical bootstrap snapshot validation failed: {detail}")

    snapshot_task = snapshot.get("task")
    snapshot_epoch = snapshot_task.get("bootstrap_epoch") if isinstance(snapshot_task, dict) else None
    if type(snapshot_epoch) is not int or snapshot_epoch < 1:
        raise ValueError("canonical bootstrap snapshot has an invalid bootstrap epoch")
    if source["bootstrap_epoch"] != snapshot_epoch:
        raise ValueError("v2 source review bootstrap epoch does not match canonical bootstrap snapshot")
    return snapshot_epoch


def integration_ci_identity(receipt: dict[str, Any]) -> dict[str, Any]:
    """Normalize a v2 trusted integration receipt into its identity object."""
    if not isinstance(receipt, dict):
        raise ValueError("integration CI receipt must be an object")
    source_head_oid = receipt.get("source_head_oid", receipt.get("head_oid"))
    integration_base_oid = receipt.get("integration_base_oid", receipt.get("base_oid"))
    run_id = receipt.get("run_id", receipt.get("integration_run_id"))
    values = {
        "repository": receipt.get("repository"),
        "task_uid": receipt.get("task_uid"),
        "pr_number": receipt.get("pr_number"),
        "source_head_oid": source_head_oid,
        "integration_base_oid": integration_base_oid,
        "workflow_ref": receipt.get("workflow_ref"),
        "workflow_sha": receipt.get("workflow_sha"),
        "request_id": receipt.get("request_id"),
        "request_created_at": receipt.get("request_created_at"),
        "run_id": run_id,
        "run_attempt": receipt.get("run_attempt"),
        "check_app_id": receipt.get("check_app_id"),
        "check_run_id": receipt.get("check_run_id"),
        "planner_digest": receipt.get("planner_digest"),
        "tested_tree_oid": receipt.get("tested_tree_oid"),
        "conclusion": receipt.get("conclusion"),
    }
    missing = [field for field in INTEGRATION_CI_FIELDS if values.get(field) is None]
    if missing:
        raise ValueError("integration CI identity is missing: " + ",".join(missing))
    if not isinstance(values["repository"], str) or not values["repository"].strip():
        raise ValueError("integration CI repository is invalid")
    if not isinstance(values["task_uid"], str) or not re.fullmatch(r"task_[0-9a-f]{32}", values["task_uid"]):
        raise ValueError("integration CI task UID is invalid")
    if type(values["pr_number"]) is not int or values["pr_number"] <= 0:
        raise ValueError("integration CI PR number is invalid")
    _require_oid(values["source_head_oid"], "source_head_oid")
    _require_oid(values["integration_base_oid"], "integration_base_oid")
    if not isinstance(values["workflow_ref"], str) or not values["workflow_ref"].strip():
        raise ValueError("workflow_ref is invalid")
    _require_oid(values["workflow_sha"], "workflow_sha")
    for field in ("request_id", "run_id", "check_app_id", "check_run_id"):
        if isinstance(values[field], bool) or not isinstance(values[field], (int, str)) or not str(values[field]).strip():
            raise ValueError(f"{field} is invalid")
    if not isinstance(values["request_created_at"], str) or not values["request_created_at"].strip():
        raise ValueError("request_created_at is invalid")
    if type(values["run_attempt"]) is not int or values["run_attempt"] < 1:
        raise ValueError("run_attempt is invalid")
    _require_digest(values["planner_digest"], "planner_digest")
    _require_oid(values["tested_tree_oid"], "tested_tree_oid")
    if not isinstance(values["conclusion"], str) or not values["conclusion"].strip():
        raise ValueError("conclusion is invalid")
    return values


def has_live_integration_attestation(receipt: dict[str, Any]) -> bool:
    return (
        isinstance(receipt, dict)
        and receipt.get("receipt_type") == "oasis7_ci_ready_receipt"
        and receipt.get("issuer") == "github_live_query"
        and receipt.get("live_validation") == "ci-ready-receipt-live"
        and receipt.get("trusted_integration_artifact") is True
    )


def _require_projection_digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", value):
        raise ValueError(f"{field} must be a prefixed SHA-256 digest")
    return value


def _validate_projection_binding(
    plan: dict[str, Any], receipt: dict[str, Any],
) -> None:
    """Bind a trusted receipt's selected obligations to the source-review plan.

    The receipt's complete planner/run identity is execution evidence.  The
    projection metadata is the smaller source-review applicability boundary and
    must therefore be present and byte-for-byte equal in both records,
    including for a source-only plan with no accepted integration identity.
    """
    if not isinstance(receipt, dict):
        raise ValueError("integration CI receipt must be an object")
    if plan.get("impact_projection_schema") != PROJECTION_SCHEMA:
        raise ValueError("review plan impact projection schema is unsupported")
    expected = {}
    for field in PROJECTION_BINDING_FIELDS:
        expected[field] = (
            PROJECTION_SCHEMA
            if field == "impact_projection_schema"
            else _require_projection_digest(plan.get(field), field)
        )
    for field, value in expected.items():
        if receipt.get(field) != value:
            raise ValueError(f"integration CI receipt {field} does not bind review plan")


def integration_ci_digest(identity: dict[str, Any]) -> str:
    canonical = json.dumps(
        {field: integration_ci_identity(identity)[field] for field in INTEGRATION_CI_FIELDS},
        sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()


def _shadow_result(
    *, reason: str, integration_provenance: str,
    professional_review_applicability: str = "requires_full_review",
    decision: str = "requires_full_review", accepted: dict[str, Any] | None = None,
    latest: dict[str, Any] | None = None, fresh_trusted_ci: bool = False,
) -> dict[str, Any]:
    """Build the non-authoritative, machine-readable shadow result."""
    result: dict[str, Any] = {
        "schema": SHADOW_REVIEW_SCHEMA,
        "authoritative": False,
        "decision": decision,
        "reason": reason,
        "integration_provenance": integration_provenance,
        "professional_review_applicability": professional_review_applicability,
        "fresh_trusted_integration_ci": fresh_trusted_ci,
        # Keep the complete execution identity visible to audit consumers.  In
        # particular, workflow_sha and tested_tree_oid are never reduced to a
        # path-overlap or source-only applicability check.
        "audit_identity": (
            {field: latest[field] for field in INTEGRATION_CI_FIELDS}
            if latest is not None else None
        ),
    }
    if accepted is not None:
        result["accepted_integration_identity"] = {
            field: accepted[field] for field in INTEGRATION_CI_FIELDS
        }
        result["accepted_integration_ci_digest"] = integration_ci_digest(accepted)
    if latest is not None:
        result["latest_integration_ci_digest"] = integration_ci_digest(latest)
    return result


def _verified_review_applicability(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError("review applicability evidence must be an object")
    expected_fields = {"identity", "identity_digest", "verified"}
    unknown = sorted(set(value) - expected_fields)
    if unknown:
        raise ValueError("review applicability evidence has unknown fields: " + ",".join(unknown))
    if value.get("verified") is not True:
        raise ValueError("review applicability identity is not independently verified")
    identity = _validate_review_applicability_identity(value.get("identity"))
    digest = value.get("identity_digest")
    if digest != review_applicability_digest(identity):
        raise ValueError("review applicability identity digest mismatch")
    return identity


def shadow_source_review_applicability(
    plan: dict[str, Any], latest_receipt: dict[str, Any],
    current_applicability: dict[str, Any] | None,
) -> dict[str, Any]:
    """Report a non-authorizing source-review reuse applicability decision.

    The result intentionally exposes two independent dimensions.  Complete
    integration provenance means the accepted and latest trusted receipts have
    parseable, audit-complete identities.  Professional-review applicability is
    reusable only when an independently verified applicability projection is
    present in the plan and is unchanged in the current projection.  This
    function is shadow-only: no promotion or closeout path calls it.
    """
    empty = _shadow_result(reason="integration_provenance_incomplete", integration_provenance="incomplete")
    if not isinstance(plan, dict) or plan.get("schema") != SOURCE_REVIEW_SCHEMA:
        return _shadow_result(
            reason="unsupported_review_plan", integration_provenance="incomplete"
        )

    try:
        source = _validate_source_identity(plan.get("source_review_identity"))
        if plan.get("source_review_digest") != source_review_digest(source):
            return _shadow_result(
                reason="source_review_identity_invalid", integration_provenance="incomplete"
            )
        if plan.get("integration_ci_provenance") != {
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
        }:
            return _shadow_result(
                reason="integration_provenance_untrusted", integration_provenance="incomplete"
            )
        accepted = integration_ci_identity(plan.get("integration_ci_identity"))
        if plan.get("integration_ci_digest") != integration_ci_digest(accepted):
            return _shadow_result(
                reason="integration_provenance_incomplete", integration_provenance="incomplete"
            )
    except (TypeError, ValueError, KeyError):
        return empty

    # Attestation and shape are checked independently of applicability.  The
    # workflow commit and tested tree remain visible in the execution audit;
    # neither is itself a professional-review applicability boundary.
    if not has_live_integration_attestation(latest_receipt):
        return _shadow_result(
            reason="integration_provenance_untrusted", integration_provenance="incomplete",
            accepted=accepted,
        )
    try:
        latest = integration_ci_identity(latest_receipt)
    except (TypeError, ValueError, KeyError):
        return _shadow_result(
            reason="integration_provenance_incomplete", integration_provenance="incomplete",
            accepted=accepted,
        )

    provenance = _shadow_result(
        reason="integration_ci_not_successful", integration_provenance="complete",
        accepted=accepted, latest=latest,
    )
    for identity in (accepted, latest):
        if (identity["repository"] != source["repository"]
                or identity["task_uid"] != source["task_uid"]
                or identity["pr_number"] != source["pr_number"]
                or identity["source_head_oid"] != source["source_head_oid"]):
            return _shadow_result(
                reason="integration_provenance_identity_mismatch",
                integration_provenance="complete", accepted=accepted, latest=latest,
            )
    if accepted["conclusion"] != "success" or latest["conclusion"] != "success":
        return provenance

    # A shadow result must still be based on a fresh trusted integration run.
    # A changed target/base alone is not allowed to turn a replayed old receipt
    # into fresh evidence; at least one dispatch/run/check identity must move.
    freshness_fields = (
        "request_id", "request_created_at", "run_id", "run_attempt", "check_run_id",
    )
    fresh = any(latest[field] != accepted[field] for field in freshness_fields)
    if not fresh:
        return _shadow_result(
            reason="integration_provenance_not_fresh", integration_provenance="complete",
            accepted=accepted, latest=latest,
        )
    provenance["fresh_trusted_integration_ci"] = True

    try:
        accepted_applicability = _verified_review_applicability(
            plan.get("professional_review_applicability")
        )
    except (TypeError, ValueError, KeyError):
        provenance.update(
            reason="applicability_identity_unknown",
            professional_review_applicability="requires_full_review",
        )
        return provenance
    if current_applicability is None:
        provenance.update(
            reason="applicability_identity_unknown",
            professional_review_applicability="requires_full_review",
        )
        return provenance
    try:
        current = _verified_review_applicability(current_applicability)
    except (TypeError, ValueError, KeyError) as exc:
        reason = (
            "applicability_identity_unverified"
            if "not independently verified" in str(exc)
            else "applicability_identity_unknown"
        )
        provenance.update(
            reason=reason,
            professional_review_applicability="requires_full_review",
        )
        return provenance
    if current != accepted_applicability:
        provenance.update(
            reason="applicability_identity_changed",
            professional_review_applicability="requires_full_review",
        )
        return provenance

    target_advanced = latest["integration_base_oid"] != accepted["integration_base_oid"]
    provenance.update(
        reason=("target_base_only_advance" if target_advanced else "applicability_identity_unchanged"),
        professional_review_applicability="unchanged",
        decision="reusable_source_review",
    )
    return provenance


def can_reuse_source_review(
    plan: dict[str, Any], latest_receipt: dict[str, Any],
    current_source_identity: dict[str, Any] | None = None,
    current_applicability: dict[str, Any] | None = None,
    *, require_fresh_integration: bool = True,
) -> bool:
    """Return whether trusted integration proves safe source-review reuse.

    ``tested_tree_oid`` belongs to integration execution provenance and is
    retained in the receipt/audit digest, but it is not a source-review reuse
    boundary.  Applicability is independently verified and digest-bound.  A
    complete plan still requires a fresh integration identity on the
    promotion/closeout path; ``require_fresh_integration=False`` is reserved
    for idempotent plan lookup while creating or refreshing an immutable plan.
    """
    try:
        if plan.get("schema") != SOURCE_REVIEW_SCHEMA:
            return False
        if not has_live_integration_attestation(latest_receipt):
            return False
        source = _validate_source_identity(plan.get("source_review_identity"))
        if plan.get("source_review_digest") != source_review_digest(source):
            return False
        if plan.get("impact_projection_schema") != PROJECTION_SCHEMA:
            return False
        if current_source_identity is not None and _validate_source_identity(current_source_identity) != source:
            return False
        _validate_projection_binding(plan, latest_receipt)
        accepted_raw = plan.get("integration_ci_identity")
        accepted: dict[str, Any] | None = None
        if accepted_raw is not None:
            accepted = integration_ci_identity(accepted_raw)
            if plan.get("integration_ci_digest") != integration_ci_digest(accepted):
                return False
            if accepted.get("conclusion") != "success":
                return False
        applicability = _verified_review_applicability(
            plan.get("professional_review_applicability")
        )
        if applicability != review_applicability_identity(source):
            return False
        if current_applicability is not None:
            if _verified_review_applicability(current_applicability) != applicability:
                return False
        latest = integration_ci_identity(latest_receipt)
        if latest.get("conclusion") != "success":
            return False
        if (latest["repository"] != source["repository"]
                or latest["task_uid"] != source["task_uid"]
                or latest["pr_number"] != source["pr_number"]
                or latest["source_head_oid"] != source["source_head_oid"]):
            return False
        if accepted is None:
            # This is the first trusted integration join for a source-only
            # plan. There is no prior execution identity to compare, but all
            # receipt fields above are still shape- and provenance-validated.
            return True
        if not all(latest.get(field) == accepted.get(field)
                   for field in INTEGRATION_AUTHORITY_REUSE_FIELDS):
            return False
        if latest == accepted:
            # A plan created from this exact successful receipt may complete
            # its first join idempotently; freshness is required only when a
            # later integration identity asks to reuse the source review.
            return True
        if require_fresh_integration:
            freshness_fields = (
                "request_id", "request_created_at", "run_id",
            )
            if not any(latest.get(field) != accepted.get(field) for field in freshness_fields):
                return False
        return True
    except (TypeError, ValueError, KeyError):
        return False


def validate_review_applicability(source_identity: dict[str, Any],
                                  applicability: dict[str, Any]) -> dict[str, Any]:
    """Require the verified applicability record to derive from source identity."""
    source = _validate_source_identity(source_identity)
    verified = _verified_review_applicability(applicability)
    if verified != review_applicability_identity(source):
        raise ValueError("review applicability does not match source review identity")
    return verified


def review_evidence_identity(receipt: dict[str, Any]) -> dict[str, Any]:
    missing = [field for field in AUTHORITY_FIELDS if field not in receipt]
    if missing:
        raise ValueError("CI receipt is missing review authority fields: " + ",".join(missing))
    result = {field: receipt[field] for field in AUTHORITY_FIELDS}
    if 'scope_base_oid' in receipt or 'integration_base_oid' in receipt:
        if not re.fullmatch(r'[0-9a-f]{40,64}', str(receipt.get('scope_base_oid', ''))) or receipt.get('integration_base_oid') != receipt['base_oid']:
            raise ValueError('CI receipt scope/integration authority is incomplete')
        result.update(scope_base_oid=receipt['scope_base_oid'], integration_base_oid=receipt['integration_base_oid'])
    if 'integration_run_id' in receipt:
        for key in ('integration_run_id','tested_tree_oid','tested_commit_oid','workflow_sha','cargo_package_profile'):
            if key not in receipt: raise ValueError('manual integration receipt authority incomplete')
            result[key]=receipt[key]
    return result


def review_evidence_digest(receipt: dict[str, Any]) -> str:
    canonical = json.dumps(
        review_evidence_identity(receipt), sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()
