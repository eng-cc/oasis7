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
  'AB_SESSION="$(agent-browser session id --scope worktree --prefix viewer)"'
  'agent-browser --session "$AB_SESSION" --headed open "http://127.0.0.1:${VIEWER_CAPTURE_PORT}/viewer.html?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"'
  'agent-browser --session "$AB_SESSION" wait --load domcontentloaded'
  'agent-browser --session "$AB_SESSION" snapshot -i'
  'agent-browser --session "$AB_SESSION" close'
)

MIRROR_REQUIRED_PATTERNS=(
  'command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }'
  'AB_SESSION="$(agent-browser session id --scope worktree --prefix viewer)"'
  'agent-browser --session "$AB_SESSION" --headed open "http://127.0.0.1:${VIEWER_CAPTURE_PORT}/viewer.html?ws=ws://127.0.0.1:5011&amp;render_mode=viewer&amp;test_api=1"'
  'agent-browser --session "$AB_SESSION" wait --load domcontentloaded'
  'agent-browser --session "$AB_SESSION" snapshot -i'
  'agent-browser --session "$AB_SESSION" close'
)

FORBIDDEN_PATTERNS=(
  'export REPO_ROOT="$(pwd)"'
  'agent-browser close-all'
  'agent-browser --headed open '
  'agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"'
  'agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&amp;render_mode=viewer&amp;test_api=1"'
  'agent-browser wait --load networkidle'
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

# Check the actual capture/build code blocks, not prose elsewhere on the page.
# HTML decoding gives the source and both mirrors the same shell contract.
python3 - "$SOURCE_MANUAL" "${MIRROR_MANUALS[@]}" <<'PYTHON'
import html
from pathlib import Path
import re
import subprocess
import sys

build_recipe = r'''npm --prefix crates/oasis7_viewer run build:viewer:visual-test && \
python3 -m http.server 4174 --bind 127.0.0.1 --directory crates/oasis7_viewer/.viewer-test-dist'''
# These controlled examples intentionally use a complete command skeleton.
# Ignoring only whole-line comments/blank lines prevents dead branches, here-docs
# and commented-out required commands from satisfying a substring-only check.
def commands(block):
    return [line.strip() for line in block.splitlines()
            if line.strip() and not line.lstrip().startswith("#")]

capture_recipe = r'''set -euo pipefail
VIEWER_CAPTURE_MODE="${VIEWER_CAPTURE_MODE:-release}"
case "$VIEWER_CAPTURE_MODE" in
  release) VIEWER_CAPTURE_PORT=4173 ;;
  visual-test) VIEWER_CAPTURE_PORT=4174 ;;
  *) echo "unknown VIEWER_CAPTURE_MODE: $VIEWER_CAPTURE_MODE" >&2; exit 2 ;;
esac
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
agent-browser --version
agent-browser doctor --offline --quick --json
AB_SESSION="$(agent-browser session id --scope worktree --prefix viewer-manual)"
export AB_SESSION
mkdir -p output/playwright/viewer
VIEWER_EVIDENCE_DIR="$(mktemp -d output/playwright/viewer/viewer-manual.XXXXXX)"
cleanup() {
  local main_status=$?
  local close_status=0
  trap - EXIT INT TERM
  if agent-browser --session "$AB_SESSION" close >"$VIEWER_EVIDENCE_DIR/close.log" 2>&1; then
    close_status=0
  else
    close_status=$?
  fi
  if (( close_status != 0 )); then
    printf 'cleanup blocker: close exited %s; see %s/close.log\n' "$close_status" "$VIEWER_EVIDENCE_DIR" >&2
  fi
  if ! printf 'main_status=%s\nclose_status=%s\n' "$main_status" "$close_status" >"$VIEWER_EVIDENCE_DIR/cleanup.status"; then
    printf 'could not write cleanup status\n' >&2
  fi
  if (( main_status != 0 )); then
    exit "$main_status"
  fi
  exit "$close_status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
agent-browser --session "$AB_SESSION" --headed open "http://127.0.0.1:${VIEWER_CAPTURE_PORT}/viewer.html?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"
agent-browser --session "$AB_SESSION" wait --load domcontentloaded
if [[ "$VIEWER_CAPTURE_MODE" == "visual-test" ]]; then
  agent-browser --session "$AB_SESSION" wait --fn "typeof window.__AW_TEST__ === 'object'"
else
  agent-browser --session "$AB_SESSION" wait --fn "typeof window.__AW_TEST__ === 'undefined'"
fi
agent-browser --session "$AB_SESSION" session info --json | tee "$VIEWER_EVIDENCE_DIR/session-info.json"
agent-browser --session "$AB_SESSION" snapshot -i
if [[ "$VIEWER_CAPTURE_MODE" == "visual-test" ]]; then
  agent-browser --session "$AB_SESSION" eval "JSON.stringify(window.__AW_TEST__.getState())" | tee "$VIEWER_EVIDENCE_DIR/state.json"
fi
agent-browser --session "$AB_SESSION" console | tee "$VIEWER_EVIDENCE_DIR/console.log"
agent-browser --session "$AB_SESSION" screenshot "$VIEWER_EVIDENCE_DIR/viewer-web.png"'''
mirror_recipe = capture_recipe.replace(
    '--prefix viewer-manual)', '--prefix viewer)').replace(
    'mktemp -d output/playwright/viewer/viewer-manual.XXXXXX',
    'mktemp -d "output/playwright/viewer/${AB_SESSION}.XXXXXX"').replace(
    'cleanup blocker: close exited %s;', 'cleanup blocker: close_status=%s;')
for name in sys.argv[1:]:
    path = Path(name)
    text = path.read_text()
    if path.suffix == ".html":
        blocks = [html.unescape(block) for block in re.findall(r"<pre><code>(.*?)</code></pre>", text, re.S)]
    else:
        blocks = re.findall(r"```bash\n(.*?)```", text, re.S)
    for block in blocks:
        for line in commands(block):
            if not line.startswith("agent-browser "):
                continue
            if (re.search(r"\bclose-all\b|\bwait --load networkidle\b", line)
                    or (re.search(r"\bopen\s", line) and '--session "$AB_SESSION"' not in line)):
                raise SystemExit(f"error: unsafe browser command in {path}: {line}")
    captures = [block for block in blocks if 'VIEWER_CAPTURE_MODE=' in block]
    if len(captures) != 1:
        raise SystemExit(f"error: expected one profile-aware capture block in {path}")
    if build_recipe not in [block.strip() for block in blocks]:
        raise SystemExit(f"error: missing isolated visual-test build-and-serve recipe in {path}")
    capture = captures[0]
    expected = mirror_recipe if path.suffix == ".html" else capture_recipe
    if commands(capture) != commands(expected):
        raise SystemExit(f"error: capture command/profile/owned-cleanup contract differs in {path}")
    for block in (capture, build_recipe):
        result = subprocess.run(["bash", "-n"], input=block, text=True, capture_output=True)
        if result.returncode:
            raise SystemExit(f"error: invalid capture/build shell syntax in {path}: {result.stderr}")
PYTHON

echo "ok: viewer manual static mirrors are synced with the current agent-browser command baseline"
