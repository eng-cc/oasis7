#!/usr/bin/env bash
set -euo pipefail

CALLER_DIR="$(pwd -P)"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/new-task-worktree.sh <module> <task> [options]

Create or attach a standardized git worktree for one task slice.

Default conventions:
- branch: task/<module>-<task>
- worktrees root: <repo-parent>/worktrees
- worktree path: <worktrees root>/<repo-name>-<module>-<task>
- base ref: HEAD

Options:
  --base <ref>            Base ref for a new branch (default: HEAD)
  --branch <name>         Override branch name
  --path <path>           Override target worktree path
  --worktrees-root <dir>  Override default worktrees root
  --allow-dirty-source    Allow creating from a dirty source worktree
  --init-docs             Inspect module PRD/project in the new worktree
  --with-harness          Asynchronously prewarm ./scripts/worktree-harness.sh up in the new worktree
  --pm-owner-role <role>  Create the .pm task inside the target worktree, move it to committed,
                          and record workflow start with this owner role
  --pm-title <title>      Required when using --pm-owner-role
  --pm-priority <P0-P3>   Optional .pm task priority (default: P2)
  --pm-source-ref <ref>   Required when using --pm-owner-role; may be passed multiple times
  --pm-doc-ref <ref>      Optional .pm task doc ref; may be passed multiple times
  --pm-related-prd <id>   Optional .pm related PRD; may be passed multiple times
  --pm-acceptance <text>  Optional .pm acceptance item; may be passed multiple times
  --pm-handoff-to <role>  Optional .pm handoff target; may be passed multiple times
  --json                  Print machine-readable JSON summary only
  -h, --help              Show this help

Examples:
  ./scripts/new-task-worktree.sh scripts task-worktree-bootstrap
  ./scripts/new-task-worktree.sh scripts task-worktree-bootstrap --init-docs
  ./scripts/new-task-worktree.sh engineering task-worktree-pm-bootstrap --pm-owner-role producer_system_designer --pm-title "atomic task worktree bootstrap" --pm-source-ref doc/engineering/project.md
  ./scripts/new-task-worktree.sh viewer hud-redesign --base main
  ./scripts/new-task-worktree.sh p2p hosted-flow --json --path ../worktrees/oasis7-codex-p2p-hosted-flow
  ./scripts/new-task-worktree.sh viewer hud-redesign --with-harness
USAGE
}

wh_require_git_worktree

ALLOW_DIRTY_SOURCE=0
INIT_DOCS=0
WITH_HARNESS=0
OUTPUT_JSON=0
BASE_REF="HEAD"
BRANCH_NAME=""
TARGET_PATH=""
WORKTREES_ROOT=""
PM_BOOTSTRAP=0
PM_OWNER_ROLE=""
PM_TITLE=""
PM_PRIORITY="P2"
declare -a PM_SOURCE_REFS=()
declare -a PM_DOC_REFS=()
declare -a PM_RELATED_PRD=()
declare -a PM_ACCEPTANCE=()
declare -a PM_HANDOFF_TO=()
POSITIONAL=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE_REF="${2:-}"
      shift 2
      ;;
    --branch)
      BRANCH_NAME="${2:-}"
      shift 2
      ;;
    --path)
      TARGET_PATH="${2:-}"
      shift 2
      ;;
    --worktrees-root)
      WORKTREES_ROOT="${2:-}"
      shift 2
      ;;
    --allow-dirty-source)
      ALLOW_DIRTY_SOURCE=1
      shift
      ;;
    --init-docs)
      INIT_DOCS=1
      shift
      ;;
    --with-harness)
      WITH_HARNESS=1
      shift
      ;;
    --pm-owner-role)
      PM_BOOTSTRAP=1
      PM_OWNER_ROLE="${2:-}"
      shift 2
      ;;
    --pm-title)
      PM_BOOTSTRAP=1
      PM_TITLE="${2:-}"
      shift 2
      ;;
    --pm-priority)
      PM_BOOTSTRAP=1
      PM_PRIORITY="${2:-}"
      shift 2
      ;;
    --pm-source-ref)
      PM_BOOTSTRAP=1
      PM_SOURCE_REFS+=("${2:-}")
      shift 2
      ;;
    --pm-doc-ref)
      PM_BOOTSTRAP=1
      PM_DOC_REFS+=("${2:-}")
      shift 2
      ;;
    --pm-related-prd)
      PM_BOOTSTRAP=1
      PM_RELATED_PRD+=("${2:-}")
      shift 2
      ;;
    --pm-acceptance)
      PM_BOOTSTRAP=1
      PM_ACCEPTANCE+=("${2:-}")
      shift 2
      ;;
    --pm-handoff-to)
      PM_BOOTSTRAP=1
      PM_HANDOFF_TO+=("${2:-}")
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
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

