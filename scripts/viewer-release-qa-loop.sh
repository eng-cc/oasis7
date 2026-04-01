#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-release-qa-loop.sh [options]

Status:
  hold-only 3D visual QA helper. Under PRD-WORLD_SIMULATOR-041 it is no longer the
  default active release gate; use Web/non-3D and software_safe closure first.

Options:
  --scenario <name>          oasis7_game_launcher scenario (default: llm_bootstrap)
  --live-bind <host:port>    live tcp bind (default: 127.0.0.1:5023)
  --web-bind <host:port>     web bridge bind (default: 127.0.0.1:5011)
  --viewer-host <host>       web viewer host (default: 127.0.0.1)
  --viewer-port <port>       web viewer port (default: 4173)
  --viewer-static-dir <dir>  viewer static asset dir passed to oasis7_game_launcher (default: web)
  --out-dir <path>           artifact output dir (default: output/playwright/viewer)
  --with-consensus-gate      deprecated no-op (viewer/node split removed this behavior)
  --skip-visual-baseline     skip scripts/viewer-visual-baseline.sh
  --headed                   open browser in headed mode
  -h, --help                 show this help

Artifacts:
  <out-dir>/release-qa-*.log
  <out-dir>/release-qa-*.png
  <out-dir>/release-qa-summary-*.md
USAGE
}

wait_for_port() {
  local host=$1
  local port=$2
  local timeout_secs=$3
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if nc -z "$host" "$port" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_http() {
  local url=$1
  local timeout_secs=$2
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if curl -sf "$url" >/dev/null; then
      return 0
    fi
    sleep 1
  done
  return 1
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys, time
ms = int(sys.argv[1])
time.sleep(ms / 1000.0)
PY
}

log_note() {
  printf '### [%s] %s\n' "$1" "$(date '+%H:%M:%S')" | tee -a "$pw_log" >/dev/null
}

ab_state() {
  ab_eval "$session" 'window.__AW_TEST__?.getState?.() ?? null'
}

ab_send_control() {
  local action=$1
  local payload_json=${2:-null}
  local action_json
  action_json=$(json_quote "$action")
  ab_eval "$session" "(() => { try { return window.__AW_TEST__?.sendControl?.(${action_json}, ${payload_json}) ?? null; } catch (err) { return { accepted: false, reason: String(err), effect: 'exception on sendControl' }; } })()"
}

ab_run_steps() {
  local steps=$1
  local steps_json
  steps_json=$(json_quote "$steps")
  ab_eval "$session" "(() => { try { return window.__AW_TEST__?.runSteps?.(${steps_json}) ?? null; } catch (err) { return { ok: false, reason: String(err) }; } })()"
}

state_tick() {
  json_get "$1" tick
}

state_event_seq() {
  json_get "$1" eventSeq
}

state_connection() {
  json_get "$1" connectionStatus
}

state_last_error() {
  json_get "$1" lastError
}

state_selected_kind() {
  json_get "$1" selectedKind
}

state_camera_mode() {
  json_get "$1" cameraMode
}

state_camera_radius() {
  json_get "$1" cameraRadius
}

state_render_mode() {
  json_get "$1" renderMode
}

state_software_safe_reason() {
  json_get "$1" softwareSafeReason
}

normalize_eval_token() {
  local raw=${1:-}
  raw=$(printf '%s' "$raw" | tr -d '\r\n')
  raw=${raw#\"}
  raw=${raw%\"}
  printf '%s' "$raw"
}

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
  local timeout_ms=${1:-15000}
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

wait_for_tick_advance() {
  local baseline_tick=$1
  local timeout_ms=${2:-6000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  local tick=0
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    tick=$(state_tick "$state")
    tick=${tick:-0}
    if [[ "$(state_connection "$state")" == "connected" ]] && (( ${tick%%.*} > baseline_tick )); then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

wait_for_selected_kind() {
  local expected_kind=$1
  local timeout_ms=${2:-6000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    if [[ "$(state_selected_kind "$state")" == "$expected_kind" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

scenario="llm_bootstrap"
live_bind="127.0.0.1:5023"
web_bind="127.0.0.1:5011"
viewer_host="127.0.0.1"
viewer_port="4173"
viewer_static_dir="web"
out_dir="output/playwright/viewer"
with_consensus_gate=0
skip_visual_baseline=0
headed=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)
      scenario=${2:-}
      shift 2
      ;;
    --live-bind)
      live_bind=${2:-}
      shift 2
      ;;
    --web-bind)
      web_bind=${2:-}
      shift 2
      ;;
    --viewer-host)
      viewer_host=${2:-}
      shift 2
      ;;
    --viewer-port)
      viewer_port=${2:-}
      shift 2
      ;;
    --viewer-static-dir)
      viewer_static_dir=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --with-consensus-gate)
      with_consensus_gate=1
      shift
      ;;
    --skip-visual-baseline)
      skip_visual_baseline=1
      shift
      ;;
    --headed)
      headed=1
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

