#!/usr/bin/env python3
"""Single-path routing proof for scope authority, metadata and consumers."""
from pathlib import Path
import json
import re
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts/pm'))
from ci_required_execution import COMMANDS


def plan(path, config=None):
    argv = [sys.executable, "-I", str(ROOT / "scripts/plan-rust-required-scope.py"),
            "--event-name", "pull_request", "--changed-path", path]
    if config:
        argv += ["--config", str(config)]
    output = subprocess.check_output(argv, cwd=ROOT, text=True)
    return dict(line.split("=", 1) for line in output.splitlines())


class RoutingTest(unittest.TestCase):
    def test_authority_single_paths_select_full_required(self):
        for path in ("scripts/pm/check-cargo-package-scope",
                     "scripts/pm/cargo_package_change_classification.py",
                     "scripts/document_corpus.py", "scripts/pm/trusted_cargo_scope.py"):
            with self.subTest(path=path):
                actual = plan(path)
                self.assertEqual(actual["scope"], "full")
                self.assertEqual(actual["run_workflow_governance_contracts"], "true")
                self.assertEqual(actual["run_doc_checker_contracts"], "true")

    def test_consumers_single_paths_run_new_regressions(self):
        for path in ("scripts/prepare-task-pr.sh", "scripts/pm/required-gate-local-env.py",
                     "scripts/pm/cargo_package_profile_planner.py",
                     "scripts/pm/workflow-impact-projection.py",
                     "scripts/pm/task_primary_package.py", "scripts/pm/ci_required_inventory.py",
                     "scripts/pm/prepare-loop-ci-authority.py", "scripts/pm/prepare-loop-ci-authority.test.py",
                     "scripts/pm/trusted-cargo-scope.test.py", "scripts/pm/required-scope-routing.test.py"):
            with self.subTest(path=path):
                self.assertEqual(plan(path)["run_workflow_governance_contracts"], "true")
        runner = (ROOT / "scripts/ci-tests.sh").read_text()
        inventory = (ROOT / "scripts/ci-required-capability-test-inventory.tsv").read_text()
        for name in ("trusted-cargo-scope.test.py", "required-scope-routing.test.py",
                     "prepare-loop-ci-authority.test.py", "task-primary-package.test.py", "task-primary-package-consumers.test.py"):
            self.assertEqual(plan("scripts/pm/" + name)["run_workflow_governance_contracts"], "true")
            group = 'run_workflow_governance_baseline_contract_tests'
            body = re.search(r'(?m)^' + group + r'\(\) \{\n(.*?)^\}', runner, re.S)
            self.assertIsNotNone(body)
            self.assertEqual(body.group(1).strip(), 'run python3 "$driver_dir/pm/ci_required_execution.py" run-group --group ' + group + ' --root "$repo_root"')
            self.assertEqual(len([c for c in COMMANDS if c['group'] == group and c['argv'] == ['python3', './scripts/pm/' + name]]), 1)
            self.assertIn("scripts/pm/" + name, inventory)

    def test_metadata_v2_does_not_select_rust_baseline_or_business(self):
        actual = plan("doc/engineering/example.md")
        self.assertEqual(actual["execution_contract"], "required-domain-split/v2")
        self.assertEqual(actual["needs_rust_toolchain"], "true")
        self.assertEqual(actual["run_rust_baseline"], "false")
        self.assertEqual(actual["run_oasis7_required_tests"], "false")
        self.assertEqual(actual["selected_capabilities"], "required_gate_baseline")

    def test_v1_and_legacy_keep_resources(self):
        for fixture in ("versioned", "legacy"):
            actual = plan("doc/engineering/example.md", ROOT / f"scripts/fixtures/ci-required-scope.{fixture}-test.json")
            self.assertEqual(actual["needs_rust_toolchain"], "false")
            self.assertEqual(actual["run_rust_baseline"], "false")
            self.assertEqual(actual.get("execution_contract", ""), "required-domain-split/v1" if fixture == "versioned" else "")


if __name__ == "__main__":
    unittest.main()
