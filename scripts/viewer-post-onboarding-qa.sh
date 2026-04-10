#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-post-onboarding-qa.sh [options] [run-game-test options...]

Validate the #46 PostOnboarding handoff in a real Web session.

Default flow:
1. bootstrap a Viewer stack via ./scripts/run-game-test.sh
2. open the Viewer in headed agent-browser
3. create one completed world-feedback step
4. select the first agent and capture the PostOnboarding handoff state
5. advance a little further and collect follow-up evidence

Options:
  --url <url>               Reuse an existing Viewer URL instead of bootstrapping a stack
  --out-dir <path>          Artifact root (default: output/playwright/playability)
  --startup-timeout <secs>  Wait timeout for stack URL (default: 240)
  --feedback-timeout-ms <n> Wait timeout for step feedback completion (default: 10000)
  --session <name>          agent-browser session name prefix
  -h, --help                Show this help

Examples:
  ./scripts/viewer-post-onboarding-qa.sh --bundle-dir output/release/game-launcher-local --no-llm
  ./scripts/viewer-post-onboarding-qa.sh --url http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&test_api=1
USAGE
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys, time
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
out_root="output/playwright/playability"
startup_timeout_secs=240
feedback_timeout_ms=10000
session_prefix="post-onboarding-qa"
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

ab_require
require_cmd rg
require_cmd stdbuf

stamp=$(date +%Y%m%d-%H%M%S)
run_id="post-onboarding-${stamp}"
out_dir="$out_root/$run_id"
mkdir -p "$out_dir"

ab_log="$out_dir/agent-browser.log"
run_log="$out_dir/run-game-test.log"
summary_json_path="$out_dir/post-onboarding-summary.json"
summary_md_path="$out_dir/post-onboarding-summary.md"
browser_env_path="$out_dir/browser-env.json"
console_log="$out_dir/console.log"
console_errors_log="$out_dir/console.errors.log"
initial_state_path="$out_dir/state-initial.json"
feedback_state_path="$out_dir/state-feedback.json"
entry_state_path="$out_dir/state-post-onboarding-entry.json"
followup_state_path="$out_dir/state-post-onboarding-followup.json"
shot_initial="$out_dir/00-initial.png"
shot_entry="$out_dir/01-post-onboarding-entry.png"
shot_followup="$out_dir/02-post-onboarding-followup.png"
session="${session_prefix}-${stamp}"

# Ensure the polling loop can read the log file before tee opens it.
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
    stdbuf -oL -eL ./scripts/run-game-test.sh "${stack_args[@]}" > >(tee "$run_log") 2>&1 &
  else
    ./scripts/run-game-test.sh "${stack_args[@]}" > >(tee "$run_log") 2>&1 &
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
state_tick() { json_get "$1" tick; }
state_event_seq() { json_get "$1" eventSeq; }
state_render_mode() { json_get "$1" renderMode; }
state_last_error() { json_get "$1" lastError; }
state_last_feedback_json() { json_get "$1" lastControlFeedback; }
feedback_stage() { json_get "$1" stage; }
feedback_effect() { json_get "$1" effect; }

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
  local timeout_ms=${1:-6000}
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

log_note step_feedback
ab_eval "$session" "window.__AW_TEST__.sendControl('step', {count: 8})" >>"$ab_log" 2>&1
feedback_state=$(wait_for_feedback_stage "completed_advanced" "$feedback_timeout_ms") || {
  echo "error: step(8) did not produce completed_advanced feedback" >&2
  exit 1
}
json_to_file "$feedback_state" "$feedback_state_path"

log_note select_first_agent
ab_eval "$session" "window.__AW_TEST__.runSteps('panel=show;layout=command;select=first_agent;wait=0.6')" >>"$ab_log" 2>&1
entry_state=$(wait_for_selected_agent 8000) || {
  echo "error: failed to select first agent after feedback" >&2
  exit 1
}
json_to_file "$entry_state" "$entry_state_path"
log_note screenshot_entry
ab_screenshot "$session" "$shot_entry" >>"$ab_log" 2>&1

log_note followup_step
ab_eval "$session" "window.__AW_TEST__.sendControl('step', {count: 24})" >>"$ab_log" 2>&1
followup_state=$(wait_for_feedback_stage "completed_advanced" "$feedback_timeout_ms") || {
  echo "error: follow-up step(24) did not produce completed_advanced feedback" >&2
  exit 1
}
json_to_file "$followup_state" "$followup_state_path"
log_note screenshot_followup
ab_screenshot "$session" "$shot_followup" >>"$ab_log" 2>&1

ab_cmd "$session" console >"$console_log" 2>&1 || true
ab_cmd "$session" errors >"$console_errors_log" 2>&1 || true

python3 - \
  "$summary_json_path" \
  "$game_url" \
  "$browser_env_path" \
  "$initial_state_path" \
  "$feedback_state_path" \
  "$entry_state_path" \
  "$followup_state_path" \
  "$shot_initial" \
  "$shot_entry" \
  "$shot_followup" \
  "$console_log" \
  "$console_errors_log" \
  "$stack_logs_dir" <<'PY'
