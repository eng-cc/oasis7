#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/land-task-worktree.sh [source-branch] [options]

Compatibility helper for local-only landing.

Rebase one task branch onto a checked-out target branch and fast-forward land it.
This is no longer the default final integration path; standard closure should go
through `./scripts/prepare-task-pr.sh` and a GitHub PR into protected `main`.

Default conventions:
- source branch: current branch
- target branch: local main
- strategy: rebase source onto target, then merge --ff-only source into target

Options:
  --target <branch>       Target branch to land into (default: local main)
  --dry-run               Validate and report landing plan without mutating git history
  --json                  Print machine-readable JSON summary only
  -h, --help              Show this help

Examples:
  ./scripts/land-task-worktree.sh
  ./scripts/land-task-worktree.sh task/scripts-main-merge-workflow --target main
  ./scripts/land-task-worktree.sh --dry-run --json
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

wh_require_git_worktree

TARGET_BRANCH="main"
DRY_RUN=0
OUTPUT_JSON=0
POSITIONAL=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET_BRANCH="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
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
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

if [[ "${#POSITIONAL[@]}" -gt 1 ]]; then
  die "expected at most one optional [source-branch]"
fi

COMMON_GIT_DIR="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
CANONICAL_REPO_ROOT="$(cd "$COMMON_GIT_DIR/.." && pwd -P)"
CURRENT_BRANCH="$(git branch --show-current)"
SOURCE_BRANCH="${POSITIONAL[0]:-}"

if [[ -z "$SOURCE_BRANCH" ]]; then
  [[ -n "$CURRENT_BRANCH" ]] || die "detached HEAD; pass [source-branch] explicitly"
  SOURCE_BRANCH="$CURRENT_BRANCH"
fi

[[ -n "$TARGET_BRANCH" ]] || die "--target cannot be empty"
[[ "$SOURCE_BRANCH" != "$TARGET_BRANCH" ]] || die "source and target branches must differ"

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

branch_head() {
  git rev-parse "refs/heads/$1^{commit}"
}

ensure_branch_exists() {
  git show-ref --verify --quiet "refs/heads/$1" || die "branch not found: $1"
}

ensure_clean_worktree() {
  local worktree_path=$1
  local label=$2
  if [[ -n "$(git -C "$worktree_path" status --short)" ]]; then
    die "$label worktree is dirty: $worktree_path"
  fi
}

ensure_branch_exists "$SOURCE_BRANCH"
ensure_branch_exists "$TARGET_BRANCH"

SOURCE_WORKTREE="$(branch_checkout_path "$SOURCE_BRANCH" 2>/dev/null || true)"
TARGET_WORKTREE="$(branch_checkout_path "$TARGET_BRANCH" 2>/dev/null || true)"

[[ -n "$SOURCE_WORKTREE" ]] || die "source branch is not checked out in any worktree: $SOURCE_BRANCH"
[[ -n "$TARGET_WORKTREE" ]] || die "target branch is not checked out in any worktree: $TARGET_BRANCH"

ensure_clean_worktree "$SOURCE_WORKTREE" "source"
ensure_clean_worktree "$TARGET_WORKTREE" "target"

SOURCE_HEAD_BEFORE="$(branch_head "$SOURCE_BRANCH")"
TARGET_HEAD_BEFORE="$(branch_head "$TARGET_BRANCH")"
SOURCE_HEAD_AFTER="$SOURCE_HEAD_BEFORE"
TARGET_HEAD_AFTER="$TARGET_HEAD_BEFORE"
REBASE_STATUS="not_needed"
RESULT="pending"

if git merge-base --is-ancestor "$SOURCE_BRANCH" "$TARGET_BRANCH"; then
  RESULT="already_landed"
fi

if [[ "$RESULT" != "already_landed" ]]; then
  if git merge-base --is-ancestor "$TARGET_BRANCH" "$SOURCE_BRANCH"; then
    REBASE_STATUS="already_up_to_date"
  elif [[ "$DRY_RUN" == "1" ]]; then
    REBASE_STATUS="would_rebase"
  else
    if ! git -C "$SOURCE_WORKTREE" rebase --quiet "$TARGET_BRANCH" >/dev/null; then
      echo "hint: resolve rebase conflicts in $SOURCE_WORKTREE, then rerun landing" >&2
      exit 1
    fi
    REBASE_STATUS="rebased"
    SOURCE_HEAD_AFTER="$(branch_head "$SOURCE_BRANCH")"
  fi

  if [[ "$DRY_RUN" == "1" ]]; then
    RESULT="dry_run"
  else
    if ! git merge-base --is-ancestor "$TARGET_BRANCH" "$SOURCE_BRANCH"; then
      die "source branch is still not a fast-forward candidate after rebase: $SOURCE_BRANCH"
    fi
    if ! git -C "$TARGET_WORKTREE" merge --ff-only --quiet "$SOURCE_BRANCH" >/dev/null; then
      echo "hint: inspect $TARGET_WORKTREE and $SOURCE_WORKTREE before retrying landing" >&2
      exit 1
    fi
    TARGET_HEAD_AFTER="$(branch_head "$TARGET_BRANCH")"
    RESULT="landed"
  fi
fi

CLEANUP_CMD_1="git -C $CANONICAL_REPO_ROOT worktree remove -f $SOURCE_WORKTREE"
CLEANUP_CMD_2="git -C $CANONICAL_REPO_ROOT branch -d $SOURCE_BRANCH"

SUMMARY_JSON="$(python3 - "$SOURCE_BRANCH" "$SOURCE_WORKTREE" "$TARGET_BRANCH" "$TARGET_WORKTREE" "$SOURCE_HEAD_BEFORE" "$SOURCE_HEAD_AFTER" "$TARGET_HEAD_BEFORE" "$TARGET_HEAD_AFTER" "$REBASE_STATUS" "$RESULT" "$DRY_RUN" "$CLEANUP_CMD_1" "$CLEANUP_CMD_2" <<'PY'
from __future__ import annotations

import json
import sys

payload = {
    "source_branch": sys.argv[1],
    "source_worktree": sys.argv[2],
    "target_branch": sys.argv[3],
    "target_worktree": sys.argv[4],
    "source_head_before": sys.argv[5],
    "source_head_after": sys.argv[6],
    "target_head_before": sys.argv[7],
    "target_head_after": sys.argv[8],
    "rebase_status": sys.argv[9],
    "result": sys.argv[10],
    "dry_run": sys.argv[11] == "1",
    "cleanup_commands": [sys.argv[12], sys.argv[13]],
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$SUMMARY_JSON"
  exit 0
fi

cat <<INFO
Local task worktree landing summary:
- source branch: $SOURCE_BRANCH
- source worktree: $SOURCE_WORKTREE
- target branch: $TARGET_BRANCH
- target worktree: $TARGET_WORKTREE
- source head before: $SOURCE_HEAD_BEFORE
- source head after: $SOURCE_HEAD_AFTER
- target head before: $TARGET_HEAD_BEFORE
- target head after: $TARGET_HEAD_AFTER
- rebase status: $REBASE_STATUS
- result: $RESULT

Required Cleanup:
  $CLEANUP_CMD_1
  $CLEANUP_CMD_2
INFO
