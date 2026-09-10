#!/usr/bin/env python3
"""Terminal delivery follows actual finalizer projection and evidence."""
import copy
import hashlib
import unittest
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch
import loop_terminal
import loop_contracts
from loop_terminal import validate_terminal_delivery
from terminal_proof import receipt_chain_digest

UID='task_'+'a'*32
URL='https://github.com/fixture/repo/issues/11'
PR_NUMBER=12
PR_URL='https://github.com/fixture/repo/pull/12'
BODY=(f'<!-- oasis7-pm-task -->\n'
      f'task_uid: {UID}\n- status: `done`\n- workflow_phase: `task_done`\n'
      f'- pr_number: `{PR_NUMBER}`\n- pr_url: `{PR_URL}`\n')
ISSUE={'number':11,'html_url':URL,'body':BODY,'state':'closed','state_reason':'completed'}
ITEM={'id':'I','project':{'id':'P','number':1,'owner':{'login':'fixture'}},'content':{'number':11,'url':URL,'body':BODY},
      'fieldValues':{'pageInfo':{'hasNextPage':False},'nodes':[{'name':value,'field':{'name':key}} for key,value in
                     [('Status','Done'),('PM Status','done'),('Workflow Phase','done')]]}}
PROJECT={'id':'P','owner':'fixture','number':1,'page_complete':True,'items':[ITEM]}
PR={'number':PR_NUMBER,'html_url':PR_URL,'body':f'Task: {UID}\nRefs #11\n','state':'closed','merged':True,
    'merged_at':'2026-09-10T00:00:00Z','merge_commit_sha':'b'*40,
    'base':{'repo':{'full_name':'fixture/repo'},'ref':'main'},
    'head':{'repo':{'full_name':'fixture/repo'},'sha':'a'*40}}

def _receipt(record):
    raw=json.dumps(record,sort_keys=True,separators=(',',':')).encode()
    return {'bytes':raw,'digest':hashlib.sha256(raw).hexdigest(),'record':record}

def _receipts(repository):
    pr_url=f'https://github.com/{repository}/pull/{PR_NUMBER}'
    merge=_receipt({'receipt_type':'oasis7_pr_merge','issuer':'github_live_query','evidence_mode':'production',
                    'repository':repository,'default_branch':'main','pr_number':PR_NUMBER,'pr_url':pr_url,
                    'state':'MERGED','merged_at':PR['merged_at'],'head_oid':'a'*40,'base_ref':'main'})
    main_sync=_receipt({'receipt_type':'oasis7_main_sync','issuer':'post-merge-main-sync','task_uid':UID,
                        'repository':repository,'default_branch':'main','merge_receipt_sha256':merge['digest'],
                        'integration_mode':'ancestry','observed_at':'2026-09-10T00:01:00Z'})
    terminal=_receipt({'receipt_type':'oasis7_terminal_cleanup','issuer':'post-merge-cleanup','task_uid':UID,
                       'repository':repository,'issue_number':11,'pr_number':PR_NUMBER,
                       'worktree':'/fixture/worktree','branch':'task/fixture',
                       'merge_receipt_sha256':merge['digest'],'main_sync_receipt_sha256':main_sync['digest'],
                       'observed_at':'2026-09-10T00:02:00Z'})
    operations={effect:{'effect':effect,'operation_id':hashlib.sha256(f'{UID}:post_merge_done:{effect}'.encode()).hexdigest(),
                        'committed':True} for effect in ('project_update','evidence_comment','issue_close')}
    ledger=_receipt({'schema':'oasis7_finalizer_ledger_v1','task_uid':UID,'operations':operations})
    tombstone=_receipt({'schema':'oasis7_terminal_tombstone_v1','task_uid':UID,'repository':repository,
                        'issue_number':11,'pr_number':PR_NUMBER,'workflow_phase':'post_merge_done',
                        'terminal_receipt_sha256':terminal['digest'],'canonical_worktree':'/fixture/worktree',
                        'task_branch':'task/fixture','checkout_recreation_forbidden':True})
    return {'merge':merge,'main_sync':main_sync,'terminal':terminal,'ledger':ledger,'tombstone':tombstone}

