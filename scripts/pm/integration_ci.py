#!/usr/bin/env python3
"""Explicit manual revalidation on the default-branch GitHub Actions workflow."""
import argparse
import base64
import datetime
import io
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile

WORKFLOW='.github/workflows/rust.yml'
ARTIFACT='oasis7-required-plan-v1'
OID=re.compile(r'[0-9a-f]{40}')
DISCOVERY_PAGE_SIZE=100
DISCOVERY_MAX_PAGES=10

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

def current_request(repository,uid,number,base,head,branch):
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
            if len(parts)!=7 or parts[:2]!=['oasis7-ci','workflow_dispatch']:
                if run['head_sha'] not in legacy_cannot_integrate:
                    source=gh('api',f"repos/{repository}/contents/{WORKFLOW}?ref={run['head_sha']}")
                    if source.get('type')!='file' or source.get('path')!=WORKFLOW or source.get('encoding')!='base64':
                        raise ValueError('integration effective workflow readback unavailable')
                    legacy_cannot_integrate[run['head_sha']]='integration_revalidation' not in base64.b64decode(source['content'],validate=False).decode()
                if legacy_cannot_integrate[run['head_sha']]: continue
                raise ValueError('integration current request identity unavailable before outcome')
            _,_,mode,request_uid,request_pr,request_base,request_head=parts
            if mode in ('full_escalation','newapi_bridge_package'): continue
            if mode!='integration_revalidation' or not re.fullmatch(r'task_[0-9a-f]{32}',request_uid) or not request_pr.isdigit() or not OID.fullmatch(request_base) or not OID.fullmatch(request_head):
                raise ValueError('integration current request identity malformed')
            if request_base!=base or request_head!=head: continue
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
            if selected.pop('execution_sha')!=base or selected.pop('execution_branch')!=branch:
                raise ValueError('integration request base differs from trusted workflow ref')
            return selected
    raise ValueError('integration discovery range exhausted; current request coverage incomplete')

def identity(repository,uid,number,base,head):
    if not OID.fullmatch(base) or not OID.fullmatch(head): raise ValueError('exact base/source OIDs required')
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    if pr.get('state')!='open' or pr.get('merged') or pr['base']['sha']!=base or pr['head']['sha']!=head:
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

def compose(root,base,head):
    for oid in (base,head):
        if not OID.fullmatch(oid): raise ValueError('exact integration OIDs required')
    scope=git(root,'merge-base','--all',base,head).splitlines()
    if len(scope)!=1: raise ValueError('ambiguous scope ancestor')
    tree=git(root,'merge-tree','--write-tree',base,head).splitlines()[0]
    env={**os.environ,'GIT_AUTHOR_NAME':'Integration CI','GIT_AUTHOR_EMAIL':'ci@example.invalid','GIT_COMMITTER_NAME':'Integration CI','GIT_COMMITTER_EMAIL':'ci@example.invalid','GIT_AUTHOR_DATE':'2000-01-01T00:00:00Z','GIT_COMMITTER_DATE':'2000-01-01T00:00:00Z'}
    commit=subprocess.check_output(['git','-C',str(root),'commit-tree',tree,'-p',base,'-p',head,'-m','Exact integration revalidation'],env=env,text=True).strip()
    git(root,'checkout','--detach',commit)
    return {'base_oid':base,'head_oid':head,'scope_base_oid':scope[0],'tested_tree_oid':tree,'tested_commit_oid':commit}

def prepare(root,repository,uid,number,base,head):
    _,branch=identity(repository,uid,number,base,head)
    if os.environ.get('GITHUB_EVENT_NAME')!='workflow_dispatch' or os.environ.get('GITHUB_REF')!=f'refs/heads/{branch}' or os.environ.get('GITHUB_SHA')!=base or os.environ.get('GITHUB_WORKFLOW_SHA')!=base:
        raise ValueError('integration workflow must execute immutable current default-branch authority')
    if git(root,'rev-parse','HEAD')!=base: raise ValueError('runner initial checkout differs from workflow authority')
    git(root,'fetch','--no-tags','--no-write-fetch-head','origin',base,head)
    result=compose(root,base,head)
    result.update(task_uid=uid,pr_number=int(number),workflow_sha=base,workflow_ref=f'{repository}/{WORKFLOW}@refs/heads/{branch}',integration_mode='integration_revalidation')
    return result

