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

def install_historical_snapshot(root, snapshot):
 import base64
 expected={'.github/workflows/rust.yml','scripts/ci-tests.sh','scripts/ci-required-capability-test-inventory.tsv','scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/pm/workflow-impact-projection.py','scripts/pm/review-role-selector.py','scripts/viewer-dependency-preflight.sh'}
 assert snapshot['schema']=='oasis7-offline-historical-producer-fixture/v1'
 assert snapshot['source_oid']=='2caabf1657e104c758b80dc5e2bdd556819eee75'
 assert {row['path'] for row in snapshot['files']}==expected and len(snapshot['files'])==len(expected)
 for row in snapshot['files']:
  raw=base64.b64decode(row['bytes_b64'],validate=True)
  assert len(raw)==row['size'] and hashlib.sha256(raw).hexdigest()==row['sha256']
  assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==row['blob_oid']
  assert row['mode'] in ('100644','100755')
  dest=Path(root)/row['path'];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(raw);dest.chmod(0o755 if row['mode']=='100755' else 0o644)


def build_review(repo, producer_snapshot=None, source_producer_change=None):
 repo=Path(repo).resolve();sys.path.insert(0,str(repo/'scripts/pm'))
 m=load(repo/'scripts/pm/review_preflight_handoff.test.py','full_review_fixture_harness')
 c=m.ReviewPreflightHandoffTests();c.setUp()
 subprocess.run(['git','-C',str(c.root),'switch','-q','--detach',m.SCOPE_OID],check=True)
 roles=['repository_health_engineer','producer_system_designer','qa_engineer','liveops_community']
 shutil.copytree(repo/'scripts/pm',c.root/'scripts/pm',dirs_exist_ok=True,ignore=shutil.ignore_patterns('__pycache__','*.pyc'))
 evidence=c.root/'closure-evidence.json'
 evidence.write_text(json.dumps({'scope':'offline workflow fixture','consumers':['review','ci','terminal'],'classification':'mixed','reason':'all fixture consumers explicitly enumerated'}))

 for rel in ('scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/ci-tests.sh','scripts/ci-required-capability-test-inventory.tsv','scripts/product_doc_markdown.py','scripts/doc-governance-requirements.txt','.github/workflows/rust.yml','scripts/pm/ci_required_inventory.py','scripts/pm/pr-merge-receipt.py','scripts/pm/task-closeout.sh',*[f'.agents/roles/{r}.md' for r in roles],'doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)

 for rel in ('scripts/pm/readiness_transport.py','scripts/pm/loop_terminal.py','scripts/pm/terminal_proof.py','scripts/pm/post-merge-finalize.py','scripts/pm/pr-lifecycle-gate.py','scripts/pm/claim-ready.sh','scripts/pm/ci_reuse_policy.py'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)
 if producer_snapshot is not None: install_historical_snapshot(c.root,producer_snapshot)
 subprocess.run(['git','-C',str(c.root),'add','closure-evidence.json','scripts','.agents','.github','doc'],check=True)
 subprocess.run(['git','-C',str(c.root),'commit','-qm','immutable common dependency baseline'],check=True)
 m.SCOPE_OID=subprocess.check_output(['git','-C',str(c.root),'rev-parse','HEAD'],text=True).strip()
 source=c.root/'scripts/pm/review_preflight_handoff.test.py'
 source.write_text(source.read_text()+"\ndef offline_recovery_fixture_identity(value):\n    return value\n")
 if source_producer_change is not None:
  rel,extra=source_producer_change;dest=c.root/rel;dest.write_bytes(dest.read_bytes()+extra)
  subprocess.run(['git','-C',str(c.root),'add',rel],check=True)
 subprocess.run(['git','-C',str(c.root),'add',str(source)],check=True)
 subprocess.run(['git','-C',str(c.root),'commit','-qm','bounded accepted PM source behavior'],check=True)
 m.HEAD=subprocess.check_output(['git','-C',str(c.root),'rev-parse','HEAD'],text=True).strip()
 paths=subprocess.check_output(['git','-C',str(c.root),'diff','--name-only',m.SCOPE_OID,m.HEAD],text=True).splitlines()
 f=c.make_fixture(create_handoff=False,publish_dispatch=False,changed_paths=paths)
 old=f['plan']['impact_projection']
 inp={k:old[k] for k in ('task_uid','source_head_oid','scope_base_oid','changed_paths','test_profile','declared_tests','consumed_contracts','public_semantics','affected_consumers')}
 inp.update(change_class='mixed',manual_roles=roles,domain_role=None,verification_affected=True,closure_status={'status':'complete','reason':'finite offline fixture consumer closure','evidence':[{'path':str(evidence),'sha256':'sha256:'+hashlib.sha256(evidence.read_bytes()).hexdigest()}]})
 
 for rel in ('scripts/plan-rust-required-scope.py','scripts/ci-required-scope.v2.json','scripts/ci-tests.sh','scripts/ci-required-capability-test-inventory.tsv','scripts/product_doc_markdown.py','scripts/doc-governance-requirements.txt','.github/workflows/rust.yml','scripts/pm/ci_required_inventory.py','scripts/pm/pr-merge-receipt.py','scripts/pm/task-closeout.sh',*[f'.agents/roles/{r}.md' for r in roles],'doc/engineering/workflow/source-of-truth.md','.agents/skills/requesting-repo-owned-review/SKILL.md'):
  dest=c.root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/rel,dest)
 if producer_snapshot is not None: install_historical_snapshot(c.root,producer_snapshot)
 if source_producer_change is not None:
  rel,_=source_producer_change;(c.root/rel).write_bytes(subprocess.check_output(['git','-C',str(c.root),'show',m.HEAD+':'+rel]))
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
        cls.review=build_review(HERE.parents[1],getattr(cls,'producer_snapshot',None),getattr(cls,'source_producer_change',None))
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



def scenario_actor(login):
    """Select the factual offline account before this scenario is built."""
    def mark(method):
        method.scenario_actor = login
        return method
    return mark


