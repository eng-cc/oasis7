"""Manual integration revalidation uses current target and unchanged source."""
import importlib.util
import base64
import hashlib
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


class TargetedProjectionPromotionTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory()
  self.root=Path(self.temp.name)
  self.git('init','-q','-b','main')
  self.git('config','user.name','Test')
  self.git('config','user.email','test@example.invalid')
  (self.root/'README').write_text('base\n',encoding='utf-8')
  (self.root/'scripts').mkdir()
  shutil.copy2(HERE.parents[1]/'scripts/ci-required-scope.v2.json',self.root/'scripts/ci-required-scope.v2.json')
  self.git('add','README','scripts/ci-required-scope.v2.json');self.git('commit','-qm','base')
  self.scope_base=self.git('rev-parse','HEAD')
  self.git('switch','-q','-c','source')
  self.changed_path='site/index.html'
  changed=self.root/self.changed_path
  changed.parent.mkdir(parents=True)
  changed.write_text('source change\n',encoding='utf-8')
  self.git('add',self.changed_path);self.git('commit','-qm','source')
  self.source_head=self.git('rev-parse','HEAD')
  self.git('switch','-q','--detach',self.scope_base)
  target_only=self.root/'scripts/pm/workflow-next.py'
  target_only.parent.mkdir(parents=True,exist_ok=True)
  target_only.write_text('target advance\n',encoding='utf-8')
  self.git('add','scripts/pm/workflow-next.py');self.git('commit','-qm','target advance')
  self.integration_base=self.git('rev-parse','HEAD')
  self.uid='task_'+'1'*32
  payload={
   'task_uid':self.uid,
   'source_head_oid':self.source_head,
   'scope_base_oid':self.scope_base,
   'changed_paths':[self.changed_path],
   'change_class':'unknown',
   'manual_roles':['repository_health_engineer','qa_engineer'],
   'domain_role':None,
   'test_profile':'required',
   'declared_tests':['required_gate_baseline'],
   'consumed_contracts':[{'id':'workflow-contract','revision':'v1'}],
   'public_semantics':[],
   'affected_consumers':['required-ci'],
   'closure_status':{'status':'complete','reason':'verified','evidence':[{
    'path':'scripts/ci-required-scope.v2.json',
    'sha256':'sha256:'+hashlib.sha256((HERE.parents[1]/'scripts/ci-required-scope.v2.json').read_bytes()).hexdigest(),
   }]},
  }
  self.payload=payload
  self.input_path=self.root/'projection-input.json'
  self.projection_path=self.root/'projection.json'
  self.write_projection()

 def tearDown(self):
  self.temp.cleanup()

 def git(self,*args):
  return subprocess.check_output(['git','-C',str(self.root),*args],text=True).strip()

 def write_projection(self):
  self.input_path.write_text(json.dumps(self.payload),encoding='utf-8')
  if self.projection_path.exists():
   self.projection_path.unlink()
  result=subprocess.run([
   str(HERE/'workflow-impact-projection.py'),'--root',str(HERE.parents[1]),
   '--input',str(self.input_path),'--out',str(self.projection_path),
  ],text=True,capture_output=True)
  self.assertEqual(result.returncode,0,result.stderr)
  self.projection=json.loads(self.projection_path.read_text(encoding='utf-8'))

 def canonical_digest(self,value):
  encoded=json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode('utf-8')
  return 'sha256:'+hashlib.sha256(encoded).hexdigest()

 def planner(self,event,base):
  result=subprocess.run([
   sys.executable,str(HERE.parents[1]/'scripts/plan-rust-required-scope.py'),
   '--event-name',event,'--run-mode','integration_revalidation' if event=='workflow_dispatch' else 'legacy',
   '--base-ref',base,'--head-ref',self.source_head,
   '--task-uid',self.uid,'--scope-base-oid',self.scope_base,
   '--impact-projection',str(self.projection_path),
  ],cwd=self.root,text=True,capture_output=True)
  self.assertEqual(result.returncode,0,result.stdout+result.stderr)
  return dict(line.split('=',1) for line in result.stdout.splitlines() if '=' in line)

 def test_targeted_pr_projection_is_accepted_and_upgraded_by_trusted_integration(self):
  pr=self.planner('pull_request',self.scope_base)
  self.assertEqual(pr['scope'],'targeted')
  self.assertEqual(pr['impact_projection_status'],'verified')
  self.assertEqual(pr['impact_projection_digest'],self.projection['projection_digest'])
  self.assertEqual(self.projection['closure_status']['status'],'complete')
  self.assertEqual(self.projection['ci_scope'],pr['scope'])
  self.assertEqual(self.projection['ci_capabilities'],pr['selected_capabilities'].split(';'))

  # The integration target has advanced with a target-only commit.  The
  # trusted workflow must execute the targeted gate while retaining the source
  # projection digest; comparing the projection to this broader diff would be
  # both incorrect and unsafe.
  integration=self.planner('workflow_dispatch',self.integration_base)
  self.assertEqual(integration['scope'],'targeted',integration)
  self.assertEqual(integration['impact_projection_status'],'verified')
  self.assertEqual(integration['impact_projection_digest'],self.projection['projection_digest'])
  self.assertEqual(integration['test_profile'],'required')
  self.assertEqual(integration['selected_capabilities'],'site_quality;workflow_governance')
  self.assertEqual(integration['needs_rust_toolchain'],'false')
  self.assertEqual(integration['changed_path_count'],'2')

 def test_unknown_closure_full_projection_is_accepted_with_exact_source_paths(self):
  self.payload['closure_status']={
   'status':'unknown','reason':'dependency closure is unavailable',
  }
  self.write_projection()
  all_capabilities=sorted(json.loads((HERE.parents[1]/'scripts/ci-required-scope.v2.json').read_text())['capabilities'])
  self.assertEqual(self.projection['ci_scope'],'full')
  self.assertEqual(self.projection['test_profile'],'full')
  self.assertEqual(self.projection['ci_capabilities'],all_capabilities)

  source=self.planner('pull_request',self.scope_base)
  self.assertEqual(source['scope'],'full')
  self.assertEqual(source['changed_path_count'],'1')
  self.assertEqual(source['changed_paths'],self.changed_path)

  integration=self.planner('workflow_dispatch',self.integration_base)
  self.assertEqual(integration['scope'],'full',integration)
  self.assertEqual(integration['selected_capabilities'],';'.join(all_capabilities))
  self.assertEqual(integration['impact_projection_status'],'verified')
  self.assertEqual(integration['impact_projection_digest'],self.projection['projection_digest'])

 def test_unknown_closure_under_scoped_projection_is_rejected(self):
  changed=dict(self.projection)
  changed['closure_status']={
   'status':'unknown','reason':'dependency closure is unavailable','evidence':[],
  }
  changed['ci_reasons']=changed['ci_reasons']+['dependency_closure_unverified:unknown']
  changed['projection_digest']=self.canonical_digest({
   key:value for key,value in changed.items() if key!='projection_digest'
  })
  self.projection_path.write_text(json.dumps(changed),encoding='utf-8')
  result=subprocess.run([
   sys.executable,str(HERE.parents[1]/'scripts/plan-rust-required-scope.py'),
   '--event-name','workflow_dispatch','--run-mode','integration_revalidation',
   '--base-ref',self.integration_base,'--head-ref',self.source_head,
   '--task-uid',self.uid,'--scope-base-oid',self.scope_base,
   '--impact-projection',str(self.projection_path),
  ],cwd=self.root,text=True,capture_output=True)
  self.assertNotEqual(result.returncode,0)
  self.assertIn('unverified dependency closure is not full',result.stderr)

 def test_integration_rejects_projection_with_different_source_paths(self):
  changed=dict(self.projection)
  changed['changed_paths']=sorted([self.changed_path,'README'])
  changed['changed_paths_digest']=self.canonical_digest(changed['changed_paths'])
  changed['projection_digest']=self.canonical_digest({
   key:value for key,value in changed.items() if key!='projection_digest'
  })
  self.projection_path.write_text(json.dumps(changed),encoding='utf-8')
  result=subprocess.run([
   sys.executable,str(HERE.parents[1]/'scripts/plan-rust-required-scope.py'),
   '--event-name','workflow_dispatch','--run-mode','integration_revalidation',
   '--base-ref',self.integration_base,'--head-ref',self.source_head,
   '--task-uid',self.uid,'--scope-base-oid',self.scope_base,
   '--impact-projection',str(self.projection_path),
  ],cwd=self.root,text=True,capture_output=True)
  self.assertNotEqual(result.returncode,0)
  self.assertIn('source changed paths identity mismatch',result.stderr)

 def test_recomputed_projection_cannot_claim_wrong_source_scope(self):
  changed=dict(self.projection,ci_scope='minimal',ci_capabilities=['required_gate_baseline'])
  changed['planner_identity']=dict(changed['planner_identity'],scope='minimal',selected_capabilities=['required_gate_baseline'])
  changed['planner_digest']=self.canonical_digest(changed['planner_identity'])
  changed['projection_digest']=self.canonical_digest({k:v for k,v in changed.items() if k!='projection_digest'})
  self.projection_path.write_text(json.dumps(changed),encoding='utf-8')
  result=subprocess.run([
   sys.executable,str(HERE.parents[1]/'scripts/plan-rust-required-scope.py'),
   '--event-name','workflow_dispatch','--run-mode','integration_revalidation',
   '--base-ref',self.integration_base,'--head-ref',self.source_head,
   '--task-uid',self.uid,'--scope-base-oid',self.scope_base,
   '--impact-projection',str(self.projection_path),
  ],cwd=self.root,text=True,capture_output=True)
  self.assertNotEqual(result.returncode,0)
  self.assertIn('source scope identity mismatch',result.stderr)


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
     (temp/'impact-projection.json').write_text('{}')
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
          'GITHUB_EVENT_NAME':event,'INTEGRATION_MODE':'integration_revalidation','OBSERVED':str(marker),
          # This contract isolates frozen driver/preflight routing.  Required CI
          # itself exports package-profile activation identity; do not leak that
          # unrelated outer workflow state into this intentionally non-Git fixture.
          'OASIS7_CARGO_SCOPE_BASE':'','OASIS7_CARGO_SCOPE_HEAD':'',
          'OASIS7_CARGO_PROFILE_PLANNER':'','OASIS7_CARGO_PROFILE_DRIVER':''}
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
 def test_independent_merge_worktree_preserves_trusted_checkout(self):
  spec=importlib.util.spec_from_file_location('integration_ci',HERE/'integration_ci.py');api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp)/'trusted';root.mkdir()
   destination=Path(tmp)/'integration'
   def git(path,*args):return subprocess.check_output(['git','-C',str(path),*args],text=True).strip()
   git(root,'init','-q','-b','main');git(root,'config','user.name','Test');git(root,'config','user.email','test@example.invalid')
   (root/'base').write_text('base');git(root,'add','.');git(root,'commit','-qm','base');original=git(root,'rev-parse','HEAD')
   git(root,'switch','-q','-c','source');(root/'source').write_text('source');git(root,'add','.');git(root,'commit','-qm','source');head=git(root,'rev-parse','HEAD')
   git(root,'switch','-q','--detach',original);(root/'target').write_text('target');git(root,'add','.');git(root,'commit','-qm','target');base=git(root,'rev-parse','HEAD')
   result=api.compose(root,base,head,worktree_path=destination)
   self.assertEqual(base,git(root,'rev-parse','HEAD'))
   self.assertEqual(result['tested_commit_oid'],git(destination,'rev-parse','HEAD'))
   self.assertEqual(result['tested_tree_oid'],git(destination,'rev-parse','HEAD^{tree}'))
   self.assertEqual([base,head],git(destination,'rev-list','--parents','-n','1','HEAD').split()[1:])
   self.assertEqual(destination.resolve(),Path(result['integration_worktree']))

