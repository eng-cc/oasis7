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
  --startup-timeout <secs> Maximum startup deadline (default: 300)
  --smoke-timeout <secs>   After boot, run a minimal agent-browser smoke within <secs>

Options for `status`:
  --json                   Print raw state.json

Options for `smoke`:
  --timeout <secs>         Smoke timeout (default: 30)
USAGE
  "$ROOT_DIR/scripts/launcher-help-contract.sh" shared
}

wh_require_git_worktree
WORKTREE_ID="$(wh_worktree_id)"
GIT_HEAD="$(wh_git_head)"
HARNESS_ROOT="$(wh_harness_root "$ROOT_DIR" "$WORKTREE_ID")"
PORT_REGISTRY_COMMON_DIR="$(wh_git_common_dir)"
STATE_FILE="$(wh_state_file "$HARNESS_ROOT")"
RUNTIME_DIR="$(wh_runtime_dir "$HARNESS_ROOT")"
ARTIFACT_DIR="$(wh_artifacts_dir "$HARNESS_ROOT")"
BROWSER_DIR="$(wh_browser_dir "$HARNESS_ROOT")"
BUNDLE_DIR="$(wh_default_bundle_dir "$HARNESS_ROOT")"
STARTUP_LOG="$(wh_startup_log "$HARNESS_ROOT")"
META_FILE="$(wh_runtime_meta_file "$HARNESS_ROOT")"
BROWSER_SESSION="$(wh_browser_session "$WORKTREE_ID")"
PORT_RESERVATION_TOKEN=""
HARNESS_IDENTITY=""
CLEANUP_PENDING_REASON=""

wh_prepare_dirs "$HARNESS_ROOT"

action=${1:-}
if [[ -z "$action" ]]; then
  usage >&2
  exit 2
fi
shift || true

case "$action" in
  up|down|status|url|logs|smoke)
    wh_lifecycle_lock_acquire "$HARNESS_ROOT" || exit 1
    trap 'wh_lifecycle_lock_release' EXIT
    ;;
esac

persist_launcher_record_from_meta() {
  local metadata_status launcher_pid launcher_pgid launcher_identity
  local recorded_pid recorded_pgid recorded_identity
  [[ -f "$META_FILE" ]] || return 1
  metadata_status=$(wh_env_file_get "$META_FILE" STACK_READY 2>/dev/null || true)
  [[ "$metadata_status" == "0" || "$metadata_status" == "1" ]] || return 1
  launcher_pid=$(wh_env_file_get "$META_FILE" LAUNCHER_PID 2>/dev/null || true)
  launcher_pgid=$(wh_env_file_get "$META_FILE" LAUNCHER_PGID 2>/dev/null || true)
  launcher_identity=$(wh_env_file_get "$META_FILE" LAUNCHER_IDENTITY 2>/dev/null || true)
  [[ -n "$launcher_pid" && -n "$launcher_pgid" && -n "$launcher_identity" ]] || return 1
  wh_process_record_alive "$launcher_pid" "$launcher_pgid" "$launcher_identity" || return 1

  recorded_pid=$(wh_state_get "$STATE_FILE" launcher_pid 2>/dev/null || true)
  recorded_pgid=$(wh_state_get "$STATE_FILE" launcher_pgid 2>/dev/null || true)
  recorded_identity=$(wh_state_get "$STATE_FILE" launcher_identity 2>/dev/null || true)
  if [[ -n "$recorded_pid" || -n "$recorded_pgid" || -n "$recorded_identity" ]]; then
    [[ "$recorded_pid" == "$launcher_pid" && "$recorded_pgid" == "$launcher_pgid" && \
      "$recorded_identity" == "$launcher_identity" ]] || return 2
    return 0
  fi
  wh_state_write "$STATE_FILE" "$(python3 - "$launcher_pid" "$launcher_pgid" "$launcher_identity" <<'PY'
import json
import sys

print(json.dumps({
    "launcher_pid": int(sys.argv[1]),
    "launcher_pgid": int(sys.argv[2]),
    "launcher_identity": sys.argv[3],
}))
PY
)"
}

