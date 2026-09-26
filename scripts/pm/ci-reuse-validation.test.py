#!/usr/bin/env python3
"""Focused tests for the validation-only producer's closed execution plan."""
from __future__ import annotations

import importlib.util
import re
import sys
from unittest.mock import patch
import unittest
from pathlib import Path


HERE = Path(__file__).parent
SPEC = importlib.util.spec_from_file_location(
    "ci_reuse_validation_producer", HERE / "ci-reuse-validation.py",
)
producer = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = producer
SPEC.loader.exec_module(producer)


class DispatchInputTests(unittest.TestCase):
    def test_dispatch_assertions_match_authority_and_only_inert_extra_inputs(self):
        expected = {
            "run_mode": "v1_reuse_validation_only",
            "task_uid": "task_" + "a" * 32,
            "pr_number": "4060",
            "integration_base": "1" * 40,
            "expected_head": "2" * 40,
            "source_scope_oid": "3" * 40,
            "projection_digest": "sha256:" + "4" * 64,
            "request_key": "5" * 64,
        }
        actual = {
            **expected,
            "newapi_bridge_build_profile": "release",
            "escalation_reason": "",
            "evidence_url": "",
            "impact_projection_b64": "",
            "validation_request_b64": "",
        }
        self.assertEqual(expected, producer.validate_dispatch_inputs(actual, expected))

        for field, value in (
            ("request_key", "6" * 64),
            ("impact_projection_b64", "caller-supplied-projection"),
            ("validation_request_b64", "caller-supplied-request"),
            ("unknown_override", "anything"),
        ):
            changed = dict(actual)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(producer.ProducerError):
                producer.validate_dispatch_inputs(changed, expected)

    def test_missing_assertion_or_changed_inert_default_fails_closed(self):
        expected = {"run_mode": "v1_reuse_validation_only", "request_key": "a" * 64}
        with self.assertRaises(producer.ProducerError):
            producer.validate_dispatch_inputs({"run_mode": expected["run_mode"]}, expected)
        with self.assertRaises(producer.ProducerError):
            producer.validate_dispatch_inputs(
                {**expected, "newapi_bridge_build_profile": "dev"}, expected,
            )


class LiveIdentityTests(unittest.TestCase):
    def test_reciprocal_pr_requires_well_typed_same_repository_refs(self):
        pr = {
            "number": 4060,
            "state": "open",
            "merged": False,
            "body": "Task: task_" + "a" * 32 + "\nRefs #4059",
            "base": {"ref": "main", "sha": "1" * 40,
                     "repo": {"full_name": "eng-cc/oasis7"}},
            "head": {"sha": "2" * 40, "repo": {"full_name": "eng-cc/oasis7"}},
        }
        self.assertEqual(("2" * 40, "1" * 40), producer._check_live_pr(pr, "task_" + "a" * 32))

        for changed in (
            {**pr, "head": {"sha": "2" * 40, "repo": None}},
            {**pr, "base": {**pr["base"], "sha": None}},
            {**pr, "head": {**pr["head"], "repo": {"full_name": "fork/project"}}},
        ):
            with self.subTest(changed=changed), self.assertRaises(producer.ProducerError):
                producer._check_live_pr(changed, "task_" + "a" * 32)

    def test_current_run_must_be_exact_default_branch_workflow_w_and_title(self):
        oid = "a" * 40
        expected_title = "oasis7-ci|workflow_dispatch|validation-id"
        row = {
            "id": 700,
            "workflow_id": 800,
            "path": producer.WORKFLOW_PATH,
            "event": "workflow_dispatch",
            "head_branch": "main",
            "display_title": expected_title,
            "head_sha": oid,
            "run_attempt": 2,
            "created_at": "2026-01-01T00:00:00Z",
            "repository": {"full_name": producer.REPOSITORY},
            "head_repository": {"full_name": producer.REPOSITORY},
        }
        environment = {
            "GITHUB_WORKFLOW_SHA": oid,
            "GITHUB_SHA": oid,
            "GITHUB_REF": "refs/heads/main",
            "GITHUB_WORKFLOW_REF": producer.WORKFLOW_REF,
        }

        class API:
            def __init__(self, value):
                self.value = value

            def get_json(self, endpoint):
                self.endpoint = endpoint
                return self.value

        api = API(row)
        result = producer._check_dispatch_run(api, 700, 800, "main", expected_title, environment)
        self.assertEqual("repos/eng-cc/oasis7/actions/runs/700", api.endpoint)
        self.assertEqual(2, result["run_attempt"])
        self.assertEqual(oid, result["workflow_sha"])

        for field, value in (
            ("event", "push"),
            ("path", "another.yml@main"),
            ("head_branch", "attacker"),
            ("display_title", expected_title + "-other"),
            ("repository", None),
            ("head_repository", {"full_name": "attacker/fork"}),
            ("workflow_id", True),
            ("run_attempt", True),
        ):
            changed = dict(row)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(producer.ProducerError):
                producer._check_dispatch_run(API(changed), 700, 800, "main", expected_title, environment)

        bad_environment = dict(environment)
        bad_environment["GITHUB_WORKFLOW_SHA"] = "b" * 40
        with self.assertRaises(producer.ProducerError):
            producer._check_dispatch_run(API(row), 700, 800, "main", expected_title, bad_environment)


