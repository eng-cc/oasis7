#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/task_primary_package.py" "$REPO/scripts/pm/task_primary_package.py"
for helper in review-closeout.sh review-batch-epoch.py record-pre-pr-review.sh validate-review-provenance.py review-findings-resolution.py review_preflight_handoff.py workflow-impact-projection.py; do
  cp "$ROOT_DIR/scripts/pm/$helper" "$REPO/scripts/pm/$helper"
done
chmod +x "$REPO/scripts/pm/review-closeout.sh" "$REPO/scripts/pm/record-pre-pr-review.sh" "$REPO/scripts/pm/review-findings-resolution.py"
printf 'scratch/\n' >"$REPO/.pm/.gitignore"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<'JSON'
{"project":{"repo":"eng-cc/oasis7"},"tasks":{"task_11111111111111111111111111111111":{"issue_number":3615}}}
JSON

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/README.md"
git -C "$REPO" add README.md .pm/.gitignore .pm/github-project-sync/tasks.json scripts
git -C "$REPO" commit -qm base
BASE_OID="$(git -C "$REPO" rev-parse HEAD)"
printf 'implementation\n' >>"$REPO/README.md"
git -C "$REPO" add README.md
git -C "$REPO" commit -qm implementation
HEAD_OID="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" branch review-base "$BASE_OID"

TASK="task_11111111111111111111111111111111"
ROLE="repository_health_engineer"
SLICE="11111111-1111-4111-8111-111111111111"
TASK_ROOT="$REPO/.pm/scratch/$TASK"
BATCH="$TASK_ROOT/review-batches/batch.json"
mkdir -p "$TASK_ROOT/review-batches" "$TASK_ROOT/review-plans" "$TASK_ROOT/review-preflight"
EVIDENCE_DIGEST="$(printf '%s' review-evidence | shasum -a 256 | awk '{print $1}')"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" create \
  --task-uid "$TASK" --head "$HEAD_OID" --evidence-digest "$EVIDENCE_DIGEST" \
  --slice "$ROLE=$SLICE" --out "$BATCH" >"$TMPDIR/batch.out"
EPOCH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["epoch"])' "$TMPDIR/batch.out")"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" preflight \
  --batch "$BATCH" --out-dir "$TASK_ROOT/review-preflight" >"$TMPDIR/preflight.out"
LEDGER="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["ledger_path"])' "$TMPDIR/preflight.out")"
ARTIFACT="$TASK_ROOT/review-preflight/$SLICE.json"
VERIFY="$TASK_ROOT/verify.txt"
printf 'exact verification bytes\n' >"$VERIFY"
python3 - "$ARTIFACT" "$LEDGER" "$TASK" "$HEAD_OID" "$EPOCH" "$VERIFY" <<'PY'
import hashlib, json, pathlib, sys
artifact_path, ledger_path, task, head, epoch, verify = sys.argv[1:]
finding = {"id": "P1", "summary": "evidence-backed fixture",
           "triage": {"classification": "blocking", "basis": "fixture evidence"}}