kill_recorded_processes() {
  local harness_pid harness_pgid launcher_pid launcher_pgid reservation_token
  local harness_identity launcher_identity cleanup_status=0
  local current_status metadata_status metadata_launcher_pid metadata_launcher_pgid metadata_launcher_identity
  local -a legacy_identityless_records=()
  CLEANUP_PENDING_REASON=""
  current_status=$(wh_state_get "$STATE_FILE" status 2>/dev/null || true)
  harness_pid=$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)
  harness_pgid=$(wh_state_get "$STATE_FILE" harness_pgid 2>/dev/null || true)
  harness_identity=$(wh_state_get "$STATE_FILE" harness_identity 2>/dev/null || true)
  launcher_pid=$(wh_state_get "$STATE_FILE" launcher_pid 2>/dev/null || true)
  launcher_pgid=$(wh_state_get "$STATE_FILE" launcher_pgid 2>/dev/null || true)
  launcher_identity=$(wh_state_get "$STATE_FILE" launcher_identity 2>/dev/null || true)
  reservation_token=$(wh_state_get "$STATE_FILE" port_reservation_token 2>/dev/null || true)

  # An outer harness can be SIGKILLed after run-launcher-stack has published a
  # complete STACK_READY=0 record but before this process copies the inner
  # launcher identity into state.json. Adopt only a non-stopped generation and
  # only when the state has no conflicting launcher record; termination below
  # performs the final identity/PGID authentication before signaling.
  if [[ "$current_status" =~ ^(booting|failed|ready)$ && -f "$META_FILE" ]]; then
    metadata_status=$(wh_env_file_get "$META_FILE" STACK_READY 2>/dev/null || true)
    metadata_launcher_pid=$(wh_env_file_get "$META_FILE" LAUNCHER_PID 2>/dev/null || true)
    metadata_launcher_pgid=$(wh_env_file_get "$META_FILE" LAUNCHER_PGID 2>/dev/null || true)
    metadata_launcher_identity=$(wh_env_file_get "$META_FILE" LAUNCHER_IDENTITY 2>/dev/null || true)
    if [[ "$metadata_status" == "0" || "$metadata_status" == "1" ]] && \
      [[ "$metadata_launcher_pid" =~ ^[1-9][0-9]*$ ]] && \
      [[ "$metadata_launcher_pgid" =~ ^[1-9][0-9]*$ ]] && \
      [[ -n "$metadata_launcher_identity" ]]; then
      if [[ -n "$launcher_pid" || -n "$launcher_pgid" || -n "$launcher_identity" ]]; then
        if [[ "$launcher_pid" != "$metadata_launcher_pid" || "$launcher_pgid" != "$metadata_launcher_pgid" || \
          "$launcher_identity" != "$metadata_launcher_identity" ]]; then
          echo "error: launcher metadata identity conflicts with persisted state" >&2
          cleanup_status=1
        fi
      else
        launcher_pid="$metadata_launcher_pid"
        launcher_pgid="$metadata_launcher_pgid"
        launcher_identity="$metadata_launcher_identity"
        wh_state_write "$STATE_FILE" "$(python3 - "$launcher_pid" "$launcher_pgid" "$launcher_identity" <<'PY'
import json
import sys

