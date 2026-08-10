#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REAL_GIT="$(command -v git)"
TMPDIR="$(mktemp -d)"
cleanup(){ "$REAL_GIT" -C "$TMPDIR/repo" worktree remove --force "$TMPDIR/worktree" >/dev/null 2>&1 || true; rm -rf "$TMPDIR"; }
trap cleanup EXIT

REPO="$TMPDIR/repo"; WORKTREE="$TMPDIR/worktree"; BRANCH="task/cleanup-trust"; TASK_UID="task_11111111111111111111111111111111"
mkdir -p "$REPO"; git -C "$REPO" init -q -b main; git -C "$REPO" config user.email test@example.invalid; git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/file"; git -C "$REPO" add file; git -C "$REPO" commit -qm base
git -C "$REPO" worktree add -qb "$BRANCH" "$WORKTREE"
printf 'reviewed\n' >>"$WORKTREE/file"; git -C "$WORKTREE" commit -qam reviewed
REVIEWED_HEAD="$(git -C "$WORKTREE" rev-parse HEAD)"; git -C "$REPO" merge --ff-only "$BRANCH" >/dev/null
mkdir -p "$REPO/.pm/github-project-sync"; printf '{"version":1,"tasks":{"%s":{"status":"done","pr_number":2198,"pr_url":"https://example.invalid/pull/2198","repository":"eng-cc/oasis7","canonical_worktree":"%s","task_branch":"%s","default_branch":"main"}}}\n' "$TASK_UID" "$WORKTREE" "$BRANCH" >"$REPO/.pm/github-project-sync/tasks.json"
RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$TASK_UID" --create)"
MERGE_RECEIPT="$RECEIPT_ROOT/merge-receipt.json"; MAIN_SYNC_RECEIPT="$RECEIPT_ROOT/main-sync-receipt.json"

if bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
  --main-ref main --reviewed-head "$REVIEWED_HEAD" --task-uid "$TASK_UID" --pr-state MERGED --dry-run \
  >"$TMPDIR/literal.out" 2>"$TMPDIR/literal.err"; then
  echo "expected literal MERGED plus mutable cache truth to be rejected without a fresh PR receipt" >&2
  exit 1
fi

OBSERVED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
MERGED_AT="2026-07-11T00:00:00Z"
cat >"$MERGE_RECEIPT" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":2198,"pr_url":"https://example.invalid/pull/2198","state":"MERGED","merged_at":"$MERGED_AT","head_oid":"$REVIEWED_HEAD","base_ref":"main","observed_at":"$OBSERVED_AT"}
EOF
MAIN_COMMIT="$(git -C "$REPO" rev-parse main)"
python3 - "$MERGE_RECEIPT" "$MAIN_SYNC_RECEIPT" "$TASK_UID" "$MAIN_COMMIT" "$OBSERVED_AT" <<'PY'
import hashlib,json,pathlib,sys
merge=pathlib.Path(sys.argv[1])
pathlib.Path(sys.argv[2]).write_text(json.dumps({
 "receipt_type":"oasis7_main_sync","issuer":"post-merge-main-sync","integration_mode":"ancestry","task_uid":sys.argv[3],
 "repository":"eng-cc/oasis7","default_branch":"main","main_commit":sys.argv[4],
 "remote_main_commit":sys.argv[4],"merge_receipt_sha256":hashlib.sha256(merge.read_bytes()).hexdigest(),
 "observed_at":sys.argv[5]})+"\n",encoding="utf-8")
PY
printf 'post-review\n' >>"$WORKTREE/file"; git -C "$WORKTREE" commit -qam post-review
mkdir -p "$TMPDIR/bin"
cat >"$TMPDIR/bin/gh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "repo view --json nameWithOwner,defaultBranchRef" ]]; then
  printf '%s\n' '{"nameWithOwner":"eng-cc/oasis7","defaultBranchRef":{"name":"main"}}'
else
  printf '%s\n' '{"number":2198,"url":"https://example.invalid/pull/2198","state":"MERGED","mergedAt":"$MERGED_AT","headRefOid":"$REVIEWED_HEAD","baseRefName":"main"}'
fi
EOF
chmod +x "$TMPDIR/bin/gh"
if PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
  --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" --dry-run \
  --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
  >"$TMPDIR/tip.out" 2>"$TMPDIR/tip.err"; then
  echo "expected cleanup to reject an unreviewed branch tip beyond receipt head" >&2
  exit 1
fi
if ! grep -Eiq 'branch tip|head_oid|reviewed head|patch equivalence' "$TMPDIR/tip.err"; then
  echo "expected branch-tip or patch-equivalence rejection, got:" >&2
  cat "$TMPDIR/tip.err" >&2
  exit 1
fi

# A crash after worktree removal leaves the branch ref alive for retry.  Move
# that ref to a different main-contained commit before resuming: cleanup must
# validate the real ref, not compare the receipt head with itself.
BASE_HEAD="$(git -C "$REPO" rev-parse "$REVIEWED_HEAD^")"
git -C "$REPO" worktree remove "$WORKTREE"
git -C "$REPO" branch -f "$BRANCH" "$BASE_HEAD"
NORMALIZED_WORKTREE="$(python3 - "$WORKTREE" <<'PY'
import os,sys
path=os.path.normcase(os.path.realpath(sys.argv[1]))
print(path.replace("\\", "/") if os.name == "nt" else path)
PY
)"
JOURNAL_JSON="$(python3 - "$TASK_UID" "$NORMALIZED_WORKTREE" "$BRANCH" <<'PY'
import json,sys
print(json.dumps({
    "receipt_type":"oasis7_cleanup_intent",
    "task_uid":sys.argv[1],
    "repository":"eng-cc/oasis7",
    "worktree":sys.argv[2],
    "branch":sys.argv[3],
    "worktree_removed":True,
    "branch_deleted":False,
    "terminal_receipt_committed":False,
    "revision":1,
}))
PY
)"
python3 "$ROOT_DIR/scripts/pm/workflow-durable-store.py" write-journal \
  --path "$RECEIPT_ROOT/cleanup-intent.json" --json "$JOURNAL_JSON"
if PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
  --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
  --terminal-receipt-output "$RECEIPT_ROOT/terminal-cleanup-receipt.json" \
  >"$TMPDIR/resume.out" 2>"$TMPDIR/resume.err"; then
  echo "expected journal-resume cleanup to reject drifted live branch ref" >&2
  exit 1
fi
if ! grep -Eiq 'branch tip|head_oid|reviewed head|patch equivalence' "$TMPDIR/resume.err"; then
  echo "expected journal-resume branch-ref rejection, got:" >&2
  cat "$TMPDIR/resume.err" >&2
  exit 1
fi

echo "post-merge-cleanup-trust.test: OK"
