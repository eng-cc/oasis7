"""Human-scoped workflow tools executed as code under test, never release authority."""
import json
import re
import subprocess
import io
import tarfile
import tempfile
import sys
import os
import hashlib
from contextlib import contextmanager
from datetime import datetime
from types import MappingProxyType
from pathlib import Path, PurePosixPath

MARKER = "Workflow Maintenance Authority:"
FIELDS = {"repository", "task_uid", "issue_number", "pr_number", "purpose",
          "allowed_write_paths", "allowed_tool_paths"}
_CONTINUATION_TOKEN = object()

def _utc_time(value):
    if not isinstance(value, str) or not re.fullmatch(r'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z', value):
        raise ValueError('maintenance server UTC timestamp is invalid')
    return datetime.fromisoformat(value[:-1] + '+00:00')

def _server_pages(value):
    if (not isinstance(value, list) or any(not isinstance(page, list) for page in value)
            or any(not isinstance(item, dict) for page in value for item in page)):
        raise ValueError('maintenance server pagination is incomplete or malformed')
    return [item for page in value for item in page]

class ReadyMaintenanceContinuation:
    """Process-local proof; never deserialize a release decision into this type."""
    __slots__ = ('_scope', '_current_target_oid', '_provenance')
    def __init__(self, token, scope, current_target_oid, provenance):
        if token is not _CONTINUATION_TOKEN:
            raise ValueError("ready continuation requires independent live verification")
        object.__setattr__(self, '_scope', _immutable_value(scope))
        object.__setattr__(self, '_current_target_oid', current_target_oid)
        object.__setattr__(self, '_provenance', _immutable_value(provenance))

    def __setattr__(self, name, value):
        raise AttributeError('ready continuation is immutable')

    @property
    def scope(self):
        return self._scope

    @property
    def current_target_oid(self):
        return self._current_target_oid

    @property
    def provenance(self):
        return self._provenance

def _immutable_value(value):
    if isinstance(value, dict):
        return MappingProxyType({key: _immutable_value(item) for key, item in value.items()})
    if isinstance(value, (list, tuple)):
        return tuple(_immutable_value(item) for item in value)
    if value is None or type(value) in (str, bool, int, float):
        return value
    raise ValueError('ready continuation contains unsupported authority data')

@contextmanager
def _immutable_archive(root, revision):
    raw = subprocess.check_output(['git', '-C', str(root), 'archive', revision,
                                   'scripts', '.agents', '.codex', '.pm'])
    with tempfile.TemporaryDirectory(prefix='maintenance-immutable-') as directory:
        with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
            for entry in archive.getmembers():
                path = PurePosixPath(entry.name)
                if path.is_absolute() or '..' in path.parts or not (entry.isfile() or entry.isdir()):
                    raise ValueError('unsafe maintenance immutable archive')
            archive.extractall(directory)
        yield Path(directory), hashlib.sha256(raw).hexdigest()

def _isolated_json(root, code, *arguments, input_data=None):
    environment = dict(os.environ, OASIS7_LOOP_TOOL_ROOT=str(root), PYTHONDONTWRITEBYTECODE='1')
    result = subprocess.run([sys.executable, '-I', '-c', code, str(root), *map(str, arguments)],
        cwd=root, env=environment, text=True, capture_output=True, timeout=180,
        input=json.dumps(input_data) if input_data is not None else None)
    if result.returncode:
        raise ValueError('immutable maintenance proof reader failed: ' + result.stderr[-2000:])
    try:
        return json.loads(result.stdout)
    except ValueError as exc:
        raise ValueError('immutable maintenance proof reader returned invalid JSON') from exc

