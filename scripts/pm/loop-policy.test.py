#!/usr/bin/env python3
"""Behavior tests for immutable loop scope policy; temporary Git only."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import unittest
from unittest import mock
from unittest.mock import patch

HERE = Path(__file__).resolve().parent

class PolicyTests(unittest.TestCase):
    def test_parallel_disjoint_branch_scope_and_integration_are_separate(self):
        self.binding.update(loop='code', write_scope=['src/a.rs'])
        self.git('switch', '-c', 'task')
        self.write('src/a.rs', 'task edit')
        self.git('add', '.'); self.git('commit', '-qm', 'task')
        head = self.git('rev-parse', 'HEAD')
        self.git('switch', '--detach', self.base)
        self.write('doc/engineering/a.md', 'unrelated main edit')
        self.git('add', '.'); self.git('commit', '-qm', 'main advanced')
        integration = self.git('rev-parse', 'HEAD')
        context = self.api.scope_context(self.root, integration, head)
        self.assertEqual(context['scope_base_oid'], self.base)
        self.assertEqual(context['integration_base_oid'], integration)
        result = self.api.validate_scope(self.tool, self.root, self.binding, context['scope_base_oid'], head)
        self.assertEqual(result['status'], 'passed', result)
        self.assertEqual([item['path'] for item in result['paths']], ['src/a.rs'])

    def test_real_gameplay_document_ownership(self):
        policy=json.loads((HERE / "loop-policy.v1.json").read_text())
        self.assertEqual(self.api.classify_path("doc/game/gameplay/gameplay-indirect-control-agency-contract.prd.md",policy),"product")
        self.assertEqual(self.api.classify_path("doc/game/gameplay/gameplay-indirect-control-agency-contract.design.md",policy),"product")
        self.assertEqual(self.api.classify_path("doc/world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md",policy),"system")
        self.assertEqual(self.api.classify_path("doc/world-runtime/design.md",policy),"system")
        self.assertIsNone(self.api.classify_path("doc/game/design.md",policy))

    def test_mixed_document_rename_endpoints_block_every_loop(self):
        mixed='doc/game/gameplay/gameplay-agent-claim-economy-contract.design.md'
        self.write(mixed,'mixed semantic authority')
        self.git('add','.');self.git('commit','-qm','mixed baseline')
        base=self.git('rev-parse','HEAD')
        self.git('mv',mixed,'doc/product/moved.md')
        self.git('commit','-qm','rename')
        head=self.git('rev-parse','HEAD')
        for loop in ('product','system','code'):
            self.binding.update(loop=loop,write_scope=['**'])
            result=self.api.validate_scope(self.tool,self.root,self.binding,base,head)
            self.assertEqual(result['status'],'blocked')
            self.assertTrue(any('mixed document' in b for b in result['blockers']),result)
        self.git('mv','doc/product/moved.md',mixed)
        self.git('commit','-qm','rename destination')
        result=self.api.validate_scope(self.tool,self.root,self.binding,head,self.git('rev-parse','HEAD'))
        self.assertTrue(any('mixed document' in b for b in result['blockers']),result)

    def setUp(self):
        spec = importlib.util.spec_from_file_location("loop_policy", HERE / "loop_policy.py")
        self.api = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.api)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        self.git("config", "remote.origin.url", "https://github.com/eng-cc/oasis7.git")
        self.write("scripts/pm/loop-policy.v1.json", (HERE / "loop-policy.v1.json").read_text())
        self.write("doc/engineering/workflow/source-of-truth.md",
                   (HERE.parents[1] / "doc/engineering/workflow/source-of-truth.md").read_text())
        self.write("scripts/document_corpus.py", '''
import hashlib
import subprocess

CORE_MARKER = "pinned-core-bytes"
CORPUS_ROOT = "doc/.governance/document-corpus-inventory.json"
EVIDENCE_ROOT = "doc/testing/evidence/inventory.json"
REGISTRY_PATH = "doc/.governance/top-level-directory-registry.json"
LEGACY_SEMANTIC_ROOT = "doc/.governance/document-semantic-review-overrides.json"
EVIDENCE_SUFFIXES = {".md", ".txt", ".json", ".jsonl", ".csv", ".tsv"}

class GitCorpusView:
    def __init__(self, root, revision):
        self.root, self.snapshot_id = root, revision
        self.paths = subprocess.check_output(["git", "-C", str(root), "ls-tree", "-r", "--name-only", revision, "--", "doc"], text=True).splitlines()
    def list_doc_paths(self):
        return self.paths
    def file_mode(self, path):
        raw = subprocess.check_output(["git", "-C", str(self.root), "ls-tree", "-z", self.snapshot_id, "--", path])
        return raw.split(b" ", 1)[0].decode() if raw else "000000"

class Model:
    metadata_paths = set()
    objects_by_path = {}
    semantic_entries_by_path = {}
    semantic_bundles_by_id = {}
    evidence_entries_by_path = {}
    evidence_groups_by_id = {}
    path_to_bundle = {}
    path_to_group = {}

def load_corpus(view):
    return Model()

def locate(model, path):
    return []

def expected_object(view, source):
    return None

def record_path(kind, source):
    key = hashlib.sha256(source.encode("utf-8")).hexdigest()
    root = "evidence/entries" if kind in {"evidence", "evidence-entry"} else "objects"
    return f"doc/.governance/document-corpus/{root}/{key[:2]}/{key}.json"
''')
        self.write("scripts/product-doc-content-check.py", "# pinned transitive checker fixture\n")
        self.write("scripts/product_doc_markdown.py", "# pinned transitive markdown fixture\n")
        self.write("doc/product/a.md", "product")
        self.write("doc/engineering/a.md", "system")
        self.write("src/a.rs", "code")
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        self.base = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        self.tool = self.root.parent / (self.root.name + "-effective")
        self.git("worktree", "add", "--detach", str(self.tool), self.base)
        self.addCleanup(lambda: self.git("worktree", "remove", "--force", str(self.tool)))
        self.binding = dict(schema="oasis7.loop-task/v1", task_uid="task_" + "a"*32,
            change_id="change-test", loop="product", owner_role="gameplay_designer",
            bootstrap_epoch=1, manual_request_ref="user-message-1", request_key="request-1",
            write_scope=["doc/product/**", "doc/.governance/document-corpus/objects/**"], out_of_scope=[], input_contracts=[],
            acceptance_refs=["acceptance-1"], dependencies=[], target_delivery="pilot",
            policy_digest="sha256:"+hashlib.sha256((self.root / "scripts/pm/loop-policy.v1.json").read_bytes()).hexdigest(),
            policy_commit=self.base)
        self.live_identity_patch = patch.object(
            self.api, "current_effective_policy_identity",
            return_value=self.live_policy_proof(self.base),
        )
        self.live_identity_mock = self.live_identity_patch.start()
        self.addCleanup(self.live_identity_patch.stop)

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def write(self, path, value):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(value)

    def check(self):
        self.git("add", ".")
        self.git("commit", "-qm", "candidate")
        return self.api.validate_scope(self.tool, self.root, self.binding, self.base, self.git("rev-parse", "HEAD"))

    def test_product_allowed(self):
        self.write("doc/product/a.md", "changed")
        self.assertEqual(self.check()["status"], "passed")

    def test_cross_loop_rename_checks_both_endpoints(self):
        self.git("mv", "doc/engineering/a.md", "doc/product/stolen.md")
        self.assertEqual(self.check()["status"], "blocked")

    def test_cross_loop_deletion_rejected(self):
        (self.root / "src/a.rs").unlink()
        self.assertEqual(self.check()["status"], "blocked")

    def test_unknown_path_rejected(self):
        self.binding.update(loop="code", write_scope=["**"])
        self.write("unexpected/unknown.bin", "unknown")
        self.assertEqual(self.check()["status"], "blocked")

    def test_symlink_and_mode_change_rejected(self):
        (self.root / "doc/product/link.md").symlink_to("../../src/a.rs")
        self.assertEqual(self.check()["status"], "blocked")

    def test_document_executable_mode_rejected(self):
        (self.root / "doc/product/a.md").chmod(0o755)
        self.assertEqual(self.check()["status"], "blocked")

    def test_candidate_policy_cannot_reclassify_itself(self):
        self.binding.update(loop="code", write_scope=["**"])
        policy = json.loads((self.root / "scripts/pm/loop-policy.v1.json").read_text())
        policy["rules"] = [{"pattern": "**", "loop": "code"}]
        self.write("scripts/pm/loop-policy.v1.json", json.dumps(policy))
        self.write("doc/product/a.md", "candidate claims code")
        self.assertEqual(self.check()["status"], "blocked")

    def test_binding_missing_identity_and_cycle_rejected(self):
        self.binding["dependencies"] = [self.binding["task_uid"]]
        self.assertEqual(self.api.validate_binding(self.binding)["status"], "blocked")
        del self.binding["manual_request_ref"]
        self.assertEqual(self.api.validate_binding(self.binding)["status"], "blocked")

    def test_optional_coordination_and_path_clause_refs_are_additive(self):
        self.assertEqual(self.api.validate_binding(self.binding)["status"], "passed")
        self.binding["coordination_ref"] = {
            "repository": "eng-cc/oasis7", "issue_number": 3671,
            "comment_id": 5636938574, "record_digest": "sha256:" + "1" * 64,
        }
        self.binding["consumed_clause_refs"] = [{
            "repository": "eng-cc/oasis7", "path": "doc/engineering/spec.md",
            "fragment": "section-1", "clause_id": "section-1",
        }]
        self.assertEqual(self.api.validate_binding(self.binding)["status"], "passed")
        self.binding["consumed_clause_refs"][0]["path"] = "../outside.md"
        self.assertEqual(self.api.validate_binding(self.binding)["status"], "blocked")

    def test_policy_digest_drift_rejected(self):
        self.binding["policy_digest"] = "sha256:" + "0"*64
        self.write("doc/product/a.md", "changed")
        self.assertEqual(self.check()["status"], "blocked")

    def test_out_of_scope_overrides_allow(self):
        self.binding["out_of_scope"] = ["doc/product/a.md"]
        self.write("doc/product/a.md", "changed")
        self.assertEqual(self.check()["status"], "blocked")

    def test_scope_globs_respect_components_and_recursive_exclusions(self):
        self.binding.update(loop="code", write_scope=["scripts/pm/*.py"])
        self.write("scripts/pm/nested/deeper/a.py", "candidate")
        result = self.check()
        self.assertTrue(any("outside declared write scope" in b for b in result["blockers"]), result)
        head = self.git("rev-parse", "HEAD")
        for allow, deny, expected in [
            (["scripts/pm/**/*.py"], [], "passed"),
            (["scripts/pm/**"], ["scripts/pm/*.py"], "passed"),
            (["scripts/pm/**"], ["scripts/pm/**/*.py"], "blocked"),
        ]:
            with self.subTest(allow=allow, deny=deny):
                self.binding.update(write_scope=allow, out_of_scope=deny)
                actual = self.api.validate_scope(self.tool, self.root, self.binding, self.base, head)
                self.assertEqual(actual["status"], expected, actual)

    def test_policy_glob_components_and_recursive_zero_depth(self):
        cases = [
            ("scripts/pm/*.py", "scripts/pm/a.py", True),
            ("scripts/pm/*.py", "scripts/pm/nested/a.py", False),
            ("scripts/pm/**.py", "scripts/pm/nested/a.py", False),
            ("scripts/pm/**/*.py", "scripts/pm/a.py", True),
            ("scripts/pm/**/*.py", "scripts/pm/a/b/c.py", True),
            ("scripts/pm/**", "scripts/pm/a/b/c.py", True),
            ("**/AGENTS.md", "AGENTS.md", True),
            ("**/AGENTS.md", "doc/deep/AGENTS.md", True),
            ("scripts/pm/?.py", "scripts/pm/a.py", True),
            ("scripts/pm/[ab].py", "scripts/pm/a.py", True),
            ("scripts/pm/[!a].py", "scripts/pm/b.py", True),
            ("scripts/pm/[!a].py", "scripts/pm/a.py", False),
            ("scripts/pm/[!a]*.py", "scripts/pm/b/c.py", False),
            ("scripts/pm", "scripts/pm/a.py", False),
        ]
        for pattern, path, expected in cases:
            with self.subTest(pattern=pattern, path=path):
                policy = {"denied": [], "rules": [{"pattern": pattern, "loop": "code"}]}
                self.assertEqual(self.api.classify_path(path, policy), "code" if expected else None)
                policy.update(denied=[pattern], rules=[{"pattern": "**", "loop": "code"}])
                self.assertEqual(self.api.classify_path(path, policy), None if expected else "code")

    def test_scope_glob_checks_both_same_loop_rename_endpoints(self):
        direct, nested = "scripts/pm/a.py", "scripts/pm/nested/a.py"
        self.write(direct, "original")
        self.git("add", ".")
        self.git("commit", "-qm", "rename baseline")
        self.binding.update(loop="code", write_scope=["scripts/pm/*.py"])
        (self.root / "scripts/pm/nested").mkdir()
        for source, target in [(direct, nested), (nested, direct)]:
            base = self.git("rev-parse", "HEAD")
            self.git("mv", source, target)
            self.git("commit", "-qm", "rename")
            actual = self.api.validate_scope(self.tool, self.root, self.binding, base,
                                             self.git("rev-parse", "HEAD"))
            self.assertIn("outside declared write scope: " + nested, actual["blockers"], actual)

    def test_real_policy_globs_keep_precedence_and_boundaries(self):
        policy = json.loads((HERE / "loop-policy.v1.json").read_text())
        for path, expected in [
            ("doc/game/gameplay/a.prd.md", "product"),
            ("doc/game/gameplay/nested/a.prd.md", "system"),
            ("doc/product/deep/AGENTS.md", "code"),
            ("doc/product/deep/a.md", "product"),
            ("third_party/deep/a.rs", None),
            (".pm/deep/a.py", None),
        ]:
            with self.subTest(path=path):
                self.assertEqual(self.api.classify_path(path, policy), expected)

    def test_executable_source_under_doc_is_not_document(self):
        self.write("doc/product/executable.py", "print('side effect')")
        self.assertEqual(self.check()["status"], "blocked")

    def test_trusted_tool_checkout_and_main_anchor(self):
        self.git("update-ref","refs/remotes/origin/main",self.base)
        proof = self.live_policy_proof(self.base)
        with patch.object(self.api, "current_effective_policy_identity", return_value=proof) as live_read:
            self.assertEqual(self.api.validate_tool_root(self.root,self.root,self.binding)["status"],"passed")
            live_read.assert_called_once_with(self.root.resolve(), "eng-cc/oasis7")
        self.write("scripts/pm/shadow.py","untracked executable authority")
        with patch.object(self.api, "current_effective_policy_identity", return_value=proof):
            self.assertEqual(self.api.validate_tool_root(self.root,self.root,self.binding)["status"],"blocked")

    def live_policy_proof(self, oid):
        return {
            "default_branch": "main", "default_branch_oid": oid,
            "policy_commit": oid, "policy_digest": self.binding["policy_digest"],
            "workflow_source_digest": "sha256:" + "a" * 64,
        }

    def test_tool_root_uses_live_default_oid_without_refreshing_stale_ref(self):
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        self.write("doc/engineering/live-policy-anchor.txt", "adopted policy commit\n")
        self.git("add", "doc/engineering/live-policy-anchor.txt")
        self.git("commit", "-qm", "adopted policy commit")
        pin = self.git("rev-parse", "HEAD")
        tool_root = self.root.parent / ("pinned-tool-" + self.root.name)
        self.git("worktree", "add", "-q", "--detach", str(tool_root), pin)
        self.write("doc/engineering/live-policy-tip.txt", "later default tip\n")
        self.git("add", "doc/engineering/live-policy-tip.txt")
        self.git("commit", "-qm", "later default tip")
        live_tip = self.git("rev-parse", "HEAD")
        self.binding["policy_commit"] = pin
        stale_ref = self.git("rev-parse", "refs/remotes/origin/main")
        proof = self.live_policy_proof(live_tip)

        with patch.object(self.api, "current_effective_policy_identity", return_value=proof) as live_read:
            result = self.api.validate_tool_root(tool_root, self.root, self.binding)
        self.assertEqual(result["status"], "passed", result)
        live_read.assert_called_once_with(self.root.resolve(), "eng-cc/oasis7")
        self.assertEqual(self.git("rev-parse", "refs/remotes/origin/main"), stale_ref)

        missing_tip = "f" * 40
        with patch.object(self.api, "current_effective_policy_identity",
                          return_value=self.live_policy_proof(missing_tip)):
            unavailable = self.api.validate_tool_root(tool_root, self.root, self.binding)
        self.assertEqual(unavailable["status"], "pending", unavailable)
        self.assertIn("tip object is unavailable locally", unavailable["blockers"][0])
        self.assertEqual(self.git("rev-parse", "refs/remotes/origin/main"), stale_ref)

    def test_trusted_core_load_uses_live_default_oid_without_tracking_ref(self):
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        self.write("doc/engineering/live-policy-tip.txt", "later protected default tip\n")
        self.git("add", "doc/engineering/live-policy-tip.txt")
        self.git("commit", "-qm", "later protected default tip")
        live_tip = self.git("rev-parse", "HEAD")
        self.git("update-ref", "-d", "refs/remotes/origin/main")

        proof = self.live_policy_proof(live_tip)
        with patch.object(self.api, "current_effective_policy_identity", return_value=proof) as live_read:
            loaded = self.api.load_trusted_corpus_module(self.tool, self.root, self.binding)

        self.assertEqual(loaded.CORE_MARKER, "pinned-core-bytes")
        self.assertEqual(Path(loaded.__file__).resolve(), (self.tool / "scripts/document_corpus.py").resolve())
        live_read.assert_called_once_with(self.root.resolve(), "eng-cc/oasis7")
        self.assertEqual(self.git("for-each-ref", "--format=%(refname)", "refs/remotes/origin/main"), "")

    def test_trusted_core_load_ignores_candidate_module_and_sys_modules_shadow(self):
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        target = self.root.parent / (self.root.name + "-candidate")
        self.git("worktree", "add", "--detach", str(target), self.base)
        self.addCleanup(lambda: self.git("worktree", "remove", "--force", str(target)))
        candidate = target / "scripts/document_corpus.py"
        candidate.parent.mkdir(parents=True, exist_ok=True)
        candidate.write_text("CORE_MARKER = 'candidate-shadow'\n")
        sys.path.insert(0, str(candidate.parent))
        self.addCleanup(lambda: sys.path.remove(str(candidate.parent)) if str(candidate.parent) in sys.path else None)
        module_name = "_oasis7_effective_document_corpus_" + self.base
        sys.modules[module_name] = types.ModuleType(module_name)
        self.addCleanup(lambda: sys.modules.pop(module_name, None))

        loaded = self.api.load_trusted_corpus_module(self.root, target, self.binding)

        self.assertEqual(loaded.CORE_MARKER, "pinned-core-bytes")
        self.assertEqual(Path(loaded.__file__).resolve(), (self.root / "scripts/document_corpus.py").resolve())
        self.assertFalse((self.root / "scripts/__pycache__").exists())

    def test_trusted_core_change_and_symlink_fail_closed(self):
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        for relative in ("scripts/document_corpus.py", "scripts/product-doc-content-check.py", "scripts/product_doc_markdown.py"):
            with self.subTest(relative=relative):
                trusted = self.root / relative
                original = trusted.read_bytes()
                trusted.write_bytes(original + b"# drift\n")
                self.assertEqual(self.api.validate_tool_root(self.root, self.root, self.binding)["status"], "blocked")
                trusted.write_bytes(original)
                trusted.unlink()
                trusted.symlink_to("../doc/product/a.md")
                verdict = self.api.validate_tool_root(self.root, self.root, self.binding)
                self.assertEqual(verdict["status"], "blocked", verdict)
                trusted.unlink()
                trusted.write_bytes(original)

    def _pin_regular_import_mode(self, mode):
        for relative in self.api.TRUSTED_IMPORT_FILES:
            path = self.root / relative
            path.chmod(mode)
            self.git('update-index', '--chmod=+x' if mode == 0o755 else '--chmod=-x', relative)
        self.git('commit', '--allow-empty', '-qm', 'pin regular imported modes')
        self.base = self.git('rev-parse', 'HEAD')
        self.binding['policy_commit'] = self.base
        self.git('update-ref', 'refs/remotes/origin/main', self.base)
        self.live_identity_mock.return_value = self.live_policy_proof(self.base)

    def test_trusted_regular_executable_imports_are_admitted(self):
        self._pin_regular_import_mode(0o755)
        for relative in self.api.TRUSTED_IMPORT_FILES:
            self.assertTrue(self.git('ls-tree', self.base, '--', relative).startswith('100755 blob '))
        verdict = self.api.validate_tool_root(self.root, self.root, self.binding)
        self.assertEqual(verdict['status'], 'passed', verdict)

    def test_trusted_regular_executable_core_loads_pinned_bytes(self):
        self._pin_regular_import_mode(0o755)
        loaded = self.api.load_trusted_corpus_module(self.root, self.root, self.binding)
        self.assertEqual(loaded.CORE_MARKER, 'pinned-core-bytes')
        self.assertFalse((self.root / 'scripts/__pycache__').exists())

    def test_regular_nonexecutable_import_and_equal_byte_symlink_control(self):
        self._pin_regular_import_mode(0o644)
        self.assertEqual(self.api.validate_tool_root(self.root, self.root, self.binding)['status'], 'passed')
        loaded = self.api.load_trusted_corpus_module(self.root, self.root, self.binding)
        self.assertEqual(loaded.CORE_MARKER, 'pinned-core-bytes')
        trusted = self.root / 'scripts/document_corpus.py'
        original = trusted.read_bytes()
        copy = self.root / 'contained-copy.py'
        copy.write_bytes(original)
        trusted.unlink()
        trusted.symlink_to('../contained-copy.py')
        self.assertEqual(trusted.read_bytes(), original)
        self.assertTrue(trusted.resolve().is_relative_to(self.root.resolve()))
        errors = self.api._trusted_file_errors(self.root, self.base, ('scripts/document_corpus.py',))
        self.assertIn('trusted module path escapes tool_root: scripts/document_corpus.py', errors)
        with self.assertRaisesRegex(ValueError, 'trusted module path escapes tool_root'):
            self.api.load_trusted_corpus_module(self.root, self.root, self.binding)

    def test_actual_symlink_and_gitlink_import_modes_are_rejected(self):
        relative = 'scripts/product-doc-content-check.py'
        trusted = self.root / relative
        original = trusted.read_bytes()
        trusted.unlink()
        trusted.symlink_to('product_doc_markdown.py')
        self.git('add', relative)
        self.git('commit', '-qm', 'unsafe symlink import')
        symlink_commit = self.git('rev-parse', 'HEAD')
        self.assertTrue(self.git('ls-tree', symlink_commit, '--', relative).startswith('120000 blob '))
        self.assertIn('trusted module has unsafe Git mode: ' + relative,
                      self.api._trusted_file_errors(self.root, symlink_commit, (relative,)))
        trusted.unlink()
        trusted.write_bytes(original)
        self.git('update-index', '--add', '--cacheinfo', '160000,' + self.base + ',' + relative)
        self.git('commit', '-qm', 'unsafe gitlink import')
        gitlink_commit = self.git('rev-parse', 'HEAD')
        self.assertTrue(self.git('ls-tree', gitlink_commit, '--', relative).startswith('160000 commit '))
        self.assertIn('trusted module has unsafe Git mode: ' + relative,
                      self.api._trusted_file_errors(self.root, gitlink_commit, (relative,)))

    def test_executable_import_byte_drift_and_missing_file_fail_closed(self):
        self._pin_regular_import_mode(0o755)
        trusted = self.root / 'scripts/document_corpus.py'
        original = trusted.read_bytes()
        trusted.write_bytes(original + b'\nCORE_MARKER = "candidate-shadow"\n')
        verdict = self.api.validate_tool_root(self.root, self.root, self.binding)
        self.assertEqual(verdict['status'], 'blocked', verdict)
        with self.assertRaises(ValueError):
            self.api.load_trusted_corpus_module(self.root, self.root, self.binding)
        trusted.unlink()
        self.assertTrue(self.api._trusted_file_errors(self.root, self.base, ('scripts/document_corpus.py',)))

    def test_malformed_import_tree_records_fail_closed(self):
        relative = 'scripts/document_corpus.py'
        oid = b'a' * 40
        records = [
            b'100600 blob ' + oid + b'\t' + relative.encode() + b'\0',
            b'100644 tree ' + oid + b'\t' + relative.encode() + b'\0',
            b'100644 blob ' + oid + b'\twrong/path.py\0',
            (b'100644 blob ' + oid + b'\t' + relative.encode() + b'\0') * 2,
        ]
        for record in records:
            with self.subTest(record=record), mock.patch.object(self.api, 'git', return_value=record):
                errors = self.api._trusted_file_errors(self.root, self.base, (relative,))
                self.assertTrue(any('unsafe Git mode' in e or 'missing or ambiguous' in e for e in errors), errors)

    def test_candidate_commit_cannot_be_effective_tool(self):
        self.git("update-ref","refs/remotes/origin/main",self.base)
        self.write("doc/product/a.md","candidate")
        self.git("add",".")
        self.git("commit","-qm","candidate")
        self.binding["policy_commit"]=self.git("rev-parse","HEAD")
        with patch.object(self.api, "current_effective_policy_identity",
                          return_value=self.live_policy_proof(self.base)):
            self.assertEqual(self.api.validate_tool_root(self.root,self.root,self.binding)["status"],"blocked")

    def test_closed_binding_rejects_unknown_effective_policy_field(self):
        self.binding["effective_policy_commit"] = self.base
        self.assertEqual("blocked", self.api.validate_binding(self.binding)["status"])

    def test_adopted_policy_chain_keeps_original_binding_and_survives_main_advance(self):
        identity = self.adoption_identity()
        auth_comment = self.authorization_comment(identity, self.base)
        adoption = self.adoption_record(identity, auth_comment, self.base, None)
        comments = {"complete": True, "repository": "eng-cc/oasis7", "issue_number": 123,
                    "comments": [auth_comment, adoption]}
        current = self.current_policy_identity(self.base)
        original = dict(self.binding)
        result = self.api.resolve_effective_policy(
            self.root, self.binding, comments, identity, current,
        )
        self.assertEqual("passed", result["status"], result)
        self.assertEqual(self.base, result["policy_commit"])
        self.assertEqual(self.binding, original, "effective pin must not mutate immutable binding")

        self.write("doc/engineering/a.md", "main advanced")
        self.git("add", ".")
        self.git("commit", "-qm", "main policy-independent advancement")
        advanced = self.git("rev-parse", "HEAD")
        result = self.api.resolve_effective_policy(
            self.root, self.binding, comments, identity, self.current_policy_identity(advanced),
        )
        self.assertEqual("passed", result["status"], result)
        self.assertEqual(self.base, result["policy_commit"])
        moved = {key: value for key, value in identity.items()
                 if key in self.api.HOSTED_ADOPTION_IDENTITY_FIELDS}
        moved.update(status="pr_watch", workflow_phase="pr_watch", hold_active=True)
        result = self.api.resolve_effective_policy(
            self.root, self.binding, comments, moved, self.current_policy_identity(advanced),
        )
        self.assertEqual(
            "passed", result["status"],
            "later task phase/hold must be evaluated by lifecycle gates, not erase pin",
        )
        self.assertEqual("blocked", self.api.validate_new_adoption_target(
            self.base, self.binding["policy_digest"], self.current_policy_identity(advanced),
        )["status"])
        self.assertEqual("passed", self.api.validate_new_adoption_target(
            advanced, self.current_policy_identity(advanced)["policy_digest"],
            self.current_policy_identity(advanced),
        )["status"])

    def test_hosted_policy_resolver_checks_chain_without_project_or_task_worktree_claim(self):
        identity = self.adoption_identity()
        auth_comment = self.authorization_comment(identity, self.base)
        adoption = self.adoption_record(identity, auth_comment, self.base, None)
        read = {"complete": True, "repository": identity["repository"],
                "issue_number": identity["issue_number"], "comments": [auth_comment, adoption]}
        hosted = {key: identity[key] for key in self.api.HOSTED_ADOPTION_IDENTITY_FIELDS}
        result = self.api.resolve_effective_policy(
            self.root, self.binding, read, hosted, self.current_policy_identity(self.base),
        )
        self.assertEqual("passed", result["status"], result)
        self.assertEqual(self.base, result["policy_commit"])
        self.assertEqual("task_issue_adoption_chain", result["pin_source"])

    def test_adoption_chain_rejects_stale_target_fork_and_identity_drift(self):
        identity = self.adoption_identity()
        auth_comment = self.authorization_comment(identity, self.base)
        first = self.adoption_record(identity, auth_comment, self.base, None)
        advanced, _ = self.next_policy_commit()
        stale_target = self.adoption_record(identity, auth_comment, self.base,
                                            predecessor="sha256:" + "a" * 64, comment_id=125)
        current = self.current_policy_identity(advanced)
        for comments, live_identity in [
            ([auth_comment, first, stale_target], identity),
            ([auth_comment, first], {**identity, "task_branch": "other-branch"}),
            ([auth_comment, {**first, "body": first["body"] + "\nforged"}], identity),
        ]:
            read = {"complete": True, "repository": "eng-cc/oasis7", "issue_number": 123,
                    "comments": comments}
            result = self.api.resolve_effective_policy(
                self.root, self.binding, read, live_identity, current,
            )
            self.assertIn(result["status"], {"blocked", "pending"}, result)

    def test_adoption_chain_rejects_missing_invalid_or_edited_comment_updates(self):
        identity = self.adoption_identity()
        auth = self.authorization_comment(identity, self.base)
        adoption = self.adoption_record(identity, auth, self.base, None)
        current = self.current_policy_identity(self.base)
        timestamp_cases = {
            "edited": lambda comment: dict(comment, updated_at="2026-10-01T00:02:00Z"),
            "missing": lambda comment: {key: value for key, value in comment.items() if key != "updated_at"},
            "invalid": lambda comment: dict(comment, updated_at="not-a-server-timestamp"),
        }
        for kind in ("authorization", "adoption"):
            for label, mutate in timestamp_cases.items():
                with self.subTest(comment_kind=kind, timestamp_case=label):
                    if kind == "authorization":
                        candidate_auth = mutate(auth)
                        candidate_adoption = self.adoption_record(
                            identity, candidate_auth, self.base, None,
                        )
                        comments = [candidate_auth, candidate_adoption]
                    else:
                        comments = [auth, mutate(adoption)]
                    read = {"complete": True, "repository": "eng-cc/oasis7",
                            "issue_number": 123, "comments": comments}
                    result = self.api.resolve_effective_policy(
                        self.root, self.binding, read, identity, current,
                    )
                    self.assertIn(result["status"], {"blocked", "pending"}, result)

    def adoption_identity(self):
        return {
            "repository": "eng-cc/oasis7", "issue_number": 123,
            "task_uid": self.binding["task_uid"], "bootstrap_epoch": 1,
            "binding_identity_digest": self.api.binding_identity_digest(self.binding),
            "write_scope_digest": self.api.write_scope_digest(self.binding),
            "owner_role": self.binding["owner_role"],
            "canonical_worktree": str(self.root), "task_branch": "task/test",
            "default_branch": "main", "project_id": "PVT_test",
            "project_number": 1, "project_item_id": "PVTI_test",
            "pr_number": 55, "pr_url": "https://github.com/eng-cc/oasis7/pull/55",
            "status": "ready", "workflow_phase": "pre_pr_ready", "hold_active": False,
        }

    def authorization_comment(self, identity, target):
        body = self.api.policy_adoption_authorization_comment({
            "task_uid": identity["task_uid"], "bootstrap_epoch": identity["bootstrap_epoch"],
            "binding_identity_digest": identity["binding_identity_digest"],
            "write_scope_digest": identity["write_scope_digest"],
            "target_policy_commit": target,
            "target_policy_digest": self.binding["policy_digest"],
        })
        return {"id": 124, "body": body, "created_at": "2026-10-01T00:00:00Z",
                "updated_at": "2026-10-01T00:00:00Z",
                "user": {"login": "human-author", "type": "User"},
                "author_association": "MEMBER"}

    def adoption_record(self, identity, auth_comment, target, predecessor,
                        comment_id=126, target_digest=None):
        value = self.api.build_policy_adoption_record(
            self.binding, identity, target_commit=target,
            target_digest=target_digest or self.binding["policy_digest"],
            target_proof=self.current_policy_identity(target),
            authorization_comment=auth_comment,
            predecessor_digest=predecessor,
        )
        return {"id": comment_id, "body": self.api.policy_adoption_comment(value),
                "created_at": "2026-10-01T00:00:01Z",
                "updated_at": "2026-10-01T00:00:01Z",
                "user": {"login": "repo-writer", "type": "User"},
                "author_association": "MEMBER"}

    def current_policy_identity(self, oid):
        policy = subprocess.check_output(["git", "-C", str(self.root), "show",
                                          f"{oid}:scripts/pm/loop-policy.v1.json"])
        source = subprocess.check_output(["git", "-C", str(self.root), "show",
                                          f"{oid}:doc/engineering/workflow/source-of-truth.md"])
        return {"default_branch": "main", "default_branch_oid": oid,
                "policy_commit": oid,
                "policy_digest": "sha256:" + hashlib.sha256(policy).hexdigest(),
                "workflow_source_digest": "sha256:" + hashlib.sha256(source).hexdigest()}

    def next_policy_commit(self):
        self.write("doc/engineering/a.md", "new committed policy-independent change")
        self.git("add", ".")
        self.git("commit", "-qm", "next default-branch revision")
        oid = self.git("rev-parse", "HEAD")
        return oid, self.current_policy_identity(oid)

    def test_dependency_closure_cycle_rejected(self):
        import copy
        other=copy.deepcopy(self.binding)
        other["task_uid"]="task_"+"b"*32
        self.binding["dependencies"]=[other["task_uid"]]
        other["dependencies"]=[self.binding["task_uid"]]
        self.assertEqual(self.api.validate_dependencies(self.binding,{other["task_uid"]:other})["status"],"blocked")

if __name__ == "__main__":
    unittest.main()
