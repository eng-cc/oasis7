#!/usr/bin/env python3
"""Explicit, conservative Git resource cleanup; dry run unless --execute."""
import argparse
import json
import os
import pathlib
import subprocess
import ctypes
import errno
import re
import sys
import time
from urllib.parse import urlsplit

class CleanupError(RuntimeError):
    pass

def _run(args, check=True, timeout=10):
    try:
        result = subprocess.run(args, text=True, capture_output=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise CleanupError(str(exc)) from exc
    if check and result.returncode:
        raise CleanupError(result.stderr.strip() or result.stdout.strip())
    return result

def git(repo, *args):
    return _run(['git', '-C', str(repo), *args]).stdout.strip()

def process_identity(pid):
    """Native start identity; only ESRCH proves an unreadable process gone."""
    try:
        if sys.platform.startswith('linux'):
            raw = pathlib.Path(f'/proc/{pid}/stat').read_text()
            fields = raw[raw.rindex(')') + 2:].split()
            return (pid, int(fields[19]), fields[0])
        if sys.platform == 'darwin':
            class BSDInfo(ctypes.Structure):
                _fields_ = [('ids', ctypes.c_uint32 * 12), ('comm', ctypes.c_char * 16),
                            ('name', ctypes.c_char * 32), ('tail', ctypes.c_uint32 * 6),
                            ('sec', ctypes.c_uint64), ('usec', ctypes.c_uint64)]
            lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
            info = BSDInfo()
            result = lib.proc_pidinfo(pid, 3, 0, ctypes.byref(info), ctypes.sizeof(info))
            if result != ctypes.sizeof(info) or info.ids[3] != pid or not info.sec:
                raise ValueError('short or invalid libproc identity')
            return (pid, (info.sec, info.usec), str(info.ids[1]))
        raise ValueError('unsupported native process identity')
    except (OSError, ValueError, IndexError):
        try:
            os.kill(pid, 0)
        except OSError as exc:
            if exc.errno == errno.ESRCH:
                return None
            raise CleanupError(f'PID {pid}: identity unreadable ({exc.strerror})') from exc
        raise CleanupError(f'PID {pid}: alive but native identity unreadable')


def snapshot_processes(deadline):
    result = _run(['ps', '-axo', 'pid='], timeout=min(2, max(.01, deadline-time.monotonic())))
    identities = {}
    for line in result.stdout.splitlines():
        if time.monotonic() >= deadline:
            raise CleanupError('process inspection budget exhausted')
        try:
            pid = int(line.strip())
        except ValueError as exc:
            raise CleanupError('malformed ps PID') from exc
        identity = process_identity(pid)
        if identity is not None:
            if pid in identities:
                raise CleanupError('duplicate ps PID')
            identities[pid] = identity
    if not identities:
        raise CleanupError('empty process census')
    return identities


def read_open_files(pids, path, deadline):
    result = _run(['lsof', '-n', '-P', '-F', 'pfnt', '-p', ','.join(map(str, pids))],
                  check=False, timeout=min(10, max(.01, deadline-time.monotonic())))
    covered, busy = set(), set()
    pid, fd, kind, name = None, None, None, None
    nonfiles = {'PIPE', 'NPOLICY', 'NEXUS', 'KQUEUE', 'unix', 'IPv4', 'IPv6', 'systm', 'CHAN', 'sock'}
    target = path.resolve()
    def finish():
        if fd is None:
            return
        if kind is None or (name is None and kind not in nonfiles):
            raise CleanupError(f'PID {pid}: incomplete descriptor')
        if kind in nonfiles:
            return
        if kind not in {'REG', 'DIR', 'CHR', 'BLK', 'FIFO', 'LINK'}:
            raise CleanupError(f'PID {pid}: unknown descriptor type {kind}')
        if not name or not name.startswith('/'):
            raise CleanupError(f'PID {pid}: ambiguous file path')
        value = re.sub(r' \((deleted|revoked)\)$', '', name)
        try:
            pathlib.Path(value).resolve().relative_to(target)
            busy.add(pid)
        except ValueError:
            pass
        if fd == 'cwd':
            covered.add(pid)
    for line in result.stdout.splitlines():
        if not line:
            continue
        field, value = line[0], line[1:]
        if field == 'p':
            finish()
            try:
                pid = int(value)
            except ValueError as exc:
                raise CleanupError('invalid lsof PID') from exc
            if pid not in pids:
                raise CleanupError('unexpected lsof PID')
            fd = kind = name = None
        elif field == 'f':
            finish()
            if pid is None or not value:
                raise CleanupError('invalid lsof descriptor')
            fd, kind, name = value, None, None
        elif field == 't' and fd is not None and kind is None and value:
            kind = value
        elif field == 'n' and fd is not None and name is None:
            name = value
        else:
            raise CleanupError('unknown or malformed lsof field')
    finish()
    # lsof exit 1 can mean no surviving requested PIDs; identity reconciliation
    # below decides coverage. Other errors and warnings remain unknown.
    if result.returncode not in (0, 1) or result.stderr.strip():
        raise CleanupError('lsof failed or reported incomplete visibility: ' + result.stderr.strip())
    return covered, busy


def inspect_process_use(path):
    if sys.platform not in ('linux', 'darwin'):
        return {'state': 'unknown', 'reason': 'unsupported process inspection platform'}
    deadline = time.monotonic() + 30
    try:
        current = {pid: value[:2] for pid, value in snapshot_processes(deadline).items()}
        evidence = {}
        for _ in range(2):
            pending = {pid: identity for pid, identity in current.items() if evidence.get(pid) != identity}
            if pending:
                covered, busy = read_open_files(pending, path, deadline)
                after = {pid: value[:2] for pid, value in snapshot_processes(deadline).items()}
                for pid in busy:
                    if pid in pending and after.get(pid) == pending[pid]:
                        return {'state': 'busy', 'reason': f'PID {pid} holds target cwd/file'}
                for pid in covered:
                    if pid in pending and after.get(pid) == pending[pid]:
                        evidence[pid] = pending[pid]
                current = after
            if time.monotonic() >= deadline:
                raise CleanupError('process inspection budget exhausted')
            if all(evidence.get(pid) == identity for pid, identity in current.items()):
                return {'state': 'clear_observed', 'reason': 'no cwd/file use observed in finite snapshot'}
        missing = [str(pid) for pid, identity in current.items() if evidence.get(pid) != identity]
        raise CleanupError('uncovered live process identities: ' + ', '.join(missing))
    except CleanupError as exc:
        return {'state': 'unknown', 'reason': str(exc)}


def _process_mentions_path(path):
    result = inspect_process_use(path)
    if result['state'] == 'unknown':
        raise CleanupError(result['reason'])
    return result['state'] == 'busy'


def worktrees(repo):
    records = []
    for row in git(repo, 'worktree', 'list', '--porcelain', '-z').split('\0\0'):
        fields = {}
        for entry in row.split('\0'):
            if entry:
                key, _, value = entry.partition(' ')
                fields[key] = value
        if fields:
            records.append(fields)
    return records


def oid(repo, ref):
    return git(repo, 'rev-parse', '--verify', ref + '^{commit}')


def ancestor(repo, older, newer):
    result = _run(['git', '-C', str(repo), 'merge-base', '--is-ancestor', older, newer], check=False)
    if result.returncode not in (0, 1):
        raise CleanupError('cannot verify ancestry')
    return result.returncode == 0


def user_material(repo, path, records):
    if git(path, 'status', '--porcelain', '--untracked-files=all'):
        raise CleanupError('uncommitted or untracked work retained')
    canonical = pathlib.Path(records[0]['worktree']).resolve()
    cache = canonical.parent / '.oasis7-cache' / 'cargo-target'
    ignored = git(path, 'ls-files', '--others', '--ignored', '--exclude-standard', '-z').split('\0')
    for item in filter(None, ignored):
        if item == 'config.toml':
            source, copy = canonical / item, path / item
            if canonical == path or source.is_symlink() or copy.is_symlink() or not source.is_file() or source.read_bytes() != copy.read_bytes():
                raise CleanupError('modified or unverifiable ignored config retained')
        elif item.rstrip('/') == 'target' and (path / 'target').is_symlink():
            destination = (path / 'target').resolve()
            try:
                destination.relative_to(cache.resolve())
            except ValueError as exc:
                raise CleanupError('target does not point into known external build cache') from exc
            if not destination.is_dir() or destination == cache.resolve() or destination.is_relative_to(path):
                raise CleanupError('target cache identity unclear')
        else:
            raise CleanupError('unknown ignored user material retained: ' + item)


def github_repository(remote):
    """Resolve only an exact GitHub host and owner/repository origin."""
    if '://' in remote:
        parsed = urlsplit(remote)
        if parsed.scheme not in {'https', 'http', 'ssh', 'git'} or parsed.hostname != 'github.com' or parsed.query or parsed.fragment:
            raise CleanupError('origin repository identity unavailable for PR proof')
        repository = parsed.path.removeprefix('/')
    else:
        match = re.fullmatch(r'(?:[^@/:]+@)?github\.com:([^?#]+)', remote)
        if not match:
            raise CleanupError('origin repository identity unavailable for PR proof')
        repository = match.group(1)
    repository = repository.removesuffix('.git')
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository) or any(part in {'.', '..'} for part in repository.split('/')):
        raise CleanupError('origin repository identity unavailable for PR proof')
    return repository


