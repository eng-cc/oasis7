#!/usr/bin/env bash
# Template: Form Automation Workflow
# Purpose: Fill and submit a web form with validation in an owned session
# Usage: ./form-automation.sh <form-url>
#
# This template demonstrates the snapshot-interact-verify pattern:
# 1. Navigate to form
# 2. Snapshot to get element refs
# 3. Fill fields using refs
# 4. Submit and verify result
#
# Customize: Update the refs (@e1, @e2, etc.) based on the snapshot output.
# Set AB_RESTORE=1 only when persistence is intentional; optional restore checks:
# AB_RESTORE_CHECK_URL, AB_RESTORE_CHECK_TEXT, AB_RESTORE_CHECK_FN.

set -euo pipefail

FORM_URL="${1:?Usage: $0 <form-url>}"

command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
agent-browser --version
agent-browser doctor --offline --quick --json

AB_SESSION="$(agent-browser session id --scope worktree --prefix form)"
export AB_SESSION
ab() { agent-browser --session "$AB_SESSION" "$@"; }

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  ab close >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT INT TERM

RESTORE_ARGS=()
if [[ "${AB_RESTORE:-0}" == "1" ]]; then
  RESTORE_ARGS+=(--restore --restore-save auto)
  [[ -n "${AB_RESTORE_CHECK_URL:-}" ]] && RESTORE_ARGS+=(--restore-check-url "$AB_RESTORE_CHECK_URL")
  [[ -n "${AB_RESTORE_CHECK_TEXT:-}" ]] && RESTORE_ARGS+=(--restore-check-text "$AB_RESTORE_CHECK_TEXT")
  [[ -n "${AB_RESTORE_CHECK_FN:-}" ]] && RESTORE_ARGS+=(--restore-check-fn "$AB_RESTORE_CHECK_FN")
fi

echo "Form automation: $FORM_URL"

# Step 1: Navigate to form
ab "${RESTORE_ARGS[@]}" open "$FORM_URL"
ab wait --load domcontentloaded

# Step 2: Snapshot to discover form elements
echo ""
echo "Form structure:"
ab snapshot -i

# Step 3: Fill form fields (customize these refs based on snapshot output)
#
# Common field types:
#   ab fill @e1 "John Doe"           # Text input
#   ab fill @e2 "user@example.com"   # Email input
#   ab fill @e3 "SecureP@ss123"      # Password input
#   ab select @e4 "Option Value"     # Dropdown
#   ab check @e5                     # Checkbox
#   ab click @e6                     # Radio button
#   ab fill @e7 "Multi-line text"   # Textarea
#   ab upload @e8 /path/to/file.pdf # File upload
#
# Uncomment and modify:
# ab fill @e1 "Test User"
# ab fill @e2 "test@example.com"
# ab click @e3  # Submit button

# Step 4: Wait for submission
# ab wait --url "**/success"  # Or use an app-specific wait --fn/--text signal

# Step 5: Verify result
echo ""
echo "Result:"
ab get url
ab snapshot -i

# Optional: Capture evidence
ab screenshot /tmp/form-result.png
echo "Screenshot saved: /tmp/form-result.png"

echo "Done"
