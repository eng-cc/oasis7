#!/usr/bin/env python3
"""Explicit, conservative Git resource cleanup; dry run unless --execute."""
import argparse
import json
import os
import pathlib
import subprocess

class CleanupError(RuntimeError):
    pass

def _run(args, check=True):
    try:
        result = subprocess.run(args, text=True, capture_output=True)
    except OSError as exc:
        raise CleanupError(str(exc)) from exc
    if check and result.returncode:
        raise CleanupError(result.stderr.strip() or result.stdout.strip())
    return result

def git(repo, *args):
    return _run(['git', '-C', str(repo), *args]).stdout.strip()

def _process_mentions_path(path: pathlib.Path) -> bool:
    # Inspect argv plus the live process cwd/open files. argv-only scanning
    # misses editors, shells, and agents whose process was started elsewhere
    # and later changed cwd into the task worktree. Unknown process readback
    # retains the worktree; an empty scan still cannot prove every external
    # App or Agent session has stopped.
    result = _run(["ps", "-axo", "pid=,ppid=,uid=,command="], check=False)
    if result.returncode:
        raise CleanupError("process-use readback is unavailable")
    rows: list[tuple[int, int, int, str]] = []
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        fields = line.strip().split(None, 3)
        if len(fields) != 4:
            raise CleanupError("process-use readback contains a malformed process row")
        try:
            rows.append((int(fields[0]), int(fields[1]), int(fields[2]), fields[3]))
        except ValueError:
            raise CleanupError("process-use readback contains an invalid process identity")
    if not rows or len({pid for pid, _ppid, _uid, _command in rows}) != len(rows):
        raise CleanupError("process-use readback has no processes or duplicate process identities")
    ancestors = {os.getpid()}
    parent = os.getppid()
    while parent > 1 and parent not in ancestors:
        ancestors.add(parent)
        next_parent = next((ppid for pid, ppid, _uid, _command in rows if pid == parent), 1)
        parent = next_parent
    for pid, _ppid, _uid, command in rows:
        if pid in ancestors:
            continue
        if str(path) in command:
            return True

    # Do not filter by UID: another user's process can retain a worktree cwd
    # or open file even when its argv omits the path. If lsof cannot prove full
    # coverage for every visible process, cleanup must retain the worktree.
    pids = [pid for pid, _ppid, _uid, _command in rows]
    # lsof reports a blank NAME for ordinary macOS non-filesystem descriptors
    # (for example PIPE, NPOLICY, NEXUS, and KQUEUE). Keep type information so
    # those records do not make every scan ambiguous, while still retaining
    # on unknown or filesystem descriptors whose path cannot be read.
    open_files = _run(["lsof", "-n", "-F", "pfnt", "-p", ",".join(map(str, pids))], check=False)
    seen_cwd: set[int] = set()
    current_pid: int | None = None
    current_fd = ""
    current_type = ""
    current_has_name = False
    target = path.resolve(strict=False)
    non_filesystem_types = {"PIPE", "NPOLICY", "NEXUS", "KQUEUE", "unix", "IPv4", "IPv6", "systm"}
    empty_name_non_filesystem_types = {"PIPE", "NPOLICY", "NEXUS", "KQUEUE"}

    def path_is_inside(value: str) -> bool:
        for suffix in (" (deleted)", " (revoked)"):
            if value.endswith(suffix):
                value = value[:-len(suffix)]
        if not value.startswith("/"):
            return False
        try:
            candidate = pathlib.Path(value).resolve(strict=False)
            candidate.relative_to(target)
            return True
        except ValueError:
            return False
        except (OSError, RuntimeError) as exc:
            raise CleanupError("process-use file path cannot be resolved") from exc

    known_pids = set(pids)
    seen_pids: set[int] = set()

    def finish_descriptor() -> None:
        if current_fd and (not current_type or not current_has_name):
            raise CleanupError("process-use descriptor path/type readback is incomplete")

    for line in open_files.stdout.splitlines():
        if not line:
            continue
        field, value = line[0], line[1:]
        if field == "p":
            finish_descriptor()
            try:
                current_pid = int(value)
            except ValueError as exc:
                raise CleanupError("process-use open-file readback has an invalid PID") from exc
            if current_pid not in known_pids:
                raise CleanupError("process-use open-file readback returned an unexpected PID")
            if current_pid in seen_pids:
                raise CleanupError("process-use open-file readback repeated a process identity")
            seen_pids.add(current_pid)
            current_fd = ""
            current_type = ""
            current_has_name = False
        elif field == "f":
            if current_pid is None:
                raise CleanupError("process-use open-file readback has no process identity")
            finish_descriptor()
            if not value:
                raise CleanupError("process-use open-file readback has an empty descriptor")
            current_fd = value
            current_type = ""
            current_has_name = False
        elif field == "t":
            if current_pid is None or not current_fd or current_type or not value:
                raise CleanupError("process-use descriptor type readback is incomplete")
            current_type = value
        elif field == "n":
            if current_pid is None or not current_fd or not current_type or current_has_name:
                raise CleanupError("process-use descriptor name readback is incomplete")
            current_has_name = True
            if not value:
                if current_fd == "cwd" or current_type not in empty_name_non_filesystem_types:
                    raise CleanupError("process-use open-file path is ambiguous")
                continue
            if value == "??":
                raise CleanupError("process-use open-file path is ambiguous")
            if current_fd == "cwd":
                if not value.startswith("/"):
                    raise CleanupError("process-use cwd readback is ambiguous")
                seen_cwd.add(current_pid)
            elif not value.startswith("/") and current_type not in non_filesystem_types:
                raise CleanupError("process-use open-file path is ambiguous")
            if path_is_inside(value):
                return True
        else:
            raise CleanupError("process-use open-file readback contains an unknown field")

    finish_descriptor()
    if open_files.returncode:
        raise CleanupError("process-use cwd/open-file readback is unavailable")
    if seen_pids != known_pids:
        raise CleanupError("process-use readback did not cover every visible process")
    if seen_cwd != known_pids:
        # Every visible process must have an inspectable cwd. A disappeared
        # PID, denied lsof read, or incomplete response remains unknown.
        raise CleanupError("process-use cwd readback is incomplete")
    return False


