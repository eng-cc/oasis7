"""Single-host OS reservations and conservative, manual recovery observations.

The common-dir registry is coordination evidence, never task or remote truth.
Lock files are permanent: unlinking a flock file would permit two owners.
"""
import fcntl
import base64
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import tempfile


class Busy(RuntimeError):
    pass


def common_dir(root):
    value = subprocess.check_output(['git', '-C', str(root), 'rev-parse', '--git-common-dir'], text=True).strip()
    return (Path(root) / value).resolve()


def _directory(common):
    path = Path(common) / 'oasis7-loop-recovery'
    path.mkdir(parents=True, exist_ok=True)
    return path


def _key(uid):
    return hashlib.sha256(uid.encode()).hexdigest()


def _overlap(left, right):
    # Wildcard scopes conservatively reserve their static parent prefix.
    def prefix(value):
        indexes = [value.index(char) for char in '*?[' if char in value]
        if indexes:
            value = value[:min(indexes)]
            value = value.rsplit('/', 1)[0] if '/' in value else ''
        return value.rstrip('/')
    a, b = prefix(left), prefix(right)
    return not a or not b or a == b or a.startswith(b + '/') or b.startswith(a + '/')


class Reservation:
    """Keep this context alive for the entire controlled write operation."""
    def __init__(self, common, uid, scopes, *, recovery=False):
        self.directory = _directory(common)
        self.uid, self.scopes, self.handle = uid, scopes, None
        self.recovery = recovery

    def __enter__(self):
        with (self.directory / 'registry.lock').open('a+') as registry:
            fcntl.flock(registry, fcntl.LOCK_EX)
            for journal in self.directory.glob('*.actions.jsonl'):
                pending = _pending(journal)
                for action in pending:
                    same = action.get('task_uid') == self.uid or journal.stem.split('.')[0] == _key(self.uid)
                    overlaps = any(_overlap(a, b) for a in self.scopes for b in action.get('scopes', ['**']))
                    if (same or overlaps) and not (self.recovery and same):
                        raise Busy('unresolved action requires manual reconciliation before writer reuse')
            for path in self.directory.glob('*.lock'):
                if path.name == 'registry.lock': continue
                with path.open('a+') as candidate:
                    try:
                        fcntl.flock(candidate, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except BlockingIOError:
                        candidate.seek(0)
                        try: record = json.load(candidate)
                        except (ValueError, OSError): raise Busy('active reservation has unreadable identity')
                        if record['task_uid'] == self.uid or any(_overlap(a, b) for a in self.scopes for b in record['scopes']):
                            raise Busy('task or write scope has an active OS lock')
            self.handle = (self.directory / (_key(self.uid) + '.lock')).open('a+')
            fcntl.flock(self.handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.handle.seek(0)
            self.handle.truncate()
            json.dump({'task_uid': self.uid, 'scopes': self.scopes, 'pid': os.getpid()}, self.handle)
            self.handle.flush()
        return self

    def __exit__(self, *args):
        if self.handle:
            self.handle.close()


def record_action(common, uid, action):
    if not all(action.get(key) for key in ('action_id', 'kind', 'expected')):
        raise ValueError('action requires action_id, kind, expected remote identity')
    action = dict(action)
    action['task_uid'] = uid
    directory = _directory(common)
    if 'scopes' not in action:
        lock = directory / (_key(uid) + '.lock')
        action['scopes'] = json.loads(lock.read_text()).get('scopes', ['**']) if lock.exists() else ['**']
    if action['kind'] == 'child_process' and not action.get('reconciled'):
        action.setdefault('process_identity', process_identity(action['expected']))
    path = directory / (_key(uid) + '.actions.jsonl')
    with path.open('a') as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        handle.write(json.dumps(action, sort_keys=True) + '\n')
        handle.flush()
        os.fsync(handle.fileno())


def process_identity(value):
    """Observe PID plus kernel start information; absence never authorizes restart."""
    try:
        pid = int(value)
        if pid <= 0: raise ValueError('child PID must be positive')
        os.kill(pid, 0)
        observed = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart='], capture_output=True, text=True)
        return {'pid': pid, 'start': observed.stdout.strip() or None}
    except ProcessLookupError:
        return {'pid': int(value), 'absent': True}
    except PermissionError:
        return {'pid': int(value), 'start': None}


def _pending(path):
    try:
        actions = [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []
        pending = {}
        for item in actions:
            if not all(item.get(k) for k in ('action_id', 'kind', 'expected')):
                raise ValueError('incomplete action')
            key = item['action_id']
            previous = pending.get(key)
            if item.get('reconciled'):
                if previous is None or not item.get('readback_evidence'):
                    raise ValueError('resolution lacks original action/readback')
                identity = lambda x: {k: v for k, v in x.items() if k not in ('reconciled', 'readback_evidence')}
                if identity(previous) != identity(item):
                    raise ValueError('resolution action identity mismatch')
                del pending[key]
            elif previous is not None and previous != item:
                raise ValueError('conflicting action identity')
            else:
                pending[key] = item
        return list(pending.values())
    except (ValueError, TypeError, KeyError, OSError) as exc:
        raise Busy('unreadable or conflicting recovery journal; reconcile_required: ' + str(exc)) from exc


def recovery_status(common, uid):
    path = _directory(common) / (_key(uid) + '.actions.jsonl')
    pending = _pending(path)
    active = []
    for item in pending:
        if item.get('kind') == 'child_process':
            try:
                os.kill(int(item['expected']), 0)
                active.append(item)
            except ProcessLookupError:
                continue
            except (ValueError, PermissionError):
                active.append(item)
    # Do not use TTL or a dead parent PID as permission to replace a child.
    return {'status': 'reconcile_required' if pending else 'can_continue', 'pending_actions': pending,
            'active_or_unverified_children': active, 'automatic_replay': False,
            'next_step': 'Read back each recorded remote object or confirm old child termination using its existing action journal; do not repeat the operation.' if pending else None}


def reconcile(common, uid, root, tool_root=None, *, reservation_fd=None):
    """Read back known push/child effects; unknown operations stay blocked."""
    for action in recovery_status(common, uid)['pending_actions']:
        evidence = None
        if action['kind'] == 'push':
            # Only the repository's existing origin and exact named ref are queried.
            ref = action.get('remote_ref', '')
            if not ref.startswith('refs/heads/') or any(c.isspace() for c in ref): continue
            output = subprocess.check_output(['git', '-C', str(root), 'ls-remote', '--refs', 'origin', ref], text=True).strip()
            if output.split('\t') == [action['expected'], ref]: evidence = output
        elif action['kind'] == 'child_process':
            try: os.kill(int(action['expected']), 0)
            except ProcessLookupError: evidence = 'kernel reports process absent; no restart performed'
            except (ValueError, PermissionError): pass
        elif action['kind'] == 'bind_loop':
            issue = json.loads(subprocess.check_output(['gh', 'api', f"repos/{action['repository']}/issues/{action['issue_number']}"], text=True))
            matches = re.findall(r'^- loop_binding_b64: `([^`]+)`$', issue.get('body', ''), re.MULTILINE)
            if len(matches) == 1:
                binding = json.loads(base64.b64decode(matches[0] + '=' * (-len(matches[0]) % 4), altchars=b'-_', validate=True))
                if binding == json.loads(action['expected']):
                    if tool_root is None:
                        continue
                    import sys
                    tool = Path(tool_root)
                    # The facade has validated stable identity and the journal's
                    # exact old/new transition. Resume the idempotent sanctioned
                    # writer to finish Project/cache/snapshot/lineage; generic
                    # refresh correctly refuses a partially migrated epoch.
                    with tempfile.NamedTemporaryFile(mode='w', suffix='.json') as intent:
                        json.dump(binding, intent); intent.flush()
                        command = [sys.executable, str(tool / 'scripts/pm/github-project-task.py'), 'bind-loop', str(root), '--task-uid', uid,
                                   '--loop-binding', intent.name, '--manual-request-ref', binding['manual_request_ref'], '--json']
                        if action.get('previous_binding', binding) != binding:
                            command += ['--migrate-epoch', str(binding['bootstrap_epoch'])]
                        repaired = subprocess.run(command, capture_output=True, text=True, pass_fds=() if reservation_fd is None else (reservation_fd,))
                    if repaired.returncode:
                        continue
                    refreshed = subprocess.run([sys.executable, str(tool / 'scripts/pm/github-project-task.py'), 'refresh-task', str(root), '--task-uid', uid, '--json'], capture_output=True, text=True, pass_fds=() if reservation_fd is None else (reservation_fd,))
                    if refreshed.returncode:
                        continue
                    mapping = json.loads((Path(root) / '.pm/github-project-sync/tasks.json').read_text())['tasks'][uid]
                    if mapping.get('loop_binding') != binding or mapping.get('bootstrap_epoch') != binding['bootstrap_epoch']:
                        continue
                    snapshot_path = Path(root) / '.pm/scratch' / uid / 'bootstrap-task-snapshot.json'
                    saved = json.loads(snapshot_path.read_text())
                    checked = subprocess.run([sys.executable, str(tool / 'scripts/pm/bootstrap-task-snapshot.py'), 'validate-epoch-identity', '--repo-root', str(root), '--task-uid', uid, '--request-identity', saved['request']['identity']], capture_output=True, text=True)
                    if checked.returncode == 0:
                        evidence = {'issue_number': action['issue_number'], 'project_cache_refresh': json.loads(refreshed.stdout), 'snapshot_validation': checked.stdout.strip()}
        elif action['kind'] == 'publish_contract':
            from loop_contracts import GitHubAuthority, MARKER, REPOSITORY, contract_digest, validate_contracts
            binding = action.get('binding')
            if not isinstance(binding, dict) or binding.get('task_uid') != uid or action.get('repository') != REPOSITORY:
                continue
            authority = GitHubAuthority(root)
            number = action['issue_number']
            authority.issue(number, uid)
            expected = json.loads(action['expected'])
            matches = []
            for page in authority.api(f'repos/{REPOSITORY}/issues/{number}/comments?per_page=100', paginate=True):
                for comment in page:
                    try: payload = json.loads(comment.get('body') or '')
                    except (ValueError, TypeError): continue
                    if not isinstance(payload, dict) or payload.get('marker') != MARKER: continue
                    contract = payload.get('contract')
                    if not isinstance(contract, dict): continue
                    if (contract.get('contract_id'), contract.get('revision')) == (expected.get('contract_id'), expected.get('revision')):
                        matches.append((comment, payload))
            if len(matches) == 1:
                comment, payload = matches[0]
                if payload.get('task_uid') != uid or payload.get('contract') != expected or payload.get('contract_digest') != contract_digest(expected):
                    continue
                reference = {'contract_id': expected['contract_id'], 'revision': expected['revision'],
                             'contract_digest': contract_digest(expected),
                             'publication_ref': {'issue_number': number, 'comment_id': comment['id']},
                             'consumed_clauses': [c for ref in expected['content_refs'] for c in ref['clauses']]}
                checked = validate_contracts(root, root, {**binding, 'input_contracts': [reference]}, authority, purpose='new_tasks')
                if checked['status'] == 'passed': evidence = {'publication_ref': reference['publication_ref'], 'validation': checked}
        if evidence:
            record_action(common, uid, {**action, 'reconciled': True, 'readback_evidence': evidence})
    return recovery_status(common, uid)
