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
from contextlib import redirect_stdout, contextmanager
import tempfile
import unittest
import os
import shutil
import textwrap
import time

HERE=Path(__file__).parent


def required_test_tier_command(repo, scope_oid='b'*40):
 workflow=(repo/'.github/workflows/rust.yml').read_text()
 step=workflow.split('      - name: Run required test tier\n',1)[1].split('\n      - name:',1)[0]
 run=step.split('        run:',1)[1]
 command=textwrap.dedent(run.split('\n',1)[1]) if run.startswith(' |') else run.strip()
 # GitHub substitutes these trusted workflow expressions before Bash runs.
 # Keep the extracted script executable in this local shell fixture.
 substitutions={
  '${{ steps.scope.outputs.source_scope_base }}':scope_oid,
  '${{ steps.scope.outputs.head_oid }}':scope_oid,
  '${{ steps.scope.outputs.integration_base_oid }}':scope_oid,
  '${{ github.token }}':'fixture-token',
  '${{ inputs.task_uid }}':'task_'+'1'*32,
  '${{ inputs.pr_number }}':'7',
 }
 for source,target in substitutions.items():
  command=command.replace(source,target)
 if '${{' in command:
  raise AssertionError('unexpanded GitHub expression remains in required-tier fixture')
 return command


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
  for relative in ['scripts/plan-rust-required-scope.py','scripts/ci-tests.sh','scripts/pm/workflow-impact-projection.py']:
   destination=self.root/relative
   destination.parent.mkdir(parents=True,exist_ok=True)
   shutil.copy2(HERE.parents[1]/relative,destination)
  self.git('add','README','scripts');self.git('commit','-qm','base')
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
  self.assertEqual(pr['needs_rust_toolchain'],'false')
  self.assertEqual(pr['run_rust_baseline'],'false')

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
  self.assertEqual(integration['needs_rust_toolchain'],'true')
  self.assertEqual(integration['run_rust_baseline'],'true')
  self.assertEqual(integration['changed_path_count'],'2')

 def workflow_run_script(self,marker):
  workflow=(HERE.parents[1]/'.github/workflows/rust.yml').read_text()
  step=workflow.split(marker,1)[1].split('\n      - ',1)[0]
  return textwrap.dedent(step.split('        run: |\n',1)[1])

 def execute_historical_pr_workflow(self,merge_target,reject=None):
  if merge_target:
   self.git('switch','-q','source')
   self.git('merge','--no-edit',self.integration_base)
   self.source_head=self.git('rev-parse','HEAD')
  fixture=self.root/'github-fixture.json'
  fixture.write_text(json.dumps({'pr':{'state':'open','merged':False,'base':{
   'ref':'main','sha':self.scope_base,'repo':{'full_name':'owner/repo'}},'head':{
   'sha':self.source_head,'repo':{'full_name':'owner/repo'}}},'target':self.integration_base}))
  if reject in ('head','ref','repo','closed'):
   payload=json.loads(fixture.read_text())
   if reject=='head':payload['pr']['head']['sha']='f'*40
   elif reject=='ref':payload['pr']['base']['ref']='release'
   elif reject=='repo':payload['pr']['head']['repo']['full_name']='fork/repo'
   else:payload['pr']['state']='closed'
   fixture.write_text(json.dumps(payload))
  bin_dir=self.root/'test-bin';bin_dir.mkdir()
  gh=bin_dir/'gh'
  gh.write_text('#!/usr/bin/env python3\nimport json,os,sys\nf=json.load(open(os.environ["GH_FIXTURE"]))\np=sys.argv[-1]\n'
   'if os.environ.get("TARGET_DRIFT") and "/git/ref/" in p:\n'
   ' c=os.environ["GH_FIXTURE"]+".count"; n=int(open(c).read()) if os.path.exists(c) else 0;open(c,"w").write(str(n+1));f["target"]=f["target"] if n==0 else "f"*40\n'
   'print(json.dumps(f["pr"] if "/pulls/" in p else {"object":{"sha":f["target"]}} if "/git/ref/" in p else {"full_name":"owner/repo","default_branch":"main"}))\n')
  gh.chmod(0o755)
  runner=self.root/'runner';runner.mkdir()
  output=self.root/'target-output'
  env={**os.environ,'PATH':str(bin_dir)+os.pathsep+os.environ['PATH'],'GH_FIXTURE':str(fixture),
   'GITHUB_OUTPUT':str(output),'RUNNER_TEMP':str(runner),'GITHUB_EVENT_NAME':'pull_request',
   'GITHUB_SHA':self.source_head,'PR_BODY':''}
  if reject=='target_drift':env['TARGET_DRIFT']='1'
  replacements={'${{ github.repository }}':'owner/repo','${{ github.event.pull_request.number }}':'7',
   '${{ github.event.pull_request.head.sha }}':self.source_head,'${{ github.event.pull_request.base.sha }}':self.scope_base,
   '${{ steps.pr_target.outputs.oid }}':self.integration_base,'${{ inputs.integration_base }}':'',
   '${{ inputs.expected_head }}':'','${{ inputs.task_uid }}':'','${{ inputs.run_mode }}':'',
   '${{ github.event.before }}':''}
  def execute(script,expect_failure=False):
   for source,target in replacements.items():script=script.replace(source,target)
   self.assertNotIn('${{',script)
   result=subprocess.run(['bash','-e','-c',script],cwd=self.root,env=env,text=True,capture_output=True)
   if expect_failure:self.assertNotEqual(result.returncode,0)
   else:self.assertEqual(result.returncode,0,result.stdout+result.stderr)
  execute(self.workflow_run_script('      - id: pr_target\n'),reject in ('head','ref','repo','closed','target_drift'))
  if reject in ('head','ref','repo','closed','target_drift'):return
  self.assertEqual(output.read_text().strip(),'oid='+self.integration_base)
  output.write_text('')
  if reject=='missing_target':replacements['${{ steps.pr_target.outputs.oid }}']='f'*40
  if reject=='ambiguous_scope':
   fake_git=bin_dir/'git'
   real_git=subprocess.check_output(['which','git'],text=True).strip()
   fake_git.write_text('#!/bin/sh\nif [ "$1" = merge-base ]; then printf "%s\\n%s\\n" '+self.scope_base+' '+self.integration_base+'; else exec '+real_git+' "$@"; fi\n')
   fake_git.chmod(0o755)
  execute(self.workflow_run_script('      - id: scope\n'),reject in ('missing_target','ambiguous_scope'))
  if reject in ('missing_target','ambiguous_scope'):return
  results=dict(line.split('=',1) for line in output.read_text().splitlines())
  expected_scope=self.git('merge-base',self.integration_base,self.source_head)
  self.assertEqual(results['source_scope_base'],expected_scope)
  self.assertEqual(results['integration_base_oid'],self.integration_base)
  self.assertEqual(results['changed_path_count'],'1')
  self.assertEqual(self.git('diff','--name-only',expected_scope,self.source_head),self.changed_path)
  # Trusted planner files originate at Q, not a candidate source helper.
  self.assertEqual((runner/'required-base-authority/plan-rust-required-scope.py').read_bytes(),
   subprocess.check_output(['git','-C',str(self.root),'show',self.integration_base+':scripts/plan-rust-required-scope.py']))

 def test_executable_workflow_historical_base_source_only_excludes_upstream(self):
  self.execute_historical_pr_workflow(False)

 def test_executable_workflow_historical_base_merged_source_excludes_upstream(self):
  self.execute_historical_pr_workflow(True)

 def test_executable_workflow_rejects_head_drift(self):self.execute_historical_pr_workflow(False,'head')
 def test_executable_workflow_rejects_wrong_ref(self):self.execute_historical_pr_workflow(False,'ref')
 def test_executable_workflow_rejects_wrong_repository(self):self.execute_historical_pr_workflow(False,'repo')
 def test_executable_workflow_rejects_closed_pr(self):self.execute_historical_pr_workflow(False,'closed')
 def test_executable_workflow_rejects_target_read_race(self):self.execute_historical_pr_workflow(False,'target_drift')
 def test_executable_workflow_rejects_missing_fetched_target(self):self.execute_historical_pr_workflow(False,'missing_target')
 def test_executable_workflow_rejects_ambiguous_scope(self):self.execute_historical_pr_workflow(False,'ambiguous_scope')

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
  repo=HERE.parents[1]
  trusted_base_ref=os.environ.get('OASIS7_CARGO_SCOPE_BASE','').strip()
  if not trusted_base_ref:
   resolved=subprocess.run(['git','merge-base','HEAD','refs/remotes/origin/main'],cwd=repo,text=True,capture_output=True)
   self.assertEqual(resolved.returncode,0,'trusted Cargo profile base unavailable: '+resolved.stderr.strip())
   trusted_base_ref=resolved.stdout.strip()
  trusted_base=subprocess.run(['git','rev-parse','--verify',f'{trusted_base_ref}^{{commit}}'],cwd=repo,text=True,capture_output=True)
  self.assertEqual(trusted_base.returncode,0,'trusted Cargo profile base unavailable: '+trusted_base.stderr.strip())
  trusted_base_oid=trusted_base.stdout.strip()
  trusted_paths=(
   '.pm/cargo-package-scope-policy.json',
   'scripts/pm/check-cargo-package-scope',
   'scripts/pm/workflow-impact-projection.py',
   'scripts/pm/cargo_package_profile_planner.py',
   'scripts/pm/cargo_package_profile_driver.py',
  )
  trusted_blobs={}
  trusted_modes={}
  for path in trusted_paths:
   blob=subprocess.run(['git','show',f'{trusted_base_oid}:{path}'],cwd=repo,capture_output=True)
   self.assertEqual(blob.returncode,0,f'trusted Cargo profile base is missing {path}')
   trusted_blobs[path]=blob.stdout
   entry=subprocess.check_output(['git','ls-tree',trusted_base_oid,'--',path],cwd=repo,text=True).split()
   self.assertTrue(entry,f'trusted Cargo profile base has no tree entry for {path}')
   trusted_modes[path]=entry[0]
  toolchain=subprocess.run(['git','show',f'{trusted_base_oid}:rust-toolchain.toml'],cwd=repo,capture_output=True)
  if toolchain.returncode==0:
   trusted_blobs['rust-toolchain.toml']=toolchain.stdout
   entry=subprocess.check_output(['git','ls-tree',trusted_base_oid,'--','rust-toolchain.toml'],cwd=repo,text=True).split()
   trusted_modes['rust-toolchain.toml']=entry[0]

  workflow=(repo/'.github/workflows/rust.yml').read_text()
  step=workflow.split('      - name: Run required test tier\n',1)[1].split('\n      - name:',1)[0]
  run=step.split('        run:',1)[1]
  raw_command=textwrap.dedent(run.split('\n',1)[1]) if run.startswith(' |') else run.strip()
  fixture_uid='task_'+'1'*32
  fixture_pr='1'
  substitutions=(
   ('${{ steps.scope.outputs.integration_base_oid }}',None),
   ('${{ github.token }}','fixture-token'),
   ('${{ inputs.task_uid }}',fixture_uid),
   ('${{ inputs.pr_number }}',fixture_pr),
  )
  for event in ('workflow_dispatch','pull_request','push'):
   for candidate_preflight in ('exit 0\n', 'viewer_dependency_preflight() { :; }\n'):
    with self.subTest(event=event,candidate_preflight=candidate_preflight), tempfile.TemporaryDirectory() as tmp:
     temp=Path(tmp);candidate=temp/'candidate';candidate.mkdir()
     frozen=temp/'integration-planner';frozen.mkdir()
     for name in ('ci-tests.sh','viewer-dependency-preflight.sh'):
      shutil.copy2(repo/'scripts'/name,frozen/name)
     frozen_preflight=(frozen/'viewer-dependency-preflight.sh').read_bytes()
     marker=temp/'observed'
     (temp/'impact-projection.json').write_text('{}',encoding='utf-8')

     def git(*args):
      return subprocess.check_output(['git','-C',str(candidate),*args],text=True).strip()

     subprocess.run(['git','init','-q','-b','main'],cwd=candidate,check=True)
     git('config','user.name','Integration Fixture')
     git('config','user.email','integration-fixture@example.invalid')
     for path,blob in trusted_blobs.items():
      destination=candidate/path
      destination.parent.mkdir(parents=True,exist_ok=True)
      destination.write_bytes(blob)
      destination.chmod(0o755 if trusted_modes[path]=='100755' else 0o644)
     (candidate/'Cargo.toml').write_text('[workspace]\nmembers = ["crates/profile-fixture"]\nresolver = "2"\n',encoding='utf-8')
     package=candidate/'crates/profile-fixture'
     (package/'src').mkdir(parents=True)
     (package/'Cargo.toml').write_text('[package]\nname = "profile-fixture"\nversion = "0.1.0"\nedition = "2021"\n',encoding='utf-8')
     (package/'src/lib.rs').write_text('pub fn fixture() {}\n',encoding='utf-8')
     git('add','-A')
     git('commit','-qm','trusted profile authority and minimal Cargo workspace')
     scope_base=git('rev-parse','HEAD')

     git('switch','-q','-c','source')
     source_path=candidate/'site/index.html'
     source_path.parent.mkdir(parents=True)
     source_path.write_text('<!doctype html><title>source change</title>\n',encoding='utf-8')
     git('add','site/index.html')
     git('commit','-qm','source-only site change')
     source_head=git('rev-parse','HEAD')

     git('switch','-q','--detach',scope_base)
     scripts=candidate/'scripts'
     (scripts/'ci-tests.sh').write_text('#!/bin/bash\nprintf candidate > "$OBSERVED"\nexit 0\n',encoding='utf-8')
     (scripts/'ci-tests.sh').chmod(0o755)
     (scripts/'viewer-dependency-preflight.sh').write_text(candidate_preflight,encoding='utf-8')
     (scripts/'doc-governance-check.sh').write_text('#!/bin/bash\npwd > "$OBSERVED"\nexit 37\n',encoding='utf-8')
     (scripts/'doc-governance-check.sh').chmod(0o755)
     git('add','scripts/ci-tests.sh','scripts/viewer-dependency-preflight.sh','scripts/doc-governance-check.sh')
     git('commit','-qm','integration-only candidate script changes')
     integration_base=git('rev-parse','HEAD')
     tested_tree=git('merge-tree','--write-tree',integration_base,source_head)
     git('merge','--no-ff','--no-edit','source')
     self.assertEqual(git('rev-parse','HEAD^{tree}'),tested_tree)
     self.assertEqual(git('diff','--name-only',scope_base,source_head),'site/index.html')
     self.assertEqual(
      git('diff','--name-only',scope_base,integration_base).splitlines(),
      ['scripts/ci-tests.sh','scripts/doc-governance-check.sh','scripts/viewer-dependency-preflight.sh'],
     )
     self.assertEqual(frozen_preflight,(repo/'scripts/viewer-dependency-preflight.sh').read_bytes())
     self.assertNotEqual(frozen_preflight,(scripts/'viewer-dependency-preflight.sh').read_bytes())

     event_path=temp/'event.json'
     event_path.write_text(json.dumps({'inputs':{
      'integration_base':integration_base,'expected_head':source_head,
      'task_uid':fixture_uid,'pr_number':fixture_pr,'run_mode':'integration_revalidation',
     }}),encoding='utf-8')
     gh_bin=temp/'bin';gh_bin.mkdir()
     gh_api_log=temp/'gh-api-calls'
     gh=(gh_bin/'gh')
     gh.write_text(textwrap.dedent('''\
      #!/usr/bin/env bash
      set -euo pipefail
      expected="api repos/${GITHUB_REPOSITORY}/commits/${GITHUB_SHA}/check-runs?per_page=100"
      if [[ "$#" -ne 2 || "$1 $2" != "$expected" ]]; then
        printf 'unexpected gh API request: %s\\n' "$*" >&2
        exit 2
      fi
      printf '%s\\n' "$*" >>"$GH_API_CALL_LOG"
      printf '{"check_runs":[{"name":"required-gate","details_url":"https://github.com/%s/actions/runs/%s","app":{"id":1},"id":1}]}\\n' "$GITHUB_REPOSITORY" "$GITHUB_RUN_ID"
     '''),encoding='utf-8')
     gh.chmod(0o755)
     env=os.environ.copy()
     for name in (
      'OASIS7_PRODUCT_DOC_BASE','OASIS7_PRODUCT_DOC_HEAD','OASIS7_CARGO_SCOPE_CHECKER',
      'OASIS7_CARGO_PROFILE_OPT_IN','OASIS7_CARGO_PROFILE_PLAN','OASIS7_CARGO_PROFILE_RESULTS',
      'OASIS7_CARGO_PROFILE_INTEGRATION_BASE','OASIS7_CARGO_PROFILE_SOURCE_HEAD',
      'OASIS7_CARGO_PROFILE_TESTED_TREE',
     ):
      env.pop(name,None)
     env.update({
      'RUNNER_TEMP':str(temp),'GITHUB_WORKSPACE':str(candidate),'INTEGRATION_WORKTREE':str(candidate),
      'GITHUB_EVENT_PATH':str(event_path),'GITHUB_EVENT_NAME':event,
      'INTEGRATION_MODE':'integration_revalidation','OBSERVED':str(marker),
      'GH_API_CALL_LOG':str(gh_api_log),'PATH':str(gh_bin)+os.pathsep+env.get('PATH',''),
      'GITHUB_REPOSITORY':'fixture/oasis7','GITHUB_SHA':source_head,
      'GITHUB_WORKFLOW_REF':'fixture/oasis7/.github/workflows/rust.yml@refs/heads/main',
      'GITHUB_WORKFLOW_SHA':scope_base,'GITHUB_RUN_ID':'1701','GITHUB_RUN_ATTEMPT':'1',
      'GITHUB_ACTIONS':'true','GH_TOKEN':'fixture-token',
      'OASIS7_CARGO_SCOPE_INTEGRATION_BASE':integration_base,
     })
     profile_output=candidate/'output/cargo-package-profile'
     command=raw_command
     for expression,replacement in substitutions:
      if replacement is None:
       replacement=integration_base
      self.assertEqual(command.count(expression),1,f'expected one workflow interpolation for {expression}')
      command=command.replace(expression,replacement)
     self.assertNotIn('${{',command)
     if event=='workflow_dispatch':
      missing_authority_env=env.copy()
      missing_authority_env.update({
       'OASIS7_CARGO_SCOPE_BASE':'','OASIS7_CARGO_SCOPE_HEAD':'',
       'OASIS7_CARGO_PROFILE_PLANNER':'','OASIS7_CARGO_PROFILE_DRIVER':'',
      })
      missing=subprocess.run(['bash','-euo','pipefail','-c',command],cwd=candidate,env=missing_authority_env,text=True,capture_output=True)
      self.assertEqual(missing.returncode,1,missing.stdout+missing.stderr)
      self.assertEqual(missing.stderr,'trusted Cargo package profile authority is unavailable\n')
      self.assertFalse(marker.exists())
      self.assertFalse(profile_output.exists())
      self.assertFalse(gh_api_log.exists())

      positive_env=env.copy()
      positive_env.update({'OASIS7_CARGO_SCOPE_BASE':scope_base,'OASIS7_CARGO_SCOPE_HEAD':source_head})
      result=subprocess.run(['bash','-euo','pipefail','-c',command],cwd=candidate,env=positive_env,text=True,capture_output=True)
      self.assertEqual(result.returncode,37,result.stdout+result.stderr)
      self.assertEqual(marker.read_text(encoding='utf-8').strip(),str(candidate))
      self.assertEqual(gh_api_log.read_text(encoding='utf-8').splitlines(),[
       f'api repos/{positive_env["GITHUB_REPOSITORY"]}/commits/{source_head}/check-runs?per_page=100',
      ])
      plan_path=profile_output/'cargo-package-profile-plan.json'
      results_path=profile_output/'cargo-package-profile-results.json'
      receipt_path=profile_output/'cargo-package-profile-receipt.json'
      envelope_path=profile_output/'cargo-package-profile-envelope.json'
      self.assertTrue(plan_path.is_file())
      self.assertTrue(results_path.is_file())
      self.assertTrue(receipt_path.is_file())
      self.assertTrue(envelope_path.is_file())
      plan=json.loads(plan_path.read_text(encoding='utf-8'))
      results=json.loads(results_path.read_text(encoding='utf-8'))
      receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
      envelope=json.loads(envelope_path.read_text(encoding='utf-8'))
      self.assertEqual(plan['integration_base'],integration_base)
      self.assertEqual(plan['source_scope_base'],scope_base)
      self.assertEqual(plan['source_head'],source_head)
      self.assertEqual(plan['tested_tree'],tested_tree)
      self.assertEqual(plan['changed_packages'],[])
      self.assertEqual(plan['selected_items'],[])
      self.assertEqual(plan['items'],[])
      self.assertEqual(plan['execution_disposition'],'legacy_required_coverage')
      self.assertIs(plan['disposition_validated'],True)
      self.assertEqual(results,[])
      self.assertEqual(receipt['status'],'passed')
      self.assertEqual(receipt['execution_disposition'],'legacy_required_coverage')
      self.assertEqual(receipt['integration_base'],integration_base)
      self.assertEqual(receipt['source_head'],source_head)
      self.assertEqual(receipt['tested_tree'],tested_tree)
      self.assertEqual(envelope['task_uid'],fixture_uid)
      self.assertEqual(envelope['pr_number'],int(fixture_pr))
      self.assertEqual(envelope['integration_base'],integration_base)
      self.assertEqual(envelope['source_head'],source_head)
      self.assertEqual(envelope['tested_tree'],tested_tree)

      trusted_authority=temp/'trusted-cargo-profile-authority'
      self.assertEqual((candidate/'.pm/cargo-package-scope-policy.json').read_bytes(),trusted_blobs['.pm/cargo-package-scope-policy.json'])
      self.assertEqual((trusted_authority/'pm/workflow-impact-projection.py').read_bytes(),trusted_blobs['scripts/pm/workflow-impact-projection.py'])
      self.assertEqual((trusted_authority/'cargo_package_profile_planner.py').read_bytes(),trusted_blobs['scripts/pm/cargo_package_profile_planner.py'])
      self.assertEqual((trusted_authority/'cargo_package_profile_driver.py').read_bytes(),trusted_blobs['scripts/pm/cargo_package_profile_driver.py'])
      self.assertEqual((temp/'trusted-check-cargo-package-scope').read_bytes(),trusted_blobs['scripts/pm/check-cargo-package-scope'])
     else:
      env.update({'OASIS7_CARGO_SCOPE_BASE':'','OASIS7_CARGO_SCOPE_HEAD':'',
                  'OASIS7_CARGO_PROFILE_PLANNER':'','OASIS7_CARGO_PROFILE_DRIVER':''})
      result=subprocess.run(['bash','-euo','pipefail','-c',command],cwd=candidate,env=env,text=True,capture_output=True)
      self.assertEqual(result.returncode,0,result.stdout+result.stderr)
      self.assertEqual(marker.read_text(encoding='utf-8'),'candidate')
      self.assertFalse(profile_output.exists())

 def test_integration_dispatch_fails_closed_without_trusted_profile_authority(self):
  with tempfile.TemporaryDirectory() as tmp:
   temp=Path(tmp);candidate=temp/'candidate';scripts=candidate/'scripts';scripts.mkdir(parents=True)
   command=required_test_tier_command(HERE.parents[1])
   observed=temp/'unexpected-side-effect'
   (scripts/'doc-governance-check.sh').write_text('#!/usr/bin/env bash\nprintf invoked > "$OBSERVED"\n')
   (scripts/'doc-governance-check.sh').chmod(0o755)
   env={**os.environ,'RUNNER_TEMP':str(temp),'GITHUB_WORKSPACE':str(candidate),
        'GITHUB_EVENT_NAME':'workflow_dispatch','INTEGRATION_MODE':'integration_revalidation',
        'OBSERVED':str(observed),'INTEGRATION_WORKTREE':'',
        'OASIS7_CARGO_SCOPE_BASE':'','OASIS7_CARGO_SCOPE_HEAD':'',
        'OASIS7_CARGO_PROFILE_PLANNER':'','OASIS7_CARGO_PROFILE_DRIVER':''}
   result=subprocess.run(['bash','-euo','pipefail','-c',command],cwd=candidate,env=env,text=True,capture_output=True)
   self.assertEqual(result.returncode,1,result.stdout+result.stderr)
   self.assertIn('trusted Cargo package profile authority is unavailable',result.stderr)
   self.assertFalse((candidate/'output/cargo-package-profile').exists())
   self.assertFalse(observed.exists())

 def test_integration_freezes_sourced_preflight_before_checkout(self):
  workflow=(HERE.parents[1]/'.github/workflows/rust.yml').read_text()
  before=workflow.split('integration_ci.py" prepare',1)[0]
  self.assertRegex(before,r'cp[^\n]*scripts/viewer-dependency-preflight\.sh[^\n]*integration-planner')

 def test_projection_consumer_uses_trusted_driver_planner_and_keeps_exact_binding(self):
  driver=(HERE.parents[1]/'scripts/ci-tests.sh').read_text()
  consumer=driver.split('run_workflow_impact_projection_consumer() {',1)[1].split('\n}',1)[0]
  self.assertIn('trusted_planner_root = Path(sys.argv[3]).resolve()',consumer)
  self.assertIn('str(trusted_planner_root / "plan-rust-required-scope.py")',consumer)
  self.assertNotIn('root / "scripts" / "plan-rust-required-scope.sh"',consumer)
  for binding in (
   '"--task-uid", projection["task_uid"]',
   '"--head-ref", projection["source_head_oid"]',
   '"--scope-base-oid", projection["scope_base_oid"]',
   'planner.extend(("--changed-path", path))',
   'planner_fields.get("impact_projection_digest") != expected_digest',
   'planner_fields.get("impact_projection_status") != "verified"',
  ):
   self.assertIn(binding,consumer)
  task_uid='task_'+'1'*32
  source_head='a'*40
  scope_base='b'*40
  projection_digest='sha256:'+'c'*64
  changed_path='scripts/ci-required-scope.v2.json'
  with tempfile.TemporaryDirectory(prefix='oasis7-trusted-integration-planner-') as temporary:
   root=Path(temporary)
   candidate=root/'candidate'
   trusted=root/'integration-planner'
   candidate_scripts=candidate/'scripts'
   candidate_pm=candidate_scripts/'pm'
   trusted.mkdir()
   candidate_pm.mkdir(parents=True)
   (candidate_scripts/'ci-required-scope.v2.json').write_text('versioned-candidate-config\n')
   projection={
    'task_uid':task_uid,
    'source_head_oid':source_head,
    'scope_base_oid':scope_base,
    'changed_paths':[changed_path],
    'change_class':'workflow-doc',
    'manual_roles':[],
    'domain_role':None,
    'verification_affected':False,
    'projection_digest':projection_digest,
   }
   projection_path=candidate/'projection.json'
   projection_path.write_text(json.dumps(projection),encoding='utf-8')
   (candidate_pm/'workflow-impact-projection.py').write_text(textwrap.dedent('''\
    import json
    from pathlib import Path
    def load_verified_projection(path, *, repo_root):
        value = json.loads(Path(path).read_text(encoding="utf-8"))
        if not (Path(repo_root) / "scripts/ci-required-scope.v2.json").is_file():
            raise ValueError("candidate source config is missing")
        return value
   '''),encoding='utf-8')
   selector_script=candidate_pm/'review-role-selector.py'
   selector_script.write_text(textwrap.dedent(f'''\
    #!/usr/bin/env python3
    import json
    import sys
    args = sys.argv[1:]
    def value(name): return args[args.index(name) + 1]
    expected = {{
        "--task-uid": {task_uid!r},
        "--source-head-oid": {source_head!r},
        "--scope-base-oid": {scope_base!r},
    }}
    if any(value(key) != wanted for key, wanted in expected.items()):
        raise SystemExit("review selector identity changed")
    print(json.dumps({{"impact_projection_digest": {projection_digest!r}, "impact_projection_status": "verified"}}))
   '''),encoding='utf-8')
   selector_script.chmod(0o755)
   candidate_runner=candidate_scripts/'plan-rust-required-scope.sh'
   candidate_runner.write_text('#!/bin/sh\necho candidate-planner-used > "$CANDIDATE_PLANNER_MARKER"\nexit 91\n')
   candidate_runner.chmod(0o755)
   (trusted/'ci-required-scope.v2.json').write_text('trusted-legacy-base-config\n')
   trusted_marker=root/'trusted-planner-used'
   (trusted/'plan-rust-required-scope.py').write_text(textwrap.dedent(f'''\
    import os
    import sys
    from pathlib import Path
    args = sys.argv[1:]
    def value(name): return args[args.index(name) + 1]
    expected = {{
        "--event-name": "pull_request",
        "--task-uid": {task_uid!r},
        "--head-ref": {source_head!r},
        "--scope-base-oid": {scope_base!r},
        "--changed-path": {changed_path!r},
    }}
    if any(value(key) != wanted for key, wanted in expected.items()):
        raise SystemExit("trusted planner identity or source path changed")
    if Path(__file__).with_name("ci-required-scope.v2.json").read_text() != "trusted-legacy-base-config\\n":
        raise SystemExit("planner did not use trusted base config")
    if (Path.cwd() / "scripts/ci-required-scope.v2.json").read_text() != "versioned-candidate-config\\n":
        raise SystemExit("candidate source config was not preserved")
    Path(os.environ["TRUSTED_PLANNER_MARKER"]).write_text("trusted\\n")
    print("impact_projection_digest={projection_digest}")
    print("impact_projection_status=verified")
   '''),encoding='utf-8')
   start=driver.index('run_workflow_impact_projection_consumer() {')
   end=driver.index('\n}\n\nproduct_doc_range()',start)+2
   harness=root/'run-consumer.sh'
   harness.write_text(
    'set -euo pipefail\n'
    'driver_dir="$TRUSTED_PLANNER_ROOT"\n'
    'repo_root="$CANDIDATE_ROOT"\n'
    'impact_projection="$PROJECTION_PATH"\n'
    'run() { local executable="$1"; shift; "$executable" "$@"; }\n'
    + driver[start:end]
    + '\nrun_workflow_impact_projection_consumer\n',
    encoding='utf-8',
   )
   env=os.environ.copy()
   env.update({
    'CANDIDATE_ROOT':str(candidate),
    'CANDIDATE_PLANNER_MARKER':str(root/'candidate-planner-used'),
    'PROJECTION_PATH':str(projection_path),
    'TRUSTED_PLANNER_MARKER':str(trusted_marker),
    'TRUSTED_PLANNER_ROOT':str(trusted),
   })
   result=subprocess.run(['bash',str(harness)],cwd=candidate,env=env,text=True,capture_output=True)
   self.assertEqual(result.returncode,0,result.stdout+result.stderr)
   self.assertEqual(trusted_marker.read_text(),'trusted\n')
   self.assertFalse((root/'candidate-planner-used').exists())

 def test_required_gate_registers_review_plan_suite(self):
  driver=(HERE.parents[1]/'scripts/ci-tests.sh').read_text()
  def function_body(name):
   return driver.split(name+'() {',1)[1].split('\n}',1)[0]
  workflow_operational=function_body('run_workflow_governance_operational_contract_tests')
  self.assertIn('run python3 ./scripts/pm/review-plan.test.py',workflow_operational)
  required=function_body('run_required_gate_capability_contracts')
  self.assertIn('OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS run_workflow_governance_contract_tests',required)
  legacy=function_body('run_legacy_mixed_operational_contract_tests')
  self.assertIn('run_workflow_governance_operational_contract_tests',legacy)
  full_capabilities=function_body('run_all_required_gate_capability_contract_tests')
  self.assertIn('run_workflow_governance_contract_tests',full_capabilities)
  for full_tier in ('run_full_core_tier_tests','run_full_support_tier_tests','run_full_required_superset'):
   self.assertIn('run_all_required_gate_capability_contract_tests',function_body(full_tier))

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

