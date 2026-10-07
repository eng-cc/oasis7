#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import re
import threading
import types
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
    "loop": "Loop",
    "change_id": "Change ID",
    "primary_package": "Primary Package",
}
SINGLE_SELECT_FIELDS = {
    "Status",
    "Owner Role",
    "Module",
    "PM Status",
    "Workflow Phase",
    "Priority",
    "Test Tier Required",
    "Loop",
}
TASK_UID_RE = re.compile(r"task_uid:\s*(task_[0-9a-f]{32})")
PRIMARY_PACKAGE_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*\Z")
ISSUE_URL_RE = re.compile(r"/issues/(\d+)(?:$|[?#])")
RECOVERY_BATCH_SIZE = 10
_PROJECT_CONTEXT_CACHE: dict[tuple[str, int], tuple[str, dict[str, dict[str, Any]]]] = {}
_GITHUB_API_MODULES: dict[pathlib.Path, types.ModuleType] = {}


def github_api_module() -> types.ModuleType:
    """Load the shared client beside this helper (or its trusted repo root)."""
    path = pathlib.Path(__file__).with_name("github_api.py").resolve()
    if not path.is_file():
        path = (pathlib.Path.cwd() / "scripts/pm/github_api.py").resolve()
    module = _GITHUB_API_MODULES.get(path)
    if module is not None:
        return module
    if not path.is_file():
        raise RuntimeError(f"shared GitHub API client is unavailable at {path}")
    module_name = "_oasis7_github_api_" + hashlib.sha256(str(path).encode("utf-8")).hexdigest()[:16]
    loaded = sys.modules.get(module_name)
    if isinstance(loaded, types.ModuleType):
        _GITHUB_API_MODULES[path] = loaded
        return loaded
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load shared GitHub API client at {path}")
    module = importlib.util.module_from_spec(spec)
    # Register before execution so standard-library decorators such as
    # dataclasses can resolve this dynamically loaded module by name.
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    _GITHUB_API_MODULES[path] = module
    return module


def github_api_client(token: Any = None) -> Any:
    """Keep caller-supplied credentials explicit; otherwise use gh precedence."""
    client_type = github_api_module().GitHubAPIClient
    if token is None:
        return client_type.from_gh()
    if hasattr(token, "graphql") and hasattr(token, "rest"):
        return token
    return client_type(token=token)


def is_github_api_error(error: BaseException) -> bool:
    return isinstance(error, github_api_module().APIError)


def github_api_error_exit(error: BaseException, script: str) -> int | None:
    """Render shared API failures consistently and preserve external-wait 75."""
    if not is_github_api_error(error):
        return None
    detail = error.as_dict()  # type: ignore[attr-defined]
    print(f"{script}: {json.dumps(detail, sort_keys=True)}", file=sys.stderr)
    return 75 if detail.get("status") == "external_wait" else 2


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def primary_package_value(task: OrderedDict[str, Any]) -> str | None:
    value = task.get("primary_package")
    if value in (None, ""):
        return None
    package = str(value).strip()
    if PRIMARY_PACKAGE_RE.fullmatch(package) is None:
        die("primary_package is not a valid declared Cargo package name")
    return package


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
        archived_mapping = record.get("github_project_mapping") or {}
        if archived_mapping.get("loop_binding") is not None:
            task["loop_binding"] = archived_mapping["loop_binding"]
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
    return durable_store.read_mapping(path, {"version": 1, "tasks": {}})


_store_path = pathlib.Path(__file__).with_name("workflow-durable-store.py")
if not _store_path.exists(): _store_path = pathlib.Path.cwd()/"scripts/pm/workflow-durable-store.py"
_store_spec = importlib.util.spec_from_file_location("workflow_durable_store", _store_path)
assert _store_spec and _store_spec.loader
durable_store = importlib.util.module_from_spec(_store_spec); _store_spec.loader.exec_module(durable_store)


def guard_missing_mapping_candidates(
    task_uids: list[str], repository: str, project_owner: str, project_number: int, *, skip_recover: bool,
) -> None:
    """Load the shared admission guard only when a candidate identity is absent."""
    guard_path = pathlib.Path(__file__).with_name("closed_duplicate_candidate_guard.py")
    spec = importlib.util.spec_from_file_location("closed_duplicate_candidate_guard_sync", guard_path)
    if spec is None or spec.loader is None:
        die(f"github-project-sync: shared candidate admission guard is unavailable at {guard_path}")
    guard = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(guard)
    except Exception as exc:
        die(f"github-project-sync: shared candidate admission guard could not be loaded: {exc}")
    try:
        guard.guard_missing_mapping_candidates(
            task_uids,
            repository,
            project_owner,
            project_number,
            skip_recover=skip_recover,
        )
    except guard.CandidateAdmissionError as exc:
        die(f"github-project-sync: {exc}")


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


