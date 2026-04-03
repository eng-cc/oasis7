#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-software-safe-chat-regression.sh [options] [run-game-test options...]

Run a Web-first QA regression for Viewer `software_safe` prompt/chat flow.
The script forces `render_mode=software_safe`, then verifies:
- selected agent prompt apply ack
- prompt rollback ack + prompt textarea refresh
- agent chat ack + outbound chatHistory entry
- optional inbound `agent_spoke` event observation

Options:
  --url <url>                    Use an existing viewer URL; skip stack bootstrap
  --out-dir <path>               Artifact root (default: output/playwright/viewer-software-safe)
  --startup-timeout <secs>       Wait timeout for stack URL (default: 240)
  --agent-id <id>                Target agent id (default: agent-0)
  --chat-message <text>          Chat message override (default: auto-generated)
  --agent-spoke-timeout-ms <ms>  Wait for inbound `agent_spoke` (default: 45000)
  --immediate-agent-spoke-timeout-ms <ms>
                                 Wait for inbound `agent_spoke` before any extra step/play
                                 (default: 4000)
  --require-agent-spoke          Treat missing inbound `agent_spoke` as failure
  --headed                       Open browser in headed mode
  Note: when the script bootstraps its own stack, it enables
  `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1` automatically and treats missing
  inbound `agent_spoke` before any extra step/play as a blocking failure.
  --headless                     Open browser in headless mode (default)
  -h, --help                     Show this help

If --url is omitted, the script starts:
  ./scripts/run-game-test.sh [remaining args...]

Artifacts:
  <out-dir>/<run-id>/run-game-test.log
  <out-dir>/<run-id>/agent-browser.log
  <out-dir>/<run-id>/browser_env.json
  <out-dir>/<run-id>/initial_state.json
  <out-dir>/<run-id>/after_apply_state.json
  <out-dir>/<run-id>/after_rollback_state.json
  <out-dir>/<run-id>/after_chat_ack_state.json
  <out-dir>/<run-id>/final_state.json
  <out-dir>/<run-id>/software-safe-chat-summary.json
  <out-dir>/<run-id>/software-safe-chat-summary.md
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
state_auth_ready() { json_get "$1" authReady; }
state_last_error() { json_get "$1" lastError; }
state_logical_time() { json_get "$1" logicalTime; }
state_event_seq() { json_get "$1" eventSeq; }

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
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
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
    "warningCategory": None if sys.argv[3] == "null" else sys.argv[3],
    "runId": sys.argv[4],
    "agentId": sys.argv[5],
    "gameUrl": sys.argv[6],
    "renderMode": sys.argv[7],
    "authReady": sys.argv[8] == "true",
    "applyAck": sys.argv[9] == "true",
    "rollbackAck": sys.argv[10] == "true",
    "promptTextareaCleared": sys.argv[11] == "true",
    "chatAck": sys.argv[12] == "true",
    "outboundHistorySeen": sys.argv[13] == "true",
    "agentSpokeSeen": sys.argv[14] == "true",
    "agentSpokeSeenImmediate": sys.argv[15] == "true",
    "agentSpokeNeededAdvance": sys.argv[16] == "true",
    "requireAgentSpoke": sys.argv[17] == "true",
    "requireImmediateAgentSpoke": sys.argv[18] == "true",
}
print(json.dumps(payload, ensure_ascii=False, indent=2))
PY
}

