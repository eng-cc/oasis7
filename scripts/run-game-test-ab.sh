#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/run-game-test-ab.sh [options] [run-game-test options...]

Run a stable A/B playability loop and emit quantitative metrics:
- A phase: play -> observe -> pause
- B phase: step-chain control probes (no seek)
- Outputs TTFC / effective control hit-rate / max no-progress window

Important guardrail:
- This script is for automated regression probing only.
- It does NOT replace manual long-play sessions or real-player card filling.

Options:
  --url <url>               Use an existing viewer URL; skip stack bootstrap
  --out-dir <path>          Artifact root (default: output/playwright/playability)
  --startup-timeout <secs>  Wait timeout for stack URL (default: 240)
  --progress-timeout-ms <n> Wait timeout for play/step probes that require world progress
                            (default: 12000)
  --headed                  Open browser in headed mode (default, recommended for Viewer Web);
                            defaults to `--use-angle=gl,--ignore-gpu-blocklist` unless
                            `AGENT_BROWSER_ARGS` overrides it
  --headless                Open browser in headless mode; fails fast when WebGL falls back to
                            SwiftShader/software rendering
  -h, --help                Show this help

If --url is omitted, the script starts:
  ./scripts/run-game-test.sh [remaining args...]

Preferred producer/release example:
  ./scripts/run-game-test-ab.sh --bundle-dir output/release/game-launcher-local --no-llm

Artifacts:
  <out-dir>/<run-id>/agent-browser.log
  <out-dir>/<run-id>/playthrough.webm
  <out-dir>/<run-id>/step0-home.png
  <out-dir>/<run-id>/step1-phase-a.png
  <out-dir>/<run-id>/step2-phase-b.png
  <out-dir>/<run-id>/step3-final.png
  <out-dir>/<run-id>/ab_metrics.json
  <out-dir>/<run-id>/ab_metrics.md
  <out-dir>/<run-id>/card_quant_metrics.md
  <out-dir>/<run-id>/browser_env.json
USAGE
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys, time
ms = int(sys.argv[1])
time.sleep(ms / 1000.0)
PY
}

GAME_URL=""
OUT_ROOT="output/playwright/playability"
STARTUP_TIMEOUT_SECS=240
PROGRESS_TIMEOUT_MS=12000
HEADED=1
STACK_ARGS=()

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
    --progress-timeout-ms)
      PROGRESS_TIMEOUT_MS="${2:-}"
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
    *)
      STACK_ARGS+=("$1")
      shift
      ;;
  esac
done

[[ -n "$OUT_ROOT" ]] || { echo "error: --out-dir cannot be empty" >&2; exit 2; }
[[ "$STARTUP_TIMEOUT_SECS" =~ ^[0-9]+$ ]] && [[ "$STARTUP_TIMEOUT_SECS" -gt 0 ]] || { echo "error: --startup-timeout must be a positive integer" >&2; exit 2; }
[[ "$PROGRESS_TIMEOUT_MS" =~ ^[0-9]+$ ]] && [[ "$PROGRESS_TIMEOUT_MS" -gt 0 ]] || { echo "error: --progress-timeout-ms must be a positive integer" >&2; exit 2; }

require_cmd python3
require_cmd rg
ab_require

RUN_ID="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="$OUT_ROOT/$RUN_ID"
mkdir -p "$OUT_DIR"

AB_LOG="$OUT_DIR/agent-browser.log"
RUN_GAME_TEST_LOG="$OUT_DIR/run-game-test.log"
CONSOLE_WARNING_LOG="$OUT_DIR/console_warning_dump.log"
CONSOLE_ALL_LOG="$OUT_DIR/console_all_messages.log"
AB_METRICS_JSON="$OUT_DIR/ab_metrics.json"
AB_METRICS_MD="$OUT_DIR/ab_metrics.md"
CARD_METRICS_MD="$OUT_DIR/card_quant_metrics.md"
BROWSER_ENV_JSON="$OUT_DIR/browser_env.json"
RECORDING_STATUS_FILE="$OUT_DIR/recording_status.txt"
SESSION="playability-ab-$RUN_ID"

STACK_PID=""
STACK_OUTPUT_DIR=""

ab_log_note() {
  printf '### [%s] %s\n' "$1" "$(date '+%H:%M:%S')" | tee -a "$AB_LOG" >/dev/null
}

reopen_game_page() {
  ab_log_note reopen_after_record
  ab_cmd "$SESSION" close >>"$AB_LOG" 2>&1 || true
  ab_open "$SESSION" "$HEADED" "$GAME_URL" >>"$AB_LOG" 2>&1 || return 1
  ab_cmd "$SESSION" wait --load networkidle >>"$AB_LOG" 2>&1 || true
  wait_for_api 20000 >/dev/null || return 1
  fail_if_software_renderer || return 1
  wait_for_connected 60000
}

