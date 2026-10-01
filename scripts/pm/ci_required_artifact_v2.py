#!/usr/bin/env python3
"""Closed schemas and deterministic names for keyed required-gate evidence.

The artifact is only data. Consumers must resolve artifact IDs and planner,
check, job, and policy authority from the exact live workflow attempt before
passing the embedded inventory to the C2 validator.
"""

from __future__ import annotations

import hashlib
import json
import re
import argparse
import base64
from pathlib import Path
import sys
from typing import Any


PLAN_SCHEMA = "oasis7-required-plan-v2"
RESULT_SCHEMA = "oasis7-required-result-v2"
CAPABILITY = "input-scope-reuse/v1"
REQUEST_SCHEMA = "oasis7-ci-validation-request/v2"
POLICY_IDENTITY_SCHEMA = "oasis7-ci-effective-policy-identity/v1"
PLANNER_INVOCATION_SCHEMA = "oasis7-required-scope-invocation/v1"
PLANNER_INVENTORY_SCHEMA = "oasis7-trusted-planner-inventory/v1"
PLANNER_AUTHORITY_SCHEMA = "oasis7-planner-inventory-authority/v1"
INPUT_SCOPE_SCHEMA = "oasis7-ci-input-scope/v2"
REQUIRED_CHECK = "required-gate"
FLEET_RUNNERS = ("macos-14", "ubuntu-24.04", "windows-2022")

PLAN_FIELDS = frozenset({
    "schema", "required_capabilities", "request_key", "request_identity",
    "repository", "task_uid", "pr_number", "bootstrap_epoch", "source_head_oid",
    "source_scope_oid", "source_projection_digest", "integration_base_oid",
    "tested_commit_oid", "tested_tree_oid", "workflow_ref", "workflow_sha",
    "workflow_run_id", "run_attempt", "check_name", "check_app_id", "check_run_id",
    "job_id", "job_name", "executor_contract_digest", "effective_policy_identity",
    "planner_inventory_authority", "planner_inventory_issuer", "planner_inventory_digest",
    "planner_config_sha256", "planner_invocation", "unit_ids", "required_test_units",
    "input_fingerprints", "unit_specs", "product_corpus", "input_scope",
    "planner_output", "execution_job_requirements", "closure_status",
})

RESULT_FIELDS = frozenset({
    "schema", "request_key", "request_identity", "repository", "task_uid", "pr_number",
    "bootstrap_epoch", "source_head_oid", "source_scope_oid", "source_projection_digest",
    "integration_base_oid", "tested_commit_oid", "tested_tree_oid", "workflow_ref",
    "workflow_sha", "workflow_run_id", "run_attempt", "check_name", "check_app_id",
    "check_run_id", "job_id", "job_name", "executor_contract_digest",
    "effective_policy_identity", "planner_inventory_digest", "planner_inventory_issuer",
    "plan_artifact_id", "unit_id", "obligation_ids", "input_digest", "status",
    "disposition", "exit_code", "execution_jobs",
})

REQUEST_IDENTITY_FIELDS = frozenset({
    "repository", "task_uid", "pr_number", "bootstrap_epoch", "source_head_oid",
    "publication_id", "source_projection_digest", "unit_ids", "input_fingerprints",
    "executor_contract_digest", "effective_policy_digest", "purpose",
    "applicability_mode", "snapshot_target_oid",
})

JOB_FIELDS = frozenset({
    "workflow_run_id", "run_attempt", "job_id", "job_name", "check_name",
    "check_app_id", "check_run_id", "head_sha", "status", "conclusion", "labels",
})

_OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")


class RequiredArtifactError(ValueError):
    """A required-plan or unit-result artifact is malformed or unproven."""


def canonical_bytes(value: Any) -> bytes:
    try:
        return json.dumps(value, ensure_ascii=False, sort_keys=True,
                          separators=(",", ":"), allow_nan=False).encode("utf-8")
    except (TypeError, ValueError, UnicodeEncodeError) as exc:
        raise RequiredArtifactError(f"value is not canonical JSON: {exc}") from exc


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def request_key_for_identity(identity: Any) -> str:
    validate_request_identity(identity)
    return canonical_digest({"schema": REQUEST_SCHEMA, **identity})


def planner_invocation_digest(invocation: Any) -> str:
    fields = (
        "schema", "planner_authority_oid", "planner_config_sha256", "event_name",
        "run_mode", "base_ref", "head_ref", "task_uid", "scope_base_oid",
        "impact_projection_sha256", "changed_paths", "planner_output_sha256",
    )
    if not isinstance(invocation, dict) or any(field not in invocation for field in fields):
        raise RequiredArtifactError("planner invocation fields are incomplete")
    return canonical_digest({field: invocation[field] for field in fields})


def unit_id_sha256(unit_id: Any) -> str:
    _string(unit_id, "unit_id")
    return hashlib.sha256(unit_id.encode("utf-8")).hexdigest()


def plan_artifact_name(run_id: Any, attempt: Any) -> str:
    _positive_int(run_id, "workflow_run_id")
    _positive_int(attempt, "run_attempt")
    return f"oasis7-required-plan-v2-{run_id}-a{attempt}"


def result_artifact_name(run_id: Any, attempt: Any, unit_id: Any) -> str:
    _positive_int(run_id, "workflow_run_id")
    _positive_int(attempt, "run_attempt")
    return f"oasis7-required-result-v2-{run_id}-a{attempt}-{unit_id_sha256(unit_id)}"


