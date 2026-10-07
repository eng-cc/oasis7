#!/usr/bin/env python3
"""Independent PR loop gate: read live task, execute only effective helpers."""
import argparse
import base64
import contextlib
import importlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time


class PublicationPending(ValueError):
    """A verified publication transition has not finished reading back yet."""


class AuthorityPending(ValueError):
    """A live authority read failed or could not be proven complete."""


def await_pr_binding(args, pr, issue, number, uid, *, wait_for_initial=True,
                     allow_missing_initial=False):
    """Only an absent initial PR-number field can enter the publication wait."""
    def pr_fields(body):
        return re.findall(r'^[ \t]*(?:-[ \t]*)?pr_number\b[^\n]*', body, re.MULTILINE)

    def identity(value):
        body = value.get('body') or ''
        head, base = value['head'], value['base']
        if (value.get('number') != args.pr_number or value.get('state') != 'open'
                or value.get('merged_at', 'missing') is not None or value.get('draft') is not True
                or head['sha'] != args.head
                or base['repo']['full_name'] != args.repository
                or not head.get('ref') or not base.get('ref')
                or set(re.findall(r'task_[0-9a-f]{32}', body)) != {uid}):
            raise ValueError('pending PR binding identity invalid or drifted')
        return (head['ref'], head['repo']['full_name'], base['ref'],
                tuple(sorted(set(re.findall(r'(?:Refs|Fixes|Closes)\s+#(\d+)', body, re.I)))))

    body = (issue.get('body') or '').replace('\r\n', '\n')
    expected = f'- pr_number: `{args.pr_number}`'
    if pr_fields(body):
        if pr_fields(body) != [expected]:
            raise ValueError('live task Issue does not bind this PR number; refresh task PR identity')
        return body
    if not wait_for_initial:
        if allow_missing_initial:
            return body
        raise ValueError('live Task Issue PR number binding is missing outside the initial publication window')
    frozen = identity(pr)
    for attempt in range(7):
        body = (issue.get('body') or '').replace('\r\n', '\n')
        if (issue.get('number') != int(number) or issue.get('state') != 'open'
                or '<!-- oasis7-pm-task -->' not in body
                or re.findall(r'^task_uid:[^\n]*$', body, re.MULTILINE) != ['task_uid: ' + uid]
                or identity(pr) != frozen):
            raise ValueError('pending task/PR binding identity invalid or drifted')
        fields = pr_fields(body)
        if fields:
            if fields != [expected]:
                raise ValueError('live task Issue does not bind this PR number; refresh task PR identity')
            return body
        if attempt == 6:
            raise ValueError('initial task PR binding publication timeout after 30 seconds of waiting')
        time.sleep(5)
        pr = json.loads(run('gh', 'api', f'repos/{args.repository}/pulls/{args.pr_number}'))
        issue = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{number}'))


