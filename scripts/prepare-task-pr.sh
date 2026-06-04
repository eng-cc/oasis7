#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/prepare-task-pr.sh [source-branch] [options]

Validate one task branch for GitHub PR closure, print the exact PR command, and
optionally push the branch plus open the PR through `gh`. The preflight summary
also reports a local required-gate validation recommendation, a claim-ready
helper command for fresh PR-readiness verification, local role-review evidence
status, plus planner reason summary derived from the current changed-path scope.
After PR creation, the default workflow continues into required-check/comment/
mergeability watch, failure fixes, merge, and cleanup unless the task explicitly
records that the PR exists only to run manual-trigger packaging/release CI.
REVIEW_REQUIRED is reported as status but is not a blocking item by itself.

Default conventions:
- source branch: current branch
- base branch: main
- remote: origin
- standard path: commit -> local role-subagent review -> prepare-task-pr -> GitHub PR watch/fix/merge

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

plan_kv_get() {
  local output="$1"
  local key=""
  key="$2"
  printf '%s\n' "$output" | sed -n "s/^${key}=//p" | head -n 1
}

plan_kv_get_default() {
  local output="$1"
  local key="$2"
  local default_value="$3"
  local value=""
  value="$(plan_kv_get "$output" "$key")"
  printf '%s\n' "${value:-$default_value}"
}

