#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REAL_GIT="$(command -v git)"

TMPDIR="$(mktemp -d)"
cleanup() {
  "$REAL_GIT" -C "$ROOT_DIR" worktree remove -f "$TMPDIR/smoke-worktree" >/dev/null 2>&1 || true
  "$REAL_GIT" -C "$ROOT_DIR" branch -D temp/prepare-pr-role-review-test >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

SMOKE_WORKTREE="$TMPDIR/smoke-worktree"
SMOKE_BRANCH="temp/prepare-pr-role-review-test"
TASK_UID="task_11111111111111111111111111111111"

"$REAL_GIT" -C "$ROOT_DIR" worktree add "$SMOKE_WORKTREE" -b "$SMOKE_BRANCH" refs/remotes/origin/main >/dev/null
"$REAL_GIT" -C "$SMOKE_WORKTREE" \
  -c user.name="oasis7 smoke" \
  -c user.email="smoke@example.invalid" \
  -c commit.gpgsign=false \
  commit --allow-empty --no-verify -m "test: prepare-task-pr smoke fixture" >/dev/null

SMOKE_WORKTREE_CANONICAL="$(cd "$SMOKE_WORKTREE" && pwd -P)"
SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"

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

if [[ -n "\${TEST_REV_LIST_COUNTS:-}" && "\${!command_index:-}" == "rev-list" ]]; then
  printf '%s\n' "\$TEST_REV_LIST_COUNTS"
  exit 0
fi

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

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$TMPDIR/bin/gh"

write_task_binding() {
  cat > "$SMOKE_WORKTREE/.pm/tasks/$TASK_UID.yaml" <<EOF
task_uid: $TASK_UID
title: "prepare task pr role review fixture"
owner_role: tpm
worktree_hint: $SMOKE_WORKTREE_CANONICAL
execution_log_path: .pm/tasks/$TASK_UID.execution.md
status: committed
priority: P3
source_signal: null
source_refs: []
doc_refs: []
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-03T00:00:00+08:00
last_started_at: 2026-06-03T00:00:00+08:00
EOF
}

write_role_review_packet() {
  local source_head="$1"
  local disposition="$2"
  cat > "$SMOKE_WORKTREE/.pm/tasks/$TASK_UID.execution.md" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: prepare task pr role review fixture
- owner_role: tpm
- worktree_hint: $SMOKE_WORKTREE_CANONICAL

## 2026-06-03 00:00:00 CST / tpm
- 完成内容: fixture pre-PR local role review packet.
- 遗留事项: none.
- Action: fixture.
- Validation Command: fixture.
- Expected Result: fixture.
- Actual Result: fixture.
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SMOKE_WORKTREE_CANONICAL
- Source Branch: $SMOKE_BRANCH
- Source Head: $source_head
- Comparison Ref: refs/remotes/origin/main
- Reviewed Changed Paths: scripts/prepare-task-pr.sh
- Role Selection Basis: changed paths include PR helper workflow; roles tpm,qa_engineer.
- Review Roles: tpm,qa_engineer
- Review Evidence: qa_engineer: 2026-06-03 00:00:00 CST; no_findings; fixture
- Review Findings Disposition: $disposition
- Finding Disposition Evidence: fixture evidence
- Residual Risk: fixture residual risk
- Blocker / Next Action: none.
EOF
}

write_shadowed_role_review_packet() {
  local source_head="$1"
  cat > "$SMOKE_WORKTREE/.pm/tasks/$TASK_UID.execution.md" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: prepare task pr role review fixture
- owner_role: tpm
- worktree_hint: $SMOKE_WORKTREE_CANONICAL

## 2026-06-03 00:00:00 CST / tpm
- 完成内容: fixture earlier integration entry.
- 遗留事项: none.
- Action: fixture.
- Validation Command: fixture.
- Expected Result: fixture.
- Actual Result: fixture.
- Review Findings Disposition: addressed.
- Residual Risk: earlier non-packet risk.
- Blocker / Next Action: none.

## 2026-06-03 00:01:00 CST / tpm
- 完成内容: fixture final pre-PR local role review packet.
- 遗留事项: none.
- Action: fixture.
- Validation Command: fixture.
- Expected Result: fixture.
- Actual Result: fixture.
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SMOKE_WORKTREE_CANONICAL
- Source Branch: $SMOKE_BRANCH
- Source Head: $source_head
- Comparison Ref: refs/remotes/origin/main
- Reviewed Changed Paths: scripts/prepare-task-pr.sh
- Role Selection Basis: changed paths include PR helper workflow; roles tpm,qa_engineer.
- Review Roles: tpm,qa_engineer
- Review Evidence: qa_engineer: 2026-06-03 00:01:00 CST; no_findings; fixture
- Review Findings Disposition: no_findings
- Finding Disposition Evidence: fixture evidence
- Residual Risk: final fixture residual risk
- Blocker / Next Action: none.
EOF
}

write_prefix_mismatch_role_review_packet() {
  local source_head="$1"
  cat > "$SMOKE_WORKTREE/.pm/tasks/$TASK_UID.execution.md" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: prepare task pr role review fixture
- owner_role: tpm
- worktree_hint: $SMOKE_WORKTREE_CANONICAL

## 2026-06-03 00:00:00 CST / tpm
- 完成内容: fixture pre-PR local role review packet with prefix-mismatched fields.
- 遗留事项: none.
- Action: fixture.
- Validation Command: fixture.
- Expected Result: fixture.
- Actual Result: fixture.
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SMOKE_WORKTREE_CANONICAL-old
- Source Branch: $SMOKE_BRANCH-old
- Source Head: $source_head
- Comparison Ref: refs/remotes/origin/main-old
- Reviewed Changed Paths: scripts/prepare-task-pr.sh
- Role Selection Basis: changed paths include PR helper workflow; roles tpm,qa_engineer.
- Review Roles: tpm,qa_engineer
- Review Evidence: qa_engineer: 2026-06-03 00:00:00 CST; no_findings; fixture
- Review Findings Disposition: no_findings
- Finding Disposition Evidence: fixture evidence
- Residual Risk: fixture residual risk
- Blocker / Next Action: none.
EOF
}

commit_fixture_evidence() {
  "$REAL_GIT" -C "$SMOKE_WORKTREE" add ".pm/tasks/$TASK_UID.yaml" ".pm/tasks/$TASK_UID.execution.md"
  "$REAL_GIT" -C "$SMOKE_WORKTREE" \
    -c user.name="oasis7 smoke" \
    -c user.email="smoke@example.invalid" \
    -c commit.gpgsign=false \
    commit --no-verify -m "test: pre-pr local role review evidence" >/dev/null
}

run_prepare() {
  local gh_log="$1"
  local git_log="$2"
  shift 2
  : > "$gh_log"
  : > "$git_log"
  PATH="$TMPDIR/bin:$PATH" \
    TEST_GH_LOG="$gh_log" \
    TEST_GIT_LOG="$git_log" \
    "$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" "$@"
}

missing_log="$TMPDIR/gh-missing.log"
missing_git_log="$TMPDIR/git-missing.log"
missing_out="$TMPDIR/missing.out"
missing_err="$TMPDIR/missing.err"
if run_prepare "$missing_log" "$missing_git_log" --create >"$missing_out" 2>"$missing_err"; then
  echo "expected --create to fail without local role review packet" >&2
  exit 1
fi

python3 - "$missing_log" "$missing_git_log" "$missing_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if gh_lines:
    raise SystemExit(f"expected no gh calls before missing-review failure, got: {gh_lines}")
if any("push" in line for line in git_lines):
    raise SystemExit(f"expected no push before missing-review failure, got: {git_lines}")
if "missing passed pre-PR local role review evidence" not in stderr:
    raise SystemExit(f"expected missing-review error, got: {stderr}")
PY

write_task_binding
write_role_review_packet "0000000000000000000000000000000000000000" "no_findings"
commit_fixture_evidence

stale_log="$TMPDIR/gh-stale.log"
stale_git_log="$TMPDIR/git-stale.log"
stale_err="$TMPDIR/stale.err"
if run_prepare "$stale_log" "$stale_git_log" --create >/dev/null 2>"$stale_err"; then
  echo "expected --create to fail with stale source head" >&2
  exit 1
fi

python3 - "$stale_log" "$stale_git_log" "$stale_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if gh_lines:
    raise SystemExit(f"expected no gh calls before stale-review failure, got: {gh_lines}")
if any("push" in line for line in git_lines):
    raise SystemExit(f"expected no push before stale-review failure, got: {git_lines}")
if "Source Head ancestor" not in stderr:
    raise SystemExit(f"expected stale Source Head marker in error, got: {stderr}")
PY

write_prefix_mismatch_role_review_packet "$SOURCE_HEAD"
commit_fixture_evidence

prefix_mismatch_json="$TMPDIR/prefix-mismatch.json"
run_prepare "$TMPDIR/gh-prefix-mismatch.log" "$TMPDIR/git-prefix-mismatch.log" --json >"$prefix_mismatch_json"

python3 - "$prefix_mismatch_json" "$SMOKE_WORKTREE_CANONICAL" "$SMOKE_BRANCH" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
expected_worktree = sys.argv[2]
expected_branch = sys.argv[3]
review = payload["pre_pr_local_role_review"]
missing = set(review["missing_markers"])
expected = {
    f"Source Worktree: {expected_worktree}",
    f"Source Branch: {expected_branch}",
    "Comparison Ref: refs/remotes/origin/main",
}
if review["status"] != "missing":
    raise SystemExit(f"expected missing review status for prefix-mismatched fields, got: {review}")
if not expected.issubset(missing):
    raise SystemExit(f"expected exact field mismatch markers {expected}, got: {missing}")
PY

write_role_review_packet "$SOURCE_HEAD" "no_findings"
commit_fixture_evidence

success_log="$TMPDIR/gh-success.log"
success_git_log="$TMPDIR/git-success.log"
success_out="$TMPDIR/success.out"
success_err="$TMPDIR/success.err"
run_prepare "$success_log" "$success_git_log" --create >"$success_out" 2>"$success_err"

python3 - "$success_log" "$success_git_log" "$success_out" "$success_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")

