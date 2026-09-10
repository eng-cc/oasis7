#!/usr/bin/env python3
"""Manual bootstrap identity and fixed-base preflight; never starts a model."""
import argparse
import hashlib
import importlib.util
import json
import os
import re
from pathlib import Path
import subprocess
import sys

def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

def fetch_base(root):
    # Remote's advertised default, never the caller's current HEAD.
    advertised = git(root, 'ls-remote', '--symref', 'origin', 'HEAD')
    branches = [line.split()[1] for line in advertised.splitlines() if line.startswith('ref: refs/heads/')]
    if len(branches) != 1:
        raise ValueError('cannot establish remote default branch')
    # Pin the OID from the same advertisement as the default-branch identity.
    # FETCH_HEAD is shared with other requests and historical contract fetches.
    heads = [line.split()[0] for line in advertised.splitlines()
             if re.fullmatch(r'[0-9a-f]{40}(?:[0-9a-f]{24})?\s+HEAD', line)]
    if len(heads) != 1:
        raise ValueError('cannot establish exact remote default branch OID')
    oid = heads[0]
    git(root, 'fetch', '--no-write-fetch-head', '--no-tags', 'origin', oid)
    if git(root, 'rev-parse', oid + '^{commit}') != oid:
        raise ValueError('fetched default branch object does not match advertised OID')
    return oid

def prepare_request(journal_root, request, root):
    path = journal_root / (hashlib.sha256(request['request_key'].encode()).hexdigest() + '.json')
    store = load('workflow-durable-store')
    with store.locked_json(path, {}) as saved:
        if saved:
            if saved.get('request') != request:
                raise ValueError('manual bootstrap request identity drift; reuse original request or explicitly start another')
            return saved['base_oid']
        base = fetch_base(root)
        saved.update(request=request, base_oid=base)
        return base

def preparation_purpose(common, request, repository, task):
    # A pinned base precedes worktree/Issue creation. Only a completed creation
    # journal plus matching live task proves this is an in-flight continuation.
    scratch = os.environ.get('OASIS7_PM_TEST_SCRATCH', '')
    if scratch and not Path(scratch).is_absolute():
        raise ValueError('OASIS7_PM_TEST_SCRATCH must be absolute')
    directory = Path(scratch) / 'bootstrap-journal' if scratch else common / 'oasis7-bootstrap-journal'
    key = hashlib.sha256('\0'.join((repository, 'manual-request', request['request_key'])).encode()).hexdigest()
    path = directory / (key + '.json')
    journal = json.loads(path.read_text()) if path.exists() else {}
    if journal.get('state') != 'completed':
        return 'new_tasks'
    binding = request['binding']
    recorded = journal.get('immutable_request') or journal.get('request') or {}
    if (journal.get('task_uid') != binding['task_uid'] or recorded.get('repo') != repository
            or recorded.get('request_key') != request['request_key'] or recorded.get('loop_binding') != binding
            or recorded.get('worktree_hint') != request['worktree']):
        raise ValueError('completed manual bootstrap creation request mismatch')
    live = task.github_issue_record(repository, binding['task_uid'])
    if not live or live.get('loop_binding') != binding or live.get('worktree_hint') != request['worktree']:
        raise ValueError('completed manual bootstrap live binding/worktree mismatch')
    return 'in_flight'

