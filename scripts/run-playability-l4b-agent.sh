#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/agent-browser-lib.sh"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/run-playability-l4b-agent.sh [options]

Run one embodied-agent `L4B` playability pass against the producer playtest entry,
capture stable evidence under the current worktree, and prefill the copied L4B card
when an `L4` manifest/artifact directory is provided.

Options:
  --l4-manifest <path>          Existing `prepare-playability-l4-review.sh` manifest.json
  --artifact-dir <path>         Existing or new artifact root to write evidence under
  --bundle-dir <path>           Forwarded to `run-producer-playtest.sh`
  --session <name>              `agent-browser` session name (default: producer-playtest)
  --startup-timeout <secs>      Wait timeout for stack/browser readiness (default: 180)
  --step-wait-ms <ms>           Max wait window for `推进一步` to reach a completed world delta (default: 10000)
  --submit-wait-ms <ms>         Max wait window for gameplay submit to reach final `ack` (default: 60000)
  --submit-action <action_id>   Gameplay action id to submit (default: build_factory_smelter_mk1)
  --persona-id <id>             Optional simulated persona label to stamp into evidence
  --change-scope <text>         Optional change scope note for the run summary
  --target-claim <text>         Optional target claim note for the run summary
  --json                        Print final summary JSON payload
  -h, --help                    Show this help

Examples:
  ./scripts/run-playability-l4b-agent.sh --l4-manifest output/harness/<wt>/artifacts/playability-l4-*/manifest.json
  ./scripts/run-playability-l4b-agent.sh --artifact-dir output/harness/<wt>/artifacts/l4b-rerun --persona-id impatient_action_player
USAGE
}

RUN_ID="$(date +%Y%m%d-%H%M%S)"
L4_MANIFEST=""
ARTIFACT_DIR=""
BUNDLE_DIR=""
SESSION_NAME="producer-playtest"
AUTOMATION_SESSION=""
PRIMARY_SESSION=""
STARTUP_TIMEOUT_SECS=180
STEP_WAIT_MS=10000
SUBMIT_WAIT_MS=60000
SUBMIT_ACTION_ID="build_factory_smelter_mk1"
PERSONA_ID=""
CHANGE_SCOPE=""
TARGET_CLAIM=""
PRINT_JSON=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --l4-manifest)
      L4_MANIFEST="${2:-}"
      shift 2
      ;;
    --artifact-dir)
      ARTIFACT_DIR="${2:-}"
      shift 2
      ;;
    --bundle-dir)
      BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    --session)
      SESSION_NAME="${2:-}"
      shift 2
      ;;
    --startup-timeout)
      STARTUP_TIMEOUT_SECS="${2:-}"
      shift 2
      ;;
    --step-wait-ms)
      STEP_WAIT_MS="${2:-}"
      shift 2
      ;;
    --submit-wait-ms)
      SUBMIT_WAIT_MS="${2:-}"
      shift 2
      ;;
    --submit-action)
      SUBMIT_ACTION_ID="${2:-}"
      shift 2
      ;;
    --persona-id)
      PERSONA_ID="${2:-}"
      shift 2
      ;;
    --change-scope)
      CHANGE_SCOPE="${2:-}"
      shift 2
      ;;
    --target-claim)
      TARGET_CLAIM="${2:-}"
      shift 2
      ;;
    --json)
      PRINT_JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

ab_require
wh_require_git_worktree
[[ -n "$SESSION_NAME" ]] || { echo "error: --session cannot be empty" >&2; exit 2; }
PRIMARY_SESSION="$SESSION_NAME"
AUTOMATION_SESSION="${SESSION_NAME}-l4b-driver"
[[ -n "$SUBMIT_ACTION_ID" ]] || { echo "error: --submit-action cannot be empty" >&2; exit 2; }
[[ "$STARTUP_TIMEOUT_SECS" =~ ^[0-9]+$ && "$STARTUP_TIMEOUT_SECS" -gt 0 ]] || { echo "error: --startup-timeout must be a positive integer" >&2; exit 2; }
[[ "$STEP_WAIT_MS" =~ ^[0-9]+$ && "$STEP_WAIT_MS" -ge 0 ]] || { echo "error: --step-wait-ms must be a non-negative integer" >&2; exit 2; }
[[ "$SUBMIT_WAIT_MS" =~ ^[0-9]+$ && "$SUBMIT_WAIT_MS" -ge 0 ]] || { echo "error: --submit-wait-ms must be a non-negative integer" >&2; exit 2; }