[[ "$viewer_port" =~ ^[0-9]+$ ]] || { echo "error: --viewer-port must be an integer" >&2; exit 2; }
[[ -n "${viewer_static_dir// }" ]] || { echo "error: --viewer-static-dir must be a non-empty path" >&2; exit 2; }
[[ "$live_bind" == *:* ]] || { echo "error: --live-bind must be host:port" >&2; exit 2; }
[[ "$web_bind" == *:* ]] || { echo "error: --web-bind must be host:port" >&2; exit 2; }

require_cmd nc
require_cmd curl
require_cmd cargo
ab_require

mkdir -p "$out_dir"
stamp=$(date +%Y%m%d-%H%M%S)
session="viewer-release-qa-$stamp"
live_log="$out_dir/release-qa-live-${stamp}.log"
web_log="$out_dir/release-qa-web-${stamp}.log"
pw_log="$out_dir/release-qa-agent-browser-${stamp}.log"
semantic_log="$out_dir/release-qa-semantic-${stamp}.json"
zoom_log="$out_dir/release-qa-zoom-${stamp}.json"
console_log="$out_dir/console.log"
console_errors_log="$out_dir/console.errors.log"
summary_path="$out_dir/release-qa-summary-${stamp}.md"
shot_path="$out_dir/release-qa-${stamp}.png"
zoom_shot_near="$out_dir/release-qa-zoom-near-${stamp}.png"
zoom_shot_mid="$out_dir/release-qa-zoom-mid-${stamp}.png"
zoom_shot_far="$out_dir/release-qa-zoom-far-${stamp}.png"

live_pid=""

cleanup() {
  set +e
  if [[ -n "$live_pid" ]]; then
    kill "$live_pid" >/dev/null 2>&1 || true
    wait "$live_pid" >/dev/null 2>&1 || true
  fi
  ab_cmd "$session" close >/dev/null 2>&1 || true
}
trap cleanup EXIT

if [[ "$skip_visual_baseline" -eq 0 ]]; then
  echo "+ ./scripts/viewer-visual-baseline.sh"
  ./scripts/viewer-visual-baseline.sh
fi

resolved_viewer_static_dir=$(resolve_viewer_static_dir_for_web_closure \
  "$repo_root" \
  "$viewer_static_dir" \
  "$out_dir")
viewer_static_dir="$resolved_viewer_static_dir"