def reject_unretired_closed_duplicate(root: pathlib.Path, mapping: dict[str, Any], task_uid: str) -> None:
    """Stop refresh before a poisoned cached candidate identity is consumed."""
    row = (mapping.get("tasks") or {}).get(task_uid)
    if not isinstance(row, dict) or str(row.get("status") or "") != "candidate":
        return
    repository = str((mapping.get("project") or {}).get("repo") or row.get("repository") or "")
    issue_number = int(row.get("issue_number") or 0)
    if not repository or issue_number <= 0:
        die(f"github-project-sync: candidate {task_uid} has incomplete Issue identity")
    issue = run_json(["gh", "api", f"repos/{repository}/issues/{issue_number}"])
    if str(issue.get("state") or "").lower() != "closed" or str(issue.get("state_reason") or "").lower() != "duplicate":
        return
    body = str(issue.get("body") or "")
    uid_fields = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", body)
    if uid_fields != [task_uid]:
        die(f"github-project-sync: closed duplicate Issue #{issue_number} has ambiguous task UID; cached identity was not used")
    comments = run_json(["gh", "api", f"repos/{repository}/issues/{issue_number}/comments?per_page=100", "--paginate", "--slurp"])
    pages = comments if isinstance(comments, list) else [comments]
    flattened = []
    for page in pages:
        if isinstance(page, list): flattened.extend(item for item in page if isinstance(item, dict))
        elif isinstance(page, dict): flattened.append(page)
        else: die(f"github-project-sync: closed duplicate Issue #{issue_number} comment pagination is malformed")
    marker = "<!-- oasis7.duplicate-candidate-disposition/v1 -->"
    marked = [item for item in flattened if str(item.get("body") or "").startswith(marker)]
    if len(marked) != 1 or not int(marked[0].get("id") or 0):
        die(f"github-project-sync: closed duplicate candidate {task_uid} needs one server disposition comment before retirement")
    launcher = pathlib.Path(__file__).with_name("retire-closed-duplicate-candidate.sh").resolve()
    command = (
        f"{launcher} --mapping-root {root} --task-uid {task_uid} "
        f"--disposition-comment-id {int(marked[0]['id'])} --preflight"
    )
    die(f"github-project-sync: closed duplicate candidate mapping must be reconciled before refresh; run: {command}")


def run_subprocess_with_retry(cmd: list[str], *, retries: int = 4) -> subprocess.CompletedProcess[str]:
    """Compatibility name; GitHub subprocesses now receive exactly one attempt."""
    # GitHub HTTP/GraphQL retries belong to GitHubAPIClient. Retrying a `gh`
    # subprocess here can wrap that policy and replay a write after an
    # ambiguous response, so every CLI request crosses this boundary once.
    del retries
    return subprocess.run(cmd, check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)


def github_token() -> Any:
    """Compatibility accessor returning one reusable shared GitHub API client."""
    return github_api_client()