print(json.dumps({
    "launcher_pid": int(sys.argv[1]),
    "launcher_pgid": int(sys.argv[2]),
    "launcher_identity": sys.argv[3],
}))
PY
)"
      fi
    fi
  fi

  # Legacy records may contain only a PID/PGID.  A live process or process
  # group without a captured identity is intentionally not actionable: the
  # PID may have been reused by a foreign process.  Keep the complete record
  # and reservation for operator-owned recovery rather than guessing.
  if [[ -z "$launcher_identity" ]] && {
    { [[ -n "$launcher_pid" ]] && wh_pid_alive "$launcher_pid"; } ||
    { [[ -n "$launcher_pgid" ]] && wh_process_group_alive "$launcher_pgid"; }
  }; then
    legacy_identityless_records+=("launcher")
  fi
  if [[ -z "$harness_identity" ]] && {
    { [[ -n "$harness_pid" ]] && wh_pid_alive "$harness_pid"; } ||
    { [[ -n "$harness_pgid" ]] && wh_process_group_alive "$harness_pgid"; }
  }; then
    legacy_identityless_records+=("harness")
  fi
  if [[ "${#legacy_identityless_records[@]}" -gt 0 ]]; then
    CLEANUP_PENDING_REASON="cannot safely clean live legacy ${legacy_identityless_records[*]} process record: stable identity unavailable; no signal sent; operator-owned recovery required; reservation retained"
    echo "error: $CLEANUP_PENDING_REASON" >&2
    return 1
  fi

  # Older state records predate PGID publication.  Derive the group only
  # while its recorded PID is live; otherwise leave the stale record alone so
  # cleanup cannot guess at an unrelated process group.
  if [[ -z "$launcher_pgid" ]] && wh_pid_alive "$launcher_pid"; then
    launcher_pgid=$(wh_process_group_id "$launcher_pid" 2>/dev/null || true)
  fi
  if [[ -z "$harness_pgid" ]] && wh_pid_alive "$harness_pid"; then
    harness_pgid=$(wh_process_group_id "$harness_pid" 2>/dev/null || true)
  fi

  if [[ -n "$launcher_pid" || -n "$launcher_pgid" ]]; then
    if [[ -z "$launcher_pid" || -z "$launcher_pgid" ]]; then
      if { [[ -n "$launcher_pid" ]] && wh_pid_alive "$launcher_pid"; } || { [[ -n "$launcher_pgid" ]] && wh_process_group_alive "$launcher_pgid"; }; then
        echo "error: incomplete launcher process-group record prevents safe cleanup" >&2
        cleanup_status=1
      fi
    elif [[ "$cleanup_status" -eq 0 ]]; then
      if ! wh_terminate_process_group "$launcher_pid" "$launcher_pgid" 2000 "$launcher_identity"; then
        echo "error: unable to prove launcher process-group quiescence" >&2
        cleanup_status=1
      fi
    fi
  fi
  if [[ "$cleanup_status" -eq 0 && ( -n "$harness_pid" || -n "$harness_pgid" ) ]]; then
    if [[ -z "$harness_pid" || -z "$harness_pgid" ]]; then
      if { [[ -n "$harness_pid" ]] && wh_pid_alive "$harness_pid"; } || { [[ -n "$harness_pgid" ]] && wh_process_group_alive "$harness_pgid"; }; then
        echo "error: incomplete harness process-group record prevents safe cleanup" >&2
        cleanup_status=1
      fi
    else
      if ! wh_terminate_process_group "$harness_pid" "$harness_pgid" 2000 "$harness_identity"; then
        echo "error: unable to prove harness process-group quiescence" >&2
        cleanup_status=1
      fi
    fi
  fi
  if [[ "$cleanup_status" -ne 0 ]]; then
    return 1
  fi
  ab_cmd "$BROWSER_SESSION" close >/dev/null 2>&1 || true
  if [[ -n "$reservation_token" ]]; then
    if ! wh_release_ports_reservation "$HARNESS_ROOT" "$reservation_token" "$PORT_REGISTRY_COMMON_DIR"; then
      echo "error: unable to release harness port reservation after shutdown" >&2
      return 1
    fi
  fi
}

write_cleanup_failure_state() {
  local fallback_reason=$1
  if [[ -n "$CLEANUP_PENDING_REASON" ]]; then
    wh_state_write "$STATE_FILE" "{\"status\": \"failed\", \"phase\": \"cleanup_pending\", \"failure_reason\": $(json_quote "$CLEANUP_PENDING_REASON") }"
  else
    wh_state_write "$STATE_FILE" "{\"status\": \"failed\", \"phase\": \"cleanup_failed\", \"failure_reason\": $(json_quote "$fallback_reason") }"
  fi
}

viewer_http_ready() {
  local viewer_url
  viewer_url=$(wh_state_get "$STATE_FILE" viewer_url 2>/dev/null || true)
  [[ -n "$viewer_url" ]] || return 1
  curl -fsS --max-time 2 "$viewer_url" >/dev/null 2>&1
}

