#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import re
import threading
import time
import urllib.error
import urllib.request
from collections import OrderedDict
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from typing import Any


ACTIVE_STATUSES = ("candidate", "committed", "blocked", "ready", "pr_watch")
ALL_STATUSES = ("candidate", "committed", "blocked", "ready", "pr_watch", "done", "deferred")
FIELD_NAMES = {
    "task_uid": "Task UID",
    "owner_role": "Owner Role",
    "module": "Module",
    "pm_status": "PM Status",
    "workflow_phase": "Workflow Phase",
    "priority": "Priority",
    "blocked_reason": "Blocked Reason",
    "canonical_worktree": "Canonical Worktree",
    "pr": "PR",
    "test_tier_required": "Test Tier Required",
    "last_pm_update": "Last PM Update",
}
SINGLE_SELECT_FIELDS = {
    "Status",
    "Owner Role",
    "Module",
    "PM Status",
    "Workflow Phase",
    "Priority",
    "Test Tier Required",
}
TASK_UID_RE = re.compile(r"task_uid:\s*(task_[0-9a-f]{32})")
ISSUE_URL_RE = re.compile(r"/issues/(\d+)(?:$|[?#])")
RECOVERY_BATCH_SIZE = 10
_PROJECT_CONTEXT_CACHE: dict[tuple[str, int], tuple[str, dict[str, dict[str, Any]]]] = {}


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


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


def load_tasks(root: pathlib.Path, statuses: set[str]) -> list[OrderedDict[str, Any]]:
    tasks: list[OrderedDict[str, Any]] = []
    task_paths = sorted((root / ".pm/tasks").glob("task_*.yaml"))
    if not task_paths:
        return load_archived_tasks(root, statuses)
    for path in task_paths:
        task = load_simple_yaml(path)
        task_uid = str(task.get("task_uid") or "")
        status = str(task.get("status") or "")
        if not task_uid.startswith("task_") or status not in statuses:
            continue
        task["task_path"] = str(path.relative_to(root))
        tasks.append(task)
    return tasks


def load_archived_tasks(root: pathlib.Path, statuses: set[str]) -> list[OrderedDict[str, Any]]:
    archive_path = root / ".pm/github-project-sync/task-archive.jsonl"
    if not archive_path.exists():
        return []
    tasks: list[OrderedDict[str, Any]] = []
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
        tasks.append(task)
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


def run_text(cmd: list[str]) -> str:
    result = run_subprocess_with_retry(cmd)
    return result.stdout.strip()


