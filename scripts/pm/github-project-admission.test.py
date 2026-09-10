import importlib.util
import json
from pathlib import Path
import tempfile
import os
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('task_admission', Path(__file__).with_name('github-project-task.py'))
task = importlib.util.module_from_spec(spec)
spec.loader.exec_module(task)
UID = 'task_' + 'a' * 32


class AdmissionTests(unittest.TestCase):
    def test_canonical_and_mention_are_not_absence(self):
        def read(args):
            if args[1:3] == ['issue', 'list']:
                return json.dumps([{'number': 1}, {'number': 2}])
            number = int(args[3])
            return json.dumps({'number': number, 'url': f'https://github.com/eng-cc/oasis7/issues/{number}',
                              'body': f'task_uid: {UID}' if number == 1 else f'mentions {UID}'})
        with patch.object(task, 'run_text', side_effect=read):
            self.assertEqual(task.github_issue_record('eng-cc/oasis7', UID)['issue_number'], 1)

    def test_discovery_limit_blocks(self):
        with patch.object(task, 'run_text', return_value=json.dumps([{'number': n} for n in range(5)])):
            with self.assertRaises(BaseException):
                task.github_issue_record('eng-cc/oasis7', UID)

    def test_bind_loop_requires_one_exact_canonical_issue_uid_before_mutation(self):
        for extra in ('', '\ntask_uid: malformed', '\ntask_uid: '+UID):
            with self.subTest(extra=extra), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                binding = {'task_uid':UID,'bootstrap_epoch':1,'owner_role':'repository_health_engineer'}
                path=root/'binding.json'; path.write_text(json.dumps(binding))
                record=dict(issue_number=1,owner_role='repository_health_engineer',worktree_hint=str(root),
                            canonical_worktree=str(root),bootstrap_base_oid='b'*40,loop_binding=binding,project_item_id='ITEM')
                args=task.build_parser().parse_args(['bind-loop',str(root),'--task-uid',UID,
                    '--loop-binding',str(path),'--manual-request-ref','message:1'])
                import base64
                encoded=base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
                body=(f'task_uid: {UID}{extra}\n- owner_role: `repository_health_engineer`\n'
                      f'- worktree_hint: `{root}`\n- loop_binding_b64: `{encoded}`')
                def read(command):
                    if command[1:3]==['issue','list']: return json.dumps([{'number':1}])
                    return json.dumps({'number':1,'body':body})
                with patch.object(task,'require_record',return_value=(root/'mapping',None,record)), \
                     patch.object(task,'run_text',side_effect=read), patch.object(task,'validate_loop_inputs'), \
                     patch.object(task,'update_issue_body') as issue_write, \
                     patch.object(task,'update_project_fields',side_effect=RuntimeError('mutation sentinel')) as write:
                    with self.assertRaises(BaseException) as caught: task.command_bind_loop(args)
                    issue_write.assert_not_called()
                    if extra: write.assert_not_called()
                    else:
                        self.assertEqual(str(caught.exception),'mutation sentinel')
                        write.assert_called_once()

    def test_ambiguous_canonical_and_failed_read_block(self):
        for body in [f'task_uid: {UID}', f'task_uid: {UID}\ntask_uid: {UID}', None]:
            def read(args):
                if args[1:3] == ['issue','list']: return json.dumps([{'number':1},{'number':2}])
                if body is None: raise RuntimeError('unavailable')
                return json.dumps({'body':body})
            with patch.object(task,'run_text',side_effect=read), self.assertRaises(BaseException):
                task.github_issue_record('eng-cc/oasis7',UID)

    def test_missing_snapshot_blocks_before_any_mutation(self):
        self.check_snapshot(None)

    def test_corrupt_snapshot_blocks_before_any_mutation(self):
        self.check_snapshot({'digest':'corrupt'})

    def test_epoch_gap_and_archive_conflict_block_before_mutation(self):
        for epoch, conflict in [(7,False),(1,True)]:
            saved={'task':{'uid':UID,'bootstrap_epoch':epoch},'request':{'identity':'request'},'git':{'base':{'oid':'b'*40}}}
            spec=importlib.util.spec_from_file_location('snapshot',Path(__file__).with_name('bootstrap-task-snapshot.py'))
            snapshot=importlib.util.module_from_spec(spec); spec.loader.exec_module(snapshot)
            saved['digest']=snapshot.digest(saved)
            self.check_snapshot(saved, conflict)

    def check_snapshot(self, saved, conflict=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binding = {'task_uid': UID, 'bootstrap_epoch': 2, 'owner_role': 'repository_health_engineer'}
            path = root / 'binding.json'; path.write_text(json.dumps(binding))
            record = dict(issue_number=1, owner_role='repository_health_engineer', worktree_hint=str(root),
                          canonical_worktree=str(root), bootstrap_base_oid='b'*40, project_item_id='I', issue_url='url')
            args = task.build_parser().parse_args(['bind-loop',str(root),'--task-uid',UID,
                   '--loop-binding',str(path),'--manual-request-ref','message:1','--migrate-epoch','2'])
            if saved is not None:
                snapshot = root/'.pm/scratch'/UID/'bootstrap-task-snapshot.json'
                snapshot.parent.mkdir(parents=True); snapshot.write_text(json.dumps(saved))
                if conflict: snapshot.with_name('bootstrap-task-snapshot.epoch-1.json').write_text('{}')
            with patch.object(task,'require_record',return_value=(root/'mapping',None,record)), \
                 patch.object(task,'github_issue_record',side_effect=[record,{**record,'loop_binding':binding}]), \
                 patch.object(task,'validate_loop_inputs'), patch.object(task,'update_issue_body') as write, \
                 patch.object(task,'update_project_fields') as project, patch.object(task,'ensure_loop_history'), \
                 patch.object(task,'merge_task_mapping') as mapping:
                with self.assertRaises(BaseException): task.command_bind_loop(args)
                write.assert_not_called()
                project.assert_not_called()
                mapping.assert_not_called()

    def test_fresh_creation_with_existing_canonical_and_mention_does_not_post(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binding = {'task_uid':UID,'request_key':'request:1','owner_role':'repository_health_engineer'}
            path=root/'binding.json'; path.write_text(json.dumps(binding))
            args=task.build_parser().parse_args(['new-task',str(root),'--title','fixture','--owner-role','repository_health_engineer',
                '--source-ref','test','--acceptance','test','--loop-binding',str(path),'--request-key','request:1','--bootstrap-base-oid','b'*40])
            def read(args):
                if args[1:3] == ['issue','list']: return json.dumps([{'number':1},{'number':2}])
                return json.dumps({'number':int(args[3]),'body':f'task_uid: {UID}' if args[3]=='1' else f'mentions {UID}'})
            with patch.object(task,'validate_loop_binding',side_effect=lambda b:b), patch.object(task,'validate_loop_inputs'), \
                 patch.object(task,'authoritative_repository_identity',return_value={}), patch.object(task,'run_text',side_effect=read), \
                 patch.dict(os.environ,{'OASIS7_PM_TEST_SCRATCH':directory}), patch.object(task,'create_issue') as create:
                with self.assertRaises(BaseException): task._command_new_task(args)
                create.assert_not_called()

    def check_initial_creation(self, pages, *, should_post=False, random_uid=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binding = {'task_uid':UID,'request_key':'request:1','owner_role':'repository_health_engineer'}
            path = root/'binding.json'; path.write_text(json.dumps(binding))
            command = ['new-task',str(root),'--title','fixture','--owner-role','repository_health_engineer','--source-ref','test','--acceptance','test']
            if not random_uid:
                command += ['--loop-binding',str(path),'--request-key','request:1','--bootstrap-base-oid','b'*40]
            args = task.build_parser().parse_args(command)
            reads = []
            def read(command):
                reads.append(command)
                if command[1:3] == ['issue','list']: return '[]'
                if command[:2] == ['gh','api']:
                    page = int(command[2].rsplit('page=',1)[1])
                    value = pages[min(page-1,len(pages)-1)]
                    if isinstance(value,Exception): raise value
                    return json.dumps(value)
                raise AssertionError('unexpected read: '+repr(command))
            with patch.object(task,'validate_loop_binding',side_effect=lambda b:b), patch.object(task,'validate_loop_inputs'), \
                 patch.object(task,'authoritative_repository_identity',return_value={}), patch.object(task,'run_text',side_effect=read), \
                 patch.dict(os.environ,{'OASIS7_PM_TEST_SCRATCH':directory}), patch.object(task,'create_issue',side_effect=RuntimeError('POST sentinel')) as create:
                with self.assertRaises(BaseException) as caught: task._command_new_task(args)
                if should_post:
                    self.assertEqual(str(caught.exception),'POST sentinel')
                    create.assert_called_once()
                else:
                    create.assert_not_called()
            return reads

    def test_empty_search_with_later_canonical_issue_cannot_post(self):
        first = [{'id':n,'number':n,'body':f'mentions {UID}'} for n in range(1,101)]
        reads = self.check_initial_creation([first,[{'id':101,'number':101,'body':f'task_uid: {UID}'}]])
        self.assertEqual(sum(command[:2]==['gh','api'] for command in reads),2)

    def test_complete_nonsearch_absence_allows_initial_post(self):
        reads = self.check_initial_creation([[{'id':1,'number':1,'body':f'mentions {UID}'},
                {'id':2,'number':2,'body':f'task_uid: {UID}','pull_request':{'url':'pr'}}]],should_post=True)
        self.assertTrue(any(command[:2]==['gh','api'] for command in reads))

    def test_complete_empty_nonsearch_enumeration_allows_initial_post(self):
        reads = self.check_initial_creation([[]],should_post=True)
        self.assertTrue(any(command[:2]==['gh','api'] for command in reads))

    def test_failed_incomplete_or_ambiguous_enumeration_cannot_post(self):
        for pages in ([RuntimeError('unavailable')], [{}], [[{'id':1,'number':1,'body':None},{'id':1,'number':1,'body':None}]],
                      [[{'id':1,'number':1,'body':f'task_uid: {UID}\ntask_uid: {UID}'}]],
                      [[{'number':1,'body':''}]], [[{'id':1,'number':1}]],
                      [[{'id':n,'number':n,'body':''} for n in range(1,101)]]):
            with self.subTest(pages=repr(pages)[:80]): self.check_initial_creation(pages)

    def test_enumeration_budget_exhaustion_cannot_post(self):
        pages = [[{'id':p*100+n,'number':p*100+n,'body':None} for n in range(1,101)] for p in range(100)]
        reads = self.check_initial_creation(pages)
        self.assertEqual(sum(command[:2]==['gh','api'] for command in reads),100)

    def test_random_uid_initial_creation_does_not_enumerate(self):
        reads = self.check_initial_creation([RuntimeError('must not enumerate')],should_post=True,random_uid=True)
        self.assertFalse(any(command[:2]==['gh','api'] for command in reads))


if __name__ == '__main__': unittest.main()
