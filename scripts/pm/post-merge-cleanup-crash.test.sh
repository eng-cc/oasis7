#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT
FAULT_FIXTURE="$ROOT_DIR/scripts/pm/fixtures/post-merge-cleanup-fault.sh"
[[ -x "$FAULT_FIXTURE" ]] || { echo "missing isolated cleanup fault fixture: $FAULT_FIXTURE" >&2; exit 1; }

run_case() {
  local name="$1" fault="$2" expected_exit="$3"
  local repo="$TMPDIR/$name/repo" worktree="$TMPDIR/$name/task" branch="task/$name"
  local uid="task_11111111111111111111111111111111" receipts
  mkdir -p "$repo" "$TMPDIR/$name/bin"
  git -C "$repo" init -q -b main; git -C "$repo" config user.email test@example.invalid; git -C "$repo" config user.name Test
  receipts="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$repo" --task-uid "$uid" --create)"
  printf 'base\n' >"$repo/file"; git -C "$repo" add file; git -C "$repo" commit -qm base
  git -C "$repo" worktree add -qb "$branch" "$worktree"; printf 'merged\n' >>"$worktree/file"
  git -C "$worktree" commit -qam merged; local head; head="$(git -C "$worktree" rev-parse HEAD)"
  git -C "$repo" merge --ff-only "$branch" >/dev/null
  mkdir -p "$repo/.pm/github-project-sync"
  cat >"$repo/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$uid":{"task_uid":"$uid","status":"done","workflow_phase":"main_sync","repository":"eng-cc/oasis7","issue_number":11,"pr_number":1,"pr_url":"https://example.invalid/pull/1","canonical_worktree":"$worktree","task_branch":"$branch","default_branch":"main"}}}
EOF
  local now main; now="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"; main="$(git -C "$repo" rev-parse main)"
  cat >"$receipts/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":1,"pr_url":"https://example.invalid/pull/1","state":"MERGED","merged_at":"$now","head_oid":"$head","base_ref":"main","observed_at":"$now"}
EOF
  python3 - "$receipts/merge-receipt.json" "$receipts/main-sync-receipt.json" "$uid" "$main" "$now" <<'PY'
import hashlib,json,pathlib,sys
m=pathlib.Path(sys.argv[1]); pathlib.Path(sys.argv[2]).write_text(json.dumps({
"receipt_type":"oasis7_main_sync","issuer":"post-merge-main-sync","integration_mode":"ancestry","task_uid":sys.argv[3],"repository":"eng-cc/oasis7",
"default_branch":"main","main_commit":sys.argv[4],"remote_main_commit":sys.argv[4],
"merge_receipt_sha256":hashlib.sha256(m.read_bytes()).hexdigest(),"observed_at":sys.argv[5]})+'\n')
PY
  cat >"$TMPDIR/$name/bin/gh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "repo view --json nameWithOwner,defaultBranchRef" ]]; then
 printf '%s\n' '{"nameWithOwner":"eng-cc/oasis7","defaultBranchRef":{"name":"main"}}'
else
 printf '%s\n' '{"number":1,"url":"https://example.invalid/pull/1","state":"MERGED","mergedAt":"$now","headRefOid":"$head","baseRefName":"main"}'
fi
EOF
  chmod +x "$TMPDIR/$name/bin/gh"
  set +e
  local isolation_root; isolation_root="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$TMPDIR/$name")"
  env PATH="$TMPDIR/$name/bin:$PATH" "$FAULT_FIXTURE" --isolation-root "$isolation_root" --fault "$fault" -- \
    "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree" --branch "$branch" \
    --main-ref main --task-uid "$uid" --pr-receipt "$receipts/merge-receipt.json" \
    --main-sync-receipt "$receipts/main-sync-receipt.json" --terminal-receipt-output "$receipts/terminal-cleanup-receipt.json" \
    >"$receipts/first.out" 2>"$receipts/first.err"
  local status=$?; set -e
  if [[ "$status" != "$expected_exit" ]]; then cat "$receipts/first.err" >&2; return 1; fi
  [[ -f "$receipts/cleanup-intent.json" ]]
  python3 - "$receipts/cleanup-intent.json" "$fault" <<'PY'
import json,sys
j=json.load(open(sys.argv[1])); assert j['receipt_type']=='oasis7_cleanup_intent',j
assert j['worktree_removed'] is True,j
if sys.argv[2].endswith('BRANCH_DELETE'): assert j['branch_deleted'] is True,j
assert j['terminal_receipt_committed'] is False,j
PY
  env PATH="$TMPDIR/$name/bin:$PATH" \
    "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree" --branch "$branch" \
    --main-ref main --task-uid "$uid" --pr-receipt "$receipts/merge-receipt.json" \
    --main-sync-receipt "$receipts/main-sync-receipt.json" --terminal-receipt-output "$receipts/terminal-cleanup-receipt.json" \
    >"$receipts/retry.out" 2>"$receipts/retry.err"
  [[ -f "$receipts/terminal-cleanup-receipt.json" ]]
  python3 - "$receipts/cleanup-intent.json" <<'PY'
import json,sys
j=json.load(open(sys.argv[1])); assert all(j[k] for k in ('worktree_removed','branch_deleted','terminal_receipt_committed')),j
PY
  [[ ! -e "$worktree" ]]; ! git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
}

run_case after_worktree TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE 86
run_case after_branch TPM_CLEANUP_FAULT_AFTER_BRANCH_DELETE 87
echo "post-merge-cleanup-crash.test: OK"
