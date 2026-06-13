#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DEFAULT_LETAI_CONFIG_PATH="/Users/scc/Documents/keys/letai.txt"
DEFAULT_TOKEN_CONFIG_PATH="${OASIS7_LETAI_TOKEN_CONFIG_PATH:-/Users/scc/Documents/keys/letai-token-local.txt}"
if [[ -n "${OASIS7_LETAI_CONFIG_PATH:-}" ]]; then
  CONFIG_PATH="$OASIS7_LETAI_CONFIG_PATH"
elif [[ -f "$DEFAULT_LETAI_CONFIG_PATH" ]]; then
  CONFIG_PATH="$DEFAULT_LETAI_CONFIG_PATH"
elif [[ -f "$DEFAULT_TOKEN_CONFIG_PATH" ]]; then
  CONFIG_PATH="$DEFAULT_TOKEN_CONFIG_PATH"
else
  CONFIG_PATH="$DEFAULT_LETAI_CONFIG_PATH"
fi
BIND_ADDR="127.0.0.1:5841"
PROXY_URL="http://127.0.0.1:7897"
SOCKS_PROXY_URL="socks5://127.0.0.1:7897"
USE_DEFAULT_PROXY="1"
SKIP_CHAT_PROBE="0"
SKIP_BRIDGE_SMOKE="0"
ENSURE_TOKEN_CONFIG="1"
CHAT_ECHO="1"
AUTO_PLAY="${OASIS7_LOCAL_LETAI_AUTO_PLAY:-1}"
DEPLOYMENT_MODE="${OASIS7_LOCAL_LETAI_DEPLOYMENT_MODE:-trusted_local_only}"
BRIDGE_SMOKE_ATTEMPTS="2"
BRIDGE_AUTO_TOPUP_USD="${OASIS7_LETAI_AUTO_TOPUP_USD:-0.1}"
CHAT_PROBE_TIMEOUT_MS="${OASIS7_LETAI_CHAT_PROBE_TIMEOUT_MS:-60000}"
AGENT_PROVIDER_CONNECT_TIMEOUT_MS="${OASIS7_AGENT_PROVIDER_CONNECT_TIMEOUT_MS:-60000}"
MODEL=""
OUTPUT_DIR=""
BRIDGE_PID=""
LAUNCHER_ARGS=()
BRIDGE_ARGS=()

usage() {
  cat <<'USAGE'
Usage: ./scripts/run-local-letai-game-test.sh [options] [-- launcher args...]

Start the canonical local real LetAI gameplay test stack:
1. load the operator-owned LetAI config
2. optionally validate chat-completions
3. start the local provider bridge on 127.0.0.1:5841
4. start run-launcher-stack.sh pointed at that bridge

Use this wrapper instead of manually stitching together the provider bridge and
run-launcher-stack.sh when validating local provider-backed gameplay or
agent_chat behavior.

Options:
  --config <path>             LetAI config file (default: $OASIS7_LETAI_CONFIG_PATH,
                              /Users/scc/Documents/keys/letai.txt, then token-local fallback)
  --model <id>                LetAI model override (default: gpt-5.4 via helper)
  --bind <host:port>          Local provider bind address (default: 127.0.0.1:5841)
  --proxy <url>               HTTP/HTTPS proxy to export if unset (default: http://127.0.0.1:7897)
  --socks-proxy <url>         all_proxy value to export if unset (default: socks5://127.0.0.1:7897)
  --no-default-proxy          Do not set proxy environment defaults
  --auto-topup-usd <amount>   Auto top up on insufficient quota (default: 0.1)
  --no-ensure-token-config    Use --config directly; do not generate token config from platform key
  --chat-echo                 Enable local QA chat echo with provider-backed gameplay (default)
  --no-chat-echo              Keep provider-backed agent chat disabled
  --auto-play                 Start gameplay/world progression on viewer connection (default)
  --no-auto-play              Require manual Play before gameplay/world progression
  --deployment-mode <mode>    Launcher deployment mode (default: trusted_local_only)
  --hosted-public-join        Use hosted_public_join instead of the local trusted playtest chain
  --skip-chat-probe           Skip the upfront LetAI chat-completions probe
  --skip-bridge-smoke         Skip provider bridge contract smoke before launcher startup
  --bridge-smoke-attempts <n> Retry provider bridge smoke up to <n> times (default: 2)
  --chat-probe-timeout-ms <ms>
                              LetAI chat probe timeout (default: 60000)
  --agent-provider-connect-timeout-ms <ms>
                              Runtime provider decision timeout (default: 60000)
  --output-dir <path>         Launcher output dir; bridge log goes there too
  -h, --help                  Show help

Examples:
  ./scripts/run-local-letai-game-test.sh
  ./scripts/run-local-letai-game-test.sh -- --viewer-port 4174 --json-ready
USAGE
}

cleanup() {
  if [[ -n "$BRIDGE_PID" ]] && kill -0 "$BRIDGE_PID" >/dev/null 2>&1; then
    kill "$BRIDGE_PID" >/dev/null 2>&1 || true
    wait "$BRIDGE_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config)
      CONFIG_PATH="${2:-}"
      shift 2
      ;;
    --model)
      MODEL="${2:-}"
      shift 2
      ;;
    --bind)
      BIND_ADDR="${2:-}"
      shift 2
      ;;
    --proxy)
      PROXY_URL="${2:-}"
      shift 2
      ;;
    --socks-proxy)
      SOCKS_PROXY_URL="${2:-}"
      shift 2
      ;;
    --no-default-proxy)
      USE_DEFAULT_PROXY="0"
      shift
      ;;
    --auto-topup-usd)
      BRIDGE_AUTO_TOPUP_USD="${2:-}"
      shift 2
      ;;
    --no-ensure-token-config)
      ENSURE_TOKEN_CONFIG="0"
      shift
      ;;
    --chat-echo)
      CHAT_ECHO="1"
      shift
      ;;
    --no-chat-echo)
      CHAT_ECHO="0"
      shift
      ;;
    --auto-play)
      AUTO_PLAY="1"
      shift
      ;;
    --no-auto-play)
      AUTO_PLAY="0"
      shift
      ;;
    --deployment-mode)
      DEPLOYMENT_MODE="${2:-}"
      shift 2
      ;;
    --hosted-public-join)
      DEPLOYMENT_MODE="hosted_public_join"
      shift
      ;;
    --skip-chat-probe)
      SKIP_CHAT_PROBE="1"
      shift
      ;;
    --skip-bridge-smoke)
      SKIP_BRIDGE_SMOKE="1"
      shift
      ;;
    --bridge-smoke-attempts)
      BRIDGE_SMOKE_ATTEMPTS="${2:-}"
      shift 2
      ;;
    --chat-probe-timeout-ms)
      CHAT_PROBE_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --agent-provider-connect-timeout-ms)
      AGENT_PROVIDER_CONNECT_TIMEOUT_MS="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      LAUNCHER_ARGS+=("$@")
      break
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$CONFIG_PATH" || -z "$BIND_ADDR" || -z "$BRIDGE_AUTO_TOPUP_USD" || -z "$BRIDGE_SMOKE_ATTEMPTS" || -z "$DEPLOYMENT_MODE" || -z "$CHAT_PROBE_TIMEOUT_MS" || -z "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" ]]; then
  echo "error: empty --config, --bind, --deployment-mode, --auto-topup-usd, --bridge-smoke-attempts, --chat-probe-timeout-ms, or --agent-provider-connect-timeout-ms is not allowed" >&2
  exit 2
