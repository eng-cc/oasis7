#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import hashlib
import json
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
PATH = ROOT / "scripts/pm/first_activation.py"
SPEC = importlib.util.spec_from_file_location("first_activation_under_test", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
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


def _comment(comment_id: int, body: str, second: int) -> dict:
    return {
        "id": comment_id,
        "issue_url": ISSUE_URL,
        "body": body,
        "user": {"login": "human"},
        "created_at": f"2026-10-03T00:00:{second:02d}Z",
    }


def review_evidence(value: dict, packet_digests: dict[str, str] | None = None) -> tuple[list[dict], dict[str, dict]]:
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
        "base_oid": BASE,
        "head_oid": HEAD,
        "authorization": value["authorization"],
        "workflow": {key: value["workflow"][key] for key in
                     ("id", "path", "ref", "sha", "file_sha256")},
        "workflow_change_paths": value["workflow"]["change_paths"],
        "implementation_slices": [{
            "role": "repository_health_engineer",
            "slice_id": "implementation-checker",
            "dispatch_comments": [{"comment_id": 2, "body_sha256": dispatch_digest}],
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
        _comment(2, dispatch_body, 2),
        _comment(3, plan_body, 3),
    ]
    envelopes: dict[str, dict] = {}
    for return_id, review_id, envelope_role, plan_role, slice_id in (
        (4, 5, "repository_health", "repository_health_engineer", "first-rh"),
        (6, 7, "qa", "qa_engineer", "first-qa"),
    ):
        returned = {
            "schema": MODULE.FIRST_REVIEW_RETURN_SCHEMA,
            "task_uid": UID,
            "issue_number": 4269,
            "role": plan_role,
            "slice_id": slice_id,
            "base_oid": BASE,
            "head_oid": HEAD,
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
            "base_oid": BASE,
            "head_oid": HEAD,
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


def overlay() -> dict:
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
        "head_oid": HEAD,
        "authorization": {"comment_id": 1, "body_sha256": SHA, "scope": "effective_package_dependency_floor"},
        "reviews": {
            "repository_health": {"comment_id": 2, "body_sha256": SHA},
            "qa": {"comment_id": 3, "body_sha256": SHA},
        },
        "workflow": {
            "id": 123456,
            "path": ".github/workflows/rust.yml",
            "ref": "refs/heads/codex/test",
            "sha": HEAD,
            "file_sha256": SHA,
            "change_paths": [
                {"path": ".github/workflows/rust.yml", "base_sha256": SHA, "head_sha256": SHA}
            ],
        },
    }


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
        plan_comment, payload, plan_sha = MODULE._plan_comment(comments, ISSUE_URL)
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
                    "issue_url": ISSUE_URL,
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


if __name__ == "__main__":
    unittest.main()
