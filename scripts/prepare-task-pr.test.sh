#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REAL_GIT="$(command -v git)"

TMPDIR="$(mktemp -d)"
cleanup() {
  "$REAL_GIT" -C "$ROOT_DIR" worktree remove -f "${SMOKE_WORKTREE:-$TMPDIR/smoke-worktree}" >/dev/null 2>&1 || true
  if [[ -n "${SMOKE_WORKTREE_CANONICAL:-}" ]]; then
    "$REAL_GIT" -C "$ROOT_DIR" worktree remove -f "$SMOKE_WORKTREE_CANONICAL" >/dev/null 2>&1 || true
  fi
  "$REAL_GIT" -C "$ROOT_DIR" worktree prune >/dev/null 2>&1 || true
  "$REAL_GIT" -C "$ROOT_DIR" branch -D "${SMOKE_BRANCH:-temp/prepare-pr-role-review-test}" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

SMOKE_WORKTREE="$TMPDIR/smoke-worktree"
SMOKE_BRANCH="temp/prepare-pr-role-review-test-$$"
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
  mkdir -p "$SMOKE_WORKTREE/.pm/tasks"
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
doc_refs:
  - doc/engineering/project.md
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-03T00:00:00+08:00
last_started_at: 2026-06-03T00:00:00+08:00
last_claim_type: ready_for_pr
last_verified_at: 2026-06-03T00:02:00+08:00
last_verification_status: verified
last_closed_at: 2026-06-03T00:03:00+08:00
EOF
}

