#!/usr/bin/env python3
"""Manual single-task facade. No task discovery, scheduler, or action replay."""
import argparse
import hashlib
import inspect
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import quote
from types import SimpleNamespace

from loop_recovery import Busy, Reservation, common_dir, recovery_status, reconcile, record_action
from loop_gate import live_binding


TRACEABILITY_BOUNDARY_COMMANDS = {
    'bind', 'resume-check', 'doctor', 'publish-contract', 'promotion', 'merge',
}

RECOVERY_BRIDGE_BASE = '3b383190916ac99123a2fc9cbc0d3a8ef0d9c516'
LEGACY_RECOVERY_POLICY = 'ddbc5a7d081cffd0c17697397ee89fbd69a98cd6'
RECOVERY_BRIDGE_FILES = ('scripts/pm/loop.py', 'scripts/pm/loop_recovery.py')
POLICY_ADOPTION_MARKER_PREFIX = 'oasis7.workflow-policy-adoption'


class PolicyReaderPending(ValueError):
    """A trusted live active-pin reader could not be selected or completed."""


def admission_purpose(command):
    return 'new_tasks' if command == 'publish-contract' else 'in_flight'


def _git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def _gh_json(endpoint):
    completed = subprocess.run(['gh', 'api', endpoint], text=True,
                               capture_output=True, check=False)
    if completed.returncode:
        raise PolicyReaderPending(
            'live workflow-policy authority read is unavailable: '
            + (completed.stderr.strip() or completed.stdout.strip() or 'gh api failed')
        )
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise PolicyReaderPending('live workflow-policy authority response is malformed') from exc
    if not isinstance(value, dict):
        raise PolicyReaderPending('live workflow-policy authority response is not an object')
    return value