def read_ready_maintenance_continuation(root, repository, comment_id, task_uid, pr_number,
                                      head_oid, *, check_name='required-gate', app,
                                      review_plan_path=None):
    """Reconstruct an existing promotion using live servers and immutable Q/H code."""
    scope = read_maintenance_scope(repository, comment_id, task_uid, pr_number, head_oid)
    receipt_closure = ('scripts/pm/ci-ready-receipt.py', 'scripts/pm/ci_ready_receipt_identity.py',
                       'scripts/pm/workflow_maintenance.py', 'scripts/pm/task_primary_package.py')
    if not set(receipt_closure).issubset(scope['allowed_tool_paths']):
        raise ValueError('ready continuation candidate proof closure is not approved')
    info = _gh(f'repos/{repository}')
    branch = info.get('default_branch')
    q = _gh(f'repos/{repository}/commits/{branch}').get('sha')
    if not re.fullmatch(r'[0-9a-f]{40,64}', str(q)):
        raise ValueError('ready continuation protected target is unavailable')
    root = Path(root).resolve()
    if review_plan_path is None:
        raise ValueError('ready continuation lacks the canonical exact-head source review plan locator')
    mapping = json.loads((root / '.pm/github-project-sync/tasks.json').read_text())
    project = mapping.get('project') or {}
    record = (mapping.get('tasks') or {}).get(task_uid) or {}
    if (record.get('repository') != repository or record.get('issue_number') != scope['issue_number']
            or not project.get('id') or type(project.get('number')) is not int
            or not record.get('project_item_id')):
        raise ValueError('ready continuation canonical Task/Project locator is incomplete')
    tree = subprocess.check_output(['git', '-C', str(root), 'rev-parse', head_oid + '^{tree}'], text=True).strip()
    with _immutable_archive(root, q) as (protected, q_digest):
        code = """
import sys,json,importlib.util
from pathlib import Path
root=Path(sys.argv[1]); sys.path.insert(0,str(root/'scripts/pm'))
spec=importlib.util.spec_from_file_location('protected_gate',root/'scripts/pm/pr-lifecycle-gate.py')
gate=importlib.util.module_from_spec(spec); spec.loader.exec_module(gate)
repo,uid,issue,number,project_id,project_number,subject_root,review_plan,head=sys.argv[2:11]
data=gate.load_live(number,repository_hint=repo,number_hint=int(number),effective_root=root,task_uid=uid)
data.update(gate.rebuild_issue_evidence(repo,int(issue),uid,data))
import review_preflight_handoff as handoff
plan,source,_,_,ledger,slices,*_=handoff.validate_plan_inputs(Path(subject_root),Path(review_plan),allow_promoted_ledger=True)
if plan['task_uid']!=uid or plan['frozen_head']!=head:
 raise ValueError('continuation canonical review plan Task/head differs')
import subprocess
roles=','.join(dict.fromkeys(item['role'] for item in slices))
subprocess.run([sys.executable,'-I',str(root/'scripts/pm/validate-review-provenance.py'),'--root',subject_root,'--task-uid',uid,'--ledger',str(ledger),'--roles',roles,'--source-head',head],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
from projection_publication_contract import decode_marker
c1=decode_marker(data.get('body',''))
if c1.get('task_uid')!=uid or c1.get('source_head_oid')!=head:
 raise ValueError('continuation current C1 Task/head differs')
result=gate.decision(data,False,evidence_mode='pending_live_loop')
spec=importlib.util.spec_from_file_location('protected_project',root/'scripts/pm/github-project-sync.py')
project=importlib.util.module_from_spec(spec); spec.loader.exec_module(project)
binding=project.read_live_issue_project_item(repo,int(issue),project_id,int(project_number))
print(json.dumps({'data':data,'result':result,'project_binding':binding,'source_review_plan_sha256':__import__('hashlib').sha256(Path(review_plan).read_bytes()).hexdigest()}))
"""
        protected_release_code = code
        release = _isolated_json(protected, code, repository, task_uid, scope['issue_number'],
            pr_number, project['id'], project['number'], root, Path(review_plan_path).resolve(), head_oid)
    data, decision = release.get('data') or {}, release.get('result') or {}
    if (decision.get('ready_for_merge') is not True or data.get('headRefOid') != head_oid
            or data.get('isDraft') is not False or data.get('state') != 'OPEN'):
        raise ValueError('ready continuation protected full release vector is blocked')
    binding = release.get('project_binding') or {}
    item = binding.get('item') or {}
    fields = item.get('field_values') or {}
    if (binding.get('complete') is not True or item.get('id') != record['project_item_id']
            or fields.get('Task UID') != task_uid
            or fields.get('PR') != f'https://github.com/{repository}/pull/{pr_number}'
            or fields.get('PM Status') != 'pr_watch' or fields.get('Workflow Phase') != 'pr_watch'):
        raise ValueError('ready continuation live reciprocal Project binding is incomplete')
    checks = (data.get('policy_discovery') or {}).get('required_status_checks') or []
    if not any(item.get('context') == check_name and item.get('app_id') in (None, app) for item in checks):
        raise ValueError('ready continuation check is not protected required policy')
    # Keep every authority dependency at Q; overlay only the explicitly approved
    # receipt closure. A complete H archive would silently execute unapproved code.
    with _immutable_archive(root, q) as (candidate, _):
        closure_hash = hashlib.sha256()
        for relative in receipt_closure:
            entry = subprocess.check_output(['git', '-C', str(root), 'ls-tree', head_oid, '--', relative], text=True).split()
            if not entry or entry[0] not in ('100644', '100755'):
                raise ValueError('ready continuation candidate proof module mode is unsafe')
            contents = subprocess.check_output(['git', '-C', str(root), 'show', head_oid + ':' + relative])
            path = candidate / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(contents)
            closure_hash.update(relative.encode() + b'\0' + contents)
        h_digest = closure_hash.hexdigest()
        code = """
import sys,json,importlib.util
from pathlib import Path
root=Path(sys.argv[1]); sys.path.insert(0,str(root/'scripts/pm'))
spec=importlib.util.spec_from_file_location('candidate_receipt',root/'scripts/pm/ci-ready-receipt.py')
m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
repo,uid,issue,number,check,app,q,branch=sys.argv[2:10]
print(json.dumps(m.read_current_target_proof_without_selection(repo,uid,int(issue),int(number),check,int(app),current_target_oid=q,base_ref=branch)))
"""
        proof = _isolated_json(candidate, code, repository, task_uid, scope['issue_number'],
            pr_number, check_name, app, q, branch)
    if (proof.get('source_head_oid') != head_oid or proof.get('current_target_oid') != q
            or proof.get('maintenance_authority_comment_id') != comment_id):
        raise ValueError('ready continuation prior successful proof identity differs')
    pages = _gh(f"repos/{repository}/issues/{scope['issue_number']}/comments", '--paginate', '--slurp')
    comments = _server_pages(pages)
    issue = _gh(f"repos/{repository}/issues/{scope['issue_number']}")
    pull = _gh(f'repos/{repository}/pulls/{pr_number}')
    with _immutable_archive(root, q) as (protected, _):
        code = """
import sys,json,re
from pathlib import Path
sys.path.insert(0,str(Path(sys.argv[1])/'scripts/pm'))
from projection_publication_contract import decode_marker
from pr_projection_publication import resolve_task_publication,_TASK_PUBLICATION_FIELDS,parse_publication_binding_comment,validate_publication_binding
x=json.load(sys.stdin); pr=x['pr']; task=x['task']; body=task.get('body','')
c1=decode_marker(pr['body']); expected={key:c1[key] for key in _TASK_PUBLICATION_FIELDS}
def scalar(name):
 values=re.findall(r'^- '+name+r': `([^`]+)`$',body,re.M)
 if len(values)!=1: raise ValueError('current C1 Task scalar is missing or ambiguous: '+name)
 return values[0]
binding={'repository':x['repository'],'number':pr['number'],'url':pr['html_url'],'state':pr['state'],'merged':pr.get('merged',False),'draft':pr['draft'],'source_ref':pr['head']['ref'],'target_ref':pr['base']['ref'],'source_head_oid':pr['head']['sha'],'task_uid':expected['task_uid'],'issue_number':task['number'],'created_at':pr['created_at'],'updated_at':pr['updated_at'],'task_status':scalar('status'),'task_phase':scalar('workflow_phase'),'task_pr_number':int(scalar('pr_number')),'task_pr_url':scalar('pr_url'),'pr_author':pr['user']['login'],'pr_author_type':pr['user']['type']}
result=resolve_task_publication({'complete':True,'repository':x['repository'],'issue_number':task['number'],'comments':x['comments']},expected,live_task_author=task.get('user'),pr_binding=binding)
if result.get('status')!='passed': raise ValueError('current C1 server publication failed: '+str(result.get('blockers')))
envelopes=[]
for comment in x['comments']:
 if '<!-- oasis7-ci-publication-binding/v1 -->' in str(comment.get('body','')):
  value=parse_publication_binding_comment(comment['body'])
  if value.get('publication_id')==result['publication']['publication_id']:
   author=comment.get('user') or {}; owner=task.get('user') or {}
   if (type(comment.get('id')) is not int or comment['id']<1 or comment.get('created_at')!=comment.get('updated_at')
       or author.get('login')!=owner.get('login') or author.get('type')!='User'
       or comment.get('issue_url')!='https://api.github.com/repos/'+x['repository']+'/issues/'+str(task['number'])
       or comment.get('html_url')!='https://github.com/'+x['repository']+'/issues/'+str(task['number'])+'#issuecomment-'+str(comment['id'])):
    raise ValueError('reciprocal C1 binding server provenance differs')
   envelopes.append(validate_publication_binding(value,result['publication']))
if len(envelopes)!=1 or envelopes[0]['pr_number']!=pr['number'] or envelopes[0]['pr_url']!=pr['html_url']:
 raise ValueError('current C1 reciprocal envelope is absent or ambiguous')
print(json.dumps(result))
"""
        c1_readback = _isolated_json(protected, code,
            input_data=dict(repository=repository, task=issue, pr=pull, comments=comments))
    closes = []
    for comment in comments:
        body = str(comment.get('body') or '')
        expected = ('<!-- oasis7-pm-evidence -->', 'Task UID: ' + task_uid,
            'Workflow Phase: pre_pr_ready', 'Task Status: ready',
            'Immutable Verification Head: ' + head_oid, 'Immutable Verification Tree: ' + tree)
        if not all(line in body.splitlines() for line in expected):
            continue
        actor = comment.get('user') or {}
        if (type(comment.get('id')) is not int or comment['id'] < 1
                or actor.get('type') != 'User' or not actor.get('login')
                or comment.get('created_at') != comment.get('updated_at')
                or comment.get('issue_url') != f"https://api.github.com/repos/{repository}/issues/{scope['issue_number']}"
                or comment.get('html_url') != f"https://github.com/{repository}/issues/{scope['issue_number']}#issuecomment-{comment['id']}"):
            continue
        _utc_time(comment.get('created_at'))
        permission = _gh(f"repos/{repository}/collaborators/{actor['login']}/permission")
        if permission.get('permission') == 'admin' and (permission.get('user') or {}).get('login') == actor['login']:
            closes.append(comment)
    if not closes:
        raise ValueError('ready continuation lacks authenticated exact-head ready closeout')
    pages = _gh(f'repos/{repository}/issues/{pr_number}/timeline', '--paginate', '--slurp')
    events = _server_pages(pages)
    transitions = [event for event in events if event.get('event') in ('ready_for_review', 'converted_to_draft')]
    for event in transitions:
        if type(event.get('id')) is not int or event['id'] < 1:
            raise ValueError('maintenance server transition identity is invalid')
        _utc_time(event.get('created_at'))
        if event.get('url') is not None and event['url'] != f"https://api.github.com/repos/{repository}/issues/events/{event['id']}":
            raise ValueError('maintenance server transition URL is invalid')
    transitions.sort(key=lambda event: (_utc_time(event['created_at']), event['id']))
    if not transitions or transitions[-1].get('event') != 'ready_for_review':
        raise ValueError('ready continuation lacks server draft-to-ready transition')
    transition = transitions[-1]
    actor = transition.get('actor') or {}
    if actor.get('type') != 'User' or not actor.get('login'):
        raise ValueError('maintenance server promotion actor is invalid')
    permission = _gh(f"repos/{repository}/collaborators/{actor['login']}/permission")
    if ((permission.get('user') or {}).get('login') != actor['login']
            or permission.get('permission') not in ('admin', 'maintain', 'write')):
        raise ValueError('maintenance promotion actor lacks current write authority')
    run = _gh(f"repos/{repository}/actions/runs/{proof['workflow_run_id']}")
    if (run.get('conclusion') != 'success' or run.get('head_sha') not in (head_oid, proof.get('checkout_oid'))
            or _utc_time(run.get('updated_at')) > _utc_time(transition['created_at'])
            or not any(_utc_time(comment['created_at']) <= _utc_time(transition['created_at']) for comment in closes)):
        raise ValueError('ready continuation does not prove successful CI and closeout before promotion')
    fresh_scope = read_maintenance_scope(repository, comment_id, task_uid, pr_number, head_oid)
    fresh_pull = _gh(f'repos/{repository}/pulls/{pr_number}')
    if (fresh_scope != scope or _gh(f'repos/{repository}/commits/{branch}').get('sha') != q
            or any(fresh_pull.get(key) != pull.get(key) for key in ('state','merged','draft','body','head','base'))):
        raise ValueError('ready continuation current scope or protected target changed')
    with _immutable_archive(root, q) as (protected, _):
        fresh_release = _isolated_json(protected, protected_release_code, repository, task_uid,
            scope['issue_number'], pr_number, project['id'], project['number'], root,
            Path(review_plan_path).resolve(), head_oid)
    if ((fresh_release.get('result') or {}).get('ready_for_merge') is not True
            or any((fresh_release.get('data') or {}).get(key) != data.get(key)
                   for key in ('headRefOid','body','state','isDraft','baseRefName','headRefName'))
            or fresh_release.get('project_binding') != binding
            or fresh_release.get('source_review_plan_sha256') != release['source_review_plan_sha256']
            or _gh(f'repos/{repository}/commits/{branch}').get('sha') != q):
        raise ValueError('ready continuation final protected full release vector changed')
    return ReadyMaintenanceContinuation(_CONTINUATION_TOKEN, scope, q,
        dict(protected_target_oid=q, source_head_oid=head_oid, source_tree_oid=tree,
             protected_archive_sha256=q_digest, candidate_archive_sha256=h_digest,
             source_review_plan_sha256=release['source_review_plan_sha256'],
             current_c1_comment=c1_readback['comment'],
             closeout_comment_ids=[comment['id'] for comment in closes],
             ready_transition_id=transition.get('id'), current_target_proof=proof))