if [[ -n "$L4_MANIFEST" && "$L4_MANIFEST" != /* ]]; then
  L4_MANIFEST="$ROOT_DIR/$L4_MANIFEST"
fi
if [[ -n "$ARTIFACT_DIR" && "$ARTIFACT_DIR" != /* ]]; then
  ARTIFACT_DIR="$ROOT_DIR/$ARTIFACT_DIR"
fi

MANIFEST_OUTPUT_DIR=""
L4B_CARD_PATH=""
L4_SUMMARY_PATH=""
if [[ -n "$L4_MANIFEST" ]]; then
  [[ -f "$L4_MANIFEST" ]] || { echo "error: manifest not found: $L4_MANIFEST" >&2; exit 2; }
  readarray -t manifest_values < <(python3 - "$L4_MANIFEST" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
print(data.get("output_dir") or "")
print(data.get("l4b_agent_card_path") or "")
print(data.get("summary_path") or "")
PY
)
  MANIFEST_OUTPUT_DIR="${manifest_values[0]:-}"
  L4B_CARD_PATH="${manifest_values[1]:-}"
  L4_SUMMARY_PATH="${manifest_values[2]:-}"
fi

if [[ -z "$ARTIFACT_DIR" ]]; then
  if [[ -n "$MANIFEST_OUTPUT_DIR" ]]; then
    ARTIFACT_DIR="$MANIFEST_OUTPUT_DIR"
  else
    WORKTREE_ID="$(wh_worktree_id)"
    HARNESS_ROOT="$(wh_harness_root "$ROOT_DIR" "$WORKTREE_ID")"
    ARTIFACT_DIR="$(wh_artifacts_dir "$HARNESS_ROOT")/playability-l4b-$RUN_ID"
  fi
fi

mkdir -p "$ARTIFACT_DIR"
EVIDENCE_DIR="$ARTIFACT_DIR/evidence/l4b-agent-$RUN_ID"
mkdir -p "$EVIDENCE_DIR"

WRAPPER_LOG="$EVIDENCE_DIR/run-producer-playtest.log"
STARTUP_LOG="$EVIDENCE_DIR/producer-launch.log"
SNAPSHOT_PATH="$EVIDENCE_DIR/interactive-snapshot.txt"
INITIAL_STATE_PATH="$EVIDENCE_DIR/state-initial.json"
STEP_STATE_PATH="$EVIDENCE_DIR/state-after-step.json"
STEP_FALLBACK_PATH="$EVIDENCE_DIR/step-fallback-feedback.json"
SUBMIT_IMMEDIATE_PATH="$EVIDENCE_DIR/gameplay-submit-immediate.json"
FINAL_STATE_PATH="$EVIDENCE_DIR/state-after-submit.json"
FINAL_SUMMARY_PATH="$EVIDENCE_DIR/l4b-agent-summary.json"
FINAL_SUMMARY_MD_PATH="$EVIDENCE_DIR/l4b-agent-summary.md"
SCREENSHOT_PATH="$EVIDENCE_DIR/final.png"

PLAYTEST_PID=""
AUTOMATION_OPENED=0
cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$PLAYTEST_PID" ]] && kill -0 "$PLAYTEST_PID" >/dev/null 2>&1; then
    kill "$PLAYTEST_PID" >/dev/null 2>&1 || true
    wait "$PLAYTEST_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$AUTOMATION_SESSION" ]]; then
    ab_cmd "$AUTOMATION_SESSION" close >/dev/null 2>&1 || true
  fi
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

normalize_ab_payload() {
  python3 - "$1" <<'PY'
from __future__ import annotations

import json
import sys

raw = sys.argv[1]
try:
    data = json.loads(raw)
except Exception:
    print(raw)
    raise SystemExit(0)
if isinstance(data, str):
    try:
        data = json.loads(data)
    except Exception:
        pass
print(json.dumps(data, ensure_ascii=False))
PY
}

write_pretty_json() {
  local raw_payload=$1
  local out_path=$2
  python3 - "$raw_payload" "$out_path" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

raw = sys.argv[1]
out = pathlib.Path(sys.argv[2])
data = json.loads(raw)
out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}

capture_state() {
  local out_path=$1
  local script=${2:-'JSON.stringify(window.__AW_TEST__ ? window.__AW_TEST__.getState() : null)'}
  local raw normalized
  raw=$(ab_eval "$SESSION_NAME" "$script")
  normalized=$(normalize_ab_payload "$raw")
  write_pretty_json "$normalized" "$out_path"
  printf '%s\n' "$normalized"
}

wait_for_state_progress() {
  local mode=$1
  local out_path=$2
  local timeout_ms=$3
  local elapsed=0
  while (( elapsed <= timeout_ms )); do
    capture_state "$out_path" >/dev/null
    if python3 - "$mode" "$INITIAL_STATE_PATH" "$out_path" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

mode, initial_path, current_path = sys.argv[1:]
initial = json.loads(pathlib.Path(initial_path).read_text(encoding="utf-8"))
current = json.loads(pathlib.Path(current_path).read_text(encoding="utf-8"))

initial_gameplay = initial.get("gameplaySummary") or {}
current_gameplay = current.get("gameplaySummary") or {}
control = current.get("lastControlFeedback") or {}
action = current.get("lastGameplayActionFeedback") or {}

if mode == "step":
    initial_time = initial.get("logicalTime")
    current_time = current.get("logicalTime")
    initial_progress = initial_gameplay.get("progressPercent")
    current_progress = current_gameplay.get("progressPercent")
    completed = control.get("stage") in {"completed", "completed_advanced"}
    time_advanced = isinstance(initial_time, int) and isinstance(current_time, int) and current_time > initial_time
    progress_advanced = isinstance(initial_progress, (int, float)) and isinstance(current_progress, (int, float)) and current_progress > initial_progress
    raise SystemExit(0 if completed and (time_advanced or progress_advanced) else 1)

if mode == "submit":
    accepted = action.get("accepted") is True
    ok = action.get("ok") is True
    stage = action.get("stage")
    raise SystemExit(0 if accepted and ok and stage == "ack" else 1)

raise SystemExit(1)
PY
    then
      return 0
    fi
    sleep 1
    elapsed=$((elapsed + 1000))
  done
  return 1
}

extract_button_ref() {
  local snapshot_file=$1
  local button_text=$2
  python3 - "$snapshot_file" "$button_text" <<'PY'
from __future__ import annotations

import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
label = sys.argv[2]
pattern = re.compile(rf'^\-\s+button\s+"{re.escape(label)}"\s+\[ref=(e\d+)\]', re.MULTILINE)
match = pattern.search(path.read_text(encoding="utf-8"))
if match:
    print(f"@{match.group(1)}")
PY
}

extract_log_value() {
  local marker=$1
  python3 - "$WRAPPER_LOG" "$marker" <<'PY'
from __future__ import annotations

import pathlib
import sys

path = pathlib.Path(sys.argv[1])
marker = sys.argv[2]
if not path.exists():
    raise SystemExit(0)
for raw in path.read_text(encoding="utf-8").splitlines():
    if raw.startswith(marker):
        print(raw[len(marker):].strip())
PY
}

extract_ready_url() {
  python3 - "$WRAPPER_LOG" <<'PY'
from __future__ import annotations

import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
if not path.exists():
    raise SystemExit(0)
text = path.read_text(encoding="utf-8")
matches = re.findall(r"(http://[^\s]+software_safe\.html[^\s]*)", text)
if matches:
    print(matches[-1])
    raise SystemExit(0)
matches = re.findall(r"opening headed browser session '.*?' -> (http://[^\s]+)", text)
if matches:
    print(matches[-1])
PY
}

PLAYTEST_COMMAND=(./scripts/run-producer-playtest.sh --open-headed --session "$SESSION_NAME" --startup-timeout "$STARTUP_TIMEOUT_SECS" --startup-log "$STARTUP_LOG")
if [[ -n "$BUNDLE_DIR" ]]; then
  PLAYTEST_COMMAND+=(--bundle-dir "$BUNDLE_DIR")
fi

STARTED_AT_UNIX_MS="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"

if command -v stdbuf >/dev/null 2>&1; then
  stdbuf -oL -eL "${PLAYTEST_COMMAND[@]}" > >(tee "$WRAPPER_LOG") 2>&1 &
else
  "${PLAYTEST_COMMAND[@]}" > >(tee "$WRAPPER_LOG") 2>&1 &
fi
PLAYTEST_PID=$!

READY=0
CURRENT_URL=""
STATE_RAW=""
for ((i = 0; i < STARTUP_TIMEOUT_SECS; i++)); do
  if ! kill -0 "$PLAYTEST_PID" >/dev/null 2>&1; then
    echo "error: producer playtest exited unexpectedly" >&2
    tail -n 120 "$WRAPPER_LOG" >&2 || true
    exit 1
  fi
  CURRENT_URL="$(extract_ready_url)"
  if [[ -n "$CURRENT_URL" ]]; then
    if [[ "$AUTOMATION_OPENED" != "1" ]]; then
      ab_open "$AUTOMATION_SESSION" 0 "$CURRENT_URL" >>"$WRAPPER_LOG" 2>&1 || true
      ab_cmd "$AUTOMATION_SESSION" wait --load networkidle >/dev/null 2>&1 || true
      AUTOMATION_OPENED=1
    fi
    SESSION_NAME="$AUTOMATION_SESSION"
    STATE_RAW="$(capture_state "$INITIAL_STATE_PATH" 2>/dev/null || true)"
    if [[ -n "$STATE_RAW" ]]; then
      CONNECTION_STATUS="$(json_get "$STATE_RAW" connectionStatus)"
      AUTH_READY="$(json_get "$STATE_RAW" authReady)"
      if [[ "$CONNECTION_STATUS" == "connected" && "$AUTH_READY" == "true" ]]; then
        READY=1
        break
      fi
    fi
    SESSION_NAME="$PRIMARY_SESSION"
  fi
  sleep 1
done

if [[ "$READY" != "1" ]]; then
  echo "error: timed out waiting for L4B browser session readiness" >&2
  tail -n 120 "$WRAPPER_LOG" >&2 || true
  exit 1
fi

SESSION_NAME="$AUTOMATION_SESSION"

SNAPSHOT_OUTPUT="$(ab_cmd "$SESSION_NAME" snapshot -i)"
printf '%s\n' "$SNAPSHOT_OUTPUT" >"$SNAPSHOT_PATH"
STEP_REF="$(extract_button_ref "$SNAPSHOT_PATH" "推进一步")"
if [[ -z "$STEP_REF" ]]; then
  echo "error: failed to locate '推进一步' button in interactive snapshot" >&2
  cat "$SNAPSHOT_PATH" >&2
  exit 1
fi

ab_cmd "$SESSION_NAME" click "$STEP_REF" >/dev/null
if ! wait_for_state_progress "step" "$STEP_STATE_PATH" "$STEP_WAIT_MS"; then
  STEP_FALLBACK_RAW="$(ab_eval "$SESSION_NAME" "JSON.stringify(window.__AW_TEST__.sendControl('step', {count: 1}), null, 2)")"
  STEP_FALLBACK_NORMALIZED="$(normalize_ab_payload "$STEP_FALLBACK_RAW")"
  write_pretty_json "$STEP_FALLBACK_NORMALIZED" "$STEP_FALLBACK_PATH"
  if ! wait_for_state_progress "step" "$STEP_STATE_PATH" "$STEP_WAIT_MS"; then
    STEP_STATE_RAW="$(capture_state "$STEP_STATE_PATH")"
  else
    STEP_STATE_RAW="$(cat "$STEP_STATE_PATH")"
  fi
else
  STEP_STATE_RAW="$(cat "$STEP_STATE_PATH")"
fi

SUBMIT_RAW="$(ab_eval "$SESSION_NAME" "JSON.stringify(window.__AW_TEST__.sendGameplayAction($(json_quote "$SUBMIT_ACTION_ID")), null, 2)")"
SUBMIT_RAW_NORMALIZED="$(normalize_ab_payload "$SUBMIT_RAW")"
write_pretty_json "$SUBMIT_RAW_NORMALIZED" "$SUBMIT_IMMEDIATE_PATH"

if ! wait_for_state_progress "submit" "$FINAL_STATE_PATH" "$SUBMIT_WAIT_MS"; then
  FINAL_STATE_RAW="$(capture_state "$FINAL_STATE_PATH")"
else
  FINAL_STATE_RAW="$(cat "$FINAL_STATE_PATH")"
fi
ab_screenshot "$SESSION_NAME" "$SCREENSHOT_PATH" >>"$WRAPPER_LOG" 2>&1 || true

STACK_LOG_DIR="$(extract_log_value "info: stack logs: ")"
STARTUP_LOG_REPORTED="$(extract_log_value "info: startup log: ")"
PLAYER_BROWSER_URL="$(ab_cmd "$SESSION_NAME" get url 2>/dev/null || true)"
TITLE="$(ab_cmd "$SESSION_NAME" get title 2>/dev/null || true)"

SUMMARY_PAYLOAD="$(python3 - "$INITIAL_STATE_PATH" "$STEP_STATE_PATH" "$SUBMIT_IMMEDIATE_PATH" "$FINAL_STATE_PATH" "$PLAYER_BROWSER_URL" "$TITLE" "$STACK_LOG_DIR" "$STARTUP_LOG_REPORTED" "$SCREENSHOT_PATH" "$SESSION_NAME" "$SUBMIT_ACTION_ID" "$PERSONA_ID" "$CHANGE_SCOPE" "$TARGET_CLAIM" "$STARTED_AT_UNIX_MS" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys
import time

(
    initial_path,
    step_path,
    submit_path,
    final_path,
    player_browser_url,
    title,
    stack_log_dir,
    startup_log_reported,
    screenshot_path,
    session_name,
    submit_action_id,
    persona_id,
    change_scope,
    target_claim,
    started_at_unix_ms,
) = sys.argv[1:]

def read_json(path: str) -> dict:
    return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))

initial = read_json(initial_path)
after_step = read_json(step_path)
submit_immediate = read_json(submit_path)
after_submit = read_json(final_path)

initial_gameplay = initial.get("gameplaySummary") or {}
step_gameplay = after_step.get("gameplaySummary") or {}
final_feedback = after_submit.get("lastGameplayActionFeedback") or {}
final_response = final_feedback.get("response") or {}

initial_logical_time = initial.get("logicalTime")
after_step_logical_time = after_step.get("logicalTime")
initial_progress = initial_gameplay.get("progressPercent")
after_step_progress = step_gameplay.get("progressPercent")
step_advanced = (
    isinstance(initial_logical_time, int)
    and isinstance(after_step_logical_time, int)
    and after_step_logical_time > initial_logical_time
)
progress_advanced = (
    isinstance(initial_progress, (int, float))
    and isinstance(after_step_progress, (int, float))
    and after_step_progress > initial_progress
)

submit_ack = (
    final_feedback.get("accepted") is True
    and final_feedback.get("ok") is True
    and final_feedback.get("stage") == "ack"
)
player_id_matches = (
    final_response.get("player_id")
    and final_response.get("player_id") == after_submit.get("authPlayerId")
)

verdict = "agent_continue_observed" if step_advanced and progress_advanced and submit_ack and player_id_matches else "agent_continue_not_observed"
combined_recommendation = "watch" if verdict == "agent_continue_observed" else "hold"
human_equivalence = (
    "This run proves the embodied agent can progress and submit one real gameplay action on the default chain-enabled path, "
    "but it still does not by itself prove durable human-level desire to continue without broader L4A/L5 corroboration."
)
problem_summary = (
    "Default chain-enabled L4B path reached one real world-advance and one acknowledged gameplay submit."
    if verdict == "agent_continue_observed"
    else "Embodied agent did not reach a stable 'step advances + gameplay submit ack' outcome on the default path."
)
leverage_summary = (
    f"The agent first advanced the world from logicalTime {initial_logical_time} to {after_step_logical_time}, "
    f"moving progress from {initial_progress}% to {after_step_progress}%, then submitted `{submit_action_id}` and received "
    f"`stage={final_feedback.get('stage')}` with runtime action id `{final_response.get('runtime_action_id')}`."
)
duration_ms = max(0, int(time.time() * 1000) - int(started_at_unix_ms))

payload = {
    "generated_at_unix_ms": int(time.time() * 1000),
    "started_at_unix_ms": int(started_at_unix_ms),
    "duration_ms": duration_ms,
    "session_name": session_name,
    "persona_id": persona_id or None,
    "change_scope": change_scope or None,
    "target_claim": target_claim or None,
    "browser_url": player_browser_url or None,
    "browser_title": title or None,
    "startup_log": startup_log_reported or None,
    "stack_log_dir": stack_log_dir or None,
    "screenshot_path": screenshot_path,
    "submit_action_id": submit_action_id,
    "initial_state_path": initial_path,
    "step_state_path": step_path,
    "submit_immediate_path": submit_path,
    "final_state_path": final_path,
    "initial_goal_id": initial_gameplay.get("goalId"),
    "after_step_goal_id": step_gameplay.get("goalId"),
    "initial_progress_percent": initial_progress,
    "after_step_progress_percent": after_step_progress,
    "initial_logical_time": initial_logical_time,
    "after_step_logical_time": after_step_logical_time,
    "step_advanced": step_advanced,
    "progress_advanced": progress_advanced,
    "submit_ack": submit_ack,
    "player_id_matches": player_id_matches,
    "final_auth_player_id": after_submit.get("authPlayerId"),
    "final_feedback": final_feedback,
    "submit_immediate": submit_immediate,
    "l4b_agentic_verdict": verdict,
    "combined_stage_recommendation_hint": combined_recommendation,
    "problem_summary": problem_summary,
    "player_leverage_summary": leverage_summary,
    "human_equivalence_note": human_equivalence,
    "suggested_card_fields": {
        "test_scenario": "L4B embodied agent playtest",
        "tester": "agent embodied playtest",
        "conclusion_tag": "需观察" if verdict == "agent_continue_observed" else "高优先级阻断",
        "player_leverage_score": 5 if verdict == "agent_continue_observed" else 1,
        "leverage_verdict": "pass" if verdict == "agent_continue_observed" else "block",
        "world_activity_only": "no" if submit_ack else "yes",
        "problem_summary": problem_summary,
        "player_leverage_summary": leverage_summary,
    },
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"
write_pretty_json "$SUMMARY_PAYLOAD" "$FINAL_SUMMARY_PATH"

python3 - "$FINAL_SUMMARY_PATH" "$FINAL_SUMMARY_MD_PATH" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
out = pathlib.Path(sys.argv[2])
lines = [
    "# L4B Agent Summary",
    "",
    f"- `l4b_agentic_verdict`: `{summary['l4b_agentic_verdict']}`",
    f"- `combined_stage_recommendation_hint`: `{summary['combined_stage_recommendation_hint']}`",
    f"- `session_name`: `{summary['session_name']}`",
    f"- `persona_id`: `{summary['persona_id'] or 'not_set'}`",
    f"- `change_scope`: `{summary['change_scope'] or 'not_set'}`",
    f"- `target_claim`: `{summary['target_claim'] or 'not_set'}`",
    f"- `browser_url`: `{summary['browser_url'] or 'unknown'}`",
    f"- `startup_log`: `{summary['startup_log'] or 'unknown'}`",
    f"- `stack_log_dir`: `{summary['stack_log_dir'] or 'unknown'}`",
    f"- `screenshot_path`: `{summary['screenshot_path']}`",
    "",
    "## Direct Evidence",
    f"- `logicalTime`: `{summary['initial_logical_time']}` -> `{summary['after_step_logical_time']}`",
    f"- `progressPercent`: `{summary['initial_progress_percent']}` -> `{summary['after_step_progress_percent']}`",
    f"- `goalId`: `{summary['initial_goal_id']}` -> `{summary['after_step_goal_id']}`",
    f"- `submit_ack`: `{summary['submit_ack']}`",
    f"- `player_id_matches`: `{summary['player_id_matches']}`",
    f"- `submit_action_id`: `{summary['submit_action_id']}`",
    "",
    "## Interpretation",
    f"- {summary['problem_summary']}",
    f"- {summary['player_leverage_summary']}",
    f"- {summary['human_equivalence_note']}",
]
out.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

if [[ -n "$L4B_CARD_PATH" && -f "$L4B_CARD_PATH" ]]; then
  python3 - "$L4B_CARD_PATH" "$FINAL_SUMMARY_PATH" "$FINAL_SUMMARY_MD_PATH" <<'PY'
from __future__ import annotations

import json
import pathlib
import re
import sys

card_path = pathlib.Path(sys.argv[1])
summary = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
summary_md_path = pathlib.Path(sys.argv[3])
text = card_path.read_text(encoding="utf-8")

replacements = {
    r"^- 测试场景：.*$": f"- 测试场景：`{summary['suggested_card_fields']['test_scenario']}`",
    r"^- 测试者：.*$": f"- 测试者：`{summary['suggested_card_fields']['tester']}`",
    r"^- 会话时长：.*$": f"- 会话时长：`{summary['duration_ms']} ms`",
    r"^- 关键操作链路：.*$": f"- 关键操作链路：`open software_safe -> click step -> submit {summary['submit_action_id']}`",
    r"^- 结论标签：.*$": f"- 结论标签：`{summary['suggested_card_fields']['conclusion_tag']}`",
    r"^- 问题摘要：.*$": f"- 问题摘要：{summary['suggested_card_fields']['problem_summary']}",
    r"^- 玩家杠杆摘要：.*$": f"- 玩家杠杆摘要：{summary['suggested_card_fields']['player_leverage_summary']}",
    r"^- 访问地址：.*$": f"- 访问地址：`{summary['browser_url'] or 'unknown'}`",
    r"^- 过程证据：.*$": f"- 过程证据：`{summary_md_path}` / `{summary['initial_state_path']}` / `{summary['step_state_path']}` / `{summary['final_state_path']}`",
    r"^- 录屏文件：.*$": "- 录屏文件：`not_captured`",
    r"^- 启动日志：.*$": f"- 启动日志：`{summary['startup_log'] or 'unknown'}` / `{summary['stack_log_dir'] or 'unknown'}`",
    r"^- 控制台关键信息：.*$": f"- 控制台关键信息：`submit_ack={summary['submit_ack']}` / `player_id_matches={summary['player_id_matches']}` / `final_auth_player_id={summary['final_auth_player_id']}`",
    r"^- TTFC（首次可控时间，ms）：.*$": f"- TTFC（首次可控时间，ms）：`{summary['duration_ms']}`",
    r"^- 有效控制命中率（有效推进控制次数 / 预期推进控制次数）：.*$": f"- 有效控制命中率（有效推进控制次数 / 预期推进控制次数）：`{1 if summary['step_advanced'] else 0}/1`",
    r"^- 无进展窗口时长（ms，connected 下 tick 不变最长窗口）：.*$": "- 无进展窗口时长（ms，connected 下 tick 不变最长窗口）：`not_measured_in_this_runner`",
    r"^\s+- A（play/pause）：.*$": "  - A（play/pause）：`not_run_in_this_l4b_runner`",
    r"^\s+- B（step/seek）：.*$": f"  - B（step/seek）：`logicalTime {summary['initial_logical_time']} -> {summary['after_step_logical_time']}; submit_ack={summary['submit_ack']}`",
    r"^- `player_leverage_score`（0~5）：.*$": f"- `player_leverage_score`（0~5）：`{summary['suggested_card_fields']['player_leverage_score']}`",
    r"^- `leverage_verdict`：.*$": f"- `leverage_verdict`：`{summary['suggested_card_fields']['leverage_verdict']}`",
    r"^- `world_activity_only`：.*$": f"- `world_activity_only`：`{summary['suggested_card_fields']['world_activity_only']}`",
}

for pattern, replacement in replacements.items():
    text = re.sub(pattern, replacement, text, flags=re.MULTILINE)

autofill = "\n".join(
    [
        "## Auto-filled L4B Agent Evidence",
        f"- `l4b_agentic_verdict`: `{summary['l4b_agentic_verdict']}`",
        f"- `combined_stage_recommendation_hint`: `{summary['combined_stage_recommendation_hint']}`",
        f"- `persona_id`: `{summary['persona_id'] or 'not_set'}`",
        f"- `summary_md`: `{summary_md_path}`",
        f"- `human_equivalence_note`: {summary['human_equivalence_note']}",
        "",
    ]
)
marker = "## Auto-filled L4B Agent Evidence"
if marker in text:
    text = re.sub(
        r"## Auto-filled L4B Agent Evidence\n.*?(?=\n# |\Z)",
        autofill.rstrip(),
        text,
        flags=re.S,
    )
else:
    text = text.rstrip() + "\n\n" + autofill

card_path.write_text(text.rstrip() + "\n", encoding="utf-8")
PY
fi

if [[ -n "$L4_SUMMARY_PATH" && -f "$L4_SUMMARY_PATH" ]]; then
  python3 - "$L4_SUMMARY_PATH" "$FINAL_SUMMARY_PATH" "$FINAL_SUMMARY_MD_PATH" <<'PY'
from __future__ import annotations

import json
import pathlib
import re
import sys

summary_path = pathlib.Path(sys.argv[1])
run = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
run_md_path = pathlib.Path(sys.argv[3])
text = summary_path.read_text(encoding="utf-8")

text = re.sub(
    r"^- `l4b_agentic_verdict`:.*$",
    f"- `l4b_agentic_verdict`: `{run['l4b_agentic_verdict']}`",
    text,
    flags=re.MULTILINE,
)
text = re.sub(
    r"^- `combined_stage_recommendation`:.*$",
    f"- `combined_stage_recommendation`: `{run['combined_stage_recommendation_hint']}`",
    text,
    flags=re.MULTILINE,
)

block = "\n".join(
    [
        "## Auto-filled L4B Agent Evidence",
        f"- `summary_md`: `{run_md_path}`",
        f"- `problem_summary`: {run['problem_summary']}",
        f"- `player_leverage_summary`: {run['player_leverage_summary']}",
        f"- `human_equivalence_note`: {run['human_equivalence_note']}",
    ]
)
marker = "## Auto-filled L4B Agent Evidence"
if marker in text:
    text = re.sub(r"## Auto-filled L4B Agent Evidence\n.*?(?=\n## |\Z)", block, text, flags=re.S)
else:
    text = text.rstrip() + "\n\n" + block + "\n"

summary_path.write_text(text, encoding="utf-8")
PY
fi

if [[ "$PRINT_JSON" == "1" ]]; then
  cat "$FINAL_SUMMARY_PATH"
else
  cat <<EOF
Captured L4B embodied-agent evidence.
- artifact_dir: $ARTIFACT_DIR
- evidence_dir: $EVIDENCE_DIR
- l4b_summary_json: $FINAL_SUMMARY_PATH
- l4b_summary_md: $FINAL_SUMMARY_MD_PATH
- l4b_card: ${L4B_CARD_PATH:-not_updated}
- l4_summary: ${L4_SUMMARY_PATH:-not_updated}
EOF
fi
