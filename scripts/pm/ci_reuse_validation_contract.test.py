"""Adversarial tests for the non-promotable CI reuse validation contract."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).parent
FIXTURE_ISSUE_NUMBER = 87
FIXTURE_PR_NUMBER = 143
SPEC = importlib.util.spec_from_file_location(
    "ci_reuse_validation_contract", HERE / "ci_reuse_validation_contract.py",
)
contract = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = contract
SPEC.loader.exec_module(contract)


def marked(marker: str, payload: dict) -> str:
    return marker + "\n" + contract.canonical_json_bytes(payload).decode("ascii")


def validation_fixture():
    """Build live-shaped inputs for contract and downstream reader tests."""
    product_link_unit = (
        "product-link::doc/product/agents-world-simulation/"
        "agent-conversation-and-prompt-control.prd.md->"
        "doc/product/agents-world-simulation/"
        "agent-conversation-and-prompt-control.design.md#1-设计原则"
    )
    product_link_obligation = (
        "product-link:doc/product/agents-world-simulation/"
        "agent-conversation-and-prompt-control.prd.md->"
        "doc/product/agents-world-simulation/"
        "agent-conversation-and-prompt-control.design.md#1-设计原则"
    )
    context = contract.TrustedRequestContext(
        task_uid="task_" + "a" * 32,
        task_issue_number=FIXTURE_ISSUE_NUMBER,
        pr_number=FIXTURE_PR_NUMBER,
        head_oid="1" * 40,
        source_scope_oid="2" * 40,
        projection_digest="sha256:" + "3" * 64,
        # The complete trusted inventory includes product-corpus unit IDs whose
        # canonical path syntax is intentionally broader than request unit IDs.
        planner_unit_ids=(
            "product-corpus:membership",
            "product-document::doc/product/system.md",
            product_link_unit,
            "required_gate_baseline",
            "workflow_governance",
        ),
        planner_unit_obligations={
            "product-corpus:membership": ("product-corpus:membership",),
            "product-document::doc/product/system.md": ("product-document:doc/product/system.md",),
            product_link_unit: (product_link_obligation,),
            "required_gate_baseline": ("required_gate_baseline:000:baseline",),
            "workflow_governance": ("workflow_governance:000:contracts",),
        },
    )
    authorization = {
        "schema": contract.AUTHORIZATION_SCHEMA,
        "repository": "eng-cc/oasis7",
        "task_uid": context.task_uid,
        "task_issue_number": FIXTURE_ISSUE_NUMBER,
        "pr_number": FIXTURE_PR_NUMBER,
        "head_oid": context.head_oid,
        "integration_base_oid": "4" * 40,
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": ["required_gate_baseline", "workflow_governance"],
        "purpose": "v1_pre_activation_validation",
        "decision": "authorize_validation_only",
    }
    authorization_body = marked(contract.AUTHORIZATION_MARKER, authorization)
    authorization_digest = contract.body_digest(authorization_body)
    request = {
        "schema": contract.REQUEST_SCHEMA,
        "repository": "eng-cc/oasis7",
        "task_uid": context.task_uid,
        "task_issue_number": FIXTURE_ISSUE_NUMBER,
        "pr_number": FIXTURE_PR_NUMBER,
        "head_oid": context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": list(authorization["validation_units"]),
        "purpose": "v1_pre_activation_validation",
        "authorization_decision": "authorize_validation_only",
        "authorization_source": {
            "issue_number": FIXTURE_ISSUE_NUMBER,
            "comment_id": 10,
            "body_digest": authorization_digest,
        },
        "authorized_actor": "approval-admin",
    }
    request["request_digest"] = contract.request_digest(request)
    request_body = marked(contract.REQUEST_MARKER, request)
    pin = {
        "schema": contract.PIN_SCHEMA,
        "repository": "eng-cc/oasis7",
        "task_uid": context.task_uid,
        "task_issue_number": FIXTURE_ISSUE_NUMBER,
        "pr_number": FIXTURE_PR_NUMBER,
        "request_comment_id": 20,
        "request_body_digest": contract.body_digest(request_body),
        "request_digest": request["request_digest"],
        "purpose": "freeze_validation_only_request",
    }
    pin_body = marked(contract.PIN_MARKER, pin)
    comments = [
        {"id": 10, "body": authorization_body, "user": {"login": "approval-admin"},
         "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
        {"id": 20, "body": request_body, "user": {"login": "requester"},
         "created_at": "2026-01-01T00:01:00Z", "updated_at": "2026-01-01T00:01:00Z"},
        {"id": 30, "body": pin_body, "user": {"login": "pin-admin"},
         "created_at": "2026-01-01T00:02:00Z", "updated_at": "2026-01-01T00:02:00Z"},
    ]
    permissions = {
        "approval-admin": {"login": "approval-admin", "permission": "admin"},
        "pin-admin": {"login": "pin-admin", "permission": "admin"},
    }
    authority = contract.resolve_authority(comments, context, permissions)
    return context, request, authorization, pin, comments, permissions, authority


def successor_fixture():
    """Build a V2 successor over an immutable V1 failure and a new live W."""
    context, old_request, _, _, comments, permissions, old_authority = validation_fixture()
    old_workflow = contract.normalize_workflow_identity(
        "main", ".github/workflows/rust.yml@main",
    )
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
        "request_digest": old_authority.request["request_digest"],
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
        "run_attempt": 2,
        "run_status": "completed",
        "run_conclusion": "failure",
        "run_head_sha": "5" * 40,
        "run_terminal_updated_at": "2026-01-01T00:10:00Z",
        "workflow_blob_oid": "a" * 40,
        "workflow_blob_digest": "sha256:" + "b" * 64,
    }
    expected_identity = {
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "head_oid": "8" * 40,
        "integration_base_oid": "9" * 40,
    }
    successor_workflow_identity = contract.normalize_workflow_identity(
        "release/next", ".github/workflows/rust.yml@release/next",
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
        "head_oid": expected_identity["head_oid"],
        "integration_base_oid": expected_identity["integration_base_oid"],
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": list(old_request["validation_units"]),
        "purpose": contract.SUCCESSOR_PURPOSE,
        "decision": contract.SUCCESSOR_REQUEST_DECISION,
        "successor_sequence": 1,
        "reason": "validate the new workflow after fixing the failed check",
        "predecessor_digest": predecessor_digest,
        "successor_workflow_digest": successor_workflow_digest,
    }
    authorization_body = marked(contract.SUCCESSOR_AUTHORIZATION_MARKER, authorization)
    request = {
        "schema": contract.SUCCESSOR_REQUEST_SCHEMA,
        "repository": contract.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": context.task_issue_number,
        "pr_number": context.pr_number,
        "head_oid": expected_identity["head_oid"],
        "integration_base_oid": expected_identity["integration_base_oid"],
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
    request_body = marked(contract.SUCCESSOR_REQUEST_MARKER, request)
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
         "created_at": "2026-01-01T00:11:00Z", "updated_at": "2026-01-01T00:11:00Z"},
        {"id": 50, "body": request_body, "user": {"login": "requester"},
         "created_at": "2026-01-01T00:12:00Z", "updated_at": "2026-01-01T00:12:00Z"},
        {"id": 60, "body": marked(contract.SUCCESSOR_PIN_MARKER, pin), "user": {"login": "pin-admin"},
         "created_at": "2026-01-01T00:13:00Z", "updated_at": "2026-01-01T00:13:00Z"},
    ]
    return {
        "context": context, "permissions": permissions,
        "comments": comments + successor_comments,
        "predecessor": predecessor, "identity": expected_identity,
        "request": request, "authorization": authorization, "pin": pin,
        "successor_workflow": successor_workflow,
    }


def live_run(authority=None):
    if authority is None:
        *_, authority = validation_fixture()
    return {
        "repository": "eng-cc/oasis7",
        "id": 700,
        "workflow_id": 900,
        "workflow_api_path": ".github/workflows/rust.yml@main",
        "workflow_default_branch": "main",
        "workflow_path": ".github/workflows/rust.yml@main",
        "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
        "event_ref": "refs/heads/main",
        "workflow_sha": "5" * 40,
        "event": "workflow_dispatch",
        "display_title": contract.expected_run_title(authority),
        "dispatched_head_sha": "5" * 40,
        "run_attempt": 1,
        "status": "completed",
        "conclusion": "success",
        "tested_merge_oid": "6" * 40,
        "tested_tree_oid": "7" * 40,
    }


def payload_fixture(authority=None, run=None):
    if authority is None:
        *_, authority = validation_fixture()
    run = run or live_run(authority)
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
        "schema": (
            contract.SUCCESSOR_PAYLOAD_SCHEMA
            if contract.is_successor_authority(authority) else contract.PAYLOAD_SCHEMA
        ),
        "run_attempt": run["run_attempt"],
        "check_name": check["name"],
        "check_run_id": check["id"],
        "check_app_id": check["app_id"],
        "selected_obligations": {
            "required_gate_baseline": ["required_gate_baseline:000:baseline"],
            "workflow_governance": ["workflow_governance:000:contracts"],
        },
        "result_digests": {
            unit: "sha256:" + str(index) * 64
            for index, unit in enumerate(authority.request["validation_units"], start=1)
        },
        "authority_digest": contract.authority_digest(record),
        "capability_under_test": contract.CAPABILITY,
        "tested_merge_oid": run["tested_merge_oid"],
        "tested_tree_oid": run["tested_tree_oid"],
        "event_inputs": contract.expected_event_inputs(authority),
        "run_id": run["id"],
        "workflow_id": run["workflow_id"],
        "workflow_path": run["workflow_path"],
        "workflow_ref": run["workflow_ref"],
        "workflow_sha": run["workflow_sha"],
        "display_title": run["display_title"],
        "event": run["event"],
        "dispatched_head_sha": run["dispatched_head_sha"],
    }
    return run, check, payload


class CanonicalAuthorityTests(unittest.TestCase):
    def test_trusted_planner_inventory_preserves_unicode_ids_and_obligations(self):
        unit_id = (
            "product-link::doc/product/agents-world-simulation/"
            "agent-conversation-and-prompt-control.prd.md->"
            "doc/product/agents-world-simulation/"
            "agent-conversation-and-prompt-control.design.md#1-设计原则"
        )
        obligation = (
            "product-link:doc/product/agents-world-simulation/"
            "agent-conversation-and-prompt-control.prd.md->"
            "doc/product/agents-world-simulation/"
            "agent-conversation-and-prompt-control.design.md#1-设计原则"
        )
        context, request, _, _, _, _, authority = validation_fixture()
        self.assertIn(unit_id, context.planner_unit_ids)
        self.assertEqual((obligation,), context.planner_unit_obligations[unit_id])
        validated_context = contract._validate_context(context)
        self.assertIn(unit_id, validated_context[6])
        self.assertEqual((obligation,), validated_context[7][unit_id])
        self.assertEqual(set(request["validation_units"]), set(authority.planner_unit_obligations))
        self.assertEqual(
            ["required_gate_baseline", "workflow_governance"],
            request["validation_units"],
        )
        self.assertEqual(
            ["required_gate_baseline", "workflow_governance"],
            contract._validate_units(
                ["required_gate_baseline", "workflow_governance"],
                "request.validation_units",
            ),
        )
        with self.assertRaises(contract.ContractError):
            contract._validate_units([unit_id], "request.validation_units")

    def test_public_validation_authority_constructor_is_closed(self):
        with self.assertRaisesRegex(contract.ContractError, "closed resolver"):
            contract.ValidationAuthority(
                request={}, authorization={}, pin={}, request_comment_id=1,
                authorization_comment_id=2, pin_comment_id=3,
                request_body_digest="sha256:" + "0" * 64,
                authorization_body_digest="sha256:" + "1" * 64,
                pin_body_digest="sha256:" + "2" * 64,
                authorized_actor="forged", pin_actor="forged",
                approval_permission="admin", pin_permission="admin",
                permission_snapshot_bound=True, validation_id="0" * 64,
                planner_unit_obligations={}, context_bound=True,
                comment_timestamps=(),
            )

    def test_canonical_json_has_ascii_keys_and_values_and_rejects_ambiguous_types(self):
        self.assertEqual(b'{"a":1,"z":"ok"}', contract.canonical_json_bytes({"z": "ok", "a": 1}))
        for invalid in ({"x": True}, {"x": None}, {"x": 1.0}, {"x": "é"}, {1: "x"}):
            with self.subTest(invalid=invalid), self.assertRaises(contract.ContractError):
                contract.canonical_json_bytes(invalid)

    def test_request_auth_pin_and_live_admins_resolve_exactly(self):
        *_, authority = validation_fixture()
        self.assertEqual(10, authority.authorization_comment_id)
        self.assertEqual(20, authority.request_comment_id)
        self.assertEqual(30, authority.pin_comment_id)
        self.assertEqual("admin", authority.approval_permission)
        self.assertEqual("admin", authority.pin_permission)
        self.assertRegex(authority.validation_id, r"^[0-9a-f]{64}$")

    def test_records_can_be_resolved_before_run_then_bound_to_live_inventory(self):
        context, _, _, _, comments, permissions, authority = validation_fixture()
        provisional = contract.resolve_records(comments, permissions)
        self.assertFalse(provisional.context_bound)
        self.assertTrue(provisional.permission_snapshot_bound)
        self.assertEqual(authority.validation_id, provisional.validation_id)
        self.assertEqual(authority.validation_id, contract.expected_run_title(provisional).rsplit("|", 1)[1])
        with self.assertRaises(contract.ContractError):
            contract.build_authority_record(provisional, live_run(provisional))
        bound = contract.bind_authority_context(provisional, context)
        self.assertTrue(bound.context_bound)
        self.assertEqual(authority.planner_unit_obligations, bound.planner_unit_obligations)
        wrong_context = contract.TrustedRequestContext(
            task_uid=context.task_uid, head_oid="8" * 40,
            task_issue_number=context.task_issue_number, pr_number=context.pr_number,
            source_scope_oid=context.source_scope_oid, projection_digest=context.projection_digest,
            planner_unit_ids=context.planner_unit_ids,
            planner_unit_obligations=context.planner_unit_obligations,
        )
        with self.assertRaises(contract.ContractError):
            contract.bind_authority_context(provisional, wrong_context)

    def test_live_task_and_pr_numbers_are_dynamic_but_exactly_bound(self):
        context, _, _, _, comments, permissions, _ = validation_fixture()
        self.assertFalse(hasattr(contract, "TASK_ISSUE_NUMBER"))
        self.assertFalse(hasattr(contract, "PR_NUMBER"))
        changed_context = contract.TrustedRequestContext(
            task_uid=context.task_uid, task_issue_number=context.task_issue_number + 1,
            pr_number=context.pr_number, head_oid=context.head_oid,
            source_scope_oid=context.source_scope_oid,
            projection_digest=context.projection_digest,
            planner_unit_ids=context.planner_unit_ids,
            planner_unit_obligations=context.planner_unit_obligations,
        )
        provisional = contract.resolve_records(comments, permissions)
        with self.assertRaisesRegex(contract.ContractError, "trusted live Task/H/S identity"):
            contract.bind_authority_context(provisional, changed_context)

    def test_readback_uses_issued_admin_snapshot_without_live_permission_lookup(self):
        context, _, _, _, comments, _, issued = validation_fixture()
        provisional = contract.resolve_records_for_readback(comments)
        self.assertFalse(provisional.permission_snapshot_bound)
        self.assertEqual(issued.validation_id, provisional.validation_id)
        with self.assertRaisesRegex(contract.ContractError, "permission snapshot"):
            contract.bind_authority_context(provisional, context)

        issued_record = contract.build_authority_record(issued, live_run(issued))
        snapshot_bound = contract.bind_recorded_admin_snapshot(provisional, issued_record)
        self.assertTrue(snapshot_bound.permission_snapshot_bound)
        self.assertEqual("admin", snapshot_bound.approval_permission)
        self.assertEqual("admin", snapshot_bound.pin_permission)
        context_bound = contract.bind_authority_context(snapshot_bound, context)
        self.assertTrue(context_bound.context_bound)

        revoked = dict(issued_record, approval_permission="maintain")
        with self.assertRaisesRegex(contract.ContractError, "approval_permission"):
            contract.bind_recorded_admin_snapshot(provisional, revoked)
        changed_comments = list(comments)
        changed_comments[2] = {**comments[2], "user": {"login": "other-pin"}}
        changed = contract.resolve_records_for_readback(changed_comments)
        with self.assertRaisesRegex(contract.ContractError, "pin_actor"):
            contract.bind_recorded_admin_snapshot(changed, issued_record)

    def test_readback_selects_latest_complete_chain_for_live_task_pr_and_head(self):
        context, request, authorization, pin, comments, _, issued = validation_fixture()
        later_authorization_body = marked(contract.AUTHORIZATION_MARKER, dict(authorization))
        later_request = dict(request)
        later_request["authorization_source"] = {
            "issue_number": context.task_issue_number,
            "comment_id": 40,
            "body_digest": contract.body_digest(later_authorization_body),
        }
        later_request["request_digest"] = contract.request_digest({
            key: value for key, value in later_request.items() if key != "request_digest"
        })
        later_request_body = marked(contract.REQUEST_MARKER, later_request)
        later_pin = {
            **pin,
            "request_comment_id": 50,
            "request_body_digest": contract.body_digest(later_request_body),
            "request_digest": later_request["request_digest"],
        }
        later_pin_body = marked(contract.PIN_MARKER, later_pin)
        comments = [
            *comments,
            {"id": 40, "body": later_authorization_body,
             "user": {"login": "approval-admin"},
             "created_at": "2026-01-01T00:03:00Z", "updated_at": "2026-01-01T00:03:00Z"},
            {"id": 50, "body": later_request_body,
             "user": {"login": "requester"},
             "created_at": "2026-01-01T00:04:00Z", "updated_at": "2026-01-01T00:04:00Z"},
            {"id": 60, "body": later_pin_body,
             "user": {"login": "pin-admin"},
             "created_at": "2026-01-01T00:05:00Z", "updated_at": "2026-01-01T00:05:00Z"},
        ]
        expected_identity = {
            "task_uid": context.task_uid,
            "task_issue_number": context.task_issue_number,
            "pr_number": context.pr_number,
            "head_oid": request["head_oid"],
            "integration_base_oid": request["integration_base_oid"],
        }

        current = contract.resolve_records_for_readback(
            comments, expected_identity=expected_identity,
        )

        self.assertEqual(50, current.request_comment_id)
        self.assertEqual(40, current.authorization_comment_id)
        self.assertEqual(60, current.pin_comment_id)
        self.assertNotEqual(issued.validation_id, current.validation_id)

    def test_duplicate_keys_noncanonical_body_extra_fields_and_marker_duplicates_fail(self):
        _, request, _, _, comments, permissions, _ = validation_fixture()
        bad = list(comments)
        bad[1] = {**bad[1], "body": contract.REQUEST_MARKER + '\n{"a":1,"a":1}'}
        with self.assertRaises(contract.ContractError):
            contract.resolve_authority(bad, validation_fixture()[0], permissions)

        _, request, _, _, comments, permissions, _ = validation_fixture()
        altered = dict(request)
        altered["unlisted"] = "value"
        body = marked(contract.REQUEST_MARKER, altered)
        bad = list(comments)
        bad[1] = {**bad[1], "body": body}
        with self.assertRaises(contract.ContractError):
            contract.resolve_authority(bad, validation_fixture()[0], permissions)

        _, _, _, _, comments, permissions, _ = validation_fixture()
        with self.assertRaises(contract.ContractError):
            contract.resolve_authority(comments + [comments[0]], validation_fixture()[0], permissions)

    def test_actor_permission_identity_and_pin_binding_fail_closed(self):
        context, _, _, pin, comments, permissions, _ = validation_fixture()
        bad_permissions = dict(permissions)
        bad_permissions["approval-admin"] = {"login": "other", "permission": "admin"}
        with self.assertRaises(contract.ContractError):
            contract.resolve_authority(comments, context, bad_permissions)
        bad_permissions = dict(permissions)
        bad_permissions["pin-admin"] = {"login": "pin-admin", "permission": "maintain"}
        with self.assertRaises(contract.ContractError):
            contract.resolve_authority(comments, context, bad_permissions)
        bad = list(comments)
        wrong_pin = dict(pin)
        wrong_pin["request_comment_id"] = 999
        bad[2] = {**bad[2], "body": marked(contract.PIN_MARKER, wrong_pin)}
        with self.assertRaises(contract.ContractError):
            contract.resolve_authority(bad, context, permissions)

    def test_authority_must_precede_run_without_post_dispatch_edits(self):
        *_, authority = validation_fixture()
        contract.validate_authority_precedes_run(authority, "2026-01-01T00:03:00Z")
        with self.assertRaises(contract.ContractError):
            contract.validate_authority_precedes_run(authority, "2026-01-01T00:02:00Z")

    def test_id_artifact_title_derivations_are_exact_and_canonical(self):
        *_, authority = validation_fixture()
        expected = hashlib.sha256(
            b"oasis7-ci-reuse-validation-id/v1\x00" + authority.request["request_digest"].encode("ascii")
        ).hexdigest()
        self.assertEqual(expected, authority.validation_id)
        self.assertEqual(
            f"oasis7-ci-reuse-validation-v1-{expected}-r700-a2",
            contract.artifact_name(expected, 700, 2),
        )
        with self.assertRaises(contract.ContractError):
            contract.artifact_name(expected, 0, 1)


class SuccessorAuthorityTests(unittest.TestCase):
    def test_dynamic_default_branch_identity_is_retained_and_derived_exactly(self):
        identity = contract.normalize_workflow_identity(
            "release/next", ".github/workflows/rust.yml@release/next",
        )
        self.assertEqual(".github/workflows/rust.yml@release/next", identity["workflow_api_path"])
        self.assertEqual("release/next", identity["workflow_default_branch"])
        self.assertEqual("eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/release/next",
                         identity["workflow_ref"])
        self.assertEqual("refs/heads/release/next", identity["event_ref"])
        for branch, raw_path in (
            ("release/next", ".github/workflows/rust.yml@main"),
            ("release/next", ".github/workflows/rust.yml@release/next/other"),
            ("release/next", ".github/workflows/other.yml@release/next"),
        ):
            with self.subTest(raw_path=raw_path), self.assertRaises(contract.ContractError):
                contract.normalize_workflow_identity(branch, raw_path)

    def test_v2_resolvers_bind_same_task_failed_v1_and_current_admins(self):
        fixture = successor_fixture()
        provisional = contract.resolve_successor_records_for_readback(
            fixture["comments"], fixture["identity"], fixture["predecessor"],
        )
        self.assertFalse(provisional.permission_snapshot_bound)
        self.assertTrue(contract.is_successor_authority(provisional))
        self.assertEqual(contract.SUCCESSOR_RUN_MODE,
                         contract.expected_event_inputs(provisional)["run_mode"])
        self.assertEqual(
            contract.successor_validation_id(fixture["request"]["request_digest"]),
            provisional.validation_id,
        )
        self.assertNotEqual(fixture["predecessor"]["head_oid"], fixture["identity"]["head_oid"])
        authority = contract.resolve_successor_records(
            fixture["comments"], fixture["identity"], fixture["permissions"],
            fixture["predecessor"],
        )
        self.assertTrue(authority.permission_snapshot_bound)
        self.assertEqual(40, authority.authorization_comment_id)
        self.assertEqual(50, authority.request_comment_id)
        self.assertEqual(60, authority.pin_comment_id)
        bad_permissions = dict(fixture["permissions"])
        bad_permissions["approval-admin"] = {
            "login": "approval-admin", "permission": "maintain",
        }
        with self.assertRaises(contract.ContractError):
            contract.resolve_successor_records(
                fixture["comments"], fixture["identity"], bad_permissions,
                fixture["predecessor"],
            )

    def test_successor_authority_payload_and_readback_preserve_full_predecessor(self):
        fixture = successor_fixture()
        provisional = contract.resolve_successor_records_for_readback(
            fixture["comments"], fixture["identity"], fixture["predecessor"],
        )
        current_context = contract.TrustedRequestContext(
            task_uid=fixture["context"].task_uid,
            task_issue_number=fixture["context"].task_issue_number,
            pr_number=fixture["context"].pr_number,
            head_oid=fixture["identity"]["head_oid"],
            source_scope_oid=fixture["context"].source_scope_oid,
            projection_digest=fixture["context"].projection_digest,
            planner_unit_ids=fixture["context"].planner_unit_ids,
            planner_unit_obligations=fixture["context"].planner_unit_obligations,
        )
        run = {
            "repository": contract.REPOSITORY,
            "id": 701,
            "workflow_id": fixture["successor_workflow"]["workflow_id"],
            **contract.normalize_workflow_identity("release/next", ".github/workflows/rust.yml@release/next"),
            "workflow_sha": fixture["successor_workflow"]["workflow_sha"],
            "event": "workflow_dispatch",
            "display_title": contract.expected_run_title(provisional),
            "dispatched_head_sha": fixture["successor_workflow"]["workflow_sha"],
            "run_attempt": 1,
            "status": "completed",
            "conclusion": "success",
            "tested_merge_oid": "d" * 40,
            "tested_tree_oid": "e" * 40,
        }
        # The V2 authority can bind the same current planner context after the
        # producer has recorded its own live admin and workflow observations.
        issued = contract.resolve_successor_records(
            fixture["comments"], fixture["identity"], fixture["permissions"],
            fixture["predecessor"],
        )
        issued = contract.bind_authority_context(issued, current_context)
        record = contract.build_authority_record(issued, run)
        snapshot_bound = contract.bind_recorded_admin_snapshot(provisional, record)
        snapshot_bound = contract.bind_authority_context(snapshot_bound, current_context)
        self.assertEqual(contract.SUCCESSOR_AUTHORITY_SCHEMA, record["schema"])
        self.assertEqual(fixture["predecessor"], record["predecessor"])
        self.assertEqual(fixture["successor_workflow"], record["successor_workflow"])
        self.assertEqual(
            "oasis7-ci-reuse-validation-v2-" + issued.validation_id + "-r701-a1",
            contract.artifact_name(issued.validation_id, 701, 1, successor=True),
        )
        check = {
            "name": contract.CHECK_NAME, "id": 801,
            "app_id": contract.GITHUB_ACTIONS_APP_ID,
            "head_sha": run["dispatched_head_sha"], "run_id": run["id"],
            "run_attempt": run["run_attempt"], "status": "completed", "conclusion": "success",
        }
        payload = {
            **record,
            "schema": contract.SUCCESSOR_PAYLOAD_SCHEMA,
            "authority_digest": contract.authority_digest(record),
            "capability_under_test": contract.CAPABILITY,
            "tested_merge_oid": run["tested_merge_oid"],
            "tested_tree_oid": run["tested_tree_oid"],
            "run_attempt": run["run_attempt"],
            "event_inputs": contract.expected_event_inputs(issued),
            "check_name": check["name"], "check_run_id": check["id"],
            "check_app_id": check["app_id"],
            "selected_obligations": {
                unit: list(issued.planner_unit_obligations[unit])
                for unit in issued.request["validation_units"]
            },
            "result_digests": {
                unit: "sha256:" + "f" * 64 for unit in issued.request["validation_units"]
            },
        }
        verified = contract.verify_payload(payload, issued, run, check)
        self.assertEqual(contract.SUCCESSOR_PAYLOAD_SCHEMA, verified["schema"])
        payload_bytes = contract.canonical_json_bytes(payload)
        artifact_bytes = b"successor archive"
        artifact = {
            "id": 902,
            "name": contract.artifact_name(issued.validation_id, 701, 1, successor=True),
        }
        envelope = {
            "schema": contract.SUCCESSOR_READBACK_SCHEMA,
            "authority_digest": contract.authority_digest(record),
            "validation_id": issued.validation_id,
            "run_id": 701, "run_attempt": 1,
            "workflow_id": run["workflow_id"],
            **{key: run[key] for key in (
                "workflow_api_path", "workflow_default_branch", "workflow_path",
                "workflow_ref", "event_ref", "workflow_sha", "event",
                "display_title", "dispatched_head_sha",
            )},
            "check_name": check["name"], "check_run_id": check["id"],
            "check_app_id": check["app_id"], "artifact_id": artifact["id"],
            "artifact_name": artifact["name"],
            "artifact_content_digest": contract.body_digest(artifact_bytes),
            "payload_digest": contract.body_digest(payload_bytes),
            "successor_sequence": 1,
            "reason": issued.request["reason"],
            "predecessor": fixture["predecessor"],
            "predecessor_digest": issued.request["predecessor_digest"],
            "successor_workflow": fixture["successor_workflow"],
            "successor_workflow_digest": issued.request["successor_workflow_digest"],
        }
        contract.verify_readback(
            envelope, issued, run, check, artifact, payload_bytes, artifact_bytes,
        )
        wrong_predecessor = dict(envelope)
        wrong_predecessor["predecessor_digest"] = "sha256:" + "0" * 64
        with self.assertRaises(contract.ContractError):
            contract.verify_readback(
                wrong_predecessor, issued, run, check, artifact, payload_bytes, artifact_bytes,
            )
        self.assertTrue(snapshot_bound.permission_snapshot_bound)

    def test_successor_proof_rejects_changed_predecessor_and_nonfailed_or_stale_authority(self):
        fixture = successor_fixture()
        for field, value in (
            ("request_comment_id", 999),
            ("request_digest", "sha256:" + "0" * 64),
            ("workflow_blob_digest", "sha256:" + "0" * 64),
            ("run_conclusion", "success"),
            ("run_terminal_updated_at", "2026-01-01T00:14:00Z"),
        ):
            observation = dict(fixture["predecessor"], **{field: value})
            with self.subTest(field=field), self.assertRaises(contract.ContractError):
                contract.resolve_successor_records_for_readback(
                    fixture["comments"], fixture["identity"], observation,
                )
        stale = list(fixture["comments"])
        stale[3] = {**stale[3], "created_at": "2026-01-01T00:09:00Z",
                    "updated_at": "2026-01-01T00:09:00Z"}
        with self.assertRaises(contract.ContractError):
            contract.resolve_successor_records_for_readback(
                stale, fixture["identity"], fixture["predecessor"],
            )

    def test_newer_malformed_successor_intent_blocks_prior_complete_chain(self):
        fixture = successor_fixture()
        newer = {
            **fixture["identity"],
            "schema": contract.SUCCESSOR_REQUEST_SCHEMA,
        }
        comments = fixture["comments"] + [{
            "id": 70,
            "body": marked(contract.SUCCESSOR_REQUEST_MARKER, newer),
            "user": {"login": "requester"},
            "created_at": "2026-01-01T00:14:00Z",
            "updated_at": "2026-01-01T00:14:00Z",
        }]
        with self.assertRaises(contract.ContractError):
            contract.resolve_successor_records_for_readback(
                comments, fixture["identity"], fixture["predecessor"],
            )


class WorkflowDiscoveryTests(unittest.TestCase):
    def test_complete_history_accepts_bounded_unicode_beside_exact_ascii_candidate(self):
        *_, authority = validation_fixture()
        historical = {"id": 699, "display_title": "nightly 日本語 🌱 — прошлый запуск"}
        target = live_run(authority)
        pages = [{"total_count": 2, "runs": [historical, target], "has_next": False}]

        complete = contract.collect_workflow_runs(pages)
        selected = contract.select_unique_run(complete, authority)

        self.assertEqual(historical["display_title"], complete[0]["display_title"])
        self.assertEqual(target["id"], selected["id"])
        self.assertEqual(contract.expected_run_title(authority), selected["display_title"])
        self.assertTrue(selected["display_title"].isascii())

    def test_malformed_or_superseded_action_titles_do_not_poison_current_request(self):
        *_, authority = validation_fixture()
        target = live_run(authority)
        expected = contract.expected_run_title(authority)
        validation_id = authority.validation_id
        prefix_candidate = {
            "id": 701,
            "display_title": expected[:-len(validation_id)] + "wrong-key-🌱",
        }
        suffix_candidate = {
            "id": 702,
            "display_title": "unrelated history—日本語|" + validation_id,
        }

        for candidate in (prefix_candidate, suffix_candidate):
            with self.subTest(run_id=candidate["id"]):
                with self.assertRaises(contract.ContractError):
                    contract.select_unique_run([candidate], authority)
                self.assertEqual(target["id"], contract.select_unique_run([target, candidate], authority)["id"])

    def test_failed_cancelled_and_superseded_runs_do_not_poison_successful_retry(self):
        *_, authority = validation_fixture()
        successful = live_run(authority)
        failed = {**successful, "id": 701, "status": "completed", "conclusion": "failure"}
        cancelled = {**successful, "id": 702, "status": "completed", "conclusion": "cancelled"}
        superseded = {**successful, "id": 703, "status": "completed", "conclusion": "timed_out"}

        selected = contract.select_unique_run([failed, cancelled, superseded, successful], authority)

        self.assertEqual(successful["id"], selected["id"])

    def test_current_dispatch_selects_current_run_and_rejects_competing_live_attempt(self):
        *_, authority = validation_fixture()
        current = {**live_run(authority), "id": 704, "status": "in_progress", "conclusion": None}
        failed = {**current, "id": 705, "status": "completed", "conclusion": "failure"}
        selected = contract.select_unique_run([failed, current], authority, current_run_id=704)
        self.assertEqual(current["id"], selected["id"])

        competing = {**current, "id": 706, "status": "in_progress", "conclusion": None}
        with self.assertRaisesRegex(contract.ContractError, "competing"):
            contract.select_unique_run([failed, current, competing], authority, current_run_id=704)

    def test_history_titles_accept_long_well_formed_unicode_without_normalization(self):
        *_, authority = validation_fixture()
        historical = {"id": 698, "display_title": "🌱" * 1025}
        target = live_run(authority)

        complete = contract.collect_workflow_runs([
            {"total_count": 2, "runs": [historical, target], "has_next": False},
        ])
        selected = contract.select_unique_run(complete, authority)

        self.assertEqual(historical["display_title"], complete[0]["display_title"])
        self.assertEqual(target["id"], selected["id"])
        self.assertEqual(contract.expected_run_title(authority), selected["display_title"])
        self.assertTrue(selected["display_title"].isascii())

    def test_history_titles_must_be_well_formed_utf8_strings(self):
        malformed_titles = ("", "unpaired surrogate \ud800", "nul\x00", "carriage\rreturn", "line\nfeed")
        for index, title in enumerate(malformed_titles, start=710):
            with self.subTest(title_kind=index):
                page = [{"total_count": 1, "runs": [{"id": index, "display_title": title}], "has_next": False}]
                with self.assertRaises(contract.ContractError):
                    contract.collect_workflow_runs(page)
        for row in ({"id": 720}, {"id": 721, "display_title": None}, {"id": 722, "display_title": 42}):
            with self.subTest(row=row):
                page = [{"total_count": 1, "runs": [row], "has_next": False}]
                with self.assertRaises(contract.ContractError):
                    contract.collect_workflow_runs(page)

    def test_unfiltered_history_exhaustion_accepts_more_than_one_thousand_runs(self):
        pages = []
        next_id = 10000
        for page_index in range(11):
            runs = [{"id": next_id + page_index * 100 + offset,
                     "display_title": f"unrelated-{page_index}-{offset}"}
                    for offset in range(100 if page_index < 10 else 1)]
            pages.append({"total_count": 1001, "runs": runs, "has_next": page_index < 10})
        self.assertEqual(1001, len(contract.collect_workflow_runs(pages)))

    def test_workflow_pages_reject_incomplete_duplicate_and_unstable_counts(self):
        failures = [
            [{"total_count": 2, "runs": [{"id": 1, "display_title": "x"}], "has_next": False}],
            [{"total_count": 2, "runs": [{"id": 1, "display_title": "x"}], "has_next": True}],
            [
                {"total_count": 2, "runs": [{"id": 1, "display_title": "x"}], "has_next": True},
                {"total_count": 3, "runs": [{"id": 2, "display_title": "y"}], "has_next": False},
            ],
            [{"total_count": 2, "runs": [{"id": 1, "display_title": "x"}, {"id": 1, "display_title": "y"}], "has_next": False}],
            [{"total_count": 1, "runs": [{"id": 1}], "has_next": False}],
        ]
        for pages in failures:
            with self.subTest(pages=pages), self.assertRaises(contract.ContractError):
                contract.collect_workflow_runs(pages)

    def test_other_request_key_for_same_task_head_is_not_a_current_action_candidate(self):
        *_, authority = validation_fixture()
        target = live_run(authority)
        malformed = {**target, "id": 701, "display_title": target["display_title"].rsplit("|", 1)[0] + "|wrong-key"}
        self.assertEqual(target["id"], contract.select_unique_run([target, malformed], authority)["id"])
        with self.assertRaises(contract.ContractError):
            contract.select_unique_run([malformed], authority)


class PayloadReadbackTests(unittest.TestCase):
    def test_payload_and_readback_bind_exact_run_attempt_check_and_bytes(self):
        *_, authority = validation_fixture()
        run, check, payload = payload_fixture(authority)
        verified = contract.verify_payload(payload, authority, run, check)
        payload_bytes = contract.canonical_json_bytes(verified)
        artifact_bytes = b"zip-archive-content"
        artifact = {"id": 910, "name": contract.artifact_name(authority.validation_id, run["id"], 1)}
        envelope = {
            "schema": contract.READBACK_SCHEMA,
            "authority_digest": payload["authority_digest"],
            "validation_id": authority.validation_id,
            "run_id": run["id"], "run_attempt": 1,
            "workflow_id": run["workflow_id"], "workflow_api_path": run["workflow_api_path"],
            "workflow_default_branch": run["workflow_default_branch"],
            "workflow_path": run["workflow_path"], "workflow_ref": run["workflow_ref"],
            "event_ref": run["event_ref"], "workflow_sha": run["workflow_sha"],
            "event": run["event"], "display_title": run["display_title"],
            "dispatched_head_sha": run["dispatched_head_sha"],
            "check_name": check["name"], "check_run_id": check["id"],
            "check_app_id": check["app_id"], "artifact_id": artifact["id"],
            "artifact_name": artifact["name"],
            "artifact_content_digest": contract.body_digest(artifact_bytes),
            "payload_digest": contract.body_digest(payload_bytes),
        }
        contract.verify_readback(
            envelope, authority, run, check, artifact,
            payload_bytes, artifact_bytes,
        )
        changed = dict(envelope)
        changed["run_attempt"] = 2
        with self.assertRaises(contract.ContractError):
            contract.verify_readback(
                changed, authority, run, check, artifact,
                payload_bytes, artifact_bytes,
            )

    def test_payload_rejects_unlisted_fields_wrong_inputs_wrong_attempt_and_production_check(self):
        *_, authority = validation_fixture()
        run, check, payload = payload_fixture(authority)
        mutated_values = []
        extra = dict(payload); extra["required_plan"] = {}; mutated_values.append(extra)
        wrong_inputs = dict(payload); wrong_inputs["event_inputs"] = {}; mutated_values.append(wrong_inputs)
        wrong_attempt = dict(payload); wrong_attempt["run_attempt"] = 2; mutated_values.append(wrong_attempt)
        wrong_check = dict(check); wrong_check["name"] = "required-gate"; mutated_values.append((payload, wrong_check))
        for mutated in mutated_values:
            candidate, candidate_check = mutated if isinstance(mutated, tuple) else (mutated, check)
            with self.subTest(candidate=candidate), self.assertRaises(contract.ContractError):
                contract.verify_payload(candidate, authority, run, candidate_check)

    def test_readback_requires_raw_canonical_payload_and_exact_archive_digest(self):
        *_, authority = validation_fixture()
        run, check, payload = payload_fixture(authority)
        payload_bytes = contract.canonical_json_bytes(payload)
        artifact_bytes = b"archive"
        artifact = {"id": 99, "name": contract.artifact_name(authority.validation_id, run["id"], 1)}
        record = contract.build_authority_record(authority, run)
        envelope = {
            "schema": contract.READBACK_SCHEMA, "authority_digest": contract.authority_digest(record),
            "validation_id": authority.validation_id, "run_id": run["id"], "run_attempt": 1,
            "workflow_id": run["workflow_id"], "workflow_api_path": run["workflow_api_path"],
            "workflow_default_branch": run["workflow_default_branch"],
            "workflow_path": run["workflow_path"], "workflow_ref": run["workflow_ref"],
            "event_ref": run["event_ref"], "workflow_sha": run["workflow_sha"],
            "event": run["event"], "display_title": run["display_title"],
            "dispatched_head_sha": run["dispatched_head_sha"], "check_name": check["name"],
            "check_run_id": check["id"], "check_app_id": check["app_id"],
            "artifact_id": artifact["id"], "artifact_name": artifact["name"],
            "artifact_content_digest": contract.body_digest(artifact_bytes),
            "payload_digest": contract.body_digest(payload_bytes),
        }
        with self.assertRaises(contract.ContractError):
            contract.verify_readback(envelope, authority, run, check, artifact, payload_bytes + b"\n", artifact_bytes)
        with self.assertRaises(contract.ContractError):
            contract.verify_readback(envelope, authority, run, check, artifact, payload_bytes, artifact_bytes + b"x")


if __name__ == "__main__":
    unittest.main()
