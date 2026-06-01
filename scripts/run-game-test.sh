#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/agent-browser-lib.sh"
source "$ROOT_DIR/scripts/bundle-freshness-lib.sh"
source "$ROOT_DIR/scripts/cargo-dev-lib.sh"
source "$ROOT_DIR/scripts/hosted-login-gate-env-lib.sh"

VIEWER_HOST="127.0.0.1"
VIEWER_PORT="4173"
LIVE_BIND_ADDR="127.0.0.1:5023"
WEB_BRIDGE_ADDR="127.0.0.1:5011"
ENABLE_LLM="1"
SCENARIO="llm_bootstrap"
VIEWER_STATIC_DIR="web"
CHAIN_ENABLED="1"
CHAIN_NODE_ID=""
CHAIN_STATUS_BIND_ADDR=""
BUNDLE_DIR=""
VIEWER_STATIC_DIR_EXPLICIT="0"
ALLOW_STALE_BUNDLE="0"
OUTPUT_DIR=""
RUN_ID=""
META_FILE=""
JSON_READY="0"
SKIP_PROVIDER_PREFLIGHT="0"
REQUIRE_PLAYABLE_SNAPSHOT="0"
PLAYABLE_SNAPSHOT_TIMEOUT_SECS=45
AGENT_PROVIDER_URL="${OASIS7_AGENT_PROVIDER_URL:-https://t2t.oasis7.tech}"
AGENT_PROVIDER_AUTH_TOKEN="${OASIS7_AGENT_PROVIDER_AUTH_TOKEN:-}"
NEWAPI_USER_REF="${OASIS7_NEWAPI_USER_REF:-}"
BRIDGE_USER_ID="${OASIS7_BRIDGE_USER_ID:-}"
AGENT_PROVIDER_CONNECT_TIMEOUT_MS="${OASIS7_AGENT_PROVIDER_CONNECT_TIMEOUT_MS:-15000}"
AGENT_PROVIDER_PROFILE="${OASIS7_AGENT_PROVIDER_PROFILE:-oasis7_p0_low_freq_npc}"
AGENT_EXECUTION_LANE="${OASIS7_AGENT_EXECUTION_LANE:-player_parity}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/run-game-test.sh [options]

Start a stable web playability test stack with safe defaults.

Preferred producer/release path:
- ./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-local
- ./scripts/run-game-test.sh --bundle-dir output/release/game-launcher-local
- stale or manifest-less bundles fail fast unless `--allow-stale-bundle` is passed

Development fallback:
- source oasis7_game_launcher via cargo run with the same runtime defaults

