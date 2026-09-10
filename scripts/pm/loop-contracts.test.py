#!/usr/bin/env python3
"""Contract eligibility must be live-authority derived, not caller approved."""
import copy
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import subprocess
import unittest
from types import SimpleNamespace
from unittest.mock import patch

HERE = Path(__file__).resolve().parent

class ContractTests(unittest.TestCase):
    def test_real_squash_contract_acquires_exact_source_in_separate_clean_clone(self):
        origin = Path(self.tmp.name) / 'remote.git'
        author = self.root
        self.git('switch', '-c', 'contract-source')
        (author / 'spec.md').write_text('real squash content')
        self.git('add', '.'); self.git('commit', '-qm', 'source delta')
        source = self.git('rev-parse', 'HEAD')
        self.git('switch', '--detach', self.merged)
        self.git('switch', '-C', 'main')
        self.git('merge', '--squash', 'contract-source')
        self.git('commit', '-qm', 'actual squash')
        merged = self.git('rev-parse', 'HEAD')
        subprocess.run(['git', 'init', '--bare', '-q', str(origin)], check=True)
        self.git('remote', 'add', 'origin', str(origin))
        self.git('push', '-q', 'origin', 'main', source + ':refs/pull/12/head')
        self.git('branch', '-D', 'contract-source')
        clean = Path(self.tmp.name) / 'clean'
        subprocess.run(['git', 'clone', '-q', '--no-local', '--single-branch', '--branch', 'main', str(origin), str(clean)], check=True)
        absent = subprocess.run(['git', '-C', str(clean), 'cat-file', '-e', source + '^{commit}'], capture_output=True)
        self.assertNotEqual(absent.returncode, 0)
        subprocess.run(['git', '-C', str(clean), 'remote', 'set-url', 'origin', 'https://github.com/eng-cc/oasis7.git'], check=True)
        subprocess.run(['git', '-C', str(clean), 'config', 'url.' + str(origin) + '.insteadOf', 'https://github.com/eng-cc/oasis7.git'], check=True)
        contract = copy.deepcopy(self.contract)
        contract.update(source_head=source, merged_head=merged)
        contract['content_refs'][0]['sha256'] = 'sha256:' + hashlib.sha256(b'real squash content').hexdigest()
        result = self.api.validate_contract_record(contract, clean, {'number': 12, 'merged': True, 'head': source, 'merge_commit': merged})
        self.assertEqual(result, [])
        self.assertEqual(subprocess.run(['git', '-C', str(clean), 'cat-file', '-e', source + '^{commit}'], capture_output=True).returncode, 0)

    def test_missing_object_never_fetches_before_approval_identity(self):
        contract=copy.deepcopy(self.contract);contract['source_head']='a'*40
        with patch.object(self.api,'ensure_contract_objects') as acquire:
            errors=self.api.validate_contract_record(contract,self.root,self.record['pr'])
        self.assertTrue(errors)
        acquire.assert_not_called()

    def test_missing_object_wrong_origin_fails_closed(self):
        self.git('remote','add','origin','https://github.com/foreign/repository.git')
        contract=copy.deepcopy(self.contract);contract['source_head']='a'*40
        errors=self.api.validate_contract_record(contract,self.root,{'number':12,'merged':True,'head':'a'*40,'merge_commit':self.merged})
        self.assertTrue(any('origin' in error for error in errors),errors)

    def test_default_obligation_uses_terminal_project_and_receipt_reader(self):
        calls = []
        def terminal(repo, uid, number):
            calls.append((repo, uid, number))
            return {'status': 'passed', 'blockers': []}
        uid = 'task_' + 'b' * 32
        with patch.dict('sys.modules', {'loop_terminal': SimpleNamespace(validate_terminal_delivery=terminal)}):
            self.assertTrue(self.api.GitHubAuthority(self.root).obligation({'task_uid': uid, 'issue_number': 11}))
        self.assertEqual(calls, [('eng-cc/oasis7', uid, 11)])

    def test_default_obligation_rejects_unfinished_terminal_reader(self):
        reader = SimpleNamespace(validate_terminal_delivery=lambda *args: {'status': 'blocked', 'blockers': ['finalizer receipt missing']})
        with patch.dict('sys.modules', {'loop_terminal': reader}):
            self.assertFalse(self.api.GitHubAuthority(self.root).obligation({'task_uid': 'task_' + 'b' * 32, 'issue_number': 11}))

    def setUp(self):
        spec = importlib.util.spec_from_file_location("loop_contracts", HERE / "loop_contracts.py")
        self.api = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.api)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        (self.root / "spec.md").write_text("approved content")
        self.git("add", ".")
        self.git("commit", "-qm", "approved")
        self.source = self.git("rev-parse", "HEAD")
        self.git("commit", "--allow-empty", "-qm", "squash equivalent")
        self.merged = self.git("rev-parse", "HEAD")
        self.contract = dict(schema="oasis7.loop-contract/v1", contract_id="S", revision=1,
            owner_loop="system", source_head=self.source, merged_head=self.merged,
            approval_ref={"repository": "eng-cc/oasis7", "pr_number": 12},
            content_refs=[{"path":"spec.md", "sha256":"sha256:"+hashlib.sha256(b"approved content").hexdigest(), "clauses":["section-1"]}],
            upstream_contracts=[], scope=["pilot"], eligibility={"new_tasks":True,"in_flight":True,"release":True})
        self.ref = {"contract_id":"S", "revision":1,"contract_digest":self.api.contract_digest(self.contract),"publication_ref":{"issue_number":11,"comment_id":123},"consumed_clauses":["section-1"]}
        self.record = {"repository":"eng-cc/oasis7","issue_number":11,"comment_id":123,"author":"owner","permission":"admin","task_uid":"task_"+"b"*32,"contract":copy.deepcopy(self.contract),"pr":{"number":12,"merged":True,"head":self.source,"merge_commit":self.merged}}

    def git(self,*args):
        return subprocess.check_output(["git","-C",str(self.root),*args],text=True).strip()

    def check(self, purpose="in_flight"):
        return self.api.validate_contracts(self.root,self.root,{"input_contracts":[self.ref],"target_delivery":"pilot"},authority_reader=lambda ref:copy.deepcopy(self.record),purpose=purpose)

    def test_squash_equivalent_content_accepted(self):
        self.assertNotEqual(self.source,self.merged)
        self.assertEqual(self.check()["status"],"passed")

    def test_revoked_input_blocks_next_admission(self):
        self.record["contract"]["eligibility"]["in_flight"]=False
        self.assertEqual(self.check()["status"],"blocked")

    def test_new_task_and_release_eligibility_are_separate(self):
        self.record["contract"]["eligibility"]["new_tasks"]=False
        self.assertEqual(self.check()["status"],"passed")
        self.assertEqual(self.check("new_tasks")["status"],"blocked")

    def test_forged_local_approval_is_ignored(self):
        self.ref["approved"]=True
        self.record["permission"]="write"
        self.assertEqual(self.check()["status"],"blocked")

    def test_wrong_publication_or_revision_rejected(self):
        self.record["comment_id"]=999
        self.assertEqual(self.check()["status"],"blocked")
        self.record["comment_id"]=123
        self.record["contract"]["revision"]=2
        self.assertEqual(self.check()["status"],"blocked")

    def test_changed_published_content_rejected(self):
        (self.root/"spec.md").write_text("unapproved")
        self.git("add",".")
        self.git("commit","-qm","drift")
        changed=self.git("rev-parse","HEAD")
        self.record["pr"]["merge_commit"]=changed
        self.record["contract"]["merged_head"]=changed
        self.ref["contract_digest"]=self.api.contract_digest(self.record["contract"])
        self.assertEqual(self.check()["status"],"blocked")

    def test_unknown_clause_and_delivery_rejected(self):
        self.ref["consumed_clauses"]=["missing"]
        self.assertEqual(self.check()["status"],"blocked")

    def test_unfinished_delivery_obligation_blocks_release(self):
        binding={"input_contracts":[self.ref],"target_delivery":"pilot", "delivery_obligations":[{"id":"manual", "task_uid":"task_"+"c"*32}]}
        result=self.api.validate_contracts(self.root,self.root,binding,authority_reader=lambda ref:copy.deepcopy(self.record),purpose="release")
        self.assertEqual(result["status"],"blocked")

    def test_live_reader_uses_server_identity_and_current_permission(self):
        reader=self.api.GitHubAuthority(self.root)
        payload={"marker":self.api.MARKER,"task_uid":self.record["task_uid"],"contract_digest":self.api.contract_digest(self.contract),"contract":self.contract}
        import json
        def api(path,body=None,paginate=False):
            if path.endswith("/issues/11"):
                return {"number":11,"body":"<!-- oasis7-pm-task -->\ntask_uid: "+self.record["task_uid"]}
            if path.endswith("/issues/comments/123"):
                return {"id":123,"issue_url":"https://api.github.com/repos/eng-cc/oasis7/issues/11","body":json.dumps(payload),"user":{"login":"owner"}}
            if path.endswith("/collaborators/owner/permission"):
                return {"permission":"admin"}
            if path.endswith("/pulls/12"):
                return {"number":12,"merged":True,"head":{"sha":self.source},"merge_commit_sha":self.merged,"base":{"repo":{"full_name":"eng-cc/oasis7"}}}
            self.fail(path)
        reader.api=api
        self.assertEqual(reader(self.ref)["permission"],"admin")
        payload["task_uid"]="task_"+"d"*32
        with self.assertRaises(ValueError):
            reader(self.ref)

    def check_production_reader_issue(self, body, contracts=None, references=None, comment_overrides=None, permissions=None):
        import json
        contracts = contracts or {123:self.contract}
        def transport(path, body=None, paginate=False):
            if path.endswith('/issues/11'): return issue
            if '/issues/comments/' in path:
                number = int(path.rsplit('/',1)[1]); contract = contracts[number]
                payload = {"marker":self.api.MARKER,"task_uid":self.record["task_uid"],
                           "contract_digest":self.api.contract_digest(contract),"contract":contract}
                return {"id":number,"issue_url":"https://api.github.com/repos/eng-cc/oasis7/issues/11",
                        "body":json.dumps(payload),"user":{"login":"owner"}, **(comment_overrides or {}).get(number,{})}
            if '/collaborators/' in path:
                return {"permission":(permissions or {}).get(path.split('/')[-2],'admin')}
            if path.endswith('/pulls/12'):
                return {"number":12,"merged":True,"head":{"sha":self.source},"merge_commit_sha":self.merged,
                        "base":{"repo":{"full_name":"eng-cc/oasis7"}}}
            self.fail('unexpected authority request: ' + path)
        issue = {'number':11,'body':'<!-- oasis7-pm-task -->\n'+body}
        with patch.object(self.api.GitHubAuthority, 'api', side_effect=transport):
            return self.api.validate_contracts(self.root,self.root,{'input_contracts':references or [self.ref],'target_delivery':'pilot'})

    def test_conflicting_revision_cannot_hide_second_upstream_graph(self):
        bad = copy.deepcopy(self.contract); bad['contract_id'] = 'UP'
        bad['content_refs'][0]['sha256'] = 'sha256:'+'0'*64
        upstream = dict(self.ref,contract_id='UP',contract_digest=self.api.contract_digest(bad),publication_ref={'issue_number':11,'comment_id':125})
        second = copy.deepcopy(self.contract); second['upstream_contracts'] = [upstream]
        second_ref = dict(self.ref,contract_digest=self.api.contract_digest(second),publication_ref={'issue_number':11,'comment_id':124})
        result = self.check_production_reader_issue('task_uid: '+self.record['task_uid'],
            {123:self.contract,124:second,125:bad},[self.ref,second_ref])
        self.assertEqual(result['status'],'blocked',result)

    def test_equivalent_revision_at_two_publications_remains_valid(self):
        second_ref = dict(self.ref,publication_ref={'issue_number':11,'comment_id':124})
        result = self.check_production_reader_issue('task_uid: '+self.record['task_uid'],
            {123:self.contract,124:copy.deepcopy(self.contract)},[self.ref,second_ref])
        self.assertEqual(result['status'],'passed',result)

    def test_equivalent_revision_rechecks_second_publication_authority(self):
        second_ref = dict(self.ref,publication_ref={'issue_number':11,'comment_id':124})
        for override in ({'user':{'login':'other'}}, {'issue_url':'https://api.github.com/repos/eng-cc/oasis7/issues/99'}):
            with self.subTest(override=override):
                result = self.check_production_reader_issue('task_uid: '+self.record['task_uid'],
                    {123:self.contract,124:copy.deepcopy(self.contract)},[self.ref,second_ref],
                    {124:override},{'other':'write'})
                self.assertEqual(result['status'],'blocked',result)

    def test_equivalent_revision_rechecks_second_live_eligibility(self):
        second = copy.deepcopy(self.contract); second['eligibility']['in_flight'] = False
        self.assertEqual(self.api.contract_digest(second),self.ref['contract_digest'])
        second_ref = dict(self.ref,publication_ref={'issue_number':11,'comment_id':124})
        result = self.check_production_reader_issue('task_uid: '+self.record['task_uid'],
            {123:self.contract,124:second},[self.ref,second_ref])
        self.assertEqual(result['status'],'blocked',result)

    def test_production_contract_reader_accepts_single_canonical_uid(self):
        result = self.check_production_reader_issue('task_uid: '+self.record['task_uid'])
        self.assertEqual(result['status'],'passed',result)
        self.assertEqual(result['authority'],'live_github')
        self.assertEqual(result['contracts_checked'],1)

    def test_production_contract_reader_blocks_ambiguous_canonical_uid(self):
        canonical = 'task_uid: '+self.record['task_uid']
        for competing in (canonical, 'task_uid: task_'+'a'*32, 'task_uid: malformed'):
            with self.subTest(competing=competing):
                result = self.check_production_reader_issue(canonical+'\n'+competing)
                self.assertEqual(result['status'],'blocked',result)
                self.assertTrue(any('identity mismatch' in error for error in result['blockers']),result)

    def test_production_contract_reader_blocks_missing_and_mention_only_uid(self):
        for body in ('', 'mentions '+self.record['task_uid'], 'task_uid: malformed'):
            with self.subTest(body=body):
                result = self.check_production_reader_issue(body)
                self.assertEqual(result['status'],'blocked',result)

    def test_publish_retry_reuses_exact_server_comment(self):
        reader=self.api.GitHubAuthority(self.root)
        import json
        payload={"marker":self.api.MARKER,"task_uid":self.record["task_uid"],"contract_digest":self.api.contract_digest(self.contract),"contract":self.contract}
        posts=[]
        def api(path,body=None,paginate=False):
            if path.endswith("/issues/11"):
                return {"number":11,"body":"<!-- oasis7-pm-task -->\ntask_uid: "+self.record["task_uid"]}
            if path=="user":
                return {"login":"owner"}
            if path.endswith("/permission"):
                return {"permission":"admin"}
            if paginate:
                return [[{"id":123,"body":json.dumps(payload)}]]
            posts.append(body)
            self.fail("unexpected write on reconciled retry")
        reader.api=api
        self.assertEqual(reader.publish({"issue_number":11,"task_uid":self.record["task_uid"]},self.contract),{"issue_number":11,"comment_id":123})
        self.assertEqual(posts,[])

    def test_same_digest_does_not_authorize_substituted_clauses(self):
        self.ref["consumed_clauses"]=["section-1","new-unapproved"]
        self.assertEqual(self.check()["status"],"blocked")

if __name__=="__main__":
    unittest.main()
