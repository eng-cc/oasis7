#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
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
        owner {
          ... on Organization {
            login
          }
          ... on User {
            login
          }
        }
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
        pageInfo {
          hasNextPage
          endCursor
        }
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
        if line.strip() in {"Source refs:", "Doc refs:", "Related PRD:", "Acceptance:"}:
            break
        if not line.strip():
            if values:
                break
            continue
        match = re.match(r"^-\s+(?:\[[ xX]\]\s*)?(.*\S)\s*$", line)
        if not match:
            break
        values.append(match.group(1).strip())
    return values


TRACEABILITY_SECTIONS = ("Source refs:", "Doc refs:", "Related PRD:", "Acceptance:")


def _normalized_issue_section(body: str, header: str, *, references: bool) -> list[str] | None:
    lines = body.replace("\r\n", "\n").splitlines()
    try:
        start = next(index for index, line in enumerate(lines) if line.strip() == header) + 1
    except StopIteration:
        return None
    values: list[str] = []
    for line in lines[start:]:
        stripped = line.strip()
        if stripped in TRACEABILITY_SECTIONS:
            break
        if not stripped:
            if values:
                break
            continue
        match = (
            re.fullmatch(r"- `([^`]+)`", line)
            if references
            else re.fullmatch(r"-\s+(?:\[[ xX]\]\s*)?(.*\S)\s*", line)
        )
        if not match:
            break
        values.append(match.group(1).strip())
    return values