refresh_state() {
  local current_status harness_pid harness_pgid harness_identity launcher_pid launcher_pgid launcher_identity reservation_token
  local harness_live=0 launcher_live=0 stale_record=0
  local -a legacy_identityless_records=()

  [[ -f "$STATE_FILE" ]] || return 0
  current_status=$(wh_state_get "$STATE_FILE" status 2>/dev/null || true)
  harness_pid=$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)
  harness_pgid=$(wh_state_get "$STATE_FILE" harness_pgid 2>/dev/null || true)
  harness_identity=$(wh_state_get "$STATE_FILE" harness_identity 2>/dev/null || true)
  launcher_pid=$(wh_state_get "$STATE_FILE" launcher_pid 2>/dev/null || true)
  launcher_pgid=$(wh_state_get "$STATE_FILE" launcher_pgid 2>/dev/null || true)
  launcher_identity=$(wh_state_get "$STATE_FILE" launcher_identity 2>/dev/null || true)
  reservation_token=$(wh_state_get "$STATE_FILE" port_reservation_token 2>/dev/null || true)

  if [[ "$current_status" == "ready" ]]; then
    if [[ -z "$harness_identity" ]] && {
      { [[ -n "$harness_pid" ]] && wh_pid_alive "$harness_pid"; } ||
      { [[ -n "$harness_pgid" ]] && wh_process_group_alive "$harness_pgid"; }
    }; then
      legacy_identityless_records+=("harness")
    fi
    if [[ -z "$launcher_identity" ]] && {
      { [[ -n "$launcher_pid" ]] && wh_pid_alive "$launcher_pid"; } ||
      { [[ -n "$launcher_pgid" ]] && wh_process_group_alive "$launcher_pgid"; }
    }; then
      legacy_identityless_records+=("launcher")
    fi
    if [[ "${#legacy_identityless_records[@]}" -gt 0 ]]; then
      local legacy_reason="cannot validate live legacy ${legacy_identityless_records[*]} process record: stable identity unavailable; no signal sent; operator-owned recovery required; reservation retained"
      wh_state_write "$STATE_FILE" "{\"status\": \"failed\", \"phase\": \"cleanup_pending\", \"failure_reason\": $(json_quote "$legacy_reason") }"
      return 1
    fi
    if [[ -n "$harness_pid" ]]; then
      if wh_process_record_alive "$harness_pid" "$harness_pgid" "$harness_identity"; then
        harness_live=1
      elif wh_pid_alive "$harness_pid" || { [[ -n "$harness_pgid" ]] && wh_process_group_alive "$harness_pgid"; }; then
        stale_record=1
      fi
    fi
    if [[ -n "$launcher_pid" ]]; then
      if wh_process_record_alive "$launcher_pid" "$launcher_pgid" "$launcher_identity"; then
        launcher_live=1
      elif wh_pid_alive "$launcher_pid" || { [[ -n "$launcher_pgid" ]] && wh_process_group_alive "$launcher_pgid"; }; then
        stale_record=1
      fi
    fi
    if [[ "$stale_record" -ne 0 ]]; then
      wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "cleanup_failed", "failure_reason": "recorded process identity or process group no longer matches"}'
      return 1
    fi
    if [[ "$harness_live" -ne 1 || "$launcher_live" -ne 1 ]]; then
      if [[ "$harness_live" -eq 0 && "$launcher_live" -eq 0 ]]; then
        if [[ -n "$reservation_token" ]]; then
          if ! wh_release_ports_reservation "$HARNESS_ROOT" "$reservation_token" "$PORT_REGISTRY_COMMON_DIR"; then
            wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "cleanup_pending", "failure_reason": "stale ready state could not release its port reservation; reservation retained"}'
            return 1
          fi
        fi
        wh_state_write "$STATE_FILE" '{"status": "stopped", "phase": "stopped", "harness_pid": null, "harness_pgid": null, "harness_identity": null, "launcher_pid": null, "launcher_pgid": null, "launcher_identity": null, "port_reservation_token": null}'
        return 0
      fi
      wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "cleanup_failed", "failure_reason": "ready state requires live valid harness and launcher process records"}'
      return 1
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
  if [[ "$timeout_secs" -le 0 ]]; then
    echo "error: smoke timeout must be greater than zero" >&2
    exit 2
  fi
  local smoke_deadline_ms=$(( $(wh_clock_ms) + timeout_secs * 1000 ))
  wh_state_phase "$STATE_FILE" "smoke" "browser smoke in progress" "$smoke_deadline_ms"

  smoke_failed() {
    local reason=$1
    wh_state_write "$STATE_FILE" "$(python3 - "$reason" "$timeout_secs" <<'PY'
import json
import sys
print(json.dumps({"phase": "smoke_failed", "last_smoke_ok": False, "last_smoke_timeout_secs": int(sys.argv[2]), "last_smoke_error": sys.argv[1]}))
PY
)"
    echo "error: worktree harness smoke failed: $reason" >&2
    return 1
  }

  smoke_step() {
    local label=$1
    shift
    local now_ms remaining_ms rc
    now_ms=$(wh_clock_ms)
    remaining_ms=$(( smoke_deadline_ms - now_ms ))
    if (( remaining_ms <= 0 )); then
      smoke_failed "deadline exceeded before ${label}"
      return 1
    fi
    wh_state_progress "$STATE_FILE" "browser operation: $label ($remaining_ms ms remaining)"
    if wh_run_with_deadline "$smoke_deadline_ms" "$@"; then
      return 0
    else
      rc=$?
    fi
    if [[ "$rc" -eq 124 ]]; then
      smoke_failed "deadline exceeded during ${label}"
    else
      smoke_failed "${label} exited with status ${rc}"
    fi
    return 1
  }

  smoke_dir="$ARTIFACT_DIR/smoke-$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$smoke_dir"

  smoke_step "open" ab_open "$BROWSER_SESSION" 0 "$viewer_url" >>"$smoke_dir/agent-browser.log" 2>&1 || return 1
  if ! smoke_step "wait for networkidle" ab_cmd "$BROWSER_SESSION" wait --load networkidle >>"$smoke_dir/agent-browser.log" 2>&1; then
    cat "$smoke_dir/agent-browser.log" >&2
    return 1
  fi
  wh_state_progress "$STATE_FILE" "browser operation: evaluate state" "0"
  if state_raw="$(wh_run_with_deadline "$smoke_deadline_ms" ab_eval "$BROWSER_SESSION" 'JSON.stringify(window.__AW_TEST__ ? window.__AW_TEST__.getState() : null)')"; then
    :
  else
    local eval_rc=$?
    if [[ "$eval_rc" -eq 124 ]]; then
      smoke_failed "deadline exceeded during state evaluation"
    else
      smoke_failed "state evaluation exited with status $eval_rc"
    fi
    cat "$smoke_dir/agent-browser.log" >&2
    return 1
  fi
  if [[ -z "$state_raw" || "$state_raw" == "null" ]]; then
    smoke_failed "__AW_TEST__.getState() returned empty payload"
    cat "$smoke_dir/agent-browser.log" >&2
    return 1
  fi
  json_to_file "$state_raw" "$smoke_dir/state.json"
  smoke_step "screenshot" ab_screenshot "$BROWSER_SESSION" "$smoke_dir/final.png" >>"$smoke_dir/agent-browser.log" 2>&1 || return 1
  wh_state_write "$STATE_FILE" "{\"phase\": \"ready\", \"last_smoke_dir\": $(json_quote "$smoke_dir"), \"last_smoke_ok\": true, \"last_smoke_timeout_secs\": $timeout_secs, \"last_smoke_deadline_epoch_ms\": $smoke_deadline_ms}"
  printf '%s\n' "$smoke_dir"
}