if [[ "${#POSITIONAL[@]}" -ne 2 ]]; then
  echo "error: expected <module> and <task>" >&2
  usage >&2
  exit 2
fi

MODULE_INPUT="${POSITIONAL[0]}"
TASK_INPUT="${POSITIONAL[1]}"
[[ -n "$MODULE_INPUT" && -n "$TASK_INPUT" ]] || { echo "error: <module> and <task> cannot be empty" >&2; exit 2; }
[[ -n "$BASE_REF" ]] || { echo "error: --base cannot be empty" >&2; exit 2; }
if [[ "$PM_BOOTSTRAP" == "1" ]]; then
  [[ -n "$PM_OWNER_ROLE" ]] || { echo "error: --pm-owner-role is required when bootstrapping .pm" >&2; exit 2; }
  [[ -n "$PM_TITLE" ]] || { echo "error: --pm-title is required when bootstrapping .pm" >&2; exit 2; }
  [[ "$PM_PRIORITY" =~ ^P[0-3]$ ]] || { echo "error: --pm-priority must be one of P0,P1,P2,P3" >&2; exit 2; }
  if [[ "${#PM_SOURCE_REFS[@]}" -eq 0 ]]; then
    echo "error: at least one --pm-source-ref is required when bootstrapping .pm" >&2
    exit 2
  fi
fi

slugify() {
  python3 - "$1" <<'PY'
from __future__ import annotations

import re
import sys

value = sys.argv[1].strip().lower()
value = re.sub(r"[^a-z0-9]+", "-", value)
value = re.sub(r"-{2,}", "-", value).strip("-")
print(value)
PY
}

resolve_abs_path() {
  python3 - "$CALLER_DIR" "$1" <<'PY'
from __future__ import annotations

from pathlib import Path
import sys

base = Path(sys.argv[1]).resolve()
raw = Path(sys.argv[2])
if raw.is_absolute():
    print(raw.resolve())
else:
    print((base / raw).resolve())
PY
}

worktree_id_for_path() {
  python3 - "$1" <<'PY'
from __future__ import annotations

import hashlib
from pathlib import Path
import sys

path = Path(sys.argv[1]).resolve()
digest = hashlib.sha256(str(path).encode("utf-8")).hexdigest()[:8]
print(f"wt-{digest}")
PY
}

branch_checkout_path() {
  python3 - "$COMMON_GIT_DIR" "$1" <<'PY'
from __future__ import annotations

import subprocess
import sys

git_dir = sys.argv[1]
target = f"refs/heads/{sys.argv[2]}"
current: dict[str, str] = {}
raw = subprocess.check_output(
    ["git", f"--git-dir={git_dir}", "worktree", "list", "--porcelain"],
    text=True,
)

def emit(record: dict[str, str]) -> None:
    if record.get("branch") == target:
        print(record.get("worktree", ""))
        raise SystemExit(0)

for line in raw.splitlines():
    if not line:
      if current:
        emit(current)
        current = {}
      continue
    key, _, value = line.partition(" ")
    current[key] = value

if current:
    emit(current)

raise SystemExit(1)
PY
}