class CurrentTargetComponent(unittest.TestCase):
    """Real descendant Git/planner and authentic PUSH transport; no validator stubs."""
    @classmethod
    def setUpClass(cls):
        MergedIntegrationComponent.setUpClass.__func__(cls)
        root=cls.review['root'];a=cls.inputs
        cls.pr_execution=subprocess.check_output(['git','-C',str(root),'commit-tree',a['head']+'^{tree}','-p',a['base'],'-p',a['head']],input='offline ordinary PR merge checkout\n',text=True).strip()
        subprocess.run(['git','-C',str(root),'update-ref','refs/pull/1/merge',cls.pr_execution],check=True)
        git=lambda *args:subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
        if getattr(cls,'producer_snapshot',None) is not None:
            for row in cls.producer_snapshot['files']:
                dest=root/row['path'];shutil.copy2(HERE.parents[1]/row['path'],dest)
        # Two advances ensure last-push and cumulative ranges cannot be conflated.
        rel='scripts/pm/readiness_transport.py'
        (root/rel).write_bytes((root/rel).read_bytes()+b'\n# offline related target advance\n')
        if getattr(cls,'producer_snapshot',None) is not None:git('add','scripts','.github')
        else:git('add',rel)
        git('commit','-qm','first related consumer advance')
        cls.push_base=git('rev-parse','HEAD')
        rel='scripts/pm/terminal_proof.py'
        (root/rel).write_bytes((root/rel).read_bytes()+b'\n# offline second related target advance\n')
        git('add',rel);git('commit','-qm','second related consumer advance')
        cls.target=git('rev-parse','HEAD');cls.target_tree=git('rev-parse','HEAD^{tree}')
        cls.cumulative_paths=git('diff','--name-only',a['merge'],cls.target).splitlines()
        cls.push_paths=git('diff','--name-only',cls.push_base,cls.target).splitlines()
        assert len(cls.push_paths)==1 and (len(cls.cumulative_paths)>=2 if getattr(cls,'producer_snapshot',None) is not None else len(cls.cumulative_paths)==2)
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
        self.scenario_actor = getattr(getattr(self, self._testMethodName), 'scenario_actor', 'repo-admin')
        MergedIntegrationComponent.setUp(self)
        self.publication_fault_path=self.review['root']/'offline-publication-faults.json'
        self.publication_fault_path.unlink(missing_ok=True)
        self.review['harness'].gh_data.write_bytes(self.initial_gh_data)
        self.review['harness'].fake_gh.write_text(self.initial_gh_script)
        (self.review['root']/'.pm/github-project-sync/tasks.json').write_bytes(self.initial_mapping)
        shutil.rmtree(self.review['root']/'.git/oasis7-workflow-receipts'/self.data['task_uid'],ignore_errors=True)
        a=self.data;repo=a['repository'];t=self.target;m=a['merge']
        self.context={k:v for k,v in self.context.items() if k in {'repository_root','source_review_plan_path','source_review_handoff_path','source_review_resolution_path','source_scope_oid','check_app_id'}}
        self.responses[f'repos/{repo}/git/ref/heads/main']={'ref':'refs/heads/main','object':{'sha':t}}
        self.responses[f'repos/{repo}/compare/{m}...{t}']={'url':f'https://api.github.com/repos/{repo}/compare/{m}...{t}','base_commit':{'sha':m},'merge_base_commit':{'sha':m},'ahead_by':2,'behind_by':0,'total_commits':2,'status':'ahead'}
        self.push_run={'id':2701,'run_attempt':1,'event':'push','head_branch':'main','head_sha':t,'path':'.github/workflows/rust.yml','repository':{'full_name':repo},'status':'completed','conclusion':'success','check_suite_id':2801,'created_at':'2026-10-02T10:00:00Z','run_started_at':'2026-10-02T10:00:00Z'}
        self.push_steps=[{'number':i,'name':name,'status':'completed','conclusion':'success','started_at':f'2026-10-02T10:0{i}:00Z','completed_at':f'2026-10-02T10:0{i}:30Z'} for i,name in enumerate(['Plan required gate scope','Write required planner artifact','Upload required planner artifact','Run required test tier'],1)]
        self.push_job={'id':2901,'run_id':2701,'run_attempt':1,'head_sha':t,'name':'required-gate','check_run_url':f'https://api.github.com/repos/{repo}/check-runs/2901','status':'completed','conclusion':'success','labels':['ubuntu-latest'],'steps':self.push_steps}
        self.push_check={'id':2901,'name':'required-gate','app':{'id':15368},'head_sha':t,'status':'completed','conclusion':'success','details_url':f'https://github.com/{repo}/actions/runs/2701/job/2901','output':{'summary':None,'text':None}}
        self.push_plan=self.produce_push_plan(self.push_raw,2701,self.push_base,t)
        self.push_artifact={'id':3101,'name':'oasis7-required-plan-v1','expired':False,'size_in_bytes':1000,'created_at':'2026-10-02T10:02:10Z','updated_at':'2026-10-02T10:03:20Z','workflow_run':{'id':2701,'repository_id':1,'head_repository_id':1,'head_branch':'main','head_sha':t}}
        self.responses.update({f'repos/{repo}/actions/workflows/rust.yml/runs?event=push&per_page=100&page=1':{'total_count':2,'workflow_runs':[self.push_run,dict(self.push_run,id=2601,head_sha=self.data['merge'],created_at='2026-10-01T10:00:00Z')]},f'repos/{repo}/actions/runs/2701':self.push_run,f'repos/{repo}/actions/runs/2701/attempts/1':self.push_run,f'repos/{repo}/actions/runs/2701/attempts/1/jobs?per_page=100&page=1':{'total_count':1,'jobs':[self.push_job]},f'repos/{repo}/check-runs/2901':self.push_check,f'repos/{repo}/check-suites/2801/check-runs?per_page=100&page=1':{'total_count':1,'check_runs':[self.push_check]},f'repos/{repo}/actions/runs/2701/artifacts?per_page=100&page=1':{'total_count':1,'artifacts':[self.push_artifact]}})
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
    def produce_source_plan(self,run_id):
        import os,textwrap
        a=self.data;root=self.review['root']
        paths=subprocess.check_output(['git','-C',str(root),'diff','--name-only','--no-renames',a['base'],a['head']],text=True).splitlines()
        args=[sys.executable,str(root/'scripts/plan-rust-required-scope.py'),'--event-name','pull_request']
        for path in paths:args+=['--changed-path',path]
        proc=subprocess.run(args,cwd=root,text=True,capture_output=True,check=True)
        raw=dict(line.split('=',1) for line in proc.stdout.splitlines() if '=' in line)
        self.source_raw=raw;self.source_paths=paths
        workflow=subprocess.check_output(['git','-C',str(root),'show',self.pr_execution+':.github/workflows/rust.yml'],text=True)
        step=workflow.split('      - name: Write required planner artifact\n',1)[1]
        code=textwrap.dedent(step.split("          python3 -I - <<'PY'\n",1)[1].split('          PY\n',1)[0])
        values=dict(raw,source_scope_base=a['base'],integration_base=a['base'],source_head=a['head'],base_oid=a['base'],integration_base_oid=a['base'],head_oid=a['head'])
        source=root/'offline-source-scope-outputs.json';source.write_text(json.dumps(values))
        env=dict(os.environ,SCOPE_OUTPUTS_PATH=str(source),REPOSITORY=a['repository'],WORKFLOW_RUN_ID=str(run_id),INTEGRATION_JSON='')
        subprocess.run([sys.executable,'-I','-c',code],cwd=root,env=env,check=True,capture_output=True)
        result=json.loads((root/'output/required-plan/oasis7-required-plan-v1.json').read_text())
        self.receipt.canonical_planner(result['planner'])
        return result

    def build_source_ci_holds(self):
        import copy
        a=self.data;repo=a['repository'];h=a['head']
        self.source_run=dict(self.push_run,id=3701,event='pull_request',head_branch='source',head_sha=h,pull_requests=[],check_suite_id=3801,created_at='2026-09-30T10:00:00Z',run_started_at='2026-09-30T10:00:00Z')
        self.source_job=dict(self.push_job,id=3901,run_id=3701,head_sha=h,check_run_url=f'https://api.github.com/repos/{repo}/check-runs/3901')
        self.source_check=dict(self.push_check,id=3901,head_sha=h,check_suite={'id':3801},details_url=f'https://github.com/{repo}/actions/runs/3701/job/3901')
        self.source_job['steps']=[dict(step,started_at=step['started_at'].replace('2026-10-02','2026-09-30'),completed_at=step['completed_at'].replace('2026-10-02','2026-09-30')) for step in self.push_steps]
        self.source_run.update(workflow_id=230018940,referenced_workflows=[])
        self.source_job['steps'].insert(0,{'number':1,'name':'Run actions/checkout@v6','status':'completed','conclusion':'success','started_at':'2026-09-30T10:00:00Z','completed_at':'2026-09-30T10:00:30Z'})
        for number,step in enumerate(self.source_job['steps'],1):step['number']=number
        e=self.pr_execution
        tree=subprocess.check_output(['git','-C',str(self.review['root']),'rev-parse',h+'^{tree}'],text=True).strip()
        self.source_checkout_log=(
            '2026-09-30T10:00:00Z ##[group]Run actions/checkout@v6\n'
            '2026-09-30T10:00:01Z with:\n'
            '2026-09-30T10:00:02Z   fetch-depth: 0\n'
            f'2026-09-30T10:00:03Z   repository: {repo}\n'
            '2026-09-30T10:00:04Z ##[endgroup]\n'
            f'2026-09-30T10:00:05Z [command]/usr/bin/git -c protocol.version=2 fetch --no-tags --prune --no-recurse-submodules origin +refs/heads/*:refs/remotes/origin/* +refs/tags/*:refs/tags/* +{e}:refs/remotes/pull/1/merge\n'
            '2026-09-30T10:00:06Z ##[group]Checking out the ref\n'
            '2026-09-30T10:00:07Z [command]/usr/bin/git checkout --progress --force refs/remotes/pull/1/merge\n'
            '2026-09-30T10:00:08Z ##[endgroup]\n'
            '2026-09-30T10:00:09Z [command]/usr/bin/git log -1 --format=%H\n'
            f'2026-09-30T10:00:10Z {e}\n')
        import base64
        self.responses[f'repos/{repo}/actions/jobs/3901/logs']={'binary_b64':base64.b64encode(self.source_checkout_log.encode()).decode()}
        self.responses[f'repos/{repo}/git/commits/{e}']={'sha':e,'tree':{'sha':tree},'parents':[{'sha':a['base']},{'sha':h}]}
        hparents=subprocess.check_output(['git','-C',str(self.review['root']),'show','-s','--format=%P',h],text=True).split()
        self.responses[f'repos/{repo}/git/commits/{h}']={'sha':h,'tree':{'sha':tree},'parents':[{'sha':v} for v in hparents]}
        merged_tree=subprocess.check_output(['git','-C',str(self.review['root']),'rev-parse',a['merge']+'^{tree}'],text=True).strip()
        merged_parents=subprocess.check_output(['git','-C',str(self.review['root']),'show','-s','--format=%P',a['merge']],text=True).split()
        self.responses[f"repos/{repo}/git/commits/{a['merge']}"]={'sha':a['merge'],'tree':{'sha':merged_tree},'parents':[{'sha':v} for v in merged_parents]}
        self.responses[f'repos/{repo}/actions/workflows/230018940']={'id':230018940,'path':'.github/workflows/rust.yml','state':'active'}
        self.source_plan=self.produce_source_plan(3701)
        self.project_item={'id':'PVTI_offline','fieldValues':{'pageInfo':{'hasNextPage':False,'endCursor':None},'nodes':[
            {'__typename':'ProjectV2ItemFieldRepositoryValue','field':{'name':'Repository'},'repository':{'nameWithOwner':repo}},
            {'__typename':'ProjectV2ItemFieldTextValue','field':{'name':'Task UID'},'text':a['task_uid']},
            {'__typename':'ProjectV2ItemFieldTextValue','field':{'name':'Canonical Worktree'},'text':str(self.review['root'])},
            {'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':'Status'},'name':'In Progress'},
            {'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':'PM Status'},'name':'committed'},
            {'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':'Workflow Phase'},'name':'verification'}]}}
        self.holds={'reviews':[],'comments':[],'threads':{'nodes':[],'pageInfo':{'hasNextPage':False,'endCursor':None}},'operator':{'login':self.scenario_actor},'permission':{'permission':'admin'}}
        import io,zipfile,base64
        source_member=(json.dumps(self.source_plan,sort_keys=True,separators=(',',':'))+'\n').encode()
        archive=io.BytesIO()
        with zipfile.ZipFile(archive,'w') as z:z.writestr('oasis7-required-plan-v1.json',source_member)
        self.responses[f'repos/{repo}/actions/artifacts/4101/zip']={'binary_b64':base64.b64encode(archive.getvalue()).decode()}
        self.responses[f'repos/{repo}/actions/runs/3701/artifacts?per_page=100&page=1']={'total_count':1,'artifacts':[{'id':4101,'name':'oasis7-required-plan-v1','expired':False,'size_in_bytes':len(archive.getvalue()),'created_at':'2026-09-30T10:02:10Z','updated_at':'2026-09-30T10:03:20Z','workflow_run':{'id':3701,'head_sha':h,'head_branch':'source'}}]}
        self.responses[f'repos/{repo}/check-suites/3801']={'id':3801,'head_branch':'source','head_sha':h,'before':'0'*40,'after':h,'pull_requests':[],'app':{'id':15368},'repository':{'full_name':repo},'status':'completed','conclusion':'success'}
        self.responses[f'repos/{repo}/check-suites/3801/check-runs?per_page=100&page=1']={'total_count':1,'check_runs':[self.source_check]}
        self.responses[f'repos/{repo}/actions/runs/3701/attempts/1']=self.source_run
        # Retain source row in broad PUSH discovery only when it is actually PUSH.
        self.responses[f'repos/{repo}/actions/workflows/rust.yml/runs?head_sha={h}&per_page=100&page=1']={'total_count':1,'workflow_runs':[self.source_run]}
        self.responses[f'repos/{repo}/pulls/1/commits?per_page=100&page=1']=[{'sha':h}]
        self.responses[f'repos/{repo}/commits/{h}']={'sha':h,'html_url':f'https://github.com/{repo}/commit/{h}'}
        self.responses[f'repos/{repo}/actions/runs/3701']=self.source_run
        self.responses[f'repos/{repo}/actions/runs/3701/attempts/1/jobs?per_page=100&page=1']={'total_count':1,'jobs':[self.source_job]}
        self.responses[f'repos/{repo}/check-runs/3901']=self.source_check
        self.responses[f'repos/{repo}/commits/{h}/check-runs?per_page=100&page=1']={'total_count':1,'check_runs':[self.source_check]}
        self.responses[f'repos/{repo}/pulls/1/reviews?per_page=100&page=1']=[]
        self.responses[f'repos/{repo}/collaborators/{self.scenario_actor}/permission']=self.holds['permission']
        self.responses['user']=self.holds['operator']
        # Transport supplies factual inputs only; no closed verification record.
        self.publish_transport()
    def prepare_full_delivery(self):
        """Produce merge receipt with real supported producer; no old claims."""
        import os
        a=self.data;root=self.review['root'];c=self.review['harness'];repo=a['repository'];uid=a['task_uid']
        self.delivery_state_path=root/'offline-delivery-state.json'
        issue_url=f"https://github.com/{repo}/issues/{a['issue']}"
        body=json.loads(c.gh_data.read_text())['issue']['body']
        self.project_item.update(project={'id':'PROJECT_offline','number':1,'owner':{'login':repo.split('/')[0]}},content={'number':a['issue'],'url':issue_url,'body':body})
        self.delivery_state={'scenario_actor':self.scenario_actor,'repository':repo,'issue':{'number':a['issue'],'html_url':issue_url,'url':f"https://api.github.com/repos/{repo}/issues/{a['issue']}",'body':body,'state':'OPEN'},'pr':a['live_pr'],'default_branch':'main','default_target':self.target,'project_item':self.project_item,'comments':[]}
        self.delivery_state_path.write_text(json.dumps(self.delivery_state))
        live=json.loads(c.gh_data.read_text());live['issue']=self.delivery_state['issue']
        # Actual review/dispatch/resolution comments remain; no historical
        # readiness/completion entry is removed or invented.
        auth={'id':9001,'body':'<!-- oasis7-terminal-recovery-authorization/v1 -->\n'+json.dumps(a['authorization'],sort_keys=True,separators=(',',':')),'user':{'login':self.scenario_actor},'html_url':issue_url+'#issuecomment-9001','issue_url':f"https://api.github.com/repos/{repo}/issues/{a['issue']}",'created_at':'2026-10-02T10:00:00Z','updated_at':'2026-10-02T10:00:00Z'}
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
fault_path=FAULT_PATH
fault=json.load(open(fault_path)) if os.path.exists(fault_path) else {'mode':None,'fired':False,'events':[]}
def save_fault():
    if len(fault['events'])>100:raise SystemExit('offline publication trace bound exceeded')
    with open(fault_path,'w') as out:json.dump(fault,out)
