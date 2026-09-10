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
p=pathlib.Path(os.environ['GH_FIXTURE']); s=json.loads(p.read_text()); a=sys.argv[1:]; path=a[1]
if path=='user': result={'login':'owner'}
elif '/collaborators/' in path: result={'permission':'admin'}
elif '/pulls/' in path: result=s['pr']
elif '/issues/comments/' in path: result=s['comments'][int(path.rsplit('/',1)[1])-1]
elif path.split('?')[0].endswith('/comments'):
 if '--method' in a:
  result={'id':len(s['comments'])+1,'body':json.load(sys.stdin)['body'],'issue_url':'https://api.github.com/repos/eng-cc/oasis7/issues/1','user':{'login':'owner'}}
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
            git = lambda *args: subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q', '-b', 'main'); git('config', 'user.name', 'Fixture'); git('config', 'user.email', 'fixture@example.invalid')
            git('add', '.'); git('commit', '-qm', 'effective')
            base = git('rev-parse', 'HEAD')
            origin = temp / 'origin.git'
            subprocess.run(['git', 'init', '--bare', '-q', str(origin)], check=True)
            git('remote', 'add', 'origin', str(origin)); git('push', '-q', 'origin', 'main')
            trusted = temp / 'trusted'; git('worktree', 'add', '--detach', str(trusted), base)
            git('switch', '-c', 'codex/publication')
            binding = dict(schema='oasis7.loop-task/v1', task_uid=UID, change_id='c', loop='system', owner_role='repository_health_engineer', bootstrap_epoch=1, manual_request_ref='user-1', request_key='r', write_scope=['doc/engineering/**'], out_of_scope=[], input_contracts=[], acceptance_refs=['a'], dependencies=[], target_delivery='pilot', policy_commit=base, policy_digest='sha256:' + hashlib.sha256((root / 'scripts/pm/loop-policy.v1.json').read_bytes()).hexdigest())
            mapping = root / '.pm/github-project-sync/tasks.json'; mapping.parent.mkdir(parents=True)
            mapping.write_text(json.dumps({'tasks': {UID: {**binding, 'loop_binding': binding, 'repository': 'eng-cc/oasis7', 'issue_number': 1, 'canonical_worktree': str(root), 'task_branch': 'codex/publication'}}}))
            body = '<!-- oasis7-pm-task -->\ntask_uid: ' + UID + '\n- loop_binding_b64: `' + base64.urlsafe_b64encode(json.dumps(binding).encode()).decode() + '`\n'
            state = temp / 'github.json'
            state.write_text(json.dumps({'comments': [], 'issue': {'number': 1, 'body': body}, 'pr': {'number': 2, 'merged': True, 'head': {'sha': base}, 'merge_commit_sha': base, 'base': {'repo': {'full_name': 'eng-cc/oasis7'}}}}))
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
            helper.write_text('from pathlib import Path\nPath(' + repr(str(sentinel)) + ').touch()\n')
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
