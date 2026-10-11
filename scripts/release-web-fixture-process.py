#!/usr/bin/env python3
"""Bound the entire owned test process group, including shutdown/HTTP workers."""
import os
import pathlib
import signal
import subprocess
import sys
import time

stop = pathlib.Path(sys.argv[1])
timeout = int(sys.argv[2])
child = subprocess.Popen(sys.argv[3:], start_new_session=True)
stop.with_suffix(".pid").write_text(str(child.pid))
closing = False
timed_out = False

def close(signum=None, frame=None):
    global closing
    closing = True

signal.signal(signal.SIGTERM, close)
signal.signal(signal.SIGINT, close)
deadline = time.monotonic() + timeout
try:
    while child.poll() is None and not closing and not stop.exists():
        if time.monotonic() >= deadline:
            print("error: owned release Web fixture process deadline exceeded", file=sys.stderr)
            closing = True
            timed_out = True
            break
        time.sleep(.1)
    if child.poll() is None:
        stop.touch(exist_ok=True)
        try:
            child.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=3)
    sys.exit(124 if timed_out else (child.returncode or 0))
finally:
    # Descendants may outlive their group leader; this group was created here.
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
