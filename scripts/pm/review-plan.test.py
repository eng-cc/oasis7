#!/usr/bin/env python3
"""Behavior contract for deterministic, reusable review planning."""
from __future__ import annotations

import io
import json
import hashlib
import importlib.util
import os
from contextlib import redirect_stdout
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("review-plan.py")
_SPEC = importlib.util.spec_from_file_location("review_plan_under_test", SCRIPT)
assert _SPEC is not None and _SPEC.loader is not None
REVIEW_PLAN = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(REVIEW_PLAN)
_PROJECTION_SPEC = importlib.util.spec_from_file_location(
    "workflow_impact_projection_under_test", Path(__file__).with_name("workflow-impact-projection.py")
)
assert _PROJECTION_SPEC and _PROJECTION_SPEC.loader
WORKFLOW_IMPACT = importlib.util.module_from_spec(_PROJECTION_SPEC)
_PROJECTION_SPEC.loader.exec_module(WORKFLOW_IMPACT)
TASK = "task_" + "1" * 32
EVIDENCE = "b" * 64
COMPARISON_REF = "refs/remotes/origin/main"


class ReviewPlanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        mapping = self.root / '.pm/github-project-sync'
        mapping.mkdir(parents=True)
        (mapping / 'tasks.json').write_text(json.dumps({'tasks': {TASK: {'task_uid': TASK, 'repository': 'fixture/repo', 'issue_number': 1, 'pr_number': 2, 'bootstrap_epoch': 1}}}))
        for role in ("repository_health_engineer", "qa_engineer"):
            role_path = self.root / ".agents/roles" / f"{role}.md"
            role_path.parent.mkdir(parents=True, exist_ok=True)
            role_path.write_text(f"# {role}\n")
        policy = self.root / "doc/engineering/workflow/source-of-truth.md"
        policy.parent.mkdir(parents=True, exist_ok=True)
        policy.write_text("# workflow policy\n")
        skill = self.root / ".agents/skills/requesting-repo-owned-review/SKILL.md"
        skill.parent.mkdir(parents=True, exist_ok=True)
        skill.write_text("# review skill\n")
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
             "--review-schema", REVIEW_PLAN.SCHEMA,
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
             "--review-schema", REVIEW_PLAN.SCHEMA,
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

    def write_impact_projection(self, path: Path) -> str:
        changed_paths = [
            line for line in self.git(
                "diff", "--name-only", "--no-renames", self.comparison_oid, self.head
            ).splitlines() if line
        ]
        projection = WORKFLOW_IMPACT.build_projection(self.root, {
            "task_uid": TASK, "source_head_oid": self.head,
            "scope_base_oid": self.comparison_oid, "changed_paths": changed_paths,
            "change_class": "workflow-doc", "manual_roles": [], "domain_role": None,
            "test_profile": "required", "declared_tests": ["required_gate_baseline"],
            "consumed_contracts": ["workflow-contract"], "public_semantics": [],
            "affected_consumers": ["required-ci"],
            "closure_status": {"status": "complete", "reason": "fixture", "evidence": [{
                "path": "README",
                "sha256": "sha256:" + hashlib.sha256((self.root / "README").read_bytes()).hexdigest(),
            }]},
        })
        path.write_text(json.dumps(projection), encoding="utf-8")
        return str(projection["changed_paths_digest"]).removeprefix("sha256:")

    def rewrite_projection_paths(self, path: Path, changed_paths: list[str]) -> None:
        projection = json.loads(path.read_text(encoding="utf-8"))
        projection["changed_paths"] = sorted(changed_paths)
        projection["changed_paths_digest"] = WORKFLOW_IMPACT.canonical_digest(
            projection["changed_paths"]
        )
        projection.pop("projection_digest", None)
        projection["projection_digest"] = WORKFLOW_IMPACT.canonical_digest(projection)
        path.write_text(json.dumps(projection), encoding="utf-8")

    def commit_source_change(self, relative_path: str, content: str = "change\n") -> None:
        target = self.root / relative_path
        target.write_text(content, encoding="utf-8")
        self.git("add", relative_path)
        self.git("commit", "-m", f"change {relative_path}")
        self.head = self.git("rev-parse", "HEAD")

    def write_source_review_input(self, path: Path, *, changed_paths_digest: str | None = None) -> Path:
        projection_path = path.with_name(path.stem + "-impact.json")
        projected_digest = self.write_impact_projection(projection_path)
        projection_digest = json.loads(projection_path.read_text())["projection_digest"]
        changed_paths_digest = changed_paths_digest or projected_digest
        path.write_text(json.dumps({
            "schema": "oasis7-review-source-input/v1",
            "task_uid": TASK,
            "bootstrap_epoch": 1,
            "repository": "example/repo",
            "pr_number": 2,
            "source_head_oid": self.head,
            "source_scope_oid": self.comparison_oid,
            "changed_paths_digest": changed_paths_digest,
            "ordered_role_ids": ["repository_health_engineer", "qa_engineer"],
            "role_contract_digest": "1" * 64,
            "review_policy_digest": "2" * 64,
            "input_contract_digest": projection_digest.removeprefix("sha256:"),
        }), encoding="utf-8")
        return projection_path

    def source_plan(self, *, out: Path | None = None, input_path: Path | None = None,
                    ok: bool = True) -> dict[str, object] | subprocess.CompletedProcess[str]:
        input_path = input_path or (self.root / "source-review-input.json")
        if not input_path.exists():
            projection_path = self.write_source_review_input(input_path)
        else:
            projection_path = input_path.with_name(input_path.stem + "-impact.json")
        command = [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
                   "--head", self.head, "--review-schema", REVIEW_PLAN.V2_SCHEMA,
                   "--source-review-input", str(input_path),
                   "--impact-projection", str(projection_path),
                   "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
                   "--comparison-oid", self.comparison_oid,
                   "--out", str(out or (self.root / "v2-source-plan.json"))]
        result = subprocess.run(command, text=True, capture_output=True)
        if not ok:
            if result.returncode == 0:
                self.fail(f"source plan unexpectedly passed: {result.stdout}")
            return result
        self.assertEqual(0, result.returncode, result.stderr)
        return json.loads(result.stdout)

    def test_default_entry_creates_v2_source_plan_before_ci(self) -> None:
        input_path = self.root / "default-source-input.json"
        projection_path = self.write_source_review_input(input_path)
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", self.head, "--source-review-input", str(input_path),
             "--impact-projection", str(projection_path),
             "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
             "--comparison-oid", self.comparison_oid,
             "--out", str(self.root / "default-v2-plan.json")],
            text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual(REVIEW_PLAN.V2_SCHEMA, plan["schema"])
        self.assertEqual(REVIEW_PLAN.V2_SCHEMA, plan["effective_mode"]["review_schema"])
        self.assertEqual("separated", plan["effective_mode"]["source_review_mode"])
        self.assertIsNone(plan.get("integration_ci_identity"))
        self.assertEqual("pending", plan["integration_ci_status"])

    def test_explicit_source_input_must_bind_projection_digest(self) -> None:
        input_path = self.root / "mismatched-source-input.json"
        projection_path = self.write_source_review_input(input_path)
        source = json.loads(input_path.read_text())
        source["input_contract_digest"] = "3" * 64
        input_path.write_text(json.dumps(source))
        result = subprocess.run([
            str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
            "--head", self.head, "--source-review-input", str(input_path),
            "--impact-projection", str(projection_path),
            "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
            "--comparison-oid", self.comparison_oid,
            "--out", str(self.root / "mismatched-source-plan.json"),
        ], text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("impact projection input contract", result.stderr)

    def test_standard_entry_derives_source_identity_from_verified_projection(self) -> None:
        projection_path = self.root / "standard-impact.json"
        self.write_impact_projection(projection_path)
        result = subprocess.run([
            str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
            "--head", self.head, "--impact-projection", str(projection_path),
            "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
            "--comparison-oid", self.comparison_oid,
            "--out", str(self.root / "standard-v2-plan.json"),
        ], text=True, capture_output=True)
        self.assertEqual(0, result.returncode, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual("oasis7-review-plan/v2", plan["schema"])
        self.assertEqual("pending", plan["integration_ci_status"])
        self.assertEqual(json.loads(projection_path.read_text())["projection_digest"],
                         plan["impact_projection_digest"])

    def test_source_only_v2_rejects_incomplete_frozen_scope_projection(self) -> None:
        self.commit_source_change("repair.md")
        projection_path = self.root / "incomplete-impact.json"
        self.write_impact_projection(projection_path)
        self.rewrite_projection_paths(projection_path, [])
        result = subprocess.run([
            str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
            "--head", self.head, "--impact-projection", str(projection_path),
            "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
            "--comparison-oid", self.comparison_oid,
            "--out", str(self.root / "incomplete-v2-plan.json"),
        ], text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("frozen base..head", result.stderr)
        self.assertIn("missing=repair.md", result.stderr)

    def test_source_only_v2_rejects_extra_projection_path(self) -> None:
        self.commit_source_change("repair.md")
        projection_path = self.root / "extra-impact.json"
        self.write_impact_projection(projection_path)
        self.rewrite_projection_paths(projection_path, ["repair.md", "phantom.md"])
        result = subprocess.run([
            str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
            "--head", self.head, "--impact-projection", str(projection_path),
            "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
            "--comparison-oid", self.comparison_oid,
            "--out", str(self.root / "extra-v2-plan.json"),
        ], text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("frozen base..head", result.stderr)
        self.assertIn("extra=phantom.md", result.stderr)

    def test_default_v2_entry_rejects_legacy_evidence_digest(self) -> None:
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", self.head, "--evidence-digest", EVIDENCE,
             "--change-class", "workflow-doc", "--comparison-ref", self.comparison_ref,
             "--comparison-oid", self.comparison_oid,
             "--out", str(self.root / "legacy-default-plan.json")],
            text=True, capture_output=True,
        )
        self.assertNotEqual(0, result.returncode)
        self.assertRegex(result.stderr, r"source-review-input|legacy.*v2|CI receipt")

    def test_v2_source_plan_reuses_only_its_immutable_source_identity(self) -> None:
        first = self.source_plan()
        retry = self.source_plan()
        self.assertTrue(retry["reused"])
        self.assertEqual(first["epoch"], retry["epoch"])
        self.assertNotIn("integration_ci_identity", first)

    def test_v2_identity_without_strict_integration_is_pending(self) -> None:
        source_plan = self.source_plan()
        identity = REVIEW_PLAN.plan_identity_v2(
            TASK, self.head, self.comparison_ref, self.comparison_oid,
            source_plan["roles"], source_plan["expected_slices"],
            source_plan["source_review_identity"], source_plan["source_review_digest"],
            None, None, source_plan["professional_review_applicability"],
            source_plan["impact_projection"],
        )
        self.assertEqual("pending", identity["integration_ci_status"])
        self.assertNotIn("integration_ci_identity", identity)

    def test_v2_ordinary_receipt_builds_source_plan_without_integration_identity(self) -> None:
        source_plan = self.source_plan()
        source = source_plan["source_review_identity"]
        projection = source_plan["impact_projection"]
        receipt = {
            "receipt_type": "oasis7_ci_ready_receipt",
            "issuer": "github_live_query",
            "repository": "example/repo",
            "task_uid": TASK,
            "task_issue_number": 1,
            "pr_number": 2,
            "base_oid": self.comparison_oid,
            "head_oid": self.head,
            "base_ref": "main",
            "check_name": "required-gate",
            "check_app_id": 42,
            "check_run_id": 7,
            "planner_digest": "c" * 64,
            "planner_config_sha256": "sha256:" + "d" * 64,
            "run_rust_baseline": True,
            "conclusion": "success",
            "ci_validation_mode": "ordinary_pr",
            "live_validation": "ci-ready-receipt-live",
            "impact_projection_schema": projection["schema"],
            "impact_projection_digest": projection["projection_digest"],
            "impact_projection_planner_digest": projection["planner_digest"],
            "scope_base_oid": self.comparison_oid,
            "integration_base_oid": self.comparison_oid,
        }
        receipt_path = self.root / "ordinary-receipt.json"
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        out = self.root / "ordinary-receipt-plan.json"
        argv = [
            str(SCRIPT), "--root", str(self.root), "--task-uid", TASK,
            "--head", self.head, "--ci-ready-receipt", str(receipt_path),
            "--impact-projection", str(self.root / "source-review-input-impact.json"),
            "--review-schema", REVIEW_PLAN.V2_SCHEMA, "--change-class", "workflow-doc",
            "--comparison-ref", self.comparison_ref, "--comparison-oid", self.comparison_oid,
            "--bootstrap-epoch", "1",
            "--role-contract-digest", source["role_contract_digest"],
            "--review-policy-digest", source["review_policy_digest"],
            "--input-contract-digest", source["input_contract_digest"],
            "--changed-paths-digest", source["changed_paths_digest"], "--out", str(out),
        ]
        with patch.object(REVIEW_PLAN, "live_verify_v2_receipt", return_value=receipt), \
             patch.object(REVIEW_PLAN.sys, "argv", argv), redirect_stdout(io.StringIO()):
            self.assertEqual(0, REVIEW_PLAN.main())
        written = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual("pending", written["integration_ci_status"])
        self.assertEqual("ordinary_pr", written["ci_validation_mode"])
        self.assertNotIn("integration_ci_identity", written)

    def test_v2_live_receipt_refresh_forwards_bound_target_ref(self) -> None:
        receipt = {
            "repository": "example/repo", "task_uid": TASK, "task_issue_number": 1,
            "pr_number": 2, "check_name": "required-gate", "check_app_id": 42,
            "planner_digest": "c" * 64, "base_ref": "release",
        }
        path = self.root / "bound-receipt.json"
        path.write_text(json.dumps(receipt), encoding="utf-8")
        completed = subprocess.CompletedProcess([], 0, stdout=json.dumps(receipt), stderr="")
        with patch.object(REVIEW_PLAN.subprocess, "run", return_value=completed) as run:
            REVIEW_PLAN.live_verify_v2_receipt(path, receipt)
        command = run.call_args.args[0]
        self.assertEqual("release", command[command.index("--base-ref") + 1])

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

    def test_standard_v1_plan_emits_machine_readable_effective_mode(self) -> None:
        mode = self.plan()["effective_mode"]
        self.assertEqual(
            {
                "effective_policy": "legacy",
                "review_schema": "oasis7-review-plan/v1",
                "source_review_mode": "combined",
                "integration_validation_mode": "legacy_evidence",
                "enabled_optimizations": [],
                "fallback_reason": "legacy_task_without_loop_binding",
            },
            mode,
        )

    def test_v1_ci_receipt_path_reports_receipt_bound_validation(self) -> None:
        receipt = self.root / "v1-effective-mode-receipt.json"
        self.write_receipt(receipt, base_oid=self.comparison_oid, head_oid=self.head)
        mode = self.receipt_plan(receipt, self.root / "v1-effective-mode-plan.json")["effective_mode"]
        self.assertEqual("oasis7-review-plan/v1", mode["review_schema"])
        self.assertEqual("combined", mode["source_review_mode"])
        self.assertEqual("receipt_bound", mode["integration_validation_mode"])
        self.assertEqual("legacy_task_without_loop_binding", mode["fallback_reason"])

    def test_bound_v1_mode_reports_compatibility_fallback(self) -> None:
        mode = REVIEW_PLAN.effective_mode(
            review_schema=REVIEW_PLAN.SCHEMA,
            loop_status="passed",
            has_ci_ready_receipt=False,
            trusted_integration_artifact=False,
        )
        self.assertEqual("loop-bound", mode["effective_policy"])
        self.assertEqual("oasis7-review-plan/v1", mode["review_schema"])
        self.assertEqual("combined", mode["source_review_mode"])
        self.assertEqual("legacy_evidence", mode["integration_validation_mode"])
        self.assertEqual([], mode["enabled_optimizations"])
        self.assertEqual("review_schema_v1_compatibility", mode["fallback_reason"])

    def test_v2_mode_reports_split_identity_optimization(self) -> None:
        mode = REVIEW_PLAN.effective_mode(
            review_schema=REVIEW_PLAN.V2_SCHEMA,
            loop_status="passed",
            has_ci_ready_receipt=True,
            trusted_integration_artifact=True,
        )
        self.assertEqual(
            {
                "effective_policy": "loop-bound",
                "review_schema": "oasis7-review-plan/v2",
                "source_review_mode": "separated",
                "integration_validation_mode": "trusted_integration",
                "enabled_optimizations": ["source_review_integration_separation"],
                "fallback_reason": None,
            },
            mode,
        )

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
             "--review-schema", REVIEW_PLAN.SCHEMA,
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
                     "--review-schema", REVIEW_PLAN.SCHEMA,
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
             "--review-schema", REVIEW_PLAN.SCHEMA,
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
             "--review-schema", REVIEW_PLAN.SCHEMA,
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
             "--review-schema", REVIEW_PLAN.SCHEMA,
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

    def test_prior_review_context_unknown_impact_escalates_to_full_review(self) -> None:
        prior_path = self.root / ".pm/scratch" / TASK / "review-plans" / "unknown-impact-prior.json"
        prior = self.plan(
            "--out", str(prior_path),
            "--preflight-dir", str(self.root / ".pm/scratch" / TASK / "unknown-impact-preflight"),
        )
        self.complete_collected_plan(prior)

        (self.root / "unknown-impact.bin").write_bytes(b"unclassified change\n")
        self.git("add", "unknown-impact.bin")
        self.git("commit", "-m", "unknown impact")
        self.head = self.git("rev-parse", "HEAD")
        current = self.plan(
            "--prior-review-plan", str(prior_path),
            "--out", str(self.root / ".pm/scratch" / TASK / "review-plans" / "unknown-impact-current.json"),
        )

        context = current["incremental_review_context"]
        self.assertEqual("full", context["review_scope"])
        self.assertEqual(["unknown_impact"], context["escalation_reasons"])

    def test_prior_review_context_binds_full_and_impact_confirmation_obligations(self) -> None:
        prior_path = self.root / ".pm/scratch" / TASK / "review-plans" / "scoped-prior.json"
        prior = self.plan(
            "--out", str(prior_path),
            "--preflight-dir", str(self.root / ".pm/scratch" / TASK / "scoped-preflight"),
        )
        self.complete_collected_plan(prior)

        (self.root / "repair.txt").write_text("repair\n", encoding="utf-8")
        self.git("add", "repair.txt")
        self.git("commit", "-m", "scoped repair")
        self.head = self.git("rev-parse", "HEAD")
        current = self.plan(
            "--prior-review-plan", str(prior_path),
            "--impacted-role", "repository_health_engineer",
            "--out", str(self.root / ".pm/scratch" / TASK / "review-plans" / "scoped-current.json"),
        )

        context = current["incremental_review_context"]
        expected_modes = {
            "qa_engineer": "impact_confirmation",
            "repository_health_engineer": "full_review",
        }
        self.assertEqual("scoped", context["review_scope"])
        self.assertEqual(expected_modes, context["role_review_modes"])
        self.assertEqual(
            REVIEW_PLAN.digest(expected_modes),
            context["role_review_modes_digest"],
        )

    def test_prior_review_context_rejects_deleted_collected_artifact(self) -> None:
        prior_path = self.root / ".pm/scratch" / TASK / "review-plans" / "deleted-artifact.json"
        prior = self.plan(
            "--out", str(prior_path),
            "--preflight-dir", str(self.root / ".pm/scratch" / TASK / "prior-preflight"),
        )
        self.complete_collected_plan(prior)
        ledger_path = Path(str(prior["preflight"]["ledger_path"]))
        row = json.loads(ledger_path.read_text(encoding="utf-8").splitlines()[0])
        artifact_path = Path(str(row["artifacts"][0]))
        artifact_path.write_text(artifact_path.read_text(encoding="utf-8") + "tampered\n", encoding="utf-8")

        (self.root / "repair.txt").write_text("repair\n", encoding="utf-8")
        self.git("add", "repair.txt")
        self.git("commit", "-m", "repair")
        self.head = self.git("rev-parse", "HEAD")
        result = self.run_plan(
            "--prior-review-plan", str(prior_path),
            "--out", str(self.root / ".pm/scratch" / TASK / "review-plans" / "current.json"),
            ok=False,
        )
        self.assertRegex(result.stderr.lower(), r"artifact|ledger")

        artifact_path.unlink()
        deleted = self.run_plan(
            "--prior-review-plan", str(prior_path),
            "--out", str(self.root / ".pm/scratch" / TASK / "review-plans" / "deleted.json"),
            ok=False,
        )
        self.assertRegex(deleted.stderr.lower(), r"artifact|ledger")

    def test_prior_review_context_rejects_artifact_path_escape(self) -> None:
        prior_path = self.root / ".pm/scratch" / TASK / "review-plans" / "escaped-artifact.json"
        prior = self.plan(
            "--out", str(prior_path),
            "--preflight-dir", str(self.root / ".pm/scratch" / TASK / "escaped-preflight"),
        )
        self.complete_collected_plan(prior)
        ledger_path = Path(str(prior["preflight"]["ledger_path"]))
        rows = [json.loads(line) for line in ledger_path.read_text(encoding="utf-8").splitlines() if line]
        original_artifact = Path(str(rows[0]["artifacts"][0]))
        outside_artifact = self.root.parent / f"{self.root.name}-outside-artifact.json"
        outside_artifact.write_bytes(original_artifact.read_bytes())
        self.addCleanup(lambda: outside_artifact.unlink(missing_ok=True))

        (self.root / "repair.txt").write_text("repair\n", encoding="utf-8")
        self.git("add", "repair.txt")
        self.git("commit", "-m", "repair")
        self.head = self.git("rev-parse", "HEAD")

        collection_path = Path(str(prior["collection_path"]))
        for index, reference in enumerate((
            str(outside_artifact),
            os.path.relpath(outside_artifact, ledger_path.parent),
        )):
            rows[0]["artifacts"] = [reference]
            ledger_path.write_text(
                "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows),
                encoding="utf-8",
            )
            collection = json.loads(collection_path.read_text(encoding="utf-8"))
            collection["ledger_digest"] = hashlib.sha256(ledger_path.read_bytes()).hexdigest()
            collection_path.write_text(json.dumps(collection), encoding="utf-8")
            result = self.run_plan(
                "--prior-review-plan", str(prior_path),
                "--out", str(self.root / ".pm/scratch" / TASK / "review-plans" / f"escape-{index}.json"),
                ok=False,
            )
            self.assertRegex(result.stderr.lower(), r"artifact|repository|escape|ledger")

    def test_binary_diff_digest_ignores_external_diff_and_textconv(self) -> None:
        prior_head = self.head
        (self.root / "repair.txt").write_text("repair\n", encoding="utf-8")
        self.git("add", "repair.txt")
        self.git("commit", "-m", "repair")
        current_head = self.git("rev-parse", "HEAD")
        external = self.root / "external-diff"
        external.write_text("#!/bin/sh\nprintf 'external diff output\\n'\n", encoding="utf-8")
        external.chmod(0o755)
        baseline = REVIEW_PLAN.binary_diff_digest(self.root, prior_head, current_head)
        self.git("config", "diff.external", str(external))
        with patch.dict(os.environ, {"GIT_EXTERNAL_DIFF": str(external)}):
            self.assertEqual(
                baseline,
                REVIEW_PLAN.binary_diff_digest(self.root, prior_head, current_head),
            )

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
