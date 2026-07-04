#!/usr/bin/env bash
set -euo pipefail

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "missing required environment variable: ${name}" >&2
    exit 64
  fi
}

require_readable_file() {
  local path="$1"
  local label="$2"
  if [[ ! -r "$path" ]]; then
    echo "${label} is not readable: ${path}" >&2
    exit 66
  fi
  if [[ ! -s "$path" ]]; then
    echo "${label} is empty: ${path}" >&2
    exit 66
  fi
}

require_unsigned_int() {
  local value="$1"
  local label="$2"
  if [[ ! "$value" =~ ^[0-9]+$ ]] || [[ "$value" == "0" ]]; then
    echo "${label} must be a positive integer: ${value}" >&2
    exit 65
  fi
}

require_env OASIS7_PUBLIC_TESTNET_FAUCET_ROOT
require_env OASIS7_PUBLIC_TESTNET_FAUCET_UPSTREAM
require_env OASIS7_PUBLIC_TESTNET_FAUCET_PUBLIC_KEY_FILE
require_env OASIS7_PUBLIC_TESTNET_FAUCET_PRIVATE_KEY_FILE
require_env OASIS7_PUBLIC_TESTNET_FAUCET_AMOUNT

ROOT_DIR="${OASIS7_PUBLIC_TESTNET_FAUCET_ROOT}"
FAUCET_BIN="${OASIS7_PUBLIC_TESTNET_FAUCET_BIN:-${ROOT_DIR}/oasis7_testnet_faucet}"
LISTEN="${OASIS7_PUBLIC_TESTNET_FAUCET_LISTEN:-0.0.0.0:6681}"
COOLDOWN_SECS="${OASIS7_PUBLIC_TESTNET_FAUCET_COOLDOWN_SECS:-3600}"
REQUEST_TIMEOUT_SECS="${OASIS7_PUBLIC_TESTNET_FAUCET_REQUEST_TIMEOUT_SECS:-10}"

if [[ ! -x "$FAUCET_BIN" ]]; then
  echo "faucet binary is not executable: ${FAUCET_BIN}" >&2
  exit 66
fi

require_readable_file "$OASIS7_PUBLIC_TESTNET_FAUCET_PUBLIC_KEY_FILE" \
  "faucet public key file"
require_readable_file "$OASIS7_PUBLIC_TESTNET_FAUCET_PRIVATE_KEY_FILE" \
  "faucet private key file"
require_unsigned_int "$OASIS7_PUBLIC_TESTNET_FAUCET_AMOUNT" \
  "OASIS7_PUBLIC_TESTNET_FAUCET_AMOUNT"
require_unsigned_int "$COOLDOWN_SECS" \
  "OASIS7_PUBLIC_TESTNET_FAUCET_COOLDOWN_SECS"
require_unsigned_int "$REQUEST_TIMEOUT_SECS" \
  "OASIS7_PUBLIC_TESTNET_FAUCET_REQUEST_TIMEOUT_SECS"

exec "$FAUCET_BIN" serve \
  --listen "$LISTEN" \
  --upstream "$OASIS7_PUBLIC_TESTNET_FAUCET_UPSTREAM" \
  --faucet-public-key-file "$OASIS7_PUBLIC_TESTNET_FAUCET_PUBLIC_KEY_FILE" \
  --faucet-private-key-file "$OASIS7_PUBLIC_TESTNET_FAUCET_PRIVATE_KEY_FILE" \
  --amount "$OASIS7_PUBLIC_TESTNET_FAUCET_AMOUNT" \
  --cooldown-secs "$COOLDOWN_SECS" \
  --request-timeout-secs "$REQUEST_TIMEOUT_SECS"
