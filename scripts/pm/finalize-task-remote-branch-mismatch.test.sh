#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMPDIR="$(mktemp -d)"
cleanup() {
  git -C "$TMPDIR/repo" worktree remove --force "$TMPDIR/task" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

REPO="$TMPDIR/repo"
TASK="$TMPDIR/task"
ORIGIN="$TMPDIR/origin.git"
UID_VALUE="task_11111111111111111111111111111111"
BRANCH="task/finalize-remote-mismatch"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync" "$TMPDIR/bin"
git init -q -b main "$REPO"
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/file"
git -C "$REPO" add file
git -C "$REPO" commit -qm base
git init -q --bare "$ORIGIN"
git -C "$REPO" remote add origin "$ORIGIN"
git -C "$REPO" push -q origin main
git -C "$REPO" worktree add -qb "$BRANCH" "$TASK"
printf 'task\n' >>"$TASK/file"
git -C "$TASK" commit -qam task
REVIEWED_HEAD="$(git -C "$TASK" rev-parse HEAD)"
git -C "$REPO" merge --ff-only "$BRANCH" >/dev/null
git -C "$REPO" push -q origin main "$BRANCH"

# Publish a different remote tip while restoring the local reviewed tip. The
# terminal cleanup must reject the reused remote name before deleting it.
printf 'remote-only\n' >>"$TASK/file"
git -C "$TASK" commit -qam remote-only
REMOTE_TIP="$(git -C "$TASK" rev-parse HEAD)"
git -C "$TASK" reset --hard "$REVIEWED_HEAD" >/dev/null
git -C "$REPO" push -q origin "$REMOTE_TIP:refs/heads/$BRANCH"

cp "$ROOT_DIR/scripts/pm/finalize-task.sh" "$REPO/scripts/pm/finalize-task.sh"
cp "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" "$REPO/scripts/pm/post-merge-cleanup.sh"
cp "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" "$REPO/scripts/pm/canonical-receipt-root.py"
cp "$ROOT_DIR/scripts/pm/validate-durable-terminal-path.py" "$REPO/scripts/pm/validate-durable-terminal-path.py"
cp "$ROOT_DIR/scripts/pm/workflow-durable-store.py" "$REPO/scripts/pm/workflow-durable-store.py"
cp "$ROOT_DIR/scripts/pm/portable_file_lock.py" "$REPO/scripts/pm/portable_file_lock.py"
cp "$ROOT_DIR/scripts/pm/pr-merge-receipt.py" "$REPO/scripts/pm/pr-merge-receipt.py"
chmod +x "$REPO/scripts/pm/"*.sh "$REPO/scripts/pm/"*.py

RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" \
  --default-worktree "$REPO" --task-uid "$UID_VALUE" --create)"
NOW="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
cat >"$RECEIPT_ROOT/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1","state":"MERGED","merged_at":"$NOW","head_oid":"$REVIEWED_HEAD","base_ref":"main","observed_at":"$NOW"}
EOF
python3 - "$RECEIPT_ROOT/merge-receipt.json" "$RECEIPT_ROOT/main-sync-receipt.json" "$UID_VALUE" "$REVIEWED_HEAD" "$NOW" <<'PY'
import hashlib
import json
import pathlib
import sys

merge = pathlib.Path(sys.argv[1])
pathlib.Path(sys.argv[2]).write_text(json.dumps({
    "receipt_type": "oasis7_main_sync",
    "issuer": "post-merge-main-sync",
    "integration_mode": "ancestry",
    "task_uid": sys.argv[3],
    "repository": "eng-cc/oasis7",
    "default_branch": "main",
    "main_commit": sys.argv[4],
    "remote_main_commit": sys.argv[4],
    "merge_receipt_sha256": hashlib.sha256(merge.read_bytes()).hexdigest(),
    "observed_at": sys.argv[5],
}) + "\n", encoding="utf-8")
PY
# The existing terminal receipt marks this as a resume lane; cleanup still
# revalidates the live PR, local tip, and remote branch before deleting.
printf '{}\n' >"$RECEIPT_ROOT/terminal-cleanup-receipt.json"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","status":"done","workflow_phase":"main_sync","repository":"eng-cc/oasis7","issue_number":1,"pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1","canonical_worktree":"$TASK","task_branch":"$BRANCH","default_branch":"main","owner_role":"repository_health_engineer"}}}
EOF
cat >"$TMPDIR/bin/gh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "pr view 1 --json number,url,state,mergedAt,headRefOid,baseRefName" ]]; then
  printf '%s\\n' '{"number":1,"url":"https://github.com/eng-cc/oasis7/pull/1","state":"MERGED","mergedAt":"$NOW","headRefOid":"$REVIEWED_HEAD","baseRefName":"main"}'
elif [[ "\$*" == "repo view --json nameWithOwner,defaultBranchRef" ]]; then
  printf '%s\\n' '{"nameWithOwner":"eng-cc/oasis7","defaultBranchRef":{"name":"main"}}'
else
  printf '%s\\n' '{}'
fi
EOF
chmod +x "$TMPDIR/bin/gh"

set +e
PATH="$TMPDIR/bin:$PATH" "$REPO/scripts/pm/finalize-task.sh" \
  --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 1 --resume --json \
  >"$TMPDIR/result.json" 2>"$TMPDIR/result.err"
status=$?
set -e
[[ "$status" != 0 ]] || { echo "finalize-task accepted a remote branch tip mismatch" >&2; exit 1; }
grep -Eiq 'remote task branch tip|disagrees with merged PR head' "$TMPDIR/result.err" || {
  echo "expected remote branch tip mismatch diagnostic, got:" >&2
  cat "$TMPDIR/result.err" >&2
  exit 1
}
[[ "$(git -C "$REPO" ls-remote --heads origin "refs/heads/$BRANCH" | awk '{print $1}')" == "$REMOTE_TIP" ]] || {
  echo "remote branch was deleted after tip mismatch" >&2
  exit 1
}

echo "finalize-task-remote-branch-mismatch.test: OK"