if gh_lines != ["pr create --base main --head temp/prepare-pr-role-review-test --fill"]:
    raise SystemExit(f"expected only gh pr create and no reviewer API calls, got: {gh_lines}")
if not any(
    line.endswith("push -u origin temp/prepare-pr-role-review-test")
    or line.endswith("push origin temp/prepare-pr-role-review-test")
    for line in git_lines
):
    raise SystemExit(f"expected push attempt after valid review packet, got: {git_lines}")
if "Created PR:" not in stdout or "https://github.com/example/oasis7/pull/999" not in stdout:
    raise SystemExit("expected created PR output")
if "Pre-PR Local Role Review:" not in stdout or "- status: passed" not in stdout:
    raise SystemExit("expected local role review status in output")
if stderr:
    raise SystemExit(f"did not expect stderr on success path: {stderr}")
PY

behind_log="$TMPDIR/gh-behind.log"
behind_git_log="$TMPDIR/git-behind.log"
behind_out="$TMPDIR/behind.out"
behind_err="$TMPDIR/behind.err"
TEST_REV_LIST_COUNTS="1 2" run_prepare "$behind_log" "$behind_git_log" --create >"$behind_out" 2>"$behind_err"

python3 - "$behind_log" "$behind_git_log" "$behind_out" "$behind_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")

