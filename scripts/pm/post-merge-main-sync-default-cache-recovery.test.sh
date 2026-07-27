#!/usr/bin/env bash
# This terminal recovery fixture must remain compatible with POSIX and Git Bash.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FIXTURE_TMPDIR="$(mktemp -d)"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) TMPDIR="$(cygpath -am "$FIXTURE_TMPDIR")" ;;
  *) TMPDIR="$FIXTURE_TMPDIR" ;;
esac
trap 'rm -rf "$TMPDIR"' EXIT

REMOTE="$TMPDIR/origin.git"
REPO="$TMPDIR/default-worktree"
TASK_WORKTREE="$TMPDIR/task-worktree"
TASK_UID="task_11111111111111111111111111111111"
TASK_BRANCH="task/default-cache-recovery"

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

git -C "$REPO" worktree add -qb "$TASK_BRANCH" "$TASK_WORKTREE"
printf 'merged\n' >>"$TASK_WORKTREE/file"
git -C "$TASK_WORKTREE" commit -qam merged
MERGED_HEAD="$(git -C "$TASK_WORKTREE" rev-parse HEAD)"
git -C "$TASK_WORKTREE" push -q origin "HEAD:main"

# Terminal Project state can make the completed task absent from a refreshed
# default-worktree cache, while its canonical task worktree retains identity.
mkdir -p "$REPO/.pm/github-project-sync" "$TASK_WORKTREE/.pm/github-project-sync"
printf '%s\n' '{"version":1,"tasks":{}}' >"$REPO/.pm/github-project-sync/tasks.json"
python3 - "$TASK_WORKTREE/.pm/github-project-sync/tasks.json" "$TASK_UID" "$TASK_WORKTREE" "$TASK_BRANCH" <<'PY'
import json, sys

path, uid, worktree, branch = sys.argv[1:]
record = {
    "task_uid": uid,
    "status": "done",
    "repository": "fixture/repo",
    "default_branch": "main",
    "canonical_worktree": worktree,
    "task_branch": branch,
    "pr_number": 7,
    "pr_url": "https://example.invalid/pull/7",
}
with open(path, "w", encoding="utf-8") as stream:
    json.dump({"version": 1, "tasks": {uid: record}}, stream)
    stream.write("\n")
PY

RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" \
  --default-worktree "$REPO" --task-uid "$TASK_UID" --create)"
MERGE_RECEIPT="$RECEIPT_ROOT/merge-receipt.json"
MAIN_SYNC_RECEIPT="$RECEIPT_ROOT/main-sync-receipt.json"
OBSERVED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
STALE_OBSERVED_AT="2000-01-01T00:00:00Z"
cat >"$MERGE_RECEIPT" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"fixture/repo","default_branch":"main","pr_number":7,"pr_url":"https://example.invalid/pull/7","state":"MERGED","merged_at":"$OBSERVED_AT","head_oid":"$MERGED_HEAD","base_ref":"main","observed_at":"$STALE_OBSERVED_AT"}
EOF

cp "$REPO/.pm/github-project-sync/tasks.json" "$TMPDIR/mapping-before-stale.json"
if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT" >"$TMPDIR/stale.stdout" 2>"$TMPDIR/stale.stderr"; then
  echo "stale merge receipt unexpectedly succeeded" >&2
  exit 1
fi
grep -q 'merge receipt is stale' "$TMPDIR/stale.stderr"
cmp "$TMPDIR/mapping-before-stale.json" "$REPO/.pm/github-project-sync/tasks.json"
test ! -f "$MAIN_SYNC_RECEIPT"

cat >"$MERGE_RECEIPT" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"fixture/repo","default_branch":"main","pr_number":7,"pr_url":"https://example.invalid/pull/7","state":"MERGED","merged_at":"$OBSERVED_AT","head_oid":"$MERGED_HEAD","base_ref":"main","observed_at":"$OBSERVED_AT"}
EOF

"$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT"

test "$(git -C "$REPO" rev-parse main)" = "$MERGED_HEAD"
test -f "$MAIN_SYNC_RECEIPT"
echo "post-merge-main-sync-default-cache-recovery.test: OK"
