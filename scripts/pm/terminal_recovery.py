"""Source-defined current terminal observations, distinct from native readiness."""
import ast
import base64
import csv
import datetime
import io
import json
import re
import sys
import tempfile
from pathlib import Path

import recovery_observation as obs

TARGET_CONTEXT={'repository_root','source_review_plan_path','source_review_handoff_path',
    'source_review_resolution_path','source_scope_oid','check_app_id'}
PLAN_FIELDS={'schema','repository','workflow_run_id','head_oid','base_oid',
    'integration_base_oid','check_name','planner'}
PROOF_FIELDS={'schema','mode','purpose','repository','task_uid','issue_number','pr_number','pr_url',
    'head_oid','merge_commit_oid','merged_at','default_branch','observed_target_oid','observed_at',
    'source','authorization_comment','prior_readiness','verification'}
COMPLETION_FIELDS={'schema','claim_type','status','exit_code','task_uid','repository','issue_number',
    'pr_number','pr_url','head_oid','merge_commit_oid','observed_target_oid','verified_at','source',
    'recovery_proof_sha256','authorization_comment_id'}
COMMON_FIELDS={'schema','repository','task_uid','head_oid','observed_at','evidence'}
RECORD_FIELDS={
    'source_review':{'plan','handoff','plan_sha256','handoff_sha256'},
    'required_ci':{'default_branch','policy_sha256','checks'},
    'integration':{'base_oid','run_id','run_attempt','app_id','check_run_id','workflow_sha','tested_tree_oid','request_key','proof'},
    'applicability':{'scope_base_oid','target_oid','policy_oid','policy_sha256','projection','projection_digest','requires_strict_integration','related_paths','disposition'},
    'source_equivalence':{'merge_commit_oid','source_base_oid','merge_parent_oid','source_tree_oid','merged_tree_oid','source_patch_sha256','merged_patch_sha256','method'},
    'merge_readback':{'pr_number','pr_url','merge_commit_oid','merged_at','default_branch','target_oid','comparison_base_oid','comparison_head_oid','comparison_merge_base_oid'},
    'current_holds':{'issue_number','pr_number','project_id','item_id','task_status','workflow_phase','hold_comment_ids','changes_requested_review_ids','unresolved_thread_ids','operator_login','operator_permission'},
    'current_target_ci':{'target_oid','target_tree_oid','default_branch','workflow_path','workflow_sha','event','plan','plan_sha256','policy_sha256','checks','coverage'}}

def _module(name):
    import integration_ci
    return integration_ci._adjacent_module(name)

def _oid(value,label):
    if not isinstance(value,str) or not re.fullmatch(r'(?:[0-9a-f]{40}|[0-9a-f]{64})',value):
        raise ValueError(label+' immutable identity unavailable')
    return value

def _record(key,repository,uid,head,**fields):
    evidence=fields.pop('_evidence')
    return {'schema':'oasis7-terminal-recovery-'+key+'/v1','repository':repository,
        'task_uid':uid,'head_oid':head,'observed_at':obs.now(),
        'evidence':evidence,**fields}

def _primary(locator,kind='github_api'):
    """Keep the complete response from a fixed, independently validated source."""
    matches=[item for item in obs.active().evidence if item['locator']==locator and item['kind']==kind]
    if not matches:raise ValueError('recovery primary observation unavailable: '+locator)
    return [matches[-1]]

def _integration_primary(repository,run_id):
    endpoint=f'repos/{repository}/actions/runs/{run_id}/artifacts'
    found=[]
    for item in obs.active().evidence:
        if item['kind']=='github_api' and item['locator'].startswith(endpoint+'?'):
            value=obs.load(base64.b64decode(item['raw_b64'],validate=True))
            found.extend(a['id'] for a in value['artifacts'] if a.get('name')=='oasis7-required-plan-v1' and not a.get('expired'))
    identities=set(found)
    if len(identities)!=1:raise ValueError('recovery integration primary artifact ambiguous')
    return _primary(f'repos/{repository}/actions/artifacts/{identities.pop()}/zip','repository_artifact')

def _merge_primary(repository,number,head,merge,target,branch,pr):
    owner,name=repository.split('/')
    query='query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){nameWithOwner defaultBranchRef{name target{... on Commit{oid}}} pullRequest(number:$number){number url state merged mergedAt headRefOid headRefName baseRefName headRepository{nameWithOwner} repository{nameWithOwner} mergeCommit{oid}}}}'
    raw=obs.capture(['gh','api','graphql','-f','query='+query,'-f','owner='+owner,'-f','name='+name,
        '-F','number='+str(number)],kind='github_api',locator='graphql')
    value=obs.load(raw)
    if not isinstance(value,dict) or ('errors' in value and value['errors']!=[]):
        raise ValueError('recovery merge GraphQL errors/malformed response')
    data=value.get('data');repo=data.get('repository') if isinstance(data,dict) else None
    if not isinstance(repo,dict) or repo.get('nameWithOwner')!=repository:
        raise ValueError('recovery merge GraphQL repository mismatch')
    ref=repo.get('defaultBranchRef');remote=repo.get('pullRequest')
    if not isinstance(ref,dict) or ref.get('name')!=branch or not isinstance(ref.get('target'),dict) or ref['target'].get('oid')!=target:
        raise ValueError('recovery merge GraphQL default target mismatch')
    expected={'number':number,'url':pr['html_url'],'state':'MERGED','merged':True,'mergedAt':pr['merged_at'],
        'headRefOid':head,'headRefName':pr['head']['ref'],'baseRefName':branch,
        'headRepository':{'nameWithOwner':repository},'repository':{'nameWithOwner':repository},'mergeCommit':{'oid':merge}}
    if not isinstance(remote,dict) or type(remote.get('number')) is not int or type(remote.get('merged')) is not bool or any(remote.get(k)!=v for k,v in expected.items()):
        raise ValueError('recovery merge GraphQL canonical PR facts mismatch')
    return _primary('graphql')

def _canonical_context(root,uid,repository,number,head,app):
    """Resolve canonical immutable review inputs, never caller verdicts."""
    task=root/'.pm'/'scratch'/uid
    matches=[]
    for path in sorted((task/'review-handoffs').glob('*.json')):
        value=obs.load(path.read_bytes())
        if not isinstance(value,dict):raise ValueError('recovery review candidate malformed')
        if value.get('task_uid')==uid and value.get('frozen_head')==head:
            context={'repository_root':str(root),'source_review_plan_path':str(root/value['plan_path']),
                'source_review_handoff_path':str(path),
                'source_review_resolution_path':str(task/'review-resolutions'/(value['epoch']+'.json')),
                'source_scope_oid':(value.get('source_review_identity') or {}).get('source_scope_oid'),
                'check_app_id':app}
            plan=obs.load(Path(context['source_review_plan_path']).read_bytes())
            context['source_scope_oid']=plan.get('source_scope_oid')
            validated=review_context(repository,uid,number,head,context)
            matches.append((value['dispatch_evidence']['comment_id'],context,validated))
    if not matches:raise ValueError('recovery accepted source review unavailable')
    ids=[item[0] for item in matches]
    if any(type(ident) is not int or ident<1 for ident in ids) or len(ids)!=len(set(ids)):
        raise ValueError('recovery review dispatch identity ambiguous')
    return max(matches,key=lambda item:item[0])[1:]

def _authorization(repository,uid,issue,number,head,merge,comments):
    marker='<!-- oasis7-terminal-recovery-authorization/v1 -->'
    selected=[c for c in comments if marker in str(c.get('body') or '')]
    if len(selected)!=1:raise ValueError('recovery authorization missing/duplicate/ambiguous')
    comment=selected[0];body=comment.get('body')
    if not isinstance(body,str) or not body.startswith(marker+'\n'):
        raise ValueError('recovery authorization marker malformed')
    value=obs.load(body[len(marker)+1:].encode())
    expected={'schema':'oasis7-terminal-recovery-authorization/v1','repository':repository,
        'task_uid':uid,'issue_number':issue,'pr_number':number,'head_oid':head,
        'merge_commit_oid':merge,'purpose':'postmerge_delivery_only',
        'decision':'authorize_current_terminal_observation'}
    obs.closed(value,set(expected)|{'reason'},'recovery authorization')
    if any(type(value[k]) is not type(v) or value[k]!=v for k,v in expected.items()) or not isinstance(value['reason'],str) or not value['reason'].strip():
        raise ValueError('recovery authorization exact delivery identity mismatch')
    readiness=_module('readiness_transport')
    captured=readiness.comment_capture(comment)
    readiness.validate_comment(repository,issue,captured,comments,body.encode(),marker,admin=True,check_permission=True)
    return captured

def _prior_readiness(receipt_root,comments):
    readiness=_module('readiness_transport')
    native=[c for c in comments if readiness.NATIVE in str(c.get('body') or '')]
    migration=[c for c in comments if readiness.MIGRATION in str(c.get('body') or '')]
    candidates=[]
    proof=receipt_root/'readiness-proof.json'
    if proof.exists():
        try:obs.load(proof.read_bytes())
        except (OSError,ValueError) as exc:raise ValueError('recovery native candidate corrupt/invalid; absence forbidden') from exc
        candidates.append(proof)
    namespace=receipt_root/'native-readiness'
    if namespace.exists():
        if not namespace.is_dir():raise ValueError('recovery native candidate namespace malformed')
        candidates.extend(path for path in namespace.rglob('*') if path.is_file())
    if native or migration or candidates:
        raise ValueError('recovery native/migration readiness candidate exists; absence forbidden')
    return {'status':'unavailable','reason':'complete canonical native/migration inventory has no prior readiness',
        'native_marker_count':0,'migration_marker_count':0,'canonical_artifact_count':0}

