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

LIFECYCLE_LOCK_UNCERTAINTY_ROOT="$TMP_DIR/lifecycle-lock-identity-uncertainty"
mkdir -p "$LIFECYCLE_LOCK_UNCERTAINTY_ROOT"
LIFECYCLE_LOCK_UNCERTAINTY_PATH="$LIFECYCLE_LOCK_UNCERTAINTY_ROOT/.lifecycle.lock"
lifecycle_lock_uncertainty_stop="$TMP_DIR/lifecycle-lock-identity-uncertainty.stop"
lifecycle_lock_uncertainty_owner() {
  while [[ ! -e "$lifecycle_lock_uncertainty_stop" ]]; do
    sleep 0.05
  done
}
lifecycle_lock_uncertainty_owner &
lifecycle_lock_uncertainty_owner_pid="$!"
lifecycle_lock_uncertainty_owner_identity="$(wh_process_identity "$lifecycle_lock_uncertainty_owner_pid")"
ln -s \
  "$lifecycle_lock_uncertainty_owner_pid:$lifecycle_lock_uncertainty_owner_identity" \
  "$LIFECYCLE_LOCK_UNCERTAINTY_PATH"
(
  set +e
  original_identity_definition="$(declare -f wh_process_identity)"
  eval "$(printf '%s\n' "$original_identity_definition" | sed 's/^wh_process_identity /original_wh_process_identity /')"
  wh_process_identity() {
    if [[ "${1:-}" == "$lifecycle_lock_uncertainty_owner_pid" ]]; then
      return 1
    fi
    original_wh_process_identity "$@"
  }
  OASIS7_HARNESS_LIFECYCLE_LOCK_TIMEOUT_MS=100 \
    wh_lifecycle_lock_acquire "$LIFECYCLE_LOCK_UNCERTAINTY_ROOT" \
    >"$TMP_DIR/lifecycle-lock-identity-uncertainty.log" 2>&1
  uncertainty_status="$?"
  touch "$lifecycle_lock_uncertainty_stop"
  wait "$lifecycle_lock_uncertainty_owner_pid" >/dev/null 2>&1 || true
  uncertainty_record="$(readlink "$LIFECYCLE_LOCK_UNCERTAINTY_PATH" 2>/dev/null || true)"
  if [[ "$uncertainty_status" -eq 0 || "$uncertainty_record" != \
    "$lifecycle_lock_uncertainty_owner_pid:$lifecycle_lock_uncertainty_owner_identity" ]]; then
    echo "lifecycle lock contract: unavailable owner identity was treated as stale" >&2
    exit 1
  fi
)
rm -f "$LIFECYCLE_LOCK_UNCERTAINTY_PATH"

LIFECYCLE_LOCK_RACE_ROOT="$TMP_DIR/lifecycle-lock-race"
LIFECYCLE_LOCK_RACE_PATH="$LIFECYCLE_LOCK_RACE_ROOT/.lifecycle.lock"
mkdir -p "$LIFECYCLE_LOCK_RACE_ROOT"
ln -s '999999999:stale-incarnation' "$LIFECYCLE_LOCK_RACE_PATH"
LIFECYCLE_LOCK_RACE_BIN="$TMP_DIR/lifecycle-lock-race-bin"
mkdir -p "$LIFECYCLE_LOCK_RACE_BIN"
LIFECYCLE_LOCK_RACE_PAUSED="$TMP_DIR/lifecycle-lock-race-paused"
LIFECYCLE_LOCK_RACE_RELEASE="$TMP_DIR/lifecycle-lock-race-release"
LIFECYCLE_LOCK_RACE_A_ACQUIRED="$TMP_DIR/lifecycle-lock-race-a-acquired"
LIFECYCLE_LOCK_RACE_B_ACQUIRED="$TMP_DIR/lifecycle-lock-race-b-acquired"
cat >"$LIFECYCLE_LOCK_RACE_BIN/rm" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
target="${OASIS7_HARNESS_TEST_RM_TARGET:?}"
paused="${OASIS7_HARNESS_TEST_RM_PAUSED:?}"
release="${OASIS7_HARNESS_TEST_RM_RELEASE:?}"
if [[ "$*" == *"$target"* && ! -e "$paused" ]]; then
  : >"$paused"
  while [[ ! -e "$release" ]]; do
    sleep 0.01
  done
