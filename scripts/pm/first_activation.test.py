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
        auth_body = "User authorizes oasis7_wasm_executor wasmtime 48.0.3 to 48.0.4 effective_package_dependency_floor"
        value["authorization"]["body_sha256"] = "sha256:" + hashlib.sha256(auth_body.encode()).hexdigest()
        review_records = {}
        comments = [{
            "id": 1, "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/4269",
            "body": auth_body, "user": {"login": "human"},
        }]
        for comment_id, role in ((2, "repository_health"), (3, "qa")):
            typed = {
                "schema": MODULE.REVIEW_SCHEMA,
                "task_uid": UID,
                "issue_number": 4269,
                "role": role,
                "slice_id": f"slice-{role}",
                "base_oid": BASE,
                "head_oid": HEAD,
                "workflow_change_paths": value["workflow"]["change_paths"],
                "review_plan_sha256": SHA,
                "admitted_packet_sha256": SHA,
                "return_sha256": SHA,
                "return_status": "completed",
                "findings": [],
                "unresolved_findings": [],
            }
            body = MODULE.REVIEW_MARKER + "\n" + MODULE._canonical(typed).decode() + "\n"
            value["reviews"][role]["body_sha256"] = "sha256:" + hashlib.sha256(body.encode()).hexdigest()
            review_records[role] = typed
            comments.append({
                "id": comment_id, "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/4269",
                "body": body, "user": {"login": "human"},
            })
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