extract_json_field() {
  local key="$1"
  local payload="$2"
  JSON_PAYLOAD="$payload" python3 - "$key" <<'PY'
from __future__ import annotations

import json
import os
import sys

payload = json.loads(os.environ["JSON_PAYLOAD"])
value = payload
for part in sys.argv[1].split("."):
    value = value[part]
if isinstance(value, bool):
    print("true" if value else "false")
elif value is None:
    print("")
else:
    print(value)
PY
}

MODULE_SLUG="$(slugify "$MODULE_INPUT")"
TASK_SLUG="$(slugify "$TASK_INPUT")"
[[ -n "$MODULE_SLUG" ]] || { echo "error: <module> becomes empty after slug normalization" >&2; exit 2; }
[[ -n "$TASK_SLUG" ]] || { echo "error: <task> becomes empty after slug normalization" >&2; exit 2; }

REPO_ROOT="$(wh_repo_root)"
COMMON_GIT_DIR="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
CANONICAL_REPO_ROOT="$(cd "$COMMON_GIT_DIR/.." && pwd -P)"
CURRENT_REPO_PARENT_NAME="$(basename "$(dirname "$REPO_ROOT")")"
FAMILY_REPO_NAME="$(git config --local --get oasis7.task-worktree-family-name 2>/dev/null || true)"
FAMILY_WORKTREES_ROOT="$(git config --local --get oasis7.task-worktrees-root 2>/dev/null || true)"
if [[ -z "$FAMILY_REPO_NAME" ]]; then
  if [[ "$CURRENT_REPO_PARENT_NAME" == "worktrees" ]]; then
    FAMILY_REPO_NAME="$(basename "$CANONICAL_REPO_ROOT")"
  else
    FAMILY_REPO_NAME="$(basename "$REPO_ROOT")"
  fi
fi
if [[ -z "$FAMILY_WORKTREES_ROOT" ]]; then
  if [[ "$CURRENT_REPO_PARENT_NAME" == "worktrees" ]]; then
    FAMILY_WORKTREES_ROOT="$(dirname "$CANONICAL_REPO_ROOT")/worktrees"
  else
    FAMILY_WORKTREES_ROOT="$(dirname "$REPO_ROOT")/worktrees"
  fi
fi
if [[ -n "$WORKTREES_ROOT" ]]; then
  WORKTREES_ROOT="$(resolve_abs_path "$WORKTREES_ROOT")"
else
  WORKTREES_ROOT="$(resolve_abs_path "$FAMILY_WORKTREES_ROOT")"
fi

if [[ -z "$BRANCH_NAME" ]]; then
  BRANCH_NAME="task/${MODULE_SLUG}-${TASK_SLUG}"
fi

if [[ -n "$TARGET_PATH" ]]; then
  TARGET_PATH="$(resolve_abs_path "$TARGET_PATH")"
else
  TARGET_PATH="$(resolve_abs_path "$WORKTREES_ROOT/$FAMILY_REPO_NAME-$MODULE_SLUG-$TASK_SLUG")"
fi

if [[ "$ALLOW_DIRTY_SOURCE" != "1" ]] && [[ -n "$(git status --short)" ]]; then
  echo "error: source worktree is dirty; commit/stash changes first or rerun with --allow-dirty-source" >&2
  exit 1
fi

if ! git rev-parse --verify --quiet "$BASE_REF^{commit}" >/dev/null; then
  echo "error: base ref not found: $BASE_REF" >&2
  exit 1
fi

if [[ -e "$TARGET_PATH" ]]; then
  echo "error: target worktree path already exists: $TARGET_PATH" >&2
  echo "hint: choose a different task slug/path or remove the old directory first" >&2
  exit 1
fi

