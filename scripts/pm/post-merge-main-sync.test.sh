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
INITIAL_MERGE_RECEIPT_SHA256="$(shasum -a 256 "$MERGE_RECEIPT" | awk '{print $1}')"

# A later live observation of this exact merge must refresh durable authority.
# Sleep rather than inventing a timestamp so the replacement receipt has a new
# observation time and byte digest while preserving its immutable merge identity.
sleep 1
REFRESHED_OBSERVED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
python3 - "$MERGE_RECEIPT" "$REFRESHED_OBSERVED_AT" <<'PY'
import json,sys
path=sys.argv[1]
receipt=json.load(open(path,encoding='utf-8'))
receipt['observed_at']=sys.argv[2]
open(path,'w',encoding='utf-8').write(json.dumps(receipt,sort_keys=True)+'\n')
PY
"$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT" >/dev/null
python3 - "$REPO/.pm/github-project-sync/tasks.json" "$TASK_UID" "$MERGE_RECEIPT" "$REFRESHED_OBSERVED_AT" "$INITIAL_MERGE_RECEIPT_SHA256" <<'PY'
import hashlib,json,pathlib,sys
record=json.load(open(sys.argv[1],encoding='utf-8'))['tasks'][sys.argv[2]]
receipt=json.load(open(sys.argv[3],encoding='utf-8'))
digest=hashlib.sha256(pathlib.Path(sys.argv[3]).read_bytes()).hexdigest()
assert receipt['observed_at']==sys.argv[4],receipt
assert digest!=sys.argv[5],(digest,sys.argv[5])
assert record['merge_receipt']==receipt,record
assert record['merge_receipt_sha256']==digest,record
PY

assert_immutable_identity_drift_rejected() {
  local field="$1" value="$2"
  cp "$MERGE_RECEIPT" "$TMPDIR/merge-receipt-before-identity-drift.json"
  python3 - "$MERGE_RECEIPT" "$field" "$value" <<'PY'
import json,sys
path=sys.argv[1]
receipt=json.load(open(path,encoding='utf-8'))
receipt[sys.argv[2]]=sys.argv[3]
open(path,'w',encoding='utf-8').write(json.dumps(receipt,sort_keys=True)+'\n')
PY
  if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
    --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
    --receipt-output "$MAIN_SYNC_RECEIPT" >/dev/null 2>"$TMPDIR/immutable-identity-drift.err"; then
    echo "expected refreshed merge receipt immutable identity drift to fail" >&2; exit 1
  fi
  grep -Fqi 'stored merge receipt conflicts with validated receipt' "$TMPDIR/immutable-identity-drift.err"
  cp "$TMPDIR/merge-receipt-before-identity-drift.json" "$MERGE_RECEIPT"
  python3 - "$REPO/.pm/github-project-sync/tasks.json" "$TASK_UID" "$MERGE_RECEIPT" <<'PY'
import hashlib,json,pathlib,sys
record=json.load(open(sys.argv[1],encoding='utf-8'))['tasks'][sys.argv[2]]
receipt=json.load(open(sys.argv[3],encoding='utf-8'))
assert record['merge_receipt']==receipt,record
assert record['merge_receipt_sha256']==hashlib.sha256(pathlib.Path(sys.argv[3]).read_bytes()).hexdigest(),record
PY
}

assert_immutable_identity_drift_rejected head_oid "$(git -C "$REPO" rev-parse "$MERGED_HEAD^")"

