#!/usr/bin/env bash

wh_require_git_worktree() {
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "error: worktree harness must run inside a git worktree" >&2
    exit 1
  fi
}

wh_repo_root() {
  git rev-parse --show-toplevel
}

wh_git_head() {
  git rev-parse HEAD
}

wh_worktree_path() {
  pwd -P
}

wh_git_common_dir() {
  local repo_root common_dir
  repo_root=$(git rev-parse --show-toplevel) || return 1
  common_dir=$(git rev-parse --git-common-dir) || return 1
  if [[ "$common_dir" == /* ]]; then
    (cd "$common_dir" && pwd -P)
  else
    (cd "$repo_root/$common_dir" && pwd -P)
  fi
}

wh_worktree_id() {
  python3 - "$(wh_worktree_path)" <<'PY'
import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1]).resolve()
digest = hashlib.sha256(str(path).encode("utf-8")).hexdigest()[:8]
print(f"wt-{digest}")
PY
}

wh_harness_root() {
  local repo_root=$1
  local worktree_id=$2
  printf '%s/output/harness/%s\n' "$repo_root" "$worktree_id"
}

wh_runtime_dir() {
  local harness_root=$1
  printf '%s/runtime\n' "$harness_root"
}

wh_artifacts_dir() {
  local harness_root=$1
  printf '%s/artifacts\n' "$harness_root"
}

wh_browser_dir() {
  local harness_root=$1
  printf '%s/browser\n' "$harness_root"
}

wh_bundle_root() {
  local harness_root=$1
  printf '%s/bundle\n' "$harness_root"
}

wh_default_bundle_dir() {
  local harness_root=$1
  printf '%s/game-launcher-local\n' "$(wh_bundle_root "$harness_root")"
}

wh_default_producer_bundle_dir() {
  local harness_root=$1
  printf '%s/game-launcher-producer-local\n' "$(wh_bundle_root "$harness_root")"
}

wh_state_file() {
  local harness_root=$1
  printf '%s/state.json\n' "$harness_root"
}

wh_startup_log() {
  local harness_root=$1
  printf '%s/startup.log\n' "$harness_root"
}

wh_runtime_meta_file() {
  local harness_root=$1
  printf '%s/session.meta\n' "$(wh_runtime_dir "$harness_root")"
}

wh_browser_session() {
  local worktree_id=$1
  printf '%s\n' "$worktree_id"
}

wh_prepare_dirs() {
  local harness_root=$1
  mkdir -p "$harness_root" "$(wh_runtime_dir "$harness_root")" "$(wh_artifacts_dir "$harness_root")" "$(wh_browser_dir "$harness_root")" "$(wh_bundle_root "$harness_root")"
}

# Publish a complete record with a same-directory temporary file and an
# atomic replacement.  Readers that do not take the lock still see either the
# old complete record or the new complete record, never a partially-written
# file.  The lock is deliberately kept beside the record so concurrent
# read/modify/write callers do not lose updates while they are assembling a
# new state snapshot.
wh_atomic_write() {
  local destination=$1
  local contents=${2-}
  mkdir -p "$(dirname "$destination")"
  python3 - "$destination" "$contents" <<'PY'
from __future__ import annotations

import fcntl
import os
import pathlib
import tempfile
import sys

destination = pathlib.Path(sys.argv[1])
contents = sys.argv[2]
destination.parent.mkdir(parents=True, exist_ok=True)
lock_path = destination.with_name(f".{destination.name}.lock")

with lock_path.open("a+", encoding="utf-8") as lock:
    fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
    mode = destination.stat().st_mode & 0o777 if destination.exists() else 0o644
    fd, temporary_name = tempfile.mkstemp(
        prefix=f".{destination.name}.",
        dir=str(destination.parent),
    )
    temporary_path = pathlib.Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as temporary:
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.chmod(temporary_path, mode)
        os.replace(temporary_path, destination)
        directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        directory_fd = os.open(destination.parent, directory_flags)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        temporary_path.unlink(missing_ok=True)
PY
}

wh_resolve_ports_json() {
  local harness_root=${1:-}
  local owner_pid=${2:-$$}
  local worktree_path=${3:-$(wh_worktree_path)}
  local common_dir=${4:-$(wh_git_common_dir)}
  [[ -n "$harness_root" ]] || {
    echo "error: wh_resolve_ports_json requires a harness root" >&2
    return 2
  }
  [[ -n "$common_dir" ]] || {
    echo "error: wh_resolve_ports_json requires a git common directory" >&2
    return 2
  }
  mkdir -p "$harness_root"
  python3 - "$harness_root" "$owner_pid" "$worktree_path" "$common_dir" <<'PY'
from __future__ import annotations

import fcntl
import hashlib
import json
import os
import socket
import sys
import tempfile
import uuid
from pathlib import Path

harness_root = Path(sys.argv[1]).resolve()
owner_pid = int(sys.argv[2]) if sys.argv[2] else os.getppid()
worktree_path = str(Path(sys.argv[3]).resolve())
common_dir = Path(sys.argv[4]).resolve()
seed = int(hashlib.sha256(worktree_path.encode("utf-8")).hexdigest()[:8], 16)
start = 43000 + (seed % 1500) * 10
reservation_path = harness_root / ".ports.reservation.json"
local_lock_path = harness_root / ".ports.lock"
registry_dir = common_dir / ".oasis7-harness-port-registry"
registry_path = registry_dir / "reservations.json"
registry_lock_path = registry_dir / "registry.lock"
registry_dir.mkdir(parents=True, exist_ok=True)


def pid_alive(pid: int) -> bool:
    if pid <= 1:
        return False
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    return True


def atomic_write(path: Path, contents: str) -> None:
    fd, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=str(path.parent))
    temporary_path = Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as temporary:
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
        directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        directory_fd = os.open(path.parent, directory_flags)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        temporary_path.unlink(missing_ok=True)


def load_registry() -> dict:
    if not registry_path.exists():
        return {"schema": 1, "reservations": {}}
    try:
        registry = json.loads(registry_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"error: invalid shared harness port registry: {exc}")
    if registry.get("schema") != 1 or not isinstance(registry.get("reservations"), dict):
        raise SystemExit("error: unsupported shared harness port registry schema")
    return registry


def save_registry(registry: dict) -> None:
    atomic_write(registry_path, json.dumps(registry, sort_keys=True) + "\n")


def reservation_ports(reservation: dict) -> set[int]:
    ports = reservation.get("ports", {})

    def port_value(value: object) -> int:
        if isinstance(value, str):
            return int(value.rsplit(":", 1)[-1])
        return int(value)

    return {
        port_value(ports[key])
        for key in ("viewer_port", "web_bind", "live_bind", "chain_status_bind")
    }


def free(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            sock.bind(("127.0.0.1", port))
        except OSError:
            return False
    return True


with local_lock_path.open("a+", encoding="utf-8") as local_lock:
    fcntl.flock(local_lock.fileno(), fcntl.LOCK_EX)
    with registry_lock_path.open("a+", encoding="utf-8") as registry_lock:
        fcntl.flock(registry_lock.fileno(), fcntl.LOCK_EX)
        if reservation_path.exists():
            try:
                local_reservation = json.loads(reservation_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                raise SystemExit(f"error: invalid harness port reservation: {exc}")
            local_owner = int(local_reservation.get("owner_pid", 0))
            if pid_alive(local_owner):
                raise SystemExit(
                    f"error: harness ports are already reserved by live owner {local_owner}"
                )
            reservation_path.unlink()

        registry = load_registry()
        reservations = registry["reservations"]
        stale_tokens = [
            token
            for token, record in reservations.items()
            if not pid_alive(int(record.get("owner_pid", 0)))
        ]
        for token in stale_tokens:
            del reservations[token]

        for token, record in reservations.items():
            if record.get("harness_root") == str(harness_root) and pid_alive(int(record.get("owner_pid", 0))):
                raise SystemExit(
                    f"error: harness ports are already reserved by live owner {record.get('owner_pid')}"
                )

        reserved_ports = set()
        for record in reservations.values():
            reserved_ports.update(reservation_ports(record))

        for step in range(1500):
            base = start + step * 10
            ports = [base, base + 1, base + 2, base + 3]
            if ports[-1] > 65000:
                continue
            if set(ports) & reserved_ports:
                continue
            if all(free(port) for port in ports):
                token = uuid.uuid4().hex
                payload = {
                    "viewer_port": ports[0],
                    "web_bind": f"127.0.0.1:{ports[1]}",
                    "live_bind": f"127.0.0.1:{ports[2]}",
                    "chain_status_bind": f"127.0.0.1:{ports[3]}",
                }
                reservation = {
                    "schema": 1,
                    "reservation_token": token,
                    "owner_pid": owner_pid,
                    "worktree_path": worktree_path,
                    "harness_root": str(harness_root),
                    "common_dir": str(common_dir),
                    "registry_path": str(registry_path),
                    "ports": payload,
                }
                reservations[token] = reservation
                save_registry(registry)
                atomic_write(reservation_path, json.dumps(reservation, sort_keys=True) + "\n")
                payload["reservation_token"] = token
                payload["reservation_file"] = str(reservation_path)
                payload["registry_file"] = str(registry_path)
                print(json.dumps(payload, ensure_ascii=True))
                break
        else:
            raise SystemExit("error: unable to allocate free loopback ports for worktree harness")
PY
}

wh_bind_ports_owner() {
  local harness_root=$1
  local reservation_token=$2
  local old_owner_pid=$3
  local new_owner_pid=$4
  local common_dir=${5:-}
  python3 - "$harness_root" "$reservation_token" "$old_owner_pid" "$new_owner_pid" "$common_dir" <<'PY'
from __future__ import annotations

import fcntl
import json
import os
import pathlib
import tempfile
import sys

harness_root = pathlib.Path(sys.argv[1]).resolve()
expected_token = sys.argv[2]
old_owner_pid = int(sys.argv[3]) if sys.argv[3] else os.getppid()
new_owner_pid = int(sys.argv[4])
common_dir = pathlib.Path(sys.argv[5]).resolve() if sys.argv[5] else None
reservation_path = harness_root / ".ports.reservation.json"


def atomic_write(path: pathlib.Path, contents: str) -> None:
    fd, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=str(path.parent))
    temporary_path = pathlib.Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as temporary:
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
        directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        temporary_path.unlink(missing_ok=True)


local_lock_path = harness_root / ".ports.lock"
with local_lock_path.open("a+", encoding="utf-8") as local_lock:
    fcntl.flock(local_lock.fileno(), fcntl.LOCK_EX)
    if not reservation_path.exists():
        raise SystemExit("error: harness port reservation disappeared before owner binding")
    reservation = json.loads(reservation_path.read_text(encoding="utf-8"))
    if reservation.get("reservation_token") != expected_token:
        raise SystemExit("error: harness port reservation token changed before owner binding")
    if int(reservation.get("owner_pid", 0)) != old_owner_pid:
        raise SystemExit("error: harness port reservation owner changed before owner binding")
    registry_path = pathlib.Path(reservation.get("registry_path", ""))
    if not registry_path.is_absolute():
        if common_dir is None:
            raise SystemExit("error: harness port reservation has no shared registry path")
        registry_path = common_dir / ".oasis7-harness-port-registry" / "reservations.json"
    registry_lock_path = registry_path.parent / "registry.lock"
    with registry_lock_path.open("a+", encoding="utf-8") as registry_lock:
        fcntl.flock(registry_lock.fileno(), fcntl.LOCK_EX)
        if not registry_path.exists():
            raise SystemExit("error: shared harness port registry disappeared before owner binding")
        registry = json.loads(registry_path.read_text(encoding="utf-8"))
        reservations = registry.get("reservations")
        record = reservations.get(expected_token) if isinstance(reservations, dict) else None
        if not isinstance(record, dict) or record.get("harness_root") != str(harness_root):
            raise SystemExit("error: shared harness port reservation changed before owner binding")
        if int(record.get("owner_pid", 0)) != old_owner_pid:
            raise SystemExit("error: shared harness port owner changed before owner binding")
        reservation["owner_pid"] = new_owner_pid
        record["owner_pid"] = new_owner_pid
        atomic_write(registry_path, json.dumps(registry, sort_keys=True) + "\n")
        atomic_write(reservation_path, json.dumps(reservation, sort_keys=True) + "\n")
PY
}

wh_release_ports_reservation() {
  local harness_root=$1
  local reservation_token=$2
  local common_dir=${3:-}
  python3 - "$harness_root" "$reservation_token" "$common_dir" <<'PY'
from __future__ import annotations

import fcntl
import json
import os
import pathlib
import tempfile
import sys

harness_root = pathlib.Path(sys.argv[1]).resolve()
expected_token = sys.argv[2]
common_dir = pathlib.Path(sys.argv[3]).resolve() if sys.argv[3] else None
reservation_path = harness_root / ".ports.reservation.json"


def atomic_write(path: pathlib.Path, contents: str) -> None:
    fd, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=str(path.parent))
    temporary_path = pathlib.Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as temporary:
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
        directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        temporary_path.unlink(missing_ok=True)


local_lock_path = harness_root / ".ports.lock"
with local_lock_path.open("a+", encoding="utf-8") as local_lock:
    fcntl.flock(local_lock.fileno(), fcntl.LOCK_EX)
    reservation = {}
    if reservation_path.exists():
        reservation = json.loads(reservation_path.read_text(encoding="utf-8"))
    registry_path = pathlib.Path(reservation.get("registry_path", ""))
    if not registry_path.is_absolute():
        if common_dir is None:
            if reservation_path.exists():
                raise SystemExit("error: harness port reservation has no shared registry path")
            raise SystemExit(0)
        registry_path = common_dir / ".oasis7-harness-port-registry" / "reservations.json"
    registry_lock_path = registry_path.parent / "registry.lock"
    with registry_lock_path.open("a+", encoding="utf-8") as registry_lock:
        fcntl.flock(registry_lock.fileno(), fcntl.LOCK_EX)
        if registry_path.exists():
            registry = json.loads(registry_path.read_text(encoding="utf-8"))
            reservations = registry.get("reservations")
            if isinstance(reservations, dict):
                record = reservations.get(expected_token)
                if isinstance(record, dict) and record.get("harness_root") == str(harness_root):
                    del reservations[expected_token]
                    atomic_write(registry_path, json.dumps(registry, sort_keys=True) + "\n")
        if reservation.get("reservation_token") == expected_token and reservation_path.exists():
            reservation_path.unlink()
PY
}

wh_state_write() {
  local state_file=$1
  local patch_json=$2
  mkdir -p "$(dirname "$state_file")"
  python3 - "$state_file" "$patch_json" <<'PY'
from __future__ import annotations

import fcntl
import json
import os
import pathlib
import tempfile
import sys

state_path = pathlib.Path(sys.argv[1])
patch = json.loads(sys.argv[2])
state_path.parent.mkdir(parents=True, exist_ok=True)
lock_path = state_path.with_name(f".{state_path.name}.lock")
with lock_path.open("a+", encoding="utf-8") as lock:
    fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
    if state_path.exists():
        current = json.loads(state_path.read_text(encoding="utf-8"))
    else:
        current = {}
    current.update(patch)
    contents = json.dumps(current, ensure_ascii=False, indent=2) + "\n"
    mode = state_path.stat().st_mode & 0o777 if state_path.exists() else 0o644
    fd, temporary_name = tempfile.mkstemp(
        prefix=f".{state_path.name}.",
        dir=str(state_path.parent),
    )
    temporary_path = pathlib.Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as temporary:
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.chmod(temporary_path, mode)
        os.replace(temporary_path, state_path)
        directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        directory_fd = os.open(state_path.parent, directory_flags)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        temporary_path.unlink(missing_ok=True)
PY
}

# Return a comparable wall-epoch clock in milliseconds.  The Codex CI runner
# may give separate Python processes independent monotonic origins, so the
# watchdog uses one epoch clock for both deadline creation and polling.
wh_clock_ms() {
  python3 - <<'PY'
import time
print(time.time_ns() // 1_000_000)
PY
}

# Run a command (or a shell function) until an absolute epoch deadline.
# `timeout(1)` is not available on every supported macOS/Linux runner, so the
# small watchdog stays in bash and kills the child on the same portable path.
# 124 mirrors the conventional timeout exit code and is reserved for the
# deadline case; callers can distinguish a browser failure from a deadline.
wh_run_with_deadline() {
  local deadline_ms=$1
  shift
  [[ "$deadline_ms" =~ ^[0-9]+$ ]] || {
    echo "error: invalid epoch deadline: $deadline_ms" >&2
    return 2
  }
  [[ "$#" -gt 0 ]] || {
    echo "error: wh_run_with_deadline requires a command" >&2
    return 2
  }

  # Enable bash job control for this one launch so the background command gets
  # its own process group.  Preserve the caller's setting after the launch;
  # this keeps shell-function commands usable on macOS where `setsid` is not a
  # guaranteed system utility, while still giving Linux the same group-bound
  # kill semantics.
  local monitor_was_enabled=0
  case "$-" in
    *m*) monitor_was_enabled=1 ;;
  esac
  set -m
  "$@" &
  local child_pid=$!
  if [[ "$monitor_was_enabled" -eq 0 ]]; then
    set +m
  fi
  while kill -0 "$child_pid" >/dev/null 2>&1; do
    local now_ms
    now_ms=$(wh_clock_ms)
    if (( now_ms >= deadline_ms )); then
      kill -TERM "-$child_pid" >/dev/null 2>&1 || kill -TERM "$child_pid" >/dev/null 2>&1 || true
      sleep 0.1
      kill -KILL "-$child_pid" >/dev/null 2>&1 || kill -KILL "$child_pid" >/dev/null 2>&1 || true
      wait "$child_pid" >/dev/null 2>&1 || true
      return 124
    fi
    sleep 0.05
  done
  wait "$child_pid"
}

wh_state_phase() {
  local state_file=$1
  local phase=$2
  local message=${3:-}
  local deadline_ms=${4:-}
  local now_ms
  now_ms=$(wh_clock_ms)
  wh_state_write "$state_file" "$(python3 - "$phase" "$message" "$deadline_ms" "$now_ms" <<'PY'
from __future__ import annotations

import json
import sys

phase, message, deadline_raw, now_raw = sys.argv[1:]
deadline = int(deadline_raw) if deadline_raw.isdigit() else None
now = int(now_raw)
payload = {
    "phase": phase,
    "phase_started_epoch_ms": now,
    "phase_deadline_epoch_ms": deadline,
    "progress": {
        "phase": phase,
        "message": message,
        "updated_epoch_ms": now,
    },
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"
}

wh_state_progress() {
  local state_file=$1
  local message=$2
  local attempt=${3:-}
  local now_ms current_phase
  now_ms=$(wh_clock_ms)
  current_phase=$(wh_state_get "$state_file" phase 2>/dev/null || true)
  wh_state_write "$state_file" "$(python3 - "$current_phase" "$message" "$attempt" "$now_ms" <<'PY'
from __future__ import annotations

import json
import sys

phase, message, attempt_raw, now_raw = sys.argv[1:]
progress = {
    "phase": phase,
    "message": message,
    "updated_epoch_ms": int(now_raw),
}
if attempt_raw.isdigit():
    progress["attempt"] = int(attempt_raw)
print(json.dumps({"progress": progress}))
PY
)"
}

wh_state_get() {
  local state_file=$1
  local key=$2
  python3 - "$state_file" "$key" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

state_path = pathlib.Path(sys.argv[1])
key = sys.argv[2]
if not state_path.exists():
    raise SystemExit(1)
data = json.loads(state_path.read_text(encoding="utf-8"))
value = data.get(key)
if value is None:
    raise SystemExit(1)
if isinstance(value, bool):
    print("true" if value else "false")
elif isinstance(value, (dict, list)):
    print(json.dumps(value, ensure_ascii=False))
else:
    print(value)
PY
}

wh_state_show() {
  local state_file=$1
  if [[ -f "$state_file" ]]; then
    cat "$state_file"
    return 0
  fi
  echo "error: state file does not exist: $state_file" >&2
  return 1
}

wh_pid_alive() {
  local pid=$1
  [[ -n "$pid" ]] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

wh_process_group_id() {
  local pid=$1
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  ps -o pgid= -p "$pid" | awk 'NF { print $1; exit }'
}

wh_process_group_alive() {
  local pgid=$1
  [[ "$pgid" =~ ^[1-9][0-9]*$ ]] || return 1
  ps -axo pid=,pgid= | awk -v target="$pgid" '$2 == target { found = 1 } END { exit found ? 0 : 1 }'
}

# Terminate only the process group that was recorded at launch.  The caller
# must provide the launcher's PID and the PGID captured from that PID; a live
# PID whose group changed is rejected before any signal is sent.  If the group
# leader was killed already, the durable PID/PGID pair still identifies the
# group that may contain descendants left behind by that failure.
wh_terminate_process_group() {
  local pid=${1:-}
  local pgid=${2:-}
  local timeout_ms=${3:-2000}
  local current_pgid deadline_ms

  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 2
  [[ "$pgid" =~ ^[1-9][0-9]*$ ]] || return 2
  [[ "$timeout_ms" =~ ^[0-9]+$ ]] || return 2
  [[ "$pgid" -gt 1 ]] || return 2

  # Never permit a stale record to target the process group running the
  # cleanup caller itself.
  current_pgid=$(wh_process_group_id "$$" 2>/dev/null || true)
  [[ -n "$current_pgid" && "$pgid" != "$current_pgid" ]] || return 2

  if wh_pid_alive "$pid"; then
    current_pgid=$(wh_process_group_id "$pid" 2>/dev/null || true)
    [[ "$current_pgid" == "$pgid" ]] || return 2
  elif ! wh_process_group_alive "$pgid"; then
    return 0
  fi

  if ! wh_process_group_alive "$pgid"; then
    return 0
  fi
  kill -TERM "-$pgid" >/dev/null 2>&1 || true
  deadline_ms=$(( $(wh_clock_ms) + timeout_ms ))
  while wh_process_group_alive "$pgid"; do
    if (( $(wh_clock_ms) >= deadline_ms )); then
      break
    fi
    sleep 0.05
  done

  if wh_process_group_alive "$pgid"; then
    kill -KILL "-$pgid" >/dev/null 2>&1 || true
    deadline_ms=$(( $(wh_clock_ms) + 1000 ))
    while wh_process_group_alive "$pgid" && (( $(wh_clock_ms) < deadline_ms )); do
      sleep 0.05
    done
  fi

  if wh_process_group_alive "$pgid"; then
    return 1
  fi
  wait "$pid" >/dev/null 2>&1 || true
}

# Launch a shell command in a dedicated process group and expose the recorded
# identity through WH_MANAGED_PID/WH_MANAGED_PGID.  Redirections belong around
# this function call so the child inherits the caller's chosen logs.
wh_start_managed() {
  local monitor_was_enabled=0
  case "$-" in
    *m*) monitor_was_enabled=1 ;;
  esac
  set -m
  "$@" &
  WH_MANAGED_PID=$!
  # The managed process outlives the launching shell in normal harness use;
  # disown it so a later group KILL does not produce a misleading job-control
  # diagnostic on the operator's terminal.
  disown "$WH_MANAGED_PID" >/dev/null 2>&1 || true
  if [[ "$monitor_was_enabled" -eq 0 ]]; then
    set +m
  fi
  WH_MANAGED_PGID="$(wh_process_group_id "$WH_MANAGED_PID" 2>/dev/null || true)"
  if [[ -z "$WH_MANAGED_PGID" ]]; then
    kill "$WH_MANAGED_PID" >/dev/null 2>&1 || true
    wait "$WH_MANAGED_PID" >/dev/null 2>&1 || true
    echo "error: unable to record managed process group for PID $WH_MANAGED_PID" >&2
    return 1
  fi
}

wh_env_file_get() {
  local env_file=$1
  local key=$2
  python3 - "$env_file" "$key" <<'PY'
from __future__ import annotations

import pathlib
import sys

env_path = pathlib.Path(sys.argv[1])
key = sys.argv[2]
if not env_path.exists():
    raise SystemExit(1)
for raw in env_path.read_text(encoding="utf-8").splitlines():
    if "=" not in raw:
        continue
    left, right = raw.split("=", 1)
    if left == key:
        print(right)
        raise SystemExit(0)
raise SystemExit(1)
PY
}
