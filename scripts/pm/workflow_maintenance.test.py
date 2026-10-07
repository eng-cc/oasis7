"""Human maintenance authorization and immutable code-under-test boundaries."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import os
import unittest
from unittest import mock

import workflow_maintenance as maintenance

UID = 'task_' + 'a' * 32
HEAD = 'b' * 40

class MaintenanceTests(unittest.TestCase):
    def fixture(self):
        scope = dict(repository='fixture/repo', task_uid=UID, issue_number=1, pr_number=2,
                     purpose='candidate-tool-verification',
                     allowed_write_paths=['scripts/pm/loop-ci.py'],
                     allowed_tool_paths=list(maintenance.TOOL_PATHS))
        comment = dict(id=7, body=maintenance.MARKER + '\n```json\n' + json.dumps(scope) + '\n```',
                       issue_url='https://api.github.com/repos/fixture/repo/issues/1',
                       html_url='https://github.com/fixture/repo/issues/1#issuecomment-7',
                       user=dict(login='human', type='User'), created_at='2026-10-07T00:00:00Z',
                       updated_at='2026-10-07T00:00:00Z')
        permission = dict(permission='admin', user=dict(login='human'))
        task = dict(number=1, state='open', html_url='https://github.com/fixture/repo/issues/1',
                    body=f'task_uid: {UID}\n- merge_hold_active: `false`\n- pr_number: `2`\n- pr_url: `https://github.com/fixture/repo/pull/2`\n')
        pr = dict(number=2, state='open', merged_at=None, draft=True,
                  html_url='https://github.com/fixture/repo/pull/2',
                  body=f'Task: {UID}\nRefs #1\nWorkflow Maintenance Authority: 7\n',
                  head=dict(sha=HEAD, ref='task/repair', repo=dict(full_name='fixture/repo')),
                  base=dict(ref='main', repo=dict(full_name='fixture/repo')))
        return scope, comment, permission, task, pr

    def validate(self, comment, permission, task, pr, head=HEAD):
        return maintenance.validate_maintenance_authority(comment, permission, task, pr, head,
            ('scripts/pm/loop-ci.py',), maintenance.TOOL_PATHS)

    def test_admin_scope_selects_exact_event_head_with_repeated_matching_uid(self):
        _, comment, permission, task, pr = self.fixture()
        pr['body'] += '\nEvidence: ' + UID
        result = self.validate(comment, permission, task, pr)
        self.assertEqual(HEAD, result['tool_revision'])
        self.assertEqual(7, result['comment_id'])
        self.assertEqual(7, maintenance.maintenance_comment_id(pr['body']))
        self.assertEqual('main', pr['base']['ref'])

    def test_live_authority_drift_and_holds_fail_closed(self):
        scope, comment, permission, task, pr = self.fixture()
        mutations = (
            lambda c,p,t,r: c.update(updated_at='edited'),
            lambda c,p,t,r: c['user'].update(type='Bot'),
            lambda c,p,t,r: c.update(issue_url='https://api.github.com/repos/fixture/repo/issues/9'),
            lambda c,p,t,r: p.update(permission='write'),
            lambda c,p,t,r: p['user'].update(login='other'),
            lambda c,p,t,r: t.update(body=f'task_uid: {UID}\n- merge_hold_active: `true`'),
            lambda c,p,t,r: t.update(state='closed'),
            lambda c,p,t,r: r['head'].update(sha='c' * 40),
            lambda c,p,t,r: r.update(draft=False),
            lambda c,p,t,r: t.update(body=t['body'].replace('pr_number: `2`','pr_number: `9`')),
            lambda c,p,t,r: r['head']['repo'].update(full_name='other/repo'),
            lambda c,p,t,r: r.update(body=r['body'] + '\nTask: malformed'),
            lambda c,p,t,r: r.update(body=r['body'] + '\nTask: ' + UID),
            lambda c,p,t,r: r.update(body=r['body'] + '\nEvidence: task_' + 'c' * 32),
            lambda c,p,t,r: r.update(body=r['body'].replace('Refs #1', 'Refs #9')),
        )
        for mutate in mutations:
            values = copy.deepcopy((comment, permission, task, pr))
            mutate(*values)
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.validate(*values)
        for key in ('allowed_write_paths', 'allowed_tool_paths'):
            changed = copy.deepcopy(scope); changed[key] = ['doc/unrelated.md']
            changed_comment = {**comment, 'body': maintenance.MARKER + json.dumps(changed)}
            with self.subTest(scope=key), self.assertRaises(ValueError):
                self.validate(changed_comment, permission, task, pr)

    def test_locator_and_scope_payload_cannot_supply_permission(self):
        for body in ('Workflow Maintenance Authority: 0',
                     'Workflow Maintenance Authority: 7\nWorkflow Maintenance Authority: 7'):
            with self.subTest(body=body), self.assertRaises(ValueError):
                maintenance.maintenance_comment_id(body)
        scope, comment, permission, task, pr = self.fixture()
        for payload in ({**scope, 'authority_oid': HEAD},
                        {**scope, 'allowed_write_paths': ['scripts/pm/*']},
                        {**scope, 'allowed_tool_paths': ['../loop-ci.py']}):
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                maintenance.parse_maintenance_authority(maintenance.MARKER + json.dumps(payload))
        duplicate = json.dumps(scope)[:-1] + ', "task_uid": "' + UID + '"}'
        with self.assertRaises(ValueError):
            maintenance.parse_maintenance_authority(maintenance.MARKER + duplicate)

    def test_draft_default_and_exact_absent_pair_window(self):
        _, comment, permission, task, pr = self.fixture()
        for draft in (False, None, 'true'):
            with self.subTest(draft=draft), self.assertRaises(ValueError):
                maintenance.validate_maintenance_authority(comment, permission, task,
                    dict(pr, draft=draft), HEAD)
        with self.assertRaises(ValueError):
            maintenance.validate_maintenance_authority(comment, permission, task, pr, HEAD,
                require_draft=False)
        absent = dict(task, body=f'task_uid: {UID}\n- merge_hold_active: `false`\n')
        with self.assertRaises(ValueError):
            maintenance.validate_maintenance_authority(comment, permission, absent, pr, HEAD)
        maintenance.validate_maintenance_authority(comment, permission, absent, pr, HEAD,
            binding_phase='validation-start-only')
        for suffix in ('- pr_number: `2`\n', '- pr_url: `https://github.com/fixture/repo/pull/2`\n',
                       '- pr_number: malformed\n'):
            with self.subTest(suffix=suffix), self.assertRaises(ValueError):
                maintenance.validate_maintenance_authority(comment, permission,
                    dict(absent, body=absent['body'] + suffix), pr, HEAD,
                    binding_phase='validation-start-only')

    def test_scope_read_is_not_candidate_selection_or_ready_authority(self):
        _, comment, permission, task, pr = self.fixture()
        pr['draft'] = False
        with mock.patch.object(maintenance, '_gh', side_effect=[comment, permission, task, pr,
                {'default_branch': 'main'}]):
            scope = maintenance.read_maintenance_scope('fixture/repo',7,UID,2,HEAD)
        self.assertIs(False, scope['selection_authorized'])
        self.assertNotIn('tool_revision', scope)
        for forged in (scope, {'ready_for_merge': True}, True):
            with self.subTest(forged=forged), self.assertRaises(ValueError):
                maintenance.validate_maintenance_authority(comment,permission,task,pr,HEAD,
                    ready_continuation=forged)
        with self.assertRaises(ValueError):
            maintenance.ReadyMaintenanceContinuation(None, scope, HEAD, {})

    def test_ready_factory_requires_canonical_review_locator(self):
        _, comment, permission, task, pr = self.fixture()
        pr['draft'] = False
        with mock.patch.object(maintenance, 'read_maintenance_scope', return_value={
                'allowed_tool_paths': ['scripts/pm/ci-ready-receipt.py',
                    'scripts/pm/ci_ready_receipt_identity.py','scripts/pm/workflow_maintenance.py',
                    'scripts/pm/task_primary_package.py']}), \
                mock.patch.object(maintenance, '_gh', side_effect=[{'default_branch':'main'},{'sha':HEAD}]):
            with self.assertRaisesRegex(ValueError, 'review plan locator'):
                maintenance.read_ready_maintenance_continuation('.', 'fixture/repo',7,UID,2,HEAD,app=1)

    def test_ready_factory_rejects_unapproved_primary_dependency(self):
        with mock.patch.object(maintenance, 'read_maintenance_scope', return_value={
                'allowed_tool_paths': ['scripts/pm/ci-ready-receipt.py',
                    'scripts/pm/ci_ready_receipt_identity.py', 'scripts/pm/workflow_maintenance.py']}), \
                mock.patch.object(maintenance, '_gh') as reader:
            with self.assertRaisesRegex(ValueError, 'closure is not approved'):
                maintenance.read_ready_maintenance_continuation('.', 'fixture/repo',7,UID,2,HEAD,app=1)
            reader.assert_not_called()

    def test_ready_selector_requires_every_independent_head_and_target_binding(self):
        scope,comment,permission,task,pr=self.fixture()
        pr['draft']=False
        q='d'*40
        scope.update(comment_id=7,subject_head_oid=HEAD,selection_authorized=False)
        provenance={'source_head_oid':HEAD,'protected_target_oid':q,
                    'current_target_proof':{'source_head_oid':HEAD,'current_target_oid':q}}
        continuation=maintenance.ReadyMaintenanceContinuation(maintenance._CONTINUATION_TOKEN,scope,q,provenance)
        with mock.patch.object(maintenance,'_gh',return_value={'sha':q}):
            result=maintenance.validate_maintenance_authority(comment,permission,task,pr,HEAD,
                ready_continuation=continuation)
        self.assertEqual(HEAD,result['tool_revision'])
        for field in ('scope_head','provenance_head','proof_head','proof_target','protected_target'):
            bad_scope=copy.deepcopy(scope); bad=copy.deepcopy(provenance)
            if field=='scope_head': bad_scope['subject_head_oid']='e'*40
            elif field=='provenance_head': bad['source_head_oid']='e'*40
            elif field=='proof_head': bad['current_target_proof']['source_head_oid']='e'*40
            elif field=='proof_target': bad['current_target_proof']['current_target_oid']='e'*40
            else: bad['protected_target_oid']='e'*40
            mixed=maintenance.ReadyMaintenanceContinuation(maintenance._CONTINUATION_TOKEN,bad_scope,q,bad)
            with self.subTest(field=field), self.assertRaises(ValueError):
                maintenance.validate_maintenance_authority(comment,permission,task,pr,HEAD,
                    ready_continuation=mixed)
        with mock.patch.object(maintenance,'_gh',return_value={'sha':'e'*40}), self.assertRaises(ValueError):
            maintenance.validate_maintenance_authority(comment,permission,task,pr,HEAD,
                ready_continuation=continuation)

    def test_immutable_tool_bytes_and_import_shadow_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q'); git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            for relative in maintenance.TOOL_PATHS:
                path = root / relative; path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('# immutable fixture\n')
            git('add', '.'); git('commit', '-qm', 'immutable helper closure')
            authority = dict(tool_revision=git('rev-parse', 'HEAD'),
                             allowed_tool_paths=list(maintenance.TOOL_PATHS))
            maintenance.validate_candidate_tool_root(root, root, authority)
            omitted = dict(authority, allowed_tool_paths=[p for p in maintenance.TOOL_PATHS
                if p != 'scripts/pm/task_primary_package.py'])
            with self.assertRaises(ValueError):
                maintenance.validate_candidate_tool_root(root, root, omitted)
            primary = root / 'scripts/pm/task_primary_package.py'
            primary.write_text('# candidate primary drift\n')
            with self.assertRaises(ValueError):
                maintenance.validate_candidate_tool_root(root, root, authority)
            primary.write_text('# immutable fixture\n')
            helper = root / 'scripts/pm/loop-ci.py'
            helper.write_text('# drift\n')
            with self.assertRaises(ValueError):
                maintenance.validate_candidate_tool_root(root, root, authority)
            helper.write_text('# immutable fixture\n')
            (root / 'scripts/pm/json.py').write_text('raise RuntimeError("shadow")\n')
            with self.assertRaises(ValueError):
                maintenance.validate_candidate_tool_root(root, root, authority)

class ReadyFactoryProcessTests(unittest.TestCase):
    def test_immutable_q_reader_and_approved_h_overlay_with_server_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
            git('init','-q'); git('config','user.name','Fixture'); git('config','user.email','fixture@example.test')
            def write(relative, body):
                path=root/relative; path.parent.mkdir(parents=True,exist_ok=True); path.write_text(body)
            write('scripts/pm/pr-lifecycle-gate.py',
                'import json,os\n'
                'def load_live(*a,**k): return json.loads(os.environ["FACTORY_DATA"])\n'
                'def rebuild_issue_evidence(*a,**k): return {}\n'
                'def decision(*a,**k): return {"ready_for_merge":True}\n')
            write('scripts/pm/github-project-sync.py',
                'import json,os\ndef read_live_issue_project_item(*a,**k): return json.loads(os.environ["FACTORY_PROJECT"])\n')
            write('scripts/pm/review_preflight_handoff.py',
                'import json,os\nfrom pathlib import Path\n'
                'def validate_plan_inputs(root,path,**k):\n'
                ' p=json.loads(path.read_text()); return p,{},path,path,path,[{"role":"qa_engineer"}],b"",b"",b""\n')
            write('scripts/pm/validate-review-provenance.py','import sys\nassert "--source-head" in sys.argv\n')
            write('scripts/pm/projection_publication_contract.py',
                'import json,os\ndef decode_marker(body): return json.loads(os.environ["FACTORY_C1"])\n')
            write('scripts/pm/pr_projection_publication.py',
                '_TASK_PUBLICATION_FIELDS=("task_uid","source_head_oid")\n'
                'def resolve_task_publication(*a,**k): return {"status":"passed","publication":{"publication_id":"P"},"comment":{"comment_id":8}}\n'
                'def parse_publication_binding_comment(body): return {"publication_id":"P","pr_number":2,"pr_url":"https://github.com/fixture/repo/pull/2"}\n'
                'def validate_publication_binding(value,*a): return value\n')
            write('scripts/pm/integration_ci.py','VALUE="protected"\n')
            write('.agents/roles/qa_engineer.md','fixture\n'); write('.codex/config.toml',''); write('.pm/policy.json','{}')
            git('add','.'); git('commit','-qm','protected'); q=git('rev-parse','HEAD')
            write('scripts/pm/pr-lifecycle-gate.py','raise RuntimeError("unapproved candidate gate")\n')
            write('scripts/pm/integration_ci.py','raise RuntimeError("unapproved candidate dependency")\n')
            write('scripts/pm/ci-ready-receipt.py',
                'import json,os,integration_ci,ci_ready_receipt_identity\n'
                'def read_current_target_proof_without_selection(*a,**k):\n'
                ' assert integration_ci.VALUE=="protected"\n'
                ' return json.loads(os.environ["FACTORY_PROOF"])\n')
            write('scripts/pm/ci_ready_receipt_identity.py','import task_primary_package\nassert task_primary_package.VALUE=="candidate primary"\n')
            write('scripts/pm/task_primary_package.py','VALUE="candidate primary"\n')
            write('scripts/pm/workflow_maintenance.py','')
            git('add','.'); git('commit','-qm','candidate'); h=git('rev-parse','HEAD'); tree=git('rev-parse','HEAD^{tree}')
            write('.pm/github-project-sync/tasks.json',json.dumps({'project':{'id':'P','number':1},
                'tasks':{UID:{'repository':'fixture/repo','issue_number':1,'project_item_id':'I'}}}))
            plan=root/'.pm/plan.json'; plan.write_text(json.dumps({'task_uid':UID,'frozen_head':h}))
            scope={'repository':'fixture/repo','task_uid':UID,'issue_number':1,'pr_number':2,
                'purpose':'candidate-tool-verification','allowed_write_paths':['scripts/pm/loop-ci.py'],
                'allowed_tool_paths':['scripts/pm/ci-ready-receipt.py','scripts/pm/ci_ready_receipt_identity.py','scripts/pm/workflow_maintenance.py',
                    'scripts/pm/task_primary_package.py'],
                'comment_id':7,'subject_head_oid':h,'selection_authorized':False}
            proof={'source_head_oid':h,'current_target_oid':q,'maintenance_authority_comment_id':7,
                   'workflow_run_id':9,'checkout_oid':h}
            data={'headRefOid':h,'isDraft':False,'state':'OPEN',
                  'policy_discovery':{'required_status_checks':[{'context':'required-gate','app_id':1}]}}
            project={'complete':True,'item':{'id':'I','field_values':{'Task UID':UID,
                'PR':'https://github.com/fixture/repo/pull/2','PM Status':'pr_watch','Workflow Phase':'pr_watch'}}}
            comment={'id':8,'body':'\n'.join(['<!-- oasis7-pm-evidence -->','Task UID: '+UID,
                'Workflow Phase: pre_pr_ready','Task Status: ready','Immutable Verification Head: '+h,
                'Immutable Verification Tree: '+tree]),'user':{'type':'User','login':'human'},
                'created_at':'2026-10-07T01:00:00Z','updated_at':'2026-10-07T01:00:00Z',
                'issue_url':'https://api.github.com/repos/fixture/repo/issues/1',
                'html_url':'https://github.com/fixture/repo/issues/1#issuecomment-8'}
            event={'id':10,'event':'ready_for_review','created_at':'2026-10-07T02:00:00Z',
                   'actor':{'type':'User','login':'human'}}
            envelope=dict(comment,id=11,body='<!-- oasis7-ci-publication-binding/v1 -->',
                html_url='https://github.com/fixture/repo/issues/1#issuecomment-11')
            task={'number':1,'user':{'type':'User','login':'human'},
                'body':'- status: `pr_watch`\n- workflow_phase: `pr_watch`\n- pr_number: `2`\n- pr_url: `https://github.com/fixture/repo/pull/2`\n'}
            pull={'number':2,'user':task['user'],'body':'fixture','html_url':'https://github.com/fixture/repo/pull/2',
                'state':'open','draft':False,'head':{'sha':h,'ref':'task/repair'},'base':{'ref':'main'},
                'created_at':'2026-10-07T00:00:00Z','updated_at':'2026-10-07T02:00:00Z'}
            def server(endpoint,*args):
                if endpoint=='repos/fixture/repo': return {'default_branch':'main'}
                if '/commits/' in endpoint: return {'sha':q}
                if endpoint.endswith('/issues/1/comments'): return [[comment,envelope]]
                if endpoint.endswith('/issues/1'): return task
                if endpoint.endswith('/pulls/2'): return pull
                if '/collaborators/' in endpoint: return {'permission':'admin','user':{'login':'human'}}
                if endpoint.endswith('/timeline'): return [[event]]
                if '/actions/runs/' in endpoint: return {'conclusion':'success','head_sha':h,'updated_at':'2026-10-07T00:00:00Z'}
                raise AssertionError(endpoint)
            environment={'FACTORY_DATA':json.dumps(data),'FACTORY_C1':json.dumps({'task_uid':UID,'source_head_oid':h}),
                'FACTORY_PROJECT':json.dumps(project),'FACTORY_PROOF':json.dumps(proof)}
            with mock.patch.dict(os.environ,environment), mock.patch.object(maintenance,'read_maintenance_scope',return_value=scope), \
                    mock.patch.object(maintenance,'_gh',side_effect=server):
                result=maintenance.read_ready_maintenance_continuation(root,'fixture/repo',7,UID,2,h,
                    app=1,review_plan_path=plan)
                self.assertIs(type(result),maintenance.ReadyMaintenanceContinuation)
                self.assertEqual(q,result.provenance['protected_target_oid'])
                self.assertEqual(tree,result.provenance['source_tree_oid'])
                self.assertEqual((8,),result.provenance['closeout_comment_ids'])
                with self.assertRaises(TypeError):
                    result.scope['subject_head_oid']='f'*40
                with self.assertRaises(TypeError):
                    result.provenance['current_target_proof']['source_head_oid']='f'*40
                with self.assertRaises(AttributeError):
                    result.current_target_oid='f'*40
                original_check_output = subprocess.check_output
                def unsafe_primary(command, **kwargs):
                    if 'ls-tree' in command and command[-1] == 'scripts/pm/task_primary_package.py':
                        return '120000 blob ' + '0'*40 + '\tscripts/pm/task_primary_package.py\n'
                    return original_check_output(command, **kwargs)
                with mock.patch.object(subprocess, 'check_output', side_effect=unsafe_primary):
                    with self.assertRaisesRegex(ValueError, 'mode is unsafe'):
                        maintenance.read_ready_maintenance_continuation(root,'fixture/repo',7,UID,2,h,
                            app=1,review_plan_path=plan)
                proof['source_head_oid']='f'*40
                self.assertEqual(h,result.provenance['current_target_proof']['source_head_oid'])
                event['event']='converted_to_draft'
                with self.assertRaisesRegex(ValueError,'draft-to-ready'):
                    maintenance.read_ready_maintenance_continuation(root,'fixture/repo',7,UID,2,h,
                        app=1,review_plan_path=plan)

if __name__ == '__main__': unittest.main()
