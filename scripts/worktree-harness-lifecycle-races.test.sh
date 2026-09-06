#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

TMP_DIR="$(mktemp -d)"
TEST_PARENT="$TMP_DIR/test-root"
HARNESS_ROOT="$TEST_PARENT/harness"
META_FILE="$(wh_runtime_meta_file "$HARNESS_ROOT")"
CRASH_METADATA_MARKER="$TMP_DIR/crash-metadata-published"
CRASH_CHILD_PID_FILE="$TMP_DIR/crash-child.pid"
CRASH_LAUNCHER_PID=""
CRASH_LAUNCHER_PGID=""
CRASH_LAUNCHER_IDENTITY=""
FAKE_LAUNCHER="$TMP_DIR/crash-window-launcher.sh"
UP_PID=""

mkdir -p "$TEST_PARENT"
printf 'oasis7-harness-lifecycle-test-v1\n' >"$TEST_PARENT/.oasis7-harness-test-root"

cleanup() {
  set +e
  if [[ -n "$UP_PID" ]]; then
    kill -KILL "$UP_PID" >/dev/null 2>&1 || true
    wait "$UP_PID" >/dev/null 2>&1 || true
  fi
  OASIS7_HARNESS_TEST_ROOT="$HARNESS_ROOT" \
    OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
  OASIS7_HARNESS_LIFECYCLE_LOCK_TIMEOUT_MS=500 \
    "$ROOT_DIR/scripts/worktree-harness.sh" down >/dev/null 2>&1 || true
  if [[ -n "$CRASH_LAUNCHER_PID" ]]; then
    wh_terminate_process_group "$CRASH_LAUNCHER_PID" "$CRASH_LAUNCHER_PGID" 100 "$CRASH_LAUNCHER_IDENTITY" >/dev/null 2>&1 || true
  fi
  if [[ -f "$CRASH_CHILD_PID_FILE" ]]; then
    kill "$(tr -d '\n' <"$CRASH_CHILD_PID_FILE")" >/dev/null 2>&1 || true
  fi
  rm -rf -- "$TMP_DIR"
}
trap cleanup EXIT

cat >"$FAKE_LAUNCHER" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail

meta_file=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --meta-file) meta_file="$2"; shift 2 ;;
    *) shift ;;
  esac
done

source "$(pwd)/scripts/worktree-harness-lib.sh"

# The production stack nests its managed launcher under the outer harness
# process group. Keep this fake outer wrapper alive while a detached inner
# process group publishes the complete non-ready metadata.
python3 - "$meta_file" "${FAKE_CRASH_METADATA_MARKER:?FAKE_CRASH_METADATA_MARKER is required}" "${FAKE_CRASH_CHILD_PID_FILE:?FAKE_CRASH_CHILD_PID_FILE is required}" <<'PY' &
import ctypes
import os
import pathlib
import subprocess
import sys
import time

meta_path = pathlib.Path(sys.argv[1])
marker_path = pathlib.Path(sys.argv[2])
child_path = pathlib.Path(sys.argv[3])
os.setsid()
child = subprocess.Popen(["sleep", "300"])
child_path.write_text(f"{child.pid}\n", encoding="utf-8")

proc_stat = pathlib.Path(f"/proc/{os.getpid()}/stat")
if proc_stat.exists():
    contents = proc_stat.read_text(encoding="utf-8")
    boot_id = pathlib.Path("/proc/sys/kernel/random/boot_id").read_text(encoding="utf-8").strip()
    right_paren = contents.rfind(") ")
    fields = contents[right_paren + 2 :].split()
    identity = f"proc-starttime:{boot_id}:{fields[19]}"
