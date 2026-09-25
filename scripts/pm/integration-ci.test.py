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
 def test_keyed_workflow_base_divergence_requires_approved_w_contract(self):
  from integration_executor_contract import executor_contract_from_contents
  workflow_sha='6'*40;run_head='6'*40
  executor_digest=executor_contract_from_contents(self.executor_contents)['digest']
  request_key,request_identity,effective_policy=self.bind_keyed_payload(
   executor_digest,run_head,workflow_sha=workflow_sha,
  )
  raw=io.BytesIO()
  with zipfile.ZipFile(raw,'w') as archive:archive.writestr(self.api.ARTIFACT+'.json',json.dumps(self.payload))
  with patch.object(self.api,'gh',side_effect=self.read),patch.object(self.api.subprocess,'check_output',return_value=raw.getvalue()):
   check,proof=self.api.verified_run('owner/repo',self.uid,12,self.base,self.head,9,42,request_key=request_key,expected_attempt=1,request_identity=request_identity,effective_policy=effective_policy,approved_executor_contract_digests=[executor_digest])
  self.assertEqual(workflow_sha,proof['workflow_sha'])
  self.assertEqual(self.base,proof['base_oid'])
  self.assertEqual(run_head,proof['workflow_run_head_sha'])
  self.assertEqual(run_head,check['head_sha'])
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