def validate_full_binding(args, pr, issue, number, uid):
    """Validate the complete live Task/PR vector used by start/final phases."""
    body = (issue.get('body') or '').replace('\r\n', '\n')
    pr_number_fields = re.findall(r'(?m)^- pr_number: `([^`]*)`$', body)
    pr_url_fields = re.findall(r'(?m)^- pr_url: `([^`]*)`$', body)
    expected_url = f'https://github.com/{args.repository}/pull/{args.pr_number}'
    if (issue.get('number') != int(number) or str(issue.get('state') or '').lower() != 'open'
            or pr_number_fields != [str(args.pr_number)]
            or pr_url_fields != [expected_url]):
        raise ValueError('live Task Issue PR number/URL binding is missing, ambiguous, or drifted')

    pr_body = (pr.get('body') if isinstance(pr.get('body'), str) else '').replace('\r\n', '\n')
    uid_tokens = set(re.findall(r'task_[0-9a-f]{32}', pr_body))
    task_fields = re.findall(r'(?m)^[ \t]*Task[ \t]*:[^\r\n]*$', pr_body)
    task_refs = re.findall(r'(?m)^\s*Task: (task_[0-9a-f]{32})\s*$', pr_body)
    refs = re.findall(r'(?m)^\s*Refs #([1-9][0-9]*)\s*$', pr_body)
    closing_refs = re.findall(r'(?im)^\s*(?:Fixes|Closes) #([1-9][0-9]*)\s*$', pr_body)
    head = pr.get('head') if isinstance(pr.get('head'), dict) else {}
    base = pr.get('base') if isinstance(pr.get('base'), dict) else {}
    head_repo = head.get('repo') if isinstance(head.get('repo'), dict) else {}
    base_repo = base.get('repo') if isinstance(base.get('repo'), dict) else {}
    expected_pr_url = f'https://github.com/{args.repository}/pull/{args.pr_number}'
    try:
        repository_info = json.loads(run('gh', 'api', f'repos/{args.repository}'))
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        raise AuthorityPending(f'live repository/default-branch read is pending: {exc}') from exc
    default_branch = repository_info.get('default_branch') if isinstance(repository_info, dict) else None
    if (pr.get('number') != args.pr_number or str(pr.get('state') or '').lower() != 'open'
            or pr.get('merged_at') is not None or pr.get('draft') is not True
            or pr.get('html_url') != expected_pr_url
            or head.get('sha') != args.head or not head.get('ref')
            or head_repo.get('full_name') != args.repository
            or base.get('ref') != default_branch or base_repo.get('full_name') != args.repository
            or uid_tokens != {uid} or len(task_fields) != 1 or task_refs != [uid]
            or refs != [str(number)] or closing_refs):
        raise ValueError('live PR Task/Refs/repository/head/base binding is missing, ambiguous, or drifted')


def _issue_scalar(body, key):
    lines = re.findall(rf'(?m)^-[ \t]+{re.escape(key)}:[^\n]*$', body)
    if len(lines) > 1:
        raise ValueError(f'live Task Issue {key} field is duplicated')
    if not lines:
        return None
    match = re.fullmatch(rf'-[ \t]+{re.escape(key)}:[ \t]*`([^`\n]*)`[ \t]*', lines[0])
    if match is None:
        raise ValueError(f'live Task Issue {key} field is malformed')
    return match.group(1) or None


def _task_pr_number(issue_body):
    value = _issue_scalar(issue_body, 'pr_number')
    if value is None:
        return None
    if not re.fullmatch(r'[1-9][0-9]*', value):
        raise ValueError('live Task Issue PR number is malformed')
    return int(value)


def _live_task_hold(issue_body):
    keys = ('kind', 'requester', 'reason', 'resume_authority', 'active')
    values = {}
    for key in keys:
        matches = re.findall(rf'(?m)^-[ \t]+merge_hold_{key}:[ \t]*`([^`\n]*)`[ \t]*$', issue_body)
        if len(matches) > 1:
            raise ValueError(f'live Task Issue merge hold {key} field is duplicated')
        if matches:
            values[key] = matches[0]
    if values and set(values) != set(keys):
        raise ValueError('live Task Issue merge hold projection is incomplete')
    if not values:
        return {'active': 'false'}
    if values['active'] not in {'true', 'false'}:
        raise ValueError('live Task Issue merge hold active field is malformed')
    return values


def _comments_read(repository, issue_number):
    try:
        pages = json.loads(run(
            'gh', 'api', f'repos/{repository}/issues/{issue_number}/comments?per_page=100',
            '--paginate', '--slurp',
        ))
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        raise AuthorityPending(f'live Task Issue comment read is pending: {exc}') from exc
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        raise AuthorityPending('live Task Issue comment pagination is incomplete or malformed')
    comments = [comment for page in pages for comment in page]
    if any(not isinstance(comment, dict) for comment in comments):
        raise AuthorityPending('live Task Issue comment pagination contains malformed entries')
    return {'complete': True, 'repository': repository,
            'issue_number': issue_number, 'comments': comments}


@contextlib.contextmanager
def _trusted_base_checkout(repo_root, commit):
    """Load policy/publication resolvers only from the event's trusted base OID."""
    with tempfile.TemporaryDirectory(prefix='oasis7-loop-base-') as temporary:
        root = Path(temporary) / 'trusted-base'
        subprocess.run(
            ['git', '-C', str(repo_root), 'worktree', 'add', '--detach', str(root), commit],
            check=True, capture_output=True,
        )
        try:
            yield root
        finally:
            subprocess.run(
                ['git', '-C', str(repo_root), 'worktree', 'remove', str(root)],
                check=True, capture_output=True,
            )


