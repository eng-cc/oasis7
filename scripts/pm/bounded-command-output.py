#!/usr/bin/env python3
"""Run a command while retaining full output and printing a bounded JSON summary."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from typing import Any


TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}")
LABEL_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}")
DEFAULT_HEAD_LINES = 40
DEFAULT_TAIL_LINES = 40
DEFAULT_MAX_BYTES = 16 * 1024


def positive_or_zero(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("must be zero or greater")
    return parsed


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capture full stdout/stderr and emit a bounded JSON summary."
    )
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--head-lines", type=positive_or_zero, default=DEFAULT_HEAD_LINES)
    parser.add_argument("--tail-lines", type=positive_or_zero, default=DEFAULT_TAIL_LINES)
    parser.add_argument(
        "--max-bytes",
        type=positive_or_zero,
        default=DEFAULT_MAX_BYTES,
        help="Maximum encoded bytes retained in each stream summary.",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if not TASK_UID_RE.fullmatch(args.task_uid):
        parser.error("--task-uid must match task_<32 lowercase hex characters>")
    if not LABEL_RE.fullmatch(args.label) or args.label in {".", ".."}:
        parser.error("--label must be 1-64 safe filename characters")
    if not args.command or args.command[0] != "--" or len(args.command) == 1:
        parser.error("a command is required after --")
    args.command = args.command[1:]
    return args


def repo_relative(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return str(path)


def bounded_stream(data: bytes, head_lines: int, tail_lines: int, max_bytes: int) -> dict[str, Any]:
    lines = data.splitlines(keepends=True)
    chosen = lines[:head_lines]
    tail_start = max(head_lines, len(lines) - tail_lines)
    chosen.extend(lines[tail_start:])
    line_limited = b"".join(chosen)

    if len(line_limited) <= max_bytes:
        rendered = line_limited
    elif max_bytes == 0:
        rendered = b""
    else:
        head_bytes = (max_bytes + 1) // 2
        tail_bytes = max_bytes - head_bytes
        rendered = line_limited[:head_bytes]
        if tail_bytes:
            rendered += line_limited[-tail_bytes:]

    truncated = rendered != data
    return {
        "bytes": len(data),
        "lines": len(lines),
        "omitted_bytes": len(data) - len(rendered),
        "summary": rendered.decode("utf-8", errors="replace"),
        "summary_bytes": len(rendered),
        "truncated": truncated,
    }


def stream_record(
    path: Path, root: Path, head_lines: int, tail_lines: int, max_bytes: int
) -> dict[str, Any]:
    data = path.read_bytes()
    record = bounded_stream(data, head_lines, tail_lines, max_bytes)
    record["artifact"] = repo_relative(path, root)
    record["sha256"] = hashlib.sha256(data).hexdigest()
    return record


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    root = args.repo_root.resolve()
    if not root.is_dir():
        raise SystemExit(f"repo root is not a directory: {root}")

    scratch_root = root / ".pm" / "scratch"
    artifact_dir = scratch_root / args.task_uid / "command-output" / args.label
    stdout_path = artifact_dir / "stdout.bin"
    stderr_path = artifact_dir / "stderr.bin"
    # Capture outside the repository so commands that audit the live filesystem
    # cannot observe their own output artifacts changing beneath them.
    with tempfile.TemporaryDirectory(prefix="oasis7-command-output-") as temp_dir:
        temp_root = Path(temp_dir)
        temp_stdout = temp_root / "stdout.bin"
        temp_stderr = temp_root / "stderr.bin"
        with temp_stdout.open("wb") as stdout_file, temp_stderr.open("wb") as stderr_file:
            try:
                completed = subprocess.run(
                    args.command,
                    cwd=root,
                    stdin=None,
                    stdout=stdout_file,
                    stderr=stderr_file,
                    check=False,
                )
                exit_status = (
                    completed.returncode
                    if completed.returncode >= 0
                    else 128 - completed.returncode
                )
            except FileNotFoundError as error:
                stderr_file.write(f"command not found: {error.filename}\n".encode("utf-8"))
                exit_status = 127
            except PermissionError as error:
                stderr_file.write(f"command is not executable: {error.filename}\n".encode("utf-8"))
                exit_status = 126
        artifact_dir.mkdir(parents=True, exist_ok=True)
        ignore = scratch_root / ".gitignore"
        if not ignore.exists():
            ignore.write_text("*\n", encoding="utf-8")
        temp_stdout.replace(stdout_path)
        temp_stderr.replace(stderr_path)

    result = {
        "command": args.command,
        "exit_status": exit_status,
        "label": args.label,
        "schema": 1,
        "stderr": stream_record(
            stderr_path, root, args.head_lines, args.tail_lines, args.max_bytes
        ),
        "stdout": stream_record(
            stdout_path, root, args.head_lines, args.tail_lines, args.max_bytes
        ),
        "task_uid": args.task_uid,
        "truncated": False,
    }
    result["truncated"] = result["stdout"]["truncated"] or result["stderr"]["truncated"]
    json.dump(result, sys.stdout, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    sys.stdout.write("\n")
    return exit_status


if __name__ == "__main__":
    raise SystemExit(main())
