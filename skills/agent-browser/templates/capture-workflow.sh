#!/usr/bin/env bash
# Template: Content Capture Workflow
# Purpose: Extract content from a web page with an owned, disposable session
# Usage: ./capture-workflow.sh <url> [output-dir]
#
# Outputs:
#   - page-full.png: Full page screenshot
#   - page-structure.txt: Page element structure with refs
#   - page-text.txt: All text content
#   - page.pdf: PDF version
#
# Optional environment:
#   AB_RESTORE=1 restores cookies/storage for this named session
#   AB_RESTORE_CHECK_URL, AB_RESTORE_CHECK_TEXT, AB_RESTORE_CHECK_FN add a restore check

set -euo pipefail

TARGET_URL="${1:?Usage: $0 <url> [output-dir]}"
OUTPUT_DIR="${2:-.}"

command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
agent-browser --version
agent-browser doctor --offline --quick --json

AB_SESSION="$(agent-browser session id --scope worktree --prefix capture)"
export AB_SESSION
ab() { agent-browser --session "$AB_SESSION" "$@"; }

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  ab close >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT INT TERM

mkdir -p "$OUTPUT_DIR"

RESTORE_ARGS=()
if [[ "${AB_RESTORE:-0}" == "1" ]]; then
  RESTORE_ARGS+=(--restore --restore-save auto)
  [[ -n "${AB_RESTORE_CHECK_URL:-}" ]] && RESTORE_ARGS+=(--restore-check-url "$AB_RESTORE_CHECK_URL")
  [[ -n "${AB_RESTORE_CHECK_TEXT:-}" ]] && RESTORE_ARGS+=(--restore-check-text "$AB_RESTORE_CHECK_TEXT")
  [[ -n "${AB_RESTORE_CHECK_FN:-}" ]] && RESTORE_ARGS+=(--restore-check-fn "$AB_RESTORE_CHECK_FN")
fi

echo "Capturing: $TARGET_URL"
ab "${RESTORE_ARGS[@]}" open "$TARGET_URL"
ab wait --load domcontentloaded

# Get metadata
TITLE="$(ab get title)"
URL="$(ab get url)"
echo "Title: $TITLE"
echo "URL: $URL"

# Capture full page screenshot
ab screenshot --full "$OUTPUT_DIR/page-full.png"
echo "Saved: $OUTPUT_DIR/page-full.png"

# Get page structure with refs
ab snapshot -i > "$OUTPUT_DIR/page-structure.txt"
echo "Saved: $OUTPUT_DIR/page-structure.txt"

# Extract all text content
ab get text body > "$OUTPUT_DIR/page-text.txt"
echo "Saved: $OUTPUT_DIR/page-text.txt"

# Save as PDF
ab pdf "$OUTPUT_DIR/page.pdf"
echo "Saved: $OUTPUT_DIR/page.pdf"

# Optional: use a domain-specific signal before a second capture.
# ab wait --text "Ready"
# ab wait --fn "window.appReady === true"

echo ""
echo "Capture complete:"
ls -la "$OUTPUT_DIR"