if gh_lines != ["pr create --base main --head temp/prepare-pr-role-review-test --fill"]:
    raise SystemExit(f"expected gh pr create on behind-but-allowed path, got: {gh_lines}")
if not any(
    line.endswith("push -u origin temp/prepare-pr-role-review-test")
    or line.endswith("push origin temp/prepare-pr-role-review-test")
    for line in git_lines
):
    raise SystemExit(f"expected push attempt on behind-but-allowed path, got: {git_lines}")
if "- behind base: 1" not in stdout or "- branch sync suggested: suggested" not in stdout:
    raise SystemExit(f"expected behind advisory in output, got: {stdout}")
if "Suggested branch sync before merge if GitHub later requires it:" not in stdout:
    raise SystemExit(f"expected non-blocking branch-sync suggestion, got: {stdout}")
if stderr:
    raise SystemExit(f"did not expect stderr on behind-but-allowed path: {stderr}")
PY

write_role_review_packet "$SOURCE_HEAD" "addressed"
commit_fixture_evidence

addressed_log="$TMPDIR/gh-addressed.log"
addressed_git_log="$TMPDIR/git-addressed.log"
addressed_out="$TMPDIR/addressed.out"
addressed_err="$TMPDIR/addressed.err"
run_prepare "$addressed_log" "$addressed_git_log" --create >"$addressed_out" 2>"$addressed_err"

python3 - "$addressed_log" "$addressed_out" "$addressed_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if gh_lines != ["pr create --base main --head temp/prepare-pr-role-review-test --fill"]:
    raise SystemExit(f"expected only gh pr create after addressed findings, got: {gh_lines}")
if "- findings disposition: addressed" not in stdout:
    raise SystemExit("expected addressed disposition in output")
if stderr:
    raise SystemExit(f"did not expect stderr on addressed path: {stderr}")
PY

write_shadowed_role_review_packet "$SOURCE_HEAD"
commit_fixture_evidence

shadowed_json="$TMPDIR/shadowed.json"
run_prepare "$TMPDIR/gh-shadowed.log" "$TMPDIR/git-shadowed.log" --json >"$shadowed_json"

python3 - "$shadowed_json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
review = payload["pre_pr_local_role_review"]
if review["status"] != "passed":
    raise SystemExit(f"expected passed review status from latest packet, got: {review}")
if review["findings_disposition"] != "no_findings":
    raise SystemExit(f"expected latest packet disposition, got: {review}")
if review["residual_risk"] != "final fixture residual risk":
    raise SystemExit(f"expected latest packet residual risk, got: {review}")
PY

json_out="$TMPDIR/preflight.json"
run_prepare "$TMPDIR/gh-json.log" "$TMPDIR/git-json.log" --json >"$json_out"

python3 - "$json_out" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
review = payload["pre_pr_local_role_review"]
if review["status"] != "passed":
    raise SystemExit(f"expected passed review status, got: {review}")
if payload["review_request_command"] is not None:
    raise SystemExit(f"expected no reviewer request command, got: {payload['review_request_command']}")
if review["findings_disposition"] != "no_findings":
    raise SystemExit(f"expected no_findings disposition, got: {review}")
PY

echo "prepare-task-pr.test: OK"
