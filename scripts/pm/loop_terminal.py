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


def normalize_project_fields(item, repository):
    """Validate the requested official union; never infer a missing type."""
    if not isinstance(repository,str) or not re.fullmatch(r'[^/\s]+/[^/\s]+',repository):
        raise ValueError('terminal Repository identity malformed')
    if not isinstance(item,dict) or not isinstance(item.get('id'),str) or not item['id']:
        raise ValueError('terminal Project item identity malformed')
    values=item.get('fieldValues')
    if (not isinstance(values,dict) or not isinstance(values.get('pageInfo'),dict)
            or values['pageInfo'].get('hasNextPage') is not False
            or not isinstance(values.get('nodes'),list)):
        raise ValueError('terminal Project field pagination/container incomplete')
    fields={}
    select_fields={'Status','PM Status','Workflow Phase'}
    text_fields={'Task UID','Canonical Worktree'}
    for value in values['nodes']:
        if not isinstance(value,dict) or not isinstance(value.get('field'),dict):
            raise ValueError('terminal Project field node malformed')
        name=value['field'].get('name')
        if not isinstance(name,str) or not name or name in fields:
            raise ValueError('terminal Project field name missing or duplicated')
        kind=value.get('__typename')
        if kind=='ProjectV2ItemFieldRepositoryValue':
            metadata=value.get('repository')
            if (name!='Repository' or not isinstance(metadata,dict)
                    or metadata.get('nameWithOwner')!=repository):
                raise ValueError('terminal Project Repository metadata identity mismatch')
            scalar=metadata['nameWithOwner']
        elif kind in ('ProjectV2ItemFieldTextValue','ProjectV2ItemFieldSingleSelectValue'):
            key='text' if kind=='ProjectV2ItemFieldTextValue' else 'name'
            scalar=value.get(key)
            if (not isinstance(scalar,str) or name=='Repository'
                    or (name in select_fields and key!='name')
                    or (name in text_fields and key!='text')):
                raise ValueError('terminal Project scalar field type/value malformed')
        else:
            raise ValueError('terminal Project field union type unknown or missing')
        fields[name]=scalar
    return fields


def _validate_selected_project_item(item, repository, number, project_id):
    if not isinstance(item,dict):
        raise ValueError('terminal Project item malformed')
    context=item.get('project');content=item.get('content')
    if (not isinstance(context,dict) or context.get('id')!=project_id
            or type(context.get('number')) is not int or context['number']!=1
            or not isinstance(context.get('owner'),dict)
            or context['owner'].get('login')!=repository.split('/')[0]
            or not isinstance(content,dict) or type(content.get('number')) is not int
            or content['number']!=number
            or content.get('url')!=f'https://github.com/{repository}/issues/{number}'):
        raise ValueError('terminal Project item content identity mismatch')
    normalize_project_fields(item,repository)


def _project_items(raw):
    if not isinstance(raw,dict) or raw.get('errors'):
        raise ValueError('selected terminal Project query failed')
    data=raw.get('data')
    repository=data.get('repository') if isinstance(data,dict) else None
    issue=repository.get('issue') if isinstance(repository,dict) else None
    items=issue.get('projectItems') if isinstance(issue,dict) else None
    if (not isinstance(items,dict) or not isinstance(items.get('pageInfo'),dict)
            or items['pageInfo'].get('hasNextPage') is not False
            or not isinstance(items.get('nodes'),list)
            or any(not isinstance(item,dict) for item in items['nodes'])):
        raise ValueError('terminal Project item pagination/container incomplete')
    return items


