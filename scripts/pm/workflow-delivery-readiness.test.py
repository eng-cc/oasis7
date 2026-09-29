#!/usr/bin/env python3
"""Contract tests for the single authorized fast-recovery artifact edge."""
from __future__ import annotations

import importlib.util
import unittest
from unittest.mock import patch
from pathlib import Path


SCRIPT = Path(__file__).with_name("workflow-delivery-readiness.py")
SPEC = importlib.util.spec_from_file_location("workflow_delivery_readiness", SCRIPT)
assert SPEC and SPEC.loader
DELIVERY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DELIVERY)

DOWNSTREAM_UID = "task_7d9db27bcc2b4359b02fdf3bff5a609e"
UPSTREAM_UID = "task_5fe52e7477774b10af6655ad298eedd5"
MERGE_OID = "52d86940cad7d6ad0b67dc1e931e350882ef5eb5"
SOURCE_HEAD = "f8b65ffa9b9a61262936ce37e52b7f7e6ea978ab"
BASE_OID = "917f7172e856fc6a3a2523bccd5a5b9b13dcfa29"
STRICT_TITLE = (
    "oasis7-ci|workflow_dispatch|integration_revalidation|"
    f"{UPSTREAM_UID}|4139|{BASE_OID}|{SOURCE_HEAD}"
)


def evidence() -> dict:
    return {
        "edge": {
            "coordinator_issue": 4082,
            "comment_id": 5885353387,
            "downstream_issue": 4095,
            "downstream_task_uid": DOWNSTREAM_UID,
            "upstream_issue": 4137,
            "upstream_task_uid": UPSTREAM_UID,
            "upstream_pr": 4139,
            "merge_commit_oid": MERGE_OID,
            "required_source_path": "doc/engineering/workflow/source-of-truth.md",
        },
        "edge_authority": {
            "comment_id": 5885353387,
            "author": "eng-cc",
            "permission": "admin",
            "body_valid": True,
        },
        "downstream": {
            "issue_number": 4095,
            "task_uid": DOWNSTREAM_UID,
            "input_commit_oid": MERGE_OID,
            "input_files_match": True,
        },
        "upstream_issue": {
            "issue_number": 4137,
            "task_uid": UPSTREAM_UID,
            "status": "done",
            "workflow_phase": "task_done",
            "pr_number": 4139,
            "merge_hold_active": False,
        },
        "pr": {
            "repository": "eng-cc/oasis7",
            "number": 4139,
            "issue_number": 4137,
            "task_uid": UPSTREAM_UID,
            "state": "closed",
            "merged": True,
            "merge_commit_oid": MERGE_OID,
            "head_oid": SOURCE_HEAD,
            "base_ref": "main",
        },
        "review": {
            "comment_id": 5884285458,
            "task_uid": UPSTREAM_UID,
            "source_head_oid": SOURCE_HEAD,
            "author": "eng-cc",
            "passed": True,
            "roles": ["producer_system_designer", "repository_health_engineer", "qa_engineer"],
            "findings_disposition": "addressed",
        },
        "source_ci": {
            "check_name": "required-gate",
            "app_id": 15368,
            "head_oid": SOURCE_HEAD,
            "status": "completed",
            "conclusion": "success",
            "run_id": 36523663147,
            "run_attempt": 1,
        },
        "strict_ci": {
            "run_id": 36523710522,
            "run_attempt": 1,
            "event": "workflow_dispatch",
            "workflow_path": ".github/workflows/rust.yml",
            "head_branch": "main",
            "head_oid": BASE_OID,
            "status": "completed",
            "conclusion": "success",
            "display_title": STRICT_TITLE,
            "required_gate_job": {
                "name": "required-gate",
                "status": "completed",
                "conclusion": "success",
                "app_id": 15368,
                "run_attempt": 1,
            },
        },
        "local_input": {
            "repository": "eng-cc/oasis7",
            "head_contains_merge_commit": True,
            "source_path_present": True,
            "source_path_matches_merge_commit": True,
            "review_handoff_closure_matches_merge_commit": True,
        },
    }


