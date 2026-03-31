#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"
source "$ROOT_DIR/scripts/agent-browser-lib.sh"
source "$ROOT_DIR/scripts/bundle-freshness-lib.sh"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/worktree-harness.sh up [options]
  ./scripts/worktree-harness.sh down
  ./scripts/worktree-harness.sh status [--json]
  ./scripts/worktree-harness.sh url
  ./scripts/worktree-harness.sh logs
  ./scripts/worktree-harness.sh smoke [--timeout <secs>]

Purpose:
  Run an isolated Viewer Web / launcher stack for the current git worktree.

Options for `up`:
  --with-llm               Enable LLM mode (default; required for gameplay)
  --no-llm                 Negative-path only; launcher boot will fail fast without LLM
  --bundle-mode            Build/reuse a worktree-local bundle and boot from it
  --source-mode            Boot directly from source (default)
  --smoke-timeout <secs>   After boot, run a minimal agent-browser smoke within <secs>

Options for `status`:
  --json                   Print raw state.json

Options for `smoke`:
  --timeout <secs>         Smoke timeout (default: 30)
USAGE
}

wh_require_git_worktree
WORKTREE_ID="$(wh_worktree_id)"
GIT_HEAD="$(wh_git_head)"
HARNESS_ROOT="$(wh_harness_root "$ROOT_DIR" "$WORKTREE_ID")"
STATE_FILE="$(wh_state_file "$HARNESS_ROOT")"
RUNTIME_DIR="$(wh_runtime_dir "$HARNESS_ROOT")"
ARTIFACT_DIR="$(wh_artifacts_dir "$HARNESS_ROOT")"
BROWSER_DIR="$(wh_browser_dir "$HARNESS_ROOT")"
BUNDLE_DIR="$(wh_default_bundle_dir "$HARNESS_ROOT")"
STARTUP_LOG="$(wh_startup_log "$HARNESS_ROOT")"
META_FILE="$(wh_runtime_meta_file "$HARNESS_ROOT")"
BROWSER_SESSION="$(wh_browser_session "$WORKTREE_ID")"

wh_prepare_dirs "$HARNESS_ROOT"

action=${1:-}
if [[ -z "$action" ]]; then
  usage >&2
  exit 2
fi
shift || true

kill_recorded_processes() {
  local harness_pid launcher_pid
  harness_pid=$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)
  launcher_pid=$(wh_state_get "$STATE_FILE" launcher_pid 2>/dev/null || true)

  if wh_pid_alive "$harness_pid"; then
    kill "$harness_pid" >/dev/null 2>&1 || true
    wait "$harness_pid" >/dev/null 2>&1 || true
  fi
  if wh_pid_alive "$launcher_pid"; then
    kill "$launcher_pid" >/dev/null 2>&1 || true
    wait "$launcher_pid" >/dev/null 2>&1 || true
  fi
  ab_cmd "$BROWSER_SESSION" close >/dev/null 2>&1 || true
}

viewer_http_ready() {
  local viewer_url
  viewer_url=$(wh_state_get "$STATE_FILE" viewer_url 2>/dev/null || true)
  [[ -n "$viewer_url" ]] || return 1
  curl -fsS --max-time 2 "$viewer_url" >/dev/null 2>&1
}

refresh_state() {
  local current_status harness_pid launcher_pid

  [[ -f "$STATE_FILE" ]] || return 0
  current_status=$(wh_state_get "$STATE_FILE" status 2>/dev/null || true)
  harness_pid=$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)
  launcher_pid=$(wh_state_get "$STATE_FILE" launcher_pid 2>/dev/null || true)

  if [[ "$current_status" == "ready" ]]; then
    if ! wh_pid_alive "$harness_pid" && ! wh_pid_alive "$launcher_pid"; then
      wh_state_write "$STATE_FILE" '{"status": "stopped", "harness_pid": null, "launcher_pid": null}'
      return 0
    fi
  fi
}

require_ready_harness() {
  local status
  refresh_state
  status=$(wh_state_get "$STATE_FILE" status 2>/dev/null || true)
  if [[ "$status" != "ready" ]]; then
    echo "error: worktree harness is not ready (status=${status:-missing})" >&2
    exit 1
  fi
  if ! viewer_http_ready; then
    echo "error: worktree harness viewer is not reachable" >&2
    exit 1
  fi
}

