"""Bounded child execution, process heartbeats and owned-process cleanup."""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import threading
import time
import uuid


def owned_processes(marker, group=None):
    """Linux environment marker survives double-fork/setsid; identity includes starttime."""
    found = {}
    for directory in Path('/proc').glob('[0-9]*'):
        try:
            if directory.stat().st_uid != os.getuid():
                continue
            fields = (directory / 'stat').read_text().rsplit(')', 1)[1].split()
            in_group = group is not None and int(fields[2]) == group
            if not in_group:
                try:
                    environment = (directory / 'environ').read_bytes().split(b'\0')
                except PermissionError:
                    # Same-UID runner services can be non-dumpable. They are not
                    # ours merely because their environment cannot be inspected.
                    continue
                if ('OASIS7_CI_PROCESS_OWNER=' + marker).encode() not in environment:
                    continue
            if fields[0] != 'Z':
                found[int(directory.name)] = fields[19]
        except (FileNotFoundError, ProcessLookupError):
            continue
    return found


def cleanup(process, marker):
    if Path('/proc').is_dir():
        # Never signal a saved PID without checking its current ownership and starttime.
        initial = owned_processes(marker, process.pid)
        for sig in (signal.SIGTERM, signal.SIGKILL):
            pending = owned_processes(marker, process.pid)
            for pid, identity in pending.items():
                if owned_processes(marker, process.pid).get(pid) == identity:
                    try:
                        os.kill(pid, sig)
                    except ProcessLookupError:
                        pass
            if pending:
                time.sleep(0.3)
        process.poll()
        return ('unresolved' if owned_processes(marker, process.pid) else 'clean'), bool(initial)
    # macOS: inspect only this new process group; zombies need no signal.
    def members():
        rows = subprocess.check_output(['ps', '-axo', 'pid=,pgid=,stat='], text=True)
        return [int(pid) for pid, group, state in (row.split() for row in rows.splitlines())
                if int(group) == process.pid and not state.startswith('Z')]
    initial = members()
    for sig in (signal.SIGTERM, signal.SIGKILL):
        if members():
            try:
                os.killpg(process.pid, sig)
            except ProcessLookupError:
                pass
            except PermissionError:
                process.poll()
                if members():
                    return 'unresolved', bool(initial)
            time.sleep(0.3)
        process.poll()
    return ('unresolved' if members() else 'clean'), bool(initial)


def run(command, cwd, env, suite, name, results, expected_count=None, timeout_seconds=2400, heartbeat_seconds=20, shard=None):
    marker = uuid.uuid4().hex
    env = dict(env, OASIS7_CI_PROCESS_OWNER=marker)
    started = datetime.now(timezone.utc).isoformat()
    start = time.monotonic()
    summaries = []
    def consume(pipe):
        for line in pipe:
            print(line, end='', flush=True)
            match = re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', line)
            if match:
                summaries.append(tuple(map(int, match.groups())))
    process = subprocess.Popen(command, cwd=cwd, env=env, start_new_session=True,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors='replace')
    reader = threading.Thread(target=consume, args=(process.stdout,), daemon=True)
    reader.start()
    timed_out = False
    cancelled = False
    previous_handlers = {}
    def interrupt(signum, frame):
        raise InterruptedError(f'cancelled by signal {signum}')
    if threading.current_thread() is threading.main_thread():
        for signum in (signal.SIGTERM, signal.SIGINT):
            previous_handlers[signum] = signal.signal(signum, interrupt)
    next_heartbeat = start + heartbeat_seconds
    try:
        while process.poll() is None:
            now = time.monotonic()
            if now - start >= timeout_seconds:
                timed_out = True
                break
            if now >= next_heartbeat:
                print(f'suite={suite} shard={shard} test={name} elapsed={int(now-start)}s state=running', flush=True)
                next_heartbeat = now + heartbeat_seconds
            time.sleep(min(0.2, heartbeat_seconds))
    except InterruptedError:
        cancelled = True
    finally:
        try:
            cleanup_result, leaked = cleanup(process, marker)
        except (OSError, subprocess.SubprocessError):
            cleanup_result, leaked = 'unresolved', True
        for signum, handler in previous_handlers.items():
            signal.signal(signum, handler)
    if cleanup_result == 'clean':
        process.wait(timeout=10)
    reader.join(timeout=2)
    if not reader.is_alive():
        process.stdout.close()
    count_ok = expected_count is None or (expected_count > 0 and bool(summaries) and summaries[-1][0] == expected_count and summaries[-1][1] == 0)
    result = dict(suite=suite, shard=shard, test_name=name, started_at=started,
                  elapsed_seconds=round(time.monotonic()-start, 3), exit_code=process.returncode,
                  timeout=timed_out, cancelled=cancelled, cleanup_result=cleanup_result, leaked_processes=leaked,
                  expected_count=expected_count, count_ok=count_ok,
                  passed=process.returncode == 0 and not timed_out and not cancelled and count_ok and cleanup_result == 'clean' and not leaked)
    results.parent.mkdir(parents=True, exist_ok=True)
    with results.open('a') as stream:
        stream.write(json.dumps(result) + '\n')
    print(json.dumps(result), flush=True)
    return result
