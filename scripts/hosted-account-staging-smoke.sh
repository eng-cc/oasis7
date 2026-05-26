#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/cargo-dev-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/hosted-account-staging-smoke.sh [options]

Run a repo-owned hosted account smoke against `oasis7_game_launcher`.

Default (`--mode local`) behavior:
1. start a local `hosted_public_join` launcher with
   `OASIS7_HOSTED_LOGIN_DELIVERY_MODE=preview_inline`
2. complete one email login and capture `hosted_account_id/player_id`
3. release the issued player slot
4. restart the launcher against the same hosted account store
5. complete the same login again and assert the same account/player ids

`--mode staging` reuses the same flow but expects a real OTP fetch command so
the smoke can complete `/login/start -> /login/complete` with
`OASIS7_HOSTED_LOGIN_DELIVERY_MODE=smtp` and the staging store backend.

Options:
  --mode <local|staging>         Smoke mode (default: local)
  --login-handle <email>         Login handle for both passes
                                 (default local: player@example.com)
  --otp-fetch-command <cmd>      Command that prints the latest OTP for the
                                 current challenge when preview_code is absent
  --otp-timeout <secs>           Wait timeout for OTP fetch command (default: 90)
  --startup-timeout <secs>       Wait timeout for launcher HTTP listener (default: 120)
  --out-dir <path>               Artifact root (default: output/playwright/hosted-account)
  --viewer-host <host>           Viewer host bind (default: 127.0.0.1)
  --viewer-port <port>           Viewer HTTP port (default: 6411)
  --web-bind <host:port>         Viewer WS bind (default: 127.0.0.1:6412)
  --live-bind <host:port>        Runtime live bind (default: 127.0.0.1:6413)
  --viewer-static-dir <path>     Viewer static dir (default: crates/oasis7_viewer)
  --delivery-mode <mode>         Override hosted login delivery mode.
                                 Defaults: local=preview_inline, staging=smtp
  --store-backend <mode>         Override hosted account store backend.
                                 Defaults: local=file, staging=inherit
  --launcher-bin <path>          Reuse an existing launcher binary
  --viewer-live-bin <path>       Reuse an existing oasis7_viewer_live binary
  --skip-build                   Do not build launcher before smoke
  -h, --help                     Show this help

Examples:
  ./scripts/hosted-account-staging-smoke.sh
  ./scripts/hosted-account-staging-smoke.sh --mode staging \
    --login-handle qa-staging@example.com \
    --otp-fetch-command 'scripts/read-staging-otp.sh'
USAGE
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

is_tcp_listener_open() {
  local host=$1
  local port=$2
  wait_for_tcp_listener "$host" "$port" 1
}

require_tool() {
  local tool=$1
  command -v "$tool" >/dev/null 2>&1 || {
    echo "error: required tool is missing: $tool" >&2
    exit 1
  }
}

json_field() {
  local path=$1
  local filter=$2
  jq -r "$filter" "$path"
}

urlencode() {
  local value=$1
  jq -rn --arg value "$value" '$value | @uri'
}

extract_otp_code() {
  local raw=${1:-}
  printf '%s' "$raw" | tr -d '\r' | grep -Eo '[0-9]{6}' | tail -n 1 || true
}