write_project_trace() {
  mkdir -p "$SMOKE_WORKTREE/doc/engineering"
  if [[ ! -f "$SMOKE_WORKTREE/doc/engineering/project.md" ]]; then
    printf '# Engineering Project Fixture\n' > "$SMOKE_WORKTREE/doc/engineering/project.md"
  fi
  cat >> "$SMOKE_WORKTREE/doc/engineering/project.md" <<EOF

- [x] PREPARE-TASK-PR-SMOKE [test_tier_required]: fixture task for prepare-task-pr workflow preflight. Trace: .pm/tasks/$TASK_UID.yaml
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
- claim-ready evidence: ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command fixture --task-uid $TASK_UID
- task-closeout evidence: ./scripts/pm/task-closeout.sh --role tpm --task-uid $TASK_UID --verify-command fixture
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SMOKE_WORKTREE_CANONICAL
- Source Branch: $SMOKE_BRANCH
- Source Head: $source_head
- Comparison Ref: refs/remotes/origin/main
- Reviewed Changed Paths: scripts/prepare-task-pr.sh
- Review Package: .pm/scratch/$TASK_UID/review-packages/review-fixture.diff
- Role Selection Basis: changed paths include PR helper workflow and project trace; roles producer_system_designer,repository_health_engineer,qa_engineer.
- Review Roles: producer_system_designer,repository_health_engineer,qa_engineer
- Review Evidence: producer_system_designer: 2026-06-03 00:00:00 CST; no_findings; fixture; repository_health_engineer: 2026-06-03 00:00:00 CST; no_findings; fixture; qa_engineer: 2026-06-03 00:00:00 CST; no_findings; fixture
- Review Verdicts: producer_system_designer scope/spec compliance=approved; role quality/risk=approved; repository_health_engineer scope/spec compliance=approved; role quality/risk=approved; qa_engineer scope/spec compliance=approved; role quality/risk=approved
- Review Findings Disposition: $disposition
- Finding Disposition Evidence: fixture evidence
- Verification Matrix: workflow helper -> prepare-task-pr smoke -> observed
- Visual Evidence: n/a; no visible surface
- WASM Evidence: n/a; no WASM surface
- Ops Evidence: n/a; no ops surface
- LiveOps Evidence: n/a; no liveops surface
- Residual Risk: fixture residual risk
- Slice Ledger: .pm/scratch/$TASK_UID/slice-ledger.jsonl
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
- claim-ready evidence: ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command fixture --task-uid $TASK_UID
- task-closeout evidence: ./scripts/pm/task-closeout.sh --role tpm --task-uid $TASK_UID --verify-command fixture
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
- claim-ready evidence: ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command fixture --task-uid $TASK_UID
- task-closeout evidence: ./scripts/pm/task-closeout.sh --role tpm --task-uid $TASK_UID --verify-command fixture
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SMOKE_WORKTREE_CANONICAL
- Source Branch: $SMOKE_BRANCH
- Source Head: $source_head
- Comparison Ref: refs/remotes/origin/main
- Reviewed Changed Paths: scripts/prepare-task-pr.sh
- Review Package: .pm/scratch/$TASK_UID/review-packages/review-fixture.diff
- Role Selection Basis: changed paths include PR helper workflow and project trace; roles producer_system_designer,repository_health_engineer,qa_engineer.
- Review Roles: producer_system_designer,repository_health_engineer,qa_engineer
- Review Evidence: producer_system_designer: 2026-06-03 00:01:00 CST; no_findings; fixture; repository_health_engineer: 2026-06-03 00:01:00 CST; no_findings; fixture; qa_engineer: 2026-06-03 00:01:00 CST; no_findings; fixture
- Review Verdicts: producer_system_designer scope/spec compliance=approved; role quality/risk=approved; repository_health_engineer scope/spec compliance=approved; role quality/risk=approved; qa_engineer scope/spec compliance=approved; role quality/risk=approved
- Review Findings Disposition: no_findings
- Finding Disposition Evidence: fixture evidence
- Verification Matrix: workflow helper -> prepare-task-pr smoke -> observed
- Visual Evidence: n/a; no visible surface
- WASM Evidence: n/a; no WASM surface
- Ops Evidence: n/a; no ops surface
- LiveOps Evidence: n/a; no liveops surface
- Residual Risk: final fixture residual risk
- Slice Ledger: .pm/scratch/$TASK_UID/slice-ledger.jsonl
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
- claim-ready evidence: ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command fixture --task-uid $TASK_UID
- task-closeout evidence: ./scripts/pm/task-closeout.sh --role tpm --task-uid $TASK_UID --verify-command fixture
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SMOKE_WORKTREE_CANONICAL-old
- Source Branch: $SMOKE_BRANCH-old
- Source Head: $source_head
- Comparison Ref: refs/remotes/origin/main-old
- Reviewed Changed Paths: scripts/prepare-task-pr.sh
- Review Package: .pm/scratch/$TASK_UID/review-packages/review-fixture.diff
- Role Selection Basis: changed paths include PR helper workflow and project trace; roles producer_system_designer,repository_health_engineer,qa_engineer.
- Review Roles: producer_system_designer,repository_health_engineer,qa_engineer
- Review Evidence: producer_system_designer: 2026-06-03 00:00:00 CST; no_findings; fixture; repository_health_engineer: 2026-06-03 00:00:00 CST; no_findings; fixture; qa_engineer: 2026-06-03 00:00:00 CST; no_findings; fixture
- Review Verdicts: producer_system_designer scope/spec compliance=approved; role quality/risk=approved; repository_health_engineer scope/spec compliance=approved; role quality/risk=approved; qa_engineer scope/spec compliance=approved; role quality/risk=approved
- Review Findings Disposition: no_findings
- Finding Disposition Evidence: fixture evidence
- Verification Matrix: workflow helper -> prepare-task-pr smoke -> observed
- Visual Evidence: n/a; no visible surface
- WASM Evidence: n/a; no WASM surface
- Ops Evidence: n/a; no ops surface
- LiveOps Evidence: n/a; no liveops surface
- Residual Risk: fixture residual risk
- Slice Ledger: .pm/scratch/$TASK_UID/slice-ledger.jsonl
- Blocker / Next Action: none.
EOF
}

commit_fixture_evidence() {
  "$REAL_GIT" -C "$SMOKE_WORKTREE" add ".pm/tasks/$TASK_UID.yaml" ".pm/tasks/$TASK_UID.execution.md" "doc/engineering/project.md"
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
    PM_ROOT_DIR="$SMOKE_WORKTREE_CANONICAL" \
    TEST_GH_LOG="$gh_log" \
    TEST_GIT_LOG="$git_log" \
    "$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" "$@"
}

reset_smoke_branch_to_base() {
  "$REAL_GIT" -C "$SMOKE_WORKTREE" reset --hard refs/remotes/origin/main >/dev/null
  "$REAL_GIT" -C "$SMOKE_WORKTREE" clean -fd >/dev/null
}

write_changed_path_fixture() {
  local changed_path="$1"
  printf '\n// prepare-task-pr local required command fixture\n' >> "$SMOKE_WORKTREE/$changed_path"
  "$REAL_GIT" -C "$SMOKE_WORKTREE" add "$changed_path"
  "$REAL_GIT" -C "$SMOKE_WORKTREE" \
    -c user.name="oasis7 smoke" \
    -c user.email="smoke@example.invalid" \
    -c commit.gpgsign=false \
    commit --no-verify -m "test: local required command fixture" >/dev/null
  SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
  write_task_binding
  write_project_trace
  write_role_review_packet "$SOURCE_HEAD" "no_findings"
  commit_fixture_evidence
}

