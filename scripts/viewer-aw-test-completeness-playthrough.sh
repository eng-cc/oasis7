#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/cargo-dev-lib.sh"
source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-aw-test-completeness-playthrough.sh [options] [run-launcher-stack options...]

Verify that window.__AW_TEST__ is a complete step-by-step test control surface:
API discovery, state read, selection/focus, snapshot action, live control,
runSteps, recommended gameplay action, and final state artifact capture.

Options:
  --url <url>               Use an existing viewer URL; skip stack bootstrap
  --out-dir <path>          Artifact root (default: output/playwright/aw-test-completeness)
  --startup-timeout <secs>  Wait timeout for stack URL (default: 240)
  --agent-id <id>           Target agent id (default: starter-agent-0)
  --step-timeout-ms <ms>    Per-step wait timeout (default: 15000)
  --headed                  Open browser in headed mode
  --headless                Open browser in headless mode (default)
  -h, --help                Show help
USAGE
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys, time
time.sleep(int(sys.argv[1]) / 1000.0)
PY
}

append_query_params() {
  python3 - "$1" <<'PY'
from urllib.parse import urlparse, parse_qsl, urlencode, urlunparse
import sys
raw = sys.argv[1]
parts = urlparse(raw)
query = dict(parse_qsl(parts.query, keep_blank_values=True))
query["render_mode"] = "viewer"
query["test_api"] = "1"
print(urlunparse(parts._replace(query=urlencode(query))))
PY
}

extract_ws_host_port() {
  python3 - "$1" <<'PY'
from urllib.parse import urlparse, parse_qs
import sys
parts = urlparse(sys.argv[1])
ws_values = parse_qs(parts.query).get("ws", [])
if not ws_values:
    raise SystemExit(1)
ws = urlparse(ws_values[0])
if not ws.hostname or not ws.port:
    raise SystemExit(1)
print(f"{ws.hostname} {ws.port}")
PY
}

wait_for_tcp_listener() {
  local host=$1
  local port=$2
  local timeout_secs=${3:-20}
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if python3 - "$host" "$port" <<'PY'
import socket, sys
try:
    with socket.create_connection((sys.argv[1], int(sys.argv[2])), timeout=1):
        pass
except OSError:
    raise SystemExit(1)
PY
    then
      return 0
    fi
    sleep 1
  done
  return 1
}

normalize_eval_token() {
  local raw=${1:-}
  raw=$(printf '%s' "$raw" | tr -d '\r\n')
  raw=${raw#\"}
  raw=${raw%\"}
  printf '%s' "$raw"
}

log_note() {
  printf '### [%s] %s\n' "$1" "$(date '+%H:%M:%S')" | tee -a "$ab_log" >/dev/null
}

ab_state() {
  ab_eval "$session" 'window.__AW_TEST__?.getState?.() ?? null'
}

json_path() {
  python3 - "$1" "$2" <<'PY'
import json, sys
try:
    value = json.loads(sys.argv[1])
except Exception:
    print("")
    raise SystemExit(0)
for part in sys.argv[2].split("."):
    if isinstance(value, dict):
        value = value.get(part)
    else:
        value = None
        break
if value is None:
    print("")
elif isinstance(value, bool):
    print("true" if value else "false")
elif isinstance(value, (dict, list)):
    print(json.dumps(value, ensure_ascii=False))
else:
    print(value)
PY
}

write_json_file() {
  json_to_file "$1" "$2"
}

wait_for_api() {
  local timeout_ms=${1:-20000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  while (( SECONDS * 1000 < deadline )); do
    local ready
    ready=$(normalize_eval_token "$(ab_eval "$session" 'typeof window.__AW_TEST__ === "object" ? "ready" : "missing"')")
    if [[ "$ready" == "ready" || "$ready" == "true" ]]; then
      return 0
    fi
    sleep_ms 200
  done
  return 1
}

wait_for_js_true() {
  local script=$1
  local timeout_ms=${2:-10000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  while (( SECONDS * 1000 < deadline )); do
    local value
    value=$(normalize_eval_token "$(ab_eval "$session" "$script")")
    if [[ "$value" == "true" ]]; then
      return 0
    fi
    sleep_ms 250
  done
  return 1
}

state_connection() { json_path "$1" connectionStatus; }
state_last_error() { json_path "$1" lastError; }

GAME_URL=""
OUT_ROOT="output/playwright/aw-test-completeness"
STARTUP_TIMEOUT_SECS=240
AGENT_ID="starter-agent-0"
STEP_TIMEOUT_MS=15000
HEADED=0
STACK_ARGS=()
BOOTSTRAP_USES_BUNDLE=0
stack_pid=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      GAME_URL="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_ROOT="${2:-}"
      shift 2
      ;;
    --startup-timeout)
      STARTUP_TIMEOUT_SECS="${2:-}"
      shift 2
      ;;
    --agent-id)
      AGENT_ID="${2:-}"
      shift 2
      ;;
    --step-timeout-ms|--beat-timeout-ms)
      STEP_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --headed)
      HEADED=1
      shift
      ;;
    --headless)
      HEADED=0
      shift
      ;;
    --bundle-dir)
      BOOTSTRAP_USES_BUNDLE=1
      STACK_ARGS+=("$1" "${2:-}")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      STACK_ARGS+=("$1")
      shift
      ;;
  esac
