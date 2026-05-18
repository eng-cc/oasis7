#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

SOURCE_MANUAL="${REPO_ROOT}/doc/world-simulator/viewer/viewer-manual.manual.md"
MIRROR_MANUALS=(
  "${REPO_ROOT}/site/doc/cn/viewer-manual.html"
  "${REPO_ROOT}/site/doc/en/viewer-manual.html"
)

SOURCE_REQUIRED_PATTERNS=(
  'command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }'
  'agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"'
  'agent-browser snapshot -i'
)

MIRROR_REQUIRED_PATTERNS=(
  'command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }'
  'agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&amp;render_mode=viewer&amp;test_api=1"'
  'agent-browser snapshot -i'
)

FORBIDDEN_PATTERNS=(
  'export REPO_ROOT="$(pwd)"'
)

MIRROR_REFERENCE_FORBIDDEN_PATTERNS=(
  'doc/world-simulator/viewer-location-fine-grained-rendering.md'
  'doc/world-simulator/viewer-auto-focus-capture.md'
  'doc/world-simulator/viewer-web-closure-testing-policy.md'
  'doc/world-simulator/viewer-selection-details.md'
  'doc/world-simulator/viewer-right-panel-module-visibility.md'
  'doc/world-simulator/viewer-overview-map-zoom.md'
  'doc/world-simulator/viewer-agent-quick-locate.md'
  'doc/world-simulator/viewer-copyable-text.md'
  'doc/world-simulator/viewer-generic-focus-targets.md'
  'doc/world-simulator/viewer-web-test-api-step-control-2026-02-24.md'
)

contains_fixed_pattern() {
  local pattern="$1"
  local file_path="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -Fq -- "$pattern" "$file_path"
    return $?
  fi
  grep -Fq -- "$pattern" "$file_path"
}

check_required_patterns() {
  local file_path="$1"
  shift
  local pattern
  for pattern in "$@"; do
    if ! contains_fixed_pattern "$pattern" "$file_path"; then
      echo "error: missing required pattern in ${file_path}: ${pattern}" >&2
      return 1
    fi
  done
}

check_forbidden_patterns() {
  local file_path="$1"
  shift
  local pattern
  for pattern in "$@"; do
    if contains_fixed_pattern "$pattern" "$file_path"; then
      echo "error: found deprecated pattern in ${file_path}: ${pattern}" >&2
      return 1
    fi
  done
}

check_required_patterns "${SOURCE_MANUAL}" "${SOURCE_REQUIRED_PATTERNS[@]}"
check_forbidden_patterns "${SOURCE_MANUAL}" "${FORBIDDEN_PATTERNS[@]}"

for mirror in "${MIRROR_MANUALS[@]}"; do
  check_required_patterns "${mirror}" "${MIRROR_REQUIRED_PATTERNS[@]}"
  check_forbidden_patterns "${mirror}" "${FORBIDDEN_PATTERNS[@]}"
  check_forbidden_patterns "${mirror}" "${MIRROR_REFERENCE_FORBIDDEN_PATTERNS[@]}"
done

echo "ok: viewer manual static mirrors are synced with the current agent-browser command baseline"
