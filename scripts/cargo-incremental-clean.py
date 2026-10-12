#!/usr/bin/env python3
"""Remove only incremental state from a managed target; dry-run by default."""
import argparse
import contextlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


cache = load("cargo_cache", "cargo-cache.py")
resources = load("cleanup_resources", "resource-cleanup-executor.py")


def clean(root, target, apply=False):
    if os.name != "posix":
        raise ValueError("incremental cleanup requires Unix locks and process inspection")
    target = Path(target).absolute()
    common, _, family = cache.layout(root)
    if not cache.valid_path(target, family) or not cache.read_metadata(target, common):
        raise ValueError("target must have a valid repository ownership marker")
    with cache.locked(family / ".attachments", family, exclusive=True, blocking=False), cache.locked(target, family, exclusive=True, blocking=False):
        if not cache.valid_path(target, family) or not cache.read_metadata(target, common):
            raise ValueError("target identity changed")
        usage = resources.inspect_process_use(target)
        if usage["state"] != "clear_observed":
            raise ValueError(usage["reason"])
        actions = []
        with contextlib.ExitStack() as stack:
            # Raw Cargo does not hold our wrapper lease. Respect its build locks too.
            import fcntl
            for directory in (target, target / "debug", target / "release", target / "packaging"):
                if directory.is_symlink():
                    raise ValueError("symlinked profile retained")
                if not directory.is_dir():
                    continue
                for name in (".cargo-lock", ".cargo-build-lock"):
                    path = directory / name
                    fd = os.open(path, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600)
                    stack.callback(os.close, fd)
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            for profile in ("debug", "release", "packaging"):
                path = target / profile / "incremental"
                if path.is_symlink():
                    raise ValueError("symlinked incremental retained")
                if path.is_dir():
                    if any(p.is_symlink() for p in path.rglob("*")):
                        raise ValueError("incremental containing symlinks retained")
                    actions.append({"path": str(path), "bytes": cache.physical_size(path), "action": "deleted" if apply else "would-delete"})
            # Validate all profiles before deleting any of them.
            if apply:
                for action in actions:
                    shutil.rmtree(action["path"])
        return {"applied": apply, "target": str(target), "actions": actions}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", default=os.getcwd())
    parser.add_argument("--target", required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    try:
        print(json.dumps(clean(args.repo_root, args.target, args.apply), indent=2))
        return 0
    except (OSError, ValueError) as exc:
        print("cargo-incremental-clean: " + str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
