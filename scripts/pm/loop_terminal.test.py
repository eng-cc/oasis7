#!/usr/bin/env python3
"""Terminal delivery follows actual finalizer projection and evidence."""
import copy
import hashlib
import unittest
import json
from pathlib import Path
from unittest.mock import patch
import loop_terminal
import loop_contracts
from loop_terminal import validate_terminal_delivery

UID='task_'+'a'*32
URL='https://github.com/fixture/repo/issues/11'
BODY=f'task_uid: {UID}\n- status: `done`\n- workflow_phase: `task_done`\n'
ISSUE={'number':11,'html_url':URL,'body':BODY,'state':'closed','state_reason':'completed'}
ITEM={'id':'I','project':{'id':'P','number':1,'owner':{'login':'fixture'}},'content':{'number':11,'url':URL,'body':BODY},
      'fieldValues':{'pageInfo':{'hasNextPage':False},'nodes':[{'name':value,'field':{'name':key}} for key,value in
                     [('Status','Done'),('PM Status','done'),('Workflow Phase','done')]]}}
PROJECT={'id':'P','owner':'fixture','number':1,'page_complete':True,'items':[ITEM]}
COMMENT={'html_url':URL+'#issuecomment-7','body':'<!-- oasis7-pm-evidence -->\nOperation-ID: '+hashlib.sha256(f'{UID}:post_merge_done:evidence_comment'.encode()).hexdigest()+f'\nTask UID: {UID}\nEvidence Phase: post_merge_done\nRole: tpm\nCompleted: receipt-bound terminal finalization.\n'}

class TerminalDelivery(unittest.TestCase):
    def check(self,issue=None,project=None,comments=None):
        return validate_terminal_delivery('fixture/repo',UID,11,
            issue_reader=lambda *a:ISSUE if issue is None else issue,
            project_reader=lambda *a:PROJECT if project is None else project,
            comments_reader=lambda *a:[COMMENT] if comments is None else comments)
    def test_real_finalizer_body_task_done_and_project_done_pass(self):
        self.assertEqual(self.check()['status'],'passed')
    def test_pending_or_cancelled_cannot_satisfy_delivery(self):
        for change in ({'state':'open'},{'state_reason':'not_planned'}):
            self.assertEqual(self.check(issue=dict(ISSUE,**change))['status'],'blocked')
        self.assertEqual(self.check(comments=[])['status'],'blocked')
    def test_incomplete_project_projection_blocks(self):
        p=copy.deepcopy(PROJECT); p['items'][0]['fieldValues']['nodes'][0]['name']='In Progress'
        self.assertEqual(self.check(project=p)['status'],'blocked')
    def test_wrong_issue_project_or_truncated_page_blocks(self):
        for mutate in (lambda p:p.update(page_complete=False), lambda p:p['items'][0]['project'].update(id='OTHER'),
                       lambda p:p['items'][0]['content'].update(number=99),lambda p:p['items'][0]['fieldValues']['pageInfo'].update(hasNextPage=True)):
            p=copy.deepcopy(PROJECT); mutate(p)
            self.assertEqual(self.check(project=p)['status'],'blocked')
    def test_duplicate_or_foreign_terminal_comment_blocks(self):
        self.assertEqual(self.check(comments=[COMMENT,COMMENT])['status'],'blocked')
        self.assertEqual(self.check(comments=[dict(COMMENT,html_url='https://github.com/other/repo/issues/11#issuecomment-7')])['status'],'blocked')

    def check_release(self, issue, project):
        def canonical(value):
            return json.loads(json.dumps(value).replace('fixture/repo','eng-cc/oasis7').replace('"fixture"','"eng-cc"'))
        with patch.object(loop_terminal,'read_issue',return_value=canonical(issue)), \
             patch.object(loop_terminal,'read_project',return_value=canonical(project)), \
             patch.object(loop_terminal,'read_comments',return_value=[canonical(COMMENT)]):
            return loop_contracts.validate_contracts(Path('.'),Path('.'),{'input_contracts':[],
                'delivery_obligations':[{'id':'done','task_uid':UID,'issue_number':11}]},purpose='release')

    def test_actual_release_obligation_accepts_unique_terminal_identity(self):
        self.assertEqual(self.check_release(ISSUE,PROJECT)['status'],'passed')

    def test_actual_release_obligation_rejects_ambiguous_issue_and_project_uid(self):
        for location in ('issue','project'):
            for extra in ('task_uid: '+UID,'task_uid: task_'+'b'*32,'task_uid: malformed'):
                with self.subTest(location=location,extra=extra):
                    issue, project = copy.deepcopy(ISSUE), copy.deepcopy(PROJECT)
                    target = issue if location=='issue' else project['items'][0]['content']
                    target['body'] += extra+'\n'
                    self.assertEqual(self.check_release(issue,project)['status'],'blocked')

if __name__=='__main__': unittest.main()
