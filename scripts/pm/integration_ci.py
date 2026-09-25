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
import zipfile

WORKFLOW='.github/workflows/rust.yml'
ARTIFACT='oasis7-required-plan-v1'
OID=re.compile(r'[0-9a-f]{40}')
DISCOVERY_PAGE_SIZE=100
DISCOVERY_MAX_PAGES=10
KEYED_RUN_NAME='oasis7-ci|${{ github.event_name }}|${{ inputs.run_mode }}|${{ inputs.task_uid }}|${{ inputs.pr_number }}|${{ inputs.integration_base }}|${{ inputs.expected_head }}|${{ inputs.request_key }}'

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
    required_inputs={key for key in (
        'run_mode','task_uid','pr_number','integration_base','expected_head',
        'request_key','validation_request_b64',
    )
                     if len([item for item in input_fields if item['key']==key])==1
                     and next(item for item in input_fields if item['key']==key)['value']==''}
    run_name=run_names[0]['value']
    return (set(('run_mode','task_uid','pr_number','integration_base','expected_head',
                 'request_key','validation_request_b64'))<=required_inputs
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

def prepare(root,repository,uid,number,base,head,*,approved_executor_contract_digests=None,integration_worktree=None,request_key=None,validation_request_b64=None,effective_policy=None):
    if type(number) is not int or number<1:
        raise ValueError('positive integer pull request number required')
    workflow_sha=os.environ.get('GITHUB_WORKFLOW_SHA','')
    execution_sha=os.environ.get('GITHUB_SHA','')
    if not OID.fullmatch(workflow_sha) or not OID.fullmatch(execution_sha):
        raise ValueError('integration workflow/run identity is invalid')
    validation_request=None
    if request_key is not None or validation_request_b64 is not None or effective_policy is not None:
        if request_key is None or validation_request_b64 is None or effective_policy is None:
            raise ValueError('validation request identity is incomplete')
        helper=_adjacent_module('integration_executor_contract')
        identity_helper=_adjacent_module('ci_ready_receipt_identity')
        policy_digest=helper.effective_policy_digest(effective_policy)
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
    _,branch=identity(repository,uid,number,base,head,allow_base_advance=keyed_mode)
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

    policy_digest=helper.effective_policy_digest(effective_policy)
    enabled=effective_policy['enabled_capabilities']
    if identity_helper.INPUT_SCOPE_REUSE_CAPABILITY not in enabled:
        raise ValueError(f'{identity_helper.INPUT_SCOPE_REUSE_CAPABILITY} is disabled')
    approved=effective_policy['approved_executor_contract_digests']
    if not approved:
        raise ValueError('effective executor contract policy has no approved contract')
    identity_value=helper.validation_request_identity(request_identity)
    if (identity_value['repository']!=repository or identity_value['task_uid']!=uid
            or identity_value['pr_number']!=int(number)):
        raise ValueError('validation request task/PR identity mismatch')
    if identity_value['effective_policy_digest']!=policy_digest:
        raise ValueError('validation request effective policy identity mismatch')
    projection_path=Path(impact_projection or '')
    if not projection_path.is_file(): raise ValueError('integration dispatch requires a readable impact projection')
    projection_raw=projection_path.read_bytes()
    try: projection=json.loads(projection_raw)
    except (UnicodeDecodeError,json.JSONDecodeError) as exc: raise ValueError('impact projection is malformed') from exc
    if identity_value['source_projection_digest']!=projection.get('projection_digest'):
        raise ValueError('validation request projection identity mismatch')

    pr=gh('api',f'repos/{repository}/pulls/{number}')
    head=pr.get('head',{}).get('sha','')
    if head!=identity_value['source_head_oid']:
        raise ValueError('validation request source head is stale')
    repo=gh('api',f'repos/{repository}')
    branch=repo.get('default_branch')
    if not isinstance(branch,str) or not branch: raise ValueError('default branch identity unavailable')
    base=default_branch_head(repository,branch)
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
    if request_key is not None and (type(run.get('run_attempt')) is not int
                                    or run['run_attempt']!=expected_attempt):
        raise ValueError('manual integration workflow attempt mismatch')
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
    return selected[0],payload

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command',choices=['dispatch','prepare'])
    parser.add_argument('--repository',required=True);parser.add_argument('--task-uid',required=True);parser.add_argument('--pr-number',required=True,type=int)
    parser.add_argument('--base');parser.add_argument('--head');parser.add_argument('--root',default='.');parser.add_argument('--output')
    parser.add_argument('--impact-projection')
    parser.add_argument('--approved-executor-contract-digest',action='append')
    parser.add_argument('--integration-worktree')
    parser.add_argument('--request-key')
    parser.add_argument('--validation-request-b64')
    parser.add_argument('--effective-policy-b64')
    a=parser.parse_args()
    try:
        if a.command=='dispatch':
            result=dispatch(a.repository,a.task_uid,a.pr_number,a.impact_projection)
        else:
            policy=decode_effective_policy(a.effective_policy_b64) if a.effective_policy_b64 else None
            result=prepare(
                Path(a.root),a.repository,a.task_uid,a.pr_number,a.base,a.head,
                approved_executor_contract_digests=a.approved_executor_contract_digest,
                integration_worktree=a.integration_worktree,
                request_key=a.request_key,
                validation_request_b64=a.validation_request_b64,
                effective_policy=policy,
            )
        if a.output: Path(a.output).write_text(json.dumps(result))
        print(json.dumps(result))
    except (ValueError,KeyError,OSError,subprocess.SubprocessError) as exc:
        print(json.dumps({'status':'blocked','blockers':[str(exc)]}));return 2
    return 0

if __name__=='__main__':raise SystemExit(main())