TOOL_PATHS = (
    "scripts/pm/workflow_maintenance.py", "scripts/pm/loop-ci.py",
    "scripts/pm/loop_ci_content.py", "scripts/pm/loop_policy.py",
    "scripts/pm/loop_contracts.py", "scripts/pm/loop_traceability.py",
    "scripts/pm/loop_approval_authority.py", "scripts/pm/loop_leaf_result.py",
    "scripts/pm/pr_projection_publication.py", "scripts/pm/pr_projection_journal.py",
    "scripts/pm/portable_file_lock.py", "scripts/pm/projection_publication_contract.py",
    "scripts/pm/workflow-impact-projection.py", "scripts/plan-rust-required-scope.py",
    "scripts/product-doc-content-check.py", "scripts/product_doc_markdown.py",
    "scripts/pm/task_primary_package.py", "scripts/pm/trusted_cargo_scope.py",
    "scripts/pm/cargo_package_change_classification.py",
    "scripts/pm/cargo_package_profile_planner.py", "scripts/document_corpus.py",
    "scripts/pm/review-role-selector.py", "scripts/pm/loop_recovery.py",
    "scripts/pm/loop_gate.py", "scripts/pm/loop.py", "scripts/pm/loop_terminal.py",
)

HUMAN_RECONCILIATION_TOOL_PATHS = tuple("scripts/pm/" + name for name in (
    "github-project-task.py", "pr_projection_record_pr.py", "pr_projection_publish.py",
    "workflow_maintenance.py", "github-project-sync.py", "github_api.py",
    "task_complete_claim.py", "task_primary_package.py", "loop_leaf_result.py",
    "workflow-durable-store.py", "pr_projection_publication.py",
    "projection_publication_contract.py", "pr_projection_journal.py",
    "portable_file_lock.py", "pr_projection_transition.py",
    "closed_duplicate_candidate_guard.py"))

