#!/usr/bin/env python3
"""Regression checks for the generic Cargo checker verification route."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/rust.yml"


def job_block(workflow: str, job: str, next_job: str) -> str:
    start = workflow.index(f"\n  {job}:")
    end = workflow.index(f"\n  {next_job}:", start + 1)
    return workflow[start:end]


class CargoCheckerRouteRetirementContract(unittest.TestCase):
    def test_integration_revalidation_runs_generic_required_coverage(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required = job_block(workflow, "required-gate", "v1-reuse-validation-only")

        self.assertIn("cargo_package_profile_driver.py", required)
        self.assertIn("CI_VERBOSE=1 bash", required)
        self.assertIn("ci-tests.sh required", required)
        self.assertNotIn("OASIS7_CARGO_STAGE_", required)
        self.assertNotIn("checker-stage", required)
        self.assertNotIn("steps.checker-stage", workflow)

    def test_full_escalation_remains_an_independent_generic_route(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        full = job_block(workflow, "full-escalation", "newapi-bridge-package")

        self.assertIn("inputs.run_mode == 'full_escalation'", full)
        self.assertNotIn("\n    needs:", full)
        self.assertIn("Run human-authorized full test tier", full)
        self.assertIn("CI_VERBOSE=1 ./scripts/ci-tests.sh full", full)
        self.assertIn("steps.full.outcome", full)


if __name__ == "__main__":
    unittest.main(verbosity=2)
