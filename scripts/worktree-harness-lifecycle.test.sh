#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

wait_for_marker() {
  local marker=$1
  local timeout_secs=$2
  local description=$3
  local deadline_ms=$(( $(wh_clock_ms) + timeout_secs * 1000 ))
  while [[ ! -e "$marker" ]]; do
    if (( $(wh_clock_ms) >= deadline_ms )); then
      echo "lifecycle acceptance: timed out waiting for ${description}: ${marker}" >&2
      return 1
    fi
    sleep 0.05
  done
}

TMP_DIR="$(mktemp -d)"
WORKTREE_ID="$(python3 - "$PWD" <<'PY'
import hashlib
import pathlib
import sys

print(f"wt-{hashlib.sha256(str(pathlib.Path(sys.argv[1]).resolve()).encode()).hexdigest()[:8]}")
PY
)"
HARNESS_ROOT="$ROOT_DIR/output/harness/$WORKTREE_ID"
READY_CHILD_PID_FILE="$TMP_DIR/ready-child.pid"
TIMEOUT_CHILD_PID_FILE="$TMP_DIR/timeout-child.pid"
FAKE_LAUNCHER="$TMP_DIR/fake-launcher.sh"
SENTINEL_PID=""
UNRELATED_PID=""
UNRELATED_PGID=""
UNRELATED_IDENTITY=""
READY_HARNESS_PID=""
READY_HARNESS_PGID=""
READY_HARNESS_IDENTITY=""
READINESS_HARNESS_PID=""
READINESS_HARNESS_PGID=""
READINESS_HARNESS_IDENTITY=""

cleanup_recorded_group() {
  local pid=${1:-}
  local pgid=${2:-}
  local identity=${3:-}
  [[ -n "$pid" ]] || return 0
  wh_terminate_process_group "$pid" "$pgid" 500 "$identity" >/dev/null 2>&1 || true
}

cleanup() {
  set +e
  if [[ -n "$SENTINEL_PID" ]]; then
    kill "$SENTINEL_PID" >/dev/null 2>&1 || true
    wait "$SENTINEL_PID" >/dev/null 2>&1 || true
  fi
  OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh down >/dev/null 2>&1 || true
  cleanup_recorded_group "$UNRELATED_PID" "$UNRELATED_PGID" "$UNRELATED_IDENTITY"
  cleanup_recorded_group "$READINESS_HARNESS_PID" "$READINESS_HARNESS_PGID" "$READINESS_HARNESS_IDENTITY"
  cleanup_recorded_group "$READY_HARNESS_PID" "$READY_HARNESS_PGID" "$READY_HARNESS_IDENTITY"
  OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh down >/dev/null 2>&1 || true
  rm -rf "$HARNESS_ROOT" "$TMP_DIR"
}
trap cleanup EXIT

rm -rf "$HARNESS_ROOT"
cat >"$FAKE_LAUNCHER" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail

viewer_port=""
meta_file=""
run_id="fake-run"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --viewer-port) viewer_port="$2"; shift 2 ;;
    --meta-file) meta_file="$2"; shift 2 ;;
    --run-id) run_id="$2"; shift 2 ;;
    *) shift ;;
  esac
done

child_pid_file="${FAKE_LAUNCHER_CHILD_PID_FILE:?FAKE_LAUNCHER_CHILD_PID_FILE is required}"
sleep 300 &
child_pid=$!
echo "$child_pid" >"$child_pid_file"
source "$(pwd)/scripts/worktree-harness-lib.sh"

python3 - "$viewer_port" <<'PY' &
import http.server
import sys

server = http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), http.server.SimpleHTTPRequestHandler)
server.serve_forever()
PY

if [[ "${FAKE_LAUNCHER_MODE:-ready}" == "ready" ]]; then
  launcher_pgid="$(ps -o pgid= -p "$$" | awk 'NF { print $1; exit }')"
  mkdir -p "$(dirname "$meta_file")"
  {
  printf 'RUN_ID=%s\n' "$run_id"
  printf 'LAUNCHER_PID=%s\n' "$$"
  printf 'LAUNCHER_PGID=%s\n' "$launcher_pgid"
  printf 'LAUNCHER_IDENTITY=%s\n' "$(wh_process_identity "$$")"
    printf 'STACK_READY=1\n'
    printf 'GAME_URL=http://127.0.0.1:%s/\n' "$viewer_port"
  } >"$meta_file"