def _current_holds(root,repository,uid,issue,number,head,record,comments):
    terminal=_module('loop_terminal');gate=_module('pr-lifecycle-gate')
    project=terminal.read_project(repository,issue)
    items=project.get('items')
    if project.get('page_complete') is not True or not isinstance(items,list):
        raise ValueError('recovery current Project pagination unavailable')
    matches=[item for item in items if item.get('id')==record.get('project_item_id')]
    if len(matches)!=1:raise ValueError('recovery current Project item ambiguous')
    item=matches[0];terminal._validate_selected_project_item(item,repository,issue,project['id'])
    fields=terminal.normalize_project_fields(item,repository)
    if fields.get('Task UID')!=uid or not isinstance(fields.get('Canonical Worktree'),str) or Path(fields['Canonical Worktree']).resolve(strict=True)!=root:
        raise ValueError('recovery current Project Task binding mismatch')
    if fields.get('PM Status')!=record.get('status'):
        raise ValueError('recovery current Project task status mismatch')
    holds=[]
    for comment in comments:
        body=str(comment.get('body') or '')
        if '<!-- oasis7-merge-hold -->' in body:
            values=dict(re.findall(r'^- ([a-z_]+): `?([^`\n]+)`?$',body,re.M))
            if values.get('task_uid')!=uid or values.get('repository')!=repository:
                raise ValueError('recovery current hold identity unavailable')
            if values.get('active') not in ('true','false'):raise ValueError('recovery current hold state unknown')
            if values['active']=='true':holds.append(obs.positive(comment.get('id'),'hold comment'))
    reviews=obs.pages(f'repos/{repository}/pulls/{number}/reviews')
    changes=[obs.positive(review.get('id'),'review') for review in gate.latest_reviews(reviews)
        if str(review.get('state') or '').upper()=='CHANGES_REQUESTED']
    owner,name=repository.split('/')
    query='''query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){pullRequest(number:$number){reviewThreads(first:100){pageInfo{hasNextPage endCursor}nodes{id isResolved isOutdated comments(first:100){pageInfo{hasNextPage}nodes{id body}}}}}}}'''
    raw=obs.load(obs.capture(['gh','api','graphql','-f','query='+query,'-f','owner='+owner,
        '-f','name='+name,'-F','number='+str(number)],kind='github_api',locator='graphql'))
    if not isinstance(raw,dict) or raw.get('errors'):raise ValueError('recovery current review threads unavailable')
    threads=(((raw.get('data') or {}).get('repository') or {}).get('pullRequest') or {}).get('reviewThreads')
    if (not isinstance(threads,dict) or not isinstance(threads.get('pageInfo'),dict)
            or threads['pageInfo'].get('hasNextPage') is not False or not isinstance(threads.get('nodes'),list)):
        raise ValueError('recovery current review thread pagination incomplete')
    unresolved=[]
    for thread in threads['nodes']:
        if not isinstance(thread,dict) or type(thread.get('isResolved')) is not bool or not isinstance(thread.get('id'),str):
            raise ValueError('recovery current review thread malformed')
        if not thread['isResolved']:unresolved.append(thread['id'])
    operator=obs.api('user');login=operator.get('login')
    _module('readiness_transport').permission(repository,login,admin=True)
    if holds or changes or unresolved:raise ValueError('recovery current holds/reviews/threads block terminal observation')
    return _record('current_holds',repository,uid,head,_evidence=_primary(f'repos/{repository}/collaborators/{login}/permission'),issue_number=issue,pr_number=number,
        project_id=project['id'],item_id=item['id'],task_status=record['status'],
        workflow_phase=record['workflow_phase'],hold_comment_ids=holds,
        changes_requested_review_ids=changes,unresolved_thread_ids=unresolved,
        operator_login=login,operator_permission='admin')

def review_context(repository,uid,number,head,context):
    obs.closed(context,TARGET_CONTEXT,'current target context')
    root=Path(context['repository_root']).resolve(strict=True)
    app=obs.positive(context['check_app_id'],'policy app')
    review=_module('review_preflight_handoff')
    validated=review.validate_handoff(root,Path(context['source_review_handoff_path']),
        expected_plan_path=Path(context['source_review_plan_path']))
    plan=validated['plan'];handoff=validated['handoff']
    if (plan['task_uid']!=uid or plan['frozen_head']!=head
            or plan['source_scope_oid']!=context['source_scope_oid']
            or handoff['repository']!=repository or handoff['pr_number']!=number):
        raise ValueError('current target source review identity mismatch')
    _module('review-findings-resolution').validate_manifest(root,
        Path(context['source_review_resolution_path']),Path(validated['ledger_path']),uid,head)
    return root,app,validated,review.canonical_task_issue_number(root,uid)

def merged_facts(repository,uid,number,head,merge,target,root,issue):
    for label,value in [('head',head),('merge',merge),('target',target)]:_oid(value,label)
    pr=obs.api(f'repos/{repository}/pulls/{number}')
    repo=obs.api(f'repos/{repository}')
    branch=repo.get('default_branch')
    if not isinstance(branch,str) or (pr.get('base') or {}).get('ref')!=branch:
        raise ValueError('current target default branch identity mismatch')
    terminal=_module('terminal_proof')
    terminal._validate_live_pr(pr,repository,uid,issue,number,
        f'https://github.com/{repository}/pull/{number}',head,merge,branch)
    ref=obs.api(f'repos/{repository}/git/ref/heads/{branch}')
    if (ref.get('object') or {}).get('sha')!=target:raise ValueError('current target identity moved')
    comparison=obs.api(f'repos/{repository}/compare/{merge}...{target}')
    terminal._validate_live_repository({'repository':repo,'ref':ref,'merge_compare':comparison},
        repository,merge,branch,target)
    if obs.git(root,'merge-base',merge,target).decode().strip()!=merge:
        raise ValueError('current target Git ancestry mismatch')
    return branch,pr

def _required_gate_app(value):
    """Resolve one pinned application without discarding wildcard obligations."""
    required=value.get('required_status_checks')
    if value.get('status')!='resolved' or not isinstance(required,list) or len(required) not in (1,2):
        raise ValueError('recovery current required policy unsupported/incomplete')
    pinned=[];wildcards=0
    for check in required:
        if not isinstance(check,dict) or check.get('context')!='required-gate' or 'app_id' not in check:
            raise ValueError('recovery current required policy unsupported/incomplete')
        app=check['app_id']
        if app is None:wildcards+=1
        elif type(app) is int and app>0:pinned.append(app)
        else:raise ValueError('recovery current required policy app unsupported/incomplete')
    if len(pinned)!=1 or wildcards>1:
        raise ValueError('recovery current required policy app unsupported/incomplete')
    return pinned[0]

def _policy(repository,branch,app):
    class ReadOnlyClient:
        def rest(self,method,path,**kwargs):
            if method!='GET':raise ValueError('required policy observation is read-only')
            return obs.api(path)
    value=_module('pr-lifecycle-gate').discover_required_policy(repository,branch,client=ReadOnlyClient())
    if _required_gate_app(value)!=app or type(app) is not int:
        raise ValueError('required policy/app coverage unsupported')
    return value

def _planner(root,revision,paths,event='push'):
    entry=obs.git(root,'show',revision+':scripts/plan-rust-required-scope.py')
    config=obs.git(root,'show',revision+':scripts/ci-required-scope.v2.json')
    # Execute immutable planner/config only; do not update the canonical checkout.
    with tempfile.TemporaryDirectory() as temp:
        script=Path(temp)/'plan.py';cfg=Path(temp)/'config.json'
        script.write_bytes(entry);cfg.write_bytes(config)
        (Path(temp)/'ci-tests.sh').write_bytes(obs.git(root,'show',revision+':scripts/ci-tests.sh'))
        command=[sys.executable,'-I',str(script),'--config',str(cfg),'--event-name',event]
        for path in paths:command+=['--changed-path',path]
        raw=obs.capture(command,cwd=root)
    outputs=dict(line.split('=',1) for line in raw.decode().splitlines() if '=' in line)
    try:normalized=_module('ci-ready-receipt').canonical_planner(outputs)
    except SystemExit as exc:raise ValueError('recovery plan metadata: '+str(exc)) from exc
    return normalized,entry,config

def _paths(root,base,head):
    # No rename collapsing: both deleted and added names are retained.
    return sorted(set(obs.git(root,'diff','--name-only','--no-renames','-z',base,head).decode().split('\0'))-{''})

def _latest_push(repository,branch,head):
    runs=obs.pages(f'repos/{repository}/actions/workflows/rust.yml/runs?event=push','workflow_runs')
    matches=[]
    for run in runs:
        if (run.get('repository') or {}).get('full_name')!=repository or run.get('path')!='.github/workflows/rust.yml' or run.get('event')!='push':
            raise ValueError('push discovery provenance malformed')
        if run.get('head_sha')==head and run.get('head_branch')==branch:
            obs.positive(run.get('id'),'push run');obs.instant(run.get('created_at'))
            matches.append(run)
    if not matches:raise ValueError('latest current push evidence unavailable')
    selected=max(matches,key=lambda r:(obs.instant(r['created_at']),r['id']))
    run=obs.api(f'repos/{repository}/actions/runs/{selected["id"]}')
    if any(run.get(k)!=selected.get(k) for k in ('id','run_attempt','head_sha','head_branch','event','path','status','conclusion')):
        raise ValueError('latest push run identity changed')
    if type(run.get('run_attempt')) is not int or run['run_attempt']!=1:
        raise ValueError('unsupported-attempt: current push requires first attempt')
    if run.get('status')!='completed' or run.get('conclusion')!='success':
        raise ValueError('latest push execution is not successful')
    attempt=obs.api(f'repos/{repository}/actions/runs/{run["id"]}/attempts/1')
    if any(attempt.get(k)!=run.get(k) for k in ('id','run_attempt','head_sha','event','status','conclusion')):
        raise ValueError('push exact attempt identity mismatch')
    return run

def _pr_commits(repository,number):
    rows=[];seen=set()
    for page in range(1,101):
        endpoint=f'repos/{repository}/pulls/{number}/commits?per_page=100&page={page}'
        batch=obs.load(obs.capture(['gh','api',endpoint],kind='github_api',locator=endpoint))
        if not isinstance(batch,list) or len(batch)>100:raise ValueError('source PR commit discovery malformed')
        for row in batch:
            sha=_oid(row.get('sha') if isinstance(row,dict) else None,'source PR commit')
            if sha in seen:raise ValueError('source PR commit discovery duplicate')
            seen.add(sha);rows.append(sha)
        if len(batch)<100:return rows
    raise ValueError('source PR commit discovery incomplete at page bound')

