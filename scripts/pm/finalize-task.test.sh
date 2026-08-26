#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"
TASK="$TMP/task"
UID_VALUE="task_11111111111111111111111111111111"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync" "$TASK" "$REPO/.git/receipts"
cp "$SOURCE_ROOT/scripts/pm/finalize-task.sh" "$REPO/scripts/pm/finalize-task.sh"
chmod +x "$REPO/scripts/pm/finalize-task.sh"

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'fixture\n' >"$REPO/README"
git -C "$REPO" add README
git -C "$REPO" commit -qm fixture
INITIAL_MAIN_OID="$(git -C "$REPO" rev-parse HEAD)"
ORIGIN="$TMP/origin.git"
git init --bare -q "$ORIGIN"
git -C "$REPO" remote add origin "$ORIGIN"
git -C "$REPO" push -q origin main

# The remote-tracking ref is deliberately stale while the real origin/main
# already contains the ordinary fast-forward integration.  The orchestrator
# must refresh before selecting the ordinary lane; otherwise it misclassifies
# this as squash/rebase and invokes the patch-equivalence helper.
printf 'ordinary change\n' >>"$REPO/README"
git -C "$REPO" add README
git -C "$REPO" commit -qm ordinary-integration
HEAD_OID="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" push -q origin main
git -C "$REPO" update-ref refs/remotes/origin/main "$INITIAL_MAIN_OID"

cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":3379,"pr_url":"https://example.invalid/pull/7","canonical_worktree":"$TASK","task_branch":"task/finalize","default_branch":"main","owner_role":"repository_health_engineer","pr_number":7}}}
EOF

make_mock() {
  local name=$1 body=$2
  cat >"$REPO/scripts/pm/$name" <<EOF
#!/usr/bin/env bash
set -euo pipefail
$body
EOF
  chmod +x "$REPO/scripts/pm/$name"
}
make_mock refresh-task-cache.sh "echo refresh >>\"\$TEST_SEQUENCE\"; printf \"{}\\n\""
cat >"$REPO/scripts/pm/canonical-receipt-root.py" <<'PY'
#!/usr/bin/env python3
import os
print(os.environ["TEST_REPO"] + "/.git/receipts")
PY
chmod +x "$REPO/scripts/pm/canonical-receipt-root.py"
cat >"$REPO/scripts/pm/pr-merge-receipt.py" <<'PY'
#!/usr/bin/env python3
import json, os
with open(os.environ["TEST_SEQUENCE"], "a") as handle: handle.write("merge-receipt\n")
print(json.dumps({"head_oid": os.environ["TEST_HEAD"]}))
PY
chmod +x "$REPO/scripts/pm/pr-merge-receipt.py"
make_mock task-closeout.sh "echo task-closeout >>\"\$TEST_SEQUENCE\""
make_mock post-merge-main-sync.sh "echo main-sync >>\"\$TEST_SEQUENCE\""
make_mock post-merge-cleanup.sh "echo cleanup >>\"\$TEST_SEQUENCE\"; : >\"\$TEST_REPO/.git/receipts/terminal-cleanup-receipt.json\""
cat >"$REPO/scripts/pm/post-merge-finalize.py" <<'PY'
#!/usr/bin/env python3
import os, pathlib
with open(os.environ["TEST_SEQUENCE"], "a") as handle: handle.write("finalize\n")
pathlib.Path(os.environ["TEST_REPO"] + "/.git/receipts/finalizer-ledger.json").touch()
if os.environ.get("FINALIZE_FAIL_ONCE") and not pathlib.Path(os.environ["TEST_REPO"] + "/.git/receipts/finalizer-crashed").exists():
    pathlib.Path(os.environ["TEST_REPO"] + "/.git/receipts/finalizer-crashed").touch()
    raise SystemExit(97)
PY
chmod +x "$REPO/scripts/pm/post-merge-finalize.py"
make_mock patch-equivalence-receipt.sh 'echo unexpected-patch >&2; exit 91'

SEQUENCE="$TMP/sequence"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/result.json"
printf '%s\n' merge-receipt task-closeout refresh main-sync cleanup finalize >"$TMP/expected"
cmp "$TMP/expected" "$SEQUENCE"
python3 - "$TMP/result.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); assert r["status"]=="finalized" and r["pr_number"]==7 and r["resume"] is True,r
PY

: >"$SEQUENCE"
if TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 8 --json >"$TMP/bad.out" 2>"$TMP/bad.err"; then
  echo "finalize-task accepted task/PR identity mismatch" >&2
  exit 1
fi
grep -F 'task/PR mismatch' "$TMP/bad.err" >/dev/null
test ! -s "$SEQUENCE"

# A crash after the terminal receipt and finalizer ledger are durable must
# resume through the finalizer readback only, without replaying earlier steps.
rm -f "$REPO/.git/receipts/terminal-cleanup-receipt.json" \
  "$REPO/.git/receipts/finalizer-ledger.json" "$REPO/.git/receipts/finalizer-crashed" \
  "$REPO/.git/receipts/merge-receipt.json" "$REPO/.git/receipts/main-sync-receipt.json"
