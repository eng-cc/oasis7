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
TEST_ROOT="$(mktemp -d "$TMP_DIR/harness-test.XXXXXX")"
TEST_ROOT_MARKER="$TEST_ROOT/.oasis7-harness-test-root"
printf 'oasis7-harness-lifecycle-test-v1\n' >"$TEST_ROOT_MARKER"
WORKTREE_ID="$(python3 - "$PWD" <<'PY'
import hashlib
import pathlib
import sys

print(f"wt-{hashlib.sha256(str(pathlib.Path(sys.argv[1]).resolve()).encode()).hexdigest()[:8]}")
PY
)"
HARNESS_ROOT="$TEST_ROOT/harness"
PRODUCTION_HARNESS_ROOT="$ROOT_DIR/output/harness/$WORKTREE_ID"
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
LEGACY_DOWN_PID=""
LEGACY_DOWN_PGID=""
LEGACY_DOWN_IDENTITY=""
LEGACY_UP_PID=""
LEGACY_UP_PGID=""
LEGACY_UP_IDENTITY=""
REAL_STACK_PID=""
REAL_STACK_READER_PID=""
CONCURRENT_PARENT_A=""
CONCURRENT_PARENT_B=""
LIFECYCLE_STEP="initialization"

run_harness() {
  OASIS7_HARNESS_TEST_ROOT="$HARNESS_ROOT" ./scripts/worktree-harness.sh "$@"
}

run_fake_harness() {
  OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" run_harness "$@"
}

run_fake_harness_at() {
  local root=$1
  shift
  OASIS7_HARNESS_TEST_ROOT="$root" \
    OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
    "$ROOT_DIR/scripts/worktree-harness.sh" "$@"
}

test_root_owned() {
  [[ "$TEST_ROOT" == "$TMP_DIR/harness-test."* ]] || return 1
  [[ "$(dirname "$TEST_ROOT")" == "$TMP_DIR" ]] || return 1
  [[ -f "$TEST_ROOT_MARKER" ]] || return 1
  [[ "$(tr -d '\n' <"$TEST_ROOT_MARKER")" == "oasis7-harness-lifecycle-test-v1" ]]
}

reset_owned_harness_root() {
  test_root_owned || {
    echo "lifecycle acceptance: refusing to reset unowned test root: $TEST_ROOT" >&2
    return 1
  }
  rm -rf -- "$HARNESS_ROOT"
}

cleanup_owned_test_root() {
  test_root_owned || {
    echo "lifecycle acceptance: refusing to remove unowned test root: $TEST_ROOT" >&2
    return 1
  }
  rm -rf -- "$TEST_ROOT"
}

lifecycle_step() {
  LIFECYCLE_STEP="$1"
  echo "lifecycle acceptance: step=${LIFECYCLE_STEP:-unknown}" >&2
}

lifecycle_process_snapshot() {
  local label=${1:-}
  local pid=${2:-}
  local pgid=${3:-}
  local state actual_pgid pid_live group_live
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 0
  state=$(ps -o stat= -p "$pid" 2>/dev/null | awk 'NF { print $1; exit }' || true)
  actual_pgid=$(ps -o pgid= -p "$pid" 2>/dev/null | awk 'NF { print $1; exit }' || true)
  pid_live=0
  group_live=0
  if wh_pid_alive "$pid"; then
    pid_live=1
  fi
  if [[ "$pgid" =~ ^[1-9][0-9]*$ ]] && wh_process_group_alive "$pgid"; then
    group_live=1
  fi
  printf 'lifecycle acceptance: process=%s pid=%s state=%s actual_pgid=%s recorded_pgid=%s helper_pid_live=%s helper_group_live=%s\n' "$label" "$pid" "${state:-unknown}" "${actual_pgid:-unknown}" "${pgid:-unknown}" "$pid_live" "$group_live" >&2
}

lifecycle_error_trap() {
  local rc=$?
  case "$-" in
    *e*) ;;
    *) return "$rc" ;;
  esac
  echo "lifecycle acceptance: ERR step=${LIFECYCLE_STEP:-unknown} source=${BASH_SOURCE[1]:-$0} line=${BASH_LINENO[0]:-unknown} rc=$rc command=${BASH_COMMAND:-unknown}" >&2
  lifecycle_process_snapshot "legacy-down" "${LEGACY_DOWN_PID:-}" "${LEGACY_DOWN_PGID:-}"
  lifecycle_process_snapshot "legacy-up" "${LEGACY_UP_PID:-}" "${LEGACY_UP_PGID:-}"
  lifecycle_process_snapshot "unrelated" "${UNRELATED_PID:-}" "${UNRELATED_PGID:-}"
  lifecycle_process_snapshot "sentinel" "${SENTINEL_PID:-}" ""
  lifecycle_process_snapshot "ready-child" "${ready_child_pid:-}" ""
  lifecycle_process_snapshot "ready-launcher" "${ready_launcher_pid:-}" "${ready_launcher_pgid:-}"
  lifecycle_process_snapshot "ready-harness" "${READY_HARNESS_PID:-}" "${READY_HARNESS_PGID:-}"
  lifecycle_process_snapshot "readiness-child" "${readiness_child_pid:-}" ""
  lifecycle_process_snapshot "readiness-harness" "${READINESS_HARNESS_PID:-}" "${READINESS_HARNESS_PGID:-}"
  lifecycle_process_snapshot "handoff-child" "${handoff_child_pid:-}" ""
  lifecycle_process_snapshot "concurrent-child" "${concurrent_child_pid:-}" ""
  lifecycle_process_snapshot "timeout-child" "${timeout_child_pid:-}" ""
  lifecycle_process_snapshot "failure" "${failure_pid:-}" "${failure_pgid:-}"
  return "$rc"
}