def _import_from_trusted_base(trusted_root, module_name):
    helper_path = str(Path(trusted_root) / 'scripts/pm')
    sys.path.insert(0, helper_path)
    old_dont_write_bytecode = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        return importlib.import_module(module_name)
    except (ImportError, OSError) as exc:
        raise ValueError(f'trusted base resolver {module_name} is unavailable: {exc}') from exc
    finally:
        sys.dont_write_bytecode = old_dont_write_bytecode
        sys.path.remove(helper_path)


def _live_pr_publication_binding(args, pr, issue, number, uid, binding,
                                 repository_info, default_branch):
    body = (issue.get('body') or '').replace('\r\n', '\n')
    if (issue.get('number') != int(number) or str(issue.get('state') or '').lower() != 'open'
            or re.findall(r'(?m)^task_uid:[^\n]*$', body) != ['task_uid: ' + uid]
            or re.findall(r'(?m)^task_uid: (task_[0-9a-f]{32})$', body) != [uid]):
        raise ValueError('live Task Issue identity is missing, ambiguous, or not OPEN')
    if not isinstance(issue.get('user'), dict):
        raise ValueError('live Task Issue author identity is unavailable')
    if (not isinstance(repository_info, dict) or type(repository_info.get('id')) is not int
            or repository_info['id'] < 1):
        raise AuthorityPending('live repository identity is unavailable')
    head = pr.get('head') if isinstance(pr.get('head'), dict) else {}
    base = pr.get('base') if isinstance(pr.get('base'), dict) else {}
    head_repo = head.get('repo') if isinstance(head.get('repo'), dict) else {}
    base_repo = base.get('repo') if isinstance(base.get('repo'), dict) else {}
    if (pr.get('number') != args.pr_number or str(pr.get('state') or '').lower() != 'open'
            or pr.get('merged_at') is not None or pr.get('draft') is not True
            or pr.get('html_url') != f'https://github.com/{args.repository}/pull/{args.pr_number}'
            or head.get('sha') != args.head or not head.get('ref')
            or base.get('ref') != default_branch
            or head_repo.get('full_name') != args.repository
            or base_repo.get('full_name') != args.repository
            or head_repo.get('id') != repository_info['id']
            or base_repo.get('id') != repository_info['id']):
        raise ValueError('live PR is not the exact same-repository open draft/event/default-base candidate')
    pr_body = (pr.get('body') if isinstance(pr.get('body'), str) else '').replace('\r\n', '\n')
    uid_tokens = set(re.findall(r'task_[0-9a-f]{32}', pr_body))
    task_fields = re.findall(r'(?m)^[ \t]*Task[ \t]*:[^\r\n]*$', pr_body)
    task_refs = re.findall(r'(?m)^\s*Task: (task_[0-9a-f]{32})\s*$', pr_body)
    refs = re.findall(r'(?m)^\s*Refs #([1-9][0-9]*)\s*$', pr_body)
    closing_refs = re.findall(r'(?im)^\s*(?:Fixes|Closes) #([1-9][0-9]*)\s*$', pr_body)
    if (uid_tokens != {uid} or len(task_fields) != 1 or task_refs != [uid]
            or refs != [str(number)] or closing_refs):
        raise ValueError('live PR Task/Refs identity is missing, ambiguous, or closing')
    status, phase = _issue_scalar(body, 'status'), _issue_scalar(body, 'workflow_phase')
    task_pr_number, task_pr_url = _task_pr_number(body), _issue_scalar(body, 'pr_url')
    if (task_pr_number is None) != (task_pr_url is None):
        raise ValueError('live Task Issue PR number/URL pair is partial')
    hold_values = _live_task_hold(body)
    normalized = {
        'repository': args.repository, 'number': pr.get('number'), 'url': pr.get('html_url'),
        'state': str(pr.get('state') or '').lower(), 'merged': pr.get('merged_at') is not None,
        'draft': pr.get('draft'), 'source_ref': head.get('ref'), 'target_ref': base.get('ref'),
        'source_head_oid': head.get('sha'), 'task_uid': uid, 'issue_number': int(number),
        'created_at': pr.get('created_at'), 'updated_at': pr.get('updated_at'),
        'task_status': status, 'task_phase': phase, 'task_pr_number': task_pr_number,
        'task_pr_url': task_pr_url,
        'pr_author': (pr.get('user') or {}).get('login') if isinstance(pr.get('user'), dict) else None,
        'pr_author_type': (pr.get('user') or {}).get('type') if isinstance(pr.get('user'), dict) else None,
    }
    if binding is not None and binding.get('bootstrap_epoch') is None:
        raise ValueError('live loop binding lacks bootstrap epoch')
    return normalized, hold_values


