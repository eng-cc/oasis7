#!/usr/bin/env python3
"""Fake-adapter coverage for repeated unrelated target advances.

This composes the production keyed-Q adapter and C0 applicability decision
with deterministic local fixtures. It does not exercise GitHub-hosted runners,
Codex orchestration, or live branch protection.
"""

from __future__ import annotations

import copy
import hashlib
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


HERE = Path(__file__).resolve().parent
LOOP_SPEC = importlib.util.spec_from_file_location(
    "pr_lifecycle_loop_adapter_fixtures", HERE / "pr-lifecycle-loop.test.py",
)
assert LOOP_SPEC is not None and LOOP_SPEC.loader is not None
LOOP_FIXTURES = importlib.util.module_from_spec(LOOP_SPEC)
LOOP_SPEC.loader.exec_module(LOOP_FIXTURES)
GATE = LOOP_FIXTURES.gate
FIXTURES = LOOP_FIXTURES._APPLICABILITY_FIXTURES


class FakeProductionAdapter:
    """Call the real Q adapter while counting effects in a fake outer loop."""

    def __init__(self):
        self.source_proof, self.initial_inventory, self.applicability, self.initial_live = (
            LOOP_FIXTURES.KeyedQApplicabilityTests().evaluator_inputs()
        )
        self.initial_source_identity = self._source_identity(self.source_proof)
        self.counters = {
            "fresh_q_observations": 0,
            "c0_applicability_decisions": 0,
            "new_candidate_heads": 0,
            "source_commit_dispatches": 0,
            "role_dispatches": 0,
            "heavy_run_dispatches": 0,
            "task_phase_regressions": 0,
        }

    @staticmethod
    def _source_identity(proof):
        plan = proof["required_plan_v2_payload"]
        return (
            plan["source_head_oid"],
            plan["integration_base_oid"],
            plan["source_scope_oid"],
            plan["request_key"],
            proof["workflow_run_id"],
            proof["run_attempt"],
            proof["check_app_id"],
            proof["check_run_id"],
        )

    def target_inventory(self, target_oid, *, input_digest=None):
        tree_oid = hashlib.sha1(("tree:" + target_oid).encode("ascii")).hexdigest()
        snapshot_args = {
            "assessed_target_oid": target_oid,
            "input_scope_commit_oid": target_oid,
            "input_scope_tree_oid": tree_oid,
        }
        if input_digest is not None:
            snapshot_args["input_digest"] = input_digest
        snapshot = FIXTURES.target_snapshot(**snapshot_args)
        observation = snapshot["input_scope"]["target_observation"]
        inventory = copy.deepcopy(self.initial_inventory)
        inventory.update({
            "assessed_target_oid": target_oid,
            "input_scope_commit_oid": target_oid,
            "input_scope_tree_oid": tree_oid,
            "planner_authority_oid": target_oid,
            "planner_config_sha256": observation["authority"]["planner_config_sha256"],
            "effective_policy_identity": observation["effective_policy_identity"],
            "inventory_digest": observation["inventory_digest"],
            "required_test_units": snapshot["required_test_units"],
            "unit_specs": snapshot["unit_specs"],
            "product_corpus": snapshot["product_corpus"],
            "closure_status": "complete",
            "target_observation": observation,
            "input_scope": snapshot["input_scope"],
            "effective_policy": FIXTURES.enabled_policy(),
        })
        return inventory

    def evaluate(self, target_oid, *, input_digest=None, proof=None):
        self.counters["fresh_q_observations"] += 1
        inventory = self.target_inventory(target_oid, input_digest=input_digest)
        source = copy.deepcopy(self.source_proof if proof is None else proof)
        source["assessed_target_oid"] = target_oid
        live = {**self.initial_live, "baseRefOid": target_oid}

        # The production decision seam should remain a local assessment. These
        # mocks count a source commit attempt and fail on any process dispatch.
        def forbid_process(command, *args, **kwargs):
            argv = list(command) if isinstance(command, (list, tuple)) else [str(command)]
            if argv and argv[0] == "git" and "commit" in argv[1:]:
                self.counters["source_commit_dispatches"] += 1
            raise AssertionError("keyed-Q applicability must not dispatch a process")

        with patch.object(GATE.subprocess, "run", side_effect=forbid_process), patch.object(
            GATE.subprocess, "check_output", side_effect=forbid_process,
        ):
            assessment = GATE.evaluate_keyed_q_applicability(
                source, inventory, self.applicability, live,
            )
        self.counters["c0_applicability_decisions"] += 1
        if assessment["source_head_oid"] != self.initial_source_identity[0]:
            self.counters["new_candidate_heads"] += 1
        if assessment["decision"]["required_review_roles"]:
            self.counters["role_dispatches"] += len(
                assessment["decision"]["required_review_roles"],
            )
        if assessment["decision"]["required_test_units"]:
            self.counters["heavy_run_dispatches"] += len(
                assessment["decision"]["required_test_units"],
            )
        return assessment

    def c0_decision_for_target(self, target):
        """Inspect exact C0 unit selection for the single related-input case."""
        self.counters["fresh_q_observations"] += 1
        self.counters["c0_applicability_decisions"] += 1
        decision = FIXTURES.ApplicabilityDecisionTests().evaluate(target=target).to_dict()
        if decision["required_review_roles"]:
            self.counters["role_dispatches"] += len(decision["required_review_roles"])
        if decision["required_test_units"]:
            # This is a simulated request count, not a real hosted CI run.
            self.counters["heavy_run_dispatches"] += len(decision["required_test_units"])
        return decision