done

[[ -n "$OUT_ROOT" ]] || { echo "error: --out-dir cannot be empty" >&2; exit 2; }
[[ "$STARTUP_TIMEOUT_SECS" =~ ^[0-9]+$ ]] && [[ "$STARTUP_TIMEOUT_SECS" -gt 0 ]] || { echo "error: --startup-timeout must be positive" >&2; exit 2; }
[[ "$STEP_TIMEOUT_MS" =~ ^[0-9]+$ ]] && [[ "$STEP_TIMEOUT_MS" -gt 0 ]] || { echo "error: --step-timeout-ms must be positive" >&2; exit 2; }

require_cmd python3
: "${AGENT_BROWSER_ARGS:=}"
ab_require

run_id="$(date +%Y%m%d-%H%M%S)"
out_dir="$OUT_ROOT/$run_id"
states_dir="$out_dir/states"
mkdir -p "$states_dir"

ab_log="$out_dir/agent-browser.log"
run_game_test_log="$out_dir/launcher-stack.log"
summary_json_path="$out_dir/aw-test-completeness-summary.json"
summary_md_path="$out_dir/aw-test-completeness-summary.md"
steps_jsonl="$out_dir/aw-test-steps.jsonl"
screenshot_path="$out_dir/aw-test-completeness.png"
session="aw-test-completeness-$run_id"
: >"$steps_jsonl"

stop_stack() {
  if [[ -n "$stack_pid" ]] && kill -0 "$stack_pid" >/dev/null 2>&1; then
    kill "$stack_pid" >/dev/null 2>&1 || true
    wait "$stack_pid" >/dev/null 2>&1 || true
  fi
  stack_pid=""
}

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  ab_cmd "$session" close >/dev/null 2>&1 || true
  stop_stack
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

capture_state() {
  local label=$1
  local raw
  local path="$states_dir/${label}.json"
  raw=$(ab_state)
  write_json_file "$raw" "$path"
  printf '%s\n' "$path"
}

record_step() {
  local step_id=$1
  local action_label=$2
  local assertion_label=$3
  local status=$4
  local result_file=${5:-}
  local state_file=${6:-}
  local reason=${7:-}
  python3 - "$steps_jsonl" "$step_id" "$action_label" "$assertion_label" "$status" "$result_file" "$state_file" "$reason" <<'PY'
import json, sys
path, step_id, action, assertion, status, result_file, state_file, reason = sys.argv[1:]
with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps({
        "step": step_id,
        "action": action,
        "assertion": assertion,
        "status": status,
        "result_file": result_file or None,
        "state_file": state_file or None,
        "reason": reason or None,
    }, ensure_ascii=False) + "\n")
PY
}