trap lifecycle_error_trap ERR

cleanup_recorded_group() {
  local pid=${1:-}
  local pgid=${2:-}
  local identity=${3:-}
  [[ -n "$pid" ]] || return 0
  wh_terminate_process_group "$pid" "$pgid" 500 "$identity" >/dev/null 2>&1 || true
}

cleanup() {
  set +e
  if [[ -n "$REAL_STACK_PID" ]]; then
    kill "$REAL_STACK_PID" >/dev/null 2>&1 || true
    wait "$REAL_STACK_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$REAL_STACK_READER_PID" ]]; then
    kill "$REAL_STACK_READER_PID" >/dev/null 2>&1 || true
    wait "$REAL_STACK_READER_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$SENTINEL_PID" ]]; then
    kill "$SENTINEL_PID" >/dev/null 2>&1 || true
    wait "$SENTINEL_PID" >/dev/null 2>&1 || true
  fi
  run_fake_harness down >/dev/null 2>&1 || true
  cleanup_recorded_group "$LEGACY_DOWN_PID" "$LEGACY_DOWN_PGID" "$LEGACY_DOWN_IDENTITY"
  cleanup_recorded_group "$LEGACY_UP_PID" "$LEGACY_UP_PGID" "$LEGACY_UP_IDENTITY"
  cleanup_recorded_group "$UNRELATED_PID" "$UNRELATED_PGID" "$UNRELATED_IDENTITY"
  cleanup_recorded_group "$READINESS_HARNESS_PID" "$READINESS_HARNESS_PGID" "$READINESS_HARNESS_IDENTITY"
  cleanup_recorded_group "$READY_HARNESS_PID" "$READY_HARNESS_PGID" "$READY_HARNESS_IDENTITY"
  run_fake_harness down >/dev/null 2>&1 || true
  if [[ -n "$CONCURRENT_PARENT_A" ]]; then
    OASIS7_HARNESS_TEST_ROOT="$CONCURRENT_PARENT_A/harness" run_fake_harness_at down >/dev/null 2>&1 || true
  fi
  if [[ -n "$CONCURRENT_PARENT_B" ]]; then
    OASIS7_HARNESS_TEST_ROOT="$CONCURRENT_PARENT_B/harness" run_fake_harness_at down >/dev/null 2>&1 || true
  fi
  cleanup_owned_test_root
  if [[ -n "$CONCURRENT_PARENT_A" && -f "$CONCURRENT_PARENT_A/.oasis7-harness-test-root" ]]; then
    rm -rf -- "$CONCURRENT_PARENT_A"
  fi
  if [[ -n "$CONCURRENT_PARENT_B" && -f "$CONCURRENT_PARENT_B/.oasis7-harness-test-root" ]]; then
    rm -rf -- "$CONCURRENT_PARENT_B"
  fi
  rm -rf -- "$TMP_DIR"
}
trap cleanup EXIT

reset_owned_harness_root

# A production harness root must never be selected by the test-only override,
# and an unmarked path must not be accepted as a deletion target.
NON_TEST_ROOT="$TMP_DIR/non-test-root"
mkdir -p "$NON_TEST_ROOT"
printf 'must-survive\n' >"$NON_TEST_ROOT/sentinel"
set +e
OASIS7_HARNESS_TEST_ROOT="$NON_TEST_ROOT" ./scripts/worktree-harness.sh status --json >"$TMP_DIR/non-test-root.log" 2>&1
non_test_root_status=$?
set -e
[[ "$non_test_root_status" -ne 0 ]] || {
  echo "lifecycle acceptance: unowned test root was accepted" >&2
  exit 1
}
[[ "$(cat "$NON_TEST_ROOT/sentinel")" == "must-survive" ]] || {
  echo "lifecycle acceptance: unowned test root sentinel changed" >&2
  exit 1
}
[[ "$HARNESS_ROOT" != "$PRODUCTION_HARNESS_ROOT" ]] || {
  echo "lifecycle acceptance: test root points at production harness root" >&2
  exit 1
}
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
chmod +x "$FAKE_LAUNCHER"

