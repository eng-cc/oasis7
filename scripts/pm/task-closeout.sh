#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/task-closeout.sh --role <role> --task-uid <task_uid> [options]

Run the standard PM close-phase chain for one started task without touching
commit or PR creation.

Default conventions:
- final task status: done
- when closing to `done`, fresh verification is mandatory before any close-phase writeback
- verify PM structure: yes
- standard path: append execution log -> task-closeout.sh -> commit -> prepare-task-pr

Options:
  --role <role>           Owner role for `workflow-report --phase close`
  --task-uid <task_uid>   Task to close
  --to-status <status>    Final task status: done or deferred (default: done)
  --verify-command <cmd>  Fresh verification command to execute before `done` closeout
  --claim-type <type>     Claim type for verification: task_complete|tests_passed|ready_for_pr|ready_for_merge
                          (default: task_complete)
  --no-lint               Skip final `./scripts/pm/lint.sh`
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help

Examples:
  ./scripts/pm/task-closeout.sh --role producer_system_designer --task-uid task_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx --verify-command "./scripts/doc-governance-check.sh"
  ./scripts/pm/task-closeout.sh --role qa_engineer --task-uid task_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx --to-status deferred --json
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
RUN_LINT=1
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
      RUN_LINT=0
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

PRECHECK_JSON="$(python3 - "$ROOT_DIR" "$TASK_UID" "$TARGET_STATUS" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path


def parse_task_file(path: Path) -> dict[str, str]:
    parsed: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith(" ") or raw.startswith("-"):
            continue
        key, sep, value = raw.partition(":")
        if not sep:
            continue
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] == '"':
            value = value[1:-1]
        parsed[key.strip()] = value
    return parsed


root = Path(sys.argv[1])
task_uid = sys.argv[2]
target_status = sys.argv[3]
task_path = root / ".pm" / "tasks" / f"{task_uid}.yaml"
if not task_path.exists():
    raise SystemExit(f"task-closeout: task file not found: {task_path}")

fields = parse_task_file(task_path)
current_status = fields.get("status", "")
if current_status in {"done", "deferred"}:
    raise SystemExit(f"task-closeout: task already closed with status={current_status}")
if not fields.get("last_started_at"):
    raise SystemExit(
        "task-closeout: task missing last_started_at; run `./scripts/pm/workflow-report.sh --phase start --role <owner_role> --task-uid <TASK-UID>` first"
    )

payload = {
    "task_uid": task_uid,
    "task_path": str(task_path.relative_to(root)),
    "execution_log_path": fields.get("execution_log_path"),
    "previous_status": current_status,
    "target_status": target_status,
    "last_started_at": fields.get("last_started_at"),
    "last_closed_at": fields.get("last_closed_at"),
}
print(json.dumps(payload, ensure_ascii=True))
PY
)"

if [[ -n "$VERIFY_COMMAND" ]]; then
  CLAIM_READY_JSON="$("$ROOT_DIR/scripts/pm/claim-ready.sh" --claim-type "$CLAIM_TYPE" --verify-command "$VERIFY_COMMAND" --task-uid "$TASK_UID" --json)"
else
  CLAIM_READY_JSON="$(python3 - "$TARGET_STATUS" <<'PY'
from __future__ import annotations

import json
import sys