GAME_URL=""
OUT_ROOT="output/playwright/viewer-software-safe"
STARTUP_TIMEOUT_SECS=240
AGENT_ID="agent-0"
CHAT_MESSAGE=""
AGENT_SPOKE_TIMEOUT_MS=45000
IMMEDIATE_AGENT_SPOKE_TIMEOUT_MS=4000
REQUIRE_AGENT_SPOKE=0
REQUIRE_AGENT_SPOKE_EXPLICIT=0
HEADED=0
STACK_ARGS=()
BOOTSTRAPPED_STACK=0
BOOTSTRAP_USES_BUNDLE=0

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
    --chat-message)
      CHAT_MESSAGE="${2:-}"
      shift 2
      ;;
    --agent-spoke-timeout-ms)
      AGENT_SPOKE_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --immediate-agent-spoke-timeout-ms)
      IMMEDIATE_AGENT_SPOKE_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --require-agent-spoke)
      REQUIRE_AGENT_SPOKE=1
      REQUIRE_AGENT_SPOKE_EXPLICIT=1
      shift
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
[[ "$AGENT_SPOKE_TIMEOUT_MS" =~ ^[0-9]+$ ]] && [[ "$AGENT_SPOKE_TIMEOUT_MS" -gt 0 ]] || { echo "error: --agent-spoke-timeout-ms must be positive" >&2; exit 2; }
[[ "$IMMEDIATE_AGENT_SPOKE_TIMEOUT_MS" =~ ^[0-9]+$ ]] && [[ "$IMMEDIATE_AGENT_SPOKE_TIMEOUT_MS" -gt 0 ]] || { echo "error: --immediate-agent-spoke-timeout-ms must be positive" >&2; exit 2; }

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
after_apply_state_json="$out_dir/after_apply_state.json"
after_rollback_state_json="$out_dir/after_rollback_state.json"
after_chat_ack_state_json="$out_dir/after_chat_ack_state.json"
final_state_json="$out_dir/final_state.json"
summary_json_path="$out_dir/software-safe-chat-summary.json"
summary_md_path="$out_dir/software-safe-chat-summary.md"
screenshot_path="$out_dir/software-safe-chat.png"
session="viewer-softsafe-chat-$run_id"
stack_pid=""

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

  BOOTSTRAPPED_STACK=1
  if [[ "$BOOTSTRAP_USES_BUNDLE" -ne 1 ]]; then
    log_note build_oasis7_viewer_live
    env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live >>"$ab_log" 2>&1
  fi
  if [[ "$REQUIRE_AGENT_SPOKE_EXPLICIT" -ne 1 && "$BOOTSTRAP_USES_BUNDLE" -ne 1 ]]; then
    REQUIRE_AGENT_SPOKE=1
  fi
  if command -v stdbuf >/dev/null 2>&1; then
    stdbuf -oL -eL env OASIS7_RUNTIME_AGENT_CHAT_ECHO=1 ./scripts/run-game-test.sh "${STACK_ARGS[@]}" >"$run_game_test_log" 2>&1 &
  else
    env OASIS7_RUNTIME_AGENT_CHAT_ECHO=1 ./scripts/run-game-test.sh "${STACK_ARGS[@]}" >"$run_game_test_log" 2>&1 &
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
if [[ -z "$CHAT_MESSAGE" ]]; then
  CHAT_MESSAGE="qa software-safe chat ${run_id}"
fi
PROMPT_GOAL="qa software-safe rollback ${run_id}"

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
auth_ready="$(state_auth_ready "$initial_state")"
[[ "$render_mode" == "software_safe" ]] || { echo "error: expected renderMode=software_safe, got $render_mode" >&2; exit 1; }
[[ "$auth_ready" == "true" ]] || { echo "error: viewer auth bootstrap is unavailable in software_safe" >&2; exit 1; }

ab_eval "$session" "window.__AW_TEST__.select('agent:${AGENT_ID}')" >>"$ab_log" 2>&1
wait_for_js_true "(() => window.__AW_TEST__?.getState?.()?.selectedId === ${AGENT_ID@Q})()" 6000 || {
  echo "error: failed to select agent ${AGENT_ID}" >&2
  exit 1
}

ab_eval "$session" "window.__AW_TEST__.sendPromptControl('apply', { agentId: ${AGENT_ID@Q}, shortTermGoal: ${PROMPT_GOAL@Q} })" >>"$ab_log" 2>&1
wait_for_js_true "(() => window.__AW_TEST__?.getState?.()?.lastPromptFeedback?.stage === 'apply_ack')()" 8000 || {
  echo "error: prompt apply did not reach apply_ack" >&2
  exit 1
}
after_apply_state=$(ab_state)
write_json_file "$after_apply_state" "$after_apply_state_json"

