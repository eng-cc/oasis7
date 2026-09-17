#!/usr/bin/env python3
"""Cross-consumer contract for one verified workflow impact projection."""
from __future__ import annotations

import json
import importlib.util
import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).parents[2]
PROJECTION = ROOT / "scripts" / "pm" / "workflow-impact-projection.py"
PLANNER = ROOT / "scripts" / "plan-rust-required-scope.sh"
SELECTOR = ROOT / "scripts" / "pm" / "review-role-selector.py"
REVIEW_PLAN = ROOT / "scripts" / "pm" / "review-plan.py"
CLOSEOUT = ROOT / "scripts" / "pm" / "task-closeout.sh"
SPEC = importlib.util.spec_from_file_location("workflow_impact_projection", PROJECTION)
assert SPEC is not None and SPEC.loader is not None
WORKFLOW_IMPACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(WORKFLOW_IMPACT)


class WorkflowImpactConsumersTests(unittest.TestCase):
    def input_payload(self) -> dict[str, object]:
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
                "path": "scripts/ci-required-scope.v2.json",
                "sha256": "sha256:" + hashlib.sha256((ROOT / "scripts/ci-required-scope.v2.json").read_bytes()).hexdigest(),
            }]},
        }

    def write_projection(self, directory: Path) -> Path:
        source = directory / "projection-input.json"
        output = directory / "projection.json"
        source.write_text(json.dumps(self.input_payload()), encoding="utf-8")
        result = subprocess.run(
            [str(PROJECTION), "--root", str(ROOT), "--input", str(source), "--out", str(output)],
            text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        return output

    def run_consumer(self, command: list[str], *, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(command, text=True, capture_output=True)
        if ok:
            self.assertEqual(0, result.returncode, result.stderr)
        else:
            self.assertNotEqual(0, result.returncode, result.stdout)
        return result

    def test_planner_and_role_selector_return_the_same_verified_projection_identity(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            projection_path = self.write_projection(Path(raw))
            planner = self.run_consumer([
                str(PLANNER), "--event-name", "pull_request",
                "--task-uid", "task_" + "1" * 32,
                "--head-ref", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path", "doc/product/world-rules-core-gameplay.prd.md",
                "--impact-projection", str(projection_path),
            ])
            selector = self.run_consumer([
                str(SELECTOR), "--change-class", "workflow-doc",
                "--task-uid", "task_" + "1" * 32,
                "--source-head-oid", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path-list", "doc/product/world-rules-core-gameplay.prd.md",
                "--impact-projection", str(projection_path), "--json",
            ])
            projection = json.loads(projection_path.read_text(encoding="utf-8"))
            planner_fields = dict(
                line.split("=", 1) for line in planner.stdout.splitlines() if "=" in line
            )
            selector_value = json.loads(selector.stdout)
            self.assertEqual(projection["projection_digest"], planner_fields["impact_projection_digest"])
            self.assertEqual(projection["projection_digest"], selector_value["impact_projection_digest"])
            self.assertEqual("verified", planner_fields["impact_projection_status"])
            self.assertEqual("verified", selector_value["impact_projection_status"])
            self.assertEqual(projection["declared_tests"], planner_fields["declared_tests"].split(";"))
            self.assertEqual(projection["test_profile"], planner_fields["test_profile"])
            self.assertEqual(projection["planner_config_sha256"], planner_fields["planner_config_sha256"])

    def test_tampered_projection_fails_closed_for_planner_and_selector(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            projection_path = self.write_projection(Path(raw))
            tampered = json.loads(projection_path.read_text(encoding="utf-8"))
            tampered["declared_tests"] = ["oasis7_required"]
            projection_path.write_text(json.dumps(tampered), encoding="utf-8")
            planner = self.run_consumer([
                str(PLANNER), "--event-name", "pull_request",
                "--task-uid", "task_" + "1" * 32,
                "--head-ref", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path", "doc/product/world-rules-core-gameplay.prd.md",
                "--impact-projection", str(projection_path),
            ], ok=False)
            self.assertIn("impact projection", planner.stderr.lower())
            selector = self.run_consumer([
                str(SELECTOR), "--change-class", "workflow-doc",
                "--task-uid", "task_" + "1" * 32,
                "--source-head-oid", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path-list", "doc/product/world-rules-core-gameplay.prd.md",
                "--impact-projection", str(projection_path), "--json",
            ], ok=False)
            self.assertIn("impact projection", selector.stderr.lower())

    def test_planner_config_identity_drift_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            projection_path = self.write_projection(directory)
            config = json.loads((ROOT / "scripts" / "ci-required-scope.v2.json").read_text(encoding="utf-8"))
            config["rules"][0]["reason"] = "governance_doc_config_drift"
            config_path = directory / "ci-required-scope-drift.json"
            config_path.write_text(json.dumps(config), encoding="utf-8")
            planner = self.run_consumer([
                str(PLANNER), "--event-name", "pull_request",
                "--task-uid", "task_" + "1" * 32,
                "--head-ref", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path", "doc/product/world-rules-core-gameplay.prd.md",
                "--config", str(config_path), "--impact-projection", str(projection_path),
            ], ok=False)
            self.assertIn("planner config identity", planner.stderr.lower())

    def test_planner_and_selector_reject_stale_head_and_base_identity(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            projection_path = self.write_projection(Path(raw))
            projection = json.loads(projection_path.read_text())
            projection["source_head_oid"] = "c" * 40
            projection["scope_base_oid"] = "d" * 40
            projection["projection_digest"] = WORKFLOW_IMPACT.canonical_digest({
                key: value for key, value in projection.items() if key != "projection_digest"
            })
            projection_path.write_text(json.dumps(projection))
            planner = self.run_consumer([
                str(PLANNER), "--event-name", "pull_request",
                "--task-uid", "task_" + "1" * 32,
                "--head-ref", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path", "doc/product/world-rules-core-gameplay.prd.md",
                "--impact-projection", str(projection_path),
            ], ok=False)
            self.assertIn("identity mismatch", planner.stderr.lower())
            selector = self.run_consumer([
                str(SELECTOR), "--change-class", "workflow-doc",
                "--task-uid", "task_" + "1" * 32,
                "--source-head-oid", "a" * 40, "--scope-base-oid", "b" * 40,
                "--changed-path-list", "doc/product/world-rules-core-gameplay.prd.md",
                "--impact-projection", str(projection_path), "--json",
            ], ok=False)
            self.assertIn("identity mismatch", selector.stderr.lower())

    def test_review_plan_and_closeout_expose_the_same_projection_contract(self) -> None:
        review_help = self.run_consumer([str(REVIEW_PLAN), "--help"])
        self.assertIn("--source-review-input", review_help.stdout)
        self.assertIn("impact_projection_digest", CLOSEOUT.read_text(encoding="utf-8"))

    def test_missing_projection_path_fails_closed(self) -> None:
        missing = "/tmp/oasis7-missing-impact-projection.json"
        planner = self.run_consumer([
            str(PLANNER), "--event-name", "pull_request",
            "--changed-path", "doc/product/world-rules-core-gameplay.prd.md",
            "--impact-projection", missing,
        ], ok=False)
        self.assertIn("impact projection", planner.stderr.lower())


if __name__ == "__main__":
    unittest.main()
