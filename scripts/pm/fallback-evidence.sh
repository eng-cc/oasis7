#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/fallback-evidence.sh <create|audit|replay> --task-uid <task_uid> [options]

Manage temporary fallback evidence under .pm/scratch/<task_uid>/fallback-evidence.
Fallback packets are not task truth until replayed to the GitHub task issue.

Commands:
  create   Read a fallback packet from stdin and write a timestamped .md file.
  audit    Fail if unreplayed fallback packets exist for the task.
  replay   Post unreplayed fallback packets to the GitHub issue and mark them replayed.

Options:
  --task-uid <task_uid>   Task UID
  --issue <number>        GitHub issue number for replay; defaults from mapping
  --repo <owner/repo>     GitHub repo for replay; defaults from mapping or eng-cc/oasis7
  --reason <text>         Required for create
  --json                  Print JSON summary
USAGE
}

die() {
  echo "fallback-evidence: $*" >&2
  exit 1
}

COMMAND="${1:-}"
[[ -n "$COMMAND" ]] || { usage >&2; exit 2; }
shift || true

TASK_UID=""
ISSUE=""
REPO=""
REASON=""
OUTPUT_JSON=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --issue) ISSUE="${2:-}"; shift 2 ;;
    --repo) REPO="${2:-}"; shift 2 ;;
    --reason) REASON="${2:-}"; shift 2 ;;
    --json) OUTPUT_JSON=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ "$COMMAND" == "create" || "$COMMAND" == "audit" || "$COMMAND" == "replay" ]] || die "unknown command: $COMMAND"
[[ "$TASK_UID" =~ ^task_[0-9a-f]{32}$ ]] || die "--task-uid must be a task_<32 hex> value"

FALLBACK_DIR="$ROOT_DIR/.pm/scratch/$TASK_UID/fallback-evidence"

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))'
}

emit_json() {
  if [[ "$OUTPUT_JSON" == "1" ]]; then
    printf '%s\n' "$1"
  else
    python3 - "$1" <<'PY'
import json, sys
payload = json.loads(sys.argv[1])
print(f"fallback-evidence: {payload['status']} command={payload['command']} task_uid={payload['task_uid']}")
for path in payload.get("paths", []):
    print(f"- {path}")
PY
  fi
}

unreplayed_files() {
  find "$FALLBACK_DIR" -maxdepth 1 -type f -name '*.md' ! -name '*.replayed.md' 2>/dev/null | sort
}

mapping_defaults() {
  python3 - "$ROOT_DIR" "$TASK_UID" <<'PY'
from __future__ import annotations
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
uid = sys.argv[2]
path = root / ".pm/github-project-sync/tasks.json"
repo = "eng-cc/oasis7"
issue = ""
if path.is_file():
    mapping = json.loads(path.read_text(encoding="utf-8"))
    repo = str((mapping.get("project") or {}).get("repo") or repo)
    record = (mapping.get("tasks") or {}).get(uid) or {}
    issue = str(record.get("issue_number") or "")
print(repo)
print(issue)
PY
}

case "$COMMAND" in
  create)
    [[ -n "$REASON" ]] || die "--reason is required for create"
    mkdir -p "$FALLBACK_DIR"
    timestamp="$(date +%Y%m%dT%H%M%S%z)"
    target="$(mktemp "$FALLBACK_DIR/$timestamp.$$.XXXXXX.md")"
    {
      echo "<!-- oasis7-pm-fallback-evidence -->"
      echo "Task UID: $TASK_UID"
      echo "Reason: $REASON"
      echo "Created At: $(date -Iseconds)"
      echo "Replay Target: GitHub task issue comment"
      echo
      cat
    } > "$target"
    paths_json="$(printf '%s\n' "$target" | python3 -c 'import json,sys; print(json.dumps([line.strip() for line in sys.stdin if line.strip()]))')"
    emit_json "{\"status\":\"created\",\"command\":\"create\",\"task_uid\":\"$TASK_UID\",\"paths\":$paths_json}"
    ;;
  audit)
    files=()
    while IFS= read -r path; do
      [[ -n "$path" ]] && files+=("$path")
    done < <(unreplayed_files)
    if [[ "${#files[@]}" -gt 0 ]]; then
      paths_json="$(printf '%s\n' "${files[@]}" | python3 -c 'import json,sys; print(json.dumps([line.strip() for line in sys.stdin if line.strip()]))')"
      emit_json "{\"status\":\"unreplayed\",\"command\":\"audit\",\"task_uid\":\"$TASK_UID\",\"paths\":$paths_json}"
      exit 1
    fi
    emit_json "{\"status\":\"ok\",\"command\":\"audit\",\"task_uid\":\"$TASK_UID\",\"paths\":[]}"
    ;;
  replay)
    read -r default_repo default_issue < <(mapping_defaults | paste -sd ' ' -)
    REPO="${REPO:-$default_repo}"
    ISSUE="${ISSUE:-$default_issue}"
    [[ -n "$ISSUE" ]] || die "--issue is required when mapping has no issue_number"
    files=()
    while IFS= read -r path; do
      [[ -n "$path" ]] && files+=("$path")
    done < <(unreplayed_files)
    posted=()
    for path in "${files[@]}"; do
      gh issue comment "$ISSUE" -R "$REPO" --body-file "$path" >/dev/null
      replayed="${path%.md}.replayed.md"
      mv "$path" "$replayed"
      posted+=("$replayed")
    done
    paths_json="$(printf '%s\n' "${posted[@]}" | python3 -c 'import json,sys; print(json.dumps([line.strip() for line in sys.stdin if line.strip()]))')"
    emit_json "{\"status\":\"replayed\",\"command\":\"replay\",\"task_uid\":\"$TASK_UID\",\"paths\":$paths_json}"
    ;;
esac