target_status = sys.argv[1]
payload = {
    "claim_type": None,
    "verify_command": None,
    "verified_at": None,
    "verification_exit_code": None,
    "status": "skipped",
    "allowed_to_claim": target_status != "done",
    "claim_message": "Fresh verification skipped because closeout target status is deferred.",
    "blocked_phrase": None,
    "success_phrase": None,
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"
fi

WORKFLOW_CLOSE_JSON="$("$ROOT_DIR/scripts/pm/workflow-report.sh" --phase close --role "$ROLE" --task-uid "$TASK_UID" --json)"
MOVE_JSON="$("$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$TASK_UID" --to-status "$TARGET_STATUS" --json)"

if [[ "$RUN_LINT" == "1" ]]; then
  if ! "$ROOT_DIR/scripts/pm/lint.sh" >/dev/null; then
    die "pm lint failed after closeout"
  fi
  PM_LINT_STATUS="ok"
else
  PM_LINT_STATUS="skipped"
fi

RESULT_JSON_FILE="$(mktemp)"
PRECHECK_JSON_FILE="$RESULT_JSON_FILE.precheck"
CLAIM_READY_JSON_FILE="$RESULT_JSON_FILE.claim"
WORKFLOW_CLOSE_JSON_FILE="$RESULT_JSON_FILE.workflow"
MOVE_JSON_FILE="$RESULT_JSON_FILE.move"
cleanup_result_json() {
  rm -f "$RESULT_JSON_FILE" "$PRECHECK_JSON_FILE" "$CLAIM_READY_JSON_FILE" "$WORKFLOW_CLOSE_JSON_FILE" "$MOVE_JSON_FILE"
}
trap cleanup_result_json EXIT

printf '%s\n' "$PRECHECK_JSON" >"$PRECHECK_JSON_FILE"
printf '%s\n' "$CLAIM_READY_JSON" >"$CLAIM_READY_JSON_FILE"
printf '%s\n' "$WORKFLOW_CLOSE_JSON" >"$WORKFLOW_CLOSE_JSON_FILE"
printf '%s\n' "$MOVE_JSON" >"$MOVE_JSON_FILE"

python3 - "$ROOT_DIR" "$ROLE" "$PM_LINT_STATUS" "$PRECHECK_JSON_FILE" "$CLAIM_READY_JSON_FILE" "$WORKFLOW_CLOSE_JSON_FILE" "$MOVE_JSON_FILE" "$RESULT_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path


def parse_task_file(path: Path) -> dict[str, str]:
    parsed: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith(" ") or raw.startswith("-"):
            continue
        key, sep, value = raw.partition(":")
        if not sep:
            continue
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] == '"':
            value = value[1:-1]
        parsed[key.strip()] = value
    return parsed


root = Path(sys.argv[1])
role = sys.argv[2]
pm_lint_status = sys.argv[3]
precheck = json.loads(Path(sys.argv[4]).read_text(encoding="utf-8"))
claim_verification = json.loads(Path(sys.argv[5]).read_text(encoding="utf-8"))
workflow_close = json.loads(Path(sys.argv[6]).read_text(encoding="utf-8"))
move_task = json.loads(Path(sys.argv[7]).read_text(encoding="utf-8"))
result_path = Path(sys.argv[8])

task_path = root / precheck["task_path"]
fields = parse_task_file(task_path)

payload = {
    "task_uid": precheck["task_uid"],
    "role": role,
    "task_path": precheck["task_path"],
    "execution_log_path": fields.get("execution_log_path"),
    "previous_status": precheck["previous_status"],
    "final_status": fields.get("status"),
    "target_status": precheck["target_status"],
    "last_started_at": fields.get("last_started_at"),
    "last_closed_at": fields.get("last_closed_at"),
    "claim_verification": claim_verification,
    "pm_lint": {
        "status": pm_lint_status,
        "ran": pm_lint_status != "skipped",
    },
    "recommended_next_command": "./scripts/prepare-task-pr.sh",
    "workflow_close": workflow_close,
    "move_task": move_task,
}
result_path.write_text(json.dumps(payload, ensure_ascii=True, indent=2), encoding="utf-8")
PY

RESULT_JSON="$(cat "$RESULT_JSON_FILE")"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

python3 - "$RESULT_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))

print("task closeout summary")
print(f"- task_uid: {payload['task_uid']}")
print(f"- role: {payload['role']}")
print(f"- previous_status: {payload['previous_status']}")
print(f"- final_status: {payload['final_status']}")
print(f"- execution_log_path: {payload['execution_log_path']}")
print(f"- last_started_at: {payload['last_started_at']}")
print(f"- last_closed_at: {payload['last_closed_at']}")
print(f"- claim_verification_status: {payload['claim_verification']['status']}")
print(f"- claim_type: {payload['claim_verification']['claim_type']}")
print(f"- verify_command: {payload['claim_verification']['verify_command']}")
print(f"- pm_lint: {payload['pm_lint']['status']}")
print(f"- next_step: {payload['recommended_next_command']}")
PY
