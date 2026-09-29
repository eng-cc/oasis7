#!/usr/bin/env bash
set -euo pipefail
export OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS=1
export PYTHONDONTWRITEBYTECODE=1

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
cp "$ROOT_DIR/scripts/pm/review-findings-resolution.py" "$TEST_REPO/scripts/pm/review-findings-resolution.py"
cp "$ROOT_DIR/scripts/pm/review-batch-epoch.py" "$TEST_REPO/scripts/pm/review-batch-epoch.py"
cp "$ROOT_DIR/scripts/pm/review_preflight_handoff.py" "$TEST_REPO/scripts/pm/review_preflight_handoff.py"
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
git -C "$TEST_REPO" add README.md .pm/.gitignore scripts/pm/record-pre-pr-review.sh scripts/pm/validate-review-provenance.py scripts/pm/review-findings-resolution.py scripts/pm/review-batch-epoch.py scripts/pm/review_preflight_handoff.py
git -C "$TEST_REPO" -c user.name="oasis7 smoke" -c user.email="smoke@example.invalid" commit -q -m "base"
git -C "$TEST_REPO" branch base

printf 'changed\n' >> "$TEST_REPO/README.md"
git -C "$TEST_REPO" add README.md
git -C "$TEST_REPO" -c user.name="oasis7 smoke" -c user.email="smoke@example.invalid" commit -q -m "change"
mkdir -p "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111"
ARTIFACT_PATH="$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-return.md"
HEAD_SHA="$(git -C "$TEST_REPO" rev-parse HEAD)"
python3 - "$ARTIFACT_PATH" "$HEAD_SHA" <<'PY'
import json, sys
json.dump({
    "task_uid": "task_11111111111111111111111111111111",
    "role": "repository_health_engineer",
    "status": "completed",
    "head": sys.argv[2],
    "slice_id": "11111111-1111-4111-8111-111111111111",
    "epoch": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "disposition": "no_findings",
    "findings": [],
    "residual_risk": "fixture risk",
}, open(sys.argv[1], "w"))
with open(sys.argv[1], "a") as handle:
    handle.write("\n")
PY
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
BATCH_EVIDENCE_DIGEST="$(python3 -c 'print("b" * 64)')"
BATCH_RESULT="$TMPDIR/frozen-batch.json"
python3 "$TEST_REPO/scripts/pm/review-batch-epoch.py" --root "$TEST_REPO" create \
  --task-uid task_11111111111111111111111111111111 --head "$HEAD_SHA" \
  --evidence-digest "$BATCH_EVIDENCE_DIGEST" \
  --slice repository_health_engineer=11111111-1111-4111-8111-111111111111 \
  >"$BATCH_RESULT"
BATCH_EPOCH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["epoch"])' "$BATCH_RESULT")"
BATCH_PATH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["batch_path"])' "$BATCH_RESULT")"
python3 - "$BATCH_RESULT" "$BATCH_PATH" "$TEST_REPO" "$HEAD_SHA" "$BATCH_EVIDENCE_DIGEST" "$BATCH_EPOCH" <<'PY'
import json, pathlib, sys
result_path, batch_path, root, head, evidence, epoch = sys.argv[1:]
result = json.loads(pathlib.Path(result_path).read_text(encoding="utf-8"))
batch = json.loads(pathlib.Path(batch_path).read_text(encoding="utf-8"))
expected = {
    "schema": "oasis7-review-batch/v1", "epoch": epoch,
    "task_uid": "task_11111111111111111111111111111111",
    "frozen_head": head, "relevant_evidence_digest": evidence,
    "expected_slices": [{"role": "repository_health_engineer",
                          "slice_id": "11111111-1111-4111-8111-111111111111"}],
}
if result.get("batch_path") != batch_path or batch != expected:
    raise SystemExit("canonical fixture batch does not match the helper-returned plan identity")
if pathlib.Path(batch_path).resolve() != (pathlib.Path(root) / ".pm" / "scratch" /
                                             expected["task_uid"] / "review-batches" / f"{epoch}.json").resolve():
    raise SystemExit("canonical fixture batch path is not the derived epoch path")
PY
python3 - "$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-return.md" \
  "$TEST_REPO/$LEDGER_REL" "$BATCH_EPOCH" <<'PY'