class CompleteCollectionTests(unittest.TestCase):
    def test_collection_requires_stable_total_unique_ids_and_complete_pagination(self):
        rows = producer._collection_rows((
            {"total_count": 2, "jobs": [{"id": 1}], "has_next": True},
            {"total_count": 2, "jobs": [{"id": 2}], "has_next": False},
        ), "jobs", "attempt jobs")
        self.assertEqual((1, 2), tuple(row["id"] for row in rows))

        invalid = (
            (({"total_count": 2, "jobs": [{"id": 1}], "has_next": False},),
             "reported total"),
            (({"total_count": 1, "jobs": [{"id": 1}], "has_next": True},
              {"total_count": 1, "jobs": [{"id": 1}], "has_next": False}), "duplicate ID"),
            (({"total_count": 1, "jobs": [{"id": 1}], "has_next": False},
              {"total_count": 1, "jobs": [], "has_next": False}), "pagination marker"),
        )
        for pages, _label in invalid:
            with self.subTest(pages=pages), self.assertRaises(producer.ProducerError):
                producer._collection_rows(pages, "jobs", "attempt jobs")


class WorkflowIsolationTests(unittest.TestCase):
    def test_validation_job_is_a_distinct_nonrequired_path(self):
        workflow = (HERE.parents[1] / ".github" / "workflows" / "rust.yml").read_text(
            encoding="utf-8",
        )
        job_match = re.search(
            r"(?ms)^  v1-reuse-validation-only:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)", workflow,
        )
        self.assertIsNotNone(job_match)
        job = job_match.group(1)
        self.assertIn("name: v1-reuse-validation-only", job)
        self.assertIn("inputs.run_mode == 'v1_reuse_validation_only'", job)
        self.assertIn("github.ref == 'refs/heads/main'", job)
        self.assertIn("GH_TOKEN: ${{ github.token }}", job)
        self.assertIn("python3 -I scripts/pm/ci-reuse-validation.py", job)
        self.assertIn("overwrite: false", job)
        self.assertNotIn("needs:", job)
        self.assertNotIn("required_plan_v2", job)
        self.assertNotIn("required_result_v2", job)

        required_gate = re.search(r"(?ms)^  required-gate:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)", workflow)
        self.assertIsNotNone(required_gate)
        self.assertNotIn("v1_reuse_validation_only", required_gate.group(1))
        required_result = re.search(r"(?ms)^  required-result-v2:\n(.*?)(?=^  full-regression:)", workflow)
        self.assertIsNotNone(required_result)
        self.assertNotIn("v1_reuse_validation_only", required_result.group(1))


class ExecutionPlanTests(unittest.TestCase):
    def test_only_authorized_units_select_runner_flags_with_required_pairs(self):
        capability_runners = {
            "required_gate_baseline": ("run_required_gate_checks",),
            "net": ("run_oasis7_net_tests",),
            "viewer_js_required": ("run_oasis7_viewer_contract_tests",),
        }
        selector_env = {
            "net": "OASIS7_CI_RUN_OASIS7_NET_TESTS",
            "viewer_js_required": "OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS",
        }
        environment = producer.build_runner_environment(
            ("net", "required_gate_baseline"), capability_runners, selector_env,
        )
        self.assertEqual("true", environment["OASIS7_CI_RUN_OASIS7_NET_TESTS"])
        self.assertEqual("true", environment["OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS"])
        self.assertEqual("false", environment["OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS"])
        self.assertEqual("false", environment["OASIS7_CI_RUN_VIEWER_WASM_CHECK"])
        self.assertEqual("true", environment["OASIS7_CI_RUN_RUST_BASELINE"])
        self.assertEqual("true", environment["OASIS7_CI_NEEDS_PYTHON"])
        self.assertEqual("true", environment["OASIS7_CI_NEEDS_MARKDOWN"])
        self.assertEqual("false", environment["OASIS7_CI_RUN_PROVIDER_LIVE_GATE"])

    def test_node_dependencies_are_installed_from_exact_tested_M_only_for_node_units(self):
        target = Path("/tmp/validation-target-m")
        with patch.object(producer.subprocess, "run") as run:
            producer._install_target_node_dependencies(target, ("net",))
            run.assert_not_called()

            producer._install_target_node_dependencies(target, ("viewer_js_required",))
            run.assert_called_once_with(
                ["npm", "ci", "--prefix", "/tmp/validation-target-m/crates/oasis7_viewer"],
                check=True,
            )

    def test_unknown_or_nonexecutable_authorized_unit_fails_closed(self):
        with self.assertRaises(producer.ProducerError):
            producer.build_runner_environment(
                ("made_up_unit",),
                {"required_gate_baseline": ("run_required_gate_checks",)},
                {},
            )

    def test_unit_result_digest_commits_to_obligations_and_exact_successful_run_output(self):
        obligations = {"net": ("net:000:cargo test", "net:001:cargo clippy")}
        first = producer.build_unit_result_digests(
            ("net",), obligations, "sha256:" + "6" * 64,
            run_id=700, run_attempt=1, tested_merge_oid="7" * 40,
            tested_tree_oid="8" * 40,
        )
        second = producer.build_unit_result_digests(
            ("net",), obligations, "sha256:" + "9" * 64,
            run_id=700, run_attempt=1, tested_merge_oid="7" * 40,
            tested_tree_oid="8" * 40,
        )
        changed_obligation = producer.build_unit_result_digests(
            ("net",), {"net": ("net:000:different",)}, "sha256:" + "6" * 64,
            run_id=700, run_attempt=1, tested_merge_oid="7" * 40,
            tested_tree_oid="8" * 40,
        )
        self.assertEqual({"net"}, set(first))
        self.assertRegex(first["net"], r"^sha256:[0-9a-f]{64}$")
        self.assertNotEqual(first, second)
        self.assertNotEqual(first, changed_obligation)


if __name__ == "__main__":
    unittest.main()
