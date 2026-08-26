#!/usr/bin/env bash
# Cross-platform contract: the review chain has one executable facade with a
# canonical operator interface and fail-closed immutable-plan behavior.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FACADE="$ROOT_DIR/scripts/pm/review-closeout.sh"

if [[ ! -x "$FACADE" ]]; then
  echo "review-closeout facade must be an executable: $FACADE" >&2
  exit 1
fi
HELP="$($FACADE --help)"
for required in '--task-uid' '--review-plan' '--role-returns' '--print-only'; do
  grep -F -- "$required" <<<"$HELP" >/dev/null || {
    echo "review-closeout --help missing canonical option: $required" >&2
    exit 1
  }
done
SOURCE="$(cat "$FACADE")"
for helper in 'review-batch-epoch.py' 'reconcile' 'collect' 'record-pre-pr-review.sh'; do
  grep -F -- "$helper" <<<"$SOURCE" >/dev/null || {
    echo "review-closeout facade does not delegate canonical helper: $helper" >&2
    exit 1
  }
done
for ordering_contract in 'planned_roles=plan.get("roles")' 'rows=[by_role[role] for role in planned_roles]'; do
  grep -F -- "$ordering_contract" <<<"$SOURCE" >/dev/null || {
    echo "review-closeout must restore immutable review-plan role order after batch reconciliation" >&2
    exit 1
  }
done

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
REPO="$TMPDIR/repo"
UID_VALUE="task_11111111111111111111111111111111"
ROLE="repository_health_engineer"
SLICE="11111111-1111-4111-8111-111111111111"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync"
for helper in review-closeout.sh review-batch-epoch.py record-pre-pr-review.sh validate-review-provenance.py; do
  cp "$ROOT_DIR/scripts/pm/$helper" "$REPO/scripts/pm/$helper"
done
chmod +x "$REPO/scripts/pm/review-closeout.sh" "$REPO/scripts/pm/record-pre-pr-review.sh"
printf 'scratch/\n' >"$REPO/.pm/.gitignore"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"project":{"repo":"eng-cc/oasis7"},"tasks":{"$UID_VALUE":{"issue_number":3379}}}
EOF

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

TASK_ROOT="$REPO/.pm/scratch/$UID_VALUE"
BATCH="$TASK_ROOT/review-batches/epoch.json"
PREFLIGHT_DIR="$TASK_ROOT/review-preflight"
mkdir -p "$(dirname "$BATCH")"
EVIDENCE_DIGEST="$(printf '%s' review-evidence | shasum -a 256 | awk '{print $1}')"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" create \
  --task-uid "$UID_VALUE" --head "$HEAD_OID" --evidence-digest "$EVIDENCE_DIGEST" \
  --slice "$ROLE=$SLICE" --out "$BATCH" >"$TMPDIR/batch.json"
python3 "$REPO/scripts/pm/review-batch-epoch.py" --root "$REPO" preflight \
  --batch "$BATCH" --out-dir "$PREFLIGHT_DIR" >"$TMPDIR/preflight.json"
LEDGER="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["ledger_path"])' "$TMPDIR/preflight.json")"
EPOCH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["epoch"])' "$BATCH")"
ARTIFACT="$PREFLIGHT_DIR/$SLICE.json"
python3 - "$ARTIFACT" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload.update({"status": "completed", "disposition": "no_findings", "findings": [], "residual_risk": "fixture residual risk"})
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
PLAN="$TASK_ROOT/review-plans/fixture.json"
mkdir -p "$(dirname "$PLAN")"
python3 - "$PLAN" "$BATCH" "$HEAD_OID" "$BASE_OID" "$EVIDENCE_DIGEST" "$EPOCH" <<'PY'
import json, sys
plan, batch, head, comparison, evidence, epoch = sys.argv[1:]
payload = {
    "schema": "oasis7-review-plan/v1", "task_uid": "task_11111111111111111111111111111111",
    "frozen_head": head, "comparison_ref": "refs/heads/review-base", "comparison_oid": comparison,
    "relevant_evidence_digest": evidence, "roles": ["repository_health_engineer"],
    "expected_slices": [{"role": "repository_health_engineer", "slice_id": "11111111-1111-4111-8111-111111111111"}],
    "epoch": epoch, "batch_path": batch, "preflight": {"status": "incomplete", "ledger_path": ""},
}
with open(plan, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY

VALID_OUT="$TMPDIR/valid.out"
VALID_ERR="$TMPDIR/valid.err"
if ! (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$VALID_OUT" 2>"$VALID_ERR"); then
  cat "$VALID_ERR" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$VALID_OUT" >/dev/null
grep -F 'Review Plan: .pm/scratch/' "$VALID_OUT" >/dev/null
[[ ! -s "$VALID_ERR" ]] || { cat "$VALID_ERR" >&2; exit 1; }
cp "$LEDGER" "$TMPDIR/complete-ledger.jsonl"

# An empty role-return ledger must be rejected before packet generation.
: >"$LEDGER"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/missing-role.out" 2>"$TMPDIR/missing-role.err"); then
  echo "review-closeout accepted a missing role return" >&2
  exit 1
fi
grep -Eiq 'identity mismatch|missing expected|ledger digest|role' "$TMPDIR/missing-role.err"

# A changed working HEAD must invalidate the immutable review plan.
cp "$TMPDIR/complete-ledger.jsonl" "$LEDGER"
COLLECTION="${BATCH%.json}.collection.json"
test -f "$COLLECTION"
cp "$LEDGER" "$TMPDIR/stale-ledger-before.jsonl"
cp "$COLLECTION" "$TMPDIR/stale-collection-before.json"
git -C "$REPO" commit --allow-empty -qm stale-head
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/stale-head.out" 2>"$TMPDIR/stale-head.err"); then
  echo "review-closeout accepted a stale frozen review head" >&2
  exit 1
fi
grep -Eiq 'source head|frozen HEAD|head mismatch' "$TMPDIR/stale-head.err"
cmp "$TMPDIR/stale-ledger-before.jsonl" "$LEDGER"
cmp "$TMPDIR/stale-collection-before.json" "$COLLECTION"

echo "review-closeout-facade.test: OK"