class FirstActivationHistoryTests(unittest.TestCase):
 # Isolated synthetic identities; workflow bytes retain the actual producer contract.
 SOURCE_LIMIT = 1024 * 1024
 PROJECTION = "oasis7-ci|${{ github.event_name }}|${{ inputs.run_mode }}|${{ inputs.task_uid }}|${{ inputs.pr_number }}|${{ inputs.integration_base }}|${{ inputs.expected_head }}${{ inputs.request_key != '' && format('|{0}', inputs.request_key) || '' }}"
 WORKFLOW_SOURCE = ('name: Rust\nrun-name: ' + PROJECTION + '\non:\n  workflow_dispatch:\n    inputs:\n      run_mode:\n        type: choice\n        options:\n          - integration_revalidation\n          - first_activation_validation_only\n      pr_number:\n        required: false\n')

 def setUp(self):
  spec=importlib.util.spec_from_file_location('first_activation_history',HERE/'integration_ci.py')
  self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)
  self.uid='task_'+'1'*32;self.base='a'*40;self.head='b'*40
  self.producer='c'*40;self.reads=[]
  self.tree_ids=['1'*40,'2'*40,'3'*40]

 def row(self,n,validation=False,uid=None):
  mode='first_activation_validation_only' if validation else 'integration_revalidation'
  return {'id':n,'run_attempt':1,'created_at':'2026-09-26T00:00:00Z',
   'event':'workflow_dispatch','path':self.api.WORKFLOW,'head_sha':self.producer if validation else self.base,
   'head_branch':'candidate-producer' if validation else 'main','repository':{'full_name':'owner/repo'},
   'status':'completed','conclusion':'success',
   'display_title':f'oasis7-ci|workflow_dispatch|{mode}|{uid or self.uid}|'+('' if validation else '7')+f'|{self.base}|{self.head}'}

 def response(self,source=None):
  raw=(self.WORKFLOW_SOURCE if source is None else source).encode()
  return {'type':'file','path':self.api.WORKFLOW,'encoding':'base64','size':len(raw),
   'sha':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),
   'content':'\n'.join(textwrap.wrap(base64.b64encode(raw).decode(),60))+'\n'}

 def select(self,rows,response=None,page_size=100,git_overrides=None,leaf_mode='100644'):
  git_overrides=git_overrides or {}
  tree_chains={r['head_sha']:(self.tree_ids if r['head_sha']==self.producer else
   [hashlib.sha1((r['head_sha']+str(i)).encode()).hexdigest() for i in range(3)]) for r in rows}
  def contents(commit):
   if isinstance(response,Exception):raise response
   path=f'repos/owner/repo/contents/{self.api.WORKFLOW}?ref={commit}'
   return response(path) if callable(response) else self.response() if response is None else response
  def blob(commit):
   # Bind the tree to the actual case bytes, not the default valid workflow.
   # Wrong declared Contents SHA and explicit wrong tree overrides stay negative.
   try:
    raw=base64.b64decode(''.join(contents(commit)['content'].splitlines()),validate=True)
    return hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
   except (ValueError,TypeError,KeyError,AttributeError,OSError,subprocess.TimeoutExpired):
    return self.response()['sha']
  def read(*args):
   path=args[-1];self.reads.append(path)
   if '/runs?' in path:
    page=int(path.rsplit('page=',1)[1]);return {'workflow_runs':rows[(page-1)*page_size:page*page_size]}
   if '/git/commits/' in path:
    oid=path.rsplit('/',1)[1]
    self.assertIn(oid,[r['head_sha'] for r in rows])
    value={'sha':oid,'tree':{'sha':tree_chains[oid][0]}}
   elif '/git/trees/' in path:
    oid=path.rsplit('/',1)[1]
    matches=[(commit,chain) for commit,chain in tree_chains.items() if oid in chain]
    self.assertEqual(len(matches),1);commit,chain=matches[0];i=chain.index(oid)
    entry=({'path':['.github','workflows'][i],'mode':'040000','type':'tree','sha':chain[i+1]} if i<2 else
     {'path':'rust.yml','mode':leaf_mode,'type':'blob','sha':blob(commit)})
    value={'sha':oid,'truncated':False,'tree':[entry]}
   else:
    self.assertIn(path,[f'repos/owner/repo/contents/{self.api.WORKFLOW}?ref={r["head_sha"]}' for r in rows])
    return contents(path.rsplit('ref=',1)[1])
   if path in git_overrides:
    value=git_overrides[path]
    if isinstance(value,Exception):raise value
   return value
  with patch.object(self.api,'gh',side_effect=read),patch.object(self.api,'_historical_json',side_effect=lambda path,budget:read(path),create=True),patch.object(self.api,'DISCOVERY_PAGE_SIZE',page_size):
   return self.api.current_request('owner/repo',self.uid,7,self.base,self.head,'main')

 def test_actual_empty_pr_producer_preserves_own_current_request(self):
  own=self.row(10);history=self.row(20,True,uid='task_'+'2'*32)
  for rows in ([own,history],[history,own]):
   with self.subTest(order=[r['id'] for r in rows]):
    self.reads=[];self.assertEqual(self.select(rows)['id'],10)
    self.assertIn(f'repos/owner/repo/contents/{self.api.WORKFLOW}?ref={self.producer}',self.reads)

 def test_same_uid_and_all_validation_outcomes_are_ineligible(self):
  for status,conclusion in [('completed','success'),('completed','failure'),('queued',None),('completed','cancelled')]:
   with self.subTest(status=status,conclusion=conclusion):
    row=self.row(20,True);row.update(status=status,conclusion=conclusion)
    self.assertIsNone(self.select([row]))

 def test_complete_pages_and_per_commit_proof_cache(self):
  rows=[self.row(20,True),self.row(21,True),self.row(10)]
  self.assertEqual(self.select(rows,page_size=1)['id'],10)
  self.assertTrue(any('page=4' in p for p in self.reads))
  self.assertEqual(sum('/contents/' in p for p in self.reads),1)

 def test_producer_proof_is_never_reused_across_commits(self):
  first=self.row(20,True);second={**self.row(21,True),'head_sha':'d'*40}
  def proof(path):
   return self.response() if path.endswith(self.producer) else self.response('name: Rust\n')
  with self.assertRaises(ValueError):self.select([first,second,self.row(10)],proof)
  self.assertTrue(any(p.endswith('d'*40) for p in self.reads))

 def test_wrapped_base64_requires_exact_commit_tree_regular_leaf(self):
  for mode in ['100644','100755']:
   with self.subTest(mode=mode):
    self.reads=[]
    self.assertEqual(self.select([self.row(20,True),self.row(10)],leaf_mode=mode)['id'],10)
    self.assertIn(f'repos/owner/repo/git/commits/{self.producer}',self.reads)
    for oid in self.tree_ids:self.assertIn(f'repos/owner/repo/git/trees/{oid}',self.reads)
    self.assertFalse(any('recursive=' in path for path in self.reads))

 def test_official_contents_symlink_dereference_cannot_prove_regular_file(self):
  # Official Contents may return type=file and target bytes for a symlink.
  for mode,kind in [('120000','blob'),('160000','commit'),('040000','tree')]:
   with self.subTest(mode=mode):
    path=f'repos/owner/repo/git/trees/{self.tree_ids[2]}'
    value={'sha':self.tree_ids[2],'truncated':False,'tree':[{'path':'rust.yml','mode':mode,'type':kind,'sha':self.response()['sha']}]}
    with self.assertRaises(ValueError):self.select([self.row(20,True)],git_overrides={path:value})

 def test_tree_chain_metadata_and_read_uncertainty_fail_closed(self):
  commit_path=f'repos/owner/repo/git/commits/{self.producer}'
  tree_path=f'repos/owner/repo/git/trees/{self.tree_ids[0]}'
  leaf_path=f'repos/owner/repo/git/trees/{self.tree_ids[2]}'
  root={'sha':self.tree_ids[0],'truncated':False,'tree':[{'path':'.github','mode':'040000','type':'tree','sha':self.tree_ids[1]}]}
  leaf={'sha':self.tree_ids[2],'truncated':False,'tree':[{'path':'rust.yml','mode':'100644','type':'blob','sha':self.response()['sha']}]}
  cases=[(commit_path,{'sha':'f'*40,'tree':{'sha':self.tree_ids[0]}}),
   (commit_path,{'sha':self.producer,'tree':None}),
   (commit_path,{'sha':self.producer,'tree':{'sha':'invalid'}}),
   (commit_path,None),
   (tree_path,{**root,'sha':'f'*40}),(tree_path,{**root,'truncated':True}),
   (tree_path,{**root,'truncated':None}),(tree_path,{**root,'tree':[]}),
   (tree_path,{**root,'tree':root['tree']*2}),(tree_path,{**root,'tree':None}),
   (tree_path,{**root,'tree':[None]}),
   (tree_path,{**root,'tree':[{**root['tree'][0],'sha':'invalid'}]}),
   (tree_path,{**root,'tree':[{**root['tree'][0],'type':'blob'}]}),
   (tree_path,{**root,'tree':[{**root['tree'][0],'mode':'120000'}]}),
   (tree_path,{**root,'tree':[{**root['tree'][0],'path':'.github/workflows'}]}),
   (leaf_path,{**leaf,'tree':[{**leaf['tree'][0],'sha':'f'*40}]}),
   (leaf_path,{**leaf,'tree':[{**leaf['tree'][0],'path':'other.yml'}]}),
   (leaf_path,{**leaf,'tree':[{**leaf['tree'][0],'mode':'100644','type':'tree'}]}),
   (commit_path,OSError('commit read unavailable')),(tree_path,subprocess.TimeoutExpired('gh',30))]
  for i in [1,2]:
   path=f'repos/owner/repo/git/trees/{self.tree_ids[i]}'
   entry=({'path':'workflows','mode':'040000','type':'tree','sha':self.tree_ids[2]} if i==1 else leaf['tree'][0])
   cases.append((path,{'sha':'f'*40,'truncated':False,'tree':[entry]}))
  for path,value in cases:
   with self.subTest(path=path,value_type=type(value).__name__),self.assertRaises(ValueError):
    self.select([self.row(20,True)],git_overrides={path:value})

 def test_malformed_titles_remain_blocking(self):
  row=self.row(20,True);parts=row['display_title'].split('|')
  variants=[]
  for index,value in [(0,'alias'),(1,'push'),(2,'first_activation_validation_only_typo'),(3,'task-invalid'),(4,'7'),(5,'A'*40),(6,'not-an-oid')]:
   changed=parts.copy();changed[index]=value;variants.append('|'.join(changed))
  variants += ['|'.join(parts[:-1]),'|'.join(parts+['']), '|'.join(parts+['sha256:'+'d'*64])]
  for title in variants:
   with self.subTest(title=title),self.assertRaises(ValueError):
    self.select([{**row,'display_title':title}])

 def test_workflow_proof_and_finite_source_bound_fail_closed(self):
  valid=self.response()
  variants=[{**valid,'type':'symlink'},{**valid,'path':'wrong.yml'},{**valid,'encoding':'none'},
   {**valid,'content':None},{**valid,'content':'%%%invalid-base64%%%'},{**valid,'sha':'d'*40},
   {**valid,'content':valid['content'].rstrip()[:-1]},
   {**valid,'size':True},{**valid,'size':len(self.WORKFLOW_SOURCE.encode())+1},
   self.response(self.WORKFLOW_SOURCE.replace(self.PROJECTION,'integration_revalidation')),
   self.response(self.WORKFLOW_SOURCE.replace('          - first_activation_validation_only','          # - first_activation_validation_only')),
   self.response('# first_activation_validation_only\nrun-name: '+self.PROJECTION+'\n'),
   self.response(self.WORKFLOW_SOURCE+'#'+('x'*self.SOURCE_LIMIT)),
   OSError('producer read unavailable'),subprocess.TimeoutExpired('gh',30)]
  invalid_utf8=b'\xff';variants.append({**valid,'content':base64.b64encode(invalid_utf8).decode(),
   'size':1,'sha':hashlib.sha1(b'blob 1\0'+invalid_utf8).hexdigest()})
  for response in variants:
   with self.subTest(response_kind=type(response).__name__),self.assertRaises(ValueError):
    self.select([self.row(20,True)],response)

 def test_wrong_row_provenance_and_overlap_fail_closed(self):
  row=self.row(20,True)
  for field,value in [('event','push'),('path','wrong.yml'),('repository',None),('head_sha','invalid'),('head_branch',None)]:
   with self.subTest(field=field),self.assertRaises(ValueError):
    self.select([{**row,field:value}])
  with self.assertRaises(ValueError):self.select([row,row])

 def test_later_page_uncertainty_never_returns_found_candidate(self):
  bad=self.row(20,True);bad['display_title']=bad['display_title'].replace('first_activation_validation_only','unknown_mode')
  with self.assertRaises(ValueError):self.select([self.row(10),bad],page_size=1)
  with patch.object(self.api,'DISCOVERY_MAX_PAGES',1),self.assertRaises(ValueError):
   self.select([self.row(10)],page_size=1)

 def test_distinct_producer_commit_budget_is_not_proven_absence(self):
  rows=[{**self.row(n+20,True),'head_sha':f'{n+100:040x}'} for n in range(33)]
  with self.assertRaises(ValueError):self.select(rows+[self.row(10)])

