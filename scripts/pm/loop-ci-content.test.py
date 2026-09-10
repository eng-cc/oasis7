"""Repository-only hosted checks never claim local live admission."""
import hashlib
import copy
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from loop_ci_content import validate_ci_content
from loop_contracts import MARKER, contract_digest


class ContentTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(Path(__file__).parent, self.root / 'scripts/pm', ignore=shutil.ignore_patterns('__pycache__'))
        (self.root / '.gitignore').write_text('__pycache__/\n')
        self.spec = self.root / 'doc/engineering/spec.md'; self.spec.parent.mkdir(parents=True); self.spec.write_text('approved')
        self.git('init', '-q'); self.git('config', 'user.name', 'Fixture'); self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('add', '.'); self.git('commit', '-qm', 'effective')
        self.base = self.git('rev-parse', 'HEAD'); self.git('update-ref', 'refs/remotes/origin/main', self.base)
        self.contract = dict(schema='oasis7.loop-contract/v1', contract_id='S', revision=1, owner_loop='system', source_head=self.base, merged_head=self.base, approval_ref={'repository':'eng-cc/oasis7','pr_number':2}, content_refs=[{'path':'doc/engineering/spec.md','sha256':'sha256:'+hashlib.sha256(b'approved').hexdigest(),'clauses':['a']}], upstream_contracts=[], scope=['pilot'], eligibility={'new_tasks':False,'in_flight':False,'release':False})
        self.reference = dict(contract_id='S',revision=1,contract_digest=contract_digest(self.contract),publication_ref={'issue_number':1,'comment_id':3},consumed_clauses=['a'])
        self.binding = dict(schema='oasis7.loop-task/v1',task_uid='task_'+'a'*32,change_id='c',loop='system',owner_role='repository_health_engineer',bootstrap_epoch=1,manual_request_ref='user',request_key='r',write_scope=['doc/engineering/**'],out_of_scope=[],input_contracts=[self.reference],acceptance_refs=['a'],dependencies=[],target_delivery='pilot',policy_commit=self.base,policy_digest='sha256:'+hashlib.sha256((self.root/'scripts/pm/loop-policy.v1.json').read_bytes()).hexdigest())
        self.calls=[]

    def git(self,*args):
        return subprocess.check_output(['git','-C',str(self.root),*args],text=True).strip()

    def reader(self,repo,path):
        self.calls.append(path)
        if path=='issues/comments/3':
            return {'id':3,'issue_url':'https://api.github.com/repos/eng-cc/oasis7/issues/1','body':json.dumps({'marker':MARKER,'contract_digest':contract_digest(self.contract),'contract':self.contract})}
        if path=='pulls/2':
            return {'number':2,'merged':True,'head':{'sha':self.base},'merge_commit_sha':self.base,'base':{'repo':{'full_name':repo}}}
        self.fail('unexpected non-repository read: '+path)

    def check(self):
        return validate_ci_content(self.root,self.root,self.binding,self.base,self.base,'eng-cc/oasis7',self.reader)

    def test_content_pass_is_explicitly_not_live_eligibility_admission(self):
        result=self.check()
        self.assertEqual(result['status'],'passed',result)
        self.assertTrue(result['local_live_admission_required'])
        self.assertEqual(self.calls,['issues/comments/3','pulls/2'])

    def test_real_parallel_target_advance_uses_only_task_scope(self):
        tools=self.root.parent / (self.root.name+'-tools')
        self.git('worktree','add','--detach',str(tools),self.base)
        self.addCleanup(lambda: self.git('worktree','remove','--force',str(tools)))
        self.git('switch','-c','task')
        (self.root/'doc/engineering/task.md').write_text('task contract')
        self.git('add','.');self.git('commit','-qm','task change')
        head=self.git('rev-parse','HEAD')
        self.git('switch','--detach',self.base)
        (self.root/'unrelated.rs').write_text('main only')
        self.git('add','.');self.git('commit','-qm','parallel target')
        integration=self.git('rev-parse','HEAD')
        self.git('update-ref','refs/remotes/origin/main',integration)
        self.git('switch','task')
        result=validate_ci_content(tools,self.root,self.binding,integration,head,'eng-cc/oasis7',self.reader)
        self.assertEqual(result['status'],'passed',result)
        self.assertEqual(result['scope_context']['scope_base_oid'],self.base)
        self.assertEqual(result['scope_context']['integration_base_oid'],integration)

    def test_changed_publication_content_blocks(self):
        self.contract['content_refs'][0]['sha256']='sha256:'+'0'*64
        self.assertEqual(self.check()['status'],'blocked')

    def test_content_hash_mismatch_blocks_even_with_updated_reference(self):
        self.contract['content_refs'][0]['sha256']='sha256:'+'0'*64
        self.reference['contract_digest']=contract_digest(self.contract)
        self.assertEqual(self.check()['status'],'blocked')

    def test_conflicting_revision_cannot_hide_second_upstream_graph(self):
        bad = copy.deepcopy(self.contract); bad['contract_id'] = 'UP'
        bad['content_refs'][0]['sha256'] = 'sha256:'+'0'*64
        upstream = dict(self.reference,contract_id='UP',contract_digest=contract_digest(bad),publication_ref={'issue_number':1,'comment_id':5})
        second = copy.deepcopy(self.contract); second['upstream_contracts'] = [upstream]
        self.binding['input_contracts'].append(dict(self.reference,contract_digest=contract_digest(second),publication_ref={'issue_number':1,'comment_id':4}))
        def reader(repo,path):
            if path in ('issues/comments/4','issues/comments/5'):
                contract = second if path.endswith('/4') else bad
                return {'id':int(path.rsplit('/',1)[1]),'issue_url':'https://api.github.com/repos/eng-cc/oasis7/issues/1',
                        'body':json.dumps({'marker':MARKER,'contract_digest':contract_digest(contract),'contract':contract})}
            return self.reader(repo,path)
        result = validate_ci_content(self.root,self.root,self.binding,self.base,self.base,'eng-cc/oasis7',reader)
        self.assertEqual(result['status'],'blocked',result)

    def test_equivalent_revision_at_two_publications_remains_valid(self):
        self.binding['input_contracts'].append(dict(self.reference,publication_ref={'issue_number':1,'comment_id':4}))
        def reader(repo,path):
            if path=='issues/comments/4':
                return dict(self.reader(repo,'issues/comments/3'),id=4)
            return self.reader(repo,path)
        result = validate_ci_content(self.root,self.root,self.binding,self.base,self.base,'eng-cc/oasis7',reader)
        self.assertEqual(result['status'],'passed',result)

    def test_equivalent_revision_rechecks_second_publication_target(self):
        self.binding['input_contracts'].append(dict(self.reference,publication_ref={'issue_number':1,'comment_id':4}))
        def reader(repo,path):
            if path=='issues/comments/4':
                return dict(self.reader(repo,'issues/comments/3'),id=4,issue_url='https://api.github.com/repos/eng-cc/oasis7/issues/99')
            return self.reader(repo,path)
        result = validate_ci_content(self.root,self.root,self.binding,self.base,self.base,'eng-cc/oasis7',reader)
        self.assertEqual(result['status'],'blocked',result)


if __name__=='__main__': unittest.main()
