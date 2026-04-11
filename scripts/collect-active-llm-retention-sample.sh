#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/collect-active-llm-retention-sample.sh [options] [run-game-test options...]

Collect one active-LLM formal retention sample through the Web Viewer:
1. bootstrap or reuse a run-game-test stack
2. advance through the software_safe first-session floor
3. confirm PostOnboarding entry
4. run a longer live-play window and capture state samples

Options:
  --url <url>                 Reuse an existing Viewer URL instead of bootstrapping a stack
  --out-dir <path>            Artifact root (default: output/playwright/retention-active-llm-formal)
  --startup-timeout <secs>    Wait timeout for stack URL (default: 240)
  --feedback-timeout-ms <n>   Wait timeout for step feedback completion (default: 30000)
  --play-duration-secs <n>    Live-play duration after PostOnboarding (default: 600)
  --sample-interval-ms <n>    Poll interval during live-play sampling (default: 15000)
  --run-id <name>             Override artifact/session run id suffix
  --session <name>            agent-browser session name prefix
  -h, --help                  Show this help

Examples:
  ./scripts/collect-active-llm-retention-sample.sh --viewer-port 4673 --web-bind 127.0.0.1:5511 --live-bind 127.0.0.1:5523 --chain-status-bind 127.0.0.1:5621
  ./scripts/collect-active-llm-retention-sample.sh --url http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&test_api=1 --play-duration-secs 180
USAGE
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys
import time

time.sleep(int(sys.argv[1]) / 1000.0)
PY
}

normalize_eval_token() {
  local raw=${1:-}
  raw=$(printf '%s' "$raw" | tr -d '\r\n')
  raw=${raw#\"}
  raw=${raw%\"}
  printf '%s' "$raw"
}

game_url=""
out_root="output/playwright/retention-active-llm-formal"
startup_timeout_secs=240
feedback_timeout_ms=30000
play_duration_secs=600
sample_interval_ms=15000
run_id_override=""
session_prefix="active-llm-retention"
stack_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      game_url="${2:-}"
      shift 2
      ;;
    --out-dir)
      out_root="${2:-}"
      shift 2
      ;;
    --startup-timeout)
      startup_timeout_secs="${2:-}"
      shift 2
      ;;
    --feedback-timeout-ms)
      feedback_timeout_ms="${2:-}"
      shift 2
      ;;
    --play-duration-secs)
      play_duration_secs="${2:-}"
      shift 2
      ;;
    --sample-interval-ms)
      sample_interval_ms="${2:-}"
      shift 2
      ;;
    --run-id)
      run_id_override="${2:-}"
      shift 2
      ;;
    --session)
      session_prefix="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      stack_args+=("$1")
      shift
      ;;
  esac
done

[[ -n "$out_root" ]] || { echo "error: --out-dir cannot be empty" >&2; exit 2; }
[[ "$startup_timeout_secs" =~ ^[0-9]+$ ]] && [[ "$startup_timeout_secs" -gt 0 ]] || {
  echo "error: --startup-timeout must be a positive integer" >&2
  exit 2
}
[[ "$feedback_timeout_ms" =~ ^[0-9]+$ ]] && [[ "$feedback_timeout_ms" -gt 0 ]] || {
  echo "error: --feedback-timeout-ms must be a positive integer" >&2
  exit 2
}
[[ "$play_duration_secs" =~ ^[0-9]+$ ]] && [[ "$play_duration_secs" -gt 0 ]] || {
  echo "error: --play-duration-secs must be a positive integer" >&2
  exit 2
}
[[ "$sample_interval_ms" =~ ^[0-9]+$ ]] && [[ "$sample_interval_ms" -gt 0 ]] || {
  echo "error: --sample-interval-ms must be a positive integer" >&2
  exit 2
}

ab_require
require_cmd rg

stamp="$(date +%Y%m%d-%H%M%S)-$$"
run_id="${run_id_override:-active-llm-retention-${stamp}}"
out_dir="$out_root/$run_id"
mkdir -p "$out_dir"