def _source_run(repository,number,head,branch,app):
    pr=obs.api(f'repos/{repository}/pulls/{number}')
    for name in ('head','base'):
        value=pr.get(name)
        if not isinstance(value,dict) or not isinstance(value.get('repo'),dict) or value['repo'].get('full_name')!=repository:
            raise ValueError('source PR repository provenance mismatch')
    if (pr.get('number')!=number or pr.get('merged') is not True or pr.get('state')!='closed'
            or pr['head'].get('sha')!=head or pr['base'].get('ref')!=branch
            or not isinstance(pr['head'].get('ref'),str) or not pr['head']['ref']):
        raise ValueError('source canonical merged PR/head/ref mismatch')
    base=_oid(pr['base'].get('sha'),'source independent PR base')
    commits=_pr_commits(repository,number)
    if commits.count(head)!=1 or commits[-1]!=head:raise ValueError('source accepted PR commit membership mismatch')
    runs=obs.pages(f'repos/{repository}/actions/workflows/rust.yml/runs?head_sha={head}','workflow_runs')
    matching=[]
    for candidate in runs:
        if not isinstance(candidate.get('repository'),dict) or candidate['repository'].get('full_name')!=repository or candidate.get('path')!='.github/workflows/rust.yml':
            raise ValueError('source workflow discovery provenance malformed')
        if candidate.get('head_sha')==head and candidate.get('head_branch')==pr['head']['ref']:
            obs.positive(candidate.get('id'),'source run');obs.instant(candidate.get('created_at'));matching.append(candidate)
    if not matching:raise ValueError('source latest execution unavailable')
    selected=max(matching,key=lambda row:(obs.instant(row['created_at']),row['id']))
    run=obs.api(f'repos/{repository}/actions/runs/{selected["id"]}')
    if any(run.get(key)!=selected.get(key) for key in ('id','run_attempt','head_sha','head_branch','event','path','status','conclusion')):
        raise ValueError('source latest execution identity changed')
    if run.get('event')!='pull_request':raise ValueError('unsupported source event execution contract')
    if type(run.get('run_attempt')) is not int or run['run_attempt']!=1:
        raise ValueError('unsupported source producer attempt provenance')
    if run.get('status')!='completed' or run.get('conclusion')!='success':raise ValueError('source latest execution unsuccessful')
    attempt=obs.api(f'repos/{repository}/actions/runs/{run["id"]}/attempts/1')
    if any(attempt.get(key)!=run.get(key) for key in ('id','run_attempt','head_sha','event','status','conclusion')):
        raise ValueError('source exact attempt identity mismatch')
    checks=obs.pages(f'repos/{repository}/commits/{head}/check-runs','check_runs')
    required=[check for check in checks if check.get('name')=='required-gate']
    if not required:raise ValueError('source required policy check unavailable')
    for check in required:
        obs.positive(check.get('id'),'source check')
        if not isinstance(check.get('app'),dict) or check['app'].get('id')!=app:
            raise ValueError('source required check App mismatch')
    latest=max(required,key=lambda check:check['id'])
    if (latest.get('head_sha')!=head or latest.get('status')!='completed' or latest.get('conclusion')!='success'
            or _module('ci-ready-receipt')._workflow_job_details(latest,repository)[0]!=run['id']):
        raise ValueError('source latest required check execution mismatch')
    return run,pr,base,commits,latest['id']

def _source_association(value,number,head,base,branch):
    rows=value.get('pull_requests',[])
    if not isinstance(rows,list):raise ValueError('source official association malformed')
    if not rows:return
    if len(rows)!=1 or not isinstance(rows[0],dict):raise ValueError('source official association conflicting/ambiguous')
    row=rows[0]
    if (type(row.get('number')) is not int or row['number']!=number
            or not isinstance(row.get('head'),dict) or row['head'].get('sha')!=head
            or not isinstance(row.get('base'),dict) or row['base'].get('sha')!=base or row['base'].get('ref')!=branch):
        raise ValueError('source official association conflicts with canonical PR')

def push_observation(repository,uid,head,execution,branch,root,range_base,app):
    return _execution_observation(repository,uid,head,execution,branch,root,range_base,app)

def source_observation(repository,uid,head,number,branch,root,app):
    return _execution_observation(repository,uid,head,head,branch,root,None,app,source_number=number)

# Private recognition of the reviewed finite historical producer contract.
# These fingerprints constrain code identity; live bindings, execution, plan,
# selected coverage and resources below independently establish acceptance.
_HISTORICAL_SOURCE_BLOBS={
    '.github/workflows/rust.yml':'101c2d0d87c04f42315878db2a2b18c0000d6d247c46e90250862ffad35bf16b',
    'scripts/ci-tests.sh':'584e1ca8ef0ba47b675026e38dadbf0254bd377cda9877d45ec76b7406c95fd0',
    'scripts/ci-required-capability-test-inventory.tsv':'ef280f9bc645eb9e0bb101a4f23de5c57e50fc874365b1bc108b17255464edc8',
    'scripts/plan-rust-required-scope.py':'0bec148fdf8c358af0f6fc7cef1f6b6c37bf3b0b8e00ef385f724401f12cc8a5',
    'scripts/ci-required-scope.v2.json':'b58ac1305cd5ddd8d009daef6e9072b22538c95c5d2707a9c9bfeece973bf7c6',
    'scripts/pm/workflow-impact-projection.py':'5e1784df832512282391f32a4f0e4d46865aeb5e68d36d17f10beb5ac9bf9079',
    'scripts/viewer-dependency-preflight.sh':'32dc1ad5ee84cb1be0aca292d5c41e40c7ff96e45acb83d1b34a6fcb25bec177',
    'scripts/pm/review-role-selector.py':'9ce658d7385f44f533660ed82220464066078d909329ea63393c8eeba4f2cf35',
}

def _historical_source_bindings(repository,uid,number,head,base,branch,root,app):
    root=Path(root).resolve(strict=True)
    context,validated=_canonical_context(root,uid,repository,number,head,app)
    if context['source_scope_oid']!=base:
        raise ValueError('historical source B differs from authenticated review scope')
    pr=obs.api(f'repos/{repository}/pulls/{number}')
    if (pr.get('base') or {}).get('sha')!=base:
        raise ValueError('historical source canonical PR base mismatch')
    merge=_oid(pr.get('merge_commit_sha'),'historical source merge')
    target=_oid((obs.api(f'repos/{repository}/git/ref/heads/{branch}').get('object') or {}).get('sha'),'historical current target')
    actual_branch,_=merged_facts(repository,uid,number,head,merge,target,root,validated[3])
    if actual_branch!=branch:raise ValueError('historical source default branch mismatch')
    commit=obs.api(f'repos/{repository}/git/commits/{merge}')
    parents=commit.get('parents') if isinstance(commit,dict) else None
    if (not isinstance(commit,dict) or commit.get('sha')!=merge or not isinstance(parents,list)
            or any(not isinstance(parent,dict) for parent in parents)
            or [parent.get('sha') for parent in parents]!=[base]
            or not isinstance(commit.get('tree'),dict)
            or commit['tree'].get('sha')!=obs.git(root,'rev-parse',merge+'^{tree}').decode().strip()
            or obs.git(root,'show','-s','--format=%P',merge).decode().split()!=[base]):
        raise ValueError('historical source B is not the exact sole merge parent')
    if obs.git(root,'diff','--binary',base,head)!=obs.git(root,'diff','--binary',base,merge):
        raise ValueError('historical source accepted patch equivalence mismatch')
    # Authenticate the original strict execution with the existing merged
    # identity seam. This grants no current-T coverage verdict; the full
    # collector separately retains its original/current integration guards.
    integration=_module('integration_ci')
    selected=integration.current_request(repository,uid,number,base,head,branch)
    if not isinstance(selected,dict):raise ValueError('historical source original integration unavailable')
    if selected.get('workflow_run_head_sha')!=base:
        raise ValueError('historical source original integration workflow B mismatch')
    run_id=obs.positive(selected.get('id'),'historical integration run')
    attempt=obs.positive(selected.get('run_attempt'),'historical integration attempt')
    _,proof=integration._verified_run(repository,uid,number,base,head,run_id,app,
        expected_attempt=attempt,_merged_branch=branch)
    if (proof.get('scope_base_oid')!=base or proof.get('tested_commit_oid')!=head
            or proof.get('tested_tree_oid')!=obs.git(root,'rev-parse',head+'^{tree}').decode().strip()):
        raise ValueError('historical source original integration B/H/tree mismatch')