SQUASH_REMOTE="$TMPDIR/squash-origin.git"
SQUASH_REPO="$TMPDIR/squash-repo"
git init -q --bare "$SQUASH_REMOTE"
git clone -q "$SQUASH_REMOTE" "$SQUASH_REPO"
git -C "$SQUASH_REPO" config user.email test@example.invalid
git -C "$SQUASH_REPO" config user.name Test
git -C "$SQUASH_REPO" switch -qc main
printf 'base\n' >"$SQUASH_REPO/file"
printf '.pm/\n' >"$SQUASH_REPO/.gitignore"
git -C "$SQUASH_REPO" add file .gitignore
git -C "$SQUASH_REPO" commit -qm base
SQUASH_BASE="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
git -C "$SQUASH_REPO" push -q -u origin main
git -C "$SQUASH_REPO" switch -qc task/change
printf 'squashed change\n' >>"$SQUASH_REPO/file"
git -C "$SQUASH_REPO" commit -qam task-change-one
SQUASH_BRANCH_FIRST="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
printf 'second rebased change\n' >"$SQUASH_REPO/second"
git -C "$SQUASH_REPO" add second
git -C "$SQUASH_REPO" commit -qm task-change-two
SQUASH_BRANCH_TIP="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
git -C "$SQUASH_REPO" switch -q main
git -C "$SQUASH_REPO" diff "$SQUASH_BASE..$SQUASH_BRANCH_FIRST" | git -C "$SQUASH_REPO" apply
git -C "$SQUASH_REPO" commit -qam rebase-merge-one
git -C "$SQUASH_REPO" diff "$SQUASH_BRANCH_FIRST..$SQUASH_BRANCH_TIP" | git -C "$SQUASH_REPO" apply
git -C "$SQUASH_REPO" add second
git -C "$SQUASH_REPO" commit -qm rebase-merge-two
SQUASH_MAIN="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
test "$(git -C "$SQUASH_REPO" rev-list --count "$SQUASH_BASE..$SQUASH_MAIN")" -eq 2
printf 'later main work\n' >"$SQUASH_REPO/later"
git -C "$SQUASH_REPO" add later
git -C "$SQUASH_REPO" commit -qm later-main-commit
SQUASH_CURRENT_MAIN="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
git -C "$SQUASH_REPO" push -q origin main
git -C "$SQUASH_REPO" reset -q --hard "$SQUASH_BASE"

mkdir -p "$SQUASH_REPO/.pm/github-project-sync"
cat >"$SQUASH_REPO/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$TASK_UID":{"status":"done","repository":"fixture/repo","default_branch":"main","canonical_worktree":"$TMPDIR/squash-task-worktree","task_branch":"task/change","pr_number":8,"pr_url":"https://example.invalid/pull/8"}}}
EOF
SQUASH_RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$SQUASH_REPO" --task-uid "$TASK_UID" --create)"
SQUASH_MERGE_RECEIPT="$SQUASH_RECEIPT_ROOT/merge-receipt.json"
SQUASH_PATCH_RECEIPT="$SQUASH_RECEIPT_ROOT/patch-equivalence-receipt.json"
SQUASH_SYNC_RECEIPT="$SQUASH_RECEIPT_ROOT/main-sync-receipt.json"
SQUASH_TERMINAL_RECEIPT="$SQUASH_RECEIPT_ROOT/terminal-cleanup-receipt.json"
SQUASH_CLEANUP_JOURNAL="$SQUASH_RECEIPT_ROOT/cleanup-intent.json"
cat >"$SQUASH_MERGE_RECEIPT" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"fixture/repo","default_branch":"main","pr_number":8,"pr_url":"https://example.invalid/pull/8","state":"MERGED","merged_at":"$OBSERVED_AT","head_oid":"$SQUASH_BRANCH_TIP","base_ref":"main","observed_at":"$OBSERVED_AT"}
EOF
bash "$ROOT_DIR/scripts/pm/patch-equivalence-receipt.sh" --root "$SQUASH_REPO" \
  --branch-tip "$SQUASH_BRANCH_TIP" --main-commit "$SQUASH_MAIN" --main-parent "$SQUASH_BASE" \
  >"$SQUASH_PATCH_RECEIPT"

"$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$SQUASH_REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$SQUASH_MERGE_RECEIPT" \
  --patch-equivalence-receipt "$SQUASH_PATCH_RECEIPT" \
  --receipt-output "$SQUASH_SYNC_RECEIPT" >/dev/null
