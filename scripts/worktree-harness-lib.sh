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

wh_resolve_ports_json() {
  python3 - "$(wh_worktree_path)" <<'PY'
from __future__ import annotations

import hashlib
import json
import socket
import sys
from pathlib import Path

worktree_path = str(Path(sys.argv[1]).resolve())
seed = int(hashlib.sha256(worktree_path.encode("utf-8")).hexdigest()[:8], 16)
start = 43000 + (seed % 1500) * 10


def free(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            sock.bind(("127.0.0.1", port))
        except OSError:
            return False
    return True


for step in range(1500):
    base = start + step * 10
    ports = [base, base + 1, base + 2, base + 3]
    if ports[-1] > 65000:
        continue
    if all(free(port) for port in ports):
        payload = {
            "viewer_port": ports[0],
            "web_bind": f"127.0.0.1:{ports[1]}",
            "live_bind": f"127.0.0.1:{ports[2]}",
            "chain_status_bind": f"127.0.0.1:{ports[3]}",
        }
        print(json.dumps(payload, ensure_ascii=True))
        break
else:
    raise SystemExit("error: unable to allocate free loopback ports for worktree harness")
PY
}

wh_state_write() {
  local state_file=$1
  local patch_json=$2
  mkdir -p "$(dirname "$state_file")"
  python3 - "$state_file" "$patch_json" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

state_path = pathlib.Path(sys.argv[1])
patch = json.loads(sys.argv[2])
if state_path.exists():
    current = json.loads(state_path.read_text(encoding="utf-8"))
else:
    current = {}
current.update(patch)
state_path.write_text(json.dumps(current, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
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