args=sys.argv[1:]
if args==['auth','token']:
    if os.environ.get('OFFLINE_HTTP_PARENT_PID')!=str(os.getppid()):raise SystemExit('offline credential provider requires instrumented parent')
    if os.path.exists(os.path.join(os.path.dirname(path),'offline-http-mode.json')) and json.load(open(os.path.join(os.path.dirname(path),'offline-http-mode.json'))).get('mode')=='missing-credential':raise SystemExit('offline credential unavailable')
    print('offline-fixture-nonsecret-sentinel');raise SystemExit(0)
def save():
    state['project_item']['content'].update(body=state['issue']['body'],number=state['issue']['number'],url=state['issue']['html_url'])
    if 'title' in state['issue']:state['project_item']['content']['title']=state['issue']['title']
    with open(path,'w') as out:json.dump(state,out)
def fields():
    return {args[i+1].split('=',1)[0]:args[i+1].split('=',1)[1] for i,x in enumerate(args[:-1]) if x in ('-f','-F','--field','--raw-field') and '=' in args[i+1]}
def emit(value):
    print(json.dumps(value));raise SystemExit(0)
endpoint=next((x for x in args if x.startswith('repos/')),None)
method=next((args[i+1] for i,x in enumerate(args[:-1]) if x=='--method'),'GET')
repo=state['repository'];issue=state['issue']['number']
governed={'FIELD_STATUS':('Status',{'STATUS_COMMITTED':'In Progress','STATUS_DONE':'Done'}),
    'FIELD_PM':('PM Status',{'PM_COMMITTED':'committed','PM_DONE':'done'}),
    'FIELD_PHASE':('Workflow Phase',{'PHASE_VERIFY':'verification','PHASE_TASK_DONE':'task_done','PHASE_DONE':'done','PHASE_POST_MERGE_DONE':'post_merge_done'})}
def apply_project_values(updates):
    validated=[]
    for field,option in updates:
        if field not in governed or option not in governed[field][1]:raise SystemExit('unprovided Project mutation field/option identity')
        name,options=governed[field]
        matches=[node for node in state['project_item']['fieldValues']['nodes'] if node['field']['name']==name]
        if len(matches)>1:raise SystemExit('ambiguous Project mutation field')
        validated.append((name,options[option],matches))
    for name,value,matches in validated:
        if matches:matches[0]['name']=value
        else:state['project_item']['fieldValues']['nodes'].append({'__typename':'ProjectV2ItemFieldSingleSelectValue','field':{'name':name},'name':value})
    save()
if args[:2]==['project','field-list']:
    if args!=['project','field-list','1','--owner',repo.split('/')[0],'--format','json']:raise SystemExit('unprovided Project field-list request')
    emit({'fields':[{'id':fid,'name':name,'type':'ProjectV2SingleSelectField','options':[{'id':oid,'name':label} for oid,label in options.items()]} for fid,(name,options) in governed.items()],'totalCount':len(governed)})
if args[:2]==['project','item-edit']:
    field=args[7] if len(args)==12 else None;option=args[9] if len(args)==12 else None
    expected=['project','item-edit','--id',state['project_item']['id'],'--project-id',state['project_item']['project']['id'],'--field-id',field,'--single-select-option-id',option,'--format','json']
    if args!=expected:raise SystemExit('unprovided Project item-edit request')
    apply_project_values([(field,option)]);emit(state['project_item'])
if args[:2]==['issue','edit']:
    body_path=args[6] if len(args)==7 else None
    if args!=['issue','edit',str(issue),'-R',repo,'--body-file',body_path]:raise SystemExit('unprovided Issue edit request')
    with open(body_path,encoding='utf-8') as source:body=source.read()
    state['issue']['body']=body;save();print(state['issue']['html_url']);raise SystemExit(0)
if args[:2]==['issue','list']:
    import re
    expected=['issue','list','-R',repo,'--state','all','--search',args[7] if len(args)>7 else '', '--json','number,url,title,state','--limit','5']
    if args!=expected or re.fullmatch(r'task_[0-9a-f]{32} in:body',args[7]) is None:
        raise SystemExit('unprovided canonical Issue discovery request')
    searched=args[7].split(' ',1)[0]
    if searched not in state['issue']['body']:emit([])
    item=state['issue']
    emit([{'number':item['number'],'url':item['html_url'],'title':item.get('title',''),'state':item['state'].upper()}])
if args[:2]==['issue','view'] and '--json' in args and args[args.index('--json')+1]=='body,number,title,url,state,stateReason,updatedAt':
    expected=['issue','view',str(issue),'-R',repo,'--json','body,number,title,url,state,stateReason,updatedAt']
    if args!=expected:raise SystemExit('unprovided canonical Issue view request')
    item=state['issue']
    emit({'body':item['body'],'number':item['number'],'title':item.get('title',''),'url':item['html_url'],'state':item['state'].upper(),'stateReason':item.get('state_reason'),'updatedAt':item.get('updated_at')})
if args[:2]==['project','view']:
    expected=['project','view','1','--owner',repo.split('/')[0],'--format','json']
    if args!=expected:raise SystemExit('unprovided canonical Project view identity')
    project=state['project_item']['project']
    emit({'id':project['id'],'number':project['number'],'owner':{'login':project['owner']['login']}})
if endpoint and endpoint.startswith(f'repos/{repo}/issues/{issue}/comments?') and '--paginate' not in args and '--slurp' not in args:
    import urllib.parse
    parsed=urllib.parse.parse_qs(urllib.parse.urlsplit(endpoint).query)
    if set(parsed)!={'per_page','page'} or parsed['per_page']!=['100'] or len(parsed['page'])!=1 or not parsed['page'][0].isdigit() or int(parsed['page'][0])<1:
        raise SystemExit('unprovided explicit comment pagination')
    if fault['mode']=='completion-readback-loss' and fault['fired'] and not fault.get('readback_lost'):
        fault['readback_lost']=True;save_fault();raise SystemExit('simulated unreadable completion inventory after persisted POST')
    page=int(parsed['page'][0]);data=json.load(open(os.environ['GH_FIXTURE']))
    comments=[comment for retained in data['comment_pages'] for comment in retained]
    emit(comments[(page-1)*100:page*100])
if args[:2]==['api','user']:
    if '--jq' in args and args[args.index('--jq')+1]=='.login':print(state['scenario_actor']);raise SystemExit(0)
    emit({'login':state['scenario_actor']})
