#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import uuid
from collections import OrderedDict
from datetime import datetime
from typing import Any


ALL_STATUSES = ("candidate", "committed", "blocked", "ready", "pr_watch", "done", "deferred")
DEFAULT_REPO = "eng-cc/oasis7"
DEFAULT_PROJECT_OWNER = "eng-cc"
DEFAULT_PROJECT_NUMBER = 1
ALLOWED_PHASE_TRANSITIONS = {"task_done": {"main_sync"}, "main_sync": {"main_sync"}}
RECEIPT_SCHEMAS = {"main_sync": ("oasis7_main_sync", "post-merge-main-sync")}
NON_MERGE_INTENT_FIELD = "closed_without_merge_intent"
NON_MERGE_RECEIPT_NAME = "closed-without-merge-receipt.json"
PR_HEAD_REQUIRED_FIELDS = ("headRefOid", "headRefName")
PR_HEAD_OPTIONAL_FIELDS = ("headRepositoryOwner", "headRepositoryName")
PR_HEAD_FIELDS = PR_HEAD_REQUIRED_FIELDS + PR_HEAD_OPTIONAL_FIELDS
NON_MERGE_RECEIPT_SCHEMA_VERSION = 1

_store_path = pathlib.Path(__file__).with_name("workflow-durable-store.py")
if not _store_path.exists(): _store_path = pathlib.Path.cwd()/"scripts/pm/workflow-durable-store.py"
_store_spec = importlib.util.spec_from_file_location("workflow_durable_store", _store_path)
assert _store_spec and _store_spec.loader
durable_store = importlib.util.module_from_spec(_store_spec); _store_spec.loader.exec_module(durable_store)


class _CommandExit(SystemExit):
    """Keep the CLI's numeric failure status while exposing the diagnostic to callers."""

    def __init__(self, message: str) -> None:
        self.message = f"github-project-task: {message}"
        super().__init__(1)

    def __str__(self) -> str:
        return self.message


def die(message: str) -> None:
    print(f"github-project-task: {message}", file=sys.stderr)
    raise _CommandExit(message)


