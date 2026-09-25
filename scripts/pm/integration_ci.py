#!/usr/bin/env python3
"""Explicit manual revalidation on the default-branch GitHub Actions workflow."""
import argparse
import base64
import datetime
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from contextlib import contextmanager
from urllib.parse import urlparse
import zipfile

WORKFLOW='.github/workflows/rust.yml'
ARTIFACT='oasis7-required-plan-v1'
PLAN_V2_MEMBER='oasis7-required-plan-v2.json'
RESULT_V2_MEMBER='oasis7-required-result-v2.json'
OID=re.compile(r'[0-9a-f]{40}')
DISCOVERY_PAGE_SIZE=100
DISCOVERY_MAX_PAGES=10
KEYED_RUN_NAME='oasis7-ci|${{ github.event_name }}|${{ inputs.run_mode }}|${{ inputs.task_uid }}|${{ inputs.pr_number }}|${{ inputs.integration_base }}|${{ inputs.expected_head }}${{ inputs.request_key != \'\' && format(\'|{0}\', inputs.request_key) || \'\' }}'
LOCAL_TARGET_INVENTORY_SCHEMA='oasis7-ci-local-target-inventory/v1'
IMPACT_PROJECTION_MARKER='<!-- oasis7-impact-projection-b64:'

def gh(*args):
    return json.loads(subprocess.check_output(['gh',*args],text=True))

def git(root,*args):
    return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()

def pages(repository,path,key):
    result=[]
    for page in range(1,101):
        batch=gh('api',f'repos/{repository}/{path}?per_page=100&page={page}')[key]
        if not isinstance(batch,list): raise ValueError('integration API pagination malformed')
        result.extend(batch)
        if len(batch)<100: return result
    raise ValueError('integration API pagination limit exceeded')

def current_request(repository,uid,number,base,head,branch,request_key=None):
    """Select request identity before outcome; absence requires complete coverage."""
    matches=[]
    seen=set()
    legacy_cannot_integrate={}
    for page in range(1,DISCOVERY_MAX_PAGES+1):
        response=gh('api',f'repos/{repository}/actions/workflows/rust.yml/runs?event=workflow_dispatch&per_page={DISCOVERY_PAGE_SIZE}&page={page}')
        batch=response.get('workflow_runs')
        if not isinstance(batch,list): raise ValueError('integration discovery readback malformed')
        for run in batch:
            run_id=run.get('id')
            if type(run_id) is not int or run_id in seen: raise ValueError('integration discovery overlapping or invalid run identity')
            seen.add(run_id)
            if run.get('event')!='workflow_dispatch' or run.get('path')!=WORKFLOW or run.get('repository',{}).get('full_name')!=repository:
                raise ValueError('integration discovery workflow provenance uncertain')
            if not OID.fullmatch(str(run.get('head_sha',''))) or not isinstance(run.get('head_branch'),str):
                raise ValueError('integration discovery ref identity uncertain')
            parts=str(run.get('display_title','')).split('|')
            if len(parts) not in (7,8) or parts[:2]!=['oasis7-ci','workflow_dispatch']:
                if run['head_sha'] not in legacy_cannot_integrate:
                    source=gh('api',f"repos/{repository}/contents/{WORKFLOW}?ref={run['head_sha']}")
                    if source.get('type')!='file' or source.get('path')!=WORKFLOW or source.get('encoding')!='base64':
                        raise ValueError('integration effective workflow readback unavailable')
                    legacy_cannot_integrate[run['head_sha']]='integration_revalidation' not in base64.b64decode(source['content'],validate=False).decode()
                if legacy_cannot_integrate[run['head_sha']]: continue
                raise ValueError('integration current request identity unavailable before outcome')
            _,_,mode,request_uid,request_pr,request_base,request_head,*request_keys=parts
            if mode in ('full_escalation','newapi_bridge_package'): continue
            if mode!='integration_revalidation' or not re.fullmatch(r'task_[0-9a-f]{32}',request_uid) or not request_pr.isdigit() or not OID.fullmatch(request_base) or not OID.fullmatch(request_head):
                raise ValueError('integration current request identity malformed')
            if request_base!=base or request_head!=head: continue
            if request_key is not None:
                if not request_keys:
                    if request_uid==uid and int(request_pr)==int(number):
                        raise ValueError('integration request key unavailable before outcome')
                    continue
                if not re.fullmatch(r'sha256:[0-9a-f]{64}',request_keys[0]):
                    raise ValueError('integration request key malformed')
                if request_keys[0]!=request_key: continue
            elif request_keys:
                # Keyed requests belong to the new explicit request protocol;
                # the legacy selector cannot consume them without its key.
                if request_uid==uid and int(request_pr)==int(number):
                    raise ValueError('integration request key required before outcome')
                continue
            if (request_uid==uid)!=(int(request_pr)==int(number)):
                raise ValueError('integration request task/PR identity conflicts')
            if request_uid==uid and int(request_pr)==int(number):
                attempt=run.get('run_attempt')
                when=run.get('created_at')
                if type(attempt) is not int or attempt<1 or not isinstance(when,str): raise ValueError('integration attempt identity unavailable')
                if not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?(?:Z|[+-]\d{2}:\d{2})',when):
                    raise ValueError('integration request time malformed')
                try:
                    parsed_time=datetime.datetime.fromisoformat(when.replace('Z','+00:00'))
                    if parsed_time.tzinfo is None or parsed_time.utcoffset() is None:
                        raise ValueError('integration request time malformed')
                    timestamp=parsed_time.timestamp()
                    sort_time=parsed_time.astimezone(datetime.timezone.utc)
                except ValueError as exc: raise ValueError('integration request time malformed') from exc
                matches.append({'id':run_id,'run_attempt':attempt,'requested_at':timestamp,
                                '_requested_at_sort':sort_time,
                                'execution_sha':run['head_sha'],'execution_branch':run['head_branch']})
        if len(batch)<DISCOVERY_PAGE_SIZE:
            if not matches: return None
            selected=max(matches,key=lambda item:(item['_requested_at_sort'],item['id'],item['run_attempt']))
            selected.pop('_requested_at_sort')
            execution_sha=selected.pop('execution_sha')
            execution_branch=selected.pop('execution_branch')
            if execution_branch!=branch or (request_key is None and execution_sha!=base):
                raise ValueError('integration request base differs from trusted workflow ref')
            selected['workflow_run_head_sha']=execution_sha
            return selected
    raise ValueError('integration discovery range exhausted; current request coverage incomplete')

def identity(repository,uid,number,base,head,*,allow_base_advance=False):
    if type(number) is not int or number<1:
        raise ValueError('positive integer pull request number required')
    if not OID.fullmatch(base) or not OID.fullmatch(head): raise ValueError('exact base/source OIDs required')
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    if pr.get('state')!='open' or pr.get('merged') or (not allow_base_advance and pr['base']['sha']!=base) or pr['head']['sha']!=head:
        raise ValueError('PR source/target moved; request a new integration run')
    if pr['base']['repo']['full_name']!=repository or pr['head']['repo']['full_name']!=repository:
        raise ValueError('integration repository identity mismatch')
    # Only the complete canonical field establishes identity; history and
    # incidental mentions must not admit a different or ambiguous task.
    body=(pr.get('body') or '').replace('\r\n','\n')
    if (not re.fullmatch(r'task_[0-9a-f]{32}',uid)
            or re.findall(r'^Task:[^\n]*$',body,re.MULTILINE)!=['Task: '+uid]):
        raise ValueError('PR task identity mismatch')
    repo=gh('api',f'repos/{repository}')
    branch=repo['default_branch']
    if pr['base']['ref']!=branch: raise ValueError('integration target must be default branch')
    return pr,branch

def default_branch_head(repository,branch):
    ref=gh('api',f'repos/{repository}/git/ref/heads/{branch}')
    sha=ref.get('object',{}).get('sha')
    if not OID.fullmatch(str(sha or '')): raise ValueError('default branch head identity unavailable')
    return sha

def _adjacent_module(name):
    path=Path(__file__).resolve().with_name(name+'.py')
    if not path.is_file(): raise ImportError(f'trusted helper is unavailable: {name}')
    spec=importlib.util.spec_from_file_location(name,path)
    if spec is None or spec.loader is None: raise ImportError(f'trusted helper cannot be loaded: {name}')
    module=importlib.util.module_from_spec(spec)
    sys.modules[name]=module
    spec.loader.exec_module(module)
    return module

def github_executor_contract(repository,revision):
    try:
        helper=_adjacent_module('integration_executor_contract')
    except ImportError as exc:
        raise ValueError('EXECUTOR_CONTRACT_CHANGED: trusted contract helper unavailable') from exc
    contents={}
    for path in helper.EXECUTOR_CONTRACT_PATHS:
        source=gh('api',f'repos/{repository}/contents/{path}?ref={revision}')
        if source.get('type')!='file' or source.get('path')!=path or source.get('encoding')!='base64':
            raise ValueError('trusted executor contract source is unavailable')
        encoded=''.join(str(source.get('content','')).split())
        try: contents[path]=base64.b64decode(encoded,validate=True)
        except (TypeError,ValueError) as exc: raise ValueError('trusted executor contract source is malformed') from exc
    return helper.executor_contract_from_contents(contents)

def _github_file_bytes(repository,path,revision):
    source=gh('api',f'repos/{repository}/contents/{path}?ref={revision}')
    if (source.get('type')!='file' or source.get('path')!=path
            or source.get('encoding')!='base64'):
        raise ValueError(f'trusted workflow content is unavailable: {path}')
    try:
        encoded=''.join(str(source.get('content','')).split())
        raw=base64.b64decode(encoded,validate=True)
    except (TypeError,ValueError) as exc:
        raise ValueError(f'trusted workflow content is malformed: {path}') from exc
    if base64.b64encode(raw).decode('ascii')!=encoded:
        raise ValueError(f'trusted workflow content is noncanonical: {path}')
    return raw