import hashlib, json, pathlib, sys
artifact_path, ledger_path, epoch = map(pathlib.Path, sys.argv[1:])
payload = json.loads(artifact_path.read_text(encoding="utf-8"))
payload["epoch"] = str(epoch)
artifact_path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
rows = [json.loads(line) for line in ledger_path.read_text(encoding="utf-8").splitlines() if line.strip()]
if len(rows) != 1 or (rows[0].get("role"), rows[0].get("slice_id")) != (
        "repository_health_engineer", "11111111-1111-4111-8111-111111111111"):
    raise SystemExit("canonical batch fixture expected exactly its one completed role row")
rows[0]["epoch"] = str(epoch)
rows[0]["artifact_digest"] = hashlib.sha256(artifact_path.read_bytes()).hexdigest()
ledger_path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
PY
python3 - "$PLAN" "$HEAD_SHA" "$BASE_A" "$LEDGER_REL" "$BATCH_EVIDENCE_DIGEST" "$BATCH_EPOCH" "$BATCH_PATH" <<'PY'
import json, sys
plan, head, comparison, ledger, evidence, epoch, batch_path = sys.argv[1:]
json.dump({"schema":"oasis7-review-plan/v1","task_uid":"task_11111111111111111111111111111111","frozen_head":head,"comparison_ref":"refs/heads/base","comparison_oid":comparison,"relevant_evidence_digest":evidence,"roles":["repository_health_engineer"],"expected_slices":[{"role":"repository_health_engineer","slice_id":"11111111-1111-4111-8111-111111111111"}],"epoch":epoch,"batch_path":batch_path,"preflight":{"status":"incomplete","ledger_path":ledger}},open(plan,"w"))
PY
python3 - "$TEST_REPO/$LEDGER_REL" "$BATCH_EPOCH" <<'PY'
import json,sys
path=sys.argv[1]
rows=[json.loads(line) for line in open(path) if line.strip()]
for row in rows: row["epoch"]=sys.argv[2]
open(path,"w").write("".join(json.dumps(row)+"\n" for row in rows))
PY
PLAN_ORIGINAL="$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-plans/frozen-plan-original.json"
cp "$PLAN" "$PLAN_ORIGINAL"
git -C "$TEST_REPO" update-ref refs/heads/base "$BASE_B"

# Review plans are authoritative, so traversal and escaping symlink paths must
# be rejected before the helper reads roles or derives the preflight ledger.
cp "$PLAN" "$TMPDIR/outside-review-plan.json"
ln -s "$TMPDIR/outside-review-plan.json" "$TEST_REPO/.pm/scratch/escaping-review-plan.json"
for escaped_plan in ../outside-review-plan.json .pm/scratch/escaping-review-plan.json; do
  if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
    --task-uid task_11111111111111111111111111111111 \
    --review-plan "$escaped_plan" \
    --review-evidence "repository_health_engineer: no_findings; smoke" \
    --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
    --finding-disposition-evidence "smoke evidence" \
    --verification "helper -> smoke -> observed" \
    --residual-risk "fixture risk" \
    --print-only >"$TMPDIR/escaped-plan.out" 2>"$TMPDIR/escaped-plan.err"; then
    echo "expected escaping review plan path to fail: $escaped_plan" >&2
    exit 1
  fi
  grep -qi "escapes repository root" "$TMPDIR/escaped-plan.err"
done

"$TEST_REPO/scripts/pm/record-pre-pr-review.sh" --task-uid task_11111111111111111111111111111111 --review-plan "$PLAN" --review-evidence "repository_health_engineer: no_findings; smoke" --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" --finding-disposition-evidence "smoke evidence" --verification "helper -> smoke -> observed" --residual-risk "fixture risk" --print-only >"$TMPDIR/frozen-plan.out"
grep -q "Reviewed Changed Paths: README.md" "$TMPDIR/frozen-plan.out"
grep -q "Review Plan: .pm/scratch/task_11111111111111111111111111111111/review-plans/frozen-plan.json" "$TMPDIR/frozen-plan.out"
grep -q "Slice Ledger: .pm/scratch/task_11111111111111111111111111111111/slice-ledger.jsonl" "$TMPDIR/frozen-plan.out"

