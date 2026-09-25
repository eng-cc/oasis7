#!/usr/bin/env python3
"""Contract tests for attempt-scoped required-gate v2 artifacts."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("ci_required_artifact_v2", HERE / "ci_required_artifact_v2.py")
ARTIFACT = importlib.util.module_from_spec(SPEC)
assert SPEC is not None and SPEC.loader is not None
SPEC.loader.exec_module(ARTIFACT)


def valid_plan():
    run_id, attempt, app_id, check_id = 90, 2, 42, 91
    b, h, s, m, t, w = (letter * 40 for letter in "abcdef")
    digest = "sha256:" + "1" * 64
    request_identity = {
        "repository": "eng-cc/oasis7", "task_uid": "task_" + "2" * 32,
        "pr_number": 4007, "bootstrap_epoch": 1, "source_head_oid": h,
        "publication_id": "publication-1", "source_projection_digest": digest,
        "unit_ids": ["unit-x"], "input_fingerprints": {"unit-x": digest},
        "executor_contract_digest": digest, "effective_policy_digest": digest,
        "purpose": "integration_revalidation", "applicability_mode": "input_scoped",
        "snapshot_target_oid": None,
    }
    authority = {
        "schema": "oasis7-planner-inventory-authority/v1", "repository": "eng-cc/oasis7",
        "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
        "planner_authority_oid": w, "planner_config_sha256": digest,
    }
    producer = {"run_id": run_id, "run_attempt": attempt,
                "check_app_id": app_id, "check_run_id": check_id}
    issuer = {
        "schema": "oasis7-trusted-planner-inventory/v1", "authority": authority,
        "producer": producer, "target_oid": m, "target_tree_oid": t,
        "unit_ids": ["unit-x"], "inventory_digest": digest,
    }
    planner_output = {
        "execution_contract": "required-domain-split/v1",
        "source_scope_base": s, "integration_base": b, "source_head": h,
        "impact_projection_digest": digest, "planner_config_sha256": digest,
        "required_test_units": "unit-x",
    }
    invocation = {
        "schema": "oasis7-required-scope-invocation/v1",
        "planner_authority_oid": w, "planner_config_sha256": digest,
        "event_name": "workflow_dispatch", "run_mode": "integration_revalidation",
        "base_ref": b, "head_ref": h, "task_uid": request_identity["task_uid"],
        "scope_base_oid": s, "impact_projection_sha256": digest,
        "changed_paths": ["crates/oasis7/src/lib.rs"],
        "planner_output_sha256": ARTIFACT.canonical_digest(planner_output),
        "producer": producer,
    }
    invocation["digest"] = ARTIFACT.planner_invocation_digest(invocation)
    unit_spec = {
        "unit_id": "unit-x", "obligation_set": ["unit-x:check"],
        "input_paths": ["crates/oasis7/src/lib.rs"], "dependency_edges": [],
    }
    scope = {
        "schema": "oasis7-ci-input-scope/v2", "target_oid": m, "target_tree_oid": t,
        "planner_inventory_issuer": issuer,
        "closure_status": {"schema": "oasis7-ci-input-scope-closure/v1", "status": "complete"},
        "required_test_units": ["unit-x"], "input_fingerprints": {"unit-x": digest},
        "dependency_edges": [], "fallback_complete": False,
        "fallback_contract": {"schema": "oasis7-ci-input-scope-fallback/v1", "status": "not-used"},
        "product_corpus": {"status": "complete", "errors": [], "unit_ids": []},
    }
    return {
        "schema": ARTIFACT.PLAN_SCHEMA,
        "required_capabilities": ["input-scope-reuse/v1"],
        "request_key": ARTIFACT.request_key_for_identity(request_identity),
        "request_identity": request_identity,
        "repository": "eng-cc/oasis7", "task_uid": request_identity["task_uid"],
        "pr_number": 4007, "bootstrap_epoch": 1, "source_head_oid": h,
        "source_scope_oid": s, "source_projection_digest": digest,
        "integration_base_oid": b, "tested_commit_oid": m, "tested_tree_oid": t,
        "workflow_ref": authority["workflow_ref"], "workflow_sha": w,
        "workflow_run_id": run_id, "run_attempt": attempt,
        "check_name": "required-gate", "check_app_id": app_id,
        "check_run_id": check_id, "job_id": 92, "job_name": "required-gate",
        "executor_contract_digest": digest,
        "effective_policy_identity": {"schema": "oasis7-ci-effective-policy-identity/v1", "digest": digest},
        "planner_inventory_authority": authority,
        "planner_inventory_issuer": issuer, "planner_inventory_digest": digest,
        "planner_config_sha256": digest, "planner_invocation": invocation,
        "unit_ids": ["unit-x"], "required_test_units": ["unit-x"],
        "input_fingerprints": {"unit-x": digest}, "unit_specs": [unit_spec],
        "execution_job_requirements": {"unit-x": []},
        "product_corpus": {"status": "complete", "errors": [], "unit_ids": []},
        "input_scope": scope, "planner_output": planner_output,
        "closure_status": "complete",
    }


def gate_job(plan):
    return {
        "workflow_run_id": plan["workflow_run_id"], "run_attempt": plan["run_attempt"],
        "job_id": plan["job_id"], "job_name": "required-gate",
        "check_name": "required-gate", "check_app_id": plan["check_app_id"],
        "check_run_id": plan["check_run_id"], "head_sha": plan["workflow_sha"],
        "status": "completed", "conclusion": "success", "labels": ["ubuntu-24.04"],
    }


class RequiredArtifactV2Tests(unittest.TestCase):
    def setUp(self):
        self.plan = valid_plan()

    def test_attempt_scoped_names_are_canonical_and_unit_keyed(self):
        self.assertEqual(
            "oasis7-required-plan-v2-90-a2",
            ARTIFACT.plan_artifact_name(90, 2),
        )
        self.assertEqual(
            "oasis7-required-result-v2-90-a2-" + ARTIFACT.unit_id_sha256("unit-x"),
            ARTIFACT.result_artifact_name(90, 2, "unit-x"),
        )
        with self.assertRaisesRegex(ValueError, "positive integer"):
            ARTIFACT.plan_artifact_name(True, 2)

    def test_plan_binds_epoch_and_planner_projection(self):
        self.assertEqual(self.plan, ARTIFACT.validate_plan_payload(self.plan))
        for field, bad in (("bootstrap_epoch", True), ("bootstrap_epoch", "1"),
                           ("source_scope_oid", "0" * 40),
                           ("source_projection_digest", "sha256:" + "9" * 64)):
            mutated = dict(self.plan)
            mutated[field] = bad
            with self.subTest(field=field, bad=bad), self.assertRaises(ValueError):
                ARTIFACT.validate_plan_payload(mutated)

    def test_snapshot_exact_request_is_bound_to_the_commit_actually_tested(self):
        exact_plan = dict(self.plan)
        exact_identity = {
            **self.plan["request_identity"],
            "applicability_mode": "snapshot_exact",
            "snapshot_target_oid": self.plan["tested_commit_oid"],
        }
        exact_plan["request_identity"] = exact_identity
        exact_plan["request_key"] = ARTIFACT.request_key_for_identity(exact_identity)
        self.assertEqual(exact_plan, ARTIFACT.validate_plan_payload(exact_plan))

        moved_target_plan = dict(exact_plan)
        moved_identity = {**exact_identity, "snapshot_target_oid": "9" * 40}
        moved_target_plan["request_identity"] = moved_identity
        moved_target_plan["request_key"] = ARTIFACT.request_key_for_identity(moved_identity)
        with self.assertRaisesRegex(ValueError, "snapshot-exact request target"):
            ARTIFACT.validate_plan_payload(moved_target_plan)

    def test_plan_rejects_unknown_closure_and_subset_inventory(self):
        mutated = dict(self.plan)
        mutated["closure_status"] = "unknown"
        with self.assertRaisesRegex(ValueError, "closure"):
            ARTIFACT.validate_plan_payload(mutated, require_complete=True)
        mutated = dict(self.plan)
        mutated["unit_ids"] = []
        with self.assertRaisesRegex(ValueError, "unit"):
            ARTIFACT.validate_plan_payload(mutated)

    def test_selected_units_bind_required_child_jobs(self):
        self.assertEqual({
            "operational_contracts": [
                "public-testnet-fleet-health-contract (macos-14)",
                "public-testnet-fleet-health-contract (ubuntu-24.04)",
                "public-testnet-fleet-health-contract (windows-2022)",
                "windows-package-rollout-behavior",
            ],
            "packaging_contracts": ["testnet-packages-macos-arm64-contract"],
            "unit-x": [],
        }, ARTIFACT.execution_job_requirements(
            ["operational_contracts", "packaging_contracts", "unit-x"],
        ))

    def test_result_binds_exact_plan_artifact_and_successful_attempt_job(self):
        result = ARTIFACT.build_result_payload(self.plan, 123, "unit-x", [gate_job(self.plan)])
        self.assertEqual(123, result["plan_artifact_id"])
        self.assertEqual("passed", result["status"])
        self.assertEqual(result, ARTIFACT.validate_result_payload(
            result, plan=self.plan, plan_artifact_id=123, expected_unit_id="unit-x",
        ))

    def test_result_rejects_parent_plan_only_failed_job_and_wrong_attempt(self):
        for mutation in (
            {"status": "failed"}, {"plan_artifact_id": 124},
            {"run_attempt": 1},
        ):
            result = ARTIFACT.build_result_payload(self.plan, 123, "unit-x", [gate_job(self.plan)])
            result.update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                ARTIFACT.validate_result_payload(
                    result, plan=self.plan, plan_artifact_id=123, expected_unit_id="unit-x",
                )
        failed = gate_job(self.plan)
        failed["conclusion"] = "failure"
        with self.assertRaisesRegex(ValueError, "successful"):
            ARTIFACT.build_result_payload(self.plan, 123, "unit-x", [failed])


if __name__ == "__main__":
    unittest.main()