def _unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate maintenance field")
        value[key] = item
    return value

def _paths(value):
    if (not isinstance(value, list) or not value or any(not isinstance(p, str) for p in value)
            or len(set(value)) != len(value)):
        raise ValueError("maintenance paths must be nonempty and unique")
    for path in value:
        if (not isinstance(path, str) or not path.isascii() or not path.strip()
                or PurePosixPath(path).is_absolute() or ".." in PurePosixPath(path).parts
                or str(PurePosixPath(path)) != path or any(c in path for c in "*?[]\\\n\r")):
            raise ValueError("maintenance path must be exact and repository relative")
    return value

def parse_maintenance_authority(body):
    if not isinstance(body, str) or body.count(MARKER) != 1:
        raise ValueError("one maintenance scope record is required")
    payload = body.split(MARKER, 1)[1].strip()
    match = re.fullmatch(r"```json\s*\n(.*?)\n```", payload, re.S)
    if match:
        payload = match[1]
    value = json.loads(payload, object_pairs_hook=_unique)
    if not isinstance(value, dict) or set(value) != FIELDS:
        raise ValueError("maintenance scope has unsupported fields")
    if (not re.fullmatch(r"[^/\s]+/[^/\s]+", str(value["repository"]))
            or not re.fullmatch(r"task_[0-9a-f]{32}", str(value["task_uid"]))
            or any(type(value[k]) is not int or value[k] < 1 for k in ("issue_number", "pr_number"))
            or value["purpose"] != "candidate-tool-verification"):
        raise ValueError("maintenance scope identity is invalid")
    _paths(value["allowed_write_paths"]); _paths(value["allowed_tool_paths"])
    return value

