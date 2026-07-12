#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

normal_hold='{"kind":"normal_pr_ci_watch","requester":"workflow","reason":"normal","resume_authority":"workflow","active":false}'
CASE="${1:-all}"

# Required checks are identities, not names.  A success from the wrong GitHub App
# must not mask the required App's failure.
if [[ "$CASE" == all || "$CASE" == check-app ]]; then
cat >"$TMPDIR/check-app.json" <<JSON
{"number":71,"url":"https://example.invalid/pull/71","state":"OPEN","mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","merge_hold":$normal_hold,"comments":[],"reviews":[],"threads":[],"required_status_checks":[{"context":"required-gate","app_id":42}],"statusCheckRollup":[{"name":"required-gate","app_id":42,"conclusion":"FAILURE"},{"name":"required-gate","app_id":99,"conclusion":"SUCCESS"}]}
JSON
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/check-app.json" --json >"$TMPDIR/check-app.out"; then
  echo "RED required-check-app-identity: wrong-app success masked required app failure" >&2
  exit 1
fi
fi

# Conversation disposition is explicit and head-bound.  Benign prose is not a
# substitute for the ledger, and an acknowledged node must not block forever.
if [[ "$CASE" == all || "$CASE" == comments ]]; then
cat >"$TMPDIR/comment-disposition.json" <<JSON
{"number":72,"url":"https://example.invalid/pull/72","state":"OPEN","headRefOid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","merge_hold":$normal_hold,"comments":[{"id":"C_lgtm","body":"LGTM","url":"https://example.invalid/c/1","author":{"login":"reviewer"}},{"id":"C_fixed","body":"please fix typo","url":"https://example.invalid/c/2","author":{"login":"reviewer"}}],"comment_dispositions":[{"node_id":"C_fixed","head_oid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","disposition":"addressed","evidence":"commit:aaaaaaaa"}],"reviews":[],"threads":[],"required_status_checks":[],"statusCheckRollup":[]}
JSON
python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" --fixture "$TMPDIR/comment-disposition.json" --json >"$TMPDIR/comment-disposition.out"
fi

# A caller-authored three-field JSON is not a ready-for-merge receipt.  It must
# bind trusted issuer, repo, PR, head, observed_at, and the current gate epoch.
if [[ "$CASE" == all || "$CASE" == ready-receipt ]]; then
REPO="$TMPDIR/repo"
mkdir -p "$REPO"
git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/README.md"
git -C "$REPO" add README.md
git -C "$REPO" commit -qm base
cat >"$TMPDIR/fake-ready.json" <<'JSON'
{"ready_for_merge":true,"status":"ready","blockers":[]}
JSON
if OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1 PM_ROOT_DIR="$REPO" \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" --claim-type ready_for_merge \
  --verification-profile fixture_repository_state --verify-command true \
  --pr-gate-json "$TMPDIR/fake-ready.json" --json >"$TMPDIR/fake-ready.out" 2>"$TMPDIR/fake-ready.err"; then
  echo "RED ready-receipt-binding: caller-authored three-field JSON was trusted" >&2
  exit 1
fi
fi

# Production provenance must expose an explicit resumable capability blocker;
# it must neither pass nor collapse into an unstructured attestation error.
if [[ "$CASE" == all || "$CASE" == provenance ]]; then
cat >"$TMPDIR/empty-ledger.jsonl" <<'JSON'
{"role":"repository_health_engineer","status":"completed","head":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","slice_id":"11111111-1111-4111-8111-111111111111","dispatch_receipt":"missing.json","activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified","artifact_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"none","artifacts":[]}
JSON
set +e
python3 "$ROOT_DIR/scripts/pm/validate-review-provenance.py" --root "$TMPDIR" \
  --ledger empty-ledger.jsonl --roles repository_health_engineer \
  --source-head aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa --mode unattended \
  >"$TMPDIR/capability.out" 2>"$TMPDIR/capability.err"
cap_status=$?
set -e
[[ "$cap_status" -ne 0 ]] || { echo "RED provenance-capability: unattested production dispatch passed" >&2; exit 1; }
grep -Eiq 'capability[-_ ]blocked|resumable.*capability|capability.*resume' "$TMPDIR/capability.err" || {
  echo "RED provenance-capability: failure is not a structured resumable capability blocker" >&2
  exit 1
}
fi

echo "workflow-adversarial-contract.test: OK"
