#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

TASK_UID="task_11111111111111111111111111111111"
mkdir -p "$TMPDIR/scripts" "$TMPDIR/.pm/tasks" "$TMPDIR/doc/engineering"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"

cat > "$TMPDIR/doc/engineering/project.md" <<EOF
# Engineering Project Fixture

- [x] workflow-lint-pr-evidence-fixture [test_tier_required]: exercise post-pr evidence chain. Trace: #123 ($TASK_UID)
EOF

cat > "$TMPDIR/.pm/tasks/$TASK_UID.yaml" <<EOF
task_uid: $TASK_UID
title: "workflow lint post-pr evidence fixture"
owner_role: tpm
worktree_hint: fixture-worktree
execution_log_path: .pm/tasks/$TASK_UID.execution.md
status: done
priority: P2
source_signal: null
source_refs: []
doc_refs:
  - doc/engineering/project.md
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-25T00:00:00+08:00
last_started_at: 2026-06-25T00:00:00+08:00
last_claim_type: task_complete
last_verified_at: 2026-06-25T00:01:00+08:00
last_verification_status: verified
last_closed_at: 2026-06-25T00:02:00+08:00
EOF

cat > "$TMPDIR/.pm/tasks/task_22222222222222222222222222222222.yaml" <<EOF
this line is intentionally invalid task yaml
EOF

set +e
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/workflow-lint.sh" --allow-unbound --phase current >"$TMPDIR/full-scan-malformed.stdout" 2>&1
FULL_SCAN_MALFORMED_STATUS=$?
set -e
if [[ "$FULL_SCAN_MALFORMED_STATUS" == "0" ]]; then
  echo "workflow-lint.test: expected full task scan to fail on malformed unrelated task yaml" >&2
  exit 1
fi
if ! grep -Fq "invalid key/value line" "$TMPDIR/full-scan-malformed.stdout"; then
  echo "workflow-lint.test: expected malformed unrelated task yaml parser failure" >&2
  cat "$TMPDIR/full-scan-malformed.stdout" >&2
  exit 1
fi

write_log() {
  local actual_result=$1
  cat > "$TMPDIR/.pm/tasks/$TASK_UID.execution.md" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: workflow lint post-pr evidence fixture
- owner_role: tpm
- worktree_hint: fixture-worktree

## 2026-06-25 00:00:00 CST / tpm
- 完成内容: prepared workflow-lint fixture.
- 遗留事项: none.
- Action: exercise post-merge trace chain.
- Validation Command: ./scripts/pm/workflow-lint.sh --phase post-pr
- Expected Result: post-pr lint accepts only task-local pull request locator evidence.
- Actual Result: $actual_result
- claim-ready evidence: ./scripts/pm/claim-ready.sh --claim-type task_complete --verify-command fixture --task-uid $TASK_UID
- task-closeout evidence: ./scripts/pm/task-closeout.sh --role tpm --task-uid $TASK_UID --verify-command fixture
- Blocker / Next Action: none.
EOF
}

write_log "root markdown file is intentionally not task-local evidence."
cat > "$TMPDIR/PR.md" <<EOF
Task UID: $TASK_UID
PR URL: https://github.com/example/oasis7/pull/1
EOF

set +e
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/workflow-lint.sh" --task-uid "$TASK_UID" --phase post-pr >"$TMPDIR/root-pr-md.stdout" 2>&1
ROOT_PR_MD_STATUS=$?
set -e
if [[ "$ROOT_PR_MD_STATUS" == "0" ]]; then
  echo "workflow-lint.test: expected root PR.md-only evidence to fail" >&2
  exit 1
fi
if ! grep -Fq "PR evidence chain not locatable" "$TMPDIR/root-pr-md.stdout"; then
  echo "workflow-lint.test: expected missing task-local PR evidence failure" >&2
  cat "$TMPDIR/root-pr-md.stdout" >&2
  exit 1
fi

mkdir -p "$TMPDIR/.pm/working_memory"
cat > "$TMPDIR/.pm/working_memory/$TASK_UID.yaml" <<EOF
task_uid: $TASK_UID
entries:
  - summary: "PR URL: https://github.com/example/oasis7/pull/1"
EOF
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/workflow-lint.sh" --task-uid "$TASK_UID" --phase post-pr >"$TMPDIR/working-memory.stdout"
if ! grep -Fq "evidence: .pm/working_memory/$TASK_UID.yaml" "$TMPDIR/working-memory.stdout"; then
  echo "workflow-lint.test: expected task-local working memory to be reported as PR evidence" >&2
  cat "$TMPDIR/working-memory.stdout" >&2
  exit 1
fi
rm -f "$TMPDIR/.pm/working_memory/$TASK_UID.yaml"

write_log "PR evidence recorded in the task execution log. PR URL: https://github.com/example/oasis7/pull/1"
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/workflow-lint.sh" --task-uid "$TASK_UID" --phase post-pr >"$TMPDIR/task-local.stdout"
if ! grep -Fq "evidence: .pm/tasks/$TASK_UID.execution.md" "$TMPDIR/task-local.stdout"; then
  echo "workflow-lint.test: expected task execution log to be reported as PR evidence" >&2
  cat "$TMPDIR/task-local.stdout" >&2
  exit 1
