#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
mkdir -p "$TMPDIR/bin" "$TMPDIR/root/.pm/github-project-sync"
TASK_UID=task_11111111111111111111111111111111
cat >"$TMPDIR/root/.pm/github-project-sync/tasks.json" <<JSON
{"tasks":{"$TASK_UID":{"issue_number":1,"merge_hold":{"kind":"normal_pr_ci_watch","requester":"workflow","reason":"normal","resume_authority":"workflow","active":false,"evidence_receipt":{"source":"github_task_issue_comment","runtime_verified":true,"task_uid":"$TASK_UID","repository":"eng-cc/oasis7","issue_number":1,"pr_number":9,"head_oid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","github_node_id":"IC_hold","url":"https://github.com/eng-cc/oasis7/issues/1#issuecomment-hold","author":"workflow","observed_at":"2026-07-11T00:00:00Z","digest":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}}}}
JSON
cat >"$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$GH_LOG"
case "$1 $2" in
  "pr view") printf '{"number":9,"url":"https://example.invalid/pull/9","state":"OPEN","mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"task/x","headRefOid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","baseRefName":"main"}\n' ;;
  "repo view") printf '{"nameWithOwner":"eng-cc/oasis7"}\n' ;;
  "api graphql")
    if [[ "$*" == *comments* || "$*" == *reviews* || "$*" == *reviewThreads* ]]; then
      surface=comments; [[ "$*" == *reviews* ]] && surface=reviews; [[ "$*" == *reviewThreads* ]] && surface=reviewThreads
      printf '{"data":{"repository":{"pullRequest":{"%s":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}}}}}\n' "$surface"
    else
      printf '{"data":{"repository":{"pullRequest":{"commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}}}}]}}}}}\n'
    fi ;;
  "api repos/eng-cc/oasis7/branches/main/protection")
    case "${POLICY_CASE:?}" in
      denied|transport) echo 'gh: Resource not accessible (HTTP 403)' >&2 ;;
      *) echo 'gh: Not Found (HTTP 404)' >&2 ;;
    esac
    exit 1 ;;
  "api repos/eng-cc/oasis7/rulesets")
    case "${POLICY_CASE:?}" in
      ruleset) printf '[{"id":7,"enforcement":"active","conditions":{"ref_name":{"include":["~DEFAULT_BRANCH"]}},"rules":[{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"required-gate","integration_id":42}]}}]}]\n' ;;
      none|transport) printf '[]\n' ;;
      paged)
        if [[ "$*" == *'page=2'* || "$*" == *'--paginate'* ]]; then
          printf '[{"id":8,"target":"branch","enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"]}},"rules":[{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"page-two-gate","integration_id":42}]}}]}]\n'
        else
          printf '[{"id":6,"target":"tag","enforcement":"active","conditions":{"ref_name":{"include":["~ALL"]}},"rules":[]}]\n'
        fi ;;
      filtered) printf '[{"id":10,"target":"tag","enforcement":"active","conditions":{"ref_name":{"include":["~ALL"]}},"rules":[{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"tag-only","integration_id":42}]}}]},{"id":11,"target":"branch","enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/develop"]}},"rules":[{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"develop-only","integration_id":42}]}}]}]\n' ;;
      denied) exit 1 ;;
    esac ;;
  "api repos/eng-cc/oasis7/issues/comments/IC_hold") printf '{"body":"","user":{"login":"workflow"},"html_url":"https://github.com/eng-cc/oasis7/issues/1#issuecomment-hold"}\n' ;;
  "api repos/eng-cc/oasis7/issues/1/comments") printf '[[{"id":101,"body":"<!-- oasis7-merge-hold -->\\n- task_uid: `task_11111111111111111111111111111111`\\n- repository: `eng-cc/oasis7`\\n- issue_number: `1`\\n- pr_number: `9`\\n- head_oid: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`\\n- node_id: `merge_hold`\\n- kind: `merge_hold`\\n- disposition: `cleared`\\n- hold_kind: `normal_pr_ci_watch`\\n- active: `false`\\n- requester: `workflow`\\n- reason: `normal`\\n- resume_authority: `workflow`\\n","user":{"login":"workflow"},"created_at":"2026-07-11T00:00:00Z","html_url":"https://github.com/eng-cc/oasis7/issues/1#issuecomment-101"}]]\n' ;;
  *) echo "unexpected gh: $*" >&2; exit 9 ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"

run_case() {
  local name="$1"
  : >"$TMPDIR/$name.log"
  set +e
  GH_LOG="$TMPDIR/$name.log" POLICY_CASE="$name" PATH="$TMPDIR/bin:$PATH" \
    python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" 9 --root "$TMPDIR/root" \
    --task-uid "$TASK_UID" --json >"$TMPDIR/$name.out" 2>"$TMPDIR/$name.err"
  local status=$?
  set -e
  if [[ "$name" == denied || "$name" == transport ]]; then
    ! grep -F 'api repos/eng-cc/oasis7/rulesets' "$TMPDIR/$name.log" >/dev/null || {
      echo "RED policy-discovery-$name: classic read error incorrectly fell back to rulesets" >&2; return 1; }
  else
    grep -F 'api repos/eng-cc/oasis7/rulesets' "$TMPDIR/$name.log" >/dev/null || {
      echo "RED policy-discovery-$name: rulesets were not queried after classic 404" >&2; return 1; }
  fi
  if [[ "$name" == none || "$name" == filtered ]]; then
    [[ "$status" == 0 ]] || { echo "RED policy-discovery-none: explicit no-required policy was not accepted" >&2; return 1; }
  elif [[ "$name" == paged ]]; then
    [[ "$status" != 0 ]] || { echo "RED policy-discovery-paged: page-two required check was not discovered" >&2; return 1; }
    grep -Eq 'page=2|--paginate' "$TMPDIR/$name.log" || { echo "RED policy-discovery-paged: rulesets list was not paginated" >&2; return 1; }
  else
    [[ "$status" != 0 ]] || { echo "RED policy-discovery-$name: unsafe policy state was accepted" >&2; return 1; }
    python3 -m json.tool "$TMPDIR/$name.out" >/dev/null || {
      echo "RED policy-discovery-$name: failure was not returned as structured gate state" >&2; return 1; }
  fi
}

run_case "${1:-ruleset}"
echo "pr-policy-discovery-contract.test: OK"
