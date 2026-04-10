#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/prepare-task-pr.sh [source-branch] [options]

Validate one task branch for GitHub PR closure, print the exact PR command, and
optionally push the branch plus open the PR through `gh`.

Default conventions:
- source branch: current branch
- base branch: main
- remote: origin
- standard path: snapshot review -> commit -> prepare-task-pr -> GitHub PR

Options:
  --base <branch>         Base branch for the PR (default: main)
  --remote <name>         Remote name for push / base comparison (default: origin)
  --create                Push branch if needed and run `gh pr create`
  --draft                 Add `--draft` when creating the PR
  --title <text>          Explicit PR title (default: use gh --fill)
  --body-file <path>      Pass an explicit PR body file to `gh pr create`
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help

Examples:
  ./scripts/prepare-task-pr.sh
  ./scripts/prepare-task-pr.sh task/engineering-github-pr-landing-governance --json
  ./scripts/prepare-task-pr.sh --create --draft
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

infer_branch_from_head() {
  python3 - <<'PY'
from __future__ import annotations

import subprocess

branches = [
    line.strip()
    for line in subprocess.check_output(
        [
            "git",
            "for-each-ref",
            "--format=%(refname:short)",
            "--points-at",
            "HEAD",
            "refs/heads",
        ],
        text=True,
    ).splitlines()
    if line.strip()
]

if len(branches) == 1:
    print(branches[0])
PY
}

wh_require_git_worktree

BASE_BRANCH="main"
REMOTE_NAME="origin"
CREATE_PR=0
DRAFT_PR=0
OUTPUT_JSON=0
PR_TITLE=""
BODY_FILE=""
POSITIONAL=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE_BRANCH="${2:-}"
      shift 2
      ;;
    --remote)
      REMOTE_NAME="${2:-}"
      shift 2
      ;;
    --create)
      CREATE_PR=1
      shift
      ;;
    --draft)
      DRAFT_PR=1
      shift
      ;;
    --title)
      PR_TITLE="${2:-}"
      shift 2
      ;;
    --body-file)
      BODY_FILE="${2:-}"
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

if [[ "${#POSITIONAL[@]}" -gt 1 ]]; then
  die "expected at most one optional [source-branch]"
fi

COMMON_GIT_DIR="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
CANONICAL_REPO_ROOT="$(cd "$COMMON_GIT_DIR/.." && pwd -P)"
CURRENT_BRANCH="$(git branch --show-current)"
SOURCE_BRANCH="${POSITIONAL[0]:-}"

if [[ -z "$SOURCE_BRANCH" ]]; then
  if [[ -z "$CURRENT_BRANCH" ]]; then
    CURRENT_BRANCH="$(infer_branch_from_head)"
  fi
  [[ -n "$CURRENT_BRANCH" ]] || die "detached HEAD; pass [source-branch] explicitly"
  SOURCE_BRANCH="$CURRENT_BRANCH"
fi

[[ -n "$BASE_BRANCH" ]] || die "--base cannot be empty"
[[ -n "$REMOTE_NAME" ]] || die "--remote cannot be empty"
[[ "$SOURCE_BRANCH" != "$BASE_BRANCH" ]] || die "source and base branches must differ"

if [[ -n "$BODY_FILE" && ! -f "$BODY_FILE" ]]; then
  die "--body-file not found: $BODY_FILE"
fi

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

render_cmd() {
  python3 - "$@" <<'PY'
from __future__ import annotations

import shlex
import sys

print(" ".join(shlex.quote(arg) for arg in sys.argv[1:]))
PY
}

ensure_branch_exists "$SOURCE_BRANCH"
SOURCE_HEAD="$(git rev-parse "refs/heads/$SOURCE_BRANCH^{commit}")"
CURRENT_HEAD="$(git rev-parse HEAD^{commit})"

SOURCE_WORKTREE="$(branch_checkout_path "$SOURCE_BRANCH" 2>/dev/null || true)"
if [[ -z "$SOURCE_WORKTREE" && "$CURRENT_HEAD" == "$SOURCE_HEAD" ]]; then
  SOURCE_WORKTREE="$(pwd -P)"