live_host=${live_bind%:*}
live_port=${live_bind##*:}
web_host=${web_bind%:*}
web_port=${web_bind##*:}
viewer_url="http://${viewer_host}:${viewer_port}/?ws=ws://${web_bind}&test_api=1"

if [[ "$with_consensus_gate" -eq 1 ]]; then
  echo "warning: --with-consensus-gate is deprecated and ignored after viewer/node hard split" >&2
fi

live_args=(
  "--scenario" "$scenario"
  "--live-bind" "$live_bind"
  "--web-bind" "$web_bind"
  "--viewer-host" "$viewer_host"
  "--viewer-port" "$viewer_port"
  "--viewer-static-dir" "$viewer_static_dir"
  "--no-open-browser"
)

echo "+ env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live --bin oasis7_chain_runtime"
env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live --bin oasis7_chain_runtime >>"$live_log" 2>&1

echo "+ env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- ${live_args[*]}"
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- "${live_args[@]}" >"$live_log" 2>&1 &
live_pid=$!

echo "+ wait for bridge $web_host:$web_port"
wait_for_port "$web_host" "$web_port" 180 || { echo "error: web bridge did not come up on $web_host:$web_port" >&2; exit 1; }

cat <<'INFO' >"$web_log"
run-viewer-web.sh no longer runs as a standalone process in this QA loop.
web viewer is served by oasis7_game_launcher built-in static server.
INFO

echo "+ wait for viewer $viewer_url"
wait_for_http "http://${viewer_host}:${viewer_port}/" 240 || { echo "error: viewer web server did not become ready: $viewer_host:$viewer_port" >&2; exit 1; }

log_note open
ab_open "$session" "$headed" "$viewer_url" 2>&1 | tee -a "$pw_log" >/dev/null
log_note wait_network
ab_cmd "$session" wait --load networkidle 2>&1 | tee -a "$pw_log" >/dev/null || true
log_note snapshot
ab_cmd "$session" snapshot -i 2>&1 | tee -a "$pw_log" >/dev/null || true

if ! wait_for_api 60000 >/dev/null; then
  ab_cmd "$session" console >"$console_log" 2>&1 || true
  ab_cmd "$session" errors >"$console_errors_log" 2>&1 || true
  echo "error: __AW_TEST__ is unavailable (see $console_log, $console_errors_log)" >&2
  exit 1
fi

initial_state=$(wait_for_connected 30000) || {
  echo "error: initial connection failed (status=$(state_connection "$initial_state"), lastError=$(state_last_error "$initial_state"))" >&2
  exit 1
}

initial_tick=$(state_tick "$initial_state")
initial_tick=${initial_tick:-0}
render_mode=$(state_render_mode "$initial_state")
software_safe_reason=$(state_software_safe_reason "$initial_state")
software_safe_mode=0
if [[ "$render_mode" == "software_safe" ]]; then
  software_safe_mode=1
fi
control_before="$initial_state"
after_play='null'
paused_state='null'
paused_followup='null'
selected_state='null'
final_state='null'
semantic_ok=1

log_note send_play
ab_send_control play '{}' 2>&1 | tee -a "$pw_log" >/dev/null || true
if ! after_play=$(wait_for_tick_advance "${initial_tick%%.*}" 3500); then
  seek_target=$(( ${initial_tick%%.*} + 1 ))
  log_note seek_fallback
  ab_send_control seek "{\"tick\":${seek_target}}" 2>&1 | tee -a "$pw_log" >/dev/null || true
  if ! after_play=$(wait_for_tick_advance "${initial_tick%%.*}" 6000); then
    semantic_ok=0
  fi
fi

log_note send_pause
ab_send_control pause '{}' 2>&1 | tee -a "$pw_log" >/dev/null || true
if ! paused_state=$(wait_for_connected 5000); then
  semantic_ok=0
fi
sleep_ms 600
paused_followup=$(ab_state)
paused_tick=$(state_tick "$paused_state")
paused_tick=${paused_tick:-0}
paused_followup_tick=$(state_tick "$paused_followup")
paused_followup_tick=${paused_followup_tick:-0}
if [[ "$(state_connection "$paused_followup")" != "connected" ]]; then
  semantic_ok=0
fi
if (( ${paused_followup_tick%%.*} > ${paused_tick%%.*} + 2 )); then
  semantic_ok=0
fi

log_note run_steps
if [[ "$software_safe_mode" -eq 1 ]]; then
  ab_run_steps '4' 2>&1 | tee -a "$pw_log" >/dev/null || true
else
  ab_run_steps 'mode=3d;focus=first_location;zoom=0.85;select=first_agent;wait=0.3' 2>&1 | tee -a "$pw_log" >/dev/null || true
fi
if ! selected_state=$(wait_for_selected_kind agent 6000); then
  semantic_ok=0
fi
if [[ "$software_safe_mode" -eq 1 ]]; then
  paused_followup_tick=$(state_tick "$paused_followup")
  paused_followup_tick=${paused_followup_tick:-0}
  if ! final_state=$(wait_for_tick_advance "${paused_followup_tick%%.*}" 6000); then
    semantic_ok=0
  fi
else
  if ! final_state=$(wait_for_connected 6000); then
    semantic_ok=0
  fi
fi
if [[ -n "$(state_last_error "$final_state")" ]]; then
  semantic_ok=0
fi

python3 - "$initial_state" "$control_before" "$after_play" "$paused_state" "$paused_followup" "$selected_state" "$final_state" "$semantic_log" <<'PY'
import json, pathlib, sys

def load(raw):
    try:
        return json.loads(raw)
    except Exception:
        return raw
out = pathlib.Path(sys.argv[8])
data = {
    "initial": load(sys.argv[1]),
    "controlBefore": load(sys.argv[2]),
    "afterPlay": load(sys.argv[3]),
    "paused": load(sys.argv[4]),
    "pausedFollowup": load(sys.argv[5]),
    "selected": load(sys.argv[6]),
    "final": load(sys.argv[7]),
}
out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

zoom_ok=1
zoom_status="passed"
zoom_results='[]'
if [[ "$software_safe_mode" -eq 1 ]]; then
  zoom_status="skipped (software_safe: ${software_safe_reason:-unknown})"
else
  for stage in near mid far; do
    case "$stage" in
      near)
        steps='mode=3d;focus=first_location;zoom=0.65;wait=0.3'
        shot="$zoom_shot_near"
        expect='decrease'
        ;;
      mid)
        steps='mode=3d;focus=first_location;zoom=0.85;wait=0.3'
        shot="$zoom_shot_mid"
        expect='baseline'
        ;;
      far)
        steps='mode=3d;focus=first_location;zoom=1.25;wait=0.3'
        shot="$zoom_shot_far"
        expect='increase'
        ;;
    esac

    before_state=$(ab_state)
    before_radius=$(state_camera_radius "$before_state")
    before_radius=${before_radius:-0}
    log_note "zoom_${stage}"
    ab_run_steps "$steps" 2>&1 | tee -a "$pw_log" >/dev/null || true
    sleep_ms 700
    stage_state=$(wait_for_connected 5000 || true)
    if [[ -z "$stage_state" ]]; then
      stage_state=$(ab_state)
      zoom_ok=0
    fi
    camera_mode=$(state_camera_mode "$stage_state")
    camera_radius=$(state_camera_radius "$stage_state")
    camera_radius=${camera_radius:-0}
    if [[ "$camera_mode" != "3d" ]]; then
      zoom_ok=0
    fi
    python3 - "$before_radius" "$camera_radius" "$expect" <<'PY' || zoom_ok=0