fi
exec /bin/rm "$@"
EOF
chmod +x "$LIFECYCLE_LOCK_RACE_BIN/rm"
(
  export PATH="$LIFECYCLE_LOCK_RACE_BIN:$PATH"
  export OASIS7_HARNESS_TEST_RM_TARGET="$LIFECYCLE_LOCK_RACE_PATH"
  export OASIS7_HARNESS_TEST_RM_PAUSED="$LIFECYCLE_LOCK_RACE_PAUSED"
  export OASIS7_HARNESS_TEST_RM_RELEASE="$LIFECYCLE_LOCK_RACE_RELEASE"
  if wh_lifecycle_lock_acquire "$LIFECYCLE_LOCK_RACE_ROOT"; then
    : >"$LIFECYCLE_LOCK_RACE_A_ACQUIRED"
    sleep 2
    wh_lifecycle_lock_release
  fi
) &
lifecycle_lock_race_a_pid="$!"
for _ in $(seq 1 40); do
  [[ -e "$LIFECYCLE_LOCK_RACE_PAUSED" ]] && break
  sleep 0.05
done
[[ -e "$LIFECYCLE_LOCK_RACE_PAUSED" ]] || {
  echo "lifecycle lock contract: stale recovery did not reach the ownership handoff" >&2
  touch "$LIFECYCLE_LOCK_RACE_RELEASE"
  kill "$lifecycle_lock_race_a_pid" >/dev/null 2>&1 || true
  wait "$lifecycle_lock_race_a_pid" >/dev/null 2>&1 || true
  exit 1
}
(
  if wh_lifecycle_lock_acquire "$LIFECYCLE_LOCK_RACE_ROOT"; then
    : >"$LIFECYCLE_LOCK_RACE_B_ACQUIRED"
    sleep 2
    wh_lifecycle_lock_release
  fi
) &
lifecycle_lock_race_b_pid="$!"
lifecycle_lock_race_b_early=0
for _ in $(seq 1 10); do
  if [[ -e "$LIFECYCLE_LOCK_RACE_B_ACQUIRED" ]]; then
    lifecycle_lock_race_b_early=1
    break
  fi
  sleep 0.05
done
touch "$LIFECYCLE_LOCK_RACE_RELEASE"
set +e
wait "$lifecycle_lock_race_a_pid"
lifecycle_lock_race_a_status="$?"
wait "$lifecycle_lock_race_b_pid"
lifecycle_lock_race_b_status="$?"
set -e
if [[ "$lifecycle_lock_race_b_early" -ne 0 || "$lifecycle_lock_race_a_status" -ne 0 || \
  "$lifecycle_lock_race_b_status" -ne 0 ]]; then
  echo "lifecycle lock contract: concurrent stale recovery lost lock ownership" >&2
  exit 1
fi

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
process_tree_identity="${WH_MANAGED_IDENTITY:-}"
for _ in $(seq 1 20); do
  [[ -s "$PROCESS_TREE_CHILD_PID_FILE" ]] && break
  sleep 0.05
done
process_tree_child_pid="$(cat "$PROCESS_TREE_CHILD_PID_FILE")"
wh_terminate_process_group "$process_tree_pid" "$process_tree_pgid" 100 "$process_tree_identity"
if kill -0 "$process_tree_pid" >/dev/null 2>&1 || kill -0 "$process_tree_child_pid" >/dev/null 2>&1; then
  echo "process-tree shutdown left a managed process alive" >&2
  exit 1
fi

wh_start_managed sleep 30 >"$TMP_DIR/reused-identity.log" 2>&1
reused_identity_pid="$WH_MANAGED_PID"
reused_identity_pgid="$WH_MANAGED_PGID"
reused_identity="${WH_MANAGED_IDENTITY:-}"
set +e
wh_terminate_process_group \
  "$reused_identity_pid" \
  "$reused_identity_pgid" \
  100 \
  "unrelated-reused-process-identity"
reused_identity_status=$?
set -e
if [[ "$reused_identity_status" -eq 0 ]] || ! kill -0 "$reused_identity_pid" >/dev/null 2>&1; then
  echo "process-group identity contract: mismatched leader identity was allowed to terminate a live group" >&2
  exit 1
fi
wh_terminate_process_group "$reused_identity_pid" "$reused_identity_pgid" 100 "$reused_identity"

PORT_ROOT="$TMP_DIR/port-reservations"
PORT_COMMON_DIR="$TMP_DIR/port-registry"
mkdir -p "$PORT_ROOT"
PORT_ONE_JSON="$TMP_DIR/ports-one.json"
PORT_TWO_JSON="$TMP_DIR/ports-two.json"
set +e
wh_resolve_ports_json "$PORT_ROOT" "$$" "$(wh_worktree_path)" "$PORT_COMMON_DIR" >"$PORT_ONE_JSON" 2>"$TMP_DIR/ports-one.err" &
port_one_pid=$!
wh_resolve_ports_json "$PORT_ROOT" "$$" "$(wh_worktree_path)" "$PORT_COMMON_DIR" >"$PORT_TWO_JSON" 2>"$TMP_DIR/ports-two.err" &
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
for ports_json in "$PORT_ONE_JSON" "$PORT_TWO_JSON"; do
  if [[ -s "$ports_json" ]]; then
    reservation_token="$(python3 - "$ports_json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text())['reservation_token'])
