"""Exact validation-only artifacts for production-boundary rejection tests.

This test support intentionally builds records through the frozen validation
contract fixture, then proves each candidate is valid before production
consumers are asked to reject it.
"""
from __future__ import annotations

import ci_reuse_validation_contract as contract


def _successor_artifacts(context, old_request, comments, permissions, old_authority,
                         units, obligations) -> tuple[dict, dict, dict]:
    """Build a fully valid V2 successor candidate for production rejection tests."""
    old_workflow = contract.normalize_workflow_identity("main", contract.WORKFLOW_FILE)
    predecessor = {
        "schema": contract.PREDECESSOR_OBSERVATION_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "head_oid": old_request["head_oid"],
        "integration_base_oid": old_request["integration_base_oid"],
        "request_comment_id": old_authority.request_comment_id,
        "request_body_digest": old_authority.request_body_digest,
        "request_digest": old_request["request_digest"],
        "authorization_comment_id": old_authority.authorization_comment_id,
        "authorization_body_digest": old_authority.authorization_body_digest,
        "pin_comment_id": old_authority.pin_comment_id,
        "pin_body_digest": old_authority.pin_body_digest,
        "validation_id": old_authority.validation_id,
        "workflow_id": 900,
        **old_workflow,
        "event": "workflow_dispatch",
        "display_title": contract.expected_run_title(old_authority),
        "dispatched_head_sha": "5" * 40,
        "run_id": 700,
        "run_attempt": 1,
        "run_status": "completed",
        "run_conclusion": "failure",
        "run_head_sha": "5" * 40,
        "run_terminal_updated_at": "2026-01-01T00:03:00Z",
        "workflow_blob_oid": "a" * 40,
        "workflow_blob_digest": "sha256:" + "b" * 64,
    }
    successor_workflow_identity = contract.normalize_workflow_identity(
        "release/next", contract.WORKFLOW_FILE,
    )
    successor_workflow = {
        "workflow_id": 901,
        **successor_workflow_identity,
        "workflow_sha": "c" * 40,
    }
    predecessor_digest = contract.body_digest(contract.canonical_json_bytes(predecessor))
    successor_workflow_digest = contract.body_digest(
        contract.canonical_json_bytes(successor_workflow),
    )
    authorization = {
        "schema": contract.SUCCESSOR_AUTHORIZATION_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "head_oid": "8" * 40,
        "integration_base_oid": "9" * 40,
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": list(old_request["validation_units"]),
        "purpose": contract.SUCCESSOR_PURPOSE,
        "decision": contract.SUCCESSOR_REQUEST_DECISION,
        "successor_sequence": 1,
        "reason": "validate the authorized current workflow after predecessor failure",
        "predecessor_digest": predecessor_digest,
        "successor_workflow_digest": successor_workflow_digest,
    }
    authorization_body = (
        contract.SUCCESSOR_AUTHORIZATION_MARKER + "\n"
        + contract.canonical_json_bytes(authorization).decode("ascii")
    )
    request = {
        "schema": contract.SUCCESSOR_REQUEST_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "head_oid": "8" * 40,
        "integration_base_oid": "9" * 40,
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": list(old_request["validation_units"]),
        "purpose": contract.SUCCESSOR_PURPOSE,
        "authorization_decision": contract.SUCCESSOR_REQUEST_DECISION,
        "authorization_source": {
            "issue_number": context.task_issue_number,
            "comment_id": 40,
            "body_digest": contract.body_digest(authorization_body),
        },
        "authorized_actor": "approval-admin",
        "successor_sequence": 1,
        "reason": authorization["reason"],
        "predecessor": predecessor,
        "predecessor_digest": predecessor_digest,
        "successor_workflow": successor_workflow,
        "successor_workflow_digest": successor_workflow_digest,
    }
    request["request_digest"] = contract.request_digest(request)
    request_body = (
        contract.SUCCESSOR_REQUEST_MARKER + "\n"
        + contract.canonical_json_bytes(request).decode("ascii")
    )
    pin = {
        "schema": contract.SUCCESSOR_PIN_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "request_comment_id": 50,
        "request_body_digest": contract.body_digest(request_body),
        "request_digest": request["request_digest"],
        "purpose": contract.SUCCESSOR_PIN_PURPOSE,
        "successor_sequence": 1,
        "predecessor_digest": predecessor_digest,
        "successor_workflow_digest": successor_workflow_digest,
    }
    successor_comments = [
        {"id": 40, "body": authorization_body, "user": {"login": "approval-admin"},
         "created_at": "2026-01-01T00:04:00Z", "updated_at": "2026-01-01T00:04:00Z"},
        {"id": 50, "body": contract.SUCCESSOR_REQUEST_MARKER + "\n"
         + contract.canonical_json_bytes(request).decode("ascii"),
         "user": {"login": "requester"}, "created_at": "2026-01-01T00:05:00Z",
         "updated_at": "2026-01-01T00:05:00Z"},
        {"id": 60, "body": contract.SUCCESSOR_PIN_MARKER + "\n"
         + contract.canonical_json_bytes(pin).decode("ascii"),
         "user": {"login": "pin-admin"}, "created_at": "2026-01-01T00:06:00Z",
         "updated_at": "2026-01-01T00:06:00Z"},
    ]
    identity = {
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "head_oid": request["head_oid"],
        "integration_base_oid": request["integration_base_oid"],
    }
    authority = contract.resolve_successor_records(
        [*comments, *successor_comments], identity, permissions, predecessor,
    )
    context = contract.TrustedRequestContext(
        task_uid=context.task_uid,
        task_issue_number=context.task_issue_number,
        pr_number=context.pr_number,
        head_oid=request["head_oid"],
        source_scope_oid=context.source_scope_oid,
        projection_digest=context.projection_digest,
        planner_unit_ids=context.planner_unit_ids,
        planner_unit_obligations=context.planner_unit_obligations,
    )
    authority = contract.bind_authority_context(authority, context)
    run = {
        "id": 701,
        "repository": contract.REPOSITORY,
        "workflow_id": successor_workflow["workflow_id"],
        **successor_workflow_identity,
        "workflow_sha": successor_workflow["workflow_sha"],
        "event": "workflow_dispatch",
        "display_title": contract.expected_run_title(authority),
        "dispatched_head_sha": successor_workflow["workflow_sha"],
        "run_attempt": 1,
        "status": "completed",
        "conclusion": "success",
        "tested_merge_oid": "d" * 40,
        "tested_tree_oid": "e" * 40,
    }
    check = {
        "name": contract.CHECK_NAME, "id": 802,
        "app_id": contract.GITHUB_ACTIONS_APP_ID,
        "head_sha": run["dispatched_head_sha"], "run_id": run["id"],
        "run_attempt": run["run_attempt"], "status": "completed", "conclusion": "success",
    }
    record = contract.build_authority_record(authority, run)
    payload = {
        **record,
        "schema": contract.SUCCESSOR_PAYLOAD_SCHEMA,
        "authority_digest": contract.authority_digest(record),
        "capability_under_test": contract.CAPABILITY,
        "tested_merge_oid": run["tested_merge_oid"],
        "tested_tree_oid": run["tested_tree_oid"],
        "run_attempt": run["run_attempt"],
        "event_inputs": contract.expected_event_inputs(authority),
        "check_name": check["name"], "check_run_id": check["id"],
        "check_app_id": check["app_id"],
        "selected_obligations": {key: list(value) for key, value in obligations.items()},
        "result_digests": {
            unit: "sha256:" + str(index) * 64
            for index, unit in enumerate(units, start=1)
        },
    }
    payload = contract.verify_payload(payload, authority, run, check)
    payload_bytes = contract.canonical_json_bytes(payload)
    artifact_bytes = b"frozen-v2-validation-artifact-bytes"
    artifact = {
        "id": 911,
        "name": contract.artifact_name(
            authority.validation_id, run["id"], run["run_attempt"], successor=True,
        ),
    }
    envelope = {
        "schema": contract.SUCCESSOR_READBACK_SCHEMA,
        "authority_digest": contract.authority_digest(record),
        "validation_id": authority.validation_id,
        "run_id": run["id"], "run_attempt": run["run_attempt"],
        "workflow_id": run["workflow_id"],
        "workflow_api_path": run["workflow_api_path"],
        "workflow_default_branch": run["workflow_default_branch"],
        "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"], "event_ref": run["event_ref"],
        "workflow_sha": run["workflow_sha"], "event": run["event"],
        "display_title": run["display_title"],
        "dispatched_head_sha": run["dispatched_head_sha"],
        "check_name": check["name"], "check_run_id": check["id"],
        "check_app_id": check["app_id"], "artifact_id": artifact["id"],
        "artifact_name": artifact["name"],
        "artifact_content_digest": contract.body_digest(artifact_bytes),
        "payload_digest": contract.body_digest(payload_bytes),
        **{key: record[key] for key in (
            "successor_sequence", "reason", "predecessor", "predecessor_digest",
            "successor_workflow", "successor_workflow_digest",
        )},
    }
    contract.verify_readback(
        envelope, authority, run, check, artifact, payload_bytes, artifact_bytes,
    )
    return payload, record, envelope


