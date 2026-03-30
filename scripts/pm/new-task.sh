#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/new-task.sh --owner-role <role> --title <title> --source-ref <path> [options]

Create one candidate task file under .pm/tasks/, register it in .pm/registry/tasks.yaml,
and append the task to the owner's backlog/candidate.yaml.

Required:
  --owner-role <role>         Canonical role name from .pm/registry/roles.yaml
  --title <title>             Candidate task title
  --source-ref <path>         Source evidence path; repeatable

Optional:
  --priority <P0|P1|P2|P3>    Default: P2
  --source-signal <signal_id> Source signal id for traceability
  --doc-ref <path>            Related formal doc; repeatable
  --related-prd <path>        Related PRD; repeatable
  --acceptance <text>         Acceptance criterion; repeatable
  --handoff-to <role>         Suggested handoff role; repeatable
  --worktree-hint <name>      Optional worktree hint
  --json                      Print machine-readable JSON summary
  -h, --help                  Show help

Notes:
  - TASK-ENGINEERING-076 only creates candidate tasks.
  - Use PM_ROOT_DIR=/tmp/... to smoke-test against a copied .pm tree.
USAGE
}

OWNER_ROLE=""
TITLE=""
PRIORITY="P2"
SOURCE_SIGNAL=""
WORKTREE_HINT=""
OUTPUT_JSON=0
SOURCE_REFS=()
DOC_REFS=()
RELATED_PRDS=()
ACCEPTANCE=()
HANDOFF_TO=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --owner-role)
      OWNER_ROLE="${2:-}"
      shift 2
      ;;
    --title)
      TITLE="${2:-}"
      shift 2
      ;;
    --priority)
      PRIORITY="${2:-}"
      shift 2
      ;;
    --source-signal)
      SOURCE_SIGNAL="${2:-}"
      shift 2
      ;;
    --source-ref)
      SOURCE_REFS+=("${2:-}")
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
      echo "new-task: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$OWNER_ROLE" ]] || { echo "new-task: --owner-role is required" >&2; exit 2; }
[[ -n "$TITLE" ]] || { echo "new-task: --title is required" >&2; exit 2; }
[[ "${#SOURCE_REFS[@]}" -gt 0 ]] || { echo "new-task: at least one --source-ref is required" >&2; exit 2; }

case "$PRIORITY" in
  P0|P1|P2|P3) ;;
  *)
    echo "new-task: unsupported priority: $PRIORITY" >&2
    exit 2
    ;;
esac

grep -Fxq "$OWNER_ROLE" < <(sed -n 's/^  - role_name: //p' .pm/registry/roles.yaml) || {
  echo "new-task: unknown owner role: $OWNER_ROLE" >&2
  exit 2
}

for handoff_role in "${HANDOFF_TO[@]}"; do
  grep -Fxq "$handoff_role" < <(sed -n 's/^  - role_name: //p' .pm/registry/roles.yaml) || {
    echo "new-task: unknown handoff role: $handoff_role" >&2
    exit 2
  }
done

UPDATED_AT="$(date -Iseconds)"
TASK_JSON="$(python3 - "$ROOT_DIR" "$OWNER_ROLE" "$TITLE" "$PRIORITY" "$SOURCE_SIGNAL" "$WORKTREE_HINT" "$UPDATED_AT" \
  "$(printf '%s\n' "${SOURCE_REFS[@]}")" \
  "$(printf '%s\n' "${DOC_REFS[@]}")" \
  "$(printf '%s\n' "${RELATED_PRDS[@]}")" \
  "$(printf '%s\n' "${ACCEPTANCE[@]}")" \
  "$(printf '%s\n' "${HANDOFF_TO[@]}")" <<'PY'
from __future__ import annotations

import json
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
owner_role = sys.argv[2]
title = sys.argv[3]
priority = sys.argv[4]
source_signal = sys.argv[5] or None
worktree_hint = sys.argv[6] or None
updated_at = sys.argv[7]
source_refs = [line for line in sys.argv[8].splitlines() if line]
doc_refs = [line for line in sys.argv[9].splitlines() if line]
related_prds = [line for line in sys.argv[10].splitlines() if line]
acceptance = [line for line in sys.argv[11].splitlines() if line]
handoff_to = [line for line in sys.argv[12].splitlines() if line]

