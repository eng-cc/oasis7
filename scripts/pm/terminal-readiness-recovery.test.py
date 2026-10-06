#!/usr/bin/env python3
"""Partial T1: truthful both-missing control through the existing effect engine.

Full recovery positives await independently validated offline review/CI inputs.
No synthetic native or ordinary completion success is supplied here.
"""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('recovery_delivery_fixture',
    HERE / 'terminal-delivery-protocol.test.py')
protocol = importlib.util.module_from_spec(spec)
spec.loader.exec_module(protocol)


class BothMissingHistoricalControl(unittest.TestCase):
    def test_actual_finalizer_cannot_manufacture_both_missing_historical_claims(self):
        with tempfile.TemporaryDirectory() as temp:
            fixture = protocol.DeliveryFixture(Path(temp))
            body = (f'task_uid: {protocol.UID}\n- pr_number: `{protocol.PR}`\n'
                    f'- pr_url: `{protocol.PR_URL}`\n')
            mapping = fixture.mapping()
            mapping['tasks'][protocol.UID].pop('claim_verifications', None)
            fixture.mapping_path.write_text(json.dumps(mapping))
            fixture.state['issue']['body'] = body
            fixture.state['project_item']['content']['body'] = body
            fixture.state['comments'] = []
            fixture.state.pop('readiness_comments', None)
            fixture._write_state()
            self.assertTrue((fixture.receipt_root / 'merge-receipt.json').exists())
            self.assertFalse((fixture.receipt_root / 'readiness-proof.json').exists())
            self.assertNotIn('claim_verifications', mapping['tasks'][protocol.UID])
            self.assertEqual(fixture.state['comments'], [])
            def snapshot():
                return (fixture.mapping_path.read_bytes(), fixture.state_path.read_bytes(),
                    {str(path.relative_to(fixture.receipt_root)): path.read_bytes()
                     for path in fixture.receipt_root.rglob('*') if path.is_file()})
            before = snapshot()
            proc = fixture.run_producer()
            self.assertNotEqual(proc.returncode, 0, proc.stdout + proc.stderr)
            self.assertRegex((proc.stdout + proc.stderr).lower(), 'readiness|task.complete|completion')
            self.assertEqual(snapshot(), before,
                'missing historical claims must not publish receipt/comment/done/ledger/tombstone')
            if fixture.log_path.exists():
                commands = [json.loads(line) for line in fixture.log_path.read_text().splitlines()]
                self.assertFalse(any(command[:2] in [['issue', 'comment'], ['issue', 'close']]
                                     for command in commands), commands)



# Self-contained T1B fixture: existing validators are never patched.
"""Offline full four-role review factory; existing validators are never patched.
Import into admitted test, passing repository root; no ignored-path dependency.
Uses the existing tracked transport fixture only for temporary Git/API harness.
"""
import hashlib,importlib.util,json,sys,uuid,shutil,subprocess
from pathlib import Path