def normalized_issue_traceability(body: str) -> dict[str, Any]:
    """Extract Issue-authoritative trace fields for bounded audit comparison."""
    body = body.replace("\r\n", "\n")
    fields: dict[str, Any] = {}
    binding_matches = re.findall(r"^- loop_binding_b64: `([^`]+)`$", body, re.MULTILINE)
    if "loop_binding_b64:" in body:
        if len(binding_matches) != 1:
            fields["trace_projection_error"] = "trace-projection-loss: loop binding is malformed or duplicated"
            return fields
        try:
            encoded = binding_matches[0]
            binding = json.loads(base64.b64decode(
                encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True,
            ).decode("utf-8"))
            if not isinstance(binding, dict):
                raise ValueError("loop binding must be an object")
            fields["loop_binding"] = binding
        except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
            fields["trace_projection_error"] = f"trace-projection-loss: malformed loop binding: {exc}"
            return fields
    context_matches = re.findall(r"^- traceability_context_b64: `([^`]+)`$", body, re.MULTILINE)
    if "traceability_context_b64:" in body:
        if len(context_matches) != 1:
            fields["trace_projection_error"] = "trace-projection-loss: traceability context is malformed or duplicated"
            return fields
        try:
            encoded = context_matches[0]
            context = json.loads(base64.b64decode(
                encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True,
            ).decode("utf-8"))
            if not isinstance(context, dict):
                raise ValueError("traceability context must be an object")
            allowed = {
                "traceability_mode", "coordination_ref", "traceability_record",
                "coordination_record", "traceability_candidate", "aggregate_candidate",
            }
            if set(context) - allowed:
                raise ValueError("traceability context contains unknown fields")
            fields.update(context)
        except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
            fields["trace_projection_error"] = f"trace-projection-loss: malformed traceability context: {exc}"
            return fields
    for key in ("workflow_phase", "completion_mode", "non_pr_completion_evidence_sha256"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    evidence_match = re.search(r"^- non_pr_completion_evidence_b64: `([^`]+)`$", body, re.MULTILINE)
    if evidence_match:
        try:
            encoded = evidence_match.group(1)
            fields["non_pr_completion_evidence"] = base64.b64decode(
                encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True
            ).decode("utf-8")
        except (ValueError, UnicodeDecodeError):
            fields["trace_projection_error"] = (
                "trace-projection-loss: malformed non-PR completion evidence encoding"
            )
    for key, header in (("source_refs", "Source refs:"), ("doc_refs", "Doc refs:"), ("related_prd", "Related PRD:")):
        values = _normalized_issue_section(body, header, references=True)
        if values is not None:
            fields[key] = values
    return fields


def canonical_non_pr_evidence_digest(value: object) -> str:
    """Return the digest used by the canonical non-PR evidence file."""
    return hashlib.sha256((str(value) + "\n").encode("utf-8")).hexdigest()


def consumption_summary(task: dict[str, Any], blockers: list[str]) -> dict[str, Any]:
    """Build a disposable read-only view from the selected authoritative record."""
    binding = task.get("loop_binding") if isinstance(task.get("loop_binding"), dict) else {}
    record = task.get("traceability_record")
    if not isinstance(record, dict):
        record = task.get("coordination_record") if isinstance(task.get("coordination_record"), dict) else {}
    candidate = task.get("aggregate_candidate")
    if not isinstance(candidate, dict):
        candidate = task.get("traceability_candidate") if isinstance(task.get("traceability_candidate"), dict) else {}
    inputs = binding.get("input_contracts") if isinstance(binding.get("input_contracts"), list) else []
    obligations = record.get("required_obligations")
    if not isinstance(obligations, list):
        obligations = binding.get("delivery_obligations") if isinstance(binding.get("delivery_obligations"), list) else []
    verified_results = candidate.get("verified_results")
    if not isinstance(verified_results, list):
        verified_results = candidate.get("results") if isinstance(candidate.get("results"), list) else []
    affected = record.get("affected_consumers")
    if not isinstance(affected, list):
        affected = record.get("reverse_consumers") if isinstance(record.get("reverse_consumers"), list) else []
    unread: list[str] = []
    if not inputs:
        unread.append("input_contracts")
    if not obligations:
        unread.append("obligations")
    if not affected:
        unread.append("affected_consumers")
    return {
        "inputs": inputs,
        "obligations": obligations,
        "verified_results": verified_results,
        "blockers": list(blockers),
        "affected_consumers": affected,
        "unread_scope": unread,
        "derived": True,
    }


TRACEABILITY_CONTEXT_KEYS = (
    "loop_binding", "traceability_mode", "coordination_ref", "traceability_record",
    "coordination_record", "traceability_candidate", "aggregate_candidate",
)


def traceability_projection_errors(uid: str, cached: dict[str, Any], live: dict[str, Any]) -> list[str]:
    """Reject deletion or drift of Issue-authoritative traceability fields."""
    errors: list[str] = []
    for key in TRACEABILITY_CONTEXT_KEYS:
        if cached.get(key) is None:
            continue
        if key not in live:
            errors.append(
                f"{uid}: live {key} projection is missing; refresh explicitly from authoritative GitHub issue"
            )
        elif cached.get(key) != live.get(key):
            errors.append(
                f"{uid}: cached {key} drift; refresh explicitly from authoritative GitHub issue"
            )
    return errors


def authoritative_selected_traceability(live: dict[str, Any]) -> dict[str, Any]:
    """Project selected-task traceability exclusively from the bounded live Issue read."""
    return {key: live[key] for key in TRACEABILITY_CONTEXT_KEYS if key in live}


def read_canonical_non_pr_evidence(
    task_uid: str, record: dict[str, Any]
) -> tuple[bytes | None, str | None]:
    """Read the identity-bound non-PR evidence file without following unsafe paths."""
    recorded_value = record.get("non_pr_completion_evidence_file")
    if recorded_value in (None, ""):
        return None, "canonical non_pr_completion_evidence file is missing"
    if not isinstance(recorded_value, str):
        return None, "canonical non_pr_completion_evidence path is not canonical"
    worktree_value = record.get("canonical_worktree")
    if worktree_value in (None, "") or not isinstance(worktree_value, str):
        return None, "canonical non_pr_completion_evidence path has no canonical worktree"
    try:
        canonical_worktree_path = pathlib.Path(worktree_value).expanduser()
        if not canonical_worktree_path.is_absolute():
            return None, "canonical non_pr_completion_evidence path has no absolute canonical worktree"
        canonical_worktree = canonical_worktree_path.resolve(strict=True)
        recorded_path = pathlib.Path(recorded_value).expanduser()
        if not recorded_path.is_absolute():
            return None, "canonical non_pr_completion_evidence path is not canonical"
        expected_path = canonical_worktree / ".pm" / "scratch" / task_uid / "non-pr-completion-evidence.txt"
        if recorded_path.resolve(strict=False) != expected_path:
            return None, "canonical non_pr_completion_evidence path is not canonical"
        if recorded_path.is_symlink():
            return None, "canonical non_pr_completion_evidence path is a symlink"
        if not recorded_path.is_file():
            return None, "canonical non_pr_completion_evidence file is unavailable"
        return recorded_path.read_bytes(), None
    except (OSError, RuntimeError, ValueError, UnicodeError) as exc:
        return None, f"canonical non_pr_completion_evidence file cannot be read: {exc}"


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
    return durable_store.read_mapping(path, {"version": 1, "tasks": {}})


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
    project = node.get("project")
    project = project if isinstance(project, dict) else {}
    owner = project.get("owner")
    owner = owner if isinstance(owner, dict) else {}
    field_values = node.get("fieldValues")
    page_info = field_values.get("pageInfo") if isinstance(field_values, dict) else None
    has_next_page = page_info.get("hasNextPage") if isinstance(page_info, dict) else None
    field_value_nodes = field_values.get("nodes") if isinstance(field_values, dict) else []
    item: dict[str, Any] = {
        "id": node.get("id") or "",
        "content": node.get("content") or {},
        "_project_id": project.get("id") or "",
        "_project_number": project.get("number") or "",
        "_project_owner": owner.get("login") or "",
        # Preserve an absent or malformed pageInfo as unknown.  Treating it as
        # false would turn an incomplete GraphQL read into terminal evidence.
        "_field_values_has_next_page": has_next_page if isinstance(has_next_page, bool) else None,
    }
    for field_value_node in field_value_nodes or []:
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
    if status == "committed" and internal_phase == "verification":
        workflow_phase = "verification"
    project_status = {
        "candidate": "Todo",
        "committed": "In Progress",
        "blocked": "Blocked",
        "ready": "Ready / PR",
        "pr_watch": "PR Watch",
        "done": "Done",
        "deferred": "Done",
    }.get(status, "Todo")
    if status == "done" and internal_phase not in {"closed_without_merge", "post_merge_done"}:
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
    task_uid = getattr(args, "task_uid", None)
    if task_uid:
        retired = durable_store.retired_task(mapping, task_uid)
        if retired is not None:
            result = {
                "status": "retired",
                "project_owner": str((mapping.get("project") or {}).get("owner") or ""),
                "project_number": (mapping.get("project") or {}).get("number"),
                "mapping_path": str(mapping_path),
                "task_uid": task_uid,
                "selected_count": 0,
                "project_item_count": 0,
                "selected_statuses": sorted(statuses),
                "status_counts": {},
                "errors": [],
                "warnings": [],
                "selected_task": {"task_uid": task_uid, "target": "retired", "workflow_phase": ""},
            }
            if args.json:
                print(json.dumps(result, indent=2, sort_keys=True))
            else:
                print(f"github-project-workflow audit: task UID {task_uid} is retired by a validated tombstone")
            return 0
    tasks = load_tasks(root, statuses, mapping)
    if task_uid:
        tasks = {uid: task for uid, task in tasks.items() if uid == args.task_uid}
    mapped_tasks = mapping.get("tasks", {})
    retired_files = retired_task_files(root)
    canonical_project = mapping.get("project") or {}
    canonical_project_owner = str(canonical_project.get("owner") or "")

    errors: list[str] = []
    warnings: list[str] = []
    live_traceability_by_task: dict[str, dict[str, Any]] = {}
    if not canonical_project_owner:
        errors.append("canonical mapping missing project owner")
    elif str(args.project_owner or "") != canonical_project_owner:
        errors.append(
            "configured Project owner does not match canonical mapping owner: "
            f"{args.project_owner!r} != {canonical_project_owner!r}"
        )
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
        if not args.full_list:
            if str(item.get("_project_owner") or "") != canonical_project_owner:
                errors.append(f"{uid}: live item project_owner does not match canonical mapping")
            if item.get("_field_values_has_next_page") is not False:
                errors.append(
                    f"{uid}: live Project item fieldValues pagination is incomplete or unknown"
                )
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
        live_traceability = normalized_issue_traceability(body)
        live_traceability_by_task[uid] = live_traceability
        trace_projection_error = live_traceability.get("trace_projection_error")
        if trace_projection_error:
            errors.append(f"{uid}: {trace_projection_error}")
            continue
        for key in ("doc_refs", "related_prd"):
            if key not in live_traceability:
                continue
            cached_values = sorted({str(value) for value in (record.get(key) or [])})
            live_values = sorted({str(value) for value in (live_traceability.get(key) or [])})
            if cached_values != live_values:
                errors.append(
                    f"{uid}: cached {key} drift; refresh explicitly from authoritative GitHub issue"
                )
        for key in ("workflow_phase", "completion_mode"):
            if key not in live_traceability:
                if key == "workflow_phase" and str(record.get(key) or ""):
                    errors.append(
                        f"{uid}: live workflow_phase projection is missing; refresh explicitly from authoritative GitHub issue"
                    )
                continue
            cached_value = str(record.get(key) or "")
            live_value = str(live_traceability.get(key) or "")
            if cached_value != live_value:
                errors.append(
                    f"{uid}: cached {key} drift; refresh explicitly from authoritative GitHub issue"
                )
        errors.extend(traceability_projection_errors(uid, record, live_traceability))
        live_evidence_present = "non_pr_completion_evidence" in live_traceability
        live_digest_present = (
            "non_pr_completion_evidence_sha256" in live_traceability
            and bool(str(live_traceability.get("non_pr_completion_evidence_sha256") or ""))
        )
        cached_evidence = record.get("non_pr_completion_evidence")
        cached_digest = record.get("non_pr_completion_evidence_sha256")
        cached_evidence_present = cached_evidence not in (None, "")
        cached_digest_present = bool(str(cached_digest or ""))
        canonical_file_claimed = record.get("non_pr_completion_evidence_file") not in (None, "")
        non_pr_mode_claimed = (
            str(live_traceability.get("completion_mode") or "") == "non_pr_task"
            or str(record.get("completion_mode") or "") == "non_pr_task"
        )
        evidence_claimed = (
            non_pr_mode_claimed
            or live_evidence_present
            or live_digest_present
            or cached_evidence_present
            or cached_digest_present
            or canonical_file_claimed
        )
        if evidence_claimed:
            if not live_evidence_present:
                errors.append(
                    f"{uid}: live non_pr_completion_evidence projection is missing; refresh explicitly from authoritative GitHub issue"
                )
            if not live_digest_present:
                errors.append(
                    f"{uid}: live non_pr_completion_evidence_sha256 projection is missing; refresh explicitly from authoritative GitHub issue"
                )
            if not cached_evidence_present:
                errors.append(
                    f"{uid}: cached non_pr_completion_evidence is missing; refresh explicitly from authoritative GitHub issue"
                )
            if not cached_digest_present:
                errors.append(
                    f"{uid}: cached non_pr_completion_evidence_sha256 is missing; refresh explicitly from authoritative GitHub issue"
                )
            if live_evidence_present and cached_evidence_present:
                live_value = str(live_traceability.get("non_pr_completion_evidence") or "")
                cached_value = str(cached_evidence)
                if live_value != cached_value:
                    errors.append(
                        f"{uid}: cached non_pr_completion_evidence drift; refresh explicitly from authoritative GitHub issue"
                    )
            if live_digest_present and cached_digest_present:
                live_value = str(live_traceability.get("non_pr_completion_evidence_sha256") or "")
                cached_value = str(cached_digest)
                if live_value != cached_value:
                    errors.append(
                        f"{uid}: cached non_pr_completion_evidence_sha256 drift; refresh explicitly from authoritative GitHub issue"
                    )
            if live_evidence_present and live_digest_present:
                live_value = str(live_traceability.get("non_pr_completion_evidence") or "")
                live_digest = str(live_traceability.get("non_pr_completion_evidence_sha256") or "")
                if live_digest != canonical_non_pr_evidence_digest(live_value):
                    errors.append(
                        f"{uid}: live non_pr_completion_evidence digest binding is invalid; refresh explicitly from authoritative GitHub issue"
                    )
            if cached_evidence_present and cached_digest_present:
                cached_value = str(cached_evidence)
                cached_digest_value = str(cached_digest)
                if cached_digest_value != canonical_non_pr_evidence_digest(cached_value):
                    errors.append(
                        f"{uid}: cached non_pr_completion_evidence digest binding is invalid; refresh explicitly from authoritative GitHub issue"
                    )
            if canonical_file_claimed:
                canonical_bytes, canonical_error = read_canonical_non_pr_evidence(uid, record)
                if canonical_error:
                    errors.append(f"{uid}: {canonical_error}")
                else:
                    canonical_digest = hashlib.sha256(canonical_bytes or b"").hexdigest()
                    if live_digest_present and canonical_digest != str(live_traceability.get("non_pr_completion_evidence_sha256") or ""):
                        errors.append(
                            f"{uid}: canonical non_pr_completion_evidence digest disagrees with live Issue authority"
                        )
                    if cached_digest_present and canonical_digest != str(cached_digest):
                        errors.append(
                            f"{uid}: canonical non_pr_completion_evidence digest disagrees with cache authority"
                        )
                    if live_evidence_present:
                        expected_live_bytes = (str(live_traceability.get("non_pr_completion_evidence") or "") + "\n").encode("utf-8")
                        if canonical_bytes != expected_live_bytes:
                            errors.append(
                                f"{uid}: canonical non_pr_completion_evidence content disagrees with live Issue authority"
                            )
                    if cached_evidence_present:
                        expected_cached_bytes = (str(cached_evidence) + "\n").encode("utf-8")
                        if canonical_bytes != expected_cached_bytes:
                            errors.append(
                                f"{uid}: canonical non_pr_completion_evidence content disagrees with cache authority"
                            )
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

    selected_task = None
    if task_uid and task_uid in tasks:
        task = tasks[task_uid]
        live_task = live_traceability_by_task.get(task_uid, {})
        selected_task = {
            "task_uid": task_uid,
            "target": str(task.get("status") or ""),
            "workflow_phase": str(task.get("workflow_phase") or ""),
        }
        # Preserve selected-task traceability metadata in the bounded audit
        # readback. These values are producer-owned mapping state; closeout
        # still binds them against the local record and the pinned helper.
        for key in (
            "owner_role",
            "change_id",
            "completion_mode",
            "doc_refs",
            "related_prd",
            "non_pr_completion_evidence_sha256",
        ):
            if key in task and task[key] is not None:
                selected_task[key] = task[key]
        selected_task.update(authoritative_selected_traceability(live_task))
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
    if selected_task is not None:
        selected_blockers = [error for error in errors if error.startswith(f"{task_uid}:")]
        selected_task["consumption_summary"] = consumption_summary(selected_task, selected_blockers)
        result["selected_task"] = selected_task
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
