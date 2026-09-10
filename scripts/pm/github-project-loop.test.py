#!/usr/bin/env python3
"""Legacy-safe loop metadata transport and bootstrap identity regressions."""
import importlib.util
import pathlib
import tempfile
import json
import subprocess
import unittest
from unittest import mock
from argparse import Namespace

ROOT = pathlib.Path(__file__).resolve().parent
def module(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), ROOT / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result

TASK = module('github-project-task')
SYNC = module('github-project-sync')
UID = 'task_' + 'a' * 32
BINDING = dict(schema='oasis7.loop-task/v1', task_uid=UID, change_id='change-1', loop='code',
               owner_role='repository_health_engineer', bootstrap_epoch=1, manual_request_ref='user-message:1',
               request_key='request:1', write_scope=['scripts/pm/**'], out_of_scope=['third_party/**'],
               input_contracts=[], acceptance_refs=['M06'], dependencies=[], target_delivery='test',
               policy_digest='sha256:' + 'b' * 64, policy_commit='c' * 40,
               delivery_obligations=[{'id': 'manual', 'status': 'pending'}])

class LoopTransport(unittest.TestCase):
    def test_dependency_readiness_requires_merged_terminal(self):
        terminal = mock.Mock()
        terminal.validate_terminal_delivery.return_value = {'status':'blocked','blockers':['incomplete terminal Project']}
        with mock.patch.dict('sys.modules', {'loop_terminal':terminal}):
            with self.assertRaises(SystemExit):
                TASK.require_loop_dependency_ready({'issue_number':11,'workflow_phase':'post_merge_done'}, UID)
            terminal.validate_terminal_delivery.return_value = {'status':'passed','blockers':[]}
            TASK.require_loop_dependency_ready({'issue_number':11,'workflow_phase':'task_done'}, UID)
        terminal.validate_terminal_delivery.assert_called_with('eng-cc/oasis7',UID,11)

    def test_new_tasks_eligibility_is_distinct_from_resume(self):
        policy = mock.Mock()
        policy.validate_tool_root.return_value = {'status':'passed'}
        policy.validate_dependencies.return_value = {'status':'passed'}
        contracts = mock.Mock()
        contracts.validate_contracts.side_effect = lambda *a, **kw: ({'status':'blocked','blockers':['new_tasks prohibited']} if kw['purpose']=='new_tasks' else {'status':'passed'})
        with mock.patch.dict('sys.modules', {'loop_policy':policy, 'loop_contracts':contracts}), \
             mock.patch.object(TASK,'run_text',side_effect=lambda args:BINDING['policy_commit'] if args[-2:]==['rev-parse','HEAD'] else ''):
            with self.assertRaises(SystemExit):
                TASK.validate_loop_inputs(pathlib.Path('.'), BINDING, 'eng-cc/oasis7', 'new_tasks')
            TASK.validate_loop_inputs(pathlib.Path('.'), BINDING, 'eng-cc/oasis7', 'in_flight')
        policy.validate_dependencies.assert_called_once()

    def test_archive_and_reload_preserve_binding(self):
        retire = module('github-project-retire-tasks')
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root/'.pm/tasks').mkdir(parents=True)
            (root/'.pm/github-project-sync').mkdir()
            (root/'.pm/tasks'/f'{UID}.yaml').write_text(f'task_uid: {UID}\nstatus: done\n')
            (root/'.pm/tasks'/f'{UID}.execution.md').write_text('fixture evidence')
            mapping = root/'.pm/github-project-sync/tasks.json'
            mapping.write_text(json.dumps({'tasks':{UID:{'issue_url':'https://github.com/eng-cc/oasis7/issues/1','issue_number':1,'project_item_id':'I','loop_binding':BINDING}}}))
            archive = root/'.pm/github-project-sync/task-archive.jsonl'
            self.assertEqual(retire.build_archive(root,mapping,archive)['status'],'ok')
            self.assertEqual(SYNC.load_archived_tasks(root,{'done'})[0]['loop_binding'],BINDING)

    def test_manual_create_retry_and_lost_response_do_not_duplicate(self):
        for lost in (False, True, 'preflight'):
            with self.subTest(lost_response=lost), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                binding = root / 'binding.json'
                binding.write_text(json.dumps(BINDING))
                args = TASK.build_parser().parse_args(['new-task', str(root), '--title', 'fixture',
                    '--owner-role', BINDING['owner_role'], '--source-ref', 'test', '--acceptance', 'M17',
                    '--loop-binding', str(binding), '--request-key', 'request:1', '--bootstrap-base-oid', 'd' * 40, '--json'])
                live = {}
                preflight_failed = False
                def create(*unused, before_write=None):
                    nonlocal preflight_failed
                    if lost == 'preflight' and not preflight_failed:
                        preflight_failed = True
                        raise subprocess.CalledProcessError(1, 'render-before-write')
                    if before_write: before_write()
                    live.update(task_uid=UID, issue_url='https://github.com/eng-cc/oasis7/issues/17', issue_number=17,
                                loop_binding=BINDING, worktree_hint='', status='committed', merge_hold={'active': True})
                    if lost is True: raise subprocess.CalledProcessError(1, 'create-response-lost')
                    return live['issue_url']
                with mock.patch.object(TASK, 'validate_loop_binding', side_effect=lambda b:b), \
                     mock.patch.object(TASK, 'validate_loop_inputs'), \
                     mock.patch.object(TASK, 'record_loop_lineage'), \
                     mock.patch.object(TASK, 'ensure_loop_history'), \
                     mock.patch.object(TASK, 'authoritative_repository_identity', return_value={}), \
                     mock.patch.object(TASK, 'github_issue_record', side_effect=lambda *a:dict(live) if live else None), \
                     mock.patch.object(TASK, 'require_supplied_uid_absent'), \
                     mock.patch.object(TASK, 'create_issue', side_effect=create) as creates, \
                     mock.patch.object(TASK, 'add_project_item', return_value='ITEM'), \
                     mock.patch.object(TASK, 'update_project_fields', return_value=2), \
                     mock.patch.dict('os.environ', {'OASIS7_PM_TEST_SCRATCH': directory}), mock.patch('builtins.print'):
                    if lost:
                        with self.assertRaises(subprocess.CalledProcessError): TASK._command_new_task(args)
                    else:
                        TASK._command_new_task(args)
                    TASK._command_new_task(args)
                    TASK._command_new_task(args)
                    self.assertEqual(creates.call_count, 2 if lost == 'preflight' else 1)

    def test_project_fields_missing_cannot_silently_skip(self):
        sync = mock.Mock(SINGLE_SELECT_FIELDS={'Loop'})
        sync.project_context.return_value = ('P', {})
        sync.project_field_values.return_value = {'Loop': 'code', 'Change ID': 'C'}
        with mock.patch.object(TASK, 'load_sync_module', return_value=sync):
            with self.assertRaises(SystemExit):
                TASK.update_project_fields(Namespace(project_owner='x', project_number=1), {'loop_binding': BINDING}, 'I')
        sync.update_fields.assert_not_called()

    def test_rebind_changes_require_new_epoch(self):
        existing = dict(loop_binding=BINDING, bootstrap_epoch=1, owner_role=BINDING['owner_role'])
        revised = dict(BINDING, write_scope=['src/**'])
        with self.assertRaises(SystemExit):
            TASK.check_loop_binding_update(existing, revised, None)
        revised['bootstrap_epoch'] = 2
        TASK.check_loop_binding_update(existing, revised, 2)

    def test_legacy_binding_requires_explicit_migration(self):
        existing = dict(bootstrap_epoch=1, owner_role=BINDING['owner_role'])
        with self.assertRaises(SystemExit):
            TASK.check_loop_binding_update(existing, BINDING, None)

    def test_identical_binding_is_idempotent(self):
        TASK.check_loop_binding_update(dict(loop_binding=BINDING), BINDING, None)

    def test_roundtrip_preserves_complete_binding(self):
        record = dict(owner_role=BINDING['owner_role'], loop_binding=BINDING)
        parsed = TASK.issue_task_fields(TASK.issue_body(TASK.task_from_record(UID, record)))
        self.assertEqual(parsed.get('loop_binding'), BINDING)

    def test_legacy_has_no_synthesized_binding(self):
        parsed = TASK.issue_task_fields(TASK.issue_body(TASK.task_from_record(UID, {})))
        self.assertNotIn('loop_binding', parsed)
        self.assertNotIn('Loop', SYNC.project_field_values({}))

    def test_project_navigation(self):
        fields = SYNC.project_field_values(dict(loop_binding=BINDING))
        self.assertEqual(fields.get('Loop'), 'code')
        self.assertEqual(fields.get('Change ID'), 'change-1')

    def test_corrupt_binding_fails_closed(self):
        with self.assertRaises(SystemExit):
            TASK.issue_task_fields('- loop_binding_b64: `not-json`\n')

if __name__ == '__main__':
    unittest.main()
