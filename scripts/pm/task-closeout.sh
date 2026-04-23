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
- verify PM structure: yes
- standard path: append execution log -> task-closeout.sh -> commit -> prepare-task-pr

Options:
  --role <role>           Owner role for `workflow-report --phase close`
  --task-uid <task_uid>   Task to close
  --to-status <status>    Final task status: done or deferred (default: done)
  --no-lint               Skip final `./scripts/pm/lint.sh`
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help

Examples:
  ./scripts/pm/task-closeout.sh --role producer_system_designer --task-uid task_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
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

RESULT_JSON="$(python3 - "$ROOT_DIR" "$ROLE" "$PM_LINT_STATUS" "$PRECHECK_JSON" "$WORKFLOW_CLOSE_JSON" "$MOVE_JSON" <<'PY'
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
precheck = json.loads(sys.argv[4])
workflow_close = json.loads(sys.argv[5])
move_task = json.loads(sys.argv[6])

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
    "pm_lint": {
        "status": pm_lint_status,
        "ran": pm_lint_status != "skipped",
    },
    "recommended_next_command": "./scripts/prepare-task-pr.sh",
    "workflow_close": workflow_close,
    "move_task": move_task,
}
print(json.dumps(payload, ensure_ascii=True, indent=2))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

python3 - "$RESULT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])

print("task closeout summary")
print(f"- task_uid: {payload['task_uid']}")
print(f"- role: {payload['role']}")
print(f"- previous_status: {payload['previous_status']}")
print(f"- final_status: {payload['final_status']}")
print(f"- execution_log_path: {payload['execution_log_path']}")
print(f"- last_started_at: {payload['last_started_at']}")
print(f"- last_closed_at: {payload['last_closed_at']}")
print(f"- pm_lint: {payload['pm_lint']['status']}")
print(f"- next_step: {payload['recommended_next_command']}")
PY
