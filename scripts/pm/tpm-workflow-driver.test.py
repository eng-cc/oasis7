#!/usr/bin/env python3
"""RED: uninterrupted, crash-resumable TPM lifecycle controller contract."""

from __future__ import annotations

import json
import hashlib
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import Optional

ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / "scripts/pm/fixtures/tpm-workflow/workflow-driver.py"
PRODUCTION_DRIVER = ROOT / "scripts/pm/tpm-workflow-driver.py"
FAKE_GITHUB = ROOT / "scripts/pm/fixtures/tpm-workflow/fake-github.py"
LIVE_VALIDATOR = ROOT / "scripts/pm/fixtures/tpm-workflow/live-receipt-validator.py"
DELIVERY_ADAPTER = ROOT / "scripts/pm/fixtures/tpm-workflow/delivery-adapter.py"
AUTHORITY_READER = ROOT / "scripts/pm/fixtures/tpm-workflow/canonical-authority-reader.py"
PHASES = ["bootstrap", "route", "dispatch", "execute", "integrate", "freeze", "verify",
          "review", "closeout", "create_pr", "record_pr", "comment", "watch", "fix",
          "reverify", "push", "merge", "merge_receipt", "task_done", "main_sync", "safe_cleanup"]


class DriverContract(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.state = self.dir / "workflow.json"
        self.remote = self.dir / "github.json"

    def seed_remote(self, **overrides: object) -> None:
        value = {"clock": 0, "calls": [], "tasks": [], "prs": [], "comments": [],
                 "merge_receipts": [], "failures": []}
        value.update(overrides)
        self.remote.write_text(json.dumps(value))

    def driver_env(self, *, clock: str = "2026-07-11T00:00:00Z",
                   test_adapter: bool = True,
                   extra_env: Optional[dict[str, str]] = None) -> dict[str, str]:
        env = os.environ.copy()
        env.update({"TPM_GITHUB_ADAPTER": str(FAKE_GITHUB), "TPM_GITHUB_STATE": str(self.remote),
                    "TPM_TEST_CLOCK": clock})
        if test_adapter:
            env["TPM_ADAPTER_MODE"] = "test_only"
        else:
            env.pop("TPM_ADAPTER_MODE", None)
        if extra_env:
            env.update(extra_env)
        return env

    def run_raw(self, *args: str, clock: str = "2026-07-11T00:00:00Z",
                test_adapter: bool = True,
                extra_env: Optional[dict[str, str]] = None) -> subprocess.CompletedProcess[str]:
        env = self.driver_env(clock=clock, test_adapter=test_adapter, extra_env=extra_env)
        return subprocess.run([str(DRIVER), "--state", str(self.state), "--json", *args],
                              text=True, capture_output=True, env=env)

    def run_driver(self, *args: str, expect: int = 0,
                   clock: str = "2026-07-11T00:00:00Z",
                   test_adapter: bool = True,
                   extra_env: Optional[dict[str, str]] = None) -> dict:
        argv = list(args)
        if "--resume" in argv and "--expected-revision" not in argv and self.state.exists():
            current = self.workflow()
            argv.extend(["--expected-revision", str(current["revision"]),
                         "--lease-token", current["lease"]["token"]])
        proc = self.run_raw(*argv, clock=clock, test_adapter=test_adapter,
                            extra_env=extra_env)
        self.assertEqual(expect, proc.returncode, proc.stderr or proc.stdout)
        return json.loads(proc.stdout)

    def workflow(self) -> dict:
        return json.loads(self.state.read_text())

    def remote_state(self) -> dict:
        return json.loads(self.remote.read_text())

    def write_receipt(self, name: str, value: dict) -> Path:
        path = self.dir / f"{name}.receipt.json"
        path.write_text(json.dumps(value, sort_keys=True))
        return path

    def test_checkpoint_has_complete_resumable_controller_state(self) -> None:
        self.seed_remote()
        self.run_driver("--initialize", "--task-uid", "task_" + "1" * 32, "--stop-after", "bootstrap")
        first = self.workflow()
        for key in ("phase", "next_action", "wake_at", "lease", "attempt", "retry", "blocker"):
            self.assertIn(key, first)
        self.run_driver("--resume", "--stop-after", "route")
        self.assertEqual("route", self.workflow()["phase"])

    def test_remote_intent_readback_commit_is_exactly_once_across_crashes(self) -> None:
        for operation in ("create_pr", "comment", "merge"):
            with self.subTest(operation=operation):
                self.state.unlink(missing_ok=True); self.seed_remote()
                self.run_driver("--initialize", "--task-uid", "task_" + "2" * 32,
                                "--crash-after-remote", operation, expect=86)
                self.run_driver("--resume", "--stop-after", operation)
                remote = self.remote_state()
                collection = {"create_pr": "prs", "comment": "comments", "merge": "merge_receipts"}[operation]
                self.assertEqual(1, len(remote[collection]), f"{operation} must use durable idempotency key")
                journal = self.workflow()["remote_journal"][operation]
                self.assertEqual("committed", journal["state"])
                self.assertIn("intent", journal); self.assertIn("readback", journal)

    def test_slice_scheduler_state_machine_batch_timeout_and_retry(self) -> None:
        self.seed_remote()
        self.run_driver("--initialize", "--task-uid", "task_" + "3" * 32,
                        "--fixture-slices", "5", "--slice-batch-width", "2", "--stop-after", "dispatch")
        slices = self.workflow()["slices"]
        allowed = {"planned", "dispatched", "running", "returned", "failed_retryable", "terminal", "superseded", "integrated"}
        self.assertTrue(all(x["state"] in allowed for x in slices))
        self.assertLessEqual(sum(x["state"] in {"dispatched", "running"} for x in slices), 2)
        self.run_driver("--resume", "--advance-clock", "PT31M", "--inject-no-payload", slices[0]["id"],
                        "--stop-after", "dispatch")
        updated = self.workflow()["slices"]
        self.assertTrue(any(x["state"] == "superseded" for x in updated))
        self.assertTrue(any(x.get("attempt", 0) == 2 for x in updated))

    def test_transient_github_failures_backoff_retry_after_and_heartbeat(self) -> None:
        failures = [{"operation": "create_pr", "status": status, "remaining": 1,
                     "retry_after": 17 if status == 429 else None} for status in (429, 500, 599)]
        for failure in failures:
            with self.subTest(status=failure["status"]):
                self.state.unlink(missing_ok=True); self.seed_remote(failures=[failure])
                out = self.run_driver("--initialize", "--task-uid", "task_" + "4" * 32,
                                      "--stop-after", "create_pr", expect=75)
                self.assertEqual("external_wait", out["status"])
                self.assertGreater(out["wake_at"], "2026-07-11T00:00:00Z")
                if failure["status"] == 429: self.assertEqual(17, out["retry"]["delay_seconds"])
                self.assertIn("heartbeat", out)

    def test_record_pr_creates_and_migrates_canonical_normal_hold(self) -> None:
        self.seed_remote()
        head1, head2 = "a" * 40, "b" * 40
        self.run_driver("--initialize", "--task-uid", "task_" + "5" * 32,
                        "--pr-head", head1, "--stop-after", "record_pr")
        self.run_driver("--resume", "--pr-head", head2, "--stop-after", "record_pr")
        holds = [x for x in self.remote_state()["comments"] if x.get("kind") == "normal_pr_ci_watch"]
        self.assertEqual([head1, head2], [x["head_oid"] for x in holds])
        self.assertEqual("superseded", holds[0]["disposition"])
        self.assertIn(holds[1]["disposition"], {"inactive", "normal"})
        self.assertEqual(holds[1]["disposition"], self.workflow()["canonical_hold"]["disposition"])

    def assert_durable_blocker(self, blocker: str, expected_status: str) -> None:
        self.state.unlink(missing_ok=True); self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "6" * 32,
                              "--inject-blocker", blocker, expect=75)
        self.assertEqual(expected_status, out["status"])
        self.assertEqual(blocker, out["blocker"]["class"])
        self.assertIn("resume_condition", out["blocker"])
        self.assertIn("escalation", out["blocker"])
        evidence = self.write_receipt("canonical-wait", {
            "schema": "tpm-canonical-evidence/v1", "task_uid": out["task_uid"],
            "blocker": blocker, "source": "github_task_issue_comment",
            "node_id": "IC_kw-canonical", "readback_verified": True,
        })
        rejected = self.run_driver("--resume", "--complete-action", "resolve_wait",
                                   "--receipt-file", str(evidence), "--stop-after", "next",
                                   expect=75)
        self.assertIn(rejected["blocker"]["class"],
                      {"invalid_action_receipt", "canonical_evidence_readback_failed"})

    def test_missing_capability_is_durable_capability_blocked(self) -> None:
        self.assert_durable_blocker("dispatch_attestation_unavailable", "capability_blocked")

    def test_human_approval_is_durable_external_wait(self) -> None:
        self.assert_durable_blocker("human_approval_required", "external_wait")

    def test_done_requires_merge_receipt_done_main_sync_and_safe_cleanup(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "7" * 32, "--run-to-completion")
        self.assertEqual("completed", out["status"])
        transitions = out["completed_transitions"]
        self.assertLess(transitions.index("merge_receipt"), transitions.index("task_done"))
        self.assertLess(transitions.index("task_done"), transitions.index("main_sync"))
        self.assertLess(transitions.index("main_sync"), transitions.index("safe_cleanup"))
        self.assertTrue(out["cleanup_receipt"]["safe"])

    def test_happy_path_is_single_task_worktree_pr_merged_done_cleaned(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "8" * 32, "--run-to-completion")
        remote = self.remote_state()
        self.assertEqual(1, len(remote["tasks"])); self.assertEqual(1, len(remote["prs"]))
        self.assertEqual(1, len(remote["merge_receipts"]))
        self.assertEqual("MERGED", out["pr_state"]); self.assertEqual("done", out["task_state"])
        self.assertEqual("cleaned", out["worktree_state"])


