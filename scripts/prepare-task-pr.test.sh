#!/usr/bin/env bash
set -euo pipefail
export OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS=1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SOURCE_ROOT="$ROOT_DIR"
REAL_GIT="$(command -v git)"

TMPDIR="$(mktemp -d)"
FIXTURE_ROOT="$TMPDIR/repo"
mkdir -p "$FIXTURE_ROOT"
(cd "$SOURCE_ROOT" && git ls-files -co --exclude-standard -z | tar --null -T - -cf -) | tar -xf - -C "$FIXTURE_ROOT"
"$REAL_GIT" -C "$FIXTURE_ROOT" init -q -b main
"$REAL_GIT" -C "$FIXTURE_ROOT" config user.email test@example.com
"$REAL_GIT" -C "$FIXTURE_ROOT" config user.name Test
"$REAL_GIT" -C "$FIXTURE_ROOT" add .
"$REAL_GIT" -C "$FIXTURE_ROOT" commit -qm "fixture snapshot"
"$REAL_GIT" -C "$FIXTURE_ROOT" update-ref refs/remotes/origin/main HEAD
ROOT_DIR="$FIXTURE_ROOT"
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

SMOKE_WORKTREE_CANONICAL="$(
  python3 - "$SMOKE_WORKTREE" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).resolve())
PY
)"
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

if [[ "${1:-}" == "pr" && "${2:-}" == "list" ]]; then
  printf '%s\n' "${TEST_EXISTING_PR_JSON:-[]}"
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  printf '%b\n' "${TEST_PR_STATE_TSV:-true\tOPEN\t}"
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "ready" ]]; then
  printf 'ready\n'
  exit 0
fi

if [[ "${1:-}" == "repo" && "${2:-}" == "view" ]]; then
  printf 'example/oasis7\n'
  exit 0
fi

if [[ "${1:-}" == "issue" && "${2:-}" == "list" ]]; then
  cat "${TEST_GH_ISSUE_LIST_JSON:?}"
  exit 0
fi

if [[ "${1:-}" == "issue" && "${2:-}" == "view" && "$*" == *"--json body,comments,number,title,url"* ]]; then
  cat "${TEST_GH_ISSUE_FULL_JSON:-${TEST_GH_ISSUE_BODY_JSON:?}}"
  exit 0
fi

if [[ "${1:-}" == "issue" && "${2:-}" == "view" && "$*" == *"--json body,number,title,url"* ]]; then
  cat "${TEST_GH_ISSUE_BODY_JSON:?}"
  exit 0
fi

if [[ "${1:-}" == "issue" && "${2:-}" == "view" ]]; then
  cat "${TEST_GH_ISSUE_VIEW_JSON:?}"
  exit 0
fi

if [[ "${1:-}" == "issue" && "${2:-}" == "edit" ]]; then
  printf 'edited\n'
  exit 0
fi

if [[ "${1:-}" == "issue" && "${2:-}" == "comment" ]]; then
  printf 'https://github.com/example/oasis7/issues/%s#issuecomment-fixture\n' "${3:-0}"
  exit 0
fi

if [[ "$*" == "project view 1 --owner eng-cc --format json" ]]; then
  printf '{"id":"PROJECT_ID","number":1,"title":"fixture","url":"https://github.com/users/eng-cc/projects/1"}\n'
  exit 0
fi

if [[ "$*" == "project field-list 1 --owner eng-cc --format json" ]]; then
  cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_PR_WATCH","name":"PR Watch"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_PR_WATCH_PM","name":"pr_watch"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P3","name":"P3"}]},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
  exit 0
fi

if [[ "${1:-}" == "project" && "${2:-}" == "item-edit" ]]; then
  printf '{}\n'
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

- [x] prepare-task-pr-smoke (PRD-ENGINEERING-999) [test_tier_required]: fixture task for prepare-task-pr workflow preflight. Trace: #123 ($TASK_UID)
EOF
}

write_slice_ledger() {
  local source_head="$1"
  local ledger_dir="$SMOKE_WORKTREE/.pm/scratch/$TASK_UID"
  local ledger="$ledger_dir/slice-ledger.jsonl"
  mkdir -p "$ledger_dir"
  : >"$ledger"
  local index=0
  for role in producer_system_designer repository_health_engineer qa_engineer; do
    index=$((index + 1))
    local artifact="$ledger_dir/${role}-return.md"
    local receipt="$ledger_dir/${role}-dispatch.json"
    local dispatch_id="11111111-1111-4111-8111-$(printf '%012d' "$index")"
    printf '%s review return\n' "$role" >"$artifact"
    local digest
    digest="$(shasum -a 256 "$artifact" | awk '{print $1}')"
    printf '{"receipt_type":"oasis7_subagent_dispatch","issuer":"codex_runtime","dispatch_id":"%s","role":"%s","source_head":"%s","contract_digest":"%064d"}\n' \
      "$dispatch_id" "$role" "$source_head" 0 >"$receipt"
    printf '{"task_uid":"%s","role":"%s","status":"completed","head":"%s","slice_id":"%s","dispatch_receipt":".pm/scratch/%s/%s-dispatch.json","activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified: fixture","artifact_digest":"%s","scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"fixture risk","artifacts":[".pm/scratch/%s/%s-return.md"]}\n' \
      "$TASK_UID" "$role" "$source_head" "$dispatch_id" "$TASK_UID" "$role" "$digest" "$TASK_UID" "$role" >>"$ledger"
  done
}

write_role_review_packet() {
  local source_head="$1"
  local disposition="$2"
  mkdir -p "$SMOKE_WORKTREE/.pm/tasks"
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
  write_slice_ledger "$source_head"
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
  write_slice_ledger "$source_head"
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
  write_slice_ledger "$source_head"
}

