"""Adversarial tests for the non-promotable CI reuse validation contract."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).parent
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
        "task_issue_number": 4059,
        "pr_number": 4060,
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
        "task_issue_number": 4059,
        "pr_number": 4060,
        "head_oid": context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": list(authorization["validation_units"]),
        "purpose": "v1_pre_activation_validation",
        "authorization_decision": "authorize_validation_only",
        "authorization_source": {
            "issue_number": 4059,
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
        "task_issue_number": 4059,
        "pr_number": 4060,
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


def live_run(authority=None):
    if authority is None:
        *_, authority = validation_fixture()
    return {
        "repository": "eng-cc/oasis7",
        "id": 700,
        "workflow_id": 900,
        "workflow_path": ".github/workflows/rust.yml@main",
        "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
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
        "schema": contract.PAYLOAD_SCHEMA,
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
        self.assertIn(unit_id, validated_context[4])
        self.assertEqual((obligation,), validated_context[5][unit_id])
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
            source_scope_oid=context.source_scope_oid, projection_digest=context.projection_digest,
            planner_unit_ids=context.planner_unit_ids,
            planner_unit_obligations=context.planner_unit_obligations,
        )
        with self.assertRaises(contract.ContractError):
            contract.bind_authority_context(provisional, wrong_context)

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

    def test_unicode_prefix_and_suffix_near_candidates_still_block(self):
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
                with self.assertRaises(contract.ContractError):
                    contract.select_unique_run([target, candidate], authority)

    def test_history_titles_must_be_bounded_well_formed_utf8_strings(self):
        malformed_titles = ("", "unpaired surrogate \ud800", "x" * 1025, "nul\x00", "line\nfeed")
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

    def test_candidate_union_rejects_second_run_even_if_only_prefix_matches(self):
        *_, authority = validation_fixture()
        target = live_run(authority)
        malformed = {**target, "id": 701, "display_title": target["display_title"].rsplit("|", 1)[0] + "|wrong-key"}
        with self.assertRaises(contract.ContractError):
            contract.select_unique_run([target, malformed], authority)
        self.assertEqual(target["id"], contract.select_unique_run([target], authority)["id"])


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
            "workflow_id": run["workflow_id"], "workflow_path": run["workflow_path"],
            "workflow_ref": run["workflow_ref"], "workflow_sha": run["workflow_sha"],
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
            "workflow_id": run["workflow_id"], "workflow_path": run["workflow_path"],
            "workflow_ref": run["workflow_ref"], "workflow_sha": run["workflow_sha"],
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