local_role_review_status() {
  local source_worktree="$1"
  local source_branch="$2"
  local source_head="$3"
  local comparison_ref="$4"
  python3 - "$source_worktree" "$source_branch" "$source_head" "$comparison_ref" <<'PY'
from __future__ import annotations

from pathlib import Path
import re
import subprocess
import sys

source_worktree = Path(sys.argv[1]).resolve()
source_branch = sys.argv[2]
source_head = sys.argv[3]
comparison_ref = sys.argv[4]
root = source_worktree
tasks_dir = root / ".pm" / "tasks"

def parse_field(text: str, key: str) -> str:
    match = re.search(rf"^- {re.escape(key)}: (.+)$", text, re.MULTILINE)
    return match.group(1).strip() if match else ""

def review_packet_blocks(text: str) -> list[str]:
    lines = text.splitlines()
    blocks: list[str] = []
    current: list[str] = []
    in_block = False
    for line in lines:
        if line.startswith("## "):
            if in_block and current:
                blocks.append("\n".join(current))
            current = [line]
            in_block = False
            continue
        if current:
            current.append(line)
            if line == "- Pre-PR Local Role Review: passed":
                in_block = True
    if in_block and current:
        blocks.append("\n".join(current))
    return blocks

def emit(
    status: str,
    task_uid: str = "",
    log_path: str = "",
    reason: str = "",
    missing_markers: list[str] | None = None,
    review_roles: str = "",
    findings_disposition: str = "",
    residual_risk: str = "",
) -> None:
    print(f"status={status}")
    print(f"task_uid={task_uid}")
    print(f"execution_log_path={log_path}")
    print(f"reason={reason}")
    print(f"missing_markers={';'.join(missing_markers or [])}")
    print(f"review_roles={review_roles}")
    print(f"findings_disposition={findings_disposition}")
    print(f"residual_risk={residual_risk}")
    raise SystemExit(0)

if not tasks_dir.is_dir():
    emit("missing", reason=".pm/tasks directory missing")

task_uid_re = re.compile(r"^task_[0-9a-f]{32}$")
candidates: list[tuple[str, Path, str]] = []
for task_file in sorted(tasks_dir.glob("task_*.yaml")):
    text = task_file.read_text(encoding="utf-8")
    if f"worktree_hint: {source_worktree}" not in text:
        continue
    task_uid = ""
    execution_log_path = ""
    for line in text.splitlines():
        key, _, value = line.partition(":")
        value = value.strip().strip('"')
        if key == "task_uid":
            task_uid = value
        elif key == "execution_log_path":
            execution_log_path = value
    if not task_uid_re.fullmatch(task_uid):
        continue
    if not execution_log_path:
        execution_log_path = f".pm/tasks/{task_uid}.execution.md"
    candidates.append((task_uid, root / execution_log_path, execution_log_path))

if not candidates:
    emit("missing", reason=f"no .pm task has worktree_hint {source_worktree}")
if len(candidates) > 1:
    emit("missing", reason=f"multiple .pm tasks match worktree_hint {source_worktree}")

task_uid, log_path, log_path_rel = candidates[0]
if not log_path.is_file():
    emit("missing", task_uid=task_uid, log_path=log_path_rel, reason="execution log missing")

text = log_path.read_text(encoding="utf-8")
blocks = review_packet_blocks(text)
if not blocks:
    emit(
        "missing",
        task_uid=task_uid,
        log_path=log_path_rel,
        reason="no pre-PR local role review packet found",
        missing_markers=["Pre-PR Local Role Review: passed"],
    )

required = {
    "Pre-PR Local Role Review": "passed",
    "Task UID": task_uid,
    "Source Worktree": str(source_worktree),
    "Source Branch": source_branch,
    "Comparison Ref": comparison_ref,
}

missing: list[str] = []
selected_block = blocks[-1]

for key, expected in required.items():
    if parse_field(selected_block, key) != expected:
        missing.append(f"{key}: {expected}")

reviewed_source_head = parse_field(selected_block, "Source Head")
if not reviewed_source_head:
    missing.append("Source Head")
elif reviewed_source_head != source_head:
    allowed_evidence_paths = {
        log_path_rel,
        f".pm/tasks/{task_uid}.yaml",
    }
    try:
        subprocess.check_call(
            ["git", "merge-base", "--is-ancestor", reviewed_source_head, source_head],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        changed_since_review = subprocess.check_output(
            ["git", "diff", "--name-only", f"{reviewed_source_head}..{source_head}"],
            text=True,
        ).splitlines()
    except subprocess.CalledProcessError:
        missing.append(f"Source Head ancestor of {source_head}")
    else:
        disallowed = [
            path for path in changed_since_review
            if path not in allowed_evidence_paths
        ]
        if disallowed:
            missing.append("Source Head has post-review non-evidence changes: " + ",".join(disallowed))

for key in (
    "Reviewed Changed Paths",
    "Role Selection Basis",
    "Review Roles",
    "Review Evidence",
    "Finding Disposition Evidence",
    "Residual Risk",
):
    if not parse_field(selected_block, key):
        missing.append(key)

findings_disposition = parse_field(selected_block, "Review Findings Disposition")
if findings_disposition not in {"addressed", "no_findings"}:
    missing.append("Review Findings Disposition: addressed|no_findings")

review_roles = parse_field(selected_block, "Review Roles")
residual_risk = parse_field(selected_block, "Residual Risk")

if missing:
    emit(
        "missing",
        task_uid=task_uid,
        log_path=log_path_rel,
        reason="missing required pre-PR local role review markers",
        missing_markers=missing,
        review_roles=review_roles,
        findings_disposition=findings_disposition,
        residual_risk=residual_risk,
    )

emit(
    "passed",
    task_uid=task_uid,
    log_path=log_path_rel,
    reason="matched source worktree, branch, head, and comparison ref",
    review_roles=review_roles,
    findings_disposition=findings_disposition,
    residual_risk=residual_risk,
)
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
if ! WORKFLOW_LINT_OUTPUT="$(cd "$SOURCE_WORKTREE" && ./scripts/pm/workflow-lint.sh --allow-unbound 2>&1)"; then
  cat >&2 <<EOF
error: workflow-lint preflight failed.
$WORKFLOW_LINT_OUTPUT
fix: apply the suggested repair command(s) above, then rerun ./scripts/prepare-task-pr.sh.
EOF
  exit 1
fi

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

LOCAL_REQUIRED_SCOPE="unavailable"
LOCAL_REQUIRED_CHANGED_PATH_COUNT=0
LOCAL_REQUIRED_CHANGED_PATHS=""
LOCAL_REQUIRED_REASON_SUMMARY=""
LOCAL_REQUIRED_COMMAND=""
CLAIM_READY_COMMAND=""
LOCAL_REQUIRED_EXTRA_COMMANDS=()

if [[ -x "./scripts/plan-rust-required-scope.sh" ]]; then
  if RUST_SCOPE_OUTPUT="$(./scripts/plan-rust-required-scope.sh --event-name pull_request --base-ref "$COMPARISON_REF" --head-ref "$SOURCE_BRANCH" 2>/dev/null)"; then
    LOCAL_REQUIRED_SCOPE="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "scope")"
    LOCAL_REQUIRED_SCOPE="${LOCAL_REQUIRED_SCOPE:-unavailable}"
    LOCAL_REQUIRED_CHANGED_PATH_COUNT="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "changed_path_count")"
    LOCAL_REQUIRED_CHANGED_PATH_COUNT="${LOCAL_REQUIRED_CHANGED_PATH_COUNT:-0}"
    LOCAL_REQUIRED_CHANGED_PATHS="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "changed_paths")"
    LOCAL_REQUIRED_REASON_SUMMARY="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "reason_summary")"
    if [[ "$LOCAL_REQUIRED_SCOPE" != "minimal" ]]; then
      RUN_OASIS7_REQUIRED_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_required_tests" "false")"
      RUN_CONSENSUS_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_consensus_tests" "false")"
      RUN_DISTFS_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_distfs_tests" "false")"
      RUN_OASIS7_NODE_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_node_tests" "false")"
      RUN_OASIS7_NET_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_net_tests" "false")"
      RUN_OASIS7_NET_LIBP2P_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_net_libp2p_tests" "false")"
      RUN_VIEWER_CONTRACT_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_viewer_contract_tests" "false")"
      RUN_VIEWER_WASM_CHECK="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_viewer_wasm_check" "false")"
      RUN_LAUNCHER_WEB_BUILD="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_launcher_web_build" "false")"
      LOCAL_REQUIRED_COMMAND="OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=$RUN_OASIS7_REQUIRED_TESTS \
OASIS7_CI_RUN_CONSENSUS_TESTS=$RUN_CONSENSUS_TESTS \
OASIS7_CI_RUN_DISTFS_TESTS=$RUN_DISTFS_TESTS \
OASIS7_CI_RUN_OASIS7_NODE_TESTS=$RUN_OASIS7_NODE_TESTS \
OASIS7_CI_RUN_OASIS7_NET_TESTS=$RUN_OASIS7_NET_TESTS \
OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=$RUN_OASIS7_NET_LIBP2P_TESTS \
OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=$RUN_VIEWER_CONTRACT_TESTS \
OASIS7_CI_RUN_VIEWER_WASM_CHECK=$RUN_VIEWER_WASM_CHECK \
OASIS7_CI_RUN_LAUNCHER_WEB_BUILD=$RUN_LAUNCHER_WEB_BUILD \
./scripts/ci-tests.sh required"
    fi
    CLAIM_READY_COMMAND="$(render_cmd "./scripts/pm/claim-ready.sh" "--claim-type" "ready_for_pr" "--verify-command" "$LOCAL_REQUIRED_COMMAND")"
  fi