commit_fixture_evidence() {
  local add_paths=(".pm/tasks/$TASK_UID.yaml" ".pm/tasks/$TASK_UID.execution.md" "doc/engineering/project.md")
  "$REAL_GIT" -C "$SMOKE_WORKTREE" add "${add_paths[@]}"
  if [[ -f "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json" ]]; then
    "$REAL_GIT" -C "$SMOKE_WORKTREE" add -f ".pm/github-project-sync/tasks.json"
  fi
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
    PREPARE_TASK_PR_ALLOW_RETIRED_PM_TASKS="${PREPARE_TASK_PR_ALLOW_RETIRED_PM_TASKS:-1}" \
    PREPARE_TASK_PR_ALLOW_GITHUB_ISSUE_FALLBACK="${PREPARE_TASK_PR_ALLOW_GITHUB_ISSUE_FALLBACK:-0}" \
    PREPARE_TASK_PR_WORKFLOW_LINT_PATH="$ROOT_DIR/scripts/pm/workflow-lint.sh" \
    TEST_GH_LOG="$gh_log" \
    TEST_GIT_LOG="$git_log" \
    TEST_GH_ISSUE_LIST_JSON="${TEST_GH_ISSUE_LIST_JSON:-}" \
    TEST_GH_ISSUE_BODY_JSON="${TEST_GH_ISSUE_BODY_JSON:-}" \
    TEST_GH_ISSUE_FULL_JSON="${TEST_GH_ISSUE_FULL_JSON:-}" \
    TEST_GH_ISSUE_VIEW_JSON="${TEST_GH_ISSUE_VIEW_JSON:-}" \
    "$ROOT_DIR/scripts/prepare-task-pr.sh" "$SMOKE_BRANCH" "$@"
}

reset_smoke_branch_to_base() {
  "$REAL_GIT" -C "$SMOKE_WORKTREE" reset --hard refs/remotes/origin/main >/dev/null
  "$REAL_GIT" -C "$SMOKE_WORKTREE" clean -fd >/dev/null
}

reset_project_mapping_after_record_pr() {
  if "$REAL_GIT" -C "$SMOKE_WORKTREE" ls-files --error-unmatch .pm/github-project-sync/tasks.json >/dev/null 2>&1; then
    "$REAL_GIT" -C "$SMOKE_WORKTREE" checkout -- .pm/github-project-sync/tasks.json
  else
    rm -f "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json"
  fi
}

write_changed_path_fixture() {
  local changed_path="$1"
  mkdir -p "$SMOKE_WORKTREE/$(dirname "$changed_path")"
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
  local expected_qa="${3:-no}"
  local roles
  roles="$(required_review_roles_from_paths "$path")"
  if [[ ",$roles," != *",$expected_role,"* ]]; then
    echo "expected $path to require $expected_role, got $roles" >&2
    exit 1
  fi
  if [[ "$expected_qa" == "yes" && ",$roles," != *",qa_engineer,"* ]]; then
    echo "expected $path to require qa_engineer, got $roles" >&2
    exit 1
  fi
}

assert_roles_for_path "doc/engineering/workflow/source-of-truth.md" "producer_system_designer" "yes"
assert_roles_for_path ".github/workflows/rust.yml" "repository_health_engineer" "yes"
assert_roles_for_path "scripts/ci-tests.sh" "repository_health_engineer" "yes"
assert_roles_for_path "scripts/plan-rust-required-scope.sh" "repository_health_engineer" "yes"
assert_roles_for_path "skills/prd/SKILL.md" "repository_health_engineer"
assert_roles_for_path "skills/prd/SKILL.md" "producer_system_designer"
assert_roles_for_path "doc/core/economy.md" "producer_system_designer"
assert_roles_for_path "doc/game/rules.md" "producer_system_designer"
assert_roles_for_path "doc/world-runtime/checkpoints.md" "runtime_engineer"
assert_roles_for_path "doc/world-simulator/economy.md" "gameplay_designer"
assert_roles_for_path "testing-manual.md" "game_visual_interaction_designer"
assert_roles_for_path "crates/oasis7/src/viewer/server.rs" "viewer_engineer"
assert_roles_for_path "scripts/run-viewer-web.sh" "viewer_engineer"
assert_roles_for_path "doc/world-simulator/viewer/readme.md" "viewer_engineer"
assert_roles_for_path "doc/world-simulator/viewer/viewer-manual.manual.md" "viewer_engineer"
assert_roles_for_path "scripts/pm/workflow-behavior-eval.sh" "repository_health_engineer" "yes"
assert_roles_for_path ".codex/config.toml" "repository_health_engineer" "yes"
assert_roles_for_path "AGENTS.md" "repository_health_engineer" "yes"
assert_roles_for_path ".agents/roles/tpm.md" "repository_health_engineer" "yes"
assert_roles_for_path ".agents/roles/templates/subagent-slice-card.md" "repository_health_engineer" "yes"
assert_roles_for_path ".agents/skills/repo-owned-workflow-router/SKILL.md" "repository_health_engineer" "yes"
assert_roles_for_path ".agents/skills/requesting-repo-owned-review/SKILL.md" "repository_health_engineer" "yes"
assert_roles_for_path "scripts/pm/validate-codex-agent-config.py" "repository_health_engineer" "yes"
assert_roles_for_path "crates/oasis7_agent/src/planner.rs" "agent_engineer"

role_card_roles="$(required_review_roles_from_paths ".agents/roles/agent_engineer.md")"
for required_role in repository_health_engineer agent_engineer; do
  if [[ ",$role_card_roles," != *",$required_role,"* ]]; then
    echo "expected agent_engineer role card to require $required_role, got $role_card_roles" >&2
    exit 1
  fi
done

registry_roles="$(required_review_roles_from_paths ".codex/config.toml")"
for required_role in \
  producer_system_designer gameplay_designer game_visual_interaction_designer \
  runtime_engineer blockchain_ops_engineer wasm_platform_engineer agent_engineer \
  viewer_engineer qa_engineer repository_health_engineer liveops_community; do
  if [[ ",$registry_roles," != *",$required_role,"* ]]; then
    echo "expected Codex registry descriptions to require $required_role, got $registry_roles" >&2
    exit 1
  fi
done

for adapter_role in \
  producer_system_designer gameplay_designer game_visual_interaction_designer \
  runtime_engineer blockchain_ops_engineer wasm_platform_engineer agent_engineer \
  viewer_engineer qa_engineer repository_health_engineer liveops_community; do
  adapter_path=".codex/agents/${adapter_role}.toml"
  adapter_roles="$(required_review_roles_from_paths "$adapter_path")"
  for required_role in repository_health_engineer qa_engineer "$adapter_role"; do
    if [[ ",$adapter_roles," != *",$required_role,"* ]]; then
      echo "expected $adapter_path to require $required_role, got $adapter_roles" >&2
      exit 1
    fi
  done
