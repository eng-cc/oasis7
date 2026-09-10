"""Current manual request selection cannot fall back to historical green."""
import importlib.util
import base64
from pathlib import Path
import unittest
from unittest.mock import patch
import integration_ci as integration
P=Path(__file__).with_name('ci-ready-receipt.py')
spec=importlib.util.spec_from_file_location('receipt_selection',P);receipt=importlib.util.module_from_spec(spec);spec.loader.exec_module(receipt)
UID='task_'+'a'*32;BASE='b'*40;HEAD='c'*40

def run(n,uid=UID,status='completed',conclusion='success'):
 return {'id':n,'run_attempt':1,'created_at':f'2026-09-09T00:{n:02d}:00Z','run_started_at':f'2026-09-09T00:{n:02d}:00Z','event':'workflow_dispatch','head_branch':'main','head_sha':BASE,'path':integration.WORKFLOW,'repository':{'full_name':'owner/repo'},'status':status,'conclusion':conclusion,'display_title':f'oasis7-ci|workflow_dispatch|integration_revalidation|{uid}|{12 if uid==UID else 99}|{BASE}|{HEAD}'}

class SelectionTests(unittest.TestCase):
 def setUp(self):
  self.pr={'draft':True,'state':'open','merged':False,'body':f'Task: {UID}\nRefs #1','base':{'sha':BASE,'ref':'main'},'head':{'sha':HEAD}}
  self.runs=[run(20,conclusion='failure'),run(10)]
 def api(self,*args):
  path=args[-1]
  if '/pulls/' in path:return self.pr
  if '/runs?' in path:
   if getattr(self,'read_error',False):raise OSError('authority read unavailable')
   page=int(path.rsplit('page=',1)[1]);size=integration.DISCOVERY_PAGE_SIZE
   return {'workflow_runs':self.runs[(page-1)*size:page*size]}
  if '/contents/' in path:return {'type':'file','path':integration.WORKFLOW,'encoding':'base64','content':base64.b64encode(getattr(self,'workflow','no integration mode').encode()).decode()}
  raise AssertionError(path)
 def check(self,locator=None,allow_ready_pr=False):
  def verify(repo,uid,number,base,head,n,app):
   r=next(r for r in self.runs if r['id']==n)
   if getattr(self,'verification_error',False):raise OSError('artifact read uncertain')
   if getattr(self,'race',False):self.runs.insert(0,run(30,status='queued',conclusion=None))
   if getattr(self,'pr_race',None):self.pr=self.pr_race
   if r['conclusion']!='success' or r['status']!='completed':raise ValueError('current request not successful')
   return {'id':n},{'workflow_run_id':n}
  with patch.object(receipt,'gh',side_effect=self.api),patch.object(integration,'gh',side_effect=self.api),patch.object(receipt,'live',return_value=(self.pr,{'id':1},BASE,HEAD)),patch.object(integration,'verified_run',side_effect=verify):
   return receipt.selected_live('owner/repo',UID,1,12,'required-gate',42,allow_ready_pr=allow_ready_pr,integration_run_id=locator)
 def test_new_failure_blocks_even_normal_green(self):
  with self.assertRaisesRegex((SystemExit,ValueError),'current request'):self.check()
 def test_explicit_old_green_does_not_bypass_new_failure(self):
  with self.assertRaisesRegex((SystemExit,ValueError),'current|superseded'):self.check(10)
 def test_verified_other_task_does_not_hide_current_green(self):
  self.runs=[run(20,uid='task_'+'d'*32,conclusion='failure'),run(10)]
  self.assertEqual(self.check()[1]['id'],10)

 def test_new_pending_and_cancelled_block_old_green(self):
  for status,conclusion in [('queued',None),('in_progress',None),('completed','cancelled')]:
   self.runs=[run(20,status=status,conclusion=conclusion),run(10)]
   with self.assertRaisesRegex(SystemExit,'current request'):self.check()
 def test_uncertain_discovery_and_artifact_block(self):
  self.read_error=True
  with self.assertRaisesRegex(SystemExit,'authority read'):self.check()
  self.read_error=False;self.verification_error=True;self.runs=[run(20),run(10)]
  with self.assertRaisesRegex(SystemExit,'artifact read'):self.check()
 def test_more_than_twenty_unrelated_runs_and_multiple_pages(self):
  self.runs=[run(n,uid='task_'+'d'*32) for n in range(140,20,-1)]+[run(10)]
  self.assertEqual(self.check()[1]['id'],10)
 def test_discovery_cap_is_not_absence(self):
  self.runs=[run(n,uid='task_'+'d'*32) for n in range(140,20,-1)]
  with patch.object(integration,'DISCOVERY_MAX_PAGES',1):
   with self.assertRaisesRegex(SystemExit,'range exhausted'):self.check()
 def test_same_run_new_attempt_pending_blocks(self):
  latest=run(20,status='queued',conclusion=None);latest.update(run_attempt=2,updated_at='2026-09-09T00:40:00Z')
  self.runs=[latest,run(10)]
  with self.assertRaisesRegex(SystemExit,'current request'):self.check()
 def test_new_request_during_verification_blocks(self):
  self.runs=[run(20),run(10)];self.race=True
  with self.assertRaisesRegex(SystemExit,'changed during'):self.check()
 def test_pr_drift_after_integration_verification_blocks(self):
  self.runs=[run(20)]
  for changed in (
   {'base':{**self.pr['base'],'sha':'e'*40}},
   {'head':{'sha':'e'*40}},
   {'base':{**self.pr['base'],'ref':'release'}},
   {'state':'closed'}, {'merged':True}, {'draft':False}, {'body':'unbound'},
  ):
   with self.subTest(changed=changed):
    original=self.pr
    self.pr_race={**original,**changed}
    with self.assertRaisesRegex(SystemExit,'PR.*changed during'):
     self.check()
    self.pr=original
 def test_ready_pr_remains_allowed_on_final_readback(self):
  self.runs=[run(20)]
  self.pr_race={**self.pr,'draft':False}
  self.assertFalse(self.check(allow_ready_pr=True)[0]['draft'])
 def test_pre_activation_old_workflow_proves_unrelated(self):
  old=run(20);old['display_title']='Rust';self.runs=[old]
  self.assertEqual(self.check()[1]['id'],1)

 def test_unknown_request_on_capable_workflow_never_skips(self):
  self.workflow='integration_revalidation';self.runs[0]['display_title']='unknown'
  with self.assertRaisesRegex(SystemExit,'identity unavailable'):self.check()
 def test_spoofed_title_without_workflow_provenance_blocks(self):
  self.runs[0]['path']='.github/workflows/other.yml'
  with self.assertRaisesRegex(SystemExit,'provenance uncertain'):self.check()

 def test_queue_delay_does_not_make_old_dispatch_newest(self):
  old=run(10);old.update(created_at='2026-09-09T00:01:00Z',run_started_at='2026-09-09T00:10:00Z')
  new=run(20,conclusion='failure');new.update(created_at='2026-09-09T00:02:00Z',run_started_at='2026-09-09T00:03:00Z')
  self.runs=[new,old]
  with self.assertRaisesRegex(SystemExit,'current request'):self.check()

 def test_old_dispatch_rerun_cannot_override_new_dispatch_failure(self):
  old=run(10);old.update(run_attempt=2,run_started_at='2026-09-09T00:40:00Z',updated_at='2026-09-09T00:45:00Z')
  self.runs=[run(20,conclusion='failure'),old]
  with self.assertRaisesRegex(SystemExit,'current request'):self.check()

 def stale(self,uid):
  value=run(5,uid=uid,conclusion='failure')
  value['display_title']=value['display_title'].replace('|'+BASE+'|','|'+'e'*40+'|')
  return value
 def test_prepare_rejects_dispatch_base_race_before_git_mutation(self):
  prior='e'*40
  pr={**self.pr,'base':{'sha':prior,'ref':'main','repo':{'full_name':'owner/repo'}},'head':{'sha':HEAD,'repo':{'full_name':'owner/repo'}}}
  def api(*args):
   return pr if '/pulls/' in args[-1] else {'default_branch':'main'}
  with patch.object(integration,'gh',side_effect=api),patch.dict(integration.os.environ,{'GITHUB_EVENT_NAME':'workflow_dispatch','GITHUB_REF':'refs/heads/main','GITHUB_SHA':BASE,'GITHUB_WORKFLOW_SHA':BASE}),patch.object(integration,'git') as git:
   with self.assertRaisesRegex(ValueError,'immutable current default-branch authority'):
    integration.prepare(Path('/unused'), 'owner/repo',UID,12,prior,HEAD)
   git.assert_not_called()
 def test_other_task_stale_dispatch_does_not_block_normal_ci(self):
  self.runs=[self.stale('task_'+'d'*32)]
  self.assertEqual(self.check()[1]['id'],1)
 def test_other_task_stale_dispatch_allows_new_and_explicit_request(self):
  self.runs=[run(20),self.stale('task_'+'d'*32)]
  self.assertEqual(self.check()[1]['id'],20)
  self.assertEqual(self.check(20)[1]['id'],20)
 def test_same_task_stale_base_allows_correct_retry(self):
  self.runs=[run(20),self.stale(UID)]
  self.assertEqual(self.check(20)[1]['id'],20)
 def test_stale_dispatch_does_not_hide_exact_current_failure(self):
  self.runs=[run(20,conclusion='failure'),run(10),self.stale('task_'+'d'*32)]
  with self.assertRaisesRegex(SystemExit,'current request not successful'):self.check()

 def test_exact_request_wrong_execution_ref_blocks_old_green(self):
  for field,value in [('head_sha','e'*40),('head_branch','other')]:
   with self.subTest(field=field):
    bad=run(20,conclusion='failure');bad[field]=value;self.runs=[bad,run(10)]
    with self.assertRaisesRegex(SystemExit,'trusted workflow ref'):self.check()
    with self.assertRaisesRegex(SystemExit,'trusted workflow ref'):self.check(10)
 def test_new_correct_request_supersedes_old_wrong_execution_ref(self):
  for field,value in [('head_sha','e'*40),('head_branch','other')]:
   with self.subTest(field=field):
    old=run(10);old[field]=value;self.runs=[run(20),old]
    self.assertEqual(self.check()[1]['id'],20)
    self.assertEqual(self.check(20)[1]['id'],20)
 def test_unrelated_request_wrong_execution_ref_is_not_current(self):
  for field,value in [('head_sha','e'*40),('head_branch','other')]:
   other=run(20,uid='task_'+'d'*32,conclusion='failure');other[field]=value
   self.runs=[other,run(10)]
   self.assertEqual(self.check(10)[1]['id'],10)

if __name__=='__main__':unittest.main()