ab_state() {
  ab_eval "$SESSION" 'window.__AW_TEST__?.getState?.() ?? null'
}

state_tick() { json_get "$1" tick; }
state_event_seq() { json_get "$1" eventSeq; }
state_connection() { json_get "$1" connectionStatus; }
state_last_error() { json_get "$1" lastError; }
state_last_feedback_json() { json_get "$1" lastControlFeedback; }
state_render_mode() { json_get "$1" renderMode; }
state_software_safe_reason() { json_get "$1" softwareSafeReason; }

browser_env() {
  ab_eval "$SESSION" '(() => {
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
      webdriver: navigator.webdriver,
      visibilityState: document.visibilityState,
      hasTestApi: typeof window.__AW_TEST__ === "object",
      state: window.__AW_TEST__?.getState?.() ?? null,
      webglVersion,
      renderer,
      vendor,
    };
  })()'
}

renderer_is_software() {
  local renderer=${1:-}
  [[ "$renderer" == *SwiftShader* || "$renderer" == *llvmpipe* || "$renderer" == *"Software Rasterizer"* || "$renderer" == *"Basic Render Driver"* ]]
}

fail_if_software_renderer() {
  local env_json renderer user_agent browser_args mode_label state_json render_mode software_safe_reason
  env_json=$(browser_env)
  json_to_file "$env_json" "$BROWSER_ENV_JSON"
  renderer=$(json_get "$env_json" renderer)
  user_agent=$(json_get "$env_json" userAgent)
  browser_args=$(ab_browser_args)
  state_json=$(json_get "$env_json" state)
  render_mode=$(state_render_mode "$state_json")
  software_safe_reason=$(state_software_safe_reason "$state_json")
  if renderer_is_software "$renderer"; then
    if [[ "$render_mode" == "software_safe" ]]; then
      echo "note: browser is using SwiftShader/software WebGL, but viewer entered software_safe mode (reason=${software_safe_reason:-unknown}); continue with minimal closure validation" >&2
      return 0
    fi
    if [[ "$HEADED" -eq 1 ]]; then
      mode_label='headed'
    else
      mode_label='headless'
    fi
    echo "error: ${mode_label} browser is using SwiftShader/software WebGL; viewer did not enter software_safe mode (see $BROWSER_ENV_JSON, renderer=$renderer, renderMode=${render_mode:-<none>}, softwareSafeReason=${software_safe_reason:-<none>}, userAgent=$user_agent, agentBrowserArgs=${browser_args:-<none>})" >&2
    return 1
  fi
  return 0
}