case "$action" in
  up)
    ENABLE_LLM="1"
    BOOT_MODE="source"
    STARTUP_TIMEOUT=300
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
        --startup-timeout)
          STARTUP_TIMEOUT="${2:-}"
          shift 2
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
    [[ "$STARTUP_TIMEOUT" =~ ^[1-9][0-9]*$ ]] || { echo "error: --startup-timeout must be a positive integer" >&2; exit 2; }
    [[ "$SMOKE_TIMEOUT" =~ ^[0-9]+$ ]] || { echo "error: --smoke-timeout must be a non-negative integer" >&2; exit 2; }
    if [[ "$ENABLE_LLM" != "1" ]]; then
      echo "error: worktree harness now boots through ./scripts/run-launcher-stack.sh and oasis7_game_launcher, both of which require active LLM access" >&2
      echo "hint: use direct oasis7_viewer_live --no-llm only for observer/debug diagnostics outside the launcher stack" >&2
      exit 2
    fi

    if wh_process_record_alive \
      "$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)" \
      "$(wh_state_get "$STATE_FILE" harness_pgid 2>/dev/null || true)" \
      "$(wh_state_get "$STATE_FILE" harness_identity 2>/dev/null || true)" && \
      wh_process_record_alive \
      "$(wh_state_get "$STATE_FILE" launcher_pid 2>/dev/null || true)" \
      "$(wh_state_get "$STATE_FILE" launcher_pgid 2>/dev/null || true)" \
      "$(wh_state_get "$STATE_FILE" launcher_identity 2>/dev/null || true)"; then
      echo "info: harness already running for $WORKTREE_ID"
      wh_state_show "$STATE_FILE"
      exit 0
    fi

    if ! kill_recorded_processes; then
      write_cleanup_failure_state "unable to prove previous harness quiescence"
      exit 1
    fi
    rm -f "$META_FILE" "$STARTUP_LOG"

    ports_json=$(wh_resolve_ports_json "$HARNESS_ROOT" "" "$(wh_worktree_path)" "$PORT_REGISTRY_COMMON_DIR")
    viewer_port=$(json_get "$ports_json" viewer_port)
    web_bind=$(json_get "$ports_json" web_bind)
    live_bind=$(json_get "$ports_json" live_bind)
    chain_status_bind=$(json_get "$ports_json" chain_status_bind)
    PORT_RESERVATION_TOKEN=$(json_get "$ports_json" reservation_token)
    STARTUP_DEADLINE_MS=$(( $(wh_clock_ms) + STARTUP_TIMEOUT * 1000 ))

    wh_state_write "$STATE_FILE" "$(python3 - \
      "$WORKTREE_ID" \
      "$PWD" \
      "$GIT_HEAD" \
      "$BOOT_MODE" \
      "$ENABLE_LLM" \
      "$viewer_port" \
      "$web_bind" \
      "$live_bind" \
      "$chain_status_bind" \
      "$BUNDLE_DIR" \
      "$RUNTIME_DIR" \
      "$ARTIFACT_DIR" \
      "$BROWSER_DIR" \
      "$BROWSER_SESSION" \
      "$STARTUP_LOG" \
      "$STARTUP_TIMEOUT" \
      "$STARTUP_DEADLINE_MS" \
      "$PORT_RESERVATION_TOKEN" \
      "$(json_get "$ports_json" reservation_file)" <<'PY'
