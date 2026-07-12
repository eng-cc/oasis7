#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/task-closeout.sh --role <role> --task-uid <task_uid> [options]

Record GitHub Project-backed workflow closeout without recreating repo-local
.pm/tasks files. By default this marks the branch ready for PR, not final done.

Options:
  --role <role>           Owner role for workflow close evidence
  --task-uid <task_uid>   Task to close
  --to-status <status>    Target task status: ready, done, or deferred (default: ready)
  --verification-profile <name> Repository-owned named verification profile
  --claim-type <type>     Claim type (default: ready_for_pr for ready, task_complete for done)
  --comparison-ref <ref>  Immutable diff base; should match the review packet Comparison Ref
  --review-packet-file <path> Passed review packet bound to frozen HEAD (required for ready)
  --pr-receipt <path>    Trusted merged-PR receipt (required for PR-backed done)
  --no-lint               Accepted for compatibility; legacy PM lint is not run
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help

Recovery after partial remote closeout:
  ./scripts/pm/refresh-task-cache.sh --task-uid <task_uid> --json
  ./scripts/pm/github-project-workflow.sh --json audit --task-uid <task_uid>
  # then retry task-closeout with the same frozen source/comparison ref
USAGE
}

die() {
  echo "task-closeout: $*" >&2
  exit 1
}

ROLE=""
TASK_UID=""
TARGET_STATUS="ready"
VERIFY_COMMAND=""
VERIFICATION_PROFILE=""
CLAIM_TYPE=""
COMPARISON_REF=""
OUTPUT_JSON=0
REVIEW_PACKET_FILE=""
PR_MERGE_RECEIPT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --role)
      ROLE="${2:-}"
      shift 2
      ;;
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
      ;;
    --to-status)
      TARGET_STATUS="${2:-}"
      shift 2
      ;;
    --verify-command)
      VERIFY_COMMAND="${2:-}"
      shift 2
      ;;
    --verification-profile) VERIFICATION_PROFILE="${2:-}"; shift 2 ;;
    --claim-type)
      CLAIM_TYPE="${2:-}"
      shift 2
      ;;
    --comparison-ref)
      COMPARISON_REF="${2:-}"
      shift 2
      ;;
    --review-packet-file) REVIEW_PACKET_FILE="${2:-}"; shift 2 ;;
    --pr-receipt) PR_MERGE_RECEIPT="${2:-}"; shift 2 ;;
    --no-lint)
      shift
      ;;
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "$ROLE" ]] || die "--role is required"
[[ -n "$TASK_UID" ]] || die "--task-uid is required"
[[ "$TARGET_STATUS" == "ready" || "$TARGET_STATUS" == "done" || "$TARGET_STATUS" == "deferred" ]] || die "--to-status must be ready, done, or deferred"
if [[ -z "$CLAIM_TYPE" ]]; then
  if [[ "$TARGET_STATUS" == "done" ]]; then
    CLAIM_TYPE="task_complete"
  else
    CLAIM_TYPE="ready_for_pr"
  fi
fi
if [[ "$TARGET_STATUS" != "deferred" && -z "$VERIFICATION_PROFILE" ]]; then
  die "--verification-profile is required when --to-status is ready or done"
fi
if [[ -n "$VERIFY_COMMAND" ]]; then
  die "--verify-command is not accepted for lifecycle transitions; select a repository-owned --verification-profile"
fi
if [[ "$TARGET_STATUS" == "done" && "$CLAIM_TYPE" != "task_complete" ]]; then
  die "--claim-type must be task_complete when --to-status is done"
fi
if [[ "$TARGET_STATUS" == "ready" && "$CLAIM_TYPE" != "ready_for_pr" ]]; then
  die "--claim-type must be ready_for_pr when --to-status is ready"
fi
if [[ "$TARGET_STATUS" == "ready" ]]; then
  [[ -n "$REVIEW_PACKET_FILE" && -f "$REVIEW_PACKET_FILE" ]] || die "ready closeout requires --review-packet-file"
  FROZEN_HEAD="$(git -C "$ROOT_DIR" rev-parse HEAD)"
  grep -q 'Pre-PR Local Role Review: passed' "$REVIEW_PACKET_FILE" || die "review packet is not passed"
  grep -q "Source Head: $FROZEN_HEAD" "$REVIEW_PACKET_FILE" || die "review packet is not bound to current frozen HEAD"
  grep -Eq 'Slice Ledger: [^[:space:]].*slice-ledger.*\.jsonl' "$REVIEW_PACKET_FILE" || die "review packet lacks a machine-checkable role-return ledger"
  REVIEW_FIELDS="$(python3 - "$REVIEW_PACKET_FILE" <<'PY'
