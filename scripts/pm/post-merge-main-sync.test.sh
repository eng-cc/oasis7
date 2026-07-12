#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REMOTE="$TMPDIR/origin.git"
REPO="$TMPDIR/repo"
TASK_UID="task_11111111111111111111111111111111"
git init -q --bare "$REMOTE"
git clone -q "$REMOTE" "$REPO"
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" switch -qc main
printf 'base\n' >"$REPO/file"
printf '.pm/\n' >"$REPO/.gitignore"
git -C "$REPO" add file .gitignore
git -C "$REPO" commit -qm base
git -C "$REPO" push -q -u origin main
printf 'merged\n' >>"$REPO/file"
git -C "$REPO" commit -qam merged
MERGED_HEAD="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" push -q origin main
git -C "$REPO" reset -q --hard HEAD^

mkdir -p "$REPO/.pm/github-project-sync" "$TMPDIR/canonical-task-worktree"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$TASK_UID":{"status":"done","repository":"fixture/repo","default_branch":"main","canonical_worktree":"$TMPDIR/canonical-task-worktree","pr_number":7,"pr_url":"https://example.invalid/pull/7"}}}
EOF
RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$TASK_UID" --create)"
MERGE_RECEIPT="$RECEIPT_ROOT/merge-receipt.json"
MAIN_SYNC_RECEIPT="$RECEIPT_ROOT/main-sync-receipt.json"
OBSERVED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
cat >"$MERGE_RECEIPT" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"fixture/repo","default_branch":"main","pr_number":7,"pr_url":"https://example.invalid/pull/7","state":"MERGED","merged_at":"$OBSERVED_AT","head_oid":"$MERGED_HEAD","base_ref":"main","observed_at":"$OBSERVED_AT"}
EOF

"$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT" >/dev/null
[[ "$(git -C "$REPO" rev-parse main)" == "$MERGED_HEAD" ]]
python3 - "$MAIN_SYNC_RECEIPT" "$MERGE_RECEIPT" "$TASK_UID" "$MERGED_HEAD" <<'PY'
import hashlib,json,pathlib,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
assert r['receipt_type']=='oasis7_main_sync' and r['issuer']=='post-merge-main-sync',r
assert r['task_uid']==sys.argv[3] and r['main_commit']==sys.argv[4],r
assert r['main_commit']==r['remote_main_commit'],r
assert r['merge_receipt_sha256']==hashlib.sha256(pathlib.Path(sys.argv[2]).read_bytes()).hexdigest(),r
PY

if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$TMPDIR/missing.json" \
  --receipt-output "$TMPDIR/missing-out.json" >/dev/null 2>"$TMPDIR/missing.err"; then
  echo "expected missing merge receipt to fail" >&2; exit 1
fi
grep -Eqi 'merge receipt is unavailable|noncanonical' "$TMPDIR/missing.err"

printf 'dirty\n' >"$REPO/untracked"
if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT" >/dev/null 2>"$TMPDIR/dirty.err"; then
  echo "expected dirty main worktree to fail" >&2; exit 1
fi
grep -Fqi 'dirty' "$TMPDIR/dirty.err"

echo "post-merge-main-sync.test: OK"
