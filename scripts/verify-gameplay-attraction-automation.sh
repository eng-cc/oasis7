#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/cargo-dev-lib.sh"
source "$repo_root/scripts/viewer-dependency-preflight.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/verify-gameplay-attraction-automation.sh [options] [launcher args...]

Verify the TASK-GAME-076 first-10/30-minute attraction automation matrix.

The required tier runs deterministic local gates:
- viewer semantic contract (`npm ... test:feedback-contract`)
- software_safe UI/Vitest
- runtime player_gameplay causality regressions
- Bevy/pixel-world visual hierarchy probe
- TASK-GAME-076 attraction cards, motivation-density card, and weak-sample regression
- TASK-GAME-076 summary writer contract for content-volume supplement reporting

The live tier also runs real stack checks:
- viewer-software-safe-step-regression.sh for browser/player-path evidence
- viewer-gameplay-attraction-playthrough.sh for beat-by-beat Playwright execution
- viewer-gameplay-attraction-ui-click-playthrough.sh for actual player-visible UI-click execution
- viewer-aw-test-completeness-playthrough.sh for __AW_TEST__ control-surface completeness
- oasis7-pure-api-parity-smoke.sh for live pure API gameplay causality

Options:
  --tier <required|live>       Validation tier (default: required)
  --out-dir <path>             Artifact root (default: output/playwright/gameplay-attraction-automation)
  --skip-bevy                  Skip Bevy visual probe and mark Bevy-only visual checks as unverified
  --skip-runtime-unit          Skip Rust runtime unit regressions and mark runtime-only checks as unverified
  --startup-timeout <secs>     Passed to live stack scripts (default: 240)
  -h, --help                   Show this help

