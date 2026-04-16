#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-software-safe-step-regression.sh [options] [run-game-test options...]

Run a Web-first QA regression for Viewer `software_safe` realtime-only closure.
The script forces `render_mode=software_safe`, then verifies:
- page load + `__AW_TEST__` availability
- runtime connection reaches `connected`
- target agent selection is reflected in state + DOM
- realtime surfaces remain readable after selection
- internal test API `step` still updates `lastControlFeedback` without relying on page buttons

Options:
  --url <url>               Use an existing viewer URL; skip stack bootstrap
  --out-dir <path>          Artifact root (default: output/playwright/viewer-software-safe-step)
  --startup-timeout <secs>  Wait timeout for stack URL (default: 240)
  --agent-id <id>           Target agent id (default: agent-0)
  --step-timeout-ms <ms>    Wait timeout for internal step completion/progress (default: 15000)
  --headed                  Open browser in headed mode
  --headless                Open browser in headless mode (default)
  -h, --help                Show this help

If --url is omitted, the script starts:
  ./scripts/run-game-test.sh [remaining args...]

Artifacts:
  <out-dir>/<run-id>/run-game-test.log
  <out-dir>/<run-id>/agent-browser.log
  <out-dir>/<run-id>/browser_env.json
  <out-dir>/<run-id>/initial_state.json
  <out-dir>/<run-id>/after_select_state.json
  <out-dir>/<run-id>/step_request.json
  <out-dir>/<run-id>/after_step_state.json
  <out-dir>/<run-id>/final_state.json
  <out-dir>/<run-id>/software-safe-step-summary.json
  <out-dir>/<run-id>/software-safe-step-summary.md
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
query["render_mode"] = "software_safe"
query["test_api"] = "1"
print(urlunparse(parts._replace(query=urlencode(query))))
PY
}

extract_ws_host_port() {
  python3 - "$1" <<'PY'
from urllib.parse import urlparse, parse_qs
import sys
raw = sys.argv[1]
parts = urlparse(raw)
ws_values = parse_qs(parts.query).get("ws", [])
if not ws_values:
    raise SystemExit(1)
ws = urlparse(ws_values[0])
host = ws.hostname or ""
port = ws.port or 0
if not host or not port:
    raise SystemExit(1)
print(f"{host} {port}")
PY
}