class HistoricalTransportTests(unittest.TestCase):
 def setUp(self):
  spec=importlib.util.spec_from_file_location('historical_transport',HERE/'integration_ci.py')
  self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)
  self.fixture_processes={}

 def fake(self,body):
  directory=tempfile.TemporaryDirectory();self.addCleanup(directory.cleanup)
  root=Path(directory.name);program=root/'gh.py';pid=root/'pid'
  program.write_text('import os,sys,time\nfrom pathlib import Path\nPath(os.environ["TEST_HISTORICAL_PID"]).write_text(str(os.getpid()))\n'+body)
  script_bytes=program.read_bytes()
  self.assertFalse(pid.exists())
  real_popen=subprocess.Popen
  records=[];self.fixture_processes[pid]=records
  allowed_paths={'repos/owner/repo/git/commits/'+letter*40 for letter in ['a','b']}
  def launch(argv,*args,**kwargs):
   self.assertEqual(len(argv),3);self.assertEqual(argv[:2],['gh','api'])
   self.assertIn(argv[2],allowed_paths);self.assertEqual(args,())
   self.assertEqual(kwargs,{'stdin':subprocess.DEVNULL,'stdout':subprocess.PIPE,'stderr':subprocess.PIPE})
   self.assertEqual(program.read_bytes(),script_bytes)
   if pid.exists():
    previous=int(pid.read_text());self.assertEqual(previous,records[-1]['pid'])
    with self.assertRaises(ProcessLookupError):os.kill(previous,0)
    pid.unlink()
   launched=[sys.executable,'-S',str(program),*argv[1:]]
   child=real_popen(launched,*args,**kwargs)
   self.assertNotIn(child.pid,[record['pid'] for record in records])
   record={'requested_argv':list(argv),'launched_argv':launched,'pid':child.pid,'kwargs':kwargs}
   records.append(record);print(json.dumps({'historical_fixture_process':record},sort_keys=True))
   return child
  @contextmanager
  def fixture():
   # The production reader starts its clocks before this real launch adapter.
   # No child is prelaunched and no process, stream or readiness is synthesized.
   with patch.dict(os.environ,{'PATH':str(root),'TEST_HISTORICAL_PID':str(pid)}),patch.object(subprocess,'Popen',side_effect=launch):
    yield
  return fixture(),pid

 def assert_reaped(self,pid):
  self.assertTrue(pid.exists(),'controlled gh process must actually execute')
  child=int(pid.read_text())
  records=self.fixture_processes[pid]
  self.assertTrue(records);self.assertEqual(child,records[-1]['pid'])
  for record in records:
   with self.assertRaises(ProcessLookupError):os.kill(record['pid'],0)

 def test_frozen_numeric_reader_contract(self):
  expected={'HISTORICAL_SOURCE_MAX_BYTES':1024*1024,'HISTORICAL_RESPONSE_MAX_BYTES':2*1024*1024,
   'HISTORICAL_TOTAL_MAX_BYTES':16*1024*1024,'HISTORICAL_MAX_COMMITS':32,
   'HISTORICAL_MAX_CALLS':160,'HISTORICAL_CALL_TIMEOUT_SECONDS':15,
   'HISTORICAL_TOTAL_TIMEOUT_SECONDS':60}
  for name,value in expected.items():
   with self.subTest(name=name):self.assertEqual(getattr(self.api,name),value)

 def test_real_process_response_overflow_is_stopped_and_reaped(self):
  environment,pid=self.fake('sys.stdout.write("x"*4096);sys.stdout.flush();time.sleep(1)\n')
  started=time.monotonic()
  with environment,patch.object(self.api,'HISTORICAL_RESPONSE_MAX_BYTES',1024,create=True):
   with self.assertRaises(ValueError):
    self.api._historical_json('repos/owner/repo/git/commits/'+'a'*40,self.api._HistoricalReadBudget())
  self.assert_reaped(pid)
  self.assertLess(time.monotonic()-started,0.7,'overflow must stop the running producer promptly')

 def test_real_process_timeout_is_stopped_and_reaped(self):
  environment,pid=self.fake('time.sleep(1)\n')
  started=time.monotonic()
  with environment,patch.object(self.api,'HISTORICAL_CALL_TIMEOUT_SECONDS',0.1,create=True):
   with self.assertRaises(ValueError):
    self.api._historical_json('repos/owner/repo/git/commits/'+'a'*40,self.api._HistoricalReadBudget())
  self.assert_reaped(pid)
  self.assertLess(time.monotonic()-started,0.7,'timeout must stop the running producer promptly')

 def test_real_process_total_bytes_and_calls_are_bounded(self):
  environment,pid=self.fake('sys.stdout.write(\'{"padding":"\'+"x"*700+\'"}\')\n')
  with environment,patch.object(self.api,'HISTORICAL_TOTAL_MAX_BYTES',1024,create=True):
   budget=self.api._HistoricalReadBudget()
   self.api._historical_json('repos/owner/repo/git/commits/'+'a'*40,budget)
   with self.assertRaises(ValueError):self.api._historical_json('repos/owner/repo/git/commits/'+'b'*40,budget)
  self.assert_reaped(pid)
  environment,pid=self.fake('sys.stdout.write(\'{"ok":true}\')\n')
  with environment,patch.object(self.api,'HISTORICAL_MAX_CALLS',2,create=True):
   budget=self.api._HistoricalReadBudget()
   for _ in range(2):self.api._historical_json('repos/owner/repo/git/commits/'+'a'*40,budget)
   with self.assertRaises(ValueError):self.api._historical_json('repos/owner/repo/git/commits/'+'a'*40,budget)
  self.assert_reaped(pid)

 def test_real_process_aggregate_deadline_bounds_multiple_reads(self):
  environment,pid=self.fake('time.sleep(0.08);sys.stdout.write(\'{"ok":true}\')\n')
  with environment,patch.object(self.api,'HISTORICAL_TOTAL_TIMEOUT_SECONDS',0.15,create=True):
   budget=self.api._HistoricalReadBudget()
   self.api._historical_json('repos/owner/repo/git/commits/'+'a'*40,budget)
   with self.assertRaises(ValueError):self.api._historical_json('repos/owner/repo/git/commits/'+'b'*40,budget)
  self.assert_reaped(pid)