def delivery(repo, expected, base_ref, pr):
    if base_ref not in {'refs/remotes/origin/main', 'refs/heads/main'}:
        raise CleanupError('invalid main base ref')
    try:
        base = oid(repo, base_ref)
        origin = oid(repo, 'refs/remotes/origin/main')
    except CleanupError as exc:
        raise CleanupError('main baseline missing; fetch normally before cleanup') from exc
    if base_ref == 'refs/heads/main' and not ancestor(repo, base, origin):
        raise CleanupError('local main has not been delivered to origin/main')
    if not ancestor(repo, expected, base):
        if pr is None:
            raise CleanupError('no main ancestry or explicit merged PR proof')
        remote = git(repo, 'remote', 'get-url', 'origin')
        slug = github_repository(remote)
        data = json.loads(_run(['gh', 'api', f'repos/{slug}/pulls/{pr}']).stdout)
        merged = data.get('merge_commit_sha')
        if (data.get('merged') is not True or data.get('head', {}).get('sha') != expected
                or data.get('base', {}).get('ref') != 'main'
                or data.get('base', {}).get('repo', {}).get('full_name', '').lower() != slug.lower()
                or not isinstance(merged, str) or not ancestor(repo, merged, base)):
            raise CleanupError('PR merged/head/base/repository proof mismatch')
    return base, origin