def attempt_execution_jobs(repository,run_id,attempt,workflow_sha,app_id,*,require_completed=False,job_names=None):
    """Return exact check-backed job records from one live Actions attempt."""
    for value,label in ((run_id,'workflow run ID'),(attempt,'workflow attempt'),(app_id,'check app ID')):
        if type(value) is not int or value<1: raise ValueError(f'{label} must be a positive integer')
    if not OID.fullmatch(str(workflow_sha or '')): raise ValueError('workflow head SHA is invalid')
    run=gh('api',f'repos/{repository}/actions/runs/{run_id}')
    if (run.get('id')!=run_id or run.get('run_attempt')!=attempt
            or run.get('path')!=WORKFLOW or run.get('event')!='workflow_dispatch'
            or run.get('repository',{}).get('full_name')!=repository
            or run.get('head_sha')!=workflow_sha):
        raise ValueError('workflow job attempt provenance mismatch')
    raw_jobs=pages(repository,f'actions/runs/{run_id}/attempts/{attempt}/jobs','jobs')
    if not raw_jobs: raise ValueError('workflow attempt job list is empty')
    if job_names is not None and (not isinstance(job_names,(set,list,tuple))
                                  or any(not isinstance(name,str) or not name for name in job_names)):
        raise ValueError('workflow attempt job-name filter is malformed')
    selected_names=set(job_names) if job_names is not None else None
    records=[];job_ids=set();seen_job_names=set();check_ids=set()
    for job in raw_jobs:
        job_id=job.get('id');name=job.get('name');check_url=job.get('check_run_url')
        if (type(job_id) is not int or job_id<1 or job_id in job_ids
                or not isinstance(name,str) or not name or name in seen_job_names
                or job.get('run_id')!=run_id or job.get('run_attempt')!=attempt
                or job.get('head_sha')!=workflow_sha):
            raise ValueError('workflow attempt job identity is incomplete or ambiguous')
        job_ids.add(job_id);seen_job_names.add(name)
        if selected_names is not None and name not in selected_names:
            continue
        parsed=urlparse(str(check_url or ''))
        match=re.fullmatch(rf'/repos/{re.escape(repository)}/check-runs/([0-9]+)',parsed.path)
        if parsed.scheme!='https' or parsed.netloc!='api.github.com' or match is None or parsed.query or parsed.fragment:
            raise ValueError('workflow job check-run locator is malformed')
        check_id=int(match.group(1))
        if check_id in check_ids: raise ValueError('workflow attempt jobs share a check-run identity')
        check=gh('api',f'repos/{repository}/check-runs/{check_id}')
        check_app=check.get('app',{}).get('id')
        if (check.get('id')!=check_id or check.get('name')!=name
                or type(check_app) is not int or check_app!=app_id
                or check.get('head_sha')!=workflow_sha
                or check.get('status')!=job.get('status')
                or check.get('conclusion')!=job.get('conclusion')):
            raise ValueError('workflow attempt job check-run identity mismatch')
        labels=job.get('labels')
        if (not isinstance(labels,list) or any(not isinstance(label,str) or not label for label in labels)
                or len(labels)!=len(set(labels))):
            raise ValueError('workflow attempt job labels are malformed')
        status=job.get('status');conclusion=job.get('conclusion')
        if status not in ('queued','in_progress','completed'):
            raise ValueError('workflow attempt job status is unsupported')
        if status=='completed' and conclusion not in ('success','failure','cancelled','skipped','timed_out','action_required','neutral','stale'):
            raise ValueError('completed workflow attempt job conclusion is unsupported')
        if status!='completed' and conclusion is not None:
            raise ValueError('incomplete workflow attempt job has a conclusion')
        if require_completed and status!='completed':
            raise ValueError(f'workflow attempt job is not completed: {name}')
        records.append({
            'workflow_run_id':run_id,'run_attempt':attempt,'job_id':job_id,
            'job_name':name,'check_name':check.get('name'),'check_app_id':check_app,
            'check_run_id':check_id,'head_sha':workflow_sha,'status':status,
            'conclusion':conclusion,'labels':sorted(labels),
        })
        check_ids.add(check_id)
    if selected_names is not None and {item['job_name'] for item in records}!=selected_names:
        raise ValueError('workflow attempt is missing an exact requested job')
    return sorted(records,key=lambda item:item['job_name'])

def _read_artifact_member(repository,artifact,run_id,name,member):
    if (artifact.get('name')!=name or artifact.get('expired') is not False
            or type(artifact.get('id')) is not int or artifact['id']<1
            or artifact.get('workflow_run',{}).get('id')!=run_id):
        raise ValueError('keyed required artifact is expired or belongs to another run')
    raw=subprocess.check_output(['gh','api',f"repos/{repository}/actions/artifacts/{artifact['id']}/zip"])
    try:
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            if archive.namelist()!=[member]: raise ValueError('keyed required artifact members mismatch')
            return archive.read(member)
    except (zipfile.BadZipFile,KeyError) as exc:
        raise ValueError('keyed required artifact archive is malformed') from exc

def _trusted_source_attempt(plan,plan_artifact,result_artifacts,gate,request_key,run_id,attempt,app_id,check_id):
    """Create the closed R/A/check/job/artifact binding after exact live readback."""
    if (not isinstance(request_key,str) or not re.fullmatch(r'sha256:[0-9a-f]{64}',request_key)
            or any(type(value) is not int or value<1 for value in
                   (run_id,attempt,app_id,check_id,plan_artifact.get('id'),gate.get('job_id')))
            or gate.get('job_name')!='required-gate'
            or gate.get('workflow_run_id')!=run_id or gate.get('run_attempt')!=attempt
            or gate.get('check_app_id')!=app_id or gate.get('check_run_id')!=check_id):
        raise ValueError('trusted source attempt live identity is malformed')
    if (plan.get('request_key')!=request_key or plan.get('workflow_run_id')!=run_id
            or plan.get('run_attempt')!=attempt or plan.get('check_app_id')!=app_id
            or plan.get('check_run_id')!=check_id or plan.get('job_id')!=gate.get('job_id')
            or plan.get('job_name')!='required-gate'):
        raise ValueError('trusted source attempt differs from the validated required plan')
    results=[]
    for item in result_artifacts:
        payload=item.get('payload')
        unit_id=payload.get('unit_id') if isinstance(payload,dict) else None
        if not isinstance(unit_id,str) or not unit_id:
            raise ValueError('trusted source attempt result unit identity is malformed')
        results.append({'unit_id':unit_id,'artifact_id':item.get('artifact_id'),'name':item.get('name')})
    results.sort(key=lambda item:item['unit_id'])
    if ([item['unit_id'] for item in results]!=plan.get('required_test_units')
            or len({item['artifact_id'] for item in results})!=len(results)
            or len({item['unit_id'] for item in results})!=len(results)):
        raise ValueError('trusted source attempt result artifacts do not close the required plan')
    return {
        'schema':'oasis7-ci-trusted-source-attempt/v1',
        'request_key':request_key,
        'workflow_run_id':run_id,
        'run_attempt':attempt,
        'check_app_id':app_id,
        'check_run_id':check_id,
        'job_id':gate['job_id'],
        'job_name':'required-gate',
        'plan_artifact_id':plan_artifact['id'],
        'plan_artifact_name':plan_artifact['name'],
        'result_artifacts':results,
    }

def _validate_trusted_source_attempt(proof,plan):
    fields={
        'schema','request_key','workflow_run_id','run_attempt','check_app_id','check_run_id',
        'job_id','job_name','plan_artifact_id','plan_artifact_name','result_artifacts',
    }
    value=proof.get('trusted_source_attempt') if isinstance(proof,dict) else None
    if not isinstance(value,dict) or set(value)!=fields or value.get('schema')!='oasis7-ci-trusted-source-attempt/v1':
        raise ValueError('source trusted attempt binding is missing or malformed')
    if (not isinstance(value.get('request_key'),str)
            or not re.fullmatch(r'sha256:[0-9a-f]{64}',value['request_key'])):
        raise ValueError('source trusted attempt request key is malformed')
    for field in ('workflow_run_id','run_attempt','check_app_id','check_run_id','job_id','plan_artifact_id'):
        if type(value.get(field)) is not int or value[field]<1:
            raise ValueError('source trusted attempt ID is invalid: '+field)
    expected_scalars={
        'request_key':proof.get('request_key'),
        'workflow_run_id':proof.get('workflow_run_id'),
        'run_attempt':proof.get('run_attempt'),
        'check_app_id':proof.get('check_app_id'),
        'check_run_id':proof.get('check_run_id'),
        'job_id':proof.get('job_id'),
        'job_name':'required-gate',
        'plan_artifact_id':proof.get('required_plan_v2_artifact_id'),
        'plan_artifact_name':proof.get('required_plan_v2_artifact_name'),
    }
    for field in ('workflow_run_id','run_attempt','check_app_id','check_run_id','job_id','plan_artifact_id'):
        if type(expected_scalars[field]) is not int or expected_scalars[field]<1:
            raise ValueError('verified source attempt ID is invalid: '+field)
    if any(value.get(field)!=expected for field,expected in expected_scalars.items()):
        raise ValueError('source trusted attempt identity differs from verified proof')
    jobs=proof.get('execution_jobs')
    gate=[job for job in jobs if isinstance(job,dict) and job.get('job_name')=='required-gate'] if isinstance(jobs,list) else []
    if (len(gate)!=1 or gate[0].get('job_id')!=value['job_id']
            or type(gate[0].get('job_id')) is not int
            or type(gate[0].get('workflow_run_id')) is not int
            or type(gate[0].get('run_attempt')) is not int
            or type(gate[0].get('check_app_id')) is not int
            or type(gate[0].get('check_run_id')) is not int
            or gate[0].get('check_run_id')!=value['check_run_id']
            or gate[0].get('check_app_id')!=value['check_app_id']
            or gate[0].get('workflow_run_id')!=value['workflow_run_id']
            or gate[0].get('run_attempt')!=value['run_attempt']):
        raise ValueError('source trusted attempt required-gate job mismatch')
    artifacts=proof.get('required_result_v2_artifacts')
    if not isinstance(artifacts,list):
        raise ValueError('source trusted attempt result artifact set is malformed')
    expected_results=[]
    for artifact in artifacts:
        if (not isinstance(artifact,dict) or set(artifact)!={'artifact_id','name','payload'}
                or not isinstance(artifact.get('payload'),dict)):
            raise ValueError('source trusted attempt result artifact is malformed')
        unit_id=artifact['payload'].get('unit_id')
        if (not isinstance(unit_id,str) or not unit_id
                or type(artifact.get('artifact_id')) is not int or artifact['artifact_id']<1
                or not isinstance(artifact.get('name'),str)):
            raise ValueError('source trusted attempt result locator is malformed')
        expected_results.append({'unit_id':unit_id,'artifact_id':artifact['artifact_id'],'name':artifact['name']})
    expected_results.sort(key=lambda item:item['unit_id'] if isinstance(item['unit_id'],str) else '')
    observed=value.get('result_artifacts')
    if (not isinstance(observed,list) or any(
                not isinstance(item,dict) or set(item)!={'unit_id','artifact_id','name'}
                or not isinstance(item.get('unit_id'),str) or not item['unit_id']
                or type(item.get('artifact_id')) is not int or item['artifact_id']<1
                or not isinstance(item.get('name'),str)
                for item in observed)
            or observed!=expected_results
            or [item['unit_id'] for item in observed]!=plan.get('required_test_units')):
        raise ValueError('source trusted attempt result artifacts differ from exact readback')
    if (len({item['artifact_id'] for item in observed})!=len(observed)
            or len({item['unit_id'] for item in observed})!=len(observed)
            or value['plan_artifact_id'] in {item['artifact_id'] for item in observed}):
        raise ValueError('source trusted attempt artifact IDs are duplicate')
    for item in observed:
        expected_name='oasis7-required-result-v2-{}-a{}-{}'.format(
            value['workflow_run_id'],value['run_attempt'],
            hashlib.sha256(item['unit_id'].encode('utf-8')).hexdigest(),
        )
        if item['name']!=expected_name:
            raise ValueError('source trusted attempt result artifact name is invalid')
    return value