def dispatch(repository,uid,number):
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    base,head=pr['base']['sha'],pr['head']['sha']
    _,branch=identity(repository,uid,number,base,head)
    source=gh('api',f'repos/{repository}/contents/{WORKFLOW}?ref={base}')
    workflow=base64.b64decode(source['content']).decode()
    if 'integration_revalidation' not in workflow:
        raise ValueError('activation pending: default-branch workflow lacks integration_revalidation; candidate workflow cannot authorize itself')
    subprocess.run(['gh','workflow','run','rust.yml','--repo',repository,'--ref',branch,'-f','run_mode=integration_revalidation','-f',f'task_uid={uid}','-f',f'pr_number={number}','-f',f'expected_head={head}','-f',f'integration_base={base}'],check=True)
    return {'status':'requested','base_oid':base,'head_oid':head,'next_command':f'gh run list --repo {repository} --workflow rust.yml --event workflow_dispatch'}

def verified_run(repository,uid,number,base,head,run_id,app_id):
    if not str(app_id or '').isdigit() or not str(run_id or '').isdigit():
        raise ValueError('numeric non-null app and workflow run identity required')
    _,branch=identity(repository,uid,number,base,head)
    run=gh('api',f'repos/{repository}/actions/runs/{run_id}')
    expected={'event':'workflow_dispatch','head_branch':branch,'head_sha':base,'path':WORKFLOW,'status':'completed','conclusion':'success'}
    if any(run.get(k)!=v for k,v in expected.items()) or run.get('repository',{}).get('full_name')!=repository:
        raise ValueError('manual integration run provenance/status mismatch')
    checks=pages(repository,f"check-suites/{run['check_suite_id']}/check-runs",'check_runs')
    selected=[c for c in checks if c.get('name')=='required-gate' and str(c.get('app',{}).get('id'))==str(app_id) and c.get('conclusion')=='success']
    if len(selected)!=1: raise ValueError('manual required-gate app/run identity missing')
    check=selected[0]
    if check.get('head_sha')!=base or check.get('status')!='completed' or not re.match(re.escape(f'https://github.com/{repository}/actions/runs/{run_id}')+r'(?:/|$)',check.get('details_url','')):
        raise ValueError('manual check belongs to a different workflow run/head')
    artifacts=pages(repository,f'actions/runs/{run_id}/artifacts','artifacts')
    found=[a for a in artifacts if a.get('name')==ARTIFACT and not a.get('expired')]
    if len(found)!=1 or found[0].get('workflow_run',{}).get('id')!=int(run_id): raise ValueError('manual integration artifact missing or ambiguous')
    raw=subprocess.check_output(['gh','api',f"repos/{repository}/actions/artifacts/{found[0]['id']}/zip"])
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        if archive.namelist()!=[ARTIFACT+'.json']: raise ValueError('manual integration artifact members mismatch')
        payload=json.loads(archive.read(ARTIFACT+'.json'))
    expected={'schema':ARTIFACT,'repository':repository,'workflow_run_id':int(run_id),'base_oid':base,'head_oid':head,'task_uid':uid,'pr_number':int(number),'workflow_sha':base,'workflow_ref':f'{repository}/{WORKFLOW}@refs/heads/{branch}','integration_mode':'integration_revalidation','check_name':'required-gate'}
    if any(payload.get(k)!=v for k,v in expected.items()) or not all(OID.fullmatch(str(payload.get(k,''))) for k in ('scope_base_oid','tested_tree_oid','tested_commit_oid')):
        raise ValueError('manual integration artifact authority mismatch')
    return selected[0],payload

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command',choices=['dispatch','prepare'])
    parser.add_argument('--repository',required=True);parser.add_argument('--task-uid',required=True);parser.add_argument('--pr-number',required=True,type=int)
    parser.add_argument('--base');parser.add_argument('--head');parser.add_argument('--root',default='.');parser.add_argument('--output')
    a=parser.parse_args()
    try:
        result=dispatch(a.repository,a.task_uid,a.pr_number) if a.command=='dispatch' else prepare(Path(a.root),a.repository,a.task_uid,a.pr_number,a.base,a.head)
        if a.output: Path(a.output).write_text(json.dumps(result))
        print(json.dumps(result))
    except (ValueError,KeyError,OSError,subprocess.SubprocessError) as exc:
        print(json.dumps({'status':'blocked','blockers':[str(exc)]}));return 2
    return 0

if __name__=='__main__':raise SystemExit(main())
