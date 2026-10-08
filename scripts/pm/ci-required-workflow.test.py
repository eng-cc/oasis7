#!/usr/bin/env python3
"""Transport, exact-attempt readback and resource regression witnesses."""
import argparse
import hashlib
import importlib.util
import json
import os
import re
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import ci_required_workflow as workflow

ROOT = Path(__file__).resolve().parents[2]
BASE = "dd04c28ab9aa850a6123d11544f1790717fb741c"


class TransportTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.directory = Path(self.scratch.name)
        self.root = self.directory / "source"
        self.root.mkdir()
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        workflow.git(self.root, "config", "user.email", "ci-fixture@example.invalid")
        workflow.git(self.root, "config", "user.name", "CI Fixture")
        for name in workflow.contracts.EXECUTOR_CONTRACT_PATHS:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(subprocess.check_output(["git", "show", BASE + ":" + name], cwd=ROOT))
        (self.root / "README.md").write_text("base\n")
        workflow.git(self.root, "add", ".")
        workflow.git(self.root, "commit", "-qm", "known legacy authority")
        self.base = workflow.git(self.root, "rev-parse", "HEAD")
        (self.root / "README.md").write_text("source\n")
        workflow.git(self.root, "commit", "-qam", "source")
        self.head = workflow.git(self.root, "rev-parse", "HEAD")
        tree = workflow.git(self.root, "rev-parse", "HEAD^{tree}")
        self.merge = workflow.git(self.root, "commit-tree", tree, "-p", self.base, "-p", self.head, "-m", "exact synthetic M")
        workflow.git(self.root, "checkout", "--detach", self.merge)
        self.transport = self.directory / "transport"
        self.authority = self.directory / "legacy"
        workflow.archive(self.root, self.base, self.authority)
        raw = workflow.command(["python3", str(self.authority / "scripts/plan-rust-required-scope.py"),
                                "--event-name", "pull_request", "--changed-path", "doc/testing/prd.md"], self.root)
        scope = dict(line.split("=", 1) for line in raw.splitlines())
        scope.update(base_oid=self.base, head_oid=self.head, source_scope_base=self.base, integration_base_oid=self.base,
                     integration_base=self.base, source_head=self.head, task_uid="", planner_authority_oid=self.base)
        self.scope = self.directory / "scope.json"
        workflow.write(self.scope, scope)
        self.env = {"GITHUB_REPOSITORY": "eng-cc/oasis7", "GITHUB_RUN_ID": "101", "GITHUB_RUN_ATTEMPT": "2",
                    "GITHUB_EVENT_NAME": "pull_request", "GITHUB_WORKFLOW_SHA": self.merge, "OASIS7_RUN_HEAD_SHA": self.head,
                    "GITHUB_OUTPUT": str(self.directory / "outputs")}
        self.environment = patch.dict(os.environ, self.env)
        self.environment.start()

    def tearDown(self):
        self.environment.stop()
        self.scratch.cleanup()

    def prepare(self):
        args = argparse.Namespace(root=str(self.root), target=str(self.root), transport=str(self.transport), scope=str(self.scope),
                                  authority=self.base, policy="", projection="", integration="", run_mode="", request_key="",
                                  pr_number=1, planner_authority=self.base)
        workflow.prepare(args)
        return workflow.read(self.transport / "transport.json")

    def materialize(self):
        target = self.directory / "destination"
        target.mkdir()
        subprocess.run(["git", "init", "-q", str(target)], check=True)
        workflow.git(target, "remote", "add", "origin", str(self.root))
        return argparse.Namespace(root=str(target), transport=str(self.transport), authority_root=str(self.directory / "trusted"),
                                  artifact_id="9", tested_sha=self.merge, tested_tree=workflow.git(self.root, "rev-parse", "HEAD^{tree}"),
                                  authority_oid=self.base, transport_digest=workflow.digest(self.transport / "transport.json"))

    def test_known_legacy_freezes_no_workers_and_transfers_exact_local_merge(self):
        payload = self.prepare()
        self.assertEqual(payload["execution_layout"], "required-serial/v1")
        self.assertEqual(workflow.read(self.transport / "schedule.json")["workers"], [])
        args = self.materialize()
        with patch.object(workflow, "artifact_identity"), patch.object(workflow, "attempt_jobs", return_value=[]):
            workflow.materialize(args)
        self.assertEqual(workflow.git(Path(args.root), "rev-parse", "HEAD"), self.merge)
        self.assertEqual(workflow.git(Path(args.root), "rev-list", "--parents", "-n", "1", "HEAD").split()[1:], [self.base, self.head])
        self.assertEqual(workflow.git(Path(args.authority_root), "rev-parse", "HEAD"), self.base)

    def test_transport_mutation_and_wrong_attempt_are_rejected(self):
        self.prepare()
        args = self.materialize()
        (self.transport / "scope.json").write_text("{}\n")
        with patch.object(workflow, "artifact_identity"):
            with self.assertRaisesRegex(ValueError, "digest"):
                workflow.materialize(args)
        os.environ["GITHUB_RUN_ATTEMPT"] = "3"
        with self.assertRaisesRegex(ValueError, "run_attempt"):
            workflow.runtime_identity(workflow.read(self.transport / "identity.json"))

    def test_wrong_frozen_merge_is_rejected(self):
        self.prepare()
        args = self.materialize()
        args.tested_sha = self.head
        with patch.object(workflow, "artifact_identity"):
            with self.assertRaisesRegex(ValueError, "tested_sha"):
                workflow.materialize(args)

    def test_coherent_identity_and_selection_mutation_cannot_replace_frozen_manifest(self):
        self.prepare()
        args = self.materialize()
        manifest = workflow.read(self.transport / "transport.json")
        identity = workflow.read(self.transport / "identity.json")
        identity["source_head_sha"] = self.base
        workflow.write(self.transport / "identity.json", identity)
        manifest["identity"] = identity
        manifest["members"]["identity.json"] = workflow.digest(self.transport / "identity.json")
        scope = workflow.read(self.transport / "scope.json")
        scope.update(head_oid=self.base, source_head=self.base)
        workflow.write(self.transport / "scope.json", scope)
        manifest["members"]["scope.json"] = workflow.digest(self.transport / "scope.json")
        selection = workflow.read(self.transport / "selection.json")
        selection["planner_output"] = scope
        workflow.write(self.transport / "selection.json", selection)
        manifest["members"]["selection.json"] = workflow.digest(self.transport / "selection.json")
        workflow.write(self.transport / "transport.json", manifest)
        with self.assertRaisesRegex(ValueError, "independently frozen"):
            workflow.materialize(args)

    def test_bundle_is_bounded_and_missing_prerequisites_fail(self):
        self.prepare()
        bundle = self.transport / "source.bundle"
        self.assertLess(bundle.stat().st_size, 4096)
        args = self.materialize()
        workflow.git(Path(args.root), "remote", "remove", "origin")
        with patch.object(workflow, "artifact_identity"):
            with self.assertRaises(subprocess.CalledProcessError):
                workflow.materialize(args)

    def test_unknown_legacy_contract_never_enters_serial(self):
        (self.authority / "scripts/ci-tests.sh").write_text("echo bypass\n")
        with self.assertRaises(ValueError):
            workflow.authority_context(self.authority)

    def test_remote_tested_commit_needs_no_delta_bundle(self):
        workflow.git(self.root, "checkout", "--detach", self.head)
        os.environ.update(GITHUB_WORKFLOW_SHA=self.head, OASIS7_RUN_HEAD_SHA=self.head, GITHUB_EVENT_NAME="push")
        self.merge = self.head
        self.prepare()
        self.assertFalse((self.transport / "source.bundle").exists())
        args = self.materialize()
        with patch.object(workflow, "artifact_identity"), patch.object(workflow, "attempt_jobs", return_value=[]):
            workflow.materialize(args)
        self.assertEqual(workflow.git(Path(args.root), "rev-parse", "HEAD"), self.head)

    def test_artifact_readback_binds_run_name_and_id(self):
        good = {"id": 9, "name": "required-transport-101-2", "expired": False, "workflow_run": {"id": 101}}
        with patch.object(workflow, "command", return_value=json.dumps(good)):
            workflow.artifact_identity(9, good["name"])
        for changed in ({**good, "id": 8}, {**good, "name": "required-transport-101-1"}, {**good, "workflow_run": {"id": 100}}):
            with patch.object(workflow, "command", return_value=json.dumps(changed)):
                with self.assertRaises(ValueError):
                    workflow.artifact_identity(9, good["name"])

    def test_parallel_empty_prepare_materialize_and_finalize(self):
        for name in workflow.contracts.PARALLEL_EXECUTOR_CONTRACT_PATHS:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((ROOT / name).read_bytes())
        # This transport witness isolates Cargo admission from orchestration;
        # the real trusted checker and authority fixtures have separate suites.
        (self.root / "scripts/pm/trusted_cargo_scope.py").write_text(
            "import subprocess\ndef run_scope(*args,**kwargs): return subprocess.CompletedProcess([],0,'{}','')\n")
        workflow.git(self.root, "add", ".")
        workflow.git(self.root, "commit", "-qm", "new explicit parallel authority")
        self.base = workflow.git(self.root, "rev-parse", "HEAD")
        leaf = self.root / "doc/testing/prd.md"
        leaf.parent.mkdir(parents=True)
        leaf.write_text("documentation only\n")
        workflow.git(self.root, "add", ".")
        workflow.git(self.root, "commit", "-qm", "docs narrow")
        self.head = workflow.git(self.root, "rev-parse", "HEAD")
        tree = workflow.git(self.root, "rev-parse", "HEAD^{tree}")
        self.merge = workflow.git(self.root, "commit-tree", tree, "-p", self.base, "-p", self.head, "-m", "exact parallel M")
        workflow.git(self.root, "checkout", "--detach", self.merge)
        os.environ.update(GITHUB_WORKFLOW_SHA=self.merge, OASIS7_RUN_HEAD_SHA=self.head)
        raw = workflow.command(["python3", str(self.root / "scripts/plan-rust-required-scope.py"), "--event-name", "pull_request",
                                "--base-ref", self.base, "--head-ref", self.head], self.root)
        scope = dict(line.split("=", 1) for line in raw.splitlines())
        scope.update(base_oid=self.base, head_oid=self.head, source_scope_base=self.base, integration_base_oid=self.base,
                     integration_base=self.base, source_head=self.head, task_uid="", planner_authority_oid=self.base)
        workflow.write(self.scope, scope)
        with patch.object(workflow, "driver") as baseline:
            payload = self.prepare()
            self.assertEqual(baseline.call_args.args[3], "required-plan-baseline")
        self.assertEqual(payload["execution_layout"], "required-parallel/v1")
        self.assertEqual(workflow.read(self.transport / "schedule.json")["workers"], {})
        args = self.materialize()
        with patch.object(workflow, "artifact_identity"), patch.object(workflow, "attempt_jobs", return_value=[]):
            workflow.materialize(args)
        identity = workflow.read(self.transport / "identity.json")
        job = {"job_name": "required-plan", "workflow_run_id": 101, "run_attempt": 2, "head_sha": self.head,
               "status": "completed", "conclusion": "success", "check_app_id": 15368}
        finish = argparse.Namespace(root=args.root, transport=args.transport, authority_root=args.authority_root,
                                    results=str(self.directory / "worker-results"))
        original = workflow.command
        def command(argv, *other, **kwargs):
            if argv[:2] == ["gh", "api"]:
                return json.dumps([{"artifacts": []}])
            return original(argv, *other, **kwargs)
        with patch.object(workflow, "attempt_jobs", return_value=[job]), patch.object(workflow, "command", side_effect=command), patch.object(workflow, "driver"):
            workflow.finalize(finish)
        # Coherently rehashing schedule/input cannot replace independently joined
        # planner/config/projection facts during gate replay.
        selection = workflow.read(self.transport / "selection.json")
        authority = Path(args.authority_root)
        for field in ("planner_digest", "config_digest", "impact_projection_digest"):
            wrong = {**identity, field: "f" * 64}
            with self.assertRaisesRegex(ValueError, "digest differs"):
                workflow.replay_selection(authority, self.transport, selection, wrong)
        with patch.object(workflow, "command", return_value="\n".join(line for line in raw.splitlines() if not line.startswith("run_doc_checker_contracts="))):
            with self.assertRaisesRegex(ValueError, "recomputation differs"):
                workflow.replay_selection(authority, self.transport, selection, identity)
        # A selected worker's transport and actual check proof take the same
        # finalization path; execution itself is represented by an API fixture.
        workflow.git(self.root, "checkout", "--detach", self.head)
        (self.root / "README.md").write_text("site update\n")
        workflow.git(self.root, "commit", "-qam", "selected site contract")
        self.head = workflow.git(self.root, "rev-parse", "HEAD")
        tree = workflow.git(self.root, "rev-parse", "HEAD^{tree}")
        self.merge = workflow.git(self.root, "commit-tree", tree, "-p", self.base, "-p", self.head, "-m", "selected worker M")
        workflow.git(self.root, "checkout", "--detach", self.merge)
        os.environ.update(GITHUB_WORKFLOW_SHA=self.merge, OASIS7_RUN_HEAD_SHA=self.head)
        raw = original(["python3", str(self.root / "scripts/plan-rust-required-scope.py"), "--event-name", "pull_request",
                        "--base-ref", self.base, "--head-ref", self.head], self.root)
        scope = dict(line.split("=", 1) for line in raw.splitlines())
        scope.update(base_oid=self.base, head_oid=self.head, source_scope_base=self.base, integration_base_oid=self.base,
                     integration_base=self.base, source_head=self.head, task_uid="", planner_authority_oid=self.base)
        workflow.write(self.scope, scope)
        self.transport = self.directory / "selected-transport"
        with patch.object(workflow, "driver"):
            self.prepare()
        schedule = workflow.read(self.transport / "schedule.json")
        self.assertEqual(set(schedule["workers"]), {"contracts"})
        identity = schedule["identity"]
        selected_root = self.directory / "selected-destination"
        selected_root.mkdir()
        subprocess.run(["git", "init", "-q", str(selected_root)], check=True)
        workflow.git(selected_root, "remote", "add", "origin", str(self.root))
        args = argparse.Namespace(root=str(selected_root), transport=str(self.transport), authority_root=str(self.directory / "selected-trusted"),
                                  artifact_id="9", tested_sha=self.merge, tested_tree=tree, authority_oid=self.base,
                                  transport_digest=workflow.digest(self.transport / "transport.json"))
        with patch.object(workflow, "artifact_identity"), patch.object(workflow, "attempt_jobs", return_value=[]):
            workflow.materialize(args)
        completed = {"schema": "oasis7-required-worker-result/v1", "worker": "contracts", "identity": identity,
                     "plan_digest": schedule["plan_digest"], "command_definition_digest": schedule["command_definition_digest"],
                     "completed": schedule["workers"]["contracts"]}
        jobs = [{**job, "head_sha": self.head}, {**job, "head_sha": self.head, "job_name": "required-work (contracts)"}]
        artifact = {"id": 33, "name": "required-worker-101-2-contracts"}
        def api_command(argv, *other, **kwargs):
            return json.dumps([{"artifacts": [artifact]}]) if argv[:2] == ["gh", "api"] else original(argv, *other, **kwargs)
        real_run = subprocess.run
        def download(argv, **kwargs):
            if argv[:3] == ["gh", "run", "download"]:
                output = Path(argv[argv.index("--dir") + 1])
                output.mkdir(parents=True)
                workflow.write(output / "worker.json", completed)
                return SimpleNamespace(returncode=0)
            return real_run(argv, **kwargs)
        finish = argparse.Namespace(root=args.root, transport=args.transport, authority_root=args.authority_root,
                                    results=str(self.directory / "selected-results"))
        with patch.object(workflow, "attempt_jobs", return_value=jobs), patch.object(workflow, "command", side_effect=api_command), \
             patch.object(workflow, "artifact_identity"), patch.object(workflow.subprocess, "run", side_effect=download), patch.object(workflow, "driver"):
            workflow.finalize(finish)