def _adjacent_source_matches_w(repository,revision,relative):
    try:
        local=Path(__file__).resolve().parents[2]/relative
        trusted=_github_file_bytes(repository,relative,revision)
    except (OSError,ValueError) as exc:
        raise ValueError(f'trusted keyed producer source is unavailable: {relative}') from exc
    if not local.is_file() or local.read_bytes()!=trusted:
        raise ValueError(f'local keyed consumer source differs from trusted W: {relative}')

def read_keyed_v2_evidence(repository,run,run_id,attempt,app_id,check,*,
                           request_key,request_identity,base,head,policy_context,
                           approved_executor_contract_digests,execution_jobs):
    """Resolve and validate attempt-scoped v2 artifacts against live W/R/A/checks."""
    artifacts=pages(repository,f'actions/runs/{run_id}/artifacts','artifacts')
    try:
        artifact_helper=_adjacent_module('ci_required_artifact_v2')
    except ImportError as exc:
        raise ValueError('trusted required-artifact v2 helper is unavailable') from exc
    workflow_sha=run['head_sha']
    for relative in ('scripts/pm/ci_required_artifact_v2.py',
                     'scripts/pm/ci_required_inventory.py',
                     'scripts/pm/ci_input_scope.py'):
        _adjacent_source_matches_w(repository,workflow_sha,relative)
    plan_name=artifact_helper.plan_artifact_name(run_id,attempt)
    plans=[item for item in artifacts if item.get('name')==plan_name]
    if len(plans)!=1: raise ValueError('keyed required-plan v2 artifact missing or ambiguous')
    plan_artifact=plans[0]
    plan_raw=_read_artifact_member(repository,plan_artifact,run_id,plan_name,PLAN_V2_MEMBER)
    plan=artifact_helper.parse_payload(plan_raw,label='required-plan v2')
    try: artifact_helper.validate_plan_payload(plan,require_complete=True)
    except ValueError as exc: raise ValueError('keyed required-plan v2 payload is invalid: '+str(exc)) from exc
    expected_plan={
        'request_key':request_key,'request_identity':request_identity,
        'repository':repository,'task_uid':request_identity['task_uid'],
        'pr_number':int(request_identity['pr_number']),
        'bootstrap_epoch':request_identity['bootstrap_epoch'],
        'source_head_oid':head,'integration_base_oid':base,
        'workflow_ref':f'{repository}/{WORKFLOW}@refs/heads/{run["head_branch"]}',
        'workflow_sha':workflow_sha,'workflow_run_id':run_id,'run_attempt':attempt,
        'check_name':'required-gate','check_app_id':int(app_id),'check_run_id':int(check['id']),
        'effective_policy_identity':policy_context['effective_policy_identity'],
    }
    if any(plan.get(field)!=value for field,value in expected_plan.items()):
        raise ValueError('keyed required-plan v2 identity differs from request and live run')
    if plan.get('planner_inventory_authority')!=policy_context.get('planner_inventory_authority'):
        raise ValueError('keyed planner inventory authority differs from trusted W policy')
    if plan.get('executor_contract_digest') not in approved_executor_contract_digests:
        raise ValueError('keyed required-plan executor contract is not approved')
    source_compare=gh('api',f'repos/{repository}/compare/{base}...{head}')
    source_scope=source_compare.get('merge_base_commit',{}).get('sha')
    if (not OID.fullmatch(str(source_scope or '')) or source_scope!=plan.get('source_scope_oid')
            or plan.get('planner_output',{}).get('source_scope_base')!=source_scope):
        raise ValueError('keyed required-plan source scope differs from the verified W projection')
    request_projection=request_identity.get('source_projection_digest')
    if plan.get('planner_output',{}).get('impact_projection_digest')!=request_projection:
        raise ValueError('keyed required-plan projection differs from the request identity')
    invocation=plan.get('planner_invocation')
    if (not isinstance(invocation,dict) or invocation.get('scope_base_oid')!=source_scope
            or invocation.get('planner_authority_oid')!=workflow_sha
            or invocation.get('producer')!={
                'run_id':run_id,'run_attempt':attempt,'check_app_id':int(app_id),
                'check_run_id':int(check['id']),
            }):
        raise ValueError('keyed required-plan W invocation does not bind source scope and producer')
    live_jobs={job['job_name']:job for job in execution_jobs}
    gate=live_jobs.get('required-gate')
    if (gate is None or gate['job_id']!=plan.get('job_id')
            or gate['check_run_id']!=check['id'] or gate['status']!='completed'
            or gate['conclusion']!='success'):
        raise ValueError('keyed plan is not backed by its exact successful required-gate job')

    c2=_adjacent_module('ci_input_scope')
    issuer=plan['planner_inventory_issuer']
    trusted_inventory={**issuer,'producer':{**issuer['producer'],'artifact_id':plan_artifact['id']}}
    try:
        c2.validate_input_scope_snapshot(plan['input_scope'],trusted_planner_inventory=trusted_inventory)
        digest=c2.planner_inventory_digest(
            plan['unit_specs'],plan['product_corpus'],plan['tested_commit_oid'],plan['tested_tree_oid'],
        )
    except (ValueError,KeyError,TypeError) as exc:
        raise ValueError('keyed required-plan C2 inventory validation failed') from exc
    if digest!=issuer['inventory_digest'] or plan.get('planner_inventory_digest')!=digest:
        raise ValueError('keyed required-plan C2 inventory digest mismatch')

    expected_unit_ids=plan['required_test_units']
    result_artifacts=[];used_ids={plan_artifact['id']}
    for unit_id in expected_unit_ids:
        name=artifact_helper.result_artifact_name(run_id,attempt,unit_id)
        matches=[item for item in artifacts if item.get('name')==name]
        if len(matches)!=1: raise ValueError(f'keyed result artifact missing or ambiguous: {unit_id}')
        artifact=matches[0]
        if artifact['id'] in used_ids: raise ValueError('keyed v2 artifacts share an artifact ID')
        raw=_read_artifact_member(repository,artifact,run_id,name,RESULT_V2_MEMBER)
        payload=artifact_helper.parse_payload(raw,label='required-result v2')
        try:
            artifact_helper.validate_result_payload(
                payload,plan=plan,plan_artifact_id=plan_artifact['id'],expected_unit_id=unit_id,
            )
        except ValueError as exc:
            raise ValueError(f'keyed required-result v2 payload is invalid: {unit_id}: {exc}') from exc
        for job in payload['execution_jobs']:
            if live_jobs.get(job['job_name'])!=job:
                raise ValueError(f'keyed result job proof differs from live attempt: {job["job_name"]}')
        result_artifacts.append({'artifact_id':artifact['id'],'name':name,'payload':payload})
        used_ids.add(artifact['id'])
    trusted_source_attempt=_trusted_source_attempt(
        plan,plan_artifact,result_artifacts,gate,request_key,run_id,attempt,int(app_id),int(check['id']),
    )
    return {
        'required_plan_v2_artifact_id':plan_artifact['id'],
        'required_plan_v2_artifact_name':plan_name,
        'required_plan_v2_payload':plan,
        'required_result_v2_artifacts':result_artifacts,
        'source_scope_oid':source_scope,
        'workflow_run_id':run_id,'run_attempt':attempt,
        'check_app_id':int(app_id),'check_run_id':int(check['id']),
        'job_id':gate['job_id'],'job_name':gate['job_name'],
        'execution_jobs':execution_jobs,
        'trusted_planner_inventory':trusted_inventory,
        'trusted_source_attempt':trusted_source_attempt,
    }

def trusted_policy_context(repository,branch,workflow_sha,default_branch_sha):
    """Resolve policy from authenticated bytes at W; no caller object grants authority."""
    policy_path='scripts/pm/ci_reuse_policy.py'
    config_path='scripts/ci-required-scope.v2.json'
    repo=gh('api',f'repos/{repository}')
    if repo.get('full_name')!=repository or repo.get('default_branch')!=branch:
        raise ValueError('trusted policy source is not the repository default branch')
    policy_source=_github_file_bytes(repository,policy_path,workflow_sha)
    planner_config=_github_file_bytes(repository,config_path,workflow_sha)
    helper=_adjacent_module('ci_reuse_policy')
    local_source=Path(helper.__file__).read_bytes()
    return helper.resolve_trusted_policy_context(
        repository=repository,default_branch=branch,
        workflow_ref=f'{repository}/{WORKFLOW}@refs/heads/{branch}',
        workflow_sha=workflow_sha,default_branch_sha=default_branch_sha,
        policy_source=policy_source,local_policy_source=local_source,
        planner_config=planner_config,
    )

def trusted_policy_context_for_run(repository,branch,run):
    if (not isinstance(run,dict) or run.get('path')!=WORKFLOW
            or run.get('repository',{}).get('full_name')!=repository
            or run.get('head_branch')!=branch):
        raise ValueError('trusted policy workflow run identity is unavailable')
    workflow_sha=run.get('head_sha')
    if not OID.fullmatch(str(workflow_sha or '')):
        raise ValueError('trusted policy workflow SHA is invalid')
    return trusted_policy_context(repository,branch,workflow_sha,workflow_sha)

def _yaml_mapping_entries(source):
    """Read mapping entries from the small YAML surface used by workflow metadata.

    This is intentionally a conservative scanner rather than a general YAML
    parser: it strips comments outside quoted scalars and recognizes mapping
    keys/indentation only. Readiness fails closed for unsupported formatting.
    """
    entry_re=re.compile(r"^( *)(?:\"((?:[^\"\\]|\\.)*)\"|'([^']*)'|([A-Za-z0-9_.-]+))\s*:\s*(.*?)\s*$")

    def without_comment(line):
        quote=None;escaped=False
        for index,char in enumerate(line):
            if quote=='"':
                if escaped: escaped=False
                elif char=='\\': escaped=True
                elif char=='"': quote=None
            elif quote=="'":
                if char=="'": quote=None
            elif char in ('"',"'"):
                quote=char
            elif char=='#' and (index==0 or line[index-1].isspace()):
                return line[:index]
        return line

    entries=[]
    for line_number,line in enumerate(source.splitlines()):
        clean=without_comment(line)
        if '\t' in clean[:len(clean)-len(clean.lstrip())]:
            continue
        match=entry_re.match(clean)
        if match is None: continue
        indent=len(match.group(1))
        key=match.group(2) if match.group(2) is not None else match.group(3)
        if key is None: key=match.group(4)
        entries.append({'indent':indent,'key':key,'value':match.group(5),'line':line_number})
    return entries