class UninterruptedSafetyContract(DriverContract):
    """Adversarial RED contract for a production-capable TPM supervisor."""

    def test_plain_resume_cannot_bypass_human_or_capability_wait(self) -> None:
        for blocker, status in (("human_approval_required", "external_wait"),
                                ("dispatch_attestation_unavailable", "capability_blocked")):
            with self.subTest(blocker=blocker, status=status):
                self.state.unlink(missing_ok=True)
                self.seed_remote()
                self.run_driver("--initialize", "--task-uid", "task_" + "9" * 32,
                                "--inject-blocker", blocker, expect=75)
                out = self.run_driver("--resume", "--run-to-completion", expect=75,
                                      clock="2026-07-11T00:00:01Z")
                self.assertEqual(status, out["status"])
                self.assertEqual(blocker, out["blocker"]["class"])
                self.assertNotEqual("MERGED", out["pr_state"])
                self.assertNotEqual("done", out["task_state"])
                self.assertNotEqual("cleaned", out["worktree_state"])

    def test_retry_after_is_enforced_before_remote_retry(self) -> None:
        self.seed_remote(failures=[{"operation": "create_pr", "status": 429,
                                   "remaining": 1, "retry_after": 60}])
        self.run_driver("--initialize", "--task-uid", "task_" + "a" * 32,
                        "--stop-after", "create_pr", expect=75)
        calls_before = len(self.remote_state()["calls"])
        out = self.run_driver("--resume", "--run-to-completion", expect=75,
                              clock="2026-07-11T00:00:30Z")
        self.assertEqual(calls_before, len(self.remote_state()["calls"]))
        self.assertEqual("external_wait", out["status"])
        self.assertNotEqual("MERGED", out["pr_state"])

    def test_fake_adapter_requires_explicit_test_only_mode(self) -> None:
        self.seed_remote()
        proc = self.run_raw("--initialize", "--task-uid", "task_" + "b" * 32,
                            "--run-to-completion", test_adapter=False)
        self.assertEqual(75, proc.returncode, proc.stderr or proc.stdout)
        out = json.loads(proc.stdout)
        self.assertIn(out["status"], {"action_required", "external_wait"})
        self.assertEqual([], self.remote_state()["merge_receipts"])
        self.assertNotEqual("MERGED", out["pr_state"])

    def test_full_lifecycle_has_receipt_or_action_required_at_every_phase(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "c" * 32,
                              "--run-to-completion")
        required = ("bootstrap", "route", "execute", "integrate", "freeze", "verify",
                    "review", "closeout", "create_pr", "watch", "fix", "reverify",
                    "push", "merge", "task_done", "main_sync", "safe_cleanup")
        transitions = out.get("transitions", {})
        for phase in required:
            with self.subTest(phase=phase):
                record = transitions.get(phase)
                self.assertIsInstance(record, dict, f"missing durable transition: {phase}")
                self.assertTrue(record.get("receipt") or record.get("action_required"), record)

    def test_concurrent_resume_has_one_owner_and_one_structured_busy_result(self) -> None:
        self.seed_remote()
        self.run_driver("--initialize", "--task-uid", "task_" + "d" * 32,
                        "--stop-after", "route")
        command = [str(DRIVER), "--state", str(self.state), "--json", "--resume",
                   "--stop-after", "create_pr", "--expected-revision", str(self.workflow()["revision"]),
                   "--lease-token", self.workflow()["lease"]["token"]]
        env = self.driver_env()
        first = subprocess.Popen(command, text=True, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, env=env)
        second = subprocess.Popen(command, text=True, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, env=env)
        results = [first.communicate(timeout=10), second.communicate(timeout=10)]
        codes = [first.returncode, second.returncode]
        self.assertEqual([0, 75], sorted(codes))
        for (stdout, stderr), code in zip(results, codes):
            self.assertNotIn("Traceback", stderr)
            payload = json.loads(stdout)
            self.assertIn("revision", payload)
            self.assertIn("token", payload["lease"])
            if code == 75:
                self.assertEqual("lease_busy", payload["blocker"]["class"])

    def test_corrupt_checkpoint_recovers_or_returns_structured_blocker(self) -> None:
        self.seed_remote()
        self.run_driver("--initialize", "--task-uid", "task_" + "e" * 32,
                        "--stop-after", "bootstrap")
        backup = self.state.with_suffix(self.state.suffix + ".bak")
        self.assertTrue(backup.exists(), "checkpoint must retain a recovery generation")
        self.state.write_text("{truncated")
        proc = self.run_raw("--resume", "--stop-after", "next")
        self.assertNotIn("Traceback", proc.stderr)
        self.assertIn(proc.returncode, {0, 75})
        out = json.loads(proc.stdout)
        self.assertTrue(out.get("recovered_from") or
                        out.get("blocker", {}).get("class") == "checkpoint_corrupt")

    def test_every_remote_and_local_effect_is_exactly_once_after_crash(self) -> None:
        operations = ("create_task", "update_comment", "merge", "task_done",
                      "main_sync", "safe_cleanup")
        for operation in operations:
            with self.subTest(operation=operation):
                self.state.unlink(missing_ok=True)
                self.seed_remote()
                flag = "--crash-after-remote" if operation in {"create_task", "update_comment", "merge"} else "--crash-after-local"
                self.run_driver("--initialize", "--task-uid", "task_" + "f" * 32,
                                flag, operation, expect=86)
                out = self.run_driver("--resume", "--run-to-completion")
                journal = out.get("transition_journal", {}).get(operation)
                self.assertIsInstance(journal, dict, f"missing crash journal for {operation}")
                self.assertEqual("committed", journal["state"])
                self.assertEqual(1, journal["effect_count"])

    def test_slice_deadline_retry_terminal_and_integration_barrier(self) -> None:
        self.seed_remote()
        self.run_driver("--initialize", "--task-uid", "task_" + "1" * 32,
                        "--fixture-slices", "1", "--stop-after", "dispatch")
        out = self.run_driver("--resume", "--advance-clock", "PT31M",
                              "--run-to-completion", expect=75)
        self.assertTrue(any(item.get("state") in {"failed_retryable", "superseded", "terminal"}
                            for item in out["slices"]))
        self.assertNotEqual("OPEN", out["pr_state"], "PR cannot open before slice integration")
        self.assertNotEqual("MERGED", out["pr_state"])
        self.assertEqual("slice_integration_pending", out["blocker"]["class"])

    def test_active_hold_or_unready_checks_are_hard_merge_barriers(self) -> None:
        for remote_gate in ({"checks_ready": False}, {"active_hold": True}):
            with self.subTest(remote_gate=remote_gate):
                self.state.unlink(missing_ok=True)
                self.seed_remote(gate=remote_gate)
                out = self.run_driver("--initialize", "--task-uid", "task_" + "2" * 32,
                                      "--run-to-completion", expect=75)
                self.assertEqual([], self.remote_state()["merge_receipts"])
                self.assertNotEqual("MERGED", out["pr_state"])
                self.assertIn(out["blocker"]["class"], {"checks_unready", "active_merge_hold"})

    def test_production_steps_map_real_helpers_or_wait_for_tpm_action(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "3" * 32,
                              "--run-to-completion", expect=75, test_adapter=False)
        record = out["transitions"][out["next_action"]]
        self.assertTrue(record.get("helper_invocation") and record.get("action_required"))
        self.assertNotEqual("MERGED", out["pr_state"])
        self.assertNotEqual("done", out["task_state"])
        self.assertNotEqual("cleaned", out["worktree_state"])

    def test_complete_action_ingests_only_stage_bound_live_readback(self) -> None:
        self.seed_remote()
        first = self.run_driver("--initialize", "--task-uid", "task_" + "4" * 32,
                                "--run-to-completion", expect=75, test_adapter=False)
        action = first["transitions"]["bootstrap"]["action_required"]
        self.assertIsInstance(action, dict)
        for key in ("action_id", "phase", "command", "receipt_schema", "readback_validator"):
            self.assertIn(key, action)
        forged = self.write_receipt("forged", {"phase": "bootstrap", "verified": True})
        rejected = self.run_raw("--resume", "--complete-action", "bootstrap",
                                "--receipt-file", str(forged), "--expected-revision", str(first["revision"]),
                                "--lease-token", first["lease"]["token"], test_adapter=False)
        self.assertEqual(75, rejected.returncode)
        self.assertEqual("invalid_action_receipt", json.loads(rejected.stdout)["blocker"]["class"])
        current = self.workflow()
        self_signed = self.write_receipt("bootstrap", {
            "schema": action["receipt_schema"], "action_id": action["action_id"],
            "phase": "bootstrap", "command": action["command"],
            "exit_code": 0, "readback": {"task_uid": first["task_uid"],
                                           "worktree": "/definitely/not/a/real/worktree"},
            "validator": action["readback_validator"],
        })
        rejected = self.run_driver("--resume", "--complete-action", "bootstrap",
                                   "--receipt-file", str(self_signed),
                                   "--expected-revision", str(current["revision"]),
                                   "--lease-token", current["lease"]["token"],
                                   expect=75, test_adapter=False)
        self.assertIn(rejected["blocker"]["class"],
                      {"invalid_live_readback", "live_validator_required"})
        self.assertEqual("bootstrap", rejected["next_action"])
        self.assertNotIn("bootstrap", rejected["transition_journal"])

    def test_production_action_loop_requires_live_validator_ack_each_stage(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "5" * 32,
                              "--run-to-completion", expect=75, test_adapter=False)
        phase = out["next_action"]
        action = out["transitions"][phase]["action_required"]
        receipt = self.write_receipt(phase, {
            "schema": action["receipt_schema"], "action_id": action["action_id"],
            "phase": phase, "command": action["command"], "exit_code": 0,
            "readback": {"task_uid": out["task_uid"], "phase": phase},
            "validator": action["readback_validator"],
        })
        rejected = self.run_driver("--resume", "--complete-action", phase,
                                   "--receipt-file", str(receipt),
                                   "--expected-revision", str(out["revision"]),
                                   "--lease-token", out["lease"]["token"],
                                   expect=75, test_adapter=False)
        self.assertIn(rejected["blocker"]["class"],
                      {"invalid_live_readback", "live_validator_required"})
        self.assertEqual(phase, rejected["next_action"])

    def test_normal_hold_is_inactive_and_watch_uses_same_source_fresh_gate(self) -> None:
        self.seed_remote(gate={"checks_ready": True, "head_oid": "a" * 40,
                               "observed_at": "2026-07-11T00:00:00Z"})
        out = self.run_driver("--initialize", "--task-uid", "task_" + "6" * 32,
                              "--pr-head", "a" * 40, "--stop-after", "record_pr")
        hold = out["canonical_hold"]
        self.assertIn(hold["disposition"], {"inactive", "normal"})
        remote_hold = self.remote_state()["comments"][0]
        self.assertEqual(hold["disposition"], remote_hold["disposition"])
        self.assertEqual(hold["node_id"], remote_hold["node_id"])
        watched = self.run_driver("--resume", "--stop-after", "watch")
        self.assertEqual(watched["transitions"]["watch"]["gate_source"], hold["source"])
        self.assertEqual("a" * 40, watched["transitions"]["watch"]["head_oid"])
        self.assertIn("hold_node_id", watched["transitions"]["watch"])
        self.assertEqual(hold["node_id"], watched["transitions"]["watch"]["hold_node_id"])
        self.assertTrue(watched["transitions"]["watch"]["live_gate_readback"])

    def test_wait_can_only_clear_with_canonical_evidence_receipt(self) -> None:
        self.seed_remote()
        blocked = self.run_driver("--initialize", "--task-uid", "task_" + "7" * 32,
                                  "--inject-blocker", "human_approval_required", expect=75)
        bypass = self.run_raw("--resume", "--resolve-blocker", "human_approval_required",
                              "--stop-after", "next")
        self.assertEqual(75, bypass.returncode)
        self.assertEqual("canonical_evidence_required", json.loads(bypass.stdout)["blocker"]["class"])
        receipt = self.write_receipt("approval", {
            "schema": "tpm-canonical-evidence/v1", "task_uid": blocked["task_uid"],
            "blocker": "human_approval_required", "source": "github_task_issue_comment",
            "node_id": "IC_kw-test", "readback_verified": True,
        })
        rejected = self.run_driver("--resume", "--complete-action", "resolve_wait",
                                   "--receipt-file", str(receipt), "--stop-after", "next",
                                   expect=75)
        self.assertIn(rejected["blocker"]["class"],
                      {"invalid_action_receipt", "canonical_evidence_readback_failed"})

    def test_helper_actions_have_executable_schema_and_nontrivial_validator(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "8" * 32,
                              "--run-to-completion", expect=75, test_adapter=False)
        action = out["transitions"][out["next_action"]]["action_required"]
        self.assertIsInstance(action, dict)
        self.assertIsInstance(action["command"], list)
        self.assertTrue(action["command"][0])
        self.assertIsInstance(action["receipt_schema"], str)
        self.assertNotIn(action["readback_validator"], {True, "true", "verified_true"})
        self.assertIn("required_readback_fields", action)

    def test_missing_or_nonexecutable_helper_is_blocked_before_action(self) -> None:
        self.seed_remote()
        proc = self.run_raw("--initialize", "--task-uid", "task_" + "8" * 32,
                            "--describe-actions", test_adapter=False)
        self.assertEqual(0, proc.returncode, proc.stderr)
        described = json.loads(proc.stdout)
        for phase, record in described["actions"].items():
            with self.subTest(phase=phase):
                if record.get("status") == "action_required":
                    command = record["command"]
                    executable = (ROOT / command[0]) if "/" in command[0] else None
                    resolved = str(executable) if executable and executable.exists() else shutil.which(command[0])
                    self.assertTrue(resolved, command)
                    self.assertTrue(os.access(resolved, os.X_OK), command)
                else:
                    self.assertEqual("helper_unavailable", record["blocker"]["class"])

    def test_fixture_live_receipt_validator_is_actually_invoked(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "8" * 32,
                              "--run-to-completion", expect=75, test_adapter=False)
        action = out["transitions"]["bootstrap"]["action_required"]
        worktree = self.dir / "real-worktree"
        worktree.mkdir()
        receipt = self.write_receipt("live-bootstrap", {
            "schema": action["receipt_schema"], "action_id": action["action_id"],
            "phase": "bootstrap", "command": action["command"], "exit_code": 0,
            "readback": {"task_uid": out["task_uid"], "worktree": str(worktree)},
            "validator": action["readback_validator"],
        })
        log = self.dir / "validator-call.json"
        self.run_driver("--resume", "--complete-action", "bootstrap",
                        "--receipt-file", str(receipt),
                        "--expected-revision", str(out["revision"]),
                        "--lease-token", out["lease"]["token"],
                        expect=75, test_adapter=False,
                        extra_env={"TPM_LIVE_RECEIPT_VALIDATOR": str(LIVE_VALIDATOR),
                                   "TPM_LIVE_VALIDATOR_LOG": str(log)})
        self.assertTrue(log.exists(), "declared fixture validator must be executed")
        invocation = json.loads(log.read_text())
        self.assertIn(str(receipt), invocation["argv"])

    def test_resume_requires_expected_revision_and_lease_token(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "9" * 32,
                              "--stop-after", "route")
        stale = self.run_raw("--resume", "--expected-revision", str(out["revision"] - 1),
                             "--lease-token", out["lease"]["token"], "--stop-after", "next")
        self.assertEqual(75, stale.returncode)
        self.assertEqual("revision_conflict", json.loads(stale.stdout)["blocker"]["class"])
        wrong_lease = self.run_raw("--resume", "--expected-revision", str(out["revision"]),
                                   "--lease-token", "wrong", "--stop-after", "next")
        self.assertEqual(75, wrong_lease.returncode)
        self.assertEqual("lease_token_mismatch", json.loads(wrong_lease.stdout)["blocker"]["class"])
        missing = self.run_raw("--resume", "--stop-after", "next")
        self.assertEqual(75, missing.returncode)
        self.assertEqual("cas_required", json.loads(missing.stdout)["blocker"]["class"])

    def test_update_comment_persists_intent_before_remote_and_recovers_readback(self) -> None:
        self.seed_remote()
        self.run_driver("--initialize", "--task-uid", "task_" + "a" * 32,
                        "--pr-head", "a" * 40, "--stop-after", "record_pr")
        self.run_driver("--resume", "--pr-head", "b" * 40,
                        "--crash-after-remote", "update_comment",
                        "--stop-after", "record_pr", expect=86)
        crashed = self.workflow()
        self.assertIn("update_comment", crashed["remote_journal"],
                      "intent must be durable before the remote mutation")
        journal = crashed["remote_journal"]["update_comment"]
        self.assertIn(journal["state"], {"intent", "acted"})
        calls = len([x for x in self.remote_state()["calls"] if x["operation"] == "update_comment"])
        out = self.run_driver("--resume", "--stop-after", "record_pr")
        self.assertEqual(calls, len([x for x in self.remote_state()["calls"]
                                    if x["operation"] == "update_comment"]))
        self.assertEqual("committed", out["remote_journal"]["update_comment"]["state"])

    def test_slice_receipts_drive_retry_terminal_and_integrated_barrier(self) -> None:
        self.seed_remote()
        out = self.run_driver("--initialize", "--task-uid", "task_" + "b" * 32,
                              "--fixture-slices", "1", "--stop-after", "dispatch")
        slice_id = out["slices"][0]["id"]
        out = self.run_driver("--resume", "--advance-clock", "PT31M",
                              "--stop-after", "dispatch")
        replacements = [x for x in out["slices"] if x.get("supersedes") == slice_id]
        self.assertTrue(replacements, "deadline must schedule a replacement slice")
        replacement = replacements[0]
        returned = self.write_receipt("slice-return", {
            "schema": "tpm-slice-return/v1", "slice_id": replacement["id"],
            "attempt": replacement["attempt"], "payload_digest": "a" * 64,
            "status": "returned",
        })
        rejected = self.run_driver("--resume", "--complete-slice", replacement["id"],
                                   "--receipt-file", str(returned), "--stop-after", "integrate",
                                   expect=75)
        self.assertEqual("slice_live_readback_required", rejected["blocker"]["class"])
        self.assertNotEqual("integrated", next(x for x in rejected["slices"]
                                                if x["id"] == replacement["id"])["state"])

    def test_scheduler_action_has_durable_delivery_and_survives_process_exit(self) -> None:
        self.seed_remote()
        first = self.run_driver("--initialize", "--task-uid", "task_" + "c" * 32,
                                "--fixture-slices", "1", "--stop-after", "dispatch")
        dispatch = first["transitions"]["dispatch"]
        self.assertIn("schedule", dispatch)
        schedule = dispatch["schedule"]
        for key in ("delivery_id", "wake_at", "receipt_schema", "slice_id"):
            self.assertIn(key, schedule)
        ack = self.dir / "delivery-ack.json"
        proc = self.run_raw("--serve", "--once", "--delivery-ack-file", str(ack),
                            "--expected-revision", str(first["revision"]),
                            "--lease-token", first["lease"]["token"],
                            clock="2026-07-11T00:31:00Z",
                            extra_env={"TPM_SCHEDULER_DELIVERY_ADAPTER": str(DELIVERY_ADAPTER),
                                       "TPM_DELIVERY_ADAPTER_LOG": str(self.dir / "delivery-old.log")})
        self.assertIn(proc.returncode, {0, 75}, proc.stderr)
        self.assertTrue(ack.exists(), "scheduler must emit a durable delivery acknowledgement")
        delivered = json.loads(ack.read_text())
        self.assertEqual(schedule["delivery_id"], delivered["delivery_id"])
        self.assertEqual("delivered", delivered["status"])

    def test_nonbootstrap_stage_requires_schema_valid_live_readback_to_advance(self) -> None:
        classes = ("route", "verify", "review", "create_pr", "watch", "merge",
                   "task_done", "safe_cleanup")
        for phase in classes:
            with self.subTest(phase=phase):
                self.state.unlink(missing_ok=True)
                self.seed_remote()
                initial = self.run_driver("--initialize", "--task-uid", "task_" + "d" * 32,
                                          "--run-to-completion", expect=75,
                                          test_adapter=False)
                described = self.run_driver("--initialize", "--task-uid", "task_" + "d" * 32,
                                             "--describe-actions", test_adapter=False)
                action = described["actions"][phase]
                self.assertEqual("action_required", action.get("status"),
                                 f"{phase} needs an executable or typed production action")
                state = self.workflow()
                state["phase"] = PHASES[PHASES.index(phase) - 1]
                state["next_action"] = phase
                state["status"] = "action_required"
                state["transitions"] = {phase: {"action_required": action}}
                self.state.write_text(json.dumps(state))
                readback = {"schema": f"tpm-{phase}-readback/v1", "phase": phase,
                            "task_uid": state["task_uid"], "repo": "eng-cc/oasis7",
                            "worktree": str(self.dir), "pr": 2198, "head": "a" * 40,
                            "epoch": "gate-1", "receipt_digest": "b" * 64}
                receipt = self.write_receipt(f"stage-{phase}", {
                    "schema": action["receipt_schema"], "action_id": action["action_id"],
                    "phase": phase, "command": action["command"], "exit_code": 0,
                    "validator": action["readback_validator"], "readback": readback})
                log = self.dir / f"validator-{phase}.json"
                out = self.run_driver("--resume", "--complete-action", phase,
                                      "--receipt-file", str(receipt),
                                      "--expected-revision", str(state["revision"]),
                                      "--lease-token", state["lease"]["token"],
                                      expect=75, test_adapter=False,
                                      extra_env={"TPM_LIVE_RECEIPT_VALIDATOR": str(LIVE_VALIDATOR),
                                                 "TPM_LIVE_VALIDATOR_LOG": str(log),
                                                 "TPM_LIVE_VALIDATOR_RESPONSE": json.dumps(
                                                     {"ok": True, "readback": readback})})
                self.assertNotEqual(phase, out["next_action"])
                self.assertEqual("committed", out["transition_journal"][phase]["state"])

    def test_serve_honors_wake_and_requires_real_delivery_adapter_ack(self) -> None:
        self.seed_remote()
        first = self.run_driver("--initialize", "--task-uid", "task_" + "e" * 32,
                                "--fixture-slices", "1", "--stop-after", "dispatch")
        ack = self.dir / "serve-ack.json"
        early = self.run_raw("--serve", "--once", "--delivery-ack-file", str(ack),
                             "--expected-revision", str(first["revision"]),
                             "--lease-token", first["lease"]["token"],
                             clock="2026-07-11T00:00:01Z")
        self.assertEqual(75, early.returncode)
        self.assertFalse(ack.exists(), "delivery before wake_at is forbidden")
        current = self.workflow()
        log = self.dir / "delivery-adapter.json"
        due = self.run_raw("--serve", "--once", "--delivery-ack-file", str(ack),
                           "--expected-revision", str(current["revision"]),
                           "--lease-token", current["lease"]["token"],
                           clock="2026-07-11T00:31:00Z",
                           extra_env={"TPM_SCHEDULER_DELIVERY_ADAPTER": str(DELIVERY_ADAPTER),
                                      "TPM_DELIVERY_ADAPTER_LOG": str(log)})
        self.assertIn(due.returncode, {0, 75})
        self.assertTrue(log.exists(), "due delivery must invoke the configured adapter")
        self.assertTrue(ack.exists())

    def test_every_resume_mutation_requires_cas(self) -> None:
        bogus = self.write_receipt("cas-bogus", {})
        mutations = (("--run-to-completion",), ("--stop-after", "route"),
                     ("--complete-action", "bootstrap", "--receipt-file", str(bogus)),
                     ("--complete-slice", "missing", "--receipt-file", str(bogus)))
        for index, mutation in enumerate(mutations):
            with self.subTest(mutation=mutation):
                self.state.unlink(missing_ok=True)
                self.seed_remote()
                self.run_driver("--initialize", "--task-uid", "task_" + str(index) * 32,
                                "--stop-after", "bootstrap")
                proc = self.run_raw("--resume", *mutation)
                self.assertEqual(75, proc.returncode)
                self.assertEqual("cas_required", json.loads(proc.stdout)["blocker"]["class"])

    def test_human_evidence_requires_digest_and_authority_bound_live_comment(self) -> None:
        task_uid = "task_" + "f" * 32
        body = "approved canonical evidence"
        comment = {"node_id": "IC_authorized", "repo": "eng-cc/oasis7", "issue": 2198,
                   "task_uid": task_uid, "blocker": "human_approval_required",
                   "author": "unauthorized-user", "created_at": "2026-07-11T00:00:00Z",
                   "body": body}
        self.seed_remote(comments=[comment])
        blocked = self.run_driver("--initialize", "--task-uid", task_uid,
                                  "--inject-blocker", "human_approval_required", expect=75)
        missing_digest = self.write_receipt("human-no-digest", {
            "schema": "tpm-canonical-evidence/v1", "task_uid": task_uid,
            "source": "github_task_issue_comment", "node_id": comment["node_id"],
            "readback_verified": True})
        rejected = self.run_driver("--resume", "--complete-action", "resolve_wait",
                                   "--receipt-file", str(missing_digest),
                                   "--expected-revision", str(blocked["revision"]),
                                   "--lease-token", blocked["lease"]["token"], expect=75)
        self.assertIn(rejected["blocker"]["class"],
                      {"evidence_digest_required", "unauthorized_evidence_author"})

    def test_resume_authority_must_come_from_task_truth_with_required_permission(self) -> None:
        task_uid = "task_" + "a" * 32
        for author, permission, should_pass in (("self-claimed", "write", False),
                                                ("task-owner", "triage", False),
                                                ("task-owner", "maintain", True)):
            with self.subTest(author=author, permission=permission):
                self.state.unlink(missing_ok=True)
                body = f"resume approval by {author}"
                comment = {"node_id": f"IC_{author}_{permission}", "repo": "eng-cc/oasis7",
                           "issue": 2198, "task_uid": task_uid,
                           "blocker": "human_approval_required", "author": author,
                           "author_permission": permission,
                           "created_at": "2026-07-11T00:00:00Z", "body": body}
                self.seed_remote(comments=[comment])
                blocked = self.run_driver("--initialize", "--task-uid", task_uid,
                                          "--inject-blocker", "human_approval_required", expect=75)
                state = self.workflow()
                state["task_truth"] = {"resume_authorities": ["task-owner"],
                                       "required_resume_permission": "maintain",
                                       "repo": "eng-cc/oasis7", "issue": 2198}
                self.state.write_text(json.dumps(state))
                receipt = self.write_receipt(f"authority-{author}-{permission}", {
                    "schema": "tpm-canonical-evidence/v1", "task_uid": task_uid,
                    "source": "github_task_issue_comment", "node_id": comment["node_id"],
                    "authority": author, "repo": "eng-cc/oasis7", "issue": 2198,
                    "created_at": comment["created_at"],
                    "digest": hashlib.sha256(body.encode()).hexdigest(),
                    "readback_verified": True})
                proc = self.run_raw("--resume", "--complete-action", "resolve_wait",
                                    "--receipt-file", str(receipt),
                                    "--expected-revision", str(state["revision"]),
                                    "--lease-token", state["lease"]["token"])
                if should_pass:
                    self.assertEqual(0, proc.returncode, proc.stderr or proc.stdout)
                else:
                    self.assertEqual(75, proc.returncode)
                    self.assertIn(json.loads(proc.stdout)["blocker"]["class"],
                                  {"resume_authority_not_allowed", "resume_permission_insufficient"})

    def test_due_delivery_ack_drives_one_cas_bound_consumer_resume(self) -> None:
        self.seed_remote()
        first = self.run_driver("--initialize", "--task-uid", "task_" + "b" * 32,
                                "--fixture-slices", "1", "--stop-after", "dispatch")
        phase_before = first["phase"]
        ack = self.dir / "consumer-ack.json"
        log = self.dir / "consumer-delivery.json"
        out = self.run_driver("--serve", "--once", "--delivery-ack-file", str(ack),
                              "--expected-revision", str(first["revision"]),
                              "--lease-token", first["lease"]["token"],
                              clock="2026-07-11T00:31:00Z",
                              extra_env={"TPM_SCHEDULER_DELIVERY_ADAPTER": str(DELIVERY_ADAPTER),
                                         "TPM_DELIVERY_ADAPTER_LOG": str(log)})
        schedule = out["transitions"]["dispatch"]["schedule"]
        self.assertIn("consumer_run", schedule,
                      "delivery ack must be consumed by one CAS-bound resume")
        consumer = schedule["consumer_run"]
        self.assertEqual(schedule["delivery_id"], consumer["delivery_id"])
        self.assertEqual(first["revision"], consumer["expected_revision"])
        self.assertEqual(first["lease"]["token"], consumer["lease_token"])
        self.assertIn(consumer["status"], {"completed", "external_wait", "action_required"})
        self.assertNotEqual(phase_before, consumer["result_phase"])

    def test_typed_actions_have_stage_operation_contracts(self) -> None:
        self.seed_remote()
        described = self.run_driver("--initialize", "--task-uid", "task_" + "c" * 32,
                                     "--describe-actions", test_adapter=False)
        for phase in ("route", "dispatch", "execute", "integrate", "review", "fix", "reverify", "push"):
            with self.subTest(phase=phase):
                action = described["actions"][phase]
                self.assertEqual("typed_tpm_action", action["action_type"])
                for key in ("operation_schema", "producer_surface", "required_inputs", "stage_validator"):
                    self.assertIn(key, action)
                self.assertIsInstance(action["operation_schema"], str)
                self.assertTrue(action["producer_surface"])
                self.assertTrue(action["required_inputs"])
                self.assertTrue(action.get("command") or action.get("dispatch_operation"))
                self.assertNotIn(action["stage_validator"], {"echo", "generic", "verified_true"})

    def test_every_typed_action_command_executes_and_returns_structured_result(self) -> None:
        self.seed_remote()
        described = self.run_driver("--initialize", "--task-uid", "task_" + "d" * 32,
                                     "--describe-actions", test_adapter=False)
        for phase, action in described["actions"].items():
            if action.get("action_type") != "typed_tpm_action":
                continue
            with self.subTest(phase=phase):
                payload = {key: f"test-{key}" for key in action["required_inputs"]}
                proc = subprocess.run(action["command"], cwd=str(ROOT), text=True,
                                      input=json.dumps(payload), capture_output=True)
                self.assertEqual(0, proc.returncode, proc.stderr or proc.stdout)
                result = json.loads(proc.stdout)
                self.assertEqual(action["dispatch_operation"], result["action"]["operation"])
                self.assertEqual(phase, result["action"]["phase"])
                self.assertIsInstance(result["result"], dict)
                self.assertIn("status", result["result"])

    def test_bootstrap_reads_canonical_task_resume_authority_truth(self) -> None:
        task_uid = "task_" + "e" * 32
        authority = {"repo": "eng-cc/oasis7", "issue": 2198, "task_uid": task_uid,
                     "issue_author": "request-owner", "issue_author_permission": "maintain",
                     "resume_authorities": ["request-owner", "release-owner"],
                     "required_resume_permission": "maintain",
                     "evidence_node_id": "IC_task_authority", "readback_verified": True}
        self.seed_remote(task_authority=authority)
        out = self.run_driver("--initialize", "--task-uid", task_uid,
                              "--stop-after", "bootstrap")
        self.assertIn("task_truth", out,
                      "bootstrap must persist canonical resume authority truth")
        self.assertEqual(authority["resume_authorities"], out["task_truth"]["resume_authorities"])
        self.assertEqual("maintain", out["task_truth"]["required_resume_permission"])
        self.assertEqual("IC_task_authority", out["task_truth"]["authority_evidence_node_id"])

        self.state.unlink(missing_ok=True)
        self.seed_remote(failures=[{"operation": "read_task_authority", "status": 503,
                                    "remaining": 1, "retry_after": 5}])
        blocked = self.run_driver("--initialize", "--task-uid", task_uid,
                                  "--stop-after", "bootstrap", expect=75)
        self.assertEqual("external_wait", blocked["status"])
        self.assertEqual("task_authority_readback_failed", blocked["blocker"]["class"])

    def test_production_bootstrap_completion_reads_canonical_authority_without_main_fixture(self) -> None:
        task_uid = "task_" + "f" * 32
        authority = {"ok": True, "result": {"repo": "eng-cc/oasis7", "issue": 2198,
                     "task_uid": task_uid, "resume_authorities": ["task-owner"],
                     "required_resume_permission": "maintain",
                     "evidence_node_id": "IC_production_authority", "readback_verified": True}}
        for fail in (False, True):
            with self.subTest(fail=fail):
                self.state.unlink(missing_ok=True)
                self.seed_remote()
                first = self.run_driver("--initialize", "--task-uid", task_uid,
                                        "--run-to-completion", expect=75, test_adapter=False)
                action = first["transitions"]["bootstrap"]["action_required"]
                worktree = self.dir / ("production-worktree-fail" if fail else "production-worktree")
                worktree.mkdir()
                subprocess.run(["git", "init", "-q", str(worktree)], check=True)
                mapping = worktree / ".pm/github-project-sync/tasks.json"
                mapping.parent.mkdir(parents=True)
                mapping.write_text(json.dumps({"tasks": {task_uid: {"issue": 2198}}}))
                helper = worktree / "scripts/pm/github-project-workflow.sh"
                helper.parent.mkdir(parents=True)
                helper.write_text("#!/bin/sh\nexit 0\n")
                helper.chmod(0o755)
                receipt = self.write_receipt(f"production-bootstrap-{fail}", {
                    "schema": action["receipt_schema"], "action_id": action["action_id"],
                    "phase": "bootstrap", "command": action["command"], "exit_code": 0,
                    "validator": action["readback_validator"],
                    "readback": {"task_uid": task_uid, "worktree": str(worktree)}})
                env = {"TPM_CANONICAL_TASK_AUTHORITY_READER": str(AUTHORITY_READER),
                       "TPM_AUTHORITY_READER_RESPONSE": json.dumps(authority)}
                if fail:
                    env["TPM_AUTHORITY_READER_FAIL"] = "1"
                out = self.run_driver("--resume", "--complete-action", "bootstrap",
                                      "--receipt-file", str(receipt),
                                      "--expected-revision", str(first["revision"]),
                                      "--lease-token", first["lease"]["token"],
                                      expect=75, test_adapter=False, extra_env=env)
                if fail:
                    self.assertEqual("external_wait", out["status"])
                    self.assertEqual("task_authority_readback_failed", out["blocker"]["class"])
                else:
                    self.assertIn("task_truth", out,
                                  "production bootstrap must persist canonical authority truth")
                    self.assertEqual(["task-owner"], out["task_truth"]["resume_authorities"])
                    self.assertEqual("maintain", out["task_truth"]["required_resume_permission"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