run_api_step() {
  local step_id=$1
  local action_label=$2
  local eval_js=$3
  local assertion_label=$4
  local assertion_js=$5
  log_note "$step_id"
  local result_file="$out_dir/${step_id}.result.json"
  local raw
  raw=$(ab_eval "$session" "$eval_js")
  write_json_file "$raw" "$result_file"
  local ok=false
  if wait_for_js_true "$assertion_js" "$STEP_TIMEOUT_MS"; then
    ok=true
  fi
  local state_file
  state_file=$(capture_state "$step_id")
  if [[ "$ok" == "true" ]]; then
    record_step "$step_id" "$action_label" "$assertion_label" "pass" "$result_file" "$state_file"
    return 0
  fi
  record_step "$step_id" "$action_label" "$assertion_label" "fail" "$result_file" "$state_file" "assertion timed out"
  return 1
}

if [[ -z "$GAME_URL" ]]; then
  {
    echo "### [bootstrap_stack] $(date '+%H:%M:%S')"
    if [[ "${#STACK_ARGS[@]}" -gt 0 ]]; then
      echo "./scripts/run-launcher-stack.sh ${STACK_ARGS[*]}"
    else
      echo "./scripts/run-launcher-stack.sh"
    fi
    echo
  } | tee -a "$ab_log" >/dev/null

  if [[ "$BOOTSTRAP_USES_BUNDLE" -ne 1 ]]; then
    log_note build_oasis7_viewer_live
    oasis7_cargo_dev build -p oasis7 --bin oasis7_viewer_live >>"$ab_log" 2>&1
  fi

  if command -v stdbuf >/dev/null 2>&1; then
    if [[ "${#STACK_ARGS[@]}" -gt 0 ]]; then
      stdbuf -oL -eL ./scripts/run-launcher-stack.sh "${STACK_ARGS[@]}" >"$run_game_test_log" 2>&1 &
    else
      stdbuf -oL -eL ./scripts/run-launcher-stack.sh >"$run_game_test_log" 2>&1 &
    fi
  else
    if [[ "${#STACK_ARGS[@]}" -gt 0 ]]; then
      ./scripts/run-launcher-stack.sh "${STACK_ARGS[@]}" >"$run_game_test_log" 2>&1 &
    else
      ./scripts/run-launcher-stack.sh >"$run_game_test_log" 2>&1 &
    fi
  fi
  stack_pid=$!

  for ((i = 0; i < STARTUP_TIMEOUT_SECS; i++)); do
    if ! kill -0 "$stack_pid" >/dev/null 2>&1; then
      echo "error: launcher stack exited unexpectedly" >&2
      tail -n 120 "$run_game_test_log" >&2 || true
      exit 1
    fi
    GAME_URL="$(sed -n 's/^- URL: \(http[^[:space:]]*\)$/\1/p' "$run_game_test_log" | tail -n 1)"
    [[ -n "$GAME_URL" ]] && break
    sleep 1
  done
  [[ -n "$GAME_URL" ]] || { echo "error: timeout waiting for game URL" >&2; exit 1; }
fi

GAME_URL="$(append_query_params "$GAME_URL")"
if ws_host_port=$(extract_ws_host_port "$GAME_URL" 2>/dev/null); then
  read -r ws_host ws_port <<<"$ws_host_port"
  wait_for_tcp_listener "$ws_host" "$ws_port" 20 || {
    echo "error: websocket bridge did not become ready: ${ws_host}:${ws_port}" >&2
    exit 1
  }
fi
sleep 4

log_note open
ab_open "$session" "$HEADED" "$GAME_URL" >>"$ab_log" 2>&1
ab_cmd "$session" wait --load networkidle >>"$ab_log" 2>&1 || true
sleep_ms 2500

wait_for_api 20000 || { echo "error: __AW_TEST__ unavailable" >&2; exit 1; }
initial_state='null'
for _ in $(seq 1 120); do
  initial_state=$(ab_state)
  if [[ "$(state_connection "$initial_state")" == "connected" ]]; then
    break
  fi
  if [[ -n "$(state_last_error "$initial_state")" ]]; then
    echo "error: viewer failed to connect (lastError=$(state_last_error "$initial_state"))" >&2
    exit 1
  fi
  sleep_ms 250
