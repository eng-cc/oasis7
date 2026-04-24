#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/worktree-gc-report.sh [options]

Summarize the current repo's git worktree lifecycle state and surface cleanup
candidates without mutating anything.

Options:
  --json           Print machine-readable JSON with all discovered worktrees
  --prunable-only  Limit human-readable output to cleanup candidates only
  -h, --help       Show this help

Examples:
  ./scripts/worktree-gc-report.sh
  ./scripts/worktree-gc-report.sh --prunable-only
  ./scripts/worktree-gc-report.sh --json
USAGE
}

wh_require_git_worktree

OUTPUT_JSON=0
PRUNABLE_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    --prunable-only)
      PRUNABLE_ONLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

COMMON_GIT_DIR="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
CANONICAL_REPO_ROOT="$(cd "$COMMON_GIT_DIR/.." && pwd -P)"
CURRENT_WORKTREE="$(pwd -P)"

python3 - "$COMMON_GIT_DIR" "$CANONICAL_REPO_ROOT" "$CURRENT_WORKTREE" "$PRUNABLE_ONLY" "$OUTPUT_JSON" <<'PY'
from __future__ import annotations

import json
import shlex
import subprocess
import sys
from pathlib import Path


common_git_dir = Path(sys.argv[1]).resolve()
repo_root = Path(sys.argv[2]).resolve()
current_worktree = Path(sys.argv[3]).resolve()
prunable_only = sys.argv[4] == "1"
output_json = sys.argv[5] == "1"


def run(*args: str) -> str:
    return subprocess.check_output(args, text=True)


def shell_command(*parts: str) -> str:
    return " ".join(shlex.quote(part) for part in parts)


def parse_porcelain() -> list[dict[str, object]]:
    raw = run("git", f"--git-dir={common_git_dir}", "worktree", "list", "--porcelain")
    records: list[dict[str, object]] = []
    current: dict[str, object] = {}
    for line in raw.splitlines():
        if not line:
            if current:
                records.append(current)
                current = {}
            continue
        key, sep, value = line.partition(" ")
        if not sep:
            current[key] = True
        elif key in {"locked", "prunable"}:
            current[key] = value
        else:
            current[key] = value
    if current:
        records.append(current)
    return records


def strip_quotes(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] == '"':
        return value[1:-1]
    return value


def parse_task_file(path: Path) -> dict[str, str]:
    parsed: dict[str, str] = {"task_uid": path.stem}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith(" ") or raw.startswith("-"):
            continue
        key, sep, value = raw.partition(":")
        if not sep:
            continue
        parsed[key.strip()] = strip_quotes(value.strip())
    return parsed


def load_task_index() -> dict[str, list[dict[str, str]]]:
    index: dict[str, list[dict[str, str]]] = {}
    task_dir = repo_root / ".pm" / "tasks"
    for task_file in sorted(task_dir.glob("task_*.yaml")):
        parsed = parse_task_file(task_file)
        hint = parsed.get("worktree_hint", "")
        if not hint or hint == "null" or not hint.startswith("/"):
            continue
        normalized = str(Path(hint).resolve())
        index.setdefault(normalized, []).append(parsed)
    return index