ab_eval "$session" "window.__AW_TEST__.sendPromptControl('rollback', { agentId: ${AGENT_ID@Q}, toVersion: 0 })" >>"$ab_log" 2>&1
wait_for_js_true "(() => window.__AW_TEST__?.getState?.()?.lastPromptFeedback?.stage === 'rollback_ack')()" 8000 || {
  echo "error: prompt rollback did not reach rollback_ack" >&2
  exit 1
}
wait_for_js_true "(() => (document.getElementById('prompt-short')?.value ?? null) === '')()" 8000 || {
  echo "error: prompt-short textarea did not refresh after rollback" >&2
  exit 1
}
after_rollback_state=$(ab_state)
write_json_file "$after_rollback_state" "$after_rollback_state_json"

ab_eval "$session" "window.__AW_TEST__.sendAgentChat(${AGENT_ID@Q}, ${CHAT_MESSAGE@Q})" >>"$ab_log" 2>&1
wait_for_js_true "(() => window.__AW_TEST__?.getState?.()?.lastChatFeedback?.stage === 'ack')()" 12000 || {
  echo "error: agent chat did not reach ack" >&2
  exit 1
}
wait_for_js_true "(() => { const h = window.__AW_TEST__?.getState?.()?.chatHistory ?? []; return h.some((entry) => entry && entry.source === 'player' && entry.message === ${CHAT_MESSAGE@Q} && entry.targetAgentId === ${AGENT_ID@Q}); })()" 4000 || {
  echo "error: outbound chat history entry missing" >&2
  exit 1
}
after_chat_ack_state=$(ab_state)
write_json_file "$after_chat_ack_state" "$after_chat_ack_state_json"

agent_spoke_seen=false
agent_spoke_seen_immediate=false
agent_spoke_needed_advance=false
require_immediate_agent_spoke=0
agent_spoke_deadline_ms=$((SECONDS * 1000 + AGENT_SPOKE_TIMEOUT_MS))
agent_spoke_expected_message=""
if [[ "$BOOTSTRAPPED_STACK" -eq 1 && "$BOOTSTRAP_USES_BUNDLE" -eq 0 ]]; then
  agent_spoke_expected_message="[qa-echo] ${CHAT_MESSAGE}"
  require_immediate_agent_spoke=1
fi

agent_spoke_match_script="(() => { const h = window.__AW_TEST__?.getState?.()?.chatHistory ?? []; return h.some((entry) => entry && entry.source === 'event' && entry.agentId === ${AGENT_ID@Q}); })()"
if [[ -n "$agent_spoke_expected_message" ]]; then
  agent_spoke_match_script="(() => { const h = window.__AW_TEST__?.getState?.()?.chatHistory ?? []; return h.some((entry) => entry && entry.source === 'event' && entry.agentId === ${AGENT_ID@Q} && entry.message === ${agent_spoke_expected_message@Q}); })()"
fi

immediate_wait_ms=$AGENT_SPOKE_TIMEOUT_MS
if (( immediate_wait_ms > IMMEDIATE_AGENT_SPOKE_TIMEOUT_MS )); then
  immediate_wait_ms=$IMMEDIATE_AGENT_SPOKE_TIMEOUT_MS
fi
if wait_for_js_true "$agent_spoke_match_script" "$immediate_wait_ms"; then
  agent_spoke_seen=true
  agent_spoke_seen_immediate=true
fi

if [[ "$agent_spoke_seen_immediate" != true && "$require_immediate_agent_spoke" -eq 1 ]]; then
  echo "error: inbound agent_spoke not observed immediately after chat ack without extra control advance (expected_message=${agent_spoke_expected_message:-<any>})" >&2
  exit 1
fi