def maintenance_comment_id(pr_body):
    lines = re.findall(r"(?m)^Workflow Maintenance Authority:[^\n]*$", pr_body or "")
    if not lines:
        return None
    if len(lines) != 1:
        raise ValueError("duplicate maintenance locator")
    match = re.fullmatch(r"Workflow Maintenance Authority: ([1-9][0-9]*)", lines[0])
    if not match:
        raise ValueError("malformed maintenance locator")
    return int(match[1])

def validate_maintenance_authority(comment_readback, author_permission, live_task, live_pr,
                                   event_head_oid, required_write_paths=(), required_tool_paths=(),
                                   *, require_draft=True, binding_phase="bound", ready_continuation=None,
                                   _scope_only=False):
    value = parse_maintenance_authority(comment_readback.get("body"))
    repo, issue, number = value["repository"], value["issue_number"], value["pr_number"]
    actor = comment_readback.get("user") or {}
    comment_id = comment_readback.get("id")
    if (type(comment_id) is not int or comment_id < 1 or actor.get("type") != "User"
            or not actor.get("login")
            or comment_readback.get("issue_url") != f"https://api.github.com/repos/{repo}/issues/{issue}"
            or comment_readback.get("html_url") != f"https://github.com/{repo}/issues/{issue}#issuecomment-{comment_id}"
            or not comment_readback.get("created_at")
            or comment_readback.get("created_at") != comment_readback.get("updated_at")):
        raise ValueError("maintenance comment server identity is invalid or edited")
    permission_user = author_permission.get("user") or {}
    if (permission_user.get("login") != actor["login"]
            or not (author_permission.get("permission") == "admin"
                    or (author_permission.get("permissions") or {}).get("admin") is True)):
        raise ValueError("maintenance scope author lacks current admin permission")
    body = live_task.get("body") or ""
    hold = re.findall(r"(?m)^- merge_hold_active: `([^`]+)`$", body)
    hold_lines = re.findall(r"(?m)^- merge_hold_active:[^\n]*$", body)
    if (live_task.get("number") != issue or str(live_task.get("state")).lower() != "open"
            or live_task.get("html_url") != f"https://github.com/{repo}/issues/{issue}"
            or re.findall(r"(?m)^task_uid: ([^\n]+)$", body) != [value["task_uid"]]
            or len(hold_lines) != len(hold) or len(hold) > 1 or (hold and hold != ["false"])):
        raise ValueError("maintenance Task identity or hold is invalid")
    head, base = live_pr.get("head") or {}, live_pr.get("base") or {}
    pr_body = live_pr.get("body") or ""
    if (not re.fullmatch(r"[0-9a-f]{40,64}", str(event_head_oid))
            or head.get("sha") != event_head_oid or live_pr.get("number") != number
            or live_pr.get("html_url") != f"https://github.com/{repo}/pull/{number}"
            or live_pr.get("state") != "open" or live_pr.get("merged_at") is not None
            or live_pr.get("merged", False) is not False
            or not isinstance(live_pr.get("draft"), bool)
            or (head.get("repo") or {}).get("full_name") != repo
            or (base.get("repo") or {}).get("full_name") != repo
            or not head.get("ref") or not base.get("ref")
            or len(re.findall(r"(?m)^Task:[^\n]*$", pr_body)) != 1
            or re.findall(r"(?m)^Task: (task_[0-9a-f]{32})$", pr_body) != [value["task_uid"]]
            or re.findall(r"(?m)^Refs #([1-9][0-9]*)$", pr_body) != [str(issue)]
            or set(re.findall(r"task_[0-9a-f]{32}", pr_body)) != {value["task_uid"]}):
        raise ValueError("maintenance live PR/event identity mismatch")
    if not _scope_only and require_draft is not True:
        raise ValueError("maintenance ready continuation lacks authenticated complete promotion proof")
    if not _scope_only and live_pr.get("draft") is not True:
        if (type(ready_continuation) is not ReadyMaintenanceContinuation
                or ready_continuation.scope.get('subject_head_oid') != event_head_oid
                or ready_continuation.provenance.get('source_head_oid') != event_head_oid
                or (ready_continuation.provenance.get('current_target_proof') or {}).get('source_head_oid') != event_head_oid
                or (ready_continuation.provenance.get('current_target_proof') or {}).get('current_target_oid') != ready_continuation.current_target_oid
                or ready_continuation.provenance.get('protected_target_oid') != ready_continuation.current_target_oid
                or any(ready_continuation.scope.get(key) != _immutable_value(value[key]) for key in FIELDS)
                or ready_continuation.scope.get('comment_id') != comment_id
                or _gh(f"repos/{repo}/commits/{base.get('ref')}").get('sha') != ready_continuation.current_target_oid):
            raise ValueError("maintenance initial selection requires a draft PR or authenticated ready continuation")
    elif not _scope_only and ready_continuation is not None:
        raise ValueError("maintenance ready continuation cannot authorize draft initial selection")
    if binding_phase not in {"bound", "validation-start-only", "metadata-only-reconciliation"}:
        raise ValueError("maintenance binding phase is unsupported")
    numbers = re.findall(r"(?m)^- pr_number: `([^`]+)`$", body)
    urls = re.findall(r"(?m)^- pr_url: `([^`]+)`$", body)
    number_lines = re.findall(r"(?m)^- pr_number:[^\n]*$", body)
    url_lines = re.findall(r"(?m)^- pr_url:[^\n]*$", body)
    if len(numbers) != len(number_lines) or len(urls) != len(url_lines):
        raise ValueError("maintenance reciprocal Task PR binding is malformed")
    complete = numbers == [str(number)] and urls == [f"https://github.com/{repo}/pull/{number}"]
    absent = not numbers and not urls
    if not complete and not (absent and binding_phase != "bound"):
        raise ValueError("maintenance reciprocal Task PR binding is incomplete or conflicting")
    if (not set(required_write_paths).issubset(value["allowed_write_paths"])
            or not set(required_tool_paths).issubset(value["allowed_tool_paths"])):
        raise ValueError("maintenance scope does not cover the selected paths")
    if _scope_only:
        return {**value, "comment_id": comment_id, "subject_head_oid": event_head_oid,
                "selection_authorized": False}
    return {**value, "comment_id": comment_id, "tool_revision": event_head_oid}