elif sys.platform == "darwin":
    class ProcBsdInfo(ctypes.Structure):
        _fields_ = [
            ("pbi_flags", ctypes.c_uint32),
            ("pbi_status", ctypes.c_uint32),
            ("pbi_xstatus", ctypes.c_uint32),
            ("pbi_pid", ctypes.c_uint32),
            ("pbi_ppid", ctypes.c_uint32),
            ("pbi_uid", ctypes.c_uint32),
            ("pbi_gid", ctypes.c_uint32),
            ("pbi_ruid", ctypes.c_uint32),
            ("pbi_rgid", ctypes.c_uint32),
            ("pbi_svuid", ctypes.c_uint32),
            ("pbi_svgid", ctypes.c_uint32),
            ("rfu_1", ctypes.c_uint32),
            ("pbi_comm", ctypes.c_char * 16),
            ("pbi_name", ctypes.c_char * 32),
            ("pbi_nfiles", ctypes.c_uint32),
            ("pbi_pgid", ctypes.c_uint32),
            ("pbi_pjobc", ctypes.c_uint32),
            ("e_tdev", ctypes.c_uint32),
            ("e_tpgid", ctypes.c_uint32),
            ("pbi_nice", ctypes.c_int32),
            ("pbi_start_tvsec", ctypes.c_uint64),
            ("pbi_start_tvusec", ctypes.c_uint64),
        ]

    try:
        libproc = ctypes.CDLL("/usr/lib/libproc.dylib")
        proc_pidinfo = libproc.proc_pidinfo
        proc_pidinfo.argtypes = [
            ctypes.c_int,
            ctypes.c_int,
            ctypes.c_uint64,
            ctypes.c_void_p,
            ctypes.c_int,
        ]
        proc_pidinfo.restype = ctypes.c_int
        info = ProcBsdInfo()
        size = ctypes.sizeof(info)
        if proc_pidinfo(os.getpid(), 3, 0, ctypes.byref(info), size) != size:
            raise SystemExit(1)
        if info.pbi_pid != os.getpid():
            raise SystemExit(1)
        identity = f"mac-proc-start:{info.pbi_start_tvsec}:{info.pbi_start_tvusec}"
    except (AttributeError, OSError, TypeError, ValueError):
        raise SystemExit(1)
else:
    raise SystemExit(1)

meta_path.parent.mkdir(parents=True, exist_ok=True)
meta_path.write_text(
    "".join([
        "RUN_ID=crash-window\n",
        f"LAUNCHER_PID={os.getpid()}\n",
        f"LAUNCHER_PGID={os.getpgrp()}\n",
        f"LAUNCHER_IDENTITY={identity}\n",
        "STACK_READY=0\n",
    ]),
    encoding="utf-8",
)
marker_path.touch()
while True:
    time.sleep(1)
PY
inner_wrapper_pid=$!
wait_for_inner_wrapper() {
  while kill -0 "$inner_wrapper_pid" >/dev/null 2>&1; do
    sleep 1
  done
}
# Keep the outer wrapper alive until the harness owns or explicitly tears down
# the inner launcher record.
wait_for_inner_wrapper
while :; do
  sleep 1
done
FAKE
chmod +x "$FAKE_LAUNCHER"

run_harness() {
  OASIS7_HARNESS_TEST_ROOT="$HARNESS_ROOT" \
    OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
    FAKE_CRASH_CHILD_PID_FILE="$CRASH_CHILD_PID_FILE" \
    FAKE_CRASH_METADATA_MARKER="$CRASH_METADATA_MARKER" \
    "$ROOT_DIR/scripts/worktree-harness.sh" "$@"
}

# The crash target must be the actual harness process.  Keep the generic
# helper synchronous-safe for cleanup and use exec only for the backgrounded
# crash lane, so $! names worktree-harness.sh rather than a function subshell.
run_harness_exec() {
  exec env \
    OASIS7_HARNESS_TEST_ROOT="$HARNESS_ROOT" \
    OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
    FAKE_CRASH_CHILD_PID_FILE="$CRASH_CHILD_PID_FILE" \
    FAKE_CRASH_METADATA_MARKER="$CRASH_METADATA_MARKER" \
    "$ROOT_DIR/scripts/worktree-harness.sh" "$@"
}

