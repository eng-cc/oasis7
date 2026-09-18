#!/usr/bin/env bash
set -euo pipefail

# TDD RED contract for the shared agent-browser lifecycle.  This file is
# intentionally tests-only: production scripts and manuals must satisfy the
# contract before the lifecycle refactor can be considered complete.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT_DIR/scripts/agent-browser-lib.sh"
PROMPT_SCRIPT="$ROOT_DIR/scripts/viewer-prompt-control-regression.sh"
PRODUCER_SCRIPT="$ROOT_DIR/scripts/run-producer-playtest.sh"
LAUNCHER_SCRIPT="$ROOT_DIR/scripts/run-launcher-stack.sh"
MANUAL="$ROOT_DIR/doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md"
MANUAL_PRD="$ROOT_DIR/doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md"

failures=0

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  failures=$((failures + 1))
}

assert_contains() {
  local path=$1
  local pattern=$2
  local label=$3
  if ! grep -Fq -- "$pattern" "$path"; then
    fail "$label: missing '$pattern' in $path"
  fi
}

assert_not_contains() {
  local path=$1
  local pattern=$2
  local label=$3
  if grep -Fq -- "$pattern" "$path"; then
    fail "$label: forbidden '$pattern' remains in $path"
  fi
}

assert_function() {
  local path=$1
  local name=$2
  if ! grep -Eq "^${name}[[:space:]]*\\(\\)" "$path"; then
    fail "shared lifecycle API '$name' is not defined in $path"
  fi
}

if [[ ! -f "$LIB" ]]; then
  fail "shared agent-browser library is missing: $LIB"
else
  assert_contains "$LIB" 'agent-browser@0.37.1' \
    "npx fallback pins the supported agent-browser version"

  # Public lifecycle contract.  The implementation may add helpers, but these
  # five entry points are the minimum required by every browser runner.
  assert_function "$LIB" ab_session_id
  assert_function "$LIB" ab_session_begin
  assert_function "$LIB" ab_session_cleanup
  assert_function "$LIB" ab_capture_diagnostics
  assert_function "$LIB" ab_read_retry

  # ab_open must not destroy a session it did not explicitly acquire.  A
  # scoped cleanup belongs in ab_session_cleanup, not in every open.
  open_body=$(sed -n '/^ab_open[[:space:]]*()/,/^}/p' "$LIB")
  if grep -Eq 'ab_run[[:space:]].*([[:space:]]|\")close([[:space:]]|\")' <<<"$open_body"; then
    fail "ab_open implicitly closes the session before opening it"
  fi
fi

# Prompt-control is a standalone browser runner and therefore must close the
# exact session it owns, while using the current wait API spelling.
assert_contains "$PROMPT_SCRIPT" 'ab_cmd "$SESSION" close' \
  "prompt-control cleanup closes its owned session"
assert_contains "$PROMPT_SCRIPT" 'wait --load domcontentloaded' \
  "prompt-control uses the current wait load option"
assert_not_contains "$PROMPT_SCRIPT" '--load-state networkidle' \
  "prompt-control does not use the removed wait load-state option"