lifecycle_step "concurrent isolated test roots"
CONCURRENT_PARENT_A="$(mktemp -d "$TMP_DIR/concurrent-test-a.XXXXXX")"
CONCURRENT_PARENT_B="$(mktemp -d "$TMP_DIR/concurrent-test-b.XXXXXX")"
printf 'oasis7-harness-lifecycle-test-v1\n' >"$CONCURRENT_PARENT_A/.oasis7-harness-test-root"
printf 'oasis7-harness-lifecycle-test-v1\n' >"$CONCURRENT_PARENT_B/.oasis7-harness-test-root"
CONCURRENT_ROOT_A="$CONCURRENT_PARENT_A/harness"
CONCURRENT_ROOT_B="$CONCURRENT_PARENT_B/harness"
CONCURRENT_CHILD_A="$TMP_DIR/concurrent-a-child.pid"
CONCURRENT_CHILD_B="$TMP_DIR/concurrent-b-child.pid"
FAKE_LAUNCHER_CHILD_PID_FILE="$CONCURRENT_CHILD_A" \
  run_fake_harness_at "$CONCURRENT_ROOT_A" up --startup-timeout 5 >"$TMP_DIR/concurrent-a-up.log" 2>&1 &
concurrent_a_pid=$!
FAKE_LAUNCHER_CHILD_PID_FILE="$CONCURRENT_CHILD_B" \
  run_fake_harness_at "$CONCURRENT_ROOT_B" up --startup-timeout 5 >"$TMP_DIR/concurrent-b-up.log" 2>&1 &
concurrent_b_pid=$!
set +e
wait "$concurrent_a_pid"
concurrent_a_status=$?
wait "$concurrent_b_pid"
concurrent_b_status=$?
set -e
if [[ "$concurrent_a_status" -ne 0 || "$concurrent_b_status" -ne 0 ]]; then
  cat "$TMP_DIR/concurrent-a-up.log" "$TMP_DIR/concurrent-b-up.log" >&2
  exit 1
fi
concurrent_a_port="$(python3 - "$CONCURRENT_ROOT_A/state.json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["viewer_port"])
PY
)"
concurrent_b_port="$(python3 - "$CONCURRENT_ROOT_B/state.json" <<'PY'
import json
import pathlib
import sys

print(json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["viewer_port"])
PY
)"
[[ "$concurrent_a_port" != "$concurrent_b_port" ]] || {
  echo "lifecycle acceptance: isolated concurrent harness roots shared a viewer port" >&2
  exit 1
}
[[ -f "$CONCURRENT_ROOT_A/.ports.reservation.json" && -f "$CONCURRENT_ROOT_B/.ports.reservation.json" ]] || {
  echo "lifecycle acceptance: isolated concurrent harness root lost its reservation" >&2
  exit 1
}
FAKE_LAUNCHER_CHILD_PID_FILE="$CONCURRENT_CHILD_A" \
  run_fake_harness_at "$CONCURRENT_ROOT_A" down >"$TMP_DIR/concurrent-a-down.log" 2>&1 &
concurrent_a_down_pid=$!
FAKE_LAUNCHER_CHILD_PID_FILE="$CONCURRENT_CHILD_B" \
  run_fake_harness_at "$CONCURRENT_ROOT_B" down >"$TMP_DIR/concurrent-b-down.log" 2>&1 &
concurrent_b_down_pid=$!
set +e
wait "$concurrent_a_down_pid"
concurrent_a_down_status=$?
wait "$concurrent_b_down_pid"
concurrent_b_down_status=$?
set -e
if [[ "$concurrent_a_down_status" -ne 0 || "$concurrent_b_down_status" -ne 0 ]]; then
  cat "$TMP_DIR/concurrent-a-down.log" "$TMP_DIR/concurrent-b-down.log" >&2
  exit 1
fi
python3 - "$CONCURRENT_ROOT_A/state.json" "$CONCURRENT_ROOT_B/state.json" <<'PY'
import json
import pathlib
import sys

for raw_path in sys.argv[1:]:
    state = json.loads(pathlib.Path(raw_path).read_text(encoding="utf-8"))
    assert state["status"] == "stopped", state
    assert state["phase"] == "stopped", state
PY

