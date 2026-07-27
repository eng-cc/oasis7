#!/usr/bin/env python3
# Cross-platform maintenance: support POSIX Python and native Windows Python under Git Bash.
"""Fingerprint one Git verification epoch without mutating repository state."""

from __future__ import annotations

import hashlib
import json
import os
import stat
import subprocess
import sys
from pathlib import Path


def git(root: Path, *args: str) -> bytes:
    bash_executable = os.environ.get("OASIS7_GIT_BASH_EXECUTABLE")
    git_command = os.environ.get("OASIS7_GIT_COMMAND")
    command = ["git", "-C", str(root), *args]
    if bash_executable and git_command:
        command = [bash_executable, git_command, "-C", str(root), *args]
    return subprocess.check_output(
        command, stderr=subprocess.DEVNULL
    )


def path_record(root: Path, relative: str) -> dict[str, object]:
    path = root / relative
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return {"path": relative, "kind": "missing"}
    result: dict[str, object] = {
        "path": relative,
        "mode": stat.S_IMODE(metadata.st_mode),
    }
    if stat.S_ISREG(metadata.st_mode):
        result.update(kind="regular", sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    elif stat.S_ISLNK(metadata.st_mode):
        result.update(kind="symlink", target=os.readlink(path))
    elif stat.S_ISDIR(metadata.st_mode):
        result["kind"] = "directory"
    else:
        result["kind"] = "other"
    return result


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    try:
        head = git(root, "rev-parse", "HEAD").decode().strip()
        index = git(root, "ls-files", "--stage", "-z")
        paths_raw = git(root, "ls-files", "-co", "--exclude-standard", "-z")
        paths = sorted({item.decode("utf-8", "surrogateescape") for item in paths_raw.split(b"\0") if item})
    except subprocess.CalledProcessError:
        # Isolated helper fixtures may intentionally omit Git. Keep their epoch
        # guard useful without claiming a HEAD/index boundary.
        head = "non-git-fixture"
        index = b""
        paths = sorted(
            str(path.relative_to(root))
            for path in root.rglob("*")
            if ".git" not in path.relative_to(root).parts
        )
    payload = {
        "head": head,
        "index_sha256": hashlib.sha256(index).hexdigest(),
        "paths": [path_record(root, path) for path in paths],
    }
    encoded = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    print(json.dumps({"sha256": hashlib.sha256(encoded).hexdigest(), **payload}, ensure_ascii=False))


if __name__ == "__main__":
    main()