echo 'lifecycle race: outer crash after complete STACK_READY=0 metadata' >&2
set +e
run_harness_exec up --startup-timeout 10 >"$TMP_DIR/crash-up.log" 2>&1 &
UP_PID=$!
set -e
for _ in $(seq 1 1000); do
  [[ -e "$CRASH_METADATA_MARKER" ]] && break
  sleep 0.01
done
[[ -e "$CRASH_METADATA_MARKER" ]] || {
  cat "$TMP_DIR/crash-up.log" >&2 || true
  echo 'lifecycle race: non-ready metadata was not published' >&2
  exit 1
}
crash_launcher_pid="$(wh_env_file_get "$META_FILE" LAUNCHER_PID)"
crash_launcher_pgid="$(wh_env_file_get "$META_FILE" LAUNCHER_PGID)"
crash_launcher_identity="$(wh_env_file_get "$META_FILE" LAUNCHER_IDENTITY)"
CRASH_LAUNCHER_PID="$crash_launcher_pid"
CRASH_LAUNCHER_PGID="$crash_launcher_pgid"
CRASH_LAUNCHER_IDENTITY="$crash_launcher_identity"
[[ "$crash_launcher_pid" =~ ^[1-9][0-9]*$ && "$crash_launcher_pgid" =~ ^[1-9][0-9]*$ ]] || exit 1
crash_harness_command=""
for _ in $(seq 1 50); do
  crash_harness_command="$(ps -o command= -p "$UP_PID" 2>/dev/null || true)"
  [[ "$crash_harness_command" == *"worktree-harness.sh up"* ]] && break
  sleep 0.01
done
[[ "$crash_harness_command" == *"worktree-harness.sh up"* ]] || {
  echo "lifecycle race: signaled PID $UP_PID is not worktree-harness.sh: $crash_harness_command" >&2
  exit 1
}
kill -KILL "$UP_PID"
set +e
wait "$UP_PID"
up_status=$?
set -e
UP_PID=""
[[ "$up_status" -eq 137 ]] || {
  echo "lifecycle race: outer crash returned $up_status, expected 137" >&2
  exit 1
}
set +e
run_harness down >"$TMP_DIR/crash-down.log" 2>&1
crash_down_status=$?
set -e
if [[ "$crash_down_status" -ne 0 ]]; then
  echo "lifecycle race: down failed with status $crash_down_status" >&2
  cat "$HARNESS_ROOT/state.json" >&2 || true
  cat "$TMP_DIR/crash-down.log" >&2 || true
  exit 1
fi
if wh_pid_alive "$crash_launcher_pid" || wh_process_group_alive "$crash_launcher_pgid"; then
  echo 'lifecycle race: down left the authenticated inner launcher alive after outer crash' >&2
  echo '--- crash state after down ---' >&2
  cat "$HARNESS_ROOT/state.json" >&2 || true
  cat "$TMP_DIR/crash-down.log" >&2 || true
  exit 1
fi
[[ ! -e "$HARNESS_ROOT/.ports.reservation.json" ]] || {
  echo 'lifecycle race: outer-crash cleanup released no local reservation' >&2
  exit 1
}
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "stopped", state
assert state["phase"] == "stopped", state
PY

REAL_PYTHON3="$(command -v python3)"
PYTHON_WRAPPER_DIR="$TMP_DIR/python-wrapper"
mkdir -p "$PYTHON_WRAPPER_DIR"
cat >"$PYTHON_WRAPPER_DIR/python3" <<'PYWRAPPER'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-" && "${2:-}" == "${RACE_STATE_FILE:-}" && "${3:-}" == *'"status": "stopped"'* ]]; then
  : >"${RACE_STATE_WRITE_BLOCKED:?RACE_STATE_WRITE_BLOCKED is required}"
  while [[ ! -e "${RACE_STATE_WRITE_RELEASE:?RACE_STATE_WRITE_RELEASE is required}" ]]; do
    sleep 0.01
  done
