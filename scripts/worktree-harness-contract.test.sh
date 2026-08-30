#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

STATE_FILE="$TMP_DIR/state.json"
wh_state_write "$STATE_FILE" '{"status":"booting"}'
wh_state_phase "$STATE_FILE" waiting_metadata "waiting for metadata" "$(wh_clock_ms)"
wh_state_progress "$STATE_FILE" "poll 1" 1

if ! declare -F wh_atomic_write >/dev/null 2>&1; then
  echo "atomic publication contract: missing wh_atomic_write helper" >&2
  exit 1
fi

ATOMIC_STATE_FILE="$TMP_DIR/atomic-state.json"
atomic_writer() {
  local attempt
  for attempt in $(seq 1 24); do
    wh_state_write "$ATOMIC_STATE_FILE" "$(python3 - "$attempt" <<'PY'
import json
import sys

print(json.dumps({"attempt": int(sys.argv[1]), "payload": "x" * 4096}))
PY
)"
  done
}

atomic_writer &
atomic_writer_pid=$!
python3 - "$ATOMIC_STATE_FILE" "$atomic_writer_pid" <<'PY'
import json
import pathlib
import sys
import time

state_path = pathlib.Path(sys.argv[1])
writer_pid = int(sys.argv[2])
while True:
    if state_path.exists():
        try:
            payload = json.loads(state_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise SystemExit(f"atomic publication exposed partial JSON: {exc}")
        assert isinstance(payload.get("attempt"), int), payload
        assert len(payload.get("payload", "")) == 4096, payload
    try:
        import os
        os.kill(writer_pid, 0)
    except OSError:
        break
    time.sleep(0.001)
PY
wait "$atomic_writer_pid"

if ! declare -F wh_terminate_process_group >/dev/null 2>&1; then
  echo "process-tree shutdown contract: missing wh_terminate_process_group helper" >&2
  exit 1
fi

PROCESS_TREE_CHILD_PID_FILE="$TMP_DIR/process-tree-child.pid"
process_tree_fixture() {
  trap '' TERM
  sleep 30 &
  echo "$!" >"$PROCESS_TREE_CHILD_PID_FILE"
  while :; do
    sleep 1
  done
}

wh_start_managed process_tree_fixture >"$TMP_DIR/process-tree.log" 2>&1
process_tree_pid="$WH_MANAGED_PID"
process_tree_pgid="$WH_MANAGED_PGID"
for _ in $(seq 1 20); do
  [[ -s "$PROCESS_TREE_CHILD_PID_FILE" ]] && break
  sleep 0.05
done
process_tree_child_pid="$(cat "$PROCESS_TREE_CHILD_PID_FILE")"
wh_terminate_process_group "$process_tree_pid" "$process_tree_pgid" 100
if kill -0 "$process_tree_pid" >/dev/null 2>&1 || kill -0 "$process_tree_child_pid" >/dev/null 2>&1; then
  echo "process-tree shutdown left a managed process alive" >&2
  exit 1
fi

PORT_ROOT="$TMP_DIR/port-reservations"
mkdir -p "$PORT_ROOT"
PORT_ONE_JSON="$TMP_DIR/ports-one.json"
PORT_TWO_JSON="$TMP_DIR/ports-two.json"
set +e
wh_resolve_ports_json "$PORT_ROOT" "$$" >"$PORT_ONE_JSON" 2>"$TMP_DIR/ports-one.err" &
port_one_pid=$!
wh_resolve_ports_json "$PORT_ROOT" "$$" >"$PORT_TWO_JSON" 2>"$TMP_DIR/ports-two.err" &
port_two_pid=$!
wait "$port_one_pid"
port_one_status=$?
wait "$port_two_pid"
port_two_status=$?
set -e
if [[ "$port_one_status" -eq 0 && "$port_two_status" -eq 0 ]]; then
  first_viewer_port="$(python3 - "$PORT_ONE_JSON" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["viewer_port"])
PY
)"
  second_viewer_port="$(python3 - "$PORT_TWO_JSON" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["viewer_port"])