done
[[ "$(state_connection "$initial_state")" == "connected" ]] || { echo "error: viewer did not connect" >&2; exit 1; }
write_json_file "$initial_state" "$states_dir/initial.json"

agent_id_json=$(json_quote "$AGENT_ID")
overall_status="pass"

run_api_step "assert_api_surface" \
  "discover __AW_TEST__ methods, controls, semantic actions, and step example" \
  '(() => { const api = window.__AW_TEST__; const required = ["getState","describeControls","fillControlExample","sendControl","sendGameplayAction","runSteps","select","focus","sendAgentChat","sendPromptControl","injectSnapshot"]; const methods = Object.fromEntries(required.map((name) => [name, typeof api?.[name]])); const controls = api.describeControls(); const stepExample = api.fillControlExample("step"); const controlNames = (controls.controls || []).map((entry) => entry.action); const semanticNames = (controls.semanticActions || []).map((entry) => entry.action); return { ok: required.every((name) => typeof api?.[name] === "function") && ["play","pause","step"].every((name) => controlNames.includes(name)) && semanticNames.includes("sendAgentChat") && semanticNames.includes("sendPromptControl") && Number(stepExample?.count || 0) >= 1, methods, controls, stepExample }; })()' \
  "__AW_TEST__ exposes the complete test control surface" \
  '(() => { const api = window.__AW_TEST__; const required = ["getState","describeControls","fillControlExample","sendControl","sendGameplayAction","runSteps","select","focus","sendAgentChat","sendPromptControl","injectSnapshot"]; const controls = api?.describeControls?.() || {}; const controlNames = (controls.controls || []).map((entry) => entry.action); return required.every((name) => typeof api?.[name] === "function") && ["play","pause","step"].every((name) => controlNames.includes(name)) && Number(api.fillControlExample("step")?.count || 0) >= 1; })()' \
  || overall_status="fail"

# assert_api_progression: the following steps prove __AW_TEST__ can drive and
# inspect a real step-by-step gameplay path, not only expose method names.
run_api_step "assert_get_state" \
  "read state through __AW_TEST__.getState" \
  '(() => { const state = window.__AW_TEST__.getState(); return { ok: !!state && state.connectionStatus === "connected", selectedId: state.selectedId, connectionStatus: state.connectionStatus, hasGameplaySummary: !!state.gameplaySummary }; })()' \
  "getState returns connected state with gameplay summary" \
  '(() => { const state = window.__AW_TEST__?.getState?.(); return state?.connectionStatus === "connected" && !!state?.gameplaySummary; })()' \
  || overall_status="fail"

run_api_step "assert_select_focus" \
  "select and focus the target agent through __AW_TEST__" \
  "(() => { const selectResult = window.__AW_TEST__.select('agent:${AGENT_ID}'); const focusResult = window.__AW_TEST__.focus('agent:${AGENT_ID}'); return { ok: selectResult.ok === true && focusResult.ok === true, selectResult, focusResult }; })()" \
  "select/focus update selectedId" \
  "(() => window.__AW_TEST__?.getState?.()?.selectedId === ${agent_id_json})()" \
  || overall_status="fail"

run_api_step "assert_snapshot_action" \
  "request a snapshot through __AW_TEST__.sendGameplayAction" \
  "(() => window.__AW_TEST__.sendGameplayAction('request_snapshot'))()" \
  "snapshot action produces acknowledged gameplay feedback" \
  "(() => { const f = window.__AW_TEST__?.getState?.()?.lastGameplayActionFeedback || {}; return f.action === 'request_snapshot' && f.stage === 'ack' && f.ok === true; })()" \
  || overall_status="fail"

