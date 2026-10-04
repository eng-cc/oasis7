#!/usr/bin/env python3
"""Bind a newly created task to one exact Git worktree admin instance."""
from __future__ import annotations

import importlib.util
import argparse
import json
import os
import pathlib
import re
import subprocess
import uuid
from typing import Any


_STORE_PATH = pathlib.Path(__file__).with_name("workflow-durable-store.py")
_STORE_SPEC = importlib.util.spec_from_file_location("workflow_durable_store_registration", _STORE_PATH)
if _STORE_SPEC is None or _STORE_SPEC.loader is None:
    raise RuntimeError(f"cannot load durable task mapping validator at {_STORE_PATH}")
DURABLE_STORE = importlib.util.module_from_spec(_STORE_SPEC)
_STORE_SPEC.loader.exec_module(DURABLE_STORE)

TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
REPOSITORY_RE = re.compile(r"^[^/\s]+/[^/\s]+$")
REGISTRATION_KEYS = frozenset({"common_dir", "admin_dir", "instance_id"})
MARKER_NAME = "oasis7-instance-id"


class RegistrationError(ValueError):
    pass


def _git(root: pathlib.Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args], text=True, capture_output=True, check=False,
    )
    if result.returncode:
        raise RegistrationError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def _resolved_git_path(root: pathlib.Path, *args: str) -> pathlib.Path:
    value = pathlib.Path(_git(root, *args))
    if not value.is_absolute():
        value = root / value
    try:
        return value.resolve(strict=True)
    except OSError as exc:
        raise RegistrationError(f"Git admin path is unavailable: {value}") from exc


def live_worktree_identity(repo_root: pathlib.Path) -> dict[str, str]:
    """Read this checkout's registered path, branch, common dir and admin dir."""
    requested = pathlib.Path(repo_root).expanduser().resolve(strict=True)
    root = pathlib.Path(_git(requested, "rev-parse", "--show-toplevel")).resolve(strict=True)
    if root != requested:
        raise RegistrationError(f"selected worktree root mismatch: {requested} != {root}")
    branch = _git(root, "symbolic-ref", "--quiet", "--short", "HEAD")
    if not branch:
        raise RegistrationError("selected worktree is detached")
    common_dir = _resolved_git_path(root, "rev-parse", "--git-common-dir")
    admin_dir = _resolved_git_path(root, "rev-parse", "--git-dir")
    try:
        common_dir.relative_to(admin_dir) if common_dir == admin_dir else admin_dir.relative_to(common_dir)
    except ValueError as exc:
        raise RegistrationError("Git worktree admin directory is outside its common directory") from exc
    registered_path = None
    registered_branch = None
    current_path = None
    def finish_entry() -> None:
        nonlocal registered_path, current_path, registered_branch
        if current_path == root and registered_branch == branch:
            registered_path = current_path
        current_path = None
        registered_branch = None

    for line in _git(root, "worktree", "list", "--porcelain").splitlines():
        if line.startswith("worktree "):
            if current_path is not None:
                finish_entry()
            current_path = pathlib.Path(line.removeprefix("worktree ")).resolve()
            registered_branch = None
        elif line.startswith("branch refs/heads/") and current_path is not None:
            registered_branch = line.removeprefix("branch refs/heads/")
        elif not line:
            finish_entry()
    if current_path is not None:
        finish_entry()
    if registered_path != root:
        raise RegistrationError("selected worktree is not registered at its exact live path and branch")
    return {
        "canonical_worktree": str(root),
        "common_dir": str(common_dir),
        "admin_dir": str(admin_dir),
        "task_branch": branch,
    }


