"""Narrow protected exception facts; optional Task evidence is never release authority.

The finite rule concerns independently controlled workflow orchestration, not
test coverage or a path-based risk label. Normal PRs never read this evidence.
"""
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import io
import tarfile
import tempfile
from contextlib import contextmanager
from pathlib import Path
from types import MappingProxyType

MARKER = 'Strict Integration Exception:'
REVIEW_MARKER = 'Strict Integration Exception Review:'
WORKFLOW = '.github/workflows/rust.yml'
# Audited existing ordinary-maintenance/strict execution contract. Unsupported
# workflow revisions require a protected policy update, not candidate assertions.
EXECUTOR_SHA256 = '689c4df2787f334933b7a755e046534f574d9f881a8845aa7aea31e64a020499'
SCOPE = {'scope': 'full', 'tier': 'required', 'profile': 'full', 'checks': ['required-gate']}
SUPPLEMENTS = ['ordinary_tests', 'ordinary_matrix', 'protected_target_driver', 'candidate_controlled_job']
_TOKEN = object()

def _maintenance():
    spec = importlib.util.spec_from_file_location('strict_exception_maintenance', Path(__file__).with_name('workflow_maintenance.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

class VerifiedStrictException:
    """Only the live protected producer can mint these process-local facts."""
    def __init__(self, token, facts):
        if token is not _TOKEN:
            raise ValueError('strict exception requires protected producer')
        def freeze(value):
            if isinstance(value, dict): return MappingProxyType({key: freeze(item) for key, item in value.items()})
            if isinstance(value, list): return tuple(freeze(item) for item in value)
            return value
        object.__setattr__(self, 'facts', freeze(facts))

    def __setattr__(self, name, value):
        raise AttributeError('verified strict exception is immutable')

    def as_dict(self):
        def thaw(value):
            if isinstance(value, MappingProxyType): return {key: thaw(item) for key, item in value.items()}
            if isinstance(value, tuple): return [thaw(item) for item in value]
            return value
        return thaw(self.facts)

def locator(body):
    lines = re.findall(r'(?m)^Strict Integration Exception:[^\n]*$', body or '')
    if not lines:
        return None
    if len(lines) != 1 or not re.fullmatch(r'Strict Integration Exception: [1-9][0-9]*', lines[0]):
        raise ValueError('malformed strict exception locator')
    return int(lines[0].split(': ')[1])

def _record(body, marker):
    if not isinstance(body, str) or body.count(marker) != 1:
        raise ValueError('one strict exception evidence record required')
    payload = body.split(marker, 1)[1].strip()
    if payload.startswith('```json\n') and payload.endswith('\n```'):
        payload = payload[8:-4]
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate strict exception field')
            result[key] = value
        return result
    return json.loads(payload, object_pairs_hook=unique)

def _auth(comment, permission, repository, issue):
    actor = comment.get('user') or {}
    ident = comment.get('id')
    if (type(ident) is not int or ident < 1 or actor.get('type') != 'User'
            or not actor.get('login') or not comment.get('created_at')
            or comment.get('created_at') != comment.get('updated_at')
            or comment.get('issue_url') != f'https://api.github.com/repos/{repository}/issues/{issue}'
            or comment.get('html_url') != f'https://github.com/{repository}/issues/{issue}#issuecomment-{ident}'
            or (permission.get('user') or {}).get('login') != actor['login']
            or not (permission.get('permission') == 'admin'
                    or (permission.get('permissions') or {}).get('admin') is True)):
        raise ValueError('exception evidence is edited, detached, or lacks current admin authority')
    return actor['login']

def _control_step(source, name):
    text = source.decode('utf-8')
    jobs = re.findall(r'(?ms)^  required-gate:\n(.*?)(?=^  [A-Za-z0-9_-]+:|\Z)', text)
    if len(jobs) != 1:
        raise ValueError('required-gate execution graph is unsupported')
    steps = re.findall(r'(?ms)^      - name: ' + re.escape(name) + r'\n(.*?)(?=^      - |\Z)', jobs[0])
    if len(steps) != 1:
        raise ValueError('required control step missing or ambiguous')
    return steps[0]

def constraint_digest(record):
    semantic = {key: value for key, value in record.items() if key != 'review_comment_ids'}
    return hashlib.sha256(json.dumps(semantic, sort_keys=True, separators=(',', ':')).encode()).hexdigest()

def verify(*, record_comment, record_permission, reviews, maintenance_scope,
           task_uid, repository, issue, pr, target_oid, source_workflow, target_workflow,
           role_bindings):
    """Verify authenticated governance adjudication AND immutable execution facts.

    Review establishes that independently controlled orchestration is actually
    necessary. The finite immutable constraint proves ordinary's protected
    driver still runs beneath candidate-owned orchestration; strict uses Q's.
    Neither side alone is sufficient.
    """
    head = (pr.get('head') or {}).get('sha')
    actor = _auth(record_comment, record_permission, repository, issue)
    record = _record(record_comment.get('body'), MARKER)
    expected = {'repository': repository, 'task_uid': task_uid, 'issue_number': issue,
                'pr_number': pr.get('number'), 'source_head_oid': head, 'target_oid': target_oid}
    fields = set(expected) | {'exception_rule', 'constraint', 'obligation', 'ordinary_supplements',
                             'required_check_scope', 'review_comment_ids'}
    if not isinstance(record, dict) or set(record) != fields or any(record.get(k) != v for k, v in expected.items()):
        raise ValueError('strict exception immutable identity mismatch')
    if record['exception_rule'] == 'candidate_context_unrepresentable':
        raise ValueError('existing strict executor only constructs Q/H merge; no additional object or ordering capability')
    if (record['exception_rule'] != 'trusted_executor_isolation'
            or record['constraint'] != 'candidate_required_gate_orchestration'
            or record['obligation'] != 'independent_workflow_orchestration'
            or record['ordinary_supplements'] != SUPPLEMENTS
            or record['required_check_scope'] != SCOPE):
        raise ValueError('unsupported execution constraint or required scope')
    if (maintenance_scope.get('subject_head_oid') != head
            or WORKFLOW not in maintenance_scope.get('allowed_write_paths', ())
            or not re.fullmatch(r'[0-9a-f]{40,64}', str(head))
            or not re.fullmatch(r'[0-9a-f]{40,64}', str(target_oid))):
        raise ValueError('independent maintenance scope does not cover frozen workflow')
    ids = record['review_comment_ids']
    if not isinstance(ids, list) or len(ids) != 2 or len(set(ids)) != 2 or any(type(i) is not int or i < 1 for i in ids):
        raise ValueError('two independent frozen exception reviews required')
    roles = set()
    for comment, permission in reviews:
        _auth(comment, permission, repository, issue)
        review = _record(comment.get('body'), REVIEW_MARKER)
        if (comment.get('id') not in ids
                or set(review) != set(expected) | {'role', 'conclusion', 'ordinary_supplements', 'required_check_scope', 'obligation', 'constraint_digest',
                    'review_plan_path', 'review_plan_sha256', 'role_return_path', 'role_return_sha256',
                    'packet_path', 'packet_digest', 'bootstrap_snapshot_path'}
                or any(review.get(k) != v for k, v in expected.items())
                or review.get('conclusion') != 'independent_orchestration_required'
                or review.get('constraint_digest') != constraint_digest(record)
                or review.get('ordinary_supplements') != SUPPLEMENTS
                or review.get('required_check_scope') != SCOPE
                or review.get('obligation') != record['obligation']):
            raise ValueError('independent exception review binding or adjudication invalid')
        binding = role_bindings.get(comment['id'])
        if (not isinstance(binding, dict) or binding.get('role') != review.get('role')
                or binding.get('head') != head or binding.get('task_uid') != task_uid
                or binding.get('packet_digest') != review.get('packet_digest')
                or binding.get('review_plan_sha256') != review.get('review_plan_sha256')
                or binding.get('role_return_sha256') != review.get('role_return_sha256')):
            raise ValueError('exception review lacks admitted digest-bound professional return')
        roles.add(review.get('role'))
    if len(reviews) != 2 or roles != {'repository_health_engineer', 'qa_engineer'}:
        raise ValueError('role-complete independent exception review required')
    projections = {binding.get('projection_digest') for binding in role_bindings.values()}
    if len(projections) != 1 or not re.fullmatch(r'sha256:[0-9a-f]{64}', str(next(iter(projections)))):
        raise ValueError('exception professional reviews disagree on exact source projection')
    if hashlib.sha256(target_workflow).hexdigest() != EXECUTOR_SHA256:
        raise ValueError('protected strict/ordinary execution contract is unsupported')
    q_step = _control_step(target_workflow, 'Run required test tier')
    h_step = _control_step(source_workflow, 'Run required test tier')
    # Explicitly rule out protected-driver deletion and ordinary test/matrix
    # gaps. The ordinary supplement exists; enclosing workflow control differs.
    ordinary_driver = 'CI_VERBOSE=1 bash "${protected_driver}/scripts/ci-tests.sh" required --repo-root "${GITHUB_WORKSPACE}" --impact-projection "${RUNNER_TEMP}/impact-projection.json"'
    if h_step == q_step or ordinary_driver not in h_step or ordinary_driver not in q_step:
        raise ValueError('no proven residual orchestration boundary after protected ordinary driver')
    guard_pattern = r'(?m)^\s*elif \[\[ "\$\{GITHUB_EVENT_NAME\}" == "pull_request"[^\n]*\]\]; then\s*$'
    q_guard, h_guard = re.findall(guard_pattern, q_step), re.findall(guard_pattern, h_step)
    if len(q_guard) != 1 or len(h_guard) != 1:
        raise ValueError('candidate ordinary execution guard syntax is unsupported')
    # Finite Boolean grammar: P && F versus P && F && (candidate environment
    # variable == true). Whitespace/comment changes cannot alter this truth table.
    ordinary = r'elif\s+\[\[\s+"\$\{GITHUB_EVENT_NAME\}"\s+==\s+"pull_request"\s+&&\s+-f\s+"\$\{RUNNER_TEMP\}/impact-projection\.json"'
    added_guard = ordinary + r'\s+&&\s+"\$\{([A-Z][A-Z0-9_]*)\}"\s+==\s+"true"\s+\]\];\s+then'
    if not re.fullmatch(added_guard, h_guard[0].strip()):
        raise ValueError('candidate did not change the ordinary protected-driver execution guard')
    return VerifiedStrictException(_TOKEN, {**expected,
        'exception_rule': record['exception_rule'],
        'ordinary_limitation': 'Required independent orchestration cannot be supplied by candidate-controlled PR workflow, including its protected-driver job.',
        'constraint_evidence': {'comment_id': record_comment['id'], 'review_comment_ids': ids,
            'reviewed_projection_digest': next(iter(projections)),
            'constraint': record['constraint'], 'protected_workflow_sha256': EXECUTOR_SHA256,
            'candidate_control_step_sha256': hashlib.sha256(h_step.encode()).hexdigest()},
        'required_check_scope': record['required_check_scope'],
        'strict_capability': {'workflow_revision': target_oid, 'workflow': WORKFLOW,
            'execution': 'protected_Q_workflow_driver_on_merge_Q_H', 'tier': 'required'},
    })

def _role_binding(root, comment):
    """Reuse existing frozen collection and live packet-admission validators."""
    review = _record(comment.get('body'), REVIEW_MARKER)
    def path(field):
        relative = review.get(field)
        if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
            raise ValueError('professional exception evidence path is invalid')
        value = (Path(root) / relative).resolve(strict=True)
        value.relative_to(Path(root).resolve())
        return value
    plan_path, returned_path, packet_path, snapshot = (path(name) for name in
        ('review_plan_path', 'role_return_path', 'packet_path', 'bootstrap_snapshot_path'))
    spec = importlib.util.spec_from_file_location('strict_exception_review_plan', Path(__file__).with_name('review-plan.py'))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    plan, plan_digest, _ = helper.validate_prior_plan(Path(root), plan_path, review['task_uid'])
    returned_bytes = returned_path.read_bytes()
    returned_digest = hashlib.sha256(returned_bytes).hexdigest()
    returned = json.loads(returned_bytes)
    if (plan_digest != review.get('review_plan_sha256') or returned_digest != review.get('role_return_sha256')
            or plan.get('frozen_head') != review.get('source_head_oid')
            or returned.get('head') != plan.get('frozen_head')
            or returned.get('role') != review.get('role') or returned.get('status') != 'completed'
            or returned.get('epoch') != plan.get('epoch')
            or returned.get('admitted_packet_digest') != review.get('packet_digest')):
        raise ValueError('professional exception review immutable artifacts mismatch')
    adjudication = returned.get('strict_exception_adjudication')
    if adjudication != {key: review[key] for key in ('constraint_digest', 'conclusion', 'ordinary_supplements', 'required_check_scope', 'obligation')}:
        raise ValueError('professional return did not adjudicate this exact exception constraint')
    ledger_path = Path(plan['preflight']['ledger_path'])
    ledger = [json.loads(line) for line in ledger_path.read_text().splitlines() if line.strip()]
    matches = [entry for entry in ledger if entry.get('role') == review.get('role')
               and entry.get('artifact_digest') == returned_digest]
    if len(matches) != 1 or helper.resolve_collected_artifact(Path(root), ledger_path, matches[0]['artifacts'][0]) != returned_path:
        raise ValueError('exception return is not the collected professional role artifact')
    # Ordinary source-review collection is enough; exception evidence neither
    # attests CI nor requires unattended runtime attestation.
    script = Path(__file__).with_name('subagent-task-packet.py')
    completed = subprocess.run([sys.executable, '-I', '-B', str(script), 'review-admission',
        '--packet', str(packet_path), '--review-plan', str(plan_path), '--bootstrap-snapshot', str(snapshot)],
        cwd=root, capture_output=True, text=True)
    if completed.returncode:
        raise ValueError('exception professional packet not admitted: ' + completed.stderr[-1000:])
    admission = json.loads(completed.stdout)
    if (admission.get('status') != 'admitted' or admission.get('role') != review.get('role')
            or admission.get('packet_digest') != review.get('packet_digest')
            or admission.get('head') != review.get('source_head_oid')
            or admission.get('task_uid') != review.get('task_uid')
            or admission.get('slice_id') != returned.get('slice_id')):
        raise ValueError('professional exception packet admission binding mismatch')
    return {**admission, 'review_plan_sha256': plan_digest, 'role_return_sha256': returned_digest,
            'projection_digest': plan.get('impact_projection_digest')}

def read(repository, task_uid, issue, pr, target_oid, root):
    """Live producer; scope and evidence are independently read each time."""
    maintenance = _maintenance()
    _gh = maintenance._gh
    maintenance_comment_id, read_maintenance_scope = maintenance.maintenance_comment_id, maintenance.read_maintenance_scope
    ident = locator(pr.get('body'))
    if ident is None:
        return None
    spec = importlib.util.spec_from_file_location('strict_exception_identity', Path(__file__).with_name('ci_ready_receipt_identity.py'))
    identity = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(identity)
    selected = identity.select_strict_exception_locator(root=root, repository=repository,
        task_uid=task_uid, issue=issue, pr_number=pr['number'], source_head_oid=pr['head']['sha'],
        target_oid=target_oid, pr_body=pr.get('body'), github=_gh)
    if selected != ident:
        raise ValueError('strict exception is not independently selected current Task evidence')
    repository_state = _gh(f'repos/{repository}')
    branch = repository_state.get('default_branch')
    if ((pr.get('base') or {}).get('ref') != branch
            or _gh(f'repos/{repository}/commits/{branch}').get('sha') != target_oid):
        raise ValueError('strict exception target is not independently observed current protected Q')
    scope_id = maintenance_comment_id(pr.get('body', ''))
    if scope_id is None:
        raise ValueError('strict isolation requires authenticated ordinary maintenance scope')
    scope = read_maintenance_scope(repository, scope_id, task_uid, pr['number'], pr['head']['sha'])
    def comment(ident):
        value = _gh(f'repos/{repository}/issues/comments/{ident}')
        actor = (value.get('user') or {}).get('login')
        if not actor or not re.fullmatch(r'[A-Za-z0-9-]+', actor):
            raise ValueError('exception evidence actor is invalid')
        return value, _gh(f'repos/{repository}/collaborators/{actor}/permission')
    value, permission = comment(ident)
    record = _record(value.get('body'), MARKER)
    ids = record.get('review_comment_ids')
    if not isinstance(ids, list) or len(ids) != 2 or any(type(i) is not int or i < 1 for i in ids):
        raise ValueError('invalid exception review locators')
    def blob(oid):
        return subprocess.check_output(['git', '-C', str(root), 'show', oid + ':' + WORKFLOW])
    review_comments = [comment(i) for i in ids]
    bindings = {item['id']: _role_binding(root, item) for item, _ in review_comments}
    facts = verify(record_comment=value, record_permission=permission,
        reviews=review_comments, role_bindings=bindings, maintenance_scope=scope, task_uid=task_uid,
        repository=repository, issue=issue, pr=pr, target_oid=target_oid,
        source_workflow=blob(pr['head']['sha']), target_workflow=blob(target_oid))
    fresh = _gh(f'repos/{repository}/pulls/{pr["number"]}')
    if (fresh != pr or _gh(f'repos/{repository}/commits/{branch}').get('sha') != target_oid):
        raise ValueError('strict exception PR or protected target changed during verification')
    return facts

def read_protected(repository, task_uid, issue, pr, target_oid, root):
    """Select only installed protected Q bytes. Candidate producer cannot activate."""
    if locator(pr.get('body')) is None:
        return None
    with _protected_script(root, target_oid) as script:
        result = subprocess.run([sys.executable, '-I', '-B', str(script), '--read', repository,
            task_uid, str(issue), str(pr['number']), target_oid, str(root)], capture_output=True, text=True)
        if result.returncode:
            raise ValueError('protected strict exception verification failed: ' + result.stderr[-2000:])
        facts = json.loads(result.stdout)
        if facts.get('source_head_oid') != pr['head']['sha'] or facts.get('target_oid') != target_oid:
            raise ValueError('protected producer result changed subject identity')
        return VerifiedStrictException(_TOKEN, facts)

@contextmanager
def _protected_script(root, target_oid):
    relative = 'scripts/pm/strict_exception_facts.py'
    protected = subprocess.check_output(['git', '-C', str(root), 'show', target_oid + ':' + relative])
    if Path(__file__).read_bytes() != protected:
        raise ValueError('strict exception producer is not installed protected Q authority')
    with tempfile.TemporaryDirectory(prefix='strict-exception-protected-') as directory:
        archive = subprocess.check_output(['git', '-C', str(root), 'archive', target_oid, 'scripts'])
        with tarfile.open(fileobj=io.BytesIO(archive)) as entries:
            for member in entries.getmembers():
                if (not (member.isfile() or member.isdir()) or Path(member.name).is_absolute()
                        or '..' in Path(member.name).parts):
                    raise ValueError('protected strict producer archive is unsafe')
            entries.extractall(directory)
        yield Path(directory) / relative

def _adjacent(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

def ensure_verified(root, facts, projection_path, *, integration, receipt, store, github, recheck=None):
    """Existing unkeyed execution with a Task effect barrier, never CI authority.

    Called only inside the isolated protected live entry after facts/projection
    admission. Keeping the narrow operation injectable lets tests exercise real
    durable state, latest-attempt choices and dispatch effects without GitHub writes.
    """
    if type(facts) is not VerifiedStrictException:
        raise ValueError('ensure requires live protected exception facts')
    value = facts.as_dict()
    repo, uid, number, q, h = (value[key] for key in
        ('repository', 'task_uid', 'pr_number', 'target_oid', 'source_head_oid'))
    branch = github(f'repos/{repo}').get('default_branch')
    identity = {'repository': repo, 'task_uid': uid, 'pr_number': number,
                'source_head_oid': h, 'target_oid': q, 'required_check_scope': value['required_check_scope'],
                'protected_executor_sha256': value['constraint_evidence']['protected_workflow_sha256']}
    operation = hashlib.sha256(json.dumps(identity, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    common = Path(subprocess.check_output(['git', '-C', str(root), 'rev-parse', '--git-common-dir'], text=True).strip())
    if not common.is_absolute(): common = Path(root).resolve() / common
    journal = common.resolve() / 'oasis7-workflow-receipts' / uid / 'strict-exception-effects.json'
    with store.locked_json(journal, {'schema': 'oasis7-strict-exception-effects/v1', 'task_uid': uid, 'operations': {}}, write_back=False) as state:
        if state.get('schema') != 'oasis7-strict-exception-effects/v1' or state.get('task_uid') != uid or not isinstance(state.get('operations'), dict):
            raise ValueError('strict exception Task effect state malformed')
        prior = state['operations'].get(operation)
        if prior is not None and prior.get('identity') != identity:
            raise ValueError('strict exception dispatch intent identity mismatch')
        selected = integration.current_request(repo, uid, number, q, h, branch)
        if selected is not None:
            run = github(f'repos/{repo}/actions/runs/{selected["id"]}')
            if run.get('run_attempt') != selected.get('run_attempt'):
                raise ValueError('strict exception latest attempt changed during readback')
            if run.get('status') not in ('queued', 'in_progress', 'waiting', 'pending', 'requested', 'completed'):
                raise ValueError('strict exception current attempt status unreadable')
            if prior is not None:
                prior.update(status='observed', run_id=selected['id'], run_attempt=selected['run_attempt'])
                store.atomic_replace_json(journal, state)
            if run.get('status') != 'completed':
                return {'status': 'waiting', 'reason': 'current_required_attempt_pending', 'run_id': selected['id']}
            if run.get('conclusion') != 'success':
                return {'status': 'blocked', 'reason': 'latest_required_attempt_failed_or_cancelled', 'run_id': selected['id']}
            # This independently verifies the exact latest attempt's full proof,
            # required jobs, source/target/tree/workflow/App and full scope.
            policy = _adjacent('strict_exception_gate_policy', 'pr-lifecycle-gate.py')
            discovery = policy.discover_required_policy(repo, branch)
            pins = {item.get('app_id') for item in discovery.get('required_status_checks', ()) if item.get('context') == 'required-gate'}
            if discovery.get('status') != 'resolved' or len(pins) != 1 or type(next(iter(pins))) is not int:
                raise ValueError('strict exception protected required check App unresolved')
            check, proof = integration.verified_run(repo, uid, number, q, h, selected['id'], next(iter(pins)), expected_attempt=selected['run_attempt'])
            planned = receipt.planner_for_run(repo, check, base_oid=q, head_oid=h)
            if planned.get('scope') != 'full' or planned.get('impact_projection_test_profile') != 'full':
                raise ValueError('strict exception current proof does not cover required full scope')
            if integration.current_request(repo, uid, number, q, h, branch) != selected:
                raise ValueError('strict exception current request changed during verification')
            if recheck is not None and recheck().as_dict() != value:
                raise ValueError('strict exception live authority changed before proof reuse')
            return {'status': 'reused', 'run_id': selected['id'], 'run_attempt': selected['run_attempt']}
        if prior is not None:
            # Successful command completion also remains uncertain until exact
            # Actions readback. Never resend merely because it is not visible.
            raise ValueError('strict exception dispatch visibility unresolved; reconcile existing intent, never resend')
        if recheck is not None and recheck().as_dict() != value:
            raise ValueError('strict exception live authority changed before dispatch')
        state['operations'][operation] = {'identity': identity, 'status': 'dispatch_uncertain'}
        store.atomic_replace_json(journal, state)
        integration.dispatch(repo, uid, number, projection_path, expected_head=h, expected_target=q)
        return {'status': 'requested', 'reason': 'dispatch_waits_for_exact_readback', **identity}

def ensure_protected(repository, task_uid, issue, number, target_oid, root, projection_path):
    with _protected_script(root, target_oid) as script:
        result = subprocess.run([sys.executable, '-I', '-B', str(script), '--ensure-live', repository,
            task_uid, str(issue), str(number), target_oid, str(root), str(projection_path)], capture_output=True, text=True)
        if result.returncode:
            raise ValueError('protected strict request ensure blocked: ' + result.stderr[-2000:])
        return json.loads(result.stdout)

def _ensure_live(repository, uid, issue, number, q, root, projection_path):
    # Even direct invocation of the private entry cannot substitute neighboring
    # candidate helpers for the isolated Q closure.
    entries = subprocess.check_output(['git', '-C', str(root), 'ls-tree', '-r', q, '--', 'scripts/pm'], text=True).splitlines()
    directory = Path(__file__).resolve().parent
    for entry in entries:
        metadata, relative = entry.split('\t', 1)
        mode, kind, oid = metadata.split()
        path = directory / relative.removeprefix('scripts/pm/')
        if mode not in ('100644', '100755') or kind != 'blob' or path.is_symlink() or not path.is_file():
            raise ValueError('protected ensure helper closure mode/path invalid')
        if path.read_bytes() != subprocess.check_output(['git', '-C', str(root), 'cat-file', 'blob', oid]):
            raise ValueError('protected ensure helper closure bytes differ from Q')
    maintenance = _maintenance()
    pr = maintenance._gh(f'repos/{repository}/pulls/{number}')
    verified = read(repository, uid, issue, pr, q, root)
    if verified is None:
        raise ValueError('ensure has no current strict exception obligation')
    projection_helper = _adjacent('strict_exception_projection', 'workflow-impact-projection.py')
    base = subprocess.check_output(['git', '-C', str(root), 'merge-base', q, pr['head']['sha']], text=True).strip()
    changed = subprocess.check_output(['git', '-C', str(root), 'diff', '--name-only', base + '..' + pr['head']['sha']], text=True).splitlines()
    projection = projection_helper.load_verified_projection(projection_path,
        expected={'task_uid': uid, 'source_head_oid': pr['head']['sha'], 'scope_base_oid': base, 'changed_paths': changed}, repo_root=root)
    config = subprocess.check_output(['git', '-C', str(root), 'show', q + ':scripts/ci-required-scope.v2.json'])
    if projection.get('planner_config_sha256') != 'sha256:' + hashlib.sha256(config).hexdigest():
        raise ValueError('exception projection planner configuration differs from current protected Q')
    identity = _adjacent('strict_exception_ensure_identity', 'ci_ready_receipt_identity.py')
    sys.modules['oasis7_protected_strict_exception_facts'] = sys.modules[__name__]
    decision = identity.evaluate_strict_integration_requirement(trusted_projection=projection,
        expected_task_uid=uid, verified_exception=verified)
    if decision.get('status') != 'required':
        raise ValueError('protected ensure ordinary scope/exception admission blocked: ' + str(decision.get('reason')))
    integration = _adjacent('integration_ci', 'integration_ci.py')
    _adjacent('ci_ready_receipt_identity', 'ci_ready_receipt_identity.py')
    receipt = _adjacent('strict_exception_ensure_receipt', 'ci-ready-receipt.py')
    store = _adjacent('strict_exception_effect_store', 'workflow-durable-store.py')
    # Dispatch precisely the admitted bytes, never reread a mutable caller file
    # after validation or while waiting for the Task effect lock.
    with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', suffix='.json') as snapshot:
        json.dump(projection, snapshot, sort_keys=True, separators=(',', ':'))
        snapshot.flush()
        return ensure_verified(root, verified, snapshot.name, integration=integration, receipt=receipt,
                               store=store, github=maintenance._gh,
                               recheck=lambda: read(repository, uid, issue,
                                   maintenance._gh(f'repos/{repository}/pulls/{number}'), q, root))

def read_plan(plan, root=None):
    """Optional plan locator is independently matched to live PR evidence."""
    ident = plan.get('strict_exception_comment_id')
    if ident is None:
        return None
    if type(ident) is not int or ident < 1:
        raise ValueError('strict exception plan locator is malformed')
    _gh = _maintenance()._gh
    source = plan['source_review_identity']
    repository, number = source['repository'], source['pr_number']
    pr = _gh(f'repos/{repository}/pulls/{number}')
    if locator(pr.get('body')) != ident or pr['head']['sha'] != source['source_head_oid']:
        raise ValueError('strict exception plan locator or head differs from live PR')
    issues = re.findall(r'(?m)^Refs #([1-9][0-9]*)$', pr.get('body') or '')
    if len(issues) != 1:
        raise ValueError('strict exception plan lacks unique live Task Issue')
    branch = _gh(f'repos/{repository}').get('default_branch')
    q = _gh(f'repos/{repository}/commits/{branch}').get('sha')
    return read_protected(repository, source['task_uid'], int(issues[0]), pr, q,
                          root or Path(__file__).resolve().parents[2])

if __name__ == '__main__':
    if len(sys.argv) == 9 and sys.argv[1] in ('--ensure', '--ensure-live'):
        repo, uid, issue, number, q, root, projection = sys.argv[2:]
        result = (ensure_protected(repo, uid, int(issue), int(number), q, Path(root), Path(projection))
                  if sys.argv[1] == '--ensure' else _ensure_live(repo, uid, int(issue), int(number), q, Path(root), Path(projection)))
        print(json.dumps(result, sort_keys=True))
        raise SystemExit(0)
    if len(sys.argv) != 8 or sys.argv[1] != '--read':
        raise SystemExit('strict exception producer requires protected live reader invocation')
    repo, uid, issue, number, q, root = sys.argv[2:]
    pr = _maintenance()._gh(f'repos/{repo}/pulls/{int(number)}')
    result = read(repo, uid, int(issue), pr, q, Path(root))
    if result is None:
        raise SystemExit('protected strict exception locator absent')
    print(json.dumps(result.as_dict(), sort_keys=True))
