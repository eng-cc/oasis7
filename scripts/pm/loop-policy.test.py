#!/usr/bin/env python3
"""Behavior tests for immutable loop scope policy; temporary Git only."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

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
        result = self.api.validate_scope(self.root, self.root, self.binding, context['scope_base_oid'], head)
        self.assertEqual(result['status'], 'passed', result)
        self.assertEqual([item['path'] for item in result['paths']], ['src/a.rs'])

    def test_real_gameplay_document_ownership(self):
        policy=json.loads((HERE / "loop-policy.v1.json").read_text())
        self.assertEqual(self.api.classify_path("doc/game/gameplay/gameplay-indirect-control-agency-contract.prd.md",policy),"product")
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
            result=self.api.validate_scope(self.root,self.root,self.binding,base,head)
            self.assertEqual(result['status'],'blocked')
            self.assertTrue(any('mixed document' in b for b in result['blockers']),result)
        self.git('mv','doc/product/moved.md',mixed)
        self.git('commit','-qm','rename destination')
        result=self.api.validate_scope(self.root,self.root,self.binding,head,self.git('rev-parse','HEAD'))
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
        self.write("scripts/pm/loop-policy.v1.json", (HERE / "loop-policy.v1.json").read_text())
        self.write("doc/product/a.md", "product")
        self.write("doc/engineering/a.md", "system")
        self.write("src/a.rs", "code")
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        self.base = self.git("rev-parse", "HEAD")
        self.binding = dict(schema="oasis7.loop-task/v1", task_uid="task_" + "a"*32,
            change_id="change-test", loop="product", owner_role="gameplay_designer",
            bootstrap_epoch=1, manual_request_ref="user-message-1", request_key="request-1",
            write_scope=["doc/product/**"], out_of_scope=[], input_contracts=[],
            acceptance_refs=["acceptance-1"], dependencies=[], target_delivery="pilot",
            policy_digest="sha256:"+hashlib.sha256((self.root / "scripts/pm/loop-policy.v1.json").read_bytes()).hexdigest(),
            policy_commit=self.base)

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def write(self, path, value):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(value)

    def check(self):
        self.git("add", ".")
        self.git("commit", "-qm", "candidate")
        return self.api.validate_scope(self.root, self.root, self.binding, self.base, self.git("rev-parse", "HEAD"))

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
                actual = self.api.validate_scope(self.root, self.root, self.binding, self.base, head)
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
            actual = self.api.validate_scope(self.root, self.root, self.binding, base,
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
        self.assertEqual(self.api.validate_tool_root(self.root,self.root,self.binding)["status"],"passed")
        self.write("scripts/pm/shadow.py","untracked executable authority")
        self.assertEqual(self.api.validate_tool_root(self.root,self.root,self.binding)["status"],"blocked")

    def test_candidate_commit_cannot_be_effective_tool(self):
        self.git("update-ref","refs/remotes/origin/main",self.base)
        self.write("doc/product/a.md","candidate")
        self.git("add",".")
        self.git("commit","-qm","candidate")
        self.binding["policy_commit"]=self.git("rev-parse","HEAD")
        self.assertEqual(self.api.validate_tool_root(self.root,self.root,self.binding)["status"],"blocked")

    def test_dependency_closure_cycle_rejected(self):
        import copy
        other=copy.deepcopy(self.binding)
        other["task_uid"]="task_"+"b"*32
        self.binding["dependencies"]=[other["task_uid"]]
        other["dependencies"]=[self.binding["task_uid"]]
        self.assertEqual(self.api.validate_dependencies(self.binding,{other["task_uid"]:other})["status"],"blocked")

if __name__ == "__main__":
    unittest.main()