import json
import pathlib
import sys

summary_json_path = pathlib.Path(sys.argv[1])
game_url = sys.argv[2]
browser_env_path = pathlib.Path(sys.argv[3])
initial_state_path = pathlib.Path(sys.argv[4])
feedback_state_path = pathlib.Path(sys.argv[5])
entry_state_path = pathlib.Path(sys.argv[6])
followup_state_path = pathlib.Path(sys.argv[7])
shot_initial = pathlib.Path(sys.argv[8])
shot_entry = pathlib.Path(sys.argv[9])
shot_followup = pathlib.Path(sys.argv[10])
console_log = pathlib.Path(sys.argv[11])
console_errors_log = pathlib.Path(sys.argv[12])
stack_logs_dir = sys.argv[13]

def load_json(path: pathlib.Path):
    return json.loads(path.read_text(encoding="utf-8"))

browser_env = load_json(browser_env_path)
initial_state = load_json(initial_state_path)
feedback_state = load_json(feedback_state_path)
entry_state = load_json(entry_state_path)
followup_state = load_json(followup_state_path)

feedback = feedback_state.get("lastControlFeedback") or {}
followup_feedback = followup_state.get("lastControlFeedback") or {}
renderer = browser_env.get("renderer") or ""
render_mode = (browser_env.get("state") or {}).get("renderMode")

summary = {
    "result": "pass",
    "gameUrl": game_url,
    "stackLogsDir": stack_logs_dir or None,
    "artifacts": {
        "browserEnv": str(browser_env_path),
        "initialState": str(initial_state_path),
        "feedbackState": str(feedback_state_path),
        "postOnboardingEntryState": str(entry_state_path),
        "postOnboardingFollowupState": str(followup_state_path),
        "initialScreenshot": str(shot_initial),
        "postOnboardingEntryScreenshot": str(shot_entry),
        "postOnboardingFollowupScreenshot": str(shot_followup),
        "consoleLog": str(console_log),
        "consoleErrorsLog": str(console_errors_log),
    },
    "checks": {
        "connected": initial_state.get("connectionStatus") == "connected",
        "hardwareRendererOrSafeMode": ("SwiftShader" not in renderer and "Software" not in renderer) or render_mode == "software_safe",
        "feedbackAdvanced": feedback.get("stage") == "completed_advanced",
        "feedbackProducedWorldDelta": (feedback.get("deltaLogicalTime") or 0) > 0 or (feedback.get("deltaEventSeq") or 0) > 0,
        "selectedAgentAfterFeedback": entry_state.get("selectedKind") == "agent" and bool(entry_state.get("selectedId")),
        "followupAdvanced": followup_feedback.get("stage") == "completed_advanced",
        "noRuntimeError": not bool(followup_state.get("lastError")),
    },
    "manualReviewChecklist": [
        "确认 4/4 完成后左侧 Mission HUD 已切换为 PostOnboarding，而不是继续停留在 onboarding。",
        "确认顶部首次总结卡显示已进入下一阶段 / PostOnboarding unlocked 语义。",
        "确认 onboarding 卡与轻提示不再持续占据主视图。",
    ],
    "notes": {
        "initialTick": initial_state.get("tick"),
        "feedbackTick": feedback_state.get("tick"),
        "entryTick": entry_state.get("tick"),
        "followupTick": followup_state.get("tick"),
        "feedbackEffect": feedback.get("effect"),
        "followupEffect": followup_feedback.get("effect"),
        "renderer": renderer,
        "renderMode": render_mode,
    },
}

summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
PY

python3 - "$summary_json_path" "$summary_md_path" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
out = pathlib.Path(sys.argv[2])
checks = summary["checks"]
artifacts = summary["artifacts"]
notes = summary["notes"]

lines = [
    "# Viewer PostOnboarding QA Summary",
    "",
    f"- Result: `{summary['result']}`",
    f"- URL: `{summary['gameUrl']}`",
]
if summary.get("stackLogsDir"):
    lines.append(f"- Stack logs: `{summary['stackLogsDir']}`")
lines.extend(
    [
        f"- Renderer: `{notes['renderer']}`",
        f"- Render mode: `{notes['renderMode']}`",
        "",
        "## Checks",
    ]
)
for key, value in checks.items():
    lines.append(f"- {key}: `{str(value).lower()}`")
lines.extend(
    [
        "",
        "## Manual Review Checklist",
    ]
)
for item in summary["manualReviewChecklist"]:
    lines.append(f"- {item}")
lines.extend(
    [
        "",
        "## Artifacts",
    ]
)
for key, value in artifacts.items():
    lines.append(f"- {key}: `{value}`")
out.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

cat <<INFO
PostOnboarding QA artifacts ready.
- URL: $game_url
- Summary JSON: $summary_json_path
- Summary MD: $summary_md_path
- Initial screenshot: $shot_initial
- Entry screenshot: $shot_entry
- Follow-up screenshot: $shot_followup
- Stack logs: ${stack_logs_dir:-n/a}
INFO