def _resolve_active_binding(args, binding, issue, pr, number, uid, comments_read,
                            *, task_pr_binding, hold_values):
    if binding is None:
        return None
    with _trusted_base_checkout(args.repo_root, getattr(args, 'tool_revision', args.base)) as trusted_root:
        policy = _import_from_trusted_base(trusted_root, 'loop_policy')
        try:
            current = policy.current_effective_policy_identity(Path(args.repo_root), args.repository)
        except (OSError, ValueError, subprocess.CalledProcessError) as exc:
            raise AuthorityPending(f'trusted default-branch policy read is pending: {exc}') from exc
        branch = current.get('default_branch')
        if not isinstance(branch, str) or not branch:
            raise AuthorityPending('trusted default-branch identity is unavailable')
        checked = subprocess.run(
            ['git', '-C', str(args.repo_root), 'check-ref-format', f'refs/heads/{branch}'],
            capture_output=True,
        )
        if checked.returncode:
            raise ValueError('trusted default-branch name is malformed')
        subprocess.run(
            ['git', '-C', str(args.repo_root), 'fetch', '--no-tags', 'origin',
             f'+refs/heads/{branch}:refs/remotes/origin/{branch}'],
            check=True, capture_output=True,
        )
        fetched = subprocess.check_output(
            ['git', '-C', str(args.repo_root), 'rev-parse', f'refs/remotes/origin/{branch}'],
            text=True,
        ).strip()
        if fetched != current.get('default_branch_oid'):
            raise AuthorityPending('fetched trusted default branch differs from live policy proof')
        live = {
            'repository': args.repository, 'issue_number': issue['number'],
            'task_uid': uid, 'bootstrap_epoch': binding.get('bootstrap_epoch'),
            'binding_identity_digest': policy.binding_identity_digest(binding),
            'write_scope_digest': policy.write_scope_digest(binding),
            'owner_role': binding.get('owner_role'), 'task_branch': pr['head']['ref'],
            'default_branch': branch, 'pr_number': task_pr_binding[0], 'pr_url': task_pr_binding[1],
            'status': _issue_scalar(issue.get('body') or '', 'status'),
            'workflow_phase': _issue_scalar(issue.get('body') or '', 'workflow_phase'),
            'hold_active': str(hold_values.get('active', 'false')).lower() == 'true',
        }
        result = policy.resolve_effective_policy(
            Path(args.repo_root), binding, comments_read, live, current,
        )
        if result.get('status') == 'pending':
            raise AuthorityPending('; '.join(result.get('blockers') or ['policy adoption evidence is pending']))
        if result.get('status') != 'passed' or not isinstance(result.get('binding'), dict):
            raise ValueError('; '.join(result.get('blockers') or ['effective policy resolution is blocked']))
        return result['binding']