fi

GITHUB_UID="task_33333333333333333333333333333333"
GITHUB_ROOT="$TMPDIR/github-backed"
mkdir -p "$GITHUB_ROOT/scripts" "$GITHUB_ROOT/.pm/tasks" "$GITHUB_ROOT/.pm/github-project-sync" "$GITHUB_ROOT/bin"
cp -R "$ROOT_DIR/scripts/pm" "$GITHUB_ROOT/scripts/pm"
cat > "$GITHUB_ROOT/bin/gh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\$*" in
  "issue view 123 -R example/oasis7 --json comments")
    cat <<'JSON'
{"comments":[
{"body":"<!-- oasis7-pm-claim-verification -->\nTask UID: $GITHUB_UID\nClaim Type: ready_for_pr\nVerification Status: verified"},
{"body":"Pre-PR Local Role Review: passed\nTask UID: $GITHUB_UID\nReview Findings Disposition: no_findings"},
{"body":"<!-- oasis7-pm-evidence -->\nTask UID: $GITHUB_UID\nEvidence Phase: close\nRole: tpm"}
]}
JSON
    ;;
  *)
    echo "unexpected gh invocation: \$*" >&2
    exit 9
    ;;
esac
EOF
chmod +x "$GITHUB_ROOT/bin/gh"
cat > "$GITHUB_ROOT/.pm/github-project-sync/tasks.json" <<EOF
{
  "project": {
    "repo": "example/oasis7"
  },
  "tasks": {
    "$GITHUB_UID": {
      "claim_verifications": [
        {
          "status": "verified",
          "task_uid": "$GITHUB_UID",
          "verification_exit_code": 0,
          "verified_at": "2026-06-30T00:01:00+08:00",
          "verify_command": "fixture"
        }
      ],
      "evidence_comments": [
        "https://github.com/example/oasis7/issues/123#issuecomment-1"
      ],
      "issue_number": 123,
      "issue_url": "https://github.com/example/oasis7/issues/123",
      "last_claim_verification_at": "2026-06-30T00:01:00+08:00",
      "last_closed_at": "2026-06-30T00:02:00+08:00",
      "project_item_id": "PVTI_fixture",
      "status": "ready",
      "task_uid": "$GITHUB_UID",
      "worktree_hint": "$GITHUB_ROOT"
    }
  },
  "version": 1
}
EOF
PATH="$GITHUB_ROOT/bin:$PATH" PM_ROOT_DIR="$GITHUB_ROOT" "$GITHUB_ROOT/scripts/pm/workflow-lint.sh" --task-uid "$GITHUB_UID" --phase pr-ready >"$TMPDIR/github-backed.stdout"
if ! grep -Fq "github-backed" "$TMPDIR/github-backed.stdout"; then
  echo "workflow-lint.test: expected GitHub-backed task fallback to pass" >&2
  cat "$TMPDIR/github-backed.stdout" >&2
  exit 1
fi

rm -f "$GITHUB_ROOT/.pm/github-project-sync/tasks.json"
cat > "$GITHUB_ROOT/bin/gh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\$*" in
  "issue list -R eng-cc/oasis7 --search $GITHUB_UID in:body --json number,url,title,state --limit 5")
    cat <<'JSON'
[{"number":123,"state":"OPEN","title":"GitHub-backed fixture","url":"https://github.com/eng-cc/oasis7/issues/123"}]
JSON
    ;;
  "issue view 123 -R eng-cc/oasis7 --json body,comments,number,title,url")
    cat <<'JSON'
{"number":123,"title":"GitHub-backed fixture","url":"https://github.com/eng-cc/oasis7/issues/123","body":"<!-- oasis7-pm-task -->\ntask_uid: task_33333333333333333333333333333333\n\nTask metadata:\n- owner_role: \`tpm\`\n- module: \`engineering\`\n- status: \`ready\`\n- priority: \`P2\`\n- worktree_hint: \`/tmp/github-backed-fixture\`\n","comments":[
{"url":"https://github.com/eng-cc/oasis7/issues/123#issuecomment-1","body":"<!-- oasis7-pm-claim-verification -->\nTask UID: task_33333333333333333333333333333333\nClaim Type: ready_for_pr\nVerification Status: verified"},
{"url":"https://github.com/eng-cc/oasis7/issues/123#issuecomment-2","body":"Pre-PR Local Role Review: passed\nTask UID: task_33333333333333333333333333333333\nReview Findings Disposition: no_findings"},
{"url":"https://github.com/eng-cc/oasis7/issues/123#issuecomment-3","body":"<!-- oasis7-pm-evidence -->\nTask UID: task_33333333333333333333333333333333\nEvidence Phase: close\nRole: tpm"}
]}
JSON
    ;;
  "issue view 123 -R eng-cc/oasis7 --json comments")
    cat <<'JSON'
{"comments":[
{"body":"<!-- oasis7-pm-claim-verification -->\nTask UID: task_33333333333333333333333333333333\nClaim Type: ready_for_pr\nVerification Status: verified"},
{"body":"Pre-PR Local Role Review: passed\nTask UID: task_33333333333333333333333333333333\nReview Findings Disposition: no_findings"},
{"body":"<!-- oasis7-pm-evidence -->\nTask UID: task_33333333333333333333333333333333\nEvidence Phase: close\nRole: tpm"}
]}
JSON
    ;;
  *)
    echo "unexpected gh invocation: \$*" >&2
    exit 9
    ;;