def keyed_request_workflow_ready(workflow):
    entries=_yaml_mapping_entries(workflow)

    def direct_children(parent):
        descendants=[]
        for item in entries:
            if item['line']<=parent['line']: continue
            if item['indent']<=parent['indent']: break
            descendants.append(item)
        if not descendants: return []
        child_indent=min(item['indent'] for item in descendants)
        return [item for item in descendants if item['indent']==child_indent]

    def one_mapping_child(parent,key):
        found=[item for item in direct_children(parent) if item['key']==key]
        return found[0] if len(found)==1 and found[0]['value']=='' else None

    triggers=[item for item in entries if item['indent']==0 and item['key']=='on']
    run_names=[item for item in entries if item['indent']==0 and item['key']=='run-name']
    if len(triggers)!=1 or triggers[0]['value']!='' or len(run_names)!=1:
        return False
    workflow_dispatch=one_mapping_child(triggers[0],'workflow_dispatch')
    if workflow_dispatch is None: return False
    inputs=one_mapping_child(workflow_dispatch,'inputs')
    if inputs is None: return False
    input_fields=direct_children(inputs)
    input_names=(
        'run_mode','task_uid','pr_number','integration_base','expected_head',
        'request_key','validation_request_b64',
    )
    if any(len([item for item in input_fields if item['key']==key])!=1
           or next(item for item in input_fields if item['key']==key)['value']!=''
           for key in input_names):
        return False

    def has_input_contract(name, expected_type):
        declaration=next(item for item in input_fields if item['key']==name)
        children=direct_children(declaration)
        values={}
        for child in children:
            if child['key'] in values:
                return False
            values[child['key']]=child['value']
        return (values.get('type')==expected_type and values.get('required')=='false'
                and 'default' not in values)

    run_name=run_names[0]['value']
    return (has_input_contract('request_key','string')
            and has_input_contract('validation_request_b64','string')
            and run_name==KEYED_RUN_NAME)

def git_common_dir(root):
    value=Path(git(root,'rev-parse','--git-common-dir'))
    if not value.is_absolute(): value=Path(root).resolve()/value
    return value.resolve()/ 'oasis7' / 'integration-requests'

def compose(root,base,head,worktree_path=None):
    for oid in (base,head):
        if not OID.fullmatch(oid): raise ValueError('exact integration OIDs required')
    scope=git(root,'merge-base','--all',base,head).splitlines()
    if len(scope)!=1: raise ValueError('ambiguous scope ancestor')
    tree=git(root,'merge-tree','--write-tree',base,head).splitlines()[0]
    env={**os.environ,'GIT_AUTHOR_NAME':'Integration CI','GIT_AUTHOR_EMAIL':'ci@example.invalid','GIT_COMMITTER_NAME':'Integration CI','GIT_COMMITTER_EMAIL':'ci@example.invalid','GIT_AUTHOR_DATE':'2000-01-01T00:00:00Z','GIT_COMMITTER_DATE':'2000-01-01T00:00:00Z'}
    commit=subprocess.check_output(['git','-C',str(root),'commit-tree',tree,'-p',base,'-p',head,'-m','Exact integration revalidation'],env=env,text=True).strip()
    if worktree_path is None:
        git(root,'checkout','--detach',commit)
    else:
        destination=Path(worktree_path).resolve()
        if destination.exists(): raise ValueError('integration worktree destination already exists')
        git(root,'worktree','add','--detach',str(destination),commit)
    result={'base_oid':base,'head_oid':head,'scope_base_oid':scope[0],'tested_tree_oid':tree,'tested_commit_oid':commit}
    if worktree_path is not None: result['integration_worktree']=str(destination)
    return result

def _exact_merge_base(root,left,right):
    try:
        values=git(root,'merge-base','--all',left,right).splitlines()
    except subprocess.CalledProcessError as exc:
        raise ValueError('target ancestry cannot be resolved') from exc
    if len(values)!=1 or not OID.fullmatch(values[0]):
        raise ValueError('target ancestry is detached or ambiguous')
    return values[0]

@contextmanager
def _local_target_worktrees(repository_root,branch,pr_number,target_oid,head_oid,source_scope_oid):
    """Fetch authenticated Q/H into a scratch object store and compose exact M/T."""
    for oid,label in ((target_oid,'current PR target'),(head_oid,'source head'),
                      (source_scope_oid,'source scope')):
        if not isinstance(oid,str) or not OID.fullmatch(oid):
            raise ValueError(f'{label} must be a full commit OID')
    if type(pr_number) is not int or pr_number<1:
        raise ValueError('positive pull request number required')
    try:
        subprocess.run(['git','-C',str(repository_root),'check-ref-format','--branch',branch],
                       check=True,capture_output=True,text=True)
        remote_url=git(repository_root,'remote','get-url','origin')
    except (subprocess.CalledProcessError,OSError) as exc:
        raise ValueError('trusted GitHub remote or default branch is unavailable') from exc
    with tempfile.TemporaryDirectory(prefix='oasis7-ci-local-target-') as temp:
        scratch=Path(temp)/'repo'
        try:
            subprocess.run(['git','init','--quiet',str(scratch)],check=True,capture_output=True,text=True)
            git(scratch,'remote','add','origin',remote_url)
            git(scratch,'fetch','--no-tags','origin',
                f'+refs/heads/{branch}:refs/remotes/oasis7/target',
                f'+refs/pull/{pr_number}/head:refs/remotes/oasis7/source')
            fetched_target=git(scratch,'rev-parse','--verify','refs/remotes/oasis7/target^{commit}')
            fetched_head=git(scratch,'rev-parse','--verify','refs/remotes/oasis7/source^{commit}')
        except (subprocess.CalledProcessError,OSError) as exc:
            raise ValueError('exact Q/H refs cannot be fetched into the isolated target store') from exc
        if fetched_target!=target_oid or fetched_head!=head_oid:
            raise ValueError('fetched default-branch or PR head differs from authenticated Q/H')

        target_scope=_exact_merge_base(scratch,target_oid,head_oid)
        if target_scope!=source_scope_oid:
            raise ValueError('current Q/H merge base differs from the frozen source scope')
        try:
            merge=subprocess.run(
                ['git','-C',str(scratch),'merge-tree','--write-tree',target_oid,head_oid],
                check=True,capture_output=True,text=True,
            )
        except (subprocess.CalledProcessError,OSError) as exc:
            raise ValueError('exact Q/H merge has conflicts or cannot be composed') from exc
        merge_lines=merge.stdout.splitlines()
        if not merge_lines or not OID.fullmatch(merge_lines[0]):
            raise ValueError('exact Q/H merge tree identity is unavailable')
        tree_oid=merge_lines[0]
        env={**os.environ,'GIT_AUTHOR_NAME':'Integration CI','GIT_AUTHOR_EMAIL':'ci@example.invalid',
             'GIT_COMMITTER_NAME':'Integration CI','GIT_COMMITTER_EMAIL':'ci@example.invalid',
             'GIT_AUTHOR_DATE':'2000-01-01T00:00:00Z','GIT_COMMITTER_DATE':'2000-01-01T00:00:00Z'}
        try:
            commit_oid=subprocess.check_output(
                ['git','-C',str(scratch),'commit-tree',tree_oid,'-p',target_oid,'-p',head_oid,
                 '-m','Exact integration revalidation'],env=env,text=True,
            ).strip()
            if not OID.fullmatch(commit_oid):
                raise ValueError('exact Q/H merge commit identity is invalid')
            planner_root=Path(temp)/'planner-w'
            target_root=Path(temp)/'target-m'
            git(scratch,'worktree','add','--detach',str(planner_root),target_oid)
            git(scratch,'worktree','add','--detach',str(target_root),commit_oid)
            planner_head=git(planner_root,'rev-parse','--verify','HEAD^{commit}')
            target_head=git(target_root,'rev-parse','--verify','HEAD^{commit}')
            target_tree=git(target_root,'show','-s','--format=%T','HEAD')
            parents=git(target_root,'rev-list','--parents','-n','1','HEAD').split()
            if (planner_head!=target_oid or target_head!=commit_oid or target_tree!=tree_oid
                    or parents!=[commit_oid,target_oid,head_oid]):
                raise ValueError('isolated target checkout differs from exact Q/H merge M/T')
            for path,label in ((planner_root,'trusted W checkout at Q'),(target_root,'target checkout at M')):
                if git(path,'status','--porcelain','--untracked-files=all'):
                    raise ValueError(f'{label} is not clean')
        except (subprocess.CalledProcessError,OSError) as exc:
            raise ValueError('isolated W/Q or M/T checkout could not be verified') from exc
        yield {
            'repository_root':scratch,'planner_root':planner_root,'target_root':target_root,
            'target_oid':target_oid,'head_oid':head_oid,'source_scope_oid':source_scope_oid,
            'input_scope_commit_oid':commit_oid,'input_scope_tree_oid':tree_oid,
        }

def _json_object_without_duplicate_keys(pairs):
    result={}
    for key,value in pairs:
        if key in result: raise ValueError('impact projection has a duplicate JSON key')
        result[key]=value
    return result

def _projection_from_pr_body(body):
    if not isinstance(body,str) or len(body.encode('utf-8'))>60*1024:
        raise ValueError('PR impact projection body is missing or oversized')
    if body.count(IMPACT_PROJECTION_MARKER)!=1:
        raise ValueError('PR impact projection marker is missing or ambiguous')
    matches=re.findall(
        r'(?m)^<!-- oasis7-impact-projection-b64:\s*([A-Za-z0-9+/=]+)\s*-->[ \t]*$',body,
    )
    if len(matches)!=1:
        raise ValueError('PR impact projection marker is malformed')
    encoded=matches[0]
    try:
        raw=base64.b64decode(encoded,validate=True)
        if base64.b64encode(raw).decode('ascii')!=encoded:
            raise ValueError('PR impact projection base64 is noncanonical')
        projection=json.loads(raw.decode('utf-8'),object_pairs_hook=_json_object_without_duplicate_keys)
    except (ValueError,UnicodeDecodeError,json.JSONDecodeError) as exc:
        raise ValueError('PR impact projection payload is malformed') from exc
    if not isinstance(projection,dict):
        raise ValueError('PR impact projection payload must be an object')
    return raw,projection

