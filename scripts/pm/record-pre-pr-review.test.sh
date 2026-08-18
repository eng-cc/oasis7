#!/usr/bin/env bash
set -euo pipefail
export OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS=1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

TEST_REPO="$TMPDIR/repo"
mkdir -p "$TEST_REPO/scripts/pm" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/record-pre-pr-review.sh" "$TEST_REPO/scripts/pm/record-pre-pr-review.sh"
cp "$ROOT_DIR/scripts/pm/validate-review-provenance.py" "$TEST_REPO/scripts/pm/validate-review-provenance.py"
chmod +x "$TEST_REPO/scripts/pm/record-pre-pr-review.sh"

cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${TEST_GH_LOG:?}"
case "$*" in
  "issue list -R eng-cc/oasis7 --search task_11111111111111111111111111111111 in:body --json number --limit 5")
    printf '[{"number":123}]\n'
    ;;
  issue\ comment\ 123\ -R\ eng-cc/oasis7\ --body\ *)
    printf 'https://github.com/eng-cc/oasis7/issues/123#issuecomment-fixture\n'
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
EOF
chmod +x "$TMPDIR/bin/gh"

git -C "$TEST_REPO" init -q -b main
printf 'base\n' > "$TEST_REPO/README.md"
mkdir -p "$TEST_REPO/.pm"
printf 'scratch/\n' >"$TEST_REPO/.pm/.gitignore"
git -C "$TEST_REPO" add README.md .pm/.gitignore scripts/pm/record-pre-pr-review.sh scripts/pm/validate-review-provenance.py
git -C "$TEST_REPO" -c user.name="oasis7 smoke" -c user.email="smoke@example.invalid" commit -q -m "base"
git -C "$TEST_REPO" branch base

