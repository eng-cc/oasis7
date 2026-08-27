#!/usr/bin/env python3
"""Refuse recreation of a checkout retired by terminal cleanup."""
from __future__ import annotations
import argparse, json, pathlib, subprocess

def norm(value: str) -> str:
    return str(pathlib.Path(value).expanduser().resolve())

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--worktree", required=True)
    parser.add_argument("--branch", required=True)
    args = parser.parse_args()
    root = pathlib.Path(subprocess.check_output(
        ["git", "-C", args.repo_root, "rev-parse", "--show-toplevel"], text=True
    ).strip()).resolve()
    requested = norm(args.worktree)
    common = pathlib.Path(subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "--git-common-dir"], text=True
    ).strip())
    if not common.is_absolute():
        common = (root / common).resolve()
    for tombstone in (common / "oasis7-workflow-receipts").glob("*/terminal-tombstone.json"):
        try:
            data = json.loads(tombstone.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if data.get("schema") != "oasis7_terminal_tombstone_v1" or data.get("checkout_recreation_forbidden") is not True:
            continue
        same_path = bool(data.get("canonical_worktree")) and norm(data["canonical_worktree"]) == requested
        same_branch = data.get("task_branch") == args.branch
        if same_path or same_branch:
            raise SystemExit("terminal-tombstone-guard: checkout recreation forbidden for terminal task "
                             f"{data.get('task_uid') or 'unknown'} ({tombstone})")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