def _gh(*args):
    return json.loads(subprocess.check_output(["gh", "api", *args], text=True))

def read_maintenance_authority(repository, comment_id, task_uid, pr_number, event_head_oid,
                               required_write_paths=(), required_tool_paths=(), *, require_draft=True,
                               binding_phase="bound", ready_continuation=None):
    comment = _gh(f"repos/{repository}/issues/comments/{comment_id}")
    value = parse_maintenance_authority(comment.get("body"))
    if (value["repository"], value["task_uid"], value["pr_number"]) != (repository, task_uid, pr_number):
        raise ValueError("maintenance locator selects another Task or PR")
    actor = (comment.get("user") or {}).get("login")
    permission = _gh(f"repos/{repository}/collaborators/{actor}/permission")
    task = _gh(f"repos/{repository}/issues/{value['issue_number']}")
    pr = _gh(f"repos/{repository}/pulls/{pr_number}")
    repository_info = _gh(f"repos/{repository}")
    if (pr.get("base") or {}).get("ref") != repository_info.get("default_branch"):
        raise ValueError("maintenance PR does not target the live default branch")
    return validate_maintenance_authority(comment, permission, task, pr, event_head_oid,
                                          required_write_paths, required_tool_paths,
                                          require_draft=require_draft, binding_phase=binding_phase,
                                          ready_continuation=ready_continuation)