Options:
  --scenario <name>        Launcher scenario (default: llm_bootstrap)
  --bundle-dir <path>      Use packaged bundle <path>/run-game.sh (recommended for producer/release playtests)
  --viewer-host <host>     Viewer HTTP host (default: 127.0.0.1)
  --viewer-port <port>     Viewer HTTP port (default: 4173)
  --live-bind <addr:port>  oasis7_game_launcher live TCP bind (default: 127.0.0.1:5023)
  --web-bind <addr:port>   WebSocket bridge bind (default: 127.0.0.1:5011)
  --viewer-static-dir <p>  Override viewer static dir; source mode defaults to fresh `web`, bundle mode only uses this as an advanced override
  --allow-stale-bundle    Skip workspace freshness guard for --bundle-dir (advanced / explicit override)
  --chain-enable           Enable chain runtime (default)
  --chain-disable          Disable chain runtime
  --chain-node-id <id>     Override chain node id (default: fresh per run)
  --chain-status-bind <a:p> Override chain status HTTP bind (default: web-bind port + 110)
  --output-dir <path>      Override runtime log/artifact output directory
  --run-id <id>            Override logical run id used for output dir / chain node id defaults
  --meta-file <path>       Override metadata file path (default: <output-dir>/session.meta)
  --json-ready             Emit one-line JSON ready payload after the stack becomes ready
  --require-playable-snapshot
                           Do not report ready until the primary Web Viewer connects and exposes
                           a non-empty agents/locations snapshot
  --playable-snapshot-timeout <secs>
                           Wait timeout for --require-playable-snapshot (default: 45)
  --llm-provider-url <url> Remote provider bridge URL (default: https://t2t.oasis7.tech)
  --newapi-user-ref <ref>  Resolve bearer via remote NewAPI bridge state as newapi_user_ref:<ref>
  --bridge-user-id <id>    Resolve bearer via remote NewAPI bridge state as bridge_user_id:<id>
  --agent-provider-auth-token <tok>
                           Explicit provider bearer token or supported bearer selector
  --agent-provider-profile <id>
                           Provider profile (default: oasis7_p0_low_freq_npc)
  --agent-provider-connect-timeout-ms <ms>
                           Provider connect timeout (default: 15000)
  --agent-execution-lane <mode>
                           Provider execution lane: player_parity|headless_agent (default: player_parity)
  --skip-provider-preflight
                           Skip remote provider contract preflight before launcher startup
  --with-llm               Enable provider-backed LLM mode (default; required for gameplay)
  --no-llm                 Negative-path only; this launcher stack now fails fast without LLM
  -h, --help               Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "error: --scenario requires a value" >&2
        usage >&2
        exit 1
      fi
      SCENARIO="$2"
      shift 2
      ;;
    --bundle-dir)
      BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    --viewer-host)
      VIEWER_HOST="${2:-}"
      shift 2
      ;;
    --viewer-port)
      VIEWER_PORT="${2:-}"
      shift 2
      ;;
    --live-bind)
      LIVE_BIND_ADDR="${2:-}"
      shift 2
      ;;
    --web-bind)
      WEB_BRIDGE_ADDR="${2:-}"
      shift 2
      ;;
    --viewer-static-dir)
      VIEWER_STATIC_DIR="${2:-}"
      VIEWER_STATIC_DIR_EXPLICIT="1"
      shift 2
      ;;
    --allow-stale-bundle)
      ALLOW_STALE_BUNDLE="1"
      shift
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --run-id)
      RUN_ID="${2:-}"
      shift 2
      ;;
    --meta-file)
      META_FILE="${2:-}"
      shift 2
      ;;
    --json-ready)
      JSON_READY="1"
      shift
      ;;
    --require-playable-snapshot)
      REQUIRE_PLAYABLE_SNAPSHOT="1"
      shift
      ;;
    --playable-snapshot-timeout)
      PLAYABLE_SNAPSHOT_TIMEOUT_SECS="${2:-}"
      shift 2
      ;;
    --llm-provider-url|--agent-provider-url)
      AGENT_PROVIDER_URL="${2:-}"
      shift 2
      ;;
    --newapi-user-ref)
      NEWAPI_USER_REF="${2:-}"
      shift 2
      ;;
    --bridge-user-id)
      BRIDGE_USER_ID="${2:-}"
      shift 2
      ;;
    --agent-provider-auth-token)
      AGENT_PROVIDER_AUTH_TOKEN="${2:-}"
      shift 2
      ;;
    --agent-provider-profile)
      AGENT_PROVIDER_PROFILE="${2:-}"
      shift 2
      ;;
    --agent-provider-connect-timeout-ms)
      AGENT_PROVIDER_CONNECT_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --agent-execution-lane)
      AGENT_EXECUTION_LANE="${2:-}"
      shift 2
      ;;
    --skip-provider-preflight)
      SKIP_PROVIDER_PREFLIGHT="1"
      shift
      ;;
    --chain-enable)
      CHAIN_ENABLED="1"
      shift
      ;;
    --chain-disable)
      CHAIN_ENABLED="0"
      shift
      ;;
    --chain-node-id)
      CHAIN_NODE_ID="${2:-}"
      shift 2
      ;;
    --chain-status-bind)
      CHAIN_STATUS_BIND_ADDR="${2:-}"
      shift 2
      ;;
    --with-llm)
      ENABLE_LLM="1"
      shift
      ;;
    --no-llm)
      ENABLE_LLM="0"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$VIEWER_HOST" || -z "$VIEWER_PORT" || -z "$LIVE_BIND_ADDR" || -z "$WEB_BRIDGE_ADDR" || -z "$VIEWER_STATIC_DIR" ]]; then
  echo "error: empty argument is not allowed" >&2
  exit 1
fi

if [[ -z "$SCENARIO" ]]; then
  echo "error: --scenario cannot be empty" >&2
  exit 1
fi

if [[ -z "$AGENT_PROVIDER_URL" ]]; then
  echo "error: --llm-provider-url cannot be empty" >&2
  exit 1
fi

if [[ -z "$AGENT_PROVIDER_PROFILE" ]]; then
  echo "error: --agent-provider-profile cannot be empty" >&2
  exit 1
fi

if ! [[ "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" =~ ^[0-9]+$ ]] || (( AGENT_PROVIDER_CONNECT_TIMEOUT_MS <= 0 )); then
  echo "error: --agent-provider-connect-timeout-ms must be a positive integer" >&2
  exit 1
fi

case "$AGENT_EXECUTION_LANE" in
  player_parity|headless_agent) ;;
  *)
    echo "error: --agent-execution-lane must be player_parity or headless_agent" >&2
    exit 1
    ;;
esac

selector_count=0
[[ -n "$AGENT_PROVIDER_AUTH_TOKEN" ]] && selector_count=$((selector_count + 1))
[[ -n "$NEWAPI_USER_REF" ]] && selector_count=$((selector_count + 1))
[[ -n "$BRIDGE_USER_ID" ]] && selector_count=$((selector_count + 1))
if (( selector_count > 1 )); then
  echo "error: choose only one of --agent-provider-auth-token, --newapi-user-ref, or --bridge-user-id" >&2
  exit 1
fi
if [[ -n "$NEWAPI_USER_REF" ]]; then
  AGENT_PROVIDER_AUTH_TOKEN="newapi_user_ref:${NEWAPI_USER_REF}"
elif [[ -n "$BRIDGE_USER_ID" ]]; then
  AGENT_PROVIDER_AUTH_TOKEN="bridge_user_id:${BRIDGE_USER_ID}"
