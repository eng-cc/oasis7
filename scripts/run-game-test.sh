#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/agent-browser-lib.sh"
source "$ROOT_DIR/scripts/bundle-freshness-lib.sh"

VIEWER_HOST="127.0.0.1"
VIEWER_PORT="4173"
LIVE_BIND_ADDR="127.0.0.1:5023"
WEB_BRIDGE_ADDR="127.0.0.1:5011"
ENABLE_LLM="1"
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
SKIP_LLM_PROVIDER_PREFLIGHT="0"

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
  --skip-llm-provider-preflight
                           Skip the active-LLM provider probe before launcher startup
  --with-llm               Enable LLM mode (default: enabled; required for gameplay)
  --no-llm                 Negative-path only; this launcher stack now fails fast without LLM
  -h, --help               Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
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
    --skip-llm-provider-preflight)
      SKIP_LLM_PROVIDER_PREFLIGHT="1"
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
  echo "hint: use env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --no-llm ... only for direct observer/debug diagnostics" >&2
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
  local base_dir
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    if [[ "${CARGO_TARGET_DIR}" == /* ]]; then
      base_dir="${CARGO_TARGET_DIR}"
    else
      base_dir="$ROOT_DIR/${CARGO_TARGET_DIR}"
    fi
  else
    base_dir="$ROOT_DIR/target"
  fi
  printf '%s/debug\n' "$base_dir"
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

tail_logs_on_error() {
  echo "--- oasis7_viewer_live.log (tail) ---" >&2
  tail -n 80 "$WORLD_LOG" >&2 || true
  if [[ -s "$WEB_LOG" ]]; then
    echo "--- web_viewer.log (tail) ---" >&2
    tail -n 80 "$WEB_LOG" >&2 || true
  fi
}

tail_probe_logs_on_error() {
  local probe_json="$1"
  local probe_log="$2"
  if [[ -f "$probe_json" ]]; then
    echo "--- oasis7_llm_provider_probe.json ---" >&2
    cat "$probe_json" >&2 || true
  fi
  if [[ -s "$probe_log" ]]; then
    echo "--- oasis7_llm_provider_probe.log ---" >&2
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
LLM_PROVIDER_PROBE_JSON="$OUTPUT_DIR/oasis7_llm_provider_probe.json"
LLM_PROVIDER_PROBE_LOG="$OUTPUT_DIR/oasis7_llm_provider_probe.log"
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
  --scenario llm_bootstrap
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
WORLD_ARGS+=(--with-llm)

if [[ -n "$BUNDLE_DIR" ]]; then
  LAUNCH_MODE="bundle"
  LAUNCH_CMD="$BUNDLE_DIR/run-game.sh"
  if [[ "$SKIP_LLM_PROVIDER_PREFLIGHT" != "1" ]]; then
    if ! (
      cd "$BUNDLE_DIR"
      "$ROOT_DIR/scripts/check-active-llm-provider.sh"
    ) >"$LLM_PROVIDER_PROBE_JSON" 2>"$LLM_PROVIDER_PROBE_LOG"; then
      echo "error: active LLM provider preflight failed before launcher startup" >&2
      echo "hint: rerun with --skip-llm-provider-preflight only when intentionally validating blocked/failure behavior after stack bootstrap" >&2
      tail_probe_logs_on_error "$LLM_PROVIDER_PROBE_JSON" "$LLM_PROVIDER_PROBE_LOG"
      exit 1
    fi
  fi
  (
    cd "$BUNDLE_DIR"
    "$BUNDLE_DIR/run-game.sh" "${WORLD_ARGS[@]}" >"$WORLD_LOG" 2>&1
  ) &
else
  LAUNCH_MODE="source"
  SOURCE_MODE_TARGET_DIR="$(resolve_source_mode_target_dir)"
  SOURCE_MODE_PROBE_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_llm_provider_probe"
  SOURCE_MODE_LAUNCHER_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_game_launcher"
  SOURCE_MODE_VIEWER_LIVE_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_viewer_live"
  SOURCE_MODE_CHAIN_RUNTIME_BIN="$SOURCE_MODE_TARGET_DIR/oasis7_chain_runtime"
  SOURCE_BUILD_ARGS=(
    build
    -p
    oasis7
    --bin
    oasis7_llm_provider_probe
    --bin
    oasis7_game_launcher
    --bin
    oasis7_viewer_live
  )
  if [[ "$CHAIN_ENABLED" == "1" ]]; then
    SOURCE_BUILD_ARGS+=(--bin oasis7_chain_runtime)
  fi
  env -u RUSTC_WRAPPER cargo "${SOURCE_BUILD_ARGS[@]}"
  [[ -x "$SOURCE_MODE_PROBE_BIN" ]] || { echo "error: built probe binary missing: $SOURCE_MODE_PROBE_BIN" >&2; exit 1; }
  [[ -x "$SOURCE_MODE_LAUNCHER_BIN" ]] || { echo "error: built launcher binary missing: $SOURCE_MODE_LAUNCHER_BIN" >&2; exit 1; }
  [[ -x "$SOURCE_MODE_VIEWER_LIVE_BIN" ]] || { echo "error: built viewer live binary missing: $SOURCE_MODE_VIEWER_LIVE_BIN" >&2; exit 1; }
  if [[ "$CHAIN_ENABLED" == "1" ]]; then
    [[ -x "$SOURCE_MODE_CHAIN_RUNTIME_BIN" ]] || { echo "error: built chain runtime binary missing: $SOURCE_MODE_CHAIN_RUNTIME_BIN" >&2; exit 1; }
  fi
  if [[ "$SKIP_LLM_PROVIDER_PREFLIGHT" != "1" ]]; then
    if ! (
      cd "$ROOT_DIR"
      "$SOURCE_MODE_PROBE_BIN"
    ) >"$LLM_PROVIDER_PROBE_JSON" 2>"$LLM_PROVIDER_PROBE_LOG"; then
      echo "error: active LLM provider preflight failed before launcher startup" >&2
      echo "hint: rerun with --skip-llm-provider-preflight only when intentionally validating blocked/failure behavior after stack bootstrap" >&2
      tail_probe_logs_on_error "$LLM_PROVIDER_PROBE_JSON" "$LLM_PROVIDER_PROBE_LOG"
      exit 1
    fi
  fi
  LAUNCH_CMD="$SOURCE_MODE_LAUNCHER_BIN"
  (
    cd "$ROOT_DIR"
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
  echo "LLM_PROVIDER_PREFLIGHT_SKIPPED=$SKIP_LLM_PROVIDER_PREFLIGHT"
  echo "LLM_PROVIDER_PROBE_JSON=$LLM_PROVIDER_PROBE_JSON"
  echo "LLM_PROVIDER_PROBE_LOG=$LLM_PROVIDER_PROBE_LOG"
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
STANDARD_VIEWER_URL_ZH="http://${URL_VIEWER_HOST}:${VIEWER_PORT}/?render_mode=standard&ws=ws://${URL_WS_HOST}:${WEB_BRIDGE_PORT}&test_api=1&locale=zh"
STANDARD_VIEWER_URL_EN="http://${URL_VIEWER_HOST}:${VIEWER_PORT}/?render_mode=standard&ws=ws://${URL_WS_HOST}:${WEB_BRIDGE_PORT}&test_api=1&locale=en"

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
  echo "LLM_PROVIDER_PREFLIGHT_SKIPPED=$SKIP_LLM_PROVIDER_PREFLIGHT"
  echo "LLM_PROVIDER_PROBE_JSON=$LLM_PROVIDER_PROBE_JSON"
  echo "LLM_PROVIDER_PROBE_LOG=$LLM_PROVIDER_PROBE_LOG"
  echo "STACK_READY=1"
  echo "GAME_URL=$GAME_URL"
  echo "STANDARD_VIEWER_URL_ZH=$STANDARD_VIEWER_URL_ZH"
  echo "STANDARD_VIEWER_URL_EN=$STANDARD_VIEWER_URL_EN"
} >"$META_FILE"

if [[ "$JSON_READY" == "1" ]]; then
  python3 - "$RUN_ID" "$OUTPUT_DIR" "$LAUNCHER_PID" "$LIVE_BIND_ADDR" "$WEB_BRIDGE_ADDR" "$VIEWER_HOST" "$VIEWER_PORT" "$CHAIN_ENABLED" "$CHAIN_NODE_ID" "$CHAIN_STATUS_BIND_ADDR" "$LAUNCH_MODE" "$LAUNCH_CMD" "$BUNDLE_DIR" "$GAME_URL" "$STANDARD_VIEWER_URL_ZH" "$STANDARD_VIEWER_URL_EN" "$META_FILE" <<'PY'
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
    "standard_viewer_url_zh": sys.argv[15],
    "standard_viewer_url_en": sys.argv[16],
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
- Standard Viewer URL (zh): $STANDARD_VIEWER_URL_ZH
- Standard Viewer URL (en): $STANDARD_VIEWER_URL_EN
- Logs: $OUTPUT_DIR
- Chain enabled: $CHAIN_ENABLED
- Chain node id: ${CHAIN_NODE_ID:-disabled}
- Chain status bind: ${CHAIN_STATUS_BIND_ADDR:-disabled}
- LLM provider preflight: $LLM_PROVIDER_PROBE_JSON

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
