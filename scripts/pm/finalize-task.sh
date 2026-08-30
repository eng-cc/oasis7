#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

usage() {
  cat <<'EOF'
Usage: ./scripts/pm/finalize-task.sh --task-uid <uid> --pr <number> [options]

Run the canonical merged-PR terminal lifecycle as one resumable, fail-closed
operation. Existing receipt validators and crash journals remain authoritative.

Options:
  --task-uid <uid>                  Bound task UID
  --pr <number>                     Bound merged pull request
  --repo-root <path>                Canonical default worktree (default: current repository root)
  --preflight                       Validate terminal identity without mutating state
  --patch-equivalence-receipt <p>  Reuse an existing canonical squash/rebase proof
  --resume                          Resume the same durable task/PR identity (default behavior)
  --json                            Print a machine-readable result
  -h, --help                        Show this help
EOF
}

fail() { echo "finalize-task: $*" >&2; exit 1; }
task_uid="" pr_number="" repo_root="$ROOT_DIR" supplied_patch="" resume=0 preflight=0 output_json=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) task_uid="${2:-}"; shift 2 ;;
    --pr) pr_number="${2:-}"; shift 2 ;;
    --repo-root) repo_root="${2:-}"; shift 2 ;;
    --patch-equivalence-receipt) supplied_patch="${2:-}"; shift 2 ;;
    --preflight) preflight=1; shift ;;
    --resume) resume=1; shift ;;
    --json) output_json=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done
[[ "$task_uid" =~ ^task_[0-9a-f]{32}$ ]] || fail "invalid --task-uid"
[[ "$pr_number" =~ ^[1-9][0-9]*$ ]] || fail "invalid --pr"
repo_root="$(git -C "$repo_root" rev-parse --show-toplevel)" || fail "invalid --repo-root"
SCRIPT_DIR="$repo_root/scripts/pm"
[[ -x "$SCRIPT_DIR/finalize-task.sh" ]] || fail "--repo-root does not contain the terminal orchestrator"
mapping="$repo_root/.pm/github-project-sync/tasks.json"
[[ -f "$mapping" ]] || fail "canonical task mapping is unavailable"
receipt_root="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$repo_root" --task-uid "$task_uid" --create)" \
  || fail "cannot resolve canonical receipt root"
merge_receipt="$receipt_root/merge-receipt.json"
main_sync_receipt="$receipt_root/main-sync-receipt.json"
terminal_receipt="$receipt_root/terminal-cleanup-receipt.json"
patch_equivalence="$receipt_root/patch-equivalence-receipt.json"
allow_missing_task_worktree=0
[[ "$preflight" == 0 && -f "$terminal_receipt" ]] && allow_missing_task_worktree=1
identity_json="$(python3 - "$repo_root" "$mapping" "$task_uid" "$pr_number" "$allow_missing_task_worktree" <<'PY'
import json
import pathlib
import re
import subprocess
import sys

repo_root, mapping_path, task_uid, pr_number, allow_missing_worktree = sys.argv[1:]
root = pathlib.Path(repo_root).resolve()
record = (json.loads(pathlib.Path(mapping_path).read_text(encoding="utf-8")).get("tasks") or {}).get(task_uid) or {}
bound = {
    "task_uid": str(record.get("task_uid") or task_uid),
    "pr_number": str(record.get("pr_number") or ""),
    "repository": str(record.get("repository") or ""),
    "issue_number": str(record.get("issue_number") or ""),
    "pr_url": str(record.get("pr_url") or ""),
    "canonical_worktree": str(record.get("canonical_worktree") or ""),
    "task_branch": str(record.get("task_branch") or ""),
    "default_branch": str(record.get("default_branch") or ""),
    "owner_role": str(record.get("owner_role") or ""),
}
blockers = []

def blocker(message):
    if message not in blockers:
        blockers.append(message)

if not record:
    blocker("task identity: task UID is absent from canonical mapping")
if bound["task_uid"] != task_uid:
    blocker("task identity: task UID mismatch")
if bound["pr_number"] != pr_number:
    blocker("task/PR mismatch: mapping PR does not match requested PR")
for key in ("issue_number", "pr_url", "canonical_worktree", "task_branch", "default_branch", "owner_role", "repository"):
    if not bound[key]:
        blocker(f"task identity: task truth missing {key}")
if bound["repository"] and not re.fullmatch(r"[^/\\s]+/[^/\\s]+", bound["repository"]):
    blocker("repository mismatch: task repository identity is malformed")
if bound["pr_url"]:
    pr_match = re.search(r"/pulls?/(\d+)(?:$|[?#])", bound["pr_url"])
    if not pr_match or pr_match.group(1) != pr_number:
        blocker("task/PR mismatch: task PR URL does not match requested PR")

