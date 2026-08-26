#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage: ./scripts/pm/review-closeout.sh --task-uid <uid> --review-plan <json> --role-returns <ledger.jsonl> [options]

Reconcile structured role returns for one immutable review plan, collect the
role-complete ledger, and generate the canonical pre-PR review packet.

Options:
  --task-uid <uid>          Bound GitHub-backed task UID
  --review-plan <json>      Immutable oasis7 review plan
  --role-returns <jsonl>    Preflight slice ledger whose artifacts contain completed returns
  --verification <text>     Verification matrix summary (default: derived from immutable plan)
  --residual-risk <text>    Optional additional residual-risk context
  --print-only              Print the packet instead of posting it to the task issue
  --json                    Print a machine-readable facade result
  -h, --help                Show this help
EOF
}

die() { echo "review-closeout: $*" >&2; exit 1; }
resolve_repo_file() {
  local label="$1"
  local raw="$2"
  python3 - "$ROOT_DIR" "$raw" "$label" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve()
candidate = Path(sys.argv[2]).expanduser()
if not candidate.is_absolute():
    candidate = root / candidate
try:
    resolved = candidate.resolve(strict=True)
except OSError as exc:
    raise SystemExit(f"review-closeout: {sys.argv[3]} cannot be resolved: {exc}")
try:
    resolved.relative_to(root)
except ValueError:
    raise SystemExit(f"review-closeout: {sys.argv[3]} escapes repository root: {sys.argv[2]}")
if not resolved.is_file():
    raise SystemExit(f"review-closeout: {sys.argv[3]} is not a file: {sys.argv[2]}")
print(resolved)
PY
}
TASK_UID="" REVIEW_PLAN="" ROLE_RETURNS="" VERIFICATION="" EXTRA_RISK="" PRINT_ONLY=0 OUTPUT_JSON=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --review-plan) REVIEW_PLAN="${2:-}"; shift 2 ;;
    --role-returns) ROLE_RETURNS="${2:-}"; shift 2 ;;
    --verification) VERIFICATION="${2:-}"; shift 2 ;;
    --residual-risk) EXTRA_RISK="${2:-}"; shift 2 ;;
    --print-only) PRINT_ONLY=1; shift ;;
    --json) OUTPUT_JSON=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done
[[ -n "$TASK_UID" ]] || die "--task-uid is required"
REVIEW_PLAN="$(resolve_repo_file "review plan" "$REVIEW_PLAN")" || exit 1
ROLE_RETURNS="$(resolve_repo_file "role-return ledger" "$ROLE_RETURNS")" || exit 1

PLAN_FIELDS="$(python3 - "$ROOT_DIR" "$REVIEW_PLAN" "$TASK_UID" <<'PY'
import json, pathlib, sys
root=pathlib.Path(sys.argv[1]).resolve(); plan_path=pathlib.Path(sys.argv[2]).resolve()
try: plan_path.relative_to(root)
except ValueError: raise SystemExit("review-closeout: review plan escapes repository root")
p=json.loads(plan_path.read_text())
if p.get("schema")!="oasis7-review-plan/v1" or p.get("task_uid")!=sys.argv[3]:
 raise SystemExit("review-closeout: review plan identity mismatch")
for key in ("batch_path","frozen_head","comparison_ref","comparison_oid","epoch","relevant_evidence_digest"):
 if not p.get(key): raise SystemExit(f"review-closeout: review plan is missing {key}")
print(p["batch_path"]); print(p["frozen_head"]); print(p["comparison_ref"]); print(p["comparison_oid"]); print(p["epoch"]); print(p["relevant_evidence_digest"])
PY
)"
BATCH_PATH="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '1p')"
BATCH_PATH="$(resolve_repo_file "review batch" "$BATCH_PATH")" || exit 1
FROZEN_HEAD="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '2p')"
PLAN_EPOCH="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '5p')"
CURRENT_HEAD="$(git -C "$ROOT_DIR" rev-parse HEAD)" || die "cannot resolve current HEAD"
[[ "$CURRENT_HEAD" == "$FROZEN_HEAD" ]] || die "review-closeout: frozen HEAD mismatch: expected $FROZEN_HEAD, actual $CURRENT_HEAD"

# Validate the immutable plan/batch/collection identity before reconcile or
# collect can rewrite the role ledger or publish a collection receipt.
COLLECTION_STATE="$(python3 - "$ROOT_DIR" "$BATCH_PATH" "$ROLE_RETURNS" "$TASK_UID" "$FROZEN_HEAD" "$PLAN_EPOCH" "$REVIEW_PLAN" <<'PY'
import hashlib, json, pathlib, sys

root = pathlib.Path(sys.argv[1]).resolve()
batch_path = pathlib.Path(sys.argv[2]).resolve()
ledger_path = pathlib.Path(sys.argv[3]).resolve()
task_uid, frozen_head, plan_epoch = sys.argv[4:7]
plan_path = pathlib.Path(sys.argv[7]).resolve()
for label, path in (("review batch", batch_path), ("role-return ledger", ledger_path), ("review plan", plan_path)):
    try:
        path.relative_to(root)
    except ValueError:
        raise SystemExit(f"review-closeout: {label} escapes repository root")