import sys
before = float(sys.argv[1] or 0)
after = float(sys.argv[2] or 0)
expect = sys.argv[3]
if before <= 0 or after <= 0:
    raise SystemExit(1)
if expect == 'decrease' and not (after < before * 0.95):
    raise SystemExit(1)
if expect == 'increase' and not (after > before * 1.05):
    raise SystemExit(1)
PY
    log_note "screenshot_${stage}"
    ab_screenshot "$session" "$shot" 2>&1 | tee -a "$pw_log" >/dev/null || zoom_ok=0
    zoom_results=$(python3 - "$zoom_results" "$stage" "$shot" "$camera_mode" "$camera_radius" <<'PY'
import json, sys
raw, stage, shot, mode, radius = sys.argv[1:6]
data = json.loads(raw)
data.append({
    "stage": stage,
    "shot": shot,
    "cameraMode": mode,
    "cameraRadius": float(radius or 0),
})
print(json.dumps(data, ensure_ascii=False))
PY
)
  done
fi
json_to_file "$zoom_results" "$zoom_log"

log_note console
ab_cmd "$session" console >"$console_log" 2>&1 || true
log_note errors
ab_cmd "$session" errors >"$console_errors_log" 2>&1 || true
log_note screenshot_main
ab_screenshot "$session" "$shot_path" >>"$pw_log" 2>&1 || true
log_note close
ab_cmd "$session" close >>"$pw_log" 2>&1 || true