PY
)"
    wh_release_ports_reservation "$PORT_ROOT" "$reservation_token" "$PORT_COMMON_DIR"
  fi
done

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
wh_resolve_ports_json "$STALE_PORT_ROOT" "$$" "$(wh_worktree_path)" "$PORT_COMMON_DIR" >"$stale_ports_json"
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
wh_release_ports_reservation "$STALE_PORT_ROOT" "$stale_token" "$PORT_COMMON_DIR"
[[ ! -e "$STALE_PORT_ROOT/.ports.reservation.json" ]] || {
  echo "port reservation contract: release left reservation behind" >&2
  exit 1
}

collision_paths="$(python3 - <<'PY'
import hashlib
import pathlib

base = pathlib.Path("/tmp/oasis7-port-collision").resolve()
seen = {}
for index in range(500_000):
    candidate = base / f"worktree-{index}"
    digest = hashlib.sha256(str(candidate).encode("utf-8")).hexdigest()[:8]
    if digest in seen:
        print(seen[digest])
        print(candidate)
        break
    seen[digest] = candidate
else:
    raise SystemExit("unable to find deterministic worktree hash collision")
PY
)"
collision_one="$(printf '%s\n' "$collision_paths" | sed -n '1p')"
collision_two="$(printf '%s\n' "$collision_paths" | sed -n '2p')"
COLLISION_COMMON_DIR="$TMP_DIR/shared-common-dir"
mkdir -p "$COLLISION_COMMON_DIR"
collision_root_one="$TMP_DIR/collision-root-one"
collision_root_two="$TMP_DIR/collision-root-two"
same_root_one="$TMP_DIR/same-worktree-root-one"
same_root_two="$TMP_DIR/same-worktree-root-two"
collision_ports_one="$TMP_DIR/collision-ports-one.json"
collision_ports_two="$TMP_DIR/collision-ports-two.json"
same_ports_one="$TMP_DIR/same-ports-one.json"
same_ports_two="$TMP_DIR/same-ports-two.json"
wh_resolve_ports_json "$collision_root_one" "$$" "$collision_one" "$COLLISION_COMMON_DIR" >"$collision_ports_one" 2>"$TMP_DIR/collision-one.err" &
collision_pid_one=$!
wh_resolve_ports_json "$collision_root_two" "$$" "$collision_two" "$COLLISION_COMMON_DIR" >"$collision_ports_two" 2>"$TMP_DIR/collision-two.err" &
collision_pid_two=$!
set +e
wait "$collision_pid_one"
collision_status_one=$?
wait "$collision_pid_two"
collision_status_two=$?
set -e
[[ "$collision_status_one" -eq 0 && "$collision_status_two" -eq 0 ]] || {
  cat "$TMP_DIR/collision-one.err" "$TMP_DIR/collision-two.err" >&2
  exit 1
}
wh_resolve_ports_json "$same_root_one" "$$" "$collision_one" "$COLLISION_COMMON_DIR" >"$same_ports_one" 2>"$TMP_DIR/same-one.err" &
same_pid_one=$!
wh_resolve_ports_json "$same_root_two" "$$" "$collision_one" "$COLLISION_COMMON_DIR" >"$same_ports_two" 2>"$TMP_DIR/same-two.err" &
same_pid_two=$!
set +e
wait "$same_pid_one"
same_status_one=$?
wait "$same_pid_two"
same_status_two=$?
set -e
[[ "$same_status_one" -eq 0 && "$same_status_two" -eq 0 ]] || {
  cat "$TMP_DIR/same-one.err" "$TMP_DIR/same-two.err" >&2
  exit 1
}
port_collision_failures="$(python3 - "$collision_ports_one" "$collision_ports_two" "$same_ports_one" "$same_ports_two" <<'PY'
import json
import pathlib
import sys

records = [json.loads(pathlib.Path(path).read_text()) for path in sys.argv[1:]]
failures = []
if records[0]["viewer_port"] == records[1]["viewer_port"]:
    failures.append("distinct colliding worktree paths received the same port set")
if records[2]["viewer_port"] == records[3]["viewer_port"]:
    failures.append("same worktree path across distinct roots received the same port set")
if failures:
    print("\n".join(failures), file=sys.stderr)
print(len(failures))
PY
)"
[[ "$port_collision_failures" == "0" ]] || exit 1

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
