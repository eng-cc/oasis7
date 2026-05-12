#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUTPUT_JSON=0
KEEP_TEMP=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/new-task-worktree-bootstrap-smoke.sh [--json] [--keep-temp]

Create a temporary task worktree, bootstrap a `.pm` task inside it through
`new-task-worktree.sh --pm-*`, and assert that the source worktree stays
unchanged while the target worktree receives the task files, start metadata,
and the canonical main-worktree `config.toml` when available.

Options:
  --json       Print machine-readable JSON summary
  --keep-temp  Keep the temporary directory for inspection
  -h, --help   Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    --keep-temp)
      KEEP_TEMP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "new-task-worktree-bootstrap-smoke: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

TMPDIR="$(mktemp -d)"
WORKTREE_PATH="$TMPDIR/worktree"
BRANCH_NAME="task/smoke-task-worktree-pm-bootstrap-$$-$(date +%s)"
SOURCE_STATUS_BEFORE="$(git -C "$ROOT_DIR" status --short)"

cleanup() {
  set +e
  if [[ "$KEEP_TEMP" == "1" ]]; then
    return
  fi
  if [[ -d "$WORKTREE_PATH" ]]; then
    git -C "$ROOT_DIR" worktree remove --force "$WORKTREE_PATH" >/dev/null 2>&1 || true
  fi
  git -C "$ROOT_DIR" branch -D "$BRANCH_NAME" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

BOOTSTRAP_JSON="$(
  cd "$ROOT_DIR" &&
  ./scripts/new-task-worktree.sh engineering smoke-task-worktree-pm-bootstrap \
    --allow-dirty-source \
    --branch "$BRANCH_NAME" \
    --path "$WORKTREE_PATH" \
    --pm-owner-role producer_system_designer \
    --pm-title "smoke bootstrap task" \
    --pm-source-ref doc/engineering/project.md \
    --pm-doc-ref doc/engineering/prd.md \
    --pm-related-prd PRD-ENGINEERING-021 \
    --pm-acceptance "bootstrap created committed task in target worktree" \
    --json
)"

SUMMARY_JSON="$(
  BOOTSTRAP_JSON="$BOOTSTRAP_JSON" python3 - "$ROOT_DIR" "$WORKTREE_PATH" "$SOURCE_STATUS_BEFORE" <<'PY'
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
worktree = Path(sys.argv[2])
source_status_before = sys.argv[3]
payload = json.loads(__import__("os").environ["BOOTSTRAP_JSON"])
pm_task = payload.get("pm_task")
if not pm_task:
    raise SystemExit("pm_task summary missing from new-task-worktree bootstrap output")
config_summary = payload.get("config")
if not config_summary:
    raise SystemExit("config summary missing from new-task-worktree output")

task_path = worktree / pm_task["task_path"]
execution_log_path = worktree / pm_task["execution_log_path"]
if not task_path.is_file():
    raise SystemExit(f"bootstrapped task file missing: {task_path}")
if not execution_log_path.is_file():
    raise SystemExit(f"bootstrapped execution log missing: {execution_log_path}")

task_text = task_path.read_text(encoding="utf-8")
if "status: committed" not in task_text:
    raise SystemExit("bootstrapped task did not move to committed")
if "last_started_at: " not in task_text or "last_started_at: null" in task_text:
    raise SystemExit("bootstrapped task missing workflow-report start timestamp")
if f"worktree_hint: {worktree}" not in task_text:
    raise SystemExit("bootstrapped task worktree_hint does not point at target worktree")

execution_log_text = execution_log_path.read_text(encoding="utf-8")
if pm_task["task_uid"] not in execution_log_text:
    raise SystemExit("execution log missing task uid header")

source_config_path = Path(config_summary["source_path"])
target_config_path = worktree / "config.toml"
if config_summary.get("source_exists"):
    if not source_config_path.is_file():
        raise SystemExit("reported canonical config source path does not exist")
    if not target_config_path.is_file():
        raise SystemExit("target worktree missing copied canonical config.toml")
    if source_config_path.read_bytes() != target_config_path.read_bytes():
        raise SystemExit("target worktree config.toml does not match canonical source config")
    config_copied = True
else:
    if target_config_path.exists():
        raise SystemExit("target worktree unexpectedly contains config.toml without canonical source file")
    config_copied = False

tracked_config = subprocess.run(
    ["git", "-C", str(worktree), "ls-files", "--error-unmatch", "config.toml"],
    text=True,
    capture_output=True,
)
if tracked_config.returncode == 0:
    raise SystemExit("copied config.toml became git-tracked inside target worktree")
ignored_status = subprocess.check_output(
    ["git", "-C", str(worktree), "status", "--short", "--ignored", "--", "config.toml"],
    text=True,
).strip()
if config_copied and ignored_status != "!! config.toml":
    raise SystemExit("copied config.toml is not ignored in target worktree as expected")
if not config_copied and ignored_status:
    raise SystemExit("target worktree reported unexpected config.toml status without canonical source file")

source_status_after = subprocess.check_output(
    ["git", "-C", str(root), "status", "--short"],
    text=True,
).rstrip("\n")
if source_status_after != source_status_before:
    raise SystemExit("source worktree status changed during PM bootstrap")

print(
    json.dumps(
        {
            "task_uid": pm_task["task_uid"],
            "task_path": pm_task["task_path"],
            "execution_log_path": pm_task["execution_log_path"],
            "source_status_preserved": True,
            "workflow_started": True,
            "worktree_path": str(worktree),
            "config_copied": config_copied,
        },
        ensure_ascii=False,
    )
)
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$SUMMARY_JSON"
  exit 0
fi

cat <<INFO
new-task-worktree bootstrap smoke passed
- worktree path: $WORKTREE_PATH
- branch: $BRANCH_NAME
- summary: $SUMMARY_JSON
INFO
