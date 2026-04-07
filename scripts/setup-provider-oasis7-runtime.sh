#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

provider_env_or_default() {
  local suffix="$1"
  local default_value="${2-}"
  local primary_key="OASIS7_PROVIDER_RUNTIME_${suffix}"
  local compatibility_key="OPENCLAW_OASIS7_${suffix}"
  if [[ -n "${!primary_key+x}" ]]; then
    printf '%s\n' "${!primary_key}"
  elif [[ -n "${!compatibility_key+x}" ]]; then
    printf '%s\n' "${!compatibility_key}"
  else
    printf '%s\n' "$default_value"
  fi
}

AGENT_ID="${1:-$(provider_env_or_default AGENT_ID oasis7_provider_agent)}"
WORKSPACE_DIR="$(provider_env_or_default WORKSPACE "$ROOT_DIR/tools/provider/oasis7_provider_workspace")"
MODEL_ID="$(provider_env_or_default MODEL custom-right-codes/gpt-5.4)"

if ! command -v openclaw >/dev/null 2>&1; then
  echo "openclaw CLI not found in PATH" >&2
  exit 1
fi

if [ ! -d "$WORKSPACE_DIR" ]; then
  echo "workspace directory not found: $WORKSPACE_DIR" >&2
  exit 1
fi

if openclaw agents list --json | jq -e --arg id "$AGENT_ID" '.[] | select(.id == $id)' >/dev/null; then
  echo "OpenClaw agent already exists: $AGENT_ID"
  openclaw agents list --json | jq --arg id "$AGENT_ID" '.[] | select(.id == $id)'
  exit 0
fi

openclaw agents add "$AGENT_ID" \
  --workspace "$WORKSPACE_DIR" \
  --model "$MODEL_ID" \
  --non-interactive \
  --json