run_id="hosted-account-smoke-$(date +%Y%m%d-%H%M%S)"
mode="local"
login_handle="${OASIS7_HOSTED_ACCOUNT_SMOKE_LOGIN_HANDLE:-player@example.com}"
otp_fetch_command="${OASIS7_HOSTED_ACCOUNT_OTP_FETCH_COMMAND:-}"
otp_timeout_secs=90
startup_timeout_secs=120
out_root="output/playwright/hosted-account"
viewer_host="127.0.0.1"
viewer_port="6411"
web_bind="127.0.0.1:6412"
live_bind="127.0.0.1:6413"
viewer_static_dir="crates/oasis7_viewer"
delivery_mode=""
store_backend=""
launcher_bin=""
viewer_live_bin=""
skip_build=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      mode="${2:-}"
      shift 2
      ;;
    --login-handle)
      login_handle="${2:-}"
      shift 2
      ;;
    --otp-fetch-command)
      otp_fetch_command="${2:-}"
      shift 2
      ;;
    --otp-timeout)
      otp_timeout_secs="${2:-}"
      shift 2
      ;;
    --startup-timeout)
      startup_timeout_secs="${2:-}"
      shift 2
      ;;
    --out-dir)
      out_root="${2:-}"
      shift 2
      ;;
    --viewer-host)
      viewer_host="${2:-}"
      shift 2
      ;;
    --viewer-port)
      viewer_port="${2:-}"
      shift 2
      ;;
    --web-bind)
      web_bind="${2:-}"
      shift 2
      ;;
    --live-bind)
      live_bind="${2:-}"
      shift 2
      ;;
    --viewer-static-dir)
      viewer_static_dir="${2:-}"
      shift 2
      ;;
    --delivery-mode)
      delivery_mode="${2:-}"
      shift 2
      ;;
    --store-backend)
      store_backend="${2:-}"
      shift 2
      ;;
    --launcher-bin)
      launcher_bin="${2:-}"
      shift 2
      ;;
    --viewer-live-bin)
      viewer_live_bin="${2:-}"
      shift 2
      ;;
    --skip-build)
      skip_build=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ "$mode" == "local" || "$mode" == "staging" ]] || {
  echo "error: --mode must be local or staging" >&2
  exit 2
}
[[ "$otp_timeout_secs" =~ ^[0-9]+$ ]] && [[ "$otp_timeout_secs" -gt 0 ]] || {
  echo "error: --otp-timeout must be a positive integer" >&2
  exit 2
}
[[ "$startup_timeout_secs" =~ ^[0-9]+$ ]] && [[ "$startup_timeout_secs" -gt 0 ]] || {
  echo "error: --startup-timeout must be a positive integer" >&2
  exit 2
}
[[ "$viewer_port" =~ ^[0-9]+$ ]] && [[ "$viewer_port" -gt 0 ]] || {
  echo "error: --viewer-port must be a positive integer" >&2
  exit 2
}
[[ -n "$login_handle" ]] || {
  echo "error: --login-handle cannot be empty" >&2
  exit 2
}

if [[ -z "$delivery_mode" ]]; then
  if [[ "$mode" == "local" ]]; then
    delivery_mode="preview_inline"
  else
    delivery_mode="smtp"
  fi
fi

if [[ -z "$store_backend" ]]; then
  if [[ "$mode" == "local" ]]; then
    store_backend="file"
  else
    store_backend="inherit"
  fi
fi

require_tool python3
require_tool jq
require_tool curl

run_dir="$out_root/$run_id"
mkdir -p "$run_dir"

summary_json_path="$run_dir/hosted-account-smoke-summary.json"
summary_md_path="$run_dir/hosted-account-smoke-summary.md"
launcher_first_log="$run_dir/launcher-first.log"
launcher_second_log="$run_dir/launcher-second.log"
login_start_first_path="$run_dir/login-start-first.json"
login_complete_first_path="$run_dir/login-complete-first.json"
release_first_path="$run_dir/release-first.json"
login_start_second_path="$run_dir/login-start-second.json"
login_complete_second_path="$run_dir/login-complete-second.json"
release_second_path="$run_dir/release-second.json"
local_store_path="$run_dir/hosted-account-store.json"

if [[ -z "$launcher_bin" ]]; then
  launcher_bin="$(oasis7_cargo_dev_debug_bin_dir "$repo_root")/oasis7_game_launcher"
fi
if [[ -z "$viewer_live_bin" ]]; then
  viewer_live_bin="$(oasis7_cargo_dev_debug_bin_dir "$repo_root")/oasis7_viewer_live"
fi