fi
exec "${RACE_REAL_PYTHON3:?RACE_REAL_PYTHON3 is required}" "$@"
PYWRAPPER
chmod +x "$PYTHON_WRAPPER_DIR/python3"

for read_action in status url logs; do
  read_root="$TMP_DIR/read-$read_action/harness"
  mkdir -p "$(dirname "$read_root")"
  printf 'oasis7-harness-lifecycle-test-v1\n' >"$(dirname "$read_root")/.oasis7-harness-test-root"
  read_state="$read_root/state.json"
  wh_state_write "$read_state" "$(python3 - "$read_action" <<'PY'
import json
import sys

print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": 999999991,
    "harness_pgid": 999999991,
    "harness_identity": "old-harness-generation",
    "launcher_pid": 999999992,
    "launcher_pgid": 999999992,
    "launcher_identity": "old-launcher-generation",
    "viewer_url": "http://127.0.0.1:9/",
    "generation_token": f"old-{sys.argv[1]}",
}))
PY
)"
  lock_release="$TMP_DIR/read-$read_action.lock-release"
  lock_held="$TMP_DIR/read-$read_action.lock-held"
  state_write_blocked="$TMP_DIR/read-$read_action.state-write-blocked"
  (
    wh_lifecycle_lock_acquire "$read_root"
    : >"$lock_held"
    while [[ ! -e "$lock_release" ]]; do
      sleep 0.01
    done
    wh_lifecycle_lock_release
  ) &
  lock_holder_pid=$!
  for _ in $(seq 1 100); do
    [[ -e "$lock_held" ]] && break
    sleep 0.01
  done
  [[ -e "$lock_held" ]] || exit 1

  set +e
  read_args=("$read_action")
  [[ "$read_action" == "status" ]] && read_args+=(--json)
  env PATH="$PYTHON_WRAPPER_DIR:$PATH" \
      RACE_REAL_PYTHON3="$REAL_PYTHON3" \
      RACE_STATE_FILE="$read_state" \
      RACE_STATE_WRITE_BLOCKED="$state_write_blocked" \
      RACE_STATE_WRITE_RELEASE="$lock_release" \
      OASIS7_HARNESS_TEST_ROOT="$read_root" \
      "$ROOT_DIR/scripts/worktree-harness.sh" "${read_args[@]}" >"$TMP_DIR/read-$read_action.log" 2>&1 &
  read_pid=$!
  set -e
  sleep 0.25
  wh_state_write "$read_state" "{\"status\": \"booting\", \"phase\": \"starting_launcher\", \"generation_token\": \"replacement-$read_action\"}"
  : >"$lock_release"
  wait "$lock_holder_pid"
  set +e
  wait "$read_pid"
  read_status=$?
  set -e
  if [[ "$read_action" == "status" || "$read_action" == "logs" ]]; then
    [[ "$read_status" -eq 0 ]] || {
      cat "$TMP_DIR/read-$read_action.log" >&2 || true
      echo "lifecycle race: $read_action failed after replacement generation" >&2
      exit 1
    }
  else
    [[ "$read_status" -ne 0 ]] || {
      echo 'lifecycle race: url unexpectedly accepted replacement booting state' >&2
      exit 1
    }
  fi
  python3 - "$read_state" "$read_action" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "booting", (sys.argv[2], state)
assert state["generation_token"] == f"replacement-{sys.argv[2]}", (sys.argv[2], state)
PY
  if [[ -e "$state_write_blocked" ]]; then
    echo "lifecycle race: $read_action reached stale state mutation without lifecycle serialization" >&2
    exit 1
  fi
done

echo 'worktree harness lifecycle race contracts: PASS'