PY
)"
  if [[ "$first_viewer_port" == "$second_viewer_port" ]]; then
    echo "port reservation contract: concurrent callers received the same viewer port" >&2
    exit 1
  fi
fi

STALE_PORT_ROOT="$TMP_DIR/stale-port-reservation"
mkdir -p "$STALE_PORT_ROOT"
python3 - "$STALE_PORT_ROOT/.ports.reservation.json" <<'PY'
import json
import pathlib
import sys

pathlib.Path(sys.argv[1]).write_text(
    json.dumps({"schema": 1, "reservation_token": "stale", "owner_pid": 999999999, "ports": {}}) + "\n",
    encoding="utf-8",
)
PY
stale_ports_json="$TMP_DIR/stale-ports.json"
wh_resolve_ports_json "$STALE_PORT_ROOT" "$$" >"$stale_ports_json"
python3 - "$stale_ports_json" "$STALE_PORT_ROOT/.ports.reservation.json" <<'PY'
import json
import pathlib
import sys

payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
reservation = json.loads(pathlib.Path(sys.argv[2]).read_text())
assert payload["reservation_token"] == reservation["reservation_token"], (payload, reservation)
assert payload["reservation_token"] != "stale", reservation
PY
stale_token="$(python3 - "$stale_ports_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())["reservation_token"])
PY
)"
wh_release_ports_reservation "$STALE_PORT_ROOT" "$stale_token"
[[ ! -e "$STALE_PORT_ROOT/.ports.reservation.json" ]] || {
  echo "port reservation contract: release left reservation behind" >&2
  exit 1
}

python3 - "$STATE_FILE" <<'PY'
import json
import sys

state = json.load(open(sys.argv[1], encoding="utf-8"))
assert state["phase"] == "waiting_metadata", state
assert state["progress"]["phase"] == "waiting_metadata", state
assert state["progress"]["message"] == "poll 1", state
assert state["progress"]["attempt"] == 1, state
assert isinstance(state["phase_started_epoch_ms"], int), state
PY

deadline="$(wh_clock_ms)"
set +e
wh_run_with_deadline "$((deadline + 100))" bash -c 'sleep 2' >/dev/null 2>&1
timeout_status=$?
set -e
[[ "$timeout_status" -eq 124 ]] || {
  echo "expected deadline watchdog status 124, got $timeout_status" >&2
  exit 1
}

wh_run_with_deadline "$((deadline + 3000))" bash -c 'sleep 0.05'

DESCENDANT_PID_FILE="$TMP_DIR/descendant.pid"
deadline="$(wh_clock_ms)"
descendant_fixture() {
  sleep 5 &
  echo "$!" >"$DESCENDANT_PID_FILE"
  wait
}
set +e
wh_run_with_deadline "$((deadline + 100))" descendant_fixture >/dev/null 2>&1
descendant_status=$?
set -e
[[ "$descendant_status" -eq 124 ]] || {
  echo "expected descendant fixture deadline status 124, got $descendant_status" >&2
  exit 1
}
descendant_pid="$(cat "$DESCENDANT_PID_FILE")"
for _ in $(seq 1 20); do
  if ! kill -0 "$descendant_pid" >/dev/null 2>&1; then
    break
  fi
  sleep 0.05
done
if kill -0 "$descendant_pid" >/dev/null 2>&1; then
  echo "deadline watchdog left descendant process alive: $descendant_pid" >&2
  exit 1
fi

bash -n "$ROOT_DIR/scripts/worktree-harness.sh" "$ROOT_DIR/scripts/worktree-harness-lib.sh"
grep -Fq -- '--startup-timeout <secs>' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'wh_state_phase "$STATE_FILE" "waiting_metadata"' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'smoke_step "open" ab_open' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'smoke_step "screenshot" ab_screenshot' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'wh_atomic_write "$META_FILE"' "$ROOT_DIR/scripts/run-launcher-stack.sh"
! grep -Fq -- 'seq 1 180' "$ROOT_DIR/scripts/worktree-harness.sh"

echo "worktree harness contract: PASS"
