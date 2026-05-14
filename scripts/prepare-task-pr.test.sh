#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  git -C "$ROOT_DIR" worktree remove -f "$TMPDIR/smoke-worktree" >/dev/null 2>&1 || true
  git -C "$ROOT_DIR" branch -D temp/prepare-pr-copilot-test >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

SMOKE_WORKTREE="$TMPDIR/smoke-worktree"
SMOKE_BRANCH="temp/prepare-pr-copilot-test"

git -C "$ROOT_DIR" worktree add "$SMOKE_WORKTREE" -b "$SMOKE_BRANCH" HEAD >/dev/null

mkdir -p "$TMPDIR/bin"
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
success_out="$TMPDIR/success.out"
success_err="$TMPDIR/success.err"
PATH="$TMPDIR/bin:$PATH" \
TEST_GH_LOG="$success_log" \
"$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" --create >"$success_out" 2>"$success_err"

python3 - "$success_log" "$success_out" "$success_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

log_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if not log_lines or not log_lines[0].startswith("pr create "):
    raise SystemExit(f"expected first gh call to be pr create, got: {log_lines}")
if len(log_lines) < 2 or log_lines[1] != "pr edit temp/prepare-pr-copilot-test --add-reviewer @copilot":
    raise SystemExit(f"expected second gh call to request @copilot review, got: {log_lines}")
if "Created PR:" not in stdout or "https://github.com/example/oasis7/pull/999" not in stdout:
    raise SystemExit("expected created PR output")
if "warning:" in stderr:
    raise SystemExit(f"did not expect warning on success path: {stderr}")
PY

no_review_log="$TMPDIR/gh-no-review.log"
PATH="$TMPDIR/bin:$PATH" \
TEST_GH_LOG="$no_review_log" \
"$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" --create --no-copilot-review > /dev/null 2>"$TMPDIR/no-review.err"

python3 - "$no_review_log" "$TMPDIR/no-review.err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

log_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[2]).read_text(encoding="utf-8")

if log_lines != ["pr create --base main --head temp/prepare-pr-copilot-test --fill"]:
    raise SystemExit(f"expected only gh pr create without reviewer request, got: {log_lines}")
if "warning:" in stderr:
    raise SystemExit(f"did not expect warning on opt-out path: {stderr}")
PY

fail_log="$TMPDIR/gh-fail.log"
fail_out="$TMPDIR/fail.out"
fail_err="$TMPDIR/fail.err"
PATH="$TMPDIR/bin:$PATH" \
TEST_GH_LOG="$fail_log" \
TEST_GH_EDIT_FAIL=1 \
"$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" --create >"$fail_out" 2>"$fail_err"

python3 - "$fail_log" "$fail_out" "$fail_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

log_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if not log_lines or log_lines[0] != "pr create --base main --head temp/prepare-pr-copilot-test --fill":
    raise SystemExit(f"expected create call even when reviewer request fails: {log_lines}")
if len(log_lines) < 2 or log_lines[1] != "pr edit temp/prepare-pr-copilot-test --add-reviewer @copilot":
    raise SystemExit(f"expected follow-up review request attempt on failure path: {log_lines}")
if "Created PR:" not in stdout or "https://github.com/example/oasis7/pull/999" not in stdout:
    raise SystemExit("expected created PR output on reviewer failure path")
if "warning: PR created, but failed to request @copilot review via: gh pr edit temp/prepare-pr-copilot-test --add-reviewer @copilot" not in stderr:
    raise SystemExit(f"expected warning on reviewer failure path: {stderr}")
PY

echo "prepare-task-pr.test: OK"
