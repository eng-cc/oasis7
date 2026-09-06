#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync" "$TMPDIR/bin"
for helper in review-closeout.sh review-batch-epoch.py record-pre-pr-review.sh validate-review-provenance.py review-findings-resolution.py; do
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
mkdir -p "$TASK_ROOT/review-batches" "$TASK_ROOT/review-plans" "$TASK_ROOT/review-preflight"
EVIDENCE_DIGEST="$(printf '%s' review-evidence | shasum -a 256 | awk '{print $1}')"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" create \
  --task-uid "$TASK" --head "$HEAD_OID" --evidence-digest "$EVIDENCE_DIGEST" \
  --slice "$ROLE=$SLICE" --out "$TASK_ROOT/review-batches/batch.json" >"$TMPDIR/batch.out"
EPOCH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["epoch"])' "$TMPDIR/batch.out")"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" preflight \
  --batch "$TASK_ROOT/review-batches/batch.json" --out-dir "$TASK_ROOT/review-preflight" >"$TMPDIR/preflight.out"
LEDGER="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["ledger_path"])' "$TMPDIR/preflight.out")"
ARTIFACT="$TASK_ROOT/review-preflight/$SLICE.json"
VERIFY="$TASK_ROOT/verify.txt"
printf 'exact verification bytes\n' >"$VERIFY"
python3 - "$ARTIFACT" "$LEDGER" "$TASK" "$HEAD_OID" "$EPOCH" "$VERIFY" <<'PY'
import hashlib, json, pathlib, sys
artifact_path, ledger_path, task, head, epoch, verify = sys.argv[1:]
finding = {"id": "P1", "summary": "evidence-backed fixture"}
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
if args[0:2] != ["api", "repos/eng-cc/oasis7/issues/3615"] and args[0:2] != ["api", "repos/eng-cc/oasis7/issues/3615/comments/3934017999"] and args[0:2] != ["api", "repos/eng-cc/oasis7/collaborators/repo-admin/permission"]:
    raise SystemExit("unexpected gh invocation: " + " ".join(args))
if args[1] == "repos/eng-cc/oasis7/issues/3615":
    print(json.dumps({"number":3615, "body":"<!-- oasis7-pm-task -->\ntask_uid: task_11111111111111111111111111111111\n"}))
elif "comments" in args[1]:
    print(json.dumps({"id":3934017999, "body":os.environ["BODY"], "user":{"login":"repo-admin"}, "created_at":"2026-09-06T10:00:00Z"}))
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

PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$TASK" --review-plan "$TASK_ROOT/review-plans/plan.json" --role-returns "$LEDGER" \
  --finding-resolution "$TASK_ROOT/review-resolutions/$EPOCH.json" --print-only >"$TMPDIR/closeout.out"
grep -F 'Review Findings Disposition: addressed' "$TMPDIR/closeout.out" >/dev/null

PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid "$TASK" --review-plan "$TASK_ROOT/review-plans/plan.json" \
  --finding-resolution "$TASK_ROOT/review-resolutions/$EPOCH.json" \
  --review-evidence 'repository_health_engineer: findings; exact fixture' \
  --review-verdicts 'repository_health_engineer scope=approved risk=accepted' \
  --finding-disposition addressed --finding-disposition-evidence fixture \
  --verification fixture --residual-risk fixture --issue 3615 --repo eng-cc/oasis7 --print-only >"$TMPDIR/recorder.out"
grep -F 'Review Findings Disposition: addressed' "$TMPDIR/recorder.out" >/dev/null

if PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid "$TASK" --review-plan "$TASK_ROOT/review-plans/plan.json" \
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

echo "review-findings-resolution.integration.test: OK"
