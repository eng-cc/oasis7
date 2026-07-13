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
from collections import OrderedDict
from datetime import datetime
from typing import Any


DEFAULT_REPO = "eng-cc/oasis7"
DEFAULT_PROJECT_OWNER = "eng-cc"
DEFAULT_PROJECT_NUMBER = 1


def now() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def die(message: str) -> None:
    print(f"audit-pr-watch-issues: {message}", file=sys.stderr)
    raise SystemExit(1)


def run_text(cmd: list[str]) -> str:
    result = subprocess.run(cmd, check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=180)
    return result.stdout.strip()


def load_module(path: pathlib.Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        die(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_mapping(root: pathlib.Path, mapping_arg: str) -> tuple[pathlib.Path, dict[str, Any]]:
    mapping_path = pathlib.Path(mapping_arg)
    if not mapping_path.is_absolute():
        mapping_path = root / mapping_path
    if not mapping_path.exists():
        return mapping_path, {"version": 1, "tasks": {}}
    return mapping_path, json.loads(mapping_path.read_text(encoding="utf-8"))


def save_mapping(path: pathlib.Path, mapping: dict[str, Any]) -> None:
    if not path.exists():
        return
    path.write_text(json.dumps(mapping, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def parse_task_body(body: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    if "<!-- oasis7-pm-task -->" not in body:
        return fields
    uid_match = re.search(r"^task_uid:\s*(task_[0-9a-f]{32})$", body, re.MULTILINE)
    if uid_match:
        fields["task_uid"] = uid_match.group(1)
    for key in ("owner_role", "module", "status", "priority", "worktree_hint", "pr_url", "pr_number"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    if "manual_packaging_ci_hold" in body or "manual_post_merge_hold" in body:
        fields["manual_hold"] = "true"
    return fields


def issue_view(repo: str, number: int) -> dict[str, Any]:
    payload = run_text(["gh", "issue", "view", str(number), "-R", repo, "--json", "body,comments,number,state,title,url"])
    data = json.loads(payload)
    if not isinstance(data, dict):
        die(f"gh issue view returned non-object for #{number}")
    return data


def list_issue_numbers(repo: str, limit: int) -> list[int]:
    query = '"oasis7-pm-task" "pr_watch" "pr_number" in:body'
    payload = run_text(
        [
            "gh",
            "issue",
            "list",
            "-R",
            repo,
            "--state",
            "all",
            "--search",
            query,
            "--json",
            "number,state,title,url",
            "--limit",
            str(limit),
        ]
    )
    items = json.loads(payload)
    if not isinstance(items, list):
        die("gh issue list returned non-list payload")
    numbers: list[int] = []
    for item in items:
        if isinstance(item, dict) and item.get("number"):
            numbers.append(int(item["number"]))
    return numbers


def broad_rate_limit_guard() -> dict[str, Any]:
    try:
        payload=json.loads(run_text(["gh","api","graphql","-f","query=query { rateLimit { remaining resetAt } }"]))
        rate=((payload.get("data") or {}).get("rateLimit") or {})
    except Exception as exc:
        return {"status":"capability_blocked","reason":"graphql_rate_limit_unavailable","error":str(exc),"resumable":True}
    remaining,reset_at=rate.get("remaining"),str(rate.get("resetAt") or "")
    if not isinstance(remaining,int) or not reset_at:
        return {"status":"capability_blocked","reason":"graphql_rate_limit_unknown","resumable":True}
    if remaining < 100:
        return {"status":"capability_blocked","reason":"graphql_budget_insufficient","remaining":remaining,"resetAt":reset_at,"resumable":True}
    return {"status":"ok","remaining":remaining,"resetAt":reset_at}


def pr_view(repo: str, number: int) -> dict[str, Any]:
    payload = run_text(["gh", "pr", "view", str(number), "-R", repo, "--json", "number,state,mergedAt,url,title"])
    data = json.loads(payload)
    if not isinstance(data, dict):
        die(f"gh pr view returned non-object for #{number}")
    return data


def issue_comment(repo: str, issue_number: int, body: str) -> str:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = handle.name
    try:
        return run_text(["gh", "issue", "comment", str(issue_number), "-R", repo, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def update_issue_body(repo: str, issue_number: int, body: str) -> None:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = handle.name
    try:
        run_text(["gh", "issue", "edit", str(issue_number), "-R", repo, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def evidence_body(task_uid: str, pr_number: int, pr_url: str, issue_state: str) -> str:
    return "\n".join(
        [
            "<!-- oasis7-pm-evidence -->",
            f"Task UID: {task_uid}",
            "Evidence Phase: task_done",
            "Role: tpm",
            f"Recorded At: {now()}",
            "",
            "Completed: Remedial PM metadata synchronization confirmed the recorded PR is merged and advanced task truth to task_done.",
            "Pending: main sync, safe cleanup receipt, and post-merge finalization.",
            "Action: audit-pr-watch-issues --close",
            f"Validation Command: gh pr view {pr_number} --json state,mergedAt,url",
            "Expected Result: PR is merged before the task_done transition.",
            f"Actual Result: {pr_url or 'PR merged'}",
            f"Previous Issue State: {issue_state}",
            "Completion Judgment: not provided by this audit; this records metadata synchronization after existing ready/pre-PR evidence and a merged PR.",
            "Blocker / Next Action: Continue the canonical terminal runbook at post-merge-main-sync.sh; only post-merge-finalize.py may close the issue.",
            "",
        ]
    )


def issue_has_required_ready_evidence(issue: dict[str, Any], task_uid: str) -> bool:
    comments = issue.get("comments") or []
    texts = [str(issue.get("body") or "")]
    if isinstance(comments, list):
        texts.extend(str(comment.get("body") or "") for comment in comments if isinstance(comment, dict))

    def has_packet(*markers: str) -> bool:
        required = (f"Task UID: {task_uid}", *markers)
        return any(all(marker in text for marker in required) for text in texts)

    has_review = has_packet("Pre-PR Local Role Review: passed")
    has_ready_claim = has_packet("Claim Type: ready_for_pr", "Verification Status: verified")
    has_pre_pr_ready_evidence = has_packet("<!-- oasis7-pm-evidence -->", "Evidence Phase: pre_pr_ready")
    return has_review and has_ready_claim and has_pre_pr_ready_evidence


def done_task_from_record(task_mod: Any, task_uid: str, record: dict[str, Any], fields: dict[str, str], issue: dict[str, Any]) -> OrderedDict[str, Any]:
    merged = dict(record)
    merged.update({k: v for k, v in fields.items() if v})
    merged["task_uid"] = task_uid
    merged["issue_number"] = int(issue["number"])
    merged["issue_url"] = str(issue.get("url") or record.get("issue_url") or "")
    merged["title"] = str(record.get("title") or issue.get("title") or "")
    if str(merged["title"]).startswith("[PM] "):
        merged["title"] = str(merged["title"])[5:]
    merged["status"] = "done"
    merged["workflow_phase"] = "task_done"
    merged["updated_at"] = now()
    return task_mod.task_from_record(task_uid, merged)


def update_done_project_fields(sync_mod: Any, args: argparse.Namespace, task: OrderedDict[str, Any], project_item_id: str) -> int:
    project_id, project_fields = sync_mod.project_context(args.project_owner, args.project_number)
    required = {"Status", "PM Status", "Workflow Phase"}
    values = sync_mod.project_field_values(task)
    missing: list[str] = []
    for field_name in sorted(required):
        field = project_fields.get(field_name)
        value = str(values.get(field_name) or "")
        if not field:
            missing.append(f"{field_name}:missing_field")
        elif not value:
            missing.append(f"{field_name}:empty_value")
        elif field_name in sync_mod.SINGLE_SELECT_FIELDS and value not in (field.get("options_by_name") or {}):
            missing.append(f"{field_name}:missing_option:{value}")
    if missing:
        raise RuntimeError("required done Project fields unavailable: " + ", ".join(missing))
    updated, skipped = sync_mod.update_fields(project_id, project_item_id, task, project_fields, only_fields=required)
    if skipped or int(updated) != len(required):
        raise RuntimeError(f"required done Project fields not fully updated: updated={updated}/{len(required)} skipped={','.join(skipped)}")
    return int(updated)


def candidate_issue_numbers(mapping: dict[str, Any], listed_numbers: list[int]) -> list[int]:
    numbers: list[int] = []
    for record in (mapping.get("tasks") or {}).values():
        if str(record.get("status") or "") != "pr_watch":
            continue
        number = str(record.get("issue_number") or "")
        if number.isdigit():
            numbers.append(int(number))
    numbers.extend(listed_numbers)
    return sorted(set(numbers))


def audit(args: argparse.Namespace) -> list[dict[str, Any]]:
    root = args.root.resolve()
    mapping_path, mapping = load_mapping(root, args.mapping)
    task_mod = load_module(root / "scripts/pm/github-project-task.py", "github_project_task_impl")
    sync_mod = load_module(root / "scripts/pm/github-project-sync.py", "github_project_sync_impl")

    selected = (mapping.get("tasks") or {}).get(args.task_uid) if args.task_uid else None
    if args.task_uid and not selected:
        return [{"task_uid": args.task_uid, "status": "blocked", "reason": "selected task is missing from local mapping; refresh it explicitly"}]
    listed = [] if args.task_uid else list_issue_numbers(args.repo, args.limit)
    if selected:
        issue_number = str(selected.get("issue_number") or "")
        listed = [int(issue_number)] if issue_number.isdigit() else []
    results: list[dict[str, Any]] = []
    issue_numbers = listed if args.task_uid else candidate_issue_numbers(mapping, listed)
    for issue_number in issue_numbers:
        try:
            issue = issue_view(args.repo, issue_number)
        except (subprocess.CalledProcessError, json.JSONDecodeError) as exc:
            results.append({"issue_number": issue_number, "status": "error", "reason": f"issue view failed: {exc}"})
            continue
        body = str(issue.get("body") or "")
        fields = parse_task_body(body)
        task_uid = fields.get("task_uid") or ""
        if args.task_uid and task_uid != args.task_uid:
            results.append({"issue_number":issue_number,"task_uid":task_uid or None,
                            "status":"blocked","reason":"selected issue task_uid does not match requested task"})
            continue
        pr_number_raw = fields.get("pr_number") or ""
        record = (mapping.get("tasks") or {}).get(task_uid) or {}
        result: dict[str, Any] = {
            "issue_number": issue_number,
            "issue_state": issue.get("state"),
            "issue_url": issue.get("url"),
            "task_uid": task_uid or None,
            "pr_number": pr_number_raw or None,
        }
        if not task_uid or fields.get("status") != "pr_watch" or not pr_number_raw.isdigit():
            result.update({"status": "skipped", "reason": "not a PM task issue in pr_watch with pr_number"})
            results.append(result)
            continue
        if fields.get("manual_hold") == "true":
            result.update({"status": "skipped", "reason": "manual hold marker present"})
            results.append(result)
            continue
        if not issue_has_required_ready_evidence(issue, task_uid):
            result.update({"status": "blocked", "reason": "missing existing pre-PR review plus ready closeout/claim evidence"})
            results.append(result)
            continue
        pr_number = int(pr_number_raw)
        try:
            pr = pr_view(args.repo, pr_number)
        except (subprocess.CalledProcessError, json.JSONDecodeError) as exc:
            result.update({"status": "blocked", "reason": f"PR view failed: {exc}"})
            results.append(result)
            continue
        result["pr_state"] = pr.get("state")
        result["pr_url"] = pr.get("url")
        if str(pr.get("state") or "").upper() != "MERGED" and not pr.get("mergedAt"):
            result.update({"status": "skipped", "reason": "recorded PR is not merged"})
            results.append(result)
            continue
        project_item_id = str(record.get("project_item_id") or "")
        if not project_item_id:
            result.update({"status": "blocked", "reason": "missing project_item_id; refusing to advance to task_done"})
            results.append(result)
            continue
        if not args.close:
            result.update({"status": "would_advance", "reason": "merged PR watch task can advance to task_done; continue the terminal runbook"})
            results.append(result)
            continue
        try:
            task = done_task_from_record(task_mod, task_uid, record, fields, issue)
            updated_fields = update_done_project_fields(sync_mod, args, task, project_item_id)
            update_issue_body(args.repo, issue_number, task_mod.issue_body(task))
            comment_url = issue_comment(args.repo, issue_number, evidence_body(task_uid, pr_number, str(pr.get("url") or ""), str(issue.get("state") or "")))
            record.update(
                {
                    "status": "done",
                    "workflow_phase": "task_done",
                    "updated_at": now(),
                    "last_closed_at": now(),
                    "last_evidence_at": now(),
                    "issue_url": str(issue.get("url") or record.get("issue_url") or ""),
                    "issue_number": issue_number,
                    "pr_number": pr_number,
                    "pr_url": str(pr.get("url") or record.get("pr_url") or ""),
                }
            )
            record.setdefault("evidence_comments", []).append(comment_url)
            mapping.setdefault("tasks", {})[task_uid] = record
            save_mapping(mapping_path, mapping)
        except (RuntimeError, subprocess.CalledProcessError) as exc:
            result.update({"status": "blocked", "reason": str(exc)})
            results.append(result)
            continue
        result.update({"status": "task_done", "comment_url": comment_url, "updated_field_values": updated_fields,
                       "next_action": "run canonical terminal runbook from post-merge-main-sync.sh"})
        results.append(result)
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit GitHub-backed pr_watch task issues whose recorded PR has merged.")
    parser.add_argument("root", type=pathlib.Path)
    parser.add_argument("--repo", default=DEFAULT_REPO)
    parser.add_argument("--project-owner", default=DEFAULT_PROJECT_OWNER)
    parser.add_argument("--project-number", type=int, default=DEFAULT_PROJECT_NUMBER)
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")
    parser.add_argument("--limit", type=int, default=100)
    parser.add_argument("--task-uid", help="audit exactly one mapped pr_watch task; avoids repository issue listing")
    parser.add_argument("--global-maintenance", action="store_true", help="explicitly authorize guarded repository-wide audit")
    parser.add_argument("--close", action="store_true", help="Remedially advance merged pr_watch tasks to task_done; terminal finalizer closes issues")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    if not args.task_uid and not args.global_maintenance:
        die("--task-uid is required by default; use --global-maintenance for guarded broad audit")
    if args.global_maintenance:
        budget=broad_rate_limit_guard()
        if budget["status"] != "ok":
            print(json.dumps(budget,indent=2,sort_keys=True)); return 2

    results = audit(args)
    payload = {"status": "ok", "close": args.close, "results": results}
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        for item in results:
            print(
                f"#{item.get('issue_number')} {item.get('status')}: "
                f"task={item.get('task_uid') or '-'} pr={item.get('pr_number') or '-'} "
                f"{item.get('reason') or item.get('comment_url') or ''}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
