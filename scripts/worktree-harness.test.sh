#!/usr/bin/env bash
set -euo pipefail

# RED contract for worktree-harness deadlines.  This test deliberately uses
# fake command surfaces: it never opens a real browser, starts a real launcher,
# or waits for a real network service.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REAL_GIT="$(command -v git)"
REAL_MKDIR="$(command -v mkdir)"
REAL_RM="$(command -v rm)"

TMPDIR="$(mktemp -d)"
FIXTURE_ROOT="$TMPDIR/fixture worktree"
BIN_DIR="$TMPDIR/fake-bin"
mkdir -p "$FIXTURE_ROOT/scripts" "$BIN_DIR"

cleanup() {
  if [[ -x "$FIXTURE_ROOT/scripts/worktree-harness.sh" ]]; then
    (cd "$FIXTURE_ROOT" && PATH="$BIN_DIR:$PATH" "$FIXTURE_ROOT/scripts/worktree-harness.sh" down >/dev/null 2>&1) || true
  fi
  if [[ -n "${DUMMY_HARNESS_PID:-}" ]]; then
    wait "$DUMMY_HARNESS_PID" >/dev/null 2>&1 || true
  fi
  if [[ "${KEEP_FIXTURE:-0}" == "1" ]]; then
    echo "fixture retained at $TMPDIR" >&2
  else
    "$REAL_RM" -rf "$TMPDIR"
  fi
}
trap cleanup EXIT
FAILURES=0

record_failure() {
  FAILURES=$((FAILURES + 1))
  echo "$1" >&2
  if [[ -n "${2:-}" && -f "$2" ]]; then
    cat "$2" >&2
  fi
}

run_bounded() {
  local output_path=$1
  shift
  python3 - "$output_path" "$@" <<'PY'
from __future__ import annotations

import subprocess
import os
import signal
import sys
from pathlib import Path

output_path = Path(sys.argv[1])
command = sys.argv[2:]
process = subprocess.Popen(
    command,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    start_new_session=True,
)
try:
    # This is a dead-test watchdog, not the product deadline. Leave enough
    # margin for CI load and post-timeout process-group cleanup.
    stdout, stderr = process.communicate(timeout=15)
except subprocess.TimeoutExpired as exc:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    stdout, stderr = process.communicate()
    if isinstance(stdout, bytes):
        stdout = stdout.decode(errors="replace")
    if isinstance(stderr, bytes):
        stderr = stderr.decode(errors="replace")
    output_path.write_text(stdout + stderr, encoding="utf-8")
    raise SystemExit(124)
output_path.write_text((stdout or "") + (stderr or ""), encoding="utf-8")
raise SystemExit(process.returncode)
PY
}

"$REAL_GIT" -C "$FIXTURE_ROOT" init -q -b main
"$REAL_GIT" -C "$FIXTURE_ROOT" config user.email qa@example.invalid
"$REAL_GIT" -C "$FIXTURE_ROOT" config user.name "oasis7 qa fixture"
printf 'fixture\n' >"$FIXTURE_ROOT/README.md"
"$REAL_GIT" -C "$FIXTURE_ROOT" add README.md
"$REAL_GIT" -C "$FIXTURE_ROOT" commit -qm "fixture"

# Keep the harness entrypoint real while replacing only its external browser
# and launcher dependencies.
cp "$ROOT_DIR/scripts/worktree-harness.sh" "$FIXTURE_ROOT/scripts/worktree-harness.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$FIXTURE_ROOT/scripts/worktree-harness-lib.sh"
cp "$ROOT_DIR/scripts/agent-browser-lib.sh" "$FIXTURE_ROOT/scripts/agent-browser-lib.sh"
cp "$ROOT_DIR/scripts/viewer-web-dist-contract.sh" "$FIXTURE_ROOT/scripts/viewer-web-dist-contract.sh"
cp "$ROOT_DIR/scripts/bundle-freshness-lib.sh" "$FIXTURE_ROOT/scripts/bundle-freshness-lib.sh"
cp "$ROOT_DIR/scripts/worktree-harness-deadline.py" "$FIXTURE_ROOT/scripts/worktree-harness-deadline.py"
chmod +x "$FIXTURE_ROOT/scripts/worktree-harness.sh"

# curl only proves the fake viewer URL is reachable; no socket is opened.
cat >"$BIN_DIR/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
chmod +x "$BIN_DIR/curl"

# The browser opens and waits successfully, then hangs forever in eval.  A
# correct implementation must bound that operation with smoke --timeout and
# return a phase/deadline diagnostic before publishing last_smoke_ok.
cat >"$BIN_DIR/agent-browser" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ " $* " == *" eval "* ]]; then
    exec /bin/sleep 60
fi
case " $* " in
  *" close "*|*" open "*|*" wait "*)
    exit 0
    ;;
  *)
    echo "unexpected fake agent-browser command: $*" >&2
    exit 2
    ;;
