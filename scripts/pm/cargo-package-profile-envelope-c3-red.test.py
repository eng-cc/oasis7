#!/usr/bin/env python3
"""C3 RED contracts for package-profile provenance envelopes."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/rust.yml"
CI_READY = ROOT / "scripts/pm/ci-ready-receipt.py"
CI_READY_TEST = ROOT / "scripts/pm/ci-ready-receipt.test.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CargoPackageProfileEnvelopeRED(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.receipt_tests = load_module(CI_READY_TEST, "ci_ready_receipt_c3_envelope")

    def _envelope(self, **overrides: object) -> dict[str, object]:
        envelope: dict[str, object] = {
            "schema": "oasis7-cargo-package-profile-envelope/v1",
            "repository": "eng-cc/oasis7",
            "task_uid": "task_12345678901234567890123456789012",
            "task_issue_number": 1,
            "pr_number": 7,
            "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
            "workflow_sha": "b" * 40,
            "run_id": 9,
            "run_attempt": 1,
            "check_name": "required-gate",
            "check_app_id": 42,
            "check_run_id": 9,
            "integration_base": "b" * 40,
            "source_head": "a" * 40,
            "tested_tree": "t" * 40,
            "plan_digest": "sha256:" + "1" * 64,
            "results_digest": "sha256:" + "2" * 64,
            "receipt_digest": "sha256:" + "3" * 64,
        }
        envelope.update(overrides)
        return envelope

    def test_workflow_and_live_receipt_consume_one_profile_provenance_envelope(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        profile_start = required.index('profile_output="${GITHUB_WORKSPACE}/output/cargo-package-profile"')
        profile_block = required[profile_start:]
        for field in (
            "cargo-package-profile-envelope",
            "task_uid",
            "pr_number",
            "repository",
            "workflow_ref",
            "workflow_sha",
            "run_id",
            "run_attempt",
            "check_app_id",
            "check_run_id",
            "integration_base",
            "source_head",
            "tested_tree",
            "plan_digest",
            "results_digest",
            "receipt_digest",
        ):
            with self.subTest(field=field):
                self.assertIn(field, profile_block)

        receipt_source = CI_READY.read_text(encoding="utf-8")
        for field in (
            "cargo_package_profile",
            "plan_digest",
            "results_digest",
            "receipt_digest",
        ):
            with self.subTest(ci_ready_field=field):
                self.assertIn(field, receipt_source)

    def test_envelope_lookup_step_exports_repository_scoped_github_token(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        profile_step = required.split(
            "      - name: Run required test tier\n", 1
        )[1].split("\n      - name:", 1)[0]
        env_block = profile_step.split("        env:\n", 1)[1].split(
            "\n        run:", 1
        )[0]
        self.assertIn(
            "GH_TOKEN: ${{ github.token }}",
            env_block,
            "the envelope lookup requires the repository-scoped workflow token",
        )

    def test_ci_ready_rejects_detached_cross_run_or_digest_mismatch_profile_receipt(self) -> None:
        case = self.receipt_tests.ReceiptTest("test_success")
        mutations = {
            "run_id": 999,  # cross-run evidence
            "repository": "other/repo",
            "plan_digest": "sha256:" + "9" * 64,  # digest mismatch
        }
        for field, bad_value in mutations.items():
            def mutate(receipt: dict[str, object], field=field, bad_value=bad_value) -> None:
                envelope = self._envelope()
                envelope[field] = bad_value
                receipt["cargo_package_profile"] = envelope

            with self.subTest(field=field), self.assertRaisesRegex(
                Exception, "package|profile|digest|run|identity|mismatch"
            ):
                case.invoke_verify(mutate)


if __name__ == "__main__":
    unittest.main(verbosity=2)