def inspect(repo, path, branch, expected, base_ref='refs/remotes/origin/main', pr=None):
    repo, path = repo.resolve(), path.resolve()
    if branch in {'main', 'master'}:
        raise CleanupError('primary branch retained')
    records = worktrees(repo)
    if not records or path == pathlib.Path(records[0]['worktree']).resolve() or not path.is_dir():
        raise CleanupError('target must be an existing separate worktree')
    wanted = next((r for r in records if r.get('worktree') == str(path)), None)
    if not wanted or wanted.get('branch') != 'refs/heads/' + branch or wanted.get('HEAD') != expected or oid(repo, 'refs/heads/' + branch) != expected or oid(path, 'HEAD') != expected:
        raise CleanupError('worktree path, branch, or expected HEAD mismatch')
    if 'locked' in wanted or 'prunable' in wanted:
        raise CleanupError('locked or ambiguous worktree retained')
    if pathlib.Path(git(path, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve() != pathlib.Path(git(repo, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve():
        raise CleanupError('repository identity mismatch')
    user_material(repo, path, records)
    base, origin = delivery(repo, expected, base_ref, pr)
    upstream = _run(['git', '-C', str(path), 'rev-parse', '--symbolic-full-name', '@{upstream}'], check=False)
    ref = upstream.stdout.strip()
    if upstream.returncode == 0 and ref not in {'refs/remotes/origin/main', 'refs/heads/main'}:
        tip = _run(['git', '-C', str(repo), 'rev-parse', '--verify', ref], check=False)
        if tip.returncode == 0 and tip.stdout.strip() != expected:
            raise CleanupError('independent remote topic changed; retained')
    if _process_mentions_path(path):
        raise CleanupError('worktree is in use')
    return {'worktree': str(path), 'branch': branch, 'head': expected, 'base_ref': base_ref,
            'base_oid': base, 'origin_main_oid': origin, 'process_state': 'clear_observed'}


def execute(repo, path, branch, expected, base_ref, pr, report):
    fresh = inspect(repo, path, branch, expected, base_ref, pr)
    if fresh['base_oid'] != report['base_oid'] or fresh['origin_main_oid'] != report['origin_main_oid']:
        raise CleanupError('base changed before deletion; retained')
    # Recheck identity and materials immediately after the potentially slow process scan.
    records = worktrees(repo)
    wanted = next((r for r in records if r.get('worktree') == str(path.resolve())), {})
    if wanted.get('HEAD') != expected or wanted.get('branch') != 'refs/heads/' + branch or 'locked' in wanted or 'prunable' in wanted or oid(path, 'HEAD') != expected or oid(repo, 'refs/heads/' + branch) != expected or oid(repo, base_ref) != report['base_oid']:
        raise CleanupError('resource identity changed before deletion')
    if pathlib.Path(git(path, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve() != pathlib.Path(git(repo, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve():
        raise CleanupError('repository identity changed before deletion')
    if base_ref == 'refs/heads/main' and oid(repo, 'refs/remotes/origin/main') != report['origin_main_oid']:
        raise CleanupError('origin main changed before deletion')
    user_material(repo, path, records)
    git(repo, 'worktree', 'remove', str(path.resolve()))
    report['worktree_removed'] = True
    if any(r.get('branch') == 'refs/heads/' + branch for r in worktrees(repo)):
        raise CleanupError('branch checked out elsewhere; partial cleanup retained branch')
    commands = ['start', f'verify {base_ref} {report["base_oid"]}']
    if base_ref == 'refs/heads/main':
        commands.append(f'verify refs/remotes/origin/main {report["origin_main_oid"]}')
    commands += [f'delete refs/heads/{branch} {expected}', 'prepare', 'commit', '']
    result = subprocess.run(['git', '-C', str(repo), 'update-ref', '--stdin'], input='\n'.join(commands), text=True, capture_output=True, timeout=10)
    if result.returncode:
        raise CleanupError('ref transaction failed; partial cleanup retained branch: ' + result.stderr.strip())
    remaining = _run(['git', '-C', str(repo), 'show-ref', '--verify', '--quiet', 'refs/heads/' + branch], check=False)
    if remaining.returncode != 1:
        report['branch_state'] = 'retained/recreated'
        raise CleanupError('branch retained/recreated after deletion')
    report['branch_removed'] = True


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog='Finite process snapshots report observed use only. Stop known users and run outside the target. Never fetches, sudo, or forces removal. Exit: 0 eligible/success, 1 blocked/partial, 2 invalid arguments.')
    parser.add_argument('--repo-root', type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument('--worktree', type=pathlib.Path, required=True)
    parser.add_argument('--branch', required=True)
    parser.add_argument('--expected-head', required=True, help='full expected commit OID')
    parser.add_argument('--base-ref', choices=['refs/remotes/origin/main', 'refs/heads/main'], default='refs/remotes/origin/main')
    parser.add_argument('--pr', type=int, help='explicit PR number for squash/rebase merged proof')
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}|[0-9a-f]{64}', args.expected_head) or (args.pr is not None and args.pr <= 0):
        parser.error('expected-head must be full OID; PR must be positive')
    if _run(['git', 'check-ref-format', 'refs/heads/' + args.branch], check=False).returncode:
        parser.error('invalid branch name')
    report = dict(eligible=False, blocked_reasons=[], executed=args.execute, worktree_removed=False, branch_removed=False)
    try:
        report.update(inspect(args.repo_root, args.worktree, args.branch, args.expected_head, args.base_ref, args.pr))
        report['eligible'] = True
        if args.execute:
            execute(args.repo_root, args.worktree, args.branch, args.expected_head, args.base_ref, args.pr, report)
    except (CleanupError, OSError, ValueError, subprocess.TimeoutExpired) as exc:
        report['eligible'] = False
        report['blocked_reasons'].append(str(exc))
    # Read actual remaining resources even on partial failure.
    try:
        report['worktree_registered'] = any(r.get('worktree') == str(args.worktree.resolve()) for r in worktrees(args.repo_root))
        report['worktree_exists'] = args.worktree.exists()
        branch_readback = _run(['git', '-C', str(args.repo_root), 'show-ref', '--verify', '--quiet', 'refs/heads/' + args.branch], check=False)
        if branch_readback.returncode not in (0, 1):
            raise CleanupError('branch readback unavailable')
        report['branch_exists'] = branch_readback.returncode == 0
        if args.execute and report['eligible'] and (report['worktree_registered'] or report['worktree_exists'] or report['branch_exists']):
            report['eligible'] = False
            report['blocked_reasons'].append('resource retained/recreated during final readback')
            report['branch_removed'] = not report['branch_exists']
    except CleanupError as exc:
        report['readback_error'] = str(exc)
        report['eligible'] = False
    print(json.dumps(report))
    return 0 if report['eligible'] else 1

if __name__ == '__main__':
    sys.exit(main())