python3 - "$SQUASH_SYNC_RECEIPT" "$SQUASH_PATCH_RECEIPT" "$SQUASH_CURRENT_MAIN" "$SQUASH_MAIN" <<'PY'
import hashlib,json,pathlib,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
p=json.load(open(sys.argv[2],encoding='utf-8'))
assert r['integration_mode']=='patch_equivalence',r
assert r['main_commit']==sys.argv[3],r
assert r['integration_commit']==sys.argv[4],r
assert r['integration_parent']==p['main_parent'],r
assert r['projected_tree_oid']==p['projected_tree_oid'],r
assert r['main_tree_oid']==p['main_tree_oid'],r
assert r['patch_equivalence_receipt_sha256']==hashlib.sha256(pathlib.Path(sys.argv[2]).read_bytes()).hexdigest(),r
PY

git -C "$SQUASH_REPO" branch side-parent "$SQUASH_BASE"
git -C "$SQUASH_REPO" switch -q side-parent
git -C "$SQUASH_REPO" commit --allow-empty -qm side-parent
SIDE_PARENT="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
git -C "$SQUASH_REPO" switch -q main
git -C "$SQUASH_REPO" merge -q --no-ff side-parent -m side-parent-merge
DAG_MAIN="$(git -C "$SQUASH_REPO" rev-parse HEAD)"
git -C "$SQUASH_REPO" push -q origin main
VALID_PATCH_RECEIPT="$TMPDIR/valid-patch-receipt.json"
cp "$SQUASH_PATCH_RECEIPT" "$VALID_PATCH_RECEIPT"
python3 - "$VALID_PATCH_RECEIPT" "$SQUASH_PATCH_RECEIPT" "$DAG_MAIN" "$SIDE_PARENT" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
p['main_commit']=sys.argv[3]; p['main_parent']=sys.argv[4]
open(sys.argv[2],'w',encoding='utf-8').write(json.dumps(p,sort_keys=True)+'\n')
PY
if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$SQUASH_REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$SQUASH_MERGE_RECEIPT" \
  --patch-equivalence-receipt "$SQUASH_PATCH_RECEIPT" \
  --receipt-output "$SQUASH_SYNC_RECEIPT" >/dev/null 2>"$TMPDIR/second-parent.err"; then
  echo "expected second-parent integration base to fail" >&2; exit 1
fi
grep -Fqi 'first-parent chain' "$TMPDIR/second-parent.err"
cp "$VALID_PATCH_RECEIPT" "$SQUASH_PATCH_RECEIPT"
git -C "$SQUASH_REPO" reset -q --hard "$SQUASH_CURRENT_MAIN"
git -C "$SQUASH_REPO" push -q --force origin main

git -C "$SQUASH_REPO" worktree add -q "$TMPDIR/squash-task-worktree" task/change
mkdir -p "$TMPDIR/squash-bin"
cat >"$TMPDIR/squash-bin/gh" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  "repo view --json nameWithOwner,defaultBranchRef")
    printf '%s\n' '{"nameWithOwner":"fixture/repo","defaultBranchRef":{"name":"main"}}'
    ;;
  *)
    printf '{"number":8,"url":"https://example.invalid/pull/8","state":"MERGED","mergedAt":"%s","headRefOid":"%s","baseRefName":"main"}\n' "${TEST_MERGED_AT:?}" "${TEST_HEAD_OID:?}"
    ;;
esac
EOF
chmod +x "$TMPDIR/squash-bin/gh"

if TEST_MERGED_AT="$OBSERVED_AT" TEST_HEAD_OID="$SQUASH_BRANCH_TIP" PATH="$TMPDIR/squash-bin:$PATH" \
  bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$SQUASH_REPO" \
  --worktree "$TMPDIR/squash-task-worktree" --branch task/change --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$SQUASH_MERGE_RECEIPT" \
  --main-sync-receipt "$SQUASH_SYNC_RECEIPT" --dry-run \
  >"$TMPDIR/squash-missing-patch.out" 2>"$TMPDIR/squash-missing-patch.err"; then
  echo "expected squash cleanup without patch receipt to fail" >&2; exit 1
fi
grep -Fqi 'no patch-equivalence receipt' "$TMPDIR/squash-missing-patch.err"