def _load_checkout_module(path,module_name):
    if not path.is_file() or path.is_symlink():
        raise ValueError(f'trusted W module is unavailable: {path.name}')
    spec=importlib.util.spec_from_file_location(module_name,path)
    if spec is None or spec.loader is None:
        raise ValueError(f'trusted W module cannot be loaded: {path.name}')
    module=importlib.util.module_from_spec(spec)
    sys.modules[module_name]=module
    try: spec.loader.exec_module(module)
    except Exception as exc: raise ValueError(f'trusted W module failed to load: {path.name}') from exc
    return module

def _validate_keyed_source_plan(repository,task_uid,pr_number,proof):
    required=(
        'request_key','request_identity','integration_base_oid','source_scope_oid',
        'workflow_run_id','run_attempt','check_app_id','check_run_id','trusted_policy_context',
        'effective_policy_identity','planner_inventory_authority','required_plan_v2_artifact_id',
        'required_plan_v2_artifact_name','trusted_source_attempt',
        'required_plan_v2_payload','required_result_v2_artifacts','execution_jobs',
        'trusted_planner_inventory',
    )
    if not isinstance(proof,dict) or any(field not in proof for field in required):
        raise ValueError('source required-plan v2 proof is incomplete')
    helper=_adjacent_module('ci_required_artifact_v2')
    try:
        request_identity=helper.validate_request_identity(proof['request_identity'])
        plan=helper.validate_plan_payload(proof['required_plan_v2_payload'],require_complete=True)
    except (KeyError,TypeError,ValueError) as exc:
        raise ValueError('source required-plan v2 proof is invalid') from exc
    request_key=helper.request_key_for_identity(request_identity)
    if proof['request_key']!=request_key or plan['request_key']!=request_key:
        raise ValueError('source request key differs from the exact request identity')
    if plan.get('request_identity')!=request_identity:
        raise ValueError('source required-plan request identity differs from verified proof')
    if (request_identity['repository']!=repository or request_identity['task_uid']!=task_uid
            or request_identity['pr_number']!=pr_number
            or plan['repository']!=repository or plan['task_uid']!=task_uid
            or plan['pr_number']!=pr_number):
        raise ValueError('source required-plan task or PR identity mismatch')
    base=proof['integration_base_oid'];head=plan['source_head_oid'];scope=proof['source_scope_oid']
    for oid,label in ((base,'immutable source integration base'),(head,'source head'),
                      (scope,'source scope')):
        if not isinstance(oid,str) or not OID.fullmatch(oid):
            raise ValueError(f'{label} is invalid')
    if (plan['integration_base_oid']!=base or plan['source_scope_oid']!=scope
            or plan['source_head_oid']!=request_identity['source_head_oid']
            or proof.get('workflow_run_id')!=plan['workflow_run_id']
            or proof.get('run_attempt')!=plan['run_attempt']
            or proof.get('check_app_id')!=plan['check_app_id']
            or proof.get('check_run_id')!=plan['check_run_id']
            or proof.get('effective_policy_identity')!=plan['effective_policy_identity']
            or proof.get('planner_inventory_authority')!=plan['planner_inventory_authority']):
        raise ValueError('source required-plan run, policy, B/H/S identity mismatch')
    policy_context=proof['trusted_policy_context']
    if (not isinstance(policy_context,dict)
            or policy_context.get('repository')!=repository
            or policy_context.get('workflow_ref')!=plan['workflow_ref']
            or policy_context.get('effective_policy_identity')!=plan['effective_policy_identity']
            or policy_context.get('planner_inventory_authority')!=plan['planner_inventory_authority']
            or policy_context.get('workflow_sha')!=plan['workflow_sha']):
        raise ValueError('source effective policy or W authority is not bound to the plan')
    artifact_id=proof['required_plan_v2_artifact_id']
    if type(artifact_id) is not int or artifact_id<1:
        raise ValueError('source required-plan artifact identity is invalid')
    issuer=plan['planner_inventory_issuer']
    expected_inventory={**issuer,'producer':{**issuer['producer'],'artifact_id':artifact_id}}
    if proof['trusted_planner_inventory']!=expected_inventory:
        raise ValueError('source planner inventory is not bound to the live artifact')
    _validate_trusted_source_attempt(proof,plan)
    invocation=plan['planner_invocation']
    if (invocation.get('base_ref')!=base or invocation.get('head_ref')!=head
            or invocation.get('scope_base_oid')!=scope or invocation.get('task_uid')!=task_uid
            or invocation.get('impact_projection_sha256')!=request_identity['source_projection_digest']):
        raise ValueError('source planner invocation differs from immutable B/H/S and projection')
    return plan,request_identity,policy_context

def _validate_source_request_journal(repository,proof,plan,request_identity):
    """Rebind the source v2 run to this checkout's observed request journal."""
    try:
        request_helper=_adjacent_module('integration_executor_contract')
        root=Path(__file__).resolve().parents[2]
        path=request_helper._request_path(git_common_dir(root),proof['request_key'])
        record=request_helper._read_request_record(path,proof['request_key'])
    except (ImportError,OSError,ValueError,KeyError) as exc:
        raise ValueError('source validation request journal is unavailable or invalid') from exc
    if (record.get('status')!='observed' or record.get('identity')!=request_identity
            or record.get('integration_base_oid')!=proof.get('integration_base_oid')
            or record.get('run_id')!=plan['workflow_run_id']
            or record.get('run_attempt')>plan['run_attempt']
            or request_identity.get('repository')!=repository):
        raise ValueError('source validation request journal does not bind B and the observed run')