class CurrentRequestSelectionTests(unittest.TestCase):
 def setUp(self):
  spec=importlib.util.spec_from_file_location('integration_ci_request_selection_test',HERE/'integration_ci.py')
  self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)
  self.uid='task_'+'1'*32
  self.base='a'*40
  self.head='b'*40

 def run_row(self,run_id,request_key,*,created_at='2026-09-26T00:00:00Z',attempt=1):
  return {
   'id':run_id,'run_attempt':attempt,'created_at':created_at,
   'event':'workflow_dispatch','path':self.api.WORKFLOW,
   'head_sha':self.base,'head_branch':'main','repository':{'full_name':'owner/repo'},
   'display_title':(
    f'oasis7-ci|workflow_dispatch|integration_revalidation|{self.uid}|7|'
    f'{self.base}|{self.head}|{request_key}'
   ),
  }

 def unrelated_run_row(self,run_id,mode,*,request_key=None,request_uid=None,request_pr=8):
  other_uid=request_uid or 'task_'+'2'*32
  title=(
   f'oasis7-ci|workflow_dispatch|{mode}|{other_uid}|{request_pr}|'
   f'{"c"*40}|{"d"*40}'
  )
  if request_key is not None:
   title+=f'|{request_key}'
  return {
   'id':run_id,'run_attempt':1,'created_at':'2026-09-26T00:00:00Z',
   'event':'workflow_dispatch','path':self.api.WORKFLOW,
   'head_sha':self.base,'head_branch':'main','repository':{'full_name':'owner/repo'},
   'display_title':title,
  }

 def validation_only_row(self,run_id,*,uid=None,pr='7',base=None,head=None,validation_id=None):
  uid=self.uid if uid is None else uid
  base=self.base if base is None else base
  head=self.head if head is None else head
  validation_id='c'*64 if validation_id is None else validation_id
  return {
   'id':run_id,'run_attempt':1,'created_at':'2026-09-26T00:00:00Z',
   'event':'workflow_dispatch','path':self.api.WORKFLOW,
   'head_sha':self.base,'head_branch':'main','repository':{'full_name':'owner/repo'},
   'display_title':(
    f'oasis7-ci|workflow_dispatch|v1_reuse_validation_only|{uid}|{pr}|'
    f'{base}|{head}|{validation_id}'
   ),
  }

 def select(self,rows,request_key):
  with patch.object(self.api,'gh',return_value={'workflow_runs':rows}):
   return self.api.current_request(
    'owner/repo',self.uid,7,self.base,self.head,'main',request_key=request_key,
   )

 def test_remote_selection_orders_run_ids_numerically_on_timestamp_tie(self):
  rows=[self.run_row(99,'sha256:'+'1'*64),self.run_row(100,'sha256:'+'1'*64)]
  selected=self.select(rows,'sha256:'+'1'*64)
  self.assertEqual(100,selected['id'])

 def test_remote_selection_isolates_exact_request_key_and_attempt(self):
  key_a='sha256:'+'1'*64
  key_b='sha256:'+'2'*64
  rows=[
   self.run_row(99,key_a,attempt=99),
   self.run_row(100,key_b,attempt=100),
  ]
  selected_a=self.select(rows,key_a)
  selected_b=self.select(rows,key_b)
  self.assertEqual((99,99),(selected_a['id'],selected_a['run_attempt']))
  self.assertEqual((100,100),(selected_b['id'],selected_b['run_attempt']))

 def test_remote_selection_skips_unrelated_v1_reuse_validation_run(self):
  key='sha256:'+'1'*64
  validation_id='e'*64
  unrelated=self.unrelated_run_row(
   98,'v1_reuse_validation_only',request_key=validation_id,
  )
  integration=self.run_row(99,key)
  selected=self.select([unrelated,integration],key)
  self.assertEqual(99,selected['id'])
  self.assertIsNone(self.select([unrelated],key))

 def test_remote_selection_still_rejects_malformed_or_conflicting_v1_identity(self):
  key='sha256:'+'1'*64
  malformed_key=self.unrelated_run_row(
   98,'v1_reuse_validation_only',request_key='not-a-validation-id',
  )
  malformed_identity=self.unrelated_run_row(
   99,'v1_reuse_validation_only',request_key='e'*64,request_uid='task-invalid',
  )
  conflicting_identity=self.unrelated_run_row(
   100,'v1_reuse_validation_only',request_key='f'*64,request_uid=self.uid,
  )
  cases=(
   (malformed_key,'request key malformed'),
   (malformed_identity,'current request identity malformed'),
   (conflicting_identity,'request task/PR identity conflicts'),
  )
  for row,diagnostic in cases:
   with self.subTest(diagnostic=diagnostic):
    with self.assertRaisesRegex(ValueError,diagnostic):
     self.select([row],key)

 def test_remote_selection_still_rejects_malformed_or_ambiguous_current_identity(self):
  key='sha256:'+'1'*64
  malformed=self.run_row(99,key)
  parts=malformed['display_title'].split('|')
  parts[3]='task-invalid'
  malformed['display_title']='|'.join(parts)
  ambiguous=self.run_row(100,key)
  parts=ambiguous['display_title'].split('|')
  parts[4]='8'
  ambiguous['display_title']='|'.join(parts)
  cases=((malformed,'malformed'),(ambiguous,'conflicts'))
  for row,diagnostic in cases:
   with self.subTest(diagnostic=diagnostic):
    with self.assertRaisesRegex(ValueError,diagnostic):
     self.select([row],key)

 def test_remote_selection_rejects_timezone_naive_creation_time(self):
  row=self.run_row(99,'sha256:'+'1'*64,created_at='2026-09-26T00:00:00')
  with self.assertRaisesRegex(ValueError,'request time malformed'):
   self.select([row],'sha256:'+'1'*64)

 def test_valid_unrelated_validation_only_titles_with_bare_ids_are_skipped(self):
  rows=[
   self.validation_only_row(
    101,uid='task_'+'2'*32,pr='8',base='c'*40,head='d'*40,validation_id='e'*64,
   ),
   # Same Task/PR, but a different frozen base and source is another request.
   self.validation_only_row(
    102,base='f'*40,head='0'*40,validation_id='a'*64,
   ),
  ]
  self.assertIsNone(self.select(rows,'sha256:'+'1'*64))

 def test_validation_only_title_is_never_selected_as_integration_proof(self):
  validation=self.validation_only_row(101,validation_id='c'*64)
  integration=self.run_row(102,'sha256:'+'1'*64)
  selected=self.select([validation,integration],'sha256:'+'1'*64)
  self.assertEqual(102,selected['id'])

 def test_validation_only_rows_with_malformed_keys_or_identity_are_rejected(self):
  malformed_rows=[
   ('key has production prefix',self.validation_only_row(101,validation_id='sha256:'+'c'*64)),
   ('key has non-hex character',self.validation_only_row(102,validation_id='g'*64)),
   ('task uid malformed',self.validation_only_row(103,uid='task_bad')),
   ('PR is not canonical decimal',self.validation_only_row(104,pr='07')),
   ('base oid malformed',self.validation_only_row(105,base='G'*40)),
   ('source oid malformed',self.validation_only_row(106,head='not-an-oid')),
  ]
  for label,row in malformed_rows:
   with self.subTest(label=label),self.assertRaises(ValueError):
    self.select([row],'sha256:'+'1'*64)

 def test_validation_only_title_missing_validation_id_suffix_is_rejected(self):
  row=self.validation_only_row(107)
  row['display_title']=row['display_title'].rsplit('|',1)[0]
  with self.assertRaises(ValueError):
   self.select([row],'sha256:'+'1'*64)

 def test_validation_only_task_pr_conflicts_are_rejected(self):
  conflicting_task=self.validation_only_row(101,pr='8')
  conflicting_pr=self.validation_only_row(102,uid='task_'+'2'*32)
  for row in (conflicting_task,conflicting_pr):
   with self.subTest(title=row['display_title']),self.assertRaisesRegex(ValueError,'conflict'):
    self.select([row],'sha256:'+'1'*64)

