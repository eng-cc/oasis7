#!/usr/bin/env python3
"""Injected-transport trust regressions for the live lifecycle gate."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import pathlib
import subprocess
import types
import tempfile
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
UID = "task_11111111111111111111111111111111"
REPOSITORY = "eng-cc/oasis7"
ISSUE = 2198
PR = 2198
HEAD = "a" * 40


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


API = load_module("pr_lifecycle_trust_live_api", ROOT / "scripts/pm/github_api.py")
GATE = load_module("pr_lifecycle_trust_live_gate", ROOT / "scripts/pm/pr-lifecycle-gate.py")


def connection(nodes=None, *, has_next=False):
    return {"pageInfo": {"hasNextPage": has_next, "endCursor": "cursor" if has_next else None},
            "nodes": nodes or []}


def snapshot_payload(*, comments=None, comments_incomplete=False):
    pr = {
        "number": PR,
        "url": f"https://github.com/{REPOSITORY}/pull/{PR}",
        "state": "OPEN",
        "isDraft": False,
        "body": f"Task: {UID}\nRefs #{ISSUE}",
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefName": "codex/task",
        "headRefOid": HEAD,
        "baseRefName": "main",
        "baseRefOid": "b" * 40,
        "comments": connection(comments, has_next=comments_incomplete),
        "reviews": connection(),
        "reviewThreads": connection(),
        "commits": {"nodes": [{"commit": {"oid": HEAD,
            "statusCheckRollup": {"contexts": connection()}}}]},
    }
    return {"data": {
        "viewer": {"login": "fixture-viewer"},
        "rateLimit": {"cost": 1, "remaining": 499, "used": 1, "limit": 500,
                      "resetAt": "2099-01-01T00:00:00Z"},
        "repository": {"nameWithOwner": REPOSITORY, "pullRequest": pr},
    }}


def active_hold_comment():
    body = "\n".join((
        "<!-- oasis7-merge-hold -->",
        f"- task_uid: `{UID}`",
        f"- repository: `{REPOSITORY}`",
        f"- issue_number: `{ISSUE}`",
        f"- pr_number: `{PR}`",
        f"- head_oid: `{HEAD}`",
        "- node_id: `merge_hold`",
        "- kind: `merge_hold`",
        "- disposition: `active`",
        "- hold_kind: `user_requested_merge_hold`",
        "- active: `true`",
        "- requester: `user`",
        "- reason: `do not merge`",
        "- resume_authority: `user`",
    ))
    return {
        "id": 501,
        "body": body,
        "user": {"login": "user"},
        "created_at": "2026-10-01T00:00:00Z",
        "html_url": f"https://github.com/{REPOSITORY}/issues/{ISSUE}#issuecomment-501",
    }


class InjectedTransport:
    def __init__(self, *, issue_comments=None, pr_comments=None, incomplete=False):
        self.issue_comments = issue_comments or []
        self.pr_comments = pr_comments or []
        self.incomplete = incomplete
        self.calls = []

    def __call__(self, method, url, headers, body, timeout):
        self.calls.append((method, url, body, dict(headers), timeout))
        if url.endswith("/graphql"):
            query = json.loads(body.decode("utf-8"))["query"]
            if "GitHubPRSnapshot" not in query:
                raise AssertionError(f"unexpected GraphQL operation: {query[:80]}")
            payload = snapshot_payload(comments=self.pr_comments,
                                       comments_incomplete=self.incomplete)
            return API.HTTPResponse(200, {}, json.dumps(payload))
        if "/branches/main/protection" in url:
            return API.HTTPResponse(404, {}, json.dumps({"message": "Not Found"}))
        if "/rulesets?" in url:
            return API.HTTPResponse(200, {}, "[]")
        if f"/issues/{ISSUE}/comments?" in url:
            return API.HTTPResponse(200, {}, json.dumps(self.issue_comments))
        raise AssertionError(f"unexpected GitHub request: {method} {url}")


class LiveLifecycleTrustTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name).resolve()
        self.mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        self.mapping_path.parent.mkdir(parents=True)
        self.record = {
            "task_uid": UID,
            "repository": REPOSITORY,
            "issue_number": ISSUE,
            "pr_number": PR,
            "pr_url": f"https://github.com/{REPOSITORY}/pull/{PR}",
            "canonical_worktree": str(self.root),
            "merge_hold": {
                "kind": "normal_pr_ci_watch", "active": False,
                "requester": "workflow", "reason": "normal", "resume_authority": "workflow",
            },
        }
        self.write_mapping()

    def tearDown(self):
        self.temp.cleanup()

    def write_mapping(self, extra=None):
        record = {**self.record, **(extra or {})}
        self.mapping_path.write_text(json.dumps({"version": 1, "tasks": {UID: record}}),
                                     encoding="utf-8")

    def run_gate(self, transport):
        client = API.GitHubAPIClient(
            "fixture-token", transport=transport,
            state_root=self.root / "api-state", clock=lambda: 1790860000.0,
            sleeper=lambda _seconds: None, random_value=lambda: 0,
        )
        old_argv = GATE.sys.argv
        GATE.sys.argv = [
            "pr-lifecycle-gate.py", str(PR), "--root", str(self.root),
            "--tool-root", str(ROOT), "--task-uid", UID, "--json",
        ]
        output = io.StringIO()
        try:
            with mock.patch.object(GATE, "_github_api_client", return_value=client), \
                 mock.patch.object(GATE, "local_loop_admission",
                                   side_effect=AssertionError("blocked gate reached loop admission")), \
                 contextlib.redirect_stdout(output):
                status = GATE.main()
        finally:
            GATE.sys.argv = old_argv
        return status, json.loads(output.getvalue()), transport

    def test_authorized_immutable_candidate_current_target_receipt_reaches_live_consumer(self):
        def git(*args):
            return subprocess.check_output(['git','-C',str(self.root),*args],text=True).strip()
        git('init','-q','-b','main');git('config','user.name','Fixture');git('config','user.email','fixture@example.invalid')
        names=('ci-ready-receipt.py','ci_ready_receipt_identity.py','integration_ci.py',
               'integration_executor_contract.py','workflow_maintenance.py')
        for name in names:
            path=self.root/'scripts/pm'/name;path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(subprocess.check_output(['git','-C',str(ROOT),'show','HEAD:scripts/pm/'+name]))
        git('add','.');git('commit','-qm','protected Q authority');q=git('rev-parse','HEAD')
        helper=self.root/'scripts/pm/ci-ready-receipt.py'
        helper.write_bytes(helper.read_bytes()+b'\n# authenticated candidate receipt fix under test\n')
        git('add','.');git('commit','-qm','approved immutable candidate H helper');h=git('rev-parse','HEAD')
        tree=git('rev-parse','HEAD^{tree}')
        e=git('commit-tree',tree,'-p',q,'-p',h,'-m','actual tested checkout E')
        scope=dict(repository=REPOSITORY,task_uid=UID,issue_number=ISSUE,pr_number=PR,
                   purpose='candidate-tool-verification',allowed_write_paths=['scripts/pm/ci-ready-receipt.py'],
                   allowed_tool_paths=['scripts/pm/ci-ready-receipt.py','scripts/pm/ci_ready_receipt_identity.py','scripts/pm/workflow_maintenance.py'])
        comment=dict(id=700,body='Workflow Maintenance Authority:\n```json\n'+json.dumps(scope)+'\n```',
            issue_url=f'https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE}',
            html_url=f'https://github.com/{REPOSITORY}/issues/{ISSUE}#issuecomment-700',
            created_at='2026-10-01T00:00:00Z',updated_at='2026-10-01T00:00:00Z',
            user={'login':'owner','type':'User'})
        body=f'Task: {UID}\nRefs #{ISSUE}\nWorkflow Maintenance Authority: 700'
        pull=dict(number=PR,html_url=f'https://github.com/{REPOSITORY}/pull/{PR}',body=body,
            state='open',merged_at=None,draft=False,
            head={'sha':h,'ref':'main','repo':{'full_name':REPOSITORY}},
            base={'sha':q,'ref':'main','repo':{'full_name':REPOSITORY}})
        def read(command):
            endpoint=command[-1]
            if endpoint.endswith('/issues/comments/700'):return comment
            if endpoint.endswith('/permission'):return {'permission':'admin','user':{'login':'owner'}}
            if endpoint.endswith('/pulls/'+str(PR)):return pull
            if endpoint.endswith('/issues/'+str(ISSUE)):return {'number':ISSUE,'state':'open','html_url':f'https://github.com/{REPOSITORY}/issues/{ISSUE}','body':f'task_uid: {UID}'}
            if endpoint.endswith('/git/ref/heads/main'):return {'object':{'sha':q}}
            if endpoint=='repos/'+REPOSITORY:return {'full_name':REPOSITORY,'default_branch':'main'}
            raise AssertionError('unhandled exact consumer fixture endpoint: '+endpoint)
        proof=dict(integration_base_oid=q,base_ref='main',head_oid=h,check_name='required-gate',
            check_run_id=9,check_app_id=42,planner_digest='sha256:'+'a'*64,
            ci_validation_mode='current_target_pr',assessed_target_oid=q,
            workflow_run_id=12345,workflow_sha=h,tested_tree_oid=tree,tested_commit_oid=e,
            current_target_proof={'schema':'oasis7-current-target-pr/v1','repository':REPOSITORY,
                'task_uid':UID,'task_issue_number':ISSUE,'pr_number':PR,'source_head_oid':h,
                'current_target_oid':q,'checkout_oid':e,'tested_tree_oid':tree,
                'checkout_parent_oids':[q,h],'workflow_revision':h,'workflow_run_id':12345,
                'workflow_run_attempt':1,'maintenance_authority_comment_id':700,
                'planner_config_sha256':'b'*64,'test_driver_sha256':'c'*64})
        data=dict(repository=REPOSITORY,number=PR,baseRefName='main',baseRefOid=q,headRefOid=h,body=body,
                  policy_discovery={'status':'resolved','required_status_checks':[{'context':'required-gate','app_id':42}]})
        admission=dict(status='admitted',tool_root=str(self.root),policy_commit=q,trusted_default_oid=q,
                       task={**self.record,'repository':REPOSITORY})
        child=types.SimpleNamespace(returncode=0,stdout=json.dumps(proof),stderr='')
        real_run=subprocess.run
        def child_only(command, *args, **kwargs):
            if len(command)>2 and command[1:3]==['-I','-c']: return child
            return real_run(command,*args,**kwargs)
        with mock.patch.object(GATE,'_run_json',side_effect=read),mock.patch.object(GATE.subprocess,'run',side_effect=child_only):
            result=GATE.live_integration_admission(data,self.root,UID,self.root,admission,require_strict=True,assessed_target_oid=q)
        self.assertEqual('current_target_pr',result['ci_validation_mode'])
        self.assertEqual(h,result['head_oid'])

    def test_live_task_issue_hold_is_rebuilt_over_injected_shared_transport(self):
        transport = InjectedTransport(issue_comments=[active_hold_comment()])
        status, result, transport = self.run_gate(transport)

        self.assertEqual(status, 3, result)
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(any("user_requested_merge_hold" in item for item in result["blockers"]), result)
        self.assertNotIn("readiness_receipt", result)
        self.assertTrue(any(url.endswith("/graphql") for _method, url, *_rest in transport.calls))
        self.assertTrue(any("/issues/2198/comments?" in url for _method, url, *_rest in transport.calls))

    def test_caller_cached_disposition_cannot_hide_live_actionable_comment(self):
        self.write_mapping({"comment_dispositions": [{
            "node_id": "page-two", "head_oid": HEAD, "disposition": "addressed",
            "evidence": "caller-authored cache text",
        }]})
        transport = InjectedTransport(pr_comments=[{
            "id": "page-two", "body": "please fix the page-two issue",
            "url": f"https://github.com/{REPOSITORY}/pull/{PR}#issuecomment-2",
            "author": {"login": "reviewer"}, "authorAssociation": "NONE",
        }])
        status, result, _transport = self.run_gate(transport)

        self.assertEqual(status, 3, result)
        self.assertFalse(result["ready_for_merge"])
        self.assertTrue(any("issuecomment-2" in item for item in result["blockers"]), result)
        self.assertNotIn("readiness_receipt", result)

    def test_incomplete_live_pr_connection_fails_before_policy_or_receipt(self):
        transport = InjectedTransport(incomplete=True)
        status, result, transport = self.run_gate(transport)

        self.assertEqual(status, 2, result)
        self.assertEqual(result["status"], "capability_blocked")
        self.assertIn("incomplete or not fresh", result["error"])
        self.assertFalse(any("/branches/" in url or "/rulesets?" in url
                             for _method, url, *_rest in transport.calls))
        self.assertNotIn("readiness_receipt", result)


if __name__ == "__main__":
    unittest.main()