def execution_job_requirements(unit_ids: Any) -> dict[str, list[str]]:
    units = _sorted_strings(unit_ids, "unit_ids", nonempty=True)
    result: dict[str, list[str]] = {}
    for unit_id in units:
        required: list[str] = []
        if unit_id == "packaging_contracts":
            required.append("testnet-packages-macos-arm64-contract")
        elif unit_id == "operational_contracts":
            required.extend((
                "windows-package-rollout-behavior",
                *(f"public-testnet-fleet-health-contract ({runner})" for runner in FLEET_RUNNERS),
            ))
        result[unit_id] = sorted(required)
    return result


def _require_fields(value: Any, fields: frozenset[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        raise RequiredArtifactError(f"{label} fields are incomplete or unsupported")
    return value


def _string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value or value != value.strip():
        raise RequiredArtifactError(f"{label} must be a non-empty trimmed string")
    if any(char in value for char in "\x00\r\n"):
        raise RequiredArtifactError(f"{label} contains a control character")
    return value


def _positive_int(value: Any, label: str) -> int:
    if type(value) is not int or value <= 0:
        raise RequiredArtifactError(f"{label} must be a positive integer")
    return value


def _digest(value: Any, label: str) -> str:
    if not isinstance(value, str) or not _DIGEST_RE.fullmatch(value):
        raise RequiredArtifactError(f"{label} must be a prefixed SHA-256 digest")
    return value


def _oid(value: Any, label: str) -> str:
    if not isinstance(value, str) or not _OID_RE.fullmatch(value):
        raise RequiredArtifactError(f"{label} must be a lowercase Git object ID")
    return value


def _sorted_strings(value: Any, label: str, *, nonempty: bool = False) -> list[str]:
    if not isinstance(value, list):
        raise RequiredArtifactError(f"{label} must be a list")
    values = [_string(item, f"{label}[{index}]") for index, item in enumerate(value)]
    if values != sorted(set(values)) or (nonempty and not values):
        raise RequiredArtifactError(f"{label} must be sorted, unique" + (", and non-empty" if nonempty else ""))
    return values


def validate_request_identity(value: Any) -> dict[str, Any]:
    identity = _require_fields(value, REQUEST_IDENTITY_FIELDS, "request identity")
    if not isinstance(identity["repository"], str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", identity["repository"]):
        raise RequiredArtifactError("request repository is invalid")
    if not isinstance(identity["task_uid"], str) or not _UID_RE.fullmatch(identity["task_uid"]):
        raise RequiredArtifactError("request task UID is invalid")
    _positive_int(identity["pr_number"], "request pr_number")
    _positive_int(identity["bootstrap_epoch"], "bootstrap_epoch")
    _oid(identity["source_head_oid"], "request source_head_oid")
    _string(identity["publication_id"], "publication_id")
    _digest(identity["source_projection_digest"], "source_projection_digest")
    _digest(identity["executor_contract_digest"], "executor_contract_digest")
    _digest(identity["effective_policy_digest"], "effective_policy_digest")
    units = _sorted_strings(identity["unit_ids"], "request unit_ids", nonempty=True)
    fingerprints = identity["input_fingerprints"]
    if not isinstance(fingerprints, dict) or set(fingerprints) != set(units):
        raise RequiredArtifactError("request input fingerprint closure is incomplete")
    for unit, value_digest in fingerprints.items():
        _digest(value_digest, f"request input fingerprint {unit}")
    _string(identity["purpose"], "request purpose")
    if identity["applicability_mode"] not in ("input_scoped", "snapshot_exact"):
        raise RequiredArtifactError("request applicability mode is invalid")
    target = identity["snapshot_target_oid"]
    if identity["applicability_mode"] == "snapshot_exact":
        _oid(target, "request snapshot_target_oid")
    elif target is not None:
        raise RequiredArtifactError("input-scoped request must not bind a snapshot target")
    return identity


def _validate_authority(value: Any) -> dict[str, Any]:
    fields = {"schema", "repository", "workflow_ref", "planner_authority_oid", "planner_config_sha256"}
    authority = _require_fields(value, fields, "planner authority")
    if authority["schema"] != PLANNER_AUTHORITY_SCHEMA:
        raise RequiredArtifactError("planner authority schema is unsupported")
    _string(authority["repository"], "planner authority repository")
    _string(authority["workflow_ref"], "planner authority workflow_ref")
    _oid(authority["planner_authority_oid"], "planner_authority_oid")
    _digest(authority["planner_config_sha256"], "planner_config_sha256")
    return authority


def _validate_issuer(value: Any) -> dict[str, Any]:
    fields = {"schema", "authority", "producer", "target_oid", "target_tree_oid", "unit_ids", "inventory_digest"}
    issuer = _require_fields(value, fields, "planner inventory issuer")
    if issuer["schema"] != PLANNER_INVENTORY_SCHEMA:
        raise RequiredArtifactError("planner inventory issuer schema is unsupported")
    _validate_authority(issuer["authority"])
    producer = _require_fields(issuer["producer"], {"run_id", "run_attempt", "check_app_id", "check_run_id"}, "inventory producer")
    for field in producer:
        _positive_int(producer[field], f"inventory producer {field}")
    _oid(issuer["target_oid"], "inventory target_oid")
    _oid(issuer["target_tree_oid"], "inventory target_tree_oid")
    _sorted_strings(issuer["unit_ids"], "inventory unit_ids", nonempty=True)
    _digest(issuer["inventory_digest"], "inventory_digest")
    return issuer


def _validate_job(value: Any) -> dict[str, Any]:
    job = _require_fields(value, JOB_FIELDS, "execution job")
    for field in ("workflow_run_id", "run_attempt", "job_id", "check_app_id", "check_run_id"):
        _positive_int(job[field], f"execution job {field}")
    _string(job["job_name"], "execution job job_name")
    _string(job["check_name"], "execution job check_name")
    _oid(job["head_sha"], "execution job head_sha")
    if not isinstance(job["status"], str) or job["status"] not in ("queued", "in_progress", "completed"):
        raise RequiredArtifactError("execution job status is invalid")
    if job["conclusion"] is not None and job["conclusion"] not in (
        "success", "failure", "cancelled", "skipped", "timed_out", "action_required", "neutral", "stale",
    ):
        raise RequiredArtifactError("execution job conclusion is invalid")
    _sorted_strings(job["labels"], "execution job labels")
    return job


def _validate_planner_invocation(value: Any, *, plan: dict[str, Any]) -> dict[str, Any]:
    fields = {
        "schema", "planner_authority_oid", "planner_config_sha256", "event_name", "run_mode",
        "base_ref", "head_ref", "task_uid", "scope_base_oid", "impact_projection_sha256",
        "changed_paths", "planner_output_sha256", "producer", "digest",
    }
    invocation = _require_fields(value, fields, "planner invocation")
    if invocation["schema"] != PLANNER_INVOCATION_SCHEMA:
        raise RequiredArtifactError("planner invocation schema is unsupported")
    if invocation["event_name"] != "workflow_dispatch" or invocation["run_mode"] != "integration_revalidation":
        raise RequiredArtifactError("planner invocation is not an integration revalidation")
    for field in ("base_ref", "head_ref", "scope_base_oid", "planner_authority_oid"):
        _oid(invocation[field], f"planner invocation {field}")
    _digest(invocation["planner_config_sha256"], "planner invocation config digest")
    _digest(invocation["impact_projection_sha256"], "planner invocation projection digest")
    _digest(invocation["planner_output_sha256"], "planner output digest")
    _string(invocation["task_uid"], "planner invocation task UID")
    paths = invocation["changed_paths"]
    if not isinstance(paths, list) or any(not isinstance(path, str) or not path for path in paths) or len(paths) != len(set(paths)):
        raise RequiredArtifactError("planner invocation changed_paths is malformed")
    expected_producer = {
        "run_id": plan["workflow_run_id"], "run_attempt": plan["run_attempt"],
        "check_app_id": plan["check_app_id"], "check_run_id": plan["check_run_id"],
    }
    if invocation["producer"] != expected_producer:
        raise RequiredArtifactError("planner invocation producer does not match the live plan check")
    if (invocation["base_ref"] != plan["integration_base_oid"]
            or invocation["head_ref"] != plan["source_head_oid"]
            or invocation["scope_base_oid"] != plan["source_scope_oid"]
            or invocation["task_uid"] != plan["task_uid"]
            or invocation["planner_authority_oid"] != plan["workflow_sha"]
            or invocation["planner_config_sha256"] != plan["planner_config_sha256"]
            or invocation["impact_projection_sha256"] != plan["source_projection_digest"]):
        raise RequiredArtifactError("planner invocation differs from the plan identity")
    if invocation["digest"] != planner_invocation_digest(invocation):
        raise RequiredArtifactError("planner invocation digest mismatch")
    return invocation


def _validate_plan(value: Any, *, require_complete: bool) -> dict[str, Any]:
    plan = _require_fields(value, PLAN_FIELDS, "required-plan v2")
    if plan["schema"] != PLAN_SCHEMA or plan["required_capabilities"] != [CAPABILITY]:
        raise RequiredArtifactError("required-plan v2 schema or capability is unsupported")
    request_identity = validate_request_identity(plan["request_identity"])
    if plan["request_key"] != request_key_for_identity(request_identity):
        raise RequiredArtifactError("required-plan request key does not match its identity")
    for field in ("repository", "task_uid"):
        _string(plan[field], field)
    for field in ("pr_number", "bootstrap_epoch", "workflow_run_id", "run_attempt",
                  "check_app_id", "check_run_id", "job_id"):
        _positive_int(plan[field], field)
    if type(plan["bootstrap_epoch"]) is not int:
        raise RequiredArtifactError("bootstrap_epoch must be a positive integer")
    for field in ("source_head_oid", "source_scope_oid", "integration_base_oid", "tested_commit_oid",
                  "tested_tree_oid", "workflow_sha"):
        _oid(plan[field], field)
    for field in ("source_projection_digest", "executor_contract_digest", "planner_inventory_digest",
                  "planner_config_sha256"):
        _digest(plan[field], field)
    if plan["check_name"] != REQUIRED_CHECK or plan["job_name"] != REQUIRED_CHECK:
        raise RequiredArtifactError("required-plan must bind the required-gate job and check")
    if plan["workflow_ref"] != f'{plan["repository"]}/.github/workflows/rust.yml@refs/heads/main':
        raise RequiredArtifactError("required-plan workflow ref is not the trusted default workflow")
    if (plan["repository"] != request_identity["repository"]
            or plan["task_uid"] != request_identity["task_uid"]
            or plan["pr_number"] != request_identity["pr_number"]
            or plan["bootstrap_epoch"] != request_identity["bootstrap_epoch"]
            or plan["source_head_oid"] != request_identity["source_head_oid"]
            or plan["source_projection_digest"] != request_identity["source_projection_digest"]
            or plan["executor_contract_digest"] != request_identity["executor_contract_digest"]):
        raise RequiredArtifactError("required-plan differs from its request identity")
    if (request_identity["applicability_mode"] == "snapshot_exact"
            and request_identity["snapshot_target_oid"] != plan["tested_commit_oid"]):
        raise RequiredArtifactError(
            "snapshot-exact request target differs from the tested commit",
        )
    policy_identity = _require_fields(
        plan["effective_policy_identity"], {"schema", "digest"}, "effective policy identity",
    )
    if policy_identity["schema"] != POLICY_IDENTITY_SCHEMA:
        raise RequiredArtifactError("effective policy identity schema is unsupported")
    _digest(policy_identity["digest"], "effective policy identity digest")
    authority = _validate_authority(plan["planner_inventory_authority"])
    issuer = _validate_issuer(plan["planner_inventory_issuer"])
    if issuer["authority"] != authority or authority["planner_authority_oid"] != plan["workflow_sha"]:
        raise RequiredArtifactError("planner inventory authority differs from workflow W")
    if authority["repository"] != plan["repository"] or authority["workflow_ref"] != plan["workflow_ref"]:
        raise RequiredArtifactError("planner inventory authority repository/workflow differs")
    if authority["planner_config_sha256"] != plan["planner_config_sha256"]:
        raise RequiredArtifactError("planner inventory config differs from the plan")
    if plan["planner_inventory_digest"] != issuer["inventory_digest"]:
        raise RequiredArtifactError("planner inventory digest alias differs from issuer")
    producer = issuer["producer"]
    expected_producer = {
        "run_id": plan["workflow_run_id"], "run_attempt": plan["run_attempt"],
        "check_app_id": plan["check_app_id"], "check_run_id": plan["check_run_id"],
    }
    if producer != expected_producer:
        raise RequiredArtifactError("planner inventory is not bound to the required-gate attempt/check")
    if (issuer["target_oid"] != plan["tested_commit_oid"]
            or issuer["target_tree_oid"] != plan["tested_tree_oid"]):
        raise RequiredArtifactError("planner inventory target differs from tested M/T")

    unit_ids = _sorted_strings(plan["unit_ids"], "unit_ids", nonempty=True)
    required_units = _sorted_strings(plan["required_test_units"], "required_test_units", nonempty=True)
    if unit_ids != required_units or issuer["unit_ids"] != unit_ids:
        raise RequiredArtifactError("required test units do not close the planner inventory")
    specs = plan["unit_specs"]
    if not isinstance(specs, list) or any(not isinstance(spec, dict) for spec in specs):
        raise RequiredArtifactError("required-plan unit_specs must be an object list")
    spec_ids = [spec.get("unit_id") for spec in specs]
    if sorted(spec_ids) != unit_ids or len(spec_ids) != len(set(spec_ids)):
        raise RequiredArtifactError("required-plan unit_specs do not close the inventory")
    fingerprints = plan["input_fingerprints"]
    if not isinstance(fingerprints, dict) or set(fingerprints) != set(unit_ids):
        raise RequiredArtifactError("required-plan input fingerprints do not close the inventory")
    for unit_id, fingerprint in fingerprints.items():
        _digest(fingerprint, f"input fingerprint for {unit_id}")
    if not isinstance(plan["product_corpus"], dict):
        raise RequiredArtifactError("required-plan product_corpus is invalid")

    scope = plan["input_scope"]
    scope_fields = {
        "schema", "target_oid", "target_tree_oid", "planner_inventory_issuer", "closure_status",
        "required_test_units", "input_fingerprints", "dependency_edges", "fallback_complete",
        "fallback_contract", "product_corpus",
    }
    _require_fields(scope, scope_fields, "input-scope snapshot")
    if (scope["schema"] != INPUT_SCOPE_SCHEMA
            or scope["target_oid"] != plan["tested_commit_oid"]
            or scope["target_tree_oid"] != plan["tested_tree_oid"]
            or scope["planner_inventory_issuer"] != issuer
            or scope["required_test_units"] != required_units
            or scope["input_fingerprints"] != fingerprints
            or scope["product_corpus"] != plan["product_corpus"]):
        raise RequiredArtifactError("input-scope snapshot differs from the full planner inventory")
    closure_status = plan["closure_status"]
    embedded_closure = scope["closure_status"].get("status") if isinstance(scope["closure_status"], dict) else None
    if closure_status != embedded_closure or closure_status not in ("complete", "unknown"):
        raise RequiredArtifactError("required-plan closure status differs from its input-scope snapshot")
    if require_complete and closure_status != "complete":
        raise RequiredArtifactError("complete inventory closure is required for passed results")

    planner_output = plan["planner_output"]
    if (not isinstance(planner_output, dict)
            or any(not isinstance(key, str) or not isinstance(value, str) for key, value in planner_output.items())):
        raise RequiredArtifactError("required-plan W planner output is malformed")
    if (planner_output.get("source_scope_base") != plan["source_scope_oid"]
            or planner_output.get("integration_base") != plan["integration_base_oid"]
            or planner_output.get("source_head") != plan["source_head_oid"]
            or planner_output.get("impact_projection_digest") != plan["source_projection_digest"]
            or planner_output.get("planner_config_sha256") != plan["planner_config_sha256"]):
        raise RequiredArtifactError("required-plan planner projection differs from its identity")
    if planner_output.get("required_test_units", "").split(";") != sorted(
        unit_id for unit_id in unit_ids if not unit_id.startswith("product-")
    ):
        raise RequiredArtifactError("W planner output test units differ from the selected capabilities")
    invocation = _validate_planner_invocation(plan["planner_invocation"], plan=plan)
    if (invocation["planner_output_sha256"] != canonical_digest(planner_output)
            or invocation["changed_paths"] != sorted(invocation["changed_paths"])):
        raise RequiredArtifactError("planner invocation does not canonically bind its W output")

    requirements = execution_job_requirements(unit_ids)
    if plan["execution_job_requirements"] != requirements:
        raise RequiredArtifactError("execution job requirements differ from the trusted unit contract")
    return plan


def build_plan_payload(value: Any) -> dict[str, Any]:
    """Validate an assembled plan; unknown/incomplete closure may not authorize results."""
    return _validate_plan(value, require_complete=False)


def validate_plan_payload(value: Any, *, require_complete: bool = False) -> dict[str, Any]:
    return _validate_plan(value, require_complete=require_complete)


def assemble_plan_payload(
    inventory: Any, *, request_identity: Any, request_key: Any,
    trusted_policy_context: Any, gate_job: Any, workflow_ref: str,
    workflow_sha: str, workflow_run_id: int, run_attempt: int,
) -> dict[str, Any]:
    """Assemble a v2 plan from independently obtained W, policy, and job inputs."""
    if not isinstance(inventory, dict) or inventory.get("schema") != "oasis7-required-test-inventory/v1":
        raise RequiredArtifactError("trusted W required-test inventory is unavailable")
    if (not isinstance(trusted_policy_context, dict)
            or trusted_policy_context.get("schema") != "oasis7-trusted-ci-reuse-policy-context/v1"):
        raise RequiredArtifactError("trusted W policy context is unavailable")
    identity = validate_request_identity(request_identity)
    if request_key != request_key_for_identity(identity):
        raise RequiredArtifactError("request key does not match the canonical request identity")
    for field, expected in (("repository", identity["repository"]),
                            ("workflow_ref", workflow_ref), ("workflow_sha", workflow_sha)):
        if trusted_policy_context.get(field) != expected:
            raise RequiredArtifactError(f"trusted policy context {field} differs from the run")
    policy = trusted_policy_context.get("effective_policy")
    if not isinstance(policy, dict) or set(policy) != {
        "enabled_capabilities", "approved_executor_contract_digests", "check_app_id",
    }:
        raise RequiredArtifactError("trusted effective policy fields are malformed")
    if CAPABILITY not in policy["enabled_capabilities"]:
        raise RequiredArtifactError("input-scope-reuse/v1 is disabled by trusted workflow policy")
    policy_identity = trusted_policy_context.get("effective_policy_identity")
    policy_identity = _require_fields(policy_identity, {"schema", "digest"}, "effective policy identity")
    if (policy_identity["schema"] != POLICY_IDENTITY_SCHEMA
            or policy_identity["digest"] != identity["effective_policy_digest"]):
        raise RequiredArtifactError("request and trusted policy identity differ")
    _positive_int(policy["check_app_id"], "trusted policy check_app_id")

    live_job = _validate_job(gate_job)
    if (live_job["job_name"] != REQUIRED_CHECK or live_job["check_name"] != REQUIRED_CHECK
            or live_job["workflow_run_id"] != workflow_run_id
            or live_job["run_attempt"] != run_attempt
            or live_job["check_app_id"] != policy["check_app_id"]
            or live_job["head_sha"] != workflow_sha
            or live_job["status"] != "in_progress"
            or live_job["conclusion"] is not None):
        raise RequiredArtifactError("live required-gate check is not the exact in-progress W attempt")

    source_scope_oid = inventory.get("planner_output", {}).get("source_scope_base")
    planner_output = inventory.get("planner_output")
    if not isinstance(planner_output, dict):
        raise RequiredArtifactError("trusted W planner output is missing")
    issuer = _validate_issuer(inventory.get("planner_inventory_issuer"))
    authority = _validate_authority(trusted_policy_context.get("planner_inventory_authority"))
    if authority != issuer["authority"]:
        raise RequiredArtifactError("planner inventory issuer differs from trusted W authority")
    if (issuer["producer"] != {
            "run_id": workflow_run_id, "run_attempt": run_attempt,
            "check_app_id": live_job["check_app_id"], "check_run_id": live_job["check_run_id"],
    }):
        raise RequiredArtifactError("planner inventory producer differs from the live attempt/check")
    specs = inventory.get("unit_specs")
    corpus = inventory.get("product_corpus")
    scope = inventory.get("input_scope")
    if not isinstance(specs, list) or not isinstance(corpus, dict) or not isinstance(scope, dict):
        raise RequiredArtifactError("trusted planner inventory closure is incomplete")
    c2_path = __import__("pathlib").Path(__file__).with_name("ci_input_scope.py")
    import importlib.util
    spec = importlib.util.spec_from_file_location("trusted_ci_input_scope_v2", c2_path)
    if spec is None or spec.loader is None:
        raise RequiredArtifactError("trusted C2 inventory validator is unavailable")
    c2 = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(c2)
    inventory_digest = c2.planner_inventory_digest(
        specs, corpus, issuer["target_oid"], issuer["target_tree_oid"],
    )
    if inventory_digest != issuer["inventory_digest"]:
        raise RequiredArtifactError("trusted planner inventory digest mismatch")
    if (scope.get("planner_inventory_issuer") != issuer
            or scope.get("target_oid") != issuer["target_oid"]
            or scope.get("target_tree_oid") != issuer["target_tree_oid"]):
        raise RequiredArtifactError("input-scope snapshot differs from the planner inventory issuer")
    if (sorted(unit.get("unit_id") for unit in specs if isinstance(unit, dict)) != issuer["unit_ids"]
            or issuer["target_oid"] != inventory.get("input_scope", {}).get("target_oid")):
        raise RequiredArtifactError("planner inventory unit or M/T closure is inconsistent")
    if set(identity["unit_ids"]) != set(issuer["unit_ids"]):
        raise RequiredArtifactError("request unit IDs differ from the complete planner inventory")
    if workflow_ref != authority["workflow_ref"] or workflow_sha != authority["planner_authority_oid"]:
        raise RequiredArtifactError("trusted planner authority differs from workflow W")
    if issuer["authority"]["planner_config_sha256"] != inventory.get("planner_config_sha256"):
        raise RequiredArtifactError("trusted planner configuration digest differs")
    invocation = inventory.get("planner_invocation")
    unit_ids = issuer["unit_ids"]
    fingerprints = scope.get("input_fingerprints")
    required_units = scope.get("required_test_units")
    if not isinstance(fingerprints, dict) or not isinstance(required_units, list):
        raise RequiredArtifactError("input-scope unit fingerprint closure is incomplete")
    if (not isinstance(source_scope_oid, str)
            or planner_output.get("impact_projection_digest") != identity["source_projection_digest"]):
        raise RequiredArtifactError("W planner output differs from request source projection")
    plan = {
        "schema": PLAN_SCHEMA,
        "required_capabilities": [CAPABILITY],
        "request_key": request_key,
        "request_identity": identity,
        "repository": identity["repository"],
        "task_uid": identity["task_uid"],
        "pr_number": identity["pr_number"],
        "bootstrap_epoch": identity["bootstrap_epoch"],
        "source_head_oid": identity["source_head_oid"],
        "source_scope_oid": source_scope_oid,
        "source_projection_digest": identity["source_projection_digest"],
        "integration_base_oid": inventory["planner_invocation"]["base_ref"],
        "tested_commit_oid": issuer["target_oid"],
        "tested_tree_oid": issuer["target_tree_oid"],
        "workflow_ref": workflow_ref,
        "workflow_sha": workflow_sha,
        "workflow_run_id": workflow_run_id,
        "run_attempt": run_attempt,
        "check_name": live_job["check_name"],
        "check_app_id": live_job["check_app_id"],
        "check_run_id": live_job["check_run_id"],
        "job_id": live_job["job_id"],
        "job_name": live_job["job_name"],
        "executor_contract_digest": identity["executor_contract_digest"],
        "effective_policy_identity": policy_identity,
        "planner_inventory_authority": authority,
        "planner_inventory_issuer": issuer,
        "planner_inventory_digest": inventory_digest,
        "planner_config_sha256": inventory["planner_config_sha256"],
        "planner_invocation": invocation,
        "unit_ids": unit_ids,
        "required_test_units": required_units,
        "input_fingerprints": fingerprints,
        "unit_specs": specs,
        "product_corpus": corpus,
        "input_scope": scope,
        "planner_output": planner_output,
        "execution_job_requirements": execution_job_requirements(unit_ids),
        "closure_status": scope.get("closure_status", {}).get("status"),
    }
    return build_plan_payload(plan)


def required_obligations(plan: dict[str, Any], unit_id: str) -> list[str]:
    matching = [spec for spec in plan["unit_specs"] if spec["unit_id"] == unit_id]
    if len(matching) != 1 or not isinstance(matching[0].get("obligation_set"), list):
        raise RequiredArtifactError(f"unit obligations are missing or ambiguous: {unit_id}")
    obligations = matching[0]["obligation_set"]
    if not obligations or any(not isinstance(item, str) or not item for item in obligations):
        raise RequiredArtifactError(f"unit obligations are incomplete: {unit_id}")
    return sorted(obligations)


def select_execution_jobs(plan: Any, unit_id: Any, live_jobs: Any) -> list[dict[str, Any]]:
    bound_plan = validate_plan_payload(plan, require_complete=True)
    _string(unit_id, "unit_id")
    if unit_id not in bound_plan["required_test_units"]:
        raise RequiredArtifactError("result unit is outside required_test_units")
    if not isinstance(live_jobs, list):
        raise RequiredArtifactError("live execution jobs must be a list")
    jobs = [_validate_job(job) for job in live_jobs]
    expected_names = [REQUIRED_CHECK, *bound_plan["execution_job_requirements"][unit_id]]
    selected: list[dict[str, Any]] = []
    for name in expected_names:
        matches = [job for job in jobs if job["job_name"] == name]
        if len(matches) != 1:
            raise RequiredArtifactError(f"exact successful execution job is unavailable: {name}")
        selected.append(matches[0])
    return sorted(selected, key=lambda item: item["job_name"])


def build_result_payload(
    plan: Any, plan_artifact_id: Any, unit_id: Any, execution_jobs: Any,
) -> dict[str, Any]:
    bound_plan = validate_plan_payload(plan, require_complete=True)
    _positive_int(plan_artifact_id, "plan_artifact_id")
    _string(unit_id, "unit_id")
    if unit_id not in bound_plan["required_test_units"]:
        raise RequiredArtifactError("result unit is outside required_test_units")
    if not isinstance(execution_jobs, list) or any(not isinstance(job, dict) for job in execution_jobs):
        raise RequiredArtifactError("result execution_jobs must be a list")
    jobs = [_validate_job(job) for job in execution_jobs]
    names = [job["job_name"] for job in jobs]
    expected_names = sorted([REQUIRED_CHECK, *bound_plan["execution_job_requirements"][unit_id]])
    if sorted(names) != expected_names or len(names) != len(set(names)):
        raise RequiredArtifactError("unit execution job proof is incomplete or ambiguous")
    for job in jobs:
        if (job["workflow_run_id"] != bound_plan["workflow_run_id"]
                or job["run_attempt"] != bound_plan["run_attempt"]
                or job["head_sha"] != bound_plan["workflow_sha"]
                or job["status"] != "completed" or job["conclusion"] != "success"):
            raise RequiredArtifactError("unit execution job is not successful for the exact attempt")
        if job["job_name"] == REQUIRED_CHECK and (
            job["job_id"] != bound_plan["job_id"] or job["check_name"] != bound_plan["check_name"]
            or job["check_app_id"] != bound_plan["check_app_id"]
            or job["check_run_id"] != bound_plan["check_run_id"]
        ):
            raise RequiredArtifactError("required-gate result job differs from the plan check")
    obligations = required_obligations(bound_plan, unit_id)
    result = {field: bound_plan[field] for field in (
        "request_key", "request_identity", "repository", "task_uid", "pr_number", "bootstrap_epoch",
        "source_head_oid", "source_scope_oid", "source_projection_digest", "integration_base_oid",
        "tested_commit_oid", "tested_tree_oid", "workflow_ref", "workflow_sha", "workflow_run_id",
        "run_attempt", "check_name", "check_app_id", "check_run_id", "job_id", "job_name",
        "executor_contract_digest", "effective_policy_identity", "planner_inventory_digest",
        "planner_inventory_issuer",
    )}
    result.update({
        "schema": RESULT_SCHEMA, "plan_artifact_id": plan_artifact_id,
        "unit_id": unit_id, "obligation_ids": obligations,
        "input_digest": bound_plan["input_fingerprints"][unit_id],
        "status": "passed", "disposition": "executed", "exit_code": 0,
        "execution_jobs": sorted(jobs, key=lambda item: item["job_name"]),
    })
    return validate_result_payload(
        result, plan=bound_plan, plan_artifact_id=plan_artifact_id, expected_unit_id=unit_id,
    )


def validate_result_payload(
    value: Any, *, plan: Any, plan_artifact_id: Any, expected_unit_id: Any,
) -> dict[str, Any]:
    bound_plan = validate_plan_payload(plan, require_complete=True)
    result = _require_fields(value, RESULT_FIELDS, "required-result v2")
    _positive_int(plan_artifact_id, "plan_artifact_id")
    if result["schema"] != RESULT_SCHEMA or result["status"] != "passed" or result["disposition"] != "executed" or result["exit_code"] != 0:
        raise RequiredArtifactError("only a successful executed result is valid")
    if (result["plan_artifact_id"] != plan_artifact_id
            or type(result["plan_artifact_id"]) is not int
            or result["unit_id"] != expected_unit_id):
        raise RequiredArtifactError("result does not identify the exact plan artifact and unit")
    if result["unit_id"] not in bound_plan["required_test_units"]:
        raise RequiredArtifactError("result unit is outside the required plan")
    if any(result.get(field) != bound_plan[field] for field in (
        "request_key", "request_identity", "repository", "task_uid", "pr_number", "bootstrap_epoch",
        "source_head_oid", "source_scope_oid", "source_projection_digest", "integration_base_oid",
        "tested_commit_oid", "tested_tree_oid", "workflow_ref", "workflow_sha", "workflow_run_id",
        "run_attempt", "check_name", "check_app_id", "check_run_id", "job_id", "job_name",
        "executor_contract_digest", "effective_policy_identity", "planner_inventory_digest",
        "planner_inventory_issuer",
    )):
        raise RequiredArtifactError("result identity differs from its exact required plan")
    unit_id = result["unit_id"]
    if result["obligation_ids"] != required_obligations(bound_plan, unit_id):
        raise RequiredArtifactError("result obligation set differs from the trusted unit spec")
    if result["input_digest"] != bound_plan["input_fingerprints"].get(unit_id):
        raise RequiredArtifactError("result input digest differs from the required plan")
    if not isinstance(result["execution_jobs"], list):
        raise RequiredArtifactError("result execution_jobs must be a list")
    jobs = [_validate_job(job) for job in result["execution_jobs"]]
    expected_names = sorted([REQUIRED_CHECK, *bound_plan["execution_job_requirements"][unit_id]])
    names = [job["job_name"] for job in jobs]
    if sorted(names) != expected_names or len(names) != len(set(names)):
        raise RequiredArtifactError("result execution job proof is incomplete or ambiguous")
    for job in jobs:
        if (job["workflow_run_id"] != bound_plan["workflow_run_id"]
                or job["run_attempt"] != bound_plan["run_attempt"]
                or job["head_sha"] != bound_plan["workflow_sha"]
                or job["status"] != "completed" or job["conclusion"] != "success"):
            raise RequiredArtifactError("result job proof is not successful for the exact attempt")
        if job["job_name"] == REQUIRED_CHECK and (
            job["job_id"] != bound_plan["job_id"] or job["check_app_id"] != bound_plan["check_app_id"]
            or job["check_run_id"] != bound_plan["check_run_id"]
        ):
            raise RequiredArtifactError("result required-gate job differs from its plan")
    return result


def parse_payload(raw: bytes, *, label: str) -> dict[str, Any]:
    """Parse exactly one canonical JSON object, rejecting duplicate keys."""
    if not isinstance(raw, bytes):
        raise RequiredArtifactError(f"{label} payload is not bytes")

    def object_pairs(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise RequiredArtifactError(f"{label} payload has duplicate JSON keys")
            result[key] = value
        return result

    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=object_pairs,
                           parse_constant=lambda item: (_ for _ in ()).throw(ValueError(item)))
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise RequiredArtifactError(f"{label} payload is malformed JSON") from exc
    if not isinstance(value, dict) or raw not in (canonical_bytes(value), canonical_bytes(value) + b"\n"):
        raise RequiredArtifactError(f"{label} payload is not one canonical JSON object")
    return value


def _write_json(path: str | Path, value: Any) -> None:
    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(canonical_bytes(value) + b"\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    plan_parser = subparsers.add_parser("build-plan")
    plan_parser.add_argument("--inventory", required=True)
    plan_parser.add_argument("--validation-request-b64", required=True)
    plan_parser.add_argument("--request-key", required=True)
    plan_parser.add_argument("--trusted-policy-json", required=True)
    plan_parser.add_argument("--execution-jobs", required=True)
    plan_parser.add_argument("--workflow-ref", required=True)
    plan_parser.add_argument("--workflow-sha", required=True)
    plan_parser.add_argument("--workflow-run-id", required=True, type=int)
    plan_parser.add_argument("--run-attempt", required=True, type=int)
    plan_parser.add_argument("--output", required=True)
    plan_parser.add_argument("--github-output", required=True)
    result_parser = subparsers.add_parser("build-result")
    result_parser.add_argument("--plan", required=True)
    result_parser.add_argument("--plan-artifact-id", required=True, type=int)
    result_parser.add_argument("--execution-jobs", required=True)
    result_parser.add_argument("--unit-id", required=True)
    result_parser.add_argument("--output", required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "build-plan":
            inventory = json.loads(Path(args.inventory).read_text(encoding="utf-8"))
            policy = json.loads(Path(args.trusted_policy_json).read_text(encoding="utf-8"))
            jobs = json.loads(Path(args.execution_jobs).read_text(encoding="utf-8"))
            if not isinstance(jobs, dict) or not isinstance(jobs.get("execution_jobs"), list):
                raise RequiredArtifactError("current attempt job proof is malformed")
            encoded = args.validation_request_b64
            try:
                raw = base64.b64decode(encoded, validate=True)
                envelope = json.loads(raw.decode("utf-8"))
            except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise RequiredArtifactError("validation request envelope is malformed") from exc
            executor_path = Path(__file__).with_name("integration_executor_contract.py")
            import importlib.util
            spec = importlib.util.spec_from_file_location("v2_request_contract", executor_path)
            if spec is None or spec.loader is None:
                raise RequiredArtifactError("trusted validation-request contract is unavailable")
            contract = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(contract)
            normalized = contract.validation_request_envelope(envelope)
            if raw != contract.canonical_bytes(normalized) or normalized["request_key"] != args.request_key:
                raise RequiredArtifactError("validation request is noncanonical or has another key")
            gates = [job for job in jobs["execution_jobs"] if job.get("job_name") == REQUIRED_CHECK]
            if len(gates) != 1:
                raise RequiredArtifactError("exact current required-gate job proof is missing")
            plan = assemble_plan_payload(
                inventory, request_identity=normalized["identity"],
                request_key=normalized["request_key"], trusted_policy_context=policy,
                gate_job=gates[0], workflow_ref=args.workflow_ref,
                workflow_sha=args.workflow_sha, workflow_run_id=args.workflow_run_id,
                run_attempt=args.run_attempt,
            )
            _write_json(args.output, plan)
            units = [
                {"unit_id": unit, "artifact_name": result_artifact_name(
                    args.workflow_run_id, args.run_attempt, unit,
                )}
                for unit in plan["required_test_units"]
            ]
            output = Path(args.github_output)
            with output.open("a", encoding="utf-8") as stream:
                stream.write("unit_matrix=" + json.dumps(units, sort_keys=True, separators=(",", ":")) + "\n")
        else:
            plan = parse_payload(Path(args.plan).read_bytes(), label="required-plan v2")
            jobs = json.loads(Path(args.execution_jobs).read_text(encoding="utf-8"))
            if not isinstance(jobs, dict) or not isinstance(jobs.get("execution_jobs"), list):
                raise RequiredArtifactError("completed attempt job proof is malformed")
            unit_jobs = select_execution_jobs(plan, args.unit_id, jobs["execution_jobs"])
            result = build_result_payload(plan, args.plan_artifact_id, args.unit_id, unit_jobs)
            _write_json(args.output, result)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        print(f"ci-required-artifact-v2: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