class ProvenanceTests(unittest.TestCase):
 def setUp(self):
  spec=importlib.util.spec_from_file_location('integration_ci',HERE/'integration_ci.py');self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)
  self.base='a'*40;self.head='b'*40;self.uid='task_'+'c'*32
  self.pr={'state':'open','merged':False,'draft':True,'body':'Task: '+self.uid+'\nRefs #1','base':{'sha':self.base,'ref':'main','repo':{'full_name':'owner/repo'}},'head':{'sha':self.head,'repo':{'full_name':'owner/repo'}}}
  self.run={'run_attempt':1,'created_at':'2026-09-09T00:00:00Z','run_started_at':'2026-09-09T00:00:00Z','event':'workflow_dispatch','head_branch':'main','head_sha':self.base,'path':self.api.WORKFLOW,'status':'completed','conclusion':'success','repository':{'full_name':'owner/repo'},'check_suite_id':8}
  self.payload=dict(schema=self.api.ARTIFACT,repository='owner/repo',workflow_run_id=9,base_oid=self.base,head_oid=self.head,task_uid=self.uid,pr_number=12,workflow_sha=self.base,workflow_ref='owner/repo/.github/workflows/rust.yml@refs/heads/main',integration_mode='integration_revalidation',check_name='required-gate',scope_base_oid='d'*40,tested_tree_oid='e'*40,tested_commit_oid='f'*40)
  from integration_executor_contract import EXECUTOR_CONTRACT_PATHS
  self.executor_contents={path:('trusted fixture '+path).encode() for path in EXECUTOR_CONTRACT_PATHS}
 def read(self,*args):
  path=args[-1]
  if '/workflows/rust.yml/runs?' in path:return {'workflow_runs':[{**self.run,'id':9,'display_title':f'oasis7-ci|workflow_dispatch|integration_revalidation|{self.uid}|12|{self.base}|{self.head}'}]}
  if '/contents/' in path:
   relative=path.split('/contents/',1)[1].split('?ref=',1)[0]
   if relative not in self.executor_contents:self.fail(path)
   return {'type':'file','path':relative,'encoding':'base64','content':base64.b64encode(self.executor_contents[relative]).decode()}
  if '/compare/' in path:return {'merge_base_commit':{'sha':self.base}}
  if path.endswith('/pulls/12'):return self.pr
  if path=='repos/owner/repo':return {'default_branch':'main'}
  if path.endswith('/runs/9'):return self.run
  if 'check-runs' in path:return {'check_runs':[{'id':10,'name':'required-gate','app':{'id':42},'conclusion':'success','status':'completed','head_sha':self.run['head_sha'],'details_url':'https://github.com/owner/repo/actions/runs/9/job/10'}]}
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
 def test_keyed_request_preflight_requires_key_in_authoritative_run_name(self):
  required='run-name: oasis7-ci|${{ inputs.request_key }}\non:\n  workflow_dispatch:\n    inputs:\n      request_key:\n      validation_request_b64:'
  missing_title=required.replace('|${{ inputs.request_key }}','|${{ inputs.expected_head }}')
  self.assertTrue(self.api.keyed_request_workflow_ready(required))
  self.assertFalse(self.api.keyed_request_workflow_ready(missing_title))
  self.assertFalse(self.api.keyed_request_workflow_ready(required+'\nrun-name: duplicate'))
 def test_keyed_request_preflight_ignores_comment_only_input_declarations(self):
  workflow='run-name: oasis7-ci|${{ inputs.request_key }}\non:\n  workflow_dispatch:\n    inputs:\n      run_mode:\n        required: true\n# request_key:\n# validation_request_b64:\n# inputs.request_key\n'
  self.assertFalse(self.api.keyed_request_workflow_ready(workflow))
 def test_new_default_workflow_run_authority_passes(self):
  check,proof=self.verify();self.assertEqual(check['id'],10);self.assertEqual(proof['head_oid'],self.head)
 def test_keyed_workflow_base_divergence_requires_approved_w_contract(self):
  from integration_executor_contract import executor_contract_from_contents
  workflow_sha='9'*40;run_head='6'*40;request_key='sha256:'+'8'*64
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  self.run['head_sha']=run_head
  self.payload.update(workflow_sha=workflow_sha,workflow_run_head_sha=run_head,request_key=request_key,executor_contract_digest=executor_digest)
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   check,proof=self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,approved_executor_contract_digests=[executor_digest])
  self.assertEqual(workflow_sha,proof['workflow_sha'])
  self.assertEqual(self.base,proof['base_oid'])
  self.assertEqual(run_head,proof['workflow_run_head_sha'])
  self.assertEqual(run_head,check['head_sha'])
 def test_keyed_workflow_base_divergence_rejects_revoked_w_contract(self):
  from integration_executor_contract import executor_contract_from_contents
  workflow_sha='9'*40;run_head='6'*40;request_key='sha256:'+'8'*64
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  self.run['head_sha']=run_head
  self.payload.update(workflow_sha=workflow_sha,workflow_run_head_sha=run_head,request_key=request_key,executor_contract_digest=executor_digest)
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   with self.assertRaisesRegex(ValueError,'EXECUTOR_CONTRACT_CHANGED'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,approved_executor_contract_digests=['sha256:'+'7'*64])
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
  profile_payloads={
   'plan':b'{"plan":true}\n','results':b'[]\n','receipt':b'{"status":"passed"}\n',
  }
  envelope={
   'schema':'oasis7-cargo-package-profile-envelope/v1','repository':'owner/repo',
   'task_uid':self.uid,'pr_number':12,
   'workflow_ref':self.payload['workflow_ref'],'workflow_sha':self.base,
   'run_id':9,'run_attempt':1,'check_name':'required-gate','check_app_id':42,'check_run_id':10,
   'integration_base':self.base,'source_head':self.head,'tested_tree':self.payload['tested_tree_oid'],
  }
  for key,value in profile_payloads.items():
   envelope[key+'_digest']='sha256:'+hashlib.sha256(value).hexdigest()
  profile_payloads['envelope']=(json.dumps(envelope)+'\n').encode()
  artifact_names={11:self.api.ARTIFACT,21:'cargo-package-profile-envelope',22:'cargo-package-profile-plan',23:'cargo-package-profile-results',24:'cargo-package-profile-receipt'}
  def reader(*args):
   if '/compare/' in args[-1]:return {'merge_base_commit':{'sha':self.payload['scope_base_oid']}}
   if 'artifacts?' in args[-1]:return {'artifacts':[{'id':identifier,'name':name,'expired':False,'workflow_run':{'id':9}} for identifier,name in artifact_names.items()]}
   return self.read(*args)
  def profile_artifact(repository,identifier):
   if identifier==11:return raw.getvalue()
   key={21:'envelope',22:'plan',23:'results',24:'receipt'}[identifier]
   member=receipt.PROFILE_ARTIFACTS[key][1]
   zipped=io.BytesIO()
   with zipfile.ZipFile(zipped,'w') as archive:archive.writestr(member,profile_payloads[key])
   return zipped.getvalue()
  output=io.StringIO()
  argv=['ci-ready-receipt.py','--repository','owner/repo','--task-uid',self.uid,'--task-issue-number','1','--pr-number','12','--check-app-id','42','--planner-digest','auto','--integration-run-id','9']
  with patch.dict(sys.modules,{'integration_ci':self.api}),patch.object(self.api,'gh',side_effect=reader),patch.object(receipt,'gh',side_effect=reader),patch.object(receipt,'artifact_bytes',side_effect=profile_artifact),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()),patch.object(sys,'argv',argv),redirect_stdout(output):
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

 def test_pull_request_base_planner_bundle_includes_impact_helper(self):
  workflow=(HERE.parents[1]/'.github/workflows/rust.yml').read_text()
  required=workflow[workflow.index('  required-gate:'):workflow.index('  windows-package-rollout-behavior:')]
  mkdir='mkdir -p "${authority_dir}/pm"'
  helper='git show "${base_ref}:scripts/pm/workflow-impact-projection.py" >"${authority_dir}/pm/workflow-impact-projection.py"'
  planner='planner=(python3 -I "${authority_dir}/plan-rust-required-scope.py")'
  self.assertIn(mkdir,required)
  self.assertIn(helper,required)
  self.assertLess(required.index(mkdir),required.index(helper))
  self.assertLess(required.index(helper),required.index(planner))

 def test_premerge_activation_cannot_dispatch_candidate(self):
  def api(*args):
   path=args[-1]
   if path.endswith('/pulls/12'):return self.pr
   if path=='repos/owner/repo':return {'default_branch':'main'}
   if path=='repos/owner/repo/git/ref/heads/main':return {'object':{'sha':self.base}}
   if path.endswith('/contents/.github/workflows/rust.yml?ref='+self.base):return {'content':'bm8gbW9kZQ=='}
   self.fail(path)
  with patch.object(self.api,'gh',side_effect=api),patch.object(self.api.subprocess,'run') as run:
   with self.assertRaisesRegex(ValueError,'activation pending'):self.api.dispatch('owner/repo',self.uid,12,None)
   run.assert_not_called()

if __name__=='__main__':unittest.main()