def trusted_local_target_inventory(repository,task_uid,pr_number,source_proof):
    """Recompute a local Q observation from trusted W over exact M=merge(Q,H).

    This reports target applicability inputs only. It does not attest tests,
    produce execution IDs or turn local state into CI success evidence.
    """
    plan,request_identity,_source_policy=_validate_keyed_source_plan(
        repository,task_uid,pr_number,source_proof,
    )
    _validate_source_request_journal(repository,source_proof,plan,request_identity)
    base=source_proof['integration_base_oid'];head=plan['source_head_oid']
    source_scope=source_proof['source_scope_oid']
    live_pr,branch=identity(repository,task_uid,pr_number,base,head,allow_base_advance=True)
    target_oid=live_pr.get('base',{}).get('sha')
    if not isinstance(target_oid,str) or not OID.fullmatch(target_oid):
        raise ValueError('current PR target Q is invalid')
    if default_branch_head(repository,branch)!=target_oid:
        raise ValueError('current PR target differs from the live default-branch head')
    selected=current_request(
        repository,task_uid,pr_number,base,head,branch,request_key=source_proof['request_key'],
    )
    if (selected is None or selected.get('id')!=plan['workflow_run_id']
            or selected.get('run_attempt')!=plan['run_attempt']):
        raise ValueError('source required-plan is not the latest keyed workflow attempt')
    _source_check,verified_source_proof=verified_run(
        repository,task_uid,pr_number,base,head,plan['workflow_run_id'],plan['check_app_id'],
        request_key=source_proof['request_key'],expected_attempt=plan['run_attempt'],
        request_identity=request_identity,
        effective_policy=source_proof['trusted_policy_context']['effective_policy'],
    )
    verified_plan,verified_identity,_verified_policy=_validate_keyed_source_plan(
        repository,task_uid,pr_number,verified_source_proof,
    )
    if verified_plan!=plan or verified_identity!=request_identity:
        raise ValueError('live source required-plan evidence changed during target assessment')
    plan=verified_plan
    source_proof=verified_source_proof
    if base!=target_oid:
        comparison=gh('api',f'repos/{repository}/compare/{base}...{target_oid}')
        if (not isinstance(comparison,dict)
                or comparison.get('base_commit',{}).get('sha')!=base
                or comparison.get('head_commit',{}).get('sha')!=target_oid
                or comparison.get('merge_base_commit',{}).get('sha')!=base):
            raise ValueError('immutable source integration base is not an ancestor of current Q')

    target_policy=trusted_policy_context(repository,branch,target_oid,target_oid)
    if (target_policy.get('repository')!=repository
            or target_policy.get('workflow_ref')!=f'{repository}/{WORKFLOW}@refs/heads/{branch}'
            or target_policy.get('workflow_sha')!=target_oid):
        raise ValueError('current Q trusted policy context is malformed')

    with _local_target_worktrees(
        Path(__file__).resolve().parents[2],branch,pr_number,target_oid,head,source_scope,
    ) as worktrees:
        planner_root=worktrees['planner_root'];target_root=worktrees['target_root']
        workflow_ref=target_policy['workflow_ref']
        authority=target_policy.get('planner_inventory_authority')
        if (not isinstance(authority,dict) or authority.get('planner_authority_oid')!=target_oid
                or authority.get('repository')!=repository or authority.get('workflow_ref')!=workflow_ref):
            raise ValueError('current Q planner authority is malformed')
        projection_raw,projection_value=_projection_from_pr_body(live_pr.get('body'))
        source_invocation=plan['planner_invocation']
        changed_paths=source_invocation['changed_paths']
        if changed_paths!=sorted(set(changed_paths)):
            raise ValueError('source planner invocation changed paths are noncanonical')
        if projection_value.get('projection_digest')!=request_identity['source_projection_digest']:
            raise ValueError('PR projection differs from the immutable request digest')
        source_diff=git(worktrees['repository_root'],'diff','--name-only',f'{source_scope}..{head}').splitlines()
        if source_diff!=changed_paths:
            raise ValueError('source planner changed paths differ from the exact S..H source diff')

        projection_path=Path(worktrees['repository_root'])/'oasis7-source-projection.json'
        projection_path.write_bytes(projection_raw)
        impact_helper=_load_checkout_module(
            planner_root/'scripts/pm/workflow-impact-projection.py',
            'trusted_local_target_workflow_impact_projection',
        )
        try:
            projection=impact_helper.load_verified_projection(
                projection_path,
                expected={'task_uid':task_uid,'source_head_oid':head,
                          'scope_base_oid':source_scope,'changed_paths':changed_paths},
                repo_root=worktrees['repository_root'],
            )
        except (OSError,TypeError,ValueError) as exc:
            raise ValueError('source projection cannot be replayed under current trusted W') from exc
        if projection.get('projection_digest')!=request_identity['source_projection_digest']:
            raise ValueError('trusted W projection digest differs from the immutable request')

        planner_script=planner_root/'scripts/plan-rust-required-scope.py'
        config_path=planner_root/'scripts/ci-required-scope.v2.json'
        command=[
            sys.executable,str(planner_script),'--event-name','workflow_dispatch',
            '--run-mode','integration_revalidation','--config',str(config_path),
            '--base-ref',base,'--head-ref',head,'--task-uid',task_uid,
            '--scope-base-oid',source_scope,'--impact-projection',str(projection_path),
        ]
        for path in changed_paths:
            command.extend(('--changed-path',path))
        try:
            replay=subprocess.run(command,cwd=planner_root,check=True,capture_output=True,text=True)
        except (OSError,subprocess.CalledProcessError) as exc:
            raise ValueError('current Q trusted planner replay failed') from exc
        planner_output=replay.stdout
        inventory_module=_load_checkout_module(
            planner_root/'scripts/pm/ci_required_inventory.py',
            'trusted_local_target_required_inventory',
        )
        try:
            source_product_environment=inventory_module.trusted_product_environment_from_plan(
                plan,source_proof['execution_jobs'],source_proof['trusted_source_attempt'],
            )
        except (OSError,ValueError,KeyError,TypeError) as exc:
            raise ValueError('source product checker environment is not bound to the exact attempt') from exc
        try:
            target_inventory=inventory_module.build_required_inventory(
                planner_root,target_root,worktrees['input_scope_commit_oid'],planner_output,
                repository=repository,workflow_ref=workflow_ref,
                planner_authority_oid=target_oid,event_name='workflow_dispatch',
                run_mode='integration_revalidation',changed_paths=changed_paths,
                base_ref=base,head_ref=head,task_uid=task_uid,
                scope_base_oid=source_scope,impact_projection=str(projection_path),
                run_id=plan['workflow_run_id'],run_attempt=plan['run_attempt'],
                check_app_id=plan['check_app_id'],check_run_id=plan['check_run_id'],
                trusted_source_product_environment=source_product_environment,
            )
        except (OSError,ValueError,KeyError,TypeError) as exc:
            raise ValueError('current Q complete required-unit inventory could not be built') from exc
        if (target_inventory.get('planner_authority_oid')!=target_oid
                or target_inventory.get('planner_config_sha256')!=authority.get('planner_config_sha256')
                or target_inventory.get('planner_output') is None
                or target_inventory.get('closure_status') not in ('complete','unknown')):
            raise ValueError('current Q inventory is not bound to the live W authority')
        local_invocation=target_inventory.get('planner_invocation')
        if not isinstance(local_invocation,dict) or 'producer' not in local_invocation:
            raise ValueError('current Q planner replay lacks its internally validated invocation')
        local_invocation={key:value for key,value in local_invocation.items() if key!='producer'}
        scope_module=_load_checkout_module(
            planner_root/'scripts/pm/ci_input_scope.py','trusted_local_target_input_scope',
        )
        issuer=target_inventory.get('planner_inventory_issuer')
        if not isinstance(issuer,dict):
            raise ValueError('current Q inventory digest is unavailable')
        if (issuer.get('target_oid')!=worktrees['input_scope_commit_oid']
                or issuer.get('target_tree_oid')!=worktrees['input_scope_tree_oid']):
            raise ValueError('current Q inventory target differs from exact M/T')
        try:
            observation=scope_module.build_target_observation(
                authority=authority,planner_invocation=local_invocation,
                repository=repository,task_uid=task_uid,pr_number=pr_number,
                source_head_oid=head,source_scope_oid=source_scope,
                assessed_target_oid=target_oid,
                input_scope_commit_oid=worktrees['input_scope_commit_oid'],
                input_scope_tree_oid=worktrees['input_scope_tree_oid'],
                effective_policy_identity=target_policy['effective_policy_identity'],
                unit_specs=target_inventory['unit_specs'],
                product_corpus=target_inventory['product_corpus'],
            )
            unknown=target_inventory['closure_status']=='unknown'
            target_scope=scope_module.build_input_scope_snapshot(
                str(target_root),worktrees['input_scope_commit_oid'],
                target_inventory['unit_specs'],target_inventory['product_corpus'],
                target_observation=observation,
                closure_status='unknown' if unknown else 'complete',
                closure_reason=target_inventory['closure_reason'] if unknown else None,
                fallback_unit_ids=[item['unit_id'] for item in target_inventory['unit_specs']] if unknown else None,
                fallback_scope_complete=unknown,
            )
        except (OSError,ValueError,KeyError,TypeError) as exc:
            raise ValueError('current Q local target observation or input scope is invalid') from exc

        # Freshly bind the assessment window after the relatively expensive W
        # replay. A changed ref, PR head, or target invalidates this result.
        fresh_pr,fresh_branch=identity(repository,task_uid,pr_number,base,head,allow_base_advance=True)
        selected_after=current_request(
            repository,task_uid,pr_number,base,head,branch,request_key=source_proof['request_key'],
        )
        if (fresh_branch!=branch or fresh_pr.get('base',{}).get('sha')!=target_oid
                or default_branch_head(repository,branch)!=target_oid
                or selected_after!=selected):
            raise ValueError('PR or default branch moved during local target observation')
        _fresh_projection_raw,fresh_projection=_projection_from_pr_body(fresh_pr.get('body'))
        if fresh_projection!=projection_value:
            raise ValueError('PR impact projection changed during local target observation')
        return {
            'schema':LOCAL_TARGET_INVENTORY_SCHEMA,
            'repository':repository,'task_uid':task_uid,'pr_number':pr_number,
            'integration_base_oid':base,'source_head_oid':head,'source_scope_oid':source_scope,
            'assessed_target_oid':target_oid,
            'input_scope_commit_oid':worktrees['input_scope_commit_oid'],
            'input_scope_tree_oid':worktrees['input_scope_tree_oid'],
            'planner_authority_oid':target_oid,
            'planner_config_sha256':authority['planner_config_sha256'],
            'effective_policy':target_policy['effective_policy'],
            'effective_policy_identity':target_policy['effective_policy_identity'],
            'target_observation':observation,'input_scope':target_scope,
            'required_test_units':target_scope['required_test_units'],
            'unit_specs':target_inventory['unit_specs'],
            'product_corpus':target_inventory['product_corpus'],
            'closure_status':target_inventory['closure_status'],
        }

def _executor_contract(root,approved_digests):
    # Imported lazily so old isolated workflow bundles remain compatible. The
    # B/W path is unavailable until C4 ships this helper with its trusted policy.
    try:
        helper=_adjacent_module('integration_executor_contract')
    except ImportError as exc:
        raise ValueError('EXECUTOR_CONTRACT_CHANGED: trusted contract helper unavailable') from exc
    contract=helper.build_executor_contract(root)
    digest=helper.require_approved_executor_contract(contract,approved_digests)
    return contract,digest

