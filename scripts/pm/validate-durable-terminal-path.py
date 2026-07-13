#!/usr/bin/env python3
"""Resolve a terminal artifact path and reject task-worktree sinks."""
from __future__ import annotations

import argparse
import json
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(f"validate-durable-terminal-path: {message}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mapping", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--path", required=True)
    parser.add_argument("--label", required=True)
    args = parser.parse_args()

    supplied = Path(args.path)
    if not supplied.is_absolute():
        fail(f"{args.label} must be absolute")
    resolved = supplied.resolve(strict=False)
    mapping = json.loads(Path(args.mapping).read_text(encoding="utf-8"))
    record = (mapping.get("tasks") or {}).get(args.task_uid) or {}
    canonical_text = str(record.get("canonical_worktree") or "").strip()
    if not canonical_text:
        fail("canonical task worktree is unavailable")
    canonical_worktree = Path(canonical_text).resolve(strict=False)
    try:
        resolved.relative_to(canonical_worktree)
    except ValueError:
        pass
    else:
        fail(f"{args.label} must not be inside canonical task worktree")
    print(resolved)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