def now() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def load_sync_module() -> Any:
    path = pathlib.Path(__file__).with_name("github-project-sync.py")
    spec = importlib.util.spec_from_file_location("github_project_sync_impl", path)
    if spec is None or spec.loader is None:
        die(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_non_merge_finalizer_module() -> Any:
    path = pathlib.Path(__file__).with_name("non-merge-finalize.py")
    spec = importlib.util.spec_from_file_location("non_merge_finalizer_impl", path)
    if spec is None or spec.loader is None:
        die(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    script_dir = str(path.parent)
    added = script_dir not in sys.path
    if added:
        sys.path.insert(0, script_dir)
    try:
        spec.loader.exec_module(module)
    finally:
        if added:
            sys.path.remove(script_dir)
    return module


def load_mapping(path: pathlib.Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": 1, "tasks": {}}
    return json.loads(path.read_text(encoding="utf-8"))


save_mapping = durable_store.replace_json


def merge_task_mapping(
    path: pathlib.Path,
    task_uid: str,
    record: dict[str, Any],
    project: dict[str, Any] | None = None,
) -> None:
    """Reload under lock and merge only one task, preventing lost updates."""
    def update(latest: dict[str, Any]) -> None:
        if project:
            latest_project = latest.get("project")
            latest_project = latest_project if isinstance(latest_project, dict) else {}
            for key, value in project.items():
                if value not in (None, "") and latest_project.get(key) in (None, ""):
                    latest_project[key] = value
            latest["project"] = latest_project
        latest_record = dict((latest.setdefault("tasks", {}).get(task_uid) or {}))
        for key, value in record.items():
            if key in {"claim_verifications", "evidence_comments"}:
                merged = list(latest_record.get(key) or [])
                for item in value or []:
                    if item not in merged:
                        merged.append(item)
                latest_record[key] = merged
            else:
                latest_record[key] = value
        latest["tasks"][task_uid] = latest_record
    durable_store.transact_json(path, update, {"version": 1, "tasks": {}})


def atomic_json(path: pathlib.Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()
    fd, temporary = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def merge_project_mapping(path: pathlib.Path, project: dict[str, Any]) -> None:
    def update(latest: dict[str, Any]) -> None:
        latest_project = dict(latest.get("project") or {})
        latest_project.update(project)
        latest["project"] = latest_project
    durable_store.transact_json(path, update, {"version": 1, "tasks": {}})


def mapping_path_for(root: pathlib.Path, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    return path if path.is_absolute() else root / path


def pending_non_merge_phase(root: pathlib.Path, task_uid: str,
                            existing: dict[str, Any], repository: str,
                            project: dict[str, Any]) -> str | None:
    """Validate a pre-terminal intent/receipt pair before Project refresh.

    A Project `done` value is a coarse projection and cannot replace a
    receipt-bound pre-terminal phase while remote terminal effects are still
    resumable.  The pair is deliberately strict: a partial or mismatched
    authority must stop refresh rather than rewrite task truth.
    """
    intent = existing.get(NON_MERGE_INTENT_FIELD)
    if intent is None:
        return None
    if not isinstance(intent, dict):
        die("refresh-task: pending non-merge intent is malformed")
    if intent.get("schema") != "oasis7_non_merge_closeout_intent_v1":
        die("refresh-task: pending non-merge intent schema is unsupported")
    if str(intent.get("task_uid") or "") != task_uid:
        die("refresh-task: pending non-merge intent Task UID mismatch")
    if str(intent.get("repository") or "") != str(repository or ""):
        die("refresh-task: pending non-merge intent repository mismatch")
    project_identity = {
        key: str(project.get(key) or "") for key in ("owner", "number", "id")
    }
    if intent.get("project_identity") != project_identity:
        die("refresh-task: pending non-merge intent Project identity mismatch")
    for key, current_key in (
        ("previous_status", "status"),
        ("previous_workflow_phase", "workflow_phase"),
    ):
        if key in intent and intent.get(key) != existing.get(current_key):
            die("refresh-task: pending non-merge intent predecessor disagrees")
    try:
        finalizer = load_non_merge_finalizer_module()
        canonical_path, receipt_path, receipt, _ = finalizer.resolve_non_merge_receipt(
            root, task_uid,
        )
    except (OSError, ValueError, SystemExit) as exc:
        die(f"refresh-task: non-merge receipt resolution failed: {exc}")
    if receipt is None or not receipt_path.is_file():
        die("refresh-task: pending non-merge intent has no receipt")
    migrated = receipt_path != canonical_path
    if receipt.get("receipt_type") != "oasis7_closed_without_merge":
        die("refresh-task: pending non-merge receipt type mismatch")
    if receipt.get("schema_version") != NON_MERGE_RECEIPT_SCHEMA_VERSION or receipt.get("issuer") != "non-merge-finalize":
        die("refresh-task: pending non-merge receipt authority mismatch")
    authority_fields = (
        "task_uid", "repository", "issue_number", "project_item_id",
        "project_identity", "reason", "evidence_sha256", "pr_number", "pr_url",
        "previous_status", "previous_workflow_phase",
    ) + PR_HEAD_FIELDS
    pr_bound = bool(intent.get("pr_number") or intent.get("pr_url"))
    if migrated:
        # A migrated sidecar is the modern authority.  Project identity and,
        # for PR-bound tasks, the required ref identity must be present.  The
        # optional head repository fields remain optional, while a no-PR task
        # must not acquire a fabricated PR head requirement.
        legacy_optional = set(PR_HEAD_OPTIONAL_FIELDS)
        if not pr_bound:
            legacy_optional.update(PR_HEAD_REQUIRED_FIELDS)
        if "evidence" not in receipt:
            die("refresh-task: migrated non-merge receipt lacks evidence authority")
    else:
        # The canonical receipt may still be a legacy pre-migration payload.
        legacy_optional = {
            "project_identity", "previous_status", "previous_workflow_phase",
            *PR_HEAD_FIELDS,
        }
    for key in authority_fields:
        if key not in receipt:
            if key not in legacy_optional:
                die("refresh-task: pending non-merge intent and receipt disagree")
            continue
        if receipt.get(key) != intent.get(key):
            die("refresh-task: pending non-merge intent and receipt disagree")
    if pr_bound:
        if migrated and any(not intent.get(key) or not receipt.get(key)
                            for key in PR_HEAD_REQUIRED_FIELDS):
            die("refresh-task: pending PR-bound non-merge authority lacks PR head identity")
    if (migrated or "previous_status" in receipt) and receipt.get("previous_status") != existing.get("status"):
        die("refresh-task: pending non-merge receipt status snapshot disagrees")
    if (migrated or "previous_workflow_phase" in receipt) and receipt.get("previous_workflow_phase") != existing.get("workflow_phase"):
        die("refresh-task: pending non-merge receipt phase snapshot disagrees")
    phase = str(existing.get("workflow_phase") or "")
    if phase in {"done", "closed_without_merge", "post_" + "merge_done"}:
        die("refresh-task: pending non-merge intent is not pre-terminal")
    return phase


def run_text(cmd: list[str]) -> str:
    result = subprocess.run(
        cmd,
        check=True,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=180,
    )
    return result.stdout.strip()


def authoritative_repository_identity(root: pathlib.Path, repository: str, worktree_hint: str) -> dict[str, str]:
    """Resolve the task/repository identity from the registered git worktree."""
    root = root.resolve(strict=True)
    requested = pathlib.Path(worktree_hint or root).expanduser()
    # A stale/missing hint is not authority.  The active command root is the
    # authoritative registered worktree fallback and is persisted separately
    # as canonical_worktree.
    canonical = (requested if requested.exists() else root).resolve(strict=True)
    def resolved_common_dir(worktree: pathlib.Path) -> pathlib.Path:
        value = pathlib.Path(run_text(["git", "-C", str(worktree), "rev-parse", "--git-common-dir"]))
        return value.resolve() if value.is_absolute() else (worktree / value).resolve()
    root_common_dir = resolved_common_dir(root)
    candidate_common_dir = resolved_common_dir(canonical)
    if candidate_common_dir != root_common_dir:
        die(f"worktree hint belongs to a different git common dir: {canonical}")
    # Membership comes only from the command root's repository registry.  A
    # foreign worktree cannot authorize itself by listing its own family.
    registered = run_text(["git", "-C", str(root), "worktree", "list", "--porcelain"])
    worktrees: list[tuple[str, str]] = []
    current_path = ""
    for line in registered.splitlines() + [""]:
        if line.startswith("worktree "):
            current_path = str(pathlib.Path(line.removeprefix("worktree ")).resolve())
        elif line.startswith("branch refs/heads/") and current_path:
            worktrees.append((current_path, line.removeprefix("branch refs/heads/")))
        elif not line:
            current_path = ""
    canonical_text = str(canonical)
    task_branch = next((branch for path, branch in worktrees if path == canonical_text), "")
    if not task_branch:
        die(f"canonical worktree is detached or unregistered: {canonical}")
    try:
        default_branch = run_text(["git", "-C", str(canonical), "symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).removeprefix("origin/")
    except subprocess.CalledProcessError:
        # `git worktree list` emits the primary worktree first; for a local
        # repository without origin this is the only authoritative default
        # branch fact available.
        default_branch = worktrees[0][1] if worktrees else ""
    normalized_repository = repository.strip().strip("/")
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", normalized_repository):
        die(f"invalid repository identity: {repository!r}")
    if not default_branch:
        die("cannot resolve repository default branch from git facts")
    return {
        "repository": normalized_repository,
        "canonical_worktree": canonical_text,
        "task_branch": task_branch,
        "default_branch": default_branch,
    }


def issue_number_from_url(issue_url: str) -> int:
    try:
        return int(issue_url.rstrip("/").rsplit("/", 1)[-1])
    except ValueError as exc:
        raise RuntimeError(f"cannot parse issue number from {issue_url}") from exc


def pr_number_from_url(pr_url: str) -> int | None:
    match = re.search(r"/pull/(\d+)(?:$|[?#])", pr_url)
    return int(match.group(1)) if match else None


def issue_task_fields(body: str) -> dict[str, Any]:
    body = body.replace("\r\n", "\n")
    fields: dict[str, Any] = {}
    for key in ("owner_role", "module", "status", "priority", "worktree_hint", "source_signal", "source_type", "severity"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    hold_values: dict[str, Any] = {}
    for key in ("kind", "requester", "reason", "resume_authority", "active"):
        match = re.search(rf"^- merge_hold_{key}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            hold_values[key] = match.group(1)
    if hold_values:
        hold_values["active"] = str(hold_values.get("active", "false")).lower() == "true"
        fields["merge_hold"] = hold_values
    for key in ("pr_url", "pr_number"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    source_refs = re.findall(r"^- `([^`]+)`$", body, re.MULTILINE)
    if source_refs:
        fields["source_refs"] = source_refs
    lines = body.splitlines()
    acceptance: list[str] = []
    for index, line in enumerate(lines):
        if line.strip() != "Acceptance:":
            continue
        for item in lines[index + 1:]:
            match = re.match(r"^-\s+(?:\[[ xX]\]\s*)?(.*\S)\s*$", item)
            if not match:
                break
            acceptance.append(match.group(1).strip())
        break
    fields["acceptance"] = acceptance
    return fields


def github_issue_record(repo: str, task_uid: str) -> dict[str, Any] | None:
    search_payload = run_text(
        [
            "gh",
            "issue",
            "list",
            "-R",
            repo,
            "--state",
            "all",
            "--search",
            f"{task_uid} in:body",
            "--json",
            "number,url,title,state",
            "--limit",
            "5",
        ]
    )
    hits = json.loads(search_payload)
    if not isinstance(hits, list) or len(hits) != 1:
        return None
    issue_number = int(hits[0].get("number") or 0)
    if not issue_number:
        return None
    issue_payload = run_text(["gh", "issue", "view", str(issue_number), "-R", repo, "--json", "body,number,title,url"])
    issue = json.loads(issue_payload)
    body = str(issue.get("body") or "").replace("\r\n", "\n")
    if not re.search(rf"^task_uid:\s*{re.escape(task_uid)}$", body, re.MULTILINE):
        return None
    record = issue_task_fields(body)
    title = str(issue.get("title") or hits[0].get("title") or "")
    if title.startswith("[PM] "):
        title = title[5:]
    record.update(
        {
            "task_uid": task_uid,
            "title": title,
            "issue_number": int(issue.get("number") or issue_number),
            "issue_url": str(issue.get("url") or hits[0].get("url") or ""),
            "_github_source": "issue_search",
        }
    )
    return record


def task_from_record(uid: str, record: dict[str, Any]) -> OrderedDict[str, Any]:
    return OrderedDict(
        [
            ("task_uid", uid),
            ("title", record.get("title") or ""),
            ("owner_role", record.get("owner_role") or ""),
            ("module", record.get("module") or ""),
            ("worktree_hint", record.get("worktree_hint") or ""),
            ("status", record.get("status") or "candidate"),
            ("workflow_phase", record.get("workflow_phase") or ""),
            ("priority", record.get("priority") or "P2"),
            ("source_signal", record.get("source_signal") or ""),
            ("source_type", record.get("source_type") or ""),
            ("severity", record.get("severity") or ""),
            ("pr_url", record.get("pr_url") or record.get("pull_request_url") or ""),
            ("pr_number", record.get("pr_number") or ""),
            ("merge_hold", record.get("merge_hold") or {}),
            ("source_refs", record.get("source_refs") or []),
            ("acceptance", record.get("acceptance") or []),
            ("updated_at", record.get("updated_at") or now()),
        ]
    )


def issue_body(task: OrderedDict[str, Any]) -> str:
    lines = [
        "<!-- oasis7-pm-task -->",
        f"task_uid: {task['task_uid']}",
        "",
        "GitHub-backed oasis7 PM task.",
        "",
        "Task metadata:",
        f"- owner_role: `{task.get('owner_role')}`",
        f"- module: `{task.get('module') or ''}`",
        f"- status: `{task.get('status')}`",
        f"- priority: `{task.get('priority')}`",
        f"- worktree_hint: `{task.get('worktree_hint') or ''}`",
    ]
    if task.get("source_signal") or task.get("source_type") or task.get("severity"):
        lines.extend(
            [
                f"- source_signal: `{task.get('source_signal') or ''}`",
                f"- source_type: `{task.get('source_type') or ''}`",
                f"- severity: `{task.get('severity') or ''}`",
            ]
        )
    if task.get("pr_url"):
        lines.append(f"- pr_url: `{task.get('pr_url')}`")
    if task.get("pr_number"):
        lines.append(f"- pr_number: `{task.get('pr_number')}`")
    if task.get("merge_hold"):
        hold = task["merge_hold"]
        for key in ("kind", "requester", "reason", "resume_authority", "active"):
            value = str(hold.get(key, "")).lower() if key == "active" else hold.get(key, "")
            lines.append(f"- merge_hold_{key}: `{value}`")
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


def create_issue(repo: str, task: OrderedDict[str, Any]) -> str:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(issue_body(task))
        body_path = handle.name
    try:
        return run_text(["gh", "issue", "create", "-R", repo, "--title", f"[PM] {task['title']}", "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def update_issue_body(repo: str, issue_number: int, task: OrderedDict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(issue_body(task))
        body_path = handle.name
    try:
        run_text(["gh", "issue", "edit", str(issue_number), "-R", repo, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def issue_comment(repo: str, issue_number: int, body: str) -> str:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = handle.name
    try:
        return run_text(["gh", "issue", "comment", str(issue_number), "-R", repo, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def evidence_body(task_uid: str, role: str, phase: str, fields: dict[str, Any]) -> str:
    lines = [
        "<!-- oasis7-pm-evidence -->",
        f"Task UID: {task_uid}",
        f"Evidence Phase: {phase}",
        f"Role: {role}",
        f"Recorded At: {now()}",
        "",
    ]
    for key, value in fields.items():
        if isinstance(value, list):
            value = ", ".join(str(item) for item in value)
        lines.append(f"{key}: {value}")
    return "\n".join(lines) + "\n"


def update_project_fields(
    args: argparse.Namespace,
    task: OrderedDict[str, Any],
    project_item_id: str,
    *,
    require_lifecycle_projection: bool = False,
) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    if require_lifecycle_projection:
        values = sync.project_field_values(task)
        missing: list[str] = []
        for field_name in ("Status", "PM Status", "Workflow Phase"):
            value = str(values.get(field_name) or "")
            field = fields.get(field_name)
            if not field:
                missing.append(f"{field_name}:missing_field")
            elif not value:
                missing.append(f"{field_name}:empty_value")
            elif field_name in sync.SINGLE_SELECT_FIELDS and value not in (field.get("options_by_name") or {}):
                missing.append(f"{field_name}:missing_option:{value}")
        if missing:
            die(
                "project lifecycle projection is unavailable; refusing to leave local task truth ahead of GitHub Project: "
                + ", ".join(missing)
                + "; add the missing Project field/options, then rerun "
                + f"./scripts/pm/refresh-task-cache.sh --task-uid {args.task_uid} --json and the same lifecycle command"
            )
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields)
    if skipped:
        print(f"github-project-task: skipped fields: {', '.join(skipped)}", file=sys.stderr)
    if require_lifecycle_projection:
        unresolved = [
            item for item in skipped
            if item.split(":", 1)[0] in {"Status", "PM Status", "Workflow Phase"}
            and not item.endswith(":unchanged")
        ]
        if unresolved:
            die(
                "project lifecycle projection was not fully updated; refusing local/Project phase drift: "
                + ", ".join(unresolved)
                + "; rerun the same lifecycle command after the Project fields are available"
            )
    return int(updated)


def update_pr_project_field(args: argparse.Namespace, task: OrderedDict[str, Any], project_item_id: str) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields, only_fields={"PR"})
    if skipped:
        print(f"github-project-task: skipped draft PR field: {', '.join(skipped)}", file=sys.stderr)
    if int(updated) != 1:
        die(f"record-pr: refusing draft candidate because Project PR field was not updated: updated={updated}/1")
    return int(updated)


def update_done_project_fields(args: argparse.Namespace, task: OrderedDict[str, Any], project_item_id: str) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    required_fields = {"Status", "PM Status", "Workflow Phase"}
    values = sync.project_field_values(task)
    missing: list[str] = []
    for field_name in sorted(required_fields):
        value = str(values.get(field_name) or "")
        field = fields.get(field_name)
        if not field:
            missing.append(f"{field_name}:missing_field")
            continue
        if not value:
            missing.append(f"{field_name}:empty_value")
            continue
        if field_name in sync.SINGLE_SELECT_FIELDS and value not in (field.get("options_by_name") or {}):
            missing.append(f"{field_name}:missing_option:{value}")
    if missing:
        die(
            "move-task: refusing done because required GitHub Project fields are unavailable: "
            + ", ".join(missing)
        )
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields, only_fields=required_fields)
    if skipped:
        print(f"github-project-task: skipped done fields: {', '.join(skipped)}", file=sys.stderr)
    if int(updated) != len(required_fields):
        die(
            "move-task: refusing done because required GitHub Project fields were not updated: "
            f"updated={updated}/{len(required_fields)}"
        )
    return int(updated)


def add_project_item(args: argparse.Namespace, issue_url: str) -> str:
    payload = json.loads(
        run_text(
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
    )
    item_id = str(payload.get("id") or "")
    if not item_id:
        die("gh project item-add returned no item id")
    return item_id


def command_new_task(args: argparse.Namespace) -> int:
    root = args.root.resolve()
    mapping_path = mapping_path_for(root, args.mapping)
    repository_identity = authoritative_repository_identity(root, args.repo, args.worktree_hint or str(root))
    immutable_request = OrderedDict(
        [
            ("repo", args.repo),
            ("title", args.title),
            ("owner_role", args.owner_role),
            ("worktree_hint", args.worktree_hint or ""),
            ("module", args.module or ""),
            ("priority", args.priority),
            ("source_refs", sorted(args.source_ref or [])),
            ("acceptance", list(args.acceptance or [])),
            ("source_signal", args.source_signal or ""),
            ("source_type", args.source_type or ""),
            ("severity", args.severity or ""),
            ("doc_refs", sorted(args.doc_ref or [])),
            ("related_prd", sorted(args.related_prd or [])),
            ("handoff_to", sorted(args.handoff_to or [])),
        ]
    )
    immutable_digest = hashlib.sha256(
        json.dumps(immutable_request, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    ).hexdigest()
    journal_key = hashlib.sha256(
        "\0".join((args.repo, args.title, args.owner_role, args.worktree_hint or "")).encode()
    ).hexdigest()
    test_scratch = os.environ.get("OASIS7_PM_TEST_SCRATCH", "")
    if test_scratch:
        scratch_root = pathlib.Path(test_scratch)
        if not scratch_root.is_absolute():
            die("OASIS7_PM_TEST_SCRATCH must be absolute")
        journal_root = scratch_root / "bootstrap-journal"
    else:
        journal_root = root / ".pm/scratch/bootstrap-journal"
    journal_path = journal_root / f"{journal_key}.json"
    journal_existed = journal_path.exists()
    journal = json.loads(journal_path.read_text(encoding="utf-8")) if journal_existed else {}
    if journal:
        recorded_request = journal.get("immutable_request") or journal.get("request")
        if not isinstance(recorded_request, dict):
            die("bootstrap journal is missing immutable request; cannot resume safely")
        normalized_recorded = OrderedDict(
            (key, sorted(recorded_request.get(key) or []) if key in {"source_refs", "doc_refs", "related_prd", "handoff_to"} else
             list(recorded_request.get(key) or []) if key == "acceptance" else
             str(recorded_request.get(key) or ""))
            for key in immutable_request
        )
        if normalized_recorded != immutable_request:
            changed = [key for key in immutable_request if normalized_recorded.get(key) != immutable_request.get(key)]
            die("bootstrap immutable request drift; start a new task bootstrap (mismatch: " + ",".join(changed) + ")")
        recorded_digest = str(journal.get("immutable_request_digest") or "")
        if recorded_digest and recorded_digest != immutable_digest:
            die("bootstrap immutable request digest mismatch; journal may be corrupt")
    task_uid = str(journal.get("task_uid") or f"task_{uuid.uuid4().hex}")
    if not journal:
        journal = {"version": 2, "task_uid": task_uid, "state": "planned", "next_action": "create_issue",
                   "immutable_request": immutable_request, "immutable_request_digest": immutable_digest,
                   "updated_at": now()}
        atomic_json(journal_path, journal)
    task = OrderedDict(
        [
            ("task_uid", task_uid),
            ("title", args.title),
            ("owner_role", args.owner_role),
            ("module", args.module or ""),
            ("worktree_hint", args.worktree_hint or ""),
            ("status", "candidate"),
            ("priority", args.priority),
            ("source_signal", args.source_signal or ""),
            ("source_type", args.source_type or ""),
            ("severity", args.severity or ""),
            ("source_refs", args.source_ref or []),
            ("doc_refs", args.doc_ref or []),
            ("related_prd", args.related_prd or []),
            ("acceptance", args.acceptance or []),
            ("handoff_to", args.handoff_to or []),
            ("updated_at", now()),
        ]
    )
    issue_url = str(journal.get("issue_url") or "")
    if not issue_url and journal_existed:
        try:
            recovered = github_issue_record(args.repo, task_uid)
        except (subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError):
            recovered = None
        issue_url = str((recovered or {}).get("issue_url") or "")
    if not issue_url:
        issue_url = create_issue(args.repo, task)
    journal.update({"issue_url": issue_url, "state": "issue_created", "next_action": "add_project_item", "updated_at": now()})
    atomic_json(journal_path, journal)
    issue_number = issue_number_from_url(issue_url)
    item_id = str(journal.get("project_item_id") or "")
    if not item_id:
        item_id = add_project_item(args, issue_url)
    journal.update({"project_item_id": item_id, "state": "project_item_added", "next_action": "update_project_fields", "updated_at": now()})
    atomic_json(journal_path, journal)
    updated_fields = update_project_fields(args, task, item_id)
    record = {
        "task_uid": task_uid,
        "title": args.title,
        "owner_role": args.owner_role,
        "module": args.module or "",
        "worktree_hint": args.worktree_hint or "",
        "status": "candidate",
        "priority": args.priority,
        "source_signal": args.source_signal or "",
        "source_type": args.source_type or "",
        "severity": args.severity or "",
        "source_refs": args.source_ref or [],
        "doc_refs": args.doc_ref or [],
        "related_prd": args.related_prd or [],
        "acceptance": args.acceptance or [],
        "handoff_to": args.handoff_to or [],
        "issue_url": issue_url,
        "issue_number": issue_number,
        "project_item_id": item_id,
        "created_at": now(),
        "updated_at": now(),
        "evidence_sink": issue_url,
        **repository_identity,
    }
    merge_task_mapping(mapping_path, task_uid, record)
    merge_project_mapping(mapping_path, {"owner": args.project_owner, "number": args.project_number, "repo": args.repo})
    journal.update({"state": "completed", "next_action": "none", "mapping_path": str(mapping_path), "updated_at": now()})
    atomic_json(journal_path, journal)
    payload = dict(record)
    payload.update(
        {
            "task_path": issue_url,
            "execution_log_path": issue_url,
            "updated_field_values": updated_fields,
            "mapping_path": str(mapping_path),
            "bootstrap_journal": str(journal_path),
        }
    )
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print(f"new-task: created {task_uid} ({issue_url})")
    return 0


def require_record(args: argparse.Namespace) -> tuple[pathlib.Path, dict[str, Any], dict[str, Any]]:
    root = args.root.resolve()
    mapping_path = mapping_path_for(root, args.mapping)
    mapping = load_mapping(mapping_path)
    record = mapping.get("tasks", {}).get(args.task_uid)
    if not record:
        try:
            record = github_issue_record(args.repo, args.task_uid)
        except (subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as exc:
            die(f"task_uid not found in mapping and GitHub issue lookup failed: {args.task_uid}: {exc}")
        if not record:
            die(f"task_uid not found in mapping or GitHub issue body: {args.task_uid}")
        mapping.setdefault("tasks", {})[args.task_uid] = record
    return mapping_path, mapping, record


def recover_missing_project_item(args: argparse.Namespace, record: dict[str, Any]) -> None:
    if record.get("project_item_id"):
        return
    try:
        recovered = load_sync_module().recover_project_mapping(args.project_owner, args.project_number)
    except Exception:
        return
    record.update(recovered.get(args.task_uid) or {})


def has_verified_task_complete(record: dict[str, Any]) -> bool:
    for item in record.get("claim_verifications") or []:
        if not isinstance(item, dict):
            continue
        if str(item.get("claim_type") or "") != "task_complete":
            continue
        if str(item.get("status") or "") != "verified":
            continue
        if str(item.get("verification_exit_code") or "") in {"", "0"}:
            return True
    return (
        str(record.get("last_claim_type") or "") == "task_complete"
        and str(record.get("last_verification_status") or "") == "verified"
        and str(record.get("last_verification_exit_code") or "") in {"", "0"}
    )


def command_append_evidence(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    issue_number = int(record["issue_number"])
    fields = {
        "Completed": args.completed,
        "Pending": args.pending,
        "Action": args.action,
        "Validation Command": args.validation_command,
        "Expected Result": args.expected_result,
        "Actual Result": args.actual_result,
        "Blocker / Next Action": args.blocker_next_action,
    }
    comment_url = issue_comment(args.repo, issue_number, evidence_body(args.task_uid, args.role, "execution", fields))
    record["last_evidence_at"] = now()
    record["updated_at"] = now()
    record.setdefault("evidence_comments", []).append(comment_url)
    merge_task_mapping(mapping_path, args.task_uid, record)
    payload = {
        "task_uid": args.task_uid,
        "issue_url": record.get("issue_url"),
        "comment_url": comment_url,
        "execution_log_path": record.get("issue_url"),
        "status": "ok",
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"append-execution-log: appended evidence to {comment_url}")
    return 0


def command_workflow_report(args: argparse.Namespace) -> int:
    if args.phase in {"start", "close"} and not args.task_uid:
        die("workflow-report: --task-uid is required for start and close")
    if args.phase == "review" and not args.task_uid:
        payload = {"phase": "review", "role": args.role, "status": "ok", "task_source": "github_project"}
        print(json.dumps(payload, indent=2, sort_keys=True) if args.json else "workflow-report review: GitHub Project is authoritative")
        return 0
    mapping_path, mapping, record = require_record(args)
    fields = {
        "Workflow Phase": args.phase,
        "Task Status": record.get("status"),
        "Issue": record.get("issue_url"),
        "Worktree": record.get("worktree_hint") or "",
    }
    comment_url = issue_comment(args.repo, int(record["issue_number"]), evidence_body(args.task_uid, args.role, args.phase, fields))
    timestamp_key = {
        "start": "last_started_at",
        "close": "last_workflow_report_close_at",
    }.get(args.phase)
    if timestamp_key:
        record[timestamp_key] = now()
    record["last_evidence_at"] = now()
    record["updated_at"] = now()
    record.setdefault("evidence_comments", []).append(comment_url)
    merge_task_mapping(mapping_path, args.task_uid, record)
    payload = {
        "task_uid": args.task_uid,
        "role": args.role,
        "phase": args.phase,
        "status": "ok",
        "issue_url": record.get("issue_url"),
        "execution_log_path": record.get("issue_url"),
        "comment_url": comment_url,
    }
    if timestamp_key:
        payload[timestamp_key] = record[timestamp_key]
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"workflow-report {args.phase}: recorded {comment_url}")
    return 0


def command_move_task(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    previous = str(record.get("status") or "")
    if args.to_status == "done" and not (record.get("last_closed_at") and has_verified_task_complete(record)):
        die(
            "move-task: refusing done without closeout and verified task_complete evidence; "
            "run ./scripts/pm/task-closeout.sh --role <owner_role> --task-uid "
            f"{args.task_uid} --verify-command '<cmd>'"
        )
    if args.to_status == "done":
        recover_missing_project_item(args, record)
    if args.to_status == "done" and not record.get("project_item_id"):
        die(
            "move-task: refusing done because GitHub Project item mapping could not be recovered; "
            f"task_uid={args.task_uid}"
        )
    record["status"] = args.to_status
    record["updated_at"] = now()
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    if args.to_status != "done" and record.get("project_item_id"):
        updated_fields = update_project_fields(args, task, str(record["project_item_id"]))
    update_issue_body(args.repo, int(record["issue_number"]), task)
    merge_task_mapping(mapping_path, args.task_uid, record)
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": args.to_status,
        "issue_url": record.get("issue_url"),
        "project_item_id": record.get("project_item_id"),
        "updated_field_values": updated_fields,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"move-task: moved {args.task_uid} {previous} -> {args.to_status}")
    return 0


def command_closeout_task(args: argparse.Namespace) -> int:
    mapping_path, mapping, original = require_record(args)
    previous = str(original.get("status") or "")
    claim = json.loads(args.claim_json)
    if args.to_status != "deferred":
        if claim.get("status") != "verified" or not claim.get("allowed_to_claim"):
            die("closeout-task: verified immutable claim evidence is required")
    record = json.loads(json.dumps(original))
    closed_at = now()
    record.setdefault("claim_verifications", []).append(claim)
    record["last_claim_verification_at"] = claim.get("verified_at")
    record["last_closed_at"] = closed_at
    record["last_evidence_at"] = closed_at
    record["status"] = args.to_status
    if args.to_status == "done" and args.pr_receipt:
        receipt = json.loads(pathlib.Path(args.pr_receipt).read_text(encoding="utf-8"))
        record["merge_receipt"] = receipt
        record["merge_receipt_sha256"] = hashlib.sha256(pathlib.Path(args.pr_receipt).read_bytes()).hexdigest()
    if args.to_status == "done":
        recover_missing_project_item(args, record)
        if not record.get("project_item_id"):
            die("closeout-task: done requires a recoverable GitHub Project item")
    task = task_from_record(args.task_uid, record)
    terminal_phase = "task_done" if args.to_status == "done" else "pre_pr_ready"
    record["workflow_phase"] = terminal_phase
    task = task_from_record(args.task_uid, record)
    evidence_fields = {
        "Workflow Phase": "task_done" if args.to_status == "done" else "pre_pr_ready",
        "Task Status": args.to_status,
        "Immutable Verification Head": claim.get("frozen_source_head") or "n/a",
        "Immutable Verification Tree": claim.get("frozen_source_tree") or "n/a",
    }
    if record.get("merge_receipt"):
        receipt = record["merge_receipt"]
        evidence_fields.update({
            "Merge Receipt Issuer": receipt.get("issuer"),
            "Merge Receipt Repository": receipt.get("repository"),
            "Merge Receipt PR": receipt.get("pr_url"),
            "Merge Receipt Head": receipt.get("head_oid"),
            "Merge Receipt Observed At": receipt.get("observed_at"),
        })
    comment_url = issue_comment(
        args.repo,
        int(record["issue_number"]),
        evidence_body(
            args.task_uid,
            args.role,
            terminal_phase,
            evidence_fields,
        ),
    )
    updated_fields = 0
    if args.to_status == "done":
        updated_fields = update_done_project_fields(args, task, str(record["project_item_id"]))
    elif record.get("project_item_id"):
        updated_fields = update_project_fields(
            args,
            task,
            str(record["project_item_id"]),
            require_lifecycle_projection=True,
        )
    update_issue_body(args.repo, int(record["issue_number"]), task)
    record.setdefault("evidence_comments", []).append(comment_url)
    if record.get("_github_source") != "issue_search" or mapping_path.exists():
        cache_patch = {
            "status": record["status"],
            "last_closed_at": record["last_closed_at"],
            "last_evidence_at": record["last_evidence_at"],
            "last_claim_verification_at": record.get("last_claim_verification_at"),
            "claim_verifications": [claim],
            "evidence_comments": [comment_url],
            "workflow_phase": terminal_phase,
        }
        if record.get("merge_receipt"):
            cache_patch["merge_receipt"] = record["merge_receipt"]
            cache_patch["merge_receipt_sha256"] = record["merge_receipt_sha256"]
        if record.get("project_item_id"):
            cache_patch["project_item_id"] = record["project_item_id"]
        merge_task_mapping(mapping_path, args.task_uid, cache_patch)
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": args.to_status,
        "issue_url": record.get("issue_url"),
        "comment_url": comment_url,
        "last_closed_at": closed_at,
        "updated_field_values": updated_fields,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"closeout-task: {args.task_uid} -> {args.to_status}")
    return 0


def command_set_phase(args: argparse.Namespace) -> int:
    mapping_path, _mapping, original = require_record(args)
    receipt = json.loads(pathlib.Path(args.receipt_json).read_text(encoding="utf-8"))
    current = str(original.get("workflow_phase") or "")
    allowed_transition = args.phase in ALLOWED_PHASE_TRANSITIONS.get(current, set())
    if not allowed_transition:
        die(f"set-phase: transition {current!r} -> {args.phase!r} is not allowed")
    receipt_schema = RECEIPT_SCHEMAS.get(args.phase)
    if not receipt_schema or (receipt.get("receipt_type"), receipt.get("issuer")) != receipt_schema:
        die("set-phase: receipt schema or issuer mismatch")
    if receipt.get("task_uid") != args.task_uid:
        die("set-phase: receipt task_uid mismatch")
    record = json.loads(json.dumps(original))
    record["workflow_phase"] = args.phase
    record.setdefault("phase_receipts", {})[args.phase] = receipt
    record.setdefault("phase_receipt_sha256", {})[args.phase] = hashlib.sha256(pathlib.Path(args.receipt_json).read_bytes()).hexdigest()
    comment_url = issue_comment(args.repo, int(record["issue_number"]), evidence_body(
        args.task_uid, args.role, args.phase,
        {"Workflow Phase": args.phase, "Receipt Type": receipt.get("receipt_type"),
         "Receipt Observed At": receipt.get("observed_at")},
    ))
    record.setdefault("evidence_comments", []).append(comment_url)
    task = task_from_record(args.task_uid, record)
    if record.get("project_item_id"):
        update_project_fields(args, task, str(record["project_item_id"]))
    merge_task_mapping(mapping_path, args.task_uid, {
        "workflow_phase": args.phase,
        "phase_receipts": record["phase_receipts"],
        "phase_receipt_sha256": record["phase_receipt_sha256"],
        "evidence_comments": [comment_url],
        "last_evidence_at": now(),
    })
    print(json.dumps({"status":"ok","task_uid":args.task_uid,"workflow_phase":args.phase,
                      "comment_url":comment_url}, sort_keys=True))
    return 0


def project_refresh_graphql(query: str, variables: list[str]) -> dict[str, Any]:
    return json.loads(run_text(["gh", "api", "graphql", "-f", f"query={query}", *variables]))


def refresh_project_identity(
    node: dict[str, Any],
    canonical_owner: str,
    canonical_number: int,
    canonical_project_id: str,
) -> dict[str, Any]:
    project = node.get("project")
    if not isinstance(project, dict):
        die("refresh-task: live Project item is missing Project identity")
    project_id = str(project.get("id") or "")
    owner = project.get("owner")
    owner = owner if isinstance(owner, dict) else {}
    project_owner = str(owner.get("login") or "")
    try:
        project_number = int(project.get("number"))
    except (TypeError, ValueError):
        die("refresh-task: live Project identity has an invalid number")
    if not project_id or not project_owner:
        die("refresh-task: live Project identity is incomplete")
    if project_owner != canonical_owner:
        die(
            "refresh-task: live Project owner does not match canonical owner: "
            f"{project_owner!r} != {canonical_owner!r}"
        )
    if project_number != canonical_number:
        die(
            "refresh-task: live Project number does not match canonical Project: "
            f"{project_number!r} != {canonical_number!r}"
        )
    if canonical_project_id and project_id != canonical_project_id:
        die(
            "refresh-task: live Project id does not match canonical Project: "
            f"{project_id!r} != {canonical_project_id!r}"
        )
    field_values = node.get("fieldValues")
    field_values = field_values if isinstance(field_values, dict) else {}
    page_info = field_values.get("pageInfo")
    page_info = page_info if isinstance(page_info, dict) else {}
    if page_info.get("hasNextPage") is not False:
        die("refresh-task: live Project item fieldValues pagination is incomplete or unknown")
    return {"id": project_id, "number": project_number, "owner": project_owner}


def command_refresh_task(args: argparse.Namespace) -> int:
    mapping_path = mapping_path_for(args.root.resolve(), args.mapping)
    latest = load_mapping(mapping_path)
    existing = dict((latest.get("tasks") or {}).get(args.task_uid) or {})
    root = args.root.resolve()
    project = latest.get("project") or {}
    project = project if isinstance(project, dict) else {}
    canonical_owner = str(project.get("owner") or args.project_owner or "")
    try:
        canonical_number = int(project.get("number") or args.project_number)
    except (TypeError, ValueError):
        die("refresh-task: canonical Project number is invalid")
    canonical_project_id = str(project.get("id") or "")
    if canonical_owner != str(args.project_owner or "") or canonical_number != int(args.project_number):
        die("refresh-task: configured Project identity does not match canonical mapping")
    pending_phase = pending_non_merge_phase(
        root, args.task_uid, existing, args.repo, project,
    )
    live = github_issue_record(args.repo, args.task_uid)
    if not live:
        die(f"refresh-task: authoritative GitHub issue not found for {args.task_uid}")
    # The command root is execution context, not task identity.  Terminal
    # refreshes intentionally run from the default worktree, so rebinding the
    # task to that root would destroy the canonical task-worktree/branch pair.
    # Resolve registered identity from task truth instead and fail closed when
    # live and cached task identities disagree.
    identity_candidates: list[dict[str, str]] = []
    for hint in (
        str(existing.get("canonical_worktree") or ""),
        str(live.get("worktree_hint") or ""),
    ):
        if not hint or not pathlib.Path(hint).expanduser().exists():
            continue
        candidate = authoritative_repository_identity(root, args.repo, hint)
        if not any(
            item["canonical_worktree"] == candidate["canonical_worktree"]
            for item in identity_candidates
        ):
            identity_candidates.append(candidate)
    if len(identity_candidates) > 1:
        die("refresh-task: cached and live canonical worktree identities disagree")
    if not identity_candidates:
        die("refresh-task: no registered canonical task worktree identity is available")
    repository_identity = identity_candidates[0]
    live["worktree_hint"] = repository_identity["canonical_worktree"]
    recovered: dict[str, Any] = {}
    item_id = str(existing.get("project_item_id") or "")
    project_fields: dict[str, str] = {}
    selected_node: dict[str, Any] | None = None
    live_project_identity: dict[str, Any] | None = None
    if item_id:
        query = """
        query($ids: [ID!]!) {
          nodes(ids: $ids) {
            ... on ProjectV2Item {
              id
              project {
                id
                number
                owner {
                  ... on Organization { login }
                  ... on User { login }
                }
              }
              fieldValues(first: 100) {
                pageInfo { hasNextPage }
                nodes {
                  ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
                  ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
                }
              }
            }
          }
        }
        """
        payload = project_refresh_graphql(query, ["-F", f"ids[]={item_id}"])
        nodes = ((payload.get("data") or {}).get("nodes") or [])
        if nodes:
            selected_node = nodes[0]
            if not isinstance(selected_node, dict):
                die("refresh-task: live Project item readback is malformed")
            live_project_identity = refresh_project_identity(
                selected_node,
                canonical_owner,
                canonical_number,
                canonical_project_id,
            )
        if selected_node is None:
            die("refresh-task: bound Project item is unavailable; refusing to refresh without Project binding")
    else:
        query = """
        query($q: String!) {
          search(query: $q, type: ISSUE, first: 2) {
            nodes { ... on Issue { number url body projectItems(first: 20) { nodes {
              id
              project {
                id
                number
                owner {
                  ... on Organization { login }
                  ... on User { login }
                }
              }
              fieldValues(first: 100) {
                pageInfo { hasNextPage }
                nodes {
                ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
                ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
                }
              }
            } } } }
          }
        }
        """
        payload = project_refresh_graphql(query, ["-f", f"q=repo:{args.repo} {args.task_uid} in:body"])
        issues = (((payload.get("data") or {}).get("search") or {}).get("nodes") or [])
        matches = [issue for issue in issues if re.search(
            rf"^task_uid:\s*{re.escape(args.task_uid)}$", str(issue.get("body") or ""), re.MULTILINE)]
        nodes = []
        if len(matches) == 1:
            for node in ((matches[0].get("projectItems") or {}).get("nodes") or []):
                project_node = node.get("project") if isinstance(node, dict) else None
                project_number = project_node.get("number") if isinstance(project_node, dict) else None
                try:
                    is_canonical_number = int(project_number) == canonical_number
                except (TypeError, ValueError):
                    is_canonical_number = False
                if not is_canonical_number:
                    continue
                if not isinstance(node, dict):
                    die("refresh-task: live Project item readback is malformed")
                live_project_identity = refresh_project_identity(
                    node,
                    canonical_owner,
                    canonical_number,
                    canonical_project_id,
                )
                nodes = [node]
                selected_node = node
                item_id = str(node.get("id") or "")
                if not item_id:
                    die("refresh-task: live Project item identity is missing")
                recovered = {"project_item_id": item_id, "issue_url": matches[0].get("url"),
                             "issue_number": matches[0].get("number")}
                break
        # The issue is still authoritative when its project item is temporarily
        # unavailable to this refresh query.  Preserve the refreshed issue and
        # repository identity; a later refresh can recover the project fields.
    if item_id:
        if selected_node:
            for value in (((selected_node.get("fieldValues") or {}).get("nodes")) or []):
                field_name = str(((value.get("field") or {}).get("name") or ""))
                field_value = str(value.get("name") or value.get("text") or "")
                if field_name and field_value:
                    project_fields[field_name] = field_value
    authoritative_keys = {
        "task_uid", "title", "issue_number", "issue_url", "owner_role", "module",
        "status", "priority", "worktree_hint", "source_signal", "source_type",
        "severity", "pr_url", "pr_number", "merge_hold", "source_refs", "acceptance",
    }
    record: dict[str, Any] = {}
    for key in authoritative_keys:
        if key in live:
            record[key] = live[key]
        elif key == "acceptance":
            record[key] = []
    record.update({key: value for key, value in recovered.items() if value not in (None, "")})
    project_status = project_fields.get("PM Status", "")
    lifecycle_rank = {
        "candidate": 0, "committed": 1, "blocked": 2, "ready": 3,
        "pr_watch": 4, "done": 5, "deferred": 5,
    }
    issue_status = str(record.get("status") or "candidate")
    if project_status in lifecycle_rank and lifecycle_rank[project_status] >= lifecycle_rank.get(issue_status, -1):
        record["status"] = project_status
        record["project_status"] = project_fields.get("Status", "")
        project_phase = project_fields.get("Workflow Phase", "")
        existing_phase = str(existing.get("workflow_phase") or "")
        fine_terminal_phases = {
            "closed_without_" + "merge",
            "post_" + "merge_done",
        }
        if pending_phase is not None:
            record["workflow_phase"] = pending_phase
            pending_intent = existing.get("closed_without_merge_intent") or {}
            if isinstance(pending_intent, dict) and pending_intent.get("previous_status"):
                # Coarse Project `done` is an in-flight terminal side effect,
                # not authority to rewrite the predecessor bound by intent.
                record["status"] = pending_intent["previous_status"]
        elif project_status == "done" and existing_phase in fine_terminal_phases:
            # Project exposes both terminal receipt phases as coarse `done`;
            # refreshing its fields must not erase the finer local phase.
            record["workflow_phase"] = existing_phase
        else:
            record["workflow_phase"] = project_phase
        record["reconciled_from_project"] = project_status != issue_status
    if pending_phase is not None:
        # Pending intent authority wins independently of Project/Issue rank:
        # either remote sink may have advanced first when a run crashed.
        pending_intent = existing.get("closed_without_merge_intent") or {}
        record["workflow_phase"] = pending_phase
        if isinstance(pending_intent, dict) and pending_intent.get("previous_status"):
            record["status"] = pending_intent["previous_status"]
    record["cache_refreshed_at"] = now()
    # Local cache identity is never accepted from stale issue/project/cache
    # values.  Every refresh overwrites it from current registered git facts.
    record.update(repository_identity)
    project_patch = None
    if live_project_identity:
        project_patch = {
            "owner": live_project_identity["owner"],
            "number": live_project_identity["number"],
            "repo": args.repo,
            "id": live_project_identity["id"],
        }
    merge_task_mapping(mapping_path, args.task_uid, record, project=project_patch)
    committed = (load_mapping(mapping_path).get("tasks") or {}).get(args.task_uid) or record
    payload = {
        "status": "refreshed",
        "task_uid": args.task_uid,
        "issue_url": committed.get("issue_url"),
        "project_item_id": committed.get("project_item_id"),
        "task_status": committed.get("status"),
        "acceptance": committed.get("acceptance") or [],
        "cache_refreshed_at": committed["cache_refreshed_at"],
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"refresh-task: refreshed {args.task_uid}")
    return 0


def command_record_pr(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    previous = str(record.get("status") or "")
    record["pr_url"] = args.pr_url
    number = pr_number_from_url(args.pr_url)
    if number is not None:
        record["pr_number"] = number
    is_draft_candidate = bool(getattr(args, "draft_candidate", False))
    target_status = "committed" if is_draft_candidate else "pr_watch"
    target_phase = "verification" if is_draft_candidate else "pr_watch"
    record["status"] = target_status
    record["workflow_phase"] = target_phase
    record.setdefault("merge_hold", {
        "kind": "normal_pr_ci_watch",
        "active": False,
        "requester": args.role,
        "reason": "default PR purpose decision recorded by record-pr",
        "resume_authority": args.role,
        "recorded_at": now(),
    })
    record["updated_at"] = now()
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    if record.get("project_item_id"):
        if is_draft_candidate:
            # A draft candidate owns the explicit committed/verification
            # projection. Updating only the PR field left Project Workflow
            # Phase at execution and forced a manual audit/repair retry.
            updated_fields = update_project_fields(
                args,
                task,
                str(record["project_item_id"]),
                require_lifecycle_projection=True,
            )
        else:
            updated_fields = update_project_fields(args, task, str(record["project_item_id"]))
    update_issue_body(args.repo, int(record["issue_number"]), task)
    comment_url = issue_comment(
        args.repo,
        int(record["issue_number"]),
        evidence_body(
            args.task_uid,
            args.role,
            target_phase,
            {
                "Completed": "Draft Candidate Action recorded without advancing PR watch." if is_draft_candidate else "PR created and task moved to PR watch.",
                "Pending": "Wait for same-head CI receipt." if is_draft_candidate else "Watch required checks, mergeability, comments, and review threads.",
                "Action": "record-pr",
                "Validation Command": args.validation_command,
                "Expected Result": f"Task phase is {target_phase} and PR URL is mapped.",
                "Actual Result": args.pr_url,
                "Blocker / Next Action": "Obtain the same-head CI receipt, complete role review and ready closeout, then promote the draft." if is_draft_candidate else "Continue normal PR watch/fix/merge unless manual packaging hold is explicitly recorded.",
            },
        ),
    )
    record.setdefault("evidence_comments", []).append(comment_url)
    merge_task_mapping(mapping_path, args.task_uid, record)
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": target_status,
        "workflow_phase": target_phase,
        "issue_url": record.get("issue_url"),
        "pr_url": args.pr_url,
        "pr_number": record.get("pr_number"),
        "comment_url": comment_url,
        "updated_field_values": updated_fields,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"record-pr: recorded {args.pr_url} for {args.task_uid}")
    return 0


def command_set_merge_hold(args: argparse.Namespace) -> int:
    mapping_path, _mapping, record = require_record(args)
    previous = record.get("merge_hold") or {}
    if args.kind == "normal_pr_ci_watch":
        if not previous.get("active"):
            die("set-merge-hold: no active hold to clear")
        if args.resume_authority != previous.get("resume_authority"):
            die("set-merge-hold: resume authority does not match persisted task truth")
        hold = {**previous, "active": False, "cleared_at": now(), "cleared_by": args.requester, "kind": "normal_pr_ci_watch"}
    else:
        if not all((args.requester, args.reason, args.resume_authority)):
            die("set-merge-hold: active hold requires requester, reason, and resume authority")
        hold = {"kind": args.kind, "active": True, "requester": args.requester, "reason": args.reason, "resume_authority": args.resume_authority, "recorded_at": now()}
    issue_number=int(record["issue_number"]); pr_number=int(record.get("pr_number") or 0)
    if pr_number <= 0:
        die("set-merge-hold: task truth has no recorded PR")
    try:
        live_pr=json.loads(run_text(["gh","pr","view",str(pr_number),"--repo",args.repo,"--json","number,headRefOid,url"]))
    except (subprocess.SubprocessError, json.JSONDecodeError) as exc:
        die(f"set-merge-hold: live PR head readback failed: {exc}")
    head_oid=str(live_pr.get("headRefOid") or "")
    if str(live_pr.get("number") or "") != str(pr_number) or not re.fullmatch(r"[0-9a-f]{40}",head_oid,re.I):
        die("set-merge-hold: live PR identity/headRefOid readback is invalid")
    canonical = "\n".join(["<!-- oasis7-merge-hold -->",f"- task_uid: `{args.task_uid}`",f"- repository: `{args.repo}`",f"- issue_number: `{issue_number}`",f"- pr_number: `{pr_number}`",f"- head_oid: `{head_oid}`","- node_id: `merge_hold`","- kind: `merge_hold`",f"- disposition: `{'active' if hold.get('active') else 'cleared'}`",f"- hold_kind: `{hold['kind']}`",f"- active: `{str(hold.get('active',False)).lower()}`",f"- requester: `{hold.get('requester') or ''}`",f"- reason: `{hold.get('reason') or ''}`",f"- resume_authority: `{hold.get('resume_authority') or ''}`",""])
    comment_url = issue_comment(args.repo, issue_number, canonical)
    comment_id=comment_url.rsplit("issuecomment-",1)[-1]
    readback=json.loads(run_text(["gh","api",f"repos/{args.repo}/issues/comments/{comment_id}"]))
    read_body=str(readback.get("body") or "")
    if read_body != canonical: die("set-merge-hold: GitHub comment readback mismatch")
    evidence_receipt={"source":"github_task_issue_comment","runtime_verified":True,"task_uid":args.task_uid,"repository":args.repo,"issue_number":issue_number,"pr_number":pr_number,"head_oid":head_oid,"node_id":"merge_hold","kind":"merge_hold","disposition":"active" if hold.get("active") else "cleared","github_node_id":str(readback.get("id")),"url":readback.get("html_url"),"author":(readback.get("user") or {}).get("login"),"observed_at":readback.get("created_at"),"digest":hashlib.sha256(read_body.encode()).hexdigest()}
    hold["evidence_receipt"]=evidence_receipt
    record["merge_hold"] = hold
    record.setdefault("evidence_comments", []).append(comment_url)
    record["updated_at"] = now()
    update_issue_body(args.repo, int(record["issue_number"]), task_from_record(args.task_uid, record))
    merge_task_mapping(mapping_path, args.task_uid, record)
    print(json.dumps({"task_uid": args.task_uid, "merge_hold": hold, "comment_url": comment_url}, indent=2, sort_keys=True) if args.json else f"set-merge-hold: {hold['kind']}")
    return 0


def add_common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("root", type=pathlib.Path)
    parser.add_argument("--repo", default=DEFAULT_REPO)
    parser.add_argument("--project-owner", default=DEFAULT_PROJECT_OWNER)
    parser.add_argument("--project-number", type=int, default=DEFAULT_PROJECT_NUMBER)
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="GitHub Project-backed active PM task lifecycle.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    new_task = subparsers.add_parser("new-task")
    add_common(new_task)
    new_task.add_argument("--owner-role", required=True)
    new_task.add_argument("--title", required=True)
    new_task.add_argument("--module")
    new_task.add_argument("--priority", choices=("P0", "P1", "P2", "P3"), default="P2")
    new_task.add_argument("--source-signal")
    new_task.add_argument("--source-type")
    new_task.add_argument("--severity", choices=("low", "medium", "high", "critical"))
    new_task.add_argument("--source-ref", action="append", default=[], required=True)
    new_task.add_argument("--doc-ref", action="append", default=[])
    new_task.add_argument("--related-prd", action="append", default=[])
    new_task.add_argument("--acceptance", action="append", default=[])
    new_task.add_argument("--handoff-to", action="append", default=[])
    new_task.add_argument("--worktree-hint")
    new_task.add_argument("--json", action="store_true")
    new_task.set_defaults(func=command_new_task)

    append = subparsers.add_parser("append-execution-log")
    add_common(append)
    append.add_argument("--task-uid", required=True)
    append.add_argument("--role", required=True)
    append.add_argument("--completed", required=True)
    append.add_argument("--pending", required=True)
    append.add_argument("--action", required=True)
    append.add_argument("--validation-command", required=True)
    append.add_argument("--expected-result", required=True)
    append.add_argument("--actual-result", required=True)
    append.add_argument("--blocker-next-action", required=True)
    append.add_argument("--json", action="store_true")
    append.set_defaults(func=command_append_evidence)

    report = subparsers.add_parser("workflow-report")
    add_common(report)
    report.add_argument("--role", required=True)
    report.add_argument("--phase", choices=("start", "close", "review"), default="start")
    report.add_argument("--task-uid")
    report.add_argument("--stale-after-days", type=int, default=7)
    report.add_argument("--json", action="store_true")
    report.set_defaults(func=command_workflow_report)

    move = subparsers.add_parser("move-task")
    add_common(move)
    move.add_argument("--task-uid", required=True)
    move.add_argument("--to-status", required=True, choices=ALL_STATUSES)
    move.add_argument("--json", action="store_true")
    move.set_defaults(func=command_move_task)

    closeout = subparsers.add_parser("closeout-task")
    add_common(closeout)
    closeout.add_argument("--task-uid", required=True)
    closeout.add_argument("--role", required=True)
    closeout.add_argument("--to-status", required=True, choices=("ready", "done", "deferred"))
    closeout.add_argument("--claim-json", required=True)
    closeout.add_argument("--pr-receipt")
    closeout.add_argument("--json", action="store_true")
    closeout.set_defaults(func=command_closeout_task)

    phase = subparsers.add_parser("set-phase")
    add_common(phase)
    phase.add_argument("--task-uid", required=True)
    phase.add_argument("--role", default="tpm")
    phase.add_argument("--phase", required=True, choices=("main_sync",))
    phase.add_argument("--receipt-json", required=True)
    phase.add_argument("--json", action="store_true")
    phase.set_defaults(func=command_set_phase)

    refresh = subparsers.add_parser("refresh-task")
    add_common(refresh)
    refresh.add_argument("--task-uid", required=True)
    refresh.add_argument("--json", action="store_true")
    refresh.set_defaults(func=command_refresh_task)

    record_pr = subparsers.add_parser("record-pr")
    add_common(record_pr)
    record_pr.add_argument("--task-uid", required=True)
    record_pr.add_argument("--pr-url", required=True)
    record_pr.add_argument("--role", default="tpm")
    record_pr.add_argument("--validation-command", default="./scripts/prepare-task-pr.sh --create")
    record_pr.add_argument("--draft-candidate", action="store_true")
    record_pr.add_argument("--json", action="store_true")
    record_pr.set_defaults(func=command_record_pr)

    hold = subparsers.add_parser("set-merge-hold")
    add_common(hold)
    hold.add_argument("--task-uid", required=True)
    hold.add_argument("--kind", required=True, choices=("normal_pr_ci_watch", "manual_packaging_ci_hold", "user_requested_merge_hold"))
    hold.add_argument("--requester", required=True)
    hold.add_argument("--reason", default="")
    hold.add_argument("--resume-authority", required=True)
    hold.add_argument("--role", default="tpm")
    hold.add_argument("--json", action="store_true")
    hold.set_defaults(func=command_set_merge_hold)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