def _existing_trusted_default_helper(root, repository):
    """Select an already-existing exact default-tip checkout; never create one."""
    root = Path(root).resolve(strict=True)
    common = Path(_git(root, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve()
    repository_info = _gh_json(f'repos/{repository}')
    if (str(repository_info.get('full_name') or '').casefold() != repository.casefold()
            or not isinstance(repository_info.get('default_branch'), str)):
        raise PolicyReaderPending('live canonical repository/default-branch identity is unavailable')
    branch = repository_info['default_branch']
    branch_info = _gh_json(f'repos/{repository}/branches/{quote(branch, safe="")}')
    commit_info = branch_info.get('commit') if isinstance(branch_info.get('commit'), dict) else {}
    tip = commit_info.get('sha')
    if (branch_info.get('name') != branch or branch_info.get('protected') is not True
            or not isinstance(tip, str) or not re.fullmatch(r'[0-9a-f]{40}', tip)):
        raise PolicyReaderPending('canonical default branch is not a live protected tip')

    listed = subprocess.check_output(
        ['git', '-C', str(root), 'worktree', 'list', '--porcelain', '-z'],
    )
    candidates = {root, Path(__file__).resolve().parents[2]}
    record = []
    for token in listed.decode('utf-8', 'replace').split('\0'):
        if not token:
            fields = {item.split(' ', 1)[0]: item.split(' ', 1)[1]
                      for item in record if ' ' in item}
            if fields.get('worktree'):
                candidates.add(Path(fields['worktree']))
            record = []
        else:
            record.append(token)
    if record:
        fields = {item.split(' ', 1)[0]: item.split(' ', 1)[1]
                  for item in record if ' ' in item}
        if fields.get('worktree'):
            candidates.add(Path(fields['worktree']))

    for candidate in sorted(candidates, key=lambda item: str(item)):
        try:
            candidate = candidate.resolve(strict=True)
            if (Path(_git(candidate, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve() != common
                    or _git(candidate, 'rev-parse', 'HEAD') != tip
                    or _git(candidate, 'symbolic-ref', '--short', 'HEAD') != branch
                    or _git(candidate, 'status', '--porcelain', '--untracked-files=all', '--', 'scripts/pm')):
                continue
            helper = candidate / 'scripts/pm/github-project-task.py'
            if helper.is_symlink() or not helper.is_file():
                continue
            return candidate
        except (OSError, subprocess.CalledProcessError, ValueError):
            continue
    raise PolicyReaderPending(
        'no existing clean helper checkout is at the live protected default tip; '
        'active-pin read did not create a worktree or update refs'
    )


def read_effective_policy_context(root, repository, task_uid):
    """Read the active pin only through C's exact-default, read-only hosted adapter."""
    helper_root = _existing_trusted_default_helper(root, repository)
    command = [
        sys.executable, '-E', '-s', '-B', str(helper_root / 'scripts/pm/github-project-task.py'),
        'read-live-policy-context', str(Path(root).resolve()), '--repo', repository,
        '--task-uid', task_uid, '--hosted-read', '--json',
    ]
    environment = dict(os.environ)
    environment['PYTHONDONTWRITEBYTECODE'] = '1'
    completed = subprocess.run(command, text=True, capture_output=True,
                               check=False, env=environment)
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise PolicyReaderPending('trusted active-pin reader returned malformed JSON') from exc
    if (not isinstance(payload, dict)
            or payload.get('schema') != 'oasis7.workflow-policy-live-context/v1'
            or payload.get('task_uid') != task_uid
            or payload.get('repository') != repository):
        raise ValueError('trusted active-pin reader returned mismatched task identity')
    if completed.returncode == 2 or payload.get('status') == 'pending':
        blockers = payload.get('blockers') if isinstance(payload.get('blockers'), list) else []
        raise PolicyReaderPending('; '.join(str(item) for item in blockers)
                                  or 'trusted active-pin evidence is pending')
    if completed.returncode or payload.get('status') != 'passed' or payload.get('complete') is not True:
        blockers = payload.get('blockers') if isinstance(payload.get('blockers'), list) else []
        raise ValueError('; '.join(str(item) for item in blockers)
                         or 'trusted active-pin evidence is blocked')
    identity = payload.get('live_task_identity')
    binding = payload.get('binding')
    effective = payload.get('effective_policy')
    if (not isinstance(identity, dict) or identity.get('task_uid') != task_uid
            or identity.get('repository') != repository
            or not isinstance(binding, dict) or not isinstance(effective, dict)
            or effective.get('status') != 'passed'
            or not isinstance(effective.get('binding'), dict)):
        raise ValueError('trusted active-pin reader omitted its closed effective identity')
    if (effective.get('policy_commit') != effective['binding'].get('policy_commit')
            or effective.get('policy_digest') != effective['binding'].get('policy_digest')
            or not re.fullmatch(r'[0-9a-f]{40}', str(effective.get('policy_commit') or ''))
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', str(effective.get('policy_digest') or ''))):
        raise ValueError('trusted active-pin result differs from its returned binding')
    return payload


def resolve_effective_binding(root, task, *, return_context=False):
    """Return the live effective binding, preserving complete no-chain legacy pins."""
    original = task.get('loop_binding')
    if not isinstance(original, dict):
        return (original, None) if return_context else original
    repository, uid = task.get('repository'), task.get('task_uid')
    if not isinstance(repository, str) or not isinstance(uid, str):
        raise ValueError('active policy resolution requires canonical task repository and UID')
    try:
        context = read_effective_policy_context(root, repository, uid)
    except PolicyReaderPending:
        # A pre-feature trusted helper can still preserve a legacy pin, but
        # only after an independent complete live Issue/comment read proves
        # that no append-only adoption chain exists.  The candidate checkout
        # never becomes the reader and marker-shaped evidence never falls back.
        issue_number = task.get('issue_number')
        if type(issue_number) is str and re.fullmatch(r'[1-9][0-9]*', issue_number):
            issue_number = int(issue_number)
        try:
            if type(issue_number) is not int or issue_number < 1:
                raise PolicyReaderPending('canonical Task Issue number is unavailable')
            issue = json.loads(subprocess.check_output(
                ['gh', 'api', f'repos/{repository}/issues/{issue_number}'], text=True,
            ))
            expected_url = f'https://github.com/{repository}/issues/{issue_number}'
            if (not isinstance(issue, dict) or issue.get('number') != issue_number
                    or issue.get('html_url') != expected_url
                    or 'pull_request' in issue
                    or re.findall(r'(?m)^task_uid:[^\r\n]*$', str(issue.get('body') or ''))
                    != ['task_uid: ' + uid]):
                raise ValueError('live Task Issue identity conflicts with the immutable pin')
            pages = json.loads(subprocess.check_output(
                ['gh', 'api', f'repos/{repository}/issues/{issue_number}/comments?per_page=100',
                 '--paginate', '--slurp'], text=True,
            ))
        except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as exc:
            raise PolicyReaderPending('complete adoption-chain read is unavailable') from exc
        if not isinstance(pages, list) or not pages or any(not isinstance(page, list) for page in pages):
            raise PolicyReaderPending('complete adoption-chain pagination is unavailable')
        comments = [comment for page in pages for comment in page]
        if any(not isinstance(comment, dict) or type(comment.get('id')) is not int
               or comment['id'] < 1 or not isinstance(comment.get('body'), str)
               for comment in comments):
            raise PolicyReaderPending('complete adoption-chain read contains malformed comments')
        if len({comment['id'] for comment in comments}) != len(comments):
            raise PolicyReaderPending('complete adoption-chain read contains duplicate comments')
        if any(POLICY_ADOPTION_MARKER_PREFIX in comment['body'] for comment in comments):
            raise PolicyReaderPending('adoption evidence exists but the trusted resolver is unavailable')
        return (dict(original), None) if return_context else dict(original)

    live_binding = context.get('binding')
    effective_binding = context['effective_policy']['binding']
    if live_binding != original:
        raise ValueError('live immutable loop binding differs from canonical task mapping')
    expected = dict(original)
    expected['policy_commit'] = effective_binding.get('policy_commit')
    expected['policy_digest'] = effective_binding.get('policy_digest')
    if expected != effective_binding:
        raise ValueError('active policy resolver changed immutable task authorization fields')
    resolved = dict(effective_binding)
    return (resolved, context) if return_context else resolved


def existing_policy_tool_root(target_root, binding, preferred=None):
    """Select an already-checked-out exact policy commit without creating state."""
    target_root = Path(target_root).resolve(strict=True)
    commit = str(binding.get('policy_commit') or '')
    if not re.fullmatch(r'[0-9a-f]{40}', commit):
        raise ValueError('missing immutable effective policy_commit')
    common = Path(_git(target_root, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve()
    candidates = {target_root, Path(__file__).resolve().parents[2]}
    if preferred is not None:
        candidates.add(Path(preferred))
    try:
        listed = subprocess.check_output(
            ['git', '-C', str(target_root), 'worktree', 'list', '--porcelain', '-z'],
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise PolicyReaderPending('existing policy helper checkout inventory is unavailable') from exc
    record: list[str] = []
    for token in listed.decode('utf-8', 'replace').split('\0'):
        if not token:
            fields = {item.split(' ', 1)[0]: item.split(' ', 1)[1]
                      for item in record if ' ' in item}
            if fields.get('worktree'):
                candidates.add(Path(fields['worktree']))
            record = []
        else:
            record.append(token)
    if record:
        fields = {item.split(' ', 1)[0]: item.split(' ', 1)[1]
                  for item in record if ' ' in item}
        if fields.get('worktree'):
            candidates.add(Path(fields['worktree']))

    ordered = []
    if preferred is not None:
        ordered.append(Path(preferred))
    ordered.extend(sorted(candidates, key=lambda item: str(item)))
    seen: set[Path] = set()
    exact_checkout_conflict = False
    for candidate in ordered:
        try:
            candidate = candidate.resolve(strict=True)
            if candidate in seen:
                continue
            seen.add(candidate)
            if Path(_git(candidate, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve() != common:
                continue
            if _git(candidate, 'rev-parse', 'HEAD') != commit:
                continue
            if _git(candidate, 'status', '--porcelain', '--untracked-files=all', '--', 'scripts/pm'):
                exact_checkout_conflict = True
                continue
            files = _git(candidate, 'ls-tree', '-r', '--name-only', commit, '--', 'scripts/pm').splitlines()
            for relative in files:
                if not relative.endswith(('.py', '.sh', '.json')):
                    continue
                path = candidate / relative
                if path.is_symlink() or path.read_bytes() != subprocess.check_output(
                        ['git', '-C', str(candidate), 'show', commit + ':' + relative]):
                    exact_checkout_conflict = True
                    break
            else:
                tracked = set(files)
                if any(str(path.relative_to(candidate)) not in tracked
                       for path in (candidate / 'scripts/pm').glob('*.py')):
                    exact_checkout_conflict = True
                    continue
                return candidate
        except (OSError, subprocess.CalledProcessError, ValueError):
            continue
    if exact_checkout_conflict:
        raise ValueError('matching effective policy checkout has modified or shadowing helper bytes')
    raise PolicyReaderPending(
        'no existing clean helper checkout is at the effective policy pin; '
        'active policy validation did not create a worktree or update refs'
    )


def _canonical_github_repository_from_origin(root):
    try:
        remote = _git(root, 'config', '--get', 'remote.origin.url')
    except subprocess.CalledProcessError as exc:
        raise PolicyReaderPending('canonical repository origin is unavailable') from exc
    normalized = re.sub(r'\.git\Z', '', remote)
    if normalized.startswith('git@github.com:'):
        normalized = normalized.removeprefix('git@github.com:')
    elif normalized.startswith('https://github.com/'):
        normalized = normalized.removeprefix('https://github.com/')
    elif normalized.startswith('ssh://git@github.com/'):
        normalized = normalized.removeprefix('ssh://git@github.com/')
    else:
        raise ValueError('canonical repository origin is not a GitHub remote')
    if not re.fullmatch(r'[^/\s]+/[^/\s]+', normalized):
        raise ValueError('canonical GitHub repository origin is malformed')
    return normalized


def _needs_live_default_ancestry_retry(result):
    if not isinstance(result, dict):
        return False
    text = ' '.join(map(str, result.get('blockers') or [])).casefold()
    return (('origin/main' in text or 'remote-tracking' in text)
            and any(word in text for word in (
                'ancestor', 'ancestry', 'not found', 'unknown revision', 'invalid object',
            )))


def _validate_pinned_tool_root(policy, tool_root, target_root, binding, trusted_default_oid=None):
    try:
        validation = policy.validate_tool_root(tool_root, target_root, binding)
    except subprocess.CalledProcessError as exc:
        detail = ' '.join(map(str, (exc.cmd, exc.stdout, exc.stderr))).casefold()
        if 'origin/main' not in detail and 'remote-tracking' not in detail:
            raise
        validation = {'status': 'blocked', 'blockers': [detail or 'origin/main ancestry check failed']}
    if _needs_live_default_ancestry_retry(validation):
        current = _validate_with_current_trusted_policy(tool_root, target_root, binding)
        return current['validation'], current['trusted_default_oid']
    if validation.get('status') == 'pending':
        raise PolicyReaderPending('; '.join(map(str, validation.get('blockers') or []))
                                  or 'pinned helper validation is pending')
    return validation, trusted_default_oid


def _validate_with_current_trusted_policy(tool_root, target_root, binding):
    """Use a current trusted helper only to repair stale-ref ancestry checks.

    The selected pinned helper remains the authority for task semantics. This
    subprocess asks an already-existing exact protected-default checkout to
    verify the immutable pin against GitHub's live protected tip, without
    fetching or changing refs in the target repository.
    """
    repository = _canonical_github_repository_from_origin(target_root)
    helper_root = _existing_trusted_default_helper(target_root, repository)
    script = (
        'import json, sys\n'
        "sys.path.insert(0, sys.argv[1] + '/scripts/pm')\n"
        'from loop_policy import current_effective_policy_identity, validate_tool_root\n'
        'binding = json.loads(sys.argv[4])\n'
        'validation = validate_tool_root(sys.argv[2], sys.argv[3], binding)\n'
        'trusted = current_effective_policy_identity(sys.argv[3], sys.argv[5])\n'
        'print(json.dumps({"validation": validation, "trusted_current_policy": trusted}, sort_keys=True))\n'
    )
    environment = dict(os.environ)
    environment['PYTHONDONTWRITEBYTECODE'] = '1'
    completed = subprocess.run(
        [sys.executable, '-I', '-B', '-c', script, str(helper_root), str(tool_root),
         str(target_root), json.dumps(binding, sort_keys=True), repository],
        text=True, capture_output=True, check=False, env=environment,
    )
    try:
        result = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise PolicyReaderPending('trusted current policy validator returned malformed JSON') from exc
    if completed.returncode or not isinstance(result, dict):
        raise PolicyReaderPending(
            'trusted current policy validator could not complete: '
            + (completed.stderr.strip() or completed.stdout.strip() or 'invalid response')
        )
    validation = result.get('validation')
    trusted = result.get('trusted_current_policy')
    if not isinstance(validation, dict) or not isinstance(trusted, dict):
        raise PolicyReaderPending('trusted current policy validator omitted its live identity')
    if validation.get('status') == 'pending':
        raise PolicyReaderPending('; '.join(map(str, validation.get('blockers') or []))
                                  or 'trusted current policy validation is pending')
    if validation.get('status') != 'passed':
        raise ValueError('; '.join(map(str, validation.get('blockers') or []))
                         or 'trusted current policy rejected the immutable pin')
    current_oid = trusted.get('default_branch_oid')
    if not re.fullmatch(r'[0-9a-f]{40}', str(current_oid or '')):
        raise PolicyReaderPending('trusted current policy omitted its live default-branch OID')
    result['trusted_default_oid'] = current_oid
    return result


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


def _bind_policy_snapshot(binding, context=None, *, bootstrap_epoch=None):
    """Return the closed active-policy identity frozen by a bind action."""
    if not isinstance(binding, dict):
        raise ValueError('binding policy snapshot requires an immutable binding')
    effective = None
    if context is not None:
        policy = context.get('effective_policy') if isinstance(context, dict) else None
        if not isinstance(policy, dict) or policy.get('status') != 'passed':
            raise ValueError('active policy pin is unresolved for bind action')
        effective = policy.get('binding')
        if not isinstance(effective, dict) or effective != binding:
            raise ValueError('active policy snapshot differs from resolved binding')
        pin_source = policy.get('pin_source')
        policy_commit = policy.get('policy_commit')
        policy_digest = policy.get('policy_digest')
        adoption_chain_tip = policy.get('adoption_chain_tip')
    else:
        # resolve_effective_binding returns no context only for a complete
        # no-adoption legacy read or a first bind with no current binding.
        pin_source = 'immutable_binding'
        policy_commit = binding.get('policy_commit')
        policy_digest = binding.get('policy_digest')
        adoption_chain_tip = None
    epoch = binding.get('bootstrap_epoch') if bootstrap_epoch is None else bootstrap_epoch
    if (not re.fullmatch(r'[0-9a-f]{40}', str(policy_commit or ''))
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', str(policy_digest or ''))
            or type(epoch) is not int or epoch < 1):
        raise ValueError('bind action effective policy identity is incomplete')
    if pin_source == 'immutable_binding':
        if adoption_chain_tip is not None:
            raise ValueError('immutable policy pin unexpectedly has an adoption-chain tip')
    elif pin_source == 'task_issue_adoption_chain':
        if not re.fullmatch(r'sha256:[0-9a-f]{64}', str(adoption_chain_tip or '')):
            raise ValueError('adopted policy pin has no exact chain tip')
    else:
        raise ValueError('bind action has an unsupported active policy source')
    return {
        'schema': 'oasis7.loop-effective-policy-snapshot/v1',
        'policy_commit': policy_commit,
        'policy_digest': policy_digest,
        'pin_source': pin_source,
        'adoption_chain_tip': adoption_chain_tip,
        'bootstrap_epoch': epoch,
    }


def _bind_action_id(expected, policy_snapshot):
    payload = json.dumps({'expected': expected,
                          'effective_policy_snapshot': policy_snapshot},
                         sort_keys=True, separators=(',', ':'))
    return 'bind:v2:' + hashlib.sha256(payload.encode()).hexdigest()


def _build_bind_action(root, task, binding, active_binding, policy_context):
    """Freeze active policy beside, and without rewriting, the immutable binding."""
    previous = task.get('loop_binding')
    if previous is None:
        policy_snapshot = _bind_policy_snapshot(binding)
    else:
        policy_snapshot = _bind_policy_snapshot(active_binding, policy_context)
        if binding != previous:
            if (policy_snapshot['pin_source'] != 'immutable_binding'
                    or binding.get('policy_commit') != policy_snapshot['policy_commit']
                    or binding.get('policy_digest') != policy_snapshot['policy_digest']):
                raise ValueError('bind-loop cannot replace an existing effective policy pin')
    if (policy_snapshot['pin_source'] == 'task_issue_adoption_chain'
            and binding.get('bootstrap_epoch') != policy_snapshot['bootstrap_epoch']):
        raise ValueError('policy adoption cannot change the bootstrap epoch')
    expected = json.dumps(binding, sort_keys=True)
    action = {
        'action_id': _bind_action_id(expected, policy_snapshot),
        'kind': 'bind_loop',
        'expected': expected,
        'effective_policy_snapshot': policy_snapshot,
        'task_uid': task['task_uid'],
        'repository': task['repository'],
        'issue_number': task['issue_number'],
        'previous_binding': previous,
        'previous_epoch': task.get('bootstrap_epoch', 1),
        'canonical_worktree': str(Path(root).resolve()),
        'task_branch': task.get('task_branch'),
        'project_item_id': task.get('project_item_id'),
    }
    return action


def _check_bind_policy_snapshot(snapshot, active_binding, context, action,
                                previous, target_binding, old_epoch):
    """Require current live policy to equal the action's immutable pin snapshot."""
    if not isinstance(snapshot, dict) or set(snapshot) != {
            'schema', 'policy_commit', 'policy_digest', 'pin_source',
            'adoption_chain_tip', 'bootstrap_epoch'}:
        raise ValueError('binding journal policy snapshot is incomplete or unsupported')
    if snapshot.get('schema') != 'oasis7.loop-effective-policy-snapshot/v1':
        raise ValueError('binding journal policy snapshot schema mismatch')
    current = _bind_policy_snapshot(active_binding, context)
    if any(snapshot[key] != current[key] for key in (
            'policy_commit', 'policy_digest', 'pin_source', 'adoption_chain_tip')):
        raise ValueError('active policy pin differs from recorded bind action')
    if snapshot['bootstrap_epoch'] == current['bootstrap_epoch']:
        pass
    elif (snapshot['pin_source'] == 'immutable_binding'
          and previous is not None and previous != target_binding
          and snapshot['bootstrap_epoch'] == old_epoch
          and target_binding.get('bootstrap_epoch') == current['bootstrap_epoch']
          and current['bootstrap_epoch'] == old_epoch + 1
          and target_binding.get('policy_commit') == snapshot['policy_commit']
          and target_binding.get('policy_digest') == snapshot['policy_digest']):
        # A no-adoption explicit one-step migration may advance the task epoch
        # while retaining the same original policy pin. Adoption itself never
        # advances the epoch.
        pass
    else:
        raise ValueError('active policy pin differs from recorded bind action')
    if (previous is not None and previous != target_binding
            and (snapshot['pin_source'] != 'immutable_binding'
                 or target_binding.get('policy_commit') != snapshot['policy_commit']
                 or target_binding.get('policy_digest') != snapshot['policy_digest'])):
        raise ValueError('bind-loop cannot replace an existing effective policy pin')


def recovery_task(root, task, tool_root):
    """Permit only a recorded old/new binding transition, never generic drift."""
    uid = task['task_uid']
    pending = recovery_status(common_dir(root), uid)['pending_actions']
    transitions = [a for a in pending if a['kind'] == 'bind_loop']
    if not transitions:
        observed = load_task(root, uid)
        if observed.get('loop_binding') is not None:
            effective, context = resolve_effective_binding(root, observed, return_context=True)
            current_oid = ((context or {}).get('trusted_current_policy') or {}).get('default_branch_oid')
            recovery_authority(root, effective, tool_root, trusted_default_oid=current_oid)
            return {**observed, '_effective_loop_binding': effective,
                    '_trusted_default_oid': current_oid}
        return observed
    if len(transitions) != 1 or len(pending) != 1:
        raise ValueError('ambiguous pending binding transition')
    action = transitions[0]
    binding = json.loads(action['expected'])
    legacy_action_id = 'bind:' + hashlib.sha256(action['expected'].encode()).hexdigest()
    policy_snapshot = action.get('effective_policy_snapshot')
    versioned_action_id = (_bind_action_id(action['expected'], policy_snapshot)
                           if isinstance(policy_snapshot, dict) else None)
    if action.get('action_id') not in (legacy_action_id, versioned_action_id):
        raise ValueError('binding journal action digest mismatch')
    if policy_snapshot is not None and not isinstance(policy_snapshot, dict):
        raise ValueError('binding journal policy snapshot is malformed')
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
    live_observed = live_binding(task)
    for observed in (task.get('loop_binding'), live_observed):
        if observed not in (previous, binding):
            raise ValueError('binding drift outside journal old/new transition')
    for path in (root / '.pm/scratch' / uid / 'bootstrap-task-snapshot.json', common_dir(root) / 'oasis7-loop-lineage' / (uid + '.json')):
        if path.exists():
            saved = json.loads(path.read_text())
            observed = saved.get('task', saved).get('loop_binding')
            if observed not in (previous, binding):
                raise ValueError('snapshot/lineage drift outside binding journal')
    if live_observed is None:
        if previous is not None:
            raise ValueError('binding transition has no live immutable binding')
        active_binding, policy_context = binding, None
    else:
        resolution_task = {**task, 'loop_binding': live_observed,
                           'bootstrap_epoch': live_observed.get('bootstrap_epoch')}
        active_binding, policy_context = resolve_effective_binding(
            root, resolution_task, return_context=True,
        )
        if not isinstance(active_binding, dict):
            raise ValueError('binding transition has no resolved active policy pin')
    policy = (policy_context or {}).get('effective_policy') if isinstance(policy_context, dict) else None
    pin_source = policy.get('pin_source') if isinstance(policy, dict) else 'immutable_binding'
    if policy_snapshot is None:
        if action.get('action_id') != legacy_action_id:
            raise ValueError('binding journal action digest mismatch')
        if pin_source != 'immutable_binding':
            raise ValueError('legacy bind action cannot prove adopted policy pin')
        active_snapshot = _bind_policy_snapshot(active_binding, policy_context)
        if (previous is not None and previous != binding
                and (binding.get('policy_commit') != active_snapshot['policy_commit']
                     or binding.get('policy_digest') != active_snapshot['policy_digest'])):
            raise ValueError('legacy bind action cannot replace an effective policy pin')
    else:
        _check_bind_policy_snapshot(policy_snapshot, active_binding, policy_context,
                                    action, previous, binding, old_epoch)
        if action.get('action_id') != versioned_action_id:
            raise ValueError('binding journal action digest mismatch')
    current_oid = ((policy_context or {}).get('trusted_current_policy') or {}).get('default_branch_oid')
    recovery_authority(root, active_binding, tool_root, trusted_default_oid=current_oid)
    return {**task, 'loop_binding': binding, 'bootstrap_epoch': binding['bootstrap_epoch'],
            '_effective_loop_binding': active_binding,
            '_trusted_default_oid': current_oid}


def recovery_authority(root, binding, tool_root, *, trusted_default_oid=None):
    if tool_root is None:
        raise ValueError('recovery requires effective trusted --tool-root')
    tool_root = existing_policy_tool_root(root, binding, tool_root)
    policy = _trusted_module_for_binding(tool_root, root, binding, 'loop_policy',
                                         trusted_default_oid)
    binding_result = policy.validate_binding(binding)
    if binding_result.get('blockers'):
        raise ValueError('; '.join(map(str, binding_result['blockers'])))
    tool_result, trusted_default_oid = _validate_pinned_tool_root(
        policy, tool_root, root, binding, trusted_default_oid,
    )
    blockers = list(tool_result.get('blockers', []))
    if blockers: raise ValueError('; '.join(blockers))
    _trusted_module_for_binding(tool_root, root, binding, 'loop_contracts',
                                trusted_default_oid)


def _trusted_module(root, target, binding, name, *, trusted_default_oid=None):
    commit = binding.get('policy_commit', '')
    if not re.fullmatch(r'[0-9a-f]{40}', commit):
        raise ValueError('missing immutable effective policy_commit')
    if _git(root, 'rev-parse', 'HEAD') != commit:
        raise ValueError('tool root HEAD is not effective policy_commit')
    if trusted_default_oid is not None:
        if not re.fullmatch(r'[0-9a-f]{40}', str(trusted_default_oid)):
            raise ValueError('live trusted default-branch OID is malformed')
        present = subprocess.run(
            ['git', '-C', str(target), 'cat-file', '-e', trusted_default_oid + '^{commit}'],
            capture_output=True, text=True,
        )
        if present.returncode:
            raise PolicyReaderPending('live trusted default-branch commit is not present locally; no fetch performed')
        ancestry = subprocess.run(
            ['git', '-C', str(target), 'merge-base', '--is-ancestor', commit, trusted_default_oid],
            capture_output=True, text=True,
        )
        if ancestry.returncode:
            raise ValueError('effective policy commit is not on the live trusted default-branch history')
    else:
        subprocess.run(['git', '-C', str(target), 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main'], check=True, capture_output=True)
    if common_dir(root) != common_dir(target):
        raise ValueError('tool root belongs to another repository')
    # Check every executable dependency before importing any candidate-controlled code.
    trusted_import_files = (
        'scripts/document_corpus.py',
        'scripts/product-doc-content-check.py',
        'scripts/product_doc_markdown.py',
    )
    files = _git(
        root, 'ls-tree', '-r', '--name-only', commit, '--', 'scripts/pm',
        *trusted_import_files,
    ).splitlines()
    trusted_root = Path(root).resolve()
    for relative in files:
        if not relative.endswith(('.py', '.sh', '.json')): continue
        expected = subprocess.check_output(['git', '-C', str(target), 'show', commit + ':' + relative])
        path = trusted_root / relative
        if (path.is_symlink() or not path.resolve().is_relative_to(trusted_root)
                or path.read_bytes() != expected):
            raise ValueError('effective helper bytes differ: ' + relative)
    untracked = _git(
        trusted_root, 'ls-files', '--others', '--', 'scripts/pm',
        *trusted_import_files, ':(exclude)**/__pycache__/**',
    ).splitlines()
    if untracked:
        raise ValueError('untracked executable in trusted tool root: ' + untracked[0])
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(root / 'scripts/pm'))
    if name == 'loop_policy':
        # loop_policy imports loop_contracts by module name. Replace any
        # preloaded candidate object with the exact effective-root module.
        contracts_path = trusted_root / 'scripts/pm/loop_contracts.py'
        contract_spec = importlib.util.spec_from_file_location('loop_contracts', contracts_path)
        if contract_spec is None or contract_spec.loader is None:
            raise ValueError('trusted loop_contracts helper unavailable')
        contracts = importlib.util.module_from_spec(contract_spec)
        sys.modules['loop_contracts'] = contracts
        contract_spec.loader.exec_module(contracts)
    path = root / 'scripts/pm' / (name + '.py')
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def trusted_module(root, target, binding, name):
    """Expose the single pinned-loader boundary to lifecycle callers."""
    return _trusted_module(root, target, binding, name)


def _trusted_module_for_binding(root, target, binding, name, trusted_default_oid=None):
    if trusted_default_oid is None:
        return _trusted_module(root, target, binding, name)
    return _trusted_module(root, target, binding, name,
                           trusted_default_oid=trusted_default_oid)


def _recovery_publication_adapter(tool_root, target_root, binding, *, require_legacy=False,
                                 trusted_default_oid=None):
    """Bind old contract authority to the narrow, merged recovery bridge."""
    if require_legacy and binding.get('policy_commit') != LEGACY_RECOVERY_POLICY:
        raise ValueError('mixed-epoch recovery requires exact legacy policy commit')
    bridge_root = Path(__file__).resolve().parents[2]
    bridge_commit = _git(bridge_root, 'rev-parse', 'HEAD')
    dirty = _git(
        bridge_root, 'status', '--porcelain', '--untracked-files=all', '--',
        *RECOVERY_BRIDGE_FILES,
    )
    if dirty:
        raise ValueError('recovery bridge helper bytes are dirty')
    ancestry = [(bridge_commit, 'refs/remotes/origin/main',
                 'recovery bridge is not merged to origin/main')]
    if require_legacy:
        ancestry.insert(0, (RECOVERY_BRIDGE_BASE, bridge_commit,
                            'recovery bridge predates supersession authority'))
    for older, newer, message in ancestry:
        checked = subprocess.run(
            ['git', '-C', str(bridge_root), 'merge-base', '--is-ancestor', older, newer],
            capture_output=True, text=True,
        )
        if checked.returncode:
            raise ValueError(message)
    file_digests = {}
    for relative in RECOVERY_BRIDGE_FILES:
        path = bridge_root / relative
        expected = subprocess.check_output(
            ['git', '-C', str(bridge_root), 'show', bridge_commit + ':' + relative]
        )
        if path.is_symlink() or path.read_bytes() != expected:
            raise ValueError('recovery bridge helper bytes differ: ' + relative)
        file_digests[relative] = hashlib.sha256(expected).hexdigest()
    bridge_digest = 'sha256:' + hashlib.sha256(json.dumps(
        {'commit': bridge_commit, 'files': file_digests},
        sort_keys=True, separators=(',', ':'),
    ).encode()).hexdigest()
    contracts = _trusted_module_for_binding(tool_root, target_root, binding,
                                            'loop_contracts', trusted_default_oid)
    return SimpleNamespace(
        contracts=contracts,
        policy_commit=binding.get('policy_commit'),
        policy_digest=binding.get('policy_digest'),
        bridge_commit=bridge_commit,
        bridge_digest=bridge_digest,
    )


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


def validate_task(root, task, tool_root, base=None, head=None, contracts=True,
                  purpose='in_flight', effective_binding=None, resolve_policy=True,
                  trusted_default_oid=None):
    if 'loop_binding' not in task or task['loop_binding'] is None:
        return {'status': 'legacy', 'blockers': []}
    try:
        if tool_root is None:
            raise ValueError('activation prerequisite: explicit trusted --tool-root required')
        if effective_binding is not None:
            binding = effective_binding
        elif resolve_policy:
            resolved = resolve_effective_binding(root, task, return_context=True)
            if isinstance(resolved, tuple) and len(resolved) == 2:
                binding, policy_context = resolved
                trusted_default_oid = trusted_default_oid or (
                    (policy_context or {}).get('trusted_current_policy') or {}
                ).get('default_branch_oid')
            else:  # Keep narrow compatibility with adapters that return only a binding.
                binding = resolved
        else:
            binding = task['loop_binding']
        if not isinstance(binding, dict):
            raise ValueError('active loop binding is unresolved')
        tool_root = existing_policy_tool_root(root, binding, tool_root)
        policy = _trusted_module_for_binding(tool_root, root, binding, 'loop_policy',
                                             trusted_default_oid)
        blockers = list(policy.validate_binding(binding).get('blockers', []))
        if blockers:
            return {'status': 'blocked', 'blockers': blockers}
        tool_validation, trusted_default_oid = _validate_pinned_tool_root(
            policy, tool_root, root, binding, trusted_default_oid,
        )
        blockers += list(tool_validation.get('blockers', []))
        closure = {binding['task_uid']: binding}
        pending = list(binding.get('dependencies', []))
        while pending:
            uid = pending.pop()
            if uid in closure: continue
            if len(closure) > 100: raise ValueError('dependency closure exceeds bounded admission limit')
            repository = task.get('repository')
            if not repository: raise ValueError('dependency validation requires live repository identity')
            number = dependency_issue(repository, uid)
            terminal = _trusted_module_for_binding(tool_root, root, binding, 'loop_terminal',
                                                   trusted_default_oid)
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
            validator = _trusted_module_for_binding(tool_root, root, binding, 'loop_contracts',
                                                    trusted_default_oid)
            blockers += validator.validate_contracts(tool_root, root, binding, purpose=purpose).get('blockers', [])
        return {'status': 'blocked' if blockers else 'passed', 'blockers': blockers,
                'loop_binding': binding, 'execution_scope': 'execution_scope_unverified'}
    except PolicyReaderPending as exc:
        return {'status': 'pending', 'blockers': [str(exc)]}
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        return {'status': 'blocked', 'blockers': [str(exc)]}


def _traceability_adapter(tool_root, target_root, binding, effective_tool_commit,
                          trusted_default_oid=None):
    """Load the traceability preflight from the pinned effective helper root."""
    if effective_tool_commit != binding.get('policy_commit'):
        raise ValueError('effective tool commit does not match loop binding policy_commit')
    return _trusted_module_for_binding(tool_root, target_root, binding,
                                       'loop_traceability', trusted_default_oid)


def _traceability_preflight(adapter, command, *, binding, target_root,
                            effective_tool_commit, record_source_commit):
    """Invoke the core's live leaf adapter without introducing a second validator.

    ``loop_traceability`` owns record parsing, live GitHub readback, and all
    checker semantics.  The facade only selects its stable preflight adapter;
    fixture readers are intentionally unavailable on this production path.
    """
    preflight = getattr(adapter, 'preflight_leaf', None)
    if callable(preflight):
        return _invoke_traceability_preflight(
            preflight,
            command,
            binding=binding,
            target_root=target_root,
            effective_tool_commit=effective_tool_commit,
            record_source_commit=record_source_commit,
        )
    # Keep a narrow compatibility spelling for the first merged core adapter;
    # this is an adapter call, not a duplicate validation implementation.
    preflight = getattr(adapter, 'validate_leaf_admission', None)
    if callable(preflight):
        return _invoke_traceability_preflight(
            preflight,
            command,
            binding=binding,
            target_root=target_root,
            effective_tool_commit=effective_tool_commit,
            record_source_commit=record_source_commit,
        )
    raise ValueError('effective traceability helper lacks preflight_leaf adapter')


def _invoke_traceability_preflight(preflight, command, *, binding, target_root,
                                   effective_tool_commit, record_source_commit):
    """Call either the split-identity adapter or its first merged spelling."""
    try:
        parameters = inspect.signature(preflight).parameters
    except (TypeError, ValueError):
        parameters = {}
    if 'effective_tool_commit' in parameters or 'record_source_commit' in parameters:
        return preflight(
            command,
            binding=binding,
            target_root=target_root,
            effective_tool_commit=effective_tool_commit,
            record_source_commit=record_source_commit,
        )
    return preflight(
        command,
        binding=binding,
        target_root=target_root,
        source_commit=record_source_commit,
    )


def pre_mutation_admission(command, *, binding, target_root, effective_tool_root,
                           source_commit=None, effective_tool_commit=None,
                           record_source_commit=None, traceability_loader=None,
                           mutation=None):
    """Run bound traceability admission before a lifecycle mutation.

    The gate is deliberately a small ordering boundary.  A legacy/unbound
    task keeps its existing behavior; a bound task must load the helper from
    the pinned effective checkout and pass the core's live preflight before
    the supplied mutation callback is invoked.
    """
    if command not in TRACEABILITY_BOUNDARY_COMMANDS:
        raise ValueError('unsupported pre-mutation admission command: ' + str(command))
    if not isinstance(binding, dict):
        raise ValueError('pre-mutation admission requires loop binding')
    callback = mutation or (lambda: None)
    # Ordinary legacy leaf lifecycle behavior remains unchanged.  A caller
    # cannot opt into a bound path by supplying a free-form flag.
    if binding.get('coordination_ref') is None:
        return callback()
    if effective_tool_root is None:
        raise ValueError('bound traceability admission requires effective --tool-root')
    effective_tool_commit = effective_tool_commit or source_commit or binding.get('policy_commit')
    coordination_ref = binding.get('coordination_ref')
    if record_source_commit is None and isinstance(coordination_ref, dict):
        record_source_commit = coordination_ref.get('source_commit')
    record_source_commit = record_source_commit or source_commit
    if not isinstance(effective_tool_commit, str) or not re.fullmatch(r'[0-9a-f]{40}', effective_tool_commit):
        raise ValueError('bound traceability admission requires immutable effective_tool_commit')
    if not isinstance(record_source_commit, str) or not re.fullmatch(r'[0-9a-f]{40}', record_source_commit):
        raise ValueError('bound traceability admission requires immutable record_source_commit')
    tool_root = Path(effective_tool_root).resolve()
    target_root = Path(target_root).resolve()
    loader = traceability_loader or (
        lambda effective_root, commit: _traceability_adapter(
            effective_root, target_root, binding, commit
        )
    )
    adapter = loader(tool_root, effective_tool_commit)
    result = _traceability_preflight(
        adapter,
        command,
        binding=binding,
        target_root=target_root,
        effective_tool_commit=effective_tool_commit,
        record_source_commit=record_source_commit,
    )
    if not isinstance(result, dict):
        raise ValueError('traceability preflight returned no structured result')
    if result.get('status') != 'passed':
        blockers = result.get('blockers') or ['traceability preflight blocked']
        raise ValueError('; '.join(str(item) for item in blockers))
    return callback()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['doctor', 'bind', 'status', 'resume-check', 'recover', 'validate-scope', 'validate-contracts', 'publish-contract'])
    parser.add_argument('--repo-root', type=Path, default=Path.cwd())
    parser.add_argument('--tool-root', type=Path)
    parser.add_argument('--task-uid', required=True)
    parser.add_argument('--manual-request-ref')
    parser.add_argument('--supersede-invalid-publication')
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
        if args.tool_root and (task.get('loop_binding') is not None or args.command == 'bind') and args.command in ('bind', 'recover', 'publish-contract'):
            subprocess.run(['git', '-C', str(root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'], check=True, capture_output=True)
        effective_binding = None
        trusted_default_oid = None
        policy_context = None
        if (effective_binding is None and task.get('loop_binding') is not None
                and args.command != 'recover'):
            resolved = resolve_effective_binding(root, task, return_context=True)
            if isinstance(resolved, tuple) and len(resolved) == 2:
                effective_binding, policy_context = resolved
                trusted_default_oid = ((policy_context or {}).get('trusted_current_policy') or {}).get('default_branch_oid')
            else:
                effective_binding = resolved
        effective_tool_root = args.tool_root
        if effective_binding is not None and effective_tool_root is not None:
            effective_tool_root = existing_policy_tool_root(root, effective_binding, effective_tool_root)
        if args.command in ('bind', 'resume-check', 'recover', 'publish-contract') and not args.manual_request_ref:
            raise ValueError('explicit current manual request reference required')
        if args.supersede_invalid_publication and args.command != 'recover':
            raise ValueError('--supersede-invalid-publication is valid only with recover')
        if args.command == 'bind':
            if not args.loop_binding: raise ValueError('--loop-binding required')
            binding = json.loads(args.loop_binding.read_text())
            if binding.get('manual_request_ref') != args.manual_request_ref:
                raise ValueError('manual request does not match proposed binding')
            active_binding = effective_binding if effective_binding is not None else binding
            if not isinstance(active_binding, dict):
                raise ValueError('bind requires a resolved effective policy pin')
            if args.tool_root is None:
                raise ValueError('bind requires an explicit trusted --tool-root')
            effective_tool_root = existing_policy_tool_root(root, active_binding, args.tool_root)
            proposed = {**task, 'loop_binding': binding}
            if args.migrate_epoch is not None:
                if args.migrate_epoch != int(task.get('bootstrap_epoch', 1)) + 1:
                    raise ValueError('migration must advance exactly one bootstrap epoch')
                proposed['bootstrap_epoch'] = args.migrate_epoch
            with Reservation(common_dir(root), args.task_uid, binding['write_scope']) as reservation:
                action = _build_bind_action(root, task, binding, active_binding, policy_context)
                command = [sys.executable, str(effective_tool_root / 'scripts/pm/github-project-task.py'), 'bind-loop', str(root), '--task-uid', args.task_uid, '--loop-binding', str(args.loop_binding.resolve()), '--manual-request-ref', args.manual_request_ref, '--json']
                if args.migrate_epoch is not None: command += ['--migrate-epoch', str(args.migrate_epoch)]
                command += ['--loop-action-json', json.dumps(action, sort_keys=True)]

                def bind_mutation():
                    validation_task = {**proposed, 'bootstrap_epoch': active_binding.get('bootstrap_epoch')}
                    checked = validate_task(
                        root,
                        validation_task,
                        effective_tool_root,
                        contracts=True,
                        purpose='in_flight' if task.get('loop_binding') == binding else 'new_tasks',
                        resolve_policy=False,
                        effective_binding=active_binding,
                        trusted_default_oid=trusted_default_oid,
                    )
                    if checked['status'] != 'passed':
                        raise ValueError('; '.join(checked['blockers']))
                    bound = json.loads(subprocess.check_output(
                        command, text=True, pass_fds=(reservation.handle.fileno(),)
                    ))
                    if bound.get('status') == 'bound':
                        record_action(common_dir(root), args.task_uid, {
                            **action, 'reconciled': True, 'readback_evidence': bound
                        })
                    return bound

                result = pre_mutation_admission(
                    'bind',
                    binding=active_binding,
                    target_root=root,
                    effective_tool_root=effective_tool_root,
                    source_commit=active_binding.get('policy_commit'),
                    effective_tool_commit=active_binding.get('policy_commit'),
                    record_source_commit=(active_binding.get('coordination_ref') or {}).get('source_commit'),
                    traceability_loader=lambda effective_root, commit: _traceability_adapter(
                        effective_root, root, active_binding, commit,
                        trusted_default_oid=trusted_default_oid,
                    ),
                    mutation=bind_mutation,
                )
        else:
            if args.command == 'validate-scope' and not args.base: raise ValueError('--base required')
            if args.command == 'recover':
                task = recovery_task(root, task, args.tool_root)
                effective_binding = task.get('_effective_loop_binding')
                trusted_default_oid = task.get('_trusted_default_oid')
                if effective_binding is not None and args.tool_root is not None:
                    effective_tool_root = existing_policy_tool_root(root, effective_binding, args.tool_root)
                publication_adapter = _recovery_publication_adapter(
                    effective_tool_root, root, effective_binding or task['loop_binding'],
                    require_legacy=bool(args.supersede_invalid_publication),
                    trusted_default_oid=trusted_default_oid,
                )
                with Reservation(common_dir(root), args.task_uid, (task.get('loop_binding') or {}).get('write_scope', []), recovery=True) as reservation:
                    recovery = reconcile(
                        common_dir(root), args.task_uid, root, effective_tool_root,
                        reservation_fd=reservation.handle.fileno(),
                        supersede_invalid_publication=args.supersede_invalid_publication,
                        manual_request_ref=args.manual_request_ref,
                        publication_adapter=publication_adapter,
                    )
                if recovery['pending_actions']:
                    print(json.dumps(recovery, sort_keys=True))
                    return 2
                task = load_task(root, args.task_uid)
            if args.command in ('resume-check', 'doctor'):
                binding = effective_binding or task.get('loop_binding')
                with Reservation(
                    common_dir(root),
                    args.task_uid,
                    (binding or {}).get('write_scope', []),
                    recovery=False,
                ):
                    def continuation_readback():
                        checked = validate_task(
                            root, task, effective_tool_root, args.base, args.head,
                            effective_binding=effective_binding,
                            trusted_default_oid=trusted_default_oid,
                        )
                        if checked['status'] in ('passed', 'legacy') and args.command == 'resume-check':
                            checked.update(recovery_status(common_dir(root), args.task_uid))
                            # Existing workflow-next checks live issue/snapshot/holds;
                            # never execute its next_command.
                            next_command = [
                                sys.executable,
                                str((effective_tool_root or root) / 'scripts/pm/workflow-next.py'),
                                '--repo-root', str(root),
                                '--task-uid', args.task_uid,
                                '--json',
                            ]
                            observed = subprocess.run(next_command, text=True, capture_output=True)
                            checked['workflow'] = json.loads(observed.stdout)
                            if observed.returncode:
                                checked['status'] = 'blocked'
                        return checked

                    result = pre_mutation_admission(
                        args.command,
                        binding=binding or {},
                        target_root=root,
                        effective_tool_root=effective_tool_root,
                        source_commit=(binding or {}).get('policy_commit'),
                        effective_tool_commit=(binding or {}).get('policy_commit'),
                        record_source_commit=((binding or {}).get('coordination_ref') or {}).get('source_commit'),
                        traceability_loader=lambda effective_root, commit: _traceability_adapter(
                            effective_root, root, binding or {}, commit,
                            trusted_default_oid=trusted_default_oid,
                        ),
                        mutation=continuation_readback,
                    )
            else:
                result = validate_task(
                    root, task, effective_tool_root, args.base, args.head,
                    purpose=admission_purpose(args.command),
                    effective_binding=effective_binding,
                    trusted_default_oid=trusted_default_oid,
                )
                if result['status'] in ('passed', 'legacy') and args.command == 'recover':
                    with Reservation(common_dir(root), args.task_uid, (task.get('loop_binding') or {}).get('write_scope', []), recovery=True) as reservation:
                        result.update(reconcile(
                            common_dir(root), args.task_uid, root, effective_tool_root,
                            reservation_fd=reservation.handle.fileno(),
                            supersede_invalid_publication=args.supersede_invalid_publication,
                            manual_request_ref=args.manual_request_ref,
                            publication_adapter=publication_adapter,
                        ))
            if args.command == 'publish-contract' and result['status'] == 'passed':
                if not args.contract: raise ValueError('--contract required')
                binding = result.get('loop_binding') or effective_binding or task['loop_binding']
                validator = _trusted_module_for_binding(
                    effective_tool_root, root, binding, 'loop_contracts', trusted_default_oid,
                )
                with Reservation(common_dir(root), args.task_uid, binding['write_scope']):
                    contract = json.loads(args.contract.read_text())
                    expected = json.dumps(contract, sort_keys=True)
                    action = {'action_id': 'publication:' + hashlib.sha256(expected.encode()).hexdigest(), 'kind': 'publish_contract', 'expected': expected,
                              'repository': task['repository'], 'issue_number': task['issue_number'], 'binding': binding}
                    started = []
                    def before_write():
                        record_action(common_dir(root), args.task_uid, action)
                        started.append(True)
                    def publish_mutation():
                        published = validator.publish_contract(
                            effective_tool_root, root,
                            {**binding, 'issue_number': task['issue_number']},
                            contract, before_write=before_write,
                        )
                        if published.get('status') == 'passed' and started:
                            record_action(common_dir(root), args.task_uid, {
                                **action, 'reconciled': True, 'readback_evidence': published,
                            })
                        return published
                    result = pre_mutation_admission(
                        'publish-contract',
                        binding=binding,
                        target_root=root,
                        effective_tool_root=effective_tool_root,
                        source_commit=binding.get('policy_commit'),
                        effective_tool_commit=binding.get('policy_commit'),
                        record_source_commit=(binding.get('coordination_ref') or {}).get('source_commit'),
                        traceability_loader=lambda effective_root, commit: _traceability_adapter(
                            effective_root, root, binding, commit,
                            trusted_default_oid=trusted_default_oid,
                        ),
                        mutation=publish_mutation,
                    )
        print(json.dumps(result, sort_keys=True))
        return 0 if result.get('status') in ('passed', 'legacy', 'bound', 'can_continue', 'task_terminal') else 2
    except PolicyReaderPending as exc:
        print(json.dumps({'status': 'pending', 'blockers': [str(exc)]}))
        return 2
    except (OSError, ValueError, KeyError, TypeError, Busy, subprocess.CalledProcessError) as exc:
        print(json.dumps({'status': 'blocked', 'blockers': [str(exc)]}))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