wait_for_tcp_listener() {
  local host=$1
  local port=$2
  local timeout_secs=${3:-20}
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if python3 - "$host" "$port" <<'PY'
import socket
import sys
host = sys.argv[1]
port = int(sys.argv[2])
try:
    with socket.create_connection((host, port), timeout=1):
        pass
except OSError:
    raise SystemExit(1)
raise SystemExit(0)
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

state_connection() { json_get "$1" connectionStatus; }
state_render_mode() { json_get "$1" renderMode; }
state_last_error() { json_get "$1" lastError; }
state_logical_time() { json_get "$1" logicalTime; }
state_event_seq() { json_get "$1" eventSeq; }
state_last_feedback_json() { json_get "$1" lastControlFeedback; }

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

wait_for_connected() {
  local timeout_ms=${1:-20000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  local last_error=''
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    last_error=$(state_last_error "$state")
    if [[ -n "$last_error" ]]; then
      printf '%s\n' "$state"
      return 2
    fi
    if [[ "$(state_connection "$state")" == "connected" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
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

write_json_file() {
  local raw_json=$1
  local out_path=$2
  json_to_file "$raw_json" "$out_path"
}

browser_env() {
  ab_eval "$session" '(() => {
    const canvas = document.createElement("canvas");
    let gl = null;
    let renderer = null;
    let vendor = null;
    let webglVersion = null;
    try {
      gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
      if (gl) {
        webglVersion = gl.getParameter(gl.VERSION);
        const dbg = gl.getExtension("WEBGL_debug_renderer_info");
        if (dbg) {
          renderer = gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL);
          vendor = gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL);
        }
      }
    } catch (err) {
      renderer = `error:${String(err)}`;
    }
    return {
      userAgent: navigator.userAgent,
      visibilityState: document.visibilityState,
      hasTestApi: typeof window.__AW_TEST__ === "object",
      state: window.__AW_TEST__?.getState?.() ?? null,
      webglVersion,
      renderer,
      vendor,
    };
  })()'
}

summary_json() {
  python3 - "$@" <<'PY'
import json
import sys
payload = {
    "ok": sys.argv[1] == "true",
    "failCategory": None if sys.argv[2] == "null" else sys.argv[2],
    "runId": sys.argv[3],
    "agentId": sys.argv[4],
    "gameUrl": sys.argv[5],
    "renderMode": sys.argv[6],
    "stepAccepted": sys.argv[7] == "true",
    "selectedAgentVisible": sys.argv[8] == "true",
    "playbackControlsVisible": sys.argv[9] == "true",
    "logicalTimeAdvanced": sys.argv[10] == "true",
    "eventSeqAdvanced": sys.argv[11] == "true",
    "feedbackStage": None if sys.argv[12] == "null" else sys.argv[12],
    "feedbackReason": None if sys.argv[13] == "null" else sys.argv[13],
}
print(json.dumps(payload, ensure_ascii=False, indent=2))
PY
}

GAME_URL=""
OUT_ROOT="output/playwright/viewer-software-safe-step"
STARTUP_TIMEOUT_SECS=240
AGENT_ID="agent-0"
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
    --step-timeout-ms)
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
    -h|--help)
      usage
      exit 0
      ;;
    --bundle-dir)
      BOOTSTRAP_USES_BUNDLE=1
      STACK_ARGS+=("$1" "${2:-}")
      shift 2
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
require_cmd rg
: "${AGENT_BROWSER_ARGS:=}"
ab_require

run_id="$(date +%Y%m%d-%H%M%S)"
out_dir="$OUT_ROOT/$run_id"
mkdir -p "$out_dir"

ab_log="$out_dir/agent-browser.log"
run_game_test_log="$out_dir/run-game-test.log"
browser_env_json="$out_dir/browser_env.json"
initial_state_json="$out_dir/initial_state.json"
after_select_state_json="$out_dir/after_select_state.json"
step_request_json="$out_dir/step_request.json"
after_step_state_json="$out_dir/after_step_state.json"
final_state_json="$out_dir/final_state.json"
summary_json_path="$out_dir/software-safe-step-summary.json"
summary_md_path="$out_dir/software-safe-step-summary.md"
screenshot_path="$out_dir/software-safe-step.png"
session="viewer-softsafe-step-$run_id"

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

if [[ -z "$GAME_URL" ]]; then
  {
    echo "### [bootstrap_stack] $(date '+%H:%M:%S')"
    echo "./scripts/run-game-test.sh ${STACK_ARGS[*]}"
    echo
  } | tee -a "$ab_log" >/dev/null

  if [[ "$BOOTSTRAP_USES_BUNDLE" -ne 1 ]]; then
    log_note build_oasis7_viewer_live
    env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live >>"$ab_log" 2>&1
  fi

  if command -v stdbuf >/dev/null 2>&1; then
    stdbuf -oL -eL ./scripts/run-game-test.sh "${STACK_ARGS[@]}" >"$run_game_test_log" 2>&1 &
  else
    ./scripts/run-game-test.sh "${STACK_ARGS[@]}" >"$run_game_test_log" 2>&1 &
  fi
  stack_pid=$!

  for ((i = 0; i < STARTUP_TIMEOUT_SECS; i++)); do
    if ! kill -0 "$stack_pid" >/dev/null 2>&1; then
      echo "error: run-game-test stack exited unexpectedly" >&2
      tail -n 120 "$run_game_test_log" >&2 || true
      exit 1
    fi
    GAME_URL="$(sed -n 's/^- URL: \(http[^[:space:]]*\)$/\1/p' "$run_game_test_log" | tail -n 1)"
    [[ -n "$GAME_URL" ]] && break
    sleep 1
  done

  if [[ -z "$GAME_URL" ]]; then
    echo "error: timeout waiting for game URL from run-game-test.sh" >&2
    tail -n 120 "$run_game_test_log" >&2 || true
    exit 1
  fi
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
initial_state=$(wait_for_connected 30000) || {
  echo "error: viewer failed to connect (lastError=$(state_last_error "$initial_state"))" >&2
  exit 1
}
write_json_file "$initial_state" "$initial_state_json"
write_json_file "$(browser_env)" "$browser_env_json"

