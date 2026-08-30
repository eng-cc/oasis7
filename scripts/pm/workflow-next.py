#!/usr/bin/env python3
"""Report the one supported next workflow command for a bound task.

The task mapping is the only workflow authority.  Bootstrap snapshots, slice
ledgers, and the durable supervisor checkpoint are read-only evidence inputs
used to detect stale or ambiguous state and to make a compact resume report.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
from typing import Any


TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
PHASES = {"intake", "bootstrap", "route", "execution", "verification",
          "pre_pr_review", "pre_pr_ready", "pr_watch", "task_done",
          "main_sync", "closed_without_merge", "post_merge_done"}
STATUS_PHASES = {
    "candidate": {"", "intake", "bootstrap", "route"},
    "committed": {"", "execution", "verification", "pre_pr_review"},
    "blocked": {"", "blocked"},
    "ready": {"pre_pr_ready"},
    "pr_watch": {"pr_watch"},
    "done": {"task_done", "main_sync", "closed_without_merge", "post_merge_done"},
    "deferred": {"", "closed_without_merge"},
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", "--root", dest="repo_root", default=".")
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--mapping")
    parser.add_argument("--snapshot")
    parser.add_argument("--slice-ledger")
    parser.add_argument("--checkpoint")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def load_json(path: pathlib.Path) -> tuple[Any | None, str | None]:
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except (OSError, json.JSONDecodeError) as exc:
        return None, str(exc)


def add_blocker(blockers: list[str], message: str) -> None:
    if message not in blockers:
        blockers.append(message)


def verify_snapshot(
    path: pathlib.Path,
    task: dict[str, Any],
    blockers: list[str],
    sources: list[str],
) -> None:
    if not path.exists():
        return
    sources.append(str(path))
    snapshot, error = load_json(path)
    if error or not isinstance(snapshot, dict):
        add_blocker(blockers, f"stale identity: bootstrap snapshot is unreadable ({error or 'not an object'})")
        return
    expected_digest = snapshot.get("digest")
    if expected_digest:
        unsigned = dict(snapshot)
        unsigned.pop("digest", None)
        actual = "sha256:" + hashlib.sha256(
            json.dumps(unsigned, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
        ).hexdigest()
        if actual != expected_digest:
            add_blocker(blockers, "stale identity: bootstrap snapshot digest mismatch")
    snapshot_task = snapshot.get("task") if isinstance(snapshot.get("task"), dict) else {}
    snapshot_git = snapshot.get("git") if isinstance(snapshot.get("git"), dict) else {}
    if snapshot_task.get("uid") not in (None, task.get("task_uid")):
        add_blocker(blockers, "stale identity: bootstrap snapshot task UID drift")
    for label, expected, actual in (
        ("repository", task.get("repository"), snapshot.get("repository")),
        ("worktree", task.get("canonical_worktree"), snapshot_git.get("worktree")),
        ("branch", task.get("task_branch"), snapshot_git.get("branch")),
    ):
        if actual not in (None, "") and expected not in (None, "") and str(actual) != str(expected):
            add_blocker(blockers, f"stale identity: bootstrap snapshot {label} drift")


def verify_ledger(
    path: pathlib.Path,
    task_uid: str,
    blockers: list[str],
    sources: list[str],
) -> None:
    if not path.exists():
        return
    sources.append(str(path))
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        add_blocker(blockers, f"stale identity: slice ledger is unreadable ({exc})")
        return
    for line_number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            add_blocker(blockers, f"ambiguous state: slice ledger line {line_number} is not JSON")
            continue
        if not isinstance(entry, dict) or entry.get("task_uid") not in (None, task_uid):
            add_blocker(blockers, f"stale identity: slice ledger line {line_number} has a different task UID")


def verify_checkpoint(
    path: pathlib.Path,
    root: pathlib.Path,
    task: dict[str, Any],
    blockers: list[str],
    sources: list[str],
) -> dict[str, Any] | None:
    if not path.exists():
        return None
    sources.append(str(path))
    checkpoint, error = load_json(path)
    if error or not isinstance(checkpoint, dict):
        add_blocker(blockers, f"stale identity: durable workflow checkpoint is unreadable ({error or 'not an object'})")
        return None
    if checkpoint.get("task_uid") not in (None, task.get("task_uid")):
        add_blocker(blockers, "stale identity: durable workflow checkpoint task UID drift")
    if checkpoint.get("repo") not in (None, "") and pathlib.Path(str(checkpoint["repo"])).resolve() != root:
        add_blocker(blockers, "stale identity: durable workflow checkpoint repository drift")
    return checkpoint


def command_for(
    phase: str,
    task: dict[str, Any],
    root: pathlib.Path,
    blockers: list[str],
) -> list[str]:
    uid = str(task.get("task_uid") or "")
    status = str(task.get("status") or "")
    pr = str(task.get("pr_number") or "")
    pr_url = str(task.get("pr_url") or task.get("pull_request_url") or "")
    if phase in {"closed_without_merge", "post_merge_done"}:
        return []
    if phase == "bootstrap":
        return [
            "python3", "scripts/pm/bootstrap-task-snapshot.py", "validate-or-create",
            "--repo-root", str(task.get("canonical_worktree") or root), "--task-uid", uid,
            "--producer", "human-operated-tpm",
        ]
    if phase == "execution":
        return ["./scripts/pm/github-project-workflow.sh", "--json", "audit", "--task-uid", uid]
    if phase in {"verification", "pre_pr_review", "pre_pr_ready"}:
        return ["./scripts/prepare-task-pr.sh", "--json"]
    if phase == "pr_watch":
        if not pr or not pr.isdigit() or not pr_url:
            add_blocker(blockers, "ambiguous state: PR watch phase lacks bound PR identity")
            return []
        return ["python3", "./scripts/pm/pr-lifecycle-gate.py", pr, "--task-uid", uid, "--json"]
    if phase == "task_done":
        if status == "done" and task.get("completion_mode") == "non_pr_task":
            evidence_path = str(task.get("non_pr_completion_evidence_file") or
                                root / ".pm/scratch" / uid / "non-pr-completion-evidence.txt")
            if not pathlib.Path(evidence_path).is_file():
                add_blocker(blockers, "ambiguous state: non-PR completion evidence file is unavailable")
                return []
            return [
                "python3", "./scripts/pm/non-merge-finalize.py", "--repo-root", str(root),
                "--task-uid", uid, "--reason", "non_pr_completed", "--evidence-file", evidence_path, "--json",
            ]
        if not pr or not pr.isdigit() or not pr_url:
            add_blocker(blockers, "ambiguous state: merged terminal phase lacks bound PR identity")
            return []
        return [
            "./scripts/pm/finalize-task.sh", "--repo-root", str(root), "--task-uid", uid,
            "--pr", pr, "--preflight", "--json",
        ]
    if phase == "main_sync":
        return ["./scripts/pm/finalize-task.sh", "--repo-root", str(root), "--task-uid", uid, "--pr", pr, "--resume", "--json"]
    add_blocker(blockers, f"ambiguous state: unsupported workflow phase {phase!r}")
    return []


def main() -> int:
    args = parse_args()
    root = pathlib.Path(args.repo_root).resolve()
    mapping_path = pathlib.Path(args.mapping).resolve() if args.mapping else root / ".pm/github-project-sync/tasks.json"
    payload: dict[str, Any] = {
        "task_uid": args.task_uid,
        "workflow_phase": "",
        "identity_status": "blocked",
        "evidence_sources": [str(mapping_path)],
        "blockers": [],
        "next_command": [],
    }
    blockers = payload["blockers"]
    if not TASK_UID_RE.fullmatch(args.task_uid):
        add_blocker(blockers, "stale identity: invalid task UID")
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 1
    mapping, error = load_json(mapping_path)
    if error or not isinstance(mapping, dict):
        add_blocker(blockers, f"stale identity: canonical task mapping is unreadable ({error or 'not an object'})")
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 1
    task = (mapping.get("tasks") or {}).get(args.task_uid)
    if not isinstance(task, dict):
        add_blocker(blockers, "stale identity: task UID is absent from canonical mapping")
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 1
    task = dict(task)
    task["task_uid"] = args.task_uid
    payload["task_uid"] = args.task_uid
    payload.update({
        "status": str(task.get("status") or ""),
        "repository": str(task.get("repository") or ""),
        "issue_number": task.get("issue_number"),
        "issue_url": str(task.get("issue_url") or ""),
        "canonical_worktree": str(task.get("canonical_worktree") or ""),
        "task_branch": str(task.get("task_branch") or ""),
        "default_branch": str(task.get("default_branch") or ""),
    })
    for key in ("repository", "issue_number", "issue_url", "canonical_worktree", "task_branch", "default_branch"):
        if payload[key] in (None, ""):
            add_blocker(blockers, f"stale identity: task truth missing {key}")
    raw_phase = str(task.get("workflow_phase") or "")
    status = str(task.get("status") or "")
    if raw_phase not in PHASES and raw_phase != "":
        add_blocker(blockers, f"ambiguous state: unsupported workflow phase {raw_phase!r}")
        phase = raw_phase
    else:
        phase = raw_phase
        if not phase:
            phase = {"candidate": "bootstrap", "committed": "execution", "ready": "pre_pr_ready",
                     "pr_watch": "pr_watch", "done": "task_done", "deferred": "closed_without_merge"}.get(status, "")
        if status not in STATUS_PHASES:
            add_blocker(blockers, f"ambiguous state: unsupported task status {status!r}")
        elif raw_phase not in STATUS_PHASES[status]:
            add_blocker(blockers, f"ambiguous state: status {status!r} cannot use workflow phase {raw_phase!r}")
    payload["workflow_phase"] = phase

    snapshot_path = pathlib.Path(args.snapshot).resolve() if args.snapshot else root / ".pm/scratch" / args.task_uid / "bootstrap-task-snapshot.json"
    ledger_path = pathlib.Path(args.slice_ledger).resolve() if args.slice_ledger else root / ".pm/scratch" / args.task_uid / "slice-ledger.jsonl"
    checkpoint_path = pathlib.Path(args.checkpoint).resolve() if args.checkpoint else root / ".pm/tasks" / f"{args.task_uid}.workflow.json"
    sources = payload["evidence_sources"]
    verify_snapshot(snapshot_path, task, blockers, sources)
    verify_ledger(ledger_path, args.task_uid, blockers, sources)
    checkpoint = verify_checkpoint(checkpoint_path, root, task, blockers, sources)
    if checkpoint and checkpoint.get("phase") not in (None, "", phase):
        add_blocker(blockers, "ambiguous state: durable workflow checkpoint phase disagrees with task mapping")
    payload["evidence_sources"] = list(dict.fromkeys(sources))
    if not blockers:
        payload["next_command"] = command_for(phase, task, root, blockers)
    if blockers:
        payload["next_command"] = []
        payload["identity_status"] = "stale" if any(item.startswith("stale identity") for item in blockers) else "ambiguous"
    else:
        payload["identity_status"] = "bound"
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 1 if blockers else 0


if __name__ == "__main__":
    raise SystemExit(main())