if [[ "$skip_build" != "1" ]]; then
  OASIS7_CARGO_DEV_REPO_ROOT="$repo_root" oasis7_cargo_dev build -q -p oasis7 --bin oasis7_game_launcher --bin oasis7_viewer_live
fi

[[ -x "$launcher_bin" ]] || {
  echo "error: expected launcher binary at $launcher_bin" >&2
  exit 1
}
[[ -x "$viewer_live_bin" ]] || {
  echo "error: expected viewer_live binary at $viewer_live_bin" >&2
  exit 1
}
[[ -d "$viewer_static_dir" ]] || {
  echo "error: viewer static dir does not exist: $viewer_static_dir" >&2
  exit 1
}

launcher_pid=""
cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" >/dev/null 2>&1; then
    kill "$launcher_pid" >/dev/null 2>&1 || true
    wait "$launcher_pid" >/dev/null 2>&1 || true
  fi
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

start_launcher() {
  local log_path=$1
  is_tcp_listener_open "$viewer_host" "$viewer_port" && {
    echo "error: launcher HTTP port is already in use before start: ${viewer_host}:${viewer_port}" >&2
    return 1
  }

  local -a env_cmd=(env)
  env_cmd+=("OASIS7_HOSTED_LOGIN_DELIVERY_MODE=$delivery_mode")
  env_cmd+=("OASIS7_VIEWER_LIVE_BIN=$viewer_live_bin")
  if [[ "$store_backend" != "inherit" ]]; then
    env_cmd+=("OASIS7_HOSTED_ACCOUNT_STORE_BACKEND=$store_backend")
  fi
  if [[ "$store_backend" == "file" ]]; then
    env_cmd+=("OASIS7_HOSTED_ACCOUNT_STORE_PATH=$local_store_path")
  fi
  "${env_cmd[@]}" \
    "$launcher_bin" \
    --deployment-mode hosted_public_join \
    --viewer-static-dir "$viewer_static_dir" \
    --viewer-host "$viewer_host" \
    --viewer-port "$viewer_port" \
    --web-bind "$web_bind" \
    --live-bind "$live_bind" \
    --chain-disable \
    --no-open-browser >"$log_path" 2>&1 &
  launcher_pid=$!
  local step
  for step in $(seq 1 "$startup_timeout_secs"); do
    if ! kill -0 "$launcher_pid" >/dev/null 2>&1; then
      echo "error: launcher exited before opening ${viewer_host}:${viewer_port}" >&2
      tail -n 120 "$log_path" >&2 || true
      return 1
    fi
    if is_tcp_listener_open "$viewer_host" "$viewer_port"; then
      return 0
    fi
    sleep 1
  done

  echo "error: timeout waiting for launcher HTTP listener on ${viewer_host}:${viewer_port}" >&2
  tail -n 120 "$log_path" >&2 || true
  return 1
}

stop_launcher() {
  if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" >/dev/null 2>&1; then
    kill "$launcher_pid" >/dev/null 2>&1 || true
    wait "$launcher_pid" >/dev/null 2>&1 || true
  fi
  launcher_pid=""
}

http_base="http://${viewer_host}:${viewer_port}"

post_json() {
  local route=$1
  local payload=$2
  local out_path=$3
  curl -fsS \
    -H 'content-type: application/json' \
    -X POST \
    -d "$payload" \
    "$http_base$route" >"$out_path"
}

post_query() {
  local route=$1
  local query=$2
  local out_path=$3
  curl -fsS \
    -X POST \
    "$http_base$route?$query" >"$out_path"
}

