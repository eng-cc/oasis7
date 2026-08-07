#!/usr/bin/env bash
# Cross-platform maintenance: keep temporary paths readable by Git Bash and native Windows Python.
set -euo pipefail

case "$(uname -s)" in
  MSYS*|MINGW*|CYGWIN*)
    if [[ -z "${TMPDIR:-}" || "$TMPDIR" == "/tmp" || "$TMPDIR" == "/tmp/" ]]; then
      TMPDIR="$(cygpath -m "${TEMP:-${TMP:-/tmp}}")"
    elif [[ "$TMPDIR" == /* ]]; then
      TMPDIR="$(cygpath -m "$TMPDIR")"
    fi
    export TMPDIR
    ;;
esac

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
  --ci-ready-receipt <path> Trusted CI receipt bound to the reviewed draft head
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
CI_READY_RECEIPT=""
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
    --ci-ready-receipt) CI_READY_RECEIPT="${2:-}"; shift 2 ;;
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
  if [[ "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
    [[ -n "$CI_READY_RECEIPT" && -f "$CI_READY_RECEIPT" ]] || die "ready closeout requires --ci-ready-receipt"
  fi
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
print('evidence_digest='+f('Review Evidence Digest'))
PY
)"
  REVIEW_ROLES="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^roles=//p')"
  REVIEW_HEAD="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^head=//p')"
  REVIEW_LEDGER="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^ledger=//p')"
  REVIEW_EVIDENCE_DIGEST="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^evidence_digest=//p')"
  if [[ "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
    [[ "$REVIEW_EVIDENCE_DIGEST" =~ ^[0-9a-f]{64}$ ]] || die "review packet lacks a canonical Review Evidence Digest"
  fi
  REVIEW_LEDGER_PATH="$REVIEW_LEDGER"
  reviewed_source_head="$REVIEW_HEAD"
  ci_receipt_head="$FROZEN_HEAD"
  if [[ -n "$CI_READY_RECEIPT" ]]; then ci_receipt_head="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("head_oid", ""))' "$CI_READY_RECEIPT")"; fi
  same_head=false
  [[ "$ci_receipt_head" == "$FROZEN_HEAD" && "$reviewed_source_head" == "$FROZEN_HEAD" ]] && same_head=true
  [[ "$same_head" == true ]] || die "ci_ready_receipt, reviewed_source_head, and frozen HEAD must satisfy same_head"
  [[ "$REVIEW_LEDGER_PATH" == /* ]] || REVIEW_LEDGER_PATH="$ROOT_DIR/$REVIEW_LEDGER_PATH"
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

# A caller-owned CI receipt is immutable. When only its observation window has
# expired, perform one complete live same-identity validation into a temp receipt.
REFRESHED_CI_READY_RECEIPT=""
if [[ "$TARGET_STATUS" == "ready" && -n "$CI_READY_RECEIPT" && "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
  CI_RECEIPT_STALE="$(python3 - "$CI_READY_RECEIPT" <<'PY'
import datetime as d,json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
seen=d.datetime.fromisoformat(str(r.get('observed_at','')).replace('Z','+00:00'))
print('1' if not 0 <= (d.datetime.now(d.timezone.utc)-seen).total_seconds() <= 600 else '0')
PY
)" || die "ci-ready receipt observed_at is invalid"
  if [[ "$CI_RECEIPT_STALE" == "1" ]]; then
    REFRESHED_CI_READY_RECEIPT="$(mktemp)"
    trap 'rm -f "$REFRESHED_CI_READY_RECEIPT"' EXIT
    CI_IDENTITY_JSON="$(python3 - "$CI_READY_RECEIPT" <<'PY'
import json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
print(json.dumps([r.get(k,'') for k in ('repository','task_issue_number','pr_number','check_name','check_app_id','planner_digest')]))
PY
)"
    CI_REPOSITORY="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[0])' "$CI_IDENTITY_JSON")"
    CI_TASK_ISSUE="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[1])' "$CI_IDENTITY_JSON")"
    CI_PR_NUMBER="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[2])' "$CI_IDENTITY_JSON")"
    CI_CHECK_NAME="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[3])' "$CI_IDENTITY_JSON")"
    CI_CHECK_APP="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[4])' "$CI_IDENTITY_JSON")"
    CI_PLANNER_DIGEST="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[5])' "$CI_IDENTITY_JSON")"
    python3 "$SCRIPT_DIR/ci-ready-receipt.py" \
      --repository "$CI_REPOSITORY" --task-uid "$TASK_UID" \
      --task-issue-number "$CI_TASK_ISSUE" --pr-number "$CI_PR_NUMBER" \
      --check-name "$CI_CHECK_NAME" --check-app-id "$CI_CHECK_APP" \
      --planner-digest "$CI_PLANNER_DIGEST" --receipt "$CI_READY_RECEIPT" \
      --refresh-same-identity --json >"$REFRESHED_CI_READY_RECEIPT" \
      || die "stale ci-ready receipt failed same-identity refresh"
    CI_READY_RECEIPT="$REFRESHED_CI_READY_RECEIPT"
  fi
fi

if [[ "$TARGET_STATUS" == "ready" && -n "$CI_READY_RECEIPT" && "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
  CI_REVIEW_EVIDENCE_DIGEST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("review_evidence_digest", ""))' "$CI_READY_RECEIPT")" \
    || die "cannot read ci-ready receipt review authority"
  [[ "$CI_REVIEW_EVIDENCE_DIGEST" =~ ^[0-9a-f]{64}$ ]] || die "ci-ready receipt lacks a canonical review evidence digest"
  [[ "$CI_REVIEW_EVIDENCE_DIGEST" == "$REVIEW_EVIDENCE_DIGEST" ]] \
    || die "ci-ready receipt authority does not match reviewed evidence digest"
fi

selected_task_audit() {
  "$SCRIPT_DIR/github-project-workflow.sh" --json audit --task-uid "$TASK_UID"
}
closeout_head_fingerprint() {
  git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || printf '%s\n' "non-git-fixture"
}
sha256_file() {
  python3 - "$1" <<'PY'
import hashlib,pathlib,sys
print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
}
AUDIT_INPUT_HEAD="$(closeout_head_fingerprint)"
AUDIT_INPUT_MAPPING_SHA="$(sha256_file "$ROOT_DIR/.pm/github-project-sync/tasks.json")"
AUDIT_INPUT_REVIEW_SHA="$([[ -n "$REVIEW_PACKET_FILE" ]] && sha256_file "$REVIEW_PACKET_FILE" || printf none)"
AUDIT_INPUT_LEDGER_SHA="$([[ -n "${REVIEW_LEDGER_PATH:-}" ]] && sha256_file "$REVIEW_LEDGER_PATH" || printf none)"
AUDIT_INPUT_PR_RECEIPT_SHA="$([[ -n "$PR_MERGE_RECEIPT" ]] && sha256_file "$PR_MERGE_RECEIPT" || printf none)"
if [[ "$TARGET_STATUS" != "deferred" ]]; then
  CLAIM_ARGS=(--claim-type "$CLAIM_TYPE" --verification-profile "$VERIFICATION_PROFILE" --task-uid "$TASK_UID" --json)
  if [[ "$CLAIM_TYPE" == "ready_for_pr" && -n "$CI_READY_RECEIPT" ]]; then
    CLAIM_ARGS+=(--ci-ready-receipt "$CI_READY_RECEIPT")
  fi
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

CURRENT_HEAD="$(closeout_head_fingerprint)"
CURRENT_MAPPING_SHA="$(sha256_file "$ROOT_DIR/.pm/github-project-sync/tasks.json")"
CURRENT_REVIEW_SHA="$([[ -n "$REVIEW_PACKET_FILE" ]] && sha256_file "$REVIEW_PACKET_FILE" || printf none)"
CURRENT_LEDGER_SHA="$([[ -n "${REVIEW_LEDGER_PATH:-}" ]] && sha256_file "$REVIEW_LEDGER_PATH" || printf none)"
CURRENT_PR_RECEIPT_SHA="$([[ -n "$PR_MERGE_RECEIPT" ]] && sha256_file "$PR_MERGE_RECEIPT" || printf none)"
[[ "$CURRENT_HEAD" == "$AUDIT_INPUT_HEAD" && "$CURRENT_MAPPING_SHA" == "$AUDIT_INPUT_MAPPING_SHA" && \
   "$CURRENT_REVIEW_SHA" == "$AUDIT_INPUT_REVIEW_SHA" && "$CURRENT_LEDGER_SHA" == "$AUDIT_INPUT_LEDGER_SHA" && \
   "$CURRENT_PR_RECEIPT_SHA" == "$AUDIT_INPUT_PR_RECEIPT_SHA" ]] \
  || die "closeout inputs changed during verification; restart selected-task closeout"
# Run exactly one authoritative selected live audit after claim/evidence inputs
# are proven stable, immediately before the transition that consumes it.
TASK_AUDIT_JSON="$(selected_task_audit)" \
  || die "selected-task audit failed at transition"
TRANSITION_AUDIT_JSON="$TASK_AUDIT_JSON"

CLOSEOUT_ARGS=(closeout-task "$ROOT_DIR" --task-uid "$TASK_UID" --role "$ROLE" \
  --to-status "$TARGET_STATUS" --claim-json "$CLAIM_READY_JSON")
[[ -z "$PR_MERGE_RECEIPT" ]] || CLOSEOUT_ARGS+=(--pr-receipt "$PR_MERGE_RECEIPT")
if ! CLOSEOUT_JSON="$(python3 "$SCRIPT_DIR/github-project-task.py" "${CLOSEOUT_ARGS[@]}" --json)"; then
  die "remote closeout was incomplete; run ./scripts/pm/refresh-task-cache.sh --task-uid $TASK_UID --json, verify selected-task audit, then retry task-closeout"
fi

# Independent selected-task postcondition readback. This is the second bounded
# task-scoped audit (after the pre-transition audit), never a broad Project read.
POSTCONDITION_AUDIT_JSON="$(selected_task_audit)" \
  || die "selected-task postcondition readback failed after closeout"
python3 - "$TASK_UID" "$TARGET_STATUS" "$CLOSEOUT_JSON" "$POSTCONDITION_AUDIT_JSON" "$VERIFICATION_PROFILE" <<'PY'
import json,sys
task_uid,target=json.loads(json.dumps(sys.argv[1])),sys.argv[2]
closeout=json.loads(sys.argv[3]); audit=json.loads(sys.argv[4])
VERIFICATION_PROFILE=sys.argv[5]
expected_phase={'ready':'pre_pr_ready','done':'task_done','deferred':'blocked'}[target]
# github-project-workflow audit is itself the selected live task/Project
# postcondition authority. Its successful exit is required above; older fixture
# adapters return only {"status":"ok"}, while production returns richer detail.
if audit.get('status') == 'ok' and set(audit) == {'status'}:
 if VERIFICATION_PROFILE != 'fixture_repository_state':
  raise SystemExit('task-closeout: live selected-task postcondition audit lacks structured task readback')
else:
 readback=audit.get('selected_task') if isinstance(audit.get('selected_task'),dict) else audit
 if (readback.get('task_uid') != task_uid or readback.get('target') != target
     or readback.get('workflow_phase') != expected_phase):
  raise SystemExit('task-closeout: selected-task postcondition audit lacks expected status/phase')
PY

RESULT_JSON="$(python3 - "$ROLE" "$TARGET_STATUS" "$CLAIM_READY_JSON" "$TASK_AUDIT_JSON" "$CLOSEOUT_JSON" "$POSTCONDITION_AUDIT_JSON" <<'PY'
import json
import sys

role = sys.argv[1]
target_status = sys.argv[2]
claim = json.loads(sys.argv[3])
audit = json.loads(sys.argv[4])
closeout = json.loads(sys.argv[5])
postcondition = json.loads(sys.argv[6])
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
    "postcondition_readback": postcondition,
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