render_mode="$(state_render_mode "$initial_state")"
[[ "$render_mode" == "software_safe" ]] || { echo "error: expected renderMode=software_safe, got $render_mode" >&2; exit 1; }

log_note select_agent
ab_eval "$session" "window.__AW_TEST__.select('agent:${AGENT_ID}')" >>"$ab_log" 2>&1
wait_for_js_true "(() => window.__AW_TEST__?.getState?.()?.selectedId === ${AGENT_ID@Q})()" 6000 || {
  echo "error: failed to select agent ${AGENT_ID}" >&2
  exit 1
}
wait_for_js_true "(() => { const agentId = ${AGENT_ID@Q}; return !!document.querySelector('[data-select-kind=\"agent\"][data-select-id=\"' + agentId + '\"][data-selected=\"true\"]'); })()" 6000 || {
  echo "error: selected agent ${AGENT_ID} is not reflected in DOM" >&2
  exit 1
}
after_select_state=$(ab_state)
write_json_file "$after_select_state" "$after_select_state_json"

before_logical_time=$(state_logical_time "$after_select_state")
before_event_seq=$(state_event_seq "$after_select_state")
before_logical_time=${before_logical_time:-0}
before_event_seq=${before_event_seq:-0}

wait_for_js_true "(() => {
  const text = document.body?.innerText || '';
  const hasRealtimeSurface =
    text.includes('Recent Events') ||
    text.includes('最近事件') ||
    text.includes('Formal Gameplay Summary') ||
    text.includes('正式玩法摘要');
  const hasPlaybackControls =
    text.includes('Playback Controls') ||
    text.includes('回放控制') ||
    text.includes('Step x1') ||
    text.includes('单步 x1') ||
    text.includes('Step custom count') ||
    text.includes('按自定义步数推进');
  return hasRealtimeSurface && !hasPlaybackControls;
})()" 4000 || {
  echo "error: realtime surface missing or playback controls are still visible in DOM" >&2
  exit 1
}

log_note step
step_request=$(ab_eval "$session" "(() => { try { return window.__AW_TEST__?.sendControl?.('step', null) ?? null; } catch (err) { return { accepted: false, reason: String(err), effect: 'exception on sendControl' }; } })()")
write_json_file "$step_request" "$step_request_json"
step_accepted=$(json_get "$step_request" accepted)
if [[ "$step_accepted" == "true" ]]; then
  wait_for_js_true "(() => {
    const snapshot = window.__AW_TEST__?.getState?.();
    const feedback = snapshot?.lastControlFeedback;
    const stage = String(feedback?.stage || '');
    if (feedback?.action !== 'step') {
      return false;
    }
    return stage === 'completed_advanced'
      || stage === 'completed_timeout'
      || stage === 'completed_no_progress'
      || stage === 'blocked'
      || Number(snapshot?.logicalTime || 0) > ${before_logical_time}
      || Number(snapshot?.eventSeq || 0) > ${before_event_seq};
  })()" "$STEP_TIMEOUT_MS" || {
    echo "error: step control did not reach terminal feedback or advance within timeout" >&2
    exit 1
  }
fi

after_step_state=$(ab_state)
write_json_file "$after_step_state" "$after_step_state_json"

selected_agent_visible=true
playback_controls_visible=false
after_logical_time=$(state_logical_time "$after_step_state")
after_event_seq=$(state_event_seq "$after_step_state")
after_logical_time=${after_logical_time:-0}
after_event_seq=${after_event_seq:-0}

