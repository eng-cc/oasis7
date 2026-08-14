#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
TASK_UID=""
OWNER_ROLE=""
OUTPUT_JSON=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/resume-task-worktree-bootstrap.sh --repo-root <path> --task-uid <uid> --owner-role <role> [--json]

Resume the post-create task-worktree bootstrap idempotently. Existing task truth
and bootstrap snapshot state are reused; already-committed and already-started
steps are skipped.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      ROOT_DIR="$(cd "${2:?missing --repo-root value}" && pwd -P)"
      shift 2
      ;;
    --task-uid)
      TASK_UID="${2:?missing --task-uid value}"
      shift 2
      ;;
    --owner-role)
      OWNER_ROLE="${2:?missing --owner-role value}"
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
      echo "resume-task-worktree-bootstrap: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ "$TASK_UID" =~ ^task_[0-9a-f]{32}$ ]] || {
  echo "resume-task-worktree-bootstrap: invalid task UID: $TASK_UID" >&2
  exit 2
}
[[ -n "$OWNER_ROLE" ]] || {
  echo "resume-task-worktree-bootstrap: --owner-role is required" >&2
  exit 2
}

MAPPING_PATH="$ROOT_DIR/.pm/github-project-sync/tasks.json"
# Refresh first so the status/start markers come from the canonical GitHub-backed
# mapping before deciding which already-completed local steps can be skipped.
"$ROOT_DIR/scripts/pm/refresh-task-cache.sh" --task-uid "$TASK_UID" --json >/dev/null
TASK_STATE_JSON="$(python3 - "$MAPPING_PATH" "$TASK_UID" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
record = (payload.get("tasks") or {}).get(sys.argv[2]) or {}
print(json.dumps({
    "status": str(record.get("status") or ""),
    "last_started_at": str(record.get("last_started_at") or ""),
}))
PY
)"

TASK_STATUS="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["status"])' "$TASK_STATE_JSON")"
MOVED=0
if [[ "$TASK_STATUS" != "committed" ]]; then
  "$ROOT_DIR/scripts/pm/move-task.sh" \
    --task-uid "$TASK_UID" --to-status committed --json >/dev/null
  MOVED=1
fi

WORKFLOW_STARTED=0
LAST_STARTED_AT="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["last_started_at"])' "$TASK_STATE_JSON")"
if [[ -z "$LAST_STARTED_AT" || "$MOVED" == "1" ]]; then
  # A move can refresh the mapping, so re-read the marker before deciding
  # whether the start evidence is still absent.
  LAST_STARTED_AT="$(python3 - "$MAPPING_PATH" "$TASK_UID" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
record = (payload.get("tasks") or {}).get(sys.argv[2]) or {}
print(record.get("last_started_at") or "")
PY
  )"
  if [[ -z "$LAST_STARTED_AT" ]]; then
    "$ROOT_DIR/scripts/pm/workflow-report.sh" \
      --phase start --role "$OWNER_ROLE" --task-uid "$TASK_UID" --json >/dev/null
    WORKFLOW_STARTED=1
  fi
fi

SNAPSHOT_JSON="$(python3 "$ROOT_DIR/scripts/pm/bootstrap-task-snapshot.py" validate-or-create \
  --repo-root "$ROOT_DIR" --task-uid "$TASK_UID" \
  --producer scripts/pm/resume-task-worktree-bootstrap.sh)"
SNAPSHOT_PATH="$ROOT_DIR/.pm/scratch/$TASK_UID/bootstrap-task-snapshot.json"
SNAPSHOT_DIGEST="$(python3 - "$SNAPSHOT_PATH" <<'PY'
import json
import sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["digest"])
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  python3 - "$TASK_UID" "$MOVED" "$WORKFLOW_STARTED" "$SNAPSHOT_PATH" "$SNAPSHOT_DIGEST" <<'PY'
import json
import sys
print(json.dumps({
    "schema": "oasis7.bootstrap_resume_result.v1",
    "status": "ok",
    "task_uid": sys.argv[1],
    "moved": sys.argv[2] == "1",
    "workflow_started": sys.argv[3] == "1",
    "bootstrap_snapshot_path": sys.argv[4],
    "bootstrap_snapshot_digest": sys.argv[5],
}, sort_keys=True))
PY
else
  printf 'resume-task-worktree-bootstrap: resumed %s (moved=%s workflow_started=%s snapshot=%s)\n' \
    "$TASK_UID" "$MOVED" "$WORKFLOW_STARTED" "$SNAPSHOT_PATH"
fi
