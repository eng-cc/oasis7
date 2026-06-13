#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_ARGS=()
MESSAGE='Return only JSON: {"decision":"wait"}'
PRINT_CONFIG="0"
MODEL_OVERRIDDEN="0"
TIMEOUT_MS="${OASIS7_LETAI_CHAT_PROBE_TIMEOUT_MS:-15000}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/check-letai-chat-completions.sh [options]

Validate the LetAI chat-completions path used by the remote provider bridge.
The raw API key and raw model response are not printed.

Options:
  --config <path>     LetAI config file for with-letai-llm-config.sh
  --model <id>        Model override
  --base-url <url>    LetAI chat API base URL override
  --message <text>    Probe prompt
  --timeout-ms <ms>   Probe timeout in milliseconds (default: $OASIS7_LETAI_CHAT_PROBE_TIMEOUT_MS or 15000)
  --print-config      Print sanitized resolved config and exit
  -h, --help          Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config|--model|--base-url)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "error: $1 requires a value" >&2
        exit 2
      fi
      CONFIG_ARGS+=("$1" "$2")
      if [[ "$1" == "--model" ]]; then
        MODEL_OVERRIDDEN="1"
      fi
      shift 2
      ;;
    --message)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "error: --message requires a value" >&2
        exit 2
      fi
      MESSAGE="$2"
      shift 2
      ;;
    --timeout-ms)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "error: --timeout-ms requires a value" >&2
        exit 2
      fi
      TIMEOUT_MS="$2"
      shift 2
      ;;
    --print-config)
      PRINT_CONFIG="1"
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

if ! [[ "$TIMEOUT_MS" =~ ^[0-9]+$ ]] || [[ "$TIMEOUT_MS" -lt 1000 ]]; then
  echo "error: --timeout-ms must be an integer >= 1000" >&2
  exit 2
fi

if [[ "$MODEL_OVERRIDDEN" == "0" ]]; then
  DEFAULT_MODEL_ARGS=(--model "${OASIS7_LETAI_CHAT_MODEL:-gpt-5.4}")
  if [[ "${#CONFIG_ARGS[@]}" -gt 0 ]]; then
    CONFIG_ARGS=("${DEFAULT_MODEL_ARGS[@]}" "${CONFIG_ARGS[@]}")
  else
    CONFIG_ARGS=("${DEFAULT_MODEL_ARGS[@]}")
  fi
fi

if [[ "$PRINT_CONFIG" == "1" ]]; then
  if [[ "${#CONFIG_ARGS[@]}" -gt 0 ]]; then
    exec "$ROOT_DIR/scripts/with-letai-llm-config.sh" "${CONFIG_ARGS[@]}" --print-config
  fi
  exec "$ROOT_DIR/scripts/with-letai-llm-config.sh" --print-config
fi

tmp_dir="${TMPDIR:-/tmp}/oasis7-letai-chat-check-$$"
mkdir -p "$tmp_dir"
trap 'rm -rf "$tmp_dir"' EXIT

set +e
if [[ "${#CONFIG_ARGS[@]}" -gt 0 ]]; then
  "$ROOT_DIR/scripts/with-letai-llm-config.sh" "${CONFIG_ARGS[@]}" -- bash -s -- "$MESSAGE" "$tmp_dir" "$TIMEOUT_MS" <<'BASH'
set -euo pipefail
message="$1"
tmp_dir="$2"
timeout_ms="$3"
export OASIS7_REMOTE_LLM_BASE_URL="$OASIS7_LLM_BASE_URL"
export OASIS7_REMOTE_LLM_API_KEY="$OASIS7_LLM_API_KEY"
export OASIS7_REMOTE_LLM_MODEL="$OASIS7_LLM_MODEL"
export OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS="${OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS:-32}"
export OASIS7_REMOTE_LLM_TEMPERATURE="${OASIS7_REMOTE_LLM_TEMPERATURE:-0}"
export OASIS7_REMOTE_LLM_STREAM="${OASIS7_REMOTE_LLM_STREAM:-true}"
export OASIS7_REMOTE_LLM_AUTO_TOPUP_USD="${OASIS7_REMOTE_LLM_AUTO_TOPUP_USD:-${OASIS7_LETAI_AUTO_TOPUP_USD:-}}"
python3 scripts/provider-remote-https/letai_provider_cli.py \
  agent \
  --agent local-smoke \
  --message "$message" \
  --timeout "$timeout_ms" \
  >"$tmp_dir/response.json" \
  2>"$tmp_dir/error.txt"
BASH
else
  "$ROOT_DIR/scripts/with-letai-llm-config.sh" -- bash -s -- "$MESSAGE" "$tmp_dir" "$TIMEOUT_MS" <<'BASH'
set -euo pipefail
message="$1"
tmp_dir="$2"
timeout_ms="$3"
export OASIS7_REMOTE_LLM_BASE_URL="$OASIS7_LLM_BASE_URL"
export OASIS7_REMOTE_LLM_API_KEY="$OASIS7_LLM_API_KEY"
export OASIS7_REMOTE_LLM_MODEL="$OASIS7_LLM_MODEL"
export OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS="${OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS:-32}"
export OASIS7_REMOTE_LLM_TEMPERATURE="${OASIS7_REMOTE_LLM_TEMPERATURE:-0}"
export OASIS7_REMOTE_LLM_STREAM="${OASIS7_REMOTE_LLM_STREAM:-true}"
export OASIS7_REMOTE_LLM_AUTO_TOPUP_USD="${OASIS7_REMOTE_LLM_AUTO_TOPUP_USD:-${OASIS7_LETAI_AUTO_TOPUP_USD:-}}"
python3 scripts/provider-remote-https/letai_provider_cli.py \
  agent \
  --agent local-smoke \
  --message "$message" \
  --timeout "$timeout_ms" \
  >"$tmp_dir/response.json" \
  2>"$tmp_dir/error.txt"
BASH
fi
status=$?
set -e

if [[ "$status" -ne 0 ]]; then
  python3 - "$tmp_dir/error.txt" "$status" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

message = Path(sys.argv[1]).read_text(errors="replace").strip()
print(json.dumps({
    "ok": False,
    "exit_status": int(sys.argv[2]),
    "error": message[:500],
}, ensure_ascii=False, sort_keys=True))
PY
  exit "$status"
fi

python3 - "$tmp_dir/response.json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
text = str((payload.get("payloads") or [{}])[0].get("text") or "")
meta = payload.get("meta") or {}
agent_meta = meta.get("agentMeta") or {}
usage = agent_meta.get("usage") or {}
print(json.dumps({
    "ok": True,
    "provider": agent_meta.get("provider"),
    "model": agent_meta.get("model"),
    "text_len": len(text),
    "prompt_tokens_present": agent_meta.get("promptTokens") is not None,
    "usage_total_present": usage.get("total") is not None,
}, ensure_ascii=False, sort_keys=True))
PY