if args[:2]==['api','graphql']:
    inputs=fields();query=inputs.get('query','')
    identity_query='query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){nameWithOwner defaultBranchRef{name target{... on Commit{oid}}} pullRequest(number:$number){number url state merged mergedAt headRefOid headRefName baseRefName headRepository{nameWithOwner} repository{nameWithOwner} mergeCommit{oid}}}}'
    if query==identity_query:
        owner,name=repo.split('/')
        if inputs.get('owner')!=owner or inputs.get('name')!=name or inputs.get('number')!=str(state['pr']['number']) or not any(x=='-F' and args[i+1]=='number='+str(state['pr']['number']) for i,x in enumerate(args[:-1])):
            raise SystemExit('unprovided canonical GraphQL variable identity/type')
        pr=state['pr']
        emit({'data':{'repository':{'nameWithOwner':repo,'defaultBranchRef':{'name':state['default_branch'],'target':{'oid':state['default_target']}},'pullRequest':{
            'number':pr['number'],'url':pr['html_url'],'state':'MERGED' if pr['merged'] else pr['state'].upper(),
            'merged':pr['merged'],'mergedAt':pr['merged_at'],'headRefOid':pr['head']['sha'],'headRefName':pr['head']['ref'],
            'baseRefName':pr['base']['ref'],'headRepository':{'nameWithOwner':pr['head']['repo']['full_name']},
            'repository':{'nameWithOwner':repo},'mergeCommit':{'oid':pr['merge_commit_sha']}}}}})
    if 'updateProjectV2ItemFieldValue' in query:
        item=inputs.get('itemId') or inputs.get('item')
        project=inputs.get('projectId') or inputs.get('project')
        if item!=state['project_item']['id'] or project!=state['project_item']['project']['id']:
            raise SystemExit('unprovided Project mutation item/project identity')
        # Real sync uses aliased fieldN/optionN batch variables. Validate every
        # binding before any simulated server effect; no false acknowledgment.
        indexes=sorted(int(k[5:]) for k in inputs if k.startswith('field') and k[5:].isdigit())
        requests=[('f'+str(i),inputs['field'+str(i)],inputs.get('option'+str(i))) for i in indexes]
        if not requests:requests=[('updateProjectV2ItemFieldValue',inputs.get('fieldId') or inputs.get('field'),inputs.get('optionId') or inputs.get('option'))]
        apply_project_values([(field,option) for alias,field,option in requests])
        emit({'data':{alias:{'projectV2Item':{'id':state['project_item']['id']}} for alias,field,option in requests}})
    if 'mutation' in query:raise SystemExit('unprovided GraphQL mutation')
    project={'id':'PROJECT_offline','number':1,'fields':{'nodes':[
        {'__typename':'ProjectV2SingleSelectField','id':fid,'name':name,'options':[{'id':oid,'name':label} for oid,label in options]}
        for fid,(name,options_map) in governed.items() for options in [list(options_map.items())]],'pageInfo':{'hasNextPage':False}}}
    connection={'nodes':[state['project_item']],'pageInfo':{'hasNextPage':False,'endCursor':None}}
    if 'nodes(ids:' in query:
        if json.loads(inputs.get('ids','null'))!=[state['project_item']['id']]:raise SystemExit('unprovided Project node IDs')
        emit({'data':{'nodes':[state['project_item']]}})
    if inputs.get('id')==project['id'] or inputs.get('project')==project['id']:
        emit({'data':{'node':project}})
    emit({'data':{'node':state['project_item'],'viewer':{'login':state['scenario_actor']},'user':{'projectV2':project},'organization':{'projectV2':project},'repository':{'issue':dict(state['issue'],projectItems=connection),'pullRequest':{'reviewThreads':{'nodes':[],'pageInfo':{'hasNextPage':False,'endCursor':None}}}}}})
if endpoint==f'repos/{repo}/issues/{issue}' and method in ('PATCH','POST'):
    state['issue'].update(fields());save();emit(state['issue'])
if endpoint==f'repos/{repo}/issues/{issue}':emit(state['issue'])
if (args[:2]==['issue','comment'] or endpoint==f'repos/{repo}/issues/{issue}/comments' and method=='POST'):
    if args[:2]==['issue','comment']:
        body_path=args[6] if len(args)==7 else None
        if args!=['issue','comment',str(issue),'-R',repo,'--body-file',body_path]:raise SystemExit('unprovided Issue comment request')
    data=json.load(open(os.environ['GH_FIXTURE']));inputs=fields()
    body=inputs.get('body')
    if '--body-file' in args:body=open(args[args.index('--body-file')+1]).read()
    if body is None:raise SystemExit('missing actual comment body')
    stamp=datetime.datetime.now(datetime.timezone.utc).isoformat()
    ident=max([c['id'] for page in data['comment_pages'] for c in page]+[9001])+1
    comment={'id':ident,'body':body,'user':{'login':state['scenario_actor']},'created_at':stamp,'updated_at':stamp,'issue_url':f'https://api.github.com/repos/{repo}/issues/{issue}','html_url':f'https://github.com/{repo}/issues/{issue}#issuecomment-{ident}'}
    data['comment_pages'][-1].append(comment)
    with open(os.environ['GH_FIXTURE'],'w') as out:json.dump(data,out)
    marker=next((m for m in ('oasis7-postmerge-delivery-proof/v1','oasis7-postmerge-completion/v1') if '<!-- '+m+' -->' in body),None)
    if marker and fault['mode']:
        fault['events'].append({'marker':marker,'id':ident,'created_at':stamp});save_fault()
        if marker=='oasis7-postmerge-delivery-proof/v1' and not fault['fired']:
            if fault['mode']=='proof-response-loss':
                fault['fired']=True;save_fault();raise SystemExit('simulated lost response after persisted proof POST')
            if fault['mode']=='post-proof-check-drift':
                transport=json.load(open(TRANSPORT_PATH));transport['responses'][f'repos/{repo}/check-runs/2901']['conclusion']='failure'
                with open(TRANSPORT_PATH,'w') as out:json.dump(transport,out)
                fault['fired']=True;save_fault()
        if marker=='oasis7-postmerge-completion/v1' and fault['mode']=='completion-readback-loss':
            fault['fired']=True;save_fault()
    if args[:2]==['issue','comment']:print(comment['html_url']);raise SystemExit(0)
    emit(comment)
if args[:2]==['issue','close']:
    state['issue'].update(state='closed',state_reason='completed');save();emit(state['issue'])
