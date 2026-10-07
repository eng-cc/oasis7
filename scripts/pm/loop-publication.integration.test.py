"""Real facade, effective linked checkout and fake GitHub publication transaction."""
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
import copy

HERE = Path(__file__).parent
UID = 'task_' + 'a' * 32
FAKE = '''#!/usr/bin/env python3
import json,os,pathlib,sys
p=pathlib.Path(os.environ['GH_FIXTURE']); s=json.loads(p.read_text()); a=sys.argv[1:]
if a[:2]==['issue','list']:
 issue=s['issue']; result=[{'number':issue['number'],'url':issue['html_url'],'title':issue['title'],'state':issue['state']}]
elif a[:2]==['issue','view']:
 issue=s['issue']; result={'number':issue['number'],'url':issue['html_url'],'title':issue['title'],'state':issue['state'],'stateReason':None,'updatedAt':issue['updated_at'],'body':issue['body']}
elif a[:1]!=['api']:
 raise SystemExit('unexpected fake request '+repr(a))
else:
 path=a[1]
 if path=='user': result={'login':'owner','type':'User'}
 elif path=='repos/eng-cc/oasis7': result={'full_name':'eng-cc/oasis7','default_branch':'main'}
 elif path=='repos/eng-cc/oasis7/branches/main': result={'name':'main','protected':True,'commit':{'sha':s['default_oid']}}
 elif '/contents/' in path:
  content_path=path.split('/contents/',1)[1]; rel,sep,ref=content_path.partition('?ref=')
  if not sep or ref!=s['default_oid'] or rel not in s['contents']: raise SystemExit('unexpected fake content request '+repr(path))
  result={'encoding':'base64','content':s['contents'][rel]}
 elif '/collaborators/' in path: result={'permission':'admin'}
 elif '/pulls/' in path: result=s['pr']
 elif '/issues/comments/' in path: result=s['comments'][int(path.rsplit('/',1)[1])-1]
 elif path.split('?')[0].endswith('/comments'):
  if '--method' in a:
   body=json.load(sys.stdin)['body']; comment_id=len(s['comments'])+1
   result={'id':comment_id,'body':body,'issue_url':'https://api.github.com/repos/eng-cc/oasis7/issues/1','html_url':f'https://github.com/eng-cc/oasis7/issues/1#issuecomment-{comment_id}','created_at':'2026-10-04T00:00:00Z','updated_at':'2026-10-04T00:00:00Z','author_association':'MEMBER','user':{'login':'owner','type':'User'}}
   s['comments'].append(result); p.write_text(json.dumps(s))
   if os.environ.get('GH_LOSE_POST'): raise SystemExit('simulated lost POST response')
  else: result=[s['comments']]
 elif path.endswith('/issues/1'): result=s['issue']
 else: raise SystemExit('unexpected fake request '+repr(a))
print(json.dumps(result))
'''