run_api_step "assert_control_step" \
  "advance one step through __AW_TEST__.sendControl" \
  "(() => window.__AW_TEST__.sendControl('step', {count: 1}))()" \
  "sendControl returns or records a live control feedback stage" \
  "(() => { const f = window.__AW_TEST__?.getState?.()?.lastControlFeedback || {}; return ['accepted','queued','completed_advanced','completed_no_progress','blocked','rejected'].includes(String(f.stage || '')); })()" \
  || overall_status="fail"

run_api_step "assert_run_steps" \
  "advance via __AW_TEST__.runSteps using fillControlExample-compatible payload" \
  "(() => window.__AW_TEST__.runSteps({count: 1}))()" \
  "runSteps returns a feedback object and keeps lastControlFeedback inspectable" \
  "(() => { const f = window.__AW_TEST__?.getState?.()?.lastControlFeedback || {}; return ['accepted','queued','completed_advanced','completed_no_progress','blocked','rejected'].includes(String(f.stage || '')); })()" \
  || overall_status="fail"

run_api_step "assert_recommended_action" \
  "submit the current recommended action through __AW_TEST__.sendGameplayAction" \
  '(() => { const state = window.__AW_TEST__.getState(); const action = state.gameplaySummary?.recommendedAction; const result = window.__AW_TEST__.sendGameplayAction(action); return { ok: !!action && result?.ok !== false, action, result }; })()' \
  "recommended action can be resolved and produces gameplay feedback or explicit result" \
  '(() => { const state = window.__AW_TEST__?.getState?.(); const f = state?.lastGameplayActionFeedback || {}; return !!state?.gameplaySummary?.recommendedAction && (!!f.action || !!f.stage || !!f.reason || !!f.effect); })()' \
  || overall_status="fail"

final_state_path=$(capture_state "final")
ab_screenshot "$session" "$screenshot_path" >>"$ab_log" 2>&1 || true

python3 - "$steps_jsonl" "$summary_json_path" "$summary_md_path" "$run_id" "$GAME_URL" "$overall_status" "$final_state_path" "$screenshot_path" <<'PY'
import json
import pathlib
import sys

steps_path = pathlib.Path(sys.argv[1])
summary_json = pathlib.Path(sys.argv[2])
summary_md = pathlib.Path(sys.argv[3])
run_id = sys.argv[4]
game_url = sys.argv[5]
overall = sys.argv[6]
final_state_path = sys.argv[7]
screenshot_path = sys.argv[8]

steps = [json.loads(line) for line in steps_path.read_text(encoding="utf-8").splitlines() if line.strip()]
payload = {
    "task": "TASK-GAME-076",
    "runId": run_id,
    "surface": "__AW_TEST__",
    "ok": overall == "pass" and all(step["status"] == "pass" for step in steps),
    "gameUrl": game_url,
    "stepCount": len(steps),
    "passedStepCount": sum(1 for step in steps if step["status"] == "pass"),
    "failedStepCount": sum(1 for step in steps if step["status"] != "pass"),
    "steps": steps,
    "finalState": final_state_path,
    "screenshot": screenshot_path,
}
summary_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

lines = [
    "# __AW_TEST__ Completeness Playthrough",
    "",
    f"- ok: `{payload['ok']}`",
    f"- surface: `{payload['surface']}`",
    f"- runId: `{run_id}`",
    f"- stepCount: `{payload['stepCount']}`",
    f"- passedStepCount: `{payload['passedStepCount']}`",
    f"- failedStepCount: `{payload['failedStepCount']}`",
    f"- gameUrl: `{game_url}`",
    f"- screenshot: `{screenshot_path}`",
    "",
    "## Steps",
    "",
]
for step in steps:
    lines.append(f"- `{step['step']}`: `{step['status']}`")
    lines.append(f"  - action: {step['action']}")
    lines.append(f"  - assertion: {step['assertion']}")
    if step.get("reason"):
        lines.append(f"  - reason: {step['reason']}")
summary_md.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

printf '__AW_TEST__ completeness summary: %s\n' "$summary_json_path"
printf '__AW_TEST__ completeness report: %s\n' "$summary_md_path"

if [[ "$overall_status" != "pass" ]]; then
  exit 1
fi