def read_maintenance_scope(repository, comment_id, task_uid, pr_number, event_head_oid,
                           required_write_paths=(), required_tool_paths=()):
    """Authenticate scope for proof inspection without selecting any candidate tool."""
    comment = _gh(f"repos/{repository}/issues/comments/{comment_id}")
    value = parse_maintenance_authority(comment.get("body"))
    if (value["repository"], value["task_uid"], value["pr_number"]) != (repository, task_uid, pr_number):
        raise ValueError("maintenance locator selects another Task or PR")
    actor = (comment.get("user") or {}).get("login")
    permission = _gh(f"repos/{repository}/collaborators/{actor}/permission")
    task = _gh(f"repos/{repository}/issues/{value['issue_number']}")
    pr = _gh(f"repos/{repository}/pulls/{pr_number}")
    repository_info = _gh(f"repos/{repository}")
    if (pr.get("base") or {}).get("ref") != repository_info.get("default_branch"):
        raise ValueError("maintenance PR does not target the live default branch")
    return validate_maintenance_authority(comment, permission, task, pr, event_head_oid,
        required_write_paths, required_tool_paths, _scope_only=True)

def validate_candidate_tool_root(tool_root, target_root, authority, *, execution_revision=None,
                                 required_tool_paths=TOOL_PATHS):
    tool, target = Path(tool_root).resolve(), Path(target_root).resolve()
    def git(root, *args):
        return subprocess.check_output(["git", "-C", str(root), *args])
    revision = execution_revision or authority["tool_revision"]
    if not re.fullmatch(r"[0-9a-f]{40,64}", str(revision)):
        raise ValueError("maintenance execution revision is malformed")
    if (git(tool, "rev-parse", "HEAD").decode().strip() != revision
            or git(tool, "rev-parse", "--path-format=absolute", "--git-common-dir")
            != git(target, "rev-parse", "--path-format=absolute", "--git-common-dir")):
        raise ValueError("maintenance tool checkout identity mismatch")
    for relative in required_tool_paths:
        if relative not in authority["allowed_tool_paths"]:
            raise ValueError("maintenance tool closure is not authorized")
        path = tool / relative
        entry = git(tool, "ls-tree", revision, "--", relative).decode().split()
        if not entry:
            raise ValueError("maintenance tool is absent from the selected revision")
        mode = entry[0]
        if mode not in {"100644", "100755"} or path.is_symlink() or not path.resolve().is_relative_to(tool):
            raise ValueError("maintenance tool mode/path is unsafe")
        if path.read_bytes() != git(tool, "show", f"{revision}:{relative}"):
            raise ValueError("maintenance tool bytes changed")
    if git(tool, "ls-files", "--others", "--", "scripts/pm", ":(exclude)**/__pycache__/**").strip():
        raise ValueError("maintenance tool import shadow")
    return authority
