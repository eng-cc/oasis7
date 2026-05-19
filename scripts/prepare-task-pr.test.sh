#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REAL_GIT="$(command -v git)"

TMPDIR="$(mktemp -d)"
cleanup() {
  "$REAL_GIT" -C "$ROOT_DIR" worktree remove -f "$TMPDIR/smoke-worktree" >/dev/null 2>&1 || true
  "$REAL_GIT" -C "$ROOT_DIR" branch -D temp/prepare-pr-copilot-test >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

SMOKE_WORKTREE="$TMPDIR/smoke-worktree"
SMOKE_BRANCH="temp/prepare-pr-copilot-test"

"$REAL_GIT" -C "$ROOT_DIR" worktree add "$SMOKE_WORKTREE" -b "$SMOKE_BRANCH" refs/remotes/origin/main >/dev/null
"$REAL_GIT" -C "$SMOKE_WORKTREE" commit --allow-empty --no-verify -m "test: prepare-task-pr smoke fixture" >/dev/null

mkdir -p "$TMPDIR/bin"
cat > "$TMPDIR/bin/git" <<EOF
#!/usr/bin/env bash
set -euo pipefail

REAL_GIT="$(printf '%s' "$REAL_GIT")"
LOG_FILE="\${TEST_GIT_LOG:?}"
printf '%s\n' "\$*" >> "\$LOG_FILE"

command_index=1
if [[ "\${1:-}" == "-C" ]]; then
  command_index=3
fi

case "\${!command_index:-}" in
  fetch|push)
    exit 0
    ;;
esac

exec "\$REAL_GIT" "\$@"
EOF
chmod +x "$TMPDIR/bin/git"

cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="${TEST_GH_LOG:?}"
printf '%s\n' "$*" >> "$LOG_FILE"

if [[ "${1:-}" == "pr" && "${2:-}" == "create" ]]; then
  printf 'https://github.com/example/oasis7/pull/999\n'
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "edit" ]]; then
  if [[ "${TEST_GH_EDIT_FAIL:-0}" == "1" ]]; then
    echo "review request unsupported" >&2
    exit 1
  fi
  exit 0
fi

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$TMPDIR/bin/gh"

success_log="$TMPDIR/gh-success.log"
success_git_log="$TMPDIR/git-success.log"
success_out="$TMPDIR/success.out"
success_err="$TMPDIR/success.err"
PATH="$TMPDIR/bin:$PATH" \
TEST_GH_LOG="$success_log" \
TEST_GIT_LOG="$success_git_log" \
"$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" --create >"$success_out" 2>"$success_err"

python3 - "$success_log" "$success_git_log" "$success_out" "$success_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

log_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_log_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")

if not log_lines or not log_lines[0].startswith("pr create "):
    raise SystemExit(f"expected first gh call to be pr create, got: {log_lines}")
if len(log_lines) < 2 or log_lines[1] != "pr edit temp/prepare-pr-copilot-test --add-reviewer @copilot":
    raise SystemExit(f"expected second gh call to request @copilot review, got: {log_lines}")
if "fetch --quiet origin main" not in git_log_lines:
    raise SystemExit(f"expected fetch attempt in git shim log, got: {git_log_lines}")
if not any(
    line.endswith("push -u origin temp/prepare-pr-copilot-test")
    or line.endswith("push origin temp/prepare-pr-copilot-test")
    for line in git_log_lines
):
    raise SystemExit(f"expected push attempt in git shim log, got: {git_log_lines}")
if "Created PR:" not in stdout or "https://github.com/example/oasis7/pull/999" not in stdout:
    raise SystemExit("expected created PR output")
if "warning:" in stderr:
    raise SystemExit(f"did not expect warning on success path: {stderr}")
PY

no_review_log="$TMPDIR/gh-no-review.log"
no_review_git_log="$TMPDIR/git-no-review.log"
PATH="$TMPDIR/bin:$PATH" \
TEST_GH_LOG="$no_review_log" \
TEST_GIT_LOG="$no_review_git_log" \
"$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" --create --no-copilot-review > /dev/null 2>"$TMPDIR/no-review.err"

python3 - "$no_review_log" "$no_review_git_log" "$TMPDIR/no-review.err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

log_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_log_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if log_lines != ["pr create --base main --head temp/prepare-pr-copilot-test --fill"]:
    raise SystemExit(f"expected only gh pr create without reviewer request, got: {log_lines}")
if "fetch --quiet origin main" not in git_log_lines:
    raise SystemExit(f"expected fetch attempt in opt-out path, got: {git_log_lines}")
if "warning:" in stderr:
    raise SystemExit(f"did not expect warning on opt-out path: {stderr}")
PY

fail_log="$TMPDIR/gh-fail.log"
fail_git_log="$TMPDIR/git-fail.log"
fail_out="$TMPDIR/fail.out"
fail_err="$TMPDIR/fail.err"
PATH="$TMPDIR/bin:$PATH" \
TEST_GH_LOG="$fail_log" \
TEST_GIT_LOG="$fail_git_log" \
TEST_GH_EDIT_FAIL=1 \
"$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" --create >"$fail_out" 2>"$fail_err"

python3 - "$fail_log" "$fail_git_log" "$fail_out" "$fail_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

log_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_log_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")

if not log_lines or log_lines[0] != "pr create --base main --head temp/prepare-pr-copilot-test --fill":
    raise SystemExit(f"expected create call even when reviewer request fails: {log_lines}")
if len(log_lines) < 2 or log_lines[1] != "pr edit temp/prepare-pr-copilot-test --add-reviewer @copilot":
    raise SystemExit(f"expected follow-up review request attempt on failure path: {log_lines}")
if not any(
    line.endswith("push -u origin temp/prepare-pr-copilot-test")
    or line.endswith("push origin temp/prepare-pr-copilot-test")
    for line in git_log_lines
):
    raise SystemExit(f"expected push attempt in failure path, got: {git_log_lines}")
if "Created PR:" not in stdout or "https://github.com/example/oasis7/pull/999" not in stdout:
    raise SystemExit("expected created PR output on reviewer failure path")
if "warning: PR created, but failed to request @copilot review via: gh pr edit temp/prepare-pr-copilot-test --add-reviewer @copilot" not in stderr:
    raise SystemExit(f"expected warning on reviewer failure path: {stderr}")
PY

echo "prepare-task-pr.test: OK"