lifecycle_step "real session metadata writer"
# Exercise the production run-launcher-stack metadata writer with a
# deterministic local bundle.  The fixture owns only two loopback listeners,
# does not open a browser, and does not contact a provider; the stack still
# publishes session.meta and its --json-ready payload through production code.
REAL_STACK_BUNDLE="$TEST_ROOT/real-stack-bundle"
REAL_STACK_OUTPUT="$TEST_ROOT/real-stack-output"
REAL_STACK_META="$REAL_STACK_OUTPUT/session.meta"
REAL_STACK_JSON="$TMP_DIR/real-stack-ready.jsonl"
REAL_STACK_LOG="$TMP_DIR/real-stack.log"
REAL_STACK_READER_ERROR="$TMP_DIR/real-stack-reader.error"
REAL_STACK_META_ZERO="$TMP_DIR/real-stack-meta-zero"
REAL_STACK_META_READY="$TMP_DIR/real-stack-meta-ready"
REAL_STACK_JSON_READY="$TMP_DIR/real-stack-json-ready"
REAL_STACK_READER_STOP="$TMP_DIR/real-stack-reader.stop"
mkdir -p "$REAL_STACK_BUNDLE" "$REAL_STACK_OUTPUT"
cat >"$REAL_STACK_BUNDLE/run-game.sh" <<'REAL_RUN_GAME'
#!/usr/bin/env python3
import http.server
import socketserver
import sys
import threading
import time


def option_value(name: str, default: str) -> str:
    for index, value in enumerate(sys.argv[1:]):
        if value == name and index + 2 <= len(sys.argv[1:]):
            return sys.argv[index + 2]
    return default


viewer_port = int(option_value("--viewer-port", "4173"))
web_bind = option_value("--web-bind", "127.0.0.1:5011")
web_port = int(web_bind.rsplit(":", 1)[-1])
ready_at = time.monotonic() + 0.4


class ViewerHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if time.monotonic() < ready_at:
            self.send_error(503, "fixture warming")
            return
        body = b"<!doctype html><title>harness fixture</title>"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


class BridgeHandler(socketserver.BaseRequestHandler):
    def handle(self):
        self.request.close()


viewer = http.server.ThreadingHTTPServer(("127.0.0.1", viewer_port), ViewerHandler)
bridge = socketserver.ThreadingTCPServer(("127.0.0.1", web_port), BridgeHandler)
threading.Thread(target=viewer.serve_forever, daemon=True).start()
threading.Thread(target=bridge.serve_forever, daemon=True).start()
while True:
    time.sleep(1)
REAL_RUN_GAME
chmod +x "$REAL_STACK_BUNDLE/run-game.sh"

read -r REAL_STACK_VIEWER_PORT REAL_STACK_WEB_PORT REAL_STACK_LIVE_PORT < <(python3 - <<'PY'
import socket

sockets = []
try:
    for _ in range(3):
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("127.0.0.1", 0))
        sockets.append(sock)
    print(" ".join(str(sock.getsockname()[1]) for sock in sockets))
finally:
    for sock in sockets:
        sock.close()
PY
)
REAL_STACK_RUN_ARGS=(
  --bundle-dir "$REAL_STACK_BUNDLE"
  --allow-stale-bundle
  --skip-llm-provider-preflight
  --chain-disable
  --viewer-static-dir "$ROOT_DIR/web"
  --viewer-port "$REAL_STACK_VIEWER_PORT"
  --web-bind "127.0.0.1:$REAL_STACK_WEB_PORT"
  --live-bind "127.0.0.1:$REAL_STACK_LIVE_PORT"
  --output-dir "$REAL_STACK_OUTPUT"
  --run-id real-writer-fixture
  --meta-file "$REAL_STACK_META"
  --json-ready
  --with-llm
)
python3 - "$REAL_STACK_META" "$REAL_STACK_META_ZERO" "$REAL_STACK_META_READY" "$REAL_STACK_JSON" "$REAL_STACK_JSON_READY" "$REAL_STACK_READER_ERROR" "$REAL_STACK_READER_STOP" <<'PY' >"$TMP_DIR/real-stack-reader.log" 2>&1 &
import json
import pathlib
import sys
import time

meta_path, meta_zero, meta_ready, json_path, json_ready, error_path, stop_path = map(pathlib.Path, sys.argv[1:])
while True:
    if stop_path.exists():
        break
    try:
        if meta_path.exists():
            raw = meta_path.read_text(encoding="utf-8")
            if not raw.endswith("\n"):
                raise AssertionError("session.meta was observed without a final newline")
            values = {}
            for line in raw.splitlines():
                if "=" in line:
                    key, value = line.split("=", 1)
                    values[key] = value
            required = {
                "RUN_ID", "OUTPUT_DIR", "LAUNCHER_PID", "LAUNCHER_PGID",
                "LAUNCHER_IDENTITY", "VIEWER_PORT", "STACK_READY",
            }
            missing = required - values.keys()
            if missing:
                raise AssertionError(f"session.meta missing keys: {sorted(missing)}")
            if values["STACK_READY"] == "0":
                meta_zero.touch()
            if values["STACK_READY"] == "1":
                if not values["GAME_URL"].startswith("http://127.0.0.1:"):
                    raise AssertionError(values)
                meta_ready.touch()
        if json_path.exists():
            for line in json_path.read_text(encoding="utf-8").splitlines():
                if not line.lstrip().startswith("{"):
                    continue
                payload = json.loads(line)
                required_json = {
                    "run_id", "output_dir", "launcher_pid", "launcher_pgid",
                    "live_bind_addr", "web_bridge_addr", "viewer_host", "viewer_port",
                    "chain_enabled", "chain_node_id", "chain_status_bind_addr",
                    "launch_mode", "launch_cmd", "bundle_dir", "game_url",
                    "viewer_url_zh", "viewer_url_en",
                    "software_safe_compat_viewer_url_zh",
                    "software_safe_compat_viewer_url_en", "meta_file",
                }
                assert required_json <= payload.keys(), payload
                assert isinstance(payload["launcher_pid"], int), payload
                assert isinstance(payload["launcher_pgid"], int), payload
                assert payload["viewer_port"] > 0, payload
                assert payload["meta_file"] == str(meta_path), payload
                assert payload["output_dir"] == str(meta_path.parent), payload
                assert payload["chain_enabled"] is False, payload
                assert payload["launch_mode"] == "bundle", payload
                json_ready.touch()
        if meta_ready.exists() and json_ready.exists():
            break
    except Exception as exc:
        error_path.write_text(str(exc), encoding="utf-8")
        break
    time.sleep(0.01)