class LocalTargetObservationTests(unittest.TestCase):
 def setUp(self):
  spec=importlib.util.spec_from_file_location('integration_ci_p1_test',HERE/'integration_ci.py')
  self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)

 def git(self,root,*args):
  return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()

 def source_proof_fixture(self):
  spec=importlib.util.spec_from_file_location(
   'ci_required_artifact_v2_test_for_local_target',HERE/'ci-required-artifact-v2.test.py',
  )
  module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  plan=module.valid_plan()
  artifact_id=1234
  policy_context={
   'schema':'oasis7-trusted-ci-reuse-policy-context/v1',
   'repository':plan['repository'],'workflow_ref':plan['workflow_ref'],
   'workflow_sha':plan['workflow_sha'],
   'effective_policy_identity':plan['effective_policy_identity'],
   'planner_inventory_authority':plan['planner_inventory_authority'],
  }
  artifact_helper=self.api._adjacent_module('ci_required_artifact_v2')
  result_name=artifact_helper.result_artifact_name(
   plan['workflow_run_id'],plan['run_attempt'],'unit-x',
  )
  plan_name=artifact_helper.plan_artifact_name(plan['workflow_run_id'],plan['run_attempt'])
  gate_job=module.gate_job(plan)
  trusted_attempt={
   'schema':'oasis7-ci-trusted-source-attempt/v1',
   'request_key':plan['request_key'],
   'workflow_run_id':plan['workflow_run_id'],'run_attempt':plan['run_attempt'],
   'check_app_id':plan['check_app_id'],'check_run_id':plan['check_run_id'],
   'job_id':plan['job_id'],'job_name':'required-gate',
   'plan_artifact_id':artifact_id,'plan_artifact_name':plan_name,
   'result_artifacts':[{'unit_id':'unit-x','artifact_id':1235,'name':result_name}],
  }
  proof={
   'request_key':plan['request_key'],'request_identity':plan['request_identity'],
   'integration_base_oid':plan['integration_base_oid'],
   'source_scope_oid':plan['source_scope_oid'],
   'workflow_run_id':plan['workflow_run_id'],'run_attempt':plan['run_attempt'],
   'check_app_id':plan['check_app_id'],'check_run_id':plan['check_run_id'],
   'trusted_policy_context':policy_context,
   'effective_policy_identity':plan['effective_policy_identity'],
   'planner_inventory_authority':plan['planner_inventory_authority'],
   'required_plan_v2_artifact_id':artifact_id,
   'required_plan_v2_artifact_name':plan_name,
   'required_plan_v2_payload':plan,
   'required_result_v2_artifacts':[{'artifact_id':1235,'name':result_name,'payload':{'unit_id':'unit-x'}}],
   'execution_jobs':[gate_job],
   'job_id':plan['job_id'],'job_name':'required-gate',
   'trusted_source_attempt':trusted_attempt,
   'trusted_planner_inventory':{
    **plan['planner_inventory_issuer'],
    'producer':{**plan['planner_inventory_issuer']['producer'],'artifact_id':artifact_id},
   },
  }
  return proof,plan

 def test_source_plan_context_and_journal_keep_immutable_b_h_s_and_request(self):
  proof,expected_plan=self.source_proof_fixture()
  plan,identity,context=self.api._validate_keyed_source_plan(
   'eng-cc/oasis7',expected_plan['task_uid'],expected_plan['pr_number'],proof,
  )
  self.assertEqual(expected_plan,plan)
  self.assertEqual(expected_plan['request_identity'],identity)
  self.assertEqual(expected_plan['planner_inventory_authority'],context['planner_inventory_authority'])
  request_helper=self.api._adjacent_module('integration_executor_contract')
  with tempfile.TemporaryDirectory() as temp:
   journal=Path(temp)
   record={
    'schema':request_helper.VALIDATION_REQUEST_SCHEMA,
    'request_key':proof['request_key'],'identity':identity,
    'integration_base_oid':proof['integration_base_oid'],
    'dispatch_attempts':1,'status':'observed',
    'run_id':expected_plan['workflow_run_id'],'run_attempt':1,
   }
   path=request_helper._request_path(journal,proof['request_key'])
   path.parent.mkdir(parents=True,exist_ok=True)
   path.write_bytes(request_helper.canonical_bytes(record)+b'\n')
   with patch.object(self.api,'git_common_dir',return_value=journal):
    self.api._validate_source_request_journal('eng-cc/oasis7',proof,plan,identity)
    for field,replacement in (
     ('integration_base_oid','9'*40),('run_id',999),('run_attempt',3),
    ):
     changed={**record,field:replacement}
     path.write_bytes(request_helper.canonical_bytes(changed)+b'\n')
     with self.subTest(field=field),patch.object(self.api,'git_common_dir',return_value=journal):
      with self.assertRaisesRegex(ValueError,'journal'):
       self.api._validate_source_request_journal('eng-cc/oasis7',proof,plan,identity)

 def test_source_attempt_binding_rejects_cross_attempt_and_artifact_locators(self):
  proof,plan=self.source_proof_fixture()
  self.assertEqual(proof['trusted_source_attempt'],self.api._validate_trusted_source_attempt(proof,plan))
  mutations=(
   ('run_attempt',lambda item:item.__setitem__('run_attempt',item['run_attempt']+1)),
   ('check_run_id',lambda item:item.__setitem__('check_run_id',item['check_run_id']+1)),
   ('plan_artifact_id',lambda item:item.__setitem__('plan_artifact_id',item['plan_artifact_id']+1)),
   ('result_artifacts',lambda item:item['result_artifacts'][0].__setitem__('artifact_id',9999)),
  )
  for field,mutate in mutations:
   changed=json.loads(json.dumps(proof))
   mutate(changed['trusted_source_attempt'])
   with self.subTest(field=field),self.assertRaisesRegex(ValueError,'trusted attempt'):
    self.api._validate_trusted_source_attempt(changed,plan)

 def test_local_target_reader_rejects_unbound_proof_before_live_reads(self):
  with patch.object(self.api,'identity') as read_pr, \
       patch.object(self.api,'current_request') as read_request, \
       patch.object(self.api,'verified_run') as read_run:
   with self.assertRaisesRegex(ValueError,'source required-plan v2 proof is incomplete'):
    self.api.trusted_local_target_inventory(
     'eng-cc/oasis7','task_'+'1'*32,7,{},
    )
   read_pr.assert_not_called()
   read_request.assert_not_called()
   read_run.assert_not_called()

 def remote_fixture(self,*,conflict=False):
  temp=tempfile.TemporaryDirectory()
  root=Path(temp.name)/'checkout';root.mkdir()
  remote=Path(temp.name)/'origin.git'
  subprocess.run(['git','init','--bare','--quiet',str(remote)],check=True)
  subprocess.run(['git','init','--quiet','-b','main',str(root)],check=True)
  self.git(root,'config','user.name','Local target test')
  self.git(root,'config','user.email','local-target@example.invalid')
  (root/'shared.txt').write_text('base\n',encoding='utf-8')
  self.git(root,'add','.');self.git(root,'commit','-qm','base')
  source_scope=self.git(root,'rev-parse','HEAD')
  self.git(root,'switch','-q','-c','source')
  if conflict:
   (root/'shared.txt').write_text('source\n',encoding='utf-8')
  else:
   (root/'source.txt').write_text('source\n',encoding='utf-8')
  self.git(root,'add','.');self.git(root,'commit','-qm','source')
  head=self.git(root,'rev-parse','HEAD')
  self.git(root,'remote','add','origin',str(remote))
  self.git(root,'push','-q','origin',f'{head}:refs/pull/7/head')
  self.git(root,'switch','-q','main')
  if conflict:
   (root/'shared.txt').write_text('target\n',encoding='utf-8')
  else:
   (root/'target.txt').write_text('target\n',encoding='utf-8')
  self.git(root,'add','.');self.git(root,'commit','-qm','target advance')
  target=self.git(root,'rev-parse','HEAD')
  self.git(root,'push','-q','origin','main')
  return temp,root,source_scope,head,target

 def test_exact_q_h_merge_uses_isolated_clean_worktrees_and_parent_order(self):
  temp,root,source_scope,head,target=self.remote_fixture()
  try:
   before=self.git(root,'rev-parse','HEAD')
   with self.api._local_target_worktrees(
       root,'main',7,target,head,source_scope,
   ) as value:
    repository=value['repository_root'];planner=value['planner_root'];checkout=value['target_root']
    merge_tree=self.git(repository,'merge-tree','--write-tree',target,head).splitlines()[0]
    parents=self.git(checkout,'rev-list','--parents','-n','1','HEAD').split()
    self.assertEqual([value['input_scope_commit_oid'],target,head],parents)
    self.assertEqual(merge_tree,value['input_scope_tree_oid'])
    self.assertEqual(target,self.git(planner,'rev-parse','HEAD'))
    self.assertEqual(value['input_scope_commit_oid'],self.git(checkout,'rev-parse','HEAD'))
    self.assertTrue((checkout/'source.txt').is_file())
    self.assertTrue((checkout/'target.txt').is_file())
    self.assertEqual('',self.git(planner,'status','--porcelain','--untracked-files=all'))
    self.assertEqual('',self.git(checkout,'status','--porcelain','--untracked-files=all'))
   self.assertEqual(before,self.git(root,'rev-parse','HEAD'))
   self.assertEqual('',self.git(root,'status','--porcelain','--untracked-files=all'))
  finally:
   temp.cleanup()

 def test_exact_q_h_merge_rejects_ref_drift_wrong_scope_and_conflicts(self):
  temp,root,source_scope,head,target=self.remote_fixture()
  try:
   with self.assertRaisesRegex(ValueError,'fetched default-branch or PR head differs'):
    with self.api._local_target_worktrees(root,'main',7,'f'*40,head,source_scope):
     self.fail('wrong Q unexpectedly accepted')
   with self.assertRaisesRegex(ValueError,'merge base differs'):
    with self.api._local_target_worktrees(root,'main',7,target,head,'e'*40):
     self.fail('wrong source scope unexpectedly accepted')
  finally:
   temp.cleanup()
  conflict_temp,conflict_root,conflict_scope,conflict_head,conflict_target=self.remote_fixture(conflict=True)
  try:
   with self.assertRaisesRegex(ValueError,'merge has conflicts'):
    with self.api._local_target_worktrees(
        conflict_root,'main',7,conflict_target,conflict_head,conflict_scope,
    ):
     self.fail('conflicting Q/H merge unexpectedly accepted')
  finally:
   conflict_temp.cleanup()

 def test_projection_marker_is_unique_canonical_and_duplicate_keys_are_rejected(self):
  value={'task_uid':'task_'+'1'*32}
  raw=json.dumps(value,separators=(',',':')).encode()
  encoded=base64.b64encode(raw).decode()
  body='PR context\n<!-- oasis7-impact-projection-b64: '+encoded+' -->\n'
  parsed_raw,parsed=self.api._projection_from_pr_body(body)
  self.assertEqual(raw,parsed_raw)
  self.assertEqual(value,parsed)
  with self.assertRaisesRegex(ValueError,'missing or ambiguous'):
   self.api._projection_from_pr_body(body+body)
  duplicate=base64.b64encode(b'{"task_uid":"one","task_uid":"two"}').decode()
  with self.assertRaisesRegex(ValueError,'malformed'):
   self.api._projection_from_pr_body('<!-- oasis7-impact-projection-b64: '+duplicate+' -->')