registry_path = root / ".pm/registry/tasks.yaml"
registry_text = registry_path.read_text(encoding="utf-8")
match = re.search(r"^next_sequence: (\d+)$", registry_text, re.MULTILINE)
if not match:
    raise SystemExit("new-task: next_sequence missing from .pm/registry/tasks.yaml")

sequence = int(match.group(1))
task_id = f"TASK-PM-{sequence:04d}"
task_path_rel = f".pm/tasks/{task_id}.yaml"
task_path = root / task_path_rel
if task_path.exists():
    raise SystemExit(f"new-task: task file already exists: {task_path_rel}")

backlog_path_rel = f".pm/roles/{owner_role}/backlog/candidate.yaml"
backlog_path = root / backlog_path_rel
if not backlog_path.exists():
    raise SystemExit(f"new-task: candidate backlog missing for role: {owner_role}")

def scalar(value: str | None) -> str:
    if value is None:
        return "null"
    if re.fullmatch(r"[A-Za-z0-9_.:-]+", value):
        return value
    return json.dumps(value, ensure_ascii=False)

def list_block(key: str, values: list[str], indent: str = "") -> str:
    if not values:
        return f"{indent}{key}: []\n"
    lines = [f"{indent}{key}:\n"]
    for value in values:
        lines.append(f"{indent}  - {scalar(value)}\n")
    return "".join(lines)

task_text = "".join(
    [
        f"task_id: {task_id}\n",
        f"title: {scalar(title)}\n",
        f"owner_role: {owner_role}\n",
        f"worktree_hint: {scalar(worktree_hint)}\n",
        "status: candidate\n",
        f"priority: {priority}\n",
        f"source_signal: {scalar(source_signal)}\n",
        list_block("source_refs", source_refs),
        list_block("doc_refs", doc_refs),
        list_block("related_prd", related_prds),
        list_block("acceptance", acceptance),
        list_block("handoff_to", handoff_to),
        f"updated_at: {updated_at}\n",
    ]
)
task_path.write_text(task_text, encoding="utf-8")

registry_entry = "".join(
    [
        f"  - task_id: {task_id}\n",
        f"    owner_role: {owner_role}\n",
        f"    task_path: {task_path_rel}\n",
        "    status: candidate\n",
        f"    priority: {priority}\n",
        f"    source_signal: {scalar(source_signal)}\n",
        f"    updated_at: {updated_at}\n",
    ]
)

if "tasks: []" in registry_text:
    registry_text = registry_text.replace("tasks: []", f"tasks:\n{registry_entry.rstrip()}", 1)
else:
    if not registry_text.endswith("\n"):
        registry_text += "\n"
    registry_text += registry_entry

registry_text = re.sub(
    r"^next_sequence: \d+$",
    f"next_sequence: {sequence + 1}",
    registry_text,
    count=1,
    flags=re.MULTILINE,
)
registry_path.write_text(registry_text if registry_text.endswith("\n") else registry_text + "\n", encoding="utf-8")

backlog_text = backlog_path.read_text(encoding="utf-8")
backlog_entry = "".join(
    [
        f"  - task_id: {task_id}\n",
        f"    title: {scalar(title)}\n",
        f"    priority: {priority}\n",
        f"    source_signal: {scalar(source_signal)}\n",
        list_block("related_prd", related_prds, "    "),
        list_block("acceptance", acceptance, "    "),
        list_block("handoff_to", handoff_to, "    "),
        "    status: candidate\n",
        f"    task_path: {task_path_rel}\n",
    ]
)

if "tasks: []" in backlog_text:
    backlog_text = backlog_text.replace("tasks: []", f"tasks:\n{backlog_entry.rstrip()}", 1)
else:
    if not backlog_text.endswith("\n"):
        backlog_text += "\n"
    backlog_text += backlog_entry

backlog_path.write_text(backlog_text if backlog_text.endswith("\n") else backlog_text + "\n", encoding="utf-8")

print(
    json.dumps(
        {
            "task_id": task_id,
            "task_path": task_path_rel,
            "backlog_path": backlog_path_rel,
            "owner_role": owner_role,
            "priority": priority,
            "status": "candidate",
            "source_signal": source_signal,
            "updated_at": updated_at,
        },
        ensure_ascii=False,
    )
)
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$TASK_JSON"
  exit 0
fi

TASK_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_id"])' <<<"$TASK_JSON")"
TASK_PATH="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_path"])' <<<"$TASK_JSON")"
echo "new-task: created $TASK_ID ($TASK_PATH)"