def run_subprocess_with_retry(cmd: list[str], *, retries: int = 4) -> subprocess.CompletedProcess[str]:
    for attempt in range(retries):
        try:
            return subprocess.run(cmd, check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            if attempt + 1 >= retries:
                raise
            time.sleep(min(30, 2 ** attempt))
    raise RuntimeError("unreachable subprocess retry state")


def github_token() -> str:
    return run_text(["gh", "auth", "token"])


def github_json_request(token: str, url: str, payload: dict[str, Any], *, retries: int = 5) -> dict[str, Any]:
    body = json.dumps(payload).encode("utf-8")
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "Content-Type": "application/json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    for attempt in range(retries):
        request = urllib.request.Request(url, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")
            retry_after = exc.headers.get("Retry-After")
            if exc.code in {403, 429, 500, 502, 503, 504} and attempt + 1 < retries:
                delay = int(retry_after) if retry_after and retry_after.isdigit() else min(60, 2 ** attempt)
                time.sleep(delay)
                continue
            raise RuntimeError(f"GitHub API request failed {exc.code}: {detail[:500]}") from exc
    raise RuntimeError("GitHub API request failed after retries")


def graphql_request(token: str, query: str, variables: dict[str, Any] | None = None) -> dict[str, Any]:
    payload = github_json_request(
        token,
        "https://api.github.com/graphql",
        {"query": query, "variables": variables or {}},
    )
    if payload.get("errors"):
        raise RuntimeError(f"GitHub GraphQL errors: {payload['errors']}")
    return payload.get("data") or {}


def broad_rate_limit_guard(token: str = "") -> dict[str, Any]:
    try:
        payload = run_json(["gh","api","graphql","-f","query=query { rateLimit { remaining resetAt } }"])
        rate = ((payload.get("data") or {}).get("rateLimit") or {})
    except Exception as exc:
        return {"status":"capability_blocked","reason":"graphql_rate_limit_unavailable","error":str(exc),
                "resumable":True,"resume":"restore rateLimit access and rerun"}
    remaining, reset_at = rate.get("remaining"), str(rate.get("resetAt") or "")
    if not isinstance(remaining, int) or not reset_at:
        return {"status":"capability_blocked","reason":"graphql_rate_limit_unknown","resumable":True,
                "resume":"restore rateLimit visibility and rerun"}
    if remaining < 100:
        return {"status":"capability_blocked","reason":"graphql_budget_insufficient","remaining":remaining,
                "resetAt":reset_at,"resumable":True,"resume":f"resume after {reset_at}"}
    return {"status":"ok","remaining":remaining,"resetAt":reset_at}


def create_issue_direct(token: str, repo: str, task: OrderedDict[str, Any]) -> dict[str, Any]:
    owner, name = repo.split("/", 1)
    payload = github_json_request(
        token,
        f"https://api.github.com/repos/{owner}/{name}/issues",
        {
            "title": f"[PM] {task.get('title')}",
            "body": issue_body(task),
        },
    )
    return {
        "issue_url": str(payload.get("html_url") or ""),
        "issue_number": int(payload.get("number") or 0),
        "content_id": str(payload.get("node_id") or ""),
    }


def add_project_item_direct(token: str, project_id: str, content_id: str) -> str:
    data = graphql_request(
        token,
        """
        mutation($projectId: ID!, $contentId: ID!) {
          addProjectV2ItemById(input: {projectId: $projectId, contentId: $contentId}) {
            item { id }
          }
        }
        """,
        {"projectId": project_id, "contentId": content_id},
    )
    item_id = str(((data.get("addProjectV2ItemById") or {}).get("item") or {}).get("id") or "")
    if not item_id:
        raise RuntimeError("GitHub GraphQL addProjectV2ItemById returned no item id")
    return item_id


def update_fields_direct(
    token: str,
    project_id: str,
    item_id: str,
    task: OrderedDict[str, Any],
    fields: dict[str, dict[str, Any]],
    only_fields: set[str] | None = None,
    current_values: dict[str, str] | None = None,
) -> tuple[int, list[str]]:
    values = project_field_values(task)
    mutations: list[str] = []
    variables: dict[str, Any] = {"projectId": project_id, "itemId": item_id}
    skipped: list[str] = []
    variable_index = 0
    for field_name, value in values.items():
        if only_fields is not None and field_name not in only_fields:
            continue
        if current_values is not None and str(current_values.get(field_name) or "") == str(value):
            skipped.append(f"{field_name}:unchanged")
            continue
        field = fields.get(field_name)
        if not field:
            skipped.append(f"{field_name}:missing_field")
            continue
        if not value:
            skipped.append(f"{field_name}:empty_value")
            continue
        if field_name == "Last PM Update":
            skipped.append(f"{field_name}:deferred_date_field")
            continue
        field_var = f"field{variable_index}"
        variables[field_var] = str(field["id"])
        if field_name in SINGLE_SELECT_FIELDS:
            option_id = (field.get("options_by_name") or {}).get(value)
            if not option_id:
                skipped.append(f"{field_name}:missing_option:{value}")
                continue
            option_var = f"option{variable_index}"
            variables[option_var] = option_id
            value_expr = f"{{singleSelectOptionId: ${option_var}}}"
            var_decl = f"${option_var}: String!, "
        else:
            text_var = f"text{variable_index}"
            variables[text_var] = value
            value_expr = f"{{text: ${text_var}}}"
            var_decl = f"${text_var}: String!, "
        mutations.append(
            f"""
            f{variable_index}: updateProjectV2ItemFieldValue(input: {{
              projectId: $projectId,
              itemId: $itemId,
              fieldId: ${field_var},
              value: {value_expr}
            }}) {{ projectV2Item {{ id }} }}
            """
        )
        variables[f"{field_var}_decl"] = var_decl
        variable_index += 1
    if not mutations:
        return 0, skipped
    dynamic_decls = []
    for index in range(variable_index):
        dynamic_decls.append(f"$field{index}: ID!")
        if f"option{index}" in variables:
            dynamic_decls.append(f"$option{index}: String!")
        if f"text{index}" in variables:
            dynamic_decls.append(f"$text{index}: String!")
    variables = {key: value for key, value in variables.items() if not key.endswith("_decl")}
    query = (
        "mutation($projectId: ID!, $itemId: ID!, "
        + ", ".join(dynamic_decls)
        + ") {"
        + "\n".join(mutations)
        + "}"
    )
    graphql_request(token, query, variables)
    return len(mutations), skipped


def issue_number_from_url(issue_url: str) -> int | None:
    match = ISSUE_URL_RE.search(issue_url)
    return int(match.group(1)) if match else None


def project_context(owner: str, number: int) -> tuple[str, dict[str, dict[str, Any]]]:
    cache_key = (owner, number)
    if cache_key in _PROJECT_CONTEXT_CACHE:
        return _PROJECT_CONTEXT_CACHE[cache_key]
    project = run_json(["gh", "project", "view", str(number), "--owner", owner, "--format", "json"])
    project_id = str(project.get("id") or "")
    if not project_id:
        die("github-project-sync: project id missing from gh project view")
    fields_payload = run_json(["gh", "project", "field-list", str(number), "--owner", owner, "--format", "json"])
    fields: dict[str, dict[str, Any]] = {}
    for field in fields_payload.get("fields", []):
        by_name = dict(field)
        options = {
            str(option.get("name")): str(option.get("id"))
            for option in field.get("options", []) or []
            if option.get("name") and option.get("id")
        }
        by_name["options_by_name"] = options
        fields[str(field.get("name"))] = by_name
    _PROJECT_CONTEXT_CACHE[cache_key] = (project_id, fields)
    return _PROJECT_CONTEXT_CACHE[cache_key]


def project_id_for(owner: str, number: int, mapping: dict[str, Any]) -> str:
    project_id = str((mapping.get("project") or {}).get("id") or "")
    if project_id:
        return project_id
    project = run_json(["gh", "project", "view", str(number), "--owner", owner, "--format", "json"])
    return str(project.get("id") or "")


def recover_project_mapping(owner: str, number: int) -> dict[str, dict[str, str]]:
    payload = run_json(["gh", "project", "item-list", str(number), "--owner", owner, "--limit", "1000", "--format", "json"])
    recovered: dict[str, dict[str, str]] = {}
    for item in payload.get("items", []) or []:
        item_id = str(item.get("id") or "")
        content = item.get("content") or {}
        issue_url = str(content.get("url") or "")
        body = str(content.get("body") or "")
        match = TASK_UID_RE.search(body)
        if match and item_id and issue_url:
            recovered[match.group(1)] = {
                "issue_url": issue_url,
                "issue_number": str(issue_number_from_url(issue_url) or ""),
                "project_item_id": item_id,
            }
    return recovered


def recover_project_mapping_for_task_uids(
    owner: str,
    number: int,
    repo: str,
    task_uids: list[str],
    project_id: str = "",
) -> dict[str, dict[str, str]]:
    recovered: dict[str, dict[str, str]] = {}
    selected = []
    for task_uid in sorted(set(task_uids)):
        if not TASK_UID_RE.fullmatch(f"task_uid: {task_uid}"):
            continue
        selected.append(task_uid)
    for start in range(0, len(selected), RECOVERY_BATCH_SIZE):
        batch = selected[start : start + RECOVERY_BATCH_SIZE]
        variable_defs = ", ".join(f"$q{index}: String!" for index in range(len(batch)))
        searches = "\n".join(
            f"""
            s{index}: search(query: $q{index}, type: ISSUE, first: 2) {{
              nodes {{
                ... on Issue {{
                  number
                  url
                  body
                  projectItems(first: 20) {{
                    nodes {{
                      id
                      project {{
                        id
                        number
                      }}
                      fieldValues(first: 100) {{ nodes {{
                        ... on ProjectV2ItemFieldTextValue {{ text field {{ ... on ProjectV2FieldCommon {{ name }} }} }}
                        ... on ProjectV2ItemFieldSingleSelectValue {{ name field {{ ... on ProjectV2FieldCommon {{ name }} }} }}
                      }} }}
                    }}
                  }}
                }}
              }}
            }}
            """
            for index in range(len(batch))
        )
        query = f"query({variable_defs}) {{\n{searches}\n}}"
        cmd = ["gh", "api", "graphql", "-f", f"query={query}"]
        for index, task_uid in enumerate(batch):
            cmd.extend(["-f", f"q{index}=repo:{repo} {task_uid} in:body"])
        payload = run_json(cmd)
        data = payload.get("data") or {}
        for index, task_uid in enumerate(batch):
            nodes = ((data.get(f"s{index}") or {}).get("nodes") or [])
            matches = []
            for issue in nodes:
                body = str(issue.get("body") or "")
                if re.search(rf"^task_uid:\s*{re.escape(task_uid)}$", body, re.MULTILINE):
                    matches.append(issue)
            if len(matches) != 1:
                continue
            issue = matches[0]
            item_id = ""
            for node in ((issue.get("projectItems") or {}).get("nodes") or []):
                project = node.get("project") or {}
                if project_id and str(project.get("id") or "") != project_id:
                    continue
                if int(project.get("number") or 0) == int(number):
                    item_id = str(node.get("id") or "")
                    break
            issue_url = str(issue.get("url") or "")
            issue_number = int(issue.get("number") or issue_number_from_url(issue_url) or 0)
            if item_id and issue_url and issue_number:
                selected_item = next(node for node in ((issue.get("projectItems") or {}).get("nodes") or [])
                                     if str(node.get("id") or "") == item_id)
                live_values = {
                    str(((value.get("field") or {}).get("name") or "")): str(value.get("name") or value.get("text") or "")
                    for value in (((selected_item.get("fieldValues") or {}).get("nodes")) or [])
                    if str(((value.get("field") or {}).get("name") or ""))
                }
                recovered[task_uid] = {
                    "issue_url": issue_url,
                    "issue_number": str(issue_number),
                    "project_item_id": item_id,
                    "project_field_values": live_values,
                }
    return recovered


def workflow_phase_for(status: str) -> str:
    if status == "blocked":
        return "blocked"
    if status == "ready":
        return "pre_pr_ready"
    if status == "pr_watch":
        return "pr_watch"
    if status in {"done", "deferred"}:
        return "task_done" if status == "done" else "blocked"
    return "execution"


def project_workflow_phase(task: OrderedDict[str, Any]) -> str:
    """Map fine terminal receipt phases onto the Project cockpit's coarse done lane."""
    internal = str(task.get("workflow_phase") or workflow_phase_for(str(task.get("status") or "")))
    if internal in {"task_done", "main_sync", "post_merge_done"}:
        return "done"
    return internal


def project_status_for(status: str) -> str:
    if status == "candidate":
        return "Todo"
    if status == "blocked":
        return "Blocked"
    if status == "ready":
        return "Ready / PR"
    if status == "pr_watch":
        return "PR Watch"
    if status == "committed":
        return "In Progress"
    if status in {"done", "deferred"}:
        return "Done"
    return "Todo"


def first_date(value: Any) -> str:
    if not value:
        return datetime.now().astimezone().date().isoformat()
    text = str(value)
    return text[:10]


def issue_body(task: OrderedDict[str, Any]) -> str:
    lines = [
        "<!-- oasis7-pm-task-sync -->",
        f"task_uid: {task.get('task_uid')}",
        "",
        "This GitHub issue was generated from an oasis7 `.pm` task during the GitHub Project migration.",
        "",
        "Canonical source during migration:",
        f"- Task file: `{task.get('task_path')}`",
        f"- Execution log: `{task.get('execution_log_path')}`",
        "",
        "Task metadata:",
        f"- owner_role: `{task.get('owner_role')}`",
        f"- module: `{task.get('module') or ''}`",
        f"- status: `{task.get('status')}`",
        f"- priority: `{task.get('priority')}`",
        f"- worktree_hint: `{task.get('worktree_hint') or ''}`",
    ]
    source_refs = task.get("source_refs") or []
    if source_refs:
        lines.append("")
        lines.append("Source refs:")
        for ref in source_refs:
            lines.append(f"- `{ref}`")
    acceptance = task.get("acceptance") or []
    if acceptance:
        lines.append("")
        lines.append("Acceptance:")
        for item in acceptance:
            lines.append(f"- {item}")
    return "\n".join(lines) + "\n"


def project_field_values(task: OrderedDict[str, Any]) -> dict[str, str]:
    status = str(task.get("status") or "")
    return {
        "Status": "In Progress" if status == "done" and task.get("workflow_phase") != "post_merge_done" else project_status_for(status),
        "Task UID": str(task.get("task_uid") or ""),
        "Owner Role": str(task.get("owner_role") or ""),
        "Module": str(task.get("module") or ""),
        "PM Status": status,
        "Workflow Phase": project_workflow_phase(task),
        "Priority": str(task.get("priority") or ""),
        "Blocked Reason": "" if task.get("status") != "blocked" else "blocked in .pm",
        "Canonical Worktree": str(task.get("worktree_hint") or ""),
        "PR": str(task.get("pr_url") or task.get("pull_request_url") or task.get("pr_number") or ""),
        "Test Tier Required": "n/a",
        "Last PM Update": first_date(task.get("updated_at")),
    }


def create_issue(repo: str, task: OrderedDict[str, Any]) -> str:
    title = f"[PM] {task.get('title')}"
    body = issue_body(task)
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = handle.name
    try:
        return run_text(["gh", "issue", "create", "-R", repo, "--title", title, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def edit_text_field(project_id: str, item_id: str, field_id: str, value: str) -> None:
    run_json(
        [
            "gh",
            "project",
            "item-edit",
            "--id",
            item_id,
            "--project-id",
            project_id,
            "--field-id",
            field_id,
            "--text",
            value,
            "--format",
            "json",
        ]
    )


def edit_date_field(project_id: str, item_id: str, field_id: str, value: str) -> None:
    run_json(
        [
            "gh",
            "project",
            "item-edit",
            "--id",
            item_id,
            "--project-id",
            project_id,
            "--field-id",
            field_id,
            "--date",
            value,
            "--format",
            "json",
        ]
    )


def edit_select_field(project_id: str, item_id: str, field: dict[str, Any], value: str) -> bool:
    option_id = (field.get("options_by_name") or {}).get(value)
    if not option_id:
        return False
    run_json(
        [
            "gh",
            "project",
            "item-edit",
            "--id",
            item_id,
            "--project-id",
            project_id,
            "--field-id",
            str(field["id"]),
            "--single-select-option-id",
            option_id,
            "--format",
            "json",
        ]
    )
    return True


def update_fields(
    project_id: str,
    item_id: str,
    task: OrderedDict[str, Any],
    fields: dict[str, dict[str, Any]],
    only_fields: set[str] | None = None,
    current_values: dict[str, str] | None = None,
) -> tuple[int, list[str]]:
    values = project_field_values(task)
    updated = 0
    skipped: list[str] = []
    for field_name, value in values.items():
        if only_fields is not None and field_name not in only_fields:
            continue
        if current_values is not None and str(current_values.get(field_name) or "") == str(value):
            skipped.append(f"{field_name}:unchanged")
            continue
        field = fields.get(field_name)
        if not field:
            skipped.append(f"{field_name}:missing_field")
            continue
        if not value:
            skipped.append(f"{field_name}:empty_value")
            continue
        if field_name in SINGLE_SELECT_FIELDS:
            if edit_select_field(project_id, item_id, field, value):
                updated += 1
            else:
                skipped.append(f"{field_name}:missing_option:{value}")
            continue
        if field_name == "Last PM Update":
            skipped.append(f"{field_name}:deferred_date_field")
        else:
            edit_text_field(project_id, item_id, str(field["id"]), value)
            updated += 1
    return updated, skipped


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Sync oasis7 .pm tasks to GitHub Issues and a GitHub Project.")
    parser.add_argument("root", help="repository root")
    parser.add_argument("--repo", required=True, help="GitHub repository, e.g. eng-cc/oasis7")
    parser.add_argument("--project-owner", required=True, help="GitHub Project owner login")
    parser.add_argument("--project-number", type=int, required=True)
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")
    parser.add_argument("--status", action="append", choices=ALL_STATUSES, help="task status to include; repeatable")
    parser.add_argument("--include-done", action="store_true", help="include done and deferred tasks")
    parser.add_argument("--limit", type=int, default=0, help="maximum tasks to sync after filtering")
    parser.add_argument("--task-uid", help="sync exactly one task_uid; avoids broad Project recovery")
    parser.add_argument("--global-maintenance", action="store_true", help="explicitly authorize guarded broad sync")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true", help="plan only after read-only GitHub Project mapping recovery")
    mode.add_argument("--apply", action="store_true", help="create/update GitHub issues and project items")
    parser.add_argument("--direct-api", action="store_true", help="use GitHub REST/GraphQL directly instead of per-field gh subprocesses")
    parser.add_argument("--jobs", type=int, default=4, help="parallel jobs for --direct-api")
    parser.add_argument("--missing-only", action="store_true", help="only sync tasks with incomplete mapping records")
    parser.add_argument("--skip-recover", action="store_true", help="skip recovering existing project items from GitHub before apply")
    parser.add_argument("--field", action="append", help="only update the named Project field; repeatable")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if not args.task_uid and not args.global_maintenance:
        die("--task-uid is required by default; use --global-maintenance for guarded broad sync")
    if args.global_maintenance:
        budget = broad_rate_limit_guard()
        if budget["status"] != "ok":
            print(json.dumps(budget, indent=2, sort_keys=True)); return 2
    root = pathlib.Path(args.root).resolve()
    if not args.apply:
        args.dry_run = True
    statuses = set(args.status or ACTIVE_STATUSES)
    if args.include_done:
        statuses.update({"done", "deferred"})
    tasks = load_tasks(root, statuses)
    if args.task_uid:
        tasks = [task for task in tasks if str(task.get("task_uid") or "") == args.task_uid]
        args.skip_recover = False
    mapping_path = pathlib.Path(args.mapping)
    if not mapping_path.is_absolute():
        mapping_path = root / mapping_path
    mapping = load_mapping(mapping_path)
    mapping.setdefault("tasks", {})
    only_fields = set(args.field) if args.field else None
    if args.missing_only:
        filtered_tasks = []
        for task in tasks:
            record = mapping.get("tasks", {}).get(str(task["task_uid"]), {})
            if not (
                record.get("issue_url")
                and record.get("issue_number")
                and record.get("project_item_id")
                and record.get("status")
            ):
                filtered_tasks.append(task)
        tasks = filtered_tasks
    if args.limit:
        tasks = tasks[: args.limit]

    if not args.skip_recover:
        selected_uids = [str(task["task_uid"]) for task in tasks]
        project_id = project_id_for(args.project_owner, args.project_number, mapping)
        for uid, recovered in recover_project_mapping_for_task_uids(
            args.project_owner,
            args.project_number,
            args.repo,
            selected_uids,
            project_id,
        ).items():
            record = mapping["tasks"].setdefault(uid, {})
            record.setdefault("issue_url", recovered["issue_url"])
            if recovered.get("issue_number"):
                record.setdefault("issue_number", int(recovered["issue_number"]))
            record.setdefault("project_item_id", recovered["project_item_id"])
            record["project_field_values"] = dict(recovered.get("project_field_values") or {})

    summary: dict[str, Any] = {
        "dry_run": bool(args.dry_run),
        "selected_count": len(tasks),
        "statuses": sorted(statuses),
        "mapping_path": str(mapping_path),
        "created_issues": 0,
        "added_items": 0,
        "updated_field_values": 0,
        "skipped_field_values": [],
        "tasks": [],
    }
    if args.dry_run:
        for task in tasks:
            uid = str(task["task_uid"])
            existing = mapping.get("tasks", {}).get(uid, {})
            summary["tasks"].append(
                {
                    "task_uid": uid,
                    "title": task.get("title"),
                    "status": task.get("status"),
                    "priority": task.get("priority"),
                    "module": task.get("module") or "",
                    "owner_role": task.get("owner_role"),
                    "would_create_issue": not bool(existing.get("issue_url")),
                    "would_add_item": not bool(existing.get("project_item_id")),
                }
            )
        if args.json:
            print(json.dumps(summary, indent=2, sort_keys=True))
        else:
            print(f"github-project-sync: dry-run selected {len(tasks)} tasks")
        return 0

    project_id, fields = project_context(args.project_owner, args.project_number)
    if args.direct_api:
        token = github_token()
        mapping_lock = threading.Lock()

        def sync_one(task: OrderedDict[str, Any]) -> dict[str, Any]:
            uid = str(task["task_uid"])
            with mapping_lock:
                record = dict(mapping["tasks"].setdefault(uid, {}))
            issue_url = record.get("issue_url")
            issue_number = record.get("issue_number")
            project_item_id = record.get("project_item_id")
            created_issue = False
            added_item = False
            if not issue_url:
                created = create_issue_direct(token, args.repo, task)
                issue_url = created["issue_url"]
                issue_number = created["issue_number"]
                content_id = created["content_id"]
                created_issue = True
            else:
                content_id = str(record.get("content_id") or "")
            if not issue_number and issue_url:
                issue_number = issue_number_from_url(str(issue_url))
            if not project_item_id:
                if not content_id:
                    # Existing issue URLs recovered from older mappings need the node id.
                    owner, name = args.repo.split("/", 1)
                    content = run_json(
                        [
                            "gh",
                            "api",
                            f"repos/{owner}/{name}/issues/{issue_number}",
                        ]
                    )
                    content_id = str(content.get("node_id") or "")
                project_item_id = add_project_item_direct(token, project_id, content_id)
                added_item = True
            updated, skipped = update_fields_direct(
                token,
                project_id,
                str(project_item_id),
                task,
                fields,
                only_fields=only_fields,
                current_values=dict(record.get("project_field_values") or {}),
            )
            with mapping_lock:
                live_record = mapping["tasks"].setdefault(uid, {})
                live_record.update(
                    {
                        "task_uid": uid,
                        "issue_url": issue_url,
                        "issue_number": int(issue_number) if issue_number else None,
                        "project_item_id": project_item_id,
                        "title": task.get("title"),
                        "status": task.get("status"),
                        "priority": task.get("priority"),
                        "module": task.get("module") or "",
                        "owner_role": task.get("owner_role"),
                        "worktree_hint": task.get("worktree_hint") or "",
                        "execution_log_path": task.get("execution_log_path") or "",
                        "last_synced_at": datetime.now().astimezone().isoformat(timespec="seconds"),
                    }
                )
                live_record["project_field_values"] = dict(project_field_values(task))
                if content_id:
                    live_record["content_id"] = content_id
                persist_mapping(mapping_path, mapping)
            return {
                "task_uid": uid,
                "issue_url": issue_url,
                "project_item_id": project_item_id,
                "created_issue": created_issue,
                "added_item": added_item,
                "updated_field_values": updated,
                "skipped_field_values": skipped,
            }

        with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as executor:
            futures = {executor.submit(sync_one, task): task for task in tasks}
            for future in as_completed(futures):
                result = future.result()
                if result["created_issue"]:
                    summary["created_issues"] += 1
                if result["added_item"]:
                    summary["added_items"] += 1
                summary["updated_field_values"] += int(result["updated_field_values"])
                summary["skipped_field_values"].extend(
                    [f"{result['task_uid']}:{item}" for item in result["skipped_field_values"]]
                )
                summary["tasks"].append(
                    {
                        "task_uid": result["task_uid"],
                        "issue_url": result["issue_url"],
                        "project_item_id": result["project_item_id"],
                        "updated_field_values": result["updated_field_values"],
                        "skipped_field_values": result["skipped_field_values"],
                    }
                )
        mapping["project"] = {
            "owner": args.project_owner,
            "number": args.project_number,
            "id": project_id,
            "repo": args.repo,
        }
        persist_mapping(mapping_path, mapping)
        if args.json:
            print(json.dumps(summary, indent=2, sort_keys=True))
        else:
            print(
                "github-project-sync: "
                f"created_issues={summary['created_issues']} "
                f"added_items={summary['added_items']} "
                f"updated_field_values={summary['updated_field_values']}"
            )
        return 0
    for task in tasks:
        uid = str(task["task_uid"])
        record = mapping["tasks"].setdefault(uid, {})
        issue_url = record.get("issue_url")
        created_issue = False
        if not issue_url:
            issue_url = create_issue(args.repo, task)
            record["issue_url"] = issue_url
            created_issue = True
        issue_number = issue_number_from_url(str(issue_url))
        if issue_number:
            record["issue_number"] = issue_number
        if created_issue:
            summary["created_issues"] += 1
        item_id = record.get("project_item_id")
        if not item_id:
            item_payload = run_json(
                [
                    "gh",
                    "project",
                    "item-add",
                    str(args.project_number),
                    "--owner",
                    args.project_owner,
                    "--url",
                    issue_url,
                    "--format",
                    "json",
                ]
            )
            item_id = item_payload.get("id")
            if not item_id:
                die(f"github-project-sync: missing item id for {uid}")
            record["project_item_id"] = item_id
            summary["added_items"] += 1
        current_values = dict(record.get("project_field_values") or {})
        updated, skipped = update_fields(project_id, str(item_id), task, fields,
                                         only_fields=only_fields, current_values=current_values)
        summary["updated_field_values"] += updated
        summary["skipped_field_values"].extend([f"{uid}:{item}" for item in skipped])
        record.update(
            {
                "task_uid": uid,
                "title": task.get("title"),
                "status": task.get("status"),
                "priority": task.get("priority"),
                "module": task.get("module") or "",
                "owner_role": task.get("owner_role"),
                "worktree_hint": task.get("worktree_hint") or "",
                "execution_log_path": task.get("execution_log_path") or "",
                "last_synced_at": datetime.now().astimezone().isoformat(timespec="seconds"),
            }
        )
        record["project_field_values"] = dict(project_field_values(task))
        persist_mapping(mapping_path, mapping)
        summary["tasks"].append(
            {
                "task_uid": uid,
                "issue_url": issue_url,
                "project_item_id": item_id,
                "updated_field_values": updated,
                "skipped_field_values": skipped,
            }
        )
    mapping["project"] = {
        "owner": args.project_owner,
        "number": args.project_number,
        "id": project_id,
        "repo": args.repo,
    }
    persist_mapping(mapping_path, mapping)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(
            "github-project-sync: "
            f"created_issues={summary['created_issues']} "
            f"added_items={summary['added_items']} "
            f"updated_field_values={summary['updated_field_values']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
