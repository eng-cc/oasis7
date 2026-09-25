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
REQUIRED_PLAN_V1_SCHEMA = "oasis7-required-plan-v1"
REQUIRED_PLAN_V2_SCHEMA = "oasis7-required-plan-v2"
REQUIRED_DOMAIN_SPLIT_EXECUTION_CONTRACT = "required-domain-split/v1"
VERSIONED_PLANNER_SELECTOR_FIELDS = (
    "run_workflow_governance_contracts", "run_packaging_contracts",
    "run_doc_checker_contracts", "run_cargo_tooling_contracts",
)
VERSIONED_PLANNER_SELECTOR_CAPABILITIES = {
    "run_workflow_governance_contracts": "workflow_governance",
    "run_packaging_contracts": "packaging_contracts",
    "run_doc_checker_contracts": "doc_checker_contracts",
    "run_cargo_tooling_contracts": "cargo_tooling_contracts",
}
VERSIONED_PLANNER_RESOURCE_FIELDS = (
    "needs_python", "needs_markdown", "needs_rust_toolchain", "needs_node",
    "needs_system_deps", "needs_trunk", "needs_wasm_target",
)
INPUT_SCOPE_REUSE_CAPABILITY = "input-scope-reuse/v1"
SUPPORTED_REQUIRED_PLAN_CAPABILITIES = frozenset({INPUT_SCOPE_REUSE_CAPABILITY})
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


def read_required_plan_capabilities(plan: Any) -> tuple[str, ...] | None:
    """Read protocol capabilities without changing the legacy v1 lane.

    ``None`` means the existing v1 envelope has no capability protocol and must
    continue through its legacy consumer.  A v2 envelope is accepted only when
    it explicitly requires the one capability this reader understands.  This
    is a protocol reader, not an enablement decision: callers still need an
    effective policy that explicitly enables the capability.
    """
    if not isinstance(plan, dict):
        raise ValueError("required plan envelope must be an object")
    schema = plan.get("schema")
    if schema == REQUIRED_PLAN_V1_SCHEMA:
        if "required_capabilities" in plan:
            raise ValueError("legacy required-plan v1 cannot declare capabilities")
        return None
    if schema != REQUIRED_PLAN_V2_SCHEMA:
        raise ValueError("required plan schema is unsupported")

    capabilities = plan.get("required_capabilities")
    if (not isinstance(capabilities, list)
            or any(not isinstance(item, str) or not item for item in capabilities)
            or capabilities != sorted(set(capabilities))):
        raise ValueError("required-plan v2 capabilities are malformed")
    unknown = sorted(set(capabilities) - SUPPORTED_REQUIRED_PLAN_CAPABILITIES)
    if unknown:
        raise ValueError("required-plan v2 has unsupported capabilities: " + ",".join(unknown))
    if capabilities != [INPUT_SCOPE_REUSE_CAPABILITY]:
        raise ValueError("required-plan v2 must explicitly require input-scope-reuse/v1")
    return tuple(capabilities)


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

    def aliased_value(primary: str, alias: str, *, numeric: bool = False) -> Any:
        value, alternate = receipt.get(primary), receipt.get(alias)
        if value is not None and alternate is not None:
            matches = str(value) == str(alternate) if numeric else value == alternate
            if not matches:
                raise ValueError(f"integration CI identity has conflicting aliases: {primary}/{alias}")
        return value if value is not None else alternate

    source_head_oid = aliased_value("source_head_oid", "head_oid")
    integration_base_oid = aliased_value("integration_base_oid", "base_oid")
    run_id = aliased_value("run_id", "integration_run_id", numeric=True)
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
        if (isinstance(values[field], bool)
                or not isinstance(values[field], (int, str))
                or not re.fullmatch(r"[1-9][0-9]*", str(values[field]))):
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


def is_ordinary_pr_ci_receipt(receipt: dict[str, Any]) -> bool:
    """Recognize source-bound PR CI without treating it as target integration.

    The mode is emitted by the live receipt helper.  Missing mode remains a
    read-only compatibility shape for older ordinary receipts; live
    validation still re-reads the check run before this helper is consulted.
    """
    if not isinstance(receipt, dict) or receipt.get("receipt_type") != "oasis7_ci_ready_receipt":
        return False
    if receipt.get("issuer") != "github_live_query" or receipt.get("trusted_integration_artifact") is True:
        return False
    if receipt.get("ci_validation_mode") not in (None, "ordinary_pr"):
        return False
    if receipt.get("conclusion") != "success":
        return False
    required = ("repository", "task_uid", "task_issue_number", "pr_number", "base_oid",
                "head_oid", "check_name", "check_app_id", "check_run_id", "planner_digest")
    if not all(receipt.get(field) not in (None, "") for field in required):
        return False
    # New ordinary receipts bind the target ref in addition to its recorded
    # base OID. Older receipts without a validation mode remain compatibility
    # inputs and are still revalidated by the live helper before reuse.
    return receipt.get("ci_validation_mode") != "ordinary_pr" or bool(receipt.get("base_ref"))


