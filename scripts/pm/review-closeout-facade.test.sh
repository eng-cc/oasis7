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
for helper in review-closeout.sh review-batch-epoch.py record-pre-pr-review.sh validate-review-provenance.py review-findings-resolution.py; do
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
python3 - "$PLAN" "$BATCH" "$HEAD_OID" "$BASE_OID" "$EVIDENCE_DIGEST" "$EPOCH" "$LEDGER" <<'PY'
import json, sys
plan, batch, head, comparison, evidence, epoch, ledger = sys.argv[1:]
payload = {
    "schema": "oasis7-review-plan/v1", "task_uid": "task_11111111111111111111111111111111",
    "frozen_head": head, "comparison_ref": "refs/heads/review-base", "comparison_oid": comparison,
    "relevant_evidence_digest": evidence, "roles": ["repository_health_engineer"],
    "expected_slices": [{"role": "repository_health_engineer", "slice_id": "11111111-1111-4111-8111-111111111111"}],
    "epoch": epoch, "batch_path": batch, "preflight": {"status": "incomplete", "ledger_path": ledger},
}
with open(plan, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY

# A malformed plan must fail before reconcile can rewrite the ledger or publish
# a collection receipt.
cp "$PLAN" "$TMPDIR/valid-plan.json"
cp "$LEDGER" "$TMPDIR/pre-malformed-ledger.jsonl"
python3 - "$PLAN" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload["roles"].append(payload["roles"][0])
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/malformed-plan.out" 2>"$TMPDIR/malformed-plan.err"); then
  echo "review-closeout accepted duplicate review-plan roles" >&2
  exit 1
fi
grep -F 'roles must be unique' "$TMPDIR/malformed-plan.err" >/dev/null
cmp -s "$LEDGER" "$TMPDIR/pre-malformed-ledger.jsonl" || {
  echo "malformed review plan mutated the preflight ledger" >&2
  exit 1
}
[[ ! -e "${BATCH%.json}.collection.json" ]] || {
  echo "malformed review plan published a collection receipt" >&2
  exit 1
}
cp "$TMPDIR/valid-plan.json" "$PLAN"

# Repository-owned artifacts are required before reconcile/collection. An
# absolute artifact outside the repository must fail without rewriting the
# preflight ledger or creating a collection receipt.
OUTSIDE_ARTIFACT="$TMPDIR/outside-review-return.json"
cp "$ARTIFACT" "$OUTSIDE_ARTIFACT"
cp "$LEDGER" "$TMPDIR/original-outside-ledger.jsonl"
python3 - "$LEDGER" "$OUTSIDE_ARTIFACT" <<'PY'
import hashlib, json, sys
ledger_path, outside = sys.argv[1:]
row = json.loads(open(ledger_path, encoding="utf-8").readline())
row["artifacts"] = [outside]
row["artifact_digest"] = hashlib.sha256(open(outside, "rb").read()).hexdigest()
open(ledger_path, "w", encoding="utf-8").write(json.dumps(row, sort_keys=True) + "\n")
PY
cp "$LEDGER" "$TMPDIR/pre-outside-ledger.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/outside-artifact.out" 2>"$TMPDIR/outside-artifact.err"); then
  echo "review-closeout accepted an artifact outside the repository root" >&2
  exit 1
fi
grep -Eqi 'escapes|repository root|artifact' "$TMPDIR/outside-artifact.err"
cmp -s "$LEDGER" "$TMPDIR/pre-outside-ledger.jsonl" || {
  echo "outside-root artifact mutated the preflight ledger" >&2
  exit 1
}
[[ ! -e "${BATCH%.json}.collection.json" ]] || {
  echo "outside-root artifact published a collection receipt" >&2
  exit 1
}
cp "$TMPDIR/original-outside-ledger.jsonl" "$LEDGER"

COLLECTION="${BATCH%.json}.collection.json"

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

# The immutable plan owns the preflight ledger identity. A caller-provided
# repository-owned alternate ledger must fail before collection or packet
# publication, even when it has matching task/head/slice contents.
ALTERNATE_LEDGER="$TASK_ROOT/alternate-slice-ledger.jsonl"
cp "$LEDGER" "$ALTERNATE_LEDGER"
cp "$LEDGER" "$TMPDIR/canonical-before-alternate.jsonl"
cp "$ALTERNATE_LEDGER" "$TMPDIR/alternate-before.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$ALTERNATE_LEDGER" --print-only \
  >"$TMPDIR/alternate-ledger.out" 2>"$TMPDIR/alternate-ledger.err"); then
  echo "review-closeout accepted a role-return ledger outside the plan preflight path" >&2
  exit 1
fi
grep -Eqi 'preflight ledger|immutable|role-return ledger|ledger path' "$TMPDIR/alternate-ledger.err"
cmp -s "$LEDGER" "$TMPDIR/canonical-before-alternate.jsonl" || {
  echo "alternate ledger changed the canonical preflight ledger" >&2
  exit 1
}
cmp -s "$ALTERNATE_LEDGER" "$TMPDIR/alternate-before.jsonl" || {
  echo "alternate ledger was mutated before rejection" >&2
  exit 1
}
test -f "$COLLECTION"

