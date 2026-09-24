#!/usr/bin/env python3
"""Behavior contract for the shared CI/review impact projection."""
from __future__ import annotations

import json
import importlib.util
import hashlib
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("workflow-impact-projection.py")
ROOT = Path(__file__).parents[2]
SPEC = importlib.util.spec_from_file_location("workflow_impact_projection", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
WORKFLOW_IMPACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(WORKFLOW_IMPACT)


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
            "task_uid": "task_" + "1" * 32,
            "source_head_oid": "a" * 40,
            "scope_base_oid": "b" * 40,
            "changed_paths": ["doc/product/world-rules-core-gameplay.prd.md"],
            "change_class": "workflow-doc",
            "manual_roles": [],
            "domain_role": None,
            "test_profile": "required",
            "declared_tests": ["required_gate_baseline"],
            "consumed_contracts": [{"id": "workflow-contract", "revision": "v1"}],
            "public_semantics": [],
            "affected_consumers": ["required-ci"],
            "closure_status": {"status": "complete", "reason": "verified", "evidence": [{
                "path": "Cargo.toml",
                "sha256": "sha256:" + hashlib.sha256((ROOT / "Cargo.toml").read_bytes()).hexdigest(),
            }]},
        }

    def test_complete_known_scope_derives_ci_and_review_obligations(self) -> None:
        first = json.loads(self.run_projection(self.base_input()).stdout)
        second = json.loads(self.run_projection(self.base_input()).stdout)

        self.assertEqual("oasis7-workflow-impact-projection/v2", first["schema"])
        self.assertEqual("task_" + "1" * 32, first["task_uid"])
        self.assertEqual("a" * 40, first["source_head_oid"])
        self.assertEqual("b" * 40, first["scope_base_oid"])
        self.assertEqual("required", first["test_profile"])
        self.assertEqual(["required_gate_baseline"], first["declared_tests"])
        self.assertRegex(first["planner_config_sha256"], r"^sha256:[0-9a-f]{64}$")
        self.assertRegex(first["planner_digest"], r"^sha256:[0-9a-f]{64}$")
        self.assertEqual(first["review_roles"], first["ordered_role_ids"])
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

    def test_verified_loader_rejects_digest_valid_projection_with_missing_field(self) -> None:
        projection = json.loads(self.run_projection(self.base_input()).stdout)
        del projection["affected_consumers"]
        projection["projection_digest"] = WORKFLOW_IMPACT.canonical_digest({
            key: value for key, value in projection.items() if key != "projection_digest"
        })
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "incomplete.json"
            path.write_text(json.dumps(projection), encoding="utf-8")
            with self.assertRaisesRegex(WORKFLOW_IMPACT.ProjectionError, "missing=affected_consumers"):
                WORKFLOW_IMPACT.load_verified_projection(path)

    def test_verified_loader_rechecks_closure_evidence_against_repository(self) -> None:
        projection = json.loads(self.run_projection(self.base_input()).stdout)
        projection["source_head_oid"] = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=True
        ).stdout.strip()
        projection["closure_status"]["evidence"][0]["sha256"] = "sha256:" + "0" * 64
        projection["projection_digest"] = WORKFLOW_IMPACT.canonical_digest({
            key: value for key, value in projection.items() if key != "projection_digest"
        })
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "forged-closure.json"
            path.write_text(json.dumps(projection), encoding="utf-8")
            with self.assertRaisesRegex(WORKFLOW_IMPACT.ProjectionError, "evidence\[0\] digest mismatch"):
                WORKFLOW_IMPACT.load_verified_projection(path, repo_root=ROOT)

    def test_verified_loader_binds_closure_evidence_to_source_head_not_worktree(self) -> None:
        projection = json.loads(self.run_projection(self.base_input()).stdout)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            evidence_path = root / "contract.txt"
            evidence_path.write_text("frozen contract\n", encoding="utf-8")
            subprocess.run(["git", "add", "contract.txt"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "frozen"], cwd=root, check=True)
            source_head = subprocess.run(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True, capture_output=True, check=True
            ).stdout.strip()
            projection["source_head_oid"] = source_head
            projection["closure_status"]["evidence"] = [{
                "path": "contract.txt",
                "sha256": "sha256:" + hashlib.sha256(b"frozen contract\n").hexdigest(),
            }]
            projection["projection_digest"] = WORKFLOW_IMPACT.canonical_digest({
                key: value for key, value in projection.items() if key != "projection_digest"
            })
            projection_path = root / "projection.json"
            projection_path.write_text(json.dumps(projection), encoding="utf-8")

            evidence_path.write_text("candidate checkout drift\n", encoding="utf-8")

            loaded = WORKFLOW_IMPACT.load_verified_projection(projection_path, repo_root=root)
            self.assertEqual(source_head, loaded["source_head_oid"])

    def test_missing_identity_and_test_contract_fields_fail_closed(self) -> None:
        for field in ("task_uid", "source_head_oid", "scope_base_oid", "test_profile", "declared_tests"):
            with self.subTest(field=field):
                payload = self.base_input()
                del payload[field]
                result = self.run_projection(payload, ok=False)
                self.assertIn(field, result.stderr)

    def test_unknown_test_profile_does_not_authorize_a_narrow_projection(self) -> None:
        payload = self.base_input()
        payload["test_profile"] = "unknown"
        result = self.run_projection(payload, ok=False)
        self.assertIn("test_profile", result.stderr)


if __name__ == "__main__":
    unittest.main()
