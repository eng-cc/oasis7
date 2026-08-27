#!/usr/bin/env python3
"""Audit one task's cross-sink terminal invariant; mutate only on explicit resume."""
from __future__ import annotations

import argparse
import json
import pathlib
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


def registered_worktrees(root: pathlib.Path) -> set[str]:
    result = subprocess.run(
        ["git", "-C", str(root), "worktree", "list", "--porcelain"],
        text=True, capture_output=True, check=True,
    )
    return {
        line.removeprefix("worktree ")
        for line in result.stdout.splitlines()
        if line.startswith("worktree ")
    }


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
    worktree_present = worktree in registered_worktrees(root)
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
    tombstone_data = json.loads(tombstone.read_text(encoding="utf-8")) if tombstone.is_file() else {}
    checks = {
        "mapping_post_merge_done": record.get("workflow_phase") == "post_merge_done",
        "terminal_receipt_present": terminal.is_file(),
        "finalizer_ledger_present": ledger.is_file(),
        "terminal_tombstone_valid": (
            tombstone_data.get("schema") == "oasis7_terminal_tombstone_v1"
            and tombstone_data.get("task_uid") == task_uid
            and tombstone_data.get("checkout_recreation_forbidden") is True
        ),
        "issue_closed": str(issue.get("state") or "").upper() == "CLOSED",
        "pr_merged": str(pr.get("state") or "").upper() == "MERGED",
        "worktree_absent": not worktree_present,
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
        "live": {"issue": issue, "pr": pr},
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