# The immutable plan owns the preflight ledger identity. A caller-provided
# repository-owned alternate ledger must fail before packet publication.
ALTERNATE_LEDGER="$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/alternate-slice-ledger.jsonl"
cp "$TEST_REPO/$LEDGER_REL" "$ALTERNATE_LEDGER"
cp "$TEST_REPO/$LEDGER_REL" "$TMPDIR/canonical-before-alternate.jsonl"
cp "$ALTERNATE_LEDGER" "$TMPDIR/alternate-before.jsonl"
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --review-plan "$PLAN" \
  --review-evidence "repository_health_engineer: no_findings; alternate ledger" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "alternate ledger" \
  --verification "alternate ledger" --residual-risk "fixture risk" \
  --slice-ledger "$ALTERNATE_LEDGER" --print-only \
  >"$TMPDIR/alternate-ledger.out" 2>"$TMPDIR/alternate-ledger.err"; then
  echo "record-pre-pr-review accepted a role-return ledger outside the plan preflight path" >&2
  exit 1
fi
grep -Eqi 'preflight ledger|immutable|slice ledger|ledger path' "$TMPDIR/alternate-ledger.err"
cmp -s "$TEST_REPO/$LEDGER_REL" "$TMPDIR/canonical-before-alternate.jsonl"
cmp -s "$ALTERNATE_LEDGER" "$TMPDIR/alternate-before.jsonl"

# The direct recorder must reject a completed role return with unresolved
# findings instead of defaulting the packet disposition to no_findings.
FIX2_FAILURES=0
python3 - "$ARTIFACT_PATH" "$TEST_REPO/$LEDGER_REL" <<'PY'
import hashlib, json, sys
artifact_path, ledger_path = sys.argv[1:]
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
artifact = json.load(open(artifact_path, encoding="utf-8"))
artifact.update({"disposition": "findings", "findings": [{"id": "FIX2-UNRESOLVED", "summary": "fixture unresolved finding"}]})
with open(artifact_path, "w", encoding="utf-8") as handle:
    json.dump(artifact, handle, sort_keys=True)
    handle.write("\n")
artifact_digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
for row in rows:
    row["findings"] = "findings"
    row["artifact_digest"] = artifact_digest
with open(ledger_path, "w", encoding="utf-8") as handle:
    handle.write("".join(json.dumps(row) + "\n" for row in rows))
PY
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --review-plan "$PLAN" \
  --review-evidence "repository_health_engineer: findings; unresolved fixture" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=blocked" \
  --finding-disposition addressed \
  --finding-disposition-evidence "arbitrary text" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "unresolved fixture risk" \
  --slice-ledger "$LEDGER_REL" \
  --print-only >"$TMPDIR/unresolved-findings.out" 2>"$TMPDIR/unresolved-findings.err"; then
  echo "record-pre-pr-review accepted unresolved role findings" >&2
  FIX2_FAILURES=1
else
  grep -Eiq 'unresolved|findings|blocked' "$TMPDIR/unresolved-findings.err"
  [[ ! -s "$TMPDIR/unresolved-findings.out" ]] || {
    echo "unresolved findings produced a review packet" >&2
    FIX2_FAILURES=1
  }
fi

# Artifact and ledger dispositions are a semantic pair. A caller must not be
# able to rewrite only the ledger field while retaining an artifact that says
# findings (or vice versa) and still obtain a passed packet.
python3 - "$ARTIFACT_PATH" "$TEST_REPO/$LEDGER_REL" <<'PY'
import hashlib, json, sys
artifact_path, ledger_path = sys.argv[1:]
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
artifact = json.load(open(artifact_path, encoding="utf-8"))
artifact.update({"disposition": "findings", "findings": [{"id": "FIX2-MISMATCH", "summary": "fixture artifact finding"}]})
with open(artifact_path, "w", encoding="utf-8") as handle:
    json.dump(artifact, handle, sort_keys=True)
    handle.write("\n")
artifact_digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
for row in rows:
    row["findings"] = "no_findings"
    row["artifact_digest"] = artifact_digest
with open(ledger_path, "w", encoding="utf-8") as handle:
    handle.write("".join(json.dumps(row) + "\n" for row in rows))