def graphql_request(
    token: Any,
    query: str,
    variables: dict[str, Any] | None = None,
    *,
    operation: str = "project_sync_read",
    mutation: bool = False,
    context: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Use the shared strict client and return its already-decoded data object."""
    client = github_api_client(token)
    return client.graphql(
        query,
        variables,
        operation=operation,
        mutation=mutation,
        context=context or {"script": "github-project-sync.py"},
    )


def broad_rate_limit_guard(token: Any = None) -> dict[str, Any]:
    """Reuse a fresh persisted budget observation or perform one shared probe."""
    client = github_api_client(token)
    return client.rate_limit_guard(
        minimum_remaining=100,
        max_age_seconds=300,
        operation="project_broad_preflight",
        context={"script": "github-project-sync.py"},
    )


def create_issue_direct(token: Any, repo: str, task: OrderedDict[str, Any]) -> dict[str, Any]:
    payload = github_api_client(token).rest(
        "POST",
        f"repos/{repo}/issues",
        {
            "title": f"[PM] {task.get('title')}",
            "body": issue_body(task),
        },
        operation="project_sync_create_issue",
        mutation=True,
        context={"script": "github-project-sync.py", "task_uid": str(task.get("task_uid") or "")},
    )
    return {
        "issue_url": str(payload.get("html_url") or ""),
        "issue_number": int(payload.get("number") or 0),
        "content_id": str(payload.get("node_id") or ""),
    }


def add_project_item_direct(token: Any, project_id: str, content_id: str) -> str:
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
        operation="project_sync_add_item",
        mutation=True,
        context={"script": "github-project-sync.py"},
    )
    item_id = str(((data.get("addProjectV2ItemById") or {}).get("item") or {}).get("id") or "")
    if not item_id:
        raise RuntimeError("GitHub GraphQL addProjectV2ItemById returned no item id")
    return item_id


def update_fields_direct(
    token: Any,
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
        if current_values is not None and field_name in current_values and str(current_values[field_name]) == str(value):
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
    graphql_request(
        token,
        query,
        variables,
        operation="project_sync_update_fields",
        mutation=True,
        context={
            "script": "github-project-sync.py",
            "task_uid": str(task.get("task_uid") or ""),
        },
    )
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
    client: Any = None,
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
        variables = {
            f"q{index}": f"repo:{repo} {task_uid} in:body"
            for index, task_uid in enumerate(batch)
        }
        data = graphql_request(
            client,
            query,
            variables,
            operation="project_sync_selected_recovery",
            context={"script": "github-project-sync.py"},
        )
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
    if internal in {"task_done", "main_sync", "closed_without_merge", "post_merge_done"}:
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
        f"- workflow_phase: `{task.get('workflow_phase') or workflow_phase_for(str(task.get('status') or ''))}`",
        f"- priority: `{task.get('priority')}`",
        f"- worktree_hint: `{task.get('worktree_hint') or ''}`",
    ]
    package = primary_package_value(task)
    if package is not None:
        lines.append(f"- primary_package: `{package}`")
    source_refs = task.get("source_refs") or []
    if source_refs:
        lines.append("")
        lines.append("Source refs:")
        for ref in source_refs:
            lines.append(f"- `{ref}`")
    if task.get("loop_binding") is not None:
        encoded = base64.urlsafe_b64encode(json.dumps(task["loop_binding"], sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).decode().rstrip("=")
        lines.append(f"- loop_binding_b64: `{encoded}`")
    acceptance = task.get("acceptance") or []
    if acceptance:
        lines.append("")
        lines.append("Acceptance:")
        for item in acceptance:
            lines.append(f"- {item}")
    return "\n".join(lines) + "\n"


def project_field_values(task: OrderedDict[str, Any]) -> dict[str, str]:
    status = str(task.get("status") or "")
    result = {
        "Status": "In Progress" if status == "done" and task.get("workflow_phase") not in {"closed_without_merge", "post_merge_done"} else project_status_for(status),
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
    package = primary_package_value(task)
    if package is not None:
        result["Primary Package"] = package
    if task.get("loop_binding") is not None:
        binding = task["loop_binding"]
        result.update({"Loop": str(binding.get("loop") or ""), "Change ID": str(binding.get("change_id") or "")})
    return result


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
        if current_values is not None and field_name in current_values and str(current_values[field_name]) == str(value):
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


def read_project_item_field_values(project_id: str, item_id: str, client: Any = None) -> dict[str, str]:
    """Read back the authoritative Project fields for one item after mutation."""
    query = """
    query($item: ID!) {
      node(id: $item) {
        ... on ProjectV2Item {
          project { id }
          fieldValues(first: 100) { nodes {
            ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
          } }
        }
      }
    }
    """
    payload = graphql_request(
        client,
        query,
        {"item": item_id},
        operation="project_sync_selected_item_readback",
        context={"script": "github-project-sync.py"},
    )
    node = payload.get("node") or {}
    if str(((node.get("project") or {}).get("id") or "")) != project_id:
        raise RuntimeError("Project item belongs to a different Project")
    values: dict[str, str] = {}
    for value in ((node.get("fieldValues") or {}).get("nodes") or []):
        field_name = str(((value.get("field") or {}).get("name") or ""))
        if field_name:
            values[field_name] = str(value.get("name") or value.get("text") or "")
    return values


def read_live_issue_project_item(repo: str, issue_number: int, project_id: str,
                                 project_number: int, *,
                                 require_open: bool = True) -> dict[str, Any]:
    """Read one Issue's complete membership and field projection in a Project.

    The Issue is selected by repository/number, then every Project item link
    and every field-value page is read from GraphQL. This proves the returned
    item belongs to that exact live Issue and Project, and reports only the
    authenticated viewer's server-computed update permission.
    """
    if (not isinstance(repo, str) or re.fullmatch(r"[^/\s]+/[^/\s]+", repo) is None
            or type(issue_number) is not int or issue_number < 1
            or not isinstance(project_id, str) or not project_id
            or type(project_number) is not int or project_number < 1):
        raise ValueError("live Issue/Project identity is malformed")
    owner, name = repo.split("/", 1)
    token = github_token()
    membership_query = """
    query($owner: String!, $name: String!, $number: Int!, $after: String) {
      repository(owner: $owner, name: $name) {
        issue(number: $number) {
          id number url state body
          projectItems(first: 100, after: $after) {
            nodes { id isArchived project {
              id number viewerCanUpdate
              owner { ... on Organization { login } ... on User { login } }
            } }
            pageInfo { hasNextPage endCursor }
          }
        }
      }
    }
    """
    issue = None
    memberships = []
    after = None
    seen_cursors = set()
    for _ in range(100):
        variables = {"owner": owner, "name": name, "number": issue_number, "after": after}
        payload = graphql_request(
            token,
            membership_query,
            variables,
            operation="project_sync_live_issue_memberships",
            context={"script": "github-project-sync.py", "issue_number": issue_number},
        )
        repository = payload.get("repository") if isinstance(payload, dict) else None
        page_issue = repository.get("issue") if isinstance(repository, dict) else None
        if not isinstance(page_issue, dict):
            raise RuntimeError("live Task Issue was not readable from its repository")
        identity = {key: page_issue.get(key) for key in ("id", "number", "url", "state", "body")}
        if issue is None:
            issue = identity
        elif identity != issue:
            raise RuntimeError("live Task Issue changed during Project membership pagination")
        connection = page_issue.get("projectItems")
        nodes = connection.get("nodes") if isinstance(connection, dict) else None
        page_info = connection.get("pageInfo") if isinstance(connection, dict) else None
        if (not isinstance(nodes, list) or not isinstance(page_info, dict)
                or type(page_info.get("hasNextPage")) is not bool):
            raise RuntimeError("live Project membership pagination is incomplete")
        if any(not isinstance(item, dict) or not isinstance(item.get("id"), str)
               or not item.get("id") or not isinstance(item.get("project"), dict)
               for item in nodes):
            raise RuntimeError("live Project membership entry is malformed")
        memberships.extend(nodes)
        if not page_info["hasNextPage"]:
            break
        cursor = page_info.get("endCursor")
        if not isinstance(cursor, str) or not cursor or cursor in seen_cursors:
            raise RuntimeError("live Project membership cursor is missing or repeated")
        seen_cursors.add(cursor)
        after = cursor
    else:
        raise RuntimeError("live Project membership pagination limit exhausted")
    if type(require_open) is not bool:
        raise ValueError("live Issue state requirement is malformed")
    if (not isinstance(issue, dict) or not isinstance(issue.get("id"), str)
            or not issue["id"]
            or issue.get("number") != issue_number
            or not isinstance(issue.get("body"), str)
            or str(issue.get("state") or "").upper() not in {"OPEN", "CLOSED"}
            or (require_open and str(issue.get("state") or "").upper() != "OPEN")):
        raise RuntimeError("live Task Issue identity/state is invalid")
    matches = [item for item in memberships
               if item["project"].get("id") == project_id]
    if len(matches) != 1:
        raise RuntimeError("live Task Issue does not have one unique item in the canonical Project")
    selected = matches[0]
    selected_project = selected["project"]
    selected_owner = selected_project.get("owner")
    owner_login = selected_owner.get("login") if isinstance(selected_owner, dict) else None
    if (selected_project.get("number") != project_number
            or selected_project.get("viewerCanUpdate") is not True
            or not isinstance(owner_login, str) or not owner_login
            or selected.get("isArchived") is not False):
        raise RuntimeError("live Project identity, active membership, or viewer update permission is invalid")

    fields_query = """
    query($item: ID!, $after: String) {
      node(id: $item) {
        ... on ProjectV2Item {
          id isArchived project {
            id number viewerCanUpdate
            owner { ... on Organization { login } ... on User { login } }
          }
          fieldValues(first: 100, after: $after) {
            nodes {
              ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
              ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
              ... on ProjectV2ItemFieldDateValue { date field { ... on ProjectV2FieldCommon { name } } }
            }
            pageInfo { hasNextPage endCursor }
          }
        }
      }
    }
    """
    item_id = selected["id"]
    field_values: dict[str, str] = {}
    after = None
    seen_cursors = set()
    item_identity = None
    for _ in range(100):
        payload = graphql_request(
            token,
            fields_query,
            {"item": item_id, "after": after},
            operation="project_sync_live_item_fields",
            context={"script": "github-project-sync.py", "issue_number": issue_number},
        )
        item = payload.get("node") if isinstance(payload, dict) else None
        if not isinstance(item, dict):
            raise RuntimeError("live Project item field readback is unavailable")
        current_identity = {
            "id": item.get("id"), "isArchived": item.get("isArchived"),
            "project": item.get("project"),
        }
        if item_identity is None:
            item_identity = current_identity
        elif current_identity != item_identity:
            raise RuntimeError("live Project item changed during field pagination")
        connection = item.get("fieldValues")
        nodes = connection.get("nodes") if isinstance(connection, dict) else None
        page_info = connection.get("pageInfo") if isinstance(connection, dict) else None
        if (not isinstance(nodes, list) or not isinstance(page_info, dict)
                or type(page_info.get("hasNextPage")) is not bool):
            raise RuntimeError("live Project field pagination is incomplete")
        for value in nodes:
            if not isinstance(value, dict):
                raise RuntimeError("live Project field value is malformed")
            field = value.get("field")
            field_name = field.get("name") if isinstance(field, dict) else None
            if not isinstance(field_name, str) or not field_name:
                continue
            if field_name in field_values:
                raise RuntimeError("live Project contains duplicate field values")
            raw = value.get("name")
            if raw is None:
                raw = value.get("text")
            if raw is None:
                raw = value.get("date")
            if raw is not None and not isinstance(raw, str):
                raise RuntimeError("live Project field value is malformed")
            field_values[field_name] = raw or ""
        if not page_info["hasNextPage"]:
            break
        cursor = page_info.get("endCursor")
        if not isinstance(cursor, str) or not cursor or cursor in seen_cursors:
            raise RuntimeError("live Project field cursor is missing or repeated")
        seen_cursors.add(cursor)
        after = cursor
    else:
        raise RuntimeError("live Project field pagination limit exhausted")
    project_identity = item_identity.get("project") if isinstance(item_identity, dict) else None
    item_owner = project_identity.get("owner") if isinstance(project_identity, dict) else None
    if (not isinstance(item_identity, dict) or item_identity.get("id") != item_id
            or item_identity.get("isArchived") is not False
            or not isinstance(project_identity, dict)
            or project_identity.get("id") != project_id
            or project_identity.get("number") != project_number
            or project_identity.get("viewerCanUpdate") is not True
            or not isinstance(item_owner, dict)
            or item_owner.get("login") != owner_login):
        raise RuntimeError("live Project item identity or permission changed during field readback")
    return {
        "complete": True, "issue": issue,
        "project": {"id": project_id, "number": project_number,
                    "owner": owner_login, "viewer_can_update": True},
        "item": {"id": item_id, "is_archived": False, "field_values": field_values},
    }


def confirmed_project_field_values(
    current_values: dict[str, str],
    task: OrderedDict[str, Any],
    skipped: list[str],
    only_fields: set[str] | None = None,
) -> dict[str, str]:
    """Merge desired values that were confirmed unchanged or successfully written."""
    confirmed = dict(current_values)
    skipped_by_field = {item.split(":", 1)[0]: item for item in skipped}
    for field_name, value in project_field_values(task).items():
        if only_fields is not None and field_name not in only_fields:
            continue
        disposition = skipped_by_field.get(field_name)
        if disposition is None or disposition.endswith(":unchanged"):
            confirmed[field_name] = value
    return confirmed


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
    api_client = None
    if args.global_maintenance:
        api_client = github_api_client()
        budget = broad_rate_limit_guard(api_client)
        if budget["status"] != "ok":
            print(json.dumps(budget, indent=2, sort_keys=True))
            return 75 if budget.get("status") == "external_wait" else 2
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
    if args.task_uid and durable_store.retired_task(mapping, args.task_uid) is not None:
        result = {"status": "retired", "task_uid": args.task_uid, "mapping_path": str(mapping_path)}
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"github-project-sync: {args.task_uid} is retired by a validated duplicate-candidate tombstone")
        return 0
    if not args.task_uid:
        for task in tasks:
            task_uid = str(task.get("task_uid") or "")
            try:
                retired = durable_store.retired_task(mapping, task_uid)
            except ValueError as exc:
                die(f"github-project-sync: retirement ledger is invalid: {exc}")
            if retired is not None:
                die(
                    "github-project-sync: global maintenance selected a retired Task UID from .pm/tasks; "
                    f"remove or reconcile the stale source entry before recovery or apply: {task_uid}"
                )
    mapping_tasks = mapping.get("tasks")
    if not isinstance(mapping_tasks, dict):
        die("github-project-sync: task mapping is malformed")
    missing_candidate_uids = []
    for task in tasks:
        if str(task.get("status") or "") != "candidate":
            continue
        task_uid = str(task.get("task_uid") or "")
        record = mapping_tasks.get(task_uid)
        has_complete_identity = isinstance(record, dict) and all(
            record.get(key) not in (None, "")
            for key in ("issue_url", "issue_number", "project_item_id")
        )
        if not has_complete_identity:
            missing_candidate_uids.append(task_uid)
    if missing_candidate_uids:
        guard_missing_mapping_candidates(
            missing_candidate_uids,
            args.repo,
            args.project_owner,
            args.project_number,
            skip_recover=args.skip_recover,
        )
    mapping.setdefault("tasks", {})
    for task in tasks:
        reject_unretired_closed_duplicate(root, mapping, str(task.get("task_uid") or ""))
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
        if selected_uids:
            api_client = api_client or github_api_client()
        current_project_values: dict[str, dict[str, str]] = {}
        for uid, recovered in recover_project_mapping_for_task_uids(
            args.project_owner,
            args.project_number,
            args.repo,
            selected_uids,
            project_id,
            client=api_client,
        ).items():
            record = mapping["tasks"].setdefault(uid, {})
            record.setdefault("issue_url", recovered["issue_url"])
            if recovered.get("issue_number"):
                record.setdefault("issue_number", int(recovered["issue_number"]))
            record.setdefault("project_item_id", recovered["project_item_id"])
            live_values = recovered.get("project_field_values")
            if isinstance(live_values, dict):
                current_project_values[uid] = dict(live_values)
                record["project_field_values"] = dict(live_values)
    else:
        # The explicit --skip-recover path has no current Project observation;
        # old mapping values must not be used as an unchanged proof.
        current_project_values = {}

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
        api_client = api_client or github_api_client()
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
                created = create_issue_direct(api_client, args.repo, task)
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
                    content = api_client.rest(
                        "GET",
                        f"repos/{owner}/{name}/issues/{issue_number}",
                        operation="project_sync_resolve_issue_node",
                        context={"script": "github-project-sync.py", "task_uid": uid},
                    )
                    content_id = str(content.get("node_id") or "")
                project_item_id = add_project_item_direct(api_client, project_id, content_id)
                added_item = True
            updated, skipped = update_fields_direct(
                api_client,
                project_id,
                str(project_item_id),
                task,
                fields,
                only_fields=only_fields,
                current_values=current_project_values.get(uid, {}),
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
                        **({"primary_package": task["primary_package"]}
                           if task.get("primary_package") not in (None, "") else {}),
                        "execution_log_path": task.get("execution_log_path") or "",
                        "last_synced_at": datetime.now().astimezone().isoformat(timespec="seconds"),
                    }
                )
                live_record["project_field_values"] = confirmed_project_field_values(
                    current_project_values.get(uid, {}), task, skipped, only_fields
                )
                if task.get("loop_binding") is not None:
                    live_record["loop_binding"] = task["loop_binding"]
                if primary_package_value(task) is None:
                    live_record.pop("primary_package", None)
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
        current_values = current_project_values.get(uid, {})
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
                **({"primary_package": task["primary_package"]}
                   if task.get("primary_package") not in (None, "") else {}),
                "execution_log_path": task.get("execution_log_path") or "",
                "last_synced_at": datetime.now().astimezone().isoformat(timespec="seconds"),
            }
        )
        record["project_field_values"] = confirmed_project_field_values(
            current_values, task, skipped, only_fields
        )
        if task.get("loop_binding") is not None:
            record["loop_binding"] = task["loop_binding"]
        if primary_package_value(task) is None:
            record.pop("primary_package", None)
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
    try:
        code = main()
    except Exception as exc:
        api_code = github_api_error_exit(exc, "github-project-sync")
        if api_code is None:
            raise
        raise SystemExit(api_code)
    raise SystemExit(code)