bevy_error_count=$(python3 - "$console_log" <<'PY'
import pathlib
import re
import sys
path = pathlib.Path(sys.argv[1])
if not path.exists():
    print(0)
    raise SystemExit(0)
count = 0
for line in path.read_text(encoding='utf-8', errors='replace').splitlines():
    has_error_marker = bool(re.search(r'\[ERROR\]|%cERROR%c|"type"\s*:\s*"error"|^error[: ]', line, re.I))
    if not has_error_marker:
        continue
    if 'Failed to load resource' in line:
        continue
    if '/bevy_' in line or '/oasis7' in line or 'bevy' in line.lower() or 'wgpu' in line.lower():
        count += 1
print(count)
PY
)

screenshot_ok=0
if [[ -s "$zoom_shot_near" && -s "$zoom_shot_mid" && -s "$zoom_shot_far" ]]; then
  screenshot_ok=1
elif [[ -s "$shot_path" ]]; then
  screenshot_ok=1
fi

visual_baseline_status="passed"
[[ "$skip_visual_baseline" -eq 1 ]] && visual_baseline_status="skipped"

overall_pass=1
[[ "$semantic_ok" -eq 1 ]] || overall_pass=0
[[ "$zoom_ok" -eq 1 ]] || overall_pass=0
[[ "$screenshot_ok" -eq 1 ]] || overall_pass=0
[[ "$bevy_error_count" -eq 0 ]] || overall_pass=0

{
  echo "# Viewer Release QA Summary"
  echo ""
  echo "- Timestamp: $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "- Scenario: \`$scenario\`"
  echo "- Viewer URL: \`$viewer_url\`"
  echo "- Viewer static dir: \`$viewer_static_dir\`"
  echo "- Render mode: \`$render_mode\`"
  echo "- Software-safe reason: \`${software_safe_reason:-n/a}\`"
  echo "- Browser automation: \`agent-browser\`"
  echo "- Visual baseline: $visual_baseline_status"
  echo "- Semantic web gate: $([[ "$semantic_ok" -eq 1 ]] && echo passed || echo failed)"
  echo "- Zoom texture gate: $([[ "$zoom_ok" -eq 1 ]] && echo "$zoom_status" || echo failed)"
  echo "- Screenshot artifact: $([[ "$screenshot_ok" -eq 1 ]] && echo passed || echo failed)"
  echo "- Bevy \`[ERROR]\` logs in console dump: $bevy_error_count"
  echo "- Overall: $([[ "$overall_pass" -eq 1 ]] && echo PASS || echo FAIL)"
  echo ""
  echo "## Artifacts"
  echo "- Live log: \`$live_log\`"
  echo "- Web log: \`$web_log\`"
  echo "- agent-browser log: \`$pw_log\`"
  echo "- Semantic gate log: \`$semantic_log\`"
  echo "- Zoom gate log: \`$zoom_log\`"
  echo "- Console dump: \`$console_log\`"
  echo "- Console error dump: \`$console_errors_log\`"
  echo "- Screenshot: \`$shot_path\`"
  echo "- Zoom near screenshot: \`$zoom_shot_near\`"
  echo "- Zoom mid screenshot: \`$zoom_shot_mid\`"
  echo "- Zoom far screenshot: \`$zoom_shot_far\`"
} >"$summary_path"

echo "release qa summary: $summary_path"
[[ "$overall_pass" -eq 1 ]] || exit 1