def inspect(repo, path, branch, expected):
    repo, path = repo.resolve(), path.resolve()
    if branch in {'main', 'master'}:
        raise CleanupError('primary branch retained')
    if path == repo or not path.is_dir():
        raise CleanupError('target must be an existing separate worktree')
    rows = git(repo, 'worktree', 'list', '--porcelain', '-z').split('\0\0')
    wanted = None
    for row in rows:
        fields = dict(line.split(' ', 1) for line in row.split('\0') if ' ' in line)
        if fields.get('worktree') == str(path):
            wanted = fields
            break
    if not wanted or wanted.get('branch') != 'refs/heads/' + branch or wanted.get('HEAD') != expected:
        raise CleanupError('worktree path, branch, or expected HEAD mismatch')
    if 'locked' in wanted or 'prunable' in wanted:
        raise CleanupError('locked or ambiguous worktree retained')
    if pathlib.Path(git(path, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve() != pathlib.Path(git(repo, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve():
        raise CleanupError('repository identity mismatch')
    if git(path, 'status', '--porcelain', '--untracked-files=all'):
        raise CleanupError('uncommitted or untracked work retained')
    ignored = git(path, 'ls-files', '--others', '--ignored', '--exclude-standard', '-z').split('\0')
    if any(item and item.split('/')[0] != 'target' for item in ignored):
        raise CleanupError('unknown ignored user material retained')
    # A branch without an upstream has no verifiable pushed delivery.
    upstream = git(path, 'rev-parse', '--abbrev-ref', '@{upstream}')
    if git(path, 'rev-parse', upstream) != expected:
        raise CleanupError('unpushed or changed upstream work retained')
    if _process_mentions_path(path):
        raise CleanupError('worktree is in use')
    return {'worktree': str(path), 'branch': branch, 'head': expected}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo-root', type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument('--worktree', type=pathlib.Path, required=True)
    parser.add_argument('--branch', required=True)
    parser.add_argument('--expected-head', required=True)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    try:
        report = inspect(args.repo_root, args.worktree, args.branch, args.expected_head)
        if args.execute:
            # Recheck immediately before mutation; never force removal.
            inspect(args.repo_root, args.worktree, args.branch, args.expected_head)
            git(args.repo_root, 'worktree', 'remove', str(args.worktree.resolve()))
            if git(args.repo_root, 'rev-parse', 'refs/heads/' + args.branch) != args.expected_head:
                raise CleanupError('branch changed; retained')
            # Preserve the ordinary merged-branch condition, then delete with
            # an atomic expected-old OID comparison so concurrent work survives.
            git(args.repo_root, 'merge-base', '--is-ancestor', args.expected_head, 'HEAD')
            git(args.repo_root, 'update-ref', '-d', 'refs/heads/' + args.branch, args.expected_head)
            remaining = _run(['git', '-C', str(args.repo_root), 'show-ref', '--verify', '--quiet', 'refs/heads/' + args.branch], check=False)
            if remaining.returncode != 1:
                raise CleanupError('branch absence could not be verified; retained or recreated')
        print(json.dumps(dict(report, executed=args.execute)))
    except CleanupError as exc:
        parser.exit(1, 'resource cleanup: ' + str(exc) + '\n')

if __name__ == '__main__':
    main()
