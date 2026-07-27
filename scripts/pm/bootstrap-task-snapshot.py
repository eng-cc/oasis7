#!/usr/bin/env python3
# Cross-platform maintenance: preserve Windows Git Bash/PowerShell and Linux/macOS Git discovery behavior.
"""Create and validate an immutable bootstrap task snapshot."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
from typing import Any


SCHEMA = "oasis7.bootstrap-task-snapshot/v1"


class SnapshotError(Exception):
    pass


def git_executable() -> str:
    executable = shutil.which("git")
    if executable is None and sys.platform == "win32":
        candidate = pathlib.Path("C:/Program Files/Git/cmd/git.exe")
        if candidate.is_file():
            executable = str(candidate)
    if executable is None:
        raise SnapshotError("git executable not found on PATH or in the standard Windows Git installation")
    return executable


def canonical_bytes(value: dict[str, Any]) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest(value: dict[str, Any]) -> str:
    unsigned = dict(value)
    unsigned.pop("digest", None)
    return "sha256:" + hashlib.sha256(canonical_bytes(unsigned)).hexdigest()


def git(repo_root: pathlib.Path, *args: str) -> str:
    executable = git_executable()
    result = subprocess.run(
        [executable, "-C", str(repo_root), *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode:
        raise SnapshotError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def load_task(tasks_json: pathlib.Path, task_uid: str) -> tuple[dict[str, Any], dict[str, Any]]:
    try:
        mapping = json.loads(tasks_json.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SnapshotError(f"cannot read tasks mapping {tasks_json}: {exc}") from exc
    task = mapping.get("tasks", {}).get(task_uid)
    if not isinstance(task, dict):
        raise SnapshotError(f"task UID not found in tasks mapping: {task_uid}")
    return mapping, task


def resolve_base(repo_root: pathlib.Path, default_branch: str) -> tuple[str, str]:
    candidates = (f"refs/remotes/origin/{default_branch}", f"refs/heads/{default_branch}")
    executable = git_executable()
    for ref in candidates:
        result = subprocess.run(
            [executable, "-C", str(repo_root), "rev-parse", "--verify", f"{ref}^{{commit}}"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        if result.returncode == 0:
            return ref, result.stdout.strip()
    raise SnapshotError(f"cannot resolve default branch {default_branch!r}")


def live_payload(
    repo_root: pathlib.Path,
    tasks_json: pathlib.Path,
    task_uid: str,
    request_identity: str,
) -> dict[str, Any]:
    mapping, task = load_task(tasks_json, task_uid)
    required = (
        "issue_number", "issue_url", "project_item_id", "status", "owner_role",
        "repository", "canonical_worktree", "task_branch", "default_branch", "acceptance",
    )
    missing = [name for name in required if task.get(name) in (None, "")]
    project = mapping.get("project")
    if missing or not isinstance(project, dict) or not project.get("number") or not project.get("owner"):
        details = ", ".join(missing) if missing else "project identity"
        raise SnapshotError(f"tasks mapping is missing required bootstrap truth: {details}")
    if not isinstance(task["acceptance"], list) or not task["acceptance"]:
        raise SnapshotError("tasks mapping acceptance must be a non-empty list")
    bootstrap_epoch = task.get("bootstrap_epoch", 1)
    if type(bootstrap_epoch) is not int or bootstrap_epoch < 1:
        raise SnapshotError("tasks mapping bootstrap_epoch must be a positive integer")

    root = pathlib.Path(git(repo_root, "rev-parse", "--show-toplevel")).resolve()
    canonical_worktree = pathlib.Path(task["canonical_worktree"]).resolve()
    if root != canonical_worktree:
        raise SnapshotError(f"worktree drift: mapping={canonical_worktree} live={root}")
    branch = git(root, "symbolic-ref", "--quiet", "--short", "HEAD")
    if branch != task["task_branch"]:
        raise SnapshotError(f"branch drift: mapping={task['task_branch']} live={branch}")
    head = git(root, "rev-parse", "HEAD")
    base_ref, base_oid = resolve_base(root, task["default_branch"])

    return {
        "schema": SCHEMA,
        "task": {
            "uid": task_uid,
            "issue": {"number": task["issue_number"], "url": task["issue_url"]},
            "project": {
                "owner": project["owner"],
                "number": project["number"],
                "item_id": task["project_item_id"],
                "status": task["status"],
            },
            "owner_role": task["owner_role"],
            "acceptance": task["acceptance"],
            "bootstrap_epoch": bootstrap_epoch,
        },
        "repository": task["repository"],
        "git": {
            "worktree": str(root),
            "branch": branch,
            "base": {"branch": task["default_branch"], "ref": base_ref, "oid": base_oid},
            "head": head,
        },
        "request": {"identity": request_identity, "acceptance": task["acceptance"]},
    }


def default_path(repo_root: pathlib.Path, task_uid: str) -> pathlib.Path:
    return repo_root / ".pm" / "scratch" / task_uid / "bootstrap-task-snapshot.json"


def derived_request_identity(repo_root: pathlib.Path, tasks_json: pathlib.Path, task_uid: str) -> str:
    """Return the deterministic request identity for a newly bound task."""
    _, task = load_task(tasks_json, task_uid)
    title = task.get("title")
    if not isinstance(title, str) or not title.strip():
        raise SnapshotError("tasks mapping is missing required bootstrap request identity: title")
    return title.strip()


def request_identity_for_validate_or_create(
    snapshot_path: pathlib.Path,
    repo_root: pathlib.Path,
    tasks_json: pathlib.Path,
    task_uid: str,
    supplied: str | None,
) -> str:
    if supplied:
        return supplied
    if snapshot_path.exists():
        try:
            saved = json.loads(snapshot_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise SnapshotError(f"cannot read snapshot {snapshot_path}: {exc}") from exc
        identity = saved.get("request", {}).get("identity") if isinstance(saved, dict) else None
        if not isinstance(identity, str) or not identity:
            raise SnapshotError("existing snapshot request identity is missing; pass --request-identity only after repairing task truth")
        return identity
    return derived_request_identity(repo_root, tasks_json, task_uid)


def create(args: argparse.Namespace) -> pathlib.Path:
    root = pathlib.Path(args.repo_root).resolve()
    tasks_json = pathlib.Path(args.tasks_json).resolve() if args.tasks_json else root / ".pm/github-project-sync/tasks.json"
    output = pathlib.Path(args.snapshot).resolve() if args.snapshot else default_path(root, args.task_uid)
    payload = live_payload(root, tasks_json, args.task_uid, args.request_identity)
    payload["producer"] = args.producer
    payload["created_at"] = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    payload["digest"] = digest(payload)
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with output.open("x", encoding="utf-8") as handle:
            json.dump(payload, handle, ensure_ascii=False, sort_keys=True, indent=2)
            handle.write("\n")
    except FileExistsError as exc:
        raise SnapshotError(f"snapshot already exists; refusing overwrite: {output}") from exc
    return output


def validate(args: argparse.Namespace) -> pathlib.Path:
    root = pathlib.Path(args.repo_root).resolve()
    tasks_json = pathlib.Path(args.tasks_json).resolve() if args.tasks_json else root / ".pm/github-project-sync/tasks.json"
    snapshot_path = pathlib.Path(args.snapshot).resolve() if args.snapshot else default_path(root, args.task_uid)
    try:
        saved = json.loads(snapshot_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SnapshotError(f"cannot read snapshot {snapshot_path}: {exc}") from exc
    if not isinstance(saved, dict):
        raise SnapshotError("snapshot root must be an object")
    if saved.get("digest") != digest(saved):
        raise SnapshotError("snapshot digest mismatch")
    expected = live_payload(root, tasks_json, args.task_uid, args.request_identity)
    for field in ("schema", "task", "repository", "git", "request"):
        if saved.get(field) != expected[field]:
            raise SnapshotError(
                f"snapshot {field} drift: expected={expected[field]!r} actual={saved.get(field)!r}; "
                "remediation: refresh bound task truth or create a new task bootstrap epoch"
            )
    if not isinstance(saved.get("producer"), str) or not saved["producer"]:
        raise SnapshotError("snapshot producer is missing")
    if not isinstance(saved.get("created_at"), str) or not saved["created_at"]:
        raise SnapshotError("snapshot creation time is missing")
    return snapshot_path


def validate_or_create(args: argparse.Namespace) -> tuple[pathlib.Path, str]:
    root = pathlib.Path(args.repo_root).resolve()
    tasks_json = pathlib.Path(args.tasks_json).resolve() if args.tasks_json else root / ".pm/github-project-sync/tasks.json"
    snapshot_path = pathlib.Path(args.snapshot).resolve() if args.snapshot else default_path(root, args.task_uid)
    request_identity = request_identity_for_validate_or_create(
        snapshot_path, root, tasks_json, args.task_uid, args.request_identity,
    )
    args.request_identity = request_identity
    if snapshot_path.exists():
        return validate(args), "reused"
    try:
        return create(args), "created"
    except SnapshotError as exc:
        # A concurrent bootstrap may have created the same immutable snapshot.
        # Validate that exact winner; never overwrite it.
        if "snapshot already exists; refusing overwrite" not in str(exc):
            raise
        return validate(args), "reused"


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    subparsers = result.add_subparsers(dest="command", required=True)
    for command in ("create", "validate", "validate-or-create"):
        sub = subparsers.add_parser(command)
        sub.add_argument("--repo-root", required=True)
        sub.add_argument("--task-uid", required=True)
        sub.add_argument("--request-identity", required=command != "validate-or-create")
        sub.add_argument("--tasks-json")
        sub.add_argument("--snapshot", help="snapshot path; defaults under .pm/scratch/<task-uid>")
        if command in {"create", "validate-or-create"}:
            sub.add_argument("--producer", required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "create":
            path, status = create(args), "created"
        elif args.command == "validate":
            path, status = validate(args), "valid"
        else:
            path, status = validate_or_create(args)
    except SnapshotError as exc:
        print(f"bootstrap-task-snapshot: {exc}", file=sys.stderr)
        return 1
    print(json.dumps({"status": status, "snapshot": str(path)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