def validate_c1_publication(args, pr, issue, number, uid, binding,
                            comments_read, repository_info, default_branch):
    """Require a uniquely read-back C1 publication plus live Task/PR identity."""
    required = (args.scope_base_oid, args.planner_config_sha256, args.planner_digest,
                args.projection_digest)
    if any(value is None for value in required):
        raise ValueError('C1 publication validation lacks trusted v2 planner/projection inputs')
    if (not re.fullmatch(r'[0-9a-f]{40,64}', args.scope_base_oid)
            or not re.fullmatch(r'[0-9a-f]{40,64}', args.base)
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', args.planner_config_sha256)
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', args.planner_digest)
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', args.projection_digest)):
        raise ValueError('C1 publication planner/projection identity is malformed')
    if (not isinstance(repository_info, dict) or type(repository_info.get('id')) is not int
            or repository_info.get('default_branch') != default_branch):
        raise AuthorityPending('live repository/default-branch identity is incomplete')
    normalized, hold_values = _live_pr_publication_binding(
        args, pr, issue, number, uid, binding, repository_info, default_branch,
    )
    active = _resolve_active_binding(
        args, binding, issue, pr, number, uid, comments_read,
        task_pr_binding=(normalized['task_pr_number'], normalized['task_pr_url']),
        hold_values=hold_values,
    )
    with _trusted_base_checkout(args.repo_root, getattr(args, 'tool_revision', args.base)) as trusted_root:
        publication = _import_from_trusted_base(trusted_root, 'pr_projection_publication')
        expected = {
            'repository': args.repository,
            'repository_id': repository_info['id'], 'task_uid': uid,
            'bootstrap_epoch': binding.get('bootstrap_epoch') if binding is not None else None,
            'source_repository_id': repository_info['id'],
            'source_ref': normalized['source_ref'], 'target_ref': normalized['target_ref'],
            'source_head_oid': args.head, 'source_scope_oid': args.scope_base_oid,
            'planner_authority_oid': args.base,
            'planner_config_sha256': args.planner_config_sha256,
            'policy_digest': args.planner_digest,
            'projection_digest': args.projection_digest,
        }
        result = publication.resolve_task_publication(
            comments_read, expected, live_task_author=issue.get('user'), pr_binding=normalized,
        )
    if result.get('status') == 'pending':
        raise AuthorityPending('; '.join(result.get('blockers') or ['C1 publication read is pending']))
    if result.get('status') != 'passed':
        raise ValueError('; '.join(result.get('blockers') or ['trusted C1 publication is blocked']))
    return active


def write_output(name, value):
    output_path = os.environ.get('GITHUB_OUTPUT')
    if not output_path:
        return
    with Path(output_path).open('a', encoding='utf-8') as output:
        output.write(f'{name}={value}\n')


def run(*args):
    try:
        return subprocess.check_output(args, text=True).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        if args and args[0] == 'gh':
            raise AuthorityPending(f'live GitHub read is unavailable: {exc}') from exc
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo-root', type=Path, default=Path.cwd())
    parser.add_argument('--repository', required=True)
    parser.add_argument('--pr-number', required=True, type=int)
    parser.add_argument('--base', required=True)
    parser.add_argument('--head', required=True)
    parser.add_argument('--phase', choices=('legacy', 'start', 'final'), default='legacy')
    parser.add_argument('--tests-passed', action='store_true')
    parser.add_argument('--start-only', action='store_true')
    parser.add_argument('--scope-base-oid')
    parser.add_argument('--planner-config-sha256')
    parser.add_argument('--planner-digest')
    parser.add_argument('--projection-digest')
    parser.add_argument('--maintenance-authority-comment-id', type=int)
    args = parser.parse_args()
    if args.start_only and args.phase != 'final':
        parser.error('--start-only is only valid with --phase final')
    if args.phase == 'final' and not args.tests_passed:
        print(json.dumps({'status': 'blocked', 'blockers': [
            'final binding verification requires the required test tier to have completed successfully',
        ]}))
        return 2
    try:
        pr = json.loads(run('gh', 'api', f'repos/{args.repository}/pulls/{args.pr_number}'))
        if pr['head']['sha'] != args.head: raise ValueError('live PR HEAD differs from CI source HEAD')
        uids = set(re.findall(r'task_[0-9a-f]{32}', pr.get('body') or ''))
        candidate_items = {}
        if len(uids) == 1:
            uid = next(iter(uids))
            refs = set(re.findall(r'(?:Refs|Fixes|Closes)\s+#(\d+)', pr.get('body') or '', re.I))
            if len(refs) > 20: raise ValueError('task reference discovery budget exhausted')
            if refs:
                candidates = sorted(int(reference) for reference in refs)
            else:
                hits = json.loads(run('gh', 'issue', 'list', '-R', args.repository, '--state', 'all', '--search', uid + ' in:body', '--json', 'number', '--limit', '5'))
                if not isinstance(hits, list) or len(hits) >= 5: raise ValueError('task search discovery incomplete')
                candidates = [hit['number'] for hit in hits]
            matches = []
            for candidate in candidates:
                item = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{candidate}'))
                candidate_items[candidate] = item
                candidate_body = (item.get('body') or '').replace('\r\n', '\n')
                candidate_fields = re.findall(r'^task_uid:[^\n]*$', candidate_body, re.MULTILINE)
                candidate_uids = re.findall(r'^task_uid: (task_[0-9a-f]{32})$', candidate_body, re.MULTILINE)
                if uid in candidate_uids:
                    if candidate_fields != ['task_uid: ' + uid] or candidate_uids != [uid]: raise ValueError('ambiguous canonical Issue UID')
                    matches.append(candidate)
            if len(matches) != 1: raise ValueError('live task Issue is ambiguous or missing')
            number = matches[0]
        else:
            refs = set(re.findall(r'(?:Refs|Fixes|Closes)\s+#(\d+)', pr.get('body') or '', re.I))
            if uids or len(refs) != 1: raise ValueError('PR must identify exactly one canonical task Issue')
            number, uid = int(next(iter(refs))), None
        candidate_issue = candidate_items.get(number) if uids else None
        issue = (candidate_issue
                 if isinstance(candidate_issue, dict)
                 and 'loop_binding_b64:' in (candidate_issue.get('body') or '')
                 else None)
        if issue is None:
            issue = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{number}'))
        body = (issue.get('body') or '').replace('\r\n', '\n')
        issue_fields = re.findall(r'^task_uid:[^\n]*$', body, re.MULTILINE)
        issue_uids = re.findall(r'^task_uid: (task_[0-9a-f]{32})$', body, re.MULTILINE)
        if len(issue_fields) != 1 or len(issue_uids) != 1: raise ValueError('Issue UID missing or ambiguous')
        if uid is None:
            if len(issue_uids) != 1: raise ValueError('Issue UID missing')
            uid = issue_uids[0]
        if issue_uids != [uid]: raise ValueError('Issue UID mismatch')
        args.maintenance_context = None
        if args.maintenance_authority_comment_id is not None:
            with _trusted_base_checkout(args.repo_root, args.head) as candidate_root:
                maintenance = _import_from_trusted_base(candidate_root, 'workflow_maintenance')
                scope_base = args.scope_base_oid or subprocess.check_output(
                    ['git', '-C', str(args.repo_root), 'merge-base', args.base, args.head], text=True).strip()
                changed = subprocess.check_output(
                    ['git', '-C', str(args.repo_root), 'diff', '--name-only', scope_base, args.head], text=True).splitlines()
                args.maintenance_context = maintenance.read_maintenance_authority(
                    args.repository, args.maintenance_authority_comment_id, uid, args.pr_number,
                    args.head, changed, maintenance.TOOL_PATHS)
                maintenance.validate_candidate_tool_root(candidate_root, args.repo_root, args.maintenance_context)
            args.tool_revision = args.head
        body = await_pr_binding(
            args, pr, issue, number, uid,
            wait_for_initial=args.phase == 'legacy',
            allow_missing_initial=(args.phase == 'start'
                                   or (args.phase == 'final' and args.start_only)),
        )
        start_only = False
        active_binding = None
        binding = None
        default_branch = None
        if args.phase in {'start', 'final'}:
            issue_pr_number = _task_pr_number(body)
            issue_pr_url = _issue_scalar(body, 'pr_url')
            if (issue_pr_number is None) != (issue_pr_url is None):
                raise ValueError('live Task Issue PR number/URL pair is partial')
            if issue_pr_number is None:
                if args.phase == 'final':
                    if args.start_only:
                        raise PublicationPending(
                            'the Task PR number/URL binding has not completed after required tests',
                        )
                    raise ValueError('final Task Issue PR number/URL binding is missing')
                start_only = True
            else:
                validate_full_binding(args, pr, issue, number, uid)

        if 'loop_binding_b64:' in body:
            matches = re.findall(r'^- loop_binding_b64: `([^`]+)`$', body, re.MULTILINE)
            if len(matches) != 1:
                raise ValueError('malformed live binding')
            binding = json.loads(base64.b64decode(
                matches[0] + '=' * (-len(matches[0]) % 4),
                altchars=b'-_', validate=True,
            ))
            if binding.get('task_uid') != uid:
                raise ValueError('loop task UID mismatch')

        if args.phase == 'start' and start_only:
            comments_read = _comments_read(args.repository, number)
            try:
                repository_info = json.loads(run('gh', 'api', f'repos/{args.repository}'))
            except (json.JSONDecodeError, TypeError) as exc:
                raise AuthorityPending(f'live repository identity is unavailable: {exc}') from exc
            default_branch = (repository_info.get('default_branch')
                              if isinstance(repository_info, dict) else None)
            if not isinstance(default_branch, str) or not default_branch:
                raise AuthorityPending('live repository default branch is unavailable')
            active_binding = validate_c1_publication(
                args, pr, issue, number, uid, binding, comments_read,
                repository_info, default_branch,
            )
        elif args.phase in {'start', 'final'} and binding is not None:
            # Fully bound runs retain the established full identity/content
            # gate. Resolve an adopted policy pin, but do not require the
            # validation-start-only C1 marker again.
            comments_read = _comments_read(args.repository, number)
            try:
                repository_info = json.loads(run('gh', 'api', f'repos/{args.repository}'))
            except (json.JSONDecodeError, TypeError) as exc:
                raise AuthorityPending(f'live repository identity is unavailable: {exc}') from exc
            default_branch = (repository_info.get('default_branch')
                              if isinstance(repository_info, dict) else None)
            if not isinstance(default_branch, str) or not default_branch:
                raise AuthorityPending('live repository default branch is unavailable')
            issue_body = (issue.get('body') or '').replace('\r\n', '\n')
            task_pr_binding = (_task_pr_number(issue_body), _issue_scalar(issue_body, 'pr_url'))
            if (task_pr_binding[0] is None) != (task_pr_binding[1] is None):
                raise ValueError('live Task Issue PR number/URL pair is partial')
            active_binding = _resolve_active_binding(
                args, binding, issue, pr, number, uid, comments_read,
                task_pr_binding=task_pr_binding,
                hold_values=_live_task_hold(issue_body),
            )
        if 'loop_binding_b64:' not in body:
            pages = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{number}/comments', '--paginate', '--slurp'))
            if not isinstance(pages, list): raise ValueError('lineage readback unavailable')
            comments = [item for page in pages for item in (page if isinstance(page, list) else [page])]
            if any('oasis7-loop-binding-history' in str(item.get('body', '')) for item in comments if isinstance(item, dict)):
                raise ValueError('loop binding deleted after immutable history')
            print(json.dumps({'status': 'legacy', 'task_uid': uid}))
            if args.phase == 'start':
                write_output('start_only', str(start_only).lower())
            return 0
        if binding is None:
            raise ValueError('loop binding disappeared during live readback')
        effective_binding = active_binding if isinstance(active_binding, dict) else binding
        commit = getattr(args, 'tool_revision', effective_binding.get('policy_commit', ''))
        if not re.fullmatch(r'[0-9a-f]{40}', commit): raise ValueError('missing immutable effective policy')
        if default_branch is None:
            try:
                repository_info = json.loads(run('gh', 'api', f'repos/{args.repository}'))
            except (json.JSONDecodeError, TypeError) as exc:
                raise AuthorityPending(f'live repository identity is unavailable: {exc}') from exc
            default_branch = (repository_info.get('default_branch')
                              if isinstance(repository_info, dict) else None)
        if not isinstance(default_branch, str) or not default_branch:
            raise AuthorityPending('live repository default branch is unavailable')
        checked = subprocess.run(
            ['git', '-C', str(args.repo_root), 'check-ref-format', f'refs/heads/{default_branch}'],
            capture_output=True,
        )
        if checked.returncode:
            raise ValueError('live repository default branch name is malformed')
        try:
            subprocess.run(
                ['git', '-C', str(args.repo_root), 'fetch', '--no-tags', 'origin',
                 f'+refs/heads/{default_branch}:refs/remotes/origin/{default_branch}'],
                check=True, capture_output=True,
            )
            subprocess.run(
                ['git', '-C', str(args.repo_root), 'merge-base', '--is-ancestor', effective_binding['policy_commit'],
                 f'refs/remotes/origin/{default_branch}'],
                check=True, capture_output=True,
            )
        except subprocess.CalledProcessError as exc:
            raise AuthorityPending(f'effective loop policy ancestry read is unavailable: {exc}') from exc
        with tempfile.TemporaryDirectory(prefix='oasis7-loop-tools-') as tmp:
            tool_root = Path(tmp) / 'tools'
            subprocess.run(['git', '-C', str(args.repo_root), 'worktree', 'add', '--detach', str(tool_root), commit], check=True, capture_output=True)
            try:
                # All Python code below is loaded from the verified effective commit.
                code = r'''
import json
import sys
from pathlib import Path

# The detached effective tool root must remain byte-for-byte clean until its
# explicit Git worktree removal. Do not let imports create interpreter caches.
sys.dont_write_bytecode = True

from loop_ci_content import validate_ci_content
from loop_traceability import GitHubAuthorityReader, _validate_authority_record


def coordinating_record(root, binding, repository):
    reference = binding.get("coordination_ref")
    if reference is None:
        return None
    if not isinstance(reference, dict):
        raise ValueError("bound coordinating reference is not an object")
    readback = GitHubAuthorityReader(root, repository)(reference)
    comment = readback.get("comment")
    try:
        payload = json.loads(comment.get("body") or "") if isinstance(comment, dict) else None
    except (TypeError, json.JSONDecodeError) as exc:
        raise ValueError("coordinating authority body is not valid JSON") from exc
    record = payload.get("record") if isinstance(payload, dict) else None
    if not isinstance(record, dict):
        raise ValueError("coordinating authority comment has no usable record projection")
    blockers = _validate_authority_record(readback, record, binding)
    if blockers:
        raise ValueError("coordinating authority readback blocked: " + "; ".join(blockers))
    return record


t, r, b, base, head, repo, maintenance_raw = sys.argv[1:]
try:
    binding = json.loads(b)
    record = coordinating_record(Path(r), binding, repo)
    result = validate_ci_content(
        Path(t), Path(r), binding, base, head, repo, record=record,
        maintenance=json.loads(maintenance_raw),
    )
except (OSError, ValueError, KeyError, TypeError) as exc:
    result = {"status": "blocked", "blockers": [str(exc)]}
print(json.dumps(result))
sys.exit(0 if result["status"] == "passed" else 2)
'''
                result = subprocess.run([sys.executable, '-c', code, str(tool_root), str(args.repo_root.resolve()), json.dumps(effective_binding), args.base, args.head, args.repository,
                                         json.dumps(args.maintenance_context)], cwd=tool_root / 'scripts/pm')
                if result.returncode == 0 and args.phase == 'start':
                    write_output('start_only', str(start_only).lower())
                return result.returncode
            finally:
                subprocess.run(['git', '-C', str(args.repo_root), 'worktree', 'remove', str(tool_root)], check=True, capture_output=True)
    except PublicationPending as exc:
        print(json.dumps({'status': 'publication_pending', 'blockers': [str(exc)]}))
        return 2
    except AuthorityPending as exc:
        print(json.dumps({'status': 'pending', 'blockers': [str(exc)]}))
        return 2
    except json.JSONDecodeError as exc:
        print(json.dumps({'status': 'pending', 'blockers': [f'live GitHub response is malformed: {exc}']}))
        return 2
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as exc:
        print(json.dumps({'status': 'blocked', 'blockers': [str(exc)]}))
        return 2


if __name__ == '__main__': raise SystemExit(main())
