#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

STATE_FILE="$TMP_DIR/state.json"
wh_state_write "$STATE_FILE" '{"status":"booting"}'
wh_state_phase "$STATE_FILE" waiting_metadata "waiting for metadata" "$(wh_clock_ms)"
wh_state_progress "$STATE_FILE" "poll 1" 1

python3 - "$STATE_FILE" <<'PY'
import json
import sys

state = json.load(open(sys.argv[1], encoding="utf-8"))
assert state["phase"] == "waiting_metadata", state
assert state["progress"]["phase"] == "waiting_metadata", state
assert state["progress"]["message"] == "poll 1", state
assert state["progress"]["attempt"] == 1, state
assert isinstance(state["phase_started_epoch_ms"], int), state
PY

deadline="$(wh_clock_ms)"
set +e
wh_run_with_deadline "$((deadline + 100))" bash -c 'sleep 2' >/dev/null 2>&1
timeout_status=$?
set -e
[[ "$timeout_status" -eq 124 ]] || {
  echo "expected deadline watchdog status 124, got $timeout_status" >&2
  exit 1
}

wh_run_with_deadline "$((deadline + 3000))" bash -c 'sleep 0.05'

bash -n "$ROOT_DIR/scripts/worktree-harness.sh" "$ROOT_DIR/scripts/worktree-harness-lib.sh"
grep -Fq -- '--startup-timeout <secs>' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'wh_state_phase "$STATE_FILE" "waiting_metadata"' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'smoke_step "open" ab_open' "$ROOT_DIR/scripts/worktree-harness.sh"
grep -Fq -- 'smoke_step "screenshot" ab_screenshot' "$ROOT_DIR/scripts/worktree-harness.sh"
! grep -Fq -- 'seq 1 180' "$ROOT_DIR/scripts/worktree-harness.sh"

echo "worktree harness contract: PASS"