done

if required_review_roles_from_paths ".codex/agents/unknown_specialist.toml" \
  >"$TMPDIR/unknown-adapter.out" 2>"$TMPDIR/unknown-adapter.err"; then
  echo "expected unknown Codex adapter basename rejection" >&2
  exit 1
fi
grep -F "unknown Codex specialist adapter basename" \
  "$TMPDIR/unknown-adapter.err" >/dev/null
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

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

unexpected_gh = [line for line in gh_lines if not line.startswith("issue list ")]
if unexpected_gh:
    raise SystemExit(f"expected only read-only issue lookup before missing-review failure, got: {gh_lines}")
if any("push" in line for line in git_lines):
    raise SystemExit(f"expected no push before missing-review failure, got: {git_lines}")
if "missing passed pre-PR local role review evidence" not in stderr:
    raise SystemExit(f"expected missing-review error, got: {stderr}")
PY

# A fresh task has no role-review packet or provenance ledger yet. The draft
# candidate exists specifically to obtain same-head CI before those gates.
reset_smoke_branch_to_base
write_task_binding
write_project_trace
mkdir -p "$SMOKE_WORKTREE/.pm/github-project-sync"
cat > "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json" <<EOF
{"project":{"repo":"example/oasis7"},"tasks":{"$TASK_UID":{"issue_number":123,"issue_url":"https://github.com/example/oasis7/issues/123","owner_role":"tpm","priority":"P3","project_item_id":"PVTI_fixture","status":"committed","workflow_phase":"implementation","task_uid":"$TASK_UID","title":"fresh draft candidate fixture","worktree_hint":"$SMOKE_WORKTREE_CANONICAL"}},"version":1}
EOF
"$REAL_GIT" -C "$SMOKE_WORKTREE" add ".pm/tasks/$TASK_UID.yaml" "doc/engineering/project.md"
"$REAL_GIT" -C "$SMOKE_WORKTREE" add -f ".pm/github-project-sync/tasks.json"
"$REAL_GIT" -C "$SMOKE_WORKTREE" \
  -c user.name="oasis7 smoke" \
  -c user.email="smoke@example.invalid" \
  -c commit.gpgsign=false \
  commit --no-verify -m "test: fresh draft candidate fixture" >/dev/null

draft_log="$TMPDIR/gh-draft-candidate.log"
draft_git_log="$TMPDIR/git-draft-candidate.log"
draft_out="$TMPDIR/draft-candidate.out"
draft_err="$TMPDIR/draft-candidate.err"
draft_issue_body="$TMPDIR/draft-issue-body.json"
printf '{"body":"Task UID: %s\n","number":123,"title":"fixture","url":"https://github.com/example/oasis7/issues/123"}\n' "$TASK_UID" >"$draft_issue_body"
if ! TEST_GH_ISSUE_BODY_JSON="$draft_issue_body" TEST_GH_ISSUE_VIEW_JSON="$draft_issue_body" \
  run_prepare "$draft_log" "$draft_git_log" --draft-candidate >"$draft_out" 2>"$draft_err"; then
  cat "$draft_err" >&2
  exit 1
fi
python3 - "$draft_log" "$draft_out" "$draft_err" "$SMOKE_BRANCH" <<'PY'
import sys
from pathlib import Path
gh=Path(sys.argv[1]).read_text(encoding="utf-8")
out=Path(sys.argv[2]).read_text(encoding="utf-8")
err=Path(sys.argv[3]).read_text(encoding="utf-8")
branch=sys.argv[4]
if f"pr create --base main --head {branch} --fill" not in gh or "--draft" not in gh:
    raise SystemExit(f"fresh task did not reach draft PR creation: {gh}")
if "Created PR:" not in out:
    raise SystemExit(f"fresh draft candidate was not recorded: {out}")
if "pre-PR local role-return validation failed" in err or "machine-checkable role-return ledger" in err:
    raise SystemExit(f"draft candidate incorrectly required review provenance: {err}")
project_writes=[line for line in gh.splitlines() if line.startswith("project item-edit ")]
if len(project_writes)!=1 or "--field-id FIELD_PR" not in project_writes[0] or "--text https://github.com/example/oasis7/pull/999" not in project_writes[0]:
    raise SystemExit(f"draft candidate must update exactly the Project PR field: {project_writes}")
for forbidden in ("FIELD_STATUS","FIELD_PM_STATUS","FIELD_WORKFLOW_PHASE"):
    if any(forbidden in line for line in project_writes):
        raise SystemExit(f"draft candidate advanced lifecycle field {forbidden}: {project_writes}")
PY

GITHUB_FALLBACK_ROOT="$TMPDIR/github-fallback-root"
GITHUB_FALLBACK_WORKTREE="$(
  python3 - "$GITHUB_FALLBACK_ROOT" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).resolve())
