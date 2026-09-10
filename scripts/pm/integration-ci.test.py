"""Manual integration revalidation uses current target and unchanged source."""
import importlib.util
import json
import io
import zipfile
from unittest.mock import patch
from pathlib import Path
import subprocess
import sys
from contextlib import redirect_stdout
import tempfile
import unittest
import os
import shutil
import textwrap

HERE=Path(__file__).parent
class IntegrationTests(unittest.TestCase):
 def test_required_workflow_uses_frozen_driver_and_preflight_on_candidate_root(self):
  for event in ('workflow_dispatch','pull_request','push'):
   for candidate_preflight in ('exit 0\n', 'viewer_dependency_preflight() { :; }\n'):
    with self.subTest(event=event, candidate_preflight=candidate_preflight), tempfile.TemporaryDirectory() as tmp:
     temp=Path(tmp);candidate=temp/'candidate';scripts=candidate/'scripts';scripts.mkdir(parents=True)
     frozen=temp/'integration-planner';frozen.mkdir()
     repo=HERE.parents[1]
     for name in ('ci-tests.sh','viewer-dependency-preflight.sh'):
      shutil.copy2(repo/'scripts'/name,frozen/name)
     marker=temp/'observed'
     (scripts/'ci-tests.sh').write_text('#!/bin/bash\nprintf candidate > "$OBSERVED"\nexit 0\n')
     (scripts/'ci-tests.sh').chmod(0o755)
     (scripts/'viewer-dependency-preflight.sh').write_text(candidate_preflight)
     (scripts/'doc-governance-check.sh').write_text('#!/bin/bash\npwd > "$OBSERVED"\nexit 37\n')
     (scripts/'doc-governance-check.sh').chmod(0o755)
     workflow=(repo/'.github/workflows/rust.yml').read_text()
     step=workflow.split('      - name: Run required test tier\n',1)[1].split('\n      - name:',1)[0]
     run=step.split('        run:',1)[1]
     command=textwrap.dedent(run.split('\n',1)[1]) if run.startswith(' |') else run.strip()
     env={**os.environ,'RUNNER_TEMP':str(temp),'GITHUB_WORKSPACE':str(candidate),
          'GITHUB_EVENT_NAME':event,'INTEGRATION_MODE':'integration_revalidation','OBSERVED':str(marker)}
     result=subprocess.run(['bash','-euo','pipefail','-c',command],cwd=candidate,env=env,text=True,capture_output=True)
     if event=='workflow_dispatch':
      self.assertEqual(result.returncode,37,result.stdout+result.stderr)
      self.assertEqual(marker.read_text().strip(),str(candidate))
     else:
      self.assertEqual(result.returncode,0,result.stdout+result.stderr)
      self.assertEqual(marker.read_text(),'candidate')

 def test_integration_freezes_sourced_preflight_before_checkout(self):
  workflow=(HERE.parents[1]/'.github/workflows/rust.yml').read_text()
  before=workflow.split('integration_ci.py" prepare',1)[0]
  self.assertRegex(before,r'cp[^\n]*scripts/viewer-dependency-preflight\.sh[^\n]*integration-planner')

 def test_required_gate_registers_review_plan_suite(self):
  driver=(HERE.parents[1]/'scripts/ci-tests.sh').read_text()
  operational=driver.split('run_operational_contract_tests() {',1)[1].split('\n}',1)[0]
  self.assertIn('run python3 ./scripts/pm/review-plan.test.py',operational)

 def test_real_parallel_merge_keeps_source_and_tests_current_base(self):
  self.assertTrue((HERE/'integration_ci.py').exists(),'manual integration recovery helper missing')
  spec=importlib.util.spec_from_file_location('integration_ci',HERE/'integration_ci.py');api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp)
   def git(*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
   git('init','-q');git('config','user.name','Test');git('config','user.email','test@example.invalid')
   (root/'base').write_text('base');git('add','.');git('commit','-qm','base');original=git('rev-parse','HEAD')
   git('switch','-c','source');(root/'source').write_text('source');git('add','.');git('commit','-qm','source');head=git('rev-parse','HEAD')
   git('switch','--detach',original);(root/'target').write_text('target');git('add','.');git('commit','-qm','target');base=git('rev-parse','HEAD')
   result=api.compose(root,base,head)
   self.assertEqual(git('rev-parse','source'),head)
   self.assertEqual(git('rev-parse','HEAD^{tree}'),result['tested_tree_oid'])
   self.assertEqual(result['scope_base_oid'],original)
   self.assertTrue((root/'target').exists());self.assertTrue((root/'source').exists())

class ProvenanceTests(unittest.TestCase):
 def setUp(self):
  spec=importlib.util.spec_from_file_location('integration_ci',HERE/'integration_ci.py');self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)
  self.base='a'*40;self.head='b'*40;self.uid='task_'+'c'*32
  self.pr={'state':'open','merged':False,'draft':True,'body':'Task: '+self.uid+'\nRefs #1','base':{'sha':self.base,'ref':'main','repo':{'full_name':'owner/repo'}},'head':{'sha':self.head,'repo':{'full_name':'owner/repo'}}}
  self.run={'run_attempt':1,'created_at':'2026-09-09T00:00:00Z','run_started_at':'2026-09-09T00:00:00Z','event':'workflow_dispatch','head_branch':'main','head_sha':self.base,'path':self.api.WORKFLOW,'status':'completed','conclusion':'success','repository':{'full_name':'owner/repo'},'check_suite_id':8}
  self.payload=dict(schema=self.api.ARTIFACT,repository='owner/repo',workflow_run_id=9,base_oid=self.base,head_oid=self.head,task_uid=self.uid,pr_number=12,workflow_sha=self.base,workflow_ref='owner/repo/.github/workflows/rust.yml@refs/heads/main',integration_mode='integration_revalidation',check_name='required-gate',scope_base_oid='d'*40,tested_tree_oid='e'*40,tested_commit_oid='f'*40)
 def read(self,*args):
  path=args[-1]
  if '/workflows/rust.yml/runs?' in path:return {'workflow_runs':[{**self.run,'id':9,'display_title':f'oasis7-ci|workflow_dispatch|integration_revalidation|{self.uid}|12|{self.base}|{self.head}'}]}
  if path.endswith('/pulls/12'):return self.pr
  if path=='repos/owner/repo':return {'default_branch':'main'}
  if path.endswith('/runs/9'):return self.run
  if 'check-runs' in path:return {'check_runs':[{'id':10,'name':'required-gate','app':{'id':42},'conclusion':'success','status':'completed','head_sha':self.base,'details_url':'https://github.com/owner/repo/actions/runs/9/job/10'}]}
  if 'artifacts?' in path:return {'artifacts':[{'id':11,'name':self.api.ARTIFACT,'expired':False,'workflow_run':{'id':9}}]}
  self.fail(path)
 def test_identity_requires_one_complete_canonical_task_field(self):
  other='task_'+'d'*32
  invalid=[
   '', 'SupersededTask: '+self.uid,
   'Task: '+other+'\nSupersededTask: '+self.uid,
   'Task: '+self.uid+'\nTask: '+self.uid,
   'Task: '+self.uid+'\nTask: '+other,
   'Task: '+self.uid+' trailing',
   'Task: '+self.uid+'\nTask: malformed',
  ]
  for body in invalid:
   with self.subTest(body=body), patch.object(self.api,'gh',side_effect=self.read):
    self.pr['body']=body
    with self.assertRaisesRegex(ValueError,'task identity'):
     self.api.identity('owner/repo',self.uid,12,self.base,self.head)

 def test_identity_accepts_exact_task_with_unrelated_history(self):
  for body in ('Task: '+self.uid+'\nRefs #1', 'Task: '+self.uid+'\r\nRefs #1',
               'Task: '+self.uid+'\nSupersededTask: task_'+'d'*32):
   with self.subTest(body=body), patch.object(self.api,'gh',side_effect=self.read):
    self.pr['body']=body
    pr,branch=self.api.identity('owner/repo',self.uid,12,self.base,self.head)
    self.assertEqual(branch,'main');self.assertEqual(pr,self.pr)
 def verify(self):
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   return self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42)
 def test_new_default_workflow_run_authority_passes(self):
  check,proof=self.verify();self.assertEqual(check['id'],10);self.assertEqual(proof['head_oid'],self.head)
 def test_old_event_or_candidate_workflow_cannot_refresh(self):
  for key,value in [('event','pull_request'),('head_sha','0'*40),('head_branch','candidate'),('conclusion','failure')]:
   old=self.run[key];self.run[key]=value
   with self.assertRaisesRegex(ValueError,'provenance'):self.verify()
   self.run[key]=old
 def test_artifact_base_source_tree_identity_fails_closed(self):
  for key,value in [('base_oid','0'*40),('head_oid','0'*40),('workflow_sha','0'*40),('task_uid','wrong'),('tested_tree_oid','invalid')]:
   old=self.payload[key];self.payload[key]=value
   with self.assertRaisesRegex(ValueError,'artifact authority'):self.verify()
   self.payload[key]=old
 def test_actual_selected_receipt_chain_binds_manual_run(self):
  spec=importlib.util.spec_from_file_location('ci_live',HERE/'ci-ready-receipt.py');receipt=importlib.util.module_from_spec(spec);spec.loader.exec_module(receipt)
  planner=dict(scope='full',selected_capabilities='',reason_summary='manual integration',changed_path_count=1,planner_config_sha256='sha256:'+'a'*64)
  planner.update({k:'true' for k in receipt.RUN_FIELDS})
  self.payload['planner']=planner
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  def reader(*args):
   if '/compare/' in args[-1]:return {'merge_base_commit':{'sha':self.payload['scope_base_oid']}}
   return self.read(*args)
  output=io.StringIO()
  argv=['ci-ready-receipt.py','--repository','owner/repo','--task-uid',self.uid,'--task-issue-number','1','--pr-number','12','--check-app-id','42','--planner-digest','auto','--integration-run-id','9']
  with patch.dict(sys.modules,{'integration_ci':self.api}),patch.object(self.api,'gh',side_effect=reader),patch.object(receipt,'gh',side_effect=reader),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()),patch.object(sys,'argv',argv),redirect_stdout(output):
   receipt.main()
  result=json.loads(output.getvalue())
  self.assertEqual(result['integration_run_id'],9)
  self.assertEqual(result['tested_tree_oid'],self.payload['tested_tree_oid'])
  self.assertEqual(result['head_oid'],self.head)
  self.assertEqual(result['base_oid'],self.base)
  self.assertEqual(result['scope_base_oid'],self.payload['scope_base_oid'])

 def test_missing_app_pin_is_rejected_before_read(self):
  with patch.object(self.api,'gh') as read:
   with self.assertRaisesRegex(ValueError,'non-null app'):self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,None)
   read.assert_not_called()
 def test_workflow_upload_precedes_candidate_execution(self):
  workflow=(HERE.parents[1]/'.github/workflows/rust.yml').read_text()
  required=workflow[workflow.index('  required-gate:'):workflow.index('  windows-package-rollout-behavior:')]
  self.assertIn('python3 -I "${RUNNER_TEMP}/integration-planner/plan-rust-required-scope.py"',required)
  self.assertIn("python3 -I - <<'PY'",required)
  self.assertLess(required.index('Upload required planner artifact'),required.index('Install pinned Rust toolchains'))
  self.assertLess(required.index('cp scripts/plan-rust-required-scope.py'),required.index('integration_ci.py" prepare'))

 def test_premerge_activation_cannot_dispatch_candidate(self):
  with patch.object(self.api,'gh',side_effect=[self.pr,self.pr,{'default_branch':'main'},{'content':'bm8gbW9kZQ=='}]),patch.object(self.api.subprocess,'run') as run:
   with self.assertRaisesRegex(ValueError,'activation pending'):self.api.dispatch('owner/repo',self.uid,12)
   run.assert_not_called()

if __name__=='__main__':unittest.main()
