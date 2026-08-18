#!/usr/bin/env python3
"""Rebuild the checksum closure after verified operator tools are installed."""

from __future__ import annotations

import hashlib
import os
import stat
import sys
from pathlib import Path
from typing import NoReturn


def die(message: str) -> NoReturn:
    print(f"error: deployed Linux bundle checksum rebuild: {message}", file=sys.stderr)
    raise SystemExit(1)


def regular_files(root: Path) -> list[Path]:
    if root.is_symlink() or not root.is_dir():
        die(f"bundle root must be a regular directory: {root}")
    files: list[Path] = []

    def fail_closed(error: OSError) -> NoReturn:
        location = error.filename or root
        detail = error.strerror or str(error)
        die(f"cannot read bundle subtree {location}: {detail}")

    for directory, directories, names in os.walk(
        root,
        followlinks=False,
        onerror=fail_closed,
    ):
        directory_path = Path(directory)
        for name in directories:
            path = directory_path / name
            if path.is_symlink() or not path.is_dir():
                die(f"bundle contains non-directory member: {path}")
        for name in names:
            path = directory_path / name
            if path.name == "SHA256SUMS" and path.relative_to(root).parent == Path("."):
                continue
            if path.is_symlink() or not path.is_file() or not stat.S_ISREG(path.lstat().st_mode):
                die(f"bundle contains non-regular member: {path}")
            files.append(path)
    return sorted(files, key=lambda path: path.relative_to(root).as_posix())


def rebuild(root: Path) -> None:
    manifest_path = root / "SHA256SUMS"
    if manifest_path.is_symlink() or not manifest_path.is_file():
        die(f"checksum manifest must already exist as a regular file: {manifest_path}")
    root_metadata = root.stat()
    files = regular_files(root)
    if not files:
        die("bundle has no regular files to checksum")
    lines = []
    for path in files:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        lines.append(f"{digest}  {path.relative_to(root).as_posix()}")
    payload = ("\n".join(lines) + "\n").encode("utf-8")
    metadata = manifest_path.stat()
    temporary = manifest_path.with_name(f".{manifest_path.name}.{os.getpid()}.tmp")
    try:
        with temporary.open("wb") as handle:
            handle.write(payload)
            os.fchmod(handle.fileno(), stat.S_IMODE(metadata.st_mode))
            try:
                os.fchown(handle.fileno(), metadata.st_uid, metadata.st_gid)
            except PermissionError:
                # Non-root test fixtures may not be allowed to chown back to a
                # package owner; mode and atomic replacement remain enforced.
                pass
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, manifest_path)
        directory_fd = os.open(root, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            # Replacing SHA256SUMS updates the bundle root mtime. Preserve the
            # extracted release directory timestamp so retention ordering stays
            # based on release promotion rather than checksum bookkeeping.
            os.utime(
                root,
                ns=(root_metadata.st_atime_ns, root_metadata.st_mtime_ns),
                follow_symlinks=False,
            )
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        if temporary.exists():
            temporary.unlink()


if __name__ == "__main__":
    if len(sys.argv) != 2:
        die("usage: p2p-rebuild-linux-bundle-checksums.py <bundle-root>")
    rebuild(Path(sys.argv[1]))