fi
if [[ -z "$AGENT_PROVIDER_AUTH_TOKEN" && "$SKIP_PROVIDER_PREFLIGHT" != "1" ]]; then
  echo "error: provider-backed playtest requires --newapi-user-ref, --bridge-user-id, or --agent-provider-auth-token" >&2
  echo "hint: set OASIS7_NEWAPI_USER_REF for the cloud NewAPI bridge path, or pass --skip-provider-preflight only for mocked/negative stack diagnostics" >&2
  exit 1
fi

if ! [[ "$PLAYABLE_SNAPSHOT_TIMEOUT_SECS" =~ ^[0-9]+$ ]] || (( PLAYABLE_SNAPSHOT_TIMEOUT_SECS <= 0 )); then
  echo "error: --playable-snapshot-timeout must be a positive integer" >&2
  exit 1
fi

if ! [[ "$VIEWER_PORT" =~ ^[0-9]+$ ]]; then
  echo "error: --viewer-port must be numeric" >&2
  exit 1
fi

if [[ "$LIVE_BIND_ADDR" != *:* || "$WEB_BRIDGE_ADDR" != *:* ]]; then
  echo "error: --live-bind/--web-bind must be in <host:port> format" >&2
  exit 1
fi

LIVE_BIND_HOST="${LIVE_BIND_ADDR%:*}"
LIVE_BIND_PORT="${LIVE_BIND_ADDR##*:}"
WEB_BRIDGE_HOST="${WEB_BRIDGE_ADDR%:*}"
WEB_BRIDGE_PORT="${WEB_BRIDGE_ADDR##*:}"

if [[ -z "$LIVE_BIND_HOST" || -z "$LIVE_BIND_PORT" || -z "$WEB_BRIDGE_HOST" || -z "$WEB_BRIDGE_PORT" ]]; then
  echo "error: invalid bind address" >&2
  exit 1
fi

if ! [[ "$LIVE_BIND_PORT" =~ ^[0-9]+$ && "$WEB_BRIDGE_PORT" =~ ^[0-9]+$ ]]; then
  echo "error: bind ports must be numeric" >&2
  exit 1
fi

if [[ -n "$BUNDLE_DIR" ]]; then
  if [[ ! -d "$BUNDLE_DIR" ]]; then
    echo "error: --bundle-dir path does not exist: $BUNDLE_DIR" >&2
    exit 1
  fi
  BUNDLE_DIR="$(cd "$BUNDLE_DIR" && pwd)"
  if [[ ! -f "$BUNDLE_DIR/run-game.sh" ]]; then
    echo "error: bundle is missing run-game.sh: $BUNDLE_DIR" >&2
    exit 1
  fi
  if [[ "$ALLOW_STALE_BUNDLE" != "1" ]]; then
    if ! freshness_note=$(bundle_check_freshness "$ROOT_DIR" "$BUNDLE_DIR" 2>&1); then
      echo "error: $freshness_note" >&2
      echo "hint: rebuild via ./scripts/build-game-launcher-bundle.sh --out-dir $BUNDLE_DIR or rerun producer entry with --rebuild; use --allow-stale-bundle only when intentionally validating an older artifact" >&2
      exit 1
    fi
  fi
fi

if [[ "$ENABLE_LLM" != "1" ]]; then
  echo "error: ./scripts/run-game-test.sh now wraps oasis7_game_launcher, which requires active LLM access" >&2
  echo "hint: use env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --no-llm ... only for raw observer/debug diagnostics" >&2
  exit 1
fi

if [[ -n "$CHAIN_STATUS_BIND_ADDR" ]]; then
  if [[ "$CHAIN_STATUS_BIND_ADDR" != *:* ]]; then
    echo "error: --chain-status-bind must be in <host:port> format" >&2
    exit 1
  fi
  CHAIN_STATUS_BIND_HOST="${CHAIN_STATUS_BIND_ADDR%:*}"
  CHAIN_STATUS_BIND_PORT="${CHAIN_STATUS_BIND_ADDR##*:}"
  if [[ -z "$CHAIN_STATUS_BIND_HOST" || -z "$CHAIN_STATUS_BIND_PORT" ]]; then
    echo "error: invalid --chain-status-bind" >&2
    exit 1
  fi
  if ! [[ "$CHAIN_STATUS_BIND_PORT" =~ ^[0-9]+$ ]]; then
    echo "error: --chain-status-bind port must be numeric" >&2
    exit 1
  fi
else
  CHAIN_STATUS_BIND_HOST=""
  CHAIN_STATUS_BIND_PORT=""
fi

port_in_use() {
  local port="$1"
  if command -v lsof >/dev/null 2>&1; then
    lsof -iTCP:"$port" -sTCP:LISTEN -n -P >/dev/null 2>&1
    return $?
  fi

  if command -v ss >/dev/null 2>&1; then
    ss -ltn | grep -Eq "[:.]${port}[[:space:]]"
    return $?
  fi

  return 1
}

print_port_owner() {
  local port="$1"
  if command -v lsof >/dev/null 2>&1; then
    lsof -iTCP:"$port" -sTCP:LISTEN -n -P || true
  elif command -v ss >/dev/null 2>&1; then
    ss -ltnp | grep -E "[:.]${port}[[:space:]]" || true
  fi
}