if existing_branch_path="$(branch_checkout_path "$BRANCH_NAME" 2>/dev/null)"; then
  echo "error: branch is already checked out in another worktree: $BRANCH_NAME" >&2
  echo "hint: existing worktree path: $existing_branch_path" >&2
  exit 1
fi

mkdir -p "$(dirname "$TARGET_PATH")"

MODE="create_new_branch"
if git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
  MODE="attach_existing_branch"
  git worktree add --quiet "$TARGET_PATH" "$BRANCH_NAME"
else
  git worktree add --quiet -b "$BRANCH_NAME" "$TARGET_PATH" "$BASE_REF"
fi
git -C "$TARGET_PATH" config oasis7.task-worktree-family-name "$FAMILY_REPO_NAME"
git -C "$TARGET_PATH" config oasis7.task-worktrees-root "$WORKTREES_ROOT"

cleanup_bootstrap_failure() {
  git worktree remove --force "$TARGET_PATH" >/dev/null 2>&1 || true
  if [[ "$MODE" == "create_new_branch" ]]; then
    git branch -D "$BRANCH_NAME" >/dev/null 2>&1 || true
  fi
}

DOC_PRD_PATH=""
DOC_PROJECT_PATH=""
DOC_PRD_EXISTS=0
DOC_PROJECT_EXISTS=0
if [[ "$INIT_DOCS" == "1" ]]; then
  DOC_PRD_PATH="$TARGET_PATH/doc/$MODULE_SLUG/prd.md"
  DOC_PROJECT_PATH="$TARGET_PATH/doc/$MODULE_SLUG/project.md"
  [[ -f "$DOC_PRD_PATH" ]] && DOC_PRD_EXISTS=1
  [[ -f "$DOC_PROJECT_PATH" ]] && DOC_PROJECT_EXISTS=1
fi

HARNESS_STATE_FILE=""
HARNESS_BOOTSTRAP_LOG=""
HARNESS_STATUS=""
HARNESS_VIEWER_URL=""
if [[ "$WITH_HARNESS" == "1" ]]; then
  HARNESS_WORKTREE_ID="$(worktree_id_for_path "$TARGET_PATH")"
  HARNESS_STATE_FILE="$TARGET_PATH/output/harness/$HARNESS_WORKTREE_ID/state.json"
  HARNESS_BOOTSTRAP_LOG="$TARGET_PATH/output/harness/new-task-worktree-harness.log"
  mkdir -p "$(dirname "$HARNESS_BOOTSTRAP_LOG")"
  (
    cd "$TARGET_PATH"
    nohup ./scripts/worktree-harness.sh up >"$HARNESS_BOOTSTRAP_LOG" 2>&1 < /dev/null &
  )
  for _ in $(seq 1 5); do
    [[ -f "$HARNESS_STATE_FILE" ]] && break
    sleep 1
  done
  HARNESS_STATUS="$(wh_state_get "$HARNESS_STATE_FILE" status 2>/dev/null || true)"
  HARNESS_VIEWER_URL="$(wh_state_get "$HARNESS_STATE_FILE" viewer_url 2>/dev/null || true)"
  [[ -n "$HARNESS_STATUS" ]] || HARNESS_STATUS="booting"
fi

