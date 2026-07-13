#!/usr/bin/env python3
"""Snapshot and compare a live file set without ever overwriting it."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import subprocess
from pathlib import Path


def index_records(root: Path, pathspec: str) -> list[dict[str, str]]:
    output = subprocess.check_output(
        ["git", "ls-files", "--stage", "-z", "--", pathspec], cwd=root
    )
    records = []
    for raw in output.split(b"\0"):
        if not raw:
            continue
        metadata, raw_path = raw.split(b"\t", 1)
        mode, oid, stage = metadata.decode("ascii").split()
        records.append(
            {
                "mode": mode,
                "oid": oid,
                "stage": stage,
                "path": raw_path.decode("utf-8"),
            }
        )
    return sorted(records, key=lambda record: (record["path"], record["stage"]))


def tracked_paths(root: Path, pathspec: str) -> list[str]:
    return sorted({record["path"] for record in index_records(root, pathspec)})


def untracked_paths(root: Path, pathspec: str) -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-files", "-z", "--others", "--exclude-standard", "--", pathspec],
        cwd=root,
    )
    return sorted(path.decode("utf-8") for path in output.split(b"\0") if path)


def worktree_record(root: Path, relative: str) -> dict[str, object]:
    path = root / relative
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return {"path": relative, "kind": "missing"}
    mode = stat.S_IMODE(metadata.st_mode)
    if stat.S_ISDIR(metadata.st_mode):
        return {"path": relative, "kind": "directory", "mode": mode}
    if stat.S_ISREG(metadata.st_mode):
        return {
            "path": relative,
            "kind": "regular",
            "mode": mode,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
    if stat.S_ISLNK(metadata.st_mode):
        return {
            "path": relative,
            "kind": "symlink",
            "mode": mode,
            "target": os.readlink(path),
        }
    return {"path": relative, "kind": "other", "mode": mode}


def filesystem_records(root: Path, pathspec: str) -> list[dict[str, object]]:
    start = root / pathspec
    paths = [start]
    if start.is_dir() and not start.is_symlink():
        for directory, names, files in os.walk(start, followlinks=False):
            base = Path(directory)
            paths.extend(base / name for name in sorted(names))
            paths.extend(base / name for name in sorted(files))
    unique = sorted(
        set(paths),
        key=lambda path: str(path.relative_to(root)),
    )
    return [worktree_record(root, str(path.relative_to(root))) for path in unique]


def snapshot(root: Path, state: Path, pathspec: str) -> None:
    state.mkdir(parents=True, exist_ok=True)
    initial_index_records = index_records(root, pathspec)
    paths = sorted({record["path"] for record in initial_index_records})
    worktree_records = [worktree_record(root, relative) for relative in paths]
    complete_filesystem_records = filesystem_records(root, pathspec)
    (state / "manifest.json").write_text(
        json.dumps(
            {
                "pathspec": pathspec,
                "tracked_paths": paths,
                "index_records": initial_index_records,
                "worktree_records": worktree_records,
                "filesystem_records": complete_filesystem_records,
                "initial_untracked_paths": untracked_paths(root, pathspec),
            }
        ),
        encoding="utf-8",
    )


def load_manifest(state: Path) -> dict[str, object]:
    return json.loads((state / "manifest.json").read_text(encoding="utf-8"))


def changed_paths(root: Path, state: Path) -> list[str]:
    manifest = load_manifest(state)
    initial = {record["path"]: record for record in manifest["worktree_records"]}
    return sorted(
        relative
        for relative, saved in initial.items()
        if worktree_record(root, relative) != saved
    )


def index_changes(root: Path, state: Path) -> tuple[list[str], list[str], bool]:
    manifest = load_manifest(state)
    initial = set(manifest["tracked_paths"])
    current_records = index_records(root, manifest["pathspec"])
    current = {record["path"] for record in current_records}
    return (
        sorted(current - initial),
        sorted(initial - current),
        current_records != manifest["index_records"],
    )


def new_untracked_paths(root: Path, state: Path) -> list[str]:
    manifest = load_manifest(state)
    initial = set(manifest["initial_untracked_paths"])
    return sorted(set(untracked_paths(root, manifest["pathspec"])) - initial)


def filesystem_changes(root: Path, state: Path) -> tuple[list[str], list[str], list[str]]:
    manifest = load_manifest(state)
    initial = {record["path"]: record for record in manifest["filesystem_records"]}
    current = {
        record["path"]: record
        for record in filesystem_records(root, manifest["pathspec"])
    }
    return (
        sorted(set(current) - set(initial)),
        sorted(set(initial) - set(current)),
        sorted(path for path in set(initial) & set(current) if initial[path] != current[path]),
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("snapshot", "check"))
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--state", required=True, type=Path)
    parser.add_argument("--pathspec", default=".pm/roles")
    args = parser.parse_args()
    root = args.root.resolve()
    state = args.state.resolve()
    if args.command == "snapshot":
        snapshot(root, state, args.pathspec)
        return
    changed = changed_paths(root, state)
    new_index, removed_index, index_drift = index_changes(root, state)
    new_untracked = new_untracked_paths(root, state)
    new_filesystem, removed_filesystem, changed_filesystem = filesystem_changes(
        root, state
    )
    failures = []
    if changed:
        failures.append("tracked projection drift: " + ", ".join(changed))
    if new_index:
        failures.append("new index projection path: " + ", ".join(new_index))
    if removed_index:
        failures.append("removed index projection path: " + ", ".join(removed_index))
    if index_drift:
        failures.append("index projection mode/oid/path drift")
    if new_untracked:
        failures.append("new untracked projection artifact: " + ", ".join(new_untracked))
    if new_filesystem:
        failures.append("new filesystem projection path: " + ", ".join(new_filesystem))
    if removed_filesystem:
        failures.append(
            "removed filesystem projection path: " + ", ".join(removed_filesystem)
        )
    if changed_filesystem:
        failures.append(
            "filesystem projection lstat/content drift: "
            + ", ".join(changed_filesystem)
        )
    if failures:
        raise SystemExit("guard-tracked-files: " + "; ".join(failures))
    print("guard-tracked-files: unchanged")


if __name__ == "__main__":
    main()