fi
if ! [[ "$BRIDGE_SMOKE_ATTEMPTS" =~ ^[0-9]+$ ]] || [[ "$BRIDGE_SMOKE_ATTEMPTS" -lt 1 ]]; then
  echo "error: --bridge-smoke-attempts must be a positive integer" >&2
  exit 2
fi
if ! [[ "$CHAT_PROBE_TIMEOUT_MS" =~ ^[0-9]+$ ]] || [[ "$CHAT_PROBE_TIMEOUT_MS" -lt 1000 ]]; then
  echo "error: --chat-probe-timeout-ms must be an integer >= 1000" >&2
  exit 2
fi
if ! [[ "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" =~ ^[0-9]+$ ]] || [[ "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" -lt 1000 ]]; then
  echo "error: --agent-provider-connect-timeout-ms must be an integer >= 1000" >&2
  exit 2
fi

if [[ ! -f "$CONFIG_PATH" ]]; then
  echo "error: LetAI config not found: $CONFIG_PATH" >&2
  exit 2
fi

if [[ "$USE_DEFAULT_PROXY" == "1" ]]; then
  export https_proxy="${https_proxy:-$PROXY_URL}"
  export http_proxy="${http_proxy:-$PROXY_URL}"
  export all_proxy="${all_proxy:-$SOCKS_PROXY_URL}"
fi
if [[ "$CHAT_ECHO" == "1" ]]; then
  export OASIS7_RUNTIME_AGENT_CHAT_ECHO=1
else
  unset OASIS7_RUNTIME_AGENT_CHAT_ECHO
fi
export OASIS7_LETAI_AUTO_TOPUP_USD="$BRIDGE_AUTO_TOPUP_USD"

RUN_STAMP="$(date +%Y%m%d-%H%M%S)"
if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="$ROOT_DIR/output/local-letai-game-test/$RUN_STAMP"
fi
mkdir -p "$OUTPUT_DIR"

if [[ -n "$MODEL" ]]; then
  BRIDGE_ARGS+=(--model "$MODEL")
fi

EFFECTIVE_CONFIG_PATH="$CONFIG_PATH"
if [[ "$ENSURE_TOKEN_CONFIG" == "1" ]]; then
  EFFECTIVE_CONFIG_PATH="$OUTPUT_DIR/letai-local-token.env"
  ENSURE_ARGS=()
  if [[ -n "$MODEL" ]]; then
    ENSURE_ARGS+=(--model "$MODEL")
  fi
  "$ROOT_DIR/scripts/ensure-letai-local-token-config.sh" \
    --config "$CONFIG_PATH" \
    --out "$EFFECTIVE_CONFIG_PATH" \
    ${ENSURE_ARGS[@]+"${ENSURE_ARGS[@]}"}