PY
)"
GITHUB_FALLBACK_HEAD="1111111111111111111111111111111111111111"
mkdir -p "$GITHUB_FALLBACK_ROOT/.pm/tasks" "$GITHUB_FALLBACK_ROOT/.pm/github-project-sync"
touch "$GITHUB_FALLBACK_ROOT/.pm/tasks/.gitkeep"
cat > "$GITHUB_FALLBACK_ROOT/.pm/github-project-sync/tasks.json" <<EOF
{
  "project": {
    "repo": "example/oasis7"
  },
  "tasks": {
    "$TASK_UID": {
      "execution_log_path": "https://github.com/example/oasis7/issues/123",
      "issue_number": 123,
      "issue_url": "https://github.com/example/oasis7/issues/123",
      "status": "committed",
      "task_uid": "$TASK_UID",
      "worktree_hint": "$GITHUB_FALLBACK_WORKTREE"
    }
  },
  "version": 1
}
EOF
GITHUB_ISSUE_VIEW_JSON="$TMPDIR/github-issue-view.json"
cat > "$GITHUB_ISSUE_VIEW_JSON" <<EOF
{
  "comments": [
    {
      "body": "## 2026-06-03 00:00:00 CST / tpm\n- Pre-PR Local Role Review: passed\n- Task UID: $TASK_UID\n- Source Worktree: github-fallback-root\n- Source Branch: $SMOKE_BRANCH\n- Source Head: $GITHUB_FALLBACK_HEAD\n- Comparison Ref: refs/remotes/origin/main\n- Reviewed Changed Paths: doc/engineering/project.md\n- Review Package: n/a; GitHub-backed smoke fixture\n- Role Selection Basis: GitHub-backed PM issue comment path smoke; roles producer_system_designer,repository_health_engineer,qa_engineer.\n- Review Roles: producer_system_designer,repository_health_engineer,qa_engineer\n- Review Evidence: producer_system_designer: no_findings; repository_health_engineer: no_findings; qa_engineer: no_findings\n- Review Verdicts: producer_system_designer scope/spec compliance=approved; role quality/risk=approved; repository_health_engineer scope/spec compliance=approved; role quality/risk=approved; qa_engineer scope/spec compliance=approved; role quality/risk=approved\n- Review Findings Disposition: no_findings\n- Finding Disposition Evidence: GitHub issue comment smoke evidence\n- Verification Matrix: GitHub-backed preflight -> prepare-task-pr --json -> observed\n- Visual Evidence: n/a; no visible surface\n- WASM Evidence: n/a; no WASM surface\n- Ops Evidence: n/a; no ops surface\n- LiveOps Evidence: n/a; no liveops surface\n- Residual Risk: fixture residual risk\n- Slice Ledger: n/a; smoke fixture\n"
    }
  ]
}
EOF
local_role_review_function="$(sed -n '/^local_role_review_status()/,/^ensure_branch_exists /p' "$ROOT_DIR/scripts/prepare-task-pr.sh" | sed '$d')"
eval "$local_role_review_function"
github_mapping_review="$TMPDIR/github-mapping-review.env"
: > "$TMPDIR/gh-github-mapping.log"
PATH="$TMPDIR/bin:$PATH" TEST_GH_LOG="$TMPDIR/gh-github-mapping.log" TEST_GH_ISSUE_VIEW_JSON="$GITHUB_ISSUE_VIEW_JSON" \
  local_role_review_status "$GITHUB_FALLBACK_WORKTREE" "$SMOKE_BRANCH" "$GITHUB_FALLBACK_HEAD" refs/remotes/origin/main > "$github_mapping_review"

python3 - "$github_mapping_review" "$TMPDIR/gh-github-mapping.log" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

fields = {}
for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
    key, _, value = line.partition("=")
    fields[key] = value
gh_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
if fields.get("status") != "passed":
    raise SystemExit(f"expected GitHub issue comment review packet to pass, got: {fields}")
if fields.get("task_uid") != "task_11111111111111111111111111111111":
    raise SystemExit(f"expected mapped task uid, got: {fields}")
if not any(line.startswith("issue view 123 -R example/oasis7 --json comments") for line in gh_lines):
    raise SystemExit(f"expected gh issue view call for GitHub-backed evidence, got: {gh_lines}")
PY

reset_smoke_branch_to_base
write_task_binding
write_project_trace
write_role_review_packet "0000000000000000000000000000000000000000" "no_findings"
commit_fixture_evidence

retired_err="$TMPDIR/retired.err"
PREPARE_TASK_PR_ALLOW_RETIRED_PM_TASKS=0 run_prepare "$TMPDIR/gh-retired.log" "$TMPDIR/git-retired.log" --json >"$TMPDIR/retired.json" 2>"$retired_err"
python3 - "$TMPDIR/retired.json" "$retired_err" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
stderr = Path(sys.argv[2]).read_text(encoding="utf-8")
review = payload["pre_pr_local_role_review"]
if stderr:
    raise SystemExit(f"expected no stderr for retired-task json report, got: {stderr}")
if review["status"] != "missing":
    raise SystemExit(f"expected retired .pm/tasks default to report missing, got: {review}")
if "retired .pm/tasks files are present" not in (review.get("reason") or ""):
    raise SystemExit(f"expected retired .pm/tasks rejection reason, got: {review}")
PY

reset_smoke_branch_to_base
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

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

unexpected_gh = [line for line in gh_lines if not line.startswith("issue list ")]
if unexpected_gh:
    raise SystemExit(f"expected only read-only issue lookup before stale-review failure, got: {gh_lines}")
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
    "Source Worktree: " + Path(expected_worktree).name + " or repo-relative worktree hint",
    f"Source Branch: {expected_branch}",
    "Comparison Ref: refs/remotes/origin/main",
}
if review["status"] != "missing":
    raise SystemExit(f"expected missing review status for prefix-mismatched fields, got: {review}")
if not expected.issubset(missing):
    raise SystemExit(f"expected exact field mismatch markers {expected}, got: {missing}")
PY

reset_smoke_branch_to_base
rm -f "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json"
if "$REAL_GIT" -C "$SMOKE_WORKTREE" ls-files --error-unmatch .pm/github-project-sync/tasks.json >/dev/null 2>&1; then
  "$REAL_GIT" -C "$SMOKE_WORKTREE" add -u .pm/github-project-sync/tasks.json
fi
printf '\n# no-cache GitHub-backed PM fixture\n' >> "$SMOKE_WORKTREE/scripts/prepare-task-pr.sh"
"$REAL_GIT" -C "$SMOKE_WORKTREE" add scripts/prepare-task-pr.sh
"$REAL_GIT" -C "$SMOKE_WORKTREE" \
  -c user.name="oasis7 smoke" \
  -c user.email="smoke@example.invalid" \
  -c commit.gpgsign=false \
  commit --allow-empty --no-verify -m "test: no-cache GitHub-backed PM fixture" >/dev/null
SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
write_slice_ledger "$SOURCE_HEAD"
NO_CACHE_ISSUE_LIST="$TMPDIR/no-cache-issue-list.json"
NO_CACHE_ISSUE_BODY="$TMPDIR/no-cache-issue-body.json"
NO_CACHE_ISSUE_FULL="$TMPDIR/no-cache-issue-full.json"
NO_CACHE_ISSUE_COMMENTS="$TMPDIR/no-cache-issue-comments.json"
cat > "$NO_CACHE_ISSUE_LIST" <<'EOF'
[{"number":123,"state":"OPEN","title":"GitHub-backed no-cache fixture","url":"https://github.com/example/oasis7/issues/123"}]
EOF
cat > "$NO_CACHE_ISSUE_BODY" <<EOF
{
  "body": "<!-- oasis7-pm-task -->\\ntask_uid: $TASK_UID\\n\\nGitHub-backed oasis7 PM task.\\n\\nTask metadata:\\n- owner_role: \`tpm\`\\n- module: \`engineering\`\\n- status: \`ready\`\\n- priority: \`P3\`\\n- worktree_hint: \`smoke-worktree\`\\n\\nSource refs:\\n- \`doc/engineering/project.md\`\\n\\nAcceptance:\\n- no-cache prepare-task-pr fixture\\n",
  "number": 123,
  "title": "GitHub-backed no-cache fixture",
  "url": "https://github.com/example/oasis7/issues/123"
}
EOF
cat > "$NO_CACHE_ISSUE_COMMENTS" <<EOF
{
  "comments": [
    {
      "body": "<!-- oasis7-pm-claim-verification -->\\nTask UID: $TASK_UID\\nClaim Type: ready_for_pr\\nVerification Status: verified"
    },
    {
      "body": "<!-- oasis7-pm-evidence -->\\nTask UID: $TASK_UID\\nEvidence Phase: pre_pr_ready\\nRole: tpm"
    },
    {
      "body": "## 2026-06-03 00:00:00 CST / tpm\\n- Pre-PR Local Role Review: passed\\n- Task UID: $TASK_UID\\n- Source Worktree: smoke-worktree\\n- Source Branch: $SMOKE_BRANCH\\n- Source Head: $SOURCE_HEAD\\n- Comparison Ref: refs/remotes/origin/main\\n- Reviewed Changed Paths: scripts/prepare-task-pr.sh\\n- Review Package: n/a; no-cache GitHub issue fixture\\n- Role Selection Basis: changed paths include PR helper workflow and GitHub issue fallback; roles repository_health_engineer,qa_engineer.\\n- Review Roles: repository_health_engineer,qa_engineer\\n- Review Evidence: repository_health_engineer: no_findings; qa_engineer: no_findings\\n- Review Verdicts: repository_health_engineer scope/spec compliance=approved; role quality/risk=approved; qa_engineer scope/spec compliance=approved; role quality/risk=approved\\n- Review Findings Disposition: no_findings\\n- Finding Disposition Evidence: no-cache fixture evidence\\n- Verification Matrix: no-cache prepare-task-pr --create -> fake gh issue search/view -> observed\\n- Visual Evidence: n/a with exemption reason: workflow helper only; no visible surface\\n- WASM Evidence: n/a; no WASM surface\\n- Ops Evidence: n/a with exemption reason: local PR helper only; no deployment change\\n- LiveOps Evidence: n/a with exemption reason: internal workflow helper only; no public-facing change\\n- Residual Risk: fixture residual risk\\n- Slice Ledger: .pm/scratch/$TASK_UID/slice-ledger.jsonl\\n"
    }
  ]
}
EOF
cat > "$NO_CACHE_ISSUE_FULL" <<EOF
{
  "body": "<!-- oasis7-pm-task -->\\ntask_uid: $TASK_UID\\n\\nGitHub-backed oasis7 PM task.\\n\\nTask metadata:\\n- owner_role: \`tpm\`\\n- module: \`engineering\`\\n- status: \`ready\`\\n- priority: \`P3\`\\n- worktree_hint: \`smoke-worktree\`\\n\\nSource refs:\\n- \`doc/engineering/project.md\`\\n\\nAcceptance:\\n- no-cache prepare-task-pr fixture\\n",
  "comments": [
    {
      "url": "https://github.com/example/oasis7/issues/123#issuecomment-1",
      "body": "<!-- oasis7-pm-claim-verification -->\\nTask UID: $TASK_UID\\nClaim Type: ready_for_pr\\nVerification Status: verified"
    },
    {
      "url": "https://github.com/example/oasis7/issues/123#issuecomment-2",
      "body": "Pre-PR Local Role Review: passed\\nTask UID: $TASK_UID\\nReview Findings Disposition: no_findings"
    },
    {
      "url": "https://github.com/example/oasis7/issues/123#issuecomment-3",
      "body": "<!-- oasis7-pm-evidence -->\\nTask UID: $TASK_UID\\nEvidence Phase: pre_pr_ready\\nRole: tpm"
    }
  ],
  "number": 123,
  "title": "GitHub-backed no-cache fixture",
  "url": "https://github.com/example/oasis7/issues/123"
}
EOF
no_cache_log="$TMPDIR/gh-no-cache.log"
no_cache_git_log="$TMPDIR/git-no-cache.log"
no_cache_out="$TMPDIR/no-cache.out"
no_cache_err="$TMPDIR/no-cache.err"
if ! TEST_GH_ISSUE_LIST_JSON="$NO_CACHE_ISSUE_LIST" \
  TEST_GH_ISSUE_BODY_JSON="$NO_CACHE_ISSUE_BODY" \
  TEST_GH_ISSUE_FULL_JSON="$NO_CACHE_ISSUE_FULL" \
  TEST_GH_ISSUE_VIEW_JSON="$NO_CACHE_ISSUE_COMMENTS" \
  PREPARE_TASK_PR_ALLOW_GITHUB_ISSUE_FALLBACK=1 \
  run_prepare "$no_cache_log" "$no_cache_git_log" --create >"$no_cache_out" 2>"$no_cache_err"; then
  cat "$no_cache_err" >&2
  cat "$no_cache_log" >&2
  exit 1
fi
python3 - "$no_cache_log" "$no_cache_out" "$no_cache_err" "$SMOKE_BRANCH" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")
branch = sys.argv[4]

if not any(line.startswith(f"pr create --base main --head {branch} --fill --body Task: ") for line in gh_lines):
    raise SystemExit(f"expected gh pr create on no-cache path, got: {gh_lines}")
if "Refs #123" not in gh_text or any(token in gh_text.lower() for token in ("closes #123", "fixes #123", "resolves #123")):
    raise SystemExit(f"generated no-cache PR body must reference without auto-closing task issue #123, got: {gh_text}")
if not any(line.startswith("issue list -R eng-cc/oasis7 --search") for line in gh_lines):
    raise SystemExit(f"expected no-cache GitHub issue search, got: {gh_lines}")
if any(line.startswith("project item-edit") for line in gh_lines):
    raise SystemExit(f"did not expect Project item edit without cached project_item_id, got: {gh_lines}")
