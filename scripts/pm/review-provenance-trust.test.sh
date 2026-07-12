#!/usr/bin/env bash
set -euo pipefail
export OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS=1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
TASK_UID="task_11111111111111111111111111111111"
ROLE="repository_health_engineer"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/scratch/$TASK_UID/artifacts"
cp "$ROOT_DIR/scripts/pm/record-pre-pr-review.sh" "$REPO/scripts/pm/record-pre-pr-review.sh"
cp "$ROOT_DIR/scripts/pm/validate-review-provenance.py" "$REPO/scripts/pm/validate-review-provenance.py"
chmod +x "$REPO/scripts/pm/record-pre-pr-review.sh"
git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/README.md"
git -C "$REPO" add .
git -C "$REPO" commit -qm base
HEAD_SHA="$(git -C "$REPO" rev-parse HEAD)"

ARTIFACT_REL=".pm/scratch/$TASK_UID/artifacts/review.txt"
RECEIPT_REL=".pm/scratch/$TASK_UID/fake-dispatch-receipt.json"
LEDGER_REL=".pm/scratch/$TASK_UID/slice-ledger.jsonl"
printf 'no_findings from fixture\n' >"$REPO/$ARTIFACT_REL"
DIGEST="$(shasum -a 256 "$REPO/$ARTIFACT_REL" | awk '{print $1}')"
cat >"$REPO/$RECEIPT_REL" <<EOF
{"receipt_type":"oasis7_subagent_dispatch","issuer":"tpm","dispatch_id":"11111111-1111-4111-8111-111111111111","role":"$ROLE","source_head":"$HEAD_SHA","contract_digest":"$(printf fixture | shasum -a 256 | awk '{print $1}')"}
EOF
cat >"$REPO/$LEDGER_REL" <<EOF
{"task_uid":"$TASK_UID","role":"$ROLE","status":"completed","head":"$HEAD_SHA","slice_id":"11111111-1111-4111-8111-111111111111","activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified","artifact_digest":"$DIGEST","scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"fixture","artifacts":["$ARTIFACT_REL"],"dispatch_receipt":"$RECEIPT_REL"}
EOF

set +e
(cd "$REPO" && SCRIPT_DIR="$REPO/scripts/pm" ./scripts/pm/record-pre-pr-review.sh \
  --task-uid "$TASK_UID" --roles "$ROLE" \
  --review-evidence "$ROLE: no_findings" \
  --review-verdicts "$ROLE scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence fixture --verification fixture --residual-risk fixture \
  --slice-ledger "$LEDGER_REL" --reviewed-paths README.md --source-head "$HEAD_SHA" \
  --allow-dirty --print-only) >"$TMPDIR/out" 2>"$TMPDIR/err"
status=$?
set -e
if [[ "$status" == "0" ]]; then
  echo "expected TPM-issued fake dispatch receipt to be rejected" >&2
  exit 1
fi
if ! grep -Eiq 'trusted dispatch|codex_runtime|receipt issuer|dispatch receipt' "$TMPDIR/err"; then
  echo "expected trusted-dispatch rejection, got:" >&2
  cat "$TMPDIR/err" >&2
  exit 1
fi

python3 - "$REPO/$RECEIPT_REL" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['issuer']='codex_runtime'
json.dump(p,open(sys.argv[1],'w',encoding='utf-8'))
PY
unset OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS
if python3 "$REPO/scripts/pm/validate-review-provenance.py" --root "$REPO" \
  --ledger "$LEDGER_REL" --roles "$ROLE" --source-head "$HEAD_SHA" \
  >"$TMPDIR/unattested.out" 2>"$TMPDIR/unattested.err"; then
  echo "expected issuer text without runtime-verifiable attestation to fail closed" >&2
  exit 1
fi
grep -F 'runtime-verifiable dispatch attestation is unavailable' "$TMPDIR/unattested.err" >/dev/null

echo "review-provenance-trust.test: OK"
