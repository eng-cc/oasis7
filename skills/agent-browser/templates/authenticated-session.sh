#!/usr/bin/env bash
# Template: Authenticated Session Workflow
# Purpose: Login once, auto-restore a named session, and verify the restored URL
# Usage: ./authenticated-session.sh <login-url> <authenticated-url>
#
# Auth Vault is preferred when a reusable credential profile is available:
#   agent-browser auth save myapp --url <login-url> --username <user> --password-stdin
#   agent-browser auth login myapp
# This template is for a flow whose form refs still need local customization.
#
# Environment variables:
#   APP_USERNAME - Login username/email
#   APP_PASSWORD - Login password
#   AB_SESSION_PREFIX - Optional purpose prefix (default: authenticated)
#   RESTORE_CHECK_URL - Optional glob overriding the authenticated URL check

set -euo pipefail

LOGIN_URL="${1:?Usage: $0 <login-url> <authenticated-url>}"
AUTHENTICATED_URL="${2:?Usage: $0 <login-url> <authenticated-url>}"
SESSION_PREFIX="${AB_SESSION_PREFIX:-authenticated}"
RESTORE_CHECK_URL="${RESTORE_CHECK_URL:-$AUTHENTICATED_URL}"

command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
agent-browser --version
agent-browser doctor --offline --quick --json

AB_SESSION="$(agent-browser session id --scope worktree --prefix "$SESSION_PREFIX")"
export AB_SESSION
ab() { agent-browser --session "$AB_SESSION" "$@"; }

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  ab close >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT INT TERM

echo "Trying to restore owned session $AB_SESSION..."
if ab --restore --restore-save auto \
  --restore-check-url "$RESTORE_CHECK_URL" \
  open "$AUTHENTICATED_URL"; then
  ab wait --load domcontentloaded
  CURRENT_URL="$(ab get url)"
  if [[ "$CURRENT_URL" != *"login"* ]] && [[ "$CURRENT_URL" != *"signin"* ]]; then
    echo "Session restored successfully"
    ab session info --json
    ab snapshot -i
    exit 0
  fi
fi

echo "Opening login page for a fresh authentication flow..."
ab --restore --restore-save auto open "$LOGIN_URL"
ab wait --load domcontentloaded

echo ""
echo "Login form structure:"
echo "---"
ab snapshot -i
echo "---"
echo ""
echo "Next steps:"
echo "  1. Note the refs: username=@e?, password=@e?, submit=@e?"
echo "  2. Update the LOGIN FLOW section below with your refs"
echo "  3. Set: export APP_USERNAME='...' APP_PASSWORD='...'"
echo "  4. Re-run after enabling the customized LOGIN FLOW"
echo ""
exit 0

# ================================================================
# LOGIN FLOW: Uncomment and customize after discovery
# ================================================================
# : "${APP_USERNAME:?Set APP_USERNAME environment variable}"
# : "${APP_PASSWORD:?Set APP_PASSWORD environment variable}"
#
# ab --restore --restore-save auto open "$LOGIN_URL"
# ab wait --load domcontentloaded
# ab snapshot -i
#
# # Fill credentials (update refs to match your form)
# ab fill @e1 "$APP_USERNAME"
# ab fill @e2 "$APP_PASSWORD"
# ab click @e3
# ab wait --url "$RESTORE_CHECK_URL"
#
# FINAL_URL="$(ab get url)"
# if [[ "$FINAL_URL" == *"login"* ]] || [[ "$FINAL_URL" == *"signin"* ]]; then
#   echo "Login failed - still on login page" >&2
#   ab screenshot /tmp/login-failed.png
#   exit 1
# fi
#
# # Re-open through restore so the check is explicit and auto-save remains owned.
# ab --restore --restore-save auto \
#   --restore-check-url "$RESTORE_CHECK_URL" \
#   open "$AUTHENTICATED_URL"
# ab wait --load domcontentloaded
# echo "Login successful"
# ab session info --json
# ab snapshot -i