fi

REMOTE_SOURCE_REF=""
if git show-ref --verify --quiet "refs/remotes/$REMOTE_NAME/$SOURCE_BRANCH"; then
  REMOTE_SOURCE_REF="refs/remotes/$REMOTE_NAME/$SOURCE_BRANCH"
fi

LOCAL_ROLE_REVIEW_OUTPUT="$(local_role_review_status "$SOURCE_WORKTREE" "$SOURCE_BRANCH" "$SOURCE_HEAD" "$COMPARISON_REF")"
LOCAL_ROLE_REVIEW_STATUS="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "status")"
LOCAL_ROLE_REVIEW_TASK_UID="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "task_uid")"
LOCAL_ROLE_REVIEW_LOG_PATH="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "execution_log_path")"
LOCAL_ROLE_REVIEW_REASON="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "reason")"
LOCAL_ROLE_REVIEW_MISSING_MARKERS="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "missing_markers")"
LOCAL_ROLE_REVIEW_ROLES="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "review_roles")"
LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "findings_disposition")"
LOCAL_ROLE_REVIEW_RESIDUAL_RISK="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "residual_risk")"

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
  command -v gh >/dev/null 2>&1 || die '`gh` not found in PATH'
  if [[ "$REBASE_REQUIRED" == "1" ]]; then
    die "source branch is behind $COMPARISON_REF; rebase before creating the PR"
  fi
  if [[ "$LOCAL_ROLE_REVIEW_STATUS" != "passed" ]]; then
    die "missing passed pre-PR local role review evidence for $SOURCE_BRANCH at $SOURCE_HEAD ($LOCAL_ROLE_REVIEW_REASON; log: ${LOCAL_ROLE_REVIEW_LOG_PATH:-unknown}; missing: ${LOCAL_ROLE_REVIEW_MISSING_MARKERS:-unknown})"
  fi
  if [[ -z "$REMOTE_SOURCE_REF" ]]; then
    git -C "$SOURCE_WORKTREE" push -u "$REMOTE_NAME" "$SOURCE_BRANCH"
  elif [[ "$LOCAL_ONLY_COUNT" != "0" || "$REMOTE_ONLY_COUNT" != "0" ]]; then
    git -C "$SOURCE_WORKTREE" push "$REMOTE_NAME" "$SOURCE_BRANCH"
  fi
  PR_URL="$("${CREATE_CMD[@]}")"
fi

