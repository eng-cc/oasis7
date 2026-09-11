#!/usr/bin/env python3
"""Behavior contract for deterministic, reusable review planning."""
from __future__ import annotations

import json
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("review-plan.py")
TASK = "task_" + "1" * 32
EVIDENCE = "b" * 64
COMPARISON_REF = "refs/remotes/origin/main"


class ReviewPlanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        mapping = self.root / '.pm/github-project-sync'
        mapping.mkdir(parents=True)
        (mapping / 'tasks.json').write_text(json.dumps({'tasks': {TASK: {'task_uid': TASK, 'repository': 'fixture/repo', 'issue_number': 1}}}))
        fakebin = self.root / 'fakebin'
        fakebin.mkdir()
        gh = fakebin / 'gh'
        gh.write_text('#!/usr/bin/env python3\nimport json,sys\nprint(json.dumps([] if any("/comments" in arg for arg in sys.argv) else {"body": "task_uid: ' + TASK + '"}))\n')
        gh.chmod(0o755)
        environment = patch.dict(os.environ, {'PATH': str(fakebin) + os.pathsep + os.environ['PATH']})
        environment.start()
        self.addCleanup(environment.stop)
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        (self.root / "README").write_text("fixture\n", encoding="utf-8")
        self.git("add", "README")
        self.git("commit", "-m", "base")
        self.head = self.git("rev-parse", "HEAD")
        self.comparison_ref = COMPARISON_REF
        self.git("update-ref", self.comparison_ref, "HEAD")
        self.comparison_oid = self.git("rev-parse", self.comparison_ref)
        self.out = self.root / "plan.json"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", "-C", str(self.root), *args], check=True, text=True,
            stdout=subprocess.PIPE,
        ).stdout.strip()

    def run_plan(self, *extra: str, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
             [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
              "--head", self.head, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
             "--comparison-oid", self.comparison_oid, "--out", str(self.out), *extra],
            text=True,
            capture_output=True,
        )
        if ok and result.returncode != 0:
            self.fail(f"command failed: {result.stderr}")
        if not ok and result.returncode == 0:
            self.fail(f"command unexpectedly passed: {result.stdout}")
        return result

    def plan(self, *extra: str) -> dict[str, object]:
        return json.loads(self.run_plan(*extra).stdout)

    def receipt_plan(self, receipt: Path, out: Path) -> dict[str, object]:
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK, "--head", self.head,
             "--ci-ready-receipt", str(receipt), "--change-class", "workflow-doc",
             "--comparison-ref", self.comparison_ref, "--comparison-oid", self.comparison_oid,
             "--out", str(out)], text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        return json.loads(result.stdout)

    def complete_collected_plan(self, plan: dict[str, object]) -> None:
        preflight = plan["preflight"]
        assert isinstance(preflight, dict)
        ledger_path = Path(str(preflight["ledger_path"]))
        rows = []
        for item in plan["expected_slices"]:
            assert isinstance(item, dict)
            artifact_path = ledger_path.parent / f"{item['slice_id']}.json"
            artifact_path.write_text(json.dumps({
                "role": item["role"], "slice_id": item["slice_id"],
                "task_uid": TASK, "head": plan["frozen_head"], "epoch": plan["epoch"],
                "status": "completed", "disposition": "no_findings",
                "findings": [], "residual_risk": "none",
            }), encoding="utf-8")
            rows.append({
                "role": item["role"], "slice_id": item["slice_id"],
                "task_uid": TASK, "head": plan["frozen_head"], "epoch": plan["epoch"],
                "status": "completed",
                "artifact_digest": hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
                "artifacts": [str(artifact_path)],
            })
        ledger_path.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows), encoding="utf-8")
        ledger_digest = hashlib.sha256(ledger_path.read_bytes()).hexdigest()
        Path(str(plan["collection_path"])).write_text(json.dumps({
            "schema": "oasis7-review-collection/v1", "status": "passed",
            "epoch": plan["epoch"], "task_uid": TASK, "frozen_head": plan["frozen_head"],
            "ledger_digest": ledger_digest,
            "roles": sorted(str(item["role"]) for item in plan["expected_slices"]),
        }), encoding="utf-8")

    def write_receipt(self, path: Path, *, base_oid: str, head_oid: str) -> None:
        path.write_text(json.dumps({
            "receipt_type": "oasis7_ci_ready_receipt", "issuer": "github_live_query",
            "repository": "example/repo", "task_uid": TASK, "task_issue_number": 1,
            "pr_number": 2, "base_oid": base_oid, "head_oid": head_oid,
            "check_name": "required-gate", "check_app_id": 42, "check_run_id": 7,
            "planner_digest": "c" * 64, "planner_config_sha256": "sha256:" + "d" * 64,
            "run_rust_baseline": True, "conclusion": "success",
            "observed_at": "2026-01-01T00:00:00Z",
        }))

    def test_ci_receipt_refresh_reuses_review_epoch_but_authority_drift_does_not(self) -> None:
        authority = {"receipt_type": "oasis7_ci_ready_receipt", "issuer": "github_live_query",
                     "repository": "example/repo", "task_uid": TASK, "task_issue_number": 1,
                     "pr_number": 2, "base_oid": self.comparison_oid, "head_oid": self.head,
                     "check_name": "required-gate", "check_app_id": 42, "check_run_id": 7,
                     "planner_digest": "c" * 64, "planner_config_sha256": "sha256:" + "d" * 64,
                     "run_rust_baseline": True, "conclusion": "success"}
        first_receipt = self.root / "receipt-a.json"
        second_receipt = self.root / "receipt-b.json"
        first_receipt.write_text(json.dumps({**authority, "observed_at": "2026-01-01T00:00:00Z"}))
        second_receipt.write_text(json.dumps({**authority, "observed_at": "2026-01-01T00:10:00Z"}))
        first = self.receipt_plan(first_receipt, self.root / "receipt-a-plan.json")
        refreshed = self.receipt_plan(second_receipt, self.root / "receipt-b-plan.json")
        self.assertEqual(first["epoch"], refreshed["epoch"])
        self.assertEqual(first["expected_slices"], refreshed["expected_slices"])
        changed = self.root / "receipt-changed.json"
        changed.write_text(json.dumps({**authority, "check_run_id": 8, "observed_at": "2026-01-01T00:10:00Z"}))
        drifted = self.receipt_plan(changed, self.root / "receipt-changed-plan.json")
        self.assertNotEqual(first["epoch"], drifted["epoch"])

    def test_explicit_document_risk_class_selects_the_minimum_deterministic_roles(self) -> None:
        plan = self.plan()
        self.assertEqual(
            ["repository_health_engineer", "qa_engineer"],
            plan["roles"],
        )
        self.assertEqual(plan["roles"], [item["role"] for item in plan["expected_slices"]])
        self.assertEqual(2, len(plan["packet_refs"]))

    def test_unchanged_identity_reuses_stable_slice_uuids_and_epoch(self) -> None:
        first = self.plan()
        retry = self.plan()
        self.assertTrue(retry["reused"])
        self.assertEqual(first["epoch"], retry["epoch"])
        self.assertEqual(first["expected_slices"], retry["expected_slices"])
        self.assertEqual(first["packet_refs"], retry["packet_refs"])
        for item in retry["expected_slices"]:
            self.assertRegex(item["slice_id"],
                             r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")

    def test_unchanged_identity_reuses_batch_with_a_different_explicit_plan_path(self) -> None:
        first = self.plan()
        alternate_out = self.root / "alternate-plan.json"
        result = subprocess.run(
             [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
              "--head", self.head, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
             "--comparison-oid", self.comparison_oid, "--out", str(alternate_out)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        retry = json.loads(result.stdout)
        self.assertTrue(retry["reused"])
        self.assertEqual(first["epoch"], retry["epoch"])
        self.assertEqual(first["expected_slices"], retry["expected_slices"])
        self.assertEqual(first["batch_path"], retry["batch_path"])

    def test_preflight_only_materializes_incomplete_artifacts_without_collection(self) -> None:
        artifacts = self.root / "preflight"
        plan = self.plan("--preflight-dir", str(artifacts))
        self.assertEqual("incomplete", plan["preflight"]["status"])
        persisted = json.loads(self.out.read_text(encoding="utf-8"))
        self.assertEqual(plan["preflight"]["ledger_path"], persisted["preflight"]["ledger_path"])
        self.assertEqual(plan["epoch"], persisted["epoch"])
        self.assertFalse(Path(plan["collection_path"]).exists())
        for artifact in plan["preflight"]["artifact_paths"]:
            returned = json.loads(Path(artifact).read_text(encoding="utf-8"))
            self.assertEqual("incomplete", returned["status"])
            self.assertEqual(plan["epoch"], returned["epoch"])
            self.assertNotEqual("passed", returned["disposition"])

    def test_preflight_fails_closed_when_an_existing_ledger_is_tampered(self) -> None:
        artifacts = self.root / "preflight-tampered-ledger"
        plan = self.plan("--preflight-dir", str(artifacts))
        ledger = Path(plan["preflight"]["ledger_path"])
        ledger.write_text("", encoding="utf-8")
        for artifact in plan["preflight"]["artifact_paths"]:
            returned = json.loads(Path(artifact).read_text(encoding="utf-8"))
            self.assertEqual("incomplete", returned["status"])

        result = self.run_plan("--preflight-dir", str(artifacts), ok=False)
        self.assertRegex(result.stderr.lower(), r"ledger|inconsistent")

    def test_head_evidence_or_role_drift_never_reuses_a_previous_plan(self) -> None:
        first = self.plan()
        self.git("commit", "--allow-empty", "-m", "head drift")
        drifted_head = self.git("rev-parse", "HEAD")
        variations = (
            ("--head", drifted_head),
            ("--evidence-digest", "d" * 64),
            ("--change-class", "domain-semantic-doc", "--domain-role", "runtime_engineer"),
        )
        for index, variation in enumerate(variations):
            with self.subTest(variation=variation):
                drifted_out = self.root / f"drift-{index}.json"
                args = list(variation) + ["--out", str(drifted_out)]
                result = subprocess.run(
                    [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
                     "--head", self.head, "--evidence-digest", EVIDENCE,
                     "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
                     "--comparison-oid", self.comparison_oid, *args],
                    text=True,
                    capture_output=True,
                )
                self.assertEqual(0, result.returncode, result.stderr)
                drifted = json.loads(result.stdout)
                self.assertFalse(drifted["reused"])
                self.assertNotEqual(first["epoch"], drifted["epoch"])

    def test_comparison_ref_and_resolved_oid_are_immutable_plan_identity(self) -> None:
        first = self.plan()
        self.assertEqual(self.comparison_ref, first["comparison_ref"])
        self.assertEqual(self.comparison_oid, first["comparison_oid"])

        (self.root / "comparison-drift").write_text("drift\n", encoding="utf-8")
        self.git("add", "comparison-drift")
        self.git("commit", "-m", "comparison drift")
        self.comparison_oid = self.git("rev-parse", "HEAD")
        self.head = self.comparison_oid
        self.git("update-ref", self.comparison_ref, self.comparison_oid)
        drifted = self.plan("--out", str(self.root / "comparison-oid-drift.json"))
        self.assertFalse(drifted["reused"])
        self.assertNotEqual(first["epoch"], drifted["epoch"])

    def test_shorthand_remote_comparison_ref_is_canonicalized_for_pr_helpers(self) -> None:
        shorthand = "origin/main"
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", self.head, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--comparison-ref", shorthand,
             "--comparison-oid", self.comparison_oid,
             "--out", str(self.root / "shorthand-comparison-plan.json")],
            text=True,
            capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual("refs/remotes/origin/main", plan["comparison_ref"])

    def test_divergent_shorthand_and_remote_refs_fail_closed(self) -> None:
        self.git("checkout", "-b", "origin/main")
        (self.root / "README").write_text("local shorthand\n", encoding="utf-8")
        self.git("commit", "-am", "local shorthand ref")
        self.git("checkout", "-b", "topic")
        (self.root / "README").write_text("topic\n", encoding="utf-8")
        self.git("commit", "-am", "topic head")
        self.head = self.git("rev-parse", "HEAD")

        out = self.root / "divergent-shorthand-plan.json"
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", self.head, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--comparison-ref", "origin/main",
             "--out", str(out)],
            text=True,
            capture_output=True,
        )

        self.assertNotEqual(0, result.returncode)
        self.assertIn("comparison ref", result.stderr.lower())
        self.assertFalse(out.exists())

    def test_divergent_comparison_fails_before_plan_creation(self) -> None:
        self.git("checkout", "-b", "topic")
        (self.root / "README").write_text("topic\n", encoding="utf-8")
        self.git("commit", "-am", "topic")
        self.head = self.git("rev-parse", "HEAD")
        self.git("checkout", "main")
        (self.root / "README").write_text("advanced base\n", encoding="utf-8")
        self.git("commit", "-am", "advance base")
        self.git("update-ref", self.comparison_ref, "HEAD")
        self.comparison_oid = self.git("rev-parse", self.comparison_ref)

        result = self.run_plan(ok=False)

        self.assertIn("ancestor", result.stderr.lower())
        self.assertFalse(self.out.exists())

    def test_ci_receipt_keeps_immutable_base_after_symbolic_ref_moves(self) -> None:
        receipt_base = self.comparison_oid
        self.git("checkout", "-b", "topic")
        self.git("commit", "--allow-empty", "-m", "implementation")
        self.head = self.git("rev-parse", "HEAD")
        moved_ref = self.git("commit-tree", "HEAD^^{tree}", "-p", receipt_base, "-m", "moved base")
        self.git("update-ref", self.comparison_ref, moved_ref)
        receipt = self.root / "immutable-base-receipt.json"
        self.write_receipt(receipt, base_oid=receipt_base, head_oid=self.head)

        plan = self.receipt_plan(receipt, self.root / "immutable-base-plan.json")

        self.assertEqual(receipt_base, plan["comparison_oid"])
        self.assertEqual(self.head, plan["frozen_head"])

    def test_divergent_integration_receipt_records_ancestor_scope(self) -> None:
        self.git("commit", "--allow-empty", "-m", "task source")
        self.head=self.git("rev-parse","HEAD")
        integration = self.git("commit-tree", self.comparison_oid + "^{tree}", "-p", self.comparison_oid, "-m", "parallel main")
        receipt = self.root / "parallel-receipt.json"
        self.write_receipt(receipt, base_oid=integration, head_oid=self.head)
        value=json.loads(receipt.read_text())
        value.update(scope_base_oid=self.comparison_oid,integration_base_oid=integration)
        receipt.write_text(json.dumps(value))
        plan=self.receipt_plan(receipt,self.root / "parallel-plan.json")
        self.assertEqual(self.comparison_oid,plan["comparison_oid"])
        self.assertEqual(integration,plan["integration_base_oid"])

    def test_ci_receipt_head_must_match_frozen_head(self) -> None:
        receipt = self.root / "wrong-head-receipt.json"
        self.write_receipt(receipt, base_oid=self.comparison_oid, head_oid="c" * 40)
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK, "--head", self.head,
             "--ci-ready-receipt", str(receipt), "--change-class", "workflow-doc",
             "--comparison-ref", self.comparison_ref, "--out", str(self.out)],
            text=True, capture_output=True,
        )
        self.assertNotEqual(0, result.returncode)
        self.assertIn("head", result.stderr.lower())
        self.assertFalse(self.out.exists())

    def test_rejects_a_caller_supplied_oid_that_does_not_match_the_real_ref(self) -> None:
        result = self.run_plan("--comparison-oid", "c" * 40, ok=False)
        self.assertIn("--comparison-oid mismatch", result.stderr)

    def test_manual_unknown_plan_binds_roles_comparison_and_packet_refs(self) -> None:
        plan = self.plan(
            "--change-class", "unknown",
            "--manual-role", "runtime_engineer",
            "--manual-role", "qa_engineer",
            "--manual-role", "repository_health_engineer",
            "--out", str(self.root / "manual-plan.json"),
        )
        self.assertEqual(
            ["runtime_engineer", "qa_engineer", "repository_health_engineer"],
            plan["roles"],
        )
        self.assertEqual(self.comparison_ref, plan["comparison_ref"])
        self.assertEqual(self.comparison_oid, plan["comparison_oid"])
        self.assertEqual(plan["roles"], [item["role"] for item in plan["expected_slices"]])
        self.assertEqual(
            [item["slice_id"] for item in plan["expected_slices"]],
            [item["slice_id"] for item in plan["packet_refs"]],
        )
        self.assertEqual(
            [
                f".pm/scratch/{TASK}/slice-packets/{item['slice_id']}.json"
                for item in plan["expected_slices"]
            ],
            [item["packet_ref"] for item in plan["packet_refs"]],
        )

    def test_manual_roles_are_part_of_the_immutable_plan_identity(self) -> None:
        result = self.run_plan(
            "--change-class", "mixed",
            "--manual-role", "runtime_engineer",
            "--manual-role", "runtime_engineer",
            ok=False,
        )
        self.assertIn("duplicate manual role", result.stderr.lower())

    def test_prior_review_context_binds_real_prior_head_delta(self) -> None:
        prior_path = self.root / ".pm/scratch" / TASK / "review-plans" / "prior.json"
        prior = self.plan(
            "--out", str(prior_path),
            "--preflight-dir", str(self.root / ".pm/scratch" / TASK / "prior-preflight"),
        )
        self.complete_collected_plan(prior)
        prior_bytes = prior_path.read_bytes()

        (self.root / "repair.txt").write_text("repair\n", encoding="utf-8")
        self.git("add", "repair.txt")
        self.git("commit", "-m", "repair")
        self.head = self.git("rev-parse", "HEAD")
        current_path = self.root / ".pm/scratch" / TASK / "review-plans" / "current.json"

        current = self.plan(
            "--prior-review-plan", str(prior_path),
            "--out", str(current_path),
        )
        context = current["incremental_review_context"]
        self.assertEqual("oasis7-review-context/v1", context["schema"])
        self.assertEqual(prior["frozen_head"], context["prior_head_oid"])
        self.assertEqual(self.head, context["current_head_oid"])
        self.assertEqual(["repair.txt"], context["delta_paths"])
        self.assertEqual(
            hashlib.sha256(prior_bytes).hexdigest(),
            context["prior_plan_digest"],
        )
        self.assertNotEqual(prior["epoch"], current["epoch"])

    def test_prior_review_context_rejects_tampered_prior_identity(self) -> None:
        prior_path = self.root / ".pm/scratch" / TASK / "review-plans" / "tampered.json"
        prior = self.plan(
            "--out", str(prior_path),
            "--preflight-dir", str(self.root / ".pm/scratch" / TASK / "prior-preflight"),
        )
        self.complete_collected_plan(prior)
        tampered = json.loads(prior_path.read_text(encoding="utf-8"))
        tampered["relevant_evidence_digest"] = "c" * 64
        prior_path.write_text(json.dumps(tampered), encoding="utf-8")

        (self.root / "repair.txt").write_text("repair\n", encoding="utf-8")
        self.git("add", "repair.txt")
        self.git("commit", "-m", "repair")
        self.head = self.git("rev-parse", "HEAD")
        result = self.run_plan(
            "--prior-review-plan", str(prior_path),
            "--out", str(self.root / ".pm/scratch" / TASK / "review-plans" / "current.json"),
            ok=False,
        )
        self.assertRegex(result.stderr.lower(), r"prior.*digest|prior.*identity|prior.*plan")


if __name__ == "__main__":
    unittest.main()