ab_log="$out_dir/agent-browser.log"
run_log="$out_dir/run-game-test.log"
summary_json_path="$out_dir/retention-sample-summary.json"
summary_md_path="$out_dir/retention-sample-summary.md"
browser_env_path="$out_dir/browser-env.json"
console_log="$out_dir/console.log"
console_errors_log="$out_dir/console.errors.log"
initial_state_path="$out_dir/state-initial.json"
step8_state_path="$out_dir/state-step8.json"
entry_state_path="$out_dir/state-post-onboarding-entry.json"
play_timeline_path="$out_dir/state-play-timeline.json"
play_samples_jsonl_path="$out_dir/state-play-samples.jsonl"
body_entry_path="$out_dir/body-post-onboarding.txt"
body_final_path="$out_dir/body-final.txt"
shot_initial="$out_dir/00-initial.png"
shot_entry="$out_dir/01-post-onboarding-entry.png"
shot_final="$out_dir/02-final.png"
session_run_id=$(printf '%s' "$run_id" | tr -cs 'A-Za-z0-9_.-' '-')
session="${session_prefix}-${session_run_id}"

touch "$run_log"

stack_pid=""
stack_logs_dir=""

log_note() {
  printf '### [%s] %s\n' "$1" "$(date '+%H:%M:%S')" | tee -a "$ab_log" >/dev/null
}

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$stack_pid" ]] && kill -0 "$stack_pid" >/dev/null 2>&1; then
    kill "$stack_pid" >/dev/null 2>&1 || true
    wait "$stack_pid" >/dev/null 2>&1 || true
  fi
  ab_cmd "$session" close >/dev/null 2>&1 || true
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

if [[ -z "$game_url" ]]; then
  if command -v stdbuf >/dev/null 2>&1; then
    stdbuf -oL -eL ./scripts/run-game-test.sh --run-id "$run_id" "${stack_args[@]}" > >(tee "$run_log") 2>&1 &
  else
    ./scripts/run-game-test.sh --run-id "$run_id" "${stack_args[@]}" > >(tee "$run_log") 2>&1 &
  fi
  stack_pid=$!

  for ((i = 0; i < startup_timeout_secs; i++)); do
    if ! kill -0 "$stack_pid" >/dev/null 2>&1; then
      echo "error: run-game-test.sh exited unexpectedly" >&2
      tail -n 120 "$run_log" >&2 || true
      exit 1
    fi
    game_url="$(sed -n 's/^- URL: \(http[^[:space:]]*\)$/\1/p' "$run_log" | tail -n 1)"
    stack_logs_dir="$(sed -n 's/^- Logs: \(.*\)$/\1/p' "$run_log" | tail -n 1)"
    [[ -n "$game_url" ]] && break
    sleep 1
  done

  [[ -n "$game_url" ]] || {
    echo "error: timeout waiting for Viewer URL from run-game-test.sh" >&2
    tail -n 120 "$run_log" >&2 || true
    exit 1
  }
fi

ab_state() {
  ab_eval "$session" 'window.__AW_TEST__?.getState?.() ?? null'
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
      url: window.location.href,
      title: document.title,
      hasTestApi: typeof window.__AW_TEST__ === "object",
      state: window.__AW_TEST__?.getState?.() ?? null,
      renderer,
      vendor,
      webglVersion,
      userAgent: navigator.userAgent,
    };
  })()'
}

state_connection() { json_get "$1" connectionStatus; }
state_selected_kind() { json_get "$1" selectedKind; }
state_selected_id() { json_get "$1" selectedId; }
state_gameplay_stage() { json_get "$1" gameplaySummary.stageId; }
state_last_error() { json_get "$1" lastError; }
state_last_feedback_json() { json_get "$1" lastControlFeedback; }
feedback_stage() { json_get "$1" stage; }