fi
[[ -n "$SOURCE_WORKTREE" ]] || die "source branch is not checked out in any worktree: $SOURCE_BRANCH"
ensure_clean_worktree "$SOURCE_WORKTREE" "source"

if [[ "$CREATE_PR" == "1" ]]; then
  git fetch --quiet "$REMOTE_NAME" "$BASE_BRANCH"
fi

LOCAL_BASE_REF=""
REMOTE_BASE_REF=""
if git show-ref --verify --quiet "refs/heads/$BASE_BRANCH"; then
  LOCAL_BASE_REF="refs/heads/$BASE_BRANCH"
fi
if git show-ref --verify --quiet "refs/remotes/$REMOTE_NAME/$BASE_BRANCH"; then
  REMOTE_BASE_REF="refs/remotes/$REMOTE_NAME/$BASE_BRANCH"
fi

COMPARISON_REF="$REMOTE_BASE_REF"
if [[ -z "$COMPARISON_REF" ]]; then
  COMPARISON_REF="$LOCAL_BASE_REF"
fi
[[ -n "$COMPARISON_REF" ]] || die "neither local nor remote base ref exists for $BASE_BRANCH"

COMPARISON_HEAD="$(git rev-parse "$COMPARISON_REF^{commit}")"
BASE_WORKTREE=""
if [[ -n "$LOCAL_BASE_REF" ]]; then
  BASE_WORKTREE="$(branch_checkout_path "$BASE_BRANCH" 2>/dev/null || true)"
fi

read -r BEHIND_COUNT AHEAD_COUNT <<<"$(git rev-list --left-right --count "$COMPARISON_REF...$SOURCE_BRANCH")"
if git merge-base --is-ancestor "$COMPARISON_REF" "$SOURCE_BRANCH"; then
  REBASE_REQUIRED=0
else
  REBASE_REQUIRED=1
fi

REMOTE_SOURCE_REF=""
if git show-ref --verify --quiet "refs/remotes/$REMOTE_NAME/$SOURCE_BRANCH"; then
  REMOTE_SOURCE_REF="refs/remotes/$REMOTE_NAME/$SOURCE_BRANCH"
fi

UPSTREAM_REF="$(git rev-parse --abbrev-ref --symbolic-full-name "$SOURCE_BRANCH@{upstream}" 2>/dev/null || true)"
LOCAL_ONLY_COUNT="$AHEAD_COUNT"
REMOTE_ONLY_COUNT=0
if [[ -n "$REMOTE_SOURCE_REF" ]]; then
  read -r REMOTE_ONLY_COUNT LOCAL_ONLY_COUNT <<<"$(git rev-list --left-right --count "$REMOTE_SOURCE_REF...$SOURCE_BRANCH")"
fi

CREATE_CMD=("gh" "pr" "create" "--base" "$BASE_BRANCH" "--head" "$SOURCE_BRANCH")
if [[ -n "$PR_TITLE" ]]; then
  CREATE_CMD+=("--title" "$PR_TITLE")
else
  CREATE_CMD+=("--fill")
fi
if [[ -n "$BODY_FILE" ]]; then
  CREATE_CMD+=("--body-file" "$BODY_FILE")
fi
if [[ "$DRAFT_PR" == "1" ]]; then
  CREATE_CMD+=("--draft")
fi
CREATE_CMD_RENDERED="$(render_cmd "${CREATE_CMD[@]}")"

SYNC_CMD=""
if [[ -n "$BASE_WORKTREE" ]]; then
  SYNC_CMD="git -C $BASE_WORKTREE pull --ff-only $REMOTE_NAME $BASE_BRANCH"
fi
CLEANUP_CMD_1="git -C $CANONICAL_REPO_ROOT worktree remove -f $SOURCE_WORKTREE"
CLEANUP_CMD_2="git -C $CANONICAL_REPO_ROOT branch -D $SOURCE_BRANCH"

