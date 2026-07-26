#!/usr/bin/env bash
# This cleanup regression must remain cross-platform on Windows, Linux, and macOS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REAL_GIT="$(command -v git)"
TMPDIR="$(mktemp -d)"
cleanup() {
  "$REAL_GIT" -C "$TMPDIR/repo" worktree remove --force "$TMPDIR/task-worktree" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

REPO="$TMPDIR/repo"
WORKTREE="$TMPDIR/task-worktree"
BRANCH="task/cleanup-fixture"
TASK_UID="task_11111111111111111111111111111111"
mkdir -p "$REPO"
git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'base\n' >"$REPO/file.txt"
git -C "$REPO" add file.txt
git -C "$REPO" commit -qm base
git -C "$REPO" worktree add -qb "$BRANCH" "$WORKTREE"
printf 'reviewed\n' >>"$WORKTREE/file.txt"
git -C "$WORKTREE" add file.txt
git -C "$WORKTREE" commit -qm reviewed
REVIEWED_HEAD="$(git -C "$WORKTREE" rev-parse HEAD)"
git -C "$REPO" merge --ff-only "$BRANCH" >/dev/null

mkdir -p "$REPO/.pm/github-project-sync"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$TASK_UID":{"task_uid":"$TASK_UID","status":"done","pr_number":1,"pr_url":"https://example.invalid/pull/1","repository":"eng-cc/oasis7","canonical_worktree":"$WORKTREE","task_branch":"$BRANCH","default_branch":"main"}}}
EOF
RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$TASK_UID" --create)"
MERGE_RECEIPT="$RECEIPT_ROOT/merge-receipt.json"
MAIN_SYNC_RECEIPT="$RECEIPT_ROOT/main-sync-receipt.json"
TERMINAL_RECEIPT="$RECEIPT_ROOT/terminal-cleanup-receipt.json"
OBSERVED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
cat >"$MERGE_RECEIPT" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":1,"pr_url":"https://example.invalid/pull/1","state":"MERGED","merged_at":"2026-07-11T00:00:00Z","head_oid":"$REVIEWED_HEAD","base_ref":"main","observed_at":"$OBSERVED_AT"}
EOF
MAIN_COMMIT="$(git -C "$REPO" rev-parse main)"
python3 - "$MERGE_RECEIPT" "$MAIN_SYNC_RECEIPT" "$TASK_UID" "$MAIN_COMMIT" "$OBSERVED_AT" <<'PY'
import hashlib,json,pathlib,sys
merge=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
out.write_text(json.dumps({
  "receipt_type":"oasis7_main_sync", "issuer":"post-merge-main-sync", "integration_mode":"ancestry",
  "task_uid":sys.argv[3], "repository":"eng-cc/oasis7", "default_branch":"main",
  "main_commit":sys.argv[4], "remote_main_commit":sys.argv[4],
  "merge_receipt_sha256":hashlib.sha256(merge.read_bytes()).hexdigest(),
  "observed_at":sys.argv[5],
})+"\n", encoding="utf-8")
PY
mkdir -p "$TMPDIR/bin"
cat >"$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  "repo view --json nameWithOwner,defaultBranchRef")
    printf '%s\n' '{"nameWithOwner":"eng-cc/oasis7","defaultBranchRef":{"name":"main"}}'
    ;;
  *)
    printf '{"number":1,"url":"https://example.invalid/pull/1","state":"%s","mergedAt":"2026-07-11T00:00:00Z","headRefOid":"%s","baseRefName":"main"}\n' "${TEST_PR_STATE:-MERGED}" "${TEST_HEAD_OID:?}"
    ;;
esac
EOF
chmod +x "$TMPDIR/bin/gh"
# Native Python on Windows resolves gh.exe instead of the extensionless Bash
# fixture. TEST_NATIVE_GH_SHIM optionally supplies that equivalent executable.
if [[ -n "${TEST_NATIVE_GH_SHIM:-}" ]]; then
  [[ -x "$TEST_NATIVE_GH_SHIM" ]] || { echo "TEST_NATIVE_GH_SHIM must name an executable" >&2; exit 1; }
  cp "$TEST_NATIVE_GH_SHIM" "$TMPDIR/bin/gh.exe"
fi

# Main-sync authority is mandatory and cryptographically bound to this merge receipt.
if TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" --dry-run \
  >"$TMPDIR/no-main-sync.out" 2>"$TMPDIR/no-main-sync.err"; then
  echo "expected cleanup without main-sync receipt to fail closed" >&2
  exit 1
fi
grep -Fqi 'main-sync-receipt' "$TMPDIR/no-main-sync.err"
python3 - "$MAIN_SYNC_RECEIPT" "$TMPDIR/forged-main-sync.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1],encoding='utf-8')); r['merge_receipt_sha256']='0'*64
json.dump(r,open(sys.argv[2],'w',encoding='utf-8'))
PY
if TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --main-sync-receipt "$TMPDIR/forged-main-sync.json" --dry-run \
  >"$TMPDIR/forged-main-sync.out" 2>"$TMPDIR/forged-main-sync.err"; then
  echo "expected cleanup with forged main-sync receipt to fail closed" >&2
  exit 1