# Unresolved role findings must fail closed before collection or packet
# publication. Keep the earlier no-findings success above as the positive
# control for the same facade.
rm -f "$COLLECTION"
python3 - "$ARTIFACT" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload.update({"disposition": "findings", "findings": [{"id": "FIX1-UNRESOLVED", "summary": "fixture unresolved finding"}]})
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/unresolved-findings.out" 2>"$TMPDIR/unresolved-findings.err"); then
  echo "review-closeout accepted unresolved role findings" >&2
  exit 1
fi
grep -Eiq 'unresolved|findings|blocked' "$TMPDIR/unresolved-findings.err"
[[ ! -s "$TMPDIR/unresolved-findings.out" ]] || {
  echo "unresolved findings produced a review packet" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "unresolved findings published a collection receipt" >&2
  exit 1
}

# A finding artifact may be ledger-parent-relative. The facade must resolve it
# before reconcile; otherwise the no-resolution scan misses it and reconcile
# mutates the preflight ledger before packet publication is rejected.
rm -f "$COLLECTION"
python3 - "$ARTIFACT" "$LEDGER" <<'PY'
import hashlib, json, sys
from pathlib import Path
artifact_path, ledger_path = sys.argv[1:]
artifact = json.load(open(artifact_path, encoding="utf-8"))
artifact.update({"disposition": "findings", "findings": [{"id": "FIX7-RELATIVE", "summary": "relative unresolved finding"}]})
with open(artifact_path, "w", encoding="utf-8") as handle:
    json.dump(artifact, handle, sort_keys=True)
    handle.write("\n")
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
for row in rows:
    row["findings"] = "findings"
    row["artifact_digest"] = digest
    row["artifacts"] = [Path(artifact_path).name]
with open(ledger_path, "w", encoding="utf-8") as handle:
    handle.write("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
PY
cp "$LEDGER" "$TMPDIR/relative-finding-before.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/relative-finding.out" 2>"$TMPDIR/relative-finding.err"); then
  echo "review-closeout accepted an unresolved ledger-parent-relative finding" >&2
  exit 1
fi
grep -Eqi 'unresolved|findings|blocked' "$TMPDIR/relative-finding.err"
cmp -s "$LEDGER" "$TMPDIR/relative-finding-before.jsonl" || {
  echo "ledger-parent-relative finding mutated the ledger before rejection" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "ledger-parent-relative finding published a collection receipt" >&2
  exit 1
}

# An explicit ledger finding disposition must agree with the returned artifact
# before reconcile. Otherwise reconcile downgrades the ledger to no_findings,
# allowing collection and packet publication to proceed.
rm -f "$COLLECTION"
python3 - "$ARTIFACT" "$LEDGER" <<'PY'
import hashlib, json, sys
artifact_path, ledger_path = sys.argv[1:]
artifact = json.load(open(artifact_path, encoding="utf-8"))
artifact.update({"disposition": "no_findings", "findings": []})
with open(artifact_path, "w", encoding="utf-8") as handle:
    json.dump(artifact, handle, sort_keys=True)
    handle.write("\n")
rows = [json.loads(line) for line in open(ledger_path, encoding="utf-8") if line.strip()]
digest = hashlib.sha256(open(artifact_path, "rb").read()).hexdigest()
for row in rows:
    row["findings"] = "findings"
    row["artifact_digest"] = digest
with open(ledger_path, "w", encoding="utf-8") as handle:
    handle.write("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
PY
cp "$LEDGER" "$TMPDIR/ledger-artifact-mismatch-before.jsonl"
if (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/ledger-artifact-mismatch.out" 2>"$TMPDIR/ledger-artifact-mismatch.err"); then
  echo "review-closeout accepted a ledger finding/artifact no-findings mismatch" >&2
  exit 1
fi
grep -Eiq 'mismatch|findings|disposition' "$TMPDIR/ledger-artifact-mismatch.err"
cmp -s "$LEDGER" "$TMPDIR/ledger-artifact-mismatch-before.jsonl" || {
  echo "ledger/artifact mismatch mutated the ledger before rejection" >&2
  exit 1
}
[[ ! -e "$COLLECTION" ]] || {
  echo "ledger/artifact mismatch published a collection receipt" >&2
  exit 1
}

# Restore the no-findings fixture so the remaining immutable-plan checks keep
# their original positive-control collection.
python3 - "$ARTIFACT" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload.update({"disposition": "no_findings", "findings": []})
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
cp "$TMPDIR/complete-ledger.jsonl" "$LEDGER"
if ! (cd "$TMPDIR" && "$REPO/scripts/pm/review-closeout.sh" \
  --task-uid "$UID_VALUE" --review-plan "$PLAN" --role-returns "$LEDGER" --print-only \
  >"$TMPDIR/restored-valid.out" 2>"$TMPDIR/restored-valid.err"); then
  cat "$TMPDIR/restored-valid.err" >&2
  exit 1
fi
grep -F 'Pre-PR Local Role Review: passed' "$TMPDIR/restored-valid.out" >/dev/null

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
