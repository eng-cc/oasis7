#!/usr/bin/env bash
# Cross-platform test contract: partial bootstrap recovery remains valid on Windows Git Bash and Linux/macOS shells.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REAL_GIT="$(command -v git)"
TMPDIR="$(mktemp -d)"
cleanup() {
  "$REAL_GIT" -C "$TMPDIR/repo" worktree remove --force "$TMPDIR/task-worktree" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

REPO="$TMPDIR/repo"
TARGET="$TMPDIR/task-worktree"
BRANCH="task/engineering-partial-bootstrap"
mkdir -p "$REPO/scripts/pm"
cp "$ROOT_DIR/scripts/new-task-worktree.sh" "$REPO/scripts/new-task-worktree.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$REPO/scripts/worktree-harness-lib.sh"
cp "$ROOT_DIR/scripts/pm/find-python-with-module.sh" "$REPO/scripts/pm/find-python-with-module.sh"
cp "$ROOT_DIR/scripts/pm/pm_store.py" "$REPO/scripts/pm/pm_store.py"
chmod +x "$REPO/scripts/new-task-worktree.sh"

cat >"$REPO/scripts/cargo-dev.sh" <<'EOF'
#!/usr/bin/env bash
[[ "${1:-}" == "--print-target-dir" ]]
printf '%s\n' "${TEST_SHARED_TARGET:?}"
EOF
cat >"$REPO/scripts/pm/new-task.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p .pm/scratch/bootstrap
cat >.pm/scratch/bootstrap/partial-remote.json <<'JSON'
{"task_uid":"task_11111111111111111111111111111111","issue_url":"https://example.invalid/issues/2198","stage":"issue_created"}
JSON
echo 'injected failure after remote issue creation' >&2
exit 77
EOF
cat >"$REPO/scripts/pm/move-task.sh" <<'EOF'
#!/usr/bin/env bash
exit 99
EOF
cat >"$REPO/scripts/pm/workflow-report.sh" <<'EOF'
#!/usr/bin/env bash
exit 99
EOF
chmod +x "$REPO/scripts/cargo-dev.sh" "$REPO/scripts/pm/"*.sh

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" add .
git -C "$REPO" commit -qm fixture

set +e
(cd "$REPO" && TEST_SHARED_TARGET="$TMPDIR/shared-target" \
  ./scripts/new-task-worktree.sh engineering partial-bootstrap \
    --branch "$BRANCH" --path "$TARGET" \
    --pm-owner-role tpm --pm-title "partial bootstrap fixture" \
    --pm-source-ref doc/engineering/project.md \
    --pm-acceptance "partial remote bootstrap remains resumable") \
  >"$TMPDIR/bootstrap.out" 2>"$TMPDIR/bootstrap.err"
status=$?
set -e
if [[ "$status" != "77" ]]; then
  echo "expected injected bootstrap failure 77, got $status" >&2
  cat "$TMPDIR/bootstrap.err" >&2
  exit 1
fi

if [[ ! -d "$TARGET" ]]; then
  echo "partial remote bootstrap must preserve the canonical task worktree for recovery" >&2
  exit 1
fi
git -C "$REPO" show-ref --verify --quiet "refs/heads/$BRANCH"
test -f "$TARGET/.pm/scratch/bootstrap/partial-remote.json"
grep -F "resume-bootstrap" "$TMPDIR/bootstrap.err" >/dev/null
if grep -Fq "cleaned up created worktree" "$TMPDIR/bootstrap.err"; then
  echo "partial remote bootstrap must not claim destructive cleanup" >&2
  exit 1
fi

echo "new-task-worktree-partial-bootstrap.test: OK"
