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
 evidence=c.root/'closure-evidence.json'
 evidence.write_text(json.dumps({'scope':'offline workflow fixture','consumers':['review','ci','terminal'],'classification':'mixed','reason':'all fixture consumers explicitly enumerated'}))

 for rel in ('scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/ci-tests.sh',*[f'.agents/roles/{r}.md' for r in roles],'doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)

 for rel in ('scripts/pm/readiness_transport.py','scripts/pm/loop_terminal.py','scripts/pm/terminal_proof.py','scripts/pm/post-merge-finalize.py','scripts/pm/pr-lifecycle-gate.py','scripts/pm/claim-ready.sh','scripts/pm/ci_reuse_policy.py'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)
 subprocess.run(['git','-C',str(c.root),'add','closure-evidence.json','scripts','.agents','doc'],check=True)
 subprocess.run(['git','-C',str(c.root),'commit','--amend','--no-edit','-q'],check=True)
 m.HEAD=subprocess.check_output(['git','-C',str(c.root),'rev-parse','HEAD'],text=True).strip()
 paths=subprocess.check_output(['git','-C',str(c.root),'diff','--name-only',m.SCOPE_OID,m.HEAD],text=True).splitlines()
 f=c.make_fixture(create_handoff=False,publish_dispatch=False,changed_paths=paths)
 old=f['plan']['impact_projection']
 inp={k:old[k] for k in ('task_uid','source_head_oid','scope_base_oid','changed_paths','test_profile','declared_tests','consumed_contracts','public_semantics','affected_consumers')}
 inp.update(change_class='mixed',manual_roles=roles,domain_role=None,verification_affected=True,closure_status={'status':'complete','reason':'finite offline fixture consumer closure','evidence':[{'path':str(evidence),'sha256':'sha256:'+hashlib.sha256(evidence.read_bytes()).hexdigest()}]})
 
 for rel in ('scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/ci-tests.sh',*[f'.agents/roles/{r}.md' for r in roles],'doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'):
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

if __name__ == '__main__':
    unittest.main(verbosity=2)
