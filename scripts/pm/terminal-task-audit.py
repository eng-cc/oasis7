#!/usr/bin/env python3
"""Audit one task's cross-sink terminal invariant; mutate only on explicit resume."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys


def run_json(command: list[str]) -> dict:
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        return {"query_error": result.stderr.strip() or result.stdout.strip()}
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"query_error": "invalid JSON response"}


def registered_worktrees(root: pathlib.Path) -> dict[str, str]:
    result = subprocess.run(
        ["git", "-C", str(root), "worktree", "list", "--porcelain"],
        text=True, capture_output=True, check=True,
    )
    entries: dict[str, str] = {}
    current = ""
    for line in result.stdout.splitlines():
        if line.startswith("worktree "):
            current = str(pathlib.Path(line.removeprefix("worktree ")).resolve())
            entries[current] = ""
        elif current and line.startswith("branch refs/heads/"):
            entries[current] = line.removeprefix("branch refs/heads/")
    return entries


def load(path: pathlib.Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else ""


def audit(root: pathlib.Path, task_uid: str) -> dict:
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    record = (mapping.get("tasks") or {}).get(task_uid)
    if not record:
        raise SystemExit(f"terminal-task-audit: unknown task UID: {task_uid}")
    receipt_root_result = subprocess.run(
        [sys.executable, str(root / "scripts/pm/canonical-receipt-root.py"),
         "--default-worktree", str(root), "--task-uid", task_uid],
        text=True, capture_output=True, check=True,
    )
    receipt_root = pathlib.Path(receipt_root_result.stdout.strip())
    terminal = receipt_root / "terminal-cleanup-receipt.json"
    ledger = receipt_root / "finalizer-ledger.json"
    tombstone = receipt_root / "terminal-tombstone.json"
    worktree = str(pathlib.Path(str(record.get("canonical_worktree") or "")).resolve())
    branch = str(record.get("task_branch") or "")
    registrations = registered_worktrees(root)
    worktree_present = worktree in registrations
    branch_registered_elsewhere = any(
        checked_branch == branch and checked_path != worktree
        for checked_path, checked_branch in registrations.items()
    )
    local_branch_present = subprocess.run(
        ["git", "-C", str(root), "show-ref", "--verify", "--quiet", f"refs/heads/{branch}"]
    ).returncode == 0
    remote_branch = subprocess.run(
        ["git", "-C", str(root), "ls-remote", "--heads", "origin", f"refs/heads/{branch}"],
        text=True, capture_output=True,
    )
    issue = run_json(["gh", "issue", "view", str(record.get("issue_number")),
                      "-R", str(record.get("repository")), "--json", "state,projectItems"])
    pr = run_json(["gh", "pr", "view", str(record.get("pr_number")),
                   "-R", str(record.get("repository")), "--json", "state,mergedAt,headRefName"])
    terminal_data, ledger_data, tombstone_data = load(terminal), load(ledger), load(tombstone)
    expected_identity = {
        "task_uid": task_uid, "repository": record.get("repository"),
        "issue_number": record.get("issue_number"), "pr_number": record.get("pr_number"),
    }
    terminal_identity = (
        terminal_data.get("receipt_type") == "oasis7_terminal_cleanup"
        and terminal_data.get("issuer") == "post-merge-cleanup"
        and all(str(terminal_data.get(key)) == str(value) for key, value in expected_identity.items())
        and str(pathlib.Path(str(terminal_data.get("worktree") or "")).resolve()) == worktree
        and terminal_data.get("branch") == branch
        and terminal_data.get("merge_receipt_sha256") == record.get("merge_receipt_sha256")
        and terminal_data.get("main_sync_receipt_sha256") == (record.get("phase_receipt_sha256") or {}).get("main_sync")
        and terminal_data == (record.get("phase_receipts") or {}).get("post_merge_done")
        and digest(terminal) == (record.get("phase_receipt_sha256") or {}).get("post_merge_done")
    )
    operations = ledger_data.get("operations") or {}
    project_bound = bool(record.get("project_item_id"))
    ledger_effects = ["issue_close"] + (["project_update", "evidence_comment"] if project_bound else [])
    ledger_valid = (
        ledger_data.get("schema") == "oasis7_finalizer_ledger_v1"
        and ledger_data.get("task_uid") == task_uid
        and all(
            (operations.get(effect) or {}).get("operation_id")
                == hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
            and (operations.get(effect) or {}).get("effect") == effect
            and (operations.get(effect) or {}).get("intent") is True
            and (operations.get(effect) or {}).get("readback") is True
            and (operations.get(effect) or {}).get("committed") is True
            for effect in ledger_effects
        )
    )
    project_item_identity = not project_bound
    project_fields_complete = not project_bound
    project_item_bound = not project_bound
    project_done = not project_bound
    project_live: dict = {}
    if project_bound:
        helper = root / "scripts/pm/github-project-workflow.py"
        spec = importlib.util.spec_from_file_location("terminal_audit_project", helper)
        if spec and spec.loader:
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            project_live = module.fetch_project_items_by_ids([str(record["project_item_id"])]).get(
                str(record["project_item_id"])
            ) or {}
            project = mapping.get("project") or {}
            content = project_live.get("content") or {}
            expected_url = (
                f"https://github.com/{record.get('repository')}/issues/{record.get('issue_number')}"
            )
            project_item_bound = str(project_live.get("id") or "") == str(record.get("project_item_id"))
            project_item_identity = (
                project_item_bound
                and bool(project.get("number"))
                and str(project_live.get("_project_number") or "") == str(project.get("number"))
                and str(project.get("repo") or "") == str(record.get("repository") or "")
                and str(content.get("number") or "") == str(record.get("issue_number"))
                and str(content.get("url") or "") == expected_url
                and bool(re.search(
                    rf"^task_uid:\s*{re.escape(task_uid)}$",
                    str(content.get("body") or ""),
                    re.MULTILINE,
                ))
            )
            # The Project helper returns this marker from the GraphQL pageInfo;
            # a truncated fieldValues page is not a terminal readback.
            project_fields_complete = project_live.get("_field_values_has_next_page") is False
            project_done = (
                project_item_identity
                and project_fields_complete
                and all(project_live.get(name) == value for name, value in {
                    "Status": "Done", "PM Status": "done", "Workflow Phase": "done",
                }.items())
            )
    checks = {
        "mapping_post_merge_done": record.get("workflow_phase") == "post_merge_done",
        "terminal_receipt_chain_valid": terminal_identity,
        "finalizer_ledger_committed": ledger_valid,
        "terminal_tombstone_valid": (
            tombstone_data.get("schema") == "oasis7_terminal_tombstone_v1"
            and tombstone_data.get("task_uid") == task_uid
            and all(str(tombstone_data.get(key)) == str(value) for key, value in expected_identity.items())
            and tombstone_data.get("canonical_worktree") == record.get("canonical_worktree")
            and tombstone_data.get("task_branch") == branch
            and tombstone_data.get("workflow_phase") == "post_merge_done"
            and tombstone_data.get("terminal_receipt_sha256") == digest(terminal)
            and tombstone_data.get("checkout_recreation_forbidden") is True
        ),
        "issue_closed": str(issue.get("state") or "").upper() == "CLOSED",
        "project_item_bound": project_item_bound,
        "project_item_identity": project_item_identity,
        "project_field_values_complete": project_fields_complete,
        "project_terminal": project_done,
        "pr_merged": str(pr.get("state") or "").upper() == "MERGED" and bool(pr.get("mergedAt"))
                     and pr.get("headRefName") == branch,
        "worktree_absent": not worktree_present and not pathlib.Path(worktree).exists(),
        "task_branch_not_registered_elsewhere": not branch_registered_elsewhere,
        "local_branch_absent": not local_branch_present,
        "remote_branch_absent": remote_branch.returncode == 0 and not remote_branch.stdout.strip(),
    }
    drift = [name for name, ok in checks.items() if not ok]
    return {
        "schema": "oasis7_terminal_task_audit_v1",
        "task_uid": task_uid,
        "status": "reconciled" if not drift else "drifted",
        "checks": checks,
        "drift": drift,
        "task": {key: record.get(key) for key in
                 ("repository", "issue_number", "pr_number", "status", "workflow_phase",
                  "canonical_worktree", "task_branch")},
        "live": {"issue": issue, "pr": pr, "project_item": project_live},
        "receipt_root": str(receipt_root),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--resume-finalizer", action="store_true")
    args = parser.parse_args()
    root = pathlib.Path(subprocess.check_output(
        ["git", "-C", args.repo_root, "rev-parse", "--show-toplevel"], text=True
    ).strip()).resolve()
    result = audit(root, args.task_uid)
    if args.resume_finalizer and result["status"] != "reconciled":
        pr_number = result["task"].get("pr_number")
        if not pr_number or not result["checks"]["pr_merged"]:
            raise SystemExit("terminal-task-audit: refusing resume without bound merged PR")
        subprocess.run(
            [str(root / "scripts/pm/finalize-task.sh"), "--repo-root", str(root),
             "--task-uid", args.task_uid, "--pr", str(pr_number), "--resume", "--json"],
            check=True,
        )
        result = audit(root, args.task_uid)
    print(json.dumps(result, indent=2, sort_keys=True) if args.json else
          f"{result['task_uid']}: {result['status']} ({', '.join(result['drift']) or 'no drift'})")
    return 0 if result["status"] == "reconciled" else 1


if __name__ == "__main__":
    raise SystemExit(main())
