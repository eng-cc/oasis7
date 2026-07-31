#!/usr/bin/env python3
# Cross-platform maintenance: keep this helper compatible with POSIX and native Windows Python.
"""Recover one terminal task record from its registered canonical worktree."""
from __future__ import annotations

import argparse
import copy
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
from typing import Any


REQUIRED_TASK_FIELDS = (
    "task_uid", "status", "repository", "default_branch",
    "canonical_worktree", "task_branch", "pr_number", "pr_url",
)
TASK_IDENTITY_FIELDS = (
    "task_uid", "repository", "default_branch", "canonical_worktree",
    "task_branch", "pr_number", "pr_url",
)
REQUIRED_RECEIPT_FIELDS = (
    "receipt_type", "issuer", "evidence_mode", "repository",
    "default_branch", "pr_number", "pr_url", "state", "merged_at",
    "head_oid", "base_ref", "observed_at",
)


def fail(message: str) -> None:
    raise SystemExit(f"recover-terminal-task-mapping: {message}")


def run_git(root: pathlib.Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args], text=True, encoding="utf-8",
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    if result.returncode:
        fail(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def normalized(path: str | pathlib.Path) -> str:
    return os.path.normcase(str(pathlib.Path(path).resolve(strict=False)))


def registered_worktrees(root: pathlib.Path) -> dict[str, dict[str, str]]:
    entries: dict[str, dict[str, str]] = {}
    current: dict[str, str] = {}
    for line in run_git(root, "worktree", "list", "--porcelain").splitlines() + [""]:
        if line.startswith("worktree "):
            if current:
                entries[normalized(current["path"])] = current
            current = {"path": line.removeprefix("worktree "), "branch": ""}
        elif line.startswith("branch refs/heads/") and current:
            current["branch"] = line.removeprefix("branch refs/heads/")
        elif not line and current:
            entries[normalized(current["path"])] = current
            current = {}
    if normalized(root) not in entries:
        fail("default worktree is not registered under its repository")
    return entries


def read_mapping(path: pathlib.Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read valid task mapping {path}: {exc}")
    if not isinstance(value, dict) or not isinstance(value.get("tasks"), dict):
        fail(f"task mapping has invalid structure: {path}")
    return value


def validate_record(
    record: Any, *, task_uid: str, main_ref: str,
    receipt: dict[str, Any], registry: dict[str, dict[str, str]],
) -> dict[str, Any]:
    if not isinstance(record, dict):
        fail("terminal task record is not an object")
    for field in REQUIRED_TASK_FIELDS:
        if record.get(field) in (None, ""):
            fail(f"terminal task record is missing {field}")
    if record.get("task_uid") != task_uid:
        fail("terminal task record task_uid mismatch")
    if record.get("status") != "done":
        fail("terminal task record status must be done")
    canonical_key = normalized(str(record["canonical_worktree"]))
    registered = registry.get(canonical_key)
    if not registered:
        fail("canonical task worktree is not registered under the default repository")
    if not registered.get("branch") or registered["branch"] != record.get("task_branch"):
        fail("canonical task worktree branch identity mismatch")
    if record.get("default_branch") != main_ref:
        fail("terminal task default branch identity mismatch")
    for field in REQUIRED_RECEIPT_FIELDS:
        if receipt.get(field) in (None, ""):
            fail(f"merge receipt is missing {field}")
    expected_receipt = {
        "receipt_type": "oasis7_pr_merge",
        "issuer": "github_live_query",
        "evidence_mode": "production",
        "state": "MERGED",
        "repository": record["repository"],
        "default_branch": record["default_branch"],
        "pr_number": record["pr_number"],
        "pr_url": record["pr_url"],
        "base_ref": main_ref,
    }
    for field, expected in expected_receipt.items():
        if str(receipt.get(field)) != str(expected):
            fail(f"merge receipt {field} disagrees with terminal task identity")
    return record


def task_identity(record: dict[str, Any]) -> tuple[str, ...]:
    values = []
    for field in TASK_IDENTITY_FIELDS:
        value = normalized(str(record[field])) if field == "canonical_worktree" else str(record[field])
        values.append(value)
    return tuple(values)


def load_store(script_dir: pathlib.Path):
    path = script_dir / "workflow-durable-store.py"
    spec = importlib.util.spec_from_file_location("workflow_durable_store", path)
    if spec is None or spec.loader is None:
        fail("cannot load workflow durable store")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def import_recovered(store: Any, mapping_path: pathlib.Path, task_uid: str,
                     recovered: dict[str, Any]) -> None:
    def import_if_absent(mapping: dict[str, Any]) -> None:
        tasks = mapping.setdefault("tasks", {})
        if not isinstance(tasks, dict):
            fail("default task mapping has invalid tasks structure")
        if task_uid not in tasks:
            tasks[task_uid] = recovered
        elif tasks[task_uid] != recovered:
            fail("default task mapping changed to a conflicting task record during recovery")
    store.transact_json(mapping_path, import_if_absent, {"version": 1, "tasks": {}})

def reconcile_recovered(store: Any, mapping_path: pathlib.Path, task_uid: str,
                        previous: dict[str, Any] | None,
                        recovered: dict[str, Any]) -> None:
    def replace_if_unchanged(mapping: dict[str, Any]) -> None:
        tasks = mapping.setdefault("tasks", {})
        if not isinstance(tasks, dict):
            fail("default task mapping has invalid tasks structure")
        current = tasks.get(task_uid)
        if current != previous:
            fail("default task mapping changed during terminal identity recovery")
        tasks[task_uid] = recovered
    store.transact_json(mapping_path, replace_if_unchanged, {"version": 1, "tasks": {}})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--mapping", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--main-ref", required=True)
    parser.add_argument("--pr-receipt", required=True)
    args = parser.parse_args()

    root = pathlib.Path(args.repo_root).resolve(strict=True)
    mapping_path = pathlib.Path(args.mapping).resolve(strict=True)
    receipt_path = pathlib.Path(args.pr_receipt).resolve(strict=True)
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if not isinstance(receipt, dict):
        fail("merge receipt is not an object")
    registry = registered_worktrees(root)
    default = read_mapping(mapping_path)
    default_tasks = default.get("tasks") or {}
    if args.task_uid in default_tasks and not isinstance(default_tasks[args.task_uid], dict):
        fail("existing terminal task record is incomplete")
    existing = default_tasks.get(args.task_uid)
    if existing is not None:
        existing_canonical = normalized(str(existing.get("canonical_worktree") or ""))
        root_canonical = normalized(str(root))
        existing_branch = str(existing.get("task_branch") or "")
        # Preserve the established fast path for a task-bound destination.
        # Recovery is required only for the known poisoned signature where a
        # terminal task was rebound to the default root/default branch.
        if existing_canonical != root_canonical and existing_branch != args.main_ref:
            print("existing")
            return 0

    discovered: list[tuple[pathlib.Path, dict[str, Any]]] = []
    for entry in registry.values():
        candidate_path = pathlib.Path(entry["path"]) / ".pm/github-project-sync/tasks.json"
        if candidate_path.resolve(strict=False) == mapping_path or not candidate_path.is_file():
            continue
        candidate_mapping = read_mapping(candidate_path)
        candidate = (candidate_mapping.get("tasks") or {}).get(args.task_uid)
        if candidate is None:
            continue
        validated = validate_record(
            candidate, task_uid=args.task_uid, main_ref=args.main_ref,
            receipt=receipt, registry=registry,
        )
        discovered.append((candidate_path, validated))
    if not discovered:
        fail("terminal task is absent from all registered worktree mappings")

    identities = {task_identity(record) for _, record in discovered}
    if len(identities) != 1:
        fail("conflicting terminal task identities exist across registered worktrees")
    canonical_key = normalized(str(discovered[0][1]["canonical_worktree"]))
    authoritative = [record for path, record in discovered
                     if normalized(path.parent.parent.parent) == canonical_key]
    if len(authoritative) != 1:
        fail("terminal task record is not retained by exactly one canonical task worktree")
    recovered = copy.deepcopy(authoritative[0])

    store = load_store(pathlib.Path(__file__).resolve().parent)
    if existing is not None:
        try:
            validated_existing = validate_record(
                existing, task_uid=args.task_uid, main_ref=args.main_ref,
                receipt=receipt, registry=registry,
            )
        except SystemExit:
            validated_existing = None
        if validated_existing is not None and task_identity(validated_existing) == task_identity(recovered):
            print("existing")
            return 0
        reconcile_recovered(store, mapping_path, args.task_uid, existing, recovered)
        print("repaired")
    else:
        import_recovered(store, mapping_path, args.task_uid, recovered)
        print("imported")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
