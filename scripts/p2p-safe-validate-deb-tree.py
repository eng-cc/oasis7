#!/usr/bin/env python3
"""Validate an extracted Debian tree before package contents are consumed.

The package is extracted into a fresh temporary directory by dpkg-deb.  This
validator is deliberately a separate, read-only pass: no caller may hash,
copy, or execute a member until every entry in the consumed /opt/oasis7
subtree is a regular file/directory with a confined physical path.  Debian
packages may contain native launcher symlinks outside that boundary, but the
deployed player bundle must be a regular, symlink-free subtree.
"""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path
from typing import NoReturn


def die(message: str) -> "NoReturn":
    print(f"error: unsafe Debian extraction: {message}", file=sys.stderr)
    raise SystemExit(1)


def validate_tree(root: Path, required_relative: str) -> None:
    if root.is_symlink() or not root.is_dir():
        die(f"extraction root must be a regular directory: {root}")
    root = root.resolve()
    # Native Debian packages may legitimately expose launcher symlinks under
    # /usr/bin.  The deployment code consumes only the player subtree, so
    # validate that subtree and every ancestor leading to it; do not reject
    # harmless package-owned entries outside the consumption boundary.
    required = root
    for component in Path(required_relative).parts:
        required /= component
        try:
            mode = required.lstat().st_mode
        except OSError as error:
            die(f"cannot inspect required package path {required}: {error}")
        if stat.S_ISLNK(mode) or not stat.S_ISDIR(mode):
            die(f"required package path must be a regular directory: {required}")

    # Walk with lstat/scandir so symlink directories are never followed.  The
    # physical containment check also catches a path that escaped through an
    # ancestor replaced during extraction.
    for current, directories, files in os.walk(required, followlinks=False):
        current_path = Path(current)
        try:
            current_path.resolve().relative_to(root)
        except ValueError:
            die(f"required package subtree escapes extraction root: {current_path}")
        for name in directories + files:
            path = current_path / name
            if path.is_symlink():
                die(f"required package subtree contains symlink: {path}")
            try:
                mode = path.lstat().st_mode
            except OSError as error:
                die(f"cannot inspect required package member {path}: {error}")
            if not (stat.S_ISDIR(mode) or stat.S_ISREG(mode)):
                die(f"required package subtree contains non-regular member: {path}")
            try:
                path.resolve().relative_to(root)
            except ValueError:
                die(f"required package member escapes extraction root: {path}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        die("usage: p2p-safe-validate-deb-tree.py <extraction-root> <required-relative-subtree>")
    validate_tree(Path(sys.argv[1]), sys.argv[2])
