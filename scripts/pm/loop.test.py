import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
import hashlib
import shutil
import subprocess
import json
import io
import os
from contextlib import redirect_stdout
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
spec = importlib.util.spec_from_file_location('loop_facade', Path(__file__).with_name('loop.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
import loop_terminal
from terminal_proof import receipt_chain_digest


def receipt_fixture(task_uid, repository, issue_number, pr_number, pr_url):
    def entry(record):
        raw = json.dumps(record, sort_keys=True, separators=(',', ':')).encode()
        return {'bytes': raw, 'digest': hashlib.sha256(raw).hexdigest(), 'record': record}
    merge = entry({'receipt_type': 'oasis7_pr_merge', 'issuer': 'github_live_query', 'evidence_mode': 'production',
                   'repository': repository, 'default_branch': 'main', 'pr_number': pr_number, 'pr_url': pr_url,
                   'state': 'MERGED', 'merged_at': '2026-09-10T00:00:00Z', 'head_oid': 'a' * 40,
                   'base_ref': 'main', 'observed_at': '2026-09-10T00:00:00Z'})
    main_sync = entry({'receipt_type': 'oasis7_main_sync', 'issuer': 'post-merge-main-sync', 'task_uid': task_uid,
                       'repository': repository, 'default_branch': 'main', 'merge_receipt_sha256': merge['digest'],
                       'main_commit': 'c' * 40, 'remote_main_commit': 'c' * 40,
                       'integration_mode': 'ancestry', 'observed_at': '2026-09-10T00:01:00Z'})
    terminal = entry({'receipt_type': 'oasis7_terminal_cleanup', 'issuer': 'post-merge-cleanup', 'task_uid': task_uid,
                      'repository': repository, 'issue_number': issue_number, 'pr_number': pr_number,
                      'worktree': '/fixture/worktree', 'branch': 'task/fixture',
                      'merge_receipt_sha256': merge['digest'], 'main_sync_receipt_sha256': main_sync['digest'],
                      'observed_at': '2026-09-10T00:02:00Z'})
    operations = {effect: {'effect': effect,
                           'operation_id': hashlib.sha256(f'{task_uid}:post_merge_done:{effect}'.encode()).hexdigest(),
                           'committed': True} for effect in ('project_update', 'evidence_comment', 'issue_close')}
    ledger = entry({'schema': 'oasis7_finalizer_ledger_v1', 'task_uid': task_uid, 'operations': operations})
    tombstone = entry({'schema': 'oasis7_terminal_tombstone_v1', 'task_uid': task_uid, 'repository': repository,
                       'issue_number': issue_number, 'pr_number': pr_number, 'workflow_phase': 'post_merge_done',
                       'terminal_receipt_sha256': terminal['digest'], 'canonical_worktree': '/fixture/worktree',
                       'task_branch': 'task/fixture', 'checkout_recreation_forbidden': True})
    return {'merge': merge, 'main_sync': main_sync, 'terminal': terminal, 'ledger': ledger, 'tombstone': tombstone}

class LoopTests(unittest.TestCase):
    def test_publication_uses_new_task_contract_eligibility(self):
        self.assertEqual(module.admission_purpose('publish-contract'), 'new_tasks')
        self.assertEqual(module.admission_purpose('resume-check'), 'in_flight')

    def test_recovery_adapter_binds_old_policy_and_attested_merged_bridge(self):
        binding = {'policy_commit': module.LEGACY_RECOVERY_POLICY, 'policy_digest': 'sha256:' + '1' * 64}
        contracts = SimpleNamespace()
        outputs = {
            ('rev-parse', 'HEAD'): '4' * 40,
            ('status', '--porcelain', '--untracked-files=all', '--',
             'scripts/pm/loop.py', 'scripts/pm/loop_recovery.py'): '',
        }

        def git(root, *args):
            if args in outputs:
                return outputs[args]
            raise AssertionError(args)

        def run(command, **kwargs):
            self.assertEqual(command[:3], ['git', '-C', str(Path(module.__file__).resolve().parents[2])])
            self.assertIn(command[3:], [
                ['merge-base', '--is-ancestor', '3b383190916ac99123a2fc9cbc0d3a8ef0d9c516', '4' * 40],
                ['merge-base', '--is-ancestor', '4' * 40, 'refs/remotes/origin/main'],
            ])
            return subprocess.CompletedProcess(command, 0, '', '')

        with patch.object(module, '_git', side_effect=git), \
             patch.object(module, '_trusted_module', return_value=contracts), \
             patch.object(module.subprocess, 'run', side_effect=run), \
             patch.object(module.subprocess, 'check_output', side_effect=lambda command: (
                 Path(module.__file__).resolve().parents[2] / command[-1].split(':', 1)[1]
             ).read_bytes()):
            adapter = module._recovery_publication_adapter(
                Path('/old'), Path('/target'), binding, require_legacy=True
            )
        self.assertIs(adapter.contracts, contracts)
        self.assertEqual(adapter.policy_commit, module.LEGACY_RECOVERY_POLICY)
        self.assertEqual(adapter.bridge_commit, '4' * 40)
        self.assertRegex(adapter.bridge_digest, r'^sha256:[0-9a-f]{64}$')

    def test_recovery_adapter_rejects_dirty_bridge(self):
        binding = {'policy_commit': module.LEGACY_RECOVERY_POLICY, 'policy_digest': 'sha256:' + '1' * 64}
        with patch.object(module, '_git', side_effect=['4' * 40, ' M scripts/pm/loop.py']):
            with self.assertRaisesRegex(ValueError, 'bridge helper bytes are dirty'):
                module._recovery_publication_adapter(Path('/old'), Path('/target'), binding)

    def test_recovery_adapter_rejects_wrong_legacy_policy(self):
        with self.assertRaisesRegex(ValueError, 'exact legacy policy commit'):
            module._recovery_publication_adapter(
                Path('/old'), Path('/target'),
                {'policy_commit': 'd' * 40, 'policy_digest': 'sha256:' + '1' * 64},
                require_legacy=True,
            )

    def dependency_command(self, command, bodies, *, search=None, terminal_pass=True):
        uid, dependency_uid = 'task_' + 'a' * 32, 'task_' + 'b' * 32
        task = {'task_uid': uid, 'repository': 'fixture/repo', 'loop_binding': {
            'task_uid': uid, 'dependencies': [dependency_uid], 'policy_commit': 'a' * 40,
            'policy_digest': 'sha256:' + '1' * 64,
        }}
        policy = SimpleNamespace(validate_binding=lambda _: {'blockers': []}, validate_tool_root=lambda *a: {'blockers': []}, validate_dependencies=lambda *a: {'blockers': []})
        def terminal_delivery(repository, selected_uid, number, **kwargs):
            url = f'https://github.com/{repository}/issues/{number}'
            pr_number = number
            pr_url = f'https://github.com/{repository}/pull/{pr_number}'
            body = (f'<!-- oasis7-pm-task -->\ntask_uid: {selected_uid}\n'
                    '- workflow_phase: `task_done`\n'
                    f'- pr_number: `{pr_number}`\n- pr_url: `{pr_url}`\n')
            issue = {'number': number, 'html_url': url, 'body': body, 'state': 'closed', 'state_reason': 'completed'}
            item = {'id': 'I', 'project': {'id': 'P', 'number': 1, 'owner': {'login': 'fixture'}}, 'content': {'number': number, 'url': url, 'body': body}, 'fieldValues': {'pageInfo': {'hasNextPage': False}, 'nodes': [{'name': v, 'field': {'name': k}} for k, v in [('Status', 'Done' if terminal_pass else 'In Progress'), ('PM Status', 'done'), ('Workflow Phase', 'done')]]}}
            project = {'id': 'P', 'owner': 'fixture', 'number': 1, 'page_complete': True, 'items': [item]}
            operation = hashlib.sha256(f'{selected_uid}:post_merge_done:evidence_comment'.encode()).hexdigest()
            receipts = receipt_fixture(selected_uid, repository, number, pr_number, pr_url)
            merge_digest, sync_digest, terminal_digest = (receipts['merge']['digest'], receipts['main_sync']['digest'], receipts['terminal']['digest'])
            chain_digest = receipt_chain_digest(selected_uid, repository, number, pr_number, pr_url,
                                                merge_digest, sync_digest, terminal_digest)
            comment = {'html_url': url + '#issuecomment-7',
                       'user': {'login': 'fixture'},
                       'body': (f'<!-- oasis7-pm-evidence -->\nOperation-ID: {operation}\n'
                                f'Task UID: {selected_uid}\nEvidence Phase: post_merge_done\n'
                                'Receipt Chain Version: 1\nReceipt Type: oasis7_terminal_cleanup\n'
                                'Receipt Issuer: post-merge-cleanup\n'
                                f'PR Number: {pr_number}\nPR URL: {pr_url}\n'
                                f'Merge Receipt SHA256: {merge_digest}\nMain Sync Receipt SHA256: {sync_digest}\n'
                f'Terminal Receipt SHA256: {terminal_digest}\nReceipt Chain Digest: {chain_digest}\n')}
            pr = {'number': pr_number, 'html_url': pr_url, 'body': f'Task: {selected_uid}\nRefs #{number}\n',
                  'state': 'closed', 'merged': True, 'merged_at': '2026-09-10T00:00:00Z',
                  'merge_commit_sha': 'b' * 40,
                  'base': {'repo': {'full_name': repository}, 'ref': 'main'},
                  'head': {'repo': {'full_name': repository}, 'sha': 'a' * 40}}
            return loop_terminal.validate_terminal_delivery(repository, selected_uid, number,
                issue_reader=lambda *a: issue, project_reader=lambda *a: project,
                comments_reader=lambda *a: [comment], pr_reader=lambda *a: pr,
                receipt_reader=lambda *a: receipts)
        terminal = SimpleNamespace(validate_terminal_delivery=terminal_delivery)
        contracts = SimpleNamespace(validate_contracts=lambda *a, **kw: {'blockers': []})
        def gh(args, **kwargs):
            if args[:3] == ['gh', 'issue', 'list']:
                return json.dumps(search if search is not None else [{'number': n} for n in bodies])
            number = int(args[-1].rsplit('/', 1)[-1])
            body = bodies[number]
            if isinstance(body, Exception): raise body
            return json.dumps({'number': number, 'html_url': f'https://github.com/fixture/repo/issues/{number}', 'body': body})
        modules = {'loop_policy': policy, 'loop_terminal': terminal, 'loop_contracts': contracts}
        output = io.StringIO()
        with patch.object(sys, 'argv', ['loop.py', command, '--task-uid', uid, '--tool-root', '.', '--manual-request-ref', 'current']), patch.object(module, 'load_task', return_value=task), patch.object(module, '_trusted_module', side_effect=lambda *a: modules[a[-1]]), patch.object(module, 'existing_policy_tool_root', return_value=Path('/trusted')), patch.object(module.subprocess, 'check_output', side_effect=gh), patch.object(module.subprocess, 'run', return_value=SimpleNamespace(returncode=0, stdout='{"status":"can_continue"}')), patch.object(module, 'live_binding', return_value={'task_uid': dependency_uid, 'dependencies': []}), patch.object(module, 'resolve_effective_binding', side_effect=lambda _root, selected, **_kwargs: (selected.get('loop_binding'), None) if _kwargs.get('return_context') else selected.get('loop_binding')), patch.object(module, 'common_dir', return_value=Path('/unused')), patch.object(module, 'Reservation'), patch.object(module, 'recovery_status', return_value={'pending_actions': []}), redirect_stdout(output):
            code = module.main()
        return code, json.loads(output.getvalue())

    def test_dependency_main_filters_incidental_mentions_before_uniqueness(self):
        uid = 'task_' + 'b' * 32
        for command in ('status', 'resume-check'):
            with self.subTest(command=command):
                code, result = self.dependency_command(command, {1: 'task_uid: ' + uid + '\r\n', 2: 'Depends on ' + uid, 3: 'task_uid: task_' + 'c' * 32 + '\nDepends on ' + uid})
                self.assertEqual(code, 0, result)
                self.assertEqual(result['status'], 'passed')

    def test_dependency_main_blocks_uncertain_ambiguous_or_unfinished(self):
        uid = 'task_' + 'b' * 32
        cases = [
            ({1: 'task_uid: ' + uid, 2: 'task_uid: ' + uid}, {}, 'ambiguous'),
            ({1: 'task_uid: ' + uid + '\ntask_uid: malformed'}, {}, 'canonical'),
            ({1: 'task_uid: ' + uid + '\ntask_uid: ' + uid}, {}, 'canonical'),
            ({1: 'task_uid: ' + uid + '\ntask_uid: task_' + 'c' * 32}, {}, 'canonical'),
            ({1: 'task_uid:  ' + uid}, {}, 'canonical'),
            ({1: 'task_uid: ' + uid, 2: OSError('read unavailable')}, {}, 'read unavailable'),
            ({1: 'task_uid: ' + uid}, {'search': [{'number': n} for n in range(1, 101)]}, 'bounded'),
            ({1: 'task_uid: ' + uid}, {'terminal_pass': False}, 'not successfully completed'),
            ({1: 'ordinary mention ' + uid}, {}, 'missing'),
        ]
        for command in ('status', 'resume-check'):
            for bodies, options, expected in cases:
                with self.subTest(command=command, expected=expected):
                    code, result = self.dependency_command(command, bodies, **options)
                    self.assertEqual(code, 2, result)
                    self.assertIn(expected, '; '.join(result['blockers']))

    def test_recovery_rejects_tampered_transition_identity(self):
        uid = 'task_' + 'a' * 32
        old = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1}
        new = dict(old,bootstrap_epoch=2)
        expected = json.dumps(new,sort_keys=True)
        task = {'task_uid':uid,'owner_role':old['owner_role'],'repository':'eng-cc/oasis7','issue_number':1,'loop_binding':old}
        action = {'task_uid':uid,'repository':task['repository'],'issue_number':1,'kind':'bind_loop','expected':expected,
                  'action_id':'bind:'+hashlib.sha256(expected.encode()).hexdigest(),'previous_binding':old,'previous_epoch':1}
        for mutation in ({'action_id':'bind:forged'},{'issue_number':2},{'previous_epoch':0}):
            with patch.object(module,'common_dir',return_value=Path('/absent')), patch.object(module,'recovery_status',return_value={'pending_actions':[{**action,**mutation}]}), patch.object(module,'_trusted_module') as trusted:
                with self.assertRaises(ValueError): module.recovery_task(Path('/absent'),task,Path('/trusted'))
                trusted.assert_not_called()

    def test_recovery_rejects_binding_outside_recorded_transition(self):
        uid = 'task_' + 'a' * 32
        old = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1}
        new = dict(old,bootstrap_epoch=2)
        expected = json.dumps(new,sort_keys=True)
        task = {'task_uid':uid,'owner_role':old['owner_role'],'repository':'eng-cc/oasis7','issue_number':1,'loop_binding':old}
        action = {'task_uid':uid,'repository':task['repository'],'issue_number':1,'kind':'bind_loop','expected':expected,'action_id':'bind:'+hashlib.sha256(expected.encode()).hexdigest(),'previous_binding':old,'previous_epoch':1}
        with patch.object(module,'common_dir',return_value=Path('/absent')), patch.object(module,'recovery_status',return_value={'pending_actions':[action]}), patch.object(module,'live_binding',return_value=dict(new,bootstrap_epoch=3)):
            with self.assertRaisesRegex(ValueError,'outside journal'): module.recovery_task(Path('/absent'),task,Path('/trusted'))

    def test_recovery_rejects_active_policy_pin_drift_for_recorded_bind(self):
        uid = 'task_' + 'a' * 32
        original = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1,
                    'policy_commit':'a'*40,'policy_digest':'sha256:'+'1'*64}
        adopted = dict(original, policy_commit='b'*40, policy_digest='sha256:'+'2'*64)
        expected = json.dumps(original, sort_keys=True)
        snapshot = {'schema':'oasis7.loop-effective-policy-snapshot/v1',
                    'policy_commit':original['policy_commit'], 'policy_digest':original['policy_digest'],
                    'pin_source':'immutable_binding', 'adoption_chain_tip':None, 'bootstrap_epoch':1}
        task = {'task_uid':uid,'owner_role':original['owner_role'],'repository':'eng-cc/oasis7',
                'issue_number':1,'task_branch':'task/fixture','project_item_id':'P',
                'bootstrap_epoch':1,'loop_binding':original}
        action = {'task_uid':uid,'repository':task['repository'],'issue_number':1,'kind':'bind_loop',
                  'expected':expected,'action_id':'bind:'+hashlib.sha256(expected.encode()).hexdigest(),
                  'previous_binding':original,'previous_epoch':1,'canonical_worktree':'/fixture',
                  'task_branch':'task/fixture','project_item_id':'P',
                  'effective_policy_snapshot':snapshot}
        context = {'effective_policy': {'status':'passed','binding':adopted,
                    'policy_commit':adopted['policy_commit'],'policy_digest':adopted['policy_digest'],
                    'pin_source':'task_issue_adoption_chain','adoption_chain_tip':'sha256:'+'3'*64,
                    'bootstrap_epoch':1},
                   'trusted_current_policy': {'default_branch_oid':'c'*40}}
        with patch.object(module,'common_dir',return_value=Path('/unused')), \
             patch.object(module,'recovery_status',return_value={'pending_actions':[action]}), \
             patch.object(module,'live_binding',return_value=original), \
             patch.object(module,'resolve_effective_binding',return_value=(adopted,context)), \
             patch.object(module,'recovery_authority') as authority:
            with self.assertRaisesRegex(ValueError,'active policy pin differs from recorded bind action'):
                module.recovery_task(Path('/fixture'),task,Path('/trusted'))
            authority.assert_not_called()

    def test_legacy_bind_recovery_does_not_silently_adopt_policy_chain(self):
        uid = 'task_' + 'a' * 32
        original = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1,
                    'policy_commit':'a'*40,'policy_digest':'sha256:'+'1'*64}
        adopted = dict(original, policy_commit='b'*40, policy_digest='sha256:'+'2'*64)
        expected = json.dumps(original, sort_keys=True)
        task = {'task_uid':uid,'owner_role':original['owner_role'],'repository':'eng-cc/oasis7',
                'issue_number':1,'task_branch':'task/fixture','project_item_id':'P',
                'bootstrap_epoch':1,'loop_binding':original}
        action = {'task_uid':uid,'repository':task['repository'],'issue_number':1,'kind':'bind_loop',
                  'expected':expected,'action_id':'bind:'+hashlib.sha256(expected.encode()).hexdigest(),
                  'previous_binding':original,'previous_epoch':1,'canonical_worktree':'/fixture',
                  'task_branch':'task/fixture','project_item_id':'P'}
        context = {'effective_policy': {'status':'passed','binding':adopted,
                    'policy_commit':adopted['policy_commit'],'policy_digest':adopted['policy_digest'],
                    'pin_source':'task_issue_adoption_chain','adoption_chain_tip':'sha256:'+'3'*64,
                    'bootstrap_epoch':1},
                   'trusted_current_policy': {'default_branch_oid':'c'*40}}
        with patch.object(module,'common_dir',return_value=Path('/unused')), \
             patch.object(module,'recovery_status',return_value={'pending_actions':[action]}), \
             patch.object(module,'live_binding',return_value=original), \
             patch.object(module,'resolve_effective_binding',return_value=(adopted,context)), \
             patch.object(module,'recovery_authority') as authority:
            with self.assertRaisesRegex(ValueError,'legacy bind action cannot prove adopted policy pin'):
                module.recovery_task(Path('/fixture'),task,Path('/trusted'))
            authority.assert_not_called()

    def test_bind_action_freezes_effective_pin_without_rewriting_immutable_binding(self):
        uid = 'task_' + 'a' * 32
        original = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1,
                    'policy_commit':'a'*40,'policy_digest':'sha256:'+'1'*64,
                    'write_scope':['doc/product/**']}
        adopted = dict(original, policy_commit='b'*40, policy_digest='sha256:'+'2'*64)
        context = {'effective_policy': {'status':'passed','binding':adopted,
                    'policy_commit':adopted['policy_commit'],'policy_digest':adopted['policy_digest'],
                    'pin_source':'task_issue_adoption_chain','adoption_chain_tip':'sha256:'+'3'*64}}
        task = {'task_uid':uid,'owner_role':original['owner_role'],'repository':'eng-cc/oasis7',
                'issue_number':1,'task_branch':'task/fixture','project_item_id':'P',
                'bootstrap_epoch':1,'loop_binding':original}
        action = module._build_bind_action(Path('/fixture'),task,original,adopted,context)
        self.assertEqual(json.loads(action['expected']), original)
        self.assertEqual(task['loop_binding'], original)
        self.assertEqual(action['effective_policy_snapshot'], {
            'schema':'oasis7.loop-effective-policy-snapshot/v1',
            'policy_commit':adopted['policy_commit'], 'policy_digest':adopted['policy_digest'],
            'pin_source':'task_issue_adoption_chain', 'adoption_chain_tip':'sha256:'+'3'*64,
            'bootstrap_epoch':1,
        })
        self.assertEqual(action['action_id'], module._bind_action_id(
            action['expected'], action['effective_policy_snapshot']))
        changed_snapshot = dict(action['effective_policy_snapshot'], adoption_chain_tip='sha256:'+'4'*64)
        self.assertNotEqual(action['action_id'], module._bind_action_id(action['expected'], changed_snapshot))

    def test_recovery_accepts_original_and_frozen_adopted_pins_without_rewriting_binding(self):
        uid = 'task_' + 'a' * 32
        original = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1,
                    'policy_commit':'a'*40,'policy_digest':'sha256:'+'1'*64,
                    'write_scope':['doc/product/**']}
        adopted = dict(original, policy_commit='b'*40, policy_digest='sha256:'+'2'*64)
        task = {'task_uid':uid,'owner_role':original['owner_role'],'repository':'eng-cc/oasis7',
                'issue_number':1,'task_branch':'task/fixture','project_item_id':'P',
                'bootstrap_epoch':1,'loop_binding':original}
        cases = (
            ('immutable', original, {'effective_policy': {'status':'passed','binding':original,
                'policy_commit':original['policy_commit'],'policy_digest':original['policy_digest'],
                'pin_source':'immutable_binding','adoption_chain_tip':None}}),
            ('adopted', adopted, {'effective_policy': {'status':'passed','binding':adopted,
                'policy_commit':adopted['policy_commit'],'policy_digest':adopted['policy_digest'],
                'pin_source':'task_issue_adoption_chain','adoption_chain_tip':'sha256:'+'3'*64}}),
        )
        for name, active, context in cases:
            with self.subTest(pin_source=name):
                action = module._build_bind_action(Path('/fixture'),task,original,active,context)
                with patch.object(module,'common_dir',return_value=Path('/unused')), \
                     patch.object(module,'recovery_status',return_value={'pending_actions':[action]}), \
                     patch.object(module,'live_binding',return_value=original), \
                     patch.object(module,'resolve_effective_binding',return_value=(active,{
                         **context,'trusted_current_policy':{'default_branch_oid':'c'*40}})), \
                     patch.object(module,'recovery_authority') as authority:
                    recovered = module.recovery_task(Path('/fixture'),task,Path('/trusted'))
                self.assertEqual(recovered['loop_binding'], original)
                self.assertEqual(recovered['_effective_loop_binding'], active)
                self.assertEqual(recovered['_trusted_default_oid'], 'c'*40)
                authority.assert_called_once_with(Path('/fixture'),active,Path('/trusted'),
                                                  trusted_default_oid='c'*40)

    def test_recovery_active_pin_drift_stops_before_helper_or_reconcile(self):
        uid = 'task_' + 'a' * 32
        original = {'task_uid':uid,'owner_role':'repository_health_engineer','bootstrap_epoch':1,
                    'policy_commit':'a'*40,'policy_digest':'sha256:'+'1'*64,
                    'write_scope':['doc/product/**']}
        adopted = dict(original, policy_commit='b'*40, policy_digest='sha256:'+'2'*64)
        snapshot = module._bind_policy_snapshot(original)
        expected = json.dumps(original, sort_keys=True)
        action = {'action_id':'bind:'+hashlib.sha256(expected.encode()).hexdigest(),
                  'kind':'bind_loop','expected':expected,'effective_policy_snapshot':snapshot,
                  'task_uid':uid,'repository':'eng-cc/oasis7','issue_number':1,
                  'previous_binding':original,'previous_epoch':1,
                  'canonical_worktree':'/fixture','task_branch':'task/fixture','project_item_id':'P'}
        context = {'effective_policy': {'status':'passed','binding':adopted,
                    'policy_commit':adopted['policy_commit'],'policy_digest':adopted['policy_digest'],
                    'pin_source':'task_issue_adoption_chain','adoption_chain_tip':'sha256:'+'3'*64},
                   'trusted_current_policy': {'default_branch_oid':'c'*40}}
        task = {'task_uid':uid,'owner_role':original['owner_role'],'repository':'eng-cc/oasis7',
                'issue_number':1,'task_branch':'task/fixture','project_item_id':'P',
                'bootstrap_epoch':1,'loop_binding':original}
        output = io.StringIO()
        with patch.object(sys,'argv',['loop.py','recover','--repo-root','/fixture','--tool-root','/trusted',
                                      '--task-uid',uid,'--manual-request-ref','current','--json']), \
             patch.object(module,'load_task',return_value=task), \
             patch.object(module,'common_dir',return_value=Path('/unused')), \
             patch.object(module,'recovery_status',return_value={'pending_actions':[action]}), \
             patch.object(module,'live_binding',return_value=original), \
             patch.object(module,'resolve_effective_binding',return_value=(adopted,context)), \
             patch.object(module.subprocess,'run',return_value=subprocess.CompletedProcess([],0,'','')), \
             patch.object(module.subprocess,'check_output') as check_output, \
             patch.object(module,'recovery_authority') as authority, \
             patch.object(module,'reconcile') as reconcile, \
             patch.object(module,'record_action') as record_action, \
             redirect_stdout(output):
            result = module.main()
        self.assertEqual(result,2)
        payload = json.loads(output.getvalue())
        self.assertEqual(payload['status'],'blocked')
        self.assertIn('active policy pin differs from recorded bind action',payload['blockers'][0])
        check_output.assert_not_called()
        authority.assert_not_called()
        reconcile.assert_not_called()
        record_action.assert_not_called()

    def test_legacy_passes_without_activation(self):
        self.assertEqual(module.validate_task(Path('.'), {'task_uid': 'x'}, None)['status'], 'legacy')

    def test_loop_without_trusted_source_fails_closed(self):
        result = module.validate_task(Path('.'), {'loop_binding': {'loop': 'code'}}, None)
        self.assertEqual(result['status'], 'blocked')

    def test_empty_binding_is_not_legacy(self):
        self.assertEqual(module.validate_task(Path('.'), {'loop_binding': {}}, None)['status'], 'blocked')

    def test_new_task_does_not_consume_in_flight_only_contract(self):
        policy = SimpleNamespace(validate_binding=lambda _: {'blockers': []}, validate_tool_root=lambda *args: {'blockers': []}, validate_dependencies=lambda *args: {'blockers': []})
        contracts = SimpleNamespace(validate_contracts=lambda *args, purpose: {'blockers': ['new_tasks disabled'] if purpose == 'new_tasks' else []})
        task = {'task_uid': 'task_' + 'a' * 32, 'owner_role': 'qa_engineer', 'bootstrap_epoch': 1}
        task['loop_binding'] = dict(task)
        with patch.object(module, 'resolve_effective_binding', side_effect=lambda _root, selected, **_kwargs: selected.get('loop_binding')), patch.object(module, 'existing_policy_tool_root', side_effect=lambda _root, _binding, preferred: Path(preferred)), patch.object(module, '_trusted_module', side_effect=lambda *args: policy if args[-1] == 'loop_policy' else contracts):
            self.assertEqual(module.validate_task(Path('.'), task, Path('.'), purpose='new_tasks')['status'], 'blocked')
            self.assertEqual(module.validate_task(Path('.'), task, Path('.'), purpose='in_flight')['status'], 'passed')

    def test_legacy_pin_keeps_old_validator_fast_path(self):
        pin = {'policy_commit': 'a' * 40, 'policy_digest': 'sha256:' + '1' * 64}
        policy = SimpleNamespace(validate_tool_root=lambda *_args: {'status': 'passed', 'blockers': []})
        with patch.object(module, '_validate_with_current_trusted_policy') as current:
            result, oid = module._validate_pinned_tool_root(policy, Path('/old'), Path('/task'), pin)
        self.assertEqual(result['status'], 'passed')
        self.assertIsNone(oid)
        current.assert_not_called()

    def test_stale_ref_uses_current_trusted_validator_without_changing_semantic_pin(self):
        pin = {'policy_commit': 'a' * 40, 'policy_digest': 'sha256:' + '1' * 64}
        stale = {'status': 'blocked', 'blockers': [
            'pinned policy is not ancestor of refs/remotes/origin/main',
        ]}
        live = {'validation': {'status': 'passed', 'blockers': []},
                'trusted_default_oid': 'b' * 40}
        policy = SimpleNamespace(validate_tool_root=lambda *_args: stale)
        with patch.object(module, '_validate_with_current_trusted_policy', return_value=live) as current:
            result, oid = module._validate_pinned_tool_root(policy, Path('/old'), Path('/task'), pin)
        current.assert_called_once_with(Path('/old'), Path('/task'), pin)
        self.assertEqual(result['status'], 'passed')
        self.assertEqual(oid, 'b' * 40)

    def test_missing_live_dependency_blocks_admission(self):
        policy = SimpleNamespace(validate_binding=lambda _: {'blockers': []}, validate_tool_root=lambda *args: {'blockers': []})
        binding = {'task_uid': 'task_' + 'a' * 32, 'dependencies': ['task_' + 'b' * 32]}
        with patch.object(module, 'resolve_effective_binding', side_effect=lambda _root, selected, **_kwargs: selected.get('loop_binding')), patch.object(module, 'existing_policy_tool_root', side_effect=lambda _root, _binding, preferred: Path(preferred)), patch.object(module, '_trusted_module', return_value=policy), patch.object(module.subprocess, 'check_output', return_value='[]'):
            result = module.validate_task(Path('.'), {'loop_binding': binding, 'repository': 'fixture/repo'}, Path('.'))
        self.assertEqual(result['status'], 'blocked')
        self.assertIn('dependency task missing', result['blockers'][0])

    def test_in_progress_dependency_blocks_admission(self):
        policy = SimpleNamespace(validate_binding=lambda _: {'blockers': []}, validate_tool_root=lambda *args: {'blockers': []})
        terminal = SimpleNamespace(validate_terminal_delivery=lambda *args, **kwargs: {'status': 'blocked', 'blockers': ['Project has not finalized dependency']})
        binding = {'task_uid': 'task_' + 'a' * 32, 'dependencies': ['task_' + 'b' * 32]}
        with patch.object(module, 'resolve_effective_binding', side_effect=lambda _root, selected, **_kwargs: selected.get('loop_binding')), patch.object(module, 'existing_policy_tool_root', side_effect=lambda _root, _binding, preferred: Path(preferred)), patch.object(module, '_trusted_module', side_effect=lambda *args: terminal if args[-1] == 'loop_terminal' else policy), patch.object(module.subprocess, 'check_output', side_effect=['[{"number":1}]', json.dumps({'number': 1, 'html_url': 'https://github.com/fixture/repo/issues/1', 'body': 'task_uid: ' + binding['dependencies'][0]})]):
            result = module.validate_task(Path('.'), {'loop_binding': binding, 'repository': 'fixture/repo'}, Path('.'))
        self.assertEqual(result['status'], 'blocked')
        self.assertIn('not successfully completed', result['blockers'][0])

    def test_completed_dependency_uses_terminal_reader(self):
        policy = SimpleNamespace(validate_binding=lambda _: {'blockers': []}, validate_tool_root=lambda *args: {'blockers': []}, validate_dependencies=lambda *args: {'blockers': []})
        terminal = SimpleNamespace(validate_terminal_delivery=lambda *args, **kwargs: {'status': 'passed', 'blockers': []})
        contracts = SimpleNamespace(validate_contracts=lambda *args, **kwargs: {'blockers': []})
        uid, dependency_uid = 'task_' + 'a' * 32, 'task_' + 'b' * 32
        binding = {'task_uid': uid, 'dependencies': [dependency_uid]}
        modules = {'loop_policy': policy, 'loop_terminal': terminal, 'loop_contracts': contracts}
        with patch.object(module, 'resolve_effective_binding', side_effect=lambda _root, selected, **_kwargs: selected.get('loop_binding')), patch.object(module, 'existing_policy_tool_root', side_effect=lambda _root, _binding, preferred: Path(preferred)), patch.object(module, '_trusted_module', side_effect=lambda *args: modules[args[-1]]), patch.object(module.subprocess, 'check_output', side_effect=['[{"number":1}]', json.dumps({'number': 1, 'html_url': 'https://github.com/fixture/repo/issues/1', 'body': 'task_uid: ' + dependency_uid})]), patch.object(module, 'live_binding', return_value={'task_uid': dependency_uid, 'dependencies': []}):
            result = module.validate_task(Path('.'), {'task_uid': uid, 'loop_binding': binding, 'repository': 'fixture/repo'}, Path('.'))
        self.assertEqual(result['status'], 'passed', result)

    def test_trusted_effective_fixture_and_candidate_tamper(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'repo'
            root.mkdir()
            git = lambda *args: subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q', '-b', 'task/fixture')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            git('remote', 'add', 'origin', 'https://github.com/fixture/repo.git')
            uid = 'task_' + 'a' * 32
            helpers = root / 'scripts/pm'
            helpers.mkdir(parents=True)
            repository = Path(__file__).resolve().parents[2]
            for filename in ('loop.py', 'loop_gate.py', 'loop_recovery.py', 'loop_policy.py', 'loop_contracts.py', 'loop_terminal.py', 'loop-policy.v1.json'):
                shutil.copy2(Path(__file__).with_name(filename), helpers / filename)
            binding_path = Path(tmp) / 'live-binding.json'
            trusted_reader = helpers / 'github-project-task.py'
            trusted_reader.write_text(f'''#!{sys.executable}
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
if args[:1] != ['read-live-policy-context']:
    raise SystemExit('unexpected trusted reader command')
task_uid = args[args.index('--task-uid') + 1]
binding = json.loads(Path(os.environ['FIXTURE_BINDING_PATH']).read_text(encoding='utf-8'))
if binding.get('task_uid') != task_uid:
    raise SystemExit('fixture Task UID mismatch')
policy = {{'status': 'passed', 'blockers': [], 'policy_commit': binding['policy_commit'],
           'policy_digest': binding['policy_digest'], 'pin_source': 'immutable_binding',
           'adoption_chain_tip': None, 'binding': binding}}
value = {{'schema': 'oasis7.workflow-policy-live-context/v1', 'status': 'passed',
         'complete': True, 'task_uid': task_uid, 'repository': 'fixture/repo',
         'live_task_identity': {{'task_uid': task_uid, 'repository': 'fixture/repo'}},
         'binding': binding, 'effective_policy': policy,
         'trusted_current_policy': {{'default_branch': 'main',
                                    'default_branch_oid': binding['policy_commit']}}}}
print(json.dumps(value, sort_keys=True))
''', encoding='utf-8')
            trusted_reader.chmod(0o755)
            shutil.copy2(repository / 'scripts/document_corpus.py', root / 'scripts/document_corpus.py')
            for relative in (
                'doc/.governance/document-corpus-inventory.json',
                'doc/.governance/top-level-directory-registry.json',
                'doc/testing/evidence/inventory.json',
                'doc/engineering/workflow/source-of-truth.md',
            ):
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(repository / relative, target)
            product = root / 'doc/product/a.md'
            product.parent.mkdir(parents=True)
            product.write_text('before')

            product_path = product.relative_to(root).as_posix()
            registry_path = 'doc/.governance/top-level-directory-registry.json'

            def sync_objects(*paths):
                command = [sys.executable, str(repository / 'scripts/document-corpus-inventory.py'),
                           '--repo-root', str(root), 'sync', '--worktree', '--apply']
                for path in paths:
                    command.extend(('--path', path))
                result = subprocess.run(command, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

            sync_objects(registry_path, product_path)
            git('add', '.')
            git('commit', '-qm', 'effective')
            base = git('rev-parse', 'HEAD')
            git('update-ref', 'refs/remotes/origin/main', base)
            trusted = Path(tmp) / 'trusted'
            git('worktree', 'add', '-b', 'main', str(trusted), base)
            product.write_text('after')
            sync_objects(product_path)
            git('add', '.')
            git('commit', '-qm', 'candidate')
            fake_bin = Path(tmp) / 'bin'
            fake_bin.mkdir()
            gh = fake_bin / 'gh'
            gh.write_text(f'''#!{sys.executable}
import json, os, sys
args = sys.argv[1:]
endpoint = args[1] if len(args) > 1 and args[0] == 'api' else ''
if endpoint == 'repos/fixture/repo':
    value = {{'full_name': 'fixture/repo', 'default_branch': 'main'}}
elif endpoint == 'repos/fixture/repo/branches/main':
    value = {{'name': 'main', 'protected': True, 'commit': {{'sha': os.environ['FIXTURE_DEFAULT_OID']}}}}
elif endpoint.startswith('repos/fixture/repo/contents/'):
    path_query = endpoint.split('/contents/', 1)[1]
    relative, separator, query = path_query.partition('?')
    allowed = {{'scripts/pm/loop-policy.v1.json',
               'doc/engineering/workflow/source-of-truth.md'}}
    if not separator or query != 'ref=' + os.environ['FIXTURE_DEFAULT_OID'] or relative not in allowed:
        print('unexpected fake gh contents request: ' + endpoint, file=sys.stderr)
        raise SystemExit(2)
    import base64, subprocess
    raw = subprocess.check_output(['git', '-C', os.environ['FIXTURE_REPO_ROOT'], 'show',
                                   os.environ['FIXTURE_DEFAULT_OID'] + ':' + relative])
    value = {{'encoding': 'base64', 'content': base64.b64encode(raw).decode('ascii')}}
elif endpoint == 'repos/fixture/repo/issues/1':
    uid = os.environ['FIXTURE_TASK_UID']
    value = {{'number': 1, 'html_url': 'https://github.com/fixture/repo/issues/1',
             'body': 'task_uid: ' + uid, 'state': 'open'}}
elif endpoint == 'repos/fixture/repo/issues/1/comments?per_page=100':
    value = [[]]
else:
    print('unexpected fake gh endpoint: ' + endpoint, file=sys.stderr)
    raise SystemExit(2)
print(json.dumps(value))
''', encoding='utf-8')
            gh.chmod(0o755)
            product_key = hashlib.sha256(product_path.encode('utf-8')).hexdigest()
            product_record = f'doc/.governance/document-corpus/objects/{product_key[:2]}/{product_key}.json'
            binding = dict(schema='oasis7.loop-task/v1', task_uid=uid, change_id='c', loop='product', owner_role='gameplay_designer', bootstrap_epoch=1, manual_request_ref='user-1', request_key='r', write_scope=['doc/product/**', product_record], out_of_scope=[], input_contracts=[], acceptance_refs=['a'], dependencies=[], target_delivery='pilot', policy_commit=base, policy_digest='sha256:' + hashlib.sha256((helpers / 'loop-policy.v1.json').read_bytes()).hexdigest())
            task = dict(task_uid=uid, owner_role='gameplay_designer', bootstrap_epoch=1,
                        repository='fixture/repo', issue_number=1, loop_binding=binding)
            fake_env = {'PATH': str(fake_bin) + os.pathsep + os.environ.get('PATH', ''),
                        'FIXTURE_DEFAULT_OID': base, 'FIXTURE_TASK_UID': uid,
                        'FIXTURE_BINDING_PATH': str(binding_path),
                        'FIXTURE_REPO_ROOT': str(root)}
            with patch.dict(os.environ, fake_env), patch.object(module, '_validate_pinned_tool_root', return_value=({'status': 'passed', 'blockers': []}, None)):
                binding['write_scope'] = ['doc/product/**']
                binding_path.write_text(json.dumps(binding, sort_keys=True), encoding='utf-8')
                result = module.validate_task(root, task, trusted, base, git('rev-parse', 'HEAD'))
                self.assertEqual(result['status'], 'blocked', result)
                binding['write_scope'] = ['doc/product/**', product_record]
                binding_path.write_text(json.dumps(binding, sort_keys=True), encoding='utf-8')
                result = module.validate_task(root, task, trusted, base, git('rev-parse', 'HEAD'))
                self.assertEqual(result['status'], 'passed', result)
                (trusted / 'scripts/pm/loop_policy.py').write_text('raise Exception("tampered")')
                binding_path.write_text(json.dumps(binding, sort_keys=True), encoding='utf-8')
                result = module.validate_task(root, task, trusted, base, git('rev-parse', 'HEAD'))
            self.assertEqual(result['status'], 'blocked', result)

    def test_missing_trusted_default_checkout_is_read_only_pending(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'repo'
            root.mkdir()
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q', '-b', 'task/fixture')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            (root / 'README').write_text('fixture\n')
            git('add', 'README')
            git('commit', '-qm', 'fixture')
            before_worktrees = git('worktree', 'list', '--porcelain')
            before_refs = git('for-each-ref', '--format=%(refname) %(objectname)')
            default_oid = 'b' * 40
            with patch.object(module, '_gh_json', side_effect=[
                {'full_name': 'fixture/repo', 'default_branch': 'main'},
                {'name': 'main', 'protected': True, 'commit': {'sha': default_oid}},
            ]):
                with self.assertRaisesRegex(module.PolicyReaderPending, 'no existing clean helper checkout'):
                    module.read_effective_policy_context(root, 'fixture/repo', 'task_' + 'a' * 32)
            self.assertEqual(git('worktree', 'list', '--porcelain'), before_worktrees)
            self.assertEqual(git('for-each-ref', '--format=%(refname) %(objectname)'), before_refs)

    def test_existing_policy_tool_root_batches_exact_helper_reads(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'repo'
            root.mkdir()

            def git(*args):
                return subprocess.check_output(
                    ['git', '-C', str(root), *args], text=True,
                ).strip()

            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            helpers = root / 'scripts' / 'pm'
            helpers.mkdir(parents=True)
            originals = {}
            for index in range(8):
                relative = f'scripts/pm/helper_{index:02}.py'
                content = f'# trusted helper {index}\n'.encode()
                (root / relative).write_bytes(content)
                originals[relative] = content
            git('add', 'scripts/pm')
            git('commit', '-qm', 'small trusted helper tree')

            def trace_read(name):
                trace = Path(tmp) / f'{name}.trace2.jsonl'
                with patch.dict(os.environ, {'GIT_TRACE2_EVENT': str(trace)}):
                    selected = module.existing_policy_tool_root(
                        root, {'policy_commit': git('rev-parse', 'HEAD')}, preferred=root,
                    )
                events = [json.loads(line) for line in trace.read_text().splitlines()]
                starts = [event.get('argv', []) for event in events
                          if event.get('event') == 'start']
                return selected, starts

            selected_small, small_commands = trace_read('small')
            self.assertEqual(selected_small, root.resolve())

            for index in range(8, 48):
                relative = f'scripts/pm/helper_{index:02}.py'
                content = f'# trusted helper {index}\n'.encode()
                (root / relative).write_bytes(content)
                originals[relative] = content
            git('add', 'scripts/pm')
            git('commit', '-qm', 'large trusted helper tree')
            selected_large, large_commands = trace_read('large')

            self.assertEqual(selected_large, root.resolve())
            for commands in (small_commands, large_commands):
                self.assertEqual(
                    sum('show' in argv for argv in commands), 0,
                    f'one git show launch per helper remains across {len(commands)} Git launches',
                )
                self.assertEqual(sum('cat-file' in argv for argv in commands), 1, commands)
            self.assertEqual(len(large_commands), len(small_commands))

            changed = 'scripts/pm/helper_00.py'
            changed_path = root / changed
            git('update-index', '--assume-unchanged', changed)
            try:
                changed_path.write_bytes(b'# modified while status is masked\n')
                with self.assertRaisesRegex(ValueError, 'modified or shadowing helper bytes'):
                    module.existing_policy_tool_root(
                        root, {'policy_commit': git('rev-parse', 'HEAD')}, preferred=root,
                    )
            finally:
                changed_path.write_bytes(originals[changed])
                git('update-index', '--no-assume-unchanged', changed)

            outside = Path(tmp) / 'outside.py'
            outside.write_bytes(originals[changed])
            git('update-index', '--assume-unchanged', changed)
            try:
                changed_path.unlink()
                changed_path.symlink_to(outside)
                with self.assertRaisesRegex(ValueError, 'modified or shadowing helper bytes'):
                    module.existing_policy_tool_root(
                        root, {'policy_commit': git('rev-parse', 'HEAD')}, preferred=root,
                    )
            finally:
                changed_path.unlink(missing_ok=True)
                changed_path.write_bytes(originals[changed])
                git('update-index', '--no-assume-unchanged', changed)

            untracked = helpers / 'shadow.py'
            untracked.write_text('# untracked helper\n')
            try:
                with self.assertRaisesRegex(ValueError, 'modified or shadowing helper bytes'):
                    module.existing_policy_tool_root(
                        root, {'policy_commit': git('rev-parse', 'HEAD')}, preferred=root,
                    )
            finally:
                untracked.unlink()

    def test_legacy_pin_fallback_requires_complete_no_marker_read(self):
        uid = 'task_' + 'a' * 32
        binding = {'task_uid': uid, 'bootstrap_epoch': 1, 'policy_commit': 'a' * 40,
                   'policy_digest': 'sha256:' + '1' * 64, 'write_scope': ['doc/product/**']}
        task = {'task_uid': uid, 'repository': 'fixture/repo', 'issue_number': 17,
                'loop_binding': binding}
        issue = {'number': 17, 'html_url': 'https://github.com/fixture/repo/issues/17',
                 'body': 'task_uid: ' + uid, 'state': 'closed'}
        with patch.object(module, 'read_effective_policy_context', side_effect=module.PolicyReaderPending('reader unavailable')), \
             patch.object(module.subprocess, 'check_output', side_effect=[json.dumps(issue), '[[]]']):
            self.assertEqual(module.resolve_effective_binding(Path('/task'), task), binding)
        comment = {'id': 51, 'body': '<!-- oasis7.workflow-policy-adoption/v1 -->\nmalformed'}
        with patch.object(module, 'read_effective_policy_context', side_effect=module.PolicyReaderPending('reader unavailable')), \
             patch.object(module.subprocess, 'check_output', side_effect=[json.dumps(issue), json.dumps([[comment]])]):
            with self.assertRaisesRegex(module.PolicyReaderPending, 'adoption evidence exists'):
                module.resolve_effective_binding(Path('/task'), task)
        with patch.object(module, 'read_effective_policy_context', side_effect=module.PolicyReaderPending('reader unavailable')), \
             patch.object(module.subprocess, 'check_output', side_effect=[json.dumps(issue), '[]']):
            with self.assertRaisesRegex(module.PolicyReaderPending, 'pagination is unavailable'):
                module.resolve_effective_binding(Path('/task'), task)

    def test_terminal_no_pr_uses_trusted_hosted_resolver_without_local_project(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'trusted-default'
            root.mkdir()
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            helpers = root / 'scripts/pm'
            helpers.mkdir(parents=True)
            uid = 'task_' + 'a' * 32
            binding = {'task_uid': uid, 'bootstrap_epoch': 1, 'policy_commit': 'c' * 40,
                       'policy_digest': 'sha256:' + '2' * 64, 'write_scope': ['doc/product/**']}
            effective = {'status': 'passed', 'policy_commit': binding['policy_commit'],
                         'policy_digest': binding['policy_digest'], 'pin_source': 'immutable_binding',
                         'adoption_chain_tip': None, 'binding': binding}
            payload = {
                'schema': 'oasis7.workflow-policy-live-context/v1', 'status': 'passed', 'complete': True,
                'task_uid': uid, 'repository': 'fixture/repo', 'issue_number': 17,
                'task_issue_state': 'closed', 'live_task_identity': {'task_uid': uid, 'repository': 'fixture/repo'},
                'binding': binding, 'project': None, 'caller': None, 'pr': None,
                'effective_policy': effective,
            }
            cli = helpers / 'github-project-task.py'
            cli.write_text('import json\nprint(' + repr(json.dumps(payload)) + ')\n')
            git('add', '.')
            git('commit', '-qm', 'trusted helper')
            tip = git('rev-parse', 'HEAD')
            task = {'task_uid': uid, 'repository': 'fixture/repo', 'issue_number': 17,
                    'loop_binding': binding}
            with patch.object(module, '_gh_json', side_effect=[
                {'full_name': 'fixture/repo', 'default_branch': 'main'},
                {'name': 'main', 'protected': True, 'commit': {'sha': tip}},
            ]):
                resolved, context = module.resolve_effective_binding(root, task, return_context=True)
            self.assertEqual(resolved, binding)
            self.assertIsNone(context['project'])
            self.assertIsNone(context['pr'])
            self.assertEqual(context['task_issue_state'], 'closed')
            self.assertEqual(context['effective_policy']['pin_source'], 'immutable_binding')

if __name__ == '__main__': unittest.main()