if [[ "$agent_spoke_seen" != true ]]; then
  for step_batch in 1 2 3 4; do
    if wait_for_js_true "$agent_spoke_match_script" 500; then
      agent_spoke_seen=true
      break
    fi

    remaining_ms=$((agent_spoke_deadline_ms - SECONDS * 1000))
    if (( remaining_ms <= 0 )); then
      break
    fi

    batch_state_before=$(ab_state)
    batch_baseline_logical_time=$(state_logical_time "$batch_state_before")
    batch_baseline_event_seq=$(state_event_seq "$batch_state_before")
    log_note "step_batch_${step_batch}"
    ab_eval "$session" 'window.__AW_TEST__.runSteps(4)' >>"$ab_log" 2>&1 || true

    step_wait_ms=18000
    if (( remaining_ms < step_wait_ms )); then
      step_wait_ms=$remaining_ms
    fi
    wait_for_js_true "(() => {
      const snapshot = window.__AW_TEST__?.getState?.();
      const feedback = snapshot?.lastControlFeedback;
      const stage = String(feedback?.stage || '');
      return Number(snapshot?.logicalTime || 0) > ${batch_baseline_logical_time:-0}
        || Number(snapshot?.eventSeq || 0) > ${batch_baseline_event_seq:-0}
        || stage === 'completed_advanced'
        || stage === 'completed_timeout';
    })()" "$step_wait_ms" >>"$ab_log" 2>&1 || true

    remaining_ms=$((agent_spoke_deadline_ms - SECONDS * 1000))
    if (( remaining_ms <= 0 )); then
      break
    fi
    probe_wait_ms=2000
    if (( remaining_ms < probe_wait_ms )); then
      probe_wait_ms=$remaining_ms
    fi
    if wait_for_js_true "$agent_spoke_match_script" "$probe_wait_ms"; then
      agent_spoke_seen=true
      agent_spoke_needed_advance=true
      break
    fi
  done
fi

if [[ "$agent_spoke_seen" != true && "$REQUIRE_AGENT_SPOKE" -eq 1 ]]; then
  echo "error: inbound agent_spoke not observed within timeout (bootstrapped_stack=$BOOTSTRAPPED_STACK, bootstrap_uses_bundle=$BOOTSTRAP_USES_BUNDLE, expected_message=${agent_spoke_expected_message:-<any>})" >&2
  exit 1
fi

final_state=$(ab_state)
write_json_file "$final_state" "$final_state_json"
ab_screenshot "$session" "$screenshot_path" >>"$ab_log" 2>&1 || true

summary_raw=$(summary_json \
  true \
  null \
  "$([[ "$agent_spoke_seen" == true ]] && printf 'null' || printf 'agent_spoke_timeout')" \
  "$run_id" \
  "$AGENT_ID" \
  "$GAME_URL" \
  "$render_mode" \
  "$auth_ready" \
  true true true true true "$agent_spoke_seen" "$agent_spoke_seen_immediate" "$agent_spoke_needed_advance" "$([[ "$REQUIRE_AGENT_SPOKE" -eq 1 ]] && printf 'true' || printf 'false')" "$([[ "$require_immediate_agent_spoke" -eq 1 ]] && printf 'true' || printf 'false')")
printf '%s\n' "$summary_raw" >"$summary_json_path"
python3 - "$summary_json_path" "$summary_md_path" <<'PY'
import json
import pathlib
import sys
src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
data = json.loads(src.read_text())
lines = [
    '# Viewer software_safe prompt/chat regression summary',
    '',
    f"- ok: `{data['ok']}`",
    f"- failCategory: `{data['failCategory']}`",
    f"- warningCategory: `{data['warningCategory']}`",
    f"- runId: `{data['runId']}`",
    f"- agentId: `{data['agentId']}`",
    f"- renderMode: `{data['renderMode']}`",
    f"- authReady: `{data['authReady']}`",
    f"- applyAck: `{data['applyAck']}`",
    f"- rollbackAck: `{data['rollbackAck']}`",
    f"- promptTextareaCleared: `{data['promptTextareaCleared']}`",
    f"- chatAck: `{data['chatAck']}`",
    f"- outboundHistorySeen: `{data['outboundHistorySeen']}`",
    f"- agentSpokeSeen: `{data['agentSpokeSeen']}`",
    f"- agentSpokeSeenImmediate: `{data['agentSpokeSeenImmediate']}`",
    f"- agentSpokeNeededAdvance: `{data['agentSpokeNeededAdvance']}`",
    f"- requireAgentSpoke: `{data['requireAgentSpoke']}`",
    f"- requireImmediateAgentSpoke: `{data['requireImmediateAgentSpoke']}`",
    f"- gameUrl: `{data['gameUrl']}`",
]
out.write_text("\n".join(lines) + "\n", encoding='utf-8')
PY

printf 'ok: artifacts written to %s\n' "$out_dir"
