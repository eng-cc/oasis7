#!/usr/bin/env python3
"""Terminalize a verified linked-delivery coordinator without inventing a PR.

The durable receipt is written before remote effects. Re-running this command
revalidates the live aggregate chain and resumes the remaining effects.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import pathlib
import subprocess
import sys

try:
    import fcntl
except ImportError:  # Git for Windows runs the native Python interpreter.
    fcntl = None
    import msvcrt


def fail(message: str) -> None:
    raise SystemExit(f"finalize-aggregate-task: {message}")


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()


def read_json(path: pathlib.Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"JSON object required: {path}")
    return value


def command(*args: str) -> str:
    result = subprocess.run(args, text=True, capture_output=True)
    if result.returncode:
        fail(f"command failed: {' '.join(args)}: {(result.stderr or result.stdout).strip()}")
    return result.stdout


def write_json_atomic(path: pathlib.Path, value: dict) -> None:
    staged = path.with_suffix(path.suffix + ".tmp")
    staged.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    staged.replace(path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=pathlib.Path, required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--record", type=pathlib.Path, required=True)
    parser.add_argument("--candidate", type=pathlib.Path, required=True)
    parser.add_argument("--evidence", type=pathlib.Path, required=True)
    parser.add_argument("--receipt", type=pathlib.Path, required=True)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    root = args.repo_root.resolve(strict=True)
    if pathlib.Path(command("git", "-C", str(root), "rev-parse", "--show-toplevel").strip()).resolve() != root:
        fail("--repo-root must be the canonical default worktree")
    inputs = [path.resolve(strict=True) for path in (args.record, args.candidate, args.evidence, args.receipt)]
    mapping = read_json(root / ".pm/github-project-sync/tasks.json")
    task = (mapping.get("tasks") or {}).get(args.task_uid)
    if not isinstance(task, dict):
        fail("task is absent from canonical mapping; refresh task cache first")
    if task.get("completion_mode") != "ordered_delivery_aggregate" or task.get("pr_number") or task.get("pr_url"):
        fail("coordinator route or singular PR identity is invalid")
    if task.get("status") != "done" or task.get("workflow_phase") not in {"task_done", "post_merge_done"}:
        fail("coordinator has no verified task_done transition")
    completion_sha = hashlib.sha256(inputs[3].read_bytes()).hexdigest()
    if task.get("aggregate_completion_receipt_sha256") != completion_sha:
        fail("aggregate completion receipt differs from task truth")
    repository = str(task.get("repository") or "")
    issue_number = task.get("issue_number")
    if not repository or not isinstance(issue_number, int):
        fail("coordinator repository/Issue identity is incomplete")
    issue = json.loads(command("gh", "issue", "view", str(issue_number), "-R", repository, "--json", "state,body"))
    if f"task_uid: {args.task_uid}" not in str(issue.get("body") or ""):
        fail("live coordinator Issue Task UID mismatch")
    if task.get("workflow_phase") == "task_done" and issue.get("state") != "OPEN":
        fail("coordinator Issue closed before terminal proof")
    if task.get("workflow_phase") == "post_merge_done" and issue.get("state") == "CLOSED":
        # The aggregate validator deliberately requires an open coordinator.
        # A completed retry instead checks the durable terminal receipt below.
        pass
    else:
        if issue.get("state") != "OPEN":
            fail("coordinator Issue state is unsupported")
        command(sys.executable, str(root / "scripts/pm/aggregate-task-completion.py"), "validate",
                "--repo-root", str(root), "--task-uid", args.task_uid,
                "--record", str(inputs[0]), "--candidate", str(inputs[1]),
                "--evidence", str(inputs[2]), "--receipt", str(inputs[3]), "--json")
    if args.preflight and not (task.get("workflow_phase") == "post_merge_done" and issue.get("state") == "CLOSED"):
        print(json.dumps({"status": "ready", "task_uid": args.task_uid, "issue_number": issue_number,
                          "aggregate_completion_receipt_sha256": completion_sha}, sort_keys=True))
        return 0

    common = pathlib.Path(command("git", "-C", str(root), "rev-parse", "--git-common-dir").strip())
    if not common.is_absolute():
        common = (root / common).resolve()
    durable = common / "oasis7-workflow-receipts" / args.task_uid
    durable.mkdir(parents=True, exist_ok=True)
    lock = (durable / "aggregate-finalizer.lock").open("a+b")
    if fcntl is not None:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
    else:
        lock.write(b"\0")
        lock.flush()
        lock.seek(0)
        msvcrt.locking(lock.fileno(), msvcrt.LK_LOCK, 1)
    terminal_path = durable / "aggregate-terminal-receipt.json"
    if terminal_path.exists():
        terminal = read_json(terminal_path)
    else:
        payload = {
            "schema": "oasis7.aggregate-terminal/v1",
            "receipt_type": "oasis7_aggregate_terminal",
            "issuer": "aggregate-task-finalizer",
            "task_uid": args.task_uid,
            "repository": repository,
            "issue_number": issue_number,
            "aggregate_completion_receipt_sha256": completion_sha,
            "plan_comment_id": read_json(inputs[3]).get("plan_comment_id"),
            "observed_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        }
        terminal = {**payload, "receipt_sha256": digest(payload)}
        write_json_atomic(terminal_path, terminal)
    payload = {key: value for key, value in terminal.items() if key != "receipt_sha256"}
    if terminal.get("receipt_sha256") != digest(payload):
        fail("durable terminal receipt digest mismatch")
    for key, expected in (("task_uid", args.task_uid), ("repository", repository),
                          ("issue_number", issue_number), ("aggregate_completion_receipt_sha256", completion_sha)):
        if terminal.get(key) != expected:
            fail(f"durable terminal receipt {key} mismatch")
    terminal_sha = hashlib.sha256(terminal_path.read_bytes()).hexdigest()
    journal_path = durable / "aggregate-terminal-effects.json"
    if journal_path.exists():
        journal = read_json(journal_path)
        if (journal.get("schema") != "oasis7.aggregate-terminal-effects/v1"
                or journal.get("task_uid") != args.task_uid
                or journal.get("terminal_receipt_sha256") != terminal_sha
                or any(type(journal.get(key)) is not bool for key in
                       ("phase_readback", "project_readback", "issue_closed_readback"))):
            fail("durable terminal effect journal identity mismatch")
    else:
        journal = {"schema": "oasis7.aggregate-terminal-effects/v1", "task_uid": args.task_uid,
                   "terminal_receipt_sha256": terminal_sha, "phase_readback": False,
                   "project_readback": False, "issue_closed_readback": False}
        write_json_atomic(journal_path, journal)
    if task.get("workflow_phase") == "post_merge_done" and issue.get("state") == "CLOSED":
        if (task.get("phase_receipt_sha256") or {}).get("post_merge_done") != terminal_sha:
            fail("completed coordinator terminal receipt did not read back")
        audit = json.loads(command(sys.executable, str(root / "scripts/pm/github-project-workflow.py"),
                                   str(root), "audit", "--task-uid", args.task_uid, "--include-done", "--json"))
        if audit.get("status") != "ok" or audit.get("selected_count") != 1:
            fail("completed coordinator Project audit did not read back")
        journal.update(phase_readback=True, project_readback=True, issue_closed_readback=True)
        write_json_atomic(journal_path, journal)
        lock.close()
        print(json.dumps({"status": "already_finalized" if args.preflight else "finalized", "task_uid": args.task_uid,
                          "terminal_receipt": str(terminal_path)}, sort_keys=True))
        return 0

    if task.get("workflow_phase") == "task_done":
        command(sys.executable, str(root / "scripts/pm/github-project-task.py"), "set-phase", str(root),
                "--task-uid", args.task_uid, "--phase", "post_merge_done", "--role", "tpm",
                "--receipt-json", str(terminal_path), "--aggregate-plan", str(inputs[0]),
                "--aggregate-candidate", str(inputs[1]), "--aggregate-evidence", str(inputs[2]),
                "--aggregate-receipt", str(inputs[3]), "--json")
    # Re-read task and Issue after the Project/Issue phase transition. A retry
    # skips the already-recorded phase and finishes the Issue close only.
    task = (read_json(root / ".pm/github-project-sync/tasks.json").get("tasks") or {}).get(args.task_uid) or {}
    if task.get("workflow_phase") != "post_merge_done" or (task.get("phase_receipt_sha256") or {}).get("post_merge_done") != terminal_sha:
        fail("post_merge_done receipt did not read back from task truth")
    journal["phase_readback"] = True
    write_json_atomic(journal_path, journal)
    audit = json.loads(command(sys.executable, str(root / "scripts/pm/github-project-workflow.py"),
                               str(root), "audit", "--task-uid", args.task_uid, "--include-done", "--json"))
    if audit.get("status") != "ok" or audit.get("selected_count") != 1:
        fail("coordinator Project audit did not read back")
    journal["project_readback"] = True
    write_json_atomic(journal_path, journal)
    issue = json.loads(command("gh", "issue", "view", str(issue_number), "-R", repository, "--json", "state"))
    if issue.get("state") != "CLOSED":
        command("gh", "issue", "close", str(issue_number), "-R", repository, "--reason", "completed")
        issue = json.loads(command("gh", "issue", "view", str(issue_number), "-R", repository, "--json", "state"))
    if issue.get("state") != "CLOSED":
        fail("coordinator Issue close did not read back")
    journal["issue_closed_readback"] = True
    write_json_atomic(journal_path, journal)
    lock.close()
    print(json.dumps({"status": "finalized", "task_uid": args.task_uid,
                      "terminal_receipt": str(terminal_path)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
