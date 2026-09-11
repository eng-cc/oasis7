#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path


SOURCE = Path(__file__).with_name("subagent-task-packet.py")
_SPEC = importlib.util.spec_from_file_location("subagent_task_packet_under_test", SOURCE)
assert _SPEC is not None and _SPEC.loader is not None
PACKET = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(PACKET)
SNAPSHOT_HELPER = Path(__file__).with_name("bootstrap-task-snapshot.py")
TASK_UID = "task_11111111111111111111111111111111"


class PacketTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name) / "repo"
        subprocess.run(["git", "init", "-b", "main", str(self.repo)], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.name", "Test"], check=True)
        for path in ("scripts/pm", ".pm/github-project-sync", ".agents/roles", "doc/engineering/workflow"):
            (self.repo / path).mkdir(parents=True, exist_ok=True)
        shutil.copy2(SOURCE, self.repo / "scripts/pm/subagent-task-packet.py")
        shutil.copy2(SOURCE.with_name("ci_ready_receipt_identity.py"), self.repo / "scripts/pm/ci_ready_receipt_identity.py")
        for helper in ('loop_gate.py', 'loop.py', 'loop_recovery.py'):
            shutil.copy2(SOURCE.with_name(helper), self.repo / 'scripts/pm' / helper)
        fakebin = Path(self.tmp.name) / 'fakebin'
        fakebin.mkdir()
        gh = fakebin / 'gh'
        gh.write_text('#!/usr/bin/env python3\nimport json,sys\nprint(json.dumps([] if any("/comments" in arg for arg in sys.argv) else {"body": "task_uid: ' + TASK_UID + '"}))\n')
        gh.chmod(0o755)
        environment = patch.dict(os.environ, {'PATH': str(fakebin) + os.pathsep + os.environ['PATH']})
        environment.start()
        self.addCleanup(environment.stop)
        shutil.copy2(SNAPSHOT_HELPER, self.repo / "scripts/pm/bootstrap-task-snapshot.py")
        for path in ("AGENTS.md", "doc/engineering/workflow/source-of-truth.md", ".agents/roles/qa_engineer.md", "scope.txt"):
            (self.repo / path).write_text(path + "\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "base"], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.repo), "switch", "-c", "task/packet"], check=True, capture_output=True)
        self.write_mapping()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def write_mapping(self, **changes: object) -> None:
        task = {"task_uid": TASK_UID, "canonical_worktree": str(self.repo.resolve()), "task_branch": "task/packet", "default_branch": "main", "owner_role": "qa_engineer", "issue_number": 1, "issue_url": "https://example.invalid/issues/1", "repository": "example/repo", "project_item_id": "PVTI_test", "status": "committed", "title": "review dispatch admission", "acceptance": ["reject stale review admission"]}
        task.update(changes)
        path = self.repo / ".pm/github-project-sync/tasks.json"
        path.write_text(json.dumps({"version": 1, "project": {"owner": "example", "number": 1}, "tasks": {TASK_UID: task}}), encoding="utf-8")

    def command(self, *extra: str) -> list[str]:
        return ["python3", "scripts/pm/subagent-task-packet.py", *extra]

    def create_args(self) -> list[str]:
        return ["create", "--task-uid", TASK_UID, "--slice-id", "qa-review", "--role", "qa_engineer", "--slice-type", "review", "--owner-role", "qa_engineer", "--integration-owner", "tpm", "--integration-order", "1/1", "--packet-producer", "tpm", "--context-delivery-mode", "minimal_head_bound_task_packet", "--intended-model-configuration", "inherit current parent selection", "--actual-dispatched-model-reasoning", "inherited/unverified", "--actual-runtime-evidence-reason", "dispatch surface does not report inherited runtime", "--role-activation", "message_assigned_adapter_inactive", "--base", "main", "--user-intent", "review packet behavior", "--work-item", "validate the bounded helper", "--non-goals", "no product changes", "--acceptance-target", "focused tests pass", "--governance-ref", "AGENTS.md", "--governance-ref", "doc/engineering/workflow/source-of-truth.md", "--governance-ref", ".agents/roles/qa_engineer.md", "--scoped-ref", "scope.txt", "--evidence-summary", "scope.txt is the only task surface", "--collaboration-boundary", "read only except assigned files", "--write-scope", "scripts/pm/**", "--return-contract", "patch and test evidence", "--validation-command", "python3 scripts/pm/subagent-task-packet.test.py", "--formal-sink", "https://example.invalid/issues/1"]

    def invoke(self, args: list[str], ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertEqual(0 if ok else 1, result.returncode, result.stdout + result.stderr)
        return result

    def test_create_and_validate(self) -> None:
        result = self.invoke(self.create_args())
        packet_path = result.stdout.splitlines()[0]
        packet = json.loads((self.repo / packet_path).read_text())
        self.assertEqual(subprocess.check_output(["git", "-C", str(self.repo), "rev-parse", "HEAD"], text=True).strip(), packet["identity"]["head"])
        self.assertEqual("tpm", packet["identity"]["packet_producer"])
        self.assertEqual("minimal_head_bound_task_packet", packet["slice"]["context_delivery_mode"])
        self.assertEqual("1/1", packet["slice"]["integration_order"])
        self.assertEqual("inherited/unverified", packet["slice"]["actual_dispatched_model_reasoning"])
        self.assertEqual("message_assigned_adapter_inactive", packet["slice"]["role_activation"])
        self.assertNotIn("embedded_docs", packet)
        self.invoke(["validate", packet_path])

    def test_missing_mandatory_fields_fail(self) -> None:
        args = self.create_args()
        index = args.index("--work-item")
        del args[index:index + 2]
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("--work-item", result.stderr)

        for option in ("--packet-producer", "--context-delivery-mode", "--integration-order", "--intended-model-configuration", "--actual-dispatched-model-reasoning", "--actual-runtime-evidence-reason", "--role-activation"):
            args = self.create_args()
            index = args.index(option)
            del args[index:index + 2]
            result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
            self.assertNotEqual(0, result.returncode)
            self.assertIn(option, result.stderr)

        args = self.create_args()
        args[args.index("message_assigned_adapter_inactive")] = "unsupported_activation"
        result = subprocess.run(self.command(*args), cwd=self.repo, text=True, capture_output=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("invalid choice", result.stderr)

    def test_delivery_mode_escalation_and_digest_tamper(self) -> None:
        args = self.create_args()
        args[args.index("minimal_head_bound_task_packet")] = "full_history_escalation"
        result = self.invoke(args, ok=False)
        self.assertIn("full_history_escalation_reason", result.stderr)

        args.extend(["--full-history-escalation-reason", "prior user authority is absent from scoped evidence"])
        result = self.invoke(args)
        packet_path = result.stdout.splitlines()[0]
        packet_file = self.repo / packet_path
        packet = json.loads(packet_file.read_text())
        packet["identity"]["packet_producer"] = "tampered"
        packet_file.write_text(json.dumps(packet), encoding="utf-8")
        self.assertIn("packet digest mismatch", self.invoke(["validate", packet_path], ok=False).stderr)

    def test_wrong_task_and_worktree_fail(self) -> None:
        args = self.create_args(); args[args.index(TASK_UID)] = "task_22222222222222222222222222222222"
        self.assertIn("not present", self.invoke(args, ok=False).stderr)
        self.write_mapping(canonical_worktree=str(self.repo.parent / "wrong"))
        self.assertIn("wrong worktree", self.invoke(self.create_args(), ok=False).stderr)

    def test_overwrite_and_stale_head_fail(self) -> None:
        result = self.invoke(self.create_args())
        packet_path = result.stdout.splitlines()[0]
        self.assertIn("refusing to overwrite", self.invoke(self.create_args(), ok=False).stderr)
        (self.repo / "new.txt").write_text("new\n")
        subprocess.run(["git", "-C", str(self.repo), "add", "new.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "advance"], check=True, capture_output=True)
        self.assertIn("stale or mismatched packet head", self.invoke(["validate", packet_path], ok=False).stderr)

    def create_snapshot(self) -> Path:
        snapshot = self.repo / ".pm/scratch" / TASK_UID / "bootstrap-task-snapshot.json"
        result = subprocess.run(
            ["python3", "scripts/pm/bootstrap-task-snapshot.py", "create",
             "--repo-root", str(self.repo), "--task-uid", TASK_UID,
             "--request-identity", "review dispatch admission", "--producer", "tpm"],
            cwd=self.repo, text=True, capture_output=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        return snapshot

    def create_review_plan(self, packet_path: str, **changes: object) -> Path:
        base_sha = self.git("rev-parse", "main")
        head = self.git("rev-parse", "HEAD")
        evidence_digest = "b" * 64
        expected_slices = sorted([
            {"role": "repository_health_engineer", "slice_id": "repository-health-review"},
            {"role": "qa_engineer", "slice_id": "qa-review"},
        ], key=lambda item: (item["role"], item["slice_id"]))
        batch_identity = {
            "task_uid": TASK_UID, "frozen_head": head,
            "relevant_evidence_digest": evidence_digest, "expected_slices": expected_slices,
        }
        epoch = hashlib.sha256(json.dumps(
            batch_identity, sort_keys=True, separators=(",", ":"),
        ).encode("utf-8")).hexdigest()
        batch_path = self.repo / ".pm/scratch" / TASK_UID / "review-batches" / f"{epoch}.json"
        batch_path.parent.mkdir(parents=True, exist_ok=True)
        batch_path.write_text(json.dumps({
            "schema": "oasis7-review-batch/v1", "epoch": epoch, "task_uid": TASK_UID,
            "frozen_head": head, "relevant_evidence_digest": evidence_digest,
            "expected_slices": expected_slices,
        }), encoding="utf-8")
        plan = {
            "schema": "oasis7-review-plan/v1", "task_uid": TASK_UID,
            "frozen_head": head, "comparison_ref": "main", "comparison_oid": base_sha,
            "epoch": epoch, "batch_path": str(batch_path), "relevant_evidence_digest": evidence_digest,
            "roles": [item["role"] for item in expected_slices],
            "expected_slices": expected_slices,
            "packet_refs": [
                {"role": "repository_health_engineer", "slice_id": "repository-health-review",
                 "packet_ref": f".pm/scratch/{TASK_UID}/slice-packets/repository-health-review.json"},
                {"role": "qa_engineer", "slice_id": "qa-review", "packet_ref": packet_path},
            ],
        }
        plan.update(changes)
        path = self.repo / ".pm/scratch" / TASK_UID / "review-plans" / "admission.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(plan), encoding="utf-8")
        return path

    def create_v2_review_plan(self, packet_path: str, bootstrap_epoch: int) -> Path:
        identity_spec = importlib.util.spec_from_file_location(
            "ci_ready_receipt_identity_for_test", SOURCE.with_name("ci_ready_receipt_identity.py")
        )
        assert identity_spec and identity_spec.loader
        identity_module = importlib.util.module_from_spec(identity_spec)
        identity_spec.loader.exec_module(identity_module)
        base_sha = self.git("rev-parse", "main")
        head = self.git("rev-parse", "HEAD")
        source = identity_module.source_review_identity(
            task_uid=TASK_UID,
            bootstrap_epoch=bootstrap_epoch,
            repository="example/repo",
            pr_number=1,
            source_head_oid=head,
            source_scope_oid=base_sha,
            changed_paths_digest="1" * 64,
            ordered_role_ids=["repository_health_engineer", "qa_engineer"],
            role_contract_digest="2" * 64,
            review_policy_digest="3" * 64,
            input_contract_digest="4" * 64,
        )
        receipt = {
            "repository": "example/repo", "task_uid": TASK_UID, "pr_number": 1,
            "source_head_oid": head, "integration_base_oid": base_sha,
            "workflow_ref": "example/repo/.github/workflows/rust.yml@refs/heads/main",
            "workflow_sha": base_sha, "request_id": 1,
            "request_created_at": "2026-09-11T00:00:00Z", "run_id": 2,
            "run_attempt": 1, "check_app_id": 3, "check_run_id": 4,
            "planner_digest": "5" * 64,
            "tested_tree_oid": self.git("rev-parse", "HEAD^{tree}"),
            "conclusion": "success",
        }
        integration = identity_module.integration_ci_identity(receipt)
        source_digest = identity_module.source_review_digest(source)
        expected_slices = sorted([
            {"role": "repository_health_engineer", "slice_id": "repository-health-review"},
            {"role": "qa_engineer", "slice_id": "qa-review"},
        ], key=lambda item: (item["role"], item["slice_id"]))
        batch_identity = {
            "task_uid": TASK_UID, "frozen_head": head,
            "relevant_evidence_digest": source_digest, "expected_slices": expected_slices,
        }
        epoch = hashlib.sha256(json.dumps(
            batch_identity, sort_keys=True, separators=(",", ":"),
        ).encode("utf-8")).hexdigest()
        batch_path = self.repo / ".pm/scratch" / TASK_UID / "review-batches" / f"{epoch}.json"
        batch_path.parent.mkdir(parents=True, exist_ok=True)
        batch_path.write_text(json.dumps({
            "schema": "oasis7-review-batch/v1", "epoch": epoch, "task_uid": TASK_UID,
            "frozen_head": head, "relevant_evidence_digest": source_digest,
            "expected_slices": expected_slices,
        }), encoding="utf-8")
        plan = {
            "schema": "oasis7-review-plan/v2", "task_uid": TASK_UID,
            "frozen_head": head, "comparison_ref": "main", "comparison_oid": base_sha,
            "epoch": epoch, "batch_path": str(batch_path),
            "relevant_evidence_digest": source_digest,
            "source_review_identity": source,
            "source_review_digest": source_digest,
            "integration_ci_identity": integration,
            "integration_ci_digest": identity_module.integration_ci_digest(integration),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
            "source_scope_oid": base_sha, "integration_base_oid": base_sha,
            "roles": [item["role"] for item in expected_slices],
            "expected_slices": expected_slices,
            "packet_refs": [
                {"role": "repository_health_engineer", "slice_id": "repository-health-review",
                 "packet_ref": f".pm/scratch/{TASK_UID}/slice-packets/repository-health-review.json"},
                {"role": "qa_engineer", "slice_id": "qa-review", "packet_ref": packet_path},
            ],
        }
        path = self.repo / ".pm/scratch" / TASK_UID / "review-plans" / f"v2-{bootstrap_epoch}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(plan), encoding="utf-8")
        return path

    def git(self, *args: str) -> str:
        return subprocess.check_output(["git", "-C", str(self.repo), *args], text=True).strip()

    def review_admission(self, packet: str, plan: Path, snapshot: Path, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            self.command("review-admission", "--packet", packet, "--review-plan", str(plan), "--bootstrap-snapshot", str(snapshot)),
            cwd=self.repo, text=True, capture_output=True,
        )
        if ok:
            self.assertEqual(0, result.returncode, result.stderr)
        else:
            self.assertNotEqual(0, result.returncode, result.stdout)
        return result

    def write_incremental_prior(self) -> tuple[Path, str, str, Path]:
        prior_head = self.git("rev-parse", "HEAD")
        requested_slices = [
            {"role": "repository_health_engineer", "slice_id": "repository-health-review"},
            {"role": "qa_engineer", "slice_id": "qa-review"},
            {"role": "producer_system_designer", "slice_id": "producer-review"},
        ]
        expected_slices = sorted(requested_slices, key=lambda item: (item["role"], item["slice_id"]))
        batch_identity = {
            "task_uid": TASK_UID, "frozen_head": prior_head,
            "relevant_evidence_digest": "b" * 64,
            "expected_slices": expected_slices,
        }
        prior_epoch = hashlib.sha256(json.dumps(
            batch_identity, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")).hexdigest()
        prior_path = self.repo / ".pm/scratch" / TASK_UID / "review-plans" / "prior.json"
        ledger_path = self.repo / ".pm/scratch" / TASK_UID / "prior-ledger.jsonl"
        batch_path = self.repo / ".pm/scratch" / TASK_UID / "review-batches" / f"{prior_epoch}.json"
        collection_path = batch_path.with_name(f"{prior_epoch}.collection.json")
        prior_path.parent.mkdir(parents=True, exist_ok=True)
        ledger_path.parent.mkdir(parents=True, exist_ok=True)
        collection_path.parent.mkdir(parents=True, exist_ok=True)
        rows = []
        for item in expected_slices:
            artifact_path = ledger_path.parent / f"{item['slice_id']}.json"
            artifact_path.write_text(json.dumps({
                "role": item["role"], "slice_id": item["slice_id"],
                "task_uid": TASK_UID, "head": prior_head, "epoch": prior_epoch,
                "status": "completed", "disposition": "no_findings",
                "findings": [], "residual_risk": "none",
            }, sort_keys=True), encoding="utf-8")
            rows.append({
                "role": item["role"], "slice_id": item["slice_id"],
                "task_uid": TASK_UID, "head": prior_head, "epoch": prior_epoch,
                "status": "completed",
                "artifact_digest": hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
                "artifacts": [str(artifact_path)],
            })
        ledger_path.write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows),
            encoding="utf-8",
        )
        ledger_digest = hashlib.sha256(ledger_path.read_bytes()).hexdigest()
        batch_path.write_text(json.dumps({
            "schema": "oasis7-review-batch/v1", "epoch": prior_epoch,
            **batch_identity,
        }), encoding="utf-8")
        prior_path.write_text(json.dumps({
            "schema": "oasis7-review-plan/v1", "task_uid": TASK_UID,
            "frozen_head": prior_head, "epoch": prior_epoch,
            "relevant_evidence_digest": "b" * 64,
            "roles": [item["role"] for item in requested_slices],
            "batch_path": str(batch_path), "expected_slices": requested_slices,
            "preflight": {"ledger_path": str(ledger_path)},
        }), encoding="utf-8")
        collection_path.write_text(json.dumps({
            "schema": "oasis7-review-collection/v1", "status": "passed",
            "epoch": prior_epoch, "task_uid": TASK_UID, "frozen_head": prior_head,
            "ledger_digest": ledger_digest,
            "roles": [item["role"] for item in expected_slices],
        }), encoding="utf-8")
        return prior_path, prior_head, prior_epoch, collection_path

    def test_review_admission_requires_current_cross_bound_packet_plan_and_snapshot(self) -> None:
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        plan = self.create_review_plan(packet)
        admitted = self.review_admission(packet, plan, snapshot)
        self.assertEqual("admitted", json.loads(admitted.stdout)["status"])

        cases = {
            "plan task mismatch": {"task_uid": "task_22222222222222222222222222222222"},
            "plan head mismatch": {"frozen_head": "a" * 40},
            "plan role mismatch": {"expected_slices": [{"role": "runtime_engineer", "slice_id": "qa-review"}]},
            "plan slice mismatch": {"expected_slices": [{"role": "qa_engineer", "slice_id": "other-slice"}]},
            "plan packet ref mismatch": {"packet_refs": [{"role": "qa_engineer", "slice_id": "qa-review", "packet_ref": "scope.txt"}]},
            "comparison oid mismatch": {"comparison_oid": "b" * 40},
        }
        for name, changes in cases.items():
            with self.subTest(name=name):
                bad_plan = self.create_review_plan(packet, **changes)
                self.review_admission(packet, bad_plan, snapshot, ok=False)

        plan = self.create_review_plan(packet)
        payload = json.loads(snapshot.read_text(encoding="utf-8"))
        payload["producer"] = "tampered"
        snapshot.write_text(json.dumps(payload), encoding="utf-8")
        self.review_admission(packet, plan, snapshot, ok=False)

    def test_v2_review_admission_binds_plan_epoch_to_bootstrap_snapshot(self) -> None:
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        valid = self.create_v2_review_plan(packet, bootstrap_epoch=1)
        admitted = self.review_admission(packet, valid, snapshot)
        self.assertEqual("admitted", json.loads(admitted.stdout)["status"])

        wrong_epoch = self.create_v2_review_plan(packet, bootstrap_epoch=2)
        rejected = self.review_admission(packet, wrong_epoch, snapshot, ok=False)
        self.assertIn("bootstrap epoch", rejected.stderr.lower())

    def test_review_admission_invalidates_after_head_or_comparison_ref_changes(self) -> None:
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        plan = self.create_review_plan(packet)
        self.review_admission(packet, plan, snapshot)

        original_base = self.git("rev-parse", "main")
        moved_base = self.git("commit-tree", "HEAD^{tree}", "-p", original_base, "-m", "moved comparison")
        self.git("update-ref", "main", moved_base)
        self.review_admission(packet, plan, snapshot, ok=False)
        self.git("update-ref", "main", original_base)

        (self.repo / "advance.txt").write_text("advance\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "advance.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "advance"], check=True, capture_output=True)
        self.review_admission(packet, plan, snapshot, ok=False)

    def test_frozen_base_packet_survives_symbolic_ref_movement(self) -> None:
        frozen_base = self.git("rev-parse", "main")
        args = self.create_args() + ["--frozen-base-oid", frozen_base]
        packet = self.invoke(args).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        plan = self.create_review_plan(packet)
        self.review_admission(packet, plan, snapshot)

        moved_base = self.git("commit-tree", "HEAD^{tree}", "-p", frozen_base, "-m", "moved comparison")
        self.git("update-ref", "main", moved_base)

        self.invoke(["validate", packet])
        admitted = self.review_admission(packet, plan, snapshot)
        self.assertEqual("admitted", json.loads(admitted.stdout)["status"])

    def test_review_admission_allows_a_valid_bootstrap_snapshot_before_review_head(self) -> None:
        snapshot = self.create_snapshot()
        (self.repo / "implementation.txt").write_text("implementation\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "implementation.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "implementation"], check=True, capture_output=True)

        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        plan = self.create_review_plan(packet)
        admitted = self.review_admission(packet, plan, snapshot)
        self.assertEqual("admitted", json.loads(admitted.stdout)["status"])

    def test_review_admission_rejects_a_replacement_plan_that_drops_a_required_batch_role(self) -> None:
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        plan = self.create_review_plan(packet)
        self.review_admission(packet, plan, snapshot)

        replacement = json.loads(plan.read_text(encoding="utf-8"))
        replacement["roles"] = ["qa_engineer"]
        replacement["expected_slices"] = [{"role": "qa_engineer", "slice_id": "qa-review"}]
        plan.write_text(json.dumps(replacement), encoding="utf-8")
        self.review_admission(packet, plan, snapshot, ok=False)

    def test_review_plan_context_is_consumed_by_packet_and_admission(self) -> None:
        prior_path, prior_head, prior_epoch, collection_path = self.write_incremental_prior()
        (self.repo / "repair.txt").write_text("repair\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "repair.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "repair"], check=True, capture_output=True)

        packet_ref = f".pm/scratch/{TASK_UID}/slice-packets/qa-review.json"
        plan = self.create_review_plan(packet_ref)
        context = {
            "schema": "oasis7-review-context/v1", "authority": "context_only",
            "task_uid": TASK_UID, "prior_plan_path": str(prior_path.relative_to(self.repo)),
            "prior_plan_digest": hashlib.sha256(prior_path.read_bytes()).hexdigest(),
            "prior_head_oid": prior_head, "prior_epoch": prior_epoch,
            "current_head_oid": self.git("rev-parse", "HEAD"),
            "prior_source_review_digest": "b" * 64, "prior_integration_ci_digest": None,
            "prior_roles": ["repository_health_engineer", "qa_engineer", "producer_system_designer"],
            "delta_paths": ["repair.txt"],
            "prior_collection_path": str(collection_path.relative_to(self.repo)),
            "prior_collection_digest": hashlib.sha256(collection_path.read_bytes()).hexdigest(),
            "prior_collection_ledger_digest": json.loads(collection_path.read_text())["ledger_digest"],
            "delta_paths_digest": hashlib.sha256(json.dumps(
                ["repair.txt"], separators=(",", ":"), sort_keys=True,
            ).encode()).hexdigest(),
            "delta_patch_digest": hashlib.sha256(subprocess.check_output(
                ["git", "-C", str(self.repo), "diff", "--binary", "--no-renames", prior_head, self.git("rev-parse", "HEAD")],
            )).hexdigest(),
            "reviewer_guidance": "confirm impact",
        }
        plan_payload = json.loads(plan.read_text(encoding="utf-8"))
        plan_payload["incremental_review_context"] = context
        plan.write_text(json.dumps(plan_payload), encoding="utf-8")

        packet = self.invoke(self.create_args() + ["--review-plan", str(plan)]).stdout.splitlines()[0]
        snapshot = self.create_snapshot()
        admitted = self.review_admission(packet, plan, snapshot)
        admitted_payload = json.loads(admitted.stdout)
        self.assertEqual("admitted", admitted_payload["status"])
        self.assertTrue(admitted_payload["incremental_review_context_digest"])

        ledger_path = self.repo / ".pm/scratch" / TASK_UID / "prior-ledger.jsonl"
        first_row = json.loads(ledger_path.read_text(encoding="utf-8").splitlines()[0])
        Path(str(first_row["artifacts"][0])).unlink()
        rejected = self.review_admission(packet, plan, snapshot, ok=False)
        self.assertRegex(rejected.stderr.lower(), r"artifact|ledger")

    def test_binary_diff_digest_ignores_external_diff_and_textconv(self) -> None:
        prior_head = self.git("rev-parse", "HEAD")
        (self.repo / "repair.txt").write_text("repair\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "repair.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "repair"], check=True, capture_output=True)
        current_head = self.git("rev-parse", "HEAD")
        external = self.repo / "external-diff"
        external.write_text("#!/bin/sh\nprintf 'external diff output\\n'\n", encoding="utf-8")
        external.chmod(0o755)
        baseline = PACKET.binary_diff_digest(self.repo, prior_head, current_head)
        self.git("config", "diff.external", str(external))
        with patch.dict(os.environ, {"GIT_EXTERNAL_DIFF": str(external)}):
            self.assertEqual(
                baseline,
                PACKET.binary_diff_digest(self.repo, prior_head, current_head),
            )

    def test_review_admission_rejects_packet_that_omits_plan_context(self) -> None:
        prior_path, prior_head, prior_epoch, collection_path = self.write_incremental_prior()
        (self.repo / "repair.txt").write_text("repair\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "repair.txt"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-m", "repair"], check=True, capture_output=True)
        packet_ref = f".pm/scratch/{TASK_UID}/slice-packets/qa-review.json"
        plan = self.create_review_plan(packet_ref)
        context = {
            "schema": "oasis7-review-context/v1", "authority": "context_only",
            "task_uid": TASK_UID, "prior_plan_path": str(prior_path.relative_to(self.repo)),
            "prior_plan_digest": hashlib.sha256(prior_path.read_bytes()).hexdigest(),
            "prior_head_oid": prior_head, "prior_epoch": prior_epoch,
            "current_head_oid": self.git("rev-parse", "HEAD"),
            "prior_source_review_digest": "b" * 64, "prior_integration_ci_digest": None,
            "prior_roles": ["repository_health_engineer", "qa_engineer", "producer_system_designer"],
            "delta_paths": ["repair.txt"],
            "prior_collection_path": str(collection_path.relative_to(self.repo)),
            "prior_collection_digest": hashlib.sha256(collection_path.read_bytes()).hexdigest(),
            "prior_collection_ledger_digest": json.loads(collection_path.read_text())["ledger_digest"],
            "delta_paths_digest": hashlib.sha256(json.dumps(
                ["repair.txt"], separators=(",", ":"), sort_keys=True,
            ).encode()).hexdigest(),
            "delta_patch_digest": hashlib.sha256(subprocess.check_output(
                ["git", "-C", str(self.repo), "diff", "--binary", "--no-renames", prior_head, self.git("rev-parse", "HEAD")],
            )).hexdigest(),
            "reviewer_guidance": "confirm impact",
        }
        plan_payload = json.loads(plan.read_text(encoding="utf-8"))
        plan_payload["incremental_review_context"] = context
        plan.write_text(json.dumps(plan_payload), encoding="utf-8")
        packet = self.invoke(self.create_args()).stdout.splitlines()[0]
        rejected = self.review_admission(packet, plan, self.create_snapshot(), ok=False)
        self.assertIn("review context", rejected.stderr.lower())


if __name__ == "__main__":
    unittest.main()
