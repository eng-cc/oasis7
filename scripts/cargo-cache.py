#!/usr/bin/env python3
"""Inventory and opt-in orphan GC for wrapper-created Cargo targets.

Unknown legacy caches are never enrolled or deleted. Unix leases are shared;
Windows leases serialize commands because msvcrt has no shared lock API.
Destructive GC is Unix-only; Windows supports inventory and dry-run.
"""
import argparse
import contextlib
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

VERSION = 1
MARKER = ".oasis7-cargo-cache.json"


def digest(path):
    return hashlib.sha256(str(Path(path).resolve()).encode()).hexdigest()[:12]


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def layout(root):
    common = Path(git(root, "rev-parse", "--git-common-dir"))
    if not common.is_absolute():
        common = Path(root) / common
    common = common.resolve()
    base = common.parent.parent / ".oasis7-cache" / "cargo-target"
    return common, base, base / ("git-" + digest(common))


def valid_path(target, family):
    target, family = Path(target).absolute(), Path(family).absolute()
    try:
        rel = target.relative_to(family)
    except ValueError:
        return False
    if len(rel.parts) != 2 or not re.fullmatch(r"worktree-[0-9a-f]{12}", rel.parts[0]) or not rel.parts[1].startswith("rustc-"):
        return False
    return not any(p.is_symlink() for p in [target, target.parent, family, family.parent, family.parent.parent])


@contextlib.contextmanager
def locked(target, family, exclusive=False, blocking=True):
    directory = family.parent.parent / "cargo-cache-locks" / family.name
    directory.mkdir(parents=True, exist_ok=True)
    if directory.is_symlink() or directory.parent.is_symlink():
        raise ValueError("symlinked lock directory")
    name = digest(target) + ".lock"
    lockpath = directory / name
    if lockpath.is_symlink():
        raise ValueError("symlinked lock file")
    fd = os.open(lockpath, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600)
    try:
        if os.name == "posix":
            import fcntl
            flags = fcntl.LOCK_EX if exclusive else fcntl.LOCK_SH
            if not blocking:
                flags |= fcntl.LOCK_NB
            fcntl.flock(fd, flags)
        elif os.name == "nt":
            import msvcrt
            if os.fstat(fd).st_size == 0:
                os.write(fd, b"0")
            os.lseek(fd, 0, os.SEEK_SET)
            msvcrt.locking(fd, msvcrt.LK_LOCK if blocking else msvcrt.LK_NBLCK, 1)
        else:
            raise ValueError("unsupported cache lock platform")
        yield fd
    finally:
        os.close(fd)


def read_metadata(target, common):
    marker = target / MARKER
    if marker.is_symlink():
        return None
    try:
        data = json.loads(marker.read_text())
        owner = Path(data["owner"]).resolve()
        if data["version"] != VERSION or data["common_dir"] != str(common) or data["target"] != str(target.resolve()) or target.parent.name != "worktree-" + digest(owner):
            return None
        used = float(data["last_used"])
        if not math.isfinite(used) or not 0 < used <= time.time() + 60:
            return None
        data["last_used"] = used
        return data
    except (OSError, ValueError, KeyError, TypeError):
        return None


def write_metadata(target, root, common):
    data = dict(version=VERSION, target=str(target.resolve()), owner=str(Path(root).resolve()), common_dir=str(common), last_used=time.time())
    tmp = target / (MARKER + "." + str(os.getpid()))
    tmp.write_text(json.dumps(data, sort_keys=True) + "\n")
    tmp.replace(target / MARKER)


def physical_size(path):
    # du counts allocated blocks and does not dereference interior symlinks.
    result = subprocess.run(["du", "-sk", str(path)], capture_output=True, text=True)
    if result.returncode:
        return None
    return int(result.stdout.split()[0]) * 1024


def inventory(root, measure=True):
    common, base, family = layout(root)
    roots = [Path(line[9:]).resolve() for line in git(root, "worktree", "list", "--porcelain", "-z").split("\0") if line.startswith("worktree ")]
    owners = {"worktree-" + digest(p): str(p) for p in roots}
    refs = []
    for source in roots:
        link = source / "target"
        if link.is_symlink():
            refs.append((str(source), link.resolve()))
    rows = []
    if family.is_dir() and not family.is_symlink():
        for legacy in sorted(family.glob("rustc-*")):
            rows.append(dict(target=str(legacy), bytes=physical_size(legacy) if measure and legacy.is_dir() and not legacy.is_symlink() else None, owner=None, registered=False, references=[], managed=False, last_used=None, status="unknown", safe=False))
        for namespace in sorted(family.iterdir()):
            if namespace.name.startswith("worktree-") and namespace.is_symlink():
                rows.append(dict(target=str(namespace), bytes=None, owner=None, registered=False, references=[], managed=False, last_used=None, status="unknown", safe=False))
                continue
            if not namespace.name.startswith("worktree-") or not namespace.is_dir():
                continue
            for target in sorted(namespace.iterdir()):
                if not target.name.startswith("rustc-"):
                    continue
                safe = valid_path(target, family) and target.is_dir()
                data = read_metadata(target, common) if safe else None
                referenced = [source for source, destination in refs if destination == target.resolve() or target.resolve() in destination.parents or destination in target.resolve().parents]
                owner = owners.get(namespace.name)
                rows.append(dict(target=str(target), bytes=physical_size(target) if safe and measure else None, owner=owner or (data or {}).get("owner"), registered=owner is not None, references=referenced, managed=data is not None, last_used=(data or {}).get("last_used"), status="registered" if owner else "referenced" if referenced else "orphan" if data else "unknown", safe=safe))
    return dict(common_dir=str(common), cache_root=str(base), family=str(family), targets=rows, total_bytes=sum(row["bytes"] or 0 for row in rows), unknown_size_count=sum(row["bytes"] is None for row in rows))