def has_live_pr_ci_attestation(receipt: dict[str, Any]) -> bool:
    """Return whether a receipt is a live, source-bound ordinary PR check."""
    return is_ordinary_pr_ci_receipt(receipt) and (
        receipt.get("live_validation") in (None, "ci-ready-receipt-live")
    )


def projection_requires_strict_integration(plan: dict[str, Any]) -> bool:
    """Fail closed for ordinary receipts when the bound projection is risky.

    Promotion and closeout receive an immutable v2 plan plus a refreshed CI
    receipt, but do not have the live PR payload used by the merge gate.  The
    plan's already verified impact projection therefore remains the authority
    for rejecting an ordinary receipt on high-risk or unknown source scope.
    """
    projection = plan.get("impact_projection") if isinstance(plan, dict) else None
    if not isinstance(projection, dict):
        return True
    source = plan.get("source_review_identity")
    if not isinstance(source, dict):
        return True
    projection_digest = projection.get("projection_digest")
    if (not isinstance(projection_digest, str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", projection_digest)):
        return True
    projection_body = {key: value for key, value in projection.items() if key != "projection_digest"}
    expected_projection_digest = "sha256:" + hashlib.sha256(
        json.dumps(projection_body, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if (projection_digest != expected_projection_digest
            or plan.get("impact_projection_digest") != projection_digest):
        return True
    if (projection.get("source_head_oid") != source.get("source_head_oid")
            or projection.get("scope_base_oid") != source.get("source_scope_oid")):
        return True
    changed_paths = projection.get("changed_paths")
    changed_paths_digest = projection.get("changed_paths_digest")
    if (not isinstance(changed_paths, list)
            or changed_paths != sorted(set(changed_paths))
            or any(not isinstance(path, str) or not path for path in changed_paths)
            or not isinstance(changed_paths_digest, str)
            or changed_paths_digest != "sha256:" + hashlib.sha256(
                json.dumps(changed_paths, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
            ).hexdigest()
            or source.get("changed_paths_digest") != changed_paths_digest.removeprefix("sha256:")):
        return True
    # Explicit public behavior changes are strict by structure; consuming a
    # stable contract alone does not imply that the target changed it.
    if projection.get("public_semantics"):
        return True
    if projection.get("review_escalated") is True or projection.get("verification_affected") is True:
        return True
    if projection.get("change_class") in {"workflow-doc", "unknown", "mixed"}:
        return True
    closure = projection.get("closure_status")
    if not isinstance(closure, dict) or closure.get("status") != "complete":
        return True
    risk_text = json.dumps(
        [reason for reason in projection.get("review_reasons", [])
         if isinstance(reason, str) and not reason.startswith("input:")],
        sort_keys=True,
    ).lower()
    risk_terms = ("api", "abi", "persistence", "serialization", "state-root", "consensus",
                  "security", "dependency", "permission", "workflow", "validation", "contract",
                  "schema", "migration", "wasm", "replay", "recovery", "critical")
    if any(term in risk_text for term in risk_terms):
        return True
    high_risk_paths = (".github/workflows/", ".codex/", "scripts/pm/", "cargo.toml", "cargo.lock")
    return any(
        any(path.lower().startswith(prefix) or path.lower() == prefix.rstrip("/")
            for prefix in high_risk_paths)
        for path in projection.get("changed_paths", [])
    )


def _related_path(path: str, other: str) -> bool:
    left, right = path.strip("/"), other.strip("/")
    return left == right or left.startswith(right + "/") or right.startswith(left + "/")


def _relation_path(value: Any) -> str | None:
    """Return a repository-relative path from a trusted relation locator."""
    if not isinstance(value, str) or not value.strip():
        return None
    path = value.strip().split("#", 1)[0].strip("/")
    if (not path or path.startswith(".") or "\x00" in path
            or "\\" in path or any(part in {"", ".", ".."} for part in path.split("/"))):
        return None
    return path


def _source_path_exists(root: pathlib.Path, source_head: str, path: str) -> bool:
    try:
        return subprocess.run(
            ["git", "-C", str(root), "cat-file", "-e", f"{source_head}:{path}"],
            check=False, capture_output=True,
        ).returncode == 0
    except OSError:
        return False


def _target_relation_paths(
    projection: dict[str, Any], *, root: pathlib.Path, source_head: str,
) -> tuple[list[str], bool]:
    """Collect verified contract/consumer paths for target-only drift checks."""
    relations = [
        path for path in (projection.get("changed_paths") or [])
        if isinstance(path, str)
    ]
    unmapped = False

    def mapped_candidates(candidates: list[Any]) -> list[str]:
        mapped: list[str] = []
        for value in candidates:
            path = _relation_path(value)
            if path and _source_path_exists(root, source_head, path):
                mapped.append(path)
        return mapped

    for item in projection.get("affected_consumers") or []:
        candidates: list[Any] = []
        if isinstance(item, str):
            candidates.append(item)
        elif isinstance(item, dict):
            candidates.extend(item.get(key) for key in ("path", "consumer_path", "contract_path"))
            refs = item.get("references")
            if isinstance(refs, list):
                candidates.extend(ref.get("path") for ref in refs if isinstance(ref, dict))
        mapped = mapped_candidates(candidates)
        if mapped:
            relations.extend(mapped)
        else:
            unmapped = True

    for item in projection.get("consumed_contracts") or []:
        candidates: list[Any] = []
        if isinstance(item, str):
            candidates.append(item)
        elif isinstance(item, dict):
            candidates.extend(item.get(key) for key in ("path", "contract_path"))
            for container_key in ("consumed_clause_refs", "content_refs", "clauses"):
                refs = item.get(container_key)
                if isinstance(refs, list):
                    candidates.extend(ref.get("path") for ref in refs if isinstance(ref, dict))
            nested = item.get("contract")
            if isinstance(nested, dict):
                candidates.append(nested.get("path"))
                refs = nested.get("content_refs")
                if isinstance(refs, list):
                    candidates.extend(ref.get("path") for ref in refs if isinstance(ref, dict))
        mapped = mapped_candidates(candidates)
        if mapped:
            relations.extend(mapped)
        else:
            unmapped = True

    closure = projection.get("closure_status") or {}
    evidence = closure.get("evidence") if isinstance(closure, dict) else None
    if evidence is not None and not isinstance(evidence, list):
        unmapped = True
    if isinstance(evidence, list):
        for item in evidence:
            path = _relation_path(item.get("path")) if isinstance(item, dict) else None
            if path and _source_path_exists(root, source_head, path):
                relations.append(path)
            else:
                unmapped = True
    return relations, unmapped


def _target_advance_requires_strict(
    plan: dict[str, Any], *, root: pathlib.Path | str | None, current_target_oid: str,
) -> bool:
    """Return whether a live target advance is related or unverifiable.

    Ordinary CI is allowed to reuse source review only when the current target
    advance can be proven unrelated from the verified projection.  A missing
    target object, non-ancestor scope, or unmapped contract/consumer therefore
    fails closed.
    """
    if root is None:
        return True
    try:
        source = _validate_source_identity(plan.get("source_review_identity"))
        target = _require_oid(current_target_oid, "current_target_oid")
        scope = _require_oid(source["source_scope_oid"], "source_scope_oid")
        source_head = _require_oid(source["source_head_oid"], "source_head_oid")
    except (TypeError, ValueError, KeyError):
        return True
    if target == scope:
        return False
    root_path = pathlib.Path(root).resolve()
    try:
        subprocess.run(
            ["git", "-C", str(root_path), "merge-base", "--is-ancestor", scope, target],
            check=True, capture_output=True,
        )
        changed = subprocess.check_output(
            ["git", "-C", str(root_path), "diff", "--name-only", f"{scope}..{target}"],
            text=True,
        ).splitlines()
    except (OSError, subprocess.CalledProcessError):
        return True
    projection = plan.get("impact_projection")
    if not isinstance(projection, dict):
        return True
    relations, unmapped = _target_relation_paths(
        projection, root=root_path, source_head=source_head,
    )
    if unmapped or not relations:
        return True
    return any(_related_path(path, relation) for path in changed for relation in relations)


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
    current_target_oid: str | None = None,
    current_target_root: pathlib.Path | str | None = None,
    *, require_fresh_integration: bool = True,
) -> bool:
    """Return whether trusted integration proves safe source-review reuse.

    ``tested_tree_oid`` belongs to integration execution provenance and is
    retained in the receipt/audit digest, but it is not a source-review reuse
    boundary.  Applicability is independently verified and digest-bound.  A
    complete plan still requires a fresh integration identity on the
    promotion/closeout path; ``require_fresh_integration=False`` is reserved
    for idempotent plan lookup while creating or refreshing an immutable plan.
    Promotion and closeout may pass ``current_target_oid`` and
    ``current_target_root`` to prove that an ordinary target advance is
    unrelated to the verified projection.
    """
    try:
        if plan.get("schema") != SOURCE_REVIEW_SCHEMA:
            return False
        source = _validate_source_identity(plan.get("source_review_identity"))
        if plan.get("source_review_digest") != source_review_digest(source):
            return False
        if plan.get("impact_projection_schema") != PROJECTION_SCHEMA:
            return False
        if current_source_identity is not None and _validate_source_identity(current_source_identity) != source:
            return False
        _validate_projection_binding(plan, latest_receipt)
        applicability = _verified_review_applicability(
            plan.get("professional_review_applicability")
        )
        if applicability != review_applicability_identity(source):
            return False
        if has_live_pr_ci_attestation(latest_receipt):
            mode = plan.get("effective_mode")
            if not isinstance(mode, dict) or mode.get("effective_policy") == "legacy":
                return False
            if projection_requires_strict_integration(plan):
                return False
            if latest_receipt.get("task_uid") != source["task_uid"]:
                return False
            if latest_receipt.get("head_oid") != source["source_head_oid"]:
                return False
            if current_applicability is not None:
                if _verified_review_applicability(current_applicability) != applicability:
                    return False
            if current_target_oid is not None and _target_advance_requires_strict(
                    plan, root=current_target_root, current_target_oid=current_target_oid):
                return False
            return True
        if not has_live_integration_attestation(latest_receipt):
            return False
        accepted_raw = plan.get("integration_ci_identity")
        accepted: dict[str, Any] | None = None
        if accepted_raw is not None:
            accepted = integration_ci_identity(accepted_raw)
            if plan.get("integration_ci_digest") != integration_ci_digest(accepted):
                return False
            if accepted.get("conclusion") != "success":
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
    execution_contract = receipt.get("execution_contract")
    if execution_contract is None:
        planner = receipt.get("planner")
        versioned_fields = VERSIONED_PLANNER_SELECTOR_FIELDS + VERSIONED_PLANNER_RESOURCE_FIELDS[:2]
        if any(field in receipt for field in VERSIONED_PLANNER_SELECTOR_FIELDS) or (
            isinstance(planner, dict) and any(field in planner for field in versioned_fields)
        ):
            raise ValueError("versioned planner fields require execution_contract")
    else:
        if execution_contract != REQUIRED_DOMAIN_SPLIT_EXECUTION_CONTRACT:
            raise ValueError("CI receipt execution_contract is unsupported")
        planner = receipt.get("planner")
        if not isinstance(planner, dict) or planner.get("execution_contract") != execution_contract:
            raise ValueError("versioned CI receipt planner contract is missing or mismatched")
        selected_capabilities = planner.get("selected_capabilities")
        if (not isinstance(selected_capabilities, list)
                or any(not isinstance(capability, str) for capability in selected_capabilities)
                or selected_capabilities != sorted(set(selected_capabilities))):
            raise ValueError("versioned CI receipt selected capabilities are malformed")
        for field in VERSIONED_PLANNER_SELECTOR_FIELDS:
            if type(planner.get(field)) is not bool:
                raise ValueError("versioned CI receipt planner selector is missing or malformed: " + field)
            if planner[field] != (VERSIONED_PLANNER_SELECTOR_CAPABILITIES[field] in selected_capabilities):
                raise ValueError("versioned CI receipt planner selector contradicts selected capabilities: " + field)
        for field in VERSIONED_PLANNER_RESOURCE_FIELDS:
            if type(planner.get(field)) is not bool:
                raise ValueError("versioned CI receipt planner resource is missing or malformed: " + field)
        if planner["needs_python"] is not True or planner["needs_markdown"] is not True:
            raise ValueError("versioned CI receipt baseline requires Python and Markdown resources")
        planner_digest = hashlib.sha256(
            json.dumps(planner, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        if planner_digest != receipt.get("planner_digest"):
            raise ValueError("versioned CI receipt planner digest mismatch")
        result["execution_contract"] = execution_contract
        result.update({field: planner[field] for field in VERSIONED_PLANNER_SELECTOR_FIELDS})
    if receipt.get("ci_validation_mode") is not None:
        if receipt.get("ci_validation_mode") not in {"ordinary_pr", "trusted_integration"}:
            raise ValueError("CI receipt validation mode is invalid")
        result["ci_validation_mode"] = receipt["ci_validation_mode"]
    if receipt.get("base_ref") is not None:
        if not isinstance(receipt.get("base_ref"), str) or not receipt["base_ref"].strip():
            raise ValueError("CI receipt target ref is invalid")
        result["base_ref"] = receipt["base_ref"]
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