def validation_only_artifacts() -> tuple[dict, ...]:
    """Return valid V1 and V2 validation-only candidates for rejection tests."""
    task_uid = "task_" + "a" * 32
    task_issue_number = 87
    pr_number = 143
    units = ["required_gate_baseline", "workflow_governance"]
    obligations = {
        "required_gate_baseline": ("required_gate_baseline:000:baseline",),
        "workflow_governance": ("workflow_governance:000:contracts",),
    }
    context = contract.TrustedRequestContext(
        task_uid=task_uid,
        task_issue_number=task_issue_number,
        pr_number=pr_number,
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
        "task_issue_number": task_issue_number,
        "pr_number": pr_number,
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
        "task_issue_number": task_issue_number,
        "pr_number": pr_number,
        "head_oid": context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": units,
        "purpose": contract.PURPOSE,
        "authorization_decision": contract.REQUEST_DECISION,
        "authorization_source": {
            "issue_number": task_issue_number,
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
        "task_issue_number": task_issue_number,
        "pr_number": pr_number,
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
    workflow_identity = contract.normalize_workflow_identity("main", contract.WORKFLOW_FILE)

    run = {
        "id": 700,
        "repository": contract.REPOSITORY,
        "workflow_id": 900,
        **workflow_identity,
        "workflow_sha": "5" * 40,
        "event": "workflow_dispatch",
        "display_title": contract.expected_run_title(authority),
        "dispatched_head_sha": "5" * 40,
        "inputs": {
            "run_mode": contract.RUN_MODE,
            "task_uid": task_uid,
            "pr_number": str(pr_number),
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
        "workflow_api_path": run["workflow_api_path"],
        "workflow_default_branch": run["workflow_default_branch"],
        "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"],
        "event_ref": run["event_ref"],
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
        "workflow_api_path": run["workflow_api_path"],
        "workflow_default_branch": run["workflow_default_branch"],
        "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"],
        "event_ref": run["event_ref"],
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
    return (
        payload, record, envelope,
        *_successor_artifacts(context, request, comments, permissions, authority, units, obligations),
    )