def complete_bootstrap(args, task, target, live, refresh):
    """Finish proven creation only; uncertain remote phases require readback."""
    binding = live.get('loop_binding')
    if not binding:
        raise ValueError('missing bootstrap snapshot needs a proven manual creation')
    common = (args.root / Path(git(args.root, 'rev-parse', '--git-common-dir'))).resolve()
    scratch = os.environ.get('OASIS7_PM_TEST_SCRATCH', '')
    if scratch and not Path(scratch).is_absolute(): raise ValueError('test scratch must be absolute')
    directory = Path(scratch) / 'bootstrap-journal' if scratch else common / 'oasis7-bootstrap-journal'
    key = hashlib.sha256('\0'.join((args.repository, 'manual-request', binding['request_key'])).encode()).hexdigest()
    creation = json.loads((directory / (key + '.json')).read_text())
    request = creation.get('immutable_request') or {}
    digest = hashlib.sha256(json.dumps(request, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
    pin_key = hashlib.sha256(binding['request_key'].encode()).hexdigest()
    pin = json.loads((common / 'oasis7-loop-bootstrap-requests' / (pin_key + '.json')).read_text())
    expected_pin = {'request_key': binding['request_key'], 'binding': binding,
                    'worktree': str(target), 'branch': git(target, 'symbolic-ref', '--short', 'HEAD')}
    if (creation.get('state') != 'completed' or creation.get('task_uid') != args.task_uid
            or creation.get('immutable_request_digest') != digest
            or request.get('repo') != args.repository or request.get('loop_binding') != binding
            or request.get('worktree_hint') != str(target) or request.get('owner_role') != live.get('owner_role')
            or creation.get('issue_url') != live.get('issue_url')
            or pin.get('request') != expected_pin or pin.get('base_oid') != request.get('bootstrap_base_oid')):
        raise ValueError('incomplete bootstrap creation identity is not proven')
    mapping = json.loads((target / '.pm/github-project-sync/tasks.json').read_text())['tasks'][args.task_uid]
    if (mapping.get('project_item_id') != creation.get('project_item_id')
            or mapping.get('task_branch') != git(target, 'symbolic-ref', '--short', 'HEAD')
            or mapping.get('bootstrap_base_oid') != request.get('bootstrap_base_oid')
            or mapping.get('status') not in ('candidate', 'committed')
            or live.get('status') != mapping.get('status')):
        raise ValueError('incomplete bootstrap live lifecycle identity drift')
    store = load('workflow-durable-store')
    path = directory / (key + '.lifecycle.json')
    with store.locked_json(path, {}) as state:
        identity = {'task_uid': args.task_uid, 'creation_digest': digest}
        if state and state.get('identity') != identity:
            raise ValueError('bootstrap lifecycle journal identity drift')
        state['identity'] = identity
        if state.get('snapshot') == 'completed':
            raise ValueError('completed bootstrap snapshot disappeared; explicit recovery required')
        def mark(stage, value):
            state[stage] = value
            store.atomic_replace_json(path, state)
        def invoke(*arguments):
            subprocess.run([sys.executable, str(Path(__file__).with_name('github-project-task.py')),
                            *arguments, str(target), '--repo', args.repository,
                            '--task-uid', args.task_uid, '--json'], check=True, stdout=subprocess.PIPE)
        if mapping['status'] == 'candidate':
            if state.get('move') in ('attempted', 'completed'):
                raise ValueError('bootstrap move outcome uncertain; reconcile before retry')
            mark('move', 'attempted')
            invoke('move-task', '--to-status', 'committed')
            subprocess.run(refresh, check=True, stdout=subprocess.PIPE)
        mark('move', 'completed')
        pages = json.loads(subprocess.check_output(['gh', 'api',
            f"repos/{args.repository}/issues/{live['issue_number']}/comments", '--paginate', '--slurp'], text=True))
        if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
            raise ValueError('bootstrap start evidence pagination unavailable')
        starts = []
        for page in pages:
            for comment in page:
                body = str(comment.get('body') or '').replace('\r\n', '\n')
                fields = dict(line.split(': ', 1) for line in body.splitlines() if ': ' in line)
                if '<!-- oasis7-pm-evidence -->' in body and fields.get('Task UID') == args.task_uid:
                    if fields.get('Evidence Phase') != 'start':
                        raise ValueError('task advanced beyond incomplete bootstrap')
                    expected = {'Role': live['owner_role'], 'Issue': live['issue_url'],
                                'Worktree': str(target), 'Task Status': 'committed'}
                    if any(fields.get(k) != v for k, v in expected.items()):
                        raise ValueError('bootstrap start evidence identity drift')
                    starts.append(comment)
        if len(starts) > 1: raise ValueError('bootstrap start evidence ambiguous')
        if not starts:
            if state.get('start') in ('attempted', 'completed'):
                raise ValueError('bootstrap start outcome uncertain; reconcile before retry')
            mark('start', 'attempted')
            invoke('workflow-report', '--phase', 'start', '--role', live['owner_role'])
        mark('start', 'completed')
        subprocess.run([sys.executable, str(Path(__file__).with_name('bootstrap-task-snapshot.py')),
                        'validate-or-create', '--repo-root', str(target), '--task-uid', args.task_uid,
                        '--producer', 'scripts/new-task-worktree.sh'], check=True, stdout=subprocess.PIPE)
        mark('snapshot', 'completed')

def resume(args):
    task = load('github-project-task')
    live = task.github_issue_record(args.repository, args.task_uid)
    if not live:
        raise ValueError('selected existing task is not uniquely readable')
    target = Path(live.get('worktree_hint') or '').resolve()
    if not target.is_dir():
        raise ValueError('canonical worktree missing; selected task recovery required')
    root_common = Path(git(args.root, 'rev-parse', '--git-common-dir'))
    target_common = Path(git(target, 'rev-parse', '--git-common-dir'))
    if (args.root / root_common).resolve() != (target / target_common).resolve():
        raise ValueError('existing task belongs to another clone/common-dir')
    if live.get('status') in {'done', 'deferred'}:
        raise ValueError('terminal task cannot be bootstrapped again')
    binding = live.get('loop_binding')
    if binding is not None:
        task.validate_loop_inputs(target, binding, args.repository, 'in_flight')
    if args.binding:
        supplied = json.loads(Path(args.binding).read_text())
        if supplied != binding:
            raise ValueError('existing task frozen binding mismatch; explicit epoch migration required')
    if args.loop and (binding or {}).get('loop') != args.loop:
        raise ValueError('existing task loop mismatch; no implicit legacy migration')
    command = [sys.executable, str(Path(__file__).with_name('github-project-task.py')), 'refresh-task',
               str(target), '--repo', args.repository, '--task-uid', args.task_uid, '--json']
    subprocess.run(command, check=True, stdout=subprocess.PIPE)
    snapshot_path = target / '.pm/scratch' / args.task_uid / 'bootstrap-task-snapshot.json'
    if not snapshot_path.exists():
        complete_bootstrap(args, task, target, task.github_issue_record(args.repository, args.task_uid), command)
    snapshot = json.loads(snapshot_path.read_text())
    subprocess.run([sys.executable, str(Path(__file__).with_name('bootstrap-task-snapshot.py')), 'validate-epoch-identity',
                    '--repo-root', str(target), '--task-uid', args.task_uid,
                    '--request-identity', snapshot['request']['identity']], check=True, stdout=subprocess.PIPE)
    print(json.dumps({'mode': 'resume_existing_task', 'worktree_path': str(target),
                      'branch': git(target, 'symbolic-ref', '--short', 'HEAD'),
                      'pm': {'task_uid': args.task_uid, 'task_path': live['issue_url'], 'loop_binding': binding}}))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['prepare', 'resume'])
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--binding')
    parser.add_argument('--loop', choices=['product', 'system', 'code'])
    parser.add_argument('--request-key')
    parser.add_argument('--manual-request-ref', required=True)
    parser.add_argument('--task-uid')
    parser.add_argument('--worktree')
    parser.add_argument('--branch')
    parser.add_argument('--repository', default='eng-cc/oasis7')
    args = parser.parse_args()
    if not args.manual_request_ref.strip():
        raise ValueError('explicit manual request reference is required')
    if args.command == 'resume':
        if not args.task_uid: raise ValueError('selected task UID required')
        resume(args)
        return
    if not all((args.binding, args.loop, args.request_key, args.worktree, args.branch)):
        raise ValueError('loop bootstrap needs binding, loop, request key, worktree and branch')
    binding = json.loads(Path(args.binding).read_text())
    if not isinstance(binding, dict): raise ValueError('binding must be an object')
    if binding['loop'] != args.loop or binding['request_key'] != args.request_key or binding['manual_request_ref'] != args.manual_request_ref:
        raise ValueError('manual request differs from frozen binding')
    common = (args.root / Path(git(args.root, 'rev-parse', '--git-common-dir'))).resolve()
    request = dict(request_key=args.request_key, binding=binding, worktree=str(Path(args.worktree).resolve()), branch=args.branch)
    task = load('github-project-task')
    task.validate_loop_inputs(args.root, binding, args.repository, preparation_purpose(common, request, args.repository, task))
    print(prepare_request(common / 'oasis7-loop-bootstrap-requests', request, args.root))

if __name__ == '__main__':
    try: main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f'loop-bootstrap: {exc}', file=sys.stderr)
        raise SystemExit(1)