import re,sys
t=open(sys.argv[1],encoding='utf-8').read()
def f(k):
 m=re.search(rf'^- {re.escape(k)}:\s*(.+)$',t,re.M); return m.group(1).strip() if m else ''
print('roles='+f('Review Roles'))
print('head='+f('Source Head'))
print('ledger='+f('Slice Ledger'))
PY
)"
  REVIEW_ROLES="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^roles=//p')"
  REVIEW_HEAD="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^head=//p')"
  REVIEW_LEDGER="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^ledger=//p')"
  python3 "$SCRIPT_DIR/validate-review-provenance.py" --root "$ROOT_DIR" --task-uid "$TASK_UID" --ledger "$REVIEW_LEDGER" --roles "$REVIEW_ROLES" --source-head "$REVIEW_HEAD" >/dev/null \
    || die "ready closeout role-return validation failed"
fi
if [[ "$TARGET_STATUS" == "done" ]]; then
  RECORDED_PR_NUMBER="$(python3 - "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json,sys
r=(json.load(open(sys.argv[1],encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
print('' if r.get('completion_mode')=='non_pr_task' and r.get('non_pr_completion_evidence') else (r.get('pr_number') or ''))
PY
)"
  [[ -z "$RECORDED_PR_NUMBER" || -f "$PR_MERGE_RECEIPT" ]] \
    || die "PR-backed done requires an existing caller-owned --pr-receipt"
  LIVE_PR_RECEIPT=""
  if [[ -n "$RECORDED_PR_NUMBER" ]]; then
    LIVE_PR_RECEIPT="$(mktemp)"
    trap 'rm -f "$LIVE_PR_RECEIPT"' EXIT
    python3 "$SCRIPT_DIR/pr-merge-receipt.py" "$RECORDED_PR_NUMBER" --json >"$LIVE_PR_RECEIPT" \
      || die "done transition fresh recorded-PR merge query failed"
  fi
  python3 - "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$PR_MERGE_RECEIPT" "$LIVE_PR_RECEIPT" <<'PY'
import datetime as d,json,sys
mapping=json.load(open(sys.argv[1],encoding='utf-8')); r=(mapping.get('tasks') or {}).get(sys.argv[2]) or {}
if r.get('completion_mode')=='non_pr_task' and r.get('non_pr_completion_evidence'):
 raise SystemExit(0)
if not r.get('pr_url') or not r.get('pr_number'): raise SystemExit('task-closeout: done requires a recorded PR in task truth')
if not sys.argv[3]: raise SystemExit('task-closeout: done requires --pr-merge-receipt')
p=json.load(open(sys.argv[3],encoding='utf-8'))
if p.get('receipt_type')!='oasis7_pr_merge' or p.get('issuer')!='github_live_query' or p.get('evidence_mode')!='production' or p.get('state')!='MERGED': raise SystemExit('task-closeout: invalid merged PR receipt')
for key in ('repository','default_branch','pr_number','pr_url','merged_at','head_oid','base_ref','observed_at'):
 if not p.get(key): raise SystemExit(f'task-closeout: merged PR receipt is missing {key}')
if str(p.get('pr_number'))!=str(r.get('pr_number')) or p.get('pr_url')!=r.get('pr_url'): raise SystemExit('task-closeout: merged PR receipt does not match recorded PR')
for receipt_key, record_key in (('repository','repository'),('default_branch','default_branch')):
 if not r.get(record_key) or p.get(receipt_key)!=r.get(record_key): raise SystemExit(f'task-closeout: merged PR receipt {receipt_key} does not match task truth')
if p.get('base_ref')!=r.get('default_branch'): raise SystemExit('task-closeout: merged PR receipt base does not match task truth')
live=json.load(open(sys.argv[4],encoding='utf-8'))
for key in ('receipt_type','issuer','evidence_mode','repository','default_branch','pr_number','pr_url','state','merged_at','head_oid','base_ref'):
 if p.get(key)!=live.get(key): raise SystemExit(f'task-closeout: supplied merge receipt disagrees with fresh live query: {key}')
seen=d.datetime.fromisoformat(str(p.get('observed_at')).replace('Z','+00:00'))
age=(d.datetime.now(d.timezone.utc)-seen).total_seconds()
if age < -30 or age>600: raise SystemExit('task-closeout: merged PR receipt is stale')
PY
  [[ -z "$LIVE_PR_RECEIPT" ]] || rm -f "$LIVE_PR_RECEIPT"
  trap - EXIT
fi

TASK_AUDIT_JSON="$("$SCRIPT_DIR/github-project-workflow.sh" --json audit --task-uid "$TASK_UID")" \
  || die "selected-task audit failed; run ./scripts/pm/refresh-task-cache.sh --task-uid $TASK_UID --json, re-run audit, then retry task-closeout"

if [[ "$TARGET_STATUS" != "deferred" ]]; then
  CLAIM_ARGS=(--claim-type "$CLAIM_TYPE" --verification-profile "$VERIFICATION_PROFILE" --task-uid "$TASK_UID" --json)
  if [[ -n "$COMPARISON_REF" ]]; then
    CLAIM_ARGS+=(--comparison-ref "$COMPARISON_REF")
  fi
  CLAIM_READY_JSON="$("$SCRIPT_DIR/claim-ready.sh" "${CLAIM_ARGS[@]}")"
else
  CLAIM_READY_JSON="$(python3 - <<'PY'
import json
print(json.dumps({
  "claim_type": None,
  "verify_command": None,
  "verified_at": None,
  "verification_exit_code": None,
  "status": "skipped",
  "allowed_to_claim": True,
  "claim_message": "Fresh verification skipped because closeout target status is deferred."
}, sort_keys=True))
PY
)"
fi

TRANSITION_AUDIT_JSON="$("$SCRIPT_DIR/github-project-workflow.sh" --json audit --task-uid "$TASK_UID")" \
  || die "selected-task audit failed before transition; run ./scripts/pm/refresh-task-cache.sh --task-uid $TASK_UID --json, re-run audit, then retry task-closeout"

CLOSEOUT_ARGS=(closeout-task "$ROOT_DIR" --task-uid "$TASK_UID" --role "$ROLE" \
  --to-status "$TARGET_STATUS" --claim-json "$CLAIM_READY_JSON")
[[ -z "$PR_MERGE_RECEIPT" ]] || CLOSEOUT_ARGS+=(--pr-receipt "$PR_MERGE_RECEIPT")
if ! CLOSEOUT_JSON="$(python3 "$SCRIPT_DIR/github-project-task.py" "${CLOSEOUT_ARGS[@]}" --json)"; then
  die "remote closeout was incomplete; run ./scripts/pm/refresh-task-cache.sh --task-uid $TASK_UID --json, verify selected-task audit, then retry task-closeout"
fi

RESULT_JSON="$(python3 - "$ROLE" "$TARGET_STATUS" "$CLAIM_READY_JSON" "$TASK_AUDIT_JSON" "$CLOSEOUT_JSON" <<'PY'
import json
import sys

role = sys.argv[1]
target_status = sys.argv[2]
claim = json.loads(sys.argv[3])
audit = json.loads(sys.argv[4])
closeout = json.loads(sys.argv[5])
payload = {
    "task_uid": closeout["task_uid"],
    "role": role,
    "target_status": target_status,
    "final_status": closeout["status"],
    "issue_url": closeout.get("issue_url"),
    "execution_log_path": closeout.get("issue_url"),
    "claim_verification": claim,
    "task_audit": audit,
    "workflow_close": closeout,
    "move_task": closeout,
    "pm_lint": {"status": "skipped", "ran": False, "reason": "repo-local .pm/tasks retired"},
    "recommended_next_command": "./scripts/prepare-task-pr.sh",
}
print(json.dumps(payload, indent=2, sort_keys=True))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

python3 - "$RESULT_JSON" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
print("task closeout summary")
print(f"- task_uid: {payload['task_uid']}")
print(f"- role: {payload['role']}")
print(f"- final_status: {payload['final_status']}")
print(f"- issue_url: {payload['issue_url']}")
print(f"- claim_verification_status: {payload['claim_verification']['status']}")
print(f"- pm_lint: {payload['pm_lint']['status']}")
print(f"- next_step: {payload['recommended_next_command']}")
PY