class PublicationIntegration(unittest.TestCase):
    def test_publish_and_retry_use_one_comment_and_resolved_journal(self):
        self.assert_publication(in_flight=True, lose_response=True)

    def test_new_only_output_publication_resolves_journal(self):
        self.assert_publication(in_flight=False, lose_response=False)

    def test_new_only_output_lost_response_recovers_and_retries(self):
        self.assert_publication(in_flight=False, lose_response=True)

    def assert_publication(self, *, in_flight, lose_response):
        with tempfile.TemporaryDirectory() as tmp:
            temp = Path(tmp)
            root = temp / 'repo'
            shutil.copytree(HERE, root / 'scripts/pm', ignore=shutil.ignore_patterns('__pycache__'))
            (root / '.gitignore').write_text('.pm/\n__pycache__/\n')
            spec = root / 'doc/engineering/spec.md'
            spec.parent.mkdir(parents=True)
            spec.write_text('approved specification')
            workflow_source = HERE.parents[1] / 'doc/engineering/workflow/source-of-truth.md'
            workflow_target = root / 'doc/engineering/workflow/source-of-truth.md'
            workflow_target.parent.mkdir(parents=True)
            workflow_target.write_bytes(workflow_source.read_bytes())
            git = lambda *args: subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q', '-b', 'main'); git('config', 'user.name', 'Fixture'); git('config', 'user.email', 'fixture@example.invalid')
            git('add', '.'); git('commit', '-qm', 'effective')
            base = git('rev-parse', 'HEAD')
            origin = temp / 'origin.git'
            subprocess.run(['git', 'init', '--bare', '-q', str(origin)], check=True)
            canonical_origin = 'https://github.com/eng-cc/oasis7.git'
            git('remote', 'add', 'origin', canonical_origin)
            git('config', '--add', 'url.' + origin.as_uri() + '.insteadOf', canonical_origin)
            git('push', '-q', 'origin', 'main')
            git('switch', '-c', 'codex/publication')
            git('commit', '--allow-empty', '-qm', 'task publication branch head')
            trusted = temp / 'trusted'; git('worktree', 'add', str(trusted), 'main')
            binding = dict(schema='oasis7.loop-task/v1', task_uid=UID, change_id='c', loop='system', owner_role='repository_health_engineer', bootstrap_epoch=1, manual_request_ref='user-1', request_key='r', write_scope=['doc/engineering/**'], out_of_scope=[], input_contracts=[], acceptance_refs=['a'], dependencies=[], target_delivery='pilot', policy_commit=base, policy_digest='sha256:' + hashlib.sha256((root / 'scripts/pm/loop-policy.v1.json').read_bytes()).hexdigest())
            mapping = root / '.pm/github-project-sync/tasks.json'; mapping.parent.mkdir(parents=True)
            mapping.write_text(json.dumps({'tasks': {UID: {**binding, 'loop_binding': binding, 'repository': 'eng-cc/oasis7', 'issue_number': 1, 'canonical_worktree': str(root), 'task_branch': 'codex/publication'}}}))
            body = ('<!-- oasis7-pm-task -->\ntask_uid: ' + UID + '\n\nGitHub-backed oasis7 PM task.\n\nTask metadata:\n'
                    + '- owner_role: `repository_health_engineer`\n- status: `committed`\n- workflow_phase: `execution`\n'
                    + f'- worktree_hint: `{root}`\n- loop_binding_b64: `' + base64.urlsafe_b64encode(json.dumps(binding,sort_keys=True,separators=(',',':')).encode()).decode().rstrip('=') + '`\n')
            state = temp / 'github.json'
            content_paths = ('scripts/pm/loop-policy.v1.json', 'doc/engineering/workflow/source-of-truth.md')
            contents = {path: base64.b64encode(subprocess.check_output(['git', '-C', str(root), 'show', f'{base}:{path}'])).decode('ascii') for path in content_paths}
            issue = {'number': 1, 'html_url': 'https://github.com/eng-cc/oasis7/issues/1', 'title': '[PM] publication fixture', 'state': 'open', 'updated_at': '2026-10-04T00:00:00Z', 'user': {'login': 'owner', 'type': 'User'}, 'body': body}
            state.write_text(json.dumps({'default_oid': base, 'contents': contents, 'comments': [], 'issue': issue, 'pr': {'number': 2, 'merged': True, 'head': {'sha': base}, 'merge_commit_sha': base, 'base': {'repo': {'full_name': 'eng-cc/oasis7'}}}}))
            binary = temp / 'bin'; binary.mkdir(); gh = binary / 'gh'; gh.write_text(FAKE); gh.chmod(0o755)
            env = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ['PATH'], GH_FIXTURE=str(state), PYTHONDONTWRITEBYTECODE='1')
            contract = dict(schema='oasis7.loop-contract/v1', contract_id='S', revision=1, owner_loop='system', source_head=base, merged_head=base, approval_ref={'repository': 'eng-cc/oasis7', 'pr_number': 2}, content_refs=[{'path': 'doc/engineering/spec.md', 'sha256': 'sha256:' + hashlib.sha256(spec.read_bytes()).hexdigest(), 'clauses': ['a']}], upstream_contracts=[], scope=['pilot'], eligibility={'new_tasks': True, 'in_flight': True, 'release': True})
            contract['eligibility']['in_flight'] = in_flight
            contract['eligibility']['release'] = in_flight
            source = temp / 'contract.json'; source.write_text(json.dumps(contract))
            command = ['python3', str(trusted / 'scripts/pm/loop.py'), 'publish-contract', '--repo-root', str(root), '--tool-root', str(trusted), '--task-uid', UID, '--manual-request-ref', 'user-2', '--contract', str(source), '--json']
            for failure in ('owner', 'digest'):
                invalid = copy.deepcopy(contract)
                if failure == 'owner': invalid['owner_loop'] = 'product'
                else: invalid['content_refs'][0]['sha256'] = 'sha256:' + '0' * 64
                source.write_text(json.dumps(invalid))
                rejected = subprocess.run(command, env=env, text=True, capture_output=True)
                self.assertNotEqual(rejected.returncode, 0, rejected.stdout)
                self.assertEqual(json.loads(state.read_text())['comments'], [])
                journals = list((root / '.git/oasis7-loop-recovery').glob('*.actions.jsonl'))
                self.assertEqual(journals, [], 'side-effect-free preflight must not leave pending actions')
            invalid = copy.deepcopy(contract)
            invalid['scope'] = ['different delivery']
            source.write_text(json.dumps(invalid))
            rejected = subprocess.run(command, env=env, text=True, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0, rejected.stdout)
            self.assertIn('contract does not cover target delivery', rejected.stdout)
            self.assertEqual(json.loads(state.read_text())['comments'], [])
            self.assertEqual(
                list((root / '.git/oasis7-loop-recovery').glob('*.actions.jsonl')),
                [],
                'invalid output coverage must fail before publication intent is recorded',
            )
            upstream = copy.deepcopy(contract)
            upstream.update(contract_id='UP', owner_loop='product')
            upstream['eligibility']['new_tasks'] = False
            immutable = {k:v for k,v in upstream.items() if k != 'eligibility'}
            digest = 'sha256:' + hashlib.sha256(json.dumps(immutable,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
            current = json.loads(state.read_text())
            current['comments'] = [{'id':1,'body':json.dumps({'marker':'oasis7-loop-contract','task_uid':UID,'contract':upstream,'contract_digest':digest}),'issue_url':'https://api.github.com/repos/eng-cc/oasis7/issues/1','user':{'login':'owner'}}]
            state.write_text(json.dumps(current))
            invalid = copy.deepcopy(contract)
            invalid['upstream_contracts'] = [{'contract_id':'UP','revision':1,'contract_digest':digest,'publication_ref':{'issue_number':1,'comment_id':1},'consumed_clauses':['a']}]
            source.write_text(json.dumps(invalid))
            rejected = subprocess.run(command, env=env, text=True, capture_output=True)
            self.assertNotEqual(rejected.returncode,0,rejected.stdout)
            self.assertIn('new_tasks',rejected.stdout)
            self.assertEqual(list((root / '.git/oasis7-loop-recovery').glob('*.actions.jsonl')),[])
            current['comments'] = []; state.write_text(json.dumps(current))
            source.write_text(json.dumps(contract))
            if not lose_response:
                for _ in range(2):
                    result = subprocess.run(command, env=env, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(len(json.loads(state.read_text())['comments']), 1)
                journal = next((root / '.git/oasis7-loop-recovery').glob('*.actions.jsonl'))
                events = [json.loads(line) for line in journal.read_text().splitlines()]
                self.assertTrue(events[-1]['reconciled'])
                return
            lost = subprocess.run(command, env=dict(env,GH_LOSE_POST='1'),text=True,capture_output=True)
            self.assertNotEqual(lost.returncode,0,lost.stdout)
            self.assertEqual(len(json.loads(state.read_text())['comments']),1)
            blocked = subprocess.run(command,env=env,text=True,capture_output=True)
            self.assertNotEqual(blocked.returncode,0,blocked.stdout)
            self.assertIn('unresolved action',blocked.stdout)
            recovery = command.copy(); recovery[2] = 'recover'
            helper = trusted / 'scripts/pm/loop_contracts.py'
            original = helper.read_bytes()
            sentinel = temp / 'untrusted-validator-ran'
            self.assertNotEqual(git('rev-parse', 'HEAD'), base,
                                'task checkout must not be an alternate clean exact-pin helper')
            self.assertEqual(subprocess.check_output(
                ['git', '-C', str(trusted), 'symbolic-ref', '--short', 'HEAD'], text=True,
            ).strip(), 'main', 'the only exact-pin helper must be the trusted main checkout')
            self.assertEqual(subprocess.check_output(
                ['git', '-C', str(trusted), 'rev-parse', 'HEAD'], text=True,
            ).strip(), base, 'trusted main checkout must be at the immutable policy pin')
            worktrees = subprocess.check_output(
                ['git', '-C', str(root), 'worktree', 'list', '--porcelain'], text=True,
            )
            self.assertEqual(sum(line == 'HEAD ' + base for line in worktrees.splitlines()), 1,
                             'fixture must contain exactly one checkout at the immutable pin')
            helper.write_text('from pathlib import Path\nPath(' + repr(str(sentinel)) + ').touch()\n')
            dirty_helpers = subprocess.check_output(
                ['git', '-C', str(trusted), 'status', '--porcelain', '--', 'scripts/pm'], text=True,
            )
            self.assertTrue(dirty_helpers, 'the sole exact-pin helper must be ineligible while tampered')
            journal = next((root / '.git/oasis7-loop-recovery').glob('*.actions.jsonl'))
            before = journal.read_bytes()
            denied = subprocess.run(recovery,env=env,text=True,capture_output=True)
            self.assertNotEqual(denied.returncode,0,denied.stdout)
            self.assertFalse(sentinel.exists())
            self.assertEqual(journal.read_bytes(),before)
            helper.write_bytes(original)
            recovered = subprocess.run(recovery,env=env,text=True,capture_output=True)
            self.assertEqual(json.loads(recovered.stdout).get('pending_actions'),[],recovered.stdout+recovered.stderr)
            for _ in range(2):
                result = subprocess.run(command, env=env, text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(len(json.loads(state.read_text())['comments']), 1)
            journals = list((root / '.git/oasis7-loop-recovery').glob('*.actions.jsonl'))
            events = [json.loads(line) for line in journals[0].read_text().splitlines()]
            self.assertEqual(events[0]['kind'], 'publish_contract')
            self.assertTrue(events[-1]['reconciled'])


if __name__ == '__main__': unittest.main()
