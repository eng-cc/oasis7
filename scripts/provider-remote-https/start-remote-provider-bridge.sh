#!/usr/bin/env bash
set -euo pipefail

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "missing required environment variable: $name" >&2
    exit 1
  fi
}

require_readable_file() {
  local path="$1"
  local label="$2"
  if [[ ! -r "$path" ]]; then
    echo "$label is not readable: $path" >&2
    exit 1
  fi
}

require_env OASIS7_PROVIDER_BRIDGE_ROOT

ROOT_DIR="$OASIS7_PROVIDER_BRIDGE_ROOT"
BRIDGE_BIN="${OASIS7_PROVIDER_BRIDGE_BIN:-$ROOT_DIR/target/release/oasis7_provider_local_bridge}"
CLI_WRAPPER="${OASIS7_PROVIDER_CLI_WRAPPER:-$ROOT_DIR/scripts/provider-remote-https/letai_provider_cli.py}"
BRIDGE_BIND="${OASIS7_PROVIDER_BRIDGE_BIND:-127.0.0.1:5841}"
BRIDGE_AGENT_ID="${OASIS7_PROVIDER_AGENT_ID:-letai-remote}"
BRIDGE_THINKING="${OASIS7_PROVIDER_THINKING:-off}"
LLM_BASE_URL="${OASIS7_REMOTE_LLM_BASE_URL:-https://api.letai.run/v1}"
LLM_HEALTH_URL="${OASIS7_REMOTE_LLM_HEALTH_URL:-${LLM_BASE_URL%/}/models}"
AUTH_ROUTE_MAP_PATH="${OASIS7_PROVIDER_AUTH_ROUTE_MAP_PATH:-}"
AUTH_ROUTE_FROM_BEARER="${OASIS7_PROVIDER_AUTH_ROUTE_FROM_BEARER:-}"

if [[ -n "${AUTH_ROUTE_MAP_PATH}" ]]; then
  require_env OASIS7_REMOTE_LLM_ROUTES_PATH
  require_readable_file "$AUTH_ROUTE_MAP_PATH" "OASIS7_PROVIDER_AUTH_ROUTE_MAP_PATH"
  require_readable_file "$OASIS7_REMOTE_LLM_ROUTES_PATH" "OASIS7_REMOTE_LLM_ROUTES_PATH"
elif [[ "${AUTH_ROUTE_FROM_BEARER}" == "1" || "${AUTH_ROUTE_FROM_BEARER}" == "true" || "${AUTH_ROUTE_FROM_BEARER}" == "yes" || "${AUTH_ROUTE_FROM_BEARER}" == "on" ]]; then
  require_env OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH
  require_readable_file "$OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH" "OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH"
else
  require_env OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN
  require_env OASIS7_REMOTE_LLM_API_KEY
  require_env OASIS7_REMOTE_LLM_MODEL
fi

if [[ ! -x "$BRIDGE_BIN" ]]; then
  echo "bridge binary is not executable: $BRIDGE_BIN" >&2
  exit 1
fi

if [[ ! -x "$CLI_WRAPPER" ]]; then
  echo "provider CLI wrapper is not executable: $CLI_WRAPPER" >&2
  exit 1
fi

cmd=(
  "$BRIDGE_BIN"
  --bind "$BRIDGE_BIND"
  --provider-cli-bin "$CLI_WRAPPER"
  --provider-agent "$BRIDGE_AGENT_ID"
  --provider-thinking "$BRIDGE_THINKING"
  --gateway-health-url "$LLM_HEALTH_URL"
)

if [[ -n "${AUTH_ROUTE_MAP_PATH}" ]]; then
  cmd+=(--auth-route-map "$AUTH_ROUTE_MAP_PATH")
elif [[ "${AUTH_ROUTE_FROM_BEARER}" == "1" || "${AUTH_ROUTE_FROM_BEARER}" == "true" || "${AUTH_ROUTE_FROM_BEARER}" == "yes" || "${AUTH_ROUTE_FROM_BEARER}" == "on" ]]; then
  cmd+=(--auth-route-from-bearer)
else
  cmd+=(--auth-token "$OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN")
fi

exec "${cmd[@]}"