fi
grep -Eqi 'not bound to merge receipt|noncanonical' "$TMPDIR/forged-main-sync.err"

# Terminal cleanup authority is the complete authoritative mapping identity.
# Omitting any component must fail before live-query or filesystem effects.
MAPPING="$REPO/.pm/github-project-sync/tasks.json"
mapping_contract_failures=0
for missing in repository canonical_worktree task_branch default_branch; do
  python3 - "$MAPPING" "$TASK_UID" "$WORKTREE" "$BRANCH" "$missing" <<'PY'
import json, pathlib, sys
path, uid, worktree, branch, missing = pathlib.Path(sys.argv[1]), *sys.argv[2:]
record = {
    "task_uid": uid, "status": "done", "pr_number": 1,
    "pr_url": "https://example.invalid/pull/1", "repository": "eng-cc/oasis7",
    "canonical_worktree": worktree, "task_branch": branch, "default_branch": "main",
}
record.pop(missing)
path.write_text(json.dumps({"version": 1, "tasks": {uid: record}}) + "\n", encoding="utf-8")
PY
  if TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
    --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
    --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
    --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
    --dry-run >"$TMPDIR/missing-$missing.out" 2>"$TMPDIR/missing-$missing.err"; then
    echo "expected cleanup to reject mapping missing $missing" >&2
    mapping_contract_failures=$((mapping_contract_failures + 1))
  elif ! grep -Fqi "$missing" "$TMPDIR/missing-$missing.err"; then
    echo "expected explicit missing-$missing diagnostic, got:" >&2
    cat "$TMPDIR/missing-$missing.err" >&2
    mapping_contract_failures=$((mapping_contract_failures + 1))
  fi
done
(( mapping_contract_failures == 0 )) || exit 1
cat >"$MAPPING" <<EOF
{"version":1,"tasks":{"$TASK_UID":{"task_uid":"$TASK_UID","status":"done","pr_number":1,"pr_url":"https://example.invalid/pull/1","repository":"eng-cc/oasis7","canonical_worktree":"$WORKTREE","task_branch":"$BRANCH","default_branch":"main"}}}
EOF

# Git for Windows emits C:/... worktree porcelain while Python path resolution
# preserves a caller/task-truth C:\\... spelling. Both spellings identify the
# same worktree and must be accepted before terminal cleanup proceeds.
WINDOWS_WORKTREE="$(cygpath -w "$WORKTREE")"
python3 - "$MAPPING" "$TASK_UID" "$WINDOWS_WORKTREE" "$BRANCH" <<'PY'
import json,sys
path, uid, worktree, branch = sys.argv[1:]
json.dump({"version": 1, "tasks": {uid: {
    "task_uid": uid, "status": "done", "pr_number": 1,
    "pr_url": "https://example.invalid/pull/1", "repository": "eng-cc/oasis7",
    "canonical_worktree": worktree, "task_branch": branch, "default_branch": "main",
}}}, open(path, "w", encoding="utf-8"))
PY
if ! TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WINDOWS_WORKTREE" --branch "$BRANCH" \
  --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
  --dry-run >"$TMPDIR/windows-path.out" 2>"$TMPDIR/windows-path.err"; then
  cat "$TMPDIR/windows-path.err" >&2
  exit 1
fi
grep -F "worktree remove" "$TMPDIR/windows-path.out" >/dev/null

printf 'dirty\n' >"$WORKTREE/untracked.txt"
if TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
  --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
  --dry-run >"$TMPDIR/dirty.out" 2>"$TMPDIR/dirty.err"; then
  echo "expected dirty worktree cleanup to fail closed" >&2
  exit 1
fi
[[ -d "$WORKTREE" ]]
git -C "$REPO" show-ref --verify --quiet "refs/heads/$BRANCH"
grep -Eiq 'dirty|untracked|not clean' "$TMPDIR/dirty.err"

rm "$WORKTREE/untracked.txt"
if TEST_PR_STATE=OPEN TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
  --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
  --dry-run >"$TMPDIR/open.out" 2>"$TMPDIR/open.err"; then
  echo "expected non-merged PR cleanup to fail closed" >&2
  exit 1
fi
[[ -d "$WORKTREE" ]]
grep -Eiq 'PR.*MERGED|not merged' "$TMPDIR/open.err"

TEST_HEAD_OID="$REVIEWED_HEAD" PATH="$TMPDIR/bin:$PATH" bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$REPO" --worktree "$WORKTREE" --branch "$BRANCH" \
  --main-ref main --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --main-sync-receipt "$MAIN_SYNC_RECEIPT" \
  --dry-run >"$TMPDIR/safe.out"
grep -F "worktree remove" "$TMPDIR/safe.out" >/dev/null
if grep -Eq 'worktree remove (--force|-f)|branch -D' "$TMPDIR/safe.out"; then
  echo "safe cleanup must never emit force-removal or force branch deletion" >&2
  exit 1
fi
[[ -d "$WORKTREE" ]]

echo "post-merge-cleanup.test: OK"
