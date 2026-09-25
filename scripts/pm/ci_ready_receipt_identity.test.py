#!/usr/bin/env python3
"""Focused tests for v2 artifact identity in CI receipt digests."""

import copy
import hashlib
import unittest

import ci_ready_receipt_identity as identity


def receipt():
    unit_id = "unit-a"
    workflow_ref = "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main"
    effective_policy_identity = {
        "schema": "oasis7-ci-effective-policy-identity/v1",
        "digest": "sha256:" + "9" * 64,
    }
    authority = {
        "schema": "oasis7-planner-inventory-authority/v1",
        "repository": "eng-cc/oasis7",
        "workflow_ref": workflow_ref,
        "planner_authority_oid": "f" * 40,
        "planner_config_sha256": "sha256:" + "4" * 64,
    }
    issuer = {
        "schema": "oasis7-trusted-planner-inventory/v1",
        "authority": authority,
        "producer": {
            "run_id": 12345,
            "run_attempt": 2,
            "check_app_id": 42,
            "check_run_id": 902,
        },
        "target_oid": "d" * 40,
        "target_tree_oid": "c" * 40,
        "unit_ids": [unit_id],
        "inventory_digest": "sha256:" + "f" * 64,
    }
    request_identity = {
        "repository": "eng-cc/oasis7",
        "task_uid": "task_12345678901234567890123456789012",
        "pr_number": 7,
        "bootstrap_epoch": 1,
        "source_head_oid": "a" * 40,
        "source_projection_digest": "sha256:" + "1" * 64,
        "executor_contract_digest": "sha256:" + "8" * 64,
    }
    request_key = "sha256:" + "2" * 64
    plan_payload = {
        "schema": "oasis7-required-plan-v2",
        "required_capabilities": ["input-scope-reuse/v1"],
        "request_key": request_key,
        "request_identity": request_identity,
        "repository": "eng-cc/oasis7",
        "task_uid": request_identity["task_uid"],
        "pr_number": 7,
        "bootstrap_epoch": 1,
        "source_head_oid": "a" * 40,
        "source_scope_oid": "e" * 40,
        "source_projection_digest": request_identity["source_projection_digest"],
        "integration_base_oid": "b" * 40,
        "tested_commit_oid": "d" * 40,
        "tested_tree_oid": "c" * 40,
        "workflow_ref": workflow_ref,
        "workflow_sha": "f" * 40,
        "workflow_run_id": 12345,
        "run_attempt": 2,
        "check_name": "required-gate",
        "check_app_id": 42,
        "check_run_id": 902,
        "job_id": 8101,
        "job_name": "required-gate",
        "executor_contract_digest": request_identity["executor_contract_digest"],
        "effective_policy_identity": effective_policy_identity,
        "planner_inventory_authority": authority,
        "planner_inventory_issuer": issuer,
        "planner_inventory_digest": issuer["inventory_digest"],
        "unit_ids": [unit_id],
    }
    trusted_inventory = {
        **issuer,
        "producer": {**issuer["producer"], "artifact_id": 7001},
    }
    return {
        "receipt_type": "oasis7_ci_ready_receipt",
        "issuer": "github_live_query",
        "repository": "eng-cc/oasis7",
        "task_uid": "task_12345678901234567890123456789012",
        "task_issue_number": 1,
        "pr_number": 7,
        "base_oid": "b" * 40,
        "head_oid": "a" * 40,
        "check_name": "required-gate",
        "check_app_id": 42,
        "check_run_id": 902,
        "planner_digest": "d" * 64,
        "planner_config_sha256": "sha256:" + "c" * 64,
        "run_rust_baseline": True,
        "conclusion": "success",
        "run_id": 12345,
        "run_attempt": 2,
        "request_id": 12345,
        "request_created_at": "2026-09-25T10:00:00Z",
        "live_validation": "ci-ready-receipt-live",
        "trusted_integration_artifact": True,
        "bootstrap_epoch": 1,
        "integration_run_id": 12345,
        "tested_tree_oid": "c" * 40,
        "tested_commit_oid": "d" * 40,
        "request_key": request_key,
        "request_identity": request_identity,
        "workflow_ref": workflow_ref,
        "workflow_sha": "f" * 40,
        "cargo_package_profile": {},
        "scope_base_oid": "e" * 40,
        "integration_base_oid": "b" * 40,
        "source_scope_oid": "e" * 40,
        "required_plan_v2_artifact_id": 7001,
        "required_plan_v2_artifact_name": "oasis7-required-plan-v2-12345-a2",
        "required_plan_v2_payload": plan_payload,
        "required_result_v2_artifacts": [
            {
                "artifact_id": 7002,
                "name": "oasis7-required-result-v2-12345-a2-" + hashlib.sha256(unit_id.encode()).hexdigest(),
                "payload": {
                    "schema": "oasis7-required-result-v2",
                    "workflow_run_id": 12345,
                    "run_attempt": 2,
                    "plan_artifact_id": 7001,
                    "unit_id": unit_id,
                    "status": "passed",
                    "disposition": "executed",
                    "exit_code": 0,
                },
            },
        ],
        "effective_policy_identity": effective_policy_identity,
        "trusted_policy_context": {
            "workflow_ref": workflow_ref,
            "workflow_sha": "f" * 40,
            "effective_policy_identity": effective_policy_identity,
        },
        "trusted_planner_inventory": trusted_inventory,
        "execution_jobs": [
            {
                "workflow_run_id": 12345,
                "run_attempt": 2,
                "job_id": 8101,
                "job_name": "required-gate",
                "check_name": "required-gate",
                "check_app_id": 42,
                "check_run_id": 902,
                "head_sha": "a" * 40,
                "status": "completed",
                "conclusion": "success",
            },
        ],
        "trusted_source_attempt": {
            "schema": "oasis7-ci-trusted-source-attempt/v1",
            "request_key": request_key,
            "workflow_run_id": 12345,
            "run_attempt": 2,
            "check_app_id": 42,
            "check_run_id": 902,
            "job_id": 8101,
            "job_name": "required-gate",
            "plan_artifact_id": 7001,
            "plan_artifact_name": "oasis7-required-plan-v2-12345-a2",
            "result_artifacts": [{
                "unit_id": unit_id,
                "artifact_id": 7002,
                "name": "oasis7-required-result-v2-12345-a2-"
                    + hashlib.sha256(unit_id.encode()).hexdigest(),
            }],
        },
    }


