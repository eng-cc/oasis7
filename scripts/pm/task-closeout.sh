#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/task-closeout.sh --role <role> --task-uid <task_uid> [options]

Close a GitHub Project-backed PM task without recreating repo-local .pm/tasks files.

Options:
  --role <role>           Owner role for workflow close evidence
  --task-uid <task_uid>   Task to close
  --to-status <status>    Final task status: done or deferred (default: done)
  --verify-command <cmd>  Fresh verification command to execute before done closeout
  --claim-type <type>     Claim type, must be task_complete for done (default: task_complete)
  --no-lint               Accepted for compatibility; legacy PM lint is not run
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help
USAGE
}

die() {
  echo "task-closeout: $*" >&2
  exit 1
}

ROLE=""
TASK_UID=""
TARGET_STATUS="done"
VERIFY_COMMAND=""
CLAIM_TYPE="task_complete"
OUTPUT_JSON=0

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
    --claim-type)
      CLAIM_TYPE="${2:-}"
      shift 2
      ;;
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
[[ "$TARGET_STATUS" == "done" || "$TARGET_STATUS" == "deferred" ]] || die "--to-status must be done or deferred"
if [[ "$TARGET_STATUS" == "done" && -z "$VERIFY_COMMAND" ]]; then
  die "--verify-command is required when --to-status is done"
fi
if [[ "$TARGET_STATUS" == "done" && "$CLAIM_TYPE" != "task_complete" ]]; then
  die "--claim-type must be task_complete when --to-status is done"
fi

if [[ -n "$VERIFY_COMMAND" ]]; then
  CLAIM_READY_JSON="$("$SCRIPT_DIR/claim-ready.sh" --claim-type "$CLAIM_TYPE" --verify-command "$VERIFY_COMMAND" --task-uid "$TASK_UID" --json)"
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

WORKFLOW_CLOSE_JSON="$("$SCRIPT_DIR/workflow-report.sh" --phase close --role "$ROLE" --task-uid "$TASK_UID" --json)"
MOVE_JSON="$("$SCRIPT_DIR/move-task.sh" --task-uid "$TASK_UID" --to-status "$TARGET_STATUS" --json)"

RESULT_JSON="$(python3 - "$ROLE" "$TARGET_STATUS" "$CLAIM_READY_JSON" "$WORKFLOW_CLOSE_JSON" "$MOVE_JSON" <<'PY'
import json
import sys

role = sys.argv[1]
target_status = sys.argv[2]
claim = json.loads(sys.argv[3])
workflow = json.loads(sys.argv[4])
move = json.loads(sys.argv[5])
payload = {
    "task_uid": move["task_uid"],
    "role": role,
    "target_status": target_status,
    "final_status": move["status"],
    "issue_url": move.get("issue_url") or workflow.get("issue_url"),
    "execution_log_path": workflow.get("execution_log_path"),
    "claim_verification": claim,
    "workflow_close": workflow,
    "move_task": move,
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
