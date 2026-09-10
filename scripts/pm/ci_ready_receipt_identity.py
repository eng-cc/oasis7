#!/usr/bin/env python3
"""Stable review identity for a trusted CI-ready receipt."""

from __future__ import annotations

import hashlib
import json
import re
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


def integration_ci_digest(identity: dict[str, Any]) -> str:
    canonical = json.dumps(
        {field: integration_ci_identity(identity)[field] for field in INTEGRATION_CI_FIELDS},
        sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()


def can_reuse_source_review(
    plan: dict[str, Any], latest_receipt: dict[str, Any],
    current_source_identity: dict[str, Any] | None = None,
) -> bool:
    """Return whether latest integration proves safe v2 source-review reuse."""
    try:
        if plan.get("schema") != SOURCE_REVIEW_SCHEMA:
            return False
        if plan.get("integration_ci_provenance") != {
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
        }:
            return False
        if not has_live_integration_attestation(latest_receipt):
            return False
        source = _validate_source_identity(plan.get("source_review_identity"))
        if plan.get("source_review_digest") != source_review_digest(source):
            return False
        if current_source_identity is not None and _validate_source_identity(current_source_identity) != source:
            return False
        accepted = integration_ci_identity(plan.get("integration_ci_identity"))
        if plan.get("integration_ci_digest") != integration_ci_digest(accepted):
            return False
        if accepted.get("conclusion") != "success":
            return False
        latest = integration_ci_identity(latest_receipt)
        if latest.get("conclusion") != "success":
            return False
        return all(latest.get(field) == accepted.get(field) for field in INTEGRATION_REUSE_FIELDS)
    except (TypeError, ValueError, KeyError):
        return False


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
        for key in ('integration_run_id','tested_tree_oid','tested_commit_oid','workflow_sha'):
            if key not in receipt: raise ValueError('manual integration receipt authority incomplete')
            result[key]=receipt[key]
    return result


def review_evidence_digest(receipt: dict[str, Any]) -> str:
    canonical = json.dumps(
        review_evidence_identity(receipt), sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()