printf 'changed\n' >> "$TEST_REPO/README.md"
git -C "$TEST_REPO" add README.md
git -C "$TEST_REPO" -c user.name="oasis7 smoke" -c user.email="smoke@example.invalid" commit -q -m "change"
mkdir -p "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111"
printf 'review return\n' >"$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-return.md"
HEAD_SHA="$(git -C "$TEST_REPO" rev-parse HEAD)"
ARTIFACT_SHA="$(shasum -a 256 "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-return.md" | awk '{print $1}')"
python3 - "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/slice-ledger.jsonl" "$HEAD_SHA" "$ARTIFACT_SHA" <<'PY'
import json, sys
dispatch_id="11111111-1111-4111-8111-111111111111"
receipt=".pm/scratch/task_11111111111111111111111111111111/dispatch.json"
open(str(__import__('pathlib').Path(sys.argv[1]).parent/'dispatch.json'),"w").write(json.dumps({"receipt_type":"oasis7_subagent_dispatch","issuer":"codex_runtime","dispatch_id":dispatch_id,"role":"repository_health_engineer","source_head":sys.argv[2],"contract_digest":"0"*64})+"\n")
open(sys.argv[1], "w").write(json.dumps({"task_uid":"task_11111111111111111111111111111111","role":"repository_health_engineer","status":"completed","head":sys.argv[2],"slice_id":dispatch_id,"dispatch_receipt":receipt,"activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified: fixture","artifact_digest":sys.argv[3],"scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"fixture risk","artifacts":[".pm/scratch/task_11111111111111111111111111111111/review-return.md"]})+"\n")
PY
LEDGER_REL=".pm/scratch/task_11111111111111111111111111111111/slice-ledger.jsonl"

if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/missing.out" 2>"$TMPDIR/missing.err"; then
  echo "expected missing review evidence to fail" >&2
  exit 1
fi
grep -q -- "--review-evidence is required" "$TMPDIR/missing.err"

printf 'dirty\n' >> "$TEST_REPO/README.md"
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/dirty.out" 2>"$TMPDIR/dirty.err"; then
  echo "expected dirty worktree to fail" >&2
  exit 1
fi
grep -q "working tree is dirty" "$TMPDIR/dirty.err"
git -C "$TEST_REPO" checkout -- README.md

"$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --review-package "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-packages/smoke.diff" \
  --slice-ledger "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/slice-ledger.jsonl" \
  --visual-evidence "screenshot/model review: smoke visual evidence" \
  --ops-evidence "readiness/rollback/runbook/operator evidence: smoke ops evidence" \
  --liveops-evidence "messaging/release-note/player/community evidence: smoke liveops evidence" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/packet.out"

grep -q "Pre-PR Local Role Review: passed" "$TMPDIR/packet.out"
grep -q "Source Worktree: repo" "$TMPDIR/packet.out"
if grep -q "$TEST_REPO" "$TMPDIR/packet.out"; then
  echo "packet should not expose the local absolute worktree path" >&2
  exit 1
fi
grep -q "Review Package: .pm/scratch/task_11111111111111111111111111111111/review-packages/smoke.diff" "$TMPDIR/packet.out"
grep -q "Slice Ledger: .pm/scratch/task_11111111111111111111111111111111/slice-ledger.jsonl" "$TMPDIR/packet.out"
grep -q "Reviewed Changed Paths: README.md" "$TMPDIR/packet.out"
grep -q "Finding Disposition Evidence: smoke evidence" "$TMPDIR/packet.out"
grep -q "Visual Evidence: screenshot/model review: smoke visual evidence" "$TMPDIR/packet.out"
grep -q "Ops Evidence: readiness/rollback/runbook/operator evidence: smoke ops evidence" "$TMPDIR/packet.out"
grep -q "LiveOps Evidence: messaging/release-note/player/community evidence: smoke liveops evidence" "$TMPDIR/packet.out"

if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" \
  --comparison-ref refs/heads/base \
  --review-plan "$TMPDIR/missing-review-plan.json" \
  --print-only >"$TMPDIR/missing-plan.out" 2>"$TMPDIR/missing-plan.err"; then
  echo "expected missing review plan preflight to fail" >&2
  exit 1
fi
if ! grep -qi "review plan" "$TMPDIR/missing-plan.err"; then
  echo "record helper did not reject the missing review-plan preflight" >&2
  cat "$TMPDIR/missing-plan.err" >&2
  exit 1
fi

if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --review-package "/tmp/non-repo-review-package.diff" \
  --slice-ledger "$LEDGER_REL" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/reject.out" 2>"$TMPDIR/reject.err"; then
  echo "expected external absolute review package path to be rejected" >&2
  exit 1
fi
grep -q "Review Package must not expose a local absolute path" "$TMPDIR/reject.err"

TEST_GH_LOG="$TMPDIR/gh.log" PATH="$TMPDIR/bin:$PATH" "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" \
  --visual-evidence "screenshot/model review: smoke visual evidence" \
  --ops-evidence "readiness/rollback/runbook/operator evidence: smoke ops evidence" \
  --liveops-evidence "messaging/release-note/player/community evidence: smoke liveops evidence" \
  --comparison-ref refs/heads/base >"$TMPDIR/no-cache-comment.out"

grep -q "issue list -R eng-cc/oasis7 --search task_11111111111111111111111111111111 in:body --json number --limit 5" "$TMPDIR/gh.log"
grep -q "issue comment 123 -R eng-cc/oasis7 --body" "$TMPDIR/gh.log"
grep -q "issuecomment-fixture" "$TMPDIR/no-cache-comment.out"

# A review plan freezes comparison commit A. Moving the symbolic ref to B must
# preserve acceptance and calculate reviewed paths from A, while a tampered OID
# remains fail-closed.
git -C "$TEST_REPO" reset --hard -q "$HEAD_SHA"
BASE_A="$(git -C "$TEST_REPO" rev-parse refs/heads/base)"
BASE_B="$(git -C "$TEST_REPO" commit-tree "$HEAD_SHA^{tree}" -p "$BASE_A" -m 'moved symbolic comparison ref')"
PLAN="$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-plans/frozen-plan.json"
mkdir -p "$(dirname "$PLAN")"
python3 - "$PLAN" "$HEAD_SHA" "$BASE_A" "$LEDGER_REL" <<'PY'
import json, sys
json.dump({"schema":"oasis7-review-plan/v1","task_uid":"task_11111111111111111111111111111111","frozen_head":sys.argv[2],"comparison_ref":"refs/heads/base","comparison_oid":sys.argv[3],"relevant_evidence_digest":"b"*64,"roles":["repository_health_engineer"],"expected_slices":[{"role":"repository_health_engineer","slice_id":"11111111-1111-4111-8111-111111111111"}],"epoch":"a"*64,"batch_path":".pm/scratch/task_11111111111111111111111111111111/review-batches/"+("a"*64)+".json","preflight":{"status":"incomplete","ledger_path":sys.argv[4]}},open(sys.argv[1],"w"))
PY
python3 - "$TEST_REPO/$LEDGER_REL" <<'PY'
import json,sys
path=sys.argv[1]
rows=[json.loads(line) for line in open(path) if line.strip()]
for row in rows: row["epoch"]="a"*64
open(path,"w").write("".join(json.dumps(row)+"\n" for row in rows))
PY
git -C "$TEST_REPO" update-ref refs/heads/base "$BASE_B"
"$TEST_REPO/scripts/pm/record-pre-pr-review.sh" --task-uid task_11111111111111111111111111111111 --review-plan "$PLAN" --review-evidence "repository_health_engineer: no_findings; smoke" --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" --finding-disposition-evidence "smoke evidence" --verification "helper -> smoke -> observed" --residual-risk "fixture risk" --print-only >"$TMPDIR/frozen-plan.out"
grep -q "Reviewed Changed Paths: README.md" "$TMPDIR/frozen-plan.out"
grep -q "Review Plan: .pm/scratch/task_11111111111111111111111111111111/review-plans/frozen-plan.json" "$TMPDIR/frozen-plan.out"
grep -q "Slice Ledger: .pm/scratch/task_11111111111111111111111111111111/slice-ledger.jsonl" "$TMPDIR/frozen-plan.out"
python3 - "$PLAN" <<'PY'
import json,sys
p=json.load(open(sys.argv[1])); p["comparison_oid"]="0"*40; json.dump(p,open(sys.argv[1],"w"))
PY
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" --task-uid task_11111111111111111111111111111111 --review-plan "$PLAN" --review-evidence "repository_health_engineer: no_findings; smoke" --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" --finding-disposition-evidence "smoke evidence" --verification "helper -> smoke -> observed" --residual-risk "fixture risk" --slice-ledger "$LEDGER_REL" --print-only >/dev/null 2>"$TMPDIR/tampered-oid.err"; then
  echo "expected tampered frozen comparison OID to fail" >&2
  exit 1
fi
grep -qi "comparison OID" "$TMPDIR/tampered-oid.err"

echo "record-pre-pr-review.test: OK"