TEST_MERGED_AT="$OBSERVED_AT" TEST_HEAD_OID="$SQUASH_BRANCH_TIP" PATH="$TMPDIR/squash-bin:$PATH" \
  bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$SQUASH_REPO" \
  --worktree "$TMPDIR/squash-task-worktree" --branch task/change --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$SQUASH_MERGE_RECEIPT" \
  --main-sync-receipt "$SQUASH_SYNC_RECEIPT" --patch-equivalence-receipt "$SQUASH_PATCH_RECEIPT" \
  --dry-run >"$TMPDIR/squash-cleanup.out"
grep -Fq 'worktree remove' "$TMPDIR/squash-cleanup.out"
test -d "$TMPDIR/squash-task-worktree"
git -C "$SQUASH_REPO" show-ref --verify --quiet refs/heads/task/change

python3 - "$SQUASH_PATCH_RECEIPT" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8')); p['patch_id']='0'*40
open(sys.argv[1], 'w', encoding='utf-8').write(json.dumps(p)+'\n')
PY
if TEST_MERGED_AT="$OBSERVED_AT" TEST_HEAD_OID="$SQUASH_BRANCH_TIP" PATH="$TMPDIR/squash-bin:$PATH" \
  bash "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$SQUASH_REPO" \
  --worktree "$TMPDIR/squash-task-worktree" --branch task/change --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$SQUASH_MERGE_RECEIPT" \
  --main-sync-receipt "$SQUASH_SYNC_RECEIPT" --patch-equivalence-receipt "$SQUASH_PATCH_RECEIPT" \
  --terminal-receipt-output "$SQUASH_TERMINAL_RECEIPT" \
  >"$TMPDIR/tampered-cleanup.out" 2>"$TMPDIR/tampered-cleanup.err"; then
  echo "expected cleanup with replaced patch receipt to fail" >&2; exit 1
fi
grep -Eqi 'digest mismatch|patch.equivalence binding' "$TMPDIR/tampered-cleanup.err"
test -d "$TMPDIR/squash-task-worktree"
git -C "$SQUASH_REPO" show-ref --verify --quiet refs/heads/task/change
test ! -e "$SQUASH_CLEANUP_JOURNAL"
test ! -e "$SQUASH_TERMINAL_RECEIPT"
if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$SQUASH_REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$SQUASH_MERGE_RECEIPT" \
  --patch-equivalence-receipt "$SQUASH_PATCH_RECEIPT" \
  --receipt-output "$SQUASH_SYNC_RECEIPT" >/dev/null 2>"$TMPDIR/forged-patch.err"; then
  echo "expected forged patch-equivalence receipt to fail" >&2; exit 1
fi
grep -Eqi 'patch.equivalence|patch id|patch_id' "$TMPDIR/forged-patch.err"

if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$TMPDIR/missing.json" \
  --receipt-output "$TMPDIR/missing-out.json" >/dev/null 2>"$TMPDIR/missing.err"; then
  echo "expected missing merge receipt to fail" >&2; exit 1
fi
grep -Eqi 'merge receipt is unavailable|noncanonical' "$TMPDIR/missing.err"

printf 'dirty\n' >"$REPO/untracked"
"$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT" >/dev/null
[[ "$(git -C "$REPO" rev-parse main)" == "$MERGED_HEAD" ]]

ADVANCE="$TMPDIR/advance-main"
git clone -q "$REMOTE" "$ADVANCE"
git -C "$ADVANCE" config user.email test@example.invalid
git -C "$ADVANCE" config user.name Test
printf 'remote advance\n' >>"$ADVANCE/file"
git -C "$ADVANCE" commit -qam remote-advance
git -C "$ADVANCE" push -q origin main
if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$REPO" --main-ref main \
  --task-uid "$TASK_UID" --pr-receipt "$MERGE_RECEIPT" \
  --receipt-output "$MAIN_SYNC_RECEIPT" >/dev/null 2>"$TMPDIR/dirty-update.err"; then
  echo "expected dirty main worktree requiring an update to fail" >&2; exit 1
fi
grep -Fqi 'dirty; refusing branch update' "$TMPDIR/dirty-update.err"
[[ "$(git -C "$REPO" rev-parse main)" == "$MERGED_HEAD" ]]

echo "post-merge-main-sync.test: OK"