helper_functions="$(
  sed -n '/^append_unique_token()/,/^local_role_review_status()/p' "$ROOT_DIR/scripts/prepare-task-pr.sh" | sed '$d'
)"
eval "$helper_functions"

assert_roles_for_path() {
  local path="$1"
  local expected_role="$2"
  local roles
  roles="$(required_review_roles_from_paths "$path")"
  if [[ ",$roles," != *",$expected_role,"* ]]; then
    echo "expected $path to require $expected_role, got $roles" >&2
    exit 1
  fi
  if [[ ",$roles," != *",qa_engineer,"* ]]; then
    echo "expected $path to require qa_engineer, got $roles" >&2
    exit 1
  fi
}

assert_roles_for_path "doc/engineering/workflow/source-of-truth.md" "producer_system_designer"
assert_roles_for_path ".github/workflows/rust.yml" "repository_health_engineer"
assert_roles_for_path "scripts/ci-tests.sh" "repository_health_engineer"
assert_roles_for_path "scripts/plan-rust-required-scope.sh" "repository_health_engineer"
assert_roles_for_path "doc/core/economy.md" "producer_system_designer"
assert_roles_for_path "doc/game/rules.md" "producer_system_designer"
assert_roles_for_path "doc/world-runtime/checkpoints.md" "runtime_engineer"
assert_roles_for_path "doc/world-simulator/economy.md" "gameplay_designer"
assert_roles_for_path "testing-manual.md" "game_visual_interaction_designer"
assert_roles_for_path "crates/oasis7/src/viewer/server.rs" "viewer_engineer"
assert_roles_for_path "scripts/run-viewer-web.sh" "viewer_engineer"
assert_roles_for_path "doc/world-simulator/viewer/readme.md" "viewer_engineer"
assert_roles_for_path "doc/viewer-manual.md" "viewer_engineer"
assert_roles_for_path "scripts/pm/workflow-behavior-eval.sh" "agent_engineer"
assert_roles_for_path "doc/readme/release-note.md" "liveops_community"
assert_roles_for_path "doc/health/node-readiness.md" "blockchain_ops_engineer"
assert_roles_for_path "crates/oasis7_wasm_abi/src/lib.rs" "wasm_platform_engineer"
assert_roles_for_path "crates/oasis7_builtin_wasm_modules/src/lib.rs" "wasm_platform_engineer"

visual_semantic_missing="$(
  semantic_review_evidence_missing "game_visual_interaction_designer" \
    "ui -> screenshot -> observed" \
    "n/a; no visible surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ "$visual_semantic_missing" != *"Visual Evidence must include screenshot/model-review evidence"* ]]; then
  echo "expected visual n/a to be rejected for visual role, got $visual_semantic_missing" >&2
  exit 1
fi

visual_exemption_missing="$(
  semantic_review_evidence_missing "game_visual_interaction_designer" \
    "ui docs -> explicit exemption -> observed" \
    "n/a with exemption reason: documentation-only accessibility wording; no rendered UI changed" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ -n "$visual_exemption_missing" ]]; then
  echo "expected explicit visual exemption to pass, got $visual_exemption_missing" >&2
  exit 1
fi

runtime_semantic_missing="$(
  semantic_review_evidence_missing "runtime_engineer" \
    "n/a; no runtime surface" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ "$runtime_semantic_missing" != *"runtime replay/recovery/checkpoint/long-run"* ]]; then
  echo "expected runtime matrix semantics to be enforced, got $runtime_semantic_missing" >&2
  exit 1
fi

runtime_exemption_missing="$(
  semantic_review_evidence_missing "runtime_engineer" \
    "n/a with deferral reason: docs-only workflow text; no runtime replay/recovery/checkpoint path changed" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ -n "$runtime_exemption_missing" ]]; then
  echo "expected explicit runtime deferral to pass, got $runtime_exemption_missing" >&2
  exit 1
fi

gameplay_semantic_missing="$(
  semantic_review_evidence_missing "gameplay_designer" \
    "n/a; no gameplay surface" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ "$gameplay_semantic_missing" != *"gameplay playability/economy/motivation-loop"* ]]; then
  echo "expected gameplay matrix semantics to be enforced, got $gameplay_semantic_missing" >&2
  exit 1
fi

ops_semantic_missing="$(
  semantic_review_evidence_missing "blockchain_ops_engineer" \
    "ops -> smoke -> observed" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ "$ops_semantic_missing" != *"Ops Evidence must include readiness/rollback/runbook/operator/health evidence"* ]]; then
  echo "expected generic ops n/a to be rejected, got $ops_semantic_missing" >&2
  exit 1