PY
REAL_STACK_READER_PID=$!
./scripts/run-launcher-stack.sh "${REAL_STACK_RUN_ARGS[@]}" >"$REAL_STACK_JSON" 2>"$REAL_STACK_LOG" &
REAL_STACK_PID=$!
for _ in $(seq 1 120); do
  [[ -e "$REAL_STACK_META_READY" && -e "$REAL_STACK_JSON_READY" ]] && break
  [[ -e "$REAL_STACK_READER_ERROR" ]] && break
  sleep 0.05
done
: >"$REAL_STACK_READER_STOP"
wait "$REAL_STACK_READER_PID" || true
[[ ! -e "$REAL_STACK_READER_ERROR" ]] || {
  cat "$REAL_STACK_READER_ERROR" >&2
  exit 1
}
[[ -e "$REAL_STACK_META_ZERO" && -e "$REAL_STACK_META_READY" && -e "$REAL_STACK_JSON_READY" ]] || {
  echo "lifecycle acceptance: real session metadata writer did not expose complete transition" >&2
  cat "$REAL_STACK_LOG" >&2 || true
  exit 1
}
python3 - "$REAL_STACK_META" "$REAL_STACK_JSON" <<'PY'
import json
import pathlib
import sys

meta_path = pathlib.Path(sys.argv[1])
json_path = pathlib.Path(sys.argv[2])
values = {}
for line in meta_path.read_text(encoding="utf-8").splitlines():
    if "=" in line:
        key, value = line.split("=", 1)
        values[key] = value
assert values["STACK_READY"] == "1", values
assert values["LAUNCHER_PID"].isdigit(), values
assert values["LAUNCHER_PGID"].isdigit(), values
assert values["LAUNCHER_IDENTITY"], values
payloads = [
    json.loads(line)
    for line in json_path.read_text(encoding="utf-8").splitlines()
    if line.lstrip().startswith("{")
]
assert len(payloads) == 1, payloads
payload = payloads[0]
required_json = {
    "run_id", "output_dir", "launcher_pid", "launcher_pgid", "live_bind_addr",
    "web_bridge_addr", "viewer_host", "viewer_port", "chain_enabled",
    "chain_node_id", "chain_status_bind_addr", "launch_mode", "launch_cmd",
    "bundle_dir", "game_url", "viewer_url_zh", "viewer_url_en",
    "software_safe_compat_viewer_url_zh", "software_safe_compat_viewer_url_en",
    "meta_file",
}
assert required_json <= payload.keys(), payload
assert payload["launcher_pid"] == int(values["LAUNCHER_PID"]), payload
assert payload["launcher_pgid"] == int(values["LAUNCHER_PGID"]), payload
assert payload["viewer_port"] == int(values["VIEWER_PORT"]), payload
assert payload["meta_file"] == str(meta_path), payload
PY
REAL_STACK_LAUNCHER_PID="$(wh_env_file_get "$REAL_STACK_META" LAUNCHER_PID)"
REAL_STACK_LAUNCHER_PGID="$(wh_env_file_get "$REAL_STACK_META" LAUNCHER_PGID)"
REAL_STACK_LAUNCHER_IDENTITY="$(wh_env_file_get "$REAL_STACK_META" LAUNCHER_IDENTITY)"
[[ "$(wh_process_identity "$REAL_STACK_LAUNCHER_PID")" == "$REAL_STACK_LAUNCHER_IDENTITY" ]] || {
  echo "lifecycle acceptance: real session metadata identity does not match launcher" >&2
  exit 1
}
kill "$REAL_STACK_PID" >/dev/null 2>&1 || true
wait "$REAL_STACK_PID" >/dev/null 2>&1 || true
REAL_STACK_PID=""