if not any(line.startswith("issue edit 123 -R eng-cc/oasis7") for line in gh_lines):
    raise SystemExit(f"expected no-cache record-pr issue body update, got: {gh_lines}")
if not any(line.startswith("issue comment 123 -R eng-cc/oasis7") for line in gh_lines):
    raise SystemExit(f"expected no-cache record-pr PR-watch evidence comment, got: {gh_lines}")
if "Created PR:" not in stdout:
    raise SystemExit(f"expected PR creation output, got: {stdout}")
if stderr:
    raise SystemExit(f"did not expect stderr on no-cache create path: {stderr}")
PY

reset_smoke_branch_to_base
write_task_binding
write_project_trace
"$REAL_GIT" -C "$SMOKE_WORKTREE" add ".pm/tasks/$TASK_UID.yaml" "doc/engineering/project.md"
"$REAL_GIT" -C "$SMOKE_WORKTREE" \
  -c user.name="oasis7 smoke" \
  -c user.email="smoke@example.invalid" \
  -c commit.gpgsign=false \
  commit --no-verify -m "test: mapping-backed PM fixture base" >/dev/null
SOURCE_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
write_role_review_packet "$SOURCE_HEAD" "no_findings"
mkdir -p "$SMOKE_WORKTREE/.pm/github-project-sync"
cat > "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json" <<EOF
{
  "project": {
    "repo": "example/oasis7"
  },
  "tasks": {
    "$TASK_UID": {
      "claim_verifications": [
        {"status": "verified", "task_uid": "$TASK_UID", "verification_exit_code": 0, "verified_at": "2026-06-03T00:02:00+08:00", "verify_command": "fixture"}
      ],
      "evidence_comments": ["https://github.com/example/oasis7/issues/123#issuecomment-1"],
      "issue_number": 123,
      "issue_url": "https://github.com/example/oasis7/issues/123",
      "owner_role": "tpm",
      "priority": "P3",
      "project_item_id": "PVTI_fixture",
      "status": "ready",
      "task_uid": "$TASK_UID",
      "title": "prepare task pr role review fixture",
      "worktree_hint": "$SMOKE_WORKTREE_CANONICAL"
    }
  },
  "version": 1
}
EOF
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

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")
branch = sys.argv[5]

if not any(line.startswith(f"pr create --base main --head {branch} --fill --body Task: ") for line in gh_lines):
    raise SystemExit(f"expected gh pr create first, got: {gh_lines}")
if "Refs #123" not in gh_text or any(token in gh_text.lower() for token in ("closes #123", "fixes #123", "resolves #123")):
    raise SystemExit(f"generated PR body must reference without auto-closing task issue #123, got: {gh_text}")
if not any(line.startswith("project item-edit") for line in gh_lines):
    raise SystemExit(f"expected record-pr Project field update calls, got: {gh_lines}")
if not any(line.startswith("issue edit 123 -R eng-cc/oasis7") for line in gh_lines):
    raise SystemExit(f"expected record-pr issue body update, got: {gh_lines}")
if not any(line.startswith("issue comment 123 -R eng-cc/oasis7") for line in gh_lines):
    raise SystemExit(f"expected record-pr issue evidence comment, got: {gh_lines}")
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

promotion_receipt="$TMPDIR/promotion-receipt.json"
PROMOTION_HEAD="$("$REAL_GIT" -C "$SMOKE_WORKTREE" rev-parse HEAD)"
cat >"$promotion_receipt" <<EOF
{"repository":"example/oasis7","task_uid":"$TASK_UID","task_issue_number":123,"pr_number":999,"check_name":"required-gate","check_app_id":42,"planner_digest":"fixture","head_oid":"$PROMOTION_HEAD"}
EOF
promotion_receipt_helper="$TMPDIR/promotion-receipt-helper.py"
cat >"$promotion_receipt_helper" <<'PY'
#!/usr/bin/env python3
import os,sys
with open(os.environ["TEST_GH_LOG"],"a") as f: f.write("receipt "+" ".join(sys.argv[1:])+"\n")
PY
promotion_project_helper="$TMPDIR/promotion-project-helper.py"
cat >"$promotion_project_helper" <<'PY'
#!/usr/bin/env python3
import json,os,pathlib,sys
root=pathlib.Path(sys.argv[2]); uid=sys.argv[sys.argv.index("--task-uid")+1]
path=root/".pm/github-project-sync/tasks.json"; data=json.loads(path.read_text())
data["tasks"][uid]["status"]="pr_watch"; data["tasks"][uid]["workflow_phase"]="pr_watch"
path.write_text(json.dumps(data)+"\n")
with open(os.environ["TEST_GH_LOG"],"a") as f: f.write("record-pr ordinary\n")
PY
chmod +x "$promotion_receipt_helper" "$promotion_project_helper"

set_promotion_ready_truth() {
  python3 - "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json,sys
p=sys.argv[1]; d=json.load(open(p)); r=d["tasks"][sys.argv[2]]
r["status"]="ready"; r["workflow_phase"]="pre_pr_ready"
open(p,"w").write(json.dumps(d)+"\n")
PY
}
assert_promoted_truth() {
  python3 - "$SMOKE_WORKTREE/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]))["tasks"][sys.argv[2]]
assert (r["status"],r["workflow_phase"])==("pr_watch","pr_watch"),r
PY
}

"$REAL_GIT" -C "$SMOKE_WORKTREE" update-index --assume-unchanged .pm/github-project-sync/tasks.json
set_promotion_ready_truth
promotion_log="$TMPDIR/gh-promotion.log"
PREPARE_TASK_PR_CI_READY_RECEIPT_PATH="$promotion_receipt_helper" \
PREPARE_TASK_PR_PROJECT_TASK_PATH="$promotion_project_helper" TEST_PR_STATE_TSV=$'true\tOPEN\t' \
  run_prepare "$promotion_log" "$TMPDIR/git-promotion.log" --promote-draft "$promotion_receipt" >/dev/null
python3 - "$promotion_log" <<'PY'
import sys
lines=open(sys.argv[1]).read().splitlines()
ready=next(i for i,x in enumerate(lines) if x.startswith("pr ready 999 "))
record=lines.index("record-pr ordinary")
assert ready < record,lines
receipt=next(x for x in lines if x.startswith("receipt "))
assert "--allow-ready-pr" not in receipt,lines
PY
assert_promoted_truth