fi

echo "local LetAI game test"
echo "config=$CONFIG_PATH"
echo "effective_config=$EFFECTIVE_CONFIG_PATH"
echo "bridge=http://$BIND_ADDR"
echo "chat_echo=$([[ "$CHAT_ECHO" == "1" ]] && echo enabled || echo disabled)"
echo "auto_play=$([[ "$AUTO_PLAY" == "1" ]] && echo enabled || echo disabled)"
echo "deployment_mode=$DEPLOYMENT_MODE"
echo "chat_probe_timeout_ms=$CHAT_PROBE_TIMEOUT_MS"
echo "agent_provider_connect_timeout_ms=$AGENT_PROVIDER_CONNECT_TIMEOUT_MS"
echo "output_dir=$OUTPUT_DIR"
export OASIS7_AGENT_PROVIDER_CONNECT_TIMEOUT_MS="$AGENT_PROVIDER_CONNECT_TIMEOUT_MS"
export OASIS7_AGENT_PROVIDER_DECISION_TIMEOUT_MS="$AGENT_PROVIDER_CONNECT_TIMEOUT_MS"

if [[ "$SKIP_CHAT_PROBE" != "1" ]]; then
  "$ROOT_DIR/scripts/check-letai-chat-completions.sh" \
    --config "$EFFECTIVE_CONFIG_PATH" \
    --timeout-ms "$CHAT_PROBE_TIMEOUT_MS" \
    ${BRIDGE_ARGS[@]+"${BRIDGE_ARGS[@]}"}
fi

"$ROOT_DIR/scripts/run-local-letai-provider-bridge.sh" \
  --config "$EFFECTIVE_CONFIG_PATH" \
  --bind "$BIND_ADDR" \
  --auto-topup-usd "$BRIDGE_AUTO_TOPUP_USD" \
  ${BRIDGE_ARGS[@]+"${BRIDGE_ARGS[@]}"} \
  >"$OUTPUT_DIR/local-letai-provider-bridge.log" 2>&1 &
BRIDGE_PID="$!"

for _ in $(seq 1 90); do
  if ! kill -0 "$BRIDGE_PID" >/dev/null 2>&1; then
    echo "error: local LetAI bridge exited early; log: $OUTPUT_DIR/local-letai-provider-bridge.log" >&2
    tail -n 80 "$OUTPUT_DIR/local-letai-provider-bridge.log" >&2 || true
    exit 1
  fi
  if curl -fsS "http://$BIND_ADDR/v1/provider/info" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -fsS "http://$BIND_ADDR/v1/provider/info" >/dev/null 2>&1; then
  echo "error: local LetAI bridge did not become ready; log: $OUTPUT_DIR/local-letai-provider-bridge.log" >&2
  tail -n 80 "$OUTPUT_DIR/local-letai-provider-bridge.log" >&2 || true
  exit 1
fi

if [[ "$SKIP_BRIDGE_SMOKE" != "1" ]]; then
  smoke_status=1
  for attempt in $(seq 1 "$BRIDGE_SMOKE_ATTEMPTS"); do
    if "$ROOT_DIR/scripts/provider-remote-https/provider-bridge-contract-smoke.sh" \
      --base-url "http://$BIND_ADDR" \
      --timeout-ms "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" \
      --decision-count 1 \
      --min-successes 1; then
      smoke_status=0
      break
    fi
    echo "provider bridge smoke attempt $attempt/$BRIDGE_SMOKE_ATTEMPTS failed" >&2
    sleep 2
  done
  if [[ "$smoke_status" -ne 0 ]]; then
    "$ROOT_DIR/scripts/provider-remote-https/provider-bridge-contract-smoke.sh" \
      --base-url "http://$BIND_ADDR" \
      --timeout-ms "$AGENT_PROVIDER_CONNECT_TIMEOUT_MS" \
      --decision-count 1 \
      --min-successes 0 >&2 || true
    exit 1
  fi
fi

set +e
LAUNCHER_MODE_ARGS=(--deployment-mode "$DEPLOYMENT_MODE")
if [[ "$DEPLOYMENT_MODE" == "trusted_local_only" ]]; then
  LAUNCHER_MODE_ARGS+=(--allow-trusted-local-playtest)
fi
if [[ "$AUTO_PLAY" == "1" ]]; then
  LAUNCHER_MODE_ARGS+=(--auto-play)
fi
"$ROOT_DIR/scripts/run-launcher-stack.sh" \
  "${LAUNCHER_MODE_ARGS[@]}" \
  --agent-decision-source provider_backed \
  --agent-provider-url "http://$BIND_ADDR" \
  --output-dir "$OUTPUT_DIR/launcher" \
  ${LAUNCHER_ARGS[@]+"${LAUNCHER_ARGS[@]}"}
launcher_status=$?
set -e

exit "$launcher_status"
