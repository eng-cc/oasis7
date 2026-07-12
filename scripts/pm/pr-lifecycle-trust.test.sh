#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

base_fixture() {
  cat <<'JSON'
{"number":2198,"url":"https://example.invalid/pull/2198","repository":"eng-cc/oasis7","state":"OPEN","headRefName":"task/test","headRefOid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","baseRefName":"main","mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"REVIEW_REQUIRED","statusCheckRollup":[{"name":"required-gate","conclusion":"SUCCESS"}],"comments":[],"reviews":[],"threads":[]}
JSON
}

base_fixture >"$TMPDIR/missing-hold.json"
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/missing-hold.json" --json >"$TMPDIR/missing-hold.out"; then
  echo "expected omitted merge-hold truth to fail closed" >&2
  exit 1
fi

ROOT_FIXTURE="$TMPDIR/root"
TASK_UID="task_11111111111111111111111111111111"
mkdir -p "$ROOT_FIXTURE/.pm/github-project-sync"
cat >"$ROOT_FIXTURE/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$TASK_UID":{"task_uid":"$TASK_UID","issue_number":2198,"merge_hold":{"kind":"user_requested_merge_hold","requester":"user","reason":"do not merge","resume_authority":"user","active":true}}}}
EOF
base_fixture >"$TMPDIR/persisted-hold.json"
mkdir -p "$TMPDIR/bin"
cat >"$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  "pr view") cat "$TEST_PR_FIXTURE" ;;
  "repo view") printf '{"nameWithOwner":"eng-cc/oasis7"}\n' ;;
  "api graphql")
    if [[ "$*" == *'comments(first:100'* ]]; then
      if [[ "${TEST_MULTIPAGE:-0}" == 1 && "$*" != *'cursor=COMMENTS_1'* ]]; then
        printf '{"data":{"repository":{"pullRequest":{"comments":{"pageInfo":{"hasNextPage":true,"endCursor":"COMMENTS_1"},"nodes":[]}}}}}\n'
      else
        printf '{"data":{"repository":{"pullRequest":{"comments":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"page-two","body":"please fix the page-two issue","url":"https://example.invalid/comment/2","author":{"login":"reviewer"}}]}}}}}\n'
      fi
    elif [[ "$*" == *'reviews(first:100'* ]]; then
      printf '{"data":{"repository":{"pullRequest":{"reviews":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}}}}}\n'
    elif [[ "$*" == *'reviewThreads(first:100'* ]]; then
      printf '{"data":{"repository":{"pullRequest":{"reviewThreads":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}}}}}\n'
    elif [[ "$*" == *'statusCheckRollup'* ]]; then
      printf '{"data":{"repository":{"pullRequest":{"commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"__typename":"CheckRun","name":"required-gate","conclusion":"SUCCESS","status":"COMPLETED"}]}}}}]}}}}}\n'
    else
      echo "unexpected graphql query: $*" >&2; exit 9
    fi ;;
  "api repos/eng-cc/oasis7/branches/main/protection") printf '{"required_status_checks":{"contexts":["required-gate"]},"required_pull_request_reviews":{"required_approving_review_count":1}}\n' ;;
  "api repos/eng-cc/oasis7/rulesets") printf '[]\n' ;;
  "api repos/eng-cc/oasis7/issues/2198/comments") printf '[[{"id":501,"body":"<!-- oasis7-merge-hold -->\\n- task_uid: `task_11111111111111111111111111111111`\\n- repository: `eng-cc/oasis7`\\n- issue_number: `2198`\\n- pr_number: `2198`\\n- head_oid: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`\\n- node_id: `merge_hold`\\n- kind: `merge_hold`\\n- disposition: `active`\\n- hold_kind: `user_requested_merge_hold`\\n- active: `true`\\n- requester: `user`\\n- reason: `do not merge`\\n- resume_authority: `user`\\n","user":{"login":"user"},"created_at":"2026-07-11T00:00:00Z","html_url":"https://github.com/eng-cc/oasis7/issues/2198#issuecomment-501"}]]\n' ;;
  *) echo "unexpected gh call: $*" >&2; exit 9 ;;
esac
EOF
chmod +x "$TMPDIR/bin/gh"
if PATH="$TMPDIR/bin:$PATH" TEST_PR_FIXTURE="$TMPDIR/persisted-hold.json" \
  python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" 2198 \
  --task-uid "$TASK_UID" --root "$ROOT_FIXTURE" --admin-merge-authorized --json \
  >"$TMPDIR/persisted-hold.out" 2>"$TMPDIR/persisted-hold.err"; then
  echo "expected persisted user merge hold to block merge" >&2
  exit 1
fi
python3 - "$TMPDIR/persisted-hold.out" <<'PY'
import json, sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
assert any('user_requested_merge_hold' in b for b in p['blockers']), p
PY

python3 - "$TMPDIR/missing-hold.json" "$TMPDIR/blocked.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['merge_hold']={'kind':'normal_pr_ci_watch','requester':'workflow','reason':'normal','resume_authority':'workflow','active':False}; p['mergeStateStatus']='BLOCKED'
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/blocked.json" \
  --admin-merge-authorized --json >"$TMPDIR/blocked.out"; then
  echo "expected BLOCKED+REVIEW_REQUIRED without approval-only receipt to remain blocked" >&2
  exit 1
fi

python3 - "$TMPDIR/blocked.json" "$TMPDIR/approval.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
p['approval_only_receipt']={'source':'github_runtime_complete_ruleset','runtime_verified':True,'pr_number':2198,'blocking_rules':['required_review_approval']}
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/approval.json" \
  --admin-merge-authorized --json >"$TMPDIR/approval.out"; then
  echo "RED approval-only receipt: unbound receipt bypassed repo/head/time/epoch trust" >&2
  exit 1