def worktree_status(path: Path) -> tuple[bool | None, int | None]:
    if not path.exists():
        return None, None
    result = subprocess.run(
        ["git", "-C", str(path), "status", "--short"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    if result.returncode != 0:
        return None, None
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    return bool(lines), len(lines)


records = parse_porcelain()
task_index = load_task_index()

branch_attached_counts: dict[str, int] = {}
for record in records:
    branch_ref = record.get("branch")
    if not isinstance(branch_ref, str):
        continue
    branch = branch_ref.removeprefix("refs/heads/")
    branch_attached_counts[branch] = branch_attached_counts.get(branch, 0) + 1

entries: list[dict[str, object]] = []
cleanup_candidates: list[dict[str, object]] = []
dirty_count = 0
prunable_count = 0

for record in records:
    path_value = str(record["worktree"])
    path_obj = Path(path_value)
    resolved_path = path_obj.resolve(strict=False)
    branch_ref = record.get("branch")
    branch = branch_ref.removeprefix("refs/heads/") if isinstance(branch_ref, str) else None
    detached = bool(record.get("detached"))
    prunable_reason = record.get("prunable")
    prunable = prunable_reason is not None
    locked_reason = record.get("locked")
    exists = path_obj.exists()
    is_current = resolved_path == current_worktree
    dirty, dirty_count_lines = worktree_status(path_obj)
    if dirty:
        dirty_count += 1
    if prunable:
        prunable_count += 1

    task_matches = task_index.get(str(resolved_path), [])
    latest_task = None
    if task_matches:
        latest_task = sorted(task_matches, key=lambda item: item.get("updated_at", ""))[-1]

    cleanup_reasons: list[str] = []
    cleanup_commands: list[str] = []
    branch_delete_candidate = False

    latest_status = latest_task.get("status") if latest_task else None
    if prunable:
        cleanup_reasons.append("prunable_worktree")
    if (
        latest_status in {"done", "deferred"}
        and exists
        and not is_current
        and dirty is False
    ):
        cleanup_reasons.append("closed_pm_task")

    cleanup_candidate = bool(cleanup_reasons)
    if cleanup_candidate:
        cleanup_commands.append(
            shell_command(
                "git",
                "-C",
                str(repo_root),
                "worktree",
                "remove",
                "-f",
                str(resolved_path),
            )
        )
        if branch and branch_attached_counts.get(branch, 0) == 1 and not is_current:
            branch_delete_candidate = True
            cleanup_commands.append(
                shell_command("git", "-C", str(repo_root), "branch", "-d", branch)
            )

    entry = {
        "path": str(resolved_path),
        "branch": branch,
        "detached": detached,
        "current": is_current,
        "exists": exists,
        "dirty": dirty,
        "dirty_entry_count": dirty_count_lines,
        "prunable": prunable,
        "prunable_reason": prunable_reason,
        "locked_reason": locked_reason,
        "pm_task_uid": latest_task.get("task_uid") if latest_task else None,
        "pm_task_status": latest_status,
        "pm_task_title": latest_task.get("title") if latest_task else None,
        "pm_task_updated_at": latest_task.get("updated_at") if latest_task else None,
        "cleanup_candidate": cleanup_candidate,
        "cleanup_reasons": cleanup_reasons,
        "branch_delete_candidate": branch_delete_candidate,
        "cleanup_commands": cleanup_commands,
    }
    entries.append(entry)
    if cleanup_candidate:
        cleanup_candidates.append(entry)

payload = {
    "repo_root": str(repo_root),
    "current_worktree": str(current_worktree),
    "summary": {
        "total_worktrees": len(entries),
        "prunable_worktrees": prunable_count,
        "dirty_worktrees": dirty_count,
        "cleanup_candidates": len(cleanup_candidates),
    },
    "entries": entries,
}

if output_json:
    print(json.dumps(payload, ensure_ascii=True, indent=2))
    raise SystemExit(0)

print("worktree lifecycle report")
print(f"- repo_root: {repo_root}")
print(f"- current_worktree: {current_worktree}")
print(f"- total_worktrees: {len(entries)}")
print(f"- prunable_worktrees: {prunable_count}")
print(f"- dirty_worktrees: {dirty_count}")
print(f"- cleanup_candidates: {len(cleanup_candidates)}")

shown = cleanup_candidates if prunable_only else entries
if not shown:
    print("- details: none")
    raise SystemExit(0)

print("- details:")
for entry in shown:
    if prunable_only and not entry["cleanup_candidate"]:
        continue
    label_parts = []
    if entry["branch"]:
        label_parts.append(str(entry["branch"]))
    elif entry["detached"]:
        label_parts.append("detached")
    else:
        label_parts.append("unknown-branch")
    if entry["current"]:
        label_parts.append("current")
    if entry["prunable"]:
        label_parts.append("prunable")
    if entry["dirty"] is True:
        label_parts.append(f"dirty={entry['dirty_entry_count']}")
    elif entry["dirty"] is False:
        label_parts.append("clean")
    else:
        label_parts.append("dirty=unknown")
    print(f"  - {' | '.join(label_parts)}")
    print(f"    path: {entry['path']}")
    if entry["pm_task_uid"]:
        print(
            "    pm_task: "
            f"{entry['pm_task_uid']} ({entry['pm_task_status']}) {entry['pm_task_title']}"
        )
    if entry["cleanup_candidate"]:
        print(f"    cleanup_reasons: {', '.join(entry['cleanup_reasons'])}")
        print("    cleanup_commands:")
        for command in entry["cleanup_commands"]:
            print(f"      - {command}")
    elif not prunable_only:
        print("    cleanup_reasons: none")
PY
