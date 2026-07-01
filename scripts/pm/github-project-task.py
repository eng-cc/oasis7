#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
import json
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


def die(message: str) -> None:
    print(f"github-project-task: {message}", file=sys.stderr)
    raise SystemExit(1)


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


def load_mapping(path: pathlib.Path) -> dict[str, Any]:
    if not path.exists():
        return {"version": 1, "tasks": {}}
    return json.loads(path.read_text(encoding="utf-8"))


def save_mapping(path: pathlib.Path, mapping: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(mapping, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def mapping_path_for(root: pathlib.Path, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    return path if path.is_absolute() else root / path


def run_text(cmd: list[str]) -> str:
    result = subprocess.run(cmd, check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=180)
    return result.stdout.strip()


def issue_number_from_url(issue_url: str) -> int:
    try:
        return int(issue_url.rstrip("/").rsplit("/", 1)[-1])
    except ValueError as exc:
        raise RuntimeError(f"cannot parse issue number from {issue_url}") from exc


def pr_number_from_url(pr_url: str) -> int | None:
    match = re.search(r"/pull/(\d+)(?:$|[?#])", pr_url)
    return int(match.group(1)) if match else None


def issue_task_fields(body: str) -> dict[str, Any]:
    fields: dict[str, Any] = {}
    for key in ("owner_role", "module", "status", "priority", "worktree_hint", "source_signal", "source_type", "severity"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    for key in ("pr_url", "pr_number"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    source_refs = re.findall(r"^- `([^`]+)`$", body, re.MULTILINE)
    if source_refs:
        fields["source_refs"] = source_refs
    acceptance_match = re.search(r"^Acceptance:\n(?P<body>(?:^- .+\n?)+)", body, re.MULTILINE)
    if acceptance_match:
        fields["acceptance"] = [
            line[2:].strip()
            for line in acceptance_match.group("body").splitlines()
            if line.startswith("- ")
        ]
    return fields


def github_issue_record(repo: str, task_uid: str) -> dict[str, Any] | None:
    search_payload = run_text(
        [
            "gh",
            "issue",
            "list",
            "-R",
            repo,
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
    body = str(issue.get("body") or "")
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
            ("priority", record.get("priority") or "P2"),
            ("source_signal", record.get("source_signal") or ""),
            ("source_type", record.get("source_type") or ""),
            ("severity", record.get("severity") or ""),
            ("pr_url", record.get("pr_url") or record.get("pull_request_url") or ""),
            ("pr_number", record.get("pr_number") or ""),
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


def update_project_fields(args: argparse.Namespace, task: OrderedDict[str, Any], project_item_id: str) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields)
    if skipped:
        print(f"github-project-task: skipped fields: {', '.join(skipped)}", file=sys.stderr)
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
    mapping = load_mapping(mapping_path)
    mapping.setdefault("tasks", {})
    task_uid = f"task_{uuid.uuid4().hex}"
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
    issue_url = create_issue(args.repo, task)
    issue_number = issue_number_from_url(issue_url)
    item_id = add_project_item(args, issue_url)
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
    }
    mapping["tasks"][task_uid] = record
    project = dict(mapping.get("project") or {})
    project.update({"owner": args.project_owner, "number": args.project_number, "repo": args.repo})
    mapping["project"] = project
    save_mapping(mapping_path, mapping)
    payload = dict(record)
    payload.update(
        {
            "task_path": issue_url,
            "execution_log_path": issue_url,
            "updated_field_values": updated_fields,
            "mapping_path": str(mapping_path),
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
    save_mapping(mapping_path, mapping)
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
    timestamp_key = "last_started_at" if args.phase == "start" else "last_closed_at"
    record[timestamp_key] = now()
    record["last_evidence_at"] = now()
    record["updated_at"] = now()
    record.setdefault("evidence_comments", []).append(comment_url)
    save_mapping(mapping_path, mapping)
    payload = {
        "task_uid": args.task_uid,
        "role": args.role,
        "phase": args.phase,
        "status": "ok",
        "issue_url": record.get("issue_url"),
        "execution_log_path": record.get("issue_url"),
        "comment_url": comment_url,
        timestamp_key: record[timestamp_key],
    }
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
    record["status"] = args.to_status
    record["updated_at"] = now()
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    if record.get("project_item_id"):
        updated_fields = update_project_fields(args, task, str(record["project_item_id"]))
    update_issue_body(args.repo, int(record["issue_number"]), task)
    if record.get("_github_source") != "issue_search" or mapping_path.exists():
        save_mapping(mapping_path, mapping)
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


def command_record_pr(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    previous = str(record.get("status") or "")
    record["pr_url"] = args.pr_url
    number = pr_number_from_url(args.pr_url)
    if number is not None:
        record["pr_number"] = number
    record["status"] = "pr_watch"
    record["updated_at"] = now()
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    if record.get("project_item_id"):
        updated_fields = update_project_fields(args, task, str(record["project_item_id"]))
    update_issue_body(args.repo, int(record["issue_number"]), task)
    comment_url = issue_comment(
        args.repo,
        int(record["issue_number"]),
        evidence_body(
            args.task_uid,
            args.role,
            "pr_watch",
            {
                "Completed": "PR created and task moved to PR watch.",
                "Pending": "Watch required checks, mergeability, comments, and review threads.",
                "Action": "record-pr",
                "Validation Command": args.validation_command,
                "Expected Result": "Task status is pr_watch and PR URL is mapped.",
                "Actual Result": args.pr_url,
                "Blocker / Next Action": "Continue normal PR watch/fix/merge unless manual packaging hold is explicitly recorded.",
            },
        ),
    )
    record.setdefault("evidence_comments", []).append(comment_url)
    if record.get("_github_source") != "issue_search" or mapping_path.exists():
        save_mapping(mapping_path, mapping)
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": "pr_watch",
        "issue_url": record.get("issue_url"),
        "pr_url": args.pr_url,
        "pr_number": record.get("pr_number"),
        "comment_url": comment_url,
        "updated_field_values": updated_fields,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"record-pr: recorded {args.pr_url} for {args.task_uid}")
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

    record_pr = subparsers.add_parser("record-pr")
    add_common(record_pr)
    record_pr.add_argument("--task-uid", required=True)
    record_pr.add_argument("--pr-url", required=True)
    record_pr.add_argument("--role", default="tpm")
    record_pr.add_argument("--validation-command", default="./scripts/prepare-task-pr.sh --create")
    record_pr.add_argument("--json", action="store_true")
    record_pr.set_defaults(func=command_record_pr)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