esac
EOF
chmod +x "$BIN_DIR/agent-browser"

# Make the readiness fixture's polling loop deterministic and immediate.  The
# fake launcher itself stays alive but never writes session.meta, modelling a
# launcher that cannot cross its readiness boundary.
cat >"$BIN_DIR/sleep" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
chmod +x "$BIN_DIR/sleep"

WORKTREE_ID="$(cd "$FIXTURE_ROOT" && source scripts/worktree-harness-lib.sh && wh_worktree_id)"
HARNESS_ROOT="$FIXTURE_ROOT/output/harness/$WORKTREE_ID"
STATE_FILE="$HARNESS_ROOT/state.json"
mkdir -p "$HARNESS_ROOT"
(tail -f /dev/null) &
DUMMY_HARNESS_PID=$!
cat >"$STATE_FILE" <<JSON
{
  "status": "ready",
  "viewer_url": "http://fake-viewer.invalid/",
  "harness_pid": $DUMMY_HARNESS_PID
}
JSON

SMOKE_LOG="$TMPDIR/smoke.log"
SMOKE_STATUS=0
set +e
run_bounded "$SMOKE_LOG" env PATH="$BIN_DIR:$PATH" \
  "$FIXTURE_ROOT/scripts/worktree-harness.sh" smoke --timeout 1 \
  
SMOKE_STATUS=$?
set -e

if [[ "$SMOKE_STATUS" -eq 124 || "$SMOKE_STATUS" -eq 137 ]]; then
  record_failure "smoke deadline was not enforced; watchdog expired (status=$SMOKE_STATUS)" "$SMOKE_LOG"
fi
if [[ "$SMOKE_STATUS" -eq 0 ]]; then
  record_failure "smoke unexpectedly published success for a hanging browser eval" "$SMOKE_LOG"
fi
if ! grep -Eq 'phase=browser_(eval|operation)' "$SMOKE_LOG"; then
  record_failure "smoke failure must identify the browser phase" "$SMOKE_LOG"
fi
if ! grep -Eiq 'deadline|timeout' "$SMOKE_LOG"; then
  record_failure "smoke failure must include its deadline/timeout" "$SMOKE_LOG"
fi
if grep -q 'last_smoke_ok.*true' "$STATE_FILE"; then
  record_failure "smoke failure must not publish last_smoke_ok=true" "$STATE_FILE"
fi

# Replace the browser fixture with a harmless command for the readiness case;
# up never reaches smoke because its fake launcher cannot become ready.
cat >"$FIXTURE_ROOT/scripts/run-launcher-stack.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
while :; do :; done
EOF
chmod +x "$FIXTURE_ROOT/scripts/run-launcher-stack.sh"

# The smoke fixture keeps a sentinel harness PID alive so refresh_state does
# not rewrite it to stopped.  Reset that sentinel before exercising `up`.
kill "$DUMMY_HARNESS_PID" >/dev/null 2>&1 || true
wait "$DUMMY_HARNESS_PID" >/dev/null 2>&1 || true
unset DUMMY_HARNESS_PID
(cd "$FIXTURE_ROOT" && PATH="$BIN_DIR:$PATH" scripts/worktree-harness.sh down >/dev/null 2>&1) || true

READINESS_LOG="$TMPDIR/readiness.log"
READINESS_STATUS=0
set +e
run_bounded "$READINESS_LOG" env PATH="$BIN_DIR:$PATH" \
  "$FIXTURE_ROOT/scripts/worktree-harness.sh" up --source-mode --ready-timeout 1 \
  
READINESS_STATUS=$?
set -e

if [[ "$READINESS_STATUS" -eq 124 || "$READINESS_STATUS" -eq 137 ]]; then
  record_failure "readiness deadline was not enforced; watchdog expired (status=$READINESS_STATUS)" "$READINESS_LOG"
fi
if [[ "$READINESS_STATUS" -eq 0 ]]; then
  record_failure "readiness unexpectedly succeeded without session.meta" "$READINESS_LOG"
fi
if ! grep -q 'phase=launcher_readiness' "$READINESS_LOG"; then
  record_failure "readiness failure must identify launcher_readiness phase" "$READINESS_LOG"
fi
if ! grep -q 'timeout_secs=1' "$READINESS_LOG"; then
  record_failure "readiness failure must include configured deadline" "$READINESS_LOG"
fi
if grep -q 'timed out waiting for worktree harness readiness' "$READINESS_LOG"; then
  record_failure "readiness failure must not collapse to generic terminal timeout" "$READINESS_LOG"
fi

if [[ "$FAILURES" -ne 0 ]]; then
  echo "worktree harness deadline RED contract: $FAILURES expected assertion(s) still unmet" >&2
  exit 1
fi
printf '%s\n' 'worktree harness deadline contract: all assertions passed'
