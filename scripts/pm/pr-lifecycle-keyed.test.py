#!/usr/bin/env python3
"""Focused keyed target-applicability tests for the production lifecycle gate."""

from __future__ import annotations

import importlib.util
import pathlib
import sys
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "pm"))
SPEC = importlib.util.spec_from_file_location(
    "pr_lifecycle_gate", ROOT / "scripts" / "pm" / "pr-lifecycle-gate.py",
)
GATE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GATE)


REPOSITORY = "eng-cc/oasis7"
UID = "task_11111111111111111111111111111111"
HEAD = "a" * 40
BASE = "b" * 40
TARGET = "c" * 40
SCOPE = "d" * 40
INPUT_COMMIT = "e" * 40
INPUT_TREE = "f" * 40
CONFIG_DIGEST = "sha256:" + "1" * 64
INVENTORY_DIGEST = "sha256:" + "2" * 64
REQUEST_KEY = "sha256:" + "3" * 64
POLICY_IDENTITY = {
    "schema": "oasis7-ci-effective-policy-identity/v1",
    "digest": "sha256:" + "4" * 64,
}


def gate_data():
    return {
        "number": 2198,
        "url": "https://github.com/eng-cc/oasis7/pull/2198",
        "repository": REPOSITORY,
        "state": "OPEN",
        "isDraft": False,
        "body": f"Task: {UID}\nRefs #2198",
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefName": "task/test",
        "headRefOid": HEAD,
        "baseRefName": "main",
        "baseRefOid": TARGET,
        "policy_discovery": {
            "status": "resolved",
            "required_status_checks": [{"context": "required-gate", "app_id": 42}],
            "active_rule_types": ["required_status_checks"],
        },
        "statusCheckRollup": [{
            "name": "required-gate", "app_id": 42,
            "status": "COMPLETED", "conclusion": "SUCCESS",
        }],
        "required_status_checks": [{"context": "required-gate", "app_id": 42}],
        "comments": [], "reviews": [], "threads": [],
        "merge_hold": {
            "kind": "normal_pr_ci_watch", "active": False,
            "requester": "workflow", "reason": "normal", "resume_authority": "workflow",
        },
    }