check_port_free() {
  local port="$1"
  if port_in_use "$port"; then
    echo "error: port ${port} is already in use" >&2
    print_port_owner "$port" >&2
    exit 1
  fi
}

resolve_source_mode_target_dir() {
  oasis7_cargo_dev_debug_bin_dir "$ROOT_DIR"
}

ensure_launcher_alive() {
  local pid="$1"
  if [[ -n "$pid" ]] && ! kill -0 "$pid" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

wait_for_http_ready() {
  local url="$1"
  local timeout_secs="$2"
  local launcher_pid="${3:-}"
  local i
  for ((i = 0; i < timeout_secs; i++)); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    if ! ensure_launcher_alive "$launcher_pid"; then
      return 2
    fi
    sleep 1
  done
  return 1
}

wait_for_tcp_listener_ready() {
  local port="$1"
  local timeout_secs="$2"
  local launcher_pid="${3:-}"
  local i
  if ! command -v lsof >/dev/null 2>&1 && ! command -v ss >/dev/null 2>&1; then
    echo "warning: neither lsof nor ss found; skip passive listener probe for port ${port}" >&2
    return 0
  fi
  for ((i = 0; i < timeout_secs; i++)); do
    if port_in_use "$port"; then
      return 0
    fi
    if ! ensure_launcher_alive "$launcher_pid"; then
      return 2
    fi
    sleep 1
  done
  return 1
}

probe_playable_viewer_state() {
  local session=$1
  ab_eval "$session" '(() => {
    const state = window.__AW_TEST__?.getState?.() ?? null;
    const snapshot = state?.snapshot ?? null;
    const agents = snapshot?.model?.agents ? Object.keys(snapshot.model.agents).length : 0;
    const locations = snapshot?.model?.locations ? Object.keys(snapshot.model.locations).length : 0;
    return {
      hasTestApi: typeof window.__AW_TEST__ === "object",
      connectionStatus: state?.connectionStatus ?? null,
      agents,
      locations,
      blockerKind: state?.gameplaySummary?.blockerKind ?? null,
      lastError: state?.lastError ?? null,
    };
  })()'
}

wait_for_playable_snapshot_ready() {
  local url=$1
  local timeout_secs=$2
  local launcher_pid=${3:-}
  local out_path=$4
  local session="run-game-test-ready-${RUN_ID}"
  local probe_state='null'
  local exit_code=1
  local i

  ab_require
  ab_run "$session" close >/dev/null 2>&1 || true
  trap 'ab_run "$session" close >/dev/null 2>&1 || true' RETURN
  ab_open "$session" 0 "$url"
  ab_cmd "$session" wait --load networkidle >/dev/null 2>&1 || true

  for ((i = 0; i < timeout_secs; i++)); do
    if ! ensure_launcher_alive "$launcher_pid"; then
      exit_code=2
      break
    fi
    probe_state=$(probe_playable_viewer_state "$session")
    json_to_file "$probe_state" "$out_path"
    if [[ "$(json_get "$probe_state" hasTestApi)" == "true" \
      && "$(json_get "$probe_state" connectionStatus)" == "connected" \
      && "$(json_get "$probe_state" agents)" =~ ^[0-9]+$ \
      && "$(json_get "$probe_state" locations)" =~ ^[0-9]+$ \
      && $(json_get "$probe_state" agents) -gt 0 \
      && $(json_get "$probe_state" locations) -gt 0 ]]; then
      exit_code=0
      break
    fi
    sleep 1
  done

  printf '%s\n' "$probe_state"
  return "$exit_code"
}

tail_logs_on_error() {
  echo "--- oasis7_viewer_live.log (tail) ---" >&2
  tail -n 80 "$WORLD_LOG" >&2 || true
  if [[ -s "$WEB_LOG" ]]; then
    echo "--- web_viewer.log (tail) ---" >&2
    tail -n 80 "$WEB_LOG" >&2 || true
  fi
}

run_provider_contract_preflight() {
  local provider_url="$1"
  local auth_token="$2"
  local timeout_ms="$3"
  local out_json="$4"
  local out_log="$5"
  local base_url="${provider_url%/}"
  local timeout_secs=$(( (timeout_ms + 999) / 1000 ))
  local curl_args=(-fsS --connect-timeout "$timeout_secs" --max-time "$timeout_secs")

  if [[ -n "$auth_token" ]]; then
    curl_args+=(-H "Authorization: Bearer ${auth_token}")
  fi

  curl "${curl_args[@]}" "${base_url}/v1/provider/info" >"$out_json" 2>"$out_log"
}

tail_provider_preflight_logs_on_error() {
  local probe_json="$1"
  local probe_log="$2"
  if [[ -f "$probe_json" ]]; then
    echo "--- provider-contract-preflight.json ---" >&2
    cat "$probe_json" >&2 || true
  fi
  if [[ -s "$probe_log" ]]; then
    echo "--- provider-contract-preflight.log ---" >&2
    tail -n 80 "$probe_log" >&2 || true
  fi
}

check_port_free "$VIEWER_PORT"
check_port_free "$WEB_BRIDGE_PORT"