def git(path, *args):
    try:
        return subprocess.check_output(["git", "-C", str(path), *args], text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return ""

def common_dir(path):
    raw = git(path, "rev-parse", "--git-common-dir")
    if not raw:
        return ""
    return str((path / raw if not pathlib.Path(raw).is_absolute() else pathlib.Path(raw)).resolve())

task_path = pathlib.Path(bound["canonical_worktree"]).expanduser()
if allow_missing_worktree == "1":
    pass
elif not task_path.exists():
    blocker("worktree mismatch: canonical task worktree is missing")
else:
    task_path = task_path.resolve()
    root_common = common_dir(root)
    task_common = common_dir(task_path)
    if not task_common:
        blocker("repository mismatch: canonical task worktree is not a Git worktree")
    elif root_common and task_common != root_common:
        blocker("repository mismatch: task worktree belongs to a different Git repository")

registered = {}
current_path = ""
for line in (git(root, "worktree", "list", "--porcelain") + "\n").splitlines():
    if line.startswith("worktree "):
        current_path = str(pathlib.Path(line[9:]).resolve())
    elif line.startswith("branch refs/heads/") and current_path:
        registered[current_path] = line.removeprefix("branch refs/heads/")
    elif not line:
        current_path = ""
if allow_missing_worktree != "1" and task_path.exists() and str(task_path) not in registered:
    blocker("worktree mismatch: canonical task worktree is detached or unregistered")
if allow_missing_worktree != "1" and task_path.exists():
    actual_branch = git(task_path, "symbolic-ref", "--short", "HEAD")
    if actual_branch != bound["task_branch"]:
        blocker(f"branch mismatch: task worktree is {actual_branch or 'detached'}, expected {bound['task_branch']}")
actual_default = git(root, "symbolic-ref", "--short", "HEAD")
if actual_default and actual_default != bound["default_branch"]:
    blocker(f"branch mismatch: default worktree is {actual_default}, expected {bound['default_branch']}")

payload = {
    "status": "ready" if not blockers else "blocked",
    "identity_status": "bound" if not blockers else "blocked",
    **bound,
    "pr_number": int(pr_number),
    "repo_root": str(root),
    "blockers": blockers,
    "next_command": ([] if blockers else [
        "./scripts/pm/finalize-task.sh", "--repo-root", str(root), "--task-uid", task_uid,
        "--pr", pr_number, "--resume", "--json",
    ]),
}
print(json.dumps(payload, sort_keys=True))
PY
)"
identity_status="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])' <<<"$identity_json")"
if [[ "$identity_status" != "ready" ]]; then
  if [[ "$preflight" == 1 ]]; then
    if [[ "$output_json" == 1 ]]; then
      python3 -m json.tool <<<"$identity_json"
    else
      python3 -c 'import json,sys; print("finalize-task preflight: " + "; ".join(json.load(sys.stdin)["blockers"]))' <<<"$identity_json" >&2
    fi
  else
    python3 -c 'import json,sys; print("; ".join(json.load(sys.stdin)["blockers"]))' <<<"$identity_json" >&2
  fi
  exit 1
fi
if [[ "$preflight" == 1 ]]; then
  if [[ "$output_json" == 1 ]]; then
    python3 -m json.tool <<<"$identity_json"
  else
    echo "finalize-task preflight: ready $task_uid PR #$pr_number"
  fi
  exit 0
fi
task_worktree="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["canonical_worktree"])' <<<"$identity_json")"
task_branch="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["task_branch"])' <<<"$identity_json")"
main_ref="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["default_branch"])' <<<"$identity_json")"
owner_role="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["owner_role"])' <<<"$identity_json")"
# already_finalized retries remain live-readback operations, never a second identity.
if [[ -f "$terminal_receipt" ]]; then
  ledger_existed=0
  [[ -f "$receipt_root/finalizer-ledger.json" ]] && ledger_existed=1
  # A terminal Codex task may have its exact checkout recreated after the first
  # cleanup. Re-run the receipt-bound cleanup before finalizer readback so
  # --resume reconciles that drift instead of accepting a stale receipt alone.
  cleanup_needed=0
  [[ -e "$task_worktree" ]] && cleanup_needed=1
  git -C "$repo_root" show-ref --verify --quiet "refs/heads/$task_branch" && cleanup_needed=1
  remote_branch="$(git -C "$repo_root" ls-remote --heads origin "refs/heads/$task_branch")" \
    || fail "cannot read remote task branch during terminal reconciliation"
  [[ -n "$remote_branch" ]] && cleanup_needed=1
  if [[ "$cleanup_needed" == 1 ]]; then
    cleanup_args=(--repo-root "$repo_root" --worktree "$task_worktree" --branch "$task_branch"
      --main-ref "$main_ref" --task-uid "$task_uid" --pr-receipt "$merge_receipt"
      --main-sync-receipt "$main_sync_receipt" --terminal-receipt-output "$terminal_receipt")
    if [[ -f "$patch_equivalence" ]]; then
      cleanup_args+=(--patch-equivalence-receipt "$patch_equivalence")
    fi
    "$SCRIPT_DIR/post-merge-cleanup.sh" "${cleanup_args[@]}"
  fi
  python3 "$SCRIPT_DIR/post-merge-finalize.py" --repo-root "$repo_root" --task-uid "$task_uid" --terminal-receipt "$terminal_receipt" >/dev/null
  status="$([[ "$ledger_existed" == 1 ]] && printf already_finalized || printf finalized)"
