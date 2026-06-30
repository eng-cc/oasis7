#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
import time
from collections import Counter, OrderedDict
from typing import Any, Optional


ACTIVE_STATUSES = ("candidate", "committed", "blocked")
ALL_STATUSES = ("candidate", "committed", "blocked", "done", "deferred")
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}")


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def normalize_key(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


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


def load_tasks(root: pathlib.Path, statuses: set[str], mapping: Optional[dict[str, Any]] = None) -> dict[str, OrderedDict[str, Any]]:
    tasks: dict[str, OrderedDict[str, Any]] = {}
    task_paths = sorted((root / ".pm/tasks").glob("task_*.yaml"))
    if not task_paths:
        tasks.update(load_archived_tasks(root, statuses))
        if mapping:
            tasks.update(load_mapping_tasks(mapping, statuses))
        return tasks
    for path in task_paths:
        task = load_simple_yaml(path)
        task_uid = str(task.get("task_uid") or "")
        status = str(task.get("status") or "")
        if not task_uid.startswith("task_") or status not in statuses:
            continue
        task["task_path"] = str(path.relative_to(root))
        tasks[task_uid] = task
    return tasks


def load_mapping_tasks(mapping: dict[str, Any], statuses: set[str]) -> dict[str, OrderedDict[str, Any]]:
    tasks: dict[str, OrderedDict[str, Any]] = {}
    for task_uid, record in (mapping.get("tasks") or {}).items():
        if not isinstance(record, dict):
            continue
        status = str(record.get("status") or "")
        if not str(task_uid).startswith("task_") or status not in statuses:
            continue
        task = OrderedDict(record)
        task["task_uid"] = str(task_uid)
        task["task_path"] = str(record.get("task_path") or record.get("issue_url") or "")
        task["execution_log_path"] = str(record.get("execution_log_path") or record.get("issue_url") or "")
        tasks[str(task_uid)] = task
    return tasks


def load_archived_tasks(root: pathlib.Path, statuses: set[str]) -> dict[str, OrderedDict[str, Any]]:
    archive_path = root / ".pm/github-project-sync/task-archive.jsonl"
    if not archive_path.exists():
        return {}
    tasks: dict[str, OrderedDict[str, Any]] = {}
    for line in archive_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        task = OrderedDict(record.get("task") or {})
        task_uid = str(task.get("task_uid") or record.get("task_uid") or "")
        status = str(task.get("status") or "")
        if not task_uid.startswith("task_") or status not in statuses:
            continue
        task["task_uid"] = task_uid
        task["task_path"] = str(record.get("task_path") or task.get("task_path") or "")
        task["execution_log_path"] = str(record.get("execution_log_path") or task.get("execution_log_path") or "")
        tasks[task_uid] = task
    return tasks


def load_mapping(path: pathlib.Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": 1, "tasks": {}}
    return json.loads(path.read_text(encoding="utf-8"))


def run_json(cmd: list[str]) -> dict[str, Any]:
    result = run_subprocess_with_retry(cmd)
    stdout = result.stdout.strip()
    if not stdout:
        return {}
    return json.loads(stdout)


def run_subprocess_with_retry(cmd: list[str], *, retries: int = 4) -> subprocess.CompletedProcess[str]:
    for attempt in range(retries):
        try:
            return subprocess.run(cmd, check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=180)
        except subprocess.TimeoutExpired:
            if attempt + 1 >= retries:
                raise
            time.sleep(min(30, 2 ** attempt))
        except subprocess.CalledProcessError as exc:
            stderr = exc.stderr or ""
            retryable = any(
                marker in stderr.lower()
                for marker in (
                    "api rate limit exceeded",
                    "secondary rate limit",
                    "timeout",
                    "temporarily unavailable",
                    "try again",
                )
            )
            if not retryable or attempt + 1 >= retries:
                raise RuntimeError(f"command failed: {' '.join(cmd)}\n{stderr.strip()}") from exc
            time.sleep(min(60, 2 ** attempt))
    raise RuntimeError("unreachable subprocess retry state")


def run_passthrough(cmd: list[str]) -> int:
    return subprocess.run(cmd, check=False).returncode


def field_value(item: dict[str, Any], field_name: str) -> str:
    normalized = normalize_key(field_name)
    for key, value in item.items():
        if normalize_key(str(key)) == normalized:
            return "" if value is None else str(value)
    return ""


def item_task_uid(item: dict[str, Any]) -> str:
    direct = field_value(item, "Task UID")
    if TASK_UID_RE.fullmatch(direct):
        return direct
    body = str((item.get("content") or {}).get("body") or "")
    match = TASK_UID_RE.search(body)
    return match.group(0) if match else ""


def expected_project_values(task: OrderedDict[str, Any]) -> dict[str, str]:
    status = str(task.get("status") or "")
    workflow_phase = "blocked" if status == "blocked" else "done" if status in {"done", "deferred"} else "execution"
    project_status = "Todo" if status == "candidate" else "In Progress" if status in {"committed", "blocked"} else "Done"
    return {
        "Status": project_status,
        "Task UID": str(task.get("task_uid") or ""),
        "Owner Role": str(task.get("owner_role") or ""),
        "Module": str(task.get("module") or ""),
        "PM Status": status,
        "Workflow Phase": workflow_phase,
        "Priority": str(task.get("priority") or ""),
        "Canonical Worktree": str(task.get("worktree_hint") or ""),
        "Test Tier Required": "n/a",
    }


def selected_statuses(args: argparse.Namespace) -> set[str]:
    statuses = set(args.status or ACTIVE_STATUSES)
    if args.include_done:
        statuses.update({"done", "deferred"})
    return statuses


def mapping_path_for(root: pathlib.Path, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    return path if path.is_absolute() else root / path


def command_sync(args: argparse.Namespace) -> int:
    script = pathlib.Path(__file__).with_name("github-project-sync.py")
    cmd = [
        sys.executable,
        str(script),
        str(args.root),
        "--repo",
        args.repo,
        "--project-owner",
        args.project_owner,
        "--project-number",
        str(args.project_number),
        "--mapping",
        args.mapping,
        "--apply",
    ]
    if args.include_done:
        cmd.append("--include-done")
    for status in args.status or []:
        cmd.extend(["--status", status])
    if args.json:
        cmd.append("--json")
    return run_passthrough(cmd)


def command_audit(args: argparse.Namespace) -> int:
    root = args.root.resolve()
    statuses = selected_statuses(args)
    mapping_path = mapping_path_for(root, args.mapping)
    mapping = load_mapping(mapping_path)
    tasks = load_tasks(root, statuses, mapping)
    mapped_tasks = mapping.get("tasks", {})

    payload = run_json(
        [
            "gh",
            "project",
            "item-list",
            str(args.project_number),
            "--owner",
            args.project_owner,
            "--limit",
            str(args.limit),
            "--format",
            "json",
        ]
    )
    items = payload.get("items", []) or []
    project_items_by_task: dict[str, dict[str, Any]] = {}
    duplicate_project_task_uids: list[str] = []
    for item in items:
        uid = item_task_uid(item)
        if not uid:
            continue
        if uid in project_items_by_task:
            duplicate_project_task_uids.append(uid)
        project_items_by_task[uid] = item

    errors: list[str] = []
    warnings: list[str] = []
    status_counts = Counter(str(task.get("status") or "") for task in tasks.values())
    for uid, task in sorted(tasks.items()):
        record = mapped_tasks.get(uid) or {}
        if not record:
            errors.append(f"{uid}: missing mapping record")
            continue
        for key in ("issue_url", "issue_number", "project_item_id"):
            if not record.get(key):
                errors.append(f"{uid}: mapping missing {key}")
        item = project_items_by_task.get(uid)
        if not item:
            errors.append(f"{uid}: missing GitHub Project item")
            continue
        if record.get("project_item_id") and str(record.get("project_item_id")) != str(item.get("id") or ""):
            errors.append(f"{uid}: mapping project_item_id does not match live item")
        content = item.get("content") or {}
        if record.get("issue_url") and str(record.get("issue_url")) != str(content.get("url") or ""):
            errors.append(f"{uid}: mapping issue_url does not match live item content")
        if record.get("issue_number") and str(record.get("issue_number")) != str(content.get("number") or ""):
            errors.append(f"{uid}: mapping issue_number does not match live item content")
        for field_name, expected in expected_project_values(task).items():
            if not expected:
                continue
            actual = field_value(item, field_name)
            if actual != expected:
                errors.append(f"{uid}: field {field_name} expected {expected!r} got {actual!r}")

    for uid in duplicate_project_task_uids:
        errors.append(f"{uid}: duplicate GitHub Project item")
    extra_mapped = sorted(set(mapped_tasks) - set(tasks))
    if extra_mapped and args.strict_mapping:
        errors.extend(f"{uid}: mapping exists outside selected statuses" for uid in extra_mapped)
    elif extra_mapped:
        warnings.append(f"mapping has {len(extra_mapped)} tasks outside selected statuses")

    result = {
        "status": "failed" if errors else "ok",
        "project_owner": args.project_owner,
        "project_number": args.project_number,
        "mapping_path": str(mapping_path),
        "selected_count": len(tasks),
        "project_item_count": len(project_items_by_task),
        "selected_statuses": sorted(statuses),
        "status_counts": dict(sorted(status_counts.items())),
        "errors": errors,
        "warnings": warnings,
    }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(
            "github-project-workflow audit: "
            f"status={result['status']} selected={result['selected_count']} "
            f"project_items={result['project_item_count']} errors={len(errors)} warnings={len(warnings)}"
        )
        for error in errors[:20]:
            print(f"ERROR: {error}")
        if len(errors) > 20:
            print(f"ERROR: ... {len(errors) - 20} more")
        for warning in warnings[:20]:
            print(f"WARN: {warning}")
    return 1 if errors else 0


def command_step3_gate(args: argparse.Namespace) -> int:
    args.include_done = True
    args.strict_mapping = True
    return command_audit(args)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="GitHub Project-backed oasis7 PM workflow adapter.")
    parser.add_argument("root", type=pathlib.Path, help="repository root")
    parser.add_argument("--repo", required=True, help="GitHub repository, e.g. eng-cc/oasis7")
    parser.add_argument("--project-owner", required=True)
    parser.add_argument("--project-number", type=int, required=True)
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")
    parser.add_argument("--status", action="append", choices=ALL_STATUSES)
    parser.add_argument("--include-done", action="store_true")
    parser.add_argument("--json", action="store_true")
    subparsers = parser.add_subparsers(dest="command", required=True)

    sync = subparsers.add_parser("sync", help="apply .pm task metadata to GitHub Project")
    sync.add_argument("--status", action="append", choices=ALL_STATUSES, default=argparse.SUPPRESS)
    sync.add_argument("--include-done", action="store_true", default=argparse.SUPPRESS)
    sync.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    sync.set_defaults(func=command_sync)

    audit = subparsers.add_parser("audit", help="verify .pm, mapping, and GitHub Project agree")
    audit.add_argument("--status", action="append", choices=ALL_STATUSES, default=argparse.SUPPRESS)
    audit.add_argument("--include-done", action="store_true", default=argparse.SUPPRESS)
    audit.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    audit.add_argument("--limit", type=int, default=1000)
    audit.add_argument("--strict-mapping", action="store_true")
    audit.set_defaults(func=command_audit)

    step3_gate = subparsers.add_parser(
        "step3-gate",
        help="require full historical GitHub Project coverage before .pm task file deletion",
    )
    step3_gate.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    step3_gate.add_argument("--limit", type=int, default=1000)
    step3_gate.set_defaults(func=command_step3_gate)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
