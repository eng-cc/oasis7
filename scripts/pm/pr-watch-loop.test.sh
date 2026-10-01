#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
cat >"$TMP/bin/python3" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == *pr-lifecycle-gate.py ]]; then
  printf '%s\n' "$*" >"$TEST_ARGS"
  printf '%s\n' "${TEST_GATE_PAYLOAD:-{\"evidence_mode\":\"observation\",\"status\":\"observed\",\"candidate_ready\":true,\"ready_for_merge\":false,\"requires_live_gate\":true,\"formal_gate_command\":\"python3 scripts/pm/pr-lifecycle-gate.py 1 --task-uid task_11111111111111111111111111111111 --json\"}}"
  exit "${TEST_GATE_RC:-0}"
fi
exec /usr/bin/python3 "$@"
SH
chmod +x "$TMP/bin/python3"
export TEST_ARGS="$TMP/args"

PATH="$TMP/bin:$PATH" bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 --task-uid task_11111111111111111111111111111111 --root /tmp/canonical >"$TMP/candidate.out"
grep -q -- '--observe --watch --json' "$TEST_ARGS"
grep -q -- '--task-uid task_11111111111111111111111111111111' "$TEST_ARGS"
grep -q '"candidate_ready":true' "$TMP/candidate.out"
grep -q '"ready_for_merge":false' "$TMP/candidate.out"
if grep -q 'readiness_receipt' "$TMP/candidate.out"; then
  echo "observation watcher emitted a readiness receipt" >&2
  exit 1
fi

set +e
PATH="$TMP/bin:$PATH" TEST_GATE_RC=75 \
  TEST_GATE_PAYLOAD='{"evidence_mode":"observation","status":"external_wait","ready_for_merge":false,"candidate_ready":false,"requires_live_gate":true}' \
  bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 --task-uid task_11111111111111111111111111111111 >"$TMP/wait.out"
rc=$?
set -e
[[ "$rc" == 75 ]]
grep -q '"status":"external_wait"' "$TMP/wait.out"

set +e
PATH="$TMP/bin:$PATH" TEST_GATE_RC=2 \
  TEST_GATE_PAYLOAD='{"evidence_mode":"observation","status":"capability_blocked","ready_for_merge":false,"candidate_ready":false,"requires_live_gate":true}' \
  bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 --task-uid task_11111111111111111111111111111111 >"$TMP/blocked.out"
rc=$?
set -e
[[ "$rc" == 2 ]]
grep -q '"status":"capability_blocked"' "$TMP/blocked.out"

echo "pr-watch-loop.test: OK"
