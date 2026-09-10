#!/usr/bin/env python3
"""Manual single-task facade. No task discovery, scheduler, or action replay."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

from loop_recovery import Busy, Reservation, common_dir, recovery_status, reconcile, record_action
from loop_gate import live_binding


def _git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def load_task(root, uid, *, recovery=False):
    if not re.fullmatch(r'task_[0-9a-f]{32}', uid):
        raise ValueError('invalid task UID')
    task = json.loads((root / '.pm/github-project-sync/tasks.json').read_text())['tasks'][uid]
    if task.get('task_uid') != uid:
        raise ValueError('mapping task identity mismatch')
    if Path(task.get('canonical_worktree') or task.get('worktree_path') or task.get('worktree') or '').resolve() != root.resolve():
        # Existing mapping uses worktree_hint as the canonical path.
        if Path(task.get('worktree_hint') or '').resolve() != root.resolve():
            raise ValueError('canonical worktree mismatch')
    if task.get('task_branch') and task['task_branch'] != _git(root, 'branch', '--show-current'):
        raise ValueError('canonical branch mismatch')
    if not recovery:
        binding = live_binding(task)
        if binding != task.get('loop_binding'):
            raise ValueError('live binding differs from cache; refresh-task before continuing')
    return task


def recovery_task(root, task, tool_root):
    """Permit only a recorded old/new binding transition, never generic drift."""
    uid = task['task_uid']
    pending = recovery_status(common_dir(root), uid)['pending_actions']
    transitions = [a for a in pending if a['kind'] == 'bind_loop']
    if not transitions:
        observed = load_task(root, uid)
        if observed.get('loop_binding') is not None:
            recovery_authority(root, observed['loop_binding'], tool_root)
        return observed
    if len(transitions) != 1 or len(pending) != 1:
        raise ValueError('ambiguous pending binding transition')
    action = transitions[0]
    binding = json.loads(action['expected'])
    if action['action_id'] != 'bind:' + hashlib.sha256(action['expected'].encode()).hexdigest():
        raise ValueError('binding journal action digest mismatch')
    if any(action.get(k) != task.get(k) for k in ('task_uid', 'repository', 'issue_number')):
        raise ValueError('binding journal stable task identity mismatch')
    previous = action.get('previous_binding', binding)
    old_epoch = action.get('previous_epoch', binding['bootstrap_epoch'])
    if binding.get('task_uid') != uid or binding.get('owner_role') != task.get('owner_role'):
        raise ValueError('binding journal owner/task mismatch')
    if previous != binding and binding['bootstrap_epoch'] != old_epoch + 1:
        raise ValueError('binding journal must advance exactly one epoch')
    if previous is not None and previous.get('bootstrap_epoch') != old_epoch:
        raise ValueError('binding journal previous epoch mismatch')
    if action.get('canonical_worktree', str(root)) != str(root) or action.get('task_branch', task.get('task_branch')) != task.get('task_branch'):
        raise ValueError('binding journal worktree/branch drift')
    if action.get('project_item_id', task.get('project_item_id')) != task.get('project_item_id'):
        raise ValueError('binding journal Project identity drift')
    if task.get('bootstrap_epoch', old_epoch) not in (old_epoch, binding['bootstrap_epoch']):
        raise ValueError('cached epoch outside binding journal')
    for observed in (task.get('loop_binding'), live_binding(task)):
        if observed not in (previous, binding):
            raise ValueError('binding drift outside journal old/new transition')
    for path in (root / '.pm/scratch' / uid / 'bootstrap-task-snapshot.json', common_dir(root) / 'oasis7-loop-lineage' / (uid + '.json')):
        if path.exists():
            saved = json.loads(path.read_text())
            observed = saved.get('task', saved).get('loop_binding')
            if observed not in (previous, binding):
                raise ValueError('snapshot/lineage drift outside binding journal')
    recovery_authority(root, binding, tool_root)
    return {**task, 'loop_binding': binding, 'bootstrap_epoch': binding['bootstrap_epoch']}


def recovery_authority(root, binding, tool_root):
    if tool_root is None:
        raise ValueError('recovery requires effective trusted --tool-root')
    policy = _trusted_module(tool_root, root, binding, 'loop_policy')
    blockers = policy.validate_binding(binding)['blockers'] + policy.validate_tool_root(tool_root, root, binding)['blockers']
    if blockers: raise ValueError('; '.join(blockers))
    _trusted_module(tool_root, root, binding, 'loop_contracts')


def _trusted_module(root, target, binding, name):
    commit = binding.get('policy_commit', '')
    if not re.fullmatch(r'[0-9a-f]{40}', commit):
        raise ValueError('missing immutable effective policy_commit')
    if _git(root, 'rev-parse', 'HEAD') != commit:
        raise ValueError('tool root HEAD is not effective policy_commit')
    subprocess.run(['git', '-C', str(target), 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main'], check=True, capture_output=True)
    if common_dir(root) != common_dir(target):
        raise ValueError('tool root belongs to another repository')
    # Check every executable dependency before importing any candidate-controlled code.
    files = _git(target, 'ls-tree', '-r', '--name-only', commit, '--', 'scripts/pm').splitlines()
    for relative in files:
        if not relative.endswith(('.py', '.sh', '.json')): continue
        expected = subprocess.check_output(['git', '-C', str(target), 'show', commit + ':' + relative])
        if (root / relative).is_symlink() or (root / relative).read_bytes() != expected:
            raise ValueError('effective helper bytes differ: ' + relative)
    tracked = set(files)
    for path in (root / 'scripts/pm').glob('*.py'):
        if str(path.relative_to(root)) not in tracked:
            raise ValueError('untracked executable in trusted tool root')
    sys.path.insert(0, str(root / 'scripts/pm'))
    path = root / 'scripts/pm' / (name + '.py')
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def dependency_issue(repository, uid):
    # Search results are locators, not identity; a full window is not complete.
    hits = json.loads(subprocess.check_output(['gh', 'issue', 'list', '-R', repository, '--state', 'all', '--search', uid + ' in:body', '--json', 'number', '--limit', '100'], text=True))
    if not isinstance(hits, list) or len(hits) >= 100:
        raise ValueError('dependency discovery exceeds bounded admission limit')
    matches, seen = [], set()
    for hit in hits:
        number = hit.get('number') if isinstance(hit, dict) else None
        if type(number) is not int or number < 1 or number in seen:
            raise ValueError('dependency discovery identity unavailable or duplicated')
        seen.add(number)
        issue = json.loads(subprocess.check_output(['gh', 'api', f'repos/{repository}/issues/{number}'], text=True))
        if (not isinstance(issue, dict) or issue.get('number') != number
                or issue.get('html_url') != f'https://github.com/{repository}/issues/{number}'
                or not isinstance(issue.get('body'), str) or 'pull_request' in issue):
            raise ValueError('dependency Issue readback identity unavailable')
        fields = re.findall(r'^task_uid:[^\n]*$', issue['body'].replace('\r\n', '\n'), re.MULTILINE)
        if not fields: continue  # Ordinary mentions do not establish task identity.
        if len(fields) != 1 or not re.fullmatch(r'task_uid: task_[0-9a-f]{32}', fields[0]):
            raise ValueError('dependency Issue canonical UID missing or ambiguous')
        if fields == ['task_uid: ' + uid]: matches.append(number)
    if len(matches) != 1:
        raise ValueError('dependency task missing or ambiguous: ' + uid)
    return matches[0]


def validate_task(root, task, tool_root, base=None, head=None, contracts=True, purpose='in_flight'):
    if 'loop_binding' not in task or task['loop_binding'] is None:
        return {'status': 'legacy', 'blockers': []}
    try:
        if tool_root is None:
            raise ValueError('activation prerequisite: explicit trusted --tool-root required')
        binding = task['loop_binding']
        policy = _trusted_module(tool_root, root, binding, 'loop_policy')
        blockers = list(policy.validate_binding(binding).get('blockers', []))
        blockers += policy.validate_tool_root(tool_root, root, binding).get('blockers', [])
        closure = {binding['task_uid']: binding}
        pending = list(binding.get('dependencies', []))
        while pending:
            uid = pending.pop()
            if uid in closure: continue
            if len(closure) > 100: raise ValueError('dependency closure exceeds bounded admission limit')
            repository = task.get('repository')
            if not repository: raise ValueError('dependency validation requires live repository identity')
            number = dependency_issue(repository, uid)
            terminal = _trusted_module(tool_root, root, binding, 'loop_terminal')
            completed = terminal.validate_terminal_delivery(repository, uid, number, repo_root=root)
            if completed['status'] != 'passed':
                raise ValueError('dependency is not successfully completed: ' + uid + ': ' + '; '.join(completed['blockers']))
            dependency = live_binding({'repository': repository, 'issue_number': number, 'task_uid': uid})
            if dependency is None: raise ValueError('dependency lacks immutable loop binding: ' + uid)
            closure[uid] = dependency
            pending.extend(dependency.get('dependencies', []))
        blockers += policy.validate_dependencies(binding, closure).get('blockers', [])
        for field in ('task_uid', 'owner_role', 'bootstrap_epoch'):
            if binding.get(field) != task.get(field): blockers.append('loop/task identity mismatch: ' + field)
        if base is not None:
            blockers += policy.validate_scope(tool_root, root, binding, base, head or _git(root, 'rev-parse', 'HEAD')).get('blockers', [])
        if contracts:
            validator = _trusted_module(tool_root, root, binding, 'loop_contracts')
            blockers += validator.validate_contracts(tool_root, root, binding, purpose=purpose).get('blockers', [])
        return {'status': 'blocked' if blockers else 'passed', 'blockers': blockers,
                'loop_binding': binding, 'execution_scope': 'execution_scope_unverified'}
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        return {'status': 'blocked', 'blockers': [str(exc)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['doctor', 'bind', 'status', 'resume-check', 'recover', 'validate-scope', 'validate-contracts', 'publish-contract'])
    parser.add_argument('--repo-root', type=Path, default=Path.cwd())
    parser.add_argument('--tool-root', type=Path)
    parser.add_argument('--task-uid', required=True)
    parser.add_argument('--manual-request-ref')
    parser.add_argument('--loop-binding', type=Path)
    parser.add_argument('--contract', type=Path)
    parser.add_argument('--migrate-epoch', type=int)
    parser.add_argument('--base')
    parser.add_argument('--head')
    parser.add_argument('--json', action='store_true')
    args = parser.parse_args()
    try:
        root = args.repo_root.resolve()
        task = load_task(root, args.task_uid, recovery=args.command == 'recover')
        if args.tool_root and (task.get('loop_binding') is not None or args.command == 'bind'):
            subprocess.run(['git', '-C', str(root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'], check=True, capture_output=True)
        if args.command in ('bind', 'resume-check', 'recover', 'publish-contract') and not args.manual_request_ref:
            raise ValueError('explicit current manual request reference required')
        if args.command == 'bind':
            if not args.loop_binding: raise ValueError('--loop-binding required')
            binding = json.loads(args.loop_binding.read_text())
            if binding.get('manual_request_ref') != args.manual_request_ref:
                raise ValueError('manual request does not match proposed binding')
            proposed = {**task, 'loop_binding': binding}
            if args.migrate_epoch is not None:
                if args.migrate_epoch != int(task.get('bootstrap_epoch', 1)) + 1:
                    raise ValueError('migration must advance exactly one bootstrap epoch')
                proposed['bootstrap_epoch'] = args.migrate_epoch
            result = validate_task(root, proposed, args.tool_root, contracts=True, purpose='in_flight' if task.get('loop_binding') == binding else 'new_tasks')
            if result['status'] != 'passed': raise ValueError('; '.join(result['blockers']))
            command = [sys.executable, str(args.tool_root / 'scripts/pm/github-project-task.py'), 'bind-loop', str(root), '--task-uid', args.task_uid, '--loop-binding', str(args.loop_binding.resolve()), '--manual-request-ref', args.manual_request_ref, '--json']
            if args.migrate_epoch is not None: command += ['--migrate-epoch', str(args.migrate_epoch)]
            with Reservation(common_dir(root), args.task_uid, binding['write_scope']) as reservation:
                expected = json.dumps(binding, sort_keys=True)
                action = {'action_id': 'bind:' + hashlib.sha256(expected.encode()).hexdigest(), 'kind': 'bind_loop', 'expected': expected,
                          'repository': task['repository'], 'issue_number': task['issue_number'],
                          'previous_binding': task.get('loop_binding'), 'previous_epoch': task.get('bootstrap_epoch', 1),
                          'canonical_worktree': str(root), 'task_branch': task.get('task_branch'), 'project_item_id': task.get('project_item_id')}
                command += ['--loop-action-json', json.dumps(action, sort_keys=True)]
                result = json.loads(subprocess.check_output(command, text=True, pass_fds=(reservation.handle.fileno(),)))
                if result.get('status') == 'bound':
                    record_action(common_dir(root), args.task_uid, {**action, 'reconciled': True, 'readback_evidence': result})
        else:
            if args.command == 'validate-scope' and not args.base: raise ValueError('--base required')
            if args.command == 'recover':
                task = recovery_task(root, task, args.tool_root)
                with Reservation(common_dir(root), args.task_uid, (task.get('loop_binding') or {}).get('write_scope', []), recovery=True) as reservation:
                    recovery = reconcile(common_dir(root), args.task_uid, root, args.tool_root, reservation_fd=reservation.handle.fileno())
                if recovery['pending_actions']:
                    print(json.dumps(recovery, sort_keys=True))
                    return 2
                task = load_task(root, args.task_uid)
            result = validate_task(root, task, args.tool_root, args.base, args.head)
            if result['status'] in ('passed', 'legacy') and args.command in ('resume-check', 'recover'):
                with Reservation(common_dir(root), args.task_uid, (task.get('loop_binding') or {}).get('write_scope', []), recovery=args.command == 'recover') as reservation:
                    result.update(reconcile(common_dir(root), args.task_uid, root, args.tool_root, reservation_fd=reservation.handle.fileno()) if args.command == 'recover' else recovery_status(common_dir(root), args.task_uid))
                # Existing workflow-next checks live issue/snapshot/holds; never execute its next_command.
                command = [sys.executable, str((args.tool_root or root) / 'scripts/pm/workflow-next.py'), '--repo-root', str(root), '--task-uid', args.task_uid, '--json']
                observed = subprocess.run(command, text=True, capture_output=True)
                result['workflow'] = json.loads(observed.stdout)
                if observed.returncode: result['status'] = 'blocked'
            if args.command == 'publish-contract' and result['status'] == 'passed':
                if not args.contract: raise ValueError('--contract required')
                validator = _trusted_module(args.tool_root, root, task['loop_binding'], 'loop_contracts')
                with Reservation(common_dir(root), args.task_uid, task['loop_binding']['write_scope']):
                    contract = json.loads(args.contract.read_text())
                    expected = json.dumps(contract, sort_keys=True)
                    action = {'action_id': 'publication:' + hashlib.sha256(expected.encode()).hexdigest(), 'kind': 'publish_contract', 'expected': expected,
                              'repository': task['repository'], 'issue_number': task['issue_number'], 'binding': task['loop_binding']}
                    started = []
                    def before_write():
                        record_action(common_dir(root), args.task_uid, action)
                        started.append(True)
                    result = validator.publish_contract(args.tool_root, root, {**task['loop_binding'], 'issue_number': task['issue_number']}, contract, before_write=before_write)
                    if result.get('status') == 'passed' and started:
                        record_action(common_dir(root), args.task_uid, {**action, 'reconciled': True, 'readback_evidence': result})
        print(json.dumps(result, sort_keys=True))
        return 0 if result.get('status') in ('passed', 'legacy', 'bound', 'can_continue', 'task_terminal') else 2
    except (OSError, ValueError, KeyError, TypeError, Busy, subprocess.CalledProcessError) as exc:
        print(json.dumps({'status': 'blocked', 'blockers': [str(exc)]}))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