PY
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --review-plan "$PLAN" \
  --review-evidence "repository_health_engineer: no_findings; semantic mismatch fixture" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "semantic mismatch fixture risk" \
  --slice-ledger "$LEDGER_REL" \
  --print-only >"$TMPDIR/artifact-ledger-mismatch.out" 2>"$TMPDIR/artifact-ledger-mismatch.err"; then
  echo "record-pre-pr-review accepted artifact-ledger disposition mismatch" >&2
  FIX2_FAILURES=1
else
  grep -Eiq 'mismatch|findings|disposition' "$TMPDIR/artifact-ledger-mismatch.err"
  [[ ! -s "$TMPDIR/artifact-ledger-mismatch.out" ]] || {
    echo "artifact-ledger mismatch produced a review packet" >&2
    FIX2_FAILURES=1
  }
fi

if (( FIX2_FAILURES != 0 )); then
  exit 1
fi

# Without a review plan, no_findings returns retain the legacy opaque path:
# arbitrary JSON objects and arrays are accepted, while the reserved schema
# opts into structured validation and must fail without the required fields.
set_opaque_artifact() {
  local content="$1"
  printf '%s\n' "$content" >"$ARTIFACT_PATH"
  python3 - "$ARTIFACT_PATH" "$TEST_REPO/$LEDGER_REL" <<'PY'
import hashlib, json, sys
artifact_path, ledger_path = sys.argv[1:]
digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
for row in rows:
    row["findings"] = "no_findings"
    row["artifact_digest"] = digest
open(ledger_path, "w", encoding="utf-8").write("".join(json.dumps(row) + "\n" for row in rows))
PY
}

# A plan-backed return must be structured even when the ledger says no_findings;
# malformed or opaque bytes are only valid on the legacy no-plan path.
for plan_opaque_content in 'not-json' '{"arbitrary":"plan-backed"}' '["opaque"]'; do
  set_opaque_artifact "$plan_opaque_content"
  if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
    --task-uid task_11111111111111111111111111111111 \
    --review-plan "$PLAN" \
    --review-evidence "repository_health_engineer: no_findings; plan opaque" \
    --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
    --finding-disposition-evidence "plan opaque" \
    --verification "plan opaque" --residual-risk "fixture risk" \
    --slice-ledger "$LEDGER_REL" --print-only \
    >"$TMPDIR/plan-opaque.out" 2>"$TMPDIR/plan-opaque.err"; then
    echo "record-pre-pr-review accepted plan-backed opaque artifact: $plan_opaque_content" >&2
    exit 1
  fi
  grep -Eiq 'structured|json|identity|disposition|findings|artifact' "$TMPDIR/plan-opaque.err"
done

set_opaque_artifact '{"arbitrary":"json object"}'
if ! "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; opaque object" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "opaque object" \
  --verification "opaque object" --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" --comparison-ref refs/heads/base --print-only \
  >"$TMPDIR/opaque-object.out" 2>"$TMPDIR/opaque-object.err"; then
  cat "$TMPDIR/opaque-object.err" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/opaque-object.out" >/dev/null

set_opaque_artifact '["opaque", 1, false]'
if ! "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; opaque array" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "opaque array" \
  --verification "opaque array" --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" --comparison-ref refs/heads/base --print-only \
  >"$TMPDIR/opaque-array.out" 2>"$TMPDIR/opaque-array.err"; then
  cat "$TMPDIR/opaque-array.err" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/opaque-array.out" >/dev/null

set_opaque_artifact 'not-json'
if ! "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; opaque bytes" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "opaque bytes" \
  --verification "opaque bytes" --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" --comparison-ref refs/heads/base --print-only \
  >"$TMPDIR/opaque-bytes.out" 2>"$TMPDIR/opaque-bytes.err"; then
  cat "$TMPDIR/opaque-bytes.err" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/opaque-bytes.out" >/dev/null

set_opaque_artifact '{"schema":"oasis7-review-return/v1","arbitrary":"reserved"}'
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; reserved" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "reserved" \
  --verification "reserved" --residual-risk "fixture risk" \
  --slice-ledger "$LEDGER_REL" --comparison-ref refs/heads/base --print-only \
  >"$TMPDIR/reserved-schema.out" 2>"$TMPDIR/reserved-schema.err"; then
  echo "record-pre-pr-review accepted malformed reserved structured artifact" >&2
  exit 1