logical_time_advanced=false
event_seq_advanced=false
if (( ${after_logical_time%%.*} > ${before_logical_time%%.*} )); then
  logical_time_advanced=true
fi
if (( ${after_event_seq%%.*} > ${before_event_seq%%.*} )); then
  event_seq_advanced=true
fi

final_state=$(ab_state)
write_json_file "$final_state" "$final_state_json"
ab_screenshot "$session" "$screenshot_path" >>"$ab_log" 2>&1 || true

feedback_json=$(state_last_feedback_json "$after_step_state")
feedback_stage=$(json_get "$feedback_json" stage)
feedback_reason=$(json_get "$feedback_json" reason)
feedback_action=$(json_get "$feedback_json" action)
if [[ "$step_accepted" != "true" ]]; then
  feedback_stage=${feedback_stage:-rejected}
  if [[ -z "${feedback_reason:-}" || "$feedback_reason" == "null" ]]; then
    feedback_reason=$(json_get "$step_request" reason)
  fi
fi
if [[ "$step_accepted" == "true" && "$feedback_action" != "step" ]]; then
  echo "error: lastControlFeedback action drifted to ${feedback_action:-<empty>}" >&2
  exit 1
fi
if [[ "$(state_connection "$after_step_state")" != "connected" ]]; then
  echo "error: viewer disconnected after step (lastError=$(state_last_error "$after_step_state"))" >&2
  exit 1
fi
fail_category=null
if [[ "$step_accepted" == "true" && "$logical_time_advanced" != true && "$event_seq_advanced" != true && "$feedback_stage" != "completed_advanced" ]]; then
  fail_category="no_progress_after_step"
elif [[ "$step_accepted" == "true" && "$feedback_stage" != "completed_advanced" && "$feedback_stage" != "completed_timeout" && "$feedback_stage" != "completed_no_progress" && "$feedback_stage" != "blocked" ]]; then
  fail_category="missing_terminal_feedback"
fi

summary_raw=$(summary_json \
  "$([[ "$fail_category" == "null" ]] && printf 'true' || printf 'false')" \
  "$fail_category" \
  "$run_id" \
  "$AGENT_ID" \
  "$GAME_URL" \
  "$render_mode" \
  "$step_accepted" \
  "$selected_agent_visible" \
  "$playback_controls_visible" \
  "$logical_time_advanced" \
  "$event_seq_advanced" \
  "${feedback_stage:-null}" \
  "${feedback_reason:-null}")
printf '%s\n' "$summary_raw" >"$summary_json_path"
python3 - "$summary_json_path" "$summary_md_path" <<'PY'
import json
import pathlib
import sys
src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
data = json.loads(src.read_text())
lines = [
    '# Viewer software_safe realtime-only regression summary',
    '',
    f"- ok: `{data['ok']}`",
    f"- failCategory: `{data['failCategory']}`",
    f"- runId: `{data['runId']}`",
    f"- agentId: `{data['agentId']}`",
    f"- renderMode: `{data['renderMode']}`",
    f"- stepAccepted: `{data['stepAccepted']}`",
    f"- selectedAgentVisible: `{data['selectedAgentVisible']}`",
    f"- playbackControlsVisible: `{data['playbackControlsVisible']}`",
    f"- logicalTimeAdvanced: `{data['logicalTimeAdvanced']}`",
    f"- eventSeqAdvanced: `{data['eventSeqAdvanced']}`",
    f"- feedbackStage: `{data['feedbackStage']}`",
    f"- feedbackReason: `{data['feedbackReason']}`",
    f"- gameUrl: `{data['gameUrl']}`",
]
out.write_text("\n".join(lines) + "\n", encoding='utf-8')
PY

if [[ "$fail_category" != "null" ]]; then
  echo "error: software_safe step did not produce playable advancement (failCategory=${fail_category}, feedbackStage=${feedback_stage:-null}, logicalTimeAdvanced=${logical_time_advanced}, eventSeqAdvanced=${event_seq_advanced})" >&2
  exit 1
fi

printf 'ok: artifacts written to %s\n' "$out_dir"
