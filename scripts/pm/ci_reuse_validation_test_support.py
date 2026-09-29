"""Exact validation-only artifacts for production-boundary rejection tests.

This test support intentionally builds records through the frozen validation
contract fixture, then proves each candidate is valid before production
consumers are asked to reject it.
"""
from __future__ import annotations

import ci_reuse_validation_contract as contract


def validation_only_artifacts() -> tuple[dict, dict, dict]:
    """Return a valid validation payload, authority record, and readback."""
    task_uid = "task_" + "a" * 32
    units = ["required_gate_baseline", "workflow_governance"]
    obligations = {
        "required_gate_baseline": ("required_gate_baseline:000:baseline",),
        "workflow_governance": ("workflow_governance:000:contracts",),
    }
    context = contract.TrustedRequestContext(
        task_uid=task_uid,
        head_oid="1" * 40,
        source_scope_oid="2" * 40,
        projection_digest="sha256:" + "3" * 64,
        planner_unit_ids=tuple(units),
        planner_unit_obligations=obligations,
    )

    authorization = {
        "schema": contract.AUTHORIZATION_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": task_uid,
        "task_issue_number": contract.TASK_ISSUE_NUMBER,
        "pr_number": contract.PR_NUMBER,
        "head_oid": context.head_oid,
        "integration_base_oid": "4" * 40,
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": units,
        "purpose": contract.PURPOSE,
        "decision": contract.REQUEST_DECISION,
    }
    authorization_body = (
        contract.AUTHORIZATION_MARKER + "\n"
        + contract.canonical_json_bytes(authorization).decode("ascii")
    )
    authorization_body_digest = contract.body_digest(authorization_body)
    request = {
        "schema": contract.REQUEST_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": task_uid,
        "task_issue_number": contract.TASK_ISSUE_NUMBER,
        "pr_number": contract.PR_NUMBER,
        "head_oid": context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": units,
        "purpose": contract.PURPOSE,
        "authorization_decision": contract.REQUEST_DECISION,
        "authorization_source": {
            "issue_number": contract.TASK_ISSUE_NUMBER,
            "comment_id": 10,
            "body_digest": authorization_body_digest,
        },
        "authorized_actor": "approval-admin",
    }
    request["request_digest"] = contract.request_digest(request)
    request_body = (
        contract.REQUEST_MARKER + "\n"
        + contract.canonical_json_bytes(request).decode("ascii")
    )
    pin = {
        "schema": contract.PIN_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": task_uid,
        "task_issue_number": contract.TASK_ISSUE_NUMBER,
        "pr_number": contract.PR_NUMBER,
        "request_comment_id": 20,
        "request_body_digest": contract.body_digest(request_body),
        "request_digest": request["request_digest"],
        "purpose": contract.PIN_PURPOSE,
    }
    pin_body = (
        contract.PIN_MARKER + "\n"
        + contract.canonical_json_bytes(pin).decode("ascii")
    )
    comments = [
        {
            "id": 10, "body": authorization_body,
            "user": {"login": "approval-admin"},
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
        },
        {
            "id": 20, "body": request_body,
            "user": {"login": "requester"},
            "created_at": "2026-01-01T00:01:00Z",
            "updated_at": "2026-01-01T00:01:00Z",
        },
        {
            "id": 30, "body": pin_body,
            "user": {"login": "pin-admin"},
            "created_at": "2026-01-01T00:02:00Z",
            "updated_at": "2026-01-01T00:02:00Z",
        },
    ]
    permissions = {
        "approval-admin": {"login": "approval-admin", "permission": "admin"},
        "pin-admin": {"login": "pin-admin", "permission": "admin"},
    }
    authority = contract.resolve_authority(comments, context, permissions)

    run = {
        "id": 700,
        "repository": contract.REPOSITORY,
        "workflow_id": 900,
        "workflow_path": contract.WORKFLOW_PATH,
        "workflow_ref": contract.WORKFLOW_REF,
        "workflow_sha": "5" * 40,
        "event": "workflow_dispatch",
        "display_title": contract.expected_run_title(authority),
        "dispatched_head_sha": "5" * 40,
        "inputs": {
            "run_mode": contract.RUN_MODE,
            "task_uid": task_uid,
            "pr_number": str(contract.PR_NUMBER),
            "integration_base": request["integration_base_oid"],
            "expected_head": request["head_oid"],
            "source_scope_oid": request["source_scope_oid"],
            "projection_digest": request["projection_digest"],
            "request_key": authority.validation_id,
        },
        "run_attempt": 1,
        "status": "completed",
        "conclusion": "success",
        "tested_merge_oid": "6" * 40,
        "tested_tree_oid": "7" * 40,
    }
    check = {
        "name": contract.CHECK_NAME,
        "id": 801,
        "app_id": contract.GITHUB_ACTIONS_APP_ID,
        "head_sha": run["dispatched_head_sha"],
        "run_id": run["id"],
        "run_attempt": run["run_attempt"],
        "status": "completed",
        "conclusion": "success",
    }
    record = contract.build_authority_record(authority, run)
    payload = {
        **record,
        "schema": contract.PAYLOAD_SCHEMA,
        "run_attempt": run["run_attempt"],
        "check_name": check["name"],
        "check_run_id": check["id"],
        "check_app_id": check["app_id"],
        "selected_obligations": {key: list(value) for key, value in obligations.items()},
        "result_digests": {
            unit: "sha256:" + str(index) * 64
            for index, unit in enumerate(units, start=1)
        },
        "authority_digest": contract.authority_digest(record),
        "capability_under_test": contract.CAPABILITY,
        "tested_merge_oid": run["tested_merge_oid"],
        "tested_tree_oid": run["tested_tree_oid"],
        "event_inputs": dict(run["inputs"]),
        "run_id": run["id"],
        "workflow_id": run["workflow_id"],
        "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"],
        "workflow_sha": run["workflow_sha"],
        "display_title": run["display_title"],
        "event": run["event"],
        "dispatched_head_sha": run["dispatched_head_sha"],
    }
    payload = contract.verify_payload(payload, authority, run, check)

    payload_bytes = contract.canonical_json_bytes(payload)
    artifact_bytes = b"frozen-validation-artifact-bytes"
    artifact = {
        "id": 910,
        "name": contract.artifact_name(
            authority.validation_id, run["id"], run["run_attempt"],
        ),
    }
    envelope = {
        "schema": contract.READBACK_SCHEMA,
        "authority_digest": contract.authority_digest(record),
        "validation_id": authority.validation_id,
        "run_id": run["id"],
        "run_attempt": run["run_attempt"],
        "workflow_id": run["workflow_id"],
        "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"],
        "workflow_sha": run["workflow_sha"],
        "event": run["event"],
        "display_title": run["display_title"],
        "dispatched_head_sha": run["dispatched_head_sha"],
        "check_name": check["name"],
        "check_run_id": check["id"],
        "check_app_id": check["app_id"],
        "artifact_id": artifact["id"],
        "artifact_name": artifact["name"],
        "artifact_content_digest": contract.body_digest(artifact_bytes),
        "payload_digest": contract.body_digest(payload_bytes),
    }
    contract.verify_readback(
        envelope, authority, run, check, artifact, payload_bytes, artifact_bytes,
    )
    return payload, record, envelope