class ProvenanceTests(unittest.TestCase):
 def setUp(self):
  spec=importlib.util.spec_from_file_location('integration_ci',HERE/'integration_ci.py');self.api=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.api)
  self.policy_module=self.api._adjacent_module('ci_reuse_policy')
  self.policy_module.TRUSTED_EFFECTIVE_POLICY={'enabled_capabilities':[],'approved_executor_contract_digests':[],'check_app_id':15368}
  # The focused provenance tests below exercise C3 artifact validation with a
  # synthetic enabled policy.  The real W resolver is covered independently
  # by ci_reuse_policy.test.py; keep these fixtures explicit and test-local.
  self.api.trusted_policy_context_for_run=self.trusted_policy_context_for_run
  self.api.trusted_policy_context=self.trusted_policy_context
  self.base='a'*40;self.head='b'*40;self.uid='task_'+'c'*32
  self.pr={'state':'open','merged':False,'draft':True,'body':'Task: '+self.uid+'\nRefs #1','base':{'sha':self.base,'ref':'main','repo':{'full_name':'owner/repo'}},'head':{'sha':self.head,'repo':{'full_name':'owner/repo'}}}
  self.run={'run_attempt':1,'created_at':'2026-09-09T00:00:00Z','run_started_at':'2026-09-09T00:00:00Z','event':'workflow_dispatch','head_branch':'main','head_sha':self.base,'path':self.api.WORKFLOW,'status':'completed','conclusion':'success','repository':{'full_name':'owner/repo'},'check_suite_id':8}
  self.payload=dict(schema=self.api.ARTIFACT,repository='owner/repo',workflow_run_id=9,base_oid=self.base,head_oid=self.head,task_uid=self.uid,pr_number=12,workflow_sha=self.base,workflow_ref='owner/repo/.github/workflows/rust.yml@refs/heads/main',integration_mode='integration_revalidation',check_name='required-gate',scope_base_oid='d'*40,tested_tree_oid='e'*40,tested_commit_oid='f'*40)
  from integration_executor_contract import EXECUTOR_CONTRACT_PATHS
  self.executor_contents={path:('trusted fixture '+path).encode() for path in EXECUTOR_CONTRACT_PATHS}

 def historical_pr_api(self,*args):
  path=args[1]
  if path=='repos/owner/repo/pulls/12':return self.pr
  if path=='repos/owner/repo':return {'default_branch':'main'}
  if path=='repos/owner/repo/git/ref/heads/main':return {'object':{'sha':self.base}}
  raise AssertionError('unexpected GitHub read '+path)

 def test_historical_pr_base_does_not_reject_exact_current_target(self):
  self.pr['base']['sha']='d'*40
  with patch.object(self.api,'gh',side_effect=self.historical_pr_api):
   actual,branch=self.api.identity('owner/repo',self.uid,12,self.base,self.head)
  self.assertEqual(branch,'main')
  self.assertEqual(actual['head']['sha'],self.head)
  # Preserve historical PR metadata; resolving the target must not rewrite it.
  self.assertEqual(actual['base']['sha'],'d'*40)

 def test_historical_pr_base_cannot_authorize_stale_integration_target(self):
  historical='d'*40
  self.pr['base']['sha']=historical
  with patch.object(self.api,'gh',side_effect=self.historical_pr_api):
   with self.assertRaisesRegex(ValueError,'target|default.branch|moved'):
    self.api.identity('owner/repo',self.uid,12,historical,self.head)
 def trusted_policy_context(self,repository,branch,workflow_sha,default_branch_sha):
  import integration_executor_contract as request_contract
  policy=dict(self.policy_module.TRUSTED_EFFECTIVE_POLICY)
  return {
   'schema':'oasis7-trusted-ci-reuse-policy-context/v1',
   'repository':repository,
   'workflow_ref':f'{repository}/.github/workflows/rust.yml@refs/heads/{branch}',
   'workflow_sha':workflow_sha,
   'effective_policy':policy,
   'effective_policy_identity':{
    'schema':request_contract.EFFECTIVE_POLICY_IDENTITY_SCHEMA,
    'digest':request_contract.effective_policy_digest(policy),
   },
   'planner_inventory_authority':{
    'schema':'oasis7-planner-inventory-authority/v1',
    'repository':repository,
    'workflow_ref':f'{repository}/.github/workflows/rust.yml@refs/heads/{branch}',
    'planner_authority_oid':workflow_sha,
    'planner_config_sha256':'sha256:'+'a'*64,
   },
  }
 def trusted_policy_context_for_run(self,repository,branch,run):
  workflow_sha=run.get('head_sha')
  return self.trusted_policy_context(repository,branch,workflow_sha,workflow_sha)
 def policy(self,approved):
  return {
   'enabled_capabilities':['input-scope-reuse/v1'],
   'approved_executor_contract_digests':[approved],
   'check_app_id':42,
  }
 def keyed_request_identity(self,executor_digest,policy=None):
  from integration_executor_contract import effective_policy_digest
  policy=policy or self.policy(executor_digest)
  return {
   'repository':'owner/repo','task_uid':self.uid,'pr_number':12,'bootstrap_epoch':1,
   'source_head_oid':self.head,'publication_id':'publication-1',
   'source_projection_digest':'sha256:'+'1'*64,'unit_ids':['required-gate'],
   'input_fingerprints':{'required-gate':'sha256:'+'2'*64},
   'executor_contract_digest':executor_digest,
   'effective_policy_digest':effective_policy_digest(policy),
   'purpose':'integration_revalidation','applicability_mode':'input_scoped',
   'snapshot_target_oid':None,
  }
 def bind_keyed_payload(self,executor_digest,run_head,*,policy=None,workflow_sha=None,attempt=1):
  import integration_executor_contract as request_contract
  policy=policy or self.policy(executor_digest)
  self.policy_module.TRUSTED_EFFECTIVE_POLICY=dict(policy)
  request_identity=self.keyed_request_identity(executor_digest,policy)
  request_key=request_contract.validation_request_key(request_identity)
  request=request_contract.validation_request_envelope({
   'schema':request_contract.VALIDATION_REQUEST_SCHEMA,'request_key':request_key,
   'identity':request_identity,'integration_base_oid':self.base,
  })
  self.run.update(head_sha=run_head,display_title=(
   f'oasis7-ci|workflow_dispatch|integration_revalidation|{self.uid}|12|'
   f'{self.base}|{self.head}|{request_key}'
  ))
  self.payload.update(
   workflow_sha=workflow_sha or run_head,workflow_run_head_sha=run_head,
   workflow_run_attempt=attempt,request_key=request_key,
   validation_request=request,executor_contract_digest=executor_digest,
  )
  return request_key,request_identity,policy
 def test_keyed_payload_decoders_require_canonical_v2_envelopes(self):
  import integration_executor_contract as request_contract
  from integration_executor_contract import canonical_bytes
  executor_digest=request_contract.executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(executor_digest,'6'*40)
  envelope={
   'schema':request_contract.VALIDATION_REQUEST_SCHEMA,'request_key':request_key,
   'identity':request_identity,'integration_base_oid':self.base,
  }
  encoded=base64.b64encode(canonical_bytes(envelope)).decode('ascii')
  self.assertEqual(envelope,self.api.decode_validation_request(encoded,request_key))
  pretty=base64.b64encode(json.dumps(envelope,indent=2).encode()).decode('ascii')
  with self.assertRaisesRegex(ValueError,'not canonical'):
   self.api.decode_validation_request(pretty,request_key)
  policy_encoded=base64.b64encode(canonical_bytes(effective_policy)).decode('ascii')
  self.assertEqual(effective_policy,self.api.decode_effective_policy(policy_encoded))
 def read(self,*args):
  path=args[-1]
  if path=='repos/owner/repo/git/ref/heads/main':return {'object':{'sha':self.base}}
  if '/workflows/rust.yml/runs?' in path:return {'workflow_runs':[{**self.run,'id':9,'display_title':f'oasis7-ci|workflow_dispatch|integration_revalidation|{self.uid}|12|{self.base}|{self.head}'}]}
  if '/contents/' in path:
   relative=path.split('/contents/',1)[1].split('?ref=',1)[0]
   if relative=='scripts/pm/ci_reuse_policy.py':
    trusted=(HERE/'ci_reuse_policy.py').read_bytes()
    return {'type':'file','path':relative,'encoding':'base64','content':base64.b64encode(trusted).decode()}
   if relative not in self.executor_contents:self.fail(path)
   return {'type':'file','path':relative,'encoding':'base64','content':base64.b64encode(self.executor_contents[relative]).decode()}
  if '/compare/' in path:return {'merge_base_commit':{'sha':self.base}}
  if path.endswith('/pulls/12'):return self.pr
  if path=='repos/owner/repo':return {'full_name':'owner/repo','default_branch':'main'}
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
 def test_attempt_execution_jobs_bind_run_attempt_and_exact_checks(self):
  jobs=[{
   'id':101,'run_id':9,'run_attempt':2,'name':'required-gate','status':'in_progress',
   'conclusion':None,'head_sha':self.base,'labels':['ubuntu-24.04'],
   'check_run_url':'https://api.github.com/repos/owner/repo/check-runs/201',
  }]
  def reader(*args):
   path=args[-1]
   if path=='repos/owner/repo/actions/runs/9':return {**self.run,'id':9,'run_attempt':2}
   if path=='repos/owner/repo/actions/runs/9/attempts/2/jobs?per_page=100&page=1':return {'jobs':jobs}
   if path=='repos/owner/repo/check-runs/201':return {
    'id':201,'name':'required-gate','app':{'id':42},'head_sha':self.base,
    'status':'in_progress','conclusion':None,
   }
   self.fail(path)
  with patch.object(self.api,'gh',side_effect=reader):
   proof=self.api.attempt_execution_jobs('owner/repo',9,2,self.base,42)
  self.assertEqual([{
   'workflow_run_id':9,'run_attempt':2,'job_id':101,'job_name':'required-gate',
   'check_name':'required-gate','check_app_id':42,'check_run_id':201,
   'head_sha':self.base,'status':'in_progress','conclusion':None,'labels':['ubuntu-24.04'],
  }],proof)
  with patch.object(self.api,'gh',side_effect=reader):
   with self.assertRaisesRegex(ValueError,'positive integer'):
    self.api.attempt_execution_jobs('owner/repo',9,True,self.base,42)
 def test_attempt_execution_jobs_reject_wrong_check_app_and_attempt(self):
  job={
   'id':101,'run_id':9,'run_attempt':2,'name':'required-gate','status':'in_progress',
   'conclusion':None,'head_sha':self.base,'labels':['ubuntu-24.04'],
   'check_run_url':'https://api.github.com/repos/owner/repo/check-runs/201',
  }
  def reader(*args):
   path=args[-1]
   if path=='repos/owner/repo/actions/runs/9':return {**self.run,'id':9,'run_attempt':2}
   if path.endswith('/attempts/2/jobs?per_page=100&page=1'):return {'jobs':[job]}
   if path=='repos/owner/repo/check-runs/201':return {
    'id':201,'name':'required-gate','app':{'id':43},'head_sha':self.base,
    'status':'in_progress','conclusion':None,
   }
   self.fail(path)
  with patch.object(self.api,'gh',side_effect=reader):
   with self.assertRaisesRegex(ValueError,'check-run identity mismatch'):
    self.api.attempt_execution_jobs('owner/repo',9,2,self.base,42)
  with patch.object(self.api,'gh',side_effect=reader):
   with self.assertRaisesRegex(ValueError,'attempt provenance mismatch'):
    self.api.attempt_execution_jobs('owner/repo',9,1,self.base,42)
 def test_attempt_execution_jobs_can_ignore_its_own_in_progress_result_job(self):
  gate={
   'id':101,'run_id':9,'run_attempt':2,'name':'required-gate','status':'completed',
   'conclusion':'success','head_sha':self.base,'labels':['ubuntu-24.04'],
   'check_run_url':'https://api.github.com/repos/owner/repo/check-runs/201',
  }
  current_result={
   'id':102,'run_id':9,'run_attempt':2,'name':'required-result-v2 (unit-x)',
   'status':'in_progress','conclusion':None,'head_sha':self.base,'labels':['ubuntu-24.04'],
   'check_run_url':None,
  }
  def reader(*args):
   path=args[-1]
   if path=='repos/owner/repo/actions/runs/9':return {**self.run,'id':9,'run_attempt':2}
   if path=='repos/owner/repo/actions/runs/9/attempts/2/jobs?per_page=100&page=1':
    return {'jobs':[gate,current_result]}
   if path=='repos/owner/repo/check-runs/201':return {
    'id':201,'name':'required-gate','app':{'id':42},'head_sha':self.base,
    'status':'completed','conclusion':'success',
   }
   self.fail(path)
  with patch.object(self.api,'gh',side_effect=reader):
   proof=self.api.attempt_execution_jobs(
    'owner/repo',9,2,self.base,42,require_completed=True,job_names={'required-gate'},
   )
  self.assertEqual(['required-gate'],[job['job_name'] for job in proof])
 def test_keyed_request_preflight_requires_key_in_authoritative_run_name(self):
  required=('run-name: '+self.api.KEYED_RUN_NAME+'\non:\n  workflow_dispatch:\n    inputs:\n'
            '      run_mode:\n        required: true\n        type: choice\n'
            '      task_uid:\n        required: false\n        type: string\n'
            '      pr_number:\n        required: false\n        type: string\n'
            '      integration_base:\n        required: false\n        type: string\n'
            '      expected_head:\n        required: false\n        type: string\n'
            '      request_key:\n        required: false\n        type: string\n'
            '      validation_request_b64:\n        required: false\n        type: string\n')
  missing_title=required.replace(self.api.KEYED_RUN_NAME,'oasis7-ci|${{ inputs.expected_head }}')
  key_only_title=required.replace(self.api.KEYED_RUN_NAME,'oasis7-ci|${{ inputs.request_key }}')
  self.assertTrue(self.api.keyed_request_workflow_ready(required))
  self.assertFalse(self.api.keyed_request_workflow_ready(missing_title))
  self.assertFalse(self.api.keyed_request_workflow_ready(key_only_title))
  self.assertFalse(self.api.keyed_request_workflow_ready(required+'\nrun-name: duplicate'))
  actual=(HERE.parents[1]/'.github/workflows/rust.yml').read_text(encoding='utf-8')
  self.assertTrue(self.api.keyed_request_workflow_ready(actual))
 def test_keyed_request_preflight_ignores_comment_only_input_declarations(self):
  workflow=('run-name: '+self.api.KEYED_RUN_NAME+'\non:\n  workflow_dispatch:\n    inputs:\n'
            '      run_mode:\n        required: true\n        type: choice\n'
            '      task_uid:\n        required: false\n        type: string\n'
            '      pr_number:\n        required: false\n        type: string\n'
            '      integration_base:\n        required: false\n        type: string\n'
            '      expected_head:\n        required: false\n        type: string\n# request_key:\n'
            '# validation_request_b64:\n# inputs.request_key\n')
  self.assertFalse(self.api.keyed_request_workflow_ready(workflow))
 def test_prepared_request_retry_keeps_journaled_base_after_main_advances(self):
  from integration_executor_contract import (
   EXECUTOR_CONTRACT_PATHS, effective_policy_digest, executor_contract_from_contents,
   reserve_validation_request, validation_request_key,
  )

  contract=executor_contract_from_contents({
   path:('trusted fixture '+path).encode() for path in EXECUTOR_CONTRACT_PATHS
  })
  effective_policy={
   'enabled_capabilities':['input-scope-reuse/v1'],
   'approved_executor_contract_digests':[contract['digest']],
   'check_app_id':42,
  }
  self.policy_module.TRUSTED_EFFECTIVE_POLICY=dict(effective_policy)
  projection_digest='sha256:'+'1'*64
  request_identity={
   'repository':'owner/repo',
   'task_uid':self.uid,
   'pr_number':12,
   'bootstrap_epoch':1,
   'source_head_oid':self.head,
   'publication_id':'publication-1',
   'source_projection_digest':projection_digest,
   'unit_ids':['required-gate'],
   'input_fingerprints':{'required-gate':'sha256:'+'2'*64},
   'executor_contract_digest':contract['digest'],
   'effective_policy_digest':effective_policy_digest(effective_policy),
   'purpose':'integration_revalidation',
   'applicability_mode':'input_scoped',
   'snapshot_target_oid':None,
  }
  request_key=validation_request_key(request_identity)
  original_base='3'*40
  advanced_base='4'*40
  workflow=(
   'run-name: '+self.api.KEYED_RUN_NAME+'\non:\n  workflow_dispatch:\n    inputs:\n'
   '      run_mode:\n        type: choice\n        required: true\n'
   '      task_uid:\n        type: string\n        required: false\n'
   '      pr_number:\n        type: string\n        required: false\n'
   '      integration_base:\n        type: string\n        required: false\n'
   '      expected_head:\n        type: string\n        required: false\n'
   '      request_key:\n        type: string\n        required: false\n'
   '      validation_request_b64:\n        type: string\n        required: false\n'
  )
  with tempfile.TemporaryDirectory() as directory:
   reserve_validation_request(directory,request_key,request_identity,original_base)
   projection_path=Path(directory)/'projection.json'
   projection_path.write_text(
    json.dumps({'projection_digest':projection_digest}),encoding='utf-8',
   )
   dispatches=[]

   def read_api(*args):
    path=args[-1]
    if path.endswith('/pulls/12'):
     return self.pr
    if path=='repos/owner/repo':
     return {'full_name':'owner/repo','default_branch':'main'}
    if '/actions/workflows/rust.yml/runs?' in path:
     return {'workflow_runs':[]}
    if path.startswith(f'repos/owner/repo/contents/{self.api.WORKFLOW}?'):
     return {
      'type':'file','path':self.api.WORKFLOW,'encoding':'base64',
      'content':base64.b64encode(workflow.encode()).decode(),
     }
    self.fail(path)

   with patch.object(self.api,'gh',side_effect=read_api), \
        patch.object(self.api,'default_branch_head',return_value=advanced_base), \
        patch.object(self.api,'github_executor_contract',return_value=contract), \
        patch.object(self.api.subprocess,'run',side_effect=lambda args,**_kwargs: dispatches.append(args)):
    result=self.api.dispatch_request(
     'owner/repo',self.uid,12,projection_path,request_identity,
     effective_policy,
     state_dir=directory,
    )

  self.assertEqual('pending',result['status'])
  self.assertEqual(original_base,result['base_oid'])
  self.assertEqual(1,len(dispatches))
  fields={}
  args=dispatches[0]
  for index,value in enumerate(args[:-1]):
   if value=='-f':
    name,field_value=args[index+1].split('=',1)
    fields[name]=field_value
  self.assertEqual(original_base,fields['integration_base'])
  payload=json.loads(base64.b64decode(fields['validation_request_b64']))
  self.assertEqual('oasis7-ci-validation-request/v2',payload['schema'])
  self.assertEqual(1,payload['identity']['bootstrap_epoch'])
  self.assertEqual(original_base,payload['integration_base_oid'])
  self.assertEqual(request_key,payload['request_key'])

 def test_keyed_dispatch_stays_disabled_without_an_authorized_capability(self):
  from integration_executor_contract import effective_policy_digest
  policy={
   'enabled_capabilities':[],
   'approved_executor_contract_digests':[],
   'check_app_id':15368,
  }
  self.assertTrue(effective_policy_digest(policy).startswith('sha256:'))
  with patch.object(self.api,'gh',side_effect=self.read), \
       patch.object(self.api,'default_branch_head',return_value=self.base), \
       self.assertRaisesRegex(ValueError,'is disabled'):
   self.api.dispatch_request('owner/repo',self.uid,12,'missing',{},policy)

 def test_new_default_workflow_run_authority_passes(self):
  check,proof=self.verify();self.assertEqual(check['id'],10);self.assertEqual(proof['head_oid'],self.head)
 def test_keyed_workflow_base_divergence_uses_v2_reader_after_approved_w_contract(self):
  from integration_executor_contract import executor_contract_from_contents
  workflow_sha='6'*40;run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(
   executor_digest,run_head,workflow_sha=workflow_sha,
  )
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  gate_job={
   'workflow_run_id':9,'run_attempt':1,'job_id':10,'job_name':'required-gate',
   'check_name':'required-gate','check_app_id':42,'check_run_id':10,
   'head_sha':run_head,'status':'completed','conclusion':'success','labels':['ubuntu-24.04'],
  }
  v2_proof={
   'required_plan_v2_artifact_id':31,'required_plan_v2_artifact_name':'oasis7-required-plan-v2-9-a1',
   'required_plan_v2_payload':{'schema':'oasis7-required-plan-v2'},
   'required_result_v2_artifacts':[], 'source_scope_oid':'d'*40,
   'workflow_run_id':9,'run_attempt':1,'check_app_id':42,'check_run_id':10,
   'job_id':10,'job_name':'required-gate','execution_jobs':[gate_job],
   'trusted_planner_inventory':{'producer':{'artifact_id':31}},
   'trusted_source_attempt':{
    'schema':'oasis7-ci-trusted-source-attempt/v1','request_key':request_key,
    'workflow_run_id':9,'run_attempt':1,'check_app_id':42,'check_run_id':10,
    'job_id':10,'job_name':'required-gate','plan_artifact_id':31,
    'plan_artifact_name':'oasis7-required-plan-v2-9-a1','result_artifacts':[],
   },
  }
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()), \
       patch.object(self.api,'attempt_execution_jobs',return_value=[gate_job]), \
       patch.object(self.api,'read_keyed_v2_evidence',return_value=v2_proof):
   check,proof=self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[executor_digest])
  self.assertEqual(workflow_sha,proof['workflow_sha'])
  self.assertEqual(self.base,proof['base_oid'])
  self.assertEqual(run_head,proof['workflow_run_head_sha'])
  self.assertEqual(run_head,check['head_sha'])
  self.assertEqual(11,proof['plan_artifact_id'])
  self.assertEqual(31,proof['required_plan_v2_artifact_id'])
  self.assertEqual(v2_proof['trusted_source_attempt'],proof['trusted_source_attempt'])
 def test_keyed_consumer_rejects_changed_request_payload(self):
  from integration_executor_contract import executor_contract_from_contents
  run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(executor_digest,run_head)
  self.payload['validation_request']['identity']['bootstrap_epoch']=2
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   with self.assertRaisesRegex(ValueError,'artifact authority'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[executor_digest])
 def test_keyed_consumer_rejects_different_effective_policy_identity(self):
  from integration_executor_contract import executor_contract_from_contents
  run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(executor_digest,run_head)
  changed_policy={**effective_policy,'check_app_id':43}
  with self.assertRaisesRegex(ValueError,'manual integration request identity mismatch'):
   self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=changed_policy,approved_executor_contract_digests=[executor_digest])
 def test_keyed_workflow_base_divergence_rejects_revoked_w_contract(self):
  from integration_executor_contract import executor_contract_from_contents
  workflow_sha='6'*40;run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  revoked='sha256:'+'7'*64
  effective_policy=self.policy(revoked)
  request_key,request_identity,effective_policy=self.bind_keyed_payload(
   executor_digest,run_head,policy=effective_policy,workflow_sha=workflow_sha,
  )
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   with self.assertRaisesRegex(ValueError,'EXECUTOR_CONTRACT_CHANGED'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[revoked])
 def test_keyed_verification_rejects_artifact_selected_workflow_sha(self):
  from integration_executor_contract import executor_contract_from_contents
  run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(
   executor_digest,run_head,workflow_sha='9'*40,
  )
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   with self.assertRaisesRegex(ValueError,'artifact authority'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[executor_digest])
 def test_keyed_verification_requires_expected_run_attempt(self):
  request_key='sha256:'+'8'*64
  with patch.object(self.api,'gh',side_effect=self.read):
   with self.assertRaisesRegex(ValueError,'expected attempt is required'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,approved_executor_contract_digests=['sha256:'+'7'*64])
 def test_keyed_verification_rejects_different_api_run_attempt(self):
  from integration_executor_contract import executor_contract_from_contents
  run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(executor_digest,run_head)
  self.run['run_attempt']=2
  with patch.object(self.api,'gh',side_effect=self.read):
   with self.assertRaisesRegex(ValueError,'attempt mismatch'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[executor_digest])
 def test_keyed_verification_rejects_artifact_attempt_from_another_attempt(self):
  from integration_executor_contract import executor_contract_from_contents
  run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(
   executor_digest,run_head,attempt=2,
  )
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   with self.assertRaisesRegex(ValueError,'artifact authority'):
    self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[executor_digest])
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
  gate_job={
   'id':10,'run_id':9,'run_attempt':1,'name':'required-gate','status':'completed','conclusion':'success',
   'head_sha':self.base,'labels':['ubuntu-24.04'],
   'check_run_url':'https://api.github.com/repos/owner/repo/check-runs/10',
   'started_at':'2026-09-09T00:00:00Z','completed_at':'2026-09-09T01:00:00Z',
  }
  child_jobs=[
   {'id':20,'run_id':9,'run_attempt':1,'name':'windows-package-rollout-behavior','status':'completed','conclusion':'success','head_sha':self.base,'labels':['windows-2022'],'check_run_url':'https://api.github.com/repos/owner/repo/check-runs/20'},
   {'id':21,'run_id':9,'run_attempt':1,'name':'testnet-packages-macos-arm64-contract','status':'completed','conclusion':'success','head_sha':self.base,'labels':['ubuntu-24.04'],'check_run_url':'https://api.github.com/repos/owner/repo/check-runs/21'},
   *({'id':30+i,'run_id':9,'run_attempt':1,'name':f'public-testnet-fleet-health-contract ({runner})','status':'completed','conclusion':'success','head_sha':self.base,'labels':[runner],'check_run_url':f'https://api.github.com/repos/owner/repo/check-runs/{30+i}'} for i,runner in enumerate(('ubuntu-24.04','windows-2022','macos-14'))),
  ]
  def reader(*args):
   path=args[-1]
   if '/compare/' in path:return {'merge_base_commit':{'sha':self.payload['scope_base_oid']}}
   if path=='repos/owner/repo/actions/jobs/10':return gate_job
   if path=='repos/owner/repo/actions/runs/9/attempts/1/jobs?per_page=100&page=1':return {'jobs':[gate_job,*child_jobs]}
   if 'artifacts?' in path:return {'artifacts':[{'id':identifier,'name':name,'expired':False,'created_at':'2026-09-09T00:30:00Z','workflow_run':{'id':9}} for identifier,name in artifact_names.items()]}
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
  self.assertLess(required.index('cp scripts/plan-rust-required-scope.py'),required.index('python3 -I "${RUNNER_TEMP}/integration_ci.py" "${prepare_args[@]}"'))

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

 def test_markerless_pull_request_still_selects_immutable_base_planner(self):
  workflow=(HERE.parents[1]/'.github/workflows/rust.yml').read_text()
  required=workflow[workflow.index('  required-gate:'):workflow.index('  windows-package-rollout-behavior:')]
  scope=required[required.index('      - id: scope\n'):required.index('      - name: Report planned scope')]
  pr_start=scope.index('          if [[ "${GITHUB_EVENT_NAME}" == pull_request')
  dispatch_start=scope.index('          elif [[ "${GITHUB_EVENT_NAME}" == workflow_dispatch ]]; then',pr_start)
  pr_branch=scope[pr_start:dispatch_start]
  self.assertTrue(pr_branch.startswith('          if [[ "${GITHUB_EVENT_NAME}" == pull_request ]]; then'))
  planner='planner=(python3 -I "${authority_dir}/plan-rust-required-scope.py")'
  projection_guard='if [[ -f "${RUNNER_TEMP}/impact-projection.json" ]]; then'
  self.assertIn('git show "${base_ref}:scripts/plan-rust-required-scope.py"',pr_branch)
  self.assertIn('git show "${base_ref}:scripts/ci-required-scope.v2.json"',pr_branch)
  self.assertIn('git show "${base_ref}:scripts/pm/workflow-impact-projection.py"',pr_branch)
  self.assertIn(planner,pr_branch)
  self.assertIn(projection_guard,pr_branch)
  self.assertLess(pr_branch.index(planner),pr_branch.index(projection_guard))
  self.assertNotIn('planner=(./scripts/plan-rust-required-scope.sh)',pr_branch)

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