if [[ -z "$RUN_ID" ]]; then
  RUN_ID="$(date +%Y%m%d-%H%M%S)"
fi
if [[ "$CHAIN_ENABLED" == "1" ]]; then
  if [[ -z "$CHAIN_STATUS_BIND_ADDR" ]]; then
    CHAIN_STATUS_BIND_PORT=$((WEB_BRIDGE_PORT + 110))
    if (( CHAIN_STATUS_BIND_PORT > 65535 )); then
      echo "error: derived --chain-status-bind port exceeds 65535" >&2
      exit 1
    fi
    CHAIN_STATUS_BIND_HOST="127.0.0.1"
    CHAIN_STATUS_BIND_ADDR="${CHAIN_STATUS_BIND_HOST}:${CHAIN_STATUS_BIND_PORT}"
  fi
  check_port_free "$CHAIN_STATUS_BIND_PORT"
  if [[ -z "$CHAIN_NODE_ID" ]]; then
    CHAIN_NODE_ID="viewer-live-node-playtest-${RUN_ID}"
  fi
fi
if [[ -n "$OUTPUT_DIR" ]]; then
  if [[ "$OUTPUT_DIR" != /* ]]; then
    OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR"
  fi
else
  OUTPUT_DIR="$ROOT_DIR/output/playwright/playability/startup-${RUN_ID}"
fi
mkdir -p "$OUTPUT_DIR"

if [[ -n "$BUNDLE_DIR" ]]; then
  if [[ "$VIEWER_STATIC_DIR_EXPLICIT" == "1" ]]; then
    if [[ "$VIEWER_STATIC_DIR" == /* ]]; then
      RESOLVED_VIEWER_STATIC_DIR="$VIEWER_STATIC_DIR"
    else
      RESOLVED_VIEWER_STATIC_DIR="$ROOT_DIR/$VIEWER_STATIC_DIR"
    fi
  else
    RESOLVED_VIEWER_STATIC_DIR=""
  fi
else
  RESOLVED_VIEWER_STATIC_DIR=$(resolve_viewer_static_dir_for_web_closure "$ROOT_DIR" "$VIEWER_STATIC_DIR" "$OUTPUT_DIR")
fi

WORLD_LOG="$OUTPUT_DIR/oasis7_viewer_live.log"
WEB_LOG="$OUTPUT_DIR/web_viewer.log"
PROVIDER_PREFLIGHT_JSON="$OUTPUT_DIR/provider-contract-preflight.json"
PROVIDER_PREFLIGHT_LOG="$OUTPUT_DIR/provider-contract-preflight.log"
PLAYABLE_SNAPSHOT_STATE_JSON="$OUTPUT_DIR/playable-snapshot-state.json"
HOSTED_ACCOUNT_STORE_PATH="$OUTPUT_DIR/hosted-account-store.json"
LAUNCHER_ENV_DEFAULTS=()
while IFS= read -r env_default; do
  LAUNCHER_ENV_DEFAULTS+=("$env_default")
done < <(oasis7_hosted_login_gate_env_defaults "$HOSTED_ACCOUNT_STORE_PATH")
if [[ -n "$META_FILE" ]]; then
  if [[ "$META_FILE" != /* ]]; then
    META_FILE="$ROOT_DIR/$META_FILE"
  fi
else
  META_FILE="$OUTPUT_DIR/session.meta"
fi
mkdir -p "$(dirname "$META_FILE")"

LAUNCHER_PID=""

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM

  if [[ -n "$LAUNCHER_PID" ]] && kill -0 "$LAUNCHER_PID" >/dev/null 2>&1; then
    kill "$LAUNCHER_PID" >/dev/null 2>&1 || true
  fi

  wait "$LAUNCHER_PID" >/dev/null 2>&1 || true

  exit "$exit_code"
}
trap cleanup EXIT INT TERM

WORLD_ARGS=(
  --scenario "$SCENARIO"
  --live-bind "$LIVE_BIND_ADDR"
  --web-bind "$WEB_BRIDGE_ADDR"
  --viewer-host "$VIEWER_HOST"
  --viewer-port "$VIEWER_PORT"
  --no-open-browser
)
if [[ -n "$RESOLVED_VIEWER_STATIC_DIR" ]]; then
  WORLD_ARGS+=(--viewer-static-dir "$RESOLVED_VIEWER_STATIC_DIR")
fi
if [[ "$CHAIN_ENABLED" == "1" ]]; then
  WORLD_ARGS+=(
    --chain-enable
    --chain-node-id "$CHAIN_NODE_ID"
    --chain-status-bind "$CHAIN_STATUS_BIND_ADDR"
  )
else
  WORLD_ARGS+=(--chain-disable)
fi
WORLD_ARGS+=(
  --with-llm
  --agent-decision-source provider_backed
  --agent-provider-backend provider_local_bridge
  --agent-provider-contract worldsim_provider_v1
  --agent-provider-transport remote_https
  --agent-provider-url "$AGENT_PROVIDER_URL"
  --agent-provider-connect-timeout-ms "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS"
  --agent-provider-profile "$AGENT_PROVIDER_PROFILE"
  --agent-execution-lane "$AGENT_EXECUTION_LANE"
)
if [[ -n "$AGENT_PROVIDER_AUTH_TOKEN" ]]; then
  WORLD_ARGS+=(--agent-provider-auth-token "$AGENT_PROVIDER_AUTH_TOKEN")
fi

if [[ "$SKIP_PROVIDER_PREFLIGHT" != "1" ]]; then
  if ! run_provider_contract_preflight \
    "$AGENT_PROVIDER_URL" \
    "$AGENT_PROVIDER_AUTH_TOKEN" \
    "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" \
    "$PROVIDER_PREFLIGHT_JSON" \
    "$PROVIDER_PREFLIGHT_LOG"; then
    echo "error: remote provider contract preflight failed before launcher startup" >&2
    echo "hint: verify --llm-provider-url and bridge auth selector; use --skip-provider-preflight only for mocked/negative stack diagnostics" >&2
    tail_provider_preflight_logs_on_error "$PROVIDER_PREFLIGHT_JSON" "$PROVIDER_PREFLIGHT_LOG"
    exit 1
  fi
fi

if [[ -n "$BUNDLE_DIR" ]]; then
  LAUNCH_MODE="bundle"
  LAUNCH_CMD="$BUNDLE_DIR/run-game.sh"
  (
    cd "$BUNDLE_DIR"
    env "${LAUNCHER_ENV_DEFAULTS[@]}" "$BUNDLE_DIR/run-game.sh" "${WORLD_ARGS[@]}" >"$WORLD_LOG" 2>&1
  ) &
else
  LAUNCH_MODE="source"
  SOURCE_MODE_TARGET_DIR="$(resolve_source_mode_target_dir)"
  SOURCE_MODE_LAUNCHER_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_game_launcher"
  SOURCE_MODE_VIEWER_LIVE_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_viewer_live"
  SOURCE_MODE_CHAIN_RUNTIME_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_chain_runtime"
  SOURCE_BUILD_ARGS=(
    build
    -p
    oasis7
    --bin
    oasis7_game_launcher
    --bin
    oasis7_viewer_live
  )
  if [[ "$CHAIN_ENABLED" == "1" ]]; then
    SOURCE_BUILD_ARGS+=(--bin oasis7_chain_runtime)
  fi
  oasis7_cargo_dev "${SOURCE_BUILD_ARGS[@]}"
  [[ -x "$SOURCE_MODE_LAUNCHER_BIN" ]] || { echo "error: built launcher binary missing: $SOURCE_MODE_LAUNCHER_BIN" >&2; exit 1; }
  [[ -x "$SOURCE_MODE_VIEWER_LIVE_BIN" ]] || { echo "error: built viewer live binary missing: $SOURCE_MODE_VIEWER_LIVE_BIN" >&2; exit 1; }
  if [[ "$CHAIN_ENABLED" == "1" ]]; then
    [[ -x "$SOURCE_MODE_CHAIN_RUNTIME_BIN" ]] || { echo "error: built chain runtime binary missing: $SOURCE_MODE_CHAIN_RUNTIME_BIN" >&2; exit 1; }
  fi
  LAUNCH_CMD="$SOURCE_MODE_LAUNCHER_BIN"
  (
    cd "$ROOT_DIR"
    env "${LAUNCHER_ENV_DEFAULTS[@]}" \
    OASIS7_VIEWER_LIVE_BIN="$SOURCE_MODE_VIEWER_LIVE_BIN" \
    OASIS7_CHAIN_RUNTIME_BIN="$SOURCE_MODE_CHAIN_RUNTIME_BIN" \
    "$SOURCE_MODE_LAUNCHER_BIN" "${WORLD_ARGS[@]}" >"$WORLD_LOG" 2>&1
  ) &
fi
LAUNCHER_PID=$!
cat <<'INFO' >"$WEB_LOG"
run-viewer-web.sh no longer runs as a standalone process in this stack.
web viewer is served by oasis7_game_launcher built-in static server.
INFO

{
  echo "RUN_ID=$RUN_ID"
  echo "OUTPUT_DIR=$OUTPUT_DIR"
  echo "WORLD_PID=$LAUNCHER_PID"
  echo "WEB_PID="
  echo "LAUNCHER_PID=$LAUNCHER_PID"
  echo "LIVE_BIND_ADDR=$LIVE_BIND_ADDR"
  echo "WEB_BRIDGE_ADDR=$WEB_BRIDGE_ADDR"
  echo "VIEWER_HOST=$VIEWER_HOST"
  echo "VIEWER_PORT=$VIEWER_PORT"
  echo "CHAIN_ENABLED=$CHAIN_ENABLED"
  echo "CHAIN_NODE_ID=$CHAIN_NODE_ID"
  echo "CHAIN_STATUS_BIND_ADDR=$CHAIN_STATUS_BIND_ADDR"
  echo "LAUNCH_MODE=$LAUNCH_MODE"
  echo "LAUNCH_CMD=$LAUNCH_CMD"
  echo "BUNDLE_DIR=$BUNDLE_DIR"
  echo "AGENT_DECISION_SOURCE=provider_backed"
  echo "AGENT_PROVIDER_TRANSPORT=remote_https"
  echo "AGENT_PROVIDER_URL=$AGENT_PROVIDER_URL"
  echo "AGENT_PROVIDER_PROFILE=$AGENT_PROVIDER_PROFILE"
  echo "AGENT_EXECUTION_LANE=$AGENT_EXECUTION_LANE"
  echo "PROVIDER_PREFLIGHT_SKIPPED=$SKIP_PROVIDER_PREFLIGHT"
  echo "PROVIDER_PREFLIGHT_JSON=$PROVIDER_PREFLIGHT_JSON"
  echo "PROVIDER_PREFLIGHT_LOG=$PROVIDER_PREFLIGHT_LOG"
  echo "REQUIRE_PLAYABLE_SNAPSHOT=$REQUIRE_PLAYABLE_SNAPSHOT"
  echo "PLAYABLE_SNAPSHOT_TIMEOUT_SECS=$PLAYABLE_SNAPSHOT_TIMEOUT_SECS"
  echo "PLAYABLE_SNAPSHOT_STATE_JSON=$PLAYABLE_SNAPSHOT_STATE_JSON"
  echo "HOSTED_ACCOUNT_STORE_PATH=$HOSTED_ACCOUNT_STORE_PATH"
  echo "STACK_READY=0"
} >"$META_FILE"

if ! wait_for_http_ready "http://${VIEWER_HOST}:${VIEWER_PORT}/" 180 "$LAUNCHER_PID"; then
  if ensure_launcher_alive "$LAUNCHER_PID"; then
    echo "error: viewer HTTP did not become ready in time" >&2
  else
    echo "error: launcher exited before viewer HTTP became ready" >&2
  fi
  tail_logs_on_error
  exit 1
fi

if ! wait_for_tcp_listener_ready "$WEB_BRIDGE_PORT" 60 "$LAUNCHER_PID"; then
  if ensure_launcher_alive "$LAUNCHER_PID"; then
    echo "error: web bridge port ${WEB_BRIDGE_PORT} did not become ready in time" >&2
  else
    echo "error: launcher exited before web bridge port ${WEB_BRIDGE_PORT} became ready" >&2
  fi
  tail_logs_on_error
  exit 1
fi

URL_VIEWER_HOST="$VIEWER_HOST"
if [[ "$URL_VIEWER_HOST" == "0.0.0.0" ]]; then
  URL_VIEWER_HOST="127.0.0.1"
fi
URL_WS_HOST="$WEB_BRIDGE_HOST"
if [[ "$URL_WS_HOST" == "0.0.0.0" ]]; then
  URL_WS_HOST="127.0.0.1"
fi

GAME_URL="http://${URL_VIEWER_HOST}:${VIEWER_PORT}/?ws=ws://${URL_WS_HOST}:${WEB_BRIDGE_PORT}&test_api=1&locale=zh"
SOFTWARE_SAFE_VIEWER_URL_ZH="http://${URL_VIEWER_HOST}:${VIEWER_PORT}/?render_mode=software_safe&ws=ws://${URL_WS_HOST}:${WEB_BRIDGE_PORT}&test_api=1&locale=zh"
SOFTWARE_SAFE_VIEWER_URL_EN="http://${URL_VIEWER_HOST}:${VIEWER_PORT}/?render_mode=software_safe&ws=ws://${URL_WS_HOST}:${WEB_BRIDGE_PORT}&test_api=1&locale=en"

if [[ "$REQUIRE_PLAYABLE_SNAPSHOT" == "1" ]]; then
  if ! playable_probe_state=$(
    wait_for_playable_snapshot_ready \
      "$GAME_URL" \
      "$PLAYABLE_SNAPSHOT_TIMEOUT_SECS" \
      "$LAUNCHER_PID" \
      "$PLAYABLE_SNAPSHOT_STATE_JSON"
  ); then
    if ensure_launcher_alive "$LAUNCHER_PID"; then
      echo "error: viewer stack reached HTTP/bridge ready, but did not expose a non-empty playable snapshot in time" >&2
    else
      echo "error: launcher exited before a playable snapshot became ready" >&2
    fi
    echo "--- playable snapshot readiness state ---" >&2
    cat "$PLAYABLE_SNAPSHOT_STATE_JSON" >&2 || true
    tail_logs_on_error
    exit 1
  fi
fi

{
  echo "RUN_ID=$RUN_ID"
  echo "OUTPUT_DIR=$OUTPUT_DIR"
  echo "WORLD_PID=$LAUNCHER_PID"
  echo "WEB_PID="
  echo "LAUNCHER_PID=$LAUNCHER_PID"
  echo "LIVE_BIND_ADDR=$LIVE_BIND_ADDR"
  echo "WEB_BRIDGE_ADDR=$WEB_BRIDGE_ADDR"
  echo "VIEWER_HOST=$VIEWER_HOST"
  echo "VIEWER_PORT=$VIEWER_PORT"
  echo "CHAIN_ENABLED=$CHAIN_ENABLED"
  echo "CHAIN_NODE_ID=$CHAIN_NODE_ID"
  echo "CHAIN_STATUS_BIND_ADDR=$CHAIN_STATUS_BIND_ADDR"
  echo "LAUNCH_MODE=$LAUNCH_MODE"
  echo "LAUNCH_CMD=$LAUNCH_CMD"
  echo "BUNDLE_DIR=$BUNDLE_DIR"
  echo "AGENT_DECISION_SOURCE=provider_backed"
  echo "AGENT_PROVIDER_TRANSPORT=remote_https"
  echo "AGENT_PROVIDER_URL=$AGENT_PROVIDER_URL"
  echo "AGENT_PROVIDER_PROFILE=$AGENT_PROVIDER_PROFILE"
  echo "AGENT_EXECUTION_LANE=$AGENT_EXECUTION_LANE"
  echo "PROVIDER_PREFLIGHT_SKIPPED=$SKIP_PROVIDER_PREFLIGHT"
  echo "PROVIDER_PREFLIGHT_JSON=$PROVIDER_PREFLIGHT_JSON"
  echo "PROVIDER_PREFLIGHT_LOG=$PROVIDER_PREFLIGHT_LOG"
  echo "REQUIRE_PLAYABLE_SNAPSHOT=$REQUIRE_PLAYABLE_SNAPSHOT"
  echo "PLAYABLE_SNAPSHOT_TIMEOUT_SECS=$PLAYABLE_SNAPSHOT_TIMEOUT_SECS"
  echo "PLAYABLE_SNAPSHOT_STATE_JSON=$PLAYABLE_SNAPSHOT_STATE_JSON"
  echo "PLAYABLE_SNAPSHOT_READY=$REQUIRE_PLAYABLE_SNAPSHOT"
  echo "HOSTED_ACCOUNT_STORE_PATH=$HOSTED_ACCOUNT_STORE_PATH"
  echo "STACK_READY=1"
  echo "GAME_URL=$GAME_URL"
  echo "SOFTWARE_SAFE_VIEWER_URL_ZH=$SOFTWARE_SAFE_VIEWER_URL_ZH"
  echo "SOFTWARE_SAFE_VIEWER_URL_EN=$SOFTWARE_SAFE_VIEWER_URL_EN"
} >"$META_FILE"

if [[ "$JSON_READY" == "1" ]]; then
  python3 - "$RUN_ID" "$OUTPUT_DIR" "$LAUNCHER_PID" "$LIVE_BIND_ADDR" "$WEB_BRIDGE_ADDR" "$VIEWER_HOST" "$VIEWER_PORT" "$CHAIN_ENABLED" "$CHAIN_NODE_ID" "$CHAIN_STATUS_BIND_ADDR" "$LAUNCH_MODE" "$LAUNCH_CMD" "$BUNDLE_DIR" "$GAME_URL" "$SOFTWARE_SAFE_VIEWER_URL_ZH" "$SOFTWARE_SAFE_VIEWER_URL_EN" "$META_FILE" <<'PY'
from __future__ import annotations

import json
import sys

payload = {
    "run_id": sys.argv[1],
    "output_dir": sys.argv[2],
    "launcher_pid": int(sys.argv[3]),
    "live_bind_addr": sys.argv[4],
    "web_bridge_addr": sys.argv[5],
    "viewer_host": sys.argv[6],
    "viewer_port": int(sys.argv[7]),
    "chain_enabled": sys.argv[8] == "1",
    "chain_node_id": sys.argv[9],
    "chain_status_bind_addr": sys.argv[10],
    "launch_mode": sys.argv[11],
    "launch_cmd": sys.argv[12],
    "bundle_dir": sys.argv[13],
    "game_url": sys.argv[14],
    "software_safe_viewer_url_zh": sys.argv[15],
    "software_safe_viewer_url_en": sys.argv[16],
    "meta_file": sys.argv[17],
}
print(json.dumps(payload, ensure_ascii=False))
PY
fi

cat <<INFO
Game test stack is ready.
- Mode: $LAUNCH_MODE
- Launcher: $LAUNCH_CMD
- Bundle dir: ${BUNDLE_DIR:-disabled}
- URL: $GAME_URL
- Software-safe URL (zh): $SOFTWARE_SAFE_VIEWER_URL_ZH
- Software-safe URL (en): $SOFTWARE_SAFE_VIEWER_URL_EN
- Logs: $OUTPUT_DIR
- Chain enabled: $CHAIN_ENABLED
- Chain node id: ${CHAIN_NODE_ID:-disabled}
- Chain status bind: ${CHAIN_STATUS_BIND_ADDR:-disabled}
- LLM decision source: provider_backed remote_https
- Provider URL: $AGENT_PROVIDER_URL
- Provider preflight: $PROVIDER_PREFLIGHT_JSON

Recommended use:
- producer/release playtests: pass --bundle-dir <bundle>
- source mode remains for development/debug only

agent-browser example:
  AGENT_BROWSER_SESSION=game-test-open \
  agent-browser --headed open "$GAME_URL"

Press Ctrl+C to stop launcher process.
INFO

while true; do
  if ! kill -0 "$LAUNCHER_PID" >/dev/null 2>&1; then
    echo "error: oasis7_game_launcher exited unexpectedly" >&2
    tail_logs_on_error
    exit 1
  fi
  sleep 1
done