fi

while :; do
  sleep 1
done
FAKE
rtk chmod +x "$FAKE_LAUNCHER"

SENTINEL_PID=""
sleep 300 &
SENTINEL_PID=$!
FAKE_LAUNCHER_CHILD_PID_FILE="$READY_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
./scripts/worktree-harness.sh up --startup-timeout 5 >"$TMP_DIR/ready-up.log" 2>&1

python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "ready", state
assert state["launcher_pgid"] == state["harness_pgid"], state
assert state["launcher_pid"] != state["harness_pid"], state
assert state["port_reservation_token"], state
assert state["harness_identity"], state
assert state["launcher_identity"], state
PY

status_json="$TMP_DIR/status.json"
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh status --json >"$status_json"
viewer_url="$(OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh url)"
[[ "$viewer_url" == http://127.0.0.1:* ]] || {
  echo "lifecycle acceptance: url did not return the ready viewer URL" >&2
  exit 1
}
ready_viewer_port="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["viewer_port"])
PY
)"
ready_child_pid="$(cat "$READY_CHILD_PID_FILE")"
ready_launcher_pid="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["launcher_pid"])
PY
)"
READY_HARNESS_PID="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["harness_pid"])
PY
)"
READY_HARNESS_PGID="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["harness_pgid"])
PY
)"
READY_HARNESS_IDENTITY="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["harness_identity"])
PY
)"
ready_launcher_pgid="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["launcher_pgid"])
PY
)"
ready_launcher_identity="$(python3 - "$status_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["launcher_identity"])
PY
)"

# A live unrelated process with the same shape of PID/PGID record must not be
# accepted as the running harness.  These four consumers previously trusted
# kill -0 and therefore all accepted this stale record.
wh_start_managed sleep 300 >"$TMP_DIR/unrelated-group.log" 2>&1
UNRELATED_PID="$WH_MANAGED_PID"
UNRELATED_PGID="$WH_MANAGED_PGID"
UNRELATED_IDENTITY="$WH_MANAGED_IDENTITY"
set_stale_live_record() {
  wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - \
    "$UNRELATED_PID" "$UNRELATED_PGID" <<'PY'
import json
import sys

print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": int(sys.argv[1]),
    "harness_pgid": int(sys.argv[2]),
    "harness_identity": "stale-unrelated-harness-incarnation",
    "launcher_pid": int(sys.argv[1]),
    "launcher_pgid": int(sys.argv[2]),
    "launcher_identity": "stale-unrelated-launcher-incarnation",
}))
PY
)"
}
restore_ready_record() {
  wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - \
    "$READY_HARNESS_PID" "$READY_HARNESS_PGID" "$READY_HARNESS_IDENTITY" \
    "$ready_launcher_pid" "$ready_launcher_pgid" "$ready_launcher_identity" \
    "$ready_viewer_port" <<'PY'
import json
import sys

print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": int(sys.argv[1]),
    "harness_pgid": int(sys.argv[2]),
    "harness_identity": sys.argv[3],
    "launcher_pid": int(sys.argv[4]),
    "launcher_pgid": int(sys.argv[5]),
    "launcher_identity": sys.argv[6],
    "viewer_url": f"http://127.0.0.1:{sys.argv[7]}/",
}))
PY
)"
}