class WorkflowWiringTests(unittest.TestCase):
    def test_actual_planner_outputs_survive_job_output_and_gate_env_boundary(self):
        source = (ROOT / ".github/workflows/rust.yml").read_text()
        plan = source.split("  required-plan:", 1)[1].split("    steps:", 1)[0]
        published = dict(re.findall(
            r"^      ([a-z0-9_]+): \$\{\{ steps\.scope\.outputs\.([a-z0-9_]+) \}\}", plan, re.M))
        consumed = set(re.findall(r"needs\.required-plan\.outputs\.((?:run_|needs_)[a-z0-9_]+)", source))
        gate = source.split("      - name: Run required test tier", 1)[1].split("        run: |", 1)[0]
        env_pattern = r"^          (OASIS7_CI_(?:RUN_|NEEDS_)[A-Z0-9_]+): \$\{\{ needs\.required-plan\.outputs\.([a-z0-9_]+) \}\}"
        environment = dict(re.findall(env_pattern, gate, re.M))
        deleted_gate = "\n".join(line for line in gate.splitlines() if "OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS:" not in line)
        deleted_environment = dict(re.findall(env_pattern, deleted_gate, re.M))
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            planners = [ROOT / "scripts/plan-rust-required-scope.py"]
            for revision in (BASE, "89ef370f66a83cba50bafa9930c74e31360e5ba0"):
                legacy_root = directory / revision
                workflow.archive(ROOT, revision, legacy_root)
                planners.append(legacy_root / "scripts/plan-rust-required-scope.py")
            for planner in planners:
                for changed_path in (".github/workflows/rust.yml", "doc/testing/prd.md"):
                    with self.subTest(planner=planner, changed_path=changed_path):
                        output = directory / "outputs"
                        output.unlink(missing_ok=True)
                        subprocess.run(["python3", str(planner), "--event-name", "pull_request", "--changed-path", changed_path,
                                        "--github-output", str(output)], cwd=ROOT, check=True, capture_output=True)
                        actual = dict(line.split("=", 1) for line in output.read_text().splitlines())
                        selectors = {key for key in actual if key.startswith(("run_", "needs_"))}
                        self.assertTrue(consumed <= published.keys(), f"unpublished outputs: {sorted(consumed - published.keys())}")
                        self.assertEqual(selectors, {key for key in published if key.startswith(("run_", "needs_"))})
                        job_outputs = {key: actual.get(value, "") for key, value in published.items()}
                        for key in selectors:
                            self.assertEqual(published[key], key)
                            self.assertIn(job_outputs[key], ("true", "false"))
                            self.assertEqual(job_outputs[key], actual[key])
                        with patch.dict(os.environ, {}, clear=True):
                            expected_env = workflow.selectors({"planner_output": {**actual, "source_scope_base": "base", "head_oid": "head",
                                                                                "integration_base_oid": "base"}})
                        event_owned = {"OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE", "OASIS7_CI_RUN_PROVIDER_LIVE_GATE"}
                        expected_keys = {key for key in expected_env if key.startswith(("OASIS7_CI_RUN_", "OASIS7_CI_NEEDS_"))} - event_owned
                        self.assertEqual(set(environment), expected_keys)
                        with self.assertRaises(AssertionError):
                            self.assertEqual(set(deleted_environment), expected_keys)
                        for name, key in environment.items():
                            self.assertEqual(job_outputs[key], expected_env[name], name)
                        self.assertEqual(job_outputs["run_oasis7_required_tests"], "true" if changed_path.startswith(".github") else "false")

    def test_dag_preserves_stable_gate_and_platform_boundary(self):
        source = (ROOT / ".github/workflows/rust.yml").read_text()
        plan = source[source.index("  required-plan:"):source.index("  required-work:")]
        worker = source[source.index("  required-work:"):source.index("  required-gate:")]
        gate = source[source.index("  required-gate:"):source.index("  v1-reuse-validation-only:")]
        self.assertIn("needs: [required-plan, required-work]", gate)
        self.assertIn("if: always()", gate)
        self.assertNotIn("windows-package-rollout-behavior", gate)
        self.assertNotIn("Build attempt-scoped required-plan v2", plan)
        self.assertIn("max-parallel: 6", worker)
        self.assertIn("fail-fast: false", worker)
        self.assertIn("artifact-ids: ${{ needs.required-plan.outputs.transport_id }}", worker)
        self.assertIn("integration_module.PLAN_V2_MEMBER", gate)
        self.assertIn("trusted_executor_context=request_contract.resolve_execution_layout", gate)
        for name in ("windows-package-rollout-behavior", "testnet-packages-macos-arm64-contract", "public-testnet-fleet-health-contract"):
            platform = source[source.index("  " + name + ":"):]
            boundary = re.search(r"\n  [a-zA-Z0-9_-]+:", platform)
            platform = platform[:boundary.start()] if boundary else platform
            self.assertIn("needs: required-plan", platform)

    def test_binary_install_is_exact_and_has_no_source_fallback(self):
        source = (ROOT / ".github/workflows/rust.yml").read_text()
        required = source[source.index("  required-plan:"):source.index("  v1-reuse-validation-only:")]
        self.assertNotIn("cargo install trunk", required)
        self.assertIn("trunk-x86_64-unknown-linux-gnu.tar.gz", required)
        self.assertIn("f2b4680cd239693a646a2795e4633c625328d7b2a044fbe749fa3a2fe9e7036b", required)
        self.assertIn("sha256sum --check -", required)
        self.assertIn('grep -Fqx "trunk ${TRUNK_VERSION}"', required)
        self.assertIn("actions/cache/restore@v4", required)
        self.assertIn("matrix.worker == 'web'", required)

    def test_profile_runs_on_exact_M_and_stops_on_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            plan = {"items": [{"id": "native", "command": ["cargo", "test"], "command_digest": "d", "package": "p",
                               "profile": "native", "target": "native", "features": []}], "plan_id": "p",
                    "trusted_authority": {"toolchain": "1.96.0"}, "integration_base": "b", "source_head": "h", "tested_tree": "t"}
            with patch.object(workflow, "profile_plan", return_value=plan), patch.object(workflow.subprocess, "run") as run:
                run.return_value.returncode = 1
                with self.assertRaisesRegex(ValueError, "profile item failed"):
                    workflow.execute_profile(directory, directory, directory, directory)
                self.assertEqual(run.call_args.kwargs["cwd"], directory)
                self.assertEqual(run.call_args.kwargs["env"]["OASIS7_WASM_BUILD_STD"], "0")
                self.assertEqual(workflow.read(directory / "cargo-package-profile-results.json")[0]["status"], "failed")

    def test_exact_attempt_job_namespace_and_empty_skipped_aggregate(self):
        identity = {"repository": "eng-cc/oasis7", "run_id": 101, "run_attempt": 2,
                    "run_head_sha": "a" * 40, "event_name": "pull_request"}
        with tempfile.TemporaryDirectory() as temporary:
            authority = Path(temporary)
            helper = authority / "scripts/pm/integration_ci.py"
            helper.parent.mkdir(parents=True)
            helper.write_text('''def attempt_execution_jobs(repository,run_id,attempt,workflow_sha,app_id,*,job_names,require_completed,expected_event):
    assert (repository,run_id,attempt,workflow_sha,app_id,require_completed,expected_event)==('eng-cc/oasis7',101,2,'a'*40,15368,True,'pull_request')
    return [{'job_name':name,'status':'completed','conclusion':'success','workflow_run_id':run_id,'run_attempt':attempt,'head_sha':workflow_sha,'check_app_id':app_id} for name in job_names]
''')
            base = {"name": "required-plan", "status": "completed", "conclusion": "success"}
            native = {"name": "required-work (native)", "status": "completed", "conclusion": "success"}
            def check(jobs, names):
                with patch.object(workflow, "command", return_value=json.dumps([{"jobs": jobs}])):
                    return workflow.attempt_jobs(identity, names, authority)
            self.assertEqual(len(check([base, native], ["required-plan", native["name"]])), 2)
            skipped = {"name": "required-work", "status": "completed", "conclusion": "skipped", "check_run_url": None}
            self.assertEqual(len(check([base, skipped], ["required-plan"])), 1)
            for bad, names in (([base, native, {**native, "name": "required-work (unknown)"}], ["required-plan", native["name"]]),
                               ([base, native, native], ["required-plan", native["name"]]), ([base], ["required-plan", native["name"]]),
                               ([base, {**skipped, "conclusion": "success"}], ["required-plan"]),
                               ([base, {**native, "name": "required-work (unexpected)"}], ["required-plan"])):
                with self.assertRaises(ValueError):
                    check(bad, names)

    def test_profile_completion_requires_full_results_and_actual_in_progress_gate(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = root / "workers"
            results.mkdir()
            identity = {"repository": "eng-cc/oasis7", "run_id": 101, "run_attempt": 2, "workflow_sha": "w",
                        "base_sha": "b", "source_head_sha": "h", "tested_tree": "t", "task_uid": "task", "pr_number": 1}
            plan = {"items": [], "integration_base": "b", "source_head": "h", "tested_tree": "t"}
            def profile_plan(_root, _authority, _transport, output):
                output.mkdir(parents=True, exist_ok=True)
                workflow.write(output / "cargo-package-profile-plan.json", plan)
                return plan
            gate = {"status": "in_progress", "check_app_id": 15368, "check_run_id": 91, "job_name": "required-gate",
                    "workflow_run_id": 101, "run_attempt": 2, "head_sha": "w"}
            fake = SimpleNamespace(attempt_execution_jobs=lambda *a, **kw: [gate])
            with patch.dict(os.environ, GITHUB_WORKFLOW_REF="eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main"), \
                 patch.dict(__import__('sys').modules, integration_ci=fake), patch.object(workflow, "profile_plan", side_effect=profile_plan), \
                 patch.object(workflow, "command", return_value='{"status":"passed"}'):
                workflow.complete_profile(root, root, root, results, {"workers": {}}, identity, [{"check_app_id": 15368}])
                envelope = workflow.read(root / "output/cargo-package-profile/cargo-package-profile-envelope.json")
                self.assertEqual((envelope["check_run_id"], envelope["run_attempt"], envelope["tested_tree"]), (91, 2, "t"))
                for mutation in ({"status": "completed"}, {"job_name": "required-plan"}, {"run_attempt": 1}, {"head_sha": "different"}, {"check_app_id": 1}):
                    fake.attempt_execution_jobs = lambda *a, mutation=mutation, **kw: [{**gate, **mutation}]
                    with self.assertRaisesRegex(ValueError, "in-progress gate"):
                        workflow.complete_profile(root, root, root, results, {"workers": {}}, identity, [{"check_app_id": 15368}])
                with self.assertRaisesRegex(ValueError, "evidence is missing"):
                    workflow.complete_profile(root, root, root, results, {"workers": {"native": []}}, identity, [{"check_app_id": 15368}])


if __name__ == "__main__":
    unittest.main()