class DeliveryReadinessTest(unittest.TestCase):
    def test_explicit_edge_is_delivered_while_managed_cleanup_is_deferred(self) -> None:
        result = DELIVERY.derive_delivery_readiness(DOWNSTREAM_UID, evidence())
        self.assertTrue(result["delivery_ready"], result)
        self.assertEqual(result["cleanup_state"], "cleanup_deferred", result)
        codes = {item["code"] for item in result["action_blockers"]}
        self.assertEqual(codes, {"CLEANUP_DEFERRED"}, result)
        blocker = result["action_blockers"][0]
        self.assertIn("consume_verified_artifact", blocker["allowed_actions"], result)
        self.assertIn("cleanup", blocker["blocks_actions"], result)
        self.assertNotIn("consume_verified_artifact", blocker["blocks_actions"], result)

    def test_unclassified_task_does_not_gain_artifact_delivery(self) -> None:
        result = DELIVERY.derive_delivery_readiness("task_11111111111111111111111111111111", evidence())
        self.assertIsNone(result["delivery_ready"], result)
        self.assertEqual(result["cleanup_state"], "not_applicable", result)
        self.assertEqual(result["action_blockers"], [], result)

    def test_wrong_merge_identity_blocks_only_the_named_consumption(self) -> None:
        proof = evidence()
        proof["pr"]["merge_commit_oid"] = "a" * 40
        result = DELIVERY.derive_delivery_readiness(DOWNSTREAM_UID, proof)
        self.assertFalse(result["delivery_ready"], result)
        self.assertEqual(result["cleanup_state"], "not_applicable", result)
        self.assertIn("TASK_BINDING_CONFLICT", {item["code"] for item in result["action_blockers"]})

    def test_missing_or_failed_current_ci_cannot_be_covered_by_cleanup_deferred(self) -> None:
        proof = evidence()
        proof["strict_ci"]["conclusion"] = "failure"
        result = DELIVERY.derive_delivery_readiness(DOWNSTREAM_UID, proof)
        self.assertFalse(result["delivery_ready"], result)
        self.assertEqual(result["cleanup_state"], "not_applicable", result)
        blocker = next(item for item in result["action_blockers"] if item["code"] == "CURRENT_CHECK_FAILED")
        self.assertIn("consume_artifact", blocker["blocks_actions"], result)

    def test_stale_review_and_hold_remain_delivery_blockers(self) -> None:
        for mutate, code in (
            (lambda proof: proof["review"].update(source_head_oid="b" * 40), "REVIEW_FINDING_BLOCKING"),
            (lambda proof: proof["upstream_issue"].update(merge_hold_active=True), "TASK_BINDING_CONFLICT"),
            (lambda proof: proof["downstream"].update(input_files_match=False), "SOURCE_NOT_PUBLISHED"),
        ):
            with self.subTest(code=code):
                proof = evidence()
                mutate(proof)
                result = DELIVERY.derive_delivery_readiness(DOWNSTREAM_UID, proof)
                self.assertFalse(result["delivery_ready"], result)
                self.assertIn(code, {item["code"] for item in result["action_blockers"]}, result)

    def test_action_projection_preserves_each_distinct_blocking_class(self) -> None:
        projected = DELIVERY.project_action_blockers([
            "stale identity: canonical task Issue URL is malformed",
            "required check conclusion=failure",
            "review finding blocks merge",
            "archive readback missing",
        ])
        self.assertEqual(
            [item["code"] for item in projected],
            ["TASK_BINDING_CONFLICT", "CURRENT_CHECK_FAILED",
             "REVIEW_FINDING_BLOCKING", "CAPABILITY_MISSING_ARCHIVE_READBACK"],
        )
        for item in projected:
            self.assertIn("blocks_actions", item)
            self.assertIn("allowed_actions", item)
            self.assertIn("next_action_kind", item)
            self.assertIsNone(item["next_command"])
            self.assertTrue(item["reason"])

    def test_current_pr_projection_keeps_local_remote_ci_and_failure_phase_separate(self) -> None:
        local_head = "1" * 40
        remote_head = "2" * 40
        task = {
            "task_uid": DOWNSTREAM_UID,
            "repository": "eng-cc/oasis7",
            "issue_number": 4095,
            "pr_number": "4200",
        }

        def run_read(conclusion: str) -> tuple[dict, list]:
            def gh_json(path: str) -> dict:
                if path == "repos/eng-cc/oasis7/pulls/4200":
                    return {
                        "number": 4200,
                        "body": f"Task: {DOWNSTREAM_UID}\nRefs #4095",
                        "head": {"sha": remote_head},
                    }
                if path == "repos/eng-cc/oasis7/actions/runs/777":
                    return {
                        "id": 777,
                        "head_sha": remote_head,
                        "run_attempt": 2,
                        "path": ".github/workflows/rust.yml",
                        "event": "pull_request",
                    }
                raise AssertionError(path)

            def pages(path: str, key: str | None = None) -> list:
                if "check-runs" in path:
                    return [{
                        "id": 999,
                        "name": "required-gate",
                        "app": {"id": 15368},
                        "pull_requests": [{"number": 4200}],
                        "details_url": "https://github.com/eng-cc/oasis7/actions/runs/777/job/4",
                        "head_sha": remote_head,
                        "status": "completed",
                        "conclusion": conclusion,
                    }]
                if "/jobs" in path:
                    row = {
                        "name": "required-gate",
                        "run_attempt": 2,
                        "check_run_url": "https://api.github.com/repos/eng-cc/oasis7/check-runs/999",
                        "status": "completed",
                        "conclusion": conclusion,
                    }
                    if conclusion == "failure":
                        row["steps"] = [{"name": "unit tests", "conclusion": "failure"}]
                    return [row]
                raise AssertionError(path)

            with patch.object(DELIVERY, "_git_value", return_value=local_head), \
                    patch.object(DELIVERY, "_gh_json", side_effect=gh_json), \
                    patch.object(DELIVERY, "_pages", side_effect=pages):
                return DELIVERY.read_current_pr_projection(Path("."), task)

        projection, blockers = run_read("success")
        self.assertEqual(projection["local_candidate_head_oid"], local_head)
        self.assertEqual(projection["remote_pr_head_oid"], remote_head)
        self.assertEqual(projection["required_ci"]["run_attempt"], 2)
        self.assertEqual(projection["required_ci"]["event"], "pull_request")
        self.assertEqual(projection["failure_phase"], None)
        self.assertIn("SOURCE_NOT_PUBLISHED", {item["code"] for item in blockers})
        publication = next(item for item in blockers if item["code"] == "SOURCE_NOT_PUBLISHED")
        self.assertNotIn("consume_artifact", publication["blocks_actions"], publication)

        projection, blockers = run_read("failure")
        self.assertEqual(projection["failure_phase"], {"job": "required-gate", "step": "unit tests"})
        self.assertIn("CURRENT_CHECK_FAILED", {item["code"] for item in blockers})

    def test_unclassified_workflow_projection_keeps_legacy_delivery_semantics(self) -> None:
        result = DELIVERY.workflow_projection(Path("."), {
            "task_uid": "task_11111111111111111111111111111111",
        }, [])
        self.assertIsNone(result["delivery_ready"])
        self.assertEqual(result["cleanup_state"], "not_applicable")
        self.assertEqual(result["action_blockers"], [])


if __name__ == "__main__":
    unittest.main()