RECEIPTS=_receipts('fixture/repo')
COMMENT_BODY=('<!-- oasis7-pm-evidence -->\n'
              'Operation-ID: '+hashlib.sha256(f'{UID}:post_merge_done:evidence_comment'.encode()).hexdigest()+'\n'
              f'Task UID: {UID}\nEvidence Phase: post_merge_done\nReceipt Chain Version: 1\n'
              'Receipt Type: oasis7_terminal_cleanup\nReceipt Issuer: post-merge-cleanup\n'
              f'PR Number: {PR_NUMBER}\nPR URL: {PR_URL}\n'
              f'Merge Receipt SHA256: {RECEIPTS["merge"]["digest"]}\nMain Sync Receipt SHA256: {RECEIPTS["main_sync"]["digest"]}\nTerminal Receipt SHA256: {RECEIPTS["terminal"]["digest"]}\n'
              f'Receipt Chain Digest: {receipt_chain_digest(UID, "fixture/repo", 11, PR_NUMBER, PR_URL, RECEIPTS["merge"]["digest"], RECEIPTS["main_sync"]["digest"], RECEIPTS["terminal"]["digest"])}\n'
              'Role: tpm\nCompleted: receipt-bound terminal finalization.\n')
COMMENT={'html_url':URL+'#issuecomment-7','user':{'login':'fixture'},'body':COMMENT_BODY}