def _historical_source_producer(repository,uid,number,head,base,branch,root,app,planner,job):
    _historical_source_bindings(repository,uid,number,head,base,branch,root,app)
    blobs={}
    for path,expected in _HISTORICAL_SOURCE_BLOBS.items():
        delivered=obs.git(root,'show',head+':'+path)
        authority=obs.git(root,'show',base+':'+path)
        if delivered!=authority or obs.digest(authority)!=expected:
            raise ValueError('unsupported historical source producer/dependency: '+path)
        blobs[path]=authority
    true_selectors={'run_required_gate_baseline','run_workflow_governance_contracts','run_rust_baseline'}
    true_resources={'needs_python','needs_markdown','needs_rust_toolchain'}
    if (planner.get('execution_contract')!='required-domain-split/v1'
            or planner.get('selected_capabilities')!=['workflow_governance']
            or planner.get('scope')!='targeted'
            or {key for key,value in planner.items() if key.startswith('run_') and value is True}!=true_selectors
            or {key for key,value in planner.items() if key.startswith('needs_') and value is True}!=true_resources):
        raise ValueError('unsupported historical source selector/resource profile')
    workflow=blobs['.github/workflows/rust.yml'].decode('utf-8')
    gate=re.findall(r'^  required-gate:\n(.*?)(?=^  [A-Za-z0-9_-]+:|\Z)',workflow,re.M|re.S)
    if len(gate)!=1 or '    runs-on: ubuntu-24.04\n' not in gate[0] or 'continue-on-error:' in gate[0]:
        raise ValueError('historical source checked required job unsupported')
    required=re.findall(r'^      - name: Run required test tier\n(.*?)(?=^      - |\Z)',gate[0],re.M|re.S)
    if len(required)!=1:raise ValueError('historical source required dispatcher step ambiguous')
    step=required[0]
    config=obs.load(blobs['scripts/ci-required-scope.v2.json'])
    bindings={row['name']:row['planner_field'] for row in config['selector_ownership'] if row['mode']=='planner-owned'}
    bindings.update({'OASIS7_CI_EXECUTION_CONTRACT':'execution_contract'})
    for resource in ('python','markdown','rust_toolchain','node','system_deps','trunk','wasm_target'):
        bindings['OASIS7_CI_NEEDS_'+resource.upper()]='needs_'+resource
    for name,field in bindings.items():
        expected='          '+name+': ${{ steps.scope.outputs.'+field+' }}'
        if step.splitlines().count(expected)!=1:
            raise ValueError('historical source selector/resource environment mismatch: '+name)
    if ('          elif [[ "${GITHUB_EVENT_NAME}" == "pull_request" && -f "${RUNNER_TEMP}/impact-projection.json" ]]; then\n'
            '            CI_VERBOSE=1 ./scripts/ci-tests.sh required --impact-projection "${RUNNER_TEMP}/impact-projection.json"\n'
            '          else\n            CI_VERBOSE=1 ./scripts/ci-tests.sh required\n          fi') not in step:
        raise ValueError('historical source ordinary PR checked dispatcher invocation mismatch')
    dispatcher=blobs['scripts/ci-tests.sh'].decode('utf-8')
    if (not dispatcher.startswith('#!/usr/bin/env bash\nset -euo pipefail\n')
            or 'run() {\n  echo "+ $*"\n  "$@"\n}' not in dispatcher
            or 'validate_required_gate_execution_contract || exit 1' not in dispatcher
            or 'source "$driver_dir/viewer-dependency-preflight.sh"' not in dispatcher):
        raise ValueError('historical source dispatcher failure propagation unsupported')
    # Traverse only the recognized straight-line selected governance route.
    # The complete byte recognition constrains branches outside this route;
    # this is not a permissive generic shell parser.
    def function(name):
        found=re.findall(r'^'+re.escape(name)+r'\(\) \{\n(.*?)^\}',dispatcher,re.M|re.S)
        if len(found)!=1:raise ValueError('historical selected dispatcher function missing: '+name)
        return found[0]
    tier=re.findall(r'^  required\)\n(.*?)^    ;;',dispatcher,re.M|re.S)
    if len(tier)!=1 or not tier[0].startswith('    run_required_gate_checks\n    if [[ "$required_gate_execution_contract" == required-domain-split/v1 ]]; then\n      run_required_gate_capability_contracts\n    fi\n'):
        raise ValueError('historical source required-tier selection mismatch')
    if 'run_required_gate_capability_component "workflow governance contracts" OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS run_workflow_governance_contract_tests' not in function('run_required_gate_capability_contracts'):
        raise ValueError('historical source selected governance dispatch missing')
    pending=['run_workflow_governance_contract_tests'];seen=set();paths=set()
    while pending:
        name=pending.pop()
        if name in seen:continue
        seen.add(name);body=function(name)
        if '||' in body or 'continue-on-error' in body:
            raise ValueError('historical selected unchecked dispatcher function')
        paths.update(re.findall(r'\./(scripts/[A-Za-z0-9_./-]+)',body))
        pending.extend(re.findall(r'^  (run_[A-Za-z0-9_]+)\s*$',body,re.M))
    reader=csv.DictReader(io.StringIO(blobs['scripts/ci-required-capability-test-inventory.tsv'].decode('utf-8')),delimiter='\t')
    expected_header=['historical_required_location','test_paths','new_required_selection','legacy_required_coverage','full_full_core_full_support']
    if reader.fieldnames!=expected_header:raise ValueError('historical source inventory header unsupported')
    rows=list(reader)
    if len(rows)!=16 or len({tuple(row.get(key) for key in expected_header) for row in rows})!=len(rows):
        raise ValueError('historical source inventory incomplete/duplicate')
    for row in rows:
        if set(row)!=set(expected_header) or any(not isinstance(value,str) or not value for value in row.values()):
            raise ValueError('historical source inventory row malformed')
        if row['new_required_selection']=='workflow_governance':
            tests=row['test_paths'].split(',')
            if any(not path or path not in paths for path in tests):
                raise ValueError('historical source selected inventory test missing from dispatcher')
    steps=job.get('steps')
    if not isinstance(steps,list) or any(not isinstance(item,dict) for item in steps):
        raise ValueError('historical source resource step metadata malformed')
    labels=job.get('labels')
    if not isinstance(labels,list) or 'ubuntu-24.04' not in labels:
        raise ValueError('historical source required runner/resource contract mismatch')
    tests=[item for item in steps if item.get('name')=='Run required test tier']
    if len(tests)!=1:raise ValueError('historical source checked test execution ambiguous')
    test_number=obs.positive(tests[0].get('number'),'historical checked test step')
    test_start=obs.instant(tests[0].get('started_at'))
    for name in ('Install pinned Rust toolchains','Install cargo-deny','Install product-document Markdown parser'):
        matches=[item for item in steps if item.get('name')==name]
        if len(matches)!=1 or matches[0].get('status')!='completed' or matches[0].get('conclusion')!='success':
            raise ValueError('historical source selected resource execution unavailable: '+name)
        resource=matches[0]
        if (obs.positive(resource.get('number'),'historical resource step')>=test_number
                or obs.instant(resource.get('started_at'))>obs.instant(resource.get('completed_at'))
                or obs.instant(resource.get('completed_at'))>test_start):
            raise ValueError('historical source resource did not precede checked execution: '+name)
    return blobs

def _selected_execution_children(workflow,planner,event):
    receipt=_module('ci-ready-receipt');selected=receipt._selected_child_groups(planner)
    prefix="(github.event_name == 'pull_request' || (github.event_name == 'workflow_dispatch' && inputs.run_mode == 'integration_revalidation')) && "
    operational="needs.required-gate.outputs.run_operational_contracts == 'true'"
    packaging="((needs.required-gate.outputs.execution_contract == 'required-domain-split/v1' && needs.required-gate.outputs.run_packaging_contracts == 'true') || (needs.required-gate.outputs.execution_contract != 'required-domain-split/v1' && needs.required-gate.outputs.run_operational_contracts == 'true'))"
    packaging_v2="(((needs.required-gate.outputs.execution_contract == 'required-domain-split/v1' || needs.required-gate.outputs.execution_contract == 'required-domain-split/v2') && needs.required-gate.outputs.run_packaging_contracts == 'true') || (needs.required-gate.outputs.execution_contract == '' && needs.required-gate.outputs.run_operational_contracts == 'true'))"
    expected={receipt.WINDOWS_ROLLOUT_JOB:prefix+operational,
        receipt.MACOS_PACKAGE_JOB:prefix+packaging,receipt.FLEET_HEALTH_JOB:prefix+operational}
    text=workflow.decode('utf-8')
    for name,condition in expected.items():
        blocks=re.findall(r'^  '+re.escape(name)+r':\n(.*?)(?=^  [A-Za-z0-9_-]+:|\Z)',text,re.M|re.S)
        if len(blocks)!=1:raise ValueError('unsupported immutable child execution job')
        matches=re.findall(r'^    if: ([^\n]+)(?:\n((?:      [^\n]*\n)*))?',blocks[0],re.M)
        if len(matches)!=1:raise ValueError('unsupported immutable child execution condition')
        line,continuation=matches[0];actual=continuation if line=='>-' else line
        supported=(condition,prefix+packaging_v2) if name==receipt.MACOS_PACKAGE_JOB else (condition,)
        if ''.join(actual.split()) not in {''.join(item.split()) for item in supported}:
            raise ValueError('unsupported changed event/selector child execution condition')
    if event=='push':return {name:False for name in selected}
    if event=='pull_request':return selected
    raise ValueError('unsupported selected-child execution event')

