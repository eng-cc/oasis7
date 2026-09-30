#!/usr/bin/env python3
"""Focused tests for the validation-only producer's closed execution plan."""
from __future__ import annotations

import importlib.util
import re
import sys
from types import SimpleNamespace
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
            "pr_number": "143",
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
    def test_successor_requester_must_be_live_admin(self):
        authority = SimpleNamespace(
            authorized_actor="approval-admin", pin_actor="pin-admin",
            request_actor="request-admin",
        )

        class API:
            def __init__(self, permissions):
                self.permissions = permissions
                self.checked = []

            def get_json(self, endpoint):
                login = endpoint.split("/collaborators/", 1)[1].split("/", 1)[0]
                self.checked.append(login)
                return {"user": {"login": login}, "permission": self.permissions[login]}

        with patch.object(producer.contract, "is_successor_authority", return_value=True):
            valid = API({
                "approval-admin": "admin", "pin-admin": "admin", "request-admin": "admin",
            })
            self.assertEqual(
                {"approval-admin", "pin-admin", "request-admin"},
                set(producer._live_admin_permissions(valid, authority)),
            )
            self.assertEqual({"approval-admin", "pin-admin", "request-admin"}, set(valid.checked))

            nonadmin = API({
                "approval-admin": "admin", "pin-admin": "admin", "request-admin": "write",
            })
            with self.assertRaises(producer.ProducerError):
                producer._live_admin_permissions(nonadmin, authority)
            self.assertEqual({"approval-admin", "pin-admin", "request-admin"}, set(nonadmin.checked))

    def test_task_lookup_uses_complete_rest_issue_pages_then_derived_issue_and_pr(self):
        task_uid = "task_" + "a" * 32
        issue_number = 87
        pr_number = 143
        body = (
            "<!-- oasis7-pm-task -->\n"
            f"task_uid: {task_uid}\n"
            f"- pr_number: `{pr_number}`\n"
            f"- pr_url: `https://github.com/eng-cc/oasis7/pull/{pr_number}`\n"
        )
        task_issue = {
            "id": 9001, "number": issue_number,
            "html_url": f"https://github.com/eng-cc/oasis7/issues/{issue_number}",
            "body": body,
        }
        live_issue = {
            "id": task_issue["id"], "number": issue_number,
            "html_url": task_issue["html_url"], "body": body,
        }

        class API:
            def __init__(self):
                self.pages_read = 0
                self.endpoints = []

            def task_issue_pages(self):
                self.pages_read += 1
                unrelated = {
                    "id": 8999, "number": 86,
                    "html_url": "https://github.com/eng-cc/oasis7/issues/86",
                    "body": "ordinary issue that does not contain a canonical Task marker",
                }
                return ([unrelated, task_issue],)

            def get_json(self, endpoint):
                self.endpoints.append(endpoint)
                return live_issue

        api = API()
        issue, resolved_issue_number, resolved_pr_number, pr_url = producer._resolve_live_task(api, task_uid)
        self.assertEqual(1, api.pages_read)
        self.assertEqual(live_issue, issue)
        self.assertEqual(issue_number, resolved_issue_number)
        self.assertEqual(pr_number, resolved_pr_number)
        self.assertEqual(f"https://github.com/eng-cc/oasis7/pull/{pr_number}", pr_url)
        self.assertEqual([f"repos/eng-cc/oasis7/issues/{issue_number}"], api.endpoints)

    def test_duplicate_task_uid_issue_matches_fail_closed_without_issue_or_pr_fallback(self):
        task_uid = "task_" + "a" * 32
        body = (
            "<!-- oasis7-pm-task -->\n"
            f"task_uid: {task_uid}\n"
            "- pr_number: `143`\n"
            "- pr_url: `https://github.com/eng-cc/oasis7/pull/143`\n"
        )

        class API:
            def task_issue_pages(self):
                return ([
                    {"id": 9001, "number": 87, "html_url": "https://github.com/eng-cc/oasis7/issues/87", "body": body},
                    {"id": 9002, "number": 88, "html_url": "https://github.com/eng-cc/oasis7/issues/88", "body": body},
                ],)

            def get_json(self, endpoint):
                raise AssertionError("ambiguous UID must fail before direct issue/PR reads")

        with self.assertRaisesRegex(producer.ProducerError, "exactly one"):
            producer._resolve_live_task(API(), task_uid)

    def test_task_issue_and_direct_rest_disagreement_fails_closed(self):
        task_uid = "task_" + "a" * 32
        body = (
            "<!-- oasis7-pm-task -->\n"
            f"task_uid: {task_uid}\n"
            "- pr_number: `143`\n"
            "- pr_url: `https://github.com/eng-cc/oasis7/pull/143`\n"
        )
        selected = {
            "id": 9001, "number": 87,
            "html_url": "https://github.com/eng-cc/oasis7/issues/87", "body": body,
        }

        class API:
            def __init__(self, live_issue):
                self.live_issue = live_issue

            def task_issue_pages(self):
                return ([selected],)

            def get_json(self, _endpoint):
                return self.live_issue

        changed_body = body.replace("pull/143", "pull/144").replace("`143`", "`144`")
        with self.assertRaises(producer.ProducerError):
            producer._resolve_live_task(API({
                "id": selected["id"], "number": 87,
                "html_url": selected["html_url"], "body": changed_body,
            }), task_uid)

    def test_missing_or_malformed_rest_issue_pages_fail_closed(self):
        task_uid = "task_" + "a" * 32

        class API:
            def __init__(self, pages):
                self.pages = pages

            def task_issue_pages(self):
                return self.pages

            def get_json(self, _endpoint):
                raise AssertionError("incomplete history must fail before fallback reads")

        for pages in ((), (None,), ([{"number": 87}],)):
            with self.subTest(pages=pages), self.assertRaises(producer.ProducerError):
                producer._resolve_live_task(API(pages), task_uid)

    def test_reciprocal_pr_requires_well_typed_same_repository_refs(self):
        pr = {
            "number": 143,
            "state": "open",
            "merged": False,
            "body": "Task: task_" + "a" * 32 + "\nRefs #87",
            "base": {"ref": "main", "sha": "1" * 40,
                     "repo": {"full_name": "eng-cc/oasis7"}},
            "head": {"sha": "2" * 40, "repo": {"full_name": "eng-cc/oasis7"}},
        }
        self.assertEqual(("2" * 40, "1" * 40),
                         producer._check_live_pr(pr, "task_" + "a" * 32, 87, 143))

        for changed in (
            {**pr, "head": {"sha": "2" * 40, "repo": None}},
            {**pr, "base": {**pr["base"], "sha": None}},
            {**pr, "head": {**pr["head"], "repo": {"full_name": "fork/project"}}},
        ):
            with self.subTest(changed=changed), self.assertRaises(producer.ProducerError):
                producer._check_live_pr(changed, "task_" + "a" * 32, 87, 143)

        for changed in (
            {**pr, "number": 144},
            {**pr, "body": pr["body"].replace("#87", "#88")},
            {**pr, "body": pr["body"].replace("task_" + "a" * 32, "task_" + "b" * 32)},
        ):
            with self.subTest(changed=changed), self.assertRaises(producer.ProducerError):
                producer._check_live_pr(changed, "task_" + "a" * 32, 87, 143)

    def test_current_run_must_be_exact_default_branch_workflow_w_and_title(self):
        oid = "a" * 40
        expected_title = "oasis7-ci|workflow_dispatch|validation-id"
        row = {
            "id": 700,
            "workflow_id": 800,
            "path": producer.WORKFLOW_FILE,
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
        self.assertEqual(producer.WORKFLOW_FILE, result["workflow_api_path"])
        self.assertEqual(producer.WORKFLOW_PATH, result["workflow_path"])

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

    def test_run_identity_uses_live_default_branch_and_exact_raw_path_forms(self):
        oid = "a" * 40
        title = "oasis7-ci|workflow_dispatch|validation-id"
        row = {
            "id": 701, "workflow_id": 801, "path": producer.WORKFLOW_FILE,
            "event": "workflow_dispatch", "head_branch": "release/ready",
            "display_title": title, "head_sha": oid, "run_attempt": 1,
            "repository": {"full_name": producer.REPOSITORY},
            "head_repository": {"full_name": producer.REPOSITORY},
        }
        environment = {
            "GITHUB_WORKFLOW_SHA": oid, "GITHUB_SHA": oid,
            "GITHUB_REF": "refs/heads/release/ready",
            "GITHUB_WORKFLOW_REF": (
                f"{producer.REPOSITORY}/{producer.WORKFLOW_FILE}@refs/heads/release/ready"
            ),
        }

        class API:
            def __init__(self, value):
                self.value = value

            def get_json(self, endpoint):
                return self.value

        result = producer._check_dispatch_run(
            API(row), 701, 801, "release/ready", title, environment,
        )
        self.assertEqual(producer.WORKFLOW_FILE, result["workflow_api_path"])
        self.assertEqual(f"{producer.WORKFLOW_FILE}@release/ready", result["workflow_path"])
        self.assertEqual("refs/heads/release/ready", result["event_ref"])
        for path in (
            f"{producer.WORKFLOW_FILE}@main",
            f"{producer.WORKFLOW_FILE}@refs/heads/release/ready",
            ".github/workflows/other.yml",
        ):
            with self.subTest(path=path), self.assertRaises(producer.ProducerError):
                producer._check_dispatch_run(
                    API({**row, "path": path}), 701, 801, "release/ready", title, environment,
                )

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
        self.assertIn("inputs.run_mode == 'v2_reuse_validation_successor_only'", job)
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
        self.assertNotIn("v2_reuse_validation_successor_only", required_gate.group(1))
        required_result = re.search(r"(?ms)^  required-result-v2:\n(.*?)(?=^  full-regression:)", workflow)
        self.assertIsNotNone(required_result)
        self.assertNotIn("v1_reuse_validation_only", required_result.group(1))
        self.assertNotIn("v2_reuse_validation_successor_only", required_result.group(1))

        jobs_section = workflow.split("jobs:\n", 1)[1]
        job_blocks = re.findall(
            r"(?ms)^  ([a-z0-9_-]+):\n(.*?)(?=^  [a-z0-9_-]+:|\Z)", jobs_section,
        )
        for name, body in job_blocks:
            if name != "v1-reuse-validation-only":
                with self.subTest(job=name):
                    self.assertNotIn("v2_reuse_validation_successor_only", body)


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
