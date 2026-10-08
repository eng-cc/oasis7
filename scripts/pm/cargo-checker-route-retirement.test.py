#!/usr/bin/env python3
"""Regression checks for the generic Cargo checker verification route."""

from pathlib import Path
import os
import subprocess
import tempfile
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
        # Execute the integration branch's actual generic dispatch with a spy
        # trusted driver; quoted paths must not obscure the required argv/M root.
        serial = required.split('            target_root="${INTEGRATION_WORKTREE:-${GITHUB_WORKSPACE}}"', 1)[1].split(
            '          elif [[ "${GITHUB_EVENT_NAME}"', 1)[0]
        preamble = required.split("      - name: Run required test tier\n", 1)[1].split("        run: |\n", 1)[1].split(
            '          trusted_profile_authority=', 1)[0]
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            driver = directory / "integration-planner/ci-tests.sh"
            driver.parent.mkdir()
            driver.write_text('#!/bin/sh\n[ -z "${GH_TOKEN+x}${GITHUB_TOKEN+x}" ] || exit 41\nprintf "%s\\n" "$@" > "$CAPTURE_ARGV"\n')
            target = directory / "exact-M"
            target.mkdir()
            capture = directory / "argv"
            environment = dict(os.environ, RUNNER_TEMP=str(directory), target_root=str(target), CAPTURE_ARGV=str(capture),
                               GH_TOKEN="fixture-gh", GITHUB_TOKEN="fixture-github")
            subprocess.run(["bash", "-euc", preamble + serial], env=environment, check=True)
            self.assertEqual(capture.read_text().splitlines(), ["required", "--repo-root", str(target),
                                                              "--impact-projection", str(directory / "impact-projection.json")])
        worker = job_block(workflow, "required-work", "required-gate")
        self.assertIn("Execute only the frozen selected worker", worker)
        self.assertIn("Verify all exact-attempt workers and complete gate obligations", required)
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