set_promotion_ready_truth
recovery_log="$TMPDIR/gh-promotion-recovery.log"
PREPARE_TASK_PR_CI_READY_RECEIPT_PATH="$promotion_receipt_helper" \
PREPARE_TASK_PR_PROJECT_TASK_PATH="$promotion_project_helper" TEST_PR_STATE_TSV=$'false\tOPEN\t' \
  run_prepare "$recovery_log" "$TMPDIR/git-promotion-recovery.log" --promote-draft "$promotion_receipt" >/dev/null
if grep -q '^pr ready ' "$recovery_log"; then
  echo "already-ready recovery must not call gh pr ready" >&2
  exit 1
fi
grep -q '^record-pr ordinary$' "$recovery_log"
grep -q '^receipt .*--allow-ready-pr' "$recovery_log"
assert_promoted_truth

for unsafe_state in $'false\tCLOSED\t' $'false\tOPEN\t2026-07-14T00:00:00Z'; do
  set_promotion_ready_truth
  unsafe_log="$TMPDIR/gh-promotion-unsafe-${unsafe_state//[^a-zA-Z]/_}.log"
  if PREPARE_TASK_PR_CI_READY_RECEIPT_PATH="$promotion_receipt_helper" \
    PREPARE_TASK_PR_PROJECT_TASK_PATH="$promotion_project_helper" TEST_PR_STATE_TSV="$unsafe_state" \
      run_prepare "$unsafe_log" "$TMPDIR/git-promotion-unsafe.log" --promote-draft "$promotion_receipt" >/dev/null 2>&1; then
    echo "closed/merged promotion recovery must fail" >&2
    exit 1
  fi
  if grep -Eq '^receipt |^record-pr ordinary$|^pr ready ' "$unsafe_log"; then
    echo "closed/merged recovery reached receipt, ready, or record: $(cat "$unsafe_log")" >&2
    exit 1
  fi
done
"$REAL_GIT" -C "$SMOKE_WORKTREE" update-index --no-assume-unchanged .pm/github-project-sync/tasks.json

reset_project_mapping_after_record_pr

title_log="$TMPDIR/gh-title.log"
title_git_log="$TMPDIR/git-title.log"
title_out="$TMPDIR/title.out"
title_err="$TMPDIR/title.err"
if ! run_prepare "$title_log" "$title_git_log" --create --title "Fixture PR title" >"$title_out" 2>"$title_err"; then
  cat "$title_err" >&2
  exit 1
fi

python3 - "$title_log" "$title_err" "$SMOKE_BRANCH" "$TASK_UID" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
stderr = Path(sys.argv[2]).read_text(encoding="utf-8")
branch = sys.argv[3]
task_uid = sys.argv[4]

expected = f"pr create --base main --head {branch} --title Fixture PR title --body Task: {task_uid}"
if not any(line.startswith(expected) for line in gh_lines):
    raise SystemExit(f"expected titled gh pr create to include generated body, got: {gh_lines}")
if "Refs #123" not in gh_text or any(token in gh_text.lower() for token in ("closes #123", "fixes #123", "resolves #123")):
    raise SystemExit(f"titled generated PR body must reference without auto-closing task issue #123, got: {gh_text}")
if stderr:
    raise SystemExit(f"did not expect stderr on titled create path: {stderr}")
PY
reset_project_mapping_after_record_pr

closed_reuse_log="$TMPDIR/gh-closed-reuse.log"
closed_reuse_git_log="$TMPDIR/git-closed-reuse.log"
closed_reuse_out="$TMPDIR/closed-reuse.out"
closed_reuse_err="$TMPDIR/closed-reuse.err"
TEST_EXISTING_PR_JSON="[{\"url\":\"https://github.com/example/oasis7/pull/closed\",\"headRefName\":\"$SMOKE_BRANCH\",\"baseRefName\":\"main\",\"state\":\"CLOSED\",\"headRepository\":{\"name\":\"oasis7\"},\"headRepositoryOwner\":{\"login\":\"example\"}}]" \
  run_prepare "$closed_reuse_log" "$closed_reuse_git_log" --create \
  >"$closed_reuse_out" 2>"$closed_reuse_err"
python3 - "$closed_reuse_log" "$closed_reuse_out" "$closed_reuse_err" "$SMOKE_BRANCH" <<'PY'
from pathlib import Path
import sys

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")
branch = sys.argv[4]
if not any(line.startswith(f"pr create --base main --head {branch}") for line in gh_lines):
    raise SystemExit(f"a CLOSED exact head/base PR must not be reused: {gh_lines}")
if "pull/closed" in stdout:
    raise SystemExit(f"closed PR URL leaked into resumed PR result: {stdout}")
if stderr:
    raise SystemExit(f"unexpected stderr on closed-PR replacement path: {stderr}")
PY
reset_project_mapping_after_record_pr

foreign_log="$TMPDIR/gh-foreign.log"
foreign_git_log="$TMPDIR/git-foreign.log"
TEST_EXISTING_PR_JSON="[{\"url\":\"https://github.com/foreign/oasis7/pull/7\",\"headRefName\":\"$SMOKE_BRANCH\",\"baseRefName\":\"main\",\"state\":\"OPEN\",\"headRepository\":{\"name\":\"oasis7\"},\"headRepositoryOwner\":{\"login\":\"foreign\"}}]" \
  run_prepare "$foreign_log" "$foreign_git_log" --create >"$TMPDIR/foreign.out" 2>"$TMPDIR/foreign.err"
grep -F "pr create --base main --head $SMOKE_BRANCH" "$foreign_log" >/dev/null
if grep -F 'foreign/oasis7/pull/7' "$TMPDIR/foreign.out" >/dev/null; then
  echo "foreign same-name head PR must not be reused" >&2; exit 1
fi
reset_project_mapping_after_record_pr

merged_log="$TMPDIR/gh-merged.log"
merged_git_log="$TMPDIR/git-merged.log"
if TEST_EXISTING_PR_JSON="[{\"url\":\"https://github.com/example/oasis7/pull/8\",\"headRefName\":\"$SMOKE_BRANCH\",\"baseRefName\":\"main\",\"state\":\"MERGED\",\"headRepository\":{\"name\":\"oasis7\"},\"headRepositoryOwner\":{\"login\":\"example\"}}]" \
  run_prepare "$merged_log" "$merged_git_log" --create >"$TMPDIR/merged.out" 2>"$TMPDIR/merged.err"; then
  echo "MERGED exact PR must block replacement creation" >&2; exit 1
