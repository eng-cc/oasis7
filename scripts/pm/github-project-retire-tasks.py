#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
from collections import Counter, OrderedDict
from datetime import datetime
from typing import Any


ALL_STATUSES = ("candidate", "committed", "blocked", "done", "deferred")


def parse_scalar(value: str) -> Any:
    value = value.strip()
    if value in {"null", "None"}:
        return None
    if value == "[]":
        return []
    if len(value) >= 2 and value[0] == value[-1] == '"':
        return value[1:-1]
    return value


def load_simple_yaml(path: pathlib.Path) -> OrderedDict[str, Any]:
    data: OrderedDict[str, Any] = OrderedDict()
    current_key: str | None = None
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        if not raw_line.startswith(" ") and ": " in raw_line:
            key, value = raw_line.split(": ", 1)
            data[key] = parse_scalar(value)
            current_key = key if value.strip() == "[]" else None
            continue
        if raw_line.startswith("  - ") and current_key:
            if not isinstance(data.get(current_key), list):
                data[current_key] = []
            data[current_key].append(parse_scalar(raw_line[4:]))
    return data


def load_mapping(path: pathlib.Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": 1, "tasks": {}}
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def relative(root: pathlib.Path, path: pathlib.Path) -> str:
    return str(path.relative_to(root))


def build_archive(root: pathlib.Path, mapping_path: pathlib.Path, archive_path: pathlib.Path) -> dict[str, Any]:
    mapping = load_mapping(mapping_path)
    mapped_tasks = mapping.get("tasks", {})
    records: list[dict[str, Any]] = []
    errors: list[str] = []
    status_counts: Counter[str] = Counter()
    archived_at = datetime.now().astimezone().isoformat(timespec="seconds")

    task_paths = sorted((root / ".pm/tasks").glob("task_*.yaml"))
    if not task_paths and archive_path.exists():
        for line in archive_path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            record = json.loads(line)
            status_counts[str(((record.get("task") or {}).get("status") or ""))] += 1
            records.append(record)
        return {
            "status": "ok",
            "archive_path": str(archive_path),
            "selected_count": len(records),
            "status_counts": dict(sorted((key, value) for key, value in status_counts.items() if key)),
            "errors": [],
            "archive_reused": True,
        }
    if not task_paths:
        return {
            "status": "failed",
            "archive_path": str(archive_path),
            "selected_count": 0,
            "status_counts": {},
            "errors": ["no .pm task files found and archive does not exist"],
            "archive_reused": False,
        }

    for task_path in task_paths:
        task = load_simple_yaml(task_path)
        task_uid = str(task.get("task_uid") or "")
        status = str(task.get("status") or "")
        if not task_uid.startswith("task_"):
            errors.append(f"{relative(root, task_path)}: missing task_uid")
            continue
        if status not in ALL_STATUSES:
            errors.append(f"{task_uid}: unexpected status {status!r}")
            continue
        record = mapped_tasks.get(task_uid) or {}
        for key in ("issue_url", "issue_number", "project_item_id"):
            if not record.get(key):
                errors.append(f"{task_uid}: mapping missing {key}")
        execution_log_rel = str(task.get("execution_log_path") or f".pm/tasks/{task_uid}.execution.md")
        execution_log_path = root / execution_log_rel
        if not execution_log_path.is_file():
            errors.append(f"{task_uid}: missing execution log {execution_log_rel}")
            execution_log_text = ""
        else:
            execution_log_text = execution_log_path.read_text(encoding="utf-8")
        task_text = task_path.read_text(encoding="utf-8")
        task["task_path"] = relative(root, task_path)
        records.append(
            {
                "task_uid": task_uid,
                "archived_at": archived_at,
                "task_path": relative(root, task_path),
                "task_sha256": sha256_text(task_text),
                "task": task,
                "execution_log_path": execution_log_rel,
                "execution_log_sha256": sha256_text(execution_log_text),
                "execution_log_text": execution_log_text,
                "github_project_mapping": record,
            }
        )
        status_counts[status] += 1

    archive_path.parent.mkdir(parents=True, exist_ok=True)
    if errors:
        return {
            "status": "failed",
            "archive_path": str(archive_path),
            "selected_count": len(records),
            "status_counts": dict(sorted(status_counts.items())),
            "errors": errors,
        }
    with archive_path.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, sort_keys=True, ensure_ascii=False) + "\n")
    return {
        "status": "ok",
        "archive_path": str(archive_path),
        "selected_count": len(records),
        "status_counts": dict(sorted(status_counts.items())),
        "errors": [],
        "archive_reused": False,
    }


def delete_task_files(root: pathlib.Path, dry_run: bool) -> dict[str, Any]:
    paths = sorted((root / ".pm/tasks").glob("task_*.yaml")) + sorted((root / ".pm/tasks").glob("*.execution.md"))
    deleted: list[str] = []
    for path in paths:
        deleted.append(relative(root, path))
        if not dry_run:
            path.unlink()
    return {"deleted_count": len(paths), "deleted_paths": deleted}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Archive and retire repo-local .pm task files after GitHub Project migration.")
    parser.add_argument("root", type=pathlib.Path, help="repository root")
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")
    parser.add_argument("--archive", default=".pm/github-project-sync/task-archive.jsonl")
    parser.add_argument("--summary", default=".pm/github-project-sync/task-retirement-summary.json")
    parser.add_argument("--delete", action="store_true", help="delete .pm/tasks task yaml and execution log files after archive")
    parser.add_argument("--json", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    root = args.root.resolve()
    mapping_path = pathlib.Path(args.mapping)
    if not mapping_path.is_absolute():
        mapping_path = root / mapping_path
    archive_path = pathlib.Path(args.archive)
    if not archive_path.is_absolute():
        archive_path = root / archive_path
    summary_path = pathlib.Path(args.summary)
    if not summary_path.is_absolute():
        summary_path = root / summary_path

    result = build_archive(root, mapping_path, archive_path)
    if result["status"] == "ok" and args.delete:
        result["deletion"] = delete_task_files(root, dry_run=False)
    else:
        result["deletion"] = {"deleted_count": 0, "deleted_paths": []}
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False))
    else:
        print(
            "github-project-retire-tasks: "
            f"status={result['status']} selected={result['selected_count']} "
            f"deleted={result['deletion']['deleted_count']} errors={len(result['errors'])}"
        )
    return 0 if result["status"] == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(main())
