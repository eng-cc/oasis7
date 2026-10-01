#!/usr/bin/env python3
"""Audit that the workflow-simplification acceptance suites remain reachable.

This aggregate is a coverage/wiring guard, not a substitute for the referenced
behavioral suites or independent human/live observations. In particular,
static repository tests cannot prove that an LLM follows a skill at runtime.
"""

from __future__ import annotations

from pathlib import Path
import re
import json
import os
import subprocess
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[2]
CI_TESTS = ROOT / "scripts/ci-tests.sh"
INVENTORY = ROOT / "scripts/ci-required-capability-test-inventory.tsv"

# Each automated suite is independently executed by
# run_workflow_governance_operational_contract_tests. Out-of-band rows need
# separate review or live read-only evidence; listing them here never marks
# them passed.
CASES: dict[str, dict[str, object]] = {}


def cases(ids: str, *, suites: tuple[str, ...] = (), out_of_band: str = "") -> None:
    for raw in ids.split():
        CASES[raw] = {"suites": suites, "out_of_band": out_of_band}


cases("T01", suites=("scripts/pm/workflow-bootstrap-fallback.test.py",))
cases("T02", out_of_band="independent professional review; static tests cannot prove LLM behavior")
cases("T03", out_of_band="human authorization-path review; no source-write permission is inferred")
cases("T04", suites=("scripts/pm/workflow-bootstrap-fallback.test.py", "scripts/pm/subagent-task-packet.test.py"))
cases("T05", suites=("scripts/pm/loop-policy.test.py", "scripts/pm/loop-ci-content.test.py"))
cases("T06", suites=("scripts/pm/loop.test.py", "scripts/pm/loop-ci.test.py"))
cases("T07 T08 T09", suites=("scripts/pm/loop-ci.test.py", "scripts/pm/loop-publication.integration.test.py"))
cases("T10 T11 T12", out_of_band="independent review of frozen diff, red-green ownership, and user-steered pause evidence")
cases("T13 T14 T15 T16 T17 T18", suites=("scripts/pm/review-plan.test.py", "scripts/pm/subagent-task-packet.test.py", "scripts/pm/ci-ready-receipt.test.py"))
cases("T19 T20 T21 T22 T23 T24 T25 T26 T27 T28", suites=("scripts/pm/loop-publication.integration.test.py", "scripts/pm/loop-recovery.test.py", "scripts/pm/loop-bootstrap.integration.test.py"))
cases("T29 T30 T31 T32 T33 T34 T35", suites=("scripts/pm/loop-ci.test.py", "scripts/pm/loop-ingress.test.py", "scripts/pm/loop.test.py", "scripts/pm/pr-lifecycle-loop.test.py", "scripts/pm/workflow-next.test.py"))
cases("T36 T37 T38 T39", suites=("scripts/pm/loop-policy.test.py", "scripts/pm/loop-contracts.test.py", "scripts/pm/github-project-task-policy-adoption.integration.test.py"))
cases("T40 T41 T42", suites=("scripts/pm/loop_terminal.test.py", "scripts/pm/loop-ci.test.py", "scripts/pm/loop-ingress.test.py", "scripts/pm/loop.test.py", "scripts/pm/pr-lifecycle-loop.test.py", "scripts/pm/workflow-next.test.py"))


class WorkflowSimplificationCoverageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.ci_tests = CI_TESTS.read_text(encoding="utf-8")
        cls.inventory = INVENTORY.read_text(encoding="utf-8")
        marker = "run_workflow_governance_operational_contract_tests() {"
        start = cls.ci_tests.index(marker)
        end = cls.ci_tests.index("\n}", start) + 2
        cls.operational_runner = cls.ci_tests[start:end]

    def test_every_design_acceptance_id_has_an_explicit_evidence_class(self) -> None:
        expected = {f"T{number:02}" for number in range(1, 43)}
        self.assertEqual(set(CASES), expected)
        for case_id, evidence in CASES.items():
            suites = evidence["suites"]
            out_of_band = evidence["out_of_band"]
            with self.subTest(case_id=case_id):
                self.assertTrue(suites or out_of_band, f"{case_id} has no evidence route")
                if suites:
                    self.assertEqual(out_of_band, "", f"{case_id} cannot be both implicit CI and out-of-band")
                else:
                    self.assertIn("review", str(out_of_band).lower(), case_id)

    def test_automated_suite_paths_exist_and_run_in_workflow_governance(self) -> None:
        suites = sorted({suite for case in CASES.values() for suite in case["suites"]})
        for suite in suites:
            with self.subTest(suite=suite):
                self.assertTrue((ROOT / suite).is_file(), f"missing acceptance suite {suite}")
                command = re.compile(rf"^\s*run (?:python3|env PYTHONDONTWRITEBYTECODE=1 python3) \./{re.escape(suite)}\s*$", re.M)
                self.assertRegex(self.operational_runner, command)

    def test_governance_aggregate_remains_reachable_from_required_and_full_entries(self) -> None:
        contract_runner = re.search(
            r"run_workflow_governance_contract_tests\(\) \{(?P<body>.*?)^\}",
            self.ci_tests, re.M | re.S,
        )
        self.assertIsNotNone(contract_runner)
        self.assertIn("run_workflow_governance_operational_contract_tests", contract_runner.group("body"))
        full_runner = re.search(
            r"run_all_required_gate_capability_contract_tests\(\) \{(?P<body>.*?)^\}",
            self.ci_tests, re.M | re.S,
        )
        self.assertIsNotNone(full_runner)
        self.assertIn("run_workflow_governance_contract_tests", full_runner.group("body"))
        rows = [line.split("\t") for line in self.inventory.splitlines() if line.strip()]
        self.assertTrue(any(
            len(row) >= 2 and row[0] == "run_operational_contract_tests"
            and "scripts/pm/workflow-simplification.test.py" in row[1].split(",")
            for row in rows
        ), "historical full-suite inventory must retain the aggregate")

    def test_start_final_and_trusted_old_helper_cases_are_not_hidden(self) -> None:
        for case_id in ("T29", "T30", "T31", "T32", "T33", "T34", "T35", "T41", "T42"):
            self.assertIn("scripts/pm/loop-ci.test.py", CASES[case_id]["suites"], case_id)
        self.assertIn("scripts/pm/loop-ingress.test.py", CASES["T42"]["suites"])
        self.assertIn("scripts/pm/workflow-next.test.py", CASES["T30"]["suites"])
        self.assertIn("scripts/pm/pr-lifecycle-loop.test.py", CASES["T31"]["suites"])
        self.assertIn("scripts/pm/github-project-task-policy-adoption.integration.test.py", CASES["T37"]["suites"])

    def test_out_of_band_rows_are_never_counted_as_static_ci_proofs(self) -> None:
        for case_id in ("T02", "T03", "T10", "T11", "T12"):
            evidence = CASES[case_id]
            with self.subTest(case_id=case_id):
                self.assertEqual(evidence["suites"], ())
                self.assertTrue(evidence["out_of_band"])
        self.assertIn("cannot prove LLM behavior", CASES["T02"]["out_of_band"])

    def test_required_gate_dispatches_exact_trusted_old_or_new_helper(self) -> None:
        workflow = (ROOT / ".github/workflows/rust.yml").read_text(encoding="utf-8")

        def step_script(marker: str) -> str:
            step = workflow.index(marker)
            run_start = workflow.index("        run: |\n", step) + len("        run: |\n")
            run_end = workflow.index("\n      - ", run_start)
            return textwrap.dedent(workflow[run_start:run_end]).rstrip() + "\n"

        admission = step_script("      - id: loop-ci-admission\n")
        final = step_script("      - name: Verify final task and PR binding before required-gate success\n")
        replacements = {
            "${{ github.repository }}": "eng-cc/oasis7",
            "${{ github.event.pull_request.base.sha || inputs.integration_base }}": "a" * 40,
            "${{ github.event.pull_request.head.sha || inputs.expected_head }}": "b" * 40,
            "${{ github.event.pull_request.number || inputs.pr_number }}": "17",
            "${{ steps.scope.outputs.source_scope_base }}": "c" * 40,
            "${{ steps.scope.outputs.planner_config_sha256 }}": "sha256:" + "1" * 64,
            "${{ steps.scope.outputs.planner_digest }}": "sha256:" + "2" * 64,
            "${{ steps.scope.outputs.impact_projection_digest }}": "sha256:" + "3" * 64,
        }

        def render(script: str, *, start_only: bool = False, base_oid: str) -> str:
            rendered_replacements = dict(replacements)
            rendered_replacements["${{ github.event.pull_request.base.sha || inputs.integration_base }}"] = base_oid
            for source, target in rendered_replacements.items():
                script = script.replace(source, target)
            return script.replace(
                "${{ steps.loop-ci-admission.outputs.start_only }}",
                "true" if start_only else "false",
            )

        def helper(supports_phase: bool, *, candidate: bool = False) -> str:
            if candidate:
                return "raise SystemExit('candidate helper was executed')\n"
            help_text = "usage: loop-ci [--phase legacy|start|final]" if supports_phase else "usage: loop-ci"
            return (
                "import json, os, sys\n"
                "args = sys.argv[1:]\n"
                "if args == ['--help']:\n"
                f"    print({help_text!r})\n"
                "else:\n"
                "    with open(os.environ['CALL_LOG'], 'a', encoding='utf-8') as out:\n"
                "        out.write(json.dumps(args) + '\\n')\n"
                "    if '--phase' in args and 'start' in args:\n"
                "        with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as out:\n"
                "            out.write('start_only=false\\n')\n"
            )

        def run_script(script: str, env: dict[str, str], cwd: Path) -> None:
            completed = subprocess.run(
                ["bash", "-eo", "pipefail", "-c", script], cwd=cwd,
                env={**os.environ, **env}, text=True, capture_output=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)

        def run_pair(supports_phase: bool, *, start_only: bool = False) -> list[list[str]]:
            with tempfile.TemporaryDirectory(prefix="workflow-loop-ci-compat-") as temporary:
                root = Path(temporary)
                (root / "scripts/pm").mkdir(parents=True)
                helper_path = root / "scripts/pm/loop-ci.py"
                helper_path.write_text(helper(supports_phase), encoding="utf-8")
                subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
                subprocess.run(["git", "-C", str(root), "config", "user.name", "Fixture"], check=True)
                subprocess.run(["git", "-C", str(root), "config", "user.email", "fixture@example.invalid"], check=True)
                subprocess.run(["git", "-C", str(root), "add", "scripts/pm/loop-ci.py"], check=True)
                subprocess.run(["git", "-C", str(root), "commit", "-qm", "trusted event-base helper"], check=True)
                base = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
                # Candidate content is deliberately different; workflow must load the exact base blob.
                helper_path.write_text(helper(False, candidate=True), encoding="utf-8")
                runner = root / "runner"
                runner.mkdir()
                output = root / "github-output"
                log = root / "calls.jsonl"
                env = {
                    "RUNNER_TEMP": str(runner), "GITHUB_OUTPUT": str(output),
                    "CALL_LOG": str(log), "GITHUB_EVENT_NAME": "pull_request",
                    "GITHUB_SHA": "b" * 40,
                }
                run_script(render(admission, base_oid=base), env, root)
                run_script(render(final, start_only=start_only, base_oid=base), env, root)
                return [json.loads(line) for line in log.read_text(encoding="utf-8").splitlines()]

        old_calls = run_pair(False)
        self.assertEqual(len(old_calls), 2)
        self.assertEqual(old_calls[0], old_calls[1])
        self.assertEqual(old_calls[0][:4], ["--repository", "eng-cc/oasis7", "--pr-number", "17"])
        self.assertEqual(old_calls[0][4], "--base")
        self.assertRegex(old_calls[0][5], r"^[0-9a-f]{40}$")
        self.assertEqual(old_calls[0][6:], ["--head", "b" * 40])
        self.assertNotIn("--phase", old_calls[0])
        self.assertNotIn("--scope-base-oid", old_calls[0])
        new_calls = run_pair(True)
        self.assertEqual(new_calls[0][:2], ["--phase", "start"])
        self.assertIn("--scope-base-oid", new_calls[0])
        self.assertEqual(new_calls[1][:3], ["--phase", "final", "--tests-passed"])
        self.assertIn("--scope-base-oid", new_calls[1])
        start_only_calls = run_pair(True, start_only=True)
        self.assertIn("--start-only", start_only_calls[1])


if __name__ == "__main__":
    unittest.main()
