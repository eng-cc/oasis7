#!/usr/bin/env python3
"""RED contract for a real, uninterrupted TPM production supervisor.

Unlike tpm-workflow-driver.test.py this suite never enables the test-only
adapter and never accepts echo/fixture receipts as production evidence.
"""

from __future__ import annotations

import json
import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PM = ROOT / "scripts/pm"
REGISTRY = PM / "tpm-live-validator-registry.py"
BUILDER = PM / "tpm-action-builder.py"
SUPERVISOR = PM / "tpm-workflow-supervisor.py"
COLLAB = PM / "tpm-collaboration-protocol.py"
WAKE = PM / "tpm-wake-owner.py"
DRIVER = PM / "tpm-workflow-driver.py"
STAGING = PM / "tpm-production-staging.test.py"

MECHANICAL_PHASES = (
    "bootstrap", "freeze", "verify", "closeout", "create_pr", "record_pr",
    "comment", "watch", "push", "merge", "merge_receipt", "task_done",
    "main_sync", "safe_cleanup",
)
PROFESSIONAL_PHASES = ("route", "dispatch", "execute", "integrate", "review", "fix", "reverify")


def run_json(argv: list[str], *, env: dict[str, str] | None = None,
             input_value: dict | None = None, expected: int = 0) -> dict:
    proc = subprocess.run(
        argv,
        cwd=ROOT,
        env=env,
        input=None if input_value is None else json.dumps(input_value),
        text=True,
        capture_output=True,
    )
    if proc.returncode != expected:
        raise AssertionError(
            f"expected exit {expected}, got {proc.returncode}\nstdout={proc.stdout}\nstderr={proc.stderr}"
        )
    return json.loads(proc.stdout)