fi
grep -Eiq 'structured|identity|task_uid|disposition|findings' "$TMPDIR/reserved-schema.err"

python3 - "$PLAN" <<'PY'
import json,sys
p=json.load(open(sys.argv[1])); p["comparison_oid"]="0"*40; json.dump(p,open(sys.argv[1],"w"))
PY
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" --task-uid task_11111111111111111111111111111111 --review-plan "$PLAN" --review-evidence "repository_health_engineer: no_findings; smoke" --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" --finding-disposition-evidence "smoke evidence" --verification "helper -> smoke -> observed" --residual-risk "fixture risk" --slice-ledger "$LEDGER_REL" --print-only >/dev/null 2>"$TMPDIR/tampered-oid.err"; then
  echo "expected tampered frozen comparison OID to fail" >&2
  exit 1
fi
grep -qi "comparison OID" "$TMPDIR/tampered-oid.err"

# A missing immutable batch is recoverable only from a fully validated plan.
# The recorder prints the exact helper command but never executes it.
MISSING_PLAN="$TEST_REPO/.pm/scratch/task_11111111111111111111111111111111/review-plans/missing-batch-plan.json"
cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
MISSING_COLLECTION="${BATCH_PATH%.json}.collection.json"
RECOVERY_LEDGER_BEFORE="$TMPDIR/recovery-ledger-before.jsonl"
cp "$TEST_REPO/$LEDGER_REL" "$RECOVERY_LEDGER_BEFORE"
rm "$BATCH_PATH"

run_missing_plan_recorder() {
  "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
    --task-uid task_11111111111111111111111111111111 \
    --review-plan "$MISSING_PLAN" \
    --review-evidence "repository_health_engineer: no_findings; recovery fixture" \
    --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
    --finding-disposition-evidence "recovery fixture" \
    --verification "validated plan -> missing batch -> printed command" \
    --residual-risk "recovery fixture risk" \
    --slice-ledger "$LEDGER_REL" --print-only "$@"
}

if run_missing_plan_recorder >"$TMPDIR/missing-batch.out" 2>"$TMPDIR/missing-batch.err"; then
  echo "record-pre-pr-review accepted a plan whose immutable batch was missing" >&2
  exit 1
fi
python3 - "$TMPDIR/missing-batch.err" "$TMPDIR/missing-batch.out" "$MISSING_PLAN" \
  "$TEST_REPO" "$TEST_REPO/scripts/pm/review-batch-epoch.py" "$HEAD_SHA" \
  "$BATCH_EVIDENCE_DIGEST" "$BATCH_PATH" "$MISSING_COLLECTION" <<'PY'
import json
import pathlib
import shlex
import sys

stderr_path, stdout_path, plan_path, root, helper, head, evidence, batch_path, collection_path = sys.argv[1:]
root = str(pathlib.Path(root).resolve())
helper = str(pathlib.Path(helper).resolve())
stderr = pathlib.Path(stderr_path).read_text(encoding="utf-8")
stdout = pathlib.Path(stdout_path).read_text(encoding="utf-8")
plan = json.loads(pathlib.Path(plan_path).read_text(encoding="utf-8"))
expected_slices = sorted(plan["expected_slices"], key=lambda item: (item["role"], item["slice_id"]))
command = ["python3", helper, "--root", root, "create", "--task-uid", plan["task_uid"],
           "--head", head, "--evidence-digest", evidence]
for item in expected_slices:
    command.extend(("--slice", f'{item["role"]}={item["slice_id"]}'))
lines = [line[2:] for line in stderr.splitlines() if line.startswith("  python3 ")]
if "immutable review-plan batch is missing" not in stderr or "not executed" not in stderr:
    raise SystemExit("missing-batch failure did not explain the nonexecuted recovery hint")
if lines != [shlex.join(command)]:
    raise SystemExit(f"missing-batch recovery command is not exact: {lines!r}")
if stdout:
    raise SystemExit("missing-batch failure emitted a review packet")
if pathlib.Path(batch_path).exists() or pathlib.Path(collection_path).exists():
    raise SystemExit("recorder executed recovery or wrote a collection")
PY
cmp -s "$TEST_REPO/$LEDGER_REL" "$RECOVERY_LEDGER_BEFORE"

