#!/usr/bin/env python3
"""Prove v2 task closeout live-validates every receipt before reuse."""

import importlib.util
import datetime as dt
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from ci_reuse_validation_test_support import validation_only_artifacts


PM = Path(__file__).parent
ROOT = PM.parent.parent
UID = "task_12345678901234567890123456789012"


def load_identity_module(path: Path):
    spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_fixture", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class TaskCloseoutV2LiveValidationTest(unittest.TestCase):
    def test_receipt_refresh_does_not_expand_an_empty_array_under_nounset(self):
        source = (PM / "task-closeout.sh").read_text(encoding="utf-8")
        self.assertNotIn('${CI_RECEIPT_ARGS[@]}', source)

    def test_v2_reuse_checks_live_target_oid(self):
        source = (PM / "task-closeout.sh").read_text(encoding="utf-8")
        self.assertIn("--json baseRefOid --jq '.baseRefOid'", source)
        self.assertIn('--no-write-fetch-head', source)
        self.assertIn('current_target_oid=current_target_oid, current_target_root=root', source)

    def test_nested_traceability_record_cannot_hide_aggregate_mode(self):
        source = (PM / "task-closeout.sh").read_text(encoding="utf-8")
        self.assertIn("def record_declares_aggregate(value):", source)
        self.assertIn("or record_declares_aggregate(declared_record)", source)
        self.assertIn("or record_declares_aggregate(record)", source)
        self.assertIn("or declared_candidate is not None", source)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "fixture"
        (self.root / "scripts/pm").mkdir(parents=True)
        (self.root / ".pm/github-project-sync").mkdir(parents=True)
        shutil.copy2(PM / "task-closeout.sh", self.root / "scripts/pm/task-closeout.sh")
        shutil.copy2(PM / "ci_ready_receipt_identity.py", self.root / "scripts/pm/ci_ready_receipt_identity.py")
        shutil.copy2(PM / "workflow-impact-projection.py", self.root / "scripts/pm/workflow-impact-projection.py")
        shutil.copy2(PM / "bootstrap-task-snapshot.py", self.root / "scripts/pm/bootstrap-task-snapshot.py")
        shutil.copy2(PM / "workflow-durable-store.py", self.root / "scripts/pm/workflow-durable-store.py")
        shutil.copy2(PM / "closed_duplicate_candidate_guard.py", self.root / "scripts/pm/closed_duplicate_candidate_guard.py")
        subprocess.run(["git", "-C", str(self.root), "init", "-q", "-b", "main"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.email", "fixture@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.name", "Fixture"], check=True)
        (self.root / "tracked").write_text("fixture\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.root), "add", "tracked"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "fixture"], check=True)
        self.head = subprocess.check_output(["git", "-C", str(self.root), "rev-parse", "HEAD"], text=True).strip()
        mapping = {
            "version": 1,
            "project": {"owner": "eng-cc", "number": 1},
            "tasks": {
                UID: {
                    "task_uid": UID,
                    "title": "fixture request",
                    "issue_number": 1,
                    "issue_url": "https://example.invalid/1",
                    "project_item_id": "PVTI_fixture",
                    "status": "ready",
                    "owner_role": "tpm",
                    "repository": "eng-cc/oasis7",
                    "canonical_worktree": str(self.root.resolve()),
                    "task_branch": "main",
                    "default_branch": "main",
                    "acceptance": ["fixture closeout"],
                    "bootstrap_epoch": 1,
                }
            },
        }
        (self.root / ".pm/github-project-sync/tasks.json").write_text(
            json.dumps(mapping) + "\n", encoding="utf-8"
        )
        subprocess.run(
            [
                sys.executable,
                str(self.root / "scripts/pm/bootstrap-task-snapshot.py"),
                "create",
                "--repo-root",
                str(self.root),
                "--task-uid",
                UID,
                "--request-identity",
                "fixture request",
                "--producer",
                "tpm",
            ],
            check=True,
            text=True,
            capture_output=True,
        )
        self.events = self.root / "events.log"
        self._write_helpers()
        self.plan, self.receipt = self._write_v2_inputs()
        self.packet = self.root / "review.md"
        self.packet.write_text(
            "\n".join(
                [
                    "Pre-PR Local Role Review: passed",
                    f"- Source Head: {self.head}",
                    "- Review Roles: qa_engineer",
                    "- Slice Ledger: ledger.jsonl",
                    "- Review Plan: plan.json",
                    "- Review Plan Schema: oasis7-review-plan/v2",
                    f"- Review Evidence Digest: {'a' * 64}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        (self.root / "ledger.jsonl").write_text("{}\n", encoding="utf-8")

    def tearDown(self):
        self.temp.cleanup()

    def _write_helpers(self):
        (self.root / "scripts/pm/validate-review-provenance.py").write_text(
            "#!/usr/bin/env python3\n", encoding="utf-8"
        )
        workflow = self.root / "scripts/pm/github-project-workflow.sh"
        workflow.write_text(
            f"#!/usr/bin/env bash\nprintf '%s\\n' '{{\"status\":\"ok\",\"selected_task\":{{\"task_uid\":\"{UID}\",\"target\":\"ready\",\"workflow_phase\":\"pre_pr_ready\"}}}}'\n",
            encoding="utf-8",
        )
        claim = self.root / "scripts/pm/claim-ready.sh"
        claim.write_text(
            "#!/usr/bin/env bash\necho claim >>\"${EVENTS}\"\nprintf '%s\\n' '{\"claim_type\":\"ready_for_pr\",\"status\":\"verified\",\"allowed_to_claim\":true,\"verification_exit_code\":0}'\n",
            encoding="utf-8",
        )
        transition = self.root / "scripts/pm/github-project-task.py"
        transition.write_text(
            f"#!/usr/bin/env python3\nfrom pathlib import Path\nimport os\nPath(os.environ['EVENTS']).write_text(Path(os.environ['EVENTS']).read_text() + 'transition\\n')\nprint('{{\"task_uid\":\"{UID}\",\"status\":\"ready\",\"issue_url\":\"https://example.invalid/1\"}}')\n",
            encoding="utf-8",
        )
        ci = self.root / "scripts/pm/ci-ready-receipt.py"
        ci.write_text(
            """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

args = sys.argv[1:]
Path(os.environ['CI_CALL_LOG']).write_text(Path(os.environ['CI_CALL_LOG']).read_text() + 'called\\n')
if '--integration-run-id' not in args:
    raise SystemExit('v2 closeout did not bind the current integration request/run')
receipt = json.loads(Path(args[args.index('--receipt') + 1]).read_text())
if os.environ['CI_MODE'] == 'reject':
    raise SystemExit('forged receipt rejected by live validator')
if os.environ['CI_MODE'] == 'validation-only':
    import ci_ready_receipt_identity
    try:
        ci_ready_receipt_identity.review_evidence_identity(receipt)
    except (TypeError, ValueError) as exc:
        raise SystemExit(f'validation-only evidence rejected: {exc}')
if os.environ['CI_MODE'] == 'changed-tree':
    receipt['tested_tree_oid'] = 'e' * 40
print(json.dumps(receipt))
""",
            encoding="utf-8",
        )
        for path in (workflow, claim, transition, ci):
            path.chmod(0o755)

    def _write_v2_inputs(self):
        identity = load_identity_module(self.root / "scripts/pm/ci_ready_receipt_identity.py")
        impact = load_identity_module(PM / "workflow-impact-projection.py")
        planner_identity = {
            "schema": "oasis7-required-plan-v1",
            "planner_config_sha256": "sha256:" + "9" * 64,
            "scope": "minimal",
            "selected_capabilities": ["required_gate_baseline"],
            "test_profile": "required",
            "declared_tests": ["required_gate_baseline"],
        }
        projection = {
            "schema": "oasis7-workflow-impact-projection/v2", "task_uid": UID,
            "source_head_oid": self.head, "scope_base_oid": "b" * 40,
            "changed_paths": ["tracked"], "changed_paths_digest": impact.canonical_digest(["tracked"]),
            "change_class": "workflow-doc", "manual_roles": [], "domain_role": None,
            "test_profile": "required", "declared_tests": ["required_gate_baseline"],
            "consumed_contracts": ["workflow"], "public_semantics": [],
            "affected_consumers": ["closeout"],
            "closure_status": {"status": "complete", "reason": "fixture", "evidence": [{
                "path": "tracked",
                "sha256": "sha256:" + __import__('hashlib').sha256((self.root / "tracked").read_bytes()).hexdigest(),
            }]},
            "ci_scope": "minimal", "ci_capabilities": ["required_gate_baseline"],
            "ci_reasons": ["required_gate_baseline:always_on"],
            "review_roles": ["qa_engineer"], "ordered_role_ids": ["qa_engineer"],
            "review_scope": "targeted", "review_escalated": False,
            "review_reasons": ["fixture"], "planner_config_sha256": "sha256:" + "9" * 64,
            "planner_identity": planner_identity,
            "planner_digest": impact.canonical_digest(planner_identity),
            "verification_affected": False,
        }
        projection["projection_digest"] = impact.canonical_digest(projection)
        source = identity.source_review_identity(
            task_uid=UID,
            bootstrap_epoch=1,
            repository="eng-cc/oasis7",
            pr_number=7,
            source_head_oid=self.head,
            source_scope_oid="b" * 40,
            changed_paths_digest="2" * 64,
            ordered_role_ids=["qa_engineer"],
            role_contract_digest="3" * 64,
            review_policy_digest="4" * 64,
            input_contract_digest=projection["projection_digest"].removeprefix("sha256:"),
        )
        receipt = {
            "receipt_type": "oasis7_ci_ready_receipt",
            "issuer": "github_live_query",
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
            "repository": "eng-cc/oasis7",
            "task_uid": UID,
            "task_issue_number": 1,
            "pr_number": 7,
            "base_oid": "c" * 40,
            "head_oid": self.head,
            "check_name": "required-gate",
            "check_app_id": 42,
            "check_run_id": 13,
            "planner_digest": "6" * 64,
            "conclusion": "success",
            "observed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            "integration_run_id": 12,
            "integration_base_oid": "c" * 40,
            "source_head_oid": self.head,
            "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
            "workflow_sha": "7" * 40,
            "request_id": 11,
            "request_created_at": "2026-09-11T00:00:00Z",
            "run_id": 12,
            "run_attempt": 1,
            "tested_tree_oid": "d" * 40,
            "impact_projection_schema": "oasis7-workflow-impact-projection/v2",
            "impact_projection_digest": projection["projection_digest"],
            "impact_projection_planner_digest": projection["planner_digest"],
        }
        integration = identity.integration_ci_identity(receipt)
        applicability_identity = identity.review_applicability_identity(source)
        plan = {
            "schema": "oasis7-review-plan/v2",
            "task_uid": UID,
            "frozen_head": self.head,
            "comparison_oid": "b" * 40,
            "roles": ["qa_engineer"],
            "source_review_identity": source,
            "source_review_digest": identity.source_review_digest(source),
            "professional_review_applicability": {
                "identity": applicability_identity,
                "identity_digest": identity.review_applicability_digest(applicability_identity),
                "verified": True,
            },
            "impact_projection_schema": "oasis7-workflow-impact-projection/v2",
            "impact_projection_digest": projection["projection_digest"],
            "impact_projection_planner_digest": projection["planner_digest"],
            "impact_projection": projection,
            "integration_ci_identity": integration,
            "integration_ci_digest": identity.integration_ci_digest(integration),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
            "preflight": {"ledger_path": str(self.root / "ledger.jsonl")},
        }
        plan_path = self.root / "plan.json"
        plan_path.write_text(json.dumps(plan), encoding="utf-8")
        receipt_path = self.root / "receipt.json"
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        return plan_path, receipt_path

    def _run_closeout(self, mode):
        self.events.write_text("", encoding="utf-8")
        ci_log = self.root / "ci-calls.log"
        ci_log.write_text("", encoding="utf-8")
        env = {
            **os.environ,
            "PM_ROOT_DIR": str(self.root),
            "EVENTS": str(self.events),
            "CI_CALL_LOG": str(ci_log),
            "CI_MODE": mode,
        }
        return subprocess.run(
            [
                str(self.root / "scripts/pm/task-closeout.sh"),
                "--role", "tpm",
                "--task-uid", UID,
                "--to-status", "ready",
                "--claim-type", "ready_for_pr",
                "--verification-profile", "production",
                "--review-packet-file", str(self.packet),
                "--ci-ready-receipt", str(self.receipt),
                "--json",
            ],
            cwd=self.root,
            env=env,
            text=True,
            capture_output=True,
        )

    def test_fresh_forged_marker_is_live_validated_before_transition(self):
        result = self._run_closeout("reject")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.root / "ci-calls.log").read_text(), "called\n")
        self.assertEqual((self.root / "events.log").read_text(), "")

    def test_validation_only_authority_payload_and_readback_cannot_advance_closeout(self):
        identity = load_identity_module(PM / "ci_ready_receipt_identity.py")
        base = json.loads(self.receipt.read_text(encoding="utf-8"))
        required_v2_values = {
            "planner_config_sha256": "sha256:" + "9" * 64,
            "run_rust_baseline": True,
            "request_key": "sha256:" + "a" * 64,
            "request_identity": {},
            "bootstrap_epoch": 1,
            "request_id": 12,
            "request_created_at": "2026-09-11T00:00:00Z",
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
            "source_scope_oid": "b" * 40,
            "trusted_policy_context": {},
            "effective_policy_identity": {},
            "required_plan_v2_artifact_id": 101,
            "required_plan_v2_artifact_name": "oasis7-required-plan-v2-12-a1",
            "required_result_v2_artifacts": [],
            "trusted_planner_inventory": {},
            "execution_jobs": [],
            "trusted_source_attempt": {},
        }
        for candidate in validation_only_artifacts():
            receipt = {**base, **required_v2_values, "required_plan_v2_payload": candidate}
            with self.subTest(schema=candidate["schema"]):
                with self.assertRaisesRegex(ValueError, "v2 required evidence plan payload is malformed"):
                    identity.review_evidence_identity(receipt)
                self.receipt.write_text(json.dumps(receipt), encoding="utf-8")
                result = self._run_closeout("validation-only")
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual((self.root / "ci-calls.log").read_text(), "called\n")
                self.assertEqual((self.root / "events.log").read_text(), "")

    def test_newer_explicit_dispatch_may_reuse_source_review_across_target_tree_drift(self):
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt.update(
            {
                "integration_run_id": 100,
                "request_id": 99,
                "request_created_at": "2026-09-11T01:00:00Z",
                "run_id": 100,
                "check_run_id": 101,
            }
        )
        self.receipt.write_text(json.dumps(receipt), encoding="utf-8")
        result = self._run_closeout("changed-tree")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.root / "ci-calls.log").read_text(), "called\n")
        self.assertEqual((self.root / "events.log").read_text(), "claim\ntransition\n")

    def test_current_generation_allows_v2_closeout(self):
        result = self._run_closeout("same")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.root / "events.log").read_text(), "claim\ntransition\n")

    def test_old_plan_is_rejected_after_bootstrap_epoch_migration(self):
        mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
        mapping["tasks"][UID]["bootstrap_epoch"] = 2
        mapping_path.write_text(json.dumps(mapping) + "\n", encoding="utf-8")
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        snapshot.unlink()
        subprocess.run(
            [
                sys.executable,
                str(self.root / "scripts/pm/bootstrap-task-snapshot.py"),
                "create",
                "--repo-root",
                str(self.root),
                "--task-uid",
                UID,
                "--request-identity",
                "fixture request",
                "--producer",
                "tpm",
            ],
            check=True,
            text=True,
            capture_output=True,
        )
        result = self._run_closeout("same")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("bootstrap snapshot", result.stderr.lower())
        self.assertEqual((self.root / "ci-calls.log").read_text(), "")
        self.assertEqual((self.root / "events.log").read_text(), "")

    def test_missing_current_generation_snapshot_is_rejected(self):
        (self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json").unlink()
        result = self._run_closeout("same")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("bootstrap snapshot", result.stderr.lower())
        self.assertEqual((self.root / "ci-calls.log").read_text(), "")
        self.assertEqual((self.root / "events.log").read_text(), "")

    def test_stale_current_snapshot_digest_is_rejected(self):
        snapshot = self.root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        saved = json.loads(snapshot.read_text(encoding="utf-8"))
        saved["task"]["bootstrap_epoch"] = 99
        snapshot.write_text(json.dumps(saved) + "\n", encoding="utf-8")
        result = self._run_closeout("same")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("bootstrap snapshot", result.stderr.lower())
        self.assertEqual((self.root / "ci-calls.log").read_text(), "")
        self.assertEqual((self.root / "events.log").read_text(), "")


if __name__ == "__main__":
    unittest.main()
