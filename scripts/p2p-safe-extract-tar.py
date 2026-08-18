#!/usr/bin/env python3
"""Extract an operator tar archive only after a complete safety preflight."""

from __future__ import annotations

import stat
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import NoReturn


def die(message: str) -> "NoReturn":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def validate_member_name(name: str) -> PurePosixPath:
    if not name or "\x00" in name:
        die("tar archive contains an invalid member name")
    path = PurePosixPath(name)
    if path.is_absolute() or name.startswith("/"):
        die(f"tar archive contains absolute member: {name}")
    parts = path.parts
    if any(part in ("", ".", "..") for part in parts):
        die(f"tar archive contains unsafe member path: {name}")
    return path


def main() -> int:
    if len(sys.argv) != 3:
        die("usage: p2p-safe-extract-tar.py <archive.tar.gz> <destination>")
    archive_path = Path(sys.argv[1])
    destination = Path(sys.argv[2])
    if not archive_path.is_file():
        die(f"missing tar archive: {archive_path}")
    destination.mkdir(parents=True, exist_ok=True)
    destination = destination.resolve()

    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
        if not members:
            die("tar archive is empty")

        entries: dict[PurePosixPath, tarfile.TarInfo] = {}
        directories: set[PurePosixPath] = set()
        files: set[PurePosixPath] = set()
        top_levels: set[str] = set()
        for member in members:
            relative = validate_member_name(member.name)
            top_levels.add(relative.parts[0])
            if relative in entries:
                die(f"tar archive contains duplicate member: {member.name}")
            if member.isdir():
                directories.add(relative)
            elif member.isreg():
                files.add(relative)
            else:
                # This rejects symlinks, hardlinks, devices, FIFOs, and all
                # other special TarInfo types before any destination writes.
                die(f"tar archive contains non-regular member: {member.name}")
            entries[relative] = member

        for top_level in top_levels:
            existing = destination / top_level
            if existing.exists() or existing.is_symlink():
                die(f"tar extraction destination already contains top-level path: {top_level}")

        for relative, member in entries.items():
            parent = relative.parent
            while parent != PurePosixPath("."):
                if parent in files:
                    die(f"tar archive has file/directory collision: {member.name}")
                parent = parent.parent
            target = destination.joinpath(*relative.parts)
            try:
                target.relative_to(destination)
            except ValueError:
                die(f"tar archive member escapes extraction root: {member.name}")
            if target.exists() or target.is_symlink():
                die(f"tar extraction destination already contains member: {member.name}")

        # All member names, types, collisions, and destination paths have
        # passed validation. Only now create directories and regular files.
        for relative in sorted(directories, key=lambda value: (len(value.parts), value.parts)):
            target = destination.joinpath(*relative.parts)
            target.mkdir()
            target.chmod((stat.S_IMODE(entries[relative].mode) & 0o777) or 0o755)
        for relative in sorted(files, key=lambda value: value.parts):
            member = entries[relative]
            target = destination.joinpath(*relative.parts)
            target.parent.mkdir(parents=True, exist_ok=True)
            source = archive.extractfile(member)
            if source is None:
                die(f"tar archive member cannot be read: {member.name}")
            with source, target.open("xb") as output:
                while chunk := source.read(1024 * 1024):
                    output.write(chunk)
            target.chmod((stat.S_IMODE(member.mode) & 0o777) or 0o644)
    return 0


if __name__ == "__main__":
    main()
