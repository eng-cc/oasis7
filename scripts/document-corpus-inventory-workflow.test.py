#!/usr/bin/env python3
"""Trusted PM scope regressions for corpus source and sidecar endpoints."""
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import types
import unittest


ROOT = Path(__file__).resolve().parent.parent
PM = ROOT / "scripts/pm"
sys.path.insert(0, str(PM))
CORE_PATH = ROOT / "scripts/document_corpus.py"
POLICY_PATH = PM / "loop-policy.v1.json"


def module_from_path(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load test module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def write(root: Path, relative: str, value: bytes | str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(value.encode("utf-8") if isinstance(value, str) else value)


class CorpusScopeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = module_from_path("loop_policy_for_corpus_scope_test", PM / "loop_policy.py")
        cls.core = module_from_path("document_corpus_for_scope_fixture", CORE_PATH)

    def setUp(self):
        self.root = self.make_fixture()
        self.addCleanup(lambda: shutil.rmtree(self.root, ignore_errors=True))
        self.product_source = "doc/product/README.md"
        self.object_endpoint = self.core.record_path("object", self.product_source)

    def make_fixture(self):
        root = Path(tempfile.mkdtemp(prefix="oasis7-corpus-scope-"))
        git(root, "init", "--quiet")
        git(root, "config", "user.name", "Corpus Scope Fixture")
        git(root, "config", "user.email", "corpus-scope@example.invalid")
        write(root, "doc/product/README.md", "# Product fixture\n")
        write(root, "doc/engineering/README.md", "# Engineering fixture\n")
        write(root, self.core.REGISTRY_PATH, self.core.canonical_json({
            "version": 1,
            "directories": [
                {"name": "engineering", "type": "professional_domain", "owner": "repository_health_engineer", "entry": "doc/engineering/README.md"},
                {"name": "product", "type": "product_overlay", "owner": "producer_system_designer", "entry": "doc/product/README.md"},
            ],
        }))
        write(root, self.core.CORPUS_ROOT, self.core.canonical_json({
            "version": 3,
            "scope": "all doc files through direct objects, delegated evidence objects, and controls",
            "decision_boundary": "routing is conservative heuristic input only",
            "product_modules": list(self.core.PRODUCT_MODULES),
            "objects_root": self.core.OBJECTS_ROOT,
            "semantic_entries_root": self.core.SEMANTIC_ENTRIES_ROOT,
            "semantic_bundles_root": self.core.SEMANTIC_BUNDLES_ROOT,
            "delegates": [{"kind": "testing-evidence", "path": self.core.EVIDENCE_ROOT, "expected_version": 2}],
        }))
        write(root, self.core.EVIDENCE_ROOT, self.core.canonical_json({
            "version": 2,
            "scope": "doc/testing/evidence/** excluding inventory.json",
            "entries_root": self.core.EVIDENCE_ENTRIES_ROOT,
            "groups_root": self.core.EVIDENCE_GROUPS_ROOT,
            "legacy_baseline_id": "document-evidence-v1-at-v3-migration",
        }))
        view = self.core.WorktreeCorpusView(root)
        for source in ("doc/engineering/README.md", "doc/product/README.md"):
            write(root, self.core.record_path("object", source), self.core._wrapped("object", self.core.expected_object(view, source)))
        git(root, "add", "-A")
        git(root, "commit", "-m", "fixture corpus baseline")
        return root

    def install_effective_tools(self, *, mixed_bundle: bool = False, unknown_bundle_member: bool = False,
                                semantic_entry: bool = False):
        if mixed_bundle:
            members = ["doc/engineering/README.md", "doc/product/README.md"]
            view = self.core.WorktreeCorpusView(self.root)
            bundle_id = "mixed-loop-fixture"
            record = {
                "id": bundle_id,
                "paths": members,
                "content_set_sha256": self.core._semantic_digest(view, members),
                "disposition": "retain_current_authority",
                "decision_owner": "producer_system_designer",
                "current_authority": "doc/product/README.md",
                "note": "Fixture proving cross-loop groups remain atomic.",
            }
            write(self.root, f"doc/.governance/document-corpus/semantic/bundles/{bundle_id}.json", self.core.canonical_json({
                "schema": "oasis7.document-semantic-bundle/v1",
                "record": record,
            }))
            git(self.root, "add", "-A")
            git(self.root, "commit", "-m", "fixture mixed semantic bundle")
        elif unknown_bundle_member:
            members = ["doc/product/README.md", "doc/product/missing.md"]
            record = {
                "id": "unknown-member-fixture",
                "paths": members,
                "content_set_sha256": self.core.sha256(b"unvalidated legacy record"),
                "disposition": "retain_current_authority",
                "decision_owner": "producer_system_designer",
                "current_authority": "doc/product/README.md",
                "note": "Fixture preserving an unresolved legacy member.",
            }
            write(self.root, "doc/.governance/document-corpus/semantic/bundles/unknown-member-fixture.json",
                  self.core._wrapped("semantic-bundle", record))
            git(self.root, "add", "-A")
            git(self.root, "commit", "-m", "fixture bundle with unknown member")
        if semantic_entry:
            source = self.product_source
            view = self.core.WorktreeCorpusView(self.root)
            record = {
                "path": source,
                "content_sha256": self.core.sha256(view.read_bytes(source)),
                "disposition": "retain_current_authority",
                "decision_owner": "producer_system_designer",
                "current_authority": source,
                "note": "Fixture semantic record for precise sidecar scope.",
            }
            endpoint = self.core.record_path("semantic", source)
            self.semantic_endpoint = endpoint
            write(self.root, endpoint, self.core._wrapped("semantic-entry", record))
            git(self.root, "add", "-A")
            git(self.root, "commit", "-m", "fixture semantic entry")

        for relative, source in (
            ("scripts/pm/loop_policy.py", PM / "loop_policy.py"),
            ("scripts/pm/loop_contracts.py", PM / "loop_contracts.py"),
            ("scripts/pm/loop-policy.v1.json", POLICY_PATH),
            ("scripts/document_corpus.py", CORE_PATH),
        ):
            write(self.root, relative, source.read_bytes())
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture effective policy")
        self.base = git(self.root, "rev-parse", "HEAD")
        git(self.root, "update-ref", "refs/remotes/origin/main", self.base)
        self.tool_root = self.root.parent / (self.root.name + "-effective-tools")
        git(self.root, "worktree", "add", "--detach", str(self.tool_root), self.base)
        self.addCleanup(lambda: shutil.rmtree(self.tool_root, ignore_errors=True))
        policy_bytes = (self.tool_root / "scripts/pm/loop-policy.v1.json").read_bytes()
        self.binding = {
            "schema": "oasis7.loop-task/v1",
            "task_uid": "task_" + "b" * 32,
            "change_id": "corpus-scope-fixture",
            "loop": "product",
            "owner_role": "gameplay_designer",
            "bootstrap_epoch": 1,
            "manual_request_ref": "fixture-user-request",
            "request_key": "fixture-request-key",
            "write_scope": [self.product_source, self.object_endpoint],
            "out_of_scope": [],
            "acceptance_refs": ["fixture-acceptance"],
            "dependencies": [],
            "input_contracts": [],
            "target_delivery": "corpus-fixture",
            "policy_commit": self.base,
            "policy_digest": "sha256:" + hashlib.sha256(policy_bytes).hexdigest(),
        }
        if semantic_entry:
            self.binding["write_scope"].append(self.semantic_endpoint)

    def commit_scope_candidate(self, path: str, content: str):
        write(self.root, path, content)
        if path == self.product_source:
            view = self.core.WorktreeCorpusView(self.root)
            write(self.root, self.object_endpoint, self.core._wrapped("object", self.core.expected_object(view, path)))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture scoped corpus edit")
        return git(self.root, "rev-parse", "HEAD")

    def test_changed_product_source_requires_its_exact_object_endpoint(self):
        self.install_effective_tools()
        head = self.commit_scope_candidate(self.product_source, "# Product fixture, changed.\n")
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

        self.binding["write_scope"] = [self.product_source]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any(self.object_endpoint in item for item in verdict["blockers"]), verdict)

    def test_semantic_entry_inherits_source_loop_and_requires_source_scope(self):
        self.install_effective_tools(semantic_entry=True)
        shard = json.loads((self.root / self.semantic_endpoint).read_text())
        shard["record"]["note"] = "changed fixture review note"
        write(self.root, self.semantic_endpoint, self.core.canonical_json(shard))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture sidecar-only edit")
        head = git(self.root, "rev-parse", "HEAD")
        self.binding["write_scope"] = [self.product_source, self.object_endpoint, self.semantic_endpoint]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

        self.binding["write_scope"] = [self.semantic_endpoint]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any(self.product_source in item for item in verdict["blockers"]), verdict)

        self.binding["write_scope"] = [self.product_source, self.object_endpoint, self.semantic_endpoint]
        self.binding["out_of_scope"] = [self.semantic_endpoint]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("outside declared write scope" in item for item in verdict["blockers"]), verdict)

    def test_cross_loop_bundle_member_blocks_ordinary_leaf(self):
        self.install_effective_tools(mixed_bundle=True)
        self.binding["write_scope"] = ["**"]
        head = self.commit_scope_candidate(self.product_source, "# Product fixture, changed.\n")
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("crosses loops" in item for item in verdict["blockers"]), verdict)

    def test_unknown_bundle_member_blocks_ordinary_leaf(self):
        self.install_effective_tools(unknown_bundle_member=True)
        self.binding["write_scope"] = ["**"]
        head = self.commit_scope_candidate(self.product_source, "# Product fixture, changed.\n")
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("crosses loops" in item for item in verdict["blockers"]), verdict)

    def test_deleted_source_uses_base_endpoint_and_out_of_scope_applies(self):
        self.install_effective_tools()
        (self.root / self.product_source).unlink()
        (self.root / self.object_endpoint).unlink()
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture source deletion")
        head = git(self.root, "rev-parse", "HEAD")
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

        self.binding["write_scope"] = ["**"]
        self.binding["out_of_scope"] = [self.object_endpoint]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("outside declared write scope" in item for item in verdict["blockers"]), verdict)

    def test_new_source_requires_exact_new_endpoint_and_generated_object(self):
        self.install_effective_tools()
        source = "doc/product/new.md"
        endpoint = self.core.record_path("object", source)
        write(self.root, source, "# New product fixture\n")
        view = self.core.WorktreeCorpusView(self.root)
        write(self.root, endpoint, self.core._wrapped("object", self.core.expected_object(view, source)))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture new source and object")
        head = git(self.root, "rev-parse", "HEAD")
        self.binding["write_scope"] = [source]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any(endpoint in item for item in verdict["blockers"]), verdict)

        self.binding["write_scope"] = [source, endpoint]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

    def test_new_semantic_sidecar_requires_its_exact_declared_endpoint(self):
        self.install_effective_tools()
        view = self.core.WorktreeCorpusView(self.root)
        record = {
            "path": self.product_source,
            "content_sha256": self.core.sha256(view.read_bytes(self.product_source)),
            "disposition": "retain_current_authority",
            "decision_owner": "producer_system_designer",
            "current_authority": self.product_source,
            "note": "New review record requires explicit sidecar scope.",
        }
        endpoint = self.core.record_path("semantic", self.product_source)
        write(self.root, endpoint, self.core._wrapped("semantic-entry", record))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture new semantic sidecar")
        head = git(self.root, "rev-parse", "HEAD")
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any(endpoint in item for item in verdict["blockers"]), verdict)

        self.binding["write_scope"].append(endpoint)
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

    def test_source_change_with_stale_generated_object_fails(self):
        self.install_effective_tools()
        write(self.root, self.product_source, "# Changed without sync\n")
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture stale generated object")
        head = git(self.root, "rev-parse", "HEAD")
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("generated object mismatch" in item for item in verdict["blockers"]), verdict)

    def test_static_registry_cannot_be_inherited_by_product_loop(self):
        self.install_effective_tools()
        registry = json.loads((self.root / self.core.REGISTRY_PATH).read_text())
        registry["fixture_note"] = "registry remains code-owned"
        write(self.root, self.core.REGISTRY_PATH, self.core.canonical_json(registry))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture registry change")
        head = git(self.root, "rev-parse", "HEAD")
        self.binding["write_scope"] = ["**"]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("loop ownership mismatch" in item for item in verdict["blockers"]), verdict)

    def test_system_source_and_object_endpoint_pass_under_system_binding(self):
        self.install_effective_tools()
        source = "doc/engineering/README.md"
        endpoint = self.core.record_path("object", source)
        write(self.root, source, "# Engineering fixture, changed.\n")
        view = self.core.WorktreeCorpusView(self.root)
        write(self.root, endpoint, self.core._wrapped("object", self.core.expected_object(view, source)))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture system source and object")
        head = git(self.root, "rev-parse", "HEAD")
        self.binding.update({"loop": "system", "owner_role": "repository_health_engineer",
                             "write_scope": [source, endpoint]})
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

    def test_product_task_cannot_write_another_source_shard(self):
        self.install_effective_tools()
        source = "doc/engineering/README.md"
        endpoint = self.core.record_path("object", source)
        write(self.root, source, "# Engineering fixture, changed by product task.\n")
        view = self.core.WorktreeCorpusView(self.root)
        write(self.root, endpoint, self.core._wrapped("object", self.core.expected_object(view, source)))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture cross-loop source shard")
        head = git(self.root, "rev-parse", "HEAD")
        self.binding["write_scope"] = ["**"]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any("loop ownership mismatch" in item for item in verdict["blockers"]), verdict)

    def test_rename_requires_old_and_new_base_head_endpoints_in_scope(self):
        self.install_effective_tools()
        renamed_source = "doc/product/renamed.md"
        renamed_endpoint = self.core.record_path("object", renamed_source)
        (self.root / self.product_source).unlink()
        (self.root / self.object_endpoint).unlink()
        write(self.root, renamed_source, "# Renamed product fixture\n")
        view = self.core.WorktreeCorpusView(self.root)
        write(self.root, renamed_endpoint,
              self.core._wrapped("object", self.core.expected_object(view, renamed_source)))
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture source rename")
        head = git(self.root, "rev-parse", "HEAD")
        self.binding["write_scope"] = [renamed_source, renamed_endpoint]
        self.binding["out_of_scope"] = [self.object_endpoint]
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "blocked", verdict)
        self.assertTrue(any(self.object_endpoint in item for item in verdict["blockers"]), verdict)

        self.binding["write_scope"] = [self.product_source, self.object_endpoint,
                                       renamed_source, renamed_endpoint]
        self.binding["out_of_scope"] = []
        verdict = self.policy.validate_scope(self.tool_root, self.root, self.binding, self.base, head)
        self.assertEqual(verdict["status"], "passed", verdict)

    def test_candidate_module_and_preloaded_shadow_cannot_select_admission_parser(self):
        self.install_effective_tools()
        marker = self.root / "candidate-parser-was-executed"
        write(self.root, "scripts/document_corpus.py",
              f"from pathlib import Path\nPath({str(marker)!r}).write_text('candidate')\n")
        git(self.root, "add", "-A")
        git(self.root, "commit", "-m", "fixture candidate parser shadow")
        module_name = "_oasis7_effective_document_corpus_" + self.base
        preloaded = types.ModuleType(module_name)
        preloaded.CORPUS_ROOT = "candidate-preloaded-value"
        sys.modules[module_name] = preloaded
        candidate_scripts = str(self.root / "scripts")
        sys.path.insert(0, candidate_scripts)
        try:
            loaded = self.policy.load_trusted_corpus_module(self.tool_root, self.root, self.binding)
        finally:
            sys.path.remove(candidate_scripts)
        self.assertIsNot(loaded, preloaded)
        self.assertEqual(loaded.CORPUS_ROOT, self.core.CORPUS_ROOT)
        self.assertEqual(Path(loaded.__file__).resolve(),
                         (self.tool_root / "scripts/document_corpus.py").resolve())
        self.assertFalse(marker.exists())

    def test_planner_routes_document_contract_dependencies_and_unknown_paths(self):
        def plan(*paths: str) -> dict[str, str]:
            command = [str(ROOT / "scripts/plan-rust-required-scope.sh"), "--event-name", "pull_request"]
            for path in paths:
                command.extend(("--changed-path", path))
            completed = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True)
            return dict(line.split("=", 1) for line in completed.stdout.splitlines() if "=" in line)

        code = plan("scripts/document_corpus.py", "scripts/document-corpus-inventory-workflow.test.py",
                    "doc/.governance/document-corpus-inventory.json")
        self.assertEqual(code["scope"], "targeted")
        self.assertEqual(code["selected_capabilities"], "doc_checker_contracts;workflow_governance")
        self.assertEqual(code["run_rust_baseline"], "false")

        data = plan("doc/.governance/document-corpus/semantic/aa/fixture.json")
        self.assertEqual(data["scope"], "minimal")
        self.assertEqual(data["selected_capabilities"], "required_gate_baseline")
        self.assertEqual(data["run_rust_baseline"], "false")

        unknown = plan("scripts/unknown-document-corpus-validator.py")
        self.assertEqual(unknown["scope"], "full")
        self.assertEqual(unknown["run_rust_baseline"], "true")

    def test_runner_inventory_keeps_baseline_and_full_tier_suite_coverage(self):
        ci_tests = (ROOT / "scripts/ci-tests.sh").read_text()
        capability_inventory = (ROOT / "scripts/ci-required-capability-test-inventory.tsv").read_text()
        baseline_inventory = module_from_path(
            "ci_required_inventory_for_document_corpus_workflow_test",
            ROOT / "scripts/pm/ci_required_inventory.py",
        )
        self.assertIn("run python3 ./scripts/document-corpus-inventory-check.test.py",
                      ci_tests[ci_tests.index("run_doc_checker_contract_tests() {"):])
        self.assertIn("run python3 ./scripts/document-corpus-inventory-workflow.test.py",
                      ci_tests[ci_tests.index("run_workflow_governance_operational_contract_tests() {"):])
        self.assertIn("scripts/document-corpus-inventory-check.test.py", capability_inventory)
        self.assertIn("scripts/document-corpus-inventory-workflow.test.py", capability_inventory)
        self.assertIn("document-corpus-v3-check", baseline_inventory.BASELINE_OBLIGATIONS)
        self.assertIn("scripts/document_corpus.py", baseline_inventory.BASELINE_CHECKER_PATHS)
        full_suite = subprocess.run([str(ROOT / "scripts/ci-tests-full-superset-contract.test.sh")],
                                    cwd=ROOT, text=True, capture_output=True)
        self.assertEqual(full_suite.returncode, 0, full_suite.stdout + full_suite.stderr)


if __name__ == "__main__":
    unittest.main()