PM_TASK_JSON=""
PM_TASK_UID=""
PM_TASK_PATH=""
PM_EXECUTION_LOG_PATH=""
if [[ "$PM_BOOTSTRAP" == "1" ]]; then
  NEW_TASK_CMD=(./scripts/pm/new-task.sh
    --owner-role "$PM_OWNER_ROLE"
    --title "$PM_TITLE"
    --priority "$PM_PRIORITY"
  )
  for source_ref in "${PM_SOURCE_REFS[@]}"; do
    NEW_TASK_CMD+=(--source-ref "$source_ref")
  done
  for doc_ref in "${PM_DOC_REFS[@]}"; do
    NEW_TASK_CMD+=(--doc-ref "$doc_ref")
  done
  for related_prd in "${PM_RELATED_PRD[@]}"; do
    NEW_TASK_CMD+=(--related-prd "$related_prd")
  done
  for acceptance in "${PM_ACCEPTANCE[@]}"; do
    NEW_TASK_CMD+=(--acceptance "$acceptance")
  done
  for handoff_role in "${PM_HANDOFF_TO[@]}"; do
    NEW_TASK_CMD+=(--handoff-to "$handoff_role")
  done
  NEW_TASK_CMD+=(--worktree-hint "$TARGET_PATH" --json)

  set +e
  PM_TASK_JSON="$(
    cd "$TARGET_PATH" &&
    "${NEW_TASK_CMD[@]}"
  )"
  BOOTSTRAP_STATUS=$?
  set -e
  if [[ "$BOOTSTRAP_STATUS" -ne 0 ]]; then
    cleanup_bootstrap_failure
    echo "error: failed to bootstrap .pm task inside target worktree; cleaned up created worktree" >&2
    exit "$BOOTSTRAP_STATUS"
  fi

  PM_TASK_UID="$(extract_json_field task_uid "$PM_TASK_JSON")"
  PM_TASK_PATH="$(extract_json_field task_path "$PM_TASK_JSON")"
  PM_EXECUTION_LOG_PATH="$(extract_json_field execution_log_path "$PM_TASK_JSON")"

  set +e
  (
    cd "$TARGET_PATH" &&
    ./scripts/pm/move-task.sh --task-uid "$PM_TASK_UID" --to-status committed >/dev/null &&
    ./scripts/pm/workflow-report.sh --phase start --role "$PM_OWNER_ROLE" --task-uid "$PM_TASK_UID" >/dev/null
  )
  BOOTSTRAP_STATUS=$?
  set -e
  if [[ "$BOOTSTRAP_STATUS" -ne 0 ]]; then
    cleanup_bootstrap_failure
    echo "error: failed to move/start bootstrapped .pm task inside target worktree; cleaned up created worktree" >&2
    exit "$BOOTSTRAP_STATUS"
  fi
fi

SUMMARY_JSON="$(python3 - "$MODULE_INPUT" "$TASK_INPUT" "$MODULE_SLUG" "$TASK_SLUG" "$BRANCH_NAME" "$TARGET_PATH" "$BASE_REF" "$MODE" "$REPO_ROOT" "$FAMILY_REPO_NAME" "$WORKTREES_ROOT" "$INIT_DOCS" "$DOC_PRD_PATH" "$DOC_PRD_EXISTS" "$DOC_PROJECT_PATH" "$DOC_PROJECT_EXISTS" "$WITH_HARNESS" "$HARNESS_BOOTSTRAP_LOG" "$HARNESS_STATE_FILE" "$HARNESS_STATUS" "$HARNESS_VIEWER_URL" "$PM_BOOTSTRAP" "$PM_OWNER_ROLE" "$PM_TITLE" "$PM_PRIORITY" "$PM_TASK_UID" "$PM_TASK_PATH" "$PM_EXECUTION_LOG_PATH" <<'PY'
from __future__ import annotations

import json
import sys

payload = {
    "module": sys.argv[1],
    "task": sys.argv[2],
    "module_slug": sys.argv[3],
    "task_slug": sys.argv[4],
    "branch": sys.argv[5],
    "worktree_path": sys.argv[6],
    "base_ref": sys.argv[7],
    "mode": sys.argv[8],
    "repo_root": sys.argv[9],
    "repo_name": sys.argv[10],
    "worktrees_root": sys.argv[11],
}
if sys.argv[12] == "1":
    payload["doc_checks"] = {
        "prd": {"path": sys.argv[13], "exists": sys.argv[14] == "1"},
        "project": {"path": sys.argv[15], "exists": sys.argv[16] == "1"},
    }