LOCAL_REQUIRED_EXTRA_COMMANDS_JOINED="$(printf '%s;' ${LOCAL_REQUIRED_EXTRA_COMMANDS[@]+"${LOCAL_REQUIRED_EXTRA_COMMANDS[@]}"})"
SUMMARY_JSON="$(
python3 - "$SOURCE_BRANCH" "$SOURCE_WORKTREE" "$SOURCE_HEAD" "$BASE_BRANCH" "$COMPARISON_REF" "$COMPARISON_HEAD" "$REMOTE_NAME" "$AHEAD_COUNT" "$BEHIND_COUNT" "$REBASE_REQUIRED" "$UPSTREAM_REF" "$LOCAL_ONLY_COUNT" "$REMOTE_ONLY_COUNT" "$CREATE_CMD_RENDERED" "$SYNC_CMD" "$CLEANUP_CMD_1" "$CLEANUP_CMD_2" "$PR_URL" "$LOCAL_REQUIRED_SCOPE" "$LOCAL_REQUIRED_CHANGED_PATH_COUNT" "$LOCAL_REQUIRED_CHANGED_PATHS" "$LOCAL_REQUIRED_REASON_SUMMARY" "$LOCAL_REQUIRED_COMMAND" "$CLAIM_READY_COMMAND" "$LOCAL_REQUIRED_EXTRA_COMMANDS_JOINED" "$LOCAL_ROLE_REVIEW_STATUS" "$LOCAL_ROLE_REVIEW_TASK_UID" "$LOCAL_ROLE_REVIEW_LOG_PATH" "$LOCAL_ROLE_REVIEW_REASON" "$LOCAL_ROLE_REVIEW_MISSING_MARKERS" "$LOCAL_ROLE_REVIEW_ROLES" "$LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION" "$LOCAL_ROLE_REVIEW_RESIDUAL_RISK" <<'PY'
from __future__ import annotations

import json
import sys

changed_paths = [path for path in sys.argv[21].split(";") if path]
reason_items = [reason for reason in sys.argv[22].split(";") if reason]
extra_commands = [cmd for cmd in sys.argv[25].split(";") if cmd]
missing_markers = [marker for marker in sys.argv[30].split(";") if marker]

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
    "review_request_command": None,
    "post_merge_commands": [cmd for cmd in sys.argv[15:18] if cmd],
    "cleanup_commands": [cmd for cmd in sys.argv[15:18] if cmd],
    "pr_url": sys.argv[18] or None,
    "local_required_validation": {
        "scope": sys.argv[19],
        "changed_path_count": int(sys.argv[20]),
        "changed_paths": changed_paths,
        "reason_summary": sys.argv[22] or None,
        "reason_items": reason_items,
        "recommended_required_command": sys.argv[23] or None,
        "recommended_claim_ready_command": sys.argv[24] or None,
        "recommended_extra_commands": extra_commands,
    },
    "pre_pr_local_role_review": {
        "status": sys.argv[26],
        "task_uid": sys.argv[27] or None,
        "execution_log_path": sys.argv[28] or None,
        "reason": sys.argv[29] or None,
        "missing_markers": missing_markers,
        "review_roles": sys.argv[31] or None,
        "findings_disposition": sys.argv[32] or None,
        "residual_risk": sys.argv[33] or None,
    },
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

echo
echo "Local Required Validation:"
echo "- scope: $LOCAL_REQUIRED_SCOPE"
echo "- changed paths: $LOCAL_REQUIRED_CHANGED_PATH_COUNT"
if [[ -n "$LOCAL_REQUIRED_REASON_SUMMARY" ]]; then
  echo "- planner reason summary: $LOCAL_REQUIRED_REASON_SUMMARY"
  while IFS= read -r reason_item; do
    [[ -n "$reason_item" ]] || continue
    echo "  - planner reason: $reason_item"
  done < <(printf '%s\n' "$LOCAL_REQUIRED_REASON_SUMMARY" | tr ';' '\n')
fi
if [[ -n "$LOCAL_REQUIRED_COMMAND" ]]; then
  echo "- recommended required command: $LOCAL_REQUIRED_COMMAND"
fi
if [[ -n "$CLAIM_READY_COMMAND" ]]; then
  echo "- recommended claim-ready command: $CLAIM_READY_COMMAND"
fi
if [[ "${#LOCAL_REQUIRED_EXTRA_COMMANDS[@]}" -gt 0 ]]; then
  for extra_cmd in "${LOCAL_REQUIRED_EXTRA_COMMANDS[@]}"; do
    echo "- recommended extra command: $extra_cmd"
  done
fi

echo
echo "Pre-PR Local Role Review:"
echo "- status: $LOCAL_ROLE_REVIEW_STATUS"
echo "- task uid: ${LOCAL_ROLE_REVIEW_TASK_UID:-"(none)"}"
echo "- execution log: ${LOCAL_ROLE_REVIEW_LOG_PATH:-"(none)"}"
echo "- reason: ${LOCAL_ROLE_REVIEW_REASON:-"(none)"}"
if [[ -n "$LOCAL_ROLE_REVIEW_MISSING_MARKERS" ]]; then
  echo "- missing markers: $LOCAL_ROLE_REVIEW_MISSING_MARKERS"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_ROLES" ]]; then
  echo "- review roles: $LOCAL_ROLE_REVIEW_ROLES"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION" ]]; then
  echo "- findings disposition: $LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_RESIDUAL_RISK" ]]; then
  echo "- residual risk: $LOCAL_ROLE_REVIEW_RESIDUAL_RISK"
fi

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
