"""Read selected merged-terminal delivery using the actual finalizer projection.

Issue body phase is not terminal authority: post-merge-finalize.py projects
Done/done/done, writes its deterministic evidence comment, then closes the Issue.
"""
import hashlib
import json
import pathlib
import re
import subprocess

from terminal_proof import read_receipt_chain, receipt_chain_digest, validate_receipt_chain


def _json(*args):
    return json.loads(subprocess.check_output(['gh',*args],text=True,timeout=180))


def read_issue(repository, number):
    return _json('api',f'repos/{repository}/issues/{number}')


def read_pull_request(repository, number):
    return _json('api',f'repos/{repository}/pulls/{number}')


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


def _single_markdown_field(body, key):
    fields = re.findall(rf'^- {re.escape(key)}: `([^`]+)`$', str(body or '').replace('\r\n', '\n'), re.MULTILINE)
    return fields[0] if len(fields) == 1 else None


def _single_plain_field(body, key):
    fields = re.findall(rf'^{re.escape(key)}: ([^\n]+)$', str(body or '').replace('\r\n', '\n'), re.MULTILINE)
    return fields[0] if len(fields) == 1 else None


def _receipt_comment_matches(comment, issue_url, repository, task_uid, issue_number, pr_number, pr_url,
                             receipt_digests):
    body = str(comment.get('body') or '')
    comment_url = str(comment.get('html_url') or '')
    if not re.fullmatch(re.escape(issue_url)+r'#issuecomment-\d+', comment_url):
        return False
    if '<!-- oasis7-pm-evidence -->' not in body:
        return False
    if (comment.get('user') or {}).get('login') != repository.split('/')[0]:
        raise ValueError('terminal evidence comment author is not the repository owner')
    expected = {
        'Task UID': task_uid,
        'Evidence Phase': 'post_merge_done',
        'Receipt Chain Version': '1',
        'Receipt Type': 'oasis7_terminal_cleanup',
        'Receipt Issuer': 'post-merge-cleanup',
        'PR Number': str(pr_number),
        'PR URL': pr_url,
    }
    for key, value in expected.items():
        if _single_plain_field(body, key) != value:
            return False
    comment_digests = {}
    for key in ('Merge Receipt SHA256', 'Main Sync Receipt SHA256', 'Terminal Receipt SHA256'):
        value = _single_plain_field(body, key)
        if not re.fullmatch(r'[0-9a-f]{64}', str(value or '')):
            return False
        comment_digests[key] = value
    if any(_single_plain_field(body, label) != value for label, value in {
        'Merge Receipt SHA256': receipt_digests['merge'],
        'Main Sync Receipt SHA256': receipt_digests['main_sync'],
        'Terminal Receipt SHA256': receipt_digests['terminal'],
    }.items()):
        return False
    if _single_plain_field(body, 'Receipt Chain Digest') != receipt_chain_digest(
        task_uid,
        repository,
        issue_number,
        pr_number,
        pr_url,
        comment_digests['Merge Receipt SHA256'],
        comment_digests['Main Sync Receipt SHA256'],
        comment_digests['Terminal Receipt SHA256'],
    ):
        return False
    operation = hashlib.sha256(f'{task_uid}:post_merge_done:evidence_comment'.encode()).hexdigest()
    return _single_plain_field(body, 'Operation-ID') == operation


def validate_terminal_delivery(repository,task_uid,issue_number,issue_reader=None,project_reader=None,
                               comments_reader=None,pr_reader=None,receipt_reader=None,repo_root=None):
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
        pr_number_text = _single_markdown_field(issue.get('body'), 'pr_number')
        pr_url = _single_markdown_field(issue.get('body'), 'pr_url')
        if not pr_number_text or not pr_url or not re.fullmatch(r'[1-9]\d*', pr_number_text):
            raise ValueError('terminal task has no canonical PR binding')
        pr_number = int(pr_number_text)
        expected_pr_url = f'https://github.com/{repository}/pull/{pr_number}'
        if pr_url != expected_pr_url:
            raise ValueError('terminal task PR URL identity mismatch')
        pr = (pr_reader or read_pull_request)(repository, pr_number)
        base_repo = ((pr.get('base') or {}).get('repo') or {}).get('full_name')
        head_repo = ((pr.get('head') or {}).get('repo') or {}).get('full_name')
        if (pr.get('number') != pr_number or pr.get('html_url') != expected_pr_url
                or base_repo != repository or head_repo != repository):
            raise ValueError('terminal PR reciprocal repository/number identity mismatch')
        pr_body = str(pr.get('body') or '').replace('\r\n', '\n')
        if re.findall(r'^Task: [^\n]+$', pr_body, re.MULTILINE) != ['Task: ' + task_uid]:
            raise ValueError('terminal PR task UID identity mismatch')
        if re.findall(r'^Refs #[1-9]\d*$', pr_body, re.MULTILINE) != [f'Refs #{issue_number}']:
            raise ValueError('terminal PR task Issue reference mismatch')
        if (str(pr.get('state','')).upper() != 'CLOSED' or pr.get('merged') is not True
                or not pr.get('merged_at') or not re.fullmatch(r'[0-9a-f]{40}', str(pr.get('merge_commit_sha') or ''))):
            raise ValueError('terminal PR is not verified merged with a merge commit')
        receipts = (receipt_reader or read_receipt_chain)(repo_root or pathlib.Path.cwd(), task_uid)
        receipt_digests = validate_receipt_chain(receipts, repository=repository, task_uid=task_uid,
                                                 issue_number=issue_number, pr_number=pr_number,
                                                 pr_url=pr_url, pr=pr)
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
        matches=[]
        for comment in (comments_reader or read_comments)(repository,issue_number):
            if _receipt_comment_matches(comment, url, repository, task_uid, issue_number, pr_number, pr_url,
                                        receipt_digests):
                matches.append(str(comment.get('html_url') or ''))
        if len(matches)!=1: raise ValueError('unique canonical post-merge terminal evidence unavailable')
        return {'status':'passed','blockers':[],'task_uid':task_uid,'issue_url':url,
                'project_item_id':item['id'],'terminal_evidence':matches[0]}
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as exc:
        blockers.append(str(exc))
    return {'status':'blocked','blockers':blockers,'task_uid':task_uid}
