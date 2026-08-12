#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-codex-role-fit-binding.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

set +e
"$ROOT_DIR/scripts/pm/verify-codex-subagent-role-fit.sh" \
  >"$TMP_DIR/no-uid.out" 2>"$TMP_DIR/no-uid.err"
status=$?
set -e
if [[ "$status" != "2" ]]; then
  echo "codex-role-fit-task-binding.test: missing task UID must fail with usage status 2 (got $status)" >&2
  cat "$TMP_DIR/no-uid.err" >&2
  exit 1
fi
grep -F -- '--task-uid must be a 32-character task UID' "$TMP_DIR/no-uid.err" >/dev/null

if grep -Fq 'task_af5894a457964a9bb8bff5e8a4f87df1' \
  "$ROOT_DIR/scripts/pm/verify-codex-subagent-role-fit.sh"; then
  echo "codex-role-fit-task-binding.test: verifier still contains a hard-coded task UID" >&2
  exit 1
fi
grep -F -- '--task-uid "$TASK_UID"' \
  "$ROOT_DIR/scripts/pm/verify-codex-subagent-role-fit.sh" >/dev/null
grep -F -- 'codex_subagent_role_fit requires --task-uid' \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" >/dev/null

# Exercise the lifecycle profile dispatch with a fixture verifier.  This keeps
# the regression focused while proving claim-ready passes the selected UID,
# rather than only matching the source text.
FIXTURE="$TMP_DIR/claim-ready-fixture"
mkdir -p "$FIXTURE/scripts/pm"
cp "$ROOT_DIR/scripts/pm/claim-ready.sh" "$FIXTURE/scripts/pm/claim-ready.sh"
cp "$ROOT_DIR/scripts/pm/repo-state-fingerprint.py" "$FIXTURE/scripts/pm/repo-state-fingerprint.py"
cat >"$FIXTURE/scripts/pm/verify-codex-subagent-role-fit.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >"${ROLE_FIT_ARGS_FILE:?}"
SH
chmod +x "$FIXTURE/scripts/pm/claim-ready.sh" "$FIXTURE/scripts/pm/verify-codex-subagent-role-fit.sh"
git -C "$FIXTURE" init -q
git -C "$FIXTURE" config user.email test@example.com
git -C "$FIXTURE" config user.name Test
git -C "$FIXTURE" commit --allow-empty -qm fixture
uid="task_c0f8dbb9cb254e7d9d68566b0c9fea47"
ROLE_FIT_ARGS_FILE="$TMP_DIR/role-fit.args" PM_ROOT_DIR="$FIXTURE" \
  "$FIXTURE/scripts/pm/claim-ready.sh" \
  --claim-type tests_passed --verification-profile codex_subagent_role_fit \
  --task-uid "$uid" --json >"$TMP_DIR/claim-ready.json"
grep -Fx -- "--task-uid $uid" "$TMP_DIR/role-fit.args" >/dev/null

echo "codex-role-fit-task-binding.test: OK"
