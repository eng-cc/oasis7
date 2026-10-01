#!/usr/bin/env python3
"""Behavior tests for immutable loop scope policy; temporary Git only."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
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
        result = self.api.validate_scope(self.root, self.root, self.binding, context['scope_base_oid'], head)
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
        self.git("config", "remote.origin.url", "https://github.com/eng-cc/oasis7.git")
        self.write("scripts/pm/loop-policy.v1.json", (HERE / "loop-policy.v1.json").read_text())
        self.write("doc/engineering/workflow/source-of-truth.md",
                   (HERE.parents[1] / "doc/engineering/workflow/source-of-truth.md").read_text())
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
