#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("required-gate-local-env.py")
SPEC = importlib.util.spec_from_file_location("required_gate_local_env", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
ROOT = Path(__file__).resolve().parents[2]


def planner_for_path(path: str) -> tuple[dict[str, str], str]:
    output = subprocess.check_output(
        [
            sys.executable,
            str(ROOT / "scripts/plan-rust-required-scope.py"),
            "--event-name",
            "pull_request",
            "--config",
            str(ROOT / "scripts/fixtures/ci-required-scope.versioned-test.json"),
            "--changed-path",
            path,
        ],
        cwd=ROOT,
        text=True,
    )
    values = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
    return values, output


class RequiredGateLocalEnvironmentTest(unittest.TestCase):
    def test_minimal_document_scope_still_renders_baseline_resources(self) -> None:
        values, planner_output = planner_for_path("doc/engineering/project.md")
        self.assertEqual("minimal", values["scope"])
        self.assertEqual("required_gate_baseline", values["selected_capabilities"])
        self.assertEqual("required-domain-split/v1", values["execution_contract"])

        environment = MODULE.render_versioned_environment(planner_output)
        expected = {
            "OASIS7_CI_EXECUTION_CONTRACT": "required-domain-split/v1",
            "OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS": "false",
            "OASIS7_CI_RUN_PACKAGING_CONTRACTS": "false",
            "OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS": "false",
            "OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS": "false",
            "OASIS7_CI_NEEDS_PYTHON": "true",
            "OASIS7_CI_NEEDS_MARKDOWN": "true",
            "OASIS7_CI_NEEDS_RUST_TOOLCHAIN": "false",
            "OASIS7_CI_NEEDS_NODE": "false",
            "OASIS7_CI_NEEDS_SYSTEM_DEPS": "false",
            "OASIS7_CI_NEEDS_TRUNK": "false",
            "OASIS7_CI_NEEDS_WASM_TARGET": "false",
        }
        self.assertEqual(expected, dict(item.split("=", 1) for item in environment.split()))

    def test_legacy_output_keeps_its_interpretation(self) -> None:
        self.assertEqual("", MODULE.render_versioned_environment("scope=minimal\nrun_rust_baseline=false\n"))
        with self.assertRaisesRegex(ValueError, "unversioned planner output contains versioned fields"):
            MODULE.render_versioned_environment("scope=minimal\nrun_doc_checker_contracts=false\n")

    def test_versioned_output_fails_closed_on_unknown_missing_or_malformed_fields(self) -> None:
        _, planner_output = planner_for_path("doc/engineering/project.md")
        lines = planner_output.splitlines()
        unknown = "\n".join("execution_contract=required-domain-split/v999" if line.startswith("execution_contract=") else line for line in lines)
        missing = "\n".join(line for line in lines if not line.startswith("needs_markdown="))
        malformed = "\n".join("needs_markdown=TRUE" if line.startswith("needs_markdown=") else line for line in lines)
        baseline_missing = "\n".join("needs_python=false" if line.startswith("needs_python=") else line for line in lines)
        for candidate in (unknown, missing, malformed, baseline_missing):
            with self.subTest(candidate=candidate):
                with self.assertRaises(ValueError):
                    MODULE.render_versioned_environment(candidate)


if __name__ == "__main__":
    unittest.main()