def _source_execution(repository,number,head,base,run,job,root):
    """Derive W from the proven ordinary PR event merge, never head_sha alone."""
    if run.get('event')!='pull_request' or run.get('path')!='.github/workflows/rust.yml' or run.get('referenced_workflows')!=[]:
        raise ValueError('unsupported source root workflow/event indirection')
    workflow_id=obs.positive(run.get('workflow_id'),'source workflow')
    namespace=obs.api(f'repos/{repository}/actions/workflows/{workflow_id}')
    if namespace.get('id')!=workflow_id or namespace.get('path')!=run['path']:
        raise ValueError('source root workflow namespace mismatch')
    checkout=[step for step in job.get('steps',[]) if isinstance(step,dict) and step.get('name')=='Run actions/checkout@v6']
    if len(checkout)!=1 or checkout[0].get('status')!='completed' or checkout[0].get('conclusion')!='success':
        raise ValueError('source default checkout step unavailable/ambiguous')
    start=obs.instant(checkout[0].get('started_at'))
    # GitHub step metadata is second precision, while logs retain fractions.
    end=obs.instant(checkout[0].get('completed_at'))+datetime.timedelta(seconds=1)
    endpoint=f'repos/{repository}/actions/jobs/{obs.positive(job.get("id"),"source job")}/logs'
    raw=obs.capture(['gh','api',endpoint],kind='github_api',locator=endpoint)
    lines=[]
    for line in raw.decode('utf-8').splitlines():
        stamp,separator,message=line.partition(' ')
        if not separator:raise ValueError('source checkout log line malformed')
        instant=obs.instant(stamp)
        if start<=instant<end:lines.append(message)
    groups=[index for index,line in enumerate(lines) if line=='##[group]Run actions/checkout@v6']
    if len(groups)!=1 or groups[0]+1>=len(lines) or lines[groups[0]+1]!='with:':
        raise ValueError('source rendered default checkout inputs unavailable')
    inputs={}
    for line in lines[groups[0]+2:]:
        if not line.startswith('  '):break
        key,separator,value=line.strip().partition(': ')
        if not separator or key in inputs:raise ValueError('source rendered checkout inputs malformed/duplicate')
        inputs[key]=value
    if (inputs.get('repository')!=repository or inputs.get('fetch-depth')!='0'
            or any(key in inputs for key in ('ref','path','sparse-checkout','filter'))
            or any(inputs.get(key,'false')!='false' for key in ('submodules','lfs'))):
        raise ValueError('source rendered checkout repository/override conflict')
    ref=f'refs/remotes/pull/{number}/merge'
    commands=[line for line in lines if re.fullmatch(r'\[command\].*?/git checkout --progress --force '+re.escape(ref),line)]
    if len(commands)!=1:raise ValueError('source historical PR merge checkout ref mismatch')
    positions=[index for index,line in enumerate(lines) if re.fullmatch(r'\[command\].*?/git log -1 --format=%H',line)]
    if len(positions)!=1 or positions[0]+1>=len(lines):raise ValueError('source checkout commit output unavailable/ambiguous')
    execution=_oid(lines[positions[0]+1],'source actual merge checkout')
    fetches=[line for line in lines if '[command]' in line and 'git ' in line and ' fetch ' in line
        and '+'+execution+':'+ref in line]
    if len(fetches)!=1:raise ValueError('source historical PR event SHA/ref provenance mismatch')
    commit=obs.api(f'repos/{repository}/git/commits/{execution}')
    accepted=obs.api(f'repos/{repository}/git/commits/{head}')
    for value,sha in ((commit,execution),(accepted,head)):
        if not isinstance(value,dict) or value.get('sha')!=sha or not isinstance(value.get('tree'),dict):
            raise ValueError('source immutable execution commit provenance mismatch')
        _oid(value['tree'].get('sha'),'source execution tree')
    parents=commit.get('parents')
    if not isinstance(parents,list) or any(not isinstance(parent,dict) for parent in parents) or [parent.get('sha') for parent in parents]!=[base,head]:
        raise ValueError('source actual merge checkout parent identity mismatch')
    tree=obs.git(root,'rev-parse',head+'^{tree}').decode().strip()
    if commit['tree']['sha']!=accepted['tree']['sha'] or accepted['tree']['sha']!=tree:
        raise ValueError('source actual merge checkout tree differs from accepted H')
    # Exact equal Git trees establish these file bytes at E without fabricating
    # a local E object or fetching/writing refs in the canonical checkout.
    workflow=obs.git(root,'show',head+':.github/workflows/rust.yml').decode('utf-8')
    trigger=re.findall(r'^  pull_request:\n(.*?)(?=^  [A-Za-z_]+:|\Z)',workflow,re.M|re.S)
    if len(trigger)!=1 or ''.join(trigger[0].split())!='branches:["main"]':
        raise ValueError('unsupported source ordinary PR trigger contract')
    if re.search(r'^    uses:',workflow,re.M):raise ValueError('unsupported source reusable workflow indirection')
    gates=re.findall(r'^  required-gate:\n(.*?)(?=^  [A-Za-z0-9_-]+:|\Z)',workflow,re.M|re.S)
    if len(gates)!=1 or '    steps:\n' not in gates[0]:raise ValueError('source required checkout contract unavailable')
    first=gates[0].split('    steps:\n',1)[1].split('\n      - ',1)[0]
    if first.rstrip()!='      - uses: actions/checkout@v6\n        with:\n          fetch-depth: 0':
        raise ValueError('unsupported source checkout override/configuration')
    return execution

def _execution_observation(repository,uid,head,execution,branch,root,range_base,app,*,source_number=None):
    source=None
    if source_number is None:run=_latest_push(repository,branch,execution)
    else:
        source=_source_run(repository,source_number,head,branch,app)
        run,pr,base,commits,latest_check=source;range_base=base
    run_id=run['id'];run_branch=run['head_branch'];event=run['event']
    suite=obs.api(f'repos/{repository}/check-suites/{run["check_suite_id"]}')
    if (suite.get('id')!=run['check_suite_id'] or suite.get('head_sha')!=execution
            or suite.get('after')!=execution or suite.get('head_branch')!=run_branch
            or (suite.get('app') or {}).get('id')!=app
            or (suite.get('repository') or {}).get('full_name')!=repository):
        raise ValueError('push check suite provenance mismatch')
    if source is None:base=_oid(suite.get('before'),'push independent before base')
    jobs=obs.pages(f'repos/{repository}/actions/runs/{run_id}/attempts/1/jobs','jobs')
    gates=[j for j in jobs if j.get('name')=='required-gate']
    if len(gates)!=1:raise ValueError('push required job identity ambiguous')
    job=gates[0];receipt=_module('ci-ready-receipt')
    try:identity=receipt._job_identity(job,repository,run_id,1,field='current push')
    except SystemExit as exc:raise ValueError('push job identity: '+str(exc)) from exc
    if identity['head_sha']!=execution or job.get('status')!='completed' or job.get('conclusion')!='success':
        raise ValueError('push job execution identity mismatch')
    check=obs.api(f'repos/{repository}/check-runs/{identity["check_run_id"]}')
    if (check.get('id')!=identity['check_run_id'] or (check.get('app') or {}).get('id')!=app
            or check.get('head_sha')!=execution or check.get('status')!='completed'
            or check.get('conclusion')!='success' or receipt._workflow_job_details(check,repository)!=(run_id,job['id'])):
        raise ValueError('push check app/job identity mismatch')
    if source is not None:
        if check['id']!=latest_check:raise ValueError('source latest check borrowed from older execution')
        for value in (run,suite,check):_source_association(value,source_number,head,base,branch)
    check_rows=obs.pages(f'repos/{repository}/check-suites/{run["check_suite_id"]}/check-runs','check_runs')
    if [c['id'] for c in check_rows if c.get('name')=='required-gate']!=[check['id']]:
        raise ValueError('push required check coverage ambiguous')
    required_names=('Plan required gate scope','Write required planner artifact','Upload required planner artifact','Run required test tier')
    steps={}
    for step in job.get('steps',[]):
        if not isinstance(step,dict):raise ValueError('push execution step malformed')
        if step.get('name') in required_names:
            if step['name'] in steps:raise ValueError('push execution step ambiguous')
            if step.get('status')!='completed' or step.get('conclusion')!='success':raise ValueError('push required execution step failed/skipped')
            obs.positive(step.get('number'),'step');steps[step['name']]=step
    if set(steps)!=set(required_names):raise ValueError('push executing tier steps unavailable')
    artifacts=obs.pages(f'repos/{repository}/actions/runs/{run_id}/artifacts','artifacts')
    named=[a for a in artifacts if a.get('name')=='oasis7-required-plan-v1']
    if len(named)!=1 or named[0].get('expired') is not False:raise ValueError('push artifact missing/ambiguous/expired')
    artifact=named[0];arun=artifact.get('workflow_run') or {}
    if arun.get('id')!=run_id or arun.get('head_sha')!=execution or arun.get('head_branch')!=run_branch:
        raise ValueError('push artifact run/head identity mismatch')
    start=obs.instant(steps[required_names[1]]['started_at']);end=obs.instant(steps[required_names[2]]['completed_at'])
    if not all(start<=obs.instant(artifact.get(k))<=end for k in ('created_at','updated_at')):
        raise ValueError('push artifact timing outside write/upload window')
    payload,raw=obs.artifact(repository,artifact['id'],'oasis7-required-plan-v1.json')
    obs.closed(payload,PLAN_FIELDS,'push plan')
    expected={'schema':'oasis7-required-plan-v1','repository':repository,'workflow_run_id':run_id,
        'head_oid':execution,'base_oid':base,'integration_base_oid':base,'check_name':'required-gate'}
    if any(type(payload[k]) is not type(v) or payload[k]!=v for k,v in expected.items()):raise ValueError('push plan base/source provenance mismatch')
    planner_input=payload['planner']
    derived_scope=obs.git(root,'merge-base',base,execution).decode().strip()
    locators={'source_scope_base':derived_scope,'integration_base':base,'source_head':execution}
    if not isinstance(planner_input,dict) or any(planner_input.get(key)!=value for key,value in locators.items()):
        raise ValueError('push planner source/base identity locator mismatch')
    try:actual=receipt.canonical_planner(payload['planner'])
    except SystemExit as exc:raise ValueError('push plan metadata: '+str(exc)) from exc
    planner_revision=base if source is not None else execution
    push_expected,entry,config=_planner(root,planner_revision,_paths(root,base,execution),event)
    # Producer attaches these range locators in addition to planner outputs.
    comparable={k:v for k,v in actual.items() if k not in ('source_scope_base','integration_base','source_head')}
    if comparable!=push_expected:raise ValueError('push plan selector/resource reproduction mismatch')
    cumulative,_,_=_planner(root,planner_revision,_paths(root,range_base,execution),event)
    selectors={k:v for k,v in actual.items() if k.startswith('run_')}
    resources={k:v for k,v in actual.items() if k.startswith('needs_')}
    expected_selectors={k:v for k,v in cumulative.items() if k.startswith('run_')}
    expected_resources={k:v for k,v in cumulative.items() if k.startswith('needs_')}
    if not selectors.get('run_required_gate_baseline') or any(v and not selectors.get(k) for k,v in expected_selectors.items()) or any(v and not resources.get(k) for k,v in expected_resources.items()):
        raise ValueError('push complete applicable selector/resource coverage missing')
    workflow=obs.git(root,'show',execution+':.github/workflows/rust.yml')
    dispatcher=obs.git(root,'show',execution+':scripts/ci-tests.sh')
    inventory=obs.git(root,'show',execution+':scripts/ci-required-capability-test-inventory.tsv')
    # Current-T trust remains the exact current supported producer. The
    # historical source has a distinct, bounded immutable contract and cannot
    # borrow current producer bytes or current-target successful executions.
    producers=[('.github/workflows/rust.yml',workflow),('scripts/ci-tests.sh',dispatcher),
        ('scripts/ci-required-capability-test-inventory.tsv',inventory)]
    changed=any(data!=(Path(__file__).resolve().parents[2]/path).read_bytes() for path,data in producers)
    if changed:
        if source is None:raise ValueError('unsupported trusted push dispatcher/workflow source changed')
        _historical_source_producer(repository,uid,source_number,head,base,branch,root,app,actual,job)
    if b'CI_VERBOSE=1 ./scripts/ci-tests.sh required' not in workflow or b'set -e' not in dispatcher:
        raise ValueError('push execution tier unchecked dispatcher unsupported')
    workflow_revision=execution if source is None else _source_execution(repository,source_number,head,base,run,job,root)
    selected=_selected_execution_children(workflow,actual,event)
    # Reuse pure child identity/outcome guards on the already bounded exact-attempt
    # collection; the ordinary receipt collector owns a separate transport.
    identities=[]
    for child in jobs:
        try:child_identity=receipt._job_identity(child,repository,run_id,1,field='push child')
        except SystemExit as exc:raise ValueError('push child identity: '+str(exc)) from exc
        if child_identity['head_sha']!=execution:raise ValueError('push child head mismatch')
        identities.append(child_identity)
    required_children=[]
    for name,runner in [(receipt.WINDOWS_ROLLOUT_JOB,'windows-2022'),(receipt.MACOS_PACKAGE_JOB,'ubuntu-24.04')]:
        if selected[name]:required_children.append((name,runner))
    if selected[receipt.FLEET_HEALTH_JOB]:
        required_children.extend((f'{receipt.FLEET_HEALTH_JOB} ({runner})',runner) for runner in receipt.FLEET_HEALTH_RUNNERS)
    child_checks=[]
    for name,runner in required_children:
        matches=[item for item in identities if item['name']==name]
        if len(matches)!=1:raise ValueError('push selected child missing/ambiguous')
        try:outcome=receipt._require_successful_child(matches[0],name,runner)
        except SystemExit as exc:raise ValueError('push child outcome: '+str(exc)) from exc
        child_checks.append(outcome['check_run_id'])
    if len(child_checks)!=len(set(child_checks)):raise ValueError('push child check identity duplicate')
    policy=_policy(repository,branch,app)
    normalized_check={'name':'required-gate','app_id':app,'check_run_id':check['id'],
        'run_id':run_id,'run_attempt':1,'job_id':job['id'],'workflow_sha':workflow_revision,
        'head_oid':execution,'ref':run_branch,'status':'completed','conclusion':'success','planner':actual}
    coverage={'range_base_oid':range_base,'range_head_oid':execution,'changed_paths':_paths(root,range_base,execution),
        'expected_selectors':expected_selectors,'actual_selectors':selectors,
        'expected_resources':expected_resources,'actual_resources':resources,
        'planner_entry_sha256':obs.digest(entry),'dispatcher_sha256':obs.digest(dispatcher),
        'inventory_sha256':obs.digest(inventory),'steps':[{'job_id':job['id'],'number':s['number'],
            'name':s['name'],'status':s['status'],'conclusion':s['conclusion'],'log_raw_sha256':None} for s in steps.values()]}
    if obs.api(f'repos/{repository}/actions/runs/{run_id}')!=run:raise ValueError('push final run identity moved')
    if obs.pages(f'repos/{repository}/actions/runs/{run_id}/artifacts','artifacts')!=artifacts:raise ValueError('push final artifact identity changed')
    final_payload,final_raw=obs.artifact(repository,artifact['id'],'oasis7-required-plan-v1.json')
    if final_raw!=raw:raise ValueError('push final artifact bytes changed')
    if source is None:
        if _latest_push(repository,branch,execution)!=run:raise ValueError('push final latest request identity changed')
    elif _source_run(repository,source_number,head,branch,app)!=source:
        raise ValueError('source final canonical PR/commits/latest/check identity changed')
    if obs.api(f'repos/{repository}/check-runs/{identity["check_run_id"]}')!=check:
        raise ValueError('push final check identity/provenance/success changed')
    if obs.pages(f'repos/{repository}/actions/runs/{run_id}/attempts/1/jobs','jobs')!=jobs:
        raise ValueError('push final exact-attempt job/step execution changed')
    if obs.api(f'repos/{repository}/check-suites/{run["check_suite_id"]}')!=suite:
        raise ValueError('push final check suite provenance changed')
    if source is not None and _source_execution(repository,source_number,head,base,run,job,root)!=workflow_revision:
        raise ValueError('source final workflow execution provenance changed')
    return _record('current_target_ci',repository,uid,head,_evidence=_primary(f'repos/{repository}/actions/artifacts/{artifact["id"]}/zip','repository_artifact'),target_oid=execution,
        target_tree_oid=obs.git(root,'rev-parse',execution+'^{tree}').decode().strip(),default_branch=branch,
        workflow_path='.github/workflows/rust.yml',workflow_sha=workflow_revision,event=event,plan=payload,
        plan_sha256=obs.digest(raw),policy_sha256=obs.digest(obs.canonical(policy)),checks=[normalized_check],coverage=coverage)