class RequiredArtifactIdentityTests(unittest.TestCase):
    def test_v2_plan_result_locators_and_payloads_are_in_digest_authority(self):
        original = receipt()
        original_identity = identity.review_evidence_identity(original)
        self.assertEqual(
            original["required_plan_v2_payload"],
            original_identity["required_plan_v2_payload"],
        )
        self.assertEqual(
            original["required_result_v2_artifacts"],
            original_identity["required_result_v2_artifacts"],
        )
        original_digest = identity.review_evidence_digest(original)

        for change in (
            "bootstrap_epoch", "request_id", "request_created_at", "trusted_marker",
            "plan_artifact_id", "plan_payload", "result_artifact_id", "result_payload",
            "trusted_inventory", "execution_job", "trusted_source_attempt",
        ):
            changed = copy.deepcopy(original)
            if change == "bootstrap_epoch":
                changed["bootstrap_epoch"] += 1
            elif change == "request_id":
                changed["request_id"] += 1
            elif change == "request_created_at":
                changed["request_created_at"] = "2026-09-25T10:00:01Z"
            elif change == "trusted_marker":
                changed["trusted_integration_artifact"] = False
            elif change == "plan_artifact_id":
                changed["required_plan_v2_artifact_id"] += 1
            elif change == "plan_payload":
                changed["required_plan_v2_payload"]["unit_ids"].append("unit-b")
            elif change == "result_artifact_id":
                changed["required_result_v2_artifacts"][0]["artifact_id"] += 1
            elif change == "result_payload":
                changed["required_result_v2_artifacts"][0]["payload"]["status"] = "unknown"
            elif change == "trusted_inventory":
                changed["trusted_planner_inventory"]["producer"]["artifact_id"] += 1
            elif change == "trusted_source_attempt":
                changed["trusted_source_attempt"]["result_artifacts"][0]["artifact_id"] += 1
            else:
                changed["execution_jobs"][0]["job_id"] += 1
            with self.subTest(change=change):
                try:
                    changed_digest = identity.review_evidence_digest(changed)
                except ValueError:
                    # Invalid changes are rejected before a digest is issued.
                    continue
                self.assertNotEqual(original_digest, changed_digest)

    def test_partial_v2_authority_is_rejected(self):
        value = receipt()
        del value["required_result_v2_artifacts"]
        with self.assertRaisesRegex(ValueError, "v2 required evidence"):
            identity.review_evidence_identity(value)


if __name__ == "__main__":
    unittest.main()