fi

python3 - "$TMPDIR/blocked.json" "$TMPDIR/approval-bound.json" <<'PY'
import datetime,json,sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
p['approval_only_receipt']={'source':'github_runtime_complete_ruleset','runtime_verified':True,
 'repository':'eng-cc/oasis7','pr_number':2198,'head_oid':p['headRefOid'],
 'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00','Z'),
 'gate_epoch':'b'*64,'blocking_rules':['required_review_approval']}
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/approval-bound.json" \
  --admin-merge-authorized --json >"$TMPDIR/approval-bound.out"; then
  echo "expected admin merge to remain capability-blocked without runtime producer" >&2
  exit 1
fi
python3 - "$TMPDIR/approval-bound.out" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
assert p['status']=='capability_blocked',p
assert p['capability_blocked']['reason']=='complete_ruleset_runtime_receipt_unavailable',p
assert not p['use_admin_merge'],p
PY

python3 - "$TMPDIR/missing-hold.json" "$TMPDIR/latest-review.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['merge_hold']={'kind':'normal_pr_ci_watch','requester':'workflow','reason':'normal','resume_authority':'workflow','active':False}; p['reviewDecision']='APPROVED'
p['reviews']=[
 {'id':'old','author':{'login':'alice'},'state':'CHANGES_REQUESTED','body':'please fix','submittedAt':'2026-07-10T01:00:00Z'},
 {'id':'new','author':{'login':'alice'},'state':'APPROVED','body':'approved','submittedAt':'2026-07-10T02:00:00Z'}]
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/latest-review.json" --json >"$TMPDIR/latest-review.out"

python3 - "$TMPDIR/missing-hold.json" "$TMPDIR/benign.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['merge_hold']={'kind':'normal_pr_ci_watch','requester':'workflow','reason':'normal','resume_authority':'workflow','active':False}
p['comments']=[
 {'id':'bot','author':{'login':'github-actions[bot]'},'body':'Automated build summary: all checks passed.'},
 {'id':'ack','author':{'login':'reviewer'},'body':'Thanks, acknowledged; no further action.'}]
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/benign.json" --json >"$TMPDIR/benign.out"

python3 - "$TMPDIR/benign.json" "$TMPDIR/optional-failure.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['required_status_contexts']=['required-gate']; p['statusCheckRollup'].append({'name':'optional-preview','conclusion':'FAILURE'})
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/optional-failure.json" --json >"$TMPDIR/optional-failure.out"

python3 - "$ROOT_FIXTURE/.pm/github-project-sync/tasks.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['tasks'][next(iter(p['tasks']))]['merge_hold']={'kind':'normal_pr_ci_watch','requester':'workflow','reason':'normal','resume_authority':'workflow','active':False}
json.dump(p,open(sys.argv[1],'w',encoding='utf-8'))
PY

# Local cache is not an authority boundary.  A hand-edited hold/disposition must
# not clear GitHub-backed truth or dispose either conversation comments or
# top-level review bodies without a verified GitHub evidence receipt.
python3 - "$ROOT_FIXTURE/.pm/github-project-sync/tasks.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); task=p['tasks'][next(iter(p['tasks']))]
task['comment_dispositions']=[{'node_id':'page-two','head_oid':'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','disposition':'addressed','evidence':'caller-authored cache text'}]
task['review_dispositions']=[{'node_id':'review-one','head_oid':'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','disposition':'addressed','evidence':'caller-authored cache text'}]
json.dump(p,open(sys.argv[1],'w',encoding='utf-8'))
PY
if PATH="$TMPDIR/bin:$PATH" TEST_PR_FIXTURE="$TMPDIR/persisted-hold.json" TEST_MULTIPAGE=1 \
  python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" 2198 --task-uid "$TASK_UID" --root "$ROOT_FIXTURE" --json >"$TMPDIR/cache-bypass.out" 2>"$TMPDIR/cache-bypass.err"; then
  echo "RED disposition trust: hand-edited tasks.json bypassed GitHub-backed evidence" >&2
  exit 1
fi

if PATH="$TMPDIR/bin:$PATH" TEST_PR_FIXTURE="$TMPDIR/persisted-hold.json" TEST_MULTIPAGE=1 \
  python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" 2198 --task-uid "$TASK_UID" --root "$ROOT_FIXTURE" --json >"$TMPDIR/multipage.out" 2>"$TMPDIR/multipage.err"; then
  echo "expected actionable comment from second cursor page to block merge" >&2
  exit 1
fi
grep -F 'actionable PR conversation comment: https://example.invalid/comment/2' "$TMPDIR/multipage.out" >/dev/null

python3 - "$TMPDIR/missing-hold.json" "$TMPDIR/actionable-bot.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['merge_hold']={'kind':'normal_pr_ci_watch','requester':'workflow','reason':'normal','resume_authority':'workflow','active':False}
p['comments']=[{'id':'security-bot','author':{'login':'security-scan[bot]'},'body':'Automated build summary: checks passed, but a critical vulnerability must be fixed before merge.'}]
json.dump(p,open(sys.argv[2],'w',encoding='utf-8'))
PY
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/actionable-bot.json" --json >"$TMPDIR/actionable-bot.out"; then
  echo "expected actionable bot comment to block merge" >&2
  exit 1
fi
grep -F 'actionable PR conversation comment: security-bot' "$TMPDIR/actionable-bot.out" >/dev/null

echo "pr-lifecycle-trust.test: OK"
