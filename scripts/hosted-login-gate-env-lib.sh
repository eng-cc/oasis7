#!/usr/bin/env bash

oasis7_hosted_login_env_default_if_unset() {
  local key="$1"
  local value="$2"
  local current=""

  current=$(printenv "$key" 2>/dev/null || true)
  case "$current" in
    *[![:space:]]*) ;;
    *) printf '%s=%s\n' "$key" "$value" ;;
  esac
}

oasis7_hosted_login_gate_env_defaults() {
  local store_path="$1"

  oasis7_hosted_login_env_default_if_unset \
    "OASIS7_HOSTED_LOGIN_SMTP_FROM_EMAIL" \
    "oasis7-release-gate@example.invalid"
  oasis7_hosted_login_env_default_if_unset \
    "OASIS7_HOSTED_LOGIN_SMTP_PASSWORD" \
    "oasis7-release-gate-placeholder"
  oasis7_hosted_login_env_default_if_unset \
    "OASIS7_HOSTED_ACCOUNT_STORE_BACKEND" \
    "file"
  oasis7_hosted_login_env_default_if_unset \
    "OASIS7_HOSTED_ACCOUNT_STORE_PATH" \
    "$store_path"
}