if sys.argv[17] == "1":
    payload["harness"] = {
        "bootstrap_log": sys.argv[18],
        "state_file": sys.argv[19],
        "status": sys.argv[20],
        "viewer_url": sys.argv[21],
    }
if sys.argv[22] == "1":
    payload["pm_task"] = {
        "owner_role": sys.argv[23],
        "title": sys.argv[24],
        "priority": sys.argv[25],
        "task_uid": sys.argv[26],
        "task_path": sys.argv[27],
        "execution_log_path": sys.argv[28],
        "status": "committed",
        "workflow_started": True,
    }
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$SUMMARY_JSON"
  exit 0
fi

cat <<INFO
Task worktree is ready.
- module: $MODULE_INPUT
- task: $TASK_INPUT
- branch: $BRANCH_NAME
- path: $TARGET_PATH
- repo family: $FAMILY_REPO_NAME
- worktrees root: $WORKTREES_ROOT
- base ref: $BASE_REF
- mode: $MODE

INFO

if [[ "$INIT_DOCS" == "1" ]]; then
  cat <<INFO

Docs bootstrap:
- module PRD: $([[ "$DOC_PRD_EXISTS" == "1" ]] && printf 'present' || printf 'missing') ($DOC_PRD_PATH)
- module project: $([[ "$DOC_PROJECT_EXISTS" == "1" ]] && printf 'present' || printf 'missing') ($DOC_PROJECT_PATH)
INFO
fi

if [[ "$WITH_HARNESS" == "1" ]]; then
  cat <<INFO

Harness bootstrap:
- status: ${HARNESS_STATUS:-unknown}
- bootstrap log: $HARNESS_BOOTSTRAP_LOG
- state file: $HARNESS_STATE_FILE
- viewer url: ${HARNESS_VIEWER_URL:-unavailable}
INFO
fi

if [[ "$PM_BOOTSTRAP" == "1" ]]; then
  cat <<INFO

PM bootstrap:
- owner role: $PM_OWNER_ROLE
- task uid: $PM_TASK_UID
- task path: $PM_TASK_PATH
- execution log: $PM_EXECUTION_LOG_PATH
- task status: committed
- workflow start: recorded
INFO
fi

cat <<INFO

Next:
  cd $TARGET_PATH
INFO

if [[ "$PM_BOOTSTRAP" == "1" ]]; then
  printf '  sed -n '\''1,200p'\'' %s\n' "$PM_EXECUTION_LOG_PATH"
else
  cat <<INFO
  ./scripts/pm/workflow-report.sh --phase start --role <owner_role> --task-uid <TASK-UID>
  sed -n '1,200p' .pm/tasks/<TASK-UID>.execution.md
INFO
fi

if [[ "$INIT_DOCS" == "1" ]]; then
  if [[ "$DOC_PRD_EXISTS" == "1" ]]; then
    printf '  sed -n '\''1,160p'\'' %s\n' "${DOC_PRD_PATH#$TARGET_PATH/}"
  else
    printf '  mkdir -p %s\n' "doc/$MODULE_SLUG"
    printf '  # create %s\n' "${DOC_PRD_PATH#$TARGET_PATH/}"
  fi
  if [[ "$DOC_PROJECT_EXISTS" == "1" ]]; then
    printf '  sed -n '\''1,160p'\'' %s\n' "${DOC_PROJECT_PATH#$TARGET_PATH/}"
  else
    printf '  # create %s\n' "${DOC_PROJECT_PATH#$TARGET_PATH/}"
  fi
else
  printf '  sed -n '\''1,160p'\'' %s\n' "doc/$MODULE_SLUG/prd.md"
  printf '  sed -n '\''1,160p'\'' %s\n' "doc/$MODULE_SLUG/project.md"
fi
if [[ "$WITH_HARNESS" == "1" ]]; then
  printf '  ./scripts/worktree-harness.sh status --json\n'
fi
cat <<INFO
  git status -sb
INFO