PR_URL=""
if [[ "$CREATE_PR" == "1" ]]; then
  command -v gh >/dev/null 2>&1 || die "`gh` not found in PATH"
  if [[ "$REBASE_REQUIRED" == "1" ]]; then
    die "source branch is behind $COMPARISON_REF; rebase before creating the PR"
  fi
  if [[ -z "$REMOTE_SOURCE_REF" ]]; then
    git -C "$SOURCE_WORKTREE" push -u "$REMOTE_NAME" "$SOURCE_BRANCH"
  elif [[ "$LOCAL_ONLY_COUNT" != "0" || "$REMOTE_ONLY_COUNT" != "0" ]]; then
    git -C "$SOURCE_WORKTREE" push "$REMOTE_NAME" "$SOURCE_BRANCH"
  fi
  PR_URL="$("${CREATE_CMD[@]}")"
fi

SUMMARY_JSON="$(
python3 - "$SOURCE_BRANCH" "$SOURCE_WORKTREE" "$SOURCE_HEAD" "$BASE_BRANCH" "$COMPARISON_REF" "$COMPARISON_HEAD" "$REMOTE_NAME" "$AHEAD_COUNT" "$BEHIND_COUNT" "$REBASE_REQUIRED" "$UPSTREAM_REF" "$LOCAL_ONLY_COUNT" "$REMOTE_ONLY_COUNT" "$CREATE_CMD_RENDERED" "$SYNC_CMD" "$CLEANUP_CMD_1" "$CLEANUP_CMD_2" "$PR_URL" <<'PY'
from __future__ import annotations

import json
import sys

payload = {
    "source_branch": sys.argv[1],
    "source_worktree": sys.argv[2],
    "source_head": sys.argv[3],
    "base_branch": sys.argv[4],
    "comparison_ref": sys.argv[5],
    "comparison_head": sys.argv[6],
    "remote_name": sys.argv[7],
    "ahead_count": int(sys.argv[8]),
    "behind_count": int(sys.argv[9]),
    "rebase_required": sys.argv[10] == "1",
    "upstream_ref": sys.argv[11] or None,
    "unpushed_commit_count": int(sys.argv[12]),
    "remote_only_commit_count": int(sys.argv[13]),
    "create_command": sys.argv[14],
    "post_merge_commands": [cmd for cmd in sys.argv[15:18] if cmd],
    "cleanup_commands": [cmd for cmd in sys.argv[15:18] if cmd],
    "pr_url": sys.argv[18] or None,
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$SUMMARY_JSON"
  exit 0
fi

REBASE_NOTE="no"
if [[ "$REBASE_REQUIRED" == "1" ]]; then
  REBASE_NOTE="yes"
fi

cat <<INFO
Task PR preflight summary:
- source branch: $SOURCE_BRANCH
- source worktree: $SOURCE_WORKTREE
- source head: $SOURCE_HEAD
- base branch: $BASE_BRANCH
- comparison ref: $COMPARISON_REF
- remote: $REMOTE_NAME
- ahead of base: $AHEAD_COUNT
- behind base: $BEHIND_COUNT
- rebase required: $REBASE_NOTE
- upstream: ${UPSTREAM_REF:-"(none)"}
- unpushed commits: $LOCAL_ONLY_COUNT
- remote-only commits on source: $REMOTE_ONLY_COUNT
- create command: $CREATE_CMD_RENDERED
INFO

if [[ "$REBASE_REQUIRED" == "1" ]]; then
  echo
  echo "Suggested rebase:"
  echo "  git -C $SOURCE_WORKTREE rebase $COMPARISON_REF"
fi

if [[ "$CREATE_PR" == "1" ]]; then
  echo
  echo "Created PR:"
  echo "  $PR_URL"
fi

echo
echo "Post-Merge Cleanup:"
if [[ -n "$SYNC_CMD" ]]; then
  echo "  $SYNC_CMD"
else
  echo "  sync local $BASE_BRANCH manually in the worktree that keeps it checked out"
fi
echo "  $CLEANUP_CMD_1"
echo "  $CLEANUP_CMD_2"