wait_for_api() {
  local timeout_ms=${1:-20000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  while (( SECONDS * 1000 < deadline )); do
    if [[ "$(ab_eval "$SESSION" 'typeof window.__AW_TEST__ === "object"')" == "true" ]]; then
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

send_control_probe() {
  local name=$1
  local action=$2
  local payload_json=$3
  local expect_progress=$4
  local timeout_ms=$5
  local before feedback after last_feedback_json accepted reason effect before_tick after_tick before_event after_event progressed first_progress_ms feedback_stage feedback_reason feedback_hint fail_category

  before=$(ab_state)
  before_tick=$(state_tick "$before"); before_tick=${before_tick:-0}
  before_event=$(state_event_seq "$before"); before_event=${before_event:-0}
  feedback=$(ab_eval "$SESSION" "(() => { try { return window.__AW_TEST__?.sendControl?.($(json_quote "$action"), ${payload_json}) ?? null; } catch (err) { return { accepted: false, reason: String(err), effect: 'exception on sendControl' }; } })()")
  accepted=$(json_get "$feedback" accepted)
  reason=$(json_get "$feedback" reason)
  effect=$(json_get "$feedback" effect)
  after="$before"
  progressed=false
  first_progress_ms=""
  local started_ms
  started_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
  local deadline_ms=$((started_ms + timeout_ms))

  while :; do
    local now_ms
    now_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
    if (( now_ms >= deadline_ms )); then
      break
    fi
    sleep_ms 250
    after=$(ab_state)
    after_tick=$(state_tick "$after"); after_tick=${after_tick:-0}
    after_event=$(state_event_seq "$after"); after_event=${after_event:-0}
    last_feedback_json=$(state_last_feedback_json "$after")
    feedback_stage=$(json_get "$last_feedback_json" stage)
    feedback_reason=$(json_get "$last_feedback_json" reason)
    feedback_hint=$(json_get "$last_feedback_json" hint)
    if (( ${after_tick%%.*} > ${before_tick%%.*} || ${after_event%%.*} > ${before_event%%.*} )); then
      progressed=true
      first_progress_ms=$((now_ms - started_ms))
      break
    fi
    if [[ "$expect_progress" == "false" ]] && (( now_ms - started_ms >= 1000 )); then
      break
    fi
    if [[ "$feedback_stage" == "completed_no_progress" || "$feedback_stage" == "blocked" ]]; then
      break
    fi
  done

  after_tick=$(state_tick "$after"); after_tick=${after_tick:-0}
  after_event=$(state_event_seq "$after"); after_event=${after_event:-0}
  last_feedback_json=$(state_last_feedback_json "$after")
  feedback_stage=$(json_get "$last_feedback_json" stage)
  feedback_reason=$(json_get "$last_feedback_json" reason)
  feedback_hint=$(json_get "$last_feedback_json" hint)

  if [[ "$accepted" != "true" ]]; then
    fail_category="rejected"
  elif [[ "$progressed" == "true" ]]; then
    fail_category="progressed"
  elif [[ "$feedback_stage" == "completed_no_progress" ]]; then
    fail_category="completed_no_progress"
  elif [[ "$feedback_stage" == "blocked" ]]; then
    fail_category="blocked_after_accept"
  elif [[ "$(state_connection "$after")" != "connected" ]]; then
    fail_category="disconnected"
  else
    fail_category="timeout_no_delta"
  fi

  python3 - <<'PY' \
    "$name" "$action" "$payload_json" "$expect_progress" "$accepted" "$reason" "$effect" \
    "$before_tick" "$after_tick" "$before_event" "$after_event" "$progressed" "$first_progress_ms" \
    "$feedback_stage" "$feedback_reason" "$feedback_hint" "$fail_category"
import json, sys
name, action, payload_json, expect_progress, accepted, reason, effect, before_tick, after_tick, before_event, after_event, progressed, first_progress_ms, feedback_stage, feedback_reason, feedback_hint, fail_category = sys.argv[1:18]
try:
    payload = json.loads(payload_json)
except Exception:
    payload = payload_json
result = {
    "name": name,
    "action": action,
    "payload": payload,
    "expectProgress": expect_progress == "true",
    "accepted": accepted == "true",
    "reason": reason or None,
    "effect": effect or None,
    "beforeTick": int(float(before_tick or 0)),
    "afterTick": int(float(after_tick or 0)),
    "beforeEventSeq": int(float(before_event or 0)),
    "afterEventSeq": int(float(after_event or 0)),
    "progressed": progressed == "true",
    "firstProgressMs": int(first_progress_ms) if first_progress_ms else None,
    "feedbackStage": feedback_stage or None,
    "feedbackReason": feedback_reason or None,
    "feedbackHint": feedback_hint or None,
    "failCategory": fail_category,
}
print(json.dumps(result, ensure_ascii=False))
PY
}

observe_no_progress_window() {
  local duration_ms=$1
  local started_ms
  started_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
  local end_ms=$((started_ms + duration_ms))
  local state last_tick current_tick stagnation_start max_window now_ms final_event
  state=$(ab_state)
  last_tick=$(state_tick "$state"); last_tick=${last_tick:-0}
  stagnation_start=$started_ms
  max_window=0
  while :; do
    now_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
    (( now_ms >= end_ms )) && break
    sleep_ms 250
    state=$(ab_state)
    current_tick=$(state_tick "$state"); current_tick=${current_tick:-0}
    if [[ "$(state_connection "$state")" == "connected" ]] && (( ${current_tick%%.*} == ${last_tick%%.*} )); then
      local current_window=$((now_ms - stagnation_start))
      (( current_window > max_window )) && max_window=$current_window
    else
      last_tick=$current_tick
      stagnation_start=$now_ms
    fi
  done
  python3 - <<'PY' "$max_window" "$last_tick" "$(state_event_seq "$state")"
import json, sys
print(json.dumps({
    "maxNoProgressWindowMs": int(float(sys.argv[1] or 0)),
    "finalObservedTick": int(float(sys.argv[2] or 0)),
    "finalObservedEventSeq": int(float(sys.argv[3] or 0)),
}, ensure_ascii=False))
PY
}

stop_stack() {
  if [[ -n "$STACK_PID" ]] && kill -0 "$STACK_PID" >/dev/null 2>&1; then
    kill "$STACK_PID" >/dev/null 2>&1 || true
    wait "$STACK_PID" >/dev/null 2>&1 || true
  fi
  STACK_PID=""
}

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  ab_cmd "$SESSION" close >/dev/null 2>&1 || true
  stop_stack
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

if [[ -z "$GAME_URL" ]]; then
  {
    echo "### [bootstrap_stack] $(date '+%H:%M:%S')"
    echo "./scripts/run-game-test.sh ${STACK_ARGS[*]}"
    echo
  } | tee -a "$AB_LOG" >/dev/null

  if command -v stdbuf >/dev/null 2>&1; then
    stdbuf -oL -eL ./scripts/run-game-test.sh "${STACK_ARGS[@]}" >"$RUN_GAME_TEST_LOG" 2>&1 &
  else
    ./scripts/run-game-test.sh "${STACK_ARGS[@]}" >"$RUN_GAME_TEST_LOG" 2>&1 &
  fi
  STACK_PID=$!

  for ((i = 0; i < STARTUP_TIMEOUT_SECS; i++)); do
    if ! kill -0 "$STACK_PID" >/dev/null 2>&1; then
      echo "error: run-game-test stack exited unexpectedly" >&2
      tail -n 120 "$RUN_GAME_TEST_LOG" >&2 || true
      exit 1
    fi
    GAME_URL="$(sed -n 's/^- URL: \(http[^[:space:]]*\)$/\1/p' "$RUN_GAME_TEST_LOG" | tail -n 1)"
    STACK_OUTPUT_DIR="$(sed -n 's/^- Logs: \(.*\)$/\1/p' "$RUN_GAME_TEST_LOG" | tail -n 1)"
    [[ -n "$GAME_URL" ]] && break
    sleep 1
  done

  if [[ -z "$GAME_URL" ]]; then
    echo "error: timeout waiting for game URL from run-game-test.sh" >&2
    tail -n 120 "$RUN_GAME_TEST_LOG" >&2 || true
    exit 1
  fi
else
  {
    echo "### [bootstrap_stack] $(date '+%H:%M:%S')"
    echo "skip stack bootstrap; using provided URL: $GAME_URL"
    echo
  } | tee -a "$AB_LOG" >/dev/null
fi

ab_log_note open
ab_open "$SESSION" "$HEADED" "$GAME_URL" 2>&1 | tee -a "$AB_LOG" >/dev/null
ab_log_note wait_network
ab_cmd "$SESSION" wait --load networkidle 2>&1 | tee -a "$AB_LOG" >/dev/null || true

SNAPSHOT_OK=0
for attempt in 1 2 3 4 5; do
  ab_log_note "snapshot_initial_attempt_${attempt}"
  if snapshot_output=$(ab_cmd "$SESSION" snapshot -i 2>&1); then
    printf "%s\n" "$snapshot_output" | tee -a "$AB_LOG" >/dev/null
    SNAPSHOT_OK=1
    break
  fi
  printf "%s\n" "$snapshot_output" | tee -a "$AB_LOG" >/dev/null
  sleep 1
done
if [[ "$SNAPSHOT_OK" -ne 1 ]]; then
  echo "warning: snapshot still failing after retries; continue with eval path" | tee -a "$AB_LOG" >/dev/null
fi

wait_for_api 20000 >/dev/null || { echo "error: __AW_TEST__ unavailable before initial connect" >&2; exit 1; }
fail_if_software_renderer || exit 1
set +e
initial=$(wait_for_connected 60000)
initial_wait_status=$?
set -e
if [[ "$initial_wait_status" -ne 0 ]]; then
  if [[ "$initial_wait_status" -eq 2 ]]; then
    echo "error: initial connection failed due to viewer fatal error: $(state_last_error "$initial")" >&2
  else
    echo "error: initial connection failed (status=$(state_connection "$initial"), lastFeedback=$(state_last_feedback_json "$initial"), lastError=$(state_last_error "$initial"))" >&2
  fi
  exit 1
fi
ab_screenshot "$SESSION" "$OUT_DIR/step0-home.png" >>"$AB_LOG" 2>&1 || true

RECORDING_ACTIVE=0
ab_log_note record_start
if ab_cmd "$SESSION" record start "$OUT_DIR/playthrough.webm" >>"$AB_LOG" 2>&1; then
  RECORDING_ACTIVE=1
  echo "info: record_start resets Viewer Web session; reopen immediately to keep recorded run connected" | tee -a "$AB_LOG" >/dev/null
  ab_log_note record_reopen_sync
  set +e
  initial_after_record=$(reopen_game_page)
  initial_after_record_status=$?
  set -e
  if [[ "$initial_after_record_status" -ne 0 ]]; then
    if [[ "$initial_after_record_status" -eq 2 ]]; then
      echo "error: connection failed after record_start sync due to viewer fatal error: $(state_last_error "$initial_after_record")" >&2
    else
      echo "error: connection failed after record_start sync (status=$(state_connection "$initial_after_record"), lastFeedback=$(state_last_feedback_json "$initial_after_record"), lastError=$(state_last_error "$initial_after_record"))" >&2
    fi
    exit 1
  fi
  printf '%s
' 'agent-browser record start creates a fresh browser context for Viewer Web; the forced reopen needed to recover connection also ends that recording. playthrough.webm is therefore a best-effort pre-sync clip, while screenshots/state/metrics remain the authoritative closure evidence.' >"$RECORDING_STATUS_FILE"
  RECORDING_ACTIVE=0
fi

phaseA_play=$(send_control_probe phase_a_play play '{}' true "$PROGRESS_TIMEOUT_MS")
no_progress_observation=$(observe_no_progress_window 6000)
phaseA_pause=$(send_control_probe phase_a_pause pause '{}' false 2500)
ab_screenshot "$SESSION" "$OUT_DIR/step1-phase-a.png" >>"$AB_LOG" 2>&1 || true

phaseB_step_primary=$(send_control_probe phase_b_step_primary step '{"count":8}' true "$PROGRESS_TIMEOUT_MS")
phaseB_step_followup=$(send_control_probe phase_b_step_followup step '{"count":2}' true "$PROGRESS_TIMEOUT_MS")
ab_screenshot "$SESSION" "$OUT_DIR/step2-phase-b.png" >>"$AB_LOG" 2>&1 || true

set +e
final_state=$(wait_for_connected 8000)
final_wait_status=$?
set -e
if [[ "$final_wait_status" -ne 0 ]]; then
  if [[ "$final_wait_status" -eq 2 ]]; then
    echo "error: final connection failed due to viewer fatal error: $(state_last_error "$final_state")" >&2
  else
    echo "error: final connection failed (status=$(state_connection "$final_state"), lastFeedback=$(state_last_feedback_json "$final_state"), lastError=$(state_last_error "$final_state"))" >&2
  fi
  exit 1
fi
ab_screenshot "$SESSION" "$OUT_DIR/step3-final.png" >>"$AB_LOG" 2>&1 || true

AB_RESULT_JSON=$(python3 scripts/render-ab-metrics.py   "$RUN_ID" "$GAME_URL" "$initial" "$final_state"   "$phaseA_play" "$phaseA_pause" "$phaseB_step_primary" "$phaseB_step_followup" "$no_progress_observation"   "$AB_METRICS_JSON" "$AB_METRICS_MD" "$CARD_METRICS_MD")

ab_log_note console_all
ab_cmd "$SESSION" console >"$CONSOLE_ALL_LOG" 2>&1 || true
python3 - "$CONSOLE_ALL_LOG" "$CONSOLE_WARNING_LOG" <<'PY'
import pathlib, sys
src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
if src.exists():
    warnings = [line for line in src.read_text(encoding='utf-8', errors='replace').splitlines() if 'warn' in line.lower() or 'warning' in line.lower()]
    out.write_text("\n".join(warnings) + ("\n" if warnings else ""), encoding='utf-8')
else:
    out.write_text("", encoding='utf-8')
PY

if [[ "${RECORDING_ACTIVE:-0}" -eq 1 ]]; then
  ab_log_note record_stop
  ab_cmd "$SESSION" record stop >>"$AB_LOG" 2>&1 || true
fi
ab_log_note close
ab_cmd "$SESSION" close >>"$AB_LOG" 2>&1 || true

if [[ -n "$STACK_OUTPUT_DIR" && -d "$STACK_OUTPUT_DIR" ]]; then
  cp "$STACK_OUTPUT_DIR/session.meta" "$OUT_DIR/startup.session.meta" 2>/dev/null || true
  cp "$STACK_OUTPUT_DIR/oasis7_viewer_live.log" "$OUT_DIR/startup.world.initial.log" 2>/dev/null || true
  cp "$STACK_OUTPUT_DIR/web_viewer.log" "$OUT_DIR/startup.web.initial.log" 2>/dev/null || true
fi

stop_stack

echo "playability A/B run complete"
echo "- run id: $RUN_ID"
echo "- url: $GAME_URL"
echo "- artifacts: $OUT_DIR"
echo "- metrics json: $AB_METRICS_JSON"
echo "- metrics summary: $AB_METRICS_MD"
echo "- card metrics snippet: $CARD_METRICS_MD"
echo "- reminder: regression probe only; still run manual long-play before final judgment"
