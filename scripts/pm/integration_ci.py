#!/usr/bin/env python3
"""Explicit manual revalidation on the default-branch GitHub Actions workflow."""
import argparse
import base64
import datetime
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
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
                try: timestamp=datetime.datetime.fromisoformat(when.replace('Z','+00:00')).timestamp()
                except ValueError as exc: raise ValueError('integration request time malformed') from exc
                matches.append({'id':run_id,'run_attempt':attempt,'requested_at':timestamp,
                                'execution_sha':run['head_sha'],'execution_branch':run['head_branch']})
        if len(batch)<DISCOVERY_PAGE_SIZE:
            if not matches: return None
            selected=max(matches,key=lambda item:(item['requested_at'],item['id'],item['run_attempt']))
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