else
  [[ -d "$task_worktree" ]] || fail "canonical task worktree is missing before task_done; identity mismatch cannot be repaired here"
  (cd "$task_worktree" && python3 "$SCRIPT_DIR/pr-merge-receipt.py" "$pr_number" --json >"$merge_receipt")
  (cd "$task_worktree" && "$SCRIPT_DIR/task-closeout.sh" --role "$owner_role" --task-uid "$task_uid" \
    --to-status "done" --verification-profile repository_required --pr-receipt "$merge_receipt" >/dev/null)
  "$SCRIPT_DIR/refresh-task-cache.sh" --task-uid "$task_uid" --json >/dev/null

  patch_receipt_arg=""
  # Resolve the current remote default branch before choosing the integration
  # lane. A stale origin/<main> tracking ref must not turn an ordinary
  # fast-forward merge into an unnecessary patch-equivalence recovery path.
  git -C "$repo_root" fetch origin "$main_ref" >/dev/null \
    || fail "failed to refresh origin default branch before integration decision"
  if [[ -n "$supplied_patch" ]]; then
    [[ "$(cd "$(dirname "$supplied_patch")" && pwd -P)/$(basename "$supplied_patch")" == "$patch_equivalence" ]] \
      || fail "supplied patch-equivalence receipt path mismatch"
    patch_receipt_arg="$patch_equivalence"
  # Ordinary integration is selected by ancestry; squash/rebase requires
  # patch_equivalence against an exact first-parent integration commit.
  elif ! git -C "$repo_root" merge-base --is-ancestor "$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["head_oid"])' "$merge_receipt")" "origin/$main_ref"; then
    # Squash/rebase integration: find the exact first-parent integration commit
    # whose tree equals the repository-generated branch projection.
    branch_tip="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["head_oid"])' "$merge_receipt")"
    found=0
    while read -r integration_commit; do
      integration_parent="$(git -C "$repo_root" rev-parse "$integration_commit^")" || continue
      candidate="$receipt_root/.patch-equivalence.candidate.json"
      if "$SCRIPT_DIR/patch-equivalence-receipt.sh" --root "$repo_root" --branch-tip "$branch_tip" \
          --main-commit "$integration_commit" --main-parent "$integration_parent" >"$candidate" 2>/dev/null; then
        mv "$candidate" "$patch_equivalence"
        found=1
        break
      fi
      rm -f "$candidate"
    done < <(git -C "$repo_root" rev-list --first-parent --max-count=200 "origin/$main_ref")
    [[ "$found" == 1 ]] || fail "squash/rebase patch_equivalence proof could not be derived"
    patch_receipt_arg="$patch_equivalence"
  fi

  if [[ -n "$patch_receipt_arg" ]]; then
    "$SCRIPT_DIR/post-merge-main-sync.sh" --repo-root "$repo_root" --main-ref "$main_ref" --task-uid "$task_uid" \
      --pr-receipt "$merge_receipt" --receipt-output "$main_sync_receipt" --patch-equivalence-receipt "$patch_receipt_arg"
    "$SCRIPT_DIR/post-merge-cleanup.sh" --repo-root "$repo_root" --worktree "$task_worktree" --branch "$task_branch" \
      --main-ref "$main_ref" --task-uid "$task_uid" --pr-receipt "$merge_receipt" \
      --main-sync-receipt "$main_sync_receipt" --terminal-receipt-output "$terminal_receipt" \
      --patch-equivalence-receipt "$patch_receipt_arg"
  else
    "$SCRIPT_DIR/post-merge-main-sync.sh" --repo-root "$repo_root" --main-ref "$main_ref" --task-uid "$task_uid" \
      --pr-receipt "$merge_receipt" --receipt-output "$main_sync_receipt"
    "$SCRIPT_DIR/post-merge-cleanup.sh" --repo-root "$repo_root" --worktree "$task_worktree" --branch "$task_branch" \
      --main-ref "$main_ref" --task-uid "$task_uid" --pr-receipt "$merge_receipt" \
      --main-sync-receipt "$main_sync_receipt" --terminal-receipt-output "$terminal_receipt"
  fi
  python3 "$SCRIPT_DIR/post-merge-finalize.py" --repo-root "$repo_root" --task-uid "$task_uid" --terminal-receipt "$terminal_receipt" >/dev/null
  status="finalized"
fi

if [[ "$output_json" == 1 ]]; then
  python3 - "$status" "$task_uid" "$pr_number" "$receipt_root" "$resume" <<'PY'
import json,sys
print(json.dumps({"status":sys.argv[1],"task_uid":sys.argv[2],"pr_number":int(sys.argv[3]),"receipt_root":sys.argv[4],"resume":sys.argv[5]=="1"}))
PY
else
  echo "finalize-task: $status $task_uid PR #$pr_number"
fi