resolve_otp_code() {
  local start_path=$1
  local attempt_label=$2
  local preview_code=""
  preview_code=$(json_field "$start_path" '.challenge.preview_code // empty')
  if [[ -n "$preview_code" && "$preview_code" != "null" ]]; then
    printf '%s\n' "$preview_code"
    return 0
  fi

  [[ -n "$otp_fetch_command" ]] || {
    echo "error: login start response did not include preview_code and no --otp-fetch-command was provided" >&2
    return 1
  }

  local challenge_id=""
  challenge_id=$(json_field "$start_path" '.challenge.challenge_id // empty')
  [[ -n "$challenge_id" && "$challenge_id" != "null" ]] || {
    echo "error: login start response is missing challenge_id" >&2
    return 1
  }

  local deadline=$(( $(date +%s) + otp_timeout_secs ))
  while [[ $(date +%s) -lt $deadline ]]; do
    local fetch_output=""
    local status=0
    set +e
    fetch_output=$(
      HOSTED_ACCOUNT_LOGIN_HANDLE="$login_handle" \
      HOSTED_ACCOUNT_CHALLENGE_ID="$challenge_id" \
      HOSTED_ACCOUNT_LOGIN_ATTEMPT="$attempt_label" \
      HOSTED_ACCOUNT_SMOKE_RUN_DIR="$run_dir" \
      bash -lc "$otp_fetch_command" 2>"$run_dir/otp-fetch-${attempt_label}.stderr.log"
    )
    status=$?
    set -e
    if [[ "$status" == "0" ]]; then
      local otp=""
      otp=$(extract_otp_code "$fetch_output")
      if [[ -n "$otp" ]]; then
        printf '%s\n' "$otp"
        return 0
      fi
    fi
    sleep 5
  done

  echo "error: timed out waiting for OTP from --otp-fetch-command" >&2
  return 1
}

run_login_round() {
  local label=$1
  local start_path=$2
  local complete_path=$3
  local release_path=$4

  post_json \
    "/api/public/hosted-account/login/start" \
    "$(jq -cn --arg channel "email" --arg handle "$login_handle" \
      '{channel: $channel, handle: $handle}')" \
    "$start_path"

  local start_ok=""
  start_ok=$(json_field "$start_path" '.ok')
  [[ "$start_ok" == "true" ]] || {
    echo "error: login/start failed for $label" >&2
    cat "$start_path" >&2
    return 1
  }

  local otp_code=""
  otp_code=$(resolve_otp_code "$start_path" "$label")

  local challenge_id=""
  challenge_id=$(json_field "$start_path" '.challenge.challenge_id // empty')
  post_json \
    "/api/public/hosted-account/login/complete" \
    "$(jq -cn --arg challenge_id "$challenge_id" --arg otp_code "$otp_code" \
      '{challenge_id: $challenge_id, otp_code: $otp_code}')" \
    "$complete_path"

  local complete_ok=""
  complete_ok=$(json_field "$complete_path" '.ok')
  [[ "$complete_ok" == "true" ]] || {
    echo "error: login/complete failed for $label" >&2
    cat "$complete_path" >&2
    return 1
  }

  local player_id=""
  local release_token=""
  player_id=$(json_field "$complete_path" '.grant.player_id // empty')
  release_token=$(json_field "$complete_path" '.grant.release_token // empty')
  [[ -n "$player_id" && "$player_id" != "null" ]] || {
    echo "error: login/complete did not return grant.player_id for $label" >&2
    return 1
  }
  [[ -n "$release_token" && "$release_token" != "null" ]] || {
    echo "error: login/complete did not return grant.release_token for $label" >&2
    return 1
  }

  post_query \
    "/api/public/player-session/release" \
    "player_id=$(urlencode "$player_id")&release_token=$(urlencode "$release_token")" \
    "$release_path"
  local release_ok=""
  release_ok=$(json_field "$release_path" '.ok')
  [[ "$release_ok" == "true" ]] || {
    echo "error: player-session/release failed for $label" >&2
    cat "$release_path" >&2
    return 1
  }
}

start_launcher "$launcher_first_log"
run_login_round "first" "$login_start_first_path" "$login_complete_first_path" "$release_first_path"
stop_launcher

start_launcher "$launcher_second_log"
run_login_round "second" "$login_start_second_path" "$login_complete_second_path" "$release_second_path"
stop_launcher