run_smoke() {
  local timeout_secs=${1:-30}
  local viewer_url smoke_dir state_raw

  require_ready_harness
  viewer_url=$(wh_state_get "$STATE_FILE" viewer_url 2>/dev/null || true)
  if [[ -z "$viewer_url" ]]; then
    echo "error: harness is not ready; missing viewer_url in $STATE_FILE" >&2
    exit 1
  fi

  ab_require
  smoke_dir="$ARTIFACT_DIR/smoke-$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$smoke_dir"

  ab_open "$BROWSER_SESSION" 0 "$viewer_url" >>"$smoke_dir/agent-browser.log" 2>&1
  ab_cmd "$BROWSER_SESSION" wait --load networkidle >>"$smoke_dir/agent-browser.log" 2>&1 || true
  state_raw=$(ab_eval "$BROWSER_SESSION" 'JSON.stringify(window.__AW_TEST__ ? window.__AW_TEST__.getState() : null)') || {
    cat "$smoke_dir/agent-browser.log" >&2
    exit 1
  }
  if [[ -z "$state_raw" || "$state_raw" == "null" ]]; then
    echo "error: worktree harness smoke failed; __AW_TEST__.getState() returned empty payload" >&2
    exit 1
  fi
  json_to_file "$state_raw" "$smoke_dir/state.json"
  ab_screenshot "$BROWSER_SESSION" "$smoke_dir/final.png" >>"$smoke_dir/agent-browser.log" 2>&1 || true
  wh_state_write "$STATE_FILE" "{\"last_smoke_dir\": $(json_quote "$smoke_dir"), \"last_smoke_ok\": true, \"last_smoke_timeout_secs\": $timeout_secs}"
  printf '%s\n' "$smoke_dir"
}

case "$action" in
  up)
    ENABLE_LLM="1"
    BOOT_MODE="source"
    SMOKE_TIMEOUT=0
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --with-llm)
          ENABLE_LLM="1"
          shift
          ;;
        --no-llm)
          ENABLE_LLM="0"
          shift
          ;;
        --bundle-mode)
          BOOT_MODE="bundle"
          shift
          ;;
        --source-mode)
          BOOT_MODE="source"
          shift
          ;;
        --smoke-timeout)
          SMOKE_TIMEOUT="${2:-}"
          shift 2
          ;;
        -h|--help)
          usage
          exit 0
          ;;
        *)
          echo "error: unknown option for up: $1" >&2
          usage >&2
          exit 2
          ;;
      esac
    done
    [[ "$SMOKE_TIMEOUT" =~ ^[0-9]+$ ]] || { echo "error: --smoke-timeout must be a non-negative integer" >&2; exit 2; }
    if [[ "$ENABLE_LLM" != "1" ]]; then
      echo "error: worktree harness now boots through ./scripts/run-game-test.sh and oasis7_game_launcher, both of which require active LLM access" >&2
      echo "hint: use direct oasis7_viewer_live --no-llm only for observer/debug diagnostics outside the launcher stack" >&2
      exit 2
    fi

    if wh_pid_alive "$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)"; then
      echo "info: harness already running for $WORKTREE_ID"
      wh_state_show "$STATE_FILE"
      exit 0
    fi

    kill_recorded_processes
    rm -f "$META_FILE" "$STARTUP_LOG"

    ports_json=$(wh_resolve_ports_json)
    viewer_port=$(json_get "$ports_json" viewer_port)
    web_bind=$(json_get "$ports_json" web_bind)
    live_bind=$(json_get "$ports_json" live_bind)
    chain_status_bind=$(json_get "$ports_json" chain_status_bind)

    wh_state_write "$STATE_FILE" "$(python3 - <<PY