def verify_current_target(repository,uid,number,head,merge,target,context):
    with obs.observation():
        root,app,review,issue=review_context(repository,uid,number,head,context)
        branch,pr=merged_facts(repository,uid,number,head,merge,target,root,issue)
        parent=obs.git(root,'rev-parse',merge+'^').decode().strip()
        if obs.git(root,'diff','--binary',context['source_scope_oid'],head)!=obs.git(root,'diff','--binary',parent,merge):
            raise ValueError('current target accepted source patch equivalence mismatch')
        result=push_observation(repository,uid,head,target,branch,root,merge,app)
        if (obs.api(f'repos/{repository}/git/ref/heads/{branch}').get('object') or {}).get('sha')!=target:
            raise ValueError('current target final identity moved')
        return result

def collect_recovery(root,uid):
    """Read actual delivery facts without publishing or changing task truth."""
    with obs.observation():
        root=Path(root).resolve(strict=True)
        mapping=obs.load((root/'.pm/github-project-sync/tasks.json').read_bytes())
        record=(mapping.get('tasks') or {}).get(uid)
        if not isinstance(record,dict) or record.get('task_uid')!=uid:
            raise ValueError('recovery canonical Task UID binding unavailable')
        repository=record.get('repository');number=obs.positive(record.get('pr_number'),'PR')
        issue=obs.positive(record.get('issue_number'),'Issue')
        if not isinstance(repository,str) or not re.fullmatch(r'[^/\s]+/[^/\s]+',repository):
            raise ValueError('recovery canonical repository malformed')
        pr=obs.api(f'repos/{repository}/pulls/{number}')
        head=_oid((pr.get('head') or {}).get('sha'),'accepted head')
        merge=_oid(pr.get('merge_commit_sha'),'merged commit')
        comments=obs.pages(f'repos/{repository}/issues/{issue}/comments')
        receipt_root=_module('readiness_transport').receipt_root(root,uid)
        prior=_prior_readiness(receipt_root,comments)
        # The distinct consistency entry preserves typed existing terminal
        # evidence without recursively invoking the generic fresh reader.
        _module('terminal_proof').validate_existing_terminal_namespace(root,uid)
        authorization=_authorization(repository,uid,issue,number,head,merge,comments)
        repo=obs.api(f'repos/{repository}');branch=repo.get('default_branch')
        if not isinstance(branch,str):raise ValueError('recovery canonical default branch unavailable')
        target=_oid((obs.api(f'repos/{repository}/git/ref/heads/{branch}').get('object') or {}).get('sha'),'current target')
        policy=_module('pr-lifecycle-gate')
        class Client:
            def rest(self,method,path,**kwargs):
                if method!='GET':raise ValueError('recovery policy observation is read only')
                return obs.api(path)
        actual_policy=policy.discover_required_policy(repository,branch,client=Client())
        app=_required_gate_app(actual_policy)
        context,validated=_canonical_context(root,uid,repository,number,head,app)
        _,_,review,_=validated;plan=review['plan'];handoff=review['handoff'];scope=context['source_scope_oid']
        branch,pr=merged_facts(repository,uid,number,head,merge,target,root,issue)
        if record.get('pr_url')!=pr.get('html_url') or not isinstance(record.get('canonical_worktree'),str) or Path(record['canonical_worktree']).resolve(strict=True)!=root:
            raise ValueError('recovery canonical PR/worktree binding mismatch')
        source_review=_record('source_review',repository,uid,head,_evidence=_primary(f'repos/{repository}/issues/comments/{handoff["dispatch_evidence"]["comment_id"]}'),plan=plan,handoff=handoff,
            plan_sha256=obs.digest(Path(context['source_review_plan_path']).read_bytes()),
            handoff_sha256=obs.digest(Path(context['source_review_handoff_path']).read_bytes()))
        source_ci=source_observation(repository,uid,head,number,branch,root,app)
        required_ci=_record('required_ci',repository,uid,head,_evidence=_primary(f'repos/{repository}/git/commits/{source_ci["workflow_sha"]}'),default_branch=branch,
            policy_sha256=source_ci['policy_sha256'],checks=source_ci['checks'])
        target_ci=verify_current_target(repository,uid,number,head,merge,target,context)
        import integration_ci
        history=obs.pages(f'repos/{repository}/actions/workflows/rust.yml/runs?event=workflow_dispatch','workflow_runs')
        requests=[]
        for run in history:
            parts=str(run.get('display_title') or '').split('|')
            if len(parts) in (7,8) and parts[:5]==['oasis7-ci','workflow_dispatch','integration_revalidation',uid,str(number)] and parts[6]==head:
                requests.append((obs.instant(run.get('created_at')),obs.positive(run.get('id'),'integration request'),parts))
        if not requests:raise ValueError('recovery accepted source integration request unavailable')
        selected_parts=max(requests,key=lambda row:row[:2])[2]
        if len(selected_parts)!=7:raise ValueError('recovery keyed integration capability unsupported/disabled')
        base=_oid(selected_parts[5],'accepted integration base')
        merged_context={**context,'merge_commit_oid':merge,'observed_target_oid':target,'request_key':None}
        original=integration_ci._merged_delivery_integration(repository,uid,number,head,base,None,
            merged_context,_observe_current_target=True)
        integration=_record('integration',repository,uid,head,_evidence=_integration_primary(repository,original['run_id']),**original)
        parent=obs.git(root,'rev-parse',merge+'^').decode().strip()
        source_patch=obs.git(root,'diff','--binary',scope,head)
        merged_patch=obs.git(root,'diff','--binary',parent,merge)
        if source_patch!=merged_patch:raise ValueError('recovery accepted source patch equivalence mismatch')
        commit=obs.api(f'repos/{repository}/git/commits/{merge}')
        if (not isinstance(commit,dict) or commit.get('sha')!=merge or not isinstance(commit.get('tree'),dict)
                or commit['tree'].get('sha')!=obs.git(root,'rev-parse',merge+'^{tree}').decode().strip()
                or not isinstance(commit.get('parents'),list) or [p.get('sha') for p in commit['parents'] if isinstance(p,dict)]!=[parent]):
            raise ValueError('recovery merge commit primary contradicts local equivalence')
        equivalence=_record('source_equivalence',repository,uid,head,_evidence=_primary(f'repos/{repository}/git/commits/{merge}'),merge_commit_oid=merge,
            source_base_oid=scope,merge_parent_oid=parent,
            source_tree_oid=obs.git(root,'rev-parse',head+'^{tree}').decode().strip(),
            merged_tree_oid=obs.git(root,'rev-parse',merge+'^{tree}').decode().strip(),
            source_patch_sha256=obs.digest(source_patch),merged_patch_sha256=obs.digest(merged_patch),method='patch_equivalence')
        related=_paths(root,merge,target)
        # Distinct current-target verification is supported by the complete,
        # authenticated target plan ZIP. Configuration is still independently
        # collected and replayed above; the artifact never replaces that trust.
        applicability_primary=(target_ci['evidence'] if target!=merge else
            _primary(target+':scripts/ci-required-scope.v2.json','git_object'))
        applicability=_record('applicability',repository,uid,head,_evidence=applicability_primary,scope_base_oid=merge,target_oid=target,
            policy_oid=target,policy_sha256=target_ci['policy_sha256'],projection=plan['impact_projection'],
            projection_digest=plan['impact_projection']['projection_digest'],requires_strict_integration=True,
            related_paths=related,disposition='unchanged_target' if target==merge else 'current_target_verified')
        merge_readback=_record('merge_readback',repository,uid,head,_evidence=_merge_primary(repository,number,head,merge,target,branch,pr),pr_number=number,pr_url=pr['html_url'],
            merge_commit_oid=merge,merged_at=pr['merged_at'],default_branch=branch,target_oid=target,
            comparison_base_oid=merge,comparison_head_oid=target,comparison_merge_base_oid=merge)
        holds=_current_holds(root,repository,uid,issue,number,head,record,comments)
        source=_module('readiness_transport').source_identity(root)
        if (obs.api(f'repos/{repository}/git/ref/heads/{branch}').get('object') or {}).get('sha')!=target:
            raise ValueError('recovery current target moved at collection barrier')
        return {'schema':'oasis7-postmerge-delivery-proof/v1','mode':'postmerge_recovery','purpose':'delivery_only',
            'repository':repository,'task_uid':uid,'issue_number':issue,'pr_number':number,'pr_url':pr['html_url'],
            'head_oid':head,'merge_commit_oid':merge,'merged_at':pr['merged_at'],'default_branch':branch,
            'observed_target_oid':target,'observed_at':obs.now(),'source':source,'authorization_comment':authorization,
            'prior_readiness':prior,'verification':{'source_review':source_review,'required_ci':required_ci,
                'integration':integration,'applicability':applicability,'source_equivalence':equivalence,
                'merge_readback':merge_readback,'current_holds':holds,'current_target_ci':target_ci}}

