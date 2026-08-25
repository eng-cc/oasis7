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
  --patch-equivalence-receipt <p>  Reuse an existing canonical squash/rebase proof
  --resume                          Resume the same durable task/PR identity (default behavior)
  --json                            Print a machine-readable result
  -h, --help                        Show this help
EOF
}

fail() { echo "finalize-task: $*" >&2; exit 1; }
task_uid="" pr_number="" repo_root="$ROOT_DIR" supplied_patch="" resume=0 output_json=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) task_uid="${2:-}"; shift 2 ;;
    --pr) pr_number="${2:-}"; shift 2 ;;
    --repo-root) repo_root="${2:-}"; shift 2 ;;
    --patch-equivalence-receipt) supplied_patch="${2:-}"; shift 2 ;;
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
fields="$(python3 - "$mapping" "$task_uid" "$pr_number" <<'PY'
import json,sys
r=(json.load(open(sys.argv[1])).get("tasks") or {}).get(sys.argv[2]) or {}
if str(r.get("pr_number") or "") != sys.argv[3]: raise SystemExit("finalize-task: task/PR mismatch")
if str(r.get("task_uid") or "") != sys.argv[2]: raise SystemExit("finalize-task: task UID mismatch")
for key in ("issue_number","pr_url","canonical_worktree","task_branch","default_branch","owner_role","repository"):
 if not r.get(key): raise SystemExit(f"finalize-task: task truth missing {key}")
print(r["canonical_worktree"]); print(r["task_branch"]); print(r["default_branch"]); print(r["owner_role"]); print(r["repository"])
PY
)"
task_worktree="$(printf '%s\n' "$fields" | sed -n '1p')"
task_branch="$(printf '%s\n' "$fields" | sed -n '2p')"
main_ref="$(printf '%s\n' "$fields" | sed -n '3p')"
owner_role="$(printf '%s\n' "$fields" | sed -n '4p')"
receipt_root="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$repo_root" --task-uid "$task_uid" --create)" \
  || fail "cannot resolve canonical receipt root"
merge_receipt="$receipt_root/merge-receipt.json"
main_sync_receipt="$receipt_root/main-sync-receipt.json"
terminal_receipt="$receipt_root/terminal-cleanup-receipt.json"
patch_equivalence="$receipt_root/patch-equivalence-receipt.json"

# already_finalized retries remain live-readback operations, never a second identity.
if [[ -f "$terminal_receipt" && -f "$receipt_root/finalizer-ledger.json" ]]; then
  python3 "$SCRIPT_DIR/post-merge-finalize.py" --repo-root "$repo_root" --task-uid "$task_uid" --terminal-receipt "$terminal_receipt" >/dev/null
  status="already_finalized"
else
  [[ -d "$task_worktree" ]] || fail "canonical task worktree is missing before task_done; identity mismatch cannot be repaired here"
  (cd "$task_worktree" && python3 "$SCRIPT_DIR/pr-merge-receipt.py" "$pr_number" --json >"$merge_receipt")
  (cd "$task_worktree" && "$SCRIPT_DIR/task-closeout.sh" --role "$owner_role" --task-uid "$task_uid" \
    --to-status "done" --verification-profile repository_required --pr-receipt "$merge_receipt" >/dev/null)
  "$SCRIPT_DIR/refresh-task-cache.sh" --task-uid "$task_uid" --json >/dev/null

  patch_receipt_arg=""
  if [[ -n "$supplied_patch" ]]; then
    [[ "$(cd "$(dirname "$supplied_patch")" && pwd -P)/$(basename "$supplied_patch")" == "$patch_equivalence" ]] \
      || fail "supplied patch-equivalence receipt path mismatch"
    patch_receipt_arg="$patch_equivalence"
  # Ordinary integration is selected by ancestry; squash/rebase requires
  # patch_equivalence against an exact first-parent integration commit.
  elif ! git -C "$repo_root" merge-base --is-ancestor "$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["head_oid"])' "$merge_receipt")" "origin/$main_ref"; then
    # Squash/rebase integration: find the exact first-parent integration commit
    # whose tree equals the repository-generated branch projection.
    git -C "$repo_root" fetch origin "$main_ref" >/dev/null
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
