#!/usr/bin/env python3
"""Run one harness operation with a portable process-group deadline."""

from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--timeout", required=True, type=int)
    parser.add_argument("--phase", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.timeout < 0:
        parser.error("--timeout must be a non-negative integer")
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command:
        parser.error("missing command")
    return args


def terminate_process_group(process: subprocess.Popen[str]) -> None:
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
        try:
            process.wait(timeout=0.25)
        except subprocess.TimeoutExpired:
            pass
        # The shell leader may exit before a descendant that inherited the
        # captured stdout/stderr pipes. Always reap the whole group so the
        # subsequent communicate() cannot wait forever on those open FDs.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    else:
        process.kill()


def main() -> int:
    args = parse_args()
    started_at = datetime.now(timezone.utc).isoformat()
    log_path = Path(args.log)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    timed_out = False
    # Real files avoid the classic timeout deadlock where a killed shell exits
    # but one of its descendants keeps a PIPE file descriptor open.
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stdout_file, tempfile.TemporaryFile(
        mode="w+", encoding="utf-8"
    ) as stderr_file:
        process = subprocess.Popen(
            args.command,
            stdout=stdout_file,
            stderr=stderr_file,
            text=True,
            start_new_session=(os.name == "posix"),
        )
        try:
            process.wait(timeout=args.timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            terminate_process_group(process)
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        stdout_file.seek(0)
        stderr_file.seek(0)
        stdout = stdout_file.read()
        stderr = stderr_file.read()

    with log_path.open("a", encoding="utf-8") as log:
        if stdout:
            log.write(stdout)
        if stderr:
            log.write(stderr)

    if stdout:
        sys.stdout.write(stdout)
    if timed_out:
        sys.stderr.write(
            f"error: phase={args.phase} timeout_secs={args.timeout} "
            f"deadline={started_at}; operation exceeded its deadline; "
            f"artifacts={args.log}\n"
        )
        return 124
    if process.returncode != 0:
        sys.stderr.write(
            f"error: phase={args.phase} command_exit={process.returncode}; "
            f"artifacts={args.log}\n"
        )
    return process.returncode


if __name__ == "__main__":
    raise SystemExit(main())