def decode_validation_request(encoded,request_key):
    """Decode only the exact canonical v2 request body sent to the runner."""
    helper=_adjacent_module('integration_executor_contract')
    if not isinstance(encoded,str) or not encoded:
        raise ValueError('validation request payload is required')
    try:
        raw=base64.b64decode(encoded,validate=True)
        value=json.loads(raw.decode('utf-8'))
    except (UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
        raise ValueError('validation request payload is malformed') from exc
    if helper.canonical_bytes(value)!=raw:
        raise ValueError('validation request payload is not canonical JSON')
    if base64.b64encode(raw).decode('ascii')!=encoded:
        raise ValueError('validation request payload is not canonical base64')
    envelope=helper.validation_request_envelope(value)
    if envelope['request_key']!=request_key:
        raise ValueError('validation request key does not match payload')
    return envelope

def decode_effective_policy(encoded):
    """Decode canonical policy bytes; callers must source them from trusted W."""
    helper=_adjacent_module('integration_executor_contract')
    if not isinstance(encoded,str) or not encoded:
        raise ValueError('effective executor policy payload is required')
    try:
        raw=base64.b64decode(encoded,validate=True)
        value=json.loads(raw.decode('utf-8'))
    except (UnicodeDecodeError,json.JSONDecodeError,ValueError) as exc:
        raise ValueError('effective executor policy payload is malformed') from exc
    if helper.canonical_bytes(value)!=raw:
        raise ValueError('effective executor policy payload is not canonical JSON')
    helper.effective_policy_digest(value)
    return value

def prepare(root,repository,uid,number,base,head,*,approved_executor_contract_digests=None,integration_worktree=None,request_key=None,validation_request_b64=None,effective_policy=None,trusted_policy=None):
    if type(number) is not int or number<1:
        raise ValueError('positive integer pull request number required')
    workflow_sha=os.environ.get('GITHUB_WORKFLOW_SHA','')
    execution_sha=os.environ.get('GITHUB_SHA','')
    if not OID.fullmatch(workflow_sha) or not OID.fullmatch(execution_sha):
        raise ValueError('integration workflow/run identity is invalid')
    keyed_inputs=request_key is not None or validation_request_b64 is not None
    _,branch=identity(repository,uid,number,base,head,allow_base_advance=keyed_inputs)
    validation_request=None
    if keyed_inputs or effective_policy is not None or trusted_policy is not None:
        if request_key is None or validation_request_b64 is None or trusted_policy is None:
            raise ValueError('validation request identity is incomplete')
        helper=_adjacent_module('integration_executor_contract')
        identity_helper=_adjacent_module('ci_ready_receipt_identity')
        if (not isinstance(trusted_policy,dict)
                or trusted_policy.get('schema')!='oasis7-trusted-ci-reuse-policy-context/v1'
                or trusted_policy.get('repository')!=repository
                or trusted_policy.get('workflow_ref')!=f'{repository}/{WORKFLOW}@refs/heads/{branch}'
                or trusted_policy.get('workflow_sha')!=workflow_sha
                or trusted_policy.get('effective_policy')!=effective_policy):
            raise ValueError('effective policy is not bound to trusted workflow W')
        policy_digest=helper.effective_policy_digest(effective_policy)
        policy_identity=trusted_policy.get('effective_policy_identity')
        if (not isinstance(policy_identity,dict)
                or policy_identity.get('schema')!=helper.EFFECTIVE_POLICY_IDENTITY_SCHEMA
                or policy_identity.get('digest')!=policy_digest):
            raise ValueError('trusted effective policy identity mismatch')
        if identity_helper.INPUT_SCOPE_REUSE_CAPABILITY not in effective_policy['enabled_capabilities']:
            raise ValueError(f'{identity_helper.INPUT_SCOPE_REUSE_CAPABILITY} is disabled')
        policy_approved=effective_policy['approved_executor_contract_digests']
        if not policy_approved:
            raise ValueError('effective executor contract policy has no approved contract')
        if (approved_executor_contract_digests is not None
                and approved_executor_contract_digests!=policy_approved):
            raise ValueError('effective executor contract policy identity mismatch')
        approved_executor_contract_digests=policy_approved
        validation_request=decode_validation_request(validation_request_b64,request_key)
        request_identity=validation_request['identity']
        if (validation_request['integration_base_oid']!=base
                or request_identity['repository']!=repository
                or request_identity['task_uid']!=uid
                or request_identity['pr_number']!=int(number)
                or request_identity['source_head_oid']!=head
                or request_identity['effective_policy_digest']!=policy_digest):
            raise ValueError('validation request does not match trusted execution identity')
    keyed_mode=workflow_sha!=base or approved_executor_contract_digests is not None
    if os.environ.get('GITHUB_EVENT_NAME')!='workflow_dispatch' or os.environ.get('GITHUB_REF')!=f'refs/heads/{branch}':
        raise ValueError('integration workflow must execute canonical default-branch authority')
    # GitHub executes the workflow version present at the workflow_dispatch
    # event's commit/ref. E is independently visible as the Actions run head,
    # so keyed mode accepts W only when the runner's W agrees with that E.
    if keyed_mode and workflow_sha!=execution_sha:
        raise ValueError('trusted workflow SHA must match the workflow-dispatch run head')
    if not keyed_mode and (workflow_sha!=base or execution_sha!=base):
        raise ValueError('integration workflow must execute immutable current default-branch authority')
    if git(root,'rev-parse','HEAD')!=workflow_sha: raise ValueError('runner checkout differs from trusted workflow identity')
    executor_digest=None
    run_attempt=None
    if keyed_mode:
        if not approved_executor_contract_digests:
            raise ValueError('EXECUTOR_CONTRACT_CHANGED: no approved executor contract')
        _,executor_digest=_executor_contract(root,approved_executor_contract_digests)
        if (validation_request is not None
                and validation_request['identity']['executor_contract_digest']!=executor_digest):
            raise ValueError('validation request executor contract identity mismatch')
        attempt_value=os.environ.get('GITHUB_RUN_ATTEMPT','')
        if not attempt_value.isdigit() or int(attempt_value)<1:
            raise ValueError('integration workflow attempt identity is invalid')
        run_attempt=int(attempt_value)
    if keyed_mode and not integration_worktree:
        raise ValueError('executor-contract integration requires an independent integration worktree')
    git(root,'fetch','--no-tags','--no-write-fetch-head','origin',base,head)
    try:
        git(root,'merge-base','--is-ancestor',base,execution_sha)
    except subprocess.CalledProcessError as exc:
        raise ValueError('frozen integration base is not an ancestor of workflow run target') from exc
    result=compose(root,base,head,worktree_path=integration_worktree if integration_worktree else None)
    result.update(task_uid=uid,pr_number=int(number),workflow_sha=workflow_sha,workflow_run_head_sha=execution_sha,workflow_ref=f'{repository}/{WORKFLOW}@refs/heads/{branch}',integration_mode='integration_revalidation')
    if executor_digest is not None: result['executor_contract_digest']=executor_digest
    if run_attempt is not None: result['workflow_run_attempt']=run_attempt
    if validation_request is not None: result['validation_request']=validation_request
    return result

def dispatch(repository,uid,number,impact_projection):
    if type(number) is not int or number<1:
        raise ValueError('positive integer pull request number required')
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    head=pr['head']['sha']
    repo=gh('api',f'repos/{repository}')
    branch=repo['default_branch']
    base=default_branch_head(repository,branch)
    _,branch=identity(repository,uid,number,base,head,allow_base_advance=True)
    source=gh('api',f'repos/{repository}/contents/{WORKFLOW}?ref={base}')
    workflow=base64.b64decode(source['content']).decode()
    if 'integration_revalidation' not in workflow:
        raise ValueError('activation pending: default-branch workflow lacks integration_revalidation; candidate workflow cannot authorize itself')
    projection_path=Path(impact_projection or '')
    if not projection_path.is_file():
        raise ValueError('integration dispatch requires a readable impact projection')
    projection_b64=base64.b64encode(projection_path.read_bytes()).decode()
    subprocess.run(['gh','workflow','run','rust.yml','--repo',repository,'--ref',branch,'-f','run_mode=integration_revalidation','-f',f'task_uid={uid}','-f',f'pr_number={number}','-f',f'expected_head={head}','-f',f'integration_base={base}','-f',f'impact_projection_b64={projection_b64}'],check=True)
    return {'status':'requested','base_oid':base,'head_oid':head,'next_command':f'gh run list --repo {repository} --workflow rust.yml --event workflow_dispatch'}

def dispatch_request(repository,uid,number,impact_projection,request_identity,effective_policy,*,state_dir=None):
    """Proposed idempotent request adapter; no current lifecycle caller enables it."""
    if type(number) is not int or number<1:
        raise ValueError('positive integer pull request number required')
    helper=_adjacent_module('integration_executor_contract')
    identity_helper=_adjacent_module('ci_ready_receipt_identity')
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    head=pr.get('head',{}).get('sha','')
    repo=gh('api',f'repos/{repository}')
    branch=repo.get('default_branch')
    if not isinstance(branch,str) or not branch: raise ValueError('default branch identity unavailable')
    base=default_branch_head(repository,branch)
    trusted_policy=trusted_policy_context(repository,branch,base,base)
    if trusted_policy.get('effective_policy')!=effective_policy:
        raise ValueError('caller effective policy differs from trusted workflow W')
    policy=trusted_policy['effective_policy']
    policy_digest=helper.effective_policy_digest(policy)
    enabled=policy['enabled_capabilities']
    if identity_helper.INPUT_SCOPE_REUSE_CAPABILITY not in enabled:
        raise ValueError(f'{identity_helper.INPUT_SCOPE_REUSE_CAPABILITY} is disabled')
    approved=policy['approved_executor_contract_digests']
    if not approved:
        raise ValueError('effective executor contract policy has no approved contract')
    identity_value=helper.validation_request_identity(request_identity)
    if (identity_value['repository']!=repository or identity_value['task_uid']!=uid
            or identity_value['pr_number']!=int(number)):
        raise ValueError('validation request task/PR identity mismatch')
    if identity_value['effective_policy_digest']!=policy_digest:
        raise ValueError('validation request effective policy identity mismatch')
    if head!=identity_value['source_head_oid']:
        raise ValueError('validation request source head is stale')
    projection_path=Path(impact_projection or '')
    if not projection_path.is_file(): raise ValueError('integration dispatch requires a readable impact projection')
    projection_raw=projection_path.read_bytes()
    try: projection=json.loads(projection_raw)
    except (UnicodeDecodeError,json.JSONDecodeError) as exc: raise ValueError('impact projection is malformed') from exc
    if identity_value['source_projection_digest']!=projection.get('projection_digest'):
        raise ValueError('validation request projection identity mismatch')
    _,branch=identity(repository,uid,number,base,head,allow_base_advance=True)
    executor_contract=github_executor_contract(repository,base)
    executor_digest=helper.require_approved_executor_contract(executor_contract,approved)
    if identity_value['executor_contract_digest']!=executor_digest:
        raise ValueError('EXECUTOR_CONTRACT_CHANGED')
    key=helper.validation_request_key(identity_value)
    workflow_source=gh('api',f'repos/{repository}/contents/{WORKFLOW}?ref={base}')
    if workflow_source.get('type')!='file' or workflow_source.get('encoding')!='base64':
        raise ValueError('trusted default-branch workflow readback unavailable')
    try:
        workflow=base64.b64decode(''.join(str(workflow_source['content']).split()),validate=True).decode('utf-8')
    except (KeyError,UnicodeDecodeError,ValueError) as exc:
        raise ValueError('trusted default-branch workflow readback malformed') from exc
    if not keyed_request_workflow_ready(workflow):
        raise ValueError('activation pending: default workflow lacks idempotent request identity inputs')

    directory=Path(state_dir) if state_dir is not None else git_common_dir('.')

    def readback(request_key,frozen_base):
        selected=current_request(repository,uid,number,frozen_base,head,branch,request_key=request_key)
        if selected is None: return None
        return {'request_key':request_key,'integration_base_oid':frozen_base,
                'run_id':selected['id'],'run_attempt':selected['run_attempt']}

    def send(record):
        frozen_base=record['integration_base_oid']
        request=helper.validation_request_envelope({
            'schema':helper.VALIDATION_REQUEST_SCHEMA,
            'request_key':key,
            'identity':record['identity'],
            'integration_base_oid':frozen_base,
        })
        request_b64=base64.b64encode(helper.canonical_bytes(request)).decode('ascii')
        # Re-read the exact source/target binding before every remote side effect.
        identity(repository,uid,number,frozen_base,head,allow_base_advance=True)
        subprocess.run([
            'gh','workflow','run','rust.yml','--repo',repository,'--ref',branch,
            '-f','run_mode=integration_revalidation','-f',f'task_uid={uid}',
            '-f',f'pr_number={number}','-f',f'expected_head={head}',
            '-f',f'integration_base={frozen_base}',
            '-f',f'impact_projection_b64={base64.b64encode(projection_raw).decode("ascii")}',
            '-f',f'request_key={key}','-f',f'validation_request_b64={request_b64}',
        ],check=True)

    record,disposition=helper.ensure_validation_request(
        directory,key,identity_value,base,readback=readback,dispatch=send,
    )
    return {'status':disposition,'request_key':key,
            'base_oid':record['integration_base_oid'],'head_oid':head,
            'run_id':record.get('run_id'),'run_attempt':record.get('run_attempt')}

def verified_run(repository,uid,number,base,head,run_id,app_id,*,request_key=None,expected_attempt=None,request_identity=None,effective_policy=None,approved_executor_contract_digests=None):
    """Verify live CI provenance; keyed callers must supply trusted policy and request identity."""
    if type(number) is not int or number<1:
        raise ValueError('positive integer pull request number required')
    if not str(app_id or '').isdigit() or not str(run_id or '').isdigit():
        raise ValueError('numeric non-null app and workflow run identity required')
    if expected_attempt is not None and (type(expected_attempt) is not int or expected_attempt<1):
        raise ValueError('manual integration expected attempt is invalid')
    helper=None
    expected_request=None
    if request_key is not None:
        if not re.fullmatch(r'sha256:[0-9a-f]{64}',request_key):
            raise ValueError('manual integration request key is invalid')
        if type(expected_attempt) is not int or expected_attempt<1:
            raise ValueError('manual integration expected attempt is required')
        try:
            helper=_adjacent_module('integration_executor_contract')
        except ImportError as exc:
            raise ValueError('EXECUTOR_CONTRACT_CHANGED: trusted contract helper unavailable') from exc
        identity_helper=_adjacent_module('ci_ready_receipt_identity')
        identity_value=helper.validation_request_identity(request_identity)
        policy_digest=helper.effective_policy_digest(effective_policy)
        if (identity_value['repository']!=repository or identity_value['task_uid']!=uid
                or identity_value['pr_number']!=int(number)
                or identity_value['source_head_oid']!=head
                or identity_value['effective_policy_digest']!=policy_digest):
            raise ValueError('manual integration request identity mismatch')
        if helper.validation_request_key(identity_value)!=request_key:
            raise ValueError('manual integration request key does not match its identity')
        if str(effective_policy['check_app_id'])!=str(app_id):
            raise ValueError('manual integration check app differs from effective policy')
        if identity_helper.INPUT_SCOPE_REUSE_CAPABILITY not in effective_policy['enabled_capabilities']:
            raise ValueError(f'{identity_helper.INPUT_SCOPE_REUSE_CAPABILITY} is disabled')
        approved=effective_policy['approved_executor_contract_digests']
        if not approved:
            raise ValueError('effective executor contract policy has no approved contract')
        if approved_executor_contract_digests is not None and approved_executor_contract_digests!=approved:
            raise ValueError('effective executor contract policy identity mismatch')
        approved_executor_contract_digests=approved
        expected_request=helper.validation_request_envelope({
            'schema':helper.VALIDATION_REQUEST_SCHEMA,
            'request_key':request_key,
            'identity':identity_value,
            'integration_base_oid':base,
        })
    elif request_identity is not None or effective_policy is not None:
        raise ValueError('manual integration keyed request identity is incomplete')
    _,branch=identity(repository,uid,number,base,head,allow_base_advance=request_key is not None)
    run=gh('api',f'repos/{repository}/actions/runs/{run_id}')
    expected={'event':'workflow_dispatch','head_branch':branch,'path':WORKFLOW,'status':'completed','conclusion':'success'}
    if request_key is None: expected['head_sha']=base
    if any(run.get(k)!=v for k,v in expected.items()) or run.get('repository',{}).get('full_name')!=repository:
        raise ValueError('manual integration run provenance/status mismatch')
    if expected_attempt is not None and (type(run.get('run_attempt')) is not int
                                         or run['run_attempt']!=expected_attempt):
        raise ValueError('manual integration workflow attempt mismatch')
    policy_context=None
    if request_key is not None:
        policy_context=trusted_policy_context_for_run(repository,branch,run)
        if policy_context.get('effective_policy')!=effective_policy:
            raise ValueError('caller effective policy differs from trusted workflow W')
        if policy_context.get('effective_policy_identity',{}).get('digest')!=helper.effective_policy_digest(effective_policy):
            raise ValueError('trusted effective policy identity mismatch')
    expected_title=f'oasis7-ci|workflow_dispatch|integration_revalidation|{uid}|{number}|{base}|{head}|{request_key}'
    if request_key is not None and run.get('display_title')!=expected_title:
        raise ValueError('manual integration workflow request title mismatch')
    checks=pages(repository,f"check-suites/{run['check_suite_id']}/check-runs",'check_runs')
    selected=[c for c in checks if c.get('name')=='required-gate' and str(c.get('app',{}).get('id'))==str(app_id) and c.get('conclusion')=='success']
    if len(selected)!=1: raise ValueError('manual required-gate app/run identity missing')
    check=selected[0]
    if check.get('head_sha')!=(run.get('head_sha') if request_key is not None else base) or check.get('status')!='completed' or not re.match(re.escape(f'https://github.com/{repository}/actions/runs/{run_id}')+r'(?:/|$)',check.get('details_url','')):
        raise ValueError('manual check belongs to a different workflow run/head')
    artifacts=pages(repository,f'actions/runs/{run_id}/artifacts','artifacts')
    found=[a for a in artifacts if a.get('name')==ARTIFACT and not a.get('expired')]
    if len(found)!=1 or found[0].get('workflow_run',{}).get('id')!=int(run_id): raise ValueError('manual integration artifact missing or ambiguous')
    raw=subprocess.check_output(['gh','api',f"repos/{repository}/actions/artifacts/{found[0]['id']}/zip"])
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        if archive.namelist()!=[ARTIFACT+'.json']: raise ValueError('manual integration artifact members mismatch')
        payload=json.loads(archive.read(ARTIFACT+'.json'))
    execution_sha=run.get('head_sha') if request_key is not None else base
    # Never choose the executor source revision from the downloaded artifact.
    # In keyed workflow_dispatch mode, the API run head is E and W must equal E.
    workflow_sha=execution_sha
    expected={'schema':ARTIFACT,'repository':repository,'workflow_run_id':int(run_id),'base_oid':base,'head_oid':head,'task_uid':uid,'pr_number':int(number),'workflow_sha':workflow_sha,'workflow_ref':f'{repository}/{WORKFLOW}@refs/heads/{branch}','integration_mode':'integration_revalidation','check_name':'required-gate'}
    if request_key is not None:
        expected['request_key']=request_key
        expected['workflow_run_head_sha']=execution_sha
        expected['workflow_run_attempt']=expected_attempt
        expected['validation_request']=expected_request
        if not approved_executor_contract_digests:
            raise ValueError('effective executor contract policy is missing or invalid')
        if not OID.fullmatch(str(workflow_sha or '')) or not OID.fullmatch(str(execution_sha or '')):
            raise ValueError('manual integration workflow identity is invalid')
        actual=gh('api',f'repos/{repository}/compare/{base}...{execution_sha}')
        if actual.get('merge_base_commit',{}).get('sha')!=base:
            raise ValueError('frozen integration base is not an ancestor of workflow run target')
        contents={}
        for path in helper.EXECUTOR_CONTRACT_PATHS:
            source=gh('api',f'repos/{repository}/contents/{path}?ref={workflow_sha}')
            if source.get('type')!='file' or source.get('path')!=path or source.get('encoding')!='base64':
                raise ValueError('trusted executor contract source is unavailable')
            try: contents[path]=base64.b64decode(''.join(str(source['content']).split()),validate=True)
            except (KeyError,ValueError) as exc: raise ValueError('trusted executor contract source is malformed') from exc
        executor_contract=helper.executor_contract_from_contents(contents)
        executor_digest=helper.require_approved_executor_contract(executor_contract,approved_executor_contract_digests)
        expected['executor_contract_digest']=executor_digest
    if any(payload.get(k)!=v for k,v in expected.items()) or not all(OID.fullmatch(str(payload.get(k,''))) for k in ('scope_base_oid','tested_tree_oid','tested_commit_oid')):
        raise ValueError('manual integration artifact authority mismatch')
    if policy_context is not None:
        payload=dict(payload)
        payload.update({
            'trusted_policy_context':policy_context,
            'effective_policy_identity':policy_context['effective_policy_identity'],
            'planner_inventory_authority':policy_context['planner_inventory_authority'],
            'workflow_run_id':int(run_id),
            'run_attempt':run['run_attempt'],
            'check_app_id':int(check['app']['id']),
            'check_run_id':int(check['id']),
            'plan_artifact_id':int(found[0]['id']),
        })
        execution_jobs=attempt_execution_jobs(
            repository,int(run_id),expected_attempt,run.get('head_sha'),int(app_id),
            require_completed=True,
        )
        gate_jobs=[job for job in execution_jobs if job['job_name']=='required-gate']
        if (len(gate_jobs)!=1 or gate_jobs[0]['check_run_id']!=check.get('id')
                or gate_jobs[0]['conclusion']!='success'):
            raise ValueError('manual required-gate check is not the exact successful attempt job')
        payload.update(read_keyed_v2_evidence(
            repository,run,int(run_id),expected_attempt,int(app_id),check,
            request_key=request_key,request_identity=identity_value,
            base=base,head=head,policy_context=policy_context,
            approved_executor_contract_digests=approved_executor_contract_digests,
            execution_jobs=execution_jobs,
        ))
    return selected[0],payload

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command',choices=['dispatch','prepare','attempt-jobs'])
    parser.add_argument('--repository',required=True);parser.add_argument('--task-uid',required=True);parser.add_argument('--pr-number',required=True,type=int)
    parser.add_argument('--base');parser.add_argument('--head');parser.add_argument('--root',default='.');parser.add_argument('--output')
    parser.add_argument('--impact-projection')
    parser.add_argument('--approved-executor-contract-digest',action='append')
    parser.add_argument('--integration-worktree')
    parser.add_argument('--request-key')
    parser.add_argument('--validation-request-b64')
    parser.add_argument('--effective-policy-b64')
    parser.add_argument('--trusted-policy-json')
    parser.add_argument('--run-id',type=int)
    parser.add_argument('--run-attempt',type=int)
    parser.add_argument('--workflow-sha')
    parser.add_argument('--check-app-id',type=int)
    parser.add_argument('--require-completed',action='store_true')
    parser.add_argument('--job-name',action='append')
    a=parser.parse_args()
    try:
        if a.command=='dispatch':
            result=dispatch(a.repository,a.task_uid,a.pr_number,a.impact_projection)
        elif a.command=='attempt-jobs':
            if a.run_id is None or a.run_attempt is None or a.workflow_sha is None or a.check_app_id is None:
                raise ValueError('attempt-jobs requires exact run, attempt, workflow SHA, and check app')
            result={'execution_jobs':attempt_execution_jobs(
                a.repository,a.run_id,a.run_attempt,a.workflow_sha,a.check_app_id,
                require_completed=a.require_completed,job_names=a.job_name,
            )}
        else:
            trusted_policy=(json.loads(Path(a.trusted_policy_json).read_text(encoding='utf-8'))
                            if a.trusted_policy_json else None)
            policy=(trusted_policy.get('effective_policy') if trusted_policy else
                    decode_effective_policy(a.effective_policy_b64) if a.effective_policy_b64 else None)
            result=prepare(
                Path(a.root),a.repository,a.task_uid,a.pr_number,a.base,a.head,
                approved_executor_contract_digests=a.approved_executor_contract_digest,
                integration_worktree=a.integration_worktree,
                request_key=a.request_key,
                validation_request_b64=a.validation_request_b64,
                effective_policy=policy,
                trusted_policy=trusted_policy,
            )
        if a.output: Path(a.output).write_text(json.dumps(result))
        print(json.dumps(result))
    except (ValueError,KeyError,OSError,subprocess.SubprocessError) as exc:
        print(json.dumps({'status':'blocked','blockers':[str(exc)]}));return 2
    return 0

if __name__=='__main__':raise SystemExit(main())