wait_for_api() {
  local timeout_ms=${1:-20000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local ready='missing'
  while (( SECONDS * 1000 < deadline )); do
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

wait_for_selected_agent() {
  local timeout_ms=${1:-8000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    if [[ "$(state_selected_kind "$state")" == "agent" && -n "$(state_selected_id "$state")" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

wait_for_feedback_stage() {
  local expected_stage=$1
  local timeout_ms=${2:-8000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  local feedback='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    feedback=$(state_last_feedback_json "$state")
    if [[ "$(feedback_stage "$feedback")" == "$expected_stage" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

log_note open
ab_open "$session" 1 "$game_url" >>"$ab_log" 2>&1
log_note wait_network
ab_cmd "$session" wait --load networkidle >>"$ab_log" 2>&1 || true

wait_for_api 60000 || {
  ab_cmd "$session" console >"$console_log" 2>&1 || true
  ab_cmd "$session" errors >"$console_errors_log" 2>&1 || true
  echo "error: __AW_TEST__ unavailable" >&2
  exit 1
}

initial_state=$(wait_for_connected 30000) || {
  echo "error: Viewer did not reach connected state (lastError=$(state_last_error "$initial_state"))" >&2
  exit 1
}
json_to_file "$initial_state" "$initial_state_path"
browser_env_json=$(browser_env)
json_to_file "$browser_env_json" "$browser_env_path"
log_note screenshot_initial
ab_screenshot "$session" "$shot_initial" >>"$ab_log" 2>&1

log_note step8
ab_eval "$session" "window.__AW_TEST__.sendControl('step', {count: 8})" >>"$ab_log" 2>&1
step8_state=$(wait_for_feedback_stage "completed_advanced" "$feedback_timeout_ms") || {
  echo "error: step(8) did not produce completed_advanced feedback" >&2
  exit 1
}
json_to_file "$step8_state" "$step8_state_path"

log_note select_first_agent
ab_eval "$session" "window.__AW_TEST__.runSteps('panel=show;layout=command;select=first_agent;wait=0.6')" >>"$ab_log" 2>&1
selected_state=$(wait_for_selected_agent 8000) || {
  echo "error: failed to select first agent after step(8)" >&2
  exit 1
}
entry_state="$selected_state"
if [[ "$(state_gameplay_stage "$entry_state")" != "post_onboarding" ]]; then
  log_note preroll_to_post_onboarding
  for attempt in 1 2 3 4; do
    ab_eval "$session" "window.__AW_TEST__.sendControl('step', {count: 4})" >>"$ab_log" 2>&1
    entry_state=$(wait_for_feedback_stage "completed_advanced" "$feedback_timeout_ms") || {
      echo "error: preroll step(4) did not produce completed_advanced feedback" >&2
      exit 1
    }
    if [[ "$(state_gameplay_stage "$entry_state")" == "post_onboarding" ]]; then
      break
    fi
  done
fi
json_to_file "$entry_state" "$entry_state_path"
ab_cmd "$session" get text body >"$body_entry_path" 2>/dev/null || true
log_note screenshot_entry
ab_screenshot "$session" "$shot_entry" >>"$ab_log" 2>&1

[[ "$(state_gameplay_stage "$entry_state")" == "post_onboarding" ]] || {
  echo "error: failed to reach post_onboarding before timed play window" >&2
  exit 1
}

log_note play_window
: >"$play_samples_jsonl_path"
play_duration_ms=$((play_duration_secs * 1000))
play_started_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
play_result=$(ab_eval "$session" "window.__AW_TEST__.sendControl('play', {})")
python3 - "$play_result" <<'PY' >>"$play_samples_jsonl_path"
import json
import sys

raw = sys.argv[1]
try:
    payload = json.loads(raw)
except Exception:
    payload = raw
print(json.dumps({"marker": "play_sent", "playResult": payload}, ensure_ascii=False))
PY

while :; do
  sleep_ms "$sample_interval_ms"
  sample_state=$(ab_state)
  now_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
  elapsed_ms=$((now_ms - play_started_ms))
  python3 - "$elapsed_ms" "$sample_state" <<'PY' >>"$play_samples_jsonl_path"
import json
import sys

elapsed_ms = int(sys.argv[1])
state = json.loads(sys.argv[2])
feedback = state.get("lastControlFeedback") or {}
summary = state.get("gameplaySummary") or {}
payload = {
    "marker": "play_sample",
    "state": {
        "elapsedMs": elapsed_ms,
        "connectionStatus": state.get("connectionStatus"),
        "logicalTime": state.get("logicalTime"),
        "eventSeq": state.get("eventSeq"),
        "tick": state.get("tick"),
        "lastError": state.get("lastError"),
        "gameplaySummary": {
            "stageId": summary.get("stageId"),
            "goalId": summary.get("goalId"),
            "goalTitle": summary.get("goalTitle"),
            "progressPercent": summary.get("progressPercent"),
            "blockerKind": summary.get("blockerKind"),
            "blockerDetail": summary.get("blockerDetail"),
            "nextStepHint": summary.get("nextStepHint"),
        },
        "lastControlFeedback": {
            "id": feedback.get("id"),
            "action": feedback.get("action"),
            "stage": feedback.get("stage"),
            "effect": feedback.get("effect"),
            "reason": feedback.get("reason"),
            "hint": feedback.get("hint"),
            "deltaLogicalTime": feedback.get("deltaLogicalTime"),
            "deltaEventSeq": feedback.get("deltaEventSeq"),
        } if feedback else None,
    },
}
print(json.dumps(payload, ensure_ascii=False))
PY
  if (( elapsed_ms >= play_duration_ms )); then
    break
  fi
  sample_connection=$(state_connection "$sample_state")
  sample_last_error=$(state_last_error "$sample_state")
  sample_feedback=$(state_last_feedback_json "$sample_state")
  sample_feedback_stage=$(feedback_stage "$sample_feedback")
  if [[ "$sample_connection" != "connected" || -n "$sample_last_error" || "$sample_feedback_stage" == "blocked" ]]; then
    break
  fi
done

play_ended_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
actual_play_duration_ms=$((play_ended_ms - play_started_ms))
pre_pause_state=$(ab_state)
pause_result=$(ab_eval "$session" "window.__AW_TEST__.sendControl('pause', {})")
sleep_ms 1200
final_state=$(ab_state)
python3 - \
  "$play_samples_jsonl_path" \
  "$play_timeline_path" \
  "$play_duration_ms" \
  "$actual_play_duration_ms" \
  "$sample_interval_ms" \
  "$play_result" \
  "$entry_state" \
  "$pre_pause_state" \
  "$pause_result" \
  "$final_state" <<'PY'
import json
import pathlib
import sys

samples_path = pathlib.Path(sys.argv[1])
timeline_path = pathlib.Path(sys.argv[2])
target_play_duration_ms = int(sys.argv[3])
actual_play_duration_ms = int(sys.argv[4])
sample_interval_ms = int(sys.argv[5])
play_result = json.loads(sys.argv[6])
entry_state = json.loads(sys.argv[7])
pre_pause_state = json.loads(sys.argv[8])
pause_result = json.loads(sys.argv[9])
final_state = json.loads(sys.argv[10])

timeline = []
for line in samples_path.read_text(encoding="utf-8").splitlines():
    if line.strip():
        timeline.append(json.loads(line))

first_post_onboarding_state = None
entry_summary = entry_state.get("gameplaySummary") or {}
if entry_summary.get("stageId") == "post_onboarding":
    first_post_onboarding_state = entry_state
for entry in timeline:
    state = entry.get("state")
    if isinstance(state, dict):
        summary = state.get("gameplaySummary") or {}
        if first_post_onboarding_state is None and summary.get("stageId") == "post_onboarding":
            first_post_onboarding_state = state
            break

payload = {
    "targetPlayDurationMs": target_play_duration_ms,
    "playDurationMs": actual_play_duration_ms,
    "sampleIntervalMs": sample_interval_ms,
    "playResult": play_result,
    "firstPostOnboardingState": first_post_onboarding_state,
    "prePauseState": pre_pause_state,
    "pauseResult": pause_result,
    "finalState": final_state,
    "timeline": timeline,
}
timeline_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

ab_cmd "$session" get text body >"$body_final_path" 2>/dev/null || true
log_note screenshot_final
ab_screenshot "$session" "$shot_final" >>"$ab_log" 2>&1

ab_cmd "$session" console >"$console_log" 2>&1 || true
ab_cmd "$session" errors >"$console_errors_log" 2>&1 || true

python3 - \
  "$summary_json_path" \
  "$summary_md_path" \
  "$game_url" \
  "$stack_logs_dir" \
  "$browser_env_path" \
  "$initial_state_path" \
  "$step8_state_path" \
  "$entry_state_path" \
  "$play_timeline_path" \
  "$play_samples_jsonl_path" \
  "$body_entry_path" \
  "$body_final_path" \
  "$shot_initial" \
  "$shot_entry" \
  "$shot_final" \
  "$console_log" \
  "$console_errors_log" <<'PY'
import json
import pathlib
import sys

summary_json_path = pathlib.Path(sys.argv[1])
summary_md_path = pathlib.Path(sys.argv[2])
game_url = sys.argv[3]
stack_logs_dir = sys.argv[4]
browser_env_path = pathlib.Path(sys.argv[5])
initial_state_path = pathlib.Path(sys.argv[6])
step8_state_path = pathlib.Path(sys.argv[7])
entry_state_path = pathlib.Path(sys.argv[8])
play_timeline_path = pathlib.Path(sys.argv[9])
play_samples_jsonl_path = pathlib.Path(sys.argv[10])
body_entry_path = pathlib.Path(sys.argv[11])
body_final_path = pathlib.Path(sys.argv[12])
shot_initial = pathlib.Path(sys.argv[13])
shot_entry = pathlib.Path(sys.argv[14])
shot_final = pathlib.Path(sys.argv[15])
console_log = pathlib.Path(sys.argv[16])
console_errors_log = pathlib.Path(sys.argv[17])


def load_json(path: pathlib.Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_text(path: pathlib.Path) -> str:
    if not path.exists():
      return ""
    return path.read_text(encoding="utf-8", errors="replace")


browser_env = load_json(browser_env_path)
initial_state = load_json(initial_state_path)
step8_state = load_json(step8_state_path)
entry_state = load_json(entry_state_path)
play_timeline = load_json(play_timeline_path)

timeline_entries = play_timeline.get("timeline") or []
play_samples = [
    entry.get("state") for entry in timeline_entries
    if isinstance(entry, dict) and isinstance(entry.get("state"), dict)
]

logical_times = [
    sample.get("logicalTime") for sample in play_samples
    if isinstance(sample.get("logicalTime"), int)
]
event_seqs = [
    sample.get("eventSeq") for sample in play_samples
    if isinstance(sample.get("eventSeq"), int)
]
max_logical_time = max(logical_times) if logical_times else None
max_event_seq = max(event_seqs) if event_seqs else None

step8_feedback = (step8_state.get("lastControlFeedback") or {})
entry_summary = entry_state.get("gameplaySummary") or {}
first_post_onboarding_state = play_timeline.get("firstPostOnboardingState") or {}
pre_pause_state = play_timeline.get("prePauseState") or {}
final_state = play_timeline.get("finalState") or {}
final_summary = final_state.get("gameplaySummary") or {}
pre_pause_feedback = pre_pause_state.get("lastControlFeedback") or {}
final_feedback = final_state.get("lastControlFeedback") or {}
reached_post_onboarding = any(
    summary.get("stageId") == "post_onboarding"
    for summary in (entry_summary, first_post_onboarding_state.get("gameplaySummary") or {}, final_summary)
)
first_post_onboarding_summary = first_post_onboarding_state.get("gameplaySummary") or {}
first_post_onboarding_goal_id = first_post_onboarding_summary.get("goalId")
first_post_onboarding_progress = first_post_onboarding_summary.get("progressPercent")
final_progress = final_summary.get("progressPercent")
final_goal_id = final_summary.get("goalId")
successful_goal_ids = {
    "post_onboarding.choose_first_expansion_tradeoff",
    "post_onboarding.choose_midloop_path",
}
retention_progressed = (
    final_summary.get("stageId") == "post_onboarding"
    and final_goal_id in successful_goal_ids
    and isinstance(final_progress, (int, float))
    and final_progress >= 92
    and (
        not isinstance(first_post_onboarding_progress, (int, float))
        or final_progress > first_post_onboarding_progress
        or final_goal_id != first_post_onboarding_goal_id
    )
)

checks = {
    "connected": initial_state.get("connectionStatus") == "connected",
    "step8Advanced": step8_feedback.get("stage") == "completed_advanced",
    "selectedAgent": entry_state.get("selectedKind") == "agent" and bool(entry_state.get("selectedId")),
    "reachedPostOnboarding": reached_post_onboarding,
    "playStayedConnected": all(sample.get("connectionStatus") == "connected" for sample in play_samples) if play_samples else False,
    "playAdvanced": (
        max_logical_time is not None
        and max_logical_time > (step8_state.get("logicalTime") or 0)
    ) or (
        max_event_seq is not None
        and max_event_seq > (step8_state.get("eventSeq") or 0)
    ),
    "noRuntimeError": not bool(final_state.get("lastError")),
    "playNotBlocked": pre_pause_feedback.get("stage") != "blocked",
    "retentionProgressed": retention_progressed,
}

summary = {
    "result": "pass" if all(checks.values()) else "watch",
    "gameUrl": game_url,
    "stackLogsDir": stack_logs_dir or None,
    "artifacts": {
        "browserEnv": str(browser_env_path),
        "initialState": str(initial_state_path),
        "step8State": str(step8_state_path),
        "selectedEntryState": str(entry_state_path),
        "playTimeline": str(play_timeline_path),
        "playSamplesJsonl": str(play_samples_jsonl_path),
        "bodyEntry": str(body_entry_path),
        "bodyFinal": str(body_final_path),
        "initialScreenshot": str(shot_initial),
        "postOnboardingEntryScreenshot": str(shot_entry),
        "finalScreenshot": str(shot_final),
        "consoleLog": str(console_log),
        "consoleErrorsLog": str(console_errors_log),
    },
    "checks": checks,
    "notes": {
        "renderer": browser_env.get("renderer"),
        "renderMode": (browser_env.get("state") or {}).get("renderMode"),
        "step8Effect": step8_feedback.get("effect"),
        "playDurationMs": play_timeline.get("playDurationMs"),
        "sampleCount": len(play_samples),
        "maxLogicalTime": max_logical_time,
        "maxEventSeq": max_event_seq,
        "entryGoalTitle": entry_summary.get("goalTitle"),
        "entryGoalId": entry_summary.get("goalId"),
        "entryProgressPercent": entry_summary.get("progressPercent"),
        "firstPostOnboardingLogicalTime": first_post_onboarding_state.get("logicalTime"),
        "firstPostOnboardingGoalId": first_post_onboarding_goal_id,
        "firstPostOnboardingProgressPercent": first_post_onboarding_progress,
        "prePauseGoalTitle": (pre_pause_state.get("gameplaySummary") or {}).get("goalTitle"),
        "prePauseGoalId": (pre_pause_state.get("gameplaySummary") or {}).get("goalId"),
        "prePauseProgressPercent": (pre_pause_state.get("gameplaySummary") or {}).get("progressPercent"),
        "prePauseFeedbackStage": pre_pause_feedback.get("stage"),
        "prePauseFeedbackEffect": pre_pause_feedback.get("effect"),
        "finalGoalTitle": final_summary.get("goalTitle"),
        "finalGoalId": final_summary.get("goalId"),
        "finalProgressPercent": final_summary.get("progressPercent"),
        "finalFeedbackStage": final_feedback.get("stage"),
        "finalFeedbackEffect": final_feedback.get("effect"),
    },
}

summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

entry_excerpt = "\n".join(load_text(body_entry_path).splitlines()[:20]).strip()
final_excerpt = "\n".join(load_text(body_final_path).splitlines()[:24]).strip()

lines = [
    "# Active-LLM formal retention sample summary",
    "",
    f"- result: `{summary['result']}`",
    f"- url: `{game_url}`",
    f"- stack logs: `{stack_logs_dir or 'n/a'}`",
    "",
    "## Checks",
]
for key, value in checks.items():
    lines.append(f"- {key}: `{value}`")
lines.extend([
    "",
    "## Notes",
    f"- renderer: `{summary['notes']['renderer']}`",
    f"- renderMode: `{summary['notes']['renderMode']}`",
    f"- step8 effect: `{summary['notes']['step8Effect']}`",
    f"- play duration ms: `{summary['notes']['playDurationMs']}`",
    f"- sample count: `{summary['notes']['sampleCount']}`",
    f"- max logicalTime: `{summary['notes']['maxLogicalTime']}`",
    f"- max eventSeq: `{summary['notes']['maxEventSeq']}`",
    f"- entry goal: `{summary['notes']['entryGoalTitle']}` (`{summary['notes']['entryGoalId']}`)",
    f"- entry progressPercent: `{summary['notes']['entryProgressPercent']}`",
    f"- first PostOnboarding logicalTime: `{summary['notes']['firstPostOnboardingLogicalTime']}`",
    f"- first PostOnboarding goal: `{summary['notes']['firstPostOnboardingGoalId']}`",
    f"- first PostOnboarding progressPercent: `{summary['notes']['firstPostOnboardingProgressPercent']}`",
    f"- pre-pause goal: `{summary['notes']['prePauseGoalTitle']}` (`{summary['notes']['prePauseGoalId']}`)",
    f"- pre-pause progressPercent: `{summary['notes']['prePauseProgressPercent']}`",
    f"- pre-pause feedback: `{summary['notes']['prePauseFeedbackStage']}` / `{summary['notes']['prePauseFeedbackEffect']}`",
    f"- final goal: `{summary['notes']['finalGoalTitle']}` (`{summary['notes']['finalGoalId']}`)",
    f"- final progressPercent: `{summary['notes']['finalProgressPercent']}`",
    f"- final feedback: `{summary['notes']['finalFeedbackStage']}` / `{summary['notes']['finalFeedbackEffect']}`",
    "",
    "## Body Excerpt At PostOnboarding",
    "```text",
    entry_excerpt,
    "```",
    "",
    "## Body Excerpt At End",
    "```text",
    final_excerpt,
    "```",
])
summary_md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

echo "active-llm retention sample complete"
echo "- run id: $run_id"
echo "- url: $game_url"
echo "- artifacts: $out_dir"
echo "- summary json: $summary_json_path"
echo "- summary md: $summary_md_path"