def _validate_stored_proof_shape(root,uid,proof):
    obs.closed(proof,PROOF_FIELDS,'recovery proof')
    obs.closed(proof['verification'],RECORD_FIELDS,'recovery verification')
    if proof['schema']!='oasis7-postmerge-delivery-proof/v1' or proof['mode']!='postmerge_recovery' or proof['purpose']!='delivery_only':
        raise ValueError('recovery proof schema/purpose unavailable')
    if proof['task_uid']!=uid:raise ValueError('recovery stored Task UID mismatch')
    for name in ('head_oid','merge_commit_oid','observed_target_oid'):_oid(proof[name],name)
    for name in ('issue_number','pr_number'):obs.positive(proof[name],name)
    prior=obs.closed(proof['prior_readiness'],{'status','reason','native_marker_count','migration_marker_count','canonical_artifact_count'},'recovery prior readiness')
    if (prior['status']!='unavailable' or not isinstance(prior['reason'],str) or not prior['reason'].strip()
            or any(type(prior[k]) is not int or prior[k]!=0 for k in ('native_marker_count','migration_marker_count','canonical_artifact_count'))):
        raise ValueError('recovery prior readiness absence malformed')
    if obs.instant(proof['observed_at'])<obs.instant(proof['merged_at']):
        raise ValueError('recovery proof cannot claim premerge observation')
    for key,fields in RECORD_FIELDS.items():
        record=obs.closed(proof['verification'][key],COMMON_FIELDS|fields,'recovery '+key)
        if record['schema']!='oasis7-terminal-recovery-'+key+'/v1' or any(record[name]!=proof[name] for name in ('repository','task_uid','head_oid')):
            raise ValueError('recovery verification record identity mismatch')
        if obs.instant(record['observed_at'])<obs.instant(proof['merged_at']):
            raise ValueError('recovery verification observation predates authoritative merge: '+key)
        if not isinstance(record['evidence'],list) or not record['evidence']:
            raise ValueError('recovery verification evidence unavailable')
        for evidence in record['evidence']:
            obs.closed(evidence,{'kind','locator','raw_b64','raw_sha256'},'recovery evidence')
            if evidence['kind'] not in ('github_api','git_object','repository_artifact') or not isinstance(evidence['locator'],str) or not evidence['locator']:
                raise ValueError('recovery evidence provenance locator malformed')
            data=_module('readiness_transport').decode(evidence['raw_b64'])
            if obs.digest(data)!=evidence['raw_sha256']:raise ValueError('recovery evidence raw hash mismatch')
    _module('readiness_transport').validate_source(Path(root),proof['source'])
    return proof

def _validate_proof(root,uid,proof,fresh):
    _validate_stored_proof_shape(root,uid,proof)
    for key in PROOF_FIELDS-{'observed_at','verification'}:
        if obs.canonical(proof[key])!=obs.canonical(fresh[key]):raise ValueError('recovery proof current identity/authorization/source changed: '+key)
    for key,fields in RECORD_FIELDS.items():
        record=proof['verification'][key]
        observed=fresh['verification'][key]
        if obs.canonical(record['evidence'])!=obs.canonical(observed['evidence']):
            raise ValueError('recovery independently recollected '+key+' primary evidence changed')
        skipped={'observed_at','evidence'}|({'task_status','workflow_phase'} if key=='current_holds' else set())
        for name in (COMMON_FIELDS|fields)-skipped:
            if obs.canonical(record[name])!=obs.canonical(observed[name]):raise ValueError('recovery actual '+key+' factual provenance changed: '+name)
    _module('readiness_transport').validate_source(Path(root),proof['source'])
    return proof

def _marker_comment(repository,issue,comments,marker,value):
    selected=[comment for comment in comments if marker in str(comment.get('body') or '')]
    if len(selected)!=1:raise ValueError('recovery publication marker missing/ambiguous')
    readiness=_module('readiness_transport');capture=readiness.comment_capture(selected[0])
    readiness.validate_comment(repository,issue,capture,comments,marker.encode()+b'\n'+obs.canonical(value),marker,
        admin=True,check_permission=True)
    return capture

def _ordinary_completion(root,uid,proof,comments):
    mapping=obs.load((root/'.pm/github-project-sync/tasks.json').read_bytes());record=mapping['tasks'][uid]
    issue=obs.api(f"repos/{proof['repository']}/issues/{proof['issue_number']}")
    helper=_module('task_complete_claim');history=record.get('claim_verifications')
    if history is not None and (not isinstance(history,list) or any(not isinstance(item,dict) for item in history)):
        raise ValueError('recovery ordinary claim history malformed')
    body=issue.get('body')
    live_history=helper.issue_claim_history(body) if isinstance(body,str) and 'claim_verifications_b64:' in body else []
    candidates=[claim for claim in (history or [])+live_history if claim.get('claim_type')=='task_complete']
    marked=[c for c in comments if '<!-- oasis7-pm-claim-verification -->' in str(c.get('body') or '')
        and re.search(r'(?im)^Claim Type:\s*task_complete\s*$',str(c.get('body') or ''))]
    if not candidates and not marked:return None
    claim,hashed,comment=helper.select_historical_task_complete_claim(proof['repository'],uid,record,issue,comments,accepted_head=proof['head_oid'])
    return claim,hashed.removeprefix('sha256:'),_module('readiness_transport').comment_capture(comment)

def _validate_completion(completion,proof):
    obs.closed(completion,COMPLETION_FIELDS,'recovery current completion')
    expected={name:proof[name] for name in ('task_uid','repository','issue_number','pr_number',
        'pr_url','head_oid','merge_commit_oid','observed_target_oid','source')}
    expected.update(schema='oasis7-postmerge-completion/v1',claim_type='postmerge_delivery_complete',
        status='verified',exit_code=0,recovery_proof_sha256=obs.digest(obs.canonical(proof)),
        authorization_comment_id=proof['authorization_comment']['id'])
    if any(obs.canonical(completion[name])!=obs.canonical(value) for name,value in expected.items()):
        raise ValueError('recovery current completion exact binding mismatch')
    if obs.instant(completion['verified_at'])<obs.instant(proof['merged_at']):
        raise ValueError('recovery completion premerge claim forbidden')

def _published_candidate(repository,issue,comments,marker):
    candidates=[comment for comment in comments if marker in str(comment.get('body') or '')]
    if not candidates:return None
    if len(candidates)!=1:raise ValueError('recovery publication candidate duplicate/ambiguous')
    body=candidates[0].get('body')
    if not isinstance(body,str) or not body.startswith(marker+'\n'):
        raise ValueError('recovery publication candidate marker malformed')
    value=obs.load(body[len(marker)+1:].encode())
    captured=_marker_comment(repository,issue,comments,marker,value)
    if isinstance(value,dict):
        stamp={'oasis7-postmerge-delivery-proof/v1':'observed_at',
            'oasis7-postmerge-completion/v1':'verified_at'}.get(value.get('schema'))
        if stamp is not None and obs.instant(captured['created_at'])<obs.instant(value.get(stamp)):
            raise ValueError('recovery publication precedes bound observation')
    return value