fi
grep -F 'already MERGED; reconcile task truth' "$TMPDIR/merged.err" >/dev/null
if grep -F "pr create --base main --head $SMOKE_BRANCH" "$merged_log" >/dev/null; then
  echo "MERGED exact PR unexpectedly created replacement" >&2; exit 1
fi
reset_project_mapping_after_record_pr

bad_body_file="$TMPDIR/bad-pr-body.md"
printf 'Task body without GitHub task reference.\n' > "$bad_body_file"
bad_body_log="$TMPDIR/gh-bad-body.log"
bad_body_git_log="$TMPDIR/git-bad-body.log"
bad_body_err="$TMPDIR/bad-body.err"
if run_prepare "$bad_body_log" "$bad_body_git_log" --create --body-file "$bad_body_file" >/dev/null 2>"$bad_body_err"; then
  echo "expected --body-file without task reference to fail" >&2
  exit 1
fi

python3 - "$bad_body_log" "$bad_body_git_log" "$bad_body_err" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")

if any(line.startswith("pr create ") for line in gh_lines):
    raise SystemExit(f"expected no PR creation for bad explicit body, got: {gh_lines}")
if any("push" in line for line in git_lines):
    raise SystemExit(f"expected no push for bad explicit body, got: {git_lines}")
if "must include a non-closing GitHub task reference" not in stderr:
    raise SystemExit(f"expected task-reference error for bad explicit body, got: {stderr}")
PY

for closing_link in \
  "Closes eng-cc/oasis7#123" \
  "Fixes https://github.com/eng-cc/oasis7/issues/123"; do
  closing_body_file="$TMPDIR/closing-pr-body.md"
  printf 'Refs #123\n%s\n' "$closing_link" > "$closing_body_file"
  if run_prepare "$TMPDIR/gh-closing-body.log" "$TMPDIR/git-closing-body.log" \
    --create --body-file "$closing_body_file" >/dev/null 2>"$TMPDIR/closing-body.err"; then
    echo "expected qualified auto-close link to fail: $closing_link" >&2
    exit 1
  fi
  grep -F 'must include a non-closing GitHub task reference' "$TMPDIR/closing-body.err" >/dev/null
done

behind_log="$TMPDIR/gh-behind.log"
behind_git_log="$TMPDIR/git-behind.log"
behind_out="$TMPDIR/behind.out"
behind_err="$TMPDIR/behind.err"
TEST_REV_LIST_COUNTS="1 2" run_prepare "$behind_log" "$behind_git_log" --create >"$behind_out" 2>"$behind_err"

python3 - "$behind_log" "$behind_git_log" "$behind_out" "$behind_err" "$SMOKE_BRANCH" <<'PY'
from __future__ import annotations

import sys
from pathlib import Path

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
git_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
stdout = Path(sys.argv[3]).read_text(encoding="utf-8")
stderr = Path(sys.argv[4]).read_text(encoding="utf-8")
branch = sys.argv[5]

if not any(line.startswith(f"pr create --base main --head {branch} --fill --body Task: ") for line in gh_lines):
    raise SystemExit(f"expected gh pr create first on behind-but-allowed path, got: {gh_lines}")
if "Refs #123" not in gh_text or any(token in gh_text.lower() for token in ("closes #123", "fixes #123", "resolves #123")):
    raise SystemExit(f"generated PR must reference without auto-closing task issue before terminal finalization, got: {gh_text}")
if not any(line.startswith("project item-edit") for line in gh_lines):
    raise SystemExit(f"expected record-pr Project field update calls on behind-but-allowed path, got: {gh_lines}")
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
reset_project_mapping_after_record_pr

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

gh_text = Path(sys.argv[1]).read_text(encoding="utf-8")
gh_lines = gh_text.splitlines()
stdout = Path(sys.argv[2]).read_text(encoding="utf-8")
stderr = Path(sys.argv[3]).read_text(encoding="utf-8")
branch = sys.argv[4]

if not any(line.startswith(f"pr create --base main --head {branch} --fill --body Task: ") for line in gh_lines):
    raise SystemExit(f"expected gh pr create first after addressed findings, got: {gh_lines}")
if "Refs #123" not in gh_text or any(token in gh_text.lower() for token in ("closes #123", "fixes #123", "resolves #123")):
    raise SystemExit(f"generated PR must reference without auto-closing task issue before terminal finalization, got: {gh_text}")
if not any(line.startswith("project item-edit") for line in gh_lines):
    raise SystemExit(f"expected record-pr Project field update calls after addressed findings, got: {gh_lines}")
if "- findings disposition: addressed" not in stdout:
    raise SystemExit("expected addressed disposition in output")
if stderr:
    raise SystemExit(f"did not expect stderr on addressed path: {stderr}")
PY
reset_project_mapping_after_record_pr

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
if not review.get("evidence_sink"):
    raise SystemExit(f"expected evidence_sink in review payload, got: {review}")
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

reset_smoke_branch_to_base
write_changed_path_fixture "crates/oasis7_viewer/src/lib.rs"
viewer_required_json="$TMPDIR/viewer-required.json"
run_prepare "$TMPDIR/gh-viewer-required.log" "$TMPDIR/git-viewer-required.log" --json >"$viewer_required_json"

python3 - "$viewer_required_json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
required = payload["local_required_validation"]
command = required["recommended_required_command"] or ""
reason = required["reason_summary"] or ""
expected_present = [
    "OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=true",
    "OASIS7_CI_RUN_VIEWER_WASM_CHECK=true",
    "OASIS7_CI_RUN_VIEWER_PERF_SMOKE=true",
]
missing = [item for item in expected_present if item not in command]
if missing:
    raise SystemExit(f"viewer required command missing {missing}: {command}")
if required["scope"] != "targeted":
    raise SystemExit(f"expected targeted viewer scope, got: {required}")
if "viewer:crates/oasis7_viewer/src/lib.rs" not in reason:
    raise SystemExit(f"expected viewer reason, got: {reason}")
PY

echo "prepare-task-pr.test: OK"