# A pre-upgrade record may contain only PIDs.  A live PID without a captured
# identity is not safe to signal because it may have been reused by a foreign
# process; down/up must retain the record and reservation for operator-owned
# recovery instead of guessing.
lifecycle_step "legacy identity-less down"
LEGACY_PID_ONLY_TOKEN="legacy-pid-only-token"
wh_start_managed sleep 300 >"$TMP_DIR/legacy-pid-only-down-group.log" 2>&1
LEGACY_DOWN_PID="$WH_MANAGED_PID"
LEGACY_DOWN_PGID="$WH_MANAGED_PGID"
LEGACY_DOWN_IDENTITY="$WH_MANAGED_IDENTITY"
wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - "$LEGACY_DOWN_PID" "$LEGACY_PID_ONLY_TOKEN" <<'PY'
import json
import sys

print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": int(sys.argv[1]),
    "launcher_pid": int(sys.argv[1]),
    "port_reservation_token": sys.argv[2],
}))
PY
)"
set +e
run_harness status --json >"$TMP_DIR/legacy-pid-only-status.log" 2>&1
legacy_status_rc=$?
set -e
[[ "$legacy_status_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: status accepted a live identity-less legacy record" >&2
  exit 1
}
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["phase"] == "cleanup_pending", state
assert "identity" in state["failure_reason"], state
PY
set +e
run_harness down >"$TMP_DIR/legacy-pid-only-down.log" 2>&1
legacy_down_status="$?"
set -e
[[ "$legacy_down_status" -ne 0 ]] || {
  echo "lifecycle acceptance: down accepted live identity-less legacy record" >&2
  exit 1
}
python3 - "$HARNESS_ROOT/state.json" "$LEGACY_PID_ONLY_TOKEN" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "failed", state
assert state["phase"] == "cleanup_pending", state
assert "identity" in state["failure_reason"], state
assert state["harness_pid"], state
assert state["launcher_pid"], state
assert state["port_reservation_token"] == sys.argv[2], state
PY
if ! wh_pid_alive "$LEGACY_DOWN_PID"; then
  echo "lifecycle acceptance: down signalled a live identity-less legacy process" >&2
  exit 1
fi
wh_terminate_process_group "$LEGACY_DOWN_PID" "$LEGACY_DOWN_PGID" 100 "$LEGACY_DOWN_IDENTITY"

lifecycle_step "legacy identity-less up"
wh_start_managed sleep 300 >"$TMP_DIR/legacy-pid-only-up-group.log" 2>&1
LEGACY_UP_PID="$WH_MANAGED_PID"
LEGACY_UP_PGID="$WH_MANAGED_PGID"
LEGACY_UP_IDENTITY="$WH_MANAGED_IDENTITY"
wh_state_write "$HARNESS_ROOT/state.json" "$(python3 - "$LEGACY_UP_PID" "$LEGACY_PID_ONLY_TOKEN" <<'PY'
import json
import sys

print(json.dumps({
    "status": "ready",
    "phase": "ready",
    "harness_pid": int(sys.argv[1]),
    "launcher_pid": int(sys.argv[1]),
    "port_reservation_token": sys.argv[2],
}))
PY
)"
set +e
run_fake_harness up --with-llm --startup-timeout 1 >"$TMP_DIR/legacy-pid-only-up.log" 2>&1
legacy_up_status="$?"
set -e
[[ "$legacy_up_status" -ne 0 ]] || {
  echo "lifecycle acceptance: up accepted live identity-less legacy record" >&2
  exit 1
}
python3 - "$HARNESS_ROOT/state.json" "$LEGACY_PID_ONLY_TOKEN" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "failed", state
assert state["phase"] == "cleanup_pending", state
assert "identity" in state["failure_reason"], state
assert state["harness_pid"], state
assert state["launcher_pid"], state
assert state["port_reservation_token"] == sys.argv[2], state
PY
if ! wh_pid_alive "$LEGACY_UP_PID"; then
  echo "lifecycle acceptance: up signalled a live identity-less legacy process" >&2
  exit 1
fi
wh_terminate_process_group "$LEGACY_UP_PID" "$LEGACY_UP_PGID" 100 "$LEGACY_UP_IDENTITY"

# Dead identity-less records are safe to tombstone and should not leave a
# cleanup-pending state behind.
lifecycle_step "dead legacy record cleanup"
wh_state_write "$HARNESS_ROOT/state.json" '{"status":"ready","phase":"ready","harness_pid":999999999,"launcher_pid":999999999}'
run_fake_harness down >"$TMP_DIR/legacy-dead-down.log" 2>&1
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "stopped", state
assert state["phase"] == "stopped", state
assert state["harness_pid"] is None, state
assert state["launcher_pid"] is None, state
PY

SENTINEL_PID=""
lifecycle_step "ready launch and state validation"
sleep 300 &
SENTINEL_PID=$!
FAKE_LAUNCHER_CHILD_PID_FILE="$READY_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
run_fake_harness up --startup-timeout 5 >"$TMP_DIR/ready-up.log" 2>&1