def read_project(repository, number):
    owner,name=repository.split('/')
    canonical=_json('project','view','1','--owner',owner,'--format','json')
    query='''query($owner:String!,$name:String!,$number:Int!) {
      repository(owner:$owner,name:$name) { issue(number:$number) {
        projectItems(first:100) { pageInfo { hasNextPage } nodes {
          id project { id number owner { ... on User { login } ... on Organization { login } } }
          content { ... on Issue { number url body } }
          fieldValues(first:100) { pageInfo { hasNextPage } nodes {
            __typename
            ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldRepositoryValue { repository { nameWithOwner } field { ... on ProjectV2FieldCommon { name } } }
          } }
        } }
      } }
    }'''
    raw=_json('api','graphql','-f','query='+query,'-f','owner='+owner,'-f','name='+name,'-F','number='+str(number))
    items=_project_items(raw)
    if (not isinstance(canonical,dict) or not isinstance(canonical.get('id'),str) or not canonical['id']
            or not isinstance(items,dict) or not isinstance(items.get('pageInfo'),dict)
            or items['pageInfo'].get('hasNextPage') is not False or not isinstance(items.get('nodes'),list)
            or any(not isinstance(item,dict) for item in items['nodes'])):
        raise ValueError('terminal Project item pagination/identity incomplete')
    matches=[item for item in items['nodes'] if isinstance(item.get('project'),dict)
             and item['project'].get('id')==canonical['id']]
    if len(matches)!=1: raise ValueError('selected terminal Project item missing or ambiguous')
    _validate_selected_project_item(matches[0],repository,number,canonical['id'])
    return {'id':canonical['id'],'owner':owner,'number':1,
            'page_complete':items.get('pageInfo',{}).get('hasNextPage') is False,'items':items.get('nodes',[])}


def read_comments(repository, number):
    pages=_json('api',f'repos/{repository}/issues/{number}/comments','--paginate','--slurp')
    if not isinstance(pages,list): raise ValueError('terminal evidence pagination malformed')
    return [item for page in pages for item in (page if isinstance(page,list) else [page])]


def task_record(repo_root, task_uid):
    path = pathlib.Path(repo_root) / '.pm/github-project-sync/tasks.json'
    def unique_object(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError('canonical task mapping contains a duplicate JSON key')
            value[key] = item
        return value
    mapping = json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=unique_object)
    if not isinstance(mapping, dict) or not isinstance(mapping.get('tasks'), dict):
        raise ValueError('canonical task mapping is malformed')
    record = mapping['tasks'].get(task_uid)
    if not isinstance(record, dict) or record.get('task_uid') != task_uid:
        raise ValueError('canonical task record is missing or mismatched')
    return record


def _v2_selector_present(record):
    return any(key in record for key in (
        'phase_receipt_type', 'phase_receipt_comment_id', 'phase_receipt_comment_sha256',
    ))


def read_live_project_item(repository, issue_number):
    owner, name = repository.split('/')
    canonical = _json('project', 'view', '1', '--owner', owner, '--format', 'json')
    project_id = canonical.get('id') if isinstance(canonical, dict) else None
    if not isinstance(project_id,str) or not project_id:
        raise ValueError('canonical terminal Project identity is unavailable')
    query = '''query($owner:String!,$name:String!,$number:Int!) {
      repository(owner:$owner,name:$name) { issue(number:$number) {
        projectItems(first:100) { pageInfo { hasNextPage } nodes {
          id project { id number owner { ... on User { login } ... on Organization { login } } }
          content { ... on Issue { number url body } }
          fieldValues(first:100) { pageInfo { hasNextPage } nodes {
            __typename
            ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldRepositoryValue { repository { nameWithOwner } field { ... on ProjectV2FieldCommon { name } } }
          } }
        } }
      } }
    }'''
    raw = _json('api', 'graphql', '-f', 'query=' + query, '-f', 'owner=' + owner,
                '-f', 'name=' + name, '-F', 'number=' + str(issue_number))
    items = _project_items(raw)
    matches = [item for item in items.get('nodes', [])
               if isinstance(item.get('project'),dict) and item['project'].get('id') == project_id]
    if len(matches) != 1:
        raise ValueError('selected terminal Project item is missing or ambiguous')
    _validate_selected_project_item(matches[0],repository,issue_number,project_id)
    return matches[0]