assert_no_recovery_hint() {
  local label="$1"
  shift
  if run_missing_plan_recorder "$@" >"$TMPDIR/$label.out" 2>"$TMPDIR/$label.err"; then
    echo "invalid missing-batch plan was accepted: $label" >&2
    exit 1
  fi
  if grep -Eq 'review-batch-epoch\.py|recovery command was not executed' "$TMPDIR/$label.err"; then
    echo "invalid missing-batch plan received an actionable recovery hint: $label" >&2
    cat "$TMPDIR/$label.err" >&2
    exit 1
  fi
  [[ ! -s "$TMPDIR/$label.out" ]] || {
    echo "invalid missing-batch plan emitted a packet: $label" >&2
    exit 1
  }
}

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
assert_no_recovery_hint wrong-head --source-head "$BASE_A"

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
python3 - "$MISSING_PLAN" <<'PY'
import json, sys
path = sys.argv[1]
plan = json.load(open(path, encoding="utf-8"))
plan["epoch"] = "0" * 64
json.dump(plan, open(path, "w", encoding="utf-8"))
PY
assert_no_recovery_hint wrong-epoch

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
python3 - "$MISSING_PLAN" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
plan = json.loads(path.read_text(encoding="utf-8"))
plan["batch_path"] = str(path.parent.parent / "review-batches" / "wrong-epoch.json")
path.write_text(json.dumps(plan), encoding="utf-8")
PY
assert_no_recovery_hint wrong-path

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
python3 - "$MISSING_PLAN" "$BATCH_PATH" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
plan = json.loads(path.read_text(encoding="utf-8"))
plan["batch_path"] = str(pathlib.Path(sys.argv[2]).with_name("custom-missing.json"))
path.write_text(json.dumps(plan), encoding="utf-8")
PY
assert_no_recovery_hint missing-custom-batch
grep -q 'no_safe_repair; new_review_epoch_required' "$TMPDIR/missing-custom-batch.err"

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
python3 - "$MISSING_PLAN" <<'PY'
import json, sys
path = sys.argv[1]
plan = json.load(open(path, encoding="utf-8"))
plan["roles"] = ["qa_engineer"]
plan["expected_slices"][0]["role"] = "qa_engineer"
json.dump(plan, open(path, "w", encoding="utf-8"))
PY
assert_no_recovery_hint wrong-role

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
python3 - "$MISSING_PLAN" <<'PY'
import json, sys
path = sys.argv[1]
plan = json.load(open(path, encoding="utf-8"))
plan["roles"] *= 2
plan["expected_slices"] *= 2
json.dump(plan, open(path, "w", encoding="utf-8"))
PY
assert_no_recovery_hint duplicate-role-slice

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
python3 - "$MISSING_PLAN" "$TEST_REPO/$LEDGER_REL" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
plan = json.loads(path.read_text(encoding="utf-8"))
plan["preflight"]["ledger_path"] = str(pathlib.Path(sys.argv[2]).with_name("missing-preflight-ledger.jsonl"))
path.write_text(json.dumps(plan), encoding="utf-8")
PY
assert_no_recovery_hint missing-preflight-ledger
grep -qi 'Review Plan preflight ledger cannot be resolved' "$TMPDIR/missing-preflight-ledger.err"

cp "$PLAN_ORIGINAL" "$MISSING_PLAN"
printf '{"schema":"oasis7-review-collection/v1","status":"passed"}\n' >"$MISSING_COLLECTION"
if run_missing_plan_recorder >"$TMPDIR/collected-missing-batch.out" 2>"$TMPDIR/collected-missing-batch.err"; then
  echo "record-pre-pr-review accepted a missing batch with a collection receipt" >&2
  exit 1
fi
if grep -Eq 'review-batch-epoch\.py|recovery command was not executed' "$TMPDIR/collected-missing-batch.err"; then
  echo "collected missing-batch plan received an actionable recovery hint" >&2
  cat "$TMPDIR/collected-missing-batch.err" >&2
  exit 1
fi
[[ ! -s "$TMPDIR/collected-missing-batch.out" ]]
cmp -s "$TEST_REPO/$LEDGER_REL" "$RECOVERY_LEDGER_BEFORE"
[[ -f "$MISSING_COLLECTION" ]]

echo "record-pre-pr-review.test: OK"