fi

liveops_semantic_missing="$(
  semantic_review_evidence_missing "liveops_community" \
    "liveops -> smoke -> observed" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a; no liveops surface"
)"
if [[ "$liveops_semantic_missing" != *"LiveOps Evidence must include messaging/release-note/player/community evidence"* ]]; then
  echo "expected generic liveops n/a to be rejected, got $liveops_semantic_missing" >&2
  exit 1
fi

ops_exemption_missing="$(
  semantic_review_evidence_missing "blockchain_ops_engineer" \
    "ops docs -> explicit exemption -> observed" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a with exemption reason: docs-only governance wording; no deployment change" \
    "n/a; no liveops surface"
)"
if [[ -n "$ops_exemption_missing" ]]; then
  echo "expected explicit ops exemption to pass, got $ops_exemption_missing" >&2
  exit 1
fi

liveops_exemption_missing="$(
  semantic_review_evidence_missing "liveops_community" \
    "liveops docs -> explicit exemption -> observed" \
    "n/a; no visual surface" \
    "n/a; no wasm surface" \
    "n/a; no ops surface" \
    "n/a with exemption reason: internal workflow wording; no public-facing change"
)"
if [[ -n "$liveops_exemption_missing" ]]; then
  echo "expected explicit liveops exemption to pass, got $liveops_exemption_missing" >&2
  exit 1
fi

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
write_project_trace
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
    raise SystemExit(f"expected stale Source Head marker in error, got: {stderr}\ngit log: {git_lines}")
PY

SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
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

SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
write_role_review_packet "$SOURCE_HEAD" "no_findings"
commit_fixture_evidence

success_log="$TMPDIR/gh-success.log"
success_git_log="$TMPDIR/git-success.log"
success_out="$TMPDIR/success.out"
success_err="$TMPDIR/success.err"
if ! run_prepare "$success_log" "$success_git_log" --create >"$success_out" 2>"$success_err"; then
  cat "$success_err" >&2
  exit 1
fi

python3 - "$success_log" "$success_git_log" "$success_out" "$success_err" "$SMOKE_BRANCH" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")
branch = sys.argv[5]

if gh_lines != [f"pr create --base main --head {branch} --fill"]:
    raise SystemExit(f"expected only gh pr create and no reviewer API calls, got: {gh_lines}")
if not any(
    line.endswith(f"push -u origin {branch}")
    or line.endswith(f"push origin {branch}")
    for line in git_lines
):
    raise SystemExit(f"expected push attempt after valid review packet, got: {git_lines}")
if "Created PR:" not in stdout or "https://github.com/example/oasis7/pull/999" not in stdout:
    raise SystemExit("expected created PR output")
if "Pre-PR Local Role Review:" not in stdout or "- status: passed" not in stdout:
    raise SystemExit("expected local role review status in output")
if "- review package: .pm/scratch/" not in stdout:
    raise SystemExit("expected review package path in local role review output")
if "- review verdicts: producer_system_designer scope/spec compliance=approved; role quality/risk=approved; repository_health_engineer scope/spec compliance=approved; role quality/risk=approved; qa_engineer scope/spec compliance=approved; role quality/risk=approved" not in stdout:
    raise SystemExit("expected multi-role dual review verdicts in local role review output")
if "- slice ledger: .pm/scratch/" not in stdout:
    raise SystemExit("expected slice ledger path in local role review output")
if stderr:
    raise SystemExit(f"did not expect stderr on success path: {stderr}")
PY

behind_log="$TMPDIR/gh-behind.log"
behind_git_log="$TMPDIR/git-behind.log"
behind_out="$TMPDIR/behind.out"
behind_err="$TMPDIR/behind.err"
TEST_REV_LIST_COUNTS="1 2" run_prepare "$behind_log" "$behind_git_log" --create >"$behind_out" 2>"$behind_err"

python3 - "$behind_log" "$behind_git_log" "$behind_out" "$behind_err" "$SMOKE_BRANCH" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")
branch = sys.argv[5]

if gh_lines != [f"pr create --base main --head {branch} --fill"]:
    raise SystemExit(f"expected gh pr create on behind-but-allowed path, got: {gh_lines}")