class ParallelReuseIntegrationTests(unittest.TestCase):
    def test_twenty_unrelated_q_advances_reuse_one_fixed_candidate(self):
        adapter = FakeProductionAdapter()
        observed_heads = set()
        source_before = copy.deepcopy(adapter.source_proof)

        for advance in range(20):
            target_oid = f"{1000 + advance:040x}"
            with self.subTest(advance=advance + 1):
                assessment = adapter.evaluate(target_oid)
                self.assertEqual("reusable", assessment["test_evidence"])
                self.assertEqual([], assessment["decision"]["required_test_units"])
                self.assertEqual([], assessment["decision"]["required_review_roles"])
                self.assertEqual(target_oid, assessment["assessed_target_oid"])
                observed_heads.add(assessment["source_head_oid"])

        self.assertEqual({adapter.initial_source_identity[0]}, observed_heads)
        self.assertEqual(source_before, adapter.source_proof)
        self.assertEqual(adapter.initial_source_identity, adapter._source_identity(adapter.source_proof))
        self.assertEqual({
            "fresh_q_observations": 20,
            "c0_applicability_decisions": 20,
            "new_candidate_heads": 0,
            "source_commit_dispatches": 0,
            "role_dispatches": 0,
            "heavy_run_dispatches": 0,
            "task_phase_regressions": 0,
        }, adapter.counters)

    def test_related_input_selects_one_unit_for_revalidation(self):
        adapter = FakeProductionAdapter()
        target_oid = "f" * 40
        changed_digest = "sha256:" + "e" * 64
        tree_oid = hashlib.sha1(("tree:" + target_oid).encode("ascii")).hexdigest()
        target = FIXTURES.target_snapshot(
            assessed_target_oid=target_oid,
            input_scope_commit_oid=target_oid,
            input_scope_tree_oid=tree_oid,
            input_digest=changed_digest,
        )

        with self.assertRaisesRegex(ValueError, "requires revalidation"):
            adapter.evaluate(target_oid, input_digest=changed_digest)

        decision = adapter.c0_decision_for_target(target)

        self.assertEqual("revalidate", decision["test_evidence"])
        self.assertEqual(["unit-a"], decision["required_test_units"])
        self.assertEqual(["product-corpus:membership"], decision["reused_units"])
        self.assertEqual(2, adapter.counters["fresh_q_observations"])
        self.assertEqual(1, adapter.counters["c0_applicability_decisions"])
        self.assertEqual(1, adapter.counters["heavy_run_dispatches"])

    def test_latest_attempt_mismatch_cannot_reuse_source_artifacts(self):
        adapter = FakeProductionAdapter()
        target_oid = "f" * 40
        stale = copy.deepcopy(adapter.source_proof)
        stale["required_result_v2_artifacts"][0]["payload"]["run_attempt"] = 2

        with self.assertRaisesRegex(ValueError, "exact R/A/check attempt"):
            adapter.evaluate(target_oid, proof=stale)

        self.assertEqual(1, adapter.counters["fresh_q_observations"])
        self.assertEqual(0, adapter.counters["c0_applicability_decisions"])
        self.assertEqual(0, adapter.counters["heavy_run_dispatches"])


if __name__ == "__main__":
    unittest.main()