set_launcher_record() {
  local launcher_pid=${1:-}
  local launcher_pgid=${2:-}
  local launcher_identity=${3:-}
  wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - \
    "$READY_HARNESS_PID" "$READY_HARNESS_PGID" "$READY_HARNESS_IDENTITY" \
    "$launcher_pid" "$launcher_pgid" "$launcher_identity" \
    "$ready_viewer_port" <<'PY'
import json
import sys

launcher_pid = sys.argv[4]
launcher_pgid = sys.argv[5]
print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": int(sys.argv[1]),
    "harness_pgid": int(sys.argv[2]),
    "harness_identity": sys.argv[3],
    "launcher_pid": int(launcher_pid) if launcher_pid else None,
    "launcher_pgid": int(launcher_pgid) if launcher_pgid else None,
    "launcher_identity": sys.argv[6] or None,
    "viewer_url": f"http://127.0.0.1:{sys.argv[7]}/",
}))
PY
  )"
}

set_stale_live_record
set +e
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh status --json >"$TMP_DIR/stale-status.log" 2>&1
stale_status_rc=$?
set -e
[[ "$stale_status_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: status accepted unrelated live PID with stale identity" >&2
  exit 1
}
restore_ready_record

# A ready harness record is not sufficient on its own.  Missing and dead
# launcher records must fail closed for status instead of retaining or
# reporting ready.  The stale launcher case below also guards the
# already-running up fast path.
set_launcher_record
set +e
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh status --json >"$TMP_DIR/missing-launcher-status.log" 2>&1
missing_launcher_status_rc=$?
set -e
[[ "$missing_launcher_status_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: status retained ready with missing launcher record" >&2
  exit 1
}
restore_ready_record

set_launcher_record "999999999" "999999999" "dead-launcher-incarnation"
set +e
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh status --json >"$TMP_DIR/dead-launcher-status.log" 2>&1
dead_launcher_status_rc=$?
set -e
[[ "$dead_launcher_status_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: status retained ready with dead launcher record" >&2
  exit 1
}
restore_ready_record

set_launcher_record "$UNRELATED_PID" "$UNRELATED_PGID" "stale-unrelated-launcher-incarnation"
set +e
FAKE_LAUNCHER_CHILD_PID_FILE="$READY_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh up --startup-timeout 5 >"$TMP_DIR/stale-launcher-up.log" 2>&1
stale_launcher_up_rc=$?
set -e
[[ "$stale_launcher_up_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: up accepted live harness with stale launcher identity" >&2
  exit 1
}
restore_ready_record

set_stale_live_record
set +e
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh url >"$TMP_DIR/stale-url.log" 2>&1
stale_url_rc=$?
set -e
[[ "$stale_url_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: url accepted unrelated live PID with stale identity" >&2
  exit 1
}
restore_ready_record

set_stale_live_record
set +e
FAKE_LAUNCHER_CHILD_PID_FILE="$READY_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh up --startup-timeout 5 >"$TMP_DIR/stale-up.log" 2>&1
stale_up_rc=$?
set -e
[[ "$stale_up_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: up accepted unrelated live PID with stale identity" >&2
  exit 1
}
restore_ready_record

OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh down
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "stopped", state
assert state["phase"] == "stopped", state
PY

for _ in $(seq 1 40); do
  if ! kill -0 "$ready_child_pid" >/dev/null 2>&1 && ! kill -0 "$ready_launcher_pid" >/dev/null 2>&1; then
    break
  fi
  sleep 0.05
done
if kill -0 "$ready_child_pid" >/dev/null 2>&1 || kill -0 "$ready_launcher_pid" >/dev/null 2>&1; then
  echo "lifecycle acceptance: ready launcher process tree survived down" >&2
  exit 1
fi
if ! kill -0 "$SENTINEL_PID" >/dev/null 2>&1; then
  echo "lifecycle acceptance: unrelated sentinel process was killed" >&2
  exit 1
fi
python3 - "$ready_viewer_port" <<'PY'
import socket
import sys
import time

port = int(sys.argv[1])
for _ in range(40):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=0.1):
            pass
    except OSError:
        break
    time.sleep(0.05)
else:
    raise SystemExit("lifecycle acceptance: viewer port was not released after down")
PY

STARTUP_TIMEOUT_SECS=5
# Launch synchronization begins before the harness establishes its own
# startup deadline. Allow one bounded startup budget for setup and one for
# the post-launch handoff without changing the harness deadline itself.
LAUNCH_SYNC_TIMEOUT_SECS=$((STARTUP_TIMEOUT_SECS * 2))
READINESS_DELAY_FILE="$TMP_DIR/readiness-delay.marker"
READINESS_ACK_FILE="$TMP_DIR/readiness-delay.ack"
READINESS_ACKED_FILE="$TMP_DIR/readiness-delay.acked"
READINESS_CHILD_PID_FILE="$TMP_DIR/readiness-child.pid"
rm -f "$READINESS_DELAY_FILE" "$READINESS_ACK_FILE" "$READINESS_ACKED_FILE" "$READINESS_CHILD_PID_FILE"
FAKE_LAUNCHER_CHILD_PID_FILE="$READINESS_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE="$READINESS_DELAY_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACK_FILE="$READINESS_ACK_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACKED_FILE="$READINESS_ACKED_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS=2 \
./scripts/worktree-harness.sh up --startup-timeout "$STARTUP_TIMEOUT_SECS" >"$TMP_DIR/readiness-up.log" 2>&1 &
readiness_up_pid=$!
wait_for_marker "$READINESS_DELAY_FILE" "$LAUNCH_SYNC_TIMEOUT_SECS" "readiness launch synchronization" || {
  cat "$TMP_DIR/readiness-up.log" >&2 || true
  exit 1
}
READINESS_HARNESS_PID="$(wh_state_get "$HARNESS_ROOT/state.json" harness_pid)"
READINESS_HARNESS_PGID="$(wh_state_get "$HARNESS_ROOT/state.json" harness_pgid)"
READINESS_HARNESS_IDENTITY="$(wh_state_get "$HARNESS_ROOT/state.json" harness_identity)"
wh_state_write "$HARNESS_ROOT/state.json" '{"harness_identity": "stale-readiness-incarnation"}'
: >"$READINESS_ACK_FILE"
wait_for_marker "$READINESS_ACKED_FILE" "$STARTUP_TIMEOUT_SECS" "stale identity mutation acknowledgement" || {
  cat "$TMP_DIR/readiness-up.log" >&2 || true
  exit 1
}
set +e
wait "$readiness_up_pid"
readiness_up_rc=$?
set -e
[[ "$readiness_up_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: launcher readiness accepted stale harness identity" >&2
  exit 1
}
wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - "$READINESS_HARNESS_IDENTITY" <<'PY'
import json
import sys
print(json.dumps({"harness_identity": sys.argv[1]}))
PY
)"
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh down >/dev/null 2>&1 || {
  echo "lifecycle acceptance: readiness fixture cleanup failed" >&2
  exit 1
}
if [[ -e "$READINESS_CHILD_PID_FILE" ]]; then
  readiness_child_pid="$(cat "$READINESS_CHILD_PID_FILE")"
  for _ in $(seq 1 40); do
    kill -0 "$readiness_child_pid" >/dev/null 2>&1 || break
    sleep 0.05
  done
  if kill -0 "$readiness_child_pid" >/dev/null 2>&1; then
    echo "lifecycle acceptance: readiness launcher child survived cleanup" >&2
    exit 1
  fi
fi
echo "unrelated live PID identity rejection: status_rc=$stale_status_rc url_rc=$stale_url_rc up_rc=$stale_up_rc readiness_rc=$readiness_up_rc"

HANDOFF_DELAY_FILE="$TMP_DIR/launcher-handoff-delay.marker"
HANDOFF_ACK_FILE="$TMP_DIR/launcher-handoff.ack"
HANDOFF_ACKED_FILE="$TMP_DIR/launcher-handoff.acked"
HANDOFF_CHILD_PID_FILE="$TMP_DIR/launcher-handoff-child.pid"
HANDOFF_META_FILE="$(wh_runtime_meta_file "$HARNESS_ROOT")"
rm -f "$HANDOFF_DELAY_FILE" "$HANDOFF_ACK_FILE" "$HANDOFF_ACKED_FILE" "$HANDOFF_CHILD_PID_FILE"
FAKE_LAUNCHER_CHILD_PID_FILE="$HANDOFF_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE="$HANDOFF_DELAY_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACK_FILE="$HANDOFF_ACK_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACKED_FILE="$HANDOFF_ACKED_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS=2 \
./scripts/worktree-harness.sh up --startup-timeout "$STARTUP_TIMEOUT_SECS" >"$TMP_DIR/launcher-handoff-up.log" 2>&1 &
handoff_up_pid=$!
wait_for_marker "$HANDOFF_DELAY_FILE" "$LAUNCH_SYNC_TIMEOUT_SECS" "launcher readiness handoff synchronization" || {
  cat "$TMP_DIR/launcher-handoff-up.log" >&2 || true
  exit 1
}
for _ in $(seq 1 40); do
  [[ "$(wh_env_file_get "$HANDOFF_META_FILE" STACK_READY 2>/dev/null || true)" == "1" ]] && break
  sleep 0.05
done
[[ "$(wh_env_file_get "$HANDOFF_META_FILE" STACK_READY 2>/dev/null || true)" == "1" ]] || {
  cat "$TMP_DIR/launcher-handoff-up.log" >&2 || true
  echo "lifecycle acceptance: launcher handoff did not publish STACK_READY" >&2
  exit 1
}
python3 - "$HANDOFF_META_FILE" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
lines = path.read_text(encoding="utf-8").splitlines()
updated = []
found = False
for line in lines:
    if line.startswith("LAUNCHER_IDENTITY="):
        updated.append("LAUNCHER_IDENTITY=stale-final-handoff-incarnation")
        found = True
    else:
        updated.append(line)
assert found, lines
path.write_text("\n".join(updated) + "\n", encoding="utf-8")
PY
: >"$HANDOFF_ACK_FILE"
wait_for_marker "$HANDOFF_ACKED_FILE" "$STARTUP_TIMEOUT_SECS" "launcher handoff mutation acknowledgement" || {
  cat "$TMP_DIR/launcher-handoff-up.log" >&2 || true
  exit 1
}
set +e
wait "$handoff_up_pid"
handoff_up_rc=$?
set -e
[[ "$handoff_up_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: ready handoff accepted stale launcher identity" >&2
  exit 1
}
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] != "ready", state
PY
if [[ -e "$HANDOFF_CHILD_PID_FILE" ]]; then
  handoff_child_pid="$(cat "$HANDOFF_CHILD_PID_FILE")"
  for _ in $(seq 1 40); do
    kill -0 "$handoff_child_pid" >/dev/null 2>&1 || break
    sleep 0.05
  done
  if kill -0 "$handoff_child_pid" >/dev/null 2>&1; then
    echo "lifecycle acceptance: stale launcher handoff child survived cleanup" >&2
    exit 1
  fi
fi

CONCURRENT_DELAY_FILE="$TMP_DIR/concurrent-delay.marker"
CONCURRENT_CHILD_PID_FILE="$TMP_DIR/concurrent-child.pid"
rm -f "$CONCURRENT_DELAY_FILE" "$CONCURRENT_CHILD_PID_FILE"
FAKE_LAUNCHER_CHILD_PID_FILE="$CONCURRENT_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE="$CONCURRENT_DELAY_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS=2 \
./scripts/worktree-harness.sh up --startup-timeout "$STARTUP_TIMEOUT_SECS" >"$TMP_DIR/concurrent-up.log" 2>&1 &
concurrent_up_pid=$!
if ! wait_for_marker "$CONCURRENT_DELAY_FILE" "$LAUNCH_SYNC_TIMEOUT_SECS" "concurrent-up launch synchronization"; then
  cat "$TMP_DIR/concurrent-up.log" >&2 || true
  exit 1
fi
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
./scripts/worktree-harness.sh down >"$TMP_DIR/concurrent-down.log" 2>&1 &
concurrent_down_pid=$!
set +e
wait "$concurrent_up_pid"
concurrent_up_status=$?
wait "$concurrent_down_pid"
concurrent_down_status=$?
set -e
if [[ "$concurrent_up_status" -ne 0 || "$concurrent_down_status" -ne 0 ]]; then
  cat "$TMP_DIR/concurrent-up.log" "$TMP_DIR/concurrent-down.log" >&2
  exit 1
fi
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if state["status"] != "stopped" or state["phase"] != "stopped":
    raise SystemExit(f"lifecycle acceptance: concurrent up/down left non-stopped state: {state}")
PY
concurrent_child_pid="$(cat "$CONCURRENT_CHILD_PID_FILE")"
for _ in $(seq 1 40); do
  if ! kill -0 "$concurrent_child_pid" >/dev/null 2>&1; then
    break
  fi
  sleep 0.05
done
if kill -0 "$concurrent_child_pid" >/dev/null 2>&1; then
  echo "lifecycle acceptance: concurrent up/down left an orphan launcher child" >&2
  exit 1
fi

FAKE_LAUNCHER_CHILD_PID_FILE="$TIMEOUT_CHILD_PID_FILE" \
FAKE_LAUNCHER_MODE=timeout \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
./scripts/worktree-harness.sh up --startup-timeout 1 >"$TMP_DIR/timeout-up.log" 2>&1 && {
  echo "lifecycle acceptance: timeout launcher unexpectedly reported success" >&2
  exit 1
} || timeout_status=$?
[[ "${timeout_status:-0}" -ne 0 ]] || exit 1
timeout_child_pid="$(cat "$TIMEOUT_CHILD_PID_FILE")"
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "failed", state
assert "deadline" in state["failure_reason"], state
PY
for _ in $(seq 1 40); do
  if ! kill -0 "$timeout_child_pid" >/dev/null 2>&1; then
    break
  fi
  sleep 0.05
done
if kill -0 "$timeout_child_pid" >/dev/null 2>&1; then
  echo "lifecycle acceptance: timeout launcher child survived cleanup" >&2
  exit 1
fi

FAILURE_COMMON_DIR="$TMP_DIR/failure-common"
failure_ports_json="$TMP_DIR/failure-ports.json"
wh_start_managed sleep 300 >"$TMP_DIR/failure-group.log" 2>&1
failure_pid="$WH_MANAGED_PID"
failure_pgid="$WH_MANAGED_PGID"
failure_identity="$WH_MANAGED_IDENTITY"
wh_resolve_ports_json "$HARNESS_ROOT" "$$" "$(wh_worktree_path)" "$FAILURE_COMMON_DIR" >"$failure_ports_json"
failure_token="$(python3 - "$failure_ports_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())['reservation_token'])
PY
)"
wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - "$failure_pid" "$failure_pgid" "$failure_token" <<'PY'
import json
import sys

print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": int(sys.argv[1]),
    "harness_pgid": int(sys.argv[2]),
    "harness_identity": "unrelated-reused-process-identity",
    "launcher_pid": None,
    "launcher_pgid": None,
    "launcher_identity": None,
    "port_reservation_token": sys.argv[3],
}))
PY
)"
set +e
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" ./scripts/worktree-harness.sh down >"$TMP_DIR/failure-down.log" 2>&1
failure_down_status=$?
set -e
[[ "$failure_down_status" -ne 0 ]] || {
  echo "lifecycle acceptance: failed cleanup unexpectedly reported success" >&2
  exit 1
}
python3 - "$HARNESS_ROOT/state.json" "$HARNESS_ROOT/.ports.reservation.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "failed", state
assert state["phase"] == "cleanup_failed", state
assert state["port_reservation_token"], state
assert pathlib.Path(sys.argv[2]).exists(), "cleanup failure released the reservation"
PY
if ! kill -0 "$failure_pid" >/dev/null 2>&1; then
  echo "lifecycle acceptance: failed cleanup killed a group without proving identity" >&2
  exit 1
fi
wh_terminate_process_group "$failure_pid" "$failure_pgid" 100 "$failure_identity"
wh_release_ports_reservation "$HARNESS_ROOT" "$failure_token" "$FAILURE_COMMON_DIR"

echo "worktree harness lifecycle acceptance: PASS"
