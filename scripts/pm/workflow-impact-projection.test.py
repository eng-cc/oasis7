#!/usr/bin/env python3
"""Behavior contract for the shared CI/review impact projection."""
from __future__ import annotations

import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("workflow-impact-projection.py")
ROOT = Path(__file__).parents[2]


class WorkflowImpactProjectionTests(unittest.TestCase):
    def run_projection(self, payload: dict[str, object], *, ok: bool = True) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as temp:
            input_path = Path(temp) / "projection-input.json"
            input_path.write_text(json.dumps(payload), encoding="utf-8")
            result = subprocess.run(
                [
                    str(SCRIPT), "--root", str(ROOT), "--input", str(input_path),
                ],
                text=True,
                capture_output=True,
            )
        if ok and result.returncode != 0:
            self.fail(f"projection command failed: {result.stderr}")
        if not ok and result.returncode == 0:
            self.fail(f"projection command unexpectedly passed: {result.stdout}")
        return result

    @staticmethod
    def base_input() -> dict[str, object]:
        return {
            "changed_paths": ["doc/product/world-rules-core-gameplay.prd.md"],
            "change_class": "workflow-doc",
            "manual_roles": [],
            "domain_role": None,
            "consumed_contracts": [{"id": "workflow-contract", "revision": "v1"}],
            "public_semantics": [],
            "affected_consumers": ["required-ci"],
            "closure_status": {"status": "complete", "reason": "verified"},
        }

    def test_complete_known_scope_derives_ci_and_review_obligations(self) -> None:
        first = json.loads(self.run_projection(self.base_input()).stdout)
        second = json.loads(self.run_projection(self.base_input()).stdout)

        self.assertEqual("oasis7-workflow-impact-projection/v1", first["schema"])
        self.assertEqual("minimal", first["ci_scope"])
        self.assertEqual(["required_gate_baseline"], first["ci_capabilities"])
        self.assertEqual(
            ["repository_health_engineer", "qa_engineer"],
            first["review_roles"],
        )
        self.assertEqual("targeted", first["review_scope"])
        self.assertFalse(first["review_escalated"])
        self.assertTrue(any("governance_doc:" in reason for reason in first["ci_reasons"]))
        self.assertIn("input:consumed_contracts", first["review_reasons"])
        self.assertRegex(first["projection_digest"], r"^sha256:[0-9a-f]{64}$")
        self.assertEqual(first["projection_digest"], second["projection_digest"])

    def test_unknown_dependency_closure_forces_full_ci_and_review_escalation(self) -> None:
        payload = self.base_input()
        payload["closure_status"] = {
            "status": "unknown",
            "reason": "dependency API unavailable",
        }
        result = json.loads(self.run_projection(payload).stdout)

        self.assertEqual("full", result["ci_scope"])
        self.assertIn("oasis7_required", result["ci_capabilities"])
        self.assertTrue(result["review_escalated"])
        self.assertEqual("full", result["review_scope"])
        self.assertIn("dependency_closure_unverified:unknown", result["ci_reasons"])
        self.assertIn("dependency_closure_unverified:unknown", result["review_reasons"])
        self.assertIn("dependency_closure_reason:dependency API unavailable", result["review_reasons"])

    def test_domain_review_class_passes_canonical_domain_role_to_selector(self) -> None:
        payload = self.base_input()
        payload.update({
            "changed_paths": ["doc/world-runtime/runtime-contract.prd.md"],
            "change_class": "domain-semantic-doc",
            "domain_role": "runtime_engineer",
            "public_semantics": ["runtime admission contract"],
            "affected_consumers": ["runtime-admission"],
        })
        result = json.loads(self.run_projection(payload).stdout)

        self.assertEqual(
            ["repository_health_engineer", "runtime_engineer"],
            result["review_roles"],
        )
        self.assertFalse(result["review_escalated"])
        self.assertIn("input:public_semantics", result["review_reasons"])
        self.assertIn("input:affected_consumers", result["review_reasons"])

    def test_unmatched_path_forces_full_scope_without_narrowing_manual_roles(self) -> None:
        payload = self.base_input()
        payload.update({
            "changed_paths": ["unclassified/generated-contract.bin"],
            "change_class": "unknown",
            "manual_roles": ["runtime_engineer", "qa_engineer"],
            "consumed_contracts": [],
            "affected_consumers": [],
        })
        result = json.loads(self.run_projection(payload).stdout)

        self.assertEqual("full", result["ci_scope"])
        self.assertTrue(result["review_escalated"])
        self.assertEqual("full", result["review_scope"])
        self.assertEqual(["runtime_engineer", "qa_engineer"], result["review_roles"])
        self.assertTrue(any("unmatched_path:unclassified/generated-contract.bin" == reason
                            for reason in result["ci_reasons"]))
        self.assertTrue(any("unmatched_path:unclassified/generated-contract.bin" == reason
                            for reason in result["review_reasons"]))

    def test_missing_explicit_input_field_fails_closed(self) -> None:
        payload = self.base_input()
        del payload["consumed_contracts"]
        result = self.run_projection(payload, ok=False)
        self.assertIn("consumed_contracts", result.stderr)


if __name__ == "__main__":
    unittest.main()