import json
payload = {
    "worktree_id": ${WORKTREE_ID@Q},
    "worktree_path": ${PWD@Q},
    "git_head": ${GIT_HEAD@Q},
    "status": "booting",
    "boot_mode": ${BOOT_MODE@Q},
    "llm_enabled": ${ENABLE_LLM@Q},
    "viewer_port": int(${viewer_port@Q}),
    "web_bind": ${web_bind@Q},
    "live_bind": ${live_bind@Q},
    "chain_status_bind": ${chain_status_bind@Q},
    "bundle_dir": ${BUNDLE_DIR@Q},
    "runtime_dir": ${RUNTIME_DIR@Q},
    "artifact_dir": ${ARTIFACT_DIR@Q},
    "browser_dir": ${BROWSER_DIR@Q},
    "browser_session": ${BROWSER_SESSION@Q},
    "startup_log": ${STARTUP_LOG@Q},
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

    if [[ "$BOOT_MODE" == "bundle" ]]; then
      if [[ ! -x "$BUNDLE_DIR/run-game.sh" ]] || ! bundle_check_freshness "$ROOT_DIR" "$BUNDLE_DIR" >/dev/null 2>&1; then
        ./scripts/build-game-launcher-bundle.sh --profile dev --out-dir "$BUNDLE_DIR" >>"$STARTUP_LOG" 2>&1
      fi
      bundle_args=(--bundle-dir "$BUNDLE_DIR")
    else
      bundle_args=()
    fi

    run_args=(
      "${bundle_args[@]}"
      --viewer-port "$viewer_port"
      --web-bind "$web_bind"
      --live-bind "$live_bind"
      --chain-node-id "$WORKTREE_ID"
      --chain-status-bind "$chain_status_bind"
      --output-dir "$RUNTIME_DIR"
      --run-id "$WORKTREE_ID"
      --meta-file "$META_FILE"
      --json-ready
    )
    run_args+=(--with-llm)

    nohup ./scripts/run-game-test.sh "${run_args[@]}" >"$STARTUP_LOG" 2>&1 < /dev/null &
    HARNESS_PID=$!
    wh_state_write "$STATE_FILE" "{\"harness_pid\": $HARNESS_PID}"

    for _ in $(seq 1 180); do
      if ! wh_pid_alive "$HARNESS_PID"; then
        echo "error: worktree harness boot failed; run-game-test.sh exited unexpectedly" >&2
        tail -n 120 "$STARTUP_LOG" >&2 || true
        exit 1
      fi
      if [[ -f "$META_FILE" ]]; then
        stack_ready=$(wh_env_file_get "$META_FILE" STACK_READY 2>/dev/null || true)
        if [[ "$stack_ready" == "1" ]]; then
          break
        fi
      fi
      sleep 1
    done

    if [[ ! -f "$META_FILE" ]] || [[ "$(wh_env_file_get "$META_FILE" STACK_READY 2>/dev/null || true)" != "1" ]]; then
      echo "error: timed out waiting for worktree harness readiness" >&2
      tail -n 120 "$STARTUP_LOG" >&2 || true
      exit 1
    fi

    viewer_url=$(wh_env_file_get "$META_FILE" GAME_URL)
    launcher_pid=$(wh_env_file_get "$META_FILE" LAUNCHER_PID 2>/dev/null || true)
    wh_state_write "$STATE_FILE" "$(python3 - <<PY
import json
payload = {
    "status": "ready",
    "viewer_url": ${viewer_url@Q},
    "launcher_pid": int(${launcher_pid@Q}) if ${launcher_pid@Q} else None,
    "meta_file": ${META_FILE@Q},
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

    if [[ "$SMOKE_TIMEOUT" -gt 0 ]]; then
      smoke_dir=$(run_smoke "$SMOKE_TIMEOUT")
      echo "info: smoke artifacts: $smoke_dir"
    fi

    echo "worktree harness ready: $viewer_url"
    ;;
  down)
    kill_recorded_processes
    wh_state_write "$STATE_FILE" '{"status": "stopped", "harness_pid": null, "launcher_pid": null}'
    echo "worktree harness stopped: $WORKTREE_ID"
    ;;
  status)
    refresh_state
    if [[ "${1:-}" == "--json" ]]; then
      wh_state_show "$STATE_FILE"
      exit 0
    fi
    python3 - "$STATE_FILE" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

state_path = pathlib.Path(sys.argv[1])
if not state_path.exists():
    raise SystemExit("error: no worktree harness state found")
state = json.loads(state_path.read_text(encoding="utf-8"))
for key in ("worktree_id", "status", "viewer_url", "runtime_dir", "artifact_dir", "startup_log"):
    if key in state:
        print(f"{key}: {state[key]}")
PY
    ;;
  url)
    require_ready_harness
    wh_state_get "$STATE_FILE" viewer_url
    ;;
  logs)
    refresh_state
    python3 - "$STATE_FILE" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
for key in ("startup_log", "runtime_dir", "artifact_dir", "last_smoke_dir"):
    value = state.get(key)
    if value:
        print(f"{key}: {value}")
PY
    ;;
  smoke)
    timeout_secs=30
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --timeout)
          timeout_secs="${2:-}"
          shift 2
          ;;
        -h|--help)
          usage
          exit 0
          ;;
        *)
          echo "error: unknown option for smoke: $1" >&2
          usage >&2
          exit 2
          ;;
      esac
    done
    [[ "$timeout_secs" =~ ^[0-9]+$ ]] || { echo "error: --timeout must be a non-negative integer" >&2; exit 2; }
    run_smoke "$timeout_secs"
    ;;
  -h|--help)
    usage
    ;;
  *)
    echo "error: unknown action: $action" >&2
    usage >&2
    exit 2
    ;;
esac
