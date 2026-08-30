#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

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

cleanup() {
  set +e
  if [[ -n "$SENTINEL_PID" ]]; then
    kill "$SENTINEL_PID" >/dev/null 2>&1 || true
    wait "$SENTINEL_PID" >/dev/null 2>&1 || true
  fi
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

echo "worktree harness lifecycle acceptance: PASS"