artifact = json.loads(pathlib.Path(artifact_path).read_text())
artifact.update({"status":"completed", "disposition":"findings", "findings":[finding], "residual_risk":"fixture risk"})
pathlib.Path(artifact_path).write_text(json.dumps(artifact, sort_keys=True) + "\n")
artifact_digest = hashlib.sha256(pathlib.Path(artifact_path).read_bytes()).hexdigest()
row = json.loads(pathlib.Path(ledger_path).read_text().strip())
row["artifact_digest"] = artifact_digest
pathlib.Path(ledger_path).write_text(json.dumps(row, sort_keys=True) + "\n")
finding_digest = hashlib.sha256(json.dumps(finding, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
def d(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
evidence = pathlib.Path(verify).read_bytes()
entry = {
    "status":"completed", "index":0, "finding_digest":finding_digest,
    "disposition":"rejected_with_evidence", "evidence_kind":"repository_verification",
    "evidence_ref":str(pathlib.Path(verify).relative_to(pathlib.Path(artifact_path).parents[4])),
    "evidence_digest":hashlib.sha256(evidence).hexdigest(),
    "verification_result":{"status":"passed", "output_digest":"f"*64},
}
entry["entry_digest"] = d({k:v for k,v in entry.items() if k != "entry_digest"})
payload = {
    "schema":"oasis7-review-resolution/v1", "task_uid":task, "head":head, "epoch":epoch,
    "role_records":[{"role":"repository_health_engineer", "slice_id":"11111111-1111-4111-8111-111111111111", "findings_digest":d([finding]), "entries":[entry]}],
}
manifest_path = pathlib.Path(artifact_path).parents[1] / "review-resolutions" / f"{epoch}.json"
manifest_path.parent.mkdir()
manifest_path.write_text(json.dumps({**payload, "manifest_digest":d(payload)}, sort_keys=True) + "\n")
body_payload = {"marker":"oasis7-review-resolution", "schema":"oasis7-review-resolution/v1", "task_uid":task, "head":head, "epoch":epoch, "manifest_digest":d(payload)}
body = json.dumps(body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
readback = {**body_payload, "repository":"eng-cc/oasis7", "issue_number":3615, "comment_id":3934017999,
            "comment_url":"https://github.com/eng-cc/oasis7/issues/3615#issuecomment-3934017999", "author":"repo-admin",
            "created_at":"2026-09-06T10:00:00Z", "observed_at":"2026-09-06T10:01:00Z",
            "body_digest":hashlib.sha256(body.encode()).hexdigest()}
(manifest_path.parent / f"{epoch}.readback.json").write_text(json.dumps(readback, sort_keys=True) + "\n")
PY

# Write the complete immutable plan.
python3 - "$TASK_ROOT/review-plans/plan.json" "$TASK" "$HEAD_OID" "$BASE_OID" "$EVIDENCE_DIGEST" "$EPOCH" "$TASK_ROOT/review-batches/batch.json" "$LEDGER" <<'PY'
import json, sys
path, task, head, base, evidence, epoch, batch, ledger = sys.argv[1:]
json.dump({"schema":"oasis7-review-plan/v1", "task_uid":task, "frozen_head":head,
           "comparison_ref":"refs/heads/review-base", "comparison_oid":base,
           "relevant_evidence_digest":evidence, "roles":["repository_health_engineer"],
           "expected_slices":[{"role":"repository_health_engineer", "slice_id":"11111111-1111-4111-8111-111111111111"}],
           "epoch":epoch, "batch_path":batch,
           "preflight":{"status":"incomplete", "ledger_path":ledger}}, open(path,"w"), sort_keys=True)
PY

cat >"$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
open(os.environ["GH_LOG"], "a").write(" ".join(args) + "\n")
if args[0:2] != ["api", "repos/eng-cc/oasis7/issues/3615"] and args[0:2] != ["api", "repos/eng-cc/oasis7/issues/comments/3934017999"] and args[0:2] != ["api", "repos/eng-cc/oasis7/collaborators/repo-admin/permission"]:
    raise SystemExit("unexpected gh invocation: " + " ".join(args))
if args[1] == "repos/eng-cc/oasis7/issues/3615":
    print(json.dumps({"number":3615, "body":"<!-- oasis7-pm-task -->\ntask_uid: task_11111111111111111111111111111111\n"}))
elif "comments" in args[1]:
    print(json.dumps({"id":3934017999, "body":os.environ["BODY"], "issue_url":"https://api.github.com/repos/eng-cc/oasis7/issues/3615", "user":{"login":"repo-admin"}, "created_at":"2026-09-06T10:00:00Z"}))
else:
    print(json.dumps({"permission":"admin"}))
EOF
BODY="$(python3 - "$TASK_ROOT/review-resolutions/$EPOCH.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1])); print(json.dumps({"marker":"oasis7-review-resolution", "schema":"oasis7-review-resolution/v1", "task_uid":p["task_uid"], "head":p["head"], "epoch":p["epoch"], "manifest_digest":p["manifest_digest"]}, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
PY
)"
export BODY
chmod +x "$TMPDIR/bin/gh"

GH_LOG="$TMPDIR/gh.log"
export GH_LOG

COLLECTION="${BATCH%.json}.collection.json"
cp "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl"
RED_FAILURES=()

# A v1 manifest is compatible only after reconciliation. It must not authorize
# this plan-owned preflight transition; retain the exact original bytes if the
# frozen closeout incorrectly does so, then continue to the legacy positive.
if PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$TASK" --review-plan "$TASK_ROOT/review-plans/plan.json" --role-returns "$LEDGER" \
  --finding-resolution "$TASK_ROOT/review-resolutions/$EPOCH.json" --print-only \
  >"$TMPDIR/v1-preflight.out" 2>"$TMPDIR/v1-preflight.err"; then
  RED_FAILURES+=("v1 manifest authorized preflight promotion")
elif ! grep -Eqi 'v1|v2|preflight|handoff|promotion|resolution' "$TMPDIR/v1-preflight.err"; then
  cat "$TMPDIR/v1-preflight.err" >&2
  echo "v1 preflight rejection failed for an unrelated setup reason" >&2
  exit 1
fi
if ! cmp -s "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl" || [[ -e "$COLLECTION" ]]; then
  RED_FAILURES+=("v1 preflight attempt changed the original ledger or created a collection")
fi
cp "$TMPDIR/original-preflight-ledger.jsonl" "$LEDGER"
rm -f "$COLLECTION"

# Missing-manifest rejection remains a distinct finding-path control.
if PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$TASK" --review-plan "$TASK_ROOT/review-plans/plan.json" --role-returns "$LEDGER" \
  --print-only >"$TMPDIR/no-manifest.out" 2>"$TMPDIR/no-manifest.err"; then
  RED_FAILURES+=("finding-bearing preflight closeout accepted no resolution manifest")
fi
cmp -s "$LEDGER" "$TMPDIR/original-preflight-ledger.jsonl" || {
  echo "missing-manifest rejection mutated the preflight ledger" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "missing-manifest rejection created a collection receipt" >&2
  exit 1
}

# Seed the already-completed v1 compatibility fixture directly. Its persisted
# row shape is the completed-ledger contract, not the preflight skeleton and
# not the output of a reconcile invocation under test.
COMPLETED_LEDGER="$TASK_ROOT/review-preflight/completed-v1.jsonl"
LEGACY_PLAN="$TASK_ROOT/review-plans/completed-v1-plan.json"
python3 - "$LEDGER" "$ARTIFACT" "$COMPLETED_LEDGER" "$TASK_ROOT/review-plans/plan.json" "$LEGACY_PLAN" <<'PY'
import hashlib, json, pathlib, sys
preflight_path, artifact_path, completed_path, plan_path, legacy_plan_path = map(pathlib.Path, sys.argv[1:])
row = json.loads(preflight_path.read_text(encoding="utf-8").splitlines()[0])
artifact_bytes = artifact_path.read_bytes()
returned = json.loads(artifact_bytes)
row.update({
    "status": "completed", "activation": "message-assigned",
    "context_delivery": "minimal-task-packet",
    "actual_runtime": "inherited/unverified: human-operated",
    "scope_verdict": "approved", "risk_verdict": "approved",
    "findings": returned["disposition"], "residual_risk": returned["residual_risk"],
    "artifact_digest": hashlib.sha256(artifact_bytes).hexdigest(),
    "artifacts": [str(artifact_path.resolve())],
})
completed_path.write_text(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
legacy_plan = json.loads(plan_path.read_text(encoding="utf-8"))
legacy_plan["preflight"] = {"status": "completed", "ledger_path": str(completed_path.resolve())}
legacy_plan_path.write_text(json.dumps(legacy_plan, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
PY
cp "$COMPLETED_LEDGER" "$TMPDIR/completed-v1-ledger-before-closeout.jsonl"
COMPLETED_LEDGER_DIGEST="$(shasum -a 256 "$COMPLETED_LEDGER" | awk '{print $1}')"

# Observe actual helper subcommands without changing any copied repository
# file: this test-local Python launcher logs the target invocation and then
# delegates to the original interpreter and helper bytes unchanged.
REAL_PYTHON="$(command -v python3)"
REVIEW_BATCH_TARGET="$REPO/scripts/pm/review-batch-epoch.py"
REVIEW_BATCH_EVENT_LOG="$TMPDIR/review-batch-events.jsonl"
: >"$REVIEW_BATCH_EVENT_LOG"
export REAL_PYTHON REVIEW_BATCH_TARGET REVIEW_BATCH_EVENT_LOG
cat >"$TMPDIR/bin/python3" <<'PY'
#!/bin/bash
set -e
if [[ "${1:-}" == "$REVIEW_BATCH_TARGET" && -n "${REVIEW_BATCH_EVENT_LOG:-}" ]]; then
  "$REAL_PYTHON" - "$@" "$REVIEW_BATCH_EVENT_LOG" <<'PYLOG'
import json, sys
args = sys.argv[1:-1]
with open(sys.argv[-1], "a", encoding="utf-8") as handle:
    handle.write(json.dumps({"argv": args[1:]}, sort_keys=True) + "\n")
PYLOG
fi
exec "$REAL_PYTHON" "$@"
PY
chmod +x "$TMPDIR/bin/python3"
PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$TASK" --review-plan "$LEGACY_PLAN" --role-returns "$COMPLETED_LEDGER" \
  --finding-resolution "$TASK_ROOT/review-resolutions/$EPOCH.json" --print-only >"$TMPDIR/v1-completed-closeout.out"
grep -F 'Review Findings Disposition: addressed' "$TMPDIR/v1-completed-closeout.out" >/dev/null
if python3 - "$REVIEW_BATCH_EVENT_LOG" "$COLLECTION" "$COMPLETED_LEDGER_DIGEST" \
  >"$TMPDIR/v1-event-oracle.out" 2>"$TMPDIR/v1-event-oracle.err" <<'PY'
import json, pathlib, sys
events = [json.loads(line)["argv"] for line in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()]
commands = [next((name for name in ("collect", "reconcile") if name in argv), "unknown") for argv in events]
problems = []
if commands != ["collect"]:
    problems.append(f"expected exactly one collect and zero reconcile calls; observed {commands!r}")
try:
    collection = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    problems.append(f"collection receipt unavailable or malformed: {exc}")
else:
    if collection.get("status") != "passed" or collection.get("ledger_digest") != sys.argv[3]:
        problems.append("collection receipt did not bind the pre-call completed-ledger digest")
if problems:
    raise SystemExit("; ".join(problems))
print("completed-v1 closeout oracle: exactly one collect, zero reconcile; receipt binds pre-call ledger digest")
PY
then
  :
else
  RED_FAILURES+=("completed-v1 closeout violated the collect-only/no-reconcile oracle")
  cat "$TMPDIR/v1-event-oracle.err" >&2
  printf 'COMPLETED-V1 closeout output:\n' >&2
  cat "$TMPDIR/v1-completed-closeout.out" >&2
  printf 'Observed batch-helper argv:\n' >&2
  cat "$REVIEW_BATCH_EVENT_LOG" >&2
fi
if ! cmp -s "$COMPLETED_LEDGER" "$TMPDIR/completed-v1-ledger-before-closeout.jsonl"; then
  RED_FAILURES+=("completed-v1 closeout rewrote the already-completed ledger")
fi

PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid "$TASK" --review-plan "$LEGACY_PLAN" \
  --slice-ledger "$COMPLETED_LEDGER" \
  --finding-resolution "$TASK_ROOT/review-resolutions/$EPOCH.json" \
  --review-evidence 'repository_health_engineer: findings; exact fixture' \
  --review-verdicts 'repository_health_engineer scope=approved risk=accepted' \
  --finding-disposition addressed --finding-disposition-evidence fixture \
  --verification fixture --residual-risk fixture --issue 3615 --repo eng-cc/oasis7 --print-only >"$TMPDIR/recorder.out"
grep -F 'Review Findings Disposition: addressed' "$TMPDIR/recorder.out" >/dev/null
cmp -s "$COMPLETED_LEDGER" "$TMPDIR/completed-v1-ledger-before-closeout.jsonl" || {
  echo "direct recorder rewrote the completed-v1 ledger" >&2
  exit 1
}

if PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid "$TASK" --review-plan "$LEGACY_PLAN" --slice-ledger "$COMPLETED_LEDGER" \
  --finding-resolution "$TASK_ROOT/review-resolutions/$EPOCH.json" \
  --review-evidence 'repository_health_engineer: findings; exact fixture' \
  --review-verdicts 'repository_health_engineer scope=approved risk=accepted' \
  --finding-disposition addressed --finding-disposition-evidence fixture \
  --verification fixture --residual-risk fixture --issue 3615 --repo evil/repo >"$TMPDIR/evil.out" 2>"$TMPDIR/evil.err"; then
  echo "direct recorder unexpectedly accepted caller repository" >&2
  exit 1
fi
if grep -F 'issue comment' "$TMPDIR/gh.log" >/dev/null; then
  echo "direct recorder attempted GitHub issue comment after repository mismatch" >&2
  exit 1
fi

if [[ ${#RED_FAILURES[@]} -gt 0 ]]; then
  printf 'EXPECTED RED: %s\n' "${RED_FAILURES[@]}" >&2
  exit 1
fi

echo "review-findings-resolution.integration.test: OK"
