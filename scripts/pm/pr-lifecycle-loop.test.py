#!/usr/bin/env python3
"""Production receipts require fresh local admission, independently of CI."""
import importlib.util
from pathlib import Path
import unittest
import base64
import json
import tempfile
import subprocess
import os
import shutil
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('pr_gate', Path(__file__).with_name('pr-lifecycle-gate.py'))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ProductionLoopTests(unittest.TestCase):
    def test_final_live_pr_admission_rejects_same_oid_drift(self):
        uid = 'task_' + '1' * 32
        data = {'number': 12, 'repository': 'owner/repo', 'state': 'OPEN', 'isDraft': False,
                'body': 'Task: ' + uid + '\nRefs #1', 'baseRefName': 'main', 'headRefName': 'codex/task',
                'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40,
                'mergeable': 'MERGEABLE', 'mergeStateStatus': 'CLEAN', 'reviewDecision': 'APPROVED',
                'merge_hold': {'kind': 'normal_pr_ci_watch', 'active': False},
                'policy_discovery': {'status': 'resolved', 'required_status_checks': []}}
        cases = [('unchanged', {}, True), ('closed', {'state': 'CLOSED'}, False),
                 ('merged', {'state': 'MERGED'}, False), ('draft', {'isDraft': True}, False),
                 ('task', {'body': 'Task: task_' + '2' * 32 + '\nRefs #1'}, False),
                 ('refs', {'body': 'Task: ' + uid + '\nRefs #2'}, False),
                 ('target', {'baseRefName': 'release'}, False),
                 ('source', {'headRefName': 'codex/replaced'}, False)]
        for label, changes, ready in cases:
            fresh = {**data, **changes}
            def read(command, **kwargs):
                self.assertEqual(command[:3], ['gh', 'pr', 'view'])
                return json.dumps({key: fresh[key] for key in command[command.index('--json') + 1].split(',')})
            with self.subTest(label=label), patch.object(gate, 'local_loop_admission', return_value={'status': 'legacy'}), \
                 patch.object(gate, 'live_integration_admission', return_value=None), \
                 patch.object(gate.subprocess, 'check_output', side_effect=read):
                result = gate.production_decision(data, False, Path('/canonical'), uid, None)
                self.assertEqual(result['ready_for_merge'], ready, result)
                self.assertEqual('readiness_receipt' in result, ready, result)

    def test_advanced_target_with_old_green_run_cannot_mint_receipt(self):
        with patch.object(gate,'live_integration_admission',side_effect=ValueError('stale integration base'),create=True):
            result, _ = self.run_gate()
        self.assertFalse(result['ready_for_merge'])
        self.assertNotIn('readiness_receipt',result)

    def test_fresh_integration_run_keeps_original_source_head(self):
        with patch.object(gate,'live_integration_admission',return_value={'head_oid':'b'*40,'integration_base_oid':'a'*40},create=True) as checked:
            result, _ = self.run_gate()
        self.assertIn('readiness_receipt',result)
        self.assertEqual(result['readiness_receipt']['head_oid'],'b'*40)
        checked.assert_called_once()

    def run_gate(self, admission=None, fresh=None, ready=True):
        data = {'number': 12, 'repository': 'owner/repo', 'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40,
                'state': 'OPEN', 'isDraft': False, 'body': 'Task: task_uid\nRefs #1',
                'baseRefName': 'main', 'headRefName': 'codex/task',
                'policy_discovery': {'status':'resolved','required_status_checks':[]}}
        def decide(data, admin, *, evidence_mode):
            result = {'ready_for_merge': ready, 'status': 'ready' if ready else 'held', 'blockers': [] if ready else ['hold']}
            if ready and evidence_mode == 'production': result['readiness_receipt'] = {'head_oid': data['headRefOid']}
            return result
        with patch.object(gate, 'decision', side_effect=decide), patch.object(gate, 'local_loop_admission', side_effect=admission,return_value={'status':'legacy'}) as check, patch.object(gate, 'read_pr_identity', return_value=fresh or data):
            result = gate.production_decision(data, False, Path('/canonical'), 'task_uid', None)
        return result, check

    def test_revoked_contract_cannot_mint_receipt(self):
        result, _ = self.run_gate(admission=ValueError('contract revoked'))
        self.assertFalse(result['ready_for_merge'])
        self.assertNotIn('readiness_receipt', result)
        self.assertIn('contract revoked', ' '.join(result['blockers']))

    def test_failed_authority_cannot_mint_receipt(self):
        result, _ = self.run_gate(admission=ValueError('effective tool root unavailable'))
        self.assertFalse(result['ready_for_merge'])
        self.assertNotIn('readiness_receipt', result)

    def test_head_or_base_drift_cannot_mint_receipt(self):
        for key in ('headRefOid', 'baseRefOid'):
            fresh = {'number': 12, 'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40, key: 'c' * 40}
            result, _ = self.run_gate(fresh=fresh)
            self.assertFalse(result['ready_for_merge'])
            self.assertNotIn('readiness_receipt', result)

    def test_admission_receives_exact_pr_identity(self):
        result, check = self.run_gate()
        self.assertIn('readiness_receipt', result)
        check.assert_called_once_with(Path('/canonical'), 'task_uid', 'a' * 40, 'b' * 40, None)

    def test_hold_preserved_without_admission(self):
        result, check = self.run_gate(ready=False)
        self.assertEqual(result['status'], 'held')
        check.assert_not_called()
        self.assertNotIn('readiness_receipt', result)


class TrustedIngressTests(unittest.TestCase):
    def test_candidate_policy_commit_cannot_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.pm/github-project-sync').mkdir(parents=True)
            binding = {'policy_commit': 'c' * 40}
            task = {'repository': 'owner/repo', 'issue_number': 3, 'loop_binding': binding}
            (root / '.pm/github-project-sync/tasks.json').write_text(json.dumps({'tasks': {'uid': task}}))
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            issue = json.dumps({'body': f'- loop_binding_b64: `{encoded}`'})
            with patch.object(gate.subprocess, 'check_output', side_effect=[issue, 'c' * 40, '/common', '/common']), patch.object(gate.subprocess, 'run', side_effect=[None, subprocess.CalledProcessError(1, ['git', 'merge-base'])]) as execute:
                with self.assertRaises(subprocess.CalledProcessError):
                    gate.local_loop_admission(root, 'uid', 'a' * 40, 'b' * 40, root)
                self.assertEqual(execute.call_count, 2)
                self.assertTrue(all(call.args[0][0] == 'git' for call in execute.call_args_list))


class IntegrationAuthorityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        directory = self.root / 'scripts/pm'; directory.mkdir(parents=True)
        for name in ('ci-ready-receipt.py','ci_ready_receipt_identity.py','integration_ci.py'):
            shutil.copy2(Path(__file__).with_name(name),directory/name)
        def git(*args): return subprocess.check_output(['git','-C',str(self.root),*args],text=True).strip()
        git('init','-q'); git('config','user.email','fixture@example.invalid'); git('config','user.name','Fixture')
        git('add','.'); git('commit','-qm','trusted')
        self.commit = git('rev-parse','HEAD')
        self.uid = 'task_'+'a'*32
        self.data = {'repository':'owner/repo','number':12,'baseRefName':'main','baseRefOid':'a'*40,'headRefOid':'b'*40,
                     'policy_discovery':{'status':'resolved','required_status_checks':[{'context':'required-gate','app_id':None},{'context':'required-gate','app_id':42}]}}
        self.context = {'status':'passed','tool_root':str(self.root),'policy_commit':self.commit,'task':{'repository':'owner/repo','issue_number':1}}
        spec = importlib.util.spec_from_file_location('fixture_ci',directory/'ci-ready-receipt.py')
        import sys
        sys.path.insert(0,str(directory))
        try:
            module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        finally: sys.path.pop(0)
        self.plan = dict(scope='test',selected_capabilities='',reason_summary='fixture',changed_path_count=0,planner_config_sha256='sha256:'+'c'*64)
        self.plan.update({name:'false' for name in module.RUN_FIELDS})
        self.state = self.root/'github.json'
        binary=self.root/'bin'; binary.mkdir()
        gh=binary/'gh'
        gh.write_text('''#!/usr/bin/env python3
import io,json,os,sys,zipfile
s=json.load(open(os.environ['CI_FIXTURE'])); path=sys.argv[2]
if '/workflows/rust.yml/runs?' in path: result={'workflow_runs':[]}
elif '/pulls/' in path: result=s['pr']
elif '/check-runs?' in path: result={'check_runs':[s['run']]}
elif '/artifacts?' in path: result={'artifacts':[{'id':3,'name':'oasis7-required-plan-v1','expired':False,'workflow_run':{'id':8}}]}
elif path.endswith('/artifacts/3/zip'):
 data=io.BytesIO()
 with zipfile.ZipFile(data,'w') as z: z.writestr('oasis7-required-plan-v1.json',json.dumps(s['artifact']))
 sys.stdout.buffer.write(data.getvalue()); raise SystemExit(0)
else: raise SystemExit('unexpected '+path)
print(json.dumps(result))
'''); gh.chmod(0o755)
        self.env = dict(os.environ,PATH=str(binary)+os.pathsep+os.environ['PATH'],CI_FIXTURE=str(self.state))

    def check(self, run_base='a', artifact_base='a', integration_run_id=None):
        pr={'draft':False,'state':'open','merged':False,'body':f'Task: {self.uid}\nRefs #1','head':{'sha':'b'*40},'base':{'ref':'main','sha':'a'*40}}
        run={'id':9,'name':'required-gate','app':{'id':42},'head_sha':'b'*40,'status':'completed','conclusion':'success','completed_at':'2026-01-01','details_url':'https://github.com/owner/repo/actions/runs/8','pull_requests':[{'number':12,'head':{'sha':'b'*40},'base':{'ref':'main','sha':run_base*40}}]}
        artifact={'schema':'oasis7-required-plan-v1','repository':'owner/repo','workflow_run_id':8,'head_oid':'b'*40,'base_oid':artifact_base*40,'check_name':'required-gate','planner':self.plan}
        self.state.write_text(json.dumps({'pr':pr,'run':run,'artifact':artifact}))
        with patch.dict(os.environ,self.env):
            return gate.live_integration_admission(self.data,self.root,self.uid,self.root,self.context,integration_run_id)

    def test_explicit_locator_is_forwarded_and_never_bypasses_live_discovery(self):
        with self.assertRaisesRegex(ValueError,"locator absent"):
            self.check(integration_run_id=8)

    def test_old_green_integration_run_blocks(self):
        with self.assertRaisesRegex(ValueError,'stale integration base'): self.check(run_base='c',artifact_base='c')

    def test_mutable_check_metadata_cannot_replace_frozen_artifact(self):
        with self.assertRaisesRegex(ValueError,'artifact identity mismatch: base_oid'): self.check(artifact_base='c')

    def test_fresh_run_and_artifact_allow_unchanged_source(self):
        proof=self.check()
        self.assertEqual(proof['head_oid'],'b'*40)
        self.assertEqual(proof['integration_base_oid'],'a'*40)
        self.assertEqual(proof['check_run_id'],9)

    def test_missing_or_ambiguous_app_pin_blocks(self):
        for pins in ([None],[42,43]):
            self.data['policy_discovery']['required_status_checks']=[{'context':'required-gate','app_id':p} for p in pins]
            with self.assertRaisesRegex(ValueError,'unambiguous app pin'): self.check()

    def test_tampered_ci_helper_cannot_execute(self):
        sentinel=self.root/'executed'
        (self.root/'scripts/pm/ci-ready-receipt.py').write_text('from pathlib import Path\nPath('+repr(str(sentinel))+').touch()')
        with self.assertRaisesRegex(ValueError,'helper bytes differ'): self.check()
        self.assertFalse(sentinel.exists())

    def test_tampered_helper_cannot_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.pm/github-project-sync').mkdir(parents=True)
            helper = root / 'scripts/pm/loop-local-gate.py'
            helper.parent.mkdir(parents=True)
            helper.write_text('candidate code')
            binding = {'policy_commit': 'c' * 40}
            task = {'repository': 'owner/repo', 'issue_number': 3, 'loop_binding': binding}
            (root / '.pm/github-project-sync/tasks.json').write_text(json.dumps({'tasks': {'uid': task}}))
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            issue = json.dumps({'body': f'- loop_binding_b64: `{encoded}`'})
            with patch.object(gate.subprocess, 'check_output', side_effect=[issue, 'c' * 40, '/common', '/common', b'trusted code']), patch.object(gate.subprocess, 'run') as execute:
                with self.assertRaisesRegex(ValueError, 'bytes differ'):
                    gate.local_loop_admission(root, 'uid', 'a' * 40, 'b' * 40, root)
                self.assertTrue(all(call.args[0][0] == 'git' for call in execute.call_args_list))

    def test_canonical_head_drift_cannot_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.pm/github-project-sync').mkdir(parents=True)
            helper = root / 'scripts/pm/loop-local-gate.py'
            helper.parent.mkdir(parents=True)
            helper.write_text('trusted code')
            binding = {'policy_commit': 'c' * 40}
            task = {'repository': 'owner/repo', 'issue_number': 3, 'loop_binding': binding}
            (root / '.pm/github-project-sync/tasks.json').write_text(json.dumps({'tasks': {'uid': task}}))
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            issue = json.dumps({'body': f'- loop_binding_b64: `{encoded}`'})
            with patch.object(gate.subprocess, 'check_output', side_effect=[issue, 'c' * 40, '/common', '/common', b'trusted code', 'd' * 40]), patch.object(gate.subprocess, 'run') as execute:
                with self.assertRaisesRegex(ValueError, 'worktree HEAD'):
                    gate.local_loop_admission(root, 'uid', 'a' * 40, 'b' * 40, root)
                self.assertTrue(all(call.args[0][0] == 'git' for call in execute.call_args_list))


if __name__ == '__main__': unittest.main()
