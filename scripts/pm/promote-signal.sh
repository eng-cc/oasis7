#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/promote-signal.sh --source-type <type> --source-ref <path> --role-hint <role> --severity <level> --summary <text> [options]

Append one signal to .pm/inbox/signals.jsonl. When --create-task is supplied, also
create a candidate task through ./scripts/pm/new-task.sh.

Required:
  --source-type <type>        e.g. devlog, incident, qa_block, community_feedback
  --source-ref <path>         Primary source reference
  --role-hint <role>          Canonical role owner hint
  --severity <level>          low | medium | high | critical
  --summary <text>            Signal summary

Optional:
  --signal-id <id>            Override auto-generated SIG-PM-XXXX id
  --create-task               Also create a candidate task
  --title <title>             Task title; defaults to summary
  --owner-role <role>         Task owner; defaults to role_hint
  --priority <P0|P1|P2|P3>    Task priority; defaults from severity
  --doc-ref <path>            Related formal doc; repeatable
  --related-prd <path>        Related PRD; repeatable
  --acceptance <text>         Acceptance criterion; repeatable
  --handoff-to <role>         Suggested handoff role; repeatable
  --worktree-hint <name>      Optional worktree hint for created task
  --json                      Print machine-readable JSON summary
  -h, --help                  Show help

Notes:
  - Without --create-task, the signal is written with promotion_state=triaged.
  - With --create-task, the signal is written with promotion_state=promoted_candidate_task.
  - Use PM_ROOT_DIR=/tmp/... to smoke-test against a copied .pm tree.
USAGE
}

SIGNAL_ID=""
SOURCE_TYPE=""
SOURCE_REF=""
ROLE_HINT=""
SEVERITY=""
SUMMARY=""
CREATE_TASK=0
TASK_TITLE=""
OWNER_ROLE=""
PRIORITY=""
WORKTREE_HINT=""
OUTPUT_JSON=0
DOC_REFS=()
RELATED_PRDS=()
ACCEPTANCE=()
HANDOFF_TO=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --signal-id)
      SIGNAL_ID="${2:-}"
      shift 2
      ;;
    --source-type)
      SOURCE_TYPE="${2:-}"
      shift 2
      ;;
    --source-ref)
      SOURCE_REF="${2:-}"
      shift 2
      ;;
    --role-hint)
      ROLE_HINT="${2:-}"
      shift 2
      ;;
    --severity)
      SEVERITY="${2:-}"
      shift 2
      ;;
    --summary)
      SUMMARY="${2:-}"
      shift 2
      ;;
    --create-task)
      CREATE_TASK=1
      shift
      ;;
    --title)
      TASK_TITLE="${2:-}"
      shift 2
      ;;
    --owner-role)
      OWNER_ROLE="${2:-}"
      shift 2
      ;;
    --priority)
      PRIORITY="${2:-}"
      shift 2
      ;;
    --doc-ref)
      DOC_REFS+=("${2:-}")
      shift 2
      ;;
    --related-prd)
      RELATED_PRDS+=("${2:-}")
      shift 2
      ;;
    --acceptance)
      ACCEPTANCE+=("${2:-}")
      shift 2
      ;;
    --handoff-to)
      HANDOFF_TO+=("${2:-}")
      shift 2
      ;;
    --worktree-hint)
      WORKTREE_HINT="${2:-}"
      shift 2
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
      echo "promote-signal: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$SOURCE_TYPE" ]] || { echo "promote-signal: --source-type is required" >&2; exit 2; }
[[ -n "$SOURCE_REF" ]] || { echo "promote-signal: --source-ref is required" >&2; exit 2; }
[[ -n "$ROLE_HINT" ]] || { echo "promote-signal: --role-hint is required" >&2; exit 2; }
[[ -n "$SEVERITY" ]] || { echo "promote-signal: --severity is required" >&2; exit 2; }
[[ -n "$SUMMARY" ]] || { echo "promote-signal: --summary is required" >&2; exit 2; }

case "$SEVERITY" in
  low|medium|high|critical) ;;
  *)
    echo "promote-signal: unsupported severity: $SEVERITY" >&2
    exit 2
    ;;
esac

grep -Fxq "$ROLE_HINT" < <(sed -n 's/^  - role_name: //p' .pm/registry/roles.yaml) || {
  echo "promote-signal: unknown role hint: $ROLE_HINT" >&2
  exit 2
}