# A long runner/worktree prefix must not be passed through to the native
# agent-browser socket name.  The compact form must remain deterministic and
# distinguish different long prefixes so concurrent agents retain isolation.
lifecycle_tmp="$(mktemp -d)"
prefix_capture="$lifecycle_tmp/prefixes.txt"
prefix_fake_bin="$lifecycle_tmp/bin"
mkdir -p "$prefix_fake_bin"
cat >"$prefix_fake_bin/agent-browser" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "session" && "${2:-}" == "id" ]]; then
  prefix=""
  while (($# > 0)); do
    if [[ "${1:-}" == "--prefix" ]]; then
      shift
      prefix="${1:-}"
    fi
    shift
  done
  printf '%s\n' "$prefix" >>"$AB_TEST_PREFIX_CAPTURE"
  printf '%s\n' 'prefix-regression-session'
  exit 0
fi
exit 0
EOF
chmod +x "$prefix_fake_bin/agent-browser"
long_prefix="$(printf '%180s' '' | tr ' ' 'x')"
PATH="$prefix_fake_bin:$PATH" AB_TEST_PREFIX_CAPTURE="$prefix_capture" \
  /bin/bash -c 'source "$1"; ab_session_id "$2" >/dev/null; ab_session_id "${2}b" >/dev/null' \
  _ "$LIB" "$long_prefix"
first_prefix="$(sed -n '1p' "$prefix_capture")"
second_prefix="$(sed -n '2p' "$prefix_capture")"
if [[ "${#first_prefix}" -gt 48 || "${#second_prefix}" -gt 48 ]]; then
  fail "long session prefix was not compacted below the socket-safe limit"
fi
if ! [[ "$first_prefix" =~ -[0-9a-f]{10}$ && "$second_prefix" =~ -[0-9a-f]{10}$ ]]; then
  fail "compacted session prefix does not carry the required short hash"
fi
if [[ "$first_prefix" == "$second_prefix" ]]; then
  fail "different long session prefixes collapsed to the same compact prefix"
fi

# Project-facing guidance must never ask a concurrent agent to terminate every
# browser session.  close-all is intentionally not searched in this test file.
assert_not_contains "$MANUAL" 'agent-browser close-all' \
  "Web manual does not perform global browser cleanup"
assert_not_contains "$MANUAL" 'agent-browser close --all' \
  "Web manual does not perform global browser cleanup"
assert_not_contains "$MANUAL_PRD" 'close-all' \
  "Web manual PRD does not prescribe global browser cleanup"
assert_not_contains "$MANUAL_PRD" 'close --all' \
  "Web manual PRD does not prescribe global browser cleanup"

# No live runner may use the stale wait spelling or global cleanup.  Keep this
# list explicit so the contract test cannot match its own assertions.
LIVE_BROWSER_SCRIPTS="
$ROOT_DIR/scripts/collect-active-llm-retention-sample.sh
$ROOT_DIR/scripts/run-game-test-ab.sh
$ROOT_DIR/scripts/run-playability-l4b-agent.sh
$ROOT_DIR/scripts/run-producer-playtest.sh
$ROOT_DIR/scripts/viewer-aw-test-completeness-playthrough.sh
$ROOT_DIR/scripts/viewer-gameplay-attraction-playthrough.sh
$ROOT_DIR/scripts/viewer-gameplay-attraction-ui-click-playthrough.sh
$ROOT_DIR/scripts/viewer-pixel-world-wasm-regression.sh
$ROOT_DIR/scripts/viewer-post-onboarding-qa.sh
$ROOT_DIR/scripts/viewer-primary-web-entry-regression.sh
$ROOT_DIR/scripts/viewer-prompt-control-regression.sh
$ROOT_DIR/scripts/viewer-software-safe-chat-regression.sh
$ROOT_DIR/scripts/viewer-software-safe-step-regression.sh
$ROOT_DIR/scripts/worktree-harness.sh
"
while IFS= read -r script_path; do
  [[ -n "$script_path" ]] || continue
  [[ -f "$script_path" ]] || continue
  assert_not_contains "$script_path" 'close-all' \
    "live runner avoids global browser cleanup"
  assert_not_contains "$script_path" 'close --all' \
    "live runner avoids global browser cleanup"
  assert_not_contains "$script_path" '--load-state networkidle' \
    "live runner uses current wait API"
done <<<"$LIVE_BROWSER_SCRIPTS"

# Fixed names are unsafe in a multi-agent worktree.  The producer runner must
# derive a scoped session, and the launcher usage example must not advertise a
# reusable global name.
assert_not_contains "$PRODUCER_SCRIPT" 'SESSION_NAME="producer-playtest"' \
  "producer playtest session is scoped/generated"
assert_not_contains "$LAUNCHER_SCRIPT" 'AGENT_BROWSER_SESSION=game-test-open' \
  "launcher browser example is scoped/generated"

if [[ "$failures" -ne 0 ]]; then
  printf 'agent-browser lifecycle contract: RED (%s failure(s))\n' "$failures" >&2
  exit 1
fi

printf 'agent-browser lifecycle contract: PASS\n'
