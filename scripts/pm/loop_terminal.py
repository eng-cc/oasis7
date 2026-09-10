"""Read selected merged-terminal delivery using the actual finalizer projection.

Issue body phase is not terminal authority: post-merge-finalize.py projects
Done/done/done, writes its deterministic evidence comment, then closes the Issue.
"""
import hashlib
import json
import re
import subprocess


def _json(*args):
    return json.loads(subprocess.check_output(['gh',*args],text=True,timeout=180))


def read_issue(repository, number):
    return _json('api',f'repos/{repository}/issues/{number}')


def read_project(repository, number):
    owner,name=repository.split('/')
    canonical=_json('project','view','1','--owner',owner,'--format','json')
    query='''query($owner:String!,$name:String!,$number:Int!) {
      repository(owner:$owner,name:$name) { issue(number:$number) {
        projectItems(first:100) { pageInfo { hasNextPage } nodes {
          id project { id number owner { ... on User { login } ... on Organization { login } } }
          content { ... on Issue { number url body } }
          fieldValues(first:100) { pageInfo { hasNextPage } nodes {
            ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
          } }
        } }
      } }
    }'''
    raw=_json('api','graphql','-f','query='+query,'-f','owner='+owner,'-f','name='+name,'-F','number='+str(number))
    if raw.get('errors'): raise ValueError('selected terminal Project query failed')
    items=raw['data']['repository']['issue']['projectItems']
    return {'id':canonical['id'],'owner':owner,'number':1,
            'page_complete':items.get('pageInfo',{}).get('hasNextPage') is False,'items':items.get('nodes',[])}


def read_comments(repository, number):
    pages=_json('api',f'repos/{repository}/issues/{number}/comments','--paginate','--slurp')
    if not isinstance(pages,list): raise ValueError('terminal evidence pagination malformed')
    return [item for page in pages for item in (page if isinstance(page,list) else [page])]


def has_unique_task_uid(body, task_uid):
    fields = re.findall(r'^task_uid:[^\n]*$', str(body or '').replace('\r\n', '\n'), re.MULTILINE)
    return fields == ['task_uid: ' + task_uid]


def validate_terminal_delivery(repository,task_uid,issue_number,issue_reader=None,project_reader=None,comments_reader=None):
    blockers=[]
    try:
        if not re.fullmatch(r'[^/\s]+/[^/\s]+',repository) or not re.fullmatch(r'task_[0-9a-f]{32}',task_uid) or type(issue_number) is not int or issue_number<1:
            raise ValueError('invalid selected terminal identity')
        url=f'https://github.com/{repository}/issues/{issue_number}'
        issue=(issue_reader or read_issue)(repository,issue_number)
        if (issue.get('number')!=issue_number or issue.get('html_url',issue.get('url'))!=url
                or not has_unique_task_uid(issue.get('body'),task_uid)):
            raise ValueError('terminal Issue identity mismatch')
        if str(issue.get('state','')).lower()!='closed' or str(issue.get('state_reason',issue.get('stateReason',''))).lower()!='completed':
            raise ValueError('delivery Issue is not closed as completed')
        project=(project_reader or read_project)(repository,issue_number)
        if (not project.get('id') or project.get('owner')!=repository.split('/')[0]
                or project.get('number')!=1 or project.get('page_complete') is not True):
            raise ValueError('canonical terminal Project identity/pagination unavailable')
        matches=[item for item in project.get('items',[]) if (item.get('project') or {}).get('id')==project['id']]
        if len(matches)!=1: raise ValueError('selected terminal Project item missing or ambiguous')
        item=matches[0]; context=item.get('project') or {}; content=item.get('content') or {}
        if (not item.get('id') or context.get('number')!=1 or (context.get('owner') or {}).get('login')!=project['owner']
                or content.get('number')!=issue_number or content.get('url')!=url
                or not has_unique_task_uid(content.get('body'),task_uid)):
            raise ValueError('terminal Project item content identity mismatch')
        values=item.get('fieldValues') or {}
        if (values.get('pageInfo') or {}).get('hasNextPage') is not False:
            raise ValueError('terminal Project field pagination incomplete')
        fields={}
        for value in values.get('nodes',[]):
            name=(value.get('field') or {}).get('name')
            if name in fields: raise ValueError('duplicate terminal Project field')
            fields[name]=value.get('name',value.get('text',''))
        if any(fields.get(k)!=v for k,v in {'Status':'Done','PM Status':'done','Workflow Phase':'done'}.items()):
            raise ValueError('terminal Project delivery projection is incomplete')
        if fields.get('Task UID',task_uid)!=task_uid:
            raise ValueError('terminal Project Task UID mismatch')
        operation=hashlib.sha256(f'{task_uid}:post_merge_done:evidence_comment'.encode()).hexdigest()
        matches=[]
        for comment in (comments_reader or read_comments)(repository,issue_number):
            body=str(comment.get('body') or '')
            comment_url=str(comment.get('html_url') or '')
            if (re.fullmatch(re.escape(url)+r'#issuecomment-\d+',comment_url)
                    and '<!-- oasis7-pm-evidence -->' in body
                    and re.search(rf'^Operation-ID: {operation}$',body,re.MULTILINE)
                    and re.search(rf'^Task UID: {re.escape(task_uid)}$',body,re.MULTILINE)
                    and re.search(r'^Evidence Phase: post_merge_done$',body,re.MULTILINE)):
                matches.append(comment_url)
        if len(matches)!=1: raise ValueError('unique canonical post-merge terminal evidence unavailable')
        return {'status':'passed','blockers':[],'task_uid':task_uid,'issue_url':url,
                'project_item_id':item['id'],'terminal_evidence':matches[0]}
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as exc:
        blockers.append(str(exc))
    return {'status':'blocked','blockers':blockers,'task_uid':task_uid}