import json
import sys
payload = {
    "worktree_id": sys.argv[1],
    "worktree_path": sys.argv[2],
    "git_head": sys.argv[3],
    "status": "booting",
    "boot_mode": sys.argv[4],
    "llm_enabled": sys.argv[5],
    "viewer_port": int(sys.argv[6]),
    "web_bind": sys.argv[7],
    "live_bind": sys.argv[8],
    "chain_status_bind": sys.argv[9],
    "bundle_dir": sys.argv[10],
    "runtime_dir": sys.argv[11],
    "artifact_dir": sys.argv[12],
    "browser_dir": sys.argv[13],
    "browser_session": sys.argv[14],
    "startup_log": sys.argv[15],
    "startup_timeout_secs": int(sys.argv[16]),
    "startup_deadline_epoch_ms": int(sys.argv[17]),
    "phase": "preparing",
    "phase_started_epoch_ms": int(sys.argv[17]) - int(sys.argv[16]) * 1000,
    "phase_deadline_epoch_ms": int(sys.argv[17]),
    "progress": {
        "phase": "preparing",
        "message": "resolving worktree ports",
        "updated_epoch_ms": int(sys.argv[17]) - int(sys.argv[16]) * 1000,
    },
    "port_reservation_token": sys.argv[18],
    "port_reservation_file": sys.argv[19],
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

    if [[ "$BOOT_MODE" == "bundle" ]]; then
      wh_state_phase "$STATE_FILE" "building_bundle" "checking worktree-local bundle freshness" "$STARTUP_DEADLINE_MS"
      if [[ ! -x "$BUNDLE_DIR/run-game.sh" ]] || ! bundle_check_freshness "$ROOT_DIR" "$BUNDLE_DIR" dev native >/dev/null 2>&1; then
        wh_state_progress "$STATE_FILE" "building fresh worktree-local launcher bundle" "0"
        if wh_run_with_deadline "$STARTUP_DEADLINE_MS" ./scripts/build-game-launcher-bundle.sh --profile dev --out-dir "$BUNDLE_DIR" >>"$STARTUP_LOG" 2>&1; then
          :
        else
          build_rc=$?
          wh_state_write "$STATE_FILE" "{\"status\": \"failed\", \"phase\": \"failed\", \"failure_reason\": \"bundle build exited with status $build_rc\"}"
          exit "$build_rc"
        fi
      fi
      bundle_args=(--bundle-dir "$BUNDLE_DIR" --bundle-profile dev --bundle-target-triple native)
    else
      wh_state_phase "$STATE_FILE" "starting_launcher" "preparing source launcher stack" "$STARTUP_DEADLINE_MS"
      bundle_args=()
    fi

    run_args=()
    if [[ "$BOOT_MODE" == "bundle" ]]; then
      run_args+=("${bundle_args[@]}")
    fi
    run_args+=(
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

    wh_state_phase "$STATE_FILE" "starting_launcher" "launching run-launcher-stack.sh" "$STARTUP_DEADLINE_MS"
    launch_stack() {
      nohup ./scripts/run-launcher-stack.sh "${run_args[@]}"
    }
    wh_start_managed launch_stack >"$STARTUP_LOG" 2>&1
    HARNESS_PID=$WH_MANAGED_PID
    HARNESS_PGID=$WH_MANAGED_PGID
    HARNESS_IDENTITY=$WH_MANAGED_IDENTITY
    wh_state_write "$STATE_FILE" "{\"harness_pid\": $HARNESS_PID, \"harness_pgid\": $HARNESS_PGID, \"harness_identity\": \"$HARNESS_IDENTITY\"}"
    if [[ -n "${OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE:-}" ]]; then
      : >"$OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_FILE"
      if [[ -n "${OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACK_FILE:-}" ]]; then
        while [[ ! -e "$OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACK_FILE" ]]; do
          if (( $(wh_clock_ms) >= STARTUP_DEADLINE_MS )); then
            wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "failed", "failure_reason": "test launch synchronization acknowledgement deadline exceeded"}'
            if ! kill_recorded_processes; then
              write_cleanup_failure_state "test launch synchronization acknowledgement cleanup failed"
            fi
            exit 1
          fi
          sleep 0.05
        done
        if [[ -n "${OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACKED_FILE:-}" ]]; then
          : >"$OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_ACKED_FILE"
        fi
      fi
      sleep "${OASIS7_HARNESS_TEST_DELAY_AFTER_LAUNCH_SECS:-1}"
    fi
    if ! wh_bind_ports_owner "$HARNESS_ROOT" "$PORT_RESERVATION_TOKEN" "$$" "$HARNESS_PID" "$PORT_REGISTRY_COMMON_DIR"; then
      if ! kill_recorded_processes; then
        write_cleanup_failure_state "unable to bind port reservation to managed harness process; cleanup failed"
      else
        wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "failed", "failure_reason": "unable to bind port reservation to managed harness process"}'
      fi
      exit 1
    fi

    wh_state_phase "$STATE_FILE" "waiting_metadata" "waiting for STACK_READY metadata" "$STARTUP_DEADLINE_MS"
    attempt=0
    while (( $(wh_clock_ms) < STARTUP_DEADLINE_MS )); do
      recorded_harness_pid=$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)
      recorded_harness_pgid=$(wh_state_get "$STATE_FILE" harness_pgid 2>/dev/null || true)
      recorded_harness_identity=$(wh_state_get "$STATE_FILE" harness_identity 2>/dev/null || true)
      attempt=$((attempt + 1))
      if ! wh_process_record_alive "$recorded_harness_pid" "$recorded_harness_pgid" "$recorded_harness_identity"; then
        wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "failed", "failure_reason": "run-launcher-stack.sh exited before STACK_READY"}'
        if ! kill_recorded_processes; then
          write_cleanup_failure_state "run-launcher-stack.sh exited before STACK_READY; cleanup failed"
        fi
        echo "error: worktree harness boot failed; run-launcher-stack.sh exited unexpectedly" >&2
        tail -n 120 "$STARTUP_LOG" >&2 || true
        exit 1
      fi
      if [[ -f "$META_FILE" ]]; then
        persist_launcher_record_from_meta || true
        stack_ready=$(wh_env_file_get "$META_FILE" STACK_READY 2>/dev/null || true)
        if [[ "$stack_ready" == "1" ]]; then
          break
        fi
      fi
      remaining_ms=$(( STARTUP_DEADLINE_MS - $(wh_clock_ms) ))
      wh_state_progress "$STATE_FILE" "waiting for STACK_READY metadata (${remaining_ms}ms remaining)" "$attempt"
      sleep 1
    done

    # The metadata can become ready after the startup deadline while a test
    # synchronization hook is holding the handoff. Revalidate the recorded
    # harness identity before publishing ready, even when the deadline loop
    # did not get another iteration.
    recorded_harness_pid=$(wh_state_get "$STATE_FILE" harness_pid 2>/dev/null || true)
    recorded_harness_pgid=$(wh_state_get "$STATE_FILE" harness_pgid 2>/dev/null || true)
    recorded_harness_identity=$(wh_state_get "$STATE_FILE" harness_identity 2>/dev/null || true)
    if ! wh_process_record_alive "$recorded_harness_pid" "$recorded_harness_pgid" "$recorded_harness_identity"; then
      wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "failed", "failure_reason": "run-launcher-stack.sh identity changed before STACK_READY"}'
      if ! kill_recorded_processes; then
        write_cleanup_failure_state "run-launcher-stack.sh identity changed before STACK_READY; cleanup failed"
      fi
      echo "error: worktree harness identity changed before readiness" >&2
      tail -n 120 "$STARTUP_LOG" >&2 || true
      exit 1
    fi

    if [[ ! -f "$META_FILE" ]] || [[ "$(wh_env_file_get "$META_FILE" STACK_READY 2>/dev/null || true)" != "1" ]]; then
      wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "failed", "failure_reason": "startup deadline exceeded waiting for STACK_READY"}'
      if ! kill_recorded_processes; then
        write_cleanup_failure_state "startup deadline exceeded waiting for STACK_READY; cleanup failed"
      fi
      echo "error: timed out waiting for worktree harness readiness" >&2
      tail -n 120 "$STARTUP_LOG" >&2 || true
      exit 1
    fi

    viewer_url=$(wh_env_file_get "$META_FILE" GAME_URL)
    launcher_pid=$(wh_env_file_get "$META_FILE" LAUNCHER_PID 2>/dev/null || true)
    launcher_pgid=$(wh_env_file_get "$META_FILE" LAUNCHER_PGID 2>/dev/null || true)
    launcher_identity=$(wh_env_file_get "$META_FILE" LAUNCHER_IDENTITY 2>/dev/null || true)
    if ! wh_process_record_alive "$launcher_pid" "$launcher_pgid" "$launcher_identity"; then
      wh_state_write "$STATE_FILE" '{"status": "failed", "phase": "failed", "failure_reason": "launcher process identity changed before readiness"}'
      if ! kill_recorded_processes; then
        write_cleanup_failure_state "launcher process identity changed before readiness; cleanup failed"
      fi
      echo "error: launcher identity changed before readiness" >&2
      tail -n 120 "$STARTUP_LOG" >&2 || true
      exit 1
    fi
    wh_state_write "$STATE_FILE" "$(python3 - \
      "$viewer_url" \
      "$launcher_pid" \
      "$launcher_pgid" \
      "$launcher_identity" \
      "$META_FILE" \
      "$PORT_RESERVATION_TOKEN" <<'PY'
import json
import sys
launcher_pid = sys.argv[2]
launcher_identity = sys.argv[4]
payload = {
    "status": "ready",
    "phase": "ready",
    "viewer_url": sys.argv[1],
    "launcher_pid": int(launcher_pid) if launcher_pid else None,
    "launcher_pgid": int(sys.argv[3]) if sys.argv[3] else None,
    "launcher_identity": launcher_identity or None,
    "meta_file": sys.argv[5],
    "port_reservation_token": sys.argv[6],
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
    if ! kill_recorded_processes; then
      write_cleanup_failure_state "unable to prove harness quiescence; reservation retained"
      echo "error: worktree harness shutdown did not reach quiescence; reservation retained" >&2
      exit 1
    fi
    wh_state_write "$STATE_FILE" '{"status": "stopped", "phase": "stopped", "harness_pid": null, "harness_pgid": null, "harness_identity": null, "launcher_pid": null, "launcher_pgid": null, "launcher_identity": null, "port_reservation_token": null}'
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