def _read_stored_recovery(root,uid):
    """Authenticate stored bindings; this phase grants no fresh CI verdict.

    Resolve all inputs here rather than accepting caller state. The distinct
    terminal namespace reader uses this nonrecursive phase; public/effect
    readers additionally recollect every factual verification record.
    """
    with obs.observation():
        root=Path(root).resolve(strict=True);readiness=_module('readiness_transport')
        receipt_root=readiness.receipt_root(root,uid);path=receipt_root/'terminal-recovery-proof.json'
        if not path.is_file():raise ValueError('explicit recovery proof candidate unavailable')
        raw=path.read_bytes();proof=obs.load(raw)
        if raw!=obs.canonical(proof):raise ValueError('recovery proof immutable canonical bytes mismatch')
        _validate_stored_proof_shape(root,uid,proof)
        mapping=obs.load((root/'.pm/github-project-sync/tasks.json').read_bytes())
        record=(mapping.get('tasks') or {}).get(uid)
        if (not isinstance(record,dict) or record.get('task_uid')!=uid
                or any(record.get(k)!=proof[k] for k in ('repository','issue_number','pr_number','pr_url'))
                or not isinstance(record.get('canonical_worktree'),str)
                or Path(record['canonical_worktree']).resolve(strict=True)!=root):
            raise ValueError('recovery stored canonical Task binding mismatch')
        branch,live=merged_facts(proof['repository'],uid,proof['pr_number'],proof['head_oid'],
            proof['merge_commit_oid'],proof['observed_target_oid'],root,proof['issue_number'])
        if branch!=proof['default_branch'] or live.get('merged_at')!=proof['merged_at'] or live.get('html_url')!=proof['pr_url']:
            raise ValueError('recovery stored reciprocal merged binding changed')
        comments=obs.pages(f"repos/{proof['repository']}/issues/{proof['issue_number']}/comments")
        if obs.canonical(_prior_readiness(receipt_root,comments))!=obs.canonical(proof['prior_readiness']):
            raise ValueError('recovery stored native/migration absence changed')
        authorization=_authorization(proof['repository'],uid,proof['issue_number'],proof['pr_number'],
            proof['head_oid'],proof['merge_commit_oid'],comments)
        if obs.canonical(authorization)!=obs.canonical(proof['authorization_comment']):
            raise ValueError('recovery stored authorization changed')
        proof_comment=_marker_comment(proof['repository'],proof['issue_number'],comments,
            '<!-- oasis7-postmerge-delivery-proof/v1 -->',proof)
        if obs.instant(proof_comment['created_at'])<obs.instant(proof['observed_at']):
            raise ValueError('recovery proof publication time mismatch')
        completion_path=receipt_root/'terminal-recovery-completion.json'
        marker='<!-- oasis7-postmerge-completion/v1 -->'
        current=[c for c in comments if marker in str(c.get('body') or '')]
        ordinary=_ordinary_completion(root,uid,proof,comments)
        if ordinary is not None:
            if current or completion_path.exists():raise ValueError('mixed ordinary/current recovery completion candidates')
            completion,completion_digest,completion_comment=ordinary
        else:
            if not completion_path.is_file():raise ValueError('recovery current completion candidate unavailable')
            claim_raw=completion_path.read_bytes();completion=obs.load(claim_raw)
            if claim_raw!=obs.canonical(completion):raise ValueError('recovery completion immutable bytes mismatch')
            _validate_completion(completion,proof)
            completion_comment=_marker_comment(proof['repository'],proof['issue_number'],comments,marker,completion)
            if obs.instant(completion_comment['created_at'])<obs.instant(completion['verified_at']):raise ValueError('recovery completion publication time mismatch')
            completion_digest=obs.digest(claim_raw)
        return {'proof':proof,'digest':obs.digest(raw),'completion':completion,'completion_digest':completion_digest,
            'proof_comment':proof_comment,'completion_comment':completion_comment,'receipt_root':receipt_root}

def validate_recovery(root,uid):
    """Validate explicit published recovery with independent fresh records."""
    with obs.observation():
        stored=_read_stored_recovery(root,uid)
        fresh=collect_recovery(root,uid)
        _validate_proof(root,uid,stored['proof'],fresh)
        return stored

def _publish_comment(repository,issue,marker,value):
    body=marker+'\n'+obs.canonical(value).decode()
    if len(body)>65536:
        raise ValueError('recovery exact comment exceeds GitHub 65536-character publication bound')
    with tempfile.NamedTemporaryFile('w',encoding='utf-8',delete=False) as handle:
        handle.write(body);path=Path(handle.name)
    try:
        obs.capture(['gh','issue','comment',str(issue),'-R',repository,'--body-file',str(path)])
    finally:path.unlink(missing_ok=True)
    comments=obs.pages(f'repos/{repository}/issues/{issue}/comments')
    return _marker_comment(repository,issue,comments,marker,value)

def _publication_barrier(root,uid,proof):
    # A previous check or a persisted proof does not authorize the next effect.
    # In particular, recollect after proof POST before completion publication.
    fresh=collect_recovery(root,uid)
    _validate_proof(root,uid,proof,fresh)
    repository=proof['repository'];issue=proof['issue_number']
    _,live=merged_facts(repository,uid,proof['pr_number'],proof['head_oid'],
        proof['merge_commit_oid'],proof['observed_target_oid'],root,issue)
    if live.get('merged_at')!=proof['merged_at']:
        raise ValueError('recovery publication merged timestamp changed')
    if (obs.api(f'repos/{repository}/git/ref/heads/{proof["default_branch"]}').get('object') or {}).get('sha')!=proof['observed_target_oid']:
        raise ValueError('recovery publication target moved')
    comments=obs.pages(f'repos/{repository}/issues/{issue}/comments')
    _authorization(repository,uid,issue,proof['pr_number'],proof['head_oid'],proof['merge_commit_oid'],comments)
    mapping=obs.load((root/'.pm/github-project-sync/tasks.json').read_bytes())
    _current_holds(root,repository,uid,issue,proof['pr_number'],proof['head_oid'],mapping['tasks'][uid],comments)
    if obs.canonical(_module('readiness_transport').source_identity(root))!=obs.canonical(proof['source']):
        raise ValueError('recovery publication source authority changed')

def publish_recovery(root,uid):
    """Publish only after actual complete collection and a fresh live barrier."""
    with obs.observation():
        root=Path(root).resolve(strict=True);readiness=_module('readiness_transport')
        prospective=collect_recovery(root,uid);receipt_root=readiness.receipt_root(root,uid)
        repository=prospective['repository'];issue=prospective['issue_number']
        comments=obs.pages(f'repos/{repository}/issues/{issue}/comments')
        marker='<!-- oasis7-postmerge-delivery-proof/v1 -->'
        published=_published_candidate(repository,issue,comments,marker)
        proof_path=receipt_root/'terminal-recovery-proof.json'
        if proof_path.exists():
            proof_raw=proof_path.read_bytes();proof=obs.load(proof_raw)
            if proof_raw!=obs.canonical(proof):raise ValueError('recovery proof immutable canonical bytes mismatch')
            if published is not None and obs.canonical(published)!=proof_raw:
                raise ValueError('recovery local/server proof candidates conflict')
        else:proof=published if published is not None else prospective
        fresh=collect_recovery(root,uid);_validate_proof(root,uid,proof,fresh)
        if len(marker+'\n'+obs.canonical(proof).decode())>65536:
            raise ValueError('recovery exact closed proof exceeds GitHub 65536-character publication bound; no publication effects')
        repository=proof['repository'];issue=proof['issue_number']
        comments=obs.pages(f'repos/{repository}/issues/{issue}/comments')
        existing=[c for c in comments if marker in str(c.get('body') or '')]
        claim_path=receipt_root/'terminal-recovery-completion.json';claim_marker='<!-- oasis7-postmerge-completion/v1 -->'
        published_claim=_published_candidate(repository,issue,comments,claim_marker)
        claim=None
        if claim_path.exists():
            claim_raw=claim_path.read_bytes();claim=obs.load(claim_raw)
            if claim_raw!=obs.canonical(claim):raise ValueError('recovery completion immutable bytes mismatch')
            if published_claim is not None and obs.canonical(published_claim)!=claim_raw:
                raise ValueError('recovery local/server completion candidates conflict')
        elif published_claim is not None:claim=published_claim
        if claim is not None:_validate_completion(claim,proof)
        ordinary=_ordinary_completion(root,uid,proof,comments)
        if ordinary is not None and claim is not None:
            raise ValueError('mixed ordinary/current recovery completion candidates')
        proof_comment=_marker_comment(repository,issue,comments,marker,proof) if existing else None
        _publication_barrier(root,uid,proof)
        readiness._store_once(proof_path,obs.canonical(proof))
        if proof_comment is None:
            _publication_barrier(root,uid,proof)
            proof_comment=_publish_comment(repository,issue,marker,proof)
        if ordinary is not None:
            verified=validate_recovery(root,uid)
            return {'schema':'oasis7-postmerge-recovery-publication/v1','task_uid':uid,
                'recovery_proof_sha256':verified['digest'],'recovery_proof_comment_id':proof_comment['id'],
                'completion_claim_sha256':verified['completion_digest'],'completion_comment_id':verified['completion_comment']['id']}
        comments=obs.pages(f'repos/{repository}/issues/{issue}/comments')
        claims=[c for c in comments if claim_marker in str(c.get('body') or '')]
        if claim is None:
            claim={name:proof[name] for name in ('task_uid','repository','issue_number','pr_number','pr_url','head_oid','merge_commit_oid','observed_target_oid','source')}
            claim.update(schema='oasis7-postmerge-completion/v1',claim_type='postmerge_delivery_complete',status='verified',exit_code=0,
                verified_at=obs.now(),recovery_proof_sha256=obs.digest(obs.canonical(proof)),authorization_comment_id=proof['authorization_comment']['id'])
        _validate_completion(claim,proof)
        # Repeat current server identities/admin/holds after proof publication;
        # no marker or prior local capture authorizes a later changed target.
        _publication_barrier(root,uid,proof)
        completion_comment=_marker_comment(repository,issue,comments,claim_marker,claim) if claims else _publish_comment(repository,issue,claim_marker,claim)
        _publication_barrier(root,uid,proof)
        readiness._store_once(claim_path,obs.canonical(claim))
        verified=validate_recovery(root,uid)
        return {'schema':'oasis7-postmerge-recovery-publication/v1','task_uid':uid,
            'recovery_proof_sha256':verified['digest'],'recovery_proof_comment_id':proof_comment['id'],
            'completion_claim_sha256':verified['completion_digest'],'completion_comment_id':completion_comment['id']}
