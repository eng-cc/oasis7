#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
PATH = ROOT / "scripts/pm/first_activation.py"
SPEC = importlib.util.spec_from_file_location("first_activation_under_test", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
GATE_SPEC = importlib.util.spec_from_file_location(
    "first_activation_gate_under_test", ROOT / "scripts/pm/pr-lifecycle-gate.py"
)
assert GATE_SPEC and GATE_SPEC.loader
GATE = importlib.util.module_from_spec(GATE_SPEC)
GATE_SPEC.loader.exec_module(GATE)
TASK_HELPER_SPEC = importlib.util.spec_from_file_location(
    "first_activation_task_helper_test", ROOT / "scripts/pm/github-project-task.py"
)
assert TASK_HELPER_SPEC and TASK_HELPER_SPEC.loader
TASK_HELPER = importlib.util.module_from_spec(TASK_HELPER_SPEC)
TASK_HELPER_SPEC.loader.exec_module(TASK_HELPER)

UID = "task_" + "a" * 32
BASE = "1" * 40
HEAD = "2" * 40
SHA = "sha256:" + "a" * 64
ISSUE_URL = "https://api.github.com/repos/eng-cc/oasis7/issues/4269"
PACKET_ISSUE_URL = "https://github.com/eng-cc/oasis7/issues/4269"


def _comment(comment_id: int, body: str, second: int) -> dict:
    return {
        "id": comment_id,
        "issue_url": ISSUE_URL,
        "body": body,
        "user": {"login": "human"},
        "created_at": f"2026-10-03T00:00:{second:02d}Z",
    }


def review_evidence(value: dict, packet_digests: dict[str, str] | None = None, *,
                    base: str = BASE, head: str = HEAD, offset: int = 0) -> tuple[list[dict], dict[str, dict]]:
    packet_digests = packet_digests or {}
    auth_body = "User authorizes oasis7_wasm_executor wasmtime 48.0.3 to 48.0.4 effective_package_dependency_floor"
    value["authorization"]["body_sha256"] = "sha256:" + hashlib.sha256(auth_body.encode()).hexdigest()
    dispatch_body = "Authorized implementation slice implementation-checker: .github/workflows/rust.yml"
    dispatch_digest = "sha256:" + hashlib.sha256(dispatch_body.encode()).hexdigest()
    plan = {
        "schema": MODULE.FIRST_REVIEW_PLAN_SCHEMA,
        "task_uid": UID,
        "issue_number": 4269,
        "bootstrap_epoch": value["bootstrap_epoch"],
        "snapshot_sha256": value["snapshot_sha256"],
        "request_sha256": value["request_sha256"],
        "acceptance_sha256": value["acceptance_sha256"],
        "raw_primary_package": value["raw_primary_package"],
        "effective_primary_package": value["effective_primary_package"],
        "dependency": value["dependency"],
        "base_oid": base,
        "head_oid": head,
        "authorization": value["authorization"],
        "workflow": {key: value["workflow"][key] for key in
                     ("id", "path", "ref", "sha", "file_sha256")},
        "workflow_change_paths": value["workflow"]["change_paths"],
        "implementation_slices": [{
            "role": "repository_health_engineer",
            "slice_id": "implementation-checker",
            "dispatch_comments": [{"comment_id": 2 + offset, "body_sha256": dispatch_digest}],
            "write_scope_paths": [".github/workflows/rust.yml"],
        }],
        "review_slices": [
            {"role": "qa_engineer", "slice_id": "first-qa",
             "packet_sha256": packet_digests.get("qa_engineer", SHA),
             "write_scope": MODULE.FIRST_REVIEW_WRITE_SCOPE},
            {"role": "repository_health_engineer", "slice_id": "first-rh",
             "packet_sha256": packet_digests.get("repository_health_engineer", SHA),
             "write_scope": MODULE.FIRST_REVIEW_WRITE_SCOPE},
        ],
    }
    plan_body = MODULE.canonical_first_review_plan_body(plan)
    plan_sha = "sha256:" + hashlib.sha256(plan_body.encode()).hexdigest()
    comments = [
        _comment(1, auth_body, 1),
        _comment(2 + offset, dispatch_body, 2 + offset),
        _comment(3 + offset, plan_body, 3 + offset),
    ]
    envelopes: dict[str, dict] = {}
    for return_id, review_id, envelope_role, plan_role, slice_id in (
        (4 + offset, 5 + offset, "repository_health", "repository_health_engineer", "first-rh"),
        (6 + offset, 7 + offset, "qa", "qa_engineer", "first-qa"),
    ):
        returned = {
            "schema": MODULE.FIRST_REVIEW_RETURN_SCHEMA,
            "task_uid": UID,
            "issue_number": 4269,
            "role": plan_role,
            "slice_id": slice_id,
            "base_oid": base,
            "head_oid": head,
            "review_plan_sha256": plan_sha,
            "admitted_packet_sha256": packet_digests.get(plan_role, SHA),
            "return_status": "completed",
            "disposition": "no_findings",
            "findings": [],
            "unresolved_findings": [],
            "residual_risk": [],
        }
        return_body = MODULE.canonical_first_review_return_body(returned)
        return_sha = "sha256:" + hashlib.sha256(return_body.encode()).hexdigest()
        typed = {
            "schema": MODULE.REVIEW_SCHEMA,
            "task_uid": UID,
            "issue_number": 4269,
            "role": envelope_role,
            "slice_id": slice_id,
            "base_oid": base,
            "head_oid": head,
            "workflow_change_paths": value["workflow"]["change_paths"],
            "review_plan_sha256": plan_sha,
            "admitted_packet_sha256": packet_digests.get(plan_role, SHA),
            "return_sha256": return_sha,
            "return_status": "completed",
            "findings": [],
            "unresolved_findings": [],
        }
        envelope_body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(typed).decode() + "\n"
        value["reviews"][envelope_role]["comment_id"] = review_id
        value["reviews"][envelope_role]["body_sha256"] = "sha256:" + hashlib.sha256(envelope_body.encode()).hexdigest()
        envelopes[envelope_role] = typed
        comments.extend((_comment(return_id, return_body, return_id),
                         _comment(review_id, envelope_body, review_id)))
    return comments, envelopes


def overlay(*, head: str = HEAD) -> dict:
    return {
        "schema": MODULE.OVERLAY_SCHEMA,
        "task_uid": UID,
        "issue_number": 4269,
        "bootstrap_epoch": 1,
        "snapshot_sha256": SHA,
        "request_sha256": SHA,
        "acceptance_sha256": SHA,
        "raw_primary_package": {"present": False, "value": None},
        "effective_primary_package": "oasis7_wasm_executor",
        "mode": "dependency_floor_update",
        "dependency": {
            "name": "wasmtime",
            "base_requirement": "48.0.3",
            "head_requirement": "48.0.4",
        },
        "base_oid": BASE,
        "head_oid": head,
        "authorization": {"comment_id": 1, "body_sha256": SHA, "scope": "effective_package_dependency_floor"},
        "reviews": {
            "repository_health": {"comment_id": 2, "body_sha256": SHA},
            "qa": {"comment_id": 3, "body_sha256": SHA},
        },
        "workflow": {
            "id": 123456,
            "path": ".github/workflows/rust.yml",
            "ref": "refs/heads/codex/test",
            "sha": head,
            "file_sha256": SHA,
            "change_paths": [
                {"path": ".github/workflows/rust.yml", "base_sha256": SHA, "head_sha256": SHA}
            ],
        },
    }


def scope_amendment() -> dict:
    return {
        "schema": MODULE.SCOPE_AMENDMENT_SCHEMA,
        "task_uid": UID,
        "issue_number": 4269,
        "bootstrap_epoch": 1,
        "snapshot_sha256": SHA,
        "request_sha256": SHA,
        "acceptance_sha256": SHA,
        "raw_primary_package": {"present": False, "value": None},
        "effective_primary_package": "oasis7_wasm_executor",
        "mode": "dependency_floor_update",
        "dependency": {
            "name": "wasmtime",
            "base_requirement": "48.0.3",
            "previous_accepted_requirement": "48.0.4",
            "replacement_requirement": "49.0.2",
        },
        "supersedes": {
            "authorization_comment_id": 10,
            "authorization_body_sha256": SHA,
            "overlay_comment_id": 20,
            "overlay_body_sha256": SHA,
            "overlay_base_oid": BASE,
            "overlay_head_oid": "3" * 40,
        },
        "authorization_evidence": {
            "source": "direct_user_conversation_recorded_by_tpm",
            "scope_record_comment_id": 11,
            "scope_record_body_sha256": SHA,
            "approval_record_comment_id": 12,
            "approval_record_body_sha256": SHA,
            "scope": "same_task_same_pr_wasmtime_49_0_2",
        },
        "approved_business_paths": [
            "Cargo.lock",
            "crates/oasis7_wasm_executor/Cargo.toml",
            "crates/oasis7_wasm_executor/src/lib.rs",
            "crates/oasis7_wasm_executor/src/tests.rs",
        ],
    }


def amended_overlay(*, head: str = HEAD) -> dict:
    row = overlay(head=head)
    row["schema"] = MODULE.OVERLAY_V2_SCHEMA
    row["dependency"]["head_requirement"] = "49.0.2"
    row["authorization"] = {
        "comment_id": 12,
        "body_sha256": SHA,
        "scope": "same_task_same_pr_wasmtime_49_0_2",
    }
    row["scope_amendment"] = {"comment_id": 13, "body_sha256": SHA}
    row["business_change_paths"] = [
        {"path": path, "base_sha256": SHA, "head_sha256": SHA}
        for path in scope_amendment()["approved_business_paths"]
    ]
    return row


class OverlayShapeTests(unittest.TestCase):
    def test_request_identity_uses_canonical_issue_title_reconstruction(self):
        self.assertEqual(
            TASK_HELPER.normalize_issue_title("[PM] Upgrade Wasmtime to patched 48.0.4"),
            "Upgrade Wasmtime to patched 48.0.4",
        )
        self.assertEqual(TASK_HELPER.normalize_issue_title("[pm] unchanged"), "[pm] unchanged")
        self.assertEqual(TASK_HELPER.normalize_issue_title("[PM]"), "[PM]")

    def test_valid_overlay_returns_only_bound_selector_fields(self):
        parsed = MODULE.validate_overlay(overlay(), task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        self.assertEqual(parsed["mode"], "dependency_floor_update")
        self.assertEqual(parsed["effective_primary_package"], "oasis7_wasm_executor")
        self.assertEqual(parsed["workflow_change_paths"][0]["path"], ".github/workflows/rust.yml")
        self.assertTrue(parsed["validation_only"])

    def test_scope_amendment_v2_binds_exact_task_target_and_business_paths(self):
        amendment = MODULE.validate_scope_amendment_payload(
            scope_amendment(), task_uid=UID, issue_number=4269,
        )
        payload = amended_overlay()
        parsed = MODULE.validate_overlay(
            payload, task_uid=UID, issue_number=4269, base_oid=BASE, head_oid=HEAD,
            expected_raw_primary={"present": False, "value": None},
            scope_amendment=amendment,
        )
        self.assertEqual(parsed["schema"], MODULE.OVERLAY_V2_SCHEMA)
        self.assertEqual(parsed["approved_business_paths"], amendment["approved_business_paths"])
        self.assertEqual(parsed["head_requirement"], "49.0.2")

    def test_scope_amendment_v2_rejects_missing_amendment_and_path_or_target_drift(self):
        payload = amended_overlay()
        with self.assertRaisesRegex(MODULE.OverlayError, "scope amendment"):
            MODULE.validate_overlay(payload, task_uid=UID, issue_number=4269,
                                    base_oid=BASE, head_oid=HEAD)
        amendment = MODULE.validate_scope_amendment_payload(
            scope_amendment(), task_uid=UID, issue_number=4269,
        )
        for mutate in (
            lambda row: row["business_change_paths"].pop(),
            lambda row: row["dependency"].update(head_requirement="49.0.3"),
            lambda row: row.update(raw_primary_package={"present": True, "value": "oasis7"}),
            lambda row: row.update(head_oid="4" * 40),
        ):
            changed = amended_overlay()
            mutate(changed)
            with self.subTest(changed=changed):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(
                        changed, task_uid=UID, issue_number=4269, base_oid=BASE,
                        head_oid=HEAD, expected_raw_primary={"present": False, "value": None},
                        scope_amendment=amendment,
                    )
        downgraded = scope_amendment()
        downgraded["dependency"]["replacement_requirement"] = "48.0.4"
        with self.assertRaisesRegex(MODULE.OverlayError, "exceed both"):
            MODULE.validate_scope_amendment_payload(downgraded, task_uid=UID, issue_number=4269)

    def test_live_scope_amendment_binds_superseded_records_and_direct_approval(self):
        prior_authorization = "User authorized oasis7_wasm_executor wasmtime 48.0.3 to 48.0.4"
        scope_record = f"Task scope record task_uid: {UID}; Wasmtime 49.0.2 same PR."
        approval_record = f"User explicit approval task_uid: {UID}; Wasmtime 49.0.2 same PR."
        old_overlay = overlay(head="3" * 40)
        old_overlay["authorization"] = {
            "comment_id": 10,
            "body_sha256": "sha256:" + hashlib.sha256(prior_authorization.encode()).hexdigest(),
            "scope": "effective_package_dependency_floor",
        }
        old_overlay_body = MODULE.canonical_overlay_body(old_overlay)
        amendment = scope_amendment()
        amendment["supersedes"]["authorization_body_sha256"] = old_overlay["authorization"]["body_sha256"]
        amendment["supersedes"]["overlay_body_sha256"] = (
            "sha256:" + hashlib.sha256(old_overlay_body.encode()).hexdigest()
        )
        amendment["authorization_evidence"]["scope_record_body_sha256"] = (
            "sha256:" + hashlib.sha256(scope_record.encode()).hexdigest()
        )
        amendment["authorization_evidence"]["approval_record_body_sha256"] = (
            "sha256:" + hashlib.sha256(approval_record.encode()).hexdigest()
        )
        amendment_body = MODULE.canonical_scope_amendment_body(amendment)
        comments = [
            _comment(10, prior_authorization, 1),
            _comment(20, old_overlay_body, 2),
            _comment(11, scope_record, 3),
            _comment(12, approval_record, 4),
            _comment(13, amendment_body, 5),
        ]
        _, parsed = MODULE._validated_scope_amendment(
            comments, issue_url=ISSUE_URL, task_uid=UID, issue_number=4269,
            overlay_ref={"comment_id": 13,
                         "body_sha256": "sha256:" + hashlib.sha256(amendment_body.encode()).hexdigest()},
        )
        self.assertEqual(parsed["dependency"]["replacement_requirement"], "49.0.2")
        with self.assertRaisesRegex(MODULE.OverlayError, "canonical Task Issue"):
            MODULE._validated_scope_amendment(
                [dict(comments[0], issue_url="https://api.github.com/repos/other/repo/issues/4269"),
                 *comments[1:]],
                issue_url=ISSUE_URL, task_uid=UID, issue_number=4269,
                overlay_ref={"comment_id": 13,
                             "body_sha256": "sha256:" + hashlib.sha256(amendment_body.encode()).hexdigest()},
            )

    def test_exact_first_activation_path_set_and_rejects_business_manifest(self):
        expected_paths = {
            "doc/engineering/workflow/source-of-truth.md",
            "doc/.governance/document-corpus/objects/48/4840d720cacf3f7d531e8a361fc146277494b75857c9bd904bbc4b0f700c6f41.json",
            ".github/workflows/rust.yml",
            "scripts/ci-tests.sh",
            "scripts/ci-required-scope-audit-contract.test.sh",
            "scripts/ci-required-capability-test-inventory.tsv",
            "scripts/pm/check-cargo-package-scope",
            "scripts/pm/check-cargo-package-scope.test.py",
            "scripts/pm/first_activation.py",
            "scripts/pm/first_activation.test.py",
            "scripts/pm/github-project-task.py",
            "scripts/pm/pr-lifecycle-gate.py",
            "scripts/prepare-task-pr.sh",
            "scripts/prepare-task-pr.test.sh",
        }
        self.assertEqual(MODULE.FIRST_ACTIVATION_REVIEWED_PATHS, expected_paths)
        self.assertTrue(MODULE._dispatch_scope_mentions(
            "scripts/pm/check-cargo-package-scope.test.py",
            "verification check-cargo-package-scope.test.py",
        ))
        self.assertFalse(MODULE._dispatch_scope_mentions(
            "scripts/pm/check-cargo-package-scope.test.py", "verification unrelated.test.py",
        ))
        row = overlay()
        row["workflow"]["change_paths"] = [
            {"path": path, "base_sha256": SHA, "head_sha256": SHA}
            for path in sorted(expected_paths)
        ]
        parsed = MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        self.assertEqual({item["path"] for item in parsed["workflow_change_paths"]}, expected_paths)

        row["workflow"]["change_paths"].append({
            "path": "crates/oasis7_wasm_executor/Cargo.toml",
            "base_sha256": SHA,
            "head_sha256": SHA,
        })
        row["workflow"]["change_paths"].sort(key=lambda item: item["path"])
        with self.assertRaises(MODULE.OverlayError):
            MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                    base_oid=BASE, head_oid=HEAD)

    def test_rejects_uid_head_mode_and_raw_primary_drift(self):
        for mutate in (
            lambda row: row.update(task_uid="task_" + "b" * 32),
            lambda row: row.update(head_oid="3" * 40),
            lambda row: row.update(mode="auto"),
            lambda row: row.update(raw_primary_package={"present": True, "value": "oasis7"}),
        ):
            row = overlay()
            mutate(row)
            with self.subTest(row=row):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                            base_oid=BASE, head_oid=HEAD,
                                            expected_raw_primary={"present": False, "value": None})

    def test_rejects_duplicate_or_escaping_workflow_paths(self):
        for path in ("../outside", "/absolute", "scripts//bad", "crates/other/src/lib.rs",
                     "Cargo.lock", ".cargo/config.toml", "crates/other/Cargo.toml"):
            row = overlay()
            row["workflow"]["change_paths"][0]["path"] = path
            row["workflow"]["path"] = path
            with self.subTest(path=path):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                            base_oid=BASE, head_oid=HEAD)

    def test_rejects_non_patch_raise_and_noncanonical_requirement(self):
        for head_req in ("48.1.0", "48.0.04", "48.0.4-alpha"):
            row = overlay()
            row["dependency"]["head_requirement"] = head_req
            with self.subTest(head_req=head_req):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_overlay(row, task_uid=UID, issue_number=4269,
                                            base_oid=BASE, head_oid=HEAD)

    def test_live_dispatch_provenance_requires_exact_workflow_and_run_attempt(self):
        parsed = MODULE.validate_overlay(overlay(), task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        run = {
            "id": 99,
            "run_attempt": 1,
            "event": "workflow_dispatch",
            "workflow_id": parsed["workflow_id"],
            "path": parsed["workflow_path"],
            "head_branch": "codex/test",
            "head_sha": HEAD,
            "repository": {"full_name": "eng-cc/oasis7"},
            "status": "completed",
            "conclusion": "success",
        }
        MODULE.validate_workflow_run_provenance(
            run, repository="eng-cc/oasis7", overlay=parsed, run_id=99, run_attempt=1,
            event_name="workflow_dispatch", workflow_ref="refs/heads/codex/test",
            workflow_sha=HEAD,
        )
        for key, value in (("id", 100), ("run_attempt", 2), ("workflow_id", 7),
                           ("head_sha", "3" * 40), ("event", "pull_request"),
                           ("conclusion", "failure")):
            changed = dict(run)
            changed[key] = value
            with self.subTest(key=key):
                with self.assertRaises(MODULE.OverlayError):
                    MODULE.validate_workflow_run_provenance(
                        changed, repository="eng-cc/oasis7", overlay=parsed, run_id=99,
                        run_attempt=1, event_name="workflow_dispatch",
                        workflow_ref="refs/heads/codex/test", workflow_sha=HEAD,
                    )

    def test_referenced_review_evidence_requires_canonical_typed_records(self):
        value = overlay()
        comments, review_records = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)
        bad = dict(review_records["qa"])
        bad["head_oid"] = BASE
        bad_body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(bad).decode() + "\n"
        comments[-1] = {**comments[-1], "body": bad_body}
        value["reviews"]["qa"]["body_sha256"] = "sha256:" + hashlib.sha256(bad_body.encode()).hexdigest()
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)

    def test_first_activation_plan_binds_all_dispatch_comments_and_path_scope(self):
        value = overlay()
        comments, _ = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)
        plan_comment, payload, plan_sha = MODULE._plan_comment(
            comments, ISSUE_URL, MODULE.validate_overlay(
                value, task_uid=UID, issue_number=4269, base_oid=BASE, head_oid=HEAD,
            ),
        )
        self.assertEqual(plan_sha, comments[2]["body"] and "sha256:" + hashlib.sha256(comments[2]["body"].encode()).hexdigest())
        wrapper = {"_overlay": parsed["_overlay"]}
        checked = MODULE._validate_first_review_plan(payload, wrapper, repository="eng-cc/oasis7",
                                                     issue_number=4269, comments=comments)
        self.assertEqual(checked["implementation_slices"][0]["dispatch_comments"][0]["comment_id"], 2)
        bad = json.loads(json.dumps(payload))
        bad["implementation_slices"][0]["dispatch_comments"].append(
            {"comment_id": 2, "body_sha256": SHA})
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_first_review_plan(bad, wrapper, repository="eng-cc/oasis7",
                                               issue_number=4269, comments=comments)

    def test_plan_history_selects_exact_head_and_rejects_duplicate_or_wrong_identity(self):
        value = overlay()
        comments, _ = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        stale = json.loads(comments[2]["body"].split("\n", 1)[1])
        stale["head_oid"] = BASE
        stale_body = MODULE.FIRST_REVIEW_PLAN_MARKER + "\n" + MODULE._canonical(stale).decode() + "\n"
        historical = _comment(30, stale_body, 30)
        selected, _, selected_digest = MODULE._plan_comment(
            [*comments, historical], ISSUE_URL, parsed,
        )
        self.assertEqual(selected["id"], comments[2]["id"])
        self.assertEqual(selected_digest, "sha256:" + hashlib.sha256(comments[2]["body"].encode()).hexdigest())
        with self.assertRaises(MODULE.OverlayError):
            MODULE._plan_comment([*comments, comments[2] | {"id": 31}], ISSUE_URL, parsed)
        with self.assertRaises(MODULE.OverlayError):
            MODULE._plan_comment([historical], ISSUE_URL, parsed)

    def test_typed_return_digest_must_resolve_to_matching_exact_head_record(self):
        value = overlay()
        comments, records = review_evidence(value)
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        bad = dict(records["qa"])
        bad["head_oid"] = BASE
        bad_body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(bad).decode() + "\n"
        comments[-1] = {**comments[-1], "body": bad_body}
        value["reviews"]["qa"]["body_sha256"] = "sha256:" + hashlib.sha256(bad_body.encode()).hexdigest()
        parsed = MODULE.validate_overlay(value, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_referenced_evidence("eng-cc/oasis7", {"number": 4269}, comments, parsed)

    def test_local_first_activation_checks_actual_packet_bytes_and_project_binding(self):
        import tempfile

        value = overlay()
        packet_values = {}
        raw_packets = {}
        for role, slice_id in (("qa_engineer", "first-qa"),
                               ("repository_health_engineer", "first-rh")):
            packet = {
                "schema": "oasis7-subagent-task-packet/v1",
                "identity": {
                    "task_uid": UID,
                    "head": HEAD,
                    "base_sha": BASE,
                    "base_binding": "immutable_oid",
                    "repository": "eng-cc/oasis7",
                    "issue_url": PACKET_ISSUE_URL,
                    "project_item_id": "project-item",
                    "branch": "codex/test",
                },
                "slice": {
                    "role": role,
                    "slice_id": slice_id,
                    "slice_type": "professional_review",
                    "write_scope": MODULE.FIRST_REVIEW_WRITE_SCOPE,
                },
            }
            packet["packet_digest"] = MODULE._packet_digest(packet)
            raw = (json.dumps(packet, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()
            raw_packets[role] = raw
            packet_values[role] = "sha256:" + hashlib.sha256(raw).hexdigest()
        comments, _ = review_evidence(value, packet_values)
        with tempfile.TemporaryDirectory() as temp:
            packet_dir = pathlib.Path(temp) / ".pm" / "scratch" / UID / "slice-packets"
            packet_dir.mkdir(parents=True)
            (packet_dir / "first-qa.json").write_bytes(raw_packets["qa_engineer"])
            (packet_dir / "first-rh.json").write_bytes(raw_packets["repository_health_engineer"])
            MODULE._validate_local_review_artifacts(
                pathlib.Path(temp), "eng-cc/oasis7", UID, "project-item", value, comments,
            )
            wrong_issue_value = overlay()
            wrong_issue_packet = json.loads(raw_packets["repository_health_engineer"])
            wrong_issue_packet["identity"]["issue_url"] = "https://github.com/eng-cc/oasis7/issues/4268"
            wrong_issue_packet["packet_digest"] = MODULE._packet_digest(wrong_issue_packet)
            wrong_issue_raw = (
                json.dumps(wrong_issue_packet, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
            ).encode()
            wrong_issue_digests = dict(packet_values)
            wrong_issue_digests["repository_health_engineer"] = (
                "sha256:" + hashlib.sha256(wrong_issue_raw).hexdigest()
            )
            wrong_issue_comments, _ = review_evidence(wrong_issue_value, wrong_issue_digests)
            (packet_dir / "first-rh.json").write_bytes(wrong_issue_raw)
            with self.assertRaisesRegex(MODULE.OverlayError, "not an exact read-only Task/base/head packet"):
                MODULE._validate_local_review_artifacts(
                    pathlib.Path(temp), "eng-cc/oasis7", UID, "project-item", wrong_issue_value,
                    wrong_issue_comments,
                )
            (packet_dir / "first-rh.json").write_bytes(raw_packets["repository_health_engineer"])
            (packet_dir / "first-rh.json").write_bytes(raw_packets["repository_health_engineer"] + b" ")
            with self.assertRaises(MODULE.OverlayError):
                MODULE._validate_local_review_artifacts(
                    pathlib.Path(temp), "eng-cc/oasis7", UID, "project-item", value, comments,
                )

    def test_activation_record_binds_exact_overlay_and_reconstructed_tier(self):
        parsed = MODULE.validate_overlay(overlay(), task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        parsed["overlay_comment_id"] = 44
        parsed["overlay_sha256"] = SHA
        proof = {
            "schema": "oasis7-cargo-dependency-floor-required-tier-proof/v1",
            "workflow_run_id": 99,
            "run_attempt": 1,
            "required_gate_job_id": 101,
            "required_gate_check_run_id": 102,
            "required_gate_check_app_id": 103,
            "inventory_digest": SHA,
            "obligation_count": 4,
            "workflow_logs_sha256": SHA,
        }
        payload = MODULE._activation_payload(parsed, proof, 99, 1)
        self.assertEqual((99, 1, proof), MODULE._validate_activation_payload(payload, parsed))
        payload["workflow"]["run_attempt"] = 2
        with self.assertRaises(MODULE.OverlayError):
            MODULE._validate_activation_payload(payload, parsed)

    def test_issue_only_activation_reader_reuses_exact_live_proof_without_project_claim(self):
        raw = overlay()
        parsed = MODULE.validate_overlay(raw, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        parsed["overlay_comment_id"] = 44
        parsed["overlay_sha256"] = "sha256:" + hashlib.sha256(
            MODULE.canonical_overlay_body(raw).encode(),
        ).hexdigest()
        with mock.patch.object(MODULE, "read_issue_overlay", return_value=parsed) as issue_reader, \
             mock.patch.object(MODULE, "_read_live_activation_proof", return_value={
                 **parsed, "activated": True, "required_tier_proof": {"workflow_run_id": 99},
             }) as proof_reader:
            result = MODULE.read_issue_activation(
                pathlib.Path("."), "eng-cc/oasis7", UID, BASE, HEAD, client=object(),
            )
        issue_reader.assert_called_once()
        proof_reader.assert_called_once()
        self.assertTrue(result["activated"])
        self.assertTrue(result["validation_only"])
        self.assertFalse(result["project_membership_verified"])
        self.assertNotIn("project_item_id", result)


class PayloadSnapshotValidationTests(unittest.TestCase):
    def test_payload_snapshot_hashes_canonical_request_and_acceptance_bytes(self):
        value = overlay()
        request_snapshot = {
            "identity": "Upgrade Wasmtime to patched 48.0.4",
            "acceptance": ["fresh executor verification", "existing lawful gates"],
        }
        value["snapshot_sha256"] = SHA
        value["request_sha256"] = "sha256:" + hashlib.sha256(
            MODULE._canonical(request_snapshot["identity"]),
        ).hexdigest()
        value["acceptance_sha256"] = "sha256:" + hashlib.sha256(
            MODULE._canonical(request_snapshot["acceptance"]),
        ).hexdigest()
        binding = {
            "snapshot": {"digest": SHA},
            "snapshot_task": {"uid": UID, "bootstrap_epoch": 1},
            "request_snapshot": request_snapshot,
            "task": {"issue_number": 4269, "repository": "eng-cc/oasis7"},
            "root": pathlib.Path("."),
            "client": type("Client", (), {
                "rest": lambda self, method, path, *args, **kwargs: {
                    "id": 123456, "path": ".github/workflows/rust.yml",
                }
            })(),
        }
        changed = [
            ".github/workflows/rust.yml",
            "Cargo.lock",
            "crates/oasis7_wasm_executor/Cargo.toml",
        ]

        def git(_root, *args):
            return {
                ("rev-parse", "HEAD"): HEAD,
                ("status", "--porcelain", "--untracked-files=all"): "",
                ("symbolic-ref", "--quiet", "--short", "HEAD"): "codex/test",
            }[tuple(args)]

        with mock.patch.object(MODULE, "_comments", return_value=[]), \
             mock.patch.object(MODULE, "_git", side_effect=git), \
             mock.patch.object(MODULE, "_changed_paths", return_value=changed), \
             mock.patch.object(MODULE, "_git_blob_sha256", return_value=SHA):
            parsed = MODULE._validate_payload_snapshot(value, binding, BASE, HEAD)
            self.assertTrue(parsed["validation_only"])
            for field in ("request_sha256", "acceptance_sha256"):
                bad = dict(value)
                bad[field] = "sha256:" + "b" * 64
                with self.subTest(field=field), self.assertRaisesRegex(
                    MODULE.OverlayError, "immutable Task epoch, snapshot, request, or acceptance",
                ):
                    MODULE._validate_payload_snapshot(bad, binding, BASE, HEAD)


class IssueOverlayHistoryTests(unittest.TestCase):
    OLD_HEAD = "3" * 40

    class Client:
        def __init__(self, comments: list[dict]):
            self.comments = comments

        def rest(self, method: str, path: str, **_kwargs):
            if path.startswith("repos/eng-cc/oasis7/issues?state=all"):
                return [{
                    "id": 4269,
                    "number": 4269,
                    "state": "open",
                    "body": f"<!-- oasis7-pm-task -->\ntask_uid: {UID}\n",
                }] if "page=1" in path else []
            if path.startswith("repos/eng-cc/oasis7/issues/4269/comments?"):
                return self.comments if "page=1" in path else []
            raise AssertionError(f"unexpected request: {method} {path}")

    @staticmethod
    def _history(current: dict | None = None, *, duplicate_current: bool = False,
                 current_reviews: dict | None = None) -> list[dict]:
        old = overlay(head=IssueOverlayHistoryTests.OLD_HEAD)
        old_comments, _ = review_evidence(old, head=IssueOverlayHistoryTests.OLD_HEAD)
        old_comment = _comment(40, MODULE.canonical_overlay_body(old), 8)
        selected = current if current is not None else overlay()
        current_comments, _ = review_evidence(selected, head=HEAD, offset=10)
        if current_reviews is not None:
            selected["reviews"] = current_reviews
        current_comment = _comment(41, MODULE.canonical_overlay_body(selected), 18)
        comments = [*old_comments, *current_comments[1:], old_comment, current_comment]
        if duplicate_current:
            comments.append(_comment(42, current_comment["body"], 19))
        return comments

    def _read(self, comments: list[dict], *, head: str = HEAD) -> dict:
        return MODULE.read_issue_overlay(
            pathlib.Path("."), "eng-cc/oasis7", UID, BASE, head,
            client=self.Client(comments),
        )

    @staticmethod
    def _publish_fixture():
        old = overlay(head=IssueOverlayHistoryTests.OLD_HEAD)
        old_comments, _ = review_evidence(old, head=IssueOverlayHistoryTests.OLD_HEAD)
        old_comment = _comment(40, MODULE.canonical_overlay_body(old), 8)
        current = overlay()
        current_comments, _ = review_evidence(current, head=HEAD, offset=10)
        comments = [*old_comments, *current_comments[1:], old_comment]
        posted_body: list[str] = []

        class Client:
            def rest(self, method: str, path: str, payload=None, **_kwargs):
                if method == "POST" and path == "repos/eng-cc/oasis7/issues/4269/comments":
                    posted_body.append(payload["body"])
                    return {"id": 50}
                if method == "GET" and path == "repos/eng-cc/oasis7/issues/comments/50":
                    return {
                        "id": 50,
                        "issue_url": ISSUE_URL,
                        "body": posted_body[-1],
                        "user": {"login": "human"},
                    }
                raise AssertionError(f"unexpected request: {method} {path}")

        binding = {
            "client": Client(),
            "issue": {"number": 4269},
            "task": {"issue_number": 4269},
        }
        return current, comments, posted_body, binding

    def test_old_head_history_is_preserved_but_only_latest_exact_head_is_selected(self):
        selected = self._read(self._history())
        self.assertEqual(selected["head_oid"], HEAD)
        self.assertEqual(selected["overlay_comment_id"], 41)
        with self.assertRaisesRegex(MODULE.OverlayError, "older.*cannot authorize"):
            self._read(self._history(), head=self.OLD_HEAD)

    def test_duplicate_current_overlay_fails_closed(self):
        with self.assertRaisesRegex(MODULE.OverlayError, "duplicate.*overlays"):
            self._read(self._history(duplicate_current=True))

    def test_current_overlay_must_preserve_authorized_task_and_package_binding(self):
        changed = overlay()
        changed["raw_primary_package"] = {"present": True, "value": "oasis7"}
        with self.assertRaisesRegex(MODULE.OverlayError, "changes immutable Task"):
            self._read(self._history(changed))

    def test_current_head_cannot_reuse_previous_head_review_evidence(self):
        old = overlay(head=self.OLD_HEAD)
        review_evidence(old, head=self.OLD_HEAD)
        with self.assertRaisesRegex(MODULE.OverlayError, "not bound to the exact Task, Issue, base, head, or role"):
            self._read(self._history(current_reviews=old["reviews"]))

    def test_publisher_appends_new_exact_head_without_replacing_history(self):
        payload, comments, posted_body, binding = self._publish_fixture()
        parsed = MODULE.validate_overlay(payload, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        with mock.patch.object(MODULE, "_read_project_task_binding", return_value=binding), \
             mock.patch.object(MODULE, "_validate_payload_snapshot", return_value=parsed), \
             mock.patch.object(MODULE, "_comments", return_value=comments), \
             mock.patch.object(MODULE, "_validate_referenced_evidence"), \
             mock.patch.object(MODULE, "_authenticated_login", return_value="human"), \
             mock.patch.object(MODULE, "read_project_overlay", return_value={"head_oid": HEAD}) as readback:
            result = MODULE.publish_overlay(pathlib.Path("."), "eng-cc/oasis7", UID,
                                            BASE, HEAD, payload, mapping_path=pathlib.Path("mapping"))
        self.assertEqual(result, {"head_oid": HEAD})
        self.assertEqual(len(posted_body), 1)
        self.assertEqual(posted_body[0], MODULE.canonical_overlay_body(payload))
        self.assertTrue(any(item["id"] == 40 for item in comments))
        readback.assert_called_once()

    def test_publisher_refuses_another_sequence_after_activation_evidence(self):
        payload, comments, posted_body, binding = self._publish_fixture()
        comments.append(_comment(49, MODULE.ACTIVATION_MARKER + "\n{}", 19))
        parsed = MODULE.validate_overlay(payload, task_uid=UID, issue_number=4269,
                                         base_oid=BASE, head_oid=HEAD)
        with mock.patch.object(MODULE, "_read_project_task_binding", return_value=binding), \
             mock.patch.object(MODULE, "_validate_payload_snapshot", return_value=parsed), \
             mock.patch.object(MODULE, "_comments", return_value=comments):
            with self.assertRaisesRegex(MODULE.OverlayError, "activated.*cannot start"):
                MODULE.publish_overlay(pathlib.Path("."), "eng-cc/oasis7", UID,
                                       BASE, HEAD, payload, mapping_path=pathlib.Path("mapping"))
        self.assertEqual(posted_body, [])

    def test_amendment_selects_only_current_v2_head_and_old_v1_becomes_audit_only(self):
        prior_payload = overlay(head=self.OLD_HEAD)
        prior_comment = _comment(20, MODULE.canonical_overlay_body(prior_payload), 8)
        prior_parsed = MODULE.validate_overlay(
            prior_payload, task_uid=UID, issue_number=4269,
            base_oid=BASE, head_oid=self.OLD_HEAD,
        )
        amendment = MODULE.validate_scope_amendment_payload(
            scope_amendment(), task_uid=UID, issue_number=4269,
        )
        current_payload = amended_overlay()
        current_comment = _comment(40, MODULE.canonical_overlay_body(current_payload), 18)
        current_parsed = MODULE.validate_overlay(
            current_payload, task_uid=UID, issue_number=4269,
            base_oid=BASE, head_oid=HEAD, scope_amendment=amendment,
        )
        amendment_comment = _comment(30, MODULE.canonical_scope_amendment_body(scope_amendment()), 10)
        history = [
            (prior_comment, prior_payload, prior_parsed),
            (current_comment, current_payload, current_parsed),
        ]
        client = self.Client([prior_comment, amendment_comment, current_comment])
        with mock.patch.object(MODULE, "_validated_overlay_history", return_value=history):
            selected = MODULE.read_issue_overlay(
                pathlib.Path("."), "eng-cc/oasis7", UID, BASE, HEAD, client=client,
            )
            self.assertEqual(selected["schema"], MODULE.OVERLAY_V2_SCHEMA)
            self.assertEqual(selected["head_requirement"], "49.0.2")
            self.assertEqual(selected["overlay_comment_id"], 40)
            with self.assertRaisesRegex(MODULE.OverlayError, "superseded"):
                MODULE.read_issue_overlay(
                    pathlib.Path("."), "eng-cc/oasis7", UID, BASE, self.OLD_HEAD,
                    client=client,
                )


class ProjectActivationMarkerTests(unittest.TestCase):
    REPOSITORY = "eng-cc/oasis7"
    ISSUE_NUMBER = 4269
    PROJECT_ID = "PVT_test"
    ITEM_ID = "PVTI_test"
    ISSUE_BODY = f"task_uid: {UID}\n- acceptance: `test acceptance`\n"

    @classmethod
    def _item(cls, *, item_id=None, issue_number=None, body=None, repository=None,
              state="OPEN", archived=False):
        number = cls.ISSUE_NUMBER if issue_number is None else issue_number
        return {
            "id": item_id or cls.ITEM_ID,
            "isArchived": archived,
            "content": {
                "__typename": "Issue",
                "number": number,
                "url": f"https://github.com/{repository or cls.REPOSITORY}/issues/{number}",
                "state": state,
                "body": cls.ISSUE_BODY if body is None else body,
                "repository": {"nameWithOwner": repository or cls.REPOSITORY},
            },
        }

    @classmethod
    def _project_page(cls, items, *, has_next=False, cursor=None, project_id=None):
        return {"node": {
            "id": project_id or cls.PROJECT_ID,
            "number": 1,
            "owner": {"login": "eng-cc"},
            "items": {"nodes": items,
                      "pageInfo": {"hasNextPage": has_next, "endCursor": cursor}},
        }}

    class Client:
        def __init__(self, pages, *, issue=None, comments=None, graphql_error=None,
                     rest_error=None, comments_error=None):
            self.pages = pages
            self.issue = issue
            self.comments = comments or []
            self.graphql_error = graphql_error
            self.rest_error = rest_error
            self.comments_error = comments_error
            self.graphql_calls = []
            self.rest_calls = []

        def graphql(self, query, variables, *, operation):
            self.graphql_calls.append((query, variables, operation))
            if self.graphql_error:
                raise self.graphql_error
            if variables.get("after") not in self.pages:
                raise AssertionError(f"unexpected Project cursor: {variables.get('after')}")
            return self.pages[variables.get("after")]

        def rest(self, method, path, payload=None, **kwargs):
            self.rest_calls.append((method, path, kwargs.get("operation")))
            if method == "GET" and path.startswith(
                    f"repos/{ProjectActivationMarkerTests.REPOSITORY}/issues/{ProjectActivationMarkerTests.ISSUE_NUMBER}/comments?"):
                if self.comments_error:
                    raise self.comments_error
                return self.comments
            if self.rest_error:
                raise self.rest_error
            if method == "GET" and path == f"repos/{ProjectActivationMarkerTests.REPOSITORY}/issues/{ProjectActivationMarkerTests.ISSUE_NUMBER}":
                return self.issue
            raise AssertionError(f"unexpected request: {method} {path}")

    def _binding(self, client, *, item_id=None, issue_number=None):
        return {
            "project": {"id": self.PROJECT_ID, "number": 1, "owner": "eng-cc"},
            "task": {"task_uid": UID, "issue_number": issue_number or self.ISSUE_NUMBER,
                     "project_item_id": item_id or self.ITEM_ID},
            "snapshot": {"repository": self.REPOSITORY},
            "snapshot_task": {
                "uid": UID,
                "issue": {"number": self.ISSUE_NUMBER,
                          "url": PACKET_ISSUE_URL},
                "project": {"item_id": self.ITEM_ID, "number": 1, "owner": "eng-cc"},
            },
            "issue": {"number": self.ISSUE_NUMBER,
                      "url": ISSUE_URL, "state": "open", "body": self.ISSUE_BODY},
            "client": client,
        }

    def _read_marker(self, client, binding):
        with mock.patch.object(MODULE, "_read_project_task_binding", return_value=binding):
            return MODULE.read_project_activation_marker(
                pathlib.Path("."), self.REPOSITORY, UID,
                mapping_path=pathlib.Path("mapping.json"), client=client,
            )

    def _client(self, items, *, comments=None, extra_pages=None, **kwargs):
        pages = {None: self._project_page(items)}
        pages.update(extra_pages or {})
        issue = {
            "number": self.ISSUE_NUMBER,
            "url": ISSUE_URL,
            "state": "open",
            "body": self.ISSUE_BODY,
        }
        return self.Client(pages, issue=issue, comments=comments, **kwargs)

    def test_absent_marker_uses_complete_unique_live_project_and_issue_binding(self):
        unrelated = self._item(item_id="PVTI_other", issue_number=777,
                                body="task_uid: task_" + "b" * 32 + "\n")
        second_page = self._project_page([self._item()], project_id=self.PROJECT_ID)
        client = self._client([unrelated], extra_pages={"next-page": second_page})
        client.pages[None] = self._project_page([unrelated], has_next=True, cursor="next-page")

        self.assertFalse(self._read_marker(client, self._binding(client)))
        self.assertEqual(len(client.graphql_calls), 2)
        self.assertTrue(any(path == f"repos/{self.REPOSITORY}/issues/{self.ISSUE_NUMBER}"
                            for method, path, _ in client.rest_calls if method == "GET"))
        self.assertTrue(any(path.startswith(
            f"repos/{self.REPOSITORY}/issues/{self.ISSUE_NUMBER}/comments?")
                            for method, path, _ in client.rest_calls))

    def test_overlay_or_activation_marker_is_only_reported_after_binding(self):
        for marker in (*MODULE.OVERLAY_MARKERS, MODULE.ACTIVATION_MARKER):
            with self.subTest(marker=marker):
                comment = {"id": 1, "issue_url": ISSUE_URL, "body": marker + "\n{}"}
                client = self._client([self._item()], comments=[comment])
                self.assertTrue(self._read_marker(client, self._binding(client)))

    def test_project_graphql_issue_url_uses_browser_identity_not_rest_api_identity(self):
        wrong_shape = self._item()
        wrong_shape["content"]["url"] = ISSUE_URL
        client = self._client([wrong_shape])
        with self.assertRaisesRegex(MODULE.OverlayError, "unique live Project item"):
            self._read_marker(client, self._binding(client))

    def test_missing_duplicate_or_mismatched_project_uid_binding_fails_closed(self):
        cases = {
            "missing item": [],
            "duplicate UID on another issue": [
                self._item(), self._item(item_id="PVTI_duplicate", issue_number=4270),
            ],
            "duplicate mapped issue item": [
                self._item(), self._item(item_id="PVTI_duplicate", body="no task marker\n"),
            ],
            "foreign UID on mapped issue": [
                self._item(body="task_uid: task_" + "c" * 32 + "\n"),
            ],
            "mapped item ID mismatch": [self._item()],
        }
        for label, items in cases.items():
            with self.subTest(label=label):
                client = self._client(items)
                binding = self._binding(
                    client, item_id="PVTI_wrong" if label == "mapped item ID mismatch" else None,
                )
                with self.assertRaises(MODULE.OverlayError):
                    self._read_marker(client, binding)

        client = self._client([self._item()])
        with self.assertRaisesRegex(MODULE.OverlayError, "snapshot"):
            self._read_marker(client, self._binding(client, issue_number=4270))

    def test_duplicate_task_uid_on_later_project_page_is_not_hidden(self):
        first = self._item()
        duplicate = self._item(item_id="PVTI_later_duplicate", issue_number=4270)
        client = self._client([first], extra_pages={
            "next-page": self._project_page([duplicate]),
        })
        client.pages[None] = self._project_page([first], has_next=True, cursor="next-page")
        with self.assertRaisesRegex(MODULE.OverlayError, "exactly one live Project item"):
            self._read_marker(client, self._binding(client))

    def test_malformed_project_pagination_identity_and_rest_errors_fail_closed(self):
        valid_item = self._item()
        wrong_project = self._client([valid_item])
        wrong_project.pages[None] = self._project_page([valid_item], project_id="PVT_wrong")
        broken_cursor = self._client([valid_item])
        broken_cursor.pages[None] = self._project_page([valid_item], has_next=True, cursor=None)
        changed_issue = self._client([valid_item])
        changed_issue.issue = {**changed_issue.issue, "body": "task_uid: task_" + "d" * 32 + "\n"}
        wrong_comment = self._client([valid_item], comments=[{"id": 1, "body": "ordinary"}])
        comments_failure = self._client([valid_item], comments_error=RuntimeError("comments failure"))
        cases = (
            (wrong_project, self._binding(wrong_project)),
            (broken_cursor, self._binding(broken_cursor)),
            (changed_issue, self._binding(changed_issue)),
            (wrong_comment, self._binding(wrong_comment)),
            (self._client([valid_item], graphql_error=RuntimeError("GraphQL failure")),
             self._binding(self._client([valid_item]))),
            (self._client([valid_item], rest_error=RuntimeError("REST failure")),
             self._binding(self._client([valid_item]))),
            (comments_failure, self._binding(comments_failure)),
        )
        for client, binding in cases:
            binding["client"] = client
            with self.subTest(error=client.graphql_error or client.rest_error or "identity"):
                with self.assertRaises((MODULE.OverlayError, RuntimeError)):
                    self._read_marker(client, binding)


class ProductionGateActivationRouteTests(unittest.TestCase):
    def _run(self, marker_result, *, issue_hint=False):
        data = {
            "number": 4269,
            "repository": "eng-cc/oasis7",
            "baseRefOid": BASE,
            "headRefOid": HEAD,
            "state": "OPEN",
            "isDraft": False,
            "body": f"Task: {UID}",
            "baseRefName": "main",
            "headRefName": "codex/test",
            "policy_discovery": {"status": "resolved", "required_status_checks": []},
        }
        pending = {"ready_for_merge": True, "status": "ready", "blockers": []}
        final = {**pending, "readiness_receipt": {"head_oid": HEAD}}
        helper = mock.Mock()
        helper.read_project_activation_marker.return_value = marker_result
        helper.OVERLAY_MARKER = MODULE.OVERLAY_MARKER
        with mock.patch.object(GATE, "decision", side_effect=[pending, final]), \
             mock.patch.object(GATE, "live_target_oid", return_value=BASE), \
             mock.patch.object(GATE, "local_loop_admission", return_value={"status": "legacy"}), \
             mock.patch.object(GATE, "live_integration_admission", return_value=None), \
             mock.patch.object(GATE, "read_pr_identity", return_value=data), \
             mock.patch.object(GATE, "_load_effective_helper", return_value=helper):
            result = GATE.production_decision(
                data, False, ROOT, UID, str(ROOT), api_client=object(),
                activation_overlay_present=issue_hint,
            )
        return result, helper

    def test_negative_local_issue_hint_still_requires_fresh_project_reader(self):
        result, helper = self._run(False, issue_hint=False)
        self.assertTrue(result["ready_for_merge"], result)
        helper.read_project_activation_marker.assert_called_once()
        helper.read_project_activation.assert_not_called()

    def test_positive_canonical_marker_requires_exact_head_activation_proof(self):
        result, helper = self._run(True, issue_hint=False)
        self.assertTrue(result["ready_for_merge"], result)
        helper.read_project_activation_marker.assert_called_once()
        helper.read_project_activation.assert_called_once_with(
            ROOT, "eng-cc/oasis7", UID, BASE, HEAD,
            mapping_path=ROOT / ".pm/github-project-sync/tasks.json",
            client=mock.ANY,
        )

    def test_project_reader_errors_block_production_readiness(self):
        data = {
            "number": 4269, "repository": "eng-cc/oasis7", "baseRefOid": BASE,
            "headRefOid": HEAD, "state": "OPEN", "isDraft": False,
            "body": f"Task: {UID}", "baseRefName": "main", "headRefName": "codex/test",
            "policy_discovery": {"status": "resolved", "required_status_checks": []},
        }
        pending = {"ready_for_merge": True, "status": "ready", "blockers": []}
        helper = mock.Mock()
        helper.read_project_activation_marker.side_effect = MODULE.OverlayError("Project readback failed")
        with mock.patch.object(GATE, "decision", return_value=pending), \
             mock.patch.object(GATE, "live_target_oid", return_value=BASE), \
             mock.patch.object(GATE, "_load_effective_helper", return_value=helper):
            result = GATE.production_decision(
                data, False, ROOT, UID, str(ROOT), api_client=object(),
                activation_overlay_present=False,
            )
        self.assertFalse(result["ready_for_merge"], result)
        self.assertIn("Project readback failed", " ".join(result["blockers"]))


class RequiredRunLeafProofTests(unittest.TestCase):
    HEADER = "historical_required_location\ttest_paths\tnew_required_selection\tlegacy_required_coverage\tfull_full_core_full_support\n"

    @staticmethod
    def _git(root: pathlib.Path, *args: str) -> str:
        return subprocess.run(["git", "-C", str(root), *args], check=True,
                              text=True, capture_output=True).stdout.strip()

    def _fixture(self, temp: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path, str, str, dict, str]:
        root = temp / "repository"
        root.mkdir()
        self._git(root, "init", "-b", "codex/test")
        self._git(root, "config", "user.name", "Activation Test")
        self._git(root, "config", "user.email", "activation-test@example.invalid")
        (root / "scripts/pm").mkdir(parents=True)
        (root / "scripts").mkdir(exist_ok=True)
        base_row = (
            "run_workflow_governance_baseline_contract_tests\t"
            "scripts/ci-required-baseline-routing.test.sh,scripts/ci-required-domain-isolation.test.sh\t"
            "workflow_governance\tbaseline retained\tall full groups\n"
        )
        inventory_path = root / "scripts/ci-required-capability-test-inventory.tsv"
        inventory_path.write_text(self.HEADER + base_row, encoding="utf-8")
        (root / "scripts/plan-rust-required-scope.py").write_text(
            "print('scope=full')\n", encoding="utf-8",
        )
        test_inventory = {
            "unit_specs": [
                {
                    "unit_id": "required_gate_baseline",
                    "obligation_set": ["required_gate_baseline:000:required-domain-selector-validation"],
                    "unit_contract": {"commands_and_obligations": [
                        "document-corpus-v3-check", "product-doc-changed-range", "product-doc-full-corpus",
                        "workflow-process-identity", "lint-skills", "windows-paths", "script-executable-bits",
                        "workflow-impact-projection-consumer", "cargo-package-scope-and-profile-completion",
                        "unified-world-terminology", "rust-file-size-regression-and-check",
                        "required-domain-selector-validation", "scripts/ci-required-baseline-routing.test.sh",
                        "scripts/ci-required-domain-isolation.test.sh", "scripts/check-rust-file-size.test.sh",
                        "scripts/check-rust-file-size.sh", "scripts/unified-world-code-terminology-scan.sh",
                    ]},
                },
                {
                    "unit_id": "oasis7_required",
                    "obligation_set": ["oasis7_required:000:cargo-test"],
                    "unit_contract": {"commands_and_obligations": [
                        "cargo test -p oasis7 --tests --features test_tier_required",
                    ]},
                },
                {
                    "unit_id": "workflow_governance",
                    "obligation_set": ["workflow_governance:000:baseline-routing-test"],
                    "unit_contract": {"commands_and_obligations": [
                        "scripts/ci-required-baseline-routing.test.sh",
                    ]},
                },
                {
                    "unit_id": "product_document:doc/product/example.prd.md",
                    "obligation_set": ["product_document:doc/product/example.prd.md"],
                    "unit_contract": {"commands_and_obligations": [
                        "product-document:doc/product/example.prd.md",
                    ]},
                },
            ],
            "planner_output": {"scope": "full"},
            "planner_inventory_issuer": {"inventory_digest": SHA},
        }
        module_source = (
            "def build_required_inventory(*args, **kwargs):\n"
            f"    return {test_inventory!r}\n"
        )
        (root / "scripts/pm/ci_required_inventory.py").write_text(module_source, encoding="utf-8")
        (root / ".github").mkdir()
        (root / ".github/workflows").mkdir()
        (root / ".github/workflows/rust.yml").write_text("name: Rust\n", encoding="utf-8")
        (root / "scripts/ci-tests.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
        self._git(root, "add", ".")
        self._git(root, "commit", "-m", "trusted base fixture")
        base_oid = self._git(root, "rev-parse", "HEAD")
        (root / "scripts/pm/first_activation.test.py").write_text("# candidate test\n", encoding="utf-8")
        inventory_path.write_text(
            self.HEADER + base_row
            + "run_workflow_governance_operational_contract_tests\t"
            + "scripts/pm/first_activation.test.py\tworkflow_governance\tadditive test\tall full groups\n",
            encoding="utf-8",
        )
        (root / "scripts/ci-tests.sh").write_text("#!/usr/bin/env bash\n# candidate runner\n", encoding="utf-8")
        self._git(root, "add", ".")
        self._git(root, "commit", "-m", "candidate head fixture")
        head_oid = self._git(root, "rev-parse", "HEAD")
        base_worktree = temp / "trusted-base"
        self._git(root, "worktree", "add", "--detach", str(base_worktree), base_oid)
        proof_overlay = {
            "task_uid": UID, "issue_number": 4269, "base_oid": base_oid, "head_oid": head_oid,
            "workflow_id": 123456, "workflow_path": ".github/workflows/rust.yml",
            "workflow_ref": "refs/heads/codex/test", "workflow_sha": head_oid,
            "workflow_change_paths": [
                {"path": "scripts/ci-required-capability-test-inventory.tsv"},
                {"path": "scripts/pm/first_activation.test.py"},
            ],
        }
        logs = [
            "required-gate\tRun required test tier\t2026-10-03T00:00:01Z\t+ ./scripts/doc-governance-check.sh --full-corpus",
            "required-gate\tRun required test tier\t2026-10-03T00:00:02Z\tproduct-doc-content: checked 0: reason=no new or substantive product-document changes in selected range",
            "required-gate\tRun required test tier\t2026-10-03T00:00:03Z\tdocument-corpus-inventory-check: OK",
            "required-gate\tRun required test tier\t2026-10-03T00:00:04Z\tworkflow-process-identity-check: PASS",
            "required-gate\tRun required test tier\t2026-10-03T00:00:05Z\tproduct-doc-content: OK (full-corpus checked 4 current-tree product documents)",
            "required-gate\tRun required test tier\t2026-10-03T00:00:06Z\t+ ./scripts/lint-skills.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:07Z\tlint-skills: OK (8 default skill entrypoints, 2 library skill entries checked)",
            "required-gate\tRun required test tier\t2026-10-03T00:00:08Z\t+ ./scripts/check-windows-paths.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:09Z\tok: checked 123 tracked paths for Windows checkout compatibility",
            "required-gate\tRun required test tier\t2026-10-03T00:00:10Z\t+ bash ./scripts/check-script-executable-bits.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:11Z\tok: required release scripts are tracked and executable",
            f"required-gate\tRun required test tier\t2026-10-03T00:00:12Z\t+ python3 - /tmp/impact-projection.json {root} {root}/scripts",
            "required-gate\tRun required test tier\t2026-10-03T00:00:13Z\tworkflow-impact-projection-consumer: verified status",
            f"required-gate\tRun required test tier\t2026-10-03T00:00:14Z\t+ python3 /tmp/trusted-check-cargo-package-scope --repo-root {root} --base {base_oid} --head {head_oid} --primary-package auto --policy {root}/.pm/cargo-package-scope-policy.json --json",
            'required-gate\tRun required test tier\t2026-10-03T00:00:15Z\t{"status":"rejected","reason":"ambiguous_package_attribution"}',
            "required-gate\tRun required test tier\t2026-10-03T00:00:16Z\tfirst-activation trusted checker observation: exit=1 reason=ambiguous_package_attribution status=failed",
            f"required-gate\tRun required test tier\t2026-10-03T00:00:17Z\t+ python3 {root}/scripts/pm/check-cargo-package-scope --repo-root {root} --base {base_oid} --head {head_oid} --first-activation-task-uid {UID} --policy {root}/.pm/cargo-package-scope-policy.json --json",
            f'required-gate\tRun required test tier\t2026-10-03T00:00:18Z\t{{"status":"allowed","primary_package":"oasis7","mode":"dependency_floor_update","task_uid":"{UID}","validation_only":true}}',
            "required-gate\tRun required test tier\t2026-10-03T00:00:19Z\tfirst-activation candidate checker observation: status=passed exit=0 validation_only=true",
            "required-gate\tRun required test tier\t2026-10-03T00:00:20Z\tcargo-package-scope-and-profile-completion: activated candidate checker passed",
            "required-gate\tRun required test tier\t2026-10-03T00:00:16Z\t+ ./scripts/unified-world-code-terminology-scan.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:17Z\tunified-world-code-terminology-scan: OK",
            "required-gate\tRun required test tier\t2026-10-03T00:00:18Z\t+ ./scripts/check-rust-file-size.test.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:19Z\t+ ./scripts/check-rust-file-size.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:20Z\tcheck-rust-file-size: OK",
            "required-gate\tRun required test tier\t2026-10-03T00:00:21Z\t+ bash ./scripts/ci-required-baseline-routing.test.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:22Z\t+ bash ./scripts/ci-required-domain-isolation.test.sh",
            "required-gate\tRun required test tier\t2026-10-03T00:00:23Z\t+ env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required --verbose",
            "required-gate\tRun required test tier\t2026-10-03T00:00:24Z\t+ python3 ./scripts/pm/first_activation.test.py",
        ]
        return root, base_worktree, base_oid, head_oid, proof_overlay, "\n".join(logs) + "\n"

    def test_trusted_inventory_import_restores_bytecode_setting_when_loader_raises(self):
        observed_settings = []

        class FailingLoader:
            def exec_module(self, _module):
                observed_settings.append(sys.dont_write_bytecode)
                raise RuntimeError("synthetic inventory loader failure")

        with mock.patch.object(sys, "dont_write_bytecode", False):
            with self.assertRaisesRegex(RuntimeError, "synthetic inventory loader failure"):
                MODULE._exec_trusted_inventory_module(FailingLoader(), object())
            self.assertEqual(observed_settings, [True])
            self.assertIs(sys.dont_write_bytecode, False)

    def test_full_reconstruction_uses_actual_job_step_commands_and_additive_candidate_test(self):
        class Client:
            def __init__(self, fail_step=False):
                self.fail_step = fail_step

            def rest(self, method, path, *args, **kwargs):
                if "/actions/runs/" in path and "/attempts/" not in path:
                    return {
                        "id": 99, "run_attempt": 1, "event": "workflow_dispatch",
                        "workflow_id": 123456, "path": ".github/workflows/rust.yml",
                        "head_branch": "codex/test", "head_sha": self.head_oid,
                        "repository": {"full_name": "eng-cc/oasis7"},
                        "status": "completed", "conclusion": "success",
                    }
                if "/attempts/1/jobs?" in path:
                    conclusion = "failure" if self.fail_step else "success"
                    return [{
                        "id": 101, "name": "required-gate", "status": "completed",
                        "conclusion": "success", "check_run_url": "https://api.github.com/repos/eng-cc/oasis7/check-runs/102",
                        "steps": [{"name": "Run required test tier", "status": "completed",
                                   "conclusion": conclusion}],
                    }]
                if path == "repos/eng-cc/oasis7/check-runs/102":
                    return {"name": "required-gate", "head_sha": self.head_oid,
                            "status": "completed", "conclusion": "success", "app": {"id": 103}}
                if path == "repos/eng-cc/oasis7":
                    return {"default_branch": "main"}
                raise AssertionError(f"unexpected GitHub API call: {path}")

        with tempfile.TemporaryDirectory() as temp_name:
            temp = pathlib.Path(temp_name)
            root, base_worktree, base_oid, head_oid, bound_overlay, logs = self._fixture(temp)
            client = Client()
            client.head_oid = head_oid

            def reconstruct(log_text, *, run_client=client):
                with mock.patch.object(sys, "dont_write_bytecode", False), \
                     mock.patch.object(sys, "pycache_prefix", None):
                    try:
                        return MODULE.reconstruct_full_required_run(
                            root, "eng-cc/oasis7", UID, bound_overlay, 99, 1,
                            client=run_client,
                            log_reader=lambda _run, _attempt: log_text,
                        )
                    finally:
                        self.assertIs(sys.dont_write_bytecode, False)
                        self.assertIsNone(sys.pycache_prefix)
                        self.assertEqual(
                            self._git(base_worktree, "status", "--porcelain", "--untracked-files=all"),
                            "",
                            "trusted frozen-base worktree must remain clean after every reconstruction",
                        )

            positive = reconstruct(logs)
            self.assertEqual(positive["workflow_run_id"], 99)
            self.assertGreater(positive["obligation_count"], 1)
            self.assertRegex(positive["workflow_logs_sha256"], r"^sha256:[0-9a-f]{64}$")

            echo_decoy = logs.replace(
                "+ ./scripts/lint-skills.sh",
                "+ echo ./scripts/lint-skills.sh",
            )
            with self.assertRaisesRegex(MODULE.OverlayError, "actual command mapped to lint-skills"):
                reconstruct(echo_decoy)

            cargo_skip = logs.replace(
                "--features test_tier_required --verbose\n",
                "--features test_tier_required --verbose -- --skip offline_server_accepts_client_and_emits_snapshot_and_event\n",
            )
            with self.assertRaisesRegex(MODULE.OverlayError, "direct command for oasis7_required"):
                reconstruct(cargo_skip)

            missing_test = "\n".join(
                line for line in logs.splitlines() if "first_activation.test.py" not in line
            )
            with self.assertRaisesRegex(MODULE.OverlayError, "candidate-added test command"):
                reconstruct(missing_test)
            wrong_job = logs.replace("required-gate\tRun required test tier", "other-job\tRun required test tier")
            with self.assertRaisesRegex(MODULE.OverlayError, "required-gate Run required test tier"):
                reconstruct(wrong_job)
            failed = Client(fail_step=True)
            failed.head_oid = head_oid
            with self.assertRaisesRegex(MODULE.OverlayError, "required-gate has a failed"):
                reconstruct(logs, run_client=failed)

    def test_checker_leaf_requires_exact_commands_observations_and_candidate_binding(self):
        overlay_value = {"task_uid": UID, "base_oid": BASE, "head_oid": HEAD}
        lines = [
            f"+ python3 /tmp/trusted-check-cargo-package-scope --repo-root {ROOT} --base {BASE} --head {HEAD} --primary-package auto --policy {ROOT}/.pm/cargo-package-scope-policy.json --json",
            '{"status":"rejected","reason":"ambiguous_package_attribution"}',
            "ordinary pull-request trusted checker observation: exit=1 reason=ambiguous_package_attribution status=failed",
            f"+ python3 ./scripts/pm/check-cargo-package-scope --repo-root {ROOT} --base {BASE} --head {HEAD} --first-activation-task-uid {UID} --policy {ROOT}/.pm/cargo-package-scope-policy.json --json",
            f'{{"status":"allowed","mode":"dependency_floor_update","task_uid":"{UID}","validation_only":true}}',
            "ordinary pull-request candidate checker observation: status=passed exit=0 validation_only=true",
        ]
        self.assertTrue(MODULE._valid_checker_json_leaf(lines, overlay_value, candidate_root=ROOT))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace("ambiguous_package_attribution", "checker_internal_error") for line in lines],
            overlay_value, candidate_root=ROOT,
        ))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace(HEAD, "3" * 40) for line in lines], overlay_value, candidate_root=ROOT,
        ))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace(UID, "task_" + "b" * 32) for line in lines], overlay_value,
            candidate_root=ROOT,
        ))
        self.assertFalse(MODULE._valid_checker_json_leaf(
            [line.replace("--json", "--json --skip unreviewed-option") for line in lines],
            overlay_value, candidate_root=ROOT,
        ))

    def test_command_identity_rejects_echo_and_accepts_runner_wrappers(self):
        self.assertFalse(MODULE._command_is_logged(
            ["+ echo ./scripts/lint-skills.sh"], "./scripts/lint-skills.sh",
        ))
        self.assertFalse(MODULE._path_is_logged(
            ["+ echo ./scripts/lint-skills.sh"], "./scripts/lint-skills.sh",
        ))
        self.assertTrue(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required --verbose"],
            "cargo test -p oasis7 --tests --features test_tier_required",
        ))
        self.assertFalse(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required --verbose -- --skip offline_test"],
            "cargo test -p oasis7 --tests --features test_tier_required",
        ))
        self.assertTrue(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo clippy --verbose -p oasis7 --lib -- -D warnings"],
            "cargo clippy -p oasis7 --lib -- -D warnings",
        ))
        self.assertFalse(MODULE._command_is_logged(
            ["+ env -u RUSTC_WRAPPER cargo clippy --verbose -p oasis7 --lib -- -D warnings --allow=all"],
            "cargo clippy -p oasis7 --lib -- -D warnings",
        ))
        self.assertTrue(MODULE._path_is_logged(
            ["+ bash ./scripts/check-script-executable-bits.sh"],
            "scripts/check-script-executable-bits.sh",
        ))


if __name__ == "__main__":
    unittest.main()