def eligible(row, age):
    return row["safe"] and row["bytes"] is not None and row["managed"] and not row["registered"] and not row["references"] and time.time() - row["last_used"] >= age * 86400


def collect(root, apply=False, min_age_days=7, budget_gib=None):
    if not math.isfinite(min_age_days) or min_age_days < 1 or (budget_gib is not None and (not math.isfinite(budget_gib) or budget_gib < 0)):
        raise ValueError("minimum age must be >= 1 day and budget nonnegative")
    if apply and os.name != "posix":
        raise ValueError("cache deletion requires Unix inherited leases; report and dry-run remain available")
    report = inventory(root)
    remaining = report["total_bytes"]
    actions = []
    for row in sorted(report["targets"], key=lambda item: item["last_used"] or 0):
        if not eligible(row, min_age_days):
            continue
        if budget_gib is not None and remaining <= budget_gib * 1024 ** 3:
            break
        target = Path(row["target"])
        try:
            with locked(target, Path(report["family"]), exclusive=True, blocking=False):
                # Recheck registration, references and marker under the same lock
                # used by wrapper commands; never delete nested symlinks.
                fresh = next((r for r in inventory(root, measure=False)["targets"] if r["target"] == str(target)), None)
                if fresh:
                    fresh["bytes"] = row["bytes"]
                if not fresh or not eligible(fresh, min_age_days):
                    actions.append(dict(target=str(target), action="preserved"))
                    continue
                if any(p.is_symlink() for p in target.rglob("*")):
                    actions.append(dict(target=str(target), action="preserved-symlink"))
                    continue
                if apply:
                    shutil.rmtree(target)
                remaining -= row["bytes"] or 0
                actions.append(dict(target=str(target), action="deleted" if apply else "would-delete"))
        except (OSError, ValueError):
            actions.append(dict(target=str(target), action="busy-or-unsafe"))
    report.update(actions=actions, remaining_bytes=remaining, size_accounting="snapshot", applied=apply, budget_met=None if budget_gib is None else report["unknown_size_count"] == 0 and remaining <= budget_gib * 1024 ** 3)
    return report


def lease(target, root, common, command):
    expected, _, family = layout(root)
    target = Path(target).absolute()
    if Path(common).resolve() != expected:
        raise ValueError("Git common directory mismatch")
    if not valid_path(target, family) or target.parent.name != "worktree-" + digest(root):
        return subprocess.call(command)
    with locked(target, family) as lock_fd:
        managed = read_metadata(target, expected)
        fresh = not target.exists() or (target.is_dir() and not any(target.iterdir()))
        target.mkdir(parents=True, exist_ok=True)
        if managed or fresh:
            write_metadata(target, root, expected)
        try:
            # Keep the lease open in the child even if the supervisor exits.
            child = subprocess.Popen(command, pass_fds=(lock_fd,) if os.name == "posix" else ())
            previous = {}
            def forward(signum, frame):
                child.send_signal(signum)
            for signum in (signal.SIGINT, signal.SIGTERM):
                previous[signum] = signal.signal(signum, forward)
            try:
                return child.wait()
            finally:
                for signum, handler in previous.items():
                    signal.signal(signum, handler)
        finally:
            if managed or fresh:
                write_metadata(target, root, expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="mode", required=True)
    for mode in ("report", "gc", "lease"):
        sub = subs.add_parser(mode)
        sub.add_argument("--repo-root", default=os.getcwd())
        if mode == "lease":
            sub.add_argument("--target", required=True)
            sub.add_argument("--common-dir", required=True)
            sub.add_argument("command", nargs=argparse.REMAINDER)
        else:
            sub.add_argument("--json", action="store_true")
            if mode == "gc":
                sub.add_argument("--apply", action="store_true")
                sub.add_argument("--min-age-days", type=float, default=7)
                sub.add_argument("--budget-gib", type=float)
    args = parser.parse_args()
    try:
        if args.mode == "lease":
            command = args.command[1:] if args.command[:1] == ["--"] else args.command
            if not command:
                parser.error("lease requires a command")
            code = lease(args.target, args.repo_root, args.common_dir, command)
            return code if code >= 0 else 128 - code
        result = inventory(args.repo_root) if args.mode == "report" else collect(args.repo_root, args.apply, args.min_age_days, args.budget_gib)
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print("cargo-cache: " + str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