Artifacts:
  <out-dir>/<run-id>/commands/*.log
  <out-dir>/<run-id>/gameplay-attraction-automation-summary.json
  <out-dir>/<run-id>/gameplay-attraction-automation-summary.md
USAGE
}

tier="required"
out_root="output/playwright/gameplay-attraction-automation"
skip_bevy=0
skip_runtime_unit=0
startup_timeout_secs=240
launcher_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tier)
      tier="${2:-}"
      shift 2
      ;;
    --out-dir)
      out_root="${2:-}"
      shift 2
      ;;
    --skip-bevy)
      skip_bevy=1
      shift
      ;;
    --skip-runtime-unit)
      skip_runtime_unit=1
      shift
      ;;
    --startup-timeout)
      startup_timeout_secs="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      launcher_args+=("$1")
      shift
      ;;
  esac
done

[[ "$tier" == "required" || "$tier" == "live" ]] || {
  echo "error: --tier must be required or live" >&2
  exit 2
}
[[ -n "$out_root" ]] || { echo "error: --out-dir cannot be empty" >&2; exit 2; }
[[ "$startup_timeout_secs" =~ ^[0-9]+$ ]] && [[ "$startup_timeout_secs" -gt 0 ]] || {
  echo "error: --startup-timeout must be a positive integer" >&2
  exit 2
}

viewer_dependency_preflight "$repo_root" test

stamp=$(date -u +"%Y-%m-%dT%H-%M-%SZ")
run_id="task-game-076-${tier}-${stamp}"
out_dir="$out_root/$run_id"
commands_dir="$out_dir/commands"
summary_json="$out_dir/gameplay-attraction-automation-summary.json"
summary_md="$out_dir/gameplay-attraction-automation-summary.md"
summary_input="$out_dir/gameplay-attraction-automation-input.json"
mkdir -p "$commands_dir"

COMMAND_KEYS=()

run_command() {
  local key=$1
  shift
  local log_path="$commands_dir/${key}.log"
  COMMAND_KEYS+=("$key")
  printf '### [%s] running: %s\n' "$(date '+%H:%M:%S')" "$*" | tee "$log_path"
  set +e
  "$@" >>"$log_path" 2>&1
  local rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    printf '### [%s] pass: %s\n' "$(date '+%H:%M:%S')" "$key" | tee -a "$log_path"
  else
    printf '### [%s] fail(%s): %s\n' "$(date '+%H:%M:%S')" "$rc" "$key" | tee -a "$log_path"
  fi
  return "$rc"
}

mark_skipped() {
  local key=$1
  local reason=$2
  local log_path="$commands_dir/${key}.log"
  COMMAND_KEYS+=("$key")
  printf 'skipped: %s\n' "$reason" >"$log_path"
}

offset_port() {
  python3 - "$1" "$2" <<'PY'
import sys
print(int(sys.argv[1]) + int(sys.argv[2]))
PY
}

offset_bind_addr() {
  python3 - "$1" "$2" <<'PY'
import sys
raw = sys.argv[1]
offset = int(sys.argv[2])
host, sep, port = raw.rpartition(":")
if not sep or not port.isdigit():
    raise SystemExit(f"invalid bind address: {raw}")
print(f"{host}:{int(port) + offset}")
PY
}

build_offset_launcher_args() {
  local offset=$1
  local out_file=$2
  shift 2
  local args=("$@")
  local saw_viewer_port=0
  local saw_web_bind=0
  local saw_live_bind=0
  : >"$out_file"
  local index=0
  while [[ "$index" -lt "${#args[@]}" ]]; do
    case "${args[$index]}" in
      --viewer-port)
        saw_viewer_port=1
        printf '%s\n' "${args[$index]}" >>"$out_file"
        index=$((index + 1))
        printf '%s\n' "$(offset_port "${args[$index]:-4173}" "$offset")" >>"$out_file"
        ;;
      --web-bind)
        saw_web_bind=1
        printf '%s\n' "${args[$index]}" >>"$out_file"
        index=$((index + 1))
        printf '%s\n' "$(offset_bind_addr "${args[$index]:-127.0.0.1:5011}" "$offset")" >>"$out_file"
        ;;
      --live-bind)
        saw_live_bind=1
        printf '%s\n' "${args[$index]}" >>"$out_file"
        index=$((index + 1))
        printf '%s\n' "$(offset_bind_addr "${args[$index]:-127.0.0.1:5023}" "$offset")" >>"$out_file"
        ;;
      *)
        printf '%s\n' "${args[$index]}" >>"$out_file"
        ;;
    esac
    index=$((index + 1))
  done
  if [[ "$saw_viewer_port" -eq 0 ]]; then
    printf '%s\n' --viewer-port "$(offset_port 4173 "$offset")" >>"$out_file"
  fi
  if [[ "$saw_web_bind" -eq 0 ]]; then
    printf '%s\n' --web-bind "$(offset_bind_addr 127.0.0.1:5011 "$offset")" >>"$out_file"
  fi
  if [[ "$saw_live_bind" -eq 0 ]]; then
    printf '%s\n' --live-bind "$(offset_bind_addr 127.0.0.1:5023 "$offset")" >>"$out_file"
  fi
}

read_args_file() {
  local out_name=$1
  local args_file=$2
  local line
  eval "$out_name=()"
  while IFS= read -r line; do
    eval "$out_name+=(\"\$line\")"
  done <"$args_file"
}

overall_status="pass"
run_or_record_failure() {
  if ! run_command "$@"; then
    overall_status="fail"
  fi
}

run_or_record_failure viewer_semantic_contract \
  npm --prefix crates/oasis7_viewer run test:feedback-contract

run_or_record_failure software_safe_ui \
  npm --prefix crates/oasis7_viewer run test:ui -- software_safe_src/main.test.jsx

if [[ "$skip_runtime_unit" -eq 0 ]]; then
  run_or_record_failure runtime_control_feeling \
    oasis7_cargo_dev test -p oasis7 \
      viewer::runtime_live::tests::snapshot_progress::compat_snapshot_surfaces_control_feeling_contract_fields_from_gameplay_feedback \
      -- --nocapture
  run_or_record_failure runtime_no_progress_recovery \
    oasis7_cargo_dev test -p oasis7 \
      viewer::runtime_live::tests::snapshot_progress::compat_snapshot_keeps_post_onboarding_no_progress_after_confirmed_progress \
      -- --nocapture
  run_or_record_failure runtime_chain_sync_blocker \
    oasis7_cargo_dev test -p oasis7 \
      viewer::runtime_live::tests::snapshot_progress::compat_snapshot_blocks_first_session_when_chain_sync_is_unavailable \
      -- --nocapture
  run_or_record_failure runtime_persist_backfill \
    oasis7_cargo_dev test -p oasis7 \
      simulator::tests::persist::snapshot_player_gameplay_execution_state_backfills_from_legacy_fields \
      -- --nocapture
else
  mark_skipped runtime_control_feeling "--skip-runtime-unit"
  mark_skipped runtime_no_progress_recovery "--skip-runtime-unit"
  mark_skipped runtime_chain_sync_blocker "--skip-runtime-unit"
  mark_skipped runtime_persist_backfill "--skip-runtime-unit"
fi

if [[ "$skip_bevy" -eq 0 ]]; then
  run_or_record_failure bevy_visual_probe \
    ./scripts/viewer-pixel-world-bevy-render-probe.sh
else
  mark_skipped bevy_visual_probe "--skip-bevy"
fi

run_or_record_failure attraction_sufficiency_cards \
  node crates/oasis7_viewer/scripts/gameplay-attraction-scenario.test.mjs

run_or_record_failure summary_writer_contract \
  node crates/oasis7_viewer/scripts/gameplay-attraction-summary-writer.test.mjs

run_or_record_failure aw_test_completeness_guard \
  node crates/oasis7_viewer/scripts/aw-test-completeness.test.mjs

if [[ "$tier" == "live" ]]; then
  playthrough_args_file="$out_dir/playthrough-launcher-args.txt"
  ui_click_args_file="$out_dir/ui-click-playthrough-launcher-args.txt"
  aw_test_args_file="$out_dir/aw-test-launcher-args.txt"
  pure_api_args_file="$out_dir/pure-api-launcher-args.txt"
  build_offset_launcher_args 10 "$playthrough_args_file" "${launcher_args[@]}"
  build_offset_launcher_args 20 "$ui_click_args_file" "${launcher_args[@]}"
  build_offset_launcher_args 30 "$aw_test_args_file" "${launcher_args[@]}"
  build_offset_launcher_args 40 "$pure_api_args_file" "${launcher_args[@]}"
  read_args_file playthrough_launcher_args "$playthrough_args_file"
  read_args_file ui_click_launcher_args "$ui_click_args_file"
  read_args_file aw_test_launcher_args "$aw_test_args_file"
  read_args_file pure_api_launcher_args "$pure_api_args_file"
  if [[ "${#launcher_args[@]}" -gt 0 ]]; then
    run_or_record_failure live_browser_player_path \
      ./scripts/viewer-software-safe-step-regression.sh \
        --out-dir "$out_dir/live-browser" \
        --startup-timeout "$startup_timeout_secs" \
        "${launcher_args[@]}"
    run_or_record_failure live_browser_30m_playthrough \
      ./scripts/viewer-gameplay-attraction-playthrough.sh \
        --out-dir "$out_dir/live-playthrough" \
        --startup-timeout "$startup_timeout_secs" \
        "${playthrough_launcher_args[@]}"
    run_or_record_failure live_browser_30m_ui_click_playthrough \
      ./scripts/viewer-gameplay-attraction-ui-click-playthrough.sh \
        --out-dir "$out_dir/live-ui-click-playthrough" \
        --startup-timeout "$startup_timeout_secs" \
        "${ui_click_launcher_args[@]}"
    run_or_record_failure live_aw_test_completeness_playthrough \
      ./scripts/viewer-aw-test-completeness-playthrough.sh \
        --out-dir "$out_dir/live-aw-test-completeness" \
        --startup-timeout "$startup_timeout_secs" \
        "${aw_test_launcher_args[@]}"
    run_or_record_failure live_pure_api_gameplay \
      ./scripts/oasis7-pure-api-parity-smoke.sh \
        --tier required \
        --out-dir "$out_dir/live-pure-api" \
        --startup-timeout "$startup_timeout_secs" \
        "${pure_api_launcher_args[@]}"
  else
    run_or_record_failure live_browser_player_path \
      ./scripts/viewer-software-safe-step-regression.sh \
        --out-dir "$out_dir/live-browser" \
        --startup-timeout "$startup_timeout_secs"
    run_or_record_failure live_browser_30m_playthrough \
      ./scripts/viewer-gameplay-attraction-playthrough.sh \
        --out-dir "$out_dir/live-playthrough" \
        --startup-timeout "$startup_timeout_secs" \
        "${playthrough_launcher_args[@]}"
    run_or_record_failure live_browser_30m_ui_click_playthrough \
      ./scripts/viewer-gameplay-attraction-ui-click-playthrough.sh \
        --out-dir "$out_dir/live-ui-click-playthrough" \
        --startup-timeout "$startup_timeout_secs" \
        "${ui_click_launcher_args[@]}"
    run_or_record_failure live_aw_test_completeness_playthrough \
      ./scripts/viewer-aw-test-completeness-playthrough.sh \
        --out-dir "$out_dir/live-aw-test-completeness" \
        --startup-timeout "$startup_timeout_secs" \
        "${aw_test_launcher_args[@]}"
    run_or_record_failure live_pure_api_gameplay \
      ./scripts/oasis7-pure-api-parity-smoke.sh \
        --tier required \
        --out-dir "$out_dir/live-pure-api" \
        --startup-timeout "$startup_timeout_secs" \
        "${pure_api_launcher_args[@]}"
  fi
else
  mark_skipped live_browser_player_path "run with --tier live"
  mark_skipped live_browser_30m_playthrough "run with --tier live"
  mark_skipped live_browser_30m_ui_click_playthrough "run with --tier live"
  mark_skipped live_aw_test_completeness_playthrough "run with --tier live"
  mark_skipped live_pure_api_gameplay "run with --tier live"
fi

python3 - "$summary_input" "$tier" "$overall_status" "$skip_bevy" "$skip_runtime_unit" "$out_dir" "${COMMAND_KEYS[@]}" <<'PY'
import json
import sys
from pathlib import Path

summary_input = Path(sys.argv[1])
tier = sys.argv[2]
overall_status = sys.argv[3]
skip_bevy = sys.argv[4] == "1"
skip_runtime_unit = sys.argv[5] == "1"
out_dir = Path(sys.argv[6])
keys = sys.argv[7:]

commands = {}
for key in keys:
    log = out_dir / "commands" / f"{key}.log"
    text = log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    if text.startswith("skipped:"):
        status = "skipped"
    elif "###" in text and " fail(" in text:
        status = "fail"
    elif "###" in text and f" pass: {key}" in text:
        status = "pass"
    else:
        status = "unknown"
    commands[key] = {"status": status, "log": str(log)}
summary_input.write_text(json.dumps({
    "tier": tier,
    "overallStatus": overall_status,
    "outDir": str(out_dir),
    "commands": commands,
    "skipBevy": skip_bevy,
    "skipRuntimeUnit": skip_runtime_unit,
}, indent=2) + "\n", encoding="utf-8")
PY

node crates/oasis7_viewer/scripts/write-gameplay-attraction-automation-summary.mjs \
  "$summary_input" \
  "$summary_json" \
  "$summary_md"

printf 'TASK-GAME-076 automation summary: %s\n' "$summary_json"
printf 'TASK-GAME-076 automation report: %s\n' "$summary_md"

if [[ "$overall_status" != "pass" ]]; then
  exit 1
fi