def load(path,name):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def build_review(repo):
 repo=Path(repo).resolve();sys.path.insert(0,str(repo/'scripts/pm'))
 m=load(repo/'scripts/pm/review_preflight_handoff.test.py','full_review_fixture_harness')
 c=m.ReviewPreflightHandoffTests();c.setUp()
 roles=['repository_health_engineer','producer_system_designer','qa_engineer','liveops_community']
 shutil.copytree(repo/'scripts/pm',c.root/'scripts/pm',dirs_exist_ok=True,ignore=shutil.ignore_patterns('__pycache__','*.pyc'))
 evidence=c.root/'closure-evidence.json'
 evidence.write_text(json.dumps({'scope':'offline workflow fixture','consumers':['review','ci','terminal'],'classification':'mixed','reason':'all fixture consumers explicitly enumerated'}))

 for rel in ('scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/ci-tests.sh','scripts/ci-required-capability-test-inventory.tsv','scripts/product_doc_markdown.py','scripts/doc-governance-requirements.txt','.github/workflows/rust.yml','scripts/pm/ci_required_inventory.py','scripts/pm/pr-merge-receipt.py','scripts/pm/task-closeout.sh',*[f'.agents/roles/{r}.md' for r in roles],'doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)

 for rel in ('scripts/pm/readiness_transport.py','scripts/pm/loop_terminal.py','scripts/pm/terminal_proof.py','scripts/pm/post-merge-finalize.py','scripts/pm/pr-lifecycle-gate.py','scripts/pm/claim-ready.sh','scripts/pm/ci_reuse_policy.py'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)
 subprocess.run(['git','-C',str(c.root),'add','closure-evidence.json','scripts','.agents','.github','doc'],check=True)
 subprocess.run(['git','-C',str(c.root),'commit','--amend','--no-edit','-q'],check=True)
 m.HEAD=subprocess.check_output(['git','-C',str(c.root),'rev-parse','HEAD'],text=True).strip()
 paths=subprocess.check_output(['git','-C',str(c.root),'diff','--name-only',m.SCOPE_OID,m.HEAD],text=True).splitlines()
 f=c.make_fixture(create_handoff=False,publish_dispatch=False,changed_paths=paths)
 old=f['plan']['impact_projection']
 inp={k:old[k] for k in ('task_uid','source_head_oid','scope_base_oid','changed_paths','test_profile','declared_tests','consumed_contracts','public_semantics','affected_consumers')}
 inp.update(change_class='mixed',manual_roles=roles,domain_role=None,verification_affected=True,closure_status={'status':'complete','reason':'finite offline fixture consumer closure','evidence':[{'path':str(evidence),'sha256':'sha256:'+hashlib.sha256(evidence.read_bytes()).hexdigest()}]})
 
 for rel in ('scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/ci-tests.sh','scripts/ci-required-capability-test-inventory.tsv','scripts/product_doc_markdown.py','scripts/doc-governance-requirements.txt','.github/workflows/rust.yml','scripts/pm/ci_required_inventory.py','scripts/pm/pr-merge-receipt.py','scripts/pm/task-closeout.sh',*[f'.agents/roles/{r}.md' for r in roles],'doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)
 inp['closure_status']['evidence'][0]['path']=evidence.relative_to(c.root).as_posix()
 projection=m.IMPACT_PROJECTION.build_projection(c.root,inp)
 assert projection['closure_status']['status']=='complete'
 roles=projection['ordered_role_ids']; assert set(roles)==set(['repository_health_engineer','producer_system_designer','qa_engineer','liveops_community'])
 identity=dict(f['plan']['source_review_identity']);identity.update(ordered_role_ids=roles,input_contract_digest=projection['projection_digest'].removeprefix('sha256:'),changed_paths_digest=projection['changed_paths_digest'].removeprefix('sha256:'))

 def contract_digest(paths):
  return m.digest([{'path':rel,'sha256':hashlib.sha256((c.root/rel).read_bytes()).hexdigest()} for rel in paths])
 identity['role_contract_digest']=contract_digest([f'.agents/roles/{r}.md' for r in roles])
 identity['review_policy_digest']=contract_digest(['doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'])
 sd=m.digest(identity)
 batchpath=c.task_root/'review-batches'/'full.json'
 slices=[{'role':r,'slice_id':str(uuid.uuid5(uuid.NAMESPACE_URL,r))} for r in roles]
 args=['create','--task-uid',m.TASK,'--head',m.HEAD,'--evidence-digest',sd]
 for s in slices:args+=['--slice',s['role']+'='+s['slice_id']]
 args+=['--out',str(batchpath)]
 batch=c.command_json(*args); epoch=batch['epoch'];pre=c.command_json('preflight','--batch',str(batchpath),'--out-dir',str(c.task_root/'full-preflight'))
 refs=[]
 for s in slices:
  packet={'schema':'oasis7-subagent-task-packet/v1','identity':{'task_uid':m.TASK,'head':m.HEAD,'base_sha':m.SCOPE_OID},'slice':dict(s,role_activation='message_assigned_adapter_inactive',context_delivery_mode='minimal_head_bound_task_packet',actual_dispatched_model_reasoning=m.MODEL_REASONING,actual_runtime_evidence_reason=m.RUNTIME_REASON)}
  packet['packet_digest']=m.digest(packet); pp=c.task_root/'slice-packets'/(s['slice_id']+'.json');pp.write_text(json.dumps(packet,sort_keys=True)+'\n');refs.append(dict(s,packet_ref=pp.relative_to(c.root).as_posix()))
  rp=c.task_root/'full-preflight'/(s['slice_id']+'.json');rv=json.loads(rp.read_text());rv.update(status='completed',activation=m.ACTIVATION,context_delivery=m.CONTEXT_DELIVERY,actual_runtime=m.MODEL_REASONING+': '+m.RUNTIME_REASON,scope_verdict='offline full-role simulation review',risk_verdict='offline simulation reviewed',disposition='no_findings',findings=[],residual_risk='simulation only, not live evidence',admitted_packet_digest=packet['packet_digest']);rp.write_text(json.dumps(rv,sort_keys=True)+'\n')
 plan=dict(f['plan']);app={k:identity[k] for k in ('changed_paths_digest','input_contract_digest','ordered_role_ids','role_contract_digest','review_policy_digest')}
 plan.update(source_review_identity=identity,source_review_digest=sd,relevant_evidence_digest=sd,professional_review_applicability={'identity':app,'identity_digest':m.digest(app),'verified':True},impact_projection=projection,impact_projection_schema=projection['schema'],impact_projection_digest=projection['projection_digest'],impact_projection_test_profile=projection['test_profile'],impact_projection_declared_tests=projection['declared_tests'],impact_projection_planner_digest=projection['planner_digest'],epoch=epoch,batch_path=str(batchpath),collection_path=str(batchpath.with_name('full.collection.json')),roles=roles,expected_slices=slices,packet_refs=refs,preflight={'status':'incomplete','ledger_path':pre['ledger_path']})
 pp=c.task_root/'review-plans'/'full.json';pp.write_text(json.dumps(plan,sort_keys=True)+'\n')
 result=m.HANDOFF.publish_dispatch(c.root,pp)
 hp=m.HANDOFF.create_handoff(c.root,pp,result['dispatch_comment_id'])
 handoff_path=c.task_root/'review-handoffs'/(epoch+'.json')
 validated=m.HANDOFF.validate_handoff(c.root,handoff_path,expected_plan_path=pp)

 resolution=load(repo/'scripts/pm/review-findings-resolution.py','recovery_fixture_resolution')
 records=c.task_root/'records.json';records.write_text('[]\n')
 mr=resolution.create_manifest(c.root.resolve(),m.TASK,m.HEAD,epoch,records.resolve(),None,handoff_path.resolve())
 manifest=json.loads(Path(mr['manifest']).read_text())
 bodypayload={'marker':'oasis7-review-resolution','schema':'oasis7-review-resolution/v2','task_uid':m.TASK,'head':m.HEAD,'epoch':epoch,'manifest_digest':manifest['manifest_digest']}
 body=m.canonical(bodypayload).decode()
 live=json.loads(c.gh_data.read_text());live['next_comment_id']=m.DISPATCH_COMMENT_ID+1;c.gh_data.write_text(json.dumps(live))
 posted=json.loads(subprocess.check_output(['gh','api',f'repos/{m.REPOSITORY}/issues/{m.TASK_ISSUE}/comments','--method','POST','-f','body='+body],text=True))
 rb={**bodypayload,'repository':m.REPOSITORY,'issue_number':m.TASK_ISSUE,'comment_id':posted['id'],'comment_url':posted['html_url'],'author':posted['user']['login'],'created_at':posted['created_at'],'observed_at':posted['created_at'],'body_digest':hashlib.sha256(body.encode()).hexdigest()}
 Path(mr['manifest']).with_suffix('.readback.json').write_text(json.dumps(rb,sort_keys=True)+'\n')
 resolution.validate_manifest(c.root.resolve(),Path(mr['manifest']),Path(pre['ledger_path']).resolve(),m.TASK,m.HEAD)
 return {'manifest_path':Path(mr['manifest']),'harness':c,'module':m,'root':c.root,'head':m.HEAD,'base':m.SCOPE_OID,'plan_path':pp,'handoff_path':handoff_path,'plan':plan,'validated_handoff':validated,'roles':roles,'epoch':epoch}

def build_api_inputs(f):
 """Authentic-shaped merged delivery inputs; only transport may be replaced."""
 import base64,io,zipfile
 from unittest.mock import patch
 import integration_ci,terminal_proof
 repo=f['module'].REPOSITORY;uid=f['module'].TASK;issue=f['module'].TASK_ISSUE;pr=1
 c=f['harness'];root=c.root;head=f['head'];base=f['base']
 git=lambda *args:subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
 git('switch','-q','--detach',base)
 source_patch=subprocess.check_output(['git','-C',str(root),'diff','--binary',base,head])
 subprocess.run(['git','-C',str(root),'apply','--index'],input=source_patch,check=True)
 git('commit','-qm','independently squashed accepted delivery')
 merged=git('rev-parse','HEAD');target=merged
 merged_patch=subprocess.check_output(['git','-C',str(root),'diff','--binary',base,merged]);assert source_patch==merged_patch
 assert git('merge-base',merged,target)==merged
 url=f'https://github.com/{repo}/pull/{pr}'
 livepr={'number':pr,'html_url':url,'state':'closed','merged':True,'merged_at':'2026-10-01T00:00:00Z','merge_commit_sha':merged,'body':f'Task: {uid}\nRefs #{issue}','head':{'sha':head,'ref':'source','repo':{'full_name':repo}},'base':{'sha':base,'ref':'main','repo':{'full_name':repo}}}
 compare={'url':f'https://api.github.com/repos/{repo}/compare/{merged}...{target}','base_commit':{'sha':merged},'merge_base_commit':{'sha':merged},'ahead_by':0,'behind_by':0,'total_commits':0,'status':'identical'}
 lr={'repository':{'full_name':repo,'default_branch':'main'},'ref':{'ref':'refs/heads/main','object':{'sha':target}},'merge_compare':compare}
 terminal_proof._validate_live_pr(livepr,repo,uid,issue,pr,url,head,merged,'main')
 terminal_proof._validate_live_repository(lr,repo,merged,'main',target)
 runid=1701;app=15368;attempt=1;checkid=1801;jobid=1901
 run={'id':runid,'run_attempt':attempt,'event':'workflow_dispatch','head_branch':'main','head_sha':base,'path':integration_ci.WORKFLOW,'repository':{'full_name':repo},'status':'completed','conclusion':'success','check_suite_id':2001,'created_at':'2026-09-30T10:00:00Z','run_started_at':'2026-09-30T10:00:00Z','display_title':f'oasis7-ci|workflow_dispatch|integration_revalidation|{uid}|{pr}|{base}|{head}'}
 job={'id':jobid,'run_id':runid,'run_attempt':attempt,'head_sha':base,'name':'required-gate','check_run_url':f'https://api.github.com/repos/{repo}/check-runs/{checkid}','status':'completed','conclusion':'success','labels':['ubuntu-latest']}
 check={'id':checkid,'name':'required-gate','app':{'id':app},'head_sha':base,'status':'completed','conclusion':'success','details_url':f'https://github.com/{repo}/actions/runs/{runid}/job/{jobid}'}
 rawplanner=f['module'].IMPACT_PROJECTION.run_scope_planner(root,f['plan']['impact_projection']['changed_paths'],f['plan']['impact_projection']['ci_scope']=='full')
 receipt=load(f['module'].PROJECT_ROOT/'scripts/pm/ci-ready-receipt.py','fixture_ci_receipt')
 planner=receipt.canonical_planner(rawplanner)
 check['output']={'summary':'<!-- oasis7-required-plan-v1 -->\n```json\n'+json.dumps(rawplanner)+'\n```'}
 assert receipt.planner_from_run(check)==planner
 proof={'schema':integration_ci.ARTIFACT,'repository':repo,'workflow_run_id':runid,'base_oid':base,'head_oid':head,'task_uid':uid,'pr_number':pr,'workflow_sha':base,'workflow_ref':f'{repo}/{integration_ci.WORKFLOW}@refs/heads/main','integration_mode':'integration_revalidation','check_name':'required-gate','scope_base_oid':base,'tested_tree_oid':git('rev-parse',head+'^{tree}'),'tested_commit_oid':head}
 archive=io.BytesIO()
 with zipfile.ZipFile(archive,'w') as z:z.writestr(integration_ci.ARTIFACT+'.json',json.dumps(proof))
 paths={f'repos/{repo}/actions/workflows/rust.yml/runs?event=workflow_dispatch&per_page=100&page=1':{'workflow_runs':[run]},f'repos/{repo}/actions/runs/{runid}':run,f'repos/{repo}/actions/runs/{runid}/attempts/1/jobs?per_page=100&page=1':{'jobs':[job]},f'repos/{repo}/check-runs/{checkid}':check,f'repos/{repo}/check-suites/2001/check-runs?per_page=100&page=1':{'check_runs':[check]},f'repos/{repo}/actions/runs/{runid}/artifacts?per_page=100&page=1':{'artifacts':[{'id':2101,'name':integration_ci.ARTIFACT,'expired':False,'workflow_run':{'id':runid}}]},f'repos/{repo}/pulls/{pr}':livepr,f'repos/{repo}':lr['repository'],f'repos/{repo}/git/ref/heads/main':lr['ref'],f'repos/{repo}/compare/{merged}...{target}':compare}
 reads=[]
 def transport(*args):
  assert args[0]=='api';path=args[1];reads.append(path)
  if path not in paths:raise AssertionError('unprovided offline API '+path)
  return json.loads(json.dumps(paths[path]))
 with patch.object(integration_ci,'gh',side_effect=transport):
  selected=integration_ci.current_request(repo,uid,pr,base,head,'main');assert selected['id']==runid
  jobs=integration_ci.attempt_execution_jobs(repo,runid,1,base,app,require_completed=True);assert jobs[0]['check_run_id']==checkid
  try:integration_ci.verified_run(repo,uid,pr,base,head,runid,app,expected_attempt=1)
  except ValueError as e:
   assert 'PR source/target moved' in str(e);gap=str(e)
  else:raise AssertionError('ordinary verifier must reject truthful merged PR')
 source={'oid':merged,'tree_oid':git('rev-parse',merged+'^{tree}'),'gate_entry_sha256':hashlib.sha256((root/'scripts/pm/pr-lifecycle-gate.py').read_bytes()).hexdigest(),'claim_entry_sha256':hashlib.sha256((root/'scripts/pm/claim-ready.sh').read_bytes()).hexdigest()}
 auth={'schema':'oasis7-terminal-recovery-authorization/v1','repository':repo,'task_uid':uid,'issue_number':issue,'pr_number':pr,'head_oid':head,'merge_commit_oid':merged,'purpose':'postmerge_delivery_only','decision':'authorize_current_terminal_observation','reason':'genuine historical readiness and completion absent'}
 return {'repository':repo,'task_uid':uid,'issue':issue,'pr':pr,'head':head,'base':base,'merge':merged,'target':target,'live_pr':livepr,'live_repository':lr,'api':paths,'api_reads':reads,'holds_inputs':{'merge_hold_comments':[],'reviews':[],'threads':{'nodes':[],'pageInfo':{'hasNextPage':False}},'operator':{'login':'repo-admin'},'operator_permission':{'permission':'admin'},'required_policy':{'required_status_checks':{'contexts':[],'checks':[{'context':'required-gate','app_id':app}]}},'rulesets':[]},'run':run,'job':job,'check':check,'planner_raw':rawplanner,'planner':planner,'artifact':proof,'artifact_zip_b64':base64.b64encode(archive.getvalue()).decode(),'source':source,'authorization':auth,'source_patch_sha256':hashlib.sha256(source_patch).hexdigest(),'merged_patch_sha256':hashlib.sha256(merged_patch).hexdigest(),'ordinary_verifier_gap':gap,'validated':['validate_handoff','validate_manifest','canonical_planner','planner_from_run','current_request','attempt_execution_jobs','_validate_live_pr','_validate_live_repository','actual_git_patch_equality','actual_git_ancestry']}


class MergedIntegrationComponent(unittest.TestCase):
    """Actual API/ZIP inputs, real validators; only external gh transport is fake.

    Proposed exact seven positional arguments below. The last closed mapping is
    locator/context input, never a caller verdict: reader revalidates plan,
    handoff, resolution, merged identity, policy and target applicability.
    Result is precisely the integration record's nine additional fields.
    """
    @classmethod
    def setUpClass(cls):
        import copy,os
        cls.review=build_review(HERE.parents[1])
        cls.inputs=build_api_inputs(cls.review)
        cls.original_gh=cls.review['harness'].fake_gh.read_text()
        cls.transport_path=cls.review['root']/'merged-api.json'
        cls.review['harness'].fake_gh.write_text(
            '#!/usr/bin/env python3\nimport base64,json,os,sys\n'
            +'data=json.load(open('+repr(str(cls.transport_path))+'))\n'
            +"endpoint=next((x for x in sys.argv[1:] if x.startswith('repos/')),None)\n"
            +"if endpoint in data['responses']:\n value=data['responses'][endpoint]\n if isinstance(value,dict) and set(value)=={'binary_b64'}:sys.stdout.buffer.write(base64.b64decode(value['binary_b64']))\n else:print(json.dumps(value))\n raise SystemExit(0)\n"
            +'exec('+repr(cls.original_gh)+')\n')
        cls.review['harness'].fake_gh.chmod(0o755)
    @classmethod
    def tearDownClass(cls):
        cls.review['harness'].tearDown();cls.review['harness'].doCleanups()
    def setUp(self):
        import copy
        self.data=copy.deepcopy(self.inputs);a=self.data;repo=a['repository'];run=a['run']['id']
        self.responses=a['api']
        self.responses[f"repos/{repo}/actions/artifacts/2101/zip"]={'binary_b64':a['artifact_zip_b64']}
        self.responses[f"repos/{repo}/branches/main/protection"]=a['holds_inputs']['required_policy']
        self.responses[f"repos/{repo}/rulesets?per_page=100&page=1"]=[]
        self.responses[f"repos/{repo}/rulesets?per_page=100"]=[]
        self.responses[f"repos/{repo}/actions/runs/{run}/attempts/1"] = a['run']
        self.context={'repository_root':str(self.review['root'].resolve()),
            'source_review_plan_path':str(self.review['plan_path'].resolve()),
            'source_review_handoff_path':str(self.review['handoff_path'].resolve()),
            'source_review_resolution_path':str(self.review['manifest_path'].resolve()),
            'source_scope_oid':a['base'],'merge_commit_oid':a['merge'],
            'observed_target_oid':a['target'],'check_app_id':15368,'request_key':None}
        self.publish_transport()
    def publish_transport(self):
        self.transport_path.write_text(json.dumps({'responses':self.responses}))
    def call(self):
        import integration_ci
        self.publish_transport()
        reader=getattr(integration_ci,'verify_merged_delivery_integration',None)
        self.assertTrue(callable(reader),'unsupported explicit merged-delivery CI reader; ordinary verified_run is not substituted')
        selected=integration_ci.current_request(self.data['repository'],self.data['task_uid'],self.data['pr'],self.data['base'],self.data['head'],'main',request_key=self.context['request_key'])
        return reader(self.data['repository'],self.data['task_uid'],self.data['pr'],self.data['head'],self.data['base'],selected,self.context)
    def archive(self,payload=None,members=None):
        import io,zipfile,base64,integration_ci
        raw=io.BytesIO()
        with zipfile.ZipFile(raw,'w') as z:
            for name,value in (members or [(integration_ci.ARTIFACT+'.json',json.dumps(payload or self.data['artifact']))]):z.writestr(name,value)
        key=f"repos/{self.data['repository']}/actions/artifacts/2101/zip"
        self.responses[key]={'binary_b64':base64.b64encode(raw.getvalue()).decode()}
    def test_authentic_existing_review_ci_and_merge_primitives_pass(self):
        import integration_ci,terminal_proof
        h=self.review['module'].HANDOFF
        h.validate_handoff(self.review['root'],self.review['handoff_path'])
        self.assertEqual(set(self.review['roles']),{'repository_health_engineer','producer_system_designer','qa_engineer','liveops_community'})
        self.assertEqual(self.review['plan']['impact_projection']['closure_status']['status'],'complete')
        jobs=integration_ci.attempt_execution_jobs(self.data['repository'],1701,1,self.data['base'],15368,require_completed=True)
        self.assertEqual(jobs[0]['check_run_id'],1801)
        with self.assertRaisesRegex(ValueError,'PR source/target moved'):
            integration_ci.verified_run(self.data['repository'],self.data['task_uid'],self.data['pr'],self.data['base'],self.data['head'],1701,15368,expected_attempt=1)
    def test_merged_valid_actual_context_returns_only_closed_integration_projection(self):
        value=self.call()
        self.assertEqual(set(value),{'base_oid','run_id','run_attempt','app_id','check_run_id','workflow_sha','tested_tree_oid','request_key','proof'})
        self.assertEqual(value['base_oid'],self.data['base']);self.assertEqual(value['run_id'],1701)
        self.assertEqual(value['run_attempt'],1);self.assertEqual(value['app_id'],15368)
        self.assertEqual(value['check_run_id'],1801);self.assertEqual(value['workflow_sha'],self.data['base'])
        self.assertIsNone(value['request_key']);self.assertEqual(value['proof'],self.data['artifact'])
        expected_tree=subprocess.check_output(['git','-C',str(self.review['root']),'rev-parse',self.data['head']+'^{tree}'],text=True).strip()
        self.assertEqual(value['tested_tree_oid'],expected_tree)
        for field in ('run_id','run_attempt','app_id','check_run_id'):
            self.assertIs(type(value[field]),int,field);self.assertGreater(value[field],0,field)
        for field in ('base_oid','workflow_sha','tested_tree_oid'):
            self.assertIs(type(value[field]),str,field);self.assertRegex(value[field],r'^(?:[0-9a-f]{40}|[0-9a-f]{64})$',field)
        self.assertIs(type(value['proof']),dict)
    def test_caller_success_flags_are_not_authority(self):
        self.context.update(verified=True,requires_strict_integration=False)
        with self.assertRaisesRegex(ValueError,'closed|unknown|context|applicability'):self.call()
    def test_payload_authority_tampering_blocks(self):
        import copy
        mutations={'base_oid':'f'*40,'head_oid':'f'*40,'task_uid':'task_'+'f'*32,'pr_number':99,'workflow_sha':'f'*40,'workflow_run_id':99,'workflow_ref':'foreign/repo/.github/workflows/rust.yml@refs/heads/main','scope_base_oid':'bad','tested_tree_oid':'bad','tested_commit_oid':'bad'}
        for field,value in mutations.items():
            with self.subTest(field=field):
                payload=copy.deepcopy(self.data['artifact']);payload[field]=value;self.archive(payload)
                with self.assertRaisesRegex(ValueError,'artifact|authority|identity|source|tree'):self.call()
    def test_archive_member_and_duplicate_artifact_blocks(self):
        self.archive(members=[('foreign.json','{}')])
        with self.assertRaisesRegex(ValueError,'artifact|member|archive'):self.call()
        self.archive();key=next(k for k in self.responses if '/artifacts?' in k)
        self.responses[key]['artifacts'].append(dict(self.responses[key]['artifacts'][0],id=2102))
        with self.assertRaisesRegex(ValueError,'artifact|ambiguous|duplicate'):self.call()
    def test_exact_attempt_job_app_head_checklink_blocks(self):
        import copy
        jobkey=next(k for k in self.responses if '/jobs?' in k);original=copy.deepcopy(self.responses)
        cases=[('run_attempt',2),('head_sha','f'*40),('check_run_url','https://api.github.com/repos/foreign/repo/check-runs/1801')]
        for field,value in cases:
            with self.subTest(field=field):
                self.responses=copy.deepcopy(original);self.responses[jobkey]['jobs'][0][field]=value
                with self.assertRaisesRegex(ValueError,'attempt|job|check|identity|provenance'):self.call()
        self.responses=copy.deepcopy(original);ck=f"repos/{self.data['repository']}/check-runs/1801";self.responses[ck]['app']['id']=99
        with self.assertRaisesRegex(ValueError,'app|check|identity'):self.call()
    def test_latest_attempt_and_newer_unsuccessful_request_blocks(self):
        runkey=f"repos/{self.data['repository']}/actions/runs/1701";self.responses[runkey]['run_attempt']=2
        with self.assertRaisesRegex(ValueError,'attempt|latest'):self.call()
        self.responses[runkey]['run_attempt']=1;rowskey=next(k for k in self.responses if '/runs?' in k)
        newer=dict(self.data['run'],id=1702,created_at='2026-09-30T11:00:00Z',run_started_at='2026-09-30T11:00:00Z',status='in_progress',conclusion=None)
        self.responses[rowskey]['workflow_runs'].append(newer);self.responses[f"repos/{self.data['repository']}/actions/runs/1702"]=newer
        with self.assertRaisesRegex(ValueError,'current|success|completed|status'):self.call()
    def test_wrong_planner_and_policy_app_blocks(self):
        ck=f"repos/{self.data['repository']}/check-runs/1801";self.responses[ck]['output']['summary']='<!-- oasis7-required-plan-v1 -->\n```json\n{}\n```'
        with self.assertRaisesRegex((ValueError,SystemExit),'planner|metadata|plan'):self.call()
    def test_truthful_merged_identity_and_unknown_target_blocks(self):
        pk=f"repos/{self.data['repository']}/pulls/1";self.responses[pk]['head']['sha']='f'*40
        with self.assertRaisesRegex(ValueError,'identity|head|source|merge'):self.call()
        self.responses[pk]['head']['sha']=self.data['head'];self.context['observed_target_oid']='f'*40
        with self.assertRaisesRegex(ValueError,'target|ancestry|applicability|identity'):self.call()

    def test_current_policy_app_identity_cannot_be_caller_selected(self):
        key=f"repos/{self.data['repository']}/branches/main/protection"
        self.responses[key]['required_status_checks']['checks'][0]['app_id']=99
        with self.assertRaisesRegex(ValueError,'policy|app|identity'):self.call()
    def test_unverified_keyed_executor_source_cannot_fall_back_to_legacy(self):
        self.context['request_key']='sha256:'+'f'*64
        with self.assertRaisesRegex(ValueError,'key|request|executor|source|identity'):self.call()
    def test_unbound_review_and_incomplete_discovery_block(self):
        hp=self.review['handoff_path'];before=hp.read_bytes();payload=json.loads(before);payload['frozen_head']='f'*40;hp.write_text(json.dumps(payload))
        try:
            with self.assertRaisesRegex(ValueError,'handoff|review|head|identity|digest'):self.call()
        finally:hp.write_bytes(before)
        key=next(k for k in self.responses if '/runs?' in k);self.responses[key]['workflow_runs']=None
        with self.assertRaisesRegex(ValueError,'discovery|malformed|pagination'):self.call()

    def trusted_disabled_keyed_inputs(self):
        import base64,integration_ci, integration_executor_contract as executor
        a=self.data;repo=a['repository'];w=a['merge']
        for relative in ('scripts/pm/ci_reuse_policy.py','scripts/ci-required-scope.v2.json'):
            raw=subprocess.check_output(['git','-C',str(self.review['root']),'show',w+':'+relative])
            self.responses[f'repos/{repo}/contents/{relative}?ref={w}']={'type':'file','path':relative,'encoding':'base64','content':base64.b64encode(raw).decode()}
        self.publish_transport()
        context=integration_ci.trusted_policy_context(repo,'main',w,w)
        self.assertEqual(context['effective_policy']['enabled_capabilities'],[])
        self.assertEqual(context['effective_policy']['approved_executor_contract_digests'],[])
        actual={relative:(HERE.parents[1]/relative).read_bytes() for relative in executor.EXECUTOR_CONTRACT_PATHS}
        digest=executor.executor_contract_from_contents(actual)['digest']
        identity={'repository':repo,'task_uid':a['task_uid'],'pr_number':a['pr'],'bootstrap_epoch':1,'source_head_oid':a['head'],'publication_id':'offline-keyed-capability-probe','source_projection_digest':self.review['plan']['impact_projection']['projection_digest'],'unit_ids':['required-gate'],'input_fingerprints':{'required-gate':'sha256:'+hashlib.sha256(json.dumps(a['planner'],sort_keys=True).encode()).hexdigest()},'executor_contract_digest':digest,'effective_policy_digest':executor.effective_policy_digest(context['effective_policy']),'purpose':'integration_revalidation','applicability_mode':'input_scoped','snapshot_target_oid':None}
        executor.validation_request_identity(identity)
        return context,identity,executor.validation_request_key(identity)
    def test_authentic_trusted_keyed_policy_is_disabled_without_substitution(self):
        import integration_ci
        context,identity,key=self.trusted_disabled_keyed_inputs()
        with self.assertRaisesRegex(ValueError,'input-scope-reuse/v1 is disabled'):
            integration_ci.verified_run(self.data['repository'],self.data['task_uid'],self.data['pr'],self.data['base'],self.data['head'],1701,15368,request_key=key,expected_attempt=1,request_identity=identity,effective_policy=context['effective_policy'])
    def test_merged_keyed_request_cannot_activate_disabled_trusted_capability(self):
        context,identity,key=self.trusted_disabled_keyed_inputs()
        self.context['request_key']=key
        self.data['run'].update(head_sha=self.data['merge'],display_title=f"oasis7-ci|workflow_dispatch|integration_revalidation|{self.data['task_uid']}|{self.data['pr']}|{self.data['base']}|{self.data['head']}|{key}")
        with self.assertRaisesRegex(ValueError,'disabled|capability|not supported'):self.call()

    def git_case(self,kind):
        """Generate accessible M/T and honest API identity; restore shared checkout."""
        import terminal_proof
        root=self.review['root'];a=self.data;repo=a['repository'];original=a['merge'];base=a['base']
        git=lambda *args:subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
        try:
            git('switch','-q','--detach',base if kind in ('unequal_patch','non_descendant') else original)
            if kind=='unequal_patch':
                patch=subprocess.check_output(['git','-C',str(root),'diff','--binary',base,a['head']])
                subprocess.run(['git','-C',str(root),'apply','--index'],input=patch,check=True)
                (root/'delivery-drift.txt').write_text('actual divergent delivered patch\n')
                git('add','delivery-drift.txt');git('commit','-qm','actual inequivalent squash')
                merged=git('rev-parse','HEAD');target=merged
                source_patch=subprocess.check_output(['git','-C',str(root),'diff','--binary',base,a['head']])
                merged_patch=subprocess.check_output(['git','-C',str(root),'diff','--binary',base,merged])
                self.assertNotEqual(source_patch,merged_patch)
                self.assertEqual(git('rev-parse',merged+'^'),base)
            elif kind=='related_target':
                rel='scripts/ci-required-scope.v2.json'
                (root/rel).write_bytes((root/rel).read_bytes()+b'\n')
                git('add',rel);git('commit','-qm','actual related planner policy target drift')
                merged=original;target=git('rev-parse','HEAD')
                self.assertEqual(git('diff','--name-only',merged,target),rel)
                self.assertEqual(git('merge-base',merged,target),merged)
            elif kind=='non_descendant':
                (root/'alternative-target.txt').write_text('actual independent target\n')
                git('add','alternative-target.txt');git('commit','-qm','real target without delivered merge')
                merged=original;target=git('rev-parse','HEAD')
                self.assertEqual(git('merge-base',merged,target),base)
                self.assertNotEqual(base,merged)
            else:raise AssertionError(kind)
            mergebase=git('merge-base',merged,target)
            ahead=int(git('rev-list','--count',merged+'..'+target));behind=int(git('rev-list','--count',target+'..'+merged))
            compare={'url':f'https://api.github.com/repos/{repo}/compare/{merged}...{target}','base_commit':{'sha':merged},'merge_base_commit':{'sha':mergebase},'ahead_by':ahead,'behind_by':behind,'total_commits':ahead,'status':'identical' if merged==target else ('ahead' if behind==0 else 'diverged')}
            self.responses[f'repos/{repo}/compare/{merged}...{target}']=compare
            self.responses[f'repos/{repo}/git/ref/heads/main']={'ref':'refs/heads/main','object':{'sha':target}}
            pr=self.responses[f'repos/{repo}/pulls/1'];pr['merge_commit_sha']=merged
            self.context.update(merge_commit_oid=merged,observed_target_oid=target)
            terminal_proof._validate_live_pr(pr,repo,a['task_uid'],a['issue'],a['pr'],pr['html_url'],a['head'],merged,'main')
            lr={'repository':self.responses[f'repos/{repo}'],'ref':self.responses[f'repos/{repo}/git/ref/heads/main'],'merge_compare':compare}
            if kind=='non_descendant':
                with self.assertRaisesRegex(ValueError,'not contained|history'):
                    terminal_proof._validate_live_repository(lr,repo,merged,'main',target)
            else:self.assertEqual(terminal_proof._validate_live_repository(lr,repo,merged,'main',target),target)
            for oid in (merged,target):self.assertEqual(git('rev-parse',oid+'^{commit}'),oid)
            return {'kind':kind,'head':a['head'],'merge':merged,'target':target,'merge_base':mergebase,'ahead':ahead,'behind':behind}
        finally:git('switch','-q','--detach',original)
    def test_actual_unequal_delivered_git_patch_blocks(self):
        self.git_case('unequal_patch')
        with self.assertRaisesRegex(ValueError,'equivalence|patch|source.*diff'):self.call()
    def test_real_related_descendant_target_drift_blocks(self):
        self.git_case('related_target')
        with self.assertRaisesRegex(ValueError,'related|applicability|target.*drift'):self.call()
    def test_real_non_descendant_target_ancestry_blocks(self):
        self.git_case('non_descendant')
        with self.assertRaisesRegex(ValueError,'ancestry|not contained|history'):self.call()
    def test_real_git_negative_inputs_reach_existing_identity_and_ancestry_primitives(self):
        cases=[self.git_case(kind) for kind in ('unequal_patch','related_target','non_descendant')]
        self.assertEqual(len(cases),3)



class CurrentTargetComponent(unittest.TestCase):
    """Real descendant Git/planner and authentic PUSH transport; no validator stubs."""
    @classmethod
    def setUpClass(cls):
        MergedIntegrationComponent.setUpClass.__func__(cls)
        root=cls.review['root'];a=cls.inputs
        git=lambda *args:subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
        # Two advances ensure last-push and cumulative ranges cannot be conflated.
        rel='scripts/pm/readiness_transport.py'
        (root/rel).write_bytes((root/rel).read_bytes()+b'\n# offline related target advance\n')
        git('add',rel);git('commit','-qm','first related consumer advance')
        cls.push_base=git('rev-parse','HEAD')
        rel='scripts/pm/terminal_proof.py'
        (root/rel).write_bytes((root/rel).read_bytes()+b'\n# offline second related target advance\n')
        git('add',rel);git('commit','-qm','second related consumer advance')
        cls.target=git('rev-parse','HEAD');cls.target_tree=git('rev-parse','HEAD^{tree}')
        cls.cumulative_paths=git('diff','--name-only',a['merge'],cls.target).splitlines()
        cls.push_paths=git('diff','--name-only',cls.push_base,cls.target).splitlines()
        assert len(cls.cumulative_paths)==2 and len(cls.push_paths)==1
        planner=cls.review['module'].IMPACT_PROJECTION
        def push_planner(paths):
            args=[sys.executable,str(root/'scripts/plan-rust-required-scope.py'),'--event-name','push']
            for path in paths:args+=['--changed-path',path]
            proc=subprocess.run(args,cwd=root,text=True,capture_output=True,check=True)
            return dict(line.split('=',1) for line in proc.stdout.splitlines() if '=' in line)
        cls.cumulative_raw=push_planner(cls.cumulative_paths)
        cls.push_raw=push_planner(cls.push_paths)
        cls.receipt=load(HERE/'ci-ready-receipt.py','current_target_fixture_receipt')
        cls.receipt.canonical_planner(cls.cumulative_raw)
        cls.receipt.canonical_planner(cls.push_raw)
        git('branch','source',a['head']);git('switch','-q','source')
        git('update-ref','refs/heads/main',cls.target)
        remote=root/'.git/offline-origin.git'
        subprocess.run(['git','clone','-q','--bare',str(root),str(remote)],check=True)
        subprocess.run(['git','--git-dir='+str(remote),'symbolic-ref','HEAD','refs/heads/main'],check=True)
        git('config','remote.origin.url','https://github.com/'+a['repository']+'.git')
        git('config','url.'+str(remote)+'.insteadOf','https://github.com/'+a['repository']+'.git')
        cls.initial_gh_data=cls.review['harness'].gh_data.read_bytes()
        cls.initial_mapping=(root/'.pm/github-project-sync/tasks.json').read_bytes()
        cls.initial_gh_script=cls.review['harness'].fake_gh.read_text()
    @classmethod
    def tearDownClass(cls):
        MergedIntegrationComponent.tearDownClass.__func__(cls)
    def setUp(self):
        import copy,base64,io,zipfile
        MergedIntegrationComponent.setUp(self)
        self.review['harness'].gh_data.write_bytes(self.initial_gh_data)
        self.review['harness'].fake_gh.write_text(self.initial_gh_script)
        (self.review['root']/'.pm/github-project-sync/tasks.json').write_bytes(self.initial_mapping)
        shutil.rmtree(self.review['root']/'.git/oasis7-workflow-receipts'/self.data['task_uid'],ignore_errors=True)
        a=self.data;repo=a['repository'];t=self.target;m=a['merge']
        self.context={k:v for k,v in self.context.items() if k in {'repository_root','source_review_plan_path','source_review_handoff_path','source_review_resolution_path','source_scope_oid','check_app_id'}}
        self.responses[f'repos/{repo}/git/ref/heads/main']={'ref':'refs/heads/main','object':{'sha':t}}
        self.responses[f'repos/{repo}/compare/{m}...{t}']={'base_commit':{'sha':m},'merge_base_commit':{'sha':m},'ahead_by':2,'behind_by':0,'total_commits':2,'status':'ahead'}
        self.push_run={'id':2701,'run_attempt':1,'event':'push','head_branch':'main','head_sha':t,'path':'.github/workflows/rust.yml','repository':{'full_name':repo},'status':'completed','conclusion':'success','check_suite_id':2801,'created_at':'2026-10-02T10:00:00Z','run_started_at':'2026-10-02T10:00:00Z'}
        self.push_steps=[{'number':i,'name':name,'status':'completed','conclusion':'success','started_at':f'2026-10-02T10:0{i}:00Z','completed_at':f'2026-10-02T10:0{i}:30Z'} for i,name in enumerate(['Plan required gate scope','Write required planner artifact','Upload required planner artifact','Run required test tier'],1)]
        self.push_job={'id':2901,'run_id':2701,'run_attempt':1,'head_sha':t,'name':'required-gate','check_run_url':f'https://api.github.com/repos/{repo}/check-runs/2901','status':'completed','conclusion':'success','labels':['ubuntu-latest'],'steps':self.push_steps}
        self.push_check={'id':2901,'name':'required-gate','app':{'id':15368},'head_sha':t,'status':'completed','conclusion':'success','details_url':f'https://github.com/{repo}/actions/runs/2701/job/2901','output':{'summary':None,'text':None}}
        self.push_plan=self.produce_push_plan(self.push_raw,2701,self.push_base,t)
        self.push_artifact={'id':3101,'name':'oasis7-required-plan-v1','expired':False,'size_in_bytes':1000,'created_at':'2026-10-02T10:02:10Z','updated_at':'2026-10-02T10:03:20Z','workflow_run':{'id':2701,'repository_id':1,'head_repository_id':1,'head_branch':'main','head_sha':t}}
        self.responses.update({f'repos/{repo}/actions/workflows/rust.yml/runs?event=push&per_page=100&page=1':{'total_count':1,'workflow_runs':[self.push_run]},f'repos/{repo}/actions/runs/2701':self.push_run,f'repos/{repo}/actions/runs/2701/attempts/1':self.push_run,f'repos/{repo}/actions/runs/2701/attempts/1/jobs?per_page=100&page=1':{'total_count':1,'jobs':[self.push_job]},f'repos/{repo}/check-runs/2901':self.push_check,f'repos/{repo}/check-suites/2801/check-runs?per_page=100&page=1':{'total_count':1,'check_runs':[self.push_check]},f'repos/{repo}/actions/runs/2701/artifacts?per_page=100&page=1':{'total_count':1,'artifacts':[self.push_artifact]}})
        self.responses[f'repos/{repo}/check-suites/2801']={'id':2801,'head_branch':'main','head_sha':t,'before':self.push_base,'after':t,'status':'completed','conclusion':'success','app':{'id':15368},'repository':{'full_name':repo}}
        self.push_check['check_suite']={'id':2801}
        self.push_archive()
        self.build_source_ci_holds()
        self.prepare_full_delivery()
    publish_transport=MergedIntegrationComponent.publish_transport
    def push_archive(self):
        import io,zipfile,base64
        raw=io.BytesIO();self.plan_bytes=(json.dumps(self.push_plan,sort_keys=True,separators=(',',':'))+'\n').encode()
        with zipfile.ZipFile(raw,'w') as z:z.writestr('oasis7-required-plan-v1.json',self.plan_bytes)
        self.responses[f"repos/{self.data['repository']}/actions/artifacts/3101/zip"]={'binary_b64':base64.b64encode(raw.getvalue()).decode()}
        self.publish_transport()
    def produce_push_plan(self,raw,run_id,base,head):
        """Execute the actual immutable workflow Write step, not a lookalike."""
        import os,textwrap
        root=self.review['root']
        workflow=subprocess.check_output(['git','-C',str(root),'show',self.target+':.github/workflows/rust.yml'],text=True)
        step=workflow.split('      - name: Write required planner artifact\n',1)[1]
        code=step.split("          python3 -I - <<'PY'\n",1)[1].split('          PY\n',1)[0]
        code=textwrap.dedent(code)
        values=dict(raw,source_scope_base=base,integration_base=base,source_head=head,base_oid=base,integration_base_oid=base,head_oid=head)
        output=root/'output/required-plan';output.mkdir(parents=True,exist_ok=True)
        source=root/'offline-scope-outputs.json';source.write_text(json.dumps(values))
        env=dict(os.environ,SCOPE_OUTPUTS_PATH=str(source),REPOSITORY=self.data['repository'],WORKFLOW_RUN_ID=str(run_id),INTEGRATION_JSON='')
        subprocess.run([sys.executable,'-I','-c',code],cwd=root,env=env,check=True,capture_output=True)
        value=json.loads((output/'oasis7-required-plan-v1.json').read_text())
        self.receipt.canonical_planner(value['planner'])
        return value
    def build_source_ci_holds(self):
        import copy
        a=self.data;repo=a['repository'];h=a['head']
        self.source_run=dict(self.push_run,id=3701,head_sha=h,check_suite_id=3801,created_at='2026-09-30T10:00:00Z',run_started_at='2026-09-30T10:00:00Z')
        self.source_job=dict(self.push_job,id=3901,run_id=3701,head_sha=h,check_run_url=f'https://api.github.com/repos/{repo}/check-runs/3901')
        self.source_check=dict(self.push_check,id=3901,head_sha=h,check_suite={'id':3801},details_url=f'https://github.com/{repo}/actions/runs/3701/job/3901')
        self.source_job['steps']=[dict(step,started_at=step['started_at'].replace('2026-10-02','2026-09-30'),completed_at=step['completed_at'].replace('2026-10-02','2026-09-30')) for step in self.push_steps]
        self.source_plan=self.produce_push_plan(a['planner_raw'],3701,a['base'],h)
        self.project_item={'id':'PVTI_offline','fieldValues':{'pageInfo':{'hasNextPage':False,'endCursor':None},'nodes':[
            {'__typename':'ProjectV2ItemFieldRepositoryValue','field':{'name':'Repository'},'repository':{'nameWithOwner':repo}},
            {'__typename':'ProjectV2ItemFieldTextValue','field':{'name':'Task UID'},'text':a['task_uid']},
            {'__typename':'ProjectV2ItemFieldTextValue','field':{'name':'Canonical Worktree'},'text':str(self.review['root'])},
            {'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':'Status'},'name':'In Progress'},
            {'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':'PM Status'},'name':'committed'},
            {'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':'Workflow Phase'},'name':'verification'}]}}
        self.holds={'reviews':[],'comments':[],'threads':{'nodes':[],'pageInfo':{'hasNextPage':False,'endCursor':None}},'operator':{'login':'repo-admin'},'permission':{'permission':'admin'}}
        import io,zipfile,base64
        source_member=(json.dumps(self.source_plan,sort_keys=True,separators=(',',':'))+'\n').encode()
        archive=io.BytesIO()
        with zipfile.ZipFile(archive,'w') as z:z.writestr('oasis7-required-plan-v1.json',source_member)
        self.responses[f'repos/{repo}/actions/artifacts/4101/zip']={'binary_b64':base64.b64encode(archive.getvalue()).decode()}
        self.responses[f'repos/{repo}/actions/runs/3701/artifacts?per_page=100&page=1']={'total_count':1,'artifacts':[{'id':4101,'name':'oasis7-required-plan-v1','expired':False,'size_in_bytes':len(archive.getvalue()),'created_at':'2026-09-30T10:02:10Z','updated_at':'2026-09-30T10:03:20Z','workflow_run':{'id':3701,'head_sha':h,'head_branch':'main'}}]}
        self.responses[f'repos/{repo}/check-suites/3801']={'id':3801,'head_branch':'main','head_sha':h,'before':a['base'],'after':h,'app':{'id':15368},'repository':{'full_name':repo},'status':'completed','conclusion':'success'}
        self.responses[f'repos/{repo}/check-suites/3801/check-runs?per_page=100&page=1']={'total_count':1,'check_runs':[self.source_check]}
        self.responses[f'repos/{repo}/actions/runs/3701/attempts/1']=self.source_run
        discovery=self.responses[f'repos/{repo}/actions/workflows/rust.yml/runs?event=push&per_page=100&page=1']
        discovery['workflow_runs'].append(self.source_run);discovery['total_count']=2
        self.responses[f'repos/{repo}/actions/runs/3701']=self.source_run
        self.responses[f'repos/{repo}/actions/runs/3701/attempts/1/jobs?per_page=100&page=1']={'total_count':1,'jobs':[self.source_job]}
        self.responses[f'repos/{repo}/check-runs/3901']=self.source_check
        self.responses[f'repos/{repo}/commits/{h}/check-runs?per_page=100&page=1']={'total_count':1,'check_runs':[self.source_check]}
        self.responses[f'repos/{repo}/pulls/1/reviews?per_page=100&page=1']=[]
        self.responses[f'repos/{repo}/collaborators/repo-admin/permission']=self.holds['permission']
        self.responses['user']=self.holds['operator']
        # Transport supplies factual inputs only; no closed verification record.
        self.publish_transport()
    def prepare_full_delivery(self):
        """Produce merge receipt with real supported producer; no old claims."""
        import os
        a=self.data;root=self.review['root'];c=self.review['harness'];repo=a['repository'];uid=a['task_uid']
        self.delivery_state_path=root/'offline-delivery-state.json'
        issue_url=f"https://github.com/{repo}/issues/{a['issue']}"
        body=f"task_uid: {uid}\n- pr_number: `1`\n- pr_url: `{a['live_pr']['html_url']}`\n"
        self.project_item.update(project={'id':'PROJECT_offline','number':1,'owner':{'login':repo.split('/')[0]}},content={'number':a['issue'],'url':issue_url,'body':body})
        self.delivery_state={'repository':repo,'issue':{'number':a['issue'],'html_url':issue_url,'url':f"https://api.github.com/repos/{repo}/issues/{a['issue']}",'body':body,'state':'OPEN'},'pr':a['live_pr'],'project_item':self.project_item,'comments':[]}
        self.delivery_state_path.write_text(json.dumps(self.delivery_state))
        live=json.loads(c.gh_data.read_text());live['issue']=self.delivery_state['issue']
        # Actual review/dispatch/resolution comments remain; no historical
        # readiness/completion entry is removed or invented.
        auth={'id':9001,'body':'<!-- oasis7-terminal-recovery-authorization/v1 -->\n'+json.dumps(a['authorization'],sort_keys=True,separators=(',',':')),'user':{'login':'repo-admin'},'html_url':issue_url+'#issuecomment-9001','issue_url':f"https://api.github.com/repos/{repo}/issues/{a['issue']}",'created_at':'2026-10-02T10:00:00Z','updated_at':'2026-10-02T10:00:00Z'}
        live['comment_pages'][-1].append(auth);c.gh_data.write_text(json.dumps(live))
        original=c.fake_gh.read_text()
        prefix=("#!/usr/bin/env python3\nimport json,sys\nstate=json.load(open("+repr(str(self.delivery_state_path))+"))\nargs=sys.argv[1:]\n"
            +"if args[:2]==['pr','view']:\n p=state['pr'];print(json.dumps({'number':p['number'],'url':p['html_url'],'state':'MERGED' if p.get('merged') else p['state'].upper(),'mergedAt':p['merged_at'],'headRefOid':p['head']['sha'],'baseRefName':p['base']['ref']}));raise SystemExit(0)\n"
            +"if args[:2]==['repo','view']:\n print(json.dumps({'nameWithOwner':state['repository'],'defaultBranchRef':{'name':'main'}}));raise SystemExit(0)\n"
            +"exec("+repr(original)+")\n")
        # Durable official-shaped transport effect engine. It never returns a
        # validator verdict and unknown operations refuse instead of guessing.
        effects=r"""
import datetime,os
path=STATE_PATH
args=sys.argv[1:]
def save():
    with open(path,'w') as out:json.dump(state,out)
def fields():
    return {args[i+1].split('=',1)[0]:args[i+1].split('=',1)[1] for i,x in enumerate(args[:-1]) if x in ('-f','-F','--field','--raw-field') and '=' in args[i+1]}
def emit(value):
    print(json.dumps(value));raise SystemExit(0)
endpoint=next((x for x in args if x.startswith('repos/')),None)
method=next((args[i+1] for i,x in enumerate(args[:-1]) if x=='--method'),'GET')
repo=state['repository'];issue=state['issue']['number']
if args[:2]==['api','user']:
    if '--jq' in args and args[args.index('--jq')+1]=='.login':print('repo-admin');raise SystemExit(0)
    emit({'login':'repo-admin'})
if args[:2]==['api','graphql']:
    inputs=fields();query=inputs.get('query','')
    if 'updateProjectV2ItemFieldValue' in query:
        item=inputs.get('itemId') or inputs.get('item')
        project=inputs.get('projectId') or inputs.get('project')
        if item!=state['project_item']['id'] or project!=state['project_item']['project']['id']:
            raise SystemExit('unprovided Project mutation item/project identity')
        governed={'FIELD_STATUS':('Status',{'STATUS_COMMITTED':'In Progress','STATUS_DONE':'Done'}),
            'FIELD_PM':('PM Status',{'PM_COMMITTED':'committed','PM_DONE':'done'}),
            'FIELD_PHASE':('Workflow Phase',{'PHASE_VERIFY':'verification','PHASE_TASK_DONE':'task_done','PHASE_DONE':'done','PHASE_POST_MERGE_DONE':'post_merge_done'})}
        # Real sync uses aliased fieldN/optionN batch variables. Validate every
        # binding before any simulated server effect; no false acknowledgment.
        indexes=sorted(int(k[5:]) for k in inputs if k.startswith('field') and k[5:].isdigit())
        requests=[('f'+str(i),inputs['field'+str(i)],inputs.get('option'+str(i))) for i in indexes]
        if not requests:requests=[('updateProjectV2ItemFieldValue',inputs.get('fieldId') or inputs.get('field'),inputs.get('optionId') or inputs.get('option'))]
        updates=[]
        for alias,field,option in requests:
            if field not in governed or option not in governed[field][1]:
                raise SystemExit('unprovided Project mutation field/option identity')
            name,options=governed[field];updates.append((alias,name,options[option]))
        for alias,name,value in updates:
            matches=[node for node in state['project_item']['fieldValues']['nodes'] if node['field']['name']==name]
            if len(matches)>1:raise SystemExit('ambiguous Project mutation field')
            if matches:matches[0]['name']=value
            else:state['project_item']['fieldValues']['nodes'].append({'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':name},'name':value})
        save();emit({'data':{alias:{'projectV2Item':{'id':state['project_item']['id']}} for alias,name,value in updates}})
    if 'mutation' in query:raise SystemExit('unprovided GraphQL mutation')
    project={'id':'PROJECT_offline','number':1,'fields':{'nodes':[
        {'__typename':'ProjectV2SingleSelectField','id':fid,'name':name,'options':[{'id':oid,'name':label} for oid,label in options]}
        for fid,name,options in [('FIELD_STATUS','Status',[('STATUS_COMMITTED','In Progress'),('STATUS_DONE','Done')]),('FIELD_PM','PM Status',[('PM_COMMITTED','committed'),('PM_DONE','done')]),('FIELD_PHASE','Workflow Phase',[('PHASE_VERIFY','verification'),('PHASE_TASK_DONE','task_done'),('PHASE_DONE','done'),('PHASE_POST_MERGE_DONE','post_merge_done')])]],'pageInfo':{'hasNextPage':False}}}
    connection={'nodes':[state['project_item']],'pageInfo':{'hasNextPage':False,'endCursor':None}}
    emit({'data':{'node':state['project_item'],'viewer':{'login':'repo-admin'},'user':{'projectV2':project},'organization':{'projectV2':project},'repository':{'issue':dict(state['issue'],projectItems=connection),'pullRequest':{'reviewThreads':{'nodes':[],'pageInfo':{'hasNextPage':False,'endCursor':None}}}}}})
if endpoint==f'repos/{repo}/issues/{issue}' and method in ('PATCH','POST'):
    state['issue'].update(fields());save();emit(state['issue'])
if endpoint==f'repos/{repo}/issues/{issue}':emit(state['issue'])
if (args[:2]==['issue','comment'] or endpoint==f'repos/{repo}/issues/{issue}/comments' and method=='POST'):
    data=json.load(open(os.environ['GH_FIXTURE']));inputs=fields()
    body=inputs.get('body')
    if '--body-file' in args:body=open(args[args.index('--body-file')+1]).read()
    if body is None:raise SystemExit('missing actual comment body')
    stamp=datetime.datetime.now(datetime.timezone.utc).isoformat()
    ident=max([c['id'] for page in data['comment_pages'] for c in page]+[9001])+1
    comment={'id':ident,'body':body,'user':{'login':'repo-admin'},'created_at':stamp,'updated_at':stamp,'issue_url':f'https://api.github.com/repos/{repo}/issues/{issue}','html_url':f'https://github.com/{repo}/issues/{issue}#issuecomment-{ident}'}
    data['comment_pages'][-1].append(comment)
    with open(os.environ['GH_FIXTURE'],'w') as out:json.dump(data,out)
    emit(comment)
if args[:2]==['issue','close']:
    state['issue'].update(state='closed',state_reason='completed');save();emit(state['issue'])
"""
        prefix=prefix.replace('exec('+repr(original)+')\n',effects.replace('STATE_PATH',repr(str(self.delivery_state_path)))+'\nexec('+repr(original)+')\n')
        c.fake_gh.write_text(prefix);c.fake_gh.chmod(0o755)
        result=subprocess.run([sys.executable,str(root/'scripts/pm/pr-merge-receipt.py'),'1','--json'],text=True,capture_output=True,check=True)
        self.merge_receipt_bytes=result.stdout.encode();self.merge_receipt=json.loads(result.stdout)
        raw=subprocess.check_output([sys.executable,str(root/'scripts/pm/canonical-receipt-root.py'),'--default-worktree',str(root),'--task-uid',uid,'--create','--json'],text=True)
        self.receipt_root=Path(json.loads(raw)['receipt_root'])
        (self.receipt_root/'merge-receipt.json').write_bytes(self.merge_receipt_bytes)
        mapping_path=root/'.pm/github-project-sync/tasks.json';mapping=json.loads(mapping_path.read_text())
        record=mapping['tasks'][uid]
        record.update(task_uid=uid,repository=repo,status='committed',workflow_phase='verification',completion_mode='single_pr',issue_number=a['issue'],issue_url=issue_url,pr_number=1,pr_url=a['live_pr']['html_url'],canonical_worktree=str(root),task_branch='source',default_branch='main',owner_role='repository_health_engineer',project_item_id='PVTI_offline',merge_receipt=self.merge_receipt,merge_receipt_sha256=hashlib.sha256(self.merge_receipt_bytes).hexdigest())
        record.pop('claim_verifications',None)
        mapping['project']={'repo':repo,'owner':repo.split('/')[0],'number':1,'id':'PROJECT_offline'}
        mapping_path.write_text(json.dumps(mapping))
    def test_real_merge_producer_accepts_truthful_merged_without_readiness_or_completion(self):
        finalizer=load(HERE/'post-merge-finalize.py','recovery_merge_receipt_primitives')
        mapping=json.loads((self.review['root']/'.pm/github-project-sync/tasks.json').read_text())
        record=mapping['tasks'][self.data['task_uid']]
        finalizer._delivery_mapping_receipt(record,self.receipt_root,self.data['repository'],1,self.data['live_pr']['html_url'],self.data['head'],self.data['merge'],'main')
        self.assertEqual(self.merge_receipt['evidence_mode'],'production')
        self.assertEqual(self.merge_receipt['state'],'MERGED')
        self.assertNotIn('claim_verifications',record)
        self.assertEqual(subprocess.check_output(['git','-C',str(self.review['root']),'rev-parse','HEAD'],text=True).strip(),self.data['head'])
        self.assertEqual(subprocess.check_output(['git','-C',str(self.review['root']),'branch','--show-current'],text=True).strip(),record['task_branch'])
        self.assertFalse((self.receipt_root/'readiness-proof.json').exists())
        live=json.loads(self.review['harness'].gh_data.read_text())
        bodies=[comment['body'] for page in live['comment_pages'] for comment in page]
        self.assertFalse(any('oasis7-native-readiness/v2' in b or 'oasis7-pr-readiness-migration/v1' in b or 'claim_type: `task_complete`' in b for b in bodies))
    def test_source_H_ci_and_factual_hold_inputs_reach_neutral_primitives(self):
        import copy,loop_terminal
        a=self.data;gate=load(HERE/'pr-lifecycle-gate.py','source_hold_fixture_gate')
        self.assertEqual(self.receipt._job_identity(self.source_job,a['repository'],3701,1,field='source H')['head_sha'],a['head'])
        self.assertEqual(self.receipt._workflow_job_details(self.source_check,a['repository']),(3701,3901))
        self.receipt.canonical_planner(self.source_plan['planner'])
        fields=loop_terminal.normalize_project_fields(self.project_item,a['repository'])
        self.assertEqual(fields['Task UID'],a['task_uid']);self.assertEqual(fields['PM Status'],'committed')
        self.assertEqual(gate.latest_reviews(self.holds['reviews']),[])
        self.assertFalse(gate.actionable('status: bounded recovery observation'))
        self.assertTrue(gate.actionable('must fix before merge'))
        changes={'id':71,'author':{'login':'reviewer'},'state':'CHANGES_REQUESTED','submittedAt':'2026-10-02T10:00:00Z'}
        self.assertEqual(gate.latest_reviews([changes])[0]['state'],'CHANGES_REQUESTED')
        bad=copy.deepcopy(self.project_item);bad['fieldValues']['pageInfo']['hasNextPage']=True
        with self.assertRaisesRegex(ValueError,'pagination'):loop_terminal.normalize_project_fields(bad,a['repository'])
        self.assertEqual(self.holds['permission'],{'permission':'admin'})
        self.assertEqual(self.responses[f"repos/{a['repository']}/pulls/1"]['state'],'closed')
    def test_readonly_full_recovery_preflight_returns_eight_records_without_effects(self):
        # A prospective observation must traverse new collectors; parser support
        # is currently absent. Empty verdict dictionaries are never injected.
        root=self.review['root'];uid=self.data['task_uid']
        mapping=root/'.pm/github-project-sync/tasks.json'
        value=json.loads(mapping.read_text());record=value['tasks'][uid]
        record.update(pr_number=1,pr_url=self.data['live_pr']['html_url'],status='committed',workflow_phase='verification',canonical_worktree=str(root))
        record.pop('claim_verifications',None);mapping.write_text(json.dumps(value))
        receipt_root=self.receipt_root
        self.assertFalse((receipt_root/'readiness-proof.json').exists())
        self.assertNotIn('claim_verifications',record)
        before=mapping.read_bytes()
        before_server=self.review['harness'].gh_data.read_bytes()
        before_state=self.delivery_state_path.read_bytes()
        before_artifacts={str(p.relative_to(receipt_root)):p.read_bytes() for p in receipt_root.rglob('*') if p.is_file()}
        proc=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(root),'--task-uid',uid,'--recover-merged-delivery'],text=True,capture_output=True,timeout=330)
        self.assertEqual(mapping.read_bytes(),before)
        self.assertEqual(self.review['harness'].gh_data.read_bytes(),before_server)
        self.assertEqual(self.delivery_state_path.read_bytes(),before_state)
        self.assertEqual({str(p.relative_to(receipt_root)):p.read_bytes() for p in receipt_root.rglob('*') if p.is_file()},before_artifacts)
        self.assertEqual(proc.returncode,0,proc.stdout+proc.stderr)
        proof=json.loads(proc.stdout)
        self.assertEqual(proof['schema'],'oasis7-postmerge-delivery-proof/v1')
        self.assertEqual(set(proof['verification']),{'source_review','required_ci','integration','applicability','source_equivalence','merge_readback','current_holds','current_target_ci'})
        self.assertEqual(proof['observed_target_oid'],self.target)
        self.assertEqual(proof['prior_readiness']['status'],'unavailable')
        self.assertEqual(mapping.read_bytes(),before)
        self.assertFalse((receipt_root/'readiness-proof.json').exists())
    def call_target(self):
        import integration_ci
        self.publish_transport()
        before=self.effect_snapshot()
        try:
            reader=getattr(integration_ci,'verify_current_target_ci',None)
            self.assertTrue(callable(reader),'unsupported admitted explicit current-target CI reader')
            a=self.data
            return reader(a['repository'],a['task_uid'],a['pr'],a['head'],a['merge'],self.target,self.context)
        finally:self.assertEqual(self.effect_snapshot(),before,'current-target reader must preserve every effect sink')
    def test_real_descendant_two_range_planner_job_and_policy_primitives(self):
        import integration_ci
        a=self.data;root=self.review['root']
        self.assertNotEqual(self.target,a['merge']);self.assertNotEqual(self.push_base,a['merge'])
        self.assertEqual(subprocess.check_output(['git','-C',str(root),'merge-base',a['merge'],self.target],text=True).strip(),a['merge'])
        self.assertEqual(set(self.push_plan),{'schema','repository','workflow_run_id','head_oid','base_oid','integration_base_oid','check_name','planner'})
        actual=self.receipt.canonical_planner(self.push_plan['planner']);expected=self.receipt.canonical_planner(self.cumulative_raw)
        for key,value in expected.items():
            if key.startswith(('run_','needs_')) and value:self.assertTrue(actual[key],key)
        identity=self.receipt._job_identity(self.push_job,a['repository'],2701,1,field='actual PUSH')
        self.assertEqual(identity['check_run_id'],2901)
        self.assertEqual(identity['head_sha'],self.target)
        self.assertEqual(self.receipt._workflow_job_details(self.push_check,a['repository']),(2701,2901))
        with self.assertRaisesRegex(ValueError,'workflow job attempt provenance mismatch'):
            integration_ci.attempt_execution_jobs(a['repository'],2701,1,self.target,15368,require_completed=True)
        gate=load(HERE/'pr-lifecycle-gate.py','target_fixture_policy_gate')
        class ReadOnlyTransport:
            def rest(self,method,path,**kwargs):
                assert method=='GET'
                return integration_ci.gh('api',path)
        policy=gate.discover_required_policy(a['repository'],'main',client=ReadOnlyTransport())
        self.assertEqual(policy['status'],'resolved')
        self.assertEqual(policy['required_status_checks'],[{'context':'required-gate','app_id':15368}])
        # Complete policy is also exercised by the preserved merged component controls.
        self.assertEqual(self.responses[f"repos/{a['repository']}/branches/main/protection"]['required_status_checks']['checks'],[{'context':'required-gate','app_id':15368}])
        self.assertEqual(self.push_check['output'],{'summary':None,'text':None})
        self.assertEqual(self.push_steps[1]['started_at'],'2026-10-02T10:02:00Z')
        self.assertLess(self.push_artifact['updated_at'],self.push_steps[2]['completed_at'])
        suite=self.responses[f"repos/{a['repository']}/check-suites/2801"]
        self.assertEqual(suite['before'],self.push_base);self.assertEqual(suite['after'],self.target)
        self.assertEqual(suite['id'],self.push_run['check_suite_id']);self.assertEqual(suite['app']['id'],15368)
        self.assertEqual(len(self.cumulative_paths),2);self.assertEqual(len(self.push_paths),1)
    def test_api_shaped_job_negative_reaches_actual_identity_validator(self):
        import copy
        bad=copy.deepcopy(self.push_job);bad['run_attempt']=2
        with self.assertRaisesRegex(SystemExit,'wrong workflow attempt'):
            self.receipt._job_identity(bad,self.data['repository'],2701,1,field='actual PUSH')
        bad=copy.deepcopy(self.push_job);bad['check_run_url']='https://api.github.com/repos/foreign/repo/check-runs/2901'
        with self.assertRaisesRegex(SystemExit,'check-run URL'):
            self.receipt._job_identity(bad,self.data['repository'],2701,1,field='actual PUSH')
    def test_current_target_returns_exact_closed_record(self):
        v=self.call_target()
        self.assertEqual(set(v),{'schema','repository','task_uid','head_oid','observed_at','evidence','target_oid','target_tree_oid','default_branch','workflow_path','workflow_sha','event','plan','plan_sha256','policy_sha256','checks','coverage'})
        self.assertEqual(v['schema'],'oasis7-terminal-recovery-current_target_ci/v1');self.assertEqual(v['head_oid'],self.data['head'])
        self.assertEqual(v['target_oid'],self.target);self.assertEqual(v['target_tree_oid'],self.target_tree);self.assertEqual(v['workflow_sha'],self.target);self.assertEqual(v['event'],'push')
        self.assertEqual(v['plan'],self.push_plan);self.assertEqual(v['plan_sha256'],hashlib.sha256(self.plan_bytes).hexdigest())
        self.assertEqual(set(v['coverage']),{'range_base_oid','range_head_oid','changed_paths','expected_selectors','actual_selectors','expected_resources','actual_resources','planner_entry_sha256','dispatcher_sha256','inventory_sha256','steps'})
        self.assertEqual(v['coverage']['range_base_oid'],self.data['merge']);self.assertEqual(v['coverage']['range_head_oid'],self.target)
        self.assertEqual(v['coverage']['changed_paths'],sorted(self.cumulative_paths))
        for expected,actual in [('expected_selectors','actual_selectors'),('expected_resources','actual_resources')]:
            for key,value in v['coverage'][expected].items():
                self.assertIs(type(value),bool)
                if value:self.assertIs(v['coverage'][actual][key],True)
    def test_missing_required_selector_resource_and_incomplete_discovery_block(self):
        import copy
        original=copy.deepcopy(self.push_plan)
        for field in ('run_required_gate_baseline','needs_rust_toolchain'):
            with self.subTest(field=field):
                self.push_plan=copy.deepcopy(original);self.push_plan['planner'][field]='false';self.push_archive()
                with self.assertRaisesRegex(ValueError,'plan|selector|resource|baseline|coverage'):self.call_target()
        self.push_plan=original;self.push_archive()
        key=f"repos/{self.data['repository']}/actions/workflows/rust.yml/runs?event=push&per_page=100&page=1"
        self.responses[key]['workflow_runs']=None
        with self.assertRaisesRegex(ValueError,'discovery|malformed|pagination'):self.call_target()
    def test_push_check_app_and_job_attempt_provenance_blocks(self):
        self.push_check['app']['id']=99
        with self.assertRaisesRegex(ValueError,'app|identity|check'):self.call_target()
    def test_source_H_check_cannot_be_replaced_by_target_T_check(self):
        self.source_check['head_sha']=self.target;self.publish_transport()
        before=self.effect_snapshot()
        proc=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(self.review['root']),'--task-uid',self.data['task_uid'],'--recover-merged-delivery'],text=True,capture_output=True,timeout=330)
        self.assertNotEqual(proc.returncode,0)
        self.assertEqual(self.effect_snapshot(),before)
        self.assertNotIn('unrecognized arguments',proc.stderr,'must reach separate accepted-H CI collector')
        self.assertRegex(proc.stdout+proc.stderr,'head|source|check|identity')
    def test_plan_base_must_match_independent_check_suite_before(self):
        self.push_plan['base_oid']=self.data['merge'];self.push_archive()
        with self.assertRaisesRegex(ValueError,'base|plan|provenance|range'):self.call_target()
    def test_unavailable_independent_push_base_blocks(self):
        self.responses[f"repos/{self.data['repository']}/check-suites/2801"]['before']=None
        with self.assertRaisesRegex(ValueError,'base|before|provenance|unsupported'):self.call_target()
    def test_durable_Project_transport_effect_readback_reaches_real_normalizer(self):
        import loop_terminal
        def read():
            raw=json.loads(subprocess.check_output(['gh','api','graphql','-f','query=query { node(id:"PVTI_offline") { ... on ProjectV2Item { id } } }'],text=True))
            self.assertEqual(json.loads(self.delivery_state_path.read_text())['project_item'],raw['data']['node'])
            return loop_terminal.normalize_project_fields(raw['data']['node'],self.data['repository'])
        initial=read()
        self.assertEqual((initial['Status'],initial['PM Status'],initial['Workflow Phase']),('In Progress','committed','verification'))
        # Exercise genuine unset-field creation, not just updating existing nodes.
        state=json.loads(self.delivery_state_path.read_text())
        state['project_item']['fieldValues']['nodes']=[node for node in state['project_item']['fieldValues']['nodes'] if node['field']['name']!='Status']
        self.delivery_state_path.write_text(json.dumps(state))
        for field,option in [('FIELD_STATUS','STATUS_DONE'),('FIELD_PM','PM_DONE'),('FIELD_PHASE','PHASE_DONE')]:
            response=json.loads(subprocess.check_output(['gh','api','graphql','-f','query=mutation { updateProjectV2ItemFieldValue(input:{}) { projectV2Item { id } } }','-F','projectId=PROJECT_offline','-F','itemId=PVTI_offline','-F','fieldId='+field,'-F','optionId='+option],text=True))
            self.assertEqual(response['data']['updateProjectV2ItemFieldValue']['projectV2Item']['id'],'PVTI_offline')
        final=read()
        self.assertEqual((final['Status'],final['PM Status'],final['Workflow Phase']),('Done','done','done'))
        before=self.effect_snapshot()
        for item,field,option in [('wrong_item','FIELD_STATUS','STATUS_DONE'),('PVTI_offline','unknown_field','STATUS_DONE'),('PVTI_offline','FIELD_STATUS','unknown_option'),('PVTI_offline','FIELD_STATUS','PM_DONE')]:
            with self.subTest(item=item,field=field,option=option):
                proc=subprocess.run(['gh','api','graphql','-f','query=mutation { updateProjectV2ItemFieldValue(input:{}) { projectV2Item { id } } }','-F','projectId=PROJECT_offline','-F','itemId='+item,'-F','fieldId='+field,'-F','optionId='+option],text=True,capture_output=True)
                self.assertNotEqual(proc.returncode,0,proc.stdout+proc.stderr)
                self.assertIn('identity',proc.stderr)
                self.assertEqual(self.effect_snapshot(),before)
    def effect_snapshot(self):
        root=self.review['root']
        return {'mapping':(root/'.pm/github-project-sync/tasks.json').read_bytes(),
            'server_comments':self.review['harness'].gh_data.read_bytes(),
            'server_state':self.delivery_state_path.read_bytes(),
            'task_files':{str(p.relative_to(root/'.pm')):p.read_bytes() for p in (root/'.pm').rglob('*') if p.is_file()},
            'receipt_files':{str(p.relative_to(self.receipt_root)):p.read_bytes() for p in self.receipt_root.rglob('*') if p.is_file()}}
    def test_full_create_completion_done_finalizer_chain_is_current_typed_authority(self):
        import terminal_proof
        root=self.review['root'];uid=self.data['task_uid']
        proc=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(root),'--task-uid',uid,'--recover-merged-delivery','--create'],text=True,capture_output=True,timeout=330)
        self.assertEqual(proc.returncode,0,proc.stdout+proc.stderr)
        pub=json.loads(proc.stdout)
        self.assertEqual(set(pub),{'schema','task_uid','recovery_proof_sha256','recovery_proof_comment_id','completion_claim_sha256','completion_comment_id'})
        self.assertEqual(pub['schema'],'oasis7-postmerge-recovery-publication/v1');self.assertEqual(pub['task_uid'],uid)
        for key in ('recovery_proof_comment_id','completion_comment_id'):
            self.assertIs(type(pub[key]),int);self.assertGreater(pub[key],0)
        for key in ('recovery_proof_sha256','completion_claim_sha256'):self.assertRegex(pub[key],r'^[0-9a-f]{64}$')
        before_comments=json.loads(self.review['harness'].gh_data.read_text())['comment_pages']
        repeated=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(root),'--task-uid',uid,'--recover-merged-delivery','--create'],text=True,capture_output=True,timeout=330)
        self.assertEqual(repeated.returncode,0,repeated.stdout+repeated.stderr)
        self.assertEqual(json.loads(repeated.stdout),pub)
        self.assertEqual(json.loads(self.review['harness'].gh_data.read_text())['comment_pages'],before_comments)
        close=subprocess.run(['bash',str(root/'scripts/pm/task-closeout.sh'),'--task-uid',uid,'--to-status','done','--verification-profile','postmerge_delivery_recovery','--claim-type','postmerge_delivery_complete'],cwd=root,text=True,capture_output=True,timeout=330)
        self.assertEqual(close.returncode,0,close.stdout+close.stderr)
        mapping=json.loads((root/'.pm/github-project-sync/tasks.json').read_text());record=mapping['tasks'][uid]
        self.assertEqual(record['status'],'done');self.assertEqual(record['workflow_phase'],'task_done')
        self.assertNotIn('claim_verifications',record,'current recovery must not manufacture ordinary historical claims')
        final=subprocess.run([sys.executable,str(HERE/'post-merge-finalize.py'),'--repo-root',str(root),'--task-uid',uid,'--delivery','--json'],text=True,capture_output=True,timeout=330)
        self.assertEqual(final.returncode,0,final.stdout+final.stderr)
        receipt=json.loads((self.receipt_root/'terminal-delivery-receipt.json').read_text())
        self.assertEqual(receipt['receipt_type'],'oasis7_terminal_recovery_delivery')
        self.assertEqual(receipt['schema_version'],1);self.assertEqual(receipt['observed_target_oid'],self.target)
        self.assertEqual(receipt['recovery_proof_sha256'],pub['recovery_proof_sha256'])
        self.assertTrue((self.receipt_root/'finalizer-ledger.json').exists());self.assertTrue((self.receipt_root/'terminal-tombstone.json').exists())
        # Shared selector must independently consume exact typed readbacks.
        import loop_terminal
        proof=loop_terminal.read_shared_terminal_proof(self.data['repository'],uid,repo_root=root)
        self.assertTrue(proof,proof)
    def test_duplicate_current_authorization_rejects_before_publication_effects(self):
        c=self.review['harness'];live=json.loads(c.gh_data.read_text())
        auth=next(comment for page in live['comment_pages'] for comment in page if comment['id']==9001)
        live['comment_pages'][-1].append(dict(auth,id=9002));c.gh_data.write_text(json.dumps(live))
        before=self.effect_snapshot()
        proc=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(self.review['root']),'--task-uid',self.data['task_uid'],'--recover-merged-delivery','--create'],text=True,capture_output=True,timeout=330)
        self.assertNotEqual(proc.returncode,0)
        self.assertEqual(self.effect_snapshot(),before)
        self.assertNotIn('unrecognized arguments',proc.stderr,'semantic rejection must reach recovery collector')
        self.assertRegex(proc.stdout+proc.stderr,'authorization|ambiguous|duplicate')
    def test_native_candidate_corruption_is_not_an_absence_fallback(self):
        (self.receipt_root/'readiness-proof.json').write_text('{corrupt')
        before=self.effect_snapshot()
        proc=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(self.review['root']),'--task-uid',self.data['task_uid'],'--recover-merged-delivery','--create'],text=True,capture_output=True,timeout=330)
        self.assertNotEqual(proc.returncode,0)
        self.assertEqual(self.effect_snapshot(),before)
        self.assertNotIn('unrecognized arguments',proc.stderr,'corruption rejection must reach recovery collector')
        self.assertRegex(proc.stdout+proc.stderr,'candidate|corrupt|invalid|absence')
    def test_attempt_two_cannot_borrow_first_artifact(self):
        self.push_run['run_attempt']=2
        with self.assertRaisesRegex(ValueError,'attempt|unsupported'):self.call_target()
    def test_artifact_outside_write_upload_window_blocks(self):
        self.push_artifact['created_at']='2026-10-02T09:59:59Z'
        with self.assertRaisesRegex(ValueError,'artifact|timing|timestamp|window'):self.call_target()
    def test_skipped_applicable_execution_blocks(self):
        self.push_steps[-1].update(conclusion='skipped',status='completed')
        with self.assertRaisesRegex(ValueError,'step|execution|tier|success'):self.call_target()
    def test_context_caller_coverage_is_not_authority(self):
        self.context['coverage']={'verified':True}
        with self.assertRaisesRegex(ValueError,'context|closed|unknown'):self.call_target()
    def test_plan_payload_cannot_invent_attempt_or_rewrite_push_base(self):
        self.push_plan['run_attempt']=1;self.push_archive()
        with self.assertRaisesRegex(ValueError,'plan|schema|closed|field'):self.call_target()
    def test_newer_failed_exact_target_does_not_borrow_green(self):
        newer=dict(self.push_run,id=2702,status='completed',conclusion='failure',created_at='2026-10-02T11:00:00Z')
        key=f"repos/{self.data['repository']}/actions/workflows/rust.yml/runs?event=push&per_page=100&page=1"
        self.responses[key]['workflow_runs'].append(newer);self.responses[key]['total_count']=2
        self.responses[f"repos/{self.data['repository']}/actions/runs/2702"]=newer
        with self.assertRaisesRegex(ValueError,'latest|success|failed|conclusion'):self.call_target()

if __name__ == '__main__':
    unittest.main(verbosity=2)