def validate_worktree_registration(
    repo_root: pathlib.Path,
    task: dict[str, Any],
) -> dict[str, str] | None:
    """Validate a stored registration against live Git facts and its admin marker.

    Legacy tasks with no registration return ``None``. A present but malformed,
    stale, or missing marker fails closed so a reused admin path cannot inherit
    the earlier worktree's identity.
    """
    raw = task.get("worktree_registration")
    if raw is None:
        return None
    if not isinstance(raw, dict) or set(raw) != REGISTRATION_KEYS:
        raise RegistrationError("worktree registration has an unknown or incomplete shape")
    live = live_worktree_identity(repo_root)
    for key in ("common_dir", "admin_dir"):
        value = raw.get(key)
        if not isinstance(value, str) or not pathlib.Path(value).is_absolute():
            raise RegistrationError(f"worktree registration {key} must be an absolute path")
        try:
            if pathlib.Path(value).resolve(strict=True) != pathlib.Path(live[key]):
                raise RegistrationError(f"worktree registration {key} differs from live Git identity")
        except OSError as exc:
            raise RegistrationError(f"worktree registration {key} is unavailable") from exc
    instance_id = raw.get("instance_id")
    try:
        parsed = uuid.UUID(str(instance_id))
    except (ValueError, TypeError, AttributeError) as exc:
        raise RegistrationError("worktree registration instance_id is not a UUID") from exc
    if parsed.version != 4 or str(parsed) != instance_id:
        raise RegistrationError("worktree registration instance_id is not a canonical helper UUID")
    marker = pathlib.Path(live["admin_dir"]) / MARKER_NAME
    try:
        if marker.is_symlink() or not marker.is_file():
            raise RegistrationError("worktree instance marker is missing or is not a regular file")
        if marker.read_text(encoding="ascii") != instance_id + "\n":
            raise RegistrationError("worktree instance marker does not match the mapped UUID")
    except OSError as exc:
        raise RegistrationError(f"cannot read worktree instance marker: {exc}") from exc
    return {key: str(raw[key]) for key in ("common_dir", "admin_dir", "instance_id")}


def _write_instance_marker(admin_dir: pathlib.Path, instance_id: str) -> None:
    marker = admin_dir / MARKER_NAME
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    flags |= getattr(os, "O_NOFOLLOW", 0)
    try:
        fd = os.open(marker, flags, 0o600)
    except FileExistsError as exc:
        raise RegistrationError("worktree instance marker already exists without a mapping") from exc
    try:
        with os.fdopen(fd, "w", encoding="ascii", newline="\n") as stream:
            stream.write(instance_id + "\n")
            stream.flush()
            os.fsync(stream.fileno())
        if os.name != "nt":
            directory_fd = os.open(admin_dir, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
    except BaseException:
        marker.unlink(missing_ok=True)
        raise


def bind_new_task_worktree(repo_root: pathlib.Path, task_uid: str) -> dict[str, str]:
    """Mint an instance UUID only for the explicit fresh-task creation path."""
    if not TASK_UID_RE.fullmatch(task_uid):
        raise RegistrationError("invalid task UID")
    root = pathlib.Path(repo_root).expanduser().resolve(strict=True)
    live = live_worktree_identity(root)
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    result: dict[str, str] = {}

    def update(mapping: dict[str, Any]) -> None:
        nonlocal result
        try:
            if DURABLE_STORE.retired_task(mapping, task_uid) is not None:
                raise RegistrationError("cannot bind a retired task UID")
        except (AttributeError, TypeError):
            pass
        task = (mapping.get("tasks") or {}).get(task_uid)
        if not isinstance(task, dict):
            raise RegistrationError("fresh task is absent from the durable task mapping")
        if (task.get("canonical_worktree") != live["canonical_worktree"]
                or task.get("task_branch") != live["task_branch"]
                or not REPOSITORY_RE.fullmatch(str(task.get("repository") or ""))):
            raise RegistrationError("task mapping does not match the selected live worktree")
        existing = task.get("worktree_registration")
        if existing is not None:
            validated = validate_worktree_registration(root, task)
            if validated is None:
                raise RegistrationError("stored worktree registration is absent")
            result = validated
            return
        instance_id = str(uuid.uuid4())
        _write_instance_marker(pathlib.Path(live["admin_dir"]), instance_id)
        result = {
            "common_dir": live["common_dir"],
            "admin_dir": live["admin_dir"],
            "instance_id": instance_id,
        }
        task["worktree_registration"] = result

    DURABLE_STORE.transact_json(mapping_path, update, {"version": 1, "tasks": {}})
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--task-uid", required=True)
    args = parser.parse_args(argv)
    try:
        registration = bind_new_task_worktree(pathlib.Path(args.repo_root), args.task_uid)
    except (OSError, json.JSONDecodeError, RegistrationError) as exc:
        parser.error(str(exc))
    print(json.dumps(registration, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
