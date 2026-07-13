#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
HEAD=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
HOLD='{"kind":"normal_pr_ci_watch","requester":"workflow","reason":"normal","resume_authority":"workflow","active":false}'

case "${1:-forged-cache}" in
  forged-cache)
    cat >"$TMPDIR/fixture.json" <<JSON
{"number":81,"repository":"eng-cc/oasis7","url":"https://example.invalid/pull/81","state":"OPEN","headRefOid":"$HEAD","mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","merge_hold":$HOLD,"comments":[{"id":"C1","body":"please fix the typo","url":"https://example.invalid/c/1","author":{"login":"reviewer"}}],"comment_dispositions":[{"node_id":"C1","head_oid":"$HEAD","disposition":"addressed","evidence":"hand edited tasks.json"}],"reviews":[],"threads":[],"required_status_checks":[],"statusCheckRollup":[]}
JSON
    if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/fixture.json" --json >"$TMPDIR/out"; then
      echo "RED disposition evidence: caller-authored cache disposition was trusted" >&2
      exit 1
    fi
    ;;
  top-review)
    mkdir -p "$TMPDIR/bin"
    cat >"$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
[[ "$1 $2" == "api repos/eng-cc/oasis7/issues/comments/IC_verified" ]] || exit 9
printf '{"body":"","user":{"login":"trusted-bot"},"created_at":"2026-07-11T00:00:00Z","html_url":"https://github.com/eng-cc/oasis7/issues/2198#issuecomment-1"}\n'
SH
    chmod +x "$TMPDIR/bin/gh"
    cat >"$TMPDIR/fixture.json" <<JSON
{"number":82,"repository":"eng-cc/oasis7","url":"https://example.invalid/pull/82","state":"OPEN","headRefOid":"$HEAD","mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","merge_hold":$HOLD,"comments":[],"reviews":[{"id":"R1","state":"COMMENTED","body":"please add the missing regression test","url":"https://example.invalid/r/1","author":{"login":"reviewer"},"submittedAt":"2026-07-11T00:00:00Z"}],"review_dispositions":[{"node_id":"R1","head_oid":"$HEAD","disposition":"addressed","evidence_receipt":{"source":"github_task_issue_comment","runtime_verified":true,"task_uid":"task_11111111111111111111111111111111","repository":"eng-cc/oasis7","issue_number":2198,"pr_number":82,"head_oid":"$HEAD","github_node_id":"IC_verified","url":"https://github.com/eng-cc/oasis7/issues/2198#issuecomment-1","author":"trusted-bot","observed_at":"2026-07-11T00:00:00Z","digest":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}],"threads":[],"required_status_checks":[],"statusCheckRollup":[]}
JSON
    PATH="$TMPDIR/bin:$PATH" python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/fixture.json" --json >"$TMPDIR/out" || {
      echo "RED disposition evidence: verified top-level review disposition was not parsed" >&2
      exit 1
    }
    ;;
  writer)
    [[ -x "$ROOT_DIR/scripts/pm/record-pr-disposition.sh" ]] || {
      echo "RED disposition evidence: GitHub-backed disposition writer is missing" >&2
      exit 1
    }
    "$ROOT_DIR/scripts/pm/record-pr-disposition.sh" --help >"$TMPDIR/help"
    grep -F -- '--task-uid' "$TMPDIR/help" >/dev/null && grep -F -- '--pr-number' "$TMPDIR/help" >/dev/null || {
      echo "RED disposition evidence: writer lacks task/PR identity contract" >&2; exit 1; }
    grep -Eq -- '--node-id|--review-node-id|--comment-node-id' "$TMPDIR/help" || {
      echo "RED disposition evidence: writer lacks GitHub node identity contract" >&2; exit 1; }
    ;;
  *) echo "unknown case" >&2; exit 2 ;;
esac
echo "pr-disposition-evidence.test: ${1:-forged-cache}: OK"
