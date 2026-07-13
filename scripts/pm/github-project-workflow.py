#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
import re
import subprocess
import sys
import time
from collections import Counter, OrderedDict
from typing import Any, Optional


ACTIVE_STATUSES = ("candidate", "committed", "blocked", "ready", "pr_watch")
ALL_STATUSES = ("candidate", "committed", "blocked", "ready", "pr_watch", "done", "deferred")
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}")
PROJECT_ITEM_NODES_QUERY = """
query($ids: [ID!]!) {
  nodes(ids: $ids) {
    ... on ProjectV2Item {
      id
      project {
        id
        number
      }
      content {
        ... on Issue {
          body
          number
          title
          url
        }
        ... on PullRequest {
          body
          number
          url
        }
      }
      fieldValues(first: 100) {
        nodes {
          ... on ProjectV2ItemFieldTextValue {
            text
            field {
              ... on ProjectV2FieldCommon {
                name
              }
            }
          }
          ... on ProjectV2ItemFieldSingleSelectValue {
            name
            field {
              ... on ProjectV2FieldCommon {
                name
              }
            }
          }
          ... on ProjectV2ItemFieldDateValue {
            date
            field {
              ... on ProjectV2FieldCommon {
                name
              }
            }
          }
          ... on ProjectV2ItemFieldNumberValue {
            number
            field {
              ... on ProjectV2FieldCommon {
                name
              }
            }
          }
        }
      }
    }
  }
}
"""


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def normalize_key(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


def normalized_acceptance(body: str) -> list[str]:
    lines = body.splitlines()
    try:
        start = next(index for index, line in enumerate(lines) if line.strip() == "Acceptance:") + 1
    except StopIteration:
        return []
    values: list[str] = []
    for line in lines[start:]:
        if not line.strip():
            if values:
                break
            continue
        match = re.match(r"^-\s+(?:\[[ xX]\]\s*)?(.*\S)\s*$", line)
        if not match:
            break
        values.append(match.group(1).strip())
    return values


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
    tasks.update(load_archived_tasks(root, statuses))
    if mapping:
        tasks.update(load_mapping_tasks(mapping, statuses))
    return tasks


def retired_task_files(root: pathlib.Path) -> list[str]:
    task_dir = root / ".pm/tasks"
    paths = sorted(task_dir.glob("task_*.yaml")) + sorted(task_dir.glob("*.execution.md"))
    return [str(path.relative_to(root)) for path in paths]


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


_store_path = pathlib.Path(__file__).with_name("workflow-durable-store.py")
if not _store_path.exists(): _store_path = pathlib.Path.cwd()/"scripts/pm/workflow-durable-store.py"
_store_spec = importlib.util.spec_from_file_location("workflow_durable_store", _store_path)
assert _store_spec and _store_spec.loader
durable_store = importlib.util.module_from_spec(_store_spec); _store_spec.loader.exec_module(durable_store)
def persist_mapping(path: pathlib.Path, snapshot: dict[str, Any]) -> None:
    """Persist explicit per-task patches through the shared field-policy CAS."""
    for task_uid, patch in (snapshot.get("tasks") or {}).items():
        durable_store.merge_task_record(path,task_uid,dict(patch))
    metadata={key:value for key,value in snapshot.items() if key!="tasks"}
    if metadata:
        durable_store.transact_json(path,lambda latest: latest.update(metadata),{"version":1,"tasks":{}})


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


def broad_graphql_budget() -> dict[str, Any]:
    """Fail closed before intentionally broad Project/issue operations."""
    query = "query { rateLimit { remaining resetAt } }"
    try:
        payload = run_json(["gh", "api", "graphql", "-f", f"query={query}"])
    except Exception as exc:
        return {"status":"capability_blocked","reason":"graphql_rate_limit_unavailable",
                "error":str(exc),"resumable":True,
                "resume":"restore GitHub rateLimit access and rerun; stale cache is not accepted"}
    rate = ((payload.get("data") or {}).get("rateLimit") or {})
    remaining, reset_at = rate.get("remaining"), str(rate.get("resetAt") or "")
    if not isinstance(remaining, int) or not reset_at:
        return {"status":"capability_blocked","reason":"graphql_rate_limit_unknown","resumable":True,
                "resume":"restore GitHub rateLimit visibility and rerun; stale cache is not accepted"}
    if remaining < 100:
        return {"status":"capability_blocked","reason":"graphql_budget_insufficient","remaining":remaining,
                "resetAt":reset_at,"resumable":True,"resume":f"resume after {reset_at}"}
    return {"status":"ok","remaining":remaining,"resetAt":reset_at}


def load_sync_module() -> Any:
    path = pathlib.Path(__file__).with_name("github-project-sync.py")
    spec = importlib.util.spec_from_file_location("github_project_sync", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def field_value(item: dict[str, Any], field_name: str) -> str:
    return field_value_from_lookup(normalized_field_values(item), field_name)


def normalized_field_values(item: dict[str, Any]) -> dict[str, str]:
    values: dict[str, str] = {}
    for key, value in item.items():
        normalized = normalize_key(str(key))
        if normalized not in values:
            values[normalized] = "" if value is None else str(value)
    return values


def field_value_from_lookup(field_values: dict[str, str], field_name: str) -> str:
    return field_values.get(normalize_key(field_name), "")


def item_task_uid(item: dict[str, Any]) -> str:
    direct = field_value_from_lookup(normalized_field_values(item), "Task UID")
    if TASK_UID_RE.fullmatch(direct):
        return direct
    body = str((item.get("content") or {}).get("body") or "")
    match = TASK_UID_RE.search(body)
    return match.group(0) if match else ""


def project_item_from_graphql_node(node: dict[str, Any]) -> dict[str, Any]:
    project = node.get("project") or {}
    item: dict[str, Any] = {
        "id": node.get("id") or "",
        "content": node.get("content") or {},
        "_project_id": project.get("id") or "",
        "_project_number": project.get("number") or "",
    }
    for field_value_node in ((node.get("fieldValues") or {}).get("nodes") or []):
        field = field_value_node.get("field") or {}
        field_name = str(field.get("name") or "")
        if not field_name:
            continue
        value = ""
        for key in ("name", "text", "date", "number"):
            if field_value_node.get(key) is not None:
                value = str(field_value_node.get(key))
                break
        item[field_name] = value
    return item


def fetch_project_items_by_ids(project_item_ids: list[str]) -> dict[str, dict[str, Any]]:
    if not project_item_ids:
        return {}
    cmd = ["gh", "api", "graphql", "-f", f"query={PROJECT_ITEM_NODES_QUERY}"]
    for project_item_id in project_item_ids:
        cmd.extend(["-f", f"ids[]={project_item_id}"])
    payload = run_json(cmd)
    items: dict[str, dict[str, Any]] = {}
    for node in (payload.get("data") or {}).get("nodes") or []:
        if not node:
            continue
        item = project_item_from_graphql_node(node)
        item_id = str(item.get("id") or "")
        if item_id:
            items[item_id] = item
    return items


def fetch_project_items_by_full_list(args: argparse.Namespace) -> tuple[dict[str, dict[str, Any]], list[str]]:
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
    project_items_by_task: dict[str, dict[str, Any]] = {}
    duplicate_project_task_uids: list[str] = []
    for item in payload.get("items", []) or []:
        uid = item_task_uid(item)
        if not uid:
            continue
        if uid in project_items_by_task:
            duplicate_project_task_uids.append(uid)
        project_items_by_task[uid] = item
    return project_items_by_task, duplicate_project_task_uids


def fetch_project_items_by_mapping(tasks: dict[str, OrderedDict[str, Any]], mapped_tasks: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], list[str]]:
    project_items_by_task: dict[str, dict[str, Any]] = {}
    item_ids: list[str] = []
    task_uid_by_item_id: dict[str, str] = {}
    for uid in sorted(tasks):
        record = mapped_tasks.get(uid) or {}
        project_item_id = str(record.get("project_item_id") or "")
        if not project_item_id:
            continue
        item_ids.append(project_item_id)
        task_uid_by_item_id[project_item_id] = uid
    items_by_id = fetch_project_items_by_ids(item_ids)
    for project_item_id, uid in task_uid_by_item_id.items():
        item = items_by_id.get(project_item_id)
        if item:
            project_items_by_task[uid] = item
    return project_items_by_task, []


def expected_project_id(args: argparse.Namespace, mapping: dict[str, Any]) -> str:
    project = mapping.get("project") or {}
    project_id = str(project.get("id") or "")
    if project_id:
        return project_id
    payload = run_json(
        [
            "gh",
            "project",
            "view",
            str(args.project_number),
            "--owner",
            args.project_owner,
            "--format",
            "json",
        ]
    )
    return str(payload.get("id") or "")


def expected_project_values(task: OrderedDict[str, Any]) -> dict[str, str]:
    status = str(task.get("status") or "")
    internal_phase = str(task.get("workflow_phase") or "")
    workflow_phase = {
        "blocked": "blocked",
        "ready": "pre_pr_ready",
        "pr_watch": "pr_watch",
        "done": "done",
        "deferred": "blocked",
    }.get(status, "execution")
    project_status = {
        "candidate": "Todo",
        "committed": "In Progress",
        "blocked": "Blocked",
        "ready": "Ready / PR",
        "pr_watch": "PR Watch",
        "done": "Done",
        "deferred": "Done",
    }.get(status, "Todo")
    if status == "done" and internal_phase != "post_merge_done":
        project_status = "In Progress"
    return {
        "Status": project_status,
        "Task UID": str(task.get("task_uid") or ""),
        "Owner Role": str(task.get("owner_role") or ""),
        "Module": str(task.get("module") or ""),
        "PM Status": status,
        "Workflow Phase": workflow_phase,
        "Priority": str(task.get("priority") or ""),
        "Canonical Worktree": str(task.get("worktree_hint") or ""),
        "PR": str(task.get("pr_url") or task.get("pull_request_url") or task.get("pr_number") or ""),
        "Test Tier Required": "n/a",
    }


def selected_statuses(args: argparse.Namespace) -> set[str]:
    if getattr(args, "task_uid", None) and not getattr(args, "status", None) and not args.include_done:
        return set(ALL_STATUSES)
    statuses = set(args.status or ACTIVE_STATUSES)
    if args.include_done:
        statuses.update({"done", "deferred"})
    return statuses


def mapping_path_for(root: pathlib.Path, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    return path if path.is_absolute() else root / path


def recover_missing_mapping_records(
    args: argparse.Namespace,
    mapping: dict[str, Any],
    tasks: dict[str, OrderedDict[str, Any]],
) -> tuple[bool, str]:
    missing = [uid for uid in sorted(tasks) if not (mapping.get("tasks") or {}).get(uid)]
    if not missing:
        return False, ""
    try:
        project_id = str((mapping.get("project") or {}).get("id") or "") or expected_project_id(args, mapping)
        recovered = load_sync_module().recover_project_mapping_for_task_uids(
            args.project_owner,
            args.project_number,
            args.repo,
            missing,
            project_id,
        )
    except Exception as exc:
        return False, f"mapping recovery failed before audit validation: {exc}"
    changed = False
    mapped_tasks = mapping.setdefault("tasks", {})
    for uid in missing:
        item = recovered.get(uid)
        if not item:
            continue
        record = mapped_tasks.setdefault(uid, {})
        for key in ("issue_url", "issue_number", "project_item_id"):
            value = item.get(key)
            if value and not record.get(key):
                record[key] = int(value) if key == "issue_number" else value
                changed = True
    return changed, ""


def command_sync(args: argparse.Namespace) -> int:
    if not getattr(args, "task_uid", None) and not getattr(args, "global_maintenance", False):
        die("sync requires --task-uid by default; broad traversal requires explicit --global-maintenance")
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
    if getattr(args, "task_uid", None):
        cmd.extend(["--task-uid", args.task_uid])
    if getattr(args, "global_maintenance", False):
        # The shared sync child owns the single live rate-limit preflight at the
        # boundary immediately before broad work. Do not duplicate that query
        # in this wrapper.
        cmd.append("--global-maintenance")
    if args.include_done:
        cmd.append("--include-done")
    for status in args.status or []:
        cmd.extend(["--status", status])
    if args.json:
        cmd.append("--json")
    return run_passthrough(cmd)


def command_audit(args: argparse.Namespace) -> int:
    if not getattr(args, "task_uid", None) and not getattr(args, "global_maintenance", False):
        die("audit requires --task-uid by default; broad traversal requires explicit --global-maintenance")
    if args.full_list or getattr(args, "global_maintenance", False):
        budget = broad_graphql_budget()
        if budget["status"] != "ok":
            print(json.dumps(budget, indent=2, sort_keys=True)); return 2
    root = args.root.resolve()
    statuses = selected_statuses(args)
    mapping_path = mapping_path_for(root, args.mapping)
    mapping = load_mapping(mapping_path)
    tasks = load_tasks(root, statuses, mapping)
    task_uid = getattr(args, "task_uid", None)
    if task_uid:
        tasks = {uid: task for uid, task in tasks.items() if uid == args.task_uid}
    mapped_tasks = mapping.get("tasks", {})
    retired_files = retired_task_files(root)

    errors: list[str] = []
    warnings: list[str] = []
    if task_uid and not tasks:
        errors.append(f"{task_uid}: task not found in selected mapping/archive records")
    try:
        if args.full_list:
            project_items_by_task, duplicate_project_task_uids = fetch_project_items_by_full_list(args)
            expected_live_project_id = ""
        else:
            project_items_by_task, duplicate_project_task_uids = fetch_project_items_by_mapping(tasks, mapped_tasks)
            expected_live_project_id = expected_project_id(args, mapping)
    except Exception as exc:
        project_items_by_task = {}
        duplicate_project_task_uids = []
        expected_live_project_id = ""
        errors.append(f"GitHub Project item fetch failed before audit validation: {exc}")
    if retired_files:
        errors.append(
            "retired .pm/tasks files present after GitHub Project Step 3: "
            + ", ".join(retired_files[:10])
            + (" ..." if len(retired_files) > 10 else "")
        )
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
        if expected_live_project_id and str(item.get("_project_id") or "") != expected_live_project_id:
            errors.append(f"{uid}: live item project_id does not match configured Project")
        content = item.get("content") or {}
        if record.get("issue_url") and str(record.get("issue_url")) != str(content.get("url") or ""):
            errors.append(f"{uid}: mapping issue_url does not match live item content")
        if record.get("issue_number") and str(record.get("issue_number")) != str(content.get("number") or ""):
            errors.append(f"{uid}: mapping issue_number does not match live item content")
        live_title = str(content.get("title") or "")
        if live_title.startswith("[PM] "):
            live_title = live_title[5:]
        if live_title and str(record.get("title") or "") != live_title:
            errors.append(f"{uid}: cached title drift; refresh explicitly from authoritative GitHub issue")
        body = str(content.get("body") or "")
        live_acceptance = normalized_acceptance(body)
        cached_acceptance = [
            re.sub(r"^\[[ xX]\]\s*", "", str(value)).strip()
            for value in (record.get("acceptance") or [])
        ]
        if cached_acceptance != live_acceptance:
            errors.append(f"{uid}: cached acceptance drift; refresh explicitly from authoritative GitHub issue")
        item_fields = normalized_field_values(item)
        for field_name, expected in expected_project_values(task).items():
            if not expected:
                continue
            actual = field_value_from_lookup(item_fields, field_name)
            if actual != expected:
                errors.append(f"{uid}: field {field_name} expected {expected!r} got {actual!r}")

    for uid in duplicate_project_task_uids:
        errors.append(f"{uid}: duplicate GitHub Project item")
    extra_mapped = [] if task_uid else sorted(set(mapped_tasks) - set(tasks))
    if extra_mapped and args.strict_mapping:
        errors.extend(f"{uid}: mapping exists outside selected statuses" for uid in extra_mapped)
    elif extra_mapped:
        warnings.append(f"mapping has {len(extra_mapped)} tasks outside selected statuses")

    result = {
        "status": "failed" if errors else "ok",
        "project_owner": args.project_owner,
        "project_number": args.project_number,
        "mapping_path": str(mapping_path),
        "task_uid": task_uid or "",
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
    args.full_list = True
    args.global_maintenance = True
    return command_audit(args)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="GitHub Project-backed oasis7 PM workflow adapter.")
    parser.add_argument("root", type=pathlib.Path, help="repository root")
    parser.add_argument("--repo", help="GitHub repository, e.g. eng-cc/oasis7; defaults to mapping project.repo")
    parser.add_argument("--project-owner", help="GitHub Project owner; defaults to mapping project.owner")
    parser.add_argument("--project-number", type=int, help="GitHub Project number; defaults to mapping project.number")
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")
    parser.add_argument("--status", action="append", choices=ALL_STATUSES)
    parser.add_argument("--include-done", action="store_true")
    parser.add_argument("--json", action="store_true")
    subparsers = parser.add_subparsers(dest="command", required=True)

    sync = subparsers.add_parser("sync", help="apply .pm task metadata to GitHub Project")
    sync.add_argument("--status", action="append", choices=ALL_STATUSES, default=argparse.SUPPRESS)
    sync.add_argument("--include-done", action="store_true", default=argparse.SUPPRESS)
    sync.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    sync.add_argument("--task-uid", help="Sync exactly one task_uid without broad Project traversal")
    sync.add_argument("--global-maintenance", action="store_true", help="explicitly authorize guarded broad sync")
    sync.set_defaults(func=command_sync)

    audit = subparsers.add_parser("audit", help="verify .pm, mapping, and GitHub Project agree")
    audit.add_argument("--status", action="append", choices=ALL_STATUSES, default=argparse.SUPPRESS)
    audit.add_argument("--include-done", action="store_true", default=argparse.SUPPRESS)
    audit.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    audit.add_argument("--limit", type=int, default=1000)
    audit.add_argument("--full-list", action="store_true")
    audit.add_argument("--strict-mapping", action="store_true")
    audit.add_argument("--task-uid", help="Audit one task_uid across all statuses unless --status is supplied")
    audit.add_argument("--global-maintenance", action="store_true", help="explicitly authorize guarded broad audit")
    audit.set_defaults(func=command_audit)

    step3_gate = subparsers.add_parser(
        "step3-gate",
        help="require full historical GitHub Project coverage before .pm task file deletion",
    )
    step3_gate.add_argument("--json", action="store_true", default=argparse.SUPPRESS)
    step3_gate.add_argument("--limit", type=int, default=1000)
    step3_gate.set_defaults(func=command_step3_gate)
    return parser


def apply_project_defaults(args: argparse.Namespace) -> None:
    mapping_path = mapping_path_for(args.root.resolve(), args.mapping)
    mapping = load_mapping(mapping_path)
    project = mapping.get("project") or {}
    if not args.repo:
        args.repo = str(project.get("repo") or "")
    if not args.project_owner:
        args.project_owner = str(project.get("owner") or "")
    if args.project_number is None and project.get("number") not in (None, ""):
        args.project_number = int(project["number"])
    missing = []
    if not args.repo:
        missing.append("--repo")
    if not args.project_owner:
        missing.append("--project-owner")
    if args.project_number is None:
        missing.append("--project-number")
    if missing:
        die(
            "missing "
            + ", ".join(missing)
            + "; pass explicit values or refresh .pm/github-project-sync/tasks.json project metadata"
        )


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    apply_project_defaults(args)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