class KeyedLifecycleTests(unittest.TestCase):
    def test_keyed_stale_proof_without_fresh_q_c0_assessment_cannot_mint_ready(self):
        data = gate_data()
        task = {
            "repository": REPOSITORY, "issue_number": 2198,
            "bootstrap_epoch": 3,
            "loop_binding": {"bootstrap_epoch": 3},
        }
        admission = {"status": "passed", "task": task, "tool_root": "/trusted",
                     "policy_commit": "d" * 40}
        keyed_proof = {
            "request_key": "sha256:" + "e" * 64,
            "request_identity": {"bootstrap_epoch": 3, "source_head_oid": HEAD},
            "integration_base_oid": BASE,
            "workflow_run_id": 101,
            "run_attempt": 2,
            "check_run_id": 201,
            "assessed_target_oid": TARGET,
            "ci_validation_mode": "trusted_integration",
        }

        with (
            mock.patch.object(GATE, "local_loop_admission", return_value=admission),
            mock.patch.object(GATE, "live_integration_admission", return_value=keyed_proof),
            mock.patch.object(GATE, "read_pr_identity", return_value={
                key: data[key] for key in (
                    "number", "state", "isDraft", "body", "baseRefName",
                    "headRefName", "baseRefOid", "headRefOid",
                )
            }),
        ):
            result = GATE.production_decision(
                data, False, pathlib.Path("/canonical"), UID, None,
            )

        self.assertFalse(result["ready_for_merge"], result)
        self.assertTrue(any(
            "keyed" in blocker.lower() and "target" in blocker.lower()
            for blocker in result["blockers"]
        ), result)

    def reusable_proof(self):
        decision = {
            "identity": {
                "source_head_oid": HEAD,
                "source_scope_oid": SCOPE,
                "assessed_target_oid": TARGET,
                "prior_assessed_target_oid": None,
                "input_scope_commit_oid": INPUT_COMMIT,
                "input_scope_tree_oid": INPUT_TREE,
                "target_inventory_digest": INVENTORY_DIGEST,
            },
            "effective_policy_identity": POLICY_IDENTITY,
            "test_evidence": "reusable",
            "blockers": [],
            "reused_units": ["unit:rust:crate-a"],
            "required_test_units": [],
        }
        assessment = {
            "schema": GATE.KEYED_Q_APPLICABILITY_SCHEMA,
            "request_key": REQUEST_KEY,
            "workflow_run_id": 101,
            "run_attempt": 2,
            "check_app_id": 42,
            "check_run_id": 201,
            "integration_base_oid": BASE,
            "source_head_oid": HEAD,
            "source_scope_oid": SCOPE,
            "assessed_target_oid": TARGET,
            "input_scope_commit_oid": INPUT_COMMIT,
            "input_scope_tree_oid": INPUT_TREE,
            "planner_authority_oid": TARGET,
            "planner_config_sha256": CONFIG_DIGEST,
            "effective_policy_identity": POLICY_IDENTITY,
            "inventory_digest": INVENTORY_DIGEST,
            "required_test_units": ["unit:rust:crate-a"],
            "closure_status": "complete",
            "test_evidence": "reusable",
            "decision": decision,
        }
        assessment["decision_digest"] = GATE._canonical_digest(decision)
        plan = {
            "integration_base_oid": BASE,
            "source_head_oid": HEAD,
            "source_scope_oid": SCOPE,
            "workflow_run_id": 101,
            "run_attempt": 2,
            "check_app_id": 42,
            "check_run_id": 201,
        }
        proof = {
            "request_key": REQUEST_KEY,
            "workflow_run_id": 101,
            "run_attempt": 2,
            "check_app_id": 42,
            "check_run_id": 201,
            "integration_base_oid": BASE,
            "assessed_target_oid": TARGET,
            "source_scope_oid": SCOPE,
            "required_plan_v2_payload": plan,
            "keyed_q_applicability": assessment,
        }
        return proof

    def test_reusable_c0_decision_binds_latest_attempt_and_fresh_q_m_t(self):
        proof = self.reusable_proof()
        epoch = GATE._validate_keyed_q_applicability(proof, gate_data())

        self.assertEqual(epoch["test_evidence"], "reusable")
        self.assertEqual(epoch["run_attempt"], 2)
        self.assertEqual(epoch["assessed_target_oid"], TARGET)
        self.assertEqual(epoch["input_scope_commit_oid"], INPUT_COMMIT)
        self.assertEqual(epoch["input_scope_tree_oid"], INPUT_TREE)
        self.assertEqual(epoch["planner_authority_oid"], TARGET)

    def test_production_accepts_stale_b_only_after_reusable_q_assessment(self):
        data = gate_data()
        task = {
            "repository": REPOSITORY, "issue_number": 2198,
            "bootstrap_epoch": 3,
            "loop_binding": {"bootstrap_epoch": 3},
        }
        admission = {"status": "passed", "task": task, "tool_root": "/trusted",
                     "policy_commit": "d" * 40}
        proof = self.reusable_proof()
        proof.update({
            "ci_validation_mode": "trusted_integration",
            "head_oid": HEAD,
            "base_ref": "main",
            "check_name": "required-gate",
        })
        live_pr = {
            key: data[key] for key in (
                "number", "state", "isDraft", "body", "baseRefName",
                "headRefName", "baseRefOid", "headRefOid",
            )
        }

        with (
            mock.patch.object(GATE, "local_loop_admission", return_value=admission),
            mock.patch.object(GATE, "live_integration_admission", return_value=proof),
            mock.patch.object(GATE, "read_pr_identity", return_value=live_pr),
        ):
            result = GATE.production_decision(
                data, False, pathlib.Path("/canonical"), UID, None,
            )

        self.assertTrue(result["ready_for_merge"], result)
        integration = result["readiness_receipt"]["integration_ci"]
        epoch = integration["keyed_target_applicability_epoch"]
        self.assertEqual(epoch["integration_base_oid"], BASE)
        self.assertEqual(epoch["assessed_target_oid"], TARGET)
        self.assertEqual(epoch["run_attempt"], 2)

    def test_reusable_c0_rejects_attempt_q_w_and_closure_drift(self):
        cases = (
            ("attempt", lambda proof: proof.update(run_attempt=3), "latest run/check identity"),
            ("Q", lambda proof: proof["keyed_q_applicability"].update(assessed_target_oid=BASE), "target Q identity drift"),
            ("W", lambda proof: proof["keyed_q_applicability"].update(planner_authority_oid=BASE), "W, inventory"),
            ("closure", lambda proof: proof["keyed_q_applicability"].update(closure_status="unknown"), "closure is unknown"),
            ("unit_type", lambda proof: proof["keyed_q_applicability"].update(required_test_units=[{}]), "closure is unknown"),
        )
        for label, mutate, message in cases:
            with self.subTest(label=label):
                proof = self.reusable_proof()
                mutate(proof)
                with self.assertRaisesRegex(ValueError, message):
                    GATE._validate_keyed_q_applicability(proof, gate_data())

    def test_disabled_policy_cannot_reuse_old_but_accepts_exact_current_b(self):
        proof = self.reusable_proof()
        assessment = proof["keyed_q_applicability"]
        decision = {
            "identity": {
                "source_head_oid": HEAD,
                "source_scope_oid": SCOPE,
                "assessed_target_oid": TARGET,
            },
            "effective_policy_identity": POLICY_IDENTITY,
            "test_evidence": "disabled",
            "blockers": [],
            "reused_units": [],
            "required_test_units": [],
        }
        assessment["test_evidence"] = "disabled"
        assessment["decision"] = decision
        assessment["decision_digest"] = GATE._canonical_digest(decision)
        with self.assertRaisesRegex(ValueError, "reuse is disabled and source B is stale"):
            GATE._validate_keyed_q_applicability(proof, gate_data())

        assessment["integration_base_oid"] = TARGET
        proof["integration_base_oid"] = TARGET
        proof["required_plan_v2_payload"]["integration_base_oid"] = TARGET
        epoch = GATE._validate_keyed_q_applicability(proof, gate_data())
        self.assertEqual(epoch["test_evidence"], "disabled")
        self.assertEqual(epoch["integration_base_oid"], TARGET)


if __name__ == "__main__":
    unittest.main()