esac
EOF
chmod +x "$GITHUB_ROOT/bin/gh"
PATH="$GITHUB_ROOT/bin:$PATH" PM_ROOT_DIR="$GITHUB_ROOT" "$GITHUB_ROOT/scripts/pm/workflow-lint.sh" --task-uid "$GITHUB_UID" --phase pr-ready >"$TMPDIR/github-backed-no-cache.stdout"
if ! grep -Fq "github-backed" "$TMPDIR/github-backed-no-cache.stdout"; then
  echo "workflow-lint.test: expected GitHub issue fallback without tasks.json to pass" >&2
  cat "$TMPDIR/github-backed-no-cache.stdout" >&2
  exit 1
fi

cat > "$GITHUB_ROOT/.pm/github-project-sync/tasks.json" <<EOF
{
  "project": {
    "repo": "example/oasis7"
  },
  "tasks": {
    "$GITHUB_UID": {
      "claim_verifications": [
        {
          "status": "verified",
          "task_uid": "$GITHUB_UID",
          "verification_exit_code": 0,
          "verified_at": "2026-06-30T00:01:00+08:00",
          "verify_command": "fixture"
        }
      ],
      "evidence_comments": [
        "https://github.com/example/oasis7/issues/123#issuecomment-1"
      ],
      "issue_number": 123,
      "issue_url": "https://github.com/example/oasis7/issues/123",
      "last_claim_verification_at": "2026-06-30T00:01:00+08:00",
      "last_closed_at": "2026-06-30T00:02:00+08:00",
      "project_item_id": "PVTI_fixture",
      "status": "ready",
      "task_uid": "$GITHUB_UID",
      "worktree_hint": "$GITHUB_ROOT"
    }
  },
  "version": 1
}
EOF

cat > "$GITHUB_ROOT/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "issue view 123 -R example/oasis7 --json comments")
    cat <<'JSON'
{"comments":[
{"body":"<!-- oasis7-pm-claim-verification -->\nTask UID: task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nClaim Type: ready_for_pr\nVerification Status: verified"},
{"body":"Pre-PR Local Role Review: passed\nTask UID: task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nReview Findings Disposition: no_findings"},
{"body":"<!-- oasis7-pm-evidence -->\nTask UID: task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nEvidence Phase: close\nRole: tpm"}
]}
JSON
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
EOF
chmod +x "$GITHUB_ROOT/bin/gh"
set +e
PATH="$GITHUB_ROOT/bin:$PATH" PM_ROOT_DIR="$GITHUB_ROOT" "$GITHUB_ROOT/scripts/pm/workflow-lint.sh" --task-uid "$GITHUB_UID" --phase pr-ready >"$TMPDIR/github-backed-wrong-uid.stdout" 2>&1
WRONG_UID_STATUS=$?
set -e
if [[ "$WRONG_UID_STATUS" == "0" ]]; then
  echo "workflow-lint.test: expected marker comments with wrong Task UID to fail" >&2
  exit 1
fi
if ! grep -Fq "missing claim-ready verification marker" "$TMPDIR/github-backed-wrong-uid.stdout"; then
  echo "workflow-lint.test: expected wrong Task UID marker failure" >&2
  cat "$TMPDIR/github-backed-wrong-uid.stdout" >&2
  exit 1
fi

mkdir -p "$GITHUB_ROOT/.pm/scratch/$GITHUB_UID/fallback-evidence"
printf 'fallback packet\n' > "$GITHUB_ROOT/.pm/scratch/$GITHUB_UID/fallback-evidence/unreplayed.md"
set +e
PATH="$GITHUB_ROOT/bin:$PATH" PM_ROOT_DIR="$GITHUB_ROOT" "$GITHUB_ROOT/scripts/pm/workflow-lint.sh" --task-uid "$GITHUB_UID" --phase pr-ready >"$TMPDIR/github-backed-fallback.stdout" 2>&1
FALLBACK_STATUS=$?
set -e
if [[ "$FALLBACK_STATUS" == "0" ]]; then
  echo "workflow-lint.test: expected unreplayed fallback evidence to fail" >&2
  exit 1
fi
if ! grep -Fq "unreplayed fallback evidence exists" "$TMPDIR/github-backed-fallback.stdout"; then
  echo "workflow-lint.test: expected unreplayed fallback evidence failure" >&2
  cat "$TMPDIR/github-backed-fallback.stdout" >&2
  exit 1
fi

echo "workflow-lint.test: OK"
