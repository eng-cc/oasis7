#!/usr/bin/env bash
# Cross-platform contract: missing PM acceptance must fail before worktree,
# branch, or PM/GitHub mutation.  This is intentionally a RED test for the
# bootstrap preflight fix.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
TARGET="$TMPDIR/task-worktree"
BRANCH="task/engineering-missing-acceptance"
PM_MARKER="$TMPDIR/pm-invoked"
mkdir -p "$REPO/scripts/pm"

cp "$ROOT_DIR/scripts/new-task-worktree.sh" "$REPO/scripts/new-task-worktree.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$REPO/scripts/worktree-harness-lib.sh"
cp "$ROOT_DIR/scripts/pm/find-python-with-module.sh" "$REPO/scripts/pm/find-python-with-module.sh"
cp "$ROOT_DIR/scripts/pm/pm_store.py" "$REPO/scripts/pm/pm_store.py"
cat >"$REPO/scripts/cargo-dev.sh" <<'EOF'
#!/usr/bin/env bash
[[ "${1:-}" == "--print-target-dir" ]]
printf '%s\n' "${TEST_SHARED_TARGET:?}"
EOF
cat >"$REPO/scripts/pm/new-task.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
: >"${PM_BOOTSTRAP_MARKER:?}"
echo "new-task PM mutation was reached before acceptance preflight" >&2
exit 73
EOF
chmod +x "$REPO/scripts/new-task-worktree.sh" "$REPO/scripts/cargo-dev.sh" "$REPO/scripts/pm/new-task.sh"

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" add .
git -C "$REPO" commit -qm fixture

set +e
(
  cd "$REPO"
  TEST_SHARED_TARGET="$TMPDIR/shared-target" PM_BOOTSTRAP_MARKER="$PM_MARKER" \
    ./scripts/new-task-worktree.sh engineering missing-acceptance \
      --branch "$BRANCH" \
      --path "$TARGET" \
      --pm-owner-role repository_health_engineer \
      --pm-title "missing acceptance fixture" \
      --pm-source-ref doc/engineering/workflow/source-of-truth.md
) >"$TMPDIR/bootstrap.out" 2>"$TMPDIR/bootstrap.err"
STATUS=$?
set -e

FAILURES=0
if [[ "$STATUS" -eq 0 ]]; then
  echo "missing PM acceptance must fail closed" >&2
  FAILURES=$((FAILURES + 1))
fi
if ! grep -Eiq 'acceptance|--pm-acceptance' "$TMPDIR/bootstrap.err"; then
  echo "failure must identify the missing --pm-acceptance input" >&2
  FAILURES=$((FAILURES + 1))
fi
if [[ -e "$TARGET" ]]; then
  echo "missing acceptance created a worktree before preflight: $TARGET" >&2
  FAILURES=$((FAILURES + 1))
fi
if git -C "$REPO" show-ref --verify --quiet "refs/heads/$BRANCH"; then
  echo "missing acceptance created a branch before preflight: $BRANCH" >&2
  FAILURES=$((FAILURES + 1))
fi
if [[ -e "$PM_MARKER" ]]; then
  echo "missing acceptance reached PM/GitHub mutation" >&2
  FAILURES=$((FAILURES + 1))
fi

if [[ "$FAILURES" -ne 0 ]]; then
  echo "new-task-worktree acceptance pre-mutation regression: $FAILURES assertion(s) failed" >&2
  cat "$TMPDIR/bootstrap.err" >&2
  exit 1
fi

# A supplied acceptance value must be meaningful too; blank/whitespace input
# must fail at the same pre-mutation boundary as an omitted value.
BLANK_TARGET="$TMPDIR/blank-task-worktree"
BLANK_BRANCH="task/engineering-blank-acceptance"
set +e
(
  cd "$REPO"
  TEST_SHARED_TARGET="$TMPDIR/shared-target" PM_BOOTSTRAP_MARKER="$PM_MARKER" \
    ./scripts/new-task-worktree.sh engineering blank-acceptance \
      --branch "$BLANK_BRANCH" \
      --path "$BLANK_TARGET" \
      --pm-owner-role repository_health_engineer \
      --pm-title "blank acceptance fixture" \
      --pm-source-ref doc/engineering/workflow/source-of-truth.md \
      --pm-acceptance "   "
) >"$TMPDIR/blank.out" 2>"$TMPDIR/blank.err"
BLANK_STATUS=$?
set -e

if [[ "$BLANK_STATUS" -eq 0 ]] || ! grep -Eiq 'acceptance|--pm-acceptance' "$TMPDIR/blank.err"; then
  echo "blank PM acceptance must fail with an actionable error" >&2
  cat "$TMPDIR/blank.err" >&2
  exit 1
fi
if [[ -e "$BLANK_TARGET" ]] || git -C "$REPO" show-ref --verify --quiet "refs/heads/$BLANK_BRANCH"; then
  echo "blank acceptance mutated the worktree or branch before preflight" >&2
  exit 1
fi
if [[ -e "$PM_MARKER" ]]; then
  echo "blank acceptance reached PM/GitHub mutation" >&2
  exit 1
fi

echo "new-task-worktree-acceptance-pre-mutation.test: OK"
