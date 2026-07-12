#!/usr/bin/env python3
"""Emit a deterministic lstat/content manifest hash for one directory tree."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
from pathlib import Path


def record(root: Path, path: Path) -> dict[str, object]:
    metadata = os.lstat(path)
    relative = "." if path == root else str(path.relative_to(root))
    payload: dict[str, object] = {
        "path": relative,
        "mode": stat.S_IMODE(metadata.st_mode),
    }
    if stat.S_ISDIR(metadata.st_mode):
        payload["kind"] = "directory"
    elif stat.S_ISREG(metadata.st_mode):
        payload["kind"] = "regular"
        payload["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    elif stat.S_ISLNK(metadata.st_mode):
        payload["kind"] = "symlink"
        payload["target"] = os.readlink(path)
    else:
        payload["kind"] = "other"
    return payload


def manifest(root: Path) -> list[dict[str, object]]:
    paths = [root]
    for directory, names, files in os.walk(root, followlinks=False):
        base = Path(directory)
        paths.extend(base / name for name in sorted(names))
        paths.extend(base / name for name in sorted(files))
    unique = sorted(set(paths), key=lambda path: str(path.relative_to(root)) if path != root else "")
    return [record(root, path) for path in unique]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--reject-symlinks", action="store_true")
    args = parser.parse_args()
    root = Path(os.path.abspath(os.fspath(args.root)))
    root_metadata = os.lstat(root)
    if args.reject_symlinks and stat.S_ISLNK(root_metadata.st_mode):
        raise SystemExit("tree-manifest: root path is a forbidden symlink: .")
    records = manifest(root)
    symlinks = [str(item["path"]) for item in records if item["kind"] == "symlink"]
    if args.reject_symlinks and symlinks:
        raise SystemExit(
            "tree-manifest: symlinks are forbidden in governed .pm: "
            + ", ".join(symlinks)
        )
    encoded = json.dumps(records, sort_keys=True, separators=(",", ":")).encode("utf-8")
    digest = hashlib.sha256(encoded).hexdigest()
    if args.json:
        print(json.dumps({"sha256": digest, "records": records}, ensure_ascii=False))
    else:
        print(digest)


if __name__ == "__main__":
    main()