"""
        prefix=prefix.replace('exec('+repr(original)+')\n',effects.replace('STATE_PATH',repr(str(self.delivery_state_path))).replace('FAULT_PATH',repr(str(self.publication_fault_path))).replace('TRANSPORT_PATH',repr(str(self.transport_path)))+'\nexec('+repr(original)+')\n')
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
        self.install_shared_client_fixture()

    def install_shared_client_fixture(self):
        """Intercept only HTTP effects, retaining real client and audit code."""
        import os
        root=self.review['root'];c=self.review['harness']
        self.http_mode_path=root/'offline-http-mode.json';self.http_log_path=root/'offline-http-requests.jsonl'
        for path in (self.http_mode_path,self.http_log_path):path.unlink(missing_ok=True)
        # These values are factual initial Project field nodes, derived from
        # the existing canonical fixture mapping, never an audit verdict.
        mapping=json.loads((root/'.pm/github-project-sync/tasks.json').read_text());record=mapping['tasks'][self.data['task_uid']]
        record.update(module='engineering',worktree_hint=str(root))
        (root/'.pm/github-project-sync/tasks.json').write_text(json.dumps(mapping))
        state=json.loads(self.delivery_state_path.read_text());nodes=state['project_item']['fieldValues']['nodes']
        for name,value in [('Owner Role',record['owner_role']),('Module',record.get('module','')),('Priority',record.get('priority','')),('PR',record['pr_url']),('Test Tier Required','n/a')]:
            nodes.append({'__typename':'ProjectV2ItemFieldTextValue','field':{'name':name},'text':value})
        producer=load(root/'scripts/pm/github-project-task.py','offline_recovery_issue_producer')
        produced=producer.issue_body(record)
        projection=[next(line for line in produced.splitlines() if line.startswith('- '+name+':'))
            for name in ('status','workflow_phase','completion_mode')]
        state['issue']['body']+='\n'+'\n'.join(projection)+'\n'
        state['project_item']['content']['body']=state['issue']['body']
        retained=json.loads(c.gh_data.read_text());retained['issue']['body']=state['issue']['body'];c.gh_data.write_text(json.dumps(retained))
        self.delivery_state_path.write_text(json.dumps(state))
        bootstrap=root/'offline-http-bootstrap.py'
        program=r'''
import io,json,os,pathlib,runpy,socket,subprocess,sys,time,urllib.error,urllib.parse,urllib.request
root=pathlib.Path(ROOT)
log=root/'offline-http-requests.jsonl';mode_path=root/'offline-http-mode.json'
def deny(*args,**kwargs):raise RuntimeError('unexpected real network blocked by offline transport')
socket.socket.connect=deny;socket.create_connection=deny
os.environ.pop('GH_TOKEN',None);os.environ.pop('GITHUB_TOKEN',None)
os.environ['OFFLINE_HTTP_PARENT_PID']=str(os.getpid())
class Reply:
    def __init__(self,status,headers,body):self.status=status;self.headers=headers;self.body=body
    def read(self):return self.body
    def __enter__(self):return self
    def __exit__(self,*args):return False
class Opener:
    def open(self,request,timeout=None):
        parsed=urllib.parse.urlsplit(request.full_url);method=request.get_method();body=request.data
        if parsed.scheme!='https' or parsed.netloc!='api.github.com' or parsed.fragment:raise RuntimeError('unprovided offline HTTP origin')
        if request.get_header('Authorization')!='Bearer offline-fixture-nonsecret-sentinel':raise RuntimeError('unexpected offline credential binding')
        if request.get_header('X-github-api-version')!='2022-11-28' or request.get_header('Accept')!='application/vnd.github+json':raise RuntimeError('unprovided HTTP API headers')
        endpoint=parsed.path.lstrip('/')+('?' + parsed.query if parsed.query else '')
        state=json.loads((root/'offline-delivery-state.json').read_text());repo=state['repository'];number=state['issue']['number']
        payload=json.loads(body) if body else None
        if endpoint=='graphql':
            if method!='POST' or not isinstance(payload,dict) or set(payload)!={'query','variables'} or not isinstance(payload['query'],str) or not isinstance(payload['variables'],dict):raise RuntimeError('unprovided GraphQL HTTP request shape')
            query=payload['query'];variables=payload['variables']
            if not any(part in query for part in ('nodes(ids:','node(id:','projectV2(','updateProjectV2ItemFieldValue','rateLimit','viewer')):raise RuntimeError('unprovided offline GraphQL query')
            args=['gh','api','graphql','-f','query='+query]
            for key,value in variables.items():
                args+=['-F' if isinstance(value,(int,bool)) else '-f',key+'='+(json.dumps(value) if isinstance(value,(dict,list,bool)) else str(value))]
        elif endpoint=='user' and method=='GET' and body is None:args=['gh','api','user']
        elif endpoint==f'repos/{repo}/issues/{number}' and method in ('GET','PATCH'):
            if method=='GET' and body is not None:raise RuntimeError('unexpected GET body')
            if method=='PATCH' and (not isinstance(payload,dict) or not set(payload)<= {'title','body','state','state_reason'}):raise RuntimeError('unprovided Issue PATCH body')
            args=['gh','api',endpoint,'--method',method]
            for key,value in (payload or {}).items():args+=['-f',key+'='+str(value)]
        else:raise RuntimeError('unprovided offline HTTP endpoint: '+method+' '+endpoint)
        record={'method':method,'url':request.full_url,'headers':{k:v for k,v in request.header_items() if k.lower()!='authorization'},'authorization_present':True,'body':payload,'timeout':timeout}
        with log.open('a') as out:out.write(json.dumps(record,sort_keys=True)+'\n')
        mode=json.loads(mode_path.read_text()).get('mode') if mode_path.exists() else None
        headers={'Content-Type':'application/json','X-GitHub-Request-Id':'offline-http-leaf','X-RateLimit-Remaining':'4999','X-RateLimit-Limit':'5000','X-RateLimit-Reset':str(int(time.time())+3600)}
        if mode in ('401','403'):raise urllib.error.HTTPError(request.full_url,int(mode),'offline authorization failure',headers,io.BytesIO(b'{"message":"offline authorization failure"}'))
        if mode=='malformed':return Reply(200,headers,b'{malformed')
        if mode=='graphql-error':return Reply(200,headers,b'{"errors":[{"message":"offline query failure"}]}')
        proc=subprocess.run(args,text=True,capture_output=True,timeout=timeout)
        if proc.returncode:raise RuntimeError('offline HTTP fixture transport rejected: '+proc.stderr)
        raw=proc.stdout.encode();decoded=json.loads(raw)
        if mode=='incomplete-page' and endpoint=='graphql':
            for node in decoded.get('data',{}).get('nodes',[]):node['fieldValues']['pageInfo']['hasNextPage']=True
            raw=json.dumps(decoded).encode()
        return Reply(200,headers,raw)
urllib.request.build_opener=lambda *handlers:Opener()
script=sys.argv[1];sys.argv=sys.argv[1:];sys.path.insert(0,str(pathlib.Path(script).resolve().parent));runpy.run_path(script,run_name='__main__')
'''.replace('ROOT',repr(str(root)))
        bootstrap.write_text(program)
        # Only these shell-spawned entrypoints acquire the shared HTTP client.
        # Credential provider parent-PID binding rejects uninjected descendants.
        allowed=[str(root/'scripts/pm'/name) for name in ('github-project-workflow.py','github-project-task.py','github-project-sync.py')]
        launcher=c.fake_gh.parent/'python3'
        launcher.write_text('#!/bin/sh\ncase "$1" in\n'+ '|'.join(allowed)+') exec '+sys.executable+' -S '+str(bootstrap)+' "$@" ;;\n*) exec '+sys.executable+' "$@" ;;\nesac\n')
        launcher.chmod(0o755)
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
    @scenario_actor('eng-cc')
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
        close=subprocess.run(['bash',str(root/'scripts/pm/task-closeout.sh'),'--task-uid',uid,'--role','repository_health_engineer','--to-status','done','--verification-profile','postmerge_delivery_recovery','--claim-type','postmerge_delivery_complete'],cwd=root,text=True,capture_output=True,timeout=330)
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
        self.responses[key]['workflow_runs'].append(newer);self.responses[key]['total_count']=3
        self.responses[f"repos/{self.data['repository']}/actions/runs/2702"]=newer
        with self.assertRaisesRegex(ValueError,'latest|success|failed|conclusion'):self.call_target()

    def test_actual_planner_identity_locators_cannot_be_rewritten(self):
        import copy
        original=copy.deepcopy(self.push_plan)
        for field in ('source_scope_base','integration_base','source_head'):
            with self.subTest(field=field):
                self.push_plan=copy.deepcopy(original)
                self.assertIn(field,self.push_plan['planner'],'actual producer locator required')
                self.push_plan['planner'][field]=self.data['merge']
                self.push_archive()
                with self.assertRaisesRegex(ValueError,'planner|locator|base|source|provenance|identity'):
                    self.call_target()

    def staged_target_transport(self,endpoint,mutate):
        import copy
        from unittest import mock
        import recovery_observation as observation
        real=observation.api;calls=[]
        def response(path):
            value=real(path)
            if path==endpoint:
                calls.append(copy.deepcopy(value))
                if len(calls)>=2:return mutate(copy.deepcopy(value))
            return value
        return mock.patch.object(observation,'api',side_effect=response),calls

    def test_final_latest_discovery_cannot_borrow_initial_green(self):
        endpoint=f"repos/{self.data['repository']}/actions/workflows/rust.yml/runs?event=push&per_page=100&page=1"
        def changed(value):
            newer=dict(self.push_run,id=2702,conclusion='failure',created_at='2026-10-02T11:00:00Z')
            value['workflow_runs'].append(newer);value['total_count']=len(value['workflow_runs'])
            self.responses[f"repos/{self.data['repository']}/actions/runs/2702"]=newer
            self.publish_transport()
            return value
        patch,calls=self.staged_target_transport(endpoint,changed)
        with patch:
            with self.assertRaisesRegex(ValueError,'latest|success|failed|conclusion|changed|moved'):self.call_target()
        self.assertGreaterEqual(len(calls),2,'final complete latest discovery must actually reread')

    def test_final_check_cannot_borrow_initial_provenance_or_success(self):
        endpoint=f"repos/{self.data['repository']}/check-runs/2901"
        for field,value in (('conclusion','failure'),('head_sha',self.data['head']),('app',{'id':99})):
            with self.subTest(field=field):
                patch,calls=self.staged_target_transport(endpoint,lambda row:dict(row,**{field:value}))
                with patch:
                    with self.assertRaisesRegex(ValueError,'check|app|identity|success|changed|provenance'):self.call_target()
                self.assertGreaterEqual(len(calls),2,'final authoritative check must actually reread')

    def test_final_exact_attempt_job_and_applicable_step_must_stay_successful(self):
        endpoint=f"repos/{self.data['repository']}/actions/runs/2701/attempts/1/jobs?per_page=100&page=1"
        for change in ('job','step'):
            with self.subTest(change=change):
                def changed(value):
                    job=value['jobs'][0]
                    if change=='job':job['conclusion']='skipped'
                    else:job['steps'][-1]['conclusion']='skipped'
                    return value
                patch,calls=self.staged_target_transport(endpoint,changed)
                with patch:
                    with self.assertRaisesRegex(ValueError,'job|step|execution|success|changed|skipped'):self.call_target()
                self.assertGreaterEqual(len(calls),2,'final exactattempt job/steps must actually reread')

    def call_source_observation(self):
        import terminal_recovery,recovery_observation
        self.publish_transport();before=self.effect_snapshot();a=self.data
        try:
            with recovery_observation.observation():
                return terminal_recovery.source_observation(a['repository'],a['task_uid'],a['head'],a['pr'],'main',self.review['root'],15368)
        finally:self.assertEqual(self.effect_snapshot(),before,'source reader must preserve every effect sink')

    def test_source_PR_execution_E_is_distinct_H_with_exact_tree_and_inventory(self):
        a=self.data;root=self.review['root'];e=self.pr_execution
        self.assertNotEqual(e,a['head'])
        parents=subprocess.check_output(['git','-C',str(root),'show','-s','--format=%P',e],text=True).split()
        self.assertEqual(parents,[a['base'],a['head']])
        tree=lambda oid:subprocess.check_output(['git','-C',str(root),'rev-parse',oid+'^{tree}'],text=True).strip()
        self.assertEqual(tree(e),tree(a['head']))
        result=self.call_source_observation()
        self.assertEqual(result['checks'][0]['workflow_sha'],e,'workflow W is derived E, never callerasserted H')
        inventory=subprocess.check_output(['git','-C',str(root),'show',e+':scripts/ci-required-capability-test-inventory.tsv'])
        self.assertEqual(result['coverage']['inventory_sha256'],hashlib.sha256(inventory).hexdigest())
        self.assertEqual(self.source_run['pull_requests'],[])
        self.assertFalse(self.receipt.canonical_planner(self.source_plan['planner'])['run_packaging_contracts'])

    def test_source_checkout_and_root_provenance_contradictions_refuse(self):
        import copy,base64
        self.call_source_observation() # A missing provenance capability cannot satisfy a negative.
        original=copy.deepcopy(self.responses);a=self.data;repo=a['repository']
        endpoint=f'repos/{repo}/actions/jobs/3901/logs'
        cases=[('log',self.source_checkout_log.replace('  fetch-depth: 0','  ref: '+a['head'])),
               ('log',self.source_checkout_log.replace('refs/remotes/pull/1/merge','refs/remotes/pull/2/merge')),
               ('log',self.source_checkout_log.replace('repository: '+repo,'repository: foreign/repo')),
               ('runpath','foreign.yml'),('event','push'),('reuse',[{'path':'foreign/reusable.yml'}])]
        for kind,value in cases:
            with self.subTest(kind=kind,value=str(value)[:60]):
                self.responses=copy.deepcopy(original)
                if kind=='log':self.responses[endpoint]={'binary_b64':base64.b64encode(value.encode()).decode()}
                else:
                    run=self.responses[f'repos/{repo}/actions/runs/3701']
                    run[{'runpath':'path','event':'event','reuse':'referenced_workflows'}[kind]]=value
                with self.assertRaisesRegex(ValueError,'checkout|workflow|source|event|root|repository|provenance|identity'):
                    self.call_source_observation()

    def test_source_E_parents_or_tree_mismatch_refuse(self):
        import copy
        self.call_source_observation()
        original=copy.deepcopy(self.responses);repo=self.data['repository']
        endpoint=f'repos/{repo}/git/commits/{self.pr_execution}'
        for field,value in [('parents',[{'sha':self.data['head']},{'sha':self.data['base']}]),('tree',{'sha':'f'*40})]:
            with self.subTest(field=field):
                self.responses=copy.deepcopy(original);self.responses[endpoint][field]=value
                with self.assertRaisesRegex(ValueError,'checkout|workflow|parent|tree|provenance|identity'):self.call_source_observation()

    def test_source_compound_PR_association_planner_and_selected_work_refuse(self):
        import copy,io,zipfile,base64
        self.call_source_observation()
        original=copy.deepcopy(self.responses);a=self.data;repo=a['repository']
        for kind in ('PRhead','PRbase','PRrepo','association','planner','selectedstep'):
            with self.subTest(kind=kind):
                self.responses=copy.deepcopy(original)
                if kind.startswith('PR'):
                    pr=self.responses[f'repos/{repo}/pulls/1']
                    if kind=='PRhead':pr['head']['sha']=self.target
                    elif kind=='PRbase':pr['base']['sha']=self.target
                    else:pr['head']['repo']['full_name']='foreign/repo'
                elif kind=='association':
                    self.responses[f'repos/{repo}/check-suites/3801']['pull_requests']=[{'number':2,'head':{'sha':self.target,'ref':'wrong','repo':{'full_name':repo}},'base':{'sha':a['base'],'ref':'main','repo':{'full_name':repo}}}]
                elif kind=='planner':
                    plan=copy.deepcopy(self.source_plan);plan['planner']['source_head']=self.target
                    archive=io.BytesIO()
                    with zipfile.ZipFile(archive,'w') as z:z.writestr('oasis7-required-plan-v1.json',json.dumps(plan).encode())
                    self.responses[f'repos/{repo}/actions/artifacts/4101/zip']={'binary_b64':base64.b64encode(archive.getvalue()).decode()}
                else:self.responses[f'repos/{repo}/actions/runs/3701/attempts/1/jobs?per_page=100&page=1']['jobs'][0]['steps'][-1]['conclusion']='skipped'
                with self.assertRaisesRegex(ValueError,'source|head|base|PR|association|identity|planner|step|execution|provenance'):
                    self.call_source_observation()

    def publication_fault(self,mode):
        self.publication_fault_path.write_text(json.dumps({'mode':mode,'fired':False,'events':[]}))
    def publication_comments(self,marker):
        pages=json.loads(self.review['harness'].gh_data.read_text())['comment_pages']
        return [c for page in pages for c in page if '<!-- '+marker+' -->' in c['body']]
    def run_publication(self,create=True):
        root=self.review['root']
        args=[sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(root),'--task-uid',self.data['task_uid'],'--recover-merged-delivery']
        if create:args.append('--create')
        return subprocess.run(args,cwd=root,text=True,capture_output=True,timeout=330)
    def unchanged_lifecycle(self,before):
        after=self.effect_snapshot()
        for key in ('mapping','server_state'):self.assertEqual(after[key],before[key])
        for filename in ('terminal-delivery-receipt.json','finalizer-ledger.json','terminal-tombstone.json'):
            self.assertFalse((self.receipt_root/filename).exists(),filename)

    def test_partial_proof_response_loss_retries_exact_persisted_binding(self):
        self.publication_fault('proof-response-loss');before=self.effect_snapshot()
        failed=self.run_publication();self.assertNotEqual(failed.returncode,0,failed.stdout+failed.stderr)
        self.assertRegex(failed.stderr,'transport|lost|publication')
        trace=json.loads(self.publication_fault_path.read_text());self.assertTrue(trace['fired'])
        proof=self.publication_comments('oasis7-postmerge-delivery-proof/v1');self.assertEqual(len(proof),1)
        self.assertEqual(self.publication_comments('oasis7-postmerge-completion/v1'),[])
        self.unchanged_lifecycle(before)
        retry=self.run_publication();self.assertEqual(retry.returncode,0,retry.stdout+retry.stderr)
        result=json.loads(retry.stdout);self.assertEqual(result['recovery_proof_comment_id'],proof[0]['id'])
        self.assertEqual(len(self.publication_comments('oasis7-postmerge-delivery-proof/v1')),1)
        self.assertEqual(len(self.publication_comments('oasis7-postmerge-completion/v1')),1)
        self.assertEqual(len(json.loads(self.publication_fault_path.read_text())['events']),2)
        self.unchanged_lifecycle(before)

    def test_partial_completion_readback_loss_retries_without_duplicate_POST(self):
        self.publication_fault('completion-readback-loss');before=self.effect_snapshot()
        failed=self.run_publication();self.assertNotEqual(failed.returncode,0,failed.stdout+failed.stderr)
        trace=json.loads(self.publication_fault_path.read_text());self.assertTrue(trace['fired']);self.assertTrue(trace['readback_lost'])
        proof=self.publication_comments('oasis7-postmerge-delivery-proof/v1');claims=self.publication_comments('oasis7-postmerge-completion/v1')
        self.assertEqual(len(proof),1);self.assertEqual(len(claims),1);self.unchanged_lifecycle(before)
        retry=self.run_publication();self.assertEqual(retry.returncode,0,retry.stdout+retry.stderr)
        result=json.loads(retry.stdout);self.assertEqual(result['recovery_proof_comment_id'],proof[0]['id']);self.assertEqual(result['completion_comment_id'],claims[0]['id'])
        self.assertEqual(len(json.loads(self.publication_fault_path.read_text())['events']),2)
        self.unchanged_lifecycle(before)

    def test_post_proof_CI_drift_blocks_completion_POST_before_effect(self):
        self.publication_fault('post-proof-check-drift');before=self.effect_snapshot()
        result=self.run_publication();self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)
        trace=json.loads(self.publication_fault_path.read_text());self.assertTrue(trace['fired'],'must reach actual persisted proof POST')
        self.assertEqual(len(self.publication_comments('oasis7-postmerge-delivery-proof/v1')),1)
        self.assertRegex(result.stderr,'check|execution|success|changed|provenance')
        self.unchanged_lifecycle(before)
        self.assertEqual(self.publication_comments('oasis7-postmerge-completion/v1'),[],'stale CI must block completion BEFORE its POST')

    def test_rehashed_fabricated_primary_refuses_independent_recollection_before_POST(self):
        import base64
        preflight=self.run_publication(create=False);self.assertEqual(preflight.returncode,0,preflight.stderr)
        proof=json.loads(preflight.stdout);evidence=proof['verification']['merge_readback']['evidence'][0]
        raw=json.loads(base64.b64decode(evidence['raw_b64']));raw['data']['repository']['pullRequest']['headRefOid']=self.target
        replacement=json.dumps(raw,sort_keys=True,separators=(',',':')).encode()
        evidence['raw_b64']=base64.b64encode(replacement).decode();evidence['raw_sha256']=hashlib.sha256(replacement).hexdigest()
        (self.receipt_root/'terminal-recovery-proof.json').write_bytes(json.dumps(proof,sort_keys=True,separators=(',',':')).encode())
        before=self.effect_snapshot();result=self.run_publication()
        self.assertNotEqual(result.returncode,0,result.stderr);self.assertRegex(result.stderr,'independently recollected.*primary evidence changed')
        self.assertEqual(self.effect_snapshot(),before)
        self.assertEqual(self.publication_comments('oasis7-postmerge-delivery-proof/v1'),[])

    def test_duplicate_persisted_proof_refuses_retry_without_effects(self):
        self.publication_fault('proof-response-loss');failed=self.run_publication();self.assertNotEqual(failed.returncode,0)
        trace=json.loads(self.publication_fault_path.read_text());self.assertTrue(trace['fired'])
        proof=self.publication_comments('oasis7-postmerge-delivery-proof/v1');self.assertEqual(len(proof),1)
        data=json.loads(self.review['harness'].gh_data.read_text());duplicate=dict(proof[0],id=proof[0]['id']+100)
        duplicate['html_url']=duplicate['html_url'].rsplit('-',1)[0]+'-'+str(duplicate['id']);data['comment_pages'][-1].append(duplicate)
        self.review['harness'].gh_data.write_text(json.dumps(data));before=self.effect_snapshot()
        result=self.run_publication();self.assertNotEqual(result.returncode,0,result.stderr);self.assertRegex(result.stderr,'duplicate|ambiguous')
        self.assertEqual(self.effect_snapshot(),before);self.assertEqual(len(json.loads(self.publication_fault_path.read_text())['events']),1)

    def test_recovery_publication_does_not_satisfy_native_or_wrong_done_pair(self):
        publication=self.run_publication();self.assertEqual(publication.returncode,0,publication.stderr);before=self.effect_snapshot();root=self.review['root'];uid=self.data['task_uid']
        native=subprocess.run([sys.executable,str(HERE/'readiness_transport.py'),'--repo-root',str(root),'--task-uid',uid,'--create'],text=True,capture_output=True,timeout=330)
        self.assertNotEqual(native.returncode,0);self.assertRegex(native.stderr,'readiness|native|migration');self.assertEqual(self.effect_snapshot(),before)
        wrong=subprocess.run(['bash',str(root/'scripts/pm/task-closeout.sh'),'--task-uid',uid,'--role','repository_health_engineer','--to-status','done','--verification-profile','postmerge_delivery_recovery','--claim-type','task_complete'],cwd=root,text=True,capture_output=True,timeout=330)
        self.assertNotEqual(wrong.returncode,0);self.assertRegex(wrong.stderr+wrong.stdout,'profile|claim|pair|requires');self.assertEqual(self.effect_snapshot(),before)

    def test_corrupt_existing_delivery_namespace_blocks_before_publication(self):
        baseline=self.run_publication(create=False);self.assertEqual(baseline.returncode,0,baseline.stderr)
        self.assertEqual(self.publication_comments('oasis7-postmerge-delivery-proof/v1'),[])
        self.assertEqual(self.publication_comments('oasis7-postmerge-completion/v1'),[])
        (self.receipt_root/'terminal-delivery-receipt.json').write_text('{malformed-existing-delivery')
        before=self.effect_snapshot();result=self.run_publication()
        self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertRegex(result.stderr,'delivery|terminal|namespace|JSON|json')
        self.assertEqual(self.effect_snapshot(),before)
        self.assertEqual(self.publication_comments('oasis7-postmerge-delivery-proof/v1'),[])
        self.assertEqual(self.publication_comments('oasis7-postmerge-completion/v1'),[])

    def shared_client_audit(self):
        root=self.review['root']
        return subprocess.run(['bash',str(root/'scripts/pm/github-project-workflow.sh'),'--json','audit','--task-uid',self.data['task_uid']],cwd=root,text=True,capture_output=True,timeout=45)

    def tearDown(self):
        if hasattr(self,'http_log_path') and self.http_log_path.exists():
            print(json.dumps({'offline_shared_client_requests':[json.loads(line) for line in self.http_log_path.read_text().splitlines()]},sort_keys=True))
        super().tearDown()

    def test_real_shared_HTTP_client_selected_audit_has_typed_task_and_request_log(self):
        before=self.effect_snapshot();result=self.shared_client_audit()
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        audit=json.loads(result.stdout);self.assertEqual(audit['status'],'ok')
        self.assertEqual(audit['selected_task']['task_uid'],self.data['task_uid'])
        requests=[json.loads(line) for line in self.http_log_path.read_text().splitlines()]
        self.assertTrue(requests);self.assertTrue(all(r['authorization_present'] for r in requests))
        self.assertTrue(any(r['url']=='https://api.github.com/graphql' and 'nodes(ids:' in r['body']['query'] for r in requests))
        self.assertTrue(all(r['url'].startswith('https://api.github.com/') for r in requests))
        self.assertEqual(self.effect_snapshot(),before)

    def test_shared_HTTP_auth_status_and_JSON_failures_are_not_audit_authority(self):
        baseline=self.shared_client_audit();self.assertEqual(baseline.returncode,0,baseline.stdout+baseline.stderr)
        for mode in ('missing-credential','401','403','malformed','graphql-error','incomplete-page'):
            with self.subTest(mode=mode):
                self.http_mode_path.write_text(json.dumps({'mode':mode}));before=self.effect_snapshot()
                result=self.shared_client_audit();self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)
                self.assertEqual(self.effect_snapshot(),before)
        self.http_mode_path.unlink()

    def test_shared_HTTP_Project_identity_contradictions_reach_real_audit(self):
        baseline=self.shared_client_audit();self.assertEqual(baseline.returncode,0,baseline.stdout+baseline.stderr)
        original=self.delivery_state_path.read_bytes()
        for field,value in [('Task UID','task_ffffffffffffffffffffffffffffffff'),('Module','game-strategy'),('Canonical Worktree','/unrelated/offline-root'),('PR','https://github.com/other/repository/pull/999')]:
            with self.subTest(field=field):
                state=json.loads(original);matches=[n for n in state['project_item']['fieldValues']['nodes'] if n['field']['name']==field]
                self.assertEqual(len(matches),1);matches[0]['text']=value
                self.delivery_state_path.write_text(json.dumps(state));before=self.effect_snapshot()
                result=self.shared_client_audit();self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)
                self.assertEqual(self.effect_snapshot(),before)
        self.delivery_state_path.write_bytes(original)

    def test_premerge_record_observation_rejects_selfconsistent_published_proof(self):
        import datetime
        root=self.review['root'];uid=self.data['task_uid']
        publication=self.run_publication();self.assertEqual(publication.returncode,0,publication.stdout+publication.stderr)
        recovery=load(root/'scripts/pm/terminal_recovery.py','offline_timestamp_actual_recovery')
        baseline=recovery.validate_recovery(root,uid)
        proof=baseline['proof'];completion=baseline['completion']
        self.assertGreaterEqual(recovery.obs.instant(proof['observed_at']),recovery.obs.instant(proof['merged_at']))
        stamp=recovery.obs.instant(proof['merged_at'])-datetime.timedelta(seconds=1)
        proof['verification']['merge_readback']['observed_at']=stamp.isoformat()
        raw=recovery.obs.canonical(proof);proof_digest=recovery.obs.digest(raw)
        (self.receipt_root/'terminal-recovery-proof.json').write_bytes(raw)
        completion['recovery_proof_sha256']=proof_digest
        (self.receipt_root/'terminal-recovery-completion.json').write_bytes(recovery.obs.canonical(completion))
        retained=json.loads(self.review['harness'].gh_data.read_text())
        for page in retained['comment_pages']:
            for comment in page:
                if comment['id']==baseline['proof_comment']['id']:
                    comment['body']='<!-- oasis7-postmerge-delivery-proof/v1 -->\n'+raw.decode()
                if comment['id']==baseline['completion_comment']['id']:
                    comment['body']='<!-- oasis7-postmerge-completion/v1 -->\n'+recovery.obs.canonical(completion).decode()
        self.review['harness'].gh_data.write_text(json.dumps(retained))
        # Exact reciprocal bindings and authenticated comments still validate;
        # only the current record's observation precedes the real merge time.
        recovery._validate_completion(completion,proof)
        comments=[comment for page in retained['comment_pages'] for comment in page]
        with recovery.obs.observation():
            recovery._marker_comment(proof['repository'],proof['issue_number'],comments,'<!-- oasis7-postmerge-delivery-proof/v1 -->',proof)
            recovery._marker_comment(proof['repository'],proof['issue_number'],comments,'<!-- oasis7-postmerge-completion/v1 -->',completion)
        before=self.effect_snapshot()
        with self.assertRaisesRegex(ValueError,'premerge|observation|observed'):
            recovery.validate_recovery(root,uid)
        self.assertEqual(self.effect_snapshot(),before)

    def _t1ac_identity_comments(self):
        c=self.review['harness'];repo=self.data['repository'];actor=self.scenario_actor
        def gh(*args):
            return subprocess.check_output([str(c.fake_gh),*args],text=True)
        self.assertEqual(json.loads(gh('api','user'))['login'],actor)
        self.assertEqual(gh('api','user','--jq','.login').strip(),actor)
        self.assertEqual(json.loads(gh('api','graphql','-f','query=query{viewer{login}}'))['data']['viewer']['login'],actor)
        self.assertEqual(json.loads(gh('api',f'repos/{repo}/collaborators/{actor}/permission'))['permission'],'admin')
        self.assertEqual(self.holds['operator']['login'],actor)
        retained=[v for page in json.loads(c.gh_data.read_text())['comment_pages'] for v in page]
        self.assertEqual(next(v for v in retained if v['id']==9001)['user']['login'],actor)
        body=protocol.terminal_proof.RECOVERY_MARKER+'\nactual offline actor transport control'
        body_path=self.review['root']/'offline-actor-body.txt';body_path.write_text(body)
        url=gh('issue','comment',str(self.data['issue']),'-R',repo,'--body-file',str(body_path)).strip()
        posted=[v for page in json.loads(c.gh_data.read_text())['comment_pages'] for v in page][-1]
        self.assertEqual(posted['user']['login'],actor);self.assertEqual(posted['html_url'],url)
        other=json.loads(gh('api',f"repos/{repo}/issues/{self.data['issue']}/comments",'--method','POST','-f','body=REST actor control'))
        self.assertEqual(other['user']['login'],actor)
        self.assertEqual(json.loads(gh('api',f"repos/{repo}/issues/comments/{posted['id']}")),posted)
        return posted,body

    @scenario_actor('eng-cc')
    def test_t1ac_owner_scenario_identity_and_durable_comments_agree(self):
        posted,body=self._t1ac_identity_comments()
        reader=load(HERE/'post-merge-finalize.py','t1ac_owner_reader')
        context={'repository':self.data['repository'],'issue_number':self.data['issue'],'recovery':True,'comments':[posted]}
        self.assertEqual(reader._delivery_comment_readback(context,body),posted)

    def test_t1ac_other_admin_is_not_terminal_owner_authority(self):
        posted,body=self._t1ac_identity_comments()
        self.assertEqual(self.scenario_actor,'repo-admin')
        reader=load(HERE/'post-merge-finalize.py','t1ac_other_admin_reader')
        context={'repository':self.data['repository'],'issue_number':self.data['issue'],'recovery':True,'comments':[posted]}
        with self.assertRaisesRegex(ValueError,'terminal v2 evidence comment author mismatch'):
            reader._delivery_comment_readback(context,body)

class HistoricalProducerCompatibility(unittest.TestCase):
    """Old immutable producer inputs and independent present helper; no verdict fakes."""
    producer_snapshot=json.loads((HERE/'fixtures/historical-required-producer-2ca.json').read_text())
    setUpClass=classmethod(CurrentTargetComponent.setUpClass.__func__)
    tearDownClass=classmethod(CurrentTargetComponent.tearDownClass.__func__)
    def setUp(self):
        CurrentTargetComponent.setUp(self)
        for index,name in enumerate(('Install pinned Rust toolchains','Install cargo-deny','Install product-document Markdown parser')):
            self.source_job['steps'].append({'number':len(self.source_job['steps'])+1,'name':name,'status':'completed','conclusion':'success','started_at':f'2026-09-30T10:03:{31+index*2:02d}Z','completed_at':f'2026-09-30T10:03:{32+index*2:02d}Z'})
        self.source_job['labels']=['ubuntu-24.04']
        # These immutable-workflow resources are not selected by the real old
        # profile. Preserve factual skips instead of borrowing their coverage.
        for index,name in enumerate(('Run actions/setup-node@v6','Install viewer web dependencies','Run Swatinem/rust-cache@v2','Install trunk','Install system deps')):
            self.source_job['steps'].append({'number':len(self.source_job['steps'])+1,'name':name,'status':'completed','conclusion':'skipped','started_at':f'2026-09-30T10:03:{38+index:02d}Z','completed_at':f'2026-09-30T10:03:{38+index:02d}Z'})
        self.source_job['steps'].sort(key=lambda step:step['started_at'])
        for index,step in enumerate(self.source_job['steps'],1):step['number']=index
        self.publish_transport()
    publish_transport=MergedIntegrationComponent.publish_transport
    push_archive=CurrentTargetComponent.push_archive
    produce_push_plan=CurrentTargetComponent.produce_push_plan
    produce_source_plan=CurrentTargetComponent.produce_source_plan
    build_source_ci_holds=CurrentTargetComponent.build_source_ci_holds
    prepare_full_delivery=CurrentTargetComponent.prepare_full_delivery
    install_shared_client_fixture=CurrentTargetComponent.install_shared_client_fixture
    effect_snapshot=CurrentTargetComponent.effect_snapshot
    call_source_observation=CurrentTargetComponent.call_source_observation

    archive=MergedIntegrationComponent.archive
    call_target=CurrentTargetComponent.call_target

    def _m2_bindings(self, base=None):
        import terminal_recovery,recovery_observation
        a=self.data;self.publish_transport();before=self.effect_snapshot()
        try:
            with recovery_observation.observation():
                terminal_recovery._historical_source_bindings(a['repository'],a['task_uid'],a['pr'],a['head'],base or a['base'],'main',self.review['root'],15368)
        finally:self.assertEqual(self.effect_snapshot(),before)

    def test_historical_runner_and_selected_resources_refuse_missing_failed_late(self):
        import copy
        self.assertEqual(self.call_source_observation()['checks'][0]['workflow_sha'],self.pr_execution)
        original=copy.deepcopy(self.source_job)
        cases=('runner','missing','failed','late')
        for case in cases:
            with self.subTest(case=case):
                self.source_job.clear();self.source_job.update(copy.deepcopy(original))
                if case=='runner':self.source_job['labels']=['ubuntu-latest']
                elif case=='missing':self.source_job['steps']=[s for s in self.source_job['steps'] if s['name']!='Install cargo-deny']
                elif case=='failed':next(s for s in self.source_job['steps'] if s['name']=='Install pinned Rust toolchains')['conclusion']='failure'
                else:next(s for s in self.source_job['steps'] if s['name']=='Install product-document Markdown parser')['completed_at']='2026-09-30T10:04:01Z'
                diagnostic='runner/resource' if case=='runner' else ('did not precede' if case=='late' else 'selected resource execution unavailable')
                with self.assertRaisesRegex(ValueError,diagnostic):self.call_source_observation()
        self.source_job.clear();self.source_job.update(original)

    def test_historical_review_base_merge_parent_and_original_integration_are_not_borrowed(self):
        import copy
        self._m2_bindings()
        a=self.data;root=self.review['root'];repo=a['repository']
        parent=subprocess.check_output(['git','-C',str(root),'rev-parse',a['base']+'^'],text=True).strip()
        self.assertNotEqual(parent,a['base'])
        with self.assertRaisesRegex(ValueError,'review scope'):self._m2_bindings(parent)
        key=f"repos/{repo}/git/commits/{a['merge']}";original=copy.deepcopy(self.responses[key])
        self.responses[key]['parents']=[{'sha':parent}]
        with self.assertRaisesRegex(ValueError,'exact sole merge parent'):self._m2_bindings()
        self.responses[key]=original
        proof=copy.deepcopy(a['artifact']);proof['tested_tree_oid']=subprocess.check_output(['git','-C',str(root),'rev-parse',a['base']+'^{tree}'],text=True).strip()
        self.assertNotEqual(proof['tested_tree_oid'],a['artifact']['tested_tree_oid'])
        self.archive(payload=proof)
        with self.assertRaisesRegex((ValueError,SystemExit),'tree|identity'):self._m2_bindings()
        self.archive(payload=a['artifact'])
        ref=f'repos/{repo}/git/ref/heads/main';compare=f"repos/{repo}/compare/{a['merge']}...{self.target}"
        original_ref=copy.deepcopy(self.responses[ref]);self.responses[ref]['object']['sha']=parent
        self.responses[f"repos/{repo}/compare/{a['merge']}...{parent}"]={'url':f"https://api.github.com/repos/{repo}/compare/{a['merge']}...{parent}",'status':'behind','base_commit':{'sha':a['merge']},'merge_base_commit':{'sha':parent},'ahead_by':0,'behind_by':2,'total_commits':0,'commits':[]}
        with self.assertRaisesRegex(ValueError,'ancestry|contained|default.*history|compare'):self._m2_bindings()
        self.responses[ref]=original_ref

    def _m2_variant(self, snapshot, source_change=None):
        from contextlib import contextmanager
        @contextmanager
        def build():
            variant=type('IsolatedHistoricalVariant',(HistoricalProducerCompatibility,),{'producer_snapshot':snapshot,'source_producer_change':source_change})
            try:
                variant.setUpClass();instance=variant('test_supported_old_source_observation_retains_actual_execution_provenance')
                instance.setUp();yield instance
            finally:
                if 'review' in variant.__dict__:variant.tearDownClass()
        return build()

    def test_historical_H_change_and_equal_unsafe_B_H_are_unsupported_versions(self):
        import copy,base64
        self.assertEqual(self.call_source_observation()['checks'][0]['workflow_sha'],self.pr_execution)
        with self._m2_variant(copy.deepcopy(self.producer_snapshot),('scripts/ci-tests.sh',b'\n# delivered producer changed outside recognized contract\n')) as changed:
            self.assertNotEqual(subprocess.check_output(['git','-C',str(changed.review['root']),'show',changed.data['base']+':scripts/ci-tests.sh']),subprocess.check_output(['git','-C',str(changed.review['root']),'show',changed.data['head']+':scripts/ci-tests.sh']))
            with self.assertRaisesRegex(ValueError,'unsupported historical source producer/dependency'):changed.call_source_observation()
        snapshot=copy.deepcopy(self.producer_snapshot)
        row=next(r for r in snapshot['files'] if r['path']=='scripts/ci-tests.sh')
        raw=base64.b64decode(row['bytes_b64']).replace(b'set -euo pipefail',b'set -uo pipefail',1)
        row.update(bytes_b64=base64.b64encode(raw).decode(),size=len(raw),sha256=hashlib.sha256(raw).hexdigest(),blob_oid=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest())
        with self._m2_variant(snapshot) as changed:
            actual=lambda oid:subprocess.check_output(['git','-C',str(changed.review['root']),'show',oid+':scripts/ci-tests.sh'])
            self.assertEqual(actual(changed.data['base']),actual(changed.data['head']))
            self.assertNotIn(b'set -euo pipefail',actual(changed.data['head']))
            with self.assertRaisesRegex(ValueError,'unsupported historical source producer/dependency'):changed.call_source_observation()

    def test_present_target_retains_independent_check_and_selected_coverage_guards(self):
        import copy
        with self._m2_variant(None) as current:
            current.call_target()
            import recovery_observation
            root=current.review['root'];a=current.data
            copied=load(root/'scripts/pm/terminal_recovery.py','m2_actual_disposable_current_reader')
            def read_actual_copy():
                current.publish_transport();before=current.effect_snapshot()
                try:
                    with recovery_observation.observation():
                        return copied._execution_observation(a['repository'],a['task_uid'],a['head'],current.target,'main',root,a['merge'],15368)
                finally:self.assertEqual(current.effect_snapshot(),before)
            read_actual_copy()
            path=root/'scripts/ci-tests.sh';before=path.read_bytes()
            path.write_bytes(before+b'\n# disposable current helper producer-byte drift\n')
            try:
                with self.assertRaisesRegex(ValueError,'unsupported trusted push dispatcher/workflow source changed'):read_actual_copy()
            finally:path.write_bytes(before)
            plan=copy.deepcopy(current.push_plan)
            current.push_plan['planner']['run_workflow_governance_contracts']='false'
            current.push_archive()
            with self.assertRaisesRegex((ValueError,SystemExit),'selector|capability|reproduction|coverage'):current.call_target()
            current.push_plan=plan;current.push_archive()
            current.push_check['conclusion']='failure'
            with self.assertRaisesRegex(ValueError,'push check app/job identity mismatch'):current.call_target()

    def test_lossless_old_B_H_E_producer_and_current_helper_are_distinct(self):
        a=self.data;root=self.review['root']
        git=lambda *args:subprocess.check_output(['git','-C',str(root),*args])
        self.assertEqual(git('show','-s','--format=%P',self.pr_execution).decode().split(),[a['base'],a['head']])
        self.assertEqual(git('rev-parse',self.pr_execution+'^{tree}'),git('rev-parse',a['head']+'^{tree}'))
        self.assertEqual(git('show','-s','--format=%P',a['merge']).decode().split(),[a['base']])
        self.assertEqual(self.context['source_scope_oid'],a['base'])
        self.assertEqual(self.review['plan']['source_review_identity']['source_scope_oid'],a['base'])
        different=[]
        for row in self.producer_snapshot['files']:
            actual=git('show',a['base']+':'+row['path'])
            self.assertEqual(hashlib.sha256(actual).hexdigest(),row['sha256'])
            self.assertEqual(actual,git('show',a['head']+':'+row['path']))
            if actual!=git('show',self.target+':'+row['path']):different.append(row['path'])
        self.assertIn('scripts/ci-tests.sh',different)
        self.assertNotEqual(git('show',a['head']+':scripts/ci-tests.sh'),(HERE.parents[1]/'scripts/ci-tests.sh').read_bytes())
        self.receipt.canonical_planner(self.source_plan['planner'])
        self.assertEqual(self.source_plan['base_oid'],a['base'])
        self.assertEqual(self.source_plan['head_oid'],a['head'])
        self.assertEqual(self.source_run['pull_requests'],[])
        self.assertEqual(self.receipt._workflow_job_details(self.source_check,a['repository']),(3701,3901))
        print('M2 historical fixture primitives: '+json.dumps({'different_paths':different,'source_selectors':{k:v for k,v in self.source_raw.items() if k.startswith('run_')},'source_resources':{k:v for k,v in self.source_raw.items() if k.startswith('needs_')},'B':a['base'],'H':a['head'],'E':self.pr_execution,'M':a['merge'],'T':self.target},sort_keys=True))

    def test_supported_old_source_observation_retains_actual_execution_provenance(self):
        result=self.call_source_observation()
        self.assertEqual(result['checks'][0]['workflow_sha'],self.pr_execution)
        inventory=next(row for row in self.producer_snapshot['files'] if row['path']=='scripts/ci-required-capability-test-inventory.tsv')
        self.assertEqual(result['coverage']['inventory_sha256'],inventory['sha256'])

if __name__ == '__main__':
    unittest.main(verbosity=2)