if [[ -z "$SIGNAL_ID" ]]; then
  SIGNAL_ID="$(python3 - "$ROOT_DIR" <<'PY'
from __future__ import annotations

import json
import pathlib
import re
import sys

signals_path = pathlib.Path(sys.argv[1]) / ".pm/inbox/signals.jsonl"
if not signals_path.exists():
    print("SIG-PM-0001")
    raise SystemExit(0)

max_seq = 0
for raw_line in signals_path.read_text(encoding="utf-8").splitlines():
    line = raw_line.strip()
    if not line:
        continue
    payload = json.loads(line)
    signal_id = payload.get("signal_id", "")
    match = re.fullmatch(r"SIG-PM-(\d{4})", signal_id)
    if match:
        max_seq = max(max_seq, int(match.group(1)))
print(f"SIG-PM-{max_seq + 1:04d}")
PY
)"
fi

PROMOTION_STATE="triaged"
TASK_JSON="null"

if [[ "$CREATE_TASK" == "1" ]]; then
  [[ -n "$TASK_TITLE" ]] || TASK_TITLE="$SUMMARY"
  [[ -n "$OWNER_ROLE" ]] || OWNER_ROLE="$ROLE_HINT"

  if [[ -z "$PRIORITY" ]]; then
    case "$SEVERITY" in
      critical) PRIORITY="P0" ;;
      high) PRIORITY="P1" ;;
      medium) PRIORITY="P2" ;;
      low) PRIORITY="P3" ;;
    esac
  fi

  TASK_ARGS=(
    --owner-role "$OWNER_ROLE"
    --title "$TASK_TITLE"
    --priority "$PRIORITY"
    --source-signal "$SIGNAL_ID"
    --source-ref "$SOURCE_REF"
    --json
  )

  for doc_ref in "${DOC_REFS[@]}"; do
    TASK_ARGS+=(--doc-ref "$doc_ref")
  done

  for related_prd in "${RELATED_PRDS[@]}"; do
    TASK_ARGS+=(--related-prd "$related_prd")
  done

  for acceptance_item in "${ACCEPTANCE[@]}"; do
    TASK_ARGS+=(--acceptance "$acceptance_item")
  done

  for handoff_role in "${HANDOFF_TO[@]}"; do
    TASK_ARGS+=(--handoff-to "$handoff_role")
  done

  if [[ -n "$WORKTREE_HINT" ]]; then
    TASK_ARGS+=(--worktree-hint "$WORKTREE_HINT")
  fi

  TASK_JSON="$("$SCRIPT_DIR/new-task.sh" "${TASK_ARGS[@]}")"
  PROMOTION_STATE="promoted_candidate_task"
fi

touch .pm/inbox/signals.jsonl
python3 - "$ROOT_DIR" "$SIGNAL_ID" "$SOURCE_TYPE" "$SOURCE_REF" "$ROLE_HINT" "$SEVERITY" "$SUMMARY" "$PROMOTION_STATE" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
signals_path = root / ".pm/inbox/signals.jsonl"
payload = {
    "signal_id": sys.argv[2],
    "source_type": sys.argv[3],
    "source_ref": sys.argv[4],
    "role_hint": sys.argv[5],
    "severity": sys.argv[6],
    "summary": sys.argv[7],
    "promotion_state": sys.argv[8],
    "memory_promotion_state": "pending",
}

with signals_path.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload, ensure_ascii=False) + "\n")
PY

RESULT_JSON="$(python3 - "$SIGNAL_ID" "$PROMOTION_STATE" "$TASK_JSON" <<'PY'
from __future__ import annotations

import json
import sys

task_payload = None if sys.argv[3] == "null" else json.loads(sys.argv[3])
print(
    json.dumps(
        {
            "signal_id": sys.argv[1],
            "promotion_state": sys.argv[2],
            "task": task_payload,
        },
        ensure_ascii=False,
    )
)
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

if [[ "$CREATE_TASK" == "1" ]]; then
  TASK_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task"]["task_id"])' <<<"$RESULT_JSON")"
  echo "promote-signal: wrote $SIGNAL_ID and created $TASK_ID"
else
  echo "promote-signal: wrote $SIGNAL_ID"
fi
