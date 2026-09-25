#!/usr/bin/env python3
"""Focused authority and planning contract for the hosted preview."""

import importlib.util
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DRIVER_PATH = ROOT / "scripts/pm/ci-required-scope-preview.py"
SPEC = importlib.util.spec_from_file_location("required_preview", DRIVER_PATH)
PREVIEW = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PREVIEW)


class RequiredScopePreviewTests(unittest.TestCase):
    def context(self):
        return {
            "PREVIEW_REPOSITORY": "eng-cc/oasis7",
            "PREVIEW_EVENT_NAME": "workflow_dispatch",
            "PREVIEW_REF": "refs/heads/main",
            "PREVIEW_WORKFLOW_SHA": "a" * 40,
            "PREVIEW_RUN_ATTEMPT": "1",
            "PREVIEW_PR_NUMBER": "3992",
            "PREVIEW_TASK_UID": "task_" + "b" * 32,
            "PREVIEW_BASE_SHA": "a" * 40,
            "PREVIEW_HEAD_SHA": "d" * 40,
            "PREVIEW_TESTED_TREE_SHA": "e" * 40,
            "PREVIEW_SCENARIO": "doc_checker_contracts",
        }

    def test_dispatch_identity_is_canonical_and_complete(self):
        context = PREVIEW.validate_context(self.context())
        self.assertEqual(context["PREVIEW_REPOSITORY"], "eng-cc/oasis7")
        for key in ("PREVIEW_WORKFLOW_SHA", "PREVIEW_BASE_SHA", "PREVIEW_HEAD_SHA", "PREVIEW_TESTED_TREE_SHA"):
            self.assertRegex(context[key], PREVIEW.SHA_RE)

    def test_rejects_noncanonical_ref_repository_uid_and_scenario(self):
        for key, value in (
            ("PREVIEW_REPOSITORY", "attacker/oasis7"),
            ("PREVIEW_REF", "refs/heads/feature"),
            ("PREVIEW_EVENT_NAME", "pull_request"),
            ("PREVIEW_RUN_ATTEMPT", "2"),
            ("PREVIEW_TASK_UID", "task_invalid"),
            ("PREVIEW_SCENARIO", "full"),
            ("PREVIEW_TESTED_TREE_SHA", "e" * 39),
            ("PREVIEW_BASE_SHA", "c" * 40),
        ):
            candidate = self.context()
            candidate[key] = value
            with self.subTest(key=key), self.assertRaises(SystemExit):
                PREVIEW.validate_context(candidate)

    def test_public_pr_identity_requires_exact_same_repo_open_candidate(self):
        context = self.context()
        payload = {
            "state": "open",
            "body": "Task: " + context["PREVIEW_TASK_UID"],
            "base": {"ref": "main", "sha": context["PREVIEW_BASE_SHA"],
                     "repo": {"full_name": "eng-cc/oasis7"}},
            "head": {"sha": context["PREVIEW_HEAD_SHA"],
                     "repo": {"full_name": "eng-cc/oasis7"}},
        }
        self.assertIs(PREVIEW.validate_pr_payload(payload, context), payload)
        for change in (
            {"state": "closed"},
            {"body": "Task: task_" + "f" * 32},
            {"base": {**payload["base"], "ref": "release"}},
            {"base": {**payload["base"], "repo": {"full_name": "other/repo"}}},
            {"head": {**payload["head"], "repo": {"full_name": "contributor/oasis7"}}},
            {"head": {**payload["head"], "sha": "f" * 40}},
        ):
            candidate = {**payload, **change}
            with self.subTest(change=change), self.assertRaises(SystemExit):
                PREVIEW.validate_pr_payload(candidate, context)

    def make_merge_fixture(self, temporary, extra_path=False, wrong_config=False, reverse_parents=False):
        root = Path(temporary)

        def git(*args):
            return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()

        def run_git(*args):
            subprocess.run(["git", "-C", str(root), *args], check=True, stdout=subprocess.DEVNULL)

        run_git("init", "-q")
        run_git("config", "user.name", "Preview test")
        run_git("config", "user.email", "preview-test@example.invalid")
        fixture_path = root / PREVIEW.FIXTURE
        fixture_path.parent.mkdir(parents=True)
        trusted_fixture = (ROOT / PREVIEW.FIXTURE).read_bytes()
        fixture_path.write_bytes(trusted_fixture)
        effective = root / PREVIEW.EFFECTIVE_CONFIG
        effective.parent.mkdir(parents=True, exist_ok=True)
        effective.write_bytes(b"legacy config\n")
        run_git("add", PREVIEW.FIXTURE, PREVIEW.EFFECTIVE_CONFIG)
        run_git("commit", "-qm", "base")
        base = git("rev-parse", "HEAD")
        effective.write_bytes(b"invalid candidate config\n" if wrong_config else trusted_fixture)
        if extra_path:
            (root / "extra.txt").write_text("unapproved tree change\n")
            run_git("add", "extra.txt")
        run_git("add", PREVIEW.EFFECTIVE_CONFIG)
        run_git("commit", "-qm", "candidate")
        head = git("rev-parse", "HEAD")
        head_tree = git("rev-parse", "HEAD^{tree}")
        if reverse_parents:
            merge = subprocess.check_output(
                ["git", "-C", str(root), "commit-tree", head_tree, "-p", head, "-p", base, "-m", "merge"],
                text=True,
            ).strip()
        else:
            merge = subprocess.check_output(
                ["git", "-C", str(root), "commit-tree", head_tree, "-p", base, "-p", head, "-m", "merge"],
                text=True,
            ).strip()
        tree = git("rev-parse", f"{merge}^{{tree}}")
        return root, base, head, merge, tree

    def test_merge_verifier_rejects_wrong_base_head_tree_parents_and_extra_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, base, head, merge, tree = self.make_merge_fixture(temporary)
            self.assertEqual(PREVIEW.validate_merge_tree(root, base, head, merge, tree), [PREVIEW.EFFECTIVE_CONFIG])
            PREVIEW.validate_candidate_config(root, merge)
            with self.assertRaises(SystemExit):
                PREVIEW.validate_merge_tree(root, "0" * 40, head, merge, tree)
            with self.assertRaises(SystemExit):
                PREVIEW.validate_merge_tree(root, base, "0" * 40, merge, tree)
            with self.assertRaises(SystemExit):
                PREVIEW.validate_merge_tree(root, base, head, merge, "0" * 40)

        with tempfile.TemporaryDirectory() as temporary:
            root, base, head, merge, tree = self.make_merge_fixture(temporary, reverse_parents=True)
            with self.assertRaises(SystemExit):
                PREVIEW.validate_merge_tree(root, base, head, merge, tree)

        with tempfile.TemporaryDirectory() as temporary:
            root, base, head, merge, tree = self.make_merge_fixture(temporary, extra_path=True)
            with self.assertRaises(SystemExit):
                PREVIEW.validate_merge_tree(root, base, head, merge, tree)

        with tempfile.TemporaryDirectory() as temporary:
            root, base, head, merge, tree = self.make_merge_fixture(temporary, wrong_config=True)
            self.assertEqual(PREVIEW.validate_merge_tree(root, base, head, merge, tree), [PREVIEW.EFFECTIVE_CONFIG])
            with self.assertRaises(SystemExit):
                PREVIEW.validate_candidate_config(root, merge)

    def test_trusted_planner_selects_each_versioned_scenario_exactly(self):
        for scenario, expected in PREVIEW.SCENARIOS.items():
            with self.subTest(scenario=scenario):
                fields = PREVIEW.plan_scenario(ROOT, scenario)
                self.assertEqual(fields["selected_capabilities"], expected["capability"] or "required_gate_baseline")
                self.assertEqual(fields["run_required_gate_baseline"], "true")
                self.assertEqual(fields["run_rust_baseline"], "false")
                self.assertEqual(fields["needs_python"], "true")
                self.assertEqual(fields["needs_markdown"], "true")
                self.assertEqual(fields["needs_rust_toolchain"], "false")
                if expected["capability"] is None:
                    self.assertEqual(fields["scope"], "minimal")
                    for capability in ("doc_checker_contracts", "workflow_governance", "packaging_contracts", "operational_contracts"):
                        selector = "run_" + capability
                        if capability == "workflow_governance":
                            selector += "_contracts"
                        self.assertEqual(fields[selector], "false")
                    continue
                for capability in PREVIEW.SCENARIOS:
                    capability_name = PREVIEW.SCENARIOS[capability]["capability"]
                    if capability_name is None:
                        continue
                    selector = "run_" + capability_name
                    if capability == "workflow_governance":
                        selector += "_contracts"
                    self.assertEqual(fields[selector], "true" if capability == scenario else "false")

    def test_dispatch_environment_never_enables_manual_only_selectors(self):
        fields = PREVIEW.plan_scenario(ROOT, "operational_contracts")
        context = self.context()
        sentinel_names = (
            "GH_TOKEN", "GITHUB_TOKEN", "ACTIONS_RUNTIME_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL",
            "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "GITHUB_OUTPUT", "GITHUB_ENV", "GITHUB_PATH",
            "GITHUB_STATE", "GITHUB_STEP_SUMMARY",
        )
        old_values = {name: os.environ.get(name) for name in (*sentinel_names, "PREVIEW_PYTHON")}
        try:
            for name in sentinel_names:
                os.environ[name] = "must-not-reach-child"
            os.environ["PREVIEW_PYTHON"] = "/tmp/preview-venv/bin/python"
            env = PREVIEW.dispatcher_environment(fields, context, "f" * 40)
        finally:
            for name, value in old_values.items():
                if value is None:
                    os.environ.pop(name, None)
                else:
                    os.environ[name] = value
        self.assertEqual(env["OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE"], "false")
        self.assertEqual(env["OASIS7_CI_RUN_PROVIDER_LIVE_GATE"], "false")
        self.assertEqual(env["OASIS7_CI_RUN_OPERATIONAL_CONTRACTS"], "true")
        self.assertEqual(env["OASIS7_CI_EXECUTION_CONTRACT"], "required-domain-split/v1")
        for name in sentinel_names:
            self.assertNotIn(name, env)
        preview_bin = str(Path("/tmp/preview-venv/bin/python").resolve().parent)
        self.assertTrue(env["PATH"].startswith(preview_bin + os.pathsep))

    def test_validated_child_outputs_bind_tree_and_selected_capabilities(self):
        context = self.context()
        context["PREVIEW_SCENARIO"] = "packaging_contracts"
        fields = PREVIEW.plan_scenario(ROOT, context["PREVIEW_SCENARIO"])
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "outputs.txt"
            PREVIEW.write_workflow_outputs(output, context, "f" * 40, fields)
            values = dict(line.split("=", 1) for line in output.read_text().splitlines())
        self.assertEqual(values["base_sha"], context["PREVIEW_BASE_SHA"])
        self.assertEqual(values["head_sha"], context["PREVIEW_HEAD_SHA"])
        self.assertEqual(values["tested_tree_sha"], context["PREVIEW_TESTED_TREE_SHA"])
        self.assertEqual(values["run_packaging_contracts"], "true")
        self.assertEqual(values["run_operational_contracts"], "false")

    def test_workflow_is_manual_unprivileged_and_non_required(self):
        workflow = (ROOT / ".github/workflows/required-gate-versioned-preview.yml").read_text()
        self.assertIn("workflow_dispatch:", workflow)
        self.assertNotRegex(workflow, r"(?m)^\s+(pull_request|pull_request_target|push|schedule):")
        self.assertIn("permissions: {}", workflow)
        self.assertNotIn("GITHUB_TOKEN", workflow)
        self.assertNotIn("secrets.", workflow)
        self.assertNotIn("upload-artifact", workflow)
        self.assertIn("refs/heads/main", workflow)
        self.assertIn("github.run_attempt == 1", workflow)
        self.assertIn("tested_tree_sha", workflow)
        self.assertNotIn("required-gate:", workflow)

    def test_selected_hosted_child_jobs_capture_exact_run_attempt(self):
        workflow = (ROOT / ".github/workflows/required-gate-versioned-preview.yml").read_text()
        for job in (
            "preview-testnet-packages-macos-arm64-contract",
            "preview-windows-package-rollout-behavior",
            "preview-fleet-health-ubuntu",
            "preview-fleet-health-windows",
            "preview-fleet-health-macos",
        ):
            self.assertIn(job + ":", workflow)
        self.assertIn("needs.preview.outputs.run_packaging_contracts == 'true'", workflow)
        self.assertIn("needs.preview.outputs.run_operational_contracts == 'true'", workflow)
        self.assertIn("VALIDATED_MERGE_COMMIT", workflow)
        self.assertIn("VALIDATED_TREE_SHA", workflow)
        self.assertEqual(workflow.count('test "$tree" = "$VALIDATED_TREE_SHA"'), 5)
        self.assertIn("bash ./.preview-candidate/scripts/testnet-packages-macos-arm64-contract.test.sh", workflow)
        self.assertIn("bash ./.preview-candidate/scripts/p2p-public-testnet-package-rollout.test.sh", workflow)
        self.assertEqual(workflow.count("python ./.preview-candidate/scripts/p2p-public-testnet-fleet-health.test.py"), 3)
        self.assertIn("RUN_ID: ${{ github.run_id }}", workflow)
        self.assertIn("RUN_ATTEMPT: ${{ github.run_attempt }}", workflow)
        self.assertIn("FLEET_MACOS_RESULT: ${{ needs.preview-fleet-health-macos.result }}", workflow)
        self.assertIn("preview-outcome-summary:", workflow)
        self.assertIn("expect_child_result \"$PACKAGING_SELECTED\" \"$PACKAGE_RESULT\"", workflow)
        self.assertIn("expect_child_result \"$OPERATIONAL_SELECTED\" \"$FLEET_MACOS_RESULT\"", workflow)
        self.assertIn("Actual B-to-T diff: scripts/ci-required-scope.v2.json only", workflow)
        self.assertIn('[[ "$PREVIEW_RESULT" == success ]]', workflow)
        self.assertIn('[[ "$RUN_ATTEMPT" == 1 ]]', workflow)
        driver = DRIVER_PATH.read_text()
        self.assertIn("synthetic_changed_path=", driver)
        self.assertIn("changed_range_document_validation=not_exercised_by_config_only_tree", driver)

    def test_driver_binds_pr_tree_to_base_head_and_trusted_policy(self):
        source = DRIVER_PATH.read_text()
        self.assertIn("read_public_pr(context)", source)
        self.assertIn("parents != [merge, base, head]", source)
        self.assertIn("PREVIEW_TESTED_TREE_SHA", source)
        self.assertIn("changed != [EFFECTIVE_CONFIG]", source)
        self.assertIn("candidate_bytes != fixture_bytes", source)
        self.assertIn("readiness_receipt=not-created", source)
        self.assertIn("required_gate_authority=unchanged", source)


if __name__ == "__main__":
    unittest.main(verbosity=2)