python3 - "$HARNESS_ROOT/state.json" "$HARNESS_ROOT/.ports.reservation.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
reservation = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
assert state["status"] == "ready", state
assert state["launcher_pgid"] == state["harness_pgid"], state
assert state["launcher_pid"] != state["harness_pid"], state
assert state["port_reservation_token"], state
assert state["harness_identity"], state
assert state["launcher_identity"], state
assert reservation["owner_pid"] == state["harness_pid"], reservation
assert reservation["owner_identity"] == state["harness_identity"], reservation
PY

status_json="$TMP_DIR/status.json"
run_fake_harness status --json >"$status_json"
viewer_url="$(run_fake_harness url)"
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

lifecycle_step "stale identity status rejection"
set_stale_live_record
set +e
run_fake_harness status --json >"$TMP_DIR/stale-status.log" 2>&1
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
lifecycle_step "missing and dead launcher status rejection"
set_launcher_record
set +e
run_fake_harness status --json >"$TMP_DIR/missing-launcher-status.log" 2>&1
missing_launcher_status_rc=$?
set -e
[[ "$missing_launcher_status_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: status retained ready with missing launcher record" >&2
  exit 1
}
restore_ready_record

lifecycle_step "stale launcher identity status rejection"
set_launcher_record "999999999" "999999999" "dead-launcher-incarnation"
set +e
run_fake_harness status --json >"$TMP_DIR/dead-launcher-status.log" 2>&1
dead_launcher_status_rc=$?
set -e
[[ "$dead_launcher_status_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: status retained ready with dead launcher record" >&2
  exit 1
}
restore_ready_record

lifecycle_step "stale launcher record up rejection"
set_launcher_record "$UNRELATED_PID" "$UNRELATED_PGID" "stale-unrelated-launcher-incarnation"
set +e
FAKE_LAUNCHER_CHILD_PID_FILE="$READY_CHILD_PID_FILE" \
run_fake_harness up --startup-timeout 5 >"$TMP_DIR/stale-launcher-up.log" 2>&1
stale_launcher_up_rc=$?
set -e
[[ "$stale_launcher_up_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: up accepted live harness with stale launcher identity" >&2
  exit 1
}
restore_ready_record

lifecycle_step "stale identity URL/up rejection"
set_stale_live_record
set +e
run_fake_harness url >"$TMP_DIR/stale-url.log" 2>&1
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
run_fake_harness up --startup-timeout 5 >"$TMP_DIR/stale-up.log" 2>&1
stale_up_rc=$?
set -e
[[ "$stale_up_rc" -ne 0 ]] || {
  echo "lifecycle acceptance: up accepted unrelated live PID with stale identity" >&2
  exit 1
}
restore_ready_record

lifecycle_step "ready down and process-tree cleanup"
run_fake_harness down
python3 - "$HARNESS_ROOT/state.json" <<'PY'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert state["status"] == "stopped", state
assert state["phase"] == "stopped", state
PY

for _ in $(seq 1 40); do
  if ! wh_pid_alive "$ready_child_pid" && ! wh_pid_alive "$ready_launcher_pid"; then
    break
  fi
  sleep 0.05
done
if wh_pid_alive "$ready_child_pid" || wh_pid_alive "$ready_launcher_pid"; then
  echo "lifecycle acceptance: ready launcher process tree survived down" >&2
  exit 1
fi
if ! wh_pid_alive "$SENTINEL_PID"; then
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
lifecycle_step "stale readiness identity rejection"
# Launch synchronization begins before the harness establishes its own
# startup deadline. Allow one bounded startup budget for setup and one for
# the post-launch handoff without changing the harness deadline itself.
LAUNCH_SYNC_TIMEOUT_SECS=$((STARTUP_TIMEOUT_SECS * 4))
READINESS_DELAY_FILE="$TMP_DIR/readiness-delay.marker"
READINESS_ACK_FILE="$TMP_DIR/readiness-delay.ack"
READINESS_ACKED_FILE="$TMP_DIR/readiness-delay.acked"
READINESS_CHILD_PID_FILE="$TMP_DIR/readiness-child.pid"
rm -f "$READINESS_DELAY_FILE" "$READINESS_ACK_FILE" "$READINESS_ACKED_FILE" "$READINESS_CHILD_PID_FILE"
FAKE_LAUNCHER_CHILD_PID_FILE="$READINESS_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE="$READINESS_DELAY_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACK_FILE="$READINESS_ACK_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACKED_FILE="$READINESS_ACKED_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS=2 \
run_fake_harness up --startup-timeout "$STARTUP_TIMEOUT_SECS" >"$TMP_DIR/readiness-up.log" 2>&1 &
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
run_fake_harness down >/dev/null 2>&1 || {
  echo "lifecycle acceptance: readiness fixture cleanup failed" >&2
  exit 1
}
if [[ -e "$READINESS_CHILD_PID_FILE" ]]; then
  readiness_child_pid="$(cat "$READINESS_CHILD_PID_FILE")"
  for _ in $(seq 1 40); do
    wh_pid_alive "$readiness_child_pid" || break
    sleep 0.05
  done
  if wh_pid_alive "$readiness_child_pid"; then
    echo "lifecycle acceptance: readiness launcher child survived cleanup" >&2
    exit 1
  fi
fi
echo "unrelated live PID identity rejection: status_rc=$stale_status_rc url_rc=$stale_url_rc up_rc=$stale_up_rc readiness_rc=$readiness_up_rc"

lifecycle_step "stale launcher handoff rejection"
HANDOFF_DELAY_FILE="$TMP_DIR/launcher-handoff-delay.marker"
HANDOFF_ACK_FILE="$TMP_DIR/launcher-handoff.ack"
HANDOFF_ACKED_FILE="$TMP_DIR/launcher-handoff.acked"
HANDOFF_CHILD_PID_FILE="$TMP_DIR/launcher-handoff-child.pid"
HANDOFF_META_FILE="$(wh_runtime_meta_file "$HARNESS_ROOT")"
rm -f "$HANDOFF_DELAY_FILE" "$HANDOFF_ACK_FILE" "$HANDOFF_ACKED_FILE" "$HANDOFF_CHILD_PID_FILE"
FAKE_LAUNCHER_CHILD_PID_FILE="$HANDOFF_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE="$HANDOFF_DELAY_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACK_FILE="$HANDOFF_ACK_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACKED_FILE="$HANDOFF_ACKED_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS=2 \
run_fake_harness up --startup-timeout "$STARTUP_TIMEOUT_SECS" >"$TMP_DIR/launcher-handoff-up.log" 2>&1 &
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
    wh_pid_alive "$handoff_child_pid" || break
    sleep 0.05
  done
  if wh_pid_alive "$handoff_child_pid"; then
    echo "lifecycle acceptance: stale launcher handoff child survived cleanup" >&2
    exit 1
  fi
fi

lifecycle_step "concurrent up/down serialization"
CONCURRENT_DELAY_FILE="$TMP_DIR/concurrent-delay.marker"
CONCURRENT_CHILD_PID_FILE="$TMP_DIR/concurrent-child.pid"
rm -f "$CONCURRENT_DELAY_FILE" "$CONCURRENT_CHILD_PID_FILE"
FAKE_LAUNCHER_CHILD_PID_FILE="$CONCURRENT_CHILD_PID_FILE" \
OASIS7_HARNESS_TEST_LAUNCHER_COMMAND="$FAKE_LAUNCHER" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE="$CONCURRENT_DELAY_FILE" \
OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS=2 \
run_fake_harness up --startup-timeout "$STARTUP_TIMEOUT_SECS" >"$TMP_DIR/concurrent-up.log" 2>&1 &
concurrent_up_pid=$!
if ! wait_for_marker "$CONCURRENT_DELAY_FILE" "$LAUNCH_SYNC_TIMEOUT_SECS" "concurrent-up launch synchronization"; then
  cat "$TMP_DIR/concurrent-up.log" >&2 || true
  exit 1
fi
run_fake_harness down >"$TMP_DIR/concurrent-down.log" 2>&1 &
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
  if ! wh_pid_alive "$concurrent_child_pid"; then
    break
  fi
  sleep 0.05
done
if wh_pid_alive "$concurrent_child_pid"; then
  echo "lifecycle acceptance: concurrent up/down left an orphan launcher child" >&2
  exit 1
fi

lifecycle_step "startup timeout cleanup"
FAKE_LAUNCHER_CHILD_PID_FILE="$TIMEOUT_CHILD_PID_FILE" \
FAKE_LAUNCHER_MODE=timeout run_fake_harness up --startup-timeout 1 >"$TMP_DIR/timeout-up.log" 2>&1 && {
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
  if ! wh_pid_alive "$timeout_child_pid"; then
    break
  fi
  sleep 0.05
done
if wh_pid_alive "$timeout_child_pid"; then
  echo "lifecycle acceptance: timeout launcher child survived cleanup" >&2
  exit 1
fi

lifecycle_step "identity-protected cleanup failure"
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
run_fake_harness down >"$TMP_DIR/failure-down.log" 2>&1
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
if ! wh_pid_alive "$failure_pid"; then
  echo "lifecycle acceptance: failed cleanup killed a group without proving identity" >&2
  exit 1
fi
wh_terminate_process_group "$failure_pid" "$failure_pgid" 100 "$failure_identity"
wh_release_ports_reservation "$HARNESS_ROOT" "$failure_token" "$FAILURE_COMMON_DIR"

echo "worktree harness lifecycle acceptance: PASS"