try:
    batch = json.loads(batch_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    raise SystemExit(f"review-closeout: review batch is not readable: {exc}")
if batch.get("schema") != "oasis7-review-batch/v1":
    raise SystemExit("review-closeout: review batch schema is invalid")
for key, expected in (("task_uid", task_uid), ("frozen_head", frozen_head), ("epoch", plan_epoch)):
    if batch.get(key) != expected:
        raise SystemExit(f"review-closeout: review batch {key} mismatch")
try:
    plan = json.loads(plan_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    raise SystemExit(f"review-closeout: review plan is not readable: {exc}")
roles = plan.get("roles")
plan_slices = plan.get("expected_slices")
batch_slices = batch.get("expected_slices")
if (not isinstance(roles, list) or not roles or
        not all(isinstance(role, str) and role for role in roles) or len(set(roles)) != len(roles)):
    raise SystemExit("review-closeout: review plan roles must be unique non-empty strings")
if not isinstance(plan_slices, list) or not isinstance(batch_slices, list):
    raise SystemExit("review-closeout: review plan/batch expected_slices are invalid")
def slice_identities(items):
    identities = []
    for item in items:
        if not isinstance(item, dict) or not isinstance(item.get("role"), str) or not isinstance(item.get("slice_id"), str):
            raise SystemExit("review-closeout: review plan/batch slice identity is invalid")
        identities.append((item["role"], item["slice_id"]))
    return identities
plan_identities = slice_identities(plan_slices)
batch_identities = slice_identities(batch_slices)
if [role for role, _ in plan_identities] != roles or len(set(plan_identities)) != len(plan_identities):
    raise SystemExit("review-closeout: review plan roles/expected_slices mismatch")
if len(plan_identities) != len(batch_identities) or set(plan_identities) != set(batch_identities):
    raise SystemExit("review-closeout: review plan/batch slice set mismatch")
collection_path = batch_path.with_name(f"{batch_path.stem}.collection.json")
if collection_path.exists():
    try:
        collection = json.loads(collection_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"review-closeout: existing collection is not readable: {exc}")
    if collection.get("schema") != "oasis7-review-collection/v1" or collection.get("status") != "passed":
        raise SystemExit("review-closeout: existing collection receipt is invalid")
    for key, expected in (("task_uid", task_uid), ("frozen_head", frozen_head), ("epoch", plan_epoch)):
        if collection.get(key) != expected:
            raise SystemExit(f"review-closeout: existing collection {key} mismatch")
    ledger_digest = hashlib.sha256(ledger_path.read_bytes()).hexdigest()
    if collection.get("ledger_digest") != ledger_digest:
        raise SystemExit("review-closeout: existing collection ledger digest mismatch")
    print("existing")
else:
    print("absent")
PY
)" || exit 1

if [[ "$COLLECTION_STATE" == "absent" ]]; then
  python3 "$SCRIPT_DIR/review-batch-epoch.py" --root "$ROOT_DIR" reconcile \
    --batch "$BATCH_PATH" --ledger "$ROLE_RETURNS" >/dev/null
fi
python3 "$SCRIPT_DIR/review-batch-epoch.py" --root "$ROOT_DIR" collect \
  --batch "$BATCH_PATH" --ledger "$ROLE_RETURNS" >/dev/null

SUMMARIES="$(python3 - "$ROLE_RETURNS" "$EXTRA_RISK" "$REVIEW_PLAN" <<'PY'
import json, pathlib, sys
rows=[json.loads(x) for x in pathlib.Path(sys.argv[1]).read_text().splitlines() if x.strip()]
if not rows: raise SystemExit("review-closeout: role-return ledger is empty")
plan=json.loads(pathlib.Path(sys.argv[3]).read_text())
planned_roles=plan.get("roles")
if not isinstance(planned_roles, list) or not all(isinstance(role, str) for role in planned_roles):
 raise SystemExit("review-closeout: review plan roles are invalid")
by_role={r.get("role"): r for r in rows}
if len(by_role) != len(rows) or set(by_role) != set(planned_roles):
 raise SystemExit("review-closeout: role-return ledger roles mismatch review plan")
rows=[by_role[role] for role in planned_roles]
roles=",".join(str(r["role"]) for r in rows)
evidence="; ".join(f'{r["role"]}: {r["findings"]}' for r in rows)
verdicts="; ".join(f'{r["role"]} scope={r["scope_verdict"]} risk={r["risk_verdict"]}' for r in rows)
disposition="; ".join(f'{r["role"]}: {r["findings"]}' for r in rows)
risks=[f'{r["role"]}: {r["residual_risk"]}' for r in rows]
if sys.argv[2]: risks.append(sys.argv[2])
print(roles); print(evidence); print(verdicts); print(disposition); print("; ".join(risks))
PY
)"
[[ -n "$VERIFICATION" ]] || VERIFICATION="immutable review plan evidence digest $(printf '%s\n' "$PLAN_FIELDS" | sed -n '6p')"
ARGS=(--task-uid "$TASK_UID" --review-plan "$REVIEW_PLAN" --roles "$(printf '%s\n' "$SUMMARIES" | sed -n '1p')"
  --review-evidence "$(printf '%s\n' "$SUMMARIES" | sed -n '2p')" --review-verdicts "$(printf '%s\n' "$SUMMARIES" | sed -n '3p')"
  --finding-disposition-evidence "$(printf '%s\n' "$SUMMARIES" | sed -n '4p')" --verification "$VERIFICATION"
  --residual-risk "$(printf '%s\n' "$SUMMARIES" | sed -n '5p')" --slice-ledger "$ROLE_RETURNS")
[[ "$PRINT_ONLY" == 0 ]] || ARGS+=(--print-only)
PACKET="$("$SCRIPT_DIR/record-pre-pr-review.sh" "${ARGS[@]}")"
if [[ "$OUTPUT_JSON" == 1 ]]; then
  python3 - "$TASK_UID" "$REVIEW_PLAN" "$ROLE_RETURNS" "$PACKET" <<'PY'
import json,sys
print(json.dumps({"status":"passed","task_uid":sys.argv[1],"review_plan":sys.argv[2],"role_returns":sys.argv[3],"packet":sys.argv[4]}))
PY
else
  printf '%s\n' "$PACKET"
fi