if not any(
    line.endswith(f"push -u origin {branch}")
    or line.endswith(f"push origin {branch}")
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

SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
write_role_review_packet "$SOURCE_HEAD" "addressed"
commit_fixture_evidence

addressed_log="$TMPDIR/gh-addressed.log"
addressed_git_log="$TMPDIR/git-addressed.log"
addressed_out="$TMPDIR/addressed.out"
addressed_err="$TMPDIR/addressed.err"
run_prepare "$addressed_log" "$addressed_git_log" --create >"$addressed_out" 2>"$addressed_err"

python3 - "$addressed_log" "$addressed_out" "$addressed_err" "$SMOKE_BRANCH" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")
branch = sys.argv[4]

if gh_lines != [f"pr create --base main --head {branch} --fill"]:
    raise SystemExit(f"expected only gh pr create after addressed findings, got: {gh_lines}")
if "- findings disposition: addressed" not in stdout:
    raise SystemExit("expected addressed disposition in output")
if stderr:
    raise SystemExit(f"did not expect stderr on addressed path: {stderr}")
PY

SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
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
json_err="$TMPDIR/preflight.err"
run_prepare "$TMPDIR/gh-json.log" "$TMPDIR/git-json.log" --json >"$json_out" 2>"$json_err"

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

if [[ -s "$json_err" ]]; then
  cat "$json_err" >&2
  exit 1
fi

post_review_views_log="$TMPDIR/post-review-views.log"
post_review_views_git_log="$TMPDIR/post-review-views.git.log"
post_review_views_json="$TMPDIR/post-review-views.json"
write_role_review_packet "$SOURCE_HEAD" "no_findings"
commit_fixture_evidence
mkdir -p "$SMOKE_WORKTREE/.pm/registry" "$SMOKE_WORKTREE/.pm/roles/tpm/backlog"
printf 'version: 2\nidentity_key: task_uid\ngenerated_from: .pm/tasks/*.yaml\ntasks: []\n' > "$SMOKE_WORKTREE/.pm/registry/tasks.yaml"
printf 'version: 1\nrole: tpm\nstatus: done\ntasks: []\n' > "$SMOKE_WORKTREE/.pm/roles/tpm/backlog/done.yaml"
"$REAL_GIT" -C "$SMOKE_WORKTREE" add -f ".pm/registry/tasks.yaml" ".pm/roles/tpm/backlog/done.yaml"
"$REAL_GIT" -C "$SMOKE_WORKTREE" \
  -c user.name="oasis7 smoke" \
  -c user.email="smoke@example.invalid" \
  -c commit.gpgsign=false \
  commit --no-verify -m "test: post-review generated pm views" >/dev/null
run_prepare "$post_review_views_log" "$post_review_views_git_log" --json >"$post_review_views_json"

python3 - "$post_review_views_json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
review = payload["pre_pr_local_role_review"]
if review["status"] != "passed":
    raise SystemExit(f"expected generated PM views to be allowed after review, got: {review}")
PY

reset_smoke_branch_to_base
write_changed_path_fixture "crates/oasis7_node/src/network_bridge.rs"
node_required_json="$TMPDIR/node-required.json"
run_prepare "$TMPDIR/gh-node-required.log" "$TMPDIR/git-node-required.log" --json >"$node_required_json"

python3 - "$node_required_json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
required = payload["local_required_validation"]
command = required["recommended_required_command"] or ""
reason = required["reason_summary"] or ""
expected_present = [
    "OASIS7_CI_RUN_OASIS7_NODE_TESTS=true",
    "OASIS7_CI_RUN_OASIS7_NET_TESTS=false",
    "OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=false",
]
missing = [item for item in expected_present if item not in command]
if missing:
    raise SystemExit(f"node required command missing {missing}: {command}")
if required["scope"] != "targeted":
    raise SystemExit(f"expected targeted node scope, got: {required}")
if "node:crates/oasis7_node/src/network_bridge.rs" not in reason:
    raise SystemExit(f"expected node reason, got: {reason}")
PY

reset_smoke_branch_to_base
write_changed_path_fixture "crates/oasis7_net/src/lib.rs"
net_required_json="$TMPDIR/net-required.json"
run_prepare "$TMPDIR/gh-net-required.log" "$TMPDIR/git-net-required.log" --json >"$net_required_json"

python3 - "$net_required_json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
required = payload["local_required_validation"]
command = required["recommended_required_command"] or ""
reason = required["reason_summary"] or ""
expected_present = [
    "OASIS7_CI_RUN_OASIS7_NODE_TESTS=false",
    "OASIS7_CI_RUN_OASIS7_NET_TESTS=true",
    "OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=true",
]
missing = [item for item in expected_present if item not in command]
if missing:
    raise SystemExit(f"net required command missing {missing}: {command}")
if required["scope"] != "targeted":
    raise SystemExit(f"expected targeted net scope, got: {required}")
if "net:crates/oasis7_net/src/lib.rs" not in reason:
    raise SystemExit(f"expected net reason, got: {reason}")
PY

echo "prepare-task-pr.test: OK"