first_account_id=$(json_field "$login_complete_first_path" '.account.hosted_account_id // empty')
second_account_id=$(json_field "$login_complete_second_path" '.account.hosted_account_id // empty')
first_player_id=$(json_field "$login_complete_first_path" '.account.player_id // empty')
second_player_id=$(json_field "$login_complete_second_path" '.account.player_id // empty')
first_delivery_mode=$(json_field "$login_start_first_path" '.challenge.delivery_mode // empty')
second_delivery_mode=$(json_field "$login_start_second_path" '.challenge.delivery_mode // empty')

same_account="false"
same_player="false"
if [[ -n "$first_account_id" && "$first_account_id" == "$second_account_id" ]]; then
  same_account="true"
fi
if [[ -n "$first_player_id" && "$first_player_id" == "$second_player_id" ]]; then
  same_player="true"
fi
overall_ok="false"
if [[ "$same_account" == "true" && "$same_player" == "true" ]]; then
  overall_ok="true"
fi

summary_json=$(
  jq -n \
    --arg run_id "$run_id" \
    --arg mode "$mode" \
    --arg login_handle "$login_handle" \
    --arg viewer_url "$http_base/" \
    --arg live_bind "$live_bind" \
    --arg web_bind "$web_bind" \
    --arg delivery_mode "$delivery_mode" \
    --arg store_backend "$store_backend" \
    --arg first_delivery_mode "$first_delivery_mode" \
    --arg second_delivery_mode "$second_delivery_mode" \
    --arg first_account_id "$first_account_id" \
    --arg second_account_id "$second_account_id" \
    --arg first_player_id "$first_player_id" \
    --arg second_player_id "$second_player_id" \
    --arg launcher_first_log "$launcher_first_log" \
    --arg launcher_second_log "$launcher_second_log" \
    --argjson ok "$overall_ok" \
    --argjson same_account "$same_account" \
    --argjson same_player "$same_player" \
    '{
      ok: $ok,
      run_id: $run_id,
      mode: $mode,
      login_handle: $login_handle,
      viewer_url: $viewer_url,
      live_bind: $live_bind,
      web_bind: $web_bind,
      delivery_mode_request: $delivery_mode,
      store_backend_request: $store_backend,
      first_delivery_mode: $first_delivery_mode,
      second_delivery_mode: $second_delivery_mode,
      first_account_id: $first_account_id,
      second_account_id: $second_account_id,
      first_player_id: $first_player_id,
      second_player_id: $second_player_id,
      continuity: {
        same_account_id: $same_account,
        same_player_id: $same_player
      },
      artifacts: {
        launcher_first_log: $launcher_first_log,
        launcher_second_log: $launcher_second_log
      }
    }'
)
printf '%s\n' "$summary_json" >"$summary_json_path"

cat >"$summary_md_path" <<EOF
# Hosted Account Smoke Summary

- run_id: \`$run_id\`
- mode: \`$mode\`
- login_handle: \`$login_handle\`
- requested_delivery_mode: \`$delivery_mode\`
- requested_store_backend: \`$store_backend\`
- first_delivery_mode: \`$first_delivery_mode\`
- second_delivery_mode: \`$second_delivery_mode\`
- first_account_id: \`$first_account_id\`
- second_account_id: \`$second_account_id\`
- first_player_id: \`$first_player_id\`
- second_player_id: \`$second_player_id\`
- same_account_id: \`$same_account\`
- same_player_id: \`$same_player\`
- overall_ok: \`$overall_ok\`
- launcher_first_log: \`$launcher_first_log\`
- launcher_second_log: \`$launcher_second_log\`
EOF

if [[ "$overall_ok" != "true" ]]; then
  echo "error: hosted account smoke failed continuity check" >&2
  cat "$summary_json_path" >&2
  exit 1
fi

echo "hosted-account-smoke: OK"
echo "- summary_json: $summary_json_path"
echo "- summary_md: $summary_md_path"