class ProductionSupervisorContract(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.dir = Path(self.temp.name)
        subprocess.run(["git", "init", "-q", "-b", "main", str(self.dir / "repo")], check=True)
        self.repo = self.dir / "repo"
        subprocess.run(["git", "-C", str(self.repo), "config", "user.email", "qa@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.name", "QA Contract"], check=True)
        (self.repo / "README.md").write_text("production supervisor contract\n")
        subprocess.run(["git", "-C", str(self.repo), "add", "README.md"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-qm", "seed"], check=True)
        self.state = self.dir / "workflow.json"
        self.task_uid = "task_" + "d" * 32
        self.base_env = os.environ.copy()
        for key in (
            "TPM_ADAPTER_MODE", "TPM_GITHUB_ADAPTER", "TPM_LIVE_RECEIPT_VALIDATOR",
            "TPM_DELIVERY_ADAPTER", "TPM_SCHEDULER_DELIVERY_ADAPTER",
            "TPM_COLLABORATION_RUNTIME_VALIDATOR", "TPM_WAKE_RUNTIME_ADAPTER",
            "TPM_WAKE_FIXTURE_STATE", "TPM_VERTICAL_SLICE_FIXTURE",
            "TPM_WAKE_RUNTIME_ATTESTATION", "TPM_QA_WRONG_ACK_FIELD",
        ):
            self.base_env.pop(key, None)

    def test_repo_owned_validator_registry_blocks_every_phase_without_independent_live_proof(self) -> None:
        described = run_json([str(REGISTRY), "--describe", "--json"], env=self.base_env)
        expected = set(MECHANICAL_PHASES + PROFESSIONAL_PHASES)
        self.assertEqual(expected, set(described["validators"]))
        for phase, entry in described["validators"].items():
            with self.subTest(phase=phase):
                executable = ROOT / entry["executable"]
                self.assertTrue(executable.is_file())
                self.assertTrue(executable.stat().st_mode & stat.S_IXUSR)
                self.assertNotIn("fixtures", executable.parts)
                probe = run_json(
                    [str(REGISTRY), "--probe", phase, "--repo", str(self.repo), "--json"],
                    env=self.base_env, expected=75,
                )
                self.assertEqual("capability_blocked", probe["status"])
                self.assertEqual("independent_phase_proof_required", probe["blocker"]["class"])
                self.assertIn(probe["source"], {"git", "github", "task_truth", "pr_gate", "filesystem"})

    def test_action_builder_emits_complete_executable_argv_for_mechanical_phases(self) -> None:
        for phase in MECHANICAL_PHASES:
            with self.subTest(phase=phase):
                action = run_json([
                    str(BUILDER), "--phase", phase, "--task-uid", self.task_uid,
                    "--repo", str(self.repo), "--state", str(self.state), "--dry-run", "--json",
                ], env=self.base_env)
                self.assertEqual("valid", action["schema_validation"])
                self.assertEqual(phase, action["phase"])
                self.assertTrue(action["argv"])
                self.assertFalse(any("FIXTURE" in arg or "fixtures/" in arg for arg in action["argv"]))
                executable = Path(action["argv"][0])
                self.assertTrue(executable.is_absolute(), action)
                self.assertTrue(executable.exists(), action)
                self.assertTrue(action["required_inputs_bound"], action)

    def test_single_supervisor_rejects_noncanonical_checkpoint_before_execution(self) -> None:
        result = run_json([
            str(SUPERVISOR), "--initialize", "--task-uid", self.task_uid,
            "--repo", str(self.repo), "--state", str(self.state),
            "--run-to-completion", "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("capability_blocked", result["status"])
        self.assertEqual("noncanonical_checkpoint_path", result["blocker"]["class"])
        self.assertFalse(result.get("production_passed", False))
        self.assertFalse(result.get("automatic", False))
        self.assertEqual("blocked", result.get("capability_status"))
        self.assertFalse(self.state.exists())
        self.assertFalse(self.state.with_suffix(self.state.suffix + ".wake-owner.json").exists())

    def test_initialize_does_not_overwrite_existing_checkpoint(self) -> None:
        original = {"schema": "sentinel/v1", "revision": 41, "lease_token": "owned"}
        self.state.write_text(json.dumps(original))
        blocked = run_json([
            str(SUPERVISOR), "--initialize", "--task-uid", self.task_uid,
            "--repo", str(self.repo), "--state", str(self.state),
            "--run-to-completion", "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("checkpoint_already_exists", blocked["blocker"]["class"])
        self.assertEqual(original, json.loads(self.state.read_text()))

    def test_initialize_requires_canonical_task_bound_checkpoint_path(self) -> None:
        arbitrary = self.dir / "caller-selected.json"
        blocked = run_json([
            str(SUPERVISOR), "--initialize", "--task-uid", self.task_uid,
            "--repo", str(self.repo), "--state", str(arbitrary),
            "--run-to-completion", "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("noncanonical_checkpoint_path", blocked["blocker"]["class"])
        self.assertFalse(arbitrary.exists())

    def test_initialize_cannot_route_before_trusted_bootstrap_completion(self) -> None:
        canonical = self.repo / ".pm" / "tasks" / f"{self.task_uid}.workflow.json"
        blocked = run_json([
            str(SUPERVISOR), "--initialize", "--task-uid", self.task_uid,
            "--repo", str(self.repo), "--state", str(canonical),
            "--run-to-completion", "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("bootstrap", blocked["phase"])
        self.assertEqual("capability_blocked", blocked["status"])
        self.assertNotIn("next_action", blocked)

    def test_missing_dispatch_attestation_is_capability_blocked(self) -> None:
        source = SUPERVISOR.read_text(encoding="utf-8")
        self.assertNotRegex(
            source,
            r"status[\"']\s*:\s*[\"']external_wait[\"'][\s\S]{0,500}dispatch_attestation_unavailable",
        )
        self.assertRegex(
            source,
            r"status[\"']\s*:\s*[\"']capability_blocked[\"'][\s\S]{0,500}dispatch_attestation_unavailable",
        )

    def test_terminal_authority_binds_full_canonical_identity(self) -> None:
        source = SUPERVISOR.read_text(encoding="utf-8")
        for field in (
            "repository", "canonical_worktree", "task_branch", "default_branch",
            "merge_receipt", "main_sync_authority", "cleanup_authority",
        ):
            self.assertIn(field, source, f"terminal authority must bind {field}")

    def test_environment_cannot_enable_fixture_mode_in_production_driver(self) -> None:
        remote = self.dir / "production-driver-remote.json"
        remote.write_text(json.dumps({"calls": [], "tasks": [], "prs": []}))
        state = self.dir / "production-driver-state.json"
        env = self.base_env.copy()
        env.update({
            "TPM_ADAPTER_MODE": "test_only",
            "TPM_GITHUB_ADAPTER": str(PM / "fixtures/tpm-workflow/fake-github.py"),
            "TPM_GITHUB_STATE": str(remote),
            "TPM_LIVE_RECEIPT_VALIDATOR": str(
                PM / "fixtures/tpm-workflow/live-receipt-validator.py"
            ),
        })
        before_remote = remote.read_bytes()
        blocked = run_json([
            str(DRIVER), "--state", str(state), "--json", "--initialize",
            "--task-uid", self.task_uid, "--run-to-completion",
        ], env=env, expected=75)
        self.assertIn(blocked["status"], {"action_required", "capability_blocked"})
        self.assertNotEqual("completed", blocked["status"])
        self.assertNotEqual("produced", blocked["status"])
        self.assertFalse(blocked.get("production_passed", False))
        self.assertEqual(before_remote, remote.read_bytes())

    def test_professional_actions_require_collaboration_return_evidence_not_echo(self) -> None:
        for phase in PROFESSIONAL_PHASES:
            with self.subTest(phase=phase):
                plan = run_json([
                    str(COLLAB), "--plan", phase, "--task-uid", self.task_uid,
                    "--repo", str(self.repo), "--json",
                ], env=self.base_env)
                self.assertIn(plan["operation"], {"spawn", "wait", "retry", "replace", "integrate"})
                self.assertGreater(plan["timeout_seconds"], 0)
                self.assertGreaterEqual(plan["max_attempts"], 1)
                self.assertIn("return_evidence_schema", plan)
                echo = {
                    "status": "produced", "phase": phase, "task_uid": self.task_uid,
                    "payload": {"inputs": "echoed"},
                }
                rejected = run_json([
                    str(COLLAB), "--validate-return", phase, "--task-uid", self.task_uid,
                    "--json",
                ], env=self.base_env, input_value=echo, expected=75)
                self.assertEqual("collaboration_return_required", rejected["blocker"]["class"])

    def test_caller_cannot_drive_watch_graph_through_production_debug_operation(self) -> None:
        head1, head2 = "a" * 40, "b" * 40
        for outcome in ("pending", "actionable", "ready", "fixed"):
            with self.subTest(outcome=outcome):
                blocked = run_json([
                    str(SUPERVISOR), "--decide-watch", outcome, "--head", head1,
                    "--reviewed-head", head1, "--gate-epoch", "epoch-1", "--json",
                ], env=self.base_env, expected=75)
                self.assertEqual("capability_blocked", blocked["status"])
                self.assertEqual("unsupported_production_operation", blocked["blocker"]["class"])
                self.assertNotIn("next_action", blocked)

    def test_repo_fixture_env_cannot_enable_production_wake(self) -> None:
        owner = self.dir / "wake-owner.json"
        runtime_state = self.dir / "wake-runtime.json"
        runtime_env = self.base_env.copy()
        runtime_env["TPM_ADAPTER_MODE"] = "test-only"
        runtime_env["TPM_WAKE_RUNTIME_ADAPTER"] = str(
            PM / "fixtures/tpm-workflow/wake-runtime-adapter.py"
        )
        runtime_env["TPM_WAKE_FIXTURE_STATE"] = str(runtime_state)
        blocked = run_json([
            str(WAKE), "install", "--state", str(self.state), "--owner", str(owner),
            "--task-uid", self.task_uid, "--json",
        ], env=runtime_env, expected=75)
        self.assertEqual("capability_blocked", blocked["status"])
        self.assertEqual("wake_runtime_unavailable", blocked["blocker"]["class"])
        self.assertFalse(owner.exists())
        self.assertFalse(runtime_state.exists())

    def test_wake_json_mutation_is_not_delivery_without_runtime_owner(self) -> None:
        owner = self.dir / "wake-owner-no-runtime.json"
        installed = run_json([
            str(WAKE), "install", "--state", str(self.state), "--owner", str(owner),
            "--task-uid", self.task_uid, "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("capability_blocked", installed["status"])
        self.assertEqual("wake_runtime_unavailable", installed["blocker"]["class"])
        self.assertFalse(installed.get("installed", False))

    def test_mechanical_remote_phases_require_real_independently_observed_effects(self) -> None:
        remote = self.dir / "remote.git"
        subprocess.run(["git", "init", "-q", "--bare", str(remote)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "remote", "add", "origin", str(remote)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "push", "-q", "-u", "origin", "main"], check=True)
        before_remote = subprocess.check_output(
            ["git", "--git-dir", str(remote), "show-ref"], text=True
        )
        for phase in ("create_pr", "push", "merge", "task_done", "safe_cleanup"):
            with self.subTest(phase=phase):
                proc = subprocess.run([
                    str(PM / "tpm-mechanical-action.py"), "--phase", phase,
                    "--task-uid", self.task_uid, "--repo", str(self.repo),
                    "--state", str(self.state), "--json",
                ], env=self.base_env, text=True, capture_output=True)
                self.assertEqual(75, proc.returncode, proc.stdout)
                result = json.loads(proc.stdout)
                self.assertEqual("capability_blocked", result["status"])
                self.assertNotEqual("produced", result["status"])
        after_remote = subprocess.check_output(
            ["git", "--git-dir", str(remote), "show-ref"], text=True
        )
        self.assertEqual(before_remote, after_remote)

    def test_production_validator_rejects_forged_receipt_without_live_fact(self) -> None:
        forged = self.dir / "forged-merge.json"
        forged.write_text(json.dumps({"status": "MERGED", "source": "github", "forged": True}))
        rejected = run_json([
            str(REGISTRY), "--validate", "merge", "--receipt", str(forged),
            "--repo", str(self.repo), "--json",
        ], env=self.base_env, expected=75)
        self.assertIn(rejected["blocker"]["class"], {
            "live_fact_unavailable", "receipt_untrusted", "github_readback_required",
            "independent_phase_proof_required",
        })
        self.assertNotEqual("live_readback", rejected.get("status"))

    def test_collaboration_forged_ack_agent_and_digest_are_rejected(self) -> None:
        forged = {
            "schema": "tpm-collaboration-return/v1", "status": "returned",
            "phase": "review", "task_uid": self.task_uid,
            "dispatch_ack": "invented", "agent_id": "agent-invented",
            "attempt": 1, "started_at": "2026-01-01T00:00:00Z",
            "returned_at": "2026-01-01T00:00:01Z", "artifact_digest": "a" * 64,
            # A caller-authored non-empty object is not a runtime attestation.
            "runtime_attestation": {"issuer": "caller", "forged": True},
        }
        rejected = run_json([
            str(COLLAB), "--validate-return", "review", "--task-uid", self.task_uid,
            "--json",
        ], env=self.base_env, input_value=forged, expected=75)
        self.assertIn(rejected["blocker"]["class"], {
            "runtime_attestation_required", "collaboration_return_untrusted",
        })

    def test_repo_fixture_env_cannot_enable_production_collaboration_validation(self) -> None:
        artifact = self.dir / "review.json"
        artifact.write_text('{"finding":"real"}\n')
        digest = __import__("hashlib").sha256(artifact.read_bytes()).hexdigest()
        validator = PM / "fixtures/tpm-workflow/collaboration-runtime-validator.py"
        env = self.base_env.copy()
        env["TPM_ADAPTER_MODE"] = "test-only"
        env["TPM_COLLABORATION_RUNTIME_VALIDATOR"] = str(validator)
        returned = {
            "schema": "tpm-collaboration-return/v1", "status": "returned",
            "phase": "review", "task_uid": self.task_uid,
            "dispatch_ack": "runtime-dispatch-1", "agent_id": "runtime-agent-1",
            "attempt": 1, "started_at": "2026-01-01T00:00:00Z",
            "returned_at": "2026-01-01T00:00:01Z", "artifact_digest": digest,
            "artifact_path": str(artifact),
        }
        blocked = run_json([
            str(COLLAB), "--validate-return", "review", "--task-uid", self.task_uid,
            "--json",
        ], env=env, input_value=returned, expected=75)
        self.assertEqual("capability_blocked", blocked["status"])
        self.assertIn(blocked["blocker"]["class"], {
            "runtime_attestation_required", "collaboration_runtime_unavailable",
        })

    def test_caller_selected_collaboration_executable_is_not_a_production_trust_root(self) -> None:
        """An env-selected echo program must not attest its caller's own return."""
        artifact = self.dir / "caller-review.json"
        artifact.write_text('{"finding":"caller authored"}\n')
        digest = __import__("hashlib").sha256(artifact.read_bytes()).hexdigest()
        spoof = self.dir / "caller-collaboration-validator.py"
        spoof.write_text(
            "#!/usr/bin/env python3\n"
            "import json,sys\n"
            "v=json.load(sys.stdin)\n"
            "print(json.dumps({'status':'live_ack','dispatch_ack':v['dispatch_ack'],"
            "'agent_id':v['agent_id'],'artifact_digest':v['artifact_digest']}))\n"
        )
        spoof.chmod(0o755)
        env = self.base_env.copy()
        env["TPM_ADAPTER_MODE"] = "test-only"
        env["TPM_COLLABORATION_RUNTIME_VALIDATOR"] = str(spoof)
        returned = {
            "schema": "tpm-collaboration-return/v1", "status": "returned",
            "phase": "review", "task_uid": self.task_uid,
            "dispatch_ack": "caller-dispatch", "agent_id": "caller-agent",
            "attempt": 1, "started_at": "2026-01-01T00:00:00Z",
            "returned_at": "2026-01-01T00:00:01Z",
            "artifact_digest": digest, "artifact_path": str(artifact),
        }
        rejected = run_json([
            str(COLLAB), "--validate-return", "review", "--task-uid", self.task_uid,
            "--json",
        ], env=env, input_value=returned, expected=75)
        self.assertEqual("capability_blocked", rejected["status"])
        self.assertIn(rejected["blocker"]["class"], {
            "runtime_attestation_required", "caller_selected_trust_root_rejected",
        })

    def test_caller_receipt_cannot_prove_local_mechanical_phases(self) -> None:
        head = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "HEAD"], text=True
        ).strip()
        for phase in ("freeze", "verify", "reverify", "main_sync", "safe_cleanup"):
            with self.subTest(phase=phase):
                forged = self.dir / f"forged-{phase}.json"
                forged.write_text(json.dumps({
                    "phase": phase, "repo": str(self.repo.resolve()), "head": head,
                    "status": "passed", "forged": True,
                }))
                rejected = run_json([
                    str(REGISTRY), "--validate", phase, "--receipt", str(forged),
                    "--repo", str(self.repo), "--json",
                ], env=self.base_env, expected=75)
                self.assertIn(rejected["blocker"]["class"], {
                    "independent_phase_proof_required", "receipt_untrusted",
                    "live_fact_unavailable",
                })

    def test_caller_environment_cannot_install_or_deliver_wake(self) -> None:
        owner = self.dir / "caller-wake-owner.json"
        caller_env = self.base_env.copy()
        caller_env["TPM_WAKE_RUNTIME_ATTESTATION"] = "runtime-owned"
        rejected = run_json([
            str(WAKE), "install", "--state", str(self.state), "--owner", str(owner),
            "--task-uid", self.task_uid, "--json",
        ], env=caller_env, expected=75)
        self.assertEqual("capability_blocked", rejected["status"])
        self.assertEqual("wake_runtime_unavailable", rejected["blocker"]["class"])
        self.assertFalse(owner.exists())

    def test_caller_selected_wake_executable_is_not_a_production_trust_root(self) -> None:
        spoof = self.dir / "caller-wake-adapter.py"
        spoof.write_text(
            "#!/usr/bin/env python3\n"
            "import json,sys\n"
            "v=json.load(sys.stdin)\n"
            "print(json.dumps({'status':'live_ack','operation':v['operation'],"
            "'owner_id':'caller-owner'}))\n"
        )
        spoof.chmod(0o755)
        env = self.base_env.copy()
        env["TPM_ADAPTER_MODE"] = "test-only"
        env["TPM_WAKE_RUNTIME_ADAPTER"] = str(spoof)
        owner = self.dir / "caller-selected-wake.json"
        rejected = run_json([
            str(WAKE), "install", "--state", str(self.state), "--owner", str(owner),
            "--task-uid", self.task_uid, "--json",
        ], env=env, expected=75)
        self.assertEqual("capability_blocked", rejected["status"])
        self.assertIn(rejected["blocker"]["class"], {
            "wake_runtime_unavailable", "caller_selected_trust_root_rejected",
        })
        self.assertFalse(owner.exists())

    def test_vertical_slice_rejects_tampered_or_unrelated_resume_state(self) -> None:
        unrelated = self.dir / "unrelated"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-q", "-b",
            "task/unrelated", str(unrelated),
        ], check=True)
        state = self.dir / "tampered-state.json"
        state.write_text(json.dumps({
            "schema": "tpm-local-terminal-slice/v1", "status": "running",
            "phase": "main_sync", "task_uid": self.task_uid,
            "repo": str(self.repo), "worktree": str(unrelated),
            "branch": "task/unrelated", "head": "0" * 40,
            "completed": ["main_sync"],
        }))
        proc = subprocess.run([
            str(SUPERVISOR), "--resume", "--state", str(state),
            "--run-to-completion", "--json",
        ], env=self.base_env, text=True, capture_output=True)
        self.assertEqual(75, proc.returncode, proc.stdout + proc.stderr)
        self.assertTrue(unrelated.exists(), "tampered state must not clean an unrelated worktree")

    def test_cleanup_rejects_same_head_clean_unrelated_worktree_even_with_matching_git_facts(self) -> None:
        """Git consistency cannot replace canonical task/mapping/intent authority."""
        unrelated = self.dir / "same-head-unrelated"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-q", "-b",
            "unrelated-clean-branch", str(unrelated),
        ], check=True)
        head = subprocess.check_output(
            ["git", "-C", str(unrelated), "rev-parse", "HEAD"], text=True
        ).strip()
        state = self.dir / "forged-same-head-cleanup.json"
        state.write_text(json.dumps({
            "schema": "tpm-local-terminal-slice/v1", "status": "running",
            "phase": "main_sync", "task_uid": self.task_uid,
            "repo": str(self.repo), "worktree": str(unrelated),
            "branch": "unrelated-clean-branch", "head": head,
            "completed": ["main_sync"],
            # Caller-authored shapes are not trusted cleanup authority.
            "cleanup_intent": {"trusted": True},
            "merged_receipt": {"status": "MERGED", "head": head},
        }))
        rejected = run_json([
            str(SUPERVISOR), "--resume", "--state", str(state),
            "--run-to-completion", "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("capability_blocked", rejected["status"])
        self.assertIn(rejected["blocker"]["class"], {
            "production_resume_connector_unavailable", "terminal_slice_authority_untrusted",
        })
        self.assertTrue(unrelated.exists())
        branch = subprocess.run([
            "git", "-C", str(self.repo), "show-ref", "--verify", "--quiet",
            "refs/heads/unrelated-clean-branch",
        ])
        self.assertEqual(0, branch.returncode)

    def test_production_staging_without_live_transport_is_explicitly_blocked(self) -> None:
        result = run_json([
            str(STAGING), "--isolated-root", str(self.dir / "staging"), "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("capability_blocked", result["status"])
        self.assertEqual("production_staging_transport_unavailable", result["blocker"]["class"])
        self.assertFalse(result.get("production_passed", False))
        self.assertNotIn("final", result, "handwritten MERGED/done is not staging evidence")

    def test_fixture_receipts_are_physically_rejected_by_production_registry(self) -> None:
        fixture = PM / "fixtures/tpm-workflow/live-receipt-validator.py"
        rejected = run_json([
            str(REGISTRY), "--validate", "watch", "--receipt", str(fixture),
            "--repo", str(self.repo), "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("fixture_boundary_violation", rejected["blocker"]["class"])
        self.assertEqual(str(fixture.resolve()), rejected["rejected_path"])

    def test_production_sources_have_no_test_runtime_or_self_attestation_knobs(self) -> None:
        production = (
            DRIVER, SUPERVISOR, COLLAB, WAKE, BUILDER,
            PM / "tpm-mechanical-action.py", REGISTRY,
        )
        forbidden = (
            "TPM_", "os.environ", "os.getenv", "--crash-after", "--vertical-slice",
            "--fault", "--fixture", "--adapter", "--executable", "--validator",
            '"status":"produced"', '"status": "produced"',
            '"status":"live_readback"', '"status": "live_readback"',
        )
        for path in production:
            text = path.read_text(encoding="utf-8")
            with self.subTest(path=path):
                for token in forbidden:
                    self.assertNotIn(token, text, f"production trust boundary exposes {token}")
                self.assertNotIn("fixtures/", text)
                self.assertNotRegex(text, r"(?i)fixture\s+(mode|executable|entry|receipt|validator)")

    def test_repo_fixture_env_cannot_enable_production_vertical_slice(self) -> None:
        worktree = self.dir / "vertical-worktree"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-q", "-b",
            "task/vertical", str(worktree),
        ], check=True)
        before_main = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "main"], text=True
        ).strip()
        fixture_env = self.base_env.copy()
        fixture_env["TPM_ADAPTER_MODE"] = "test-only"
        fixture_env["TPM_VERTICAL_SLICE_FIXTURE"] = str(
            PM / "fixtures/tpm-workflow/vertical-slice-authority.py"
        )
        state = self.dir / "fixture-env-state.json"
        blocked = run_json([
            str(SUPERVISOR), "--vertical-slice", "main-sync-safe-cleanup",
            "--task-uid", self.task_uid, "--repo", str(self.repo),
            "--worktree", str(worktree), "--state", str(state), "--json",
        ], env=fixture_env, expected=75)
        self.assertEqual("capability_blocked", blocked["status"])
        self.assertTrue(worktree.exists())
        self.assertFalse(state.exists())
        self.assertEqual(before_main, subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "main"], text=True
        ).strip())

    def test_production_vertical_slice_is_capability_blocked_and_has_no_side_effects(self) -> None:
        worktree = self.dir / "production-vertical-worktree"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-q", "-b",
            "task/production-vertical", str(worktree),
        ], check=True)
        before_main = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "main"], text=True
        ).strip()
        mapping = self.repo / ".pm/github-project-sync/tasks.json"
        before_mapping = mapping.read_bytes() if mapping.exists() else None
        state = self.dir / "production-vertical-state.json"
        blocked = run_json([
            str(SUPERVISOR), "--vertical-slice", "main-sync-safe-cleanup",
            "--task-uid", self.task_uid, "--repo", str(self.repo),
            "--worktree", str(worktree), "--state", str(state), "--json",
        ], env=self.base_env, expected=75)
        self.assertEqual("capability_blocked", blocked["status"])
        self.assertIn(blocked["blocker"]["class"], {
            "terminal_slice_runtime_unavailable", "test_only_vertical_slice",
            "unsupported_production_operation",
        })
        self.assertEqual(before_main, subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "main"], text=True
        ).strip())
        self.assertTrue(worktree.exists())
        self.assertFalse(state.exists())
        self.assertEqual(before_mapping, mapping.read_bytes() if mapping.exists() else None)

    def test_any_env_selected_vertical_entry_is_ignored_without_side_effects(self) -> None:
        worktree = self.dir / "test-only-nonallowlisted"
        subprocess.run([
            "git", "-C", str(self.repo), "worktree", "add", "-q", "-b",
            "task/test-only-nonallowlisted", str(worktree),
        ], check=True)
        spoof = self.dir / "vertical-slice-authority.py"
        spoof.write_text("#!/usr/bin/env python3\n")
        spoof.chmod(0o755)
        env = self.base_env.copy()
        env["TPM_ADAPTER_MODE"] = "test-only"
        env["TPM_VERTICAL_SLICE_FIXTURE"] = str(spoof)
        blocked = run_json([
            str(SUPERVISOR), "--vertical-slice", "main-sync-safe-cleanup",
            "--task-uid", self.task_uid, "--repo", str(self.repo),
            "--worktree", str(worktree), "--state", str(self.dir / "bad-fixture.json"),
            "--json",
        ], env=env, expected=75)
        self.assertEqual("capability_blocked", blocked["status"])
        self.assertTrue(worktree.exists())
        self.assertFalse((self.dir / "bad-fixture.json").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