def read_shared_terminal_proof(repository, task_uid, *, repo_root=None, record=None,
                               live_issue=None, live_project_item=None, live_pr=None,
                               live_repository=None, comments=None, issue_reader=None,
                               project_item_reader=None, pr_reader=None,
                               repository_reader=None, comments_reader=None,
                               include_readbacks=False):
    """Resolve live facts and delegate both receipt versions to the shared reader."""
    root = pathlib.Path(repo_root or pathlib.Path.cwd()).resolve()
    record = record if isinstance(record, dict) else task_record(root, task_uid)
    issue_number = record.get('issue_number')
    if isinstance(issue_number, str) and re.fullmatch(r'[1-9][0-9]*', issue_number):
        issue_number = int(issue_number)
    if type(issue_number) is not int or issue_number < 1 or record.get('repository') != repository:
        raise ValueError('mapped terminal task identity is incomplete')
    issue = live_issue if isinstance(live_issue, dict) else (issue_reader or read_issue)(repository, issue_number)
    project_item = (live_project_item if isinstance(live_project_item, dict)
                    else (project_item_reader or read_live_project_item)(repository, issue_number))
    pr_number = record.get('pr_number')
    if isinstance(pr_number, str) and re.fullmatch(r'[1-9][0-9]*', pr_number):
        pr_number = int(pr_number)
    if type(pr_number) is not int or pr_number < 1:
        pr_number_text = _single_markdown_field(issue.get('body'), 'pr_number')
        if not pr_number_text or not re.fullmatch(r'[1-9][0-9]*', pr_number_text):
            raise ValueError('mapped terminal task has no PR number')
        pr_number = int(pr_number_text)
    pr = live_pr if isinstance(live_pr, dict) else (pr_reader or read_pull_request)(repository, pr_number)
    type_map = record.get('phase_receipt_type')
    v2_selected = isinstance(type_map, dict) and type_map.get('post_merge_done') == 'oasis7_terminal_delivery'
    if not isinstance(live_repository, dict) and v2_selected:
        import terminal_proof
        resolver = repository_reader or getattr(terminal_proof, 'read_live_repository', None)
        if resolver is None:
            raise ValueError('shared live repository resolver is unavailable')
        phase_receipts = record.get('phase_receipts')
        selected_receipt = phase_receipts.get('post_merge_done') if isinstance(phase_receipts, dict) else None
        target_oid = selected_receipt.get('observed_target_oid') if isinstance(selected_receipt, dict) else None
        merge_oid = pr.get('merge_commit_sha') if isinstance(pr, dict) else None
        live_repository = resolver(repository, merge_oid, target_oid)
    elif not isinstance(live_repository, dict):
        live_repository = {}
    comment_rows = comments if isinstance(comments, list) else (comments_reader or read_comments)(repository, issue_number)
    import terminal_proof
    reader = getattr(terminal_proof, 'read_terminal_proof', None)
    if reader is None:
        raise ValueError('shared terminal proof reader is unavailable')
    proof = reader(
        root, task_uid, record,
        live_issue=issue,
        live_project_item=project_item,
        live_pr=pr,
        live_repository=live_repository,
        comments=comment_rows,
    )
    if include_readbacks:
        proof['_readbacks'] = {
            'issue': issue, 'project_item': project_item, 'pr': pr,
            'repository': live_repository, 'comments': comment_rows,
        }
    return proof


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
                               comments_reader=None,pr_reader=None,receipt_reader=None,repo_root=None,
                               task_mapping_record=None,project_item_reader=None,repository_reader=None):
    blockers=[]
    try:
        if not re.fullmatch(r'[^/\s]+/[^/\s]+',repository) or not re.fullmatch(r'task_[0-9a-f]{32}',task_uid) or type(issue_number) is not int or issue_number<1:
            raise ValueError('invalid selected terminal identity')
        root = pathlib.Path(repo_root or pathlib.Path.cwd()).resolve()
        mapped = task_mapping_record
        if not isinstance(mapped, dict):
            try:
                mapped = task_record(root, task_uid)
            except (OSError, json.JSONDecodeError, ValueError):
                mapped = None
        mapped_receipts = mapped.get('phase_receipts') if isinstance(mapped, dict) else None
        has_terminal_selector = (
            _v2_selector_present(mapped)
            or (isinstance(mapped_receipts, dict) and 'post_merge_done' in mapped_receipts)
        ) if isinstance(mapped, dict) else False
        if isinstance(mapped, dict) and has_terminal_selector:
            proof = read_shared_terminal_proof(
                repository, task_uid, repo_root=root, record=mapped,
                issue_reader=issue_reader, project_item_reader=project_item_reader,
                pr_reader=pr_reader, repository_reader=repository_reader,
                comments_reader=comments_reader,
            )
            if proof.get('status') != 'passed':
                raise ValueError('shared terminal proof reader did not pass')
            return {
                'status': 'passed', 'blockers': [], 'task_uid': task_uid,
                'issue_url': f'https://github.com/{repository}/issues/{issue_number}',
                'protocol_version': proof.get('protocol_version'),
                'terminal_evidence': proof.get('comment_id'),
                'proof': proof,
            }
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
        fields=normalize_project_fields(item,repository)
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