class TerminalDelivery(unittest.TestCase):
    def check(self,issue=None,project=None,comments=None,pr=None,receipts=None):
        return validate_terminal_delivery('fixture/repo',UID,11,
            issue_reader=lambda *a:ISSUE if issue is None else issue,
            project_reader=lambda *a:PROJECT if project is None else project,
            comments_reader=lambda *a:[COMMENT] if comments is None else comments,
            pr_reader=lambda *a:PR if pr is None else pr,
            receipt_reader=lambda *a:RECEIPTS if receipts is None else receipts)
    def test_real_finalizer_body_task_done_and_project_done_pass(self):
        self.assertEqual(self.check()['status'],'passed')
    def test_comment_only_closed_done_non_pr_issue_blocks(self):
        """A computable finalizer comment cannot prove a merged delivery."""
        no_pr_body=(f'<!-- oasis7-pm-task -->\n'
                    f'task_uid: {UID}\n- status: `done`\n- workflow_phase: `task_done`\n')
        issue=copy.deepcopy(ISSUE); issue['body']=no_pr_body
        project=copy.deepcopy(PROJECT); project['items'][0]['content']['body']=no_pr_body
        self.assertEqual(self.check(issue=issue,project=project)['status'],'blocked')
    def test_unmerged_cross_task_or_missing_merge_commit_pr_blocks(self):
        cases=[]
        unmerged=copy.deepcopy(PR); unmerged.update(merged=False,merged_at=None)
        cases.append(unmerged)
        cross_task=copy.deepcopy(PR); cross_task['body']='Task: task_'+'b'*32+'\nRefs #11\n'
        cases.append(cross_task)
        missing_version=copy.deepcopy(PR); missing_version['merge_commit_sha']=None
        cases.append(missing_version)
        for pr in cases:
            with self.subTest(pr=pr):
                self.assertEqual(self.check(pr=pr)['status'],'blocked')
    def test_receipt_head_version_or_cross_receipt_link_blocks(self):
        wrong_head=copy.deepcopy(RECEIPTS)
        wrong_head['merge']['record']['head_oid']='c'*40
        wrong_link=copy.deepcopy(RECEIPTS)
        wrong_link['terminal']['record']['main_sync_receipt_sha256']='f'*64
        for receipts in (wrong_head,wrong_link):
            with self.subTest(receipts=receipts):
                self.assertEqual(self.check(receipts=receipts)['status'],'blocked')
    def test_wrong_author_or_missing_or_mismatched_receipt_blocks(self):
        wrong_author=copy.deepcopy(COMMENT); wrong_author['user']['login']='attacker'
        missing=copy.deepcopy(COMMENT); missing['body']=missing['body'].replace(f'Main Sync Receipt SHA256: {RECEIPTS["main_sync"]["digest"]}\n','')
        mismatched=copy.deepcopy(COMMENT); mismatched['body']=mismatched['body'].replace('Terminal Receipt SHA256: '+RECEIPTS['terminal']['digest'],'Terminal Receipt SHA256: '+'4'*64)
        forged=copy.deepcopy(COMMENT)
        for label in ('Merge Receipt SHA256', 'Main Sync Receipt SHA256', 'Terminal Receipt SHA256'):
            forged['body']=replaced=forged['body'].replace(
                f'{label}: {RECEIPTS[{"Merge Receipt SHA256":"merge","Main Sync Receipt SHA256":"main_sync","Terminal Receipt SHA256":"terminal"}[label]]["digest"]}',
                f'{label}: {"f"*64}')
        forged['body']=forged['body'].replace(
            receipt_chain_digest(UID, 'fixture/repo', 11, PR_NUMBER, PR_URL,
                                 RECEIPTS['merge']['digest'], RECEIPTS['main_sync']['digest'], RECEIPTS['terminal']['digest']),
            receipt_chain_digest(UID, 'fixture/repo', 11, PR_NUMBER, PR_URL, 'f'*64, 'f'*64, 'f'*64),
        )
        for comments in ([wrong_author],[missing],[mismatched],[forged]):
            with self.subTest(comments=comments):
                self.assertEqual(self.check(comments=comments)['status'],'blocked')
    def test_missing_canonical_receipt_root_blocks(self):
        with tempfile.TemporaryDirectory() as temp:
            result=validate_terminal_delivery('fixture/repo',UID,11,
                issue_reader=lambda *a: ISSUE, project_reader=lambda *a: PROJECT,
                comments_reader=lambda *a: [COMMENT], pr_reader=lambda *a: PR,
                repo_root=Path(temp))
        self.assertEqual(result['status'],'blocked')
    def test_canonical_receipt_root_bytes_are_used(self):
        with tempfile.TemporaryDirectory() as temp:
            subprocess.run(['git','init','-q',temp],check=True)
            root=Path(subprocess.check_output([
                sys.executable, 'scripts/pm/canonical-receipt-root.py',
                '--default-worktree', temp, '--task-uid', UID, '--create',
            ],text=True).strip())
            for key, name in (('merge','merge-receipt.json'),('main_sync','main-sync-receipt.json'),
                              ('terminal','terminal-cleanup-receipt.json'),('ledger','finalizer-ledger.json'),
                              ('tombstone','terminal-tombstone.json')):
                (root/name).write_bytes(RECEIPTS[key]['bytes'])
            result=validate_terminal_delivery('fixture/repo',UID,11,
                issue_reader=lambda *a: ISSUE, project_reader=lambda *a: PROJECT,
                comments_reader=lambda *a: [COMMENT], pr_reader=lambda *a: PR,
                repo_root=Path(temp))
        self.assertEqual(result['status'],'passed',result)
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
    def test_unrelated_foreign_discussion_does_not_abort_valid_terminal_scan(self):
        discussion={'html_url':URL+'#issuecomment-6',
                    'user':{'login':'reviewer'},
                    'body':'A normal discussion comment without terminal evidence.'}
        self.assertEqual(self.check(comments=[discussion,COMMENT])['status'],'passed')

    def check_release(self, issue, project):
        def canonical(value):
            return json.loads(json.dumps(value).replace('fixture/repo','eng-cc/oasis7').replace('"fixture"','"eng-cc"'))
        canonical_comment = canonical(COMMENT)
        canonical_receipts = _receipts('eng-cc/oasis7')
        canonical_comment['body'] = canonical_comment['body'].replace(
            receipt_chain_digest(UID, 'fixture/repo', 11, PR_NUMBER, PR_URL,
                                 RECEIPTS['merge']['digest'], RECEIPTS['main_sync']['digest'], RECEIPTS['terminal']['digest']),
            receipt_chain_digest(UID, 'eng-cc/oasis7', 11, PR_NUMBER, PR_URL.replace('fixture/repo', 'eng-cc/oasis7'),
                                 canonical_receipts['merge']['digest'], canonical_receipts['main_sync']['digest'], canonical_receipts['terminal']['digest']),
        )
        for old, new in ((RECEIPTS['merge']['digest'],canonical_receipts['merge']['digest']),
                         (RECEIPTS['main_sync']['digest'],canonical_receipts['main_sync']['digest']),
                         (RECEIPTS['terminal']['digest'],canonical_receipts['terminal']['digest'])):
            canonical_comment['body']=canonical_comment['body'].replace(old,new)
        with patch.object(loop_terminal,'read_issue',return_value=canonical(issue)), \
             patch.object(loop_terminal,'read_project',return_value=canonical(project)), \
             patch.object(loop_terminal,'read_comments',return_value=[canonical_comment]), \
             patch.object(loop_terminal,'read_pull_request',return_value=canonical(PR)), \
             patch.object(loop_terminal,'read_receipt_chain',return_value=canonical_receipts):
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