: >"$SEQUENCE"
if FINALIZE_FAIL_ONCE=1 TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/crash.out" 2>"$TMP/crash.err"; then
  echo "finalize-task unexpectedly hid the injected finalizer crash" >&2
  exit 1
fi
printf '%s\n' merge-receipt task-closeout refresh main-sync cleanup finalize >"$TMP/crash-expected"
cmp "$TMP/crash-expected" "$SEQUENCE"
test -f "$REPO/.git/receipts/terminal-cleanup-receipt.json"
test -f "$REPO/.git/receipts/finalizer-ledger.json"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/retry.json"
python3 - "$TMP/retry.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); assert r["status"]=="already_finalized" and r["pr_number"]==7,r
PY
test "$(cat "$SEQUENCE")" = finalize

# Build a real squash/rebase-shaped history.  The task head is not an
# ancestor of origin/main, so the orchestrator must derive and bind a
# repository-generated patch-equivalence receipt before sync/cleanup.
git -C "$REPO" switch -q -c task/finalize-squash
printf 'squash change\n' >"$REPO/squash.txt"
git -C "$REPO" add squash.txt
git -C "$REPO" commit -qm task-squash
SQUASH_TASK_HEAD="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" switch -q main
printf 'squash change\n' >"$REPO/squash.txt"
git -C "$REPO" add squash.txt
git -C "$REPO" commit -qm squash-integration
SQUASH_MAIN_COMMIT="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" push -q origin main
python3 - "$REPO/.pm/github-project-sync/tasks.json" "$UID_VALUE" <<'PY'
import json,sys
path,uid=sys.argv[1:]
data=json.load(open(path,encoding='utf-8'))
r=data['tasks'][uid]
r.update(task_branch='task/finalize-squash',pr_number=9,pr_url='https://example.invalid/pull/9')
json.dump(data,open(path,'w',encoding='utf-8'))
PY
rm -f "$REPO/.git/receipts/terminal-cleanup-receipt.json" \
  "$REPO/.git/receipts/finalizer-ledger.json" "$REPO/.git/receipts/finalizer-crashed" \
  "$REPO/.git/receipts/merge-receipt.json" "$REPO/.git/receipts/main-sync-receipt.json" \
  "$REPO/.git/receipts/patch-equivalence-receipt.json"
PATCH_HELPER="$SOURCE_ROOT/scripts/pm/patch-equivalence-receipt.sh"
export PATCH_HELPER
cat >"$REPO/scripts/pm/patch-equivalence-receipt.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo patch-equivalence >>"$TEST_SEQUENCE"
exec "$PATCH_HELPER" "$@"
EOF
chmod +x "$REPO/scripts/pm/patch-equivalence-receipt.sh"
cat >"$REPO/scripts/pm/post-merge-main-sync.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo main-sync >>"$TEST_SEQUENCE"
patch=""
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "--patch-equivalence-receipt" ]]; then patch="${2:-}"; shift 2
  else shift
  fi
done
[[ -n "$patch" && -s "$patch" ]] || { echo 'automatic squash proof was not forwarded' >&2; exit 91; }
EOF
chmod +x "$REPO/scripts/pm/post-merge-main-sync.sh"
cat >"$REPO/scripts/pm/post-merge-cleanup.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo cleanup >>"$TEST_SEQUENCE"
patch=""
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "--patch-equivalence-receipt" ]]; then patch="${2:-}"; shift 2
  else shift
  fi
done
[[ -n "$patch" && -s "$patch" ]] || { echo 'automatic squash proof was not forwarded to cleanup' >&2; exit 92; }
: >"$TEST_REPO/.git/receipts/terminal-cleanup-receipt.json"
EOF
chmod +x "$REPO/scripts/pm/post-merge-cleanup.sh"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$SQUASH_TASK_HEAD" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 9 --resume --json >"$TMP/squash.json"
printf '%s\n' merge-receipt task-closeout refresh patch-equivalence main-sync cleanup finalize >"$TMP/squash-expected"
cmp "$TMP/squash-expected" "$SEQUENCE"
python3 - "$TMP/squash.json" "$REPO/.git/receipts/patch-equivalence-receipt.json" "$SQUASH_TASK_HEAD" "$SQUASH_MAIN_COMMIT" <<'PY'
import json,sys
result=json.load(open(sys.argv[1])); assert result['status']=='finalized' and result['pr_number']==9,result
proof=json.load(open(sys.argv[2])); assert proof['receipt_type']=='oasis7_patch_equivalence',proof
assert proof['branch_tip']==sys.argv[3],proof
assert proof['main_commit']==sys.argv[4],proof
assert proof['projected_tree_oid']==proof['main_tree_oid'],proof
PY

echo "finalize-task.test: OK"
