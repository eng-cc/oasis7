#!/usr/bin/env python3
"""Record and read back task/head/base identity before draft PR side effects."""
from __future__ import annotations
import argparse, json, os, re, subprocess, sys, tempfile
from datetime import datetime, timezone
from pathlib import Path

MARKER = "<!-- oasis7-pm-evidence -->"
KEYS = ("Task UID", "Source Worktree", "Source Branch", "Source Head", "Comparison Ref", "Comparison OID")

def fail(message: str) -> None:
    raise SystemExit(f"draft freeze evidence: {message}")

def git(worktree: Path, *args: str) -> str:
    try:
        return subprocess.check_output(["git", "-C", str(worktree), *args], text=True, stderr=subprocess.DEVNULL).strip()
    except subprocess.CalledProcessError:
        fail(f"cannot resolve git identity: {' '.join(args)}")
    return ""

def parse_fields(body: str) -> dict[str, str]:
    result = {}
    for key in KEYS:
        match = re.search(rf"^{re.escape(key)}:\s*(.+)$", body, re.MULTILINE)
        if match:
            result[key] = match.group(1).strip().strip("`")
    return result

def main() -> int:
    parser = argparse.ArgumentParser()
    for name in ("worktree", "branch", "head", "comparison-ref", "comparison-oid"):
        parser.add_argument(f"--{name}", required=True)
    args = parser.parse_args()
    worktree = Path(args.worktree).resolve()
    try:
        mapping = json.loads((worktree / ".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read canonical task mapping: {exc}")
    tasks = mapping.get("tasks") if isinstance(mapping, dict) else None
    if not isinstance(tasks, dict): fail("canonical task mapping has no tasks object")
    resolved = os.path.realpath(worktree)
    hits = [(str(uid), record) for uid, record in tasks.items() if isinstance(record, dict)
            and os.path.realpath(str(record.get("canonical_worktree") or "")) == resolved
            and str(record.get("task_branch") or "") == args.branch]
    if len(hits) != 1: fail(f"expected one canonical worktree/branch mapping, found {len(hits)}")
    task_uid, record = hits[0]
    if not re.fullmatch(r"task_[0-9a-f]{32}", task_uid) or record.get("task_uid") != task_uid: fail("canonical task UID is invalid")
    issue = str(record.get("issue_number") or "")
    repo = str((mapping.get("project") or {}).get("repo") or record.get("repository") or "")
    if not issue.isdigit() or not re.fullmatch(r"[^/\s]+/[^/\s]+", repo): fail("canonical issue or repository is invalid")
    try:
        live_issue = json.loads(subprocess.check_output(
            ["gh", "issue", "view", issue, "-R", repo, "--json", "body,number,url"], text=True
        ))
    except (subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        fail(f"cannot validate bound issue identity: {exc}")
    live_body = str(live_issue.get("body") or "") if isinstance(live_issue, dict) else ""
    live_number = live_issue.get("number") if isinstance(live_issue, dict) else None
    live_url = str(live_issue.get("url") or "") if isinstance(live_issue, dict) else ""
    if (live_number != int(issue) or not live_url.endswith(f"/issues/{issue}")
            or "<!-- oasis7-pm-task -->" not in live_body
            or not re.search(rf"(?m)^task_uid:\s*{re.escape(task_uid)}\s*$", live_body)):
        fail("live issue identity does not match canonical task mapping")
    if not re.fullmatch(r"[0-9a-f]{40}", args.head) or not re.fullmatch(r"[0-9a-f]{40}", args.comparison_oid): fail("head or comparison OID is invalid")
    if git(worktree, "rev-parse", "--verify", "HEAD^{commit}") != args.head: fail("worktree HEAD differs from frozen source head")
    if git(worktree, "rev-parse", "--verify", f"refs/heads/{args.branch}^{{commit}}") != args.head: fail("task branch differs from frozen source head")
    if git(worktree, "rev-parse", "--verify", f"{args.comparison_ref}^{{commit}}") != args.comparison_oid: fail("comparison ref differs from frozen comparison OID")
    expected = {"Task UID": task_uid, "Source Worktree": str(worktree), "Source Branch": args.branch,
                "Source Head": args.head, "Comparison Ref": args.comparison_ref, "Comparison OID": args.comparison_oid}
    body = "\n".join([MARKER, f"Task UID: {task_uid}", "Evidence Phase: draft_candidate_freeze", "Role: tpm",
                       f"Recorded At: {datetime.now(timezone.utc).isoformat()}", "",
                       *(f"{key}: {expected[key]}" for key in KEYS if key != "Task UID"), ""])
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body); body_path = Path(handle.name)
    try:
        comment_url = subprocess.check_output(
            ["gh", "issue", "comment", issue, "-R", repo, "--body-file", str(body_path)], text=True
        ).strip()
    except subprocess.CalledProcessError as exc: fail(f"cannot record frozen identity on bound issue: {exc}")
    finally: body_path.unlink(missing_ok=True)
    if f"/{repo}/issues/{issue}#issuecomment-" not in comment_url:
        fail("frozen identity writer returned an unexpected comment identity")
    try:
        payload = json.loads(subprocess.check_output(["gh", "issue", "view", issue, "-R", repo, "--json", "comments"], text=True))
    except (subprocess.CalledProcessError, json.JSONDecodeError) as exc: fail(f"cannot read back bound issue comments: {exc}")
    comments = payload.get("comments") if isinstance(payload, dict) else None
    if not isinstance(comments, list) or expected not in [parse_fields(str(item.get("body") or "")) for item in comments if isinstance(item, dict) and MARKER in str(item.get("body") or "")]:
        fail("written frozen identity was not observed on bound issue readback")
    return 0

if __name__ == "__main__": sys.exit(main())
